//! GPIO interface for the THEJAS32 SoC.
//!
//! Two banks of 16 pins each: [`Gpio0`] covers pins 0–15, [`Gpio1`] pins
//! 16–31 (addressed as 0–15 within the bank).
//!
//! # Bit-banded data access
//!
//! The data registers are *bit-banded*: the address encodes a pin mask, and
//! an access only affects the pins selected by that mask. For pin `N` the
//! address is `BASE + ((1 << N) << 2)`, and the value written is the level
//! already shifted into bit `N`. Reading that address yields nonzero when the
//! pin is high.
//!
//! This is why per-pin writes here need no read-modify-write and cannot
//! disturb neighbouring pins — the hardware masks the access. Register
//! semantics are taken from CDAC's `vega-sdk` reference driver
//! (`bsp/drivers/gpio/gpio.c`).
//!
//! Note that `thejas32-pac` cannot express these apertures: it models only
//! mask 0 (`data`) and mask `0xFFFF` (`dataout`), so the per-pin addresses
//! are computed here directly from the peripheral base.
//!
//! The direction register (`BASE + 0x40000`) is *not* bit-banded and does
//! need a read-modify-write — see [`Gpio::into_output`].
//!
//! # Interrupts are level-sensitive, active high
//!
//! Bank 0 pins 0–12 each own a PLIC line (see [`Pin::interrupt`]), and that is
//! the **only** control: this block has no interrupt enable, no edge select and
//! no status register.
//!
//! The trigger is **level-sensitive on a high input**, measured rather than
//! documented: with the pin held at a static level — so no edges exist at all —
//! a high input produced interrupts continuously while a low input produced
//! none (`gpio-intr-selftest`). Two consequences follow, and both bite.
//!
//! **A handler must mask its own source.** There is no status bit to clear, so
//! while the pin stays high the interrupt re-asserts the instant the handler
//! returns. A handler that only counts will livelock the core. Disable the
//! pin's line in the PLIC from inside the handler, and re-enable it once the
//! condition has passed:
//!
//! ```no_run
//! # use thejas32_pac::interrupt::Interrupt;
//! # fn handler() {
//! let plic = unsafe { thejas32_pac::Plic::steal() };
//! let enabled = plic.intr_en().read().bits();
//! // SAFETY: single-hart; nothing else writes this register concurrently.
//! unsafe { plic.intr_en().write(|w| w.bits(enabled & !(1 << (Interrupt::GPIO0 as u8)))) };
//! # }
//! ```
//!
//! **Edges cannot be counted.** A pulse train does not yield one interrupt per
//! pulse; it yields as many as the core can take while the level is high. Use
//! these for "something is asserted" conditions — a button held down, a sensor
//! output — not for measuring frequency or counting events. A timer capture is
//! the right tool for that.

// These `let _ = ...::PIN_IN_RANGE` bindings look inert but are load-bearing:
// referencing the associated const is what forces its `assert!` to be
// evaluated, rejecting out-of-range pin numbers at compile time. Removing
// them silently disables that check.
#![allow(clippy::let_unit_value)]

use core::{convert::Infallible, marker::PhantomData};
use embedded_hal::digital::{ErrorType, InputPin, OutputPin, StatefulOutputPin};
use thejas32_pac::interrupt::Interrupt;
use thejas32_pac::{Gpio0, Gpio1};

/// Offset of the direction register from a bank's base address.
const DIR_OFFSET: usize = 0x4_0000;

/// Offset of the bulk data aperture (mask `0xFFFF`, i.e. all 16 pins).
const DATAOUT_OFFSET: usize = 0x3_FFFC;

/// A GPIO bank (`Gpio0` or `Gpio1`).
///
/// This trait is sealed to prevent external crates from implementing it.
pub trait GpioBank: private::Sealed {
    /// Base address of this bank's register block.
    ///
    /// Deliberately an associated function rather than a method: taking no
    /// `self` lets [`Pin`] recover the address from its type parameter alone,
    /// so pins carry no runtime data and stay zero-sized.
    fn base_addr() -> usize;

    /// Reads all 16 pins of this bank, without needing the bank itself.
    ///
    /// [`Gpio::split`] consumes the bank and its `Parts` gives pins away by
    /// move, which partially moves that struct — so a `&self` accessor on it
    /// would stop compiling after the first pin is taken. Taking no `self`
    /// sidesteps that.
    fn read_all_of() -> u16 {
        let addr = (Self::base_addr() + DATAOUT_OFFSET) as *const u16;
        // SAFETY: valid MMIO derived from the PAC's base address.
        unsafe { addr.read_volatile() }
    }

    /// Reads this bank's direction register: a set bit means output.
    fn direction_of() -> u16 {
        let addr = (Self::base_addr() + DIR_OFFSET) as *const u16;
        // SAFETY: as `read_all_of`.
        unsafe { addr.read_volatile() }
    }
}

