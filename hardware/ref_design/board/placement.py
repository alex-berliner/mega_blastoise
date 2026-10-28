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
import placement_zones  # noqa: E402  (the search's result, board/picks.json or PLACE_PICKS; a no-op without them)
PICKS = placement_zones.load_picks()

# ---- the board 2 mm wider than the case had it (2026-10-22, Alex: up to 72.4 mm) ----------------
# 1 mm more on each side, still centred on the case, which grows to match: the USB-C and its tab
# go out with the right wall. The cell's hole moved 1 mm left (CUTOUTS), so the left strip keeps
# its 7.5 mm and the right strip beside J1 gains the whole 2 mm (8.9 -> 10.9): the nets out of it
# had only the 2.8 mm between J1's pads and the hole. case/CELLCUT_FOR_CASE.md has the outline.
# The case took it on 2026-10-22 (case.scad writes the 72.4 board, USB-C and tab into
# positions.json), so this adds nothing now; it stays so the placement can't widen it twice
WIDEN = 0.0      # (positions.json has the 72.4 wide board since 2026-10-22; the ESP32-S3 board's left edge is out by GROW_L, below)
GROW_L = CASE["board"].get("grow_l", 0.0)          # the microSD slot's 1.3 mm on the left edge only (case.scad grow_l)
BOARD_W = CASE["board"]["w"] - GROW_L              # the board as it was, centred on x 100
BOARD_X0, BOARD_X1 = 100 - BOARD_W / 2 - GROW_L, 100 + BOARD_W / 2

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
    dict(name="cell", box=(81.1, 52.0, 118.9, 106.8), r=1.0),     # 37.8 x 54.8 along the board (case x -18.9 .. 18.9, y -6.8 .. 48 since the placement search of 2026-10-27, 1 mm towards seat 2 to widen the gap to J3 (case.scad cell_dy), was -7.8 .. 47: 4 mm further towards seat 2 than the first try, which left the MCU-to-right-strip crossing 1.3 mm wide): the worst-case 52 x 36 cell with the pocket insets (V&V cellgap)
    # the speaker, sunk like the cell (2026-10-26): its outline plus 0.5, a stadium (case x 24.2 .. 35.2, y -31 .. -15)
    dict(name="speaker", box=(124.2, 116.5, 135.2, 132.5), r=5.5),
]
CUTOUTS = placement_zones.shift_hole(CUTOUTS, PICKS)

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
SPEAKER = dict(at=(129.7, 124.5), size=(10.0, 15.0), grow=0.5)   # KiCad x, y; x by y, mm (the hole's outline: CUTOUTS)
CUTOUTS, SPEAKER = placement_zones.shift_speaker(CUTOUTS, SPEAKER, PICKS)    # the search may have moved it

# ---- no core any more (2026-10-28): the ESP32-S3-WROOM-1U-N16R8 module is placed like any other part, below ----------------------
# (build_pcb.py's copy_core and its reference board are unused: these names stay so it imports)
CORE_SIDE = "F"
CORE_AT, CORE_ROT, CORE_Y = (0.0, 0.0), 0, 0.0
CORE_ESCAPES = False
CORE = {}
CORE_NETS = {}


def core_pt(dx, dy):
    return (0.0, 0.0)


# ---- everything else (re-placed for the 3.5 in panel, 2026-10-26) ----------------------------------
# Positions below are in the case frame (x right, y up towards seat 2, origin at the centre); `at` turns them into KiCad's.
# Rotations are KiCad's, as before (a rotation of 0 puts a PicoBlade's mouth towards case -y, 180 towards +y).
b = CASE["buttons"]
s = list(CASE["pins"])      # the screws: the four side ones at the glass's corners, the two end ones (case.scad)
P = {}                       # ref: (x, y, rot, side)   side "F" or "B"


def put(ref, x, y, rot=0, side="F"):
    P[ref] = (round(x, 3), round(y, 3), rot, side)


def at(ref, x, y, rot=0, side="F"):
    put(ref, 100 + x, 100 - y, rot, side)


for i in range(16):
    put(f"SW{i + 1}", *b[f"SW{i + 1}"])
for ref, xy in CASE["ss_buttons"].items():  # Start/Select, between each seat's clusters at the case's end
    put(ref, *xy)
for i, (x, y) in enumerate(s):
    put(f"H{i + 1}", x, y)
at("FID1", -34.0, -72.5)
at("FID2", 34.0, -72.5)
at("FID3", -34.0, 72.5)

