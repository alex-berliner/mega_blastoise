#!/usr/bin/env python3
"""Side sections through the whole handheld, every part in place, as SVG.

    python3 case/sections.py OUT_DIR NAME=TUB.stl,LID.stl[,face_t=F][,j2_h=H] ...

Each NAME gets two cuts, along the case's length (seat 1 on the left):
  NAME_spine.svg   through the centre: J2 and its plug, the cell, the speaker, the panel and J3
  NAME_dpad.svg    through seat 1's D-pad (and seat 2's A/B/X/Y): the button stack and the ridge

The tub and lid are sliced from STLs rendered from case.scad in the assembly frame (z = 0 at the
tub's bottom); the board, its parts (fitkit_parts.scad: every part's 3D-model envelope), the
panel, cell, speaker, silicone and caps are drawn from the same numbers case.scad uses. Heights
and lengths share one scale. Fills are CSS classes, so a page can theme them.
"""
import json
import os
import re
import sys

import numpy as np
import shapely
import trimesh
from shapely.geometry import LineString, Point, Polygon, box
from shapely.ops import polygonize, unary_union

HERE = os.path.dirname(os.path.abspath(__file__))
POS = json.load(open(os.path.join(HERE, "positions.json")))
S = 9.0                                   # px per mm (set per drawing)

# ---- numbers from case.scad (the stack) ----
floor_t, cell_t, cell_gap, pcb_t = 1.5, 4.0, 0.5, 1.6
glass_gap, panel_t, tape_t, lid_t = 2.4, 2.45, 0.15, 2.0
z_board = floor_t + cell_t + cell_gap
z_top = z_board + pcb_t
panel_w, panel_l = 54.5, 83.0
cell = (34, 50, cell_y := -17)
spk = ((0, 26), 15, 4.0)
board_w, board_l, board_r = POS["board"]["w"], POS["board"]["l"], POS["board"]["r"]
dome_h, flange_t, cap_h, cap_d, flange_d = 1.75, 0.55, 3.56, 7.0, 8.8
dpad_l, dpad_w, dpad_h, dpad_fr = 17.8, 5.9, 3.34, 6.5
piece_d = POS["piece_d"]


def kicad_to_case(p):
    return (p[0] - 100, 100 - p[1])


CLUSTERS = [kicad_to_case(p) for p in POS["pieces"]]          # dpad1, abxy1, dpad2, abxy2
BUTTONS = {k: kicad_to_case(v) for k, v in POS["buttons"].items()}
SLOT = [kicad_to_case(p) for p in POS["slot"]]
BATT = kicad_to_case(POS["batt_conn"])


def kit_parts():
    text = open(os.path.join(HERE, "fitkit_parts.scad")).read()
    rows = re.findall(r'\["(\w+)", ([-\d.]+), ([-\d.]+), ([-\d.]+), ([-\d.]+), ([-\d.]+), ([-\d.]+)\]', text)
    return [(r[0], *map(float, r[1:])) for r in rows]


def board_outline():
    b = box(-board_w / 2 + board_r, -board_l / 2 + board_r, board_w / 2 - board_r, board_l / 2 - board_r).buffer(board_r, 32)
    (x0, y0), (x1, y1) = SLOT
    hole = box(min(x0, x1) + 0.6, min(y0, y1) + 0.6, max(x0, x1) - 0.6, max(y0, y1) - 0.6).buffer(0.6, 8)
    return b.difference(hole)


def cut_2d(shape2d, x, z0, z1):
    """Section at x of shape2d extruded from z0 to z1: rectangles in (y, z)."""
    line = LineString([(x, -500), (x, 500)])
    hit = shape2d.intersection(line)
    out = []
    for g in getattr(hit, "geoms", [hit]):
        if g.is_empty or g.length == 0:
            continue
        ys = [c[1] for c in g.coords]
        out.append(box(min(ys), z0, max(ys), z1))
    return out


def slice_mesh(path, x):
    """Section of an STL at plane x, as shapely polygons in (y, z), holes kept (even-odd)."""
    mesh = trimesh.load(path, force="mesh")
    segs = trimesh.intersections.mesh_plane(mesh, [1, 0, 0], [x, 0, 0])
    lines = [LineString([(round(a[1], 4), round(a[2], 4)), (round(b[1], 4), round(b[2], 4))])
             for a, b in segs if np.linalg.norm(a - b) > 1e-6]
    faces = list(polygonize(unary_union(lines)))
    shells = [Polygon(f.exterior) for f in faces]
    solid = []
    for f in faces:
        p = f.representative_point()
        if sum(1 for s in shells if s.contains(p)) % 2 == 1:
            solid.append(f)
    return unary_union(solid)


def svg_poly(geom, cls, extra=""):
    out = []
    for g in getattr(geom, "geoms", [geom]):
        if g.is_empty or g.geom_type != "Polygon":
            continue
        d = ""
        for ring in [g.exterior, *g.interiors]:
            pts = list(ring.coords)
            d += "M" + " L".join(f"{X(y):.1f},{Z(z):.1f}" for y, z in pts) + " Z "
        out.append(f'<path class="{cls}" d="{d}" fill-rule="evenodd"{extra}/>')
    return out


