//! SPI interrupt diagnostic 2 — why is the delivery rate inconsistent?
//!
//! `spi-intr-selftest` passed its oracles but produced counts that contradict
//! each other: **1** interrupt across 51,021 transfers in phase 1, and a storm
//! into the 2,000 backstop in phase 4 — from the identical loop. One of those
//! is wrong and neither is understood.
//!
//! Reading the code cannot separate the candidates, so this measures the whole
//! state around **each individual transfer** and prints a line per iteration:
//!
//! * `arm` — the receive enable immediately after `listen()`. If this reads 0,
//!   `listen()` is not taking effect and everything downstream is explained.
//! * `int` — handler entries caused by this transfer.
//! * `aft` — the receive enable after the transfer, showing whether the handler
//!   masked it as intended.
//! * `MIE` — `mstatus.MIE`. If this reaches 0 the core has stopped taking
//!   interrupts entirely, which would look exactly like "delivery stopped".
//!   This part clears `MPIE` under `csrrci`, so an MIE that dies is a known
//!   failure mode here rather than a wild guess.
//! * `plic` — the PLIC enable mask, showing whether the storm backstop fired.
//!
//! Reading the *control* register is safe. Reading the *status* register is
//! not — its receive flags are read-to-clear (§ the read-to-clear finding), so
//! observing it would destroy what the transfer is waiting for. This diagnostic
//! therefore never reads status outside the transfer itself.
//!
//! The transfer is driven at register level with bounded waits, so a stalled
//! iteration reports rather than hanging.

#![no_std]
#![no_main]

use panic_halt as _;

use core::sync::atomic::{AtomicU32, Ordering};
use riscv::interrupt::Interrupt as CoreInterrupt;
use thejas32_examples::console::{write_dec, write_hex, write_str};
use thejas32_hal::{
    plic,
    serial::Serial,
    spi::{Config, Event, Spi},
};
use thejas32_pac::interrupt::Interrupt;

const PATTERN: u8 = 0xA5;
/// Bound on each register wait.
const LIMIT: u32 = 100_000;
/// Iterations to observe. Small, because each prints a line.
const ITERATIONS: u32 = 12;
/// Handler entries after which the PLIC mask is dropped entirely.
const STORM_LIMIT: u32 = 300;

const ST_RX_COMPLETE: u32 = 1 << 6;
const ST_BUSY: u32 = 1 << 4;

static HANDLER_RUNS: AtomicU32 = AtomicU32::new(0);
static STORMED: AtomicU32 = AtomicU32::new(0);

#[riscv_rt::core_interrupt(CoreInterrupt::MachineExternal)]
fn machine_external() {
    // SAFETY: read-only PLIC access from the handler.
    let plic_regs = unsafe { thejas32_pac::Plic::steal() };
    let count = HANDLER_RUNS.load(Ordering::Relaxed) + 1;
    HANDLER_RUNS.store(count, Ordering::Relaxed);

    // SAFETY: the handler needs the peripheral and cannot be passed one.
    let spi = unsafe { thejas32_pac::Spi0::steal() };
    // Never reads status: that register is read-to-clear and would consume the
    // flag the transfer below is polling. Masking is a control-register write.
    spi.control().modify(|r, w| {
        // SAFETY: value masked to the 2-bit field width.
        unsafe { w.rx_tx_intr().bits(r.rx_tx_intr().bits() & !0b01) }
    });
    thejas32_hal::fence();

    if count >= STORM_LIMIT {
        STORMED.store(1, Ordering::Relaxed);
        // SAFETY: single-hart; nothing else writes this register concurrently.
        unsafe { plic_regs.intr_en().write(|w| w.bits(0)) };
        thejas32_hal::fence();
    }
}

/// The receive-enable bit, read from the *control* register (safe to read).
fn rx_enabled() -> u32 {
    // SAFETY: read-only access to fixed MMIO.
    let spi = unsafe { thejas32_pac::Spi0::steal() };
    (spi.control().read().rx_tx_intr().bits() & 0b01) as u32
}

