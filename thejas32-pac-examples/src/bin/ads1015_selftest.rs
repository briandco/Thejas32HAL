//! ADS1015 self-test — analog input, and the I2C ACK path at last.
//!
//! Two things are validated here for the first time on this hardware.
//!
//! **The I2C ACK path.** Every I2C test until now ran against an empty bus: 112
//! NACKs, zero devices. Successful addressing, multi-byte writes, and reads had
//! never executed against a real slave, so `i2c.rs` carried a ⚠ rather than a ✅.
//! The board's ADS1015 supplies that slave.
//!
//! **Analog input.** The THEJAS32 package exposes no analog pins; the ARIES
//! V3.0's A0–A3 are this chip, on the I2C controller at `0x1000_1000`. Our
//! `adc.rs` drives that address as if it were an ADC register block, which the
//! bus scan disproved — a device answered at 0x48, and an ADC block cannot
//! implement the I2C state machine.
//!
//! # Oracles, and why these
//!
//! No external analog source is assumed, so nothing asserts a particular
//! voltage. What *can* be checked exactly:
//!
//! 1. **ACK is distinguished from NACK.** 0x48 must acknowledge and an absent
//!    address must not. A driver that reports success for everything once
//!    produced 112 phantom devices here and was briefly declared "validated end
//!    to end". One-sided evidence is not enough: both answers must be right.
//! 2. **The device echoes what was written.** Write the config register, read it
//!    back, compare. Exact, and it proves a 3-byte write and a 2-byte read moved
//!    real data in both directions.
//! 3. **A different write produces a different readback.** Writing the channel-1
//!    config must read back as channel 1 and differ from channel 0 in the MUX
//!    field. This rules out a readback that echoes our own buffer or returns a
//!    constant — the differential form of check 2.
//! 4. **Conversions are 12-bit.** Every channel must fall within 0x0FFF. A
//!    driver that forgot the 4-bit shift would exceed this.
//!
//! Voltages are printed but not scored: with nothing wired to A0–A3 the inputs
//! float, and asserting a value would be asserting noise.

#![no_std]
#![no_main]

use panic_halt as _;

use embedded_hal::delay::DelayNs;
use thejas32_examples::ads1015::{Ads1015, DEFAULT_ADDRESS, MAX_COUNT, config_for, counts_to_mv};
use thejas32_examples::console::{write_dec, write_hex, write_str};
use thejas32_hal::{PERIPHERAL_CLOCK_HZ, i2c::I2c, serial::Serial, timer::Timer};

/// Standard mode — the most forgiving of weak pull-ups.
const BUS_HZ: u32 = 100_000;

/// An address the scan found nothing at, used as the NACK control.
const ABSENT_ADDRESS: u8 = 0x50;

/// A conversion at 3300 SPS takes ~303 us. 2 ms is ample.
const CONVERSION_DELAY_MS: u32 = 2;

/// The OS bit reads back as "not currently converting", so it is masked out of
/// config comparisons rather than being predicted.
const OS_MASK: u16 = 0x7FFF;

