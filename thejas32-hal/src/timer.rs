//! Timer interface for the THEJAS32 SoC.
//!
//! Three Synopsys DesignWare APB timers at `0x1000_0A00`, spaced `0x14`
//! apart. Each counts **down** from its load count to zero, then raises an
//! interrupt and reloads (user-defined mode) or wraps (free-running mode).
//!
//! Register semantics follow CDAC's `vega-sdk` (`bsp/drivers/timer/timer.c`),
//! with three corrections noted below.
//!
//! # Write ordering
//!
//! Arming a timer writes `load_count` and then `control`. Those two writes
//! **must** be separated by a [`crate::fence`]: without it the enable reaches
//! the peripheral first and the counter free-runs from `0xFFFF_FFFF`,
//! ignoring the load count entirely — even though `load_count` reads back
//! correctly. Verified on hardware; the fence is the only difference between
//! a working and a broken arm sequence. `load_count` must also be written
//! *before* enabling, as the value is latched at the enable edge.
//!
//! # Polling a masked timer
//!
//! DesignWare exposes `IntrStatus` as *raw status AND NOT mask*, so a timer
//! polled with its interrupt masked never sets its own `IntrStatus`. The
//! expiry flag must be read from the **global raw** status register at
//! `+0xA8` instead. (`vega-sdk` reaches the same conclusion the hard way —
//! its `IntrStatus` poll is commented out in favour of the global register.)
//!
//! # The polled flag and the interrupt run at different rates
//!
//! With a load count of N the **interrupt** fires every N clocks, but the
//! **polled** expiry flag appears only every 2N — the interrupt asserts on
//! every underflow while the status flag surfaces every second one. Both were
//! measured on hardware. [`Timer::start_periodic`] therefore programs half the
//! requested period and [`Timer::start_periodic_interrupt`] programs it whole;
//! use one path per timer rather than mixing them.
//!
//! # The interrupt mask gates the PLIC, not the flag
//!
//! Arming masks the interrupt (`intr_mask = 1`) because expiry is polled from
//! the *global raw* status, which the mask does not affect. The mask does gate
//! the peripheral's interrupt output, so a masked timer never reaches the
//! interrupt controller — polling behaves perfectly while interrupts never
//! arrive. Use [`Timer::listen`] for interrupt-driven use. (`vega-sdk` draws
//! the same distinction: `0x07` for polled delays, `0x03` for interrupt mode.)
//!
//! # Clearing expiry
//!
//! Reading `EOI` clears a timer's pending flag. `vega-sdk`'s `timer_put_delay`
//! leaves that read commented out, so its raw status stays set after the
//! first delay and a second call can observe a stale flag and return
//! immediately. This driver clears `EOI` both before arming and after
//! expiry, so back-to-back delays are each timed independently.

use core::ops::Deref;
use embedded_hal::delay::DelayNs;
use thejas32_pac::{timer0, Timer0, Timer1, Timer2, TimerGlobal};

/// A DesignWare timer instance.
///
/// This trait is sealed to prevent external crates from implementing it.
pub trait TimerX: Deref<Target = timer0::RegisterBlock> + private::Sealed {
    /// This timer's bit position in the global raw interrupt status register.
    const INDEX: u8;

    /// Steals a handle to this timer.
    ///
    /// # Safety
    ///
    /// Breaks the singleton guarantee. Used by [`on_interrupt`], where the
    /// handler needs the peripheral but cannot be handed one.
    unsafe fn steal() -> Self;
}

macro_rules! impl_timerx {
    ($($Tim:ty => $idx:expr),* $(,)?) => {
        $(
            impl TimerX for $Tim {
                const INDEX: u8 = $idx;
                unsafe fn steal() -> Self {
                    unsafe { <$Tim>::steal() }
                }
            }
        )*
    };
}

impl_timerx!(Timer0 => 0, Timer1 => 1, Timer2 => 2);

/// Count-down cycles the counter completes before the expiry flag asserts.
///
/// **Measured, not specified.** With a load count of N the flag appears after
/// 2N peripheral clocks — verified at N = 1,000 / 10,000 / 100,000 / 200,000,
/// all landing within 0.1% of exactly 200%. The counter itself is not slow:
/// sampling TIMER0 and TIMER1 across one window shows both advancing by an
/// identical amount, and TIMER1's rate is independently confirmed at 100 MHz
/// against the UART baud clock. Counting reloads directly shows two before
/// the flag appears.
///
/// [`Timer::start_periodic`] divides by this so a requested period of N ticks
/// takes N ticks.
///
/// Note this characterises the *first* flag after arming, which is what
/// [`Timer::delay_ticks`] depends on. The steady-state interval between
/// successive flags without re-arming is verified by `timer-selftest` phase 6.
const CLOCKS_PER_FLAG: u32 = 2;

