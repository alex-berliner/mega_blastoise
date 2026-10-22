"""Pours, ground connections, autorouting for the 2-layer board build_pcb.py made.

    FREEROUTING_JAR=/path/freerouting-1.9.0.jar python3 tools/route.py [passes] [--reuse]

--reuse imports tools/last_route.ses instead of routing again (valid while the
placed board and the pre-routes are unchanged).
--attempts=N routes N copies at once and keeps the first that DRC passes (default 3, or
ROUTE_ATTEMPTS; 1 routes the board in place as before). See attempts().
--finish takes the saved, routed board and only closes what is still open (stubs,
the maze router with a wide search, plane repairs): the step after a full run that
left a connection or two.

  1. BOM fields hidden (KiCad puts new fields on the silkscreen)
  2. over the RP2354B core a 3V3 pour on the top layer, as on Raspberry Pi's
     2-layer board (its keep-outs shape it); GND is the bottom layer, poured last
  3. pre-routes that the router handles badly:
       membrane contacts: each comb's anchor -> radial stub -> via outside the pill
       display connector: the panel's grounded pins joined by a bar per run
       every other GND pad: a short stub and a via down to the GND pour
       USB-C's shell legs: joined in pairs on the back, and a via beside each
  4. Freerouting (1.9.0; 2.x loops on this board) routes the signals and the
     supplies (3V3 included: two layers have no 3V3 plane) on F.Cu and B.Cu. GND is
     left out of its job. (Its per-layer costs would keep B.Cu clearer, but 1.9's
     reader rejects an autoroute_settings block in the structure.) Its copy of the
     board is adjusted first (see dsn_for_router). Any connection it leaves open goes
     to the maze router.
  5. GND pours on both layers, unused stub vias removed, GND stitching, zones filled
kicad-cli's DRC decides whether it is done (tools/check_pcb.py).
"""
import math
import os
import re
import subprocess
import sys
import tempfile

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD_DIR = os.path.dirname(HERE)
sys.path.insert(0, HERE)
sys.path.insert(0, BOARD_DIR)
from placement import CASE, CORE, CUTOUTS  # noqa: E402
from build_pcb import core_layer, core_xy, net_class  # noqa: E402
from maze import Maze  # noqa: E402

PCB = os.path.join(BOARD_DIR, "battler.kicad_pcb")
SCRATCH = BOARD_DIR       # DRC's scratch copy of the board: beside the project file (its rules)
LOGS = HERE               # freerouting.log, last_route.dsn and last_route.ses
JAR = os.environ.get("FREEROUTING_JAR", "")
STUB_VIAS = []
STUB_R = 3.3              # contact via distance from the contact centre
CONTACT_R = 2.9           # nothing but the combs inside this (no pour, no vias, no other tracks)
VIA_D, VIA_DRILL = 0.5, 0.3     # JLC's standard hole: anything smaller costs extra
CLEAR = 0.2                 # 0.15 rule + margin: pad outlines are polygons, the check rounds
# Areas of Raspberry Pi's core the router must stay out of on its top layer (F.Cu, or B.Cu when
# placement.py puts the core on the back; reference coordinates),
# both already connected by the copied routing:
#   the 1.1 V switcher's copper (its four small zones)
#   everything inside the chip's pads (they end 4.5 mm from its centre): the 1.1 V ring
#   around the centre pad and, between it and the pads, the 3V3 pour that the 3V3 pins'
#   stubs reach. Two layers have no 3V3 plane to rescue a pour cut into pieces, so no
#   escape may cross it (as on Raspberry Pi's own 2-layer board): every pin leaves outwards.
CORE_FENCES_REF = [((101.4, 90.5), (104.4, 95.8)), ((95.55, 95.55), (104.45, 104.45))]
PAD_RING = 4.5            # the chip's pads end this far from its centre


def mm(v):
    return pcbnew.FromMM(v)


def V(x, y):
    return pcbnew.VECTOR2I(mm(x), mm(y))


def xy(v):
    return pcbnew.ToMM(v.x), pcbnew.ToMM(v.y)


def footprints(board):
    """Footprints by reference: KiCad's own order changes from one build to the next,
    and the pre-routes must come out the same every time for --reuse to be valid."""
    return sorted(board.GetFootprints(), key=lambda f: (f.GetReference().rstrip("0123456789"),
                                                         int("0" + "".join(c for c in f.GetReference() if c.isdigit()))))


def pads_of(fp):
    return sorted(fp.Pads(), key=lambda p: (p.GetNumber(), xy(p.GetPosition())))


def board_polygon(inset):
    """Board outline as a polygon (corners in 15 degree steps), inset by `inset`."""
    bw, bl, r = CASE["board"]["w"], CASE["board"]["l"], CASE["board"]["r"]
    x0, y0, x1, y1 = 100 - bw / 2 + inset, 100 - bl / 2 + inset, 100 + bw / 2 - inset, 100 + bl / 2 - inset
    r = max(r - inset, 0.1)
    pts = []
    for cx, cy, a0 in ((x1 - r, y0 + r, -90), (x1 - r, y1 - r, 0), (x0 + r, y1 - r, 90), (x0 + r, y0 + r, 180)):
        for k in range(7):
            a = math.radians(a0 + 15 * k)
            pts.append((cx + r * math.cos(a), cy + r * math.sin(a)))
    return pts


def rect(x0, y0, x1, y1):
    return [(min(x0, x1), min(y0, y1)), (max(x0, x1), min(y0, y1)), (max(x0, x1), max(y0, y1)), (min(x0, x1), max(y0, y1))]


def circle(x, y, r, step=15):
    return [(x + r * math.cos(math.radians(a)), y + r * math.sin(math.radians(a))) for a in range(0, 360, step)]


def add_zone(board, net, layer, pts, priority, name, keepout=None):
    z = pcbnew.ZONE(board)
    z.SetLayer(layer)
    z.Outline().NewOutline()
    for x, y in pts:
        z.Outline().Append(mm(x), mm(y))
    if keepout:
        z.SetIsRuleArea(True)
        z.SetDoNotAllowZoneFills("pour" in keepout)
        z.SetDoNotAllowVias("vias" in keepout)
        z.SetDoNotAllowTracks("tracks" in keepout)
        z.SetDoNotAllowPads(False)
        z.SetDoNotAllowFootprints(False)
    else:
        z.SetNet(board.FindNet(net))
        z.SetAssignedPriority(priority)
        z.SetLocalClearance(mm(0.2))
        z.SetMinThickness(mm(0.2))
        z.SetThermalReliefGap(mm(0.25))
        z.SetThermalReliefSpokeWidth(mm(0.3))
        z.SetPadConnection(pcbnew.ZONE_CONNECTION_THERMAL)
    z.SetZoneName(name)
    board.Add(z)
    return z


def core_box(board, grow=0.8):
    xs, ys = [], []
    for ref in CORE.values():
        b = board.FindFootprintByReference(ref).GetBoundingBox(False)
        xs += [pcbnew.ToMM(b.GetX()), pcbnew.ToMM(b.GetRight())]
        ys += [pcbnew.ToMM(b.GetY()), pcbnew.ToMM(b.GetBottom())]
    return min(xs) - grow, min(ys) - grow, max(xs) + grow, max(ys) + grow


def core_fences():
    out = []
    for a, b in CORE_FENCES_REF:
        pa, pb = core_xy(*a), core_xy(*b)
        out.append((min(pa[0], pb[0]), min(pa[1], pb[1]), max(pa[0], pb[0]), max(pa[1], pb[1])))
    return out


PLANE_NETS = ("GND",)       # nets the pours carry; everything else is routed


def planes(board):
    for ref, (x, y) in CASE["buttons"].items():
        add_zone(board, None, pcbnew.F_Cu, circle(x, y, CONTACT_R), 0, f"{ref} contact", keepout=("pour", "vias"))
    u4 = board.FindFootprintByReference("U4")
    cx, cy = xy(u4.GetPosition())
    add_zone(board, "+3V3", core_layer(pcbnew.F_Cu), rect(cx - PAD_RING, cy - PAD_RING, cx + PAD_RING, cy + PAD_RING), 2,
             "3V3 inside the pads")


