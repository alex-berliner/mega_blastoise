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
| Panel | EastRising ER-TFT035IPS-6, ILI9488, 320x480 IPS, 54.50 x 83.00 x 2.30 mm, about $14.20 a unit with shipping at 10 | a published datasheet and 3D model, sold by the maker; plugs in instead of being soldered (Alex, 2026-09-24; the QD3525 it replaced only came with a solder tail) |
| Panel FPC | 50 contacts, 0.5 mm pitch, in a Hirose FH12-50S bottom-contact ZIF (J3), assembled by JLC | no hand soldering per unit; a cracked panel swaps in a minute |
| Backlight | 6 LEDs in parallel on two cathode pins, anode on VSYS; a linear current sink per pin (MCP6002 + AO3400A + 2.2 ohm), set by one GPIO's filtered PWM | constant current down to the 3.5 V cutoff, DC in the LEDs, all Basic/Preferred parts; replaced the AP3156 charge pump, out of stock on 2026-10-07 (stability simulated: board/sim/backlight.py) |
| MCU core | RP2354B (2 MB flash inside) + ABM8-272-T3 + AOTA 3.3 uH | Raspberry Pi RP2350B minimal design (MIT, 2 layers like this board); no flash chip to route |
| Charger | TP4056 at 500 mA, TEMP on a board NTC over the cell (1-46 C); USB load sharing (B5819W, AO3401A, BAT54WS gate feed, 22 uF on VSYS; a second B5819W across the AO3401A carries the unplug gap); 2.2 ohm in series with the input cap against hot-plug ringing | a fifth of the BQ24074's price, no stock risk; the NTC added on request (Extended part, $3 an order) |
| Cell | 403450, about 700 mAh, Molex PicoBlade (MX1.25) plug into a 3.4 mm J2 (JST PH until 2026-10-19: too tall for the lowered lid), under the board on the tub's floor | 13.4 mm at the screen (the panel sits in a pocket in the lid, 2026-10-19), 11.1 at the ends; the plug passes up through a 5.5 x 4.5 mm hole in front of J2; chosen over the cell on the board under the screen (13.9 mm, 7.9: branch ref-design-cell-on-board) (Alex, 2026-10-09) |
| 3V3 | ME6211 LDO on the power latch | $1.40 a unit cheaper than the buck-boost; firmware shuts down at 3.5 V as computed (the ADC reads against 3V3, which follows the cell once the LDO drops out) |
| Audio | PWM (GPIO8, 8 bits at 586 kHz) through two RC stages (1k, 4.7 nF; -3 dB about 14 kHz) into an FM8002A, gain 2 | the cheapest way to keep a speaker (approved over the NS4168, which is 50 cents more and cleaner) |
| Buttons | 16 x DSL_button_4ring (D-pads, A/B/X/Y) and 4 x DSL_button_3ring (each seat's Start and Select, under the DS Lite kit's Start/Select membrane, between the clusters at the case's end), direct GPIO, internal pull-ups; seat 1's Start is also the power button | no matrix; no side switch (approved); Start/Select added and power moved off Y (Alex, 2026-10-19) |
| Pin map | each pin faces where its net goes, in fan-out order; Raspberry Pi's pin escapes copied with the core | two layers route completely (docs/pinmap.md) |
| Power switch | seat 1's Start contact joins VSYS to the latch; MCU hold or USB raise PWR_EN; releasing with hold low turns off | a reset drops the hold, so a crash turns it off |
| USB | one USB-C, charge + data, SRV05-4 ESD referenced to 3V3, 27 ohm 0402 series in Raspberry Pi's spots | two Extended parts ($3 an order each): the 27 ohm 0402 (C25100; 22 ohm until the 2026-10-05 pin review, and a 27 ohm 0603 doesn't fit the copied layout) and, since 2026-10-07, the ESD array: ProTek's Preferred SRV05-4 (C85364) ran out, so U1 is Semtech's SRV05-4.TCT (C13612) |
| Debug | JST SH SWD and UART (Raspberry Pi debug pinout), BOOTSEL and RESET under lid pinholes; SWD again on six 2.54 mm pogo pads on the back (J7: 3V3, SWCLK, SWDIO, RUN, BOOTSEL, GND); probe pads on every rail (VBUS, VSYS, VBAT, 3V3, 1V1), the panel's SPI, BL_REF, PWR_EN, LS_GATE, AMP_SHDN and two GND clips; cut jumpers isolate the 3V3 regulator, backlight, amplifier, panel logic and cell (JP1-JP5) | no LEDs (approved); pads and jumpers added after Alex's debug-readiness review (2026-10-19) |
| Board | 2 layers, 1.6 mm, ENIG, every hole >= 0.3 mm | $3.34 a board at 10, against $9.74 for the 4-layer one with 0.25 mm holes |
| Case | six M2 screws through the board clamp tub and lid (as a DS Lite); four hooks on the long walls hold the lid while it is screwed; a lip in a step at the seam; crush ribs centre the board and bosses clamp it (it fits one way only: J1 and J2 key it); the board rests on floor ribs; button decks step down 2.34 mm (the panel sits in a pocket in the lid) for DS Lite caps, keyed by tab and notches; no lens, light pipes or power cap | approved; FDM at home, about $1.50 of filament |
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
- [ ] Router reliability: `route.py 80` passed 1 run in 6 on the J2 rebuild. Most failures leave J1's
      outer rear shell leg (SH at 134.95, 104.32, on the USB tab) cut off from both GND pours, which
      the repair pass can't close; the rest leave a long seat-1 line to U4 open (P1_UP, P1_LEFT,
      P1_RIGHT, P1_SELECT). Pre-route J1's shell legs to GND like the other GND pads, and give
      those lines room, so a single run passes
