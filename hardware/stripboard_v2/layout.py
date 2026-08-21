#!/usr/bin/env python3
"""Single source of truth for the mega_blastoise single-screen stripboard.

Everything physical about the board is described here in hole coordinates:
column 1..24 left to right, row 1..56 from Player 2's edge to Player 1's edge.
The KiCad board, BUILD.md, BOM.csv and the SVG preview are all generated from
this file, so they can never disagree.

Substrate: ONE 24x56 stripboard (~61 x 142 mm), strips running the long way.
The display stands portrait in the middle and each player gets a BAND at their
own end of the board holding both of their button crosses side by side. The
crosses are the COMPACT pattern: N/S arms 10.16 mm apart, W/E arms 15.24 mm,
staggered so no two switches share a hole. That compaction is what makes the
single board work, and it is also why the button caps must shrink to 9 mm -
the 11.3 mm caps collide at this pitch.

Cut plan is three straight lines (rows 12, 30, 44) plus 13 singles: the fences
at 12 and 44 isolate each band from the middle, and row 30 splits every column
under the Pico. Between the fences, every column's segment is a free wire that
usually already carries a Pico pin.

Run with no arguments to write BUILD.md, BOM.csv and preview.svg next to this
file, and to dump the KiCad placement data as JSON on stdout.
"""

import itertools
import math
import json
import sys
from pathlib import Path

HERE = Path(__file__).parent

# ---------------------------------------------------------------- board grid

PITCH = 2.54
COLS = 24                 # strips, running top to bottom
ROWS = 56
BOARD_W = 60.96
BOARD_H = 142.24
MARGIN_X = (BOARD_W - (COLS - 1) * PITCH) / 2     # 1.27 mm
MARGIN_Y = (BOARD_H - (ROWS - 1) * PITCH) / 2     # 1.27 mm

# Where the board sits on the KiCad page.
ORIGIN_X = 20.0
ORIGIN_Y = 20.0


def col_name(col):
    """Columns are lettered A..X, the usual stripboard convention and the one
    the earlier boards in this project used. 24 columns lands exactly on X."""
    return chr(ord("A") + col - 1)


def hole_name(col, row):
    """A hole as it is written everywhere a human reads it: E12, W44."""
    return f"{col_name(col)}{row}"


def hole(col, row):
    """Board-relative mm position of a hole."""
    return (MARGIN_X + (col - 1) * PITCH, MARGIN_Y + (row - 1) * PITCH)


def page(col, row):
    """KiCad page mm position of a hole."""
    x, y = hole(col, row)
    return (round(ORIGIN_X + x, 3), round(ORIGIN_Y + y, 3))


# ------------------------------------------------------------------ the Pico
#
# The Pico lies across the board under the display, USB pointing left, its two
# pin rows on rows 26 and 33 and its 20 pins per row on columns 3..22. Row 33
# carries pins 1..20 left to right; row 26 carries 40..21. The cut on row 30
# isolates the two rows, and the fences at rows 12 and 44 isolate the button
# bands, so each column's middle segment belongs to exactly one Pico pin.

PICO_COL0 = 3
PICO_ROW_LOWER = 37       # pins 1..20  (GP0..GP15) -> Player 1 signals
PICO_ROW_UPPER = 30       # pins 40..21 (GP16..GP28, power) -> Player 2
PICO_CUT_ROW = 34
FENCE_NORTH = 12          # isolates the P2 band (rows 1-11)
FENCE_SOUTH = 44          # isolates the P1 band (rows 45-56)

PICO_PIN_NAME = {
    1: "GP0", 2: "GP1", 3: "GND", 4: "GP2", 5: "GP3", 6: "GP4", 7: "GP5",
    8: "GND", 9: "GP6", 10: "GP7", 11: "GP8", 12: "GP9", 13: "GND",
    14: "GP10", 15: "GP11", 16: "GP12", 17: "GP13", 18: "GND", 19: "GP14",
    20: "GP15", 21: "GP16", 22: "GP17", 23: "GND", 24: "GP18", 25: "GP19",
    26: "GP20", 27: "GP21", 28: "GND", 29: "GP22", 30: "RUN", 31: "GP26",
    32: "GP27", 33: "AGND", 34: "GP28", 35: "ADC_VREF", 36: "3V3_OUT",
    37: "3V3_EN", 38: "GND", 39: "VSYS", 40: "VBUS",
}


def lower_pin(col):
    return col - PICO_COL0 + 1        # column 3 -> pin 1 ... 22 -> pin 20


def upper_pin(col):
    return 41 - (col - PICO_COL0 + 1)  # column 3 -> pin 40 ... 22 -> pin 21


def col_of_pin(pin):
    return PICO_COL0 + (pin - 1) if pin <= 20 else PICO_COL0 + (40 - pin)


# ------------------------------------------------------------------ switches
#
# The COMPACT cross. Each 6 mm switch still straddles two columns 3 apart
# (its terminals) and two rows 2 apart (the paired legs of one terminal), but
# the four switches interlock:
#
#          [N]        N: cols A+4/A+7, rows B-3/B-1
#     [W]      [E]    W: cols A/A+3,   rows B-1/B+1
#          [S]        E: cols A+8/A+11, rows B-1/B+1
#                     S: cols A+4/A+7, rows B+1/B+3
#
# Centre distances: N-S 10.16 mm, N-W and N-E 11.4 mm, W-E 20.3 mm. No two
# switches share a hole, and one cut at (A+4, B) separates N's terminal from
# S's; their other terminals share column A+7 as common ground, uncut.
# A cross spans 12 columns, so two crosses fill the 24-column band exactly.
# Caps are 9 mm: the tightest pair (N-S, and E of one cross to W of the next)
# is 10.16 mm centre to centre, leaving 1.2 mm between cap edges.

CROSSES = {
    # name:            (base col A, base row B)
    "P1_DPAD":         (1, 52),     # Player 1 (near seat), board-left
    "P1_ACTIONS":      (13, 52),    # Player 1, board-right
    "P2_ACTIONS":      (1, 5),      # Player 2, board-left
    "P2_DPAD":         (13, 5),     # Player 2, board-right
}

# Which switch of each cross carries which label. Board compass -> button.
# Player 2 sits at the opposite edge, so their up/down and left/right are the
# board's south/north and east/west.
CROSS_LABELS = {
    "P1_DPAD":    {"N": "P1_UP",   "S": "P1_DOWN", "W": "P1_LEFT", "E": "P1_RIGHT"},
    "P1_ACTIONS": {"N": "P1_MENU", "S": "P1_B",    "W": "P1_INFO", "E": "P1_A"},
    "P2_ACTIONS": {"N": "P2_B",    "S": "P2_MENU", "W": "P2_A",    "E": "P2_INFO"},
    "P2_DPAD":    {"N": "P2_DOWN", "S": "P2_UP",   "W": "P2_RIGHT", "E": "P2_LEFT"},
}

SWITCH_REFS = {
    "P1_UP": "SW1", "P1_DOWN": "SW2", "P1_LEFT": "SW3", "P1_RIGHT": "SW4",
    "P1_A": "SW5", "P1_B": "SW6", "P1_INFO": "SW7", "P1_MENU": "SW8",
    "P2_UP": "SW9", "P2_DOWN": "SW10", "P2_LEFT": "SW11", "P2_RIGHT": "SW12",
    "P2_A": "SW13", "P2_B": "SW14", "P2_INFO": "SW15", "P2_MENU": "SW16",
}


def cross_switches(base_col, base_row):
    """Return {compass: (col_left, row_top)} for the four switches of a cross.

    A switch footprint's origin is its top-left pad; pads sit at
    (col, row), (col, row+2), (col+3, row), (col+3, row+2).
    """
    a, b = base_col, base_row
    return {
        "N": (a + 4, b - 3),
        "W": (a,     b - 1),
        "E": (a + 8, b - 1),
        "S": (a + 4, b + 1),
    }


# ------------------------------------------------------ the electrical map
#
# Band segments (rows 1-11 and 45-56) carry the button nets; every signal gets
# one wire from its band strip to the middle segment of its GPIO's column,
# where the Pico pin already sits on the strip.

BUTTON_NETS = [
    # net,        strip column, rows, Pico pin, how
    #
    # "direct" means the fence cut is OMITTED on that column, so the button's
    # band strip merges with the Pico-pin segment: the strip IS the wire.
    ("P1_LEFT",   4,  "35-56", 2,  "direct"),
    ("P1_UP",     8,  "35-51", 6,  "direct"),
    ("P1_RIGHT",  12,  "48-56", 4,  "jumper (12,55)->(6,36)"),
    ("P1_DOWN",   8,  "53-56", 5,  "jumper (8,56)->(7,36)"),
    ("P1_INFO",   13,  "48-56", 17, "jumper (13,55)->(19,36)"),
    ("P1_MENU",   17,  "35-51", 15, "direct"),
    ("P1_A",      21,  "35-56", 19, "direct"),
    ("P1_B",      17,  "53-56", 16, "jumper (17,56)->(18,36)"),
    ("P2_A",      1,  "1-43",  32, "jumper (1,2)->(11,15)"),
    ("P2_B",      8,  "1-4",   29, "jumper (8,1)->(14,15)"),
    ("P2_INFO",   12,  "1-33",  31, "direct"),
    ("P2_MENU",   8,  "6-11",  25, "jumper (8,10)->(18,15)"),
    ("P2_RIGHT",  16,  "1-33",  27, "direct"),
    ("P2_DOWN",   17,  "1-4",   24, "jumper (17,1)->(19,15)"),
    ("P2_LEFT",   21,  "1-33",  22, "direct"),
    ("P2_UP",     17,  "6-33",  26, "direct"),
]

# The module Alex actually received is pin-REVERSED relative to the usual
# ST7796 boards: read at the socket, GND arrives at column P and BL at column
# I. The socket stays put and the strips still do the work - five signals fall
# through to the GPIO directly below their column - but the roles move.
# The price of the reversal is hardware SPI: SCK lands over GP9 and MOSI over
# GP8, a pair no SPI block can drive, so the firmware clocks the panel by PIO.
# GND now needs its column split off GP10 and bridged to the P1 bus; CS takes
# the orphaned GP10 through the one new signal wire; VCC keeps its wire from
# 3V3, just to the other end of the socket.
DISPLAY_HEADER_COL0 = 9       # J1 pin 1; pins 1..8 on columns 9..16
DISPLAY_HEADER_ROW = 46

DISPLAY_NETS = [
    # socket position (J1 pin), net, strip column, rows, Pico pin, how.
    # The MODULE's own pin at each position is 9 minus the socket position.
    (1, "LCD_BL",    9,  "35-47", 7,  "direct to GP5; module pin 8 lands here"),
    (2, "LCD_CS",    10, "40-56", 14, "jumper (10,41)->(16,41) to GP10"),
    (3, "LCD_DC",    11, "35-56", 9,  "direct to GP6"),
    (4, "LCD_RST",   12, "35-46", 10, "direct to GP7"),
    (5, "LCD_MOSI",  13, "35-46", 11, "direct to GP8"),
    (6, "LCD_SCK",   14, "35-56", 12, "direct to GP9"),
    (7, "VCC",       15, "40-56", 36, "jumper (7,33)->(15,41) from 3V3"),
    (8, "GND",       16, "43-47", 8,  "bridge (16,47)->(16,49) onto the P1 bus segment"),
]

# Every track cut. A cut destroys the hole it names. The fences skip the
# columns where a net deliberately runs through: that is where the direct
# connections live, so treat the exception lists as load-bearing.
# Only the load-bearing fence cuts survive: a fence cut exists where the band
# net and the middle net are BOTH real and different. Columns whose band or
# middle is empty copper skip the cut - the net just extends into dead strip.
FENCE_NORTH_COLS = [4, 8, 9, 13, 24]
FENCE_SOUTH_COLS = [1, 9, 10, 15, 23, 24]

# Every track cut. A cut destroys the hole it names.
#
# The row-30 line under the Pico is structural: it splits all twenty of the
# Pico's columns into a Player 2 half above and a Player 1 half below. Every
# other cut is a per-column fence, and where a fence sits decides how far a
# net can reach - so each one is slid as close to the Pico as its own pads
# allow. Moving column 10's fence from row 44 up to row 34 took 25 mm off the
# wiring on its own. They no longer line up into straight rows; cut them from
# the list, or print cut_template.svg and follow the crosses.
#
# One hard limit on how close a fence may get: leave at least one whole row
# between a Pico pin and the cut on that pin's column. Cutting the hole
# immediately next to a pin leaves no copper to solder to and no room to get
# a probe on the pin, so cuts on columns 3-22 stay two rows clear of rows
# 26 and 33.
CUTS = (
    [(c, PICO_CUT_ROW) for c in range(PICO_COL0, PICO_COL0 + 20)]
    + [(8, 5), (17, 5)]
    + [(4, 12), (8, 12), (9, 12), (13, 12), (24, 7)]
    + [(23, 18), (24, 19)]
    + [(24, 22)]
    + [(23, 24)]
    + [(10, 39), (15, 39)]
    + [(1, 44), (23, 28), (24, 37)]
    + [(9, 48)]    # BL: split the socket+GP5 strip from SW4's ground and the bus
    + [(12, 47), (13, 47)]
    + [(16, 42), (16, 48)]   # GND: GP10 above, socket in the middle, bus below
    + [(22, 28)]   # keep GP16 off the sense tap
    + [(8, 52), (17, 52)]
)

# Insulated wires on the component side, soldered only at the listed holes.
# Two rules beyond "make it short": a wire may not cross under a switch body,
# where it would be unreachable and would stop the switch seating flat, and a
# signal wire may not run alongside a ground bus. Crossing a bus at an angle is
# fine; running parallel and touching is not. Both buses are single straight
# runs along the outer rows, which are the only rows in each band with no
# switch bodies over them.
#
# Wires may pass under a switch body: threading one between the legs is easy
# and the switch still seats flat, so that is not a constraint. Keeping wires
# off the ground buses is a preference the router tries first and drops for a
# given wire if that is what makes it fit. What IS fixed is cluster position:
# neither cluster can move a row without its caps fouling the display (0.56 mm
# of clearance as it stands) or the adhesive standoffs (0.74 mm).
# Endpoints are the CLOSEST free hole pair on the two strip segments each wire
# joins - a jumper only has to reach the segment, not any particular hole on
# it, so picking the nearest pair halves the total wire length. Holes under
# the Pico body are excluded so every joint stays reachable for rework.
JUMPERS = [
    ("P1_INFO to GP13",         [(13, 48), (19, 48)]),
    ("P1_RIGHT to GP2",         [(12, 48), (6, 48)]),
    ("P1_DOWN to GP3",          [(8, 54), (7, 54)]),
    ("P1_B to GP12",            [(17, 54), (18, 54)]),
    ("P2_A to GP27",            [(1, 3), (11, 3)]),
    ("P2_B to GP22",            [(8, 3), (14, 3)]),
    ("P2_MENU to GP19",         [(8, 7), (18, 7)]),
    ("P2_DOWN to GP18",         [(17, 3), (19, 3)]),
    ("display VCC from 3V3",    [(15, 41), (7, 33)]),
    ("display CS to GP10",      [(10, 41), (16, 41)]),
    ("display GND",             [(16, 47), (16, 49)]),
    ("D1 cathode to VSYS",      [(23, 25), (4, 25)]),
    ("charger OUT- to GND",     [(24, 20), (20, 20)]),
    ("battery sense to GP28", [(22, 12), (9, 13)]),
    ("P1 ground bus",           
     [(1, 56), (5, 56), (9, 56), (16, 56), (20, 56), (24, 56)]),
    ("P2 ground bus",           
     [(4, 1), (5, 1), (9, 1), (13, 1), (20, 1), (24, 1)]),
]

# ------------------------------------------------------------- placed parts
#
# (reference, footprint, base column, base row, rotation, note)
# The power train lives on columns 23-24, the two strips the Pico does not
# reach: battery in at the top, charger out, slide switch, then D1 into the
# VSYS carrier (column 23 below its cut) and the sense divider on column 22.

def switch_placements():
    out = []
    for cross, (a, b) in CROSSES.items():
        for compass, (col, row) in cross_switches(a, b).items():
            net = CROSS_LABELS[cross][compass]
            out.append((SWITCH_REFS[net], "mb_stripboard:SW_Tactile_6mm_Stripboard",
                        col, row, 0, f"{cross} {compass} = {net}"))
    return out


PLACEMENTS = switch_placements() + [
    ("U1", "Module:RaspberryPi_Pico_Common_THT", PICO_COL0, PICO_ROW_LOWER, 90,
     "USB left; pin 1 at C37, pin 40 at C30; under the display"),
    ("J1", "Connector_PinHeader_2.54mm:PinHeader_1x08_P2.54mm_Vertical",
     DISPLAY_HEADER_COL0, DISPLAY_HEADER_ROW, 90,
     "socket for the ST7796 module's header"),
    ("BT1", "Connector_PinHeader_2.54mm:PinHeader_1x02_P2.54mm_Vertical",
     23, 8, 90, "LiPo cell leads, brought round the edge from the back"),
    ("J2", "Connector_PinHeader_2.54mm:PinHeader_1x02_P2.54mm_Vertical",
     23, 10, 90, "TP4056 B+ / B- (cell side of the charger)"),
    ("J3", "Connector_PinHeader_2.54mm:PinHeader_1x02_P2.54mm_Vertical",
     23, 21, 90, "TP4056 OUT+ / OUT- (protected output)"),
    ("SW17", "Connector_PinHeader_2.54mm:PinHeader_1x02_P2.54mm_Vertical",
     23, 23, 90, "power slide switch, panel mounted"),
    ("D1", "Diode_THT:D_DO-41_SOD81_P2.54mm_Vertical_AnodeUp", 23, 27, 0,
     "cathode column 23 (VSYS carrier), anode column 24 (switched battery)"),
    ("R1", "Resistor_THT:R_Axial_DIN0207_L6.3mm_D2.5mm_P5.08mm_Vertical",
     24, 26, 180, "battery divider top, X -> V: switched battery to the tap"),
    ("R2", "Resistor_THT:R_Axial_DIN0207_L6.3mm_D2.5mm_P5.08mm_Vertical",
     22, 24, 180, "battery divider bottom, V -> T: tap to ground"),
    ("C1", "Capacitor_THT:C_Disc_D5.0mm_W2.5mm_P5.00mm", 22, 22, 180,
     "ADC filter across R2, V -> T"),
]