impl GpioBank for Gpio0 {
    fn base_addr() -> usize {
        Gpio0::PTR as usize
    }
}

impl GpioBank for Gpio1 {
    fn base_addr() -> usize {
        Gpio1::PTR as usize
    }
}

/// A pin that has not been configured yet.
///
/// [`Gpio::split`] hands out pins in this state. It implements none of the
/// `embedded-hal` traits, so a pin must be explicitly turned into an input or an
/// output before it can be used.
pub struct Unknown;

/// Pin configured as an output.
pub struct Output;

/// Pin configured as an input.
pub struct Input;

/// A GPIO bank, used to configure individual [`Pin`]s.
pub struct Gpio<GPIO> {
    bank: GPIO,
}

impl<GPIO: GpioBank> Gpio<GPIO> {
    /// Takes ownership of a GPIO bank.
    pub fn new(bank: GPIO) -> Self {
        Gpio { bank }
    }

    /// Configures pin `N` as an output, leaving its current level unchanged.
    ///
    /// The direction register is not bit-banded, so this performs a
    /// read-modify-write. Configuring pins from an interrupt handler while
    /// another configuration is in progress on the same bank can therefore
    /// lose an update; do the configuration up front, or guard it with a
    /// critical section.
    pub fn into_output<const N: u8>(&self) -> Pin<GPIO, N, Output> {
        let _ = Pin::<GPIO, N, Output>::PIN_IN_RANGE;
        self.set_dir::<N>(true);
        Pin {
            _bank: PhantomData,
            _mode: PhantomData,
        }
    }

    /// Configures pin `N` as an input.
    ///
    /// Carries the same read-modify-write caveat as [`Gpio::into_output`].
    pub fn into_input<const N: u8>(&self) -> Pin<GPIO, N, Input> {
        let _ = Pin::<GPIO, N, Input>::PIN_IN_RANGE;
        self.set_dir::<N>(false);
        Pin {
            _bank: PhantomData,
            _mode: PhantomData,
        }
    }

    /// Reads all 16 pins of this bank at once, via the bulk (mask `0xFFFF`)
    /// aperture.
    ///
    /// For pins configured as outputs this reflects the output latch, so it
    /// confirms what the driver *wrote* rather than the physical pin voltage.
    ///
    /// Also available as [`GpioBank::read_all_of`] without holding the bank,
    /// which is what a caller that has already split it into pins needs.
    pub fn read_all(&self) -> u16 {
        let addr = (GPIO::base_addr() + DATAOUT_OFFSET) as *const u16;
        // SAFETY: valid MMIO address derived from the PAC's base address;
        // volatile access is required for MMIO.
        unsafe { addr.read_volatile() }
    }

    /// Reads this bank's direction register: a set bit means output.
    pub fn direction(&self) -> u16 {
        let addr = (GPIO::base_addr() + DIR_OFFSET) as *const u16;
        // SAFETY: as `read_all`.
        unsafe { addr.read_volatile() }
    }

    /// Releases the GPIO bank.
    ///
    /// Pins handed out by this bank are zero-sized and borrow nothing, so they
    /// remain usable afterwards; drop them first if that matters.
    pub fn free(self) -> GPIO {
        self.bank
    }

    /// Sets pin `N`'s direction bit: `true` = output, `false` = input.
    fn set_dir<const N: u8>(&self, output: bool) {
        let dir = (GPIO::base_addr() + DIR_OFFSET) as *mut u16;
        let mask = 1u16 << N;
        // SAFETY: `dir` is this bank's direction register, a valid MMIO
        // address derived from the PAC's base address. Volatile access is
        // required for MMIO.
        unsafe {
            let current = dir.read_volatile();
            let next = if output {
                current | mask
            } else {
                current & !mask
            };
            dir.write_volatile(next);
        }
    }
}

/// The sixteen pins of one bank, each handed out exactly once.
///
/// Produced by [`Gpio::split`]. Fields are moved out individually, which is what
/// makes double-ownership impossible: a pin taken from here cannot be taken
/// again.
pub struct Parts<GPIO> {
    /// Pin 0 of this bank.
    pub pin0: Pin<GPIO, 0, Unknown>,
    /// Pin 1 of this bank.
    pub pin1: Pin<GPIO, 1, Unknown>,
    /// Pin 2 of this bank.
    pub pin2: Pin<GPIO, 2, Unknown>,
    /// Pin 3 of this bank.
    pub pin3: Pin<GPIO, 3, Unknown>,
    /// Pin 4 of this bank.
    pub pin4: Pin<GPIO, 4, Unknown>,
    /// Pin 5 of this bank.
    pub pin5: Pin<GPIO, 5, Unknown>,
    /// Pin 6 of this bank.
    pub pin6: Pin<GPIO, 6, Unknown>,
    /// Pin 7 of this bank.
    pub pin7: Pin<GPIO, 7, Unknown>,
    /// Pin 8 of this bank.
    pub pin8: Pin<GPIO, 8, Unknown>,
    /// Pin 9 of this bank.
    pub pin9: Pin<GPIO, 9, Unknown>,
    /// Pin 10 of this bank.
    pub pin10: Pin<GPIO, 10, Unknown>,
    /// Pin 11 of this bank.
    pub pin11: Pin<GPIO, 11, Unknown>,
    /// Pin 12 of this bank.
    pub pin12: Pin<GPIO, 12, Unknown>,
    /// Pin 13 of this bank.
    pub pin13: Pin<GPIO, 13, Unknown>,
    /// Pin 14 of this bank.
    pub pin14: Pin<GPIO, 14, Unknown>,
    /// Pin 15 of this bank.
    pub pin15: Pin<GPIO, 15, Unknown>,
}

