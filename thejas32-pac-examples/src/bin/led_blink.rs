//! LED blink — toggles all 9 onboard LEDs using the thejas32-pac crate

#![no_std]
#![no_main]

// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use panic_halt as _;

// All 9 LED pins (bank 1, pins 0-8)
const LED_PINS: u16 = 0x1FF;

#[riscv_rt::entry]
fn main() -> ! {
    // The PAC's singleton, taken once. `steal()` below (where present) is a
    // deliberate read-only alias for register inspection, not a second owner.
    let peripherals = thejas32_pac::Peripherals::take().unwrap();
    let gpio1 = peripherals.gpio1;

    // Set pins 0-8 as output
    gpio1
        .dir()
        .modify(|r, w| unsafe { w.pin().bits(r.pin().bits() | LED_PINS) });

    loop {
        // All LEDs ON
        gpio1
            .dataout()
            .write(|w| unsafe { w.pins().bits(LED_PINS) });
        delay();

        // All LEDs OFF
        gpio1.dataout().write(|w| unsafe { w.pins().bits(0) });
        delay();
    }
}

#[inline(never)]
fn delay() {
    for _ in 0..0x80_0000u32 {
        unsafe { core::arch::asm!("nop") };
    }
}
