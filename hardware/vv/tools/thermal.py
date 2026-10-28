#!/usr/bin/env python3
"""Class 15 on paper: each part's worst-case dissipation per mode, its junction's rise, the case's inside and the cell.

    python3 hardware/vv/tools/thermal.py modes       # VER-144: U2, U3, Q5, Q6 per mode; the firmware heat rule
    python3 hardware/vv/tools/thermal.py board       # VER-193: every other part, the cell's own heat, the case's inside
    python3 hardware/vv/tools/thermal.py amp         # VER-200: the amplifier (U7), on a stand-in theta (fm8002a.pdf prints none)
    python3 hardware/vv/tools/thermal.py casemat     # VER-192: the case's material against its hottest inner surface (class 5)
    python3 hardware/vv/tools/thermal.py <check> --seed    # a seeded fault (a hotter part): must exit 1

The model, stated once: the heat inside the case leaves through its outside, area A from case.scad (case_w, case_l,
case_h), at h_out W/m2K (still air, convection and radiation together); the air and the board inside sit a second
P / (h_in A) above the case's skin. A part's junction is that inside temperature plus its dissipation times its
datasheet's junction-to-ambient resistance (each quoted from its file and line through tools/ds.py, or from its
pin_review quote, marked not on disk). h_out and h_in are stand-ins (no file gives them) and are printed. Every
dissipation comes from design.py's values and the datasheets' maxima at their worst corner; the 3V3 load is VER-31's.

Limits (VER-144, VER-193): each junction at or under its maximum less 20 C at 40 C ambient in the worst mode; U2's
junction under its thermal regulation point in a 40 C case; the cell's surface at or under 45 C charging while playing
at 25 C ambient.

Python 3 standard library and pdftotext; nothing is written under hardware/ref_design.
"""
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import design_numbers as dn  # noqa: E402
import ds  # noqa: E402

# stand-ins: no file gives these; each printed where it is used
H_OUT = 10.0            # W/m2K: a handheld's skin in still air, convection about 5 and radiation about 5
H_IN = 10.0             # W/m2K: from the board and parts to the case's inside, through the air gaps
I_AMP_AVG = 0.15        # A: the amplifier's mean supply current at loud game audio (its peak 0.5 A: VSYS / 8 ohm)
CELL_ESR = 0.3          # ohm: a 700 mAh pouch with its PCM and leads, pessimistic until VER-37 reads the cell
VF_LED_MIN = 2.8        # V: the panel's LED forward voltage low end (the drawing gives 3.0 typical, 3.2 maximum)
T_AMB, T_AMB_CELL = 40.0, 25.0
MARGIN = 20.0


def quotes():
    q = ds.quote
    return {
        "u2_theta": (44.5, q("bq24074", 666, "RθJA", "44.5")),
        "u2_tj": (125.0, q("bq24074", 641, "TJ", "125")),
        "u2_treg": (125.0, q("bq24074", 831, "TJ(REG)", "125")),
        "u2_eq": q("bq24074", 2662, "P = [V(IN) – V(OUT)]"),
        "u2_vbat_min": (3.4, q("bq24074", 2655, "3.4 V is a good minimum voltage")),
        "u2_vout": ((4.3, 4.5), q("bq24074", 729, "4.3", "4.5")),
        "u2_iin": (0.5, q("bq24074", 733, "450", "500")),
        "u2_kiset": (975.0, q("bq24074", 785, "975")),
        "u2_rbat": (0.1, q("bq24074", 724, "100")),
        "u3_theta": (231.1, q("tlv757p", 232, "RθJA", "231.1")),
        "u3_theta_evm": (100.8, q("tlv757p", 229, "RθJA", "100.8")),
        "u3_tj": (125.0, q("tlv757p", 213, "Junction temperature", "125")),
        "u4_ta": (65.0, q("esp32-s3-wroom-1", 1598, "65 °C version", "65")),
        "u7_tj": (150.0, q("fm8002a", 112, "TJ", "150")),
        "u8_theta": (108.0, q("sn74hc165", 281, "108")),
        "led_vf": (3.2, q("er-tft035ips-6", 758, "3.0", "3.2")),
    }


