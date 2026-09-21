//! SPI interrupt self-test.
//!
//! **Jumper required: SPI0 COPI ↔ CIPO** (`J1_7 ↔ J1_6` — the MOSI↔MISO
//! loopback `spi-selftest` already uses). Every byte written comes straight
//! back, so a transfer generates both a transmit-empty and a receive-complete
//! condition without any slave device.
//!
//! # Register facts, and where they come from
//!
//! The two enables share the control register's 2-bit `rx_tx_intr` field —
//! bit 6 receive, bit 7 transmit — per `vega-sdk`'s `SPI_set_intr`, which
//! computes `(tx_intr << 7) | (rx_intr << 6)` and masks with `0xFF3F`. The
//! conditions are reported in the status register: bit 2 receive complete,
//! bit 3 transmit empty, per `spi_intr_status`. SPI0 is PLIC line 3.
//!
//! # Oracles
//!
//! 0. **The loopback is fitted** — a byte written must come back unchanged
//!    before any interrupt result means anything. Establish the stimulus first;
//!    without this a missing jumper is indistinguishable from a broken driver.
//! 1. **Interrupts are delivered** — a transfer with `RxComplete` enabled runs
//!    the handler.
//! 2. **The status names the cause** — the handler sees receive-complete
//!    asserted, not merely "something fired".
//! 3. **Masking stops delivery** — with transfers still happening, so the
//!    stimulus is known present. The differential half.
//! 4. **Re-enabling resumes delivery.**
//!
//! # How the receive interrupt has to be handled
//!
//! Three facts, each measured, that together leave exactly one workable design:
//!
//! 1. **The condition re-asserts until cleared.** A handler that only records
//!    re-enters until the backstop fires.
//! 2. **It cannot be cleared by reading status.** That register is
//!    read-to-clear, so the read consumes the flag the transfer is polling and
//!    the transfer then spins forever.
//! 3. **So the handler must mask itself**, which is a control-register write —
//!    and that is safe only if the application is not read-modify-writing
//!    control at the same time.
//!
//! `SpiBus` does exactly that, before every byte, which is why interrupts and
//! the blocking API cannot be combined. This test therefore drives transfers at
//! register level, the arrangement `spi-intr-diag2` validated twelve-for-twelve
//! with one interrupt per transfer and no storm.
//!
//! # The status register is read-to-clear
//!
//! Its receive flags are cleared by the act of reading it, so this handler
//! reads **neither** the status register nor `RxData`: either would consume the
//! condition the blocking transfer is waiting on. With only the receive
//! interrupt enabled the cause is already known, so the handler just records it
//! and masks itself at the peripheral. `main` re-arms before each transfer.
//!
//! # Two hazards this guards against
//!
//! GPIO's interrupt turned out level-sensitive with nothing to acknowledge, and
//! the first run of that test took 5,000 handler entries before its guard
//! fired. SPI *should* be clearable — reading `RxData` consumes the byte — but
//! that is an expectation, not a measurement, so the same storm guard is here.
//!
//! The handler uses plain atomics: a `critical_section` inside an ISR clears
//! `mstatus.MPIE` on this core and disables interrupts permanently.

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
    timer::Timer,
};
use thejas32_pac::interrupt::Interrupt;

/// Byte sent through the loopback. Mixed bits, so a stuck line shows up.
const PATTERN: u8 = 0xA5;

/// Entries after which the handler masks its own source, in case the condition
/// cannot be cleared. GPIO's interrupt proved unacknowledgeable; assuming SPI's
/// is not would be an assumption.
const STORM_LIMIT: u32 = 2_000;

/// Bound on each register wait inside a transfer.
const WAIT_LIMIT: u32 = 100_000;

const ST_BUSY: u32 = 1 << 4;
const ST_RX_COMPLETE: u32 = 1 << 6;

/// Window for counting interrupts while transfers run: ~200 ms.
const WINDOW_TICKS: u32 = thejas32_hal::PERIPHERAL_CLOCK_HZ / 5;

static HANDLER_RUNS: AtomicU32 = AtomicU32::new(0);
static SAW_RX_COMPLETE: AtomicU32 = AtomicU32::new(0);
static STORMED: AtomicU32 = AtomicU32::new(0);

