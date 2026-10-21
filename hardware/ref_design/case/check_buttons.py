#!/usr/bin/env python3
"""Check that the D-pad, A/B/X/Y and Start/Select work with their silicone under the top shell.

Renders case/check_buttons.scad: the pads must clear the shell (the top piece and the deck), every cap must sit at rest and press (A/B/X/Y and Start/Select straight down
dome_travel, the D-pad rocked toward each of its 8 directions) without touching the shell, and at
rest the caps may only touch the domes (no preload), and pressed caps must miss the pads' rims,
the D-pad's collar and the A/B/X/Y centre square (pressed_silicone, the domes left out: a pressed
cap squashes its dome).

    python3 case/check_buttons.py                       # every check, both lids
    python3 case/check_buttons.py silicone_shell -D sil_rib_gap=0.5   # some checks, with a setting changed
"""
import os
import re
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
SCAD = os.path.join(HERE, "check_buttons.scad")
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "tools"))
import scad  # noqa: E402  (which OpenSCAD, which backend, how many at once)
TOUCH = 0.01                        # mm^3 two parts may share and still only be touching

CHECKS = [(c, {}) for c in ("silicone_shell", "caps_shell", "caps_silicone", "silicone_tub")] + \
         [("pressed_shell", {"dpad_dir": d}) for d in range(0, 360, 45)] + \
         [("pressed_silicone", {"which": '"abxy"'}), ("pressed_silicone", {"which": '"ss"'})] + \
         [("pressed_silicone", {"which": '"dpad"', "dpad_dir": d}) for d in range(0, 360, 45)]


def volume(stl):
    """The enclosed volume of an ASCII STL (signed tetrahedra; 0 for an empty or flat one)."""
    v, tri = 0.0, []
    with open(stl) as f:
        for line in f:
            if line.lstrip().startswith("vertex"):
                tri.append([float(x) for x in line.split()[1:4]])
                if len(tri) == 3:
                    (a, b, c) = tri
                    v += (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                          + a[2] * (b[0] * c[1] - b[1] * c[0])) / 6
                    tri = []
    return abs(v)


EXTRA = []                          # -D name=value from the command line, for every render


def run(job):
    check, defs = job
    with tempfile.TemporaryDirectory() as tmp:
        out = os.path.join(tmp, "x.stl")
        args = [*scad.cmd(), "--export-format", "asciistl", "-o", out, "-D", 'part="none"', "-D", f'check="{check}"']
        for k, v in defs.items():
            args += ["-D", f"{k}={v}"]
        r = subprocess.run(args + EXTRA + [SCAD], capture_output=True, text=True)
        if "ERROR" in r.stderr or "Assertion" in r.stderr:
            return job, None, r.stderr[-800:]
        if "top level object is empty" in r.stderr or not os.path.exists(out):
            return job, 0.0, ""
        return job, volume(out), ""


def main():
    argv, only = sys.argv[1:], []
    while argv:
        a = argv.pop(0)
        if a == "-D":
            EXTRA.extend(["-D", argv.pop(0)])
        else:
            only.append(a)
    jobs = [c for c in CHECKS if not only or c[0] in only]
    with ThreadPoolExecutor(max_workers=scad.jobs()) as pool:
        results = list(pool.map(run, jobs))
    bad = 0
    print("Button stack check (seat 1; seat 2 is the same turned round)")
    for (check, defs), vol, err in results:
        what = check + (f" {defs['which'].strip(chr(34))}" if "which" in defs else "") + \
            (f" D-pad {defs['dpad_dir']} deg" if "dpad_dir" in defs else "")
        if vol is None:
            bad += 1
            print(f"  FAIL  {what}: openscad error\n{err}")
        elif vol > TOUCH:
            bad += 1
            print(f"  FAIL  {what}: the parts overlap by {vol:.3f} mm^3")
        else:
            print(f"  ok    {what}")
    print("RESULT: " + ("PASS" if not bad else f"FAIL ({bad})"))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
