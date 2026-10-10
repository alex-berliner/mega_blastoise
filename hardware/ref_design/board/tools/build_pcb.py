"""Build the board from the schematic, the case and the placement table.

    python3 tools/build_pcb.py            # close the board in KiCad first

Starts from an empty 2-layer board every time:
  1. stack-up, design rules (JLCPCB 2-layer, every hole >= 0.3 mm: smaller ones cost
     extra), net classes
  2. outline and battery cable slot from case/positions.json
  3. every footprint from KiCad's netlist, with its nets, fields and a link to
     its schematic symbol (so KiCad's DRC can check board against schematic)
  4. placement from placement.py
  5. the RP2354B core: placement, tracks, vias and small zones copied from
     Raspberry Pi's RP2350B minimal design (MIT, 2 layers like this board), turned and
     moved onto this board; the reference's flash and its routing stay behind
  6. the reference's routing out of the chip's pins, for every pin used here, as far
     as ESCAPE past the pads (copy_escapes): the router starts from those ends
Routing and pours are done afterwards by tools/route.py.
"""
import math
import os
import sys

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD_DIR = os.path.dirname(HERE)
sys.path.insert(0, HERE)
sys.path.insert(0, BOARD_DIR)

import netlist  # noqa: E402
from placement import CASE, CORE, CORE_AT, CORE_NETS, CORE_ROT, P  # noqa: E402

PCB = os.path.join(BOARD_DIR, "battler.kicad_pcb")
REF = os.path.join(os.path.dirname(BOARD_DIR), "..", "..", "..", "tmp")  # replaced below
REF_PCB = os.environ.get("RP2350B_REF", "")
SYSFP = "/usr/share/kicad/footprints"
LIBS = {"battler": os.path.join(BOARD_DIR, "lib", "battler.pretty")}
NO_ASSEMBLY = ("SW1", "SW2", "SW3", "SW4", "SW5", "SW6", "SW7", "SW8", "SW9", "SW10", "SW11", "SW12",
               "SW13", "SW14", "SW15", "SW16", "TP1", "TP2", "TP3", "TP4", "TP5", "J4",
               "H1", "H2", "H3", "H4", "H5", "H6", "FID1", "FID2", "FID3")


CLASSES = {
    # widths must still leave 0.4-0.5 mm pitch pads (QFNs, the FPC connector) cleanly.
    # Two layers: 3V3 has no plane and is routed like the other supplies (the core's
    # 3V3 pins reach the pour over the core through the copied stubs); GND is the pours
    "PWR": dict(track=0.3, clear=0.15, via=0.6, drill=0.3,
                nets=["VSYS", "VBAT", "VBUS", "+3V3", "/Display/LED_A", "/Audio/SPK_P", "/Audio/SPK_N",
                      "/USB_Charger/VBUS_D5", "/Power/3V3_LDO", "/Display/LED_K1", "/Display/LED_K2"]),
    "USB": dict(track=0.2, clear=0.15, via=0.5, drill=0.3,
                nets=["USB_DP", "USB_DM", "/MCU/USB_DP_MCU", "/MCU/USB_DM_MCU"]),
}
DEFAULT_CLASS = dict(track=0.15, clear=0.15, via=0.5, drill=0.3)
MIN_DRILL = 0.3            # JLC's standard hole: 0.25 mm adds a fee, a 4-wire test and TG155


def net_class(net):
    return next((c for c in CLASSES.values() if net in c["nets"]), DEFAULT_CLASS)


def mm(v):
    return pcbnew.FromMM(v)


def V(x, y):
    return pcbnew.VECTOR2I(mm(x), mm(y))


def xy(v):
    return pcbnew.ToMM(v.x), pcbnew.ToMM(v.y)


def core_xy(x, y):
    """Reference board coordinates -> this board (reference U1 sits at 100,100)."""
    dx, dy = x - 100.0, y - 100.0
    a = math.radians(CORE_ROT)
    # KiCad turns counter-clockwise on screen with Y pointing down
    nx = dx * math.cos(a) + dy * math.sin(a)
    ny = -dx * math.sin(a) + dy * math.cos(a)
    return CORE_AT[0] + nx, CORE_AT[1] + ny


