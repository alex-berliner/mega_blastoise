# Fit kit: test the case with the real parts before ordering boards

Printable stand-ins and case coupons, all generated from `case/case.scad` and the routed board, so
they always match the design. Regenerate after any case or board change:

    python3 case/export.py --stl      # the real tub and lid
    python3 case/fitkit.py            # this folder (about 7 minutes)

![kit, exploded](kit_assembly.png)

## The parts

| File | What it is | Print |
|---|---|---|
| `../tub.stl`, `../lid.stl` | the real case | tub opening up; lid face down as exported, supports under the four button decks (they sit 6.0 mm below the screen face) |
| `board_mock.stl` | the board: outline, the six locating holes (0.2 mm oversize), the height envelope of every part 0.8 mm or taller (from its 3D model), the 16 membrane contacts and the four silicone pieces engraved, J3's outline to tape a real connector on | flat, as exported |
| `panel_dummy.stl` | the panel at its thickest (2.45 mm), active area engraved on the glass side | flat, glass side up |
| `cell_dummy.stl` | the 403450 cell (50 x 34 x 4 mm) with its lead tab at its right-hand end | flat |
| `cell_plug_dummy.stl` | a JST PHR-2 plug with a cable stub (approximate) | upright |
| `speaker_dummy.stl` | the 15 mm speaker, 4 mm thick | flat |
| `coupon_snap_tub.stl` + `coupon_snap_lid.stl` | one snap hook and its groove, cut from the real tub and lid | as exported, same printer settings as the case |
| `coupon_buttons_tub.stl` + `_board.stl` + `_lid.stl` | seat 1's end of the tub, board and lid: the button stack | as exported |
| `coupon_usb_tub.stl` | the tub wall around the USB-C opening | as exported |
| `coupon_frame_lid.stl` | the lid's window, ledge and panel frame | face down, as exported |

PLA or PETG, 0.2 mm layers, 3 walls; no supports, except under the lid's button decks (the
lid and `coupon_buttons_lid`). Print the coupons with the same settings
you'll use for the case: they're how those settings get tuned.

## Have on hand

The real parts, as far as they've arrived: the ER-TFT035IPS-6 panel, one Hirose FH12-50S-0.5SH(55)
connector (LCSC C202116), both DS Lite silicone sets and caps, the 403450 cell with its plug, the
speaker, 0.15 mm double-sided tape, the foam pad, a USB-C cable, calipers.

## Test in this order

**1. Coupons first (about 2 hours of printing):**
- **Snap:** press `coupon_snap_lid` onto `coupon_snap_tub`. It should click with firm thumb pressure,
  hold, and come apart with a blade in the gap. Too tight or cracking: `snap_barb` down (0.25 ->
  0.2) or the hook thinner (`snap_t_root`, `snap_t_tip`); too loose: `snap_barb` up (0.3). The first
  print's hooks snapped in PLA: print the lid in PETG if they still do.
- **Panel frame:** drop the real panel (or the dummy) into `coupon_frame_lid`, glass down. It should
  go in with a little play, and the active area should sit centred in the window with only a thin
  black border showing; the FPC's neck should pass the notch. Tune `panel_fit`, `window_margin`,
  `aa_to_top`, `fpc_neck`.
- **Buttons:** put the three button coupons together with the real silicone and caps: silicone on
  the board coupon (the pills should land on the engraved contact rings), caps in the lid coupon's
  holes, lid coupon down onto the tub coupon. Check each cap stands about 1.3 mm proud of the deck, can't
  fall out or turn (its flange tab sits in the pocket's slot), presses cleanly and springs back, and that the D-pad's pills sit on its four rings. The
  design assumes a D-pad pill spacing of 14.0 mm (another source says 13.5): measure it. Tune
  `dome_h` (sets the deck height), `piece_d`, `dpad_pitch`; the cap numbers from `case/DSLITE.md`
  (`cap_*`, `pocket_*`, `slot_*`, `dpad_*`) if a real cap disagrees.
  The JST plug (real or dummy) should go into J2 between seat 1's clusters, its wires through the slot beside it, without forcing them.
- **USB-C:** a USB-C cable's plug should pass the opening in `coupon_usb_tub` with some clearance.

**2. Everything together (about a day of printing):**
1. Cell (or dummy) in its pocket on the tub's floor, its lead at seat 1's end; speaker (or dummy)
   on the grille beside it.
2. The cell's lead up through the board mock's slot, then the board mock into the tub, onto the
   floor's ribs (the crush ribs on the walls centre it); the lead to J2's block.
3. Tape the real FH12 connector on J3's outline (mouth towards seat 1, pin 1 at the marked end).
   Fold the real panel's FPC under it and slide it into the connector. The design expects a little
   slack; if the tail bows up under the panel or can't reach, note by how much: `conn_rear` in
   `case.scad` moves J3.
4. Panel into the lid's frame on the tape, silicone and caps in, lid down (its lip into the wall's step): all six hooks should
   click, the lid should sit flush, nothing should rattle, the case should measure 14.6 mm at the screen, 11.1 at the ends.
5. Press every button: each should click through and spring back.

**3. Measure and record** (design_review.md, "Measure before ordering"): the silicone's pill
spacing, pill diameter, piece outline, dome height, cap flange and height; the panel's outline,
thickness, active-area offset, FPC neck width and how deep the FPC goes into the connector; the
cell's size with its protection board and its plug's polarity; the speaker's diameter, thickness
and lead exit. Enter them in `case.scad` (the values marked ESTIMATE), then run
`case/export.py --stl` and `case/check_fit.py`. If board positions moved, rebuild the board (the
pipeline in the ref_design README) before ordering.

## What the kit can't tell you

Nothing electrical. The membrane contacts have no copper here, so whether the silicone pills
actually close the combs needs real copper: a small test board of contacts, or the first boards.
The USB-C receptacle is a solid envelope, so plug depth is only tested at the wall (the coupon).
