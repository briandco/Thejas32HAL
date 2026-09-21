# Peripheral evidence

What each driver is derived from, where it deviates from its sources, and what
is measured rather than assumed.

Structure borrowed from a generated HAL bundle's per-peripheral comparison
document, which separates these cleanly in a way our README did not: source
evidence, intentional differences, and items only hardware can settle.

**Why this matters here.** Two of this project's sources are actively wrong. The
vendor SDK's UART clock constant is off by 2.6× and fails silently; the vendor's
own I2C and SPI drivers contain defects. A driver that "follows the reference"
without saying which parts it follows is not traceable.

## Status key

| Mark | Meaning |
|---|---|
| ✅ | measured on hardware by a self-test |
| ⚠ | implemented and compiles, not proven on silicon |
| ❌ | known not to work |

---

## serial (UART)

**Sources.** 16550-compatible register block; init sequence and status bits
cross-checked against `vega-sdk`'s `bsp/drivers/uart/uart.c`. Interrupt enable
bit from `uart_intr_enable`, which writes `(tx_intr << 1) | rx_intr`.

**Deviations.** `Serial::new_preconfigured` adopts the bootloader's existing
configuration rather than programming a divisor. Deliberate: the input clock is
not reliably documented, and a wrong divisor produces garbage rather than an
error.

**Hardware findings.** Peripheral clock is **100 MHz**, derived from the divisor
(54) the ROM bootloader leaves in UART0's latch for its working 115200 console.
`vega-sdk`'s `40_000_000` does not hold for this board.

**Status.** ✅ `uart-echo`; interrupts ✅ `uart-intr-selftest` 4/4, including that
masking gates the interrupt without swallowing the byte.

---

## gpio

**Sources.** `vega-sdk`'s `bsp/drivers/gpio/gpio.c` for the bit-banded aperture;
ARIES v3.0 pinout for board mapping.

**Deviations.** The PAC models only mask 0 and mask `0xFFFF` of the data
aperture, so per-pin addresses are computed directly from the peripheral base.
The direction register at `BASE + 0x40000` is not bit-banded and needs a
read-modify-write.

**Hardware findings.**

- Interrupts are **level-sensitive, active high**, with **no** enable, edge-select
  or status register — the PLIC line is the only control. Measured with static
  levels, which contain no edges: high produced interrupts continuously, low
  produced none.
- A handler must therefore **mask its own PLIC line** or livelock the core.
- GPIO22/23/24 drive the onboard RGB LED and are not free pins.

**Open.** No `fence()` between a direction write and a subsequent data write.
The bus is known to reorder writes to the same peripheral; whether GPIO is
affected is **unverified**, and a colour anomaly in a third-party LED sweep may
or may not be evidence of it.

**Status.** ✅ `gpio-selftest` 9/9 with per-pin cross-talk isolation; interrupts
✅ `gpio-intr-selftest` 4/4.

---

## spi

**Sources.** `vega-sdk`'s `bsp/drivers/spi/spi.c`. Interrupt enables are control
bits 6 (RX) and 7 (TX), from `SPI_set_intr`; status bits 2 and 3 from
`spi_intr_status`.

**Hardware findings.**

- The **status register is read-to-clear**. Three consecutive reads after a
  transfer give `0x00c4`, `0x0080`, `0x0080` with the byte still in `RxData` — so
  observing the register destroys the flag, not consuming the data.
- Consequently **interrupts and the blocking `SpiBus` API cannot be combined**:
  `SpiBus` polls that register and read-modify-writes the control register before
  every byte, both of which an interrupt handler needs to own. Interrupt-driven
  use requires register-level transfers.

**Status.** ✅ `spi-selftest` — config 3/3, state machine 4/4, loopback 5/5
(needs `J1_7 ↔ J1_6`). Interrupts ✅ for the register-level pattern.

---

## i2c

**Sources.** `vega-sdk`'s `bsp/drivers/i2c/i2c.c`; `IER` bits from
`I2C_enable_intr`, `SR1` bit 1 from `I2C_intr_handler`.

**Deviations from the vendor driver — three defects in it.**

1. `i2c_data_read` advances its output pointer without ever reading `RxFF`, so
   received bytes are discarded and the caller's buffer is untouched.
2. `I2C_intr_handler`'s receive branch, `(status & 0x03) == 0x03`, is
   unreachable — the transmit test catches every case with bit 1 set.
3. Every wait is unbounded, so an absent device hangs the caller. This driver
   bounds all of them and returns `Error::Timeout`.

**Hardware findings.**

