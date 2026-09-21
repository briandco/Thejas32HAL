"""
SVD file generator for CDAC VEGA THEJAS32 SoC.

Parses the register structures from bsp/include/ headers and produces
an SVD (System View Description) XML file suitable for svd2rust.

Usage:
    uv run main.py                          # writes thejas32.svd
    uv run main.py -o custom_name.svd       # writes custom_name.svd
"""

from __future__ import annotations

import argparse
import sys
import xml.etree.ElementTree as ET
from xml.dom import minidom
from pathlib import Path

from xmlschema import XMLSchema, XMLSchemaValidationError

# The CMSIS-SVD schema is vendored alongside this script as `cmsis-svd.xsd`
# and loaded from disk by validate_against_cmsis() — this script never touches
# the network.
#
# Origin (for reference / future re-vendoring only, not fetched at runtime):
#   https://github.com/cmsis-svd/svd-schema/blob/main/schemas/CMSIS-SVD_1_3_12.xsd
#
# The device's `schemaVersion` attribute below is the coarser CMSIS-SVD
# version "1.3"; the .12 suffix is the editorial revision of the XSD file.


# ---------------------------------------------------------------------------
# Peripheral definitions derived from the C headers
# ---------------------------------------------------------------------------


def build_device() -> ET.Element:
    device = ET.Element("device")
    device.set("schemaVersion", "1.3")
    device.set("xmlns:xs", "http://www.w3.org/2001/XMLSchema-instance")
    device.set("xs:noNamespaceSchemaLocation", "CMSIS-SVD.xsd")

    _t(device, "vendor", "CDAC")
    _t(device, "name", "THEJAS32")
    _t(device, "series", "VEGA")
    _t(device, "version", "1.0")
    _t(device, "description", "CDAC VEGA THEJAS32 RISC-V SoC")
    _t(device, "licenseText", "MIT License")

    # CMSIS-SVD `deviceType` sequence: <cpu> precedes <addressUnitBits>/<width>
    # and the device-level registerPropertiesGroup (<size>, <access>, …).
    cpu = ET.SubElement(device, "cpu")
    _t(cpu, "name", "other")
    _t(cpu, "revision", "r0p0")
    _t(cpu, "endian", "little")
    _t(cpu, "mpuPresent", "false")
    _t(cpu, "fpuPresent", "false")
    # `nvicPresent` is not a CMSIS-SVD element (Cortex-M-only NVIC has no
    # equivalent on this RISC-V SoC; interrupts are routed via the PLIC,
    # described in the <peripherals> section). The schema requires
    # nvicPrioBits + vendorSystickConfig regardless.
    _t(cpu, "nvicPrioBits", "0")
    _t(cpu, "vendorSystickConfig", "false")

    _t(device, "addressUnitBits", "8")
    _t(device, "width", "32")

    # Default register properties (registerPropertiesGroup: size, access,
    # protection?, resetValue, resetMask).
    _t(device, "size", "32")
    _t(device, "access", "read-write")
    _t(device, "resetValue", "0x00000000")
    _t(device, "resetMask", "0xFFFFFFFF")

    peripherals = ET.SubElement(device, "peripherals")

    # ---- UART (3 instances: UART_BASE + 0x100*i) ----
    add_uart(peripherals, "UART0", 0x10000100, "UART channel 0", interrupts=[("UART0", 0)])
    add_uart(peripherals, "UART1", 0x10000200, "UART channel 1", derived_from="UART0", interrupts=[("UART1", 1)])
    add_uart(peripherals, "UART2", 0x10000300, "UART channel 2", derived_from="UART0", interrupts=[("UART2", 2)])

    # ---- SPI (4 instances) ----
    # SPI_BASE_ADDR(i): i=0,1 -> 0x10000600; i=2,3 -> 0x10200100
    # SPIreg(i) = SPI_BASE_ADDR(i) + 0x100 * (i % 2)
    add_spi(peripherals, "SPI0", 0x10000600, "SPI controller 0", interrupts=[("SPI0", 3)])
    add_spi(peripherals, "SPI1", 0x10000700, "SPI controller 1", derived_from="SPI0", interrupts=[("SPI1", 4)])
    add_spi(peripherals, "SPI2", 0x10200100, "SPI controller 2", derived_from="SPI0")
    add_spi(peripherals, "SPI3", 0x10200200, "SPI controller 3", derived_from="SPI0")

    # ---- I2C (3 instances: I2C_BASE + 0x100*i) ----
    add_i2c(peripherals, "I2C0", 0x10000800, "I2C controller 0", interrupts=[("I2C0", 5)])
    add_i2c(peripherals, "I2C1", 0x10000900, "I2C controller 1", derived_from="I2C0", interrupts=[("I2C1", 6)])
    # Instance 8, at I2C_BASE + 0x100*8 = 0x1000_1000. The vendor declares it as
    #   i2c.h:   #define I2C_ADC 8  // at address 0x1000_1000
    # and reaches it through the same `I2CReg(i)` macro, which has no bounds
    # check, so the standard I2C register map applies. `config.h` calls the same
    # address ADC_BASE_ADDRESS and `adc.h` declares a three-register ADC block
    # there — but `adc_analogRead` is declared and **never implemented anywhere
    # in the SDK**, whereas the I2C reading is exercised by
    # examples/i2c/industrial_robot_demo (`ads1015Begin(I2C_ADC)`).
    #
    # This resolves why the SoC package exposes no analog pins: the board's
    # A0–A3 are a TI ADS1015 12-bit I2C ADC at address 0x48 on this bus. It also
    # explains why `adc-selftest` found the block unresponsive — it was driving
    # an I2C controller with ADC register semantics.
    #
    # No PLIC line: interrupt.h maps only I2C0 (5) and I2C1 (6). Polled only.
    add_i2c(
        peripherals,
        "I2C2",
        0x10001000,
        "I2C controller 2 - bus for the board's onboard ADS1015 ADC",
        derived_from="I2C0",
    )

    # ---- Timer (3 instances: TIMER_BASE + n*20) ----
    add_timer(peripherals, "TIMER0", 0x10000A00, "Timer channel 0", interrupts=[("TIMER0", 7)])
    add_timer(
        peripherals, "TIMER1", 0x10000A14, "Timer channel 1", derived_from="TIMER0", interrupts=[("TIMER1", 8)]
    )
    add_timer(
        peripherals, "TIMER2", 0x10000A28, "Timer channel 2", derived_from="TIMER0", interrupts=[("TIMER2", 9)]
    )
    add_timer_global(
        peripherals,
        "TIMER_GLOBAL",
        0x10000A00,
        "Timer global interrupt status registers",
    )

    # ---- GPIO (2 banks, address-decoded bit-banding interface) ----
    # GPIO pins 0-12 each have their own IRQ (10-22 from interrupt.h)
    gpio_irqs = [(f"GPIO{i}", 10 + i) for i in range(13)]
    add_gpio(peripherals, "GPIO0", 0x10080000, "GPIO bank 0 (pins 0-15)", interrupts=gpio_irqs)
    add_gpio(peripherals, "GPIO1", 0x10180000, "GPIO bank 1 (pins 16-31)")

    # ---- ADC ----
    add_adc(peripherals, "ADC", 0x10001000, "4-channel ADC")

    # ---- PWM (8 channels, base + ch*0x10, plus global control at +0x80) ----
    add_pwm(peripherals, "PWM0", 0x10400000, "PWM channel 0")
    add_pwm(peripherals, "PWM1", 0x10400010, "PWM channel 1", derived_from="PWM0")
    add_pwm(peripherals, "PWM2", 0x10400020, "PWM channel 2", derived_from="PWM0")
    add_pwm(peripherals, "PWM3", 0x10400030, "PWM channel 3", derived_from="PWM0")
    add_pwm(peripherals, "PWM4", 0x10400040, "PWM channel 4", derived_from="PWM0")
    add_pwm(peripherals, "PWM5", 0x10400050, "PWM channel 5", derived_from="PWM0")
    add_pwm(peripherals, "PWM6", 0x10400060, "PWM channel 6", derived_from="PWM0")
    add_pwm(peripherals, "PWM7", 0x10400070, "PWM channel 7", derived_from="PWM0")
    add_pwm_global(peripherals, "PWM_GLOBAL", 0x10400080, "PWM global control")

    # ---- PLIC (interrupt controller) ----
    add_plic(peripherals, "PLIC", 0x20010000, "Platform-Level Interrupt Controller")

    return device