- [ ] U1: swap Semtech's SRV05-4.TCT (C13612, Extended, $3 an order) for hongjiacheng's SRV05-4A
      (C20615829, Preferred, about $0.03): same pinout and 1.0 pF; ±15 kV against ±30 kV ESD, still
      above IEC 61000-4-2 level 4. `design.py` only, then check_pins against its datasheet; no copper
      change. Update the USB row above
- [ ] Schematic: every power:GND symbol hides its "GND" value (77 of them), so a ground reads as a
      bare triangle. Show it (schematic generator: for the schematic session)
- [ ] Move the speaker to a long side and play it out of the side wall (Alex): the floor grille
      (spk_pos, 25 holes of 1.4 mm) faces the table when two players share the screen. The 15 mm,
      4 mm speaker can't stand on edge (the case is 13.4 mm), so it stays flat on the tub floor
      beside the cell (x 17.5 to the wall at 35.7: 18 mm) with a sealed channel or slots out
      through the wall. The right side has USB-C at y = 0; the left has the SWD/UART connectors and
      the BOOTSEL/RESET pinholes, so pick a y clear of them, the hooks (y ±30) and the bosses (y ±36).
      Its leads to J4 and rib_free move with it
- [ ] The speaker is INGHAi's GSPK151035PN (C530539, Alex): 15 x 10 x 3.5 mm, 8 ohm, 0.5 W, 90 dB,
      about $0.42, with 35 mm leads crimped into a 1.25 mm 2-pin plug (a PicoBlade-pattern copy).
      No soldering at assembly (Alex): J4's wire pads become a 1.25 mm PicoBlade-style header that
      JLC places. Keep J4 out of reach of the cell's plug, which would also fit it and put the cell
      across the amplifier's outputs. It fits flat under the board (4.5 mm of room); its sound
      leaves its face, so a printed channel takes it to the side-wall outlet. Before ordering
      boards, try a sample speaker's plug in a genuine 53261-0271 header. Update docs/bom.md
      (speaker row) and docs/assembly.md (no soldering)
- [ ] Remove the tub's four feet (6 mm, 0.8 mm high, outside the floor) (Alex). Free to do once the
      speaker leaves the floor
- [ ] PLAN.md's Cell row still describes the plug hole "in front of J2": it now sits a plug's length
      past J2 (5.5 mm, the cell 5 mm towards seat 2)

Later (Alex: wait until the other edits settle):
- [ ] Narrow the case 1 mm a side (75.4 to 73.4 mm; a DS Lite is 73.9): USB-C to the panel frame
      1.5 to 0.5 mm, silicone edge_margin 4 to 3, about 15 edge parts re-placed, re-route. Only
      once a real silicone piece measures 26 mm or less
- [ ] The battery plug on the fit kit: a pocket in the board mock for a real 53261-0271 header
      (or a printed shroud) to try the plug's tilt from the hole into J2. The plug's size is from
      distributor listings, and plug_out = 3.0 in case.scad is an estimate
- [ ] End bezels: seat 1's 10.1 mm against seat 2's 5.5 (the panel's own border is 7.06 against
      2.5); matching them makes the case 4.6 mm longer (tabled 2026-10-20)

Housekeeping:
- [ ] Delete branch ref-design-j2-hole: its commit went in as 4241580 by cherry-pick, so `git branch -d` refuses
- [ ] Alex's main checkout (~/Code/mega_blastoise) is still at 76955d2 with uncommitted changes,
      among them ref_design/board/battler.kicad_pro, which the pull touches: pull it with Alex

## Measure before ordering

- DS Lite silicone piece outline and cap sizes (pads and caps ordered)
- An ER-TFT035IPS-6 against its drawing: outline, that its folded FPC reaches J3 with a little slack, and that contact 1 lands on J3's pin 50 (the right end seen from above, seat 1 nearest)
- The 403450 cell: length with its protection board, plug polarity
- A printed tub and lid: the snap fit
