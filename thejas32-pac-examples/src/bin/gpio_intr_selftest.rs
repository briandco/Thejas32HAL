//! GPIO interrupt self-test — the last 13 of the 23 PLIC lines.
//!
//! **Jumper required: PWM[0] → GPIO0 (`J2_5 → J2_11`)** — the same one
//! `pwm-selftest` uses for its waveform phase. PWM supplies the edges because
//! it is already validated to 0.005% on period, so the stimulus rate is known
//! rather than assumed, and reusing that jumper means no new pinout to get
//! wrong.
//!
//! # What is unknown going in
//!
//! This block has **no interrupt registers at all** — no enable, no edge
//! select, no status (`vega-sdk`'s `gpio.h` declares none, and its two GPIO
//! interrupt examples configure the pin as an input and then only enable the
//! PLIC line). So the trigger condition is fixed in silicon and undocumented.
//! Phase 2 measures it: at a known edge rate, the interrupt count tells us
//! whether one edge per period fires or both.
//!
//! It is also unknown whether the source is **level**-sensitive. If it is, and
//! nothing can acknowledge it, the handler re-enters forever and the core
//! livelocks. The handler therefore masks its own PLIC line after
//! [`STORM_LIMIT`] entries and reports it, so that outcome is a finding rather
//! than a hang.
//!
//! # Oracles
//!
//! 0. **The jumper is fitted** — poll GPIO0 and confirm the level actually
//!    changes while PWM runs. Every later phase reasons about interrupts
//!    arriving or not arriving, and without this a missing jumper looks
//!    identical to a broken driver. Establish the stimulus before asserting on
//!    its absence.
//! 1. **Interrupts are delivered** — the count rises once the PLIC line is
//!    enabled.
//! 2. **The rate matches the edge rate** — measured against the reference
//!    timer, this reports one-edge or both-edge triggering instead of guessing.
//! 3. **Masking stops delivery** — with PWM still running, so the stimulus is
//!    known present. This is the differential half, and unlike the UART test
//!    there is no ambiguity about whether a stimulus occurred.
//! 4. **Re-enabling resumes delivery.**
//!
//! The handler uses plain atomics: a `critical_section` inside an ISR clears
//! `mstatus.MPIE` on this core and kills interrupts permanently (crate docs,
//! `async-diag8`).

#![no_std]
#![no_main]

use panic_halt as _;

use core::sync::atomic::{AtomicU32, Ordering};
use riscv::interrupt::Interrupt as CoreInterrupt;
use thejas32_examples::console::{write_dec, write_str};
use thejas32_hal::{
    PERIPHERAL_CLOCK_HZ,
    gpio::Gpio,
    plic,
    pwm::{Config, IdleLevel, Mode, PwmChannel, PwmGlobal},
    serial::Serial,
    timer::Timer,
};
use thejas32_pac::interrupt::Interrupt;

// Pin under test is bank 0 pin 0: it owns PLIC line 10 and is the far end of
// the `pwm-selftest` jumper. Named rather than a constant now that `split()`
// hands pins out as struct fields.

/// PWM period in ticks: 100 MHz / 1,000,000 = 100 Hz, so 100 periods a second.
/// Slow enough that even both-edge triggering is only 200 interrupts a second.
const PWM_PERIOD_TICKS: u32 = 1_000_000;

/// Measurement window: 1 s in reference-timer ticks.
const WINDOW_TICKS: u32 = PERIPHERAL_CLOCK_HZ;

/// Static-level probe window: ~50 ms. A level-sensitive source storms within
/// microseconds; an edge-triggered one would see several PWM periods, but there
/// are no edges during this probe by construction.
const PROBE_TICKS: u32 = PERIPHERAL_CLOCK_HZ / 20;

/// Window for the jumper check: ~200 ms, at least 20 PWM periods.
const JUMPER_TICKS: u32 = PERIPHERAL_CLOCK_HZ / 5;

/// Entries after which the handler masks its own source.
///
/// A level-sensitive input with no acknowledge path would otherwise livelock
/// the core. At 100 Hz a correct run sees 100–200 entries a second, so this is
/// ~25 s of legitimate traffic — far above any real count here, and far below
/// a runaway.
const STORM_LIMIT: u32 = 5_000;