def load_fp(fpid):
    nick, name = fpid.split(":")
    path = LIBS.get(nick) or os.path.join(SYSFP, nick + ".pretty")
    fp = pcbnew.FootprintLoad(path, name)
    if fp is None:
        raise SystemExit(f"footprint {fpid} not found in {path}")
    fp.SetFPID(pcbnew.LIB_ID(nick, name))
    return fp


def setup(board):
    board.SetCopperLayerCount(2)
    ds = board.GetDesignSettings()
    ds.SetBoardThickness(mm(1.6))
    # JLCPCB 2-layer capability with margin; the hole minimum is the price break, not the limit
    ds.m_TrackMinWidth = mm(0.1)
    ds.m_MinClearance = mm(0.1)
    ds.m_ViasMinSize = mm(0.45)
    ds.m_ViasMinAnnularWidth = mm(0.1)
    ds.m_MinThroughDrill = mm(MIN_DRILL)
    ds.m_HoleClearance = mm(0.2)
    ds.m_CopperEdgeClearance = mm(0.2)       # JLCPCB routed-edge minimum
    ds.m_MinResolvedSpokes = 1               # small pads in the pours may have one thermal spoke
    # like Raspberry Pi's board: no mask expansion, so 0.4 mm pitch pads keep their webs
    ds.m_SolderMaskExpansion = 0
    ds.m_SolderMaskMinWidth = 0
    ns = ds.m_NetSettings
    # the project file keeps the last build's classes and patterns: start clean
    ns.ClearNetclassPatternAssignments()
    ns.ClearNetclasses()
    dflt = ns.GetDefaultNetclass()
    dflt.SetClearance(mm(DEFAULT_CLASS["clear"]))
    dflt.SetTrackWidth(mm(DEFAULT_CLASS["track"]))
    dflt.SetViaDiameter(mm(DEFAULT_CLASS["via"]))
    dflt.SetViaDrill(mm(DEFAULT_CLASS["drill"]))
    for name, c in CLASSES.items():
        nc = pcbnew.NETCLASS(name)
        nc.SetTrackWidth(mm(c["track"]))
        nc.SetClearance(mm(c["clear"]))
        nc.SetViaDiameter(mm(c["via"]))
        nc.SetViaDrill(mm(c["drill"]))
        ns.SetNetclass(name, nc)
        for n in c["nets"]:
            ns.SetNetclassPatternAssignment(n, name)
    return CLASSES


def arc(board, c, r, a0, a1):
    s = pcbnew.PCB_SHAPE(board, pcbnew.SHAPE_T_ARC)
    pts = [(c[0] + r * math.cos(math.radians(a)), c[1] + r * math.sin(math.radians(a))) for a in (a0, (a0 + a1) / 2, a1)]
    s.SetArcGeometry(V(*pts[0]), V(*pts[1]), V(*pts[2]))
    s.SetLayer(pcbnew.Edge_Cuts)
    s.SetWidth(mm(0.1))
    board.Add(s)


def line(board, a, b, layer=pcbnew.Edge_Cuts, w=0.1):
    s = pcbnew.PCB_SHAPE(board, pcbnew.SHAPE_T_SEGMENT)
    s.SetStart(V(*a))
    s.SetEnd(V(*b))
    s.SetLayer(layer)
    s.SetWidth(mm(w))
    board.Add(s)


def rounded_rect(board, x0, y0, x1, y1, r):
    line(board, (x0 + r, y0), (x1 - r, y0))
    line(board, (x1, y0 + r), (x1, y1 - r))
    line(board, (x1 - r, y1), (x0 + r, y1))
    line(board, (x0, y1 - r), (x0, y0 + r))
    arc(board, (x1 - r, y0 + r), r, -90, 0)
    arc(board, (x1 - r, y1 - r), r, 0, 90)
    arc(board, (x0 + r, y1 - r), r, 90, 180)
    arc(board, (x0 + r, y0 + r), r, 180, 270)


def outline(board):
    bw, bl, r = CASE["board"]["w"], CASE["board"]["l"], CASE["board"]["r"]
    rounded_rect(board, 100 - bw / 2, 100 - bl / 2, 100 + bw / 2, 100 + bl / 2, r)
    # the board's one cutout (the cell's cable slot): round when its corners are
    # rounded to half its side
    (sx0, sy0), (sx1, sy1) = CASE["slot"]
    r = CASE.get("slot_r", 0.6)
    if abs(sx1 - sx0) <= 2 * r + 1e-6:
        c = pcbnew.PCB_SHAPE(board, pcbnew.SHAPE_T_CIRCLE)
        c.SetCenter(V((sx0 + sx1) / 2, (sy0 + sy1) / 2))
        c.SetRadius(mm(r))
        c.SetLayer(pcbnew.Edge_Cuts)
        c.SetWidth(mm(0.1))
        board.Add(c)
    else:
        rounded_rect(board, sx0, min(sy0, sy1), sx1, max(sy0, sy1), r)


