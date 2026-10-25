#!/usr/bin/env python3
"""The pre-order numbers, recomputed from the design files so nobody quotes them by hand (DEF-28).

    python3 hardware/vv/tools/design_numbers.py <check>      # exit 1 when the check's limit fails
    python3 hardware/vv/tools/design_numbers.py --list

Each check reads design.py, placement.py, case.scad, docs/bom.md, board/tools/fab.py or the
routed board (battler.kicad_pcb, through KiCad's pcbnew module), prints its numbers and ends
with RESULT: PASS or FAIL against the pass limit its verification.csv row states. Datasheet
numbers that design.py does not carry are named constants below, each with its source; a
part change in design.py makes the evidence stale through the row's part: covers.

  inrush     VER-99   VBUS capacitance and what charges VSYS at a plug-in
  backlight  VER-137  worst-case LED current and the halves' match at the minimum duty
  cutoff     VER-138  where the 3.50 V cutoff fires after the gain calibration pinmap.md describes
  screws     VER-147  M2 screw engagement in the case's pilots
  mlcc       VER-142  MLCCs against the case's screw bosses (pcbnew)
  th1        VER-164  TH1 against the cell's hole; bare copper facing the cell
  cpl        VER-176  CPL position (footprint origin) against the pads' centre (pcbnew)
  drill      VER-131  plated and unplated holes in separate drill files
  vbus       VER-177  the lowest VBUS that over-stresses a part (a faulty source)
  bom        VER-167  every contact class has its bought parts in docs/bom.md
  holes      VER-114  every drilled hole 0.30 mm or larger (pcbnew)
  swell      VER-20   free height above a worst-case cell, under the deck and the panel (part of VER-20)

Python 3 standard library, plus pcbnew for mlcc and cpl.
"""
import json
import math
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = subprocess.run(["git", "-C", HERE, "rev-parse", "--show-toplevel"], capture_output=True,
                      text=True).stdout.strip()
REF = os.path.join(ROOT, "hardware", "ref_design")
BOARD = os.path.join(REF, "board")

# Datasheet numbers design.py does not hold. Each is checked again in VER-34's source audit.
VREG_TOL = 0.02          # RT9013 output accuracy +-2% (DS9013-10 p4)
CV_TOL = 0.01            # BQ24073 VBAT(REG) 4.2 V +-1% (SLUS810N; design.py and pinmap.md quote +-1%)
I_3V3_AT_CUTOFF = 0.15   # A: REQ-PWR-15's 3V3 budget, the load the cutoff is read under
OPAMP_VOS = 0.0045       # V: MCP6002 input offset, maximum (pin_review/u6.json)
R_TOL = 0.01             # every resistor in design.py's RES table is 1%
PANEL_MAX_MA = 80.0      # ER-TFT024IPS-3 backlight, 4 LEDs in parallel, 80 mA maximum (pin_review/panel.json)
LED_MAX_MA = 20.0        # one LED: the drawing's 4 x 20 mA (pin_review/panel.json)
MIN_DUTY = 0.29          # the firmware's lowest backlight duty (docs/pinmap.md, Backlight)
EN1_INTERNAL = 285e3     # BQ24073 EN1 internal pull-down (pin_review/u2.json pin 6)
D3_VF = 0.3              # BAT54WS at a few hundred uA
ABS = {"U2 IN": 28.0, "U2 EN1": 7.0, "U3 EN": 6.0}    # absolute maxima (u2.json, u3.json)
PD_MAX = 20.0            # V: the highest fixed USB PD voltage, what a broken source may put out


def design():
    ns = {"__name__": "design_numbers"}
    exec(compile(open(os.path.join(BOARD, "design.py")).read(), "design.py", "exec"), ns)
    return {p["ref"]: p for p in ns["PARTS"]}, ns


def ohms(v):
    v = v.split()[0]
    m = re.fullmatch(r"([\d.]+)([kM]?)", v)
    return float(m.group(1)) * {"": 1, "k": 1e3, "M": 1e6}[m.group(2)]


def farads(v):
    m = re.match(r"([\d.]+)([pnu])", v)
    return float(m.group(1)) * {"p": 1e-12, "n": 1e-9, "u": 1e-6}[m.group(2)]


def scad():
    """Top-level 'name = expression;' assignments of case.scad, evaluated in order where Python can."""
    env = {"min": min, "max": max, "abs": abs}
    for line in open(os.path.join(REF, "case", "case.scad")):
        for stmt in line.split("//")[0].split(";")[:-1]:
            m = re.fullmatch(r"\s*([a-z_][a-z0-9_]*)\s*=\s*(.+?)\s*", stmt)
            if not m:
                continue
            try:
                env[m.group(1)] = eval(m.group(2), {"__builtins__": {}}, env)
            except Exception:
                pass
    return env