Y0, Y1, ZTOP, ZBOT = -80.0, 80.0, 17.5, -1.0     # drawing window (set per drawing)


def X(y):
    return (y - Y0) * S


def Z(z):
    return (ZTOP - z) * S


def section(x, tub, lid, face_t, j2_h, title):
    case_h = z_top + glass_gap + panel_t + tape_t + face_t
    parts = []
    W = (Y1 - Y0) * S
    H = (ZTOP - ZBOT) * S
    # tub and lid
    parts += svg_poly(slice_mesh(tub, x), "tub")
    parts += svg_poly(slice_mesh(lid, x), "lid")
    # cell and speaker on the floor
    cell_shape = box(-cell[0] / 2, cell_y - cell[1] / 2, cell[0] / 2, cell_y + cell[1] / 2)
    for r in cut_2d(cell_shape, x, floor_t, floor_t + cell_t):
        parts += svg_poly(r, "cell")
    for r in cut_2d(Point(*spk[0]).buffer(spk[1] / 2, 64), x, floor_t, floor_t + spk[2]):
        parts += svg_poly(r, "spk")
    # board
    for r in cut_2d(board_outline(), x, z_board, z_top):
        parts += svg_poly(r, "board")
    # panel and its tape
    pshape = box(-panel_w / 2, -panel_l / 2, panel_w / 2, panel_l / 2)
    for r in cut_2d(pshape, x, z_top + glass_gap, z_top + glass_gap + panel_t):
        parts += svg_poly(r, "panel")
    for r in cut_2d(pshape, x, z_top + glass_gap + panel_t, z_top + glass_gap + panel_t + tape_t):
        parts += svg_poly(r, "tape")
    # the panel's FPC: round the seat-1 edge, back under the panel to J3
    j3 = next(p for p in kit_parts() if p[0] == "J3")
    if j3[1] < x < j3[3]:
        e = -panel_l / 2
        zb = z_top + glass_gap
        fpc = [(e + 1.0, zb + panel_t - 0.4), (e - 0.6, zb + panel_t - 0.6), (e - 1.0, zb + 0.8), (e - 0.5, zb - 0.25),
               (e + 2.0, zb - 0.35), (j3[2] + 0.4, z_top + 1.0)]
        parts.append('<polyline class="fpc" points="' + " ".join(f"{X(a):.1f},{Z(b):.1f}" for a, b in fpc) + '"/>')
    # silicone pieces and caps
    for i, c in enumerate(CLUSTERS):
        for r in cut_2d(Point(*c).buffer(piece_d / 2, 64), x, z_top, z_top + dome_h * 0.55):
            parts += svg_poly(r, "sil")
    z_fl = z_top + dome_h
    for i, c in enumerate(CLUSTERS):
        if i in (0, 2):                    # D-pads: the cross and its flange
            cross = unary_union([box(c[0] - dpad_w / 2, c[1] - dpad_l / 2, c[0] + dpad_w / 2, c[1] + dpad_l / 2),
                                 box(c[0] - dpad_l / 2, c[1] - dpad_w / 2, c[0] + dpad_l / 2, c[1] + dpad_w / 2)])
            for r in cut_2d(unary_union([cross, Point(*c).buffer(dpad_fr, 48)]), x, z_fl, z_fl + flange_t):
                parts += svg_poly(r, "cap")
            for r in cut_2d(cross, x, z_fl, z_fl + dpad_h):
                parts += svg_poly(r, "cap")
        else:
            a = 7.5
            for d in ((a, 0), (-a, 0), (0, a), (0, -a)):
                p = Point(c[0] + d[0], c[1] + d[1])
                for r in cut_2d(p.buffer(flange_d / 2, 48), x, z_fl, z_fl + flange_t):
                    parts += svg_poly(r, "cap")
                for r in cut_2d(p.buffer(cap_d / 2, 48), x, z_fl, z_fl + cap_h):
                    parts += svg_poly(r, "cap")
    # the board's parts (top side), J2 at its new place and height
    labels = []
    for ref, x0, y0, x1, y1, z0, z1 in kit_parts():
        if ref == "J2":
            dy = BATT[1] - (-52.82)
            y0, y1, z1 = y0 + dy, y1 + dy, j2_h
        if x0 <= x <= x1:
            cls = "j2" if ref == "J2" else "part"
            parts += svg_poly(box(y0, z_top + max(z0, 0), y1, z_top + z1), cls)
            if ref == "J2":
                labels.append(("J2", (y0 + y1) / 2, z_top + z1 / 2 - 0.3, "lbl on"))
            elif ref in NAMES and Y0 + 4 < (y0 + y1) / 2 < Y1 - 4:
                labels.append((NAMES[ref], (y0 + y1) / 2, z_top + z1 + 0.5, "lbl"))
        if ref == "J2":
            j2 = (y0, y1, z1)
    # the cell's plug in J2 and its wires down through the hole to the cell
    if -3.0 <= x <= 3.0:
        front = j2[1]
        plug = box(front, z_top + 0.4, front + 2.0, z_top + min(4.9, j2[2] - 0.5))
        parts += svg_poly(plug, "plug")
        sy = (SLOT[0][1] + SLOT[1][1]) / 2
        wire = [(front + 2.0, z_top + 2.4), (front + 3.2, z_top + 1.6), (sy + 1.0, z_top - 0.8),
                (sy + 1.4, floor_t + 0.6), (cell_y - cell[1] / 2 - 0.5, floor_t + 0.6), (cell_y - cell[1] / 2, floor_t + 1.5)]
        parts.append('<polyline class="wire" points="' + " ".join(f"{X(a):.1f},{Z(b):.1f}" for a, b in wire) + '"/>')
    # names on the big parts, and the heights that matter
    def name(text, y, z, cls="lbl"):
        if Y0 + 2 < y < Y1 - 2:
            labels.append((text, y, z, cls))
    if abs(x) <= cell[0] / 2:
        name("cell", cell_y, floor_t + 1.4, "lbl on")
    if abs(x - spk[0][0]) <= spk[1] / 2:
        name("speaker", spk[0][1], floor_t + 1.4, "lbl on")
    if abs(x) <= panel_w / 2:
        name("panel", -10 if Y1 > 0 else (Y0 + Y1) / 2 + 30, z_top + glass_gap + 0.8, "lbl on")
    name("board", Y0 + 8 if Y1 - Y0 > 100 else -26, z_board + 0.35, "lbl on")
    for i in (0, 2):
        if abs(x - CLUSTERS[i][0]) < dpad_w / 2:
            name("D-pad", CLUSTERS[i][1], z_top + dome_h + 1.2)
    deck = z_top + dome_h + cap_h - 1.85
    dims = []
    if x != 0.0:
        dims.append(("ridge", -48.0, deck, case_h))
    if -3.0 <= x <= 3.0 and j2[2] + 0.2 + 0.6 > case_h - z_top:
        dims.append(("pad", BATT[1], case_h, z_top + j2[2] + 0.8))
    return W, H, parts, labels, case_h, dims


