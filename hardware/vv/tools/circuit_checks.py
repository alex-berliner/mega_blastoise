#!/usr/bin/env python3
"""Circuit checks of classes 2, 3, 5, 7 and 12, from design.py, the pin reviews and the datasheets on disk.

    python3 hardware/vv/tools/circuit_checks.py <check>           # exit 1 when the check's limit fails, 2 BLOCKED
    python3 hardware/vv/tools/circuit_checks.py <check> --seed    # the same check on a seeded fault: must exit 1
    python3 hardware/vv/tools/circuit_checks.py --list

  appcircuit VER-198  each IC whose datasheet is on disk against its application circuit and pin rules
                      (U2, U3, U4's pins outside the core, U8, U9, the panel), and every constant the vv tools
                      cite as '<file>.txt:<line>' found on that line of its extract

Every datasheet number below is read through ds.quote(): the check fails if the cited line does not hold it.
A number from a datasheet that is not on disk comes from board/pin_review/<part>.json (which quotes the page)
and is printed 'not on disk' (VER-34 confirms it). --seed moves a design value in memory to where the check
must fail (README: a check that can be a script can fail). Nothing is written under hardware/ref_design.

Python 3 standard library, pdftotext, and pcbnew for the checks that read the routed board.
"""
import glob
import math
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import design_numbers as dn  # noqa: E402
import ds  # noqa: E402

R_TOL = dn.R_TOL


def load(seed):
    parts, ns = dn.design()
    for ref, change in (seed or {}).items():
        parts[ref] = dict(parts[ref], **change)
    return parts, ns


def val(parts, ref):
    v = parts[ref]["value"]
    return dn.farads(v) if parts[ref]["lib"] == "Device:C" else dn.ohms(v)


def nets_of(parts, ref):
    return set(parts[ref]["pins"].values())


def on(parts, net, kind=None):
    """Refs with a pin on net (kind: 'C' or 'R' for passives only)."""
    out = []
    for ref, p in parts.items():
        if net in p["pins"].values() and p["bom"] and (kind is None or p["lib"] == f"Device:{kind}"):
            out.append(ref)
    return sorted(out)


def between(parts, a, b, kind):
    return [r for r in on(parts, a, kind) if set(parts[r]["pins"].values()) == {a, b}]


class Rows:
    """Rows of a check: each PASS or FAIL, printed, counted."""

    def __init__(self):
        self.bad = []

    def __call__(self, good, text, sub=""):
        print(("  ok    " if good else "  FAIL  ") + (f"[{sub}] " if sub else "") + text)
        if not good:
            self.bad.append(text)
        return good


# ---- VER-198: application circuits ------------------------------------------------------------------------
SEED_APP = {"R64": {"pins": {"1": None, "2": "GND"}}}   # ILIM left open: 'Leaving ILIM unconnected disables all charging'


