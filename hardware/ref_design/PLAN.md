# Reference design: the complete version

Claude's full design of the two-player handheld, built in parallel with Alex's
hand-made learning project (`hardware/prod_board/`, `hardware/prod_case/`),
which this folder never touches.

"Finished" here means a design package ready to order a first prototype:
schematic, routed board, fab outputs, case, BOM, docs. It cannot mean
"proven": parts marked MEASURE below need a physical sample first.

## Decisions (locked unless noted)

The 2-layer build applies the cost changes Alex approved on 2026-09-24 and has been
`main` since that day; branch `ref-design-4layer` keeps the 4-layer design it started from.

| Area | Choice | Why |
|---|---|---|
| Panel | EastRising ER-TFT035IPS-6, ILI9488, 320x480 IPS, 54.50 x 83.00 x 2.30 mm, about $14.20 a unit with shipping at 10 | a published datasheet and 3D model, sold by the maker; plugs in instead of being soldered (Alex, 2026-09-24; the QD3525 it replaced only came with a solder tail) |
| Panel FPC | 50 contacts, 0.5 mm pitch, in a Hirose FH12-50S bottom-contact ZIF (J3), assembled by JLC | no hand soldering per unit; a cracked panel swaps in a minute |
| Backlight | 6 LEDs in parallel on two cathode pins; AP3156 charge pump, three current sinks per pin | constant current from any cell voltage; 32 levels over one GPIO (1-wire) |
| MCU core | RP2354B (2 MB flash inside) + ABM8-272-T3 + AOTA 3.3 uH | Raspberry Pi RP2350B minimal design (MIT, 2 layers like this board); no flash chip to route |
| Charger | TP4056 at 500 mA, TEMP on a board NTC over the cell (1-46 C); USB load sharing (B5819W, AO3401A, BAT54WS gate feed, 22 uF on VSYS; a second B5819W across the AO3401A carries the unplug gap); 2.2 ohm in series with the input cap against hot-plug ringing | a fifth of the BQ24074's price, no stock risk; the NTC added on request (Extended part, $3 an order) |
| Cell | 403450, about 700 mAh, JST PH 2.0, sideways on the board under the screen | 4 mm keeps the case 13.9 mm; on the board, not under it, so the board rests on the floor (Alex, 2026-10-05) |
| 3V3 | ME6211 LDO on the power latch | $1.40 a unit cheaper than the buck-boost; firmware shuts down at 3.5 V as computed (the ADC reads against 3V3, which follows the cell once the LDO drops out) |
| Audio | PWM (GPIO8, 8 bits at 586 kHz) through two RC stages (1k, 4.7 nF; -3 dB about 14 kHz) into an FM8002A, gain 2 | the cheapest way to keep a speaker (approved over the NS4168, which is 50 cents more and cleaner) |
| Buttons | 16 x DSL_button_4ring, direct GPIO, internal pull-ups; seat 1's Y is also the power button | no matrix; no side switch (approved) |
| Pin map | each pin faces where its net goes, in fan-out order; Raspberry Pi's pin escapes copied with the core | two layers route completely (docs/pinmap.md) |
| Power switch | seat 1's Y contact joins VSYS to the latch; MCU hold or USB raise PWR_EN; releasing with hold low turns off | a reset drops the hold, so a crash turns it off |
| USB | one USB-C, charge + data, SRV05-4 ESD referenced to 3V3, 22 ohm 0402 series in Raspberry Pi's spots | preferred/basic parts: no JLC setup fee (a 27 ohm 0603 doesn't fit the copied layout) |
| Debug | JST SH SWD and UART (Raspberry Pi debug pinout), BOOTSEL and RESET under lid pinholes | no LEDs (approved) |
| Board | 2 layers, 1.6 mm, ENIG, every hole >= 0.3 mm | $3.34 a board at 10, against $9.74 for the 4-layer one with 0.25 mm holes |
| Case | tub and lid snap together on six hooks; crush ribs centre the board and bosses clamp it (it fits one way only: J1 and J2 key it); the board rests on floor ribs; button decks step down 6.0 mm for genuine DS Lite caps, keyed so they can't turn; no lens, light pipes or power cap | approved; FDM at home, about $1.50 of filament |
| Schematic | generated from `board/design.py` via Konnect; checked by `check_netlist.py` and ERC | the data is the source of truth |

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

## Measure before ordering

- DS Lite silicone piece outline and cap sizes (pads and caps ordered)
- An ER-TFT035IPS-6 against its drawing: outline, that its folded FPC reaches J3 with a little slack, and that contact 1 lands on J3's pin 50 (the right end seen from above, seat 1 nearest)
- The 403450 cell: length with its protection board, plug polarity
- A printed tub and lid: the snap fit
