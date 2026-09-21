//! Shared helpers for the example binaries.

#![no_std]

pub mod ads1015;

pub mod console {
    use embedded_hal_nb::{nb, serial::Write};

    /// Writes a byte slice.
    pub fn write_str<W: Write<u8>>(serial: &mut W, bytes: &[u8]) {
        for &byte in bytes {
            nb::block!(serial.write(byte)).ok();
        }
    }

    /// Writes a `u32` as decimal.
    pub fn write_dec<W: Write<u8>>(serial: &mut W, mut value: u32) {
        if value == 0 {
            nb::block!(serial.write(b'0')).ok();
            return;
        }
        let mut buf = [0u8; 10];
        let mut len = 0;
        while value > 0 {
            buf[len] = b'0' + (value % 10) as u8;
            value /= 10;
            len += 1;
        }
        while len > 0 {
            len -= 1;
            nb::block!(serial.write(buf[len])).ok();
        }
    }

    /// Writes the low `digits` nibbles of `value` as hex, most significant first.
    pub fn write_hex<W: Write<u8>>(serial: &mut W, value: u32, digits: u8) {
        for i in (0..digits).rev() {
            let nibble = ((value >> (i * 4)) & 0xF) as u8;
            let ch = if nibble < 10 {
                b'0' + nibble
            } else {
                b'a' + (nibble - 10)
            };
            nb::block!(serial.write(ch)).ok();
        }
    }
}

/// A minimal single-future executor.
///
/// Deliberately lives here rather than in the HAL: `embedded-hal-async` drivers
/// must not drag in an executor, so the HAL provides futures and the
/// application chooses how to run them.
///
/// # Why this busy-polls
///
/// An earlier version registered a real waker and slept in `wfi` between
/// polls. It completed the first await and then hung on every one after,
/// while an otherwise identical busy-polling version completed three in a row
/// with identical timing. Two facts make the sleeping design pointless here
/// anyway:
///
/// * `wfi` is a **NOP** on this core (measured — see the `wfi-diag` example),
///   so sleeping saves nothing.
/// * With no `wfi`, the waker has nothing to wake, so the whole
///   `WOKEN`-flag-and-critical-section apparatus is dead weight.
///
/// So this polls in a plain loop with a no-op waker. Async here buys
/// composability, not power saving. A core with a working `wfi` would want the
/// sleeping variant back, along with the critical section that makes the
/// check-then-sleep race-free.
pub mod executor {
    use core::future::Future;
    use core::task::{Context, RawWaker, RawWakerVTable, Waker};

    fn noop(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(core::ptr::null(), &VTABLE)
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);

    /// Polls `future` to completion, up to `limit` polls.
    ///
    /// Returns `None` if the bound is hit. Prefer this over an unbounded loop:
    /// any broken link in the wake chain otherwise becomes a silent hang with
    /// no output to diagnose from.
    pub fn try_block_on<F: Future>(future: F, limit: u32) -> (Option<F::Output>, u32) {
        // SAFETY: every vtable operation is a no-op and sound for a unit waker.
        let waker = unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) };
        let mut cx = Context::from_waker(&waker);
        let mut future = core::pin::pin!(future);

        for polls in 1..=limit {
            if let core::task::Poll::Ready(value) = future.as_mut().poll(&mut cx) {
                return (Some(value), polls);
            }
        }
        (None, limit)
    }
}