def three_v3():
    """VER-31's 3V3 load (the same parts and stand-ins), in A."""
    import io
    import contextlib
    import circuit_checks as cc
    parts, _ = dn.design()
    return sum(i for _, i, _ in cc.loads_3v3(parts, "avg"))      # Wi-Fi listening on a busy CPU, the SD card writing


def case_area():
    e = dn.scad()
    w, l, h = e["case_w"] / 1e3, e["case_l"] / 1e3, e["case_h"] / 1e3
    return 2 * (w * l + w * h + l * h), (e["case_w"], e["case_l"], e["case_h"])


def inside(p_total, t_amb):
    a, _ = case_area()
    skin = t_amb + p_total / (H_OUT * a)
    return skin, skin + p_total / (H_IN * a)


class Rows:
    def __init__(self):
        self.bad = []

    def __call__(self, good, text):
        print(("  ok    " if good else "  FAIL  ") + text)
        if not good:
            self.bad.append(text)
        return good


def modes_table(Q, seed=None):
    """{mode: {part: watts}} and {mode: total heat inside}, every part at its worst corner."""
    parts, _ = dn.design()
    r5 = dn.ohms(parts["R5"]["value"]) * (1 - dn.R_TOL)
    ichg = Q["u2_kiset"][0] / r5
    vin, vout_lo, vout_hi = 5.25, Q["u2_vout"][0][0], Q["u2_vout"][0][1]
    vbat = Q["u2_vbat_min"][0]
    i3 = three_v3()
    i_led = 0.1177                                       # A: VER-137's worst backlight current
    i_sys = i3 + i_led + I_AMP_AVG
    q_led = (vout_hi - VF_LED_MIN - 0.13) * i_led / 2    # each of Q5, Q6: VSYS less the LEDs and the 0.13 V sense
    r_led = (i_led / 2) ** 2 * 2.2                       # R52, R53
    out = {}
    # charging while playing: IN at its 500 mA limit; the system first, the charge current the rest, at most ICHG
    ich = min(ichg, Q["u2_iin"][0] - i_sys) if i_sys < Q["u2_iin"][0] else 0.0
    u2_cwp = (vin - vout_lo) * Q["u2_iin"][0] + (vout_hi - vbat) * ich
    # the corner U2 sees hottest: the system drawing just enough to leave the whole ICHG (equation 11's two terms at once)
    u2_max = (vin - vout_lo) * Q["u2_iin"][0] + (vout_hi - vbat) * ichg
    if seed:
        u2_max += 1.0
    u3 = (vout_hi - 3.3) * i3
    u3_bat = (dn.VBAT_REG[1] - 3.3) * i3
    sys_heat_usb = vout_hi * i_sys                       # every watt into VSYS ends as heat inside (the speaker too)
    sys_heat_bat = dn.VBAT_REG[1] * i_sys
    i_cell_play = i_sys
    out["off"] = ({}, 0.0)
    out["cell play"] = ({"U2": i_cell_play ** 2 * Q["u2_rbat"][0], "U3": u3_bat, "Q5": q_led, "Q6": q_led, "R52": r_led,
                         "R53": r_led, "cell": i_cell_play ** 2 * CELL_ESR}, sys_heat_bat + i_cell_play ** 2 * (Q["u2_rbat"][0] + CELL_ESR))
    p_usb_u2 = (vin - vout_lo) * min(i_sys, Q["u2_iin"][0])
    out["USB play"] = ({"U2": p_usb_u2, "U3": u3, "Q5": q_led, "Q6": q_led, "R52": r_led, "R53": r_led},
                       sys_heat_usb + p_usb_u2)
    p_chg = (vin - vout_lo) * ichg + (vout_hi - vbat) * ichg
    out["charging, off"] = ({"U2": p_chg, "cell": ichg ** 2 * CELL_ESR}, p_chg + ichg ** 2 * CELL_ESR)
    out["charging while playing"] = ({"U2": max(u2_cwp, u2_max), "U3": u3, "Q5": q_led, "Q6": q_led, "R52": r_led,
                                      "R53": r_led, "cell": ich ** 2 * CELL_ESR},
                                     sys_heat_usb + u2_cwp + ich ** 2 * CELL_ESR)
    return out, dict(ichg=ichg, i3=i3, i_sys=i_sys, ich=ich, vbat=vbat, u2_max=u2_max, u2_cwp=u2_cwp)