# ---------------------------------------------------------------------------
# Peripheral builders
# ---------------------------------------------------------------------------


def add_uart(
    parent: ET.Element, name: str, base: int, desc: str, derived_from: str | None = None,
    interrupts: list[tuple[str, int]] | None = None,
) -> None:
    """
    UART register map (from uart.h UART_REG_TYPE):
      0x00  UART_DR       - Data Register (also divisor latch low when LCR.DLAB=1)
      0x04  UART_IE       - Interrupt Enable (also divisor latch high when LCR.DLAB=1)
      0x08  UART_IIR_FCR  - Interrupt ID / FIFO Control
      0x0C  UART_LCR      - Line Control Register
      0x10  Dummy         - Reserved
      0x14  UART_LSR      - Line Status Register
    """
    p = _peripheral(parent, name, base, desc, derived_from, interrupts=interrupts)
    if derived_from:
        return

    regs = ET.SubElement(p, "registers")

    r = _register(regs, "RBR", 0x00, "Receiver Buffer Register", access="read-only")
    _field(r, "DATA", 0, 8, "Received data byte")

    r = _register(
        regs,
        "THR",
        0x00,
        "Transmit Holding Register",
        access="write-only",
        alternate_to="RBR",
        alternate_group="UART_DR",
    )
    _field(r, "DATA", 0, 8, "Transmit data byte")

    r = _register(
        regs,
        "DLL",
        0x00,
        "Divisor Latch Low Register",
        alternate_to="RBR",
        alternate_group="UART_DR",
    )
    _field(r, "DATA", 0, 8, "Divisor latch low byte when LCR.DLAB=1")

    r = _register(regs, "IER", 0x04, "Interrupt Enable Register")
    _field(r, "ERBFI", 0, 1, "Enable received data available interrupt")
    _field(r, "ETBEI", 1, 1, "Enable transmitter holding register empty interrupt")
    _field(r, "ELSI", 2, 1, "Enable receiver line status interrupt")
    _field(r, "EDSSI", 3, 1, "Enable modem status interrupt")

    r = _register(
        regs,
        "DLH",
        0x04,
        "Divisor Latch High Register",
        alternate_to="IER",
        alternate_group="UART_IE",
    )
    _field(r, "DATA", 0, 8, "Divisor latch high byte when LCR.DLAB=1")

    # Hardware-verified reset: 0x01 (no interrupt pending, IID=1)
    r = _register(
        regs,
        "IIR",
        0x08,
        "Interrupt Identification Register",
        access="read-only",
        reset_value=0x01,
    )
    _field(r, "IID", 0, 4, "Interrupt identification")
    _field(r, "FIFOEN", 6, 2, "FIFO enable status")

    r = _register(
        regs,
        "FCR",
        0x08,
        "FIFO Control Register",
        access="write-only",
        alternate_to="IIR",
        alternate_group="UART_IIR_FCR",
    )
    _field(r, "FIFO_EN", 0, 1, "FIFO enable")
    _field(r, "RX_FIFO_RST", 1, 1, "Receive FIFO reset")
    _field(r, "TX_FIFO_RST", 2, 1, "Transmit FIFO reset")

    r = _register(regs, "LCR", 0x0C, "Line Control Register")
    _field(r, "WLS", 0, 2, "Word length select (0=5bit,1=6bit,2=7bit,3=8bit)")
    _field(r, "STB", 2, 1, "Number of stop bits")
    _field(r, "PEN", 3, 1, "Parity enable")
    _field(r, "EPS", 4, 1, "Even parity select")
    _field(r, "STKP", 5, 1, "Stick parity")
    _field(r, "SBRK", 6, 1, "Set break")
    _field(r, "DLAB", 7, 1, "Divisor latch access bit")

    _register(regs, "RESERVED", 0x10, "Reserved", access="read-only")

    # Hardware-verified reset: 0x60 (THRE=1, TEMT=1: transmitter empty/idle)
    r = _register(
        regs, "LSR", 0x14, "Line Status Register", access="read-only", reset_value=0x60
    )
    _field(r, "DR", 0, 1, "Data ready")
    _field(r, "OE", 1, 1, "Overrun error")
    _field(r, "PE", 2, 1, "Parity error")
    _field(r, "FE", 3, 1, "Framing error")
    _field(r, "BI", 4, 1, "Break interrupt")
    _field(r, "THRE", 5, 1, "Transmitter holding register empty")
    _field(r, "TEMT", 6, 1, "Transmitter empty")
    _field(r, "RXFE", 7, 1, "Receiver FIFO error")


