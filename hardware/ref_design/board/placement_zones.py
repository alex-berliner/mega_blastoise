"""Zones and movable groups for the placement search (data for tools/place_search.py); placement.py's own table is every
group's home. Two stages: (1) macro: each group goes to one of its allowed zones, placed in it by a simple packing
(centre first, then the nearest legal spot); (2) nudges: small shifts and quarter turns per group and per part.

The search writes its result as board/picks.json, which placement.py applies on top of its table (explicit poses; the core's
centre and turn; the cell hole's shift); without that file, or PLACE_PICKS (the same JSON), nothing moves:

    {"poses": {ref: [x, y, rot, side]}, "core": [x, y, rot], "hole": [dx, dy], "usb_dy": mm}

KiCad frame (x right, y down, mm); the case frame is x - 100, 100 - y. The board is 72.4 x 151.5, the cell hole 37.8 x 54.8
at x 81.1 .. 118.9, y 53 .. 107.8, J3 (the panel) at y 114.4 .. 122.5 below it.

FIXED (the face and the anchor): the 20 face buttons and their domes, the panel's J3, the case outline, J1 on the RIGHT
wall (it may slide along it, usb_dy, with U1/R1-R4 and the case's opening), the screws H1-H6 (the case's bosses).
"""
import json
import math
import os

HERE = os.path.dirname(os.path.abspath(__file__))

ZONES = {   # name: (x0, y0, x1, y1)
    "end2_l": (64.5, 26.0, 88.0, 52.0), "end2_m": (88.0, 26.0, 112.0, 52.0), "end2_r": (112.0, 26.0, 135.5, 52.0),
    "ls_top": (64.5, 55.0, 80.6, 72.0), "ls_mid": (64.5, 72.0, 80.6, 90.0), "ls_bot": (64.5, 90.0, 80.6, 108.0),
    "rs_top": (119.4, 55.0, 135.6, 72.0), "rs_mid": (119.4, 72.0, 135.6, 90.0), "rs_bot": (119.4, 90.0, 135.6, 108.0),
    "gap": (64.5, 108.0, 135.6, 114.5),
    "low_l": (64.5, 108.0, 86.0, 140.0), "low_r": (114.0, 108.0, 135.6, 140.0),
    # below J3 down to seat 1's Start/Select (was low_m and end1_m)
    "mid_low": (86.0, 123.0, 114.0, 175.0),
    "end1_l": (64.5, 140.0, 88.0, 175.0), "end1_r": (112.0, 140.0, 135.5, 175.0),
}
STRIPS = ["ls_top", "ls_mid", "ls_bot", "rs_top", "rs_mid", "rs_bot"]
RIGHT = ["rs_top", "rs_mid", "rs_bot", "low_r", "end1_r", "end2_r"]
LEFT = ["ls_top", "ls_mid", "ls_bot", "low_l", "end1_l", "end2_l"]
BANDS = ["end2_l", "end2_m", "end2_r", "end1_l", "mid_low", "end1_r", "low_l", "low_r", "gap"]

# Zones are not walls: a group's anchor may sit up to SLACK mm past a zone's edge (about half a small group) and its parts spill
# over; the anchors tried cover the whole zone on a 2 mm grid, nearest the zone's centre first.
SLACK = 3.0
ANCHOR_STEP = 2.0

# the core's loose parts (placed by hand in placement.py); U4 and the reference's own parts are rigid with it
CORE_LOOSE = ["R23", "C13", "C23", "C18", "C14", "C15", "C17", "C22", "C16", "C20", "C19", "R65"]