/// A countdown timer.
pub struct Timer<TIM> {
    tim: TIM,
    clock_hz: u32,
}

impl<TIM: TimerX> Timer<TIM> {
    /// Wraps a timer, assuming the SoC's [`crate::PERIPHERAL_CLOCK_HZ`].
    pub fn new(tim: TIM) -> Self {
        Self::with_clock(tim, crate::PERIPHERAL_CLOCK_HZ)
    }

    /// Wraps a timer driven by a non-default clock.
    pub fn with_clock(tim: TIM, clock_hz: u32) -> Self {
        let timer = Timer { tim, clock_hz };
        timer.stop();
        timer
    }

    /// Disables the timer and clears any pending expiry.
    pub fn stop(&self) {
        self.tim.control().write(|w| {
            w.enable()
                .bit(false)
                .mode()
                .bit(false)
                .intr_mask()
                .bit(false)
        });
        crate::fence();
        self.clear_expired();
        crate::fence();
    }

    /// Routes this timer's expiry to the interrupt controller.
    ///
    /// Arming leaves the interrupt **masked**, because [`Timer::is_expired`]
    /// polls the *global raw* status, which the mask does not affect. The mask
    /// does gate the interrupt line out of the peripheral, so a masked timer
    /// never reaches the PLIC — polling works while interrupts silently do
    /// not. Call this after arming to drive an interrupt.
    ///
    /// The caller must still unmask the source in the PLIC and enable
    /// external interrupts on the core; see [`crate::plic`].
    pub fn listen(&self) {
        self.tim.control().modify(|_, w| w.intr_mask().bit(false));
        crate::fence();
    }

    /// Stops routing expiry to the interrupt controller. Polling still works.
    pub fn unlisten(&self) {
        self.tim.control().modify(|_, w| w.intr_mask().bit(true));
        crate::fence();
    }

    /// Reads `EOI`, clearing this timer's pending flag.
    pub fn clear_expired(&self) {
        let _ = self.tim.eoi().read().bits();
    }

    /// Whether this timer has counted down to zero since it was last cleared.
    ///
    /// Reads the *global raw* status, so this is valid whether or not the
    /// interrupt is masked.
    pub fn is_expired(&self) -> bool {
        // SAFETY: `raw_intr_status` is a read-only register, and this handle
        // is used only to read it — the case `Periph::steal` documents as
        // sound.
        let global = unsafe { TimerGlobal::steal() };
        let status = global.raw_intr_status().read();
        match TIM::INDEX {
            0 => status.timer0().bit_is_set(),
            1 => status.timer1().bit_is_set(),
            _ => status.timer2().bit_is_set(),
        }
    }

    /// Starts a periodic timer that expires every `ticks` peripheral clocks.
    ///
    /// `ticks` is the *period in clocks*, not the raw load count — the load
    /// register is programmed with half of it to compensate for the hardware's
    /// late flag (see `CLOCKS_PER_FLAG`). Periods below 2 clocks cannot be
    /// represented and are clamped.
    ///
    /// The interrupt is masked, so expiry is observed by polling
    /// [`Timer::is_expired`] rather than through the PLIC.
    pub fn start_periodic(&mut self, ticks: u32) {
        // The flag is one count-down cycle late; halve the load so the caller
        // gets the period they asked for. See `CLOCKS_PER_FLAG`.
        let load = (ticks / CLOCKS_PER_FLAG).max(1);
        self.stop();
        // SAFETY: the load count register takes an arbitrary 32-bit value.
        unsafe { self.tim.load_count().write(|w| w.bits(load)) };
        // Required: without this the enable below overtakes the load and the
        // counter free-runs from 0xFFFF_FFFF. See `crate::fence`.
        crate::fence();
        self.tim.control().write(|w| {
            w.enable()
                .bit(true)
                .mode()
                .bit(true) // user-defined: reload from load_count
                .intr_mask()
                .bit(true) // masked; poll the global raw status
        });
        crate::fence();
    }

