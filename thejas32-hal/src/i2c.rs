//! I2C master interface for the THEJAS32 SoC.
//!
//! Three controllers: [`I2c0`] at `0x1000_0800`, [`I2c1`] at `0x1000_0900`,
//! and [`I2c2`] at `0x1000_1000`. Registers are byte-wide and byte-spaced.
//!
//! [`I2c2`] is the vendor's `I2C_ADC` (instance 8, `I2C_BASE + 0x100*8`) and is
//! the bus carrying the board's onboard TI ADS1015 12-bit ADC at address 0x48 —
//! the ARIES V3.0's A0–A3 analog inputs. The SoC package exposes no analog pins
//! of its own. Note that `config.h` calls this same address `ADC_BASE_ADDRESS`
//! and `adc.h` declares an ADC register block there, but no implementation of
//! `adc_analogRead` exists anywhere in the vendor SDK, while the I2C reading is
//! exercised by its `industrial_robot_demo`. Unlike the other two, this
//! instance has no PLIC line, so it is polled only.
//!
//! The PAC models these registers without bit fields, so every status and
//! control bit here is named from CDAC's `vega-sdk` reference driver
//! (`bsp/drivers/i2c/i2c.c`) rather than from the SVD.
//!
//! # No repeated START
//!
//! This controller only accepts a START once the bus has been released, so
//! [`I2c::transaction`] issues a STOP after **every** operation rather than the
//! repeated STARTs `embedded-hal` specifies. Measured with an ADS1015 on the
//! bus: a write ending in STOP followed by a fresh START reads correctly, while
//! the same sequence without the intervening STOP fails every read
//! (`i2c-read-diag`). Devices that require a true repeated START to hold a
//! register pointer across the turnaround will not work; most, including the
//! ADS1015, accept either form.
//!
//! # Bounded waits
//!
//! Unlike the other drivers here, every wait is **bounded** and returns
//! [`Error::Timeout`] rather than blocking forever. A stuck I2C bus is an
//! ordinary failure — a missing pull-up, an absent device, or a slave holding
//! SCL low mid-transaction all produce a condition the vendor driver spins on
//! indefinitely. On a board with no I2C device attached that is the *default*
//! outcome, so an unbounded driver would simply hang.
//!
//! # Deviation from the vendor driver
//!
//! `vega-sdk`'s `i2c_data_read` advances its output pointer without ever
//! reading `RxFF`, so received bytes are discarded — the returned buffer is
//! left untouched. This driver reads the FIFO properly.
//!
//! # Limits
//!
//! A read is capped at 16 bytes by the receive FIFO, and the length must be
//! known when the START is issued (it is encoded into the control register).
//! Longer reads return [`Error::TooLong`].

use core::ops::Deref;
use embedded_hal::i2c::{ErrorKind, ErrorType, I2c as I2cTrait, NoAcknowledgeSource, Operation};
use thejas32_pac::{i2c0, I2c0, I2c1, I2c2};

// --- SR0 status bits (from vega-sdk's i2c.c) ---
const SR0_START_DONE: u8 = 0x01;
const SR0_STOP_DONE: u8 = 0x02;
const SR0_TX_FIFO_FULL: u8 = 0x04;
const SR0_TX_FIFO_EMPTY: u8 = 0x08;
const SR0_TRANSFER_COMPLETE: u8 = 0x10;
const SR0_RX_FIFO_EMPTY: u8 = 0x40;
const SR0_RX_COMPLETE: u8 = 0x80;

// --- SR1 status bits ---
const SR1_NACK: u8 = 0x01;

// --- CR control bits ---
const CR_START: u8 = 0x01;
const CR_STOP: u8 = 0x02;

/// Receive FIFO depth; the hardware cannot read more than this in one go.
pub const MAX_READ: usize = 16;

/// Default bound on status polling, in loop iterations.
pub const DEFAULT_TIMEOUT: u32 = 1_000_000;

/// Poll iterations spent waiting for a byte to be *concluded*, at 100 kHz.
///
/// `SR0.xfer_complete` asserting does **not** mean `SR1.NACK` is valid.
/// Measured on hardware: after `xfer_complete`, a NACK only becomes visible
/// once the controller finishes the byte — which on a NACK means completing
/// its automatic STOP. That takes ~1,500 poll iterations (~45 us, roughly
/// 4.5 SCL bit periods at 100 kHz). Reading `SR1` at `xfer_complete`, after a
/// fence, or after a 20 us delay all report "no NACK" every single time, so a
/// bus scan finds a phantom device at every address.
///
/// This is the bound, not the cost: on a NACK the wait exits as soon as the
/// STOP appears (~1,500), and only an ACK — where no STOP ever comes — pays
/// the full bound. 4,500 gives ~3x margin over the measured value.
pub const NACK_SETTLE_AT_100KHZ: u32 = 4_500;

