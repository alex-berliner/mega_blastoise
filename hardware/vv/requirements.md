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
re-seeded them on 2026-10-26 for be437f8, on 2026-10-27 for the RP2354B board (71737ae), and on
2026-10-28 for the board being ordered, the ESP32-S3 board at 39cc07f (branch ref-design-esp32s3:
the ESP32-S3-WROOM-1U-N16R8 module with an external flex antenna on a U.FL cable and Wi-Fi on, the
TLV75733P for 3V3, the BQ24073 charger, the 3.5 in ER-TFT035IPS-6 panel (ILI9488) on its 8-bit 8080
bus, a microSD slot, three 74HC165s, a dedicated power key SW24, TH1 a 10k B4460 0805, U10 buffering
VBAT/2, the cell along the board in a 38.2 x 55.2 mm hole, J4 a JST SH header, the speaker sunk in a
hole, a 73.9 x 151.48 mm board in a one-piece top 1.5 mm wider on the left). The radio (RF) and
storage (STO) sections are new with it. A requirement whose subject left
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
- Statement: The device shall read 20 momentary inputs, 10 per seat (up, down, left, right, A, B, X, Y, Start, Select), and the power key, each on its own line, with any combination of up to 21 held at once read correctly.
- Rationale: since 39cc07f all 20 contacts and the power key SW24 come in through three chained 74HC165s (24 bits, U11 the third: pinmap.md Buttons); the Starts are plain buttons, the power key alone turns the device on (Was: both Starts turned it on).
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
- Rationale: about 700 mAh against an estimated 90-120 mA (core up to 50 mA, panel logic 10 mA, backlight about 54 mA at 50% duty, half its 108.2 mA nominal at 100% with R61 3.74k, amplifier 4.4 mA plus sound); the usable capacity to the cutoff is measured per lot (VER-153).
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
- Source: design_review.md's Off state (18-22 uA) predates the BQ24073 (Was: it counts D5's reverse leakage, and D5 left in 2d1124d); VER-103 re-derives the budget for the BQ24073 board (O-R23); validation_plan #33.
- Method: T, A
- Verified by: VER-65, VER-103
- Phase: EVT
- Status: PROPOSED

### REQ-PRD-06: Size
- Level: product
- Board: ref_design
- Statement: The closed case shall measure at most 78.9 x 156.64 mm in plan and 10.7 mm thick at the screen, 7.7 mm at the button ends, each +0.3 mm for print tolerance.
- Rationale: a DS Lite's feel with the width the buttons set and the length the 3.5 in panel sets (hardware/ref_design/PLAN.md, Case).
- Source: hardware/ref_design/README.md; case.scad (case_w, case_l, case_h, deck_face).
- Method: I, A
- Verified by: VER-48, VER-20
- Phase: CASE
- Status: PROPOSED
- Notes: restated at 39cc07f from 77.4 x 156.64 x 9.6 mm: Alex raised the glass 1.1 mm for the module under it (glass_gap 3.5, 2026-10-27: 'raising the glass seems fine') and moved the left wall 1.5 mm out for the microSD slot (grow_l, 2026-10-28); the restated numbers are PROPOSED like the rest. VER-190 reads the outline with grow_l.

