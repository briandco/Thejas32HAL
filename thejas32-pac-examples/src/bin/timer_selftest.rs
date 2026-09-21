//! Timer self-test — validates the timer driver over UART, no extra hardware.
//!
//! The interesting problem is finding an *independent* time reference. Timing
//! a timer against another timer only proves they agree with each other, not
//! that either ticks at the rate we think. Phase 5 solves this by measuring
//! against the **UART baud rate**: at 115200 8N1 a byte takes exactly 10 bit
//! times (86.8 us), and the serial link is already known good. If the timer
//! agrees with the UART, both are right about absolute time.
//!
//! Phase 4 specifically targets the bug in `vega-sdk`'s `timer_put_delay`,
//! which never clears `EOI`: a second delay can see the stale pending flag
//! and return immediately. Three back-to-back delays catch that — a broken
//! driver shows a normal first delay and near-zero second and third.

#![no_std]
#![no_main]

use panic_halt as _;

use thejas32_examples::console::{write_dec, write_str};
use thejas32_hal::{PERIPHERAL_CLOCK_HZ, serial::Serial, timer::Timer};

const POLL_LIMIT: u32 = 50_000_000;
/// Delay under test in phase 4: 1 ms at 100 MHz.
const DELAY_TICKS: u32 = 100_000;
/// Bytes transmitted for the phase 5 rate check.
const RATE_BYTES: u32 = 200;
const BAUD: u32 = 115_200;
/// 8N1 => 1 start + 8 data + 1 stop bit per byte.
const BITS_PER_BYTE: u32 = 10;
/// Tolerance for the rate comparison, in percent.
const TOLERANCE_PCT: u32 = 5;