def placement_put(ref):
    src = open(os.path.join(BOARD, "placement.py")).read()
    m = re.search(r'put\("' + ref + r'",\s*([\d.]+),\s*([\d.]+)(?:,\s*(-?\d+))?', src)
    return (float(m.group(1)), float(m.group(2)), int(m.group(3) or 0)) if m else None


def cutouts():
    src = open(os.path.join(BOARD, "placement.py")).read()
    m = re.search(r'name="cell",\s*box=\(([\d.]+),\s*([\d.]+),\s*([\d.]+),\s*([\d.]+)\)', src)
    return tuple(float(x) for x in m.groups())


def result(ok):
    print("RESULT: " + ("PASS" if ok else "FAIL"))
    return 0 if ok else 1


# ---- the checks -------------------------------------------------------------------
def inrush():
    parts, _ = design()
    caps = {r: p for r, p in parts.items() if p["lib"] == "Device:C" and p["bom"]}
    vbus_nets = {"VBUS", "VBUS_C1"}
    vbus = sum(farads(p["value"]) for p in caps.values() if set(p["pins"].values()) & vbus_nets)
    bridges = sorted(r for r, p in parts.items() if not r.startswith("#") and p["bom"]
                     and set(p["pins"].values()) & vbus_nets and "VSYS" in p["pins"].values())
    print(f"VBUS capacitance {vbus * 1e6:.1f} uF (USB 2.0 cl. 7.2.4.1: 10 uF): {vbus * 5e6:.0f} uC at 5.0 V, "
          f"{vbus * 5.25e6:.1f} uC at 5.25 V (nominal values; DC bias lowers them)")
    print(f"parts joining VBUS to VSYS: {', '.join(bridges) or 'none'} (want U2 alone: its input switch "
          "brings VSYS up inside the input limit, so the VSYS bulk is not seen at the plug)")
    print(f"U2 is {parts['U2']['value']}")
    return result(vbus <= 10e-6 * 1.001 and bridges == ["U2"] and parts["U2"]["value"].startswith("BQ2407"))


def backlight():
    parts, _ = design()
    r60, r61 = ohms(parts["R60"]["value"]), ohms(parts["R61"]["value"])
    rs = min(ohms(parts["R52"]["value"]), ohms(parts["R53"]["value"]))
    v = 3.3
    nom = v * r61 / (r60 + r61) / rs
    ref_hi = v * (1 + VREG_TOL) * r61 * (1 + R_TOL) / (r60 * (1 - R_TOL) + r61 * (1 + R_TOL))
    sink_hi = (ref_hi + OPAMP_VOS) / (rs * (1 - R_TOL))
    total = 2 * sink_hi * 1e3
    print(f"R60 {r60:g}, R61 {r61:g}, sense {rs:g} ohm: nominal {nom * 1e3:.1f} mA a sink, {2 * nom * 1e3:.1f} mA in all")
    print(f"worst case at 100% duty (3V3 +{VREG_TOL:.0%}, R61 +1%, R60 -1%, offset +{OPAMP_VOS * 1e3:.1f} mV, "
          f"sense -1%): {sink_hi * 1e3:.1f} mA a sink, {total:.1f} mA in all (panel maximum {PANEL_MAX_MA:g} mA)")
    print(f"two LEDs share each sink: {sink_hi * 1e3:.1f} mA against {2 * LED_MAX_MA:g} mA for the pair "
          "(one LED's share is not controlled: VER-62 measures it)")
    ref_min = v * r61 / (r60 + r61) * MIN_DUTY
    dev = OPAMP_VOS / ref_min
    print(f"at the minimum duty {MIN_DUTY:.0%}: BL_REF {ref_min * 1e3:.1f} mV, offset +-{dev:.1%} a sink "
          f"(limit +-25%, REQ-DSP-12)")
    return result(total <= PANEL_MAX_MA and dev <= 0.25)


