# Requirements

What the handheld must do, as numbers a check can pass or fail. Each requirement has a
value and a tolerance, why it is that number, where the number comes from, how it is
proven (T test, A analysis, I inspection, D demonstration), the checks in
[verification.csv](verification.csv) that prove it, the phase by which it must be proven
first, and a status.

**Every requirement here is PROPOSED.** Claude seeded them on 2026-10-24 from
`hardware/ref_design/docs/validation_plan.md` (#92 and the methods it names), the datasheet
reviews in `hardware/ref_design/board/pin_review/`, `docs/design_review.md` and IEC 62368-1.
None is binding until Alex marks it APPROVED (or REJECTED, or DEFERRED with a reason).
`check_trace.py --gate` fails while any is PROPOSED.

How to read a block:

- Level: product (what a player or Alex sees), subsystem (a block of the design), or
  board (one board's make and test).
- Board: both, ref_design (`hardware/ref_design`, REF-A) or prod_board (Alex's
  `hardware/prod_board` and `hardware/prod_case`). A requirement for both needs evidence
  from each board separately.
- Source: a datasheet page (the review file that quotes it is named), a standard clause, a
  validation_plan method (#n), or Alex's use. "Claude's estimate" means nobody has set the
  number yet: Alex should.
- Phase: ORDER (before the 5-board order), EVT (the 5 boards), DVT (the revised spin), PVT
  (the 30-board run), SALE (before any unit is sold or given away).

The standards' clause numbers below are from memory of IEC 62368-1:2018, IEC 62133-2:2017,
USB 2.0, IEC 61000-4-2 and EN 55035. Check each against the purchased text before relying on it.

## Product level

### REQ-PRD-01: Twenty independent inputs
- Level: product
- Board: both
- Statement: The device shall read 20 momentary inputs, 10 per seat (up, down, left, right, A, B, X, Y, Start, Select), each on its own line, with any combination of up to 20 held at once read correctly.
- Rationale: two players press at the same time; a matrix would ghost.
- Source: Alex's use; README (Start and Select added 2026-10-19, a3158fc).
- Method: T
- Verified by: VER-56, VER-76
- Phase: EVT
- Status: PROPOSED
- Notes: prod_board has 16 contact footprints (no Start or Select) at 76955d2.

### REQ-PRD-02: One readable screen
- Level: product
- Board: both
- Statement: The 320 x 480 panel shall show a full-screen test image with 0 missing lines or columns and correct red, green and blue order, seen from either seat.
- Rationale: the one shared screen is the product.
- Source: Alex's use; ER-TFT035IPS-6 datasheet (pin_review/panel.json).
- Method: T
- Verified by: VER-60, VER-61
- Phase: EVT
- Status: PROPOSED

### REQ-PRD-03: Play time
- Level: product
- Board: both
- Statement: From charge-done, the device shall play for at least 4.0 h before the 3.50 V cutoff at 25 C, backlight at 50% duty and sound at -12 dBFS.
- Rationale: about 700 mAh against an estimated 110-140 mA (core up to 50 mA, panel logic 10 mA, backlight 56 mA at 50%, amplifier 4.4 mA plus sound).
- Source: Claude's estimate; Alex to set the target.
- Method: T
- Verified by: VER-101
- Phase: EVT
- Status: PROPOSED

### REQ-PRD-04: Charge time
- Level: product
- Board: both
- Statement: From the 3.50 V cutoff the cell shall reach charge-done (CHG_DONE_N low) within 3.0 h at 25 C from a source offering 1.5 A or more, device on the charging screen.
- Rationale: 333-364 mA constant current into about 700 mAh is about 1.7 h, plus the constant-voltage tail.
- Source: TP4056 datasheet p.9 PROG table (pin_review/u2.json); Claude's estimate for the tail.
- Method: T
- Verified by: VER-102
- Phase: EVT
- Status: PROPOSED

### REQ-PRD-05: Shelf life when off
- Level: product
- Board: both
- Statement: Switched off with a full cell and no USB, the board shall drain the cell by no more than 30 uA, so that the board alone takes 2 years or more to empty it.
- Rationale: 700 mAh / 30 uA is about 2.7 years; the cell's own self-discharge dominates.
- Source: design_review.md, Off state (18-22 uA computed); validation_plan #33.
- Method: T, A
- Verified by: VER-65, VER-103
- Phase: EVT
- Status: PROPOSED

### REQ-PRD-06: Size
- Level: product
- Board: ref_design
- Statement: The closed case shall measure at most 75.4 x 156.2 mm in plan and 13.4 mm thick at the screen, 11.1 mm at the button decks, each +0.3 mm for print tolerance.
- Rationale: the DS Lite feel the case was drawn for.
- Source: design_review.md (case 75.4 x 156.2 x 13.4 mm); case.scad.
- Method: I, A
- Verified by: VER-48, VER-20
- Phase: ORDER
- Status: PROPOSED
- Notes: ref_design only. prod_case's preliminary layout is 70.5 x 153 mm and about 19.1 mm thick; Alex sets his own number.

### REQ-PRD-07: Single-fault safety
- Level: product
- Board: both
- Statement: No single fault in the electronics (one open, short, adjacent-pin bridge, wrong part or wrong rotation) shall cause fire, cell venting or an accessible surface above 48 C, with 1 or more independent protection layers left working after the fault.
- Rationale: a Li-ion cell in a printed case held in the hands.
- Source: IEC 62368-1 Annex B.4 (single fault conditions); validation_plan #89, #151.
- Method: A, T
- Verified by: VER-98, VER-97
- Phase: ORDER
- Status: PROPOSED

### REQ-PRD-08: Recoverable firmware
- Level: product
- Board: both
- Statement: A unit with blank, corrupt or hung firmware shall enter the USB boot loader through the lid's pinholes with the case closed, with the cell flat and USB in, 2 out of 2 tries; with the case open SWD shall attach through J5 and through J7.
- Rationale: bring-up and field recovery must not need the case opened or a soldering iron.
- Source: validation_plan #182, #202; bringup.md.
- Method: D, T
- Verified by: VER-107, VER-58
- Phase: EVT
- Status: PROPOSED

### REQ-PRD-09: Cost
- Level: product
- Board: ref_design
- Statement: The all-in cost per unit at 10 units shall be $45 or less (board, assembly, shipping, panel, cell, speaker, silicone, caps, screws, filament).
- Rationale: bom.md puts it at about $40-42 at 10 (2026-09-24 prices).
- Source: docs/bom.md; Alex's use.
- Method: I
- Verified by: VER-117
- Phase: ORDER
- Status: PROPOSED

### REQ-PRD-10: Serviceable cell and panel
- Level: product
- Board: both
- Statement: Alex shall replace the cell or the panel with a screwdriver and no soldering in 10 min or less each.
- Rationale: cells age and glass breaks; J2 is a plug and J3 a ZIF so both swap.
- Source: validation_plan #194, #203 (EU Regulation 2023/1542 Art. 11 if sold in the EU).
- Method: D
- Verified by: VER-91
- Phase: EVT
- Status: PROPOSED

### REQ-PRD-11: Survives drops
- Level: product
- Board: both
- Statement: After 3 drops from 1.0 m onto hardwood (face, end, corner) the device shall still pass the self-test, with no crack that exposes the cell or a board.
- Rationale: a handheld is dropped.
- Source: IEC 62368-1 Annex T.7 (drop test, 1 m for hand-held equipment, to confirm); validation_plan #50.
- Method: T
- Verified by: VER-93
- Phase: DVT
- Status: PROPOSED

## Subsystem: cell and charging (BAT)

### REQ-BAT-01: Charge voltage
- Level: subsystem
- Board: both
- Statement: The charger shall stop the cell at 4.20 V +-1% (4.158-4.242 V at the cell terminals at charge-done) and never above the cell maker's maximum charge voltage.
- Rationale: overcharge is the main cell hazard.
- Source: TP4056 datasheet (4.2 V +-1%, pin_review/u2.json); IEC 62133-2 (upper limit charging voltage set by the cell maker).
- Method: T, A
- Verified by: VER-67, VER-14
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-02: Charge current
- Level: subsystem
- Board: both
- Statement: The constant-current charge shall measure 300-400 mA (designed 333-364 mA) and shall not exceed 0.5 C of the cell's rated capacity or the cell maker's standard charge current, whichever is lower.
- Rationale: 0.5 C is the usual standard rate for small pouch cells; the 403450's own limit is not known yet.
- Source: TP4056 datasheet p.9 (1100-1200 V / R5, pin_review/u2.json); validation_plan #28, #128.
- Method: T, I
- Verified by: VER-66, VER-37
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-03: Charge temperature window
- Level: subsystem
- Board: both
- Statement: Charging shall pause when the cell is above 45 C or below 0 C, each +-3 C, or inside the cell maker's charge range if that is narrower, and resume when back in range.
- Rationale: charging a Li-ion cell outside its range plates lithium (cold) or speeds failure (hot).
- Source: IEC 62133-2 (charging within the cell maker's temperature limits); TP4056 TEMP thresholds 45% and 80% of VBUS (pin_review/u2.json); validation_plan #30.
- Method: A, T
- Verified by: VER-32, VER-69
- Phase: EVT
- Status: PROPOSED
- Notes: the design's worst case is -3 to 49 C (design_review.md), so the top edge fails by 1 C as drawn: DF-05.

### REQ-BAT-04: Cell temperature during charge
- Level: subsystem
- Board: both
- Statement: The cell surface shall stay at or below 45 C through a full charge in the closed case at 25 C ambient, and at or below the cell maker's maximum charge temperature at 35 C ambient.
- Rationale: the cell lies 0.5 mm under the board in a sealed printed case.
- Source: validation_plan #31; IEC 62133-2.
- Method: T
- Verified by: VER-85
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-05: Cell polarity
- Level: subsystem
- Board: both
- Statement: 100% of cells shall be checked with a meter before first mating, red lead on J2 pin 1 (silk '+'), and the result recorded per lot.
- Rationale: PicoBlade has no battery polarity standard; a reversed cell destroys the TP4056 and shorts the cell into it.
- Source: pin_review/j2.json; assembly.md section 2.
- Method: I
- Verified by: VER-49, VER-54
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-06: Cell protection
- Level: subsystem
- Board: both
- Statement: Every cell shall carry a protection circuit with over-voltage at 4.35 V or less, under-voltage at 2.4 V or more and an over-current trip, each measured on 1 spare cell per lot.
- Rationale: the PCM is the independent layer behind every charge-path fault (REQ-BAT-09).
- Source: validation_plan #32; the cell's spec sheet once obtained (VER-37).
- Method: T, I
- Verified by: VER-88, VER-37
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-07: Room to swell
- Level: subsystem
- Board: ref_design
- Statement: The closed case shall leave 0.4 mm or more of free height (air, or foam that compresses to half its thickness) above the cell at worst-case tolerance.
- Rationale: pouch cells grow 8-10% in thickness over life; 0.4 mm for a 4.0 mm cell.
- Source: validation_plan #26.
- Method: A, I
- Verified by: VER-20, VER-48
- Phase: ORDER
- Status: PROPOSED
- Notes: case.scad has cell_gap = 0.5 mm, and assembly.md puts a 0.45 mm foam pad in it: DF-84.

### REQ-BAT-08: Nothing hard against the pouch
- Level: subsystem
- Board: ref_design
- Statement: No through-hole lead, part body or rib edge shall be within 0.5 mm of the cell's faces; only the foam pad touches it.
- Rationale: a pierced pouch shorts internally and can burn.
- Source: validation_plan #27; case.scad cell_gap 0.5 mm.
- Method: A, I
- Verified by: VER-06, VER-48, VER-109
- Phase: ORDER
- Status: PROPOSED

### REQ-BAT-09: Charge path single faults
- Level: subsystem
- Board: ref_design
- Statement: No single fault in U2, Q2, D5, D6, D7, R5, R6, R39, R40 or TH1 shall put more than 4.25 V or more than 1 C on the cell without at least 1 independent layer (the cell's PCM counts as one) still working, and each latent fault that removes a layer shall be found by a per-unit test.
- Rationale: a shorted D7 or Q2, or an open D6, charges the cell straight from VSYS with no regulation (validation_plan #89).
- Source: validation_plan #89; IEC 62368-1 Annex B.4; IEC 62133-2.
- Method: A, T
- Verified by: VER-98, VER-82, VER-88
- Phase: ORDER
- Status: PROPOSED

### REQ-BAT-10: Deep discharge
- Level: subsystem
- Board: both
- Statement: After the firmware shuts down at 3.50 V, the off-state drain shall keep the cell above its PCM's under-voltage trip for 2 months or more at 25 C.
- Rationale: a cell left flat in a drawer should still charge; between 3.50 V and 3.0 V there is only about 5-10% of the capacity (35-70 mAh), which 30 uA empties in 7-14 weeks.
- Source: Claude's estimate; validation_plan #33.
- Method: A, T
- Verified by: VER-103, VER-65
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-11: Transport documents
- Level: subsystem
- Board: both
- Statement: Before any unit with a cell is posted or sold, the UN38.3 test summary and the MSDS for the exact cell model shall be on file (2 documents).
- Rationale: carriers refuse lithium cells without them.
- Source: UN Manual of Tests and Criteria, Part III, 38.3; validation_plan #34.
- Method: I
- Verified by: VER-38
- Phase: SALE
- Status: PROPOSED

## Subsystem: power path (PWR)

### REQ-PWR-01: USB input range
- Level: subsystem
- Board: both
- Statement: The device shall run and charge from 4.40-5.50 V at the USB-C connector, both plug orientations, and shall not limit-cycle on a source that folds back at 500 mA.
- Rationale: USB 2.0 sources give 4.75-5.25 V; cables drop more.
- Source: USB 2.0 cl. 7.2.1; TP4056 VCC 4.0-8.0 V (pin_review/u2.json); validation_plan #51, #80.
- Method: T
- Verified by: VER-80, VER-55
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-02: Hot-plug peaks
- Level: subsystem
- Board: ref_design
- Statement: At a live plug-in from an always-on USB-A port with a 2 m cable and no cell, VBUS shall peak at 7.5 V or less and VSYS at 5.8 V or less.
- Rationale: TP4056 absolute maximum 8.0 V; FM8002A absolute maximum 6.0 V (the lowest on VSYS); ME6211 6.5 V.
- Source: TP4056 p.2 (pin_review/u2.json), FM8002A p.3 (pin_review/u7.json), ME6211 p.5 (pin_review/u3.json); design.py R47 comment.
- Method: A, T
- Verified by: VER-12, VER-71
- Phase: ORDER
- Status: PROPOSED

### REQ-PWR-03: LDO enable never above its input
- Level: subsystem
- Board: ref_design
- Statement: PWR_EN shall stay at or below VSYS + 0.3 V at every instant, including a live plug-in with the cell connected and the device off (10 plug-ins).
- Rationale: the ME6211's CE absolute maximum is VIN + 0.3 V; PWR_EN rises from VBUS ahead of VSYS.
- Source: ME6211 p.5 (pin_review/u3.json); design.py C47 comment.
- Method: T, A
- Verified by: VER-72, VER-12
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-04: 3V3 regulation
- Level: subsystem
- Board: both
- Statement: 3V3 shall stay at 3.30 V +-3% (3.20-3.40 V) for VSYS 3.60-5.50 V and loads of 0-150 mA.
- Rationale: RP2354B IOVDD and the panel's logic supply; the ME6211 is +-2%.
- Source: ME6211 (pin_review/u3.json); ER-TFT035IPS-6 VCI 2.5-3.3 V recommended (pin_review/panel.json: up to 3.366 V is 66 mV over, accepted as minor: DF-96).
- Method: T
- Verified by: VER-83, VER-55
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-05: 3V3 through the unplug
- Level: subsystem
- Board: ref_design
- Statement: Through the USB-to-cell hand-over at full load (backlight 100%, sound playing), 3V3 shall stay at or above 2.97 V for any cell at or above 3.50 V.
- Rationale: QSPI_IOVDD's minimum (the RP2354B's internal flash). A dip below it can corrupt a flash access; the RP2350 has no 3V3 supervisor.
- Source: design_review.md 'The unplug'; RP2350 datasheet Table 1688 (pin_review/u4.json).
- Method: A, T
- Verified by: VER-11, VER-70
- Phase: EVT
- Status: PROPOSED
- Notes: open question for Alex: VREG_AVDD and USB_OTP_VDD have a 3.135 V minimum (pin_review/u4.json pins 61 and 68). The design meets that only for cells above about 3.6 V during the 1.5-3.3 ms dip. DF-34.

### REQ-PWR-06: Hand-over time
- Level: subsystem
- Board: ref_design
- Statement: Q2's channel shall conduct within 5 ms of VBUS removal (designed 1.5-3.3 ms).
- Rationale: until then D7 carries the system at 0.34 V drop.
- Source: design_review.md; design.py R38 comment.
- Method: T, A
- Verified by: VER-70, VER-11
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-07: Off-state current
- Level: subsystem
- Board: both
- Statement: With the device off and no USB, the cell current shall be 30 uA or less at 25 C for a cell at 3.7-4.2 V.
- Rationale: designed 18-22 uA (divider, charger standby, D5 leakage, the amplifier's shutdown current).
- Source: design_review.md Off state; bringup.md step 5.1.
- Method: T
- Verified by: VER-65
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-08: Turn-on margin
- Level: subsystem
- Board: ref_design
- Statement: With 1 kohm in series with seat 1's Start contact and the cell at 3.50 V, pressing Start shall bring 3V3 to 3.20 V or more within 50 ms and PWR_BTN_N shall read low.
- Rationale: a worn or dirty pill adds resistance; the latch must still start near the cutoff.
- Source: validation_plan section 1 'Latch replica' and Fable's note 6; design.py R13 comment.
- Method: T, A
- Verified by: VER-78, VER-33
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-09: Turn-off after a reset or crash
- Level: subsystem
- Board: ref_design
- Statement: On the cell with no USB, after RESET, a watchdog reset or a hard fault, PWR_HOLD shall fall below 0.5 V within 10 ms and 3V3 below 0.5 V within 200 ms of Start being released.
- Rationale: a crashed unit must turn itself off, not drain the cell; RP2350-E9 can hold a released pad near 2 V.
- Source: RP2350 erratum E9 (pin_review/u4.json); design.py R42 comment; pinmap.md Power control.
- Method: T
- Verified by: VER-77, VER-73
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-10: No hang keeps it on
- Level: subsystem
- Board: both
- Statement: Without USB, no firmware hang shall keep the device on for more than 10 s; every image, the self-test included, shall enable the watchdog before any blocking init.
- Rationale: hardware alone cannot tell a hang with PWR_HOLD high from normal play.
- Source: validation_plan gate item 3 and Fable's note 1; pinmap.md.
- Method: I, T
- Verified by: VER-104, VER-105
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-11: No turn-on from leakage
- Level: subsystem
- Board: ref_design
- Statement: With 1 Mohm across the Start contact (VSYS to PWR_BTN_RAW) and the cell at 4.2 V, the device shall stay off (3V3 below 0.5 V) for 10 min; the design shall turn on only below about 56 kohm.
- Rationale: the Start comb carries VSYS all the time; sweat or flux can bridge it.
- Source: validation_plan #185 (written for P1_Y, now SW21); design.py R13 comment.
- Method: T, A
- Verified by: VER-79, VER-33
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-12: USB input current
- Level: subsystem
- Board: ref_design
- Statement: From a default-power USB source (CC at 0.25-0.61 V) the input current shall be 500 mA or less within 2 s of plug-in.
- Rationale: the TP4056 has no input limit; charge plus system can reach about 600 mA.
- Source: USB 2.0 cl. 7.2.1; pinmap.md Charger; validation_plan #94.
- Method: T
- Verified by: VER-81
- Phase: EVT
- Status: PROPOSED
- Notes: decision for Alex: USB 2.0 also caps an unconfigured device at 100 mA and a suspended one at 2.5 mA. This design charges regardless; either accept that or add a firmware rule.

### REQ-PWR-13: VBUS capacitance and inrush
- Level: subsystem
- Board: ref_design
- Statement: The board shall present 10 uF or less on VBUS and draw 50 uC or less at plug-in, or carry a written deviation.
- Rationale: C1 is 10 uF, and with no cell the VSYS capacitors (about 45 uF) charge through D5 at plug-in.
- Source: USB 2.0 cl. 7.2.4.1; validation_plan #125.
- Method: A
- Verified by: VER-99
- Phase: ORDER
- Status: PROPOSED

### REQ-PWR-14: Derating
- Level: subsystem
- Board: ref_design
- Statement: Every part's voltage, current and power stress in every mode (off, cell, USB, charging, hot plug, unplug) shall be 80% or less of its absolute maximum, 90% or less for transients under 10 us, or carry a waiver.
- Rationale: one consistent margin rule, so a reviewer can tell an intended stress from an oversight.
- Source: validation_plan #5, #123 (the 80/90% rule is Claude's proposal).
- Method: A
- Verified by: VER-31, VER-02
- Phase: ORDER
- Status: PROPOSED
- Notes: the FM8002A at 5.7 V of 6.0 V is 95%: it needs a fix or a waiver.

### REQ-PWR-15: 3V3 budget
- Level: subsystem
- Board: ref_design
- Statement: The 3V3 load shall be 150 mA or less and U3's dissipation 0.20 W or less at VSYS 5.2 V on USB.
- Rationale: the SOT-23's 300 mW allows about 158 mA at that input; the budget was never written down.
- Source: ME6211 p.5 (pin_review/u3.json: 3V3 budget UNKNOWN).
- Method: A, T
- Verified by: VER-31, VER-83
- Phase: ORDER
- Status: PROPOSED

## Subsystem: core (MCU)

### REQ-MCU-01: Boot from blank
- Level: subsystem
- Board: ref_design
- Statement: A blank board on USB with no cell shall enumerate as the RP2350 boot drive within 5 s.
- Rationale: proves chip, crystal, USB and 1V1 in one step.
- Source: bringup.md step 1.
- Method: T
- Verified by: VER-55
- Phase: EVT
- Status: PROPOSED

### REQ-MCU-02: USB full speed
- Level: subsystem
- Board: ref_design
- Statement: D+ and D- shall each pass a 27 ohm +-1% series resistor, stubs shall be 3.5 mm or shorter, and the device shall enumerate on 3 different hosts.
- Rationale: the RP2350 requires 27 ohm; long stubs reflect.
- Source: RP2350 datasheet Table 2 (pin_review/u4.json pins 66, 67); pin_review/j1.json.
- Method: A, T
- Verified by: VER-02, VER-108, VER-80
- Phase: ORDER
- Status: PROPOSED

### REQ-MCU-03: Crystal
- Level: subsystem
- Board: ref_design
- Statement: The 12.000 MHz crystal (+-30 ppm) shall start at every power-up across VSYS 3.5-5.5 V with a 10 pF load (15 pF + 15 pF in series plus about 3 pF stray).
- Rationale: USB and the boot loader need it.
- Source: ABM8-272-T3 drawing p.1 (pin_review/x1.json); Raspberry Pi hardware design guide p.14.
- Method: A, T
- Verified by: VER-02, VER-55
- Phase: EVT
- Status: PROPOSED

### REQ-MCU-04: Debug ports
- Level: subsystem
- Board: ref_design
- Statement: SWD shall attach through J5 and through J7, and the UART on J6 shall carry text at 115200 baud 8N1 in the Raspberry Pi debug connector pinout (pin 1 the board's RX).
- Rationale: bring-up and recovery.
- Source: RP-003139-SP (pin_review/j5.json, j6.json); the J6 swap found 2026-10-05.
- Method: T
- Verified by: VER-58, VER-02
- Phase: EVT
- Status: PROPOSED

### REQ-MCU-05: Revision strap
- Level: subsystem
- Board: ref_design
- Statement: GPIO25..22 shall read BOARD_REV (1 on this board) with the internal pull-ups on.
- Rationale: firmware tells board revisions apart.
- Source: design.py R48-R51; pin_review/u4.json pin 22 (reset state is pull-down).
- Method: T
- Verified by: VER-59
- Phase: EVT
- Status: PROPOSED

### REQ-MCU-06: GPIO functions
- Level: subsystem
- Board: ref_design
- Statement: Every GPIO shall be assigned only a function that pin has in the RP2350 function table, with 0 PWM-slice or peripheral conflicts (PWM4 A on GPIO8, PWM10 A on GPIO44, SPI0 on GPIO36-39, UART1 on GPIO20-21, ADC on GPIO45-47).
- Rationale: a pin that cannot do its job is a respin.
- Source: RP2350 datasheet Table 3 (pin_review/u4.json); validation_plan #55, #140.
- Method: A
- Verified by: VER-15
- Phase: ORDER
- Status: PROPOSED

### REQ-MCU-07: Pull-downs for E9
- Level: subsystem
- Board: ref_design
- Statement: Every GPIO that enables power or a load (PWR_HOLD, CHG_OFF, AMP_EN, BL_PWM) shall have an external pull-down of 8.2 kohm or less, unless the placed stepping is shown not to have erratum E9.
- Rationale: on A2 silicon a released pad with its input enabled can sit near 2 V.
- Source: RP2350 erratum E9 (pin_review/u4.json pins 9 and 55); validation_plan #41, #178, #179.
- Method: A, T
- Verified by: VER-02, VER-75
- Phase: ORDER
- Status: PROPOSED
- Notes: PWR_HOLD (R42 4.7k) and CHG_OFF (R44 4.7k) meet it; AMP_EN (R37 100k) and BL_PWM (103.9k) do not: DF-45, DF-47.

### REQ-MCU-08: Chip identity
- Level: subsystem
- Board: ref_design
- Statement: Each unit shall report an RP2354B with 2 MB of internal flash, and its chip ID and stepping shall be recorded at first boot (1 record per unit).
- Rationale: a substituted RP2350B has no flash; the stepping decides which errata apply.
- Source: validation_plan #179, #217.
- Method: T
- Verified by: VER-75
- Phase: EVT
- Status: PROPOSED

### REQ-MCU-09: Core supply
- Level: subsystem
- Board: ref_design
- Statement: The 1V1 rail shall stay inside the RP2350's DVDD range (Table 1688) including ripple at full CPU load, with the decoupling the RP2350 datasheet 6.3.8.1 recommends or a written reason for any difference.
- Rationale: the copied core is the most layout-sensitive part; the datasheet recommends a second 4.7 uF that design.py lacks.
- Source: RP2350 datasheet 6.3.8.1, Table 1688 (pin_review/u4.json pin 10).
- Method: A, T
- Verified by: VER-23, VER-84
- Phase: EVT
- Status: PROPOSED
- Notes: write the DVDD minimum and maximum into VER-84 from Table 1688 on first use.

## Subsystem: display (DSP)

### REQ-DSP-01: Panel interface
- Level: subsystem
- Board: ref_design
- Statement: The panel shall run 4-wire 8-bit SPI (IM2..0 = 111) and return 0x94 0x88 for ID4 (0xD3) at a read clock of 5 MHz or less.
- Rationale: proves the interface straps and the read path.
- Source: ER-TFT035IPS-6 datasheet section 4.1 (pin_review/panel.json); ILI9488 serial read cycle 150 ns minimum.
- Method: T
- Verified by: VER-43, VER-60
- Phase: ORDER
- Status: PROPOSED

### REQ-DSP-02: Write speed
- Level: subsystem
- Board: ref_design
- Statement: SPI writes at 20 MHz shall draw 10 full frames of a checker pattern with 0 corrupted pixels.
- Rationale: the datasheet's serial write rating; 461 kB a frame.
- Source: ILI9488 datasheet; bringup.md display step 4; validation_plan #16, #143.
- Method: T
- Verified by: VER-61
- Phase: EVT
- Status: PROPOSED

### REQ-DSP-03: Panel pin map
- Level: subsystem
- Board: ref_design
- Statement: J3 pin j shall carry panel pin 51 - j for all 50 contacts, so LED_A (panel pin 1) is on J3 pin 50.
- Rationale: the folded FPC lands its contact 1 at J3's far end; a reversed map puts VSYS on a logic pin.
- Source: design.py display section; pin_review/panel.json.
- Method: A, T
- Verified by: VER-02, VER-43
- Phase: ORDER
- Status: PROPOSED

### REQ-DSP-04: FPC fit
- Level: subsystem
- Board: ref_design
- Statement: The panel's FPC tail shall be 0.30 +-0.05 mm thick and 25.5 +-0.07 mm wide, and the folded FPC shall reach J3 with 0.5-3 mm of slack and no crease.
- Rationale: the FH12-50S's FPC specification; too short cannot mate, too long bows under the panel.
- Source: Hirose FH12 catalog p.2, p.11 (pin_review/j3.json); validation_plan #215, #160.
- Method: I
- Verified by: VER-44, VER-106
- Phase: ORDER
- Status: PROPOSED

### REQ-DSP-05: Backlight current
- Level: subsystem
- Board: ref_design
- Statement: At 100% duty the six LEDs shall draw 113 mA +-5% in total (56 mA +-5% a cathode pin, the two pins within 5% of each other), and never more than 120 mA DC at any duty for VSYS 3.5-5.7 V.
- Rationale: the panel's 120 mA maximum; even brightness across the halves.
- Source: ER-TFT035IPS-6 section 4.4 (pin_review/panel.json); design.py backlight section.
- Method: T, A
- Verified by: VER-62, VER-05
- Phase: EVT
- Status: PROPOSED

### REQ-DSP-06: Backlight transient
- Level: subsystem
- Board: ref_design
- Statement: The LED current shall exceed 120 mA in total for 5 us or less at any event, a USB plug-in while lit included.
- Rationale: simulation shows 133 mA a pin for about 2 us on a 1.9 V VSYS step; the panel publishes no pulse rating.
- Source: board/sim/backlight_results.txt.
- Method: A, T
- Verified by: VER-05, VER-62
- Phase: ORDER
- Status: PROPOSED

### REQ-DSP-07: Backlight headroom
- Level: subsystem
- Board: ref_design
- Statement: At 100% duty and VSYS 3.5 V with LED Vf up to 3.2 V, the LED current shall drop by 5% or less from its value at 4.2 V.
- Rationale: brightness should not sag until the cutoff.
- Source: board/sim/backlight_results.txt; design_review.md Backlight.
- Method: A, T
- Verified by: VER-05, VER-62
- Phase: ORDER
- Status: PROPOSED

### REQ-DSP-08: Backlight loop stability
- Level: subsystem
- Board: ref_design
- Statement: Each sink's loop shall have 45 degrees of phase margin or more at every simulated corner, and an 18% to 95% duty step shall show no ringing on the scope.
- Rationale: an op-amp driving a FET gate can oscillate.
- Source: board/sim/backlight.py (72.5 degrees or more over 720 corners).
- Method: A, T
- Verified by: VER-05, VER-62
- Phase: ORDER
- Status: PROPOSED

### REQ-DSP-09: Backlight off when 3V3 is off
- Level: subsystem
- Board: ref_design
- Statement: With 3V3 off, a VSYS hot plug shall produce less than 5 mA of LED current for less than 10 us.
- Rationale: no flash at plug-in while the device is off.
- Source: board/sim/backlight_results.txt (4.7 mA peak).
- Method: A
- Verified by: VER-05
- Phase: ORDER
- Status: PROPOSED

### REQ-DSP-10: Tearing-effect signal
- Level: subsystem
- Board: ref_design
- Statement: With TEON set, GPIO32 shall see one TE pulse per frame at 60 Hz +-10%.
- Rationale: firmware starts frames on it to avoid tearing.
- Source: bringup.md display step 6.
- Method: T
- Verified by: VER-63
- Phase: EVT
- Status: PROPOSED

## Subsystem: audio (AUD)

### REQ-AUD-01: Loudness
- Level: subsystem
- Board: ref_design
- Statement: A full-scale 1 kHz sine shall deliver 0.35 W or more into the 8 ohm speaker (4.7 V peak-to-peak across it or more) on a 3.8 V cell.
- Rationale: designed 0.4-0.5 W on the cell, 0.55 W on USB; it clips below a 3.7 V cell.
- Source: design_review.md Audio; pin_review/u7.json.
- Method: T
- Verified by: VER-64
- Phase: EVT
- Status: PROPOSED

### REQ-AUD-02: Carrier rejection
- Level: subsystem
- Board: ref_design
- Statement: The 586 kHz PWM carrier at the speaker shall be 40 dB or more below the full-scale 1 kHz level.
- Rationale: designed 49 dB with 8-bit PWM; 10-bit would only reach 26 dB.
- Source: design_review.md Audio; ad291a8.
- Method: T, A
- Verified by: VER-64, VER-100
- Phase: EVT
- Status: PROPOSED

### REQ-AUD-03: Bandwidth
- Level: subsystem
- Board: ref_design
- Statement: The audio chain shall be -3 dB at 10 kHz or higher and at 50 Hz or lower.
- Rationale: designed 14 kHz and 36 Hz; about what a 15 mm speaker reproduces.
- Source: design_review.md Audio.
- Method: A, T
- Verified by: VER-100, VER-64
- Phase: EVT
- Status: PROPOSED

### REQ-AUD-04: Amplifier heat
- Level: subsystem
- Board: ref_design
- Statement: After 10 min of a continuous full-scale 1 kHz tone on USB at 25 C, U7's top shall be 85 C or less.
- Rationale: about 0.54 W in an SOIC-8; the case skin limit is REQ-THM-01.
- Source: design_review.md Audio; bringup.md step 4.
- Method: T
- Verified by: VER-64
- Phase: EVT
- Status: PROPOSED

### REQ-AUD-05: Amplifier off and wake
- Level: subsystem
- Board: ref_design
- Statement: With AMP_EN low the amplifier shall draw 10 uA or less and be silent; with AMP_EN high it shall reach full output within 150 ms.
- Rationale: 4.2 uA shutdown typical; about 100 ms to charge the bypass capacitor.
- Source: FM8002A datasheet p.3 (pin_review/u7.json); pinmap.md Audio.
- Method: T
- Verified by: VER-64
- Phase: EVT
- Status: PROPOSED

## Subsystem: buttons (BTN)

### REQ-BTN-01: Contact closes
- Level: subsystem
- Board: both
- Statement: A DS Lite pill pressed on a contact shall measure under 1 kohm on ENIG, and each input shall still read pressed up to 10 kohm of pill resistance.
- Rationale: carbon pill on gold; 10 kohm is the margin against the RP2350's internal pull-up.
- Source: validation_plan gate item 5, #42, #137.
- Method: T
- Verified by: VER-45, VER-56
- Phase: ORDER
- Status: PROPOSED

### REQ-BTN-02: Contact stays open
- Level: subsystem
- Board: both
- Statement: Leakage between a contact's two combs shall be 100 Mohm or more at 4.2 V, clean and after 24 h at high humidity.
- Rationale: on SW21 the comb carries VSYS, so leakage turns the device on (REQ-PWR-11); on the others it reads as a press.
- Source: validation_plan gate item 5, #185.
- Method: T
- Verified by: VER-46, VER-79
- Phase: ORDER
- Status: PROPOSED

### REQ-BTN-03: Comb geometry
- Level: subsystem
- Board: both
- Statement: The combs shall keep a copper gap of 0.20 mm or more, any pill spot of 1.60 mm or more shall bridge them, and one 4.8 mm mask opening shall uncover all comb copper with no paste or silk.
- Rationale: the DS Lite's measured comb; 0.20 mm sits above JLC's minimum spacing.
- Source: hardware/membrane/README.md; check_contact_fp.py.
- Method: A
- Verified by: VER-08, VER-09, VER-10, VER-25
- Phase: ORDER
- Status: PROPOSED

### REQ-BTN-04: Keep-out under the pill
- Level: subsystem
- Board: both
- Statement: No via, pour or other copper shall lie within 2.9 mm of a contact's centre except its own two 0.2 mm stubs.
- Rationale: a tented via under a pill lifts it.
- Source: design_review.md Membrane contacts.
- Method: A
- Verified by: VER-25
- Phase: ORDER
- Status: PROPOSED

### REQ-BTN-05: Feel
- Level: subsystem
- Board: both
- Statement: Each button's actuation force shall be within +-30% of a genuine DS Lite's, and its travel within +-0.2 mm, measured the same way.
- Rationale: the point of DS Lite parts is the DS Lite feel; a relative limit avoids inventing an absolute number.
- Source: validation_plan #43; case/DSLITE.md.
- Method: T
- Verified by: VER-50
- Phase: ORDER
- Status: PROPOSED

### REQ-BTN-06: Pill registration
- Level: subsystem
- Board: both
- Statement: Each pill's centre shall land within 0.9 mm of its comb's centre at worst-case tolerance.
- Rationale: (3.5 mm pill - 1.6 mm bridging spot) / 2 is about 0.95 mm.
- Source: validation_plan #197.
- Method: A, I
- Verified by: VER-20, VER-51
- Phase: ORDER
- Status: PROPOSED

### REQ-BTN-07: Button life
- Level: subsystem
- Board: both
- Statement: Each contact shall keep its closed resistance under 1 kohm for 100,000 presses.
- Rationale: Claude's estimate for a toy used daily for a few years.
- Source: Claude's estimate; validation_plan #159.
- Method: T
- Verified by: VER-94
- Phase: DVT
- Status: PROPOSED

## Subsystem: mechanics (MEC)

### REQ-MEC-01: Heights inside the case
- Level: subsystem
- Board: ref_design
- Statement: Every part on the board shall clear its local case limit by 0.2 mm or more, using heights from each part's drawing or a measurement, with 0 parts on guessed heights.
- Rationale: check_fit guesses package heights today (DF-91).
- Source: validation_plan #96, #130, gate item 7.
- Method: A
- Verified by: VER-06, VER-112
- Phase: ORDER
- Status: PROPOSED

### REQ-MEC-02: Stack closes
- Level: subsystem
- Board: ref_design
- Statement: The Z stack (floor, cell, gap, board, parts, panel, tape, lid) shall close with 0.2 mm or more of margin at worst-case tolerance including 0.4 mm of cell swelling.
- Rationale: nominal fits fail at tolerance.
- Source: validation_plan #25, #132, gate item 7.
- Method: A
- Verified by: VER-20
- Phase: ORDER
- Status: PROPOSED

### REQ-MEC-03: Buttons work under the shell
- Level: subsystem
- Board: ref_design
- Statement: Every cap shall sit and press its full dome travel (the D-pad rocked in 8 directions) with 0 contact with the shell, and at rest touch only its dome.
- Rationale: a cap that drags or is preloaded feels wrong or sticks.
- Source: case/check_buttons.py.
- Method: A, D
- Verified by: VER-07, VER-48
- Phase: ORDER
- Status: PROPOSED

### REQ-MEC-04: Assembly keyed
- Level: subsystem
- Board: ref_design
- Statement: The board shall fit the tub in 1 orientation only, and each A/B/X/Y cap in 1 hole and 1 rotation only.
- Rationale: no wrong assembly by hand.
- Source: assembly.md sections 3 and 5.
- Method: I
- Verified by: VER-48
- Phase: ORDER
- Status: PROPOSED

### REQ-MEC-05: Opening and closing
- Level: subsystem
- Board: ref_design
- Statement: The lid shall survive 20 open/close cycles with its six M2 screws and four hooks, with 0 cracked hooks and 0 stripped bosses.
- Rationale: the first PLA print's hooks snapped (DEF-15); service needs opening.
- Source: validation_plan #194; assembly.md step 8.
- Method: T
- Verified by: VER-92
- Phase: DVT
- Status: PROPOSED

### REQ-MEC-06: Parts stay put
- Level: subsystem
- Board: ref_design
- Statement: After the drop test the cell shall have moved 1 mm or less, the cell plug and the panel FPC shall still be seated, and the panel shall be uncracked.
- Rationale: a loose cell or plug in a dropped handheld is a hazard and a fault.
- Source: validation_plan #183.
- Method: T
- Verified by: VER-93
- Phase: DVT
- Status: PROPOSED

### REQ-MEC-07: USB-C plugs seat
- Level: subsystem
- Board: both
- Statement: 5 different USB-C cables shall seat fully with the case closed.
- Rationale: overmolds vary; the opening was widened once already (c35514f).
- Source: validation_plan #51.
- Method: T
- Verified by: VER-80
- Phase: EVT
- Status: PROPOSED

### REQ-MEC-08: No unintended openings
- Level: subsystem
- Board: ref_design
- Statement: The printed lid and tub shall have 0 through-openings other than the listed ones (USB-C, pinholes, cap holes, speaker grille, screw holes).
- Rationale: screw pilots and a notch once came through the face (DEF-19).
- Source: 177123c.
- Method: A, I
- Verified by: VER-113, VER-48
- Phase: ORDER
- Status: PROPOSED

## Subsystem: thermal (THM)

### REQ-THM-01: Touch temperature
- Level: subsystem
- Board: both
- Statement: Every outer surface a player holds shall stay at or below 43 C (target) at 25 C ambient, and the measured rise plus the rated maximum ambient shall stay at or below 48 C, while charging and playing with the backlight at 100% and audio at full scale.
- Rationale: IEC 62368-1's TS1 limit for plastic held 10 min to 8 h is 48 C (43 C beyond 8 h, to confirm); a handheld is held for hours.
- Source: IEC 62368-1 cl. 9.3, Table 38; validation_plan #93.
- Method: T
- Verified by: VER-86
- Phase: EVT
- Status: PROPOSED

### REQ-THM-02: Thermistor tracks the cell
- Level: subsystem
- Board: ref_design
- Statement: During a full charge in the closed case, TH1's reading shall stay within 5 C of the cell's surface.
- Rationale: TH1 sits on the board over the cell, not on it; the window in REQ-BAT-03 is only as good as this.
- Source: design_review.md Open risks; validation_plan #30.
- Method: T
- Verified by: VER-87
- Phase: EVT
- Status: PROPOSED

### REQ-THM-03: No pause at room temperature
- Level: subsystem
- Board: ref_design
- Statement: At 25 C ambient a full charge in the closed case shall complete with 0 temperature pauses (TEMP stays at 0.50 x VBUS or above).
- Rationale: U2's heat must not reach TH1.
- Source: bringup.md step 5.4.
- Method: T
- Verified by: VER-68
- Phase: EVT
- Status: PROPOSED

## Subsystem: ESD and EMC (EMC)

### REQ-EMC-01: ESD
- Level: subsystem
- Board: both
- Statement: The device shall survive +-4 kV contact discharge on the USB-C shell and +-8 kV air on seams and button gaps with no damage, recovering by itself or by a power cycle.
- Rationale: consumer ESD level; hands touch it all day.
- Source: IEC 61000-4-2; EN 55035 (Table 1, ESD); validation_plan #163.
- Method: T
- Verified by: VER-95
- Phase: DVT
- Status: PROPOSED

### REQ-EMC-02: Emissions
- Level: subsystem
- Board: both
- Statement: Before any sale, radiated and conducted emissions shall pass FCC Part 15 Class B / CISPR 32 Class B with 3 dB of margin; in DVT a near-field pre-scan shall name the 5 largest peaks and their sources.
- Rationale: an unintentional radiator needs it to be sold.
- Source: FCC 47 CFR Part 15 Subpart B; CISPR 32; validation_plan #67, #162.
- Method: T
- Verified by: VER-96, VER-118
- Phase: DVT
- Status: PROPOSED

## Board: manufacture (FAB)

### REQ-FAB-01: Design round trip
- Level: board
- Board: ref_design
- Statement: KiCad's netlist shall match design.py with 0 differences, ERC shall report 0 errors, and DRC 0 errors, 0 unconnected and 0 parity issues.
- Rationale: the generated KiCad files must say what design.py means.
- Source: validation_plan #75, #76, #173.
- Method: A
- Verified by: VER-01, VER-03, VER-04
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-02: Pins against datasheets
- Level: board
- Board: ref_design
- Statement: Every assembled part's every pin shall be reviewed against its current datasheet with a page citation, with 0 unresolved issues, 0 unknown ratings and 0 open questions.
- Rationale: ERC and parity trust design.py; only the datasheet can say a pin is wrong.
- Source: validation_plan #1, #113, gate item 1.
- Method: A, I
- Verified by: VER-02, VER-36, VER-35
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-03: JLC capability
- Level: board
- Board: ref_design
- Statement: The board shall pass DRC against JLC's published 2-layer rules (0.127 mm track and space, the copied core's 0.10 mm confirmed by JLC or widened; copper 0.30 mm or more from the edge; annular ring 0.13 mm or more) with 0 errors.
- Rationale: KiCad's defaults do not encode JLC's limits.
- Source: validation_plan #10, #118, #120; JLC capabilities page on order day.
- Method: A, I
- Verified by: VER-17, VER-18, VER-28
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-04: Holes
- Level: board
- Board: ref_design
- Statement: Every drilled hole shall be 0.30 mm or larger.
- Rationale: smaller holes cost about $40 an order at JLC (a fee, a 4-wire test, TG155).
- Source: design_review.md Board; assembly.md section 1.
- Method: A
- Verified by: VER-114
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-05: Order options
- Level: board
- Board: both
- Statement: The order shall be 2 layers, 1.6 mm, ENIG, with every option recorded in the release record (1 record per order).
- Rationale: the pills press on bare combs, so the finish must be gold.
- Source: assembly.md section 1; validation_plan #189.
- Method: I
- Verified by: VER-42
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-06: Part identity
- Level: board
- Board: both
- Statement: Every LCSC number shall resolve to the MPN, package, pin count and rating design.py intends, with 0 substitutions on the order.
- Rationale: the right family in the wrong variant passes every other check.
- Source: validation_plan #2, #122.
- Method: A, I
- Verified by: VER-14, VER-28, VER-41
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-07: Orientation
- Level: board
- Board: both
- Statement: Every polarised part's pin 1 in JLC's placement preview shall match the board's pin 1, with 0 unexplained differences, U1 and D7 included.
- Rationale: KiCad's footprint zero differs from JLC's for some parts.
- Source: validation_plan #9, #116; 1fd0956 (U1's JLC footprint).
- Method: I, A
- Verified by: VER-28, VER-29
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-08: Reproducible outputs
- Level: board
- Board: ref_design
- Statement: The gerbers, drill, BOM and CPL shall come from the released commit, two runs of fab.py shall give identical copper and drill files, and the ordered zip's sha256 shall be in the release record.
- Rationale: the board ordered must be the board checked.
- Source: validation_plan #12, #88, #121, gate item 11.
- Method: A, I
- Verified by: VER-24, VER-42
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-09: Independent connectivity
- Level: board
- Board: both
- Statement: JLC shall e-test 100% of the boards against an IPC-D-356 netlist exported from the ordered board.
- Rationale: a check from outside KiCad's own reading.
- Source: validation_plan #103.
- Method: I
- Verified by: VER-26
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-10: Stock
- Level: board
- Board: both
- Statement: On order day every BOM line shall have stock of 1.1 x the order quantity plus attrition, or be pre-ordered or consigned; RP2354B and L2 shall never be substituted.
- Rationale: RP2354B and L2 were out at JLC and LCSC on 2026-10-21; the AP3156 and a ProTek SRV05-4 ran out before.
- Source: DEF-12, DEF-13, DEF-29.
- Method: I
- Verified by: VER-41
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-11: Flatness
- Level: board
- Board: both
- Statement: Each board's bow and twist shall be 0.75% or less (1.1 mm over 151 mm).
- Rationale: the case clamps the board flat; a bowed board cracks MLCCs.
- Source: IPC-6012 Class 2 (bow and twist for SMT boards); validation_plan #78.
- Method: T
- Verified by: VER-90
- Phase: EVT
- Status: PROPOSED

### REQ-FAB-12: Workmanship
- Level: board
- Board: both
- Statement: Every assembled board shall meet IPC-A-610 Class 2 under the microscope, with 0 bridges, tombstones, lifted leads or missing parts.
- Rationale: Economic PCBA has no X-ray; the loupe is the inspection.
- Source: IPC-A-610 Class 2; validation_plan #53, #157.
- Method: I
- Verified by: VER-89
- Phase: EVT
- Status: PROPOSED

## Board: hand assembly (ASM)

### REQ-ASM-01: Hand assembly
- Level: board
- Board: both
- Statement: Alex shall assemble 1 unit from assembly.md in 30 min or less with an iron, a screwdriver and a knife, on an ESD mat with a strap, damaging 0 parts.
- Rationale: 30 units by hand.
- Source: validation_plan #86, #161.
- Method: D
- Verified by: VER-91
- Phase: EVT
- Status: PROPOSED

### REQ-ASM-02: Panel seated
- Level: board
- Board: ref_design
- Statement: The panel's FPC shall be fully inserted, square and latched, and pass the display test after 10 lid closures.
- Rationale: a half-seated FPC gives intermittent lines.
- Source: assembly.md step 4; validation_plan #160.
- Method: I, T
- Verified by: VER-106
- Phase: EVT
- Status: PROPOSED

## Board: test and traceability (TST)

### REQ-TST-01: Test access
- Level: board
- Board: ref_design
- Statement: Every measurement in bringup.md shall have a probe point (a TP, a jumper pad or a named part pad), with 0 measurements that need a lifted part.
- Rationale: bring-up must not need rework to measure.
- Source: validation_plan #21, #155.
- Method: I
- Verified by: VER-115
- Phase: ORDER
- Status: PROPOSED

### REQ-TST-02: Self-test
- Level: board
- Board: both
- Statement: A self-test image shall check the 20 buttons, GPIO neighbour bridges, panel ID and fills, backlight, an audio tone, the ADC, the charger states and the revision strap, print PASS or FAIL per item, take 5 min or less a unit, and write 0 bytes to OTP.
- Rationale: 30 units need a repeatable test; OTP writes can brick a unit.
- Source: validation_plan #57, #100, #174, #202.
- Method: T
- Verified by: VER-76
- Phase: EVT
- Status: PROPOSED

### REQ-TST-03: Traceability
- Level: board
- Board: both
- Statement: Every unit shall carry a serial number linked in 1 record to its chip ID, cell lot, test log and release.
- Rationale: a field fault must lead back to its lot and its tests.
- Source: validation_plan #195.
- Method: I
- Verified by: VER-110, VER-75
- Phase: PVT
- Status: PROPOSED

### REQ-TST-04: First article
- Level: board
- Board: both
- Statement: The first unit of each lot shall pass a first article inspection covering 100% of the BOM lines (present, marked as designed, oriented) plus the key dimensions and measurements, before any other unit of the lot is powered.
- Rationale: one bad reel or rotation affects the whole lot.
- Source: AS9102 (first article inspection).
- Method: I
- Verified by: VER-109
- Phase: EVT
- Status: PROPOSED

### REQ-TST-05: Yield
- Level: board
- Board: both
- Statement: The PVT lot's first-pass yield on the self-test shall be 90% or more, with every failure logged as a defect.
- Rationale: below that the process, not the unit, is the problem.
- Source: Claude's estimate.
- Method: I
- Verified by: VER-111
- Phase: PVT
- Status: PROPOSED

## Board: prod_board (PBD)

Alex's board supplies its own evidence. These cover what it has today (16 contacts placed
from prod_case, an empty schematic); the product-level requirements above apply to it as
it grows.

### REQ-PBD-01: Contacts where the case says
- Level: board
- Board: prod_board
- Statement: Each of the 16 contact footprints shall sit within 0.05 mm of its position in prod_case/layout.scad.
- Rationale: the case is the source of truth for prod_board.
- Source: hardware/prod_case/README.md; prod_board/tools/place_buttons.py.
- Method: A
- Verified by: VER-119
- Phase: ORDER
- Status: PROPOSED

### REQ-PBD-02: Contact footprint
- Level: board
- Board: prod_board
- Statement: The DSL_button_4ring footprint in prod.pretty shall keep a 0.20 mm or larger copper gap and a 1.60 mm or smaller bridging spot.
- Rationale: the same comb as ref_design.
- Source: hardware/membrane/README.md.
- Method: A
- Verified by: VER-10
- Phase: ORDER
- Status: PROPOSED

### REQ-PBD-03: Design checks
- Level: board
- Board: prod_board
- Statement: Before an order, prod_board's ERC shall report 0 errors and its DRC 0 errors, 0 unconnected and 0 parity issues, and the board shall fit prod_case with 0 collisions.
- Rationale: the same gate ref_design passes.
- Source: this register.
- Method: A
- Verified by: VER-120, VER-121, VER-122
- Phase: ORDER
- Status: PROPOSED
