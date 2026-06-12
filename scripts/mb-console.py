#!/usr/bin/env python3
"""
mb-console — streaming RTT + USB dev console for mega-blastoise.

RTT (probe-rs) and USB CDC output are merged into a single, color-coded,
timestamped stream. Lines typed at the prompt are forwarded to the USB port.

Usage:
    mb-console [ELF] [options]

    If ELF is omitted the most-recent debug/release build is used.
    If --dev is omitted the firmware CDC port is found via sysfs VID:PID.

Host commands (handled by this script):
    :help / ?   show host commands, then query the device for its own list
    :reflash    re-flash current ELF and reset
    :reset      reset the board (via probe)
    :dev        re-detect USB port
    :kill       kill stray probe-rs processes
    :q / :quit  exit

Everything else is forwarded as-is over USB. The firmware enumerates its own
commands in response to ':help' / '?' — the device is the source of truth.
"""

from __future__ import annotations

import argparse
import os
import queue
import re
import signal
import subprocess
import sys
import termios
import threading
import time
from datetime import datetime
from pathlib import Path

# ── ANSI colours ─────────────────────────────────────────────────────────────

_USE_COLOR = sys.stdout.isatty()

_C = {
    "rtt": "\033[36m",   # cyan
    "usb": "\033[32m",   # green
    "sys": "\033[33m",   # yellow
    "err": "\033[31m",   # red
    "rst": "\033[0m",
}

_ANSI_RE = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")


def _strip_ansi(s: str) -> str:
    return _ANSI_RE.sub("", s)


def _fmt(tag: str, text: str) -> str:
    ts = datetime.now().strftime("%H:%M:%S")
    label = f"[{tag.upper():3s} {ts}]"
    if _USE_COLOR:
        c = _C.get(tag, "")
        return f"{c}{label}{_C['rst']} {text}"
    return f"{label} {text}"


# ── Device / ELF discovery ───────────────────────────────────────────────────

_VID, _PID = "c0de", "cafe"


def find_fw_tty() -> str | None:
    """Locate /dev/ttyACMx for the firmware CDC port via sysfs VID:PID."""
    for tty_dir in Path("/sys/class/tty").glob("ttyACM*"):
        dev_link = tty_dir / "device"
        if not dev_link.exists():
            continue
        try:
            # dev_link resolves to USB interface; parent is USB device
            usb = dev_link.resolve().parent
            if (
                (usb / "idVendor").read_text().strip() == _VID
                and (usb / "idProduct").read_text().strip() == _PID
            ):
                return f"/dev/{tty_dir.name}"
        except OSError:
            pass
    return None


def find_elf() -> str | None:
    """Return most-recent firmware ELF from the cargo target directory."""
    root = Path(__file__).resolve().parent.parent
    candidates = []
    for profile in ("debug", "release"):
        elf = root / "target" / "thumbv6m-none-eabi" / profile / "mega-blastoise-fw"
        if elf.is_file():
            candidates.append(elf)
    if not candidates:
        return None
    return str(max(candidates, key=lambda p: p.stat().st_mtime))


# ── Serial port helpers (no pyserial dependency) ──────────────────────────────

def _open_serial(dev: str) -> int:
    """Open and configure a CDC serial port; return file descriptor."""
    # Pre-disable ECHO before the main open so there is no echo window.
    # tty settings persist while at least one fd is open; holding fd_pre
    # across the O_RDWR open guarantees ECHO is already off when fd is opened.
    fd_pre = os.open(dev, os.O_RDONLY | os.O_NOCTTY)
    a = termios.tcgetattr(fd_pre)
    a[3] &= ~termios.ECHO
    termios.tcsetattr(fd_pre, termios.TCSANOW, a)
    termios.tcflush(fd_pre, termios.TCOFLUSH)  # cancel any echo queued before ECHO was disabled

    fd = os.open(dev, os.O_RDWR | os.O_NOCTTY)
    a = termios.tcgetattr(fd)
    # raw mode
    a[0] &= ~(
        termios.IGNBRK | termios.BRKINT | termios.PARMRK |
        termios.ISTRIP | termios.INLCR | termios.IGNCR |
        termios.ICRNL | termios.IXON
    )
    a[1] &= ~termios.OPOST
    a[2] &= ~(termios.CSIZE | termios.PARENB)
    a[2] |= termios.CS8 | termios.CLOCAL | termios.CREAD
    a[3] &= ~(termios.ECHO | termios.ECHONL | termios.ICANON | termios.ISIG | termios.IEXTEN)
    a[6][termios.VMIN] = 0
    a[6][termios.VTIME] = 1   # 100 ms read timeout
    termios.tcsetattr(fd, termios.TCSANOW, a)
    os.close(fd_pre)
    return fd


