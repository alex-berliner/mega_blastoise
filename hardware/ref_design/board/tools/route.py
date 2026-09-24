"""Planes, plane connections, autorouting and pours for the board build_pcb.py made.

    FREEROUTING_JAR=/path/freerouting-1.9.0.jar python3 tools/route.py [passes] [--reuse]

--reuse imports tools/last_route.ses instead of routing again (valid while the
placed board and the pre-routes are unchanged).

  1. BOM fields hidden (KiCad puts new fields on the silkscreen)
  2. In1 = GND plane, In2 = 3V3 plane; over the RP2350B core a 3V3 pour on the
     top layer, as on Raspberry Pi's board (its keep-outs shape it)
  3. pre-routes that the router handles badly:
       membrane contacts: each comb's anchor -> radial stub -> via outside the pill
       display connector: the twenty grounded RGB pins joined by two bars
       every other GND/3V3 pad: a short stub and a via into its plane
  4. Freerouting (1.9.0; 2.x loops on this board) routes the signals on F.Cu and
     B.Cu. Its copy of the board is adjusted first (see dsn_for_router).
  5. outer GND pours, unused stub vias removed, GND stitching, zones filled
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
from placement import CASE, CORE  # noqa: E402
from build_pcb import core_xy  # noqa: E402

PCB = os.path.join(BOARD_DIR, "battler.kicad_pcb")
JAR = os.environ.get("FREEROUTING_JAR", "")
STUB_VIAS = []
STUB_R = 3.3              # contact via distance from the contact centre (keep-out is 2.9)
VIA_D, VIA_DRILL = 0.5, 0.25
CLEAR = 0.2                 # 0.15 rule + margin: pad outlines are polygons, the check rounds
# Area of Raspberry Pi's core the router must stay out of on F.Cu (reference coordinates):
# the 1.1 V switcher's copper (its four small zones). Everything the router could want
# in there is already connected by the copied routing.
CORE_FENCES_REF = [((101.4, 90.5), (104.4, 95.8))]


def mm(v):
    return pcbnew.FromMM(v)


def V(x, y):
    return pcbnew.VECTOR2I(mm(x), mm(y))


def xy(v):
    return pcbnew.ToMM(v.x), pcbnew.ToMM(v.y)


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


def planes(board):
    outline = board_polygon(0.4)
    add_zone(board, "GND", pcbnew.In1_Cu, outline, 0, "GND plane")
    add_zone(board, "+3V3", pcbnew.In2_Cu, outline, 0, "3V3 plane")
    for l in (pcbnew.In1_Cu, pcbnew.In2_Cu):
        board.SetLayerType(l, pcbnew.LT_POWER)
    for ref, (x, y) in CASE["buttons"].items():
        add_zone(board, None, pcbnew.F_Cu, circle(x, y, 2.9), 0, f"{ref} contact", keepout=("pour", "vias"))
    add_zone(board, "+3V3", pcbnew.F_Cu, rect(*core_box(board)), 2, "3V3 core pour")


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


def contact_stubs(board):
    """Each comb's anchor sits on its outer ring. A radial stub takes it to a via
    outside the silicone footprint; the router starts from there."""
    contacts = []
    for fp in board.GetFootprints():
        if str(fp.GetFPID().GetLibItemName()) != "DSL_button_4ring":
            continue
        cx, cy = xy(fp.GetPosition())
        contacts.append((fp.GetReference(), cx, cy))
        for pad in fp.Pads():
            ax, ay = xy(pad.GetPosition())
            a = math.atan2(ay - cy, ax - cx)
            v = (cx + STUB_R * math.cos(a), cy + STUB_R * math.sin(a))
            add_track(board, (ax, ay), v, pad.GetNet())
            add_via(board, v, pad.GetNet())
            STUB_VIAS.append(v)
    return contacts


def fpc_ground_bars(board):
    """The panel's RGB bus (pins 7-26) is tied to GND: twenty 0.3 mm pads in two
    staggered rows. A bar behind each row joins them and three vias per bar take
    it to the GND plane, outside the connector's body."""
    fp = board.FindFootprintByReference("J3")
    gnd = board.FindNet("GND")
    ox, oy = xy(fp.GetPosition())

    def pad_x(k):
        return 5.7 - (k - 1) * 0.3
    for pins, y_pad, y_bar, y_via in ((range(7, 26, 2), 1.44, 2.35, 3.25), (range(8, 27, 2), -1.44, -2.35, -3.25)):
        xs = [pad_x(k) for k in pins]
        for x in xs:
            add_track(board, (ox + x, oy + y_pad), (ox + x, oy + y_bar), gnd)
        add_track(board, (ox + max(xs), oy + y_bar), (ox + min(xs), oy + y_bar), gnd)
        for x in (max(xs) - 0.3, (max(xs) + min(xs)) / 2, min(xs) + 0.3):
            add_track(board, (ox + x, oy + y_bar), (ox + x, oy + y_via), gnd)
            add_via(board, (ox + x, oy + y_via), gnd)