def expected_pads():
    """Where each footprint's pads must land, for build_board.py."""
    want = {}
    for cross, (a, b) in CROSSES.items():
        for compass, (col, row) in cross_switches(a, b).items():
            ref = SWITCH_REFS[CROSS_LABELS[cross][compass]]
            want[ref] = {"1": [(col, row), (col, row + 2)],
                         "2": [(col + 3, row), (col + 3, row + 2)]}
    want["U1"] = {"1": [(PICO_COL0, PICO_ROW_LOWER)],
                  "20": [(PICO_COL0 + 19, PICO_ROW_LOWER)],
                  "40": [(PICO_COL0, PICO_ROW_UPPER)],
                  "21": [(PICO_COL0 + 19, PICO_ROW_UPPER)]}
    want["J1"] = {str(i + 1): [(DISPLAY_HEADER_COL0 + i, DISPLAY_HEADER_ROW)]
                  for i in range(8)}
    want["BT1"] = {"1": [(23, 8)], "2": [(24, 8)]}
    want["J2"] = {"1": [(23, 10)], "2": [(24, 10)]}
    want["J3"] = {"1": [(23, 21)], "2": [(24, 21)]}
    want["SW17"] = {"1": [(23, 23)], "2": [(24, 23)]}
    want["D1"] = {"1": [(23, 27)], "2": [(24, 27)]}
    want["R1"] = {"1": [(24, 26)], "2": [(22, 26)]}
    want["R2"] = {"1": [(22, 24)], "2": [(20, 24)]}
    want["C1"] = {"1": [(22, 22)], "2": [(20, 22)]}
    return want


# --------------------------------------------------------- module mechanics

DISPLAY_MM = (55.5, 98.0)         # ST7796 module outline
DISPLAY_ACTIVE_MM = (48.96, 73.44)
BATTERY_MM = (50.0, 34.0)         # 103450 LiPo, mounted on the BACK

# The TP4056 rides on the FRONT, under the display deck, with its USB-C
# overhanging the right edge so a cable can reach it. Cell on the back,
# charger on the front: the two are wired through the board rather than
# stacked on the same face. All four connections leave the edge opposite the
# USB-C, reading OUT- B- B+ OUT+ across it, with IN+/IN- up by the connector
# left unused. It is about 4.8 mm tall including the connector,
# so it clears the 8.5 mm deck. J3 and SW17 sit one row below its footprint
# on purpose: that is what keeps the charger's ground wire out from under the
# module, where it could not be reached or reworked.
TP4056_MM = (26.0, 17.0)
TP4056_COL0, TP4056_ROW0 = 15, 13   # top-left hole its outline hangs from

# The display's far end is held by four self-adhesive nylon standoffs, chosen
# over screws because the module's own corner holes sit about 50 x 92 mm apart
# while the 0.1 in grid can only land on multiples of 2.54 - they would not
# line up. Adhesive also means nothing is drilled: the pads bond to the bare
# component side, and the copper on the reverse is untouched, which is what
# lets two of them sit over live power strips. Positions are checked by
# make_layered.py's geometry pass: the base must sit wholly on the board,
# wholly under the module, and clear of every cap and part body.
STANDOFF_HOLES = [(3, 9), (22, 9), (3, 47), (22, 47)]
STANDOFF_BASE_MM = 10.0           # self-adhesive base diameter
STANDOFF_HEIGHT_MM = 8.0          # plus a ~0.5-1 mm pad; see DECK_HEIGHT_MM
DECK_HEIGHT_MM = 8.5              # module underside above the board, set by J1
CAP_DIA_MM = 9.0                  # 11.3 mm caps collide at the compact pitch

# Part bodies are drawn translucent so the rails they cover stay countable.
# Fly wires are NOT: they are the thing you trace, so they stay opaque.
PART_OPACITY = 0.5


def display_outline():
    """Board-relative rectangle of the display module."""
    hx, hy = hole(DISPLAY_HEADER_COL0 + 3.5, DISPLAY_HEADER_ROW)
    w, h = DISPLAY_MM
    return (hx - w / 2, hy - h + 4.0, w, h)


def charger_outline():
    """Board-relative rectangle of the TP4056, right-aligned to the board edge
    so the USB-C sticks out where a plug can reach it."""
    w, h = TP4056_MM
    y0 = hole(TP4056_COL0, TP4056_ROW0)[1] - 1.0
    return (BOARD_W - w + 1.0, y0, w, h)


def battery_outline():
    """Board-relative rectangle of the cell, which sits on the copper side."""
    dx, dy, dw, dh = display_outline()
    bw, bh = BATTERY_MM
    return (dx + (dw - bw) / 2, dy + (dh - bh) / 2, bw, bh)



# Physical body outlines, so the preview shows what actually occupies the
# board rather than just where the pads are. Values are the real part bodies;
# each is drawn from its pad bounding box grown by these margins (mm).
PART_BODY = {
    # ref prefix: (grow_x, grow_y, label)
    "U1":   (1.37, 1.61, "Pico 2"),      # 51 x 21 mm body over a 48.26 x 17.78 pad field
    "J1":   (1.27, 1.27, "J1 display socket"),
    "BT1":  (1.27, 1.27, "BT1"),
    "J2":   (1.27, 1.27, "J2"),
    "J3":   (1.27, 1.27, "J3"),
    "SW17": (1.27, 1.27, "SW17"),
    "D1":   (1.0, 2.0, "D1"),
    "R1":   (1.0, 1.6, "R1"),
    "R2":   (1.0, 1.6, "R2"),
    "C1":   (1.4, 2.5, "C1"),
}

SWITCH_BODY_MM = 6.0      # the tactile switch's own body, inside the cap


def part_pad_holes(ref):
    """Every (col, row) a part's pads occupy, so the preview can mark them.

    expected_pads() only pins down the corners of the big parts; the Pico's
    forty pins are generated here so the drawing shows which rail each one
    lands on.
    """
    if ref == "U1":
        return ([(PICO_COL0 + i, PICO_ROW_LOWER) for i in range(20)]
                + [(PICO_COL0 + i, PICO_ROW_UPPER) for i in range(20)])
    pads = expected_pads().get(ref)
    if not pads:
        return []
    return [h for v in pads.values() for h in v]


def part_body(ref):
    """Board-relative (x, y, w, h) of a placed part's physical body."""
    pads = expected_pads().get(ref)
    if not pads or ref not in PART_BODY:
        return None
    holes = [h for v in pads.values() for h in v]
    xs = [hole(c, r)[0] for c, r in holes]
    ys = [hole(c, r)[1] for c, r in holes]
    gx, gy, _ = PART_BODY[ref]
    return (min(xs) - gx, min(ys) - gy,
            (max(xs) - min(xs)) + 2 * gx, (max(ys) - min(ys)) + 2 * gy)


# ------------------------------------------------------------------- the BOM

BOM = [
    # qty, part, value/spec, refs, note, est USD, Amazon search string
    (1, "Stripboard 24x56", "24 strips x 56 holes, 0.1 in pitch, ~61 x 142 mm",
     "-", "already on hand; strips run the long way",
     0, "-- on hand"),
    (1, "Raspberry Pi Pico 2", "RP2350, 4 MB flash", "U1",
     "with 2x20 pin header, soldered flat, USB facing the board's left edge",
     12, "Raspberry Pi Pico 2 RP2350 with pre-soldered headers"),
    (1, "3.5 in TFT display module", "ST7796, 320x480, SPI, 8 pin, 3.3 V", "J1",
     "55.5 x 98 mm; the XIITIA 2-pack is one board plus a spare; already ordered",
     17, "XIITIA 3.5 inch SPI TFT LCD ST7796 320x480 8pin module"),
    (1, "Female header socket", "1x8, 2.54 mm", "J1",
     "the display's male header plugs into this; sets the deck height",
     7, "2.54mm female pin header socket 1x8 single row"),
    (16, "Tactile switch", "6 x 6 mm through hole, 4 leg",
     "SW1-SW16", "legs splay onto the 0.2 x 0.3 in hole pattern",
     8, "6x6x5mm tactile push button switch 4 pin through hole 100 pack"),
    (16, "Button cap", "9 mm dia, 3D printed - NEW PRINT, 11.3 mm caps do not fit",
     "-", "regenerate hardware/button_cap/ with d=9.0, bore for 6 mm plunger",
     0, "-- printed, filament only"),
    (1, "USB-C LiPo charger", "TP4056 + DW01 + FS8205, 1 A", "J2 J3",
     "protected variant only (OUT+/OUT- pads). Mounts on the FRONT under the "
     "display deck, USB-C overhanging the right edge; the cell is on the back",
     9, "TP4056 USB C lithium battery charging module with protection DW01"),
    (1, "LiPo cell", "3.7 V 2000 mAh, 103450, 50 x 34 x 10 mm", "BT1",
     "mounts on the BACK on a foam pad over the solder joints",
     13, "103450 3.7V 2000mAh lipo battery JST PH 2.0 connector"),
    (1, "Schottky diode", "1N5817, DO-41", "D1",
     "battery to VSYS; lets both USB ports be plugged in at once",
     7, "1N5817 Schottky diode DO-41 through hole"),
    (1, "Slide switch", "SPDT, panel mount", "SW17", "power",
     7, "SPDT slide switch 3 pin panel mount small"),
    (2, "Resistor", "100k, 1/4 W, stood vertical", "R1 R2", "battery sense divider",
     10, "1/4W metal film resistor assortment kit 1% 100k"),
    (1, "Ceramic capacitor", "100 nF, 5 mm disc", "C1", "ADC filter",
     10, "ceramic capacitor assortment kit 0.1uF 104 through hole"),
    (4, "Self-adhesive nylon standoff", "8 mm tall, ~10 mm adhesive base", "-",
     "holds the display's far end; bonds to the bare component side, nothing "
     "drilled. Shim to the socket's deck height before sticking down",
     9, "self adhesive nylon standoff spacer PCB 8mm"),
    (1, "Foam tape pad", "~55 x 40 mm, 1-2 mm thick", "-",
     "between the cell and the copper-side solder joints",
     6, "double sided foam mounting tape 1mm"),
    (1, "Wire", "30 AWG insulated wire-wrap, ~1.5 m", "-",
     "15 jumper runs, longest is about 75 mm",
     10, "30 AWG wire wrap wire kit 6 colors solid core"),
]

BOM_NOTE = ("Prices are typical Amazon listings as of August 2026, for the "
            "smallest sensible purchase rather than a per-unit cost. Most of "
            "these parts only sell in packs, so the pack is what you pay. "
            "Estimates, not quotes.")


def bom_total():
    return sum(row[5] for row in BOM)


def bom_total_new():
    """Total minus the display, which is already ordered."""
    return sum(row[5] for row in BOM if not row[1].startswith("3.5 in"))


def bom_csv():
    rows = ["qty,part,value,refs,est_usd,amazon_search,note"]
    for qty, part, value, refs, note, usd, search in BOM:
        cells = [str(qty), part, value, refs, str(usd), search, note]
        rows.append(",".join('"%s"' % c.replace('"', "'") for c in cells))
    rows.append('"","TOTAL","","","%d","",""' % bom_total())
    rows.append('"","TOTAL less the display already ordered","","","%d","",""'
                % bom_total_new())
    rows.append('"","","","","","","%s"' % BOM_NOTE)
    return "\n".join(rows) + "\n"


# ----------------------------------------------------------------- outputs

def strip_segments():
    """Every copper strip, split at its cuts. Yields (col, row_start, row_end)."""
    cuts_by_col = {}
    for col, row in CUTS:
        cuts_by_col.setdefault(col, []).append(row)
    for col in range(1, COLS + 1):
        rows = sorted(cuts_by_col.get(col, []))
        start = 1
        for cut in rows:
            if cut - 1 >= start:
                yield (col, start, cut - 1)
            start = cut + 1
        if start <= ROWS:
            yield (col, start, ROWS)


def kicad_data():
    return {
        "board": {"origin": [ORIGIN_X, ORIGIN_Y], "w": BOARD_W, "h": BOARD_H},
        "placements": [
            {"ref": r, "fp": fp, "col": c, "row": w, "rot": rot,
             "x": page(c, w)[0], "y": page(c, w)[1], "note": note}
            for r, fp, c, w, rot, note in PLACEMENTS
        ],
        "strips": [
            {"col": c, "r0": r0, "r1": r1,
             "x": page(c, r0)[0], "y0": page(c, r0)[1], "y1": page(c, r1)[1]}
            for c, r0, r1 in strip_segments()
        ],
        "cuts": [{"col": c, "row": r, "x": page(c, r)[0], "y": page(c, r)[1]}
                 for c, r in CUTS],
    }


# Strips cycle through these so neighbouring columns stay tellable-apart when
# tracing a net across the preview.
STRIP_COLORS = ["#c9803a", "#8fb4d9", "#7dc98f", "#d98fb4"]



# Fly-wire colour code. Matches a standard 6-colour 30 AWG wire kit so the
# built board reads the same as the preview: red = power, white = ground,
# yellow = display signals, blue = P1 buttons, green = P2 buttons,
# orange = battery sense.
def wire_color(name):
    if name.startswith("P1 ground") or name.startswith("P2 ground") \
            or name == "charger OUT- to GND" or name == "display GND":
        return ("#f2f2f2", "white")
    if name == "display VCC from 3V3" or name == "D1 cathode to VSYS":
        return ("#ff5555", "red")
    if name.startswith("display"):
        return ("#ffd24d", "yellow")
    if name.startswith("P1_"):
        return ("#4da6ff", "blue")
    if name.startswith("P2_"):
        return ("#5fe08a", "green")
    return ("#ff9e4d", "orange")     # battery sense


# How the layers stack, bottom to top, and which ones live in a folder.
# Grouped by MODULE, not by type: each functional block owns its parts, its
# fly wires and its mechanical outline, so you can isolate "the display" or
# "P1's controls" in one click.
#
# Draw order IS stack order, so the module order is also the painter's
# algorithm. The Pico goes down first because its translucent body is the one
# that overlaps everything else; every later module's wires then draw on top
# of it rather than under it. Legend and cuts sit at the bottom of the panel,
# just above the substrate, where the 50%-opacity bodies still let them read.
LAYER_TREE = [
    ("substrate",       ["background", "copper strips", "holes"]),
    (None,              ["legend"]),
    (None,              ["cuts"]),
    ("Pico and ground", ["U1 Pico 2", "Pico pin labels", "wires GND"]),
    ("display",         ["J1 display socket", "wires display",
                         "display standoffs", "display outline"]),
    ("power",           ["BT1 battery leads", "J2 charger B+ B-",
                         "J3 charger OUT", "SW17 power switch", "D1 Schottky",
                         "sense divider R1 R2 C1", "charger module", "wires power",
                         "wires sense", "battery on back"]),
    ("P1 controls",     ["buttons p1 dpad", "buttons p1 actions", "wires P1"]),
    ("P2 controls",     ["buttons p2 dpad", "buttons p2 actions", "wires P2"]),
    # a reading aid, so it goes on top of everything it is meant to index
    (None,              ["grid and labels"]),
]


def svg_layer_tree(scale=5.0):
    """Return ([(group_or_None, [(name, elements), ...]), ...], W, H).

    Same content as svg_layers(), but keeping the folder structure so the
    layered exports can nest it.
    """
    flat, W, H = svg_layers(scale=scale)
    by_name = dict(flat)
    tree = []
    for group, names in LAYER_TREE:
        members = [(n, by_name[n]) for n in names if by_name.get(n)]
        if members:
            tree.append((group, members))
    seen = {n for _, names in LAYER_TREE for n in names}
    missing = [n for n, _ in flat if n not in seen]
    if missing:
        raise SystemExit(f"layers missing from LAYER_TREE: {missing}")
    return tree, W, H


