#!/usr/bin/env python3
"""The back-power path table (VER-159): every place a live net reaches an input of a part whose own supply is off.

    python3 hardware/vv/tools/check_backpower.py          # exit 1 on a path the table below does not list
    python3 hardware/vv/tools/check_backpower.py --seed   # DEF-01 seeded (U1's REF, pin 5, back on VBUS): must exit 1

It reads design.py's parts and nets (design_numbers.design()). For each power state below it marks the
live nets: the rails the state powers, the nets a powered part or an outside source drives, and every
net a resistor, a bridged jumper, an inductor or a forward diode joins to a live one (never into a
rail: a rail is live only when the state says so). A part's input whose own supply rail is dead and
whose net is live is a back-power path: current flows from that net into the part's protection diode and
out onto its dead rail. Each path needs a line in ACCEPTED with its worst-case current and the DFMEA row
that carries it; at the order Alex reads the paths by hand (F-R2(c)).

Python 3 standard library only.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import design_numbers as dn  # noqa: E402

# which of a part's pins belong to which supply; every other non-GND pin is an input or output it guards
SUPPLY = {
    "U1": {"+3V3": None},                       # SRV05-4: REF (pin 5) is the positive steering rail
    "U2": {"VBUS": ("1", "4", "6", "7", "9", "12", "15", "16")},   # BQ24073's input-side pins (TS, CE, EN1, PG, CHG,
                                                # ILIM, TD, ISET); BAT and OUT run from the cell
    "U3": {"VSYS": ("5",)},                     # 39cc07f: the TLV75733P's OUT against its IN: VOUT over VIN + 0.3 V is over
                                                # its absolute maximum (tlv757p.txt:180, 1137-1138)
    "U4": {"+3V3": None},
    "U6": {"+3V3": None},
    "U7": {"AMP_VDD": None},
    "U8": {"+3V3": None},
    "U9": {"+3V3": None},
    "U11": {"+3V3": None},
    "U10": {"+3V3": None},
    "J3": {"LCD_3V3": None},                    # the panel: its logic pins only (LED_A, LED_K1, LED_K2 are the backlight's)
    "J8": {"SD_VDD": None},                     # the microSD card: its bus pins against its own supply (JP6)
}
NOT_LOGIC = {"LED_A", "LED_K1", "LED_K2", "GND"}
RAILS = {"VBUS", "VBUS_C1", "VSYS", "VBAT", "CELL_P", "AMP_VDD", "LED_A", "3V3_LDO", "+3V3", "LCD_3V3", "SD_VDD"}
CELL = {"VBAT", "CELL_P", "VSYS", "AMP_VDD", "LED_A"}
ON = CELL | {"3V3_LDO", "+3V3", "LCD_3V3", "SD_VDD"}
POWERED = {"U4", "U6", "U8", "U9", "U11", "U10"}
STATES = {
    # name: (live rails, nets an outside source or a powered part drives, jumpers cut)
    "off on the cell": (CELL, set(), set()),
    "on the cell, no USB": (ON, POWERED, set()),
    "plug-in, before 3V3": (CELL | {"VBUS", "VBUS_C1"}, {"CC1", "CC2"}, set()),
    # 39cc07f: J7's pads (3V3, EN, BOOT, GND) for a pogo jig; design.py: 'power through its 3V3 pad only with the cell out and
    # nothing on USB', so 3V3 is live and VSYS dead
    "jig on J7's 3V3, no cell, no USB": ({"+3V3", "3V3_LDO", "LCD_3V3", "SD_VDD"}, POWERED | {"EN", "BOOT"}, set()),
    "on, JP4 cut (panel unpowered)": (ON - {"LCD_3V3"}, POWERED, {"JP4"}),
    "on, JP6 cut (card unpowered)": (ON - {"SD_VDD"}, POWERED, {"JP6"}),
}
# Outputs a powered part drives (a net it can hold high), per part
DRIVES = {
    # the module's every GPIO it can drive, and its USB D+ (the PHY's pull-up when the device is enabled)
    "U4": lambda parts: {n for n in dn_gpio(parts) if n} | {"USB_DP_MCU"},
    "U6": lambda parts: {"BL_OUT1", "BL_OUT2"},
    "U8": lambda parts: {"SR_DATA"},
    "U9": lambda parts: {"SR_CHAIN"},
    "U11": lambda parts: {"SR_CHAIN2"},
    "U10": lambda parts: {"VBAT_SENSE"},
}
# The listed paths: (state or '*' for every state, ref, pin or '*'): (worst-case current, why it is accepted, the
# DFMEA row)
ACCEPTED = {
    ("*", "U10", "3"): ("about 6.8 uA at a 4.2 V cell: VBAT/2 behind R17 470k into U10's input diode "
                        "(design_numbers.py cutoff), whenever 3V3 is off", "the sense buffer's input, inside the MCP6001's "
                        "2 mA and VDD + 1.0 V (mcp6001.txt:115-117); counted in the off budget (VER-103) and REQ-PWR-07",
                        "DF-182"),
    ("*", "U2", "7"): ("33 uA at most: R9 100k from 3V3 into PGOOD with VBUS at 0; PGOOD is open-drain and 7 V rated "
                       "(bq24074.txt:535, 598)", "an open-drain status output to VSS; SLUS810N draws no path from it to "
                       "IN", "DF-98"),
    ("*", "U2", "9"): ("33 uA at most: R10 100k from 3V3 into CHG, as pin 7 (bq24074.txt:507)", "as pin 7", "DF-98"),
    ("*", "U2", "4"): ("0.7 mA at most while IO46 drives CHG_OFF high, most of it into R44 4.7k: CE is a 7 V rated "
                       "logic input with a 285k pull-down (bq24074.txt:505, 598)", "firmware drives CHG_OFF high only "
                       "to stop a charge", "DF-98"),
    ("on, JP4 cut (panel unpowered)", "J3", "*"): ("a module output's few mA into each of the panel's inputs with "
                                                   "LCD_3V3 off", "JP4 is cut only at bring-up; design.py's JP4 note: "
                                                   "firmware holds the bus pins low while it is open", "DF-98"),
    ("on, JP6 cut (card unpowered)", "J8", "*"): ("the SD bus's 10k pull-ups (R79-R83) and the module's outputs into an "
                                                  "unpowered card's pins: 0.33 mA a line", "JP6 is cut only to measure the "
                                                  "card's current; firmware leaves the SDMMC host off while it is open",
                                                  "DF-98"),
    ("plug-in, before 3V3", "U4", "4"): ("CC2 through R4 10k into IO4: at most (2.0 V - 0.5 V) / 10k = 0.15 mA for the "
                                         "40 ms PWR_EN takes to rise (R14, C47)", "R4 limits it (design.py: 'keeps a CC "
                                         "short to VBUS off the pin')", "DF-98"),
    ("plug-in, before 3V3", "U4", "38"): ("CC1 through R3 10k into IO2: as IO4", "as IO4", "DF-98"),
    ("jig on J7's 3V3, no cell, no USB", "U3", "5"): (
        "the jig's 3V3 on U3's OUT with its IN (VSYS) at 0 V: VOUT over VIN + 0.3 V is over the TLV75733P's absolute "
        "maximum (tlv757p.txt:180), and the current runs back through its body diode into VSYS (1137-1138)",
        "only with JP1 cut: the jig's procedure (bringup.md, VER-58) must cut JP1 first or power the board from USB",
        "DF-189"),
}


def dn_gpio(parts):
    return {n for n in parts["U4"]["pins"].values() if n and n not in ("+3V3", "GND", "EN", "USB_DM_MCU", "USB_DP_MCU")}


def conducts(p):
    """The pin pairs of a part that carry DC both ways (a resistor, a bridged jumper, L2, TH1) or one way (a diode:
    (anode, cathode))."""
    lib = p["lib"]
    pins = p["pins"]
    if lib in ("Device:R", "Device:L", "Device:Thermistor_NTC") or lib.startswith("Jumper:"):
        return [(pins["1"], pins["2"], True)]
    if lib == "Device:R_Pack04":
        return [(pins[str(i)], pins[str(9 - i)], True) for i in range(1, 5)]
    if lib == "Device:D_Schottky":
        return [(pins["2"], pins["1"], False)]
    if lib == "Diode:BAT54C":
        return [(pins["1"], pins["3"], False), (pins["2"], pins["3"], False)]
    return []


def live_nets(parts, rails, drivers, cut):
    live = set(rails)
    for d in drivers:
        live |= DRIVES[d](parts) if d in DRIVES else {d}
    links = []
    for ref, p in parts.items():
        if ref.startswith("#") or ref in cut:
            continue
        links += conducts(p)
    changed = True
    while changed:
        changed = False
        for a, b, both in links:
            for x, y in ((a, b), (b, a)) if both else ((a, b),):
                if x in live and y and y not in live and y not in RAILS and y != "GND":
                    live.add(y)
                    changed = True
    return live


def paths(parts):
    found = {}
    for state, (rails, drivers, cut) in STATES.items():
        live = live_nets(parts, rails, drivers, cut)
        for ref, sup in SUPPLY.items():
            for rail, pins in sup.items():
                if rail in live:
                    continue
                for pin, net in parts[ref]["pins"].items():
                    # a regulator's output is a rail, but it is the pin a live rail back-feeds (U3's OUT against its IN)
                    if net is None or net in NOT_LOGIC or (net in RAILS and ref != "U3") or (pins and pin not in pins):
                        continue
                    if ref == "U1" and pin == "5":
                        continue
                    if net in live:
                        found[(state, ref, pin)] = (net, rail)
    return found


def main():
    parts, _ = dn.design()
    if "--seed" in sys.argv:
        parts["U1"] = dict(parts["U1"], pins=dict(parts["U1"]["pins"], **{"5": "VBUS"}))
        SUPPLY["U1"] = {"VBUS": None}
        print("SEEDED: DEF-01, U1 pin 5 (REF) on VBUS")
    found = paths(parts)
    bad = 0
    used = set()
    print(f"{len(STATES)} power states: {', '.join(STATES)}; {len(found)} back-power paths found, "
          f"{len(ACCEPTED)} table lines")
    for key in sorted(found, key=lambda k: (list(STATES).index(k[0]), k[1], int(k[2]) if k[2].isdigit() else 0)):
        net, rail = found[key]
        state, ref, pin = key
        hit = next((a for a in ((state, ref, pin), ("*", ref, pin), (state, ref, "*")) if a in ACCEPTED), None)
        if hit:
            used.add(hit)
            acc = ACCEPTED[hit]
            print(f"  listed   [{state}] {ref} pin {pin} ({net}) into dead {rail}: {acc[0]}; {acc[1]} ({acc[2]})")
        else:
            bad += 1
            print(f"  UNLISTED [{state}] {ref} pin {pin} ({net}) into dead {rail}")
    for key in sorted(set(ACCEPTED) - used):
        print(f"  listed but no longer found: {key} (remove it from ACCEPTED)")
        bad += 1
    print("not a path in any state: U1 (SRV05-4), whose REF is on 3V3: nothing drives D+ or D- while 3V3 is off, "
          "since USB in turns the board on through R14 and D3 (paper_checks.py latch)")
    print("RESULT: " + ("PASS" if not bad else "FAIL"))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