def cutoff():
    parts, _ = design()
    u3 = json.load(open(os.path.join(BOARD, "pin_review", "u3.json")))
    drop = next(r for r in u3["ratings"] if r["what"] == "Dropout")["limit"]
    m = re.search(r"([\d.]+) mV max at ([\d.]+) mA", drop)
    vd_max = float(m.group(1)) / 1e3 * I_3V3_AT_CUTOFF / (float(m.group(2)) / 1e3)
    m = re.search(r"([\d.]+) mV typ", drop)
    vd_typ = float(m.group(1)) / 1e3 * I_3V3_AT_CUTOFF / 0.5
    print(f"U3 {parts['U3']['value']}: dropout {drop}; at {I_3V3_AT_CUTOFF * 1e3:.0f} mA about {vd_typ * 1e3:.0f} mV "
          f"typical, {vd_max * 1e3:.0f} mV maximum (scaled with the current)")
    print(f"R17/R18 {parts['R17']['value']}/{parts['R18']['value']}: the ADC reads VBAT/2 against 3V3 itself")
    print("calibration (pinmap.md): the gain is set at charge-done, taking the cell as 4.20 V; "
          f"the charger's CV is 4.20 V +-{CV_TOL:.0%}")
    ok = True
    # in regulation the reading is VBAT x 4.2 / CV: the cutoff fires at a true 3.50 x CV / 4.2
    lo, hi = 3.50 * (1 - CV_TOL), 3.50 * (1 + CV_TOL)
    print(f"3V3 in regulation: the cutoff fires at a true {lo:.3f}-{hi:.3f} V (REQ-BAT-12: 3.50 +0.05/-0.00)")
    ok &= lo >= 3.50 - 1e-9
    # in dropout 3V3 = VBAT - Vd and the reading rises as the cell falls: its lowest is at the knee
    for label, vreg, vd, cv in (("nominal", 3.3, vd_typ, 1.0), ("worst", 3.3 * (1 + VREG_TOL), vd_max, 1 - CV_TOL)):
        knee = (vreg + vd) * 4.2 / (4.2 * cv)
        fires = knee <= 3.50
        print(f"{label}: 3V3 {vreg:.3f} V, dropout {vd * 1e3:.0f} mV, CV {4.2 * cv:.3f} V: lowest reading in dropout "
              f"{knee:.3f} V: {'fires' if fires else 'never reaches 3.50: the cutoff never fires'}")
        ok &= fires
    return result(ok)


def screws():
    e = scad()
    need = ["cell", "cell_top", "pcb_t", "cell_pocket", "floor_t", "screw_cb", "screw_l", "glass_gap", "panel_t",
            "tape_t", "face_t", "dome_h", "flange_t", "lip_t"]
    miss = [n for n in need if n not in e]
    if miss:
        print("case.scad: could not read " + " ".join(miss))
        return result(False)
    z_board = e["floor_t"] + e["cell"][2] - e["cell_top"] - e["pcb_t"] - e["cell_pocket"]
    z_top = z_board + e["pcb_t"]
    case_h = z_top + e["glass_gap"] + e["panel_t"] + e["tape_t"] + e["face_t"]
    tip = e["screw_cb"][1] + e["screw_l"]
    engage = tip - z_top
    island = min(6.0, case_h - z_top - 0.8)
    deck = e["dome_h"] + e["flange_t"] + e["lip_t"] - 0.6
    print(f"board top at z {z_top:.2f}; M2 x {e['screw_l']:g} from a {e['screw_cb'][1]:g} mm counterbore: tip at "
          f"z {tip:.2f}")
    print(f"thread into the island's pilots {engage:.2f} mm of {island:.2f}; into the decks' {engage:.2f} of {deck:.2f}")
    print(f"engagement {engage / 2:.1f} x d (REQ-MEC-09: 4.0 mm, 2 x d); the island's pilots would take "
          f"M2 x {math.floor(island - 0.5 + z_top - e['screw_cb'][1]):d}, the decks' at most {deck:.2f} mm of thread")
    return result(engage >= 4.0 and tip <= z_top + min(island, deck) - 0.0)


def board():
    import pcbnew  # noqa: F401  (KiCad's module)
    return pcbnew, pcbnew.LoadBoard(os.path.join(BOARD, "battler.kicad_pcb"))


