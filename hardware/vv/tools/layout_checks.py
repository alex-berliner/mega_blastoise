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
    if any(x in ("NC:U4.28", "NC:U4.29", "NC:U4.30") for x in nc):
        return "self-test", "DF-185", "a module pin wired to the octal PSRAM inside (IO35-IO37): a bridge stops the PSRAM (unit 1's first article)"
    if s == {"VSYS", "PWR_BTN_RAW"}:
        return "dangerous", "DF-188", "SW24's own pads shorted: the power button held for ever, the device cannot turn off (the cell drains to its PCM)"
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
        return "dangerous", "DF-51", "VBUS on a CC pin: the module's IO2/IO4 over their limit through 10k (VER-195)"
    if s == {"+3V3", "GND"}:
        return "dangerous", "DF-162", "3V3 shorted (a 74HC165's CLK INH to VCC, or the module's pins 1-2): U3 in current limit"
    if s == {"EN", "GND"} or s == {"EN", "+3V3"}:
        return "self-test", "DF-185", "the module's EN held: it never starts, or J7's reset does nothing (bring-up)"
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
    keep, pour_min = [], {}
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
                # 39cc07f: route.py's keep-out is a 24-gon 2.9 mm across its flats, so a pour can touch 2.900 mm: the rule is
                # 2.9 mm or more, met to the micrometre (pcbnew's Collide counts a touch)
                if mm(hi) < KEEP_R - 0.001:
                    keep.append(f"{r}: the {z.GetNetname()} pour on F.Cu at {mm(hi):.3f} mm")
                else:
                    pour_min[r] = min(pour_min.get(r, 9.0), mm(hi))
    if pour_min:
        print(f"  the GND pour's nearest edge to a comb's centre: {min(pour_min.values()):.4f} mm (the rule area's flats, "
              f"{KEEP_R} mm), on {len(pour_min)} combs")
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
        # 39cc07f: no round plated hole is left on the board (J1's are slots), so the minimum may be over nothing
        "PTH annular ring": min([(min(mm(p.GetSize(pcb.F_Cu).x), mm(p.GetSize(pcb.F_Cu).y)) - mm(p.GetDrillSize().x)) / 2
                                 for p in pth if p.GetDrillSize().x == p.GetDrillSize().y] or [float("inf")]),
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


# ---- VER-21: return paths under the fast nets ------------------------------------------------------------------------
# 39cc07f: the panel's 8080 bus (WR and D0-D7, 25 MHz) and the microSD's 4-bit bus replace the SPI nets
FAST = ["LCD_WR", "LCD_WR_MCU"] + [f"LCD_D{k}{s}" for k in range(8) for s in ("", "_MCU")] + \
       ["SD_CLK", "SD_CMD", "SD_D0", "SD_D1", "SD_D2", "SD_D3", "USB_DP", "USB_DM", "USB_DP_MCU", "USB_DM_MCU", "AUDIO_PWM",
        "SR_CLK", "SR_CLK_MCU", "SR_PL", "SR_DATA"]
GAP_MM, STEP_MM = 2.0, 0.1
SEED_RET = "nofill"          # the opposite layer's GND fill dropped in memory: every sample is over a gap


def ret(seed=None):
    pcb, b = dn.board()
    mm = pcb.ToMM
    row = Rows()
    fills = {}
    for z in b.Zones():
        if z.GetIsRuleArea() or z.GetNetname() != "GND":
            continue
        for lay in (pcb.F_Cu, pcb.B_Cu):
            if z.IsOnLayer(lay):
                fills.setdefault(lay, []).append(z.GetFilledPolysList(lay))
    if seed == "nofill":
        fills = {}
    print(f"GND fills: F.Cu {len(fills.get(pcb.F_Cu, []))} zone(s), B.Cu {len(fills.get(pcb.B_Cu, []))}; each fast net's tracks "
          f"sampled every {STEP_MM} mm, a sample over no GND fill on the other layer is a gap; limit {GAP_MM} mm in a run")
    worst = {}
    for t in b.GetTracks():
        n = t.GetNetname().split("/")[-1]
        if n not in FAST or t.GetClass() != "PCB_TRACK":
            continue
        other = pcb.B_Cu if t.GetLayer() == pcb.F_Cu else pcb.F_Cu
        x0, y0, x1, y1 = mm(t.GetStart().x), mm(t.GetStart().y), mm(t.GetEnd().x), mm(t.GetEnd().y)
        L = math.hypot(x1 - x0, y1 - y0)
        k = max(1, int(L / STEP_MM))
        run = best = 0.0
        for i in range(k + 1):
            px, py = x0 + (x1 - x0) * i / k, y0 + (y1 - y0) * i / k
            pt = pcb.VECTOR2I(pcb.FromMM(px), pcb.FromMM(py))
            covered = any(f.Contains(pt) for f in fills.get(other, []))
            run = 0.0 if covered else run + (L / k if i else 0.0)
            best = max(best, run)
        if best > worst.get(n, (0, None))[0]:
            worst[n] = (best, (round(x0, 2), round(y0, 2)), "F.Cu" if t.GetLayer() == pcb.F_Cu else "B.Cu")
    for n in FAST:
        g, at, lay = worst.get(n, (0.0, None, ""))
        row(g <= GAP_MM, f"{n}: longest run over no GND on the other layer {g:.2f} mm" + (f" (a {lay} track from {at})" if g else ""))
    print(f"{len(row.bad)} failing")
    return dn.result(not row.bad)


