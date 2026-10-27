#!/usr/bin/env python3
"""Paper checks of classes 4 (values and worst case) and 6 (states and sequencing), from the design files.

    python3 hardware/vv/tools/paper_checks.py <check>           # exit 1 when the check's limit fails
    python3 hardware/vv/tools/paper_checks.py <check> --seed    # the same check on a seeded fault: must exit 1
    python3 hardware/vv/tools/paper_checks.py --list

  latch     VER-33   both Start paths into U3's EN: the sure-on, sure-off and ghost-press bands, E9 after a reset
  usb       VER-191  R21/R22 at their corners against the RP2350's 27 ohm; D+ and D- capacitance to ground
  audio     VER-100  the PWM filter, Ci/Ri and the bridge gain at every tolerance corner: band edges and the carrier
  core      VER-197  the copied core's values against rp2350.pdf's own rules (VREG, DVDD decoupling, crystal, AVDD filter)
  shelf     VER-103  the off-state budget with the cell in, and the months from the cutoff to the PCM's UVP
  ampstate  VER-196  U7 shut down in every state but the firmware's AMP_EN high
  hotplug   VER-12   a 2 m cable plugged live into R41/C1 and U2's IN (an ODE): VBUS, EN1 and PWR_EN peaks
  mating    VER-140  the cell plugged into J2: the peak into the VBAT and VSYS capacitors, and PWR_EN through the edge
  mlccbias  VER-30   each MLCC's bias against its part's minimum: the derating the curves must show

Each check reads design.py (through design_numbers.design()) and, for 'usb' and 'core', the routed board
(pcbnew). A datasheet number design.py does not hold is a named constant below with its file and line.
rp2350.txt and bq24074.txt are pdftotext -layout copies of the PDFs in hardware/ref_design/datasheets
(tools/datasheet_hashes.py records their sha256); rt9013.pdf is cited by page. A number from a
datasheet that is not on disk is taken from board/pin_review/<part>.json (which quotes the page) and is
marked 'not on disk': VER-34 confirms it. A stand-in is a number no file gives, chosen pessimistic, and
says so where it is printed.

--seed applies the check's SEED (a design value moved to where the check must fail) to the parts in
memory, so each check is shown failing once (README: a check that can be a script can fail).

Python 3 standard library, plus pcbnew for 'usb' and 'core'.
"""
import cmath
import itertools
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import design_numbers as dn  # noqa: E402

# ---- datasheet numbers ----------------------------------------------------------------------------------
# RT9013 (U3), DS9013-10, rt9013.pdf
U3_VIH, U3_VIL = 1.6, 0.6        # V: EN logic high minimum, logic low maximum (p4)
U3_IEN = 1e-6                    # A: EN pin current, maximum (p4)
U3_ISHDN = 1.5e-6                # A: shutdown current at VEN 0 V, maximum, 25 C (p3)
U3_EN_ABS = 6.0                  # V: EN absolute maximum (p3)
# BQ24073 (U2), SLUS810N, bq24074.txt
U2_IBAT_PDWN = 6.5e-6            # A: sleep current into BAT, input not detected, TJ 85 C, maximum (bq24074.txt:714)
U2_OVP_MIN = 6.4                 # V: input OVP threshold, minimum (bq24074.txt:699)
U2_EN_ABS = 7.0                  # V: EN1, EN2, CE ... absolute maximum (bq24074.txt:598-599)
U2_EN_PD = 285e3                 # ohm: EN1's internal pull-down, approximately (bq24074.txt:512)
U2_VO_REG = (4.3, 4.5)           # V: OUT regulation with input (bq24074.txt:729)
U2_BAT_OUT_R = 0.1               # ohm: VBAT - VOUT 100 mV max at 1 A, VIN 0 (bq24074.txt:724)
# RP2350 (U4), rp2350.txt
RP_IIN = 1e-6                    # A: GPIO input leakage, maximum (rp2350.txt:101100)
RP_VOH = 2.62                    # V: GPIO VOH minimum at IOVDD 3.3 V and the set drive (rp2350.txt:101144)
RP_E9_PULL = 8.2e3               # ohm: RP2350-E9, a pull this low or lower holds a pad low (rp2350.txt:102915)
RP_E9_V = 2.2                    # V: E9's leakage holds an undriven pad near 2.2 V (rp2350.txt:102909)
RP_USB_R = 27.0                  # ohm: 'A 27 ohm series termination resistor is required on each pin' (rp2350.txt:1196)
RP_XTAL_CL = 10e-12              # F: ABM8-272-T3 load capacitance, the recommended crystal (rp2350.txt:40927, 40950)
RP_VREG_C = 4.7e-6               # F: VREG CIN at least 4.7 uF, COUT 4.7 uF +-20% (rp2350.txt:33598, 33603)
RP_VREG_L = 3.3e-6               # H: 3.3 uH +-20%, the AOTA-B201610S3R3-101-T recommended (rp2350.txt:33608-33612)
RP_AVDD_RC = (33.0, 4.7e-6)      # VREG_AVDD's RC filter: 33 ohm and 4.7 uF (Figure 19, rp2350.txt:33281-33290)
RP_PWM_CLK = 150e6               # Hz: clk_sys at the SDK default; AUDIO_PWM at 8 bits is clk_sys / 256 (design.py's 586 kHz)
# Parts whose datasheets are not on disk (pin_review quotes the page; VER-34 confirms)
Q_VTH = (1.0, 2.5)               # V: 2N7002 VGS(th) min, max (q1.json 'VGS(th) 1..2.5 V', C8545 p2; not on disk)
VF_01MA, VF_DECADE = 0.240, 0.080  # V: BAT54C 240 mV max at 0.1 mA (d2.json p3); BAT54WS 240 mV at 0.1 mA, 320 mV at 1 mA (d3.json p2)
U1_C = 5e-12                     # F: SRV05-4 I/O to GND, maximum (u1.json p2; srv05-4.pdf not on disk)
U7_ISD = 4.2e-6                  # A: FM8002A shutdown current, typical only (u7.json p3; fm8002a.pdf not on disk)
# Stand-ins: no file gives these; each is pessimistic and printed as a stand-in
STAND_U7_ISD = 2 * U7_ISD        # A: twice the FM8002A's typical, as no maximum is printed
STAND_IDSS = 1e-6                # A: a 2N7002 or AO3400A off at a few volts (the usual 1 uA IDSS at rated VDS)
STAND_CLAMP_V = dn.CLAMP_V       # V: U10's unpowered input clamp (design_numbers.py, the cutoff check)
STAND_RP_USB_C = 10e-12          # F: the RP2350's USB pad (rp2350.pdf gives none)
STAND_TRACK_C = 0.1e-12          # F/mm: a track over the B.Cu pour on 1.6 mm FR4, about twice the usual 0.05 pF/mm
STAND_XTAL_STRAY = (2e-12, 5e-12)  # F: board and pin stray at XIN/XOUT
STAND_CELL_MAH = 700.0           # mAh: REQ-PRD-03.R's 'about 700 mAh' until VER-37 reads the cell
STAND_LOW_SHARE = (0.05, 0.10)   # share of the capacity between 3.50 V and the PCM's UVP (REQ-BAT-10's 5-10%)
STAND_PCM = (3e-6, 6e-6)         # A: a DW01-class PCM's own drain (REQ-BAT-10), until VER-37
STAND_SELF = (0.01, 0.03)        # share a month: the cell's self-discharge at 25 C (REQ-BAT-10), doubled at 45 C
STAND_CABLE = dict(r=(0.13, 0.85), l=(1.0e-6, 2.0e-6))  # ohm and H, 2 m loop: 20-28 AWG at 0.033-0.213 ohm/m a
                                                       # conductor; 0.5-1.0 uH/m of loop inductance
