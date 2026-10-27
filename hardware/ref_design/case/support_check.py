#!/usr/bin/env python3
"""Where did the slicer put supports? Read from a sliced Bambu Studio or Snapmaker Orca project's own G-code.

    python3 case/support_check.py plate.3mf [object ...] [--designed object ...]
        # objects that must get none; objects that may have support filament under them (the top and its coupons)

The one-piece top prints on support (2026-10-25): tree supports with the interface layers, the ones that touch the
part, in the second filament (Bambu Support For PLA) and the body in PLA. So besides "no support inside the outline of a
part that needs none" it checks the support filament: it may print only "Support interface" and the prime tower (the
purge and the start moves aside), and only inside the outlines of the parts it is designed for (the top and the coupons
cut from it). That is the top's designed support; any other part with support fails.

For each object on the plate (its mesh placed by the project's own transforms, then moved onto
its own first layer in the G-code: Bambu Studio's G-code sits 2.0 mm off the project's placement
in y on a P1S, so no offset is assumed), the support and support interface extruded inside its outline, layer by layer, and
for the tub (and its screw holes coupon) every counterbore found in its own first millimetre. print_package_p1s.py calls
check() on every plate and fails the build if a part that needs no supports got some: the parts
overhangs.py passes at case.scad's `overhang` (60 degrees from vertical on the 60 degree variant).
"""
import io
import re
import sys
import zipfile

import numpy as np
import shapely
import trimesh
from shapely.geometry import MultiPoint, Polygon

SUPPORT = ("Support", "Support interface", "Support transition")
SUPPORT_TOOL = 1                       # the second filament (T1): Bambu Support For PLA
SUPPORT_TOOL_FEATURES = {"Support interface", "Prime tower", "Flush", "Custom", "Undefined"}   # all it may print
TOOL = re.compile(r"^T(\d+)\s*$")
WORD = re.compile(r"([XYZEIJ])(-?[\d.]+)")


def extrusions(gcode):
    """[(feature, z, points (n, 2), tool)] for every extruding move: G1 lines, and G2/G3 arcs sampled
    along the arc. Bambu writes relative E (M83), absolute XY, ; Z_HEIGHT: per layer and Tn for the filament."""
    out, x, y, z, feature, tool = [], 0.0, 0.0, 0.0, "", 0
    for line in gcode.splitlines():
        m = TOOL.match(line)
        if m:
            tool = int(m.group(1))
            continue
        if line.startswith(";"):
            if line.startswith("; FEATURE:"):
                feature = line[10:].strip()
            elif line.startswith("; Z_HEIGHT:"):
                z = float(line[11:])
            elif line.startswith(";TYPE:"):           # Snapmaker Orca's: ;TYPE: per feature, ;Z: per layer
                feature = line[6:].strip()
            elif line.startswith(";Z:"):
                z = float(line[3:])
            continue
        cmd = line.split(";")[0].split()
        if not cmd or cmd[0] not in ("G0", "G1", "G2", "G3"):
            continue
        w = dict(WORD.findall(line.split(";")[0]))
        nx, ny = float(w.get("X", x)), float(w.get("Y", y))
        if float(w.get("E", 0)) > 0 and ((nx, ny) != (x, y) or cmd[0] != "G1"):
            if cmd[0] in ("G2", "G3"):
                cx, cy = x + float(w.get("I", 0)), y + float(w.get("J", 0))
                a0, a1 = np.arctan2(y - cy, x - cx), np.arctan2(ny - cy, nx - cx)
                if cmd[0] == "G3" and a1 <= a0:
                    a1 += 2 * np.pi
                if cmd[0] == "G2" and a1 >= a0:
                    a1 -= 2 * np.pi
                r = np.hypot(x - cx, y - cy)
                t = np.linspace(a0, a1, max(2, int(abs(a1 - a0) * r / 0.1) + 1))
                pts = np.c_[cx + r * np.cos(t), cy + r * np.sin(t)]
            else:
                n = max(2, int(np.hypot(nx - x, ny - y) / 0.1) + 1)
                pts = np.c_[np.linspace(x, nx, n), np.linspace(y, ny, n)]
            out.append((feature, z, pts, tool))
        x, y = nx, ny
    return out


def objects(z):
    """{name: mesh on the plate} from a Bambu project: each object's mesh moved by its component's
    and its build item's transforms."""
    model = z.read("3D/3dmodel.model").decode()
    cfg = z.read("Metadata/model_settings.config").decode()
    names = {oid: re.search(r'key="name" value="([^"]+?)(?:\.stl)?"', blk).group(1)
             for oid, blk in re.findall(r'<object id="(\d+)">(.*?)</object>', cfg, re.S)}
    items = dict(re.findall(r'<item objectid="(\d+)"[^>]*transform="([^"]+)"', model))
    out = {}
    for oid, body in re.findall(r'<object id="(\d+)"[^>]*>\s*<components>(.*?)</components>', model, re.S):
        for path, t in re.findall(r'<component p:path="([^"]+)"[^>]*transform="([^"]+)"', body):
            part = z.read(path.lstrip("/")).decode()
            v = np.array(re.findall(r'<vertex x="([^"]+)" y="([^"]+)" z="([^"]+)"', part), float)
            f = np.array(re.findall(r'<triangle v1="(\d+)" v2="(\d+)" v3="(\d+)"', part), int)
            m = trimesh.Trimesh(v, f, process=False)
            for tr in (t, items.get(oid, "1 0 0 0 1 0 0 0 1 0 0 0")):
                a = np.array(tr.split(), float)
                mat = np.eye(4)
                mat[:3, :3], mat[:3, 3] = a[:9].reshape(3, 3).T, a[9:]
                m.apply_transform(mat)
            out.setdefault(names.get(oid, oid), []).append(m)
    return {n: trimesh.util.concatenate(ms) for n, ms in out.items()}