- **No repeated START.** A START is accepted only after the bus is released, so
  `transaction` issues a STOP after every operation. This deviates from
  `embedded-hal`; devices needing a true repeated START are unsupported.
- `SR1.NACK` is **not valid when `xfer_complete` asserts** — reading it too early
  reports every address as acknowledging, producing a phantom device at all 112
  scanned addresses.
- Status registers are **not** read-to-clear, unlike SPI's — verified directly.
- There are **three** controllers, not two. `0x1000_1000` is `I2C_ADC`
  (instance 8), not the ADC block the SDK's `config.h` labels it.

**Status.** ✅ `ads1015-selftest` 4/4 against the board's ADS1015 — ACK/NACK
discrimination, 3-byte write, 2-byte read, exact config echo. Interrupts ✅
`i2c-intr-selftest` 4/4, 1,716 interrupts for 1,716 writes.

---

## timer

**Sources.** Synopsys DesignWare APB timer; `vega-sdk`'s `timer.c`.

**Hardware findings.**

- **The polled flag and the interrupt run at different rates.** The polled path
  needs `CLOCKS_PER_FLAG = 2`; the interrupt path does not. Hence separate
  `start_periodic` and `start_periodic_interrupt`.
- Arming requires a **`fence` between `load_count` and `control`**, or the enable
  overtakes the load and the counter free-runs from `0xFFFF_FFFF` — while
  `load_count` still reads back correctly.

**Status.** ✅ `timer-selftest` 8/8 — delays within 0.03%, period within 0.006%,
rate cross-checked against UART baud. Async ✅ `async-selftest` 3/3.

---

## pwm

**Sources.** `vega-sdk`'s `pwm.c`.

**Hardware findings.** The global control register does not read back; its state
must be tracked in software.

**Status.** ✅ `pwm-selftest` 4/4 — with `J2_5 → J2_11` the measured waveform is
within 0.005% on period and 0.012% on high time.

---

## plic

**Sources.** `vega-sdk`'s `interrupt.h` (RV32 block — the RV64 block in the same
file has different numbers and does not apply).

**Deviations.** Non-standard controller: a flat enable bitmask with **no
claim/complete cycle**. The source acknowledges itself. `mip.MSIP` is writable,
which the RISC-V specification does not require.

**Hardware findings.** `csrrci mstatus, 8` — what `critical_section::acquire`
emits — can leave `mstatus.MIE` stuck clear, after which interrupts are off
permanently and silently. Architecturally that instruction may write bit 3 only,
so this is a privileged-spec deviation.

**This is not confined to interrupt handlers.** It was first found inside one
(`async-diag8`: an *empty* critical section turned `0x1880` into `0x1800`), and
the rule was written as "never inside a handler". `cs-storm-diag` later showed
that ordinary task code does it too: N back-to-back empty critical sections with
the timer running survived at N = 1k/10k/100k/1M, then died at N = 4M with
`mstatus=0x1800` (MIE=0, MPIE=0) and the tick counter frozen. A bare
`csrsi mstatus, 8` resumed ticks, so `MIE` was the stranded bit.

The mechanism: a trap landing exactly at a *release* has its hardware MPIE
update clobbered; the next acquire then reads `MIE` as already 0, so its release
skips the re-enable. A trap landing *inside* a section is harmless — the release
re-enables normally — which is why it self-heals almost always.

It is a **race, not a threshold**: the 1M storm ran clean at full tick rate while
the 4M storm died ~3 ms in (~37k iterations). Rate is roughly one death per few
million critical sections, so short self-tests effectively never see it. Do not
put a critical section in a hot loop on this core.

**Status.** ✅ `plic-selftest` 5/5.

---

## adc — deprecated

**`0x1000_1000` is not an ADC.** It is the third I2C controller. The SDK
describes it both ways: `config.h` calls it `ADC_BASE_ADDRESS` with a register
struct in `adc.h`, while `i2c.h` declares `I2C_ADC 8`. Only the I2C reading has
an implementation behind it — `adc_analogRead` is declared and implemented
nowhere.

Hardware settles it: a device acknowledges at 0x48 there and all four channels
convert. The THEJAS32 package exposes **no analog pins**; the board's A0–A3 are a
TI ADS1015 on that bus.

**Status.** ❌ Module deprecated, scheduled for removal. Use the ADS1015 driver
over `I2c2`.

---

## Update policy

Re-check a section when its vendor source, the PAC's register block, or the
board revision changes. A ✅ claim must name the self-test that produced it; if a
test is removed or its oracle weakened, the claim drops to ⚠.