GROUPS = {
    # kind "rigid": the RP2350B (U4, X1, L2, decoupling copied from Raspberry Pi's layout) moves and turns as one, the loose caps are packed round it
    "core": dict(kind="core", refs=CORE_LOOSE, zones=RIGHT[:4] + ["low_l", "ls_bot", "ls_mid", "ls_top", "mid_low", "end1_r", "end1_l"],
                 turns=[0, 90, 180, 270]),
    # J1 stays on the right wall, sliding along it (dy, mm); its ESD array and the CC resistors slide with it
    "usb": dict(kind="slide", refs=["J1", "U1", "C1", "R41", "R1", "R2", "R3", "R4"], dys=[-6.0, -3.0, 0.0, 3.0]),
    "charger": dict(kind="packed", refs=["U2", "C3", "C2", "R17", "R18", "C8", "R5", "R9", "R10", "R43", "R44", "R64", "TP7", "U10", "C54"],
                    zones=STRIPS + ["low_l", "low_r", "mid_low", "end2_r", "end2_l", "end1_r"], turns=[0, 180]),
    "vsense": dict(kind="packed", refs=["U10", "C54"], override=True, zones=STRIPS + ["low_l", "low_r", "end2_r"], turns=[0, 180]),
    # the power button's latch (PWR_BTN_RAW comes from SW21 at the bottom): D2, R13, R42, Q1, R16
    "latch": dict(kind="packed", refs=["D2", "R13", "R42", "Q1", "R16"], zones=BANDS + STRIPS, turns=[0, 180]),
    "ldo": dict(kind="packed", refs=["U3", "C5", "C6", "C7", "JP1"], zones=STRIPS + ["low_l", "low_r", "end2_l", "end2_r"], turns=[0, 180]),
    "j5": dict(kind="packed", refs=["J5"], zones=LEFT + RIGHT[:3] + ["gap"], turns=[0, 90, 180, 270]),
    # BOOT and RESET with their resistors; the case's pinholes follow (the left bezel by default)
    "boot": dict(kind="packed", refs=["SW18", "SW19", "R25", "R26"], zones=["low_l", "ls_bot", "end1_l", "ls_mid", "low_r", "end1_r"], turns=[0, 180]),
    # the speaker's connector (the speaker's 35 mm leads reach it); packed again automatically when J1's slide pushes it
    "j4": dict(kind="packed", refs=["J4"], zones=["low_r", "rs_bot", "gap", "end1_r", "mid_low"], turns=[0, 90, 180, 270]),
    # grey parts the core may need the room of: JP2 (VSYS to the panel's LED_A), C10; evicted to the nearest legal spot when something lands on them
    "misc": dict(kind="packed", refs=["JP2", "C10"], zones=["gap", "mid_low", "low_l", "low_r", "rs_bot", "end1_r"], turns=[0, 90]),
    # the seat-2 Start's row above the hole (Q7, D8 and the parts on PWR_EN): a hole shifted towards seat 2 lands on it
    "row2": dict(kind="packed", refs=["C47", "Q7", "R62", "D8", "R15", "TP6", "R63"], zones=BANDS + STRIPS, turns=[0, 180]),
    "spi_r": dict(kind="packed", refs=["R28", "R29"], zones=["mid_low", "low_l", "gap"], turns=[0, 90]),
    "c2930": dict(kind="packed", refs=["C29", "C30"], zones=["gap", "mid_low", "low_l", "low_r"], turns=[0, 90]),
    # the cell's connector J2 (at the hole's lead end), its thermistor TH1 (against the cell) and the VBAT jumper/test pad: in a strip
    # beside the hole, turned 180 in the left strip so TH1 stays on the hole's side and J2's mouth towards the hole (case: the
    # cell is pushed to that side)
    "cellconn": dict(kind="packed", refs=["J2", "TH1", "JP5", "TP3"],
                     options=[["rs_top", 0], ["rs_mid", 0], ["rs_bot", 0], ["ls_top", 180], ["ls_mid", 180], ["ls_bot", 180]]),
    # the cell hole shifted (case frame mm); what is on the cell's side of it moves with it (cellconn is placed after the shift)
    "hole": dict(kind="hole", refs=[], shifts=[(0, 0), (0, -3), (0, 3), (-3, 0), (3, 0), (0, 6)]),
}
# per-part rules the packing and the validity filter apply: TH1's pads within `gap` of the hole's edge (the cell's temperature),
# J2 within `reach` of the hole's lead end corner (the leads must reach it)
CONSTRAINTS = {"TH1": ("near_hole", 1.6), "J2": ("near_lead_end", 22.0), "J4": ("near_speaker", 28.0)}
SMALL = ["spi_r", "c2930"]

# the speaker, its hole and the amplifier group that sits beside it (U7 and its passives); J4 follows. An option is
# [dx, dy, zone, turn]: the hole and the speaker moved by (dx, dy) KiCad mm (-59.4 mirrors it about the board's centre line,
# same y), the amplifier group packed in `zone` turned `turn`, J4 beside it. The case's grille follows (a case respin).
AUDIO = ["U7", "C34", "C35", "C40", "R36", "R37", "R34", "R35", "C39", "R32", "C37", "R33", "C38", "Q3", "TP8", "JP3", "TP15"]
MIRROR_DX = -59.4
GROUPS["audio"] = dict(kind="packed", refs=AUDIO, zones=["low_l", "low_r"], turns=[0, 180])
SPEAKER_STEP = 4.0           # mm: the speaker's centre on this grid over the whole face (place_search.speaker_options keeps the legal ones)
SPEAKER_HOME = (129.7, 124.5)
SPEAKER_BOX = (11.0, 16.0)   # the hole: the racetrack (10 x 15) plus 0.5 each side; long side along the board (the case's grille is along y)
GROUPS["speaker"] = dict(kind="speaker", refs=AUDIO, options=None)    # options: place_search.speaker_options (a grid, see SPEAKER_STEP)


def load_picks():
    if os.environ.get("PLACE_PICKS"):
        return json.loads(os.environ["PLACE_PICKS"])
    path = os.path.join(HERE, "picks.json")
    return json.load(open(path)) if os.path.exists(path) else {}


def core_pose(home, picks):
    c = picks.get("core")
    return ((c[0], c[1]), c[2]) if c else ((home[0], home[1]), home[2])


def shift_hole(cutouts, picks):
    dx, dy = picks.get("hole", (0, 0))
    out = []
    for c in cutouts:
        if c["name"] == "cell" and (dx or dy):
            x0, y0, x1, y1 = c["box"]
            c = dict(c, box=(x0 + dx, y0 - dy, x1 + dx, y1 - dy))        # the case frame's y is up, KiCad's down
        out.append(c)
    return out


def shift_speaker(cutouts, speaker, picks):
    dx, dy = picks.get("speaker", (0, 0))
    if not (dx or dy):
        return cutouts, speaker
    out = [dict(c, box=(c["box"][0] + dx, c["box"][1] + dy, c["box"][2] + dx, c["box"][3] + dy)) if c["name"] == "speaker" else c for c in cutouts]
    return out, dict(speaker, at=(speaker["at"][0] + dx, speaker["at"][1] + dy))


def apply(P, picks):
    """Put the search's poses into placement.py's table P (in place)."""
    for ref, (x, y, rot, side) in picks.get("poses", {}).items():
        P[ref] = (round(x, 3), round(y, 3), rot, side)