/// An I2C controller instance.
///
/// This trait is sealed to prevent external crates from implementing it.
pub trait I2cX: Deref<Target = i2c0::RegisterBlock> + private::Sealed {}

impl I2cX for I2c0 {}
impl I2cX for I2c1 {}
impl I2cX for I2c2 {}

/// An I2C interrupt source.
///
/// Discriminants are the `IER` bit values, from `vega-sdk`'s `I2C_enable_intr`:
/// `(rx_intr << 2) | (tx_intr << 1) | 1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// A byte has been transmitted.
    TxComplete = 0b010,
    /// A byte has been received.
    RxComplete = 0b100,
}

/// `IER` bit 0, which the vendor driver sets whenever it enables either
/// interrupt. Its meaning is undocumented and no vendor code ever clears it
/// selectively, so this driver mirrors that behaviour rather than guessing.
const IER_ENABLE: u8 = 0b001;

/// `SR1` bit 1: a transmit interrupt is pending. From `I2C_intr_handler`.
///
/// Note that the vendor's receive branch there — `(status & 0x03) == 0x03` — is
/// unreachable, since the transmit test catches every case with bit 1 set. It
/// is not a usable reference for identifying a receive interrupt.
const SR1_TX_INTERRUPT: u8 = 0x02;

/// I2C error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The slave did not acknowledge its address or a data byte.
    NoAcknowledge,
    /// A status flag did not appear within the timeout — typically a stuck
    /// bus: no pull-ups, no device, or a slave holding SCL low.
    Timeout,
    /// Read longer than the receive FIFO ([`MAX_READ`]).
    TooLong,
}

impl embedded_hal::i2c::Error for Error {
    fn kind(&self) -> ErrorKind {
        match self {
            Error::NoAcknowledge => ErrorKind::NoAcknowledge(NoAcknowledgeSource::Unknown),
            Error::Timeout | Error::TooLong => ErrorKind::Other,
        }
    }
}

/// I2C master abstraction.
pub struct I2c<I2C> {
    i2c: I2C,
    timeout: u32,
    nack_settle: u32,
}

impl<I2C: I2cX> I2c<I2C> {
    /// Configures an I2C controller as a master at `i2c_hz`.
    ///
    /// `clock_hz` is the peripheral input clock — pass
    /// [`crate::PERIPHERAL_CLOCK_HZ`]. Note `vega-sdk` hardcodes 40 MHz here,
    /// which is wrong for this board and would halve the bus speed.
    pub fn new(i2c: I2C, clock_hz: u32, i2c_hz: u32) -> Self {
        // Scale the settle with the bus period: everything takes proportionally
        // longer at lower bus speeds.
        let nack_settle = NACK_SETTLE_AT_100KHZ * (100_000 / i2c_hz.max(1)).max(1);
        let this = I2c {
            i2c,
            timeout: DEFAULT_TIMEOUT,
            nack_settle,
        };
        this.configure(clock_hz, i2c_hz);
        this
    }

    /// Overrides the polling bound used by every wait.
    pub fn with_timeout(mut self, timeout: u32) -> Self {
        self.timeout = timeout;
        self
    }

    /// Overrides how long a byte is given to conclude before `SR1` is read.
    ///
    /// See [`NACK_SETTLE_AT_100KHZ`] — too small a value silently reports every
    /// address as acknowledging.
    pub fn with_nack_settle(mut self, iterations: u32) -> Self {
        self.nack_settle = iterations;
        self
    }

    /// Programs the clock-period registers. Matches `i2c_initialize`: the low
    /// period is `clock / (2 * i2c)` and the high period `clock / (4 * i2c)`,
    /// each split across a low and high byte.
    fn configure(&self, clock_hz: u32, i2c_hz: u32) {
        let ch = (clock_hz / (2 * i2c_hz)) as u16;
        let chh = (clock_hz / (4 * i2c_hz)) as u16;

        // SAFETY: these registers have no modelled fields; raw byte writes are
        // the only way to program them.
        unsafe {
            self.i2c.txclr().write(|w| w.bits(0xFF));
            crate::fence();
            self.i2c.chl().write(|w| w.bits((ch & 0xFF) as u8));
            self.i2c.chh().write(|w| w.bits((ch >> 8) as u8));
            self.i2c.chhl().write(|w| w.bits((chh & 0xFF) as u8));
            self.i2c.chhh().write(|w| w.bits((chh >> 8) as u8));
            crate::fence();
            // Polling only.
            self.i2c.ier().write(|w| w.bits(0));
            crate::fence();
        }
    }

