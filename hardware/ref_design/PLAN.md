# Reference design: the complete version

Claude's full design of the two-player handheld, built in parallel with Alex's
hand-made learning project (`hardware/prod_board/`, `hardware/prod_case/`),
which this folder never touches.

"Finished" here means a design package ready to order a first prototype:
schematic, routed board, fab outputs, case, BOM, docs. It cannot mean
"proven": parts marked MEASURE below need a physical sample first.

## Decisions (locked unless noted)

| Area | Choice | Why |
|---|---|---|
| Panel | Displaytech DT035CTFT-IPS-SHB, ST7796, 320x480 IPS, 54.40 x 86.24 x 3.50 mm | SPI like current firmware; full mechanical drawing |
| Panel connector | Hirose FH35C-39S-0.3SHW(50), dual contact, 0.9 mm tall | FPC folds under the panel; dual contact makes the fold side irrelevant |
| Backlight | 6 LEDs 3S2P, 11.4-13.4 V, 60 mA; TPS61160 boost set to 56 mA | PWM dimming on one GPIO |
| MCU core | RP2350B + W25Q128JVP (WSON, 0.8 mm) + ABM8-272-T3 + AOTA 3.3 uH | Raspberry Pi RP2350B minimal design (MIT); WSON so it fits under the glass |
| Charger | BQ24074 power path, 500 mA default, 1.34 A on a Type-C 1.5/3 A source (firmware reads CC), 740 mA charge | on-board NTC over the cell |
| 3V3 | TPS63001 buck-boost | uses the cell down to its cutoff |
| Audio | MAX98357A, left channel, 9 dB, 8 ohm 1 W speaker on solder pads | I2S from PIO |
| Buttons | 16 x DSL_button_4ring, direct GPIO, internal pull-ups | no matrix |
| Power switch | button, MCU hold or USB raise PWR_EN; releasing with hold low turns off | a reset drops the hold, so a crash turns it off |
| USB | one USB-C, charge + data, USBLC6 ESD | |
| Debug | JST SH SWD and UART (Raspberry Pi debug pinout), BOOTSEL and RESET under lid pinholes, red error LED | |
| Schematic | generated from `board/design.py` via Konnect; checked by `check_netlist.py` and ERC | the data is the source of truth |

## Phases

- [x] 1 Parts: datasheet-verified pinouts, LCSC numbers (`board/design.py`)
- [x] 2 Case layout revised for real parts (`case/case.scad` owns every position; `case/export.py`)
- [x] 3 Schematic, hierarchical (9 sheets), netlist = design.py, ERC 0 errors
- [x] 4 Footprints not in KiCad libs (`board/lib/battler.pretty`, with 3D models)
- [ ] 5 Board: placement, RP2350 core copied from reference, routing, pours, DRC clean
- [ ] 6 Fab outputs: gerbers, drill, BOM, CPL, 3D
- [x] 7 Case: tub, lid, power cap, lens outline, STLs; board fit check (`case/check_fit.py`)
- [x] 8 Docs: README, BOM, assembly, bring-up, pin map, design review, Konnect notes

## Measure before ordering

- DS Lite silicone piece outline and cap sizes
- Panel sample against the drawing
- BQ24074 stock at JLC (217 in July 2026)
