# Requirements

What the handheld must do, as numbers a check can pass or fail. Each requirement has a
value and a tolerance, why it is that number, where the number comes from, how it is
proven (T test, A analysis, I inspection, D demonstration), the checks in
[verification.csv](verification.csv) that prove it, the phase by which it must be proven
first, and a status.

**Every requirement here is PROPOSED.** Claude seeded them on 2026-10-24 from
`hardware/ref_design/docs/validation_plan.md` (archived), the datasheet reviews in
`hardware/ref_design/board/pin_review/`, `docs/design_review.md` and IEC 62368-1, revised
them the same day from the two-model review, re-seeded them on 2026-10-25 for 684a609, and
re-seeded them again on 2026-10-26 for the board being ordered, be437f8 (branch
ref-design-35in: the BQ24073 charger, RT9013 LDO, the 3.5 in ER-TFT035IPS-6 panel (ILI9488)
with R61 3.9k, 74HC165s in the button ends, metal domes for Start and Select, the cell in a
hole under the glass's seat-2 half, the speaker sunk in a hole, a 72.4 x 151.64 mm board in a
one-piece top). A requirement whose subject left
the design is RETIRED with the commit that removed it; it keeps its ID. None is binding until
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
  the printed case is released and the EVT units are assembled), EVT (the 5 boards), DVT (the
  revised spin), PVT (the 30-board run), SALE (before any unit is sold or given away).
