//! Async diagnostic 8 — which handler statement strands MIE?
//!
//! `async-diag7` settled the core's behaviour: on trap entry `mstatus` reads
//! `0x1880` (MIE=0, MPIE=1), an empty `critical_section::with` leaves it exactly
//! so, and `mret` restores MIE=1. The core conforms to spec, and neither the
//! critical-section implementation nor MPIE is at fault.
//!
//! But diag7 differed from the failing handler in three ways, not one, and only
//! the critical section was under test. Two candidates remain, and this run
//! samples raw `mstatus` between each so the transition names itself:
//!
//! 1. **The PLIC `intr_status()` read** — diag7's handler never touched the
//!    PLIC. An MMIO read must not alter `mstatus`; if it does, something is
//!    surfacing as a trap.
//! 2. **`timer::on_interrupt`** — its control write and EOI read are already
//!    proven safe by diag7, so what it adds is one critical section holding
//!    `Cell::set`, `RefCell::borrow_mut().take()` and `waker.wake()`.
//!
//! If `mstatus` survives the whole handler with MPIE=1 and MIE still reads 0 in
//! `main` afterwards, then the loss happens at `mret` or later and not in the
//! handler at all — the last branch reports exactly that rather than pretending
//! one of the two candidates was caught.
//!
//! The statement-by-statement MIE trace inherited from diag3 is retained, so
//! this re-confirms that baseline in the same run.

#![no_std]
#![no_main]

use panic_halt as _;

use core::sync::atomic::{AtomicU32, Ordering};
use riscv::interrupt::Interrupt as CoreInterrupt;
use thejas32_examples::console::{write_dec, write_hex, write_str};
use thejas32_hal::{
    plic,
    serial::Serial,
    timer::{self, Timer},
};
use thejas32_pac::interrupt::Interrupt;

const DELAY: u32 = 100_000;

/// Raw `mstatus` sampled between each statement of the handler.
/// `AtomicU32` store/load is available on riscv32im (no CAS needed).
static MSTATUS_ENTRY: AtomicU32 = AtomicU32::new(0);
static MSTATUS_AFTER_CS: AtomicU32 = AtomicU32::new(0);
static MSTATUS_AFTER_PLIC: AtomicU32 = AtomicU32::new(0);
static MSTATUS_AFTER_ONINT: AtomicU32 = AtomicU32::new(0);

/// Reads raw `mstatus`.
#[inline(always)]
fn mstatus_bits() -> u32 {
    riscv::register::mstatus::read().bits() as u32
}

#[riscv_rt::core_interrupt(CoreInterrupt::MachineExternal)]
fn machine_external() {
    MSTATUS_ENTRY.store(mstatus_bits(), Ordering::Relaxed);

    // Proven harmless by diag7; kept so the sequence stays comparable.
    critical_section::with(|_| {});
    MSTATUS_AFTER_CS.store(mstatus_bits(), Ordering::Relaxed);

    // Candidate 1: the PLIC status read that diag7's handler did not perform.
    // SAFETY: read-only PLIC access from the handler.
    let plic_regs = unsafe { thejas32_pac::Plic::steal() };
    let active = plic_regs.intr_status().read().bits();
    MSTATUS_AFTER_PLIC.store(mstatus_bits(), Ordering::Relaxed);

    // Candidate 2: the real handler. Its control write and EOI read are already
    // proven safe by diag7, so what this adds is the critical section holding
    // `Cell::set`, `RefCell::borrow_mut().take()` and `waker.wake()`. It also
    // performs the acknowledge, which is why the inline one is gone.
    if active & (1 << (Interrupt::TIMER0 as u8)) != 0 {
        timer::on_interrupt::<thejas32_pac::Timer0>();
    }
    MSTATUS_AFTER_ONINT.store(mstatus_bits(), Ordering::Relaxed);
}

/// Prints `mstatus.MIE` next to a label.
fn mie<W: embedded_hal_nb::serial::Write<u8>>(serial: &mut W, label: &[u8]) {
    write_str(serial, b"    MIE=");
    write_dec(serial, riscv::register::mstatus::read().mie() as u32);
    write_str(serial, b"  after ");
    write_str(serial, label);
    write_str(serial, b"\r\n");
}

