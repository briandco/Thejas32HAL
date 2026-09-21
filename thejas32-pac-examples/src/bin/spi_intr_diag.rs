//! SPI interrupt diagnostic — where does the interrupt-enabled transfer stall?
//!
//! `spi-intr-selftest` printed its banner, passed the loopback check, and then
//! hung in phase 1. It hung in the same place after a fix, so the first guess
//! was wrong and this stops guessing.
//!
//! Everything here is bounded and every step prints a marker, so the last line
//! on the console names the stall. Nothing in this file can hang.
//!
//! # What is actually unknown
//!
//! The SVD gives the status register **two** receive flags:
//!
//! | bit | field | used by |
//! |-----|-------|---------|
//! | 2 | `RX_INT_STATUS` | `vega-sdk`'s `spi_intr_status` |
//! | 6 | `RX_COMPLETE` | this HAL's `transfer_byte` |
//!
//! So the interrupt condition and the polled condition are *different bits*,
//! and it is unknown whether reading `RxData` clears bit 2 as well as bit 6. If
//! it clears only bit 6, the interrupt re-asserts forever.
//!
//! Two candidate stalls, and this separates them:
//!
//! 1. **An interrupt storm.** If bit 2 cannot be cleared, the handler re-enters
//!    without limit. The counters here show it.
//! 2. **A control-register race.** The handler masks `rx_tx_intr` with a
//!    read-modify-write on the control register — the same register `SpiBus`
//!    uses for `CSAAT`. An update lost either way could leave a transfer
//!    unable to finish.
//!
//! The transfer here is driven at register level with bounded waits, so a stall
//! reports a count rather than spinning.

#![no_std]
#![no_main]

use panic_halt as _;

use core::sync::atomic::{AtomicU32, Ordering};
use riscv::interrupt::Interrupt as CoreInterrupt;
use thejas32_examples::console::{write_dec, write_hex, write_str};
use thejas32_hal::{
    plic,
    serial::Serial,
    spi::{Config, Spi},
};
use thejas32_pac::interrupt::Interrupt;

const PATTERN: u8 = 0xA5;

/// Bound on every register wait. Generous, and small enough to report quickly.
const LIMIT: u32 = 200_000;

/// Handler entries after which the PLIC line is dropped.
const STORM_LIMIT: u32 = 500;

const ST_RX_INT: u32 = 1 << 2;
const ST_TX_INT: u32 = 1 << 3;
const ST_BUSY: u32 = 1 << 4;
const ST_RX_COMPLETE: u32 = 1 << 6;

static HANDLER_RUNS: AtomicU32 = AtomicU32::new(0);
static STORMED: AtomicU32 = AtomicU32::new(0);
/// Status as seen on the first handler entry.
static FIRST_STATUS: AtomicU32 = AtomicU32::new(0xFFFF_FFFF);

#[riscv_rt::core_interrupt(CoreInterrupt::MachineExternal)]
fn machine_external() {
    // SAFETY: read-only PLIC access from the handler.
    let plic_regs = unsafe { thejas32_pac::Plic::steal() };
    // Count *every* external interrupt, not only ones the PLIC attributes to
    // SPI0. An earlier handler returned early before counting, which would have
    // hidden a storm from another source entirely.
    let count = HANDLER_RUNS.load(Ordering::Relaxed) + 1;
    HANDLER_RUNS.store(count, Ordering::Relaxed);

    // SAFETY: diagnostic access to fixed MMIO.
    let spi = unsafe { thejas32_pac::Spi0::steal() };
    if FIRST_STATUS.load(Ordering::Relaxed) == 0xFFFF_FFFF {
        FIRST_STATUS.store(spi.status().read().bits(), Ordering::Relaxed);
    }

    // Drop the whole PLIC enable mask: the cheapest possible escape, touching
    // no peripheral register, so it cannot itself race with the transfer.
    if count >= STORM_LIMIT {
        STORMED.store(1, Ordering::Relaxed);
        // SAFETY: single-hart; nothing else writes this register concurrently.
        unsafe { plic_regs.intr_en().write(|w| w.bits(0)) };
        thejas32_hal::fence();
    }
}

/// Reads the SPI status register.
fn status() -> u32 {
    // SAFETY: read-only access to fixed MMIO.
    unsafe { thejas32_pac::Spi0::steal() }
        .status()
        .read()
        .bits()
}

