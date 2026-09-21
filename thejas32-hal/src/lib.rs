//! Hardware abstraction layer for the CDAC VEGA THEJAS32 RISC-V SoC.
//!
//! Sits on top of [`thejas32_pac`] and targets `embedded-hal` 1.0 traits.
//! Every driver here is hand-written and validated against real silicon; see
//! the workspace README for the per-peripheral validation status.
//!
//! # Hardware quirks that affect every driver
//!
//! **Write ordering.** The peripheral bus reorders writes to different
//! registers of the same peripheral. Use [`fence`] between a configuration
//! write and the write that acts on it.
//!
//! **Peripheral clock.** [`PERIPHERAL_CLOCK_HZ`] is measured rather than taken
//! from vendor sources, which disagree with each other.
//!
//! **Never take a critical section inside an interrupt handler.** On this core
//! `csrrci mstatus, 8` — the instruction `critical_section::acquire` uses —
//! also clears `mstatus.MPIE` when the trap was taken at the moment another
//! critical section released. `mret` then restores `mstatus.MIE` from a zeroed
//! `MPIE`, so interrupts are disabled permanently and silently: the first
//! interrupt is serviced correctly and no further one is ever delivered.
//! Architecturally that instruction may write bit 3 only, so this is a
//! deviation from the RISC-V privileged specification, not a driver bug.
//!
//! It is also a *race*, which is what makes it dangerous: a handler that takes
//! a critical section works fine until an interrupt happens to arrive while the
//! main program is inside one. During bring-up the same binary passed three
//! runs and then failed three runs with no source change.
//!
//! Handlers already run with `MIE` clear, and this is a single-hart part, so an
//! ISR holds exclusive access without acquiring anything. Use
//! [`critical_section::CriticalSection::new`] to mint a token, as
//! [`timer::on_interrupt`] does, or plain atomics — riscv32im has load/store
//! atomics and lacks only CAS. Established by the `async-diag8` example, which
//! reads `mstatus` between each statement of a handler.

#![deny(missing_docs)]
#![no_std]

pub use thejas32_pac as pac;

/// Peripheral input clock, in hertz.
///
/// **Measured on hardware**, not assumed: the ROM bootloader leaves a baud
/// divisor of 54 in UART0's latch for its working 115200 console, giving
/// `54 * 115200 * 16 = 99_532_800 Hz`. Integer truncation means the divisor
/// is consistent with any clock in roughly 99.5–101.4 MHz, and 100 MHz is the
/// nominal design value in that range (it also matches the THEJAS32 Zephyr
/// port and `TrustedV_RTOS`'s BSP).
///
/// Note that CDAC's `vega-sdk` examples pass `40_000_000` as the UART clock.
/// That value does **not** hold for this board — it yields divisor 21 and a
/// line rate ~2.6x too fast, which fails silently as garbage rather than as
/// an error. Use this constant instead.
///
/// Re-measure with the `clock-probe` example if you move to different
/// silicon or a different board revision.
pub const PERIPHERAL_CLOCK_HZ: u32 = 100_000_000;

/// Orders a device write against accesses that follow it.
///
/// THEJAS32's peripheral bus can apply writes to different registers of the
/// same peripheral **out of order**, even though the core is in-order and the
/// accesses are `volatile` (which only constrains the compiler). A
/// configuration write must therefore be fenced before the write that acts on
/// it.
///
/// This is not theoretical. Arming a timer as `load_count = 1000` followed
/// immediately by `control = enable` leaves the counter free-running from
/// `0xFFFF_FFFF`: the load register reads back correctly, but the enable
/// overtakes it. Inserting this fence between the two writes — the only
/// difference — makes the counter load 1000 and expire correctly. CDAC's
/// `vega-sdk` fences every device write for this reason.
#[inline(always)]
pub fn fence() {
    // SAFETY: `fence` takes no operands and only orders memory accesses.
    unsafe { core::arch::asm!("fence") };
}

pub mod adc;
pub mod gpio;
pub mod i2c;
pub mod plic;
pub mod pwm;
pub mod serial;
pub mod spi;
pub mod timer;