STAND_C1_EFF = (0.3, 1.0)        # share of C1's 10 uF left at 5 V (no DC-bias curve on disk: VER-30)
CLOSE_MM = 5.0                   # mm: this check's reading of the datasheet's 'close to the pin', pad centre to pad centre
R_TOL = dn.R_TOL                 # every resistor in design.py's RES table is 1%
C_TOL = {"K": 0.10, "M": 0.20, "J": 0.05}   # MPN tolerance letter: Samsung CLxx and FH 0402 codes


def seeded(parts, seed):
    for ref, val in seed.items():
        parts[ref] = dict(parts[ref], value=val)
    return parts


def load(seed):
    parts, ns = dn.design()
    return (seeded(parts, seed) if seed else parts), ns


def r(parts, ref):
    return dn.ohms(parts[ref]["value"])


def c(parts, ref):
    return dn.farads(parts[ref]["value"])


def ctol(parts, ref):
    mpn = parts[ref]["mpn"] or ""
    if mpn.startswith("CL"):                     # Samsung: CL05B104KO5..., tolerance after the value code
        return C_TOL[mpn[8]]
    if mpn.startswith("0402"):                   # FH: 0402CG150J500NT, 0402B472K500NT
        return C_TOL[mpn.rstrip("NT")[-4]]
    return 0.2


def vf_max(i):
    """Schottky forward voltage, the datasheets' maximum line (240 mV at 0.1 mA, 80 mV a decade)."""
    return VF_01MA + VF_DECADE * math.log10(max(i, 1e-9) / 1e-4)


def bisect(f, lo, hi, n=80):
    flo = f(lo)
    for _ in range(n):
        mid = (lo + hi) / 2
        if (f(mid) > 0) == (flo > 0):
            lo, flo = mid, f(mid)
        else:
            hi = mid
    return (lo + hi) / 2


# ---- VER-33 -----------------------------------------------------------------------------------------------
SEED_LATCH = {"R42": "47k"}          # PWR_HOLD's pull-down weaker than E9 allows


def latch_point(vsys, rs, rpd, ren, vf_on, ien):
    """The Start contact (rs) from VSYS into the RAW node (rpd to GND, a Schottky into PWR_EN, ren to GND)."""
    def diode_i(v):
        if v <= 0:
            return 0.0
        if not vf_on:
            return v / ren + ien
        if v <= vf_max(ien):
            return 0.0
        return bisect(lambda i: v - vf_max(i) - max(i - ien, 0.0) * ren, ien, v / ren + ien)

    raw = bisect(lambda v: (vsys - v) / rs - v / rpd - diode_i(v), 0, vsys)
    i = diode_i(raw)
    en = max(0.0, (i - ien) * ren) if vf_on else raw
    return raw, en


def latch(seed=None):
    parts, _ = load(seed)
    lo, hi = 1 - R_TOL, 1 + R_TOL
    r15 = r(parts, "R15")
    ok = True
    print(f"U3 RT9013 EN: VIH {U3_VIH} V, VIL {U3_VIL} V, IEN {U3_IEN * 1e6:.0f} uA max (rt9013.pdf p4); "
          f"Q1/Q7 2N7002 VGS(th) {Q_VTH[0]}-{Q_VTH[1]} V (q1.json p2, not on disk)")
    print(f"Schottky VF max {VF_01MA * 1e3:.0f} mV at 0.1 mA, {VF_DECADE * 1e3:.0f} mV a decade (d2.json p3, d3.json p2); "
          "for the off cases VF is taken as 0, the worst there")
    for seat, (pd, dio, q) in (("seat 1 (SW21)", ("R13", "D2", "Q1")), ("seat 2 (SW23)", ("R62", "D8", "Q7"))):
        rpd = r(parts, pd)
        raw, en = latch_point(3.42, 1e3, rpd * lo, r15 * lo, True, U3_IEN)
        on = en >= U3_VIH and raw >= Q_VTH[1]
        ok &= on
        print(f"{seat}: 1 kohm in series at VSYS 3.42 V: {q}'s gate {raw:.3f} V ({raw - Q_VTH[1]:+.3f} V over its 2.5 V "
              f"maximum threshold), EN {en:.3f} V ({en - U3_VIH:+.3f} V over VIH): {'on' if on else 'NOT SURE ON'}")
        raw, en = latch_point(4.2, 1e6, rpd * hi, r15 * hi, False, 0)
        off = en <= U3_VIL and raw < Q_VTH[0]
        ok &= off
        print(f"{seat}: 1 Mohm across the dome at 4.2 V: RAW {raw:.3f} V, EN at most {en:.3f} V (VIL {U3_VIL}); "
              f"{q}'s gate {raw:.3f} V against its 1.0 V minimum threshold: {'off' if off else 'NOT SURE OFF'}")
        # the bands
        on_max = bisect(lambda rs: min(latch_point(3.42, rs, rpd * lo, r15 * lo, True, U3_IEN)[1] - U3_VIH,
                                       latch_point(3.42, rs, rpd * lo, r15 * lo, True, U3_IEN)[0] - Q_VTH[1]), 1, 1e6)
        ghost = bisect(lambda rs: latch_point(4.2, rs, rpd * hi, r15 * hi, False, 0)[0] - Q_VTH[0], 1, 1e8)
        offmin = bisect(lambda rs: latch_point(4.2, rs, rpd * hi, r15 * hi, False, 0)[1] - U3_VIL, 1, 1e8)
        print(f"  bands: sure on (EN and {q} both) up to {on_max / 1e3:.1f} kohm at VSYS 3.42 V; {q} may read a ghost "
              f"press under {ghost / 1e3:.0f} kohm of leakage at 4.2 V; EN may turn on under {offmin / 1e3:.0f} kohm; "
              f"sure off above {max(ghost, offmin) / 1e3:.0f} kohm (1 Mohm is {'above' if 1e6 > max(ghost, offmin) else 'NOT above'} it)")
        ok &= 1e6 > max(ghost, offmin)
    r42 = r(parts, "R42")
    e9 = r42 <= RP_E9_PULL
    ok &= e9
    print(f"after RESET or a crash with E9 (A2 stepping): R42 {parts['R42']['value']} against E9's 8.2 kohm "
          f"(rp2350.txt:102915): {'PWR_HOLD pulled low, EN 0 V' if e9 else 'E9 can hold PWR_HOLD near 2.2 V: EN stays up'}")
    hold = latch_point(RP_VOH, 1e-3, r42 * lo, r15 * lo, True, U3_IEN)[1]
    ok &= hold >= U3_VIH
    print(f"held by firmware (PWR_HOLD at the GPIO's {RP_VOH} V VOH minimum, rp2350.txt:101144): EN {hold:.3f} V "
          f"({hold - U3_VIH:+.3f} V over VIH); GPIO44 sources {RP_VOH / (r42 * lo) * 1e3:.2f} mA into R42")
    r14 = r(parts, "R14")
    for vbus in (4.75, 5.25):
        v = latch_point(vbus, r14 * hi, 1e12, r15 * lo, True, U3_IEN)[1]
        print(f"USB at {vbus} V: PWR_EN {v:.2f} V through R14 {parts['R14']['value']} and D3 (turns the device on: {v >= U3_VIH})")
    return dn.result(ok)


