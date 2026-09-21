//! SPI master interface for the THEJAS32 SoC.
//!
//! Four controllers: [`Spi0`]/[`Spi1`] at `0x1000_0600`/`0x1000_0700` and
//! [`Spi2`]/[`Spi3`] at `0x1020_0100`/`0x1020_0200`.
//!
//! Register and bit semantics follow CDAC's `vega-sdk` reference driver
//! (`bsp/drivers/spi/spi.c` and the control-word table in `spi.c`'s
//! `SPI_config` documentation).
//!
//! # Interrupts and the blocking API do not mix
//!
//! [`Spi::listen`] gives working interrupt-driven operation, but **only when the
//! application drives transfers at register level**. It must not be combined
//! with the [`SpiBus`] methods on the same controller.
//!
//! Those methods are built for polling and claim both resources an interrupt
//! handler needs: they read-modify-write the control register before every byte
//! (for `CSAAT`), and they poll the status register — which is read-to-clear —
//! in a tight loop. An interrupt handler that touches either one races them, and
//! the loser is silent. Measured across four arrangements of a self-test, the
//! same loop delivered 1 interrupt in 51,021 transfers, then 0, then stormed
//! into a 2,000-entry backstop, depending only on where the interrupt landed
//! inside a read-modify-write. The register-level equivalent is twelve-for-twelve
//! clean.
//!
//! # Why the combination cannot work
//!
//! Three measured facts leave exactly one workable design:
//!
//! 1. **The receive condition re-asserts until cleared.** A handler that merely
//!    records re-enters until something stops it.
//! 2. **It cannot be cleared by reading the status register.** That register is
//!    read-to-clear, so the read consumes the flag a polled transfer is waiting
//!    on, and the transfer spins forever.
//! 3. **The handler must therefore mask itself**, which is a control-register
//!    write — safe only while the application is not read-modify-writing
//!    control concurrently.
//!
//! [`SpiBus`] does precisely that, before every byte, so the two cannot
//! coexist. Drive transfers at register level when using interrupts. See
//! `spi-intr-selftest` for the pattern and `spi-intr-diag2` for the evidence.
//!
//! # Chip select
//!
//! Chip select is driven by the controller, not by a separate GPIO: the
//! `PCS` field selects which of four slaves is addressed, and `CSAAT`
//! ("chip select active after transfer") holds it asserted between bytes.
//! Per the vendor driver, CSAAT must be set while a multi-byte command or
//! data phase is in progress and cleared for its final byte.
//!
//! Each [`SpiBus`] method therefore holds CSAAT across its own buffer and
//! releases it on the last byte. That makes a single `write`/`transfer` call
//! one complete chip-select transaction. Chip select is *not* held across
//! separate calls — if a device needs one assertion spanning several calls,
//! drive [`Spi::set_csaat`] manually around them.

use core::ops::Deref;
use embedded_hal::spi::{ErrorKind, ErrorType, SpiBus};
use thejas32_pac::{spi0, Spi0, Spi1, Spi2, Spi3};

/// A SPI controller instance.
///
/// This trait is sealed to prevent external crates from implementing it.
pub trait SpiX: Deref<Target = spi0::RegisterBlock> + private::Sealed {}

macro_rules! impl_spix {
    ($($Spi:ty),*) => { $(impl SpiX for $Spi {})* };
}

impl_spix!(Spi0, Spi1, Spi2, Spi3);

/// SPI clock polarity/phase, encoded as the controller's 2-bit `MODE` field
/// (`CPOL` in bit 1, `CPHA` in bit 0 of the field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// CPOL = 0, CPHA = 0
    Mode0 = 0,
    /// CPOL = 0, CPHA = 1
    Mode1 = 1,
    /// CPOL = 1, CPHA = 0
    Mode2 = 2,
    /// CPOL = 1, CPHA = 1
    Mode3 = 3,
}

/// Bit transmission order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitOrder {
    /// Most significant bit first.
    MsbFirst = 0,
    /// Least significant bit first.
    LsbFirst = 1,
}

