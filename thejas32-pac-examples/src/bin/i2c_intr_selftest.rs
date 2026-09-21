//! I2C interrupt self-test — the last 4 of the 23 PLIC lines.
//!
//! **No jumper needed.** The stimulus is a write to an address nobody answers:
//! the controller still drives START and clocks out the address byte, so a
//! transmit event occurs whether or not anything acknowledges. I2C0 and I2C1
//! are empty on this board (`i2c-scan-all`), which makes an unanswered write
//! the most repeatable stimulus available. The board's only device, the
//! ADS1015, sits on I2C2 — which has no PLIC line at all, so it cannot be used
//! here.
//!
//! # Register facts, and where they come from
//!
//! `IER` bit 1 enables the transmit interrupt and bit 2 the receive interrupt,
//! from `vega-sdk`'s `I2C_enable_intr`: `(rx_intr << 2) | (tx_intr << 1) | 1`.
//! Bit 0 is set unconditionally there and its meaning is undocumented, so this
//! driver mirrors that rather than guessing. `SR1` bit 1 reports a pending
//! transmit interrupt, per `I2C_intr_handler`. I2C0 is PLIC line 5.
//!
//! # Why this should behave better than SPI
//!
//! SPI's interrupt support fought the polled driver on two fronts: its status
//! register is read-to-clear, and `SpiBus` read-modify-writes the control
//! register before every byte. Neither applies here. The I2C data path writes
//! `cr`, `txff` and `txclr` **whole**, never read-modify-write, and never
//! touches `IER` at all — so an interrupt handler that owns `IER` shares nothing
//! with a transfer in flight.
//!
//! That is a prediction from reading the code, so phase 1 tests it: `SR1` is
//! read twice in a row and the value must not change. If these registers turn
//! out to be read-to-clear like SPI's, that assumption is wrong and everything
//! after it is suspect.
//!
//! # Oracles
//!
//! 1. **`SR1` survives being read** — the assumption everything else rests on.
//! 2. **Interrupts are delivered** — an unanswered write runs the handler.
//! 3. **Masking stops delivery** — with writes still happening, so the stimulus
//!    is known present.
//! 4. **Re-enabling resumes delivery.**
//!
//! The handler uses plain atomics and never reads a status register: a
//! `critical_section` inside an ISR clears `mstatus.MPIE` on this core, and a
//! status read is how the SPI equivalent deadlocked. It masks itself via `IER`,
//! which nothing else writes during a transfer.

#![no_std]
#![no_main]

use panic_halt as _;

use core::sync::atomic::{AtomicU32, Ordering};
use riscv::interrupt::Interrupt as CoreInterrupt;
use thejas32_examples::console::{write_dec, write_hex, write_str};
use thejas32_hal::{
    PERIPHERAL_CLOCK_HZ,
    i2c::{Event, I2c},
    plic,
    serial::Serial,
    timer::Timer,
};
use thejas32_pac::interrupt::Interrupt;

const BUS_HZ: u32 = 100_000;
/// Nothing answers here — confirmed by `i2c-scan-all`, which found I2C0 empty.
const ABSENT_ADDRESS: u8 = 0x50;
/// Measurement window: ~200 ms.
const WINDOW_TICKS: u32 = PERIPHERAL_CLOCK_HZ / 5;
/// Handler entries **per phase** after which it drops its own PLIC line.
///
/// A legitimate phase delivers one interrupt per write — measured at 1,716 in a
/// 200 ms window — so the first version's 2,000 *total* tripped on ordinary
/// traffic and truncated phase 4. The counter is reset per phase and the limit
/// set an order of magnitude above real traffic: a genuine runaway is
/// unbounded, so anything in this range is not one.
const STORM_LIMIT: u32 = 20_000;

static HANDLER_RUNS: AtomicU32 = AtomicU32::new(0);
static STORMED: AtomicU32 = AtomicU32::new(0);

#[riscv_rt::core_interrupt(CoreInterrupt::MachineExternal)]
fn machine_external() {
    // SAFETY: read-only PLIC access from the handler.
    let plic_regs = unsafe { thejas32_pac::Plic::steal() };
    if plic_regs.intr_status().read().bits() & (1 << (Interrupt::I2C0 as u8)) == 0 {
        return;
    }

    let count = HANDLER_RUNS.load(Ordering::Relaxed) + 1;
    HANDLER_RUNS.store(count, Ordering::Relaxed);

    // Masks itself through IER, and reads no status register.
    //
    // Only the transmit interrupt is enabled, so the cause is already known.
    // IER is untouched by the data path — `cr`, `txff` and `txclr` are written
    // whole — so this cannot race a transfer, unlike the SPI equivalent.
    //
    // SAFETY: the handler needs the peripheral and cannot be passed one.
    let i2c = unsafe { thejas32_pac::I2c0::steal() };
    unsafe { i2c.ier().write(|w| w.bits(0)) };
    thejas32_hal::fence();

    if count >= STORM_LIMIT {
        STORMED.store(1, Ordering::Relaxed);
        let enabled = plic_regs.intr_en().read().bits();
        // SAFETY: single-hart; nothing else writes this register concurrently.
        unsafe {
            plic_regs
                .intr_en()
                .write(|w| w.bits(enabled & !(1 << (Interrupt::I2C0 as u8))))
        };
        thejas32_hal::fence();
    }
}

