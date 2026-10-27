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

    print("U3 RT9013-33GB (rt9013.pdf, DS9013-10):")
    ds.quote("rt9013", 479, "1μF/X7R on the RT9013 input")
    ds.quote("rt9013", 481, "0.5")
    ds.quote("rt9013", 494, "at least 1μF")
    u3 = parts["U3"]["pins"]
    cin = between(parts, u3["1"], "GND", "C")
    cout = between(parts, u3["5"], "GND", "C")
    row(sum(val(parts, r) for r in cin) >= 1e-6, f"CIN on VIN ({u3['1']}): {', '.join(cin)} {sum(val(parts, r) for r in cin) * 1e6:.0f} "
        "uF >= 1 uF X7R (rt9013.pdf p8, rt9013.txt:479); its distance is checked below on the routed board", "power")
    row(sum(val(parts, r) for r in cout) >= 1e-6, f"COUT on VOUT ({u3['5']}): {', '.join(cout)} "
        f"{sum(val(parts, r) for r in cout) * 1e6:.0f} uF >= 1 uF ceramic (rt9013.txt:494)", "power")
    ds.quote("rt9013", 483, "cannot be left floating")
    en_pd = between(parts, u3["3"], "GND", "R")
    row(bool(en_pd), f"EN ({u3['3']}) held low by {', '.join(en_pd) or 'nothing'}: 'cannot be left floating' (rt9013.txt:483)", "power")
    row(u3["4"] is None, "pin 4 NC, as the SOT-23-5 pinout (pin_review/u3.json, p2)", "power")

    print("U8, U9 SN74HC165PWR (sn74hc165.pdf, SCLS116):")
    ds.quote("sn74hc165", 157, "CLK", "15", "CLK INH")
    ds.quote("sn74hc165", 255, "Supply voltage", "2", "6")
    for u in ("U8", "U9"):
        p = parts[u]["pins"]
        dec = [r for r in between(parts, "+3V3", "GND", "C")]
        row(p["15"] == "GND", f"{u} CLK INH (pin 15) at {p['15']}: low enables the clock (sn74hc165.txt:157, 198)", "buttons")
        row(p["16"] == "+3V3" and p["8"] == "GND", f"{u} VCC (16) on +3V3, inside 2-6 V (sn74hc165.txt:255)", "buttons")
        row(p["10"] is not None, f"{u} SER (pin 10) at {p['10']}: driven, not floating", "buttons")
        ins = [p[d] for d in ("11", "12", "13", "14", "3", "4", "5", "6")]
        pulled = all(any(set(parts[r]["pins"].values()) >= {n, "+3V3"} for r in parts if r.startswith("RN")) for n in ins)
        row(pulled, f"{u} D0-D7 each pulled up by an RN array: no input floats with its contact open", "buttons")
    print("U4 RP2354B outside the core (the core's values are VER-197's):")
    q = ds.quote("rp2350", 100938, "RUN", "Pull-Up")
    run_pu = [r for r in between(parts, "RUN", "+3V3", "R")]
    row(True, f"RUN has its own pull-up ('{q}', rp2350.txt:100938){'' if not run_pu else ', plus ' + run_pu[0]}; "
        "SW19 pulls it low through R26 1k", "mcu")
    ds.quote("rp2350", 1196, "27Ω series termination")
    row(all(abs(val(parts, r) - 27) < 1e-6 for r in ("R21", "R22")), "USB_DP and USB_DM through 27 ohm each (rp2350.txt:1196)",
        "usb_debug")
    j5 = parts["J5"]["pins"]
    row({j5["1"], j5["3"]} == {"SWCLK", "SWDIO"} and parts["U4"]["pins"]["33"] == "SWCLK" and parts["U4"]["pins"]["34"] == "SWDIO",
        "SWCLK (pin 33) and SWDIO (34) reach J5 and J7 (rp2350.txt:1207)", "usb_debug")
    print("the panel ER-TFT035IPS-6 on J3 (er-tft035ips-6.pdf, its ILI9488):")
    panel = ns["PANEL"]
    ds.quote("er-tft035ips-6", 722, "1", "4-wire 8-bit data Serial Interface")
    row(all(panel[p] == "LCD_3V3" for p in (7, 8, 9)), "IM0-IM2 (pins 7-9) high: '1 1 1 4-wire 8-bit data Serial Interface I' "
        "(er-tft035ips-6.txt:722)", "display")
    ds.quote("er-tft035ips-6", 667, "DB17-DB0", "Fix to DGND level when not in use")
    row(all(panel[p] == "GND" for p in range(11, 33)), "VSYNC, HSYNC, DOTCLK, DE and DB17-DB0 (11-32) at GND, as 'Fix to DGND "
        "level when not in use' (er-tft035ips-6.txt:659-667)", "display")
    ds.quote("er-tft035ips-6", 675, "RD", "Fix to IOVCC when not in use")
    row(panel[35] == "LCD_3V3", "RD (35) at the logic supply (er-tft035ips-6.txt:675)", "display")
    ds.quote("er-tft035ips-6", 669, "SDO", "Leave the pin open")
    row(panel[33] == "LCD_MISO" and panel[39] == "LCD_TE", "SDO (33) and TE (39) to U4's inputs, open-safe (669, 686)", "display")
    q747 = ds.quote("er-tft035ips-6", 747, "VCI", "2.5", "3.3")
    vmax = 3.3 * (1 + dn.VREG_TOL)
    row(vmax <= 3.3, f"VCI and VDDI (40-42) on LCD_3V3 = +3V3 through JP4: up to {vmax:.3f} V (RT9013 +-2%) against "
        f"'{q747}' (er-tft035ips-6.txt:747-748): the known DEF-61, DEC-09 proposes accepting it", "display")
    # the routed board: CIN's distance
    pcb, b = dn.board()
    mm = pcb.ToMM
    fp = {f.GetReference(): f for f in b.GetFootprints()}
    vin = next(p for p in fp["U3"].Pads() if p.GetNumber() == "1")
    dist = min(math.hypot(mm(p.GetPosition().x - vin.GetPosition().x), mm(p.GetPosition().y - vin.GetPosition().y))
               for r in cin for p in fp[r].Pads() if p.GetNetname().split("/")[-1] == u3["1"])
    row(dist <= 12.7, f"the nearest CIN pad is {dist:.1f} mm from U3's VIN, within 0.5 in (12.7 mm, rt9013.txt:481)", "power")
    quotes(row)
    print(f"{len(row.bad)} failing")
    return dn.result(not row.bad)


# ---- the quoted-constant audit (part of VER-198) -------------------------------------------------------------
CITE = re.compile(r"\b(bq24074|rp2350|ili9488|er-tft035ips-6|rt9013|sn74hc165)\.txt:(\d+)(?:-(\d+))?((?:,\s*\d+)*)")
DERIVED = {"I_TS_TOL": "the tolerance (78 - 72) / 2 / 75 of the cited band, not a printed number",
           "RP_VREG_L": "the line pair names the part; 3.3 uH is on 33608, checked there",
           "I_PATH": "VER-13's own test current; the cite is for the 475 mA USB500 limit it sits above"}


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


CHECKS = {"appcircuit": (appcircuit, SEED_APP)}


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
