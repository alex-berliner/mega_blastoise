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


# ---- VER-15: the GPIO function mux --------------------------------------------------------------------------------
TABLE = (1261, 1390)          # rp2350.txt: Table 3, the GPIO function table (F0-F11), GPIO0-47
SPI_JOB = {"LCD_SCK_MCU": "SPI0 SCK", "LCD_MOSI_MCU": "SPI0 TX", "LCD_MISO": "SPI0 RX", "LCD_CS": "SPI0 CSn"}
PWM_NETS = ("BL_PWM", "AUDIO_PWM")
ADC_NETS = ("CC1_SENSE", "CC2_SENSE", "VBAT_SENSE")
PWM_RS = "embassy-rp-0.8.0/src/pwm.rs"
SEED_GPIO = {"gpio": {29: "AUDIO_PWM", 31: "SR_CLK_MCU"}}   # the audio PWM onto the backlight's slice 6


def gpio_table():
    """{gpio: set of function names} from rp2350.txt's Table 3 (QFN-80: GPIO0-47)."""
    t = ds.text("rp2350")
    out = {}
    for k in range(TABLE[0], TABLE[1]):
        m = re.match(r"^\s+(\d{1,2})\s{2,}(\S.*)$", t[k - 1])
        if m and int(m.group(1)) <= 47:
            out[int(m.group(1))] = {s.strip() for s in re.split(r"\s{2,}", m.group(2)) if s.strip()}
    return out


def gpio(seed=None):
    parts, ns = load(None)
    g = dict(ns["GPIO"])
    if seed:
        g.update(seed["gpio"])
    row = Rows()
    ds.quote("rp2350", 1261, "GPIO", "F0", "F11")
    tab = gpio_table()
    row(sorted(tab) == list(range(48)), f"Table 3 read for GPIO0-47 (rp2350.txt:{TABLE[0]}-{TABLE[1]}): {len(tab)} rows")
    q = ds.quote("rp2350", 80737, "QFN-80", "GPIO40")
    print(f"  ADC: '{q}' (rp2350.txt:80737): GPIO40-47 are ADC0-7")
    slices = {}
    print("  the whole GPIO-to-PWM table, from the datasheet, against (n >> 1) & 7 (n < 32) and 8 + ((n >> 1) & 3) (n >= 32):")
    for n in range(48):
        pw = [f for f in tab.get(n, ()) if re.fullmatch(r"PWM\d+ [AB]", f)]
        want = ((n >> 1) & 7) if n < 32 else 8 + ((n >> 1) & 3)
        ch = "A" if n % 2 == 0 else "B"
        got = pw[0] if pw else "none"
        slices[n] = got
        print(f"    GPIO{n:<2} {got:8} formula PWM{want} {ch}  net {g.get(n) or '-'}")
        row(got == f"PWM{want} {ch}", f"GPIO{n}: the table's {got} is the formula's PWM{want} {ch}") if got != f"PWM{want} {ch}" else None
    # embassy-rp's own table
    rs = glob.glob(os.path.expanduser(f"~/.cargo/registry/src/*/{PWM_RS}"))
    if not rs:
        ds.blocked(f"{PWM_RS} is not in ~/.cargo/registry (cargo fetch in crates/mega_blastoise_fw)")
    agree = 0
    for ln in open(rs[0], encoding="utf-8"):
        m = re.match(r"\s*impl_pin!\(PIN_(\d+), PWM_SLICE(\d+), Channel([AB])Pin\);", ln)
        if m:
            n = int(m.group(1))
            ok = slices.get(n) == f"PWM{m.group(2)} {m.group(3)}"
            agree += ok
            if not ok:
                row(False, f"pwm.rs says PIN_{n} is PWM{m.group(2)} {m.group(3)}; the datasheet {slices.get(n)}")
    row(agree == 48, f"embassy-rp 0.8.0's pwm.rs agrees on {agree} of 48 pins ({rs[0].split('/src/', 1)[1]}: PIN_28 slice 6 A, "
                     "PIN_31 slice 7 B among them)")
    print("  each net's job against the functions its pin has:")
    for n, net in sorted(g.items()):
        if not net:
            continue
        fns = tab.get(n, set())
        if net in SPI_JOB:
            row(SPI_JOB[net] in fns, f"GPIO{n} {net}: needs {SPI_JOB[net]}; the pin has {', '.join(sorted(f for f in fns if f.startswith('SPI')))}")
        elif net in PWM_NETS:
            row(slices[n] != "none", f"GPIO{n} {net}: a PWM output, {slices[n]}")
        elif net in ADC_NETS:
            row(40 <= n <= 47, f"GPIO{n} {net}: an ADC input, ADC{n - 40}" if 40 <= n <= 47 else f"GPIO{n} {net}: NOT an ADC pin")
        else:
            row("SIO" in fns, f"GPIO{n} {net}: plain I/O (SIO{', PIO' if 'PIO0' in fns else ''})")
    used = {}
    for n in (k for k, v in g.items() if v in PWM_NETS):
        used.setdefault(slices[n].split()[0], []).append(f"GPIO{n} {g[n]}")
    for sl, who in sorted(used.items()):
        row(len(who) == 1, f"{sl}: {', '.join(who)}" + ("" if len(who) == 1 else ": two PWM jobs on one slice (one frequency)"))
    spi = {SPI_JOB[g[n]].split()[0] for n in g if g[n] in SPI_JOB}
    row(len(spi) == 1, f"the panel's SPI pins all on one instance: {', '.join(sorted(spi))}")
    print(f"{len(row.bad)} failing")
    return dn.result(not row.bad)


