# Reference design: the complete version

Claude's full design of the two-player handheld, built in parallel with Alex's
hand-made learning project (`hardware/prod_board/`, `hardware/prod_case/`),
which this folder never touches.

"Finished" here means a design package ready to order a first prototype:
schematic, routed board, fab outputs, case, BOM, docs. It cannot mean
"proven": parts marked MEASURE below need a physical sample first.

## Decisions (locked unless noted)

The 2-layer build applies the cost changes Alex approved on 2026-09-24 and has been
`main` since that day; tag `archive/ref-design-4layer` keeps the 4-layer design it started from.

| Area | Choice | Why |
|---|---|---|
| Panel | EastRising ER-TFT024IPS-3, ST7789V, 240x320 IPS, 42.72 x 59.46 x 2.3 mm (2026-10-21; the 3.5 in ER-TFT035IPS-6, ILI9488, 54.50 x 83.00, until then) | the case is a DS Lite's length (132.7 mm, was 156.2) with the width the buttons set; same maker, same 50-pin FPC into J3: the pin map changed (K3/K4 on pins 4-5, IM3-0 on 6-9 strapped 1110, D/CX and SCL swapped on 36/37) and R61 went to 2.4k for 70 mA |
| Panel FPC | 50 contacts, 0.5 mm pitch, in a Hirose FH12-50S bottom-contact ZIF (J3), assembled by JLC | no hand soldering per unit; a cracked panel swaps in a minute |
| Backlight | 4 LEDs in parallel on four cathode pins, anode on VSYS; a linear current sink per pair of cathode pins (MCP6002 + AO3400A + 2.2 ohm), 35 mA each, set by one GPIO's filtered PWM | constant current down to the 3.5 V cutoff, DC in the LEDs, all Basic/Preferred parts; replaced the AP3156 charge pump, out of stock on 2026-10-07 (stability simulated: board/sim/backlight.py) |
| MCU core | RP2354B (2 MB flash inside) + ABM8-272-T3 + AOTA 3.3 uH, in the left bezel since 2026-10-21 (the cell's hole took the middle) | Raspberry Pi RP2350B minimal design (MIT, 2 layers like this board), copied as drawn; no flash chip to route; its side decouplers placed by hand beside it |
| Charger | BQ24073 power path (2026-10-21, row 16): OVP 6.6 V, OUT 4.4 V into VSYS, 475 mA input limit, 270 mA charge, 5 h timer, TS on a B3950 NTC (TH1) at the cell hole's right edge (3.5-47 C) | replaced the TP4056 and its load sharing (hot-plug rings into VSYS, no timer, termination lost under load); about +$0.80 a unit at 10 |
| Cell | 403450, about 700 mAh, in a 54.0 x 37.0 mm hole in the board (2026-10-21, row 19): across the board between seat 2's deck and J3's range, 0.5 mm round a worst-case 52 x 36 cell plus 1 mm at its left end for its leads to J2 (PicoBlade, 3.4 mm) at the hole's seat-1 corner; its top at most 1.6 mm over the board, its bottom in a 0.6 mm pocket in the tub's floor (case/CELLCUT_FOR_CASE.md; the case took it 2026-10-22) | 9.55 mm at the screen and 7.7 at the ends (the floor pocketed under the cell), against 13.35 and 11.5 (Alex, 2026-10-21) |
| 3V3 | RT9013 LDO on the power latch (the ME6211 until 2026-10-22) | $1.40 a unit cheaper than the buck-boost; firmware shuts down at 3.5 V as computed (the ADC reads against 3V3, which follows the cell once the LDO drops out) |
| Audio | The FM8002A into INGHAi's GSPK151035PN on J4 (2026-10-22): the speaker lies on the board's top in the right bezel beside the screen and plays up through three slots in the face; the case holds it in a pocket in the top piece's underside (case/SHELL.md) | Alex chose the side bezel over the side wall (row 7 dropped); for a while (row 19 on 2026-10-21) there was no audio at all |
| Buttons | 16 x DSL_button_4ring (D-pads, A/B/X/Y) read through a 74HC165 per seat, chained (SR_PL GPIO28, SR_CLK GPIO29, SR_DATA GPIO30, 10k array pull-ups); Start/Select on 4 mm metal snap domes (HYP 600-415S-260, row 4) on their own GPIOs; either seat's Start turns the device on (row 1, D8/Q7) | only load, clock, data and 3V3 cross the strips beside the cell's hole (2026-10-21) |
| Pin map | each pin faces where its net goes, in fan-out order; Raspberry Pi's pin escapes copied with the core | two layers route completely (docs/pinmap.md) |
| Power switch | seat 1's Start contact joins VSYS to the latch; MCU hold or USB raise PWR_EN; releasing with hold low turns off | a reset drops the hold, so a crash turns it off |
| USB | one USB-C, charge + data, SRV05-4 ESD referenced to 3V3, 27 ohm 0402 series in Raspberry Pi's spots | one Extended part ($3 an order): the 27 ohm 0402 (C25100; 22 ohm until the 2026-10-05 pin review, and a 27 ohm 0603 doesn't fit the copied layout) and the ESD array was the second from 2026-10-07 (ProTek's Preferred SRV05-4, C85364, ran out, so U1 became Semtech's Extended SRV05-4.TCT, C13612) until 2026-10-20, when U1 became hongjiacheng's Preferred SRV05-4A (C20615829, Alex): same pinout, no setup fee |
| Debug | JST SH SWD and UART (Raspberry Pi debug pinout), BOOTSEL and RESET under pinholes in the top piece; SWD again on six 2.54 mm pogo pads on the back (J7: 3V3, SWCLK, SWDIO, RUN, BOOTSEL, GND); probe pads on every rail (VBUS, VSYS, VBAT, 3V3, 1V1), the panel's SPI, BL_REF, PWR_EN, LS_GATE, AMP_SHDN and two GND clips; cut jumpers isolate the 3V3 regulator, backlight, amplifier, panel logic and cell (JP1-JP5) | no LEDs (approved); pads and jumpers added after Alex's debug-readiness review (2026-10-19) |
| Board | 2 layers, 1.6 mm, ENIG, every hole >= 0.3 mm; parts on the top only | $3.34 a board at 10, against $9.74 for the 4-layer one with 0.25 mm holes |
| Case | 77.4 x 132.7 x 9.55 mm (7.7 at the decks; 2026-10-22, for the board with the cell in it): a top piece and two decks (one part) curve over the sides and meet the tub (the back) flush at one seam low on the walls, over the tub's tongue (3.0 tall, 2.5 at the decks, since 2026-10-22), which keys them: four tabs into the top piece, a post in each corner that pushes a deck onto its wedges, grip ribs, so the parts hold together before the screws; each deck seats into the top piece from behind on a 45 degree wedge; built upside down; six M2 x 6 screws from the back through the board, four into the top piece beside the glass (no shoulders since 2026-10-22) and one into each deck; no hooks; crush ribs on the tub centre the board; the cell in a pocket in the tub's floor, the speaker under a grille in the right bezel; every part prints without supports, every edge and the step to each deck an arc ending in a chamfer at `overhang` (45 until the 6_tests ladder) (case/SHELL.md, 2026-10-21); the USB-C opening's lower half in a tab of the tub; the button openings, lips and walls from Alex's DS Lite faceplate (2026-10-21), keyed by tab and notches; no lens, light pipes or power cap | Alex's notes 10, 13, 14 and 15 (2026-10-21); FDM at home, about $1.50 of filament |
| Schematic | generated from `board/design.py` by `tools/build_sch.py`; checked by `check_netlist.py` and ERC | the data is the source of truth |

