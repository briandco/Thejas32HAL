//! Driver for the TI ADS1015 12-bit I2C ADC.
//!
//! This is the ARIES V3.0's analog input. The THEJAS32 package exposes no
//! analog pins of its own, so the board's A0–A3 headers are wired to this chip
//! on the I2C bus at `0x1000_1000` (the vendor's `I2C_ADC`, instance 8).
//!
//! Deliberately generic over [`embedded_hal::i2c::I2c`] rather than over our
//! own driver: an off-chip device driver has no business depending on one SoC's
//! HAL, and going through the trait proves our `I2c` implementation is usable by
//! ordinary ecosystem drivers.
//!
//! Register model is simple — an address pointer plus 16-bit big-endian
//! registers. A conversion is: write the config register with the channel and a
//! start bit, wait, then read the conversion register.

use embedded_hal::i2c::I2c;

/// Default address, with the ADDR pin tied to GND. The scan found the board's
/// chip here.
pub const DEFAULT_ADDRESS: u8 = 0x48;

/// Register pointers.
const REG_CONVERT: u8 = 0x00;
const REG_CONFIG: u8 = 0x01;

/// Config bits, from the ADS1015 datasheet and CDAC's `ads1015.h`.
const OS_SINGLE: u16 = 0x8000; // start a single conversion
const MUX_SINGLE_0: u16 = 0x4000; // single-ended AIN0; +0x1000 per channel
const PGA_4_096V: u16 = 0x0200; // ±4.096 V full scale
const MODE_SINGLE: u16 = 0x0100; // single-shot, power down between
const DR_3300SPS: u16 = 0x00C0; // 3300 samples/s => ~303 us a conversion
const CQUE_NONE: u16 = 0x0003; // comparator off

/// Raw counts are 12-bit, left-aligned in a 16-bit register.
const RESULT_SHIFT: u8 = 4;

/// Largest raw count a 12-bit conversion can produce.
pub const MAX_COUNT: u16 = 0x0FFF;

/// Full-scale voltage in millivolts, matching [`PGA_4_096V`].
const FULL_SCALE_MV: u32 = 4096;

/// The config word this driver uses for a single-shot read of `channel`.
///
/// A free function rather than a method: it depends on nothing but the channel,
/// and a test needs it to check the device echoes back what was written instead
/// of trusting that the write landed.
pub fn config_for(channel: u8) -> u16 {
    OS_SINGLE
        | (MUX_SINGLE_0 + ((channel as u16) << 12))
        | PGA_4_096V
        | MODE_SINGLE
        | DR_3300SPS
        | CQUE_NONE
}

/// Converts a raw count to millivolts at the ±4.096 V range.
///
/// Single-ended readings are non-negative, so the signed range is not used.
pub fn counts_to_mv(counts: u16) -> u32 {
    (counts as u32 * FULL_SCALE_MV) / (MAX_COUNT as u32 + 1)
}

/// Errors talking to the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The channel index was not 0–3.
    InvalidChannel,
    /// The I2C transfer failed — no ACK, bus error, or timeout.
    Bus,
}

/// An ADS1015 on some I2C bus.
pub struct Ads1015<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C: I2c> Ads1015<I2C> {
    /// Wraps an I2C bus and device address.
    pub fn new(i2c: I2C, address: u8) -> Self {
        Self { i2c, address }
    }

    /// Writes a 16-bit register, big-endian. A three-byte write.
    pub fn write_register(&mut self, register: u8, value: u16) -> Result<(), Error> {
        self.i2c
            .write(self.address, &[register, (value >> 8) as u8, value as u8])
            .map_err(|_| Error::Bus)
    }

    /// Reads a 16-bit register, big-endian.
    ///
    /// Uses `write_read`, which sets the address pointer and then reads without
    /// releasing the bus. Our HAL issues a fresh START rather than a true
    /// repeated START; the ADS1015 accepts either.
    pub fn read_register(&mut self, register: u8) -> Result<u16, Error> {
        let mut buffer = [0u8; 2];
        self.i2c
            .write_read(self.address, &[register], &mut buffer)
            .map_err(|_| Error::Bus)?;
        Ok(u16::from_be_bytes(buffer))
    }

    /// Starts a conversion on `channel`. Does **not** wait for it.
    ///
    /// Split from reading the result so the caller supplies its own delay and
    /// this driver need not depend on a timer.
    pub fn start_conversion(&mut self, channel: u8) -> Result<(), Error> {
        if channel > 3 {
            return Err(Error::InvalidChannel);
        }
        self.write_register(REG_CONFIG, config_for(channel))
    }

    /// Reads the last conversion result, as a 12-bit count.
    pub fn read_result(&mut self) -> Result<u16, Error> {
        Ok(self.read_register(REG_CONVERT)? >> RESULT_SHIFT)
    }

    /// Reads back the configuration register.
    pub fn read_config(&mut self) -> Result<u16, Error> {
        self.read_register(REG_CONFIG)
    }

    /// Releases the I2C bus.
    pub fn free(self) -> I2C {
        self.i2c
    }
}
