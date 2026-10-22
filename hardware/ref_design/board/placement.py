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

# ---- the hole the cell sits in (2026-10-21, row 19 of Alex's list) ------------------------------
# The case is too thick with the cell under the board, so it sits in a cutout in the board's
# plane. The cell (403450: 50 +-2 x 34 +-2 x 4.0 +-0.5) lies across the board between seat 2's
# deck and J3's range, the only place a 52 x 36 cell fits: 0.5 mm round a worst-case cell, and
# 1.0 mm more at its left end, where its leads come out and run to J2 at the hole's seat-1 corner
# (pushed right, a worst-case cell leaves them 2 mm there, a nominal one 4 mm). Any longer and
# the USB-C's pads, which face the hole, can't get their tracks out.
# The case pushes the cell to the right end, where TH1 sits at the edge. The hole keeps 1 mm or
# more from the screw bosses, the clusters' rings, J3's range and USB-C. Every machine-placed part
# stays on the top (Alex: one-sided assembly), and with the core moved to the right bezel there
# was no room left for the speaker (row 18): it and its amplifier went.
CUTOUTS = [
    dict(name="cell", box=(72.3, 66.8, 126.3, 103.8), r=1.0),     # 54.0 x 37.0
]

# ---- RP2350B core: copied from the reference board ------------------------------
# In the left bezel since the cell took the middle of the board (2026-10-21), as Raspberry Pi drew
# it: seat 2's pins face the strip beside the cell's hole, the panel's pins face J3, seat 1's
# cross is below. The decouplers on its long sides would land on the side screw, J2 or the folded
# FPC: they are placed by hand below, as close to their pins as that allows.
CORE_SIDE = "F"
CORE_AT = (78.5, 119.5)
CORE_Y = CORE_AT[1]
CORE_ESCAPES = False        # straight stubs (build_pcb.straight_escapes): Raspberry Pi's 45 degree escapes run into the side screw here
CORE_ROT = 0                 # the reference as drawn: GPIO4-20 face the left strip, GPIO33-47 J3
CORE = {                     # reference designator -> ours (same nets, same job); the RP2354B
    "U1": "U4", "X1": "X1", "L1": "L2",    # has its flash inside, so the reference's U3 and R10 stay behind,
    "C12": "C11", "C13": "C12", "C10": "C21",   # and its C2, R1 and R6 (C10, R23, R25 here) sat out by
    "C6": "C24", "C7": "C25",                   # the flash; the decouplers C13-C20, C22, C23, C26 and R19 here
    "C3": "C27", "C4": "C28",                   # are placed by hand (the screw, J2 and the folded FPC)
    "R2": "R20", "R7": "R21", "R8": "R22",
}
CORE_NETS = {                # reference net -> ours, for the copied tracks and zones
    "+1V1": "+1V1", "+3V3": "+3V3", "GND": "GND", "/QSPI_SS": "QSPI_SS",
    "/VREG_AVDD": "VREG_AVDD", "/VREG_LX": "VREG_LX",
    "/XIN": "/MCU/XIN", "/XOUT": "/MCU/XOUT", "Net-(C4-Pad1)": "/MCU/XO",
    "Net-(U1-USB_DM)": "/MCU/USB_DM_MCU", "Net-(U1-USB_DP)": "/MCU/USB_DP_MCU",
}



def core_pt(dx, dy):
    """A point of Raspberry Pi's layout, dx, dy from its U1, on this board (build_pcb.core_xy)."""
    import math
    a = math.radians(CORE_ROT)
    nx = dx * math.cos(a) + dy * math.sin(a)
    ny = -dx * math.sin(a) + dy * math.cos(a)
    return round(CORE_AT[0] + nx, 3), round(CORE_AT[1] + (-ny if CORE_SIDE == "B" else ny), 3)


# ---- everything else ------------------------------------------------------------
b = CASE["buttons"]
# the screws (2026-10-21, Alex: any of the six may move): the four side screws went to the ends
# of the long sides, past the cell's hole, where the board is full width; in the strips beside
# the hole they closed half of each. The end screws stay. The case moves its bosses to match
SIDE_SCREWS = {0: (131.4, 64.0), 1: (68.6, 64.0), 2: (131.4, 136.0), 3: (68.6, 136.0)}
s = [SIDE_SCREWS.get(i, xy) for i, xy in enumerate(CASE["pins"])]
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
# towards seat 1; the panel's supply caps and jumper left of it, the SPI terminations by the core
put("J3", CASE["fpc_conn"]["x"], CASE["fpc_conn"]["y"], 0)
put("C29", 84.3, 132.2, 90)                  # panel VCI/VDDI
put("C30", 85.5, 132.2, 90)
put("JP4", 90.0, 124.0, 0)                  # 3V3 -> panel
put("R28", 83.0, 113.3, 90)                # SPI source terminations, by the core's panel pins
put("R29", 76.4, 105.2, 90)