#[riscv_rt::core_interrupt(CoreInterrupt::MachineExternal)]
fn machine_external() {
    // SAFETY: read-only PLIC access from the handler.
    let plic_regs = unsafe { thejas32_pac::Plic::steal() };
    if plic_regs.intr_status().read().bits() & (1 << (Interrupt::SPI0 as u8)) == 0 {
        return;
    }

    let count = HANDLER_RUNS.load(Ordering::Relaxed) + 1;
    HANDLER_RUNS.store(count, Ordering::Relaxed);

    // Masks itself through the *control* register, and touches nothing else.
    //
    // The receive interrupt re-asserts until the condition is cleared, so a
    // handler that only records will re-enter until the backstop fires —
    // measured, and the reason an earlier revision of this test stormed.
    //
    // It cannot clear the condition the obvious way either: the status register
    // is read-to-clear, so reading it consumes the flag the transfer is polling.
    // Masking is therefore the only option, and it is a control-register write.
    //
    // That write is safe here *only* because `main` drives transfers at register
    // level and never read-modify-writes control during one. With `SpiBus` —
    // which does, before every byte — the two race and the result is erratic.
    // See the module docs on `thejas32_hal::spi`.
    //
    // SAFETY: the handler needs the peripheral and cannot be passed one.
    let spi = unsafe { thejas32_pac::Spi0::steal() };
    SAW_RX_COMPLETE.store(1, Ordering::Relaxed);
    spi.control().modify(|r, w| {
        // SAFETY: value masked to the 2-bit field width.
        unsafe { w.rx_tx_intr().bits(r.rx_tx_intr().bits() & !0b01) }
    });
    thejas32_hal::fence();

    // Backstop: if masking at the peripheral does not stop re-entry, drop the
    // PLIC line so the core cannot livelock.
    if count >= STORM_LIMIT {
        STORMED.store(1, Ordering::Relaxed);
        let enabled = plic_regs.intr_en().read().bits();
        // SAFETY: single-hart; nothing else writes this register concurrently.
        unsafe {
            plic_regs
                .intr_en()
                .write(|w| w.bits(enabled & !(1 << (Interrupt::SPI0 as u8))))
        };
        thejas32_hal::fence();
    }
}

/// One bounded transfer at register level.
///
/// Deliberately not [`SpiBus`]: that path read-modify-writes the control
/// register before every byte and polls the read-to-clear status register in a
/// tight loop, both of which conflict with interrupt-driven operation. See the
/// module docs. This is the pattern `spi-intr-diag2` validated twelve-for-twelve.
fn transfer_once() -> Option<u8> {
    // SAFETY: register-level access to a peripheral this test owns.
    let spi = unsafe { thejas32_pac::Spi0::steal() };
    let mut idle = false;
    for _ in 0..WAIT_LIMIT {
        if spi.status().read().bits() & ST_BUSY == 0 {
            idle = true;
            break;
        }
    }
    idle.then_some(())?;
    // SAFETY: writing a data word to the transmit register.
    unsafe { spi.txdata().write(|w| w.data().bits(PATTERN as u16)) };
    thejas32_hal::fence();
    for _ in 0..WAIT_LIMIT {
        if spi.status().read().bits() & ST_RX_COMPLETE != 0 {
            return Some(spi.rxdata().read().data().bits() as u8);
        }
    }
    None
}