def add_spi(
    parent: ET.Element, name: str, base: int, desc: str, derived_from: str | None = None,
    interrupts: list[tuple[str, int]] | None = None,
) -> None:
    """
    SPI register map (from spi.h SPIregType):
      0x00  Control  (16-bit with bitfields)
      0x04  Status
      0x08  Baudrate
      0x0C  TxData
      0x10  RxData
    """
    p = _peripheral(parent, name, base, desc, derived_from, interrupts=interrupts)
    if derived_from:
        return

    regs = ET.SubElement(p, "registers")

    r = _register(regs, "CONTROL", 0x00, "SPI Control Register")
    _field(r, "PERIPH_CS", 0, 2, "Peripheral chip select")
    _field(r, "PERIPH", 2, 1, "Peripheral type (0=fixed, 1=variable)")
    _field(r, "DATA_ORDER", 3, 1, "Data order (0=MSB first, 1=LSB first)")
    _field(r, "MODE", 4, 2, "SPI mode (clock polarity/phase)")
    _field(r, "RX_TX_INTR", 6, 2, "RX/TX interrupt enable")
    _field(r, "CSAAT", 8, 1, "Chip select active after transfer")
    _field(r, "DBITS", 9, 4, "Data bits per transfer")

    # Hardware-verified reset: 0x80 (TX_HOLD_EMPTY=1: transmitter ready)
    r = _register(
        regs,
        "STATUS",
        0x04,
        "SPI Status Register",
        access="read-only",
        reset_value=0x00000080,
    )
    _field(r, "RX_INT_STATUS", 2, 1, "RX interrupt status")
    _field(r, "TX_INT_STATUS", 3, 1, "TX interrupt status")
    _field(r, "BUSY", 4, 1, "SPI busy")
    _field(r, "OVERRUN", 5, 1, "Overrun error")
    _field(r, "RX_COMPLETE", 6, 1, "Receive complete")
    _field(r, "TX_HOLD_EMPTY", 7, 1, "Transmit holding register empty")

    # Hardware-verified reset: 0x60 (default baudrate divisor = 96)
    _register(regs, "BAUDRATE", 0x08, "SPI Baud Rate Divisor", reset_value=0x00000060)

    r = _register(
        regs, "TXDATA", 0x0C, "SPI Transmit Data Register", access="write-only"
    )
    _field(r, "DATA", 0, 16, "Transmit data")

    r = _register(regs, "RXDATA", 0x10, "SPI Receive Data Register", access="read-only")
    _field(r, "DATA", 0, 16, "Received data")


