# Design review: REF-A, 2-layer build

What the reference design is, why each part of it is the way it is, what has
been checked and what hasn't. Numbers here come from the datasheets in
`datasheets/SOURCES.md` and from the files in this folder.

This is the 2-layer build (branch `ref-design-2layer`): the 4-layer design on `main`
with the cost changes Alex approved on 2026-09-24.

## The device

Two players face each other across a 3.5in 320x480 IPS screen; each has a
D-pad and four face buttons under DS Lite silicone at their end. One Li-ion
cell, one speaker, one USB-C for charging and firmware. No radios.

Case 75.4 x 156.2 x 15.0 mm. Board 70.4 x 151.2 mm, 2 layers, 1.6 mm, ENIG,
every hole 0.3 mm or larger.

## Block by block

### USB-C and charging (sheet USB_Charger)

- Sink only: 5.1k Rd on CC1 and CC2, no PD. A Type-C charger turns VBUS on
  because of them. VBUS bulk capacitance 10 uF (USB's limit).
- SRV05-4 on D+/D-, next to the connector: a 4-line TVS array referenced to VBUS
  (two of its lines unused). A JLC preferred part, so no setup fee.
- TP4056 linear charger: 4.2 V, 1200 V / R5 = 500 mA (0.5 C for the 1000 mAh
  503450). TEMP is grounded, which turns its temperature check off: no thermistor,
  as approved. CHRG and STDBY go to GPIOs; there are no LEDs.
- Worst-case charger dissipation (5 V in, a 3.0 V cell, 500 mA): 1.0 W; about
  0.65 W at a 3.7 V cell. The ESOP-8's thermal pad has six 0.3 mm vias into the
  ground pours on both layers. The TP4056 lowers its own current when its die
  passes about 120 C, so heat slows the charge rather than hurting the chip.
- Load sharing: USB feeds the system (VSYS) through D5 (B5819W, 1 A Schottky)
  while the TP4056 charges the cell; without USB, Q2 (AO3401A, source on VSYS,
  drain on the cell) connects the cell. With USB, VSYS is about 4.6 V, so Q2's
  body diode is reverse biased and nothing flows back into the cell.
- The unplug: Q2's gate sees VBUS through D6 (BAT54WS), so it isn't held up by
  C1's 10 uF. R6 (10k) pulls it down within microseconds of an unplug, instead of
  the 100 ms C1 would take, and C9 (22 uF) carries VSYS across the change. In the
  gap, Q2's body diode holds VSYS at the cell voltage less 0.6 V. A dip here resets
  the MCU, and a reset turns the device off, so bring-up checks it with a scope.

### 3V3 and the power switch (sheet Power)

- ME6211 LDO from VSYS: 500 mA maximum, 120 mV dropout at 100 mA. It holds 3.3 V
  down to a cell of about 3.42 V; below that 3V3 follows the cell. The RP2354B
  needs VREG_AVDD at 3.135 V or more, so the firmware shuts down at about 3.35 V,
  which leaves roughly 5-10% of the cell unused against the buck-boost it replaced.
- Load under 100 mA (RP2354B up to ~50 mA, panel logic ~10 mA): at most 0.13 W in
  the LDO on USB power.
- PWR_EN = (button) OR (PWR_HOLD from the MCU) OR (VBUS through 10k and a diode).
  All three through Schottky diodes so none back-feeds another; 100k pull-down.
- The button is seat 1's Y contact (SW8). Its common comb is VSYS instead of
  GND, so pressing Y joins VSYS to PWR_BTN_RAW, as the side switch did. The MCU
  sees it through Q1 (2N7002, gate on the button node) on GPIO3, so no MCU pin
  ever touches VSYS; firmware reads seat 1's Y there.
- Off state: ME6211 off (CE low), amplifier shut down (R36 holds SHUTDOWN high,
  4.2 uA), backlight driver shut down by R31, charger drawing about 2 uA from the
  cell, battery divider 4.5 uA. About 12 uA total: years of shelf life.
- A reset or crash drops PWR_HOLD. On the cell alone that turns the device off,
  which is the safe failure. On USB it stays on (VBUS holds PWR_EN).

### RP2354B core (sheets MCU, MCU_Power, Debug)

- Schematic, values and placement copied from Raspberry Pi's RP2350B minimal
  design (MIT), turned 90 degrees. Its routing, vias and the small copper areas
  around the internal 1.1 V switcher are copied too (`board/tools/build_pcb.py`),
  so the most layout-sensitive part of the board is Raspberry Pi's, not mine. That
  design has two layers, like this board.
- The chip is the RP2354B: the same die and pinout with 2 MB of flash stacked inside.
  Only QSPI_SS is used outside, as the BOOTSEL strap (1k to the button) with the
  reference's 10k pull-up; the reference's flash and its routing stay behind.
- The copy keeps what that layout depends on: the core's 0402 parts use Raspberry
  Pi's own land patterns (the switcher's LX copper runs between the pads of C24
  and C25, which KiCad's larger pads would cut), and over the chip the top layer
  is a 3V3 pour joined by short stubs from the 3V3 pins, as on their board. The
  LDO and its output capacitors sit inside that pour, so the pour itself carries
  3V3 to the chip. The copied vias keep Raspberry Pi's 0.6 mm pads with 0.3 mm holes
  instead of 0.25 mm.
- USB series resistors: 22 ohm 0402 (a basic part) in Raspberry Pi's own spots. The approved
  27 ohm 0603 doesn't fit the copied layout, and 22 ohm works as well at full speed.
- 12 MHz ABM8 crystal with 15 pF loads and 1k series, as in the reference.
- Debug: SWD and UART on JST SH 3-pin in the Raspberry Pi Debug Probe pinout;
  BOOTSEL and RESET buttons under pinholes. No LEDs.
- GPIO map: [pinmap.md](pinmap.md).

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

- PWM from GPIO8 (PWM4 A) through two RC poles (1k and 10 nF, 16 kHz each): a
  146 kHz carrier (10 bits at 150 MHz) comes out about 38 dB down. C39 (220 nF)
  blocks the DC and, with R34, sets a 36 Hz low corner.
- FM8002A bridge-tied amplifier from VSYS: gain 2 x R35 / R34 = 2, the datasheet's
  circuit (+IN on the bypass, 1 uF bypass capacitor). About 1 W into 8 ohm at 5 V
  (datasheet: 1.1 W at 1% THD); less from the cell.
- SHUTDOWN is high (off) through R36 to VSYS; Q3 pulls it low when AMP_EN is high,
  and R37 keeps the amplifier off while the MCU boots. On: 4.4 mA quiescent.
- Class AB: it turns about half its battery draw into sound at high volume, against
  80-90% for the class-D NS4168 that was offered instead (50 cents a unit more).

### Membrane contacts (sheet Buttons)

- 16 x DSL_button_4ring: two interleaved combs (0.15 mm lines, 0.20 mm gaps,
  checked by `hardware/membrane/tools/check_contact_fp.py`) under one 4.8 mm mask
  opening, ENIG. A carbon pill of 1.6 mm or more bridges them anywhere on the pad.
- Each comb leaves through a 0.2 mm stub to a via 3.3 mm from the contact's
  centre; nothing else (no vias, no pour) inside 2.9 mm.
- SW8 (seat 1's Y) is the power button: its combs are VSYS and PWR_BTN_RAW.

## Board

- Stack-up: two layers. F.Cu: parts, most routing, and the 3V3 pour over the chip
  (as on Raspberry Pi's board). B.Cu: the ground pour, crossed by the routing that
  wouldn't fit on top. GND pours on both layers, stitched by vias on a 5 mm grid.
  There is no 3V3 plane: 3V3 is routed as a 0.3 mm track like the other supplies.
- Rules: 0.15 mm track and clearance (0.1 mm minimum inside the copied core),
  0.3 mm power tracks (3V3 included), 0.2 mm USB; vias 0.5 mm with 0.3 mm holes
  (0.6 mm on power). Every hole is 0.3 mm or larger, JLC's standard: smaller ones
  cost about $40 an order (a fee, a required 4-wire test and TG155 material).
- Heights: the panel (2.5 mm) hangs from the lid's window ledge on double-sided
  tape, in a 4.9 mm bay that leaves it 2.4 mm clear of the board, so every part
  under it is 2.2 mm or less; the ESOP-8 and SOIC-8 chips (1.75 mm) sit under it.
  Taller parts live in the side bezels or between a seat's clusters: USB-C
  (3.3 mm), debug connectors (2.9), battery connector (5.5, under a pocket in the
  lid). The case is built around the bay, not the panel: a panel that fits
  56.0 x 86.24 x 4.9 mm changes nothing else.
- The lid's snap hooks hang 4.3 mm below it, just inside the tub wall, so nothing
  taller than 0.6 mm may sit under them (`case/check_fit.py` checks it).

## Checks run

| Check | Tool | Result |
|---|---|---|
| Schematic says what `design.py` means | `board/tools/check_netlist.py` on KiCad's netlist | 122 parts, 115 nets, 83 named and 32 no-connects: all match |
| ERC | kicad-cli | 0 errors, 0 warnings |
| DRC with schematic parity | kicad-cli, `board/tools/check_pcb.py` | 0 errors, 0 unconnected, 0 parity issues; 2 cosmetic warnings (USB-C outline silk at the board edge, where the receptacle overhangs by design). `board/fab/drc.txt` |
| Every hole 0.3 mm or larger | the drill file, `board/fab/gerbers/battler.drl` | 0.3, 0.6, 0.65 and 2.2 mm only |
| Contact pattern | `hardware/membrane/tools/check_contact_fp.py` | 0.200 mm minimum gap, 1.60 mm bridging spot |
| Board inside the case | every part's 3D model against the case's height limits, bosses, snap hooks and keep-outs (`case/check_fit.py`) | 90 parts, no collisions (`case/fit_report.txt`) |

Routing took `board/tools/route.py` and then `route.py --finish` for the last
connection (a contact stub the first pass had cleaned away). Seat 1's right and
the debug UART moved to free pins at the chip's top-left corner (GPIO23, and
UART1 on GPIO20/21) to get out of the crystal's crowded corner.

## Not checked

- No part of this has been built. Every number comes from datasheets.
- No signal-integrity or thermal simulation; no EMC or ESD testing; no USB
  compliance testing. A product needs FCC/CE (unintentional radiator) and UN38.3
  paperwork for the cell before it ships.
- The unplug hand-over (USB to cell) is calculated, not measured: bring-up step 5.
- The snap fit is modelled, not printed: tune `snap_barb` and `snap_t` on a test print.
- Firmware for this board doesn't exist yet: see [pinmap.md](pinmap.md).

## Measure before ordering

| What | Why | Where it goes |
|---|---|---|
| DS Lite silicone piece outline, dome height, pill spacing, cap flange and height | case clearances are estimates (`ESTIMATE` in `case/case.scad`); the D-pad pill spacing (14.0 mm here, 13.5 in another project) | `piece_d`, `dome_h`, `flange_t`, `cap_*`, `dpad_pitch` |
| A real QD3525 panel | outline and thickness, active area offset, and where the folded tail's contacts land (it sets J3's position) | `panel_*`, `fpc_front`, `aa_*` |
| The chosen cell (503450) | length with its protection board, and connector polarity (JST PH has no standard polarity) | `cell`, J2 |
| The chosen speaker | diameter, thickness, lead position | `spk_*`, J4 |
| A printed tub and lid | the snap fit | `snap_*` |

## Open risks

| Risk | Impact | Mitigation |
|---|---|---|
| RP2354B and AP3156 stock at JLC (1454 and 1490 when checked) | assembly blocked for a larger run | buy ahead, or consign parts |
| No charge temperature cutoff (approved) | the cell charges however hot it is; the TP4056 only protects itself | keep the device out of hot places while charging; a thermistor on TEMP can come back (two resistors) |
| USB-to-cell hand-over | a reset on unplug turns the device off | bring-up step 5 with a scope; a second 22 uF on VSYS if needed |
| Panel supply | the QD3525 comes from one maker through marketplace sellers; a lookalike may differ in pinout or tail length | buy by the part number printed on the tail, check it against the drawing before soldering; the same 48-pin ST7796U panel from another maker needs only case.scad's panel numbers and J3's pin map checked |
| Soldered panel tail | a panel is harder to swap than on a connector | tin-and-reflow joint (JLC's paste leaves the pads tinned); a hot-air or iron rework frees it |
| DS Lite silicone fit | cap retention and feel depend on unmeasured heights | pads and caps ordered: try them in a printed lid before the board order |
| Snap fit | too tight cracks a hook, too loose rattles; FDM snap fits wear | tune on a test print; PETG over PLA |
| JLC rotation offsets | parts placed 90 or 180 degrees off | review JLC's placement preview; the CPL carries KiCad's angles |
| Thin lid over the battery connector (0.6 mm) | cracks in the FDM-printed lid | 100% infill there for now; a lower-profile cell connector (JST SH/GH) removes the pocket |
