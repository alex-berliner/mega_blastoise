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

(2026-10-26: the 3.5 in panel is back in a 77.4 x 156.6 x 9.6 mm case, [case/SHELL.md](../case/SHELL.md); the dimensions in this review's older paragraphs are the earlier cases'.) Case 75.4 x 156.2 mm, 13.4 mm thick at the screen and 11.5 at the ends (14.6 until 2026-10-19, when the panel's top went into a pocket in the lid under a 0.8 mm skin, with a 0.5 mm pad over J2): the cell lies on the tub's floor under the board and the board rests on ribs 4.5 mm over the floor (since 2026-10-09; tag archive/ref-design-cell-on-board has the cell on the board under the screen, 13.9 mm and 7.86 at the ends). Each end steps down 1.9 mm to a button deck (a spine between the clusters stays full height over the end snap hooks; 2.34 until 2026-10-21, when the deck's lip became Alex's DS Lite faceplate's 1.6 mm), so genuine DS Lite caps stand 1.41 mm proud, as on a DS Lite; `case/DSLITE.md` has the cap numbers and their sources. Board 70.4 x 151.2 mm (70.5 x 151.34 over the outline's
line width, the size JLC quotes), 2 layers, 1.6 mm, ENIG,
every hole 0.3 mm or larger.

## Block by block

### Schematic overview and root sheet

The sheet-by-sheet detail under each block below comes from the schematic walk-through.

The schematic is a root sheet and eight sub-sheets in KiCad 10's format. Since 2026-10-20
[`board/tools/build_sch.py`](../board/tools/build_sch.py) writes them directly
from design.py (before that they went through Konnect), and
[`sch_sheets.py`](../board/tools/sch_sheets.py) lays each sheet out by hand as
functional blocks: parts wired to each other, decoupling beside its pins, series parts in a chain.
design.py decides how each net is drawn: a power rail gets a power symbol, a net used on more than
one sheet gets a global label, and a net used on one sheet only gets a local label (KiCad prefixes
it with the sheet's name). The drawing is traced and compared with design.py before it is
written; afterwards `check_netlist.py` compares KiCad's own netlist with design.py, and KiCad's
ERC runs.

In numbers: design.py has 173 reference designators, of which JLC places 118. The other 55 are
copper features or empty footprints: the 20 contacts, 16 test pads, 5 solder jumpers, J7's pads,
6 holes, 3 fiducials, and 4 unfitted footprints (C41-C43, R48). Its 101 named nets are 6 power
rails, 48 global labels and 47 local nets. Eight `#FLG` power flags are not parts: they tell ERC
that a rail fed through a jumper, diode or FET is driven.

The sheets, with links to their walk-throughs:

| Sheet (file) | Title block | Walk-through |
|---|---|---|
| root (`battler.kicad_sch`) | Sheets and mechanical parts | 7.4 |
| USB_Charger (`usb_charger.kicad_sch`) | USB-C, charger, load sharing, cell connector | 7.5 |
| Power (`power.kicad_sch`) | 3V3 regulator, power latch, cell voltage sense | 7.6 |
| MCU (`mcu.kicad_sch`) | RP2354B | 7.7 |
| MCU_Power (`mcu_power.kicad_sch`) | RP2354B supply: decoupling and core regulator | 7.8 |
| Debug (`debug.kicad_sch`) | SWD, UART, debug pads, BOOTSEL and RESET | 7.9 |
| Display (`display.kicad_sch`) | Panel connector and backlight sinks | 7.10 |
| Audio (`audio.kicad_sch`) | PWM audio and amplifier | 7.11 |
| Buttons (`buttons.kicad_sch`) | Membrane contacts | 7.12 |

**Power rails.** Six nets get power symbols. VSYS and VBAT use KiCad's VCC symbol renamed through
its value.

| Rail | Made by | Level | Used by |
|---|---|---|---|
| VBUS | the USB-C receptacle J1 | 5 V from the source; a simulated hot plug peaks at 6.7 V | the charger U2 and its TEMP and CE networks, D5 and D6 (load sharing), the bleeder R38, and R14 (USB holds the device on) |
| VSYS | D5 from VBUS, or Q2 and D7 from the cell | about 4.6 V on USB, the cell's voltage otherwise | the 3V3 regulator U3, the backlight anode (through JP2), the amplifier (through JP3), the power contact SW21 |
| VBAT | the cell, through J2 and JP5; U2 charges it | 4.2 V full; the firmware shuts down at a computed 3.50 V | Q2, D7, the divider R17/R18 |
| +3V3 | U3, through JP1, only while PWR_EN is high | 3.3 V | U4, the panel's logic (through JP4), the backlight op-amp U6, U1's reference, the pull-ups, J7 |
| +1V1 | U4's internal switcher, through L2 | 1.1 V | U4's core (DVDD) only |
| GND | | | every sub-sheet; poured on both layers and stitched by vias on a 5 mm grid |

VSYS is never switched: with a cell in, it is always live. "Off" means U3 is disabled, so +3V3
and +1V1 are gone. The loads that sit on VSYS directly are kept off by their own circuits: the
backlight sinks run from 3V3 (the Display sheet) and the amplifier is held in shutdown (the Audio sheet).

```
 J1 USB-C --- VBUS --+-- R47 -- D5 ----------------------+-- VSYS
                     |                                   |
                     +-- U2 TP4056 -- VBAT -- Q2 and D7 -+
                                       |
 J2 cell --- JP5 ----------------------+

 VSYS ---+-- U3 ME6211, enabled by PWR_EN -- 3V3_LDO -- JP1 -- +3V3
         +-- JP2 -- LED_A: the panel's six backlight LEDs
         +-- JP3 -- AMP_VDD: the FM8002A amplifier
         +-- SW21: seat 1's Start contact, the power button

 +3V3 ---+-- U4 RP2354B supply pins -- U4's switcher -- L2 -- +1V1 (core)
         +-- JP4 -- LCD_3V3: the panel's logic
         +-- U6: the backlight sinks' op-amp
         +-- U1's reference, the pull-ups R9, R10, R16, R23, J7's 3V3 pad
```

Each sub-rail behind a solder jumper (3V3_LDO, LED_A, AMP_VDD, LCD_3V3, CELL_P) is a local net.
Cutting one of JP1 to JP5 isolates that block or puts a meter in series with it.

**Signals between sheets.** Every global label but one runs between the MCU sheet and one other
sheet; the exception is PWR_BTN_RAW, from the power contact on the Buttons sheet to the latch on
the Power sheet.

```
                     Debug      MCU_Power
                         \       /
      USB_Charger ------- MCU (U4) ------- Display
                         /   |    \
                    Power  Audio   Buttons
                      |               |
                      +- PWR_BTN_RAW -+
```

| Between MCU and | Global labels | What they carry |
|---|---|---|
| USB_Charger | USB_DP, USB_DM | USB data (the series resistors are on the MCU sheet, the ESD array on USB_Charger) |
| | CC1_SENSE, CC2_SENSE | the USB-C CC lines through 10k, into ADC6 and ADC5 |
| | CHG_STAT_N, CHG_DONE_N, CHG_OFF | the charger's two status outputs, and the MCU's charge pause |
| Power | PWR_HOLD, PWR_BTN_N, VBAT_SENSE | the MCU's hold on the latch, the power button as the MCU sees it, the cell voltage halved |
| MCU_Power | VREG_AVDD, VREG_LX | the core switcher's filtered analogue supply and its inductor node |
| Debug | SWCLK, SWDIO, RUN, QSPI_SS, UART_TX, UART_RX | SWD, reset, the BOOTSEL strap, the UART console |
| Display | LCD_SCK_MCU, LCD_MOSI_MCU, LCD_MISO, LCD_CS, LCD_DC, LCD_RST, LCD_TE, BL_PWM | the panel's SPI bus and control lines, and the backlight's set point |
| Audio | AUDIO_PWM, AMP_EN | the audio PWM, and the amplifier's enable |
| Buttons | P1_UP to P1_SELECT (9), P2_UP to P2_START (10) | one net per contact; seat 1's Start is PWR_BTN_RAW instead |

SCK and MOSI change name at their series resistors on the Display sheet: `_MCU` on the chip's side,
plain on the panel's.

The root sheet holds a title note, the eight sheet symbols (each with a four-line note of the
blocks inside it) and the parts that have no circuit. Neither kind of mechanical part is in JLC's
BOM or placement file: there is nothing to place.

| Parts | Role | Why |
|---|---|---|
| H1-H6 | 2.2 mm holes for M2 screws | the case is held by six M2 screws through the board, which clamp tub and lid as on a DS Lite; the positions come from case.scad (through `positions.json` and placement.py), so the board always matches the case |
| FID1-FID3 | 1 mm fiducials with a 2 mm mask opening | optical alignment marks for JLC's placement machines; the router keeps 0.6 mm of copper clear around each |

The board fits the case one way only: J1 and J2 key it (PLAN.md, Case row).

### USB-C and charging (sheet USB_Charger)

- Sink only: 5.1k Rd on CC1 and CC2, no PD. A Type-C charger turns VBUS on
  because of them. VBUS bulk capacitance 10 uF (USB's limit).
- SRV05-4 on D+/D-, next to the connector: a 4-line TVS array referenced to 3V3
  (two of its lines unused). Not VBUS: with no cable in, the MCU's D+ pull-up (on in
  BOOTSEL mode and whenever USB is running) would feed VBUS through the D+ steering
  diode to about 2.7 V, which holds PWR_EN up and half-closes Q2. Semtech's SRV05-4.TCT
  (C13612, Extended, stocked at JLC; hongjiacheng's SRV05-4A held the spot 2026-10-20 to 10-22).
  JLC's footprint for it is KiCad's turned 180 degrees: check U1 in the placement preview.
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
  ME6211's maximum was 6.5 V; the RT9013 that replaced it is rated 6 V, and with the BQ24073
  VSYS is its regulated OUT, 4.3-4.5 V). Most of the ring goes through D5 into the VSYS capacitors,
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
  curves, the TP4056's shut-off, the ME6211's dropout (the RT9013's is lower) and the routed copper), checked
  by two independent model reviews; bring-up step 5 measures it.

#### USB-C, charger, load sharing and cell connector (sheet USB_Charger)

**What it does.** This sheet takes 5 V from USB-C, charges the cell, and makes VSYS, the system
rail: from USB when a cable is in, from the cell otherwise. It also passes USB data on to the MCU
sheet and tells the MCU what the charger and the USB source are doing. Its blocks, as the sheet
heads them: the USB-C receptacle and CC resistors, the ESD array, the VBUS input, the charger,
the cell temperature window, load sharing, and the cell connector.

**Load sharing.** USB feeds VSYS through D5 while the TP4056 charges the cell on its own output.
Without USB, a P-channel FET (Q2) connects the cell to VSYS.

```
               R47 0.1 ohm           D5
 VBUS --+------/\/\/---------------|>|------+-----------+----- VSYS
        |                                   |           |
        |      D6                         +----+      +----+
        +------|>|------ LS_GATE ---------| Q2 |      | D7 |
        |                    |            +----+      +----+
        |                 R6 10k            |           |
        |                    |              |           |
        |                   GND             |           |
        +-- U2 TP4056 --- BAT --------------+-----------+----- VBAT -- JP5 -- J2 (cell)

 Q2: AO3401A P-channel FET: source on VSYS, drain on VBAT, gate on LS_GATE.
 D7: B5819W Schottky, anode on VBAT, cathode on VSYS: in parallel with Q2's body diode.
```

- **USB in:** VSYS is VBUS less D5's drop, about 4.6 V (bring-up expects 4.5 to 4.7 V). LS_GATE
  sits at VBUS less D6's drop, at or above VSYS, so Q2 is off; Q2's body diode and D7 are reverse
  biased.
- **USB out:** R6 pulls LS_GATE to ground and Q2's channel joins the cell to VSYS (at most
  85 mOhm with 2.5 V of gate drive).
- **The unplug.** At an unplug C1 holds VBUS up, and LS_GATE can only fall as fast as VBUS does.
  R38 (1k) drains VBUS, so Q2's channel takes over 1.5 to 3.3 ms after the cable leaves (about 2 to
  3 ms in design.py's comment; tens of ms without R38). Until then D7 carries VSYS at the cell less
  about 0.34 V, and C9 covers the step.

**Parts.**

| Parts | Role | Why this part or value |
|---|---|---|
| J1 (HRO TYPE-C-31-M-12) | USB 2.0 Type-C receptacle: VBUS, CC1 and CC2, D+ and D- (both rows joined), shield to GND; the SBU pins are left open | one USB-C for charging, firmware and console (the stripboard had two USB ports) |
| R1, R2 (5.1k) | Rd pull-downs on CC1 and CC2 | they mark the board as a power sink: a Type-C source turns VBUS on only when it sees them. There is no USB PD |
| R3, R4 (10k); C42, C43 (empty) | CC1 and CC2 into ADC6 and ADC5 (GPIO46, GPIO45) | firmware reads the source's current offer; the 10k keeps a CC line shorted to VBUS off the pin. C42 and C43 are empty 1206 footprints for filter capacitors, fitted by hand only if bring-up shows noise |
| U1 (SRV05-4A, SOT-23-6) | ESD steering array on D+ and D-; its other two lines are unused | 1.0 pF, 15 kV contact and air discharge, next to the connector. Its reference pin is on 3V3, not VBUS (see Protection). hongjiacheng's part since 2026-10-20 (Alex): Preferred at JLC, so no loading fee. Before it: Semtech's Extended SRV05-4.TCT from 2026-10-07, when ProTek's part ran out |
| R41 (2.2 ohm 0603), C1 (10 uF 25 V) | the VBUS bulk capacitor, behind a series resistor | 10 uF is USB's limit for bulk capacitance on VBUS. The resistor is the TP4056 datasheet's fix for a ceramic input capacitor ringing when a live source is plugged in (it asks 1.5 ohm; 2.2 is the nearest Basic part). Added 2026-09-26 |
| R38 (1k 0603) | VBUS bleeder | it drains VBUS after an unplug so Q2 takes over in milliseconds, and it holds D5's leakage away from PWR_EN (see Protection). It costs 5 mA (25-28 mW) with USB in, so it is an 0603 at 25-30% of its rating. 10k on 2026-09-25, 1k after the third audit round the same day, 0603 on 2026-09-26 |
| U2 (TP4056, ESOP-8) | linear Li-ion charger to 4.2 V | a fifth of the BQ24074's price with no stock risk (2026-09-24). Its thermal pad has six 0.3 mm vias into the ground pours on both layers; it lowers its own current when its die passes about 145 C, so heat slows the charge rather than hurting the chip |
| R5 (3.3k) | sets the charge current | 1100 or 1200 V (by datasheet revision) / 3.3k = 333-364 mA, about 0.5 C for the 700 mAh cell, the usual standard rate for small pouch cells. It gave 460-500 mA until 2026-10-05, which the validation plan judged too high for this cell |
| R39 (5.6k), R40 (150k), TH1 (10k NTC, B3380); C41 (empty) | the charger's TEMP input | the TP4056 charges only while TEMP sits between 45% and 80% of VBUS. With TH1 in parallel with R40 that is 1 to 46 C (-3 to 49 C across the chip's threshold spread). The 403450 has no thermistor wire, so TH1 sits on the board over the cell, reading it through the board, about 45 mm from U2 so that U2's 0.65 W while charging doesn't trip it. Fed from VBUS: about 0.6 mA from USB, nothing from the cell. Added 2026-09-25 |
| R43 (100k), Q4 (2N7002), R44 (4.7k) | the charger's enable (CE) | R43 pulls CE to VBUS, so the charger charges whenever USB is in and nothing drives Q4, even with a blank or crashed MCU. CHG_OFF (GPIO19) high turns Q4 on and pauses charging (a weak USB port, a warm case). R44 holds Q4 off through any processor state, at 4.7k because erratum RP2350-E9 needs 8.2k or less. Added 2026-10-05 |
| R9, R10 (100k to 3V3) | pull-ups on the open-drain STDBY (CHG_DONE_N, GPIO18) and CHRG (CHG_STAT_N, GPIO17) | the MCU reads the charger's state; there are no charge LEDs (an approved decision) |
| R47 (0.1 ohm 1206) | damper in D5's path | a live plug-in rings through D5 into the VSYS capacitors: with no cell (or a tripped pack) VSYS peaked at about 6.6 V against the FM8002A's 6 V maximum, about 5.7 V with R47. It costs 50 mV of VSYS at 0.5 A. Added 2026-10-05 |
| D5 (B5819W, 1 A Schottky) | USB into VSYS | a Schottky keeps the drop small, so VSYS sits near 4.6 V on USB |
| D6 (BAT54WS), R6 (10k) | Q2's gate follows VBUS | with USB in the gate is high and Q2 off; without, R6 pulls it low |
| Q2 (AO3401A) | cell into VSYS | P-channel: with LS_GATE at ground its gate sees the whole cell voltage (about -3.4 V at a 3.5 V cell), more than the -2.5 V at which its on-resistance is rated 85 mOhm or less |
| D7 (B5819W) | across Q2 | carries VSYS through the unplug gap at 0.34 V (at 0.25 A) against the body diode's 0.6 V. Added 2026-09-26 after two independent model reviews |
| C9 (22 uF 25 V), C2 (4.7 uF); C3 (10 uF) | VSYS bulk; the charger's output capacitor on VBAT | C9 covers the step at an unplug |
| J2 (Molex PicoBlade 53261-0271), JP5 | the cell connector; a cut jumper from CELL_P to VBAT | J2 is 3.4 mm tall: the 5.5 mm JST PH it replaced on 2026-10-19 needed a pad on the lowered lid. Cells come with PicoBlade ("MX1.25") plugs. Cutting JP5 and bridging it with an ammeter reads everything the cell gives or takes |

**Protection and failure cases.**

- **Hot plug.** The worst case is an A-to-C cable from an always-on USB-A port: a USB-C source
  turns VBUS on only after it sees the Rd resistors, and ramps it. Simulated with a 2 m cable into a
  board with no cell, R41 brings the peaks from 7.1 to 6.7 V on VBUS (U2's maximum is 8 V) and from
  6.7 to 6.2 V on VSYS (the ME6211's maximum is 6.5 V). R47, added later, brings VSYS from about
  6.6 V to about 5.7 V in design.py's own figures, under the FM8002A's 6 V. With a cell in, the
  peaks are lower. There is no TVS on VBUS: an SMF5.0A's 5.0 V working voltage is below USB's
  5.5 V, and it would barely conduct at these peaks.
- **The unplug gap.** Without D7, Q2's body diode would carry the gap at about 0.6 V (0.25 A, full
  backlight): 3V3 would then be the cell less about 0.71 V, under the internal flash's 2.97 V
  minimum (QSPI_IOVDD) on every unplug with the cell below about 3.7 V. With D7, 3V3 stays above
  2.97 V down to a cell of about 3.45 V (design.py; design_review.md gives 3.42 V, or 3.49 V with
  0.25 A of audio as well), inside the 3.5 V cutoff. This matters because the RP2350 has no 3V3
  supervisor (its brown-out detector watches the 1.1 V core): a dip would not reset it cleanly, it
  could corrupt a flash access. And a reset turns the device off on battery.
- **Back-powering through D+.** U1's reference pin was on VBUS until 2026-09-25. With no cable in,
  the MCU's D+ pull-up (on in BOOTSEL mode and whenever USB runs) then fed VBUS through U1's D+
  steering diode to about 2.7 V, which held PWR_EN up (the device could not turn off) and left Q2
  half on. On 3V3 the path is gone.
- **Leakage.** With the cell on VSYS, D5's reverse leakage flows into the VBUS node, and from there
  through R14 and D3 into PWR_EN. Into R15's 100k alone, about 10 uA (a warm Schottky) would reach
  the ME6211's 1.0 V enable level; with R38 on VBUS it takes about 1.3 mA. In the other direction,
  D7's reverse leakage flows into the cell while USB is in: about 5 uA at 25 C and 50 uA at 75 C. A
  full cell left on USB therefore floats near 4.2 V instead of cycling through the 4.05 V recharge
  threshold, a small calendar-ageing cost backstopped by the pack's overvoltage cut-off.
- **Charging outside 1 to 46 C** pauses by itself and resumes when TH1 is back in range. CHRG and
  STDBY then both read high, as with no USB; firmware tells the two apart by the CC lines.
- **A weak USB port.** The TP4056 has no input current limit, so a 500 mA default-USB port can be
  asked for the charge current plus the system's draw. Firmware reads the source's offer on the CC
  lines and dims the backlight on a weak one.
- **A reversed cell destroys the charger.** PicoBlade plugs have no standard polarity; the red lead
  must land on J2's pin 1 (silkscreen `+`), checked before plugging in
  ([assembly.md](assembly.md)).

**Bring-up hooks and firmware.** Probe pads: TP1 (VBUS), TP2 (VSYS) and TP3 (VBAT) are drawn on the
Power sheet, TP7 (LS_GATE) here, and TP15 is a ground clip by the charger. JP5 reads the cell's
current; C41-C43 take hand-fitted filter capacitors. Bring-up step 1 (USB, no cell) checks VBUS
and VSYS; step 5 covers the charger, the temperature cutoff with a hair dryer and a cold pack, the
scoped unplug with the cell at 3.5 V, the hot plug with no cell, and PWR_EN at plug-in. Two of
step 0's unpowered expectations predate D7 and R38: the diode test from TP3 to TP2 reads D7's drop
(about 0.25 V, per the validation plan), not Q2's body diode, and TP1 reads R38's 1k to ground.
Firmware:

- Turn off the pull-downs of CHG_STAT_N and CHG_DONE_N at boot (every GPIO resets with its
  pull-down on, which fights the 100k pull-ups). Decode the states with the table in
  [pinmap.md](pinmap.md): low/high charging, high/low done, high/high no
  USB or a temperature pause. The validation plan adds that the status pins can blink.
- Read CC1_SENSE and CC2_SENSE and take the higher: with 5.1k Rd the active line sits at
  0.25-0.61 V for default USB, 0.70-1.16 V for 1.5 A and 1.31-2.04 V for 3 A.
- Drive CHG_OFF high to pause charging; release it (R44 pulls it low) to charge.

**Open items and risks.**

- The unplug hand-over and the hot plug are simulated (the AO3401A and B5819W curves, the TP4056's
  shut-off, the ME6211's dropout, the routed copper; checked by two model reviews), not measured.
- C9's DC-bias derating, with the unplug and hot-plug simulations re-run, is item 8 of the
  validation plan's go/no-go gate; no document records it as done.
- D5's surge rating and R47's short-time overload against the plug-in inrush are marked unknown in
  the pin review ([pin_review/d5.json](../board/pin_review/d5.json),
  [r47.json](../board/pin_review/r47.json)); the TP4056 datasheet misprints the
  absolute maximum for BAT and the status pins ([u2.json](../board/pin_review/u2.json)).
- The validation plan asks for a short failure table of D5, D7, Q2 and U2: a shorted D7 or Q2
  would charge the cell straight from USB with no regulation.
- TH1 reads the board over the cell and lags it; a charge in a hot car or near 46 C may pause and
  resume.
- The cell's length with its protection board, and its plug polarity, are still to be measured.

### 3V3 and the power switch (sheet Power)

- RT9013-33GB LDO from VSYS (the ME6211 until 2026-10-22, the same pinout; pin 4 is NC on
  both): 500 mA maximum, 250 mV typical and 400 mV maximum dropout at 500 mA, so about 50-80 mV
  at 100 mA against the ME6211's 120 mV. VIN 5.5 V maximum (6 V absolute) against VSYS at
  most 4.5 V; EN high from 1.6 V, 6 V absolute whatever VIN is; at least 1 uF ceramic in and
  out with an ESR over 5 mOhm (2 x 10 uF on its output here). It holds 3.3 V
  down to a cell of about 3.42 V (a little lower with its dropout); below that 3V3 follows the cell. The firmware shuts
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
  about 10 uA (a warm Schottky) would reach the ME6211's 1.0 V CE level (the RT9013's EN needs 1.6 V). R38 (1k, the
  unplug bleeder, see USB) holds VBUS down: that now takes about 1.3 mA. It costs 5 mA
  with USB in.
  (A 10k R15 would do the same but cut how much resistance the Start contact's pill may have
  before Q1 misses a press by about five times.)
- The button is seat 1's Start contact (SW21; seat 1's Y until 2026-10-19, which put it
  on Start so no game button doubles as power). Its common comb is VSYS instead of
  GND, so pressing Start joins VSYS to PWR_BTN_RAW, as the side switch did. The MCU
  sees it through Q1 (2N7002, gate on the button node) on GPIO3, so no MCU pin
  ever touches VSYS; firmware reads seat 1's Start there.
- Off state: RT9013 off (EN low, 0.7 uA), amplifier shut down (R36 holds SHUTDOWN high,
  4.2 uA), backlight sinks off (they run from 3V3; R56/R57 hold the FET gates low), charger drawing about 2 uA from the
  cell, battery divider 4.5 uA (up to 9 uA: with the chip unpowered, GPIO47 sits above
  IOVDD and its ESD diode takes a few uA, harmless and the same as on Raspberry Pi's
  Pico), and D5's reverse leakage from VSYS into R38 (1-6 uA at 25 C). About 18-22 uA
  total: years of shelf life. The divider stays at 470k/470k for this reason: 100k/100k
  would add about 30 uA, most of it into the unpowered chip's clamp, and still not
  guarantee the reading against the pad's worst-case leakage.
- A reset or crash drops PWR_HOLD. On the cell alone that turns the device off,
  which is the safe failure. On USB it stays on (VBUS holds PWR_EN).

#### 3V3 regulator, power latch and cell voltage sense (sheet Power)

**What it does.** This sheet makes the 3V3 rail from VSYS and decides when it exists. It holds the
soft power latch, the circuit that lets the MCU see the power button without touching VSYS, the
cell voltage divider for the ADC, and most of the board's probe pads. Its blocks: the 3V3
regulator, the power latch, button sense, cell voltage sense, and ground pads.

**The latch.** U3's enable (PWR_EN) is the OR of three sources, each through its own Schottky
diode so none can back-feed another:

```
 VSYS --[SW21: seat 1 Start]--+-- PWR_BTN_RAW ------|>|--+   D2, first diode
                              |                          |
                       R13 33k to GND                    |
                       Q1's gate (PWR_BTN_N to GPIO3)    |
                                                         |
 GPIO2 (PWR_HOLD) ------------+---------------------|>|--+   D2, second diode
                              |                          |
                       R42 4.7k to GND                   +-- PWR_EN --> U3 (ME6211) enable
                                                         |   R15 100k and C47 100 nF to GND; TP6
 VBUS -- R14 10k -- VBUS_EN ------------------------|>|--+   D3
```

1. **Off.** R15 holds PWR_EN low; U3 is off and there is no 3V3. The cell still feeds VSYS.
2. **Press Start.** SW21 joins VSYS to PWR_BTN_RAW, D2 raises PWR_EN, U3 starts and the MCU boots.
   Q1 sees the press and pulls PWR_BTN_N (GPIO3) low.
3. **Hold.** Firmware sets PWR_HOLD high once Start has been held for about a second, so a knock in
   a bag doesn't turn the device on: let go sooner and it powers off again.
4. **Turn off.** Firmware watches PWR_BTN_N for a long press and drops PWR_HOLD; power goes when
   Start is released.
5. **USB in.** VBUS holds PWR_EN up through R14 and D3 whatever PWR_HOLD does, so the firmware runs
   while charging.

**Parts.**

| Parts | Role | Why this part or value |
|---|---|---|
| U3 (ME6211C33, SOT-23-5); C5, C6, C7 (10 uF) | 3V3 LDO from VSYS, enabled by PWR_EN; input and output capacitors | $1.40 a unit cheaper than the buck-boost regulator it replaced (2026-09-24). 500 mA maximum, 120 mV dropout at 100 mA: it holds 3.3 V down to a cell of about 3.42 V, and below that 3V3 follows the cell. The load is under 100 mA (the RP2354B up to about 50 mA, the panel's logic about 10 mA), at most 0.13 W in the LDO on USB. The cost: roughly 5-10% of the cell goes unused against the buck-boost |
| JP1 | joins the regulator's output (3V3_LDO) to +3V3 | cut at bring-up to check the regulator alone before the rest of the board sees 3V3, or to feed the rail from a bench supply on TP4 |
| D2 (BAT54C, common cathode) | ORs the button and PWR_HOLD into PWR_EN | two Schottky diodes in one SOT-23 |
| R14 (10k), D3 (BAT54WS) | USB into the OR | keeps the device on while USB is in |
| R15 (100k) | PWR_EN pull-down | 100k, not 10k: a 10k cut by about five times how much resistance the Start contact's pill may have before Q1 misses a press. It was 10k for part of 2026-09-25, until the R38 bleeder took over the leakage job |
| C47 (100 nF) | slows PWR_EN's rise from VBUS to about 1 ms (R14 x C47) | at a live plug-in VSYS lags behind R47 and D5, and the ME6211's enable may not go more than 0.3 V above its input (its absolute maximum). Found by the datasheet pin review, 2026-10-05 |
| R42 (4.7k) | PWR_HOLD pull-down | erratum RP2350-E9 can hold a released pad near 2 V; through D2 that would keep the regulator on after a crash and drain the cell in a bag. E9 needs 8.2k or less. Added 2026-10-05 |
| R13 (33k) | PWR_BTN_RAW pull-down | 33k, not 100k: the node tolerates about twice the surface leakage (sweat, condensation) across the Start contact before it lifts PWR_EN (2026-10-05) |
| Q1 (2N7002), R16 (100k to 3V3) | button sense: gate on PWR_BTN_RAW, drain on PWR_BTN_N (GPIO3) | the MCU reads the power button (low = pressed) and no MCU pin ever touches VSYS |
| R17, R18 (470k each), C8 (100 nF) | the cell's voltage halved, into ADC7 (GPIO47) | 470k keeps the divider's drain to 4.5 uA. 100k/100k would add about 30 uA, most of it into the unpowered chip's clamp, and still not guarantee the reading against the pad's worst-case leakage. C8 smooths the 235k source (23.5 ms time constant) |
| TP1-TP5; TP6; TP15, TP16 | probe pads | VBUS, VSYS, VBAT, 3V3 and GND on the back; PWR_EN on top; two GND pads on top for scope ground clips (2026-10-19) |

**Protection and failure cases.**

- **A crash or reset turns the device off on battery.** The reset releases GPIO2 and R42 pulls
  PWR_HOLD low; once Start is up, PWR_EN falls. On USB it stays on. This is the intended safe
  failure: a device that crashes in a bag turns itself off instead of draining the cell.
- **A hang with PWR_HOLD high** keeps the device on with no way to turn it off but unplugging the
  cell, so the firmware enables the watchdog before any blocking initialisation.
- **The enable above its input at plug-in** (C47) and **leakage lifting PWR_EN** (R38 on the
  USB_Charger sheet, R13 here) are covered above.
- **Off-state drain**, about 18-22 uA in all: the ME6211 off; the amplifier in shutdown (4.2 uA);
  the backlight sinks off; the charger drawing about 2 uA from the cell; the divider 4.5 uA (up to
  9 uA, because with the chip unpowered GPIO47 sits above IOVDD and its ESD diode takes a few uA,
  as on Raspberry Pi's Pico; accepted); and D5's reverse leakage from VSYS into R38 (1-6 uA at
  25 C). That is years of shelf life.
- **The ADC reads against 3V3.** Once the LDO drops out, 3V3 follows the cell and the computed cell
  voltage levels off at 3.37-3.43 V whatever the cell really is, so a cutoff below that would never
  fire. The firmware shuts down at a computed 3.50 V, which also keeps 3V3 above the flash's
  2.97 V through an unplug.

**Bring-up hooks and firmware.** Bring-up step 1 checks 3V3 on TP4 with USB only (R14 and D3 raise
PWR_EN). Step 5.1 connects the cell with USB out and expects nothing to turn on and about 20 uA
(under 30) through JP5. Step 5.7 scopes PWR_EN against VSYS at a live plug-in. Step 6 exercises the
switch: Start powers the board while held, the hold keeps it on, a long press turns it off, and
RESET on battery turns it off. Firmware:

- Turn off GPIO3's pull-down at boot. Set PWR_HOLD only after Start has been held about a second;
  drop it after a long press. On USB, show a charging screen, turn the backlight off and sleep.
- Enable the watchdog before any blocking initialisation.
- The validation plan's power-off order: DISPOFF, then SLPIN, then drop the hold.
- Cell voltage: VBAT = 2 x ADC7 x 3.3 V / 4096, times a gain calibrated once when CHG_DONE_N goes
  low (the cell is then at 4.2 V +-1%). Keep the ADC channel for the whole program (in embassy-rp
  0.8 dropping it turns the pad's digital input and pull-down back on, which drains C8), wait about
  200 ms after setting it up, read bursts of up to 16 one-shot samples at most once a second, and
  shut down only after several low bursts. Details in [pinmap.md](pinmap.md).

**Open items and risks.**

- The latch has not been built. The validation plan asks for a replica from the real parts that
  holds and releases at VSYS 3.5 and 4.2 V and still starts with 1 kohm in series with the button;
  whether that gates the order is disputed.
- A hardware long-press kill (an RC into Q1's gate, as an unfitted footprint) was proposed in the
  validation plan because a watchdog doesn't cover firmware that runs correctly but never drops
  the hold. It was not opposed, and it is not in design.py.
- The pin review notes that the SOT-23's 300 mW limit caps 3V3 at about 158 mA with VSYS at 5.2 V;
  the design's load is under 100 mA.
- A lower-dropout regulator would widen the unplug margin and use more of the cell; it is the
  respin fallback if bring-up step 5 shows 3V3 under 2.97 V at a 3.5 V cell.
- [pinmap.md](pinmap.md) and [bringup.md](bringup.md)
  step 6 still name seat 1's Y as the power button; design.py moved it to Start on 2026-10-19.

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
- Debug: SWD on JST SH 3-pin in the Raspberry Pi Debug Probe pinout (the UART connector
  and the revision straps went on 2026-10-24 to clear the left strip); BOOTSEL and RESET
  buttons under pinholes in the left bezel. No LEDs.
- GPIO map: [pinmap.md](pinmap.md).

#### RP2354B (sheet MCU)

**What it does.** This sheet holds U4 and what it needs besides power: the crystal, the USB series
resistors, the QSPI_SS boot strap's pull-up, the board revision straps, and a label on every GPIO
that leaves for another sheet. Supplies are on the MCU_Power sheet and debug parts on the Debug
sheet.

**The chip.** U4 is an RP2354B in QFN-80: the RP2350B die (48 GPIOs) with 2 MB of flash stacked in
the package. Only the flash's chip select, QSPI_SS, comes out, as the BOOTSEL strap; the five QSPI
data and clock pins are left open. Raspberry Pi's RP2350B minimal design (MIT) supplies the core
circuit and its layout, turned 90 degrees.

**Pin assignment.** With two layers there is no room for routes to cross, so each pin faces where
its net goes and, along an edge, the pins are in the order their nets fan out: seat 1 below the
chip, seat 2 above, the charger and USB to the right, the panel's connector below. Seat 1's cross
and the panel's TE leave down the left edge on GPIO28-32; seat 1's face buttons, the panel's reset,
the backlight and the ADC inputs run along the bottom edge on GPIO40-47. Raspberry Pi's own pin
escapes are copied with the core, so every used pin leaves the chip on their track. GPIO0, 1, 16,
26 and 27 are spare. The full table is [pinmap.md](pinmap.md).

**Parts.**

| Parts | Role | Why this part or value |
|---|---|---|
| U4 (RP2354B) | the microcontroller | no flash chip to route (2026-09-24); a bare chip because no RP2350B module exists that JLC will place, and the Pico family has too few GPIOs (2026-09-21). $1.43 at 10 |
| X1 (Abracon ABM8-272-T3, 12 MHz); C27, C28 (15 pF); R20 (1k) | the crystal, its load capacitors, and a series resistor on XOUT | as in Raspberry Pi's design: two 15 pF in series make 7.5 pF, plus about 3 pF of stray, 10.5 pF against the crystal's 10 pF. R20 limits the drive into the crystal, per Raspberry Pi's hardware guide |
| R21, R22 (27 ohm) | series resistors on D+ and D- | the RP2350 datasheet requires 27 ohm. They sit in Raspberry Pi's own 0402 spots (a 27 ohm 0603 doesn't fit the copied layout; the 27 ohm 0402 is an Extended part). 22 ohm until the datasheet pin review of 2026-10-05 |
| R23 (10k to 3V3) | QSPI_SS pull-up | holds the internal flash's chip select high at power-up (the reference's value) |
| R48-R51 (1k, or unfitted) | 4-bit board revision on GPIO22-25 | a 1k to GND reads 0; an unfitted bit reads 1 through the pin's pull-up. This board is revision 1, so R48 is unfitted. JLC fits the 0 bits, so a new revision changes the BOM, not the copper; a resistor rather than a hard tie in case firmware ever drives a pin high. Added 2026-10-05 |

**Protection and failure cases.** RUN has an internal pull-up, so it needs no external one (a
reviewer's claim otherwise was rejected on 2026-09-25). Erratum E9 is covered by 4.7k external
pull-downs on PWR_HOLD (R42) and CHG_OFF (R44), under E9's 8.2k limit. AMP_EN and BL_PWM rely on
larger pull-downs (100k, 103.9k), which the pin review finds safe while they are outputs or at
reset (input buffer off), not if firmware releases them as inputs with the buffer on. Buttons and
REV straps use pull-ups, which E9 doesn't affect.

**Bring-up hooks and firmware.** A blank chip boots into its USB boot loader, so the first sign of
life is a drive called RP2350 on the computer (bring-up step 1): that proves the chip, the crystal,
USB and the 1.1 V core at once. Step 2.5 reads the revision straps. Firmware:

- embassy-rp with the `rp235xb` feature (the current firmware builds for `rp235xa`), and a memory
  map with 2048K of flash. The validation plan proposes generating `pins.rs` from design.py.
- Every GPIO resets with its pull-down on. Turn on the pull-ups of the buttons and REV0-REV3, turn
  off the pull-downs of PWR_BTN_N, CHG_STAT_N and CHG_DONE_N, and turn off both the input buffer
  and the pull-down on the ADC pins.
- Erratum E12: firmware using USB must run clk_sys at 1.1 times clk_usb or more.

**Open items and risks.**

- **The RP2354B was out of stock at JLC and LCSC in the 2026-10-21 BOM audit, which blocks ordering**
  (the manufacturing and cost section of bom.md).
- The chip's stepping is unknown. Errata E9 and E16 apply only to the A2 stepping, and which one
  JLC supplies isn't known; the E9 pull-downs are fitted either way.
- Whether the firmware and its assets fit 2 MB of flash has not been measured. No firmware exists
  for this board yet.
- design_review.md and pinmap.md still describe 22 ohm USB resistors, and design.py's own comment
  above R21 still argues for 22 ohm; the value is 27 ohm.

#### Decoupling and the 1.1 V core switcher (sheet MCU_Power)

**What it does.** The RP2354B runs its I/O, USB, ADC and flash interface from 3V3 and its core from
1.1 V, which its own switching regulator makes from 3V3. This sheet holds the capacitors at every
supply pin and the parts around that switcher. Everything on it is copied, value for value, from
Raspberry Pi's minimal design.

```
 +3V3 ---+------------------------- VREG_VIN (U4)     C24 4.7 uF to GND
         +-- R19 33 ohm --+-------- VREG_AVDD (U4)    C26 4.7 uF to GND

 U4's switcher: VREG_LX -- L2 3.3 uH --+-- +1V1 --> DVDD (three pins) and VREG_FB (U4)
                                       +-- C25 4.7 uF, C20-C22 100 nF to GND
                                       +-- TP9
```

**Parts.**

| Parts | Role | Why this part or value |
|---|---|---|
| C10-C19 (ten 100 nF), C23 (10 uF) | 3V3 decoupling at IOVDD (eight pins), ADC_AVDD, USB_OTP_VDD, QSPI_IOVDD and VREG_VIN; bulk | Raspberry Pi's values and placement |
| C20-C22 (three 100 nF) | DVDD decoupling on +1V1 | the same |
| C24, C25 (4.7 uF 0402) | the switcher's input (on 3V3) and output (on 1V1) | on Raspberry Pi's own wide land pattern: the switcher's LX copper runs between their pads, which KiCad's larger pads would cut |
| R19 (33 ohm), C26 (4.7 uF) | RC filter into VREG_AVDD | gives the switcher's analogue supply a filtered 3V3 |
| L2 (Abracon AOTA-B201610S3R3, 3.3 uH) | the switcher's inductor | meets the RP2350 datasheet's requirements: 3.3 uH +-20%, DCR at most 250 mOhm (this part: 140 max), saturation at least 1.5 A (this part: 2.4 A). Polarity-marked: fit as drawn. $0.27 at 10 |
| TP9 | +1V1 probe pad, on the core's own via left of the chip | added 2026-10-19 |

**Layout.** The copied part of the board keeps what Raspberry Pi's layout depends on:
`build_pcb.py` copies their routing, vias and the small copper areas around the switcher, the
0402 passives use their land patterns, and over the chip the top layer is a 3V3 pour joined by
short stubs from the 3V3 pins. Clearances inside the copied core go down to 0.1 mm (0.15 mm on the
rest of the board), and the copied vias keep 0.6 mm pads with 0.3 mm holes, so the whole board
stays on JLC's standard process.

**Protection and failure cases.** The chip's brown-out detector watches the 1.1 V core, not 3V3,
so nothing on the board resets the chip cleanly if 3V3 dips. That is why the unplug gap (D7) and the
3.5 V cutoff matter. On the A2 stepping only, erratum E16 says a disturbed USB_OTP_VDD can corrupt
OTP reads; the pin review flags that the unplug dips toward 2.97 V go below that pin's 3.135 V
minimum.

**Bring-up.** Bring-up step 1's USB drive proves the core rail; TP9 measures it.

**Open items and risks.**

- **L2 was out of stock with the RP2354B in the 2026-10-21 BOM audit** (757 in stock on 2026-10-07);
  any replacement must meet the three requirements above.
- The validation plan asks for L2's orientation to be checked in JLC's placement preview, and for
  a netlist diff of the copied core against Raspberry Pi's design file (RP-010329).

#### SWD, UART, debug pads, BOOTSEL and RESET (sheet Debug)

**What it does.** Everything needed to load and debug firmware besides USB: two 3-pin
connectors for a Raspberry Pi Debug Probe, the same SWD on pads on the back, and the BOOTSEL
and RESET buttons. J7 and the board's probe pads came from Alex's debug-readiness review on
2026-10-19.

| Parts | Role | Why this part or value |
|---|---|---|
| J5 (JST SH, SM03B-SRSS-TB) | SWD: SWCLK, GND, SWDIO | the Raspberry Pi 3-pin debug connector pinout, so the Debug Probe's cable plugs straight in |
| J6 (JST SH, SM03B-SRSS-TB) | UART console (UART1, GPIO20 and GPIO21) | the same spec (RP-003139-SP), named from the board's side: pin 1 is the board's RX, where the probe's TX arrives, and pin 3 its TX. The two were swapped until the pin review of 2026-10-05 |
| J7 (six bare pads at 2.54 mm, on the back) | 3V3, SWCLK, SWDIO, RUN, BOOTSEL, GND | for pogo pins in a jig, a clip or soldered wires: flash and debug without J5's cable or USB. Not assembled: it is copper only |
| SW18 (BOOTSEL), R25 (1k) | pulls QSPI_SS to GND through 1k | holding it through a reset enters the USB boot loader; the reference design's strap |
| SW19 (RESET), R26 (1k) | pulls RUN to GND through 1k | resets the chip |

Both buttons are XKB TS-1088 tactile switches under pinholes in the lid. There are no LEDs (an
approved decision).

**Protection and failure cases.** J7's 3V3 pad feeds the 3V3 rail directly, so power the board
through it only with the cell out and nothing on USB. RESET on battery turns the device off, since
the reset drops PWR_HOLD; that is the intended behaviour, and it means a player's only reset is
power-cycling.

**Bring-up and firmware.** Blank flash always lands in the boot loader. With firmware on it, hold
SW18 and tap SW19 to get back there; on J7, short BOOT to GND while tapping RUN to GND. Bring-up
step 2 attaches probe-rs or openocd through J5 and then J7, and reads the bring-up firmware's
report on J6 at 115200 baud.

**Open items.** No LED means no visible error indicator beyond the screen (the Known gaps section below).

### Display (sheet Display)

- EastRising ER-TFT035IPS-6: 3.5in IPS, 320x480, ILI9488, 4-wire SPI, 3.3 V,
  54.50 x 83.00 x 2.30 mm (back since 2026-10-26; it was replaced by the 2.4in ER-TFT024IPS-3 from 2026-10-21 to then).
  Sold by the maker: $14.20 a unit at 10, $9.86 at 50 ([bom.md](bom.md)). Its datasheet
  ([datasheet](https://www.buydisplay.com/download/manual/ER-TFT035IPS-6_Datasheet.pdf)) was not re-read on 2026-10-26.
- Its 50-contact FPC (0.5 mm pitch, 0.3 mm thick with its stiffener) leaves the
  panel's seat-1 edge, folds under the panel and plugs into J3, a Hirose FH12-50S
  bottom-contact ZIF connector that JLC places: no hand soldering. J3 sits where the
  folded FPC's tip reaches less 1 mm of slack (case.scad `conn_in`), its mouth towards
  seat 1. Seen from above the FPC's contact 1 lands at the right, J3's pin 1 is at the
  left, so J3 pin j carries panel pin 51 - j (`board/design.py`).
- Pins wired as the datasheet says: the parallel bus (DB0-17) and the RGB strobes low,
  RD high, IM3-0 = 1110 (4-line 8-bit serial interface II: SDI in, SDO out, D/CX on pin
  36, SCL on pin 37); the touch pins open (no touch layer). TE goes
  to GPIO32 so the firmware can start frames on the panel's refresh and avoid tearing.
- The ST7789V takes 16-bit colour over serial: two bytes a pixel, 154 kB a full frame.
  The battle screen redraws in parts.
- SPI clock and data get 33 ohm source terminations next to the MCU.
- Backlight: four LEDs in parallel (3.2 V typical, 3.4 V max; 70 mA typical, 80 mA at
  most), common anode on pin 1, a cathode pin each (K1-K4, pins 2-5). The anode is on VSYS
  (through JP2); K1 with K3 and K2 with K4 each go to a linear current sink: half of U6
  (MCP6002) drives an AO3400A (Q5, Q6) and holds its 2.2 ohm sense resistor (R52, R53) at
  BL_REF. BL_REF is GPIO28's PWM through R60/R61 (2.4k) and C50, 0.077 V at 100%: 35 mA a
  sink, 70 mA in all. The sink needs about 0.08 V, so the current holds from a full cell
  (or USB) down to the 3.5 V cutoff even at the 3.4 V maximum Vf. The two LEDs on a sink
  share its current by their Vf match. Efficiency is that of the AP3156 in its 1x mode (Vf / VSYS), so
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
  op-amp's offset (4.5 mV max) sets the accuracy: +-6% at full, +-20% a sink at 29%
  duty, so firmware keeps the dimmest level at about 29%. The loop was simulated at the
  3.5in panel's 56 mA a sink (R61 3.9k again; the 2.4in panel ran it at 35 mA with R61 2.4k).

#### Panel connector and backlight sinks (sheet Display)

**What it does.** The panel is an EastRising ER-TFT035IPS-6: 3.5 in IPS, 320x480, an ILI9488
controller on 4-wire SPI at 3.3 V. Its 50-contact FPC (0.5 mm pitch) leaves the panel's seat-1
edge, folds under it and plugs into J3. This sheet holds J3 and the way each panel pin is tied, the
panel's logic supply, the SPI series resistors and probe pads, and the backlight: a filtered PWM
reference and two linear current sinks.

**The panel's pins.** Wired as the datasheet says: IM2-IM0 high (4-wire 8-bit SPI), RD (a parallel
read strobe) high, the parallel bus DB0-DB17 and the RGB strobes (VSYNC, HSYNC, DOTCLK, DE) low,
and the touch pins open (this panel has no touch layer). The folded FPC's contact 1 lands at the
right of J3 while J3's pin 1 is at its left, so J3 pin j carries panel pin 51 - j. TE goes to
GPIO32.

**The backlight.** Six LEDs in parallel (3.0-3.2 V, 120 mA at most), common anode on VSYS, three on
each of two cathode pins. Each cathode pin has its own current sink:

```
 GPIO44 (BL_PWM) -- R60 100k --+-- BL_REF ----------------> +IN of both halves of U6
                               |   R61 3.9k and C50 1 uF to GND; TP10

 VSYS -- JP2 -- LED_A -- three LEDs -- LED_K1 -- Q5 drain
                                                 Q5 source (BL_S1) -- R52 2.2 ohm -- GND
 U6, one half:  output -- R54 330 ohm -- Q5 gate     (R56 100k from the gate to GND)
                -IN    -- R58 10k -- BL_S1           (C48 1 nF from the output to -IN)
```

The op-amp drives Q5's gate until the sense voltage on R52 equals BL_REF: 0.124 V at 100% duty is
56 mA a pin, 113 mA for all six LEDs. LED_K2 has the same sink built from the other half of U6, Q6,
R53, R55, R57, R59 and C49.

**Parts.**

| Parts | Role | Why this part or value |
|---|---|---|
| J3 (Hirose FH12-50S-0.5SH(55)) | 50-pin, 0.5 mm, bottom-contact ZIF connector | JLC places it, so no hand soldering per unit, and a cracked panel swaps in a minute (2026-09-24, when this panel replaced the solder-tail QD3525). It sits where the folded FPC's tip reaches. $0.81 at 10 |
| JP4; C29 (1 uF), C30 (100 nF) | the panel's logic supply (LCD_3V3) from 3V3; decoupling at the connector | JP4 is cut to run the board with the panel unpowered, or to read the panel's logic current |
| R28, R29 (33 ohm) | source terminations on the SPI clock and data (SCK, MOSI) | fitted next to the MCU, the source end of each line |
| TP11-TP14 | SCK, MOSI, CS, DC | four logic-analyser channels; the panel is the part most likely to need one (2026-10-19) |
| U6 (MCP6002); C51 (100 nF) | dual op-amp, one half per cathode pin; its supply capacitor | replaced the AP3156 charge pump, out of stock at JLC and LCSC on 2026-10-07; every sink part is Basic or Preferred. $0.24 at 10 |
| Q5, Q6 (AO3400A) | the pass transistors, run in their linear region | the pin review checked the simulation's AO3400 model against this part's datasheet (threshold, on-resistance) |
| R52, R53 (2.2 ohm 0603) | sense resistors | 2.2 ohm x 56 mA = 0.124 V; the sink needs only about 0.13 V across it, so the current holds from a full cell (or USB) down to the 3.5 V cutoff with the panel's 3.2 V maximum Vf |
| R54, R55 (330 ohm); R58, R59 (10k); C48, C49 (1 nF) | gate resistors; local feedback loop | the 330 ohm isolates the FET's gate capacitance from the op-amp, and the 10k with 1 nF closes a local loop: 72.5 degrees of phase margin or more at all 720 corners simulated |
| R56, R57 (100k) | gate pull-downs | the sinks are off whenever 3V3 is (U6 runs from 3V3) |
| R60 (100k), R61 (3.9k), C50 (1 uF) | turns GPIO44's PWM into the DC reference BL_REF | 3.3 V x 3.9 / 103.9 = 0.124 V at 100% duty. Time constant 3.75 ms: 0.3% ripple at 36.6 kHz. The LED current is DC, so there is no PWM in the LEDs for the amplifier to pick up |
| JP2 | VSYS to the LED anode | cut it and bridge it with an ammeter to read the LED current, or leave it open to run the panel dark |
| TP10 | BL_REF probe pad, by U6 | the backlight's set point (2026-10-19) |

The sinks were simulated in ngspice before layout with Microchip's MCP6001/2 and Alpha & Omega's
AO3400 models ([board/sim/backlight.py](../board/sim/backlight.py), results in
`backlight_results.txt`), over duty, VSYS 3.5-5.7 V, LED Vf, FPC inductance and doubled gate
capacitance. Their efficiency is that of the AP3156 in its 1x mode (Vf / VSYS), so battery life is
unchanged.

**Protection and failure cases.**

- **The backlight is off whenever 3V3 is**, though its anode sits on the always-live VSYS: U6 runs
  from 3V3 and R56/R57 hold the gates low. GPIO44 resets low and R61 holds BL_REF at 0, so it also
  stays off until firmware starts the PWM.
- **No spike at power-up, power-down or a VSYS hot plug with 3V3 off** (0.5-4.7 mA for about a
  microsecond, simulated). A VSYS step of 1.9 V in 2 us while lit (USB plugged in) briefly
  overshoots to 133 mA a pin for about 2 us.
- **The LED limit.** 100% duty gives 113 mA against the panel's 120 mA maximum.
- **Back-powering the panel.** With JP4 open the panel's inputs would feed it from the SPI lines,
  so firmware must hold them low.

**Bring-up hooks and firmware.** Bring-up step 3 covers the panel and its backlight. Firmware:

- The ILI9488 needs a new driver (18-bit colour over SPI, unlike the stripboard's ST7796).
  Pulse LCD_RST, send the init sequence over SPI0 at 10-20 MHz (the datasheet rates serial writes
  to 20 MHz), and read ID4 (0xD3) back on LCD_MISO at 5 MHz or less: expect 0x94 0x88.
- Over SPI the ILI9488 takes 18-bit colour only (COLMOD 0x66): three bytes a pixel, 461 kB a full
  frame, so redraw only what changes. Enable TE (TEON, 0x35) and start frames on GPIO32's pulse,
  about 60 Hz, to avoid tearing.
- Backlight: PWM10 A on GPIO44, for example 12 bits at 36.6 kHz. Current is proportional to duty,
  so apply a gamma curve; keep the lowest level at about 18% duty, where the op-amp's offset (up to
  4.5 mV against 22 mV) could make the two halves visibly unequal. A change settles in about 15 ms.
  Duty 0 is off.

**Open items and risks.**

- The panel has one maker, EastRising, selling direct. Another 3.5 in panel that fits the
  56.0 x 86.24 x 4.9 mm bay needs only case.scad's panel numbers, J3 and its pin map changed.
- The FPC's reach into J3 and the reversed pin order (contact 1 on J3's pin 50) are checked only
  on paper. Lighting a real panel through a bottom-contact breakout with design.py's mapping is
  item 4 of the validation plan's gate. The model errs long; if the tail bows, J3 moves 1.5-2 mm
  towards seat 1.
- An LED at the older datasheet's 3.4 V Vf would lose 6% of its current at 100% on a 3.5 V cell.
- Accuracy is set by the op-amp's offset: +-3.6% at full, +-20% a pin at 18% duty.
- The pin review notes that the panel's VCI and VDDI are rated to 3.3 V, and the ME6211's
  tolerance allows up to 3.366 V: minor, well inside the absolute maximum.
- The validation plan also proposed a spare filter footprint on LCD_RST; design.py has none.

### Audio (sheet Audio)

- PWM from GPIO31 (PWM7 B, 8 bits at 150 MHz: a 586 kHz carrier) through two RC
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

#### PWM audio and amplifier (sheet Audio)

**What it does.** A GPIO's PWM is filtered into an audio signal and played by a bridge-tied
amplifier into an 8 ohm speaker. The cost route was approved on 2026-09-24: PWM instead of an I2S
amplifier.

```
 GPIO8 (AUDIO_PWM) -- R32 1k --+-- R33 1k --+-- C39 220 nF -- R34 20k --+-- U7 -IN
                               |            |                           |
                          C37 4.7 nF   C38 4.7 nF                       +-- R35 20k -- U7 VO1
                             to GND       to GND

 U7 (FM8002A): +IN and BYPASS on AMP_BYP, C40 1 uF to GND
               VO1 (SPK_P) -- J4 pin 1, VO2 (SPK_N) -- J4 pin 2: the speaker, bridge-tied

 VSYS -- JP3 -- AMP_VDD (C34 10 uF, C35 100 nF) -- U7's supply
 AMP_VDD -- R36 100k -- SHUTDOWN (TP8) -- Q3 drain;  Q3 gate = AMP_EN (GPIO11), R37 100k to GND
```

**Parts.**

| Parts | Role | Why this part or value |
|---|---|---|
| R32, R33 (1k); C37, C38 (4.7 nF) | two RC low-pass stages | with the PWM at 8 bits (586 kHz) the carrier comes out about 49 dB down, and the audio band is -3 dB at about 14 kHz loaded, about what a 15 mm speaker reproduces. 4.7 nF is a JLC Basic part; 3.9 nF, which would give 17 kHz, is only Extended. Changed from 10 nF on 2026-09-25 |
| C39 (220 nF) | DC block | blocks the PWM's DC (it idles at half scale); with R34 it sets a 36 Hz low corner |
| R34, R35 (20k) | input and feedback resistors | gain 2 x R35 / R34 = 2, the datasheet's circuit; with the filter's 2k in front of R34 the chain's gain is about 1.8 |
| C40 (1 uF) | the bypass capacitor: a half-supply reference on +IN | the datasheet's value; charging it is why the amplifier takes about 100 ms to wake |
| U7 (FM8002A, SOIC-8) | class AB bridge-tied amplifier from VSYS, rated 1.1 W | the cheapest way to keep a speaker, approved over the class-D NS4168, which is 50 cents a unit more and cleaner. Class AB turns about half its battery draw into sound at high volume, against 80-90% for class D |
| JP3; C34 (10 uF), C35 (100 nF) | AMP_VDD from VSYS; supply decoupling | JP3 is cut to take a noisy or shorted amplifier off the board, or to measure its current |
| R36 (100k), Q3 (2N7002), R37 (100k) | shutdown control | R36 holds SHUTDOWN high, so the amplifier is off unless the MCU turns it on; AMP_EN high turns Q3 on and pulls SHUTDOWN low. R37 keeps it off while the MCU boots. Off: 4.2 uA. On: 4.4 mA, plus 37-57 uA through R36 |
| TP8 | AMP_SHDN probe pad | added 2026-10-05 |
| J4 (Molex PicoBlade 53261-0271) | the speaker's header | the speaker (INGHAi GSPK151035PN: 15 x 10 x 3.5 mm, 8 ohm, 0.5 W, 90 dB) comes with a 1.25 mm 2-pin plug, so nothing is soldered at assembly (Alex, 2026-10-20). It is J2's part, so it adds no part number at JLC |

The speaker lies face down on the tub floor beside the cell and plays out of the left wall's lower
edge (2026-10-20); until then it fired down through a floor grille that a table covers.

**Protection and failure cases.**

- **Off unless asked.** Through boot, a reset or a crash, R37 holds AMP_EN low and R36 keeps the
  amplifier in shutdown, drawing 4.2 uA from the cell.
- **Supply transients.** VSYS rings at a hot plug; R47 (the USB_Charger sheet) keeps it at about 5.7 V in
  simulation, under the FM8002A's 6 V maximum (0.3 V of margin).
- **Bridge-tied outputs.** Neither speaker pin may touch GND. The cell's plug would also fit J4 and
  put the cell across the amplifier's outputs, so J4 sits out of the cell plug's reach.
- **Heat.** The worst case, a continuous full-scale sine on USB, is about 0.54 W in the SOIC-8.
- **No backlight whine.** The backlight current is DC, so there is no PWM in the LEDs to couple into
  the audio.

**Bring-up hooks and firmware.** Bring-up step 4 plays a 1 kHz sine, checks that the carrier is gone
at C39, listens for hiss, measures the wake delay and checks U7's temperature after 10 minutes at
full scale on USB. Firmware:

- PWM4 A on GPIO8 at 8 bits (586 kHz). Not 10 bits: 146 kHz would come out only 26 dB down. Dither
  or noise-shape for more resolution.
- Idle the PWM at half scale so turning on doesn't pop. Keep AMP_EN high while the device is awake
  (about 4.4 mA), or raise it at least 150 ms before a sound.
- Full scale is about 0.55 W on USB and 0.4-0.5 W on the cell, and it clips below a 3.7 V cell:
  back the amplitude off there, and cap it on USB if U7 runs hot.
- A mixer, when written, lives in core so the browser sounds like the device (the zero-drift rule in CLAUDE.md).

**Open items and risks.**

- Nothing here has been measured. A sample speaker's plug must still be tried in a genuine
  53261-0271 header, and how loud it is out of the side wall is unknown.
- Full scale on USB (about 0.55 W) is a little above the speaker's 0.5 W rating in bom.md; the
  documents don't reconcile the two.
- The validation plan proposed an unfitted RC footprint on the speaker outputs; design.py has none.
- AMP_EN relies on R37's 100k against erratum E9 (the MCU sheet): safe while GPIO11 is an output.
- The sheet's own note still says "8 ohm 1 W speaker on the J4 pads"; J4 is a header and the
  speaker is rated 0.5 W.

### Membrane contacts (sheet Buttons)

- 16 x DSL_button_4ring (D-pads, A/B/X/Y) and 4 x DSL_button_3ring (Start, Select: the same comb
  without its outer ring, for the Start/Select membrane's smaller pills; ESTIMATE): two interleaved combs (0.15 mm lines, 0.20 mm gaps,
  checked by `hardware/membrane/tools/check_contact_fp.py`) under one 4.8 mm mask
  opening, ENIG. A carbon pill of 1.6 mm or more bridges them anywhere on the pad.
- Each comb leaves through a 0.2 mm stub to a via 3.3 mm from the contact's
  centre; nothing else (no vias, no pour) inside 2.9 mm.
- SW21 (seat 1's Start) is the power button: its combs are VSYS and PWR_BTN_RAW.

#### Membrane contacts (sheet Buttons)

**What it does.** 20 contacts under DS Lite silicone: per seat a D-pad, A, B, X, Y, Start and
Select. Each joins its own GPIO to GND when pressed, except seat 1's Start, which joins VSYS to
the power latch.

| Parts | Role | Footprint |
|---|---|---|
| SW1-SW8 | seat 1's Up, Down, Left, Right, A, B, X, Y | DSL_button_4ring |
| SW9-SW16 | seat 2's Up, Down, Left, Right, A, B, X, Y | DSL_button_4ring |
| SW20, SW22 | seat 1's and seat 2's Select | DSL_button_3ring |
| SW21 | seat 1's Start, and the power button: VSYS to PWR_BTN_RAW | DSL_button_3ring |
| SW23 | seat 2's Start | DSL_button_3ring |

**The contacts.** Each is two interleaved combs of bare copper with an ENIG finish (0.15 mm lines,
0.20 mm gaps) under one 4.8 mm mask opening; a carbon pill of 1.6 mm or more bridges them anywhere
on the pad. The 3-ring version is the same comb without its outer ring, for the Start/Select
membrane's smaller pills (an estimate). Each comb leaves through a 0.2 mm stub to a via 3.3 mm from
the contact's centre, with nothing else (no vias, no pour) inside 2.9 mm. The contacts are copper,
not parts: JLC places nothing here. The footprints are generated and checked by the tools in
[hardware/membrane/](../../membrane/README.md) (0.200 mm minimum gap, 1.60 mm bridging spot).

**Why it is built this way.**

- **One GPIO per contact, no matrix.** v1's button matrix ghosted presses across players; direct
  GPIO cannot (2026-08-19, kept here).
- **DS Lite silicone on bare contacts** (2026-09-21): half the footprint of SNES parts, sold as
  repair spares, one D-pad piece and one ABXY piece per seat. ENIG because the pills press straight
  on the copper.
- **Start and Select per seat, power on seat 1's Start** (Alex, 2026-10-19): it matches the DS Lite
  layout, and no game button doubles as power (the power button was seat 1's Y before). The
  contacts sit between the clusters near the case's end, under the DS Lite kit's Start/Select
  membrane.

**Protection and failure cases.** The power contact is the only one on VSYS. Its other side reaches
the MCU only through Q1, so no GPIO ever sees VSYS, and R13 (33k) keeps surface leakage across the
contact from lifting PWR_EN (the Power sheet). There are no ESD parts on the contact lines; ESD has not
been tested (design_review.md, "Not checked").

**Bring-up and firmware.** Bring-up step 2.1 prints all 20 buttons while each contact's combs are
shorted with tweezers. Firmware enables the internal pull-ups (the pins reset with pull-downs on);
a contact reads low when pressed, and seat 1's Start reads on GPIO3 (PWR_BTN_N), also low when
pressed. Tap versus 500 ms hold is the platform's job; what a press means is core's (the zero-drift rule in CLAUDE.md).
Start and Select have no game meaning in core yet.

**Open items and risks.**

- The validation plan's gate asks for pill resistance under 1 kohm on a matched ENIG coupon, and
  leakage above 100 Mohm on the power contact at 4.2 V (it names P1_Y, the power contact before
  2026-10-19).
- ENIG combs may drift over a product's life; carbon ink would last longer but needs a coarser
  pattern.
- The Start/Select pad is not measured, so the 3-ring footprint and its wells are estimates; the
  A/B/X/Y stems against the domes and the D-pad's pivot are open, and `check_buttons.py` fails until
  they settle (the Known gaps section below).

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
| Hot plug with no cell | a live source into a bare board rang VSYS to about 6.2 V in simulation (R41 in place) in the TP4056 design, where VSYS followed VBUS; the RT9013 is rated 6 V absolute, and with the BQ24073 VSYS is its regulated OUT (4.3-4.5 V) | bring-up step 5 scopes it; the worst case is an A-to-C cable from an always-on USB-A port (a USB-C source switches VBUS on only after it sees the board, and ramps it), so on the bench prefer a USB-C source or fit the cell first |
| Panel supply | one maker (EastRising), sold direct | it publishes its drawings; another panel needs case.scad's panel numbers, J3's place and its pin map changed (the 3.5in ER-TFT035IPS-6 to this 2.4in one, 2026-10-21, was that plus R61) |
| FPC fit in J3 | too short a fold can't reach the connector; too long bows under the panel | the model errs long (safe); if the tail bows, move J3 1.5-2 mm towards seat 1 (`conn_rear` about -0.6); check with a real panel and connector before the board order |
| DS Lite silicone fit | cap retention and feel depend on unmeasured heights | pads and caps ordered: try them in a printed lid before the board order |
| Snap fit | too tight cracks a hook, too loose rattles; FDM snap fits wear | tune on a test print; PETG over PLA |
| JLC rotation offsets | parts placed 90 or 180 degrees off | review JLC's placement preview; the CPL carries KiCad's angles |
| Thin lid over the battery connector (0.6 mm) | cracks in the FDM-printed lid | 100% infill there for now; a lower-profile cell connector (JST SH/GH) removes the pocket |

## Known gaps

- No firmware for the reference design (RP2354B pin map, ILI9488 driver, audio, power latch,
  charger status, battery cutoff).
- No sound anywhere yet: no mixer in core and no web audio. The options menu dropped its sound and
  turn-timer rows because nothing was behind them.
- No turn timer, so a stalled player can still hold up a game.
- Start and Select have no meaning in core; MENU acts as B.
- The device's Gen 3 draft gives no abilities, items or EV spreads.
- A team reveal screen and sprite compression for flash are not built.
- D-pad auto-repeat exists only in the web page's input code; the firmware does not repeat yet.
- Whether the firmware and its assets fit the RP2354B's 2 MB of flash has not been measured.
- No persistent battle log in flash for post-mortems (a long-standing to-do).
- The reference design has no LED: there is no visible error indicator beyond the screen, and a
  player's only reset is power-cycling (RESET sits under a pinhole).

## Unknown

- Battery life of the reference design: not measured or estimated in the repo.
- The finished unit's mass against the about 160 g target: not measured.
