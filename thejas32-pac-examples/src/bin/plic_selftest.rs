//! PLIC self-test — validates the interrupt controller, no extra hardware.
//!
//! Interrupts are the first thing built here that can fail by *not happening*,
//! so each phase is bounded and reports a timeout rather than hanging.
//!
//! Phases build up in order of how much has to work:
//!
//! 1. **Enable mask** — pure register readback, interrupts still off.
//! 2. **Mask semantics** — arm a timer with its PLIC bit *masked*: it must
//!    appear in `raw_pending` but not in `active`. Unmask it and it appears in
//!    both. This validates the masking logic with the core still not taking
//!    interrupts, so a mistake cannot run away.
//! 3. **Software interrupt** — the cheapest real interrupt: no peripheral
//!    involved, triggered by writing `mip.MSIP`. If this fires, the trap
//!    vector and handler wiring are correct.
//! 4. **Timer interrupt through the PLIC** — the real path. The already
//!    validated timer supplies both the source *and* an independent reference
//!    for how long N interrupts should take, so this checks delivery *rate*,
//!    not merely that something fired.
//! 5. **Masking stops delivery** — disable the source and confirm the count
//!    stops. Without this, phase 4 passing could still coexist with an enable
//!    bit that does nothing.

#![no_std]
#![no_main]

use panic_halt as _;

use core::sync::atomic::{AtomicU32, Ordering};
use riscv::interrupt::Interrupt as CoreInterrupt;
use thejas32_examples::console::{write_dec, write_hex, write_str};
use thejas32_hal::{plic, serial::Serial, timer::Timer};
use thejas32_pac::interrupt::Interrupt;

/// Timer period driving phase 4: 1 ms at 100 MHz.
const TIMER_PERIOD: u32 = 100_000;
/// Interrupts to collect in phase 4.
const TARGET_IRQS: u32 = 100;
/// Give up if the reference timer passes this without reaching the target
/// (500 ms at 100 MHz) — five times the expected duration.
const WINDOW_LIMIT: u32 = 50_000_000;