- Status: PROPOSED until Alex decides; DEFERRED holds only up to the phase his line in
  APPROVALS.md names, then the requirement is PROPOSED again; RETIRED (with a 'Retired:' line
  naming the commit) when its subject left the design.
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
- Rationale: Start and Select added 2026-10-19 (a3158fc); since 2d1124d the 16 D-pad and A/B/X/Y contacts come in through two chained 74HC165s and both Starts turn the device on.
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
- Statement: The 320 x 480 ER-TFT035IPS-6 (ILI9488) shall show a full-screen test image with 0 missing lines or columns and correct red, green and blue order, seen from either seat.
- Rationale: the one shared screen is the product.
- Source: ER-TFT035IPS-6 datasheet (pin_review/panel.json); the panel since 79de1f7 (it was the 2.4 in ER-TFT024IPS-3 from 53e21c8).
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
- Rationale: about 700 mAh against an estimated 90-120 mA (core up to 50 mA, panel logic 10 mA, backlight 56 mA at 50%, amplifier 4.4 mA plus sound); the usable capacity to the cutoff is measured per lot (VER-153).
- Source: Claude's estimate; Alex to set the target.
- Method: T
- Verified by: VER-101, VER-153
- Phase: EVT
- Status: PROPOSED
- Notes: proposed: DEFERRED to EVT pending VER-37 (the cell's spec sheet) and VER-153 (F-33); only Alex sets it.

### REQ-PRD-04: Charge time
- Level: product
- Board: both
- Statement: From the low-cell cutoff the cell shall reach charge-done within 4.0 h at 25 C from a 5 V USB source offering 0.5 A or more, device on the charging screen; the time is Alex's to set and no tighter than its allocations.
- Rationale: a flat unit is ready again within an evening.
- Source: Claude's estimate; Alex to set the target.
- Method: T
- Verified by: allocated: REQ-PRD-04.R
- Phase: EVT
- Notes: the first draft said 3.0 h from 1.5 A, the allocation REQ-PRD-04.R says 4.0 h from 0.5 A (the BQ24073's USB500 input limit is 475 mA): Alex sets the time (PLAN.md decision DEC-04, F-R12).
- Status: PROPOSED

### REQ-PRD-04.R: Charge time (ref_design)
- Level: product
- Board: ref_design
- Statement: From the 3.50 V cutoff the cell shall reach charge-done (CHG_STAT_N high with CHG_PG_N low) within 4.0 h at 25 C from a source offering 0.5 A or more at 4.75-5.25 V, device on the charging screen.
- Rationale: 242-295 mA of constant current (R5 3.3k) into about 700 mAh is 2.4-2.9 h to 80%, plus the constant-voltage tail; the BQ24073's own fast-charge timer stops at 5 h (DF-152); the system's draw comes out of the 475 mA input limit first.
- Source: BQ24073 datasheet SLUS810N, ISET (pin_review/u2.json pin 16); Claude's estimate for the tail.
- Method: T
- Verified by: VER-102
- Phase: EVT
- Status: PROPOSED
- Notes: proposed: DEFERRED to EVT pending VER-37 (F-33); only Alex sets it.

### REQ-PRD-05: Shelf life when off
- Level: product
- Board: both
- Statement: Switched off with a full cell and no USB, the board shall drain the cell by no more than 30 uA, so that the board alone takes 2 years or more to empty it.
- Rationale: 700 mAh / 30 uA is about 2.7 years; the cell's own self-discharge dominates.
- Source: design_review.md's Off state (18-22 uA) still counts D5's reverse leakage, and D5 left in 2d1124d: VER-103 re-derives the budget for the BQ24073 board (O-R23); validation_plan #33.
- Method: T, A
- Verified by: VER-65, VER-103
- Phase: EVT
- Status: PROPOSED

### REQ-PRD-06: Size
- Level: product
- Board: ref_design
- Statement: The closed case shall measure at most 77.4 x 156.64 mm in plan and 9.6 mm thick at the screen, 7.7 mm at the button ends, each +0.3 mm for print tolerance.
- Rationale: a DS Lite's feel with the width the buttons set and the length the 3.5 in panel sets (hardware/ref_design/PLAN.md, Case).
- Source: hardware/ref_design/README.md; case.scad (case_w, case_l, case_h, deck_face).
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
- Statement: A unit with blank, corrupt or hung firmware shall enter the USB boot loader through the top piece's pinholes with the case closed, with the cell flat and USB in, 2 out of 2 tries; with the case open SWD shall attach through J5 and through J7.
- Rationale: BOOTSEL and RESET sit under the pinholes in the left bezel below H4 (86bf694); J5 is a JST SH and J7 a row of pogo pads.
- Source: validation_plan #182, #202; bringup.md.
- Method: D, T
- Verified by: VER-107, VER-58
- Phase: EVT
- Status: PROPOSED

### REQ-PRD-09: Cost
- Level: product
- Board: ref_design
- Statement: The all-in cost per unit at 10 units shall be $45 or less (board, assembly, shipping, panel, cell, speaker, silicone, caps, screws, filament).
- Rationale: the number comes from VER-117's re-run of cost.py with a new board quote at 72.4 x 151.64 mm; bom.md's board line is still the quote for 70.5 x 151.34 mm (3% less area).
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
- Source: BQ24073 datasheet SLUS810N, VBAT(REG) (design.py and pinmap.md quote 4.2 V +-1%); IEC 62133-2 (upper limit charging voltage set by the cell maker).
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
- Statement: The constant-current charge shall measure 242-295 mA (R5 = 3.3k against the BQ24073's KISET of 890 A-ohm) and shall not exceed 0.5 C of the 403450's rated capacity or its maker's standard charge current, whichever is lower; cell.json holds the cell's numbers and an assert recomputes the limit whenever R5 or the cell changes.
- Rationale: R5 was once kept from the 503450 (DEF-11); the 403450's own limit is not known yet.
- Source: pin_review/u2.json pin 16 (ICHG = 890 / R5).
- Method: T, I, A
- Verified by: VER-66, VER-37, VER-160
- Phase: EVT
- Status: PROPOSED
- Notes: proposed: DEFERRED to EVT pending VER-37 (F-33); only Alex sets it.

### REQ-BAT-03: Charge temperature window
- Level: subsystem
- Board: both
- Statement: At 99.9% of units, charging shall pause when the cell is above 45 C or below 0 C, or outside the cell maker's charge range if that is narrower: the hot edge at or under 45 C and the cold edge at or over 0 C (one-sided limits), and resume when back in range.
- Rationale: charging a Li-ion cell outside its range plates lithium (cold) or speeds failure (hot).
- Source: IEC 62133-2 (charging within the cell maker's temperature limits); the BQ24073's TS thresholds, 0.30 V hot and 2.1 V cold at 75 uA (pin_review/u2.json pin 1); validation_plan #30.
- Method: A, T
- Verified by: VER-32, VER-69, VER-135
- Phase: EVT
- Status: PROPOSED
- Notes: with TH1 (B3950 10k) the window is 3.5-47.2 C nominal, so the hot edge fails before any tolerance. No series or parallel resistor fixes both edges (a parallel one that brings the hot edge to 45 C drags the cold edge to -11.6 C). PLAN.md decision DEC-05: a BOM-only TH1 with a higher B (B 4250: 4.9-45.5 C; B 4350: about 5.3-45.0 C nominal) if JLC stocks one, or accept 47.2 C against the cell's datasheet (DF-147).

### REQ-BAT-04: Cell temperature during charge
- Level: subsystem
- Board: both
- Statement: The cell surface shall stay at or below 45 C through a full charge in the closed case at 25 C ambient, and at or below the cell maker's maximum charge temperature at 35 C ambient.
- Rationale: the cell sits in a hole in the board in a printed case, under the panel.
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
- Rationale: PicoBlade has no battery polarity standard; a reversed cell destroys the BQ24073 and shorts the cell through it (DF-55); generic MX1.25 housings may not latch (DF-113).
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
- Statement: The closed case shall leave 0.4 mm or more of free height above the cell at worst-case tolerance (a 4.5 mm cell), under everything that lies over its pocket.
- Rationale: pouch cells grow 8-10% in thickness over life; 0.4 mm for a 4.0 mm cell.
- Source: validation_plan #26; case.scad (cell_top 1.6 over the board; the panel's back 2.4, the panel frame's walls 2.8, the button ends' underside 1.9 where they reach).
- Method: A, I
- Verified by: VER-182, VER-20, VER-48
- Phase: CASE
- Status: PROPOSED
- Notes: at be437f8 the cell lies under the glass's seat-2 half and no button end reaches it: 0.80 mm under the panel's back (VER-182 PASS); it was 0.30 mm under a deck at 684a609 (DEF-51).

### REQ-BAT-08: Nothing hard against the pouch
- Level: subsystem
- Board: ref_design
- Statement: No routed board edge, part body, plug or rib shall press on the cell's faces: 0.5 mm or more between a worst-case cell and the board's hole on every face it can rest against (1.5 mm at its lead end), after the fab, print and fastening tolerances, and no back-side copper within 1 mm of the hole.
- Rationale: a pierced pouch shorts internally and can burn.
- Source: validation_plan #27; placement.py CUTOUTS (the hole 54.0 x 37.0 mm); case.scad cell.
- Method: A, I
- Verified by: VER-181, VER-06, VER-48, VER-109, VER-164
- Phase: ORDER
- Status: PROPOSED
- Notes: Not met on any side at be437f8: the tub's pocket sits 0.25 mm inside the hole's outline on every side (case.scad cell_pocket_fit), so a worst-case 52 x 36 cell against any wall of it is 0.25 mm from the routed edge, -0.15 mm after tolerances, and its lead end 1.35 mm (VER-181 FAIL). Widening one end cannot help while the pocket follows the hole. What passes (design_numbers.py cellgap): the pocket 0.9 mm inside the hole and a cell of at most 51.2 x 35.2 mm (case and cell spec), or a 54.8 x 37.8 mm hole (copper). PLAN.md decision DEC-27 before the order.


### REQ-BAT-09: Charge path single faults
- Level: subsystem
- Board: ref_design
- Statement: No single fault in U2, R5, R64, R43, R44, TH1, C3 or the cell's leads shall put more than 4.25 V or more than 1 C on the cell without at least 1 independent layer (the cell's PCM counts as one) still working, and each latent fault that removes a layer shall be found by a per-unit test.
- Rationale: a BQ24073 whose CV or battery FET fails, or whose TS is held in range (DF-145, DF-146, DF-151), charges the cell with only the PCM left.
- Source: validation_plan #89; IEC 62368-1 Annex B.4; IEC 62133-2.
- Method: A, T
- Verified by: VER-98, VER-133, VER-134, VER-135, VER-169, VER-82, VER-88, VER-168
- Phase: ORDER
- Status: PROPOSED

### REQ-BAT-10: Deep discharge
- Level: subsystem
- Board: both
- Statement: After the firmware shuts down at its cutoff, the board's off-state drain plus the cell's PCM drain and its self-discharge shall keep the cell above its PCM's under-voltage trip for 2 months or more at 25 C, and for the period Alex sets at 45 C storage.
- Rationale: a cell left flat in a drawer should still charge. Between 3.50 V and 3.0 V there is only about 5-10% of the capacity (35-70 mAh). 30 uA of board drain alone is 21.6 mAh a month; a DW01-class PCM adds 3-6 uA that never passes JP5; self-discharge is 1-3% a month (7-21 mAh); VSYS stays live through U2's battery FET when off, so U7's shutdown current is in the board's drain. With both terms the figure may be nearer 1 month: Alex sets the number knowing them.
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
- Statement: The firmware's low-cell cutoff shall act at a true cell voltage of 3.50 V +0.05/-0.00 V on every unit after its per-unit calibration (REQ-TST-06), at the highest 3V3 load the firmware draws, including units whose 3V3 plus the RT9013's dropout exceeds 3.50 x CV/4.2.
- Rationale: the ADC reads VBAT/2 against 3V3 itself: in regulation a gain set at charge-done is only as good as the charger's +-1% CV (a cutoff anywhere from 3.465 to 3.535 V), and in dropout the reading rises as the cell falls, so on a unit with 3V3 +2% and 120 mV of dropout it never reaches 3.50 V (3.521 V, VER-138: DF-116). GPIO47 also reads through R17/R18's 235 kohm: every 100 nA of pad leakage moves the pin 23.5 mV, 47 mV of VBAT, the whole +0.05 V window, and it changes with temperature, which a one-point calibration does not remove (O-R15).
- Source: design.py R17/R18; pinmap.md Battery voltage; pin_review/u3.json; Claude's analysis (O-23, O-49).
- Method: A, T
- Verified by: VER-138, VER-139, VER-67
- Phase: ORDER
- Status: PROPOSED
- Notes: decide before the order (PLAN.md decision DEC-07, with R18): a bench calibration at 3.50 V and a firmware dropout rule, or a 3.6 V cutoff; a voltage reference needs copper.

### REQ-BAT-13: Charge timeout
- Level: subsystem
- Board: both
- Statement: A charge that has not terminated shall stop within 5 h +-20% of fast charge (and 30 min of pre-charge under 3.0 V), and the firmware shall show a charge fault until USB is re-plugged.
- Rationale: a soft-shorted or damaged cell must not charge without end; the BQ24073's safety timers do it with TMR open, the firmware reads CHG at 2 Hz as the fault.
- Source: BQ24073 datasheet (pin_review/u2.json pins 9 and 14); Claude's proposal (O-50).
- Method: T
- Verified by: VER-145
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-14: Mating the cell
- Level: subsystem
- Board: ref_design
- Statement: Plugging a 4.2 V cell into J2 with no USB shall not trip the PCM (VBAT present after each plug), shall leave the device off (PWR_EN low), and shall peak VSYS at 4.5 V or less, 10 plugs out of 10.
- Rationale: with no USB, U2's battery FET is on, so the plug connects C3 directly and about 25 uF more on VSYS through the FET: a peak of roughly 10-15 A for microseconds (DF-103).
- Source: Claude's analysis (O-42).
- Method: A, T
- Verified by: VER-140, VER-141
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-15: Cell leads protected
- Level: subsystem
- Board: ref_design
- Statement: Both cell leads shall be sleeved (heat-shrink or a printed grommet) where they run along the board's routed hole edge to J2, with 0 bare lead insulation against the FR4 edge, seen in the FAI and after the drop test.
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
- Rationale: capacity on low-cost cells is often overstated; play time (REQ-PRD-03.R) and the cutoff choice (DF-116) depend on it.
- Source: Claude's proposal (O-58): Alex sets the lot limit.
- Method: T
- Verified by: VER-153
- Phase: EVT
- Status: PROPOSED

## Subsystem: power path (PWR)

### REQ-PWR-01: USB input range: run
- Level: subsystem
- Board: both
- Statement: With a cell in, the device shall run (3V3 in regulation) from 4.40-5.50 V at the USB-C connector, both plug orientations; with no cell, from 4.75-5.50 V; and it shall not limit-cycle on a source that folds back at 500 mA.
- Rationale: USB 2.0 sources give 4.75-5.25 V; cables drop more; under the BQ24073's VIN-DPM (4.5 V) the input current is cut back and the cell makes up the rest, which a board with no cell cannot.
- Source: USB 2.0 cl. 7.2.1; pin_review/u2.json pin 5 (VIN-DPM 4.5 V on USB500); validation_plan #51, #80.
- Method: T
- Verified by: VER-80, VER-55
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-16: USB input range: charge
- Level: subsystem
- Board: ref_design
- Statement: From 4.75-5.25 V at the USB-C connector the cell shall charge to done within REQ-PRD-04.R's time; between 4.40 V and 4.75 V the input current, whether the cell supplements the system, and whether the charge completes shall be measured and recorded.
- Rationale: the BQ24073 cuts its input current back at 4.5 V (VIN-DPM), so a weak source charges slowly or not at all.
- Source: Claude's analysis (O-24, F-9); pin_review/u2.json.
- Method: T
- Verified by: VER-80, VER-102
- Phase: EVT
- Status: PROPOSED
- Notes: decision for Alex after VER-80: raise the lower bound, or accept (PLAN.md decision DEC-15).

### REQ-PWR-02: Hot-plug peaks
- Level: subsystem
- Board: ref_design
- Statement: At a live plug-in from an always-on USB-A port with a 2 m cable and no cell, VBUS shall peak at or under U2's OVP minimum less 0.2 V (6.2 V if SLUS810N's minimum is 6.4 V), so U2 does not retry, EN1 at 6.7 V or less, PWR_EN at 5.5 V or less and VSYS at 4.6 V or less.
- Rationale: REQ-PWR-14's transient rule: EN1 0.3 V under its 7 V absolute maximum (6.7 V), U3's EN (6 V absolute) with margin; the OVP limit from its minimum, not its 6.6 V nominal (O-R18); VSYS is U2's regulated OUT.
- Source: pin_review/u2.json (IN 28 V absolute, OVP 6.6 V, EN1 7 V), pin_review/u3.json (EN 6 V).
- Method: A, T
- Verified by: VER-12, VER-71
- Phase: EVT
- Status: PROPOSED
- Notes: bringup.md 5.6 still describes the TP4056 design's simulation (DEF-21). u2.json quotes only OVP's 6.6 V nominal: VER-34 takes SLUS810N's minimum into it (pin_review is outside hardware/vv) before VER-12 is built.

### REQ-PWR-03: LDO enable never above its input
- Level: subsystem
- Board: ref_design
- Statement: PWR_EN shall stay at or below VSYS + 0.3 V at every instant, including a live plug-in with the cell connected and the device off (10 plug-ins).
- Rationale: the ME6211's CE absolute maximum is VIN + 0.3 V; PWR_EN rises from VBUS ahead of VSYS.
- Source: ME6211 p.5 (pin_review/u3.json); design.py C47 comment.
- Method: T, A
- Verified by: VER-72, VER-12
- Phase: EVT
- Status: RETIRED
- Retired: RETIRED in 1fd0956: the ME6211 left; the RT9013's EN is 6 V absolute whatever VIN is (pin_review/u3.json), so PWR_EN may lead VSYS; the enable's own limit is REQ-PWR-02 and REQ-PWR-17

### REQ-PWR-04: 3V3 regulation
- Level: subsystem
- Board: both
- Statement: 3V3 shall stay at 3.30 V +-3% (3.20-3.40 V) for VSYS 3.60-4.50 V and loads of 0-150 mA.
- Rationale: RP2354B IOVDD and the panel's logic supply; the RT9013 is +-2%.
- Source: pin_review/u3.json; ER-TFT035IPS-6 VCI 2.5-3.3 V recommended (pin_review/panel.json: up to 3.366 V is 66 mV over, accepted as minor: DF-96).
- Method: T
- Verified by: VER-83, VER-55
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-05: 3V3 through the unplug
- Level: subsystem
- Board: ref_design
- Statement: Through the USB-to-cell hand-over at full load (backlight 100%, sound playing), 3V3 shall stay at or above 3.135 V for any cell at or above 3.50 V.
- Rationale: VREG_AVDD's and USB_OTP_VDD's minimum (3.135 V, above QSPI_IOVDD's 2.97 V); the BQ24073 joins OUT to BAT through its battery FET with no gap, so 3V3 is the cell less the FET and the RT9013's dropout (about 3.33 V at 3.50 V and 150 mA, worst case).
- Source: pin_review/u4.json pins 61 and 68; pin_review/u3.json (dropout); design.py U2.
- Method: A, T
- Verified by: VER-70, VER-138
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-06: Hand-over time
- Level: subsystem
- Board: ref_design
- Statement: Q2's channel shall conduct within 5 ms of VBUS removal (designed 1.5-3.3 ms).
- Rationale: until then D7 carries the system at 0.34 V drop.
- Source: design_review.md; design.py R38 comment.
- Method: T, A
- Verified by: VER-70, VER-11
- Phase: EVT
- Status: RETIRED
- Retired: RETIRED in 2d1124d: Q2 and D7 left with the TP4056's load sharing; the BQ24073 has no hand-over gap (REQ-PWR-05 covers 3V3 through the unplug)

### REQ-PWR-07: Off-state current
- Level: subsystem
- Board: both
- Statement: With the device off and no USB, the cell current shall be 30 uA or less at 25 C and 60 uA or less at 45 C, for a cell at 3.7-4.2 V.
- Rationale: designed under 20 uA at 25 C (the 940k divider 4.5 uA, U2 from BAT, U7's shutdown on the live VSYS, U3 off); leakage rises with temperature (O-51).
- Source: bringup.md step 5.1; the 45 C number is Claude's estimate (O-51).
- Method: T
- Verified by: VER-65, VER-146
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-08: Turn-on margin
- Level: subsystem
- Board: ref_design
- Statement: With 1 kohm in series with either Start dome (SW21, SW23) and the cell at 3.50 V, pressing it shall bring 3V3 to 3.20 V or more within 50 ms and PWR_BTN_N (or P2_START) shall read low, with Q1's and Q7's gate margin over their maximum threshold reported in volts.
- Rationale: a dirty or worn metal dome adds resistance: at VSYS 3.45 V Q1's gate sees 3.45 x 33/34 = 3.35 V through 1 kohm against a 2N7002 Vgs(th) of up to 2.5 V.
- Source: design.py R13, R62; pin_review/q1.json; validation_plan section 1 'Latch replica'.
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
- Statement: With 1 Mohm across either Start dome (VSYS to PWR_BTN_RAW or P2_START_RAW) and the cell at 4.2 V, the device shall stay off (3V3 below 0.5 V) for 10 min; the design's sure-on, sure-off and ghost-press bands of dome resistance are those VER-33 prints from U3's VIH and VIL, Q1's and Q7's threshold, R13, R62, R15 and the diodes' Vf (O-R17), and 1 Mohm shall sit above its sure-off band.
- Rationale: both Start domes carry VSYS on their rings all the time; sweat or flux can bridge them.
- Source: validation_plan #185 (written for P1_Y, now SW21 and SW23); design.py R13, R62 and R15; pin_review/u3.json and q1.json; VER-33's output.
- Method: T, A
- Verified by: VER-79, VER-33
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-12: USB input current
- Level: subsystem
- Board: ref_design
- Statement: From a default-power USB source (CC at 0.25-0.61 V) the input current shall be 500 mA or less within 2 s of plug-in.
- Rationale: U2's USB500 limit is 475 mA (EN1 high by R43, EN2 grounded); the charge and the system share it.
- Source: USB 2.0 cl. 7.2.1; pin_review/u2.json pins 5 and 6; validation_plan #94.
- Method: T
- Verified by: VER-81
- Phase: EVT
- Status: PROPOSED
- Notes: decision for Alex: USB 2.0 also caps an unconfigured device at 100 mA and a suspended one at 2.5 mA; U2 charges from plug-in regardless (PLAN.md decision DEC-08).

### REQ-PWR-13: VBUS capacitance and inrush
- Level: subsystem
- Board: ref_design
- Statement: The board shall present 10 uF or less on VBUS (C1, 50 uC at 5.0 V), and nothing but U2's input switch shall join VBUS to VSYS, so the VSYS bulk is charged inside U2's input limit, not by the plug.
- Rationale: USB 2.0's inrush limit is 10 uF or 50 uC at the plug; the TP4056 design's 265 uC came from D5 charging VSYS, which left in 2d1124d.
- Source: USB 2.0 cl. 7.2.4.1; validation_plan #125; design.py C1, U2.
- Method: A
- Verified by: VER-99
- Phase: ORDER
- Status: PROPOSED
- Notes: no deviation is needed at be437f8 (VER-99). U2's current limit while it brings VSYS up is a datasheet claim; VER-71's scope picture shows it.

### REQ-PWR-14: Derating
- Level: subsystem
- Board: ref_design
- Statement: In every mode (off, cell, USB, charging, hot plug, unplug): each IC's steady-state stress shall be at or below its recommended operating maximum and each transient at least 0.3 V below its absolute maximum; each passive and discrete (MLCC voltage, FET Vds and Vgs, diode VR and IF, resistor power) shall be at 80% or less of its rating, 90% for transients under 10 us; every exception named here.
- Rationale: the old rule (80% of every absolute maximum) failed by construction on every 3.3 V part, so it could not tell an intended stress from an oversight (O-22).
- Source: validation_plan #5, #123; Claude's proposal, revised (O-22).
- Method: A
- Verified by: VER-31, VER-36
- Phase: ORDER
- Status: PROPOSED
- Notes: named exceptions: (1) GPIO47 with VBAT/2 on a non-FT pad while IOVDD is 0 V, 24 hours a day on every unit with a cell (pin_review/u4.json pin 58, waived there by the design session; DF-95), unless Alex takes R18 = 56k (PLAN.md decision DEC-07); (2) the panel's VCI and VDDI at up to 3.366 V against 3.3 V recommended (DF-96).

### REQ-PWR-15: 3V3 budget
- Level: subsystem
- Board: ref_design
- Statement: The 3V3 load shall be 150 mA or less and U3's dissipation 0.20 W or less at VSYS 4.5 V on USB.
- Rationale: the RT9013's SOT-23-5 is 0.4 W at 250 C/W; 0.20 W keeps it 50 C over the board; the budget was never written down.
- Source: pin_review/u3.json (0.4 W, dropout).
- Method: A, T
- Verified by: VER-31, VER-83, VER-144
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-17: A faulty USB source
- Level: subsystem
- Board: ref_design
- Statement: A source putting 6-20 V on VBUS (a broken charger, a PD source stuck high) shall over-stress no part: the lowest VBUS that takes a part over its absolute maximum shall be 20 V or more, or the value Alex accepts.
- Rationale: U2 blocks the fault from VSYS (OVP 6.6 V, 28 V absolute), but PWR_EN through R14/D3 takes U3's EN (6 V absolute) over at 6.9 V and EN1 through R43 takes U2's EN1 (7 V) over at 9.5 V (VER-177).
- Source: pin_review/u2.json, u3.json; Claude's analysis (O's sign-off objection on the cell path, DF-101).
- Method: A, T
- Verified by: VER-177, VER-97
- Phase: ORDER
- Status: PROPOSED
- Notes: PLAN.md decision DEC-17: R14 47k and R43 220k (BOM only) raise it to about 9.1 V; 20 V needs a clamp (copper); or accept for EVT.

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
- Statement: SWD shall attach through J5 and through J7, and firmware logs shall reach the host over USB CDC or RTT at 1 line or more a second.
- Rationale: bring-up and recovery; the UART connector J6 left in 86bf694.
- Source: RP-003139-SP (pin_review/j5.json); design.py J5, J7.
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
- Status: RETIRED
- Retired: RETIRED in 86bf694: the revision strap R48-R51 left; BOARD_REV is on the silkscreen and the FAI reads it (VER-109)

### REQ-MCU-06: GPIO functions
- Level: subsystem
- Board: ref_design
- Statement: Every GPIO shall be assigned only a function that pin has in the RP2350 function table, with 0 PWM-slice or peripheral conflicts (PWM6 A on GPIO28, PWM7 B on GPIO31, SPI0 on GPIO33 and 36-39, ADC on GPIO45-47).
- Rationale: a pin that cannot do its job is a respin.
- Source: RP2350 datasheet Table 3 (pin_review/u4.json); validation_plan #55, #140.
- Method: A
- Verified by: VER-15
- Phase: ORDER
- Status: PROPOSED
- Notes: F's objection (round 2) read GPIO28 as PWM2 A; the RP2350's table maps GPIO 0-31 to slices (n >> 1) & 7 and GPIO 32-47 to 8 + ((n >> 1) & 3), so GPIO28 is slice 6 A and GPIO31 slice 7 B (embassy-rp 0.8.0 src/pwm.rs: 'impl_pin!(PIN_28, PWM_SLICE6, ChannelAPin)', 'impl_pin!(PIN_31, PWM_SLICE7, ChannelBPin)'). VER-15 derives each slice from the table, not from pin_review.

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
- Notes: PWR_HOLD (R42 4.7k) and CHG_OFF (R44 4.7k) meet it; AMP_EN (R37 100k) and BL_PWM (R60 + R61, 103.9k) do not: DF-45, DF-47. Decision for Alex (PLAN.md decision DEC-11): 4.7k pull-downs (BOM only), or rely on firmware and the placed stepping (VER-75).

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
- Statement: Before the boards arrive, a self-test image shall build and link for the RP2354B (embassy-rp rp235xb, 2048K of flash) from pin definitions generated from design.py, with 0 hand-typed pin numbers.
- Rationale: the fw crate targets the A package with 4 MB and the ST7796, and the only ref-board branch (fw-ref-board a01f6fb) predates the routing levers: it holds PWR_HOLD on GPIO2 and drives SR_PL on GPIO28 (DEF-34); the image is needed the day the boards arrive, not to order them (round 2's proportionality objection), while the GPIO choices that copper freezes are VER-15's.
- Source: crates/mega_blastoise_fw/Cargo.toml; vp_map #56; Claude's review (F-44, O-37, O-56).
- Method: A
- Verified by: VER-149, VER-150
- Phase: EVT
- Status: PROPOSED

## Subsystem: display (DSP)

### REQ-DSP-01: Panel interface
- Level: subsystem
- Board: ref_design
- Statement: The panel shall run 4-wire 8-bit SPI (IM2-0 = 111, as design.py straps them) and return 0x94 0x88 for ID4 (0xD3) at a read clock of 5 MHz or less.
- Rationale: proves the interface straps and the read path.
- Source: ER-TFT035IPS-6 datasheet section 4.1 (pin_review/panel.json); the ILI9488's serial read cycle (VER-34 quotes it); bringup.md display step 2.
- Method: T
- Verified by: VER-43, VER-60
- Phase: ORDER
- Status: PROPOSED

### REQ-DSP-02: Write speed
- Level: subsystem
- Board: ref_design
- Statement: SPI writes at 20 MHz shall draw 10 full 320 x 480 frames (461 kB each in 18-bit colour) of a checker pattern with 0 corrupted pixels.
- Rationale: the ILI9488 takes only 18-bit colour over SPI; 20 MHz is a conservative clock until VER-34 quotes its serial write cycle.
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
- Source: design.py display section; pin_review/panel.json; the ER-TFT035IPS-6's map since 79de1f7.
- Method: A, T
- Verified by: VER-02, VER-43, VER-136
- Phase: ORDER
- Status: PROPOSED

### REQ-DSP-04: FPC fit
- Level: subsystem
- Board: ref_design
- Statement: The panel's FPC tail shall be 0.30 +-0.05 mm thick (the ER-TFT035IPS-6's drawing: 0.3 +-0.03) and 25.5 +-0.07 mm wide, and the folded FPC shall reach J3 with 0.5-3 mm of slack and no crease.
- Rationale: the FH12-50S's FPC specification; too short cannot mate, too long bows under the panel.
- Source: Hirose FH12 catalog p.2, p.11 (pin_review/j3.json); validation_plan #215, #160.
- Method: I
- Verified by: VER-44, VER-106
- Phase: ORDER
- Status: PROPOSED

### REQ-DSP-05: Backlight current
- Level: subsystem
- Board: ref_design
- Statement: At the firmware's highest duty the six LEDs shall draw 120 mA or less in total at worst-case tolerance (3V3 +2%, R60/R61 1%, U6 offset 4.5 mV, R52/R53 -1%) for VSYS 3.5-4.5 V; at nominal parts and 100% duty they draw 113 mA +-5% (56 mA a cathode pin, the two pins within 5% of each other).
- Rationale: the panel's 120 mA (6 x 20 mA); the worst case at 100% is 122.4 mA (VER-137), so the firmware caps the duty at 97% or R61 becomes 3.74k (DEC-31).
- Source: ER-TFT035IPS-6 section 4.4 (pin_review/panel.json); design.py backlight section; Claude's analysis (F-7, O-21).
- Method: T, A
- Verified by: VER-62, VER-137, VER-05
- Phase: ORDER
- Status: PROPOSED

### REQ-DSP-06: Backlight transient
- Level: subsystem
- Board: ref_design
- Statement: The LED current shall exceed 120 mA in total for 5 us or less at any event, a USB plug-in while lit included (VSYS from the cell's 3.5-4.2 V up to 4.4 V).
- Rationale: the panel publishes no pulse rating; the committed simulation is for this circuit again (56 mA a pin, R61 3.9k), but backlight.py reads nothing from design.py (DEF-45).
- Source: board/sim/backlight_results.txt (to re-run).
- Method: A, T
- Verified by: VER-05, VER-62
- Phase: EVT
- Status: PROPOSED

### REQ-DSP-07: Backlight headroom
- Level: subsystem
- Board: ref_design
- Statement: At the firmware's maximum duty and VSYS 3.5 V the LED current shall drop by 5% or less from its value at 4.2 V for LED Vf up to 3.2 V, and by 10% or less for Vf up to 3.4 V (the newer datasheet's maximum).
- Rationale: brightness should not sag until the cutoff; at 18.8 mA an LED (56 mA over three) its Vf sits near its 3.2 V typical (rated at 20 mA), and the sink needs about 0.13 V.
- Source: pin_review/panel.json; design_review.md Backlight.
- Method: A, T
- Verified by: VER-05, VER-137, VER-62
- Phase: ORDER
- Status: PROPOSED

### REQ-DSP-08: Backlight loop stability
- Level: subsystem
- Board: ref_design
- Statement: Each sink's loop shall have 45 degrees of phase margin or more at every simulated corner, and an 18% to 95% duty step shall show no ringing on the scope.
- Rationale: an op-amp driving a FET gate can oscillate.
- Source: board/sim/backlight.py (72.5 degrees or more over 720 corners, run for 56 mA a pin and R61 3.9k: this board's values again, DEF-45).
- Method: A, T
- Verified by: VER-05, VER-62
- Phase: EVT
- Status: PROPOSED

### REQ-DSP-09: Backlight off when 3V3 is off
- Level: subsystem
- Board: ref_design
- Statement: With 3V3 off, a VSYS hot plug shall produce less than 5 mA of LED current for less than 10 us.
- Rationale: no flash at plug-in while the device is off.
- Source: board/sim/backlight_results.txt (4.7 mA peak).
- Method: A
- Verified by: VER-05
- Phase: EVT
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
- Statement: A full-screen 320 x 480 redraw, sent to the ILI9488 as 18-bit colour, shall complete in 120 ms or less (Alex sets the number: DEC-13), and a battle-text update in 50 ms or less, with the frame held in 320 KB of RAM or less.
- Rationale: the ILI9488 takes only 18-bit colour over SPI: 461 kB a frame, 92 ms at 40 MHz and 184 ms at 20 MHz, and an 18-bit frame does not fit beside the game; the RGB565 frame is expanded on the fly (DF-180). The 2.4 in panel's RGB565 that took this risk away left again in 79de1f7.
- Source: ILI9488 datasheet (18-bit only over serial); crates/mega_blastoise_fw; validation_plan #180.
- Method: A, T
- Verified by: VER-43, VER-61
- Phase: ORDER
- Status: PROPOSED

### REQ-DSP-12: Backlight halves match at low duty
- Level: subsystem
- Board: ref_design
- Statement: At the lowest duty the firmware uses (18%, pinmap.md), each sink's current shall be within +-25% of its nominal; the minimum duty shall be written in pinmap.md.
- Rationale: U6's 4.5 mV offset is +-20.2% of BL_REF's 22.3 mV there (VER-137); the two halves light one diffuser, so a 25% band per half is not visible as two halves.
- Source: Claude's analysis (O-21); pinmap.md Backlight.
- Method: A, T
- Verified by: VER-137, VER-62
- Phase: ORDER
- Status: PROPOSED

## Subsystem: audio (AUD)

### REQ-AUD-01: Loudness
- Level: subsystem
- Board: ref_design
- Statement: A full-scale 1 kHz sine shall deliver 0.35 W or more into the 8 ohm speaker (4.75 V peak-to-peak across it or more) on a 3.8 V cell.
- Rationale: designed 0.4-0.5 W on the cell, about 0.5 W on USB (VSYS 4.4 V); it clips below a 3.7 V cell.
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
- Statement: The firmware shall cap full scale to the speaker's rating: 0.5 W into 8 ohm for the GSPK151035PN, or the rating of the speaker the release names.
- Rationale: U7 gives about 0.5 W into 8 ohm on USB at full scale and more once it clips toward a square wave (DF-49).
- Source: bom.md (INGHAi GSPK151035PN, C530539, 0.5 W); Claude's review (O-31).
- Method: I
- Verified by: VER-128
- Phase: EVT
- Status: PROPOSED

## Subsystem: buttons (BTN)

### REQ-BTN-01: Contact closes
- Level: subsystem
- Board: both
- Statement: A DS Lite pill pressed on a contact shall measure under 1 kohm on ENIG, and each input shall still read pressed up to 3.3 kohm of pill resistance (ref_design: through the 74HC165 against its 10k pull-up).
- Rationale: carbon pill on gold; a 74HC165's VIL at 3.3 V is about 0.94 V, so against a 10k pull-up a pill reads pressed only under about 3.7 kohm (the RP2350's 50k internal pull-up allowed 10 kohm): DF-164, PLAN.md decision DEC-24.
- Source: validation_plan gate item 5, #42, #137; TI SN74HC165 VIL.
- Method: T
- Verified by: VER-45, VER-56
- Phase: EVT
- Status: PROPOSED
- Notes: measured on a bare board from the same JLC order (F-49); a coupon ordered now is advisory.

### REQ-BTN-02: Contact stays open
- Level: subsystem
- Board: both
- Statement: Leakage between a contact's two combs shall be 100 Mohm or more at 4.2 V, clean and after 24 h at high humidity.
- Rationale: the Start domes SW21 and SW23 carry VSYS on their rings, so leakage turns the device on (REQ-PWR-11); on the others it reads as a press.
- Source: validation_plan gate item 5, #185.
- Method: T
- Verified by: VER-46, VER-79
- Phase: EVT
- Status: PROPOSED

### REQ-BTN-03: Comb geometry
- Level: subsystem
- Board: both
- Statement: The combs shall keep a copper gap of 0.20 mm or more; the worst bridging spot shall be 1.60 mm or less for the 4-ring under the 3.5 mm D-pad and A/B/X/Y pill; one 4.8 mm mask opening shall uncover all comb copper with no paste or silk.
- Rationale: the DS Lite's measured comb; 0.20 mm sits above JLC's minimum spacing. Start and Select are metal domes since 2d1124d (REQ-BTN-08).
- Source: hardware/membrane/README.md; check_contact_fp.py; case/DSLITE.md.
- Method: A
- Verified by: VER-08, VER-10, VER-132, VER-162
- Phase: ORDER
- Status: PROPOSED

### REQ-BTN-04: Keep-out under the pill
- Level: subsystem
- Board: both
- Statement: No via, pour or other copper shall lie within 2.9 mm of a contact's centre except its own two 0.2 mm stubs.
- Rationale: a tented via under a pill lifts it.
- Source: design_review.md Membrane contacts.
- Method: A
- Verified by: VER-132
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

### REQ-BTN-08: Start/Select domes
- Level: subsystem
- Board: ref_design
- Statement: Each 4 mm metal dome (HYP 600-415S-260) shall sit with its rim on its footprint's ring all round at 0.1 mm of placement error, close under its cap's full travel, and rest open with its plunger 0.05 mm or more clear of its apex at the measured Start/Select strip.
- Rationale: a dome pressed at rest under SW21 or SW23 turns the device on in the case for good (DF-167); the strip's plunger height is an estimate (ss_lift).
- Source: design.py SW20-SW23 (Dome_4mm_Round); case.scad ss_metal_dome, ss_lift; hardware/ref_design/PLAN.md row 4.
- Method: A, I
- Verified by: VER-178, VER-180, VER-48
- Phase: CASE
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
- Statement: The Z stack (the tub's floor and pockets, cell, board, parts, speaker, panel, tape, the one-piece top) shall close with 0.2 mm or more of margin at worst-case tolerance including 0.4 mm of cell swelling.
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
- Rationale: a cap that drags or is preloaded feels wrong or sticks; a Start dome held down keeps the device on.
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
- Statement: The case shall survive 20 open/close cycles with its six M2 screws, with 0 cracked keys or grip ribs, 0 stripped bosses and 0 torn cell leads.
- Rationale: the first PLA print's hooks snapped (DEF-15) and the one-piece top's joints are thin printed features (DF-181); service needs opening, and the cell comes out with the tub on its leads (DF-169).
- Source: validation_plan #194; assembly.md steps 9-10.
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
- Statement: The printed top and tub shall have 0 through-openings other than the listed ones (USB-C, pinholes, cap holes, speaker grille, screw holes).
- Rationale: screw pilots and a notch once came through the face (DEF-19).
- Source: 177123c.
- Method: A, I
- Verified by: VER-113, VER-48
- Phase: CASE
- Status: PROPOSED

### REQ-MEC-09: Screws engage
- Level: subsystem
- Board: ref_design
- Statement: Each of the four side screws (H1-H4, under the screen band) shall engage 4.0 mm or more of its pilot above the board and end 0.5 mm or more short of the pilot's end; each of the two end screws (H5, H6, under the button ends) shall engage 3.0 mm or more and end 0.1 mm or more short.
- Rationale: self-tapping screws in printed plastic want 2 to 2.5 x d where the pilot allows it; the end pilots stop 0.6 mm under the ends' face and hold 3.30 mm at most, so one number for all six could never pass (F's objection). M2 x 6 gives 3.2 mm in both today (VER-147); M2 x 7 in the side pilots gives 4.2 mm with the tip 0.8 mm short (DEC-18, DEF-31).
- Source: case.scad; Claude's analysis (O-52).
- Method: A, T
- Verified by: VER-147, VER-92
- Phase: CASE
- Status: PROPOSED

### REQ-MEC-10: Panel stays bonded
- Level: subsystem
- Board: ref_design
- Statement: The panel shall stay bonded in the top's pocket, with no mura and no new contact marks, after 100 presses of 20 N at the glass centre and 24 h at 50 C, on a tape whose part number the release record names.
- Rationale: two players point at and press the shared screen; the panel lies on 0.15 mm tape 0.8 mm over the cell and 0.4 mm over J3 (DF-107).
- Source: Claude's review (O-59).
- Method: T
- Verified by: VER-154
- Phase: EVT
- Status: PROPOSED

### REQ-MEC-11: Connector access
- Level: subsystem
- Board: ref_design
- Statement: Every connector's mouth (J1 with a mated plug, J2's plug and wires, J3's FPC lead-in, J4 with the speaker's plug and its 35 mm leads, J5 with a JST SH cable) shall be clear of parts taller than 1 mm and of every case wall but its own opening, and no pinhole shall fall inside a boss.
- Rationale: J6 once faced J5's back (DEF-20), the pinholes once met a side screw (2f43660) and the USB-C opening was fixed twice.
- Source: Claude's review (F-28).
- Method: A
- Verified by: VER-165
- Phase: ORDER
- Status: PROPOSED

### REQ-MEC-12: MLCCs clear of the screw bosses
- Level: subsystem
- Board: ref_design
- Statement: Every MLCC's pad centre shall be 4.0 mm or more from a screw hole's centre (the tub's 6.0 mm boss plus 1 mm), and 6.0 mm or more for an 0603 or larger.
- Rationale: the six screws clamp the board between bosses; a cracked MLCC on VSYS or VBAT shorts the cell inside the closed case (C9 sat 0.9 mm from a boss at 76955d2).
- Source: Claude's review (O-43); case.scad tub_boss_d.
- Method: A
- Verified by: VER-142
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
- Statement: During a full charge in the closed case, TH1's reading shall stay within 5 C of the cell's hotter point (its centre or its end facing TH1).
- Rationale: TH1 sits on the board 0.80 mm outside the cell hole's right edge, facing the cell's end across the FR4 edge and an air gap (DF-04); the window in REQ-BAT-03 is only as good as this.
- Source: placement.py TH1, CUTOUTS; case.scad; validation_plan #30.
- Method: T, A
- Verified by: VER-87, VER-164
- Phase: EVT
- Status: PROPOSED

### REQ-THM-03: No pause at room temperature
- Level: subsystem
- Board: ref_design
- Statement: At 25 C ambient a full charge in the closed case shall complete with 0 temperature pauses (TS stays at 0.49 V or above, TH1 under 35 C: B3950 at 75 uA).
- Rationale: the board's own heat (U2, U3, the backlight's FETs) must not reach TH1.
- Source: bringup.md step 5.4.
- Method: T
- Verified by: VER-68
- Phase: EVT
- Status: PROPOSED

### REQ-THM-04: Heat budget per mode
- Level: subsystem
- Board: ref_design
- Statement: Charging while playing (5.5 V source, 3.0 V cell, backlight and audio at full scale) shall dissipate 2.0 W or less in total, tabulated per mode (off, cell play, USB play, charging, charging while playing) with each part's position, and the firmware rule the table implies (for example CHG_OFF or a capped volume while the backlight is at full on USB) shall be in the bring-up image before the touch test.
- Rationale: a rough worst case on a 5.5 V source with a 3.0 V cell is about 1.6 W (U2 about 0.66 W, U7 0.4 W, U3 0.17 W, the backlight's FETs 0.15 W at 56 mA a pin) in a 77.4 x 156.64 x 9.6 mm case, beside the glass; the table decides a firmware rule, not copper, so it is due with the bring-up image (round 2's proportionality objection).
- Source: Claude's analysis (O-47, F-48).
- Method: A
- Verified by: VER-144
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
- Notes: not met at be437f8 by the copper to the edge: GND pours 0.20 mm from the cell's and the speaker's holes and J2's mounting pad 0.275 mm from the left edge (VER-18 FAIL, DEF-56; the board's own rule is 0.2 mm). PLAN.md decision DEC-32.

### REQ-FAB-04: Holes
- Level: board
- Board: ref_design
- Statement: Every drilled hole shall be 0.30 mm or larger, and plated and unplated holes shall be in separate drill files (J1's two 0.65 mm pegs, SW20-SW23's 0.5 mm holes and H1-H6 unplated).
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
- Notes: At be437f8 the marker is in neither battler.kicad_pcb nor silk.py (O-R19), so JLC would place its number anywhere (DF-125); and the board's name and BOARD_REV text, where the marker was to go, sit inside the cell's hole (DEF-57). PLAN.md decision DEC-28: both into a clear patch in silk.py (a design change and a new baseline), or JLC's paid 'remove order number' option with this statement restated. VER-132 (k) checks it.


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
- Statement: Every polarised part's pin 1 in JLC's placement preview shall match the board's pin 1, U1, U2, U3, U4, U8, U9 and J3's pin-1 end included, and every CPL position shall be within 0.1 mm of the part's pad centre (J1, J2, J3 and J4 corrected), with 0 unexplained differences.
- Rationale: KiCad's footprint zero differs from JLC's for some parts (DEF-14), and J1's origin is 1.50 mm, J2's and J4's 0.95 mm and J3's 0.23 mm off their pads' centre (VER-176, DEF-33).
- Source: validation_plan #9, #116; 1fd0956.
- Method: I, A
- Verified by: VER-28, VER-29, VER-176
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-08: Reproducible outputs
- Level: board
- Board: ref_design
- Statement: The gerbers, drill, BOM and CPL shall come from the released commit by one release run from a clean worktree; two runs of fab.py shall give identical copper and drill files once their creation dates are stripped; and the uploaded zip's sha256 shall be in the release record.
- Rationale: the board ordered must be the board checked; Alex's main checkout had an uncommitted KiCad GUI change to battler.kicad_pro on 2026-10-24.
- Source: validation_plan #12, #88, #121, gate item 11.
- Method: A, I
- Verified by: VER-143, VER-42, VER-132
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
- Rationale: RP2354B and L2 were out at JLC and LCSC on 2026-10-21; U1 and U3 were changed on 2026-10-22 to parts JLC stocks; the AP3156 and a ProTek SRV05-4 ran out before.
- Source: DEF-12, DEF-13, DEF-29.
- Method: I
- Verified by: VER-41
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-11: Flatness
- Level: board
- Board: both
- Statement: Each board's bow and twist shall be 0.75% or less of its length (1.14 mm over the ref_design board's 151.64 mm; the Edge.Cuts outline).
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
- Statement: Alex shall assemble 1 unit from its board's assembly instructions in 30 min or less, on an ESD mat with a strap, damaging 0 parts.
- Rationale: 30 units by hand.
- Source: validation_plan #86, #161.
- Method: D
- Verified by: allocated: REQ-ASM-01.R
- Phase: EVT
- Status: PROPOSED

