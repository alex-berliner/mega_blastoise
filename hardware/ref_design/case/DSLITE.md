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
| Keying tab | out to r 5.44-5.6, 1.8-2 wide | `cap_tab_r`, `cap_tab_w`; the lid's slot is a 30 degree wedge (`slot_a`) | measured |
| Pitch (diamond) | 15.0 | `abxy_pitch` | our gerbers, Pi Tin agrees |
| Lid hole | 7.34 (Pi Tin) | `cap_hole_d` 7.4 | shell built for genuine caps |
| Flange pocket | 9.15 across, 1.35-1.45 deep | `pocket_d` 9.2, `pocket_h` 1.4 | shell built for genuine caps |
| Lip over the flange | 1.05 straight + 0.6 chamfer | 1.71 (from `cap_proud`) | shell built for genuine caps |
| Cap above the face | about 1.3 | `cap_proud` 1.3 | shell built for genuine caps |

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
35 degrees wide). The lid cuts a slot for each tab, so a cap can't turn; it does not add ribs for
the notches (only two models, possibly not independent, show them), so a swapped cap still fits
its hole with its tab in the slot. Add the ribs once a real cap confirms the notches.

## D-pad

| | mm | case.scad |
|---|---|---|
| Cross | 17.5-17.8 x 5.8-5.9 arms, 3.34 tall | `dpad_arm_l` 17.8, `dpad_arm_w` 5.9, `dpad_h` |
| Flange | fills the corners between the arms to about r 6.5, 0.6 thick | `dpad_flange_r` |
| Lid opening | 18.2 x 6.2 (Pi Tin) | the cross plus 0.2 all round |
| Contact pitch | 14.0 | `dpad_pitch` |

No rotational keying beyond the cross was found; the pivot is a nub on the cap or a boss in
the membrane (about 2.4 across in Pi Tin's).

## Still unknown

The silicone: its outline (`piece_d`) and its height at rest (`dome_h`, board to the flange's
underside). `dome_h` sets how far the ends step down: the deck face is `dome_h` + 2.26 mm above
the board. Measure it with the button coupons before ordering.

A pointer from the DS Lite itself (a 3D scan of an aftermarket shell, Wesk on BitBuilt, and
gorgyrip's reverse-engineered USG-CPU-10 board): its face plate is 1.4-1.5 mm, the button guide
tubes end about 3.0 mm under the face, and the board's top sits about 3.0-3.4 mm under the face.
That would put `dome_h` nearer 1.1 mm than the 2.0 here, which would leave the caps about 0.4 mm
proud instead of 1.3. Inferred from an aftermarket shell with an unknown board thickness: a
reason to measure, not a number to use.

Also from the DS Lite: its face holes are 7.6 mm for A/B/X/Y and 18.9 mm across the D-pad's
cross (taken here: `cap_hole_d`, `dpad_gap`); it closes with seven screws into the face's bosses,
two of them through the board, with the face nested in a stepped wall (taken here: the six M2
screws and the lid's lip in the tub's step).

The DS Lite shell itself is 133.0 x 73.9 mm (Nintendo's figure). This case is its own shape
(the panel and the two seats set it), so only the button geometry is taken from the DS Lite.