def svg_layers(scale=5.0):
    """Return [(layer_name, svg_body_elements)] in draw order.

    One source of truth for the flat preview and the layered exports: the flat
    SVG is every layer concatenated, and svg_layer_tree() groups the same list.
    """
    LEGEND_H = 40
    GUTTER = 26          # room outside the board for the row/column rulers
    W = BOARD_W * scale + 2 * GUTTER
    H = BOARD_H * scale + 2 * GUTTER + LEGEND_H

    def sx(v):
        """board-relative mm -> canvas px, horizontally"""
        return GUTTER + v * scale

    def sy(v):
        return GUTTER + v * scale

    def px(col, row):
        x, y = hole(col, row)
        return sx(x), sy(y)

    layers = []

    # --- substrate ---
    layers.append(("background", [
        f'<rect width="{W:.0f}" height="{H:.0f}" fill="#0d1117"/>']))

    # --- row and column rulers ---
    # Every coordinate in BUILD.md is a (column, row); without a ruler you are
    # counting holes with a fingertip. Numbers sit outside the board on all
    # four sides so an edge is never what you count from, and a faint line
    # every fifth hole gives the eye somewhere to land.
    grid = []
    for col in range(1, COLS + 1):
        x, _ = px(col, 1)
        heavy = col % 5 == 0 or col in (1, COLS)   # every fifth letter is bold
        if heavy:
            grid.append(f'<line x1="{x:.1f}" y1="{sy(0):.1f}" x2="{x:.1f}" '
                        f'y2="{sy(BOARD_H):.1f}" stroke="#58a6ff" '
                        f'stroke-width="0.7" opacity="0.30"/>')
            for ty in (GUTTER - 9, sy(BOARD_H) + 15):
                grid.append(f'<text x="{x:.1f}" y="{ty:.1f}" fill="#9fd0ff" '
                            f'font-size="10" font-family="monospace" '
                            f'text-anchor="middle">{col_name(col)}</text>')
        else:
            for ty in (GUTTER - 10, sy(BOARD_H) + 14):
                grid.append(f'<text x="{x:.1f}" y="{ty:.1f}" fill="#8ab4d8" '
                            f'font-size="8" font-family="monospace" '
                            f'text-anchor="middle">{col_name(col)}</text>')
    for row in range(1, ROWS + 1):
        _, y = px(1, row)
        heavy = row % 5 == 0 or row in (1, ROWS)
        if heavy:
            grid.append(f'<line x1="{sx(0):.1f}" y1="{y:.1f}" '
                        f'x2="{sx(BOARD_W):.1f}" y2="{y:.1f}" stroke="#58a6ff" '
                        f'stroke-width="0.7" opacity="0.30"/>')
            grid.append(f'<text x="{GUTTER - 6:.1f}" y="{y + 3:.1f}" fill="#9fd0ff" '
                        f'font-size="9" font-family="monospace" '
                        f'text-anchor="end">{row}</text>')
            grid.append(f'<text x="{sx(BOARD_W) + 6:.1f}" y="{y + 3:.1f}" '
                        f'fill="#9fd0ff" font-size="9" font-family="monospace">{row}</text>')
        else:
            grid.append(f'<text x="{GUTTER - 6:.1f}" y="{y + 2.5:.1f}" fill="#5a7fa6" '
                        f'font-size="6.5" font-family="monospace" '
                        f'text-anchor="end">{row}</text>')
            grid.append(f'<text x="{sx(BOARD_W) + 6:.1f}" y="{y + 2.5:.1f}" '
                        f'fill="#5a7fa6" font-size="6.5" font-family="monospace">{row}</text>')
    grid.append(f'<rect x="{sx(0):.1f}" y="{sy(0):.1f}" '
                f'width="{BOARD_W*scale:.1f}" height="{BOARD_H*scale:.1f}" '
                f'fill="none" stroke="#8ab4d8" stroke-width="1.2"/>')

    # --- copper strips, colour-cycled by column ---
    strips = []
    for col, r0, r1 in strip_segments():
        x, y0 = px(col, r0)
        _, y1 = px(col, r1)
        color = STRIP_COLORS[(col - 1) % len(STRIP_COLORS)]
        strips.append(f'<line x1="{x:.1f}" y1="{y0:.1f}" x2="{x:.1f}" y2="{y1:.1f}" '
                      f'stroke="{color}" stroke-width="{1.9 * scale:.1f}" '
                      f'stroke-linecap="round" opacity="0.8"/>')
    layers.append(("grid and labels", grid))
    layers.append(("copper strips", strips))

    # --- holes ---
    holes = []
    for col in range(1, COLS + 1):
        for row in range(1, ROWS + 1):
            x, y = px(col, row)
            holes.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" '
                         f'r="{0.5 * scale:.1f}" fill="#0d1117"/>')
    layers.append(("holes", holes))

    # --- button clusters, one layer each ---
    ref_cross = {}
    for cross, (a, b) in CROSSES.items():
        for compass in ("N", "W", "E", "S"):
            ref_cross[SWITCH_REFS[CROSS_LABELS[cross][compass]]] = cross
    cluster = {}
    for ref, fp, col, row, rot, note in PLACEMENTS:
        if not fp.startswith("mb_stripboard"):
            continue
        cx, cy = px(col + 1.5, row + 1)
        el = cluster.setdefault(ref_cross[ref], [])
        # outline the pad span, not an abstract 6 mm square: the switch's
        # terminals sit on rails `col` and `col+3`, four columns apart
        x0, y0 = px(col, row)
        x1, y1 = px(col + 3, row + 2)
        m = 0.9 * scale
        el.append(f'<rect x="{x0 - m:.1f}" y="{y0 - m:.1f}" '
                  f'width="{x1 - x0 + 2*m:.1f}" height="{y1 - y0 + 2*m:.1f}" '
                  f'fill="#d29922" fill-opacity="{PART_OPACITY}" stroke="#d29922" '
                  f'stroke-width="1.4" rx="3"/>')
        el.append(f'<circle cx="{cx:.1f}" cy="{cy:.1f}" '
                  f'r="{CAP_DIA_MM / 2 * scale:.1f}" fill="none" '
                  f'stroke="#ffd88a" stroke-width="1.2" stroke-dasharray="4 3"/>')
        for hc, hr in ((col, row), (col, row + 2), (col + 3, row), (col + 3, row + 2)):
            hx, hy = px(hc, hr)
            el.append(f'<circle cx="{hx:.1f}" cy="{hy:.1f}" r="{0.95 * scale:.1f}" '
                      f'fill="#ffd88a" stroke="#5a4a1a" stroke-width="0.8"/>')
        el.append(f'<text x="{cx:.1f}" y="{cy + 4:.1f}" fill="#fff" font-size="11" '
                  f'font-family="monospace" text-anchor="middle" '
                  f'stroke="#000" stroke-width="2.5" paint-order="stroke">{ref}</text>')
    for cross in ("P1_DPAD", "P1_ACTIONS", "P2_DPAD", "P2_ACTIONS"):
        if cross in cluster:
            layers.append((f"buttons {cross.replace('_', ' ').lower()}",
                           cluster[cross]))

    # --- part bodies, one layer per part (passives grouped) ---
    PASSIVES = ("R1", "R2", "C1")
    part_layers = {}
    for ref, fp, col, row, rot, note in PLACEMENTS:
        if fp.startswith("mb_stripboard"):
            continue
        body = part_body(ref)
        if body is None:
            continue
        key = "sense divider R1 R2 C1" if ref in PASSIVES else ref
        parts = part_layers.setdefault(key, [])
        bx, by, bw, bh = body
        bx, by, bw, bh = sx(bx), sy(by), bw * scale, bh * scale
        heavy = ref in ("U1", "J1")
        parts.append(f'<rect x="{bx:.1f}" y="{by:.1f}" width="{bw:.1f}" '
                     f'height="{bh:.1f}" fill="#8b949e" '
                     f'fill-opacity="{PART_OPACITY}" stroke="#e6edf3" '
                     f'stroke-width="{2.0 if heavy else 1.2}" rx="3"/>')
        for hc, hr in part_pad_holes(ref):
            hx, hy = px(hc, hr)
            parts.append(f'<circle cx="{hx:.1f}" cy="{hy:.1f}" '
                         f'r="{0.95 * scale:.1f}" fill="#e6edf3" '
                         f'stroke="#30363d" stroke-width="0.8"/>')
        label = PART_BODY[ref][2]
        if ref == "U1":
            uw, uh = 8.0 * scale, 6.0 * scale
            parts.append(f'<rect x="{bx - uh * 0.45:.1f}" '
                         f'y="{by + bh / 2 - uw / 2:.1f}" width="{uh * 0.45:.1f}" '
                         f'height="{uw:.1f}" fill="#8b949e" stroke="#e6edf3" '
                         f'stroke-width="1"/>')
            p1x, p1y = px(PICO_COL0, PICO_ROW_LOWER)
            parts.append(f'<rect x="{p1x - 4:.1f}" y="{p1y - 4:.1f}" width="8" '
                         f'height="8" fill="none" stroke="#ff5555" stroke-width="1.5"/>')
            parts.append(f'<text x="{bx + bw / 2:.1f}" y="{by + bh / 2 + 4:.1f}" '
                         f'fill="#fff" font-size="13" font-family="monospace" '
                         f'text-anchor="middle" stroke="#000" stroke-width="3" '
                         f'paint-order="stroke">{label}</text>')
            parts.append(f'<text x="{bx + bw / 2:.1f}" y="{by + bh / 2 + 18:.1f}" '
                         f'fill="#c9d1d9" font-size="10" font-family="monospace" '
                         f'text-anchor="middle" stroke="#000" stroke-width="2.5" '
                         f'paint-order="stroke">USB left, pin 1 boxed red</text>')
        elif ref == "J1":
            parts.append(f'<text x="{bx + bw / 2:.1f}" y="{by - 4:.1f}" '
                         f'fill="#fff" font-size="11" font-family="monospace" '
                         f'text-anchor="middle" stroke="#000" stroke-width="2.5" '
                         f'paint-order="stroke">{label}</text>')
        else:
            # parts near the right edge get their label on the inboard side,
            # otherwise it runs off the canvas
            flip = bx + bw + 4 > sx(BOARD_W) - 30
            lx = (bx - 4) if flip else (bx + bw + 4)
            parts.append(f'<text x="{lx:.1f}" y="{by + bh / 2 + 4:.1f}" '
                         f'fill="#fff" font-size="11" font-family="monospace" '
                         f'text-anchor="{"end" if flip else "start"}" '
                         f'stroke="#000" stroke-width="2.5" paint-order="stroke">{ref}</text>')
    part_names = {
        "U1": "U1 Pico 2", "J1": "J1 display socket", "BT1": "BT1 battery leads",
        "J2": "J2 charger B+ B-", "J3": "J3 charger OUT", "SW17": "SW17 power switch",
        "D1": "D1 Schottky", "sense divider R1 R2 C1": "sense divider R1 R2 C1",
    }
    for key in ("U1", "J1", "D1", "sense divider R1 R2 C1", "BT1", "J2", "J3", "SW17"):
        if key in part_layers:
            layers.append((part_names[key], part_layers[key]))

    # --- Pico pin numbers and names, off by default ---
    # Each pad gets its number written on it, then the pin's function and the
    # net it carries running outward from the row, rotated so neighbouring
    # columns stay parallel instead of colliding.
    pins = []
    net_of = {}
    for net, col, rows, pin, how in BUTTON_NETS:
        net_of[pin] = net
    for jpin, name, col, rows, pin, how in DISPLAY_NETS:
        net_of[pin] = name
    net_of.update({39: "VSYS", 36: "3V3", 34: "VBAT_SENSE"})

    FN_OFF, NET_OFF = 8.0, 52.0     # px from the pad, outward
    for i in range(20):
        for pin, row, up in ((i + 1, PICO_ROW_LOWER, False),
                             (40 - i, PICO_ROW_UPPER, True)):
            col = PICO_COL0 + i
            x, y = px(col, row)
            fn = PICO_PIN_NAME[pin]
            used = net_of.get(pin)
            colr = "#8b949e" if fn == "GND" else ("#7ee787" if used else "#f0883e")
            sign = -1 if up else 1
            rot = -90 if up else 90

            pins.append(f'<text x="{x:.1f}" y="{y + 2.6:.1f}" fill="#fff" '
                        f'font-size="7.5" font-family="monospace" '
                        f'text-anchor="middle" stroke="#000" stroke-width="2.2" '
                        f'paint-order="stroke">{pin}</text>')

            fy = y + sign * FN_OFF
            pins.append(f'<text x="{x:.1f}" y="{fy:.1f}" fill="{colr}" '
                        f'font-size="7.5" font-family="monospace" '
                        f'text-anchor="start" stroke="#000" stroke-width="2.2" '
                        f'paint-order="stroke" '
                        f'transform="rotate({rot} {x:.1f} {fy:.1f})">{fn}</text>')

            if used:
                ny = y + sign * NET_OFF
                pins.append(f'<text x="{x:.1f}" y="{ny:.1f}" fill="#c9d1d9" '
                            f'font-size="7" font-family="monospace" '
                            f'text-anchor="start" stroke="#000" stroke-width="2.2" '
                            f'paint-order="stroke" '
                            f'transform="rotate({rot} {x:.1f} {ny:.1f})">{used}</text>')
    layers.append(("Pico pin labels", pins))

    # --- fly wires, one layer per colour group ---
    groups = {}
    for name, pts in JUMPERS:
        color, cname = wire_color(name)
        groups.setdefault(cname, (color, []))[1].append((name, pts))
    order = ["blue", "green", "yellow", "red", "white", "orange"]
    pretty = {"blue": "wires P1", "green": "wires P2", "yellow": "wires display",
              "red": "wires power", "white": "wires GND", "orange": "wires sense"}
    for cname in order:
        if cname not in groups:
            continue
        color, items = groups[cname]
        el = []
        for wi, (name, pts) in enumerate(items):
            pairs = [px(c, r) for c, r in pts]
            if len(pairs) == 2:
                # Straight wires that share a row land on top of each other and
                # you cannot tell which end pairs with which. Bow each one by a
                # different amount and label it, so the run is followable.
                (x0, y0), (x1, y1) = pairs
                mx, my = (x0 + x1) / 2, (y0 + y1) / 2
                dx, dy = x1 - x0, y1 - y0
                ln = max(1e-6, (dx * dx + dy * dy) ** 0.5)
                nx, ny = -dy / ln, dx / ln          # unit normal
                bow = 9 + 7 * (wi % 3)
                if wi % 2:
                    bow = -bow
                cxp, cyp = mx + nx * bow, my + ny * bow
                el.append(f'<path d="M{x0:.1f},{y0:.1f} Q{cxp:.1f},{cyp:.1f} '
                          f'{x1:.1f},{y1:.1f}" stroke="{color}" stroke-width="2.4" '
                          f'fill="none"/>')
                lx, ly = mx + nx * bow * 0.55, my + ny * bow * 0.55
                el.append(f'<text x="{lx:.1f}" y="{ly + 3:.1f}" fill="{color}" '
                          f'font-size="8" font-family="monospace" '
                          f'text-anchor="middle" stroke="#0d1117" stroke-width="2.4" '
                          f'paint-order="stroke">{name.split(" to ")[0]}</text>')
            else:
                d = " ".join(("M" if i == 0 else "L") + f"{x:.1f},{y:.1f}"
                             for i, (x, y) in enumerate(pairs))
                el.append(f'<path d="{d}" stroke="{color}" stroke-width="2.4" '
                          f'fill="none"/>')
                lx, ly = pairs[len(pairs) // 2]
                el.append(f'<text x="{lx:.1f}" y="{ly - 6:.1f}" fill="{color}" '
                          f'font-size="8" font-family="monospace" '
                          f'text-anchor="middle" stroke="#0d1117" stroke-width="2.4" '
                          f'paint-order="stroke">{name}</text>')
            for x, y in pairs:
                el.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="3.2" fill="{color}" '
                          f'stroke="#0d1117" stroke-width="1"/>')
        layers.append((pretty[cname], el))

    # --- cuts, above the bodies so they stay visible ---
    cuts = []
    for col, row in CUTS:
        x, y = px(col, row)
        d = 1.1 * scale
        cuts.append(f'<path d="M{x-d:.1f},{y-d:.1f} L{x+d:.1f},{y+d:.1f} '
                    f'M{x-d:.1f},{y+d:.1f} L{x+d:.1f},{y-d:.1f}" '
                    f'stroke="#ff4d4d" stroke-width="{0.5 * scale:.1f}"/>')
    layers.append(("cuts", cuts))

    # --- mechanical overlays ---
    dx, dy, dw, dh = display_outline()
    disp = [f'<rect x="{sx(dx):.1f}" y="{sy(dy):.1f}" width="{dw*scale:.1f}" '
            f'height="{dh*scale:.1f}" fill="none" stroke="#58a6ff" '
            f'stroke-width="2.5" stroke-dasharray="8 5"/>',
            f'<text x="{sx(dx+2):.1f}" y="{sy(dy+5):.1f}" fill="#58a6ff" '
            f'font-size="13" font-family="monospace">ST7796 on 10mm standoffs</text>']
    layers.append(("display outline", disp))

    so = []
    for c, r in STANDOFF_HOLES:
        x, y = px(c, r)
        so.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" '
                  f'r="{STANDOFF_BASE_MM / 2 * scale:.1f}" fill="#a371f7" '
                  f'fill-opacity="{PART_OPACITY}" stroke="#d2a8ff" '
                  f'stroke-width="1.4"/>')
        so.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{1.25 * scale:.1f}" '
                  f'fill="none" stroke="#d2a8ff" stroke-width="1"/>')
    layers.append(("display standoffs", so))

    cx0, cy0, cw, ch = charger_outline()
    chg = [f'<rect x="{sx(cx0):.1f}" y="{sy(cy0):.1f}" width="{cw*scale:.1f}" '
           f'height="{ch*scale:.1f}" fill="#f0883e" fill-opacity="{PART_OPACITY}" '
           f'stroke="#ffa657" stroke-width="2"/>',
           f'<rect x="{sx(cx0+cw-1.5):.1f}" y="{sy(cy0+ch/2-4.5):.1f}" '
           f'width="{3.0*scale:.1f}" height="{9.0*scale:.1f}" fill="#ffa657" '
           f'stroke="#0d1117" stroke-width="1"/>',
           f'<text x="{sx(cx0+2):.1f}" y="{sy(cy0+5):.1f}" fill="#fff" '
           f'font-size="11" font-family="monospace" stroke="#000" '
           f'stroke-width="2.5" paint-order="stroke">TP4056</text>',
           f'<text x="{sx(cx0+2):.1f}" y="{sy(cy0+9.5):.1f}" fill="#ffd8a8" '
           f'font-size="9" font-family="monospace" stroke="#000" '
           f'stroke-width="2" paint-order="stroke">USB-C over the edge</text>']
    # The module is a module: it reaches the board through four flying leads,
    # not through pads of its own. Draw them, or the outline reads as an
    # ornament that happens to sit near some headers. Each lead gets its own
    # exit point along the module edge so neither the lines nor their labels
    # land on top of one another.
    # Pad order is taken from the module itself: USB-C on one short edge and
    # all four connections on the opposite one, reading OUT- B- B+ OUT+ across
    # it. Mounted with the connector over the right board edge, that edge faces
    # left, so the leads fan from a single edge rather than from two.
    # Rotating the module the other way in-plane reverses this order; the two
    # OUT leads simply swap.
    LEADS = [
        # label, board hole, distance down the module's left edge
        ("OUT-", (24, 21), 3.0),
        ("B-",   (24, 10), 7.0),
        ("B+",   (23, 10), 11.0),
        ("OUT+", (23, 21), 15.0),
    ]
    for label, hole_cr, oy in LEADS:
        hx, hy = px(*hole_cr)
        ex, ey = sx(cx0), sy(cy0 + oy)
        chg.append(f'<path d="M{ex:.1f},{ey:.1f} C{(ex + hx) / 2:.1f},{ey:.1f} '
                   f'{(ex + hx) / 2:.1f},{hy:.1f} {hx:.1f},{hy:.1f}" '
                   f'stroke="#ffa657" stroke-width="1.8" fill="none" '
                   f'stroke-dasharray="5 3"/>')
        chg.append(f'<circle cx="{ex:.1f}" cy="{ey:.1f}" r="2.6" fill="#ffa657" '
                   f'stroke="#0d1117" stroke-width="0.8"/>')
        chg.append(f'<text x="{ex + 6:.1f}" y="{ey + 3:.1f}" fill="#ffd8a8" '
                   f'font-size="8.5" font-family="monospace" text-anchor="start" '
                   f'stroke="#0d1117" stroke-width="2.4" '
                   f'paint-order="stroke">{label}</text>')

    layers.append(("charger module", chg))

    bx, by, bw, bh = battery_outline()
    batt = [f'<rect x="{sx(bx):.1f}" y="{sy(by):.1f}" width="{bw*scale:.1f}" '
            f'height="{bh*scale:.1f}" fill="none" stroke="#3fb950" '
            f'stroke-width="2.5" stroke-dasharray="5 5"/>',
            f'<text x="{sx(bx+2):.1f}" y="{sy(by+5):.1f}" fill="#3fb950" '
            f'font-size="13" font-family="monospace">LiPo on BACK</text>']
    layers.append(("battery on back", batt))

    # --- legend ---
    leg = []
    entries = [("#4da6ff", "P1"), ("#5fe08a", "P2"), ("#ffd24d", "display"),
               ("#ff5555", "power"), ("#f2f2f2", "GND"), ("#ff9e4d", "sense")]
    for i, (color, label) in enumerate(entries):
        ly = sy(BOARD_H) + GUTTER + 14 + (i // 3) * 16
        lx = GUTTER + 8 + (i % 3) * 100
        leg.append(f'<line x1="{lx}" y1="{ly}" x2="{lx + 16}" y2="{ly}" '
                   f'stroke="{color}" stroke-width="3"/>')
        leg.append(f'<text x="{lx + 20}" y="{ly + 4}" fill="#c9d1d9" '
                   f'font-size="11" font-family="monospace">{label}</text>')
    layers.append(("legend", leg))

    return layers, W, H


def svg():
    tree, W, H = svg_layer_tree()
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W:.0f}" '
           f'height="{H:.0f}" viewBox="0 0 {W:.0f} {H:.0f}">']
    for _, members in tree:
        for _, elements in members:
            out.extend(elements)
    out.append("</svg>")
    return "\n".join(out)


