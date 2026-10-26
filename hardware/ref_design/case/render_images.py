#!/usr/bin/env python3
"""The docs' images (case/images/): the outside views, the case exploded in its build order, and sections.

    python3 case/render_images.py [name ...]      # from hardware/ref_design, after case/export.py --stl and case/fitkit.py

Names: outside_front outside_back outside_side outside_usb exploded_build (also written to fitkit/kit_assembly.png)
section_length section_screen section_slope section_seam section_key section_usb. No name: all of them.

The tub and the top come from case/tub.stl and case/top.stl (the top is turned back from its print pose into the
case frame: y -> -y, z -> case_h - z); the loose parts (silicone, caps, the board's slab, the cell, the speaker)
are rendered from case.scad's own modules; the board's parts are the boxes of fitkit_parts.scad (their 3D models'
envelopes) and the panel a box. Outside views and the exploded view are OpenSCAD renders; sections are trimesh
cuts drawn with matplotlib. Everything in mm, the case frame (origin at the case centre, z up from the tub's bottom).
"""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
import trimesh
from matplotlib.patches import PathPatch, Rectangle
from matplotlib.path import Path
from shapely.geometry import MultiLineString, Polygon
from shapely.ops import polygonize, unary_union

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "tools"))
import scad  # noqa: E402

IMG = os.path.join(HERE, "images")
POS = json.load(open(os.path.join(HERE, "positions.json")))
CASE_H = POS["case"]["h"]                       # 9.55
Z_BOARD, PCB_T = POS["case"]["z_board"], POS["board"]["t"]
Z_TOP = Z_BOARD + PCB_T                         # the board's top
SEAM = POS["case"]["seam"]
PANEL = (54.5, 83.0, 2.45)                     # case.scad: panel_w, panel_l, panel_t
Z_GLASS, TAPE = Z_TOP + 2.4, 0.15
SCAD = os.path.join(HERE, "case.scad")
COL = {"tub": "#3a3a44", "top": "#6d7482", "board": "#2e7d4f", "parts": "#8cc19c", "panel": "#101014", "cell": "#c9c9d1",
       "speaker": "#b8912e", "silicone": "#b4b7c0", "caps": "#f2f2f6", "j1": "#f08a3c"}
SIZE = 130                                      # dpi of the section figures


def run_scad(src, out, args=()):
    r = subprocess.run([*scad.cmd(out), "-o", out, *args, src], capture_output=True, text=True)
    if r.returncode or not os.path.exists(out):
        sys.exit(f"openscad failed for {src}:\n{r.stderr[-1500:]}")


def case_part_stls(tmp):
    """STLs of the loose parts, in the case frame, from case.scad's modules: {name: path}."""
    pieces = {"silicone": "silicone();", "caps": "caps();", "board": "board3d();", "cell": "cell3d();", "speaker": "speaker3d();"}
    out = {}

    def one(item):
        name, call = item
        src = os.path.join(tmp, name + ".scad")
        open(src, "w").write(f'include <{SCAD}>\n{call}\n')
        out[name] = os.path.join(tmp, name + ".stl")
        run_scad(src, out[name], ["-D", 'part="none"'])
    with ThreadPoolExecutor(scad.jobs()) as pool:
        list(pool.map(one, pieces.items()))
    return out


def kit_parts():
    """[(ref, x0, y0, x1, y1, z0, z1)] from fitkit_parts.scad: the board parts' boxes, z over the board's top."""
    text = open(os.path.join(HERE, "fitkit_parts.scad")).read()
    rows = re.findall(r'\["(\w+)", ([-\d.]+), ([-\d.]+), ([-\d.]+), ([-\d.]+), ([-\d.]+), ([-\d.]+)\]', text)
    return [(r[0], *map(float, r[1:])) for r in rows]


def top_in_case_frame():
    m = trimesh.load(os.path.join(HERE, "top.stl"))
    v = m.vertices.copy()
    v[:, 1] *= -1
    v[:, 2] = CASE_H - v[:, 2]
    return trimesh.Trimesh(v, m.faces[:, ::-1], process=False)


# ---------------------------------------------------------------- OpenSCAD views
def parts_scad(tmp):
    """The tub and the top in the case frame (the top turned back from its print pose)."""
    return (f'color("{COL["tub"]}") import("{HERE}/tub.stl");\n'
            f'color("#9a9aa6") translate([0, 0, {CASE_H}]) rotate([180, 0, 0]) import("{HERE}/top.stl");\n')


VIEWS = {   # name: (eye, centre, projection, size)
    "outside_front": ((-120, -175, 125), (0, 0, 3), "p", (1400, 1000)),
    "outside_back": ((120, 175, 125), (0, 0, 3), "p", (1400, 1000)),
    "outside_side": ((80, 0, 6), (0, 0, 5), "o", (1800, 300)),
    "outside_usb": ((100, 24, 26), (38, 2, 5.5), "p", (1200, 900)),
}


