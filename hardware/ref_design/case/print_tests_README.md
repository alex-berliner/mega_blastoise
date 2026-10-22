# Overhang and first-layer tests (Mega Blastoise)

Print-settings tests only, no case parts. P1S/ for the Bambu P1S, U1/ for the Snapmaker U1. All print
without supports at 220 C (black PLA+). Print in number order and stop whenever you like; each one
answers less than the one before it.

1. `1_first_layer_020` (about 6 min): the lip at the tub's bottom edge, the one thing a real print
   showed. Report the best coupon, e.g. "P1S: L20 E25".
2. `2_overhang_settings` (about 32 min): how steep the 0.20 mm layers overhang cleanly, with the
   default settings (DEF) and tuned ones (TUNED). Report the steepest clean fin of each.
3. `3_overhangs_016`, 4. `4_overhangs_012`: the same ladder at finer layers; only matter if the case
   gets printed finer than 0.20.
5. `5_first_layer_012`: the lip test with a 0.12 mm first layer, if no coupon in 1 came out clean.
6. `6_overhangs_008`: the finest layers; least likely to be used.

(`7_overhangs_020` is left out: `2_overhang_settings`'s DEF ladder is the same print.)
The sections below use the plates' original names: 8_first_layer = 1 and 5, 9_overhang_settings = 2,
7_overhangs = 3, 4 and 6. Built by `case/print_package.py` and `case/print_package_p1s.py`;
`python3 case/print_package.py --tests` packs this zip again.

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
(naming only test plates leaves the case kits as they are; the tests zip has what was built).

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

**Overhang settings (`9_overhang_settings`):** the 0.20 mm ladder twice, side by side, its pad engraved
`DEF` or `TUNED`. `DEF` prints with the package's settings; `TUNED` has its own settings in the project
(an object's settings, kept as they are when the project opens; the P1S's G-code shows them at work).
What the two slicers have for overhangs (Bambu Studio 2.08 and Snapmaker Orca 2.4.0, from their
settings lists; the OrcaSlicer wiki, Quality > Overhangs and Speed > Overhang speed; the Bambu Lab
wiki, Bambu Studio's auto cooling):

| Setting (key) | Bambu Studio (P1S) | Snapmaker Orca (U1) | What it does | In TUNED |
|---|---|---|---|---|
| Detect overhang wall (`detect_overhang_wall`) | yes, on | yes, on | finds walls over thin air and prints them with the overhang speeds (100 % over air: as a bridge) | already on |
| Slow down for overhang (`enable_overhang_speed`, `overhang_1_4_speed` to `overhang_4_4_speed`) | yes: 0 (no slowdown), 50, 30, 10 mm/s for 0-25, 25-50, 50-75, 75-100 % of a line overhanging | yes, the same | slower walls cool before the next layer leans on them | 50 to 30, 30 to 15 (a fin's wall at 0.20 mm: 45 degrees is about 50 % over, 55 about 70, 60 and up 80 to 100) |
| Fan for overhangs (`overhang_fan_speed`, `overhang_fan_threshold`, `enable_overhang_bridge_fan`) | yes: 100 % past 50 % overhang | yes, the same; also `fan_speedup_overhangs` | forces the fan up on overhanging walls | nothing to gain: both PLA profiles run the fan at 100 % all the time |
| Slow down for curled perimeters (`slowdown_for_curled_perimeters`) | no | yes, off | slows where a wall may curl up (the lip the nozzle hits) | on (U1) |
| Reverse on even (`overhang_reverse`, `_threshold` 50 %, `_internal_only`) | no | yes, off | prints overhanging walls the other way round on even layers, so the pull alternates and they don't curl one way | on (U1) |
| Extra perimeters on overhangs (`extra_perimeters_on_overhangs`) | no | yes, off | adds overhang walls over steep overhangs and unanchored bridges | on (U1) |
| Bridge flow / density (`bridge_flow`, `bridge_density`, `thick_bridges`) | flow 1.0, no density | flow 0.8, density 80 % | thinner bridge lines cool faster and sag less (the 75 and 80 fins print as bridges) | P1S flow 0.85 (the U1 has 0.8 already) |
| Smooth speed changes (`smooth_speed_discontinuity_area`) | yes, on | no | eases the speed steps between overhang bands | already on |
| Make overhang printable (`make_overhang_printable`, `_angle`) | no | yes, off | reshapes the model so nothing overhangs past the angle | not used: `overhang` in `case.scad` does that in the design |
| Arc overhangs (Steven McCulloch's concentric arcs) | no | no | prints flat overhangs in arcs grown from their edge | not in either slicer, nor in OrcaSlicer itself: only Pleccer (a PrusaSlicer fork) and post-processing scripts have it |

Compare the two ladders fin for fin, the same way as `7_overhangs` (clean: no sag over about 0.3 mm,
no curl the nozzle would hit, a smooth face), F side and B side. Report the steepest clean fin of
each (e.g. "U1: DEF 57.5, TUNED 62.5"). If `TUNED` gains, the packages take its settings for the
case; if not, the defaults stay. The settings only change speed and paths, so they cost a few
minutes (the ladder: 30 min with the defaults, about 33 tuned, on the U1).
