#!/usr/bin/env python3
"""Render preview.svg as a layered image, one layer per concern.

The flat preview crams strips, cuts, part bodies and six colours of fly wire
into one picture, which is hard to read while building. This writes
`preview.psd` with every layer named and individually toggleable, plus
`preview.ora` for Krita and friends.

    python3 make_layered.py [--scale 5.0]

PSD is the GIMP-facing output because it is the only layered format this
install can actually open: `file-psd` is a native plug-in, whereas ORA support
is a Python-Fu plug-in and GIMP 2.10 needs Python 2 for that, which the distro
no longer ships. XCF is not an option either - writing it means driving
gimp-console, and GIMP batch cannot write files in a sandbox (it blocks at 0%
CPU even on a plug-in-free core save). Both formats here are written directly.

Open in GIMP with File > Open, then save as .xcf if you want the native format.

Needs a headless Chromium to rasterise the layers (the playwright one is fine).
"""

import argparse
import concurrent.futures as cf
import glob
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import zipfile
from xml.sax.saxutils import escape

from PIL import Image

import psd_writer

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import layout as L  # noqa: E402

OUT_ORA = os.path.join(HERE, "preview.ora")
OUT_PSD = os.path.join(HERE, "preview.psd")

# Layers that start hidden, so you can bring up one wire colour at a time.
# Everything starts visible; hide what you do not need from the layers panel.
HIDDEN_PREFIXES = ()


def find_chromium():
    for pattern in (
        os.path.expanduser("~/.cache/ms-playwright/chromium-*/chrome-linux64/chrome"),
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/google-chrome",
    ):
        hits = sorted(glob.glob(pattern))
        if hits:
            return hits[-1]
    raise SystemExit("no chromium found for rasterising")


def render(chromium, svg_path, png_path, w, h):
    """SVG -> transparent PNG at exactly w x h device pixels."""
    subprocess.run(
        [chromium, "--headless=new", "--disable-gpu", "--hide-scrollbars",
         "--no-sandbox", "--disable-dev-shm-usage",
         "--default-background-color=00000000",
         f"--window-size={int(w)},{int(h)}",
         f"--screenshot={png_path}", f"file://{svg_path}"],
        check=True, capture_output=True, timeout=120)


def png_size(path):
    """(width, height) straight out of the IHDR, so PIL is not required."""
    with open(path, "rb") as fh:
        head = fh.read(24)
    if head[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path} is not a PNG")
    return struct.unpack(">II", head[16:24])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--scale", type=float, default=5.0,
                    help="px per mm; 5.0 gives a ~300 x 750 px canvas")
    args = ap.parse_args()

    tree, W, H = L.svg_layer_tree(scale=args.scale)
    layers = [(n, e) for _, members in tree for n, e in members]
    chromium = find_chromium()
    tmp = os.environ.get("MB_ORA_TMP") or tempfile.mkdtemp(prefix="mb-ora-")
    os.makedirs(tmp, exist_ok=True)

    jobs = []
    for i, (name, elements) in enumerate(layers):
        # only the background layer is opaque
        svg = L.layer_svg(name, elements, W, H,
                          transparent=(name != "background"))
        svg_path = os.path.join(tmp, f"{i:02d}.svg")
        png_path = os.path.join(tmp, f"{i:02d}.png")
        with open(svg_path, "w") as fh:
            fh.write(svg)
        jobs.append((name, svg_path, png_path))

    todo = [j for j in jobs if not os.path.exists(j[2])]
    if todo:
        with cf.ThreadPoolExecutor(max_workers=6) as pool:
            list(pool.map(lambda j: render(chromium, j[1], j[2], W, H), todo))
    print(f"  rendered {len(jobs)} layers ({len(todo)} new)")

    # the flat preview doubles as ORA's required merged image
    flat_svg = os.path.join(tmp, "merged.svg")
    flat_png = os.path.join(tmp, "merged.png")
    with open(flat_svg, "w") as fh:
        fh.write(L.svg())
    if not os.path.exists(flat_png):
        render(chromium, flat_svg, flat_png, W, H)

    pw, ph = png_size(jobs[0][2])

    # ORA lists the TOP entry first, so both the groups and their contents
    # come out reversed relative to draw order.
    png_of = {name: os.path.basename(png) for name, _, png in jobs}

    def layer_xml(name, indent):
        hidden = name.startswith(HIDDEN_PREFIXES)
        return (f'{indent}<layer name="{escape(name)}" '
                f'src="data/{png_of[name]}" x="0" y="0" opacity="1.0" '
                f'visibility="{"hidden" if hidden else "visible"}"/>')

    rows = []
    for group, members in reversed(tree):
        if group is None:
            rows += [layer_xml(n, "    ") for n, _ in reversed(members)]
            continue
        rows.append(f'    <stack name="{escape(group)}" opacity="1.0" '
                    f'visibility="visible">')
        rows += [layer_xml(n, "      ") for n, _ in reversed(members)]
        rows.append("    </stack>")
    stack = (f'<?xml version="1.0" encoding="UTF-8"?>\n'
             f'<image w="{pw}" h="{ph}" xres="72" yres="72">\n'
             f'  <stack>\n' + "\n".join(rows) + "\n  </stack>\n</image>\n")

    with zipfile.ZipFile(OUT_ORA, "w", zipfile.ZIP_DEFLATED) as z:
        # the mimetype entry must be first and stored uncompressed
        z.writestr(zipfile.ZipInfo("mimetype"), "image/openraster",
                   compress_type=zipfile.ZIP_STORED)
        z.writestr("stack.xml", stack)
        for _, _, png in jobs:
            z.write(png, f"data/{os.path.basename(png)}")
        z.write(flat_png, "mergedimage.png")
        z.write(flat_png, "Thumbnails/thumbnail.png")

    # --- the same stack as a PSD, which is what GIMP here can open ---
    planes = {}
    for name, _, png in jobs:
        im = Image.open(png).convert("RGBA")
        if im.size != (pw, ph):
            im = im.resize((pw, ph))
        planes[name] = tuple(c.tobytes() for c in im.split())

    def node(name):
        return ("layer", name, not name.startswith(HIDDEN_PREFIXES),
                planes[name])

    psd_layers = []
    for group, members in tree:
        if group is None:
            psd_layers += [node(n) for n, _ in members]
        else:
            psd_layers.append(("group", group, True,
                               [node(n) for n, _ in members]))
    merged = Image.open(flat_png).convert("RGB")
    if merged.size != (pw, ph):
        merged = merged.resize((pw, ph))
    psd_writer.write_psd(OUT_PSD, pw, ph, psd_layers,
                         tuple(c.tobytes() for c in merged.split()))

    shutil.rmtree(tmp, ignore_errors=True)
    hidden = sum(1 for n, _, _ in jobs if n.startswith(HIDDEN_PREFIXES))
    for path in (OUT_PSD, OUT_ORA):
        print(f"wrote {path} ({os.path.getsize(path) / 1e6:.1f} MB)")
    print(f"  {len(jobs)} layers, {pw} x {ph} px, {hidden} start hidden")
    print("  GIMP: File > Open the .psd (this install has no ORA plug-in)")


if __name__ == "__main__":
    main()
