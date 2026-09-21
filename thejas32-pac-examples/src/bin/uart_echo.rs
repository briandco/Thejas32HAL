//! UART echo — reads bytes from UART0 and writes them straight back.
//!
//! Uses `thejas32-hal`'s `Serial` driver instead of touching `thejas32-pac`
//! registers directly (compare with `led_blink.rs`).
//!
//! Adopts the bootloader's UART configuration (115200 8N1) rather than
//! reprogramming the baud divisor, since the UART input-clock frequency is
//! not reliably documented for this SoC.

#![no_std]
#![no_main]

use panic_halt as _;

use embedded_hal_nb::{nb, serial::Read, serial::Write};
use thejas32_hal::serial::Serial;

const BANNER: &[u8] = b"\r\nuart-echo ready\r\n";

#[riscv_rt::entry]
fn main() -> ! {
    // The PAC's singleton, taken once. `steal()` below (where present) is a
    // deliberate read-only alias for register inspection, not a second owner.
    let peripherals = thejas32_pac::Peripherals::take().unwrap();
    let uart0 = peripherals.uart0;
    let mut serial = Serial::new_preconfigured(uart0);

    // Prove TX works before we depend on RX.
    for &byte in BANNER {
        nb::block!(serial.write(byte)).ok();
    }

    loop {
        match nb::block!(serial.read()) {
            Ok(byte) => {
                nb::block!(serial.write(byte)).ok();
            }
            // Line errors (parity/overrun/framing) are reported as '!' so a
            // misconfigured link is visibly distinct from correct echoing.
            Err(_) => {
                nb::block!(serial.write(b'!')).ok();
            }
        }
    }
}