### REQ-PRD-07: Single-fault safety
- Level: product
- Board: both
- Statement: No single fault in the electronics (one open, short, adjacent-pin bridge, wrong part or wrong rotation) shall cause fire, cell venting, a part above its material limit, or an accessible surface above IEC 62368-1's single-fault limit for its class (TS2, clause to confirm), with 1 or more independent protection layers left working after the fault.
- Rationale: a Li-ion cell in a printed case held in the hands; the standard sets touch limits for normal operation (REQ-THM-01) and looser ones under a single fault.
- Source: IEC 62368-1 Annex B.4 (single fault conditions) and clause 9 (to confirm); validation_plan #89, #151.
- Method: A, T
- Verified by: VER-98, VER-97, VER-168
- Phase: EVT
- Status: PROPOSED
- Notes: the phase moved to EVT with VER-98 (O-R9: the DFMEA review is advisory at ORDER and binding at G-EVT-16); nothing proves a single-fault limit before that.

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
- Statement: A unit with blank, corrupt or hung firmware shall take a new image over USB-C (the ESP32-S3's USB Serial/JTAG) with the case closed, and with the case open shall enter the ROM's download mode through J7's BOOT and EN pads, 3 out of 3 tries.
- Rationale: the BOOT and RESET buttons and their pinholes left at 39cc07f (the microSD slot took the left bezel; docs/esp32s3_build.md); J7 keeps 3V3, EN, BOOT and GND on pogo pads on the back. An image that turns USB off can only be recovered with the case open (DEC-41).
- Source: docs/esp32s3_build.md; esp32-s3-wroom-1.txt:810-817 (boot mode); design.py J7.
- Method: D, T
- Verified by: VER-212
- Phase: EVT
- Status: PROPOSED

### REQ-PRD-09: Cost
- Level: product
- Board: ref_design
- Statement: The all-in cost per unit at 10 units shall be $45 or less (board, assembly, shipping, panel, cell, speaker, silicone, caps, screws, filament).
- Rationale: the number comes from VER-117's re-run of cost.py with a new board quote at 72.4 x 151.48 mm (73.4 mm across J1's tab; case.scad's board_l, the Edge.Cuts outline); bom.md's board line still asks for 72.4 x 151.64 mm and its price is still the quote for 70.5 x 151.34 mm (DEF-79).
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
- Rationale: charging a Li-ion cell outside its range plates lithium (cold) or speeds failure (hot); TH1's resistance at 25 C and its B are each +-1% (tctr0805.txt:260, the part number's two F's).
- Source: IEC 62133-2 (charging within the cell maker's temperature limits); the BQ24073's TS thresholds from SLUS810N: V_HOT 0.270-0.330 V and V_COLD 2.0-2.2 V at I_TS 72-78 uA (bq24074.txt:823-826, quoted by VER-34); validation_plan #30.
- Method: A, T
- Verified by: VER-32, VER-69, VER-135
- Phase: EVT
- Status: PROPOSED
- Notes: at 39cc07f TH1 is a 10k B4460 0805 (TCTR0805F10K0F4460T, C525375), the highest B among the 10k NTCs JLC stocks, its resistance and B each +-1% (tctr0805.txt:260): charging runs from 5.8 C to 44.5 C at nominal values, 4.3 C to 47.8 C at SLUS810N's limits and 3.9 C to 48.2 C at the worst corner (`design_numbers.py tempwin`, VER-32). Was: B3950, 1.4 C to 51.5 C at be437f8. The worst corner bounds every unit, so it is the 99.9% figure. A 45 C hot edge at the worst corner needs B 5130 or more; no stocked 10k part has it; no series or parallel resistor fixes both edges. PLAN.md decision DEC-05: accept the 48.2 C hot edge for EVT against the cell's datasheet (DF-147), or restate this requirement to the cell's own range if VER-37 shows it runs to 50 C.

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
- Statement: No routed board edge, part body, plug or rib shall press on the cell's faces: 0.5 mm or more between a worst-case cell and the board's hole on every face it can rest against (1.5 mm at its lead end), after the fab, print and fastening tolerances; the tub's pocket at least 0.1 mm a side larger than a worst-case cell after the print's tolerance (0.4 mm in each direction); and no back-side copper within 1 mm of the hole.
- Rationale: a pierced pouch shorts internally and can burn; a pouch pressed into a pocket too small for it is stressed the same way (round 3's O-S1, 2026-10-27: the fit clause, a PROPOSED number).
- Source: validation_plan #27; placement.py CUTOUTS (the hole 37.8 x 54.8 mm since 71737ae); case.scad cell, cell_pocket_box.
- Method: A, I
- Verified by: VER-181, VER-06, VER-48, VER-109, VER-164
- Phase: ORDER
- Status: PROPOSED
- Notes: at 71737ae the gaps are met exactly and the fit is not (VER-181 FAIL): the hole is 37.8 x 54.8 mm with the cell along the board, the pocket 0.9 mm inside it on the long sides and the seat-1 end and 1.9 mm at the lead end, so a worst-case cell is 0.50 mm from the edge (1.50 mm at its lead end) after tolerances wherever it lies, and the pocket's walls are the stop (no tape); but the pocket is 52.0 x 36.0 mm, the worst-case cell's own size, with 0.00 mm spare against the fit clause's 0.40. What passes (design_numbers.py cellgap): a cell of at most 51.6 x 35.6 mm with its PCM by its spec sheet (no copper), or a 55.2 x 38.2 mm hole (copper). Was: at be437f8 the pocket sat 0.25 mm inside a 54.0 x 37.0 mm hole, -0.15 mm after tolerances. PLAN.md decision DEC-27 before the order, with DEC-25.


### REQ-BAT-09: Charge path single faults
- Level: subsystem
- Board: ref_design
- Statement: No single fault in U2, R5, R64, R43, R44, TH1, C3 or the cell's leads shall put more than 4.25 V or more than 1 C on the cell without at least 1 independent layer (the cell's PCM counts as one) still working, and each latent fault that removes a layer shall be found by a per-unit test.
- Rationale: a BQ24073 whose CV or battery FET fails, or whose TS is held in range (DF-145, DF-146, DF-151), charges the cell with only the PCM left.
- Source: validation_plan #89; IEC 62368-1 Annex B.4; IEC 62133-2.
- Method: A, T
- Verified by: VER-98, VER-133, VER-134, VER-135, VER-169, VER-82, VER-88, VER-168
- Phase: EVT
- Status: PROPOSED
- Notes: the phase moved to EVT with VER-98 (O-R9).

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
- Statement: At 25 C ambient, the firmware's low-cell cutoff shall act at a true cell voltage of 3.50 V +0.05/-0.00 V on every unit after its per-unit calibration at 25 C (REQ-TST-06), at the highest 3V3 load the firmware draws, with Wi-Fi on and off.
- Rationale: since 39cc07f the ESP32-S3's ADC1 reads VBAT/2 at 12 dB against its own reference: its total error after Espressif's calibration is +-50 mV at the pin (esp32-s3.txt:3804), +-100 mV of VBAT, measured with Wi-Fi off (3780-3781), and the datasheet does not split offset from gain, so a one-point gain set at charge-done (the charger's CV is +-1%) cannot be shown to meet the window: the per-unit point at 3.50 V does (VER-138, DF-116). U10 buffers the divider, so the pad's leakage does not flow through R17/R18's 235 kohm. Was: the 3V3-referenced ADC and its dropout knee, which left at 39cc07f.
- Source: design.py R17, R18, U10; pinmap.md Battery voltage; esp32-s3.txt:3779-3804 (ADC); mcp6001.txt:135-142; Claude's analysis (O-23, O-49).
- Method: A, T
- Verified by: VER-138, VER-139, VER-67
- Phase: EVT
- Status: PROPOSED
- Notes: U10 (MCP6001) buffers VBAT_DIV onto IO1, so the pad's leakage flows from U10's output: the temperature-varying error is +-0.87 mV of VBAT with pessimistic stand-ins for U10's bias and drift, whose datasheet gives typical figures only (VER-138). The ADC's own +-50 mV at the pin is what the per-unit calibration at 3.50 V (REQ-TST-06) takes out; its drift with temperature and Wi-Fi is VER-139's and VER-67's.

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
- Notes: bringup.md 5.6 still describes the old charger's simulation (DEF-21; Was: the TP4056 design's). u2.json quotes only OVP's 6.6 V nominal: VER-34 takes SLUS810N's minimum into it (pin_review is outside hardware/vv) before VER-12 is built.

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
- Statement: 3V3 shall stay at 3.30 V +-3% (3.20-3.40 V) for VSYS 3.60-4.50 V and loads of 0-620 mA.
- Rationale: the module's VDD33 is 3.0-3.6 V (esp32-s3-wroom-1.txt:1596) and the panel's logic supply 3.3 V maximum; the TLV75733P is +-1.5% over TJ -40 to 125 C (tlv757p.txt:255) and its load regulation 0.060 V/A (260); 620 mA is VER-31's peak in a Wi-Fi TX burst.
- Source: tlv757p.txt:252-260; esp32-s3-wroom-1.txt:1596; ER-TFT035IPS-6 VCI 2.5-3.3 V recommended (er-tft035ips-6.txt:747-748: up to 3.350 V is 50 mV over, DF-96).
- Method: T
- Verified by: VER-83, VER-55
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-05: 3V3 through the unplug
- Level: subsystem
- Board: ref_design
- Statement: Through the USB-to-cell hand-over at full load (backlight 100%, sound playing, Wi-Fi transmitting), 3V3 at the module shall stay at or above 3.0 V for any cell at or above 3.50 V.
- Rationale: the module's VDD33 minimum is 3.0 V (esp32-s3-wroom-1.txt:1596); the BQ24073 joins OUT to BAT through its battery FET with no gap, so 3V3 is the cell less the FET, VSYS's copper to U3 and the TLV75733P's dropout: 2.976 V at 3.50 V in a TX burst (VER-204, DF-187).
- Source: esp32-s3-wroom-1.txt:1596; tlv757p.txt:298; bq24074.txt:724; design.py U2.
- Method: A, T
- Verified by: VER-70, VER-204, VER-208
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
- Rationale: designed under 30 uA at 25 C: 26.8 uA at a 4.2 V cell (VER-103: U2 6.5, U3 1.0 (tlv757p.txt:265), R17 into U10's unpowered input 7.9, U7 8.4 on a stand-in, the FETs' leakage 3.0); leakage rises with temperature (O-51).
- Source: bringup.md step 5.1; the 45 C number is Claude's estimate (O-51).
- Method: T
- Verified by: VER-65, VER-146
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-08: Turn-on margin
- Level: subsystem
- Board: ref_design
- Statement: With 1 kohm in series with the power key SW24 and the cell at 3.50 V, pressing it shall bring 3V3 to 3.20 V or more within 50 ms and PWR_BTN_N shall read low, with Q1's gate margin over its maximum threshold reported in volts.
- Rationale: a dirty or worn contact adds resistance: at VSYS 3.0 V Q1's gate sees 2.88 V through 1 kohm and SW24's 500 mOhm against a 2N7002 Vgs(th) of up to 2.5 V (VER-33; 2n7002_cj.txt:55, rocpu_tps850.txt:137). Was: either Start dome, through Q1 and Q7.
- Source: design.py SW24, R13; 2n7002_cj.txt:55; rocpu_tps850.txt:20, 137; validation_plan section 1 'Latch replica'.
- Method: T, A
- Verified by: VER-211, VER-33
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-09: Turn-off after a reset or crash
- Level: subsystem
- Board: ref_design
- Statement: On the cell with no USB, after a reset on J7's EN pad, a watchdog reset or a panic, PWR_HOLD shall fall below 0.5 V within 10 ms and 3V3 below 0.5 V within 200 ms of the power key being released.
- Rationale: a crashed unit must turn itself off, not drain the cell: IO21 resets as an input with no pull (pinmap.md 'Pins at reset') and R42 4.7k pulls it down (VER-33).
- Source: design.py R42; pinmap.md Power control; esp32-s3.txt:3748.
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
- Statement: With 1 Mohm across the power key SW24 (VSYS to PWR_BTN_RAW) and the cell at 4.2 V, the device shall stay off (3V3 below 0.5 V) for 10 min; the design's sure-on, sure-off and ghost-press bands of contact resistance are those VER-33 prints from U3's VIH and VIL, Q1's threshold, R13, R15 and the diodes' Vf (O-R17), and 1 Mohm shall sit above its sure-off band.
- Rationale: SW24 carries VSYS on one pad all the time; sweat, flux or a pressed cap can bridge it (DF-188). The Starts left the power path at 39cc07f (Was: both Start domes).
- Source: design.py R13, R15, SW24; tlv757p.txt:328-331; 2n7002_cj.txt:55; VER-33's output.
- Method: T, A
- Verified by: VER-211, VER-33
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
- Rationale: USB 2.0's inrush limit is 10 uF or 50 uC at the plug; the old power path's 265 uC came from its diode charging VSYS (Was: the TP4056 design's D5, which left in 2d1124d).
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
- Phase: EVT
- Status: PROPOSED
- Notes: the phase moved to EVT with VER-36 (F-R2(d)). Named exception: (2) the panel's VCI and VDDI at up to 3.333 V (3.350 V at TJ 125 C) against 3.3 V recommended (DF-96). Exception (1), U10's input at VBAT/2 with VDD at 0 V, is inside the MCP6001's own limits since its datasheet is on disk (2 mA, VDD + 1.0 V: mcp6001.txt:115-117, VER-31).

### REQ-PWR-15: 3V3 budget
- Level: subsystem
- Board: ref_design
- Statement: The 3V3 load in a Wi-Fi TX burst shall be 80% or less of U3's 1 A, U3's current limit shall be over the module's 0.5 A supply minimum, and U3's junction shall meet REQ-THM-04's per-mode limits.
- Rationale: since 39cc07f the module's Wi-Fi peaks at 355 mA (esp32-s3-wroom-1.txt:1662) and Espressif asks a 0.5 A supply (1597); the TLV75733P is rated 1 A (tlv757p.txt:209) with a 1.2 A current limit (267); its heat is VER-144's (DF-31).
- Source: tlv757p.txt:209, 267; esp32-s3-wroom-1.txt:1597, 1662-1741.
- Method: A, T
- Verified by: VER-31, VER-83, VER-144
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-17: A faulty USB source
- Level: subsystem
- Board: ref_design
- Statement: A source putting 6-20 V on VBUS (a broken charger, a PD source stuck high) shall over-stress no part: the lowest VBUS that takes a part over its absolute maximum shall be 20 V or more, or the value Alex accepts.
- Rationale: U2 blocks the fault from VSYS (OVP 6.6 V, 28 V absolute), but PWR_EN through R14/D3 takes U3's EN (6 V absolute) over at 9.1 V and EN1 through R43 takes U2's EN1 (7 V) over at 12.4 V (VER-177, with R14 47k and R43 220k since 71737ae; Was: 6.9 V and 9.5 V with 10k and 100k).
- Source: pin_review/u2.json, u3.json; Claude's analysis (O's sign-off objection on the cell path, DF-101).
- Method: A, T
- Verified by: VER-177, VER-97
- Phase: EVT
- Status: PROPOSED
- Notes: DEC-17's BOM half is in design.py at 71737ae (R14 47k, R43 220k: 9.1 V, DEF-50 FIXED); 20 V needs a clamp (copper); proposed: Alex accepts 9.1 V for EVT as this statement's value. The decision is made before the order, because DEF-50 keeps Gate ORDER and closes on it; the check (VER-177) binds at G-EVT-04, so an acceptance for EVT does not leave a failing ORDER check behind (O-R9; F-R2(f) wanted VER-177 at ORDER, but DEF-50's Gate already holds the BOM decision there).

### REQ-PWR-18: The power key
- Level: subsystem
- Board: ref_design
- Statement: The power key SW24 shall turn the device on with a press held 1 s, leave it off after a press under 1 s, and turn it off with a long press; its cap shall sit 0.4-0.6 mm under the deck's face and need 300 gf or less, and nothing pressing the case face down shall press it.
- Rationale: the dedicated key since 39cc07f (Alex, 2026-10-28): a ROCPU TPS850 J14K (250 +-50 gf, 0.2 +0.1 mm, rocpu_tps850.txt:20, 33) under a printed cap (case.scad pwr_cap); pinmap.md's Power control gives the hold rule.
- Source: design.py SW24; case.scad pwr_*; rocpu_tps850.txt:17-33; pinmap.md Power control.
- Method: T
- Verified by: VER-211, VER-213, VER-33
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-19: A jig on J7 does not back-feed U3
- Level: subsystem
- Board: ref_design
- Statement: No procedure shall drive J7's 3V3 pad while JP1 is bridged and VSYS is 0 V; the bring-up and jig procedures shall say how J7 is powered.
- Rationale: U3's OUT over its IN + 0.3 V is over the TLV75733P's absolute maximum and runs current back through its body diode (tlv757p.txt:180, 1137-1138); design.py's J7 note invites exactly that (DF-189).
- Source: tlv757p.txt:180, 1129-1142; design.py J7; VER-159.
- Method: A, I
- Verified by: VER-159, VER-212
- Phase: EVT
- Status: PROPOSED

## Subsystem: core (MCU)

### REQ-MCU-01: Boot from blank
- Level: subsystem
- Board: ref_design
- Statement: A blank board on USB with no cell shall enumerate as the ESP32-S3's USB Serial/JTAG within 5 s and take an image over it.
- Rationale: proves the module, its supply, EN and USB in one step (the module carries flash, PSRAM and crystal).
- Source: esp32-s3-wroom-1.txt:810-837 (boot modes); bringup.md step 1.
- Method: T
- Verified by: VER-55
- Phase: EVT
- Status: PROPOSED

### REQ-MCU-02: USB full speed
- Level: subsystem
- Board: ref_design
- Statement: D+ and D- shall each pass a series resistor of 22-33 ohm placed within 5 mm of the module's pin, stubs shall be 3.5 mm or shorter, and the device shall enumerate on 3 different hosts.
- Rationale: Espressif's guidelines: 'initial value can be 22/33', 'close to the chip' (esp32-s3-hw-design-guidelines.txt:873-875, 1307); at 39cc07f they sit 11.5 mm away (VER-191, DF-198). Was: the RP2350's 27 ohm.
- Source: esp32-s3-hw-design-guidelines.txt:873-875, 1304-1313; pin_review/j1.json.
- Method: A, T
- Verified by: VER-191, VER-108, VER-80
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
- Status: RETIRED
- Retired: RETIRED in 39cc07f: the module carries its own 40 MHz crystal; X1, C27, C28 and R20 left with the RP2354B

### REQ-MCU-04: Debug ports
- Level: subsystem
- Board: ref_design
- Statement: Firmware shall flash and log over USB-C through the ESP32-S3's USB Serial/JTAG at 1 line or more a second, and J7's pads (3V3, EN, BOOT, GND) shall reset the module and enter its download mode from a pogo jig.
- Rationale: bring-up and recovery (Was: SWD on J5 and J7, the UART header); the module's USB Serial/JTAG carries both the flash and the log (docs/esp32s3_board_proposal.md item 7).
- Source: docs/esp32s3_board_proposal.md; design.py J7; esp32-s3-wroom-1.txt:860-871.
- Method: T
- Verified by: VER-212, VER-02
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
- Statement: Every module pin shall carry only a function that pin has in the module's Table 3-1, ADC nets on ADC1 channels (Wi-Fi on), USB on IO19 and IO20, IO35-IO37 unconnected, and no load-switching output on a pin with a high-level power-up glitch.
- Rationale: a pin that cannot do its job is a respin; ADC2 cannot run with Wi-Fi (esp32-s3.txt:3415); the N16R8's PSRAM owns IO35-IO37 (esp32-s3-wroom-1.txt:691-693); Table 2-2 lists the glitching pins (esp32-s3.txt:958-1006). Was: the RP2350's function table and PWM slices, VER-15.
- Source: esp32-s3-wroom-1.txt:629-696; esp32-s3.txt:958-1006, 3415.
- Method: A
- Verified by: VER-203
- Phase: ORDER
- Status: PROPOSED
- Notes: the module pin, GPIO and net table is VER-203's evidence; the GPIO matrix routes the panel, the SD bus and the LEDC to any pin, so only ADC1 and USB are fixed (docs/esp32s3_board_proposal.md 'Pin map').

### REQ-MCU-07: Pull-downs for E9
- Level: subsystem
- Board: ref_design
- Statement: Every GPIO that enables power or a load (PWR_HOLD, CHG_OFF, AMP_EN, BL_PWM) shall have an external pull-down of 8.2 kohm or less, unless the placed stepping is shown not to have erratum E9.
- Rationale: on A2 silicon a released pad with its input enabled can sit near 2 V.
- Source: RP2350 erratum E9 (pin_review/u4.json pins 9 and 55); validation_plan #41, #178, #179.
- Method: A, T
- Verified by: VER-02, VER-75
- Phase: ORDER
- Status: RETIRED
- Notes: PWR_HOLD (R42 4.7k) and CHG_OFF (R44 4.7k) meet it; AMP_EN (R37 100k) and BL_PWM on GPIO28 (R60 + R61, 103.74k since R61 3.74k) do not: DF-45, DF-47. Decision for Alex (PLAN.md decision DEC-11): 4.7k pull-downs (BOM only), or rely on firmware and the placed stepping (VER-75).
- Retired: RETIRED in 39cc07f: erratum E9 left with the RP2350; the ESP32-S3's strapping pins at reset are REQ-MCU-12's

### REQ-MCU-08: Chip identity
- Level: subsystem
- Board: ref_design
- Statement: Each unit shall report an ESP32-S3 with 16 MB of flash and 8 MB of PSRAM at 3.3 V VDD_SPI (the N16R8), and its chip revision, MAC and eFuse summary shall be recorded at first boot (1 record per unit).
- Rationale: a substituted N16R16VA runs its PSRAM at 1.8 V and a WROOM-1 has a PCB antenna; the chip revision decides which errata apply (VER-214).
- Source: esp32-s3-wroom-1.txt:130-171.
- Method: T
- Verified by: VER-210
- Phase: EVT
- Status: PROPOSED

### REQ-MCU-09: Core supply
- Level: subsystem
- Board: ref_design
- Statement: The module's 3V3 pin shall have 22 uF and 0.1 uF within 5 mm, EN a 10k and 1 uF RC that rises past VIH_nRST after the rails, and 3V3 at the pin shall stay inside VDD33's 3.0-3.6 V through a Wi-Fi TX burst at the cutoff.
- Rationale: Espressif's peripheral schematic and guidelines (esp32-s3-wroom-1.txt:2875; esp32-s3-hw-design-guidelines.txt:299-303, 338); at 39cc07f C10 sits 23.6 mm from pin 2 and 3V3 reaches 2.976 V in a burst at 3.50 V (VER-204, DF-187). Was: the RP2350's 1V1 core supply.
- Source: esp32-s3-wroom-1.txt:931, 1596, 2875; esp32-s3-hw-design-guidelines.txt:299-303, 316-338.
- Method: A, T
- Verified by: VER-204, VER-208
- Phase: EVT
- Status: PROPOSED

### REQ-MCU-10: Flash budget
- Level: subsystem
- Board: ref_design
- Statement: The firmware image shall fit its app partition in the module's 16 MB flash at 80% or less, with a written partition table (bootloader, app, settings, battle log, game data), checked in scripts/check.sh on every change.
- Rationale: the 2 MB of the MCU before it was the reason for the change (docs/esp32s3_board_proposal.md); the partition table is what can still run out.
- Source: docs/esp32s3_board_proposal.md; esp32-s3-wroom-1.txt:130-150.
- Method: A
- Verified by: VER-216, VER-185
- Phase: EVT
- Status: PROPOSED
- Notes: the phase moved from ORDER to EVT at 39cc07f: with 16 MB the order freezes no flash decision (G-ORD-23 retired).

### REQ-MCU-11: A bring-up image for the ordered chip
- Level: subsystem
- Board: ref_design
- Statement: Before the boards arrive, a self-test image shall build for the ESP32-S3 (esp-hal) from pin definitions generated from design.py, with 0 hand-typed GPIO numbers.
- Rationale: the image is needed the day the boards arrive (round 2's proportionality objection); the GPIO choices that copper freezes are VER-203's.
- Source: docs/esp32s3_board_proposal.md 'Firmware with Wi-Fi'; Claude's review (F-44, O-37, O-56).
- Method: A
- Verified by: VER-149, VER-215
- Phase: EVT
- Status: PROPOSED

### REQ-MCU-12: Strapping pins at reset
- Level: subsystem
- Board: ref_design
- Statement: At every reset the module shall read IO0 high (SPI boot), IO45 low (VDD_SPI 3.3 V for the N16R8's flash and PSRAM), IO46 low, and IO3 driven, from the board's resistors and drivers alone, with no eFuse burnt that changes them.
- Rationale: a high IO45 runs the 3.3 V flash and PSRAM at 1.8 V: no boot; IO0 low is the download mode (esp32-s3-wroom-1.txt:714-754, 810-899); AMP_EN and CHG_OFF share IO45 and IO46.
- Source: esp32-s3-wroom-1.txt:748-754, 816-817, 849, 876-899; design.py R37, R44.
- Method: A, T
- Verified by: VER-203, VER-55
- Phase: ORDER
- Status: PROPOSED

### REQ-MCU-13: The module's ambient
- Level: subsystem
- Board: ref_design
- Statement: The air round the module shall stay at or under 65 C (the N16R8's ambient limit) in every mode at 40 C outside, or the firmware shall enable the PSRAM's ECC (85 C).
- Rationale: the N16R8 and N16R16V modules' octal PSRAM limits them to 65 C ambient, 85 C with ECC on and 1/16 of the PSRAM lost (esp32-s3-wroom-1.txt:102-105, 1598); at 39cc07f VER-193 puts it at 84.9 C on a stand-in (DF-193).
- Source: esp32-s3-wroom-1.txt:102-105, 1598.
- Method: A, T
- Verified by: VER-193, VER-86
- Phase: EVT
- Status: PROPOSED

### REQ-MCU-14: The module's errata
- Level: subsystem
- Board: ref_design
- Statement: Every erratum Espressif lists for the ESP32-S3's chip revision on the board shall be answered for this design: applies or not, and its fix or firmware rule.
- Rationale: the errata table of the MCU before it set two BOM values (Was: VER-16); the ESP32-S3's errata may set firmware rules.
- Source: Espressif's ESP32-S3 errata (not on disk: VER-214 BLOCKED).
- Method: I
- Verified by: VER-214
- Phase: EVT
- Status: PROPOSED

## Subsystem: display (DSP)

### REQ-DSP-01: Panel interface
- Level: subsystem
- Board: ref_design
- Statement: The panel shall run the 8080 8-bit bus (IM2-0 = 011, as design.py straps them, CSX tied low, RDX high) and return 0x94 0x88 for ID4 (0xD3).
- Rationale: proves the interface straps and the bus; since 39cc07f the panel is on the 8-bit 8080 bus (er-tft035ips-6.txt:718; docs/display_parallel_proposal.md). Was: 4-wire SPI.
- Source: er-tft035ips-6.txt:709-722; design.py PANEL.
- Method: T
- Verified by: VER-43, VER-60
- Phase: ORDER
- Status: PROPOSED

### REQ-DSP-02: Write speed
- Level: subsystem
- Board: ref_design
- Statement: 8080 writes at 25 MHz or less shall draw 10 full 320 x 480 frames of a checker pattern with 0 corrupted pixels.
- Rationale: the ILI9488's parallel write cycle is 40 ns (twc, ili9488.txt:16159), so 25 MHz is the limit with 0 ns spare (VER-199, DF-191); RGB565 is two bytes a pixel over the 8-bit bus.
- Source: ili9488.txt:16159-16172; pinmap.md Display.
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
- Notes: the map is a fold, not a mirror: once the contacts face the connector's contacts, the tail's contact strip can meet a bottom-contact FH12-50S in only one way, and the product's and the loose breakout's panel-side pairing stays 51-j (F-R1). What is not proved is the bought breakout's header numbering: VER-43 finds it by diode test (which header pin is panel 1), checks the GND pins' continuity, records the header pin each panel pin lands on and wires the dev board by the panel's own pin names (1 LEDA, 2-3 LEDK, 7-9 IM, 10 RESET, 33 SDO, 34 SDA, 36 SCL, 37 D/CX, 38 CSX, 40-42 VDDI/VCI, 11-32 GND). VER-136 adds the fold's reach and seating, not a different map.

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
- Statement: At the firmware's highest duty the six LEDs shall draw 120 mA or less in total at worst-case tolerance (3V3 +2%, R60/R61 1%, U6 offset 4.5 mV, R52/R53 -1%) for VSYS 3.5-4.5 V; at nominal parts and 100% duty they draw 108 mA +-5% (54 mA a cathode pin, the two pins within 5% of each other).
- Rationale: the panel's 120 mA (6 x 20 mA); with R61 3.74k (71737ae, DEC-31) the worst case at 100% is 117.7 mA (VER-137). Was: 122.4 mA with R61 3.9k at be437f8.
- Source: ER-TFT035IPS-6 section 4.4 (pin_review/panel.json); design.py backlight section; Claude's analysis (F-7, O-21).
- Method: T, A
- Verified by: VER-62, VER-137, VER-05
- Phase: ORDER
- Status: PROPOSED

### REQ-DSP-06: Backlight transient
- Level: subsystem
- Board: ref_design
- Statement: The LED current shall exceed 120 mA in total for 5 us or less at any event, a USB plug-in while lit included (VSYS from the cell's 3.5-4.2 V up to 4.4 V).
- Rationale: the panel publishes no pulse rating; the committed simulation models R61 3.9k (0.124 V, 56 mA a pin), not the board's 3.74k on BL_PWM (IO9's LEDC: 0.119 V, 54.1 mA a pin, 117.2 mA worst case for the six LEDs), and reads nothing from design.py (DEF-45, VER-05).
- Source: board/sim/backlight_results.txt (to re-run).
- Method: A, T
- Verified by: VER-05, VER-62
- Phase: EVT
- Status: PROPOSED

### REQ-DSP-07: Backlight headroom
- Level: subsystem
- Board: ref_design
- Statement: At the firmware's maximum duty and VSYS 3.5 V the LED current shall drop by 5% or less from its value at 4.2 V for LED Vf up to 3.2 V, and by 10% or less for Vf up to 3.4 V (the newer datasheet's maximum).
- Rationale: brightness should not sag until the cutoff; at 18.0 mA an LED (54.1 mA a pin over three, nominal, R61 3.74k) its Vf sits near its 3.2 V typical (rated at 20 mA), and the sink needs about 0.13 V.
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
- Source: board/sim/backlight.py (72.5 degrees or more over 720 corners, run for R61 3.9k and 56 mA a pin; to re-run for 71737ae's R61 3.74k and 54.1 mA a pin once it reads design.py: DEF-45, VER-05).
- Method: A, T
- Verified by: VER-05, VER-62
- Phase: EVT
- Status: PROPOSED

### REQ-DSP-09: Backlight off when 3V3 is off
- Level: subsystem
- Board: ref_design
- Statement: With 3V3 off, a VSYS hot plug shall produce less than 5 mA of LED current for less than 10 us.
- Rationale: no flash at plug-in while the device is off.
- Source: board/sim/backlight_results.txt (4.7 mA peak, an older circuit); since 2026-10-28 VER-05 prints only an estimate (the gate's kick through Crss turns on the AO3400A's capacitance curve, a figure, and on how far the unlit LEDs let the drain move), so VER-62 measures it.
- Method: T
- Verified by: VER-62
- Phase: EVT
- Status: PROPOSED

### REQ-DSP-10: Tearing-effect signal
- Level: subsystem
- Board: ref_design
- Statement: With TEON set, IO10 (LCD_TE) shall see one TE pulse per frame at 60 Hz +-10%.
- Rationale: firmware starts frames on it to avoid tearing.
- Source: bringup.md display step 6.
- Method: T
- Verified by: VER-63
- Phase: EVT
- Status: PROPOSED

### REQ-DSP-11: Redraw time
- Level: subsystem
- Board: ref_design
- Statement: A full-screen 320 x 480 redraw shall complete in 20 ms or less (60 fps with a frame in the PSRAM), and a battle-text update in 5 ms or less.
- Rationale: the 8080 bus at 25 MHz moves an RGB565 frame in 12.3 ms of bus time (18.4 ms in RGB666, VER-199); the PSRAM holds the 307,200 B frame (docs/esp32s3_board_proposal.md). Was: 220 ms over SPI.
- Source: ili9488.txt:16159; docs/display_parallel_proposal.md; Claude's estimate (Alex sets the numbers: DEC-13).
- Method: A, T
- Verified by: VER-43, VER-61
- Phase: ORDER
- Status: PROPOSED
- Notes: PLAN.md decision DEC-13 was taken in the design: the 8080 8-bit bus (docs/display_parallel_proposal.md, 39cc07f); Alex sets the two numbers.

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
- Statement: The 312.5 kHz PWM carrier at the speaker shall be 40 dB or more below the full-scale 1 kHz level.
- Rationale: the LEDC runs from the 80 MHz APB clock: 8 bits is 312.5 kHz (pinmap.md Audio); R32 and R33 1.5k (39cc07f) give 44.5 dB nominal and 42.6 dB at the worst corner (VER-100). Was: 586 kHz from the RP2350.
- Source: design_review.md Audio; ad291a8.
- Method: T, A
- Verified by: VER-64, VER-100
- Phase: EVT
- Status: PROPOSED

### REQ-AUD-03: Bandwidth
- Level: subsystem
- Board: ref_design
- Statement: The audio chain shall be -3 dB at 10 kHz or higher and at 50 Hz or lower.
- Rationale: a 15 mm speaker reproduces little above 10 kHz; since 39cc07f the 1.5k filter gives 9.7 kHz nominal and 8.2 kHz at the worst corner (VER-100: FAIL), while design.py's comment says 6.7 kHz (its sim): Alex restates the number or the filter changes (DEF-93).
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
- Statement: A DS Lite pill pressed on a contact shall measure under 1 kohm on ENIG, and each input shall still read pressed up to 15 kohm of pill resistance (ref_design: through the 74HC165 against its 47k pull-up).
- Rationale: carbon pill on gold; a 74HC165's VIL at 3.3 V is about 0.94 V, so against a 47k (+-5%) pull-up (RN1-RN4 since 71737ae, DEC-24) a pill reads pressed under about 17 kohm (Was: the RP2350's 50k internal pull-up allowed 10 kohm; 3.7 kohm with 10k at be437f8): DF-164.
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
- Rationale: the power key SW24 carries VSYS on one pad, so leakage across it turns the device on (REQ-PWR-11); on the others it reads as a press.
- Source: validation_plan gate item 5, #185.
- Method: T
- Verified by: VER-46, VER-211
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

### REQ-BTN-09: The 24-bit chain
- Level: subsystem
- Board: ref_design
- Statement: The three 74HC165s (U8, U9, U11) shall read all 24 bits in pinmap.md's order with every input pulled up or tied, at an SR_CLK of 1 MHz or less with 0 errors in a million reads.
- Rationale: U11 joined the chain at 39cc07f with the Starts, the Selects, the power key's read and the charger's status (pinmap.md Buttons); SR_CLK runs 185.8 mm to three loads (VER-199).
- Source: design.py U8, U9, U11; sn74hc165.txt:371-504; pinmap.md Buttons.
- Method: A, T
- Verified by: VER-198, VER-199, VER-56, VER-179
- Phase: EVT
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
- Rationale: a cap that drags or is preloaded feels wrong or sticks; the power key's cap held down keeps the device on.
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
- Statement: The printed top and tub shall have 0 through-openings other than the listed ones (USB-C, the microSD opening, the power key's hole, cap holes, speaker grille, screw holes).
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
- Statement: Every connector's mouth (J1 with a mated plug, J2's plug and wires, J3's FPC lead-in, J4 with the speaker's JST SH plug and its 35 mm leads) shall be clear of parts taller than 1 mm and of every case wall but its own opening, and J8's body shall clear the left wall's SD opening by 0.2 mm a side.
- Rationale: the USB-C opening was fixed twice; the SD slot's body is 13.85 mm wide in a 14.0 mm opening at 39cc07f (VER-165, DF-195). Was: J5 with a JST SH cable, and the pinholes.
- Source: Claude's review (F-28).
- Method: A
- Verified by: VER-165
- Phase: CASE
- Status: PROPOSED
- Notes: the phase moved to CASE with VER-165 (F-R9: the connector mouths are a case matter; a case defect does not hold the order back).

### REQ-MEC-12: MLCCs clear of the screw bosses
- Level: subsystem
- Board: ref_design
- Statement: Every MLCC's pad centre shall be 4.0 mm or more from a screw hole's centre (the tub's 6.0 mm boss plus 1 mm), and 6.0 mm or more for an 0603 or larger.
- Rationale: the six screws clamp the board between bosses; a cracked MLCC on VSYS or VBAT shorts the cell inside the closed case (an MLCC on VSYS sat 0.9 mm from a boss at 76955d2; Was: C9).
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
- Rationale: TH1 sits on the board 1.05 mm outside the cell hole's right edge (an 0805 since 71737ae), 1.95 mm or more from the cell's long side across the FR4 edge and an air gap, the pocket keeping the cell 0.9 mm inside the edge (DF-04); the window in REQ-BAT-03 is only as good as this.
- Source: placement.py TH1, CUTOUTS; case.scad; validation_plan #30.
- Method: T, A
- Verified by: VER-87, VER-164
- Phase: EVT
- Status: PROPOSED

### REQ-THM-03: No pause at room temperature
- Level: subsystem
- Board: ref_design
- Statement: At 25 C ambient a full charge in the closed case shall complete with 0 temperature pauses (TS stays at 0.46 V or above, TH1 under 35 C: B4460 at 75 uA).
- Rationale: the board's own heat (U2, U3, the backlight's FETs) must not reach TH1.
- Source: bringup.md step 5.4.
- Method: T
- Verified by: VER-68
- Phase: EVT
- Status: PROPOSED

### REQ-THM-04: Heat budget per mode
- Level: subsystem
- Board: ref_design
- Statement: Charging while playing with Wi-Fi on (5.5 V source, 3.0 V cell, backlight and audio at full scale) shall dissipate 2.0 W or less in total, tabulated per mode (off, cell play, USB play, charging, charging while playing) with each part's position, and the firmware rule the table implies (CHG_OFF, a capped volume, a lower CPU clock or modem sleep) shall be in the bring-up image before the touch test.
- Rationale: at 39cc07f the worst mode is 3.31 W inside the case at VER-31's Wi-Fi load (VER-144: U3 0.43 W, U2 0.80 W, the module 0.74 W), with the air round the module over its 65 C (DF-193); the table decides a firmware rule or copper (DEF-87).
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
- Statement: Before any sale, the device shall carry the module's FCC ID with an antenna the grant lists, and the rest of it shall pass FCC Part 15 Class B / CISPR 32 Class B emissions with 3 dB of margin; in DVT a near-field pre-scan shall name the 5 largest peaks and their sources.
- Rationale: since 39cc07f the device transmits (Wi-Fi): it relies on the WROOM-1U's modular grant, kept only with an antenna type the grant lists (docs/esp32s3_board_proposal.md 'Open checks' 5).
- Source: FCC 47 CFR Part 15 Subpart B; CISPR 32; validation_plan #67, #162.
- Method: T
- Verified by: VER-96, VER-118, VER-217
- Phase: DVT
- Status: PROPOSED

## Subsystem: radio (RF), since 39cc07f

### REQ-RF-01: The antenna's keep-out on the board
- Level: subsystem
- Board: ref_design
- Statement: The board shall keep a copper-free rule area on both layers under the flex antenna's plate (no track, via, pour or footprint): 0 copper items inside, and the area no more than 1.0 mm short of the plate's outline at any end on the board's plane.
- Rationale: copper under an antenna detunes it (esp32-s3-hw-design-guidelines.txt:1290-1299); at 39cc07f the rule area is x board edge to 71.4, y 62.6-107.0, 0 copper inside, 0.5 and 0.9 mm short of the plate's ends (VER-205).
- Source: esp32-s3-hw-design-guidelines.txt:1281-1301; placement.py ANTENNA_KEEPOUT; docs/esp32s3_build.md.
- Method: A
- Verified by: VER-205
- Phase: ORDER
- Status: PROPOSED

### REQ-RF-02: The antenna fitted and clear of metal
- Level: subsystem
- Board: ref_design
- Statement: The flex antenna and its U.FL cable shall be bought with the boards, fixed on the case's left wall as assembly.md says, and kept the clearance its maker's datasheet gives from metal (the panel, the cell, the screws), 15 mm until that datasheet is on disk.
- Rationale: a WROOM-1U has no antenna of its own (esp32-s3-wroom-1.txt:100); at 39cc07f the panel's outline is 1.3 mm from the plate, the cell 10.6 mm, H2's boss over its end, and bom.md buys no antenna (VER-205, VER-167; DF-192, DF-197).
- Source: esp32-s3-wroom-1.txt:100; esp32-s3-hw-design-guidelines.txt:1299; the antenna's datasheet (not on disk).
- Method: A, I
- Verified by: VER-205, VER-167, VER-48
- Phase: CASE
- Status: PROPOSED

### REQ-RF-03: Wi-Fi range
- Level: subsystem
- Board: ref_design
- Statement: In its closed case with a cell, held or face up, the device shall keep an RSSI of -70 dBm or better from an access point 10 m away line of sight, within 10 dB of the same antenna in free air, and hold a Showdown battle for 30 min with no dropped connection.
- Rationale: Wi-Fi is the reason for the module (docs/esp32s3_board_proposal.md 'Radio'); the numbers are Claude's proposal for Alex to set.
- Source: Claude's estimate; docs/esp32s3_board_proposal.md.
- Method: T
- Verified by: VER-207
- Phase: EVT
- Status: PROPOSED

### REQ-RF-04: The radio's grant
- Level: subsystem
- Board: ref_design
- Statement: Before any unit is sold or given away, the antenna fitted shall be of a type and gain the WROOM-1U's FCC grant lists, and the label shall carry the module's FCC ID.
- Rationale: an intentional radiator relies on the module's modular grant only with a listed antenna (docs/esp32s3_board_proposal.md 'Open checks' 5).
- Source: docs/esp32s3_board_proposal.md; the module's FCC grant (not on disk).
- Method: I
- Verified by: VER-217
- Phase: SALE
- Status: PROPOSED

## Subsystem: storage (STO), the microSD slot, since 39cc07f

### REQ-STO-01: The microSD bus
- Level: subsystem
- Board: ref_design
- Statement: The microSD slot J8 shall read and write a card at 4 bits with 10k pull-ups on SD_CMD and SD_D0 to SD_D3, card detect low with a card in, at 20 MHz and 40 MHz with 0 errors over 4 MB.
- Rationale: the slot is new at 39cc07f (docs/esp32s3_board_proposal.md item 5); Espressif asks pull-ups on the SDIO lines (esp32-s3-hw-design-guidelines.txt:864-865); the DM3AT's detect switch is normally open (hirose_dm3_catalog_C114218.txt:27).
- Source: design.py J8, R79-R84; esp32-s3-hw-design-guidelines.txt:861-865; hirose_dm3_catalog_C114218.txt:26-27.
- Method: A, T
- Verified by: VER-198, VER-209
- Phase: EVT
- Status: PROPOSED

### REQ-STO-02: A card plugged in while running
- Level: subsystem
- Board: ref_design
- Statement: Inserting a microSD card while the device runs shall leave 3V3 at the module at or over 3.0 V and cause no reset.
- Rationale: J8's VDD comes straight off 3V3 through JP6 with no load switch: a card over 1.8 uF of its own dips 3V3 under 3.0 V at once (VER-206 on stand-ins, DF-194).
- Source: esp32-s3-wroom-1.txt:1596; tlv757p.txt:267; VER-206.
- Method: A, T
- Verified by: VER-206, VER-209
- Phase: EVT
- Status: PROPOSED

### REQ-STO-03: The SD opening in the case
- Level: subsystem
- Board: ref_design
- Statement: With the case closed a card shall go in and latch, and come out, 20 of 20 times, with the slot's body clear of the left wall's opening by 0.2 mm a side.
- Rationale: the opening is 14.0 mm over the slot's 13.85 mm body at 39cc07f (VER-165, DF-195).
- Source: case.scad sd_y, sd_z; hirose_dm3_catalog_C114218.pdf.
- Method: A, T
- Verified by: VER-165, VER-48
- Phase: CASE
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
- Notes: the copper to the edge is met in the data at 71737ae: 0.308 mm at the closest, KiCad's edge clearance 0.31 mm and a 0.30 mm copper-to-NPTH rule (battler.kicad_dru), VER-18 PASS; at be437f8 GND pours sat 0.20 mm from both holes. As built, with the 0.2 mm outline tolerance (ROUTE_TOL), the closest is 0.108 mm, and the narrowest FR4 web 1.00 mm can be 0.60 mm (O-R2): whether 0.30 mm is a data rule or an as-built one is the dated page's to say (VER-17, DEF-56). The dated page and the vias' 0.10 mm ring are still open.

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
- Notes: at 71737ae silk.py places the board's name, BOARD_REV and the build line on clear spots on the back (VER-18: every text on the board), but the marker is still in neither battler.kicad_pcb nor silk.py (O-R19), so JLC would place its number anywhere (DF-125, DEF-55). PLAN.md decision DEC-28: the marker in silk.py (a design change and a new baseline), or JLC's paid 'remove order number' option with this statement restated. VER-132 (k) checks it.


### REQ-FAB-06: Part identity
- Level: board
- Board: both
- Statement: Every LCSC number shall resolve to the MPN, package, pin count and rating design.py intends, with 0 substitutions on the order.
- Rationale: the right family in the wrong variant passes every other check.
- Source: validation_plan #2, #122.
- Method: A, I
- Verified by: VER-14, VER-186, VER-130, VER-28, VER-41
- Phase: ORDER
- Status: PROPOSED
- Notes: VER-14 is the by-hand pass at ORDER (every line of JLC's matched-parts page against design.py, saved in VER-28's evidence); the script over the parts database is VER-186's, at PVT (F-R9).

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
- Statement: Every polarised part's pin 1 in JLC's placement preview shall match the board's pin 1, U1, U2, U3, U4, U8, U9 and J3's pin-1 end included, and every CPL position shall be within 0.1 mm of the part's pad centre, or the reference's origin offset shall be recorded in a JLC_FIXED entry that names the placement preview screenshot and the offset JLC's library applies (J1, J2, J3 and J4: 1.50, 0.95, 0.95 and 0.23 mm, which may be zero), with 0 unexplained differences.
- Rationale: KiCad's footprint zero differs from JLC's for some parts (DEF-14), and J1's origin is 1.50 mm, J2's and J4's 0.95 mm and J3's 0.23 mm off their pads' centre (VER-176, DEF-33). Whether JLC's own library places them off their pads is unknown until the placement preview (O-R7): no CPL row moves before the dry run, and a per-reference correction table in fab.py is filled only from a JLC_FIXED entry that names the preview screenshot and the offset JLC's library applies, which may be zero. Such an entry counts only when its evidence file is committed and names the designator (F-R4).
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
- Phase: DVT
- Status: PROPOSED
- Notes: the phase moved to DVT with VER-26 (O-R9: the e-test is advisory at ORDER, with the default "send no netlist", and binding at G-DVT-01). JLC e-tests against its own Gerber netlist, and an IPC-356 file without merged net ties would only trigger false shorts (F-R9).

### REQ-FAB-10: Stock
- Level: board
- Board: both
- Statement: On order day every BOM line shall have stock of 1.1 x the order quantity plus attrition, or be pre-ordered or consigned; the module (C3013946), the TLV75733P, the DM3AT and the TPS850 shall never be substituted.
- Rationale: the new parts' JLC class and stock were not checked by the board session (docs/esp32s3_build.md, 'Not verified'); a substituted module can carry 1.8 V PSRAM (DF-69, DF-70). Was: the RP2354B and L2.
- Source: DEF-12, DEF-13, DEF-29.
- Method: I
- Verified by: VER-41
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-11: Flatness
- Level: board
- Board: both
- Statement: Each board's bow and twist shall be 0.75% or less of its length (1.14 mm over the ref_design board's 151.48 mm at 71737ae; the Edge.Cuts outline).
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
- Verified by: VER-110, VER-210
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
- Statement: No shipped unit shall have any connector (J2, J3, J4, J8) mated more than half its rated cycles, counted per unit.
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
