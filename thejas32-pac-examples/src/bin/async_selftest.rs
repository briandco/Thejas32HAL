//! Async self-test — validates the interrupt-driven timer future.
//!
//! This is the first thing in the HAL that does not busy-wait. Everything else
//! spins on a status flag; here the core sleeps in `wfi` until the timer's
//! interrupt arrives through the PLIC.
//!
//! What each phase establishes:
//!
//! 1. **The future completes at all** — proves the whole chain works: PLIC
//!    unmasked, external interrupts enabled, handler installed, source
//!    acknowledged, waker fired, executor resumed. Any broken link hangs, so
//!    this phase is bounded by a watchdog timer rather than trusted to finish.
//! 2. **Duration is correct** — measured against the free-running reference
//!    timer, the same yardstick used for the blocking driver.
//! 3. **It genuinely sleeps** — poll count is the tell. A working async wait
//!    polls a handful of times; a busy-wait dressed up as a future polls
//!    thousands. Without this check, phases 1 and 2 would pass on an
//!    implementation that spun the whole time.
//! 4. **`embedded_hal_async::DelayNs`** — the ecosystem trait, not just our
//!    inherent method.
//!
//! Wiring required: TIMER0 unmasked in the PLIC, `MachineExternal` routed to
//! `timer::on_interrupt`, and core external interrupts enabled. All three are
//! set up below.

#![no_std]
#![no_main]

use panic_halt as _;

use core::sync::atomic::{AtomicU32, Ordering};

use embedded_hal_async::delay::DelayNs;
use riscv::interrupt::Interrupt as CoreInterrupt;
use thejas32_examples::console::{write_dec, write_str};
use thejas32_examples::executor::try_block_on;
use thejas32_hal::{
    plic,
    serial::Serial,
    timer::{self, AsyncTimer, Timer},
};
use thejas32_pac::interrupt::Interrupt;

/// Handler entries, so a stalled run can be told apart from an undelivered
/// interrupt.
///
/// Plain atomics rather than `critical_section::Mutex`: acquiring a critical
/// section inside an ISR clears `mstatus.MPIE` on this core, which strands
/// interrupts off after the first one (see `async-diag8`). riscv32im has
/// load/store atomics — only CAS is missing — so `store`/`load` are available
/// and need no critical section at all.
static HANDLER_RUNS: AtomicU32 = AtomicU32::new(0);
static TIMER_HITS: AtomicU32 = AtomicU32::new(0);

/// 1 ms at 100 MHz.
const DELAY: u32 = 100_000;
/// A sleeping wait should poll only a few times; well under this.
const POLL_CEILING: u32 = 32;
/// Tolerance on measured duration, in percent.
const TOLERANCE_PCT: u32 = 10;
/// Poll bound. `wfi` is a NOP on this core (see `wfi-diag`), so the executor
/// busy-polls; the working case needs ~2,800 polls for 1 ms. 500k is ~180x
/// margin and times out in well under a second — a bound so large that a
/// timeout looks like a hang defeats the point of having one.
const POLL_LIMIT: u32 = 500_000;