def appcircuit(seed=None):
    parts, ns = load(seed)
    row = Rows()
    u2 = parts["U2"]["pins"]

    def caps(net):
        return sum(val(parts, r) for r in between(parts, net, "GND", "C"))

    print("U2 BQ24073RGTR (bq24074.pdf, SLUS810N), Table 7-1 and section 8:")
    q = ds.quote("bq24074", 520, "1 μF to 10 μF")
    row(1e-6 <= caps("VBUS_C1") + caps("VBUS") <= 10e-6 * 1.001,
        f"IN bypass {caps('VBUS_C1') * 1e6:.1f} uF (C1 through R41) against '{q}' (bq24074.txt:520)", "power")
    q = ds.quote("bq24074", 533, "4.7-μF to 47-μF")
    row(4.7e-6 <= caps("VSYS") <= 47e-6, f"OUT bypass {caps('VSYS') * 1e6:.1f} uF ({', '.join(between(parts, 'VSYS', 'GND', 'C'))}) "
        f"against '4.7-uF to 47-uF' (bq24074.txt:533)", "power")
    ds.quote("bq24074", 501, "4.7-μF to 47-μF")
    row(4.7e-6 <= caps("VBAT") <= 47e-6, f"BAT bypass {caps('VBAT') * 1e6:.1f} uF (C3) against 4.7-47 uF (bq24074.txt:501)", "power")
    ds.quote("bq24074", 643, "RISET", "590", "8900")
    iset = [r for r in between(parts, "CHG_ISET", "GND", "R")]
    row(len(iset) == 1 and u2["16"] == "CHG_ISET" and 590 <= val(parts, iset[0]) <= 8900,
        f"ISET (pin 16) to VSS through {iset[0] if iset else 'nothing'} {parts[iset[0]]['value'] if iset else ''}: "
        "590-8900 ohm (bq24074.txt:643); 'Charging is disabled if ISET is left unconnected' (522)", "power")
    ds.quote("bq24074", 642, "RILIM", "1100", "8000")
    ds.quote("bq24074", 515, "Leaving ILIM unconnected disables all charging")
    ilim = between(parts, "CHG_ILIM", "GND", "R")
    row(len(ilim) == 1 and u2["12"] == "CHG_ILIM" and 1100 <= val(parts, ilim[0]) <= 8000,
        f"ILIM (pin 12) to VSS through {ilim[0] if ilim else 'nothing'}: 1100-8000 ohm (bq24074.txt:642), never open (515)",
        "power")
    ds.quote("bq24074", 558, "TD", "15")
    ds.quote("bq24074", 556, "Connect TD to VSS to enable")
    row(u2["15"] == "GND", f"pin 15 is TD on the '73 (bq24074.txt:558): at {u2['15']}, termination enabled "
        "('Connect TD to VSS to enable charger termination', 556)", "power")
    ds.quote("bq24074", 568, "Leave TMR unconnected to set the timers to the default")
    row(u2["14"] is None, "TMR (pin 14) open: the default safety timers (bq24074.txt:568)", "power")
    ds.quote("bq24074", 570, "10kΩ NTC thermistor")
    th = [r for r in on(parts, "CHG_TEMP") if r.startswith("TH")]
    row(u2["1"] == "CHG_TEMP" and len(th) == 1 and abs(dn.ohms(parts[th[0]]["value"]) - 10e3) < 1,
        f"TS (pin 1) to a 10 kohm NTC ({th[0] if th else 'none'} {parts[th[0]]['value'] if th else ''}): 'TS monitors a "
        "10kOhm NTC thermistor' (bq24074.txt:570); its window is VER-32's", "power")
    ds.quote("bq24074", 578, "500 mA. USB500 mode")
    ds.quote("bq24074", 512, "285 kΩ", "Do not leave EN1 or EN2 unconnected")
    ds.quote("bq24074", 848, "VIH", "1.4")
    r43 = val(parts, "R43") * (1 + R_TOL)
    en1 = 4.35 * dn.EN1_INTERNAL / (dn.EN1_INTERNAL + r43)
    row(u2["5"] == "GND" and u2["6"] == "CHG_EN1" and en1 >= 1.4,
        f"EN2 (pin 5) at {u2['5']}, EN1 (pin 6) from VBUS through R43 {parts['R43']['value']}: USB500, 'EN2 0, EN1 1: 500 mA' "
        f"(bq24074.txt:578); EN1 at IN's 4.35 V minimum {en1:.2f} V against VIH 1.4 V (848) with the ~285k pull-down (512)",
        "power")
    ds.quote("bq24074", 505, "CE is internally pulled down", "Do not leave CE unconnected")
    ce = between(parts, "CHG_OFF", "GND", "R")
    row(u2["4"] == "CHG_OFF" and ce and "CHG_OFF" in parts["U4"]["pins"].values(),
        f"CE (pin 4) on CHG_OFF from U4 with {ce[0]} {parts[ce[0]]['value']} to GND: never floating (bq24074.txt:505)", "power")
    ds.quote("bq24074", 509, "1kΩ-100kΩ resistor")
    ds.quote("bq24074", 536, "1-kΩ to 100-kΩ resistor")
    for pin, net, line in (("9", "CHG_STAT_N", 509), ("7", "CHG_PG_N", 536)):
        pu = between(parts, net, "+3V3", "R")
        row(u2[pin] == net and len(pu) == 1 and 1e3 <= val(parts, pu[0]) <= 100e3 * 1.001,
            f"{'CHG' if pin == '9' else 'PGOOD'} (pin {pin}) open drain to +3V3 through {pu[0] if pu else 'nothing'} "
            f"{parts[pu[0]]['value'] if pu else ''}: 1k-100k (bq24074.txt:{line})", "power")
    ds.quote("bq24074", 562, "The thermal pad must be connected to the same potential as the VSS pin")
    row(u2["17"] == "GND" and u2["8"] == "GND", "the thermal pad (17) and VSS (8) on GND (bq24074.txt:562, 572)", "power")

    print("U3 TLV75733PDBVR (tlv757p.pdf, SBVS322C):")
    ds.quote("tlv757p", 210, "Input capacitor", "1")
    ds.quote("tlv757p", 211, "Output capacitor", "1", "200")
    ds.quote("tlv757p", 1333, "Place input and output capacitors as close as possible")
    u3 = parts["U3"]["pins"]
    cin = between(parts, u3["1"], "GND", "C")
    cout = between(parts, u3["5"], "GND", "C")
    # COUT is every capacitor U3's output sees through JP1's bridge: 3V3_LDO and +3V3 (the module's 22 uF included)
    cout_all = cout + between(parts, "+3V3", "GND", "C")
    c_all = sum(val(parts, r) for r in cout_all)
    row(sum(val(parts, r) for r in cin) >= 1e-6, f"CIN on IN ({u3['1']}): {', '.join(cin)} {sum(val(parts, r) for r in cin) * 1e6:.0f} "
        "uF >= 1 uF (tlv757p.txt:210); its distance is checked below on the routed board", "power")
    row(1e-6 <= c_all <= 200e-6, f"COUT: {', '.join(cout)} on {u3['5']} and {len(cout_all) - len(cout)} more on +3V3 through JP1, "
        f"{c_all * 1e6:.1f} uF nominal, inside 1-200 uF (tlv757p.txt:211)", "power")
    ds.quote("tlv757p", 917, "Enable the device by forcing the EN pin to exceed VHI")
    en_pd = between(parts, u3["3"], "GND", "R")
    row(bool(en_pd), f"EN ({u3['3']}) held low by {', '.join(en_pd) or 'nothing'} when nothing drives it (tlv757p.txt:917)", "power")
    row(u3["4"] is None, "pin 4 NC on the DBV SOT-23-5 (IN 1, GND 2, EN 3, NC 4, OUT 5; tlv757p.txt:114, design.py U3)", "power")

    print("U8, U9, U11 SN74HC165PWR (sn74hc165.pdf, SCLS116):")
    ds.quote("sn74hc165", 157, "CLK", "15", "CLK INH")
    ds.quote("sn74hc165", 255, "Supply voltage", "2", "6")
    regs = sorted(r for r, p in parts.items() if p["mpn"] == "SN74HC165PWR")
    row(len(regs) == 3, f"three registers in the chain: {', '.join(regs)} (24 bits, pinmap.md Buttons)", "buttons")
    for u in regs:
        p = parts[u]["pins"]
        row(p["15"] == "GND", f"{u} CLK INH (pin 15) at {p['15']}: low enables the clock (sn74hc165.txt:157, 198)", "buttons")
        row(p["16"] == "+3V3" and p["8"] == "GND", f"{u} VCC (16) on +3V3, inside 2-6 V (sn74hc165.txt:255)", "buttons")
        row(p["10"] is not None, f"{u} SER (pin 10) at {p['10']}: driven or tied, not floating", "buttons")
        ins = [p[d] for d in ("11", "12", "13", "14", "3", "4", "5", "6")]

        def held(n):
            # tied to +3V3, a resistor or array to +3V3, or an open-drain line with its pull-up (PWR_BTN_N's R16)
            return n == "+3V3" or any(set(parts[r]["pins"].values()) >= {n, "+3V3"} for r in parts
                                      if parts[r]["lib"] in ("Device:R", "Device:R_Pack04") and parts[r]["bom"])
        free = [n for n in ins if not held(n)]
        row(not free, f"{u} D0-D7 each pulled up or tied high: no input floats with its contact open"
            + (f" (floating: {', '.join(free)})" if free else ""), "buttons")
    print("U4 ESP32-S3-WROOM-1U-N16R8 (esp32-s3-wroom-1.pdf, the module; esp32-s3-hw-design-guidelines.pdf):")
    u4 = parts["U4"]["pins"]
    ds.quote("esp32-s3-hw-design-guidelines", 338, "R = 10 kΩ and C = 1 μF")
    ds.quote("esp32-s3-hw-design-guidelines", 336, "CHIP_PU must not be left floating")
    en_r = between(parts, "EN", "+3V3", "R")
    en_c = between(parts, "EN", "GND", "C")
    row(u4["3"] == "EN" and len(en_r) == 1 and len(en_c) == 1 and abs(val(parts, en_r[0]) - 10e3) < 1
        and abs(val(parts, en_c[0]) - 1e-6) < 1e-9,
        f"EN (pin 3): {en_r[0] if en_r else 'nothing'} to +3V3 and {en_c[0] if en_c else 'nothing'} to GND, the RC delay "
        "'usually R = 10 kOhm and C = 1 uF' (esp32-s3-hw-design-guidelines.txt:338), never floating (336)", "mcu")
    ds.quote("esp32-s3-wroom-1", 2875, "22uF", "0.1uF")
    c3 = between(parts, "+3V3", "GND", "C")
    row(any(abs(val(parts, r) - 22e-6) < 1e-9 for r in c3) and any(abs(val(parts, r) - 0.1e-6) < 1e-9 for r in c3),
        "3V3 (pin 2): a 22 uF and a 0.1 uF on +3V3, as the module's peripheral schematic (esp32-s3-wroom-1.txt:2875); their "
        "distance to pin 2 is VER-204's", "mcu")
    ds.quote("esp32-s3-hw-design-guidelines", 874, "22/33 Ω")
    rr = [r for r in ("R21", "R22") if r in parts]
    row(len(rr) == 2 and all(22.0 - 1e-6 <= val(parts, r) <= 33.0 + 1e-6 for r in rr)
        and {parts["R21"]["pins"]["2"], parts["R22"]["pins"]["2"]} == {u4.get("14"), u4.get("13")},
        "USB D+ (pin 14, IO20) and D- (pin 13, IO19) each through a series resistor of 22-33 ohm (R21, R22 22 ohm; "
        "esp32-s3-hw-design-guidelines.txt:874); their distance to the module is VER-191's", "usb_debug")
    row(all(u4.get(str(p)) is None for p in (28, 29, 30)), "IO35-IO37 (pins 28-30) unconnected: the octal PSRAM's "
        "(esp32-s3-wroom-1.txt, the pin table's footnote; pinmap.md)", "mcu")
    row(all(u4.get(str(p)) == "GND" for p in (1, 40, 41)), "GND on pins 1, 40 and the 41 pad", "mcu")
    jtag = parts.get("J7", {}).get("pins", {})
    row({jtag.get("2"), jtag.get("3")} == {"EN", "BOOT"} and u4.get("27") == "BOOT",
        "J7's pads carry EN and BOOT (IO0, pin 27) with 3V3 and GND: the download mode by a pogo jig (no buttons)", "usb_debug")
    print("the panel ER-TFT035IPS-6 on J3 (er-tft035ips-6.pdf, its ILI9488), on its 8-bit 8080 bus since 39cc07f:")
    panel = ns["PANEL"]
    ds.quote("er-tft035ips-6", 718, "0", "1", "1", "8080 MCU 8-bit Bus Interface")
    row(panel[9] == "GND" and panel[8] == "LCD_3V3" and panel[7] == "LCD_3V3",
        "IM2 (pin 9) low, IM1 (8) and IM0 (7) high: '0 1 1 8080 MCU 8-bit Bus Interface D[7:0]' (er-tft035ips-6.txt:718)", "display")
    ds.quote("er-tft035ips-6", 667, "DB17-DB0", "Fix to DGND level when not in use")
    row(all(panel[p] == "GND" for p in range(11, 25)) and all(panel[32 - k] == f"LCD_D{k}" for k in range(8)),
        "VSYNC, HSYNC, DOTCLK, DE and DB17-DB8 (11-24) at GND ('Fix to DGND level when not in use', er-tft035ips-6.txt:659-667); "
        "DB7-DB0 (25-32) on LCD_D7-LCD_D0 (pins 15-32 are DB17-DB0, 667)", "display")
    ds.quote("er-tft035ips-6", 675, "RD", "Fix to IOVCC when not in use")
    row(panel[35] == "LCD_3V3", "RD (35) at the logic supply (er-tft035ips-6.txt:675)", "display")
    ds.quote("er-tft035ips-6", 669, "SDO", "Leave the pin open")
    ds.quote("er-tft035ips-6", 673, "Fix to DGND level when not in use")
    row(panel[33] is None and panel[34] == "GND" and panel[39] == "LCD_TE",
        "SDO (33) open, SDA (34) at GND, TE (39) to the module (er-tft035ips-6.txt:669, 673, 686)", "display")
    row(panel[36] == "LCD_WR" and panel[37] == "LCD_DC" and panel[38] == "GND" and panel[10] == "LCD_RST",
        "WRX (36), D/CX (37), RESET (10) from the module; CSX (38) tied low, always selected (the sheet says 'Fix to IOVCC when "
        "not in use', 683: it is in use, held selected)", "display")
    q747 = ds.quote("er-tft035ips-6", 747, "VCI", "2.5", "3.3")
    vmax = 3.3 * (1 + dn.VREG_TOL_85)
    row(vmax <= 3.3, f"VCI and VDDI (40-42) on LCD_3V3 = +3V3 through JP4: up to {vmax:.3f} V (TLV75733P +-1% to TJ 85 C, "
        f"tlv757p.txt:252; {3.3 * (1 + dn.VREG_TOL):.3f} V at +-1.5%, 255) against '{q747}' (er-tft035ips-6.txt:747-748): "
        "DEF-61, DEC-09 proposes accepting it", "display")
    print("J8 microSD (Hirose DM3AT-SF-PEJM5) and its bus (esp32-s3-hw-design-guidelines.txt:864):")
    ds.quote("esp32-s3-hw-design-guidelines", 864, "Please add pull-up resistors to the SDIO GPIO")
    for net in ("SD_CMD", "SD_D0", "SD_D1", "SD_D2", "SD_D3"):
        pu = between(parts, net, "+3V3", "R")
        row(len(pu) == 1 and 10e3 * 0.99 <= val(parts, pu[0]) <= 100e3, f"{net}: pull-up {pu[0] if pu else 'none'} "
            f"{parts[pu[0]]['value'] if pu else ''} (the guidelines' 1.3.12; the SD spec's 10-100 kohm, from memory)", "storage")
    row(between(parts, "SD_CD", "+3V3", "R") != [] and parts["J8"]["pins"].get("10") == "GND",
        "SD_CD pulled up, the detect switch to GND: low with a card in (normally open, hirose_dm3_catalog_C114218.txt:27)", "storage")
    sdc = between(parts, "SD_VDD", "GND", "C")
    row(sum(val(parts, r) for r in sdc) >= 1e-6 and parts["J8"]["pins"].get("4") == "SD_VDD",
        f"the card's VDD (pin 4) on SD_VDD with {', '.join(sdc)} ({sum(val(parts, r) for r in sdc) * 1e6:.1f} uF) beside it", "storage")
    # the routed board: CIN's distance
    pcb, b = dn.board()
    mm = pcb.ToMM
    fp = {f.GetReference(): f for f in b.GetFootprints()}
    vin = next(p for p in fp["U3"].Pads() if p.GetNumber() == "1")
    dist = min(math.hypot(mm(p.GetPosition().x - vin.GetPosition().x), mm(p.GetPosition().y - vin.GetPosition().y))
               for r in cin for p in fp[r].Pads() if p.GetNetname().split("/")[-1] == u3["1"])
    row(dist <= 12.7, f"the nearest CIN pad is {dist:.1f} mm from U3's IN: 'as close as possible' (tlv757p.txt:1333) gives no "
        "number; read as the register's 0.5 in (12.7 mm, the RT9013's own figure before 39cc07f), pad centre to pad centre",
        "power")
    quotes(row)
    print(f"{len(row.bad)} failing")
    return dn.result(not row.bad)