## Phases

- [x] 1 Parts: datasheet-verified pinouts, LCSC numbers (`board/design.py`)
- [x] 2 Case layout revised for real parts (`case/case.scad` owns every position; `case/export.py`)
- [x] 3 Schematic, hierarchical (9 sheets), netlist = design.py, ERC 0 errors
- [x] 4 Footprints not in KiCad libs (`board/lib/battler.pretty`, with 3D models)
- [x] 5 Board: placement, RP2350 core copied from reference, routing, pours, DRC clean
- [x] 6 Fab outputs: gerbers, drill, BOM, CPL, 3D
- [x] 7 Case: tub and lid (snap fit), STLs; board fit check (`case/check_fit.py`)
- [x] 8 Docs: README, BOM, assembly, bring-up, pin map, design review, Konnect notes
- [x] 9 Fit kit to print: board mock, part dummies, case coupons (`case/fitkit/`)

## To do (from 2026-10-20)

Next:
- [x] Board with the cell in it (row 19, 2026-10-22): routed clean, all board checks pass
- [x] The case for it ([case/CELLCUT_FOR_CASE.md](case/CELLCUT_FOR_CASE.md), done 2026-10-22,
      case/SHELL.md): 77.4 x 132.7 x 9.55 mm (7.7 at the decks), the cell's floor pocket and ribs
      under the strips, the side screws in shoulders of the top piece, M2 x 6, USB-C 1 mm out, the
      crush ribs on the tub, the speaker's pocket and grille, smooth face rounds. check_fit passes
      against the final board; placement.py's WIDEN now comes out 0 (the case carries the 72.4 board)
