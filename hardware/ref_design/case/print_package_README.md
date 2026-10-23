# Mega Blastoise: 3D print package

Everything to print for the reference design's case and fit kit, generated from `case/case.scad`
on the current design (2026-10-22: the 77.4 x 132.7 x 9.55 mm shell for the board with the cell in
it, built upside down, its parts keyed to hold together before the screws; since 2026-10-23 the
island: see `case/SHELL.md`, "The island"): the tub, the island (the raised screen block) and two decks.

**This is the 60 degree variant** (2026-10-23): every round, the
steps to the decks and the tub's bottom edge are shaped for a printer that prints a downward face
60 degrees from vertical (30 from the bed) clean, not 45. Print it only once the overhang kit
(`overhang_kit.zip`) has shown your printer does: its 60 degree fin and the R3 round down to its 60
degree tick clean. Otherwise print the main line's kit (`case/SHELL.md`, first section).

## The projects

<!-- printer -->
Each `.3mf` is a Snapmaker Orca project built from your `fit_test_v5.3mf`, so it keeps that
project's printer and settings: Snapmaker U1, 0.20 mm Standard, textured PEI. PLA only, on
toolhead 1 (bay 1, where the PLA is loaded): no PETG, no second toolhead, no prime tower. Open it, check the plate, slice.
(`mega_blastoise_print_P1S.zip` has the same plates sliced for a Bambu Lab P1S.)

Supports are per part, not per plate: only `board_mock` and `coupon_usb_board` get them (in
`1_case`, `4_fit_coupons` and `5_unit`), measured part by part (see below).
<!-- /printer -->