def render_outside(name, tmp):
    eye, c, proj, size = VIEWS[name]
    src = os.path.join(tmp, name + ".scad")
    open(src, "w").write(parts_scad(tmp))
    out = os.path.join(IMG, name + ".png")
    r = subprocess.run([scad.binary(), "--colorscheme=Tomorrow", f"--projection={proj}", f"--imgsize={size[0]},{size[1]}",
                        "--camera=" + ",".join(str(v) for v in (*eye, *c)), "-o", out, src], capture_output=True, text=True)
    if r.returncode or not os.path.exists(out):
        sys.exit(f"{name}: {r.stderr[-800:]}")


# the build order, face down (z' = case_h - z, y' = -y: the top lies on the bench, face on it): how far each part is lifted
LIFT = [("top", 0), ("caps", 16), ("silicone", 25), ("speaker", 35), ("panel", 48), ("board", 62), ("cell", 82), ("tub", 82), ("screws", 108)]


def render_exploded(tmp):
    stls = case_part_stls(tmp)
    # the panel and the screws as solids too
    open(os.path.join(tmp, "extra.scad"), "w").write(
        f'include <{SCAD}>\n'
        f'if (part == "none") {{ }}\n'
        f'translate([0, 0, z_glass]) linear_extrude(panel_t) square([panel_w, panel_l], center = true);\n')
    run_scad(os.path.join(tmp, "extra.scad"), os.path.join(tmp, "panel.stl"), ["-D", 'part="none"'])
    open(os.path.join(tmp, "screws.scad"), "w").write(
        f'include <{SCAD}>\n'
        f'for (s = pins) translate([s[0], s[1], 0]) {{ cylinder(d = 4, h = 1, $fn = 32); cylinder(d = 2, h = screw_cb[1] + screw_l, $fn = 24); }}\n')
    run_scad(os.path.join(tmp, "screws.scad"), os.path.join(tmp, "screws.stl"), ["-D", 'part="none"'])
    # the board's parts: boxes of their 3D models' envelopes
    boxes = "\n".join(f"translate([{p[1]}, {p[2]}, {Z_TOP + p[5]}]) cube([{p[3] - p[1]}, {p[4] - p[2]}, {p[6] - p[5]}]);"
                      for p in kit_parts() if p[6] - p[5] >= 0.4)
    body = []
    colors = {"top": "#9a9aa6", "caps": "#f2f2f6", "silicone": "#b4b7c0", "speaker": "#b8912e", "panel": "#101014",
              "board": "#2e7d4f", "cell": "#c9c9d1", "tub": "#3a3a44", "screws": "#a9afb8"}
    paths = {**stls, "panel": os.path.join(tmp, "panel.stl"), "screws": os.path.join(tmp, "screws.stl")}
    for name, lift in LIFT:
        if name == "top":
            body.append(f'color("{colors[name]}") import("{HERE}/top.stl");')
            continue
        src = f'import("{HERE}/tub.stl");' if name == "tub" else f'import("{paths[name]}");'
        if name == "board":
            src += f'\ncolor("#555a66") {{ {boxes} }}'
        body.append(f'translate([0, 0, {lift}]) translate([0, 0, {CASE_H}]) rotate([180, 0, 0]) color("{colors[name]}") {{ {src} }}')
    src = os.path.join(tmp, "exploded.scad")
    open(src, "w").write("\n".join(body) + "\n")
    out = os.path.join(IMG, "exploded_build.png")
    r = subprocess.run([scad.binary(), "--colorscheme=Tomorrow", "--projection=p", "--imgsize=1500,1100",
                        "--camera=-250,-230,160,0,0,58", "-o", out, src], capture_output=True, text=True)
    if r.returncode or not os.path.exists(out):
        sys.exit(f"exploded: {r.stderr[-800:]}")
    shutil.copy(out, os.path.join(HERE, "fitkit", "kit_assembly.png"))


# ---------------------------------------------------------------- sections
def cut(m, axis, pos):
    """The material of mesh m cut by the plane axis = pos (0: x, 1: y), as a shapely geometry in (the other horizontal axis, z);
    nested loops by the even-odd rule."""
    n, o = [0, 0, 0], [0, 0, 0]
    n[axis], o[axis] = 1, pos
    segs = trimesh.intersections.mesh_plane(m, n, o)
    if len(segs) == 0:
        return None
    h = 1 - axis
    lines = unary_union(MultiLineString([[(round(float(p[h]), 5), round(float(p[2]), 5)) for p in s] for s in segs]))
    polys = sorted(polygonize(lines), key=lambda p: -p.area)
    out = Polygon()
    for p in polys:
        out = out.symmetric_difference(Polygon(p.exterior))
    return out