# ---- VER-196 -----------------------------------------------------------------------------------------------
SEED_AMP = {"R37": "1M"}             # AMP_EN's pull-down so weak the pad's leakage nears Q3's threshold


def ampstate(seed=None):
    parts, _ = load(seed)
    r36, r37 = r(parts, "R36") * (1 + R_TOL), r(parts, "R37") * (1 + R_TOL)
    gate = RP_IIN * r37
    margin = Q_VTH[0] - gate
    ok = margin >= 0.4
    print(f"Q3 2N7002 VGS(th) {Q_VTH[0]}-{Q_VTH[1]} V (q1.json p2, not on disk); AMP_EN held by R37 {parts['R37']['value']} "
          f"(+1%) against the pad's {RP_IIN * 1e6:.0f} uA leakage maximum (rp2350.txt:101100): gate at most {gate:.2f} V, "
          f"{margin:.2f} V under the minimum threshold (limit 0.4 V)")
    states = [
        ("off, VSYS live", "U4 unpowered, its pad passes no current; R37 holds the gate at 0 V"),
        ("power-up before firmware", "pads reset with IE clear, pull-down on (PADS_BANK0 GPIO reset: IE 0, PDE 1, ISO 1, "
         "rp2350.txt:59412-59433); E9 does not act after a PoR or RUN reset (rp2350.txt:102952)"),
        ("RESET (RUN)", "a RUN reset resets the pads as at power-up (rp2350.txt:102952)"),
        ("watchdog reset", "the pads reset only if the watchdog's PSM WDSEL includes RESETS (rp2350.txt:36739-36791, all "
         "0 at reset): the SDK's watchdog_enable() sets it; firmware rule: keep that, or E9 (A2) can hold AMP_EN near "
         f"{RP_E9_V} V behind {parts['R37']['value']}, over Q3's {Q_VTH[0]} V"),
        ("sleep or dormant", "outputs keep their level; firmware rule: drive AMP_EN low before sleep, never leave it an "
         "input (E9 on A2 needs 8.2 kohm or less, R37 is 100k)"),
        ("USB in, cell flat", "VSYS is U2's OUT, 4.3-4.5 V (bq24074.txt:729); AMP_EN as above"),
    ]
    for name, why in states:
        print(f"  {name}: AMP_EN <= {gate:.2f} V, Q3 off, U7 shut down: {why}")
    for vsys in (3.0, 4.4):
        shdn = vsys - STAND_IDSS * r36
        print(f"VSYS {vsys} V: AMP_SHDN at least {shdn:.2f} V (R36 {parts['R36']['value']} with Q3's off leakage, "
              f"stand-in {STAND_IDSS * 1e6:.0f} uA): within {vsys - shdn:.2f} V of VDD, the 'supply voltage' the "
              "FM8002A names for shutdown (u7.json pin 1; it prints no VIH: limit 95% of VDD)")
        ok &= shdn >= 0.95 * vsys
    off = STAND_U7_ISD + STAND_IDSS
    print(f"U7's shutdown {U7_ISD * 1e6:.1f} uA typical (u7.json p3, not on disk; {STAND_U7_ISD * 1e6:.1f} uA as a stand-in "
          f"maximum) and R36's {STAND_IDSS * 1e6:.0f} uA (Q3's leakage): {off * 1e6:.1f} uA of the off budget; "
          "the whole budget is 'shelf' (VER-103), against VER-73's 30 uA")
    total = off_budget(parts, 4.2, 25)[0]
    ok &= total <= 30e-6
    print(f"the whole off budget at 4.2 V, 25 C: {total * 1e6:.1f} uA (limit 30 uA)")
    return dn.result(ok)


# ---- VER-103 ------------------------------------------------------------------------------------------------
def off_budget(parts, vbat, temp_c):
    """The cell's current with the device off and no USB, part by part (A), and the lines printed."""
    warm = 2.0 if temp_c > 30 else 1.0
    r17, r18 = r(parts, "R17"), r(parts, "R18")
    vdiv = min(vbat * r18 / (r17 + r18), STAND_CLAMP_V)
    lines = [
        ("U2 sleep current into BAT", U2_IBAT_PDWN, "6.5 uA max at TJ 85 C (bq24074.txt:714): used at 25 and 45 C"),
        ("U3 shut down", U3_ISHDN * warm, f"1.5 uA max at 25 C (rt9013.pdf p3){'; doubled at 45 C (stand-in)' if warm > 1 else ''}"),
        ("R17 into U10's unpowered input", (vbat - vdiv) / r17,
         f"VBAT_DIV held at {vdiv:.2f} V by U10's input clamp (stand-in {STAND_CLAMP_V} V: MCP6001 not on disk)"),
        ("U7 shut down on VSYS", STAND_U7_ISD * warm, "FM8002A 4.2 uA typical (u7.json p3, not on disk): 2 x typical, stand-in"),
        ("Q3 off (R36 from VSYS)", STAND_IDSS * warm, "2N7002 IDSS, stand-in"),
        ("Q5, Q6 off (the backlight LEDs from VSYS)", 2 * STAND_IDSS * warm, "AO3400A IDSS, stand-in"),
    ]
    return sum(i for _, i, _ in lines), lines


SEED_SHELF = {"R17": "47k", "R18": "47k"}     # a stiff divider: 10 x the drain


