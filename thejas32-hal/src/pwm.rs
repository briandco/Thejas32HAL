//! PWM interface for the THEJAS32 SoC.
//!
//! Eight channels at `0x1040_0000`, spaced `0x10`, plus a global control
//! register at `0x1040_0080`. Each channel has a control word, a status bit, a
//! period, and an on/off time.
//!
//! Register semantics follow CDAC's `vega-sdk` (`bsp/drivers/pwm/pwmc.c`); the
//! PAC's control-word layout matches that driver's bitfield exactly.
//!
//! # Channels do not run until the global enable is set
//!
//! Configuring a channel is not enough — [`PwmGlobal::enable`] gates every
//! channel at once. A channel configured while the global enable is clear sits
//! idle with no output and no error.
//!
//! Note that enabling writes **zero first, then the enable bit**. `vega-sdk`
//! does the same, and it is reproduced here.
//!
//! # The global control register does not read back
//!
//! Measured: after [`PwmGlobal::enable`] the channel reports active, but `GCR`
//! reads `0x0000_0000`. The register is therefore write-only or self-clearing,
//! which is consistent with the enable being a pulse rather than a level — and
//! with the vendor's write-zero-then-enable sequence. **Do not treat
//! [`PwmGlobal::bits`] as the enable state**; use [`PwmChannel::is_active`].
//!
//! # On/off time means different things per alignment
//!
//! With [`Alignment::Left`] the on/off register is the **on** (high) time;
//! with [`Alignment::Right`] it is the **off** (low) time. Duty-cycle helpers
//! here assume left alignment, which is what [`Config::default`] selects.

use core::{convert::Infallible, ops::Deref};
use embedded_hal::pwm::{ErrorType, SetDutyCycle};
use thejas32_pac::{pwm0, Pwm0, Pwm1, Pwm2, Pwm3, Pwm4, Pwm5, Pwm6, Pwm7, PwmGlobal as PacGlobal};

/// Global control register: global PWM enable.
const GCR_ENABLE: u32 = 1 << 0;
/// Global control register: global interrupt enable.
const GCR_INTERRUPT: u32 = 1 << 1;

/// Smallest period the hardware accepts, per `vega-sdk`.
pub const MIN_PERIOD: u32 = 2;

/// A PWM channel instance.
///
/// This trait is sealed to prevent external crates from implementing it.
pub trait PwmX: Deref<Target = pwm0::RegisterBlock> + private::Sealed {}

macro_rules! impl_pwmx {
    ($($Pwm:ty),*) => { $(impl PwmX for $Pwm {})* };
}

impl_pwmx!(Pwm0, Pwm1, Pwm2, Pwm3, Pwm4, Pwm5, Pwm6, Pwm7);

/// Channel operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Output held at the idle level; nothing is generated.
    Idle = 0,
    /// Runs for `repeat_count` cycles, then stops.
    OneShot = 1,
    /// Runs until stopped.
    Continuous = 2,
}

/// Which edge of the period the pulse is aligned to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    /// On/off register holds the **on** time.
    Left = 0,
    /// On/off register holds the **off** time.
    Right = 1,
}

/// Output level while the channel is idle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleLevel {
    /// Output sits low while idle.
    Low = 0,
    /// Output sits high while idle.
    High = 1,
}

/// Channel configuration.
///
/// `#[non_exhaustive]`, so build it from [`Config::default`] and the `with_*`
/// methods rather than a struct literal.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Config {
    /// Idle, one-shot, or continuous.
    pub mode: Mode,
    /// Which edge of the period the pulse aligns to.
    pub alignment: Alignment,
    /// Output level while the channel is idle.
    pub idle_level: IdleLevel,
    /// Raise an interrupt on cycle completion. Routed via the PLIC.
    pub interrupt: bool,
    /// Cycles to run in [`Mode::OneShot`]; ignored otherwise. 16 bits.
    pub repeat_count: u16,
}

impl Default for Config {
    /// Continuous, left-aligned, idle low, no interrupt.
    fn default() -> Self {
        Config {
            mode: Mode::Continuous,
            alignment: Alignment::Left,
            idle_level: IdleLevel::Low,
            interrupt: false,
            repeat_count: 0,
        }
    }
}

impl Config {
    /// Sets idle / one-shot / continuous mode.
    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets pulse alignment within the period.
    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Sets the output level while the channel is idle.
    pub fn with_idle_level(mut self, idle_level: IdleLevel) -> Self {
        self.idle_level = idle_level;
        self
    }

    /// Enables or disables the per-channel completion interrupt.
    pub fn with_interrupt(mut self, interrupt: bool) -> Self {
        self.interrupt = interrupt;
        self
    }

    /// Sets the one-shot repeat count.
    pub fn with_repeat_count(mut self, repeat_count: u16) -> Self {
        self.repeat_count = repeat_count;
        self
    }
}

/// A single PWM channel.
pub struct PwmChannel<PWM> {
    pwm: PWM,
    period: u32,
}

impl<PWM: PwmX> PwmChannel<PWM> {
    /// Takes ownership of a channel and configures it.
    ///
    /// `period` is in peripheral clock cycles (see
    /// [`crate::PERIPHERAL_CLOCK_HZ`]) and is clamped to [`MIN_PERIOD`]. The
    /// channel produces nothing until [`PwmGlobal::enable`] is called.
    /// Ordering matches `vega-sdk`'s working examples: period and on/off time
    /// are written **before** the control word, and the global enable comes
    /// last. On a bus that reorders writes (see [`crate::fence`]) that order is
    /// not assumed to be incidental.
    pub fn new(pwm: PWM, config: Config, period: u32) -> Self {
        let mut this = PwmChannel { pwm, period: 0 };
        this.set_period(period);
        this.configure(config);
        this
    }

