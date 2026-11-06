//! PWM self-test — validates the PWM driver over UART.
//!
//! PWM is an output, so unlike the input peripherals there is nothing to read
//! back to prove the waveform. Phases 1-3 check what registers can show;
//! phase 4 measures the real signal, and needs one jumper.
//!
//! 1. **Control word encoding** — several configurations written and read
//!    back, checking mode/alignment/idle-level/repeat land in the right bits.
//! 2. **Period and on-time** — the two data registers hold what was written.
//! 3. **Global enable gates the channel** — a configured channel must report
//!    inactive until the global enable is set, and active after. This is the
//!    trap worth catching: a channel configured without the global enable sits
//!    silent with no error.
//! 4. **Waveform measurement** *(needs a jumper)* — with **PWM\[0\] wired to
//!    GPIO0**, the pin is sampled as an input and edges are timestamped with
//!    the already-validated timer, giving measured high-time, period and duty
//!    against what was configured. Three validated drivers (GPIO, Timer, UART)
//!    are used to check the fourth.
//!
//! Without the jumper phase 4 reports no signal rather than failing silently,
//! and the verdict says the waveform is unproven instead of implying it passed.

#![no_std]
#![no_main]

use panic_halt as _;

use embedded_hal::{digital::InputPin, pwm::SetDutyCycle};
use thejas32_examples::console::{write_dec, write_hex, write_str};
use thejas32_hal::{
    gpio::Gpio,
    pwm::{Alignment, Config, IdleLevel, Mode, PwmChannel, PwmGlobal},
    serial::Serial,
    timer::Timer,
};

/// 1 kHz at 100 MHz — slow enough to sample comfortably in software.
const PERIOD: u32 = 100_000;
/// Bound on edge waits; a full period is ~33k loop iterations.
const EDGE_LIMIT: u32 = 2_000_000;