def shelf(seed=None):
    parts, _ = load(seed)
    ok = True
    print("off, no USB, cell in: VSYS is live through U2's battery FET; 3V3, +1V1 and the panel are off")
    for temp in (25, 45):
        tot, lines = off_budget(parts, 4.2, temp)
        print(f"-- {temp} C, VBAT 4.2 V")
        for name, i, why in lines:
            print(f"  {name:44s} {i * 1e6:5.1f} uA  {why}")
        print(f"  board total {tot * 1e6:.1f} uA (REQ-PWR-07: 30 uA at 25 C, 60 uA at 45 C): "
              f"{'within' if tot <= (30e-6 if temp == 25 else 60e-6) else 'OVER'}")
        ok &= tot <= (30e-6 if temp == 25 else 60e-6)
        at35 = off_budget(parts, 3.5, temp)[0]
        warm = 2 if temp > 30 else 1
        for case, k in (("nominal", 0), ("worst", 1)):
            pcm = STAND_PCM[k]
            selfd = STAND_SELF[k] * warm * STAND_CELL_MAH
            low = STAND_CELL_MAH * STAND_LOW_SHARE[1 - k]
            month = 730.0                                         # h
            drain = (at35 + pcm) * 1e3 * month + selfd             # mAh a month from the cutoff
            m_cut = low / drain
            m_full = STAND_CELL_MAH * (1 - STAND_LOW_SHARE[1 - k]) / ((tot + pcm) * 1e3 * month + selfd) + m_cut
            print(f"  {case}: board {at35 * 1e6:.1f} uA at 3.5 V + PCM {pcm * 1e6:.0f} uA + self-discharge "
                  f"{STAND_SELF[k] * warm * 100:.0f}% a month = {drain:.1f} mAh a month; {low:.0f} mAh from 3.50 V to the "
                  f"UVP: {m_cut:.1f} months from the cutoff (limit 2), {m_full:.1f} months from full (limit 24)")
            if temp == 25 or case == "worst":
                ok &= m_cut >= 2 and (temp > 30 or m_full >= 24)
    print("stand-ins: capacity 700 mAh (REQ-PRD-03.R), 5-10% of it between 3.50 V and the UVP, a DW01-class PCM's "
          "3-6 uA and 1-3% a month of self-discharge (REQ-BAT-10), all until VER-37 reads the cell; the 45 C period "
          "is Alex's to set (REQ-BAT-10), so 45 C is printed and counted only at the worst case from the cutoff")
    return dn.result(ok)


# ---- VER-191 -------------------------------------------------------------------------------------------------
SEED_USB = {"R21": "22"}


def usb(seed=None):
    parts, _ = load(seed)
    pcb, b = dn.board()
    mm = pcb.ToMM
    ok = True
    for ref in ("R21", "R22"):
        v = r(parts, ref)
        lo, hi = v * (1 - R_TOL), v * (1 + R_TOL)
        good = lo <= RP_USB_R <= hi
        ok &= good
        print(f"{ref} {parts[ref]['value']} ohm 1%: {lo:.2f}-{hi:.2f} ohm against the 27 ohm rp2350.txt:1196 requires: "
              f"{'meets it' if good else 'NOT 27 ohm'}")
    print("ZDRV (USB 2.0's 28-44 ohm with the series resistor) also needs the PHY's own output impedance, which "
          "rp2350.pdf does not give: the 27 ohm is Raspberry Pi's figure for that sum (rp2350.txt:1196, 100753)")
    length = {}
    vias = {}
    for t in b.GetTracks():
        n = t.GetNetname().split("/")[-1]
        if n in ("USB_DP", "USB_DM", "USB_DP_MCU", "USB_DM_MCU"):
            if t.GetClass() == "PCB_TRACK":
                length[n] = length.get(n, 0) + mm(t.GetLength())
            else:
                vias[n] = vias.get(n, 0) + 1
    for line in ("DP", "DM"):
        mm_ = length.get(f"USB_{line}", 0) + length.get(f"USB_{line}_MCU", 0)
        nv = vias.get(f"USB_{line}", 0) + vias.get(f"USB_{line}_MCU", 0)
        ctot = U1_C + STAND_RP_USB_C + mm_ * STAND_TRACK_C + nv * 0.5e-12
        good = ctot <= 100e-12
        ok &= good
        print(f"D{'+' if line == 'DP' else '-'}: track {mm_:.1f} mm ({mm_ * STAND_TRACK_C * 1e12:.1f} pF at "
              f"{STAND_TRACK_C * 1e12:.1f} pF/mm, stand-in), {nv} vias (0.5 pF each), U1 {U1_C * 1e12:.0f} pF max "
              f"(u1.json p2, not on disk), the RP2350's pad {STAND_RP_USB_C * 1e12:.0f} pF (stand-in): {ctot * 1e12:.1f} pF "
              f"against 100 pF; it passes unless U1 and the pad together exceed {(100e-12 - mm_ * STAND_TRACK_C - nv * 0.5e-12) * 1e12:.0f} pF")
    mismatch = abs(length.get("USB_DP", 0) + length.get("USB_DP_MCU", 0) - length.get("USB_DM", 0) - length.get("USB_DM_MCU", 0))
    print(f"D+/D- length mismatch {mismatch:.2f} mm (VER-108 owns the limit)")
    return dn.result(ok)


# ---- VER-100 ------------------------------------------------------------------------------------------------
SEED_AUDIO = {"C37": "47n", "C38": "47n"}


def audio_h(f, r32, c37, r33, c38, c39, r34, r35, rsrc):
    """Bridge output over the PWM's swing: two RC stages, then Ci and Ri into the inverting input's virtual
    ground (the FM8002A's -IN; gain 2 x Rf / Ri, u7.json pin 4)."""
    w = 2 * math.pi * f
    zc = lambda cap: 1 / (1j * w * cap)                     # noqa: E731
    zin = zc(c39) + r34                                     # into the virtual ground
    z2 = 1 / (1 / zc(c38) + 1 / zin)                        # node AUDIO_F2
    zb = r33 + z2
    z1 = 1 / (1 / zc(c37) + 1 / zb)                         # node AUDIO_F1
    v1 = z1 / (z1 + r32 + rsrc)
    v2 = v1 * z2 / zb
    return 2 * r35 * v2 / zin


