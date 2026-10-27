#!/usr/bin/env python3
"""Layout checks of classes 7, 10, 12 and 13, from the routed board (battler.kicad_pcb, through pcbnew).

    python3 hardware/vv/tools/layout_checks.py <check>           # exit 1 when the check's limit fails, 2 BLOCKED
    python3 hardware/vv/tools/layout_checks.py <check> --seed    # the same check on a seeded fault: must exit 1
    python3 hardware/vv/tools/layout_checks.py --list

  bridges   VER-148  every pair of pads on different nets closer than 0.35 mm against hardware/vv/bridges.csv, the
                     reviewed table that classes each net pair benign, self-test or dangerous (a dangerous pair names
                     a live DFMEA row); --write-table rewrites the table's proposal from the rules below

Nothing is written under hardware/ref_design; seeds move a pad or a track in memory only.

Python 3 standard library and pcbnew (KiCad 10).
"""
import csv
import math
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
VV = os.path.dirname(HERE)
sys.path.insert(0, HERE)
import design_numbers as dn  # noqa: E402


class Rows:
    def __init__(self):
        self.bad = []

    def __call__(self, good, text):
        print(("  ok    " if good else "  FAIL  ") + text)
        if not good:
            self.bad.append(text)
        return good


def net(p):
    n = p.GetNetname().split("/")[-1]
    m = re.match(r"unconnected-\((\w+)-.*-Pad(\w+)\)", n)
    return f"NC:{m.group(1)}.{m.group(2)}" if m else n


# ---- VER-148: adjacent-pad bridges --------------------------------------------------------------------------------
BRIDGE_MM = 0.35
TABLE = os.path.join(VV, "bridges.csv")
COLS = ["net_a", "net_b", "class", "df", "pads", "reason"]
POWER = {"VBUS", "VSYS", "VBAT", "CELL_P", "AMP_VDD", "LED_A"}
TIES = {frozenset(x) for x in (("+3V3", "3V3_LDO"), ("+3V3", "LCD_3V3"), ("AMP_VDD", "VSYS"), ("CELL_P", "VBAT"),
                               ("LED_A", "VSYS"))}


def close_pairs(b, pcb, lim_mm=BRIDGE_MM, moved=None):
    """[(net_a, net_b, 'REF.pad-REF.pad', distance mm)] for pads on different nets closer than lim_mm on a shared
    copper layer (an unnumbered, netless pad is a hole, not copper)."""
    lim = pcb.FromMM(lim_mm)
    pads = [(f.GetReference(), p) for f in b.GetFootprints() for p in f.Pads()
            if (p.GetNumber() or p.GetNetname()) and (p.IsOnLayer(pcb.F_Cu) or p.IsOnLayer(pcb.B_Cu))]
    if moved:                                 # a seed: one pad put on another net, in memory
        for ref, p in pads:
            if (ref, p.GetNumber()) == moved[0]:
                p.SetNet(b.FindNet(moved[1]))
    boxes = [(r, p, p.GetBoundingBox()) for r, p in pads]
    boxes.sort(key=lambda t: t[2].GetLeft())
    out = []
    for i, (ri, pi, bi) in enumerate(boxes):
        for rj, pj, bj in boxes[i + 1:]:
            if bj.GetLeft() > bi.GetRight() + lim:
                break
            if bj.GetTop() > bi.GetBottom() + lim or bi.GetTop() > bj.GetBottom() + lim:
                continue
            na, nb = net(pi), net(pj)
            if na == nb:
                continue
            for layer in (pcb.F_Cu, pcb.B_Cu):
                if not (pi.IsOnLayer(layer) and pj.IsOnLayer(layer)):
                    continue
                sa, sb = pi.GetEffectiveShape(layer), pj.GetEffectiveShape(layer)
                if not sa.Collide(sb, lim):
                    continue
                lo, hi = 0, lim                    # bisect the gap with the native collision test
                for _ in range(20):
                    mid = (lo + hi) // 2
                    if sa.Collide(sb, mid):
                        hi = mid
                    else:
                        lo = mid + 1
                out.append((min(na, nb), max(na, nb), f"{ri}.{pi.GetNumber()}-{rj}.{pj.GetNumber()}", pcb.ToMM(hi)))
                break
    return out