# ---- VER-108: USB D+/D- stubs and pair length ------------------------------------------------------------------------
STUB_MM, SKEW_MM = 3.5, 2.0


def net_lengths(pcb, b, nets):
    """{(ref, pad): {(ref, pad): track length}} by Dijkstra over the nets' tracks (vias join layers, a pad joins every
    track end inside it)."""
    import heapq
    mm = pcb.ToMM
    key = lambda x, y, l: (round(x, 3), round(y, 3), l)   # noqa: E731
    adj = {}

    def link(a, c, w):
        adj.setdefault(a, []).append((c, w))
        adj.setdefault(c, []).append((a, w))
    ends = []
    for t in b.GetTracks():
        if t.GetNetname().split("/")[-1] not in nets:
            continue
        if t.GetClass() == "PCB_VIA":
            x, y = mm(t.GetPosition().x), mm(t.GetPosition().y)
            link(key(x, y, "F"), key(x, y, "B"), 0.0)
            ends += [(key(x, y, "F"), t), (key(x, y, "B"), t)]
        else:
            lay = "F" if t.GetLayer() == pcb.F_Cu else "B"
            a = key(mm(t.GetStart().x), mm(t.GetStart().y), lay)
            c = key(mm(t.GetEnd().x), mm(t.GetEnd().y), lay)
            link(a, c, mm(t.GetLength()))
            ends += [(a, t), (c, t)]
    nodes = list(adj)
    for i, n in enumerate(nodes):                      # ends that touch (a T or a width change)
        for m in nodes[i + 1:]:
            if n[2] == m[2] and math.hypot(n[0] - m[0], n[1] - m[1]) < 0.02:
                link(n, m, 0.0)
    pads = {}
    for fp in b.GetFootprints():
        for p in fp.Pads():
            if p.GetNetname().split("/")[-1] in nets:
                pk = (fp.GetReference(), p.GetNumber())
                pads[pk] = p
                for n in nodes:
                    lay = pcb.F_Cu if n[2] == "F" else pcb.B_Cu
                    if p.IsOnLayer(lay) and p.HitTest(pcb.VECTOR2I(pcb.FromMM(n[0]), pcb.FromMM(n[1]))):
                        link(("pad",) + pk, n, 0.0)
    out = {}
    for pk in pads:
        src = ("pad",) + pk
        dist = {src: 0.0}
        h, k = [(0.0, 0, src)], 0
        while h:
            d, _, u = heapq.heappop(h)
            if d > dist.get(u, 1e18):
                continue
            for v, w in adj.get(u, []):
                if d + w < dist.get(v, 1e18):
                    dist[v] = d + w
                    k += 1
                    heapq.heappush(h, (d + w, k, v))
        out[pk] = {q: dist[("pad",) + q] for q in pads if ("pad",) + q in dist}
    return out


SEED_USB = 5.0               # mm added in memory to one J1 branch: the stub must fail


