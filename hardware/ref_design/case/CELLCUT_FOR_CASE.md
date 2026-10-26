# The board with the cell in it (2026-10-22): what the case needs to know

Routed clean on branch `ref-design-cellcut` at 4af3024 (0 unconnected, DRC 0 errors). With the speaker back and the board 72.4 mm wide (2026-10-22): routed clean again (0 unconnected, DRC 0 errors). With the side screws back inward (2026-10-22, next section): routed clean again (0 unconnected, DRC 0 errors).

For the Blastoise Shell session. The cell now sits in a hole in the board (row 19 of Alex's
2026-10-21 list), and since 2026-10-22 the speaker lies on the board's top in the right bezel
beside the screen (section "The speaker on the board", below). Every machine-placed part is on the top (one-sided assembly).
Alex also allowed the board session to move the side screws and to slide the USB-C along its
wall: the screws moved; the USB-C moved down its wall and, with the speaker, back to the case's own
position. `case.scad` doesn't know any of this yet; nothing in the case was changed.
The board's side is `board/placement.py` (`CUTOUTS`, `SIDE_SCREWS`, `USB_Y`).

## Re-placed for the 3.5 in panel (2026-10-26): this section wins over every number below

Board 72.4 x 151.64 (case 77.4 x 156.64 x 9.6). Case frame (x right, y up towards seat 2), heights over the board's top copper.

