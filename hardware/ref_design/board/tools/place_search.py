"""Placement search for the board: zones (macro), nudges (fine), a quick score instead of routing, then routing of the best.

    python3 tools/place_search.py score [picks-json]        # one placement (default: home): validity and score
    python3 tools/place_search.py stage1 --out DIR [--keep 40] [--jobs 16]   # zone assignments, scored; DIR/assign.json
    python3 tools/place_search.py stage2 --out DIR [--top 40] [--rounds 250] [--jobs 16]   # nudges; DIR/candidates.json
    python3 tools/place_search.py route DIR --hosts il:8,desktop-ubuntu:8,xps13:4 [--top 40] [--skip K] [--seeds 1] [--run NAME]
    python3 tools/place_search.py report DIR                # routed results against the quick score
    python3 tools/place_search.py stop [--hosts ...]        # kill a route run's processes, by PID, on every host
    python3 tools/place_search.py measure BOARD.kicad_pcb   # (on a host) DRC errors, unconnected, track length: JSON
    python3 tools/place_search.py zones-png OUT.png         # the zones and which group may use which
    python3 tools/place_search.py preflight [picks-json]    # build the picks into a scratch folder, run route.py's pre-flight DRC

The zones and each group's allowed zones, turns and nudge limits are data in board/placement_zones.py; placement.py's own
table is every group's home, and the search's result is board/picks.json ({"poses", "core", "hole", "usb_dy"}).

Stage 1: a candidate assigns each group to one of its zones (and a turn). `realize` puts it on the built board in memory
(pcbnew), no board is built per candidate: the core group's rigid set (U4 and the reference's parts) first, then each
other group: its parts go where they sit at home relative to the group's anchor (the zone's centre, then the nearest anchor
on a grid in the zone that works), each moved to the nearest legal spot (a ring search: courtyards, copper 0.31 from the
outline, the cutouts and NPTH holes). Assignments are enumerated in steps (core x charger x usb, then the latch, the rest
by greedy zone choice) and the best are kept.
Stage 2: per kept assignment, a local search: shifts of 0.5 to 2 mm and quarter turns of a group or one part, accepted
when still valid and the score is lower; limited to NUDGE_MAX from where the zone put it.
Validity: new courtyard overlaps, copper to the edge/cutouts/NPTH, nothing inside the (shifted) hole, the case's height
and zone rules (case/check_fit.py check_part on each moved part's cached 3D extent).
Score (lower is better): the nets that must cross each bottleneck (the gap between the hole and J3, the strips beside the
hole, the board's top end), weighted by track width, against what fits (width / pitch, 2 layers, less pours and vias); the
ratsnest length (minimum spanning trees round the hole, the core and the speaker's hole); a congestion count on a grid.
route: copies the board folder to ~/mb_route on each host (rsync), then per candidate build_pcb.py + route.py (one attempt,
its own seed), a few at a time per host; boards come back under DIR/routed.
"""
import json
import math
import os
import random
import shutil
import subprocess
import sys
import tempfile
import time

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD_DIR = os.path.dirname(HERE)
REF = os.path.dirname(BOARD_DIR)
CASE_DIR = os.path.join(REF, "case")
sys.path.insert(0, HERE)
sys.path.insert(0, BOARD_DIR)
sys.path.insert(0, CASE_DIR)

PITCH = 0.45                 # track + clearance incl. the vias' share, mm
LAYERS_USABLE = 2 * 0.6      # two layers, less the pours, pre-routes and vias
COPPER_EDGE = 0.31           # copper to the outline, the cutouts and NPTH holes
TOL = 0.0                   # a pad's box is a little bigger than its shape: the fast checks allow this much
GRID = 1.5
NUDGE_MAX = 4.0              # a nudged group stays within this of where its zone put it (mm)
NUDGE_STEPS = (0.5, 1.0, 1.5, 2.0)
ESCAPE = 4.4                 # the core's copied escape tracks run this far out of U4's pads: they must stay inside the board's edge
EDGE_R = 5.5                 # the board's corner radius
SPEAKER_STEP_NOTE = "4 mm grid, long side along the board"
STAGES = ("speaker", "core", "cellconn", "usb", "charger", "latch", "ldo", "j5", "boot", "misc")    # stage 1's beam, in order (the small groups, vsense and J4 are nudged in stage 2; the hole shifts need a case respin)
# power and high-current nets (and the charger's sensitive programming nets): their width-weighted length is a term of its own, and
# the charger is kept near J1 (VBUS) and J2 (VBAT), the LDO near the core, by a soft penalty beyond these distances (mm)
POWER_NETS = {"VBUS", "VBAT", "VSYS", "/USB_Charger/VBUS_C1", "/USB_Charger/CELL_P", "/Power/3V3_LDO", "/Display/LED_A", "/Display/LED_K1",
              "/Display/LED_K2", "/Audio/SPK_P", "/Audio/SPK_N", "/USB_Charger/CHG_ISET", "/USB_Charger/CHG_ILIM"}
POWER_W = 0.5                # weight of sum(width x length) over POWER_NETS
SOFT = (("U2", "J1", 15.0), ("U2", "J2", 15.0), ("U3", "U4", 20.0))
SOFT_W = 8.0                 # score per mm beyond
BOTTLENECKS = ("chan", "left", "right", "top")


def rot_box(b, deg):
    """Box (x0, y0, x1, y1) about the origin turned by deg (multiple of 90, CCW on screen, y down)."""
    d = deg % 360
    if d == 0:
        return b
    pts = [(b[0], b[1]), (b[2], b[1]), (b[2], b[3]), (b[0], b[3])]
    a = math.radians(d)
    c, s = round(math.cos(a)), round(math.sin(a))
    q = [(x * c + y * s, -x * s + y * c) for x, y in pts]
    xs, ys = [p[0] for p in q], [p[1] for p in q]
    return (min(xs), min(ys), max(xs), max(ys))


def rot_pt(p, deg):
    d = math.radians(deg % 360)
    c, s = round(math.cos(d)), round(math.sin(d))
    return (p[0] * c + p[1] * s, -p[0] * s + p[1] * c)