# ---- VER-36: the pin reviews' open questions and ratings -----------------------------------------------------------
PR = os.path.join(dn.BOARD, "pin_review")
APPROVALS = os.path.join(os.path.dirname(HERE), "releases", "APPROVALS.md")
FLAG = re.compile(r"OPEN QUESTION|UNKNOWN|UNVERIFIED")
SEED_PR = {"R43": {"value": "100k", "lcsc": "C25741", "mpn": "0402WGF1003TCE"}}   # a BOM value the review did not see


def pinreview(seed=None):
    parts, _ = load(seed)
    row = Rows()
    alines = open(APPROVALS, encoding="utf-8").read() if os.path.exists(APPROVALS) else ""
    signed = set(re.findall(r"\bA-\d{3}\b", alines))
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
    print(f"signed A-lines in releases/APPROVALS.md: {len(signed)} (a waiver counts only when it cites one)")
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
    want = {f"PIN_{n}": net for n, net in ns["GPIO"].items()}
    print(f"design.py's GPIO map: {len(want)} nets on GPIO " + ", ".join(f"{n}={v}" for n, v in sorted(ns["GPIO"].items())))
    pinfile = f"{FW}/board_ref_a.rs"
    if pinfile not in names:
        row(False, f"{pinfile} does not exist at {ref}: the firmware has no generated REF-A pin file (DEF-34; "
                   "board/tools/pinmap.py does not write one yet)")
    else:
        text = subprocess.run(git + ["show", f"{ref}:{pinfile}"], capture_output=True, text=True).stdout
        got = {m.group(2): m.group(1) for m in re.finditer(r"pub const (\w+)\s*:\s*\w+\s*=\s*(\d+)\s*;", text)}
        for n, net in sorted(ns["GPIO"].items()):
            row(str(n) in got and got[str(n)].lower().replace("_", "") == net.lower().replace("_", ""),
                f"GPIO{n} {net}: the pin file says {got.get(str(n), 'nothing')}")
    lit = []
    for f in names:
        if re.search(r"ref", os.path.basename(f)) and f.endswith(".rs") and f != pinfile:
            text = subprocess.run(git + ["show", f"{ref}:{f}"], capture_output=True, text=True).stdout
            lit += [(f, m.group(0)) for m in re.finditer(r"\bPIN_\d+\b", text)]
    row(not lit, f"literal PIN_n in the board_ref code at {ref}: {len(lit)}" + (": " + ", ".join(sorted({f'{os.path.basename(a)} {b}' for a, b in lit}))[:300] if lit else ""))
    return dn.result(not row.bad)


CHECKS = {"appcircuit": (appcircuit, SEED_APP), "gpio": (gpio, SEED_GPIO), "pinreview": (pinreview, SEED_PR),
          "fwpins": (fwpins, SEED_FW)}


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