# ---- the quoted-constant audit (part of VER-198) -------------------------------------------------------------
# 39cc07f: any '<name>.txt:<line>' whose <name>.pdf is in hardware/ref_design/datasheets (the ESP32-S3's, the
# TLV757P's, the MCP6001's and the rest the register reads now), not a fixed list of six
CITE = re.compile(r"(?<![\w.-])([a-z0-9][a-z0-9_-]*)\.txt:(\d+)(?:-(\d+))?((?:,\s*\d+)*)")
DERIVED = {"I_TS_TOL": "the tolerance (78 - 72) / 2 / 75 of the cited band, not a printed number",
           "I_PATH": "VER-13's own test current; the cite is for the 475 mA USB500 limit it sits above",
           "I_LEAK": "a stand-in, 20 x the cited 50 nA",
           "I_3V3_PEAK": "the cited 355 mA plus the 150 mA average load: a sum, not a printed number",
           "U3_IEN": "a stand-in; the cited line gives 10 nA typical",
           "OPAMP_IB_MAX": "a stand-in, 50 x the cited 19 pA", "OPAMP_DRIFT": "a stand-in, 2.5 x the cited 2 uV/C",
           "CLAMP_V": "an estimate; the cited lines give the input's limits", "ESP_VOH": "0.8 x VDD at 3.3 V, the cited formula",
           "U7_ISD": "u7.json's figure; fm8002a.txt:123 prints it, VER-34 compares",
           "ESP_PWM_CLK": "the APB clock design.py and pinmap.md name; no datasheet line cited",
           "V3V3": "3.3 V x (1 + the cited 1.5%), a product", "VF_DECADE": "320 - 240 mV, the difference of the two cited lines",
           "STAND_ESP_USB_C": "a stand-in; the cited line gives a GPIO's 2 pF"}


def numbers_near(name, lines):
    out = []
    t = ds.text(name)
    for ln in lines:
        for k in range(max(0, ln - 2), min(len(t), ln + 1)):
            for m in re.finditer(r"(?<![\w.])(\d+(?:\.\d+)?)", t[k].replace(",", "")):
                out.append(float(m.group(1)))
    return out


def same(x, y):
    return abs(x - y) <= 1e-9 * max(abs(x), abs(y), 1e-30)


def quotes(row):
    """Every 'NAME = value  # ... (file.txt:line)' constant in the vv tools: each number of the value is printed on
    the cited line (or the line before it), at some power of 1000 (6.5e-6 A is '6.5' in a uA column)."""
    print("constants the vv tools cite by line, against the extract's line:")
    n = 0
    for path in sorted(glob.glob(os.path.join(HERE, "*.py"))):
        for k, line in enumerate(open(path, encoding="utf-8"), 1):
            m = re.match(r"^([A-Z][A-Z_0-9]*(?:\s*,\s*[A-Z][A-Z_0-9]*)*)\s*=\s*([^#]+)#(.*)$", line)
            if not m:
                continue
            cites = list(CITE.finditer(m.group(3)))
            if not cites:
                continue
            names = [s.strip() for s in m.group(1).split(",")]
            try:
                value = eval(m.group(2), {"__builtins__": {}}, {"dn": dn, "math": math})
            except Exception:
                continue
            vals = list(value) if isinstance(value, (tuple, list)) else [value]
            if len(names) > 1:
                vals = [value] if not isinstance(value, tuple) else list(value)
            for name, v in zip(names if len(names) == len(vals) else names * len(vals), vals):
                if not isinstance(v, (int, float)) or name in DERIVED:
                    continue
                c = cites[0]
                if not ds.have(c.group(1)):
                    row(False, f"{os.path.basename(path)}:{k} {name} cites {c.group(1)}.txt, whose PDF is not on disk")
                    continue
                lines = [int(c.group(2))] + ([int(c.group(3))] if c.group(3) else []) + \
                    [int(x) for x in re.findall(r"\d+", c.group(4) or "")]
                if c.group(3):
                    lines = list(range(int(c.group(2)), int(c.group(3)) + 1))
                near = numbers_near(c.group(1), lines)
                hit = any(same(v * 10 ** (3 * e), x) for e in range(-5, 6) for x in near)
                n += 1
                row(hit, f"{os.path.basename(path)}:{k} {name} = {v:g} on {c.group(1)}.txt:{','.join(map(str, lines))}"
                    + ("" if hit else ": NOT on the cited line"))
    for name, why in DERIVED.items():
        print(f"  skipped {name}: {why}")
    print(f"  {n} cited numbers read")


# ---- VER-203: the ESP32-S3 module's pins (since 39cc07f; VER-15 read the RP2350's GPIO mux) --------------------------
MOD_TABLE = (629, 688)        # esp32-s3-wroom-1.txt: Table 3-1, the module's pin definitions (two pages)
GLITCH = (958, 997)           # esp32-s3.txt: Table 2-2, the power-up glitches
ADC_NETS = ("CC1_SENSE", "CC2_SENSE", "VBAT_SENSE")
USB_NETS = {"USB_DM_MCU": "USB_D-", "USB_DP_MCU": "USB_D+"}
# outputs that switch a load or the power: a high-level glitch at power-up would switch it (Table 2-2)
LOAD_NETS = ("PWR_HOLD", "AMP_EN", "CHG_OFF", "BL_PWM")
SEED_GPIO = {"esp": {}, "parts": {"R37": {"pins": {"1": "AMP_EN", "2": "+3V3"}}}}   # AMP_EN (IO45) pulled up: VDD_SPI 1.8 V


def module_table():
    """{module pin: (name, gpio or None, set of functions)} from esp32-s3-wroom-1.txt's Table 3-1."""
    t = ds.text("esp32-s3-wroom-1")
    out = {}
    for k in range(MOD_TABLE[0], MOD_TABLE[1] + 1):
        m = re.match(r"^\s+(\w+)\s*[abc]?\s+(\d{1,2})\s+(\S+)(?:\s+(.*))?$", t[k - 1])
        if not m:
            continue
        name, no, fn = m.group(1), int(m.group(2)), (m.group(4) or "")
        # a function list the layout wrapped round the name: its first half on the line before, its rest after
        prev = t[k - 2].strip() if k >= 2 else ""
        if prev.startswith("RTC_GPIO") and not re.match(r"^\w+\s+\d", prev):
            fn = (prev + " " + fn).strip()
            nxt = t[k].strip() if k < len(t) else ""
            if nxt and not re.match(r"^\w+\s+\d", nxt):
                fn = fn.rstrip(",") + ", " + nxt
        fns = {s.strip() for s in re.split(r",\s*", fn) if s.strip()}
        g = next((int(f[4:]) for f in fns if re.fullmatch(r"GPIO\d+", f)), None)
        out[no] = (name, g, fns)
    return out


def glitches():
    """{GPIO number: set of glitch kinds} from esp32-s3.txt's Table 2-2 (XTAL_32K_P/N are GPIO15/16)."""
    t = ds.text("esp32-s3")
    out, cur = {}, None
    for k in range(GLITCH[0], GLITCH[1] + 1):
        ln = t[k - 1]
        m = re.match(r"^\s+(GPIO\d+|XTAL_32K_[PN])\b(.*)$", ln)
        if m:
            cur = {"XTAL_32K_P": 15, "XTAL_32K_N": 16}.get(m.group(1)) or int(m.group(1)[4:])
            rest = m.group(2)
        else:
            rest = ln
        for kind in ("Low-level glitch", "High-level glitch", "Pull-down glitch", "Pull-up glitch"):
            if kind in rest and cur is not None:
                out.setdefault(cur, set()).add(kind)
        # a pin whose glitch rows straddle its name (GPIO18-20): the row above the name belongs to it too
        nxt = t[k] if k < len(t) else ""
        mn = re.match(r"^\s+(GPIO\d+)\s*$", nxt)
        if mn and any(kind in ln for kind in ("glitch",)) and not m:
            out.setdefault(int(mn.group(1)[4:]), set()).update(
                {kd for kd in ("Low-level glitch", "High-level glitch", "Pull-down glitch") if kd in ln})
    return out