def pours(board):
    outline = board_polygon(0.4)
    add_zone(board, "GND", pcbnew.F_Cu, outline, 0, "GND top")
    add_zone(board, "GND", pcbnew.B_Cu, outline, 0, "GND bottom")


def hide_bom_fields(board):
    for fp in board.GetFootprints():
        for f in fp.GetFields():
            if f.GetName() in ("LCSC", "MPN", "Manufacturer", "BOM", "Datasheet", "Description"):
                f.SetVisible(False)
                f.SetLayer(pcbnew.F_Fab if fp.GetLayer() == pcbnew.F_Cu else pcbnew.B_Fab)


def add_track(board, a, b, net, width=0.2, layer=pcbnew.F_Cu):
    t = pcbnew.PCB_TRACK(board)
    t.SetStart(V(*a))
    t.SetEnd(V(*b))
    t.SetWidth(mm(width))
    t.SetLayer(layer)
    t.SetNet(net)
    t.SetLocked(True)
    board.Add(t)
    return t


def add_via(board, p, net):
    v = pcbnew.PCB_VIA(board)
    v.SetPosition(V(*p))
    v.SetWidth(mm(VIA_D))
    v.SetDrill(mm(VIA_DRILL))
    v.SetNet(net)
    v.SetLocked(True)
    board.Add(v)
    return v


def core_3v3_stubs(board):
    """Every 3V3 pin of the chip reaches the 3V3 pour between its pads and the 1.1 V ring
    through a short stub inwards. The copied routing has one on most of them; a pin
    without one gets it here (touching the pour's edge isn't enough for the fill)."""
    u4 = board.FindFootprintByReference("U4")
    cx, cy = xy(u4.GetPosition())
    ring = lambda p: max(abs(p[0] - cx), abs(p[1] - cy))
    segs = [(xy(t.GetStart()), xy(t.GetEnd())) for t in board.GetTracks()
            if t.GetClass() != "PCB_VIA" and t.GetNetname() == "+3V3"]
    net = board.FindNet("+3V3")
    added = []
    for pad in pads_of(u4):
        if pad.GetNetname() != "+3V3" or pad.GetAttribute() != pcbnew.PAD_ATTRIB_SMD:
            continue
        inward = any((pad.HitTest(V(*a)) and ring(b) < PAD_RING - 0.1) or (pad.HitTest(V(*b)) and ring(a) < PAD_RING - 0.1)
                     for a, b in segs)
        if inward:
            continue
        px, py = xy(pad.GetPosition())
        dx, dy = px - cx, py - cy
        if abs(dx) > abs(dy):
            end = (cx + math.copysign(PAD_RING - 0.6, dx), py)
        else:
            end = (px, cy + math.copysign(PAD_RING - 0.6, dy))
        add_track(board, (px, py), end, net, 0.2, core_layer(pcbnew.F_Cu))
        added.append(pad.GetNumber())
    print("3V3 stubs into the pour for U4 pins:", added)


def contact_stubs(board):
    """Each comb's anchor sits on its outer ring. A radial stub takes it to a via
    outside the silicone footprint; the router starts from there."""
    contacts = []
    for fp in footprints(board):
        if str(fp.GetFPID().GetLibItemName()) != "DSL_button_4ring":
            continue
        cx, cy = xy(fp.GetPosition())
        contacts.append((fp.GetReference(), cx, cy))
        for pad in pads_of(fp):
            ax, ay = xy(pad.GetPosition())
            a = math.atan2(ay - cy, ax - cx)
            v = (cx + STUB_R * math.cos(a), cy + STUB_R * math.sin(a))
            add_track(board, (ax, ay), v, pad.GetNet())
            add_via(board, v, pad.GetNet())
            STUB_VIAS.append(v)
    return contacts


def usb_shell_ground(board):
    """J1's four shell legs (all pad "SH") are plated holes, so fanout() leaves them to the
    pours, but on the USB tab the routing often cut a leg's patch of pour off (2026-10-20: 5
    runs in 6 failed there). So each side's front and rear legs are joined on the back, and
    each leg gets its own stub and via down into the pours."""
    fp = board.FindFootprintByReference("J1")
    gnd = board.FindNet("GND")
    legs = [p for p in fp.Pads() if p.GetNumber() == "SH"]
    for side in (-1, 1):
        pair = sorted((p for p in legs if (xy(p.GetPosition())[1] - xy(fp.GetPosition())[1]) * side > 0),
                      key=lambda p: p.GetPosition().x)
        if len(pair) == 2:
            add_track(board, xy(pair[0].GetPosition()), xy(pair[1].GetPosition()), gnd, 0.4, pcbnew.B_Cu)
    space = Space(board)
    fx, fy = xy(fp.GetPosition())
    for p in legs:
        px, py = xy(p.GetPosition())
        if not via_near(board, space, (px, py), "GND", pcbnew.B_Cu, math.atan2(py - fy, px - fx),
                        lambda q, p=p: p.HitTest(V(*q)), p):
            print(f"J1 shell leg at ({px:.2f}, {py:.2f}): no room for its via")


def fpc_ground_bar(board):
    """The panel connector's grounded pins (GND, the parallel bus and the RGB interface's
    strobes) come in runs of neighbours: each run is joined by a bar behind its pads (the
    side away from the connector's mouth), with vias every few pins down to the GND pour.
    The signal pins between runs keep their way out."""
    fp = board.FindFootprintByReference("J3")
    gnd = board.FindNet("GND")
    pads = sorted((p for p in fp.Pads() if p.GetNetname() == "GND" and p.GetNumber() != "MP"),
                  key=lambda p: p.GetPosition().x)
    top = min(pcbnew.ToMM(p.GetBoundingBox().GetY()) for p in pads)
    bar_y, via_y = top - 0.45, top - 1.25
    runs, run = [], []
    for p in pads:
        x = xy(p.GetPosition())[0]
        if run and x - run[-1][0] > 0.6:
            runs.append(run)
            run = []
        run.append((x, p))
    runs.append(run)
    for run in runs:
        if len(run) < 2:
            continue                           # a lone ground pin gets its own via in fanout()
        xs = [x for x, _ in run]
        for x, p in run:
            add_track(board, (x, xy(p.GetPosition())[1]), (x, bar_y), gnd, 0.3)
        add_track(board, (xs[0], bar_y), (xs[-1], bar_y), gnd, 0.3)
        for i in range(0, len(xs), 5):
            add_track(board, (xs[i], bar_y), (xs[i], via_y), gnd, 0.3)
            add_via(board, (xs[i], via_y), gnd)


def box(item):
    b = item.GetBoundingBox()
    return (pcbnew.ToMM(b.GetX()), pcbnew.ToMM(b.GetY()), pcbnew.ToMM(b.GetRight()), pcbnew.ToMM(b.GetBottom()))


def exposed_pad(pad):
    """A thermal/exposed pad: vias in it are the point."""
    sz = pad.GetSize(pcbnew.F_Cu)
    return min(pcbnew.ToMM(sz.x), pcbnew.ToMM(sz.y)) >= 1.5