def audio(seed=None):
    parts, _ = load(seed)
    names = ["R32", "C37", "R33", "C38", "C39", "R34", "R35"]
    nom = [r(parts, n) if n[0] == "R" else c(parts, n) for n in names]
    tol = [R_TOL if n[0] == "R" else ctol(parts, n) for n in names]
    carrier = RP_PWM_CLK / 256
    worst = {"hi3db": 1e9, "lo3db": 0, "carrier": 0}
    for signs in itertools.product((-1, 1), repeat=len(names)):
        vals = [v * (1 + s * t) for v, s, t in zip(nom, signs, tol)]
        for rsrc in (0.0, 170.0):                 # the pad's own resistance: 0 to (3.3 - 2.62 V) / 4 mA (rp2350.txt:101144)
            h = lambda f: abs(audio_h(f, *vals, rsrc))   # noqa: E731
            mid = h(1e3)
            hi = bisect(lambda f: h(f) - mid / math.sqrt(2), 2e3, 2e5)
            lo = bisect(lambda f: h(f) - mid / math.sqrt(2), 1.0, 900.0)
            att = 20 * math.log10(mid / h(carrier))
            worst["hi3db"] = min(worst["hi3db"], hi)
            worst["lo3db"] = max(worst["lo3db"], lo)
            worst["carrier"] = min(worst["carrier"] or 1e9, att)
    h0 = lambda f: abs(audio_h(f, *nom, 0.0))    # noqa: E731
    print("parts: " + ", ".join(f"{n} {parts[n]['value']} {t * 100:.0f}%" for n, t in zip(names, tol)) +
          " (C tolerances from the MPNs' letter: K 10%, M 20%)")
    print(f"nominal: gain at 1 kHz {20 * math.log10(h0(1e3)):.1f} dB over the PWM's swing (2 x Rf/Ri, u7.json pin 4); "
          f"-3 dB at {bisect(lambda f: h0(f) - h0(1e3) / math.sqrt(2), 1.0, 900.0):.1f} Hz and "
          f"{bisect(lambda f: h0(f) - h0(1e3) / math.sqrt(2), 2e3, 2e5) / 1e3:.1f} kHz; the carrier at "
          f"{carrier / 1e3:.1f} kHz (clk_sys 150 MHz / 256) {20 * math.log10(h0(1e3) / h0(carrier)):.1f} dB down")
    print(f"every corner (2^7 tolerance corners x the pad's 0-170 ohm): high -3 dB at least {worst['hi3db'] / 1e3:.1f} kHz "
          f"(limit 10), low corner at most {worst['lo3db']:.1f} Hz (limit 50), carrier at least {worst['carrier']:.1f} dB "
          "down (limit 40)")
    def att_at(share):
        v = list(nom)
        v[1] *= share * (1 - tol[1])
        v[3] *= share * (1 - tol[3])
        return 20 * math.log10(abs(audio_h(1e3, *v, 0.0)) / abs(audio_h(carrier, *v, 0.0)))

    keep = bisect(lambda s: att_at(s) - 40, 0.05, 1.0)
    print(f"C37 and C38 are X7R (0402B472K, 50 V): DC bias at the PWM's 1.65 V mean lowers them (VER-30); the carrier "
          f"stays 40 dB down while they keep {keep * 100:.0f}% of their -10% value")
    print("not in this check: the FM8002A's own bandwidth and PSRR (fm8002a.pdf not on disk)")
    return dn.result(worst["hi3db"] >= 10e3 and worst["lo3db"] <= 50 and worst["carrier"] >= 40)


# ---- VER-197 -------------------------------------------------------------------------------------------------
SEED_CORE = {"C25": "1u"}


def core(seed=None):
    parts, _ = load(seed)
    pcb, b = dn.board()
    mm = pcb.ToMM
    ok = True

    def row(good, text):
        nonlocal ok
        ok &= good
        print(("  ok    " if good else "  FAIL  ") + text)

    print("the copied core against rp2350.pdf's own component rules (section 6.3.8.2, 6.1, 8.2.1.1):")
    row(c(parts, "C24") >= RP_VREG_C, f"C24 VREG_VIN {parts['C24']['value']}: CIN at least 4.7 uF (rp2350.txt:33598, 32715)")
    row(abs(c(parts, "C25") - RP_VREG_C) < 1e-9 and ctol(parts, "C25") <= 0.20,
        f"C25 VREG output {parts['C25']['value']} {parts['C25']['mpn']} (+-{ctol(parts, 'C25') * 100:.0f}%): COUT 4.7 uF +-20% "
        "(rp2350.txt:33603); its DC-bias loss at 1.1 V is VER-30's")
    row(abs(dn.farads(parts["L2"]["value"]) - RP_VREG_L) < 1e-9, f"L2 {parts['L2']['value']}H: 3.3 uH +-20% (rp2350.txt:33608)")
    row(parts["L2"]["mpn"] == "AOTA-B201610S3R3-101-T",
        f"L2 is {parts['L2']['mpn']}, the part rp2350.txt:33612 recommends (its DCR and Isat then meet 33608)")
    row(abs(r(parts, "R19") - RP_AVDD_RC[0]) < 1e-6 and abs(c(parts, "C26") - RP_AVDD_RC[1]) < 1e-9
        and parts["R19"]["pins"]["2"] == "VREG_AVDD" and "VREG_AVDD" in parts["C26"]["pins"].values(),
        f"VREG_AVDD filter R19 {parts['R19']['value']} ohm and C26 {parts['C26']['value']}: 33 ohm and 4.7 uF as Figure 19 "
        "(rp2350.txt:33281-33290)")
    row(parts["X1"]["mpn"] == "ABM8-272-T3", f"X1 {parts['X1']['mpn']}: the recommended crystal (rp2350.txt:40927)")
    cs = 1 / (1 / c(parts, "C27") + 1 / c(parts, "C28"))
    cl = [cs + s for s in STAND_XTAL_STRAY]
    row(all(abs(x - RP_XTAL_CL) <= 2.5e-12 for x in cl),
        f"X1's load: C27 {parts['C27']['value']} and C28 {parts['C28']['value']} in series {cs * 1e12:.1f} pF + "
        f"{STAND_XTAL_STRAY[0] * 1e12:.0f}-{STAND_XTAL_STRAY[1] * 1e12:.0f} pF stray (stand-in) = {cl[0] * 1e12:.1f}-"
        f"{cl[1] * 1e12:.1f} pF against CL 10 pF (rp2350.txt:40950), limit +-2.5 pF")
    row(abs(r(parts, "R21") - RP_USB_R) < 1e-6 and abs(r(parts, "R22") - RP_USB_R) < 1e-6,
        "R21, R22 27 ohm on USB_DP, USB_DM (rp2350.txt:1196)")
    # decoupling by distance on the routed board
    fps = {f.GetReference(): f for f in b.GetFootprints()}
    u4 = {p.GetNumber(): p for p in fps["U4"].Pads()}
    caps = {ref: f for ref, f in fps.items() if ref.startswith("C") and ref in parts and parts[ref]["bom"]}

    def pos(p):
        return mm(p.GetPosition().x), mm(p.GetPosition().y)

    def nearest(pin, net):
        px, py = pos(u4[pin])
        best = []
        for ref, f in caps.items():
            if net not in parts[ref]["pins"].values():
                continue
            pad = next(q for q in f.Pads() if q.GetNetname().split("/")[-1] == net)
            qx, qy = pos(pad)
            best.append((math.hypot(qx - px, qy - py), ref))
        return sorted(best)

    reg = pos(u4["63"])
    dv = sorted(("10", "32", "51"), key=lambda p: math.hypot(pos(u4[p])[0] - reg[0], pos(u4[p])[1] - reg[1]))
    print("  DVDD pins by distance from VREG_LX (pin 63): " + ", ".join(
        f"{p} ({math.hypot(pos(u4[p])[0] - reg[0], pos(u4[p])[1] - reg[1]):.1f} mm)" for p in dv))
    print(f"  'close to the pin' (rp2350.txt:32665, 32686, 32705) is read here as a capacitor pad within {CLOSE_MM} mm of "
          "the pin's pad, centre to centre; Raspberry Pi's own layout puts its 0402s 1-3 mm from the pins")
    for k, p in enumerate(dv):
        near = nearest(p, "+1V1")
        d, ref = near[0]
        want = 100e-9 if k < 2 else 4.7e-6
        big = [x for x in near if c(parts, x[1]) >= want * 0.99]
        dw, rw = big[0] if big else (1e9, "none")
        good = dw <= CLOSE_MM
        row(good, f"DVDD pin {p}: rp2350.txt:32665-32675 asks {'100 nF' if k < 2 else '4.7 uF (the pin furthest from the regulator)'}"
            f" close to it; the nearest +1V1 capacitor is {ref} {parts[ref]['value']} at {d:.1f} mm, the nearest of "
            f"{'100 nF' if k < 2 else '4.7 uF'} or more {rw} at {dw:.1f} mm")
    named = {"59": "ADC_AVDD", "68": "USB_OTP_VDD", "69": "QSPI_IOVDD"}
    for pin in sorted((q for q, n in parts["U4"]["pins"].items() if n == "+3V3" and q not in ("64",)), key=int):
        d, ref = nearest(pin, "+3V3")[0]
        name = named.get(pin, "IOVDD")
        row(d <= CLOSE_MM, f"{name} pin {pin}: nearest +3V3 capacitor {ref} {parts[ref]['value']} at {d:.1f} mm"
            + (" (100 nF close to the pin, rp2350.txt:32705)" if pin == "59" else
               " (100 nF close to the pin, rp2350.txt:32686)" if pin == "68" else ""))
    return dn.result(ok)


