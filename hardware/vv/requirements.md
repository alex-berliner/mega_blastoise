# Requirements

What the handheld must do, as numbers a check can pass or fail. Each requirement has a
value and a tolerance, why it is that number, where the number comes from, how it is
proven (T test, A analysis, I inspection, D demonstration), the checks in
[verification.csv](verification.csv) that prove it, the phase by which it must be proven
first, and a status.

**Every requirement here is PROPOSED.** Claude seeded them on 2026-10-24 from
`hardware/ref_design/docs/validation_plan.md` (archived), the datasheet reviews in
`hardware/ref_design/board/pin_review/`, `docs/design_review.md` and IEC 62368-1, and revised
them the same day from the two-model review (hardware/vv/PLAN.md). None is binding until
Alex decides it in [releases/APPROVALS.md](releases/APPROVALS.md): APPROVED, REJECTED, or
DEFERRED with a reason. A Status here that has no matching line he committed there is a
structure error, and a statement edited after he approved it needs his approval again.
`check_trace.py --gate` fails while any requirement in scope is PROPOSED.

How to read a block:

- Level: product (what a player or Alex sees), subsystem (a block of the design), or
  board (one board's make and test).
- Board: both, ref_design (`hardware/ref_design`, REF-A) or prod_board (Alex's
  `hardware/prod_board` and `hardware/prod_case`).
  A product requirement that names one board's parts or numbers is split: the parent
  (Board: both) states the product need without them, and each board gets its own
  allocation, `REQ-X.R` for ref_design and `REQ-X.P` for prod_board. Claude writes the .R
  ones. Alex writes, or opts into, the .P ones for his board. Until he does,
  `--gate --board prod_board` evaluates only the requirements marked prod_board
  (REQ-PBD-* and any .P), and the ref_design gate evaluates ref_design and both.
- Source: a datasheet page (the review file that quotes it is named), a standard clause, a
  validation_plan method (#n), or Alex's use. "Claude's estimate" means nobody has set the
  number yet: Alex should.
- Phase: ORDER (before the 5-board order: what the copper and the BOM freeze), CASE (before
  the printed case is released for the EVT units), EVT (the 5 boards), DVT (the revised
  spin), PVT (the 30-board run), SALE (before any unit is sold or given away).
- Applies when (optional, for a .P allocation): the condition on prod_board's design under
  which it applies, for example "its schematic holds a charger".

The standards' clause numbers below are from memory of IEC 62368-1:2018, IEC 62133-2:2017,
USB 2.0, IEC 61000-4-2 and EN 55035. Check each against the purchased text before relying on it.

## Product level

### REQ-PRD-01: Independent inputs
- Level: product
- Board: both
- Statement: The device shall read at least 8 momentary inputs per seat (D-pad and A, B, X, Y), each on its own line, with any combination held at once read correctly.
- Rationale: two players press at the same time; a matrix would ghost.
- Source: Alex's use.
- Method: T
- Verified by: allocated: REQ-PRD-01.R
- Phase: EVT
- Status: PROPOSED
- Notes: prod_board has 16 contacts and no Start or Select at 76955d2; its .P allocation is Alex's.

### REQ-PRD-01.R: Twenty independent inputs (ref_design)
- Level: product
- Board: ref_design
- Statement: The device shall read 20 momentary inputs, 10 per seat (up, down, left, right, A, B, X, Y, Start, Select), each on its own line, with any combination of up to 20 held at once read correctly.
- Rationale: Start and Select added 2026-10-19 (a3158fc); seat 1's Start is also the power button.
- Source: Alex's use; README.
- Method: T
- Verified by: VER-56, VER-76
- Phase: EVT
- Status: PROPOSED

### REQ-PRD-02: One readable screen
- Level: product
- Board: both
- Statement: The one shared screen shall show a full-screen test image with 0 missing lines or columns and correct red, green and blue order, seen from either seat.
- Rationale: the one shared screen is the product.
- Source: Alex's use.
- Method: T
- Verified by: allocated: REQ-PRD-02.R
- Phase: EVT
- Status: PROPOSED

### REQ-PRD-02.R: One readable screen (ref_design)
- Level: product
- Board: ref_design
- Statement: The 320 x 480 ER-TFT035IPS-6 shall show a full-screen test image with 0 missing lines or columns and correct red, green and blue order, seen from either seat.
- Rationale: the one shared screen is the product.
- Source: ER-TFT035IPS-6 datasheet (pin_review/panel.json).
- Method: T
- Verified by: VER-60, VER-61
- Phase: EVT
- Status: PROPOSED

### REQ-PRD-03: Play time
- Level: product
- Board: both
- Statement: From charge-done, the device shall play for at least 4.0 h before its low-cell cutoff at 25 C, backlight at 50% and sound at -12 dBFS.
- Rationale: a session of play on one charge.
- Source: Claude's estimate; Alex to set the target.
- Method: T
- Verified by: allocated: REQ-PRD-03.R
- Phase: EVT
- Status: PROPOSED

### REQ-PRD-03.R: Play time (ref_design)
- Level: product
- Board: ref_design
- Statement: From charge-done, the device shall play for at least 4.0 h before the 3.50 V cutoff at 25 C, backlight at 50% duty and sound at -12 dBFS.
- Rationale: about 700 mAh against an estimated 110-140 mA (core up to 50 mA, panel logic 10 mA, backlight 56 mA at 50%, amplifier 4.4 mA plus sound); the usable capacity to the cutoff is measured per lot (VER-153), and raising the cutoff to about 3.6 V (DF-34) costs a share of it.
- Source: Claude's estimate; Alex to set the target.
- Method: T
- Verified by: VER-101, VER-153
- Phase: EVT
- Status: PROPOSED
- Notes: proposed: DEFERRED pending VER-37 (the cell's spec sheet) and VER-153 (F-33); only Alex sets it.

### REQ-PRD-04: Charge time
- Level: product
- Board: both
- Statement: From the low-cell cutoff the cell shall reach charge-done within 3.0 h at 25 C from a source offering 1.5 A or more, device on the charging screen.
- Rationale: a flat unit is ready again within an evening.
- Source: Claude's estimate; Alex to set the target.
- Method: T
- Verified by: allocated: REQ-PRD-04.R
- Phase: EVT
- Status: PROPOSED

### REQ-PRD-04.R: Charge time (ref_design)
- Level: product
- Board: ref_design
- Statement: From the 3.50 V cutoff the cell shall reach charge-done (CHG_DONE_N low) within 3.0 h at 25 C from a source offering 1.5 A or more, at 4.75-5.25 V, device on the charging screen.
- Rationale: 333-364 mA constant current into about 700 mAh is about 1.7 h, plus the constant-voltage tail; below about 4.6 V D7 can hold the charge from terminating (DF-99).
- Source: TP4056 datasheet p.9 PROG table (pin_review/u2.json); Claude's estimate for the tail.
- Method: T
- Verified by: VER-102
- Phase: EVT
- Status: PROPOSED
- Notes: proposed: DEFERRED pending VER-37 (F-33); only Alex sets it.

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
- Phase: CASE
- Status: PROPOSED
- Notes: ref_design only. prod_case's preliminary layout is 70.5 x 153 mm and about 19.1 mm thick; Alex sets his own number.

### REQ-PRD-07: Single-fault safety
- Level: product
- Board: both
- Statement: No single fault in the electronics (one open, short, adjacent-pin bridge, wrong part or wrong rotation) shall cause fire, cell venting, a part above its material limit, or an accessible surface above IEC 62368-1's single-fault limit for its class (TS2, clause to confirm), with 1 or more independent protection layers left working after the fault.
- Rationale: a Li-ion cell in a printed case held in the hands; the standard sets touch limits for normal operation (REQ-THM-01) and looser ones under a single fault.
- Source: IEC 62368-1 Annex B.4 (single fault conditions) and clause 9 (to confirm); validation_plan #89, #151.
- Method: A, T
- Verified by: VER-98, VER-97, VER-168
- Phase: ORDER
- Status: PROPOSED

### REQ-PRD-08: Recoverable firmware
- Level: product
- Board: both
- Statement: A unit with blank, corrupt or hung firmware shall enter the USB boot loader with the case closed, with the cell flat and USB in, 2 out of 2 tries, and shall take a debugger with the case open.
- Rationale: bring-up and field recovery must not need the case opened or a soldering iron.
- Source: validation_plan #182, #202.
- Method: D, T
- Verified by: allocated: REQ-PRD-08.R
- Phase: EVT
- Status: PROPOSED

### REQ-PRD-08.R: Recoverable firmware (ref_design)
- Level: product
- Board: ref_design
- Statement: A unit with blank, corrupt or hung firmware shall enter the USB boot loader through the lid's pinholes with the case closed, with the cell flat and USB in, 2 out of 2 tries; with the case open SWD shall attach through J5 and through J7.
- Rationale: BOOTSEL and RESET sit under the pinholes; J5 is a JST SH and J7 a row of pogo pads.
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
- Phase: EVT
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
- Verified by: VER-67, VER-134, VER-14
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-02: Charge current
- Level: subsystem
- Board: both
- Statement: The constant-current charge shall not exceed 0.5 C of the cell's rated capacity or the cell maker's standard charge current, whichever is lower.
- Rationale: 0.5 C is the usual standard rate for small pouch cells.
- Source: IEC 62133-2 (the cell maker's charging limits); validation_plan #28, #128.
- Method: T, I, A
- Verified by: allocated: REQ-BAT-02.R
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-02.R: Charge current (ref_design)
- Level: subsystem
- Board: ref_design
- Statement: The constant-current charge shall measure 300-400 mA (designed 333-364 mA by R5 = 3.3k) and shall not exceed 0.5 C of the 403450's rated capacity or its maker's standard charge current, whichever is lower; cell.json holds the cell's numbers and an assert recomputes the limit whenever R5 or the cell changes.
- Rationale: R5 was once kept from the 503450 (DEF-11); the 403450's own limit is not known yet.
- Source: TP4056 datasheet p.9 (1100-1200 V / R5, pin_review/u2.json).
- Method: T, I, A
- Verified by: VER-66, VER-37, VER-160
- Phase: EVT
- Status: PROPOSED
- Notes: proposed: DEFERRED pending VER-37 (F-33); only Alex sets it.

### REQ-BAT-03: Charge temperature window
- Level: subsystem
- Board: both
- Statement: Charging shall pause when the cell is above 45 C or below 0 C, each +-3 C, or inside the cell maker's charge range if that is narrower, and resume when back in range.
- Rationale: charging a Li-ion cell outside its range plates lithium (cold) or speeds failure (hot).
- Source: IEC 62133-2 (charging within the cell maker's temperature limits); TP4056 TEMP thresholds 45% and 80% of VBUS (pin_review/u2.json); validation_plan #30.
- Method: A, T
- Verified by: VER-32, VER-69, VER-135
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
- Statement: 100% of cells shall be checked with a meter before first mating and the result recorded per lot.
- Rationale: small cell connectors have no battery polarity standard; a reversed cell can destroy the charger.
- Source: assembly practice; IEC 62133-2.
- Method: I
- Verified by: allocated: REQ-BAT-05.R
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-05.R: Cell polarity (ref_design)
- Level: subsystem
- Board: ref_design
- Statement: 100% of cells shall be checked with a meter before first mating, red lead on J2 pin 1 (silk '+'), and every plug shall latch in a 53261-0271 sample, the results recorded per lot.
- Rationale: PicoBlade has no battery polarity standard; a reversed cell destroys the TP4056 and shorts the cell into it (DF-55); generic MX1.25 housings may not latch (DF-113).
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
- Verified by: VER-88, VER-37, VER-140
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
- Phase: CASE
- Status: PROPOSED
- Notes: case.scad has cell_gap = 0.5 mm, and assembly.md puts a 0.45 mm foam pad in it: DF-84.

### REQ-BAT-08: Nothing hard against the pouch
- Level: subsystem
- Board: ref_design
- Statement: No through-hole lead, part body or rib edge shall be within 0.5 mm of the cell's faces; only the foam pad touches it.
- Rationale: a pierced pouch shorts internally and can burn.
- Source: validation_plan #27; case.scad cell_gap 0.5 mm.
- Method: A, I
- Verified by: VER-06, VER-48, VER-109, VER-164
- Phase: ORDER
- Status: PROPOSED

### REQ-BAT-09: Charge path single faults
- Level: subsystem
- Board: ref_design
- Statement: No single fault in U2, Q2, D5, D6, D7, R5, R6, R39, R40 or TH1 shall put more than 4.25 V or more than 1 C on the cell without at least 1 independent layer (the cell's PCM counts as one) still working, and each latent fault that removes a layer shall be found by a per-unit test.
- Rationale: a shorted D7 or Q2, or an open D6, charges the cell straight from VSYS with no regulation (validation_plan #89).
- Source: validation_plan #89; IEC 62368-1 Annex B.4; IEC 62133-2.
- Method: A, T
- Verified by: VER-98, VER-133, VER-169, VER-82, VER-88, VER-168
- Phase: ORDER
- Status: PROPOSED

### REQ-BAT-10: Deep discharge
- Level: subsystem
- Board: both
- Statement: After the firmware shuts down at its cutoff, the board's off-state drain plus the cell's PCM drain and its self-discharge shall keep the cell above its PCM's under-voltage trip for 2 months or more at 25 C, and for the period Alex sets at 45 C storage.
- Rationale: a cell left flat in a drawer should still charge. Between 3.50 V and 3.0 V there is only about 5-10% of the capacity (35-70 mAh). 30 uA of board drain alone is 21.6 mAh a month; a DW01-class PCM adds 3-6 uA that never passes JP5; self-discharge is 1-3% a month (7-21 mAh); D5's leakage roughly doubles the board's drain at 45 C (DF-105). With both terms the figure may be nearer 1 month: Alex sets the number knowing them.
- Source: Claude's estimate; validation_plan #33.
- Method: A, T
- Verified by: VER-103, VER-65, VER-146
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

### REQ-BAT-12: Cutoff accuracy
- Level: subsystem
- Board: ref_design
- Statement: The firmware's low-cell cutoff shall act at a true cell voltage of 3.50 V +0.05/-0.00 V on every unit after its per-unit calibration (REQ-TST-06), at the highest 3V3 load the firmware draws, including units whose LDO output plus dropout (Vreg + Vd) exceeds 3.50 x CV/4.2.
- Rationale: the unplug margin (REQ-PWR-05) holds only to about a 3.45 V cell, 50 mV under the cutoff; the VBAT reading uses 3V3 as its reference (ME6211 +-2%), so uncalibrated it is good to about +-105 mV, and on a high-Vreg unit the calibrated reading levels off above 3.50 V and the cutoff never fires (DF-116).
- Source: design.py D7 comment; pinmap.md; Claude's analysis (O-23, O-49).
- Method: A, T
- Verified by: VER-138, VER-139, VER-67
- Phase: ORDER
- Status: PROPOSED
- Notes: if this cannot be met in firmware, the fallback is a 3.6 V cutoff (which also settles DF-34) or a voltage reference, which needs copper: decide before the order.

### REQ-BAT-13: Charge timeout
- Level: subsystem
- Board: both
- Statement: The firmware shall stop a charge with CHG_OFF and show a charge fault until USB is re-plugged when CHG_STAT_N has stayed low for more than 4 h, or VBAT has stayed under 3.0 V for more than 30 min while charging.
- Rationale: the TP4056 has no safety or pre-charge timer (DF-104); about 2 h of CC plus 1 h of CV for 700 mAh at 0.35 A, plus margin.
- Source: Claude's proposal (O-50); TP4056 datasheet (pin_review/u2.json).
- Method: T
- Verified by: VER-145
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-14: Mating the cell
- Level: subsystem
- Board: ref_design
- Statement: Plugging a 4.2 V cell into J2 with no USB shall not trip the PCM (VBAT present after each plug), shall leave the device off (PWR_EN low), and shall peak VSYS at 4.5 V or less, 10 plugs out of 10.
- Rationale: with no USB, R6 holds Q2 on, so the plug connects about 47 uF through 0.25-0.35 ohm: about 12-17 A (DF-103).
- Source: Claude's analysis (O-42).
- Method: A, T
- Verified by: VER-140, VER-141
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-15: Cell leads protected
- Level: subsystem
- Board: ref_design
- Statement: Both cell leads shall be sleeved (heat-shrink or a printed grommet) where they pass the board's hole, with 0 bare lead insulation against the routed FR4 edge, seen in the FAI and after the drop test.
- Rationale: the routed edge wears the insulation and shorts CELL_P to the GND pour, with only the PCM left (DF-108).
- Source: Claude's review (F-14).
- Method: I
- Verified by: VER-109, VER-93
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-16: Usable capacity per lot
- Level: subsystem
- Board: ref_design
- Statement: One cell per lot shall deliver 600 mAh or more from charge-done to the chosen cutoff at about 140 mA, with its capacity to 3.50 V and to 3.60 V recorded.
- Rationale: capacity on low-cost cells is often overstated; play time (REQ-PRD-03.R) and the cutoff choice (DF-34) depend on it.
- Source: Claude's proposal (O-58): Alex sets the lot limit.
- Method: T
- Verified by: VER-153
- Phase: ORDER
- Status: PROPOSED

## Subsystem: power path (PWR)

### REQ-PWR-01: USB input range: run
- Level: subsystem
- Board: both
- Statement: The device shall run (3V3 in regulation, no limit-cycling) from 4.40-5.50 V at the USB-C connector, both plug orientations, and shall not limit-cycle on a source that folds back at 500 mA.
- Rationale: USB 2.0 sources give 4.75-5.25 V; cables drop more.
- Source: USB 2.0 cl. 7.2.1; TP4056 VCC 4.0-8.0 V (pin_review/u2.json); validation_plan #51, #80.
- Method: T
- Verified by: VER-80, VER-55
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-16: USB input range: charge
- Level: subsystem
- Board: ref_design
- Statement: From 4.75-5.25 V at the USB-C connector the cell shall charge to done within REQ-PRD-04.R's time; between 4.40 V and 4.75 V whether CHG_DONE_N goes low, and how much current D7 carries, shall be measured and recorded.
- Rationale: below about 4.6 V VSYS falls under the cell's CV and D7 conducts, so the charge may never terminate (DF-99); no TP4056 board can promise charge-done at 4.40 V.
- Source: Claude's analysis (O-24, F-9).
- Method: T
- Verified by: VER-80, VER-102
- Phase: EVT
- Status: PROPOSED
- Notes: decision for Alex after VER-80: raise the lower bound, or accept.

### REQ-PWR-02: Hot-plug peaks
- Level: subsystem
- Board: ref_design
- Statement: At a live plug-in from an always-on USB-A port with a 2 m cable and no cell, VBUS shall peak at 7.7 V or less and VSYS at 5.7 V or less.
- Rationale: REQ-PWR-14's transient rule (0.3 V under the absolute maximum): TP4056 8.0 V; FM8002A 6.0 V, the lowest on VSYS; ME6211 6.5 V.
- Source: TP4056 p.2 (pin_review/u2.json), FM8002A p.3 (pin_review/u7.json), ME6211 p.5 (pin_review/u3.json); design.py R47 comment.
- Method: A, T
- Verified by: VER-12, VER-71
- Phase: ORDER
- Status: PROPOSED
- Notes: bringup.md 5.6 still passes 6.5 V (DEF-06).

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
- Verified by: VER-11, VER-70, VER-138
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
- Statement: With the device off and no USB, the cell current shall be 30 uA or less at 25 C and 60 uA or less at 45 C, for a cell at 3.7-4.2 V.
- Rationale: designed 18-22 uA at 25 C (divider, charger standby, D5 leakage, the amplifier's shutdown current); D5 alone adds about 20 uA at 45 C (DF-105).
- Source: design_review.md Off state; bringup.md step 5.1; the 45 C number is Claude's estimate (O-51).
- Method: T
- Verified by: VER-65, VER-146
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-08: Turn-on margin
- Level: subsystem
- Board: ref_design
- Statement: With 10 kohm in series with seat 1's Start contact and the cell at 3.50 V, pressing Start shall bring 3V3 to 3.20 V or more within 50 ms and PWR_BTN_N shall read low, with Q1's gate margin over its maximum threshold reported in volts.
- Rationale: a worn or dirty pill adds resistance (REQ-BTN-01 allows 10 kohm): at VSYS 3.42 V Q1's gate sees 3.42 x 33/43 = 2.62 V against a 2N7002 Vgs(th) of up to 2.5 V, about 0.1 V of margin; 1 kohm would prove nothing.
- Source: validation_plan section 1 'Latch replica' and Fable's note 6; design.py R13 comment; q1.json.
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
- Statement: The board shall present 10 uF or less on VBUS. With no cell, its plug-in charge is about 265 uC (C1 10 uF x 5 V = 50 uC, plus 46.8 uF of VSYS capacitance reached through D5 x 4.6 V = 215 uC), over USB 2.0's 50 uC: a written deviation, accepted only with this requirement.
- Rationale: sources with soft start tolerate it; some hubs fold back at plug-in, which looks like a bad cable at EVT; R47's 0.1 ohm does not slow it.
- Source: USB 2.0 cl. 7.2.4.1; validation_plan #125; design.py C1, C2, C5, C9, C34, C35.
- Method: A
- Verified by: VER-99
- Phase: ORDER
- Status: PROPOSED

### REQ-PWR-14: Derating
- Level: subsystem
- Board: ref_design
- Statement: In every mode (off, cell, USB, charging, hot plug, unplug): each IC's steady-state stress shall be at or below its recommended operating maximum and each transient at least 0.3 V below its absolute maximum; each passive and discrete (MLCC voltage, FET Vds and Vgs, diode VR and IF, resistor power) shall be at 80% or less of its rating, 90% for transients under 10 us; every exception named here.
- Rationale: the old rule (80% of every absolute maximum) failed by construction: 3V3 at 3.40 V is 94% of the RP2350's 3.63 V, U7 on a 5.45 V USB VSYS is 91% of 6.0 V and U3 is 84% of 6.5 V, so it could not tell an intended stress from an oversight.
- Source: validation_plan #5, #123; Claude's proposal, revised (O-22).
- Method: A
- Verified by: VER-31, VER-36
- Phase: ORDER
- Status: PROPOSED
- Notes: named exceptions: (1) U7 (FM8002A) at a simulated 5.7 V hot-plug peak against 6.0 V, exactly 0.3 V under, for microseconds, with no cell, from a live always-on port (DEF-06, DF-41); (2) GPIO47 with VBAT/2 on a non-FT pad while IOVDD is 0 V, 24 hours a day on every unit with a cell (pin_review/u4.json pin 58, DF-95), unless Alex takes the BOM-only fix R18 = 56k (0.447 V at 4.20 V, 8 uA of divider drain): decision for Alex (F-46).

### REQ-PWR-15: 3V3 budget
- Level: subsystem
- Board: ref_design
- Statement: The 3V3 load shall be 150 mA or less and U3's dissipation 0.20 W or less at VSYS 5.2 V on USB.
- Rationale: the SOT-23's 300 mW allows about 158 mA at that input; the budget was never written down.
- Source: ME6211 p.5 (pin_review/u3.json: 3V3 budget UNKNOWN).
- Method: A, T
- Verified by: VER-31, VER-83, VER-144
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
- Notes: PWR_HOLD (R42 4.7k) and CHG_OFF (R44 4.7k) meet it; AMP_EN (R37 100k) and BL_PWM (R60 + R61, 103.9k) do not: DF-45, DF-47. Decision for Alex: 4.7k pull-downs (BOM only), or rely on firmware and the placed stepping (VER-75).

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

### REQ-MCU-10: Flash budget
- Level: subsystem
- Board: ref_design
- Statement: The firmware image shall be 1.6 MB or less of the RP2354B's 2 MB (or the limit Alex sets), with a written flash map (image, settings, battle log), checked in scripts/check.sh on every change.
- Rationale: the firmware already fills about 1.37 MB; Gen 3 data and flash logging are still to land (DF-121).
- Source: crates/mega_blastoise_fw/memory-rp2350.x; Claude's proposal (O-53).
- Method: A
- Verified by: VER-151, VER-150
- Phase: ORDER
- Status: PROPOSED

### REQ-MCU-11: A bring-up image for the ordered chip
- Level: subsystem
- Board: ref_design
- Statement: Before the order, a self-test image shall build and link for the RP2354B (embassy-rp rp235xb, 2048K of flash) from pin definitions generated from design.py, with 0 hand-typed pin numbers.
- Rationale: the fw crate targets the A package with 4 MB and the ST7796, and the only ref-board branch uses origin/main's pin map (DEF-34): the first build for the real chip would be the day the boards arrive.
- Source: crates/mega_blastoise_fw/Cargo.toml; vp_map #56; Claude's review (F-44, O-37, O-56).
- Method: A
- Verified by: VER-149, VER-150
- Phase: ORDER
- Status: PROPOSED

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
- Verified by: VER-02, VER-43, VER-136
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
- Statement: At the firmware's maximum duty the six LEDs shall draw 120 mA or less in total at worst-case tolerance (3V3 +2%, R60/R61 1%, U6 offset 4.5 mV, R52/R53 -1%) for VSYS 3.5-5.7 V; at nominal parts and 100% duty they draw 113 mA +-5% (56 mA a cathode pin, the two pins within 5% of each other).
- Rationale: the panel's 120 mA maximum; the worst case at 100% is about 122.5 mA (61.2 mA a pin), so the firmware caps the duty at 95%, or R61 becomes 3.6k (BOM only).
- Source: ER-TFT035IPS-6 section 4.4 (pin_review/panel.json); design.py backlight section; Claude's analysis (F-7, O-21).
- Method: T, A
- Verified by: VER-62, VER-137, VER-05
- Phase: ORDER
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
- Statement: At the firmware's maximum duty and VSYS 3.5 V the LED current shall drop by 5% or less from its value at 4.2 V for LED Vf up to 3.2 V, and by 10% or less for Vf up to 3.4 V (the newer datasheet's maximum).
- Rationale: brightness should not sag until the cutoff; at Vf 3.4 V the loop opens at VSYS 3.5 V (52.5 mA, -6.3%).
- Source: board/sim/backlight_results.txt; design_review.md Backlight; pin_review/panel.json.
- Method: A, T
- Verified by: VER-05, VER-137, VER-62
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

### REQ-DSP-11: Redraw time
- Level: subsystem
- Board: ref_design
- Statement: A full-screen redraw from the core's 320 x 480 RGB565 frame, expanded to the ILI9488's 18-bit format on the fly, shall complete in 120 ms or less (Alex sets the number; 200 ms is the slowest the review called acceptable), and a battle-text update in 50 ms or less, with the frame held in 320 KB of RAM or less.
- Rationale: the ILI9488 takes only 18-bit colour over SPI: 461 kB a frame, 184 ms at 20 MHz, and an RGB666 frame does not fit the firmware's 384 KB heap; the pico renders a 300 KB RGB565 frame (DF-120).
- Source: design_review.md (ILI9488 serial: 18-bit only); crates/mega_blastoise_fw/src/mem_profile.rs, st7796.rs; validation_plan #180 (un-cut).
- Method: A, T
- Verified by: VER-43
- Phase: ORDER
- Status: PROPOSED
- Notes: if it fails, the fallback is a 320 x 480 ST7796 panel the firmware already drives, or origin/main's 2.4in ST7789V (240 x 320, RGB565): either changes J3, the backlight and the case, so decide before the order.

### REQ-DSP-12: Backlight halves match at low duty
- Level: subsystem
- Board: ref_design
- Statement: At the lowest duty the firmware uses, the two LED groups' currents shall be within 20% of each other; the minimum duty (about 18%, or a firmware floor) shall be written in pinmap.md.
- Rationale: U6's offset is a larger share of a small BL_REF, so the halves part at low brightness.
- Source: Claude's analysis (O-21).
- Method: A, T
- Verified by: VER-137, VER-62
- Phase: ORDER
- Status: PROPOSED

## Subsystem: audio (AUD)

### REQ-AUD-01: Loudness
- Level: subsystem
- Board: ref_design
- Statement: A full-scale 1 kHz sine shall deliver 0.35 W or more into the 8 ohm speaker (4.75 V peak-to-peak across it or more) on a 3.8 V cell.
- Rationale: designed 0.4-0.5 W on the cell, 0.55 W on USB; it clips below a 3.7 V cell (4.7 Vpp, the old number, is 0.345 W).
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

### REQ-AUD-06: Speaker rating
- Level: subsystem
- Board: ref_design
- Statement: The speaker shall be rated 0.6 W or more, or the firmware shall cap full scale to its rating.
- Rationale: U7 gives about 0.55 W into 8 ohm on USB, more once it clips toward a square wave; 15 mm 8 ohm speakers are often rated 0.25-0.5 W (DF-49).
- Source: Claude's review (O-31).
- Method: I
- Verified by: VER-128
- Phase: ORDER
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
- Phase: EVT
- Status: PROPOSED
- Notes: measured on a bare board from the same JLC order (F-49); a coupon ordered now is advisory.

### REQ-BTN-02: Contact stays open
- Level: subsystem
- Board: both
- Statement: Leakage between a contact's two combs shall be 100 Mohm or more at 4.2 V, clean and after 24 h at high humidity.
- Rationale: on SW21 the comb carries VSYS, so leakage turns the device on (REQ-PWR-11); on the others it reads as a press.
- Source: validation_plan gate item 5, #185.
- Method: T
- Verified by: VER-46, VER-79
- Phase: EVT
- Status: PROPOSED

### REQ-BTN-03: Comb geometry
- Level: subsystem
- Board: both
- Statement: The combs shall keep a copper gap of 0.20 mm or more; the worst bridging spot shall be 1.60 mm or less for the 4-ring under the 3.5 mm D-pad and A/B/X/Y pill, and under half the measured Start/Select pill for the 3-ring; one 4.8 mm mask opening shall uncover all comb copper with no paste or silk.
- Rationale: the DS Lite's measured comb; 0.20 mm sits above JLC's minimum spacing; SW21, the power button, sits on the 3-ring (DF-124).
- Source: hardware/membrane/README.md; check_contact_fp.py; case/DSLITE.md.
- Method: A
- Verified by: VER-08, VER-09, VER-10, VER-25, VER-162
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
- Phase: CASE
- Status: PROPOSED

### REQ-BTN-06: Pill registration
- Level: subsystem
- Board: both
- Statement: Each pill's centre shall land within 0.9 mm of its comb's centre at worst-case tolerance.
- Rationale: (3.5 mm pill - 1.6 mm bridging spot) / 2 is about 0.95 mm.
- Source: validation_plan #197.
- Method: A, I
- Verified by: VER-174, VER-51
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
- Phase: CASE
- Status: PROPOSED

### REQ-MEC-03: Buttons work under the shell
- Level: subsystem
- Board: ref_design
- Statement: Every cap shall sit and press its full dome travel (the D-pad rocked in 8 directions) with 0 contact with the shell, and at rest touch only its dome.
- Rationale: a cap that drags or is preloaded feels wrong or sticks.
- Source: case/check_buttons.py.
- Method: A, D
- Verified by: VER-07, VER-48
- Phase: CASE
- Status: PROPOSED

### REQ-MEC-04: Assembly keyed
- Level: subsystem
- Board: ref_design
- Statement: The board shall fit the tub in 1 orientation only, and each A/B/X/Y cap in 1 hole and 1 rotation only.
- Rationale: no wrong assembly by hand.
- Source: assembly.md sections 3 and 5.
- Method: I
- Verified by: VER-48
- Phase: CASE
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
- Phase: CASE
- Status: PROPOSED

### REQ-MEC-09: Screws engage
- Level: subsystem
- Board: ref_design
- Statement: Each M2 case screw shall engage 4 mm or more of its lid pilot and end 0.5 mm or more short of the pilot's end.
- Rationale: self-tapping screws in printed PETG want 2 to 2.5 x d; M2 x 8 engages 1.4 mm today (DEF-31).
- Source: case.scad; Claude's analysis (O-52).
- Method: A, T
- Verified by: VER-147, VER-92
- Phase: CASE
- Status: PROPOSED

### REQ-MEC-10: Panel stays bonded
- Level: subsystem
- Board: ref_design
- Statement: The panel shall stay bonded to the lid, with no mura and no new contact marks, after 100 presses of 20 N at the glass centre and 24 h at 50 C, on a tape whose part number the release record names.
- Rationale: two players point at and press the shared screen; the panel hangs on 0.15 mm tape 0.2-0.4 mm above parts (DF-107).
- Source: Claude's review (O-59).
- Method: T
- Verified by: VER-154
- Phase: EVT
- Status: PROPOSED

### REQ-MEC-11: Connector access
- Level: subsystem
- Board: ref_design
- Statement: Every connector's mouth (J1 with a mated plug, J2's plug and wires, J3's FPC lead-in, J5 and J6 with a JST SH cable) shall be clear of parts taller than 1 mm and of every case wall but its own opening, and no pinhole shall fall inside a boss.
- Rationale: J6 once faced J5's back (DEF-20), the pinholes once met a side screw (2f43660) and the USB-C opening was fixed twice.
- Source: Claude's review (F-28).
- Method: A
- Verified by: VER-165
- Phase: ORDER
- Status: PROPOSED

## Subsystem: thermal (THM)

### REQ-THM-01: Touch temperature
- Level: subsystem
- Board: both
- Statement: In normal operation, every outer surface a player holds shall stay at or below 43 C (target) at 25 C ambient, and the measured rise plus the rated maximum ambient shall stay at or below 48 C, while charging and playing with the backlight at 100% and audio at full scale.
- Rationale: IEC 62368-1's TS1 limit for plastic held 10 min to 8 h is 48 C (43 C beyond 8 h, to confirm); a handheld is held for hours. Single faults have their own, looser limit (REQ-PRD-07).
- Source: IEC 62368-1 cl. 9.3, Table 38 (to confirm); validation_plan #93.
- Method: T, A
- Verified by: VER-86, VER-144
- Phase: EVT
- Status: PROPOSED

### REQ-THM-02: Thermistor tracks the cell
- Level: subsystem
- Board: ref_design
- Statement: During a full charge in the closed case, TH1's reading shall stay within 5 C of the cell's hotter point (its centre or its edge under TH1).
- Rationale: TH1 sits 2.2 mm inside the cell's long edge, about 15 mm from its centre, behind the board and the foam pad (DF-04); the window in REQ-BAT-03 is only as good as this.
- Source: design_review.md Open risks; validation_plan #30; placement.py, case.scad.
- Method: T, A
- Verified by: VER-87, VER-164
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

### REQ-THM-04: Heat budget per mode
- Level: subsystem
- Board: ref_design
- Statement: Charging while playing (5.5 V source, 3.0 V cell, backlight and audio at full scale) shall dissipate 2.0 W or less in total, tabulated per mode (off, cell play, USB play, charging, charging while playing) with each part's position before the order, and the firmware rule the table implies (for example CHG_OFF or a capped volume while the backlight is at full on USB) shall be in the bring-up image before the touch test.
- Rationale: a rough worst case on a 5.5 V source with a 3.0 V cell is about 2 W in a 75.4 x 156.24 x 13.4 mm case: about 7 K of mean rise and several times that over U2, beside the glass.
- Source: Claude's analysis (O-47, F-48).
- Method: A
- Verified by: VER-144
- Phase: ORDER
- Status: PROPOSED

## Subsystem: ESD and EMC (EMC)

### REQ-EMC-01: ESD
- Level: subsystem
- Board: both
- Statement: The device shall survive +-4 kV contact discharge on the USB-C shell and +-8 kV air on seams and button gaps with no damage, recovering by itself or by a power cycle.
- Rationale: consumer ESD level; hands touch it all day.
- Source: IEC 61000-4-2; EN 55035 (Table 1, ESD); validation_plan #163.
- Method: T
- Verified by: VER-95, VER-170
- Phase: DVT
- Status: PROPOSED
- Notes: screened at EVT on unit 4 with a piezo igniter (VER-95, comb pads first); the calibrated IEC test on the DVT spin (VER-170).

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
- Statement: The board shall pass DRC against JLC's published 2-layer limits, copied with the capability page's date into a data file the build reads (track, space, the minimum via pad for a 0.3 mm hole, annular ring, copper to the edge 0.30 mm or more), with 0 errors; where the board's numbers are below the page's (the copied core's 0.10 mm spaces; 0.5 mm vias on 0.3 mm holes, a 0.10 mm ring) JLC confirms them in writing or they are widened.
- Rationale: KiCad's defaults do not encode JLC's limits; the old 0.13 mm ring, written from memory, failed every via on the board (F-12).
- Source: validation_plan #10, #118, #120; JLC capabilities page, dated on order day.
- Method: A, I
- Verified by: VER-17, VER-18, VER-28
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-04: Holes
- Level: board
- Board: ref_design
- Statement: Every drilled hole shall be 0.30 mm or larger, and plated and unplated holes shall be in separate drill files (J1's two 0.65 mm pegs and H1-H6 unplated).
- Rationale: smaller holes cost about $40 an order at JLC (a fee, a 4-wire test, TG155); a mixed file may plate J1's pegs (DEF-32).
- Source: design_review.md Board; assembly.md section 1; JLC's KiCad guide.
- Method: A
- Verified by: VER-114, VER-131
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-05: Order options
- Level: board
- Board: both
- Statement: The order shall be 2 layers, 1.6 mm, ENIG, with the order number's location specified ('JLCJLCJLCJLC' on the back silk), edge rails off (or routed removal), and every option recorded line by line in the release record (1 record per order).
- Rationale: the pills press on bare combs, so the finish must be gold; a V-cut rail leaves a ridge the case slot is not sized for; a stray order number can land on a comb (DF-125).
- Source: assembly.md section 1; validation_plan #189.
- Method: I
- Verified by: VER-42, VER-28
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-06: Part identity
- Level: board
- Board: both
- Statement: Every LCSC number shall resolve to the MPN, package, pin count and rating design.py intends, with 0 substitutions on the order.
- Rationale: the right family in the wrong variant passes every other check.
- Source: validation_plan #2, #122.
- Method: A, I
- Verified by: VER-14, VER-130, VER-28, VER-41
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-07: Orientation
- Level: board
- Board: both
- Statement: Every polarised part's pin 1 in the fab house's placement preview shall match the board's pin 1, and every part shall sit on its pads, with 0 unexplained differences.
- Rationale: a CAD footprint's zero and origin can differ from the fab house's.
- Source: validation_plan #9, #116.
- Method: I, A
- Verified by: allocated: REQ-FAB-07.R
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-07.R: Orientation and centroids (ref_design)
- Level: board
- Board: ref_design
- Statement: Every polarised part's pin 1 in JLC's placement preview shall match the board's pin 1, U1, D7, U4 and J3's pin-1 end included, and every CPL position shall be within 0.1 mm of the part's pad centroid (J1 and J2 corrected), with 0 unexplained differences.
- Rationale: KiCad's footprint zero differs from JLC's for some parts (DEF-14), and J1's origin is 1.46 mm and J2's 0.60 mm off their pad centres (DEF-33).
- Source: validation_plan #9, #116; 1fd0956.
- Method: I, A
- Verified by: VER-28, VER-29
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-08: Reproducible outputs
- Level: board
- Board: ref_design
- Statement: The gerbers, drill, BOM and CPL shall come from the released commit by one release run from a clean worktree; two runs of fab.py shall give identical copper and drill files once their creation dates are stripped; and the uploaded zip's sha256 shall be in the release record.
- Rationale: the board ordered must be the board checked; Alex's main checkout had an uncommitted KiCad GUI change to battler.kicad_pro on 2026-10-24.
- Source: validation_plan #12, #88, #121, gate item 11.
- Method: A, I
- Verified by: VER-24, VER-143, VER-42, VER-132
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-09: Independent connectivity
- Level: board
- Board: both
- Statement: JLC shall e-test 100% of the boards against an IPC-D-356 netlist exported from the ordered board with each net-tie pair merged into one net (0 pairs of differently named nets joined by copper), or the order shall go without a netlist.
- Rationale: a check from outside KiCad's own reading; JP1-JP5 would read as 5 shorts (DF-126).
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
- Verified by: VER-115, VER-158
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
- Verified by: VER-109, VER-166
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

### REQ-TST-06: Per-unit VBAT calibration
- Level: board
- Board: ref_design
- Statement: Each unit's VBAT gain shall be set in production test at a true 3.50 V from the bench supply (no USB, the self-test at its highest 3V3 load, a DMM checked that session against a 4.096 V reference) and lie within 0.95-1.05; a re-read at 3.50 V shall be within 10 mV; the firmware shall store no gain from a no-cell CHG_DONE_N edge.
- Rationale: the charger's CV is only +-1% (+-42 mV) and with no cell CHG_DONE_N flickers, so a gain from it can be off by about 4%, the cutoff's whole margin (DF-117).
- Source: Claude's review (O-63, O-60).
- Method: T
- Verified by: VER-155, VER-156
- Phase: EVT
- Status: PROPOSED

### REQ-TST-07: Connector mating budget
- Level: board
- Board: ref_design
- Statement: No unit shall ship with J2, J3, J5 or J6 past half its rated mating cycles, counted per unit; no unit used for an FAI or an abuse test shall ship.
- Rationale: PicoBlade and FH12 are rated for roughly 10-30 matings (confirm per maker) and bring-up alone can use them up (DF-118).
- Source: Claude's review (O-61).
- Method: I
- Verified by: VER-157
- Phase: EVT
- Status: PROPOSED

### REQ-TST-08: Per-unit findings
- Level: board
- Board: both
- Statement: Every finding on a built unit shall be recorded with its serial, how it was found, its disposition (rework, scrap or use as is) and its re-test; a finding on 2 units shall become a DEF with a check.
- Rationale: defects.md is for design defects; rework without a re-test, or a recurring process fault never raised, ships bad units.
- Source: Claude's review (O-36).
- Method: I
- Verified by: VER-110
- Phase: EVT
- Status: PROPOSED

## Board: prod_board (PBD)

Alex's board supplies its own evidence. These cover what it has today (16 contacts placed
from prod_case, an empty schematic). The product requirements above (Board: both) reach
prod_board only through the `.P` allocations Alex writes or opts into; until then
`--gate --board prod_board` evaluates these PBD requirements alone. prod_case uses a 10 mm
103450 cell (about 2000 mAh, a 19.1 mm case) and 16 contacts with no Start or Select, so
ref_design's numbers are not his.

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