def mlcc():
    pcbnew, b = board()
    mm = pcbnew.ToMM
    e = scad()
    boss_r = max(e.get("tub_boss_d", [6.0])) / 2 if isinstance(e.get("tub_boss_d"), list) else 3.0
    fps = {f.GetReference(): f for f in b.GetFootprints()}
    holes = {r: (mm(f.GetPosition().x), mm(f.GetPosition().y)) for r, f in fps.items() if re.fullmatch(r"H\d", r)}
    rows = []
    for r, f in fps.items():
        if not r.startswith("C") or not (f.HasField("LCSC") and f.GetFieldText("LCSC")):
            continue
        size = str(f.GetFPID().GetLibItemName())
        bulk = not ("0402" in size)
        for h, (hx, hy) in holes.items():
            d = min(math.hypot(mm(p.GetPosition().x) - hx, mm(p.GetPosition().y) - hy) for p in f.Pads())
            rows.append((d, r, h, f.GetValue(), size, bulk))
    rows.sort()
    bad = [x for x in rows if x[0] < boss_r + (3.0 if x[5] else 1.0)]
    print(f"{len(holes)} screw holes, boss radius {boss_r:g} mm (the tub's largest); rule: an MLCC pad centre "
          f"{boss_r + 1:g} mm or more from a hole centre, a 0603 or larger {boss_r + 3:g} mm or more")
    for d, r, h, val, size, bulk in rows[:6]:
        print(f"  {r} {val} ({size}) {d:.2f} mm from {h}")
    print("too near: " + (", ".join(f"{r} {d:.2f} mm from {h}" for d, r, h, *_ in bad) or "none"))
    return result(not bad)


def th1():
    x0, y0, x1, y1 = cutouts()
    e = scad()
    tx, ty, rot = placement_put("TH1")
    gap = (x1 - x0 - e["cell"][0]) / 2, (y1 - y0 - e["cell"][1]) / 2
    to_edge = tx - x1
    print(f"cell hole x {x0}-{x1}, y {y0}-{y1}; worst-case cell {e['cell'][0]} x {e['cell'][1]}; "
          f"TH1 at ({tx}, {ty}), {to_edge:.2f} mm right of the hole's right edge (the case pushes the cell there)")
    in_span = y0 + gap[1] + 2 <= ty <= y1 - gap[1] - 2
    print(f"TH1 within the cell's span along the edge: {in_span}; air between cell and board edge 0 to "
          f"{x1 - x0 - 48:.1f} mm (a nominal-minus-2 cell) unless pushed")
    src = open(os.path.join(BOARD, "placement.py")).read()
    back = [(r, float(x), float(y)) for r, x, y in re.findall(r'put\("(\w+)",\s*([\d.]+),\s*([\d.]+)[^)]*"B"\)', src)]
    facing = [r for r, x, y in back if x0 - 1 <= x <= x1 + 1 and y0 - 1 <= y <= y1 + 1]
    print(f"back-side pads within 1 mm of the hole: {', '.join(facing) or 'none'}")
    return result(0 <= to_edge <= 1.0 and in_span and not facing)


def cpl():
    pcbnew, b = board()
    mm = pcbnew.ToMM
    off = []
    for f in sorted(b.GetFootprints(), key=lambda f: f.GetReference()):
        if not (f.HasField("LCSC") and f.GetFieldText("LCSC")):
            continue
        xs = [mm(p.GetPosition().x) for p in f.Pads()]
        ys = [mm(p.GetPosition().y) for p in f.Pads()]
        d = math.hypot((min(xs) + max(xs)) / 2 - mm(f.GetPosition().x), (min(ys) + max(ys)) / 2 - mm(f.GetPosition().y))
        if d > 0.10:
            off.append((f.GetReference(), d))
    fab = open(os.path.join(BOARD, "tools", "fab.py")).read()
    corrected = "centroid" in fab.lower() or "CPL_OFFSET" in fab
    print("fab.py writes each footprint's origin as Mid X / Mid Y" + (" with a correction table" if corrected else ""))
    print("origin more than 0.10 mm off its pads' centre: " + (", ".join(f"{r} {d:.2f} mm" for r, d in off) or "none"))
    return result(not off or corrected)


def drill():
    fab = open(os.path.join(BOARD, "tools", "fab.py")).read()
    sep = "--excellon-separate-th" in fab
    print(f"fab.py asks kicad-cli for separate plated and unplated drill files: {sep}")
    try:
        pcbnew, b = board()
        npth = {}
        for f in b.GetFootprints():
            for p in f.Pads():
                if p.GetAttribute() == pcbnew.PAD_ATTRIB_NPTH:
                    npth.setdefault(round(pcbnew.ToMM(p.GetDrillSize().x), 2), set()).add(f.GetReference())
        print("unplated holes on the board: " + "; ".join(f"{d} mm: {' '.join(sorted(r))}" for d, r in sorted(npth.items())))
    except ImportError:
        print("pcbnew not available: hole list skipped")
    return result(sep)