    /// Starts a periodic timer whose **interrupt** fires every `ticks` clocks,
    /// and routes it to the interrupt controller.
    ///
    /// This programs a *different* load count than [`Timer::start_periodic`]
    /// for the same nominal period, because the two paths run at different
    /// rates on this hardware — see the note below. Do not poll
    /// [`Timer::is_expired`] on a timer armed this way and expect the same
    /// period; pick one path per timer.
    ///
    /// The caller must still unmask this timer in the PLIC and enable external
    /// interrupts on the core; see [`crate::plic`].
    ///
    /// # Interrupts run at twice the polled rate
    ///
    /// Measured on hardware: with a load count of N the interrupt fires every
    /// N clocks, while the polled flag appears only every 2N. 100 interrupts
    /// at a load of 50,000 took 5,000,172 clocks (≈50,002 each), against a
    /// polled steady-state period of 100,000 for the same load. The interrupt
    /// asserts on every underflow; the status flag surfaces every second one.
    pub fn start_periodic_interrupt(&mut self, ticks: u32) {
        // Interrupt period == load count, so no halving here.
        let load = ticks.max(1);
        self.stop();
        // SAFETY: the load count register takes an arbitrary 32-bit value.
        unsafe { self.tim.load_count().write(|w| w.bits(load)) };
        crate::fence();
        self.tim.control().write(|w| {
            w.enable()
                .bit(true)
                .mode()
                .bit(true) // user-defined: reload from load_count
                .intr_mask()
                .bit(false) // unmasked: drive the interrupt controller
        });
        crate::fence();
    }

    /// Starts a free-running down-counter from `0xFFFF_FFFF`, for measuring
    /// elapsed time with [`Timer::counter`].
    pub fn start_free_running(&mut self) {
        self.stop();
        // SAFETY: as `start_periodic`.
        unsafe { self.tim.load_count().write(|w| w.bits(u32::MAX)) };
        crate::fence();
        self.tim.control().write(|w| {
            w.enable()
                .bit(true)
                .mode()
                .bit(false) // free-running
                .intr_mask()
                .bit(true)
        });
        // Also ensures the counter is running before the first `counter()`
        // read; without it the first read returns a pre-load value.
        crate::fence();
    }

    /// The current counter value. Counts **down**, so elapsed ticks between
    /// two samples are `earlier.wrapping_sub(later)`.
    pub fn counter(&self) -> u32 {
        self.tim.current_value().read().bits()
    }

    /// Blocks for `ticks` timer cycles.
    pub fn delay_ticks(&mut self, ticks: u32) {
        if ticks == 0 {
            return;
        }
        self.start_periodic(ticks);
        while !self.is_expired() {}
        self.stop();
    }

    /// Converts a duration in nanoseconds to timer ticks, saturating.
    pub(crate) fn ticks_for_ns(&self, ns: u32) -> u32 {
        let ticks = (ns as u64 * self.clock_hz as u64) / 1_000_000_000;
        ticks.min(u32::MAX as u64) as u32
    }

    /// Releases the timer peripheral.
    pub fn free(self) -> TIM {
        self.stop();
        self.tim
    }
}

impl<TIM: TimerX> DelayNs for Timer<TIM> {
    fn delay_ns(&mut self, ns: u32) {
        self.delay_ticks(self.ticks_for_ns(ns));
    }
}

mod private {
    use thejas32_pac::{Timer0, Timer1, Timer2};

    pub trait Sealed {}

    impl Sealed for Timer0 {}
    impl Sealed for Timer1 {}
    impl Sealed for Timer2 {}
}

// ============================================================================
// Async support
// ============================================================================

/// Per-timer state shared between the interrupt handler and the future.
///
/// `riscv32im` has load/store atomics but **no compare-and-swap**, so the usual
/// lock-free `AtomicWaker` is unavailable. A `critical-section` mutex is used
/// instead; the critical-section implementation comes from the `riscv` crate's
/// `critical-section-single-hart` feature.
struct TimerState {
    /// Set by the interrupt handler when the timer has expired.
    fired: critical_section::Mutex<core::cell::Cell<bool>>,
    /// Waker for the task awaiting this timer.
    waker: critical_section::Mutex<core::cell::RefCell<Option<core::task::Waker>>>,
}

impl TimerState {
    const fn new() -> Self {
        TimerState {
            fired: critical_section::Mutex::new(core::cell::Cell::new(false)),
            waker: critical_section::Mutex::new(core::cell::RefCell::new(None)),
        }
    }
}

static TIMER_STATE: [TimerState; 3] = [TimerState::new(), TimerState::new(), TimerState::new()];

