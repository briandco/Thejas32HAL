//! ADC interface for the THEJAS32 SoC.
//!
//! # DEPRECATED — this peripheral does not exist
//!
//! **This module addresses a register block that is not an ADC.** Do not use
//! it. Use the ADS1015 driver in the examples crate over [`crate::i2c`] with
//! [`thejas32_pac::I2c2`] instead; that is the board's real analog input, and
//! it is hardware-validated.
//!
//! `0x1000_1000` is an **I2C controller** — the vendor's `I2C_ADC`, instance 8
//! at `I2C_BASE + 0x100*8`. Proven on hardware: a device acknowledges at 0x48
//! on that address, its configuration register echoes written values exactly
//! (`0xC3C3`, then `0xD3C3`), and conversions read back in range. A register
//! block that was an ADC could not implement the I2C state machine.
//!
//! The vendor SDK describes the same address both ways — `config.h` calls it
//! `ADC_BASE_ADDRESS` and `adc.h` declares a three-register ADC block, while
//! `i2c.h` declares `I2C_ADC 8`. Only the I2C reading is backed by working
//! code: `adc_analogRead` is declared and **never implemented anywhere in the
//! SDK**. This module was written from the header that had no implementation
//! behind it, which is why `adc-selftest` found the block unresponsive.
//!
//! The board's A0–A3 are a TI ADS1015 12-bit I2C ADC on that bus. The THEJAS32
//! package exposes no analog pins of its own, so there is no on-chip ADC to
//! drive at all.
//!
//! Kept only so the mistake is documented and the history is legible; it is
//! scheduled for removal along with the `ADC` peripheral in the SVD generator,
//! `adc-selftest`, and `adc-diag`.
//!
//! ---
//!
//! Original documentation follows.
//!
//! A single 12-bit ADC at `0x1000_1000` with four channels. Selecting a
//! channel starts a conversion; the result is read once the status register
//! reports completion.
//!
//! # The PAC's channel field is incomplete — do not use it
//!
//! The SVD describes the address register as *"Bits 0:1 — ADC channel number
//! (0-3)"*, which is true but not sufficient. CDAC's `vega-sdk` writes
//! `0x14 + channel` (`0x14`/`0x15`/`0x16`/`0x17`): the channel really does
//! occupy the low two bits, but **bits 2 and 4 must also be set** and the SVD
//! does not model them.
//!
//! Writing the channel number through the PAC's typed `channel()` field would
//! therefore emit `0x00`–`0x03` and leave those bits clear. This driver writes
//! the raw encoding instead. What bits 2 and 4 select is undocumented — the
//! register is named "address", suggesting an internal mux or register
//! address rather than a simple channel index.
//!
//! This is worth noting as a class of hazard: a PAC field that is *present and
//! plausible* but incomplete is more dangerous than one that is missing
//! outright, because nothing about the generated API signals that more is
//! required.
//!
//! # No `embedded-hal` trait
//!
//! `embedded-hal` 1.0 ships no ADC trait (it provides only `delay`, `digital`,
//! `i2c`, `pwm` and `spi`), so this exposes an inherent API rather than
//! implementing a standard one.

use core::ops::Deref;
use thejas32_pac::{adc, Adc as PacAdc};

/// Base value of the address register; the channel occupies the low two bits.
///
/// Taken from `vega-sdk`'s `adc_analogRead`, which writes `0x14`–`0x17`. See
/// the module docs — the PAC does not model bits 2 and 4.
const ADDRESS_BASE: u32 = 0x14;

/// Status register bit: conversion complete.
const STATUS_DONE: u32 = 0x1;

/// Largest value a conversion can return (12-bit).
pub const MAX_VALUE: u16 = 4095;

/// Conversion resolution, in bits.
pub const RESOLUTION_BITS: u8 = 12;

/// Default bound on polling for conversion completion, in loop iterations.
pub const DEFAULT_TIMEOUT: u32 = 1_000_000;

/// An ADC input channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// Analog input 0.
    A0 = 0,
    /// Analog input 1.
    A1 = 1,
    /// Analog input 2.
    A2 = 2,
    /// Analog input 3.
    A3 = 3,
}

impl Channel {
    /// All channels, in order.
    pub const ALL: [Channel; 4] = [Channel::A0, Channel::A1, Channel::A2, Channel::A3];

    /// The value written to the address register to select this channel.
    #[inline]
    pub const fn address(self) -> u32 {
        ADDRESS_BASE | (self as u32)
    }
}

/// ADC error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The conversion did not complete within the timeout.
    Timeout,
}

/// ADC abstraction.
pub struct Adc {
    adc: PacAdc,
    timeout: u32,
}

impl Adc {
    /// Takes ownership of the ADC.
    pub fn new(adc: PacAdc) -> Self {
        Adc {
            adc,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Overrides the conversion-completion polling bound.
    pub fn with_timeout(mut self, timeout: u32) -> Self {
        self.timeout = timeout;
        self
    }

    /// Starts a conversion on `channel` and returns the 12-bit result.
    ///
    /// Selecting the channel is what starts the conversion; there is no
    /// separate trigger.
    pub fn read(&mut self, channel: Channel) -> Result<u16, Error> {
        // SAFETY: the address register needs the raw encoding, not the PAC's
        // partial `channel` field — see the module docs.
        unsafe { self.adc.address().write(|w| w.bits(channel.address())) };
        // The status poll below acts on this write; order the two.
        crate::fence();

        for _ in 0..self.timeout {
            if self.adc.status().read().bits() & STATUS_DONE != 0 {
                return Ok(self.adc.data().read().value().bits());
            }
        }
        Err(Error::Timeout)
    }

    /// Reads `channel` `n` times, returning `(min, max, mean)`.
    ///
    /// Useful for telling a driven input from a floating one: a pin tied to a
    /// rail reads stably, while a floating pin wanders. Any timeout aborts.
    pub fn sample(&mut self, channel: Channel, n: u16) -> Result<(u16, u16, u16), Error> {
        if n == 0 {
            return Ok((0, 0, 0));
        }
        let mut min = u16::MAX;
        let mut max = 0u16;
        let mut sum = 0u32;
        for _ in 0..n {
            let v = self.read(channel)?;
            min = min.min(v);
            max = max.max(v);
            sum += v as u32;
        }
        Ok((min, max, (sum / n as u32) as u16))
    }

    /// The current address-register value, for diagnostics.
    pub fn address_register(&self) -> u32 {
        self.adc.address().read().bits()
    }

    /// Releases the ADC peripheral.
    pub fn free(self) -> PacAdc {
        self.adc
    }
}

impl Deref for Adc {
    type Target = adc::RegisterBlock;
    fn deref(&self) -> &Self::Target {
        &self.adc
    }
}
