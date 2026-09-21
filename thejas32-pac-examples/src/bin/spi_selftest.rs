//! SPI self-test — validates the SPI driver over UART, no slave device needed.
//!
//! Three phases, in increasing dependence on external hardware:
//!
//! 1. **Control-word encoding** — writes a config, reads the register back,
//!    and checks every field decodes to what was requested. This validates
//!    the bit layout in `Config` against silicon without clocking anything.
//! 2. **State machine** — clocks bytes out and checks the status flags
//!    sequence (`TX_HOLD_EMPTY` → `RX_COMPLETE`, `BUSY` clears, no overrun).
//!    Polling here is **bounded**: the HAL's blocking loops would hang
//!    forever if a flag never sets, which on a never-before-tested peripheral
//!    would just look like a dead board. A `TIMEOUT` verdict is far more
//!    useful than silence.
//! 3. **Data path** — sends known patterns and reports what came back. With
//!    MOSI and MISO left alone this only shows the idle level; jumper
//!    **MOSI to MISO** for a true loopback, where the driver is proven
//!    end-to-end because received must equal sent.

#![no_std]
#![no_main]

use panic_halt as _;

use embedded_hal::spi::SpiBus;
use thejas32_examples::console::{write_hex, write_str};
use thejas32_hal::{
    serial::Serial,
    spi::{BitOrder, ClockDivider, Config, Mode, Spi},
};

/// Bound on status-flag polling, in loop iterations.
const POLL_LIMIT: u32 = 200_000;

/// Patterns chosen to catch stuck/shorted bits and bit-order errors.
const PATTERNS: [u8; 5] = [0xA5, 0x5A, 0xFF, 0x00, 0x01];