# ---- display: J3 where the folded FPC reaches (case.scad conn_in), its mouth towards seat 1 ----------------
put("J3", CASE["fpc_conn"]["x"], CASE["fpc_conn"]["y"], 0)
at("C29", -10.6, -13.0, 90)                  # panel VCI/VDDI, over J3's 3V3 pins
at("C30", -9.4, -13.0, 90)
at("JP4", -10.0, -25.4, 0)                   # 3V3 -> panel: flat, under the tail
at("JP2", 26.9, -13.0, 0)                    # VSYS to LED_A (J3 pin 50): flat, by J3's right end

# ---- backlight current sinks: the ends' spine (U6, 1.7 tall in its pocket) and the row under the slope above it
at("U6", -3.2, -55.0, 90)
at("Q6", -7.5, -46.0, 0)
at("R53", -11.5, -46.0, 90)
at("R57", -15.0, -44.4, 0)
at("R55", -15.0, -45.7, 0)
at("C49", -15.0, -47.0, 0)
at("R59", -17.2, -44.4, 0)
at("R60", -17.2, -45.7, 0)
at("C50", -17.2, -47.0, 0)
at("R61", -19.4, -45.7, 0)
at("TP10", -19.4, -44.0, 0)
at("Q5", 7.5, -46.0, 0)
at("R52", 11.5, -46.0, 90)
at("R56", 15.0, -44.4, 0)
at("R54", 15.0, -45.7, 0)
at("C48", 15.0, -47.0, 0)
at("R58", 17.2, -44.4, 0)
at("C51", 17.2, -45.7, 0)
at("TP16", 19.4, -44.0, 0)

# ---- seat 1's buttons: U8 beside U6 in the spine
at("U8", 3.2, -56.0, 90)
at("RN1", -23.5, -45.8, 90)
at("RN2", -26.5, -45.8, 90)
at("C52", -9.0, -51.0, 0)

# ---- the module (2026-10-28): its U.FL side towards the left wall, the antenna's flex plate on the wall above it (x 64.5 .. 71, y 62.1 .. 107.9)
# the footprint's natural pose has the antenna end up; turned 90 degrees CCW it is on the left. Body x 64.8 .. 84.0, y 107.8 .. 125.8
# (0.7 up from the mock's spot: the slot's top pads keep 0.6 from its pads)
put("U4", 73.9, 116.8, 90)
# the antenna's copper-free zone (both layers, pours too): build_pcb.py's antenna_keepout (the flex antenna's footprint, a margin of 0.4
# in x, and none below it so the module's top row of pads and their escapes keep their lane): not a Kinghelm number, see docs
ANTENNA_KEEPOUT = (BOARD_X0 - 0.5, 62.6, 71.4, 107.0)   # (H2's boss, y 57.5 .. 62.5, already touches the mock's plate: 0.4 mm of its end)
put("C11", 63.4, 125.6, 90)                  # 3V3 at the module's pin 2, in the strip beside it
put("C12", 63.4, 124.1, 90)                  # EN's 1 uF
put("R23", 63.4, 122.6, 90)                  # EN's 10k
put("C10", 79.6, 104.9, 0)                   # the module's 22 uF, above its top row
put("R22", 80.8, 128.6, 90)                  # USB 22 ohm, by the module's D- / D+ pins
put("R21", 82.0, 128.6, 90)
put("C13", 80.8, 130.4, 90)                  # DNP pads
put("C14", 82.0, 130.4, 90)
# 33 ohm source terminations: D0-D4 above the module's top row, D5/D6 one row up, D7 and WR on the module's right side
for k, x in enumerate((76.9, 75.6, 74.3, 73.1, 71.8)):
    put(f"R{70 + k}", x, 105.5, 90)
put("R75", 72.2, 103.9, 90)
put("R76", 73.5, 103.9, 90)
put("R77", 85.4, 112.36, 0)
put("R78", 85.4, 111.1, 0)
put("R65", 85.4, 109.8, 0)                   # SR_CLK source termination (module pin 18's neighbour on the right side)

# ---- the microSD slot (Hirose DM3AT-SF-PEJM5, C114218) below the module, its mouth to the left wall: the case's opening is y 127.2 .. 141.2
# (case.scad sd_y), the slot's front 0.45 past the board's edge (card tip 0.4 short of the outside), its back at x 77.95 (the panel's FPC zone
# starts at 78.05: 1.05 mm there, the slot is 1.68). Turned 270 so its mouth (+y of the footprint) is -x
put("J8", 70.4, 134.6, 270)
for k in range(6):                           # pull-ups: CMD, D0-D3, CD
    put(f"R{79 + k}", 80.6, 129.0 + 1.4 * k, 90)
