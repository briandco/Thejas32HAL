//! Serial (UART) interface for the THEJAS32 SoC.
//!
//! Supports UART0, UART1, UART2. The register block is a classic
//! 16550-compatible UART (divisor latch multiplexed onto the data/interrupt
//! registers via `LCR.DLAB`) — the init sequence and status-bit meanings
//! here are cross-checked against CDAC's `vega-sdk` reference driver
//! (`bsp/drivers/uart/uart.c`).

use core::{hint::spin_loop, ops::Deref};
use embedded_hal_nb::{nb, serial};
use thejas32_pac::{uart0, Uart0, Uart1, Uart2};

/// UartX extends a UART peripheral type with the ability to steal a handle
/// to it.
///
/// This trait is sealed to prevent external crates from implementing it.
pub trait UartX: Deref<Target = uart0::RegisterBlock> + private::Sealed {
    /// Steals the UART peripheral.
    ///
    /// # Safety
    ///
    /// Using this function may break the guarantees of the singleton pattern.
    unsafe fn steal() -> Self;
}

macro_rules! impl_uartx {
    ($($Uart:ty),*) => {
        $(
            impl UartX for $Uart {
                unsafe fn steal() -> Self {
                    <$Uart>::steal()
                }
            }
        )*
    };
}

impl_uartx!(Uart0, Uart1, Uart2);

/// A UART line error.
///
/// A dedicated type rather than returning [`serial::ErrorKind`] directly, so
/// the crate owns its error surface and can grow variants without changing the
/// trait contract. [`serial::Error::kind`] maps each one honestly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Parity bit did not match the received data.
    Parity,
    /// A byte arrived before the previous one was read.
    Overrun,
    /// Stop bit was not where the frame format required.
    FrameFormat,
}

impl serial::Error for Error {
    fn kind(&self) -> serial::ErrorKind {
        match self {
            Error::Parity => serial::ErrorKind::Parity,
            Error::Overrun => serial::ErrorKind::Overrun,
            Error::FrameFormat => serial::ErrorKind::FrameFormat,
        }
    }
}

/// UART abstraction providing blocking/non-blocking serial I/O.
pub struct Serial<UART> {
    uart: UART,
}

impl<UART: UartX> Serial<UART> {
    /// Adopts a UART that has already been configured by the bootloader,
    /// leaving the baud divisor and line settings untouched.
    ///
    /// On THEJAS32 the ROM bootloader brings UART0 up at 115200 8N1 before
    /// handing control to the loaded program, so adopting that configuration
    /// is both correct and avoids depending on the UART input-clock frequency
    /// (which is *not* reliably documented — see [`Serial::new`]). This is the
    /// same approach CDAC's own `TrustedV_RTOS` BSP takes for console output.
    ///
    /// Interrupts are disabled so the peripheral is safe to poll; nothing else
    /// is modified.
    pub fn new_preconfigured(uart: UART) -> Self {
        uart.ier()
            .write(|w| w.erbfi().bit(false).etbei().bit(false));
        Serial { uart }
    }

    /// Configures a UART peripheral for 8 data bits, 1 stop bit, no parity
    /// ("8N1"), with the FIFO and all interrupts disabled.
    ///
    /// `clock_hz` is the UART's input clock frequency and `baud_rate` the
    /// desired line rate; the divisor is `clock_hz / (baud_rate * 16)`, per
    /// the 16550 standard (matches `vega-sdk`'s `uart_configure`).
    ///
    /// # Choosing `clock_hz`
    ///
    /// Pass [`crate::PERIPHERAL_CLOCK_HZ`] unless you have measured something
    /// different. Getting this wrong produces silent garbage on the wire
    /// rather than an error, so do not take the value from `vega-sdk`'s
    /// examples — their `40_000_000` does not hold for this board.
    ///
    /// If you only need the bootloader's 115200, prefer
    /// [`Serial::new_preconfigured`], which sidesteps the divisor entirely.
    pub fn new(uart: UART, baud_rate: u32, clock_hz: u32) -> Self {
        let divisor = clock_hz / (baud_rate * 16);

        // DLAB dance, ordering matches vega-sdk's uart_configure(): set
        // DLAB to expose the divisor latch, write it, then drop back into
        // 8N1 operating mode before touching IER/FCR.
        unsafe {
            uart.lcr().write(|w| {
                w.dlab().bit(true);
                w.wls().bits(0b11); // 8 data bits
                w.stb().bit(false); // 1 stop bit
                w.pen().bit(false) // no parity
            });
            crate::fence();
            uart.dll().write(|w| w.data().bits((divisor & 0xFF) as u8));
            uart.dlh()
                .write(|w| w.data().bits(((divisor >> 8) & 0xFF) as u8));
            crate::fence();
            uart.lcr().write(|w| {
                w.dlab().bit(false);
                w.wls().bits(0b11);
                w.stb().bit(false);
                w.pen().bit(false)
            });
        }
        uart.ier()
            .write(|w| w.erbfi().bit(false).etbei().bit(false));
        uart.fcr().write(|w| {
            w.fifo_en()
                .bit(false)
                .rx_fifo_rst()
                .bit(false)
                .tx_fifo_rst()
                .bit(false)
        });

        Serial { uart }
    }

