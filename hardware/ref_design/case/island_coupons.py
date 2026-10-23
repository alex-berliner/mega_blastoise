#!/usr/bin/env python3
"""The island's coupons, 45 against 60 degrees: one small plate, for Alex to print both limits and pick.

    BAMBU_STUDIO=/path/to/BambuStudio python3 case/island_coupons.py

Renders every piece of island_coupons.scad twice from this branch's case.scad, with -D overhang=45 and
-D overhang=60 (everything that follows the limit follows it: the rounds, the island's edge, the laps;
at 60 the parts are ca1f236's to the last digit), into case/island_coupons/. Each piece must be manifold
(check_manifold.bad_edges) and print without supports at its own limit (overhangs.check with that
limit, and no sloping downward face flatter than it at all). Then the plate, laid out as the kits' are:
a Snapmaker U1 project (print_package.build, from Alex's template) and a Bambu Lab P1S project sliced
by Bambu Studio (print_package_p1s.slice_plate, support_check on its G-code), and
print/island_coupons.zip: both projects, the STLs and island_coupons_README.md (its P1S line filled in).
Not part of the case kits or overhang_kit.zip, and not a tools/build.py stage: run it by hand.
"""
import os
import shutil
import subprocess
import sys
import tempfile
import zipfile
from concurrent.futures import ProcessPoolExecutor, ThreadPoolExecutor

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "tools"))
import scad  # noqa: E402

OUT = os.path.join(HERE, "island_coupons")
PRINT = os.path.join(HERE, "print", "island_coupons")
ZIP = os.path.join(HERE, "print", "island_coupons.zip")
README = os.path.join(HERE, "island_coupons_README.md")
ANGLES = (45, 60)
PIECES = [  # (piece, what it is)
    ("corner_island", "seat 1's right corner of the island: the edge profile round the corner, the wedge"),
    ("corner_deck", "seat 1's deck round it: the core's lip across the step, the right arm beside the island"),
    ("meet_seat1", "seat 1's left arm's end, its lap"),
    ("meet_seat2", "seat 2's left arm's end, the skirt the lap goes behind"),
    ("ear_island", "the ear's seat-1 end, its lap, the island's side beside it"),
    ("ear_arm", "seat 1's right arm's end, under the ear's lap"),
]
PLATE = "island_coupons"


def render(piece, angle):
    out = os.path.join(OUT, f"{piece}_{angle}.stl")
    r = subprocess.run([*scad.cmd(), "-o", out, "-D", 'part="none"', "-D", f'piece="{piece}"', "-D", f"overhang={angle}",
                        os.path.join(HERE, "island_coupons.scad")], capture_output=True, text=True)
    if r.returncode or not os.path.exists(out) or "ERROR" in r.stderr or "WARNING: Assert" in r.stderr:
        sys.exit(f"openscad failed for {piece} at {angle}:\n{r.stderr[-2000:]}")
    return out


def verify(job):
    """(path, problems): its mesh manifold, and nothing that needs supports at its own limit."""
    path, angle = job
    import trimesh
    import check_manifold
    import overhangs
    overhangs.OVERHANG, overhangs.STEEP = angle, np.sin(np.radians(angle)) + 1e-4
    problems = []
    n, bad = check_manifold.bad_edges(path)
    if bad:
        problems.append(f"{len(bad)} of its {n} edges not on two faces")
    problems += [f"{a:.1f} mm2 at z {z:.2f}: {why}" for a, z, xy, why in overhangs.check(path)]
    m = trimesh.load(path, force="mesh")
    nz, c = m.face_normals[:, 2], m.triangles_center[:, 2]
    sloped = (nz < -overhangs.STEEP) & (nz > -0.9999) & (c > m.bounds[0][2] + overhangs.BED)
    if m.area_faces[sloped].sum() > 0.01:
        problems.append(f"{m.area_faces[sloped].sum():.2f} mm2 sloping flatter than {angle} degrees")
    return path, problems, m.volume


def main():
    os.makedirs(OUT, exist_ok=True)
    jobs = [(p, a) for p, _ in PIECES for a in ANGLES]
    with ThreadPoolExecutor(scad.jobs()) as pool:
        paths = list(pool.map(lambda j: render(*j), jobs))
    with ProcessPoolExecutor() as ex:
        checked = list(ex.map(verify, [(p, a) for p, (_, a) in zip(paths, jobs)]))
    failed = False
    for (piece, angle), (path, problems, vol) in zip(jobs, checked):
        print(f"  {os.path.basename(path):24s} {vol:7.1f} mm3  " + ("; ".join(problems) if problems else "manifold, no supports"))
        failed |= bool(problems)
    if failed:
        sys.exit("FAIL: a coupon isn't manifold or needs supports")
    parts = [(f"{p}_{a}", path) for (p, a), path in zip(jobs, paths)]

    import print_package
    shutil.rmtree(PRINT, ignore_errors=True)
    for d in ("U1", "P1S", "stl"):
        os.makedirs(os.path.join(PRINT, d))
    for _, path in parts:
        shutil.copy(path, os.path.join(PRINT, "stl"))
    print_package.OUT = os.path.join(PRINT, "U1")          # build() writes the U1 project there
    with zipfile.ZipFile(print_package.TEMPLATE) as z:
        u1, _ = print_package.build(PLATE, parts, z, set())
    print(f"  {os.path.relpath(u1, HERE)}")
    p1s_line = "P1S: not sliced (no BAMBU_STUDIO)"
    if os.environ.get("BAMBU_STUDIO"):
        import print_package_p1s
        with tempfile.TemporaryDirectory() as tmp:
            done = print_package_p1s.slice_plate(PLATE, parts, set(), tmp)
            if done is None:
                sys.exit("the P1S plate didn't slice")
            path, grams, secs, _ = done
            shutil.copy(path, os.path.join(PRINT, "P1S", PLATE + ".3mf"))
        p1s_line = f"P1S (0.20 mm Standard, Bambu PLA Basic): {grams:.1f} g, {int(secs // 3600)} h {int(secs % 3600 // 60):02d} min, no supports"
        print(f"  {p1s_line}")
    text = open(README).read().replace("<!-- P1S -->", p1s_line)
    open(os.path.join(PRINT, "README.md"), "w").write(text)
    with zipfile.ZipFile(ZIP, "w", zipfile.ZIP_DEFLATED) as o:
        for root, _, names in os.walk(PRINT):
            for n in sorted(names):
                p = os.path.join(root, n)
                o.write(p, os.path.join("island_coupons", os.path.relpath(p, PRINT)))
    print(f"{ZIP} ({os.path.getsize(ZIP) // 1024} kB)")


if __name__ == "__main__":
    main()