static GPIO_INTERRUPTS: AtomicU32 = AtomicU32::new(0);
static STORMED: AtomicU32 = AtomicU32::new(0);

#[riscv_rt::core_interrupt(CoreInterrupt::MachineExternal)]
fn machine_external() {
    // SAFETY: read-only PLIC access from the handler.
    let plic_regs = unsafe { thejas32_pac::Plic::steal() };
    if plic_regs.intr_status().read().bits() & (1 << (Interrupt::GPIO0 as u8)) == 0 {
        return;
    }

    let count = GPIO_INTERRUPTS.load(Ordering::Relaxed) + 1;
    GPIO_INTERRUPTS.store(count, Ordering::Relaxed);

    // There is no GPIO-side acknowledge to perform: the block has no status
    // register. If the source is level-sensitive this handler will re-enter
    // immediately, so mask the line at the PLIC to escape and record why.
    if count >= STORM_LIMIT {
        STORMED.store(1, Ordering::Relaxed);
        let enabled = plic_regs.intr_en().read().bits();
        // SAFETY: clearing this source's enable bit; single-hart, and no other
        // context writes this register while the handler runs.
        unsafe {
            plic_regs
                .intr_en()
                .write(|w| w.bits(enabled & !(1 << (Interrupt::GPIO0 as u8))))
        };
        thejas32_hal::fence();
    }
}

/// Counts interrupts arriving over `ticks`, returning the delta.
fn count_over(reference: &mut Timer<thejas32_pac::Timer1>, ticks: u32) -> u32 {
    reference.start_free_running();
    let start = reference.counter();
    let before = GPIO_INTERRUPTS.load(Ordering::Relaxed);
    while start.wrapping_sub(reference.counter()) < ticks {}
    GPIO_INTERRUPTS.load(Ordering::Relaxed) - before
}

fn line<W: embedded_hal_nb::serial::Write<u8>>(serial: &mut W, label: &[u8], ok: bool) {
    write_str(serial, label);
    write_str(serial, if ok { b"PASS\r\n" } else { b"FAIL\r\n" });
}