def propose(a, b):
    """The rule that classes a net pair: (class, df, reason). Only a proposal: the committed table is what counts."""
    s = {a, b}
    nc = [x for x in s if x.startswith("NC:")]
    if frozenset(s) in TIES:
        return "benign", "", "a solder jumper's own two pads: bridged in copper by design"
    if any(x in ("NC:U4.70", "NC:U4.71", "NC:U4.72", "NC:U4.73", "NC:U4.74") for x in nc):
        return "self-test", "DF-72", "an RP2354B QSPI pin wired to the flash inside the package: a bridge stops the boot (unit 1's first article)"
    if "NC:U2.14" in s:
        return "dangerous", "DF-152", "U2's TMR: to VSS it disables every safety timer (bq24074.txt:566-567), to IN it upsets them"
    if any(x.startswith("NC:U8.7") or x.startswith("NC:U9.7") for x in nc):
        return "self-test", "DF-163", "a 74HC165's unused QH' output fighting its neighbour: a stuck button or SR_DATA (button test)"
    if nc:
        return "benign", "", "an unconnected pin: nothing on it"
    if s == {"VBAT", "GND"}:
        return "dangerous", "DF-148", "the cell shorted at U2 (BAT to the exposed pad)"
    if s == {"CHG_ISET", "CHG_TEMP"}:
        return "dangerous", "DF-146", "ISET's 2.5 V on TS holds it inside the charge window whatever the cell's temperature"
    if s == {"CHG_ISET", "GND"}:
        return "dangerous", "DF-06", "RISET shorted: the charge current unset"
    if s in ({"CHG_OFF", "VBAT"}, {"CHG_STAT_N", "VSYS"}):
        return "dangerous", "DF-149", "a U2 neighbour pair putting the cell or VSYS on a GPIO (over IOVDD + 0.5 V, and it back-powers 3V3 when off)"
    if s == {"VSYS", "GND"}:
        return "dangerous", "DF-149", "VSYS shorted to U2's pad: the cell through U2's battery FET, no fuse (DF-172)"
    if s in ({"CHG_ILIM", "VBUS"}, {"CHG_ILIM", "VSYS"}):
        return "self-test", "DF-149", "ILIM is unused in USB500 and rated 7 V (bq24074.txt:598-600): a supply on it is inside its rating; VER-66 sees a changed limit"
    if s == {"VBUS", "GND"}:
        return "dangerous", "DF-51", "VBUS shorted at J1's row (DF-51) or U2's IN to its pad (DF-149): the source's limit is the only stop"
    if s == {"CC2", "VBUS"} or s == {"CC1", "VBUS"}:
        return "dangerous", "DF-51", "VBUS on a CC pin: GPIO45/46 over their limit through 10k (VER-195)"
    if "+1V1" in s:
        return "dangerous", "DF-72", "DVDD (1.21 V absolute, rp2350.txt:101023) joined to a pin driven at 3.3 V or to IOVDD"
    if s & {"VREG_LX", "VREG_AVDD"}:
        return "dangerous", "DF-72", "the core regulator's switch node or analogue supply shorted"
    if s == {"+3V3", "GND"}:
        return "dangerous", "DF-162", "3V3 shorted at a 74HC165 (CLK INH to VCC): U3 in current limit"
    if s == {"LCD_3V3", "GND"}:
        return "dangerous", "DF-59", "the panel's supply shorted at J3: U3 in current limit through JP4"
    if "GND" in s and re.fullmatch(r"P[12]_(UP|DOWN|LEFT|RIGHT|A|B|X|Y)", (s - {"GND"}).pop()):
        return "self-test", "", "a contact's own comb gap (0.20 mm by design, REQ-BTN-03), or a 74HC165 input beside CLK INH: a held button (the button test)"
    if "+3V3" in s and "VBAT_SENSE" in s:
        return "self-test", "DF-72", "U10's output held at 3V3: VBAT reads 6.6 V, a value the firmware can refuse (VER-67 on unit 1)"
    if s & {"CC1", "CC2", "USB_DP", "USB_DM", "USB_DP_MCU", "USB_DM_MCU"}:
        return "self-test", "DF-51", "a USB or CC line: enumeration or the source's offer fails (bring-up)"
    if s & {"LED_A", "LED_K1", "LED_K2"}:
        return "self-test", "DF-59", "a backlight string shorted or paralleled: visible at the display test"
    if s & {"CHG_TEMP", "CHG_EN1", "CHG_PG_N", "CHG_STAT_N", "CHG_OFF", "CHG_ILIM"}:
        return "self-test", "DF-149", "a charger control or status pin held: the charge test (VER-66) or the status read sees it"
    return "self-test", "", "two signals or a signal and a rail the firmware drives and reads: the bring-up and self-test see it"