fn show<W: embedded_hal_nb::serial::Write<u8>>(serial: &mut W, label: &[u8]) {
    let s = status();
    write_str(serial, label);
    write_str(serial, b" status=0x");
    write_hex(serial, s, 4);
    write_str(serial, b" [rxint=");
    write_dec(serial, (s & ST_RX_INT != 0) as u32);
    write_str(serial, b" txint=");
    write_dec(serial, (s & ST_TX_INT != 0) as u32);
    write_str(serial, b" busy=");
    write_dec(serial, (s & ST_BUSY != 0) as u32);
    write_str(serial, b" rxdone=");
    write_dec(serial, (s & ST_RX_COMPLETE != 0) as u32);
    write_str(serial, b"]  handlers=");
    write_dec(serial, HANDLER_RUNS.load(Ordering::Relaxed));
    write_str(serial, b"\r\n");
}

/// Bounded wait for a status condition; returns iterations, or `None`.
fn wait<F: Fn(u32) -> bool>(cond: F) -> Option<u32> {
    (0..LIMIT).find(|_| cond(status()))
}

fn report<W: embedded_hal_nb::serial::Write<u8>>(serial: &mut W, label: &[u8], r: Option<u32>) {
    write_str(serial, label);
    match r {
        Some(i) => {
            write_str(serial, b" ok after ");
            write_dec(serial, i);
        }
        None => write_str(serial, b" TIMED OUT"),
    }
    write_str(serial, b"\r\n");
}