def usbpair(seed=None):
    pcb, b = dn.board()
    row = Rows()
    parts, _ = dn.design()
    total = {}
    for line, rser in (("DP", "R21"), ("DM", "R22")):
        d = net_lengths(pcb, b, {f"USB_{line}"})
        j1 = sorted(pk for pk in d if pk[0] == "J1")
        rp = next(pk for pk in d if pk[0] == rser)
        u1 = [pk for pk in d if pk[0] == "U1"]
        a, c = j1
        da, dc, dac = d[a][rp], d[c][rp], d[a][c]
        sa, sc = (da + dac - dc) / 2, (dc + dac - da) / 2       # each branch from its pad to the junction
        if seed:                                                 # lengthen the shorter branch in memory
            sa, sc = (sa + seed, sc) if sa < sc else (sa, sc + seed)
        stub = min(sa, sc)
        row(stub <= STUB_MM, f"USB_{line}: J1 {a[1]} and {c[1]} join {sa:.2f} and {sc:.2f} mm from their pads; the stub (the shorter "
            f"branch) {stub:.2f} mm against {STUB_MM} mm (pin_review/j1.json)")
        if u1:
            du = min(d[u1[0]][rp], min(d[u1[0]][q] for q in j1))
            print(f"  USB_{line}: U1 pin {u1[0][1]} is {du:.2f} mm from the line's nearest point it was measured to")
        dm = net_lengths(pcb, b, {f"USB_{line}_MCU"})
        u4 = next(pk for pk in dm if pk[0] == "U4")
        r2 = next(pk for pk in dm if pk[0] == rser)
        total[line] = max(da, dc) + dm[r2][u4]
        print(f"  USB_{line}: J1 to {rser} {max(da, dc):.2f} mm (the longer branch), {rser} to U4 {dm[r2][u4]:.2f} mm")
    skew = abs(total["DP"] - total["DM"])
    row(skew <= SKEW_MM, f"D+ {total['DP']:.2f} mm, D- {total['DM']:.2f} mm: mismatch {skew:.2f} mm against {SKEW_MM} mm")
    return dn.result(not row.bad)


# ---- VER-205 (39cc07f): the flex antenna's keep-out on the board and its clearance to metal ---------------------------
ANT_CLEAR = 15.0             # mm: 'A clearance of at least 15 mm is recommended in all directions' (esp32-s3-hw-design-guidelines.txt:1299),
                             # written for the module's own PCB antenna: a stand-in until the flex antenna's maker's figure is on disk
ANT_DOC = os.path.join(dn.REF, "docs", "esp32s3_build.md")
SEED_ANT = "pour"            # a GND pour sample put inside the keep-out in memory: the copper count must fail