# ---- VER-144 ------------------------------------------------------------------------------------------------------------
THETA = {"Q5": (125.0, "RthJA 100-125 C/W steady (q5.json p1, not on disk)"),
         "Q6": (125.0, "as Q5"), "R52": (300.0, "an 0603 resistor, stand-in"), "R53": (300.0, "an 0603 resistor, stand-in")}
TJMAX = {"Q5": (125.0, "stand-in: the AO3400A's maximum is not in q5.json (VER-34)"), "Q6": (125.0, "as Q5"),
         "R52": (155.0, "the resistor's 155 C (JLC's description, passives.json)"), "R53": (155.0, "as R52")}


def modes(seed=None):
    Q = quotes()
    row = Rows()
    tab, k = modes_table(Q, seed)
    a, dims = case_area()
    THETA.update({"U2": (Q["u2_theta"][0], "bq24074.txt:666"), "U3": (Q["u3_theta"][0], "tlv757p.txt:232, DBV on JEDEC's "
                                                                                       "board; 100.8 on TI's EVM, 229")})
    TJMAX.update({"U2": (Q["u2_tj"][0], "bq24074.txt:641"), "U3": (Q["u3_tj"][0], "tlv757p.txt:213")})
    print(f"the case {dims[0]:.2f} x {dims[1]:.2f} x {dims[2]:.2f} mm:{a * 1e4:.1f} cm2 outside; h_out {H_OUT:g} and h_in {H_IN:g} W/m2K "
          f"(stand-ins): {1 / (H_OUT * a):.1f} C/W skin to air, {1 / (H_IN * a):.1f} C/W inside to skin")
    print(f"3V3 load {k['i3'] * 1e3:.1f} mA (VER-31's average: Wi-Fi listening, the CPU busy, the SD card writing); the system while playing {k['i_sys'] * 1e3:.0f} mA "
          f"(3V3, the backlight's 117.7 mA, the amplifier's mean {I_AMP_AVG * 1e3:.0f} mA: a stand-in); ICHG at most "
          f"{k['ichg'] * 1e3:.0f} mA (KISET 975 / R5 -1%); VBAT {k['vbat']} V in fast charge (bq24074.txt:2655)")
    print(f"U2 by TI's equation (11) (bq24074.txt:2662): {k['u2_max']:.3f} W at the corner where IN carries its 500 mA, the "
          "system draws just the rest and the cell takes the whole ICHG; "
          f"{k['u2_cwp']:.3f} W with this system's own load ({k['ich'] * 1e3:.0f} mA left to charge)")
    print(f"{'mode':26s} {'heat in':>8s} {'skin':>6s} {'inside':>7s}  parts (W)")
    worst = {}
    for mode, (pp, heat) in tab.items():
        skin, air = inside(heat, T_AMB)
        print(f"{mode:26s} {heat:7.2f}W {skin:5.1f}C {air:6.1f}C  " + ", ".join(f"{p} {w:.3f}" for p, w in pp.items()))
        for p, w in pp.items():
            if p not in THETA:
                continue
            tj = air + w * THETA[p][0]
            if tj > worst.get(p, (0,))[0]:
                worst[p] = (tj, mode, w, air)
    print(f"each junction in its worst mode at {T_AMB:g} C outside, against its maximum less {MARGIN:g} C:")
    for p in ("U2", "U3", "Q5", "Q6", "R52", "R53"):
        tj, mode, w, air = worst[p]
        lim = TJMAX[p][0] - MARGIN
        row(tj <= lim, f"{p}: {w:.3f} W in {mode}: {air:.1f} + {w:.3f} x {THETA[p][0]:g} C/W ({THETA[p][1]}) = {tj:.1f} C against "
            f"{TJMAX[p][0]:g} - {MARGIN:g} = {lim:g} C ({TJMAX[p][1]})")
    u2w = tab["charging while playing"][0]["U2"]
    t40 = 40.0 + u2w * Q["u2_theta"][0]
    row(t40 < Q["u2_treg"][0], f"U2 in a 40 C case: 40 + {u2w:.3f} x {Q['u2_theta'][0]:g} = {t40:.1f} C, under its {Q['u2_treg'][0]:g} C "
        "thermal regulation (bq24074.txt:831)")
    th_lim = (TJMAX["U2"][0] - MARGIN - worst["U2"][3]) / worst["U2"][2]
    print(f"sensitivity: U2 reaches its limit at {th_lim:.1f} C/W against the datasheet's 44.5 (a JEDEC board with inner planes); a "
          "2-layer board is worse, so VER-68 measures the case under U2 on unit 1, and U2's own regulation lowers ICHG first")
    lim3 = TJMAX["U3"][0] - MARGIN
    u3_evm = worst["U3"][3] + worst["U3"][2] * Q["u3_theta_evm"][0]
    th3 = (lim3 - worst["U3"][3]) / worst["U3"][2]
    print(f"sensitivity: U3 on TI's EVM figure (100.8 C/W, tlv757p.txt:229) reaches {u3_evm:.1f} C; it meets {lim3:g} C below "
          f"{th3:.0f} C/W, between the EVM's and JEDEC's: the board's own figure decides (VER-68 reads the case over U3 on unit 1)")
    print("the firmware heat rule (for pinmap.md before VER-86 runs, proposed): on USB with the backlight above 75%, cap the "
          "amplifier's volume (its share is the largest heat after U2) or pause charging with CHG_OFF while the case reads hot; "
          "VER-86 measures the skin")
    print("U7, the amplifier, is VER-200's: fm8002a.pdf gives its 150 C junction maximum (fm8002a.txt:112) and no thermal resistance")
    print(f"{len(row.bad)} over")
    return dn.result(not row.bad)