def gpio(seed=None):
    parts, ns = load(seed.get("parts") if seed else None)
    esp = dict(ns["ESP32"])
    if seed:
        esp.update(seed["esp"])
    row = Rows()
    if not parts["U4"]["value"].startswith("ESP32-S3-WROOM-1"):
        sys.exit("circuit_checks gpio: U4 is " + parts["U4"]["value"] + ", not an ESP32-S3-WROOM-1 module")
    ds.quote("esp32-s3-wroom-1", 627, "Table 3-1. Pin Definitions")
    tab = module_table()
    row(sorted(tab) == list(range(1, 42)), f"Table 3-1 read for pins 1-41 (esp32-s3-wroom-1.txt:{MOD_TABLE[0]}-{MOD_TABLE[1]}): "
        f"{len(tab)} rows")
    gp = {no: g for no, (_, g, _) in tab.items()}
    print("  each module pin, its GPIO and its net (design.py ESP32):")
    for no in sorted(esp):
        name, g, fns = tab.get(no, ("?", None, set()))
        print(f"    pin {no:2d} {name:5s} GPIO{g if g is not None else '-':<3} {esp[no] or '(open)'}")
    for net in ADC_NETS:
        pins = [no for no, n in esp.items() if n == net]
        fns = tab[pins[0]][2] if pins else set()
        adc1 = sorted(f for f in fns if f.startswith("ADC1_"))
        row(bool(adc1), f"{net} on pin {pins[0] if pins else '?'} (GPIO{gp.get(pins[0]) if pins else '?'}): "
            + (f"{adc1[0]}" if adc1 else "NOT an ADC1 channel") + " (ADC2 cannot be used with Wi-Fi, esp32-s3.txt:3415)")
    ds.quote("esp32-s3", 3415, "ADC2_CH", "cannot be used with Wi-Fi")
    for net, fn in USB_NETS.items():
        pins = [no for no, n in esp.items() if n == net]
        row(bool(pins) and fn in tab[pins[0]][2], f"{net} on pin {pins[0] if pins else '?'}: {fn} (Table 3-1)")
    ds.quote("esp32-s3-wroom-1", 692, "IO35, IO36, and IO37 are connected to the Octal SPI PSRAM")
    psram = [no for no, (_, g, _) in tab.items() if g in (35, 36, 37)]
    row(all(esp.get(no) is None for no in psram), f"IO35-IO37 (pins {', '.join(map(str, psram))}) unconnected: the R8's octal "
        "PSRAM (esp32-s3-wroom-1.txt:691-693)")
    ds.quote("esp32-s3-wroom-1", 694, "ESP32-S3R16V")
    row(parts["U4"]["mpn"].endswith("N16R8"), f"IO47 and IO48 run at 3.3 V: only the R16V's VDD_SPI is 1.8 V "
        f"(esp32-s3-wroom-1.txt:694-696); U4 is {parts['U4']['mpn']} (LCD_D7, LCD_WR on them)")
    # strapping pins (Table 4-1, 4-3, 4-4, 4-5)
    ds.quote("esp32-s3-wroom-1", 751, "GPIO0", "Weak pull-up", "1")
    ds.quote("esp32-s3-wroom-1", 753, "GPIO45", "Weak pull-down", "0")
    ds.quote("esp32-s3-wroom-1", 754, "GPIO46", "Weak pull-down", "0")
    ds.quote("esp32-s3-wroom-1", 816, "SPI Boot", "1")
    ds.quote("esp32-s3-wroom-1", 849, "3.3 V", "0")
    ds.quote("esp32-s3-wroom-1", 876, "does not have any internal pull resistors")
    pin_of = {g: no for no, g in gp.items() if g is not None}

    def strap_level(g):
        """the level the board holds a strapping pin at reset: 0, 1, 'internal 1/0' or 'driven by <ref>'"""
        net = esp.get(pin_of[g])
        if net is None:
            return None, "open"
        pd = between(parts, net, "GND", "R")
        pu = between(parts, net, "+3V3", "R")
        drv = [r for r, p in parts.items() if r != "U4" and p["bom"] and net in p["pins"].values()
               and p["lib"].startswith("74xx")]
        if pd and not pu:
            return 0, f"{net}: {pd[0]} {parts[pd[0]]['value']} to GND"
        if pu and not pd:
            return 1, f"{net}: {pu[0]} {parts[pu[0]]['value']} to +3V3"
        if drv:
            return "driven", f"{net}: driven by {drv[0]}'s output"
        return "internal", f"{net}: no resistor on the board (the pin's internal default holds it)"
    lv0, why0 = strap_level(0)
    row(lv0 in ("internal", 1), f"GPIO0 (pin {pin_of[0]}) at reset: {why0}: 1 = SPI boot (esp32-s3-wroom-1.txt:751, 816); J7's "
        "BOOT pad pulls it low for the download mode")
    lv45, why45 = strap_level(45)
    row(lv45 in (0, "internal"), f"GPIO45 (pin {pin_of[45]}) at reset: {why45}: 0 = VDD_SPI at 3.3 V for the N16R8's 3.3 V "
        "flash and PSRAM (esp32-s3-wroom-1.txt:849; only the N16R16VA is 1.8 V, 170)")
    lv46, why46 = strap_level(46)
    row(lv46 in (0, "internal"), f"GPIO46 (pin {pin_of[46]}) at reset: {why46}: 0, so GPIO0 low gives the joint download mode "
        "(esp32-s3-wroom-1.txt:817); with GPIO0 high any value boots from flash (816)")
    lv3, why3 = strap_level(3)
    row(lv3 != "open", f"GPIO3 (pin {pin_of[3]}) at reset: {why3}: not high impedance (esp32-s3-wroom-1.txt:876); its level is "
        "ignored while the JTAG eFuses are unburnt (893): firmware must never burn EFUSE_STRAP_JTAG_SEL")
    # power-up glitches on the load-switching outputs
    ds.quote("esp32-s3", 958, "Table 2-2. Power-Up Glitches on Pins")
    gl = glitches()
    for net in LOAD_NETS:
        pins = [no for no, n in esp.items() if n == net]
        g = gp.get(pins[0]) if pins else None
        kinds = gl.get(g, set())
        row("High-level glitch" not in kinds and "Pull-up glitch" not in kinds,
            f"{net} on GPIO{g}: power-up glitches {', '.join(sorted(kinds)) or 'none'} (esp32-s3.txt:958-1006): "
            + ("never driven high before firmware" if "High-level glitch" not in kinds else "a HIGH glitch would switch its load"))
    for g in sorted(gl):
        if "High-level glitch" in gl[g]:
            net = esp.get(pin_of.get(g))
            print(f"  GPIO{g} ({net or 'open'}) has a 60 us high-level glitch at power-up (esp32-s3.txt:990-1006): "
                  + {"SR_CLK_MCU": "one stray clock edge before the first load: harmless (SR_PL loads first)",
                     "USB_DM_MCU": "USB, the ROM's own", "USB_DP_MCU": "USB, the ROM's own"}.get(net, "check its load"))
    print(f"{len(row.bad)} failing")
    return dn.result(not row.bad)


# ---- VER-36: the pin reviews' open questions and ratings -----------------------------------------------------------
PR = os.path.join(dn.BOARD, "pin_review")
APPROVALS = os.path.join(os.path.dirname(HERE), "releases", "APPROVALS.md")
FLAG = re.compile(r"OPEN QUESTION|UNKNOWN|UNVERIFIED")
SEED_PR = {"R43": {"value": "100k", "lcsc": "C25741", "mpn": "0402WGF1003TCE"}}   # a BOM value the review did not see


def signed_approvals():
    """The A-ids check_trace itself counts: table rows of releases/APPROVALS.md, committed, in a commit signed with the
    key README pins (2026-10-28: this check used to count every 'A-NNN' in the file, its examples included, as signed)."""
    import check_trace as ct
    n0, w0 = len(ct.ERRORS), len(ct.WARNINGS)
    rows, _, _ = ct.parse_approvals()
    del ct.ERRORS[n0:], ct.WARNINGS[w0:]            # its own report is check_trace's, not this check's
    return {r["id"] for r in rows if r["committed"]}, len(rows)


def pinreview(seed=None):
    parts, _ = load(seed)
    row = Rows()
    signed, n_rows = signed_approvals()
    kinds = {}

    def fail(kind, text):
        kinds[kind] = kinds.get(kind, 0) + 1
        row(False, f"[{kind}] {text}")

    def waived(entry):
        w = entry.get("waiver", "")
        cites = set(re.findall(r"\bA-\d{3}\b", w))
        return bool(w) and bool(cites & signed), w

    files = sorted(glob.glob(os.path.join(PR, "*.json")))
    for f in files:
        name = os.path.basename(f)
        d = __import__("json").load(open(f, encoding="utf-8"))
        if name == "passives.json":
            for p in d["passives"]:
                ref = p["ref"]
                if ref not in parts:
                    fail("orphan", f"passives.json {ref}: not in design.py")
                    continue
                q = parts[ref]
                if not q["bom"]:
                    continue
                for k in ("lcsc", "mpn"):
                    if p.get(k) and q.get(k) and p[k] != q[k]:
                        fail("identity", f"passives.json {ref} {k} {p[k]} against design.py's {q[k]} (value {q['value']})")
                rv, nv = p.get("rating_v"), p.get("net_max_v")
                if rv is not None and nv is not None and rv < 1.25 * nv:
                    fail("rating", f"passives.json {ref}: rated {rv} V against 1.25 x its net's {nv} V")
                basis = p.get("net_max_basis", "")
                # the limit the basis states outright: '(3.3)' or '<= 5.5'; a formula or a prose figure is not compared
                volts = [float(x) for x in re.findall(r"\((\d+(?:\.\d+)?)\)|<=\s*(\d+(?:\.\d+)?)", basis) for x in x if x]
                if nv is not None and volts and not any(abs(x - nv) < 1e-6 for x in volts):
                    fail("basis", f"passives.json {ref}: net_max_basis {basis!r} states {volts[0]:g} V, net_max_v is {nv} V")
                if p.get("ok") is False or (p.get("ok") is None and not waived(p)[0]):
                    fail("ok", f"passives.json {ref}: ok {p.get('ok')}" + (f", waiver not cited to a signed A-line: "
                                                                           f"{p.get('waiver')!r}" if p.get("waiver") else ""))
            continue
        refs = d.get("refs") or ([d["ref"]] if d.get("ref") else [])
        live = [r for r in refs if r in parts]
        for r in refs:
            if r not in parts:
                fail("orphan", f"{name}: {r} is not in design.py")
        if name == "panel.json":
            live = ["J3"]                       # the panel is no design.py part: it plugs into J3, and its ratings count
        if not live:
            continue
        for r in live if name != "panel.json" else ():
            for k in ("lcsc", "mpn"):
                # a review's mpn may carry its maker and LCSC number after the part number: the part number must lead it
                if d.get(k) and parts[r].get(k) and not str(d[k]).startswith(parts[r][k]):
                    fail("identity", f"{name} {r} {k} {d[k]!r} against design.py's {parts[r][k]!r}")
        if FLAG.search(__import__("json").dumps(d.get("datasheet", ""))):
            fail("source", f"{name}: its datasheet entry says {FLAG.search(__import__('json').dumps(d['datasheet'])).group(0)}")
        for e in d.get("ratings", []):
            what = e.get("what") or e.get("param")
            if e.get("ok") is False or (e.get("ok") is None and not waived(e)[0]):
                fail("rating", f"{name} '{what}': ok {e.get('ok')}" + (f"; waiver {e.get('waiver')!r} cites no signed A-line"
                                                                     if e.get("waiver") else ""))
            elif FLAG.search(" ".join(str(v) for v in e.values())):
                fail("text", f"{name} '{what}': ok but says {FLAG.search(' '.join(str(v) for v in e.values())).group(0)}")
        for e in d.get("pins", []):
            txt = " ".join(str(e.get(k, "")) for k in ("issue", "requirement", "quote"))
            if e.get("ok") is False and not waived(e)[0]:
                fail("pin", f"{name} pin {e.get('pin')}: ok false" + (f"; waiver {e.get('waiver')[:80]!r}... cites no signed A-line"
                                                                     if e.get("waiver") else f": {e.get('issue', '')[:90]}"))
            elif e.get("ok") is None and not waived(e)[0]:
                fail("pin", f"{name} pin {e.get('pin')}: ok null: {e.get('issue', '')[:90]}")
            elif e.get("ok") and FLAG.search(txt):
                fail("text", f"{name} pin {e.get('pin')}: ok but says {FLAG.search(txt).group(0)}: {e.get('issue', '')[:90]}")
    have = {os.path.basename(f)[:-5].upper() for f in files}
    for ref, p in sorted(parts.items()):
        ic = re.match(r"(U|Q|D|J|X|L)\d+$", ref) and p["bom"]
        if ic and not any(ref in (__import__("json").load(open(f)).get("refs") or [__import__("json").load(open(f)).get("ref")])
                          for f in files if not f.endswith("passives.json")):
            fail("missing", f"{ref} {p['mpn']}: no pin_review file names it")
    print("failures by kind: " + ", ".join(f"{k} {n}" for k, n in sorted(kinds.items())) if kinds else "no failures")
    print(f"A-lines check_trace counts in releases/APPROVALS.md: {len(signed)} of {n_rows} table rows (committed and signed "
          "with README's pinned key; the header's examples are not rows); a waiver counts only when it cites one")
    return dn.result(not row.bad)


