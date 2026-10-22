"""Where every part goes on the board (KiCad frame, mm, degrees CCW).

Positions that the case owns (buttons, locating pins, edge connectors, pinholes)
are read from case/positions.json, which tools/export_case.py writes from
case/case.scad. The RP2350B core is not listed here: tools/build_pcb.py copies its
placement and routing from Raspberry Pi's minimal design (CORE below), turned so its
USB and flash side faces the USB-C, and turned over onto the back (CORE_SIDE).

Since 2026-10-21 the cell and the speaker sit in holes in the board (CUTOUTS), so J2, J4,
the debug connectors and the parts round them are placed here, not by the case; the case
learns the holes from case/CELLCUT_FOR_CASE.md.

Height rules (from the case, checked by case/check_fit.py; case/BOARD_OUTLINE.md): under the
glass <= 2.2 mm, in the side bezels and spines <= 3.45 mm, under the decks <= 1.6 mm, round the
clusters <= 0.5 mm, nothing under the panel's folded FPC tail or a silicone piece, nothing
within 2.8 mm of a screw. On the back, with the cell in its hole: <= 1.2 mm (the tub's floor
1.3 mm under the board; positions.json still says 0.5 until the case takes the holes).
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
CASE = json.load(open(os.path.join(HERE, "..", "case", "positions.json")))

# ---- the holes the cell and the speaker sit in (2026-10-21, rows 18/19 of Alex's list) ----------
# The case is too thick with the cell and the speaker under the board, so both sit in cutouts in
# the board's plane. The cell (403450: 50 +-2 x 34 +-2 x 4.0 +-0.5) lies across the board between
# seat 2's deck and J3's range, the only place a 52 x 36 cell fits: 0.5 mm round a worst-case cell,
# and 2.5 mm more at its left end, where its leads come out and run to J2 at the hole's seat-1
# corner. The case pushes the cell to the right end, where TH1 sits at the edge. The speaker
# (15 x 10 racetrack) stands in its own hole in the right bezel, near the right wall, J4 above it.
# Both holes keep 1 mm or more from the screw bosses, the clusters' rings, J3's range and USB-C.
CUTOUTS = [
    dict(name="cell", box=(72.3, 66.8, 127.8, 103.8), r=1.0),     # 55.5 x 37.0
    dict(name="speaker", box=(116.5, 118.6, 127.5, 134.6), r=1.0),  # 11.0 x 16.0
]

# ---- RP2350B core: copied from the reference board ------------------------------
# On the back since the cell took the middle of the board (2026-10-21): the copied block (about
# 29 x 20 mm) fits nowhere else, and under the panel's folded FPC (where the top may carry
# nothing over 0.3 mm) the back is free. It is Raspberry Pi's layout turned over, top to bottom
# (build_pcb.core_xy), so its LCD side faces J3 above it and its USB side still faces J1.
CORE_SIDE = "B"
CORE_AT = (101.0, 121.0)     # where the reference's U1 (at 100,100) lands: under the FPC fold, 9 mm from the
                             # cell's hole so the pins facing it fan out
CORE_Y = CORE_AT[1]
CORE_ROT = -90               # reference rotated clockwise: its USB/QSPI side faces +X
CORE = {                     # reference designator -> ours (same nets, same job); the RP2354B
    "U1": "U4", "X1": "X1", "L1": "L2",    # has its flash inside, so the reference's U3 and R10 stay behind,
    "C12": "C11",                          # and its C2, R1 and R6 (C10, R23, R25 here) sat out by the flash,
 "C13": "C12", "C14": "C13", "C15": "C14",
    "C16": "C15", "C17": "C16", "C18": "C17", "C20": "C18", "C21": "C19",
    "C8": "C20", "C10": "C21", "C11": "C22",
    "C5": "C23", "C6": "C24", "C7": "C25", "C9": "C26", "C3": "C27", "C4": "C28",
    "R2": "R20", "R3": "R19", "R7": "R21", "R8": "R22",
}
CORE_NETS = {                # reference net -> ours, for the copied tracks and zones
    "+1V1": "+1V1", "+3V3": "+3V3", "GND": "GND", "/QSPI_SS": "QSPI_SS",
    "/VREG_AVDD": "VREG_AVDD", "/VREG_LX": "VREG_LX",
    "/XIN": "/MCU/XIN", "/XOUT": "/MCU/XOUT", "Net-(C4-Pad1)": "/MCU/XO",
    "Net-(U1-USB_DM)": "/MCU/USB_DM_MCU", "Net-(U1-USB_DP)": "/MCU/USB_DP_MCU",
}

# ---- everything else ------------------------------------------------------------
b = CASE["buttons"]
s = CASE["pins"]
P = {}                       # ref: (x, y, rot, side)   side "F" or "B"


def put(ref, x, y, rot=0, side="F"):
    P[ref] = (round(x, 3), round(y, 3), rot, side)


for i in range(16):
    put(f"SW{i + 1}", *b[f"SW{i + 1}"])
for ref, xy in CASE["ss_buttons"].items():  # Start/Select, between each seat's clusters at the case's end
    put(ref, *xy)
for i, (x, y) in enumerate(s):
    put(f"H{i + 1}", x, y)
put("FID1", 66.6, 100.6)
put("FID2", 133.2, 141.6)
put("FID3", 66.8, 141.6)

# display: the panel's ZIF connector where its folded FPC reaches (case.scad conn_in), its mouth
# towards seat 1. The core lies under it on the back, so the SPI terminations, the panel's supply
# caps and its jumper sit on the back too, under J3's pins and clear of the core's pin escapes
put("J3", CASE["fpc_conn"]["x"], CASE["fpc_conn"]["y"], 0)
put("R28", 93.9, 106.4, 90, "B")            # SPI source terminations, by GPIO38/39
put("R29", 95.1, 106.4, 90, "B")
put("C29", 85.0, 106.4, 90, "B")            # panel VCI/VDDI, behind connector pins 9-11
put("C30", 86.2, 106.4, 90, "B")
put("JP4", 88.8, 106.6, 0, "B")             # 3V3 -> panel

# backlight current sinks: beside J3's LED end (pins 46-50), under the glass left of J4: the
# op-amp, and each FET above it; their resistors and caps on the back underneath. Channel 2
# (Q6, LED_K2) on the left, channel 1 (Q5, LED_K1) on the right
put("U6", 118.6, 112.6, 90)                 # op-amp half A runs LED_K2, half B LED_K1
put("Q6", 117.65, 106.2, 0)                 # LED_K2 (J3 pins 46, 48)
put("Q5", 121.4, 106.1, 90)                 # LED_K1 (J3 pins 47, 49)
put("R57", 117.4, 105.6, 90, "B")           # channel 2: gate pull-down, gate resistor
put("R55", 118.6, 105.6, 90, "B")
put("R60", 119.8, 105.6, 90, "B")           # PWM into BL_REF
put("R56", 121.0, 105.6, 90, "B")           # channel 1: gate pull-down, gate resistor
put("R54", 122.2, 105.6, 90, "B")
put("C49", 117.4, 108.0, 90, "B")           # channel 2: feedback C and R
put("R59", 118.6, 108.0, 90, "B")
put("R61", 119.8, 108.0, 90, "B")
put("C48", 121.0, 108.0, 90, "B")           # channel 1: feedback C and R
put("R58", 122.2, 108.0, 90, "B")
put("C50", 117.4, 110.4, 90, "B")           # BL_REF filter
put("C51", 118.6, 110.4, 90, "B")           # U6 supply
put("R53", 118.4, 113.2, 0, "B")            # sense resistors
put("R52", 121.8, 113.2, 0, "B")
put("TP10", 118.4, 116.0, 0, "B")           # BL_REF

# debug: left bezel, beside the cell's hole
put("SW18", CASE["pinholes"][0][0], CASE["pinholes"][0][1], 90)
put("SW19", CASE["pinholes"][1][0], CASE["pinholes"][1][1], 90)
put("R26", 71.2, 91.0, 90)
put("J6", 68.75, 71.65, 0)                  # UART: above the side screw, clear of seat 2's deck
put("J5", 68.8, 128.4, 180)                 # SWD: below the seat-1 side screw (J2's plug took its place)

# edge part on the right: USB-C
usb_x = CASE["usb"]["front_x"] - 3.65       # footprint front is 3.65 ahead of its origin
put("J1", usb_x, CASE["usb"]["y"], 90)

# USB input: the ESD array and D5 below J4, round the screw at 131.4, 122; the CC resistors, the
# input cap and its resistor, and D5's damping resistor on the back under J1 and J4; the spare
# CC caps above J1
put("U1", 130.4, 116.6, 90)                 # ESD
put("D5", 131.2, 127.6, 0)                  # VBUS -> VSYS
put("R1", 124.4, 106.0, 90, "B")
put("R3", 125.6, 106.0, 90, "B")
put("R2", 126.8, 106.0, 90, "B")
put("R4", 128.0, 106.0, 90, "B")
put("C1", 125.0, 109.6, 0, "B")
put("R41", 128.8, 109.6, 0, "B")            # C1's series resistor (hot-plug ringing)
put("R47", 127.0, 112.8, 0, "B")            # D5's damping resistor
put("C42", 131.4, 91.6, 0)                  # spares on CC1 and CC2, above the USB-C
put("C43", 131.6, 82.2, 0)

# the cell: J2 at its hole's seat-1 corner, mouth into the hole's lead pocket, so the plug's
# back and wires drop straight down beside the cell's left end; the charger and the load sharing
# below it, round the screw at 68.6, 122
put("J2", 72.3, 107.24, 180)                # PicoBlade: mouth towards the hole, contacts behind
put("JP5", 80.0, 106.0, 0)                  # cell -> VBAT
put("Q2", 69.5, 113.0, 0)                   # cell -> VSYS without USB
put("C3", 73.4, 113.0, 90)                  # VBAT
put("D7", 70.0, 117.0, 0)                   # Schottky across Q2: carries the unplug gap
put("C2", 73.4, 117.2, 90)                  # VSYS
put("U2", 79.2, 113.8, 0)                   # TP4056: its thermal pad spreads into the GND pours
put("C9", 76.4, 118.9, 0)                   # holds VSYS through the switch-over
put("D6", 73.4, 121.3, 0)                   # Q2's gate from VBUS
put("R6", 76.6, 121.0, 90)
put("R5", 79.0, 120.4, 90)                  # PROG
put("R9", 80.2, 120.4, 90)
put("R10", 81.4, 120.4, 90)
put("R39", 82.6, 120.4, 90)                 # TEMP divider: VBUS side
put("R40", 82.6, 123.4, 90)
put("R43", 80.2, 123.4, 90)
put("R44", 81.4, 123.4, 90)
put("Q4", 80.4, 127.0, 0)
put("C41", 131.8, 86.0, 90)                 # spare across TEMP, by TH1
put("TP7", 76.6, 129.8, 0)                  # LS_GATE
put("TP15", 74.0, 129.8, 0)                 # GND for a scope clip, by the charger
# the cell's temperature: TH1 at the hole's right edge, which the cell is pushed against
put("TH1", 128.6, 86.0, 90)

# power switch latch: D2 beside the LDO under seat 1's deck, D3 and Q1 at the charger's foot,
# its resistors and cap on the back under the LDO
put("D2", 112.8, 133.4, 0)
put("D3", 80.6, 130.2, 0)
put("Q1", 68.6, 134.2, 0)
put("TP6", 103.2, 138.6, 0, "B")                 # PWR_EN
put("R13", 109.6, 134.8, 0, "B")
put("R14", 109.6, 136.2, 0, "B")
put("R42", 111.8, 134.8, 0, "B")
put("R15", 109.6, 137.6, 0, "B")
put("R16", 109.6, 139.0, 0, "B")
put("C47", 111.8, 136.2, 0, "B")            # PWR_EN's slow-rise cap
put("R38", 110.4, 140.6, 0, "B")            # VBUS bleeder

# 3V3 LDO: under seat 1's deck right of the spine, close to the core it feeds, its caps on the back
put("U3", 108.6, 133.6, 0)
put("C5", 106.0, 135.0, 0, "B")             # VSYS in
put("C6", 106.0, 137.0, 0, "B")             # 3V3 out
put("C7", 106.0, 139.0, 0, "B")
put("JP1", 112.0, 129.0, 0)                # flat: under the folded FPC

# the reference's parts by its flash, placed here on the back right of the core: QSPI_SS's pull-up,
# its 1k to the BOOTSEL button, and a 3V3 decoupler
put("R23", 113.6, 125.5, 90, "B")
put("R25", 114.8, 125.5, 90, "B")
put("C10", 113.6, 128.4, 90, "B")

# battery sense: on the back right of the core, clear of its pin escapes
put("R17", 113.9, 106.6, 90, "B")
put("R18", 115.1, 106.6, 90, "B")
put("C8", 114.5, 109.6, 0, "B")

# audio: under seat 1's spine (the amplifier) and deck, between the clusters. The PWM comes in
# through the filter (R32 C37 R33 C38), Ci and Ri, in rows left of U7; Rf and the supply parts on
# its right; the outputs go to J4 by the speaker's hole
put("U7", 100.0, 136.4, 90)                 # FM8002A: 1 SHUTDOWN .. 4 -IN, 5 VO1 .. 8 VO2
put("R32", 90.6, 133.0, 90)
put("C37", 91.8, 133.0, 90)
put("R33", 93.0, 133.0, 90)
put("C38", 94.2, 133.0, 90)
put("C39", 95.4, 133.0, 90)                 # Ci
put("R34", 95.4, 136.0, 90)                 # Ri to -IN
put("R35", 94.2, 136.0, 90)                 # Rf, VO1 to -IN
put("C40", 93.0, 136.0, 90)                 # BYPASS
put("R36", 91.8, 136.0, 90)                 # SHUTDOWN pull-up
put("Q3", 88.0, 133.4, 0)                   # SHUTDOWN switch
put("R37", 92.0, 139.0, 0)
put("C34", 103.6, 140.2, 90)                # VDD
put("C35", 105.0, 139.6, 90)                # by VDD (pin 6)
put("JP3", 104.0, 143.2, 0)                 # VSYS -> amplifier
put("TP8", 84.0, 133.0, 0)                  # SHUTDOWN
put("TP16", 97.0, 141.6, 0)                 # GND for a scope clip, by the amplifier
put("J4", 128.0, 111.0, 0)                  # PicoBlade like J2: mouth towards the speaker's hole below

# test pads on the back, between seat 2's clusters
for i in range(5):
    put(f"TP{i + 1}", 92.0 + 4.0 * i, 53.0, 0, "B")

# 2026-10-05, from the decisions sheet: firmware charge control, PWR_HOLD pull-down, D5's
# damping resistor, bring-up jumpers and test pads, hand-solderable spare footprints. The REV
# straps sit on the back by their pins, left of the core
put("JP2", 113.6, 116.0, 0)                 # VSYS to LED_A (J3 pin 50), on the front under the FPC
put("R48", 88.6, 127.0, 0, "B")
put("R49", 88.6, 128.4, 0, "B")
put("R50", 88.6, 129.8, 0, "B")
put("R51", 88.6, 131.2, 0, "B")

# 2026-10-19, debug readiness (Alex's review): SWD pads, probe pads and isolation jumpers
put("J7", 80.0, 133.4, 0, "B")              # SWD pogo pads on the back, under the charger
_cx = CORE_AT[0] - 7.22                     # +1V1: on the core's own +1V1 via (the rail never leaves the core),
put("TP9", _cx, CORE_Y - 0.8 if CORE_SIDE == "B" else CORE_Y + 0.8, 0, "F" if CORE_SIDE == "B" else "B")
for i, ref in enumerate(("TP11", "TP12", "TP13", "TP14")):    # LCD SCK, MOSI, CS, DC on the back, below left of the core
    put(ref, 70.6 + 2.54 * i, 130.0, 0, "B")
