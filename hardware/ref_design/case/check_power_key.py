#!/usr/bin/env python3
"""The power button's cap against the top (case.scad pwr_cap, pwr_cuts), in the assembled pose:

    python3 case/check_power_key.py

  at rest (on the switch's actuator) and fully pressed: the cap touches nothing of the top (no volume in common);
  lifted by the recess's play plus 0.05: the flange meets the plate over the recess (it is captive);
  its top stays under the face and over the pocket's floor at the end of its stroke.
The geometry asserts in case.scad cover the numbers; this is the solids."""
import os
import subprocess
import sys
import tempfile

import trimesh

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "tools"))
import scad  # noqa: E402


def overlap(dz):
    """Volume (mm^3) the cap, moved dz from rest, shares with the top."""
    with tempfile.TemporaryDirectory() as tmp:
        src, stl = os.path.join(tmp, "k.scad"), os.path.join(tmp, "k.stl")
        open(src, "w").write(f'include <{os.path.join(HERE, "case.scad")}>\n'
                             f'intersection() {{ translate([pwr_pos[0], pwr_pos[1], pwr_act_top + ({dz})]) pwr_cap(); top(); }}\n')
        subprocess.run([*scad.cmd(), "-o", stl, "-D", 'part="none"', src], capture_output=True, text=True)
        if not os.path.exists(stl) or os.path.getsize(stl) < 100:
            return 0.0
        m = trimesh.load(stl)
        return abs(float(m.volume)) if len(m.faces) else 0.0


def main():
    import re
    src = open(os.path.join(HERE, "case.scad")).read()
    play = 0.2 if "pwr_recess_h = 0.4" in src else None
    checks = []
    for name, dz, want in (("at rest", 0.0, 0.0), ("pressed", -0.3, 0.0), ("lifted past the recess's play", 0.25, None)):
        v = overlap(dz)
        ok = (v < 1e-3) if want == 0.0 else (v > 0.05)
        checks.append(ok)
        print(f"  {'ok  ' if ok else 'FAIL'}  cap {name}: {v:.3f} mm^3 shared with the top" + ("" if want == 0.0 else " (it must catch)"))
    print("RESULT: " + ("PASS" if all(checks) else f"FAIL ({checks.count(False)})"))
    sys.exit(0 if all(checks) else 1)


if __name__ == "__main__":
    main()