# ---- VER-12 --------------------------------------------------------------------------------------------------
SEED_HOTPLUG = {"R41": "0"}


def hotplug_peak(vs, rc, lc, r41, c1, dt=2e-9, t_end=60e-6):
    """Series cable (rc, lc) from a 5.25 V step into R41 + C1 (U2's IN is the node after the cable). RK4."""
    def f(s):
        i, vc = s
        vin = vc + i * r41
        return ((vs - rc * i - vin) / lc, i / c1)

    s = (0.0, 0.0)
    peak = 0.0
    for _ in range(int(t_end / dt)):
        k1 = f(s)
        k2 = f((s[0] + dt / 2 * k1[0], s[1] + dt / 2 * k1[1]))
        k3 = f((s[0] + dt / 2 * k2[0], s[1] + dt / 2 * k2[1]))
        k4 = f((s[0] + dt * k3[0], s[1] + dt * k3[1]))
        s = (s[0] + dt / 6 * (k1[0] + 2 * k2[0] + 2 * k3[0] + k4[0]), s[1] + dt / 6 * (k1[1] + 2 * k2[1] + 2 * k3[1] + k4[1]))
        peak = max(peak, s[1] + s[0] * r41)
    return peak


def hotplug(seed=None):
    parts, _ = load(seed)
    r41 = dn.ohms(parts["R41"]["value"]) if parts["R41"]["value"] != "0" else 0.0
    c1 = c(parts, "C1")
    limit = U2_OVP_MIN - 0.2
    worst = (0, None)
    for rc, lc, share in itertools.product(STAND_CABLE["r"], STAND_CABLE["l"], STAND_C1_EFF):
        for rr in (r41 * (1 - R_TOL), r41 * (1 + R_TOL)):
            pk = hotplug_peak(5.25, rc, lc, rr, c1 * share)
            if pk > worst[0]:
                worst = (pk, (rc, lc, share, rr))
    pk, (rc, lc, share, rr) = worst
    ok = pk <= limit
    print(f"a 5.25 V source stepped onto a 2 m cable (loop {STAND_CABLE['r'][0]}-{STAND_CABLE['r'][1]} ohm, "
          f"{STAND_CABLE['l'][0] * 1e6:.0f}-{STAND_CABLE['l'][1] * 1e6:.0f} uH: stand-ins) into R41 {parts['R41']['value']} ohm "
          f"and C1 {parts['C1']['value']} at {STAND_C1_EFF[0] * 100:.0f}-{STAND_C1_EFF[1] * 100:.0f}% of nominal (DC bias, VER-30)")
    print(f"worst VBUS at U2's IN {pk:.2f} V (cable {rc} ohm, {lc * 1e6:.0f} uH, C1 {share * c1 * 1e6:.1f} uF, R41 {rr:.2f} ohm) "
          f"against OVP's minimum {U2_OVP_MIN} V less 0.2 V = {limit:.1f} V (bq24074.txt:699)")
    r43 = r(parts, "R43") * (1 - R_TOL)
    en1 = pk * U2_EN_PD / (U2_EN_PD + r43)
    ok &= en1 <= U2_EN_ABS - 0.3
    print(f"EN1 at that peak {en1:.2f} V through R43 {parts['R43']['value']} and the 285k pull-down (bq24074.txt:512), "
          f"limit {U2_EN_ABS - 0.3:.1f} V (7 V absolute, bq24074.txt:598)")
    r14, r15 = r(parts, "R14") * (1 - R_TOL), r(parts, "R15") * (1 + R_TOL)
    pwr_en = pk * r15 / (r14 + r15)
    ok &= pwr_en <= 5.5
    print(f"PWR_EN at most {pwr_en:.2f} V (R14/R15 with D3's drop taken as 0, C47 {parts['C47']['value']} only slows it), "
          f"limit 5.5 V (U3's EN 6 V absolute, rt9013.pdf p3)")
    return dn.result(ok)


# ---- VER-13 ---------------------------------------------------------------------------------------------------
CU_T = 0.035                     # mm: 1 oz outer copper, JLC's default 2-layer order (no weight is named in assembly.md:10)
CU_RHO = 1.72e-5                 # ohm.mm: copper at 20 C
VIA_PLATE = 0.018                # mm: via barrel plating, a thin figure (to confirm on JLC's capability page, VER-17)
I_PATH = 0.75                    # A: the row's current through the input paths (U2's 475 mA USB500 limit, bq24074.txt:733,
                                 # sits under it)
I_LOADS = {"U3": ("1", 0.15), "J3": (None, 0.12), "U7": ("6", 0.5)}   # A: 3V3 budget (REQ-PWR-15), the panel's 120 mA
                                 # backlight (pin_review/panel.json), the amplifier's peak (stand-in: 0.35 W into 8 ohm
                                 # bridged at 4 V needs about 0.3 A, plus margin)
SEED_COPPER = {"CELL_P": 0.05}   # a 0.05 mm track for the cell (applied to the board in memory)


def ipc2221_amps(width_mm, dt=10.0):
    """IPC-2221's external-layer chart as its formula: I = 0.048 dT^0.44 A^0.725, A in square mils (from memory:
    IPC-2152 allows more on most boards, so this is the stricter reading)."""
    area = (width_mm / 0.0254) * (CU_T / 0.0254)
    return 0.048 * dt ** 0.44 * area ** 0.725


