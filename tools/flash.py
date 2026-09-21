# /// script
# requires-python = ">=3.10"
# dependencies = ["pyserial>=3.5,<4", "xmodem>=0.4.7,<0.5"]
# ///
"""Flash a THEJAS32 image over the ROM bootloader's XMODEM, then capture output.

Replaces the interactive `minicom aries` loop. The point is not convenience: a
self-test whose result arrives in the terminal can be re-run after every change,
and a test that can be repeated ten times in a row catches races that a single
manual flash cannot. Two of this project's hardest bugs -- the `mstatus.MPIE`
corruption and the SPI interrupt-rate inconsistency -- were races that presented
differently between runs of the *same binary*.

Dependencies are declared inline (PEP 723), so `uv run` fetches them; nothing
needs installing first.

Usage:

    uv run tools/flash.py path/to/image.bin
    uv run tools/flash.py image.bin --capture 20
    uv run tools/flash.py image.bin --repeat 5 --capture 3

The board must be sitting in the ROM bootloader, which announces itself by
sending 'C' characters continuously. If it is running a previously flashed
program instead, press RESET when prompted.
"""

from __future__ import annotations

import argparse
import hashlib
import sys
import time
from pathlib import Path

import serial
from xmodem import XMODEM

# The ROM bootloader emits this repeatedly while waiting for an XMODEM transfer:
# the XMODEM-CRC handshake character, roughly every 0.7 s, indefinitely.
#
# Matching a bare 'C' anywhere in the stream is NOT sufficient, and getting this
# wrong is what broke the first working version. The boot banner contains "C" in
# "C-DAC", "CPU" and "Compiled", so a naive search fires while the banner is
# still arriving; the transfer then starts early and XMODEM reads the remaining
# banner text ('D', '-', ...) as protocol responses.
#
# The handshake is distinguishable because it arrives *alone*: a read that
# returns nothing but 'C' characters. No part of the banner does that.
BOOTLOADER_READY = b"C"

# Consecutive handshake-only reads required before starting. Two costs about a
# second and rules out a read that happens to land on a single banner 'C'.
HANDSHAKE_READS = 2

# Printed by the bootloader once the transfer is accepted and it jumps to the
# image. Everything after this line is the program's own output.
PROGRAM_BANNER = "Starting program"

# The bootloader's prompt reads "send file using XMODEM and then press ENTER
# key", and it means it: after a successful transfer it waits for a newline
# before jumping to the image. In minicom a human supplies that keystroke, which
# is why it is easy to miss when automating -- the upload reports success and
# the board simply sits there.
START_PROGRAM = b"\r"


def wait_for_bootloader(connection: serial.Serial, timeout: float) -> bool:
    """Waits for the bootloader's 'C' handshake, prompting for RESET if needed.

    Returns True once the board is ready. A board running a previously flashed
    program stays silent here, which is why the prompt appears rather than a
    bare timeout: the fix is a button press, not a longer wait.
    """
    # Prompt immediately rather than after a delay. The board only reaches the
    # bootloader by a button press, and a program already running produces no
    # output at all -- so silence here is the *normal* starting state, not a
    # symptom. Measured: the bootloader then sends a 'C' about every 0.7 s and
    # waits indefinitely, so there is no hurry once it is reached.
    print(f"  >>> press RESET on the board (waiting up to {timeout:.0f}s) <<<", flush=True)
    deadline = time.monotonic() + timeout
    clean_reads = 0
    while time.monotonic() < deadline:
        chunk = (connection.read(256) or b"").strip()
        if not chunk:
            continue
        if set(chunk) <= set(BOOTLOADER_READY):
            clean_reads += 1
            if clean_reads >= HANDSHAKE_READS:
                # Drop anything still buffered so the transfer cannot trip over
                # banner residue. The bootloader sends another 'C' within a
                # second, which XMODEM's own handshake will pick up.
                connection.reset_input_buffer()
                print("  bootloader ready", flush=True)
                return True
        else:
            # Banner text, or the tail of a previous program: start again.
            clean_reads = 0
    return False


def upload(connection: serial.Serial, image: Path) -> bool:
    """Sends `image` over XMODEM. Returns whether the transfer was accepted."""

    def getc(size: int, timeout: float = 1):
        connection.timeout = timeout
        return connection.read(size) or None

    def putc(data: bytes, timeout: float = 1):
        connection.write_timeout = timeout
        written = connection.write(data)
        connection.flush()
        return written

    with image.open("rb") as stream:
        return bool(XMODEM(getc, putc).send(stream, retry=16, timeout=10))


def capture(connection: serial.Serial, seconds: float) -> str:
    """Streams the program's output for `seconds`, echoing it as it arrives.

    Streamed rather than buffered so a hang is visible immediately -- the last
    line printed localises it, which is how several bugs here were narrowed
    down.
    """
    connection.timeout = 0.1
    deadline = time.monotonic() + seconds
    chunks: list[str] = []
    while time.monotonic() < deadline:
        data = connection.read(4096)
        if not data:
            continue
        text = data.decode("utf-8", errors="replace")
        chunks.append(text)
        sys.stdout.write(text)
        sys.stdout.flush()
    return "".join(chunks)