- [x] The side screws back inward (Alex, 2026-10-22: no shoulders; board 2877462): H1/H2 at
      (+-32.5, 29), H3 (25.9, -21.95) between the speaker and J4, H4 (-33.0, -15.2), all in the top
      piece; the shoulders and the decks' notches went. The tub's bosses at H2 and H4 are 5.0 (1.3
      to the cell's hole, 0.7 inside the board's edge). The speaker (2.1 up), J4's pocket, the USB-C
      opening (2 mm up), the debug pinholes and J5/J6 follow. check_fit passes. The speaker stays in
      the top piece: J4 is plugged with the board nearly in place and unplugged before the parts
      come apart (its 35 mm leads: 14 mm round H3's boss closed, about 24 with room to plug in)
- [x] The joints, after Alex printed the shell (2026-10-22: "the top pieces need to flow better and
      interlock", a taller inner wall, tabs on the tub wall; and it needed the screws to fit together
      at all, where 4df3f96's held before them). What got looser: only the board and its crush ribs
      (4df3f96's board sat inside the skirts, pressed into ribs on the top piece and both decks, and
      tied them together; the fits, the wedges and the 1.5 tongue were the same). Now the tongue is
      3.0 (2.5 at the decks, leaning in under their round; it was 1.5), with four tabs into pockets in
      the top piece, the top piece's skirt keyed over the USB-C notch, a post in each corner that
      pushes a deck onto its wedges (no clearance there; `deck_gap` 0.15 on the straight faces), and
      twelve grip ribs 0.05 short of the skirts (one across each side joint, so the skirts lie flush).
      positions.json unchanged, check_buttons the same 11 known failures, check_fit passes, every
      part manifold and support-free (case/SHELL.md, "The joints"). To check on the next print: the
      tub presses on, holds the case together lifted by the top piece, and eases off; the decks sit
      on their wedges; `grip_reach` (GUESS)
- [x] Row 4, metal snap domes under Start/Select: the board has them (SW20-SW23); the case has
      `ss_metal_dome` and `ss_plunger_gap` and two asserts (the dome fits the pad's well; the
      plunger rests clear of the apex). `ss_lift` stays 0.35 on the assumed 0.36 plunger height.
      Still open: measure the Start/Select strip's plunger height (g) with the fit kit's coupon; if
      it is under 0.36, raise `ss_lift` by 0.36 - g (at most 0.15, or 0.45 with a 0.3 chamfer)
- [x] Row 7, the speaker out of the right wall: dropped. Alex chose the side bezel (2026-10-22): the
      speaker lies on the board's top and plays up through a grille in the face
- [x] Rows 18 and 19: row 18 (keep the speaker beside the cell) didn't route with every part on the
      top; row 19 (the cell in the board, no speaker) did, and the speaker came back on the board's
      top in the right bezel (2026-10-22). Board final at f0b014b, the case for it after
- [x] Router reliability: `route.py 80` passed 1 run in 6 on the J2 rebuild. Most failures leave J1's
      outer rear shell leg (SH at 134.95, 104.32, on the USB tab) cut off from both GND pours, which
      the repair pass can't close; the rest leave a long seat-1 line to U4 open (P1_UP, P1_LEFT,
      P1_RIGHT, P1_SELECT). Pre-route J1's shell legs to GND like the other GND pads, and give
      those lines room, so a single run passes
      Done 2026-10-20: route.py joins J1's shell legs on the back and gives each a via
      (usb_shell_ground), and the maze finish can take one other net's copper up anywhere along a
      long connection (corridor_ripup). With them 5 runs in 6 passed (2 of 3, then 3 of 3; the
      failure was P1_UP, before corridor_ripup existed), and J1 never failed again. Corridor
      rip-up didn't fire in the 3 runs since, so it is untested in a real run
- [x] U1: swap Semtech's SRV05-4.TCT (C13612, Extended, $3 an order) for hongjiacheng's SRV05-4A
      (C20615829, Preferred, about $0.03): same pinout and 1.0 pF; ±15 kV against ±30 kV ESD, still
      above IEC 61000-4-2 level 4. `design.py` only, then check_pins against its datasheet; no copper
      change. Update the USB row above
- [x] Parts JLC stocks, for a 30-board order with no pre-orders (2026-10-22, Alex): U1 back to
      Semtech's SRV05-4.TCT (C13612, Extended), U3 the RT9013-33GB (C47773, Extended) for the
      ME6211. Same pinouts; `design.py`, the pin reviews, sync_pcb; no copper change, DRC clean.
      J5/J6's Megastar ZX-SH1.0-3PWT (C7430445) clone is not swapped: its land pattern and body
      differ from the SM03B-SRSS-TB footprint (tabs 0.475 mm nearer the pins, 3.15 mm tall)
- [x] Schematic: every power:GND symbol hides its "GND" value (77 of them), so a ground reads as a
      bare triangle. Show it (schematic generator: for the schematic session)
- [x] Move the speaker to a long side and play it out of the side wall (Alex): the floor grille
      (spk_pos, 25 holes of 1.4 mm) faces the table when two players share the screen. The speaker (the
      GSPK151035PN below) lies flat on the tub floor beside the cell (x 17.5 to the wall at 35.7: 18 mm) with a sealed channel or slots out
      through the wall. The right side has USB-C at y = 0; the left has the SWD/UART connectors and
      the BOOTSEL/RESET pinholes, so pick a y clear of them, the hooks (y ±30) and the bosses (y ±36).
      Its leads to J4 and rib_free move with it
- [x] The speaker is INGHAi's GSPK151035PN (C530539, Alex): 15 x 10 x 3.5 mm, 8 ohm, 0.5 W, 90 dB,
      about $0.42, with 35 mm leads crimped into a 1.25 mm 2-pin plug (a PicoBlade-pattern copy).
      No soldering at assembly (Alex): J4's wire pads become a 1.25 mm PicoBlade-style header that
      JLC places. Keep J4 out of reach of the cell's plug, which would also fit it and put the cell
      across the amplifier's outputs. It fits flat under the board (4.5 mm of room); its sound
      leaves its face, so a printed channel takes it to the side-wall outlet. Before ordering
      boards, try a sample speaker's plug in a genuine 53261-0271 header. Update docs/bom.md
      (speaker row) and docs/assembly.md (no soldering)
- [x] Remove the tub's four feet (6 mm, 0.8 mm high, outside the floor) (Alex). Free to do once the
      speaker leaves the floor
- [x] USB-C opening (Alex, from the viewer): its outline still doesn't follow the receptacle's
      shell, "a different kind of oval". The opening is a full stadium (usb_hole: ends of radius
      half its height); match it to the TYPE-C-31-M-12 shell's real profile from its drawing or 3D
      model (end radius and flats) with usb_gap all round
- [x] USB-C: the lid's part of the closure over the port looks like a single layer that light
      shows through (Alex). Check the tongue's thickness above the opening and at the outer face
      (the lid's outline sits tub_lip + seam_fit inside the case's, so the tongue may stop short
      of the surface), then thicken it or let the tub's skin close over it
- [x] PLAN.md's Cell row still describes the plug hole "in front of J2": it now sits a plug's length
      past J2 (5.5 mm, the cell 5 mm towards seat 2)

Later (Alex: wait until the other edits settle):
- [ ] Narrow the case (77.4 mm since 2026-10-22, when the board grew 2 mm for its strips beside
      the cell's hole; a DS Lite is 73.9): only with a re-placed board, once a real silicone piece
      measures 26 mm or less
- [ ] The battery plug on the fit kit: a pocket in the board mock for a real 53261-0271 header
      (or a printed shroud) to try the plug's tilt from the hole into J2. The plug's size is from
      distributor listings, and plug_out = 3.0 in case.scad is an estimate
- [ ] End bezels: seat 1's 10.1 mm against seat 2's 5.5 (the panel's own border is 7.06 against
      2.5); matching them makes the case 4.6 mm longer (tabled 2026-10-20)

Buttons (from Alex's pad measurements, 2026-10-20; check_buttons fails on these until they're settled):
- [ ] A/B/X/Y stems against the domes: a cap resting at dome_h puts its 1.3 mm stem's tip 0.45 mm over
      the board, so a 2.8 mm dome would be pushed down about 2.35 mm at rest (caps_silicone, pressed
      A/B/X/Y). Either each dome's top has a recess the stem fits (Alex checking), or the flanges must
      sit higher (dome_h, the deck and the tuning heights move)
- [ ] D-pad pivot: rocked on its far edge (our model), the D-pad's flange sinks 0.16 to 0.17 mm into the
      pad's 1.7 mm centre collar in every direction (pressed_silicone D-pad). A DS Lite D-pad probably
      pivots on that collar: model the pivot on it (dpad_lift, dpad_tilt) once the collar is measured
- [ ] Start/Select: pressed, the cap's flange meets the pad's estimated wells (pressed_silicone ss).
      Settle it with the Start/Select pad's measurements
- [x] Alex's DS Lite faceplate (`~/Downloads/ds/front.stl`, 2026-10-21) is the basis for the buttons:
      measured in `case/DSLITE.md`. The case takes its openings and 45 degree chamfers (A/B/X/Y 8.0
      holes, 0.6 chamfer; D-pad 0.8 chamfer; Start/Select 3.8 holes, 0.6 chamfer), its 1.6 lip (caps
      1.41 proud, decks 3.90 over the board, a 1.9 step), its wall shapes (the A/B/X/Y clover's runs
      and fillets, a D-pad wall inside the rim, one Start/Select frame round two cells, a flange
      pocket over each), its centre-square roof, and its Start/Select lift. `dpad_gap` 0.65 (faceplate 0.5,
      too tight on the first print; 0.8 too loose on the next). Kept over it: `dpad_pocket_gap` 0.5 (0.25), `pocket_d` 9.2 (9.18), `dpad_lift` 0.35
      (0.4), `dome_h` 1.75 (1.70), the walls 0.15 over the web, the deck's thickness, the notch ribs
      and the board's pitches (15.0, 14.0, 7.5 against its 15.13, 7.85)
- [ ] Faceplate questions for Alex: (1) the D-pad pad's shape along an arm (the faceplate's wall
      says round-ended stadiums 6.8 out, our model says circles round the domes 7.0 out): its width
      across one lobe 5 mm in from the tip settles it; (2) do the A/B/X/Y flanges have a flat, and is
      the tab 45 or 52 degrees from it (the faceplate's slots are at 52; the lid's wedge takes both);
      (3) the Start/Select caps' body and flange (now estimates from the faceplate's 3.8 hole and 5.35
      pocket); (4) try the 8.0 holes and the 0.65 D-pad opening in `tune_openings` (7.6/7.8/8.2 and 0.55/0.65/0.75)

