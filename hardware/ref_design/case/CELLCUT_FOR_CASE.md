# The board with the cell in it (2026-10-22): what the case needs to know

Routed clean on branch `ref-design-cellcut` (0 unconnected, DRC 0 errors).

For the Blastoise Shell session. The cell now sits in a hole in the board (row 19 of Alex's
2026-10-21 list: no speaker). Every machine-placed part is on the top (one-sided assembly).
Alex also allowed the board session to move the side screws and to slide the USB-C along its
wall; both moved. `case.scad` doesn't know any of this yet; nothing in the case was changed.
The board's side is `board/placement.py` (`CUTOUTS`, `SIDE_SCREWS`, `USB_Y`).

Coordinates: the case's frame (origin at the case's centre, x right, y up towards seat 2), with
KiCad's in brackets (y down, the centre at 100, 100). Heights are over the board's top copper.

## Moved: the screws and the USB-C

| | Case (x, y) | KiCad | Was (case) |
|---|---|---|---|
| H1, right, seat 2 side | **31.4, 36.0** | 131.4, 64.0 | 31.4, 22.0 |
| H2, left, seat 2 side | **-31.4, 36.0** | 68.6, 64.0 | -31.4, 22.0 |
| H3, right, seat 1 side | **31.4, -36.0** | 131.4, 136.0 | 31.4, -22.0 |
| H4, left, seat 1 side | **-31.4, -36.0** | 68.6, 136.0 | -31.4, -22.0 |
| H5, seat 2's end | 3.0, 50.75 (unchanged) | 103, 49.25 | |
| H6, seat 1's end | -3.0, -50.75 (unchanged) | 97, 150.75 | |
| **USB-C J1 centre** | **x face 37.55, y -11.0** | y 111.0 | y 0.0 |
| USB-C tab under it | x 35.2..36.2, **y -5.0..-17.0** | 135.2..136.2 x 105..117 | y 6..-6 |

- The side screws sat in the 7-8 mm strips beside the cell's hole and closed half of each; they
  went to the ends of the long sides, past the hole, where the board is full width: 17.1 mm from
  the nearest cluster centre (outside the rings), 2.8 mm or more from every part. The new y
  (+-36) is in the decks' region (the step's foot is at y +-33.75): the bosses there come down
  from the decks or need a pocket in the step; the case's call.
- J1 moved down its wall (same wall, orientation and tab) below the hole: beside it, its pads
  faced the cut-out and their tracks couldn't get out. Its shell legs (through-hole) are at
  y -5.3..-16.7 on the back; `rib_free` needs the new window.

## The hole

| | Case (x, y) | KiCad | Size | Corners |
|---|---|---|---|---|
| Cell | x -27.7 .. 26.3, y -3.8 .. 33.2 | 72.3 .. 126.3 x 66.8 .. 103.8 | 54.0 x 37.0 | r 1.0 |

- A worst-case 403450 (50 +-2 x 34 +-2 x 4.0 +-0.5, so 52 x 36 x 4.5) with 0.5 mm round it, plus
  1.0 mm at the left end: the cell's leads come out of its left end and run along it to J2. With
  the cell pushed right (against x 26.3), the leads get 2 mm (worst case) to 4 mm (nominal).
- **Push the cell to the right end** (x 26.3): TH1, the charger's thermistor, sits on the board
  0.45 mm from that edge (case 27.1, 14.0) and reads the cell through it.
- **J2** (cell plug, PicoBlade, 3.4 mm): case (-29.7, -7.24), mouth towards +y (into the hole's
  lower left corner); the plug sticks 3 mm out over the board's left strip and the hole's corner.
- The size lives in one place, `placement.py` `CUTOUTS`: if Alex measures real cells it changes
  there (and here).
- The plug holes (`slot`, `spk_slot`) are no longer cut. No speaker, no speaker hole: the
  speaker pad, chamber and outlet go.

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

## Parts the case sees that moved

| Part | Case (x, y) | Height | Note |
|---|---|---|---|
| J2 cell plug | -29.7, -7.24 | 3.4 | left bezel |
| J5 SWD (JST SH) | -31.2, 20.4 | 2.9 | left strip (`swd_pos` was -31.7, -2.0) |
| J6 UART (JST SH) | -31.25, 28.35 | 2.9 | left strip (`uart_pos` was -31.7, -12.0) |
| U6 backlight op-amp, SOIC-8 | 0, -36.4 | 1.75 | under seat 1's spine |
| U8 / U9, 74HC165 TSSOP-16 | 1.2, -44.2 / 0, 41.5 | 1.2 | between each seat's clusters |
| TH1 | 27.1, 14.0 | 0.5 | at the hole's right edge |

## Thickness

| | Today (cell under the board) | Flat floor | Floor pocketed 0.6 under the cell |
|---|---|---|---|
| At the screen | 13.35 | 10.15 | **9.55** |
| At the decks | 11.5 | 8.3 | **7.7** |

(floor 1.5 + `board_lift` + board 1.6 + 5.75 at the screen, + 3.9 at the decks.)

Also for `case.scad`: the tub's tongue no longer fits under the board (`z_seam` + `tongue_h` +
`seam_gap` must stay under `z_board`, which drops from 6.0 to 2.2-2.8); the screws get shorter;
the speaker's chamber and outlet go.

## check_fit against this board (today's positions.json)

No collisions with the glass, frame, decks, clusters, FPC, crush ribs or anything the case still
has right. Its 17 failures all come from things the case doesn't know yet: parts within the
*old* side screws' bosses (C16, JP1 at the old (-31.4, -22); D8, Q7, R62, R63 at (31.4, 22); J5
at (-31.4, 22)), J1's six shell legs at its new y, and the four dome vents on floor ribs.