# ---- VER-193 ------------------------------------------------------------------------------------------------------------
STAND_SMALL_THETA = 400.0       # C/W: worse than any SMD package's junction-to-ambient, for parts whose power is tiny
STAND_MODULE_THETA = 30.0       # C/W: the module's shield to the air round it on a 2-layer board (no Espressif figure)


def board(seed=None):
    Q = quotes()
    row = Rows()
    tab, k = modes_table(Q)
    pp, heat = tab["USB play"]
    _, air40 = inside(max(h for _, h in tab.values()), T_AMB)
    worst_mode = max(tab, key=lambda m: tab[m][1])
    print(f"the hottest mode for the inside is {worst_mode} ({tab[worst_mode][1]:.2f} W): inside {air40:.1f} C at {T_AMB:g} C outside")
    i3 = k["i3"]
    import circuit_checks as cc
    p_u4 = 3.3 * (cc.ESP_CPU + cc.ESP_RX + cc.STAND_PSRAM)     # the module's own share of the 3V3 load (VER-31)
    # the module is rated by its ambient (65 C for the R8's octal PSRAM, esp32-s3-wroom-1.txt:1598), not by a junction: its
    # ambient is the case's inside air, and the module's own heat warms the board round it (stand-in: 30 C/W to that air)
    t_mod = air40 + p_u4 * STAND_MODULE_THETA
    row(t_mod <= Q["u4_ta"][0], f"U4 the module: {p_u4:.3f} W (CPU 107.9 mA, Wi-Fi RX 97 mA, PSRAM 20 mA at 3.3 V, VER-31) puts the "
        f"air round it at {air40:.1f} + {p_u4:.3f} x {STAND_MODULE_THETA:g} C/W (stand-in) = {t_mod:.1f} C against its 65 C "
        "ambient limit (esp32-s3-wroom-1.txt:1598; 85 C with the PSRAM's ECC on and 1/16 of it lost, 103-105)")
    parts = [
        ("U8", 3.3 * 80e-6 + 2 * 10e-12 * 3.3 ** 2 * 5e5, Q["u8_theta"][0], 125.0,
         "ICC and the scan's switching at 0.5 MHz; TSSOP 108 C/W (sn74hc165.txt:281); 125 C, the HC family's usual maximum (stand-in)"),
        ("U9", 3.3 * 80e-6 + 2 * 10e-12 * 3.3 ** 2 * 5e5, Q["u8_theta"][0], 125.0, "as U8"),
        ("U11", 3.3 * 80e-6 + 2 * 10e-12 * 3.3 ** 2 * 5e5, Q["u8_theta"][0], 125.0, "as U8"),
        ("U1", 3.3 * 5e-6, STAND_SMALL_THETA, 125.0, "leakage only, 5 uA at 5 V (srv05-4.txt:94); any package's theta"),
        ("U6", 3.3 * 0.34e-3 + (3.3 - 1.5) * 0.5e-3 * 2, STAND_SMALL_THETA, 125.0,
         "quiescent 170 uA a channel (mcp6001.txt:160) and two gates' charge (stand-in)"),
        ("U10", 3.3 * 0.17e-3, STAND_SMALL_THETA, 125.0, "quiescent 170 uA maximum (mcp6001.txt:160)"),
        ("RN1-RN4", 4 * 3.3 ** 2 / 47e3, STAND_SMALL_THETA, 155.0, "four 47k elements an array, all pressed"),
    ]
    # a connector is rated by its rise at its rated current: J1's 0.5 A against a USB-C receptacle's 3 A (Type-C, from memory)
    # at the catalogue's 30 C rise, so its rise goes with the square of the share; 85 C operating (stand-ins: not on disk)
    j1_rise = 30.0 * (0.5 / 3.0) ** 2
    print(f"  J1: 0.5 A of a 3 A receptacle: +{j1_rise:.1f} C (30 C at rated current, scaled by the current's square: stand-ins), "
          f"{air40 + j1_rise:.1f} C against 85 - {MARGIN:g}")
    row(air40 + j1_rise <= 85.0 - MARGIN, f"J1 at {air40 + j1_rise:.1f} C")
    if seed:
        parts.append(("seeded", 1.0, 200.0, 125.0, "a 1 W part at 200 C/W"))
    print(f"each part at {T_AMB:g} C outside in the hottest mode, against its maximum less {MARGIN:g} C:")
    for name, w, th, tj, why in parts:
        t = air40 + w * th
        row(t <= tj - MARGIN, f"{name}: {w * 1e3:.2f} mW x {th:g} C/W = +{w * th:.1f} C: {t:.1f} C against {tj - MARGIN:g} ({why})")
    # the cell, charging while playing at 25 C
    pp_c, heat_c = tab["charging while playing"]
    skin25, air25 = inside(heat_c, T_AMB_CELL)
    e = dn.scad()
    cell_area = 2 * (e["cell_nom"][0] * e["cell_nom"][1]) / 1e6
    own = pp_c.get("cell", 0.0)
    t_cell = air25 + own / (H_IN * cell_area)
    row(t_cell <= 45.0, f"the cell charging while playing at {T_AMB_CELL:g} C outside: {heat_c:.2f} W inside, the air round it "
        f"{air25:.1f} C, its own {own * 1e3:.0f} mW (ESR {CELL_ESR} ohm, a stand-in until VER-37) over its {cell_area * 1e4:.0f} cm2 "
        f"faces: {t_cell:.1f} C against 45 C")
    print(f"  (U2's {pp_c['U2']:.2f} W sits on the same board; how much of it reaches the cell through the hole's edge is what "
          "VER-85 and VER-87 measure on unit 1; TH1 reads the cell's edge)")
    # the case's inside, for VER-192
    _, hot = inner_surface(Q, tab, T_AMB)
    print(f"the case's inner surface for VER-192: {air40:.1f} C on average at {T_AMB:g} C outside; over U2 about {hot:.1f} C "
          f"(the inside plus {U2_TO_LID:.0%} of U2's rise across the 2.4 mm gap: a stand-in until VER-86 measures)")
    print(f"{len(row.bad)} over")
    return dn.result(not row.bad)


