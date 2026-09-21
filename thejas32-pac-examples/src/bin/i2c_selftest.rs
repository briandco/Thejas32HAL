//! I2C self-test — validates the I2C driver and scans the bus.
//!
//! I2C cannot be loopback-tested the way SPI was: there is no way to talk to
//! yourself, so the data path needs a real slave device. What *can* be
//! verified without one:
//!
//! 1. **Clock configuration** — write the period registers, read them back,
//!    and check they encode the requested bus frequency.
//! 2. **Bus idle state** — with the controller configured but idle, the
//!    status register should show the transmit FIFO empty and no transfer in
//!    progress. A stuck-low bus shows up here.
//! 3. **Bus scan** — probe every valid 7-bit address. Every wait in the
//!    driver is bounded, so this completes even with a dead bus.
//!
//! Interpreting the scan:
//!
//! * **devices found** — the data path works end to end; those addresses
//!   acknowledged.
//! * **all NACK** — the controller generates START/address/STOP correctly and
//!   sees the (absent) slaves decline. Signalling works; there is simply
//!   nothing attached. This is the expected result on a bare board *with*
//!   pull-up resistors.
//! * **all timeout** — nothing is driving the bus back. Most likely no
//!   pull-ups on SDA/SCL, so the lines never rise. Not necessarily a driver
//!   fault, but nothing about the transaction machinery is proven either.

#![no_std]
#![no_main]

use panic_halt as _;

use thejas32_examples::console::{write_dec, write_hex, write_str};
use thejas32_hal::{
    PERIPHERAL_CLOCK_HZ,
    i2c::{Error, I2c},
    serial::Serial,
};

const I2C_HZ: u32 = 100_000;
/// Valid 7-bit address range; 0x00-0x07 and 0x78-0x7F are reserved.
const FIRST_ADDR: u8 = 0x08;
const LAST_ADDR: u8 = 0x77;
/// Short bound per probe so a dead bus still finishes the sweep quickly.
const PROBE_TIMEOUT: u32 = 20_000;
/// More devices than this on one bus is not credible; treat it as a broken
/// probe rather than a discovery. An earlier version of this test reported
/// all 112 addresses as devices and called it success — a false pass is far
/// more dangerous than a false failure, so the oracle now says so.
const PLAUSIBLE_MAX: u32 = 8;

#[riscv_rt::entry]
fn main() -> ! {
    // The PAC's singleton, taken once. `steal()` below (where present) is a
    // deliberate read-only alias for register inspection, not a second owner.
    let peripherals = thejas32_pac::Peripherals::take().unwrap();
    let mut serial = Serial::new_preconfigured(peripherals.uart0);
    write_str(&mut serial, b"\r\ni2c self-test (I2C0 @ 100 kHz)\r\n");

    let mut i2c =
        I2c::new(peripherals.i2c0, PERIPHERAL_CLOCK_HZ, I2C_HZ).with_timeout(PROBE_TIMEOUT);

    // ---- [1] clock configuration readback --------------------------------
    let raw = unsafe { thejas32_pac::I2c0::steal() };
    let chl = raw.chl().read().bits() as u32;
    let chh = raw.chh().read().bits() as u32;
    let chhl = raw.chhl().read().bits() as u32;
    let chhh = raw.chhh().read().bits() as u32;
    let low = (chh << 8) | chl;
    let high = (chhh << 8) | chhl;

    let want_low = PERIPHERAL_CLOCK_HZ / (2 * I2C_HZ);
    let want_high = PERIPHERAL_CLOCK_HZ / (4 * I2C_HZ);
    let p1 = low == want_low && high == want_high;

    write_str(&mut serial, b"\r\n[1] clock config\r\n    low  period: ");
    write_dec(&mut serial, low);
    write_str(&mut serial, b" (want ");
    write_dec(&mut serial, want_low);
    write_str(&mut serial, b")\r\n    high period: ");
    write_dec(&mut serial, high);
    write_str(&mut serial, b" (want ");
    write_dec(&mut serial, want_high);
    write_str(&mut serial, b")\r\n");
    line(&mut serial, b"    readback matches      ", p1);

    // ---- [2] idle bus state ----------------------------------------------
    let sr0 = raw.sr0().read().bits();
    let sr1 = raw.sr1().read().bits();
    // Idle: transmit FIFO empty (0x08) and transfer complete (0x10).
    let p2 = sr0 & 0x08 != 0 && sr0 & 0x10 != 0;
    write_str(&mut serial, b"\r\n[2] idle status\r\n    sr0=");
    write_hex(&mut serial, sr0 as u32, 2);
    write_str(&mut serial, b" sr1=");
    write_hex(&mut serial, sr1 as u32, 2);
    write_str(&mut serial, b"\r\n");
    line(&mut serial, b"    tx empty + idle       ", p2);

    // ---- [3] bus scan -----------------------------------------------------
    write_str(&mut serial, b"\r\n[3] bus scan 0x08-0x77\r\n");
    let mut found = 0u32;
    let mut nacks = 0u32;
    let mut timeouts = 0u32;

    for addr in FIRST_ADDR..=LAST_ADDR {
        match i2c.probe(addr) {
            Ok(true) => {
                found += 1;
                write_str(&mut serial, b"    device at 0x");
                write_hex(&mut serial, addr as u32, 2);
                write_str(&mut serial, b"\r\n");
            }
            Ok(false) => nacks += 1,
            Err(Error::Timeout) => timeouts += 1,
            Err(_) => {}
        }
    }

    write_str(&mut serial, b"    found=");
    write_dec(&mut serial, found);
    write_str(&mut serial, b" nack=");
    write_dec(&mut serial, nacks);
    write_str(&mut serial, b" timeout=");
    write_dec(&mut serial, timeouts);
    write_str(&mut serial, b"\r\n");

    write_str(&mut serial, b"\r\nverdict: ");
    if found > PLAUSIBLE_MAX {
        write_str(
            &mut serial,
            b"IMPLAUSIBLE -- too many 'devices' to be real.\r\n\
              \x20        NACK detection is not working; treat every result\r\n\
              \x20        here as meaningless.\r\n",
        );
    } else if found > 0 {
        write_str(
            &mut serial,
            b"DEVICE(S) FOUND -- data path validated end to end\r\n",
        );
    } else if nacks > 0 && timeouts == 0 {
        write_str(
            &mut serial,
            b"all NACK -- START/address/STOP work, bus is simply empty\r\n",
        );
    } else if timeouts > 0 && nacks == 0 {
        write_str(
            &mut serial,
            b"all TIMEOUT -- nothing drives the bus back; check SDA/SCL\r\n\
              \x20        pull-ups. Transaction machinery unproven.\r\n",
        );
    } else {
        write_str(&mut serial, b"mixed NACK/timeout -- see counts above\r\n");
    }

    let passed = [p1, p2].iter().filter(|&&b| b).count();
    write_str(&mut serial, b"config checks: ");
    write_dec(&mut serial, passed as u32);
    write_str(&mut serial, b"/2 passed\r\n");

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}

fn line<W: embedded_hal_nb::serial::Write<u8>>(serial: &mut W, label: &[u8], ok: bool) {
    write_str(serial, label);
    write_str(serial, if ok { b"PASS\r\n" } else { b"FAIL\r\n" });
}
