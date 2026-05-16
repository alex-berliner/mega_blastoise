#!/usr/bin/env python3
"""Reconstruct OLED screens from `oledfb|pN|<hex>` RTT log lines.

Firmware emits one compact defmt message per framebuffer change:
    oledfb|p2|<2048 hex chars>   (1024 packed bytes, 128x64 mono, 16 B/row)

This expands each into the same 32x128 half-block art the `:oled` USB
dump uses and writes the full sequence to an output file.

Usage:
    oled_render.py <rtt_log> <out_file> [--player N]
"""
import re
import sys

GLYPH = {(0, 0): " ", (1, 0): "▀", (0, 1): "▄", (1, 1): "█"}
LINE_RE = re.compile(r"oledfb\|p(\d)\|([0-9a-fA-F]{2048})")


def render(frame_bytes):
    # 64 rows x 16 bytes, MSB = leftmost pixel. Pair rows -> half-block.
    rows = [frame_bytes[y * 16:(y + 1) * 16] for y in range(64)]
    out = []
    for r in range(32):
        top, bot = rows[r * 2], rows[r * 2 + 1]
        line = []
        for col in range(128):
            tb = (top[col >> 3] >> (7 - (col & 7))) & 1
            bb = (bot[col >> 3] >> (7 - (col & 7))) & 1
            line.append(GLYPH[(tb, bb)])
        out.append("".join(line).rstrip())
    return out


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)
    src, dst = sys.argv[1], sys.argv[2]
    want = None
    if "--player" in sys.argv:
        want = sys.argv[sys.argv.index("--player") + 1]

    frames = 0
    with open(src, "r", errors="replace") as f, open(dst, "w") as o:
        for ln in f:
            m = LINE_RE.search(ln)
            if not m:
                continue
            pn, hexstr = m.group(1), m.group(2)
            if want and pn != want:
                continue
            fb = bytes.fromhex(hexstr)
            frames += 1
            o.write(f"\n===== frame {frames}  (P{pn}) =====\n")
            o.write("+" + "-" * 128 + "+\n")
            for row in render(fb):
                o.write("|" + row.ljust(128) + "|\n")
            o.write("+" + "-" * 128 + "+\n")
    print(f"wrote {frames} frame(s) -> {dst}")


if __name__ == "__main__":
    main()