U2_TO_LID = 0.3                 # share of U2's junction rise that reaches the lid over it across the 2.4 mm air gap (stand-in)


def inner_surface(Q, tab, t_amb):
    """(average, over U2): the case's inner surface in the hottest mode at t_amb outside, and over U2 charging while
    playing (VER-193 prints them, VER-192 holds the material to them)."""
    _, air = inside(max(h for _, h in tab.values()), t_amb)
    return air, air + U2_TO_LID * (tab["charging while playing"][0]["U2"] * Q["u2_theta"][0])


# ---- VER-192 ------------------------------------------------------------------------------------------------------------
PRINT_README = os.path.join(dn.REF, "case", "print_package_README.md")
STAND_HDT = 57.0                # C: PLA's heat deflection at 0.45 MPa, the usual 55-60 C (from memory: the maker's sheet is
                                # not on disk; VER-192's needs names it)
HDT_MARGIN = 15.0               # C: VER-192's pass limit, the heat deflection over the hottest inner surface


def casemat(seed=None):
    import re
    Q = quotes()
    row = Rows()
    tab, _ = modes_table(Q)
    text = open(PRINT_README, encoding="utf-8").read().split("\n")
    line = next((k for k, t in enumerate(text, 1) if t.startswith("| **PLA** (every part)")), None)
    brand = re.search(r"AMS slot 1\*\*: ([^|]+)", text[line - 1]).group(1).strip() if line else "unknown"
    print(f"the case's material: {brand} for every printed part (print_package_README.md:{line}); the support filament "
          "touches only the top's overhangs and is removed")
    hdt = STAND_HDT - (30.0 if seed else 0.0)
    print(f"its heat deflection temperature at 0.45 MPa: {hdt:g} C, a stand-in (PLA's usual 55-60 C, from memory): Bambu "
          "Lab's technical data sheet for it is not on disk")
    for t_amb in (T_AMB, T_AMB_CELL):
        avg, hot = inner_surface(Q, tab, t_amb)
        need = hot + HDT_MARGIN
        line_ = (f"at {t_amb:g} C outside, the hottest mode: the inner surface {avg:.1f} C on average and {hot:.1f} C over U2 "
                 f"(VER-193's model; {U2_TO_LID:.0%} of U2's rise reaches the lid, a stand-in): the material needs "
                 f"{need:.1f} C or more, it has {hdt:g} C")
        if t_amb == T_AMB:
            row(hdt >= need, line_ + " (VER-192's case: 40 C, charging while playing, backlight full)")
            print(f"  this fails for any heat deflection under {need:.1f} C, whatever the sheet says for PLA; "
                  f"{hot - hdt:+.1f} C over U2 and {avg - hdt:+.1f} C on average against the stand-in itself")
        else:
            print("  " + line_ + " (room temperature, for the decision)")
    print(f"{len(row.bad)} over")
    return dn.result(not row.bad)


