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
  cellgap    VER-181  a worst-case cell to the routed FR4 edge on every side, after tolerances, and its fit in the pocket
  cpl        VER-176  the CPL fab.py writes against each footprint's pads' centre (pcbnew)
  drill      VER-131  the drill files fab.py writes: plated and unplated apart, every hole placed
  vbus       VER-177  the lowest VBUS that over-stresses a part (a faulty source)
  bom        VER-167  every contact class has its bought parts in docs/bom.md
  holes      VER-114  every drilled hole 0.30 mm or larger (pcbnew)
  swell      VER-182  free height above a worst-case cell, under whatever lies over its pocket
  edge       VER-18   copper to every routed edge, pads and part bodies inside the outline (pcbnew)
  plugs      VER-184  the cell's plug (from the cell's spec, analysis/cell.json) fits the cell's header and not
                      the speaker's; the two headers keyed apart (their footprints' families)
  spkplug    VER-202  the speaker's plug (bom.md's speaker row) fits J4 and not J2; its leads reach J4, not J2
  netclass   VER-201  build_pcb.py's net classes name only the board's nets, and every net carrying 0.25 A or
                      more sits in one
  tempwin    VER-32   the charge window TH1 gives against U2's TS thresholds, from SLUS810N
  cellspec   VER-160  the cell's spec sheet (analysis/cell.json, VER-37) against R5's charge current, TH1's window and
                      case.scad's cell size; BLOCKED until VER-37 writes cell.json

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
VREG_TOL = 0.015         # TLV75733P output accuracy +-1.5% over TJ -40 to 125 C (tlv757p.txt:255); +-1% to 85 C (252)
VREG_TOL_85 = 0.01       # TLV75733P output accuracy, TJ -40 to 85 C (tlv757p.txt:252)
U3_VDO_1A = 0.425        # V: TLV75733P dropout at 1 A, VOUT 3.3-5.0 V, DBV, TJ to 85 C (tlv757p.txt:298); 475 mV to 125 C (316)
CV_TOL = 0.01            # BQ24073 VBAT(REG) 4.2 V +-1% (SLUS810N; design.py and pinmap.md quote +-1%)
I_3V3_AT_CUTOFF = 0.15   # A: the 3V3 load between Wi-Fi bursts (REQ-PWR-15's average), the load the cutoff is read under
I_3V3_PEAK = 0.355 + 0.15   # A: a Wi-Fi TX burst (802.11b 20.5 dBm, 355 mA peak, esp32-s3-wroom-1.txt:1662) on that load
ADC_ATTEN3_ERR = 0.050   # V: the ESP32-S3 ADC's total error at ATTEN3 (0-2900 mV) after its calibration (esp32-s3.txt:3804)
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
# TH1's R25 and B are read from design.py's value ('10k B4460'; th1_spec below), so a TH1 change moves every number
# here.  Its tolerances are not in design.py: 1% on R25 and 1% on B, as design.py's comment and the part numbers
# say (TCTR0805F10K0F4460T at 71737ae; SDNT1005X103F3950FTF before).  The TCTR0805's datasheet is not on disk
# (LCSC C525375): VER-34 holds the B tolerance BLOCKED until it is (2026-10-27).
TH1_TOL = 0.01               # TH1's R25 tolerance
TH1_B_TOL = 0.01             # TH1's B tolerance (to confirm on the TCTR0805 datasheet, VER-34)
# The VBAT sense buffer (U10, MCP6001T-I/OT).  Its datasheet is on disk since 2026-10-28 (mcp6001.pdf): the bias
# is typical only (1 pA at 25 C, 19 pA at 85 C) and the drift typical only (2 uV/C), so those stay pessimistic:
OPAMP_IB_MAX = 1e-9          # A: input bias over temperature, 50 x the datasheet's 19 pA typical at 85 C (mcp6001.txt:141)
OPAMP_ROUT = 100.0           # ohm: closed-loop output resistance at DC, a pessimistic figure for a unity buffer
OPAMP_VOS_BUF = 0.0045       # V: input offset, maximum (mcp6001.txt:135); constant, so the gain calibration takes most of it
OPAMP_DRIFT = 5e-6           # V/C: offset drift, 2.5 x the datasheet's 2 uV/C typical (mcp6001.txt:136)
CLAMP_V = 0.5                # V: an unpowered CMOS input's protection diode to its VDD at a few uA (an estimate; MCP6001's
                             # analog inputs may go to VDD + 1.0 V at up to 2 mA, mcp6001.txt:115-117)
# The per-unit CV screen's load (VER-134, VER-145): a resistor alone cannot do it (cvload), so the
# load is a resistor in parallel with a shunt branch that conducts only above a set voltage: a TL431
# (cathode-to-anode set to 3.5 V by 10k over 24.9k, 1%) in series with 22 ohm.  A low-voltage zener
# is too soft for this branch (about 90 ohm of dynamic resistance at a few mA), and three diodes
# conduct at about 2 V, under V_LOWV.
CV_LOAD_R = 160.0            # ohm, 1%: the load's resistor
CV_SHUNT_V = (3.43, 3.57)    # V: the TL431 branch's set point, 3.5 V +-2% (Vref 2.495 V x (1 + 10k / 24.9k): a plain
                             # TL431's reference and the divider's 1% resistors)
CV_SHUNT_R = 22.0            # ohm, 1%: in series with the TL431
# ESP32-S3 GPIO input current, maximum: IIH and IIL 50 nA (esp32-s3.txt:3748-3749), at 25 C only: a stand-in of
# 20 x that (1 uA, the RP2350's figure before 39cc07f) covers 45 C
I_LEAK = 1e-6                # A: stand-in, 20 x the ESP32-S3's IIH 50 nA (esp32-s3.txt:3748)
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
ABS = {"U2 IN": 28.0, "U2 EN1": 7.0, "U3 EN": 6.0}    # absolute maxima (bq24074.txt:595, 599; TLV75733P EN 6.0 V, tlv757p.txt:179)
PD_MAX = 20.0            # V: the highest fixed USB PD voltage, what a broken source may put out


def design():
    ns = {"__name__": "design_numbers"}
    exec(compile(open(os.path.join(BOARD, "design.py")).read(), "design.py", "exec"), ns)
    return {p["ref"]: p for p in ns["PARTS"]}, ns