def stage_svg(names, scale=5.0):
    """One SVG containing only the named layers, in their usual stacking order.

    Used for the build guides: a picture of the board showing just what the
    stage is about, drawn from the same layer definitions as everything else
    so it cannot show something the board does not have.
    """
    tree, W, H = svg_layer_tree(scale=scale)
    want = set(names)
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W:.0f}" '
           f'height="{H:.0f}" viewBox="0 0 {W:.0f} {H:.0f}">']
    for _, members in tree:
        for name, elements in members:
            if name in want:
                out.extend(elements)
    missing = want - {n for _, m in tree for n, _ in m}
    if missing:
        raise SystemExit(f"stage_svg: no such layer {sorted(missing)}")
    out.append("</svg>")
    return "\n".join(out)


# The pictures that go with the staged build, each naming the layers it needs.
STAGE_VIEWS = {
    "cuts_guide": ["background", "copper strips", "holes", "grid and labels",
                   "cuts", "legend"],
}


def layer_svg(name, elements, W, H, transparent=True):
    """One layer as a standalone SVG on a transparent canvas."""
    bg = "" if transparent else f'<rect width="{W:.0f}" height="{H:.0f}" fill="#0d1117"/>'
    return "\n".join(
        [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W:.0f}" '
         f'height="{H:.0f}" viewBox="0 0 {W:.0f} {H:.0f}">', bg]
        + elements + ["</svg>"])


def cut_template_svg():
    """1:1 printable cut map, MIRRORED so it can be taped to the copper side.

    Print at 100% scale, tape to the copper, and cut every X. Column numbers
    are mirrored to match what the copper side sees.
    """
    W, H = BOARD_W, BOARD_H
    FENCE = [c for c, r in CUTS if r == PICO_CUT_ROW]

    def px(col, row):
        x, y = hole(col, row)
        return (BOARD_W - x, y)      # mirror horizontally

    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}mm" height="{H + 21}mm" '
           f'viewBox="0 0 {W} {H + 21}" font-family="monospace">',
           f'<rect width="{W}" height="{H}" fill="none" stroke="#000" stroke-width="0.3"/>']
    for col in range(1, COLS + 1):
        for row in range(1, ROWS + 1):
            x, y = px(col, row)
            out.append(f'<circle cx="{x:.2f}" cy="{y:.2f}" r="0.5" '
                       f'fill="none" stroke="#999" stroke-width="0.15"/>')
    for col, row in CUTS:
        x, y = px(col, row)
        out.append(f'<path d="M{x-1:.2f},{y-1:.2f} L{x+1:.2f},{y+1:.2f} '
                   f'M{x-1:.2f},{y+1:.2f} L{x+1:.2f},{y-1:.2f}" '
                   f'stroke="#000" stroke-width="0.5"/>')
    for col in (1, 5, 10, 15, 20, 24):
        x, _ = px(col, 1)
        out.append(f'<text x="{x:.2f}" y="{H + 3.4}" font-size="2.4" '
                   f'text-anchor="middle" fill="#000">{col}</text>')
    out.append(f'<text x="1" y="{H + 8.4}" font-size="2.4" fill="#000">'
               f'COPPER SIDE - print at 100%</text>')
    out.append(f'<text x="1" y="{H + 11.8}" font-size="2.4" fill="#666">'
               f'{len(CUTS)} cuts; {len(FENCE)} are the line at row '
               f'{PICO_CUT_ROW}</text>')
    # a bar to put a ruler against; printer scaling is the usual failure, and
    # 50 mm is the longest that fits across the board
    bar = 50.0
    x0 = (W - bar) / 2
    out.append(f'<text x="{W / 2:.2f}" y="{H + 15.6}" font-size="2.4" '
               f'text-anchor="middle" fill="#000">'
               f'this bar must measure {bar:.0f} mm</text>')
    out.append(f'<path d="M{x0:.2f},{H + 18.0} h{bar} '
               f'm0,-1.5 v3 m-{bar},-3 v3 m{bar / 2},-2 v2" stroke="#000" '
               f'stroke-width="0.4" fill="none"/>')
    out.append("</svg>")
    return "\n".join(out)


def assembled_scad():
    """A solid model of the finished board, generated from the same geometry.

    The SVG shows what it looks like; this is the thing itself, for checking
    against the enclosure or for anyone who would rather turn it over in a
    viewer. Everything is at its true height, so the display really does sit
    on its standoffs at the deck height the socket sets.
    """
    def rect(name):
        b = part_body(name)
        return f"[{b[0]:.2f}, {b[1]:.2f}, {b[2]:.2f}, {b[3]:.2f}]"

    dsp = display_outline()
    chg = charger_outline()
    cell = battery_outline()
    caps = ", ".join(
        f"[{hole(c + 1.5, r + 1)[0]:.2f}, {hole(c + 1.5, r + 1)[1]:.2f}]"
        for a, b in CROSSES.values()
        for c, r in cross_switches(a, b).values())
    stand = ", ".join(f"[{hole(c, r)[0]:.2f}, {hole(c, r)[1]:.2f}]"
                      for c, r in STANDOFF_HOLES)
    headers = ", ".join(rect(n) for n in ("J1", "BT1", "J2", "J3", "SW17"))
    passives = ", ".join(f"[{part_body(n)[0]:.2f}, {part_body(n)[1]:.2f}, "
                         f"{part_body(n)[2]:.2f}, {part_body(n)[3]:.2f}, {h}]"
                         for n, h in (("D1", 2.2), ("R1", 2.0),
                                      ("R2", 2.0), ("C1", 2.6)))
    wires = ", ".join(
        "[" + ", ".join(f"[{hole(c, r)[0]:.2f}, {hole(c, r)[1]:.2f}]"
                        for c, r in pts) + "]"
        for _, pts in JUMPERS)
    strips = ", ".join(f"[{col}, {r0}, {r1}]" for col, r0, r1 in strip_segments())
    cuts = ", ".join(f"[{c}, {r}]" for c, r in CUTS)
    return f"""// mega blastoise - the assembled board, as a solid
// GENERATED by layout.py; edit the geometry there and re-run, not here.
//
//   openscad -o assembled.stl assembled.scad
//   openscad -o part.stl -D 'part="display"' assembled.scad
//
// part = "all" | "board" | "parts" | "display" | "cell" | "wires"

part        = "all";
show_holes  = true;    // 1344 of them; off makes the STL export instant
show_copper = false;   // strips and cuts: worth seeing in preview (F5), but an
                       // STL has no colour and unioning a second 1300-hole mesh
                       // into the board costs CGAL many minutes
sit_on_bed  = true;    // lift so the lowest point is z = 0

pitch      = {PITCH};
cols       = {COLS};
rows       = {ROWS};
grid_inset = {PITCH / 2:g};
board_w    = {BOARD_W};
board_l    = {BOARD_H};
board_t    = {BOARD_T_MM};
hole_d     = 1.0;
strip_w    = 1.9;
copper_t   = 0.05;
copper_gap = 0.01;   // holds the copper off the board's coplanar face

// heights above the component-side face, which is z = 0
header_h   = {HEADER_MM};
pico_t     = {PICO_T_MM};
switch_h   = {SWITCH_T_MM};
cap_h      = {CAP_T_MM};
cap_d      = {CAP_DIA_MM};
charger_t  = {CHARGER_T_MM};
foam_t     = {FOAM_MM};
deck_h     = {DECK_HEIGHT_MM};
display_t  = {DISPLAY_T_MM};
cell_t     = {CELL_T_MM};
wire_d     = 1.0;
wire_z     = 0.6;
standoff_d = 6.0;
standoff_base_d = {STANDOFF_BASE_MM};

pico     = {rect("U1")};
display  = [{dsp[0]:.2f}, {dsp[1]:.2f}, {dsp[2]:.2f}, {dsp[3]:.2f}];
active   = [{DISPLAY_ACTIVE_MM[0]:.2f}, {DISPLAY_ACTIVE_MM[1]:.2f}];
charger  = [{chg[0]:.2f}, {chg[1]:.2f}, {chg[2]:.2f}, {chg[3]:.2f}];
cell     = [{cell[0]:.2f}, {cell[1]:.2f}, {cell[2]:.2f}, {cell[3]:.2f}];
caps     = [{caps}];
standoffs = [{stand}];
headers  = [{headers}];
passives = [{passives}];
wires    = [{wires}];
strips   = [{strips}];
cuts     = [{cuts}];

function hole_x(c) = grid_inset + (c - 1) * pitch;
function hole_y(r) = grid_inset + (r - 1) * pitch;

module slab(r, z0, h) {{ translate([r[0], r[1], z0]) cube([r[2], r[3], h]); }}

// --------------------------------------------------------------- the board

// The 1344 holes are the expensive part, so the board is built as one drilled
// band that render() bakes once and 56 copies reuse. Subtracting the whole
// grid in one go is the same shape and costs CGAL minutes instead of a second.

module drilled_band() {{
    render()
        difference() {{
            square([board_w, pitch]);
            for (c = [1:cols])
                translate([hole_x(c), pitch / 2]) circle(d = hole_d, $fn = 8);
        }}
}}

// One drilled cell of copper, reused for all 1301 of them. Drilling each
// strip to its own length instead means 21 separate CGAL renders and a union
// of 67 different polygons, which takes minutes rather than half a second.
module copper_cell() {{
    render()
        difference() {{
            square([strip_w, pitch], center = true);
            circle(d = hole_d, $fn = 8);
        }}
}}

module board() {{
    translate([0, 0, -board_t])
        linear_extrude(board_t)
            if (show_holes)
                for (r = [0:rows - 1]) translate([0, r * pitch]) drilled_band();
            else
                square([board_w, board_l]);
    // a hair of air under the board: two 1300-hole meshes meeting on a
    // coplanar face is the most expensive thing CGAL can be asked to do here
    if (show_copper)
        translate([0, 0, -board_t - copper_t - copper_gap])
            linear_extrude(copper_t)
                for (s = strips)
                    if (show_holes)
                        for (r = [s[1]:s[2]])
                            translate([hole_x(s[0]), hole_y(r)]) copper_cell();
                    else
                        translate([hole_x(s[0]) - strip_w / 2,
                                   hole_y(s[1]) - pitch / 2])
                            square([strip_w, (s[2] - s[1] + 1) * pitch]);
}}

// --------------------------------------------------------- what stands on it

module pico() {{
    // the 2x20 header holds it off the board
    for (dx = [1.27, pico[2] - 1.27])
        translate([pico[0] + dx - 1.27, pico[1], 0])
            cube([2.54, pico[3], header_h]);
    slab(pico, header_h, pico_t);
    // the RP2350 and the USB shell, so it reads as a Pico
    translate([pico[0] + pico[2] / 2 - 3.5, pico[1] + pico[3] / 2 - 3.5,
               header_h + pico_t])
        cube([7, 7, 1.0]);
    translate([pico[0] + 1.5, pico[1] + pico[3] / 2 - 3.75,
               header_h + pico_t])
        cube([8.0, 7.5, 2.9]);
}}

module charger() {{
    slab(charger, foam_t, 1.6);
    translate([charger[0] + charger[2] - 9.0,
               charger[1] + charger[3] / 2 - 4.5, foam_t + 1.6])
        cube([9.0, 9.0, charger_t - 1.6]);
}}

module switches() {{
    for (p = caps) {{
        translate([p[0] - 3, p[1] - 3, 0]) cube([6, 6, switch_h]);
        translate([p[0], p[1], switch_h])
            cylinder(h = 1.0, d = 3.5, $fn = 24);
        translate([p[0], p[1], switch_h])
            cylinder(h = cap_h, d = cap_d, $fn = 40);
    }}
}}

module small_parts() {{
    for (h = headers) slab(h, 0, deck_h);
    for (p = passives) slab([p[0], p[1], p[2], p[3]], 0, p[4]);
}}

module standoff_posts() {{
    for (p = standoffs) {{
        translate([p[0], p[1], 0]) cylinder(h = 1.0, d = standoff_base_d, $fn = 40);
        translate([p[0], p[1], 0]) cylinder(h = deck_h, d = standoff_d, $fn = 32);
    }}
}}

module fly_wires() {{
    for (w = wires)
        for (i = [0:len(w) - 2])
            hull() {{
                translate([w[i][0], w[i][1], wire_z])
                    sphere(d = wire_d, $fn = 12);
                translate([w[i + 1][0], w[i + 1][1], wire_z])
                    sphere(d = wire_d, $fn = 12);
            }}
}}

module display_module() {{
    slab(display, deck_h, 1.6);
    slab(display, deck_h + 1.6, display_t - 1.6);
    translate([display[0] + (display[2] - active[0]) / 2, display[1] + 6.0,
               deck_h + display_t])
        cube([active[0], active[1], 0.4]);
}}

module cell_pack() {{
    slab(cell, -board_t - foam_t - cell_t, cell_t);
}}

// ------------------------------------------------------------------ assembly

module everything() {{
    if (part == "all" || part == "board") board();
    if (part == "all" || part == "wires") fly_wires();
    if (part == "all" || part == "parts") {{
        pico(); charger(); switches(); small_parts(); standoff_posts();
    }}
    if (part == "all" || part == "display") display_module();
    if (part == "all" || part == "cell") cell_pack();
}}

lowest = -board_t - foam_t - cell_t;
translate([0, 0, sit_on_bed ? -lowest : 0]) everything();

// ------------------------------------------------------------ self-checking

echo(str("board: ", board_w, " x ", board_l, " x ", board_t, " mm"));
echo(str("caps: ", len(caps), ", headers: ", len(headers),
         ", passives: ", len(passives), ", wires: ", len(wires)));
echo(str("strips: ", len(strips), ", cuts: ", len(cuts)));
echo(str("stack: cell bottom ", lowest, " mm, screen top ",
         deck_h + display_t, " mm, total ", deck_h + display_t - lowest, " mm"));
echo(str("tallest cap top: ", switch_h + cap_h, " mm, deck at ", deck_h, " mm"));

assert(len(caps) == 16, "there should be sixteen buttons");
assert(len(standoffs) == 4, "the display needs four standoffs");
assert(switch_h + cap_h <= deck_h + 0.001,
       "a cap stands taller than the display deck");
assert(charger_t + foam_t < deck_h,
       "the charger will not fit under the display");
assert(header_h + pico_t + 2.9 < deck_h,
       "the Pico's USB shell will not fit under the display");
for (p = caps) {{
    assert(p[0] - cap_d / 2 > -0.001 && p[0] + cap_d / 2 < board_w + 0.001,
           "a cap hangs off the side of the board");
    assert(p[1] - cap_d / 2 > -0.001 && p[1] + cap_d / 2 < board_l + 0.001,
           "a cap hangs off the end of the board");
}}
"""


# ------------------------------------------------------------- the button cap

CAP_PLUNGER_D = 3.5       # round post on a 6x6 tactile; MEASURE yours
CAP_TOP_T = 1.6           # roof over the bore
CAP_DISH = 0.5            # how far the top face is hollowed for a fingertip
CAP_BORE_CLEARANCE = 0.1  # printed holes come out under size


def button_cap_scad():
    """A printed cap that grips the round plunger of a 6x6 tactile switch.

    The switches sold as cap-ready are a different, larger part; the ones in
    every assortment kit have a plain round post, and a bore of the right size
    holds one perfectly well by friction. That is what the old board's caps did
    on its 12x12 switches. This is the same idea at the diameter this board can
    actually fit, which is not the diameter the old caps were.

    The width is not a preference. At the compact cross pitch there is
    {gap:.2f} mm between neighbouring caps and {dsp:.2f} mm to the display, so
    9 mm is a ceiling rather than a choice.
    """
    caps = [hole(c + 1.5, r + 1) for a, b in CROSSES.values()
            for c, r in cross_switches(a, b).values()]
    tight = min(math.dist(a, b) for a, b in itertools.combinations(caps, 2))
    dsp = display_outline()
    to_screen = min(dsp[1] - (max(y for _, y in caps if y < BOARD_H / 2)
                              + CAP_DIA_MM / 2),
                    (min(y for _, y in caps if y >= BOARD_H / 2) - CAP_DIA_MM / 2)
                    - (dsp[1] + dsp[3]))
    return f"""// mega blastoise - button cap for a 6x6 tactile switch
// GENERATED by layout.py; edit the geometry there and re-run, not here.
//
//   openscad -o cap.stl button_cap.scad
//   openscad -o gauge.stl -D 'part="gauge"' button_cap.scad
//
// Print the GAUGE first. It carries five bores either side of the nominal
// size; whichever grips the plunger without splitting is the number to put in
// plunger_d before printing sixteen caps.

part        = "cap";     // "cap" | "gauge" | "row"

cap_d       = {CAP_DIA_MM};      // a ceiling, not a preference: see below
plunger_d   = {CAP_PLUNGER_D};      // round post on the switch. MEASURE YOURS.
plunger_h   = 5.2;      // how far it stands above the switch body
bore_clear  = {CAP_BORE_CLEARANCE};      // printed holes come out under size
top_t       = {CAP_TOP_T};      // roof over the bore
dish        = {CAP_DISH};      // hollow in the top face, for a fingertip
chamfer     = 0.6;
$fn         = 72;

bore_d = plunger_d + bore_clear;
bore_h = plunger_h - 0.4;          // stop short so the cap seats on the post
cap_h  = bore_h + top_t;
wall   = (cap_d - bore_d) / 2;

module cap() {{
    difference() {{
        // body, chamfered top and bottom so it prints and handles cleanly
        hull() {{
            translate([0, 0, chamfer])
                cylinder(h = cap_h - 2 * chamfer, d = cap_d);
            cylinder(h = cap_h - chamfer, d = cap_d - 2 * chamfer);
            translate([0, 0, chamfer]) cylinder(h = cap_h - chamfer, d = cap_d - 2 * chamfer);
        }}
        // the bore, mouth chamfered so it finds the post
        translate([0, 0, -0.01]) cylinder(h = bore_h, d = bore_d);
        translate([0, 0, -0.01])
            cylinder(h = 0.8, d1 = bore_d + 1.2, d2 = bore_d);
        // fingertip dish
        if (dish > 0)
            translate([0, 0, cap_h + (cap_d * cap_d / 4 + dish * dish) / (2 * dish) - dish])
                sphere(r = (cap_d * cap_d / 4 + dish * dish) / (2 * dish));
    }}
}}

// Five bores around the nominal, so one test print settles the fit.
module gauge() {{
    steps = [-0.2, -0.1, 0, 0.1, 0.2];
    for (i = [0:len(steps) - 1])
        translate([i * (cap_d + 2), 0, 0])
            difference() {{
                cylinder(h = 4, d = cap_d);
                translate([0, 0, -0.01])
                    cylinder(h = 4.02, d = bore_d + steps[i]);
                translate([0, -cap_d / 2 - 0.4, 3.4])
                    linear_extrude(0.7)
                        text(str(steps[i]), size = 1.8, halign = "center",
                             font = "monospace");
            }}
}}

if (part == "cap")        cap();
else if (part == "gauge") gauge();
else if (part == "row")   for (i = [0:3]) translate([i * (cap_d + 2), 0, 0]) cap();

// ------------------------------------------------------------ self-checking

echo(str("cap: ", cap_d, " mm across, ", cap_h, " mm tall, wall ", wall, " mm"));
echo(str("bore ", bore_d, " mm x ", bore_h, " mm deep, for a ", plunger_d,
         " mm post standing ", plunger_h, " mm proud"));
echo(str("cap top ends up ", 4.3 + plunger_h + top_t,
         " mm above the board"));
echo(str("gap to the next cap ", {tight:.2f} - cap_d,
         " mm, to the display ", {to_screen:.2f} + ({CAP_DIA_MM} - cap_d) / 2, " mm"));

assert(cap_d <= {CAP_DIA_MM} + 0.001,
       "wider than {CAP_DIA_MM} mm and the caps foul each other at the cross pitch");
assert(cap_d < {tight:.2f}, "caps would overlap their neighbours");
assert(wall >= 1.0, "wall too thin to survive a thumb");
assert(bore_h > 2.5, "not enough bore to hold the cap on");
assert(bore_h + top_t > bore_h, "no roof on the bore");
assert(dish < top_t, "the dish would break through the roof");
"""