impl<GPIO: GpioBank> Gpio<GPIO> {
    /// Consumes the bank and returns its pins, each exactly once.
    ///
    /// Prefer this to [`Gpio::into_output`] and [`Gpio::into_input`]. Those take
    /// `&self`, so calling one twice with the same `N` yields two values that
    /// both believe they own the pin — nothing prevents two drivers from driving
    /// it in opposite directions. `split` gives each pin away once and cannot be
    /// called twice, since it consumes the bank.
    ///
    /// Bank-wide reads remain available as [`GpioBank::read_all_of`] and
    /// [`GpioBank::direction_of`], which need no `self`.
    ///
    /// ```no_run
    /// # use thejas32_hal::gpio::Gpio;
    /// # let peripherals = unsafe { thejas32_pac::Peripherals::steal() };
    /// let pins = Gpio::new(peripherals.gpio1).split();
    /// let mut led = pins.pin0.into_output();
    /// let button = pins.pin1.into_input();
    /// ```
    pub fn split(self) -> Parts<GPIO> {
        Parts {
            pin0: Pin::new(),
            pin1: Pin::new(),
            pin2: Pin::new(),
            pin3: Pin::new(),
            pin4: Pin::new(),
            pin5: Pin::new(),
            pin6: Pin::new(),
            pin7: Pin::new(),
            pin8: Pin::new(),
            pin9: Pin::new(),
            pin10: Pin::new(),
            pin11: Pin::new(),
            pin12: Pin::new(),
            pin13: Pin::new(),
            pin14: Pin::new(),
            pin15: Pin::new(),
        }
    }
}

/// A single GPIO pin, parameterised by bank, pin number, and direction.
///
/// The direction is tracked in the type, so reading an output or driving an
/// input is a compile-time error rather than a silent no-op.
pub struct Pin<GPIO, const N: u8, MODE> {
    _bank: PhantomData<GPIO>,
    _mode: PhantomData<MODE>,
}

impl<GPIO: GpioBank, const N: u8, MODE> Pin<GPIO, N, MODE> {
    /// Forces a compile-time error if `N` is outside a bank's 16 pins.
    const PIN_IN_RANGE: () = assert!(N < 16, "GPIO pin number must be 0..=15");

    /// Creates the zero-sized handle. Private: pins come from [`Gpio::split`].
    const fn new() -> Self {
        Pin {
            _bank: PhantomData,
            _mode: PhantomData,
        }
    }

    /// Configures this pin as an output, leaving its current level unchanged.
    ///
    /// Consumes the pin, so the old mode's trait impls are no longer reachable.
    pub fn into_output(self) -> Pin<GPIO, N, Output> {
        let _ = Self::PIN_IN_RANGE;
        Self::set_dir(true);
        Pin::new()
    }

    /// Configures this pin as an input.
    pub fn into_input(self) -> Pin<GPIO, N, Input> {
        let _ = Self::PIN_IN_RANGE;
        Self::set_dir(false);
        Pin::new()
    }

    /// Sets this pin's direction bit. The direction register is not bit-banded,
    /// so this is a read-modify-write and can lose an update if raced.
    fn set_dir(output: bool) {
        let dir = (GPIO::base_addr() + DIR_OFFSET) as *mut u16;
        let mask = 1u16 << N;
        // SAFETY: valid MMIO derived from the PAC's base address.
        unsafe {
            let current = dir.read_volatile();
            dir.write_volatile(if output {
                current | mask
            } else {
                current & !mask
            });
        }
    }

    /// Bit-banded address for this pin: only pin `N` responds to accesses here.
    #[inline]
    fn data_addr(&self) -> *mut u16 {
        (GPIO::base_addr() + ((1usize << N) << 2)) as *mut u16
    }