#[riscv_rt::entry]
fn main() -> ! {
    // The PAC's singleton, taken once. `steal()` below (where present) is a
    // deliberate read-only alias for register inspection, not a second owner.
    let peripherals = thejas32_pac::Peripherals::take().unwrap();
    let mut serial = Serial::new_preconfigured(peripherals.uart0);
    let global = PwmGlobal::new(peripherals.pwm_global);
    let mut ch = PwmChannel::new(peripherals.pwm0, Config::default(), PERIOD);
    let mut reference = Timer::new(peripherals.timer1);

    write_str(&mut serial, b"\r\npwm self-test (PWM0 @ 1 kHz)\r\n");

    // ---- [1] control word encoding ---------------------------------------
    let cases = [
        Config::default(),
        Config::default()
            .with_mode(Mode::OneShot)
            .with_alignment(Alignment::Right)
            .with_idle_level(IdleLevel::High)
            .with_interrupt(true)
            .with_repeat_count(5),
        // Left / low / no-interrupt are already the defaults.
        Config::default()
            .with_mode(Mode::Idle)
            .with_repeat_count(0x1234),
    ];

    let mut p1 = true;
    write_str(&mut serial, b"\r\n[1] control word encoding\r\n");
    for cfg in cases {
        ch.configure(cfg);
        let got = ch.control_bits();
        let want = (cfg.mode as u32)
            | ((cfg.alignment as u32) << 2)
            | ((cfg.interrupt as u32) << 4)
            | (((cfg.idle_level == IdleLevel::High) as u32) << 5)
            | ((cfg.repeat_count as u32) << 6);
        let ok = got == want;
        p1 &= ok;
        write_str(&mut serial, b"    mode=");
        write_dec(&mut serial, cfg.mode as u32);
        write_str(&mut serial, b" align=");
        write_dec(&mut serial, cfg.alignment as u32);
        write_str(&mut serial, b" rpt=");
        write_dec(&mut serial, cfg.repeat_count as u32);
        write_str(&mut serial, b"  ctrl=");
        write_hex(&mut serial, got, 8);
        write_str(&mut serial, b" (want ");
        write_hex(&mut serial, want, 8);
        write_str(
            &mut serial,
            if ok { b")  ok\r\n" } else { b")  MISMATCH\r\n" },
        );
    }
    line(&mut serial, b"    encoding correct     ", p1);

    // ---- [2] period and on-time ------------------------------------------
    ch.configure(Config::default());
    ch.set_period(PERIOD);
    let raw = unsafe { thejas32_pac::Pwm0::steal() };

    // Direct tick count must be exact.
    ch.set_on_off_time(PERIOD / 4);
    let exact_rb = raw.on_off().read().bits();

    // The percent path goes through two truncating divisions (the trait's own
    // `50 * 65535 / 100 = 32767`, then our scale back up), so it lands within
    // one part in 65535 rather than exactly. Asserting equality here was wrong;
    // allow 0.1%.
    ch.set_duty_cycle_percent(50).ok();
    let period_rb = raw.period().read().bits();
    let onoff_rb = raw.on_off().read().bits();
    let want = PERIOD / 2;
    let tolerance = PERIOD / 1000;
    let near = onoff_rb.abs_diff(want) <= tolerance;
    let p2 = period_rb == PERIOD && exact_rb == PERIOD / 4 && near;

    write_str(&mut serial, b"\r\n[2] period / on-time\r\n    period:     ");
    write_dec(&mut serial, period_rb);
    write_str(&mut serial, b" (want ");
    write_dec(&mut serial, PERIOD);
    write_str(&mut serial, b")\r\n    on (exact): ");
    write_dec(&mut serial, exact_rb);
    write_str(&mut serial, b" (want ");
    write_dec(&mut serial, PERIOD / 4);
    write_str(&mut serial, b")\r\n    on (50%):   ");
    write_dec(&mut serial, onoff_rb);
    write_str(&mut serial, b" (want ");
    write_dec(&mut serial, want);
    write_str(&mut serial, b" +/- ");
    write_dec(&mut serial, tolerance);
    write_str(&mut serial, b")\r\n");
    line(&mut serial, b"    registers hold values", p2);

    // ---- [3] global enable gates the channel -----------------------------
    global.disable();
    let active_before = ch.is_active();
    global.enable();
    let active_after = ch.is_active();
    let p3 = !active_before && active_after;

    write_str(
        &mut serial,
        b"\r\n[3] global enable gates channel\r\n    disabled: active=",
    );
    write_dec(&mut serial, active_before as u32);
    write_str(&mut serial, b" (want 0)\r\n    enabled:  active=");
    write_dec(&mut serial, active_after as u32);
    write_str(&mut serial, b" (want 1)\r\n    gcr=");
    write_hex(&mut serial, global.bits(), 8);
    write_str(&mut serial, b"\r\n");
    line(&mut serial, b"    gating works          ", p3);

    // ---- [4] electrical path: PWM as a static level -----------------------
    // Borrowed from vega-sdk's `pwm_as_gpio`: in Idle mode the OPC bit drives
    // the pin to a fixed level. That tests the wiring with no timing involved,
    // so it separates "the jumper/pin is wrong" from "the waveform is wrong".
    write_str(
        &mut serial,
        b"\r\n[4] static level via PWM idle (jumper J2_5 PWM[0] -> J2_11 GPIO0)\r\n",
    );
    let pins = Gpio::new(peripherals.gpio0).split();
    let mut pin = pins.pin0.into_input();

    let drive = |ch: &mut PwmChannel<thejas32_pac::Pwm0>, level: IdleLevel| {
        ch.configure(
            Config::default()
                .with_mode(Mode::Idle)
                .with_idle_level(level),
        );
        global.enable();
        for _ in 0..200_000 {
            unsafe { core::arch::asm!("nop") };
        }
    };

    drive(&mut ch, IdleLevel::High);
    let reads_high = pin.is_high().unwrap_or(false);
    drive(&mut ch, IdleLevel::Low);
    let reads_low = !pin.is_high().unwrap_or(true);
    let p4 = reads_high && reads_low;

    write_str(&mut serial, b"    idle=High -> pin ");
    write_str(&mut serial, if reads_high { b"HIGH" } else { b"low " });
    write_str(&mut serial, b" (want HIGH)\r\n    idle=Low  -> pin ");
    write_str(&mut serial, if reads_low { b"LOW " } else { b"high" });
    write_str(&mut serial, b" (want LOW)\r\n");
    line(&mut serial, b"    electrical path ok    ", p4);

    if !p4 {
        write_str(
            &mut serial,
            b"    PWM[0] does not reach GPIO0. Check the jumper is on\r\n\
              \x20   J2 pin 5 and J2 pin 11 before reading anything into [5].\r\n",
        );
    }

    // ---- [5] waveform measurement ----------------------------------------
    write_str(&mut serial, b"\r\n[5] waveform\r\n");
    ch.configure(Config::default());
    ch.set_period(PERIOD);
    ch.set_duty_cycle_percent(50).ok();
    global.enable();
    write_str(&mut serial, b"    channel active: ");
    write_dec(&mut serial, ch.is_active() as u32);
    write_str(&mut serial, b"\r\n");
    reference.start_free_running();

    // Establish that the pin is moving at all before trying to time it.
    let mut saw_high = false;
    let mut saw_low = false;
    for _ in 0..EDGE_LIMIT {
        if pin.is_high().unwrap_or(false) {
            saw_high = true;
        } else {
            saw_low = true;
        }
        if saw_high && saw_low {
            break;
        }
    }

    if !(saw_high && saw_low) {
        write_str(&mut serial, b"    pin never changed (stuck ");
        write_str(&mut serial, if saw_high { b"high" } else { b"low" });
        write_str(
            &mut serial,
            b")\r\n    NO SIGNAL -- jumper missing, or PWM not reaching the pin.\r\n\
              \x20   Waveform unproven; phases 1-3 above still stand.\r\n",
        );
    } else {
        // Start from a known level, then timestamp three consecutive edges.
        let _ = wait_for(&mut pin, false, EDGE_LIMIT);
        let ok_rise = wait_for(&mut pin, true, EDGE_LIMIT);
        let t_rise = reference.counter();
        let ok_fall = wait_for(&mut pin, false, EDGE_LIMIT);
        let t_fall = reference.counter();
        let ok_rise2 = wait_for(&mut pin, true, EDGE_LIMIT);
        let t_rise2 = reference.counter();

        if ok_rise && ok_fall && ok_rise2 {
            let high = t_rise.wrapping_sub(t_fall);
            let period = t_rise.wrapping_sub(t_rise2);
            let duty = if period == 0 { 0 } else { high * 100 / period };

            write_str(&mut serial, b"    high:   ");
            write_dec(&mut serial, high);
            write_str(&mut serial, b" ticks (want ~");
            write_dec(&mut serial, PERIOD / 2);
            write_str(&mut serial, b")\r\n    period: ");
            write_dec(&mut serial, period);
            write_str(&mut serial, b" ticks (want ~");
            write_dec(&mut serial, PERIOD);
            write_str(&mut serial, b")\r\n    duty:   ");
            write_dec(&mut serial, duty);
            write_str(&mut serial, b"% (want ~50%)\r\n");

            let lo = PERIOD - PERIOD / 10;
            let hi = PERIOD + PERIOD / 10;
            let p4 = period >= lo && period <= hi && (40..=60).contains(&duty);
            line(&mut serial, b"    waveform correct     ", p4);
        } else {
            write_str(
                &mut serial,
                b"    edge timing timed out -- signal unstable\r\n",
            );
        }
    }

    global.disable();
    ch.stop();

    let passed = [p1, p2, p3, p4].iter().filter(|&&b| b).count();
    write_str(&mut serial, b"\r\nresult: ");
    write_dec(&mut serial, passed as u32);
    write_str(&mut serial, b"/4 (3 register + 1 electrical)\r\n");

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}

/// Waits, bounded, for the pin to reach `level`.
fn wait_for<P: InputPin>(pin: &mut P, level: bool, limit: u32) -> bool {
    for _ in 0..limit {
        if pin.is_high().unwrap_or(false) == level {
            return true;
        }
    }
    false
}

fn line<W: embedded_hal_nb::serial::Write<u8>>(serial: &mut W, label: &[u8], ok: bool) {
    write_str(serial, label);
    write_str(serial, if ok { b"PASS\r\n" } else { b"FAIL\r\n" });
}