/// Services a timer interrupt: acknowledges the source, stops the timer, and
/// wakes whatever was awaiting it.
///
/// Call this from your `MachineExternal` handler when the PLIC reports this
/// timer active. Nothing else acknowledges the source — this controller has no
/// claim/complete cycle (see [`crate::plic`]), so omitting this re-enters the
/// handler forever.
///
/// ```ignore
/// #[riscv_rt::core_interrupt(CoreInterrupt::MachineExternal)]
/// fn machine_external() {
///     if plic.is_active(Interrupt::TIMER0) {
///         thejas32_hal::timer::on_interrupt::<thejas32_pac::Timer0>();
///     }
/// }
/// ```
pub fn on_interrupt<TIM: TimerX>() {
    // SAFETY: the handler needs the peripheral and cannot be passed one; access
    // here is confined to acknowledging and disabling this timer.
    let tim = unsafe { TIM::steal() };

    // Acknowledge at the source, then stop so it cannot re-fire while the
    // woken task is still running.
    tim.control().write(|w| {
        w.enable()
            .bit(false)
            .mode()
            .bit(false)
            .intr_mask()
            .bit(false)
    });
    crate::fence();
    let _ = tim.eoi().read().bits();
    crate::fence();

    // Deliberately *not* `critical_section::with`. Acquiring one here executes
    // `csrrci mstatus, 8`, and on this core that instruction also clears
    // `mstatus.MPIE` when the trap was taken at the moment another critical
    // section released. `mret` then restores `MIE` from a zeroed `MPIE` and
    // interrupts are disabled for good — the first awaited delay completes and
    // every one after it hangs. Measured; see the crate docs.
    //
    // SAFETY: the core clears `mstatus.MIE` on trap entry (verified: `mstatus`
    // reads `0x1880` on entry to this handler), this is a single-hart system,
    // and nested traps cannot occur with `MIE` clear. So exclusive access to
    // `TIMER_STATE` is already held and re-acquiring it is both unnecessary and
    // destructive. The token does not outlive the handler.
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    let cs = unsafe { critical_section::CriticalSection::new() };

    let state = &TIMER_STATE[TIM::INDEX as usize];
    state.fired.borrow(cs).set(true);
    if let Some(waker) = state.waker.borrow(cs).borrow_mut().take() {
        waker.wake();
    }

    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

/// A timer that yields instead of spinning.
///
/// Wraps a [`Timer`] rather than adding traits to it: the blocking and async
/// `DelayNs` traits share a method name, so having both on one type would make
/// every `delay_ns` call ambiguous.
///
/// Requires that [`on_interrupt`] is wired into the `MachineExternal` handler,
/// that this timer is unmasked in the PLIC, and that external interrupts are
/// enabled on the core. Without those the future never completes.
pub struct AsyncTimer<TIM> {
    timer: Timer<TIM>,
}

impl<TIM: TimerX> AsyncTimer<TIM> {
    /// Wraps a timer for async use.
    pub fn new(timer: Timer<TIM>) -> Self {
        AsyncTimer { timer }
    }

    /// Returns the underlying blocking timer.
    pub fn free(self) -> Timer<TIM> {
        self.timer
    }

    /// Waits `ticks` peripheral clocks, yielding while the timer runs.
    ///
    /// Uses the interrupt rate, so `ticks` is the real wait in clocks — see
    /// [`Timer::start_periodic_interrupt`] for why that differs from the polled
    /// path.
    pub async fn delay_ticks(&mut self, ticks: u32) {
        if ticks == 0 {
            return;
        }
        let index = TIM::INDEX as usize;

        // Clear any stale completion before arming, or the first poll returns
        // immediately with a result from a previous wait.
        critical_section::with(|cs| {
            TIMER_STATE[index].fired.borrow(cs).set(false);
            TIMER_STATE[index].waker.borrow(cs).replace(None);
        });

        self.timer.start_periodic_interrupt(ticks);

        core::future::poll_fn(|cx| {
            critical_section::with(|cs| {
                let state = &TIMER_STATE[index];
                if state.fired.borrow(cs).get() {
                    core::task::Poll::Ready(())
                } else {
                    // Register before returning Pending: the interrupt can land
                    // between the check and the store otherwise, and the wake
                    // would be lost. The critical section closes that window.
                    state.waker.borrow(cs).replace(Some(cx.waker().clone()));
                    core::task::Poll::Pending
                }
            })
        })
        .await;

        self.timer.stop();
    }
}

impl<TIM: TimerX> embedded_hal_async::delay::DelayNs for AsyncTimer<TIM> {
    async fn delay_ns(&mut self, ns: u32) {
        let ticks = self.timer.ticks_for_ns(ns);
        self.delay_ticks(ticks).await;
    }
}
