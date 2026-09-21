//! Clock probe — measures the UART input clock from the bootloader's setup.
//!
//! The ROM bootloader configures UART0 for a working 115200 baud before
//! handing over, so the divisor it left in the latch encodes the clock:
//!
//! ```text
//! clock_hz = divisor * 115200 * 16
//! ```
//!
//! This measures the clock rather than trusting a document. The two candidate
//! values in circulation are far enough apart to be unambiguous:
//!
//! | divisor | implied clock | matches |
//! |---|---|---|
//! | 21 | 38.7 MHz | `vega-sdk`'s 40 MHz |
//! | 54 | 99.5 MHz | Zephyr/`TrustedV_RTOS`'s 100 MHz |
//!
//! Printing happens at the bootloader's own baud rate, which is known good,
//! so the output is readable regardless of which answer is correct.

#![no_std]
#![no_main]

use panic_halt as _;

use embedded_hal_nb::{nb, serial::Write};
use thejas32_hal::serial::Serial;

/// The baud rate the bootloader configured, per its own banner.
const ASSUMED_BAUD: u32 = 115_200;

#[riscv_rt::entry]
fn main() -> ! {
    // The PAC's singleton, taken once. `steal()` below (where present) is a
    // deliberate read-only alias for register inspection, not a second owner.
    let peripherals = thejas32_pac::Peripherals::take().unwrap();
    let mut serial = Serial::new_preconfigured(peripherals.uart0);

    let divisor = serial.divisor();
    let clock = divisor as u32 * ASSUMED_BAUD * 16;

    write_str(&mut serial, b"\r\nclock probe\r\n");

    write_str(&mut serial, b"  bootloader divisor : ");
    write_dec(&mut serial, divisor as u32);
    write_str(&mut serial, b"\r\n");

    write_str(&mut serial, b"  assumed baud       : ");
    write_dec(&mut serial, ASSUMED_BAUD);
    write_str(&mut serial, b"\r\n");

    write_str(&mut serial, b"  => uart clock      : ");
    write_dec(&mut serial, clock);
    write_str(&mut serial, b" Hz\r\n");

    // Round to the nearest MHz for readability.
    write_str(&mut serial, b"  => approx          : ");
    write_dec(&mut serial, (clock + 500_000) / 1_000_000);
    write_str(&mut serial, b" MHz\r\n");

    // Close the loop: reprogram the divisor ourselves using the measured
    // clock. Everything above printed at the bootloader's baud rate; if the
    // line below is also readable, then `Serial::new`'s divisor arithmetic is
    // correct against real hardware, and `PERIPHERAL_CLOCK_HZ` is usable for
    // the peripherals that must compute their own dividers.
    let uart = serial.free();
    let mut serial = Serial::new(uart, ASSUMED_BAUD, thejas32_hal::PERIPHERAL_CLOCK_HZ);
    write_str(
        &mut serial,
        b"  reconfigured via Serial::new -- if this is readable, it works\r\n",
    );

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}

fn write_str<W: Write<u8>>(serial: &mut W, bytes: &[u8]) {
    for &byte in bytes {
        nb::block!(serial.write(byte)).ok();
    }
}

/// Writes a u32 in decimal.
fn write_dec<W: Write<u8>>(serial: &mut W, mut value: u32) {
    if value == 0 {
        nb::block!(serial.write(b'0')).ok();
        return;
    }
    let mut buf = [0u8; 10];
    let mut len = 0;
    while value > 0 {
        buf[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    while len > 0 {
        len -= 1;
        nb::block!(serial.write(buf[len])).ok();
    }
}