SEED_BRIDGES = (("U2", "14"), "VSYS")        # U2's TMR pad on VSYS: VBUS-VSYS beside it, a pair no row lists


def bridges(seed=None):
    pcb, b = dn.board()
    pairs = close_pairs(b, pcb, moved=seed)
    byn = {}
    for a, c, pads, d in pairs:
        byn.setdefault((a, c), []).append((pads, d))
    print(f"{len(pairs)} pad pairs on {len(byn)} net pairs closer than {BRIDGE_MM} mm (pcbnew's own shapes, per copper layer)")
    if "--write-table" in sys.argv:
        rows = []
        for (a, c), lst in sorted(byn.items()):
            cl, df, why = propose(a, c)
            rows.append([a, c, cl, df, " ".join(sorted(p for p, _ in lst)), why])
        with open(TABLE, "w", newline="", encoding="utf-8") as f:
            w = csv.writer(f, lineterminator="\n")
            w.writerow(COLS)
            w.writerows(rows)
        print(f"wrote {os.path.relpath(TABLE, dn.ROOT)}: {len(rows)} net pairs (a proposal: review each row)")
        return 0
    row = Rows()
    if not os.path.exists(TABLE):
        print("BLOCKED: hardware/vv/bridges.csv is missing\nRESULT: BLOCKED")
        return 2
    table = {(r["net_a"], r["net_b"]): r for r in csv.DictReader(open(TABLE, encoding="utf-8"))}
    live = {}
    for r in csv.DictReader(open(os.path.join(VV, "dfmea.csv"), encoding="utf-8")):
        live[r["id"]] = r.get("status", "") != "RETIRED" and not r.get("failure_mode", "").startswith("RETIRED")
    counts = {}
    for key, lst in sorted(byn.items()):
        t = table.get(key)
        pads = ", ".join(f"{p} {d:.3f}" for p, d in lst)
        if not t:
            row(False, f"{key[0]} - {key[1]} ({pads}): not in bridges.csv")
            continue
        counts[t["class"]] = counts.get(t["class"], 0) + 1
        if t["class"] not in ("benign", "self-test", "dangerous"):
            row(False, f"{key[0]} - {key[1]}: class {t['class']!r}")
        if t["class"] == "dangerous":
            row(bool(t["df"]) and live.get(t["df"], False),
                f"dangerous {key[0]} - {key[1]} ({pads}): {t['df'] or 'no DF row'}: {t['reason']}")
    gone = sorted(set(table) - set(byn))
    for key in gone:
        row(False, f"{key[0]} - {key[1]}: in bridges.csv but no longer a close pair (stale row)")
    print("net pairs by class: " + ", ".join(f"{k} {v}" for k, v in sorted(counts.items())))
    print(f"{len(row.bad)} failing")
    return dn.result(not row.bad)


# ---- VER-132: the fab files' content -----------------------------------------------------------------------------
LAYER_FILES = ["F_Cu.gtl", "B_Cu.gbl", "F_Mask.gts", "B_Mask.gbs", "F_Paste.gtp", "B_Paste.gbp", "F_Silkscreen.gto",
               "B_Silkscreen.gbo", "Edge_Cuts.gm1"]
COMB_D, KEEP_R, MIN_HOLE, EDGE_CU = 4.8, 2.9, 0.30, 0.30
MARKER = "JLCJLCJLCJLC"


