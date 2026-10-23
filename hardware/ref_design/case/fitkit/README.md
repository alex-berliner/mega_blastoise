# Fit kit: test the case with the real parts before ordering boards

Printable stand-ins and case coupons, all generated from `case/case.scad` and the routed board, so
they always match the design. The STLs are build output, not in git: generate them (and again after
any case or board change) with:

    python3 case/export.py --stl      # the real tub, island and decks
    python3 case/fitkit.py            # this folder (under a minute with Manifold)

![the shell, exploded in its assembly order](kit_assembly.png)

Until the board is re-placed for the 133 mm case (the case sets the board now: `../BOARD_OUTLINE.md`),
the board mock has the new outline and holes but today's part envelopes, cut to the outline.

## The parts

| File | What it is | Print |
|---|---|---|
| `../tub.stl`, `../top.stl`, `../deck1.stl`, `../deck2.stl` | the real case: the tub (the back), the island (the raised screen block), seat 1's and seat 2's decks | each as exported, no supports: the tub back down, the island and the decks face down |
| `build_bar.stl` (print two) | a bar 1.85 mm tall under each deck's end while the case is built face down, so the island's face (1.85 mm proud of the decks') rests on the bench and the caps stand clear of it | flat, as exported |
| `board_mock.stl` | the board: outline, the six screw holes (0.2 mm oversize), the height envelope of every part 0.8 mm or taller (from its 3D model), the 20 membrane contacts, the four silicone pieces and the two Start/Select membranes engraved, J3's outline to tape a real connector on (ticks mark the two ends of the range J3 may move over) | flat, as exported (supports under the USB-C stand-in) |
| `panel_dummy.stl` | the panel at its thickest (2.4 mm), active area engraved on the glass side | flat, glass side up |
| `cell_dummy.stl` | the 403450 cell (50 x 34 x 4 mm, nominal) with a stub where its leads leave its left end: for the board mock's hole and the tub's pocket | flat |
| `cell_plug_dummy.stl` | a PicoBlade (MX1.25) plug with a cable stub (approximate) | upright |
| `speaker_dummy.stl` | the GSPK151035PN speaker, 10 x 15 x 3.5 mm, its cone's outline engraved: for its pocket under the grille | flat |
| `coupon_seam_tub.stl` + `coupon_seam_deck.stl` | a stretch of the left wall at the seam, cut from the real tub and seat 2's deck: the tub's tongue with a key, grip ribs and a crush rib, the arm's skirt over it | as exported |
| `coupon_buttons_tub.stl` + `_board.stl` + `_top.stl` + `_deck.stl` | seat 1's end of the tub, board, island (the step, the spine and the wedge round them) and seat 1's deck's core: the button stack and the island's wedge under the deck's lip | as exported |
| `coupon_usb_tub.stl` + `coupon_usb_top.stl` + `coupon_usb_arms.stl` + `coupon_usb_board.stl` | the USB-C opening: its lower half in the tub's tab, its upper half in the island's ear, the two right arms' ends either side of the ear (its laps go behind their skirts), and the board's tab with a stand-in for the receptacle's shell | as exported |
| `coupon_meet_tub.stl` + `coupon_meet_top.stl` + `coupon_meet_arms.stl` | where the arms meet on the left wall: the tub's tongue and the lap's slot, the island's side and wedge, both arms' ends (seat 1's lap behind seat 2's skirt) | as exported |
| `tune_openings.stl` | **the quickest button test (about 30 min):** D-pad openings at 0.55, 0.65 and 0.75 mm round the cross, a whole keyed A/B/X/Y cluster (tab slots and notch ribs), and plain holes at 7.6, 7.8 and 8.2 mm round its 8.0 (`cap_hole_d`); the numbers are engraved in the face | face down, as exported |
| `tune_stack_dpad_board.stl` + `tune_stack_dpad_lid_dome*.stl` | one D-pad's slice of the board (contact rings engraved) and of the deck, the deck slice on four legs at its real height. Three, for a silicone 1.6, 1.75 or 1.9 mm tall at rest (`dome_h`); the value is engraved in each | board flat, deck slices face down |
| `tune_stack_abxy_board.stl` + `tune_stack_abxy_lid_dome*.stl` | the same for one A/B/X/Y cluster | as above |
| `tune_stack_ss_board.stl` + `tune_stack_ss_lid_dome*.stl` | the same for seat 1's Start/Select membrane (its outline, pill pitch and caps are ESTIMATEs in `case.scad`: `ss_mem`, `ss_pitch`, `ss_cap_*`) | as above |
| `coupon_frame_top.stl` | the island's window, pocket and panel frame | face down, as exported |
| `test_overhangs.stl` | the test plate's overhang ladder: fins whose undersides slope 45, 50, 55, 60, 65, 70 and 75 degrees from vertical (engraved on top), and a convex 6 mm quarter round from 85 degrees up to vertical, ticked at 45, 60 and 75 on its end face | as exported, no supports (on purpose) |
| `coupon_screws_tub.stl` | two of the tub's screw holes (H4's slimmed side boss by the wall and an end boss), cut from the real tub: counterbore, clearance hole, boss, floor | flat, as the tub |
| `coupon_screws_top.stl` | one of the island's screw bosses with its 1.6 mm pilot, cut from the real island | face down, as the island |
| `test_column.stl` + `test_rings.stl` | a 5.0 mm column (as the screw bosses) and four rings over it, 0.2, 0.3, 0.4 and 0.5 mm a side (engraved) | as exported |

PLA, 0.2 mm layers, 3 walls; no supports except under the board mock's USB-C stand-in
(`overhangs.py` decides). Print the coupons with the same settings you'll use
for the case: they're how those settings get tuned.

## Have on hand