def vbus():
    parts, _ = design()
    r43 = ohms(parts["R43"]["value"])
    r14, r15 = ohms(parts["R14"]["value"]), ohms(parts["R15"]["value"])
    en1 = ABS["U2 EN1"] * (r43 + EN1_INTERNAL) / EN1_INTERNAL
    en = ABS["U3 EN"] * (r14 + r15) / r15 + D3_VF
    c1 = parts["C1"]["mpn"]
    samsung_v = {"Q": 6.3, "P": 10.0, "O": 16.0, "A": 25.0, "L": 35.0, "B": 50.0}   # CLxx code, 10th letter
    limits = {"U3 EN through R14/D3 (PWR_EN)": en, "U2 EN1 through R43": en1, "U2 IN": ABS["U2 IN"],
              f"C1 ({c1})": samsung_v.get(c1[9], 0.0) if c1.startswith("CL") else 0.0}
    for k, v in sorted(limits.items(), key=lambda kv: kv[1]):
        print(f"  {k}: over its rating above VBUS {v:.1f} V")
    low = min(limits.values())
    print(f"lowest VBUS that over-stresses a part: {low:.1f} V (U2's OVP at 6.6 V protects VSYS, not these); "
          f"limit {PD_MAX:g} V, a broken USB PD source")
    return result(low >= PD_MAX)


def bom():
    parts, _ = design()
    text = open(os.path.join(REF, "docs", "bom.md")).read()
    want = {"battler:DSL_button_4ring": ["DS Lite silicone", "cross and A/B/X/Y"],
            "battler:Dome_4mm_Round": ["600-415S-260", "Start/Select cap", "Start/Select strip"]}
    classes = sorted({p["fp"] for r, p in parts.items() if r.startswith("SW") and not p["bom"] and "DSL" in p["fp"]
                      or r.startswith("SW") and "Dome" in p["fp"]})
    ok = True
    for c in classes:
        refs = sorted((r for r, p in parts.items() if p["fp"] == c), key=lambda r: int(r[2:]))
        miss = [t for t in want.get(c, ["(no rule for this class)"]) if t.lower() not in text.lower()]
        print(f"{c} ({refs[0]}-{refs[-1]}): " + ("bought" if not miss else "missing from bom.md: " + "; ".join(miss)))
        ok &= not miss
    if re.search(r"Start/Select strip isn't used", text):
        print("bom.md says the Start/Select strip isn't used, but SW20-SW23 sit under it")
        ok = False
    return result(ok)


def holes():
    pcbnew, b = board()
    mm = pcbnew.ToMM
    sizes = {}
    for t in b.GetTracks():
        if t.GetClass() == "PCB_VIA":
            sizes.setdefault(round(mm(t.GetDrillValue()), 3), set()).add("via")
    for f in b.GetFootprints():
        for p in f.Pads():
            d = p.GetDrillSize()
            if d.x > 0:
                sizes.setdefault(round(mm(min(d.x, d.y)), 3), set()).add(f.GetReference())
    for d, who in sorted(sizes.items()):
        print(f"  {d:.2f} mm: {' '.join(sorted(who)[:10])}{' ...' if len(who) > 10 else ''}")
    low = min(sizes)
    print(f"smallest hole {low:.2f} mm (limit 0.30 mm, JLC's standard)")
    return result(low >= 0.30 - 1e-6)


def swell():
    e = scad()
    cell_t, top = e["cell"][2], e["cell_top"]
    deck_under = e["dome_h"] + e["flange_t"] + e["lip_t"] - e["top_t"]
    panel = e["glass_gap"]
    print(f"worst-case cell {cell_t} mm, its top {top} mm over the board; the deck's underside {deck_under:.2f} mm, "
          f"the panel's back {panel:.2f} mm over the board")
    free = min(deck_under, panel) - top
    print(f"free height above the cell: {deck_under - top:.2f} mm under the deck, {panel - top:.2f} mm under the panel; "
          f"swelling of 10% of {e['cell_nom'][2]} mm: {0.1 * e['cell_nom'][2]:.2f} mm")
    return result(free >= 0.1 * e["cell_nom"][2] - 1e-9)


CHECKS = {"holes": holes, "swell": swell, "inrush": inrush, "backlight": backlight, "cutoff": cutoff, "screws": screws, "mlcc": mlcc, "th1": th1,
          "cpl": cpl, "drill": drill, "vbus": vbus, "bom": bom}


def main():
    if len(sys.argv) != 2 or sys.argv[1] not in CHECKS:
        print(__doc__.split("\n\n")[2] if "--list" in sys.argv else __doc__)
        return 0 if "--list" in sys.argv else 2
    return CHECKS[sys.argv[1]]()


if __name__ == "__main__":
    sys.exit(main())