def net_graph(pcb, b, net, narrow=None):
    """Nodes and resistive edges of one net's tracks and vias; pads join every node inside them."""
    mm = pcb.ToMM
    key = lambda x, y, layer: (round(x, 3), round(y, 3), layer)   # noqa: E731
    edges, segs = [], []
    for t in b.GetTracks():
        if t.GetNetname().split("/")[-1] != net:
            continue
        if t.GetClass() == "PCB_VIA":
            x, y = mm(t.GetPosition().x), mm(t.GetPosition().y)
            d = mm(t.GetDrillValue())
            rv = CU_RHO * 1.6 / (math.pi * d * VIA_PLATE)
            edges.append((key(x, y, "F"), key(x, y, "B"), rv, None))
        else:
            w = narrow if narrow else mm(t.GetWidth())
            lay = "F" if t.GetLayer() == pcb.F_Cu else "B"
            a = (mm(t.GetStart().x), mm(t.GetStart().y))
            z = (mm(t.GetEnd().x), mm(t.GetEnd().y))
            segs.append((a, z, w, lay))
    # split a segment where another's end lands on it (T-junctions)
    ends = [(p, s[3]) for s in segs for p in (s[0], s[1])]
    for a, z, w, lay in segs:
        pts = [a, z]
        for p, l2 in ends:
            if l2 != lay or p in (a, z):
                continue
            dx, dy = z[0] - a[0], z[1] - a[1]
            L2 = dx * dx + dy * dy
            if L2 == 0:
                continue
            u = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / L2
            if 0 < u < 1 and math.hypot(a[0] + u * dx - p[0], a[1] + u * dy - p[1]) <= w / 2:
                pts.append(p)
        pts.sort(key=lambda p: (p[0] - a[0]) ** 2 + (p[1] - a[1]) ** 2)
        for p, q in zip(pts, pts[1:]):
            length = math.hypot(q[0] - p[0], q[1] - p[1])
            edges.append((key(*p, lay), key(*q, lay), max(CU_RHO * length / (w * CU_T), 1e-7), w))
    nodes = {n for e in edges for n in e[:2]}
    # merge coincident ends of different widths (within the narrower half-width)
    near = []
    nl = sorted(nodes)
    for i, n in enumerate(nl):
        for m in nl[i + 1:i + 40]:
            if m[2] == n[2] and n != m and math.hypot(n[0] - m[0], n[1] - m[1]) < 0.05:
                near.append((n, m, 1e-7, None))
    edges += near
    pads = {}
    for f in b.GetFootprints():
        for pad in f.Pads():
            if pad.GetNetname().split("/")[-1] != net:
                continue
            pk = ("pad", f.GetReference(), pad.GetNumber())
            pads.setdefault((f.GetReference(), pad.GetNumber()), []).append(pk)
            for n in nodes:
                lay = pcb.F_Cu if n[2] == "F" else pcb.B_Cu
                if pad.IsOnLayer(lay) and pad.HitTest(pcb.VECTOR2I(pcb.FromMM(n[0]), pcb.FromMM(n[1]))):
                    edges.append((pk, n, 1e-6, None))
    return edges, pads


def solve(edges, src, sink, amps):
    """Node voltages with 'amps' injected at the src node and the sink node held at 0 V (Gaussian elimination)."""
    nodes = sorted({n for e in edges for n in e[:2]}, key=str)
    idx = {n: i for i, n in enumerate(nodes)}
    if src not in idx or sink not in idx:
        return None
    n = len(nodes)
    g = [[0.0] * n for _ in range(n)]
    for a, z, res, _ in edges:
        i, j = idx[a], idx[z]
        if i == j:
            continue
        y = 1 / res
        g[i][i] += y
        g[j][j] += y
        g[i][j] -= y
        g[j][i] -= y
    rhs = [0.0] * n
    rhs[idx[src]] = amps
    k = idx[sink]
    keep = [i for i in range(n) if i != k]
    a = [[g[i][j] for j in keep] + [rhs[i]] for i in keep]
    m = len(keep)
    for col in range(m):
        piv = max(range(col, m), key=lambda r_: abs(a[r_][col]))
        if abs(a[piv][col]) < 1e-12:
            return None                                  # not connected through these tracks
        a[col], a[piv] = a[piv], a[col]
        for r_ in range(col + 1, m):
            fct = a[r_][col] / a[col][col]
            if fct:
                for c_ in range(col, m + 1):
                    a[r_][c_] -= fct * a[col][c_]
    x = [0.0] * m
    for r_ in range(m - 1, -1, -1):
        x[r_] = (a[r_][m] - sum(a[r_][c_] * x[c_] for c_ in range(r_ + 1, m))) / a[r_][r_]
    v = {}
    for pos_, i in enumerate(keep):
        v[nodes[i]] = x[pos_]
    v[sink] = 0.0
    return v


def path_drop(pcb, b, net, src_pads, sink_pad, amps, narrow=None):
    edges, pads = net_graph(pcb, b, net, narrow)
    src = ("src", net)
    for p in src_pads:
        for pk in pads.get(p, []):
            edges.append((src, pk, 1e-7, None))
    sink = pads.get(sink_pad, [None])[0]
    v = solve(edges, src, sink, amps)
    if v is None:
        return None, None
    worst = []
    for a, z, res, w in edges:
        if w:
            i = abs(v.get(a, 0) - v.get(z, 0)) / res
            worst.append((i / ipc2221_amps(w), i, w))
    return v[src], max(worst) if worst else (0, 0, 0)