Alex's notes on the print from the kit before f3d5863 (2026-10-21), the ones for the case; the
shell's redesign (a shorter case for a 2.4 in panel, the top curving over the tub, decks built
upside down) is done (case/SHELL.md) and takes the answers of the `6_tests` plate:
- [ ] 5 Openings: confirm the 0.65 D-pad opening and the 8.0 A/B/X/Y holes with real caps
      (`tune_openings`, on `6_tests`). Best guess committed 2026-10-21: `cap_hole_d` 8.0 and
      `dpad_gap` 0.65 kept, marked GUESS in case.scad and in case/SHELL.md ("Best guesses until
      `6_tests`"); `6_tests` confirms or moves them
- [x] 6 "The speaker sticks out of the side of the housing": the speaker sat inside the wall, but
      the pad it lies on was cut to the case's plain outline, not the back's round, and stood up to
      1.0 mm proud of the round as a 0.6 mm slab under it. Now cut to the tub's outside
- [ ] 8 Overhang limit for the redesign: `test_overhangs` on `6_tests` (fins 45 to 75 degrees from
      vertical, a convex 6 mm round). Best guess committed 2026-10-21: `overhang` 45, the
      conservative limit, marked GUESS; `6_tests` on the P1S and the U1 confirms or raises it
      (2026-10-22: the finer `7_overhangs_020`/`_016`/`_012`/`_008` plates bracket it, 30 to 80 every 2.5
      to 5, rounds R3/R6/R10, at 0.20/0.16/0.12/0.08 mm: the answer is an angle and a layer height)
- [x] 9 "The curve up from the base ends in a vertical lip" (Alex, relayed 2026-10-21: on the
      outside, at the bottom edge, all round where the floor meets the walls). Rechecked against the
      10-20 kit's tub (case.scad of e217c6d): the mesh is the same all round (sections every 1 mm of
      the perimeter, corners too, within 0.003 mm of each other and 0.04 of the ideal profile) and
      has no lip but the speaker pad (fixed, row 6). The kit had no brim, skirt or perimeter support
      (P1S: the kit's own G-code; U1: re-sliced with Alex's profile). What the G-code did show: the
      round was built in flat 0.1 steps whose faces fall on the P1S's slicing planes, so its chamfer
      printed 0.35, 0.1, 0.3, 0.1 out per layer, pairs of layers stacked straight (0.4 risers) at the
      bed. The tub's round is now one smooth loft (2026-10-21), so every layer steps out 0.2 on both
      printers. The first-layer test (2026-10-22, P1S, 0.20 first layer): 0.15 elephant-foot
      compensation left a lip, 0.25 didn't, so the P1S kit prints 0.25. The U1 keeps 0.15 until
      it's tested (or takes 0.25 if a U1 print shows the lip)
