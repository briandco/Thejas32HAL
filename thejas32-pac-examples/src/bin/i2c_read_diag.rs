//! I2C read diagnostic — where exactly does a read fail?
//!
//! `ads1015-selftest` established two things against a real device on the bus
//! at `0x1000_1000`:
//!
//! * **Writes work.** 0x48 acknowledges and 0x50 does not, and
//!   `start_conversion` — a three-byte write — returns `Ok`.
//! * **Reads fail.** `read_result` returns an error for every channel.
//!
//! That is the path `i2c.rs` has always carried a ⚠ for: never exercised,
//! because until now there was no device to read from.
//!
//! The self-test could not say *which* error, because the ADS1015 driver maps
//! every HAL error onto one variant, and its phases 2 and 3 print `read 0x0000`
//! identically whether the write failed, the read failed, or the read genuinely
//! returned zero. Three failures, one output — a conflation that should not
//! have been there.
//!
//! So this replicates `read_from`'s register sequence one step at a time
//! against the raw PAC, printing SR0 and SR1 after each, and names the first
//! step whose status flag never arrives. No error mapping, no inference.
//!
//! SR0 bits: 0x01 START done, 0x02 STOP done, 0x04 TX FIFO full,
//! 0x08 TX FIFO empty, 0x10 transfer complete, 0x40 RX FIFO empty,
//! 0x80 RX complete. SR1 bit 0x01 is NACK.

#![no_std]
#![no_main]

use panic_halt as _;

use thejas32_examples::console::{write_dec, write_hex, write_str};
use thejas32_hal::{PERIPHERAL_CLOCK_HZ, i2c::I2c, serial::Serial};

const BUS_HZ: u32 = 100_000;
const ADDRESS: u8 = 0x48;

/// The ADS1015 conversion register — the read that fails in the self-test.
const REG_CONVERT: u8 = 0x00;

/// Bytes to read. The ADS1015 returns 16-bit registers.
const READ_LEN: usize = 2;

const SR0_START_DONE: u8 = 0x01;
const SR0_STOP_DONE: u8 = 0x02;
const SR0_TX_FIFO_FULL: u8 = 0x04;
const SR0_TX_FIFO_EMPTY: u8 = 0x08;
const SR0_TRANSFER_COMPLETE: u8 = 0x10;
const SR0_RX_FIFO_EMPTY: u8 = 0x40;
const SR0_RX_COMPLETE: u8 = 0x80;
/// NACK is SR1 bit 0, per `i2c.c:136` (`I2C_SR1 & 0x01`). An earlier
/// revision of this file used 0x02, which was wrong; SR1 read 0x00 throughout
/// that run, so its ACK conclusion was unaffected.
const SR1_NACK: u8 = 0x01;

const CR_START: u8 = 0x01;
const CR_STOP: u8 = 0x02;

/// Matches the HAL's default, so timing behaviour is comparable.
const TIMEOUT: u32 = 1_000_000;

/// Reads SR0 / SR1 straight from the peripheral.
fn status() -> (u8, u8) {
    // SAFETY: read-only access to fixed MMIO for diagnostics.
    let p = unsafe { thejas32_pac::I2c2::steal() };
    (p.sr0().read().bits(), p.sr1().read().bits())
}

/// Spins until `cond` holds on SR0, reporting how many iterations it took.
/// Returns `None` on timeout.
fn wait<F: Fn(u8) -> bool>(cond: F) -> Option<u32> {
    (0..TIMEOUT).find(|_| cond(status().0))
}

/// Prints the outcome of one step and returns whether it succeeded.
fn step<W: embedded_hal_nb::serial::Write<u8>>(
    serial: &mut W,
    label: &[u8],
    waited: Option<u32>,
) -> bool {
    let (sr0, sr1) = status();
    write_str(serial, b"    ");
    write_str(serial, label);
    match waited {
        Some(iterations) => {
            write_str(serial, b" ok after ");
            write_dec(serial, iterations);
        }
        None => write_str(serial, b" TIMED OUT"),
    }
    write_str(serial, b"   sr0=0x");
    write_hex(serial, sr0 as u32, 2);
    write_str(serial, b" sr1=0x");
    write_hex(serial, sr1 as u32, 2);
    write_str(serial, b"\r\n");
    waited.is_some()
}