# ------------------------------------------------------- the marking stencil

STENCIL_DOWELS = [(1, 1), (24, 1), (1, 56), (24, 56)]
STENCIL_HOLE_D = 1.7        # 0.84 mm of wall between the row-34 fence holes
STENCIL_PLATE_T = 1.0
STENCIL_LIP_H = 2.6         # wraps a 1.6 mm board with room to lift it off
STENCIL_SLOP = 0.6          # total, so the board drops in without forcing
STENCIL_MARGIN = 3.0        # plate overhang outside the lip


def stencil_scad():
    """A drop-on mask for marking the 43 cut holes, generated from CUTS.

    Counting 56 rows by eye is what loses a cut; this removes the counting.
    The mask captures the board on its outline, four wire dowels through the
    corner holes stop it rotating, and a fine scribe through each opening
    lands in exactly one hole.

    It is written mirrored, because the cuts are made from the copper side.
    """
    cuts = ", ".join(f"[{c},{r}]" for c, r in sorted(CUTS, key=lambda t: (t[1], t[0])))
    dowels = ", ".join(f"[{c},{r}]" for c, r in STENCIL_DOWELS)
    return f"""// mega blastoise - track-cut marking stencil
// GENERATED by layout.py; edit the geometry there and re-run, not here.
//
// Lay it on the COPPER side, drop wire dowels through the four corner holes,
// then put a scribe or an ultra-fine pen through each opening. Every opening
// is one hole to destroy. {len(CUTS)} of them.

pitch       = {PITCH};
cols        = {COLS};
rows        = {ROWS};
grid_inset  = {PITCH / 2:g};      // first hole centre in from the board edge

// Measure YOUR board and put the numbers here; the pocket is cut to fit it.
board_w     = {BOARD_W};
board_l     = {BOARD_H};
board_t     = 1.6;

plate_t     = {STENCIL_PLATE_T};
hole_d      = {STENCIL_HOLE_D};
dowel_d     = 1.3;       // clearance for 0.8-1.0 mm wire
lip_h       = {STENCIL_LIP_H};
slop        = {STENCIL_SLOP};
margin      = {STENCIL_MARGIN};
label_depth = 0.4;
corner_chamfer = 6;   // the A1 corner, cut off so the mask reads one way

// The cuts are made from the copper side, which mirrors left for right.
mirror_for_copper = true;

cuts   = [{cuts}];
dowels = [{dowels}];

// ---------------------------------------------------------------- geometry

pocket_w = board_w + slop;
pocket_l = board_l + slop;
outer_w  = pocket_w + 2 * margin;
outer_l  = pocket_l + 2 * margin;

function hole_x(c) = grid_inset + (c - 1) * pitch;
function hole_y(r) = grid_inset + (r - 1) * pitch;
// mirroring keeps the row and flips the column, so the mask reads the way the
// copper side does when the board is turned over about its long edge
function map_x(c) = mirror_for_copper ? board_w - hole_x(c) : hole_x(c);

module opening(c, r, d) {{
    translate([map_x(c), hole_y(r), -1])
        cylinder(h = plate_t + 2, d = d, $fn = 24);
}}

module plate() {{
    difference() {{
        union() {{
            // the mask itself
            translate([-margin, -margin, 0])
                cube([outer_w, outer_l, plate_t]);
            // lip walls hanging down around the board
            translate([-margin, -margin, -lip_h])
                difference() {{
                    cube([outer_w, outer_l, lip_h]);
                    translate([margin - slop / 2, margin - slop / 2, -1])
                        cube([pocket_w, pocket_l, lip_h + 2]);
                }}
        }}
        for (cr = cuts)   opening(cr[0], cr[1], hole_d);
        for (cr = dowels) opening(cr[0], cr[1], dowel_d);

        // the A1 corner is cut away, so you can see at a glance which end is
        // which; the hole grid is symmetric under a half turn and nothing
        // physical can stop you fitting it backwards
        translate([map_x(1) > board_w / 2 ? outer_w - margin : -margin,
                   -margin, -lip_h - 1])
            rotate([0, 0, 45])
                translate([-corner_chamfer / sqrt(2), -corner_chamfer / sqrt(2), 0])
                    cube([corner_chamfer * sqrt(2), corner_chamfer * sqrt(2),
                          lip_h + plate_t + 2]);

        // which way round, engraved where you will read it
        translate([outer_w / 2 - margin, board_l + margin - 2.0,
                   plate_t - label_depth])
            linear_extrude(label_depth + 0.1)
                text("COPPER SIDE UP", size = 3.2, halign = "center",
                     valign = "top", font = "monospace");
        translate([map_x(1) + (mirror_for_copper ? -5.5 : 5.5), hole_y(1) + 6.5,
                   plate_t - label_depth])
            linear_extrude(label_depth + 0.1)
                text("A1", size = 3.0, halign = "center", font = "monospace");
        translate([map_x(cols) + (mirror_for_copper ? 4.5 : -4.5),
                   hole_y(rows) - 3.0, plate_t - label_depth])
            linear_extrude(label_depth + 0.1)
                text("X56", size = 3.0, halign = "center", font = "monospace");
    }}
}}

plate();

// ------------------------------------------------------------ self-checking

echo(str("cuts: ", len(cuts), ", dowels: ", len(dowels)));
echo(str("plate: ", outer_w, " x ", outer_l, " x ", plate_t, " mm, lip ", lip_h));
echo(str("pocket: ", pocket_w, " x ", pocket_l, " mm for a ",
         board_w, " x ", board_l, " board"));
echo(str("wall between the closest openings: ", pitch - hole_d, " mm"));

assert(len(cuts) == {len(CUTS)}, "cut count drifted from layout.py");
assert(hole_d < pitch - 0.5, "openings would merge into their neighbours");
assert(dowel_d < pitch - 0.5, "dowel holes would merge into their neighbours");
for (cr = cuts) {{
    assert(cr[0] >= 1 && cr[0] <= cols, "cut column off the board");
    assert(cr[1] >= 1 && cr[1] <= rows, "cut row off the board");
}}

// nothing may fall inside the chamfered corner, openings least of all
chamfer_line = (outer_w - margin) + margin - corner_chamfer;
for (cr = concat(cuts, dowels))
    assert(map_x(cr[0]) - hole_y(cr[1]) < chamfer_line - hole_d,
           str("opening ", cr, " is inside the A1 chamfer"));
"""


def build_md():
    L = []
    A = L.append
    A("# Mega Blastoise single-screen stripboard - build guide")
    A("")
    A("Generated by `layout.py`. Do not edit by hand; edit the geometry there and")
    A("re-run it. The KiCad schematic in this directory is the electrical")
    A("authority, this file is the physical one, and both come from the same map.")
    A("")
    A("## 0. What this board is")
    A("")
    A(f"ONE 24x56 stripboard ({BOARD_W:.0f} x {BOARD_H:.0f} mm, strips running the")
    A("long way) carrying a Raspberry Pi Pico 2, one 3.5\" ST7796 320x480 SPI")
    A("display standing portrait in the middle, and a BAND at each end holding")
    A("that player's two compact button crosses side by side - eight buttons per")
    A("player, d-pad cross on the player's left, action cross on their right.")
    A("")
    A("The compact cross interlocks its four switches: N/S centres are 10.16 mm")
    A("apart, W/E 20.3 mm, everything else 11.4 mm. **Button caps must be 9 mm**")
    A("- the old 11.3 mm caps collide at this pitch. Cap edge clearance at the")
    A("tightest pairs is 1.2 mm.")
    A("")
    A("**Strips run top to bottom** (Player 2's edge to Player 1's edge), one")
    A(f"strip per column. Three straight cut lines organise the board: fences on")
    A(f"rows {FENCE_NORTH} and {FENCE_SOUTH} isolate the two button bands, and")
    A(f"row {PICO_CUT_ROW} splits every column under the Pico so the middle of")
    A("each strip belongs to exactly one Pico pin. Buttons then need one wire")
    A("each: band strip to GPIO strip.")
    A("")
    A("There is no button matrix and there are no diodes. All 16 switches go")
    A("straight to their own GPIO with the internal pull-up enabled, so ghosting")
    A("cannot happen. The LiPo cell mounts on the BACK (copper side) on a foam")
    A("pad; the TP4056 mounts on the FRONT under the display deck with its")
    A("USB-C overhanging the right edge, so cell and charger sit on opposite")
    A("faces and the cell's leads come round the edge to BT1.")
    A("")
    A("## 1. Parts")
    A("")
    A("| Qty | Part | Spec | Refs | Est USD | Note |")
    A("|---|---|---|---|---|---|")
    for qty, part, value, refs, note, usd, search in BOM:
        A(f"| {qty} | {part} | {value} | {refs} | {usd} | {note} |")
    A(f"| | **Total** | | | **{bom_total()}** | |")
    A(f"| | **Total less the display, already ordered** | | | "
      f"**{bom_total_new()}** | |")
    A("")
    A(BOM_NOTE)
    A("")
    A("`BOM.csv` next to this file carries the same table plus an Amazon search")
    A("string for each line.")
    A("")
    A("## 2. Coordinates")
    A("")
    A(f"Holes are named the usual stripboard way: **letter then number**, the")
    A(f"letter being the column (A..{col_name(COLS)}, left to right) and the")
    A(f"number the row (1..{ROWS}, from Player 2's edge to Player 1's). So E12")
    A("is column 5, row 12. Hole A1 sits")
    A(f"{MARGIN_X:.2f} mm in from the left edge and {MARGIN_Y:.2f} mm down from")
    A("the top. All views are of the component side.")
    A("")
    A("## 3. Cut the tracks first")
    A("")
    A(f"{len(CUTS)} cuts. Do every one before any part goes down; most sit under")
    A("the Pico and the display and cannot be reached later. A cut destroys the")
    A("hole it names. Verify each with a meter before moving on.")
    A("")
    A(f"**Row {PICO_CUT_ROW}, columns {col_name(PICO_COL0)}-"
      f"{col_name(PICO_COL0 + 19)}** is the")
    A("structural line: it splits every one of the Pico's columns into a")
    A("Player 2 half above and a Player 1 half below. Cut it first.")
    A("")
    A("The rest are per-column fences. They do NOT line up into rows - each one")
    A("is placed as close to the Pico as its own pads allow, because a fence")
    A("decides how far a net can reach and therefore how long its wire has to")
    A("be. Work down this list, or print `cut_template.svg` at 100% scale, tape")
    A("it to the copper side and cut every cross on it.")
    A("")
    A("| Column | Cut at | Holes |")
    A("|---|---|---|")
    by_col = {}
    for col, row in CUTS:
        if row == PICO_CUT_ROW and PICO_COL0 <= col <= PICO_COL0 + 19:
            continue
        by_col.setdefault(col, []).append(row)
    for col in sorted(by_col):
        rows = sorted(by_col[col])
        A(f"| {col_name(col)} | row{'s' if len(rows) > 1 else ''} "
          f"{', '.join(str(r) for r in rows)} | "
          f"{' '.join(hole_name(col, r) for r in rows)} |")
    A("")
    A("Any column not listed here is uncut on purpose: it is a signal riding")
    A("its own strip straight to a Pico pin. Do not add cuts that are not here.")
    A("")
    A("## 4. Where every part sits")
    A("")
    A("| Ref | Part | First pad | Notes |")
    A("|---|---|---|---|")
    for ref, fp, col, row, rot, note in PLACEMENTS:
        A(f"| {ref} | {fp.split(':')[-1]} | {hole_name(col, row)} | {note} |")
    A("")
    A("### The crosses")
    A("")
    A("Each switch straddles two columns 3 apart and two rows 2 apart. Solder all")
    A("four legs: the two on one column form one terminal, the two on the other")
    A("form the second. In the compact pattern the N and S switches share their")
    A("ground column; the single cut between them separates the two signals.")
    A("")
    for cross, (a, b) in CROSSES.items():
        A(f"**{cross}** - base {hole_name(a, b)}:")
        A("")
        A("| Position | Button | Pads |")
        A("|---|---|---|")
        for compass, (col, row) in cross_switches(a, b).items():
            net = CROSS_LABELS[cross][compass]
            A(f"| {compass} | {net} ({SWITCH_REFS[net]}) | "
              f"{hole_name(col, row)} {hole_name(col, row+2)} "
              f"{hole_name(col+3, row)} {hole_name(col+3, row+2)} |")
        A("")
    A("## 5. Which strip carries what")
    A("")
    A("| Net | Strip | Rows | Pico pin | Connection |")
    A("|---|---|---|---|---|")
    # the connection column is derived from the wire table, so it can never
    # describe a joint the wires no longer have
    jmap = {n.split(" to ")[0]: pts for n, pts in JUMPERS}

    def how_text(key, stored):
        pts = jmap.get(key)
        if not pts:
            return stored
        return "jumper " + " -> ".join(hole_name(c, r) for c, r in pts)

    for net, col, rows, pin, how in BUTTON_NETS:
        A(f"| {net} | {col_name(col)} | {rows} | {pin} ({PICO_PIN_NAME[pin]}) "
          f"| {how_text(net, how)} |")
    for jpin, name, col, rows, pico_pin, how in DISPLAY_NETS:
        key = {"VCC": "display VCC from 3V3", "LCD_SCK": "display SCK",
               "LCD_MOSI": "display MOSI", "LCD_RST": "display RST",
               "LCD_DC": "display DC", "LCD_CS": "display CS",
               "LCD_BL": "display BL"}.get(name, name)
        A(f"| {name} (J1.{jpin}) | {col_name(col)} | {rows} | {pico_pin} "
          f"({PICO_PIN_NAME[pico_pin]}) | {how_text(key, how)} |")
    A("| BATT+ | W | 1-17 | - | BT1 and J2 share the strip |")
    A("| BATT- | X | 8-18 | - | BT1 and J2 share the strip |")
    A("| VBATT | W | 19-23 | - | J3 OUT+ to SW17 on the strip |")
    A("| VBATT_SW | X | 23-36 | - | SW17 to D1 anode to R1 on the strip |")
    A("| VSYS | W | 25-27 | 39 (VSYS) | D1 cathode; see the wire table |")
    A("| VBAT_SENSE | V | 36-56 | 34 (GP28) | divider tap; see the wire table |")
    A("")
    A("Columns E and T are uncut through each band and meet real Pico ground")
    A("pins (3 and 18 below, 38 and 23 above): each cross's N/S switches ground")
    A("there for free. The outer-arm grounds (columns A, I, M, P, X band")
    A("segments) hang off the two ground buses, and the J1 GND pin joins the P1")
    A("bus at I56.")
    A("")
    A("## 6. Jumper wires")
    A("")
    A(f"{len(JUMPERS)} wires. Insulated, on the component side, soldered **only** at")
    A("the holes listed. The two buses are one wire each with several joints.")
    A("Endpoints are the closest free hole pair on the two strips each wire")
    A("joins, and each fence cut is slid as close to the Pico as its own pads")
    A("allow, since where a cut sits decides how far a net can reach. Together")
    A("that is 368 mm of wire, longest single run 48 mm.")
    A("")
    A("Wires may pass under a switch body - thread them between the legs before")
    A("the switch goes in and it still seats flat. Three wires also run beside a")
    A("ground bus; that is allowed, it just means dressing them apart by eye.")
    A("Wires that share a row simply lie over each other - only the endpoints")
    A("are electrical.")
    A("")
    A("Wire colours follow a standard 6-colour kit and match the preview:")
    A("blue = P1 buttons, green = P2 buttons, yellow = display signals,")
    A("red = power (3V3, VSYS), white = ground, orange = battery sense.")
    A("")
    A("| # | Wire | Colour | Solder at |")
    A("|---|---|---|---|")
    for i, (name, pts) in enumerate(JUMPERS, 1):
        holes = " ".join(hole_name(c, r) for c, r in pts)
        A(f"| {i} | {name} | {wire_color(name)[1]} | {holes} |")
    A("")
    A("## 7. Build order")
    A("")
    A("1. Make all cuts in section 3. Meter every one.")
    A("2. Solder the jumper wires in section 6. They run under the Pico and the")
    A("   display, so they must go down first. No solder joint sits under the")
    A("   Pico's body - every endpoint stays reachable for rework.")
    A("3. Solder the Pico. Headers on the Pico first, then drop it in with the")
    A(f"   USB pointing left, check it straddles the row {PICO_CUT_ROW} cuts,")
    A("   then solder.")
    A("4. Solder the 16 switches, all four legs each.")
    A("5. Solder D1, R1, R2, C1, the J1 socket and the 2-pin headers.")
    A("6. Wire the charger module, cell and slide switch to their headers.")
    A("7. Foam-pad the cell to the back of the board under the display area.")
    A(f"8. Plug the display into the J1 socket at columns "
      f"{DISPLAY_HEADER_COL0}-{DISPLAY_HEADER_COL0 + 7}, row {DISPLAY_HEADER_ROW}.")
    A("9. With the module seated, shim the four adhesive standoffs to the gap")
    A("   under its corners, then peel and stick them to the board. Doing this")
    A("   last lets them find their own height instead of fighting the socket.")
    A("")
    A("## 8. Power")
    A("")
    A("```")
    A("USB-C -> TP4056 (protected) -> OUT+ -> slide switch -> 1N5817 -> VSYS")
    A("                            -> OUT- -> GND")
    A("```")
    A("")
    A("The charger's USB-C and the Pico's micro-USB may be plugged in at the same")
    A("time. VBUS reaches VSYS through the Pico's own internal Schottky and the")
    A("cell reaches it through D1, so the higher rail simply wins - about 4.7 V")
    A("from USB against about 3.9 V from a full cell - and neither source can push")
    A("current back into the other. Charge current is set by the module's")
    A("programming resistor; 1 A (1.2 kohm) suits a 2000 mAh cell.")
    A("")
    A("Battery sense: cell through R1/R2 (100k/100k) into GP28 (ADC2), filtered by")
    A("C1. The divider hangs off the switched side, so it draws nothing when the")
    A("console is off. Reading is half the cell voltage: 2.1 V at 4.2 V full,")
    A("1.5 V at 3.0 V empty.")
    A("")
    A("### The charger")
    A("")
    A(f"The TP4056 is {TP4056_MM[0]:.0f} x {TP4056_MM[1]:.0f} mm and sits on the")
    A(f"FRONT with its top-left corner near hole "
      f"{hole_name(TP4056_COL0, TP4056_ROW0)},")
    A("right-aligned so the USB-C hangs 1 mm past the board edge. It is about")
    A(f"4.8 mm tall including the connector, well under the {DECK_HEIGHT_MM} mm")
    A("display deck it lives beneath. Stick it down on foam tape once its four")
    A("leads are soldered.")
    A("")
    A("It has no pads on the board of its own - it reaches the circuit entirely")
    A("through four flying leads. On this module all four sit on the short edge")
    A("**opposite the USB-C**, reading OUT- B- B+ OUT+ across it; the IN+ and")
    A("IN- pads up beside the connector are unused, since charging comes in")
    A("through the USB-C itself.")
    A("")
    A("Mounted with the connector over the right board edge, that pad edge")
    A("faces left, so all four leads fan out from one side:")
    A("")
    A("| Module pad | Goes to | Hole |")
    A("|---|---|---|")
    A("| OUT- | J3.2 | X21 |")
    A("| B- | J2.2 | X10 |")
    A("| B+ | J2.1 | W10 |")
    A("| OUT+ | J3.1 | W21 |")
    A("")
    A("Rotating the module the other way in its own plane reverses that order,")
    A("swapping the two OUT leads. Read the silkscreen, not the picture.")
    A("")
    A("The cell's own leads land on BT1 at W8 and X8, which share those")
    A("strips with J2 - that is how cell and charger meet.")
    A("")
    A("Cell on the back, charger on the front. The cell's leads come round the")
    A("board edge to BT1; keep them clear of the P1 button caps.")
    A("")
    A("## 9. Display and access")
    A("")
    A(f"The module is {DISPLAY_MM[0]} x {DISPLAY_MM[1]} mm with a")
    A(f"{DISPLAY_ACTIVE_MM[0]} x {DISPLAY_ACTIVE_MM[1]} mm active area, portrait in")
    A("the middle, split into two 320x240 halves with the far half rotated 180")
    A("degrees. It spans the board's full width less 2.7 mm a side, and clears")
    A("the nearest button caps by about half a millimetre at each band - check")
    A("cap seating before final assembly.")
    A("")
    A("### Holding the display down")
    A("")
    A("The 8-pin socket carries the electrical connection but the module")
    A("cantilevers about 94 mm past it, so the far end needs support. Four")
    A(f"**self-adhesive nylon standoffs** ({STANDOFF_HEIGHT_MM:.0f} mm tall,")
    A(f"~{STANDOFF_BASE_MM:.0f} mm base) sit under the module's corners at holes:")
    A("")
    for c, r in STANDOFF_HOLES:
        A(f"- {hole_name(c, r)}")
    A("")
    A("Nothing is drilled. The pads bond to the bare component side and the")
    A("copper is on the reverse, which is why two of these can sit over live")
    A("power strips without consequence. Screws were rejected: the module's own")
    A("corner holes are roughly 50 x 92 mm apart and the 0.1 in grid can only")
    A("land on multiples of 2.54 mm, so they will not line up.")
    A("")
    A(f"Deck height is set by the socket, about {DECK_HEIGHT_MM:.1f} mm. Plug the")
    A("module in first, measure the real gap under each corner, then shim each")
    A("standoff with washers or foam tape before peeling the backing - stick")
    A("them down last, with the module in place, so they self-align.")
    A("")
    A("**Verify the pin order against the module's silkscreen before soldering.**")
    A("This layout assumes the common order 1=GND 2=VCC 3=SCL/SCK 4=SDA/MOSI")
    A("5=RES 6=DC 7=CS 8=BLK. If your module differs, the fix is to re-target the")
    A("eight display jumpers in section 6, not to move the socket.")
    A("")
    A("The Pico's micro-USB faces the left board edge under the display overhang;")
    A("the plug (8 mm tall) fits under the module on its 10 mm standoffs, but it")
    A("is snug - check with your cable. BOOTSEL ends up under the display too:")
    A("unplug the module from its socket to reach it, or flash over SWD/picotool.")
    A("The case needs ~10 mm of depth behind the board for the cell.")
    A("")
    A("## 10. Firmware pin map")
    A("")
    A("| Signal | GPIO | Pico pin |")
    A("|---|---|---|")
    for net, col, rows, pin, how in BUTTON_NETS:
        A(f"| {net} | {PICO_PIN_NAME[pin]} | {pin} |")
    for jpin, name, col, rows, pico_pin, how in DISPLAY_NETS:
        if name not in ("VCC", "GND"):
            A(f"| {name} | {PICO_PIN_NAME[pico_pin]} | {pico_pin} |")
    A("| VBAT_SENSE | GP28 | 34 |")
    A("")
    A("The display bus is **PIO, not hardware SPI**. The reversed module puts")
    A("SCK over GP9 and MOSI over GP8, a pair neither SPI block can drive, and")
    A("PIO clocks the panel just as fast. Buttons are inputs with `Pull::Up`,")
    A("active low. GP0, GP15 and GP16 are unused - GP0 is UART0 TX, kept free")
    A("for a debug console. There is no buzzer: this console is silent by")
    A("design.")
    A("")
    return "\n".join(L)



