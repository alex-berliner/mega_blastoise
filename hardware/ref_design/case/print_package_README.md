# Mega Blastoise: 3D print package

Everything to print for the reference design's case and fit kit, generated from `case/case.scad` on the current design
(2026-10-26: the 77.4 x 156.64 x 9.6 mm shell for the ER-TFT035IPS-6 and the board with the cell in it, built upside down: a **tub** and **one top piece**
whose screen block is a full-width band with the DS Lite's slope at its two ends; see `case/SHELL.md`). **The case kits hold only the case:** `1_case` (the tub and
the top) and `5_unit` (the whole unit with its dummies); the test and coupon plates are not in them. Numbers below that say 132.7 or 9.55 are the 2.4 in case's.

**This is the 60 degree variant** (2026-10-23): the tub's bottom edge and the skirts' round are shaped for a printer that prints a
downward face 60 degrees from vertical (30 from the bed) clean, not 45. (The top's own outside no longer follows that limit: it
prints on support filament.) Print it only once the overhang kit (`overhang_kit.zip`) has shown your printer does: its 60 degree fin
and the R3 round down to its 60 degree tick clean. Otherwise set `overhang = 45` in `case.scad` and rebuild the kits (both parts
still build and every assert passes at 45).

## Filaments: two, since the one-piece top

| | P1S | U1 |
|---|---|---|
| **PLA** (every part) | **AMS slot 1**: Bambu PLA Basic | **toolhead 1** (bay 1, where the PLA is loaded) |
| **Support filament** (the top's supports' interface only) | **AMS slot 2**: Bambu Support For PLA | **toolhead 2**: Bambu Support For PLA (or any PLA-compatible support filament) |

The top prints face down with its ends 1.85 mm above the bed, so its slopes, side rounds and the ends' faces hang over it and need
supports. They are tree supports with PLA for the body and **Bambu Support For PLA for the layers that touch the part** (top Z
distance 0), so the faces come off clean and the support peels off by hand. Only the plates that have the top, or a coupon cut from it
that includes its slope or rounds, use the second filament and carry a prime tower: `0_buttons_end`, `1_case`, `4_fit_coupons` and
`5_unit`. Every other plate is PLA only, slot 2 untouched, no tower. The tub prints back down with **no supports**.

<!-- printer -->
Each `.3mf` is a Snapmaker Orca project built from your `fit_test_v5.3mf`, so it keeps that
project's printer and settings: Snapmaker U1, 0.20 mm Standard, textured PEI. PLA on toolhead 1 and Bambu Support For PLA on
toolhead 2 (the template's slot 2's settings, marked as a support filament, in slot 2; slot 1 takes the PLA settings: no PETG), and on a plate with the top a
prime tower at the back right of the bed. Open it, check the plate, slice.
(`mega_blastoise_print_P1S.zip` has the same plates sliced for a Bambu Lab P1S, with their weights and times.)
Print times and weights below are Snapmaker Orca 2.4.1-beta's, sliced from the command line on the build machine (0.08 mm fine layers on
the release plates, outer wall 60, inner wall 120 mm/s; the weights are what the slicer reports, the prime tower and the purge included).
The projects stay unsliced: open one, check the plate, slice it yourself. **Use Snapmaker Orca 2.4.1-beta or later**: 2.4.0 opens these
projects, but its tree supports in the default style count as Organic and Orca refuses Organic supports with the fine layers, and
`tools/build.py --only print_u1` needs 2.4.1-beta's command line (2.4.0 crashes in it).