#[riscv_rt::entry]
fn main() -> ! {
    let mut serial = Serial::new_preconfigured(unsafe { thejas32_pac::Uart0::steal() });
    write_str(
        &mut serial,
        b"\r\ni2c read diagnostic -- ADS1015 @ 0x48 on I2C2\r\n",
    );

    // Configure the controller through the HAL, then drive registers directly.
    // Clock setup is already validated; only the read sequence is in question.
    let mut i2c = I2c::new(
        unsafe { thejas32_pac::I2c2::steal() },
        PERIPHERAL_CLOCK_HZ,
        BUS_HZ,
    );

    // ---- control: the write path, known to work ---------------------------
    write_str(&mut serial, b"\r\n[a] pointer write (known-good path)\r\n");
    let wrote = i2c.write_to(ADDRESS, &[REG_CONVERT]).is_ok();
    write_str(&mut serial, b"    write_to -> ");
    write_str(&mut serial, if wrote { b"Ok" } else { b"Err" });
    let (sr0, sr1) = status();
    write_str(&mut serial, b"   sr0=0x");
    write_hex(&mut serial, sr0 as u32, 2);
    write_str(&mut serial, b" sr1=0x");
    write_hex(&mut serial, sr1 as u32, 2);
    write_str(&mut serial, b"\r\n");

    // ---- the read sequence, one register at a time ------------------------
    write_str(&mut serial, b"\r\n[b] read sequence, step by step\r\n");
    let p = unsafe { thejas32_pac::I2c2::steal() };

    // 1. Clear the TX FIFO and wait for the controller to go idle.
    unsafe { p.txclr().write(|w| w.bits(0xFF)) };
    thejas32_hal::fence();
    let ok1 = step(
        &mut serial,
        b"1 idle (txe+xfer)   ",
        wait(|s| s & SR0_TX_FIFO_EMPTY != 0 && s & SR0_TRANSFER_COMPLETE != 0),
    );

    // 2. START, with the read length encoded into CR bits 2+.
    let cr = ((READ_LEN as u8) << 2) | CR_START;
    unsafe { p.cr().write(|w| w.bits(cr)) };
    thejas32_hal::fence();
    write_str(&mut serial, b"    (wrote cr=0x");
    write_hex(&mut serial, cr as u32, 2);
    write_str(&mut serial, b")\r\n");
    let ok2 = step(
        &mut serial,
        b"2 start done        ",
        wait(|s| s & SR0_START_DONE != 0),
    );

    // 3. Address byte with the read bit set.
    let ok3 = step(
        &mut serial,
        b"3 tx fifo not full  ",
        wait(|s| s & SR0_TX_FIFO_FULL == 0),
    );
    unsafe { p.txff().write(|w| w.bits((ADDRESS << 1) | 1)) };
    thejas32_hal::fence();

    // 4. Transfer complete for the address byte.
    let ok4 = step(
        &mut serial,
        b"4 xfer complete     ",
        wait(|s| s & SR0_TRANSFER_COMPLETE != 0),
    );

    // 5. Did the device acknowledge its read address?
    let (_, sr1_after) = status();
    let acked = sr1_after & SR1_NACK == 0;
    write_str(&mut serial, b"    5 address ACKed     ");
    write_str(
        &mut serial,
        if acked { b"yes" } else { b"NO (sr1 NACK set)" },
    );
    write_str(&mut serial, b"\r\n");

    // 6. RX complete — the step the HAL's `read_bytes` waits on first.
    let ok6 = step(
        &mut serial,
        b"6 rx complete       ",
        wait(|s| s & SR0_RX_COMPLETE != 0),
    );

    // 7. Drain the FIFO.
    let mut data = [0u8; READ_LEN];
    let mut ok7 = true;
    for (i, slot) in data.iter_mut().enumerate() {
        let waited = wait(|s| s & SR0_RX_FIFO_EMPTY == 0);
        write_str(&mut serial, b"    7.");
        write_dec(&mut serial, i as u32);
        ok7 &= step(&mut serial, b" rx fifo has data ", waited);
        if waited.is_some() {
            *slot = p.rxff().read().bits();
        }
    }

    unsafe { p.cr().write(|w| w.bits(CR_STOP)) };
    thejas32_hal::fence();
    let _ = wait(|s| s & SR0_STOP_DONE != 0);

    write_str(&mut serial, b"\r\n    bytes read: 0x");
    write_hex(&mut serial, data[0] as u32, 2);
    write_str(&mut serial, b" 0x");
    write_hex(&mut serial, data[1] as u32, 2);
    write_str(&mut serial, b"\r\n");

    // ---- name the first failing step --------------------------------------
    write_str(&mut serial, b"\r\nverdict:\r\n");
    if !ok1 {
        write_str(&mut serial, b"  Controller never goes idle -- the bus is stuck\r\n  before the read even starts.\r\n");
    } else if !ok2 {
        write_str(&mut serial, b"  START never completes with a read length in CR.\r\n  Suspect the CR encoding for reads.\r\n");
    } else if !ok3 || !ok4 {
        write_str(&mut serial, b"  The read-address byte never transfers. The write path\r\n  works, so this is specific to the R/W bit or CR state.\r\n");
    } else if !acked {
        write_str(&mut serial, b"  The device NACKed its read address although it ACKs\r\n  writes. Suspect the address shift for reads.\r\n");
    } else if !ok6 {
        write_str(&mut serial, b"  Address ACKed but RX complete never asserts. The\r\n  controller is not clocking in data -- suspect the read\r\n  length in CR, which is what tells it how many bytes.\r\n");
    } else if !ok7 {
        write_str(&mut serial, b"  RX complete asserted but the FIFO reports empty.\r\n  Suspect the RX FIFO empty bit polarity.\r\n");
    } else {
        write_str(&mut serial, b"  Every step succeeded here. If the HAL still fails,\r\n  the difference is in what precedes it -- most likely the\r\n  preceding write leaving state the HAL does not clear.\r\n");
    }

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}