def gerber(path):
    """(apertures {D: (shape, [mm])}, flashes [(x, y, D)], draws [(x0, y0, x1, y1, D, centre)], regions [[(x, y)]]) in
    KiCad's coordinates (Y down: the gerber's Y negated), for KiCad's RS-274X X2 output (%FSLAX46Y46*%, %MOMM*%). Only
    dark objects are kept: --subtract-soldermask plots the mask openings onto the silk layers in clear polarity (%LPC*%).
    An arc keeps its centre (from I and J); a full circle starts and ends at the same point."""
    txt = open(path, encoding="utf-8", errors="replace").read()
    m = re.search(r"%FSLAX(\d)(\d)Y\d\d\*%", txt)
    scale = 10 ** int(m.group(2)) if m else 10 ** 6
    aps = {}
    for a in re.finditer(r"%ADD(\d+)([A-Za-z_]+),?([^*]*)\*%", txt):
        nums = []
        for v in re.split(r"[X,]", a.group(3)):
            try:
                nums.append(float(v))
            except ValueError:
                pass
        aps[int(a.group(1))] = (a.group(2), nums)
    x = y = 0.0
    ap, inreg, cur, dark = None, False, [], True
    fl, dr, rg = [], [], []
    for line in re.split(r"[\n\r]+", txt):
        line = line.strip()
        if line.startswith("%LPC"):
            dark = False
            continue
        if line.startswith("%LPD"):
            dark = True
            continue
        if line.startswith("G36"):
            inreg, cur = True, []
            continue
        if line.startswith("G37"):
            if len(cur) > 2 and dark:
                rg.append(cur)
            inreg, cur = False, []
            continue
        d = re.match(r"^(?:G\d+)?D(\d+)\*$", line)
        if d and int(d.group(1)) >= 10:
            ap = int(d.group(1))
            continue
        c = re.match(r"^(?:G0?[123])?(?:X(-?\d+))?(?:Y(-?\d+))?(?:I(-?\d+))?(?:J(-?\d+))?D0?([123])\*$", line)
        if not c:
            continue
        px, py = x, y
        if c.group(1) is not None:
            x = int(c.group(1)) / scale
        if c.group(2) is not None:
            y = -int(c.group(2)) / scale
        op = c.group(5)
        arc = None
        if c.group(3) is not None or c.group(4) is not None:
            arc = (px + int(c.group(3) or 0) / scale, py - int(c.group(4) or 0) / scale)
        if inreg:
            if op == "2":
                if len(cur) > 2 and dark:
                    rg.append(cur)
                cur = [(x, y)]
            else:
                cur.append((x, y))
        elif op == "1" and dark:
            dr.append((px, py, x, y, ap, arc))
        elif op == "3" and dark:
            fl.append((x, y, ap))
    return aps, fl, dr, rg


def stroke_dist(px, py, d, aps):
    """the distance from a point to a dark draw's inked edge (negative inside it); a full-circle arc is a ring"""
    x0, y0, x1, y1, a, arc = d
    w = (aps.get(a, ("C", [0.1]))[1] or [0.1])[0]
    if arc and math.hypot(x1 - x0, y1 - y0) < 1e-6:
        r = math.hypot(x0 - arc[0], y0 - arc[1])
        return abs(math.hypot(px - arc[0], py - arc[1]) - r) - w / 2
    return seg_dist(px, py, x0, y0, x1, y1) - w / 2


def excellon(path):
    """[(x, y, diameter)] in KiCad's coordinates (Y negated), slots by their start point."""
    tools, cur, out = {}, None, []
    for line in open(path, encoding="utf-8"):
        line = line.strip()
        t = re.match(r"^T(\d+)C([\d.]+)", line)
        if t:
            tools[int(t.group(1))] = float(t.group(2))
            continue
        t = re.match(r"^T(\d+)$", line)
        if t:
            cur = int(t.group(1))
            continue
        c = re.match(r"^X(-?[\d.]+)Y(-?[\d.]+)(?:G85X(-?[\d.]+)Y(-?[\d.]+))?", line)
        if c and cur in tools:
            x, y = float(c.group(1)), -float(c.group(2))
            if c.group(3):                      # a routed slot: its middle is the pad's centre
                x, y = (x + float(c.group(3))) / 2, (y - float(c.group(4))) / 2
            out.append((x, y, tools[cur]))
    return tools, out


def seg_dist(px, py, x0, y0, x1, y1):
    dx, dy = x1 - x0, y1 - y0
    L = dx * dx + dy * dy
    u = 0 if L == 0 else max(0.0, min(1.0, ((px - x0) * dx + (py - y0) * dy) / L))
    return math.hypot(x0 + u * dx - px, y0 + u * dy - py)


