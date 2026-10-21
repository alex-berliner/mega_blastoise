"""Regenerate everything derived from case.scad.

    python3 export.py          # positions.json, board.dxf
    python3 export.py --stl    # also tub.stl, lid.stl and insert.stl, the deck insert (slow)

positions.json is what the board scripts read: button centres, screw holes,
connector positions and height limits, all in KiCad coordinates. It comes from
the JSON line in case.scad's echo block, so the case stays the only source.
"""
import json
import os
import re
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
SCAD = os.path.join(HERE, "case.scad")
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "tools"))
import scad  # noqa: E402  (which OpenSCAD, which backend, how many at once)


def openscad(out, part, extra=()):
    r = subprocess.run([*scad.cmd(out), "-o", out, "-D", f'part="{part}"', *extra, SCAD],
                       capture_output=True, text=True)
    if r.returncode or "ERROR" in r.stderr or "WARNING: Assert" in r.stderr:
        sys.exit(f"openscad failed for {part}:\n{r.stderr[-2000:]}")
    return r.stderr


def main():
    with tempfile.TemporaryDirectory() as tmp:
        echo = os.path.join(tmp, "case.echo")
        openscad(echo, "none")
        text = open(echo).read()
    m = re.search(r'ECHO: "JSON (\{.*\})"', text)
    if not m:
        sys.exit("no JSON line in the echo block")
    data = json.loads(m.group(1).replace('\\"', '"'))
    with open(os.path.join(HERE, "positions.json"), "w") as f:
        json.dump(data, f, indent=1)
    print("positions.json:", len(data["buttons"]), "buttons,", len(data["pins"]), "locating pins")
    pool, renders = ThreadPoolExecutor(scad.jobs()), []    # the renders run at once, reported in order
    for part in ("board",):
        renders.append((f"{part}.dxf", pool.submit(openscad, os.path.join(HERE, f"{part}.dxf"), part)))
    if "--stl" in sys.argv:
        for part in ("tub", "lid", "insert"):
            renders.append((f"{part}.stl", pool.submit(openscad, os.path.join(HERE, f"{part}.stl"), part)))
    for name, render in renders:
        render.result()
        print(name)


if __name__ == "__main__":
    main()
