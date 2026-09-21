//! Interrupt controller (PLIC) for the THEJAS32 SoC.
//!
//! # This is not a standard RISC-V PLIC
//!
//! Despite the name, THEJAS32's controller shares almost nothing with the
//! RISC-V PLIC specification:
//!
//! * **Flat bitmask, no claim/complete.** There is no claim register and no
//!   completion write. The handler reads the masked status register and
//!   services every set bit itself.
//! * **The source peripheral clears its own pending bit.** Nothing in this
//!   controller acknowledges an interrupt — e.g. a timer interrupt is cleared
//!   by reading that timer's `EOI`, not by writing here. Forgetting this
//!   leaves the handler re-entering forever.
//! * **No priorities and no thresholds.** An interrupt is either enabled or
//!   not; when several are pending the service order is whatever the handler
//!   chooses.
//!
//! Generic RISC-V PLIC knowledge is therefore actively misleading here, and
//! `riscv-rt`'s `#[external_interrupt]` dispatch — which assumes
//! claim/complete — must not be used. Handle `MachineExternal` directly and
//! dispatch from [`Plic::active`].
//!
//! Register layout and semantics follow CDAC's `vega-sdk`
//! (`bsp/drivers/interrupt/interrupt.c`).
//!
//! # Software interrupts are non-standard too
//!
//! A machine software interrupt is raised by writing `mip.MSIP` **directly**.
//! In standard RISC-V that bit is read-only and owned by the CLINT; THEJAS32
//! has no CLINT, so it is writable here. The `riscv` crate consequently
//! offers no setter for it (only the supervisor-mode equivalents), which is
//! why [`trigger_software_interrupt`] uses raw CSR instructions.

use thejas32_pac::{interrupt::Interrupt, Plic as PacPlic};

/// `mip`/`mie` bit for a machine software interrupt.
const MSIP_BIT: usize = 1 << 3;

/// The interrupt controller.
pub struct Plic {
    plic: PacPlic,
}

impl Plic {
    /// Takes ownership of the controller and masks every source.
    ///
    /// Starting from a known state matters: the ROM bootloader runs before
    /// this and does not necessarily leave the enable register clear.
    pub fn new(plic: PacPlic) -> Self {
        let this = Plic { plic };
        this.disable_all();
        this
    }

    /// Bit position of `irq` in the enable and status registers.
    #[inline]
    fn bit(irq: Interrupt) -> u32 {
        1u32 << (irq as u8)
    }

    /// Unmasks `irq` in the controller.
    ///
    /// This is only half of what an interrupt needs: the core must also have
    /// external interrupts enabled ([`enable_external_interrupts`]) and its
    /// global interrupt bit set ([`enable_global_interrupts`]).
    pub fn enable(&self, irq: Interrupt) {
        let enabled = self.plic.intr_en().read().bits() | Self::bit(irq);
        // SAFETY: the enable register is a plain bitmask of interrupt sources.
        unsafe { self.plic.intr_en().write(|w| w.bits(enabled)) };
        crate::fence();
    }

    /// Masks `irq`.
    pub fn disable(&self, irq: Interrupt) {
        let enabled = self.plic.intr_en().read().bits() & !Self::bit(irq);
        // SAFETY: as `enable`.
        unsafe { self.plic.intr_en().write(|w| w.bits(enabled)) };
        crate::fence();
    }

    /// Masks every source.
    pub fn disable_all(&self) {
        // SAFETY: as `enable`.
        unsafe { self.plic.intr_en().write(|w| w.bits(0)) };
        crate::fence();
    }

    /// Whether `irq` is unmasked.
    pub fn is_enabled(&self, irq: Interrupt) -> bool {
        self.plic.intr_en().read().bits() & Self::bit(irq) != 0
    }

    /// The enable mask, as raw bits.
    pub fn enabled_mask(&self) -> u32 {
        self.plic.intr_en().read().bits()
    }