def th1_spec():
    """(R25 in ohm, B, mpn) from design.py's TH1, whose value reads '<R25> B<beta>' (e.g. '10k B4460')."""
    parts, _ = design()
    p = parts["TH1"]
    m = re.search(r"\bB(\d+)\b", p["value"])
    if not m:
        sys.exit(f"design_numbers: TH1's value {p['value']!r} names no B")
    return ohms(p["value"]), float(m.group(1)), p["mpn"]


def ohms(v):
    v = v.split()[0]
    m = re.fullmatch(r"([\d.]+)([kM]?)", v)
    return float(m.group(1)) * {"": 1, "k": 1e3, "M": 1e6}[m.group(2)]


def farads(v):
    m = re.match(r"([\d.]+)([pnu])", v)
    return float(m.group(1)) * {"p": 1e-12, "n": 1e-9, "u": 1e-6}[m.group(2)]


def scad():
    """Top-level 'name = expression;' assignments of case.scad, evaluated in order where Python can."""
    rad = math.radians                    # OpenSCAD's trigonometry is in degrees (the one-piece top's slope)
    env = {"min": min, "max": max, "abs": abs, "sqrt": math.sqrt, "sin": lambda a: math.sin(rad(a)),
           "cos": lambda a: math.cos(rad(a)), "asin": lambda x: math.degrees(math.asin(x)),
           "acos": lambda x: math.degrees(math.acos(x))}
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
    # carry-over P8 (2026-10-28): the charge at the worst-case voltage and the part's own +tolerance, through its DC-bias curve
    import paper_checks as pc
    q_hi, why = 0.0, []
    for r, p in sorted(caps.items()):
        if not set(p["pins"].values()) & vbus_nets:
            continue
        hi = farads(p["value"]) * (1 + pc.ctol(parts, r))
        cur = pc.bias_curve(p["mpn"])
        if cur is None:
            q_hi += hi * 5.25
            why.append(f"{r} {p['mpn']} +{pc.ctol(parts, r):.0%}: {hi * 1e6:.1f} uF, no DC-bias curve on disk (taken whole)")
            continue
        # the charge a plug-in delivers is the integral of C(V) dV from 0 to 5.25 V, read off Samsung's curve
        steps = 200
        q = sum(hi * (1 + pc.at_bias(cur, 5.25 * (k + 0.5) / steps) / 100) * 5.25 / steps for k in range(steps))
        q_hi += q
        why.append(f"{r} {p['mpn']} +{pc.ctol(parts, r):.0%}: {hi * 1e6:.1f} uF at 0 V, {q * 1e6:.1f} uC charged to 5.25 V along "
                   f"{cur[1]}")
    print(f"worst-case charge at 5.25 V and +tolerance through the DC-bias curve: {q_hi * 1e6:.1f} uC against 50 uC ("
          + "; ".join(why) + ")")
    return result(vbus <= 10e-6 * 1.001 and q_hi <= 50e-6 and bridges == ["U2"] and parts["U2"]["value"].startswith("BQ2407"))


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
    print(f"worst case at {dmax:.0%} duty (3V3 +{VREG_TOL:.1%}, R61 +1%, R60 -1%, offset +{OPAMP_VOS * 1e3:.1f} mV, "
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
    """VER-138 since 39cc07f: the ESP32-S3's ADC has its own reference (not 3V3), so the old dropout knee, where the
    reading rose as 3V3 fell, is gone. What is left: the ADC's own error at ATTEN3, the gain calibration's CV term,
    the buffer's temperature terms, and 3V3 in dropout during a Wi-Fi burst at the cutoff (the module's 3.0 V)."""
    parts, _ = design()
    if not parts["U4"]["value"].startswith("ESP32-S3"):
        sys.exit("design_numbers: cutoff reads the ESP32-S3's ADC; U4 is " + parts["U4"]["value"])
    print(f"U4 {parts['U4']['value']}: VBAT_SENSE on ADC1 at 12 dB (ATTEN3, 0-2900 mV: VBAT/2 is 1.75 V at the cutoff); "
          f"total error after Espressif's calibration +-{ADC_ATTEN3_ERR * 1e3:.0f} mV at the pin (esp32-s3.txt:3804), "
          "measured at 25 C with Wi-Fi off (3780-3781); its own reference, not 3V3 (pinmap.md, Battery voltage)")
    print(f"R17/R18 {parts['R17']['value']}/{parts['R18']['value']}: VBAT/2; calibration (pinmap.md): one gain point at "
          f"charge-done, the cell taken as 4.20 V; the charger's CV is 4.20 V +-{CV_TOL:.0%}")
    ok = True
    gain = 2.0
    raw = ADC_ATTEN3_ERR * gain
    print(f"with Espressif's calibration only: +-{raw * 1e3:.0f} mV of VBAT at 3.50 V against REQ-BAT-12's window "
          "(3.50 +0.05/-0.00, 50 mV wide): " + ("inside" if 2 * raw <= 0.05 else "OVER: the window needs a calibration on "
                                                 "the unit (REQ-TST-06)"))
    ok &= 2 * raw <= 0.05
    # one gain point at 4.20 V: an offset in the ADC's error is not removed by a gain; the datasheet gives the total only
    lo, hi = 3.50 * (1 - CV_TOL), 3.50 * (1 + CV_TOL)
    print(f"with the gain set at charge-done and no offset: the cutoff fires at a true {lo:.3f}-{hi:.3f} V from the CV's "
          "+-1% alone; esp32-s3.pdf splits its +-50 mV into no offset and gain, so no one-point calibration can be shown "
          "to bring it inside 50 mV: the per-unit 3.50 V point (VER-155) and the bench sweep (VER-139) must")
    ok &= lo >= 3.50 - 1e-9
    print("3V3 itself at the cutoff during a Wi-Fi burst is VER-204's (paper_checks.py module): the ADC's reference is "
          "its own, so the rail's sag does not move the reading")
    # O-R15: GPIO47's pad leakage through whatever drives the pin; it changes with temperature, so a one-point
    # gain calibration does not remove it.  Since 71737ae a unity buffer (U10) sits between the divider's node and
    # the pin: the pad's leakage then flows from the op-amp's output, and the op-amp's own bias current flows
    # through the divider instead (DEF-40, DEC-07)
    r17, r18 = ohms(parts["R17"]["value"]), ohms(parts["R18"]["value"])
    rth = r17 * r18 / (r17 + r18)
    gain = (r17 + r18) / r18                             # VBAT per volt at the pin
    node = parts["R17"]["pins"]["2"]                     # the divider's node (R17: VBAT to it)
    buf = sorted(r for r, p in parts.items() if r.startswith("U") and node in p["pins"].values()
                 and "VBAT_SENSE" in p["pins"].values()) if node != "VBAT_SENSE" else []
    if not buf:
        band = I_LEAK * rth * gain                       # the pin's error, scaled back to VBAT
        fit = 0.05 / 2 / (rth * gain)
        print(f"pad leakage: +-{I_LEAK * 1e6:g} uA (a stand-in, 20 x the ESP32-S3's IIH, esp32-s3.txt:3748) "
              f"through {rth / 1e3:.0f} kohm moves VBAT's reading +-{band:.3f} V; REQ-BAT-12's 50 mV window allows "
              f"+-{fit * 1e9:.0f} nA of leakage that changes with temperature, before any other error")
    else:
        u = parts[buf[0]]
        pad = I_LEAK * OPAMP_ROUT * gain
        bias = OPAMP_IB_MAX * rth * gain
        drift = OPAMP_DRIFT * 20 * gain                  # 25 C to 45 C
        band = pad + bias + drift
        # the offset is constant: the gain set at charge-done (4.20 V) takes it in, leaving this much at 3.50 V
        resid = 4.2 * (1.75 + OPAMP_VOS_BUF) / (2.1 + OPAMP_VOS_BUF) - 3.5
        print(f"{buf[0]} ({u['value']}, {u['mpn']}) buffers {node} onto VBAT_SENSE: the pad's +-{I_LEAK * 1e6:g} uA "
              f"(stand-in, esp32-s3.txt:3748) through its output ({OPAMP_ROUT:g} ohm, pessimistic) moves VBAT's reading "
              f"+-{pad * 1e3:.2f} mV; its bias ({OPAMP_IB_MAX * 1e9:g} nA, a stand-in 50 x the 19 pA typical at 85 C, "
              f"mcp6001.txt:141) "
              f"through {rth / 1e3:.0f} kohm +-{bias * 1e3:.2f} mV; its drift ({OPAMP_DRIFT * 1e6:g} uV/C over 20 C) "
              f"+-{drift * 1e3:.2f} mV: +-{band * 1e3:.2f} mV in all that changes with temperature (was +-"
              f"{I_LEAK * rth * gain:.3f} V unbuffered)")
        print(f"  its {OPAMP_VOS_BUF * 1e3:g} mV offset is constant: after the gain calibration at 4.20 V it leaves "
              f"{resid * 1e3:+.1f} mV at 3.50 V (inside the CV term above); offset +-4.5 mV maximum (mcp6001.txt:135), "
              "drift and bias typical only (136, 141): the stand-ins above stay")
        # off (3V3 down), the buffer is unpowered and its input clamps the divider's node about CLAMP_V over 0 V
        i_off = (4.2 - CLAMP_V) / r17
        print(f"  off: {buf[0]} unpowered, its input's protection diode holds {node} near {CLAMP_V:g} V, so R17 draws "
              f"{i_off * 1e6:.1f} uA from a 4.2 V cell (was {4.2 / (r17 + r18) * 1e6:.1f} uA) and "
              f"{(i_off - CLAMP_V / r18) * 1e6:.1f} uA flows into the unpowered +3V3 rail (REQ-PWR-07, VER-65, VER-159)")
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
            # 39cc07f: H4 left the glass's corner for seat 1's deck (case.scad h4_pin, the end screws' pilot)
            ("side (H1-H3)", e.get("screw_l_side", e["screw_l"]), e["side_engage"], 4.0, 0.5),
            ("deck (H4, H5, H6)", e.get("screw_l_end", e["screw_l"]), e["deck_engage"], 3.0, 0.1)):
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
    # the cell's span along the hole's right edge (KiCad y = 100 - case y): the pocket's, less what a nominal cell
    # leaves at the end away from the one it is pushed to; with the cell along x (before 71737ae) the old rule
    p = pocket(e)
    if p["along_y"]:
        nom_short = (p["y1"] - p["y0"]) - e["cell_nom"][0]
        lo, hi = 100 - p["y1"] + nom_short, 100 - p["y0"]
        cell_face = 100 + p["x1"]
    else:
        gap_y = (y1 - y0 - e["cell"][1]) / 2
        lo, hi = y0 + gap_y, y1 - gap_y
        cell_face = 100 + p["x1"]
    to_edge = tx - x1
    print(f"cell hole x {x0}-{x1}, y {y0}-{y1} (its long side along {'y' if p['along_y'] else 'x'}); worst-case cell "
          f"{e['cell'][0]} x {e['cell'][1]}; TH1 ({fps['TH1'].GetValue()}) at ({tx:.2f}, {ty:.2f}) on the board, "
          f"{to_edge:.2f} mm right of the hole's right edge, {tx - cell_face:.2f} mm from the pocket's right wall "
          "(the nearest a cell's face can be)")
    in_span = lo + 2 <= ty <= hi - 2
    print(f"TH1 within the cell's span along the edge (y {lo:.1f}-{hi:.1f}, 2 mm in from each end): {in_span}")
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