# ---------------------------------------------------------------------------------------------------------- context
class Ctx:
    """The built board of the home placement, in memory, and what the checks need from it."""

    def __init__(self, pcb, cache_dir):
        import pcbnew
        import placement as pl
        import placement_zones as pz
        from shapely.geometry import Polygon, box
        self.pcbnew, self.pl, self.pz, self.box = pcbnew, pl, pz, box
        self.pcb = pcb
        self.board = pcbnew.LoadBoard(pcb)
        self.fp = {f.GetReference(): f for f in self.board.GetFootprints()}
        self.home_P = dict(pl.P)
        self.core_home = pl.CORE_HOME
        self.core_parts = set(pl.CORE.values()) | {"TP9"}
        self.cell_home = tuple(next(c["box"] for c in pl.CUTOUTS if c["name"] == "cell"))
        self.spk_home = tuple(next(c["box"] for c in pl.CUTOUTS if c["name"] == "speaker"))
        self.spk = self.spk_home
        self.hole = self.cell_home
        bb = self.board.GetBoardEdgesBoundingBox()
        mm = pcbnew.ToMM
        bw, bl = pl.CASE["board"]["w"], pl.CASE["board"]["l"]        # the outline without the USB tab
        self.edge = (100 - bw / 2, 100 - bl / 2, 100 + bw / 2, 100 + bl / 2)
        so = pcbnew.SHAPE_POLY_SET()
        self.board.GetBoardPolygonOutlines(so, False)
        o = so.Outline(0)
        self.outer = Polygon([(mm(o.CPoint(k).x), mm(o.CPoint(k).y)) for k in range(o.PointCount())])
        self.usb_tab = pl.USB_TAB
        self.set_hole((0, 0))
        self.npth = []
        for f in self.board.GetFootprints():
            for p in f.Pads():
                if p.GetAttribute() == pcbnew.PAD_ATTRIB_NPTH:
                    c = p.GetPosition()
                    self.npth.append((mm(c.x), mm(c.y), max(mm(p.GetDrillSizeX()), mm(p.GetDrillSizeY())) / 2, f.GetReference()))
        self.npth_fixed = [n for n in self.npth if n[3] not in set(self.core_parts) | {r for g in pz.GROUPS.values() for r in g["refs"]}]
        self.movable = set(self.core_parts)
        for g in pz.GROUPS.values():
            self.movable |= set(g["refs"])
        self.base_poses = {r: self.pose(r) for r in self.movable}
        self.base_over = self.overlaps(set(self.fp))
        self.geom = {r: self.local_geom(r) for r in self.movable}
        self.fixed_boxes = {False: [], True: []}
        for r in self.fp:
            if r not in self.movable and "Fiducial" not in str(self.fp[r].GetFPID().GetLibItemName()):
                self.fixed_boxes[self.fp[r].IsFlipped()].append(self.court(r))
        self.fixed_np = {k: np.array(v).reshape(-1, 4) for k, v in self.fixed_boxes.items()}
        self.extents = self.load_extents(cache_dir)
        self.j3_top = self.court("J3")[1]
        self.nets = {}
        for r, f in self.fp.items():
            for p in f.Pads():
                n = p.GetNetname()
                if n and n != "GND" and not n.startswith("unconnected"):
                    self.nets.setdefault(n, []).append((r, p.GetNumber()))
        import build_pcb
        self.width = {n: (build_pcb.net_class(n)["track"] + 0.15) / 0.30 for n in self.nets}

    # -- geometry of the cutouts
    def set_hole(self, shift, usb_dy=0.0, spk=(0, 0)):
        dx, dy = shift
        sx0, sy0, sx1, sy1 = self.spk_home
        self.spk = (sx0 + spk[0], sy0 + spk[1], sx1 + spk[0], sy1 + spk[1])
        x0b, y0b, x1b, y1b = self.edge
        (_, ta), (tx, tb) = self.usb_tab
        rr = self.box(x0b, y0b, x1b, y1b).buffer(-EDGE_R).buffer(EDGE_R)
        tab = self.box(x1b - 1.0, min(ta, tb) + usb_dy, tx, max(ta, tb) + usb_dy)
        self.inner0 = rr.union(tab).buffer(-COPPER_EDGE + TOL)        # pads' boxes are a little bigger than their shapes
        x0, y0, x1, y1 = self.cell_home
        self.hole = (x0 + dx, y0 - dy, x1 + dx, y1 - dy)
        h, s = self.hole, self.spk
        self.inner = self.inner0.difference(self.box(*h).buffer(COPPER_EDGE - TOL)).difference(self.box(*s).buffer(COPPER_EDGE - TOL))

    def pose(self, ref):
        f = self.fp[ref]
        return (f.GetPosition().x, f.GetPosition().y, f.GetOrientationDegrees(), f.IsFlipped())

    def court(self, ref):
        f, mm = self.fp[ref], self.pcbnew.ToMM
        lay = self.pcbnew.B_CrtYd if f.IsFlipped() else self.pcbnew.F_CrtYd
        bb = f.GetCourtyard(lay).BBox()
        if bb.GetWidth() == 0:
            bb = f.GetBoundingBox(False)
        x0, y0, x1, y1 = mm(bb.GetX()), mm(bb.GetY()), mm(bb.GetRight()), mm(bb.GetBottom())
        for p in f.Pads():          # a footprint's pads can stick out of its courtyard (the QFN's): the box covers them, 0.15 round
            pb = p.GetBoundingBox()
            x0, y0 = min(x0, mm(pb.GetX()) - 0.15), min(y0, mm(pb.GetY()) - 0.15)
            x1, y1 = max(x1, mm(pb.GetRight()) + 0.15), max(y1, mm(pb.GetBottom()) + 0.15)
        return (x0, y0, x1, y1)

    def local_geom(self, ref):
        """Courtyard and pad boxes relative to the footprint's origin, at its home turn (no flip handling: back parts only slide)."""
        f, mm = self.fp[ref], self.pcbnew.ToMM
        x, y = mm(f.GetPosition().x), mm(f.GetPosition().y)
        c = self.court(ref)
        pads = []
        for p in f.Pads():
            bb = p.GetBoundingBox()
            pads.append((mm(bb.GetX()) - x, mm(bb.GetY()) - y, mm(bb.GetRight()) - x, mm(bb.GetBottom()) - y))
        return dict(court=(c[0] - x, c[1] - y, c[2] - x, c[3] - y), pads=pads, flipped=f.IsFlipped(), rot=f.GetOrientationDegrees())

    def overlaps(self, refs):
        out = set()
        boxes = {}
        for r in self.fp:
            if "Fiducial" in str(self.fp[r].GetFPID().GetLibItemName()):
                continue
            boxes[r] = (self.court(r), self.fp[r].IsFlipped())
        for a in refs:
            if a not in boxes:
                continue
            (ba, fa) = boxes[a]
            for b, (bb, fb) in boxes.items():
                if a != b and fa == fb and min(ba[2], bb[2]) - max(ba[0], bb[0]) > 0.01 and min(ba[3], bb[3]) - max(ba[1], bb[1]) > 0.01:
                    out.add(tuple(sorted((a, b))))
        return out

    # -- a state: {"poses": {ref: [x, y, rot, side]}, "core": [x, y, rot], "hole": [dx, dy]} on the board in memory
    def set_abs(self, ref, x, y, o, side):
        pcbnew = self.pcbnew
        f = self.fp[ref]
        if f.IsFlipped():
            f.Flip(f.GetPosition(), pcbnew.FLIP_DIRECTION_LEFT_RIGHT)
        f.SetPosition(pcbnew.VECTOR2I(pcbnew.FromMM(x), pcbnew.FromMM(y)))
        f.SetOrientationDegrees(o)
        if side == "B":
            f.Flip(f.GetPosition(), pcbnew.FLIP_DIRECTION_LEFT_RIGHT)

    def apply_state(self, st):
        """Put the state on the board; returns the set of refs that are not where they are at home."""
        pcbnew = self.pcbnew
        V = lambda x, y: pcbnew.VECTOR2I(pcbnew.FromMM(x), pcbnew.FromMM(y))
        for r, (x, y, o, fl) in self.base_poses.items():
            f = self.fp[r]
            f.SetPosition(pcbnew.VECTOR2I(x, y))
            f.SetOrientationDegrees(o)
        moved = set()
        core = st.get("core")
        if core and (core[0], core[1], core[2]) != tuple(self.core_home):
            c = V(self.core_home[0], self.core_home[1])
            for r in self.core_parts:
                f = self.fp[r]
                f.Rotate(c, pcbnew.EDA_ANGLE(core[2] - self.core_home[2], pcbnew.DEGREES_T))
                f.Move(V(core[0] - self.core_home[0], core[1] - self.core_home[1]))
                moved.add(r)
        for r, (x, y, o, side) in st.get("poses", {}).items():
            self.set_abs(r, x, y, o, side)
            moved.add(r)
        self.set_hole(st.get("hole", (0, 0)), st.get("usb_dy", 0.0), st.get("speaker", (0, 0)))
        return moved

    # -- the case's extents for the moved parts
    def load_extents(self, cache_dir):
        path = os.path.join(cache_dir, "extents.json")
        if os.path.exists(path):
            cached = json.load(open(path))
            if set(self.movable) <= set(cached):        # a group added since the cache was made has no extents: its case rules would go unchecked
                return cached
        import check_fit
        check_fit.PCB = self.pcb
        refs = sorted(self.movable)
        boxes = check_fit.extents(refs)
        out = {}
        for r in refs:
            b = boxes.get(r)
            f = self.fp[r]
            x, y = (self.pcbnew.ToMM(f.GetPosition().x), self.pcbnew.ToMM(f.GetPosition().y))
            if b is None:
                name = str(f.GetFPID().GetLibItemName())
                c = self.court(r)
                h = next((v for k, v in check_fit.PACKAGE_HEIGHT.items() if k in name), 1.5)
                b = (c[0], c[1], c[2], c[3], 0.0, h)
            out[r] = dict(rel=[b[0] - x, b[1] - y, b[2] - x, b[3] - y], z=[b[4], b[5]], rot=f.GetOrientationDegrees(), flipped=f.IsFlipped())
        os.makedirs(cache_dir, exist_ok=True)
        json.dump(out, open(path, "w"))
        return out

    def push_pos(self):
        """check_fit's POS with this candidate's cell hole and speaker; returns what to restore."""
        import check_fit
        pos = check_fit.POS
        saved = (pos["cell_hole"], dict(pos["speaker"]))
        x0, y0, x1, y1 = self.hole
        pos["cell_hole"] = [[x0, y1], [x1, y0]]        # KiCad frame corners as positions.json has them
        sx = (self.spk[0] + self.spk[2]) / 2 - (self.spk_home[0] + self.spk_home[2]) / 2
        sy = (self.spk[1] + self.spk[3]) / 2 - (self.spk_home[1] + self.spk_home[3]) / 2
        pos["speaker"] = dict(saved[1], c=[saved[1]["c"][0] + sx, saved[1]["c"][1] + sy])
        return saved

    def pop_pos(self, saved):
        import check_fit
        check_fit.POS["cell_hole"], check_fit.POS["speaker"] = saved

    def case_problems(self, refs, echoed, hole_shift=(0, 0)):
        import check_fit
        saved = self.push_pos()
        problems, under = [], []
        try:
            for r in sorted(refs):
                e = self.extents.get(r)
                if e is None:
                    continue
                f = self.fp[r]
                mm = self.pcbnew.ToMM
                x, y = mm(f.GetPosition().x), mm(f.GetPosition().y)
                d = math.radians((f.GetOrientationDegrees() - e["rot"]) * (-1 if f.IsFlipped() else 1))
                c, s = round(math.cos(d)), round(math.sin(d))
                xs, ys = [], []
                for px in (e["rel"][0], e["rel"][2]):
                    for py in (e["rel"][1], e["rel"][3]):
                        xs.append(x + px * c + py * s)
                        ys.append(y - px * s + py * c)
                box = (min(xs), min(ys), max(xs), max(ys), e["z"][0], e["z"][1])
                check_fit.check_part(r, box, f.IsFlipped(), echoed, problems, under)
        finally:
            self.pop_pos(saved)
        return problems

    def validity(self, moved, echoed, st):
        mm = self.pcbnew.ToMM
        bad = []
        new = {p for p in self.overlaps(moved) if p not in self.base_over}
        bad += [f"courtyards {a}/{b}" for a, b in sorted(new)][:3]
        h = self.hole
        for r in self.fixed_refs():          # nothing fixed may fall into a shifted hole
            c = self.court(r)
            if min(c[2], h[2]) - max(c[0], h[0]) > 0.01 and min(c[3], h[3]) - max(c[1], h[1]) > 0.01 and not self.fp[r].IsFlipped():
                bad.append(f"{r} in the cell hole")
                break
        if "U4" in moved:
            u = [p.GetBoundingBox() for p in self.fp["U4"].Pads()]
            ex0, ey0, ex1, ey1 = self.edge
            if (min(p.GetX() for p in u) / 1e6 - ESCAPE < ex0 + COPPER_EDGE or max(p.GetRight() for p in u) / 1e6 + ESCAPE > ex1 - COPPER_EDGE
                    or min(p.GetY() for p in u) / 1e6 - ESCAPE < ey0 or max(p.GetBottom() for p in u) / 1e6 + ESCAPE > ey1):
                bad.append("core's escape tracks would run off the board")
        sp = self.spk
        if min(sp[2], h[2]) - max(sp[0], h[0]) > -1.0 and min(sp[3], h[3]) - max(sp[1], h[1]) > -1.0:
            bad.append("speaker hole within 1 mm of the cell hole")
        sk = (sp[0] - 0.5, sp[1] - 0.5, sp[2] + 0.5, sp[3] + 0.5)         # the rule area: the hole's outline plus the speaker's grow
        for r in (list(self.fixed_refs()) + sorted(self.movable)) if self.spk != self.spk_home else ():
            c = self.court(r)
            if not self.fp[r].IsFlipped() and min(c[2], sk[2]) - max(c[0], sk[0]) > 0.01 and min(c[3], sk[3]) - max(c[1], sk[1]) > 0.01:
                bad.append(f"{r} in the speaker's keep-out")
                break
        if self.spk != self.spk_home:
            import check_fit
            saved = self.push_pos()
            try:
                for r in self.fixed_refs():
                    if r.startswith("H") and check_fit.near_speaker((self.court(r)[0], self.court(r)[1], self.court(r)[2], self.court(r)[3])):
                        bad.append(f"screw {r} runs into the speaker's pocket")
            finally:
                self.pop_pos(saved)
        for r in sorted(moved):
            for p in self.fp[r].Pads():
                bb = p.GetBoundingBox()
                b = self.box(mm(bb.GetX()), mm(bb.GetY()), mm(bb.GetRight()), mm(bb.GetBottom()))
                if not self.inner.contains(b):
                    bad.append(f"{r} pad {p.GetNumber()} within {COPPER_EDGE} of the edge or a cutout")
                    break
                for hx, hy, hr, hf in self.npth_now():
                    if hf != r and math.hypot(max(b.bounds[0] - hx, 0, hx - b.bounds[2]), max(b.bounds[1] - hy, 0, hy - b.bounds[3])) < hr + 0.3:
                        bad.append(f"{r} pad within 0.3 of {hf}'s hole")
                        break
        for r in self.pz.CONSTRAINTS:
            f = self.fp[r]
            pads = [(mm(p.GetBoundingBox().GetX()), mm(p.GetBoundingBox().GetY()), mm(p.GetBoundingBox().GetRight()), mm(p.GetBoundingBox().GetBottom()))
                    for p in f.Pads()]
            if r in moved and not self.constraint_ok(r, mm(f.GetPosition().x), mm(f.GetPosition().y), 0, pads):
                bad.append(f"{r} breaks its constraint")
        if not bad:
            bad += [f"case: {t}" for t in self.case_problems([r for r in moved if r not in self.pz.GROUPS["usb"]["refs"]], echoed)][:3]
        return bad

    def constraint_ok(self, ref, x, y, deg, pads=None):
        rule = self.pz.CONSTRAINTS.get(ref)
        if not rule:
            return True
        ctx = self
        h = ctx.hole
        if rule[0] == "near_hole":      # the nearest pad edge within rule[1] of the hole's side edge
            gs = []
            for b in (pads if pads is not None else
                      [(q[0] + x, q[1] + y, q[2] + x, q[3] + y) for q in (rot_box(p, deg) for p in ctx.geom[ref]["pads"])]):
                gs.append(max(h[0] - b[2], b[0] - h[2]))
            return min(gs) <= rule[1] and (h[1] - 1 <= y <= h[3] + 1)
        if rule[0] == "near_speaker":
            sp = ctx.spk
            return math.dist((x, y), ((sp[0] + sp[2]) / 2, (sp[1] + sp[3]) / 2)) <= rule[1]
        if rule[0] == "near_lead_end":  # J2 near the hole's seat-2 end corner on its side
            cx = h[0] if x < (h[0] + h[2]) / 2 else h[2]
            return math.dist((x, y), (cx, h[1])) <= rule[1]
        return True

    def npth_now(self):
        """The NPTH holes where they are now: the fixed footprints' and the moved ones' at their poses."""
        pcbnew, mm = self.pcbnew, self.pcbnew.ToMM
        out = list(self.npth_fixed)
        for r in self.movable:
            for p in self.fp[r].Pads():
                if p.GetAttribute() == pcbnew.PAD_ATTRIB_NPTH:
                    c = p.GetPosition()
                    out.append((mm(c.x), mm(c.y), max(mm(p.GetDrillSizeX()), mm(p.GetDrillSizeY())) / 2, r))
        return out

    def fixed_refs(self):
        if not hasattr(self, "_fixed"):
            self._fixed = [r for r in self.fp if r not in self.movable and "Fiducial" not in str(self.fp[r].GetFPID().GetLibItemName())]
        return self._fixed

    def pad_points(self):
        mm = self.pcbnew.ToMM
        pts = {}
        for n, pins in self.nets.items():
            seen = []
            for r, num in pins:
                for p in self.fp[r].Pads():
                    if p.GetNumber() == num:
                        c = p.GetPosition()
                        q = (mm(c.x), mm(c.y))
                        if all(math.dist(q, s) > 0.05 for s in seen):
                            seen.append(q)
                        break
            if len(seen) > 1:
                pts[n] = seen
        return pts