The real parts, as far as they've arrived: the ER-TFT024IPS-3 panel, one Hirose FH12-50S-0.5SH(55)
connector (LCSC C202116), both DS Lite silicone sets and caps, the 403450 cell with its plug, the
speaker, 0.15 mm double-sided tape, a USB-C cable, calipers.

## Test in this order

**0. Buttons, quickly (your caps and silicone, about 1 hour of printing in all):**
- `tune_openings`: drop the D-pad into each of the three openings from the face side. Pick the
  tightest one it moves freely in without rattling and set `dpad_gap` in `case.scad` to that number
  (now 0.65: 0.5 was too tight in the first print, 0.8 too loose in the next). Put each of A, B, X and Y into its keyed hole
  from the back: the flange's tab must drop into the slot and each notch onto its rib, and it must
  not fit any other hole. If a rib fights a cap whose notch is elsewhere, note the angle and fix
  `abxy_notches` (or set `notch_keys = false`). The three plain holes tune `cap_hole_d` (now 8.0,
  Alex's DS Lite faceplate's, 2026-10-21).
- `tune_stack_*`: silicone on the board slice, cap in the deck slice, deck slice on its legs. Try the
  three: keep the one where the cap stands about 1.4 mm proud of the deck, presses cleanly and
  springs back, and set `dome_h` to its number. If the D-pad then sits higher or lower than A/B/X/Y
  wants, `dpad_lift` moves only the D-pad's lip.

**1. Coupons first (about 2 hours of printing):**
- **Seam:** lower `coupon_seam_deck` over `coupon_seam_tub`. The arm's skirt should slide down over
  the tongue with no force, the key into its pocket, and sit flush with the tub's outside, a thin
  even line between them (`seam_fit`, `seam_gap`).
- **Where the arms meet:** `coupon_meet_arms`' two pieces face down, seat 2's first, then seat 1's,
  its lap going down behind the other's skirt; `coupon_meet_top` face down beside them, its wedge
  under their lips; `coupon_meet_tub` over them. The two skirts should lie flush, no light through
  the joint.
- **Panel frame:** drop the real panel (or the dummy) into `coupon_frame_top`, glass down. It should
  go in with a little play, and the active area should sit centred in the window with only a thin
  black border showing; the FPC should pass the notch. Tune `panel_fit`, `window_margin`,
  `aa_to_top`, `fpc_x`.
- **Buttons:** build seat 1's end upside down with the real silicone and caps: the deck coupon face
  down on a build bar, caps into it from behind, silicone on them, the top coupon (the island's end)
  face down into it from behind (its wedge settles under the deck's lip: it should drop in, centre
  itself and not rock), the board coupon
  face down on top (the pills should land on the engraved contact rings), the tub coupon over it. Check each cap stands about 1.3 mm proud of the deck, can't
  fall out or turn (its flange tab sits in the pocket's slot), presses cleanly and springs back, and that the D-pad's pills sit on its four rings. The
  design assumes a D-pad pill spacing of 14.0 mm (another source says 13.5): measure it. Tune
  `dome_h` (sets the deck height), `piece_d`, `dpad_pitch`; the cap numbers from `case/DSLITE.md`
  (`cap_*`, `pocket_*`, `slot_*`, `dpad_*`) if a real cap disagrees.
- **USB-C:** the arm coupons face down, the top coupon (the island's ear) face down between them
  (its laps behind their skirts), the board coupon face down into it, the tub coupon over it (its tab
  rises into the ear's notch, the board's tab in its slot). The opening is the receptacle's own shape, `usb_gap` (0.25 mm) round its shell, and
  the shell's face sits 0.15 mm under the case's surface (`usb_recess`): the board now has a tab
  that carries the receptacle out to the wall, instead of leaving it 1.15 mm inside. Too tight or
  too loose round the stand-in: `usb_gap`. A real plug's overmold should rest flat on the case.

**2. Everything together (about a day of printing), upside down:**
1. Tape the real FH12 connector on the board mock's J3 outline (mouth towards seat 1, pin 1 at the
   marked end).
2. The two build bars on the table, about 126 mm apart; seat 2's deck face down on them, then seat
   1's beside it (its left arm's lap goes down behind seat 2's skirt). Caps into the decks from
   behind, silicone on them. The speaker (or its dummy) face down into the island's pocket over the
   grille; the island face down into the decks' window from behind, its face on the table.
3. Panel glass down into the island's pocket on its tape, centred by the frame. Slide its FPC
   into the connector with the board mock held on edge beside it. The design expects a little
   slack; if the tail bows up under the panel or can't reach, note by how much (`tip_range`).
4. The cell (or its dummy) into the tub's floor pocket on tape, against its right wall. Board mock
   face down onto the bosses; tub over it all (the cell drops into the board's hole, the tub's
   crush ribs centre the board); six M2 x 6. The case should measure 9.55 mm at the screen, 7.7 at
   the ends, and nothing should rattle.
5. Press every button: each should click through and spring back.

**3. Measure and record** (design_review.md, "Measure before ordering"): the silicone's pill
spacing, pill diameter, piece outline, dome height, cap flange and height; the panel's outline,
thickness, active-area offset, where the folded FPC's tip lands and how deep the FPC goes into the connector; the
cell's size with its protection board and its plug's polarity; the speaker's diameter, thickness
and lead exit. Enter them in `case.scad` (the values marked ESTIMATE), then run
`case/export.py --stl` and `case/check_fit.py`. If board positions moved, rebuild the board (the
pipeline in the ref_design README) before ordering.

## What the kit can't tell you

Nothing electrical. The membrane contacts have no copper here, so whether the silicone pills
actually close the combs needs real copper: a small test board of contacts, or the first boards.
The USB-C receptacle is a solid envelope, so plug depth is only tested at the wall (the coupon).