POCKET_CLEAR = 0.1   # mm a side: the cell's free fit in its printed pocket, beyond PRINT_TOL (round 3's O-S1; PROPOSED in
                     # REQ-BAT-08): a pocket printed PRINT_TOL small must still take a cell at its spec maximum unpressed


def pocket(e):
    """The tub's pocket for the cell, in the case frame: its box, its inset from the hole on each face, which way the
    cell's long side runs, and which face its leads leave.  Since 71737ae case.scad draws the pocket as cell_pocket_box
    (the cell along y, pushed to the seat-1 end, its leads out of the seat-2 end, cell_fit_side/push/lead); before, it
    was cell_pocket_fit inside the hole all round, the cell along x with its leads at the left end."""
    (hx0, hy0), (hx1, hy1) = e["cell_hole"]
    if "cell_pocket_box" in e:
        (px0, py0), (px1, py1) = e["cell_pocket_box"]
    else:
        f = e["cell_pocket_fit"]
        px0, py0, px1, py1 = hx0 + f, hy0 + f, hx1 - f, hy1 - f
    along_y = (hy1 - hy0) > (hx1 - hx0)
    inset = {"left": px0 - hx0, "right": hx1 - px1, "seat-1 end": py0 - hy0, "seat-2 end": hy1 - py1}
    lead = "seat-2 end" if along_y else "left"
    return dict(x0=px0, y0=py0, x1=px1, y1=py1, along_y=along_y, inset=inset, lead=lead,
                length=(py1 - py0) if along_y else (px1 - px0), width=(px1 - px0) if along_y else (py1 - py0),
                hole=((hy1 - hy0) if along_y else (hx1 - hx0), (hx1 - hx0) if along_y else (hy1 - hy0)))


