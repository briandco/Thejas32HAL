//! GPIO self-test — verifies the bit-banded GPIO driver over UART alone.
//!
//! No LEDs, probes, or external wiring required. For each pin it drives the
//! pin through its *bit-banded* address, then bulk-reads all 16 pins and
//! checks that **exactly** the expected bit changed. That cross-check is what
//! makes this meaningful: if the per-pin address arithmetic were wrong, a
//! different bit would move and the mismatch is reported, whereas a
//! single-pin readback could pass by coincidence.
//!
//! Scope: bank 1, pins 0-8 only — the pins the existing `led_blink` example
//! already drives, so they are known safe to configure as outputs. Pins 9-15
//! are left alone rather than risk driving against something else on the
//! board.
//!
//! Caveat: reads of an output pin return the output latch, so a PASS proves
//! the driver's register and address logic are correct. It does not prove the
//! physical pin voltage moved — that still needs a meter or a load.

#![no_std]
#![no_main]

use panic_halt as _;

use embedded_hal::digital::OutputPin;
use embedded_hal_nb::{nb, serial::Write};
use thejas32_hal::{
    gpio::{Gpio, GpioBank},
    serial::Serial,
};

/// Pins under test: bank 1, pins 0-8.
const TEST_MASK: u16 = 0x01FF;

macro_rules! test_pin {
    ($pin:expr, $serial:expr, $n:literal, $passed:expr) => {{
        // The pin arrives already owned, from `split()`. Bank-wide reads come
        // from the associated functions, which need no live bank handle -- the
        // bank was consumed when it was split.
        let mut pin = $pin.into_output();
        let expect = 1u16 << $n;

        let dir = <thejas32_pac::Gpio1 as GpioBank>::direction_of();
        pin.set_high().ok();
        let high = <thejas32_pac::Gpio1 as GpioBank>::read_all_of() & TEST_MASK;
        pin.set_low().ok();
        let low = <thejas32_pac::Gpio1 as GpioBank>::read_all_of() & TEST_MASK;

        // Compare high against low rather than against zero: pins not yet
        // configured as outputs float and may read high (bits 6 and 8 do on
        // this board), so an absolute expectation gives false failures. The
        // differential check is also the stronger cross-talk test — it
        // requires that toggling this pin moved exactly one bit, whatever the
        // surrounding pins happen to be doing.
        let changed = high ^ low;
        let ok = (dir & expect) != 0     // direction bit applied
            && changed == expect          // exactly this pin moved, no other
            && (high & expect) != 0; // and set_high drove it high, not low
        if ok {
            $passed += 1;
        }

        write_str($serial, b"pin ");
        nb::block!($serial.write(b'0' + $n)).ok();
        write_str($serial, b"  dir=");
        write_hex($serial, dir);
        write_str($serial, b"  hi=");
        write_hex($serial, high);
        write_str($serial, b"  lo=");
        write_hex($serial, low);
        write_str($serial, b"  chg=");
        write_hex($serial, changed);
        write_str($serial, if ok { b"  PASS\r\n" } else { b"  FAIL\r\n" });
    }};
}

#[riscv_rt::entry]
fn main() -> ! {
    // `take()` rather than `steal()`: this is the singleton the PAC exists to
    // provide, and taking it once here means no second driver can be built for
    // the same peripheral. It cannot fail on the first call in `main`.
    let peripherals = thejas32_pac::Peripherals::take().unwrap();
    let mut serial = Serial::new_preconfigured(peripherals.uart0);
    let pins = Gpio::new(peripherals.gpio1).split();

    write_str(&mut serial, b"\r\ngpio self-test (bank 1, pins 0-8)\r\n");

    let mut passed = 0u8;
    test_pin!(pins.pin0, &mut serial, 0, passed);
    test_pin!(pins.pin1, &mut serial, 1, passed);
    test_pin!(pins.pin2, &mut serial, 2, passed);
    test_pin!(pins.pin3, &mut serial, 3, passed);
    test_pin!(pins.pin4, &mut serial, 4, passed);
    test_pin!(pins.pin5, &mut serial, 5, passed);
    test_pin!(pins.pin6, &mut serial, 6, passed);
    test_pin!(pins.pin7, &mut serial, 7, passed);
    test_pin!(pins.pin8, &mut serial, 8, passed);

    write_str(&mut serial, b"result: ");
    nb::block!(serial.write(b'0' + passed)).ok();
    write_str(&mut serial, b"/9 passed\r\n");

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}

fn write_str<W: Write<u8>>(serial: &mut W, bytes: &[u8]) {
    for &byte in bytes {
        nb::block!(serial.write(byte)).ok();
    }
}

/// Writes a u16 as four hex digits.
fn write_hex<W: Write<u8>>(serial: &mut W, value: u16) {
    for shift in [12, 8, 4, 0] {
        let nibble = ((value >> shift) & 0xF) as u8;
        let ch = if nibble < 10 {
            b'0' + nibble
        } else {
            b'a' + (nibble - 10)
        };
        nb::block!(serial.write(ch)).ok();
    }
}