def add_i2c(
    parent: ET.Element, name: str, base: int, desc: str, derived_from: str | None = None,
    interrupts: list[tuple[str, int]] | None = None,
) -> None:
    """
    I2C register map (from i2c.h I2C_REG_TYPE):
    The struct uses UC (unsigned char = 8-bit) fields, packed contiguously.
      0x00  I2C_CR     - Control Register
      0x01  I2C_SR0    - Status Register 0
      0x02  I2C_SR1    - Status Register 1
      0x03  I2C_IER    - Interrupt Enable Register
      0x04  I2C_TxFF   - Transmit FIFO
      0x05  I2C_RxFF   - Receive FIFO
      0x06  I2C_CHL    - Clock Low Period
      0x07  I2C_CHH    - Clock High Period
      0x08  I2C_CHHL   - Clock Low Period (extended)
      0x09  I2C_CHHH   - Clock High Period (extended)
      0x0A  I2C_TXCLR  - TX Clear
    """
    p = _peripheral(parent, name, base, desc, derived_from, interrupts=interrupts)
    if derived_from:
        return

    regs = ET.SubElement(p, "registers")

    # Hardware-verified reset values from Aries v3.0 probe
    _register(regs, "CR", 0x00, "Control Register", size=8)
    _register(
        regs,
        "SR0",
        0x01,
        "Status Register 0",
        size=8,
        access="read-only",
        reset_value=0xD8,
    )
    _register(regs, "SR1", 0x02, "Status Register 1", size=8, access="read-only")
    _register(regs, "IER", 0x03, "Interrupt Enable Register", size=8)
    _register(regs, "TXFF", 0x04, "Transmit FIFO", size=8, access="write-only")
    _register(regs, "RXFF", 0x05, "Receive FIFO", size=8, access="read-only")
    _register(regs, "CHL", 0x06, "Clock Low Period", size=8, reset_value=0x7D)
    _register(regs, "CHH", 0x07, "Clock High Period", size=8)
    _register(
        regs, "CHHL", 0x08, "Clock Low Period (extended)", size=8, reset_value=0x3E
    )
    _register(regs, "CHHH", 0x09, "Clock High Period (extended)", size=8)
    _register(regs, "TXCLR", 0x0A, "TX Clear", size=8, access="write-only")