def assembly_md():
    """A staged build order, with a test at the end of every stage.

    BUILD.md is the reference: what goes where, and why. This is the procedure:
    what to do next, in what order, and how to know the stage worked before
    the next one buries it. Everything here is generated from the same tables,
    so a change to the layout changes the instructions.
    """
    L = []
    A = L.append
    sig = {n: c for n, c, r, p, h in BUTTON_NETS}
    pin_of = {n: p for n, c, r, p, h in BUTTON_NETS}
    switches = []
    for cross, (a, b) in CROSSES.items():
        for cm, (col, row) in cross_switches(a, b).items():
            net = CROSS_LABELS[cross][cm]
            gnd = col + 3 if sig[net] == col else col
            switches.append((SWITCH_REFS[net], net, sig[net], gnd, row,
                             pin_of[net]))
    switches.sort(key=lambda t: int(t[0][2:]))

    A("# Assembly, stage by stage")
    A("")
    A("Generated by `layout.py`. `BUILD.md` is the reference - what goes where")
    A("and why. This is the procedure: the order to do it in, and a test at the")
    A("end of each stage so a mistake is found while it is still reachable.")
    A("")
    A("**Do not skip the checks.** Almost everything on this board ends up")
    A("under the Pico or the display, and the cost of finding a bad joint")
    A("later is measured in desoldering, not minutes.")
    A("")
    A("## Before you start")
    A("")
    A("Orientation, used by every coordinate here: **component side up, Player")
    A(f"2's edge at the top**. Columns are lettered A..{col_name(COLS)} left to")
    A(f"right, rows numbered 1..{ROWS} top to bottom. So H54 is column H, row")
    A("54. Mark A1 in the corner with a pen before you start; every mistake in")
    A("this build begins with counting from the wrong edge.")
    A("")
    A("Tools: fine-tip iron, 0.7 mm solder, a multimeter with a continuity")
    A("beeper, a spot cutter or 3 mm drill bit held in the fingers, flush")
    A("cutters, tweezers. A helping hand is worth having for the module leads.")
    A("")
    A("| Stage | What | Rough time |")
    A("|---|---|---|")
    A("| 1 | Cut the tracks | 45 min |")
    A("| 2 | Jumper wires | 60 min |")
    A("| 3 | The Pico | 20 min |")
    A("| 4 | Switches | 40 min |")
    A("| 5 | Power train parts | 30 min |")
    A("| 6 | Charger, cell, switch | 30 min |")
    A("| 7 | Display | 20 min |")
    A("| 8 | First power | 20 min |")
    A("")

    # ---- stage 1
    A(f"## Stage 1 - Cut the tracks ({len(CUTS)} cuts)")
    A("")
    A("Everything else sits on top of these, and several end up under the Pico")
    A("or the display, so they all happen now. Cut from the **copper side**;")
    A("`cut_template.svg` prints 1:1 and is already mirrored for it.")
    A("")
    A("### Mark them with the stencil, not by counting")
    A("")
    A("Counting to row 34 by eye is how a cut goes missing, and the one that")
    A("goes missing is usually one of the twenty in the fence, because they")
    A("look like a single thing rather than twenty things. Two ways to avoid")
    A("counting at all:")
    A("")
    A("1. **Printed mask** - `cuts_stencil.scad` builds a 1 mm plate the size")
    A("   of the board with an opening over each of the "
      f"{len(CUTS)} holes to destroy.")
    A("   Measure your board first and set `board_w` and `board_l`, then print")
    A("   it flat, no supports, 0.15-0.2 mm layers. Drop it on the copper side,")
    A("   push a scrap of 0.8-1 mm wire through each of the four corner dowel")
    A("   holes to stop it turning, and put a scribe or an ultra-fine pen")
    A("   through every opening. Lift it off and you have "
      f"{len(CUTS)} marks and nothing to count.")
    A("")
    A("2. **Paper** - print `cut_template.svg` at 100% and check the printed")
    A("   ruler bar really measures 100 mm before you trust it. Tape it to the")
    A("   copper, prick each X through with a needle, peel it off.")
    A("")
    A("Either way the mask is mirrored, so the A1 corner ends up on your right")
    A("when the copper faces you. Put a pen dot on the board's A1 corner before")
    A("you flip it: the hole grid is symmetric under a half turn, and nothing")
    A("physical stops the mask going on backwards. The chamfered corner and the")
    A("engraved `A1` are there to be checked, not to save you.")
    A("")
    A("![Every track cut](cuts_guide.png)")
    A("")
    A("*Component side, with only the cuts shown. The wire-colour key at the")
    A("bottom belongs to later stages and can be ignored here.*")
    A("")
    A(f"Start with the long line: **row {PICO_CUT_ROW}, columns "
      f"{col_name(PICO_COL0)} to {col_name(PICO_COL0 + 19)}** "
      f"({len(range(PICO_COL0, PICO_COL0 + 20))} cuts). This is the one that")
    A("splits the board into its two player halves.")
    A("")
    A("Then the per-column fences:")
    A("")
    A("| Column | Holes to destroy |")
    A("|---|---|")
    by_col = {}
    for col, row in CUTS:
        if row == PICO_CUT_ROW and PICO_COL0 <= col <= PICO_COL0 + 19:
            continue
        by_col.setdefault(col, []).append(row)
    for col in sorted(by_col):
        A(f"| {col_name(col)} | "
          f"{' '.join(hole_name(col, r) for r in sorted(by_col[col]))} |")
    A("")
    A("**Check:** meter across every cut, one probe each side of the destroyed")
    A("hole. All must read open. A cut that looks clean under a lamp and still")
    A("beeps is the single most common fault on stripboard: the copper tears")
    A("rather than lifts and leaves a whisker. Re-cut and re-test.")
    A("")

    # ---- stage 2
    A(f"## Stage 2 - Jumper wires ({len(JUMPERS)})")
    A("")
    A("These go down before anything else because most of them run under the")
    A("Pico, the display or the charger. Insulated wire on the component side,")
    A("soldered only at the holes listed. Colours are the ones in `BOM.csv` and")
    A("match the preview.")
    A("")
    A("Do the two ground buses last: they are the longest, and having them in")
    A("place makes the shorter wires harder to reach.")
    A("")
    A("| # | Wire | Colour | Solder at |")
    A("|---|---|---|---|")
    ordered = [w for w in JUMPERS if len(w[1]) == 2] + \
              [w for w in JUMPERS if len(w[1]) > 2]
    for i, (name, pts) in enumerate(ordered, 1):
        A(f"| {i} | {name} | {wire_color(name)[1]} | "
          f"{' '.join(hole_name(c, r) for c, r in pts)} |")
    A("")
    A("**Check:** with the meter, confirm each of these reads closed, probing")
    A("the two holes named. They are the connections the wires just made, and")
    A("every one of them is buried by a later stage:")
    A("")
    A("| Probe | And | Expect |")
    A("|---|---|---|")
    for name, pts in ordered[:6]:
        A(f"| {hole_name(*pts[0])} | {hole_name(*pts[1])} | closed |")
    A(f"| {hole_name(*ordered[-1][1][0])} | "
      f"{hole_name(*ordered[-1][1][-1])} | closed (P2 bus, end to end) |")
    A("")
    A("Then confirm the buses did **not** catch a neighbour: probe a bus joint")
    A("against the column beside it, and expect open.")
    A("")

    # ---- stage 3
    A("## Stage 3 - The Pico")
    A("")
    A("Solder its headers on first, then drop it in. **USB to the left.** Pin 1")
    A(f"lands in {hole_name(PICO_COL0, PICO_ROW_LOWER)} and pin 40 in")
    A(f"{hole_name(PICO_COL0, PICO_ROW_UPPER)}; the preview marks pin 1 with a")
    A("red box. Before soldering, check it straddles the row")
    A(f"{PICO_CUT_ROW} cuts - if it does not, it is a row out and every net on")
    A("the board is wrong.")
    A("")
    A("**Check:** pick three pins at random and confirm each reads closed to a")
    A("free hole further along its own column, and open to its neighbour. Then")
    A("look along the pin rows with a lamp for solder bridges, which are easy")
    A("to make at 2.54 mm and invisible from directly above.")
    A("")

    # ---- stage 4
    A(f"## Stage 4 - Switches ({len(switches)})")
    A("")
    A("Solder **all four legs** of each. The two legs on one column form one")
    A("terminal, the two on the other form the second; using only two legs")
    A("works electrically and then fails mechanically the first time a thumb")
    A("lands on it.")
    A("")
    A("| Ref | Button | Signal column | Ground column | Top row |")
    A("|---|---|---|---|---|")
    for ref, net, sc, gc, row, pin in switches:
        A(f"| {ref} | {net} | {col_name(sc)} | {col_name(gc)} | {row} |")
    A("")
    A("**Check:** this is the best test on the whole board, and it takes two")
    A("minutes. Put the meter across the switch's signal column and its ground")
    A("column, then press. Closed on press, open on release, every time:")
    A("")
    A("| Ref | Probe | And | Press |")
    A("|---|---|---|---|")
    for ref, net, sc, gc, row, pin in switches[:4]:
        A(f"| {ref} | {hole_name(sc, row)} | {hole_name(gc, row)} | closes |")
    A("| ... | and so on for all sixteen | | |")
    A("")
    A("If one is closed all the time, its signal and ground columns are")
    A("bridged - most likely a fence cut that did not take. If one never")
    A("closes, the switch is rotated 90 degrees.")
    A("")

    # ---- stage 5
    A("## Stage 5 - Power train parts")
    A("")
    A("| Ref | Part | Holes | Watch for |")
    A("|---|---|---|---|")
    pads = expected_pads()
    for ref, what, why in (
            ("D1", "1N5817 Schottky, standing up",
             "**cathode stripe toward " + hole_name(*pads["D1"]["1"][0]) +
             "** - the only polarised part here"),
            ("R1", "100k, standing up", "either way round"),
            ("R2", "100k, standing up", "either way round"),
            ("C1", "100 nF ceramic", "either way round"),
            ("J1", "1x8 female socket",
             "a socket, not pins: the display plugs into it"),
            ("BT1", "1x2 header", "the cell's leads land here"),
            ("J2", "1x2 header", "charger B+ / B-"),
            ("J3", "1x2 header", "charger OUT+ / OUT-"),
            ("SW17", "1x2 header", "flying leads out to the panel switch")):
        holes = " ".join(hole_name(*h) for v in pads[ref].values() for h in v)
        A(f"| {ref} | {what} | {holes} | {why} |")
    A("")
    A("**Check, before any power exists:** meter between the holes below and")
    A("expect **open** on all of them. A short here is the one fault that can")
    A("damage something later.")
    A("")
    A("| Probe | And | Expect |")
    A("|---|---|---|")
    A(f"| {hole_name(*pads['J3']['1'][0])} (VBATT) | "
      f"{hole_name(*pads['J3']['2'][0])} (OUT-) | open |")
    A(f"| {hole_name(*pads['BT1']['1'][0])} (BATT+) | "
      f"{hole_name(*pads['BT1']['2'][0])} (BATT-) | open |")
    A(f"| {hole_name(*pads['D1']['1'][0])} (VSYS) | any ground hole | open |")
    A("")
    A("And expect **closed** for the divider chain, which proves the tap is")
    A("really one node:")
    A("")
    A("| Probe | And | Expect |")
    A("|---|---|---|")
    A(f"| {hole_name(*pads['R1']['2'][0])} | {hole_name(*pads['R2']['1'][0])} "
      f"| closed (the tap) |")
    A(f"| {hole_name(*pads['R2']['2'][0])} | any ground hole | closed |")
    A("")

    # ---- stage 6
    A("## Stage 6 - Charger, cell and switch")
    A("")
    A("The charger goes on the **front**, under where the display will sit,")
    A("with its USB-C hanging over the right board edge. Solder its four leads")
    A("**before** peeling the tape: you cannot reach underneath afterwards.")
    A("")
    A("All four pads are on the short edge opposite the USB-C, reading")
    A("OUT- B- B+ OUT+ across it. Read the silkscreen, not this table, and if")
    A("the module is rotated the other way the two OUT leads swap:")
    A("")
    A("| Module pad | Goes to |")
    A("|---|---|")
    A(f"| B+ | J2.1, {hole_name(*pads['J2']['1'][0])} |")
    A(f"| B- | J2.2, {hole_name(*pads['J2']['2'][0])} |")
    A(f"| OUT+ | J3.1, {hole_name(*pads['J3']['1'][0])} |")
    A(f"| OUT- | J3.2, {hole_name(*pads['J3']['2'][0])} |")
    A("")
    A("Then the slide switch to SW17 on flying leads, and the cell to BT1 with")
    A("its leads brought round the board edge from the back. Foam-pad the cell")
    A("to the copper side last.")
    A("")
    A("**Check, in this order:**")
    A("")
    A("1. Switch **off**, cell connected: VSYS reads 0 V. If it does not, the")
    A("   switch is bypassed somewhere.")
    A(f"2. Switch **on**: {hole_name(*pads['D1']['1'][0])} reads the cell")
    A("   voltage less about 0.3 V, the drop across D1. Around 3.4 to 3.9 V on")
    A("   a part-charged cell.")
    A("3. USB-C plugged in: the module's red LED lights. Green means already")
    A("   full.")
    A("")

    # ---- stage 7
    A("## Stage 7 - Display")
    A("")
    A(f"The socket is already in from stage 5, at "
      f"{hole_name(DISPLAY_HEADER_COL0, DISPLAY_HEADER_ROW)} to "
      f"{hole_name(DISPLAY_HEADER_COL0 + 7, DISPLAY_HEADER_ROW)}.")
    A("")
    A("**Check the module's pin order against its own silkscreen first.** This")
    A("layout assumes 1=GND 2=VCC 3=SCL/SCK 4=SDA/MOSI 5=RES 6=DC 7=CS 8=BLK.")
    A("If yours differs, stop and re-target the display jumpers from stage 2;")
    A("do not rotate the socket.")
    A("")
    A("Plug the module in, then shim the four adhesive standoffs to the gap")
    A("under its corners and stick them down with the module in place, so they")
    A("find their own height rather than fighting the socket.")
    A("")

    # ---- stage 8
    A("## Stage 8 - First power")
    A("")
    A("Flash firmware over the Pico's own micro-USB before trusting the")
    A("battery path. BOOTSEL is under the display, so unplug the module from")
    A("its socket if you need it, or use SWD.")
    A("")
    A("In order:")
    A("")
    A("1. USB only, cell disconnected. The Pico should enumerate.")
    A("2. Read GP28 in firmware with the cell disconnected: it should sit near")
    A("   zero, since the divider has nothing to divide.")
    A("3. Cell connected, switch on, USB out. Firmware should read about half")
    A("   the cell voltage on GP28: 2.10 V at a full 4.2 V cell.")
    A("4. Every button, one at a time, against the map in section 10 of")
    A("   `BUILD.md`. A button that reports as its neighbour means two signal")
    A("   columns are bridged; a button that never reports means its jumper.")
    A("5. Display last. If it stays dark but the backlight is on, suspect CS")
    A("   or DC before suspecting the panel.")
    A("")
    A("Known and expected: the battery reading over-reads while USB-C is")
    A("plugged in, because the charger holds the cell up. Only trust the gauge")
    A("on battery.")
    A("")
    return "\n".join(L)



