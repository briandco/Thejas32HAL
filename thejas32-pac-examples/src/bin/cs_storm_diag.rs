//! Does a *task-context* critical-section storm strand `MIE`?
//!
//! Our established finding is narrower than it may need to be: "never take a
//! `critical_section` inside an interrupt handler", because `csrrci mstatus, 8`
//! also clears `mstatus.MPIE` when a trap lands as another critical section
//! releases. Every example here obeys that rule and every self-test passes.
//!
//! But a 10-minute RTOS benchmark run on this board showed its 1 ms tick
//! counter frozen across two consecutive 2-minute phases, then advancing
//! normally in a third. The phases that froze were tight loops executing
//! `csrrci`/`csrsi` millions of times **in task context, never in a handler**.
//! The phase that recovered was the first one to perform a context switch,
//! which rewrites `mstatus` wholesale and would paper over a stuck `MPIE`.
//!
//! That suggests the hazard is not "inside a handler" but "a trap arriving
//! during the release window", which task code hits too — just rarely enough
//! that short tests never see it. Our own self-tests take at most a few
//! thousand critical sections; the benchmark took millions.
//!
//! So this sweeps storm size and asks one question per round: after N
//! back-to-back critical sections in task context, does the timer interrupt
//! still fire?
//!
//! Oracle, stated before the run:
//!
//! * **Hypothesis refuted** — ticks keep advancing after every N, including
//!   1,000,000. Then task-context critical sections are safe, the RTOS freeze
//!   has some other cause, and `thejas32-rs` is unaffected.
//! * **Hypothesis confirmed** — at some N the tick counter stops. The round
//!   that fails names the rate, and `mstatus` is printed so the mechanism is
//!   visible rather than inferred.
//!
//! A dead round re-enables `MIE` directly before the next one, so a single run
//! reports every storm size instead of stopping at the first failure — and
//! whether the failure reproduces is itself evidence (a one-off would suggest
//! something other than a race).

#![no_std]
#![no_main]

use panic_halt as _;

use core::sync::atomic::{AtomicU32, Ordering};
use riscv::interrupt::Interrupt as CoreInterrupt;
use thejas32_examples::console::{write_dec, write_hex, write_str};
use thejas32_hal::{plic, serial::Serial, timer::Timer};
use thejas32_pac::interrupt::Interrupt;

/// 100_000 cycles at the measured 100 MHz peripheral clock = 1 ms.
const DELAY: u32 = 100_000;

/// Storm sizes, ascending. The RTOS phases that froze ran roughly 8-10 million
/// critical sections over two minutes, so a failure is expected — if it comes
/// at all — somewhere in the upper half of this sweep.
const STORMS: [u32; 5] = [1_000, 10_000, 100_000, 1_000_000, 4_000_000];

/// Incremented by the timer handler. Load/store only: riscv32im has no CAS, so
/// `fetch_add` is unavailable. The handler is non-reentrant (the core clears
/// `MIE` on trap entry), so a plain read-add-write cannot race with itself.
static TICKS: AtomicU32 = AtomicU32::new(0);

#[riscv_rt::core_interrupt(CoreInterrupt::MachineExternal)]
fn machine_external() {
    // Clear the timer's pending flag at source. Deliberately no critical
    // section and no PLIC status read: both are known hazards on this core,
    // and this handler exists to observe interrupt delivery, not to test it.
    let tim = unsafe { thejas32_pac::Timer0::steal() };
    let _ = tim.eoi().read().bits();
    TICKS.store(
        TICKS.load(Ordering::Relaxed).wrapping_add(1),
        Ordering::Relaxed,
    );
}

#[inline(always)]
fn mstatus_bits() -> u32 {
    riscv::register::mstatus::read().bits() as u32
}

/// Burns roughly `cycles` worth of nops. Used to let several timer periods
/// elapse; precision does not matter, only that it is comfortably longer than
/// one 1 ms period.
#[inline(never)]
fn spin(cycles: u32) {
    for _ in 0..cycles {
        unsafe { core::arch::asm!("nop", options(nomem, nostack)) };
    }
}