#[riscv_rt::entry]
fn main() -> ! {
    // The PAC's singleton, taken once. `steal()` below (where present) is a
    // deliberate read-only alias for register inspection, not a second owner.
    let peripherals = thejas32_pac::Peripherals::take().unwrap();
    let mut serial = Serial::new_preconfigured(peripherals.uart0);
    let mut timer = Timer::new(peripherals.timer0);
    let mut reference = Timer::new(peripherals.timer1);

    write_str(&mut serial, b"\r\ntimer self-test (TIMER0, ref TIMER1)\r\n");

    // ---- phase 1: load count readback ------------------------------------
    // `start_periodic` takes a *period*, and programs half of it into the
    // load register to compensate for the hardware's late flag, so the
    // readback is expected to be half the requested value.
    const P1_PERIOD: u32 = 0x0012_3456;
    timer.start_periodic(P1_PERIOD);
    let readback = unsafe { thejas32_pac::Timer0::steal() }
        .load_count()
        .read()
        .bits();
    timer.stop();
    let p1 = readback == P1_PERIOD / 2;
    line(&mut serial, b"[1] load count readback   ", p1);

    // ---- phase 2: counter runs, and counts down --------------------------
    reference.start_free_running();
    let a = reference.counter();
    spin(2_000);
    let b = reference.counter();
    let moved = a != b;
    let counts_down = a.wrapping_sub(b) < 0x8000_0000; // down-counter
    line(&mut serial, b"[2] counter advances      ", moved);
    line(&mut serial, b"    counts downward       ", counts_down);

    // ---- phase 3: expiry flag sets, and clears ---------------------------
    timer.start_periodic(10_000);
    let set_ok = poll(POLL_LIMIT, || timer.is_expired());
    timer.stop();
    // stop() clears EOI; the flag must now read false again. If EOI did not
    // clear it, every later delay would return instantly.
    let clear_ok = !timer.is_expired();
    line(&mut serial, b"[3] expiry flag sets      ", set_ok);
    line(&mut serial, b"    and clears via EOI    ", clear_ok);

    // ---- phase 4: back-to-back delays are each timed ----------------------
    let mut measured = [0u32; 3];
    for slot in measured.iter_mut() {
        reference.start_free_running();
        let start = reference.counter();
        timer.delay_ticks(DELAY_TICKS);
        let end = reference.counter();
        *slot = start.wrapping_sub(end);
    }

    // Each must land near the requested duration; a stale-flag bug shows up
    // as a correct first delay followed by near-zero ones.
    let lo = DELAY_TICKS - DELAY_TICKS / 10;
    let hi = DELAY_TICKS + DELAY_TICKS / 10;
    let p4 = measured.iter().all(|&m| m >= lo && m <= hi);
    line(&mut serial, b"[4] 3 delays each ~1ms    ", p4);
    for (i, &m) in measured.iter().enumerate() {
        write_str(&mut serial, b"    delay ");
        write_dec(&mut serial, i as u32 + 1);
        write_str(&mut serial, b": ");
        write_dec(&mut serial, m);
        write_str(&mut serial, b" ticks (want ");
        write_dec(&mut serial, DELAY_TICKS);
        write_str(&mut serial, b")\r\n");
    }

    // ---- phase 5: absolute rate vs UART baud ------------------------------
    // Measure the wall time to shift out a known number of bytes. Flushing to
    // TEMT means the last byte has fully left the shift register, so the
    // result does not depend on FIFO depth.
    reference.start_free_running();
    let start = reference.counter();
    for _ in 0..RATE_BYTES {
        write_str(&mut serial, b".");
    }
    serial.flush();
    let end = reference.counter();
    let elapsed = start.wrapping_sub(end);

    let expected = (PERIPHERAL_CLOCK_HZ as u64 * RATE_BYTES as u64 * BITS_PER_BYTE as u64
        / BAUD as u64) as u32;
    let (bigger, smaller) = if elapsed > expected {
        (elapsed, expected)
    } else {
        (expected, elapsed)
    };
    let deviation_pct = ((bigger - smaller) as u64 * 100 / expected as u64) as u32;
    let p5 = deviation_pct <= TOLERANCE_PCT;

    write_str(&mut serial, b"\r\n[5] rate vs UART baud\r\n");
    write_str(&mut serial, b"    measured: ");
    write_dec(&mut serial, elapsed);
    write_str(&mut serial, b" ticks\r\n    expected: ");
    write_dec(&mut serial, expected);
    write_str(&mut serial, b" ticks\r\n    deviation: ");
    write_dec(&mut serial, deviation_pct);
    write_str(&mut serial, b"%\r\n");
    line(&mut serial, b"    within tolerance      ", p5);

    // Implied clock, derived purely from the UART reference.
    let implied_hz =
        (elapsed as u64 * BAUD as u64 / (RATE_BYTES as u64 * BITS_PER_BYTE as u64)) as u32;
    write_str(&mut serial, b"    implied clock: ");
    write_dec(&mut serial, implied_hz / 1_000_000);
    write_str(&mut serial, b" MHz\r\n");

    // ---- phase 6: steady-state period without re-arming -------------------
    // Phases 3-4 only exercise the first flag after arming. A periodic timer
    // is also expected to keep flagging at the same interval, so measure the
    // gap between successive flags with the timer left running.
    timer.start_periodic(DELAY_TICKS);
    reference.start_free_running();
    while !timer.is_expired() {}
    timer.clear_expired();
    let mut mark = reference.counter();
    let mut intervals = [0u32; 3];
    for slot in intervals.iter_mut() {
        while !timer.is_expired() {}
        timer.clear_expired();
        let now = reference.counter();
        *slot = mark.wrapping_sub(now);
        mark = now;
    }
    timer.stop();

    let p6 = intervals.iter().all(|&m| m >= lo && m <= hi);
    write_str(&mut serial, b"\r\n[6] repeat interval (no re-arm)\r\n");
    for (i, &m) in intervals.iter().enumerate() {
        write_str(&mut serial, b"    gap ");
        write_dec(&mut serial, i as u32 + 1);
        write_str(&mut serial, b": ");
        write_dec(&mut serial, m);
        write_str(&mut serial, b" ticks (want ");
        write_dec(&mut serial, DELAY_TICKS);
        write_str(&mut serial, b")\r\n");
    }
    line(&mut serial, b"    steady period ok      ", p6);

    let passed = [p1, moved, counts_down, set_ok, clear_ok, p4, p5, p6]
        .iter()
        .filter(|&&b| b)
        .count();
    write_str(&mut serial, b"\r\nresult: ");
    write_dec(&mut serial, passed as u32);
    write_str(&mut serial, b"/8 checks passed\r\n");

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}

fn line<W: embedded_hal_nb::serial::Write<u8>>(serial: &mut W, label: &[u8], ok: bool) {
    write_str(serial, label);
    write_str(serial, if ok { b"PASS\r\n" } else { b"FAIL\r\n" });
}

fn poll<F: Fn() -> bool>(limit: u32, cond: F) -> bool {
    for _ in 0..limit {
        if cond() {
            return true;
        }
    }
    false
}

#[inline(never)]
fn spin(n: u32) {
    for _ in 0..n {
        unsafe { core::arch::asm!("nop") };
    }
}