def add_timer(
    parent: ET.Element, name: str, base: int, desc: str, derived_from: str | None = None,
    interrupts: list[tuple[str, int]] | None = None,
) -> None:
    """
    Timer register map (from timer.h TimerReg_type):
    Each timer is 5 x 32-bit = 20 bytes apart.
      0x00  LoadCount
      0x04  CurrentValue
      0x08  Control
      0x0C  EOI
      0x10  IntrStatus
    """
    p = _peripheral(parent, name, base, desc, derived_from, interrupts=interrupts)
    if derived_from:
        return

    regs = ET.SubElement(p, "registers")

    _register(regs, "LOAD_COUNT", 0x00, "Timer load count value")

    _register(regs, "CURRENT_VALUE", 0x04, "Timer current value", access="read-only")

    r = _register(regs, "CONTROL", 0x08, "Timer control register")
    _field(r, "ENABLE", 0, 1, "Timer enable")
    _field(r, "MODE", 1, 1, "Timer mode (0=free-running, 1=user-defined)")
    _field(r, "INTR_MASK", 2, 1, "Interrupt mask (0=unmasked, 1=masked)")

    _register(
        regs,
        "EOI",
        0x0C,
        "Timer end-of-interrupt (read clears interrupt)",
        access="read-only",
    )

    _register(regs, "INTR_STATUS", 0x10, "Timer interrupt status", access="read-only")