#[riscv_rt::entry]
fn main() -> ! {
    let mut serial = Serial::new_preconfigured(unsafe { thejas32_pac::Uart0::steal() });
    write_str(
        &mut serial,
        b"\r\nasync diag 8 -- which handler statement strands MIE?\r\n",
    );

    let plic = plic::Plic::new(unsafe { thejas32_pac::Plic::steal() });
    let mut timer = Timer::new(unsafe { thejas32_pac::Timer0::steal() });
    let mut reference = Timer::new(unsafe { thejas32_pac::Timer1::steal() });
    mie(&mut serial, b"taking peripherals");

    plic.enable(Interrupt::TIMER0);
    mie(&mut serial, b"plic.enable");

    // SAFETY: handler installed above.
    unsafe { plic::enable_external_interrupts() };
    mie(&mut serial, b"enable_external_interrupts");

    // SAFETY: as above.
    unsafe { plic::enable_global_interrupts() };
    mie(
        &mut serial,
        b"enable_global_interrupts  <-- should be 1 from here",
    );

    // --- the statements the await performs, one at a time -----------------
    reference.start_free_running();
    mie(&mut serial, b"reference.start_free_running");

    let _ = reference.counter();
    mie(&mut serial, b"reference.counter");

    critical_section::with(|_| {});
    mie(&mut serial, b"empty critical_section::with");

    timer.start_periodic_interrupt(DELAY);
    mie(&mut serial, b"timer.start_periodic_interrupt");

    critical_section::with(|_| {});
    mie(&mut serial, b"another critical_section::with");

    // Let the timer expire; the handler should fire and clear it.
    for _ in 0..400_000 {
        unsafe { core::arch::asm!("nop") };
    }
    mie(&mut serial, b"waiting for the interrupt");

    let t = unsafe { thejas32_pac::Timer0::steal() };
    write_str(&mut serial, b"    timer ctrl now: ");
    write_dec(&mut serial, t.control().read().bits());
    write_str(
        &mut serial,
        b"  (0 = handler ran and stopped it, 3 = still running)\r\n",
    );

    timer.stop();
    mie(&mut serial, b"timer.stop");

    // ---- mstatus between each handler statement --------------------------
    let entry = MSTATUS_ENTRY.load(Ordering::Relaxed);
    let after_cs = MSTATUS_AFTER_CS.load(Ordering::Relaxed);
    let after_plic = MSTATUS_AFTER_PLIC.load(Ordering::Relaxed);
    let after_onint = MSTATUS_AFTER_ONINT.load(Ordering::Relaxed);
    write_str(&mut serial, b"\r\n  inside the handler:\r\n");
    field(&mut serial, b"    on entry          ", entry);
    field(&mut serial, b"    after empty CS    ", after_cs);
    field(&mut serial, b"    after PLIC read   ", after_plic);
    field(&mut serial, b"    after on_interrupt", after_onint);

    write_str(&mut serial, b"\r\nverdict:\r\n");
    if entry == 0 {
        write_str(
            &mut serial,
            b"  The handler never ran -- this run proves nothing.\r\n",
        );
    } else if after_cs != entry {
        write_str(
            &mut serial,
            b"  The empty critical section changed mstatus, contradicting\r\n\
              \x20 diag7. Re-run diag7 before trusting either result.\r\n",
        );
    } else if after_plic != after_cs {
        write_str(
            &mut serial,
            b"  Reading the PLIC status register changed mstatus. An MMIO read\r\n\
              \x20 must not do that -- suspect a bus fault surfacing as a trap.\r\n",
        );
    } else if after_onint != after_plic {
        write_str(
            &mut serial,
            b"  timer::on_interrupt changed mstatus. Its control write and EOI\r\n\
              \x20 read are proven safe, so the fault is in its critical section:\r\n\
              \x20 Cell::set, RefCell::borrow_mut().take(), or waker.wake().\r\n",
        );
    } else if (after_onint & 0x80) == 0 {
        write_str(
            &mut serial,
            b"  mstatus is unchanged throughout but MPIE is 0, so mret will\r\n\
              \x20 disable interrupts. MPIE was already 0 on entry.\r\n",
        );
    } else {
        write_str(
            &mut serial,
            b"  mstatus survives the whole handler with MPIE=1. If MIE still\r\n\
              \x20 reads 0 in main afterwards, the loss happens at mret or\r\n\
              \x20 later -- not inside the handler at all.\r\n",
        );
    }

    write_str(
        &mut serial,
        b"\r\nThe first line showing MIE=0 names the statement that clears it.\r\n",
    );

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}

/// Prints raw `mstatus` with the two fields that decide interrupt delivery.
fn field<W: embedded_hal_nb::serial::Write<u8>>(serial: &mut W, label: &[u8], bits: u32) {
    write_str(serial, label);
    write_str(serial, b"mstatus=");
    write_hex(serial, bits, 8);
    write_str(serial, b"  MIE=");
    write_dec(serial, (bits >> 3) & 1);
    write_str(serial, b"  MPIE=");
    write_dec(serial, (bits >> 7) & 1);
    write_str(serial, b"\r\n");
}