| | Case (x, y) now | Was (2.4 in) |
|---|---|---|
| Cell hole (54 x 37, r 1) | x -27.0 .. 27.0, y 6 .. 43 (under the glass's seat-2 half) | -28.7 .. 25.3, -3.8 .. 33.2 |
| Speaker hole (stadium, 11 x 16) | centre 29.7, -24.5; the speaker sunk in it on a 0.6 floor pocket, 0.9 over the board | lying on the board's top at 28.7, -11.4 |
| J3 (the FPC's ZIF) | 0, -17.59, mouth towards seat 1 | 5.5, -8.2 |
| J1 | y 1.5 | 2.0 |
| J2 / J4 | -31.7, 2.56 (mouth +y) / 31.8, -8.2 (mouth -y, towards the speaker) | -30.7, -7.24 / 31.9, -27.0 |
| J5, SW18/19 | -32.1, -5.0 / x -31.0, y -30.2 and -33.85 | -32.6, 15.2 / -32.35, -20.7 and -24.35 |
| Side screws H1 to H4 | (+-32.5, +-40), the glass's corners | (32.5, 29), (-32.5, 29), (25.9, -21.95), (-33, -15.2) |
| U6, U8 | beside each other in seat 1's spine (-3.2, -55), (3.2, -56) | 0, -36.4 and 1.2, -44.2 |
| Core U4 | -24, -14 (its panel pins face J3's pads) | -22.2, -19.5 |

The FPC (the old design's numbers, unverified): the tail zone (x +-13.25, y -27.1 to -13.7) keeps nothing but J3 and flat pads under it; the **body** (x -22 to 27, y -43 to -27.1)
allows parts up to **1.05** tall (it lies on the glass's back; U4, X1, L2, U2 and every 0402/0603 are 1.0 or lower), but nothing 5.1 mm in from the glass's edge (the neck).
Taller parts are under the glass (2.2 limit) or beside it under the full-width band (3.5), the ends' ceiling beyond its slopes (`case/check_fit.py` computes it
from `case.scad`'s `ISLAND` echo line). The routing: `route.py` no longer pre-routes the USB pair by hand (`ROUTE_USB_PAIR=1` brings it back), Freerouting does.

## Routing levers (2026-10-24, branch `ref-design-levers`, an experiment Alex has not picked yet)

To make the board easier to route without changing its size: the UART connector J6, the four
revision-strap resistors R48-R51 and the spare C41 are gone, the debug buttons left the left strip,
and the power latch's parts left the right strip. check_fit passes. What the case would change:

| | Case (x, y) now | Was |
|---|---|---|
| **Debug pinholes** SW18 BOOTSEL / SW19 RESET (`pin_pos`) | **-32.35, -20.7 / -32.35, -24.35** (left bezel below the screw H4, one above the other, switches lying across the bezel) | -31.2, 8.6 / -31.2, 1.6 |
| J6 UART (`uart_pos`) | gone: J5 alone sets the island's wall on the left (`side_tall`) | -32.6, 22.7 |
| U3, C5-C7 (the LDO) | 2 mm towards seat 1 (C5-C7 now under the deck's joint band, 0.9 mm tall) | |
| Q1, R16 | left of U9 under seat 2's deck: (-9.5, 35.5), (-6.4, 35.2) | right strip |
| D8, Q7, R62, R63, R15, C47, TP6 | under seat 2's right deck along the hole's top edge, x 4.8 .. 9.7, y 34.6 .. 37.7 | right strip |
| C42 (unfitted 1206) | right bezel above J4's plug: (26.0, -27.2) | right strip |

- Each pinhole keeps 5.54 mm from H4 (the case's 5.4 assert); the switches (1.94 mm) sit under the
  left bezel, clear of the joint along the left edge (x <= -35.4).
- The strip ribs: the left one no longer has buttons to back (only J5, where a cable is pushed in);
  the debug buttons now sit on the full-width board, which needs a floor rib or boss under each
  pinhole (about (-32.35, -20.7) and (-32.35, -24.35)) so a pin press doesn't bend the board.
  The right strip keeps TH1 and J1 only.
- **Done in the case (2026-10-24):** the pinholes moved, a floor rib (`pin_rib`) under both switches into
  H4's boss, J6 and `uart_pos` removed; see `SHELL.md`, "The island", for the asserted clearances.

## Moved again: the side screws back inward (2026-10-22, Alex: no shoulders)

**The four side screws now sit at |y| <= 29, inside the top piece's area**, so the shoulders,
the decks' notches and the bosses past the step go. This section wins over every older number
below.

| | Case (x, y) | KiCad | Was (case) |
|---|---|---|---|
| **H1**, right strip, seat 2's end | **32.5, 29.0** | 132.5, 71.0 | 31.4, 36.0 |
| **H2**, left strip, seat 2's end | **-32.5, 29.0** | 67.5, 71.0 | -31.4, 36.0 |
| **H3**, right bezel, between the speaker and J4 | **25.9, -21.95** | 125.9, 121.95 | 31.4, -36.0 |
| **H4**, left bezel, between J2 and the LDO | **-33.0, -15.2** | 67.0, 115.2 | -31.4, -36.0 |
| H5, H6 (the ends) | 3.0, 50.75 / -3.0, -50.75, unchanged | | |

- Every part keeps 2.8 mm (`boss_d` / 2 + 0.3) from each screw's centre. A 5 mm boss keeps 1.2 mm
  from the board's edge at H1 and H2, 0.7 at H4 (x -35.5 against -36.2), and 1.3 from the cell's
  hole at H2 (x -30.0 against the hole's -28.7): the tub's 6.0 bosses would leave only 0.8 to the
  hole at H2 and 0.2 to the board's edge at H4, so slim them there.
- H3 is 25.9 from the centre, not at the wall: the speaker (10 wide) and J4 fill the bezel's width
  below the hole, so the screw sits under the bezel next to the panel's frame (x 22.86): a
  5 mm boss there spans x 23.4 .. 28.4, 0.54 off the frame band, 0.3 below the speaker's
  parts keep-out (body 0.8 off) and 0.3 above J4's mated plug.

Other things that moved for it (the tables below have the old numbers; this list wins):

| | Case (x, y) now | Was |
|---|---|---|
| **USB-C J1** centre (slid up its wall) | x face 38.55, **y 2.0** | y 0.0 |
| USB-C tab | x 36.2 .. 37.2, **y 8 .. -4** | y 6 .. -6 |
| **Speaker** centre (and cone, grille) | **28.7, -11.4** (body y -3.9 .. -18.9, parts kept out y -3.4 .. -19.4, x as before) | 28.7, -13.5 |
| Speaker grille racetrack | x 25.2 .. 32.2, **y -5.4 .. -17.4** | y -7.5 .. -19.5 |
| **J4** | **31.9, -27.0** (body x 28.75 .. 35.65, y -22.17 .. -31.73; plug x about 24.75 .. 28.75, y -24.75 .. -29.25) | 31.4, -26.75 |
| **Debug pinholes** SW18 / SW19 | **-31.2, 8.6 / -31.2, 1.6** | -31.2, 13.0 / 6.0 |
| J6 UART / J5 SWD | **-32.6, 22.7 / -32.6, 15.2** (courtyards y 26.1 .. 19.3 / 18.6 .. 11.8) | -32.6, 28.4 / 20.9 |
| Q3 (SOT-23) | 25.5, -33.9 | 25.5, -33.4 |

The speaker's top now runs to y -3.9, 0.1 inside the hole's edge (-3.8): the pocket in the top
piece moves up with it. The power latch's first rows went up under seat 2's deck where the old
H1 was (all 1.2 mm or lower).

Coordinates: the case's frame (origin at the case's centre, x right, y up towards seat 2), with
KiCad's in brackets (y down, the centre at 100, 100). Heights are over the board's top copper.

## The board 2 mm wider: 72.4 x 127.7 mm (2026-10-22, Alex: up to 2 mm)

The outline grows 1 mm on each side and stays centred on the case: **x -36.2 .. 36.2** (was
-35.2 .. 35.2), y unchanged (-63.85 .. 63.85), corners r 5.5, the USB-C tab step out 1 mm with it.
The case grows 2 mm wide to match (walls, crush ribs and the seam 1 mm out on each side). The cell's
hole moved 1 mm left (below), so the left strip keeps its 7.5 mm and the right strip beside J1
gets the whole 2 mm (10.9 mm): the power-on nets couldn't get past J1 there. What moved with it:

| | Case (x, y) now | Was |
|---|---|---|
| Board edges | x -36.2 / 36.2 | -35.2 / 35.2 |
| USB-C J1 face | x 38.55, y 0.0 | x 37.55 |
| USB-C tab | x 36.2 .. 37.2, y 6 .. -6 | 35.2 .. 36.2 |
| Cell hole | x -28.7 .. 25.3 | -27.7 .. 26.3 |
| J2, TH1 | -30.7, -7.24 / 26.1, 14.0 | -29.7 / 27.1 |
| J5, J6 | -32.6, 20.9 / -32.6, 28.4 | -31.6 |

Unchanged then: the screws (x +-31.4, y +-36; the end ones), the clusters, Start/Select, the debug
pinholes (-31.2, 13.0 / 6.0), the glass and frame, the speaker and J4. The tables below give the
old numbers where they predate this; this table wins.

## Moved: the screws and the USB-C (superseded: the screws moved back inward, above)

| | Case (x, y) | KiCad | Was (case) |
|---|---|---|---|
| H1, right, seat 2 side | **31.4, 36.0** | 131.4, 64.0 | 31.4, 22.0 |
| H2, left, seat 2 side | **-31.4, 36.0** | 68.6, 64.0 | -31.4, 22.0 |
| H3, right, seat 1 side | **31.4, -36.0** | 131.4, 136.0 | 31.4, -22.0 |
| H4, left, seat 1 side | **-31.4, -36.0** | 68.6, 136.0 | -31.4, -22.0 |
| H5, seat 2's end | 3.0, 50.75 (unchanged) | 103, 49.25 | |
| H6, seat 1's end | -3.0, -50.75 (unchanged) | 97, 150.75 | |
| USB-C J1 centre | x face 38.55 (1 mm out with the wider board), y 0.0 | y 100.0 | |
| USB-C tab under it | x 36.2..37.2, y 6..-6 | 136.2..137.2 x 94..106 | |

- The side screws sat in the 7-8 mm strips beside the cell's hole and closed half of each; they
  went to the ends of the long sides, past the hole, where the board is full width: 17.1 mm from
  the nearest cluster centre (outside the rings), 2.8 mm or more from every part. The new y
  (+-36) is in the decks' region (the step's foot is at y +-33.75): the bosses there come down
  from the decks or need a pocket in the step; the case's call.
- J1 is back where `case.scad` has it (2026-10-22): the speaker and J4 took the bezel below the
  hole, where it had gone. Its opening, tab and `rib_free` window need no change.

## The hole

| | Case (x, y) | KiCad | Size | Corners |
|---|---|---|---|---|
| Cell | x -28.7 .. 25.3, y -3.8 .. 33.2 | 71.3 .. 125.3 x 66.8 .. 103.8 | 54.0 x 37.0 | r 1.0 |

- A worst-case 403450 (50 +-2 x 34 +-2 x 4.0 +-0.5, so 52 x 36 x 4.5) with 0.5 mm round it, plus
  1.0 mm at the left end: the cell's leads come out of its left end and run along it to J2. With
  the cell pushed right (against x 25.3), the leads get 2 mm (worst case) to 4 mm (nominal).
- **Push the cell to the right end** (x 25.3): TH1, the charger's thermistor, sits on the board
  0.45 mm from that edge (case 26.1, 14.0) and reads the cell through it.
- **J2** (cell plug, PicoBlade, 3.4 mm): case (-30.7, -7.24), mouth towards +y (into the hole's
  lower left corner); the plug sticks 3 mm out over the board's left strip and the hole's corner.
- The size lives in one place, `placement.py` `CUTOUTS`: if Alex measures real cells it changes
  there (and here).
- The plug holes (`slot`, `spk_slot`) are no longer cut. No speaker hole: the speaker lies on
  the board's top (below); the speaker pad, chamber and side outlet go.

## Heights and the stack

| | Over the top copper | Under the board's back |
|---|---|---|
| Cell's top, worst case (4.5 mm) | 1.6 | |
| Cell's top, nominal (4.0 mm) | 1.1 | |
| Cell's bottom, on the tub's floor | | 1.3 |
| Start/Select metal dome apex (row 4) | 0.26 nominal, 0.31 worst | |

- **Split:** at most 1.6 over the board (the decks' band allows 1.6 where a worst-case cell's
  seat-2 edge can reach, y 32.7 against the band from 31.8), the rest under it: the cell's
  bottom sits 2.9 under the top copper, 1.3 under the back.
- **The back now has no machine-placed parts**: only bare copper (test pads TP1-TP5 between seat
  2's clusters, TP3 at case (-10.6, -14.0), TP9, TP11-TP14 under J3, the SWD pads J7 at
  (0, -40.4), 14.9 x 4.0) and the Start/Select domes' 0.5 mm vents. `back_limit` 0.5 can stay.
- So `board_lift` is set by the cell alone: 1.3 with a flat floor, or **0.7 with the floor
  pocketed 0.6 mm under the cell** (0.9 floor left there).
- Start/Select: metal snap domes (HYP 600-415S-260, 4.0 mm, 0.20 high) under 0.06 mm Kapton dots;
  the switch closes about 0.11 over the board. Per the row 4 note: add `ss_metal_dome = [4.0,
  0.31]` and the two asserts; `ss_lift` stays 0.35 unless the strip's plunger measures under 0.36.
  The domes' vents (0.5 mm holes at the contacts' centres + 1.25 mm towards seat 1) land on floor
  ribs: keep the ribs off them. Each dome also has a 0.5 mm via 1.25 mm the other side of its
  centre (tented), its signal's way out of the ring.

Other limits unchanged (glass 2.2, bezels and spines 3.45, decks 1.6, round the clusters 0.5,
nothing under the folded FPC but J3). Seat 2's spine now only covers U9 (the shift register,
1.2 mm: the deck would do); seat 1's spine covers U6 (1.75 mm, needs it).

## J5 and J6, the debug connectors (2026-10-22)

They faced each other nose to nose, so neither plug went in. Both are now turned with their
mouths towards the board's left edge (x -36.2), courtyards x -35.9 .. -29.3, J6
y 25.0 .. 31.8 (clear of seat 2's deck band), J5 y 17.5 .. 24.3; each housing's front is about
0.6 mm in from the board's
edge. The plugs go in sideways with the case open and stick out past the board's edge (a JST
SH plug is about 4.5 mm long), so a cable can't stay in with the case closed unless the left
wall gets a notch or slot there (the case's call; debug is case-open).

## The strips beside the hole (2026-10-22 review)

The board beside the hole is two strips, x -36.2 .. -28.7 (7.5 mm) and 25.3 .. 36.2 (10.9 mm), y
-3.8 .. 33.2: the board's weakest part. H1/H2 hold their seat-2 ends 2.8 mm past the hole; their
seat-1 ends run into the full-width board, so the side screws stay where they are (in the strips
they closed half of each). Since the screws moved back inward (above), H1/H2 sit 4.2 mm inside the
strips' seat-2 ends. What the case should add: **a floor rib under each strip**, along its
length, meeting the board's back (only soldermask there: the GND pour, tracks and tented vias),
so nothing presses the strips into a bend:

- under the left strip at least at the debug buttons a pin presses (SW18, SW19 at case (-31.2,
  8.6) and (-31.2, 1.6)) and under J5/J6, where a cable is pushed in (-32.6, 15.2 / 22.7);
- under the right strip (the power latch and TH1) at its middle, y about 14.

Keep the ribs out of the hole (the cell) and 0.5 mm off its edge.

## Parts the case sees that moved

| Part | Case (x, y) | Height | Note |
|---|---|---|---|
| J2 cell plug | -30.7, -7.24 | 3.4 | left bezel |
| J5 SWD (JST SH) | -32.6, 20.9 | 2.9 | left strip, mouth towards the left wall (`swd_pos` was -31.7, -2.0) |
| J6 UART (JST SH) | -32.6, 28.4 | 2.9 | left strip, mouth towards the left wall (`uart_pos` was -31.7, -12.0) |
| U6 backlight op-amp, SOIC-8 | 0, -36.4 | 1.75 | under seat 1's spine |
| U8 / U9, 74HC165 TSSOP-16 | 1.2, -44.2 / 0, 41.5 | 1.2 | between each seat's clusters |
| TH1 | 26.1, 14.0 | 0.5 | at the hole's right edge |

## Thickness

| | Today (cell under the board) | Flat floor | Floor pocketed 0.6 under the cell |
|---|---|---|---|
| At the screen | 13.35 | 10.15 | **9.55** |
| At the decks | 11.5 | 8.3 | **7.7** |

(floor 1.5 + `board_lift` + board 1.6 + 5.75 at the screen, + 3.9 at the decks.)

Also for `case.scad`: the tub's tongue no longer fits under the board (`z_seam` + `tongue_h` +
`seam_gap` must stay under `z_board`, which drops from 6.0 to 2.2-2.8); the screws get shorter;
the speaker's chamber and side outlet go (it now fires up through the face: below).

## The speaker on the board (2026-10-22)

The INGHAi GSPK151035PN (15 x 10 x 3.5 mm racetrack, 8 ohm, 35 mm leads to a 1.25 mm plug) lies
flat on the board's top, face (cone) up, long axis along the case (y), in the right bezel below
the cell's hole, between J1 and the seat-1 deck. It is not machine-placed: at assembly it is
stuck down with foam tape and plugged into J4. The board keeps parts out of its outline plus
0.5 mm (copper runs under it) and marks it on the front silkscreen. `placement.py` `SPEAKER`.

| | Case (x, y) | KiCad | Size |
|---|---|---|---|
| Speaker centre (and its cone's) | **28.7, -13.5** | 128.7, 113.5 | |
| Speaker body | x 23.7 .. 33.7, y -6.0 .. -21.0 | 123.7 .. 133.7 x 106.0 .. 121.0 | 10 x 15 |
| Parts kept out | x 23.2 .. 34.2, y -5.5 .. -21.5 | 123.2 .. 134.2 x 105.5 .. 121.5 | 11 x 16 |
| **J4** (speaker plug, PicoBlade 53261, horizontal) | **31.4, -26.75** (body x 28.25 .. 35.2, y -22.0 .. -31.5) | 131.4, 126.75, turned 270 | 3.4 tall |
| J4's mouth and the mated plug | plug x about 24.4 .. 28.4, y -24.5 .. -29.0 | | about 3.2 tall |

- **Height:** its top is 3.5 mm over the top copper, 3.8 worst case (tolerance and the foam
  tape), over the bezel's 3.45: **the top piece needs a pocket about 0.5 mm deep** over the parts
  keep-out above (x 23.2 .. 34.2, y -5.5 .. -21.5), so 3.95 clear there. The panel frame band
  (x up to 22.86) and the glass are untouched: the speaker starts at x 23.7.
- **Grille:** in the face over the cone, centred on (28.7, -13.5): holes or slots inside the
  speaker's outline inset 1.5 mm (a 12 x 7 racetrack, x 25.2 .. 32.2, y -7.5 .. -19.5); keep
  the pocket's rim (0.5 mm round the body) solid so the speaker's front seals against it.
- **J4** sits below the speaker against the right wall, mouth towards the glass; the plug goes in
  from the left (x 28.4 towards 24.4) and its two leads lie on the board up to the speaker.
  J4 (3.4) and the plug fit under the bezel's 3.45; a 0.3 mm pocket over x 24 .. 35.2,
  y -22 .. -31.5 would give the plug room if the top piece allows it. J4 keeps 1.1 mm off the
  crush rib at (35.7, -28).
- The amplifier U7 (SOIC-8, 1.75) is under the glass's right edge at (20.3, -25.5); its small
  parts are under the glass and the seat-1 deck (Q3, SOT-23, 1.19 tall, at (25.5, -33.4), above
  the screw H3 and outside the cluster's 0.5 mm band): all inside today's limits. TP7, TP8, TP15
  (flat test pads) and JP3 (a solder jumper) lie under the folded FPC.
- The old side outlet and chamber go.

## check_fit against this board (today's positions.json, 2026-10-22, the screws back inward)

12 failures, all from the case's old positions: D2, D3, R13, R38, R42 inside the old H1 boss at
(31.4, 36); R1, R3, R4 inside the old speaker keep-out (the speaker moved up 2.1 mm; they now sit
above J4, right of H3); J1 2 mm up its wall: its opening off the mouth by 2.0 and its two
through-hole leads (case (31.77, 6.32) and (35.95, 6.32)) on a floor rib, so the `rib_free`
window under J1 moves up 2 mm too. The case's new bosses at the new positions are not checked
until `case.scad` has them; the board keeps every part 2.8 mm from each (checked on the board).
J4's mated plug (about y -24.75 .. -29.25) reaches 0.04 mm into C43's courtyard (1.0 mm tall) at
its lower edge: an "about" box, as before.

### Before the screws moved back (superseded)

14 failures, all from things the case doesn't know yet: parts within the *old* side screws'
bosses (C16, JP1 at the old (-31.4, -22); D8, Q7, R62, R63 at (31.4, 22); J5 at (-31.4, 22); J4
at the old (31.4, -22)), the four dome vents on floor ribs, and the wider board: the USB-C face
at x 38.55 against the old 37.55, and J6 by the old crush rib at (-35.7, 28) (with the ribs 1 mm
out at -36.7 it clears by 1.1 mm). Not checked by check_fit (the speaker is not a footprint): the
speaker's 3.5-3.8 mm in the bezel (the 0.5 mm pocket) and its grille, above.