- [x] 11 Screw holes filled in: the plate's supports filled the tub's six counterbores. Both kits
      now give supports only to the parts that need them; the P1S kit checks its G-code
      (`case/support_check.py`). `coupon_screws_tub` and `coupon_screws_top` on `6_tests` confirm it
- [x] 12 The deck insert's ring round the lid's 5.0 boss was too tight (`insert_gap` 0.15 a side).
      Gone with the new shell: each deck has its own boss, so no column passes through another
      part. The shell has the same kind of fit twice (the deck in the top piece's undercut, the tub's
      tongue in the skirts), both 0.15 until 2026-10-21; now one parameter, `slide_fit` 0.25 a side
      (`deck_gap` and `seam_fit` take it), a best guess marked GUESS; `test_rings` (0.2 to 0.5 over
      `test_column`) on `6_tests` confirms or moves it. positions.json unchanged
- [x] 10 "The top pieces should curve over the sides and meet the tub flush, or go over it" (and keep
      the width): done 2026-10-21 (case/SHELL.md). The top piece and the decks roll over through
      `face_r` and meet the tub at one seam where the back's round ends, their skirts over the tub's
      tongue; still 75.4 wide; the hooks went
- [x] 13 "The top piece's 45 degree ridge down to the control deck: a curve": done as far as the 45
      degree limit allows: the same round as the outside edges (`step_r` 3.0), an arc that ends in a
      chamfer at `overhang` near the screen face. Under a 45 degree limit a straight slope is the
      gentlest line there is, so the curve gets rounder only when `overhang` goes up after the
      6_tests ladder (one line). `overhang` 45 is a best guess (row 8) until `6_tests` confirms it