| Project | Supports | PLA | Support filament | Tool changes | Time | Parts |
|---|---|---|---|---|---|---|
<!-- u1 -->
| `0_buttons_end.3mf` | buttons_top: tree, interface on toolhead 2 | 19.8 g | 2.1 g | 16 | 1 h 26 min | buttons_tub, buttons_board, buttons_top |
| `1_case.3mf` | top: tree, interface on toolhead 2 | 45.7 g | 2.7 g | 24 | 4 h 03 min | tub_print, top |
| `2_buttons_quick.3mf` | none | 18.0 g | 0.0 g | 0 | 0 h 59 min | tune_openings, tune_stack_dpad_board, tune_stack_abxy_board, tune_stack_ss_board, tune_stack_dpad_lid_dome1.75, tune_stack_abxy_lid_dome1.75, tune_stack_ss_lid_dome1.75 |
| `3_buttons_heights.3mf` | none | 8.1 g | 0.0 g | 0 | 0 h 35 min | tune_stack_dpad_lid_dome1.6, tune_stack_dpad_lid_dome1.9, tune_stack_abxy_lid_dome1.6, tune_stack_abxy_lid_dome1.9, tune_stack_ss_lid_dome1.6, tune_stack_ss_lid_dome1.9 |
| `4_fit_coupons.3mf` | usb_top, seam_top, buttons_top: tree, interface on toolhead 2; usb_board: plain PLA | 26.3 g | 2.2 g | 16 | 2 h 02 min | usb_tub, usb_top, usb_board, seam_tub, seam_top, frame_top, buttons_tub, buttons_board, buttons_top |
| `5_unit.3mf` | top: tree, interface on toolhead 2; board_mock: plain PLA | 78.8 g | 2.7 g | 24 | 6 h 17 min | tub_print, top, board_mock, panel_dummy, speaker_dummy, cell_dummy, cell_plug_dummy |
| `6_tests.3mf` | none | 15.7 g | 0.0 g | 0 | 0 h 59 min | tune_openings, test_overhangs, screws_tub, screws_top, test_column, test_rings |
<!-- /u1 -->

Supports are per part, not per plate: the top and the coupons cut from it (`coupon_seam_top`, `coupon_usb_top`, `coupon_buttons_top`)
get the two-filament tree supports above; `board_mock` and `coupon_usb_board` get plain PLA trees (the USB-C stand-in); nothing else
has any (see below).
<!-- /printer -->