/// One bounded transfer at register level. Returns the byte, or `None`.
fn transfer() -> Option<u8> {
    // SAFETY: diagnostic access to fixed MMIO.
    let spi = unsafe { thejas32_pac::Spi0::steal() };
    let mut idle = false;
    for _ in 0..LIMIT {
        if spi.status().read().bits() & ST_BUSY == 0 {
            idle = true;
            break;
        }
    }
    if !idle {
        return None;
    }
    // SAFETY: writing a data word to the transmit register.
    unsafe { spi.txdata().write(|w| w.data().bits(PATTERN as u16)) };
    thejas32_hal::fence();
    for _ in 0..LIMIT {
        if spi.status().read().bits() & ST_RX_COMPLETE != 0 {
            return Some(spi.rxdata().read().data().bits() as u8);
        }
    }
    None
}

#[riscv_rt::entry]
fn main() -> ! {
    let mut serial = Serial::new_preconfigured(unsafe { thejas32_pac::Uart0::steal() });
    let plic = plic::Plic::new(unsafe { thejas32_pac::Plic::steal() });
    let spi = Spi::new(unsafe { thejas32_pac::Spi0::steal() }, Config::default());
    let plic_regs = unsafe { thejas32_pac::Plic::steal() };

    write_str(
        &mut serial,
        b"\r\nSPI interrupt diagnostic 2 -- per-transfer state\r\n",
    );

    plic.enable(Interrupt::SPI0);
    // SAFETY: the handler masks the source and drops the PLIC mask if it storms.
    unsafe {
        plic::enable_external_interrupts();
        plic::enable_global_interrupts();
    }

    write_str(
        &mut serial,
        b"\r\n  n  arm int aft MIE plic_en   byte\r\n\
          \x20 --  --- --- --- --- --------  ----\r\n",
    );

    for n in 0..ITERATIONS {
        spi.listen(Event::RxComplete);
        let armed = rx_enabled();
        let before = HANDLER_RUNS.load(Ordering::Relaxed);

        let byte = transfer();

        let fired = HANDLER_RUNS.load(Ordering::Relaxed) - before;
        let after = rx_enabled();
        let mie = riscv::register::mstatus::read().mie() as u32;
        let en = plic_regs.intr_en().read().bits();

        write_str(&mut serial, b"  ");
        write_dec(&mut serial, n);
        write_str(&mut serial, if n < 10 { b"   " } else { b"  " });
        write_dec(&mut serial, armed);
        write_str(&mut serial, b"   ");
        write_dec(&mut serial, fired);
        write_str(&mut serial, if fired < 10 { b"   " } else { b"  " });
        write_dec(&mut serial, after);
        write_str(&mut serial, b"   ");
        write_dec(&mut serial, mie);
        write_str(&mut serial, b"  0x");
        write_hex(&mut serial, en, 6);
        write_str(&mut serial, b"  ");
        match byte {
            Some(b) => {
                write_str(&mut serial, b"0x");
                write_hex(&mut serial, b as u32, 2);
            }
            None => write_str(&mut serial, b"STALL"),
        }
        write_str(&mut serial, b"\r\n");
    }

    // SAFETY: quiesce before the verdict.
    unsafe { plic::disable_global_interrupts() };
    plic.disable_all();

    write_str(&mut serial, b"\r\n  total handler runs: ");
    write_dec(&mut serial, HANDLER_RUNS.load(Ordering::Relaxed));
    write_str(&mut serial, b"\r\n  stormed: ");
    write_dec(&mut serial, STORMED.load(Ordering::Relaxed));
    write_str(
        &mut serial,
        b"\r\n\r\nreading the table:\r\n\
          \x20 arm=0        -> listen() is not taking effect\r\n\
          \x20 int=0 arm=1  -> armed but nothing delivered; check MIE and plic\r\n\
          \x20 MIE 1->0     -> the core stopped taking interrupts mid-run\r\n\
          \x20 int>1        -> more than one interrupt per transfer\r\n\
          \x20 aft=1        -> the handler did not mask, so it should storm\r\n",
    );

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}
