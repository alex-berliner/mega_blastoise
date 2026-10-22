# The board with the cell and the speaker in it (2026-10-21): what the case needs to know

For the Blastoise Shell session. The board session (branch `ref-design-cellcut`) cut two holes in
the board so the cell and the speaker sit in the board's plane, not under it (rows 18/19 of Alex's
2026-10-21 list; row 18 done). `case.scad` doesn't know yet; nothing in the case was changed. The
board's side of it is `board/placement.py` (`CUTOUTS`, the moved parts).

Coordinates: the case's frame (origin at the case's centre, x right, y up towards seat 2), with
KiCad's in brackets (x right, y down, the case's centre at 100, 100). Heights are over the board's
top copper unless they say otherwise.

## The holes

| | Case (x, y) | KiCad | Size | Corners |
|---|---|---|---|---|
| Cell | x -27.7 .. 27.8, y -3.8 .. 33.2 | 72.3 .. 127.8 x 66.8 .. 103.8 | 55.5 x 37.0 | r 1.0 |
| Speaker | x 16.5 .. 27.5, y -34.6 .. -18.6 | 116.5 .. 127.5 x 118.6 .. 134.6 | 11.0 x 16.0 | r 1.0 |

- **Cell hole:** a worst-case 403450 (50 +-2 x 34 +-2 x 4.0 +-0.5, so 52 x 36 x 4.5) with 0.5 mm
  round it takes x -25.2 .. 27.8 (53.0); the 2.5 mm left of that (x -27.7 .. -25.2, the full
  height) is the **lead pocket**: the cell's leads leave its left end and run down that pocket to
  J2 at the hole's seat-1 corner. Turn the cell so its protection board and leads are at the
  left end. A nominal 50 mm cell pushed right leaves 5.5 mm at the left end.
- **The cell is pushed to the hole's right end** (x 27.8): TH1, the charger's NTC, sits on the board
  0.45 mm from that edge (case 28.6, 14.0), and reads the cell through it. Please locate the cell
  against that end (a floor pocket whose right wall is at x 27.8, or a rib or foam at the cell's
  left face that leaves the lead pocket free).
- **Speaker hole:** the GSPK151035PN (15 x 10 racetrack, 3.5 thick) stands lengthwise along y in
  it, 0.5 mm round it; centre (22.0, -26.6). Its right edge is 10.2 mm in from the case's outside
  at x 37.7 (8 mm of board and wall between), for row 7's side outlet. Its leads go up to J4 above
  the hole; leads at the end towards J4 (y -18.6) are best.
- Both holes keep at least 1 mm from the screw bosses (the speaker hole's nearest point is 3.5 mm
  from the screw at (31.4, -22)), clear of the clusters' rings, J3's whole range and the USB-C tab.
- **The plug holes are gone:** `slot` and `spk_slot` are no longer cut in the board (the board
  reads its cutouts from `placement.py` now, not `positions.json`). Both plugs now plug in on the
  board's top beside their holes.

## The stack

| | Over the top copper | Under the board's back |
|---|---|---|
| Cell's top, worst case (4.5 mm) | 1.6 | |
| Cell's top, nominal (4.0 mm) | 1.1 | |
| Cell's bottom (on the tub's floor) | | 1.3 |
| Speaker's top (3.5 mm, on the floor) | 0.6 | |
| Speaker's face (down, on the floor) | | 1.3 |
| Tallest part on the back (L2, 1.08) | | 1.08 |

**The split:** 1.6 mm over the top and 2.9 under it (1.3 under the back) for a worst-case 4.5 mm
cell. Why not the 2.2 under the glass: a 36 mm cell's seat-2 edge can reach y 32.7, under the
decks' 1.6 mm band (it starts at y 31.8), so 1.6 is the most the cell may stand over the board
there; and the RP2354B core now sits on the back (below), which needs 1.2 mm there anyway. A higher
cell would buy nothing: the floor has to be 1.3 mm down for the back's parts.

