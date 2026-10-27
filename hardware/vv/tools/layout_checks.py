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


CHECKS = {"bridges": (bridges, SEED_BRIDGES)}


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
