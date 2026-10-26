#!/usr/bin/env python3
"""The top's wall under its side rounds, measured square to the outer surface (section every 1 mm along the case,
both sides): the thinnest plastic between the round (beyond x 35.8, over z 4.5) and the inside, groove or opening.

    python3 case/check_wall.py           # a stage of tools/build.py (check_wall), about 2 minutes

Renders the top (the assembly frame, case.scad's top()) and prints the minimum over the slope and the band (must be >= 0.7:
the inside's corner is cut by the round's offset, `uni_in_limit` in case.scad) and over the ends' sides and ends (the skirts'
groove over the tub's tongue: 0.669 at its upper outer corner since the tongue's chamfer, `core_lead`; see SHELL.md), leaving
out the USB-C opening (y -4 to 8, right side), which breaks through the round by design. Then the same in 3D, the whole round
including the plan corners (where a section isn't square to the surface): points on the outside against the nearest point
of every other surface. Fails when any drops below its figure.
"""
import os
import subprocess
import sys
import tempfile

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "tools"))
import scad  # noqa: E402

import json
import re

SLOPE_MIN, DECK_MIN, MESH_MIN = 0.699, 0.66, 0.62   # under an ear's lifted round; the sides and the ends' round; the 3D check (its points are 0.1 apart)
POS = json.load(open(os.path.join(HERE, "positions.json")))
CASE_W, CASE_L = POS["case"]["w"], POS["case"]["l"]
SRC = open(os.path.join(HERE, "case.scad")).read()
RUN = 5.85                            # the slope's run (case.scad uni_run, 5.85 with the 3.5 in panel's 9.6 mm face)
EARS = [[float(v) for v in e.split(",")] for e in re.search(r"uni_ears = \[\[(.*?)\]\];", SRC).group(1).split("], [")]   # [side, y0, y1]
END_Y = CASE_L / 2 - 3.35             # where the ends' round starts (63.0 on the 133 mm case)


def in_ear(y, side):
    """Is the side wall at y under an ear's flat top or its slope (the lifted round)?"""
    return any(e[0] == side and e[1] - RUN - 0.5 <= y <= e[2] + RUN + 0.5 for e in EARS)


def sections(m, positions, axis=1):
    """(u, the material's section at u along `axis` as a shapely polygon in (the other horizontal axis, z))."""
    import trimesh
    from shapely.geometry import MultiLineString
    from shapely.ops import polygonize, unary_union
    h = 0 if axis == 1 else 1
    for y in positions:
        n, o = [0, 0, 0], [0, 0, 0]
        n[axis], o[axis] = 1, y
        s = trimesh.intersections.mesh_plane(m, n, o)
        if len(s) == 0:
            continue
        polys = sorted(polygonize(unary_union(MultiLineString([[(round(float(p[h]), 5), round(float(p[2]), 5)) for p in seg] for seg in s]))), key=lambda p: -p.area)
        out = None
        for p in polys:
            out = p if out is None else out.symmetric_difference(p)
        yield y, out


def round_walls(p, side, start=35.8):
    from shapely.affinity import scale
    from shapely.geometry import LineString, Point
    if side < 0:
        p = scale(p, -1, 1, origin=(0, 0))
    hull = p.convex_hull.exterior
    n = 4000
    for s in np.linspace(0, hull.length, n):
        q, q2, q0 = hull.interpolate(s), hull.interpolate(min(s + 0.01, hull.length)), hull.interpolate(max(s - 0.01, 0))
        if not (q.x > start and q.y > 4.5) or p.boundary.distance(q) > 1e-3:
            continue
        tx, tz = q2.x - q0.x, q2.y - q0.y
        nx, nz = -tz / np.hypot(tx, tz), tx / np.hypot(tx, tz)
        if not p.contains(Point(q.x + nx * 0.05, q.y + nz * 0.05)):
            nx, nz = -nx, -nz
        hit = LineString([(q.x + nx * 1e-3, q.y + nz * 1e-3), (q.x + nx * 3, q.y + nz * 3)]).intersection(p.boundary)
        ds = [np.hypot(g.x - q.x, g.y - q.y) for g in getattr(hit, "geoms", [hit]) if g.geom_type == "Point"]
        if ds:
            yield min(ds), q.x * side, q.y


