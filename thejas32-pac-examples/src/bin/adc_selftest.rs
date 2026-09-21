//! ADC self-test — validates the ADC driver over UART.
//!
//! An ADC measures the outside world, so unlike the digital peripherals its
//! *values* cannot be checked without controlling an input. What the test does
//! without any wiring:
//!
//! 1. **Address encoding** — write a channel selection, read the register
//!    back, and confirm it holds `0x14 | channel`. This specifically guards
//!    the PAC gap: the SVD models only bits 0:1, so a driver using the typed
//!    `channel()` field would read back `0x00`–`0x03` here and fail.
//! 2. **Conversion completes** — every channel must set its done flag within
//!    the bound, rather than hanging.
//! 3. **Sample statistics** — 32 samples per channel, reporting min/max/mean.
//!    Stability is itself evidence: a pin tied to a rail reads steadily, a
//!    floating pin wanders.
//!
//! To validate the *data path*, jumper one channel and re-run:
//!
//! * `ADC_CH0` → `GND`      expect mean ≈ 0, spread ≈ 0
//! * `ADC_CH0` → `VCC_3V3`  expect mean ≈ 4095, spread ≈ 0
//!
//! Both pins sit on the ARIES V3.0 analog header. Two runs at known voltages
//! give a genuine two-point check of scale and offset; without them, only
//! addressing and completion are proven.

#![no_std]
#![no_main]

use panic_halt as _;

use thejas32_examples::console::{write_dec, write_hex, write_str};
use thejas32_hal::{
    adc::{Adc, Channel, MAX_VALUE},
    serial::Serial,
};

const SAMPLES: u16 = 32;
/// Spread below this counts as a steady reading rather than a floating pin.
const STEADY_SPREAD: u16 = 64;
/// Within this of a rail counts as sitting at that rail.
const RAIL_MARGIN: u16 = 128;

#[riscv_rt::entry]
fn main() -> ! {
    // The PAC's singleton, taken once. `steal()` below (where present) is a
    // deliberate read-only alias for register inspection, not a second owner.
    let peripherals = thejas32_pac::Peripherals::take().unwrap();
    let mut serial = Serial::new_preconfigured(peripherals.uart0);
    let mut adc = Adc::new(peripherals.adc);

    write_str(&mut serial, b"\r\nadc self-test (12-bit, 4 channels)\r\n");

    // ---- [1] address encoding --------------------------------------------
    // Reading a channel leaves its encoding in the address register.
    let mut p1 = true;
    write_str(&mut serial, b"\r\n[1] address encoding\r\n");
    for ch in Channel::ALL {
        let _ = adc.read(ch);
        let got = adc.address_register();
        let want = ch.address();
        let ok = got == want;
        p1 &= ok;
        write_str(&mut serial, b"    A");
        write_dec(&mut serial, ch as u32);
        write_str(&mut serial, b": reg=");
        write_hex(&mut serial, got, 2);
        write_str(&mut serial, b" (want ");
        write_hex(&mut serial, want, 2);
        write_str(
            &mut serial,
            if ok { b")  ok\r\n" } else { b")  MISMATCH\r\n" },
        );
    }
    line(&mut serial, b"    encoding correct     ", p1);

    // ---- [2] conversions complete ----------------------------------------
    let mut p2 = true;
    write_str(&mut serial, b"\r\n[2] conversion completes\r\n");
    for ch in Channel::ALL {
        let ok = adc.read(ch).is_ok();
        p2 &= ok;
        write_str(&mut serial, b"    A");
        write_dec(&mut serial, ch as u32);
        write_str(&mut serial, if ok { b": ok\r\n" } else { b": TIMEOUT\r\n" });
    }
    line(&mut serial, b"    all channels convert ", p2);

    // ---- [3] sample statistics -------------------------------------------
    write_str(&mut serial, b"\r\n[3] samples (32 per channel)\r\n");
    for ch in Channel::ALL {
        match adc.sample(ch, SAMPLES) {
            Ok((min, max, mean)) => {
                let spread = max - min;
                write_str(&mut serial, b"    A");
                write_dec(&mut serial, ch as u32);
                write_str(&mut serial, b": min=");
                write_dec4(&mut serial, min);
                write_str(&mut serial, b" max=");
                write_dec4(&mut serial, max);
                write_str(&mut serial, b" mean=");
                write_dec4(&mut serial, mean);
                write_str(&mut serial, b" spread=");
                write_dec4(&mut serial, spread);
                write_str(&mut serial, b"  ");
                write_str(&mut serial, classify(mean, spread));
                write_str(&mut serial, b"\r\n");
            }
            Err(_) => {
                write_str(&mut serial, b"    A");
                write_dec(&mut serial, ch as u32);
                write_str(&mut serial, b": TIMEOUT\r\n");
            }
        }
    }

    write_str(
        &mut serial,
        b"\r\nAddressing and completion are proven above. Values are not:\r\n\
          \x20 jumper ADC_CH0 to GND (expect ~0) then to VCC_3V3 (expect\r\n\
          \x20 ~4095) and re-run for a two-point check of the data path.\r\n",
    );

    let passed = [p1, p2].iter().filter(|&&b| b).count();
    write_str(&mut serial, b"\r\nresult: ");
    write_dec(&mut serial, passed as u32);
    write_str(
        &mut serial,
        b"/2 checks passed (data path unproven without a jumper)\r\n",
    );

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}

/// Describes a channel from its mean and spread.
fn classify(mean: u16, spread: u16) -> &'static [u8] {
    if spread > STEADY_SPREAD {
        b"wandering -- floating input?"
    } else if mean <= RAIL_MARGIN {
        b"steady at ground"
    } else if mean >= MAX_VALUE - RAIL_MARGIN {
        b"steady at full scale"
    } else {
        b"steady, mid-scale"
    }
}

/// Right-aligns a value to four columns so the table lines up.
fn write_dec4<W: embedded_hal_nb::serial::Write<u8>>(serial: &mut W, value: u16) {
    let pad: &[u8] = match value {
        0..=9 => b"   ",
        10..=99 => b"  ",
        100..=999 => b" ",
        _ => b"",
    };
    write_str(serial, pad);
    write_dec(serial, value as u32);
}

fn line<W: embedded_hal_nb::serial::Write<u8>>(serial: &mut W, label: &[u8], ok: bool) {
    write_str(serial, label);
    write_str(serial, if ok { b"PASS\r\n" } else { b"FAIL\r\n" });
}
