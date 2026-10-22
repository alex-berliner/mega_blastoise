# Mega Blastoise: 3D print package

Everything to print for the reference design's case and fit kit, generated from `case/case.scad`
on the current design (2026-10-21: the 133 mm shell for the 2.4 in panel, built upside down; see
`case/SHELL.md`): the tub, the top piece and two decks.

## The projects

<!-- printer -->
Each `.3mf` is a Snapmaker Orca project built from your `fit_test_v5.3mf`, so it keeps that
project's printer and settings: Snapmaker U1, 0.20 mm Standard, textured PEI. PLA only, on
toolhead 2: no PETG, no second toolhead, no prime tower. Open it, check the plate, slice.
(`mega_blastoise_print_P1S.zip` has the same plates sliced for a Bambu Lab P1S.)

Supports are per part, not per plate: only `board_mock`, `coupon_usb_board` and `speaker_dummy`
get them (in `1_case`, `4_fit_coupons` and `5_unit`), measured part by part (see below).
<!-- /printer -->

| Project | What's on it | Print it |
|---|---|---|
| `2_buttons_quick.3mf` | **Start here (about an hour).** `tune_openings` (three D-pad opening sizes, the keyed A/B/X/Y cluster, three hole sizes), and the D-pad, A/B/X/Y and Start/Select stacks: a board slice each and a deck slice at the 1.75 mm silicone height | first |
| `3_buttons_heights.3mf` | the same three deck slices at 1.6 and 1.9 mm silicone height | if the 1.75 mm slices don't sit right |
| `0_buttons_end.3mf` | Seat 1's whole end: tub, board mock (to place the silicone on), the top piece's end (the step, the spine, the undercut), seat 1's deck | any time; it also tests the deck's joint |
| `4_fit_coupons.3mf` | USB-C opening (the tub's tab, the top piece's skirt, the board's tab with a receptacle stand-in), a stretch of the seam (the tub's tongue under the top piece's skirt, a crush rib), the top piece's panel frame, and seat 1's whole end | second |
| `1_case.3mf` | the tub, the top piece, the board mock, and dummies of the panel, speaker, cell and cell plug | last, once the coupons fit |
| `1_case_decks.3mf` | the two decks (the same part, one for each end) | with `1_case` |
| `6_tests.3mf` | **Questions for the shell (about 40 min, no supports):** `tune_openings` (as in `2_buttons_quick`), an overhang ladder, two tub screw holes and a top piece screw boss cut from the real case, and a 5.0 mm column with four clearance rings | print it on both printers |
| `7_overhangs_020.3mf`, `_016`, `_012`, `_008` | **The steepest overhang each printer prints clean (about 20, 23, 28 and 40 min, 6 g each, no supports):** an overhang ladder and three convex rounds, the same parts at 0.20, 0.16, 0.12 and 0.08 mm layers (see "Overhang kit" below) | print all four on both printers |
| `8_first_layer_020.3mf`, `8_first_layer_012.3mf` | **The lip at the tub's bottom edge (about 6 min, 3 g each):** two corners of the tub's bottom, its real round from the bed, at elephant's-foot compensation 0.15 and 0.25 mm; the first layer 0.20 mm on one plate, 0.12 on the other (see "First layer" below) | both, on both printers |
| `5_unit.3mf` | **One whole unit on one plate:** the tub, the top piece, both decks and the board mock (no coupons, no dummies) | instead of `1_case` and `1_case_decks`, once the coupons fit |

The top piece, the decks and the tuning deck slices print face down as exported (the slices legs
up), the tub back down, everything else flat. `overhangs.py` checked every part for a downward face
flatter than 45 degrees that the printer can't bridge (5 mm at most) or step out to (1 mm): only
three parts have one, so only those parts get tree supports from the build plate (each part's own
setting in the project; the plate's are off). Until 2026-10-21 the whole plate had them, and the
slicer also filled the tub's six screw counterbores with support that fused into them: if you
change a plate, keep supports on those parts only. The P1S kit checks its G-code for it
(`case/support_check.py`):
- `board_mock` (in `1_case` and `5_unit`) and `coupon_usb_board` (in `4_fit_coupons`): the USB-C stand-in's
  front stands 1.2 mm out past the board's edge, round-cornered underneath. Supported, so it
  doesn't droop: the opening's fit round it is what the coupon tests.
- `speaker_dummy` (in `1_case`): its solder tab and lead exit stick out 1.9 and 2.4 mm.

`stl/` has every part as a plain STL, in case a project won't open in your Orca version: drag them
onto a plate with the same settings. `top.stl`, `deck.stl` (print two) and `tub.stl` are the
real case, each in its print pose.

**The shell (2026-10-21).** The top piece (the screen face, the curved steps, the sides) and the
two decks (the button faces) curve over the sides and meet the tub flush at one seam low on the
walls; each deck seats into the top piece from behind on a 45 degree wedge and has its own screw.
Everything is built upside down, and nothing needs supports. To assemble:
1. Board face up: the cell's plug up through its hole into J2, the speaker's into J4; the cell on
   the board's back on its foam pad, the speaker by its back on thin tape (outlines on the back).
2. Top piece face down on a cloth. Each deck face down into its end from behind: it drops onto its
   wedges and sits flat.
3. Caps into the decks from behind (tabs into the slots, notches onto the ribs), silicone on them.
4. Panel glass down into the top piece's pocket on its tape. Slide its FPC into J3 with the board
   held on edge at seat 1's end; close the latch.
5. Board face down onto the bosses (the crush ribs centre it). Tub over it all. Six M2 x 8 from the
   back, snug.

Check that each deck drops in without force and doesn't rock, that the seam round the case is an
even thin line with the outsides flush, and that the seam in the crease at each step's foot looks
right.

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
  into `overhang` in `case.scad` (45 now): one line makes the steps and every edge rounder.
- Screw holes: `coupon_screws_tub` (two of the tub's holes, printed flat as the tub) should come out
  with the 4.4 mm counterbore and the 2.4 mm hole open, and an M2 screw should pass; the old plates
  had supports in every counterbore (see above). `coupon_screws_top` (a top piece boss, face down)
  should take an M2 x 8 self-tapper into its 1.6 mm pilot. Say which of the case's holes were filled.
- Column clearance: slide each ring of `test_rings` over `test_column` (5.0 mm, as the bosses):
  report the tightest that slides on freely and the loosest that doesn't rattle. (The new decks have
  their own bosses, so no column passes through another part now; the answer is for later.)

**Overhang kit (`7_overhangs_020`, `_016`, `_012`, `_008`), for the shell's curves:** print all four on
the P1S and the U1, supports off (they're off in the projects: these parts overhang on purpose). The
four plates are the same two parts and the same settings except the layer height, which is in the
plate's name and engraved on each part's pad (0.20, 0.16, 0.12, 0.08), so the prints can't be mixed up.
Each project names the slicer's process preset at its layer height (P1S: 0.20mm Standard, 0.16mm
Optimal, 0.12mm Fine, 0.08mm Extra Fine @BBL X1C, Bambu's presets for the P1S; U1: 0.20, 0.16, 0.12,
0.08mm Standard @Snapmaker U1) and lists what it changes from it, so it opens at its own layer height
(the process shows as modified): check the layer height in the process tab before slicing.
- `test_overhang_ladder`: fifteen fins side by side, each a 2 mm wall (the case's) whose underside
  leaves a stem 2 mm up and slopes out 8 mm at its angle from vertical: 30, 35, 40, 45, 50, then 52.5
  to 70 every 2.5, 75, 80 (the number on each fin's top; 30 to 50 reach less, to stay low). Every fin
  reaches both ways: towards the front of the bed (the door: the side of the pad engraved F) and
  towards the back (B). The P1S's part fan should blow from the toolhead's front, so F is its fan
  side; check which side the U1's duct is on.
- `test_overhang_rounds`: convex quarter rounds R10, R6 and R3, as the shell's edges print (face down,
  the round leaving the bed flat and turning up towards vertical), from 80 degrees from vertical at
  the stem up to 25, both ways (F and B as above). Ticks on each round's end face (the face away from
  the pad), in along the surface: every 5 degrees on R10, every 10 on R6 and R3, from 30 to 80; the
  long ticks are 30, 50 and 70 (numbered on R10).

Clean means: no sag (the underside within about 0.3 mm of a straight line, or of the round), no
edge curled up that the nozzle would hit (curl shows as a raised lip at the fin's tip or the round's
edge, and you may have heard the nozzle tick over it), and a smooth face, no loose loops or strands.
On the ladder, start at 45 (clean everywhere) and go up until a fin isn't; read each side. On a
round, read from vertical down towards the stem: the tick where the surface starts to sag or roughen.

Why four layer heights: each layer steps out layer height x tan(angle) past the one under it, so at
60 degrees 0.35 mm at 0.20, 0.28 at 0.16, 0.21 at 0.12 and 0.14 at 0.08; a thinner layer should hold a
steeper angle, for a longer print. Compare the four parts from one printer side by side, the same fin and
the same tick on each. The case's curves can then take the layer height and the angle together:
`overhang` in `case.scad` (45 now) is the angle, and the shell prints at that plate's layer height
(the steeper the angle, the rounder the steps and edges).

Report, per printer and per layer height: the steepest clean fin, F side and B side (e.g. "P1S 0.16:
F 62.5, B 60"), the steepest clean tick on each round (e.g. "R10 65, R6 60, R3 60"), and a photo of
the undersides if anything is borderline.

To build just these (after `python3 case/fitkit.py test_overhang_`): `python3 case/print_package.py
7_overhangs_020 7_overhangs_016 7_overhangs_012 7_overhangs_008`, and the same names to `case/print_package_p1s.py`
(a run naming plates leaves only those plates in its package folder and zip).

**First layer (`8_first_layer_020`, `8_first_layer_012`), for the lip at the tub's bottom edge:**
each coupon is a 25 mm corner of the tub's bottom, 3 mm tall, with the tub's real bottom-edge round
on its two outer sides (as `tub.stl` prints, floor down). The engraving on top says how it was printed:
`L20` or `L12` is the first layer, 0.20 or 0.12 mm (a plate's setting in both slicers, so one plate
each; the layers above are 0.20), and `E15` or `E25` the elephant's-foot compensation, 0.15 or 0.25
mm (each object's own setting, in the project). The packages print 0.15 now, with a 0.20 mm first
layer on the P1S and 0.25 on the U1 (Alex's template). Run a fingernail up each coupon's round from
the bed, and lay a straightedge on the bed against it: the best pair has no ridge or flare at the
bottom, and the round meets the bed cleanly, with no step. Report the best pair per printer (e.g.
"P1S: L12 E25"); the packages' first layer and compensation then take it.

**Fit coupons (`4_fit_coupons`):**
- USB-C: board coupon face down into the top coupon, the tub coupon over it (its tab rises into the
  skirt's notch). The opening should hug the receptacle stand-in with a small even gap (`usb_gap`
  0.25), and a real plug's overmold should sit flat on the case.
- Seam: lower `coupon_seam_top` over `coupon_seam_tub`: the skirt slides over the tongue without
  force and sits flush with the tub (`seam_fit`, `seam_gap`).
- Panel frame: the real panel (or the dummy) drops in glass down with a little play, the active
  area centred in the window (`panel_fit`, `window_margin`).
- Seat 1's end, built upside down: the top coupon face down, the deck into it (it should drop onto
  its wedges and sit without rocking: `deck_gap`), caps and silicone, the board mock face down, the
  tub coupon. The cell plug (real or dummy) passes up through the board mock's hole and into J2.

**The whole case (`1_case` and `1_case_decks`, or `5_unit`):** build it upside down as above. It
should measure 13.35 mm at the screen and 11.5 mm at the decks, the seam flush all round, and
nothing should rattle.