# backlight current sinks: under seat 1's spine (the op-amp, 1.75 mm) and deck, J3's LED pins
# 25 mm up under the folded FPC. Channel 2 (Q6, LED_K2) on the left, channel 1 (Q5, LED_K1) on
# the right, each FET beside its sense resistor, the op-amp's parts in rows below
put("U6", 100.0, 136.4, 90)                 # op-amp half A runs LED_K2, half B LED_K1
put("Q6", 93.6, 133.4, 0)                   # LED_K2 (J3 pins 46, 48)
put("R53", 90.0, 133.4, 90)                 # sense resistors
put("Q5", 106.4, 133.4, 0)                  # LED_K1 (J3 pins 47, 49)
put("R52", 110.0, 133.4, 90)
put("R57", 89.0, 136.5, 90)                 # channel 2: gate pull-down, gate resistor
put("R55", 90.2, 136.5, 90)
put("C49", 91.4, 136.5, 90)                 # channel 2: feedback C and R
put("R59", 92.6, 136.5, 90)
put("R60", 93.8, 136.5, 90)                 # PWM into BL_REF
put("C50", 95.0, 136.5, 90)                 # BL_REF filter
put("R61", 91.4, 139.0, 90)
put("TP10", 93.6, 139.4, 0)                 # BL_REF
put("R56", 105.0, 136.5, 90)                # channel 1: gate pull-down, gate resistor
put("R54", 106.2, 136.5, 90)
put("C48", 107.4, 136.5, 90)                # channel 1: feedback C and R
put("R58", 108.6, 136.5, 90)
put("C51", 109.8, 136.5, 90)                # U6 supply
put("JP2", 113.0, 116.0, 0)                 # VSYS to LED_A (J3 pin 50): flat, under the FPC
put("TP16", 97.0, 141.6, 0)                 # GND for a scope clip

# debug: left bezel, beside the cell's hole
put("SW18", CASE["pinholes"][0][0], CASE["pinholes"][0][1], 90)
put("SW19", CASE["pinholes"][1][0], CASE["pinholes"][1][1], 90)
put("R26", 71.2, 91.0, 90)
put("J6", 68.75, 71.65, 0)                  # UART: above the side screw, clear of seat 2's deck
put("J5", 68.8, 128.4, 180)                 # SWD: below the seat-1 side screw (J2's plug took its place)

# edge part on the right: USB-C
# Moved down the right wall below the cell's hole (2026-10-21, Alex: it may slide along its wall):
# beside the hole its pads faced the cut-out with no room to bring their tracks out. The case
# moves its opening and the tab's slot to match (case/CELLCUT_FOR_CASE.md)
USB_Y = 111.0                               # KiCad y of the receptacle's centre (case y -11.0; was 100)
_dy = USB_Y - CASE["usb"]["y"]
USB_TAB = [[x, y + _dy] for x, y in CASE["usb"]["tab"]]    # the board's tab under the receptacle
usb_x = CASE["usb"]["front_x"] - 3.65       # footprint front is 3.65 ahead of its origin
put("J1", usb_x, USB_Y, 90)

# USB input
put("U1", 125.0, 107.6, 90)                 # ESD, beside the core's crystal)
put("R1", 122.0, 106.0, 90)
put("R3", 120.8, 106.0, 90)
put("R2", 122.0, 108.4, 90)
put("R4", 120.8, 108.4, 90)
put("C1", 125.2, 111.6, 0)
put("R41", 125.2, 113.4, 0)                # C1's series resistor (hot-plug ringing)
put("D5", 121.0, 111.0, 0)                  # VBUS -> VSYS)
put("R47", 121.0, 113.6, 0)                # D5's damping resistor)
put("C42", 132.4, 126.6, 0)                 # spares on CC1 and CC2)
put("C43", 132.0, 129.8, 0)