#[riscv_rt::entry]
fn main() -> ! {
    // The PAC's singleton, taken once. `steal()` below (where present) is a
    // deliberate read-only alias for register inspection, not a second owner.
    let peripherals = thejas32_pac::Peripherals::take().unwrap();
    let mut serial = Serial::new_preconfigured(peripherals.uart0);
    write_str(&mut serial, b"\r\nspi self-test (SPI0)\r\n");

    // ---- phase 1: control-word encoding ----------------------------------
    write_str(&mut serial, b"\r\n[1] control word readback\r\n");
    let mut pass = 0u32;
    let mut total = 0u32;

    let cases = [
        Config::default(),
        Config::default()
            .with_mode(Mode::Mode0)
            .with_bit_order(BitOrder::LsbFirst)
            .with_chip_select(2)
            .with_divider(ClockDivider::Div4),
        Config::default()
            .with_mode(Mode::Mode1)
            .with_chip_select(3)
            .with_divider(ClockDivider::Div2048)
            .with_bits_per_transfer(12),
    ];

    // Ownership is threaded through the loop with `free()` rather than stolen
    // afresh each iteration: one peripheral, one owner at a time.
    let mut spi0 = peripherals.spi0;
    for cfg in cases {
        let spi = Spi::new(spi0, cfg);
        // Read the raw registers back through a separate PAC handle.
        let raw = unsafe { thejas32_pac::Spi0::steal() };
        let ctrl = raw.control().read();
        let baud = raw.baudrate().read().bits();

        let ok = ctrl.mode().bits() == cfg.mode as u8
            && ctrl.data_order().bit() == (cfg.bit_order == BitOrder::LsbFirst)
            && ctrl.periph_cs().bits() == cfg.chip_select
            && ctrl.dbits().bits() == cfg.bits_per_transfer
            && !ctrl.periph().bit()
            && (baud >> 4) == cfg.divider as u32;

        total += 1;
        if ok {
            pass += 1;
        }

        write_str(&mut serial, b"  mode=");
        write_hex(&mut serial, cfg.mode as u32, 1);
        write_str(&mut serial, b" cs=");
        write_hex(&mut serial, cfg.chip_select as u32, 1);
        write_str(&mut serial, b" dbits=");
        write_hex(&mut serial, cfg.bits_per_transfer as u32, 2);
        write_str(&mut serial, b"  ctrl=");
        write_hex(&mut serial, ctrl.bits(), 4);
        write_str(&mut serial, b" baud=");
        write_hex(&mut serial, baud, 4);
        write_str(&mut serial, if ok { b"  PASS\r\n" } else { b"  FAIL\r\n" });

        spi0 = spi.free();
    }

    // ---- phase 2: state machine (bounded) --------------------------------
    write_str(&mut serial, b"\r\n[2] state machine (bounded poll)\r\n");
    let raw = unsafe { thejas32_pac::Spi0::steal() };
    let configured = Spi::new(spi0, Config::default());
    spi0 = configured.free();

    let idle_ok = poll(POLL_LIMIT, || !raw.status().read().busy().bit_is_set());
    report(&mut serial, b"  busy clears at idle  ", idle_ok);

    let thre_ok = poll(POLL_LIMIT, || {
        raw.status().read().tx_hold_empty().bit_is_set()
    });
    report(&mut serial, b"  tx_hold_empty set    ", thre_ok);

    // Clock one byte out and wait, bounded, for receive-complete.
    let rx_ok = if thre_ok {
        unsafe { raw.txdata().write(|w| w.data().bits(0xA5)) };
        poll(POLL_LIMIT, || {
            raw.status().read().rx_complete().bit_is_set()
        })
    } else {
        false
    };
    report(&mut serial, b"  rx_complete set      ", rx_ok);

    let first = raw.rxdata().read().data().bits() as u8;
    let overrun = raw.status().read().overrun().bit_is_set();
    report(&mut serial, b"  no overrun           ", !overrun);

    if !rx_ok {
        write_str(
            &mut serial,
            b"\r\n  rx_complete never set -- stopping before the blocking\r\n\
              \x20 HAL path, which would hang here. Check SPI0 pinmux/clock.\r\n",
        );
        halt();
    }

    // ---- phase 3: data path ----------------------------------------------
    write_str(&mut serial, b"\r\n[3] data path\r\n");
    let mut spi = Spi::new(spi0, Config::default());

    let mut echoed = 0u32;
    let mut all_same = true;
    for (i, &pattern) in PATTERNS.iter().enumerate() {
        let mut buf = [pattern];
        let got = match spi.transfer_in_place(&mut buf) {
            Ok(()) => buf[0],
            Err(_) => {
                write_str(&mut serial, b"  transfer error\r\n");
                break;
            }
        };
        if got == pattern {
            echoed += 1;
        }
        if got != first && i > 0 {
            all_same = false;
        }

        write_str(&mut serial, b"  sent=");
        write_hex(&mut serial, pattern as u32, 2);
        write_str(&mut serial, b" got=");
        write_hex(&mut serial, got as u32, 2);
        write_str(&mut serial, b"\r\n");
    }

    // ---- verdict ----------------------------------------------------------
    write_str(&mut serial, b"\r\nphase 1: ");
    write_hex(&mut serial, pass, 1);
    write_str(&mut serial, b"/");
    write_hex(&mut serial, total, 1);
    write_str(&mut serial, b" config cases passed\r\n");

    write_str(&mut serial, b"verdict: ");
    if echoed as usize == PATTERNS.len() {
        write_str(
            &mut serial,
            b"LOOPBACK OK -- data path validated end to end\r\n",
        );
    } else if all_same {
        write_str(
            &mut serial,
            b"MISO idle (no loopback) -- clocking works, data path\r\n\
              \x20        unproven. Jumper MOSI to MISO and re-run.\r\n",
        );
    } else {
        write_str(
            &mut serial,
            b"data varied but did not match sent -- investigate\r\n",
        );
    }

    halt();
}

/// Polls `cond` up to `limit` times; true if it became true in the budget.
fn poll<F: Fn() -> bool>(limit: u32, cond: F) -> bool {
    for _ in 0..limit {
        if cond() {
            return true;
        }
    }
    false
}

fn report<W: embedded_hal_nb::serial::Write<u8>>(serial: &mut W, label: &[u8], ok: bool) {
    write_str(serial, label);
    write_str(serial, if ok { b"PASS\r\n" } else { b"TIMEOUT\r\n" });
}

fn halt() -> ! {
    loop {
        unsafe { core::arch::asm!("nop") };
    }
}