def copper(seed=None):
    parts, _ = load(None)
    pcb, b = dn.board()
    narrow_net = list(seed)[0] if seed else None
    ok = True
    print(f"copper {CU_T * 1e3:.0f} um (1 oz, JLC's default; stand-in until the order options name it), rho {CU_RHO} ohm.mm, "
          f"vias {VIA_PLATE * 1e3:.0f} um plating over 1.6 mm; IPC-2221's external chart for a 10 C rise (formula, from memory)")

    def leg(net, src, sink, amps, label):
        nonlocal ok
        drop, (ratio, i, w) = path_drop(pcb, b, net, src, sink, amps, seed[net] if seed and net == narrow_net else None)
        if drop is None:
            print(f"  {label}: {net} not connected by tracks between {src} and {sink} (a pour or a jumper carries it)")
            ok = False
            return 0.0
        good = ratio <= 1.0
        ok &= good
        print(f"  {label}: {net} {drop * 1e3:.1f} mV at {amps} A ({drop / amps * 1e3:.1f} mohm); its most loaded track "
              f"{w:.2f} mm carries {i:.2f} A, {ratio * 100:.0f}% of IPC-2221's {ipc2221_amps(w):.2f} A for 10 C"
              + ("" if good else ": OVER"))
        return drop

    u2_out = [("U2", "10"), ("U2", "11")]
    print(f"VBUS to VSYS with {I_PATH} A in (the row's current):")
    d_in = leg("VBUS", [("J1", p) for p in ("A4", "A9", "B4", "B9")], ("U2", "13"), I_PATH, "J1 to U2 IN")
    print(f"VBAT to VSYS with {I_PATH} A out of the cell:")
    d_cell = leg("CELL_P", [("J2", "1")], ("JP5", "1"), I_PATH, "J2 to JP5")
    d_bat = leg("VBAT", [("JP5", "2")], ("U2", "2"), I_PATH, "JP5 to U2 BAT")
    print("VSYS from U2's OUT to each load at its own current:")
    d_sys = []
    for ref, (pin, amps) in I_LOADS.items():
        if ref == "J3":
            d_sys.append(leg("VSYS", u2_out, ("JP2", "1"), amps, "U2 OUT to JP2 (LED_A)"))
            leg("LED_A", [("JP2", "2")], next(("J3", p) for p, n in parts["J3"]["pins"].items() if n == "LED_A"), amps,
                "JP2 to J3 (LED_A)")
        elif ref == "U7":
            d_sys.append(leg("VSYS", u2_out, ("JP3", "1"), amps, "U2 OUT to JP3 (AMP_VDD)"))
            leg("AMP_VDD", [("JP3", "2")], ("U7", pin), amps, "JP3 to U7 VDD")
        else:
            d_sys.append(leg("VSYS", u2_out, (ref, pin), amps, f"U2 OUT to {ref} pin {pin}"))
    worst_sys = max(d_sys)
    a = d_in + worst_sys
    bt = d_cell + d_bat + worst_sys
    print(f"VBUS to VSYS copper: {a * 1e3:.1f} mV (limit 50); VBAT to VSYS copper: {bt * 1e3:.1f} mV (limit 30), each "
          "with the worst VSYS branch; U2's own switches are not copper and are not counted")
    ok &= a <= 0.050 and bt <= 0.030
    return dn.result(ok)


# ---- VER-140 (blocked on the PCM) -----------------------------------------------------------------------------
def mating(seed=None):
    parts, _ = load(seed)
    vbat_c = sum(c(parts, p) for p in ("C3",))
    vsys = [p for p, q in parts.items() if q["lib"] == "Device:C" and q["bom"] and
            set(q["pins"].values()) & {"VSYS", "AMP_VDD"}]
    vsys_c = sum(c(parts, p) for p in vsys)
    print(f"plugging the cell (no USB): C3 {vbat_c * 1e6:.1f} uF on VBAT at once, and {', '.join(sorted(vsys))} "
          f"({vsys_c * 1e6:.1f} uF) on VSYS through U2's battery FET (at most {U2_BAT_OUT_R} ohm: 100 mV at 1 A, "
          "bq24074.txt:724)")
    for rs in (0.05, 0.15, 0.35):
        ipk = 4.2 / rs
        tau = (rs + U2_BAT_OUT_R) * vsys_c
        print(f"  series {rs} ohm (cell, PCM, leads; stand-in): peak about {ipk:.0f} A into C3, VSYS charged with tau "
              f"{tau * 1e6:.1f} us; {(vbat_c + vsys_c) * 4.2 * 1e6:.0f} uC in all")
    print("PWR_EN through the edge: no capacitor or diode joins VSYS to PWR_EN with the domes open (D2, D8 and D3 are "
          "reverse-biased or at 0 V; R13 and R62 hold the RAW nodes at 0 V), so PWR_EN moves only by the open dome's "
          "pad capacitance into Q1's gate: well under U3's 0.6 V VIL")
    print("BLOCKED: the PCM's short-circuit threshold and delay come from the cell's spec sheet (VER-37); with them, "
          "compare the peak and tau above. No result until then.")
    print("RESULT: BLOCKED")
    return 2


# ---- VER-30 (blocked on Samsung's DC-bias curves) ----------------------------------------------------------------
def mlccbias(seed=None):
    parts, _ = load(seed)
    rail = {"VBUS_C1": 5.25, "VSYS": 4.5, "VBAT": 4.2, "3V3_LDO": 3.37, "+3V3": 3.37, "+1V1": 1.16,
            "VREG_AVDD": 3.37, "AMP_VDD": 4.5}
    need = [(("C1",), 1e-6, "U2 IN 1-10 uF (bq24074.txt:520)"),
            (("C2", "C5", "C34"), 4.7e-6, "U2 OUT 4.7-47 uF (bq24074.txt:533), C34 through JP3"),
            (("C3",), 4.7e-6, "U2 BAT 4.7-47 uF (bq24074.txt:501)"),
            (("C5",), 1e-6, "U3 input 1 uF X7R or more (rt9013.pdf p8)"),
            (("C6", "C7"), 1e-6, "U3 output 1 uF or more (rt9013.pdf p8)"),
            (("C24",), 4.7e-6, "VREG CIN at least 4.7 uF (rp2350.txt:33598)"),
            (("C25",), 3.76e-6, "VREG COUT 4.7 uF -20% (rp2350.txt:33603)"),
            (("C34",), 1e-6, "FM8002A Cs 1 uF (u7.json p6, not on disk)")]
    short = []
    for refs, cmin, why in need:
        v = max(rail.get(n, 0) for ref in refs for n in parts[ref]["pins"].values())
        have = sum(c(parts, ref) * (1 - ctol(parts, ref)) for ref in refs)
        if have < cmin * 0.999:
            short.append("+".join(refs))
        print(f"  {'+'.join(refs)} ({', '.join(parts[ref]['mpn'] for ref in refs)}) at {v:.2f} V: {have * 1e6:.2f} uF at "
              f"its low tolerance; needs {cmin * 1e6:.2f} uF ({why}): "
              + (f"may lose up to {(1 - cmin / have) * 100:.0f}% to DC bias" if have >= cmin * 0.999 else
                 "SHORT before any DC-bias loss (the curve can only lower it)"))
    print("Samsung's DC-bias curves for CL10A106KP8NNNC, CL10A106MA8NRNC, CL10A475KO8NNNC, CL05A475MP5NRNC "
          "(product.samsungsem.com, the MLCC pages) are not in hardware/ref_design/datasheets: each 'may lose' above "
          "waits for its curve (BLOCKED)")
    if short:
        print(f"short at the part's own tolerance, whatever the curve: {', '.join(short)}")
        return dn.result(False)
    print("RESULT: BLOCKED")
    return 2


CHECKS = {"latch": (latch, SEED_LATCH), "usb": (usb, SEED_USB), "audio": (audio, SEED_AUDIO), "core": (core, SEED_CORE),
          "shelf": (shelf, SEED_SHELF), "ampstate": (ampstate, SEED_AMP), "hotplug": (hotplug, SEED_HOTPLUG),
          "copper": (copper, SEED_COPPER), "mating": (mating, None), "mlccbias": (mlccbias, None)}


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if "--list" in sys.argv or not args:
        print(__doc__)
        return 0
    fn, seed = CHECKS[args[0]]
    if "--seed" in sys.argv:
        if not seed:
            sys.exit(f"paper_checks: {args[0]} has no seed")
        print(f"SEEDED: {seed}")
        return fn(seed)
    return fn()


if __name__ == "__main__":
    sys.exit(main())
