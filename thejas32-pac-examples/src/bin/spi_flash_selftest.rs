//! SPI3 boot-flash self-test: checks the HAL's word-size encoding by timing,
//! then reads the ARIES v3 boot flash through the HAL's public `SpiBus` API.
//!
//! No jumpers. The flash (AT25SF161, JEDEC `1f:86:01`) is on SPI3, chip
//! select 0 — established by disassembling CDAC's `flasher.bin` and measured
//! in rustBoot's SPI diagnostic (SPI2 reads `00:00:00`).
//!
//! 1. **Word size, by timing.** At a fixed SPI clock a transfer lasts in
//!    proportion to its bit count. Timing the same transfers at /32 and /64
//!    cancels the fixed software overhead: the difference is 32 timer ticks
//!    per bit time. Every transfer also carries 2 bit times of controller
//!    overhead, so the check is relative: `bits_per_transfer` N must take
//!    exactly N - 8 bit times longer than 8. Run on SPI0 and SPI3. Harmless
//!    to the flash: it is only sent `0x00`, which is not a command.
//!
//!    The raw field was measured the same way before the HAL fix (writing
//!    DBITS directly): DBITS 0, 1, 4, 7, 8 took 10, 11, 14, 17, 18 bit times,
//!    and 12 and 15 also 18. So a transfer is `8 + DBITS` bits, at most 16.
//! 2. **Flash through `SpiBus`**, with the default `Config`:
//!    - `transfer` of `[0x9F, 0, 0, 0]` returns the JEDEC ID `1f 86 01`;
//!    - a `0x03` read at address 0 returns `97 02 00 00`, if rustBoot is
//!      provisioned (its first instruction is `auipc t0, 0`); otherwise the
//!      bytes are shown and the check is reported as not applicable;
//!    - two ID reads in a row agree, so chip select is released between
//!      transactions.
//!
//! Load into RAM with `just flash-run spi-flash-selftest` (J12 open).

#![no_std]
#![no_main]

use panic_halt as _;

use core::fmt::Write;
use embedded_hal::spi::SpiBus;
use thejas32_hal::{
    pac,
    serial::Serial,
    spi::{ClockDivider, Config, Spi, SpiX},
    timer::Timer,
};

const TRANSFERS: u32 = 64;

/// Average timer ticks (100 MHz) per `transfer_byte`.
fn ticks_per_transfer<S: SpiX>(spi: &mut Spi<S>, timer: &Timer<pac::Timer0>) -> u32 {
    let t0 = timer.counter();
    for _ in 0..TRANSFERS {
        let _ = spi.transfer_byte(0x00);
    }
    t0.wrapping_sub(timer.counter()) / TRANSFERS // down-counter
}

/// Bit times per transfer for a `bits_per_transfer` setting, by the /32 vs
/// /64 difference. Includes the controller's fixed 2 bit times.
fn bit_times<S: SpiX>(make: impl Fn() -> S, bits: u8, timer: &Timer<pac::Timer0>) -> u32 {
    let cfg = Config::default().with_bits_per_transfer(bits);
    let mut fast = Spi::new(make(), cfg.with_divider(ClockDivider::Div32));
    let t32 = ticks_per_transfer(&mut fast, timer);
    let mut slow = Spi::new(make(), cfg.with_divider(ClockDivider::Div64));
    let t64 = ticks_per_transfer(&mut slow, timer);
    // 32 ticks per bit time; round to nearest.
    (t64.saturating_sub(t32) + 16) / 32
}

struct Checks<'a, W: Write> {
    out: &'a mut W,
    n: u32,
    passed: u32,
}

impl<W: Write> Checks<'_, W> {
    fn check(&mut self, name: &str, ok: bool) {
        self.n += 1;
        self.passed += ok as u32;
        let _ = writeln!(
            self.out,
            "[{}] {:<42} {}\r",
            self.n,
            name,
            if ok { "PASS" } else { "FAIL" }
        );
    }
}