| Project | What's on it | Print it |
|---|---|---|
| `2_buttons_quick.3mf` | **Start here (about an hour).** `tune_openings` (three D-pad opening sizes, the keyed A/B/X/Y cluster, three hole sizes), and the D-pad, A/B/X/Y and Start/Select stacks: a board slice each and a slice of the top's end at the 1.75 mm silicone height | first |
| `3_buttons_heights.3mf` | the same three end slices at 1.6 and 1.9 mm silicone height | if the 1.75 mm slices don't sit right |
| `0_buttons_end.3mf` | Seat 1's end: tub, board mock (to place the silicone on), the top's end (the DS Lite slope, the end, the buttons' openings and walls) | any time; it tests the slope and the button stack under the real shell. Two filaments |
| `4_fit_coupons.3mf` | USB-C opening (the tub's tab, the top's wall over the port, the board's tab with a receptacle stand-in), a stretch of the left wall's seam (the tub's tongue with a key, grip ribs and a crush rib; the top's skirt over it), the top's panel frame, and seat 1's end | second. Two filaments |
| `1_case.3mf` | the tub and the top | last, once the coupons fit. Two filaments |
| `6_tests.3mf` | **Questions for the shell (about 40 min, no supports):** `tune_openings` (as in `2_buttons_quick`), an overhang ladder, two tub screw holes and a top screw boss cut from the real case, and a 5.0 mm column with four clearance rings | print it on both printers |
| `5_unit.3mf` | **One whole unit on one plate:** the tub, the top, the board mock and the dummies of the panel, speaker, cell and cell plug to fit in it (no coupons) | instead of `1_case`, once the coupons fit. Two filaments |

The printer tests (the steepest clean overhang, the first layer at the tub's bottom edge, the
slicers' overhang settings) are a kit of their own, `overhang_kit.zip`, with their own README.

The top and its coupons print face down as exported (the tuning slices legs up), the tub back down, everything else flat.
`overhangs.py` checked every part for a downward face flatter than 60 degrees (this variant's limit) that the printer can't bridge
(5 mm at most) or step out to (1 mm): the top, the three coupons cut from it that have its slope or rounds, and the board mock's USB-C
stand-in (and its coupon) have such faces; nothing else does, **and the build fails if any other part gets one** (or if the
support filament prints anything but the top's interface layers and the prime tower: `case/support_check.py` reads the sliced
G-code). The flat coupons cut from the top (`coupon_frame_top`, `coupon_screws_top`, the tuning slices) stop short of the slope
and the end's round for that reason. Until 2026-10-21 the whole plate had supports, and the slicer also filled the tub's six screw
counterbores with support that fused into them: if you change a plate, keep supports on the parts above only.
- `board_mock` (in `5_unit`) and `coupon_usb_board` (in `4_fit_coupons`): the USB-C stand-in's front stands 1.2 mm out past the
  board's edge, round-cornered underneath. Plain PLA supports, so it doesn't droop: the opening's fit round it is what the
  coupon tests.

`stl/` has every part as a plain STL, in case a project won't open in your slicer: drag them
onto a plate with the same settings. `top.stl` and `tub.stl` are the real case, each in its print pose.

**The shell (2026-10-21; the one-piece top, 2026-10-25).** The top is the case's face and sides in one piece: a band at the screen's
height (9.6 mm) across the whole width, its two ends the DS Lite's lower screen frame in section, and beyond them the ends at the
buttons' height (7.7 mm), curving over the sides and down the back's round to the seam with the tub (2.55 mm up). The tub's tongue
stands up inside its skirt all round (4.05 mm round the ends, 3.03 along the side walls) and keys it: two tabs into pockets in each
side wall and grip ribs, so the two parts hold together before the screws go in. The four side screws go into the top beside the
glass, one more at each end. The cell sits in a hole in the board, its bottom in a pocket in the tub's floor. Everything is built
upside down.

The case's parts, as they print (P1S, 0.20 mm, from its sliced projects):

| Part | Size (mm) | On |
|---|---|---|
| `tub.stl` | 77.4 x 156.64 x 6.7 (the tongue's top round the ends; 5.6 along the sides, the keys' tabs 6.7) | `1_case`, `5_unit` |
| `top.stl` | 77.4 x 156.64 x 7.0 (face down: the screen face on the bed, the ends 1.85 above it) | `1_case`, `5_unit` |

`1_case` (the tub and the top, a release plate) is 45.7 g of PLA and 2.7 g of support filament, with 24 filament changes, in 4 h 03 min (U1, Snapmaker Orca 2.4.1-beta, 2026-10-26; the earlier 2.4 in case: 49.7 g, 4 h 27 min on a P1S); `5_unit` (the
tub, the top, the board mock and the dummies on one plate, which fits both beds) is 75.6 g and 5.8 g in 6 h 22 min. The weights include the
purge (the flush at each change and the prime tower: 11.4 g of PLA and 4.0 g of support filament on `1_case`). Those are with the print
settings below; before them (2026-10-25, 2 walls, 200 mm/s outer wall, 0.20 mm layers) `1_case` was 47.1 g and 6.9 g in 2 h 10 min.

To assemble (every part goes in straight down; `docs/assembly.md` has the full steps):
1. Pull the supports off the top. It lies face down on a flat bench, on its screen face; the ends stand 1.85 mm off the bench, so the
   caps clear it.
2. The speaker face down into its pocket over the grille, its front foam ring stuck to the pocket's ceiling. Caps into the top's
   ends from behind (tabs into the slots, notches onto the ribs), silicone on them.
3. Panel glass down into the top's pocket on its tape.
4. The cell into the tub's floor pocket on double-sided tape, against the pocket's right wall.
5. Board on edge at seat 1's end: the FPC into J3 (close the latch), the cell's plug into J2.
6. Board face down towards the bosses. Just before it lands, the speaker's plug into J4 (by the right edge, under the speaker; its
   mouth faces the glass): the speaker stays in the top, and its 35 mm leads reach J4 only when the board is nearly in place. Lay
   the leads round the screw boss between the speaker and J4, on the glass's side, then the board down onto the bosses.
7. Tub over it all, straight down, the board lying centred on its bosses (the cell drops into the board's hole, the tub's crush ribs
   centre the board, the tongue slides into the skirt's groove, its keys into the pockets in the top's sides). Press it down all
   round until the seam is an even line: it grips, and the case now holds together on its own. Six M2 x 6 from the back, snug: they
   lock it.

Opening it: screws out, ease the tub off (it grips: straight up, an end at a time), then lift the board only a little and unplug J4
before it comes away.

Check that the tub goes on straight down without force and comes off by hand; that with the tub on and no screws the case lifts by
its ends without anything coming loose and nothing rocks; that the seam round the case is an even thin line with the outsides flush;
and that the finish where the support interface touched the top is what you want (print `coupon_buttons_top` first).

## Print settings (2026-10-25)

The case plates (every plate but the printer tests in `overhang_kit.zip`, which ask what the preset does) are sliced with these over
the slicers' 0.20 mm Standard. Each is a named, commented setting: `CASE_SETTINGS`, `FINE_LAYERS` and `layer_ranges_xml` in
`print_package.py` (the same names and values for the P1S and the U1), `P1S_FLUSH*` and `P1S_ELEPHANT_FOOT` in `print_package_p1s.py`.
The P1S plates are sliced and measured; the U1's projects get the same settings but can't be sliced here (Orca's command line crashes
on a U1 preset), so their numbers are the P1S's.

| Setting | Was | Now | Why |
|---|---|---|---|
| Outer wall speed | 200 mm/s (U1: 200, 500) | **60** | the outer wall is the surface, so it runs slow (Alex's tip; 100 was compared, below) |
| Inner wall speed | 300 / 400 (U1: 300, 600) | **120** | Alex's tip. The profile's speed was capped by PLA's 21 mm3/s melt limit anyway, so 120 is a little slower (5 to 8 min) for no loss |
| Small perimeters | none (threshold 0) | loops under 15 mm at 50 % | the screw pilots, counterbores and round openings |
| Walls | 2 | **3** | the fits and the thin top are all walls |
| Infill | 15 % grid | 15 % **gyroid** | no crossings to bump the walls |
| Wall order | inner, outer | **inner, outer, inner** | the outer wall is held between two walls, so the fits keep their size; the support interface's surface is a bottom surface, which a wall order doesn't touch (support_check passes, interface extrusion 101 m before, 115 m now with the finer layers) |
| Seam | aligned | **back** | the long walls show no seam line; it sits at the +Y end of each part, on a plate's back |
| Elephant's foot | P1S 0.25 (U1 0.15) | **unchanged** | Alex's first-layer test (2026-10-22) found 0.15 left a lip on the tub's edge and 0.25 didn't, so it isn't lowered to the 0.1 to 0.15 asked for; `overhang_kit.zip`'s `8_first_layer_*` plates answer it again |
| Layer height | 0.20 | **0.08 on the release plates, 0.12 on the test plates**, over the first 4.5 mm of the top (and the coupons cut from it) and 2.5 mm of the tub, 0.20 above | the DS curve and side rounds rise from the bed and a 0.20 mm layer steps them 0.35 mm (0.12: 0.21 mm, 0.08: 0.14 mm). Release plates (`RELEASE_PLATES`: `1_case`, the case itself, and `5_unit`, the whole unit) are what you keep, so they get the finest the P1S profile takes (0.08 is its `min_layer_height`); the test plates (`0_buttons_end`, `4_fit_coupons`) only have to fit, so 0.12 saves 35 min on `0_buttons_end` (2 h 00 against 2 h 35 at 0.08) and 49 min on `4_fit_coupons` (2 h 35 against 3 h 24). Bambu Studio's and Orca's own per-object height ranges (`Metadata/layer_config_ranges.xml`), as adaptive layer height isn't in a project |
| Flush multiplier (P1S, two filaments) | 1 | **0.7** | the profile's flush is 230 mm3 from PLA to the support filament and 700 back; 0.6 to 0.8 was asked for, the middle is kept (0.6 would be less purge, but a white support filament left in a dark PLA shows) |
| Brim, mouse ears | auto brim (none was drawn) | unchanged | the first layer is 2465 mm2 for the top (a 77 x 133 mm frame) and 8924 mm2 for the tub, 6.9 mm tall: nothing to lift |

What the slicer wouldn't take: a plate with a prime tower needs every object on it at the same layer height ranges (Bambu Studio:
"The prime tower is only supported if all objects have the same variable layer height"), so on `0_buttons_end`, `1_case`,
`4_fit_coupons` and `5_unit` the tub and every other object get the top's fine layers to 4.5 mm too. (Snapmaker Orca compares the objects'
layer height profiles element by element, the z of each object's top included, so on the U1 the ranges are also split at every object's top, each
piece 0.0002 mm off its neighbours' height, which makes each object's profile a prefix of the tallest's; `layer_ranges_xml` in `print_package.py`.) The fine layers are the
cost (the release plates' 0.08; the test plates' 0.12 costs less): each layer of the top's slope carries support interface in the second filament, so the changes go from 9 to 21 and the purge
and the prime tower grow with them (at 0.08 mm the tower flushes more per layer and its G-code left the P1S's bed at x 210, so the P1S's
tower stands at x 200, `TOWER_AT`; the U1's tower, sliced at 0.08 mm in Snapmaker Orca 2.4.1-beta, spans 54 mm with its ribs and brim and left the bed at x 274 from x 223, so it stands at x 210, `TOWER_AT`). `1_case` with only 0.20 mm layers took 2 h 19 min
(46.0 g, 6.4 g, 9 changes). To drop them, empty
`FINE_LAYERS` in `print_package.py`. Flushing into the objects' infill (`flush_into_infill`, `flush_into_objects`) made the support
filament print part of the walls and infill, which the build refuses, so they stay off; `flush_into_support` is on in the profile
already and made no difference to the weights; a narrower prime tower (25 mm instead of 35) changed nothing either, so it stays 35.

The fine layer height and the wall speeds, compared on `1_case` and `5_unit` (P1S, the other settings as above, 2026-10-25;
layers in the fine zone are those up to 4.5 mm; support_check passes on all of them, the support filament printing interface and the tower only):

| Plate | Variant | PLA | Support filament | Purge (PLA + support) | Changes | Time | Fine-zone layers |
|---|---|---|---|---|---|---|---|
| `1_case` | before all this | 47.1 g | 6.9 g | 7.4 + 2.0 g | 9 | 2 h 10 | 22 |
| | (c) 0.12, outer 100, inner 300 | 48.6 g | 5.8 g | 9.1 + 2.9 g | 17 | 3 h 04 | 36 |
| | (b) 0.08, outer 60, inner 300 | 49.7 g | 5.8 g | 11.4 + 4.0 g | 21 | 4 h 23 | 54 |
| | **(a) 0.08, outer 60, inner 120** | 49.7 g | 5.8 g | 11.4 + 4.0 g | 21 | **4 h 27** | 54 |
| `5_unit` | before all this | 73.2 g | 6.9 g | 7.4 + 2.0 g | 9 | 2 h 57 | 22 |
| | (c) 0.12, outer 100, inner 300 | 75.2 g | 5.8 g | 9.1 + 2.9 g | 17 | 4 h 18 | 36 |
| | (b) 0.08, outer 60, inner 300 | 75.6 g | 5.8 g | 11.4 + 4.0 g | 21 | 6 h 14 | 54 |
| | **(a) 0.08, outer 60, inner 120** | 75.6 g | 5.8 g | 11.4 + 4.0 g | 21 | **6 h 22** | 54 |

The outer wall at
60 instead of 100 cost 19 min on `1_case` at 0.12 mm, 28 on `5_unit`; to change a speed, set it in `CASE_SETTINGS` and rebuild.
No print has compared the finishes yet.

## What to check, and what to change

Numbers live in `case/case.scad`; after changing one, rebuild the packages
(`python3 tools/build.py`, or by hand `python3 case/export.py --stl`, `python3 case/fitkit.py`, `python3 case/print_package.py`,
`python3 case/print_package_p1s.py`).

**Buttons (`2_buttons_quick`):**
- D-pad in each of the three `tune_openings` crosses (gaps 0.55, 0.65, 0.75 mm, engraved): pick the
  tightest it moves freely in. Now `dpad_gap` = 0.65 (0.5 was too tight in the first print, 0.8 too loose in the next).
- A, B, X, Y into the keyed cluster from the back: tab into its slot, each notch onto its rib, and
  no cap fits another cap's hole. A rib fighting a cap: note the angle, fix `abxy_notches` (or
  `notch_keys = false`). The keyed holes are 8.0 (`cap_hole_d`, Alex's DS Lite faceplate's; 7.6 was too tight, 2026-10-21);
  the plain holes (7.6, 7.8, 8.2) say whether it can be tighter or must be looser.
- Each stack: silicone on the board slice, caps in the end slice, the slice on its four legs. The cap
  should stand about 1.4 mm above the face (1.41, the faceplate's 1.6 mm lip), press cleanly and spring back. If the 1.75 mm slice is
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
  into `overhang` in `case.scad` (60 on this variant, 45 on the old main line): it sets the tub's back round and the skirts' round.
- Screw holes: `coupon_screws_tub` (two of the tub's holes, printed flat as the tub: H4's slimmed
  5.0 boss by the left wall and an end boss) should come out
  with the 4.4 mm counterbore and the 2.4 mm hole open, and an M2 screw should pass; the old plates
  had supports in every counterbore (see above). `coupon_screws_top` (the top's boss H3, face down)
  should take an M2 x 6 self-tapper into its 1.6 mm pilot. Say which of the case's holes were filled.
- Column clearance: slide each ring of `test_rings` over `test_column` (5.0 mm, as the bosses):
  report the tightest that slides on freely and the loosest that doesn't rattle. (No column passes through another printed part now;
  the answer is for later.)

**Fit coupons (`4_fit_coupons`, `0_buttons_end`):**
- USB-C: `coupon_usb_top` face down, the board coupon face down into it, the tub coupon over it (its tab rises into the top's notch).
  The opening should hug the receptacle stand-in with a small even gap (`usb_gap` 0.25), and a real plug's overmold should sit flat on the case.
- Seam: lower `coupon_seam_top` (the top's left wall) over `coupon_seam_tub`: the skirt slides over the tongue without force, the key tab
  into its pocket, and sits flush with the tub (`seam_fit`, `seam_gap`); the grip ribs should hold the two together lightly
  (`grip_reach`: raise it if they fall apart, lower it if they won't go together by hand). Look at the finish the support interface left.
- Panel frame: the real panel (or the dummy) drops in glass down with a little play, the active
  area centred in the window (`panel_fit`, `window_margin`).
- Seat 1's end, built upside down: `coupon_buttons_top` face down, caps and silicone in from behind, the board mock face down, the tub
  coupon: the seam at the end an even line, the caps standing about 1.4 mm proud of the end's face.
- Speaker (in `5_unit`): the dummy (or the real speaker) drops face down into the top's pocket over the grille without play to speak
  of, and the cell dummy into the board mock's hole and the tub's pocket.

**The whole case (`1_case`, or `5_unit`):** build it upside down as above. It should measure 9.6 mm at the screen and 7.7 mm at
the ends, the seam flush all round, and nothing should rattle. Press each grille slot's rim: the speaker should sit flat behind it.