def add_timer_global(parent: ET.Element, name: str, base: int, desc: str) -> None:
    """
    Timer global register map (from timer.c):
        0xA8  TIMERS_RAW_INTR_STATUS - Global raw interrupt status register
    """
    p = _peripheral(parent, name, base, desc)
    regs = ET.SubElement(p, "registers")

    r = _register(
        regs,
        "RAW_INTR_STATUS",
        0xA8,
        "Global raw timer interrupt status register",
        access="read-only",
    )
    _field(r, "TIMER0", 0, 1, "Timer 0 raw interrupt status")
    _field(r, "TIMER1", 1, 1, "Timer 1 raw interrupt status")
    _field(r, "TIMER2", 2, 1, "Timer 2 raw interrupt status")


def add_gpio(parent: ET.Element, name: str, base: int, desc: str,
             interrupts: list[tuple[str, int]] | None = None) -> None:
    """
    GPIO register map (from gpio.c driver analysis):
    Uses address-decoded bit-banding for data access:
      Base + (pin_bit << 2)  = data for that pin (read/write)
      Base + 0x40000         = direction register (0=input, 1=output)

    We expose the direction register and a bulk data register.
    Individual pin access uses address bit-banding (base + (1<<pin)<<2).
    """
    p = _peripheral(parent, name, base, desc, interrupts=interrupts)

    regs = ET.SubElement(p, "registers")

    r = _register(
        regs,
        "DATA",
        0x00,
        "GPIO data register (bit-banded: access pin N via base + (1<<N)<<2)",
        size=16,
    )
    _field(r, "PIN", 0, 16, "Pin data value")

    r = _register(
        regs, "DATAOUT", 0x3FFFC, "GPIO data register (all 16 pins, bulk read/write)",
        size=16,
    )
    _field(r, "PINS", 0, 16, "All pin values")

    r = _register(regs, "DIR", 0x40000, "GPIO direction register (0=input, 1=output)",
                  size=16)
    _field(r, "PIN", 0, 16, "Direction bits for pins 0-15")


def add_adc(parent: ET.Element, name: str, base: int, desc: str) -> None:
    """
    ADC register map (from adc.h ADCReg_type):
      0x00  address_reg  - channel select
      0x04  status_reg   - conversion status
      0x08  data_reg     - conversion result
    """
    p = _peripheral(parent, name, base, desc)

    regs = ET.SubElement(p, "registers")

    r = _register(regs, "ADDRESS", 0x00, "ADC channel select register")
    _field(r, "CHANNEL", 0, 2, "ADC channel number (0-3)")

    r = _register(regs, "STATUS", 0x04, "ADC status register", access="read-only")
    _field(r, "DONE", 0, 1, "Conversion complete")

    r = _register(
        regs, "DATA", 0x08, "ADC conversion data register", access="read-only"
    )
    _field(r, "VALUE", 0, 12, "ADC conversion result")


def add_pwm(
    parent: ET.Element, name: str, base: int, desc: str, derived_from: str | None = None,
    interrupts: list[tuple[str, int]] | None = None,
) -> None:
    """
    PWM channel register map (from pwmc.h PWMCregType):
      0x00  Control (bitfield union)
      0x04  Status
      0x08  Period
      0x0C  On_Off
    """
    p = _peripheral(parent, name, base, desc, derived_from, interrupts=interrupts)
    if derived_from:
        return

    regs = ET.SubElement(p, "registers")

    r = _register(regs, "CONTROL", 0x00, "PWM channel control register")
    _field(r, "MODE", 0, 2, "Mode (0=idle, 1=one-shot, 2=continuous)")
    _field(r, "ALIGN", 2, 2, "Alignment control (0=left, 1=right)")
    _field(r, "IE", 4, 1, "Interrupt enable")
    _field(r, "OPC", 5, 1, "Output polarity control")
    _field(r, "REPEAT_COUNT", 6, 16, "Repeat count for one-shot mode")

    r = _register(
        regs, "STATUS", 0x04, "PWM channel status register", access="read-only"
    )
    _field(r, "ACTIVE", 0, 1, "Channel active")

    _register(regs, "PERIOD", 0x08, "PWM period value")
    _register(regs, "ON_OFF", 0x0C, "PWM on/off time value")