def run_once(args: argparse.Namespace, payload: bytes, run: int, total: int) -> bool:
    if total > 1:
        print(f"\n--- run {run}/{total} " + "-" * 48, flush=True)

    # Opened separately from the `with` so a failure here can be explained.
    # `main` already checked the node exists, which is necessary but not
    # sufficient: a /dev/ttyUSB* node outlives the device it belonged to, so a
    # board whose USB link has dropped presents as a node that opens with EIO.
    # pyserial's own error for that is a traceback ending in "Input/output
    # error", which buries the one thing worth knowing -- that this is physical.
    try:
        port = serial.Serial(args.port, args.baud, timeout=1, write_timeout=10)
    except (serial.SerialException, OSError) as error:
        print(f"  could not open {args.port}: {error}")
        print("  The device node exists but the link is down. That is what a")
        print("  loose cable, a failing port, or a dropped VM passthrough looks")
        print("  like -- not a software fault, and not something re-running fixes.")
        print("  `dmesg | tail` usually names it outright: 'Cannot enable',")
        print("  'unable to enumerate USB device', 'failed set request'.")
        print("  Re-seat the cable (ideally a different port), then re-run.")
        return False

    with port as connection:
        connection.reset_input_buffer()

        if not wait_for_bootloader(connection, args.wait_reset):
            # Deliberately not blaming J12 or the port. By far the most common
            # cause is simply that RESET was not pressed while this was
            # listening: whatever was flashed before is still running, and many
            # of these images print nothing at all.
            print("  no bootloader handshake seen.")
            print("  The previously flashed image is probably still running --")
            print("  press RESET *after* starting this command, not before.")
            return False

        try:
            uploaded = upload(connection, args.image)
        except serial.SerialException as error:
            print(f"  serial error during upload: {error}")
            return False
        if not uploaded:
            print("  XMODEM upload FAILED -- reset into UART boot mode and check the port")
            return False

        print(f"  uploaded {len(payload)} bytes", flush=True)

        # Hand the bootloader the ENTER it asked for.
        connection.write(START_PROGRAM)
        connection.flush()

        if args.capture > 0:
            print("  --- output " + "-" * 52, flush=True)
            text = capture(connection, args.capture)
            if PROGRAM_BANNER not in text:
                print(
                    f"\n  note: never saw '{PROGRAM_BANNER}' -- the image may not have started",
                    flush=True,
                )
    return True


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Flash a THEJAS32 image over XMODEM and capture its output."
    )
    parser.add_argument("image", type=Path, help="raw .bin image to flash")
    parser.add_argument("--port", default="/dev/ttyUSB0", help="serial device (default: %(default)s)")
    parser.add_argument("--baud", type=int, default=115200, help="default: %(default)s")
    parser.add_argument(
        "--capture",
        type=float,
        default=10.0,
        help="seconds of output to capture after upload; 0 to skip (default: %(default)s)",
    )
    parser.add_argument(
        "--repeat",
        type=int,
        default=1,
        help="flash and capture N times, to expose races that a single run hides",
    )
    parser.add_argument(
        "--wait-reset",
        type=float,
        default=30.0,
        help="seconds to wait for the bootloader handshake (default: %(default)s)",
    )
    args = parser.parse_args()

    if not args.image.is_file():
        raise SystemExit(f"no such image: {args.image}")

    # Check the port exists before printing anything about the image. A missing
    # port is the single most common failure -- the board gets unplugged, or a
    # VM drops its USB passthrough -- and pyserial's own error for it is a
    # traceback, which buries the one fact that matters.
    if not Path(args.port).exists():
        candidates = sorted(
            str(p) for p in Path("/dev").glob("tty*")
            if p.name.startswith(("ttyUSB", "ttyACM"))
        )
        print(f"serial port not found: {args.port}")
        if candidates:
            print("  available: " + ", ".join(candidates))
            print(f"  re-run with --port {candidates[0]}")
        else:
            print("  no ttyUSB*/ttyACM* devices present at all.")
            print("  The board is unplugged, powered off, or (in a VM) the USB")
            print("  passthrough has dropped.")
        raise SystemExit(2)

    payload = args.image.read_bytes()
    digest = hashlib.sha256(payload).hexdigest()

    # The hash is not ceremony. During the async debugging it was repeatedly
    # unclear whether the freshly built binary had actually been flashed, and at
    # least one confusing result came from a stale image.
    print(f"image  {args.image}")
    print(f"bytes  {len(payload)}")
    print(f"sha256 {digest}")
    print(f"port   {args.port} @ {args.baud}")

    failures = 0
    for run in range(1, args.repeat + 1):
        # A disconnect can land anywhere: opening the port, waiting for the
        # handshake, mid-XMODEM, or mid-capture. pyserial raises from whichever
        # read or write was in flight, and the traceback names that call rather
        # than the actual cause. Catching here keeps every one of those paths
        # reporting the same physical fact.
        try:
            completed = run_once(args, payload, run, args.repeat)
        except (serial.SerialException, OSError) as error:
            print(f"  link lost mid-session: {error}")
            print("  The USB bridge dropped part-way through. This is the same")
            print("  physical fault as a failed open, just caught later -- a")
            print("  flapping link enumerates, works briefly, then goes away.")
            print("  `dmesg | tail` will show the re-enumerations.")
            completed = False
        if not completed:
            failures += 1

    if args.repeat > 1:
        print(f"\n{args.repeat - failures}/{args.repeat} runs uploaded successfully")
    raise SystemExit(1 if failures else 0)


if __name__ == "__main__":
    main()
