#!/usr/bin/env python3
"""Every printed part's mesh is manifold: each edge is shared by exactly two faces.

    python3 case/check_manifold.py               # after case/export.py --stl and case/fitkit.py
    python3 case/check_manifold.py a.stl ...     # just these

The parts are the print plates' (print_package.PLATES): the case, the fit kit, its coupons and
the printer tests. Solids that only touch (coincident faces or edges after a union, a cut ending
exactly on a face) leave edges shared by four or six faces, which the slicers report as
non-manifold edges (Bambu Studio on top.stl, 2026-10-22); case.scad overlaps them by `eps`.
"""
import os
import sys
from concurrent.futures import ProcessPoolExecutor

import numpy as np
import trimesh

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from print_package import PLATES  # noqa: E402

MERGE = 1e-5        # vertices this close are one


def bad_edges(path):
    """(edge count, [(faces on it, midpoint), ...]) for the edges not shared by exactly two faces.
    A sliver face (two corners merged) counts against its edges too: the slicers see it."""
    m = trimesh.load(path, force="mesh", process=False)
    v = np.round(m.vertices / MERGE).astype(np.int64)
    _, idx = np.unique(v, axis=0, return_inverse=True)
    f = idx.reshape(-1)[m.faces]
    e = np.sort(f[:, [0, 1, 1, 2, 2, 0]].reshape(-1, 2), axis=1)
    edges, counts = np.unique(e, axis=0, return_counts=True)
    bad = np.nonzero(counts != 2)[0]
    pts = np.zeros((idx.max() + 1, 3))
    pts[idx.reshape(-1)] = m.vertices
    return len(edges), [(int(counts[i]), pts[edges[i]].mean(axis=0)) for i in bad]


def main():
    paths = sys.argv[1:] or sorted({p for parts in PLATES.values() for _, p in parts})
    with ProcessPoolExecutor() as ex:
        found = dict(zip(paths, ex.map(bad_edges, paths)))
    failed = 0
    for p in paths:
        n, bad = found[p]
        name = os.path.relpath(p, HERE) if os.path.abspath(p).startswith(HERE) else os.path.basename(p)
        if not bad:
            print(f"  {name:44s} {n:7d} edges, manifold")
            continue
        failed += 1
        hist = {}
        for c, _ in bad:
            hist[c] = hist.get(c, 0) + 1
        print(f"  {name:44s} {len(bad)} edges not shared by two faces ("
              + ", ".join(f"{k} on {c} faces" for c, k in sorted(hist.items())) + ")")
        for c, (x, y, z) in bad[:5]:
            print(f"      {c} faces at ({x:.3f}, {y:.3f}, {z:.3f})")
    if failed:
        sys.exit(f"FAIL: {failed} of {len(paths)} printed parts are not manifold")
    print(f"PASS: {len(paths)} printed parts, every edge on two faces")


if __name__ == "__main__":
    main()
