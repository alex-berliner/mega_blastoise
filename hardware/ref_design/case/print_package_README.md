# Mega Blastoise: 3D print package

Everything to print for the reference design's case and fit kit, generated from `case/case.scad`
on the current design (screen in a lid pocket, smooth step to the decks, Start/Select, flush USB-C).

## The projects

Each `.3mf` is a Snapmaker Orca project built from your `fit_test_v5.3mf`, so it keeps that
project's printer and settings: Snapmaker U1, 0.20 mm Standard, PLA on toolhead 2, PETG support
interface on toolhead 1, textured PEI, auto supports. Open it, check the plate, slice.

| Project | What's on it | Print it |
|---|---|---|
| `2_buttons_quick.3mf` | **Start here (about an hour).** `tune_openings` (three D-pad opening sizes, the keyed A/B/X/Y cluster, three hole sizes), and the D-pad, A/B/X/Y and Start/Select stacks: a board slice each and a lid slice at the 1.75 mm silicone height | first |
| `3_buttons_heights.3mf` | the same three lid slices at 1.6 and 1.9 mm silicone height | if the 1.75 mm lids don't sit right |
| `4_fit_coupons.3mf` | USB-C opening (tub, lid tongue, board tab with a receptacle stand-in), one snap hook and its groove, the lid's panel frame, and seat 1's whole end (tub, board mock, lid) | second |
| `1_case.3mf` | the tub, the lid, the board mock, and dummies of the panel, speaker, cell and cell plug | last, once the coupons fit |
| `5_inserts_experiment.3mf` | **Experiment.** Seat 1's end with its deck as a separate insert (`deck_inserts` in `case.scad`): tub, board mock (to place the silicone on), lid, insert. `5_inserts_experiment_P1S.3mf` is the same plate as a Bambu Studio project for a P1S (PLA only) Supports are off: every part prints flat, the insert face down on the bed | whenever you like |

The lid and the lid slices print face down as exported; let the auto supports go under the button
decks (they sit 2.34 mm below the screen face). Everything else prints flat as exported.

`stl/` has every part as a plain STL, in case a project won't open in your Orca version: drag them
onto a plate with the same settings. `lid.stl` and `tub.stl` are the real case; `tub_print.stl`
is the tub without its feet (prints flat without supports).

**The deck-insert experiment (`5_inserts_experiment`).** Printed face down, the lid's decks hang
2.3 mm above the bed, so they need supports and keep the PETG's peel-away texture. With
`deck_inserts = true` each end's deck (both clusters and Start/Select) is its own part that prints
face down on the bed like the screen face, so no part needs supports. The step to the screen
becomes a rounded edge and a 45 degree slope. To fit it: tilt the insert's outer end under the
tub end wall's lip, lower it onto the board, then put the lid on. The lid's step walls press down
on a 45 degree wedge along the insert's inner edges, and the insert's deck runs on under the
spine to a ring round the end screw's boss: the lid's boss drops through it as the lid goes on,
so the end screw holds the insert too. Check that it sits flat with no rattle, that
the seam round it looks acceptable, and that the caps press as well as in `0_buttons_end`. The
default stays `deck_inserts = false` until the experiment wins.

## What to check, and what to change

Numbers live in `case/case.scad`; after changing one, rebuild the package
(`python3 case/export.py --stl`, `python3 case/fitkit.py`, `python3 case/print_package.py`).

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
  (`snap_barb`, hook thickness). PETG takes the bending better than PLA.
- Panel frame: the real panel (or the dummy) drops in glass down with a little play, the active
  area centred in the window (`panel_fit`, `window_margin`).
- Seat 1's end: the cell plug (real or dummy) passes up through the board mock's hole in front of
  J2; caps, silicone and the smooth step from the deck up to the screen face.

**The whole case (`1_case`):** cell (or dummy) on the tub floor, plug up through the board mock's
hole, board mock in, panel into the lid's pocket on tape, silicone and caps, lid down: the four
hooks click, the lid sits flush, nothing rattles. It should measure 13.4 mm at the screen and
11.1 mm at the ends.