- [x] 14 No more deck cards slotted in from the front: each deck reaches the case's outline, curves
      over the end and sides to the seam (the tub supports it all round), seats into the top piece
      from behind on a 45 degree wedge, and has its own screw (the end screw). No tub lip
- [x] 15 Built upside down: board (cell and speaker plugged and stuck on its back), top piece face
      down, decks in, caps and silicone, panel, FPC into J3, board face down, tub, screws
      (case/SHELL.md, print_package_README.md)
- [x] The board for the new case: re-placed and re-routed to the new positions.json
      (case/BOARD_OUTLINE.md: 70.4 x 127.7, the six screws, the clusters moved in 11.77 mm, J3's
      whole range kept clear, the back's silkscreen outlines for the cell and the speaker). The
      core moved 2.5 mm towards seat 1 to fan its top pins out below the speaker plug's hole, the
      amplifier into the left bezel's seat-2 corner, the backlight sinks right of the core.
      2026-10-21: check_fit 0, DRC 0 errors and 0 unrouted, ERC 0
- [ ] Questions from the shell (case/SHELL.md, "Open questions"): the deck's wedge clearance and the
      seam's fit (the `0_buttons_end` and seam coupons), J3's range (`tip_range`) on a real panel,
      the speaker taped to the board's back, and whether to drop the spines (J2 and J4 into the
      side bezels)