    /// Rewrites the control word.
    pub fn configure(&mut self, config: Config) {
        // SAFETY: every field is masked to its width by the PAC writers.
        unsafe {
            self.pwm.control().write(|w| {
                w.mode().bits(config.mode as u8);
                w.align().bits(config.alignment as u8);
                w.ie().bit(config.interrupt);
                w.opc().bit(config.idle_level == IdleLevel::High);
                w.repeat_count().bits(config.repeat_count)
            })
        };
        crate::fence();
    }

    /// Sets the period, in peripheral clock cycles.
    pub fn set_period(&mut self, period: u32) {
        let period = period.max(MIN_PERIOD);
        self.period = period;
        // SAFETY: the period register takes an arbitrary 32-bit value.
        unsafe { self.pwm.period().write(|w| w.bits(period)) };
        crate::fence();
    }

    /// The configured period.
    pub fn period(&self) -> u32 {
        self.period
    }

    /// Sets the on/off register directly — **on** time under
    /// [`Alignment::Left`], **off** time under [`Alignment::Right`].
    pub fn set_on_off_time(&mut self, ticks: u32) {
        // SAFETY: the on/off register takes an arbitrary 32-bit value.
        unsafe { self.pwm.on_off().write(|w| w.bits(ticks)) };
        crate::fence();
    }

    /// Whether the channel reports itself active.
    pub fn is_active(&self) -> bool {
        self.pwm.status().read().active().bit_is_set()
    }

    /// Puts the channel in idle mode, leaving other settings alone.
    pub fn stop(&mut self) {
        self.pwm
            .control()
            .modify(|_, w| unsafe { w.mode().bits(Mode::Idle as u8) });
        crate::fence();
    }

    /// The raw control word, for diagnostics.
    pub fn control_bits(&self) -> u32 {
        self.pwm.control().read().bits()
    }

    /// Releases the channel.
    pub fn free(self) -> PWM {
        self.pwm
    }
}

impl<PWM: PwmX> ErrorType for PwmChannel<PWM> {
    type Error = Infallible;
}

impl<PWM: PwmX> SetDutyCycle for PwmChannel<PWM> {
    /// The period, saturated to `u16`.
    ///
    /// Periods longer than `u16::MAX` still work — [`Self::set_duty_cycle`]
    /// scales into the full period — but the resolution the trait can express
    /// is capped here.
    fn max_duty_cycle(&self) -> u16 {
        self.period.min(u16::MAX as u32) as u16
    }

    /// # Rounding
    ///
    /// With a period above `u16::MAX` the duty is scaled twice — once by the
    /// trait's own helpers, once here — and both divisions truncate. Asking
    /// for 50% of a 100,000-tick period yields 49,999, not 50,000: the trait
    /// computes `50 * 65535 / 100 = 32767`, and `32767 * 100000 / 65535`
    /// is 49,999.2. That 0.002% shortfall is inherent to a 16-bit duty
    /// channel, not a defect. Use [`PwmChannel::set_on_off_time`] when an
    /// exact tick count matters.
    fn set_duty_cycle(&mut self, duty: u16) -> Result<(), Self::Error> {
        let max = self.max_duty_cycle() as u32;
        // Scale into the real period so long periods keep full range.
        let on = if max == 0 {
            0
        } else {
            ((duty as u64 * self.period as u64) / max as u64) as u32
        };
        self.set_on_off_time(on);
        Ok(())
    }
}

/// The global PWM control register, gating every channel.
pub struct PwmGlobal {
    global: PacGlobal,
}

impl PwmGlobal {
    /// Takes ownership of the global control register, leaving PWM disabled.
    pub fn new(global: PacGlobal) -> Self {
        let this = PwmGlobal { global };
        this.disable();
        this
    }

    /// Enables all configured channels.
    fn write_gcr(&self, value: u32) {
        // Zero first, then the value. `vega-sdk` does this and it reads as a
        // deliberate restart pulse rather than redundancy, so it is kept.
        // SAFETY: the global control register is a plain bitmask.
        unsafe {
            self.global.gcr().write(|w| w.bits(0));
            crate::fence();
            self.global.gcr().write(|w| w.bits(value));
        }
        crate::fence();
    }

    /// Starts all configured channels.
    pub fn enable(&self) {
        self.write_gcr(GCR_ENABLE);
    }

    /// Starts all configured channels with interrupts enabled globally.
    ///
    /// Individual channels still need `Config::interrupt`, and the PWM source
    /// must be unmasked in the PLIC.
    pub fn enable_with_interrupts(&self) {
        self.write_gcr(GCR_ENABLE | GCR_INTERRUPT);
    }

    /// Stops all channels.
    pub fn disable(&self) {
        // SAFETY: as `write_gcr`.
        unsafe { self.global.gcr().write(|w| w.bits(0)) };
        crate::fence();
    }

    /// The raw global control value.
    ///
    /// Reads `0` even while PWM is enabled — see the module docs. Useful only
    /// for diagnostics; [`PwmChannel::is_active`] is the real enable state.
    pub fn bits(&self) -> u32 {
        self.global.gcr().read().bits()
    }
}

mod private {
    use thejas32_pac::{Pwm0, Pwm1, Pwm2, Pwm3, Pwm4, Pwm5, Pwm6, Pwm7};

    pub trait Sealed {}

    macro_rules! impl_sealed {
        ($($Pwm:ty),*) => { $(impl Sealed for $Pwm {})* };
    }

    impl_sealed!(Pwm0, Pwm1, Pwm2, Pwm3, Pwm4, Pwm5, Pwm6, Pwm7);
}