class Space:
    """What copper is where, for placing fan-out vias without a DRC run each time."""

    def __init__(self, board):
        self.items = []
        for fp in board.GetFootprints():
            for p in fp.Pads():
                # a pad may ask for more than the rules (fiducials: 0.6 mm)
                extra = max(0.0, pcbnew.ToMM(p.GetLocalClearance() or 0) - CLEAR)
                self.items.append((p, p.GetNetname(), box(p), extra))
        for t in board.GetTracks():
            self.items.append((t, t.GetNetname(), box(t), 0.0))
        self.holes = [(xy(fp.GetPosition()), 3.2) for fp in board.GetFootprints()
                      if fp.GetReference().startswith("H")]
        self.contacts = [((x, y), 3.3) for x, y in CASE["buttons"].values()]
        bw, bl = CASE["board"]["w"], CASE["board"]["l"]
        self.edge = (100 - bw / 2 + 0.6, 100 - bl / 2 + 0.6, 100 + bw / 2 - 0.6, 100 + bl / 2 - 0.6)
        self.slots = [(b[0] - 0.6, b[1] - 0.6, b[2] + 0.6, b[3] + 0.6) for b in slot_boxes()]
        self.fences = core_fences()

    def clear_at(self, p, r, net, via=False):
        """Nothing of another net within r of p; with via=True no pad of any net under
        the via either (a via on a pad wicks its solder away)."""
        x, y = p
        if not (self.edge[0] <= x <= self.edge[2] and self.edge[1] <= y <= self.edge[3]):
            return False
        if any(s[0] <= x <= s[2] and s[1] <= y <= s[3] for s in self.slots):
            return False
        if any(f[0] - r <= x <= f[2] + r and f[1] - r <= y <= f[3] + r for f in self.fences):
            return False
        if any(math.dist(p, c) < cr for c, cr in self.holes + self.contacts):
            return False
        pos = V(x, y)
        reach = r + 0.8
        for item, n, bb, extra in self.items:
            rr = r + extra
            if n == net and item.GetClass() != "PCB_VIA":
                if not (via and isinstance(item, pcbnew.PAD)) or exposed_pad(item):
                    continue
                rr = VIA_D / 2 + 0.05
            if x < bb[0] - reach - extra or x > bb[2] + reach + extra or y < bb[1] - reach - extra or y > bb[3] + reach + extra:
                continue
            if item.HitTest(pos, mm(rr)):
                return False
        return True

    def add(self, item):
        self.items.append((item, item.GetNetname(), box(item), 0.0))


def reaches_plane(board, nets):
    """Pads of `nets` from which the net's own tracks lead to a via or a plated hole
    (any of which ends up in the plane). Returns a set of (ref, pad number, x, y)."""
    out = set()
    for net in nets:
        tracks = [tr for tr in board.GetTracks() if tr.GetNetname() == net and tr.GetClass() != "PCB_VIA"]
        vias = [xy(v.GetPosition()) for v in board.GetTracks() if v.GetNetname() == net and v.GetClass() == "PCB_VIA"]
        holes = [xy(p.GetPosition()) for fp in board.GetFootprints() for p in fp.Pads()
                 if p.GetNetname() == net and p.GetAttribute() == pcbnew.PAD_ATTRIB_PTH]
        anchors = vias + holes
        # union-find over track endpoints
        parent = {}

        def find(a):
            while parent.setdefault(a, a) != a:
                parent[a] = parent[parent[a]]
                a = parent[a]
            return a

        def key(p):
            return (round(p[0], 2), round(p[1], 2))
        segs = []
        for tr in tracks:
            a, b = xy(tr.GetStart()), xy(tr.GetEnd())
            parent[find(key(a))] = find(key(b))
            segs.append((a, b))
        # T-junctions: an end landing part-way along another track joins it
        for a, b in segs:
            for p in (a, b):
                for c, d in segs:
                    if (c, d) == (a, b):
                        continue
                    L2 = (d[0] - c[0]) ** 2 + (d[1] - c[1]) ** 2
                    if L2 < 1e-9:
                        continue
                    u = max(0.0, min(1.0, ((p[0] - c[0]) * (d[0] - c[0]) + (p[1] - c[1]) * (d[1] - c[1])) / L2))
                    if math.dist(p, (c[0] + u * (d[0] - c[0]), c[1] + u * (d[1] - c[1]))) < 0.01:
                        parent[find(key(p))] = find(key(c))
        good = {find(key(a)) for a in anchors if key(a) in parent}
        for tr in tracks:
            for end in (tr.GetStart(), tr.GetEnd()):
                if key(xy(end)) in parent and find(key(xy(end))) in good:
                    good.add(find(key(xy(end))))
        for fp in board.GetFootprints():
            for p in fp.Pads():
                if p.GetNetname() != net:
                    continue
                if p.GetAttribute() == pcbnew.PAD_ATTRIB_PTH:
                    out.add((fp.GetReference(), p.GetNumber()) + xy(p.GetPosition()))
                    continue
                hit = any(p.HitTest(V(*a)) for a in anchors)
                if not hit:
                    for tr in tracks:
                        for end in (tr.GetStart(), tr.GetEnd()):
                            if p.HitTest(end) and find(key(xy(end))) in good:
                                hit = True
                                break
                        if hit:
                            break
                if hit:
                    out.add((fp.GetReference(), p.GetNumber()) + xy(p.GetPosition()))
    return out


def via_near(board, space, origin, net, layer, avoid_angle=None, hit=None, origin_pad=None):
    """Put a via within ~2 mm of `origin` (x, y) and a stub to it on `layer`;
    `hit(q)` says whether a stub point is still inside the item it starts from."""
    ox, oy = origin
    base = avoid_angle if avoid_angle is not None else 0.0
    for dist in (0.75, 0.95, 1.2, 1.5, 1.9, 2.4):
        for da in (0, 30, -30, 60, -60, 90, -90, 120, -120, 150, -150, 180):
            a = base + math.radians(da)
            p = (ox + dist * math.cos(a), oy + dist * math.sin(a))
            if hit and hit(p) and not (origin_pad is not None and exposed_pad(origin_pad)):
                continue                     # no via inside its own pad (a long pad would wick it dry)
            if not space.clear_at(p, VIA_D / 2 + CLEAR, net, via=True):
                continue
            n = max(3, int(dist / 0.1))
            path = [(ox + (p[0] - ox) * k / n, oy + (p[1] - oy) * k / n) for k in range(1, n)]
            if all((hit and hit(q)) or space.clear_at(q, 0.1 + CLEAR, net) for q in path):
                netinfo = board.FindNet(net)
                space.add(add_track(board, origin, p, netinfo, layer=layer))
                space.add(add_via(board, p, netinfo))
                return p
    return None


def fanout(board, nets=PLANE_NETS):
    """A stub and a via down to the GND pour for every GND pad whose tracks don't
    already lead to one."""
    space = Space(board)
    ok = reaches_plane(board, nets)
    done, left = 0, []
    for fp in footprints(board):
        if str(fp.GetFPID().GetLibItemName()) == "DSL_button_4ring":
            continue
        fx, fy = xy(fp.GetPosition())
        for pad in pads_of(fp):
            net = pad.GetNetname()
            if net not in nets or pad.GetAttribute() != pcbnew.PAD_ATTRIB_SMD:
                continue
            if (fp.GetReference(), pad.GetNumber()) + xy(pad.GetPosition()) in ok:
                continue
            layer = pcbnew.B_Cu if pad.IsOnLayer(pcbnew.B_Cu) else pcbnew.F_Cu
            px, py = xy(pad.GetPosition())
            base = math.atan2(py - fy, px - fx) if abs(px - fx) + abs(py - fy) > 1e-3 else 0.0
            if via_near(board, space, (px, py), net, layer, base, lambda q, pad=pad: pad.HitTest(V(*q)), pad):
                done += 1
            else:
                left.append(f"{fp.GetReference()}.{pad.GetNumber()}")
    print(f"fan-out: {done} pads given a via; no room at {left}")


def edge_box(inset=0.6):
    bw, bl = CASE["board"]["w"], CASE["board"]["l"]
    return (100 - bw / 2 + inset, 100 - bl / 2 + inset, 100 + bw / 2 - inset, 100 + bl / 2 - inset)


def slot_boxes():
    """The board's cutouts (the cell's and the speaker's holes), as (x0, y0, x1, y1)."""
    return [tuple(c["box"]) for c in CUTOUTS]


MAX_RIPPED = 6            # connections the maze pass may take other nets' copper up for


def maze_for(board, net, a, b, reach=6.0, ignore=()):
    c = net_class(net)
    e = edge_box()
    box = (max(min(a[0], b[0]) - reach, e[0]), max(min(a[1], b[1]) - reach, e[1]),
           min(max(a[0], b[0]) + reach, e[2]), min(max(a[1], b[1]) + reach, e[3]))
    return Maze(board, net, box, (pcbnew.F_Cu, pcbnew.B_Cu), c["track"], c["clear"], c["via"], c["drill"],
                fences=core_fences(), fence_layer=core_layer(pcbnew.F_Cu), blocks=slot_boxes(), edge=e,
                ignore=set(ignore),
                discs=[((x, y), CONTACT_R) for x, y in CASE["buttons"].values()])