def cellgap():
    """O-R7, round 2's and round 3's objections: the gap from a worst-case cell to the routed FR4 edge on every face
    (REQ-BAT-08), after the fab and print tolerances, and the cell's fit in its printed pocket (O-S1).  The cell lies
    in the tub's pocket, so the gap at each face is at least the pocket's inset there, wherever the cell rests: the
    pocket's walls are the stop (DEF-58), and no tape holds it.  Widening the hole alone fixes nothing while the
    pocket follows it (seeded in VER-129).  The fit: the pocket, printed PRINT_TOL small on each side, must still
    exceed a cell at its spec maximum by POCKET_CLEAR a side, or the pouch is pressed in (O-S1).  It prints what
    would pass."""
    e = scad()
    p = pocket(e)
    tol = ROUTE_TOL + PRINT_TOL + SCREW_FLOAT
    hl, hw = p["hole"]
    print(f"board hole {hl:.1f} x {hw:.1f} mm (the cell's long side along {'y' if p['along_y'] else 'x'}); the tub's "
          f"pocket {p['length']:.2f} x {p['width']:.2f} mm, inside the hole by "
          + ", ".join(f"{v:.2f} at the {k}" for k, v in p["inset"].items())
          + f" (its leads leave the {p['lead']}); tolerances: routing {ROUTE_TOL:g}, print {PRINT_TOL:g}, the board's "
          f"float on its screws {SCREW_FLOAT:g} mm ({tol:.2f} mm in all)")
    ok = True
    fit_need = 2 * (PRINT_TOL + POCKET_CLEAR)
    for label, (cl, cw, _) in (("worst-case", e["cell"]), ("nominal", e["cell_nom"])):
        for side, nom in p["inset"].items():
            need = 1.5 if side == p["lead"] else 0.5
            worst = nom - tol
            good = worst >= need - 1e-9
            ok &= good
            print(f"  {label} cell {cl:g} x {cw:g}, {side}{' (its leads)' if side == p['lead'] else ''}: at least "
                  f"{nom:.2f} mm nominal, {worst:.2f} mm after tolerances (REQ-BAT-08: {need:g}): "
                  f"{'ok' if good else 'too close'}")
        for axis, room, size in (("length", p["length"], cl), ("width", p["width"], cw)):
            spare = room - size
            good = spare >= fit_need - 1e-9
            ok &= good
            print(f"  {label} cell {cl:g} x {cw:g} in the pocket's {axis}: {spare:.2f} mm spare against "
                  f"{fit_need:.2f} (the print {PRINT_TOL:g} and a {POCKET_CLEAR:g} mm fit, each side): "
                  f"{'fits' if good else 'pressed or does not go in'}")
    f_need, l_need = 0.5 + tol, 1.5 + tol
    cl, cw = e["cell"][0], e["cell"][1]
    print(f"what passes: the pocket {f_need:.2f} mm inside the hole on every face but the lead end, {l_need:.2f} mm "
          f"there (the walls are the stop), and {fit_need:.2f} mm longer and wider than the cell; then")
    print(f"  with this hole ({hl:.1f} x {hw:.1f}): a cell of at most {hl - f_need - l_need - fit_need:.2f} x "
          f"{hw - 2 * f_need - fit_need:.2f} mm (its spec sheet, VER-37, and incoming inspection, VER-49)")
    need_l, need_w = cl + f_need + l_need + fit_need, cw + 2 * f_need + fit_need
    print(f"  with the worst-case {cl:g} x {cw:g} cell: a hole of at least {need_l:.2f} x {need_w:.2f} mm "
          f"(+{need_l - hl:.2f} along the cell, +{need_w - hw:.2f} across it: copper)")
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
    # 39cc07f: the hand-fitted parts the new board needs. A WROOM-1U has a U.FL connector and no antenna of its own
    # (esp32-s3-wroom-1.txt:100); the power key's cap is printed (case.scad pwr_cap), so it is the case's, not bom.md's
    for r, p in sorted(parts.items()):
        if "WROOM-1U" in (p["mpn"] or ""):
            hit = re.search(r"(?i)\bantenna\b", text) and re.search(r"(?i)u\.fl|ipex|mhf", text)
            print(f"{r} {p['mpn']}: an external antenna on a U.FL cable (esp32-s3-wroom-1.txt:100): "
                  + ("bom.md buys it" if hit else "bom.md buys no antenna and no U.FL cable: the module has no radio path"))
            ok &= bool(hit)
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
    need = ["cell", "cell_top", "cell_hole", "panel_w", "panel_l", "panel_fit", "frame_wall",
            "frame_h", "glass_gap", "z_ceil", "z_top_cu", "z_deck_under", "joint_y", "joint_w"]
    miss = [n for n in need if n not in e] + ([] if "cell_pocket_box" in e or "cell_pocket_fit" in e
                                               else ["cell_pocket_box"])
    if miss:
        print("case.scad: could not read " + " ".join(miss))
        return result(False)
    cell_t, top = e["cell"][2], e["cell_top"]
    pk = pocket(e)
    px0, px1, py0, py1 = pk["x0"], pk["x1"], pk["y0"], pk["y1"]           # the pocket: where the cell can be
    pw, pl = (e["panel_w"]) / 2, (e["panel_l"]) / 2
    fw, fl = (e["panel_w"] + e["panel_fit"]) / 2 + e["frame_wall"], (e["panel_l"] + e["panel_fit"]) / 2 + e["frame_wall"]
    ends = e["joint_y"] - e["joint_w"]

    def over(x0, x1, y0, y1):
        return x0 < px1 and x1 > px0 and y0 < py1 and y1 > py0
    regions = [("the panel's back", (-pw, pw, -pl, pl), e["glass_gap"]),
               ("the panel frame's walls", (-fw, fw, -fl, fl), e["z_ceil"] - e["frame_h"] - e["z_top_cu"])]
    if "uni_foot" in e and "uni_lim_phi" in e:
        # the one-piece top (since be437f8): case.scad's own uni_ceil(), z_deck_under from uni_foot out and the slope's
        # inside (uni_lim) between the band and the foot; joint_y - joint_w is the old decks' line and is not used
        # (it put the ends' 1.9 mm ceiling over the cell's seat-2 end, where the slope stands 3.4 mm over the board)
        def uni_lim(y):
            d, rs = e["uni_foot"] - abs(y), e["uni_r"] + e["uni_skin"]
            s = d / rs
            if d <= 0:
                return e["deck_face"] - e["uni_skin"]
            if s >= math.sin(math.radians(e["uni_lim_phi"])):
                return e["case_h"] - e["uni_skin"]
            return e["deck_face"] + e["uni_r"] - rs * math.sqrt(1 - s * s)
        foot = e["uni_foot"]
        for sy, name in ((1, "seat 2's"), (-1, "seat 1's")):
            y0, y1 = sorted((sy * fl, sy * foot))
            ys = [y0 + (y1 - y0) * k / 40 for k in range(41)]
            ys = [y for y in ys if py0 <= y <= py1]
            h = min((min(e["z_ceil"], uni_lim(y)) for y in ys), default=e["z_ceil"]) - e["z_top_cu"]
            regions.append((f"the slope's inside past the panel's frame ({name} side, uni_ceil)",
                            (-99, 99, y0, y1), h))
            regions.append((f"the ends' underside from the slope's foot out ({name}, |y| >= {foot:.2f})",
                            (-99, 99, foot, 99) if sy > 0 else (-99, 99, -99, -foot),
                            e["z_deck_under"] - e["z_top_cu"]))
        regions.append(("the band's underside", (-99, 99, -fl, fl), e["z_ceil"] - e["z_top_cu"]))
    else:
        regions += [("the ends' underside (past the band)", (-99, 99, ends, 99), e["z_deck_under"] - e["z_top_cu"]),
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
EDGE_ALLOWED = {"J1", "J8"}   # parts whose body stands past the outline by design: the USB-C receptacle's face on its tab;
                              # the microSD slot's mouth 0.45 mm past the left edge (placement.py J8, 39cc07f). Both need
                              # JLC's assembly to accept an overhang (VER-28's DFM report)


def edge(b=None):
    """VER-18: every copper feature (pours, tracks, vias, pads) at least COPPER_EDGE from every routed edge (the
    outline, the cell's hole, the speaker's hole), every pad and part body inside the outline (J1 by design
    excepted; a courtyard past it is printed), every silkscreen text wholly on the board, and the narrowest FR4
    web between two routed edges, printed. The edges are the board polygon pcbnew builds from Edge.Cuts (arcs as
    chords).  b: a pcbnew board already loaded (seed_faults.py moves a part in memory, writing no KiCad file).
    O-R2: the rule is compared with the data's distance, and each distance is printed beside its worst case, the
    nominal less ROUTE_TOL (JLC's outline tolerance, to confirm at VER-17): whether JLC's 0.30 mm is a data rule
    or an as-built one is what analysis/jlc_capabilities.md must say (DEF-56)."""
    pcbnew, b = board() if b is None else (__import__("pcbnew"), b)
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
    courtyard_out, body_out = [], []
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
            # a courtyard is the body plus KiCad's assembly margin: what may not cross the outline is a pad or the
            # body itself (its F.Fab or B.Fab outline), 2026-10-27
            pts = []
            for p in f.Pads():
                bb = p.GetBoundingBox()
                pts += [(bb.GetLeft(), bb.GetTop()), (bb.GetRight(), bb.GetBottom()), (bb.GetLeft(), bb.GetBottom()),
                        (bb.GetRight(), bb.GetTop())]
            for g in f.GraphicalItems():
                if g.GetLayer() in (pcbnew.F_Fab, pcbnew.B_Fab):
                    bb = g.GetBoundingBox()
                    pts += [(bb.GetLeft(), bb.GetTop()), (bb.GetRight(), bb.GetBottom()), (bb.GetLeft(), bb.GetBottom()),
                            (bb.GetRight(), bb.GetTop())]
            out = [q for q in pts if not outl.Contains(pcbnew.VECTOR2I(int(q[0]), int(q[1])))]
            if out:
                body_out.append(f"{ref} (a pad or its body past the outline)")
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
        print(f"  {d:.3f} mm (worst case {d - ROUTE_TOL:.3f})  {w} at ({p[0]:.2f}, {p[1]:.2f}), to the {c}")
    print(f"copper closer than {COPPER_EDGE:.2f} mm to a routed edge in the data (REQ-FAB-03): {len(bad)}"
          + (": " + "; ".join(f"{w} {d:.3f} mm ({c})" for d, w, p, c in bad) if bad else ""))
    worst = [r for r in rows if r[0] - ROUTE_TOL < COPPER_EDGE - 1e-6]
    print(f"worst case, the data's distance less ROUTE_TOL {ROUTE_TOL:.2f} mm (printed, not failed: O-R2, DEF-56): "
          f"{len(worst)} features under {COPPER_EDGE:.2f} mm"
          + (f", the closest {rows[0][0] - ROUTE_TOL:.3f} mm ({rows[0][1]}, to the {rows[0][3]})" if rows else ""))
    print("courtyards past the outline or into a hole (J1 by design excepted; printed, not failed): "
          + (", ".join(sorted(set(courtyard_out))) or "none"))
    print("pads or part bodies past the outline or into a hole (J1 by design excepted): "
          + (", ".join(sorted(set(body_out))) or "none"))
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
        print(f"narrowest FR4 web: {d:.2f} mm (worst case {d - 2 * ROUTE_TOL:.2f}, both edges off by ROUTE_TOL) "
              f"between the {a} and the {c} at ({p[0]:.1f}, {p[1]:.1f})")
    return result(not bad and not body_out and not lost_text)


FAMILY_FP = {"PicoBlade": "PicoBlade", "JST SH": "JST_SH", "JST PH": "JST_PH"}   # footprint name -> family
CELL_SPEC = os.path.join(os.path.dirname(HERE), "analysis", "cell.json")       # the cell's spec sheet, VER-37


def header(parts, net):
    """(ref, family, pitch in mm) of the connector whose pins carry net, from its footprint name in design.py."""
    for ref, p in sorted(parts.items()):
        if re.fullmatch(r"J\d+", ref) and net in (p.get("pins") or {}).values():
            fp = str(p.get("fp") or "")
            fam = next((f for f, k in FAMILY_FP.items() if k in fp), None)
            m = re.search(r"_P(\d+\.\d+)mm", fp)
            return ref, fam, float(m.group(1)) if m else None
    return None, None, None


def bom_row(word):
    """The line of docs/bom.md's parts table whose first cell names word."""
    for line in open(os.path.join(REF, "docs", "bom.md"), encoding="utf-8"):
        if line.startswith("|") and word in line.split("|")[1]:
            return line
    return ""


def plugs():
    """VER-184 (DF-170, F-R2): the cell's plug fits the cell's header and cannot enter the speaker's. The headers'
    families and pitches come from their footprint names in design.py (the connector on CELL_P, the one on SPK_P);
    the cell's plug comes from the cell's spec sheet, recorded in hardware/vv/analysis/cell.json ('plug_family',
    'plug_pitch_mm', 'source') once VER-37 has it. Until then the check is BLOCKED (exit 2) and prints what bom.md
    asks the seller for, which is an order instruction, not the part. The cells take 21 days (VER-49), so this is an
    ORDER check."""
    parts, _ = design()
    c_ref, c_fam, c_p = header(parts, "CELL_P")
    s_ref, s_fam, s_p = header(parts, "SPK_P")
    print(f"the cell's header {c_ref}: {c_fam}, {c_p} mm pitch; the speaker's header {s_ref}: {s_fam}, {s_p} mm pitch "
          "(footprint names in design.py)")
    keyed = (c_fam, c_p) != (s_fam, s_p) and None not in (c_fam, s_fam)
    print("the two headers are " + ("keyed apart: another family and another pitch" if keyed else
                                     "NOT keyed apart: one plug fits both (DF-170)"))
    row = bom_row("Li-ion cell")
    asked = [f for f in FAMILY_FP if f in row] or ["no family named"]
    print(f"bom.md asks the cell's seller for: {', '.join(asked)} (an order instruction, not the part)")
    if not os.path.exists(CELL_SPEC):
        print(f"the cell's plug as bought: no record ({os.path.relpath(CELL_SPEC, ROOT)} is written from the cell's "
              "spec sheet at VER-37)")
        print("RESULT: BLOCKED (the cell's spec sheet, VER-37)")
        return 2 if keyed else 1
    spec = json.load(open(CELL_SPEC, encoding="utf-8"))
    fam, pitch = spec.get("plug_family"), spec.get("plug_pitch_mm")
    print(f"the cell's plug as bought: {fam}, {pitch} mm pitch ({spec.get('source', 'no source')})")
    ok = keyed and (fam, pitch) == (c_fam, c_p) and (fam, pitch) != (s_fam, s_p)
    return result(ok)


def speaker_hole(b, pcbnew):
    """(x, y) of the speaker's hole: the smaller hole in the board's outline (the cell's is the larger)."""
    mm = pcbnew.ToMM
    outl = pcbnew.SHAPE_POLY_SET()
    b.GetBoardPolygonOutlines(outl, False)
    holes = []
    for i in range(outl.OutlineCount()):
        for k in range(outl.HoleCount(i)):
            ring = outl.Hole(i, k)
            xs = [mm(ring.CPoint(n).x) for n in range(ring.PointCount())]
            ys = [mm(ring.CPoint(n).y) for n in range(ring.PointCount())]
            holes.append(((max(xs) - min(xs)) * (max(ys) - min(ys)), ((max(xs) + min(xs)) / 2, (max(ys) + min(ys)) / 2)))
    return min(holes)[1] if holes else None


def spkplug():
    """VER-202 (DF-183, DEF-64, F-R2): the speaker's plug fits its header and cannot enter the cell's, and its leads
    reach its header but not the cell's. The plug and the lead length come from bom.md's speaker row (the part as
    sold: '<n> mm leads with the <p> mm 2-pin plug'); a re-termination counts only once the row no longer says it is
    not yet ordered. The places are the routed board's (pcbnew): the speaker's hole centre and each header's."""
    parts, _ = design()
    c_ref, c_fam, c_p = header(parts, "CELL_P")
    s_ref, s_fam, s_p = header(parts, "SPK_P")
    row = bom_row("Speaker")
    lead = re.search(r"(\d+(?:\.\d+)?) mm leads", row)
    sold = re.search(r"(\d\.\d+) mm 2-pin plug", row)
    refit = re.search(r"(\d\.\d+) mm (SH) plug", row)
    pending = bool(re.search(r"not yet ordered", row, re.I))
    lead = float(lead.group(1)) if lead else None
    print(f"bom.md's speaker row: leads {lead} mm, sold with a {sold.group(1) if sold else '?'} mm plug; "
          + (f"to be refitted with a {refit.group(1)} mm SH plug, " + ("not yet ordered" if pending else "ordered")
             if refit else "no refit named"))
    if refit and not pending:
        fam, pitch = "JST SH", float(refit.group(1))
    else:
        fam, pitch = None, float(sold.group(1)) if sold else None
    print(f"the speaker's plug at assembly: {fam or 'the plug it is sold with'}, {pitch} mm pitch; its header {s_ref} "
          f"is {s_fam}, {s_p} mm; the cell's {c_ref} is {c_fam}, {c_p} mm")
    fits = pitch is not None and abs(pitch - (s_p or 0)) < 0.01 and fam == s_fam
    enters_cell = pitch is not None and abs(pitch - (c_p or 0)) < 0.01 and fam in (c_fam, None)
    print("the speaker's plug " + ("fits" if fits else "does NOT fit") + f" {s_ref}; it "
          + ("could enter" if enters_cell else "cannot enter") + f" {c_ref}")
    pcbnew, b = board()
    mm = pcbnew.ToMM
    fps = {f.GetReference(): f for f in b.GetFootprints()}
    spk = speaker_hole(b, pcbnew)
    reach = {}
    for r in (s_ref, c_ref):
        p = fps[r].GetPosition()
        reach[r] = math.dist(spk, (mm(p.x), mm(p.y)))
    print(f"the speaker's hole centre ({spk[0]:.2f}, {spk[1]:.2f}): {reach[s_ref]:.1f} mm to {s_ref}, "
          f"{reach[c_ref]:.1f} mm to {c_ref}, against {lead} mm leads")
    reaches = lead is not None and reach[s_ref] <= lead
    short = lead is not None and reach[c_ref] > lead
    print(f"the leads {'reach' if reaches else 'do NOT reach'} {s_ref}; they {'cannot' if short else 'CAN'} reach {c_ref}")
    return result(fits and not enters_cell and reaches and short)


CURRENT = {   # net: (the worst current it carries in A, where the number comes from); 0.25 A or more needs a class
    "VBUS": (0.75, "VER-13: J1 to U2 IN at the charge current"),
    "CELL_P": (0.75, "VER-13: J2 to JP5"), "VBAT": (0.75, "VER-13: JP5 to U2 BAT"),
    "VSYS": (0.98, "VER-31: U3 0.36 A with Wi-Fi on, LED_A 0.12 A and AMP_VDD 0.5 A held (39cc07f); 1.24 A in a TX burst"),
    "AMP_VDD": (0.5, "VER-13: the amplifier, a stand-in (fm8002a.pdf prints no supply current at full power)"),
    "SPK_P": (0.35, "0.5 W into 8 ohm, the peak"), "SPK_N": (0.35, "0.5 W into 8 ohm, the peak"),
    "+3V3": (0.62, "VER-31: the 3V3 load in a Wi-Fi TX burst (39cc07f)"), "3V3_LDO": (0.62, "as +3V3"),
    "LED_A": (0.12, "the panel's 117.7 mA worst case, VER-137"),
    "SD_VDD": (0.10, "a microSD card writing (VER-31's stand-in)"),
}
CLASS_AMPS = 0.25


def netclass():
    """VER-201 (O-R8, DEF-68): build_pcb.py's net classes against the routed board's nets. Every net a class names
    is a net of battler.kicad_pcb (a stale name never matches and routes nothing), and every net CURRENT gives
    CLASS_AMPS or more sits in a class: a net in none routes at the default 0.15 mm. Read with Python's ast and a
    text scan; nothing is imported or written."""
    import ast
    src = open(os.path.join(BOARD, "tools", "build_pcb.py"), encoding="utf-8").read()
    classes, default = {}, None
    for node in ast.walk(ast.parse(src)):
        if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == "CLASSES" for t in node.targets):
            for k, v in zip(node.value.keys, node.value.values):
                kw = {x.arg: ast.literal_eval(x.value) for x in v.keywords}
                classes[ast.literal_eval(k)] = kw
        if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == "DEFAULT_CLASS" for t in node.targets):
            default = {x.arg: ast.literal_eval(x.value) for x in node.value.keywords}
    nets = set(re.findall(r'\(net "([^"]*)"\)', open(os.path.join(BOARD, "battler.kicad_pcb"), encoding="utf-8").read()))
    print(f"build_pcb.py: {len(classes)} classes ({', '.join(f'{k} {c.get(chr(116) + 'rack')} mm' for k, c in classes.items())}); "
          f"default {default.get('track') if default else '?'} mm; the board has {len(nets)} nets")
    bad = []
    for name, c in classes.items():
        for n in c.get("nets", []):
            if n not in nets:
                bad.append(f"{name} names {n}, which is no net of the board")
    for net, (amps, why) in CURRENT.items():
        full = next((n for n in nets if n.split("/")[-1] == net), None)
        cls = next((k for k, c in classes.items() if full in c.get("nets", [])), None)
        print(f"  {net} ({full}): {amps:.2f} A ({why}): " + (f"class {cls}" if cls else "no class, the default"))
        if full is None:
            bad.append(f"{net} is no net of the board")
        elif amps >= CLASS_AMPS and not cls:
            bad.append(f"{full} carries {amps:.2f} A in no class")
        elif cls:
            # 39cc07f (carry-over P4): the class's width must carry the net's current by IPC-2221's external chart (10 C)
            import paper_checks as pc
            w = classes[cls].get("track") or 0.0
            cap = pc.ipc2221_amps(w)
            print(f"    class {cls} {w} mm carries {cap:.2f} A by IPC-2221 for 10 C (paper_checks.ipc2221_amps): "
                  + ("enough" if cap >= amps else "NOT enough"))
            if cap < amps:
                bad.append(f"{full} carries {amps:.2f} A on class {cls}'s {w} mm, which carries {cap:.2f} A")
    print(f"findings: {len(bad)}" + ("".join(f"\n  {x}" for x in bad)))
    return result(not bad)