put("JP6", 83.0, 138.0, 0)
put("C60", 83.0, 145.0, 0)
put("C61", 86.0, 145.0, 0)

# ---- the third shift register (Starts, Selects, charger status), in seat 1's spine
put("U11", 101.0, 147.2, 0)
put("C62", 106.5, 147.2, 0)
put("R86", 96.3, 146.0, 90)
put("R87", 96.3, 147.6, 90)

# ---- USB-C on the right wall, its ESD array and the CC resistors
USB_Y = CASE["usb"]["y"]
USB_DY = PICKS.get("usb_dy", 0.0)       # J1 stays on the right wall; the search may slide it along it (the case's usb_y follows)
USB_TAB = [[x, y + USB_DY] for x, y in CASE["usb"]["tab"]]
usb_x = CASE["usb"]["front_x"] - 3.65       # footprint front is 3.65 ahead of its origin
put("J1", usb_x, USB_Y, 90)
at("U1", 24.5, 1.5, 0)
at("C1", 20.8, 4.6, 0)
at("R41", 20.8, 6.2, 0)
at("R1", 29.0, 8.0, 0)                       # CC resistors above J1, beside the hole's strip
at("R2", 31.2, 8.0, 0)
at("R3", 29.0, 9.2, 0)
at("R4", 31.2, 9.2, 0)
at("C42", 25.0, -45.5, 0)                    # CC1/CC2 spares, DNP
at("C43", 30.5, -45.5, 0)

# ---- the cell: J2 at the hole's seat-1 corner (mouth into it), the charger, the load sharing
at("J2", 31.55, 30.0, 180)
at("JP5", 24.0, 29.0, 0)                     # cell -> VBAT
at("U2", 25.0, 13.0, 0)                      # BQ24073: its thermal pad spreads into the GND pours
at("C3", 20.4, 16.6, 90)                      # VBAT
at("C2", 20.4, 13.5, 90)                     # VSYS
at("R17", 20.4, 10.5, 90)                    # battery sense
at("R18", 20.4, 8.3, 90)
at("C8", 23.4, 8.4, 90)
at("U10", 25.0, 5.4, 0)                      # VBAT_SENSE buffer (V&V DEC-07), its decoupling beside it
at("C54", 28.2, 5.2, 90)
at("R5", 21.8, 21.5, 90)                     # PROG
at("R9", 23.0, 21.5, 90)
at("R10", 24.2, 21.5, 90)
at("R43", 25.4, 21.5, 90)
at("R44", 26.6, 21.5, 90)
at("R64", 27.8, 21.5, 90)                    # ILIM
at("TP7", 26.6, 17.8, 0)                     # CHG_TEMP
at("TH1", 19.95, 20.0, 90)                    # the cell's temperature, at the hole's right edge

# ---- 3V3 LDO under the glass left of the cell's hole (x 72.5 .. 80.5, the antenna's strip is left of it; U3 is 1.45 tall: not under the FPC)
put("U3", 77.0, 92.0, 90)
put("C5", 75.0, 86.5, 90)
put("C6", 77.2, 86.5, 90)
put("C7", 79.4, 86.5, 90)
put("JP1", 77.0, 98.0, 0)

# ---- test pads on the back
for i in range(5):
    at(f"TP{i + 1}", -8.0 + 4.0 * i, 51.5, 0, "B")      # between seat 2's clusters
at("TP3", 26.0, 25.0, 0, "B")                # VBAT: on the back by JP5 and the cell's path
at("J7", 0.0, -48.5, 0, "B")                 # SWD pogo pads on the back, under the row
at("TP9", 14.8, -33.5, 0)                    # the audio PWM, by its filter
for i, ref in enumerate(("TP11", "TP12", "TP13", "TP14")):    # LCD WR, DC, D0, TE on the back, below J3
    at(ref, -6.0 + 2.54 * i, -25.0, 0, "B")