def maze_connect(board, net, a, la, b, lb, reach=6.0):
    """One connection, a -> b, by tools/maze.py on the outer layers; added locked.
    Returns the new route as [(layer, points), ...] or None."""
    runs = maze_for(board, net, a, b, reach=reach).route(a, la, b, lb)
    if runs:
        add_route(board, net, runs)
    return runs


def add_route(board, net, runs):
    c = net_class(net)
    n = board.FindNet(net)
    for k, (layer, pts) in enumerate(runs):
        for p, q in zip(pts, pts[1:]):
            add_track(board, p, q, n, c["track"], layer)
        if k < len(runs) - 1:
            v = add_via(board, pts[-1], n)
            v.SetWidth(mm(c["via"]))
            v.SetDrill(mm(c["drill"]))
    return runs


def conflicts(item, runs, net):
    """Does `item` (another net's track or via) come too close to a new route?"""
    c = net_class(net)
    for k, (layer, pts) in enumerate(runs):
        if item.IsOnLayer(layer):
            for p, q in zip(pts, pts[1:]):
                n = max(1, int(math.dist(p, q) / 0.02))
                for i in range(n + 1):
                    x, y = p[0] + (q[0] - p[0]) * i / n, p[1] + (q[1] - p[1]) * i / n
                    if item.HitTest(V(x, y), mm(c["clear"] + c["track"] / 2)):
                        return True
        if k < len(runs) - 1 and item.HitTest(V(*pts[-1]), mm(c["clear"] + c["via"] / 2)):
            return True
    return False


def item_by_uuid(board):
    out = {}
    for fp in board.GetFootprints():
        for p in fp.Pads():
            out[p.m_Uuid.AsString()] = p
    for t in board.GetTracks():
        out[t.m_Uuid.AsString()] = t
    return out


def anchor(board, item, other_xy):
    """Where a connection to `item` can start: (x, y, layer)."""
    if isinstance(item, pcbnew.PAD):
        fp = item.GetParentFootprint()
        if str(fp.GetFPID().GetLibItemName()) == "DSL_button_4ring":
            # a membrane contact's comb is enclosed by the other comb: start where its
            # stub ends, outside the contact (contact_stubs)
            (cx, cy), (ax, ay) = xy(fp.GetPosition()), xy(item.GetPosition())
            ang = math.atan2(ay - cy, ax - cx)
            return (cx + STUB_R * math.cos(ang), cy + STUB_R * math.sin(ang), pcbnew.F_Cu)
        layer = pcbnew.F_Cu if item.IsOnLayer(pcbnew.F_Cu) else pcbnew.B_Cu
        return xy(item.GetPosition()) + (layer,)
    if isinstance(item, pcbnew.PCB_VIA):
        return xy(item.GetPosition()) + (pcbnew.F_Cu,)
    ends = [xy(item.GetStart()), xy(item.GetEnd())]
    # a contact's stub ends on the comb inside the contact: start from its outer end
    outside = [e for e in ends if all(math.dist(e, c) > CONTACT_R + 0.05 for c in CASE["buttons"].values())]
    return min(outside or ends, key=lambda e: math.dist(e, other_xy)) + (item.GetLayer(),)