#[riscv_rt::entry]
fn main() -> ! {
    let mut serial = Serial::new_preconfigured(unsafe { thejas32_pac::Uart0::steal() });
    let plic = plic::Plic::new(unsafe { thejas32_pac::Plic::steal() });
    write_str(&mut serial, b"\r\nSPI interrupt diagnostic (bounded)\r\n");

    let spi = Spi::new(unsafe { thejas32_pac::Spi0::steal() }, Config::default());
    write_str(&mut serial, b"  [a] configured\r\n");
    show(&mut serial, b"      ");

    // ---- a polled transfer, before any interrupt is enabled --------------
    // The control: if this path already stalls, interrupts are irrelevant.
    write_str(
        &mut serial,
        b"\r\n  [b] polled transfer, interrupts OFF\r\n",
    );
    let raw = unsafe { thejas32_pac::Spi0::steal() };
    report(
        &mut serial,
        b"      tx ready ",
        wait(|s| s & (1 << 1) != 0 || s & ST_BUSY == 0),
    );
    // SAFETY: writing a data word to the transmit register.
    unsafe { raw.txdata().write(|w| w.data().bits(PATTERN as u16)) };
    thejas32_hal::fence();
    let rx = wait(|s| s & ST_RX_COMPLETE != 0);
    report(&mut serial, b"      rx complete ", rx);
    let byte = raw.rxdata().read().data().bits() as u8;
    write_str(&mut serial, b"      byte back: 0x");
    write_hex(&mut serial, byte as u32, 2);
    write_str(&mut serial, b"\r\n");
    show(&mut serial, b"      after read:");

    // Does reading RxData clear the *interrupt* status bit, or only rxdone?
    write_str(
        &mut serial,
        b"      rx interrupt bit after reading RxData: ",
    );
    write_dec(&mut serial, (status() & ST_RX_INT != 0) as u32);
    write_str(
        &mut serial,
        b"  (1 here means it cannot be cleared that way)\r\n",
    );

    // ---- now enable the interrupt ----------------------------------------
    write_str(&mut serial, b"\r\n  [c] enabling RX interrupt\r\n");
    spi.listen(thejas32_hal::spi::Event::RxComplete);
    write_str(&mut serial, b"      listen ok\r\n");
    show(&mut serial, b"      ");

    plic.enable(Interrupt::SPI0);
    write_str(&mut serial, b"      plic enabled\r\n");

    // SAFETY: the handler is installed and drops the PLIC mask if it storms.
    unsafe {
        plic::enable_external_interrupts();
        plic::enable_global_interrupts();
    }
    write_str(&mut serial, b"      core interrupts on\r\n");
    show(&mut serial, b"      ");

    // ---- one transfer with the interrupt live ----------------------------
    write_str(&mut serial, b"\r\n  [d] one transfer, interrupt ON\r\n");
    let tx_ok = wait(|s| s & ST_BUSY == 0);
    report(&mut serial, b"      idle before tx ", tx_ok);

    // SAFETY: as above.
    unsafe { raw.txdata().write(|w| w.data().bits(PATTERN as u16)) };
    thejas32_hal::fence();
    write_str(&mut serial, b"      byte written\r\n");

    let rx2 = wait(|s| s & ST_RX_COMPLETE != 0);
    report(&mut serial, b"      rx complete ", rx2);
    show(&mut serial, b"      ");

    let byte2 = raw.rxdata().read().data().bits() as u8;
    write_str(&mut serial, b"      byte back: 0x");
    write_hex(&mut serial, byte2 as u32, 2);
    write_str(&mut serial, b"\r\n");
    show(&mut serial, b"      after read:");

    // SAFETY: quiesce before the verdict.
    unsafe { plic::disable_global_interrupts() };
    plic.disable_all();

    // ---- [e] is the status register read-to-clear? -----------------------
    //
    // Everything above is consistent with the handler's single *status read*
    // consuming the flag `transfer_byte` waits on. That is an inference, so
    // test it directly: with interrupts off, start a transfer, wait without
    // touching the status register at all, then read it three times in a row
    // and see whether the flags survive their own observation.
    //
    // The delay is a plain nop loop precisely because polling would read the
    // register and destroy what is being measured.
    write_str(&mut serial, b"\r\n  [e] is status read-to-clear?\r\n");
    // SAFETY: writing a data word to the transmit register.
    unsafe { raw.txdata().write(|w| w.data().bits(PATTERN as u16)) };
    thejas32_hal::fence();
    // [b] completed in 41 status-read iterations, so this is ample.
    for _ in 0..50_000 {
        unsafe { core::arch::asm!("nop") };
    }
    let read1 = raw.status().read().bits();
    let read2 = raw.status().read().bits();
    let read3 = raw.status().read().bits();

    let show_read = |serial: &mut Serial<thejas32_pac::Uart0>, label: &[u8], v: u32| {
        write_str(serial, label);
        write_hex(serial, v, 4);
        write_str(serial, b" [rxint=");
        write_dec(serial, (v & ST_RX_INT != 0) as u32);
        write_str(serial, b" rxdone=");
        write_dec(serial, (v & ST_RX_COMPLETE != 0) as u32);
        write_str(serial, b"]\r\n");
    };
    show_read(&mut serial, b"      read 1: 0x", read1);
    show_read(&mut serial, b"      read 2: 0x", read2);
    show_read(&mut serial, b"      read 3: 0x", read3);
    let leftover = raw.rxdata().read().data().bits() as u8;
    write_str(&mut serial, b"      byte still in RxData: 0x");
    write_hex(&mut serial, leftover as u32, 2);
    write_str(&mut serial, b"\r\n");

    let read_to_clear = (read1 & ST_RX_COMPLETE != 0) && (read2 & ST_RX_COMPLETE == 0);
    write_str(&mut serial, b"      verdict: ");
    if read_to_clear {
        write_str(&mut serial, b"READ-TO-CLEAR confirmed\r\n");
    } else if read1 & ST_RX_COMPLETE != 0 {
        write_str(
            &mut serial,
            b"flags persist across reads -- not read-to-clear\r\n",
        );
    } else {
        write_str(
            &mut serial,
            b"no flag even on the first read -- inconclusive\r\n",
        );
    }

    // ---- verdict ---------------------------------------------------------
    let handlers = HANDLER_RUNS.load(Ordering::Relaxed);
    let stormed = STORMED.load(Ordering::Relaxed) != 0;
    let first = FIRST_STATUS.load(Ordering::Relaxed);

    write_str(&mut serial, b"\r\n  handler entries: ");
    write_dec(&mut serial, handlers);
    if first != 0xFFFF_FFFF {
        write_str(&mut serial, b"\r\n  status on first entry: 0x");
        write_hex(&mut serial, first, 4);
    }
    write_str(&mut serial, b"\r\n\r\nverdict:\r\n");
    if stormed {
        write_str(
            &mut serial,
            b"  STORM: the handler re-entered without limit, so the condition\r\n\
              \x20 cannot be cleared by reading RxData. An SPI handler must mask\r\n\
              \x20 its source, exactly as GPIO's must.\r\n",
        );
    } else if handlers == 0 {
        write_str(
            &mut serial,
            b"  No interrupt was delivered at all, yet the transfer completed.\r\n\
              \x20 Suspect the enable bit or the PLIC line, not the transfer.\r\n",
        );
    } else if rx2.is_none() && read_to_clear {
        write_str(
            &mut serial,
            b"  The status register is READ-TO-CLEAR: the handler's single\r\n\
              \x20 status read consumed the receive flag that transfer_byte\r\n\
              \x20 polls, so the polled transfer waited forever. Nothing to do\r\n\
              \x20 with CSAAT or with stealing RxData.\r\n",
        );
    } else if rx2.is_none() {
        write_str(
            &mut serial,
            b"  The transfer stalled with the interrupt enabled but completed\r\n\
              \x20 without it, and no storm occurred -- yet the status register\r\n\
              \x20 is not read-to-clear either. Cause still unidentified.\r\n",
        );
    } else {
        write_str(
            &mut serial,
            b"  Everything completed here. The hang in spi-intr-selftest is in\r\n\
              \x20 what it does differently: re-arming inside the loop, or the\r\n\
              \x20 SpiBus path rather than raw register writes.\r\n",
        );
    }

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}