# ---------------------------------------------------------------------------------------------------------- geometry
def clip(a, b, r):
    t0, t1 = 0.0, 1.0
    dx, dy = b[0] - a[0], b[1] - a[1]
    for p, q in ((-dx, a[0] - r[0]), (dx, r[2] - a[0]), (-dy, a[1] - r[1]), (dy, r[3] - a[1])):
        if abs(p) < 1e-12:
            if q < 0:
                return 0.0, 0.0
        else:
            t = q / p
            if p < 0:
                t0 = max(t0, t)
            else:
                t1 = min(t1, t)
    return t0, t1


class Geo:
    """Shortest paths round rectangular walls (the hole, the speaker's, the core's body)."""

    def __init__(self, walls):
        e = 1e-6
        self.inner = [(w[0] + e, w[1] + e, w[2] - e, w[3] - e) for w in walls]
        self.C = []
        for w in walls:
            g = 0.4
            for c in ((w[0] - g, w[1] - g), (w[2] + g, w[1] - g), (w[2] + g, w[3] + g), (w[0] - g, w[3] + g)):
                if not any(i[0] < c[0] < i[2] and i[1] < c[1] < i[3] for i in self.inner):
                    self.C.append(c)
        n = len(self.C)
        INF = 1e9
        self.D = [[0.0 if i == j else (math.dist(self.C[i], self.C[j]) if not self.blocked(self.C[i], self.C[j]) else INF)
                   for j in range(n)] for i in range(n)]
        self.nx = [[j if self.D[i][j] < INF else None for j in range(n)] for i in range(n)]
        for k in range(n):
            for i in range(n):
                for j in range(n):
                    if self.D[i][k] + self.D[k][j] < self.D[i][j]:
                        self.D[i][j] = self.D[i][k] + self.D[k][j]
                        self.nx[i][j] = self.nx[i][k]
        self.vis = {}

    def blocked(self, a, b):
        for r in self.inner:
            t0, t1 = clip(a, b, r)
            if t1 - t0 > 1e-9:
                return True
        return False

    def visible(self, a):
        v = self.vis.get(a)
        if v is None:
            v = self.vis[a] = [i for i, c in enumerate(self.C) if not self.blocked(a, c)]
        return v

    def path(self, a, b):
        if not self.blocked(a, b):
            return math.dist(a, b), [a, b]
        best, bi, bj = 1e18, None, None
        vb = self.visible(b)
        for i in self.visible(a):
            da = math.dist(a, self.C[i])
            for j in vb:
                d = da + self.D[i][j] + math.dist(self.C[j], b)
                if d < best:
                    best, bi, bj = d, i, j
        if bi is None:
            return math.dist(a, b) * 3, [a, b]
        pts, k = [a, self.C[bi]], bi
        while k != bj:
            k = self.nx[k][bj]
            pts.append(self.C[k])
        pts.append(b)
        return best, pts


def seg_cross(p, q, a, b):
    def o(u, v, w):
        return (v[0] - u[0]) * (w[1] - u[1]) - (v[1] - u[1]) * (w[0] - u[0])
    d1, d2, d3, d4 = o(a, b, p), o(a, b, q), o(p, q, a), o(p, q, b)
    return (d1 * d2 <= 0) and (d3 * d4 <= 0)


def mst(pts, geo):
    inside, edges, total = {0}, [], 0.0
    cache = {}
    while len(inside) < len(pts):
        best = None
        for i in inside:
            for j in range(len(pts)):
                if j in inside:
                    continue
                k = (i, j)
                if k not in cache:
                    cache[k] = geo.path(pts[i], pts[j])
                if best is None or cache[k][0] < best[0]:
                    best = (cache[k][0], j, cache[k][1])
        total += best[0]
        edges.append(best[2])
        inside.add(best[1])
    return total, edges


