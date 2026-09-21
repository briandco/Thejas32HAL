//! Scans all three I2C buses for devices.
//!
//! Two questions at once.
//!
//! **Is `0x1000_1000` an I2C controller or an ADC?** The vendor SDK says both.
//! `config.h` calls it `ADC_BASE_ADDRESS` and `adc.h` declares a three-register
//! ADC block there; `i2c.h` says `#define I2C_ADC 8  // at address 0x1000_1000`
//! and reaches it through the generic `I2CReg(i)` macro, which has no bounds
//! check. Only one of those is backed by working code: `adc_analogRead` is
//! declared and never implemented anywhere in the SDK, while the I2C reading is
//! exercised by `examples/i2c/industrial_robot_demo`. Our own `adc-selftest`
//! found the block unresponsive when driven with ADC register semantics, which
//! is consistent with it being an I2C controller.
//!
//! A device acknowledging on that bus settles it. Nothing else would explain an
//! ACK, since an ADC block would not implement the I2C state machine.
//!
//! **Where is the board's analog input?** The ARIES V3.0's A0–A3 are a TI
//! ADS1015 12-bit I2C ADC — the SoC package exposes no analog pins. The ADS1015
//! answers at **0x48** by default (0x49/0x4A/0x4B if ADDR is strapped to VDD,
//! SDA or SCL). Finding it is what makes analog input reachable at all, and it
//! finally supplies a slave device for the one driver path still unvalidated:
//! I2C's ACK handling. Every I2C test so far has run against an empty bus —
//! 112 NACKs, zero devices — so successful transfers, multi-byte writes and
//! reads have never been exercised on hardware.
//!
//! Scanning is non-destructive: each probe is an address-only write, and a NACK
//! is an expected answer rather than an error.

#![no_std]
#![no_main]

use panic_halt as _;

use thejas32_examples::console::{write_dec, write_hex, write_str};
use thejas32_hal::{
    PERIPHERAL_CLOCK_HZ,
    i2c::{I2c, I2cX},
    serial::Serial,
};

/// Standard-mode bus clock. The ADS1015 also supports 400 kHz, but nothing is
/// gained by rushing a scan, and 100 kHz is the most forgiving of weak pull-ups.
const BUS_HZ: u32 = 100_000;

/// Reserved ranges are skipped, per the I2C specification: 0x00-0x07 and
/// 0x78-0x7F. That leaves 112 addresses, matching earlier scans.
const FIRST: u8 = 0x08;
const LAST: u8 = 0x77;

/// Addresses an ADS1015 can occupy, by ADDR pin strap.
const ADS1015_ADDRESSES: [u8; 4] = [0x48, 0x49, 0x4A, 0x4B];

/// A bus with more devices than this is not credible on this board and almost
/// certainly means the controller is answering its own probes — the failure
/// mode that once produced 112 phantom devices before the NACK timing was
/// understood.
const PLAUSIBLE_MAX: u32 = 8;

fn scan<I: I2cX, W: embedded_hal_nb::serial::Write<u8>>(
    serial: &mut W,
    i2c: &mut I2c<I>,
    label: &[u8],
) -> u32 {
    write_str(serial, b"\r\n  ");
    write_str(serial, label);
    write_str(serial, b"\r\n    devices: ");

    let mut found = 0;
    let mut errors = 0;
    for address in FIRST..=LAST {
        match i2c.probe(address) {
            Ok(true) => {
                found += 1;
                write_str(serial, b"0x");
                write_hex(serial, address as u32, 2);
                // Name it if it is what we came for.
                if ADS1015_ADDRESSES.contains(&address) {
                    write_str(serial, b"(ADS1015?)");
                }
                write_str(serial, b" ");
            }
            Ok(false) => {}
            Err(_) => errors += 1,
        }
    }

    if found == 0 {
        write_str(serial, b"none");
    }
    write_str(serial, b"\r\n    found: ");
    write_dec(serial, found);
    write_str(serial, b"   bus errors: ");
    write_dec(serial, errors);
    if found > PLAUSIBLE_MAX {
        write_str(
            serial,
            b"   IMPLAUSIBLE -- suspect the NACK path, not the bus",
        );
    }
    write_str(serial, b"\r\n");
    found
}

#[riscv_rt::entry]
fn main() -> ! {
    // The PAC's singleton, taken once. `steal()` below (where present) is a
    // deliberate read-only alias for register inspection, not a second owner.
    let peripherals = thejas32_pac::Peripherals::take().unwrap();
    let mut serial = Serial::new_preconfigured(peripherals.uart0);
    write_str(
        &mut serial,
        b"\r\ni2c bus scan -- all three controllers\r\n",
    );

    let mut i2c0 = I2c::new(peripherals.i2c0, PERIPHERAL_CLOCK_HZ, BUS_HZ);
    let mut i2c1 = I2c::new(peripherals.i2c1, PERIPHERAL_CLOCK_HZ, BUS_HZ);
    let mut i2c2 = I2c::new(peripherals.i2c2, PERIPHERAL_CLOCK_HZ, BUS_HZ);

    let n0 = scan(&mut serial, &mut i2c0, b"I2C0 @ 0x1000_0800");
    let n1 = scan(&mut serial, &mut i2c1, b"I2C1 @ 0x1000_0900");
    let n2 = scan(
        &mut serial,
        &mut i2c2,
        b"I2C2 @ 0x1000_1000  (vendor I2C_ADC)",
    );

    // ---- what the result means -------------------------------------------
    write_str(&mut serial, b"\r\ninterpretation:\r\n");
    let total = n0 + n1 + n2;
    if n2 > 0 && n2 <= PLAUSIBLE_MAX {
        write_str(
            &mut serial,
            b"  A device acknowledged on 0x1000_1000. That address implements\r\n\
              \x20 the I2C state machine, so it is a controller and not an ADC\r\n\
              \x20 register block -- adc.rs models the wrong thing.\r\n",
        );
    } else if total > 0 && total <= PLAUSIBLE_MAX {
        write_str(
            &mut serial,
            b"  Devices answered, but not on 0x1000_1000. The ADS1015 may sit\r\n\
              \x20 on another bus; note which address answered where.\r\n",
        );
    } else if total == 0 {
        write_str(
            &mut serial,
            b"  Nothing answered anywhere. Either the ADS1015 is not populated\r\n\
              \x20 on this board revision, or the bus lacks pull-ups. This does\r\n\
              \x20 NOT settle what 0x1000_1000 is -- silence is not evidence.\r\n",
        );
    } else {
        write_str(
            &mut serial,
            b"  Implausible device count: the NACK path is reporting ACKs.\r\n\
              \x20 Fix that before drawing any conclusion from this scan.\r\n",
        );
    }

    write_str(&mut serial, b"\r\nscan complete\r\n");
    loop {
        unsafe { core::arch::asm!("nop") };
    }
}
