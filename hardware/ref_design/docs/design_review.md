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
- Its layout isn't left to the autorouter: `board/tools/route.py` draws the power
  loop on the top layer the way TI's layout guidance asks (input capacitor on VIN,
  both switch pins straight onto the inductor, output capacitors on VOUT, the GND
  and PS pins into the PowerPAD's vias, FB on VOUT, VINA's RC on its pin).
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
- The copy keeps what that layout depends on: the core's 0402 parts use Raspberry
  Pi's own land patterns (the switcher's LX copper runs between the pads of C24
  and C25, which KiCad's larger pads would cut), and inside the chip's pin ring the
  top layer is a 3V3 pour joined by short stubs from the 3V3 pins, as on their
  board. Theirs has two layers and reaches every pin from outside the ring; this
  one also uses the band between the pins and the 1.1 V ring for escapes (twelve
  signals drop to the bottom layer through vias there), so its 3V3 pour is in
  pieces. Every 3V3 pin still reaches the 3V3 plane, through its capacitor's via
  or a via of its own (DRC: nothing unconnected). The 1.1 V ring around the
  centre pad is untouched.
- Flash: W25Q128JVP in WSON (0.8 mm) instead of the reference's SOIC (1.75 mm),
  because it sits under the panel. Its exposed pad sits where Raspberry Pi's SD2
  track reached its pin, so that one line is routed anew (`board/tools/maze.py`);
  the other five QSPI lines are theirs.
- 12 MHz ABM8 crystal with 15 pF loads and 1k series, as in the reference.
- Debug: SWD and UART on JST SH 3-pin in the Raspberry Pi Debug Probe pinout;
  BOOTSEL and RESET buttons under pinholes; a red LED for firmware errors.
- GPIO map: [pinmap.md](pinmap.md). 39 of 48 GPIOs used.

### Display (sheet Display)

- QD3525 (Shenzhen Quandong Electronic's 3.5in IPS, BOE glass, ST7796U): 4-line
  SPI, 3.3 V, 55.50 x 84.96 x 2.50 mm, six white LEDs in parallel (15 mA each
  typical, 20 mA maximum). About $5-8 as a bare panel; the maker's drawing and
  pinout (QD3525 Specifications rev A) are published with LCDwiki's MSP3525
  module, which uses it. The Displaytech DT035 it replaced was $25-35 for the
  same class of panel.
- Its 48-contact tail (0.8 mm pitch) is soldered to J3's pads: this panel comes
  with a solder tail only. The tail leaves the panel's seat-1 edge and folds
  under it, contacts facing the board, contact 1 at the left end seen from above
  with seat 1 nearest you (the panel maker's folded view); J3 sits where the
  folded tail reaches (case.scad `fpc_front`).
- Pins wired as the panel maker's own module wires them: the parallel bus
  (DB0-17) and the RGB strobes low, RD and IM2-0 high (4-line 8-bit SPI); the
  resistive-touch pins open (no touch layer). TE goes to GPIO32 so the firmware
  can start frames on the panel's refresh and avoid tearing.
- SPI clock and data get 33 ohm source terminations next to the MCU.
- Backlight: AP3156 charge pump (1x/1.5x/2x) from VSYS drives the common anode;
  each LED's cathode has its own current sink, so the six share current evenly
  and the brightness holds from a full cell to a flat one. Current is set over
  one GPIO (1-wire pulses, 32 levels, 20 mA per LED at the top); it starts at
  0 mA, and R31 holds the line low (off) until the firmware drives it.

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
- Heights: the panel (2.5 mm) hangs from the lid's window ledge on double-sided
  tape, in a 4.9 mm bay that leaves it 2.4 mm clear of the board, so every part
  under it is 2.2 mm or less. Taller parts live in the
  side bezels or between a seat's clusters: USB-C (3.3 mm), power switch (3.3),
  debug connectors (2.9), inductors (1.2), battery connector (5.5, under a pocket
  in the lid). The case is built around the bay, not the panel: a panel that
  fits 56.0 x 86.24 x 4.9 mm changes nothing else.

## Checks run

| Check | Tool | Result |
|---|---|---|
| Schematic says what `design.py` means | `board/tools/check_netlist.py` on KiCad's netlist | 123 parts, 98 nets, 25 no-connects: all match |
| ERC | kicad-cli | 0 errors; 1 warning: the MAX98357A symbol calls its exposed pad "unspecified" (it is ground) |
| DRC with schematic parity | kicad-cli, `board/tools/check_pcb.py` | 0 errors, 0 unconnected, 0 parity issues; 2 cosmetic warnings (USB-C outline silk at the board edge, where the receptacle overhangs by design). `board/fab/drc.txt` |
| Contact pattern | `hardware/membrane/tools/check_contact_fp.py` | 0.200 mm minimum gap, 1.60 mm bridging spot |
| Board inside the case | every part's 3D model against the case's height limits, bosses and keep-outs (`case/check_fit.py`) | 91 parts, no collisions (`case/fit_report.txt`) |

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
| A real QD3525 panel | outline and thickness, active area offset, and where the folded tail's contacts land (it sets J3's position) | `panel_*`, `fpc_front`, `aa_*` |
| The chosen cell | size and connector polarity (JST PH has no standard polarity) | `cell`, J2 |
| The chosen speaker | diameter, thickness, lead position | `spk_*`, J4 |

## Open risks

| Risk | Impact | Mitigation |
|---|---|---|
| BQ24074 stock at JLC (217 in July 2026) | assembly blocked for a larger run | same package, same pinout: BQ24073 (4.4 V OUT, no ITERM) or BQ24072 (VBAT+225 mV); or consign parts |
| Panel supply | the QD3525 comes from one maker through marketplace sellers; a lookalike may differ in pinout or tail length | buy by the part number printed on the tail, check it against the drawing before soldering; the same 48-pin ST7796U panel from another maker needs only case.scad's panel numbers and J3's pin map checked |
| Soldered panel tail | a panel is harder to swap than on a connector | tin-and-reflow joint (JLC's paste leaves the pads tinned); a hot-air or iron rework frees it |
| DS Lite silicone fit | cap retention and feel depend on unmeasured heights | print the lid first and try real parts before the board order |
| JLC rotation offsets | parts placed 90 or 180 degrees off | review JLC's placement preview; the CPL carries KiCad's angles |
| Alps switch land pattern | KiCad's pattern puts the locating holes 0.075 mm from copper; the pads here are pulled back to 0.2 mm | check solder fillets on the first boards |
| Thin lid over the battery connector (0.6 mm) | cracks in an FDM print | resin print, or a lower-profile cell connector (JST SH/GH) in production |