def add_pwm_global(parent: ET.Element, name: str, base: int, desc: str) -> None:
    """PWM global control register at PWMC_BASE + 0x80."""
    p = _peripheral(parent, name, base, desc)
    regs = ET.SubElement(p, "registers")

    r = _register(regs, "GCR", 0x00, "PWM Global Control Register")
    _field(r, "GPE", 0, 1, "Global PWM enable")
    _field(r, "GIE", 1, 1, "Global interrupt enable")


def add_plic(parent: ET.Element, name: str, base: int, desc: str) -> None:
    """
    PLIC register map (from interrupt.h INTR_REG, 32-bit variant):
      0x00  RAW_INTR      - Raw interrupt status
      0x04  dummy0        - Reserved
      0x08  INTR_EN       - Interrupt enable
      0x0C  dummy1        - Reserved
      0x10  INTR_STATUS   - Interrupt status (after masking)
      ...
      0x400 SW_INTR_EN    - Software interrupt enable
    """
    p = _peripheral(parent, name, base, desc)
    regs = ET.SubElement(p, "registers")

    r = _register(regs, "RAW_INTR", 0x00, "Raw interrupt status", access="read-only")
    _add_irq_fields(r)

    _register(regs, "RESERVED0", 0x04, "Reserved", access="read-only")

    r = _register(regs, "INTR_EN", 0x08, "Interrupt enable register")
    _add_irq_fields(r)

    _register(regs, "RESERVED1", 0x0C, "Reserved", access="read-only")

    r = _register(
        regs, "INTR_STATUS", 0x10, "Masked interrupt status", access="read-only"
    )
    _add_irq_fields(r)

    _register(regs, "SW_INTR_EN", 0x400, "Software interrupt enable")


def _add_irq_fields(r: ET.Element) -> None:
    """Add IRQ bit fields matching the THEJAS32 (32-bit) IRQ mapping from interrupt.h."""
    irqs = [
        (0, "UART0"),
        (1, "UART1"),
        (2, "UART2"),
        (3, "SPI0"),
        (4, "SPI1"),
        (5, "I2C0"),
        (6, "I2C1"),
        (7, "TIMER0"),
        (8, "TIMER1"),
        (9, "TIMER2"),
    ]
    for i in range(13):
        irqs.append((10 + i, f"GPIO{i}"))

    for bit, irq_name in irqs:
        _field(r, irq_name, bit, 1, f"{irq_name} interrupt")


# ---------------------------------------------------------------------------
# SVD XML helpers
# ---------------------------------------------------------------------------


def _t(parent: ET.Element, tag: str, text: str) -> ET.Element:
    el = ET.SubElement(parent, tag)
    el.text = text
    return el


def _peripheral(
    parent: ET.Element,
    name: str,
    base: int,
    desc: str,
    derived_from: str | None = None,
    interrupts: list[tuple[str, int]] | None = None,
) -> ET.Element:
    p = ET.SubElement(parent, "peripheral")
    if derived_from:
        p.set("derivedFrom", derived_from)
    _t(p, "name", name)
    _t(p, "description", desc)
    _t(p, "baseAddress", f"0x{base:08X}")
    ab = ET.SubElement(p, "addressBlock")
    _t(ab, "offset", "0x0")
    # Use a size large enough to cover the register space
    if "GPIO" in name:
        _t(ab, "size", "0x50000")
    elif name == "PLIC":
        _t(ab, "size", "0x800")
    else:
        _t(ab, "size", "0x100")
    _t(ab, "usage", "registers")
    if interrupts:
        for irq_name, irq_value in interrupts:
            intr = ET.SubElement(p, "interrupt")
            _t(intr, "name", irq_name)
            _t(intr, "value", str(irq_value))
    return p