def power_diagram_svg():
    """The power train drawn as a circuit, not as a board.

    Same nets as the layout, but every connection is a line you can follow
    instead of two pads that happen to share a strip. Hole references sit
    beside each connection so it maps back to the board. This is a reading
    aid; the board and the schematic remain the authority.
    """
    W, H = 1120, 670
    o = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" '
         f'viewBox="0 0 {W} {H}" font-family="monospace">',
         f'<rect width="{W}" height="{H}" fill="#0d1117"/>']

    def esc(t):
        return (t.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;"))

    def txt(x, y, t, fill="#e6edf3", size=11, anchor="middle"):
        o.append(f'<text x="{x}" y="{y}" fill="{fill}" font-size="{size}" '
                 f'text-anchor="{anchor}">{esc(t)}</text>')

    def box(x, y, w, h, title, sub="", fill="#161b22", stroke="#8b949e"):
        o.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="5" '
                 f'fill="{fill}" stroke="{stroke}" stroke-width="1.8"/>')
        txt(x + w / 2, y + (h / 2 + 4 if not sub else h / 2 - 2), title, size=13)
        if sub:
            txt(x + w / 2, y + h / 2 + 14, sub, "#8b949e", 9)

    def wire(pts, color="#ff5555", label="", holes="", dash="", lpos=None):
        d = " ".join(("M" if i == 0 else "L") + f"{x},{y}"
                     for i, (x, y) in enumerate(pts))
        da = f' stroke-dasharray="{dash}"' if dash else ""
        o.append(f'<path d="{d}" stroke="{color}" stroke-width="2.4" '
                 f'fill="none"{da}/>')
        lx, ly = lpos if lpos else pts[len(pts) // 2]
        if label:
            txt(lx, ly - 8, label, color, 11)
        if holes:
            txt(lx, ly + 15, holes, "#6e7681", 9)

    def dot(x, y, color="#ff5555"):
        o.append(f'<circle cx="{x}" cy="{y}" r="4.5" fill="{color}"/>')

    txt(20, 32, "Power train, drawn as a circuit", "#e6edf3", 17, "start")
    txt(20, 52, "Grey text is the board hole each connection lands in.",
        "#8b949e", 11, "start")
    txt(20, 68, "Red = supply, white = ground, orange = the sense tap.",
        "#8b949e", 11, "start")

    GND = 580

    # sources
    box(30, 95, 110, 44, "USB-C", "5 V in")
    box(30, 215, 110, 56, "LiPo cell", "3.0 - 4.2 V")
    box(200, 95, 180, 180, "TP4056", "charger + DW01 protection",
        fill="#3d2a12", stroke="#f0883e")
    txt(212, 150, "B+", "#ffd8a8", 10, "start")
    txt(212, 205, "B-", "#ffd8a8", 10, "start")
    txt(368, 150, "OUT+", "#ffd8a8", 10, "end")
    txt(368, 205, "OUT-", "#ffd8a8", 10, "end")
    txt(290, 240, "B+ and OUT+ are one copper;", "#f0883e", 9)
    txt(290, 253, "B- reaches OUT- via the FETs", "#f0883e", 9)

    wire([(140, 117), (200, 117)], "#ff5555", "5 V")
    wire([(140, 233), (172, 233), (172, 147), (200, 147)], "#ff5555",
         "", "BT1 W8, J2 W10", lpos=(172, 190))
    wire([(140, 262), (184, 262), (184, 202), (200, 202)], "#f2f2f2",
         "", "BT1 X8, J2 X10", lpos=(184, 285))

    # switch, switched rail
    box(440, 125, 120, 44, "SW17", "power switch")
    txt(500, 186, "W23 to X23", "#6e7681", 9)
    wire([(380, 147), (440, 147)], "#ff5555", "VBATT", "W21", lpos=(410, 147))
    wire([(560, 147), (640, 147)], "#ff5555", "VBATT_SW", "")
    dot(640, 147)

    # D1 into VSYS
    box(680, 125, 90, 44, "D1", "1N5817")
    wire([(640, 147), (680, 147)], "#ff5555", "", "X27")
    wire([(770, 147), (860, 147)], "#ff5555", "VSYS", "W27 to D25")
    dot(860, 147)

    # the Pico's own USB, OR'd in through its internal Schottky
    box(680, 40, 100, 40, "micro-USB", "on the Pico")
    wire([(780, 60), (860, 60), (860, 147)], "#ff5555", "", "", "6 4",
         lpos=(820, 60))
    txt(822, 52, "VBUS 5 V", "#ff5555", 10)
    txt(822, 92, "through the Pico's own Schottky", "#8b949e", 9)

    # Pico and 3V3
    box(850, 200, 140, 120, "Pico 2", "buck-boost, 1.8-5.5 V in",
        fill="#1c2431", stroke="#58a6ff")
    wire([(860, 147), (860, 200)], "#ff5555")
    box(850, 400, 140, 44, "ST7796", "display")
    wire([(920, 320), (920, 400)], "#ff5555", "3V3", "G28 to J40", lpos=(920, 355))

    # sense divider
    box(600, 270, 80, 38, "R1 100k")
    box(600, 350, 80, 38, "R2 100k")
    box(700, 350, 76, 38, "C1 100nF")
    wire([(640, 147), (640, 270)], "#ff5555", "", "X26", lpos=(640, 215))
    wire([(640, 308), (640, 350)], "#ff9e4d", "tap", "V26, V24", lpos=(640, 329))
    dot(640, 329)
    wire([(640, 329), (738, 329), (738, 350)], "#ff9e4d", "", "V22",
         lpos=(700, 329))
    wire([(640, 329), (820, 329), (820, 240), (850, 240)], "#ff9e4d",
         "to GP28", "V12 to I13", lpos=(800, 240))
    wire([(640, 388), (640, GND)], "#f2f2f2", "", "T24", lpos=(640, 470))
    wire([(738, 388), (738, GND)], "#f2f2f2", "", "T22", lpos=(738, 500))

    # ground
    o.append(f'<path d="M160,{GND} L1060,{GND}" stroke="#f2f2f2" '
             f'stroke-width="3" fill="none"/>')
    txt(160, GND + 22, "GND", "#f2f2f2", 12, "start")
    wire([(380, 202), (380, GND)], "#f2f2f2", "OUT-", "X21 to T20",
         lpos=(380, 400))
    wire([(990, 260), (1040, 260), (1040, GND)], "#f2f2f2", "", "Pico GND",
         lpos=(1040, 340))
    wire([(920, 444), (920, GND)], "#f2f2f2", "", "J1 I46", lpos=(920, 510))

    txt(20, H - 20, "Only one source conducts at a time: whichever is higher "
                    "pulls VSYS up and reverse-biases the other diode.",
        "#8b949e", 10, "start")
    o.append("</svg>")
    return "\n".join(o)



# ------------------------------------------------------- the showpiece view

ISO_COS, ISO_SIN = 0.866, 0.5


def exploded_svg(scale=2.5, spread=150):
    """An exploded isometric of the whole build, one plate per layer.

    The layered preview stacks everything in one plane, which is right for
    working from and muddy to look at. This pulls the same geometry apart
    along the vertical so each layer can be seen whole: cell underneath,
    copper, board, wires, parts, display on its standoffs.
    """
    plates = ["cell", "copper", "board", "wires", "parts", "display"]
    OX = 330.0
    # the stack grows upward from the bottom plate, so the origin sits low
    OY = 120.0 + spread * (len(plates) - 1)
    W = 1000
    H = int(OY + (BOARD_W + BOARD_H) * ISO_SIN * scale + 110)
    o = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" '
         f'viewBox="0 0 {W} {H}" font-family="monospace">',
         f'<defs><linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">'
         f'<stop offset="0" stop-color="#11161d"/>'
         f'<stop offset="1" stop-color="#05070a"/></linearGradient></defs>',
         f'<rect width="{W}" height="{H}" fill="url(#bg)"/>']

    def iso(bx, by, lift):
        """board mm -> screen px, lifted by a plate's explode height"""
        return (OX + (bx - by) * ISO_COS * scale,
                OY + (bx + by) * ISO_SIN * scale - lift)

    def poly(pts, fill, stroke=None, sw=1.0, op=1.0):
        d = " ".join(f"{x:.1f},{y:.1f}" for x, y in pts)
        st = f' stroke="{stroke}" stroke-width="{sw}"' if stroke else ""
        o.append(f'<polygon points="{d}" fill="{fill}" fill-opacity="{op}"{st}/>')

    def slab(x, y, w, h, lift, top, side, thick=6, op=1.0):
        """a rectangle in board space, drawn as a solid with thickness"""
        a, b = iso(x, y, lift), iso(x + w, y, lift)
        c, d = iso(x + w, y + h, lift), iso(x, y + h, lift)
        poly([(b[0], b[1] + thick), (c[0], c[1] + thick), c, b], side, op=op)
        poly([(c[0], c[1] + thick), (d[0], d[1] + thick), d, c], side, op=op)
        poly([a, b, c, d], top, "#0d1117", 0.8, op)
        return a, b, c, d

    def line(p, q, color, sw=2.0, op=1.0):
        o.append(f'<line x1="{p[0]:.1f}" y1="{p[1]:.1f}" x2="{q[0]:.1f}" '
                 f'y2="{q[1]:.1f}" stroke="{color}" stroke-width="{sw}" '
                 f'stroke-opacity="{op}" stroke-linecap="round"/>')

    def label(lift, title, sub):
        x, y = iso(BOARD_W, 0, lift)
        o.append(f'<line x1="{x + 6:.1f}" y1="{y:.1f}" x2="{x + 60:.1f}" '
                 f'y2="{y - 16:.1f}" stroke="#30363d" stroke-width="1"/>')
        o.append(f'<text x="{x + 66:.1f}" y="{y - 12:.1f}" fill="#e6edf3" '
                 f'font-size="14">{title}</text>')
        o.append(f'<text x="{x + 66:.1f}" y="{y + 3:.1f}" fill="#7d8590" '
                 f'font-size="10">{sub}</text>')

    lift = {name: i * spread for i, name in enumerate(plates)}

    # ---- cell, underneath everything -------------------------------------
    bx, by, bw, bh = battery_outline()
    slab(bx, by, bw, bh, lift["cell"], "#1d3b26", "#12251a", 10)
    label(lift["cell"], "LiPo cell", "on the back, foam taped")

    # ---- copper: the strips and their cuts -------------------------------
    z = lift["copper"]
    slab(0, 0, BOARD_W, BOARD_H, z, "#0f1318", "#080b0f", 4, op=0.65)
    for col, r0, r1 in strip_segments():
        c = STRIP_COLORS[(col - 1) % len(STRIP_COLORS)]
        line(iso(*hole(col, r0), z), iso(*hole(col, r1), z), c, 2.2, 0.95)
    for col, row in CUTS:
        x, y = iso(*hole(col, row), z)
        o.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="3.2" fill="none" '
                 f'stroke="#ff4d4d" stroke-width="1.6"/>')
    label(z, "Copper", f"{len(list(strip_segments()))} segments, "
                       f"{len(CUTS)} cuts")

    # ---- the board itself -------------------------------------------------
    z = lift["board"]
    slab(0, 0, BOARD_W, BOARD_H, z, "#2a2416", "#1a1610", 5)
    for col in range(1, COLS + 1):
        for row in range(1, ROWS + 1):
            x, y = iso(*hole(col, row), z)
            o.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="0.9" '
                     f'fill="#0d1117" fill-opacity="0.8"/>')
    label(z, "Stripboard", f"{COLS} x {ROWS} holes, {BOARD_W:.0f} x "
                           f"{BOARD_H:.0f} mm")

    # ---- fly wires --------------------------------------------------------
    z = lift["wires"]
    a, b, c, d = slab(0, 0, BOARD_W, BOARD_H, z, "#12161c", "#0b0e12", 3, op=0.35)
    poly([a, b, c, d], "none", "#2f3844", 1.0)
    for name, pts in JUMPERS:
        c = wire_color(name)[0]
        for a, b in zip(pts, pts[1:]):
            line(iso(*hole(*a), z), iso(*hole(*b), z), c, 2.2)
        for p in pts:
            x, y = iso(*hole(*p), z)
            o.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="2.4" fill="{c}"/>')
    label(z, "Fly wires", f"{len(JUMPERS)} runs, 354 mm of wire")

    # ---- parts -------------------------------------------------------------
    z = lift["parts"]
    a, b, c, d = slab(0, 0, BOARD_W, BOARD_H, z, "#12161c", "#0b0e12", 3, op=0.28)
    poly([a, b, c, d], "none", "#2f3844", 1.0)
    # the Pico
    pb = part_body("U1")
    slab(pb[0], pb[1], pb[2], pb[3], z, "#1b2b3d", "#101b27", 9)
    px_, py_ = iso(pb[0] + pb[2] / 2, pb[1] + pb[3] / 2, z)
    o.append(f'<text x="{px_:.1f}" y="{py_:.1f}" fill="#9fd0ff" font-size="10" '
             f'text-anchor="middle">Pico 2</text>')
    # the charger
    cb = charger_outline()
    slab(cb[0], cb[1], cb[2], cb[3], z, "#4a3115", "#2e1e0d", 8)
    cx_, cy_ = iso(cb[0] + cb[2] / 2, cb[1] + cb[3] / 2, z)
    o.append(f'<text x="{cx_:.1f}" y="{cy_:.1f}" fill="#ffb877" font-size="9" '
             f'text-anchor="middle">TP4056</text>')
    # switches, with caps
    for cross, (a, b) in CROSSES.items():
        for cm, (col, row) in cross_switches(a, b).items():
            hx, hy = hole(col + 1.5, row + 1)
            slab(hx - 3, hy - 3, 6, 6, z, "#4a3f1c", "#2c2511", 5)
            cx2, cy2 = iso(hx, hy, z + 6)
            o.append(f'<ellipse cx="{cx2:.1f}" cy="{cy2:.1f}" '
                     f'rx="{CAP_DIA_MM / 2 * scale * 1.15:.1f}" '
                     f'ry="{CAP_DIA_MM / 2 * scale * 0.62:.1f}" fill="#d29922" '
                     f'stroke="#8a6410" stroke-width="1"/>')
    # headers and passives
    for ref in ("J1", "BT1", "J2", "J3", "SW17", "D1", "R1", "R2", "C1"):
        bod = part_body(ref)
        if bod:
            slab(bod[0], bod[1], bod[2], bod[3], z, "#3a4048", "#23272c", 4)
    label(z, "Components", "Pico, 16 switches, charger, power train")

    # ---- display -----------------------------------------------------------
    z = lift["display"]
    dx, dy, dw, dh = display_outline()
    slab(dx, dy, dw, dh, z, "#12233a", "#0b1626", 10)
    ax, ay = iso(dx + dw / 2, dy + dh / 2, z)
    o.append(f'<text x="{ax:.1f}" y="{ay:.1f}" fill="#79c0ff" font-size="12" '
             f'text-anchor="middle">ST7796 320x480</text>')
    for c, r in STANDOFF_HOLES:
        sx0, sy0 = iso(*hole(c, r), lift["parts"])
        sx1, sy1 = iso(*hole(c, r), z)
        line((sx0, sy0), (sx1, sy1), "#a371f7", 2.0, 0.7)
    label(z, "Display", "on four adhesive standoffs")

    o.append(f'<text x="34" y="46" fill="#e6edf3" font-size="22">'
             f'mega blastoise</text>')
    o.append(f'<text x="34" y="68" fill="#7d8590" font-size="12">'
             f'single-screen stripboard, exploded</text>')
    o.append(f'<text x="34" y="{H - 26}" fill="#4d5560" font-size="10">'
             f'{COLS}x{ROWS} stripboard . 16 buttons, no matrix . '
             f'{len(CUTS)} cuts . {len(JUMPERS)} wires</text>')
    o.append("</svg>")
    return "\n".join(o)


# Real heights above the component-side surface, in mm, so the assembled view
# stands up at the proportions the parts actually have.
BOARD_T_MM = 1.6
FOAM_MM = 1.5
CELL_T_MM = 10.0                  # the 10 in "103450"
HEADER_MM = 2.5                   # the Pico rides on its 2x20 header
PICO_T_MM = 1.0
SWITCH_T_MM = 4.3                 # bare tactile switch body
CAP_T_MM = 4.2                    # the cap that slips over its plunger
CHARGER_T_MM = 5.0                # PCB plus the USB-C shell
DISPLAY_T_MM = 4.0


