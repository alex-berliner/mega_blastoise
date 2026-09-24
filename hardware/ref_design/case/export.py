"""Regenerate everything derived from case.scad.

    python3 export.py          # positions.json, board.dxf
    python3 export.py --stl    # also tub.stl, lid.stl (slow)

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

HERE = os.path.dirname(os.path.abspath(__file__))
SCAD = os.path.join(HERE, "case.scad")


def openscad(out, part, extra=()):
    r = subprocess.run(["openscad", "-o", out, "-D", f'part="{part}"', *extra, SCAD],
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
    for part in ("board",):
        openscad(os.path.join(HERE, f"{part}.dxf"), part)
        print(f"{part}.dxf")
    if "--stl" in sys.argv:
        for part in ("tub", "lid"):
            openscad(os.path.join(HERE, f"{part}.stl"), part)
            print(f"{part}.stl")


if __name__ == "__main__":
    main()