Housekeeping:
- [ ] Delete branch ref-design-j2-hole: its commit went in as 4241580 by cherry-pick, so `git branch -d` refuses
- [ ] Alex's main checkout (~/Code/mega_blastoise) is still at 76955d2 with uncommitted changes,
      among them ref_design/board/battler.kicad_pro, which the pull touches: pull it with Alex

## Measure before ordering

- DS Lite silicone pads: A/B/X/Y and D-pad measured (Alex, 2026-10-20: outlines, 0.65 web, rim 0.4 x 1.8,
  4.0 plungers, 4.9 x 1.7 centre square, D-pad collar 3.3 wide at the top x 1.7 round a 2.4 hole, free
  domes D-pad 2.3 and A/B/X/Y 2.8, 1.5 nubs; A/B/X/Y caps' stems 2.4 x 1.3). Still open: the
  Start/Select pad (outline, dome spacing, wells); the D-pad collar's base width; whether each dome's
  top has a recess for the cap's stem; the gap under the nub; the caps' flange thickness; and the
  DS Lite shell's clover (10.45 across one opening, wall 1.8 thick?)
- An ER-TFT024IPS-3 against its drawing: outline, where its folded FPC's tip lands (the case keeps 22.5 to 25 mm in from the seat-1 edge clear), that it reaches J3 with a little slack, and which end contact 1 lands on
- A printed top piece, decks and tub: a deck dropping onto its wedges, the seam's fit, the grip
  ribs' hold (`grip_reach`) and the corner posts' push
- The 403450 cell: length with its protection board, plug polarity