def mesh_min(m):
    """(thinnest, where): the round's outside (case.scad's uni_outer, sampled) to the nearest sample of any other surface."""
    import trimesh
    from scipy.spatial import cKDTree
    with tempfile.TemporaryDirectory() as tmp:
        src, stl = os.path.join(tmp, "o.scad"), os.path.join(tmp, "o.stl")
        open(src, "w").write(f'include <{os.path.join(HERE, "case.scad")}>\nuni_outer();\n')
        r = subprocess.run([*scad.cmd(), "-o", stl, "-D", 'part="none"', src], capture_output=True, text=True)
        if r.returncode or "ERROR" in r.stderr:
            sys.exit(r.stderr[-1500:])
        outer = trimesh.load(stl)
    region = lambda p: (p[:, 2] > 4.5) & ((np.abs(p[:, 0]) > 34) | (np.abs(p[:, 1]) > CASE_L / 2 - 6.3))
    op = trimesh.sample.sample_surface(outer, 4000000, seed=1)[0]
    op = op[region(op)]
    tp = trimesh.sample.sample_surface(m, 4000000, seed=2)[0]
    tp = tp[region(tp)]
    inner = tp[cKDTree(op).query(tp)[0] >= 0.2]          # not on the outside
    usb = (op[:, 0] > 36) & (op[:, 1] > -6) & (op[:, 1] < 10)
    o = op[(op[:, 2] > 4.95) & ((np.abs(op[:, 0]) > CASE_W / 2 - 2.75) | (np.abs(op[:, 1]) > CASE_L / 2 - 2.75)) & ~usb]
    d = cKDTree(inner).query(o)[0]
    # the island's slopes (all four sides and the ears'): the outside above the ends' face, beyond the panel's pocket, clear of the
    # speaker's grille, the pinholes and the USB-C opening
    gx0, gx1 = [float(v) for v in re.search(r"grille_x = \[([-\d.]+), ([-\d.]+)\]", SRC).group(1, 2)]
    spk = POS["speaker"]["c"]
    pins = [(p[0] - 100, 100 - p[1]) for p in POS["pinholes"]]
    isl = op[(op[:, 2] > 7.78) & ((np.abs(op[:, 0]) > 28.1) | (np.abs(op[:, 1]) > 42.4)) & (np.abs(op[:, 0]) < 35.5) & ~usb
             & ~((op[:, 0] > gx0 - 1) & (op[:, 0] < gx1 + 1) & (np.abs(op[:, 1] - (100 - spk[1])) < 8.5)) 
             & ~np.any([np.hypot(op[:, 0] - px, op[:, 1] - py) < 2.5 for px, py in pins], axis=0)]
    ds = cKDTree(inner).query(isl)[0]
    return float(d.min()), tuple(o[d.argmin()].round(2)), float(ds.min()), tuple(isl[ds.argmin()].round(2))


def main():
    import trimesh
    with tempfile.TemporaryDirectory() as tmp:
        src, stl = os.path.join(tmp, "t.scad"), os.path.join(tmp, "t.stl")
        open(src, "w").write(f'include <{os.path.join(HERE, "case.scad")}>\ntop();\n')
        r = subprocess.run([*scad.cmd(), "-o", stl, "-D", 'part="none"', src], capture_output=True, text=True)
        if r.returncode or "ERROR" in r.stderr:
            sys.exit(r.stderr[-1500:])
        m = trimesh.load(stl)
    low = {"slope": (9, None), "sides": (9, None), "ends": (9, None)}
    ym = np.floor(m.bounds[1][1] - 0.3)
    for y, p in sections(m, np.arange(-ym, ym + 1e-9, 1.0)):
        for side in (1, -1):
            if side == 1 and POS["usb"]["y"] and abs(y - (100 - POS["usb"]["y"])) <= 6:
                continue
            ws = list(round_walls(p, side))
            if ws:
                d, x, z = min(ws)
                k = "slope" if in_ear(y, side) else "sides"
                if d < low[k][0]:
                    low[k] = (d, (y, side, round(x, 2), round(z, 2)))
    # the ends: sections across the case at x -20 to 20 (the plan corners are further out), the end rounds from y 63
    for x, p in sections(m, np.arange(-20, 20.1, 5.0), axis=0):
        for side in (1, -1):
            ws = list(round_walls(p, side, start=END_Y))
            if ws:
                d, y, z = min(ws)
                if d < low["ends"][0]:
                    low["ends"] = (d, (x, side, round(y, 2), round(z, 2)))
    r = mesh_min(m)
    low["3d"] = r[:2]
    low["island"] = r[2:]
    for k, floor in (("slope", SLOPE_MIN), ("sides", DECK_MIN), ("ends", DECK_MIN), ("3d", MESH_MIN), ("island", MESH_MIN)):
        print(f"{k:6s} min {low[k][0]:.3f} at (y or x, side, x or y, z) {low[k][1]}  (needs {floor})")
    if low["slope"][0] < SLOPE_MIN or min(low["sides"][0], low["ends"][0]) < DECK_MIN or low["3d"][0] < MESH_MIN or low["island"][0] < MESH_MIN:
        sys.exit("FAIL: the wall under a side round is thinner than it was")


if __name__ == "__main__":
    main()