def _serial_send(dev: str, text: str) -> None:
    """Send a line to the USB serial port.

    The line is prefixed with DEL bytes to clear any junk sitting in the
    firmware's partial-line buffer (e.g. output echoed back into the device
    during the brief cooked-tty window when the port is first opened — see
    _open_serial). The firmware pops one buffered char per DEL and ignores
    DEL on an empty buffer, so this never completes or repeats a line.
    """
    fd = os.open(dev, os.O_WRONLY | os.O_NOCTTY)
    try:
        os.write(fd, b"\x7f" * 64 + (text + "\n").encode())
    finally:
        os.close(fd)


# ── Background reader threads ─────────────────────────────────────────────────

def _rtt_reader(
    elf: str,
    out_q: queue.Queue,
    stop: threading.Event,
    proc_ref: list,
) -> None:
    """Stream probe-rs RTT output into *out_q*.  Restarts on disconnect."""
    cmd = [
        "probe-rs", "attach", "--preset", "pico",
        "--no-location", "--no-timestamps", elf,
    ]
    while not stop.is_set():
        try:
            proc = subprocess.Popen(
                cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL,
                text=True,
                bufsize=1,
            )
            proc_ref[0] = proc
            for raw in proc.stdout:
                line = _strip_ansi(raw.rstrip("\r\n"))
                if line:
                    out_q.put(("rtt", line))
                if stop.is_set():
                    proc.terminate()
                    break
            proc_ref[0] = None
            proc.wait()
            if not stop.is_set():
                out_q.put(("sys", "RTT disconnected — reconnecting in 2 s"))
                stop.wait(2.0)
        except FileNotFoundError:
            proc_ref[0] = None
            out_q.put(("err", "probe-rs not found in PATH"))
            return
        except Exception as exc:
            proc_ref[0] = None
            if not stop.is_set():
                out_q.put(("err", f"RTT: {exc}"))
                stop.wait(2.0)


def _usb_reader(dev_ref: list[str | None], out_q: queue.Queue, stop: threading.Event) -> None:
    """Stream USB CDC output into *out_q*.  Reconnects when device reappears."""
    buf = b""
    last_dev: str | None = None

    while not stop.is_set():
        dev = dev_ref[0]
        if not dev or not Path(dev).exists():
            # Device path stale or gone — try to find it by VID:PID.
            found = find_fw_tty()
            if found:
                dev_ref[0] = found
            stop.wait(1.0)
            continue

        if dev != last_dev:
            out_q.put(("sys", f"USB: {dev}"))
            last_dev = dev

        try:
            fd = _open_serial(dev)
            buf = b""
            while not stop.is_set():
                try:
                    chunk = os.read(fd, 512)
                except OSError:
                    break
                if not chunk:
                    continue
                buf += chunk
                while b"\n" in buf:
                    line_b, buf = buf.split(b"\n", 1)
                    # Drop backspace echo ("\b \b" per char the fw popped, e.g.
                    # from the DEL prefix _serial_send uses to clear junk).
                    line_b = line_b.replace(b"\x08 \x08", b"")
                    line = line_b.rstrip(b"\r").decode("utf-8", errors="replace")
                    if line:
                        out_q.put(("usb", line))
            os.close(fd)
        except OSError as exc:
            out_q.put(("sys", f"USB error: {exc}"))

        if not stop.is_set():
            stop.wait(1.0)


def _printer(out_q: queue.Queue, stop: threading.Event, log_fh) -> None:
    """Pull items from *out_q* and print them."""
    while not stop.is_set():
        try:
            tag, text = out_q.get(timeout=0.1)
        except queue.Empty:
            continue

        line = _fmt(tag, text)
        # \r\033[K clears the current readline input line before printing,
        # then readline redraws it on the next keypress.
        sys.stdout.write(f"\r\033[K{line}\n")
        sys.stdout.flush()

        if log_fh:
            plain = _fmt.__wrapped__(tag, text) if hasattr(_fmt, "__wrapped__") else \
                f"[{tag.upper():3s} {datetime.now().strftime('%H:%M:%S')}] {text}"
            log_fh.write(plain + "\n")
            log_fh.flush()


# ── probe-rs helpers ──────────────────────────────────────────────────────────

def _probe_run(args: list[str], out_q: queue.Queue, label: str) -> bool:
    out_q.put(("sys", label))
    r = subprocess.run(
        ["probe-rs"] + args + ["--preset", "pico"],
        capture_output=True, text=True,
    )
    if r.returncode != 0:
        for line in (r.stderr or r.stdout or "failed").strip().splitlines():
            out_q.put(("err", line))
        return False
    return True


def _kill_probe_rs(out_q: queue.Queue | None = None) -> None:
    result = subprocess.run(["pkill", "-f", "probe-rs"], capture_output=True)
    if out_q is not None:
        if result.returncode == 0:
            out_q.put(("sys", "killed stray probe-rs processes"))
        else:
            out_q.put(("sys", "no stray probe-rs processes"))


# ── Main ─────────────────────────────────────────────────────────────────────