So **`board_lift` (floor to the board's back) becomes 1.3** (was `cell[2] + cell_gap` = 4.5). The cell
and the speaker rest on the floor; nothing is stuck to the board's back any more.

## New height limits

| Zone | Was | Now |
|---|---|---|
| The back (`back_limit`) | 0.5 (the gap over the cell) | **1.3**: parts up to 1.2 mm (check_fit keeps 0.1) |
| The cell over the board | (under it) | 1.6 worst case; everything it lies under allows that (glass 2.2, frame 2.45, the decks' band 1.6, the side bezels 3.45) |
| The speaker over the board | (under it) | 0.6 |
| J2's pocket in the spine (`pocket_limit`) | 3.6 over J2 | **not needed**: J2 is in the left bezel (3.45) |

Everything else on the top is unchanged (glass 2.2, bezels and spines 3.45, decks 1.6, round the
clusters 0.5, nothing under the folded FPC but J3).

## The parts the case sees that moved

| Part | Case (x, y) | KiCad | Height | Note |
|---|---|---|---|---|
| J2 (cell plug, PicoBlade) | -27.7, -7.24, mouth to +y | 72.3, 107.24, rot 180 | 3.4 | left bezel; its mated plug sticks 3 mm out over the lead pocket's corner |
| J4 (speaker plug, PicoBlade) | 28.0, -11.0, mouth to -y | 128.0, 111.0, rot 0 | 3.4 | right bezel, above the speaker's hole |
| J5 (SWD, JST SH) | -31.2, -28.4 | 68.8, 128.4, rot 180 | 2.9 | below the seat-1 side screw (was -31.7, -2.0: `swd_pos`) |
| J6 (UART, JST SH) | -31.25, 28.35 | 68.75, 71.65, rot 0 | 2.9 | above the seat-2 side screw (was -31.7, -12.0: `uart_pos`) |
| U7 (FM8002A, SOIC-8) | 0, -36.4 | 100, 136.4 | 1.75 | under **seat 1's spine**, which it needs (the deck allows 1.6) |
| U6 (MCP6002, SOIC-8) | 18.6, -12.6 | 118.6, 112.6 | 1.75 | under the glass |
| TH1 (NTC) | 28.6, 14.0 | 128.6, 86.0 | 0.5 | at the cell hole's right edge |

- **Seat 2's spine** has nothing under it now (J4 and the backlight moved): it could go (SHELL.md,
  open question 6), and with it J2's/J4's pockets. Seat 1's spine stays for U7.
- The BOOTSEL and RESET pinholes, the USB-C, J3 and every button are where they were.

## The board's back

The board now has parts on both sides (two-sided assembly). Back parts, all 1.1 mm or less, in
these areas (KiCad; case in brackets), which the floor's ribs must clear:

- the core and its neighbours: 85.6 .. 116.1 x 104.4 .. 132.2 [x -14.4 .. 16.1, y -4.4 .. -32.2]
- the backlight's and USB input's resistors: 116.3 .. 130.4 x 104.6 .. 117.1 [16.3 .. 30.4, -4.6 .. -17.1]
- the LDO's and latch's: 102.2 .. 112.8 x 130.9 .. 141.4 [2.2 .. 12.8, -30.9 .. -41.4]
- test pads and pogo pads (flat): 69.5 .. 87.5 x 128.9 .. 135.5 [-30.5 .. -12.5, -28.9 .. -35.5]
  and TP1-TP5 between seat 2's clusters as before

(`board/fab/battler_bottom.png` shows them.) The floor's ring ribs under the clusters stay clear of
all of it.

## Assembly order

Built upside down as before, but the cell and the speaker can't ride on the board's back now.
Suggested: stick the cell to the tub's floor (in its place, pushed right) and the speaker face down
on its pad, plug both into J2 and J4 with the board face down in the top piece and the tub held
beside it, then close the tub; the cell and the speaker drop into their holes.

## Thickness

| | Today (cell under the board) | With the holes |
|---|---|---|
| At the screen | 13.35 | **10.15** (floor 1.5 + 1.3 + board 1.6 + 2.4 + panel 2.3 + tape 0.25 + face 0.8) |
| At the decks | 11.5 | **8.3** (1.5 + 1.3 + 1.6 + 3.9) |

If the floor is thinned to 0.9 under the cell and under the back's parts (0.6 mm pockets),
`board_lift` can be 0.7 (the speaker then stands 1.2 over the board, still within the glass's 2.2
and the decks' 1.6): **9.55 and 7.7**.

Things that follow in `case.scad`: the tub's tongue no longer fits under the board (`z_seam` +
`tongue_h` + `seam_gap` must stay 0.2 under `z_board`, which drops from 6.0 to 2.8), so the seam
and the back's round (`back_r` 4.2) need another look; `screw_engage` and the M2 x 8 screws' length
change with the lower stack; the floor's speaker chamber and outlet move to the new speaker spot.

## check_fit against this board

`python3 case/check_fit.py` with today's positions.json: no collisions on the top (every part
clears the glass, frame, decks, clusters, FPC, screws and crush ribs). Its failures are all
"(back) is N mm tall; the ribs, cell and speaker leave 0.5 mm": the back's parts against the 0.5 mm
that the cell under the board used to leave. They clear with `back_limit` 1.3.
