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
  screws     VER-147  M2 screw engagement in the case's pilots, the side ones and the end ones apart;
                      bom.md's screw length against case.scad's
  mlcc       VER-142  MLCCs against the case's screw bosses (pcbnew)
  th1        VER-164  TH1 against the cell's hole; bare copper facing the cell (pcbnew)
  cellgap    VER-181  a worst-case cell to the routed FR4 edge on every side, after tolerances
  cpl        VER-176  the CPL fab.py writes against each footprint's pads' centre (pcbnew)
  drill      VER-131  the drill files fab.py writes: plated and unplated apart, every hole placed
  vbus       VER-177  the lowest VBUS that over-stresses a part (a faulty source)
  bom        VER-167  every contact class has its bought parts in docs/bom.md
  holes      VER-114  every drilled hole 0.30 mm or larger (pcbnew)
  swell      VER-182  free height above a worst-case cell, under whatever lies over its pocket
  edge       VER-18   copper to every routed edge, part courtyards inside the outline (pcbnew)
  plugs      VER-184  J2 (the cell) and J4 (the speaker) keyed apart, and how far apart they sit
  tempwin    VER-32   the charge window TH1 gives against U2's TS thresholds, from SLUS810N

Helpers (they print numbers a bench procedure uses and always exit 0):
  tsband [C] the TS voltage band VER-133, VER-135 and VER-68 accept at the measured ambient
  cvload     the load that holds U2 in CV without terminating (VER-134, VER-145): there is no
             resistive window, so it checks the TL431 shunt load at its corners (O-R2)

cpl and drill import board/tools/fab.py, point its OUT at a temp dir and run its own export
(kicad-cli, a second or two), so they read the files that would be uploaded, never fab.py's
text: a comment in fab.py cannot change their result (O-R1, seeded in VER-129). Nothing is
written under hardware/ref_design.

