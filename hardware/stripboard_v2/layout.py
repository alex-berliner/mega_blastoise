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

# The display header pins fall through to the GPIOs directly below them: the
# fence cut is omitted on columns 11-14 and 16, so SCK, MOSI, RST, DC and BL
# ride their own strips down to the Pico. That is what moves the display to
# SPI0 - GP6/GP7 are SPI0's SCK/TX and they sit exactly under J1 pins 3 and 4.
# GND joins the P1 ground bus; only VCC and CS still need a wire.
DISPLAY_HEADER_COL0 = 9       # J1 pin 1; pins 1..8 on columns 9..16
DISPLAY_HEADER_ROW = 46

DISPLAY_NETS = [
    # J1 pin, name,  strip column, rows, Pico pin, how
    (1, "GND",       9,  "46-56", 8,  "direct: shares column 9 with the E-switch ground and the P1 bus"),
    (2, "VCC",       10, "40-56", 36, "jumper (7,20)->(10,45) from 3V3"),
    (3, "LCD_SCK",   11, "35-56", 9,  "direct"),
    (4, "LCD_MOSI",  12, "35-46", 10, "direct"),
    (5, "LCD_RST",   13, "35-46", 11, "direct"),
    (6, "LCD_DC",    14, "35-56", 12, "direct"),
    (7, "LCD_CS",    15, "40-56", 7,  "jumper (15,45)->(9,39)"),
    (8, "LCD_BL",    16, "35-47", 14, "direct"),
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
    + [(9, 45)]
    + [(12, 47), (13, 47)]
    + [(16, 48)]
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
    ("display VCC from 3V3",    [(10, 40), (7, 28)]),
    ("display CS to GP5",       [(15, 40), (9, 40)]),
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

    def px(col, row):
        x, y = hole(col, row)
        return (BOARD_W - x, y)      # mirror horizontally

    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}mm" height="{H + 10}mm" '
           f'viewBox="0 0 {W} {H + 10}" font-family="monospace">',
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
        out.append(f'<text x="{x:.2f}" y="{H + 4}" font-size="3" '
                   f'text-anchor="middle" fill="#000">{col}</text>')
    out.append(f'<text x="2" y="{H + 9}" font-size="3" fill="#000">'
               f'COPPER SIDE - print at 100% scale, cut every X ({len(CUTS)} cuts)</text>')
    out.append("</svg>")
    return "\n".join(out)


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
    A("**SPI0** is the display bus: SCK on GP6 and MOSI (TX) on GP7 are that")
    A("peripheral's hardware pins, chosen because they sit directly under J1")
    A("pins 3 and 4. Buttons are inputs with `Pull::Up`, active low. GP0, GP15")
    A("and GP16 are unused - GP0 is UART0 TX, kept free for a debug console.")
    A("There is no buzzer: this console is silent by design.")
    A("")
    return "\n".join(L)


def main():
    (HERE / "BUILD.md").write_text(build_md())
    (HERE / "preview.svg").write_text(svg())
    (HERE / "BOM.csv").write_text(bom_csv())
    (HERE / "cut_template.svg").write_text(cut_template_svg())
    json.dump(kicad_data(), sys.stdout, indent=1)


if __name__ == "__main__":
    main()