def footprints(board, comps, nets):
    netinfo = {}
    for name in nets:
        ni = pcbnew.NETINFO_ITEM(board, name)
        board.Add(ni)
        netinfo[name] = ni
    pin_net = {}
    for name, nodes in nets.items():
        if name.startswith("unconnected-("):
            continue            # no-connect pins stay netless, as KiCad's own update leaves them
        for ref, pin in nodes:
            pin_net[(ref, pin)] = name
    fps = {}
    for ref, c in sorted(comps.items()):
        fp = load_fp(c["footprint"])
        fp.SetReference(ref)
        fp.SetValue(c["value"])
        fp.SetPath(pcbnew.KIID_PATH(c["path"]))
        for key in ("LCSC", "MPN", "Manufacturer", "BOM", "Datasheet", "Description"):
            if key in c["fields"]:
                fp.SetField(key, c["fields"][key])
        # BOM membership follows the schematic (every symbol is in_bom); the JLC
        # BOM script skips parts without an LCSC number instead
        fp.SetExcludedFromBOM(False)
        fp.SetExcludedFromPosFiles(ref in NO_ASSEMBLY)
        for pad in fp.Pads():
            n = pin_net.get((ref, pad.GetNumber()))
            if n:
                pad.SetNet(netinfo[n])
        board.Add(fp)
        fps[ref] = fp
    return fps, netinfo


def place(fp, x, y, rot, side="F"):
    fp.SetPosition(V(x, y))
    fp.SetOrientationDegrees(rot)
    if side == "B":
        fp.Flip(fp.GetPosition(), pcbnew.FLIP_DIRECTION_LEFT_RIGHT)