def inside(pt, poly):
    x, y = pt
    n, hit = len(poly), False
    for i in range(n):
        x0, y0 = poly[i]
        x1, y1 = poly[(i + 1) % n]
        if (y0 > y) != (y1 > y) and x < x0 + (y - y0) * (x1 - x0) / (y1 - y0):
            hit = not hit
    return hit


def quiet(fn, *a):
    import contextlib
    import io
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = fn(*a)
    out = buf.getvalue().strip().splitlines()
    return rc, out


SEED_FAB = "paste"           # a paste flash in memory at SW7's centre: (b) must fail


def fab(seed=None):
    pcb, b = dn.board()
    mm = pcb.ToMM
    row = Rows()
    f = dn.fab_module()
    files = f.gerbers()
    n_lines, n_rows, skipped = f.bom_cpl()
    g = os.path.join(f.OUT, "gerbers")
    print(f"fab.py's own export into {f.OUT}: {len(files)} files; BOM {n_lines} lines, CPL {n_rows} rows")
    # (a) Excellon tools
    pth_t, pth = excellon(os.path.join(g, "battler-PTH.drl"))
    npth_t, npth = excellon(os.path.join(g, "battler-NPTH.drl"))
    small = sorted(d for d in list(pth_t.values()) + list(npth_t.values()) if d < MIN_HOLE - 1e-6)
    row(not small, f"(a) every Excellon tool >= {MIN_HOLE} mm: PTH {sorted(pth_t.values())}, NPTH {sorted(npth_t.values())}"
        + (f"; under: {small}" if small else ""))
    # (b) the contacts in the gerbers
    fps = {fp.GetReference(): fp for fp in b.GetFootprints()}
    contacts = [r for r in fps if re.fullmatch(r"SW(\d+)", r) and (int(r[2:]) <= 16 or 20 <= int(r[2:]) <= 23)]
    paste = gerber(os.path.join(g, "battler-F_Paste.gtp"))
    silk = gerber(os.path.join(g, "battler-F_Silkscreen.gto"))
    mask = gerber(os.path.join(g, "battler-F_Mask.gts"))
    if seed == "paste":
        c = fps["SW7"].GetPosition()
        paste[1].append((mm(c.x), mm(c.y), next(iter(paste[0]))))
    bad_p, bad_s, bad_m, mask_d = [], [], [], {}
    for r in sorted(contacts, key=lambda s: int(s[2:])):
        cx, cy = mm(fps[r].GetPosition().x), mm(fps[r].GetPosition().y)
        R = COMB_D / 2
        if any(math.hypot(x - cx, y - cy) < R for x, y, _ in paste[1]) or \
                any(any(math.hypot(x - cx, y - cy) < R for x, y in poly) for poly in paste[3]):
            bad_p.append(r)
        if any(stroke_dist(cx, cy, d, silk[0]) < R for d in silk[2]):
            bad_s.append(r)
        # the mask opening: a filled circle (a full-circle arc whose stroke reaches its centre) or a circular flash at the
        # centre, or a region round it
        best = 0.0
        halves = 0
        for d in mask[2]:
            x0, y0, x1, y1, a, arc = d
            if arc and math.hypot(arc[0] - cx, arc[1] - cy) < 0.05:     # KiCad plots a filled circle as two half arcs
                w = (mask[0].get(a, ("C", [0.1]))[1] or [0.1])[0]
                rad = math.hypot(x0 - arc[0], y0 - arc[1])
                if rad <= w / 2 + 1e-6:
                    halves += 1 if math.hypot(x1 - x0, y1 - y0) > 1e-6 else 2
                    if halves >= 2:
                        best = max(best, 2 * rad + w)
        for x, y, a in mask[1]:
            shp, prm = mask[0].get(a, ("", []))
            if shp == "C" and prm and math.hypot(x - cx, y - cy) < 0.05:
                best = max(best, prm[0])
        for poly in mask[3]:
            if inside((cx, cy), poly):
                rr = min(seg_dist(cx, cy, *poly[i], *poly[(i + 1) % len(poly)]) for i in range(len(poly)))
                best = max(best, 2 * rr)
        mask_d[r] = best
        if best < COMB_D - 0.01 and int(r[2:]) <= 16:
            bad_m.append(f"{r} {best:.2f}")
    row(not bad_p, f"(b) no F.Paste inside any of the {len(contacts)} contacts' {COMB_D} mm circles" + (f": {bad_p}" if bad_p else ""))
    row(not bad_s, f"(b) no F.Silkscreen inside them" + (f": {bad_s}" if bad_s else ""))
    row(not bad_m, f"(b) the 16 combs' F.Mask openings >= {COMB_D} mm: smallest {min(mask_d[r] for r in mask_d if int(r[2:]) <= 16):.2f} mm"
        + (f"; under: {bad_m}" if bad_m else "")
        + f"; the four domes' (SW20-SW23, 4 mm domes, VER-178's) {', '.join(f'{mask_d[r]:.2f}' for r in ('SW20', 'SW21', 'SW22', 'SW23'))} mm")
    # (b) keep-out: no via, pour or foreign track within 2.9 mm of a comb's centre on F.Cu (pcbnew)
    keep = []
    for r in sorted((s for s in contacts if int(s[2:]) <= 16), key=lambda s: int(s[2:])):
        c = fps[r].GetPosition()
        own = {p.GetNetname() for p in fps[r].Pads()}
        for t in b.GetTracks():
            via = t.GetClass() == "PCB_VIA"
            w = mm(t.GetWidth(pcb.F_Cu) if via else t.GetWidth())
            d = mm(int(seg_dist(c.x, c.y, t.GetStart().x, t.GetStart().y, t.GetEnd().x, t.GetEnd().y))) - w / 2
            if d < KEEP_R:
                if via:
                    keep.append(f"{r}: a via on {t.GetNetname()} at {d + w / 2:.2f} mm")
                elif t.IsOnLayer(pcb.F_Cu) and t.GetNetname() not in own:
                    keep.append(f"{r}: a {t.GetNetname()} track at {d:.2f} mm")
        for z in b.Zones():
            if not z.IsOnLayer(pcb.F_Cu) or z.GetIsRuleArea():
                continue
            fill = z.GetFilledPolysList(pcb.F_Cu)
            if fill.OutlineCount() and fill.Collide(pcb.VECTOR2I(c.x, c.y), pcb.FromMM(KEEP_R)):
                lo, hi = 0, pcb.FromMM(KEEP_R)       # the pour's nearest edge, by bisection on pcbnew's test
                for _ in range(24):
                    mid = (lo + hi) // 2
                    if fill.Collide(pcb.VECTOR2I(c.x, c.y), mid):
                        hi = mid
                    else:
                        lo = mid + 1
                keep.append(f"{r}: the {z.GetNetname()} pour on F.Cu at {mm(hi):.3f} mm")
    row(not keep, f"(b) no via, pour or other track within {KEEP_R} mm of the 16 combs' centres on F.Cu (vias on any layer)"
        + (f": {'; '.join(keep[:8])}{' ...' if len(keep) > 8 else ''}" if keep else ""))
    # (c) copper under every CPL centroid
    import csv as _csv
    cpl = list(_csv.DictReader(open(os.path.join(f.OUT, "battler_cpl_jlc.csv"), encoding="utf-8")))
    off = []
    for rr in cpl:
        fp = fps[rr["Designator"]]
        x, y = float(rr["Mid X"][:-2]), -float(rr["Mid Y"][:-2])
        xs = [mm(p.GetPosition().x) for p in fp.Pads()]
        ys = [mm(p.GetPosition().y) for p in fp.Pads()]
        box = b2 = None
        bb = [(mm(p.GetBoundingBox().GetLeft()), mm(p.GetBoundingBox().GetTop()), mm(p.GetBoundingBox().GetRight()),
               mm(p.GetBoundingBox().GetBottom())) for p in fp.Pads()]
        if not (min(q[0] for q in bb) <= x <= max(q[2] for q in bb) and min(q[1] for q in bb) <= y <= max(q[3] for q in bb)):
            off.append(rr["Designator"])
    row(not off, f"(c) every one of the {len(cpl)} CPL centroids inside its part's pads (on its side's copper)" + (f": {off}" if off else ""))
    # (d) counts against design.py
    parts, _ = dn.design()
    want_rows = sorted(r for r, p in parts.items() if p["bom"] and p["lcsc"])
    want_lines = len({p["lcsc"] for p in parts.values() if p["bom"] and p["lcsc"]})
    row(n_rows == len(want_rows) and n_lines == want_lines,
        f"(d) BOM {n_lines} lines and CPL {n_rows} rows against design.py's {want_lines} part numbers on {len(want_rows)} parts")
    # (e) copper to the routed edges, (g) silk on the board: design_numbers.py edge (VER-18) on the same board
    rc, out = quiet(dn.edge, b)
    row(rc == 0, f"(e, g) copper 0.30 mm from every routed edge but J1's pads, every silk text on the board: design_numbers.py "
        f"edge says {out[-1] if out else '?'}")
    # (f) nothing on B.Paste
    bp = gerber(os.path.join(g, "battler-B_Paste.gbp"))
    row(not (bp[1] or bp[2] or bp[3]), f"(f) B.Paste empty: {len(bp[1])} flashes, {len(bp[2])} draws, {len(bp[3])} regions")
    # (g) the files and the outline
    need = [f"battler-{x}" for x in LAYER_FILES] + ["battler-PTH.drl", "battler-NPTH.drl", "battler-job.gbrjob"]
    miss = [x for x in need if x not in files]
    row(not miss, f"(g) all 9 layers, both drill files and the job file: " + (f"missing {miss}" if miss else "present"))
    poly = pcb.SHAPE_POLY_SET()
    closed = b.GetBoardPolygonOutlines(poly, False)
    holes = poly.HoleCount(0) if poly.OutlineCount() else 0
    row(closed and poly.OutlineCount() == 1 and holes == 2,
        f"(g) Edge.Cuts closed: {poly.OutlineCount()} outline with {holes} holes (the cell's and the speaker's)")
    # (h) one origin for all exports; every hole on a pad or via centre
    pcbtext = open(os.path.join(dn.BOARD, "battler.kicad_pcb"), encoding="utf-8").read()
    aux = re.search(r"\(aux_axis_origin ([-\d.]+) ([-\d.]+)\)", pcbtext)
    row(not aux or (float(aux.group(1)) == 0 and float(aux.group(2)) == 0),
        f"(h) no auxiliary origin in the board (gerbers use the drill-file origin, the drill files the absolute one): "
        + (f"aux_axis_origin {aux.group(1)}, {aux.group(2)}" if aux else "none set, so both are (0, 0)"))
    centres = [(mm(p.GetPosition().x), mm(p.GetPosition().y)) for fp in b.GetFootprints() for p in fp.Pads()]
    centres += [(mm(t.GetPosition().x), mm(t.GetPosition().y)) for t in b.GetTracks() if t.GetClass() == "PCB_VIA"]
    grid = {}
    for x, y in centres:
        grid.setdefault((round(x, 1), round(y, 1)), []).append((x, y))

    def near(x, y):
        for dx in (-0.1, 0, 0.1):
            for dy in (-0.1, 0, 0.1):
                for qx, qy in grid.get((round(x + dx, 1), round(y + dy, 1)), []):
                    if math.hypot(qx - x, qy - y) <= 0.01:
                        return True
        return False
    stray = [(x, y, d) for x, y, d in pth + npth if not near(x, y)]
    slots = sum(1 for ln in open(os.path.join(g, "battler-PTH.drl"), encoding="utf-8") if "G85" in ln or ln.startswith("G00"))
    row(not stray, f"(h) every one of {len(pth) + len(npth)} Excellon holes on a pad or via centre within 0.01 mm"
        + (f": {len(stray)} not, e.g. {stray[:3]}" if stray else ""))
    # (i) the drill files, (j) the CPL: VER-131's and VER-176's checks on the same exports
    rc, out = quiet(dn.drill)
    row(rc == 0, f"(i) the drill files as VER-131: design_numbers.py drill says {out[-1] if out else '?'}")
    rc, out = quiet(dn.cpl)
    row(rc == 0, f"(j) the CPL as VER-176: design_numbers.py cpl says {out[-1] if out else '?'}")
    # (k) JLC's order-number marker
    texts = [t.GetText() for t in b.GetDrawings() if t.GetClass() in ("PCB_TEXT", "PCB_TEXTBOX") and t.GetLayer() == pcb.B_SilkS]
    texts += [t.GetText() for fp in b.GetFootprints() for t in fp.GraphicalItems()
              if t.GetClass() in ("PCB_TEXT", "PCB_FIELD") and t.GetLayer() == pcb.B_SilkS]
    row(any(MARKER in t for t in texts), f"(k) '{MARKER}' on B.Silkscreen (REQ-FAB-05): " +
        ("present" if any(MARKER in t for t in texts) else f"absent ({len(texts)} back-silk texts: DEF-55); or the release "
                                                          "record names JLC's paid removal option"))
    import hashlib
    for name in ("battler_gerbers.zip", "battler_bom_jlc.csv", "battler_cpl_jlc.csv"):
        p = os.path.join(f.OUT, name)
        print(f"  sha256 {hashlib.sha256(open(p, 'rb').read()).hexdigest()}  {name} (into the release record; the zip's "
              "dates differ run to run)")
    print(f"{len(row.bad)} failing")
    return dn.result(not row.bad)


