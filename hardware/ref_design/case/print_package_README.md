# Mega Blastoise: 3D print package

Everything to print for the reference design's case and fit kit, generated from `case/case.scad`
on the current design (screen in a lid pocket, the button decks as separate inserts, Start/Select,
flush USB-C).

## The projects

<!-- printer -->
Each `.3mf` is a Snapmaker Orca project built from your `fit_test_v5.3mf`, so it keeps that
project's printer and settings: Snapmaker U1, 0.20 mm Standard, textured PEI. PLA only, on
toolhead 2: no PETG, no second toolhead, no prime tower. Open it, check the plate, slice.
(`mega_blastoise_print_P1S.zip` has the same plates sliced for a Bambu Lab P1S.)

Supports are on only for `1_case` and `4_fit_coupons`, measured part by part (see below).
<!-- /printer -->

| Project | What's on it | Print it |
|---|---|---|
| `2_buttons_quick.3mf` | **Start here (about an hour).** `tune_openings` (three D-pad opening sizes, the keyed A/B/X/Y cluster, three hole sizes), and the D-pad, A/B/X/Y and Start/Select stacks: a board slice each and a lid slice at the 1.75 mm silicone height | first |
| `3_buttons_heights.3mf` | the same three lid slices at 1.6 and 1.9 mm silicone height | if the 1.75 mm lids don't sit right |
| `0_buttons_end.3mf` | Seat 1's whole end: tub, board mock (to place the silicone on), lid, deck insert | any time |
| `4_fit_coupons.3mf` | USB-C opening (tub, lid tongue, board tab with a receptacle stand-in), one snap hook and its groove, the lid's panel frame, and seat 1's whole end (tub, board mock, lid, insert) | second |
| `1_case.3mf` | the tub, the lid, the board mock, and dummies of the panel, speaker, cell and cell plug | last, once the coupons fit |
| `1_case_inserts.3mf` | the two deck inserts (the same part, one for each end) | with `1_case` |

The lid, the deck inserts and the tuning lid slices print face down as exported (the slices deck
face down, legs up), everything else flat. `overhangs.py` checked every part for a downward face
flatter than 45 degrees that the printer can't bridge (5 mm at most) or step out to (1 mm): only
three parts have one, so only their plates get tree supports from the build plate:
- `board_mock` (in `1_case`) and `coupon_usb_board` (in `4_fit_coupons`): the USB-C stand-in's
  front stands 1.2 mm out past the board's edge, round-cornered underneath. Supported, so it
  doesn't droop: the opening's fit round it is what the coupon tests.
- `speaker_dummy` (in `1_case`): its solder tab and lead exit stick out 1.9 and 2.4 mm.

`stl/` has every part as a plain STL, in case a project won't open in your Orca version: drag them
onto a plate with the same settings. `lid.stl`, `insert.stl` (print two) and `tub.stl` are the
real case; the tub prints flat as exported (it has no feet).

**The deck inserts.** Each end's deck (both clusters and Start/Select) is its own part, printed
face down on the bed like the screen face, so it comes out smooth with no supports. The step
to the screen is a rounded edge and a 45 degree slope. Both ends take the same insert. To fit one:
caps into it, tilt its outer end under the tub end wall's lip, lower it onto the board, then put
the lid on. The lid's step walls press down on a 45 degree wedge along the insert's inner edges,
and the insert's deck runs on under the spine to a ring round the end screw's boss: the lid's
boss drops through it, so the end screw holds the insert too. Check that it sits flat with no
rattle and the seam round it looks right. (`deck_inserts = false` in `case.scad` gives back the
one-piece lid, which needs supports under its decks.)

## What to check, and what to change

Numbers live in `case/case.scad`; after changing one, rebuild the packages
(`python3 case/export.py --stl`, `python3 case/fitkit.py`, `python3 case/print_package.py`,
`python3 case/print_package_p1s.py`).

**Buttons (`2_buttons_quick`):**
- D-pad in each of the three `tune_openings` crosses (gaps 0.6, 0.8, 1.0 mm, engraved): pick the
  tightest it moves freely in. Now `dpad_gap` = 0.8 (0.5 was too tight in the first print).
- A, B, X, Y into the keyed cluster from the back: tab into its slot, each notch onto its rib, and
  no cap fits another cap's hole. A rib fighting a cap: note the angle, fix `abxy_notches` (or
  `notch_keys = false`). The plain holes (7.4, 7.8, 8.0) tune `cap_hole_d` (now 7.6).
- Each stack: silicone on the board slice, caps in the lid slice, lid on its four legs. The cap
  should stand about 1.85 mm above the deck, press cleanly and spring back. If the 1.75 mm lid is
  wrong, try the 1.6 and 1.9 lids (`3_buttons_heights`) and set `dome_h`; `dpad_lift` moves only
  the D-pad.
- **Start/Select:** their membrane, pill spacing and caps are estimates (`ss_mem` 14 x 6 mm,
  `ss_pitch` 7.5, `ss_cap_d` 4.5). Measure your real Start/Select membrane and caps and send the
  numbers; the stack shows whether the estimates already work.

**Fit coupons (`4_fit_coupons`):**
- USB-C: board coupon into the tub coupon (its tab drops into the wall's slot), lid coupon on
  top. The opening should hug the receptacle stand-in with a small even gap (`usb_gap` 0.25), and
  a real plug's overmold should sit flat on the case.
- Snap hook: should click with firm thumb pressure and come apart with a blade
  (`snap_barb`, hook thickness). PLA is stiffer than PETG: if the hook cracks, say so.
- Panel frame: the real panel (or the dummy) drops in glass down with a little play, the active
  area centred in the window (`panel_fit`, `window_margin`).
- Seat 1's end: the cell plug (real or dummy) passes up through the board mock's hole in front of
  J2; caps, silicone, the insert, and the step from the deck up to the screen face.

**The whole case (`1_case`):** cell (or dummy) on the tub floor, plug up through the board mock's
hole, board mock in, panel into the lid's pocket on tape, silicone, caps into the inserts, inserts
in, lid down: the four hooks click, the lid and inserts sit flush, nothing rattles. It should measure 13.4 mm at the screen and
11.1 mm at the ends.