# ---- VER-200 ------------------------------------------------------------------------------------------------------------
def amp(seed=None):
    Q = quotes()
    tab, k = modes_table(Q)
    vdd = 4.5
    pd = 2 * vdd ** 2 / (math.pi ** 2 * 8.0)
    _, air = inside(tab["USB play"][1], T_AMB)
    print(f"U7 FM8002A bridged into 8 ohm at VDD {vdd} V (VSYS on USB): its worst dissipation 2 VDD^2 / (pi^2 R) = {pd:.3f} W "
          "(the bridge-tied load's peak, a textbook result, not from its datasheet)")
    for th in (128.0, 150.0, 160.0):
        print(f"  at {th:g} C/W: {air:.1f} + {pd * th:.1f} = {air + pd * th:.1f} C")
    th = STAND_SOIC8_THETA * (2.0 if seed else 1.0)
    tj = air + pd * th
    lim = Q["u7_tj"][0] - MARGIN
    print(f"fm8002a.pdf (on disk since 2026-10-28) gives TJ 150 C (fm8002a.txt:112) and says to design to the chip's theta JA "
          f"(64) without printing one: a JEDEC SOIC-8's {th:g} C/W is the stand-in")
    ok = tj <= lim
    print(("  ok    " if ok else "  FAIL  ") + f"U7 at {air:.1f} + {pd:.3f} x {th:g} = {tj:.1f} C against {Q['u7_tj'][0]:g} - "
          f"{MARGIN:g} = {lim:g} C")
    return dn.result(ok)


STAND_SOIC8_THETA = 160.0       # C/W: a JEDEC SOIC-8 on a low-k board, pessimistic (fm8002a.pdf prints no theta)
CHECKS = {"modes": (modes, True), "board": (board, True), "amp": (amp, True), "casemat": (casemat, True)}


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if "--list" in sys.argv or not args:
        print(__doc__)
        return 0
    fn, seed = CHECKS[args[0]]
    if "--seed" in sys.argv:
        if not seed:
            sys.exit(f"thermal: {args[0]} has no seed")
        print("SEEDED: a hotter part")
        return fn(seed)
    return fn()


if __name__ == "__main__":
    sys.exit(main())