def copy_core(board, fps, netinfo):
    """Place the core parts and copy their routing from the reference board."""
    ref = pcbnew.LoadBoard(REF_PCB)
    rfps = {f.GetReference(): f for f in ref.GetFootprints()}
    # placement, and where each reference pad centre lands on this board
    pad_map = {}
    for rref, ours in CORE.items():
        rf = rfps[rref]
        x, y = core_xy(*xy(rf.GetPosition()))
        place(fps[ours], x, y, rf.GetOrientationDegrees() + CORE_ROT)
        new_pads = {}
        for p in fps[ours].Pads():
            new_pads.setdefault(p.GetNumber(), []).append(xy(p.GetPosition()))
        for p in rf.Pads():
            key = tuple(round(v, 3) for v in xy(p.GetPosition()))
            cands = new_pads.get(p.GetNumber(), [])
            if cands:
                tx, ty = core_xy(*key)
                pad_map[key] = min(cands, key=lambda c: (c[0] - tx) ** 2 + (c[1] - ty) ** 2)
    # pads of reference parts we don't copy: routing that touches them on their layer stays
    # behind (the reference has a second flash/PSRAM footprint on the back, under the first)
    foreign = []
    for f in ref.GetFootprints():
        if f.GetReference() in CORE:
            continue
        for p in f.Pads():
            bb = p.GetBoundingBox()
            layers = {l for l in (pcbnew.F_Cu, pcbnew.B_Cu) if p.IsOnLayer(l)}
            foreign.append(((pcbnew.ToMM(bb.GetX()), pcbnew.ToMM(bb.GetY()),
                             pcbnew.ToMM(bb.GetRight()), pcbnew.ToMM(bb.GetBottom())), layers))
    core_box = (86.0, 79.0, 114.5, 117.5)

    def inside(pt, box):
        return box[0] <= pt[0] <= box[2] and box[1] <= pt[1] <= box[3]

    items = []
    for t in ref.GetTracks():
        net = CORE_NETS.get(t.GetNetname())
        if not net:
            continue
        if t.GetClass() == "PCB_VIA":
            p = xy(t.GetPosition())
            if inside(p, core_box) and not any(inside(p, b) for b, _ in foreign):
                # Raspberry Pi's vias are 0.6/0.25: the same pad with a standard 0.3 mm hole
                items.append(("via", net, p, None, pcbnew.ToMM(t.GetWidth(pcbnew.F_Cu)),
                              max(MIN_DRILL, pcbnew.ToMM(t.GetDrillValue()))))
            continue
        s, e = xy(t.GetStart()), xy(t.GetEnd())
        if not (inside(s, core_box) and inside(e, core_box)):
            continue
        if any((inside(s, b) or inside(e, b)) and t.GetLayer() in layers for b, layers in foreign):
            continue
        items.append(("track", net, s, e, pcbnew.ToMM(t.GetWidth()), t.GetLayer()))

    def land(pt):
        k = tuple(round(v, 3) for v in pt)
        return pad_map.get(k) or core_xy(*pt)
    pads = [p for f in fps.values() for p in f.Pads()]

    def hits_foreign_pad(net, a, b, width, layer):
        n = max(2, int(math.dist(a, b) / 0.05))
        for p in pads:
            if not p.IsOnLayer(layer) or p.GetNetname() == net:
                continue
            if p.GetBoundingBox().Distance(V(*a)) > mm(8) and p.GetBoundingBox().Distance(V(*b)) > mm(8):
                continue
            for k in range(n + 1):
                q = (a[0] + (b[0] - a[0]) * k / n, a[1] + (b[1] - a[1]) * k / n)
                if p.HitTest(V(*q), mm(width / 2 + 0.1)):
                    return True
        return False
    # the flash here is a WSON with an exposed pad; the reference's SOIC had none
    before = len(items)
    items = [it for it in items
             if it[0] == "via" or not hits_foreign_pad(it[1], land(it[2]), land(it[3]), it[4], it[5])]
    dropped = before - len(items)

    # Raspberry Pi's board has two layers: a 3V3 pour on top, GND underneath. Routing
    # that ends in those pours is meant to (the 3V3 pins' short stubs into the pour
    # inside the chip's pads); route.py pours 3V3 there on top and GND underneath.
    ux, uy = xy(fps["U4"].GetPosition())
    pour = (ux - 4.5, uy - 4.5, ux + 4.5, uy + 4.5)

    def in_pour(it, pt):
        if it[0] != "track":
            return False
        if it[1] == "GND" and it[5] == pcbnew.B_Cu:
            return True
        return it[1] == "+3V3" and it[5] == pcbnew.F_Cu and inside(land(pt), pour)

    def copied_zone(z):
        bb = z.GetBoundingBox()
        c = (pcbnew.ToMM(bb.GetCenter().x), pcbnew.ToMM(bb.GetCenter().y))
        return pcbnew.ToMM(bb.GetWidth()) <= 13 and pcbnew.ToMM(bb.GetHeight()) <= 13 and inside(c, core_box)
    small_zones = [z for z in (ref.GetArea(i) for i in range(ref.GetAreaCount()))
                   if not z.GetIsRuleArea() and z.GetNetname() in CORE_NETS and copied_zone(z)]

    def zone_layers(net, pt):
        return {l for z in small_zones if CORE_NETS[z.GetNetname()] == net
                for l in (pcbnew.F_Cu, pcbnew.B_Cu) if z.IsOnLayer(l) and z.Outline().Contains(V(*pt))}

    # drop routing left dangling by what we didn't copy (repeat until stable)
    def anchored(pt, skip):
        k = tuple(round(v, 3) for v in pt)
        if k in pad_map:
            return True
        for rf_ in (rfps[r] for r in CORE):
            for p in rf_.Pads():
                if p.HitTest(V(*pt)):
                    return True
        for j, it in enumerate(items):
            if j == skip:
                continue
            if it[0] == "via" and math.dist(it[2], pt) < 0.01:
                return True
            if it[0] == "track" and (math.dist(it[2], pt) < 0.01 or math.dist(it[3], pt) < 0.01):
                return True
        return False
    changed = True
    while changed:
        changed = False
        for i in range(len(items) - 1, -1, -1):
            it = items[i]
            ends = [it[2]] if it[0] == "via" else [it[2], it[3]]
            if it[0] == "via":
                # a via must lead somewhere: tracks on both sides (GND and 3V3 vias join the planes here)
                sides = {o[5] for j, o in enumerate(items) if j != i and o[0] == "track"
                         and (math.dist(o[2], it[2]) < 0.01 or math.dist(o[3], it[2]) < 0.01)}
                sides |= zone_layers(it[1], it[2])
                if len(sides) < 2 and it[1] not in ("GND", "+3V3"):
                    items.pop(i); changed = True
                continue
            if not all(anchored(pt, i) or in_pour(it, pt) for pt in ends):
                items.pop(i); changed = True

    n_t = n_v = 0
    for it in items:
        if it[0] == "track":
            a, b = land(it[2]), land(it[3])
        if it[0] == "via":
            v = pcbnew.PCB_VIA(board)
            v.SetPosition(V(*core_xy(*it[2])))
            v.SetWidth(mm(it[4]))
            v.SetDrill(mm(it[5]))
            v.SetNet(netinfo[it[1]])
            v.SetLocked(True)              # the autorouter must not move Raspberry Pi's core routing
            board.Add(v)
            n_v += 1
        else:
            t = pcbnew.PCB_TRACK(board)
            t.SetStart(V(*a))
            t.SetEnd(V(*b))
            t.SetWidth(mm(it[4]))
            t.SetLayer(it[5])
            t.SetNet(netinfo[it[1]])
            t.SetLocked(True)
            board.Add(t)
            n_t += 1
    # small zones and pour keep-outs around the core (not the big pours)
    n_z = 0
    for i in range(ref.GetAreaCount()):
        z = ref.GetArea(i)
        if not copied_zone(z):
            continue
        if not z.GetIsRuleArea() and z.GetNetname() not in CORE_NETS:
            continue
        nz = pcbnew.ZONE(board)
        nz.SetLayerSet(z.GetLayerSet())
        outline_ = z.Outline().Outline(0)
        pts = [core_xy(*xy(outline_.CPoint(k))) for k in range(outline_.PointCount())]
        nz.Outline().NewOutline()
        for p in pts:
            nz.Outline().Append(mm(p[0]), mm(p[1]))
        if z.GetIsRuleArea():
            nz.SetIsRuleArea(True)
            nz.SetDoNotAllowTracks(z.GetDoNotAllowTracks())
            nz.SetDoNotAllowVias(z.GetDoNotAllowVias())
            nz.SetDoNotAllowZoneFills(z.GetDoNotAllowZoneFills())
            nz.SetDoNotAllowPads(z.GetDoNotAllowPads())
            nz.SetDoNotAllowFootprints(z.GetDoNotAllowFootprints())
        else:
            nz.SetNet(netinfo[CORE_NETS[z.GetNetname()]])
            nz.SetAssignedPriority(z.GetAssignedPriority() + 10)
            nz.SetPadConnection(z.GetPadConnection())
            nz.SetMinThickness(z.GetMinThickness())
            nz.SetLocalClearance(z.GetLocalClearance())
            nz.SetThermalReliefGap(z.GetThermalReliefGap())
            nz.SetThermalReliefSpokeWidth(z.GetThermalReliefSpokeWidth())
        board.Add(nz)
        n_z += 1
    print(f"core: {len(CORE)} parts, {n_t} tracks, {n_v} vias, {n_z} zones copied; "
          f"{dropped} tracks left out (they crossed a pad of another net on this board)")


