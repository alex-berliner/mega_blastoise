# Genuine DS Lite caps: where the numbers come from

The case takes genuine Nintendo DS Lite (USG-001) ABXY and D-pad caps and silicone. No scan
of a genuine part was found, so the numbers below are measured from open 3D models: replicas
of the caps, and shells built to take the genuine ones. Check one real cap with calipers before
ordering; `case/fitkit/coupon_buttons_*.stl` is the quick test.

| Source | Licence | Used for |
|---|---|---|
| Pi Tin, github.com/jackw01/pi-tin | CERN-OHL-W-2.0 | front panel made for genuine ABXY and D-pad caps: hole, flange pocket, cap proud, D-pad opening, pitches |
| jusbot, Printables 1178557 | public domain | ABXY and D-pad cap replicas: sizes, tab, flat, notches |
| facelesstech, Printables 205921 | CC BY-NC-SA | DS Lite front plate: tab slot directions (a cross-check) |
| MacroSOS, Printables 986494 | CC BY-NC | shell holding two genuine caps: slot and flat directions (a cross-check) |

Only the numbers are used; no geometry is copied from any of them.

## ABXY caps

| | mm | case.scad | Confidence |
|---|---|---|---|
| Body diameter | 6.94-7.00 | `cap_abxy_d` 7.0 | measured (two models agree) |
| Flange diameter | 8.80 | `cap_flange_d` | measured (two models agree) |
| Flange thickness | 0.5-0.6 | `flange_t` 0.55 | measured |
| Height, flange bottom to top | 3.56-3.60 | `cap_h` 3.56 | measured |
| Keying tab | out to r 5.44-5.6, 1.8-2 wide | `cap_tab_r`, `cap_tab_w`; the lid's slot is a 35 degree wedge (`slot_a`, `slot_off`) | measured |
| Pitch (diamond) | 15.0 | `abxy_pitch` | our gerbers, Pi Tin agrees |
| Lid hole | 7.34 (Pi Tin), 8.00 (Alex's faceplate) | `cap_hole_d` 8.0 | shells built for genuine caps |
| Flange pocket | 9.15 across, 1.35-1.45 deep (faceplate 9.18) | `pocket_d` 9.2 | shells built for genuine caps |
| Lip over the flange | 1.05 straight + 0.6 chamfer (faceplate 1.0 + 0.6) | `lip_t` 1.6, `cap_chamfer` 0.6 | shells built for genuine caps |
| Cap above the face | about 1.3 (faceplate 1.41) | `cap_proud` 1.41 (from `lip_t`) | shells built for genuine caps |

Keying (seen from the front, degrees clockwise from the seat's right; A is the right cap): each
flange has a flat facing the diamond's centre and a tab 45 degrees clockwise from it, so the four
tabs make a pinwheel.

| Cap | Tab | Flat | Notches (not used) |
|---|---|---|---|
| A | 135 | 180 | 225? |
| B | 225 | 270 | 0, 180 |
| X | 45 | 90 | 225 |
| Y | 315 | 0 | 90, 180 |

The tab directions agree across three sources (facelesstech's slots sit 7.5 degrees off, cut
35 degrees wide; Alex's faceplate's sit 7 degrees off too, 52 degrees from the flat: see below). The lid cuts a slot for each tab, so a cap can't turn; it does not add ribs for
the notches (only two models, possibly not independent, show them), so a swapped cap still fits
its hole with its tab in the slot. Since 2026-10-19 (Alex's caps have notches; the first print's lid had nothing to match) ribs in each pocket fill the notches at these angles (`abxy_notches`, `notch_keys` in case.scad); `case/fitkit/tune_openings.stl` checks them against real caps.

## D-pad

| | mm | case.scad |
|---|---|---|
| Cross | 17.5-17.8 x 5.8-5.9 arms, 3.34 tall | `dpad_arm_l` 17.8, `dpad_arm_w` 5.9, `dpad_h` |
| Flange | fills the corners between the arms to about r 6.5, 0.6 thick | `dpad_flange_r` |
| Lid opening | 18.2 x 6.2 (Pi Tin), 18.80 x 6.90 (faceplate) | the cross plus `dpad_gap` 0.8 all round (tuned on a print) |
| Contact pitch | 14.0 | `dpad_pitch` |

No rotational keying beyond the cross was found; the pivot is a nub on the cap or a boss in
the membrane (about 2.4 across in Pi Tin's).

## Alex's DS Lite faceplate (front.stl, measured 2026-10-21)

Alex supplied a DS Lite lower front faceplate as an STL (`~/Downloads/ds/front.stl`, 133.0 x 73.67
x 9.0 mm, one watertight body) and asked for it to be the basis of everything about the buttons and
the D-pad. Its flat outside face is its top; seen from the front, the D-pad is on the left and A/B/X/Y
on the right, with Start/Select below them. Its surfaces sit on a 0.2 mm grid of depths (CAD, not a
scan), so the numbers below are exact to the STL: depths are under the face, diameters fitted to the
sections (within 0.01). The case's buttons now follow it, except the clearances a print has tuned
and the walls Alex's real pads and shell disagree with (the last column says which).

A/B/X/Y (centres X -248.241, 80.643; Y -255.807, 73.077; B -248.241, 65.511; A -240.675, 73.077):

| Feature | Faceplate | case.scad | Old | New |
|---|---|---|---|---|
| Opposite holes apart | 15.13 | `abxy_pitch` | 15.0 | 15.0 (the board is routed; the faceplate's features are centred on our switches) |
| Hole at the face | 9.17 | `cap_hole_d` + 2 `cap_chamfer` | 7.6, no chamfer | 9.2 |
| Chamfer | 45 degrees, 0.6 deep | `cap_chamfer` | none | 0.6 |
| Hole below it | 8.00, straight from 0.6 to 1.6 | `cap_hole_d` | 7.6 | 8.0 (looser than the 7.6 under test) |
| Lip (face to the flange's top) | 1.60 | `lip_t` (sets `cap_proud`) | 1.16 | 1.6 |
| Flange pocket | 9.18, from 1.6 to 3.2 | `pocket_d` | 9.2 | 9.2 (FDM, kept) |
| Flat in each pocket | 4.32 from the hole's centre, facing the cluster's centre (the centre frame's side) | none | none | none: our cap model has no flat (see the notes) |
| Plate outside the cluster | 1.4 thick | the deck, `lid_t` | 2.0 | 2.0 (kept) |
| Ceiling inside the clover | 1.6 (the lip's level) | the deck between the pockets | solid | solid (kept, it holds the tab slots) |
| Clover wall, lobe outer radius | 5.525 round each hole (11.05 across a lobe; Alex's shell: 10.45) | `abxy_ring` | 5.25, four circles | 5.25 (inside Alex's pad's rim) |
| Clover wall, lobe inner radius | 4.59 (the pocket), 0.935 thick | `pocket_d` / 2 | 4.6 | 4.6 |
| Between the lobes | straight runs tangent to the lobes, 1.0 fillets where they meet; waist 8.23 from the centre (16.46 notch to notch), 1.0 thick there | `abxy_clover2d`, `abxy_fillet` | the circles only | the faceplate's runs and fillets, trimmed `rim_clear` inside the pad's rim |
| Across the lobes | 26.18 | from `abxy_ring` | 25.5 | 25.5 |
| Wall's bottom | 3.2 (1.8 below the plate, as Alex's shell's 1.8) | `sil_rib_gap` | 0.8 over the board | 0.8 (0.15 over the web; the faceplate's 0.65 stands on it) |
| Centre square pocket | 5.10 inside, 0.70 wall, roof at 1.6 (2.25 over the board) | `abxy_square` | 5.15, 0.65, 2.1 | 5.15, 0.7, 2.25 |
| Tab gaps | between the pockets, round pinwheel ears on the centre frame's corners; a 2.4 wide slot at X 38, A 128, B 218, Y 308 (52 degrees from each flat) | `abxy_tabs`, `slot_a`, `slot_off` | 45/135/225/315, a 30 degree wedge | the same tabs, a 35 degree wedge 2.5 degrees towards the faceplate's: it takes either |
| Notch ribs | none | `abxy_notches`, `notch_keys` | ribs | ribs (Alex's caps have the notches, 2026-10-19) |

D-pad (centre -350.75, 70.461):

| Feature | Faceplate | case.scad | Old | New |
|---|---|---|---|---|
| Opening at the face | 20.38 tip to tip, arms 8.48 wide | the opening + 2 `dpad_chamfer` | 19.4 x 7.5, no chamfer | 21.0 x 9.1 |
| Chamfer | 45 degrees, 0.8 deep | `dpad_chamfer` | none | 0.8 |
| Opening below it | 18.80 x 6.90 (0.5 round a 17.8 x 5.9 cross); inside corners sharp, tip corners r 0.42 | `dpad_gap` | 0.8 | 0.8 (0.5 was too tight on a print) |
| Lip at the corners | 1.20 (0.4 under the A/B/X/Y lip) | `dpad_lift` | 0.35 | 0.35 (both put the flange 2.10 over the board) |
| Flange's corner pockets | r 6.75 round the centre, from 1.2 to 3.2 | `dpad_flange_r` + `dpad_pocket_gap` | 7.0 | 7.0 (0.35 bound on a print) |
| Along the arms | the opening's walls run on to 2.0 deep; inside the wall the ceiling is 2.0 (1.85 over the board) | the rim relief | 1.95 over the board | 1.95 |
| Wall | a lobe per arm: outer r 4.8, inner r 4.175, centred 6.8 out; straight sides, a 45 degree chamfer from lobe to lobe (7.46 from the centre on the diagonal); 0.625 thick; 23.2 across the tips | `dpad_wall` | a ring outside the pad (lobes 5.15, waist 7.95, 0.8 wide) | the faceplate's lobes, 0.65 thick, trimmed `rim_clear` inside the pad's rim (11.5 out at the tips) |
| Wall's bottom | 3.2, on the web | `sil_rib_gap` | 0.8 | 0.8 |
| Over the collar | nothing (the opening) | | | |

Start/Select (centres -257.880, 48.786 and -257.880, 40.936, one above the other):

| Feature | Faceplate | case.scad | Old | New |
|---|---|---|---|---|
| Pitch | 7.85 | `ss_pitch` | 7.5 | 7.5 (board) |
| Hole at the face | 4.98 | `ss_hole_d` + 2 `cap_chamfer` | 5.1, no chamfer | 5.0 |
| Chamfer | 45 degrees, 0.6 deep | `cap_chamfer` | none | 0.6 |
| Hole below it | 3.80, straight to 1.2 | `ss_hole_d` | 5.1 (4.5 cap + 0.3) | 3.8 |
| Lip | 1.20 (0.4 under the A/B/X/Y lip) | `ss_lift` | 0 | 0.35 (flange top 2.65 over the board, as the faceplate's) |
| Flange pocket | 5.35 round, 1.0 tall (1.2 to 2.2) | `ss_pocket` | the cell | 5.35 x 1.0 |
| Cells | 5.70 square, corners about r 0.3, from 2.2 to 3.2 (roof 1.65 over the board) | `ss_cell`, `ss_cell_r` | 6.4, up to the lip | 5.8, roof 1.65 over the board |
| Frame | one rounded rectangle round both cells: 6.90 x 14.60, corners r 1.0, walls 0.6, 2.0 between the cells, bottom 3.2 | `ss_frame_w`, `ss_frame_r` | a square frame per cap, 0.9 walls | one frame, 0.6 walls, r 1.0 |
| Caps (ESTIMATES) | the hole and pocket say a body under 3.8 and a flange under 5.35 | `ss_cap_d`, `ss_flange_d` | 4.5, 6.0 | 3.2, 5.0 |

### The stack under the faceplate

Every wall and frame on the faceplate ends 3.2 under its face. The walls stand inside the pads'
rims (Alex, 2026-10-20: his shell's clover stands inside the rim; and the faceplate's D-pad wall
grown by the 0.4 rim gives 24.0 across the lobes and 15.7 notch to notch, his pad's 24 and 15.6),
so they bear on the 0.65 web, which puts the board's top 3.85 under the face. (Standing on the
board instead, at 3.2, the flanges would sit 1.05 over it and the caps' 1.3 stems would reach under
it: impossible.) Then, over the board:

| | Faceplate | Here |
|---|---|---|
| A/B/X/Y flange underside (`dome_h`) | 1.70 | 1.75 (kept: within the web's and the flange's tolerance; the 1.6 and 1.9 tuning lids bracket both) |
| A/B/X/Y flange top (the lip's underside) | 2.25 | 2.30 |
| D-pad and Start/Select flange tops | 2.65 | 2.65 |
| Plate's underside over the rims | 2.45 (0.65 over them) | 1.95 (`rim_relief`, a thicker deck) |
| The face | 3.85 | 3.90 |
| A/B/X/Y caps above the face | 1.41 (3.56 - 0.55 - 1.6) | 1.41 |

The faceplate confirms the open A/B/X/Y question: a flange 1.70 over the board over a dome that
stands 2.8 free, with a 1.3 stem under it. Either the dome's top takes the stem, or the cap is
hollow there; the faceplate leaves no more room than the case does.

### Where the faceplate and Alex's parts disagree

- Its A/B/X/Y lobes (5.525) would sit on Alex's pad's rim (inner edge 5.35 round each dome); his
  own shell's clover is 10.45 across (5.225), which clears it. The case keeps 5.25.
- Its D-pad wall's lobes are round-ended stadiums centred 6.8 out; our pad model's lobes are
  circles round the domes (7.0 out). Both match Alex's two measurements, but they differ along the
  arms' sides, where the faceplate's wall would sit 0.2 onto the modelled rim. The case trims the
  wall to the model; a third measurement (the pad's width across one lobe, 5 mm in from its tip)
  would settle the shape.
- Its holes are 15.13 apart (ours 15.0, from the gerbers), its Start/Select holes 7.85 (ours 7.5)
  and stacked one above the other (ours side by side).
- Each A/B/X/Y pocket has a flat 4.32 from its centre facing the cluster's centre, and the tab gaps
  sit 52 degrees from it, not our caps' 45. Our cap model has no flat; the case's pockets are round
  and the slot takes either tab.
- Start/Select: a 4.5 cap can't pass its 3.8 hole; the estimates are now from its openings.

## Still unknown

The silicone: its outline (`piece_d`) and its height at rest (`dome_h`, board to the flange's
underside). `dome_h` sets how far the ends step down: the deck face is `dome_h` + 2.15 mm above
the board (`flange_t` + `lip_t`). Measure it with the button coupons before ordering.

A pointer from the DS Lite itself (a 3D scan of an aftermarket shell, Wesk on BitBuilt, and
gorgyrip's reverse-engineered USG-CPU-10 board): its face plate is 1.4-1.5 mm, the button guide
tubes end about 3.0 mm under the face, and the board's top sits about 3.0-3.4 mm under the face.
That would put `dome_h` nearer 1.1 mm than the 2.0 here, which would leave the caps about 0.4 mm
proud instead of 1.3. Inferred from an aftermarket shell with an unknown board thickness: a
reason to measure, not a number to use.

Also from the DS Lite: its face holes are 7.6 mm for A/B/X/Y and 18.9 mm across the D-pad's
cross (taken here until 2026-10-21, when Alex's faceplate replaced them); it closes with seven screws into the face's bosses,
two of them through the board, with the face nested in a stepped wall (taken here: the six M2
screws and the lid's lip in the tub's step).

The DS Lite shell itself is 133.0 x 73.9 mm (Nintendo's figure). This case is its own shape
(the panel and the two seats set it), so only the button geometry is taken from the DS Lite.