def register(objs, moves):
    """Move each object's mesh onto its first layer in the G-code: the centre of the outer wall
    extruded round it (within 3 mm of its outline) against its own section's centre. (Not the
    start G-code's purge lines: they're "Custom".)"""
    import overhangs
    z0 = min(zz for f, zz, *_ in moves if f == "Outer wall")
    first = np.concatenate([p for f, zz, p, _ in moves if zz == z0 and f == "Outer wall"])
    for name, m in objs.items():
        b = np.array(overhangs._slice(m, m.bounds[0][2] + z0 / 2).bounds)
        near = MultiPoint(m.vertices[:, :2]).convex_hull.buffer(3)
        q = first[shapely.contains_xy(near, first[:, 0], first[:, 1])]
        if len(q) and len(b) == 4:
            m.apply_translation([*((q.min(0) + q.max(0)) / 2 - (b[:2] + b[2:]) / 2), 0])
    return objs


def counterbores(mesh, z=0.5, d=(3.5, 6.0)):
    """The holes in the mesh's section z above its bottom whose size is a counterbore's."""
    import overhangs
    sec = overhangs._slice(mesh, mesh.bounds[0][2] + z)
    polys = list(sec.geoms) if hasattr(sec, "geoms") else [sec]
    holes = [Polygon(r) for p in polys for r in p.interiors]
    return [h for h in holes if d[0] < 2 * np.sqrt(h.area / np.pi) < d[1]]


def check(path, no_support, designed=(), moves=None):
    """Support extrusion inside each object's outline, and inside each of the tub's counterbores:
    (lines of a report, [no_support objects that got some], {object: mm of support in its outline},
    [problems with the support filament's use: it prints only interface and the tower, and only under `designed`])."""
    z = zipfile.ZipFile(path)
    gcode = z.read("Metadata/plate_1.gcode").decode()
    moves = moves if moves is not None else extrusions(gcode)    # (already read: slice_u1 reads them for the bed too)
    objs = register(objects(z), moves)
    sup = [(f, zz, p) for f, zz, p, _ in moves if f in SUPPORT]
    pts = np.concatenate([p for *_, p in sup]) if sup else np.zeros((0, 2))
    zs = np.concatenate([[zz] * len(p) for _, zz, p in sup]) if sup else np.zeros(0)
    import overhangs
    report, bad, mm = [f"{len(pts) * 0.1:.0f} mm of support extrusion on the plate (the parts that need none: "
                       f"no face flatter than {overhangs.OVERHANG:g} degrees from vertical, overhangs.py)"], [], {}
    # the support filament: only its interface and the prime tower; and the interface only in it
    printed = {}
    for f, zz, p, tool in moves:
        printed.setdefault(tool, {})
        printed[tool][f] = printed[tool].get(f, 0.0) + 0.1 * len(p)
    filament = []
    for tool, features in sorted(printed.items()):
        report.append(f"  filament {tool + 1} prints " + ", ".join(f"{f} {mmv:.0f} mm" for f, mmv in sorted(features.items())))
    for f in printed.get(SUPPORT_TOOL, {}):
        if f not in SUPPORT_TOOL_FEATURES:
            filament.append(f"the support filament prints {f}")
    own = [p for f, zz, p, tool in moves if tool == SUPPORT_TOOL and f == "Support interface"]
    own = np.concatenate(own) if own else np.zeros((0, 2))
    for name, m in sorted(objs.items()):
        outline = MultiPoint(m.vertices[:, :2]).convex_hull.buffer(-0.3)
        inside = shapely.contains_xy(outline, pts[:, 0], pts[:, 1]) if len(pts) else np.zeros(0, bool)
        must_not = name in no_support
        # beside it: the support's trunks and the parts of it under an overhang past the outline
        beside = (shapely.contains_xy(outline.buffer(5.3), pts[:, 0], pts[:, 1]) & ~inside) if len(pts) else inside
        mm[name] = (inside.sum() + beside.sum()) * 0.1
        layers = sorted({float(v) for v in np.round(zs[inside], 2)})
        report.append(f"  {name:28s} {'no supports' if must_not else 'may have supports':18s} "
                      f"{inside.sum() * 0.1:7.1f} mm inside its outline, {beside.sum() * 0.1:5.1f} within 5 mm of it"
                      + (f" (layers {layers})" if layers else ""))
        if must_not and inside.any():
            bad.append(name)
        if name not in designed and len(own) and shapely.contains_xy(outline, own[:, 0], own[:, 1]).any():
            filament.append(f"the support filament prints inside {name}'s outline, which has none of its own")
        if name.startswith("tub") or "screws_tub" in name:      # the tub, and its screw holes coupon
            for i, h in enumerate(counterbores(m)):
                c = h.centroid
                n = shapely.contains_xy(h.buffer(0.2), pts[:, 0], pts[:, 1]).sum() if len(pts) else 0
                report.append(f"    counterbore {i + 1} at ({c.x:6.1f}, {c.y:6.1f}), "
                              f"{2 * np.sqrt(h.area / np.pi):.2f} across: {n * 0.1:.1f} mm of support")
    return report, bad, mm, filament


if __name__ == "__main__":
    args = sys.argv[2:]
    designed = set(args[args.index("--designed") + 1:]) if "--designed" in args else set()
    lines, bad, _, filament = check(sys.argv[1], set(args[:args.index("--designed")] if "--designed" in args else args), designed)
    print("\n".join(lines))
    if bad or filament:
        sys.exit("; ".join(([f"support on {', '.join(bad)}"] if bad else []) + filament))
