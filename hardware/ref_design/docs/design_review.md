# Design review: REF-A, 2-layer build

What the reference design is, why each part of it is the way it is, what has
been checked and what hasn't. Numbers here come from the datasheets in
`datasheets/SOURCES.md` and from the files in this folder.

This is the 2-layer build, `main` since 2026-09-24: the first, 4-layer design (tag
`archive/ref-design-4layer`) with the cost changes Alex approved that day.

## The device

Two players face each other across a 3.5in 320x480 IPS screen; each has a
D-pad and four face buttons under DS Lite silicone at their end. One Li-ion
cell, one speaker, one USB-C for charging and firmware. No radios.

Case 75.4 x 156.2 mm, 13.4 mm thick at the screen and 11.06 at the ends (14.6 until 2026-10-19, when the panel's top went into a pocket in the lid under a 0.8 mm skin, with a 0.5 mm pad over J2): the cell lies on the tub's floor under the board and the board rests on ribs 4.5 mm over the floor (since 2026-10-09; branch ref-design-cell-on-board has the cell on the board under the screen, 13.9 mm and 7.86 at the ends). Each end steps down 2.34 mm to a button deck (a spine between the clusters stays full height over the end snap hooks), so genuine DS Lite caps stand 1.3 mm proud, as on a DS Lite; `case/DSLITE.md` has the cap numbers and their sources. Board 70.4 x 151.2 mm (70.5 x 151.34 over the outline's
line width, the size JLC quotes), 2 layers, 1.6 mm, ENIG,
every hole 0.3 mm or larger.

## Block by block

### USB-C and charging (sheet USB_Charger)

- Sink only: 5.1k Rd on CC1 and CC2, no PD. A Type-C charger turns VBUS on
  because of them. VBUS bulk capacitance 10 uF (USB's limit).
- SRV05-4 on D+/D-, next to the connector: a 4-line TVS array referenced to 3V3
  (two of its lines unused). Not VBUS: with no cable in, the MCU's D+ pull-up (on in
  BOOTSEL mode and whenever USB is running) would feed VBUS through the D+ steering
  diode to about 2.7 V, which holds PWR_EN up and half-closes Q2. A JLC preferred part.
- TP4056 linear charger: 4.2 V; R5 = 3.3k sets about 333-364 mA (the datasheet's
  revisions give 1100 or 1200 V / R5), about 0.5 C for the 403450 (about 700 mAh), the
  usual standard charge rate for small pouch cells. CE is pulled up to VBUS (R43), so it
  charges by default; firmware can pause it with CHG_OFF (GPIO19, Q4; R44 holds it off). CHRG and STDBY go to GPIOs; there are no LEDs.
- Charge temperature: the TP4056 charges only while TEMP is between 45% and 80% of
  VBUS. The 403450 has no NTC wire, so TH1 (NCP15XH103F03RC, 10k B3380) sits on the
  board over the cell, which lies 0.5 mm under the board's back, in parallel with R40 (150k),
  under R39 (5.6k) from VBUS. Window: 1 to 46 C nominal, -3 to 49 C across the datasheet's
  threshold spread (43-45%, 80-82%). TH1 is about 45 mm from U2, whose 0.65 W while
  charging would otherwise trip it, and it reads the cell's own underside. Out of range, CHRG and
  STDBY are both high-impedance, as with no USB; firmware tells them apart by the CC
  lines. Divider current comes from VBUS only (about 0.6 mA), none from the cell.
- Worst-case charger dissipation (5 V in, a 3.0 V cell, 500 mA): 1.0 W; about
  0.65 W at a 3.7 V cell. The ESOP-8's thermal pad has six 0.3 mm vias into the
  ground pours on both layers. The TP4056 lowers its own current when its die
  passes about 145 C, so heat slows the charge rather than hurting the chip.
- Load sharing: USB feeds the system (VSYS) through D5 (B5819W, 1 A Schottky)
  while the TP4056 charges the cell; without USB, Q2 (AO3401A, source on VSYS,
  drain on the cell) connects the cell. With USB, VSYS is about 4.6 V, so Q2's
  body diode and D7 (below) are reverse biased; only D7's leakage flows back into the
  cell, about 5 uA at 25 C and 50 uA at 75 C. A full cell left on USB therefore floats
  near 4.2 V instead of cycling through the 4.05 V recharge: a small calendar-ageing
  cost, backstopped by the pack's overvoltage cut-off.
- The input: C1 sits behind R41 (2.2 ohm), the TP4056 datasheet's fix for a ceramic
  input capacitor ringing when a live source is plugged in (it asks 1.5 ohm; 2.2 is the
  nearest Basic part). Simulated, a 2 m cable into a board with no cell peaks VBUS at
  6.7 V (7.1 without R41; U2's maximum is 8 V) and VSYS at 6.2 V (6.7 without; the
  ME6211's maximum is 6.5 V). Most of the ring goes through D5 into the VSYS capacitors,
  which damps it; with a cell in, the peaks are lower. No TVS: an SMF5.0A's 5.0 V working
  voltage is below USB's 5.5 V, and it would barely conduct at these peaks.
- The unplug: Q2's gate is fed from VBUS through D6 (BAT54WS) and pulled down by R6
  (10k), but it can only fall as fast as VBUS does, and C1 (10 uF) holds VBUS up
  after the charger stops drawing. R38 (1k, an 0603) bleeds VBUS, so Q2's channel
  takes the load 1.5-3.3 ms after an unplug, depending on its threshold (without R38,
  tens of ms). Until then something else has to carry VSYS. Q2's body diode alone
  drops about 0.6 V at a 0.25 A load (full backlight): 3V3 would then be about the cell
  less 0.71 V, under the internal flash's 2.97 V QSPI_IOVDD minimum on every unplug
  with the cell below about 3.7 V, most of its range. So D7 (B5819W, the same part as
  D5) sits across Q2 and carries the gap at about 0.34 V: 3V3 stays above 2.97 V down
  to a cell of about 3.42 V (3.49 V with 0.25 A of audio as well), inside the 3.5 V
  cutoff. The RP2350 has no 3V3 supervisor (its brown-out detector watches the 1.1 V
  core), so an out-of-spec dip would not reset it cleanly: it could corrupt a flash
  access instead. Numbers from a simulation of the unplug (the AO3401A and B5819W
  curves, the TP4056's shut-off, the ME6211's dropout and the routed copper), checked
  by two independent model reviews; bring-up step 5 measures it.

### 3V3 and the power switch (sheet Power)

- ME6211 LDO from VSYS: 500 mA maximum, 120 mV dropout at 100 mA. It holds 3.3 V
  down to a cell of about 3.42 V; below that 3V3 follows the cell. The firmware shuts
  down at 3.50 V as it computes it, never lower: the ADC's reference is the 3V3 rail,
  so once the LDO drops out the computed voltage levels off at 3.37-3.43 V whatever
  the cell is, and a lower threshold would never fire (`docs/pinmap.md` has the
  reading and calibration rules). That leaves roughly 5-10% of the cell unused against
  the buck-boost it replaced.
- Load under 100 mA (RP2354B up to ~50 mA, panel logic ~10 mA): at most 0.13 W in
  the LDO on USB power.
- PWR_EN = (button) OR (PWR_HOLD from the MCU) OR (VBUS through 10k and a diode).
  All three through Schottky diodes so none back-feeds another; 100k pull-down (R15).
  With the cell on VSYS, D5's reverse leakage flows into the VBUS node; into R15's 100k
  about 10 uA (a warm Schottky) would reach the ME6211's 1.0 V CE level. R38 (1k, the
  unplug bleeder, see USB) holds VBUS down: that now takes about 1.3 mA. It costs 5 mA
  with USB in.
  (A 10k R15 would do the same but cut how much resistance the Start contact's pill may have
  before Q1 misses a press by about five times.)
- The button is seat 1's Start contact (SW21; seat 1's Y until 2026-10-19, which put it
  on Start so no game button doubles as power). Its common comb is VSYS instead of
  GND, so pressing Start joins VSYS to PWR_BTN_RAW, as the side switch did. The MCU
  sees it through Q1 (2N7002, gate on the button node) on GPIO3, so no MCU pin
  ever touches VSYS; firmware reads seat 1's Start there.