def ntc(t_c, r25, beta):
    return r25 * math.exp(beta * (1 / (t_c + 273.15) - 1 / 298.15))


def tsband():
    """The TS voltage a good unit shows at the measured ambient: I_TS into TH1, with I_TS's tolerance, TH1's 1%
    R25 and B, and the ambient reading +-2 C (the meter and the board's own warmth)."""
    TH1_R25, TH1_B, mpn = th1_spec()
    amb = [float(a) for a in sys.argv[2:]] or [18.0, 20.0, 22.0, 25.0, 28.0]
    print(f"TS = I_TS x TH1: I_TS {I_TS * 1e6:g} uA +-{I_TS_TOL:.0%} (SLUS810N's INTC, 72-78 uA), TH1 {TH1_R25 / 1e3:g}k "
          f"B{TH1_B:g} +-{TH1_TOL:.0%} ({mpn}, design.py), the ambient +-2 C")
    for t in amb:
        lo = I_TS * (1 - I_TS_TOL) * ntc(t + 2, TH1_R25 * (1 - TH1_TOL), TH1_B * (1 + TH1_TOL))
        hi = I_TS * (1 + I_TS_TOL) * ntc(t - 2, TH1_R25 * (1 + TH1_TOL), TH1_B * (1 - TH1_TOL))
        print(f"  ambient {t:.1f} C: TS {lo:.3f}-{hi:.3f} V (nominal {I_TS * ntc(t, TH1_R25, TH1_B):.3f} V)")
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