def antenna(seed=None):
    pcb, b = dn.board()
    mm = pcb.ToMM
    row = Rows()
    doc = open(ANT_DOC, encoding="utf-8").read()
    m = re.search(r"x ([\d.]+) \.\. ([\d.]+),\s*y ([\d.]+) \.\. ([\d.]+), on a (\d+) mm U\.FL cable", doc)
    if not m:
        print("docs/esp32s3_build.md names no antenna plate 'x .. , y .. , on a .. mm U.FL cable'\nRESULT: FAIL")
        return 1
    ax0, ax1, ay0, ay1 = (float(m.group(i)) for i in range(1, 5))
    print(f"the flex antenna's plate (docs/esp32s3_build.md): x {ax0}-{ax1}, y {ay0}-{ay1} (KiCad), on a {m.group(5)} mm U.FL cable; "
          "it lies on the case's left wall, fitted by hand (JLC does not fit cables)")
    zones = [z for z in b.Zones() if z.GetIsRuleArea() and z.GetZoneName() == "antenna_keepout"]
    row(bool(zones), f"a rule area 'antenna_keepout' on the board: {len(zones)}")
    if not zones:
        return dn.result(False)
    z = zones[0]
    bb = z.GetBoundingBox()
    kx0, ky0, kx1, ky1 = mm(bb.GetLeft()), mm(bb.GetTop()), mm(bb.GetRight()), mm(bb.GetBottom())
    both = z.IsOnLayer(pcb.F_Cu) and z.IsOnLayer(pcb.B_Cu)
    rules = z.GetDoNotAllowTracks() and z.GetDoNotAllowVias() and z.GetDoNotAllowZoneFills() and z.GetDoNotAllowFootprints()
    row(both and rules, f"the keep-out x {kx0:.2f}-{kx1:.2f}, y {ky0:.2f}-{ky1:.2f} on both copper layers ({both}), forbidding tracks, "
        f"vias, pours and footprints ({rules})")
    print(f"  it covers the plate's x {ax0}-{ax1} out to {kx1 - ax1:+.2f} mm; along y it stops {ky0 - ay0:.2f} mm short of the plate's "
          f"top end and {ay1 - ky1:.2f} mm short of its bottom (placement.py: none below, the module's top row keeps its lane)")
    inside = []

    def hit(x, y):
        return kx0 < x < kx1 and ky0 < y < ky1
    for t in b.GetTracks():
        for p in (t.GetStart(), t.GetEnd()):
            if hit(mm(p.x), mm(p.y)):
                inside.append(f"{t.GetClass()} {t.GetNetname()} at ({mm(p.x):.2f}, {mm(p.y):.2f})")
    for f in b.GetFootprints():
        for p in f.Pads():
            if hit(mm(p.GetPosition().x), mm(p.GetPosition().y)):
                inside.append(f"pad {f.GetReference()}.{p.GetNumber()}")
    for zz in b.Zones():
        if zz.GetIsRuleArea() or not zz.IsFilled():
            continue
        for lay in (pcb.F_Cu, pcb.B_Cu):
            if not zz.IsOnLayer(lay):
                continue
            poly = zz.GetFilledPolysList(lay)
            for i in range(poly.OutlineCount()):
                o = poly.Outline(i)
                for k in range(o.PointCount()):
                    q = o.CPoint(k)
                    if hit(mm(q.x), mm(q.y)):
                        inside.append(f"{zz.GetNetname()} pour on {b.GetLayerName(lay)}")
                        break
    if seed == "pour":
        inside.append("seeded: a GND pour vertex at the keep-out's centre")
    row(not inside, f"copper inside the keep-out: {len(inside)}" + (f" ({'; '.join(sorted(set(inside))[:6])})" if inside else ""))
    # the plate against the metal near it: the module's shield, the cell's pouch, the panel, the slot's shell, the screws
    fps = {f.GetReference(): f for f in b.GetFootprints()}

    def gap_rect(x0, y0, x1, y1):
        dx = max(x0 - ax1, ax0 - x1, 0.0)
        dy = max(y0 - ay1, ay0 - y1, 0.0)
        return math.hypot(dx, dy), (x0 < ax1 and x1 > ax0 and y0 < ay1 and y1 > ay0)
    metal = []
    for ref in ("U4", "J8", "J3"):
        f = fps[ref]
        fb = f.GetBoundingBox(False)
        metal.append((ref, mm(fb.GetLeft()), mm(fb.GetTop()), mm(fb.GetRight()), mm(fb.GetBottom())))
    cx0, cy0, cx1, cy1 = dn.cutouts()
    metal.append(("the cell (its pocket, 0.9 mm inside the hole)", cx0 + 0.9, cy0 + 0.9, cx1 - 0.9, cy1 - 0.9))
    pos_glass = None
    try:
        import case_checks as cs
        with cs.ref_copy() as (_, pos, _l):
            pos_glass = pos["glass"]
    except Exception as ex:                       # noqa: BLE001
        print(f"  (the panel's outline from positions.json not read: {ex})")
    if pos_glass:
        (gx0, gy0), (gx1, gy1) = pos_glass
        metal.append(("the panel (its glass outline)", min(gx0, gx1), min(gy0, gy1), max(gx0, gx1), max(gy0, gy1)))
    for i in range(1, 7):
        h = fps.get(f"H{i}")
        if h:
            hx, hy = mm(h.GetPosition().x), mm(h.GetPosition().y)
            metal.append((f"H{i}'s M2 screw (its 5 mm boss)", hx - 2.5, hy - 2.5, hx + 2.5, hy + 2.5))
    print(f"  the plate to metal near it, against {ANT_CLEAR:g} mm (esp32-s3-hw-design-guidelines.txt:1299, the module's PCB antenna's "
          "figure as a stand-in: the flex antenna's maker's keep-out is not on disk, Kinghelm C4943394, https://www.lcsc.com/"
          "product-detail/C4943394.html):")
    near = []
    for name, x0, y0, x1, y1 in metal:
        d, over = gap_rect(x0, y0, x1, y1)
        if d < 30:
            near.append((d, name, over))
    for d, name, over in sorted(near):
        print(f"    {name}: {'OVERLAPS the plate' if over else f'{d:.1f} mm'}")
    close = [n for d, n, o in near if d < ANT_CLEAR and not n.startswith("U4")]
    row(not close, f"metal within {ANT_CLEAR:g} mm of the plate, the module's own shield aside (its U.FL feeds the cable): "
        + (", ".join(close) if close else "none"))
    print("  the antenna's own datasheet (gain, keep-out, ground plane, cable loss) is not in hardware/ref_design/datasheets: the "
          "range test VER-207 on unit 1 decides; REQ-RF-04's grant names the antenna types the module may use")
    return dn.result(not row.bad)


CHECKS = {"bridges": (bridges, SEED_BRIDGES), "fab": (fab, SEED_FAB), "jlc": (jlc, None), "ret": (ret, SEED_RET),
          "usbpair": (usbpair, SEED_USB), "antenna": (antenna, SEED_ANT)}


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