/// Increments a counter.
///
/// `riscv32im` has no atomic extension, so `AtomicU32` offers only load and
/// store — no `fetch_add`. A plain read-modify-write is still sound here: this
/// is a single hart, and for each counter the interrupt handler is the only
/// writer. `main` resets them only while the corresponding source is masked.
#[inline]
fn bump(counter: &AtomicU32) {
    counter.store(counter.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
}

static SW_COUNT: AtomicU32 = AtomicU32::new(0);
static EXT_COUNT: AtomicU32 = AtomicU32::new(0);
static TIMER_IRQS: AtomicU32 = AtomicU32::new(0);

#[riscv_rt::core_interrupt(CoreInterrupt::MachineSoft)]
fn machine_soft() {
    bump(&SW_COUNT);
    // Nothing else clears this: the bit stays set and we would re-enter.
    // SAFETY: called from the software-interrupt handler, as required.
    unsafe { plic::clear_software_interrupt() };
}

#[riscv_rt::core_interrupt(CoreInterrupt::MachineExternal)]
fn machine_external() {
    bump(&EXT_COUNT);

    // No claim/complete on this controller: read the masked status and
    // service every set bit directly.
    // SAFETY: read-only access from the handler; no other code holds these
    // while interrupts are enabled.
    let plic_regs = unsafe { thejas32_pac::Plic::steal() };
    let active = plic_regs.intr_status().read().bits();

    if active & (1 << (Interrupt::TIMER0 as u8)) != 0 {
        bump(&TIMER_IRQS);
        // The PLIC cannot acknowledge anything — the *source* must clear
        // itself, here by reading the timer's EOI. Omitting this re-enters
        // forever.
        // SAFETY: EOI is read-only and clearing it is the documented ack.
        let timer = unsafe { thejas32_pac::Timer0::steal() };
        let _ = timer.eoi().read().bits();
    }
}

#[riscv_rt::entry]
fn main() -> ! {
    // The PAC's singleton, taken once. `steal()` below (where present) is a
    // deliberate read-only alias for register inspection, not a second owner.
    let peripherals = thejas32_pac::Peripherals::take().unwrap();
    let mut serial = Serial::new_preconfigured(peripherals.uart0);
    let plic = plic::Plic::new(peripherals.plic);
    let mut timer = Timer::new(peripherals.timer0);
    let mut reference = Timer::new(peripherals.timer1);

    write_str(&mut serial, b"\r\nplic self-test\r\n");

    // ---- [1] enable mask readback ----------------------------------------
    plic.enable(Interrupt::TIMER0);
    plic.enable(Interrupt::UART0);
    let mask_after_enable = plic.enabled_mask();
    plic.disable(Interrupt::UART0);
    let mask_after_disable = plic.enabled_mask();

    let want_enable = (1 << (Interrupt::TIMER0 as u8)) | (1 << (Interrupt::UART0 as u8));
    let want_disable = 1 << (Interrupt::TIMER0 as u8);
    let p1 = mask_after_enable == want_enable && mask_after_disable == want_disable;

    write_str(&mut serial, b"\r\n[1] enable mask\r\n    after enable:  ");
    write_hex(&mut serial, mask_after_enable, 8);
    write_str(&mut serial, b" (want ");
    write_hex(&mut serial, want_enable, 8);
    write_str(&mut serial, b")\r\n    after disable: ");
    write_hex(&mut serial, mask_after_disable, 8);
    write_str(&mut serial, b" (want ");
    write_hex(&mut serial, want_disable, 8);
    write_str(&mut serial, b")\r\n");
    line(&mut serial, b"    mask readback         ", p1);

    // ---- [2] raw vs masked status ----------------------------------------
    // Interrupts are still globally disabled, so this is safe to provoke.
    plic.disable_all();
    timer.start_periodic(TIMER_PERIOD);
    // Arming leaves the interrupt masked at the timer, which gates its output
    // to the PLIC entirely. Without this the controller sees nothing.
    timer.listen();
    // Let the timer expire at least once.
    let _ = poll_reference(&mut reference, TIMER_PERIOD * 4);

    let raw_masked = plic.is_raw_pending(Interrupt::TIMER0);
    let active_masked = plic.is_active(Interrupt::TIMER0);
    plic.enable(Interrupt::TIMER0);
    let raw_unmasked = plic.is_raw_pending(Interrupt::TIMER0);
    let active_unmasked = plic.is_active(Interrupt::TIMER0);

    // Masked: visible in raw, absent from active. Unmasked: visible in both.
    let p2 = raw_masked && !active_masked && raw_unmasked && active_unmasked;
    write_str(&mut serial, b"\r\n[2] raw vs masked status\r\n");
    write_str(&mut serial, b"    masked:   raw=");
    write_dec(&mut serial, raw_masked as u32);
    write_str(&mut serial, b" active=");
    write_dec(&mut serial, active_masked as u32);
    write_str(
        &mut serial,
        b"  (want raw=1 active=0)\r\n    unmasked: raw=",
    );
    write_dec(&mut serial, raw_unmasked as u32);
    write_str(&mut serial, b" active=");
    write_dec(&mut serial, active_unmasked as u32);
    write_str(&mut serial, b"  (want raw=1 active=1)\r\n");
    line(&mut serial, b"    masking works         ", p2);

    timer.stop();
    plic.disable_all();

    // ---- [3] software interrupt ------------------------------------------
    // SAFETY: the MachineSoft handler above is installed and clears MSIP.
    unsafe {
        plic::enable_software_interrupts();
        plic::enable_global_interrupts();
        plic::trigger_software_interrupt();
    }
    let p3 = poll(200_000, || SW_COUNT.load(Ordering::Relaxed) > 0);
    let sw_count = SW_COUNT.load(Ordering::Relaxed);

    write_str(
        &mut serial,
        b"\r\n[3] software interrupt\r\n    handler calls: ",
    );
    write_dec(&mut serial, sw_count);
    write_str(&mut serial, b" (want 1)\r\n");
    line(
        &mut serial,
        b"    fired and cleared     ",
        p3 && sw_count == 1,
    );

    // ---- [4] timer interrupt through the PLIC ----------------------------
    TIMER_IRQS.store(0, Ordering::Relaxed);
    EXT_COUNT.store(0, Ordering::Relaxed);
    plic.enable(Interrupt::TIMER0);
    // SAFETY: MachineExternal handler is installed and clears the source.
    unsafe { plic::enable_external_interrupts() };

    // Interrupt period != polled period on this hardware, so arm for the
    // interrupt rate rather than arming for polling and unmasking.
    timer.start_periodic_interrupt(TIMER_PERIOD);
    reference.start_free_running();
    let start = reference.counter();
    let mut elapsed;
    let mut timed_out = true;
    loop {
        elapsed = start.wrapping_sub(reference.counter());
        if TIMER_IRQS.load(Ordering::Relaxed) >= TARGET_IRQS {
            timed_out = false;
            break;
        }
        if elapsed > WINDOW_LIMIT {
            break;
        }
    }
    // Stop delivery before printing, so the console is not interrupted.
    plic.disable(Interrupt::TIMER0);
    timer.stop();

    let irqs = TIMER_IRQS.load(Ordering::Relaxed);
    let expected = TARGET_IRQS * TIMER_PERIOD;
    let lo = expected - expected / 10;
    let hi = expected + expected / 10;
    let p4 = !timed_out && elapsed >= lo && elapsed <= hi;

    write_str(
        &mut serial,
        b"\r\n[4] timer IRQ via PLIC\r\n    interrupts: ",
    );
    write_dec(&mut serial, irqs);
    write_str(&mut serial, b" (want ");
    write_dec(&mut serial, TARGET_IRQS);
    write_str(&mut serial, b")\r\n    elapsed:    ");
    write_dec(&mut serial, elapsed);
    write_str(&mut serial, b" ticks (want ");
    write_dec(&mut serial, expected);
    write_str(&mut serial, b")\r\n    ext handler calls: ");
    write_dec(&mut serial, EXT_COUNT.load(Ordering::Relaxed));
    write_str(&mut serial, b"\r\n");
    if timed_out {
        write_str(&mut serial, b"    TIMEOUT -- interrupts not delivered\r\n");
    }
    line(&mut serial, b"    delivery rate correct ", p4);

    // ---- [5] masking stops delivery --------------------------------------
    // TIMER0 is already disabled in the PLIC; re-arm the timer and confirm
    // nothing is delivered.
    let before = TIMER_IRQS.load(Ordering::Relaxed);
    timer.start_periodic_interrupt(TIMER_PERIOD);
    let _ = poll_reference(&mut reference, TIMER_PERIOD * 20);
    timer.stop();
    let after = TIMER_IRQS.load(Ordering::Relaxed);
    // "The count did not rise" proves nothing if the count never rose in the
    // first place: with zero interrupts ever delivered this check is vacuous
    // and must not be scored as a pass.
    let vacuous = before == 0;
    let p5 = before == after && !vacuous;

    write_str(
        &mut serial,
        b"\r\n[5] masking stops delivery\r\n    count before: ",
    );
    write_dec(&mut serial, before);
    write_str(&mut serial, b"\r\n    count after:  ");
    write_dec(&mut serial, after);
    write_str(&mut serial, b"\r\n");
    if vacuous {
        write_str(
            &mut serial,
            b"    VACUOUS -- nothing was ever delivered, so this proves\r\n\
              \x20            nothing. Depends on [4] passing.\r\n",
        );
    }
    line(&mut serial, b"    no delivery when masked", p5);

    // SAFETY: shutting interrupts off before the summary.
    unsafe { plic::disable_global_interrupts() };

    let passed = [p1, p2, p3 && sw_count == 1, p4, p5]
        .iter()
        .filter(|&&b| b)
        .count();
    write_str(&mut serial, b"\r\nresult: ");
    write_dec(&mut serial, passed as u32);
    write_str(&mut serial, b"/5 checks passed\r\n");

    loop {
        unsafe { core::arch::asm!("nop") };
    }
}

/// Spins until the reference timer has counted `ticks`, returning elapsed.
fn poll_reference(reference: &mut Timer<thejas32_pac::Timer1>, ticks: u32) -> u32 {
    reference.start_free_running();
    let start = reference.counter();
    loop {
        let elapsed = start.wrapping_sub(reference.counter());
        if elapsed >= ticks {
            return elapsed;
        }
    }
}

fn poll<F: Fn() -> bool>(limit: u32, cond: F) -> bool {
    for _ in 0..limit {
        if cond() {
            return true;
        }
    }
    false
}

fn line<W: embedded_hal_nb::serial::Write<u8>>(serial: &mut W, label: &[u8], ok: bool) {
    write_str(serial, label);
    write_str(serial, if ok { b"PASS\r\n" } else { b"FAIL\r\n" });
}