def patch(ax, geom, color, **kw):
    if geom is None or geom.is_empty:
        return
    for p in (geom.geoms if hasattr(geom, "geoms") else [geom]):
        if p.is_empty or p.geom_type != "Polygon":
            continue
        verts, codes = [], []
        for ring in [p.exterior, *p.interiors]:
            xy = np.array(ring.coords)
            verts += list(xy)
            codes += [Path.MOVETO] + [Path.LINETO] * (len(xy) - 2) + [Path.CLOSEPOLY]
        ax.add_patch(PathPatch(Path(verts, codes), fc=color, ec=kw.pop("ec", "#1d1d24") if False else "#1d1d24", lw=0.6, **kw))


def box(ax, h0, h1, z0, z1, color, **kw):
    ax.add_patch(Rectangle((h0, z0), h1 - h0, z1 - z0, fc=color, ec="#1d1d24", lw=0.5, **kw))


class Scene:
    def __init__(self, tmp):
        self.tub = trimesh.load(os.path.join(HERE, "tub.stl"))
        self.top = top_in_case_frame()
        stls = case_part_stls(tmp)
        self.mesh = {k: trimesh.load(v) for k, v in stls.items()}
        self.parts = kit_parts()

    def draw(self, ax, axis, pos, hlim, highlight=("J1",)):
        """The whole stack cut at axis = pos, into ax (horizontal: the other axis)."""
        h = 1 - axis
        for name in ("board", "cell", "speaker", "silicone", "caps"):
            patch(ax, cut(self.mesh[name], axis, pos), COL[name])
        # the board's parts (their models' envelopes), drawn over the slab
        for ref, x0, y0, x1, y1, z0, z1 in self.parts:
            if z1 - z0 < 0.4:
                continue
            lo, hi = (x0, x1) if axis == 0 else (y0, y1)
            if not lo <= pos <= hi:
                continue
            a, b = (y0, y1) if axis == 0 else (x0, x1)
            box(ax, a, b, Z_TOP + z0, Z_TOP + z1, COL["j1"] if ref in highlight else COL["parts"], alpha=0.9)
        # the panel and its tape
        px = (PANEL[0] if axis == 1 else PANEL[1]) / 2
        if (axis == 0 and abs(pos) <= PANEL[0] / 2) or (axis == 1 and abs(pos) <= PANEL[1] / 2):
            box(ax, -px, px, Z_GLASS, Z_GLASS + PANEL[2], COL["panel"])
            box(ax, -px, px, Z_GLASS + PANEL[2], Z_GLASS + PANEL[2] + TAPE, "#d8c56a")
        patch(ax, cut(self.tub, axis, pos), COL["tub"])
        patch(ax, cut(self.top, axis, pos), COL["top"])
        ax.set_xlim(*hlim)


def figure(scene, axis, pos, hlim, ylim, title, size, notes=(), highlight=("J1",), xlabel=None, grid=2.0):
    fig, ax = plt.subplots(figsize=size)
    scene.draw(ax, axis, pos, hlim, highlight)
    ax.set_ylim(*ylim)
    ax.set_aspect("equal")
    step = 10 if hlim[1] - hlim[0] > 100 else 5 if hlim[1] - hlim[0] > 30 else 2 if hlim[1] - hlim[0] > 12 else 1
    ax.set_xticks(np.arange(np.ceil(hlim[0] / step) * step, hlim[1] + 1e-6, step))
    ax.set_yticks(np.arange(0, ylim[1], 2))
    ax.grid(alpha=0.25, lw=0.5)
    ax.set_xlabel(xlabel or f"{'y' if axis == 0 else 'x'} (mm, case frame)")
    ax.set_ylabel("z (mm)")
    ax.set_title(title, fontsize=10)
    for n in notes:
        n(ax)
    return fig


def label(text, xy, xytext, **kw):
    def f(ax):
        ax.annotate(text, xy=xy, xytext=xytext, fontsize=kw.pop("fontsize", 8), ha=kw.pop("ha", "center"),
                    arrowprops=dict(arrowstyle="-", color="#333", lw=0.7), bbox=dict(fc="white", ec="none", alpha=0.8, pad=1), **kw)
    return f


def vline(y, text, color="#b03030"):
    def f(ax):
        ax.axvline(y, color=color, lw=0.8, ls="--")
        ax.text(y, 0.2, text, rotation=90, va="bottom", ha="right", fontsize=7.5, color=color)
    return f


def save(fig, name):
    out = os.path.join(IMG, name + ".png")
    fig.savefig(out, dpi=SIZE, bbox_inches="tight")
    plt.close(fig)
    print(f"{name}.png {os.path.getsize(out) // 1024} kB")