/// SPI clock divider, as the controller's baud-code: the peripheral clock is
/// divided by `4 << code`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockDivider {
    /// Peripheral clock / 4.
    Div4 = 0,
    /// Peripheral clock / 8.
    Div8 = 1,
    /// Peripheral clock / 16.
    Div16 = 2,
    /// Peripheral clock / 32.
    Div32 = 3,
    /// Peripheral clock / 64.
    Div64 = 4,
    /// Peripheral clock / 128.
    Div128 = 5,
    /// Peripheral clock / 256.
    Div256 = 6,
    /// Peripheral clock / 512.
    Div512 = 7,
    /// Peripheral clock / 1024.
    Div1024 = 8,
    /// Peripheral clock / 2048.
    Div2048 = 9,
}

impl ClockDivider {
    /// The resulting SPI clock in hertz, given the peripheral input clock.
    pub const fn frequency(self, clock_hz: u32) -> u32 {
        clock_hz / (4u32 << (self as u32))
    }
}

/// SPI controller configuration.
///
/// `#[non_exhaustive]`, so construct it from [`Config::default`] and the
/// `with_*` builders rather than a struct literal; new fields can then be added
/// without breaking callers.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Config {
    /// Clock polarity and phase.
    pub mode: Mode,
    /// Bit transmission order.
    pub bit_order: BitOrder,
    /// Which of the four hardware chip selects to address (0-3).
    pub chip_select: u8,
    /// SPI clock divider.
    pub divider: ClockDivider,
    /// Bits per transfer; the field encodes the count directly (8 = 8 bits).
    pub bits_per_transfer: u8,
}

impl Default for Config {
    /// Matches `vega-sdk`'s `SPI_init`: mode 3, MSB first, chip select 0,
    /// divide-by-16, 8 bits per transfer.
    fn default() -> Self {
        Config {
            mode: Mode::Mode3,
            bit_order: BitOrder::MsbFirst,
            chip_select: 0,
            divider: ClockDivider::Div16,
            bits_per_transfer: 8,
        }
    }
}

impl Config {
    /// Sets clock polarity and phase.
    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the bit transmission order.
    pub fn with_bit_order(mut self, bit_order: BitOrder) -> Self {
        self.bit_order = bit_order;
        self
    }

    /// Selects one of the four hardware chip selects (0-3).
    pub fn with_chip_select(mut self, chip_select: u8) -> Self {
        self.chip_select = chip_select;
        self
    }

    /// Sets the SPI clock divider.
    pub fn with_divider(mut self, divider: ClockDivider) -> Self {
        self.divider = divider;
        self
    }

    /// Sets the number of bits per transfer.
    pub fn with_bits_per_transfer(mut self, bits: u8) -> Self {
        self.bits_per_transfer = bits;
        self
    }
}

/// An SPI interrupt source.
///
/// The discriminants are the bit values of the control register's `rx_tx_intr`
/// field (bit 6 receive, bit 7 transmit), so they can be OR-ed directly.
/// Matches `vega-sdk`'s `TX_DIS_RX_EN_INTR` (1) and `RX_DIS_TX_EN_INTR` (2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// A byte has been received and is waiting in `RxData`.
    RxComplete = 0b01,
    /// The transmit register is empty and can take another byte.
    TxEmpty = 0b10,
}

/// Which interrupt conditions the controller is currently reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterruptStatus {
    /// A byte has arrived; clear it by reading `RxData`.
    pub rx_complete: bool,
    /// The transmit register is empty; clear it by writing `TxData`.
    pub tx_empty: bool,
}

/// Status bit 2: receive complete. From `vega-sdk`'s `spi_intr_status`.
const STATUS_RX_COMPLETE: u32 = 1 << 2;
/// Status bit 3: transmit register empty. Same source.
const STATUS_TX_EMPTY: u32 = 1 << 3;

/// SPI error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Receive data was not read before the next word arrived.
    Overrun,
}

impl embedded_hal::spi::Error for Error {
    fn kind(&self) -> ErrorKind {
        match self {
            Error::Overrun => ErrorKind::Overrun,
        }
    }
}

/// SPI master abstraction.
pub struct Spi<SPI> {
    spi: SPI,
}