| Project | What's on it | Print it |
|---|---|---|
| `2_buttons_quick.3mf` | **Start here (about an hour).** `tune_openings` (three D-pad opening sizes, the keyed A/B/X/Y cluster, three hole sizes), and the D-pad, A/B/X/Y and Start/Select stacks: a board slice each and a deck slice at the 1.75 mm silicone height | first |
| `3_buttons_heights.3mf` | the same three deck slices at 1.6 and 1.9 mm silicone height | if the 1.75 mm slices don't sit right |
| `0_buttons_end.3mf` | Seat 1's end: tub, board mock (to place the silicone on), the island's end (the step, the spine, the wedge round them), seat 1's deck's core | any time; it also tests the island's wedge under the deck's lip |
| `4_fit_coupons.3mf` | USB-C opening (the tub's tab, the island's ear, the two right arms' ends either side of it with the ear's laps, the board's tab with a receptacle stand-in), a stretch of the left wall's seam (the tub's tongue under seat 2's arm, with a key, grip ribs and a crush rib), where the arms meet on the left wall (the tub, the island's side, both arms' ends with seat 1's lap), the island's panel frame, and seat 1's end | second |
| `1_case.3mf` | the tub, the island, the board mock, and dummies of the panel, speaker, cell and cell plug | last, once the coupons fit |
| `1_case_decks.3mf` | seat 1's and seat 2's decks (two parts since the island: their arms end in different places) and the two build bars the decks lie on while the case is built | with `1_case` |
| `6_tests.3mf` | **Questions for the shell (about 40 min, no supports):** `tune_openings` (as in `2_buttons_quick`), an overhang ladder, two tub screw holes and an island screw boss cut from the real case, and a 5.0 mm column with four clearance rings | print it on both printers |
| `5_unit.3mf` | **One whole unit on one plate:** the tub, the island, both decks (turned a quarter round to fit), the build bars, the board mock and the dummies of the panel, speaker, cell and cell plug to fit in it (no coupons) | instead of `1_case` and `1_case_decks`, once the coupons fit |

The printer tests (the steepest clean overhang, the first layer at the tub's bottom edge, the
slicers' overhang settings) are a kit of their own, `overhang_kit.zip`, with their own README.

The island, the decks and the tuning deck slices print face down as exported (the slices legs
up), the tub back down, everything else flat. `overhangs.py` checked every part for a downward face
flatter than 60 degrees (this variant's limit) that the printer can't bridge (5 mm at most) or step out to (1 mm): only
two parts have one, so only those parts get tree supports from the build plate (each part's own
setting in the project; the plate's are off). Until 2026-10-21 the whole plate had them, and the
slicer also filled the tub's six screw counterbores with support that fused into them: if you
change a plate, keep supports on those parts only. The P1S kit checks its G-code for it
(`case/support_check.py`):
- `board_mock` (in `1_case` and `5_unit`) and `coupon_usb_board` (in `4_fit_coupons`): the USB-C stand-in's
  front stands 1.2 mm out past the board's edge, round-cornered underneath. Supported, so it
  doesn't droop: the opening's fit round it is what the coupon tests.

`stl/` has every part as a plain STL, in case a project won't open in your Orca version: drag them
onto a plate with the same settings. `top.stl` (the island), `deck1.stl`, `deck2.stl` and `tub.stl`
are the real case, each in its print pose.

**The shell (2026-10-21; the island, 2026-10-23).** The case's face and sides are the two decks:
each is a core (the buttons, at one end) and two arms along the side walls, all at the decks' height
(7.7 mm), curving over the sides and down the back's round to the seam with the tub (2.55 mm up). The
screen block is the island, 1.85 mm higher, sitting in the window the decks form, 2.6 mm in from the
sides with one rounded edge all round; under the decks' face its edge slopes out at 45 degrees under a
lip on the decks' edges, so it can't come out of the front. On the right wall it comes out to the side
as an ear over the USB-C (the receptacle is too tall for a wall at the decks' height). On the left wall
the arms meet each other, seat 1's lap behind seat 2's skirt; on the right they meet the ear, whose
laps go behind theirs. The four side screws go into the island beside the glass, one into each deck.
The tub's tongue stands up inside every skirt (4.15 mm round the decks' cores, 3.0 along the side
walls) and keys them: a tab into each arm, a post in each corner that pushes a deck onto the island's
wedge, and grip ribs, so the case holds together before the screws go in. The cell sits in a hole in
the board, its bottom in a pocket in the tub's floor. Everything is built upside down, and nothing
needs supports.

The case's parts, as they print (P1S, 0.20 mm, from its sliced projects):

| Part | Size (mm) | On |
|---|---|---|
| `tub.stl` | 77.4 x 132.7 x 6.7 (the tongue's top round the cores; 5.6 along the sides, the tabs 6.7) | `1_case` (55.1 g, 1 h 43 min with the island, the board mock and the dummies), `5_unit` |
| `top.stl` (the island) | 76.0 x 94.9 x 6.9 (the ear to the right wall) | `1_case`, `5_unit` |
| `deck1.stl`, `deck2.stl` | 77.4 x 67.4 x 5.05 and 77.4 x 68.3 x 5.05 (seat 1's lap reaches 3.15 past its left arm) | `1_case_decks` (10.1 g, 25 min for both and the bars), `5_unit` |
| `build_bar.stl` (print two) | 71.0 x 2.0 x 1.85 | `1_case_decks`, `5_unit` |

`5_unit` (the tub, the island, both decks, the bars, the board mock and the dummies on one plate) is 65.1 g and 2 h 08 min.

To assemble (every part goes in straight down):
1. The two build bars on a flat bench, parallel, about 126 mm apart (centre to centre).
2. Seat 2's deck face down on them, its end's flat face on one bar: its face now stands 1.85 mm off
   the bench. Then seat 1's deck face down on the other bar, straight down beside it: the lap on its
   left arm goes down behind seat 2's arm's skirt. Caps into the decks from behind (tabs into the
   slots, notches onto the ribs), silicone on them.
3. The speaker face down into the island's pocket over the grille, its front foam ring stuck to the
   pocket's ceiling.
4. The island face down into the window between the decks from behind, straight down: the laps at
   the ear's ends go down behind the right arms' skirts, its wedge settles onto the decks' lips, and its
   face comes down onto the bench.
5. Panel glass down into the island's pocket on its tape.
6. The cell into the tub's floor pocket on double-sided tape, against the pocket's right wall.
7. Board on edge at seat 1's end: the FPC into J3 (close the latch), the cell's plug into J2.
8. Board face down towards the bosses. Just before it lands, the speaker's plug into J4 (by the
   right edge, under the speaker; its mouth faces the glass): the speaker stays in the island, and
   its 35 mm leads reach J4 only when the board is nearly in place. Lay the leads round the screw boss
   between the speaker and J4, on the glass's side, then the board down onto the bosses.
9. Tub over it all, straight down, the board lying centred on its bosses (the cell drops into the
   board's hole, the tub's crush ribs centre the board, the tongue slides into the skirts, its
   tabs into the arms' pockets, the corner posts push each deck onto the island's wedge). Press it
   down all round until the seam is an even line: it grips, and the case now holds together on its
   own. Lift it off the bars. Six M2 x 6 from the back, snug: they lock it.

Opening it: screws out, ease the tub off (it grips: straight up, an end at a time), then lift the
board only a little and unplug J4 before it comes away: the island lifts out of the decks with the
speaker stuck in it.

Check that the decks and the island go together straight down without force; that with the tub on
and no screws the case lifts by its ends without anything coming loose and nothing rocks; that the
tub still goes on and comes off by hand; that the seam round the case is an even thin line with the
outsides flush; that the seam round the island's foot is an even 0.15 and the island doesn't move
out of the front; that the two arms meet flush on the left wall and the ear's ends sit flush on the
right; and that the arms don't spring out from the tub along the sides.

## What to check, and what to change

Numbers live in `case/case.scad`; after changing one, rebuild the packages
(`python3 case/export.py --stl`, `python3 case/fitkit.py`, `python3 case/print_package.py`,
`python3 case/print_package_p1s.py`).

**Buttons (`2_buttons_quick`):**
- D-pad in each of the three `tune_openings` crosses (gaps 0.55, 0.65, 0.75 mm, engraved): pick the
  tightest it moves freely in. Now `dpad_gap` = 0.65 (0.5 was too tight in the first print, 0.8 too loose in the next).
- A, B, X, Y into the keyed cluster from the back: tab into its slot, each notch onto its rib, and
  no cap fits another cap's hole. A rib fighting a cap: note the angle, fix `abxy_notches` (or
  `notch_keys = false`). The keyed holes are 8.0 (`cap_hole_d`, Alex's DS Lite faceplate's; 7.6 was too tight, 2026-10-21);
  the plain holes (7.6, 7.8, 8.2) say whether it can be tighter or must be looser.
- Each stack: silicone on the board slice, caps in the deck slice, the slice on its four legs. The cap
  should stand about 1.4 mm above the deck (1.41, the faceplate's 1.6 mm lip), press cleanly and spring back. If the 1.75 mm slice is
  wrong, try the 1.6 and 1.9 slices (`3_buttons_heights`) and set `dome_h`; `dpad_lift` moves only
  the D-pad.
- **Start/Select:** their membrane, pill spacing and caps are estimates (`ss_mem` 14 x 6 mm,
  `ss_pitch` 7.5, `ss_cap_d` 3.2 from the faceplate's 3.8 mm hole). Measure your real Start/Select membrane and caps and send the
  numbers; the stack shows whether the estimates already work.

**Test plate (`6_tests`), for the shell:** print it on the P1S and the U1.
- Openings: `tune_openings` again, with the D-pad (crosses 0.55, 0.65, 0.75) and A/B/X/Y caps (the
  keyed cluster is 8.0, the plain holes 7.6, 7.8, 8.2): the tightest the D-pad moves freely in, and
  the tightest A/B/X/Y hole a cap drops through without catching.
- Overhangs (`test_overhangs`, printed with no supports on purpose): the fins' undersides slope 45
  to 75 degrees from vertical (the number on each fin's top). Report the steepest fin whose
  underside is still clean (no drooping or loose strands) on each printer. The quarter round at the
  ladder's end goes from 85 degrees (near flat, where it leaves its stem) up to vertical, ticked
  at 45, 60 and 75 on its end face: report the tick below which it turns rough. That number goes
  into `overhang` in `case.scad` (60 on this variant, 45 on the main line): one line makes the steps and every edge rounder.
- Screw holes: `coupon_screws_tub` (two of the tub's holes, printed flat as the tub: H4's slimmed
  5.0 boss by the left wall and an end boss) should come out
  with the 4.4 mm counterbore and the 2.4 mm hole open, and an M2 screw should pass; the old plates
  had supports in every counterbore (see above). `coupon_screws_top` (an island boss, face down)
  should take an M2 x 6 self-tapper into its 1.6 mm pilot. Say which of the case's holes were filled.
- Column clearance: slide each ring of `test_rings` over `test_column` (5.0 mm, as the bosses):
  report the tightest that slides on freely and the loosest that doesn't rattle. (The new decks have
  their own bosses, so no column passes through another part now; the answer is for later.)

**Fit coupons (`4_fit_coupons`):**
- USB-C: the two arm coupons (`coupon_usb_arms`) face down, the island's ear coupon face down
  between them (its laps go down behind the arms' skirts), the board coupon face down into it, the
  tub coupon over it (its tab rises into the ear's notch). The opening should hug the receptacle
  stand-in with a small even gap (`usb_gap` 0.25), a real plug's overmold should sit flat on the
  case, and the ear's ends should sit flush with the arms'.
- Seam: lower `coupon_seam_deck` (seat 2's left arm) over `coupon_seam_tub`: the skirt slides over the
  tongue without force, the key tab into its pocket, and sits flush with the tub (`seam_fit`,
  `seam_gap`); the grip ribs should hold the two together lightly (`grip_reach`: raise it if they fall
  apart, lower it if they won't go together by hand).
- Where the arms meet (`coupon_meet_*`): the two arm pieces face down, seat 2's first, seat 1's lap
  down behind its skirt; the island's side face down beside them (its wedge under their lips); the tub
  coupon over them: the two skirts flush, no light through the joint.
- Panel frame: the real panel (or the dummy) drops in glass down with a little play, the active
  area centred in the window (`panel_fit`, `window_margin`).
- Seat 1's end, built upside down: the deck coupon face down on a build bar, caps and silicone, the
  island's end face down into it (its wedge settles under the deck's lip across the step and round the
  spine without force: `deck_gap`), the board mock face down, the tub coupon: its corner posts push
  the deck onto the wedge, the seam at the step an even line.
- Speaker (in `1_case`): the dummy (or the real speaker) drops face down into the island's
  pocket over the grille without play to speak of, and the cell dummy into the board mock's hole
  and the tub's pocket.

**The whole case (`1_case` and `1_case_decks`, or `5_unit`):** build it upside down as above. It
should measure 9.55 mm at the screen and 7.7 mm at the decks, the seam flush all round, and
nothing should rattle. Press each grille slot's rim: the speaker should sit flat behind it.