    fn sr0(&self) -> u8 {
        self.i2c.sr0().read().bits()
    }

    /// Waits, bounded, for `cond` to hold of SR0.
    fn wait_sr0<F: Fn(u8) -> bool>(&self, cond: F) -> Result<(), Error> {
        for _ in 0..self.timeout {
            if cond(self.sr0()) {
                return Ok(());
            }
        }
        Err(Error::Timeout)
    }

    /// Issues a START. `read_len` is encoded into the control register for
    /// reads, so it must be known up front.
    fn start(&mut self, read_len: usize, read: bool) -> Result<(), Error> {
        if read && read_len > MAX_READ {
            return Err(Error::TooLong);
        }

        // SAFETY: raw byte writes to field-less registers.
        unsafe { self.i2c.txclr().write(|w| w.bits(0xFF)) };
        crate::fence();
        self.wait_sr0(|s| s & SR0_TX_FIFO_EMPTY != 0 && s & SR0_TRANSFER_COMPLETE != 0)?;

        let cr = if read {
            ((read_len as u8) << 2) | CR_START
        } else {
            CR_START
        };
        // SAFETY: as above.
        unsafe { self.i2c.cr().write(|w| w.bits(cr)) };
        crate::fence();

        self.wait_sr0(|s| s & SR0_START_DONE != 0)
    }

    /// Issues a STOP and waits for it to take effect.
    pub fn stop(&mut self) -> Result<(), Error> {
        // SAFETY: raw byte write to a field-less register.
        unsafe { self.i2c.cr().write(|w| w.bits(CR_STOP)) };
        crate::fence();
        self.wait_sr0(|s| s & SR0_STOP_DONE != 0)
    }

    /// Pushes bytes through the transmit FIFO, then checks for a NACK.
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        for &byte in bytes {
            self.wait_sr0(|s| s & SR0_TX_FIFO_FULL == 0)?;
            // SAFETY: raw byte write to a field-less register.
            unsafe { self.i2c.txff().write(|w| w.bits(byte)) };
            crate::fence();
        }
        self.wait_sr0(|s| s & SR0_TRANSFER_COMPLETE != 0)?;

        // `xfer_complete` is not the point at which NACK is valid. Give the
        // controller time to conclude the byte: on a NACK it auto-issues STOP
        // and we exit as soon as that lands; on an ACK no STOP arrives and we
        // fall through after the bound. See `NACK_SETTLE_AT_100KHZ`.
        for _ in 0..self.nack_settle {
            if self.sr0() & SR0_STOP_DONE != 0 {
                break;
            }
        }

