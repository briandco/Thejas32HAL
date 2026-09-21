//! UART receive-interrupt self-test.
//!
//! `serial::listen`/`unlisten` have existed since the driver was written and
//! have **never been exercised on hardware**. That is the same class of
//! unverified claim that I2C's ACK path turned out to be — it carried a ⚠ for
//! months and, when a device finally appeared on the bus, reads failed on the
//! first try. An API that compiles is not an API that works.
//!
//! Needs no extra hardware: the console itself is the signal source. Press the
//! key it asks for.
//!
//! # Oracles
//!
//! 1. **The interrupt is delivered** — pressing a key runs the handler. A
//!    bounded wait, so a broken chain times out rather than hanging.
//! 2. **The right byte arrives** — the handler captures what was typed and it
//!    must equal the key requested. Merely counting handler entries would pass
//!    on a driver that fires spuriously and reads garbage.
//! 3. **Masking gates the interrupt but not the data** — after `unlisten`, a
//!    keypress must *not* run the handler, and the byte must still be readable
//!    by polling. This is the differential half: phase 1 alone would pass on an
//!    implementation where `listen` did nothing and the handler ran for some
//!    other reason.
//! 4. **`listen` is reversible** — re-enabling delivers again, so `unlisten`
//!    masks rather than permanently breaking the source.
//!
//! # No critical section in the handler
//!
//! The handler uses plain atomics. Taking a `critical_section` inside an ISR on
//! this core clears `mstatus.MPIE`, and `mret` then restores `MIE` from zero —
//! interrupts stop for good after the first one. See the crate docs and
//! `async-diag8`. riscv32im has load/store atomics and lacks only CAS, so
//! `store`/`load` are available and no critical section is needed.

#![no_std]
#![no_main]

use panic_halt as _;

use core::sync::atomic::{AtomicU32, Ordering};
use embedded_hal_nb::serial::Read;
use riscv::interrupt::Interrupt as CoreInterrupt;
use thejas32_examples::console::{write_dec, write_hex, write_str};
use thejas32_hal::{plic, serial::Serial, timer::Timer};
use thejas32_pac::interrupt::Interrupt;

/// Handler entries.
static HANDLER_RUNS: AtomicU32 = AtomicU32::new(0);
/// Last byte the handler read, or `NO_BYTE`.
static LAST_BYTE: AtomicU32 = AtomicU32::new(NO_BYTE);
/// Sentinel outside the `u8` range, so 0x00 is a legitimate value.
const NO_BYTE: u32 = 0x100;

/// The key the operator is asked to press. Checking the value rules out a
/// handler that fires but reads nothing useful.
const EXPECTED_KEY: u8 = b'a';

/// How long to wait for a keypress, in reference-timer ticks. 100 MHz, so this
/// is ~10 s — long enough to type, short enough that a hang is obvious.
const WAIT_TICKS: u32 = 1_000_000_000;

#[riscv_rt::core_interrupt(CoreInterrupt::MachineExternal)]
fn machine_external() {
    // SAFETY: read-only PLIC access from the handler.
    let plic_regs = unsafe { thejas32_pac::Plic::steal() };
    if plic_regs.intr_status().read().bits() & (1 << (Interrupt::UART0 as u8)) == 0 {
        return;
    }

    HANDLER_RUNS.store(HANDLER_RUNS.load(Ordering::Relaxed) + 1, Ordering::Relaxed);

    // SAFETY: the handler needs the peripheral and cannot be passed one.
    let uart = unsafe { thejas32_pac::Uart0::steal() };
    // Reading IIR acknowledges; reading RBR is what actually clears a receive
    // interrupt. Without the RBR read this handler would re-enter forever.
    let _ = uart.iir().read().bits();
    if uart.lsr().read().dr().bit_is_set() {
        let byte = uart.rbr().read().data().bits();
        LAST_BYTE.store(byte as u32, Ordering::Relaxed);
    }
}