ESCAPE = 3.2       # mm past the chip's pad ring that Raspberry Pi's pin escapes are copied


def copy_escapes(board, fps, netinfo):
    """Raspberry Pi's routing out of the chip, for the pins used here, as far as ESCAPE
    past the pad ring: each pin's track turns out of its neighbours' way (the left edge's
    eight pins leave as staggered 45 degree lines, SWD west under the crystal, the rest
    south as a bus). Autorouters do this badly on two layers: a pin gets boxed in by its
    neighbours' tracks and vias, and nothing afterwards gets it out. Locked, like the
    core's routing; the router carries on from the ends."""
    ref = pcbnew.LoadBoard(REF_PCB)
    rref = next(r for r, o in CORE.items() if o == "U4")
    ref_pins = {p.GetNumber(): p.GetNetname() for p in ref.FindFootprintByReference(rref).Pads()}
    u4 = fps["U4"]
    skip = {"", "GND", "+3V3", "+1V1"} | set(CORE_NETS.values())
    ux, uy = xy(u4.GetPosition())
    r = 5.3 + ESCAPE
    box = (ux - r, uy - r, ux + r, uy + r)
    pads = [p for f in fps.values() for p in f.Pads()]

    def inside(pt):
        return box[0] <= pt[0] <= box[2] and box[1] <= pt[1] <= box[3]

    def clip(a, b):
        # where a (inside) -> b leaves the box
        t = 1.0
        for i, (lo, hi) in enumerate(((box[0], box[2]), (box[1], box[3]))):
            d = b[i] - a[i]
            if d > 0 and b[i] > hi:
                t = min(t, (hi - a[i]) / d)
            if d < 0 and b[i] < lo:
                t = min(t, (lo - a[i]) / d)
        return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)

    def hits_pad(net, a, b, width):
        n = max(2, int(math.dist(a, b) / 0.05))
        for p in pads:
            if p.GetNetname() == net or not p.IsOnLayer(pcbnew.F_Cu):
                continue
            if p.GetBoundingBox().Distance(V(*a)) > mm(4) and p.GetBoundingBox().Distance(V(*b)) > mm(4):
                continue
            for k in range(n + 1):
                q = (a[0] + (b[0] - a[0]) * k / n, a[1] + (b[1] - a[1]) * k / n)
                if p.HitTest(V(*q), mm(width / 2 + DEFAULT_CLASS["clear"])):
                    return True
        return False

    done = []
    for pad in sorted(u4.Pads(), key=lambda p: int(p.GetNumber())):
        net, num = pad.GetNetname(), pad.GetNumber()
        rnet = ref_pins.get(num)
        if net in skip or not rnet:
            continue
        segs = [(core_xy(*xy(t.GetStart())), core_xy(*xy(t.GetEnd())), pcbnew.ToMM(t.GetWidth()))
                for t in ref.GetTracks()
                if t.GetNetname() == rnet and t.GetClass() != "PCB_VIA" and t.GetLayer() == pcbnew.F_Cu]
        vias = [core_xy(*xy(t.GetPosition())) for t in ref.GetTracks()
                if t.GetNetname() == rnet and t.GetClass() == "PCB_VIA"]
        frontier, used, out = [xy(pad.GetPosition())], set(), []
        while frontier:
            a = frontier.pop()
            for k, (s0, s1, w) in enumerate(segs):
                if k in used or not (math.dist(s0, a) < 0.01 or math.dist(s1, a) < 0.01):
                    continue
                used.add(k)
                b = s1 if math.dist(s0, a) < 0.01 else s0
                end = b if inside(b) else clip(a, b)
                if math.dist(a, end) < 0.05 or hits_pad(net, a, end, w):
                    continue
                out.append((a, end, w))
                if inside(b) and not any(math.dist(b, v) < 0.01 for v in vias):
                    frontier.append(b)
        for a, b, w in out:
            t = pcbnew.PCB_TRACK(board)
            t.SetStart(V(*a))
            t.SetEnd(V(*b))
            t.SetWidth(mm(w))
            t.SetLayer(pcbnew.F_Cu)
            t.SetNet(netinfo[net])
            t.SetLocked(True)
            board.Add(t)
        if out:
            done.append(num)
    print(f"pin escapes from Raspberry Pi's layout: {len(done)} pins ({', '.join(done)})")


def main():
    global REF_PCB
    if not REF_PCB:
        sys.exit("set RP2350B_REF to RPI-RP2350B-MINIMAL_R4-S1.kicad_pcb (Raspberry Pi RP-010329-CA)")
    if os.path.exists(os.path.join(BOARD_DIR, "~battler.kicad_pcb.lck")):
        sys.exit("the board is open in KiCad; close it first")
    comps, nets = netlist.read()
    # a fresh board each time (removing items from a loaded board upsets KiCad's SWIG layer)
    board = pcbnew.NewBoard(PCB)
    setup(board)
    outline(board)
    fps, netinfo = footprints(board, comps, nets)
    missing = [r for r in fps if r not in P and r not in CORE.values()]
    if missing:
        sys.exit(f"no position for {missing}")
    for ref, (x, y, rot, side) in P.items():
        place(fps[ref], x, y, rot, side)
    copy_core(board, fps, netinfo)
    copy_escapes(board, fps, netinfo)
    pcbnew.SaveBoard(PCB, board)
    print("saved", PCB, len(fps), "footprints,", len(nets), "nets")


if __name__ == "__main__":
    main()
