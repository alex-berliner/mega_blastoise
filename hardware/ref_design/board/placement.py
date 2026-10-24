"""Where every part goes on the board (KiCad frame, mm, degrees CCW).

Positions that the case owns (buttons, locating pins, edge connectors, pinholes)
are read from case/positions.json, which tools/export_case.py writes from
case/case.scad. The RP2350B core is not listed here: tools/build_pcb.py copies its
placement and routing from Raspberry Pi's minimal design (CORE below), turned so its
USB and flash side faces the USB-C, and turned over onto the back (CORE_SIDE).

Since 2026-10-21 the cell sits in a hole in the board (CUTOUTS), and since 2026-10-22 the speaker
lies on the board's top beside the screen (SPEAKER), so J2, J4, the debug connectors and the parts
round them are placed here, not by the case; the case learns both from case/CELLCUT_FOR_CASE.md.

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

# ---- the board 2 mm wider than the case had it (2026-10-22, Alex: up to 72.4 mm) ----------------
# 1 mm more on each side, still centred on the case, which grows to match: the USB-C and its tab
# go out with the right wall. The cell's hole moved 1 mm left (CUTOUTS), so the left strip keeps
# its 7.5 mm and the right strip beside J1 gains the whole 2 mm (8.9 -> 10.9): the nets out of it
# had only the 2.8 mm between J1's pads and the hole. case/CELLCUT_FOR_CASE.md has the outline.
# The case took it on 2026-10-22 (case.scad writes the 72.4 board, USB-C and tab into
# positions.json), so this adds nothing now; it stays so the placement can't widen it twice
WIDEN = 72.4 - CASE["board"]["w"]
CASE["board"]["w"] += WIDEN
CASE["usb"]["front_x"] += WIDEN / 2
CASE["usb"]["tab"] = [[x + WIDEN / 2, y] for x, y in CASE["usb"]["tab"]]

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
# The strips beside the hole (7.5 mm on the left, 8.9 on the right, 37 mm long) are the board's
# weakest part (2026-10-22 review): only the parts that must be there sit in them (since 2026-10-24,
# the routing levers: J5 on the left; TH1, J1 and the latch's VBUS resistors at the top of the
# right), and route.py stitches
# their two GND pours together (strip_stitch). The screws H1/H2 hold each strip's seat-2 end 2.8 mm past the
# hole; the seat-1 ends run into the full-width board. The case backs the strips from the tub's
# floor (case/CELLCUT_FOR_CASE.md, "The strips").
CUTOUTS = [
    dict(name="cell", box=(71.3, 66.8, 125.3, 103.8), r=1.0),     # 54.0 x 37.0
]

# ---- the speaker, lying on the board's top (2026-10-22, Alex) --------------------------------
# The INGHAi GSPK151035PN (15 x 10 x 3.5 mm racetrack, 8 ohm, 35 mm leads to a 1.25 mm plug) lies
# face up on the board in the right bezel beside the screen, its long axis along the board,
# stuck down with foam tape and plugged into J4 at assembly (not machine-placed), firing up
# through a grille in the face. The right bezel below the cell's hole is the only place it fits
# (the left bezel has the core); it is 28 mm long there, and J1 (10.7 mm), the speaker (16) and
# J4 with room for its plug (9.5) don't all fit, so J1 went back up its wall to the case's own
# position (USB_Y 100, beside the hole). build_pcb.py draws the speaker's outline on the
# silkscreen and keeps parts (not copper) out of its footprint plus `grow` (a rule area).
# 2026-10-22, the side screws back inward: 2.1 mm up, its top 0.1 below the hole's edge (J1 went up
# its wall to make room), so the screw H3 fits between it and J4
SPEAKER = dict(at=(128.7, 111.4), size=(10.0, 15.0), grow=0.5)   # KiCad x, y; x by y, mm

# ---- RP2350B core: copied from the reference board ------------------------------
# In the left bezel since the cell took the middle of the board (2026-10-21), as Raspberry Pi drew
# it: seat 2's pins face the strip beside the cell's hole, the panel's pins face J3, seat 1's
# cross is below. The decouplers on its long sides would land on the side screw, J2 or the folded
# FPC: they are placed by hand below, as close to their pins as that allows.
CORE_SIDE = "F"
CORE_AT = (77.8, 119.5)
CORE_Y = CORE_AT[1]
CORE_ESCAPES = True         # Raspberry Pi's pin escapes (False: straight stubs, build_pcb.straight_escapes)
CORE_ROT = 0                 # the reference as drawn: GPIO4-19 face the left bezel, GPIO33-47 J3
CORE = {                     # reference designator -> ours (same nets, same job); the RP2354B
    "U1": "U4", "X1": "X1", "L1": "L2",    # has its flash inside, so the reference's U3 and R10 stay behind,
    "C12": "C11", "C13": "C12", "C10": "C21",   # and its C2, R1 and R6 (C10, R23, R25 here) sat out by
    "C6": "C24", "C7": "C25", "C9": "C26",      # the flash; the decouplers C13-C20, C22 and C23 here
    "C3": "C27", "C4": "C28",                   # are placed by hand (the screw, J2 and the folded FPC)
    "R2": "R20", "R3": "R19", "R7": "R21", "R8": "R22",
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
# Back inward (2026-10-22, Alex: |case y| <= 29, inside the top piece, no shoulders): H1/H2 at the
# seat-2 ends of the strips (the wider board left them room; the parts that sat there went down
# the strip or up where the old H1 was), H3 between the speaker and J4 in the right bezel, H4
# between J2 and the LDO in the left bezel. Each hole keeps 2 mm or more from the board's and the
# cell hole's edges, each 5 mm boss 0.7 or more
SIDE_SCREWS = {0: (132.5, 71.0), 1: (67.5, 71.0), 2: (125.9, 121.95), 3: (67.0, 115.2)}
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
put("FID1", 65.3, 102.6)
put("FID2", 133.2, 141.6)
put("FID3", 66.8, 141.6)

# display: the panel's ZIF connector where its folded FPC reaches (case.scad conn_in), its mouth
# towards seat 1; the panel's supply caps and jumper left of it, the SPI terminations by the core
put("J3", CASE["fpc_conn"]["x"], CASE["fpc_conn"]["y"], 0)
put("C29", 84.3, 132.2, 90)                  # panel VCI/VDDI
put("C30", 85.5, 132.2, 90)
put("JP4", 90.0, 124.0, 0)                  # 3V3 -> panel
put("R28", 81.2, 105.2, 90)                # SPI source terminations, by the core's panel pins
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
put("TP16", 93.0, 141.6, 0)                 # GND for a scope clip

# debug: left bezel
# the debug buttons (Alex: anywhere, next to each other; the case's pinholes follow): since
# 2026-10-24 (routing levers) out of the left strip, side by side in the left bezel below the screw
# H4, where JP1 was (each 5.4 mm or more from H4's centre, case.scad's pinhole assert), their 1k
# resistors left of U3 (which, with its caps, went 2 mm down for them). Lying across the bezel,
# one above the other, between the island's joint along the left edge (x <= 64.6) and the core's
# decouplers. The strip keeps J5 alone
PINHOLES = [(67.65, 120.7), (67.65, 124.35)]
put("SW18", *PINHOLES[0], 0)
put("SW19", *PINHOLES[1], 0)
put("R25", 65.7, 127.2, 90)                # QSPI_SS -> BOOTSEL, left of U3
put("R26", 65.7, 129.5, 90)                # RUN -> RUN_SW
# J5 (SWD) turned so its mouth faces the board's left edge (2026-10-22, Alex): the plug goes in
# from the side with the case open, the pins face the cell's hole. J6 (UART) went on 2026-10-24
put("J5", 67.4, 84.8, 270)                  # SWD

# edge part on the right: USB-C
# Moved down the right wall below the cell's hole (2026-10-21, Alex: it may slide along its wall):
# beside the hole its pads faced the cut-out with no room to bring their tracks out. The case
# moves its opening and the tab's slot to match (case/CELLCUT_FOR_CASE.md)
# Back at the case's own position (2026-10-22): the speaker and J4 took the bezel below the hole.
# Its pads face the hole: the USB pair drops to the back beside them (route.py usb_pair()), the
# other pads come down the 2.8 mm between them and the hole's edge
# 2 mm up its wall (2026-10-22, the side screws back inward), clear of the speaker that moved up
USB_Y = 98.0                                # KiCad y of the receptacle's centre (case y 2.0)
_dy = USB_Y - CASE["usb"]["y"]
USB_TAB = [[x, y + _dy] for x, y in CASE["usb"]["tab"]]    # the board's tab under the receptacle
usb_x = CASE["usb"]["front_x"] - 3.65       # footprint front is 3.65 ahead of its origin
put("J1", usb_x, USB_Y, 90)

# USB input
# U1 (ESD) on the USB pair, turned so D+ and D- each run over their own pin (pins 1 and 3) with
# no stub: route.py's usb_pair() draws the pair from J1 through U1 to R21/R22 by the core. Under
# the glass's right edge, left of the speaker. The CC resistors in the bezel above J4's plug (in
# a row across the strip above J1 they walled off its tracks); C42, CC1's spare, along the hole's
# edge in the strip
put("U1", 120.6, 106.5, 0)
put("R3", 130.0, 120.0, 0)                  # since 2026-10-22 right of the screw H3, above J4
put("R4", 132.0, 120.0, 0)
put("R1", 130.0, 121.15, 0)
put("R2", 132.0, 121.15, 0)
put("C1", 120.3, 109.4, 0)                  # below the USB pair
put("R41", 120.3, 111.1, 0)                # C1's series resistor (hot-plug ringing)
put("C42", 126.0, 127.2, 0)                 # spares on CC1 and CC2 (C42 out of the right strip, 2026-10-24)
put("C43", 125.7, 130.4, 0)

# the cell: J2 at its hole's seat-1 corner, mouth into the hole's lead pocket, so the plug's
# back and wires drop straight down beside the cell's left end; the charger and the load sharing
# below it
put("J2", 69.3, 107.24, 180)                # PicoBlade: mouth towards the hole, contacts behind
put("JP5", 90.0, 117.0, 0)                  # cell -> VBAT
put("C3", 116.9, 110.0, 90)                  # VBAT
put("C2", 120.3, 112.8, 0)                   # VSYS
put("U2", 121.0, 116.0, 0)                   # TP4056: its thermal pad spreads into the GND pours
put("R5", 116.6, 120.4, 90)                  # PROG
put("R9", 121.4, 120.4, 90)
put("R10", 122.6, 120.4, 90)
put("R43", 119.0, 120.4, 90)
put("R44", 120.2, 120.4, 90)
put("TP7", 113.0, 119.6, 0)                 # CHG_TEMP: flat, under the folded FPC by U2
put("TP15", 113.0, 130.0, 0)                 # GND for a scope clip, by the charger: flat, under the folded FPC
# the cell's temperature: TH1 at the hole's right edge, which the cell is pushed against
put("TH1", 126.1, 86.0, 90)

# power switch latch: right strip, below the screw H1, its first rows above it (under seat 2's
# deck, where the old H1 was)
put("D2", 132.6, 63.4, 0)
put("D3", 132.7, 66.6, 180)
put("R13", 128.6, 63.6, 90)
put("R42", 129.8, 63.6, 90)
put("R14", 128.6, 66.6, 90)
put("R38", 130.0, 66.8, 90)
# 2026-10-24 (routing levers): the latch's parts that sat in the right strip (Q1, R15, R16, C47,
# TP6 and seat 2's Start: D8, Q7, R62, R63) moved up past the hole's top edge under seat 2's right
# deck, between U9 and the cluster's retainer ring (14.5 mm round its centre for parts over
# 0.5 mm), Q1 and R16 left of U9: the strip keeps only TH1 and J1. P2_START_RAW no longer
# crosses the board
put("R62", 104.8, 65.4, 90)                 # row by the hole's edge
put("R63", 105.9, 65.4, 90)
put("R15", 107.0, 65.4, 90)
put("C47", 108.1, 65.4, 90)                 # PWR_EN's slow-rise cap
put("TP6", 109.7, 65.4, 0)                  # PWR_EN
put("Q7", 105.9, 62.3, 0)                   # seat 2's Start into P2_START
put("D8", 109.6, 62.6, 0)                   # seat 2's Start into PWR_EN
put("Q1", 90.5, 64.5, 0)                    # seat 1's Start into PWR_BTN_N: left of U9 (no room by D8)
put("R16", 93.6, 64.8, 90)

# 3V3 LDO: in the left bezel under the core it feeds
put("U3", 68.6, 128.4, 90)                  # 2 mm down since 2026-10-24 (the debug buttons above it)
put("C5", 66.4, 133.0, 90)                  # VSYS in
put("C6", 68.6, 133.0, 90)                  # 3V3 out)
put("C7", 70.8, 133.0, 90)
put("JP1", 73.6, 130.6, 0)                  # below the core, right of C7 (the debug buttons took its place)

# the core's loose parts: the reference's parts by its flash (QSPI_SS's pull-up, its 1k to the
# BOOTSEL button, a 3V3 decoupler) and the decouplers that would land on the screw or the FPC
put("R23", 71.2, 113.0, 90)                # QSPI_SS pull-up, by the core (it was across the board)
put("C10", 121.0, 130.8, 90)

# battery sense
put("R17", 116.6, 105.4, 90)
put("R18", 116.6, 107.45, 90)
put("C8", 116.6, 112.6, 90)

# test pads on the back, between seat 2's clusters
for i in range(5):
    put(f"TP{i + 1}", 92.0 + 4.0 * i, 53.0, 0, "B")
put("TP3", 89.4, 114.0, 0, "B")             # VBAT: on the back under J3, by JP5 and the cell's path


# 2026-10-19, debug readiness (Alex's review): SWD pads, probe pads and isolation jumpers
put("J7", 100.0, 140.4, 0, "B")              # SWD pogo pads on the back, under seat 1's spine
put("TP9", *core_pt(0.8, 7.22), 0, "B" if CORE_SIDE == "F" else "F")    # +1V1: on the core's own +1V1 via
put("C23", 80.6, 107.9, 90)
put("C13", 80.0, 105.2, 90)
put("C19", 73.6, 126.8, 90)
put("R64", 117.8, 120.4, 90)                # the charger's ILIM
put("U9", 100.0, 58.5, 90)                  # seat 2's buttons: a 74HC165 between the clusters, 1.2 mm
put("RN3", 94.2, 57.0, 90)
put("RN4", 94.2, 60.4, 90)
put("C53", 100.0, 53.6, 0)
put("R65", 82.6, 132.7, 90)                # SR_CLK's series resistor, below the core's escapes (GPIO29, pin 37)
                                            # and clear of the folded FPC: about 12 mm of track from the pin
put("U8", 101.2, 144.2, 90)                 # seat 1's, below the backlight's op-amp
put("RN1", 105.6, 143.0, 90)
put("RN2", 105.6, 146.4, 90)
put("C52", 104.6, 148.8, 0)
put("C14", 71.2, 117.0, 90)                # the core's decouplers, placed by hand (above)
put("C15", 86.7, 132.2, 90)
put("C17", 87.9, 132.2, 90)
put("C18", 77.6, 105.2, 90)
put("C22", 86.7, 134.4, 90)
put("C16", 71.2, 121.6, 270)
put("C20", 71.2, 119.3, 90)
for i, ref in enumerate(("TP11", "TP12", "TP13", "TP14")):    # LCD SCK, MOSI, CS, DC on the back, under J3's lead-in
    put(ref, 92.2 + 2.54 * i, 110.0, 0, "B")

# audio (2026-10-22): the amplifier under the glass's right edge beside the speaker, its supply and
# bypass parts in the column left of it, the PWM filter, Ci, Ri and Rf in two rows below under
# seat 1's deck, J4 in the bezel below the speaker, its mouth (and the plug) towards the glass
put("U7", 120.3, 125.5, 90)                 # FM8002A: VO1 (5) and VO2 (8) on top, -IN (4) bottom right
put("C34", 116.75, 123.2, 90)               # VDD
put("C35", 116.6, 125.8, 90)
put("C40", 116.6, 127.8, 90)                # BYPASS
put("R36", 116.6, 129.75, 90)               # SHUTDOWN pull-up
put("R37", 116.55, 132.45, 0)
put("R34", 118.53, 132.45, 0)               # Ri to -IN
put("R35", 120.51, 132.45, 0)               # Rf, VO1 to -IN
put("C39", 122.49, 132.45, 0)               # Ci
put("R32", 116.55, 133.6, 0)
put("C37", 118.53, 133.6, 0)
put("R33", 120.51, 133.6, 0)
put("C38", 122.49, 133.6, 0)
put("Q3", 125.5, 133.4, 0)                  # SHUTDOWN switch: under seat 1's deck, out of the cluster's band
put("TP8", 113.0, 123.5, 0)                 # SHUTDOWN: flat, under the folded FPC like JP2
put("JP3", 113.0, 127.0, 0)                 # VSYS -> amplifier
put("J4", 131.9, 127.0, 270)                # PicoBlade like J2: mouth towards the glass, pins by the wall