# the cell: J2 at its hole's seat-1 corner, mouth into the hole's lead pocket, so the plug's
# back and wires drop straight down beside the cell's left end; the charger and the load sharing
# below it, round the screw at 68.6, 122
put("J2", 70.3, 107.24, 180)                # PicoBlade: mouth towards the hole, contacts behind
put("JP5", 90.0, 117.0, 0)                  # cell -> VBAT
put("Q2", 126.0, 116.2, 0)                   # cell -> VSYS without USB
put("C3", 117.0, 123.6, 90)                  # VBAT
put("D7", 126.4, 119.8, 0)                   # Schottky across Q2: carries the unplug gap
put("C2", 116.8, 131.8, 90)                  # VSYS
put("U2", 120.0, 118.6, 0)                   # TP4056: its thermal pad spreads into the GND pours
put("C9", 120.6, 123.8, 0)                   # holds VSYS through the switch-over
put("D6", 126.4, 122.8, 0)                   # Q2's gate from VBUS
put("R6", 124.0, 124.8, 90)
put("R5", 117.0, 126.4, 90)                  # PROG
put("R9", 118.2, 126.4, 90)
put("R10", 119.4, 126.4, 90)
put("R39", 120.6, 126.4, 90)                 # TEMP divider: VBUS side
put("R40", 117.0, 128.8, 90)
put("R43", 118.2, 128.8, 90)
put("R44", 119.4, 128.8, 90)
put("Q4", 123.6, 128.8, 0)
put("TP7", 122.0, 133.2, 0)                  # LS_GATE
put("TP15", 124.6, 133.2, 0)                 # GND for a scope clip, by the charger
# the cell's temperature: TH1 at the hole's right edge, which the cell is pushed against
put("TH1", 127.1, 86.0, 90)
put("C41", 133.4, 90.4, 90)                 # spare across TEMP)

# power switch latch: right strip, above and below the screw at 131.4, 78
put("D2", 132.6, 70.2, 0)
put("D3", 132.7, 73.4, 180)
put("R13", 128.6, 70.4, 90)
put("R42", 129.8, 70.4, 90)
put("R14", 128.6, 73.4, 90)
put("R38", 130.0, 73.6, 90)
put("Q1", 131.5, 82.9, 0)
put("R15", 133.6, 86.4, 90)
put("R16", 131.0, 85.8, 0)
put("C47", 131.0, 87.4, 0)                  # PWR_EN's slow-rise cap)
put("TP6", 129.6, 89.6, 0)                  # PWR_EN)

# 3V3 LDO: under seat 2's deck, by the latch at the right strip's top
put("U3", 110.0, 64.4, 0)
put("C5", 106.4, 64.6, 90)                  # VSYS in
put("C6", 104.4, 64.6, 90)                  # 3V3 out)
put("C7", 102.6, 64.6, 90)
put("JP1", 100.0, 61.4, 0)

# the core's loose parts: the reference's parts by its flash (QSPI_SS's pull-up, its 1k to the
# BOOTSEL button, a 3V3 decoupler) and the decouplers that would land on the screw or the FPC
put("R23", 111.4, 134.4, 90)
put("R25", 71.2, 84.4, 90)
put("C10", 121.0, 130.8, 90)

# battery sense
put("R17", 126.6, 128.8, 90)
put("R18", 127.8, 128.8, 90)
put("C8", 129.0, 128.8, 90)

# test pads on the back, between seat 2's clusters
for i in range(5):
    put(f"TP{i + 1}", 92.0 + 4.0 * i, 53.0, 0, "B")

# 2026-10-05: the board revision straps, by their pins
put("R48", 111.4, 132.2, 90)
put("R49", 112.6, 132.2, 90)
put("R50", 113.8, 132.2, 90)
put("R51", 115.0, 132.2, 90)

# 2026-10-19, debug readiness (Alex's review): SWD pads, probe pads and isolation jumpers
put("J7", 100.0, 140.4, 0, "B")              # SWD pogo pads on the back, under seat 1's spine
put("TP9", *core_pt(0.8, 7.22), 0, "B" if CORE_SIDE == "F" else "F")    # +1V1: on the core's own +1V1 via
put("C23", 82.6, 108.4, 90)
put("C13", 80.0, 105.2, 90)
put("C19", 73.6, 126.8, 90)
put("R19", 77.6, 107.6, 90)
put("C26", 81.2, 105.2, 90)
put("C14", 78.8, 107.6, 90)                # the core's decouplers, placed by hand (above)
put("C15", 86.7, 132.2, 90)
put("C17", 87.9, 132.2, 90)
put("C18", 77.6, 105.2, 90)
put("C22", 86.7, 134.4, 90)
put("C16", 80.0, 107.6, 90)
put("C20", 81.2, 107.6, 90)
for i, ref in enumerate(("TP11", "TP12", "TP13", "TP14")):    # LCD SCK, MOSI, CS, DC on the back, under J3's lead-in
    put(ref, 92.2 + 2.54 * i, 110.0, 0, "B")
