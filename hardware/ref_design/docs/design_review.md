# Design review: REF-A

What the reference design is, why each part of it is the way it is, what has
been checked and what hasn't. Numbers here come from the datasheets in
`datasheets/SOURCES.md` and from the files in this folder.

## The device

Two players face each other across a 3.5in 320x480 IPS screen; each has a
D-pad and four face buttons under DS Lite silicone at their end. One Li-ion
cell, one speaker, one USB-C for charging and firmware. No radios.

Case 75.4 x 156.2 x 20.0 mm. Board 70.4 x 151.2 mm, 4 layers, 1.6 mm, ENIG.

## Block by block

### USB-C and charging (sheet USB_Charger)

- Sink only: 5.1k Rd on CC1 and CC2, no PD. A Type-C charger turns VBUS on
  because of them. VBUS bulk capacitance 10 uF (USB's limit).
- USBLC6-2SC6 on D+/D-, next to the connector.
- BQ24074 power path. OUT (VSYS) regulates to 4.4 V from USB, and follows the
  cell when unplugged, so the device runs with a flat or missing cell on USB.
- Input limit: 500 mA at power-up (EN2 pulled low, EN1 pulled high). Firmware can
  read the charger's advertised current on CC1/CC2 (ADC through 10k) and, on a
  1.5 A or 3 A source, select the ILIM resistor: 1610 / 1.2k = 1.34 A.
- Charge current 890 / 1.2k = 740 mA: 0.4C on the 103450 (about 1800 mAh).
- Worst-case charger dissipation (5 V in, 3.6 V cell, 740 mA): about 1.0 W. On a
  4-layer board with the thermal pad stitched to the ground plane that is roughly
  45 K above ambient, well under the 125 C point where the chip starts cutting
  current. The charger sits in the side bezel, not under the panel.
- NTC: 10k B3380 on the board, over the cell. The BQ24074 charges between
  about 0 C (2.1 V at TS) and 51 C (0.3 V at TS) with its 75 uA TS current.
  A board-mounted NTC reads the cell through 1.6 mm of FR4 and 0.5 mm of air: it
  lags a real cell-mounted NTC, which a production cell should have.

### 3V3 and the power switch (sheet Power)

- TPS63001 buck-boost: 3.3 V from a cell anywhere between 3.0 and 4.4 V
  (buck, boost or pass-through), 1.8 A switch limit, power-save mode on.
  Load: RP2350B up to ~50 mA, panel logic ~10 mA, flash, LEDs: under 100 mA.
- PWR_EN = (button) OR (PWR_HOLD from the MCU) OR (VBUS through 10k and a diode).
  All three through Schottky diodes so none back-feeds another; 100k pull-down.
  EN's absolute maximum is 7 V independent of VIN, so the USB path is safe
  even with a flat cell.
- The button also reaches the MCU through Q1 (2N7002, gate on the button node),
  so no MCU pin ever sees VSYS.
- Off state: TPS63001 off (<1 uA), amplifier shut down by its own pull-down,
  backlight driver shut down by R31, charger in battery mode (6.5 uA), battery
  divider 4.5 uA. About 13 uA total: years of shelf life.
- A reset or crash drops PWR_HOLD. On the cell alone that turns the device off,
  which is the safe failure. On USB it stays on (VBUS holds PWR_EN).

### RP2350B core (sheets MCU, MCU_Power, Debug)

- Schematic, values and placement copied from Raspberry Pi's RP2350B minimal
  design (MIT), turned 90 degrees. Its routing, vias and the small copper areas
  around the internal 1.1 V switcher are copied too (`board/tools/build_pcb.py`),
  so the most layout-sensitive part of the board is Raspberry Pi's, not mine.
- Flash: W25Q128JVP in WSON (0.8 mm) instead of the reference's SOIC (1.75 mm),
  because it sits under the panel. Its exposed pad forced one copied QSPI track
  to be re-routed.
- 12 MHz ABM8 crystal with 15 pF loads and 1k series, as in the reference.
- Debug: SWD and UART on JST SH 3-pin in the Raspberry Pi Debug Probe pinout;
  BOOTSEL and RESET buttons under pinholes; a red LED for firmware errors.
- GPIO map: [pinmap.md](pinmap.md). 39 of 48 GPIOs used.

### Display (sheet Display)

- Displaytech DT035CTFT-IPS-SHB: ST7796, 4-wire SPI, 3.3 V, 60 mA backlight of
  two strings of three LEDs (11.4-13.4 V).
- Hirose FH35C-39S-0.3SHW: dual-contact, 0.9 mm tall. The panel's FPC folds
  under the panel; with a dual-contact connector which side of the FPC faces
  up doesn't matter. Pin k of the panel lands on pin k of J3 (the EasyEDA land
  pattern was renumbered to Hirose's contact order to make that true).
- Unused RGB interface pins (7-26) are tied to ground rather than left floating;
  touch pins (30-36) are left open (this panel variant has no touch).
- SPI clock and data get 33 ohm source terminations next to the MCU.
- Backlight: TPS61160 boost from VSYS, current set by R30: 0.2 V / 3.6 ohm =
  56 mA, under the panel's 60 mA rating. PWM dimming on CTRL. 26 V over-voltage
  protection covers an unplugged panel; the 1 uF output capacitor is rated 50 V.

### Audio (sheet Audio)

- MAX98357A from VSYS: at 4.2 V into 8 ohm it can deliver about 1 W before
  clipping, which matches a 1 W speaker; gain 9 dB (GAIN_SLOT open).
- SD_MODE through 47k from a GPIO: 2.2 V selects the left channel, 0 V shuts
  down (0.6 uA). Firmware writes the mono mix to both slots, so either channel
  choice works.
- Ferrite beads in the speaker leads for emissions; the leads are short.

### Membrane contacts (sheet Buttons)

- 16 x DSL_button_4ring: two interleaved combs (0.15 mm lines, 0.20 mm gaps,
  checked by `hardware/membrane/tools/check_contact_fp.py`) under one 4.8 mm mask
  opening, ENIG. A carbon pill of 1.6 mm or more bridges them anywhere on the pad.
- Each comb leaves through a 0.2 mm stub to a via 3.3 mm from the contact's
  centre; nothing else (no vias, no pour) inside 2.9 mm.

## Board

- Stack-up: F.Cu signals and parts, In1 solid GND, In2 solid 3V3, B.Cu signals
  and GND pour; GND stitching vias on a 5 mm grid.
- Rules: 0.15 mm track and clearance (0.1 mm minimum inside the copied core),
  0.25 mm vias, 0.3 mm power tracks, 0.2 mm USB. All within JLCPCB's standard
  4-layer process.
- Heights: under the panel every part is 1.2 mm or less (the panel sits 1.4 mm
  above the board). Taller parts live in the side bezels or between a seat's
  clusters: USB-C (3.3 mm), power switch (3.3), debug connectors (2.9),
  inductors (1.2), battery connector (5.5, under a pocket in the lid).

## Checks run

| Check | Tool | Result |
|---|---|---|
| Schematic says what `design.py` means | `board/tools/check_netlist.py` on KiCad's netlist | 125 parts, 90 nets, 29 no-connects: all match |
| ERC | kicad-cli | 0 errors; 1 warning: the MAX98357A symbol calls its exposed pad "unspecified" (it is ground) |
| DRC with schematic parity | kicad-cli, `board/tools/check_pcb.py` | see the latest run in `board/fab/drc.txt` |
| Contact pattern | `hardware/membrane/tools/check_contact_fp.py` | 0.200 mm minimum gap, 1.60 mm bridging spot |
| Board inside the case | board STL from kicad-cli against the case in OpenSCAD (`case/check_fit.py`) | see its report |

## Not checked

- No part of this has been built. Every number comes from datasheets.
- No signal-integrity or thermal simulation; no EMC or ESD testing; no USB
  compliance testing. A product needs FCC/CE (unintentional radiator) and UN38.3
  paperwork for the cell before it ships.
- Firmware for this board doesn't exist yet: see [pinmap.md](pinmap.md).

## Measure before ordering

| What | Why | Where it goes |
|---|---|---|
| DS Lite silicone piece outline, dome height, cap flange and height | case clearances are estimates (`ESTIMATE` in `case/case.scad`) | `piece_d`, `dome_h`, `flange_t`, `cap_*` |
| A real DT035 panel | FPC length and position, thickness, active area offset | `panel_*`, `fpc_*`, `aa_*` |
| The chosen cell | size and connector polarity (JST PH has no standard polarity) | `cell`, J2 |
| The chosen speaker | diameter, thickness, lead position | `spk_*`, J4 |

## Open risks

| Risk | Impact | Mitigation |
|---|---|---|
| BQ24074 stock at JLC (217 in July 2026) | assembly blocked for a larger run | same package, same pinout: BQ24073 (4.4 V OUT, no ITERM) or BQ24072 (VBAT+225 mV); or consign parts |
| Panel SPI mode | the module fixes its interface internally; if it came configured RGB-only, SPI would not show pixels | read the ID first at bring-up; Displaytech confirms the mode per order |
| DS Lite silicone fit | cap retention and feel depend on unmeasured heights | print the lid first and try real parts before the board order |
| JLC rotation offsets | parts placed 90 or 180 degrees off | review JLC's placement preview; the CPL carries KiCad's angles |
| Alps switch land pattern | KiCad's pattern puts the locating holes 0.075 mm from copper; the pads here are pulled back to 0.2 mm | check solder fillets on the first boards |
| Thin lid over the battery connector (0.6 mm) | cracks in an FDM print | resin print, or a lower-profile cell connector (JST SH/GH) in production |