def charge_window():
    """(cold C, hot C): the charge window TH1 gives at the worst corner of U2's TS limits and TH1's tolerance
    (tempwin's last row; VER-160 compares it with the cell's own charge range)."""
    def temp_at(r, beta, r25):
        return 1 / (1 / 298.15 + math.log(r / r25) / beta) - 273.15
    r25, beta, _ = th1_spec()
    i_lo, i_hi = I_TS * (1 - I_TS_TOL), I_TS * (1 + I_TS_TOL)
    return (temp_at(V_COLD[1] / i_lo, beta * (1 - TH1_B_TOL), r25 * (1 - TH1_TOL)),
            temp_at(V_HOT[0] / i_hi, beta * (1 - TH1_B_TOL), r25 * (1 + TH1_TOL)))


def cellspec():
    """VER-160 (O-S6, carry-over C3 and C4): the cell's spec sheet, recorded by VER-37 in hardware/vv/analysis/cell.json,
    against the three numbers the board and the case freeze: the charge current R5 sets (KISET / R5 at its worst, under
    half the capacity in C and under the standard charge current), the charge window TH1 gives (tempwin's worst corner,
    inside the cell's charge range) and the cell case.scad's pocket is drawn for (its nominal and largest size equal the
    sheet's). cell.json's keys: model, source, capacity_mah, std_charge_ma, charge_temp_c [lo, hi], size_mm [l, w, t]
    (nominal) and size_max_mm [l, w, t]. BLOCKED (exit 2) until VER-37 writes it."""
    parts, _ = design()
    r5 = ohms(parts["R5"]["value"])
    ichg_hi = KISET[1] / (r5 * (1 - R_TOL))
    cold, hot = charge_window()
    e = scad()
    print(f"R5 {r5:g} 1%: ICHG up to {ichg_hi * 1e3:.0f} mA (KISET {KISET[1]:g} A-ohm, bq24074.txt:785); TH1's charge window "
          f"{cold:.1f} C to {hot:.1f} C at the worst corner (tempwin); case.scad's cell: nominal "
          + " x ".join(f"{v:g}" for v in e["cell_nom"]) + " mm, largest " + " x ".join(f"{v:g}" for v in e["cell"]) + " mm")
    if not os.path.exists(CELL_SPEC):
        print(f"BLOCKED: {os.path.relpath(CELL_SPEC, ROOT)} does not exist: VER-37 writes it from the cell's spec sheet "
              "(the 403450 seller's, or a cell sold with one: DEC-23)")
        print("RESULT: BLOCKED")
        return 2
    spec = json.load(open(CELL_SPEC, encoding="utf-8"))
    need = ("model", "source", "capacity_mah", "std_charge_ma", "charge_temp_c", "size_mm", "size_max_mm")
    miss = [k for k in need if k not in spec]
    if miss:
        print(f"cell.json lacks {', '.join(miss)}")
        return result(False)
    print(f"the cell: {spec['model']} ({spec['source']}): {spec['capacity_mah']} mAh, standard charge "
          f"{spec['std_charge_ma']} mA, charge {spec['charge_temp_c'][0]}-{spec['charge_temp_c'][1]} C, "
          + " x ".join(str(v) for v in spec["size_mm"]) + " mm, at most "
          + " x ".join(str(v) for v in spec["size_max_mm"]) + " mm")
    lim = min(0.5 * spec["capacity_mah"], spec["std_charge_ma"]) / 1e3
    rows = [(ichg_hi <= lim, f"ICHG {ichg_hi * 1e3:.0f} mA against min(0.5 C, the standard charge current) {lim * 1e3:.0f} mA"),
            (spec["charge_temp_c"][0] <= cold and hot <= spec["charge_temp_c"][1],
             f"TH1's window {cold:.1f}-{hot:.1f} C inside the cell's {spec['charge_temp_c'][0]}-{spec['charge_temp_c'][1]} C"),
            (all(abs(a - b) < 1e-6 for a, b in zip(e["cell_nom"], spec["size_mm"])),
             f"case.scad's nominal cell {e['cell_nom']} equals the sheet's {spec['size_mm']}"),
            (all(abs(a - b) < 1e-6 for a, b in zip(e["cell"], spec["size_max_mm"])),
             f"case.scad's largest cell {e['cell']} equals the sheet's {spec['size_max_mm']}")]
    for good, txt in rows:
        print(("  ok    " if good else "  FAIL  ") + txt)
    return result(all(g for g, _ in rows))


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

    TH1_R25, TH1_B, mpn = th1_spec()
    i_lo, i_hi = I_TS * (1 - I_TS_TOL), I_TS * (1 + I_TS_TOL)
    hot_nom_r, cold_nom_r = sum(V_HOT) / 2 / I_TS, sum(V_COLD) / 2 / I_TS
    hot_r, cold_r = V_HOT[0] / i_hi, V_COLD[1] / i_lo
    r25_hot, r25_cold = TH1_R25 * (1 + TH1_TOL), TH1_R25 * (1 - TH1_TOL)
    print(f"SLUS810N: V_HOT {V_HOT[0]}-{V_HOT[1]} V, V_COLD {V_COLD[0]}-{V_COLD[1]} V, INTC {i_lo * 1e6:.0f}-"
          f"{i_hi * 1e6:.0f} uA (bq24074.txt:823-826); TH1 {TH1_R25 / 1e3:g}k +-{TH1_TOL:.0%}, B{TH1_B:g} "
          f"+-{TH1_B_TOL:.0%} ({mpn}, design.py)")
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
    for b in sorted({3950.0, 4250.0, 4350.0, 4460.0} - {TH1_B}):
        print(f"  B{b:g} 1%: the hot edge {temp_at(hot_r, b * (1 - TH1_B_TOL), r25_hot):.1f} C at the worst corner "
              f"({temp_at(hot_nom_r, b, TH1_R25):.1f} C nominal)")
    ok = hot <= 45.0 and cold >= 0.0
    print(f"REQ-BAT-03 (0-45 C, or the cell's own limits once VER-37 has its spec sheet): the window is {cold:.1f} C "
          f"to {hot:.1f} C at the worst corner: " + ("met" if ok else "NOT met"))
    return result(ok)


CHECKS = {"holes": holes, "swell": swell, "inrush": inrush, "backlight": backlight, "cutoff": cutoff, "screws": screws,
          "mlcc": mlcc, "th1": th1, "cellgap": cellgap, "cpl": cpl, "drill": drill, "vbus": vbus, "bom": bom,
          "edge": edge, "plugs": plugs, "spkplug": spkplug, "netclass": netclass, "tsband": tsband, "cvload": cvload, "tempwin": tempwin,
          "cellspec": cellspec}


def main():
    if len(sys.argv) < 2 or sys.argv[1] not in CHECKS or (len(sys.argv) > 2 and sys.argv[1] != "tsband"):
        print(__doc__.split("\n\n")[2] + "\n\n" + __doc__.split("\n\n")[3] if "--list" in sys.argv else __doc__)
        return 0 if "--list" in sys.argv else 2
    return CHECKS[sys.argv[1]]()


if __name__ == "__main__":
    sys.exit(main())