impl<SPI: SpiX> Spi<SPI> {
    /// Configures a SPI controller as a master.
    pub fn new(spi: SPI, config: Config) -> Self {
        // SAFETY: writing raw field values to the control and baud registers;
        // all values are masked to their field widths by the PAC writers.
        unsafe {
            spi.baudrate()
                .write(|w| w.bits((config.divider as u32) << 4));
            crate::fence();
            spi.control().write(|w| {
                w.periph_cs().bits(config.chip_select & 0b11);
                w.periph().bit(false); // fixed peripheral
                w.data_order().bit(config.bit_order == BitOrder::LsbFirst);
                w.mode().bits(config.mode as u8);
                w.rx_tx_intr().bits(0); // polling only
                w.csaat().bit(false);
                w.dbits().bits(config.bits_per_transfer & 0xF)
            });
        }
        Spi { spi }
    }

    /// Sets `CSAAT` — whether chip select stays asserted after a transfer.
    ///
    /// The [`SpiBus`] methods manage this per call; use it directly only to
    /// hold one chip-select assertion across several calls.
    pub fn set_csaat(&self, active: bool) {
        self.spi.control().modify(|_, w| w.csaat().bit(active));
    }

    /// Enables an interrupt source.
    ///
    /// The controller must also be unmasked in the PLIC ([`crate::plic`]) and
    /// the core's external interrupts enabled before anything is delivered.
    ///
    /// The two enables share the control register's 2-bit `rx_tx_intr` field:
    /// bit 6 is receive, bit 7 is transmit. Taken from `vega-sdk`'s
    /// `SPI_set_intr`, which computes `(tx_intr << 7) | (rx_intr << 6)` and
    /// masks the register with `0xFF3F` before merging.
    ///
    /// # Never call this from an interrupt handler
    ///
    /// This is a read-modify-write on the control register, and [`SpiBus`]
    /// read-modify-writes the *same* register mid-transfer — `set_csaat` runs
    /// before every byte. A handler calling `listen`/`unlisten` while a
    /// transfer is in flight therefore loses one update or the other, silently.
    ///
    /// Measured: with a handler masking itself this way, one phase delivered a
    /// single interrupt across 51,021 transfers while another stormed into its
    /// backstop — from identical code. A variant whose `main` never touched the
    /// control register mid-transfer was a clean twelve-for-twelve.
    ///
    /// Keep exactly one writer. Arm before the transfer and disarm after, from
    /// the application, and leave the handler touching nothing but its own
    /// state.
    ///
    /// # Do not mix a receive interrupt with the blocking transfers
    ///
    /// [`Spi::transfer_byte`] and the [`SpiBus`] methods spin on
    /// `status.rx_complete` and then read `RxData` themselves. Reading `RxData`
    /// is also what clears a receive interrupt, so a handler that consumes the
    /// byte clears the flag the blocking transfer is still waiting on — and
    /// that transfer then spins forever. This deadlocks in practice, not just
    /// in theory.
    ///
    /// Pick one owner of the receive path:
    ///
    /// * **Handler owns it** — the handler reads `RxData`, and the application
    ///   never calls the blocking transfers on that controller.
    /// * **Application owns it** — the handler records the event and calls
    ///   [`Spi::unlisten`] to stop re-entry, leaving the byte for the blocking
    ///   transfer to collect. Re-arm before the next one.
    ///
    /// In the second arrangement the handler **must not read the status
    /// register at all** — not via [`Spi::take_interrupt_status`], not
    /// directly. That read clears the receive flag the blocking transfer is
    /// waiting on, and the transfer then spins forever. With a single event
    /// enabled the cause is already known, so there is nothing to read.
    pub fn listen(&self, event: Event) {
        self.spi.control().modify(|r, w| {
            // SAFETY: value is masked to the 2-bit field width.
            unsafe { w.rx_tx_intr().bits(r.rx_tx_intr().bits() | event as u8) }
        });
        crate::fence();
    }

    /// Disables an interrupt source, leaving the other one alone.
    pub fn unlisten(&self, event: Event) {
        self.spi.control().modify(|r, w| {
            // SAFETY: as `listen`.
            unsafe { w.rx_tx_intr().bits(r.rx_tx_intr().bits() & !(event as u8)) }
        });
        crate::fence();
    }