### REQ-ASM-01.R: Hand assembly (ref_design)
- Level: board
- Board: ref_design
- Statement: Alex shall assemble 1 ref_design unit from assembly.md in 30 min or less with a screwdriver and a knife and no iron (nothing is soldered at assembly), on an ESD mat with a strap, damaging 0 parts, in the order J3 latched, the cell into the tub's pocket, J2 then J4 plugged, and on opening with J2 unplugged before the tub lifts away.
- Rationale: 30 units by hand; the speaker plugs into J4 since 7da6a90, and the opening order keeps the cell's leads off the hole's edge (DF-169, DF-170).
- Source: docs/assembly.md; validation_plan #86, #161.
- Method: D
- Verified by: VER-91
- Phase: EVT
- Status: PROPOSED


### REQ-ASM-02: Panel seated
- Level: board
- Board: ref_design
- Statement: The panel's FPC shall be fully inserted, square and latched, and pass the display test after 10 case closures.
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
- Verified by: VER-158
- Phase: ORDER
- Status: PROPOSED

### REQ-TST-02: Self-test
- Level: board
- Board: both
- Statement: A self-test image shall check the 20 buttons (16 through the shift registers), GPIO neighbour bridges, panel ID and fills, backlight, an audio tone, the ADC and the charger states, print PASS or FAIL per item, take 5 min or less a unit, and write 0 bytes to OTP.
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
- Statement: Each unit's VBAT gain shall be set in production test at a true 3.50 V from the bench supply (no USB, the self-test at its highest 3V3 load, a DMM checked that session against a 4.096 V reference) and lie within 0.95-1.05; a re-read at 3.50 V shall be within 10 mV; the firmware shall store no gain from a no-cell CHG_STAT_N edge.
- Rationale: the charger's CV is only +-1% (+-42 mV), and with no cell the BQ24073's battery detection toggles CHG (CHG_STAT_N), so a gain from an end-of-charge edge can be off by the cutoff's whole margin (DF-117).
- Source: Claude's review (O-63, O-60).
- Method: T
- Verified by: VER-155, VER-156
- Phase: EVT
- Status: PROPOSED

### REQ-TST-07: Connector mating budget
- Level: board
- Board: ref_design
- Statement: No unit shall ship with J2, J3, J4 or J5 past half its rated mating cycles, counted per unit; no unit used for an FAI or an abuse test shall ship.
- Rationale: PicoBlade and FH12 are rated for roughly 10-30 matings (confirm per maker), bring-up alone can use them up, and J4 is unplugged at every opening (DF-118).
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