def box(item):
    b = item.GetBoundingBox()
    return (pcbnew.ToMM(b.GetX()), pcbnew.ToMM(b.GetY()), pcbnew.ToMM(b.GetRight()), pcbnew.ToMM(b.GetBottom()))


class Space:
    """What copper is where, for placing fan-out vias without a DRC run each time."""

    def __init__(self, board):
        self.items = []
        for fp in board.GetFootprints():
            for p in fp.Pads():
                self.items.append((p, p.GetNetname(), box(p)))
        for t in board.GetTracks():
            self.items.append((t, t.GetNetname(), box(t)))
        self.holes = [(xy(fp.GetPosition()), 3.2) for fp in board.GetFootprints()
                      if fp.GetReference().startswith("H")]
        self.contacts = [((x, y), 3.3) for x, y in CASE["buttons"].values()]
        bw, bl = CASE["board"]["w"], CASE["board"]["l"]
        self.edge = (100 - bw / 2 + 0.6, 100 - bl / 2 + 0.6, 100 + bw / 2 - 0.6, 100 + bl / 2 - 0.6)
        (sx0, sy0), (sx1, sy1) = CASE["slot"]
        self.slot = (min(sx0, sx1) - 0.6, min(sy0, sy1) - 0.6, max(sx0, sx1) + 0.6, max(sy0, sy1) + 0.6)
        self.fences = core_fences()

    def clear_at(self, p, r, net):
        x, y = p
        if not (self.edge[0] <= x <= self.edge[2] and self.edge[1] <= y <= self.edge[3]):
            return False
        if self.slot[0] <= x <= self.slot[2] and self.slot[1] <= y <= self.slot[3]:
            return False
        if any(f[0] - r <= x <= f[2] + r and f[1] - r <= y <= f[3] + r for f in self.fences):
            return False
        if any(math.dist(p, c) < cr for c, cr in self.holes + self.contacts):
            return False
        pos = V(x, y)
        reach = r + 0.8
        for item, n, bb in self.items:
            if n == net and item.GetClass() != "PCB_VIA":
                continue
            if x < bb[0] - reach or x > bb[2] + reach or y < bb[1] - reach or y > bb[3] + reach:
                continue
            if item.HitTest(pos, mm(r)):
                return False
        return True

    def add(self, item):
        self.items.append((item, item.GetNetname(), box(item)))


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


def via_near(board, space, origin, net, layer, avoid_angle=None, hit=None):
    """Put a via within ~2 mm of `origin` (x, y) and a stub to it on `layer`;
    `hit(q)` says whether a stub point is still inside the item it starts from."""
    ox, oy = origin
    base = avoid_angle if avoid_angle is not None else 0.0
    for dist in (0.75, 0.95, 1.2, 1.5, 1.9, 2.4):
        for da in (0, 30, -30, 60, -60, 90, -90, 120, -120, 150, -150, 180):
            a = base + math.radians(da)
            p = (ox + dist * math.cos(a), oy + dist * math.sin(a))
            if not space.clear_at(p, VIA_D / 2 + CLEAR, net):
                continue
            n = max(3, int(dist / 0.1))
            path = [(ox + (p[0] - ox) * k / n, oy + (p[1] - oy) * k / n) for k in range(1, n)]
            if all((hit and hit(q)) or space.clear_at(q, 0.1 + CLEAR, net) for q in path):
                netinfo = board.FindNet(net)
                space.add(add_track(board, origin, p, netinfo, layer=layer))
                space.add(add_via(board, p, netinfo))
                return p
    return None