# ---- VER-17: the board against JLC's dated 2-layer capability page --------------------------------------------------
JLC_PAGE = os.path.join(VV, "analysis", "jlc_capabilities.md")   # the page copied with its date: 'name: value mm' lines


def jlc(seed=None):
    pcb, b = dn.board()
    mm = pcb.ToMM
    tracks = [t for t in b.GetTracks() if t.GetClass() == "PCB_TRACK"]
    vias = [t for t in b.GetTracks() if t.GetClass() == "PCB_VIA"]
    pth = [p for fp in b.GetFootprints() for p in fp.Pads() if p.GetAttribute() == pcb.PAD_ATTRIB_PTH]
    have = {
        "track width": min(mm(t.GetWidth()) for t in tracks),
        "via drill": min(mm(v.GetDrillValue()) for v in vias),
        "via annular ring": min((mm(v.GetWidth(pcb.F_Cu)) - mm(v.GetDrillValue())) / 2 for v in vias),
        "PTH annular ring": min((min(mm(p.GetSize(pcb.F_Cu).x), mm(p.GetSize(pcb.F_Cu).y)) - mm(p.GetDrillSize().x)) / 2
                                for p in pth if p.GetDrillSize().x == p.GetDrillSize().y),
    }
    narrow = sorted({round(mm(t.GetWidth()), 3) for t in tracks})[:3]
    print("the board's minima (pcbnew): " + ", ".join(f"{k} {v:.3f} mm" for k, v in have.items())
          + f"; the narrowest track widths {narrow}; spacing is VER-04's DRC against battler.kicad_pro's rules")
    if not os.path.exists(JLC_PAGE):
        print("BLOCKED: JLC's 2-layer capability page (https://jlcpcb.com/capabilities/pcb-capabilities) is not saved with its "
              f"date in {os.path.relpath(JLC_PAGE, dn.ROOT)}; the numbers to compare above wait for it\nRESULT: BLOCKED")
        return 2
    rules = {}
    for ln in open(JLC_PAGE, encoding="utf-8"):
        m = re.match(r"^-?\s*([a-zA-Z ]+):\s*([\d.]+)\s*mm", ln.strip())
        if m:
            rules[m.group(1).strip().lower()] = float(m.group(2))
    row = Rows()
    for k, v in have.items():
        if k in rules:
            row(v >= rules[k] - 1e-6, f"{k}: {v:.3f} mm against JLC's {rules[k]} mm")
        else:
            row(False, f"{k}: not on the saved page")
    return dn.result(not row.bad)


CHECKS = {"bridges": (bridges, SEED_BRIDGES), "fab": (fab, SEED_FAB), "jlc": (jlc, None)}


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if "--list" in sys.argv or not args:
        print(__doc__)
        return 0
    fn, seed = CHECKS[args[0]]
    if "--seed" in sys.argv:
        if not seed:
            sys.exit(f"layout_checks: {args[0]} has no seed")
        print(f"SEEDED: {seed}")
        return fn(seed)
    return fn()


if __name__ == "__main__":
    sys.exit(main())
