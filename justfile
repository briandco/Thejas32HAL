TARGET  := "riscv32im-unknown-none-elf"
BIN_DIR := "target/" + TARGET + "/release"
PORT    := "/dev/ttyUSB0"
# CDAC-supplied QSPI flash writer, vendored from the vega docs repo
# (source/_static/files/flasher.bin, also at cdac-vega.gitlab.io/_static/files/flasher.bin)
FLASHER := "tools/flasher.bin"

# Common aliases
alias b    := build
alias be   := build-example
alias f    := flash
alias fp   := flash-persist
alias fr   := flash-run
alias o    := objdump
alias s    := size
alias up   := update-pac
alias dp   := doc-pac
alias c    := clean

default: help

# Show available recipes and usage info
help:
    @echo "THEHAS32 SoC Rust Ecosystem"
    @echo "==========================="
    @echo ""
    @echo ""
    @just --list --unsorted

# Build all examples
build:
    cargo build --release -p thejas32-examples

# Build a specific example
build-example name:
    cargo build --release -p thejas32-examples --bin {{name}}

# Flash an example into RAM over UART XMODEM (lost on power cycle). Quit 'just monitor' first.
flash name: (build-example name)
    riscv32-unknown-elf-objcopy -O binary {{BIN_DIR}}/{{name}} {{BIN_DIR}}/{{name}}.bin
    stty -F {{PORT}} 115200 cs8 -cstopb -parenb -crtscts raw -echo
    @echo ">>> BOOT SEL OPEN (J12 jumper OFF). Press RESET on the board, wait ~2s for the"
    @echo ">>> bootloader, then press ENTER here. (Serial output is not shown -- that is"
    @echo ">>> expected; use 'just monitor' if you want to watch it.)"
    @read -r _ < /dev/tty
    @timeout 1 cat {{PORT}} > /dev/null || true
    sx --xmodem {{BIN_DIR}}/{{name}}.bin < {{PORT}} > {{PORT}}
    printf '\r' > {{PORT}}
    @echo ">>> Running. 'just monitor' to watch the serial output."

# Watch the serial port (Ctrl-t q to quit -- quit it before flashing, it holds the port)
monitor:
    tio -b 115200 {{PORT}}

# Flash an example into QSPI flash so it survives power cycles. Quit 'just monitor' first.
flash-persist name: (build-example name)
    riscv32-unknown-elf-objcopy -O binary {{BIN_DIR}}/{{name}} {{BIN_DIR}}/{{name}}.bin
    stty -F {{PORT}} 115200 cs8 -cstopb -parenb -crtscts raw -echo
    @echo ">>> BOOT SEL OPEN (J12 jumper OFF). Press RESET on the board, wait ~2s for the"
    @echo ">>> bootloader, then press ENTER here. (Serial output is not shown -- that is"
    @echo ">>> expected; use 'just monitor' if you want to watch it.)"
    @read -r _ < /dev/tty
    @timeout 1 cat {{PORT}} > /dev/null || true
    @echo ">>> Sending flasher.bin (erases QSPI flash)..."
    sx --xmodem {{FLASHER}} < {{PORT}} > {{PORT}}
    printf '\r' > {{PORT}}
    @echo ">>> Waiting for flash erase, then sending {{name}}.bin..."
    @timeout 8 cat {{PORT}} > /dev/null || true
    sx --xmodem {{BIN_DIR}}/{{name}}.bin < {{PORT}} > {{PORT}}
    printf '\r' > {{PORT}}
    @echo ">>> Done. Now place the J12 (BOOT_SEL) jumper and reset to boot from flash."

# Convert a built example to a raw .bin image
bin name: (build-example name)
    riscv32-unknown-elf-objcopy -O binary target/{{TARGET}}/release/{{name}} target/{{TARGET}}/release/{{name}}.bin
    @echo "target/{{TARGET}}/release/{{name}}.bin"

# `flash` sends the image but shows no output; watching it needs `just monitor`
# in a second step. `flash-run` drives the same transfer through tools/flash.py
# and captures the self-test's output into this terminal, so a change can be
# re-verified on hardware in one command.

# Build, flash over XMODEM, and capture the output
flash-run name capture="10": (bin name)
    uv run tools/flash.py target/{{TARGET}}/release/{{name}}.bin --capture {{capture}}

# Races do not show up in a single run. The `mstatus.MPIE` bug passed three
# consecutive runs before failing three, and the SPI interrupt rate differed
# between two executions of identical code. Repeat runs catch those
# deliberately rather than by luck.

# Flash and capture N times in a row, to expose races
flash-repeat name runs="5" capture="5": (bin name)
    uv run tools/flash.py target/{{TARGET}}/release/{{name}}.bin --repeat {{runs}} --capture {{capture}}

# Show disassembly
objdump name: (build-example name)
    riscv32-unknown-elf-objdump -d {{BIN_DIR}}/{{name}}

# Show section sizes
size name: (build-example name)
    riscv32-unknown-elf-size {{BIN_DIR}}/{{name}}

# Regenerate PAC from SVD
update-pac:
    ./update-pac.sh

# Generate doc for PAC
doc-pac:
    cd thejas32-pac
    RUSTDOCFLAGS="--cfg docsrs --generate-link-to-definition -Z unstable-options" cargo +nightly doc --all-features
    cd ..

# Clean everything
clean:
    cargo clean