def fanout(board, nets=("GND", "+3V3")):
    """A stub and a via into the plane for every GND/3V3 pad whose tracks don't
    already lead to one."""
    space = Space(board)
    ok = reaches_plane(board, nets)
    done, left = 0, []
    for fp in board.GetFootprints():
        if str(fp.GetFPID().GetLibItemName()) == "DSL_button_4ring":
            continue
        fx, fy = xy(fp.GetPosition())
        for pad in fp.Pads():
            net = pad.GetNetname()
            if net not in nets or pad.GetAttribute() != pcbnew.PAD_ATTRIB_SMD:
                continue
            if (fp.GetReference(), pad.GetNumber()) + xy(pad.GetPosition()) in ok:
                continue
            layer = pcbnew.B_Cu if pad.IsOnLayer(pcbnew.B_Cu) else pcbnew.F_Cu
            px, py = xy(pad.GetPosition())
            base = math.atan2(py - fy, px - fx) if abs(px - fx) + abs(py - fy) > 1e-3 else 0.0
            if via_near(board, space, (px, py), net, layer, base, lambda q, pad=pad: pad.HitTest(V(*q))):
                done += 1
            else:
                left.append(f"{fp.GetReference()}.{pad.GetNumber()}")
    print(f"fan-out: {done} pads given a via; no room at {left}")


def dsn_block_ends(text, i):
    depth, j = 0, i
    while True:
        depth += {"(": 1, ")": -1}.get(text[j], 0)
        j += 1
        if depth == 0:
            return j


def dsn_for_router(text, contacts, fences):
    """Adjust the router's copy of the board (never the board):
    - membrane contacts: KiCad exports each comb as one polygon that also covers the
      gaps between the combs, so a contact's two pads overlap and the router gives up
      on them. Their pins leave the router's job (stubs and vias carry the nets) and
      each contact is fenced off instead.
    - the core's "no copper pour" rule areas (Raspberry Pi's) are exported as full
      keep-outs that wall off the MCU's pins: removed. The switcher's copper and the
      crystal's ground island get proper fences instead."""
    refs = {r for r, _, _ in contacts}

    def drop_pins(m):
        pins = [p for p in m.group(2).split() if p.rsplit("-", 1)[0] not in refs]
        return m.group(1) + " ".join(pins) + m.group(3)
    text = re.sub(r"(\(pins )([^()]*)(\))", drop_pins, text)
    text = re.sub(r"\(image battler:DSL_button_4ring.*?\n    \)",
                  lambda m: re.sub(r"\n\s*\(pin [^\n]*", "", m.group(0)), text, flags=re.S)
    while True:
        i = text.find('(keepout "" (polygon F.Cu')
        if i < 0:
            break
        text = text[:i].rstrip(" ") + text[dsn_block_ends(text, i):].lstrip("\n")
    extra = "".join(f'\n    (keepout "" (circle F.Cu 5800 {x * 1000:.0f} {-y * 1000:.0f}))' for _, x, y in contacts)
    for x0, y0, x1, y1 in fences:
        pts = " ".join(f"{x * 1000:.0f} {-y * 1000:.0f}" for x, y in rect(x0, y0, x1, y1))
        extra += f'\n    (keepout "" (polygon F.Cu 0 {pts}))'
    i = text.index("  (placement")
    j = text.rindex("  )", 0, i)          # end of the structure section
    return text[:j] + extra + "\n" + text[j:]


SAVED_SES = os.path.join(HERE, "last_route.ses")