    /// Takes the pending interrupt conditions, **clearing them**.
    ///
    /// # This read is destructive
    ///
    /// The receive flags in the status register are cleared by the act of
    /// reading it. Measured: after a transfer, the first read returns `0x00C4`
    /// (receive-interrupt and receive-complete set) and the second returns
    /// `0x0080`, with the byte still sitting untouched in `RxData`
    /// (`spi-intr-diag`). One observation is all there is.
    ///
    /// Two consequences, both sharp:
    ///
    /// * **Only one reader may exist.** If a handler calls this while the
    ///   application is inside a blocking transfer, the transfer's own
    ///   `rx_complete` poll never sees the flag and spins forever. This is not
    ///   theoretical — it deadlocked during bring-up.
    /// * **The value must be used, not re-read.** Store what this returns;
    ///   calling it twice gives an empty result the second time.
    ///
    /// Named `take_` rather than `interrupt_status` so the destruction is
    /// visible at the call site.
    pub fn take_interrupt_status(&self) -> InterruptStatus {
        let status = self.spi.status().read();
        InterruptStatus {
            rx_complete: status.bits() & STATUS_RX_COMPLETE != 0,
            tx_empty: status.bits() & STATUS_TX_EMPTY != 0,
        }
    }

    /// Blocks until the controller is idle.
    pub fn wait_if_busy(&self) {
        while self.spi.status().read().busy().bit_is_set() {}
    }

    /// Exchanges one word: writes `byte` and returns the word clocked in.
    ///
    /// SPI is full duplex, so every write yields a read. Reading the receive
    /// register each time is also what keeps the overrun flag clear.
    pub fn transfer_byte(&mut self, byte: u8) -> Result<u8, Error> {
        self.wait_if_busy();
        while !self.spi.status().read().tx_hold_empty().bit_is_set() {}

        // SAFETY: writing a data word to the transmit register.
        unsafe { self.spi.txdata().write(|w| w.data().bits(byte as u16)) };
        // Ensure the data write reaches the controller before we poll for the
        // completion it causes. See `crate::fence`.
        crate::fence();

        while !self.spi.status().read().rx_complete().bit_is_set() {}
        let received = self.spi.rxdata().read().data().bits() as u8;

        if self.spi.status().read().overrun().bit_is_set() {
            return Err(Error::Overrun);
        }
        Ok(received)
    }

    /// Runs `f` for each index, holding CSAAT for every byte but the last so
    /// the whole buffer is one chip-select transaction.
    fn framed<F>(&mut self, len: usize, mut f: F) -> Result<(), Error>
    where
        F: FnMut(&mut Self, usize) -> Result<(), Error>,
    {
        for i in 0..len {
            self.set_csaat(i + 1 < len);
            f(self, i)?;
        }
        // Leave chip select released even if the buffer was empty.
        self.set_csaat(false);
        Ok(())
    }

    /// Releases the SPI peripheral.
    pub fn free(self) -> SPI {
        self.spi
    }
}

impl<SPI: SpiX> ErrorType for Spi<SPI> {
    type Error = Error;
}

impl<SPI: SpiX> SpiBus<u8> for Spi<SPI> {
    fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        self.framed(words.len(), |spi, i| {
            // Clock out a zero to generate the clock for an incoming word.
            words[i] = spi.transfer_byte(0)?;
            Ok(())
        })
    }

    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        self.framed(words.len(), |spi, i| {
            spi.transfer_byte(words[i])?;
            Ok(())
        })
    }

    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        // Per embedded-hal, the buffers may differ in length: surplus writes
        // discard their reads, and surplus reads clock out zeros.
        let len = read.len().max(write.len());
        self.framed(len, |spi, i| {
            let out = write.get(i).copied().unwrap_or(0);
            let got = spi.transfer_byte(out)?;
            if let Some(slot) = read.get_mut(i) {
                *slot = got;
            }
            Ok(())
        })
    }

    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        self.framed(words.len(), |spi, i| {
            words[i] = spi.transfer_byte(words[i])?;
            Ok(())
        })
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.wait_if_busy();
        Ok(())
    }
}

mod private {
    use thejas32_pac::{Spi0, Spi1, Spi2, Spi3};

    pub trait Sealed {}

    macro_rules! impl_sealed {
        ($($Spi:ty),*) => { $(impl Sealed for $Spi {})* };
    }

    impl_sealed!(Spi0, Spi1, Spi2, Spi3);
}
