#!/usr/bin/env python3
"""The one-piece top's wall under its side rounds, measured square to the outer surface (section every 1 mm along the case,
both sides): the thinnest plastic between the round (beyond x 35.8, over z 4.5) and the inside, groove or opening.

    python3 case/uni_wall.py

Renders top_unibody (the assembly frame) and prints the minimum over the slope and the band (must be >= 0.7: the inside's
corner is cut by the round's offset, `uni_in_limit` in case.scad) and over the decks' ends (the skirts' groove over the
tub's tongue: 0.467 at its upper outer corner, the most the tongue leaves; see SHELL.md), leaving out the USB-C opening
(y -4 to 8, right side), which breaks through the round by design. Fails when either drops below its figure.
"""
import os
import subprocess
import sys
import tempfile

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "tools"))
import scad  # noqa: E402

SLOPE_MIN, DECK_MIN = 0.699, 0.46     # the slope and band (y within 35.2 of the middle); the decks' ends
SLOPE_Y = 35.2                        # where the core grooves start (tongue_y - tongue_y_fit)


def sections(m, step=1.0):
    import trimesh
    from shapely.geometry import MultiLineString
    from shapely.ops import polygonize, unary_union
    for y in np.arange(-np.floor(m.bounds[1][1] - 0.3), np.floor(m.bounds[1][1] - 0.3) + 1e-9, step):
        s = trimesh.intersections.mesh_plane(m, [0, 1, 0], [0, y, 0])
        if len(s) == 0:
            continue
        polys = sorted(polygonize(unary_union(MultiLineString([[(round(float(p[0]), 5), round(float(p[2]), 5)) for p in seg] for seg in s]))), key=lambda p: -p.area)
        out = None
        for p in polys:
            out = p if out is None else out.symmetric_difference(p)
        yield y, out


def round_walls(p, side):
    from shapely.affinity import scale
    from shapely.geometry import LineString, Point
    if side < 0:
        p = scale(p, -1, 1, origin=(0, 0))
    hull = p.convex_hull.exterior
    n = 4000
    for s in np.linspace(0, hull.length, n):
        q, q2, q0 = hull.interpolate(s), hull.interpolate(min(s + 0.01, hull.length)), hull.interpolate(max(s - 0.01, 0))
        if not (q.x > 35.8 and q.y > 4.5) or p.boundary.distance(q) > 1e-3:
            continue
        tx, tz = q2.x - q0.x, q2.y - q0.y
        nx, nz = -tz / np.hypot(tx, tz), tx / np.hypot(tx, tz)
        if not p.contains(Point(q.x + nx * 0.05, q.y + nz * 0.05)):
            nx, nz = -nx, -nz
        hit = LineString([(q.x + nx * 1e-3, q.y + nz * 1e-3), (q.x + nx * 3, q.y + nz * 3)]).intersection(p.boundary)
        ds = [np.hypot(g.x - q.x, g.y - q.y) for g in getattr(hit, "geoms", [hit]) if g.geom_type == "Point"]
        if ds:
            yield min(ds), q.x * side, q.y


def main():
    import trimesh
    with tempfile.TemporaryDirectory() as tmp:
        src, stl = os.path.join(tmp, "t.scad"), os.path.join(tmp, "t.stl")
        open(src, "w").write(f'include <{os.path.join(HERE, "case.scad")}>\ntop_unibody();\n')
        r = subprocess.run([*scad.cmd(), "-o", stl, "-D", 'part="none"', src], capture_output=True, text=True)
        if r.returncode or "ERROR" in r.stderr:
            sys.exit(r.stderr[-1500:])
        m = trimesh.load(stl)
    low = {"slope": (9, None), "decks": (9, None)}
    for y, p in sections(m):
        for side in (1, -1):
            if side == 1 and -4 <= y <= 8:
                continue
            ws = list(round_walls(p, side))
            if ws:
                d, x, z = min(ws)
                k = "slope" if abs(y) < SLOPE_Y else "decks"
                if d < low[k][0]:
                    low[k] = (d, (y, side, round(x, 2), round(z, 2)))
    for k, floor in (("slope", SLOPE_MIN), ("decks", DECK_MIN)):
        print(f"{k:6s} min {low[k][0]:.3f} at (y, side, x, z) {low[k][1]}  (needs {floor})")
    if low["slope"][0] < SLOPE_MIN or low["decks"][0] < DECK_MIN:
        sys.exit("FAIL: the wall under a side round is thinner than it was")


if __name__ == "__main__":
    main()
