//! GPIO walking-LED — lights each of the 9 onboard LEDs in turn.
//!
//! Uses `thejas32-hal`'s type-state GPIO driver rather than raw PAC register
//! access (compare with `led_blink.rs`, which bulk-writes all pins at once).
//!
//! A walking pattern exercises each pin's bit-banded address individually, so
//! incorrect address arithmetic shows up as the wrong LED lighting rather
//! than passing unnoticed. UART0 reports the active pin so the run can be
//! verified over serial as well as visually.

#![no_std]
#![no_main]

use panic_halt as _;

use core::convert::Infallible;
use embedded_hal::digital::OutputPin;
use embedded_hal_nb::{nb, serial::Write};
use thejas32_hal::{gpio::Gpio, serial::Serial};

/// Onboard LEDs are bank 1, pins 0-8.
const LED_COUNT: usize = 9;

#[riscv_rt::entry]
fn main() -> ! {
    // The PAC's singleton, taken once. `steal()` below (where present) is a
    // deliberate read-only alias for register inspection, not a second owner.
    let peripherals = thejas32_pac::Peripherals::take().unwrap();
    let mut serial = Serial::new_preconfigured(peripherals.uart0);
    let pins = Gpio::new(peripherals.gpio1).split();

    let mut p0 = pins.pin0.into_output();
    let mut p1 = pins.pin1.into_output();
    let mut p2 = pins.pin2.into_output();
    let mut p3 = pins.pin3.into_output();
    let mut p4 = pins.pin4.into_output();
    let mut p5 = pins.pin5.into_output();
    let mut p6 = pins.pin6.into_output();
    let mut p7 = pins.pin7.into_output();
    let mut p8 = pins.pin8.into_output();

    // Distinct types per pin, unified behind the embedded-hal trait.
    let mut pins: [&mut dyn OutputPin<Error = Infallible>; LED_COUNT] = [
        &mut p0, &mut p1, &mut p2, &mut p3, &mut p4, &mut p5, &mut p6, &mut p7, &mut p8,
    ];

    write_str(&mut serial, b"\r\ngpio-walk ready\r\n");

    loop {
        for (i, pin) in pins.iter_mut().enumerate() {
            pin.set_high().ok();
            write_str(&mut serial, b"led ");
            nb::block!(serial.write(b'0' + i as u8)).ok();
            write_str(&mut serial, b"\r\n");
            delay();
            pin.set_low().ok();
        }
    }
}

fn write_str<W: Write<u8>>(serial: &mut W, bytes: &[u8]) {
    for &byte in bytes {
        nb::block!(serial.write(byte)).ok();
    }
}

#[inline(never)]
fn delay() {
    for _ in 0..0x40_0000u32 {
        unsafe { core::arch::asm!("nop") };
    }
}