/// Issues unanswered writes for `ticks`, returning (interrupts, attempts).
fn drive_for(
    i2c: &mut I2c<thejas32_pac::I2c0>,
    reference: &mut Timer<thejas32_pac::Timer1>,
    ticks: u32,
    rearm: bool,
) -> (u32, u32) {
    // Per-phase accounting: the storm cap should measure this phase, not the
    // sum of everything before it.
    HANDLER_RUNS.store(0, Ordering::Relaxed);
    STORMED.store(0, Ordering::Relaxed);
    reference.start_free_running();
    let start = reference.counter();
    let before = HANDLER_RUNS.load(Ordering::Relaxed);
    let mut attempts = 0u32;
    while start.wrapping_sub(reference.counter()) < ticks {
        // Arming happens between transfers, never during, so `main` and the
        // handler are never writing IER at the same time.
        if rearm {
            i2c.listen(Event::TxComplete);
        }
        // A NACK is the expected outcome and not an error here.
        let _ = i2c.probe(ABSENT_ADDRESS);
        attempts += 1;
    }
    (HANDLER_RUNS.load(Ordering::Relaxed) - before, attempts)
}

fn line<W: embedded_hal_nb::serial::Write<u8>>(serial: &mut W, label: &[u8], ok: bool) {
    write_str(serial, label);
    write_str(serial, if ok { b"PASS\r\n" } else { b"FAIL\r\n" });
}

#[riscv_rt::entry]
fn main() -> ! {
    // The PAC's singleton, taken once. `steal()` below (where present) is a
    // deliberate read-only alias for register inspection, not a second owner.
    let peripherals = thejas32_pac::Peripherals::take().unwrap();
    let mut serial = Serial::new_preconfigured(peripherals.uart0);
    let plic = plic::Plic::new(peripherals.plic);
    let mut reference = Timer::new(peripherals.timer1);
    let mut i2c = I2c::new(peripherals.i2c0, PERIPHERAL_CLOCK_HZ, BUS_HZ);

    write_str(
        &mut serial,
        b"\r\nI2C interrupt self-test (I2C0, empty bus)\r\n",
    );

    // ---- [1] is SR1 stable across reads? --------------------------------
    // SPI's status register is read-to-clear, which deadlocked its interrupt
    // support. Establish that I2C's is not before relying on it.
    write_str(&mut serial, b"\r\n[1] SR1 survives being read\r\n");
    let raw = unsafe { thejas32_pac::I2c0::steal() };
    let _ = i2c.probe(ABSENT_ADDRESS);
    let first = raw.sr1().read().bits();
    let second = raw.sr1().read().bits();
    write_str(&mut serial, b"    read 1: 0x");
    write_hex(&mut serial, first as u32, 2);
    write_str(&mut serial, b"   read 2: 0x");
    write_hex(&mut serial, second as u32, 2);
    write_str(&mut serial, b"\r\n");
    let p1 = first == second;
    line(&mut serial, b"    stable               ", p1);
    if !p1 {
        write_str(
            &mut serial,
            b"    SR1 is read-to-clear like SPI's status register. The handler\r\n\
              \x20   below must not read it, and the polled driver's NACK\r\n\
              \x20   detection needs reviewing.\r\n",
        );
    }

    // ---- [2] delivery ----------------------------------------------------
    write_str(&mut serial, b"\r\n[2] interrupt delivery\r\n");
    plic.enable(Interrupt::I2C0);
    // SAFETY: the handler masks its own source and drops its PLIC line if it
    // storms.
    unsafe {
        plic::enable_external_interrupts();
        plic::enable_global_interrupts();
    }

    let (delivered, attempts) = drive_for(&mut i2c, &mut reference, WINDOW_TICKS, true);
    write_str(&mut serial, b"    writes: ");
    write_dec(&mut serial, attempts);
    write_str(&mut serial, b"   interrupts: ");
    write_dec(&mut serial, delivered);
    write_str(&mut serial, b"\r\n");
    let p2 = delivered > 0;
    line(&mut serial, b"    delivered            ", p2);

    if STORMED.load(Ordering::Relaxed) != 0 {
        write_str(
            &mut serial,
            b"\r\n    STORM: masking via IER did not stop re-entry, so the\r\n\
              \x20   condition survives it. The PLIC line had to be dropped.\r\n",
        );
    }

    // ---- [3] masking stops delivery --------------------------------------
    // Writes continue throughout, so the stimulus is known present and silence
    // can only mean the mask worked.
    write_str(&mut serial, b"\r\n[3] unlisten masks delivery\r\n");
    i2c.unlisten(Event::TxComplete);
    let (masked, _) = drive_for(&mut i2c, &mut reference, WINDOW_TICKS, false);
    write_str(&mut serial, b"    interrupts while masked: ");
    write_dec(&mut serial, masked);
    write_str(&mut serial, b"\r\n");
    let p3 = masked == 0;
    line(&mut serial, b"    masked               ", p3);

    // ---- [4] and back again ----------------------------------------------
    write_str(&mut serial, b"\r\n[4] listen again\r\n");
    STORMED.store(0, Ordering::Relaxed);
    plic.enable(Interrupt::I2C0);
    let (again, _) = drive_for(&mut i2c, &mut reference, WINDOW_TICKS, true);
    write_str(&mut serial, b"    interrupts: ");
    write_dec(&mut serial, again);
    write_str(&mut serial, b"\r\n");
    let p4 = again > 0;
    line(&mut serial, b"    re-enable works      ", p4);

    // SAFETY: quiesce before the summary so the console is not interrupted.
    i2c.unlisten(Event::TxComplete);
    unsafe { plic::disable_global_interrupts() };
    plic.disable_all();

    let passed = [p1, p2, p3, p4].iter().filter(|&&b| b).count();
    write_str(&mut serial, b"\r\nresult: ");
    write_dec(&mut serial, passed as u32);
    write_str(&mut serial, b"/4 checks passed\r\n");

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}