NAMES = {"J2": "J2", "J3": "J3", "U4": "RP2354B", "U7": "amp", "J1": "USB-C",
         "U2": "charger", "U6": "backlight", "J5": "SWD", "SW18": "BOOT"}


def write(path, x, tub, lid, face_t, j2_h, title, window, scale):
    global Y0, Y1, S
    (Y0, Y1), S = window, scale
    W, H, parts, labels, case_h, dims = section(x, tub, lid, face_t, j2_h, title)
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W:.0f} {H:.0f}" role="img" aria-label="{title}">']
    out += [f'<clipPath id="win"><rect width="{W:.0f}" height="{H:.0f}"/></clipPath><g clip-path="url(#win)">'] + parts + ["</g>"]
    for text, y, z, cls in labels:
        dy = 0 if "on" in cls else -3
        out.append(f'<text class="{cls}" x="{X(y):.1f}" y="{Z(z) + dy:.1f}" text-anchor="middle">{text}</text>')
    for kind, y, za, zb in dims:
        if not Y0 < y < Y1:
            continue
        xa = X(y)
        if kind == "ridge":
            out.append(f'<line class="dimx" x1="{X(Y0):.1f}" y1="{Z(zb):.1f}" x2="{X(-44):.1f}" y2="{Z(zb):.1f}"/>')
            out.append(f'<line class="dim" x1="{xa:.1f}" y1="{Z(za):.1f}" x2="{xa:.1f}" y2="{Z(zb):.1f}"/>')
            out.append(f'<text class="dimt" x="{xa - 6:.1f}" y="{Z(zb) - 6:.1f}" text-anchor="end">ridge {zb - za:.2f} mm</text>')
        else:
            out.append(f'<text class="dimt" x="{xa + 6.5 * S:.1f}" y="{Z(zb) - 4:.1f}" text-anchor="start">pad +{zb - za:.1f} mm over J2</text>')
    out.append("</svg>")
    open(path, "w").write("\n".join(out))
    return case_h


if __name__ == "__main__":
    outdir = sys.argv[1]
    os.makedirs(outdir, exist_ok=True)
    meta = {}
    for spec in sys.argv[2:]:
        name, rest = spec.split("=", 1)
        bits = rest.split(",")
        tub, lid = bits[0], bits[1]
        kw = dict(b.split("=") for b in bits[2:])
        face_t = float(kw.get("face_t", 2.0))
        j2_h = float(kw.get("j2_h", 5.5))
        meta[name] = {}
        for cut, x, what in (("spine", 0.0, "the centre, through J2"), ("dpad", CLUSTERS[0][0], "seat 1's D-pad")):
            for view, window, scale in (("full", (-80.0, 80.0), 5.2), ("end", (-80.0, -18.0), 13.5)):
                meta[name][cut] = write(os.path.join(outdir, f"{name}_{cut}_{view}.svg"), x, tub, lid, face_t, j2_h,
                                        f"{name}: section through {what}", window, scale)
        print(name, "done")
    json.dump(meta, open(os.path.join(outdir, "meta.json"), "w"))
