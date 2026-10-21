#!/usr/bin/env python3
"""Does a part need supports? Measured from its STL in its print pose (bed at its lowest z).

    python3 case/overhangs.py part.stl [...]     # one verdict line per part

A part needs supports only where it has a real overhang: a downward face flatter than 45 degrees,
off the bed, that a printer can't bridge or step out to. Two kinds don't count:
  - terraces: the 0.1 mm steps case.scad builds 45 degree slopes from (each under TERRACE wide);
  - bridges and ledges: a flat underside every point of which is within BRIDGE of an edge held up
    from below on two or more sides (a span up to 2 * BRIDGE, like the 4.4 mm screw counterbore),
    or within LEDGE of one held up on one side only.
Everything else is reported, with its area and where it is.
"""
import sys

import numpy as np
import shapely
import trimesh
from shapely.geometry import MultiLineString, Polygon
from shapely.ops import polygonize, unary_union

STEEP = np.cos(np.radians(45)) + 1e-4   # a downward face flatter than 45 degrees has n.z below -STEEP
BED = 0.2                               # faces this close to the bed print on it
TERRACE = 0.16                          # a step this narrow is a 45 degree slope's terrace
BRIDGE = 2.6                            # held up on two sides: spans up to about 5 mm
LEDGE = 1.0                             # held up on one side only
GRID = 0.25                             # sample spacing over a flat underside, mm


def _slice(mesh, z):
    """The part's solid cross-section at height z, as shapely polygons."""
    segs = trimesh.intersections.mesh_plane(mesh, [0, 0, 1], [0, 0, z])
    if len(segs) == 0:
        return Polygon()
    # rounded so neighbouring triangles' cut ends meet exactly and the loops close
    lines = unary_union(MultiLineString([np.round(s[:, :2], 4) for s in segs]))
    polys = list(polygonize(lines))
    if not polys:
        return Polygon()
    # nested loops: the even-odd rule (a hole inside a wall inside a hole...), on each face's
    # outer loop (polygonize already gives a face its holes, and XOR with them would fill them)
    out = Polygon()
    for p in sorted(polys, key=lambda p: -p.area):
        out = out.symmetric_difference(Polygon(p.exterior))
    return out


def _flat_regions(mesh, faces, z):
    tris = [Polygon(t[:, :2]) for t in mesh.vertices[mesh.faces[faces]]]
    u = unary_union([t.buffer(1e-4) for t in tris if t.area > 1e-9])
    return list(u.geoms) if hasattr(u, "geoms") else [u]


def _verdict(region, below):
    """None if the region prints without support, else why not."""
    if region.buffer(-TERRACE / 2).is_empty:
        return None                                                  # a terrace
    held = below.buffer(0.15)
    runs = []                                       # the held edge's pieces, round every loop
    for ring in [region.exterior, *region.interiors]:
        pts = np.array([ring.interpolate(d).coords[0] for d in np.arange(0, ring.length, 0.1)])
        if len(pts) == 0:
            continue
        ok = shapely.contains_xy(held, pts[:, 0], pts[:, 1])
        ring_runs, cur = [], []
        for p, h in zip(pts, ok):
            if h:
                cur.append(p)
            elif cur:
                ring_runs.append(cur); cur = []
        if cur:
            if ring_runs and ok[0]:
                ring_runs[0] = cur + ring_runs[0]     # the run that wraps past the loop's start
            else:
                ring_runs.append(cur)
        runs += ring_runs
    if not runs:
        return "floats: no edge held up from below"
    held_edges = MultiLineString([r if len(r) > 1 else [r[0], r[0] + 1e-3] for r in runs])
    x0, y0, x1, y1 = region.bounds
    gx, gy = np.meshgrid(np.arange(x0, x1 + GRID, GRID), np.arange(y0, y1 + GRID, GRID))
    gx, gy = gx.ravel(), gy.ravel()
    inside = shapely.contains_xy(region, gx, gy)
    if not inside.any():
        return None                                                  # thinner than the grid
    d = shapely.distance(shapely.points(gx[inside], gy[inside]), held_edges).max()
    reach = BRIDGE if len(runs) >= 2 else LEDGE
    if d <= reach:
        return None
    return f"{'bridge' if len(runs) >= 2 else 'ledge'} reaches {d:.1f} mm from a held edge (limit {reach})"


def check(path):
    """[(area mm2, z, (x, y), why)] for each real overhang of the STL at path."""
    mesh = trimesh.load(path, force="mesh")
    mesh.merge_vertices()
    zmin = mesh.bounds[0][2]
    n = mesh.face_normals
    lo = mesh.vertices[mesh.faces][:, :, 2].min(1)
    down = (n[:, 2] < -STEEP) & (lo > zmin + BED)
    flat = down & (n[:, 2] < -0.9999)
    found = []
    sloped = down & ~flat
    if sloped.any():
        a = mesh.area_faces[sloped].sum()
        if a > 1.0:
            c = mesh.triangles_center[sloped][np.argmax(mesh.area_faces[sloped])]
            found.append((a, c[2], (c[0], c[1]), "faces sloping flatter than 45 degrees"))
    zs = np.round(mesh.triangles_center[flat][:, 2], 3)
    idx = np.nonzero(flat)[0]
    for z in np.unique(zs):
        faces = idx[zs == z]
        below = _slice(mesh, z - 0.1)
        for region in _flat_regions(mesh, faces, z):
            why = _verdict(region, below)
            if why:
                c = region.representative_point()
                found.append((region.area, z, (c.x, c.y), why))
    return found


def needs_support(path):
    return bool(check(path))


if __name__ == "__main__":
    for p in sys.argv[1:]:
        f = check(p)
        if not f:
            print(f"{p}: no supports")
        for a, z, (x, y), why in f:
            print(f"{p}: OVERHANG {a:.1f} mm2 at z {z - trimesh.load(p).bounds[0][2]:.2f}, ({x:.1f}, {y:.1f}): {why}")