- Off state: ME6211 off (CE low), amplifier shut down (R36 holds SHUTDOWN high,
  4.2 uA), backlight sinks off (they run from 3V3; R56/R57 hold the FET gates low), charger drawing about 2 uA from the
  cell, battery divider 4.5 uA (up to 9 uA: with the chip unpowered, GPIO47 sits above
  IOVDD and its ESD diode takes a few uA, harmless and the same as on Raspberry Pi's
  Pico), and D5's reverse leakage from VSYS into R38 (1-6 uA at 25 C). About 18-22 uA
  total: years of shelf life. The divider stays at 470k/470k for this reason: 100k/100k
  would add about 30 uA, most of it into the unpowered chip's clamp, and still not
  guarantee the reading against the pad's worst-case leakage.
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
  is a 3V3 pour joined by short stubs from the 3V3 pins, as on their board. 3V3
  reaches that pour from the LDO (U3, right of the chip) through Raspberry Pi's
  copied stubs, 0.15-0.2 mm wide: ample for the chip's 100 mA or less. The copied vias keep Raspberry Pi's 0.6 mm pads with 0.3 mm holes
  instead of 0.25 mm.
- USB series resistors: 22 ohm 0402 (a basic part) in Raspberry Pi's own spots. The approved
  27 ohm 0603 doesn't fit the copied layout, and 22 ohm works as well at full speed.