# ---- audio: the amplifier under the glass right of J3, J4 in the strip (mouth towards the speaker), the filter in the rows below
at("U7", 21.0, -20.0, 90)                    # FM8002A
at("C34", 17.4, -17.5, 90)                   # VDD
at("C35", 17.4, -20.0, 90)
at("C40", 17.4, -22.2, 90)                   # BYPASS
at("R36", 17.4, -24.2, 90)                   # SHUTDOWN pull-up
at("R37", 17.6, -26.5, 90)
at("R34", 18.8, -26.5, 90)                    # Ri to -IN
at("R35", 20.0, -26.5, 90)                    # Rf, VO1 to -IN
at("C39", 21.2, -26.5, 90)                    # Ci
at("R32", 17.6, -29.0, 90)
at("C37", 18.8, -29.0, 90)
at("R33", 20.0, -29.0, 90)
at("C38", 21.2, -29.0, 90)
at("Q3", 21.0, -13.0, 0)                     # SHUTDOWN switch
at("TP8", 14.8, -25.9, 0)                    # SHUTDOWN: flat, under the tail's edge
at("JP3", 14.8, -27.8, 0)                    # VSYS -> amplifier
at("TP15", 14.8, -30.5, 0)
at("J4", 31.6, -8.2, 0)                      # PicoBlade: mouth towards the speaker (case -y), pins by the wall

# ---- seat 2's end: U9 in the spine, the power latch and Start/Select's parts in the row above the cell's hole
at("U9", 0.0, 55.0, 90)
at("RN3", -5.8, 51.5, 90)
at("RN4", -5.8, 55.0, 90)
at("C53", 5.5, 51.5, 0)
at("Q1", -7.6, 48.75, 0)
at("R16", -4.0, 48.6, 90)
at("Q7", 4.5, 49.0, 0)
at("D8", 8.3, 49.0, 0)
at("R62", 10.9, 48.6, 90)
at("R63", 12.1, 48.6, 90)
at("R15", 13.3, 48.6, 90)
at("C47", 14.5, 48.6, 90)
at("TP6", -13.5, 48.6, 0)
at("D2", 29.5, 45.8, 0)                      # the latch's other half: right end of the row
at("D3", 22.0, 45.0, 180)
at("R13", 24.5, 45.0, 90)
at("R14", 25.7, 45.0, 90)
at("R38", 33.0, 44.0, 90)
at("R42", 34.2, 44.0, 90)

# ---- the placement search's result (2026-10-27, tools/place_search.py: hole 1 mm towards seat 2, the latch row in end2_l) ------------------
# The cell hole 1 mm towards seat 2 (CUTOUTS above, case.scad cell_dy 1, j2_dy 0.3) with J2 (0.3), TH1, JP5 and TP3, the power latch
# (Q1, R16, D2, R13, R42: at seat 1's end) and the seat-2 Start's row (Q7, D8, R62, R63, R15, C47, TP6: end2_l) that the hole's new edge
# landed on, put where the search found room (the zones in placement_zones.py; check_fit passes). KiCad frame: x, y, rot, side.
ADOPTED = {
    "J2": (131.55, 69.7, 180, "F"),
    "TH1": (119.95, 79.0, 90, "F"),
    "JP5": (124.0, 70.0, 0, "F"),
    "TP3": (126.0, 74.0, 0, "B"),
    "Q1": (75.2, 144.7, 0, "F"),
    "D2": (109.08, 149.543, 0, "F"),
    "R13": (106.489, 150.046, 90, "F"),
    "R42": (117.43, 148.073, 90, "F"),
    "R16": (79.08, 146.869, 90, "F"),
    "Q7": (79.107, 54.948, 180, "F"),
    "D8": (75.093, 54.549, 180, "F"),
    "TP6": (96.645, 50.484, 180, "F"),
    "R62": (74.645, 51.524, 270, "F"),
    "R15": (68.206, 51.084, 270, "F"),
    "R63": (67.822, 49.028, 270, "F"),
    "C47": (67.006, 51.084, 270, "F"),
}
for _ref, (_x, _y, _rot, _side) in ADOPTED.items():
    P[_ref] = (round(_x, 3), round(_y, 3), _rot, _side)

# ---- tools/place_new.py's result for the ESP32-S3 build's new parts (placement_new.json): nearest legal spots to the poses above ----
_NEW = os.path.join(HERE, "placement_new.json")
if os.path.exists(_NEW):
    for _ref, (_x, _y, _rot, _side) in json.load(open(_NEW)).items():
        P[_ref] = (_x, _y, _rot, _side)

# ---- the placement search's result (tools/place_search.py): a no-op unless picks.json or PLACE_PICKS says otherwise ----------------
placement_zones.apply(P, PICKS)