    /// Reads the currently programmed baud divisor.
    ///
    /// Temporarily sets `LCR.DLAB` to expose the divisor latch, then restores
    /// `LCR` byte-for-byte, so a working configuration is left untouched.
    ///
    /// Useful for deriving the UART input clock from a known-good setup:
    /// `clock_hz = divisor * baud_rate * 16`. Since the ROM bootloader
    /// configures UART0 for 115200, reading the divisor it left behind
    /// measures the clock without needing it documented.
    pub fn divisor(&self) -> u16 {
        let lcr = self.uart.lcr().read().bits();
        // SAFETY: writing raw LCR bits is how DLAB is toggled; the original
        // value is restored before returning, so the peripheral's
        // configuration is unchanged on exit.
        unsafe {
            self.uart.lcr().write(|w| w.bits(lcr | 0x80));
            crate::fence();
            let lo = self.uart.dll().read().data().bits() as u16;
            let hi = self.uart.dlh().read().data().bits() as u16;
            crate::fence();
            self.uart.lcr().write(|w| w.bits(lcr));
            (hi << 8) | lo
        }
    }

    /// Enables the receive-data-available interrupt.
    ///
    /// The source must also be unmasked in the PLIC ([`crate::plic`]) and the
    /// core's external interrupts enabled before anything is delivered.
    ///
    /// `IER` bit 0 is the receive enable, confirmed against `vega-sdk`'s
    /// `uart_intr_enable`, which writes `(tx_intr << 1) | rx_intr`.
    ///
    /// Masking gates only the *interrupt*: a byte still arrives and still sets
    /// `LSR.DR`, so a polled read finds it either way.
    pub fn listen(&self) {
        // `modify`, not `write`: the transmit-holding-register-empty enable
        // shares this register, and a plain write would clear it.
        self.uart.ier().modify(|_, w| w.erbfi().bit(true));
        crate::fence();
    }

    /// Disables the receive-data-available interrupt.
    pub fn unlisten(&self) {
        self.uart.ier().modify(|_, w| w.erbfi().bit(false));
        crate::fence();
    }

    /// Reads the interrupt identification register, acknowledging the
    /// transmitter-empty interrupt as a side effect.
    ///
    /// A receive interrupt is **not** cleared by this — it is cleared by
    /// reading the received byte out of `RBR`, which
    /// [`embedded_hal_nb::serial::Read::read`] does. A handler that reads
    /// neither will re-enter immediately.
    ///
    /// Returns the raw `IIR` value; bits 3:1 identify the cause. The PAC
    /// models this register 32 bits wide, though only the low byte is real.
    pub fn interrupt_id(&self) -> u32 {
        self.uart.iir().read().bits()
    }

    /// Blocks until the transmitter is completely idle (TEMT set).
    pub fn flush(&self) {
        while !self.uart.lsr().read().temt().bit_is_set() {
            spin_loop();
        }
    }

    /// Releases the UART peripheral.
    pub fn free(self) -> UART {
        self.uart
    }
}

impl<UART: UartX> serial::ErrorType for Serial<UART> {
    type Error = Error;
}

impl<UART: UartX> serial::Read for Serial<UART> {
    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        let lsr = self.uart.lsr().read();
        if lsr.pe().bit_is_set() {
            Err(nb::Error::Other(Error::Parity))
        } else if lsr.oe().bit_is_set() {
            Err(nb::Error::Other(Error::Overrun))
        } else if lsr.fe().bit_is_set() {
            Err(nb::Error::Other(Error::FrameFormat))
        } else if lsr.dr().bit_is_set() {
            Ok(self.uart.rbr().read().data().bits())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl<UART: UartX> serial::Write for Serial<UART> {
    fn write(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
        if self.uart.lsr().read().thre().bit_is_set() {
            self.uart.thr().write(|w| unsafe { w.data().bits(byte) });
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        if self.uart.lsr().read().temt().bit_is_set() {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

/// Blocking formatted output, so `write!`/`writeln!` work directly.
///
/// Without this every caller ends up hand-rolling a byte loop — which is
/// exactly what the examples in this workspace did before it existed.
impl<UART: UartX> core::fmt::Write for Serial<UART> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.as_bytes() {
            nb::block!(serial::Write::write(self, *byte)).map_err(|_| core::fmt::Error)?;
        }
        Ok(())
    }
}

mod private {
    use thejas32_pac::{Uart0, Uart1, Uart2};

    pub trait Sealed {}

    macro_rules! impl_sealed {
        ($($Uart:ty),*) => {
            $(impl Sealed for $Uart {})*
        };
    }

    impl_sealed!(Uart0, Uart1, Uart2);
}