def assembled_svg(scale=4.0, inset=1.32):
    """The board as it will look built, drawn at true relative heights.

    The exploded view answers "what goes where"; this one answers "what will
    I be holding". Everything stands at its real height above the board and
    near parts are painted over far ones, which means the Pico and most of the
    wiring end up where they really are: hidden under the display deck. Two
    small views make up for that, one with the module unplugged and one from
    underneath.
    """
    W, H = 1240, 1020
    o = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" '
         f'viewBox="0 0 {W} {H}" font-family="monospace">',
         '<defs><linearGradient id="bg2" x1="0" y1="0" x2="0.4" y2="1">'
         '<stop offset="0" stop-color="#131a24"/>'
         '<stop offset="1" stop-color="#05070a"/></linearGradient>'
         '<linearGradient id="glass" x1="0" y1="0" x2="1" y2="1">'
         '<stop offset="0" stop-color="#1d4f7d"/>'
         '<stop offset="0.45" stop-color="#123a5c"/>'
         '<stop offset="1" stop-color="#0d2438"/></linearGradient>'
         '<radialGradient id="pool" cx="0.5" cy="0.5" r="0.5">'
         '<stop offset="0" stop-color="#000000" stop-opacity="0.6"/>'
         '<stop offset="1" stop-color="#000000" stop-opacity="0"/>'
         '</radialGradient></defs>',
         f'<rect width="{W}" height="{H}" fill="url(#bg2)"/>']

    def esc(t):
        return (str(t).replace("&", "&amp;").replace("<", "&lt;")
                .replace(">", "&gt;"))

    class View:
        """One isometric projection: an origin, a scale and an optional flip."""

        def __init__(self, ox, oy, sc, flip=False):
            self.ox, self.oy, self.sc, self.flip = ox, oy, sc, flip

        def at(self, bx, by, mm=0.0):
            if self.flip:
                bx = BOARD_W - bx
            return (self.ox + (bx - by) * ISO_COS * self.sc,
                    self.oy + (bx + by) * ISO_SIN * self.sc - mm * self.sc)

        def depth(self, x, y, w, h):
            """How far toward the viewer the footprint's nearest corner sits."""
            xs = (BOARD_W - x - w, BOARD_W - x) if self.flip else (x, x + w)
            return max(xs) + y + h

    def poly(pts, fill, stroke=None, sw=1.0, op=1.0):
        d = " ".join(f"{x:.1f},{y:.1f}" for x, y in pts)
        st = f' stroke="{stroke}" stroke-width="{sw}"' if stroke else ""
        o.append(f'<polygon points="{d}" fill="{fill}" fill-opacity="{op}"{st}/>')

    def box(v, x, y, w, h, z0, z1, top, side, dark=None, op=1.0, edge="#0a0d12"):
        """A rectangular solid standing between two heights."""
        dark = dark or side
        lo = [v.at(x, y, z0), v.at(x + w, y, z0),
              v.at(x + w, y + h, z0), v.at(x, y + h, z0)]
        hi = [v.at(x, y, z1), v.at(x + w, y, z1),
              v.at(x + w, y + h, z1), v.at(x, y + h, z1)]
        poly([hi[1], lo[1], lo[2], hi[2]], side, op=op)
        poly([hi[2], lo[2], lo[3], hi[3]], dark, op=op)
        poly(hi, top, edge, 0.8, op)
        return hi

    def text(x, y, s, fill, size, anchor="start", weight=None, op=1.0):
        wt = f' font-weight="{weight}"' if weight else ""
        o.append(f'<text x="{x:.1f}" y="{y:.1f}" fill="{fill}" '
                 f'font-size="{size}" text-anchor="{anchor}" '
                 f'fill-opacity="{op}"{wt}>{esc(s)}</text>')

    def leader(v, bx, by, mm, dx, dy, title, sub=None):
        x, y = v.at(bx, by, mm)
        o.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="2.6" fill="none" '
                 f'stroke="#8b949e" stroke-width="1.2"/>')
        run = 26 if dx > 0 else -26
        o.append(f'<path d="M{x + (3 if dx > 0 else -3):.1f},{y:.1f} '
                 f'L{x + dx:.1f},{y + dy:.1f} L{x + dx + run:.1f},{y + dy:.1f}" '
                 f'fill="none" stroke="#3a434f" stroke-width="1"/>')
        tx = x + dx + (32 if dx > 0 else -32)
        anc = "start" if dx > 0 else "end"
        text(tx, y + dy - (2 if sub else -4), title, "#e6edf3", 12, anc)
        if sub:
            text(tx, y + dy + 12, sub, "#7d8590", 9.5, anc)

    def shadow(v, pad=14):
        cx, cy = v.at(BOARD_W / 2, BOARD_H / 2, 0)
        rx = (BOARD_W + BOARD_H) * ISO_COS * v.sc / 2 + pad
        ry = (BOARD_W + BOARD_H) * ISO_SIN * v.sc / 2 + pad
        o.append(f'<ellipse cx="{cx:.1f}" cy="{cy + 14:.1f}" rx="{rx:.1f}" '
                 f'ry="{ry:.1f}" fill="url(#pool)"/>')

    pb = part_body("U1")
    cb = charger_outline()
    dx, dy, dw, dh = display_outline()

    def draw_stack(v, with_display=True, holes=True):
        """Board, wires and parts in one view, painted far corner first."""
        shadow(v, 14 if v.sc > 2 else 6)
        box(v, 0, 0, BOARD_W, BOARD_H, -BOARD_T_MM, 0.0,
            "#3d3419", "#241e0e", "#1a150a")
        if holes:
            r = max(0.6, 0.55 * v.sc / 4.0)
            for col in range(1, COLS + 1):
                for row in range(1, ROWS + 1):
                    hx, hy = v.at(*hole(col, row), 0.0)
                    o.append(f'<circle cx="{hx:.1f}" cy="{hy:.1f}" r="{r:.1f}" '
                             f'fill="#15110a" fill-opacity="0.85"/>')
        for name, pts in JUMPERS:
            col = wire_color(name)[0]
            for a, b in zip(pts, pts[1:]):
                p, q = v.at(*hole(*a), 0.6), v.at(*hole(*b), 0.6)
                o.append(f'<line x1="{p[0]:.1f}" y1="{p[1]:.1f}" '
                         f'x2="{q[0]:.1f}" y2="{q[1]:.1f}" stroke="#05070a" '
                         f'stroke-width="{1.2 * v.sc / 4 + 2.6:.1f}" '
                         f'stroke-linecap="round" stroke-opacity="0.55"/>')
                o.append(f'<line x1="{p[0]:.1f}" y1="{p[1]:.1f}" '
                         f'x2="{q[0]:.1f}" y2="{q[1]:.1f}" stroke="{col}" '
                         f'stroke-width="{0.7 * v.sc / 4 + 1.5:.1f}" '
                         f'stroke-linecap="round"/>')

        parts = []

        def add(x, y, w, h, fn):
            parts.append((v.depth(x, y, w, h), fn))

        def draw_pico():
            box(v, pb[0], pb[1], pb[2], pb[3], 0.0, HEADER_MM,
                "#161b22", "#0e1218", "#090c10")
            box(v, pb[0], pb[1], pb[2], pb[3], HEADER_MM, HEADER_MM + PICO_T_MM,
                "#1f6f4a", "#155037", "#0f3826")
            zt = HEADER_MM + PICO_T_MM
            box(v, pb[0] + pb[2] / 2 - 3.5, pb[1] + pb[3] / 2 - 3.5, 7, 7,
                zt, zt + 1.0, "#0d1117", "#080b0f")
            box(v, pb[0] + 1.5, pb[1] - 1.0, 8.0, 7.5, zt, zt + 2.9,
                "#adb6c0", "#6b7480", "#4c545e")
            cx, cy = v.at(pb[0] + pb[2] * 0.62, pb[1] + pb[3] * 0.78, zt)
            if v.sc > 2:
                text(cx, cy, "Pico 2", "#7ee2b8", 9, "middle")
        add(pb[0], pb[1], pb[2], pb[3], draw_pico)

        def draw_charger():
            box(v, cb[0], cb[1], cb[2], cb[3], FOAM_MM, FOAM_MM + 1.6,
                "#8d2c1b", "#521810", "#3a110b")
            box(v, cb[0] + cb[2] - 9.0, cb[1] + cb[3] / 2 - 4.5, 9.0, 9.0,
                FOAM_MM + 1.6, FOAM_MM + CHARGER_T_MM,
                "#d3dae1", "#8b949e", "#6b7480")
            cx, cy = v.at(cb[0] + cb[2] * 0.3, cb[1] + cb[3] / 2, FOAM_MM + 1.6)
            if v.sc > 2:
                text(cx, cy, "TP4056", "#ffb877", 8, "middle")
        add(cb[0], cb[1], cb[2], cb[3], draw_charger)

        for _, (a, b) in CROSSES.items():
            for _, (col, row) in cross_switches(a, b).items():
                hx, hy = hole(col + 1.5, row + 1)

                def draw_sw(hx=hx, hy=hy):
                    s = SWITCH_BODY_MM / 2
                    box(v, hx - s, hy - s, SWITCH_BODY_MM, SWITCH_BODY_MM,
                        0.0, SWITCH_T_MM, "#2b2f36", "#1a1d22", "#131519")
                    cx, cy = v.at(hx, hy, SWITCH_T_MM + CAP_T_MM)
                    rx = CAP_DIA_MM / 2 * v.sc
                    ry = rx * (ISO_SIN / ISO_COS)
                    wall = CAP_T_MM * v.sc * 0.5
                    o.append(f'<ellipse cx="{cx:.1f}" cy="{cy + wall:.1f}" '
                             f'rx="{rx:.1f}" ry="{ry:.1f}" fill="#7a5a0d"/>')
                    o.append(f'<rect x="{cx - rx:.1f}" y="{cy:.1f}" '
                             f'width="{2 * rx:.1f}" height="{wall:.1f}" '
                             f'fill="#7a5a0d"/>')
                    o.append(f'<ellipse cx="{cx:.1f}" cy="{cy:.1f}" '
                             f'rx="{rx:.1f}" ry="{ry:.1f}" fill="#e3a91f" '
                             f'stroke="#8a6410" stroke-width="1"/>')
                    o.append(f'<ellipse cx="{cx:.1f}" cy="{cy - ry * 0.22:.1f}" '
                             f'rx="{rx * 0.6:.1f}" ry="{ry * 0.6:.1f}" '
                             f'fill="#f2c452" fill-opacity="0.5"/>')
                add(hx - 4.5, hy - 4.5, 9, 9, draw_sw)

        for ref, tall, top, side in (("J1", 8.5, "#2a2f37", "#171a20"),
                                     ("BT1", 8.5, "#2a2f37", "#171a20"),
                                     ("J2", 8.5, "#2a2f37", "#171a20"),
                                     ("J3", 8.5, "#2a2f37", "#171a20"),
                                     ("SW17", 8.5, "#2a2f37", "#171a20"),
                                     ("D1", 2.2, "#33383f", "#1d2126"),
                                     ("R1", 2.0, "#33383f", "#1d2126"),
                                     ("R2", 2.0, "#33383f", "#1d2126"),
                                     ("C1", 2.6, "#414852", "#262b31")):
            bod = part_body(ref)
            if not bod:
                continue

            def draw_part(bod=bod, tall=tall, top=top, side=side):
                box(v, bod[0], bod[1], bod[2], bod[3], 0.0, tall, top, side)
            add(bod[0], bod[1], bod[2], bod[3], draw_part)

        def draw_standoffs():
            for c, r in STANDOFF_HOLES:
                hx, hy = hole(c, r)
                box(v, hx - 2.2, hy - 2.2, 4.4, 4.4, 0.0, DECK_HEIGHT_MM,
                    "#8a72d6", "#5c48a0", "#3f3172")
        if with_display:
            def draw_display():
                draw_standoffs()
                z0 = DECK_HEIGHT_MM
                box(v, dx, dy, dw, dh, z0, z0 + 1.6,
                    "#1a5c3f", "#0e3626", "#0a2419")
                box(v, dx, dy, dw, dh, z0 + 1.6, z0 + DISPLAY_T_MM,
                    "#15191f", "#0d1015", "#080a0d", edge="#2a3038")
                aw, ah = DISPLAY_ACTIVE_MM
                box(v, dx + (dw - aw) / 2, dy + 6.0, aw, ah,
                    z0 + DISPLAY_T_MM, z0 + DISPLAY_T_MM + 0.2,
                    "#14395c", "#0d2338", "#0a1b2b", edge="#2f6491")
                if v.sc > 2:
                    ax, ay = v.at(dx + dw / 2, dy + 6.0 + ah / 2,
                                  z0 + DISPLAY_T_MM)
                    text(ax, ay - 4, "ST7796", "#79c0ff", 13, "middle")
                    text(ax, ay + 11, "320 x 480", "#4d7ea8", 9, "middle")
            add(dx, dy, dw, dh, draw_display)
        else:
            for c, r in STANDOFF_HOLES:
                hx, hy = hole(c, r)
                add(hx - 2.2, hy - 2.2, 4.4, 4.4,
                    lambda hx=hx, hy=hy: box(v, hx - 2.2, hy - 2.2, 4.4, 4.4,
                                             0.0, DECK_HEIGHT_MM, "#8a72d6",
                                             "#5c48a0", "#3f3172"))

        for _, fn in sorted(parts, key=lambda p: p[0]):
            fn()

    def caption(v, title, sub):
        cx, cy = v.at(BOARD_W / 2, 0, 0)
        text(cx, cy - 44, title, "#e6edf3", 13, "middle")
        text(cx, cy - 30, sub, "#7d8590", 9.5, "middle")

    # ------------------------------------------------------------ main view
    main = View(585, 258, scale)
    draw_stack(main, with_display=True)
    leader(main, dx + dw, dy + 2, DECK_HEIGHT_MM + DISPLAY_T_MM, 56, -74,
           "ST7796 3.5 in", f"plugged into J1, {DECK_HEIGHT_MM:.1f} mm deck")
    a, b = CROSSES["P1_DPAD"]
    n = cross_switches(a, b)["N"]
    leader(main, *hole(n[0] + 1.5, n[1] + 1), SWITCH_T_MM + CAP_T_MM, 26, 214,
           "16 caps, 9 mm", "one GPIO each, no matrix")
    leader(main, cb[0] + cb[2], cb[1] + cb[3] / 2, FOAM_MM + CHARGER_T_MM,
           62, 20, "TP4056 USB-C", "overhangs the edge for a plug")
    leader(main, STANDOFF_HOLES[3][0] and hole(*STANDOFF_HOLES[3])[0],
           hole(*STANDOFF_HOLES[3])[1], DECK_HEIGHT_MM * 0.55, 170, 104,
           "nylon standoff", "8 mm, self-adhesive, four of them")

    # ------------------------------------------------- module off, from below
    deck = View(1010, 560, inset)
    draw_stack(deck, with_display=False)
    caption(deck, "module unplugged",
            "the Pico and the wiring live under the deck")

    under = View(1010, 862, inset, flip=True)
    shadow(under, 6)
    box(under, 0, 0, BOARD_W, BOARD_H, 0.0, BOARD_T_MM,
        "#4a3f1e", "#2a2412", "#1d1a0d")
    for col, r0, r1 in strip_segments():
        c = STRIP_COLORS[(col - 1) % len(STRIP_COLORS)]
        p = under.at(*hole(col, r0), BOARD_T_MM)
        q = under.at(*hole(col, r1), BOARD_T_MM)
        o.append(f'<line x1="{p[0]:.1f}" y1="{p[1]:.1f}" x2="{q[0]:.1f}" '
                 f'y2="{q[1]:.1f}" stroke="{c}" stroke-width="1.7" '
                 f'stroke-linecap="round"/>')
    for col, row in CUTS:
        hx, hy = under.at(*hole(col, row), BOARD_T_MM)
        o.append(f'<circle cx="{hx:.1f}" cy="{hy:.1f}" r="2.1" fill="#0a0d12" '
                 f'stroke="#ff4d4d" stroke-width="1.2"/>')
    bx, by, bw, bh = battery_outline()
    box(under, bx, by, bw, bh, BOARD_T_MM + FOAM_MM,
        BOARD_T_MM + FOAM_MM + CELL_T_MM, "#1f4029", "#12251a", "#0c180f")
    cx, cy = under.at(bx + bw / 2, by + bh / 2,
                      BOARD_T_MM + FOAM_MM + CELL_T_MM)
    text(cx, cy, "2000 mAh", "#7ee2b8", 9, "middle")
    caption(under, "underside", f"{len(CUTS)} cuts, cell on foam tape")

    # ---------------------------------------------------------------- chrome
    text(40, 54, "mega blastoise", "#e6edf3", 25)
    text(40, 76, "single-screen stripboard, assembled", "#7d8590", 12)
    facts = [(f"{COLS} x {ROWS}", "holes, 61 x 142 mm"),
             ("16", "buttons, one GPIO each"),
             (f"{len(CUTS)}", "track cuts"),
             (f"{len(JUMPERS)}", "fly wires, 354 mm"),
             ("8.5", "mm from board to module")]
    fy = H - 170
    o.append(f'<line x1="40" y1="{fy - 28}" x2="300" y2="{fy - 28}" '
             f'stroke="#242b34" stroke-width="1"/>')
    for i, (big, small) in enumerate(facts):
        text(40, fy + i * 28, big, "#e6edf3", 15, weight="bold")
        text(136, fy + i * 28, small, "#7d8590", 10)
    o.append("</svg>")
    return "\n".join(o)


def main():
    (HERE / "BUILD.md").write_text(build_md())
    (HERE / "ASSEMBLY.md").write_text(assembly_md())
    for stem, names in STAGE_VIEWS.items():
        (HERE / f"{stem}.svg").write_text(stage_svg(names))
    (HERE / "power_diagram.svg").write_text(power_diagram_svg())
    (HERE / "exploded.svg").write_text(exploded_svg())
    (HERE / "assembled.svg").write_text(assembled_svg())
    (HERE / "preview.svg").write_text(svg())
    (HERE / "BOM.csv").write_text(bom_csv())
    (HERE / "cut_template.svg").write_text(cut_template_svg())
    MODEL = HERE / "model"
    MODEL.mkdir(exist_ok=True)
    (MODEL / "cuts_stencil.scad").write_text(stencil_scad())
    (MODEL / "assembled.scad").write_text(assembled_scad())
    (MODEL / "button_cap.scad").write_text(button_cap_scad())
    json.dump(kicad_data(), sys.stdout, indent=1)


if __name__ == "__main__":
    main()
