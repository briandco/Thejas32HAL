//! ADC diagnostic — why does no conversion complete?
//!
//! `adc-selftest` reported a timeout on every channel: the status register's
//! bit 0 never sets, even given ~30 ms. It also showed bit 0 of the address
//! register failing to stick (`0x15` reads back `0x14`), though the vendor
//! driver never reads that register, so its readback semantics are unknown
//! and "written == read" was an assumption, not a requirement.
//!
//! This asserts nothing. It writes a range of values to the address register
//! and dumps all three registers in full 32-bit form, sampling the status
//! register over time after each write. Questions it answers:
//!
//! * Does *any* status bit move after a channel write, or is the register
//!   entirely static? (Maybe completion is not bit 0.)
//! * Which address bits are writable? (Bit 0 apparently is not.)
//! * Does the data register change even without a completion flag?
//!
//! Full-width printing throughout — the self-test truncated to two hex digits
//! and hid whatever is in the upper bits.

#![no_std]
#![no_main]

use panic_halt as _;

use thejas32_examples::console::{write_hex, write_str};
use thejas32_hal::{fence, serial::Serial};

/// Values to try in the address register: the vendor's `0x14`–`0x17`, bare
/// channel numbers, and the individual bits the vendor sets.
const CANDIDATES: [u32; 10] = [0x14, 0x15, 0x16, 0x17, 0x00, 0x01, 0x02, 0x03, 0x04, 0x10];

const STATUS_SAMPLES: usize = 8;
/// ~100 us between samples at 100 MHz — far longer than any plausible
/// conversion, so a flag that ever sets should be caught.
const SPACING: u32 = 10_000;

fn adc() -> thejas32_pac::Adc {
    // SAFETY: single-threaded diagnostic; this is the only handle in use.
    unsafe { thejas32_pac::Adc::steal() }
}

#[riscv_rt::entry]
fn main() -> ! {
    let mut serial = Serial::new_preconfigured(unsafe { thejas32_pac::Uart0::steal() });
    let a = adc();

    write_str(&mut serial, b"\r\nadc diag -- full register dump\r\n");
    write_str(
        &mut serial,
        b"base 0x10001000: +0 address, +4 status, +8 data\r\n",
    );

    // --- idle state, before touching anything ---
    write_str(&mut serial, b"\r\n[idle]\r\n");
    dump(&mut serial, &a);

    // --- try each candidate address value ---
    for &value in CANDIDATES.iter() {
        write_str(&mut serial, b"\r\n[write address = ");
        write_hex(&mut serial, value, 8);
        write_str(&mut serial, b"]\r\n");

        // SAFETY: raw write to the address register, which is what selects a
        // channel and starts a conversion.
        unsafe { a.address().write(|w| w.bits(value)) };
        fence();

        write_str(&mut serial, b"    readback: ");
        write_hex(&mut serial, a.address().read().bits(), 8);
        let stuck = a.address().read().bits() ^ value;
        write_str(&mut serial, b"  differs-by: ");
        write_hex(&mut serial, stuck, 8);
        write_str(&mut serial, b"\r\n");

        write_str(&mut serial, b"    status over time:");
        let mut samples = [0u32; STATUS_SAMPLES];
        let mut data = [0u32; STATUS_SAMPLES];
        for i in 0..STATUS_SAMPLES {
            samples[i] = a.status().read().bits();
            data[i] = a.data().read().bits();
            for _ in 0..SPACING {
                unsafe { core::arch::asm!("nop") };
            }
        }
        for s in samples.iter() {
            write_str(&mut serial, b" ");
            write_hex(&mut serial, *s, 8);
        }
        write_str(&mut serial, b"\r\n    data over time:  ");
        for d in data.iter() {
            write_str(&mut serial, b" ");
            write_hex(&mut serial, *d, 8);
        }
        write_str(&mut serial, b"\r\n");

        // Did anything move at all?
        let status_moved = samples.iter().any(|&s| s != samples[0]);
        let data_moved = data.iter().any(|&d| d != data[0]);
        write_str(&mut serial, b"    status changed: ");
        write_str(&mut serial, if status_moved { b"YES" } else { b"no" });
        write_str(&mut serial, b"   data changed: ");
        write_str(&mut serial, if data_moved { b"YES" } else { b"no" });
        write_str(&mut serial, b"\r\n");
    }

    write_str(
        &mut serial,
        b"\r\nIf status is identical for every write, the ADC is not\r\n\
          \x20converting at all -- suspect a missing enable or clock rather\r\n\
          \x20than a wrong flag bit.\r\n",
    );

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}

fn dump<W: embedded_hal_nb::serial::Write<u8>>(serial: &mut W, a: &thejas32_pac::Adc) {
    write_str(serial, b"    address: ");
    write_hex(serial, a.address().read().bits(), 8);
    write_str(serial, b"\r\n    status:  ");
    write_hex(serial, a.status().read().bits(), 8);
    write_str(serial, b"\r\n    data:    ");
    write_hex(serial, a.data().read().bits(), 8);
    write_str(serial, b"\r\n");
}