def _register(
    parent: ET.Element,
    name: str,
    offset: int,
    desc: str,
    size: int = 32,
    access: str | None = None,
    alternate_to: str | None = None,
    alternate_group: str | None = None,
    reset_value: int = 0,
) -> ET.Element:
    r = ET.SubElement(parent, "register")
    # Child order is dictated by CMSIS-SVD `registerType`:
    #   name, displayName?, description?,
    #   (alternateGroup | alternateRegister)?,
    #   addressOffset,
    #   registerPropertiesGroup -> size, access, protection?, resetValue, resetMask
    _t(r, "name", name)
    _t(r, "description", desc)
    # alternateGroup / alternateRegister are an <xs:choice> in CMSIS-SVD 1.3
    # (at most one), and they precede addressOffset. When a caller supplies
    # both, prefer alternateRegister — it names the specific sibling that
    # aliases this address, which is the relationship svd2rust acts on.
    if alternate_to:
        _t(r, "alternateRegister", alternate_to)
    elif alternate_group:
        _t(r, "alternateGroup", alternate_group)
    _t(r, "addressOffset", f"0x{offset:X}")
    _t(r, "size", str(size))
    if access:
        _t(r, "access", access)
    if size == 8:
        _t(r, "resetValue", f"0x{reset_value & 0xFF:02X}")
    else:
        _t(r, "resetValue", f"0x{reset_value:08X}")
    return r


def _field(
    parent: ET.Element, name: str, bit_offset: int, bit_width: int, desc: str
) -> ET.Element:
    fields = parent.find("fields")
    if fields is None:
        fields = ET.SubElement(parent, "fields")
    f = ET.SubElement(fields, "field")
    _t(f, "name", name)
    _t(f, "description", desc)
    _t(f, "bitOffset", str(bit_offset))
    _t(f, "bitWidth", str(bit_width))
    return f


# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------


def prettify(elem: ET.Element) -> str:
    raw = ET.tostring(elem, encoding="unicode", xml_declaration=False)
    dom = minidom.parseString(raw)
    lines = dom.toprettyxml(indent="  ", encoding=None)
    # Remove extra blank lines minidom sometimes adds
    return "\n".join(line for line in lines.splitlines() if line.strip())


def validate_against_cmsis(svd_path: Path) -> None:
    """Validate the generated SVD against the vendored CMSIS-SVD 1.3.12 XSD.

    Raises XMLSchemaValidationError on any schema violation.
    """
    schema_path = Path(__file__).parent / "cmsis-svd.xsd"
    if not schema_path.exists():
        raise FileNotFoundError(f"CMSIS-SVD schema not found at {schema_path}")
    schema = XMLSchema(str(schema_path))
    schema.validate(str(svd_path))


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate SVD for THEJAS32 SoC")
    parser.add_argument(
        "-o",
        "--output",
        default="thejas32.svd",
        help="Output SVD file path (default: thejas32.svd)",
    )
    args = parser.parse_args()

    device = build_device()
    xml_str = prettify(device)

    out = Path(args.output)
    out.write_text(xml_str, encoding="utf-8")
    print(f"SVD written to {out} ({out.stat().st_size} bytes)")

    try:
        validate_against_cmsis(out)
    except XMLSchemaValidationError as exc:
        bad = out.with_suffix(out.suffix + ".invalid")
        out.rename(bad)
        print(
            "ERROR: generated SVD failed CMSIS-SVD 1.3 validation",
            file=sys.stderr,
        )
        print(f"  bad file preserved at: {bad}", file=sys.stderr)
        print(f"  schema error:\n{exc}", file=sys.stderr)
        sys.exit(2)
    print("SVD validated against CMSIS-SVD 1.3 schema (XSD revision 1.3.12)")


if __name__ == "__main__":
    main()