#[riscv_rt::entry]
fn main() -> ! {
    // The PAC's singleton, taken once. `steal()` below (where present) is a
    // deliberate read-only alias for register inspection, not a second owner.
    let peripherals = thejas32_pac::Peripherals::take().unwrap();
    let mut serial = Serial::new_preconfigured(peripherals.uart0);
    let mut delay = Timer::new(peripherals.timer1);
    let i2c = I2c::new(peripherals.i2c2, PERIPHERAL_CLOCK_HZ, BUS_HZ);
    let mut adc = Ads1015::new(i2c, DEFAULT_ADDRESS);

    write_str(
        &mut serial,
        b"\r\nADS1015 self-test (I2C2 @ 0x1000_1000)\r\n",
    );

    // ---- [1] ACK and NACK are told apart -------------------------------
    write_str(&mut serial, b"\r\n[1] addressing\r\n");
    // Reach past the driver for the probes: `Ads1015` owns the bus, and this
    // check is about the bus rather than the device.
    let mut bus = adc.free();
    let present = bus.probe(DEFAULT_ADDRESS).unwrap_or(false);
    let absent = bus.probe(ABSENT_ADDRESS).unwrap_or(true);
    adc = Ads1015::new(bus, DEFAULT_ADDRESS);

    write_str(&mut serial, b"    0x48 acknowledges   ");
    verdict(&mut serial, present);
    write_str(&mut serial, b"    0x50 does not       ");
    verdict(&mut serial, !absent);
    let p1 = present && !absent;

    // ---- [2] the device echoes a written register ----------------------
    write_str(&mut serial, b"\r\n[2] config write then read back\r\n");
    let want0 = config_for(0);
    let mut got0 = 0;
    let mut p2 = false;
    if adc.write_register(0x01, want0).is_ok() {
        delay.delay_ms(CONVERSION_DELAY_MS);
        if let Ok(value) = adc.read_config() {
            got0 = value;
            p2 = (value & OS_MASK) == (want0 & OS_MASK);
        }
    }
    report_pair(&mut serial, b"    ch0 config", want0, got0);
    write_str(&mut serial, b"    matches            ");
    verdict(&mut serial, p2);

    // ---- [3] a different write reads back differently ------------------
    write_str(&mut serial, b"\r\n[3] a second, different config\r\n");
    let want1 = config_for(1);
    let mut got1 = 0;
    let mut p3 = false;
    if adc.write_register(0x01, want1).is_ok() {
        delay.delay_ms(CONVERSION_DELAY_MS);
        if let Ok(value) = adc.read_config() {
            got1 = value;
            // Must match what was asked for, *and* differ from the previous
            // readback -- otherwise a constant would satisfy phase 2.
            p3 = (value & OS_MASK) == (want1 & OS_MASK) && (value & OS_MASK) != (got0 & OS_MASK);
        }
    }
    report_pair(&mut serial, b"    ch1 config", want1, got1);
    write_str(&mut serial, b"    matches and differs ");
    verdict(&mut serial, p3);

    // ---- [4] conversions are 12-bit ------------------------------------
    write_str(&mut serial, b"\r\n[4] conversions (A0-A3)\r\n");
    let mut p4 = true;
    for channel in 0..4u8 {
        write_str(&mut serial, b"    A");
        write_dec(&mut serial, channel as u32);
        write_str(&mut serial, b": ");
        match adc.start_conversion(channel) {
            Ok(()) => {
                delay.delay_ms(CONVERSION_DELAY_MS);
                match adc.read_result() {
                    Ok(counts) => {
                        write_dec(&mut serial, counts as u32);
                        write_str(&mut serial, b" counts (");
                        write_dec(&mut serial, counts_to_mv(counts));
                        write_str(&mut serial, b" mV)");
                        if counts > MAX_COUNT {
                            write_str(&mut serial, b"  OUT OF RANGE");
                            p4 = false;
                        }
                    }
                    Err(_) => {
                        write_str(&mut serial, b"read failed");
                        p4 = false;
                    }
                }
            }
            Err(_) => {
                write_str(&mut serial, b"start failed");
                p4 = false;
            }
        }
        write_str(&mut serial, b"\r\n");
    }
    write_str(&mut serial, b"    all within 12 bits  ");
    verdict(&mut serial, p4);

    // Floating inputs are noise, so this is reported and not scored.
    write_str(
        &mut serial,
        b"\r\n    (voltages are informational: nothing is wired to A0-A3, so\r\n\
          \x20    the inputs float and any value is plausible)\r\n",
    );

    let passed = [p1, p2, p3, p4].iter().filter(|&&b| b).count();
    write_str(&mut serial, b"\r\nresult: ");
    write_dec(&mut serial, passed as u32);
    write_str(&mut serial, b"/4 checks passed\r\n");
    if p1 && p2 && p3 {
        write_str(
            &mut serial,
            b"\r\nThe I2C ACK path is now validated: addressing, a 3-byte write\r\n\
              and a 2-byte read all moved real data to a real device.\r\n",
        );
    }

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}

fn verdict<W: embedded_hal_nb::serial::Write<u8>>(serial: &mut W, ok: bool) {
    write_str(serial, if ok { b"PASS\r\n" } else { b"FAIL\r\n" });
}

fn report_pair<W: embedded_hal_nb::serial::Write<u8>>(
    serial: &mut W,
    label: &[u8],
    want: u16,
    got: u16,
) {
    write_str(serial, label);
    write_str(serial, b": wrote 0x");
    write_hex(serial, want as u32, 4);
    write_str(serial, b"  read 0x");
    write_hex(serial, got as u32, 4);
    write_str(serial, b"\r\n");
}