def seg_dist(p, a, b):
    L2 = (b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2
    if L2 < 1e-12:
        return math.dist(p, a)
    u = max(0.0, min(1.0, ((p[0] - a[0]) * (b[0] - a[0]) + (p[1] - a[1]) * (b[1] - a[1])) / L2))
    return math.dist(p, (a[0] + u * (b[0] - a[0]), a[1] + u * (b[1] - a[1])))


def near_router_copper(board, net, points, radius):
    """The router's tracks and vias of other signal nets near `points` (pre-routes and
    copied routing are locked and never taken up)."""
    out = []
    for t in board.GetTracks():
        n = t.GetNetname()
        if t.IsLocked() or n in ("GND", "+3V3", net):
            continue
        if isinstance(t, pcbnew.PCB_VIA):
            d = min(math.dist(xy(t.GetPosition()), p) for p in points)
        else:
            d = min(seg_dist(p, xy(t.GetStart()), xy(t.GetEnd())) for p in points)
        if d < radius:
            out.append(t)
    return out


def open_pairs(board):
    """Open signal connections as (net, (x, y, layer), (x, y, layer)), from KiCad's DRC."""
    pcbnew.ZONE_FILLER(board).Fill(board.Zones())
    items = item_by_uuid(board)
    pairs, odd = [], []
    for u in drc_unconnected(board):
        if any(f"[{n}]" in i["description"] for i in u["items"] for n in PLANE_NETS):
            continue                            # the pours' nets: repair() sees to them
        its = [items.get(i.get("uuid")) for i in u["items"]]
        if len(its) != 2 or None in its:        # a zone: nothing to route to
            odd.append(" / ".join(i["description"] for i in u["items"]))
            continue
        p0, p1 = xy(its[0].GetPosition()), xy(its[1].GetPosition())
        pairs.append((its[0].GetNetname(), anchor(board, its[0], p1), anchor(board, its[1], p0)))
    return pairs, odd


def corridor_ripup(board, net, a, b, tries=12):
    """A long connection that one other net blocks part-way, away from its ends (2026-10-20:
    seat 1's D-pad lines to U4): route it as if one other net's tracks in the way weren't
    there, the nets with the most copper there first, and take up what the new route
    crosses; that net comes back open for the next round. Locked pre-routes stay."""
    reach = 6.0
    x0, y0 = min(a[0], b[0]) - reach, min(a[1], b[1]) - reach
    x1, y1 = max(a[0], b[0]) + reach, max(a[1], b[1]) + reach
    def inside(t):
        bx = box(t)
        return bx[0] < x1 and bx[2] > x0 and bx[1] < y1 and bx[3] > y0
    by_net = {}
    for t in board.GetTracks():
        n = t.GetNetname()
        if n in (net, "") or n in PLANE_NETS or t.IsLocked() or not inside(t):
            continue
        by_net.setdefault(n, []).append(t)
    for other in sorted(by_net, key=lambda n: -len(by_net[n]))[:tries]:
        cand = by_net[other]
        runs = maze_for(board, net, a[:2], b[:2], ignore={t.m_Uuid.AsString() for t in cand}).route(a[:2], a[2], b[:2], b[2])
        if not runs:
            continue
        for t in [t for t in cand if conflicts(t, runs, net)]:
            board.Remove(t)
        add_route(board, net, runs)
        print(f"  {net}: routed through {other}'s copper, which comes back open", flush=True)
        return True
    return False


def finish_open(board, rounds=24):
    """Signal connections Freerouting left open, one at a time by tools/maze.py. When a
    pin is boxed in by the router's tracks for a neighbour, the route is found as if
    those near it weren't there; if it crosses only one other net, that net's crossed
    copper is taken up and comes back as an open connection for the next round (at
    most MAX_RIPPED times: taking up more tends to leave more open than it closes)."""
    done = ripped = 0
    for r in range(rounds):
        pairs, odd = open_pairs(board)
        print(f"  maze round {r + 1}: {len(pairs)} open", flush=True)
        if not pairs:
            break
        took_up = progress = False
        for net, a, b in pairs:
            if maze_connect(board, net, a[:2], a[2], b[:2], b[2]):
                done += 1
                progress = True
                continue
            # a long connection may need to swing wide of the straight line between its ends
            if maze_connect(board, net, a[:2], a[2], b[:2], b[2], reach=20.0):
                done += 1
                progress = True
                continue
            if ripped >= MAX_RIPPED:
                continue
            for radius in (1.2, 2.0):
                cand = near_router_copper(board, net, [a[:2], b[:2]], radius)
                if not cand:
                    continue
                m = maze_for(board, net, a[:2], b[:2], ignore={t.m_Uuid.AsString() for t in cand})
                runs = m.route(a[:2], a[2], b[:2], b[2])
                if not runs:
                    continue
                gone = [t for t in cand if conflicts(t, runs, net)]
                if len({t.GetNetname() for t in gone}) > 1:
                    continue                 # only take up one other net's copper per connection
                ripped += 1
                for t in gone:
                    board.Remove(t)
                add_route(board, net, runs)
                done += 1
                took_up = progress = True
                break
            if not took_up and ripped < MAX_RIPPED and corridor_ripup(board, net, a, b):
                ripped += 1
                done += 1
                took_up = progress = True
            if took_up:
                break                        # what was taken up changes the open list: ask the DRC again
        if not progress:
            break
    pairs, odd = open_pairs(board)
    left = [f"{n}: {a} / {b}" for n, a, b in pairs] + odd
    print(f"open connections finished by the maze router: {done} (nets taken up and re-routed: {ripped}); "
          f"left: {left}")


def dsn_block_ends(text, i):
    depth, j = 0, i
    while True:
        depth += {"(": 1, ")": -1}.get(text[j], 0)
        j += 1
        if depth == 0:
            return j


def fiducial_keepouts(board):
    """Fiducials want 0.6 mm around their copper, which the DSN doesn't carry."""
    out = []
    for fp in footprints(board):
        if str(fp.GetFPID().GetLibItemName()).startswith("Fiducial"):
            for p in fp.Pads():
                r = pcbnew.ToMM(max(p.GetSize(pcbnew.F_Cu).x, p.GetSize(pcbnew.F_Cu).y)) / 2
                out.append(xy(p.GetPosition()) + (r + pcbnew.ToMM(p.GetLocalClearance() or 0) + 0.05,))
    return out


def dsn_for_router(text, contacts, fences, circles=()):
    """Adjust the router's copy of the board (never the board):
    - membrane contacts: KiCad exports each comb as one polygon that also covers the
      gaps between the combs, so a contact's two pads overlap and the router gives up
      on them. Their pins leave the router's job (stubs and vias carry the nets) and
      each contact is fenced off instead.
    - the core's "no copper pour" rule areas (Raspberry Pi's) are exported as full
      keep-outs that wall off the MCU's pins: removed. The switcher's copper and the
      crystal's ground island get proper fences instead."""
    refs = {r for r, _, _ in contacts}
    # Freerouting rounds its geometry and can finish a hair inside the clearance (0.1474 mm
    # against 0.15 at a pad corner): it routes to 5 um more than the board's rules
    text = re.sub(r"\(clearance ([0-9.]+)\)", lambda m: f"(clearance {float(m.group(1)) + 5:g})", text)

    # KiCad keeps copper 0.2 mm from the outer side of the 0.1 mm outline; Freerouting keeps
    # only its clearance from the outline itself: its copy of the outline is 0.12 mm inside
    def shrink(m):
        v = [float(n) for n in m.group(2).split()]
        xs, ys = v[0::2], v[1::2]
        cx, cy = (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2
        hx, hy = (max(xs) - min(xs)) / 2, (max(ys) - min(ys)) / 2
        d = 120.0                                  # um, the DSN's unit
        pts = [f"{cx + (x - cx) * (hx - d) / hx:.1f} {cy + (y - cy) * (hy - d) / hy:.1f}" for x, y in zip(xs, ys)]
        return m.group(1) + "  ".join(pts) + ")"
    text = re.sub(r"(\(boundary\s*\(path pcb 0\s+)([-0-9.\s]+)\)", shrink, text, count=1)

    def drop_pins(m):
        pins = [p for p in m.group(2).split() if p.rsplit("-", 1)[0] not in refs]
        return m.group(1) + " ".join(pins) + m.group(3)
    text = re.sub(r"(\(pins )([^()]*)(\))", drop_pins, text)
    # the pours carry GND: its pins leave the router's job (its fixed copper stays as obstacles)
    for net in PLANE_NETS:
        text = re.sub(r"(\(net " + re.escape(net) + r"\s*\(pins )([^()]*)(\))", r"\1\3", text)
    text = re.sub(r"\(image battler:DSL_button_4ring.*?\n    \)",
                  lambda m: re.sub(r"\n\s*\(pin [^\n]*", "", m.group(0)), text, flags=re.S)
    for layer in ("F.Cu", "B.Cu"):
        while True:
            i = text.find(f'(keepout "" (polygon {layer}')
            if i < 0:
                break
            text = text[:i].rstrip(" ") + text[dsn_block_ends(text, i):].lstrip("\n")
    extra = "".join(f'\n    (keepout "" (circle F.Cu 5800 {x * 1000:.0f} {-y * 1000:.0f}))' for _, x, y in contacts)
    extra += "".join(f'\n    (keepout "" (circle F.Cu {2 * r * 1000:.0f} {x * 1000:.0f} {-y * 1000:.0f}))'
                     for x, y, r in circles)
    fence_layer = "B.Cu" if core_layer(pcbnew.F_Cu) == pcbnew.B_Cu else "F.Cu"
    for x0, y0, x1, y1 in fences:
        pts = " ".join(f"{x * 1000:.0f} {-y * 1000:.0f}" for x, y in rect(x0, y0, x1, y1))
        extra += f'\n    (keepout "" (polygon {fence_layer} 0 {pts}))'
    # the cutouts: the router's copy of the outline doesn't keep its tracks 0.2 mm off
    # their rounded corners, so each gets a keep-out on both layers
    for sx0, sy0, sx1, sy1 in slot_boxes():
        pts = " ".join(f"{x * 1000:.0f} {-y * 1000:.0f}" for x, y in rect(sx0 - 0.3, sy0 - 0.3, sx1 + 0.3, sy1 + 0.3))
        for layer in ("F.Cu", "B.Cu"):
            extra += f'\n    (keepout "" (polygon {layer} 0 {pts}))'
    i = text.index("  (placement")
    j = text.rindex("  )", 0, i)          # end of the structure section
    return text[:j] + extra + "\n" + text[j:]


def saved_ses():
    return os.path.join(LOGS, "last_route.ses")


def freerouting_19(args):
    """Freerouting 1.9 needs a display even in batch mode, and when the design it is
    given carries wiring it can't tidy ("normalization of net ... failed") it waits on
    a warning dialog. Run it on a private Xvfb and click that dialog's OK."""
    import time
    xvfb, n = None, 98
    while xvfb is None:
        # routes running at once can pick the same free display: the Xvfb that finds its lock
        # taken exits, and that route tries the next one
        n += 1
        if n >= 200:
            sys.exit("no free X display for Freerouting")
        if os.path.exists(f"/tmp/.X{n}-lock") or os.path.exists(f"/tmp/.X11-unix/X{n}"):
            continue
        xvfb = subprocess.Popen(["Xvfb", f":{n}", "-screen", "0", "1280x1024x24", "-nolisten", "tcp"],
                                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(2)
        if xvfb.poll() is not None:
            xvfb = None
    env = dict(os.environ, DISPLAY=f":{n}")
    env.pop("XAUTHORITY", None)
    try:
        # in the session's folder: it writes logs/ into its working directory
        java = subprocess.Popen(["java", "-jar", JAR, *args], env=env, stdout=subprocess.PIPE,
                                stderr=subprocess.STDOUT, text=True, cwd=os.path.dirname(args[1]))
        start = time.time()
        seen = set()
        while java.poll() is None:
            time.sleep(5)
            if time.time() - start > 3 * 3600:
                java.kill()
                break
            found = set(subprocess.run(["xdotool", "search", "--name", "DSN file reader"], env=env,
                                       capture_output=True, text=True).stdout.split())
            stuck, seen = found & seen, found      # still there 5 s later: the dialog, not the loader
            for w in stuck:
                geo = dict(line.split("=", 1) for line in subprocess.run(
                    ["xdotool", "getwindowgeometry", "--shell", w], env=env, capture_output=True,
                    text=True).stdout.split())
                x, y, wd, ht = (int(geo.get(k, 0)) for k in ("X", "Y", "WIDTH", "HEIGHT"))
                if wd > 300 and ht > 200:
                    # its OK button sits in the bottom-right corner (a click at the bottom
                    # centre misses it, and Freerouting then waits for ever)
                    subprocess.run(["xdotool", "mousemove", str(x + wd - 44), str(y + ht - 27), "click", "1"],
                                   env=env, capture_output=True)
                    print("  Freerouting warning dialog dismissed", flush=True)
        return java.stdout.read()
    finally:
        xvfb.terminate()


def run_freerouting(board, passes, contacts):
    """Export the board (as it stands: locked copper goes out as fixed wiring, the
    router's own as wiring it may redo), route, import. Returns the session text."""
    with tempfile.TemporaryDirectory() as tmp:
        dsn, ses = os.path.join(tmp, "b.dsn"), os.path.join(tmp, "b.ses")
        if not pcbnew.ExportSpecctraDSN(board, dsn):
            sys.exit("DSN export failed")
        text = dsn_for_router(open(dsn).read(), contacts, core_fences(), fiducial_keepouts(board))
        open(dsn, "w").write(text)
        open(os.path.join(LOGS, "last_route.dsn"), "w").write(text)
        args = ["-de", dsn, "-do", ses, "-mp", str(passes)]
        print("routing:", " ".join(args), flush=True)
        if "1.9" in os.path.basename(JAR):
            log = freerouting_19(args)
        else:
            r = subprocess.run(["java", "-Djava.awt.headless=true", "-jar", JAR, "--gui.enabled=false", *args],
                               capture_output=True, text=True, timeout=3 * 3600, cwd=tmp)
            log = r.stdout + r.stderr
        open(os.path.join(LOGS, "freerouting.log"), "a").write(log)
        if not os.path.exists(ses):
            sys.exit("Freerouting wrote no session; see tools/freerouting.log")
        session = open(ses).read()
        if not pcbnew.ImportSpecctraSES(board, ses):
            sys.exit("SES import failed")
    return session


def freeroute(board, passes, contacts, reuse=False, again=2):
    if reuse and os.path.exists(saved_ses()):
        # same placed board + same pre-routes = the saved session still fits
        print("reusing", saved_ses())
        if not pcbnew.ImportSpecctraSES(board, saved_ses()):
            sys.exit("SES import failed")
        return
    open(os.path.join(LOGS, "freerouting.log"), "w").close()
    session = run_freerouting(board, passes, contacts)
    # what it leaves open often goes through on a second run from its own result
    for _ in range(again):
        pairs, _ = open_pairs(board)
        if not pairs:
            break
        print(f"Freerouting left {len(pairs)} open; running it again from its result", flush=True)
        session = run_freerouting(board, max(10, passes // 2), contacts)
    open(saved_ses(), "w").write(session)


def drop_unused_stub_vias(board, stub_vias):
    """A contact's via is only there for the router; if it connected to the stub
    instead, the via leads nowhere on the far side."""
    gone = 0
    tracks = [t for t in board.GetTracks() if t.GetClass() != "PCB_VIA"]
    for v in [t for t in board.GetTracks() if t.GetClass() == "PCB_VIA"]:
        p = xy(v.GetPosition())
        if not any(math.dist(p, s) < 0.02 for s in stub_vias):
            continue
        layers = {t.GetLayer() for t in tracks if t.GetNetname() == v.GetNetname()
                  and min(math.dist(p, xy(t.GetStart())), math.dist(p, xy(t.GetEnd()))) < 0.05}
        if v.GetNetname() not in PLANE_NETS and len(layers) < 2:
            board.Remove(v)
            gone += 1
    return gone


def stitch(board, pitch=5.0):
    """GND vias on a grid wherever there is room, so the outer pours share the plane."""
    space = Space(board)
    gnd = board.FindNet("GND")
    cx0, cy0, cx1, cy1 = core_box(board, 1.5)
    x0, x1 = 100 - CASE["board"]["w"] / 2 + 2.0, 100 + CASE["board"]["w"] / 2 - 2.0
    y0, y1 = 100 - CASE["board"]["l"] / 2 + 2.0, 100 + CASE["board"]["l"] / 2 - 2.0
    n = 0
    y = y0
    while y <= y1:
        x = x0
        while x <= x1:
            if not (cx0 <= x <= cx1 and cy0 <= y <= cy1) and space.clear_at((x, y), VIA_D / 2 + 0.25, "GND", via=True):
                v = pcbnew.PCB_VIA(board)
                v.SetPosition(V(x, y))
                v.SetWidth(mm(VIA_D))
                v.SetDrill(mm(VIA_DRILL))
                v.SetNet(gnd)
                board.Add(v)
                space.add(v)
                n += 1
            x += pitch
        y += pitch
    return n


def drc_report(board):
    """Save a scratch copy next to the project (so its rules apply) and ask kicad-cli."""
    import json
    scratch = os.path.join(SCRATCH, "_repair.kicad_pcb")
    pro = os.path.join(SCRATCH, "_repair.kicad_pro")
    open(pro, "w").write(open(os.path.join(BOARD_DIR, "battler.kicad_pro")).read())
    pcbnew.SaveBoard(scratch, board)
    with tempfile.TemporaryDirectory() as tmp:
        out = os.path.join(tmp, "d.json")
        subprocess.run(["kicad-cli", "pcb", "drc", "--format", "json", "-o", out, scratch], capture_output=True)
        d = json.load(open(out))
    for f in (scratch, pro, os.path.join(SCRATCH, "_repair.kicad_prl")):
        if os.path.exists(f):
            os.remove(f)
    return d


def pair_key(u):
    return tuple((i["description"], round(i["pos"]["x"], 3), round(i["pos"]["y"], 3)) for i in u["items"])


def drc_unconnected(board):
    # in a fixed order: KiCad's depends on item ids, which change from build to build
    return sorted(drc_report(board).get("unconnected_items", []), key=pair_key)


def prune_dangling(board, rounds=5, locked_too=False):
    """Copper with a loose end: pieces of routes the maze pass took up, or (at the end,
    locked_too) copied stubs whose far end meets nothing. A loose track is cut back to
    the last point where something joins it (the router often meets a stub part-way
    along), and taken away only when nothing does, so no connection is ever lost."""
    gone = 0
    for _ in range(rounds):
        pcbnew.ZONE_FILLER(board).Fill(board.Zones())
        loose = {i["uuid"] for v in drc_report(board).get("violations", [])
                 if v["type"] in ("track_dangling", "via_dangling") for i in v["items"]}
        # a pour's vias stay while the pours are still changing; at the end (locked_too) a
        # dangling one joins nothing on either layer (a pocket the pours never reached)
        items = [t for t in board.GetTracks() if t.m_Uuid.AsString() in loose
                 and (locked_too or (not t.IsLocked() and t.GetNetname() not in PLANE_NETS))
                 and not (isinstance(t, pcbnew.PCB_VIA) and t.GetNetname() in PLANE_NETS and not locked_too)]
        changed = 0
        for t in items:
            if isinstance(t, pcbnew.PCB_VIA):
                board.Remove(t)
                changed += 1
                continue
            span = joined_span(board, t)
            if span is None:
                board.Remove(t)
                changed += 1
            elif span != (xy(t.GetStart()), xy(t.GetEnd())):
                t.SetStart(V(*span[0]))
                t.SetEnd(V(*span[1]))
                changed += 1
        if not changed:
            break
        gone += changed
    return gone


def split_at_joins(board):
    """A track that ends part-way along another of its net makes a T: KiCad counts it as
    connected but calls the end dangling, and the stub past the T is never trimmed. Split
    the other track there so the two meet end to end, and drop a track that exactly
    repeats another. Returns (splits, duplicates dropped)."""
    tracks = [t for t in board.GetTracks() if not isinstance(t, pcbnew.PCB_VIA)]
    splits = 0
    # One pass, with the same splits in the same order as rescanning from the first track after
    # each split: a split adds no new end point (q was one already, b stays as the new track's
    # end), so the tracks before the split one stay clean. ends keeps each net and layer's
    # points in the order a rescan would list them; end_at, where in it each track's end is.
    ends, end_at = {}, {}
    for t in tracks:
        pts = ends.setdefault((t.GetNetname(), t.GetLayer()), [])
        pts += [xy(t.GetStart()), xy(t.GetEnd())]
        end_at[id(t)] = len(pts) - 1
    i = 0
    while i < len(tracks):
        t = tracks[i]
        a, b = xy(t.GetStart()), xy(t.GetEnd())
        pts = ends.get((t.GetNetname(), t.GetLayer()), [])
        q = next((q for q in pts if seg_dist(q, a, b) < 0.001 and math.dist(q, a) > 0.001
                  and math.dist(q, b) > 0.001), None)
        if q is None:
            i += 1
            continue
        u = pcbnew.PCB_TRACK(board)
        u.SetStart(V(*q))
        u.SetEnd(V(*b))
        u.SetWidth(t.GetWidth())
        u.SetLayer(t.GetLayer())
        u.SetNet(t.GetNet())
        u.SetLocked(t.IsLocked())
        board.Add(u)
        t.SetEnd(V(*q))
        tracks.append(u)
        pts[end_at[id(t)]] = xy(t.GetEnd())
        pts += [xy(u.GetStart()), xy(u.GetEnd())]
        end_at[id(u)] = len(pts) - 1
        splits += 1                     # (t again: what is left of it may need another split)
    seen, dropped = set(), 0
    for t in tracks:
        a, b = tuple(round(v, 4) for v in xy(t.GetStart())), tuple(round(v, 4) for v in xy(t.GetEnd()))
        key = (t.GetNetname(), t.GetLayer(), frozenset((a, b)), t.GetWidth())
        if key in seen:
            board.Remove(t)
            dropped += 1
        else:
            seen.add(key)
    return splits, dropped


def joined_span(board, t):
    """The part of track t between the outermost points where other copper of its net
    joins it (other tracks' ends or bodies, vias, pads, the net's pour), or None when
    it adds no connection: a join point only counts if the copper there doesn't also
    touch t somewhere else (a stub's tail that ends inside the track it branched from)."""
    net, layer, w = t.GetNetname(), t.GetLayer(), pcbnew.ToMM(t.GetWidth())
    a, b = xy(t.GetStart()), xy(t.GetEnd())
    L = math.dist(a, b)
    if L < 1e-6:
        return None

    def u_of(p):
        return round(max(0.0, min(L, ((p[0] - a[0]) * (b[0] - a[0]) + (p[1] - a[1]) * (b[1] - a[1])) / L)), 4)
    joins = {}

    def join(u, item):
        joins.setdefault(u, set()).add(item.m_Uuid.AsString())
    me = t.m_Uuid.AsString()      # pcbnew hands out a new wrapper each time: "is" never matches
    for o in board.GetTracks():
        if o.GetNetname() != net or o.m_Uuid.AsString() == me:
            continue
        if isinstance(o, pcbnew.PCB_VIA):
            c, r = xy(o.GetPosition()), pcbnew.ToMM(o.GetWidth(pcbnew.F_Cu)) / 2
            if seg_dist(c, a, b) < r:
                join(u_of(c), o)
            continue
        if not o.IsOnLayer(layer):
            continue
        oa, ob, ow = xy(o.GetStart()), xy(o.GetEnd()), pcbnew.ToMM(o.GetWidth())
        for q in (oa, ob):                          # its end on this track
            if seg_dist(q, a, b) < max(w, ow) / 2:
                join(u_of(q), o)
        for q, u in ((a, 0.0), (b, round(L, 4))):  # this track's end on it
            if seg_dist(q, oa, ob) < max(w, ow) / 2:
                join(u, o)
    for fp in board.GetFootprints():
        for p in fp.Pads():
            if p.GetNetname() != net or not p.IsOnLayer(layer):
                continue
            for q, u in ((a, 0.0), (b, round(L, 4))):
                if p.HitTest(V(*q)):
                    join(u, p)
            c = xy(p.GetPosition())
            if seg_dist(c, a, b) < pcbnew.ToMM(min(p.GetSize(layer).x, p.GetSize(layer).y)) / 2:
                join(u_of(c), p)
    for z in board.Zones():
        if z.GetIsRuleArea() or z.GetNetname() != net or not z.IsOnLayer(layer):
            continue
        fill = z.GetFilledPolysList(layer)
        for q, u in ((a, 0.0), (b, round(L, 4))):
            if fill.Contains(V(*q)):
                join(u, z)
    # an outermost join adds nothing if everything there also touches t elsewhere
    while len(joins) >= 2:
        for u in (min(joins), max(joins)):
            rest = set().union(*(v for k, v in joins.items() if k != u))
            if joins[u] <= rest:
                del joins[u]
                break
        else:
            break
    if len(joins) < 2:
        return None
    lo, hi = min(joins), max(joins)

    def at(u):
        return (a[0] + (b[0] - a[0]) * u / L, a[1] + (b[1] - a[1]) * u / L)
    return (a if lo == 0.0 else at(lo)), (b if hi == round(L, 4) else at(hi))


def anchor_islands(board, space, nets=PLANE_NETS):
    """Every filled island of a GND pour gets a via to the other layer's pour unless it
    already holds one (or a plated hole)."""
    added = 0
    zones = sorted(board.Zones(), key=lambda z: (z.GetZoneName(), z.GetNetname(), z.GetAssignedPriority(),
                                                 xy(z.GetBoundingBox().GetOrigin())))
    for z in zones:
        if z.GetIsRuleArea() or z.GetNetname() not in nets:
            continue
        for layer in (pcbnew.F_Cu, pcbnew.B_Cu):
            if not z.IsOnLayer(layer):
                continue
            fill = z.GetFilledPolysList(layer)
            anchors = [v.GetPosition() for v in board.GetTracks()
                       if v.GetClass() == "PCB_VIA" and v.GetNetname() == z.GetNetname()]
            anchors += [p.GetPosition() for fp in board.GetFootprints() for p in fp.Pads()
                        if p.GetNetname() == z.GetNetname() and p.GetAttribute() == pcbnew.PAD_ATTRIB_PTH]
            for i in range(fill.OutlineCount()):
                outline = fill.Outline(i)
                if any(outline.PointInside(a) for a in anchors):
                    continue
                bb = outline.BBox()
                x0, y0 = pcbnew.ToMM(bb.GetX()), pcbnew.ToMM(bb.GetY())
                w, h = pcbnew.ToMM(bb.GetWidth()), pcbnew.ToMM(bb.GetHeight())
                done = False
                steps = max(2, int(max(w, h) / 0.25))
                for a in range(steps + 1):
                    for b in range(steps + 1):
                        p = (x0 + w * a / steps, y0 + h * b / steps)
                        # the whole via must sit in the island's copper
                        if not all(fill.Contains(V(p[0] + dx, p[1] + dy)) for dx, dy in
                                   ((0, 0), (0.3, 0), (-0.3, 0), (0, 0.3), (0, -0.3))):
                            continue
                        if not space.clear_at(p, VIA_D / 2 + CLEAR, z.GetNetname(), via=True):
                            continue
                        space.add(add_via(board, p, board.FindNet(z.GetNetname())))
                        added += 1
                        done = True
                        break
                    if done:
                        break
    return added


def repair(board, rounds=4):
    """Plane connections the router left out: pads get a stub and via, zone islands a via."""
    for r in range(rounds):
        pcbnew.ZONE_FILLER(board).Fill(board.Zones())
        space = Space(board)
        added = anchor_islands(board, space)
        items = drc_unconnected(board)
        plane = [u for u in items if any(f"[{n}]" in i["description"] for i in u["items"] for n in PLANE_NETS)]
        for u in plane:
            for it in u["items"]:
                # "Pad 3 [GND] of U1", or "PTH pad SH [GND] of J1" for a through-hole pad
                m = re.match(r"(?:PTH )?[Pp]ad (\S+) \[([^\]]+)\] of (\S+)", it["description"])
                if not m:
                    continue
                num, net, ref = m.groups()
                if net not in PLANE_NETS:
                    continue
                fp = board.FindFootprintByReference(ref)
                # the pad DRC means, by where it says it is: several pads can share a number (J1's SH)
                same = [p for p in fp.Pads() if p.GetNumber() == num]
                if same:
                    at = (it["pos"]["x"], it["pos"]["y"]) if "pos" in it else xy(same[0].GetPosition())
                    pad = min(same, key=lambda p: math.dist(xy(p.GetPosition()), at))
                    layer = pcbnew.B_Cu if pad.IsOnLayer(pcbnew.B_Cu) else pcbnew.F_Cu
                    if via_near(board, space, xy(pad.GetPosition()), net, layer, None,
                                lambda q, pad=pad: pad.HitTest(V(*q)), pad):
                        added += 1
                break
        print(f"repair round {r + 1}: {len(items)} unconnected ({len(plane)} on planes), {added} vias added")
        if not plane or not added:
            break
    pcbnew.ZONE_FILLER(board).Fill(board.Zones())


def restore_contact_stubs(board):
    """A contact whose net was left unrouted loses its stub and via to the final clean-up
    (they led nowhere); the maze router's anchor for it is the stub's outer end, so put
    them back first."""
    ends = [xy(t.GetStart()) for t in board.GetTracks() if t.GetClass() != "PCB_VIA"] + \
           [xy(t.GetEnd()) for t in board.GetTracks() if t.GetClass() != "PCB_VIA"]
    back = []
    for fp in footprints(board):
        if str(fp.GetFPID().GetLibItemName()) != "DSL_button_4ring":
            continue
        cx, cy = xy(fp.GetPosition())
        for pad in pads_of(fp):
            if any(pad.HitTest(V(*e)) for e in ends):
                continue
            ax, ay = xy(pad.GetPosition())
            a = math.atan2(ay - cy, ax - cx)
            v = (cx + STUB_R * math.cos(a), cy + STUB_R * math.sin(a))
            add_track(board, (ax, ay), v, pad.GetNet())
            add_via(board, v, pad.GetNet())
            back.append(f"{fp.GetReference()}.{pad.GetNumber()}")
    if back:
        print("contact stubs put back:", back)


def finish():
    """--finish: the routed, saved board again: stubs, open connections and repairs only,
    without routing it all again (the pours stay; the maze router looks past them)."""
    board = pcbnew.LoadBoard(PCB)
    core_3v3_stubs(board)
    restore_contact_stubs(board)
    finish_open(board)
    print("loose router copper removed:", prune_dangling(board))
    repair(board)
    print("tracks split where another meets them part-way, duplicates dropped:", split_at_joins(board))
    print("loose copper removed at the end:", prune_dangling(board, locked_too=True))
    pcbnew.SaveBoard(PCB, board)
    print("saved", PCB)


def drc_counts(path):
    """(errors, unconnected) by check_pcb.py's rules (its cosmetic silkscreen kinds don't count;
    schematic parity is left out: routing doesn't change it)."""
    import json
    from check_pcb import COSMETIC
    with tempfile.TemporaryDirectory() as tmp:
        out = os.path.join(tmp, "drc.json")
        subprocess.run(["kicad-cli", "pcb", "drc", "--format", "json", "--severity-all", "-o", out, path],
                       capture_output=True, cwd=tmp)
        try:
            d = json.load(open(out))
        except (OSError, ValueError):
            return None
    errors = [v for v in d.get("violations", []) if v["severity"] == "error" and v["type"] not in COSMETIC]
    return len(errors), len(d.get("unconnected_items", []))


def attempts(passes, n, argv):
    """Route n copies of the board at once and keep the first that comes out clean. Freerouting
    1.9 has no seed: each connection it routes has a time limit, so runs differ with the
    machine's load, and about one in six left something open. Each attempt is this script on
    its own copy of the board (--board=), with its own Xvfb, logs and session; the first whose
    DRC is clean wins and the rest are stopped. If none is, the one with the fewest problems is
    kept and --finish closes what it can."""
    import shutil
    import signal
    import time
    work = tempfile.mkdtemp(prefix="route_")
    procs, start = {}, time.time()
    for k in range(1, n + 1):
        d = os.path.join(work, f"attempt{k}")
        os.makedirs(d)
        for f in ("battler.kicad_pcb", "battler.kicad_pro"):       # the board in its project (its rules)
            shutil.copy(os.path.join(BOARD_DIR, f), d)
        log = open(os.path.join(d, "route.log"), "w")
        procs[k] = subprocess.Popen([sys.executable, os.path.abspath(__file__), str(passes), *argv,
                                     f"--board={os.path.join(d, 'battler.kicad_pcb')}"], cwd=d,
                                    stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
    print(f"routing {n} attempts at once ({work}; ROUTE_ATTEMPTS or --attempts= sets how many)", flush=True)
    results, winner = {}, None
    try:
        while procs and winner is None:
            time.sleep(2)
            for k, p in sorted(procs.items()):
                if p.poll() is None:
                    continue
                del procs[k]
                d = os.path.join(work, f"attempt{k}")
                results[k] = drc_counts(os.path.join(d, "battler.kicad_pcb")) if p.returncode == 0 else None
                mins = (time.time() - start) / 60
                if results[k] is None:
                    print(f"  attempt {k}: route.py failed after {mins:.1f} min (exit {p.returncode}), "
                          f"see {d}/route.log", flush=True)
                    continue
                print(f"  attempt {k}: {results[k][0]} DRC errors, {results[k][1]} unconnected after {mins:.1f} min",
                      flush=True)
                if results[k] == (0, 0):
                    winner = k
                    break
    finally:
        for p in procs.values():           # each attempt's own session: route.py, Freerouting, Xvfb
            try:
                os.killpg(p.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        for p in procs.values():
            p.wait()
    done = {k: v for k, v in results.items() if v is not None}
    if winner is None:
        if not done:
            sys.exit(f"every attempt failed: see the logs in {work}")
        winner = min(done, key=lambda k: (sum(done[k]), k))
    d = os.path.join(work, f"attempt{winner}")
    print(f"attempt {winner} kept; its output:")
    print("    " + open(os.path.join(d, "route.log")).read().rstrip().replace("\n", "\n    "), flush=True)
    shutil.copy(os.path.join(d, "battler.kicad_pcb"), PCB)
    for f in ("last_route.ses", "last_route.dsn", "freerouting.log"):
        if os.path.exists(os.path.join(d, f)):
            shutil.copy(os.path.join(d, f), os.path.join(LOGS, f))
    print("saved", PCB)
    shutil.rmtree(work, ignore_errors=True)
    if done[winner] != (0, 0):
        print("no attempt came out clean: closing what is left open (as route.py --finish)", flush=True)
        finish()


def main():
    global PCB, SCRATCH, LOGS
    if "--finish" in sys.argv:
        return finish()
    if not JAR:
        sys.exit("set FREEROUTING_JAR")
    if os.path.exists(os.path.join(BOARD_DIR, "~battler.kicad_pcb.lck")):
        sys.exit("the board is open in KiCad; close it first")
    opts = dict(a[2:].split("=", 1) for a in sys.argv[1:] if a.startswith("--") and "=" in a)
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    passes = int(args[0]) if args else 40
    if "board" in opts:                 # one of several attempts: a copy of the board in its own folder
        PCB = opts["board"]
        SCRATCH = LOGS = os.path.dirname(PCB)
    else:
        n = int(opts.get("attempts", os.environ.get("ROUTE_ATTEMPTS", "3")))
        if n > 1 and "--reuse" not in sys.argv:
            return attempts(passes, n, [a for a in sys.argv[1:] if a.startswith("--") and "=" not in a])
    board = pcbnew.LoadBoard(PCB)
    hide_bom_fields(board)
    planes(board)
    core_3v3_stubs(board)
    contacts = contact_stubs(board)
    fpc_ground_bar(board)
    usb_shell_ground(board)
    fanout(board)
    freeroute(board, passes, contacts, reuse="--reuse" in sys.argv)
    finish_open(board)
    print("loose router copper removed:", prune_dangling(board))
    pours(board)
    print("unused stub vias removed:", drop_unused_stub_vias(board, STUB_VIAS))
    print("stitching vias:", stitch(board))
    repair(board)
    print("tracks split where another meets them part-way, duplicates dropped:", split_at_joins(board))
    print("loose copper removed at the end:", prune_dangling(board, locked_too=True))
    pcbnew.SaveBoard(PCB, board)
    print("saved", PCB)


if __name__ == "__main__":
    main()