        if self.i2c.sr1().read().bits() & SR1_NACK != 0 {
            // The controller releases the bus itself on NACK; wait for it.
            let _ = self.wait_sr0(|s| s & SR0_STOP_DONE != 0);
            return Err(Error::NoAcknowledge);
        }
        Ok(())
    }

    /// Drains the receive FIFO into `buf`.
    fn read_bytes(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        if buf.len() > MAX_READ {
            return Err(Error::TooLong);
        }
        self.wait_sr0(|s| s & SR0_RX_COMPLETE != 0)?;
        for slot in buf.iter_mut() {
            self.wait_sr0(|s| s & SR0_RX_FIFO_EMPTY == 0)?;
            // Unlike vega-sdk, actually read the FIFO.
            *slot = self.i2c.rxff().read().bits();
        }
        Ok(())
    }

    /// Writes `bytes` to `address`, framed by START and STOP.
    pub fn write_to(&mut self, address: u8, bytes: &[u8]) -> Result<(), Error> {
        self.start(0, false)?;
        let result = self
            .write_bytes(&[address << 1])
            .and_then(|()| self.write_bytes(bytes));
        // Always release the bus, even on NACK.
        let _ = self.stop();
        result
    }

    /// Reads `buf.len()` bytes from `address`, framed by START and STOP.
    pub fn read_from(&mut self, address: u8, buf: &mut [u8]) -> Result<(), Error> {
        if buf.len() > MAX_READ {
            return Err(Error::TooLong);
        }
        self.start(buf.len(), true)?;
        let result = self
            .write_bytes(&[(address << 1) | 1])
            .and_then(|()| self.read_bytes(buf));
        let _ = self.stop();
        result
    }

    /// Probes `address`, returning `true` if a device acknowledged.
    ///
    /// Issues an address-only write; a NACK means nothing is there, which is
    /// an expected result rather than an error.
    pub fn probe(&mut self, address: u8) -> Result<bool, Error> {
        match self.write_to(address, &[]) {
            Ok(()) => Ok(true),
            Err(Error::NoAcknowledge) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Enables an interrupt source.
    ///
    /// The controller must also be unmasked in the PLIC ([`crate::plic`]) and
    /// the core's external interrupts enabled. Note that [`I2c2`] has no PLIC
    /// line at all — only controllers 0 and 1 can interrupt.
    ///
    /// # Call this only between transfers
    ///
    /// This read-modify-writes `IER`, and an interrupt handler will typically
    /// write the same register to mask itself. Doing both concurrently loses one
    /// update. Unlike SPI, nothing in the *data* path touches `IER` — `cr`,
    /// `txff` and `txclr` are all written whole — so keeping `listen` and
    /// `unlisten` outside a transfer is sufficient to keep a single writer.
    pub fn listen(&self, event: Event) {
        let current = self.i2c.ier().read().bits();
        // SAFETY: raw byte write to a field-less register.
        unsafe {
            self.i2c
                .ier()
                .write(|w| w.bits(current | IER_ENABLE | event as u8))
        };
        crate::fence();
    }

    /// Disables an interrupt source, leaving any other one enabled.
    ///
    /// When no source remains, `IER` is cleared entirely rather than left with
    /// the vendor's always-set bit 0 standing on its own.
    pub fn unlisten(&self, event: Event) {
        let current = self.i2c.ier().read().bits();
        let remaining = current & !(event as u8);
        let next = if remaining & 0b110 == 0 { 0 } else { remaining };
        // SAFETY: as `listen`.
        unsafe { self.i2c.ier().write(|w| w.bits(next)) };
        crate::fence();
    }

    /// Whether a transmit interrupt is pending, from `SR1`.
    ///
    /// Reading is non-destructive: the polled driver reads `SR0` and `SR1`
    /// continuously and is hardware-validated, so these registers are not
    /// read-to-clear — unlike the SPI status register, which is. The self-test
    /// checks this directly rather than resting on that inference.
    pub fn tx_interrupt_pending(&self) -> bool {
        self.i2c.sr1().read().bits() & SR1_TX_INTERRUPT != 0
    }

    /// Releases the I2C peripheral.
    pub fn free(self) -> I2C {
        self.i2c
    }
}

impl<I2C: I2cX> ErrorType for I2c<I2C> {
    type Error = Error;
}

impl<I2C: I2cX> I2cTrait for I2c<I2C> {
    /// # Deviation
    ///
    /// Each operation is framed by its own START, with a single STOP at the
    /// end. True repeated-start behaviour between operations has not been
    /// verified — that needs a real slave device on the bus.
    fn transaction(&mut self, address: u8, operations: &mut [Operation<'_>]) -> Result<(), Error> {
        let mut result = Ok(());
        for op in operations.iter_mut() {
            result = match op {
                Operation::Write(bytes) => self
                    .start(0, false)
                    .and_then(|()| self.write_bytes(&[address << 1]))
                    .and_then(|()| self.write_bytes(bytes)),
                Operation::Read(buf) => {
                    if buf.len() > MAX_READ {
                        Err(Error::TooLong)
                    } else {
                        self.start(buf.len(), true)
                            .and_then(|()| self.write_bytes(&[(address << 1) | 1]))
                            .and_then(|()| self.read_bytes(buf))
                    }
                }
            };
            // Release the bus after every operation, not just the last.
            //
            // This controller does not support a repeated START: a START is
            // only accepted once the bus has been released. Measured — with an
            // ADS1015 on the bus, `write_to` (which always issues STOP)
            // followed by a fresh START reads correctly, while this loop
            // without the intervening STOP fails every read. See the
            // `i2c-read-diag` example.
            //
            // `embedded-hal` specifies repeated STARTs between the operations
            // of a transaction, so this is a documented deviation rather than
            // conformance. Devices that distinguish the two — those needing a
            // true repeated START to keep a register pointer across the turn —
            // will not work here. Most, including the ADS1015, accept either.
            let _ = self.stop();
            if result.is_err() {
                break;
            }
        }
        result
    }
}

mod private {
    use thejas32_pac::{I2c0, I2c1, I2c2};

    pub trait Sealed {}

    impl Sealed for I2c0 {}
    impl Sealed for I2c1 {}
    impl Sealed for I2c2 {}
}