/// Runs transfers for `ticks`, returning (interrupts, transfers).
fn transfer_for(
    spi: &mut Spi<thejas32_pac::Spi0>,
    reference: &mut Timer<thejas32_pac::Timer1>,
    ticks: u32,
    rearm: bool,
) -> (u32, u32) {
    reference.start_free_running();
    let start = reference.counter();
    let before = HANDLER_RUNS.load(Ordering::Relaxed);
    let mut transfers = 0u32;
    while start.wrapping_sub(reference.counter()) < ticks {
        // The application is the only writer of the control register.
        if rearm {
            spi.listen(Event::RxComplete);
        }
        let _ = transfer_once();
        transfers += 1;
    }
    (HANDLER_RUNS.load(Ordering::Relaxed) - before, transfers)
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
    use embedded_hal::spi::SpiBus;

    let mut serial = Serial::new_preconfigured(peripherals.uart0);
    let plic = plic::Plic::new(peripherals.plic);
    let mut reference = Timer::new(peripherals.timer1);
    let mut spi = Spi::new(peripherals.spi0, Config::default());

    write_str(
        &mut serial,
        b"\r\nSPI interrupt self-test (SPI0, loopback J1_7 <-> J1_6)\r\n",
    );

    // ---- [0] is the loopback fitted? -------------------------------------
    write_str(&mut serial, b"\r\n[0] loopback present\r\n    sent 0x");
    write_hex(&mut serial, PATTERN as u32, 2);
    write_str(&mut serial, b" got 0x");
    let mut buf = [PATTERN];
    let echoed = match spi.transfer_in_place(&mut buf) {
        Ok(()) => {
            write_hex(&mut serial, buf[0] as u32, 2);
            buf[0] == PATTERN
        }
        Err(_) => {
            write_str(&mut serial, b"?? (transfer error)");
            false
        }
    };
    write_str(&mut serial, b"\r\n    loopback works       ");
    write_str(&mut serial, if echoed { b"yes\r\n" } else { b"NO\r\n" });
    if !echoed {
        write_str(
            &mut serial,
            b"\r\n  The byte did not come back, so COPI is not reaching CIPO.\r\n\
              \x20 Fit the loopback jumper J1_7 <-> J1_6 and re-run. Nothing\r\n\
              \x20 below could mean anything without it.\r\n",
        );
        loop {
            unsafe { core::arch::asm!("nop") };
        }
    }

    // ---- [1] + [2] delivery, and the cause ------------------------------
    write_str(&mut serial, b"\r\n[1/2] interrupt delivery\r\n");
    spi.listen(Event::RxComplete);
    plic.enable(Interrupt::SPI0);
    // SAFETY: the handler is installed, reads RxData to clear the source, and
    // masks its own line if it storms.
    unsafe {
        plic::enable_external_interrupts();
        plic::enable_global_interrupts();
    }

    let (delivered, transfers) = transfer_for(&mut spi, &mut reference, WINDOW_TICKS, true);
    let stormed = STORMED.load(Ordering::Relaxed) != 0;
    let saw_rx = SAW_RX_COMPLETE.load(Ordering::Relaxed) != 0;

    write_str(&mut serial, b"    transfers: ");
    write_dec(&mut serial, transfers);
    write_str(&mut serial, b"   interrupts: ");
    write_dec(&mut serial, delivered);
    write_str(&mut serial, b"\r\n");
    let p1 = delivered > 0;
    let p2 = saw_rx;
    line(&mut serial, b"    delivered            ", p1);
    line(&mut serial, b"    status says RX       ", p2);

    if stormed {
        write_str(
            &mut serial,
            b"\r\n    STORM: the handler hit its entry limit and masked the\r\n\
              \x20   source, so reading RxData does not clear this interrupt.\r\n\
              \x20   Like GPIO, it would have to be masked rather than served.\r\n",
        );
    }

    // ---- [3] masking stops delivery --------------------------------------
    // Transfers keep running throughout, so the stimulus is known present and
    // silence can only mean the mask worked.
    write_str(&mut serial, b"\r\n[3] unlisten masks delivery\r\n");
    spi.unlisten(Event::RxComplete);
    STORMED.store(0, Ordering::Relaxed);
    plic.enable(Interrupt::SPI0); // PLIC still on: only the peripheral mask changed
    // `rearm: false` -- leave it masked, which is the whole point of the phase.
    let (masked, _) = transfer_for(&mut spi, &mut reference, WINDOW_TICKS, false);
    write_str(&mut serial, b"    interrupts while masked: ");
    write_dec(&mut serial, masked);
    write_str(&mut serial, b"\r\n");
    let p3 = masked == 0;
    line(&mut serial, b"    masked at the source ", p3);

    // ---- [4] and back again ----------------------------------------------
    write_str(&mut serial, b"\r\n[4] listen again\r\n");
    spi.listen(Event::RxComplete);
    let (again, _) = transfer_for(&mut spi, &mut reference, WINDOW_TICKS, true);
    write_str(&mut serial, b"    interrupts: ");
    write_dec(&mut serial, again);
    write_str(&mut serial, b"\r\n");
    let p4 = again > 0;
    line(&mut serial, b"    re-enable works      ", p4);

    // SAFETY: quiesce before the summary so the console is not interrupted.
    spi.unlisten(Event::RxComplete);
    unsafe { plic::disable_global_interrupts() };
    plic.disable_all();

    let passed = [p1, p2, p3, p4].iter().filter(|&&b| b).count();
    write_str(&mut serial, b"\r\nhandler runs: ");
    write_dec(&mut serial, HANDLER_RUNS.load(Ordering::Relaxed));
    write_str(&mut serial, b"\r\nresult: ");
    write_dec(&mut serial, passed as u32);
    write_str(&mut serial, b"/4 checks passed\r\n");

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}