def score_board(ctx, detail=False):
    hole = ctx.hole
    pts = ctx.pad_points()
    u4 = [(p.GetPosition().x / 1e6, p.GetPosition().y / 1e6) for p in ctx.fp["U4"].Pads()]
    core = (min(p[0] for p in u4) + 0.3, min(p[1] for p in u4) + 0.3, max(p[0] for p in u4) - 0.3, max(p[1] for p in u4) - 0.3)
    geo = Geo([hole, core, ctx.spk])
    x0, y0, x1, y1 = hole
    ex0, ey0, ex1, ey1 = ctx.edge
    xc = (x0 + x1) / 2
    ym = (y0 + y1) / 2
    cuts = {
        "chan": ((xc, y1 + 0.3), (xc, ctx.j3_top - 0.3)),
        "left": ((ex0 + 0.3, ym), (x0 - 0.3, ym)),
        "right": ((x1 + 0.3, ym), (ex1 - 0.3, ym)),
        "top": ((xc, ey0 + 0.3), (xc, y0 - 0.3)),
    }
    load = {k: 0.0 for k in cuts}
    users = {k: [] for k in cuts}
    total = totalw = power = 0.0
    cells = {}
    for n, p in pts.items():
        d, edges = mst(p, geo)
        total += d
        w = ctx.width.get(n, 1.0)
        totalw += w * d
        if n in POWER_NETS:
            power += w * d
        hit, seen = set(), set()
        for path in edges:
            for a, b in zip(path, path[1:]):
                for k, (c0, c1) in cuts.items():
                    if k not in hit and seg_cross(a, b, c0, c1):
                        hit.add(k)
                L = math.dist(a, b)
                for s in range(int(L / 0.6) + 1):
                    t = s * 0.6 / L if L else 0
                    seen.add((int((a[0] + (b[0] - a[0]) * t) // GRID), int((a[1] + (b[1] - a[1]) * t) // GRID)))
        for k in hit:
            load[k] += w
            users[k].append(n)
        for c in seen:
            cells[c] = cells.get(c, 0.0) + w
    cap = {k: max(0.5, math.dist(*cuts[k])) / PITCH * LAYERS_USABLE for k in cuts}
    press = {k: load[k] / cap[k] for k in cuts}
    cong = sum(max(0.0, v - GRID / PITCH * LAYERS_USABLE) ** 2 for v in cells.values())
    soft = 0.0
    for a, b, lim in SOFT:
        pa, pb = ctx.fp[a].GetPosition(), ctx.fp[b].GetPosition()
        soft += SOFT_W * max(0.0, math.hypot(pa.x - pb.x, pa.y - pb.y) / 1e6 - lim)
    score = (100 * press["chan"] + 25 * max(press["left"], press["right"], press["top"]) + totalw / 60 + POWER_W * power + soft + cong / 4)
    out = dict(score=round(score, 2), len=round(total), lenw=round(totalw), power=round(power), soft=round(soft, 1), chan=round(load["chan"], 1), chan_cap=round(cap["chan"], 1),
               left=round(load["left"], 1), right=round(load["right"], 1), top=round(load["top"], 1), cong=round(cong, 1))
    if detail:
        out["users"] = users
    return out


# ---------------------------------------------------------------------------------------------------------- stage 1: zones
class Planner:
    """Evaluate states, and realize a zone assignment as a state."""

    def __init__(self, work):
        import check_fit
        self.work = work
        pcb = os.path.join(work, "base", "board", "battler.kicad_pcb")
        if not os.path.exists(pcb):
            stage_base(os.path.join(work, "base"))
        self.ctx = Ctx(pcb, os.path.join(work, "cache"))
        ep = os.path.join(work, "cache", "echo.json")
        if os.path.exists(ep):
            self.echoed = json.load(open(ep))
        else:
            self.echoed = check_fit.case_echo()
            json.dump(self.echoed, open(ep, "w"))
        self.pz = self.ctx.pz
        self.memo = {}

    def evaluate(self, st):
        key = json.dumps(st, sort_keys=True)
        if key in self.memo:
            return self.memo[key]
        ctx = self.ctx
        moved = ctx.apply_state(st)
        bad = ctx.validity(moved, self.echoed, st)
        r = dict(valid=not bad, why=bad[:3])
        if not bad:
            r.update(score_board(ctx))
        if len(self.memo) > 20000:
            self.memo.clear()
        self.memo[key] = r
        return r

    # -- packing
    def legal(self, ref, x, y, deg, taken, flipped_side=False):
        """Would ref at (x, y) turned deg more than at home be legal against the boxes taken (a list of F boxes)? Cheap, approximate."""
        ctx = self.ctx
        g = ctx.geom[ref]
        c = rot_box(g["court"], 0 if g["flipped"] else deg)
        cb = (c[0] + x, c[1] + y, c[2] + x, c[3] + y)
        fl = g["flipped"]
        occ = ctx.fixed_np[fl]
        if taken[fl]:
            occ = np.vstack([occ, np.array(taken[fl])])
        if len(occ) and (((np.minimum(cb[2], occ[:, 2]) - np.maximum(cb[0], occ[:, 0])) > 0.01)
                         & ((np.minimum(cb[3], occ[:, 3]) - np.maximum(cb[1], occ[:, 1])) > 0.01)).any():
            return False
        ex0, ey0, ex1, ey1 = ctx.edge
        h, s = ctx.hole, ctx.spk
        if not fl and min(cb[2], s[2] + 0.5) - max(cb[0], s[0] - 0.5) > 0.01 and min(cb[3], s[3] + 0.5) - max(cb[1], s[1] - 0.5) > 0.01:
            return False
        for p in g["pads"]:
            q = rot_box(p, 0 if g["flipped"] else deg)
            b = (q[0] + x, q[1] + y, q[2] + x, q[3] + y)
            if b[0] < ex0 + (COPPER_EDGE - TOL) or b[2] > ex1 - (COPPER_EDGE - TOL) or b[1] < ey0 + (COPPER_EDGE - TOL) or b[3] > ey1 - (COPPER_EDGE - TOL):
                return False
            for w in (h, s):
                if b[2] > w[0] - (COPPER_EDGE - TOL) and b[0] < w[2] + (COPPER_EDGE - TOL) and b[3] > w[1] - (COPPER_EDGE - TOL) and b[1] < w[3] + (COPPER_EDGE - TOL):
                    return False
            for cx, cy in ((ex0, ey0), (ex1, ey0), (ex0, ey1), (ex1, ey1)):       # the board's rounded corners
                kx, ky = (cx + EDGE_R if cx == ex0 else cx - EDGE_R), (cy + EDGE_R if cy == ey0 else cy - EDGE_R)
                if (b[0] < kx < b[2] or True) and ((cx == ex0 and b[0] < kx) or (cx == ex1 and b[2] > kx)) and \
                        ((cy == ey0 and b[1] < ky) or (cy == ey1 and b[3] > ky)):
                    px = b[0] if cx == ex0 else b[2]
                    py = b[1] if cy == ey0 else b[3]
                    if math.hypot(px - kx, py - ky) > EDGE_R - (COPPER_EDGE - TOL):
                        return False
            for hx, hy, hr, hf in ctx.npth_fixed:
                if hf != ref and math.hypot(max(b[0] - hx, 0, hx - b[2]), max(b[1] - hy, 0, hy - b[3])) < hr + 0.3:
                    return False
        if not self.constraint_ok(ref, x, y, deg):
            return False
        return self.case_ok(ref, x, y, deg)

    def constraint_ok(self, ref, x, y, deg, pads=None):
        return self.ctx.constraint_ok(ref, x, y, deg, pads)

    def case_ok(self, ref, x, y, deg):
        """The case's rules for this part at this pose (check_fit.check_part on its cached extent)."""
        import check_fit
        e = self.ctx.extents.get(ref)
        if e is None or e["flipped"]:
            return True
        rb = rot_box(tuple(e["rel"]), deg)
        box = (rb[0] + x, rb[1] + y, rb[2] + x, rb[3] + y, e["z"][0], e["z"][1])
        saved = self.ctx.push_pos()
        problems = []
        try:
            check_fit.check_part(ref, box, False, self.echoed, problems, [])
        finally:
            self.ctx.pop_pos(saved)
        return not problems

    def ring_spots(self, rmax, step=0.5):
        pts = [(0.0, 0.0)]
        n = int(rmax / step)
        for i in range(1, n + 1):
            r = i * step
            m = max(8, int(2 * math.pi * r / step))
            for k in range(m):
                a = 2 * math.pi * k / m
                pts.append((r * math.cos(a), r * math.sin(a)))
        return pts

    def anchor_grid(self, z):
        """Anchors over the whole zone (and SLACK past its edges), nearest its centre first."""
        pz = self.pz
        cx, cy = (z[0] + z[2]) / 2, (z[1] + z[3]) / 2
        st = pz.ANCHOR_STEP
        xs = [z[0] - pz.SLACK + i * st for i in range(int((z[2] - z[0] + 2 * pz.SLACK) / st) + 1)]
        ys = [z[1] - pz.SLACK + i * st for i in range(int((z[3] - z[1] + 2 * pz.SLACK) / st) + 1)]
        return sorted(((x, y) for x in xs for y in ys), key=lambda a: math.dist(a, (cx, cy)))

    def pack_group(self, name, zone, turn, taken, skip=()):
        """Poses {ref: [x, y, rot, side]} for a packed group in its zone, or None. taken is updated on success."""
        ctx, pz = self.ctx, self.pz
        g = pz.GROUPS[name]
        refs = [r for r in g["refs"] if r not in skip]
        hp = {r: ctx.home_P[r] for r in refs}
        hx = sum(p[0] for p in hp.values()) / len(refs)
        hy = sum(p[1] for p in hp.values()) / len(refs)
        z = pz.ZONES[zone]
        cx, cy = (z[0] + z[2]) / 2, (z[1] + z[3]) / 2
        order = sorted(refs, key=lambda r: -((ctx.geom[r]["court"][2] - ctx.geom[r]["court"][0]) * (ctx.geom[r]["court"][3] - ctx.geom[r]["court"][1])))
        anchors = self.anchor_grid(z)
        spots = self.ring_spots(5.0, 0.6)
        for ax, ay in anchors:
            mine = {False: [], True: []}
            out = {}
            ok = True
            for r in order:
                rel = rot_pt((ctx.home_P[r][0] - hx, ctx.home_P[r][1] - hy), turn)
                tx, ty = ax + rel[0], ay + rel[1]
                found = None
                for dx, dy in spots:
                    if self.legal(r, tx + dx, ty + dy, turn, {False: taken[False] + mine[False], True: taken[True] + mine[True]}):
                        found = (tx + dx, ty + dy)
                        break
                if not found:
                    ok = False
                    break
                g2 = ctx.geom[r]
                c = rot_box(g2["court"], 0 if g2["flipped"] else turn)
                mine[g2["flipped"]].append((c[0] + found[0], c[1] + found[1], c[2] + found[0], c[3] + found[1]))
                p = ctx.home_P[r]
                out[r] = [round(found[0], 3), round(found[1], 3), p[2] if g2["flipped"] else (p[2] + turn) % 360, p[3]]
            if ok:
                for k in (False, True):
                    taken[k] += mine[k]
                return out
        return None

    def place_core(self, zone, turn, taken):
        """The core's rigid set (U4 and the reference's parts) at the zone's nearest working spot, its loose caps packed round it:
        (core pose [x, y, rot], loose poses, boxes it takes) or None."""
        ctx, pz = self.ctx, self.pz
        z = pz.ZONES[zone]
        cx, cy = (z[0] + z[2]) / 2, (z[1] + z[3]) / 2
        hx, hy, hrot = ctx.core_home
        tries = self.anchor_grid(z)
        ex0, ey0, ex1, ey1 = ctx.edge
        best = None
        self.core_fail = None
        for ax, ay in tries:
            ctx.apply_state(dict({"core": [ax, ay, (hrot + turn) % 360]}, **self.geo))
            rigid = {False: [], True: []}
            ok = True
            for r in ctx.core_parts:
                c = ctx.court(r)
                fl = ctx.fp[r].IsFlipped()
                if any(min(c[2], o[2]) - max(c[0], o[0]) > 0.01 and min(c[3], o[3]) - max(c[1], o[1]) > 0.01
                       for o in ctx.fixed_boxes[fl] + taken[fl]):
                    ok = False
                    break
                rigid[fl].append(c)
                for p in ctx.fp[r].Pads():
                    bb = p.GetBoundingBox()
                    b = (bb.GetX() / 1e6, bb.GetY() / 1e6, bb.GetRight() / 1e6, bb.GetBottom() / 1e6)
                    if b[0] < ex0 + (COPPER_EDGE - TOL) or b[2] > ex1 - (COPPER_EDGE - TOL) or b[1] < ey0 + (COPPER_EDGE - TOL) or b[3] > ey1 - (COPPER_EDGE - TOL) or \
                            any(b[2] > w[0] - (COPPER_EDGE - TOL) and b[0] < w[2] + (COPPER_EDGE - TOL) and b[3] > w[1] - (COPPER_EDGE - TOL) and b[1] < w[3] + (COPPER_EDGE - TOL)
                                for w in (ctx.hole, ctx.spk)):
                        ok = False
                        break
                if not ok:
                    break
            if ok:
                u = [p.GetBoundingBox() for p in ctx.fp["U4"].Pads()]
                ux0, uy0 = min(p.GetX() for p in u) / 1e6, min(p.GetY() for p in u) / 1e6
                ux1, uy1 = max(p.GetRight() for p in u) / 1e6, max(p.GetBottom() for p in u) / 1e6
                ok = ux0 - ESCAPE >= ex0 + COPPER_EDGE and ux1 + ESCAPE <= ex1 - COPPER_EDGE and uy0 - ESCAPE >= ey0 and uy1 + ESCAPE <= ey1
            if not ok:
                continue
            cp = ctx.case_problems(ctx.core_parts, self.echoed)
            if cp:                       # remember the fewest-problem anchor, for the report
                if best is None or len(cp) < best[0]:
                    best = (len(cp), (ax, ay), cp[:2])
                self.core_fail = best
                continue
            t2 = {False: taken[False] + rigid[False], True: taken[True] + rigid[True]}
            loose = self.pack_loose(ax, ay, turn, t2)
            if loose is not None:
                return [ax, ay, (hrot + turn) % 360], loose, {k: t2[k][len(taken[k]):] for k in (False, True)}
        return None

    def pack_loose(self, ax, ay, turn, taken):
        """The core's loose parts, each at the nearest legal spot to where it sits at home relative to U4; boxes are added to taken."""
        ctx, pz = self.ctx, self.pz
        hx, hy, _ = ctx.core_home
        spots = self.ring_spots(8.0, 0.7)
        out = {}
        for r in pz.GROUPS["core"]["refs"]:
            rel = rot_pt((ctx.home_P[r][0] - hx, ctx.home_P[r][1] - hy), turn)
            found = None
            for dx, dy in spots:
                if self.legal(r, ax + rel[0] + dx, ay + rel[1] + dy, turn, taken):
                    found = (ax + rel[0] + dx, ay + rel[1] + dy)
                    break
            if not found:
                return None
            g2 = ctx.geom[r]
            c = rot_box(g2["court"], 0 if g2["flipped"] else turn)
            taken[g2["flipped"]].append((c[0] + found[0], c[1] + found[1], c[2] + found[0], c[3] + found[1]))
            p = ctx.home_P[r]
            out[r] = [round(found[0], 3), round(found[1], 3), (p[2] + turn) % 360, p[3]]
        return out

    ORDER = ("audio", "j4", "cellconn", "core", "charger", "vsense", "latch", "row2", "ldo", "j5", "boot", "misc", "spi_r", "c2930")

    def realize(self, assign):
        """assign: {group: [zone, turn]}; "hole": [dx, dy] (case frame), "usb": dy, "speaker": [dx, dy, zone, turn] (KiCad frame; the
        amplifier group is packed in the zone and J4 beside it); groups left out stay at home.
        Returns a state {"poses", "core", "hole", "usb_dy", "speaker"} or None when a group has no legal place."""
        ctx, pz = self.ctx, self.pz
        assign = dict(assign)
        st = {"poses": {}}
        hole = tuple(assign.get("hole", (0, 0)))
        spk = assign.pop("speaker", None)
        usb_dy = assign.get("usb", 0.0)
        if spk:
            st["speaker"] = [spk[0], spk[1]]
            assign["audio"] = [spk[2], spk[3]]
            assign.setdefault("j4", [spk[2], 0])
        elif usb_dy and "j4" not in assign:                      # J1 slid: J4 is packed again where it was
            assign["j4"] = ["low_r", 0]
        ctx.apply_state({})
        ctx.set_hole(hole, usb_dy, (spk[0], spk[1]) if spk else (0, 0))
        self.geo = dict(hole=list(hole), usb_dy=usb_dy, speaker=[spk[0], spk[1]] if spk else [0, 0])
        if hole != (0, 0):
            st["hole"] = list(hole)
        if usb_dy:
            st["usb_dy"] = usb_dy
            for r in pz.GROUPS["usb"]["refs"]:
                p = ctx.home_P[r]
                st["poses"][r] = [p[0], p[1] + usb_dy, p[2], p[3]]
        moving = [n for n in self.ORDER if n in assign]
        if hole != (0, 0) and "cellconn" not in assign:     # the cell's connector follows a shifted hole (home zone, same turn)
            for r in pz.GROUPS["cellconn"]["refs"]:
                p = ctx.home_P[r]
                st["poses"][r] = [p[0] + hole[0], p[1] - hole[1], p[2], p[3]]
        for r, v in st["poses"].items():
            ctx.set_abs(r, *v)
        movers = set()
        for n in moving:
            movers |= set(pz.GROUPS[n]["refs"])
        if "core" in assign:
            movers |= ctx.core_parts
        fixed_here = set(pz.GROUPS["usb"]["refs"]) | set(pz.GROUPS["cellconn"]["refs"]) | set(st["poses"])
        # parts that stay at home are not obstacles while the groups are placed: the ones a group lands on are moved at the end
        evictable = [r for r in ctx.movable if r not in movers and r not in fixed_here and not (r in ctx.core_parts)]
        taken = {False: [], True: []}
        for r in fixed_here:
            if r not in movers:
                taken[ctx.fp[r].IsFlipped()].append(ctx.court(r))
        for n in moving:
            zone, turn = assign[n]
            if n == "core":
                res = self.place_core(zone, turn, taken)
                if res is None:
                    self.fail = "core"
                    return None
                st["core"], loose, added = res
                st["poses"].update(loose)
                for k in (False, True):
                    taken[k] += added[k]
                continue
            res = self.pack_group(n, zone, turn, taken, skip=pz.GROUPS["vsense"]["refs"] if (n == "charger" and "vsense" in assign) else ())
            if res is None:
                self.fail = n
                return None
            st["poses"].update(res)
        # the parts left at home keep their place unless something landed on it: those go to the nearest legal spot
        ctx_home = {r: ctx_court_home(ctx, r) for r in evictable}
        spots = self.ring_spots(8.0, 0.7)
        for r in sorted(evictable, key=lambda r: r):
            g = ctx.geom[r]
            fl = g["flipped"]
            hb = ctx_home[r]
            x, y, o, side = ctx.home_P[r] if r in ctx.home_P else (None, None, None, None)
            if x is None:
                continue
            sp, ho = ctx.spk, ctx.hole
            cut = []
            if not fl:           # a home part is legal against the cutouts where they are at home; only a moved cutout can land on it
                if sp != ctx.spk_home:
                    cut.append((sp[0] - 0.5, sp[1] - 0.5, sp[2] + 0.5, sp[3] + 0.5))
                if ho != ctx.cell_home:
                    cut.append(ho)
            if not any(min(hb[2], o[2]) - max(hb[0], o[0]) > 0.01 and min(hb[3], o[3]) - max(hb[1], o[1]) > 0.01 for o in taken[fl] + cut):
                taken[fl].append(hb)         # nothing landed on it: it stays (home is legal by construction)
                continue
            for dx, dy in spots:
                if self.legal(r, x + dx, y + dy, 0, taken):
                    b = g["court"]
                    taken[fl].append((b[0] + x + dx, b[1] + y + dy, b[2] + x + dx, b[3] + y + dy))
                    st["poses"][r] = [round(x + dx, 3), round(y + dy, 3), o, side]
                    break
            else:
                self.fail = f"evict {r}"
                return None
        return st


def ctx_court_home(ctx, r):
    f = ctx.fp[r]
    x, y, o, fl = ctx.base_poses[r]
    g = ctx.geom[r]
    b = g["court"]
    mx, my = x / 1e6, y / 1e6
    return (b[0] + mx, b[1] + my, b[2] + mx, b[3] + my)


def holescan(work, dys, jobs, keep=8):
    """The cell hole shifted towards seat 2 by each dy (case frame, mm): the latch parts and the seat-2 Start's row it lands on are
    placed again (every zone for each of the two groups), the rest at home; the best `keep` per shift, written to DIR/hole_candidates.json."""
    pl = Planner(work)
    pz = pl.pz
    cand = []
    for dy in dys:
        for lo in options(pz, "latch"):
            for ro in options(pz, "row2"):
                a = {"hole": [0, dy]}
                if lo:
                    a["latch"] = lo
                if ro:
                    a["row2"] = ro
                cand.append(a)
    out = []
    with _pool(work, jobs) as pool:
        res = []
        for i, r in enumerate(pool.imap_unordered(_eval_assign, cand, chunksize=4)):
            res.append(r)
            if (i + 1) % 200 == 0 or i + 1 == len(cand):
                print(f"holescan: {i + 1}/{len(cand)} done, {sum(1 for x in res if x['valid'])} valid", flush=True)
    for dy in dys:
        v = sorted([r for r in res if r["valid"] and r["assign"]["hole"][1] == dy], key=lambda r: r["score"])[:keep]
        print(f"dy {dy}: {sum(1 for r in res if r['assign']['hole'][1] == dy and r['valid'])} valid; best", [round(r["score"], 1) for r in v[:4]],
              "chan/left/right/top of the best", [(r["chan"], r["left"], r["right"], r["top"]) for r in v[:1]])
        for r in v:
            out.append(dict(assign=r["assign"], picks=r["state"], score=r["score"], chan=r["chan"], left=r["left"], right=r["right"], top=r["top"], len=r["len"], tag=f"hole dy {dy}"))
    json.dump(out, open(os.path.join(work, "hole_candidates.json"), "w"))


def stage_base(dest, placement_picks=None):
    """A standalone copy of the board folder (and the case's positions.json) built for the home placement."""
    os.makedirs(dest, exist_ok=True)
    subprocess.run(["rsync", "-a", "--exclude", "fab", "--exclude", "pin_review", "--exclude", "__pycache__", "--exclude", "sim",
                    "--exclude", "*.3dshapes", "--exclude", "picks.json", "--exclude", "last_route.*", "--exclude", "freerouting.log",
                    BOARD_DIR + "/", os.path.join(dest, "board") + "/"], check=True)
    os.makedirs(os.path.join(dest, "case"), exist_ok=True)
    shutil.copy(os.path.join(CASE_DIR, "positions.json"), os.path.join(dest, "case", "positions.json"))
    env = dict(os.environ, RP2350B_REF=os.environ.get("RP2350B_REF") or default_ref())
    env.pop("PLACE_PICKS", None)
    if placement_picks:
        json.dump(placement_picks, open(os.path.join(dest, "board", "picks.json"), "w"))
    r = subprocess.run([sys.executable, "tools/build_pcb.py"], cwd=os.path.join(dest, "board"), env=env, capture_output=True, text=True)
    if r.returncode:
        raise SystemExit("build failed: " + r.stdout[-600:] + r.stderr[-600:])


def default_ref():
    import glob
    g = glob.glob(os.path.expanduser("~/.cache/ref_design_tools/rp2350b/*/RPI-RP2350B-MINIMAL_R4-S1.kicad_pcb"))
    return g[0] if g else ""


# ---------------------------------------------------------------------------------------------------------- search drivers
_PL = None


def _init(work):
    global _PL
    _PL = Planner(work)


def _eval_assign(assign):
    st = _PL.realize(assign)
    if st is None:
        why = ["no legal packing: " + str(getattr(_PL, "fail", None))]
        if getattr(_PL, "fail", None) == "core" and getattr(_PL, "core_fail", None):
            n, at, cp = _PL.core_fail
            why.append(f"nearest miss: {n} case problems at U4 {at[0]:.0f},{at[1]:.0f}: {cp[0]}")
        return dict(assign=assign, valid=False, why=why)
    r = _PL.evaluate(st)
    return dict(assign=assign, state=st, **r)


def _pool(work, jobs):
    import multiprocessing as mp
    return mp.get_context("spawn").Pool(jobs, initializer=_init, initargs=(work,))


def speaker_options(pl):
    """Speaker positions on a SPEAKER_STEP grid over the whole board: [dx, dy, zone, turn] for the legal ones (the hole inside the board with
    its 0.9 rim, 1 mm from the cell's hole, the grille (the outline less 1.5) at least 4 mm wide outside the glass, nothing the case forbids
    under a 0.9 mm tall part there, no fixed part in its keep-out), the audio group turned 0 or 180 in the zone nearest it."""
    import check_fit
    ctx, pz = pl.ctx, pl.pz
    ctx.apply_state({})
    ctx.set_hole((0, 0), 0.0, (0, 0))
    (gx0, gy0), (gx1, gy1) = check_fit.POS["glass"]
    gx0, gx1, gy0, gy1 = min(gx0, gx1), max(gx0, gx1), min(gy0, gy1), max(gy0, gy1)
    w, l = pz.SPEAKER_BOX
    ex0, ey0, ex1, ey1 = ctx.edge
    h = ctx.hole
    hx, hy = pz.SPEAKER_HOME
    out, step = [], pz.SPEAKER_STEP
    n = 0
    cx = ex0 + 0.9 + w / 2
    xs = []
    while cx <= ex1 - 0.9 - w / 2 + 1e-6:
        xs.append(cx)
        cx += step
    xs.append(ex1 - 0.9 - w / 2)                  # and flush with the right rim
    xs = sorted(set(round(x, 2) for x in xs))
    for cx in xs:
        cy = ey0 + 0.9 + l / 2
        while cy <= ey1 - 0.9 - l / 2 + 1e-6:
            b = (cx - w / 2, cy - l / 2, cx + w / 2, cy + l / 2)
            cy += step
            if min(b[2], h[2]) - max(b[0], h[0]) > -1.0 and min(b[3], h[3]) - max(b[1], h[1]) > -1.0:
                continue
            sx0, sy0, sx1, sy1 = b[0] + 1.5, b[1] + 1.5, b[2] - 1.5, b[3] - 1.5
            if min(sx1, gx1) - max(sx0, gx0) > 0 and min(sy1, gy1) - max(sy0, gy0) > 0:      # grille region meets the glass: the part outside must be wide enough
                outs = [max(0, min(sx1, gx0) - sx0), max(0, sx1 - max(sx0, gx1)), max(0, min(sy1, gy0) - sy0), max(0, sy1 - max(sy0, gy1))]
                if max(outs) < 4.0:
                    continue
            sk = (b[0] - 0.5, b[1] - 0.5, b[2] + 0.5, b[3] + 0.5)
            if any(min(sk[2], o[2]) - max(sk[0], o[0]) > 0.01 and min(sk[3], o[3]) - max(sk[1], o[1]) > 0.01 for o in ctx.fixed_boxes[False]):
                continue
            saved = (check_fit.POS["speaker"],)
            check_fit.POS["speaker"] = dict(saved[0], c=[-999.0, -999.0])
            problems, under = [], []
            try:
                check_fit.check_part("SPK", (b[0], b[1], b[2], b[3], 0.0, 0.9), False, pl.echoed, problems, under)
            finally:
                check_fit.POS["speaker"] = saved[0]
            if problems:
                continue
            def dist(z):
                r = pz.ZONES[z]
                return math.hypot(max(r[0] - cx, 0, cx - r[2]), max(r[1] - cy, 0, cy - r[3]))
            zone = min(("low_l", "low_r", "mid_low", "end1_l", "end1_r", "end2_l", "end2_m", "end2_r", "ls_bot", "rs_bot", "gap"), key=dist)
            for t in (0, 180):
                out.append([round(cx - hx, 2), round(cy - hy, 2), zone, t])
    return out


def options(pz, name):
    g = pz.GROUPS[name]
    return [None] + (g["options"] if "options" in g else [[z, t] for z in g["zones"] for t in g["turns"]])


def seeds_all_right(pz):
    """Hand seeds: the speaker mirrored to the left, the core in low_r beside USB-C, the charger group, the latch and the cell's
    connector on the right (J1 slid 0 or 3 mm)."""
    out = []
    spk = [pz.MIRROR_DX, 0.0, "low_l", 0]
    for t in (0, 90, 180, 270):
        for ch in (["rs_mid", 0], ["rs_bot", 0], ["rs_mid", 180], ["ls_mid", 0]):
            for la in (["rs_top", 0], ["end2_r", 0]):
                for dy in (0.0, 3.0):
                    out.append({"speaker": spk, "core": ["low_r", t], "charger": ch, "cellconn": ["rs_top", 0], "latch": la, "usb": dy})
    return out


def stage1(work, keep, jobs, width=40):
    """A beam over the groups in turn (speaker, core, the cell's connector, J1's slide, the charger group, the latch, ...): each stage
    tries every option of one group on every assignment in the beam and keeps the best `width` plus the valid hand seeds."""
    pl = Planner(work)        # builds the base and the caches once
    pz = pl.pz
    t0 = time.time()
    log = open(os.path.join(work, "stage1.log"), "w")
    seen_all = []              # every evaluated assignment (small records), for the report

    def say(msg):
        line = f"[{time.time() - t0:5.0f} s] {msg}"
        print(line, flush=True)
        print(line, file=log, flush=True)

    with _pool(work, jobs) as pool:
        def run(cands, label, chunk=4):
            res, best, n = [], None, 0
            for r in pool.imap_unordered(_eval_assign, cands, chunksize=chunk):
                res.append(r)
                n += 1
                if r["valid"] and (best is None or r["score"] < best):
                    best = r["score"]
                if n % 200 == 0 or n == len(cands):
                    say(f"{label}: {n}/{len(cands)} done, {sum(1 for x in res if x['valid'])} valid, best {best}")
            for r in res:
                a = r["assign"]
                seen_all.append(dict(assign=a, valid=r["valid"], score=r.get("score"), chan=r.get("chan"), why=r.get("why"),
                                     core_at=(r.get("state") or {}).get("core")))
            return res

        speaker_opts = speaker_options(pl)
        say(f"speaker grid: {len(speaker_opts) // 2} legal positions ({SPEAKER_STEP_NOTE}), {len(speaker_opts)} options with the audio group's two turns")
        seeds = seeds_all_right(pz)
        sres = run(seeds, "seeds (speaker left, core low_r, charger/latch/cell connector right)")
        pinned = sorted([r for r in sres if r["valid"]], key=lambda r: r["score"])[:12]
        say(f"{len([r for r in sres if r['valid']])}/{len(seeds)} hand seeds valid, best {pinned[0]['score'] if pinned else None}")
        home = run([{}], "home")
        beam = list(home)
        for name in STAGES:
            if name == "usb":
                opts = [dy for dy in pz.GROUPS["usb"]["dys"] if dy]
            elif name == "hole":
                opts = [[sh[0], sh[1]] for sh in pz.GROUPS["hole"]["shifts"] if tuple(sh) != (0, 0)]
            elif name == "speaker":
                opts = speaker_opts
            else:
                opts = [o for o in options(pz, name) if o is not None]
            cand, keys = [], set()
            late = name in ("ldo", "j5", "boot", "misc")          # the small groups: a narrower beam keeps the run short
            for b in (beam[:8] if late else beam) + pinned[:6 if late else 12]:
                for o in opts:
                    a = dict(b["assign"])
                    a[name] = o
                    k = json.dumps(a, sort_keys=True)
                    if k not in keys:
                        keys.add(k)
                        cand.append(a)
            res = run(cand, f"stage {name}")
            valid = sorted([r for r in res if r["valid"]], key=lambda r: r["score"])
            merged = {json.dumps(r["assign"], sort_keys=True): r for r in valid[:width] + beam[:5]}
            beam = sorted(merged.values(), key=lambda r: r["score"])
            say(f"stage {name}: {len(cand)} tried, {len(valid)} valid, beam best {beam[0]['score']} (chan {beam[0]['chan']})")
    final = {json.dumps(r["assign"], sort_keys=True): r for r in beam + pinned}
    allr = sorted(final.values(), key=lambda r: r["score"])
    # reports: core placements tried in mid_low, best score by speaker side, the seeds' rank
    json.dump([x for x in seen_all if (x["assign"].get("core") or [None])[0] == "mid_low"], open(os.path.join(work, "core_mid_low.json"), "w"))
    side = {"home": None, "left": None, "right": None}
    for x in seen_all:
        if not x["valid"]:
            continue
        sp = x["assign"].get("speaker")
        k = "home" if not sp else ("left" if sp[0] < -1 else "right")
        if side[k] is None or x["score"] < side[k]:
            side[k] = x["score"]
    right = set(pz.RIGHT) | set(pz.STRIPS[3:])
    seedrank = [(i, round(r["score"], 1)) for i, r in enumerate(allr)
                if all(k in r["assign"] and r["assign"][k][0] in right | {"low_l"} for k in ("core", "charger", "latch"))]
    json.dump(dict(assign=allr[:keep], seed_rank=seedrank[:8], best_by_speaker_side=side), open(os.path.join(work, "assign.json"), "w"))
    json.dump(seen_all, open(os.path.join(work, "all_evals.json"), "w"))
    say(f"best score by speaker side: {side}")
    for r in allr[:10]:
        print(round(r["score"], 1), r["chan"], r["len"], r["assign"])
    say(f"all-right seeds in the final ranking (rank, score): {seedrank[:8]}")


def nudge_moves(pl, st, rng, groups):
    """A random small move of a state: returns the new state."""
    s = json.loads(json.dumps(st))
    step = rng.choice(NUDGE_STEPS)
    kind = rng.random()
    names = [n for n in groups]
    g = rng.choice(names)
    refs = [r for r in pl.pz.GROUPS[g]["refs"] if r in s["poses"]] if g != "core" else [r for r in pl.pz.GROUPS["core"]["refs"] if r in s["poses"]]
    dx, dy = rng.choice([(step, 0), (-step, 0), (0, step), (0, -step), (step, step), (-step, -step), (step, -step), (-step, step)])
    if kind < 0.45:                                  # the whole group
        for r in refs:
            p = s["poses"][r]
            s["poses"][r] = [p[0] + dx, p[1] + dy, p[2], p[3]]
        if g == "core" and "core" in s:
            s["core"] = [s["core"][0] + dx, s["core"][1] + dy, s["core"][2]]
    elif kind < 0.85 and refs:                       # one part
        r = rng.choice(refs)
        p = s["poses"][r]
        s["poses"][r] = [p[0] + dx, p[1] + dy, p[2], p[3]]
    elif kind < 0.93 and refs:                       # a part turned a quarter
        r = rng.choice(refs)
        p = s["poses"][r]
        if p[3] == "F":
            s["poses"][r] = [p[0], p[1], (p[2] + rng.choice((90, 270))) % 360, p[3]]
    elif g == "core" and "core" in s:                # the core's turn
        s["core"] = [s["core"][0], s["core"][1], (s["core"][2] + rng.choice((90, 270))) % 360]
    return s


def within_limit(pl, st, ref_st):
    for r, p in st["poses"].items():
        q = ref_st["poses"].get(r) or [pl.ctx.home_P[r][0], pl.ctx.home_P[r][1]]
        if math.dist((p[0], p[1]), (q[0], q[1])) > NUDGE_MAX:
            return False
    c, c0 = st.get("core"), ref_st.get("core")
    if c and c0 and math.dist(c[:2], c0[:2]) > NUDGE_MAX:
        return False
    return True


def _nudge_one(args):
    item, rounds, seed = args
    pl = _PL
    rng = random.Random(seed)
    st0 = item["state"]
    cur, cr = st0, pl.evaluate(st0)
    groups = [n for n in item["assign"] if n in pl.pz.GROUPS and pl.pz.GROUPS[n]["kind"] in ("packed", "core")]
    groups += [n for n in ("spi_r", "c2930") if n not in groups]
    stale = 0
    for it in range(rounds):
        nb = nudge_moves(pl, cur, rng, groups)
        if not within_limit(pl, nb, st0):
            continue
        r = pl.evaluate(nb)
        if r["valid"] and r["score"] < cr["score"] - 1e-6:
            cur, cr, stale = nb, r, 0
        else:
            stale += 1
            if stale > 150:
                break
    return dict(assign=item["assign"], picks=cur, before={k: item[k] for k in ("score", "chan", "left", "right", "top", "len")},
                **{k: cr[k] for k in ("score", "chan", "chan_cap", "left", "right", "top", "len", "cong")})


def stage2(work, top, rounds, jobs, seed=1000, skip=0, out="candidates.json", infile="assign.json"):
    data = json.load(open(os.path.join(work, infile)))["assign"][skip:skip + top]
    with _pool(work, jobs) as pool:
        res = pool.map(_nudge_one, [(it, rounds, seed + i) for i, it in enumerate(data)], chunksize=1)
    res.sort(key=lambda r: r["score"])
    json.dump(res, open(os.path.join(work, out), "w"), indent=0)
    for r in res[:10]:
        print(r["score"], "was", r["before"]["score"], "chan", r["chan"], r["assign"])


# ---------------------------------------------------------------------------------------------------------- measure
def measure(path):
    import pcbnew
    import route
    counts = route.drc_counts(os.path.abspath(path))
    b = pcbnew.LoadBoard(path)
    tl = sum(t.GetLength() for t in b.GetTracks() if t.GetClass() != "PCB_VIA") / 1e6
    vias = sum(1 for t in b.GetTracks() if t.GetClass() == "PCB_VIA")
    print(json.dumps(dict(errors=counts[0] if counts else None, unconnected=counts[1] if counts else None, track_mm=round(tl, 1), vias=vias)))


# ---------------------------------------------------------------------------------------------------------- routing
def parse_hosts(s):
    return [(h.split(":")[0], int(h.split(":")[1])) for h in s.split(",")]


def sh(host, cmd, timeout=600, inp=None):
    if host == "il":
        return subprocess.run(["bash", "-c", cmd], capture_output=True, text=True, timeout=timeout, input=inp)
    return subprocess.run(["ssh", "-o", "ConnectTimeout=15", host, cmd], capture_output=True, text=True, timeout=timeout, input=inp)


def remote_root(host):
    return os.path.expanduser("~/mb_route") if host == "il" else "$HOME/mb_route"


def stage_tree(stage):
    os.makedirs(stage, exist_ok=True)
    subprocess.run(["rsync", "-a", "--exclude", "fab", "--exclude", "pin_review", "--exclude", "__pycache__", "--exclude", "sim",
                    "--exclude", "*.3dshapes", "--exclude", "picks.json", "--exclude", "last_route.*", "--exclude", "freerouting.log",
                    "--exclude", "battler.kicad_pcb", "--exclude", "*.lck", BOARD_DIR + "/", os.path.join(stage, "board") + "/"], check=True)
    os.makedirs(os.path.join(stage, "case"), exist_ok=True)
    shutil.copy(os.path.join(CASE_DIR, "positions.json"), os.path.join(stage, "case", "positions.json"))


def sync_hosts(hosts, run):
    """rsync the board folder, the case's positions.json, the Freerouting jar and the RP2350B reference to each host."""
    stage = os.path.join(tempfile.gettempdir(), f"mbstage_{run}")
    if os.path.exists(stage):
        shutil.rmtree(stage)
    stage_tree(stage)
    for host, _ in hosts:
        root = remote_root(host)
        sh(host, f"mkdir -p {root}/runs {root}/tools {root}/bin {root}/lib")
        if host == "il":
            subprocess.run(["rsync", "-a", "--delete", stage + "/", os.path.expanduser(f"{root}/base_{run}") + "/"], check=True)
            subprocess.run(["rsync", "-a", os.path.expanduser("~/.cache/ref_design_tools/freerouting-1.9.0.jar"), os.path.expanduser(root) + "/tools/"], check=True)
            subprocess.run(["rsync", "-a", os.path.dirname(default_ref()) + "/", os.path.expanduser(root) + "/tools/rp2350b/"], check=True)
            continue
        subprocess.run(["rsync", "-a", "--delete", stage + "/", f"{host}:mb_route/base_{run}/"], check=True)
        subprocess.run(["rsync", "-a", os.path.expanduser("~/.cache/ref_design_tools/freerouting-1.9.0.jar"), f"{host}:mb_route/tools/"], check=True)
        subprocess.run(["rsync", "-a", os.path.dirname(default_ref()) + "/", f"{host}:mb_route/tools/rp2350b/"], check=True)
        if sh(host, "test -d /usr/share/kicad/footprints").returncode:     # no KiCad footprint libraries on the host: a copy of il's, in the user's home
            subprocess.run(["rsync", "-az", "/usr/share/kicad/footprints/", f"{host}:mb_route/footprints/"], check=True)
        if sh(host, "ls $(dirname $(dirname $(readlink -f /usr/bin/java)))/lib | grep -q awt_xawt").returncode:    # a headless JRE: Freerouting's GUI needs AWT
            subprocess.run(["rsync", "-az", os.path.dirname(os.path.dirname(os.path.realpath("/usr/bin/java"))) + "/", f"{host}:mb_route/jdk/"], check=True)       # (the JRE's /etc symlinks are dereferenced below: conf, jvm.cfg, fonts)
            jd = os.path.dirname(os.path.dirname(os.path.realpath("/usr/bin/java")))
            subprocess.run(["rsync", "-azL", jd + "/conf/", f"{host}:mb_route/jdk/conf/"], check=True)
            subprocess.run(["rsync", "-azL", jd + "/lib/jvm.cfg", jd + "/lib/psfontj2d.properties", f"{host}:mb_route/jdk/lib/"], check=True)
        if sh(host, "which xdotool").returncode:           # no xdotool: il's binary and its libxdo, copied into the user's home (no install)
            subprocess.run(["rsync", "-a", "/usr/bin/xdotool", f"{host}:mb_route/bin/"], check=True)
            subprocess.run(["rsync", "-aL", "/lib/x86_64-linux-gnu/libxdo.so.3", f"{host}:mb_route/lib/"], check=True)
    return stage


RUN_ONE = r"""#!/bin/bash
# in the candidate's folder (board/ and case/ beside); args: ID SEED
cd "$(dirname "$0")/board"
export PATH="$HOME/mb_route/bin:$PATH" LD_LIBRARY_PATH="$HOME/mb_route/lib:$LD_LIBRARY_PATH"
export RP2350B_REF="$(ls $HOME/mb_route/tools/rp2350b/*MINIMAL_R4-S1.kicad_pcb | head -1)"
export FREEROUTING_JAR="$HOME/mb_route/tools/freerouting-1.9.0.jar"
[ -d "$HOME/mb_route/footprints" ] && export KICAD_FOOTPRINTS="$HOME/mb_route/footprints"
[ -d "$HOME/mb_route/jdk" ] && export PATH="$HOME/mb_route/jdk/bin:$PATH" JAVA_HOME="$HOME/mb_route/jdk"
echo $$ > ../pid
date +%s > ../t0
echo running > ../status
python3 tools/build_pcb.py > ../build.log 2>&1 || { echo failed-build > ../status; exit 1; }
timeout 2700 python3 tools/route.py --attempts=1 --seed="$2" > ../route.log 2>&1
rc=$?
python3 tools/place_search.py measure battler.kicad_pcb > ../result.json 2> ../measure.err
date +%s > ../t1
echo "done $rc" > ../status
"""


def prefilter(cands, jobs):
    """route.py's own pre-flight DRC on each candidate's built board (a few seconds each, `jobs` at once); the ones with a placement
    error are dropped (they would stop at the pre-flight on a host)."""
    from concurrent.futures import ThreadPoolExecutor
    def one(c):
        r = subprocess.run([sys.executable, os.path.abspath(__file__), "preflight", json.dumps(c["picks"])], capture_output=True, text=True)
        return r.returncode == 0, [l for l in (r.stdout + r.stderr).splitlines() if "DRC errors" in l or "clearance" in l or "shorting" in l][:2]
    with ThreadPoolExecutor(jobs) as pool:
        res = list(pool.map(one, cands))
    keep = []
    for c, (ok, why) in zip(cands, res):
        if ok:
            keep.append(c)
        else:
            print("pre-flight rejects", c.get("tag") or c.get("assign"), why, flush=True)
    print(f"pre-flight: {len(keep)} of {len(cands)} candidates pass", flush=True)
    return keep


def route(args):
    import threading
    work = args["dir"]
    hosts = parse_hosts(args["hosts"])
    cands = json.load(open(os.path.join(work, args.get("file", "candidates.json"))))
    top = cands[args["skip"]:args["skip"] + args["top"]]
    run = args["run"]
    top = prefilter(top, args.get("pre_jobs", 8))
    sync_hosts(hosts, run)
    jobs = [(f"{args.get('prefix', '')}c{args['skip'] + i:03d}s{s}", c, s) for i, c in enumerate(top)
            for s in range(args.get("seed0", 0), args.get("seed0", 0) + args["seeds"])]
    res_dir = os.path.join(work, "routed")
    os.makedirs(res_dir, exist_ok=True)
    state = {"i": 0}
    lock = threading.Lock()
    results = {}

    def worker(host):
        root = remote_root(host)
        while True:
            with lock:
                if state["i"] >= len(jobs):
                    return
                jid, c, seed = jobs[state["i"]]
                state["i"] += 1
            d = f"{root}/runs/{run}_{jid}"
            picks = json.dumps(c["picks"])
            cmd = (f"rm -rf {d}; mkdir -p {d} && cp -r {root}/base_{run}/. {d}/ && printf %s '{picks}' > {d}/board/picks.json && "
                   f"cd {d} && cat > run_one.sh <<'EOS'\n{RUN_ONE}\nEOS\n"
                   f"setsid bash run_one.sh {jid} {seed} < /dev/null > run.out 2>&1 &\nsleep 1; echo started")
            sh(host, cmd)
            t0 = time.time()
            st = ""
            while True:
                time.sleep(40)
                st = sh(host, f"cat {d}/status 2>/dev/null").stdout.strip()
                if st.startswith("done") or st.startswith("failed") or time.time() - t0 > 3000:
                    break
            out = os.path.join(res_dir, jid)
            os.makedirs(out, exist_ok=True)
            if host == "il":
                for f in ("result.json", "route.log", "build.log", "status", "t0", "t1"):
                    if os.path.exists(os.path.expanduser(f"{d}/{f}")):
                        shutil.copy(os.path.expanduser(f"{d}/{f}"), out)
                if os.path.exists(os.path.expanduser(f"{d}/board/battler.kicad_pcb")):
                    shutil.copy(os.path.expanduser(f"{d}/board/battler.kicad_pcb"), out)
                    shutil.copy(os.path.expanduser(f"{d}/board/picks.json"), out)
            else:
                subprocess.run(["rsync", "-a", "--include", "result.json", "--include", "route.log", "--include", "build.log", "--include", "status",
                                "--include", "t0", "--include", "t1", "--include", "board/", "--include", "board/battler.kicad_pcb",
                                "--include", "board/picks.json", "--exclude", "*", f"{host}:mb_route/runs/{run}_{jid}/", out + "/"], capture_output=True)
            rj = os.path.join(out, "result.json")
            r = json.load(open(rj)) if os.path.exists(rj) and os.path.getsize(rj) else {}
            r.update(host=host, id=jid, picks=c["picks"], assign=c.get("assign"), score=c.get("score"), chan=c.get("chan"),
                     len_est=c.get("len"), status=st, secs=int(time.time() - t0))
            results[jid] = r
            json.dump(r, open(os.path.join(out, "summary.json"), "w"))
            print(f"[{host}] {jid} score {c.get('score')}: {r.get('errors')} errors, {r.get('unconnected')} open, "
                  f"{r.get('track_mm')} mm, {r['secs'] // 60} min ({st})", flush=True)
            sh(host, f"rm -rf {d}/board/lib {d}/board/tools/__pycache__")

    ths = []
    for host, n in hosts:
        for k in range(n):
            t = threading.Thread(target=worker, args=(host,), daemon=True)
            t.start()
            ths.append(t)
            time.sleep(2)
    for t in ths:
        t.join()
    json.dump(results, open(os.path.join(work, f"routed_{run}.json"), "w"), indent=1)


def stop(args):
    for host, _ in parse_hosts(args["hosts"]):
        r = sh(host, "for f in $HOME/mb_route/runs/*/pid; do echo $f $(cat $f); done 2>/dev/null")
        for line in r.stdout.splitlines():
            f, pid = line.split()
            if sh(host, f"kill -0 {pid} 2>&1 && echo alive").stdout.strip() == "alive":
                print(host, "kill process group", pid, f)
                sh(host, f"kill -TERM -{pid}")


def collect(work):
    """Re-measure every routed board here (the DRC needs the project's rules beside it) and update its summary."""
    rd = os.path.join(work, "routed")
    for d in sorted(os.listdir(rd)):
        pcb = os.path.join(rd, d, "battler.kicad_pcb")
        sp = os.path.join(rd, d, "summary.json")
        if not (os.path.exists(pcb) and os.path.exists(sp)):
            continue
        for f in ("battler.kicad_pro", "battler.kicad_dru"):
            shutil.copy(os.path.join(BOARD_DIR, f), os.path.join(rd, d, f))
        r = subprocess.run([sys.executable, os.path.abspath(__file__), "measure", pcb], capture_output=True, text=True)
        try:
            m = json.loads(r.stdout.strip().splitlines()[-1])
        except (ValueError, IndexError):
            continue
        s = json.load(open(sp))
        s.update(m)
        json.dump(s, open(sp, "w"))
        print(d, m, flush=True)


def report(work):
    rows = []
    rd = os.path.join(work, "routed")
    for d in sorted(os.listdir(rd)):
        p = os.path.join(rd, d, "summary.json")
        if os.path.exists(p):
            rows.append(json.load(open(p)))
    for r in sorted(rows, key=lambda r: r.get("score") or 0):
        print(f"{r['id']} score {r.get('score')} chan {r.get('chan')} -> errors {r.get('errors')} open {r.get('unconnected')} "
              f"track {r.get('track_mm')} {r['host']} {r['secs'] // 60} min")


def zones_png(out):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    from matplotlib.patches import Rectangle
    import placement as pl
    import placement_zones as pz
    fig, ax = plt.subplots(figsize=(11, 15))
    for name, (x0, y0, x1, y1) in pz.ZONES.items():
        ax.add_patch(Rectangle((x0, y0), x1 - x0, y1 - y0, fill=True, alpha=0.18, ec="blue"))
        users = [g for g, v in pz.GROUPS.items() if name in v.get("zones", [])]
        ax.text(x0 + 0.5, y0 + 1, name + "\n" + "\n".join(users), fontsize=6.5, va="top")
    for c in pl.CUTOUTS:
        x0, y0, x1, y1 = c["box"]
        ax.add_patch(Rectangle((x0, y0), x1 - x0, y1 - y0, fill=True, color="grey", alpha=0.5))
        ax.text((x0 + x1) / 2, (y0 + y1) / 2, c["name"] + " hole", ha="center")
    ax.add_patch(Rectangle((63.8, 24.26), 72.4, 151.48, fill=False, lw=1.5))
    ax.set_xlim(60, 140)
    ax.set_ylim(180, 20)
    ax.set_aspect("equal")
    ax.set_title("placement zones (KiCad mm) and the groups that may use each; J1/U1/R1-R4 slide along the right wall, the hole shifts")
    plt.savefig(out, dpi=100, bbox_inches="tight")


def main():
    a = sys.argv[1:]
    if not a:
        sys.exit(__doc__)
    cmd = a[0]
    opt = {x[2:].split("=", 1)[0]: (x.split("=", 1)[1] if "=" in x else True) for x in a[1:] if x.startswith("--")}
    pos = [x for x in a[1:] if not x.startswith("--")]
    work = opt.get("out") or os.environ.get("PS_WORK", "/tmp/ps/work")
    if cmd == "score":
        pl = Planner(work)
        print(json.dumps(pl.evaluate(json.loads(pos[0]) if pos else {})))
    elif cmd == "stage1":
        stage1(work, int(opt.get("keep", 60)), int(opt.get("jobs", 16)), int(opt.get("width", 40)))
    elif cmd == "holescan":
        holescan(work, [float(x) for x in opt.get("dys", "1,2,3").split(",")], int(opt.get("jobs", 15)), int(opt.get("keep", 8)))
    elif cmd == "stage2":
        stage2(work, int(opt.get("top", 40)), int(opt.get("rounds", 250)), int(opt.get("jobs", 16)), int(opt.get("seed", 1000)), int(opt.get("skip", 0)), opt.get("outfile", "candidates.json"), opt.get("infile", "assign.json"))
    elif cmd == "route":
        route(dict(dir=pos[0], hosts=opt.get("hosts", "il:8"), top=int(opt.get("top", 40)), seeds=int(opt.get("seeds", 1)),
                   skip=int(opt.get("skip", 0)), run=opt.get("run", time.strftime("%m%d%H%M")), file=opt.get("file", "candidates.json"),
                   prefix=opt.get("prefix", ""), seed0=int(opt.get("seed0", 0))))
    elif cmd == "preflight":
        d = os.path.join(tempfile.gettempdir(), "mbpre_" + str(os.getpid()))
        stage_base(d, json.loads(pos[0]) if pos else {})
        env = dict(os.environ, RP2350B_REF=os.environ.get("RP2350B_REF") or default_ref())
        r = subprocess.run([sys.executable, "tools/route.py", "--preflight"], cwd=os.path.join(d, "board"), env=env, capture_output=True, text=True)
        print("\n".join(l for l in (r.stdout + r.stderr).splitlines() if "assert" not in l and "Debug" not in l)[-1500:])
        shutil.rmtree(d, ignore_errors=True)
        sys.exit(r.returncode)
    elif cmd == "measure":
        measure(pos[0])
    elif cmd == "stop":
        stop(dict(hosts=opt.get("hosts", "il:1,xps13:1")))
    elif cmd == "collect":
        collect(pos[0])
    elif cmd == "report":
        report(pos[0])
    elif cmd == "zones-png":
        zones_png(pos[0])
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