    /// Reads this pin's bit-banded data register.
    #[inline]
    fn read_level(&self) -> bool {
        // SAFETY: the bit-banded address is derived from the PAC's base
        // address and is a valid MMIO location for pin N. Volatile access is
        // required for MMIO.
        unsafe { self.data_addr().read_volatile() != 0 }
    }

    /// Writes this pin's bit-banded data register. Other pins are unaffected
    /// because the hardware masks the access by address.
    #[inline]
    fn write_level(&self, high: bool) {
        let value = if high { 1u16 << N } else { 0 };
        // SAFETY: as `read_level`, and the address mask confines this write
        // to pin N alone.
        unsafe { self.data_addr().write_volatile(value) }
    }
}

impl<GPIO: GpioBank, const N: u8, MODE> ErrorType for Pin<GPIO, N, MODE> {
    type Error = Infallible;
}

impl<GPIO: GpioBank, const N: u8> OutputPin for Pin<GPIO, N, Output> {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.write_level(false);
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.write_level(true);
        Ok(())
    }
}

impl<GPIO: GpioBank, const N: u8> StatefulOutputPin for Pin<GPIO, N, Output> {
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        Ok(self.read_level())
    }

    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!self.read_level())
    }
}

impl<GPIO: GpioBank, const N: u8> InputPin for Pin<GPIO, N, Input> {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok(self.read_level())
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!self.read_level())
    }
}

impl<const N: u8> Pin<Gpio0, N, Input> {
    /// The PLIC source for this pin, or `None` if it has none.
    ///
    /// Bank 0 pins 0–12 each own a dedicated interrupt, `GPIO0`–`GPIO12`
    /// (PLIC lines 10–22). Pins 13–15 and the whole of [`Gpio1`] have no line,
    /// so this returns `None` for them; a `None` here means the pin genuinely
    /// cannot interrupt, not that support is missing.
    ///
    /// Interrupts here are **level-sensitive, active high**, and there is
    /// nothing to acknowledge — see the module documentation before writing a
    /// handler, because one that does not mask its own line will livelock.
    ///
    /// # There is no GPIO-side interrupt enable
    ///
    /// Unlike [`crate::serial`] and [`crate::timer`], which gate interrupts at
    /// the peripheral before the PLIC ever sees them, this block has **no**
    /// interrupt registers at all — no enable, no edge select, no status. The
    /// PLIC is the only gate. That is why there is no `listen`/`unlisten` here:
    /// inventing one that secretly reached into the PLIC would suggest a
    /// peripheral-side mask that does not exist.
    ///
    /// Confirmed against `vega-sdk`, whose two GPIO interrupt examples
    /// (`external_interrupt_pgm`, `ldr_sensor_intr_pgm`) configure the pin as
    /// an input and then do nothing but enable the PLIC line, and whose
    /// `gpio.h` declares no register block.
    ///
    /// ```no_run
    /// # use thejas32_hal::{gpio::Gpio, plic::Plic};
    /// # let gpio = Gpio::new(unsafe { thejas32_pac::Gpio0::steal() });
    /// # let plic = Plic::new(unsafe { thejas32_pac::Plic::steal() });
    /// let button = gpio.into_input::<0>();
    /// if let Some(irq) = button.interrupt() {
    ///     plic.enable(irq);
    /// }
    /// ```
    pub const fn interrupt(&self) -> Option<Interrupt> {
        match N {
            0 => Some(Interrupt::GPIO0),
            1 => Some(Interrupt::GPIO1),
            2 => Some(Interrupt::GPIO2),
            3 => Some(Interrupt::GPIO3),
            4 => Some(Interrupt::GPIO4),
            5 => Some(Interrupt::GPIO5),
            6 => Some(Interrupt::GPIO6),
            7 => Some(Interrupt::GPIO7),
            8 => Some(Interrupt::GPIO8),
            9 => Some(Interrupt::GPIO9),
            10 => Some(Interrupt::GPIO10),
            11 => Some(Interrupt::GPIO11),
            12 => Some(Interrupt::GPIO12),
            _ => None,
        }
    }
}

/// `Pin` must stay zero-sized: it exists only to name a bank, a pin number and
/// a direction, all of which live in the type. Storing anything here would put
/// a word behind every pin in a driver.
const _: () = {
    assert!(core::mem::size_of::<Pin<Gpio0, 0, Output>>() == 0);
    assert!(core::mem::size_of::<Pin<Gpio1, 15, Input>>() == 0);
    assert!(core::mem::size_of::<Pin<Gpio0, 0, Unknown>>() == 0);
    // A whole bank's worth of pins must also cost nothing, or `split` would
    // trade the safety it buys for sixteen words of stack.
    assert!(core::mem::size_of::<Parts<Gpio0>>() == 0);
};

mod private {
    use thejas32_pac::{Gpio0, Gpio1};

    pub trait Sealed {}

    impl Sealed for Gpio0 {}
    impl Sealed for Gpio1 {}
}