# ---- VER-149: the firmware's REF-A pin file against design.py ------------------------------------------------------
FW = "crates/mega_blastoise_fw/src"
SEED_FW = None


def fwpins(seed=None):
    import subprocess
    _, ns = load(None)
    ref = next((a.split("=", 1)[1] for a in sys.argv if a.startswith("--ref=")), "HEAD")
    row = Rows()
    git = ["git", "-C", dn.ROOT]
    names = subprocess.run(git + ["ls-tree", "-r", "--name-only", ref, "--", FW], capture_output=True, text=True).stdout.split()
    # 39cc07f: design.py maps the ESP32-S3 module's pins (ESP32), the module's Table 3-1 gives each pin's GPIO
    tab = module_table()
    gmap = {tab[no][1]: net for no, net in ns["ESP32"].items() if net and tab.get(no, (0, None))[1] is not None}
    print(f"design.py's GPIO map (module pins through esp32-s3-wroom-1.txt's Table 3-1): {len(gmap)} nets on GPIO "
          + ", ".join(f"{n}={v}" for n, v in sorted(gmap.items())))
    pinfile = f"{FW}/board_ref_a.rs"
    if pinfile not in names:
        row(False, f"{pinfile} does not exist at {ref}: the firmware has no generated pin file for the ESP32-S3 board "
                   "(board/tools/pinmap.py writes docs/pinmap.md only)")
    else:
        text = subprocess.run(git + ["show", f"{ref}:{pinfile}"], capture_output=True, text=True).stdout
        got = {m.group(2): m.group(1) for m in re.finditer(r"pub const (\w+)\s*:\s*\w+\s*=\s*(\d+)\s*;", text)}
        for n, net in sorted(gmap.items()):
            row(str(n) in got and got[str(n)].lower().replace("_", "") == net.lower().replace("_", ""),
                f"GPIO{n} {net}: the pin file says {got.get(str(n), 'nothing')}")
    lit = []
    for f in names:
        if re.search(r"ref", os.path.basename(f)) and f.endswith(".rs") and f != pinfile:
            text = subprocess.run(git + ["show", f"{ref}:{f}"], capture_output=True, text=True).stdout
            lit += [(f, m.group(0)) for m in re.finditer(r"\bPIN_\d+\b", text)]
    row(not lit, f"literal PIN_n in the board_ref code at {ref}: {len(lit)}" + (": " + ", ".join(sorted({f'{os.path.basename(a)} {b}' for a, b in lit}))[:300] if lit else ""))
    return dn.result(not row.bad)


# ---- VER-31: ratings and derating per part and mode; the 3V3 budget ----------------------------------------------
MODES = ("off", "cell", "usb", "charging")
VBUS_MAX = 5.25                   # V: a USB 2.0 port's VBUS maximum (USB 2.0 Table 7-7, from memory: README's note)
CC_MAX = 2.18                     # V: CC at a sink's Rd with a 3 A source (Type-C vRd-3.0 maximum, from memory)
USB_DATA_MAX = 3.6                # V: a full-speed line's high level maximum (USB 2.0 7.1.1, from memory)
V3V3 = 3.3 * (1 + dn.VREG_TOL)    # V: the TLV75733P's +1.5% over TJ -40 to 125 C (design_numbers.VREG_TOL, tlv757p.txt:255)
# the 3V3 budget since 39cc07f: the module from its own tables, the rest stand-ins, each printed as one
ESP_CPU = 107.9e-3                # A: modem-sleep, 240 MHz, dual core, 128-bit, all peripheral clocks on (esp32-s3-wroom-1.txt:1741)
ESP_RX = 97e-3                    # A: Wi-Fi RX, 802.11n HT40, peak, CPU idle (esp32-s3-wroom-1.txt:1670)
ESP_TX = 355e-3                   # A: Wi-Fi TX, 802.11b 1 Mbps at 20.5 dBm, peak (esp32-s3-wroom-1.txt:1662)
ESP_IVDD = 0.5                    # A: the current the supply must deliver, minimum (esp32-s3-wroom-1.txt:1597)
STAND_PSRAM = 20e-3               # A: the octal PSRAM and flash at the frame's refresh (the module's tables omit it: 1698)
STAND_SD = 100e-3                 # A: a microSD card writing (the SD spec's class figures, from memory; no card sheet on disk)
STAND_PANEL_LOGIC = 20e-3         # A: the panel's VCI and VDDI together (er-tft035ips-6.pdf prints no current)
STAND_OPAMP_IQ = 0.17e-3          # A: an MCP600x amplifier's quiescent current, maximum (mcp6001.txt:160)
STAND_BUS_C = 30e-12              # F: one panel bus line's load, track and the panel's input
STAND_BUS_HZ = 12.5e6             # Hz: a data line toggling at half the 25 MHz write clock (pinmap.md's LCD_CAM limit)
# connectors' current ratings: catalogue figures, stand-ins (J8's is Hirose's own line, VER-34 confirms the rest)
STAND_CONN_A = {"J1": 5.0, "J2": 1.0, "J4": 1.0, "J8": 0.5}


def loads_3v3(parts, wifi="avg"):
    """[(name, amps, why)]: the 3V3 load with Wi-Fi on. wifi 'avg': listening (RX) on a busy CPU, the steady load the
    heat budget uses; 'peak': a TX burst instead, the load U3 and the decoupling must carry for its duration."""
    pulls = sum(V3V3 / val(parts, r) for r in ("R42", "R44")) + 16 * V3V3 / 47e3 + 4 * V3V3 / 47e3 \
        + 2 * V3V3 / 100e3 + 6 * V3V3 / 10e3 + V3V3 / 103.74e3
    radio = (ESP_TX, "TX burst 355 mA peak (esp32-s3-wroom-1.txt:1662)") if wifi == "peak" else \
        (ESP_RX, "RX 97 mA peak (esp32-s3-wroom-1.txt:1670)")
    return [("U4 CPU at 240 MHz, both cores", ESP_CPU, "107.9 mA (esp32-s3-wroom-1.txt:1741), all peripheral clocks on"),
            ("U4 Wi-Fi", radio[0], radio[1]),
            ("U4's PSRAM and flash", STAND_PSRAM, "stand-in (the module's tables leave PSRAM out, 1698)"),
            ("J8, a microSD card writing", STAND_SD, "stand-in, 100 mA"),
            ("the panel's VCI and VDDI", STAND_PANEL_LOGIC, "stand-in: the panel prints none"),
            ("U6, U10", 3 * STAND_OPAMP_IQ, "170 uA a channel, maximum (mcp6001.txt:160)"),
            ("U8, U9, U11", 3 * 80e-6, "ICC 80 uA at 6 V, the SN74HC165's maximum (not quoted), each"),
            ("pull-ups and pull-downs driven against", pulls,
             "R42 and R44 driven high, 20 x 47k, 2 x 100k, the SD bus's 6 x 10k held low, R60+R61"),
            ("the panel bus at its write clock", 9 * STAND_BUS_C * V3V3 * STAND_BUS_HZ, "C V f on D0-D7 and WR, stand-in 30 pF")]
SEED_RATINGS = {"R38": {"value": "330", "lcsc": "C25104", "mpn": "0402WGF3300TCE"}}   # VBUS's bleeder at 330 ohm: 84 mW


