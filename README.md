# thejas32-rs

Bare-metal Rust ecosystem for the [CDAC VEGA THEJAS32](https://vegaprocessors.in/thejas32-soc.php) RISC-V SoC.

## Crates

| Crate | Description |
|---|---|
| `thejas32-pac` | Auto-generated Peripheral Access Crate (UART×3, SPI×2, I2C×2, Timer×3, GPIO×2, ADC, PWM×8, PLIC) |
| `thejas32-hal` | Hand-written HAL on top of the PAC, targeting `embedded-hal` 1.0 traits (UART, GPIO, SPI, Timer, I2C, PLIC) |
| `thejas32-pac-examples` | Bare-metal firmware examples |

### HAL status

| Peripheral | Driver | Hardware-validated |
|---|---|---|
| UART | `serial::Serial` — `embedded-hal-nb` `Read`/`Write` | ✅ `uart-echo` |
| GPIO | `gpio::Pin` — `embedded-hal` `OutputPin`/`InputPin`/`StatefulOutputPin`, type-state direction, per-pin PLIC interrupts | ✅ `gpio-selftest` (9/9); `gpio-intr-selftest` 4/4 — delivery, masking, re-enable, and trigger semantics determined (**level-sensitive, active high** — see below) |
| SPI | `spi::Spi` — `embedded-hal` `SpiBus<u8>`, 4 controllers, hardware chip select, RX/TX interrupt enables | ✅ `spi-selftest` — config 3/3, state machine 4/4, MOSI↔MISO loopback 5/5. ✅ `spi-flash-selftest` 6/6 — word size by timing on SPI0 and SPI3, and the boot flash read through `SpiBus` on SPI3 (JEDEC ID and data). See "SPI word size and chip select" below. ⚠️ `spi-intr-selftest` 4/4 on delivery/masking/re-enable, but the interrupt **rate** is not understood — see below |
| Timer | `timer::Timer` — `embedded-hal` `DelayNs`, 3 timers, periodic + free-running counter | ✅ `timer-selftest` 8/8 — delays within 0.03%, period within 0.006%, rate cross-checked against UART baud |
| PLIC | `plic::Plic` — enable/mask, raw vs active status, CSR helpers, software interrupts | ✅ `plic-selftest` 5/5 — SW interrupt, timer IRQ delivery rate within 0.002%, masking verified |
| PWM | `pwm::Pwm` — `embedded-hal` `SetDutyCycle`, 8 channels, global gate | ✅ `pwm-selftest` 4/4 — encoding 3/3, period/on-time, global gating, and with PWM[0]→GPIO0 jumpered the measured waveform is within **0.005%** on period and 0.012% on high time |
| ADC | `adc::Adc` — **deprecated, models a peripheral that does not exist** | ❌ `0x1000_1000` is an I2C controller, not an ADC (proven: a device ACKs at 0x48 there). Analog input is the board's TI ADS1015 over I2C2 — see `ads1015-selftest`, ✅ 4/4 |
| I2C | `i2c::I2c` — `embedded-hal` `I2c`, 3 controllers, bounded waits, TX/RX interrupt enables | ✅ `i2c-scan-all`, `ads1015-selftest` 4/4 — ACK/NACK discrimination, 3-byte write, 2-byte read, exact config echo against the board's ADS1015. `i2c-intr-selftest` 4/4 — **1,716 interrupts for 1,716 writes**, masking and re-enable verified. **No repeated START** |
| Async | `timer::AsyncTimer` — `embedded-hal-async` `DelayNs`, interrupt-driven timer future | ✅ `async-selftest` 3/3 — three repeated awaits all within 0.26%, identical to the tick; `delay_ms` via the ecosystem trait within 0.15% |

Physical wiring is in **[docs/BOARD.md](docs/BOARD.md)**; what each driver is
derived from, where it deviates, and what is measured rather than assumed is in
**[docs/EVIDENCE.md](docs/EVIDENCE.md)**.

Self-tests print results over UART0 at the bootloader's 115200 and need no external hardware, except two jumpers: **SPI0_MOSI ↔ SPI0_MISO** (`J1_7 ↔ J1_6`) for `spi-selftest`'s loopback phase, and **PWM[0] → GPIO0** (`J2_5 → J2_11`) for `pwm-selftest`'s waveform phase. Without the jumper it still validates configuration and the transfer state machine, and reports the data path as unproven rather than passing it.

### Write ordering — fences are required

THEJAS32's peripheral bus can apply writes to different registers of the same peripheral **out of order**, even though the core is in-order and accesses are `volatile` (which constrains only the compiler). Use `thejas32_hal::fence()` between a configuration write and the write that acts on it.

This is not theoretical: arming a timer with `load_count = N` immediately followed by `control = enable` leaves the counter free-running from `0xFFFF_FFFF`, ignoring the load — while `load_count` still *reads back correctly*. Adding a fence between those two writes, changing nothing else, fixes it. CDAC's `vega-sdk` fences every device write for this reason; that is not legacy caution.

### Timer: the polled flag and the interrupt run at different rates

With a load count of N the **interrupt** fires every N clocks, but the **polled** expiry flag appears only every **2N**. The interrupt asserts on every underflow; the status flag surfaces every second one. The counter itself runs at full rate in both cases — this is flag/interrupt timing, not a slow counter.

Both were measured: the polled 2N ratio at N = 1k/10k/100k/200k (all within 0.1% of 200%), and the interrupt rate as 100 interrupts in 5,000,172 clocks at a load of 50,000 (≈50,002 each).

Consequently `Timer::start_periodic` programs half the requested period and `Timer::start_periodic_interrupt` programs it whole. **Use one path per timer** — a timer armed for polling and then unmasked will interrupt at twice its polled period.

### Timer interrupts need `listen()` — the mask gates the PLIC

Arming leaves the timer's interrupt masked, which gates its output to the interrupt controller entirely. Polling works perfectly while interrupts silently never arrive. `Timer::start_periodic_interrupt` unmasks automatically; after a plain `start_periodic`, call `Timer::listen()`.

### GPIO interrupts are level-sensitive, active high — and cannot be acknowledged

Bank 0 pins 0–12 each own a PLIC line (10–22), and that is the **only** control:
the block has no interrupt enable, no edge select, and no status register.
`vega-sdk`'s two GPIO interrupt examples confirm this — they configure the pin
as an input and then do nothing but enable the PLIC line.

The trigger condition is undocumented, so it was measured. Holding the pin at a
**static** level — where no edges exist at all — a high input produces
interrupts continuously and a low input produces none:

```
pin held HIGH -> interrupts: 4992      (capped by the test's storm guard)
pin held LOW  -> interrupts: 0
verdict: LEVEL-sensitive, active HIGH
```

The level is set and allowed to settle *before* the PLIC line is enabled, so the
transition's own edge is never counted; otherwise an edge-triggered source would
look level-sensitive. See `gpio-intr-selftest`.

Two consequences, both of which bite:

**A handler must mask its own source.** With no status bit to clear, the
interrupt re-asserts the instant the handler returns while the pin stays high. A
handler that merely counts will livelock the core — during bring-up this
produced 5,000 entries before the test's own guard masked the line. Disable the
pin's PLIC line inside the handler and re-enable it once the condition passes.

**Edges cannot be counted.** A pulse train yields as many interrupts as the core
can take while the level is high, not one per pulse. These are for "something is
asserted" conditions — a button held, a sensor output — not for measuring
frequency. Use a timer capture for that.

### SPI word size and chip select

Two driver defaults were wrong and are fixed; both were invisible to a
loopback test and caught by a real device, the boot flash on SPI3.

- **Word size.** A transfer is `8 + DBITS` bits, at most 16. Measured by
  timing transfers at two clock dividers (`spi-flash-selftest`): DBITS 0, 1,
  4, 7, 8 took 10, 11, 14, 17, 18 bit times (2 are fixed controller
  overhead), and 12 and 15 also 18. Identical on SPI0 and SPI3. The HAL
  used to write DBITS = 8, as `vega-sdk`'s `SPI_init` does, which sends
  **16-bit** words: the flash then sees `0x00 0x9F` instead of `0x9F`, and
  every read returned `00`. `Config::bits_per_transfer` is now the real bit
  count (8..=16, default 8) and the driver writes `bits - 8`.
- **Chip select.** The `SpiBus` methods used to clear CSAAT *before* the
  last byte, following the vendor driver's comment. With the flash that
  lost the last byte (ID `1f 86 00`). They now hold CSAAT through the last
  byte and clear it after it completes, as CDAC's `flasher.bin` does, and
  also on error. `set_csaat` now fences, so the data write after it cannot
  overtake it.

Before the fix `spi-flash-selftest` read the ID as `00 00 00` (1/4 checks);
after it, `1f 86 01` and rustBoot's first instruction from flash `0x0` (6/6).

### SPI: only one writer of the control register

`SpiBus` read-modify-writes the control register **during** a transfer —
`framed()` calls `set_csaat` before every byte — so an interrupt handler that
also modifies it loses updates. The two writers interleave and one of them wins.

The symptom is not a clean failure. With the handler masking itself through
`control().modify()`, one phase delivered **1 interrupt across 51,021
transfers** while another **stormed into its 2,000-entry backstop**, from
identical code. Which way it goes depends on where the interrupt lands inside
the read-modify-write:

* the handler's mask is overwritten by the application's stale copy → storm;
* the application's `CSAAT` update is overwritten → chip select misbehaves and
  delivery stalls.

`spi-intr-diag2`, whose `main` touches the control register only *before* each
transfer and drives the transfer at register level, is a clean twelve-for-twelve
with one interrupt per transfer, `MIE` steady and no storm. That is the same
peripheral and the same handler logic — only the overlap removed.

**Rule:** the application owns the control register. Arm before a transfer,
disarm after, and let the handler touch nothing but its own state.
`Spi::listen`/`unlisten` must never be called from an interrupt handler.

### I2C interrupts work cleanly — and why SPI's do not

Worth stating together, because the contrast is the useful part. Both
peripherals have PLIC lines and interrupt-enable bits; one is straightforward to
use from an interrupt handler and the other is not, for reasons visible in the
register design:

| | SPI | I2C |
|---|---|---|
| Status register | **read-to-clear** — observing it consumes the flag | reads freely; verified stable across back-to-back reads |
| Interrupt enable | shares the **control** register, which the data path read-modify-writes before every byte | separate `IER`, which the data path never touches |
| Data path writes | `control().modify()` — read-modify-write | `cr`, `txff`, `txclr` written **whole** |
| Result | handler and transfer fight over both; unusable with `SpiBus` | handler owns `IER`, shares nothing |

Measured: `i2c-intr-selftest` delivers **1,716 interrupts for 1,716 writes** — a
clean 1:1 — with masking and re-enabling both behaving. The equivalent SPI
arrangement produced 1, then 0, then a storm, depending on where the interrupt
landed inside a read-modify-write.

The lesson generalises beyond this chip: a peripheral is usable from an
interrupt handler when the handler and the polled path can own **disjoint**
registers. Where an enable bit shares a register the data path rewrites, or a
status flag is destroyed by reading it, the two designs cannot coexist however
the driver is arranged.

Stimulus note: I2C0 and I2C1 are empty on this board, so the self-test uses a
write to an unanswered address — START and the address byte are still clocked
out, so a transmit event occurs regardless of the NACK. The ADS1015 cannot serve
here: it is on I2C2, which has **no PLIC line**.

### The SPI status register is read-to-clear

Its receive flags — bit 2 `RX_INT_STATUS` and bit 6 `RX_COMPLETE` — are cleared
by the act of reading the register. Measured with `spi-intr-diag`: after a
transfer, three consecutive reads with nothing in between give

```
read 1: 0x00c4 [rxint=1 rxdone=1]
read 2: 0x0080 [rxint=0 rxdone=0]
read 3: 0x0080 [rxint=0 rxdone=0]
byte still in RxData: 0xa5
```

The flags survive exactly one observation, and the received byte is still
sitting in `RxData` — so this is the *status read* clearing them, not data
consumption. Bit 7 persists across reads and `busy` is live state, so this
applies to the latched receive flags rather than the whole register.

**Only one reader may exist.** `Spi::transfer_byte` polls `rx_complete` and then
reads the byte. An interrupt handler that also reads the status register
consumes that flag, and the polled transfer spins forever. This deadlocked
`spi-intr-selftest` twice during bring-up — the first fix removed an `RxData`
read from the handler and did not help, because the status read was the real
cause.

So a handler must either own the whole receive path (read status, read `RxData`,
and never let the application call the blocking transfers on that controller),
or touch neither and simply mask itself at the peripheral — a *control* register
write, which is unaffected. With one event enabled the cause is already known,
so there is nothing to inspect.

`Spi::take_interrupt_status` is named for this: the read is destructive, and the
name says so at the call site.

### I2C has no repeated START

A START is only accepted once the bus has been released. `I2c::transaction`
therefore issues a STOP after **every** operation, not merely the last, which is
a documented deviation from `embedded-hal` — that trait specifies repeated
STARTs between the operations of a transaction.

Measured with the board's ADS1015 on the bus. Driving the registers by hand,
`write_to` (which always ends in STOP) followed by a fresh START reads correctly
and returned `0x0AC0`; every step of the sequence passes. The identical sequence
composed through `transaction`, whose only difference was the missing STOP
between the write and the read, failed *every* read. See `i2c-read-diag`.

Devices needing a true repeated START to hold a register pointer across the
turnaround will not work on this part. Most, including the ADS1015, accept
either form.

### The vendor I2C read function discards its data

`vega-sdk`'s `i2c_data_read` advances its output pointer in the receive loop and
never reads `I2C_RxFF`, so the caller's buffer is left untouched:

```c
for (int i = 0; i < read_length; i++) {
    while ((I2CReg(i2c_number).I2C_SR0 & 0x40) == 0x40) ;  // wait if RXfifo empty
    read_data++;      // increments the pointer -- never reads the FIFO
}
```

It remains a correct reference for the *status-flag sequence*, which is how the
register order here was derived, but not for the data path.

### Never take a critical section inside an interrupt handler

`critical_section::acquire` emits `csrrci mstatus, 8`. On this core that
instruction **also clears `mstatus.MPIE`** when the trap was taken at the moment
another critical section released. `mret` then restores `MIE` from a zeroed
`MPIE`, so interrupts are disabled permanently: the first interrupt is serviced
correctly and no later one is ever delivered. Architecturally `csrrci` with
`uimm=8` may write bit 3 only, so this is a deviation from the RISC-V privileged
specification — the same class of surprise as this part's writable `mip.MSIP`.

Measured by `async-diag8`, which reads raw `mstatus` between each statement of a
handler:

```
on entry          mstatus=00001880  MIE=0  MPIE=1
after empty CS    mstatus=00001800  MIE=0  MPIE=0
```

The instruction sequence is byte-identical to a handler that does *not* show the
fault, so this is a **race**, not a deterministic bug — and that is what makes it
dangerous. A handler taking a critical section works until an interrupt happens
to arrive while the main program is inside one. During bring-up the same binary
passed three runs, then failed three runs, with no source change.

Handlers already run with `MIE` clear and this is a single-hart part, so an ISR
holds exclusive access without acquiring anything. Either mint a token:

```rust
// SAFETY: MIE is clear on trap entry, single-hart, no nesting.
let cs = unsafe { critical_section::CriticalSection::new() };
```

as `timer::on_interrupt` does, or use plain atomics — riscv32im has load/store
atomics and lacks only CAS.

### ADC: there is no on-chip ADC — analog input is an I2C chip

Two independent sources agree, and hardware settles it.

The [ARIES v3.0 Product Reference Manual](https://vegaprocessors.in/files/ARIESv3%200_Datasheet-4.pdf)
shows the board's four analog inputs do not come from the THEJAS32 at all:

| Signal | THEJAS32 pin | ARIES board | Remark (datasheet §5) |
|---|---|---|---|
| ADC_CH0–CH3 | *(none)* | J10_2/4/6/8, U9_4–U9_7 | "ANALOG INPUTS Connected to HEADER J10 & **I2C ADC IC**" |
| SCL_2 / SDA_2 | pins 32 / 31 | U9_10 / U9_9 | "Connected to **I2C ADC IC**" |

The SoC's full 128-pin table lists **no analog pins at all**.

The vendor SDK then describes `0x1000_1000` two contradictory ways:

```c
config.h:  #define ADC_BASE_ADDRESS  0x10001000UL   // adc.h declares an ADC block here
i2c.h:     #define I2C_ADC  8   // at address 0x1000_1000
```

`I2C_BASE_ADDR + 0x100*8` is exactly `0x1000_1000`, reached through the generic
`I2CReg(i)` macro, which has no bounds check. Only the I2C reading is backed by
working code — **`adc_analogRead` is declared and never implemented anywhere in
the SDK**, while `ads1015Begin(I2C_ADC)` is exercised by the vendor's own
`industrial_robot_demo`.

Hardware settles it: a device acknowledges at **0x48** on that bus, its
configuration register echoes written values exactly, and conversions read back
in range. A register block that were an ADC could not implement the I2C state
machine.

So `0x1000_1000` is the SoC's **third I2C controller**, and U9 is a TI **ADS1015**
12-bit ADC on it. `thejas32_hal::adc` models a peripheral that does not exist and
is deprecated; `adc-diag` saw "writes land, status never sets, data frozen at
`0x3e`" because it was driving an I2C controller with ADC register semantics.

**To read analog inputs on ARIES V3.0**, use `thejas32_pac::I2c2` with the
ADS1015 driver in the examples crate — `ads1015-selftest` passes 4/4 and reports
all four channels.

### GPIO: pins 22–24 drive the onboard RGB LED

`GPIO22` (green), `GPIO23` (blue), `GPIO24` (red) are wired to LD1 — that is **bank 1, pins 6, 7 and 8**. Anything driving those pins changes the onboard LED, and they do not float freely.

This explains the constant `0x140` background in `gpio-selftest`: bits 6 and 8 are the green and red LED pins reading high at reset.

### Peripheral clock

**100 MHz** (`thejas32_hal::PERIPHERAL_CLOCK_HZ`), measured on hardware rather than assumed: the ROM bootloader leaves a divisor of 54 in UART0's latch for its working 115200 console, and `54 × 115200 × 16 = 99_532_800 Hz`. Reproduce with the `clock-probe` example.

⚠️ CDAC's `vega-sdk` examples pass `40000000` as the UART clock. **That value is wrong for this board** — it produces divisor 21 and a line rate ~2.6× too fast. The failure is silent: the firmware compiles, links, flashes, boots, and emits garbage rather than reporting an error. Always use `PERIPHERAL_CLOCK_HZ`, or side-step divisors entirely with `Serial::new_preconfigured`.

## Flashing and capturing test output

`just flash-run <example>` builds, converts to a raw image, uploads it over the
ROM bootloader's XMODEM, and prints whatever the program sends back — no
separate terminal needed:

```bash
just flash-run gpio-selftest        # build, flash, capture 10s of output
just flash-run uart-echo 5          # shorter capture window
just flash-repeat async-selftest 5  # flash and capture 5 times in a row
just bin <example>                  # just produce the .bin
```

Press RESET when it prompts — **after** starting the command, not before. A
board running a previously flashed image is silent (many of these print
nothing), so the bootloader has to be reached by a button press while the tool
is listening.

`flash-repeat` is the one worth reaching for on anything interrupt-driven. Races
do not show up in a single run: the `mstatus.MPIE` corruption passed three
consecutive runs before failing three, and the SPI interrupt rate differed
between two executions of *identical code*. Both cost several manual flashes to
notice.

Interactive tests need a live terminal — `uart-intr-selftest` waits for
keypresses. Use `just flash` to load it, then `just monitor` to talk to it.

### RAM or flash

`just flash` loads the image into RAM over XMODEM; it is lost on power cycle.
`just flash-persist` writes it to the on-board QSPI flash instead, so it survives:

```bash
just flash led-blink          # load into RAM (lost on power cycle)
just flash-persist led-blink  # write to QSPI flash
just monitor                  # watch the serial port
```

`flash-persist` first uploads `tools/flasher.bin` — C-DAC's QSPI flash writer,
vendored unmodified from the vega documentation repository and not built from
source here — which erases the flash, then sends the image. Afterwards, fit the
**J12 (BOOT_SEL)** jumper and reset to boot from flash.

`just monitor` holds the serial port open. Quit it (`Ctrl-t q`) before flashing,
or the transfer cannot reach the bootloader.

### Two details of the bootloader protocol

Both were found by automating this, and neither is obvious from a manual session:

- **It waits for ENTER after the transfer.** The prompt says "send file using
  XMODEM and then press ENTER key" and means it literally: without a trailing
  newline the upload reports success and the board sits in the bootloader
  forever. A human supplies that keystroke without thinking about it.
- **The handshake is a lone `C`, not any `C`.** The boot banner contains `C` in
  "C-DAC", "CPU" and "Compiled", so matching a bare `C` fires while the banner is
  still arriving and the transfer starts early — XMODEM then reads banner text as
  protocol responses. A read containing *nothing but* `C` is the real signal.

## Continuous integration

`.github/workflows/ci.yml` runs on every push and pull request. It cannot prove a
driver works — there is no simulator for this SoC, and every behavioural claim
below came from a self-test on real hardware — but it keeps a hardware session
from being spent on a build error:

| Gate | What it catches |
|---|---|
| `cargo fmt --all --check` | formatting drift |
| `cargo clippy --workspace -- -D warnings` | lints; the workspace is clippy-clean |
| Build HAL + all 40 examples | a HAL change that breaks any caller |
| `cargo doc` with `RUSTDOCFLAGS=-D warnings` | broken intra-doc links |
| Regenerate the PAC and diff | hand-edits to generated code, which the next `just update-pac` would silently undo |

CI uses Rust **1.88**, the HAL's documented minimum, pinned so that a lint
added in a later release cannot fail CI without a change here. All five gates
were run locally with 1.88 before the workflow was added.

There are no host unit tests, and they are not reachable without restructuring:
the HAL uses `asm!("fence")` and other RISC-V-only intrinsics, so the crate
cannot compile for the host at all. Testing the pure arithmetic would mean
extracting it into a dependency-free crate first.

## Prerequisites

- Rust toolchain with `riscv32im-unknown-none-elf` target
  ```bash
  rustup target add riscv32im-unknown-none-elf
  ```
- [just](https://github.com/casey/just) — task runner
- [svd2rust](https://github.com/rust-embedded/svd2rust) and [form](https://github.com/djmcgill/form) — PAC generation
  ```bash
  cargo install svd2rust form
  ```
- [uv](https://github.com/astral-sh/uv) — Python package manager (for the SVD generator)
- RISC-V GNU toolchain (`riscv32-unknown-elf-objcopy`, `objdump`, `size`) — binary inspection and flashing
- **lrzsz** (`sx`) — XMODEM upload to the bootloader, used by `just flash` and `just flash-persist`
- **tio** — serial monitor, used by `just monitor`

## Quick Start

```bash
# Build all examples
just build

# Build and flash a specific example into RAM
just flash led-blink

# ...or into QSPI flash, so it survives a power cycle
just flash-persist led-blink

# Watch its serial output
just monitor

# Show disassembly
just objdump led-blink

# Show section sizes
just size led-blink
```

## PAC Regeneration

The PAC source in `thejas32-pac/src/` is **auto-generated** — do not edit it directly. The source of truth is the SVD generator in `tools/thejas32-svd/main.py`.

```bash
just update-pac
```

This runs the full pipeline: Python SVD generator → `svd2rust` → `form` → `rustfmt`.

The generated SVD is validated against the vendored CMSIS-SVD 1.3 schema
(`tools/thejas32-svd/cmsis-svd.xsd`, file revision 1.3.12), and the pipeline
aborts on any violation. When it does, fix `main.py` — never relax the validator.

Firmware startup is provided by `riscv-rt`. The linker receives `memory.x`, from
the examples crate, for the RAM layout, and `device.x`, which the PAC's build
script emits when its `rt` feature is enabled, for the interrupt vector symbols.

## Memory Map

Measured on hardware. The program used to probe it is not included in this
repository.

| Region | Address | Size | Notes |
|---|---|---|---|
| Boot ROM | `0x01_0000`–`0x01_FFFF` | **8 KB**, mirrored 8× across a 64 KB window | Vendor bootloader |
| IRAM | `0x20_0000`–`0x23_FFFF` | 256 KB | The bootloader keeps the top 6 KB (`0x23_E800` up) for its own stack while loading — hence its "250 KB" banner. That RAM is real and writable once your program runs. |
| SPI flash | not CPU-addressable | 2 MB | AT25SF161 (JEDEC `1f:86:01`) on **SPI3** at `0x1020_0200`, chip select 0 (`spi-flash-selftest`; SPI2 at `0x1020_0100` reads nothing). With J12 fitted the ROM copies 250 KB from flash `0x0` to IRAM. `flasher.bin` erases only `0x00_0000`–`0x03_FFFF` |

`memory.x` gives the linker **250 KB**, not 256 KB. Because the bootloader is
still using the top 6 KB while it loads the image, a 256 KB region would place
`_stack_start` inside memory the bootloader is standing on.

ET1031 does not fault on unmapped reads — they return zeros. Probe memory by
content, not by waiting for a trap.

Key peripherals: UART @ `0x1000_0100`, SPI0/1 @ `0x1000_0600`/`0x1000_0700`, SPI2/3 @ `0x1020_0100`/`0x1020_0200` (boot flash on SPI3), I2C @ `0x1000_0800`, Timer @ `0x1000_0A00`, GPIO0 @ `0x1008_0000`, GPIO1 @ `0x1018_0000`, ADC @ `0x1000_1000`, PWM @ `0x1040_0000`, PLIC @ `0x2001_0000`.

## License

Apache-2.0