#[riscv_rt::entry]
fn main() -> ! {
    let mut serial = Serial::new_preconfigured(unsafe { thejas32_pac::Uart0::steal() });
    write_str(
        &mut serial,
        b"\r\ncs-storm-diag -- do task-context critical sections strand MIE?\r\n\r\n",
    );

    let plic = plic::Plic::new(unsafe { thejas32_pac::Plic::steal() });
    let mut timer = Timer::new(unsafe { thejas32_pac::Timer0::steal() });

    plic.enable(Interrupt::TIMER0);
    // SAFETY: the handler above is installed by the `core_interrupt` attribute.
    unsafe { plic::enable_external_interrupts() };
    // SAFETY: as above.
    unsafe { plic::enable_global_interrupts() };

    timer.start_periodic_interrupt(DELAY);

    // Baseline: prove interrupts are alive before storming anything. Without
    // this a dead round later would be ambiguous -- it could just mean the
    // timer never started.
    let base_before = TICKS.load(Ordering::Relaxed);
    spin(1_000_000);
    let base_after = TICKS.load(Ordering::Relaxed);
    write_str(&mut serial, b"baseline: ticks advanced by ");
    write_dec(&mut serial, base_after.wrapping_sub(base_before));
    write_str(&mut serial, b"  mstatus=");
    write_hex(&mut serial, mstatus_bits(), 8);
    if base_after == base_before {
        write_str(
            &mut serial,
            b"\r\n  INCONCLUSIVE: timer never fired, nothing below means anything.\r\n",
        );
        loop {
            core::hint::spin_loop();
        }
    }
    write_str(&mut serial, b"   (interrupts confirmed alive)\r\n\r\n");

    let mut failures = 0u32;

    for &n in STORMS.iter() {
        write_str(&mut serial, b"storm n=");
        write_dec(&mut serial, n);
        write_str(&mut serial, b"\r\n");

        // The storm itself: N back-to-back acquire/release pairs, in task
        // context, doing nothing inside. `critical_section::with` compiles to
        // `csrrci mstatus, 8` ... `csrsi mstatus, 8` and contains volatile asm,
        // so the loop cannot be optimised away.
        let before = TICKS.load(Ordering::Relaxed);
        for _ in 0..n {
            critical_section::with(|_| {});
        }
        let during = TICKS.load(Ordering::Relaxed);
        let ms_after_storm = mstatus_bits();

        // Now the actual question: with the storm over and no critical section
        // held, does the timer still fire?
        spin(1_000_000);
        let after = TICKS.load(Ordering::Relaxed);

        write_str(&mut serial, b"  ticks during storm: ");
        write_dec(&mut serial, during.wrapping_sub(before));
        write_str(&mut serial, b"   after storm: ");
        write_dec(&mut serial, after.wrapping_sub(during));
        write_str(&mut serial, b"\r\n  mstatus=");
        write_hex(&mut serial, ms_after_storm, 8);
        write_str(&mut serial, b"  MIE=");
        write_dec(&mut serial, (ms_after_storm >> 3) & 1);
        write_str(&mut serial, b"  MPIE=");
        write_dec(&mut serial, (ms_after_storm >> 7) & 1);

        if after == during {
            failures += 1;
            write_str(
                &mut serial,
                b"\r\n  >>> DEAD: no interrupt after the storm <<<\r\n",
            );
            // Re-enable MIE directly so the next round still has something to
            // measure. MIE is writable regardless of MPIE's state, so this
            // recovers delivery without disturbing what we are testing.
            unsafe { core::arch::asm!("csrsi mstatus, 8", options(nomem, nostack)) };
            let recovered_before = TICKS.load(Ordering::Relaxed);
            spin(1_000_000);
            let recovered = TICKS.load(Ordering::Relaxed) != recovered_before;
            write_str(&mut serial, b"  after csrsi mstatus,8: ");
            write_str(
                &mut serial,
                if recovered {
                    b"ticks resumed (MIE was the stranded bit)\r\n" as &[u8]
                } else {
                    b"still dead (something beyond MIE is wrong)\r\n" as &[u8]
                },
            );
        } else {
            write_str(&mut serial, b"\r\n  ok: interrupts survived\r\n");
        }
        write_str(&mut serial, b"\r\n");
    }

    write_str(&mut serial, b"========================================\r\n");
    if failures == 0 {
        write_str(
            &mut serial,
            b"REFUTED: task-context critical sections did not strand MIE\r\n\
              at any storm size up to 4,000,000. The RTOS tick freeze has\r\n\
              some other cause.\r\n",
        );
    } else {
        write_str(&mut serial, b"CONFIRMED: interrupts died in ");
        write_dec(&mut serial, failures);
        write_str(&mut serial, b" of ");
        write_dec(&mut serial, STORMS.len() as u32);
        write_str(
            &mut serial,
            b" rounds.\r\n  A task-context critical section is sufficient to strand MIE on\r\n\
              this core. The handler-only rule is too narrow.\r\n",
        );
    }
    write_str(&mut serial, b"========================================\r\n");

    loop {
        core::hint::spin_loop();
    }
}