Python 3 standard library, plus pcbnew (KiCad's module) and kicad-cli for the board checks.
"""
import csv
import importlib.util
import json
import math
import os
import re
import subprocess
import sys
import tempfile

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
PANEL_MAX_MA = 120.0     # ER-TFT035IPS-6 backlight, 6 LEDs in parallel, 120 mA (pin_review/panel.json: 'If 120 mA',
                         # read as the limit since the table prints '110 120' without columns)
LED_MAX_MA = 20.0        # one LED: 120 mA over the six (pin_review/panel.json)
LEDS_PER_PIN = 3         # the drawing's LED-K1 and LED-K2 carry three LEDs each (pin_review/panel.json pins 2, 3)
# The charger's pre-charge, termination and TS bias, from SLUS810N (the BQ24073's datasheet, on disk as
# bq24074.txt): VER-34 quotes the lines and tools/datasheet_hashes.py records the file's sha256, size
# and date.  Pre-charge has a factor of its own in A-ohm (KPRECHG), not a share of ISET, so the
# pre-charge current is KPRECHG / R5 and does not track ICHG (O-R2).  Termination is a share of the
# unit's own ICHG (ITERM), so it does.
KPRECHG = (70.0, 106.0)      # A-ohm: SLUS810N's KPRECHG, 70 / 88 / 106 (bq24074.txt:787)
KISET = (797.0, 975.0)       # A-ohm: SLUS810N's KISET, 797 / 890 / 975, so ICHG = KISET / R5 (bq24074.txt:785)
ITERM = (0.09, 0.11)         # share of ICHG: SLUS810N's ITERM with (EN1, EN2) not (LO, LO) (bq24074.txt:789)
V_LOWV = (2.9, 3.1)          # V: the pre-charge to fast-charge threshold, its band (bq24074.txt:776)
VBAT_REG = (4.16, 4.23)      # V: SLUS810N's VBAT(REG) for the '73 (bq24074.txt:773)
V_HOT = (0.270, 0.330)       # V: SLUS810N's TS hot threshold, VTS falling (bq24074.txt:824)
V_COLD = (2.0, 2.2)          # V: SLUS810N's TS cold threshold, VTS rising (bq24074.txt:826)
I_TS, I_TS_TOL = 75e-6, 0.04 # A: TS bias current 72-78 uA (SLUS810N's INTC, bq24074.txt:823): 75 uA +-3 uA
TH1_B, TH1_R25, TH1_TOL = 3950.0, 10e3, 0.01   # design.py TH1: 10k B3950 1% (SDNT1005X103F3950FTF)
TH1_B_TOL = 0.01             # TH1's B tolerance (the part number's second F: 1%)
# The per-unit CV screen's load (VER-134, VER-145): a resistor alone cannot do it (cvload), so the
# load is a resistor in parallel with a shunt branch that conducts only above a set voltage: a TL431
# (cathode-to-anode set to 3.5 V by 10k over 24.9k, 1%) in series with 22 ohm.  A low-voltage zener
# is too soft for this branch (about 90 ohm of dynamic resistance at a few mA), and three diodes
# conduct at about 2 V, under V_LOWV.
CV_LOAD_R = 160.0            # ohm, 1%: the load's resistor
CV_SHUNT_V = (3.43, 3.57)    # V: the TL431 branch's set point, 3.5 V +-2% (Vref 2.495 V x (1 + 10k / 24.9k): a plain
                             # TL431's reference and the divider's 1% resistors)
CV_SHUNT_R = 22.0            # ohm, 1%: in series with the TL431
# RP2350 GPIO input leakage, maximum: the datasheet's pad table, 'Pin Input Leakage Current IIN 1 uA'
# (rp2350.txt:101199, quoted by VER-34)
I_LEAK = 1e-6
# Fab and print tolerances for the cell's gap (VER-181): JLC's standard outline tolerance (+-0.2 mm,
# to confirm on the dated capability page, VER-17); an FDM wall's position (+-0.1 mm, an estimate
# until VER-174's XY chain); the board's float on its screws (M2 in 2.2 mm holes: +-0.1 mm)
ROUTE_TOL, PRINT_TOL, SCREW_FLOAT = 0.2, 0.1, 0.1
JLC_MIN_PLATED_SLOT = 0.50   # mm, JLC's minimum plated slot width (to confirm on the dated page, VER-17)
# A CPL row more than 0.10 mm off its pads passes only if JLC's correction for that ref is recorded
# here, with the dry run's screenshot named (VER-28).  An entry counts only when its evidence file
# exists, is committed and names the designator in an ARTEFACT line (O-R7, F-R4); otherwise the row
# fails as before.  None today.
JLC_FIXED = {
    # "J1": {"evidence": "hardware/vv/evidence/2026-10-27_VER-28_jlc_preview.txt",
    #         "offset": "JLC's library puts J1's origin 1.50 mm behind its pads' centre"},
}
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


def backlight_section():
    text = open(os.path.join(REF, "docs", "pinmap.md")).read()
    sect = re.search(r"^## Backlight.*?(?=^## )", text, re.M | re.S)
    return sect.group(0) if sect else ""


def min_duty():
    """The firmware's lowest backlight duty, from docs/pinmap.md's Backlight section (O-R21)."""
    m = re.search(r"lowest level at about (\d+)%", backlight_section())
    if not m:
        sys.exit("design_numbers: pinmap.md's Backlight section names no 'lowest level at about N%'")
    return int(m.group(1)) / 100


def max_duty():
    """The firmware's highest backlight duty: pinmap.md's 'highest level N%' (or 'cap the duty at N%'), else 100%."""
    m = re.search(r"(?:highest level|cap the duty) at (?:about )?(\d+)%", backlight_section())
    return (int(m.group(1)) / 100, "pinmap.md") if m else (1.0, "pinmap.md names no lower maximum")


def fab_module():
    """board/tools/fab.py as a module whose OUT is a fresh temp dir."""
    spec = importlib.util.spec_from_file_location("fab_for_vv", os.path.join(BOARD, "tools", "fab.py"))
    fab = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(fab)
    fab.OUT = tempfile.mkdtemp(prefix="vv_fab_")
    return fab


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
    dmax, src = max_duty()
    sink_hi = (ref_hi * dmax + OPAMP_VOS) / (rs * (1 - R_TOL))
    total = 2 * sink_hi * 1e3
    print(f"R60 {r60:g}, R61 {r61:g}, sense {rs:g} ohm: nominal {nom * 1e3:.1f} mA a sink, {2 * nom * 1e3:.1f} mA in all "
          "at 100% duty")
    print(f"the firmware's highest duty: {dmax:.0%} ({src})")
    print(f"worst case at {dmax:.0%} duty (3V3 +{VREG_TOL:.0%}, R61 +1%, R60 -1%, offset +{OPAMP_VOS * 1e3:.1f} mV, "
          f"sense -1%): {sink_hi * 1e3:.1f} mA a sink, {total:.1f} mA in all (panel maximum {PANEL_MAX_MA:g} mA)")
    print(f"{LEDS_PER_PIN} LEDs share each sink: {sink_hi * 1e3 / LEDS_PER_PIN:.1f} mA an LED on average against "
          f"{LED_MAX_MA:g} mA (one LED's share is not controlled: VER-43 measures each cathode pin on the breakout)")
    # what would pass: the highest duty, or the largest R61 at 100%
    d_ok = (PANEL_MAX_MA / 2e3 * rs * (1 - R_TOL) - OPAMP_VOS) / ref_hi
    print(f"to stay within {PANEL_MAX_MA:g} mA: cap the duty at {math.floor(d_ok * 1000) / 10:.1f}% or less (firmware), "
          "or lower R61 (BOM):")
    for alt in (3.6e3, 3.65e3, 3.74e3, 3.83e3):
        rh = v * (1 + VREG_TOL) * alt * (1 + R_TOL) / (r60 * (1 - R_TOL) + alt * (1 + R_TOL))
        t = 2 * (rh + OPAMP_VOS) / (rs * (1 - R_TOL)) * 1e3
        print(f"  R61 {alt / 1e3:g}k: {t:.1f} mA worst case at 100%, {v * alt / (r60 + alt) / rs * 2e3:.1f} mA nominal")
    duty = min_duty()
    ref_min = v * r61 / (r60 + r61) * duty
    dev = OPAMP_VOS / ref_min
    print(f"at the minimum duty {duty:.0%} (pinmap.md): BL_REF {ref_min * 1e3:.1f} mV, offset +-{dev:.1%} a sink "
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
    # O-R15: GPIO47's pad leakage through the divider's Thevenin source; it changes with temperature,
    # so a one-point gain calibration does not remove it
    r17, r18 = ohms(parts["R17"]["value"]), ohms(parts["R18"]["value"])
    rth = r17 * r18 / (r17 + r18)
    gain = (r17 + r18) / r18                             # VBAT per volt at the pin
    band = I_LEAK * rth * gain                           # the pin's error, scaled back to VBAT
    fit = 0.05 / 2 / (rth * gain)
    print(f"pad leakage: +-{I_LEAK * 1e6:g} uA (the RP2350's IIN maximum, rp2350.txt:101199, quoted by VER-34) "
          f"through {rth / 1e3:.0f} kohm moves VBAT's reading +-{band:.3f} V; REQ-BAT-12's 50 mV window allows "
          f"+-{fit * 1e9:.0f} nA of leakage that changes with temperature, before any other error")
    ok &= 2 * band <= 0.05
    return result(ok)


def screws():
    """REQ-MEC-09, split per pilot (F's objection): the four side screws (H1-H4, under the screen band) want
    4.0 mm of thread (2 x d) and their tip 0.5 mm short of the pilot's end; the two end screws (H5, H6, under
    the button ends, whose pilots stop 0.6 mm under the face) want 3.0 mm and the tip 0.1 mm short."""
    e = scad()
    need = ["z_top_cu", "screw_cb", "screw_l", "side_engage", "deck_engage"]
    miss = [n for n in need if n not in e]
    if miss:
        print("case.scad: could not read " + " ".join(miss))
        return result(False)
    z_top, cb = e["z_top_cu"], e["screw_cb"][1]
    # the screws bought are bom.md's: its Screws line must name every length case.scad uses (the audit, 2026-10-27)
    used = {float(e.get("screw_l_side", e["screw_l"])), float(e.get("screw_l_end", e["screw_l"]))}
    line = next((ln for ln in open(os.path.join(REF, "docs", "bom.md")) if re.match(r"\|\s*Screws\b", ln)), "")
    named = {float(x) for x in re.findall(r"M2 x (\d+(?:\.\d+)?)", line)}
    ok = bool(named) and used <= named
    print(f"bom.md's Screws line names M2 x {', '.join(f'{n:g}' for n in sorted(named)) or 'nothing'}; case.scad uses "
          f"M2 x {', '.join(f'{n:g}' for n in sorted(used))}: {'ok' if ok else 'fails'}")
    for label, length, pilot, want, short in (
            ("side (H1-H4)", e.get("screw_l_side", e["screw_l"]), e["side_engage"], 4.0, 0.5),
            ("end (H5, H6)", e.get("screw_l_end", e["screw_l"]), e["deck_engage"], 3.0, 0.1)):
        tip = cb + length
        thread = tip - z_top
        spare = z_top + pilot - tip
        lo, hi = want + z_top - cb, pilot - short + z_top - cb
        fits = [n for n in range(4, 13) if lo - 1e-9 <= n <= hi + 1e-9]
        good = thread >= want - 1e-9 and spare >= short - 1e-9
        ok &= good
        print(f"{label}: M2 x {length:g} from a {cb:g} mm counterbore, tip at z {tip:.2f}, board top at z {z_top:.2f}: "
              f"{thread:.2f} mm of thread in a {pilot:.2f} mm pilot, tip {spare:.2f} mm short of its end "
              f"(want {want:g} mm and {short:g} mm): {'ok' if good else 'fails'}; lengths that meet it: "
              + (", ".join(f"M2 x {n}" for n in fits) or "none"))
    return result(ok)


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


def th1(b=None):
    """O-R14: TH1 and every flipped footprint's pads from the routed board, not placement.py's text.
    b: a pcbnew board already loaded (seed_faults.py moves a pad in memory, writing no KiCad file)."""
    pcbnew, b = board() if b is None else (__import__("pcbnew"), b)
    mm = pcbnew.ToMM
    x0, y0, x1, y1 = cutouts()
    e = scad()
    fps = {f.GetReference(): f for f in b.GetFootprints()}
    tx, ty = mm(fps["TH1"].GetPosition().x), mm(fps["TH1"].GetPosition().y)
    gap = (x1 - x0 - e["cell"][0]) / 2, (y1 - y0 - e["cell"][1]) / 2
    to_edge = tx - x1
    print(f"cell hole x {x0}-{x1}, y {y0}-{y1}; worst-case cell {e['cell'][0]} x {e['cell'][1]}; "
          f"TH1 at ({tx:.2f}, {ty:.2f}) on the board, {to_edge:.2f} mm right of the hole's right edge (the case pushes "
          "the cell there)")
    in_span = y0 + gap[1] + 2 <= ty <= y1 - gap[1] - 2
    print(f"TH1 within the cell's span along the edge: {in_span}")
    flipped = sorted(r for r, f in fps.items() if f.IsFlipped())
    near = []
    for r in flipped:
        for p in fps[r].Pads():
            px, py = mm(p.GetPosition().x), mm(p.GetPosition().y)
            dx, dy = max(x0 - px, 0, px - x1), max(y0 - py, 0, py - y1)
            near.append((math.hypot(dx, dy), r))
    near.sort()
    facing = sorted({r for d, r in near if d <= 1.0})
    print(f"back-side footprints ({len(flipped)}): {' '.join(flipped)}; the nearest pad {near[0][0]:.2f} mm from the "
          f"hole ({near[0][1]})" if near else "no back-side footprints")
    print(f"back-side pads within 1 mm of the hole: {', '.join(facing) or 'none'}")
    return result(0 <= to_edge <= 1.0 and in_span and not facing)


def cellgap():
    """O-R7 and round 2's objections: the gap from a worst-case cell to the routed FR4 edge on every face it can
    rest against (the tub's pocket wall on each side; assembly.md step 5 pushes it to the right wall), after the
    fab and print tolerances (REQ-BAT-08). The pocket sits cell_pocket_fit inside the hole all round, so the gap
    at the pushed end and the long sides is that fit whatever the hole's size: widening one end alone fixes
    nothing (seeded in VER-129). It also prints what would pass."""
    e = scad()
    (hx0, hy0), (hx1, hy1) = e["cell_hole"]
    fit = e["cell_pocket_fit"]
    tol = ROUTE_TOL + PRINT_TOL + SCREW_FLOAT
    hl, hw = hx1 - hx0, hy1 - hy0
    print(f"board hole {hl:.1f} x {hw:.1f} mm; the tub's pocket {fit:g} mm inside it each side "
          f"(case.scad cell_pocket_fit); tolerances: routing {ROUTE_TOL:g}, print {PRINT_TOL:g}, the board's float "
          f"on its screws {SCREW_FLOAT:g} mm ({tol:.2f} mm in all)")
    ok = True
    for label, (cl, cw, _) in (("worst-case", e["cell"]), ("nominal", e["cell_nom"])):
        if cw > hw - 2 * fit + 1e-9 or cl > hl - 2 * fit + 1e-9:
            print(f"  {label} cell {cl:g} x {cw:g} does not fit the pocket")
            ok = False
            continue
        sides = {"right end (pushed, at TH1)": fit,
                 "left end (its leads, J2), the cell pushed right": hl - fit - cl,
                 "left end (its leads, J2), the cell NOT pushed (against the pocket's left wall)": fit,
                 "each long side (at worst against the pocket's wall)": fit}
        for side, nom in sides.items():
            need = 1.5 if "left" in side else 0.5
            worst = nom - tol
            good = worst >= need - 1e-9
            ok &= good
            print(f"  {label} cell {cl:g} x {cw:g}, {side}: {nom:.2f} mm nominal, {worst:.2f} mm after tolerances "
                  f"(REQ-BAT-08: {need:g}): {'ok' if good else 'too close'}")
    f_need = 0.5 + tol
    cl, cw = e["cell"][0], e["cell"][1]
    print("the pushed cell is held there by assembly.md step 5's tape; an unpushed cell rests against the "
          "pocket's left wall, so its lead end has only the pocket's fit (O-R8: a printed stop at the pocket's "
          "left end is the case-side option)")
    l_need = 1.5 + tol
    print(f"what passes: the pocket {f_need:.2f} mm inside the hole at the right end and on both long sides, and "
          f"{l_need:.2f} mm inside it at the lead end, so the pocket itself holds every cell off the lead end (that "
          f"wall is the printed stop DEF-58 asks for, and no tape is needed); then")
    print(f"  with this hole ({hl:.1f} x {hw:.1f}): a cell of at most {hl - f_need - l_need:.2f} x "
          f"{hw - 2 * f_need:.2f} mm (its spec sheet, VER-37, and incoming inspection, VER-49)")
    print(f"  with the worst-case {cl:g} x {cw:g} cell: a hole of at least {cl + f_need + l_need:.2f} x "
          f"{cw + 2 * f_need:.2f} mm (+{cl + f_need + l_need - hl:.2f} in x, +{cw + 2 * f_need - hw:.2f} in y: copper)")
    return result(ok)


def jlc_fixed_ok(ref):
    """F-R4: a JLC_FIXED entry counts only when its evidence file exists, is committed and names the
    designator.  Returns (ok, why)."""
    e = JLC_FIXED.get(ref)
    if not e:
        return False, "no entry"
    path = e.get("evidence", "")
    if not path:
        return False, "the entry names no evidence file"
    if not os.path.isfile(os.path.join(ROOT, path)):
        return False, f"the evidence file {path} does not exist"
    tr = subprocess.run(["git", "-C", ROOT, "ls-files", "--error-unmatch", path], capture_output=True, text=True)
    if tr.returncode != 0:
        return False, f"the evidence file {path} is not committed"
    if ref not in open(os.path.join(ROOT, path), encoding="utf-8").read():
        return False, f"the evidence file {path} does not name {ref}"
    return True, e.get("note") or e.get("offset", "")


def cpl():
    """O-R1: the CPL fab.py writes, row by row, against each footprint's pads' centre."""
    pcbnew, b = board()
    mm = pcbnew.ToMM
    fab = fab_module()
    n_lines, n_parts, _ = fab.bom_cpl()
    path = os.path.join(fab.OUT, "battler_cpl_jlc.csv")
    fps = {f.GetReference(): f for f in b.GetFootprints()}
    off, fixed, rows = [], [], list(csv.DictReader(open(path)))
    for r in rows:
        f = fps.get(r["Designator"])
        if f is None:
            off.append((r["Designator"], float("inf")))
            continue
        xs = [mm(p.GetPosition().x) for p in f.Pads()]
        ys = [mm(p.GetPosition().y) for p in f.Pads()]
        cx, cy = (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2
        mx, my = float(r["Mid X"].rstrip("m")), -float(r["Mid Y"].rstrip("m"))
        d = math.hypot(mx - cx, my - cy)
        if d > 0.10:
            ok, why = jlc_fixed_ok(r["Designator"])
            if ok:
                fixed.append((r["Designator"], d, why))
            else:
                off.append((r["Designator"], d))
    print(f"fab.py's bom_cpl() wrote {n_lines} BOM lines and {n_parts} CPL rows (read from {os.path.basename(path)})")
    print("CPL row more than 0.10 mm off its pads' centre: " + (", ".join(f"{r} {d:.2f} mm" for r, d in off) or "none"))
    for r, d, why in fixed:
        print(f"  {r} {d:.2f} mm: accepted by a JLC_FIXED entry: {why}")
    for r, d in off:
        ok, why = jlc_fixed_ok(r)
        if why not in ("no entry",):
            print(f"  {r} {d:.2f} mm: a JLC_FIXED entry does not count: {why}")
    return result(not off and len(rows) == n_parts)


def excellon(path):
    """(file function, [(dia, plated, x, y, slot)]) from a KiCad Excellon file (absolute, mm)."""
    func, tools, holes, aper, cur, x, y = "?", {}, [], None, None, None, None
    for ln in open(path):
        ln = ln.strip()
        m = re.match(r"; #@! TF\.FileFunction,(\w+)", ln)
        if m:
            func = m.group(1)
        m = re.match(r"; #@! TA\.AperFunction,(\w+)", ln)
        if m:
            aper = m.group(1)
        m = re.fullmatch(r"T(\d+)C([\d.]+)", ln)
        if m:
            tools[m.group(1)] = (float(m.group(2)), aper == "Plated" if aper else None)
            continue
        m = re.fullmatch(r"T(\d+)", ln)
        if m:
            cur = m.group(1)
            continue
        m = re.fullmatch(r"(?:X(-?[\d.]+))?(?:Y(-?[\d.]+))?(G85(?:X(-?[\d.]+))?(?:Y(-?[\d.]+))?)?", ln)
        if m and cur and (m.group(1) or m.group(2)):
            x = float(m.group(1)) if m.group(1) else x
            y = float(m.group(2)) if m.group(2) else y
            dia, plated = tools[cur]
            if m.group(3):
                x2 = float(m.group(4)) if m.group(4) else x
                y2 = float(m.group(5)) if m.group(5) else y
                holes.append((dia, plated, (x + x2) / 2, -(y + y2) / 2, True))
            else:
                holes.append((dia, plated, x, -y, False))
    return func, holes


def drill():
    """O-R1: the drill files fab.py's own export writes, parsed: plated and unplated apart, and
    every hole on the board in the right one."""
    pcbnew, b = board()
    mm = pcbnew.ToMM
    fab = fab_module()
    fab.gerbers()
    g = os.path.join(fab.OUT, "gerbers")
    files = sorted(f for f in os.listdir(g) if f.endswith(".drl"))
    found = []
    ok = True
    for f in files:
        func, holes = excellon(os.path.join(g, f))
        print(f"{f}: FileFunction {func}, {len(holes)} holes, "
              f"{sum(1 for h in holes if h[4])} slots")
        if func not in ("Plated", "NonPlated"):
            print(f"  {f} is {func}: plated and unplated holes share one file")
            ok = False
        found += [(func, *h) for h in holes]
    want = []
    for t in b.GetTracks():
        if t.GetClass() == "PCB_VIA":
            want.append(("via", True, round(mm(t.GetDrillValue()), 3), mm(t.GetPosition().x), mm(t.GetPosition().y), False))
    for fp in b.GetFootprints():
        for p in fp.Pads():
            d = p.GetDrillSize()
            if d.x > 0:
                plated = p.GetAttribute() != pcbnew.PAD_ATTRIB_NPTH
                want.append((fp.GetReference(), plated, round(mm(min(d.x, d.y)), 3), mm(p.GetPosition().x),
                             mm(p.GetPosition().y), d.x != d.y))
    npth = sorted({(w[2], w[0]) for w in want if not w[1]})
    print("unplated holes on the board: " + "; ".join(
        f"{d} mm: {' '.join(sorted(r for dd, r in npth if dd == d))}" for d in sorted({d for d, _ in npth})))
    wrong, missing = [], []
    for ref, plated, dia, x, y, slot in want:
        hit = [h for h in found if abs(h[3] - x) < 0.02 and abs(h[4] - y) < 0.02 and abs(h[1] - dia) < 0.011]
        if not hit:
            missing.append(f"{ref} {dia} at ({x:.2f}, {y:.2f})")
        elif hit[0][0] != ("Plated" if plated else "NonPlated"):
            wrong.append(f"{ref} {dia} {'plated' if plated else 'unplated'} in a {hit[0][0]} file")
    slots = sorted({(round(h[1], 2), h[0]) for h in found if h[5] and h[2]})
    print(f"plated slots: {', '.join(f'{d} mm ({fn})' for d, fn in slots) or 'none'} "
          f"(JLC's minimum {JLC_MIN_PLATED_SLOT} mm)")
    narrow = [d for d, _ in slots if d < JLC_MIN_PLATED_SLOT - 1e-9]
    extra = len(found) - (len(want) - len(missing))
    print(f"board holes {len(want)}, in the drill files {len(found)}; missing {len(missing)}, in the wrong file "
          f"{len(wrong)}, unaccounted {extra}")
    for x in (missing + wrong)[:10]:
        print("  " + x)
    return result(ok and not missing and not wrong and not narrow and extra == 0)


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
    """The free height above a worst-case cell, under whatever lies over the tub's pocket (case.scad): the
    panel's back, the panel frame's walls, the ends' underside past the band, the band's underside elsewhere."""
    e = scad()
    need = ["cell", "cell_top", "cell_hole", "cell_pocket_fit", "panel_w", "panel_l", "panel_fit", "frame_wall",
            "frame_h", "glass_gap", "z_ceil", "z_top_cu", "z_deck_under", "joint_y", "joint_w"]
    miss = [n for n in need if n not in e]
    if miss:
        print("case.scad: could not read " + " ".join(miss))
        return result(False)
    cell_t, top, f = e["cell"][2], e["cell_top"], e["cell_pocket_fit"]
    (hx0, hy0), (hx1, hy1) = e["cell_hole"]
    px0, px1, py0, py1 = hx0 + f, hx1 - f, hy0 + f, hy1 - f           # the pocket: where the cell can be
    pw, pl = (e["panel_w"]) / 2, (e["panel_l"]) / 2
    fw, fl = (e["panel_w"] + e["panel_fit"]) / 2 + e["frame_wall"], (e["panel_l"] + e["panel_fit"]) / 2 + e["frame_wall"]
    ends = e["joint_y"] - e["joint_w"]

    def over(x0, x1, y0, y1):
        return x0 < px1 and x1 > px0 and y0 < py1 and y1 > py0
    regions = [("the panel's back", (-pw, pw, -pl, pl), e["glass_gap"]),
               ("the panel frame's walls", (-fw, fw, -fl, fl), e["z_ceil"] - e["frame_h"] - e["z_top_cu"]),
               ("the ends' underside (past the band)", (-99, 99, ends, 99), e["z_deck_under"] - e["z_top_cu"]),
               ("the ends' underside (seat 1's)", (-99, 99, -99, -ends), e["z_deck_under"] - e["z_top_cu"]),
               ("the band's underside", (-99, 99, -ends, ends), e["z_ceil"] - e["z_top_cu"])]
    print(f"worst-case cell {cell_t} mm, its top {top} mm over the board; its pocket x {px0:.2f} to {px1:.2f}, "
          f"y {py0:.2f} to {py1:.2f} (case frame)")
    free = None
    for name, box, h in regions:
        if over(*box):
            print(f"  over the cell: {name}, {h:.2f} mm over the board: {h - top:.2f} mm free")
            free = h - top if free is None else min(free, h - top)
        else:
            print(f"  not over the cell: {name} ({h:.2f} mm over the board)")
    want = 0.1 * e["cell_nom"][2]
    print(f"free height above the cell: {free:.2f} mm; swelling of 10% of {e['cell_nom'][2]} mm: {want:.2f} mm")
    return result(free is not None and free >= want - 1e-9)


COPPER_EDGE = 0.30   # mm: REQ-FAB-03's copper to a routed edge (JLC's figure, to confirm on the dated page: VER-17)
EDGE_ALLOWED = {"J1"}   # parts whose body stands past the outline by design: the USB-C receptacle's face on its tab


def edge():
    """VER-18: every copper feature (pours, tracks, vias, pads) at least COPPER_EDGE from every routed edge (the
    outline, the cell's hole, the speaker's hole), every courtyard inside the outline (J1 by design excepted),
    every silkscreen text wholly on the board, and the narrowest FR4 web between two routed edges, printed. The
    edges are the board polygon pcbnew builds from Edge.Cuts (arcs as chords)."""
    pcbnew, b = board()
    mm = pcbnew.ToMM
    outl = pcbnew.SHAPE_POLY_SET()
    b.GetBoardPolygonOutlines(outl, False)
    contours = []
    for i in range(outl.OutlineCount()):
        rings = [outl.Outline(i)] + [outl.Hole(i, k) for k in range(outl.HoleCount(i))]
        for k, ring in enumerate(rings):
            pts = [(mm(ring.CPoint(n).x), mm(ring.CPoint(n).y)) for n in range(ring.PointCount())]
            xs, ys = [p[0] for p in pts], [p[1] for p in pts]
            contours.append(("outline" if k == 0 else f"{max(xs) - min(xs):.1f} x {max(ys) - min(ys):.1f} mm hole", pts))
    cell = 1.0
    grid = {}
    segs = []
    for ci, (_, pts) in enumerate(contours):
        for n in range(len(pts)):
            a, c = pts[n], pts[(n + 1) % len(pts)]
            segs.append((a, c, ci))
            for gx in range(int(math.floor((min(a[0], c[0]) - 1) / cell)), int(math.floor((max(a[0], c[0]) + 1) / cell)) + 1):
                for gy in range(int(math.floor((min(a[1], c[1]) - 1) / cell)), int(math.floor((max(a[1], c[1]) + 1) / cell)) + 1):
                    grid.setdefault((gx, gy), []).append(len(segs) - 1)

    def seg_dist(p, a, c):
        dx, dy = c[0] - a[0], c[1] - a[1]
        L = dx * dx + dy * dy
        t = 0.0 if L == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / L))
        return math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy)

    def near(p, skip=None):
        """(distance, contour) to the nearest routed edge within about 1 mm, or (None, None)."""
        best = (None, None)
        for si in grid.get((int(math.floor(p[0] / cell)), int(math.floor(p[1] / cell))), ()):
            a, c, ci = segs[si]
            if ci == skip:
                continue
            d = seg_dist(p, a, c)
            if best[0] is None or d < best[0]:
                best = (d, ci)
        return best

    def densify(pts, closed=True, step=0.05):
        out = []
        rng = len(pts) if closed else len(pts) - 1
        for n in range(rng):
            a, c = pts[n], pts[(n + 1) % len(pts)]
            k = max(1, int(math.hypot(c[0] - a[0], c[1] - a[1]) / step))
            out += [(a[0] + (c[0] - a[0]) * j / k, a[1] + (c[1] - a[1]) * j / k) for j in range(k)]
        return out or list(pts)

    found = {}

    def note(what, d, p, ci):
        if d is None:
            return
        key = (what, contours[ci][0])
        if key not in found or d < found[key][0]:
            found[key] = (d, p, contours[ci][0])
    for z in b.Zones():
        if z.GetIsRuleArea():
            continue
        for layer in z.GetLayerSet().Seq():
            fp = z.GetFilledPolysList(layer)
            for i in range(fp.OutlineCount()):
                o = fp.Outline(i)
                pts = [(mm(o.CPoint(n).x), mm(o.CPoint(n).y)) for n in range(o.PointCount())]
                for p in densify(pts):
                    d, ci = near(p)
                    note(f"{z.GetNetname()} pour on {b.GetLayerName(layer)}", d, p, ci)
    for t in b.GetTracks():
        half = mm(t.GetWidth()) / 2 if t.GetClass() != "PCB_VIA" else mm(t.GetWidth(pcbnew.F_Cu)) / 2
        a, c = (mm(t.GetStart().x), mm(t.GetStart().y)), (mm(t.GetEnd().x), mm(t.GetEnd().y))
        for p in densify([a, c], closed=False) + [c]:
            d, ci = near(p)
            note(f"{'via' if t.GetClass() == 'PCB_VIA' else 'track'} {t.GetNetname()}", None if d is None else d - half, p, ci)
    courtyard_out = []
    for f in b.GetFootprints():
        ref = f.GetReference()
        for p in f.Pads():
            for layer in (pcbnew.F_Cu, pcbnew.B_Cu):
                if not p.IsOnLayer(layer):
                    continue
                poly = p.GetEffectivePolygon(layer, pcbnew.ERROR_INSIDE)
                for i in range(poly.OutlineCount()):
                    o = poly.Outline(i)
                    pts = [(mm(o.CPoint(n).x), mm(o.CPoint(n).y)) for n in range(o.PointCount())]
                    for q in densify(pts):
                        d, ci = near(q)
                        note(f"pad {ref}.{p.GetNumber()}", d, q, ci)
        if ref in EDGE_ALLOWED:
            continue
        worst = None
        for layer in (pcbnew.F_CrtYd, pcbnew.B_CrtYd):
            cy = f.GetCourtyard(layer)
            for i in range(cy.OutlineCount()):
                o = cy.Outline(i)
                pts = [(mm(o.CPoint(n).x), mm(o.CPoint(n).y)) for n in range(o.PointCount())]
                for q in densify(pts, step=0.1):
                    if not outl.Contains(pcbnew.VECTOR2I(pcbnew.FromMM(q[0]), pcbnew.FromMM(q[1]))):
                        d, ci = near(q)
                        if worst is None or (d or 9) > worst[0]:
                            worst = (d or 9.0, contours[ci][0] if ci is not None else "outline")
        if worst:
            courtyard_out.append(f"{ref} {worst[0]:.2f} mm past the {worst[1]}")
    # board texts on the silkscreens (the name, BOARD_REV, test-pad names): a corner of one off the board or
    # inside a hole is not printed
    lost_text = []
    for d in b.GetDrawings():
        if d.GetLayer() not in (pcbnew.F_SilkS, pcbnew.B_SilkS) or not isinstance(d, pcbnew.PCB_TEXT):
            continue
        bb = d.GetBoundingBox()
        corners = [(bb.GetLeft(), bb.GetTop()), (bb.GetRight(), bb.GetTop()), (bb.GetLeft(), bb.GetBottom()),
                   (bb.GetRight(), bb.GetBottom())]
        if not all(outl.Contains(pcbnew.VECTOR2I(int(x), int(y))) for x, y in corners):
            p = d.GetPosition()
            lost_text.append(f"'{d.GetText()}' ({b.GetLayerName(d.GetLayer())}, at {mm(p.x):.1f}, {mm(p.y):.1f})")
    rows = sorted((d, f"{w}", p, c) for (w, _), (d, p, c) in found.items())
    bad = [r for r in rows if r[0] < COPPER_EDGE - 1e-6]
    print(f"routed edges: {len(contours)} contours ({', '.join(c for c, _ in contours)}); copper within about 1 mm "
          f"of one: {len(rows)} features")
    for d, w, p, c in rows[:12]:
        print(f"  {d:.3f} mm  {w} at ({p[0]:.2f}, {p[1]:.2f}), to the {c}")
    print(f"copper closer than {COPPER_EDGE:.2f} mm to a routed edge (REQ-FAB-03): {len(bad)}"
          + (": " + "; ".join(f"{w} {d:.3f} mm ({c})" for d, w, p, c in bad) if bad else ""))
    print("courtyards past the outline or into a hole (J1 by design excepted): "
          + (", ".join(sorted(set(courtyard_out))) or "none"))
    print("silkscreen texts not wholly on the board (off its edge or in a hole): " + ("; ".join(lost_text) or "none"))
    webs = []
    for ci, (name, pts) in enumerate(contours):
        best = None
        for p in densify(pts, step=0.1):
            d, cj = near(p, skip=ci)
            if d is not None and (best is None or d < best[0]):
                best = (d, p, contours[cj][0])
        if best:
            webs.append((best[0], name, best[2], best[1]))
    for d, a, c, p in sorted(webs)[:3]:
        print(f"narrowest FR4 web: {d:.2f} mm between the {a} and the {c} at ({p[0]:.1f}, {p[1]:.1f})")
    return result(not bad and not courtyard_out and not lost_text)


def plugs():
    """VER-184 (DF-170): the cell's plug must not fit the speaker's header. Today J2 and J4 are the same PicoBlade
    part, so it does whatever the distances; they are printed (centre to centre, pcbnew) with the speaker's lead
    length, to show the speaker cannot reach J2."""
    parts, _ = design()
    j2, j4 = parts["J2"], parts["J4"]
    same = (j2["fp"], j2["lcsc"]) == (j4["fp"], j4["lcsc"])
    pcbnew, b = board()
    mm = pcbnew.ToMM
    fps = {f.GetReference(): f for f in b.GetFootprints()}
    pos = {r: (mm(fps[r].GetPosition().x), mm(fps[r].GetPosition().y)) for r in ("J2", "J4")}
    sx, sy = [float(v) for v in re.search(r"SPEAKER = dict\(at=\(([\d.]+), ([\d.]+)\)",
                                          open(os.path.join(BOARD, "placement.py")).read()).groups()]
    lead = 35.0      # mm: the GSPK151035PN's leads (bom.md, placement.py)
    print(f"J2 {j2['value']} ({j2['lcsc']}) at ({pos['J2'][0]:.2f}, {pos['J2'][1]:.2f}); "
          f"J4 {j4['value']} ({j4['lcsc']}) at ({pos['J4'][0]:.2f}, {pos['J4'][1]:.2f}): "
          f"{math.dist(pos['J2'], pos['J4']):.1f} mm apart")
    print(f"the speaker's centre ({sx}, {sy}) is {math.dist((sx, sy), pos['J2']):.1f} mm from J2: its {lead:g} mm "
          "leads cannot reach J2; the cell's lead length is not known until VER-37 and VER-49, and with the case open "
          "the cell can sit anywhere")
    print("J2 and J4 are " + ("the same part: the cell's plug fits J4 (DF-170)" if same else "different parts: keyed apart"))
    return result(not same)


def ntc(t_c, r25=TH1_R25, beta=TH1_B):
    return r25 * math.exp(beta * (1 / (t_c + 273.15) - 1 / 298.15))


def tsband():
    """The TS voltage a good unit shows at the measured ambient: I_TS into TH1, with I_TS's tolerance, TH1's 1%
    R25 and B, and the ambient reading +-2 C (the meter and the board's own warmth)."""
    amb = [float(a) for a in sys.argv[2:]] or [18.0, 20.0, 22.0, 25.0, 28.0]
    print(f"TS = I_TS x TH1: I_TS {I_TS * 1e6:g} uA +-{I_TS_TOL:.0%} (SLUS810N's INTC, 72-78 uA), TH1 {TH1_R25 / 1e3:g}k "
          f"B{TH1_B:g} +-{TH1_TOL:.0%}, the ambient +-2 C")
    for t in amb:
        lo = I_TS * (1 - I_TS_TOL) * ntc(t + 2, TH1_R25 * (1 - TH1_TOL), TH1_B * (1 + TH1_TOL))
        hi = I_TS * (1 + I_TS_TOL) * ntc(t - 2, TH1_R25 * (1 + TH1_TOL), TH1_B * (1 - TH1_TOL))
        print(f"  ambient {t:.1f} C: TS {lo:.3f}-{hi:.3f} V (nominal {I_TS * ntc(t):.3f} V)")
    print("a fixed 10k in TH1's place reads about 0.75 V at any temperature: VER-135's finger step tells it apart")
    return 0


def cvload():
    """The load in J2's place that keeps U2 in CV without terminating, on any unit (O-R2).

    SLUS810N gives pre-charge a factor of its own (KPRECHG, 70-106 A-ohm), so the pre-charge current is
    KPRECHG / R5 and does not track the unit's ICHG; termination is 0.09-0.11 of the unit's own ICHG.
    Leaving pre-charge needs the load's voltage at the smallest pre-charge current to clear V_LOWV's
    maximum (a resistor above a low bound); never terminating needs the load to draw more than the
    largest termination current at VBAT(REG)'s minimum (a resistor below a high bound).  The two
    bounds cross, so no resistor does both.  The load that does is CV_LOAD_R in parallel with a TL431
    shunt set to 3.5 V in series with CV_SHUNT_R: off in pre-charge, on in CV.  Both bounds are
    checked at the load's own worst corners (1% resistors, the TL431's set point at +-2%)."""
    parts, _ = design()
    r5 = ohms(parts["R5"]["value"])
    ichg = (KISET[0] / (r5 * 1.01), KISET[1] / (r5 * 0.99))      # A: ICHG = KISET / R5, R5 1%
    pre = (KPRECHG[0] / (r5 * 1.01), KPRECHG[1] / (r5 * 0.99))   # A: the pre-charge current, KPRECHG / R5
    term = (ITERM[0] * ichg[0], ITERM[1] * ichg[1])               # A: the termination current band
    lo = V_LOWV[1] / pre[0]            # ohm: above this a resistor leaves pre-charge on every unit
    hi = VBAT_REG[0] / term[1]         # ohm: below this a resistor never terminates on every unit
    print(f"R5 {r5:g} 1%: fast charge {ichg[0] * 1e3:.0f}-{ichg[1] * 1e3:.0f} mA (KISET {KISET[0]:g}-{KISET[1]:g} A-ohm "
          f"over R5); pre-charge {pre[0] * 1e3:.1f}-{pre[1] * 1e3:.1f} mA (KPRECHG {KPRECHG[0]:g}-{KPRECHG[1]:g} A-ohm "
          f"over R5: its own factor, not a share of ICHG); termination {term[0] * 1e3:.1f}-{term[1] * 1e3:.1f} mA "
          f"({ITERM[0]:.0%}-{ITERM[1]:.0%} of the unit's own ICHG); V_LOWV {V_LOWV[0]}-{V_LOWV[1]} V; "
          f"VBAT(REG) {VBAT_REG[0]}-{VBAT_REG[1]} V (SLUS810N)")
    print(f"a resistor alone: above {lo:.1f} ohm it leaves pre-charge on every unit, below {hi:.1f} ohm it never "
          f"terminates on every unit")
    if lo > hi:
        print(f"no resistive window: {lo:.1f} ohm is above {hi:.1f} ohm, so no single resistor holds CV and refuses "
              f"to terminate at the same time (O-R2); a two-step load does not help either: pre-charge ends after a "
              f"25 ms deglitch and termination follows after another 25 ms (tDGL1(LOWV), tDGL(TERM))")
    else:
        print(f"the resistive window is {lo:.1f}-{hi:.1f} ohm; its geometric middle {math.sqrt(lo * hi):.1f} ohm")

    div = 10e3 + 24.9e3                # ohm: the TL431's set-point divider, always across the shunt

    def amps(v, r, vs, rs):
        """The load's current at a cell voltage v: the resistor, the divider and the shunt branch."""
        return v / r + v / div + max(0.0, (v - vs) / rs)

    def volts(i, r, vs, rs):
        lo_v, hi_v = 0.0, 10.0
        for _ in range(60):
            mid = (lo_v + hi_v) / 2
            lo_v, hi_v = (mid, hi_v) if amps(mid, r, vs, rs) < i else (lo_v, mid)
        return lo_v

    r_lo, r_hi = CV_LOAD_R * (1 - R_TOL), CV_LOAD_R * (1 + R_TOL)
    s_lo, s_hi = CV_SHUNT_R * (1 - R_TOL), CV_SHUNT_R * (1 + R_TOL)
    v_pre = volts(pre[0], r_lo, CV_SHUNT_V[0], s_lo)                       # the lowest voltage in pre-charge
    i_cv = amps(VBAT_REG[0], r_hi, CV_SHUNT_V[1], s_hi)                    # the least current in CV
    i_max = amps(VBAT_REG[1], r_lo, CV_SHUNT_V[0], s_lo)                   # the most current in CV
    i_sh = max(0.0, (VBAT_REG[1] - CV_SHUNT_V[0]) / s_lo)
    print(f"the load: {CV_LOAD_R:g} ohm 1% in parallel with a TL431 set to {sum(CV_SHUNT_V) / 2:g} V "
          f"({CV_SHUNT_V[0]}-{CV_SHUNT_V[1]} V: 10k over 24.9k, 1%, any TL431 grade) in series with {CV_SHUNT_R:g} ohm 1%")
    ok_pre = v_pre > V_LOWV[1]
    ok_cv = i_cv > term[1]
    ok_max = i_max < ichg[0]
    print(f"  pre-charge: at the smallest pre-charge current ({pre[0] * 1e3:.1f} mA) the load sits at {v_pre:.3f} V "
          f"against V_LOWV's {V_LOWV[1]} V maximum: {'it leaves pre-charge' if ok_pre else 'STUCK in pre-charge'} "
          f"({v_pre - V_LOWV[1]:+.3f} V)")
    print(f"  CV: at VBAT(REG) {VBAT_REG[0]} V the load draws at least {i_cv * 1e3:.1f} mA against the largest "
          f"termination current {term[1] * 1e3:.1f} mA: {'it never terminates' if ok_cv else 'it TERMINATES'} "
          f"({(i_cv - term[1]) * 1e3:+.1f} mA)")
    print(f"  at most {i_max * 1e3:.1f} mA at {VBAT_REG[1]} V, under the smallest fast-charge current "
          f"{ichg[0] * 1e3:.0f} mA, so the unit reaches CV: {'ok' if ok_max else 'NOT in CV'}; the TL431 then "
          f"dissipates {CV_SHUNT_V[1] * i_sh * 1e3:.0f} mW (a TO-92 part), the 22 ohm {s_lo * i_sh ** 2 * 1e3:.0f} mW "
          f"and the {CV_LOAD_R:g} ohm {VBAT_REG[1] ** 2 / r_lo * 1e3:.0f} mW (0.25 W parts)")
    print("not this load: a low-voltage zener (about 90 ohm of dynamic resistance at a few mA, so it starves the CV "
          "current) or three silicon diodes (they conduct at about 2 V, so the cell never leaves pre-charge)")
    print("the other loads that work: a real cell at 4.0-4.1 V, read at TP3 while its taper current is over "
          f"{term[1] * 1e3:.0f} mA; an electronic load only with its turn-on voltage set over V_LOWV, since a constant "
          "current faces the same two bounds as a resistor")
    print("the load " + ("works on every unit" if ok_pre and ok_cv and ok_max else "does NOT work on every unit"))
    return 0


def tempwin():
    """O-R3: the charge window TH1 gives against U2's fixed TS thresholds, from SLUS810N's limits.

    U2 drives I_TS (72-78 uA) into TH1, an NTC to ground, and watches TS: charging pauses when TS falls
    under V_HOT (hot) or rises over V_COLD (cold).  The hot edge is the highest temperature at which
    charging can still run: the trip resistance is smallest (V_HOT's minimum over I_TS's maximum) and
    TH1 falls slowest above 25 C (R25 high, B low).  The cold edge is the lowest temperature at which
    charging can still run: the trip resistance is largest (V_COLD's maximum over I_TS's minimum) and
    TH1 rises slowest below 25 C (R25 low, B low).  Every limit here is a datasheet minimum or maximum,
    so the worst corner bounds every unit: it is the one-sided 99.9% figure REQ-BAT-03 asks for, without
    assuming a distribution inside the limits."""
    def temp_at(r, beta, r25):
        return 1 / (1 / 298.15 + math.log(r / r25) / beta) - 273.15

    def b_for(t_c, r, r25):
        return math.log(r / r25) / (1 / (t_c + 273.15) - 1 / 298.15)

    i_lo, i_hi = I_TS * (1 - I_TS_TOL), I_TS * (1 + I_TS_TOL)
    hot_nom_r, cold_nom_r = sum(V_HOT) / 2 / I_TS, sum(V_COLD) / 2 / I_TS
    hot_r, cold_r = V_HOT[0] / i_hi, V_COLD[1] / i_lo
    r25_hot, r25_cold = TH1_R25 * (1 + TH1_TOL), TH1_R25 * (1 - TH1_TOL)
    print(f"SLUS810N: V_HOT {V_HOT[0]}-{V_HOT[1]} V, V_COLD {V_COLD[0]}-{V_COLD[1]} V, INTC {i_lo * 1e6:.0f}-"
          f"{i_hi * 1e6:.0f} uA (bq24074.txt:823-826); TH1 {TH1_R25 / 1e3:g}k +-{TH1_TOL:.0%}, B{TH1_B:g} "
          f"+-{TH1_B_TOL:.0%} (design.py)")
    rows = [("nominal (0.300 V and 2.100 V at 75 uA, TH1 nominal)",
             temp_at(cold_nom_r, TH1_B, TH1_R25), temp_at(hot_nom_r, TH1_B, TH1_R25)),
            ("the charger's limits, TH1 nominal",
             temp_at(cold_r, TH1_B, TH1_R25), temp_at(hot_r, TH1_B, TH1_R25)),
            (f"the worst corner (the charger's limits, R25 and B at TH1's tolerance)",
             temp_at(cold_r, TH1_B * (1 - TH1_B_TOL), r25_cold), temp_at(hot_r, TH1_B * (1 - TH1_B_TOL), r25_hot))]
    print(f"the trip resistances: hot {hot_r:.0f} ohm (nominal {hot_nom_r:.0f}), cold {cold_r:.0f} ohm "
          f"(nominal {cold_nom_r:.0f})")
    for name, cold, hot in rows:
        print(f"  {name}: charging runs from {cold:.1f} C to {hot:.1f} C")
    cold, hot = rows[-1][1], rows[-1][2]
    # the B that holds the hot edge at 45 C at the worst corner, and what it does to the cold edge
    b_nom = b_for(45.0, hot_r, TH1_R25)
    b_worst = b_for(45.0, hot_r, r25_hot) / (1 - TH1_B_TOL)
    cold_at = temp_at(cold_r, b_worst * (1 - TH1_B_TOL), r25_cold)
    print(f"a 45 C hot edge: B {b_nom:.0f} with TH1 exactly nominal; B {b_worst:.0f} or more for a 10k 1% part "
          f"with B +-1% at the worst corner, whose cold edge is then {cold_at:.1f} C")
    for b in (4250.0, 4350.0):
        print(f"  B{b:g} 1%: the hot edge {temp_at(hot_r, b * (1 - TH1_B_TOL), r25_hot):.1f} C at the worst corner "
              f"({temp_at(hot_nom_r, b, TH1_R25):.1f} C nominal)")
    ok = hot <= 45.0 and cold >= 0.0
    print(f"REQ-BAT-03 (0-45 C, or the cell's own limits once VER-37 has its spec sheet): the window is {cold:.1f} C "
          f"to {hot:.1f} C at the worst corner: " + ("met" if ok else "NOT met"))
    return result(ok)


CHECKS = {"holes": holes, "swell": swell, "inrush": inrush, "backlight": backlight, "cutoff": cutoff, "screws": screws,
          "mlcc": mlcc, "th1": th1, "cellgap": cellgap, "cpl": cpl, "drill": drill, "vbus": vbus, "bom": bom,
          "edge": edge, "plugs": plugs, "tsband": tsband, "cvload": cvload, "tempwin": tempwin}


def main():
    if len(sys.argv) < 2 or sys.argv[1] not in CHECKS or (len(sys.argv) > 2 and sys.argv[1] != "tsband"):
        print(__doc__.split("\n\n")[2] + "\n\n" + __doc__.split("\n\n")[3] if "--list" in sys.argv else __doc__)
        return 0 if "--list" in sys.argv else 2
    return CHECKS[sys.argv[1]]()


if __name__ == "__main__":
    sys.exit(main())