def freeroute(board, passes, contacts, reuse=False):
    if reuse and os.path.exists(SAVED_SES):
        # same placed board + same pre-routes = the saved session still fits
        print("reusing", SAVED_SES)
        if not pcbnew.ImportSpecctraSES(board, SAVED_SES):
            sys.exit("SES import failed")
        return
    with tempfile.TemporaryDirectory() as tmp:
        dsn, ses = os.path.join(tmp, "b.dsn"), os.path.join(tmp, "b.ses")
        if not pcbnew.ExportSpecctraDSN(board, dsn):
            sys.exit("DSN export failed")
        text = dsn_for_router(open(dsn).read(), contacts, core_fences())
        open(dsn, "w").write(text)
        open(os.path.join(HERE, "last_route.dsn"), "w").write(text)
        if "1.9" in os.path.basename(JAR):
            # Freerouting 1.9 needs a display even in batch mode
            cmd = ["xvfb-run", "-a", "java", "-jar", JAR, "-de", dsn, "-do", ses, "-mp", str(passes)]
        else:
            cmd = ["java", "-Djava.awt.headless=true", "-jar", JAR, "--gui.enabled=false",
                   "-de", dsn, "-do", ses, "-mp", str(passes)]
        print("routing:", " ".join(cmd[-6:]))
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=3 * 3600)
        open(os.path.join(HERE, "freerouting.log"), "w").write(r.stdout + r.stderr)
        if not os.path.exists(ses):
            sys.exit("Freerouting wrote no session; see tools/freerouting.log")
        open(SAVED_SES, "w").write(open(ses).read())
        if not pcbnew.ImportSpecctraSES(board, ses):
            sys.exit("SES import failed")


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
        if v.GetNetname() not in ("GND", "+3V3") and len(layers) < 2:
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
            if not (cx0 <= x <= cx1 and cy0 <= y <= cy1) and space.clear_at((x, y), VIA_D / 2 + 0.25, "GND"):
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


def drc_unconnected(board):
    """Save a scratch copy next to the project (so its rules apply) and ask kicad-cli."""
    import json
    scratch = os.path.join(BOARD_DIR, "_repair.kicad_pcb")
    pro = os.path.join(BOARD_DIR, "_repair.kicad_pro")
    open(pro, "w").write(open(os.path.join(BOARD_DIR, "battler.kicad_pro")).read())
    pcbnew.SaveBoard(scratch, board)
    with tempfile.TemporaryDirectory() as tmp:
        out = os.path.join(tmp, "d.json")
        subprocess.run(["kicad-cli", "pcb", "drc", "--format", "json", "-o", out, scratch], capture_output=True)
        d = json.load(open(out))
    for f in (scratch, pro, os.path.join(BOARD_DIR, "_repair.kicad_prl")):
        if os.path.exists(f):
            os.remove(f)
    return d.get("unconnected_items", [])


def anchor_islands(board, space, nets=("GND", "+3V3")):
    """Every filled island of a GND/3V3 zone on an outer layer gets a via into its plane
    unless it already holds one (or a plated hole)."""
    added = 0
    for z in board.Zones():
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
                        if not space.clear_at(p, VIA_D / 2 + CLEAR, z.GetNetname()):
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
        plane = [u for u in items if any(f"[{n}]" in i["description"] for i in u["items"] for n in ("GND", "+3V3"))]
        for u in plane:
            for it in u["items"]:
                m = re.match(r"Pad (\S+) \[([^\]]+)\] of (\S+)", it["description"])
                if not m:
                    continue
                num, net, ref = m.groups()
                if net not in ("GND", "+3V3"):
                    continue
                fp = board.FindFootprintByReference(ref)
                for pad in fp.Pads():
                    if pad.GetNumber() == num:
                        layer = pcbnew.B_Cu if pad.IsOnLayer(pcbnew.B_Cu) else pcbnew.F_Cu
                        if via_near(board, space, xy(pad.GetPosition()), net, layer, None,
                                    lambda q, pad=pad: pad.HitTest(V(*q))):
                            added += 1
                        break
                break
        print(f"repair round {r + 1}: {len(items)} unconnected ({len(plane)} on planes), {added} vias added")
        if not plane or not added:
            break
    pcbnew.ZONE_FILLER(board).Fill(board.Zones())


def main():
    if not JAR:
        sys.exit("set FREEROUTING_JAR")
    if os.path.exists(os.path.join(BOARD_DIR, "~battler.kicad_pcb.lck")):
        sys.exit("the board is open in KiCad; close it first")
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    passes = int(args[0]) if args else 40
    board = pcbnew.LoadBoard(PCB)
    hide_bom_fields(board)
    planes(board)
    contacts = contact_stubs(board)
    fpc_ground_bars(board)
    fanout(board)
    freeroute(board, passes, contacts, reuse="--reuse" in sys.argv)
    pours(board)
    print("unused stub vias removed:", drop_unused_stub_vias(board, STUB_VIAS))
    print("stitching vias:", stitch(board))
    repair(board)
    pcbnew.SaveBoard(PCB, board)
    print("saved", PCB)


if __name__ == "__main__":
    main()