    /// Sources asserting, **ignoring** the enable mask.
    ///
    /// Useful for polling a peripheral without letting it interrupt the core.
    pub fn raw_pending(&self) -> u32 {
        self.plic.raw_intr().read().bits()
    }

    /// Sources asserting **and** unmasked — what the core actually sees.
    ///
    /// This is the register a `MachineExternal` handler dispatches from.
    pub fn active(&self) -> u32 {
        self.plic.intr_status().read().bits()
    }

    /// Whether `irq` is currently asserting and unmasked.
    pub fn is_active(&self, irq: Interrupt) -> bool {
        self.active() & Self::bit(irq) != 0
    }

    /// Whether `irq` is asserting, regardless of its mask.
    pub fn is_raw_pending(&self, irq: Interrupt) -> bool {
        self.raw_pending() & Self::bit(irq) != 0
    }

    /// The lowest-numbered active source, if any.
    ///
    /// With no hardware priorities, "lowest number first" is a policy choice
    /// this function makes, not something the controller imposes.
    pub fn next_active(&self) -> Option<Interrupt> {
        let active = self.active();
        if active == 0 {
            return None;
        }
        Interrupt::try_from(active.trailing_zeros() as u8).ok()
    }

    /// Releases the controller.
    pub fn free(self) -> PacPlic {
        self.plic
    }
}

/// Enables machine **external** interrupts in `mie` (`MEIE`).
///
/// # Safety
///
/// Interrupts will begin dispatching to the trap handler once
/// [`enable_global_interrupts`] is also called; the handler must be able to
/// service and clear every unmasked source.
pub unsafe fn enable_external_interrupts() {
    unsafe { riscv::register::mie::set_mext() };
}

/// Disables machine external interrupts in `mie`.
///
/// # Safety
///
/// Callers must not rely on pending external interrupts being delivered.
pub unsafe fn disable_external_interrupts() {
    unsafe { riscv::register::mie::clear_mext() };
}

/// Enables machine **software** interrupts in `mie` (`MSIE`).
///
/// # Safety
///
/// As [`enable_external_interrupts`].
pub unsafe fn enable_software_interrupts() {
    unsafe { riscv::register::mie::set_msoft() };
}

/// Sets the global machine interrupt enable (`mstatus.MIE`).
///
/// # Safety
///
/// Nothing is delivered until this is called, so this is the point at which
/// handlers must already be correct.
pub unsafe fn enable_global_interrupts() {
    unsafe { riscv::register::mstatus::set_mie() };
}

/// Clears the global machine interrupt enable.
///
/// # Safety
///
/// Interrupts stop being delivered; anything relying on them will stall.
pub unsafe fn disable_global_interrupts() {
    unsafe { riscv::register::mstatus::clear_mie() };
}

/// Raises a machine software interrupt by setting `mip.MSIP`.
///
/// Standard RISC-V makes this bit read-only, owned by the CLINT, so the
/// `riscv` crate provides no setter — hence the raw CSR write. THEJAS32 has
/// no CLINT and makes the bit directly writable (`vega-sdk`'s
/// `generate_sw_irq` does the same).
///
/// # Safety
///
/// Raises an interrupt immediately if software interrupts are enabled; a
/// handler must be installed. The handler is responsible for calling
/// [`clear_software_interrupt`], or it will re-enter forever.
pub unsafe fn trigger_software_interrupt() {
    // SAFETY: `csrs` on `mip` with the MSIP bit; writable on this SoC.
    unsafe { core::arch::asm!("csrs mip, {bit}", bit = in(reg) MSIP_BIT) };
}

/// Clears `mip.MSIP`. Must be called from the software-interrupt handler.
///
/// # Safety
///
/// As [`trigger_software_interrupt`] — a raw CSR write.
pub unsafe fn clear_software_interrupt() {
    // SAFETY: `csrc` on `mip` with the MSIP bit.
    unsafe { core::arch::asm!("csrc mip, {bit}", bit = in(reg) MSIP_BIT) };
}
