"""Write the reference schematic from design.py, through Konnect.

    python3 tools/build_sch.py        # from hardware/ref_design/board/

Drawing rules, chosen around what Konnect can and cannot write:

  * Konnect writes labels without KiCad's "justify" flag, so a label's text
    always runs right (or up) from its anchor. Labels that must sit left of (or
    below) a pin therefore go at the far end of a longer stub, with their text
    lying along the wire back towards the pin, the way KiCad draws net labels.
  * Global labels can't lie on a wire (the wire would cross their outline), so
    a global net on a left-facing pin fans out: out past the other stubs, down
    below the symbol, then right into a label.
  * Power nets use plain labels at the pins and one power symbol per rail per
    sheet, in a "rails" column (Konnect leaves a power symbol's own text at the
    page corner, so the label carries the name).
  * Konnect puts a symbol's reference above its origin and value below, so
    two-pin parts lie horizontal and those texts land clear of the wires.

Pin tips come from kisym (Konnect reports pin roots). The result is checked by
check_netlist.py against design.py and by KiCad's ERC. Deletes and regenerates
the .kicad_sch files; nothing else in the folder.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD = os.path.dirname(HERE)
sys.path.insert(0, HERE)
sys.path.insert(0, BOARD)

import design  # noqa: E402
import kisym  # noqa: E402
from konnect_client import Konnect  # noqa: E402

PROJECT = "battler"
ROOT_FILE = PROJECT + ".kicad_sch"
G = 1.27                 # connection grid
STUB = 2.54
CHAR = 1.2               # label text width per character at 1.27 mm
STEP = {"right": (1, 0), "left": (-1, 0), "up": (0, -1), "down": (0, 1)}
TWO_PIN = {"Device:R", "Device:C", "Device:L", "Device:FerriteBead_Small", "Device:Thermistor_NTC"}
HORIZ = {"Device:LED", "Device:D_Schottky", "Switch:SW_Push"}      # pin 1 left, pin 2 right at 0
POWER_SYM = {"GND": ("power:GND", "GND"), "+3V3": ("power:+3V3", "+3V3"), "+1V1": ("power:+1V1", "+1V1"),
             "VBUS": ("power:VBUS", "VBUS"), "VSYS": ("power:VCC", "VSYS"), "VBAT": ("power:VCC", "VBAT")}

# Regions per sheet: (x0, y0, x1, y1, refs), filled left to right, top to bottom.
REGIONS = {
    ROOT_FILE: [(20, 150, 175, 190, ["H1", "H2", "H3", "H4", "H5", "H6", "FID1", "FID2", "FID3"])],
    "usb_charger.kicad_sch": [
        (15, 22, 125, 175, ["J1", "U1", "R1", "R2", "R3", "R4", "C1", "#FLG01", "#FLG02"]),
        (130, 22, 250, 175, ["U2", "R5", "R9", "R10", "C2", "C3", "D5", "D6", "Q2", "R6", "C9", "#FLG03", "J2"]),
    ],
    "power.kicad_sch": [
        (15, 22, 250, 80, ["U3", "C5", "C6", "C7"]),
        (15, 85, 250, 175, ["R13", "D2", "R14", "D3", "R15", "Q1", "R16",
                             "R17", "R18", "C8", "TP1", "TP2", "TP3", "TP4", "TP5"]),
    ],
    "mcu.kicad_sch": [
        (12, 17, 138, 198, ["U4"]),
        (142, 28, 280, 172, ["X1", "R20", "C27", "C28", "R21", "R22", "R23"]),
    ],
    "mcu_power.kicad_sch": [
        (15, 28, 250, 170, ["C10", "C11", "C12", "C13", "C14", "C15", "C16", "C17", "C18", "C19",
                             "C23", "C24", "C20", "C21", "C22", "C25", "L2", "R19", "C26",
                             "#FLG05", "#FLG06"]),
    ],
    "debug.kicad_sch": [
        (15, 22, 250, 120, ["R25", "SW18", "R26", "SW19", "J5", "J6"]),
    ],
    "display.kicad_sch": [
        (15, 22, 95, 195, ["J3"]),
        (110, 30, 250, 75, ["R28", "R29", "C29", "C30"]),
        (110, 80, 250, 175, ["U6", "C31", "C32", "C33", "C36", "R31"]),
    ],
    "audio.kicad_sch": [(15, 22, 250, 150, ["U7", "R32", "C37", "R33", "C38", "C39", "R34", "R35", "C40",
                                            "R36", "Q3", "R37", "C34", "C35", "J4"])],
    "buttons.kicad_sch": [
        (15, 22, 250, 90, ["SW1", "SW2", "SW3", "SW4", "SW5", "SW6", "SW7", "SW8"]),
        (15, 95, 250, 165, ["SW9", "SW10", "SW11", "SW12", "SW13", "SW14", "SW15", "SW16"]),
    ],
}
RAILS_X = 254.0            # rails column (power symbols) on every sheet
SHEET_NOTES = [
    ("usb_charger.kicad_sch", "USB_Charger", ("USB-C sink, ESD, TP4056 charger,", "load sharing, battery connector")),
    ("power.kicad_sch", "Power", ("ME6211 3V3 LDO, power button latch,", "battery and button sensing")),
    ("mcu.kicad_sch", "MCU", ("RP2354B (2 MB flash inside),", "12 MHz crystal, USB series resistors")),
    ("mcu_power.kicad_sch", "MCU_Power", ("RP2354B decoupling and its", "internal 1.1 V switcher")),
    ("debug.kicad_sch", "Debug", ("SWD and UART connectors,", "BOOTSEL and RESET buttons")),
    ("display.kicad_sch", "Display", ("50-pin ZIF connector for the 3.5in IPS", "panel, AP3156 backlight driver")),
    ("audio.kicad_sch", "Audio", ("PWM low-pass into an FM8002A", "bridge amplifier, speaker pads")),
    ("buttons.kicad_sch", "Buttons", ("16 membrane contacts,", "one GPIO each")),
]
TEXT = {
    ROOT_FILE: [
        (20, 20, 2.0, "Two-player handheld: reference design (REF-A)"),
        (20, 27, 1.27, "RP2354B, 3.5in 320x480 IPS (ILI9488, SPI), 16 membrane contacts under DS Lite silicone,"),
        (20, 31, 1.27, "PWM audio into an FM8002A, one Li-ion cell on a TP4056 charger with load sharing, ME6211 3V3."),
        (20, 35, 1.27, "Every sheet is generated from board/design.py by tools/build_sch.py. Edit design.py, not the sheets."),
        (20, 145, 1.27, "Mechanical: locating holes at positions set by the case; fiducials for assembly."),
    ] + [item for i, (f, name, desc) in enumerate(SHEET_NOTES)
         for item in ((22 + (i % 4) * 65, 47 + (i // 4) * 45, 1.8, name),
                      (22 + (i % 4) * 65, 55 + (i // 4) * 45, 1.0, f),
                      (22 + (i % 4) * 65, 60 + (i // 4) * 45, 1.0, desc[0]),
                      (22 + (i % 4) * 65, 63.5 + (i // 4) * 45, 1.0, desc[1]))],
    "usb_charger.kicad_sch": [
        (15, 185, 1.27, "TP4056 charges at 500 mA (R5); TEMP is grounded (no thermistor). USB feeds the system through D5;"),
        (15, 189, 1.27, "without USB the cell does, through Q2. D6 keeps Q2's gate off C1, so R6 turns Q2 on at once on an unplug."),
        (15, 193, 1.27, "CC1/CC2_SENSE let firmware read the source's current offer (dim the backlight on a 500 mA port)."),
    ],
    "power.kicad_sch": [
        (15, 185, 1.27, "Power: the button (seat 1's Y contact), the MCU (PWR_HOLD) or USB raise PWR_EN. Releasing it with"),
        (15, 189, 1.27, "PWR_HOLD low turns the device off. A reset drops PWR_HOLD, so a crash turns it off."),
        (15, 193, 1.27, "Q1 lets the MCU see the button (PWR_BTN_N low = pressed) without exposing a pin to VSYS."),
    ],
    "mcu.kicad_sch": [
        (142, 18, 1.27, "Core from Raspberry Pi's RP2350B minimal design (MIT), fitted with an RP2354B: its 2 MB flash"),
        (142, 22, 1.27, "is inside the package, so only QSPI_SS (the BOOTSEL strap, R23 pull-up) is wired outside."),
    ],
    "mcu_power.kicad_sch": [
        (15, 180, 1.27, "RP2354B supply, as in Raspberry Pi's minimal design: 100 nF at every IOVDD, DVDD, ADC, OTP and QSPI"),
        (15, 184, 1.27, "pin, 4.7 uF at the regulator's input and output, and L2 (a polarity-marked 3.3 uH inductor, fit as drawn)."),
    ],
    "debug.kicad_sch": [
        (15, 130, 1.27, "J5 SWD and J6 UART follow the Raspberry Pi 3-pin debug connector pinout (Debug Probe cable)."),
        (15, 134, 1.27, "SW18 BOOTSEL and SW19 RESET sit under pinholes in the lid."),
    ],
    "display.kicad_sch": [
        (110, 15, 1.27, "EastRising ER-TFT035IPS-6 (ILI9488, 4-wire SPI). Its 50-pin FPC folds under the panel into J3 (ZIF)."),
        (110, 19, 1.27, "J3 pin j is panel pin 51 - j. Unused bus pins as the datasheet says: DB0-17 and RGB low, RD and IM2-0 high."),
        (110, 23, 1.27, "U6 AP3156: three sinks per cathode pin (LED_K1, LED_K2), 20 mA per LED at most, set over SDI (1-wire)."),
    ],
    "audio.kicad_sch": [
        (15, 160, 1.27, "AUDIO_PWM (GPIO8, PWM4 A) through two 16 kHz RC poles; gain 2 x R35/R34 = 2. AMP_EN high"),
        (15, 164, 1.27, "turns the amplifier on (Q3 pulls SHUTDOWN low). 8 ohm 1 W speaker on the J4 pads, from below."),
    ],
    "buttons.kicad_sch": [
        (15, 175, 1.27, "Bare ENIG contacts (DSL_button_4ring) under DS Lite silicone; the firmware enables pull-ups."),
    ],
}


def snap(v, g=G):
    return round(round(v / g) * g, 4)


def up_to(v, g=2.54):
    """Round up to a multiple of g (tolerant of float noise: 27.94 stays 27.94)."""
    import math
    return round(math.ceil(v / g - 1e-6) * g, 4)


def textw(s):
    return len(s) * CHAR + 1.0


def orientation(p):
    """Rotation that puts the most public net on the right-hand pin."""
    rank = lambda n: 0 if n is None else {"local": 1, "power": 2, "global": 3}[design.label_kind(n)]
    lib = p["lib"]
    if lib in TWO_PIN:
        a, b = p["pins"].get("1"), p["pins"].get("2")
        return 270 if rank(a) > rank(b) else 90
    if lib in HORIZ:
        a, b = p["pins"].get("1"), p["pins"].get("2")
        return 180 if rank(a) > rank(b) else 0
    if lib.startswith("Connector_Generic"):
        return 180
    if lib in ("power:PWR_FLAG", "Connector:TestPoint"):
        # pin points down; a global net's label must not lie on its wire, so turn it up
        return 180 if design.label_kind(p["pins"]["1"]) == "global" else 0
    return p["rot"]


def plan(p, rot):
    """Drawing operations for one part, relative to its origin."""
    ops, fan = [], []
    tips = {}
    for num, pin in kisym.placed(p["lib"], 0, 0, rot).items():
        if num not in p["pins"]:
            raise SystemExit(f"{p['ref']} pin {num} ({pin['name']}) has no entry in design.py")
        key = (pin["sx"], pin["sy"])
        net = p["pins"][num]
        if key in tips and tips[key][0] != net:
            raise SystemExit(f"{p['ref']}: stacked pins at {key} disagree: {tips[key][0]} vs {net}")
        tips[key] = (net, pin["out"])
    for (x, y), (net, out) in sorted(tips.items(), key=lambda kv: kv[0][1]):
        if net is None:
            ops.append(("nc", x, y))
            continue
        kind = design.label_kind(net)
        dx, dy = STEP[out]
        if kind == "global" and out in ("left", "down"):
            if out == "left":
                fan.append((x, y, net))
                continue
            print(f"  note: {p['ref']} {net} faces down; label lies on its wire")
        if out in ("right", "up"):
            ex, ey = snap(x + dx * STUB), snap(y + dy * STUB)
        else:
            ln = up_to(textw(net) + (3.5 if kind == "global" else 1.3)) + STUB
            ex, ey = snap(x + dx * ln), snap(y + dy * ln)
        ops.append(("wire", x, y, ex, ey))
        rot_label = 0 if out in ("right", "left") else 90
        ops.append(("label", net, ex, ey, rot_label, "global" if kind == "global" else "local"))
    if fan:
        # stubs of the other left-facing pins below the first fanned pin decide how far out to go
        fan.sort(key=lambda f: f[1])
        top_y = fan[0][1]
        px = min(f[0] for f in fan)
        reach = STUB
        for op in ops:
            if op[0] == "wire" and op[2] >= top_y and op[3] < op[1]:        # horizontal, heading left
                reach = max(reach, op[1] - op[3])
        bottom = max([y for o in ops for y in ((o[2], o[4]) if o[0] == "wire" else (o[3],) if o[0] == "label" else (o[2],))] + [f[1] for f in fan])
        n = len(fan)
        base = up_to(bottom + 3.0)
        for k, (x, y, net) in enumerate(fan):
            s = up_to(reach + 2.54 * (n - k))
            row = snap(base + 2.54 * (n - 1 - k))
            vx = snap(px - s)
            ax = snap(px - STUB)
            ops += [("wire", x, y, vx, y), ("wire", vx, y, vx, row), ("wire", vx, row, ax, row),
                    ("label", net, ax, row, 0, "global")]
    return ops


def extent(p, rot, ops):
    xs, ys = [0.0], [0.0]
    for num, pin in kisym.placed(p["lib"], 0, 0, rot).items():
        xs.append(pin["sx"]); ys.append(pin["sy"])
    for o in ops:
        if o[0] == "wire":
            xs += [o[1], o[3]]; ys += [o[2], o[4]]
        elif o[0] == "label":
            _, net, x, y, r, kind = o
            w = textw(net) + (3.5 if kind == "global" else 0)
            xs += [x, x + (w if r == 0 else 1.5)]
            ys += [y - (w if r == 90 else 1.5), y + 1.5]
        elif o[0] == "nc":
            xs.append(o[1]); ys.append(o[2])
    # Konnect puts the reference 3.81 above the origin and the value 3.81 below,
    # turned with the symbol: across for rotations 90/270, along for 0/180
    for text, dy in ((p["ref"], -3.81), (p["value"], 3.81)):
        half = len(text) * 0.62
        if rot in (90, 270):
            xs += [-1.0, 1.0]; ys += [dy - half, dy + half]
        else:
            xs += [-half, half]; ys += [dy - 1.0, dy + 1.0]
    return min(xs) - 1.5, min(ys) - 1.5, max(xs) + 1.5, max(ys) + 1.5


def layout(sheet):
    """Bottom-left fill: each part, in order, goes to the highest then leftmost
    free spot of its region where its box (labels and texts included) fits."""
    parts = {p["ref"]: p for p in design.PARTS if p["sheet"] == sheet}
    placed, boxes = {}, []
    for x0, y0, x1, y1, refs in REGIONS[sheet]:
        for ref in refs:
            p = parts[ref]
            rot = orientation(p)
            ops = plan(p, rot)
            ex0, ey0, ex1, ey1 = extent(p, rot, ops)
            w, h = ex1 - ex0, ey1 - ey0
            xs = sorted({x0} | {b[2] for b in boxes if x0 <= b[2] <= x1})
            ys = sorted({y0} | {b[3] for b in boxes if y0 <= b[3] <= y1})
            spot = None
            for y in ys:
                for x in xs:
                    if x + w > x1 + 0.01 or y + h > y1 + 0.01:
                        continue
                    if all(x + w <= b[0] or b[2] <= x or y + h <= b[1] or b[3] <= y for b in boxes):
                        spot = (x, y)
                        break
                if spot:
                    break
            if spot is None:
                raise SystemExit(f"{sheet}: no room for {ref} ({w:.0f} x {h:.0f} mm) in its region")
            ox, oy = snap(spot[0] - ex0, 2.54), snap(spot[1] - ey0, 2.54)
            boxes.append((ox + ex0, oy + ey0, ox + ex1, oy + ey1))
            placed[ref] = (ox, oy, rot, ops)
    missing = set(parts) - set(placed)
    if missing:
        raise SystemExit(f"{sheet}: no region for {sorted(missing)}")
    return placed


def main():
    # Konnect looks for libraries in KICAD10_SYMBOL_DIR before KiCad's own folder; the
    # project's symbols (tools/sym_lib.py) live in lib/, not in the project's library table
    os.environ["KICAD10_SYMBOL_DIR"] = os.path.join(BOARD, "lib")
    k = Konnect()
    for ts in ("project", "sch_components", "sch_wiring", "sch_hierarchy", "library", "sch_batch"):
        k.call("load_toolset", name=ts)
    for f in [ROOT_FILE] + [s for s, _ in design.SHEETS]:
        path = os.path.join(BOARD, f)
        if os.path.exists(path):
            os.remove(path)
    if not os.path.exists(os.path.join(BOARD, PROJECT + ".kicad_pro")):
        k.call("create_project", path=BOARD, name=PROJECT)
    else:
        k.call("create_schematic", path=os.path.join(BOARD, ROOT_FILE))
    k.call("register_footprint_library", library_path=os.path.join(BOARD, "lib", "battler.pretty"),
           nickname="battler", project=os.path.join(BOARD, PROJECT + ".kicad_pro"), scope="project")
    root = os.path.join(BOARD, ROOT_FILE)
    for i, (f, name) in enumerate(design.SHEETS):
        col, row = i % 4, i // 4
        k.call("add_hierarchical_sheet", schematic=root, sheet_file=f, sheet_name=name,
               x=20 + col * 65, y=50 + row * 45, width=55.88, height=25.4, project_name=PROJECT)

    pwr = 0
    for sheet in [ROOT_FILE] + [s for s, _ in design.SHEETS]:
        path = os.path.join(BOARD, sheet)
        placed = layout(sheet)
        print(f"{sheet}: {len(placed)} parts")
        for x, y, size, text in TEXT.get(sheet, []):
            # Konnect centres text on its anchor; these coordinates are the left edge
            k.call("add_schematic_text", schematic=path, text=text, x=snap(x + len(text) * size * 0.39),
                   y=y, size=size)
        edits = []
        rails = set()
        for p in design.PARTS:
            if p["sheet"] != sheet:
                continue
            ox, oy, rot, ops = placed[p["ref"]]
            k.call("add_schematic_component", schematic=path, lib_id=p["lib"], x=ox, y=oy,
                   reference=p["ref"], value=p["value"], rotation=rot)
            if p["fp"]:
                edits.append({"reference": p["ref"], "footprint": p["fp"]})
            for key, name in (("lcsc", "LCSC"), ("mpn", "MPN"), ("mfr", "Manufacturer")):
                if p.get(key):
                    k.call("add_component_annotation", schematic=path, reference=p["ref"], key=name, value=p[key])
            if not p["bom"] and not p["ref"].startswith("#"):
                k.call("add_component_annotation", schematic=path, reference=p["ref"], key="BOM", value="exclude")
            for o in ops:
                if o[0] == "nc":
                    k.call("add_no_connect", schematic=path, x=snap(ox + o[1]), y=snap(oy + o[2]))
                elif o[0] == "wire":
                    k.call("add_wire", schematic=path, x1=snap(ox + o[1]), y1=snap(oy + o[2]),
                           x2=snap(ox + o[3]), y2=snap(oy + o[4]))
                else:
                    _, net, x, y, r, kind = o
                    k.call("add_schematic_net_label", schematic=path, net=net, x=snap(ox + x), y=snap(oy + y),
                           label_type="global_label" if kind == "global" else "net_label",
                           rotation=r, shape="passive")
                    if net in design.POWER:
                        rails.add(net)
        # one power symbol per rail used on this sheet, fed by a label of the same name
        for i, net in enumerate(sorted(rails, key=lambda n: (n != "GND", n))):
            y = 25.4 + i * 12.7
            x0 = RAILS_X
            x1 = snap(x0 + up_to(textw(net) + 1.3) + 5.08)
            k.call("add_schematic_net_label", schematic=path, net=net, x=x0, y=y, label_type="net_label", rotation=0)
            k.call("add_wire", schematic=path, x1=x0, y1=y, x2=x1, y2=y)
            lib, value = POWER_SYM[net]
            pwr += 1
            k.call("add_schematic_component", schematic=path, lib_id=lib, x=x1, y=y,
                   reference=f"#PWR{pwr:03d}", value=value)
        if edits:
            res = k.call("batch_edit_schematic_components", schematic=path, edits=edits)
            if res.get("errors"):
                raise SystemExit(f"{sheet}: {res['errors']}")
    k.close()
    print("power symbols:", pwr)


if __name__ == "__main__":
    main()