def net_vmax(parts, net, mode, hot):
    """The highest steady voltage on net in a mode (hot: the hot-plug peak on VBUS, a transient)."""
    usb = mode in ("usb", "charging")
    on = mode != "off"
    vsys = dn.VBAT_REG[1] if not usb else 4.5
    vbus = VBUS_MAX if usb else 0.0
    r = lambda ref: val(parts, ref)  # noqa: E731
    table = {
        "VBUS": vbus, "VBUS_C1": vbus, "VBUS_EN": vbus, "VSYS": vsys, "AMP_VDD": vsys, "LED_A": vsys,
        "VBAT": dn.VBAT_REG[1], "CELL_P": dn.VBAT_REG[1], "VBAT_DIV": dn.VBAT_REG[1] / 2,
        "CHG_EN1": vbus * dn.EN1_INTERNAL / (dn.EN1_INTERNAL + r("R43") * (1 - R_TOL)),
        "PWR_EN": max(vsys if on else 0, vbus * r("R15") / (r("R15") + r("R14"))),
        "PWR_BTN_RAW": vsys, "P2_START_RAW": vsys, "LED_K1": vsys, "LED_K2": vsys, "SPK_P": vsys, "SPK_N": vsys,
        "AMP_SHDN": vsys, "AMP_BYP": vsys / 2, "AMP_INN": vsys / 2, "AMP_CI": vsys / 2,
        "CC1": CC_MAX if usb else 0, "CC2": CC_MAX if usb else 0, "USB_DP": USB_DATA_MAX if usb else 0,
        "USB_DM": USB_DATA_MAX if usb else 0, "+1V1": 1.16 if on else 0,
        "CHG_TEMP": 4.5 if usb else 0, "CHG_ISET": 4.5 if usb else 0, "CHG_ILIM": 4.5 if usb else 0,
        "BL_REF": V3V3 * r("R61") / (r("R60") + r("R61")) if on else 0,
    }
    if net in table:
        return table[net]
    return V3V3 if on else 0.0          # every other net is on +3V3's domain: off with the LDO


def ratings(seed=None):
    parts, ns = load(seed)
    row = Rows()
    import json
    pas = {p["lcsc"]: p["db_description"] for p in json.load(open(os.path.join(PR, "passives.json")))["passives"]
           if p.get("lcsc") and p.get("db_description")}
    sys.path.insert(0, HERE)
    import paper_checks as pc
    hot = max(pc.hotplug_peak(5.25, rc, lc, rr, val(parts, "C1") * sh)
              for rc in pc.STAND_CABLE["r"] for lc in pc.STAND_CABLE["l"] for sh in pc.STAND_C1_EFF
              for rr in (dn.ohms(parts["R41"]["value"]) * (1 - R_TOL), dn.ohms(parts["R41"]["value"]) * (1 + R_TOL)))
    print(f"modes {', '.join(MODES)}, plus hot plug (VBUS up to {hot:.2f} V, paper_checks.py hotplug's worst corner) and "
          "unplug (U2's battery FET takes VSYS from OUT: no node rises above its steady value, bq24074.txt:530)")
    print(f"VBUS {VBUS_MAX} V, CC {CC_MAX} V, D+/D- {USB_DATA_MAX} V: USB figures from memory (README: clauses to check); "
          f"+3V3 up to {V3V3:.3f} V")
    vmax = {n: max(net_vmax(parts, n, m, hot) for m in MODES) for n in ns["nets"]()}
    vmax_hot = dict(vmax, VBUS=hot, VBUS_C1=hot, VBUS_EN=hot,
                    CHG_EN1=hot * dn.EN1_INTERNAL / (dn.EN1_INTERNAL + val(parts, "R43") * (1 - R_TOL)))
    vmax_hot["PWR_EN"] = max(vmax["PWR_EN"], hot * val(parts, "R15") / (val(parts, "R15") + val(parts, "R14")))
    # ICs: (ref, pins, recommended maximum, absolute maximum, source); a named exception of REQ-PWR-14 is printed as one
    q = ds.quote
    q("bq24074", 634, "6.4"); q("bq24074", 595, "28"); q("bq24074", 596, "BAT", "5"); q("bq24074", 599, "7")
    q("tlv757p", 206, "Input voltage", "5.5"); q("tlv757p", 178, "6.0"); q("tlv757p", 179, "Enable voltage", "6.0")
    q("tlv757p", 208, "Enable voltage", "5.5"); q("tlv757p", 209, "Output current", "1")
    q("esp32-s3-wroom-1", 1586, "VDD33", "3.6"); q("esp32-s3-wroom-1", 1596, "VDD33", "3.0", "3.6")
    q("esp32-s3-wroom-1", 1597, "0.5"); q("esp32-s3", 3746, "VIH", "VDD 1 + 0.3")
    q("sn74hc165", 255, "6"); q("sn74hc165", 233, "7"); q("er-tft035ips-6", 747, "3.3"); q("er-tft035ips-6", 734, "4.6")
    q("mcp6001", 159, "Supply Voltage", "6.0"); q("mcp6001", 113, "7.0"); q("fm8002a", 109, "VDD", "6")
    q("fm8002a", 122, "1.6", "6"); q("srv05-4", 89, "VRWM", "5")
    u4_sig = [p for p, n in parts["U4"]["pins"].items() if n and n not in ("+3V3", "GND")]
    ics = [
        ("U2", ["13"], 6.4, 28.0, "IN: 4.35-6.4 V recommended (bq24074.txt:634), 28 V absolute (595)"),
        ("U2", ["2", "3"], None, 5.0, "BAT: 5 V absolute (bq24074.txt:596)"),
        ("U2", ["10", "11", "6", "5", "4", "1", "16", "7", "9", "12", "15"], None, 7.0,
         "OUT, EN1, EN2, CE, TS, ISET, PGOOD, CHG, ILIM, TD: 7 V absolute (bq24074.txt:598-600)"),
        ("U3", ["1"], 5.5, 6.0, "IN: 1.45-5.5 V recommended (tlv757p.txt:206), 6.0 V absolute (178)"),
        ("U3", ["3"], 5.5, 6.0, "EN: 0-5.5 V recommended (tlv757p.txt:208), 6.0 V absolute (179)"),
        ("U4", [p for p, n in parts["U4"]["pins"].items() if n == "+3V3"], 3.6, 3.6,
         "3V3: 3.0-3.6 V recommended (esp32-s3-wroom-1.txt:1596), 3.6 V absolute (1586)"),
        ("U4", u4_sig, V3V3, V3V3 + 0.3, "every signal pin: VIH at most VDD + 0.3 V (esp32-s3.txt:3746), steady at most VDD"),
        ("U8", ["16"], 6.0, 7.0, "VCC: 2-6 V (sn74hc165.txt:255), 7 V absolute (233)"),
        ("U9", ["16"], 6.0, 7.0, "VCC (sn74hc165.txt:255, 233)"),
        ("U11", ["16"], 6.0, 7.0, "VCC (sn74hc165.txt:255, 233)"),
        ("U6", ["8"], 6.0, 7.0, "VDD 1.8-6.0 V (mcp6001.txt:159, the MCP6002 shares the sheet), 7.0 V absolute (113)"),
        ("U7", ["6"], 6.0, 6.0, "VDD 1.6-6 V (fm8002a.txt:122), 6 V absolute (109)"),
        ("U10", ["5"], 6.0, 7.0, "VDD 1.8-6.0 V (mcp6001.txt:159), 7.0 V absolute (113)"),
        ("U1", ["5", "1", "3"], 5.0, 6.0, "VRWM 5 V (srv05-4.txt:89)"),
        ("J3", [p for p, n in parts["J3"]["pins"].items() if n == "LCD_3V3"], 3.3, 4.6,
         "the panel's VCI and VDDI: 3.3 V recommended (er-tft035ips-6.txt:747-748), 4.6 V absolute (734)"),
        ("J8", ["4"], 3.6, 3.6, "the card's VDD 2.7-3.6 V (the SD spec, from memory: pin_review/j8.json)"),
    ]
    named = {("J3", "LCD_3V3"): "REQ-PWR-14 exception (2): the panel at 3.333 V, 3.350 V at TJ 125 C (DF-96, DEF-61, DEC-09)"}
    print("ICs: steady at or under the recommended maximum (or 0.3 V under the absolute maximum where none is printed), "
          "the hot-plug transient 0.3 V under the absolute maximum:")
    for ref, pins, rec, ab, why in ics:
        nets = sorted({parts[ref]["pins"][p] for p in pins if parts[ref]["pins"].get(p)})
        st = max(vmax[n] for n in nets)
        tr = max(vmax_hot[n] for n in nets)
        lim = rec if rec is not None else ab - 0.3
        exc = next((named[k] for k in named if k[0] == ref and k[1] in nets), None)
        # the 0.3 V margin applies to a transient above the steady value (the hot plug), not to the steady value itself
        good = (st <= lim + 1e-9 or exc) and (tr <= st + 1e-9 or tr <= ab - 0.3 + 1e-9)
        row(good, f"{ref} {','.join(nets)[:60]}: steady {st:.3f} V, transient {tr:.2f} V against {why}"
            + (f" [{exc}]" if exc and st > lim + 1e-9 else ""))
    # U10's input with the device off: within the MCP6001's analog-input limits when the current is held under 2 mA
    q("mcp6001", 115, "2 mA"); q("mcp6001", 117, "VDD + 1.0V")
    i_in = (dn.VBAT_REG[1] - dn.CLAMP_V) / val(parts, "R17")
    row(i_in <= 2e-3 and dn.CLAMP_V <= 1.0, f"U10 pin 3 VBAT_DIV while its supply is 0 V (off): clamped near {dn.CLAMP_V} V, "
        f"{i_in * 1e6:.1f} uA through R17: inside 'VSS - 1.0 V to VDD + 1.0 V' at '+-2 mA' (mcp6001.txt:115-117); "
        "REQ-PWR-14's exception (1) is no longer needed (DF-182)")
    # discretes at 80%, from their datasheets on disk since 2026-10-28
    q("2n7002_cj", 55, "Vth(GS)", "2.5")
    print("FETs and diodes at 80% of their ratings:")
    disc = {"2N7002": (60.0, 20.0, 0.115, "2n7002_cj.pdf"), "AO3400A": (30.0, 12.0, 5.7, "ao3400a.pdf")}
    for ref, p in sorted(parts.items()):
        if p["mpn"] in disc and p["bom"]:
            vds, vgs, idm, src = disc[p["mpn"]]
            d, g = vmax[p["pins"]["3"]], vmax[p["pins"]["1"]]
            ids = 0.06 if p["mpn"] == "AO3400A" else V3V3 / 33e3
            row(d <= 0.8 * vds and g <= 0.8 * vgs and ids <= 0.8 * idm,
                f"{ref} {p['mpn']}: VDS {d:.2f} V of {vds:g} ({d / vds * 100:.0f}%), VGS {g:.2f} V of {vgs:g} "
                f"({g / vgs * 100:.0f}%), ID {ids * 1e3:.1f} mA of {idm * 1e3:g} ({src})")
    for ref, p in sorted(parts.items()):
        if p["mpn"] and p["mpn"].startswith("BAT54") and p["bom"]:
            vr = max(vmax[n] for n in p["pins"].values() if n)
            row(vr <= 0.8 * 30, f"{ref} {p['mpn']}: VR at most {vr:.2f} V of 30 V ({vr / 30 * 100:.0f}%), IF well under "
                                "200 mA (bat54c.pdf, bat54ws.pdf)")
    q("rocpu_tps850", 17, "MAXIMUM VOLTAGE", "12 VDC"); q("rocpu_tps850", 18, "MAXIMUM CURRENT DC", "50 mA"); q("rocpu_tps850", 136, "RATING", "15V 20mA")
    sw = parts.get("SW24")
    if sw:
        i_sw = vmax["VSYS"] / (val(parts, "R13") * (1 - R_TOL)) + vmax["VSYS"] / (val(parts, "R15") * (1 - R_TOL))
        row(vmax["VSYS"] <= 0.8 * 12 and i_sw <= 0.8 * 20e-3, f"SW24 TPS850: {vmax['VSYS']:.2f} V across it open, "
            f"{i_sw * 1e3:.2f} mA through it closed (R13 and R15 from VSYS) against 12 V and 20 mA (rocpu_tps850.txt:17, 136)")
    # passives: MLCC voltage, resistor power, from JLC's own part data (passives.json db_description by LCSC number)
    print("passives at 80% (MLCC voltage, resistor power) from JLC's part description, matched by LCSC number:")
    worst = []
    for ref, p in sorted(parts.items()):
        if p["lib"] not in ("Device:C", "Device:R") or not p["bom"]:
            continue
        desc = pas.get(p["lcsc"])
        if not desc:
            row(False, f"{ref} {p['lcsc']} {p['mpn']}: no JLC description in passives.json to read its rating from")
            continue
        a, b = (p["pins"]["1"], p["pins"]["2"])
        dv, dvh = abs(vmax[a] - (0 if b == "GND" else 0)), max(vmax_hot.get(a, 0), vmax_hot.get(b, 0))
        if p["lib"] == "Device:C":
            m = re.search(r"(\d+(?:\.\d+)?)V\b", desc)
            vr = float(m.group(1))
            st = max(vmax[a], vmax[b])
            share, share_h = st / vr, dvh / vr
            worst.append((share, ref))
            row(share <= 0.8 and share_h <= 0.9, f"{ref} {p['value']} {vr:g} V: {st:.2f} V steady ({share * 100:.0f}%)"
                + (f", {dvh:.2f} V hot plug ({share_h * 100:.0f}%)" if dvh > st else "")) if share > 0.5 or share_h > 0.5 else None
        else:
            m = re.search(r"(\d+(?:\.\d+)?)mW", desc)
            pw = float(m.group(1)) * 1e-3 if m else (0.0625 if "0402" in p["fp"] else 0.1)
            rv = val(parts, ref) if p["value"] != "0" else 0.0
            if rv == 0:
                continue
            i_led = 0.0
            if ref in ("R52", "R53"):
                i_led = 0.0592                       # one cathode pin's worst 59.2 mA (VER-137's 117.7 mA over two pins)
            v = max(vmax[a], vmax[b]) - (0 if "GND" in (a, b) else min(vmax[a], vmax[b]))
            pd = i_led ** 2 * rv if i_led else v * v / rv
            if ref == "R41":
                pd = 0.0                              # carries C1's charge only (no DC path)
            worst.append((pd / pw, ref))
            row(pd <= 0.8 * pw, f"{ref} {p['value']}: {pd * 1e3:.2f} mW of {pw * 1e3:g} mW ({pd / pw * 100:.0f}%)") \
                if pd / pw > 0.2 else None
    worst.sort(reverse=True)
    print(f"  {len(worst)} passives read; the five most stressed: " + ", ".join(f"{r} {s * 100:.0f}%" for s, r in worst[:5]))
    # the 3V3 budget (39cc07f: Wi-Fi on; REQ-PWR-15 as restated: the peak inside 80% of U3's 1 A, the supply at least
    # the module's 0.5 A; U3's heat is VER-144's)
    avg, peak = loads_3v3(parts, "avg"), loads_3v3(parts, "peak")
    tot, tpk = sum(i for _, i, _ in avg), sum(i for _, i, _ in peak)
    print("the 3V3 load with Wi-Fi on (listening; a TX burst in place of RX for the peak):")
    for name, i, why in avg:
        print(f"    {name:40s} {i * 1e3:6.1f} mA  {why}")
    q("tlv757p", 267, "1.2", "1.55", "1.78")
    row(tpk <= 0.8 * 1.0, f"3V3 peak {tpk * 1e3:.0f} mA (a TX burst), average {tot * 1e3:.0f} mA, against 80% of U3's 1 A "
        "(tlv757p.txt:209); its current limit 1.2 A minimum (267) is over the module's 0.5 A supply minimum "
        "(esp32-s3-wroom-1.txt:1597)")
    # connectors' current (stand-in ratings)
    i_cell = tpk + 0.1177 + 0.5
    q("jst_sh", 7, "Current rating", "1.0 A")
    print("connectors at 80% of their current (J4's from jst_sh.txt:7; J1, J2 and J8 catalogue figures, stand-ins: "
          "molex_53261.pdf has no text layer, so J2's 1 A is not read from it):")
    for ref, i, why in (("J1", 0.5, "VBUS at U2's USB500 limit (bq24074.txt:733)"),
                        ("J2", i_cell, f"the cell's peak: 3V3 {tpk * 1e3:.0f} mA in a TX burst + backlight 117.7 mA + the "
                                       "amplifier's 0.5 A, all at once"),
                        ("J4", vmax["AMP_VDD"] / 8.0, "the speaker's peak, AMP_VDD across 8 ohm bridged"),
                        ("J8", STAND_SD, "a card writing, through its VDD contact")):
        row(i <= 0.8 * STAND_CONN_A[ref], f"{ref}: {i:.2f} A of {STAND_CONN_A[ref]:g} A ({i / STAND_CONN_A[ref] * 100:.0f}%): {why}")
    print(f"{len(row.bad)} failing")
    return dn.result(not row.bad)