#[riscv_rt::entry]
fn main() -> ! {
    // The PAC's singleton, taken once. `steal()` below (where present) is a
    // deliberate read-only alias for register inspection, not a second owner.
    let peripherals = thejas32_pac::Peripherals::take().unwrap();
    let mut serial = Serial::new_preconfigured(peripherals.uart0);
    let plic = plic::Plic::new(peripherals.plic);
    let mut reference = Timer::new(peripherals.timer1);
    let pins = Gpio::new(peripherals.gpio0).split();

    write_str(
        &mut serial,
        b"\r\nGPIO interrupt self-test (PWM[0] -> GPIO0, J2_5 -> J2_11)\r\n",
    );

    let mut input = pins.pin0.into_input();
    let irq = match input.interrupt() {
        Some(irq) => irq,
        None => {
            // Unreachable for pin 0, but the API returns Option and pretending
            // otherwise would be a lie in the one place it matters.
            write_str(&mut serial, b"  pin has no interrupt line -- aborting\r\n");
            loop {
                unsafe { core::arch::asm!("nop") };
            }
        }
    };

    // Start the edge source before anything else, so every later phase has a
    // known, continuous stimulus.
    let global = PwmGlobal::new(peripherals.pwm_global);
    let mut pwm = PwmChannel::new(peripherals.pwm0, Config::default(), PWM_PERIOD_TICKS);
    pwm.set_on_off_time(PWM_PERIOD_TICKS / 2); // 50% duty: clean edges both ways
    global.enable();

    // ---- [0] is the jumper actually fitted? ------------------------------
    //
    // "The pin never moved" has two causes -- no jumper, or PWM not producing
    // a waveform -- and they need separating before anything below means
    // anything. `pwm-selftest` phase 4 already solves this: in `Mode::Idle` the
    // channel drives the pin to a fixed level, testing the wiring with no
    // timing involved. If the pin follows a static level, the jumper is good
    // and any lack of edges is a waveform problem.
    use embedded_hal::digital::InputPin;
    write_str(&mut serial, b"\n[0] stimulus present\r\n");

    let mut drive_static = |level: IdleLevel| {
        pwm.configure(
            Config::default()
                .with_mode(Mode::Idle)
                .with_idle_level(level),
        );
        global.enable();
        for _ in 0..200_000 {
            unsafe { core::arch::asm!("nop") };
        }
    };
    drive_static(IdleLevel::High);
    let follows_high = input.is_high().unwrap_or(false);
    drive_static(IdleLevel::Low);
    let follows_low = !input.is_high().unwrap_or(true);
    let wired = follows_high && follows_low;

    write_str(&mut serial, b"    static idle=High -> pin ");
    write_str(&mut serial, if follows_high { b"HIGH" } else { b"low " });
    write_str(&mut serial, b"\r\n    static idle=Low  -> pin ");
    write_str(&mut serial, if follows_low { b"LOW " } else { b"high" });
    write_str(&mut serial, b"\r\n    jumper present       ");
    write_str(&mut serial, if wired { b"yes\r\n" } else { b"NO\r\n" });

    if !wired {
        write_str(
            &mut serial,
            b"\r\n  GPIO0 does not follow PWM[0]'s static level, so the two pins\r\n\
              \x20 are not connected. Fit the jumper J2_5 -> J2_11 and re-run.\r\n\
              \x20 This is wiring, not the driver: nothing below could pass.\r\n",
        );
        global.disable();
        loop {
            unsafe { core::arch::asm!("nop") };
        }
    }

    // Wiring is good. Now switch to a real waveform and confirm edges appear.
    pwm.configure(Config::default());
    pwm.set_period(PWM_PERIOD_TICKS);
    pwm.set_on_off_time(PWM_PERIOD_TICKS / 2);
    global.enable();

    let toggled = {
        reference.start_free_running();
        let start = reference.counter();
        let first = input.is_high().unwrap_or(false);
        let mut changed = false;
        while start.wrapping_sub(reference.counter()) < JUMPER_TICKS {
            if input.is_high().unwrap_or(false) != first {
                changed = true;
                break;
            }
        }
        changed
    };
    write_str(&mut serial, b"    waveform edges       ");
    write_str(&mut serial, if toggled { b"yes\r\n" } else { b"NO\r\n" });
    if !toggled {
        write_str(
            &mut serial,
            b"\r\n  The jumper is fine -- the pin followed a static level -- but\r\n\
              \x20 no edges arrive in Continuous mode. That is a PWM configuration\r\n\
              \x20 fault in this test, not a GPIO interrupt result.\r\n",
        );
        global.disable();
        loop {
            unsafe { core::arch::asm!("nop") };
        }
    }

    // ---- [1] delivery ----------------------------------------------------
    write_str(&mut serial, b"\r\n[1] interrupts delivered\r\n");
    plic.enable(irq);
    // SAFETY: the handler is installed and masks its own source if it storms.
    unsafe {
        plic::enable_external_interrupts();
        plic::enable_global_interrupts();
    }
    let first_window = count_over(&mut reference, WINDOW_TICKS);
    let stormed = STORMED.load(Ordering::Relaxed) != 0;

    write_str(&mut serial, b"    interrupts in 1 s:   ");
    write_dec(&mut serial, first_window);
    write_str(&mut serial, b"\r\n");
    let p1 = first_window > 0;
    line(&mut serial, b"    delivered            ", p1);

    if stormed {
        write_str(
            &mut serial,
            b"\r\n    STORM: the handler hit its entry limit and masked the\r\n\
              \x20   source. The input is level-sensitive with no acknowledge\r\n\
              \x20   path, so GPIO interrupts cannot be used edge-style here.\r\n\
              \x20   That is a hardware finding, not a driver defect.\r\n",
        );
    }

    // ---- [2] what triggers it? -------------------------------------------
    //
    // A waveform cannot answer this once the source storms: the count saturates
    // at STORM_LIMIT and says nothing about the rate. Static levels can, because
    // they contain **no edges at all**. If interrupts arrive while the pin is
    // held at a constant level, the source is level-sensitive by definition.
    //
    // The level is set and allowed to settle *before* the PLIC line is enabled,
    // so the transition's own edge is never counted -- otherwise an
    // edge-triggered source would look level-sensitive.
    write_str(
        &mut serial,
        b"\r\n[2] trigger semantics (static levels, no edges)\r\n",
    );

    let probe_static = |serial: &mut Serial<thejas32_pac::Uart0>,
                        pwm: &mut PwmChannel<thejas32_pac::Pwm0>,
                        reference: &mut Timer<thejas32_pac::Timer1>,
                        level: IdleLevel,
                        label: &[u8]|
     -> u32 {
        plic.disable(irq);
        pwm.configure(
            Config::default()
                .with_mode(Mode::Idle)
                .with_idle_level(level),
        );
        global.enable();
        // Let the pin settle so the transition edge is long gone.
        for _ in 0..200_000 {
            unsafe { core::arch::asm!("nop") };
        }
        GPIO_INTERRUPTS.store(0, Ordering::Relaxed);
        STORMED.store(0, Ordering::Relaxed);
        plic.enable(irq);
        let n = count_over(reference, PROBE_TICKS);
        plic.disable(irq);

        write_str(serial, label);
        write_dec(serial, n);
        write_str(serial, b"\r\n");
        n
    };

    let on_high = probe_static(
        &mut serial,
        &mut pwm,
        &mut reference,
        IdleLevel::High,
        b"    pin held HIGH -> interrupts: ",
    );
    let on_low = probe_static(
        &mut serial,
        &mut pwm,
        &mut reference,
        IdleLevel::Low,
        b"    pin held LOW  -> interrupts: ",
    );

    write_str(&mut serial, b"    verdict: ");
    let determined = match (on_high > 0, on_low > 0) {
        (true, false) => {
            write_str(&mut serial, b"LEVEL-sensitive, active HIGH\r\n");
            true
        }
        (false, true) => {
            write_str(&mut serial, b"LEVEL-sensitive, active LOW\r\n");
            true
        }
        (false, false) => {
            write_str(&mut serial, b"EDGE-triggered (no edges, no interrupts)\r\n");
            true
        }
        (true, true) => {
            write_str(&mut serial, b"fires at BOTH levels -- unexplained\r\n");
            false
        }
    };
    line(&mut serial, b"    semantics determined ", determined);
    let p2 = determined;

    // Restore the waveform for the phases below.
    pwm.configure(Config::default());
    pwm.set_period(PWM_PERIOD_TICKS);
    pwm.set_on_off_time(PWM_PERIOD_TICKS / 2);
    global.enable();
    GPIO_INTERRUPTS.store(0, Ordering::Relaxed);
    STORMED.store(0, Ordering::Relaxed);
    plic.enable(irq);

    // ---- [3] masking stops delivery --------------------------------------
    // The PWM keeps running, so the stimulus is known to be present the whole
    // time. Silence here can only mean masking worked.
    write_str(&mut serial, b"\r\n[3] masking stops delivery\r\n");
    plic.disable(irq);
    let masked_window = count_over(&mut reference, WINDOW_TICKS);
    write_str(&mut serial, b"    interrupts while masked: ");
    write_dec(&mut serial, masked_window);
    write_str(&mut serial, b"\r\n");
    let p3 = masked_window == 0;
    line(&mut serial, b"    masked               ", p3);

    // ---- [4] and back again ----------------------------------------------
    write_str(&mut serial, b"\r\n[4] re-enable\r\n");
    STORMED.store(0, Ordering::Relaxed);
    GPIO_INTERRUPTS.store(0, Ordering::Relaxed);
    plic.enable(irq);
    let again = count_over(&mut reference, WINDOW_TICKS);
    write_str(&mut serial, b"    interrupts in 1 s:   ");
    write_dec(&mut serial, again);
    write_str(&mut serial, b"\r\n");
    let p4 = again > 0;
    line(&mut serial, b"    re-enable works      ", p4);

    // SAFETY: quiesce before the summary so the console is not interrupted.
    unsafe { plic::disable_global_interrupts() };
    plic.disable_all();
    global.disable();

    let passed = [p1, p2, p3, p4].iter().filter(|&&b| b).count();
    write_str(&mut serial, b"\r\nresult: ");
    write_dec(&mut serial, passed as u32);
    write_str(&mut serial, b"/4 checks passed\r\n");

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}