/// Waits up to `ticks` for `cond`, using the free-running reference timer.
/// Returns true if it became true in time.
fn wait_for<F: Fn() -> bool>(
    reference: &mut Timer<thejas32_pac::Timer1>,
    ticks: u32,
    cond: F,
) -> bool {
    reference.start_free_running();
    let start = reference.counter();
    loop {
        if cond() {
            return true;
        }
        if start.wrapping_sub(reference.counter()) >= ticks {
            return false;
        }
    }
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

    write_str(&mut serial, b"\r\nUART RX interrupt self-test\r\n");

    plic.enable(Interrupt::UART0);
    // SAFETY: the handler above is installed and reads RBR, which clears the
    // source; it cannot re-enter indefinitely.
    unsafe {
        plic::enable_external_interrupts();
        plic::enable_global_interrupts();
    }

    // ---- [1] + [2] delivery, and the right byte -------------------------
    write_str(&mut serial, b"\r\n[1/2] interrupt delivery\r\n");
    serial.listen();
    LAST_BYTE.store(NO_BYTE, Ordering::Relaxed);
    let before = HANDLER_RUNS.load(Ordering::Relaxed);

    write_str(&mut serial, b"    press the 'a' key... ");
    let fired = wait_for(&mut reference, WAIT_TICKS, || {
        HANDLER_RUNS.load(Ordering::Relaxed) > before
    });
    let got = LAST_BYTE.load(Ordering::Relaxed);
    write_str(
        &mut serial,
        if fired {
            b"got it\r\n"
        } else {
            b"TIMED OUT\r\n"
        },
    );

    write_str(&mut serial, b"    byte received: ");
    if got == NO_BYTE {
        write_str(&mut serial, b"none");
    } else {
        write_str(&mut serial, b"0x");
        write_hex(&mut serial, got, 2);
    }
    write_str(&mut serial, b"  (want 0x");
    write_hex(&mut serial, EXPECTED_KEY as u32, 2);
    write_str(&mut serial, b")\r\n");

    let p1 = fired;
    let p2 = got == EXPECTED_KEY as u32;
    line(&mut serial, b"    interrupt delivered  ", p1);
    line(&mut serial, b"    byte is correct      ", p2);

    // ---- [3] masking gates the interrupt, not the data ------------------
    //
    // The oracle here has to be ordered carefully. Waiting a fixed window and
    // calling silence a pass is wrong: "no interrupt" is also what you get when
    // nobody pressed a key, so silence proves nothing on its own. An earlier
    // revision did exactly that and scored a FAIL that was purely an operator
    // timing artifact.
    //
    // So: wait for the byte to *arrive* first, watching `LSR.DR` without
    // consuming it, and only then ask whether the handler ran. If no byte ever
    // shows up the phase reports itself inconclusive rather than failing.
    write_str(&mut serial, b"\r\n[3] unlisten masks delivery\r\n");
    serial.unlisten();
    // Drain anything already buffered, so the next byte is unambiguous.
    while serial.read().is_ok() {}
    let quiet_before = HANDLER_RUNS.load(Ordering::Relaxed);

    // SAFETY: read-only status access; the byte is left in RBR deliberately.
    let uart = unsafe { thejas32_pac::Uart0::steal() };
    write_str(&mut serial, b"    press any key... ");
    let arrived = wait_for(&mut reference, WAIT_TICKS, || {
        uart.lsr().read().dr().bit_is_set()
    });
    // Sampled only once a byte is known to have arrived.
    let ran_while_masked = HANDLER_RUNS.load(Ordering::Relaxed) > quiet_before;
    write_str(
        &mut serial,
        if arrived {
            b"byte arrived\r\n"
        } else {
            b"NO KEY PRESSED\r\n"
        },
    );

    write_str(&mut serial, b"    handler ran: ");
    write_str(&mut serial, if ran_while_masked { b"yes" } else { b"no" });
    write_str(&mut serial, b"\r\n");

    // Now consume it: masking must not have swallowed the data.
    let polled = serial.read().ok();
    write_str(&mut serial, b"    polled read: ");
    match polled {
        Some(byte) => {
            write_str(&mut serial, b"0x");
            write_hex(&mut serial, byte as u32, 2);
        }
        None => write_str(&mut serial, b"nothing"),
    }
    write_str(&mut serial, b"\r\n");

    let p3 = arrived && !ran_while_masked && polled.is_some();
    if !arrived {
        write_str(
            &mut serial,
            b"    masked, data intact  INCONCLUSIVE (no key)\r\n",
        );
    } else {
        line(&mut serial, b"    masked, data intact  ", p3);
    }

    // ---- [4] and it comes back ------------------------------------------
    write_str(&mut serial, b"\r\n[4] listen again\r\n");
    serial.listen();
    while serial.read().is_ok() {}
    let again_before = HANDLER_RUNS.load(Ordering::Relaxed);
    write_str(&mut serial, b"    press any key... ");
    let p4 = wait_for(&mut reference, WAIT_TICKS, || {
        HANDLER_RUNS.load(Ordering::Relaxed) > again_before
    });
    write_str(
        &mut serial,
        if p4 { b"got it\r\n" } else { b"TIMED OUT\r\n" },
    );
    line(&mut serial, b"    re-enable works      ", p4);

    // SAFETY: quiesce before the summary so the console is not interrupted.
    serial.unlisten();
    unsafe { plic::disable_global_interrupts() };
    plic.disable_all();

    let passed = [p1, p2, p3, p4].iter().filter(|&&b| b).count();
    // p3 is false when inconclusive, so an unpressed key never scores a pass.
    write_str(&mut serial, b"\r\nhandler runs: ");
    write_dec(&mut serial, HANDLER_RUNS.load(Ordering::Relaxed));
    write_str(&mut serial, b"\r\nresult: ");
    write_dec(&mut serial, passed as u32);
    write_str(&mut serial, b"/4 checks passed\r\n");

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}
