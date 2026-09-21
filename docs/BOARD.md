# ARIES v3.0 board reference

Physical wiring for the C-DAC ARIES v3.0 (THEJAS32, LQFP128). Written because
"which pin is GPIO0?" came up three separate times during bring-up and the
answer lived only in scattered commit messages.

## Sources, and how far to trust each

| Source | Used for |
|---|---|
| [CIRCUITSTATE ARIES V3 pinout r0.2](https://www.circuitstate.com/pinouts/c-dac-aries-v3-thejas32-risc-v-development-board-pinout-diagram-and-reference/) | header layouts and signal names |
| ARIES v3.0 Product Reference Manual | package pins, connectors, LD1 |
| **Our own measurements** | marked ✅ below |

Anything marked ✅ was confirmed by a self-test that measured a real signal
through the pin. Everything else is documentation, and documentation about this
board has been wrong before — the vendor SDK's 40 MHz UART clock is off by 2.6×
and fails silently.

## Connector numbering

Each 2-row header numbers its **GPIO-side column with odd numbers** — 1, 3, 5, 7,
9, 11, … — so the *n*-th position down that column is pin `2n-1`.

This is not a guess: `PWM0` at position 3 is `J2_5` and `GP0` at position 6 is
`J2_11`, and both were confirmed by a working jumper. The same rule makes
`GPIO16` at position 6 of its header `J10_11`, which matches the ARIES manual.

## Headers

### J2 — the main GPIO header

| Outer column | | Inner column | Pin |
|---|---|---|---|
| GP10 | | PWM2 | 1 |
| GP9 | | PWM1 | 3 |
| GP8 | | **PWM0** | **5** ✅ |
| GP7 | | GP2 | 7 |
| GP6 | | GP1 | 9 |
| GP5 | | **GP0** | **11** ✅ |
| GP4 | | TX1 | 13 |
| GP3 | | RX1 | 15 |

### J10 — analog and I2C header

| Outer column | | Inner column | Pin |
|---|---|---|---|
| ADC0 | | GP21 | 1 |
| ADC1 | | GP20 | 3 |
| ADC2 | | GP19 | 5 |
| ADC3 | | GP18 | 7 |
| SDA0 | | GP17 | 9 |
| SCL0 | | **GP16** | **11** |

ADC0–ADC3 are **not** SoC pins. They are the inputs of an onboard TI ADS1015
I2C ADC; the THEJAS32 package has no analog pins at all. See the ADC section of
the README.

### J1 — SPI0 header

| Signal | Pin |
|---|---|
| SCL1 | 1 |
| SDA1 | 3 |
| GND | 7 (outer) |
| SCK0 | 9 |
| **CIPO0** (MISO) | **6** ✅ |
| **COPI0** (MOSI) | **7** ✅ |
| CS0 | 8 |

### Other signals

| Purpose | Signal | Package pin | Location |
|---|---|---:|---|
| Console + power | `UART_TX0`, `UART_RX0` | 108, 107 | J6, through the onboard CP2102N |
| Boot select | `BOOT_SEL` | 15 | J12 pin 1 — **leave open** for UART boot |
| Reset | `PUSH_RESETN` | 43 | RESET button via reset IC U8 |
| RGB LED green | GPIO22 | 122 | LD1 pin 4 |
| RGB LED blue | GPIO23 | 121 | LD1 pin 3 |
| RGB LED red | GPIO24 | 120 | LD1 pin 2 |

LD1 is driven **active-low** (drive the pin low to light it) per the vendor's
RGB example. We have not independently measured this.

GPIO22–24 are therefore **not free pins** — using them as general I/O drives the
onboard LED.

## Jumpers the self-tests need

Most self-tests need no wiring. Three do:

| Test | Jumper | Purpose |
|---|---|---|
| `spi-selftest`, `spi-intr-selftest` | **J1_7 ↔ J1_6** (COPI0 ↔ CIPO0) | SPI loopback |
| `pwm-selftest`, `gpio-intr-selftest` | **J2_5 → J2_11** (PWM0 → GP0) | a known-rate edge source on a GPIO input |
| *(external GPIO bundle)* `gpio_loopback` | **J10_11 → J2_11** (GP16 → GP0) | GPIO output-to-input loopback |

⚠️ The last two both land on **J2_11 (GP0)**. Only one may be fitted at a time —
two outputs driving one input makes the result meaningless.

Each of those tests checks for its own jumper first and reports "no signal"
rather than failing, so a missing wire cannot be mistaken for a broken driver.

## Boot and reset

- **J12 open** → boots from UART (XMODEM). **J12 shorted** → boots from SPI flash.
- The console is **115200 8N1**, configured by the ROM bootloader before it hands
  over. `Serial::new_preconfigured` adopts that rather than recomputing a divisor.
- After RESET the bootloader prints its banner and then emits `C` about every
  0.7 s, indefinitely, waiting for an XMODEM transfer.
- It waits for **ENTER after the transfer** before jumping to the image. See the
  README's flashing section — this is easy to miss when automating.

### Clock straps

The ARIES manual describes strapping GPIO31–GPIO25 to `0110110` on J9 to select
the 100 MHz case. **We have never set these**, and the board runs at 100 MHz
regardless — measured from the divisor the bootloader leaves in UART0's latch
(54 → `54 × 115200 × 16` ≈ 100 MHz, see `clock-probe`).

So the default strap state already gives 100 MHz on this board. Changing J9 to
match the manual would be changing a working configuration for no benefit.