#[riscv_rt::core_interrupt(CoreInterrupt::MachineExternal)]
fn machine_external() {
    HANDLER_RUNS.store(HANDLER_RUNS.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
    // SAFETY: read-only PLIC access from the handler.
    let plic_regs = unsafe { thejas32_pac::Plic::steal() };
    if plic_regs.intr_status().read().bits() & (1 << (Interrupt::TIMER0 as u8)) != 0 {
        TIMER_HITS.store(TIMER_HITS.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
        // Acknowledges the source and wakes the future. Nothing else clears
        // it — this controller has no claim/complete cycle.
        timer::on_interrupt::<thejas32_pac::Timer0>();
    }
}

#[riscv_rt::entry]
fn main() -> ! {
    // The PAC's singleton, taken once. `steal()` below (where present) is a
    // deliberate read-only alias for register inspection, not a second owner.
    let peripherals = thejas32_pac::Peripherals::take().unwrap();
    let mut serial = Serial::new_preconfigured(peripherals.uart0);
    let plic = plic::Plic::new(peripherals.plic);
    let mut async_timer = AsyncTimer::new(Timer::new(peripherals.timer0));
    let mut reference = Timer::new(peripherals.timer1);

    write_str(
        &mut serial,
        b"\r\nasync self-test (TIMER0 future, ref TIMER1)\r\n",
    );

    plic.enable(Interrupt::TIMER0);
    // SAFETY: the MachineExternal handler above is installed and acknowledges
    // the source before returning.
    unsafe {
        plic::enable_external_interrupts();
        plic::enable_global_interrupts();
    }

    // ---- [1] + [2] the future completes, and takes the right time --------
    let mut measured = [0u32; 3];
    let mut polls = [0u32; 3];
    let mut completed = [false; 3];
    write_str(&mut serial, b"\r\n[1/2] awaited delays\r\n");
    for i in 0..3 {
        // Marker before the await: if anything stalls, the last line printed
        // says exactly which run it was.
        write_str(&mut serial, b"    run ");
        write_dec(&mut serial, i as u32 + 1);
        write_str(&mut serial, b": awaiting... ");
        reference.start_free_running();
        let start = reference.counter();
        let (outcome, count) = try_block_on(
            async {
                async_timer.delay_ticks(DELAY).await;
            },
            POLL_LIMIT,
        );
        completed[i] = outcome.is_some();
        measured[i] = start.wrapping_sub(reference.counter());
        polls[i] = count;
        write_str(
            &mut serial,
            if completed[i] { b"done" } else { b"TIMED OUT" },
        );
        let (h, t) = (
            HANDLER_RUNS.load(Ordering::Relaxed),
            TIMER_HITS.load(Ordering::Relaxed),
        );
        write_str(&mut serial, b"  handlers=");
        write_dec(&mut serial, h);
        write_str(&mut serial, b" hits=");
        write_dec(&mut serial, t);
        write_str(&mut serial, b" MIE=");
        write_dec(&mut serial, riscv::register::mstatus::read().mie() as u32);
        write_str(&mut serial, b"\r\n");
    }

    let lo = DELAY - DELAY / (100 / TOLERANCE_PCT);
    let hi = DELAY + DELAY / (100 / TOLERANCE_PCT);
    let p1 = completed.iter().all(|&c| c);
    let p2 = measured.iter().all(|&m| m >= lo && m <= hi);

    for i in 0..3 {
        write_str(&mut serial, b"    run ");
        write_dec(&mut serial, i as u32 + 1);
        write_str(&mut serial, b": ");
        write_dec(&mut serial, measured[i]);
        write_str(&mut serial, b" ticks (want ");
        write_dec(&mut serial, DELAY);
        write_str(&mut serial, b"), polls=");
        write_dec(&mut serial, polls[i]);
        write_str(&mut serial, b"\r\n");
    }
    line(&mut serial, b"    future completes     ", p1);
    line(&mut serial, b"    duration correct     ", p2);

    // ---- [3] does it sleep, or spin? --------------------------------------
    // Reported honestly rather than scored: `wfi` is a NOP on this core
    // (measured, see `wfi-diag`), so the executor cannot sleep and a high poll
    // count is expected. Relaxing the threshold to make this "pass" would hide
    // a real limitation of the platform.
    let max_polls = *polls.iter().max().unwrap_or(&0);
    let sleeping = max_polls <= POLL_CEILING;
    write_str(
        &mut serial,
        b"\r\n[3] sleep vs spin (informational)\r\n    max polls: ",
    );
    write_dec(&mut serial, max_polls);
    write_str(&mut serial, b"\r\n    ");
    write_str(
        &mut serial,
        if sleeping {
            b"slept between wakes\r\n"
        } else {
            b"busy-polled -- expected: wfi is a NOP on this core\r\n"
        },
    );

    // ---- [4] the ecosystem trait -----------------------------------------
    write_str(
        &mut serial,
        b"\r\n[4] embedded_hal_async::DelayNs\r\n    delay_ms(2): awaiting... ",
    );
    reference.start_free_running();
    let start = reference.counter();
    let trait_done = try_block_on(
        async {
            // Via embedded_hal_async::delay::DelayNs, not our inherent method.
            async_timer.delay_ms(2).await;
        },
        POLL_LIMIT,
    )
    .0
    .is_some();
    let trait_measured = start.wrapping_sub(reference.counter());
    let want = 2 * DELAY;
    let p4 = trait_done && trait_measured >= want - want / 10 && trait_measured <= want + want / 10;

    write_str(
        &mut serial,
        if trait_done {
            b"done\r\n    "
        } else {
            b"TIMED OUT\r\n    "
        },
    );
    write_dec(&mut serial, trait_measured);
    write_str(&mut serial, b" ticks (want ");
    write_dec(&mut serial, want);
    write_str(&mut serial, b")\r\n");
    line(&mut serial, b"    trait impl works     ", p4);

    // SAFETY: quiesce before the summary so the console is not interrupted.
    unsafe { plic::disable_global_interrupts() };
    plic.disable_all();

    let passed = [p1, p2, p4].iter().filter(|&&b| b).count();
    write_str(&mut serial, b"\r\nresult: ");
    write_dec(&mut serial, passed as u32);
    write_str(
        &mut serial,
        b"/3 checks passed (sleep is informational)\r\n",
    );

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}

fn line<W: embedded_hal_nb::serial::Write<u8>>(serial: &mut W, label: &[u8], ok: bool) {
    write_str(serial, label);
    write_str(serial, if ok { b"PASS\r\n" } else { b"FAIL\r\n" });
}