# ---- VER-195: VBUS on J1's CC and data pins ---------------------------------------------------------------------------
PD_MAX = dn.PD_MAX                # V: the highest fixed USB PD voltage, what a broken source may put out
U1_VF = (0.6, 1.2, 15e-3)         # V, V, A: U1's steering diode, about 0.6 V at onset (stand-in) and 1.2 V max at 15 mA
                                  # (srv05-4.txt:97)
U1_VC = 17.5                      # V: U1's clamp at 5 A, maximum (srv05-4.txt:102)
STAND_PAD_VF = (0.5, 20e-6, 0.06)  # V at A, V a decade: a pad's ESD diode reaches VDD + 0.5 V at about 20 uA (stand-in)
STAND_3V3_SINK = 10e-3            # A: the least 3V3 load that can absorb current pushed into the rail (stand-in: the
                                  # TLV75733P sources only; VOUT over VIN + 0.3 V is over its absolute maximum, tlv757p.txt:180)
SEED_USBF = None                  # it fails at 39cc07f: no seed is needed to show it can fail


def usbfault(seed=None):
    """PASS means no part over its absolute maximum for VBUS up to 20 V on any of J1's CC and data pins."""
    parts, ns = load(seed)
    row = Rows()
    ds.quote("esp32-s3", 3746, "VIH", "VDD 1 + 0.3")
    ds.quote("srv05-4", 97, "Forward Voltage", "1.2")
    ds.quote("srv05-4", 102, "Clamping Voltage", "17.5")
    vdd = 3.3 * (1 - dn.VREG_TOL)
    tab = module_table()
    gpio_of = {net: f"GPIO{tab[no][1]}" for no, net in ns["ESP32"].items() if net and tab.get(no, (0, None))[1] is not None}

    def pad_onset(r_series):
        """the source voltage where a pad behind r_series first passes VDD + 0.3 V"""
        return vdd + STAND_PAD_VF[0] + STAND_PAD_VF[1] * r_series

    print(f"VBUS up to {PD_MAX:g} V (PD_MAX) on each of J1's CC and data pins; VDD {vdd:.3f} V (3V3 at -1.5%); an ESP32-S3 "
          "pad's input is VDD + 0.3 V at most (esp32-s3.txt:3746: the chip's absolute table, 3674-3678, rates only its power "
          f"pins); a pad's diode modelled as {STAND_PAD_VF[0]} V at {STAND_PAD_VF[1] * 1e6:.0f} uA (stand-in)")
    for cc, rd, rs, net in (("CC1 (A5)", "R1", "R3", "CC1_SENSE"), ("CC2 (B5)", "R2", "R4", "CC2_SENSE")):
        r_d, r_s = val(parts, rd), val(parts, rs)
        p_rd = PD_MAX ** 2 / r_d
        v_rd = math.sqrt(0.0625 * r_d)
        i_pad = (PD_MAX - vdd - STAND_PAD_VF[0]) / r_s
        on = pad_onset(r_s)
        row(p_rd <= 0.0625, f"{cc}: {rd} {parts[rd]['value']} dissipates {p_rd * 1e3:.0f} mW at {PD_MAX:g} V against its 62.5 mW "
            f"(0402, JLC's description); it reaches 62.5 mW at {v_rd:.1f} V")
        row(on >= PD_MAX, f"{cc}: {gpio_of.get(net, '?')} {net} behind {rs} {parts[rs]['value']} passes VDD + 0.3 V from about "
            f"{on:.1f} V; at {PD_MAX:g} V {i_pad * 1e3:.2f} mA flows into its clamp (esp32-s3.pdf gives no injection limit)")
    u1_on = "USB_DP" in parts["U1"]["pins"].values()
    for line, pins in (("D+ (A6, B6)", ("R21", "USB_DP_MCU")), ("D- (A7, B7)", ("R22", "USB_DM_MCU"))):
        onset = V3V3 + U1_VF[0] if u1_on else pad_onset(val(parts, pins[0]))
        print(f"  {line}: from about {onset:.1f} V {'U1 steers' if u1_on else 'the pad steers'} the line into +3V3 "
              f"({'U1 VF 1.2 V max at 15 mA, srv05-4.txt:97' if u1_on else 'no U1'}); the TLV75733P cannot sink, so once "
              f"the fault pushes more than the rail's load (stand-in {STAND_3V3_SINK * 1e3:.0f} mA) +3V3 rises; through "
              f"{pins[0]} {parts[pins[0]]['value']} ohm the module's USB pad ({gpio_of.get(pins[1], '?')}) sits at the line's clamp")
        row(onset >= PD_MAX, f"{line}: the module's USB pad and the whole +3V3 domain are over their absolute maximum "
            f"(VDD33 3.6 V, esp32-s3-wroom-1.txt:1586) from about {onset:.1f} V; at {PD_MAX:g} V U1's clamp holds +3V3 near "
            f"{U1_VC:g} V at 5 A (srv05-4.txt:102)" if u1_on else f"{line}: the pad alone from about {onset:.1f} V")
    print("for DEC-17: the CC paths are over from about 3.8 V (the pads, at a current R3 and R4 limit to under 2 mA) and "
          "17.9 V (R1, R2); a data line shorted to VBUS above about 4 V destroys the 3V3 domain whatever the series parts")
    print(f"{len(row.bad)} over")
    return dn.result(not row.bad)