def main() -> None:
    global _USE_COLOR

    ap = argparse.ArgumentParser(
        prog="mb-console",
        description="Streaming RTT + USB dev console for mega-blastoise.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="Host commands" + __doc__.split("Host commands")[1],
    )
    ap.add_argument("elf", nargs="?", help="Firmware ELF (auto-detected if omitted)")
    ap.add_argument("--dev", metavar="PATH", help="USB device (auto-detected if omitted)")
    ap.add_argument("--no-rtt", action="store_true", help="Disable RTT reader")
    ap.add_argument("--no-usb", action="store_true", help="Disable USB reader")
    ap.add_argument("--no-color", action="store_true", help="Disable colour output")
    ap.add_argument("--log", metavar="FILE", help="Append all output to FILE")
    args = ap.parse_args()

    if args.no_color:
        _USE_COLOR = False

    elf = args.elf or find_elf()

    usb_dev: str | None
    if args.no_usb:
        usb_dev = None
    else:
        usb_dev = args.dev or find_fw_tty() or "/dev/ttyACM1"

    log_fh = open(args.log, "a") if args.log else None

    out_q: queue.Queue = queue.Queue()
    stop = threading.Event()
    dev_ref: list[str | None] = [usb_dev]

    # Kill any stray probe-rs processes before starting our own.
    _kill_probe_rs()

    # Start background threads
    bg: list[threading.Thread] = []
    rtt_proc_ref: list = [None]

    if not args.no_rtt:
        if elf:
            t = threading.Thread(
                target=_rtt_reader, args=(elf, out_q, stop, rtt_proc_ref), daemon=True, name="rtt"
            )
            t.start(); bg.append(t)
        else:
            print("warning: no ELF found, RTT disabled", file=sys.stderr)

    if not args.no_usb:
        t = threading.Thread(
            target=_usb_reader, args=(dev_ref, out_q, stop), daemon=True, name="usb"
        )
        t.start(); bg.append(t)

    pt = threading.Thread(
        target=_printer, args=(out_q, stop, log_fh), daemon=True, name="printer"
    )
    pt.start()

    def _shutdown(*_):
        stop.set()
        p = rtt_proc_ref[0]
        if p is not None:
            try:
                p.terminate()
            except Exception:
                pass
        _kill_probe_rs()
        if log_fh:
            log_fh.close()

    signal.signal(signal.SIGTERM, _shutdown)

    # Header
    elf_label = elf or "(none)"
    dev_label = usb_dev or "(none)"
    print(f"mb-console  ELF={elf_label}  USB={dev_label}")
    print("Type to send over USB.  :help for commands.")
    print()

    def send_to_fw(line: str) -> None:
        dev = dev_ref[0]
        if not dev:
            out_q.put(("err", "no USB device (try :dev to detect)"))
            return
        # Auto-redetect if the stored path is gone.
        if not Path(dev).exists():
            found = find_fw_tty()
            if found:
                dev_ref[0] = found
                dev = found
            else:
                out_q.put(("err", f"USB device gone ({dev}); replug or :dev"))
                return
        try:
            _serial_send(dev, line)
            out_q.put(("sys", f"→ {line}"))
        except OSError as exc:
            out_q.put(("err", f"send failed: {exc}"))

    try:
        while True:
            try:
                line = input()
            except (EOFError, KeyboardInterrupt):
                break

            line = line.strip()
            if not line:
                continue

            if line in (":q", ":quit", ":exit"):
                break

            elif line in (":help", ":h", "?"):
                print(
                    "\n"
                    "  Host commands (handled by mb-console):\n"
                    "    :reflash          re-flash current ELF and reset\n"
                    "    :reset            reset the board (via probe, not the fw :reset)\n"
                    "    :dev              re-detect USB device\n"
                    "    :kill             kill stray probe-rs processes\n"
                    "    :q / :quit        exit\n"
                    "\n"
                    "  Anything else is sent as-is over USB; the device's own\n"
                    "  command list follows (printed by the firmware):\n"
                )
                # Forward to the firmware so the device enumerates its own
                # commands — the firmware, not this script, is the source of
                # truth for what runs on the device.
                send_to_fw(line)

            elif line == ":dev":
                found = find_fw_tty()
                if found:
                    dev_ref[0] = found
                    out_q.put(("sys", f"USB device: {found}"))
                else:
                    out_q.put(("sys", "firmware USB device not found"))

            elif line == ":kill":
                _kill_probe_rs(out_q)

            elif line == ":reset":
                # Kill RTT reader and wait for it to release the probe.
                p = rtt_proc_ref[0]
                if p is not None:
                    p.terminate()
                    try:
                        p.wait(timeout=2.0)
                    except subprocess.TimeoutExpired:
                        p.kill()
                        p.wait()
                _probe_run(["reset"], out_q, "resetting…")

            elif line == ":reflash":
                if not elf:
                    out_q.put(("err", "no ELF — pass ELF path as argument"))
                else:
                    p = rtt_proc_ref[0]
                    if p is not None:
                        p.terminate()
                    if _probe_run(["download", elf], out_q, f"flashing {Path(elf).name}…"):
                        _probe_run(["reset"], out_q, "resetting…")

            else:
                send_to_fw(line)

    finally:
        _shutdown()


if __name__ == "__main__":
    main()