#[riscv_rt::entry]
fn main() -> ! {
    // SAFETY: single-threaded test; each controller is re-created per
    // measurement and only one instance is used at a time.
    let p = unsafe { pac::Peripherals::steal() };
    let mut uart = Serial::new_preconfigured(p.uart0);
    let mut timer = Timer::new(p.timer0);
    timer.start_free_running();

    let _ = writeln!(
        uart,
        "\r\nSPI flash self-test (SPI3 boot flash, no jumpers)\r"
    );
    let _ = writeln!(
        uart,
        "word size by timing (bit times, incl. 2 of controller overhead)\r"
    );
    let settings = [0u8, 8, 9, 12, 16, 20];
    let mut spi0 = [0u32; 6];
    let mut spi3 = [0u32; 6];
    for (i, &bits) in settings.iter().enumerate() {
        spi0[i] = bit_times(|| unsafe { pac::Peripherals::steal() }.spi0, bits, &timer);
        spi3[i] = bit_times(|| unsafe { pac::Peripherals::steal() }.spi3, bits, &timer);
        let _ = writeln!(
            uart,
            "  bits_per_transfer={:>2}: SPI0 {:>2}  SPI3 {:>2}\r",
            bits, spi0[i], spi3[i]
        );
    }

    let mut c = Checks {
        out: &mut uart,
        n: 0,
        passed: 0,
    };
    // Relative to 8 bits: 9 -> +1, 12 -> +4, 16 -> +8; out-of-range values
    // clamp to 8 and 16.
    let relative = |t: &[u32; 6]| {
        t[2] == t[1] + 1 && t[3] == t[1] + 4 && t[4] == t[1] + 8 && t[0] == t[1] && t[5] == t[4]
    };
    c.check(
        "N-bit setting is N-8 bit times over 8 (SPI0)",
        relative(&spi0),
    );
    c.check(
        "N-bit setting is N-8 bit times over 8 (SPI3)",
        relative(&spi3),
    );
    c.check(
        "Config::default() is 8 bits",
        Config::default().bits_per_transfer == 8,
    );

    let mut spi = Spi::new(unsafe { pac::Peripherals::steal() }.spi3, Config::default());
    let mut id = [0u8; 4];
    let ok = spi.transfer(&mut id, &[0x9F, 0, 0, 0]).is_ok();
    let _ = writeln!(
        c.out,
        "    JEDEC via SpiBus::transfer: {:02x} {:02x} {:02x}\r",
        id[1], id[2], id[3]
    );
    c.check(
        "JEDEC ID reads 1f 86 01",
        ok && id[1..] == [0x1F, 0x86, 0x01],
    );

    let mut rd = [0u8; 8];
    let ok = spi.transfer(&mut rd, &[0x03, 0, 0, 0]).is_ok();
    let first = [rd[4], rd[5], rd[6], rd[7]];
    let _ = writeln!(
        c.out,
        "    flash[0..4] via SpiBus::transfer: {:02x} {:02x} {:02x} {:02x}\r",
        first[0], first[1], first[2], first[3]
    );
    if first == [0xFF; 4] {
        let _ = writeln!(
            c.out,
            "    (flash erased at 0: provision rustBoot for this check)\r"
        );
    } else {
        c.check(
            "flash[0..4] is rustBoot's auipc t0, 0",
            ok && first == [0x97, 0x02, 0x00, 0x00],
        );
    }

    let mut again = [0u8; 4];
    let ok = spi.transfer(&mut again, &[0x9F, 0, 0, 0]).is_ok();
    // Requires a valid ID, so two identical wrong reads cannot pass.
    c.check(
        "second JEDEC read is also 1f 86 01",
        ok && again[1..] == [0x1F, 0x86, 0x01],
    );

    let (passed, n) = (c.passed, c.n);
    let _ = writeln!(uart, "result: {passed}/{n} checks passed\r");
    loop {
        core::hint::spin_loop();
    }
}