- 12 MHz ABM8 crystal with 15 pF loads and 1k series, as in the reference.
- Debug: SWD and UART on JST SH 3-pin in the Raspberry Pi Debug Probe pinout;
  BOOTSEL and RESET buttons under pinholes. No LEDs.
- GPIO map: [pinmap.md](pinmap.md).

### Display (sheet Display)

- EastRising ER-TFT035IPS-6: 3.5in IPS, 320x480, ILI9488, 4-wire SPI, 3.3 V,
  54.50 x 83.00 x 2.30 mm with its FPC folded, the same 48.96 x 73.44 mm active area
  as the QD3525 it replaced. Sold by the maker, on its own site and its AliExpress
  store: about $14.20 a unit with shipping at 10 units, $9.86 at 50 ([bom.md](bom.md)).
  It has a published datasheet, outline drawing and 3D model
  ([datasheet](https://www.buydisplay.com/download/manual/ER-TFT035IPS-6_Datasheet.pdf)).
- Its 50-contact FPC (0.5 mm pitch, 0.3 mm thick with its stiffener) leaves the
  panel's seat-1 edge, folds under the panel and plugs into J3, a Hirose FH12-50S
  bottom-contact ZIF connector that JLC places: no hand soldering. J3 sits where the
  folded FPC's tip reaches less 1 mm of slack (case.scad `conn_in`), its mouth towards
  seat 1. Seen from above the FPC's contact 1 lands at the right, J3's pin 1 is at the
  left, so J3 pin j carries panel pin 51 - j (`board/design.py`).
- Pins wired as the datasheet says: the parallel bus (DB0-17) and the RGB strobes low,
  RD and IM2-0 high (4-wire 8-bit SPI); the touch pins open (no touch layer). TE goes
  to GPIO32 so the firmware can start frames on the panel's refresh and avoid tearing.
- ILI9488 over SPI takes 18-bit colour only: three bytes a pixel, 461 kB a full frame,
  half as much again as the ST7796U's 16-bit, so full-screen redraws are slower. The
  battle screen redraws in parts. The datasheet rates serial writes to 20 MHz.
- SPI clock and data get 33 ohm source terminations next to the MCU.
- Backlight: six LEDs in parallel (3.0-3.2 V, 120 mA at most), common anode, three
  on each of two cathode pins. The anode is on VSYS (through JP2); each cathode pin has
  its own linear current sink: half of U6 (MCP6002) drives an AO3400A (Q5, Q6) and
  holds its 2.2 ohm sense resistor (R52, R53) at BL_REF. BL_REF is GPIO44's PWM through
  R60/R61 and C50, 0.124 V at 100%: 56 mA a pin, 113 mA in all. The sink needs about
  0.13 V, so the current holds from a full cell (or USB) down to the 3.5 V cutoff with
  the panel's 3.2 V maximum Vf; with an LED at the older datasheet's 3.4 V it falls 6% at
  100% on a 3.5 V cell. Efficiency is that of the AP3156 in its 1x mode (Vf / VSYS), so
  battery life is unchanged. The LED current is DC (the PWM is filtered), so there is no
  PWM in the LEDs for the amplifier to pick up. It replaced the AP3156 charge pump, out
  of stock at JLC and LCSC on 2026-10-07: every part is Basic or Preferred.
- Backlight loop: R54/R55 (330 ohm) isolate the gate capacitance from the op-amp and
  C48/C49 (1 nF) with R58/R59 (10k) close a local loop. Simulated in ngspice with
  Microchip's MCP6001/2 macromodel and Alpha & Omega's AO3400 model before layout
  (board/sim/backlight.py, results in board/sim/backlight_results.txt): phase margin
  72.5 deg or more over 720 corners (duty, VSYS 3.5-5.7 V, Vf +0/+0.2/+0.4 V, 20-50 nH
  of FPC, gate capacitance doubled), no overshoot on large steps (2.6% on a 5%->6%
  step), 0.3% ripple at 36.6 kHz PWM, no current spike at power-up, power-down or a
  VSYS hot plug with 3V3 off (0.5-4.7 mA for about a us). A VSYS step of 1.9 V in 2 us
  while lit (USB plugged in) briefly overshoots to 133 mA a pin for about 2 us. The
  op-amp's offset (4.5 mV max) sets the accuracy: +-3.6% at full, +-20% a pin at 18%
  duty, so firmware keeps the dimmest level at about 18%.

### Audio (sheet Audio)

- PWM from GPIO8 (PWM4 A, 8 bits at 150 MHz: a 586 kHz carrier) through two RC
  stages (1k and 4.7 nF each). Loaded, the audio band is -3 dB at about 14 kHz, about
  what a 15 mm speaker reproduces, and the carrier comes out about 49 dB down. 10-bit
  PWM (146 kHz) would only come out 26 dB down, so firmware runs 8 bits and dithers or
  noise-shapes if it wants more resolution. (4.7 nF is a JLC basic part; 3.9 nF, which
  would give 17 kHz, is only an extended one.) C39 (220 nF) blocks the DC and, with
  R34, sets a 36 Hz low corner.
- FM8002A bridge-tied amplifier from VSYS: gain 2 x R35 / R34 = 2, the datasheet's
  circuit (+IN on the bypass, 1 uF bypass capacitor). The filter's 2k in front of R34
  makes the chain's gain about 1.8, so full scale is about 0.55 W into 8 ohm on USB
  and 0.4-0.5 W on the cell (the amplifier is rated 1.1 W); below a 3.7 V cell full
  scale clips. Worst-case dissipation (a continuous full-scale sine on USB) is about
  0.54 W in the SOIC-8: bring-up step 4 checks its temperature.
- SHUTDOWN is high (off) through R36 to VSYS; Q3 pulls it low when AMP_EN is high,
  and R37 keeps the amplifier off while the MCU boots. On: 4.4 mA quiescent. It takes
  about 100 ms to wake (the bypass capacitor charging), so firmware keeps it on while
  the device is awake or raises AMP_EN well before a sound.
- Class AB: it turns about half its battery draw into sound at high volume, against
  80-90% for the class-D NS4168 that was offered instead (50 cents a unit more).

### Membrane contacts (sheet Buttons)

- 16 x DSL_button_4ring (D-pads, A/B/X/Y) and 4 x DSL_button_3ring (Start, Select: the same comb
  without its outer ring, for the Start/Select membrane's smaller pills; ESTIMATE): two interleaved combs (0.15 mm lines, 0.20 mm gaps,
  checked by `hardware/membrane/tools/check_contact_fp.py`) under one 4.8 mm mask
  opening, ENIG. A carbon pill of 1.6 mm or more bridges them anywhere on the pad.
- Each comb leaves through a 0.2 mm stub to a via 3.3 mm from the contact's
  centre; nothing else (no vias, no pour) inside 2.9 mm.
- SW21 (seat 1's Start) is the power button: its combs are VSYS and PWR_BTN_RAW.

## Board

- Stack-up: two layers. F.Cu: parts, most routing, and the 3V3 pour over the chip
  (as on Raspberry Pi's board). B.Cu: the ground pour, crossed by the routing that
  wouldn't fit on top. GND pours on both layers, stitched by vias on a 5 mm grid.
  There is no 3V3 plane: 3V3 is routed as a 0.3 mm track like the other supplies.
- Rules: 0.15 mm track and clearance (0.1 mm minimum inside the copied core),
  0.3 mm power tracks (3V3 included; each membrane contact's combs leave on 0.2 mm
  stubs, the power button's VSYS comb too), 0.2 mm USB; vias 0.5 mm with 0.3 mm holes
  (0.6 mm on power). Every hole is 0.3 mm or larger, JLC's standard: smaller ones
  cost about $40 an order (a fee, a required 4-wire test and TG155 material).
- Heights, from the floor: tub floor 1.5, the cell (403450, 4 mm) lengthwise on the floor
  with 0.5 mm to the board's back, board 1.6 on ribs 4.5 mm high everywhere else. Beside
  the cell on the floor, the speaker (GSPK151035PN, 15 x 10 x 3.5 mm) lies face down on a pad
  by the left wall over a front chamber in the floor, which opens out of the wall's rounded
  lower edge (it fired down through a grille a table covers until 2026-10-20); its plug comes
  up through a hole at seat 2's end to J4, J2's PicoBlade header under seat 2's spine. The panel's back is 2.4 mm over the board
  (parts under it to 2.2; J3 is 2.0); the panel (2.35 +-0.1 in its drawing, counted as
  2.45) hangs from the lid's window ledge on 0.15 mm tape. The battery connector J2
  (5.5 mm) sits under the spine between seat 1's clusters, in a pocket in the lid; the
  cell's wires come up through a 5 x 2.3 mm slot beside its plug.
- The lid's snap hooks hang 4.3 mm below it, just inside the tub wall, so nothing
  taller than 0.7 mm may sit under them (`case/check_fit.py` checks it).

## Checks run

| Check | Tool | Result |
|---|---|---|
| Datasheet pin review | `board/tools/check_pins.py` over `board/pin_review/`: every assembled part's every pin against its current datasheet (name, requirement, page/table citation, quote), every resistor and capacitor against its LCSC part and voltage rating; fails when a reviewed pin's net changes or an issue has no fix or waiver | PASS: 29 parts, 244 pins, 94 passives, 5 waived (2026-10-07, after the backlight sinks replaced the AP3156; first run 2026-10-05). Found and fixed: J6 TX/RX swapped, USB series resistors 22 to 27 ohm, the ME6211 CE above VIN at a live plug-in (C47) |
| Schematic says what `design.py` means | `board/tools/check_netlist.py` on KiCad's netlist | 157 parts, 124 nets, 94 named and 30 no-connects: all match |
| Backlight sink stability | ngspice, `board/sim/backlight.py` (Microchip and Alpha & Omega models) | phase margin 72.5 deg or more over 720 corners; flat 3.5-5.7 V; 0.3% ripple; no power-sequencing spikes (`board/sim/backlight_results.txt`) |
| ERC | kicad-cli | 0 errors, 0 warnings |
| DRC with schematic parity | kicad-cli, `board/tools/check_pcb.py` | 0 errors, 0 unconnected, 0 parity issues; 2 cosmetic warnings (USB-C outline silk at the board edge, where the receptacle overhangs by design). `board/fab/drc.txt` |
| Every hole 0.3 mm or larger | the drill file, `board/fab/gerbers/battler.drl` | 0.3, 0.6, 0.65 and 2.2 mm only |
| Contact pattern | `hardware/membrane/tools/check_contact_fp.py` | 0.200 mm minimum gap, 1.60 mm bridging spot |
| Board inside the case | every part's 3D model against the case's height limits, bosses, snap hooks and keep-outs (`case/check_fit.py`) | 123 parts, no collisions, including under the lid's panel frame (`case/fit_report.txt`) |

Routing: two things make the 2-layer board route completely, in about ten minutes.
The pins are assigned for two layers: each faces where its net goes, in the order the
nets fan out, so routes don't cross (seat 1's face buttons, the panel's reset, the
backlight and the ADC inputs on GPIO40-47 along the chip's bottom edge; seat 1's cross
and the panel's TE on GPIO28-32 down its left edge, after SWD and RUN). And Raspberry
Pi's own pin escapes are copied with the core (`build_pcb.copy_escapes`): every pin in
use leaves the chip on their track, locked, as far as 3.2 mm past the pads, so the
router starts from a ring of stub ends and no pin gets boxed in by its neighbours.

## Not checked

- No part of this has been built. Every number comes from datasheets.
- No signal-integrity or thermal simulation; no EMC or ESD testing; no USB
  compliance testing. A product needs FCC/CE (unintentional radiator) and UN38.3
  paperwork for the cell before it ships.
- The unplug hand-over (USB to cell) is calculated, not measured: bring-up step 5.
- The snap fit: the first print (PLA) found the hooks snapping, the lid too tight at the deck steps and
  the pins binding; this version drops the pins, adds 0.3 mm at the steps and tapers the hooks. Print
  the lid in PETG if the hooks still break.
- Firmware for this board doesn't exist yet: see [pinmap.md](pinmap.md).

## Measure before ordering

The fit kit (`case/fitkit/`, see its README) prints a board mock, part dummies and coupons of
the case for these checks.

| What | Why | Where it goes |
|---|---|---|
| DS Lite silicone piece outline and height, pill spacing; cap keying | the caps are measured from open models (`case/DSLITE.md`), the silicone is still estimated; `dome_h` sets the deck height, so a wrong one leaves the caps high or low; the D-pad pill spacing (14.0 mm here, 13.5 in another project) | `piece_d`, `dome_h`, `dpad_pitch`; the cap numbers in `case/DSLITE.md` |
| A real ER-TFT035IPS-6 and an FH12-50S | outline and thickness, active area offset; how far the FPC goes into J3 (Hirose's recommended FPC stops 3.5-4 mm in, so expect 2.5-3 mm of slack, not the 1 mm modelled); that its contact 1 meets J3's pin 50 | `panel_*`, `aa_*`, `fpc_*`, `conn_rear`; J3's pin map |
| The chosen cell (403450) | length with its protection board, and connector polarity (PicoBlade has no standard polarity) | `cell`, J2 |
| The chosen speaker (GSPK151035PN) | its tab's length (spk_tab is estimated), that its plug mates a genuine 53261-0271, that its foam ring seals on the pad, and how loud it is out of the side | `spk_*`, J4 |
| A printed tub and lid | the snap fit | `snap_*` |

## Open risks

| Risk | Impact | Mitigation |
|---|---|---|
| RP2354B stock at JLC (1665 on 2026-10-07; the AP3156 it shared this row with ran out and was replaced by the discrete backlight sink) | assembly blocked for a larger run | buy ahead, or consign parts |
| Board thermistor, not a cell one | TH1 reads the board over the cell; a charge in a hot car or near the 46 C edge may pause and resume | the pause is safe and charging resumes by itself; bring-up 5.4 and 5.4a check TH1's reading while charging and the pause points |
| USB-to-cell hand-over | for 1.5-3.3 ms after an unplug D7 carries VSYS until Q2 takes over; below a 3.42-3.49 V cell 3V3 still dips under the flash's 2.97 V minimum | the 3.5 V cutoff keeps the cell above that; bring-up step 5 scopes it with a nearly flat cell; a larger margin would need a lower-dropout 3V3 regulator |
| Hot plug with no cell | a live source into a bare board rings VSYS to about 6.2 V in simulation (R41 in place), against the ME6211's 6.5 V maximum | bring-up step 5 scopes it; the worst case is an A-to-C cable from an always-on USB-A port (a USB-C source switches VBUS on only after it sees the board, and ramps it), so on the bench prefer a USB-C source or fit the cell first |
| Panel supply | one maker (EastRising), sold direct | it publishes its drawings; another 3.5in panel that fits the 56.0 x 86.24 x 4.9 mm bay needs only case.scad's panel numbers, J3 and its pin map changed |
| FPC fit in J3 | too short a fold can't reach the connector; too long bows under the panel | the model errs long (safe); if the tail bows, move J3 1.5-2 mm towards seat 1 (`conn_rear` about -0.6); check with a real panel and connector before the board order |
| DS Lite silicone fit | cap retention and feel depend on unmeasured heights | pads and caps ordered: try them in a printed lid before the board order |
| Snap fit | too tight cracks a hook, too loose rattles; FDM snap fits wear | tune on a test print; PETG over PLA |
| JLC rotation offsets | parts placed 90 or 180 degrees off | review JLC's placement preview; the CPL carries KiCad's angles |
| Thin lid over the battery connector (0.6 mm) | cracks in the FDM-printed lid | 100% infill there for now; a lower-profile cell connector (JST SH/GH) removes the pocket |