def render_sections(names, tmp):
    scene = Scene(tmp)
    L = case_l = POS["case"]["l"]
    W = POS["case"]["w"]
    if "section_length" in names:
        notes = [label("DS Lite slope", (-33.5, 8.6), (-45, 10.4)), label("button end, 7.7 thick", (-60, 7.3), (-60, 10.4)),
                 label("panel", (8, 7.3), (14, 10.4)), label("cell in the board's hole", (10, 3), (14, -0.6)) if False else (lambda ax: None),
                 label("tub", (45, 0.8), (50, 10.0))]
        save(figure(scene, 0, -20.7, (-L / 2 - 1.5, L / 2 + 1.5), (-1, 11.6), "Along the case at x -20.7: seat 1's D-pad (left), seat 2's A/B/X/Y (right), the band between", (20, 4.2), notes), "section_length")
    if "section_screen" in names:
        notes = [label("panel", (-12, 7.4), (-18, 10.4)), label("cell in the board's hole, its bottom in the floor's pocket", (-5, 2.8), (-12, -0.8))]
        save(figure(scene, 1, 0, (-W / 2 - 1.5, W / 2 + 1.5), (-1.5, 11.6), "Across the case at y 0: the screen, the cell in the board's hole, both walls (the top's side rounds at the screen's height)", (14, 4.4), notes), "section_screen")
    if "section_slope" in names:
        notes = [vline(-30.53, "top edge, y 30.5"), vline(-36.22, "foot, y 36.2"),
                 label("step 1.85: screen face 9.55\nto the buttons' end 7.7", (-33.4, 8.5), (-47, 10.9)),
                 label("one concave arc, R 9.69\n(the DS Lite's R 8.0 scaled 1.21),\n36 degrees at the top edge", (-34.5, 8.2), (-47, 13.1))]
        save(figure(scene, 0, -20.7, (-67.5, -27), (-1, 14.8), "Seat 1's end at x -20.7: the DS Lite slope between the screen face and the buttons' end", (11.5, 5.2), notes, grid=1.0), "section_slope")
    wall = (28.5, 43)
    ylim = (-3.4, 10.8)
    if "section_seam" in names:
        notes = [label("tongue (the tub's wall's inner half)", (37.0, 4.8), (40.4, 9.8)),
                 label("skirt of the top,\noutside the tongue", (38.3, 4.2), (40.6, 6.6)),
                 label("seam, z 2.55", (38.0, SEAM), (40.6, 0.8)), label("crush rib (a lead-in at its top)", (36.4, 4.1), (35.2, -2.6)),
                 label("board edge", (36.2, 3.0), (30.6, -2.6)), label("back's round", (37.3, 1.0), (40.0, -1.4)),
                 label("J4's envelope", (32, 6.5), (30.6, 9.7))]
        save(figure(scene, 1, -28, wall, ylim, "Across the right wall at y -28 (a crush rib, no key): the top's skirt and groove over the tub's tongue, the seam low on the back's round", (9, 6.9), notes, grid=1.0), "section_seam")
    if "section_key" in names:
        notes = [label("key on the tongue", (37.0, 6.2), (33.2, 9.4)), label("its pocket in the top", (37.4, 6.4), (40.6, 9.6)),
                 label("seam, z 2.55", (38.0, SEAM), (40.6, 0.8)), label("board edge", (36.2, 3.0), (31.2, -2.4))]
        save(figure(scene, 1, 21, wall, ylim, "Across the right wall at y 21: a key on the tub's tongue (5 wide, 1.1 higher) in its pocket in the top", (9, 6.9), notes, grid=1.0), "section_key")
    if "section_usb" in names:
        notes = [label("J1 (its model's envelope)", (34.5, 5.2), (31.0, 9.4)), label("USB-C opening, through the wall", (38.0, 5.4), (40.0, 9.6)),
                 label("tub's tab, the opening's lower half", (37.5, 3.3), (34.5, -2.4)), label("seam, z 2.55", (38.4, SEAM), (40.6, 0.8))]
        save(figure(scene, 1, 2, wall, ylim, "Across the right wall at y 2 through the USB-C opening: the tub's tab below, the top's notch above", (9, 6.9), notes, grid=1.0), "section_usb")


def main():
    names = sys.argv[1:] or [*VIEWS, "exploded_build", "section_length", "section_screen", "section_slope", "section_seam", "section_key", "section_usb"]
    os.makedirs(IMG, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        jobs = [n for n in names if n in VIEWS]
        with ThreadPoolExecutor(min(4, scad.jobs())) as pool:
            list(pool.map(lambda n: render_outside(n, tmp), jobs))
        for n in jobs:
            print(n + ".png")
        if "exploded_build" in names:
            render_exploded(tmp)
            print("exploded_build.png, fitkit/kit_assembly.png")
        secs = [n for n in names if n.startswith("section_")]
        if secs:
            render_sections(secs, tmp)


if __name__ == "__main__":
    main()
