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
| Panel | EastRising ER-TFT035IPS-6, ILI9488, 320x480 IPS, 54.56 x 82.84 x 2.15 +-0.15 mm (the datasheet's outline drawing, page 6; 2.45 in the case); back since 2026-10-26 (the 2.4 in ER-TFT024IPS-3 from 2026-10-21 to then) | cost, minimal bezel and height at the size Alex wants; same maker, same 50-pin FPC into J3 and the 3.5 in pin map of the first design (K3/K4 and IM3 not connected, IM0-2 high, SCL on 36, D/CX on 37), R61 3.74k for 56 mA a sink (worst case 117.7 mA over the panel's 120). Outline, active area and the 4.47 mm neck are the drawing's; the FPC's body and tail and fpc_low (1.05 mm under it) are ESTIMATES |
| Panel FPC | 50 contacts, 0.5 mm pitch, in a Hirose FH12-50S bottom-contact ZIF (J3), assembled by JLC | no hand soldering per unit; a cracked panel swaps in a minute |
| Backlight | 6 LEDs in parallel on two cathode pins, anode on VSYS; a linear current sink per pin (MCP6002 + AO3400A + 2.2 ohm), set by one GPIO's filtered PWM | constant current down to the 3.5 V cutoff, DC in the LEDs, all Basic/Preferred parts; replaced the AP3156 charge pump, out of stock on 2026-10-07 (stability simulated: board/sim/backlight.py) |
| MCU core | RP2354B (2 MB flash inside) + ABM8-272-T3 + AOTA 3.3 uH, in the left bezel since 2026-10-21 (the cell's hole took the middle) | Raspberry Pi RP2350B minimal design (MIT, 2 layers like this board), copied as drawn; no flash chip to route; its side decouplers placed by hand beside it |
| Charger | BQ24073 power path (2026-10-21, row 16): OVP 6.6 V, OUT 4.4 V into VSYS, 475 mA input limit, 270 mA charge, 5 h timer, TS on a 10k B4460 NTC (TH1, TCTR0805F10K0F4460T; the hot edge reads 48.2 C worst case against the 45 C limit: no stocked 10k NTC is steeper) at the cell hole's right edge (1.05 mm centre to edge: its pads keep 0.30 from it) (3.5-47 C) | replaced the TP4056 and its load sharing (hot-plug rings into VSYS, no timer, termination lost under load); about +$0.80 a unit at 10 |
| Cell | 403450, about 700 mAh, in a 37.8 x 54.8 mm hole in the board, turned 90 degrees (2026-10-27, Alex: rotate it; long side along the board, x -18.9 to 18.9, y -6.8 to 48 since 2026-10-27, 1 mm towards seat 2 to widen the gap to J3 (case.scad `cell_dy`; 2 mm puts the cell's top into the ends' ceiling), was -7.8 to 47; the tub's pocket 0.9 inside it at the sides and the seat-1 end, 1.9 at the lead end: a worst-case 52 x 36 cell with the tolerance stack) (2026-10-21, row 19): across the board between seat 2's deck and J3's range, 0.5 mm round a worst-case 52 x 36 cell plus 1 mm at its left end for its leads to J2 (PicoBlade, 3.4 mm) at the hole's seat-1 corner; its top at most 1.6 mm over the board, its bottom in a 0.6 mm pocket in the tub's floor (case/CELLCUT_FOR_CASE.md; the case took it 2026-10-22) | 9.55 mm at the screen and 7.7 at the ends (the floor pocketed under the cell), against 13.35 and 11.5 (Alex, 2026-10-21) |
| 3V3 | RT9013 LDO on the power latch (the ME6211 until 2026-10-22) | $1.40 a unit cheaper than the buck-boost; firmware shuts down at 3.5 V as computed (the ADC reads against 3V3, which follows the cell once the LDO drops out) |
| Audio | The FM8002A into INGHAi's GSPK151035PN on J4 (2026-10-22): the speaker lies on the board's top in the right bezel beside the screen and plays up through three slots in the face; the case holds it in a pocket in the top piece's underside (case/SHELL.md) | Alex chose the side bezel over the side wall (row 7 dropped); for a while (row 19 on 2026-10-21) there was no audio at all |
| Buttons | 16 x DSL_button_4ring (D-pads, A/B/X/Y) read through a 74HC165 per seat, chained (SR_PL GPIO28, SR_CLK GPIO29, SR_DATA GPIO30, 10k array pull-ups); Start/Select on 4 mm metal snap domes (HYP 600-415S-260, row 4) on their own GPIOs; either seat's Start turns the device on (row 1, D8/Q7) | only load, clock, data and 3V3 cross the strips beside the cell's hole (2026-10-21) |
| Pin map | each pin faces where its net goes, in fan-out order; Raspberry Pi's pin escapes copied with the core | two layers route completely (docs/pinmap.md) |
| Power switch | seat 1's Start contact joins VSYS to the latch; MCU hold or USB raise PWR_EN; releasing with hold low turns off | a reset drops the hold, so a crash turns it off |
| USB | one USB-C, charge + data, SRV05-4 ESD referenced to 3V3, 27 ohm 0402 series in Raspberry Pi's spots | one Extended part ($3 an order): the 27 ohm 0402 (C25100; 22 ohm until the 2026-10-05 pin review, and a 27 ohm 0603 doesn't fit the copied layout) and the ESD array was the second from 2026-10-07 (ProTek's Preferred SRV05-4, C85364, ran out, so U1 became Semtech's Extended SRV05-4.TCT, C13612) until 2026-10-20, when U1 became hongjiacheng's Preferred SRV05-4A (C20615829, Alex): same pinout, no setup fee |
| Debug | JST SH SWD (Raspberry Pi debug pinout; the UART's J6 went on 2026-10-24), BOOTSEL and RESET under pinholes in the top piece; SWD again on six 2.54 mm pogo pads on the back (J7: 3V3, SWCLK, SWDIO, RUN, BOOTSEL, GND); probe pads on every rail (VBUS, VSYS, VBAT, 3V3, 1V1), the panel's SPI, BL_REF, PWR_EN, LS_GATE, AMP_SHDN and two GND clips; cut jumpers isolate the 3V3 regulator, backlight, amplifier, panel logic and cell (JP1-JP5) | no LEDs (approved); pads and jumpers added after Alex's debug-readiness review (2026-10-19) |
| Board | 2 layers, 1.6 mm, ENIG, every hole >= 0.3 mm; parts on the top only | $3.34 a board at 10, against $9.74 for the 4-layer one with 0.25 mm holes |
| Case | 77.4 x 132.7 x 9.55 mm (7.7 at the button ends; 2026-10-22, for the board with the cell in it): since 2026-10-25 a tub and one top piece (case/SHELL.md; the island and two decks of 2026-10-23 to 2026-10-25 are rejected, `case/SHELL_ISLAND.md`). The top is a band at the screen's height across the whole width, its two ends the DS Lite's lower screen frame in section (a sharp edge and one concave arc), the ends beyond it at the buttons' height, rolling over the sides in true arcs and down the back's round to one seam low on it (2.55 mm up), over the tub's tongue (6.6 round the ends, 5.58 along the sides), which keys it: two tabs into pockets in each side wall and grip ribs, so the parts hold together before the screws; built upside down (caps and silicone into the top from behind); six M2 x 6 screws from the back through the board, four into the top beside the glass and one at each end; no snap hooks; crush ribs on the tub centre the board; the cell in a pocket in the tub's floor, the speaker under a grille in the right bezel; the tub prints back down with no supports (its round ends in a chamfer at `overhang`, 60 since 2026-10-23, a variant until the overhang kit confirms it), the top face down with supports whose interface is Bambu Support For PLA (a second filament, a prime tower); the USB-C opening's lower half in a tab of the tub; the button openings, lips and walls from Alex's DS Lite faceplate (2026-10-21), keyed by tab and notches; no lens, light pipes or power cap | Alex's notes 10, 13, 14 and 15 (2026-10-21), "make unibody main" (2026-10-25); FDM at home, about $1.50 of filament |
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
- [x] The seam on the round (2026-10-22, Alex: the top pieces' walls over most of the back's round
      to a much lower ledge on the tub, the tongue kept): `z_seam` 4.2 to 2.55, the lowest that keeps
      the skirts' tips 0.7 thick with the tongue where it is (2.5 would leave 0.68). The skirts follow
      the round down; the tongue keeps its tops and now overlaps 4.55 (4.05 at the decks; was 2.9 and
      2.4). Beside the board's edge the skirt, fit and tongue leave 0.5 (the tongue 0.3 off the board
      below 4.2). The USB-C tab stays, rising from the new seam: the receptacle stands 0.85 into the
      skirt, so the opening's lower edge can't be the top piece's. positions.json: only `case.seam`
      changed; check_buttons the same 11, check_fit passes, manifold and support-free; P1S 1_case
      56.0 g, 1_case_decks 9.1 g, 5_unit 65.0 g (case/SHELL.md, "The seam on the round"). To check on
      the next print: the skirts' 0.7 tips, and the 4.65 tongue's stiffness off the case
- [x] The side joint (2026-10-22, Alex on the joint's outside: "These parts should flow into each
      other better"): the butt joint at each side wall became a lap, 0.85 thick and 3.0 along the
      wall, on the deck's skirt behind the top piece's (0.15 off it), its free end's top at 45
      degrees so it prints face down; the tub's tongue stops either side of it (a 5.55 slot), and the
      grip rib across the joint went (one more on each deck at y +-40). The top piece's side round
      now comes down to the deck's over 9.4 mm in a smooth S, its top chamfer carried up to the face
      (every face 45 degrees or steeper face down): the outside is the deck's at the joint. For it the
      tongue's tall part ends at y +-24.0 (was +-31.8), the left keys moved to +-20 and the top
      piece's grip ribs to +-26. positions.json unchanged; check_buttons the same 11, check_fit
      passes, manifold and support-free; P1S 1_case 55.8 g, 1_case_decks 9.2 g, 5_unit 64.9 g
      (case/SHELL.md, "The side joint"). To check on the next print: the decks still drop in, the
      skirts flush across the joint, the lap's tip clean
- [x] The tongue under the step (2026-10-23, Alex: the top piece "kind of just sits on top of the
      deck"; an indentation across their whole contact that it inserts into): each deck's wedge runs
      on down its 45 degree line to 4.6 (0.8 over the board, 0.3 over its 0402s), 2.3 in under the
      top piece, across the step and down the spine's sides and round its end; the top piece has a
      0.8 tongue in its land's plane down through slots in it (0.15 a side, 0.3 lead-ins), 2.1 to 4.0
      long between 1.5 ties, the flange beyond each slot a fin bridged from tie to tie (0.7 thick at
      its top, 0.53 of engagement). Face down a flat flange or one long groove would start in mid air,
      so the tongue is in pieces (25.9 of the step's 57.9 mm and 13.0 of the spine's sides' 23.8 per
      end). Over the board's taller parts (Q3, Q5, Q6, X1, D3, C5-C7, U8, U9) the flange stops 0.3
      over them, with no slot; the spine's end has none. The wedge stays. check_fit checks the board
      under it (case.scad's FLANGE line); positions.json unchanged, check_buttons the same 11, the
      outside unchanged; P1S 1_case 55.9 g, 1_case_decks 9.4 g, 5_unit 65.2 g (case/SHELL.md, "Alternatives
      considered"). To check on the next print: the decks drop in over the tongues, the
      fins' bridges print clean, the deck stays put along the step with the tub off. Replaced the
      same day by the lip, below
- [x] The lip under the step (2026-10-23, Alex on the tongue: "I think this tooth method isn't
      viable. I don't want them breaking."): the teeth, slots, ties, fins and the wedge are gone.
      Each deck's plate runs on 1.6 under the top piece at its own level (5.7 to 7.7, its face on the
      bed in print), across the step, down the spine's sides and round its end, to a 0.8 end wall;
      the top piece is hollow over it (0.15), the step comes straight down to the deck (no land, its
      last 0.35 vertical), and one continuous lip at its foot, 0.75 wide, goes 0.5 into a 1.05 groove
      in the deck's face (0.15 round it, a 1.05 bridge face down), from side wall to side wall, into
      the top piece's skirt over the lap. The end wall's lowest 0.5 is the register the corner posts
      push each deck onto. Thinnest: the strip beyond the groove 0.70, the skin over the hollow 0.74,
      the plate under the groove 1.35. positions.json unchanged, check_buttons the same 11,
      check_fit PASS (38 parts under the joint, the tallest 1.19 of 1.60), the tub unchanged; P1S
      1_case 55.6 g, 1_case_decks 9.7 g, 5_unit 65.1 g (case/SHELL.md, "The lip under the step"). To
      check on the next print: the decks drop in over the lip, the groove's roof prints clean, the
      deck stays put with the tub off, and the posts seat it without forcing
- [x] The 60 degree variant (2026-10-23, branch `ref-design-cellcut-60` only; Alex: "improved curves
      on the tub sides, the ridges and so forth" if the printers can do 60 degrees): `overhang` 60.
      The tub's round r 4.2 carried to 60 (its bottom chamfer 0.56 tall, was 1.23); the face rounds
      r 2.4 to 60 (was r 3.0 to 45; the top edge stays 1.757 in, so nothing inside moves); the step
      r 3.0 to 60, a 1.79 run (was 1.36). The side blend's band stays 45 (the skin over the hollow
      and J4's pocket). Fixed on the way: the joint's hollow at the side walls came within 0.07 of
      the outside; it and the decks' plate now stop 1.0 inside the walls. overhangs.py reads the
      limit from case.scad (and uses sin, not cos), the slicers' support threshold follows it.
      positions.json unchanged, check_buttons the same 11, check_fit PASS; P1S 1_case 55.6 g,
      1_case_decks 9.8 g, 5_unit 65.2 g. Depends on the overhang kit showing 60 prints clean on
      both printers (case/SHELL.md, "This branch is the 60 degree variant")
- [x] The island (2026-10-23, Alex on the side blend: "It needs to blend and flow better ... one piece
      one flow, not two"; at 60 degrees): the case's side is the decks' profile end to end. The
      screen block is an island 2.6 in from the sides (J5/J6, J2, J4 and the speaker along the walls
      allow no more; the aim was 4 to 6), one edge profile all round (the step's, an offset loft), a
      45 degree wedge 1.2 under the decks' lips. The decks are C-shaped (a core and two arms) and two
      parts: J1 forces an ear of the island at the screen's height over the USB-C on the right wall,
      where the right arms end; the left arms meet in a lap. Side round r 3.0 again. Tongue 3.03 along
      the sides, keys into the arms, two grip ribs per arm, no screw in any arm; the face-down build
      stands the decks on two printed bars (1.85). positions.json unchanged, check_buttons the same 11,
      check_fit PASS (41 parts under the joint zones, tallest 1.19 of 1.6), no part needs supports;
      P1S 1_case 55.1 g, 1_case_decks 10.1 g, 5_unit 65.1 g (one plate). To check on the next print:
      the arms don't spring out along the sides, the island seats under the lips without binding, the
      decks sit steady on the bars (case/SHELL.md, "The island")
- [x] The left ear, the hooks, the 2.8 round (2026-10-24, Alex's choices): the right ear turned round
      on the left wall (blank), the left arms end at it with laps, the arm-meets-arm lap removed; the
      left keys and grip ribs the right ones turned round, so the decks are one part printed twice
      (export.py checks seat 2's against it: 0.000 mm3). A hook on each arm (y -23.5, 23.5, -31.5,
      31.5: as near the free ends as the laps, keys and ribs allow): a 1.6 neck through a 1.9 slot in
      the tongue, a 3.6 x 0.55 head 0.15 inside the tongue, bearing 0.67, its foot 0.5 over the board
      (check_fit HOOKS: nothing within 0.3). face_r 2.8 (the arms' flat 0.40). positions.json
      unchanged, check_buttons the same 11, check_fit PASS, no case part needs supports; P1S 1_case
      55.3 g, 1_case_decks 10.1 g, 5_unit 65.3 g (one plate). To check: the hooks go in and hold
      (island_coupons.zip on the 45 branch has a hook pair at 45 and 60)
- [x] The case for the routing levers (2026-10-24, board 86bf694): the BOOTSEL/RESET pinholes to
      (-32.35, -20.7) and (-32.35, -24.35) in the left bezel below H4 (asserted against H4, the island's
      wall, the left key, hook and grip ribs, the deck's joint, the cell's hole, J2's plug, J5 and the
      panel's frame), a floor rib under both switches into H4's boss (`pin_rib`), J6 and `uart_pos`
      removed (J5 alone sets the island's 2.6: nothing freed). positions.json: only `pinholes` and
      `uart` changed (no board tool reads either). check_buttons the same 11, check_fit PASS, no case
      part needs supports; P1S 1_case 55.2 g, 1_case_decks 10.1 g, 5_unit 65.2 g (one plate)
- [x] The one-piece top is the top (2026-10-25, Alex: "make unibody main and update 3d viewer"; the branch `ref-design-unibody`):
      the island, both decks and their joints (wedge, lips, laps, hooks, ears, corner posts, build bars) are gone from
      `case.scad`, the kits, the viewer and the docs (case/SHELL.md, "Rejected"; the old notes are `case/SHELL_ISLAND.md`,
      the parts are at the tag `ref-design-island-archive`). The tub lost the lap and hook slots and the posts (nothing
      else; the keys, grip ribs and tongue stay for the top). The top is the earlier mock-up less four corner notches.
      positions.json byte for byte as before, check_buttons the same 11 known failures, check_fit PASS (its joint zones
      are now the strip between the clusters under the band's end), the wall under the side rounds is a build stage
      (`check_wall`: 0.708 slope, 0.669 sides and ends, 3D 0.666), every printed part manifold. Print kits: two filaments
      (PLA in slot 1, Bambu Support For PLA in slot 2, the top's supports' interface only; a prime tower), `1_case` is the
      tub and the top (P1S: 49.7 g PLA, 5.8 g support filament, 21 changes, 4 h 27, with the print settings of 2026-10-25 in print_package_README.md), `5_unit` one whole unit on one plate
      (75.6 g, 6 h 22), `1_case_decks` is gone. To check on the first print: the finish on the support interface, the
      top's warp (133 mm, 0.7 skirts), the caps and silicone going in from behind, the keys and grip ribs (case/SHELL.md,
      "Open questions"). docs/bom.md said M2 x 8: it is M2 x 6 (`screw_l`)
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
- [ ] 5 Openings: confirm the 0.60 D-pad opening (0.65 until 2026-10-26) and the 8.0 A/B/X/Y holes with real caps
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
      to 5, rounds R3/R6/R10, at 0.20/0.16/0.12/0.08 mm: the answer is an angle and a layer height;
      2026-10-23: the branch `ref-design-cellcut-60` is the case at 60, ready if the kit says 60)
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
- An ER-TFT035IPS-6 against its drawing (the old design's numbers, not re-read on 2026-10-26): outline, active area, its FPC's folded shape (the case keeps 24.9 to 27.3 mm in from the seat-1 edge clear, and lets parts up to 1.05 mm lie under the FPC's body), where its folded FPC's tip lands, that it reaches J3 with a little slack, and which end contact 1 lands on
- A printed top piece, decks and tub: a deck dropping onto its wedges, the seam's fit, the grip
  ribs' hold (`grip_reach`) and the corner posts' push
- The 403450 cell: length with its protection board, plug polarity