# ---- VER-199: the clocked buses on paper ------------------------------------------------------------------------------
LCD_HZ = 25e6                     # Hz: the LCD_CAM write clock pinmap.md allows ('LCD_CAM at 25 MHz or less')
STAND_PIN_C = 10e-12              # F: one CMOS input (the ILI9488's, a 74HC165's): datasheets on disk give none
STAND_FPC_C = 3e-12               # F: J3 and the panel's FPC to the controller
STAND_PAD_R = 33.0                # ohm: an ESP32-S3 pad at its default drive (IOH 40 mA at VOH 2.64 V is PAD_DRIVER 3,
                                  # esp32-s3.txt:3752-3754: 16.5 ohm; the reset drive is weaker, about twice that: stand-in)
STAND_SKEW = 2e-9                 # s: LCD_CAM's data change against WRX's edge inside the chip (the TRM gives no figure)
SEED_TIMING = {"R78": {"value": "1k"}}   # a 1k source resistor on LCD_WR: its edges eat the WRX high and low times


def timing(seed=None):
    parts, _ = load(seed)
    row = Rows()
    pcb, b = dn.board()
    mm = pcb.ToMM
    length = {}
    for t in b.GetTracks():
        if t.GetClass() == "PCB_TRACK":
            n = t.GetNetname().split("/")[-1]
            length[n] = length.get(n, 0.0) + mm(t.GetLength())
    q = ds.quote
    q("ili9488", 16159, "twc", "Write cycle", "40")
    q("ili9488", 16160, "twrh", "15")
    q("ili9488", 16161, "twrl", "15")
    q("ili9488", 16170, "tdst", "10")
    q("ili9488", 16172, "tdht", "10")
    q("esp32-s3", 3753, "IOH")
    import paper_checks as pc
    cpm = pc.STAND_TRACK_C
    print(f"an ESP32-S3 pad: {STAND_PAD_R:g} ohm (stand-in); tracks {cpm * 1e12:.1f} pF/mm (paper_checks.py's stand-in); an input "
          f"{STAND_PIN_C * 1e12:.0f} pF, the FPC {STAND_FPC_C * 1e12:.0f} pF (stand-ins)")
    # the panel's 8080 bus (DBI type B), 8 bits, CSX tied low, WRX's rising edge latches the data
    print(f"LCD_CAM to the panel at {LCD_HZ / 1e6:.2f} MHz (pinmap.md):")
    edges = {}
    for k in list(range(8)) + ["WR"]:
        net = f"LCD_{'D' + str(k) if k != 'WR' else 'WR'}"
        rs = next((r for r in between(parts, net, net + "_MCU", "R")), None)
        c = (length.get(net, 0) + length.get(net + "_MCU", 0)) * cpm + STAND_FPC_C + STAND_PIN_C
        tr = 2.2 * (STAND_PAD_R + (val(parts, rs) if rs else 0)) * c
        edges[net] = tr
        print(f"  {net}: {length.get(net + '_MCU', 0) + length.get(net, 0):.1f} mm of track, {c * 1e12:.1f} pF, through "
              f"{rs or 'no resistor'} {parts[rs]['value'] if rs else ''} ohm: 10-90% edge {tr * 1e9:.1f} ns")
    period = 1 / LCD_HZ
    row(period >= 40e-9 - 1e-15, f"write cycle {period * 1e9:.1f} ns against twc 40 ns (ili9488.txt:16159): "
        f"{period * 1e9 - 40:.1f} ns spare (firmware: a clock whose divider is fractional makes some cycles shorter than "
        "the mean; 0 ns spare leaves none for them)")
    t_wr = edges["LCD_WR"]
    hi = period / 2 - t_wr
    row(hi >= 15e-9, f"WRX high and low {period / 2 * 1e9:.1f} ns less its edge {t_wr * 1e9:.1f} ns = {hi * 1e9:.1f} ns against "
        "twrh and twrl 15 ns (16160-16161)")
    worst_d = max(v for k, v in edges.items() if k != "LCD_WR")
    setup = period / 2 + 0.31 * t_wr - STAND_SKEW - 0.31 * worst_d
    row(setup >= 10e-9, f"data set-up before WRX rises, at the 50% crossings: half a cycle less the skew {STAND_SKEW * 1e9:.0f} ns "
        f"(stand-in) and the slowest data line's edge ({worst_d * 1e9:.1f} ns): {setup * 1e9:.1f} ns against tdst 10 ns (16170)")
    hold = period / 2 - STAND_SKEW
    row(hold >= 10e-9, f"data hold after WRX rises, the data changing on WRX's falling edge: {hold * 1e9:.1f} ns against tdht "
        "10 ns (16172)")
    frame = 320 * 480 * 2 * period
    print(f"  a full RGB565 frame: 320 x 480 x 2 bytes x {period * 1e9:.0f} ns = {frame * 1e3:.1f} ms of bus time "
          f"({320 * 480 * 3 * period * 1e3:.1f} ms in RGB666); REQ-DSP-11's times are measured on the panel (VER-43)")
    # the buttons' chain: three registers now
    q("sn74hc165", 371, "2V", "6")
    q("sn74hc165", 377, "2V", "75", "150")
    q("sn74hc165", 472, "2V", "80")
    q("sn74hc165", 488, "2V", "80")
    regs = sorted(r for r, p in parts.items() if p["mpn"] == "SN74HC165PWR")
    c_clk = (length.get("SR_CLK", 0) + length.get("SR_CLK_MCU", 0)) * cpm + len(regs) * STAND_PIN_C
    t_clk = 2.2 * (STAND_PAD_R + val(parts, "R65")) * c_clk
    f_sr = min(6e6, 1 / (2 * 80e-9), 1 / (150e-9 + 2 * t_clk))
    print(f"SR_CLK to {', '.join(regs)}: {length.get('SR_CLK', 0) + length.get('SR_CLK_MCU', 0):.1f} mm of track, {c_clk * 1e12:.1f} "
          f"pF, edge {t_clk * 1e9:.1f} ns through R65; at 3.3 V the 74HC165 is read at its 2 V figures (fmax 6 MHz, tw 80 ns, "
          "tpd CLK to QH 150 ns, SH/LD high 80 ns before CLK: sn74hc165.txt:371-377, 472, 488), which bound it from the slow side")
    row(t_clk <= 500e-9, f"SR_CLK's edge {t_clk * 1e9:.1f} ns against the 74HC165's input transition limit, 1000 ns/V at 2 V "
        "(sn74hc165.txt:264-265)")
    print(f"  firmware rule: SR_CLK at {f_sr / 1e6:.2f} MHz or less, SR_PL low 80 ns or more and high 80 ns before the first edge; "
          f"{8 * len(regs)} bits a read")
    skew = (length.get("SR_CLK", 0)) * 6.5e-12
    row(skew < 5e-9, f"the chain's QH to SER links: the clock reaches the registers at most {skew * 1e12:.0f} ps apart (6.5 ps/mm), "
        "inside the 5 ns SER hold (sn74hc165.txt:503-504)")
    pm = open(os.path.join(dn.REF, "docs", "pinmap.md"), encoding="utf-8").read()
    for what, pat in (("the panel bus's write clock", r"(?i)(LCD_CAM|8080|write cycle)[^\n]*\b\d+(\.\d+)?\s*(MHz|ns)"),
                      ("the buttons' SR_CLK rate and SR_PL widths", r"(?i)SR_CLK[^\n]*\b\d+(\.\d+)?\s*(MHz|kHz)")):
        row(bool(re.search(pat, pm)), f"pinmap.md states {what}" + ("" if re.search(pat, pm) else
                                                                    ": it does not, so no firmware is held to these limits"))
    print(f"{len(row.bad)} failing")
    return dn.result(not row.bad)


CHECKS = {"appcircuit": (appcircuit, SEED_APP), "gpio": (gpio, SEED_GPIO), "pinreview": (pinreview, SEED_PR),
          "fwpins": (fwpins, SEED_FW), "ratings": (ratings, SEED_RATINGS), "usbfault": (usbfault, SEED_USBF),
          "timing": (timing, SEED_TIMING)}


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if "--list" in sys.argv or not args:
        print(__doc__)
        return 0
    fn, seed = CHECKS[args[0]]
    if "--seed" in sys.argv:
        if not seed:
            sys.exit(f"circuit_checks: {args[0]} has no seed")
        print(f"SEEDED: {seed}")
        return fn(seed)
    return fn()


if __name__ == "__main__":
    sys.exit(main())
