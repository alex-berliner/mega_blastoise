# The board for the 133 mm case: what the case asks of it (2026-10-21)

The shell's redesign (`SHELL.md`) sets the board now; the board will be re-placed and re-routed
to match. Everything below comes from `case.scad` through `case/positions.json` (`python3
case/export.py`), which `placement.py`, `build_pcb.py`, `route.py` and `silk.py` already read: the
numbers here are a snapshot of it for reading, and the JSON is the source. Coordinates are KiCad's
(the case's centre at 100, 100, y down, seat 1 at the bottom, y > 100); the case frame (origin at
the centre, y up towards seat 2) is in brackets where it helps. Heights are over the board's top
copper.

## Outline

- **70.4 x 127.7 mm**, 1.6 thick, corner radius 5.5 (was 70.4 x 151.24: 23.54 shorter, the width
  unchanged). `board.w`, `board.l`, `board.r`.
- **USB-C tab** on the right edge: 12.0 wide, 1.0 out past the edge, centred on y 100
  (`usb.tab`: 135.2..136.2 x 94..106). The receptacle's face at x 137.55 (`usb.front_x`).
- **Two plug holes** (rounded rectangles, corners r 0.6): the cell's `slot` at x 97.275..102.725,
  y 127.05..131.45 [case y -27.05..-31.45], and the speaker's `spk_slot`, the same turned round, at
  y 68.55..72.95.
- No other notches. The board sits 0.5 inside the walls all round.

## Screws

Six M2 x 8 self-tappers from the back; the board's holes are **2.2 mm** (`hole_d`), with a **5.0 mm**
boss (`boss_d`) on each face: the tub's (6.0 across) under the board, a printed boss on top.

| Hole | KiCad | Case | Goes into |
|---|---|---|---|
| right, seat 2 side | 131.4, 78 | 31.4, 22 | top piece |
| left, seat 2 side | 68.6, 78 | -31.4, 22 | top piece |
| right, seat 1 side | 131.4, 122 | 31.4, -22 | top piece |
| left, seat 1 side | 68.6, 122 | -31.4, -22 | top piece |
| seat 2's end | 103, 49.25 | 3.0, 50.75 | seat 2's deck |
| seat 1's end | 97, 150.75 | -3.0, -50.75 | seat 1's deck |

Keep 2.5 mm round each hole's centre free of parts on both faces (check_fit allows nothing within
the boss's radius + 0.3 on top, + 0.5 on the back).

## Buttons (membrane contacts)

| | KiCad |
|---|---|
| SW1-SW4 (P1 up, down, left, right) | 79.3, 142.35 / 79.3, 156.35 / 72.3, 149.35 / 86.3, 149.35 |
| SW5-SW8 (P1 A, B, X, Y) | 128.2, 149.35 / 120.7, 156.85 / 120.7, 141.85 / 113.2, 149.35 |
| SW9-SW12 (P2 up, down, left, right) | 120.7, 57.65 / 120.7, 43.65 / 127.7, 50.65 / 113.7, 50.65 |
| SW13-SW16 (P2 A, B, X, Y) | 71.8, 50.65 / 79.3, 43.15 / 79.3, 58.15 / 86.8, 50.65 |
| SW20, SW21 (P1 Select, Start) | 96.25, 158.45 / 103.75, 158.45 |
| SW22, SW23 (P2 Select, Start) | 103.75, 41.55 / 96.25, 41.55 |

The cluster centres (`pieces`) are 79.3 / 120.7, 149.35 and 50.65: each cluster moved 11.77 mm
towards the middle with the case's end; their pitches are unchanged (14.0, 15.0, 7.5).

## Keep-outs

| Zone | Where (KiCad) | What may be there |
|---|---|---|
| Silicone pieces | a 26 mm circle round each cluster centre; the Start/Select membranes 93..107 x 155.45..161.45 and 38.55..44.55 (`ss_pieces`) | nothing (bare contacts only) |
| Round the clusters | within 1.5 mm of those | 0.5 mm (`retainer_limit`) |
| Panel bay | 77.89..122.11 x 68.65..131.35 (`glass`) | 2.2 mm (`glass_limit`) |
| The folded FPC | 83.36..115.86 x 104.23..131.23 (`fpc_zones`: the FPC's 31.5 width + 0.5, from 1.5 past the seat-1 glass edge to 0.5 past the furthest the tip can land) | nothing over 0.3 mm but J3 |
| **J3's range** | courtyard anywhere in 84.56..114.66 x 104.43..114.33 (`j3_zone`); place it at x 99.61, y 108.18 (`fpc_conn`, the middle of `fpc_conn.y_range` 109.43 to 106.93) | J3; it may move up to 1.25 mm either way along y once a real panel shows where its FPC's tip lands |
| Panel frame | the band between 77.14..122.86 x 68.77..131.23 and 78.24..121.76 x 69.87..130.13 (`frame`) | 2.45 mm |
| Side bezels and spines (the top piece's underside) | everywhere else under the screen face | 3.45 mm (`bezel_limit`) |
| Under the decks and the top piece's step walls | `decks` (six rectangles: each end from y 131.8 / 68.2 outwards, less the spine; the spine's end from 143.695 / 56.305) | 1.6 mm (`deck_limit`) |
| J2 / J4 under the spines | J2 at 100, 139.55 (`batt_conn`), J4 at 100, 60.45 (`spk_conn`), mouths towards the panel; the spine's inside is 8.4 wide (x 95.8..104.2) | 3.4 mm; a pocket in the top piece allows up to 3.6 over J2 (`pocket_limit`) |
| Crush ribs | 1 mm round 135.7 / 64.3, 72 and 128 (the side walls) and 122 / 78, 164.35 and 35.65 (the decks' end walls) (`crush`) | nothing at the board's edge there |
| USB-C | J1 at y 100 on the right edge, its face at 137.55; the tub's slot takes the board's tab | J1 |
| Debug | SW18 BOOTSEL at 68.8, 87 and SW19 RESET at 68.8, 94 under the face's pinholes (were 80 and 88); J5 SWD at 68.3, 102; J6 UART at 68.3, 112 | as placed |
| The back | the cell under 82.5..117.5 x 74.75..125.25 (`cell`), the speaker centred at 75.58, 69 (15 x 10, `speaker`) | 0.5 mm on the back (`back_limit`); through-hole leads only where the floor's ribs are cut away (`rib_free`) |

Height limits in one line: 2.2 under the glass, 3.45 in the bezels and spines, 0.5 round the
clusters, 1.6 under the decks, 0.5 on the back.

## For the silkscreen on the back

The new assembly (SHELL.md, item 15) sticks the cell and the speaker to the board's back before the
board goes face down: please draw the cell's outline (34 x 50, centred at 100, 100) and the speaker's
(a 15 x 10 racetrack centred at 75.58, 69) on the back silkscreen, so they go where the tub's floor
expects them.

## check_fit against today's board

`cd board && python3 ../case/check_fit.py` checks today's board (placed for the 156 mm case)
against the new positions: 34 problems, every one from the board not being re-placed yet (J2 and J4
now under the decks, the side screws at y 78 / 122 instead of 64 / 136, J3 and the parts round it
under the new FPC zone, SW18 on the moved boss, JP5 under a cluster that moved in). The list is in
the session report; it clears when `placement.py` places the board from the new positions.json.
