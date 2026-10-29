# Requirements

What the handheld must do, as numbers a check can pass or fail. Each requirement has a
value and a tolerance, why it is that number, where the number comes from, how it is
proven (T test, A analysis, I inspection, D demonstration), the checks in
[verification.csv](verification.csv) that prove it, the phase by which it must be proven
first, and a status.

**Every requirement here is PROPOSED.** Claude seeded them on 2026-10-24 and re-seeded them for each
board since (684a609, be437f8, the RP2354B board at 71737ae, the ESP32-S3 board at 39cc07f). On
2026-10-28 both of those boards were dropped and the register was re-seeded for an Allwinner F1C200s
board; on 2026-10-29 Alex changed the next spin again (docs/static/DESIGN.md:74 and
docs/static/next_spin_requirements.md): the RP2350B (QFN-80, no flash inside) with one GD25Q128 16 MB QSPI
flash holding the program and the sound, no GBA, no Linux, no SD card, no Wi-Fi, online play through an
Android phone's Chrome (WebUSB). The register is now the RP2350B board's (board `rp2350b`, allocations
`.B`). The F1C200s board's own requirements (GBA at full speed and link play, games and saves, the
microSD slot, the FUSB303B's host and device roles, VBUS out as host) are DROPPED with it and name its
tag, `vv-f1c200s-final` (cffe55b). REQ-PRD-14 and -15 are rewritten for the phone bridge and the
bare-metal boot, REQ-PRD-18 (sound from the flash) is new, REQ-PRD-09 is Alex's all-in cost, and the
chip (SOC) section is the RP2350B's with its flash and its minimal design. The ESP32-S3 board's own
requirements (the module, its radio, its allocations `.R`, its power key, its backlight sinks, its PWM
audio, its 74HC165 chain) stay DROPPED with it and name its tag, `vv-esp32s3-final`. A carried requirement whose rationale or source still reasons
from the ESP32-S3 board's parts says so ('Was: on the ESP32-S3 board ..., to redo for the RP2350B
board'): the statement is what binds. Where DESIGN.md gives no number, the number is Claude's proposal
and says so. A requirement whose subject left the design is RETIRED with the commit that removed it; it
keeps its ID. None is binding until Alex decides it in [releases/APPROVALS.md](releases/APPROVALS.md):
APPROVED, REJECTED, or DEFERRED with a reason. A Status here that has no matching line he committed
there is a structure error, and a statement edited after he approved it needs his approval again.
`check_trace.py --gate` fails while any requirement in scope is PROPOSED.

Alex reviewed the product requirements (REQ-PRD) in chat on 2026-10-29 and the register now carries his
decisions, word for word where he gave a wording (PLAN.md section b2): REQ-PRD-02.B, -03.B, -14 and -15 approved as
written; -08.B and -18 approved and rewritten; REQ-PRD-19 new; REQ-PRD-09 with his limits ($60 at 10 units, $40 at 50).
The other live REQ-PRD rows are unchanged from c3aec22, where he approved them as worded. All stay PROPOSED until
he signs them.

How to read a block:

- Level: product (what a player or Alex sees), subsystem (a block of the design), or
  board (one board's make and test).
- Board: both or rp2350b (the RP2350B board, the next design of `hardware/ref_design`, REF-A).
  'both' is the product level: the need stated without one board's parts or numbers. A product
  requirement that names the board's parts or numbers is split: the parent (Board: both) states
  the need, and its allocation `REQ-X.B` (Board: rp2350b) holds the board's numbers. The gate
  evaluates rp2350b and both. `esp32s3` and `f1c200s` mark a DROPPED row (the ESP32-S3 board's `.R` and the F1C200s board's `.F` allocations among them);
  `prod_board` (Alex's `hardware/prod_board@611ebe3` and `hardware/prod_case@611ebe3`, deleted in
  574c94c, Alex: not needed) marks REQ-PBD-*, RETIRED.
- Subsystem: the RP2350B board's subsystems the requirement is allocated to (grid.md's power,
  cell, soc, storage, usbc, display, audio, buttons, case), or 'all'.
- Source: a datasheet page (the review file that quotes it is named), a standard clause, a
  validation_plan method (#n), a line of DESIGN.md (Alex's spec) or next_spin_requirements.md,
  or Alex's use. "Claude's estimate" or "Claude's proposal" means nobody has set the number yet:
  Alex should.
- Phase: ORDER (before the 5-board order: what the copper and the BOM freeze), CASE (before
  the printed case is released and the EVT units are assembled), EVT (the 5 boards), DVT (the
  revised spin), PVT (the 30-board run), SALE (before any unit is sold or given away).
- Status: PROPOSED until Alex decides; DEFERRED holds only up to the phase his line in
  APPROVALS.md names, then the requirement is PROPOSED again; RETIRED (with a 'Retired:' line
  naming the commit) when its subject left the design; DROPPED (with a 'Dropped:' line naming the
  board and its tag) when it belonged to a dropped board.

The standards' clause numbers below are from memory of IEC 62368-1:2018, IEC 62133-2:2017,
USB 2.0, IEC 61000-4-2 and EN 55035. Check each against the purchased text before relying on it.

## Product level

### REQ-PRD-01: Independent inputs
- Level: product
- Board: both
- Subsystem: buttons
- Statement: The device shall read at least 8 momentary inputs per seat (D-pad and A, B, X, Y), each on its own line, with any combination held at once read correctly.
- Rationale: two players press at the same time; a matrix would ghost.
- Source: Alex's use.
- Method: T
- Verified by: allocated: REQ-PRD-01.B (Was: REQ-PRD-01.R, the ESP32-S3 board's)
- Phase: EVT
- Status: APPROVED
- Notes: Was: prod_board had 16 contacts and no Start or Select at 76955d2, and its own allocation was Alex's; 574c94c deleted prod_board from the repository (Alex: not needed).

### REQ-PRD-01.R: Twenty independent inputs (ref_design)
- Level: product
- Board: esp32s3
- Statement: The device shall read 20 momentary inputs, 10 per seat (up, down, left, right, A, B, X, Y, Start, Select), and the power key, each on its own line, with any combination of up to 21 held at once read correctly.
- Rationale: since 39cc07f all 20 contacts and the power key SW24 come in through three chained 74HC165s (24 bits, U11 the third: pinmap.md Buttons); the Starts are plain buttons, the power key alone turns the device on (Was: both Starts turned it on).
- Source: Alex's use; README.
- Method: T
- Verified by: VER-56, VER-76
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the three 74HC165s (U11 the third) and the power key SW24 were the ESP32-S3 board's; REQ-PRD-01.B allocates the need to the RP2350B board

### REQ-PRD-01.B: Twenty independent inputs (rp2350b)
- Level: product
- Board: rp2350b
- Subsystem: buttons
- Statement: The device shall read 20 momentary inputs, 10 per seat (up, down, left, right, A, B, X, Y, Start, Select), each on its own line, with any combination held at once read correctly. Seat 1's Start is also the power key. There is no power slider.
- Rationale: two players press at the same time; the 20 contacts are read directly on 20 of the RP2350B's 48 GPIOs, with the 47k pull-up arrays (Alex, 2026-10-29); seat 1's Start also joins VSYS to PWR_EN through a diode and is read through the buffered power-key line, so it takes no extra pin (REQ-PWR-21). The DS Lite power slider is dropped (Alex, 2026-10-29). The 74HC165 chain (about $0.95 a unit at 10, freeing about 18 pins) is the designated pin-relief option and the first thing to change if the pin plan runs short (Alex, 2026-10-29: 'make it the first choice to change if we need more pins ever'; VER-232, VER-262).
- Source: DESIGN.md:39 ('Each player has the same controls: a D-pad, four face buttons (A, B, X, Y), Start and Select'), DESIGN.md:60 (seat 1's Start is the power key).
- Method: T
- Verified by: VER-56, VER-76
- Phase: EVT
- Status: APPROVED
- Notes: Reworded on 2026-10-29 at Alex's decision (the power slider is dropped; seat 1's Start is the power key). The statement changed, so it needs Alex's re-approval.

### REQ-PRD-02: One readable screen
- Level: product
- Board: both
- Subsystem: display
- Statement: The one shared screen shall show a full-screen test image with 0 missing lines or columns and correct red, green and blue order, seen from either seat.
- Rationale: the one shared screen is the product.
- Source: Alex's use.
- Method: T
- Verified by: allocated: REQ-PRD-02.B (Was: REQ-PRD-02.R)
- Phase: EVT
- Status: APPROVED

### REQ-PRD-02.R: One readable screen (ref_design)
- Level: product
- Board: esp32s3
- Statement: The 320 x 480 ER-TFT035IPS-6 (ILI9488) shall show a full-screen test image with 0 missing lines or columns and correct red, green and blue order, seen from either seat.
- Rationale: the one shared screen is the product.
- Source: ER-TFT035IPS-6 datasheet (pin_review/panel.json); the panel since 79de1f7 (it was the 2.4 in ER-TFT024IPS-3 from 53e21c8).
- Method: T
- Verified by: VER-60, VER-61
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board's allocation; the panel stays on the RP2350B board, allocated in REQ-PRD-02.B

### REQ-PRD-02.B: One readable screen (rp2350b)
- Level: product
- Board: rp2350b
- Subsystem: display
- Statement: The 320 x 480 ER-TFT035IPS-6 (ILI9488) shall show a full-screen test image with 0 missing lines or columns and correct red, green and blue order, seen from either seat, driven by the RP2350B on the panel's 8-bit 8080 bus.
- Rationale: the one shared screen is the product; the panel is today's. The 8-bit 8080 bus is fast enough (Notes).
- Source: DESIGN.md:56 ('one 3.5 inch IPS screen'); next_spin_requirements.md:12 ('a display bus fast enough for the 320 x 480 panel'), :27 ('today's ILI9488'); hardware/ref_design/datasheets/ili9488.pdf and hardware/ref_design/datasheets/er-tft035ips-6.pdf (Notes); Alex confirmed the 8-bit bus on 2026-10-29.
- Method: T
- Verified by: VER-224, VER-60, VER-61, VER-199
- Phase: EVT
- Status: APPROVED
- Notes: Alex reviewed this row in chat on 2026-10-29: approved as written, the 8-bit 8080 bus confirmed. The evidence, read from the two datasheets (VER-199 repeats it on paper and VER-224 measures it): (1) the ILI9488 datasheet gives a 40 ns minimum write cycle (twc, section 17.4.1, DBI Type B timing characteristics); at 40 ns a 320 x 480 RGB565 frame, two writes a pixel, is 320 x 480 x 2 = 307,200 writes = 12.3 ms, so the bus maxes out at about 81 fps, 1.36x over 60. (2) The ER-TFT035IPS-6 datasheet puts IM0 to IM2 on FPC pins 7 to 9 (the 8-bit 8080 mode is IM2 IM1 IM0 = 0,1,1) and DB17 to DB0 on pins 15 to 32; the 8-bit bus uses DB7 to DB0, pins 25 to 32, and the unused data lines are tied to ground ('Fix to DGND level when not in use'). (3) The module datasheet gives no timing of its own, so the controller's is the limit. The datasheets are hardware/ref_design/datasheets/er-tft035ips-6.pdf and hardware/ref_design/datasheets/ili9488.pdf.

### REQ-PRD-03: Play time
- Level: product
- Board: both
- Subsystem: cell power
- Statement: From charge-done, the device shall play for at least 4.0 h before its low-cell cutoff at 25 C, backlight at 50% and sound at -12 dBFS.
- Rationale: a session of play on one charge.
- Source: Claude's estimate; Alex to set the target.
- Method: T
- Verified by: allocated: REQ-PRD-03.B (Was: REQ-PRD-03.R)
- Phase: EVT
- Status: APPROVED

### REQ-PRD-03.R: Play time (ref_design)
- Level: product
- Board: esp32s3
- Statement: From charge-done, the device shall play for at least 4.0 h before the 3.50 V cutoff at 25 C, backlight at 50% duty and sound at -12 dBFS.
- Rationale: about 700 mAh against an estimated 90-120 mA (core up to 50 mA, panel logic 10 mA, backlight about 54 mA at 50% duty, half its 108.2 mA nominal at 100% with R61 3.74k, amplifier 4.4 mA plus sound); the usable capacity to the cutoff is measured per lot (VER-153).
- Source: Claude's estimate; Alex to set the target.
- Method: T
- Verified by: VER-101, VER-153
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): its 90-120 mA budget was the ESP32-S3 board's; REQ-PRD-03.B holds the RP2350B board's
- Notes: proposed: DEFERRED to EVT pending VER-37 (the cell's spec sheet) and VER-153 (F-33); only Alex sets it.

### REQ-PRD-03.B: Play time (rp2350b)
- Level: product
- Board: rp2350b
- Subsystem: cell power
- Statement: From charge-done the device shall play for at least 4.0 h of battling (backlight at 50%, sound at -12 dBFS) at 25 C before the low-cell cutoff.
- Rationale: a session of play on one charge; the RP2350B at 150 MHz draws far less than an application processor would, so the 4.0 h of the parent holds with margin on paper. The 4.0 h is Alex's (2026-10-29); Claude's estimate is 5 to 6 h at about 100 to 120 mA.
- Source: Alex, 2026-10-29, in chat (4.0 h, approved as written); Claude's estimate; the stripboard Pico 2's measured draw and the power budget (VER-231).
- Method: T, A
- Verified by: VER-231, VER-101, VER-153
- Phase: EVT
- Status: APPROVED
- Notes: Alex reviewed this row in chat on 2026-10-29: approved as written, 4.0 h, and the 700 mAh 403450 cell stays. Claude's estimate with that cell is 5 to 6 h at about 100 to 120 mA, so 4.0 h leaves margin. VER-231 works the draw out on paper from the stripboard's measured currents; VER-101 and VER-153 measure the hours and the usable capacity on built units.

### REQ-PRD-04: Charge time
- Level: product
- Board: both
- Subsystem: cell power
- Statement: From the low-cell cutoff the cell shall reach charge-done within 4.0 h at 25 C from a 5 V USB source offering 0.5 A or more, device on the charging screen; the time is Alex's to set and no tighter than its allocations.
- Rationale: a flat unit is ready again within an evening.
- Source: Claude's estimate; Alex to set the target.
- Method: T
- Verified by: allocated: REQ-PRD-04.B (Was: REQ-PRD-04.R)
- Phase: EVT
- Notes: the first draft said 3.0 h from 1.5 A, the allocation REQ-PRD-04.B says 4.0 h from 0.5 A (the BQ24073's USB500 input limit is 475 mA): Alex sets the time (PLAN.md decision DEC-04, F-R12).
- Status: APPROVED

### REQ-PRD-04.R: Charge time (ref_design)
- Level: product
- Board: esp32s3
- Statement: From the 3.50 V cutoff the cell shall reach charge-done (CHG_STAT_N high with CHG_PG_N low) within 4.0 h at 25 C from a source offering 0.5 A or more at 4.75-5.25 V, device on the charging screen.
- Rationale: 242-295 mA of constant current (R5 3.3k) into about 700 mAh is 2.4-2.9 h to 80%, plus the constant-voltage tail; the BQ24073's own fast-charge timer stops at 5 h (DF-152); the system's draw comes out of the 475 mA input limit first.
- Source: BQ24073 datasheet SLUS810N, ISET (pin_review/u2.json pin 16); Claude's estimate for the tail.
- Method: T
- Verified by: VER-102
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the BQ24073's status pins and R5 were the ESP32-S3 board's; REQ-PRD-04.B holds the RP2350B board's
- Notes: proposed: DEFERRED to EVT pending VER-37 (F-33); only Alex sets it.

### REQ-PRD-04.B: Charge time (rp2350b)
- Level: product
- Board: rp2350b
- Subsystem: cell power
- Statement: From the low-cell cutoff the cell shall reach charge-done within 4.0 h at 25 C from a source offering 0.5 A or more at 4.75-5.25 V, device on the charging screen.
- Rationale: a flat unit is ready again within an evening; the charger is the RP board's BQ24073 (C15220), kept on the RP2350B board (REQ-BAT-01, REQ-BAT-03, REQ-BAT-13, VER-109, VER-133).
- Source: Claude's estimate; Alex to set the target.
- Method: T
- Verified by: VER-102
- Phase: EVT
- Status: APPROVED

### REQ-PRD-05: Shelf life when off
- Level: product
- Board: both
- Subsystem: power cell
- Statement: Switched off with a full cell and no USB, the board shall drain the cell by no more than 30 uA, so that the board alone takes 2 years or more to empty it.
- Rationale: 700 mAh / 30 uA is about 2.7 years; the cell's own self-discharge dominates.
- Source: design_review.md's Off state (18-22 uA) predates the BQ24073 (Was: it counts D5's reverse leakage, and D5 left in 2d1124d); VER-103 re-derives the budget for the RP2350B board from the RP board's circuits, the TPS3421 on 3V3 drawing nothing while off (REQ-PWR-07; Alex, 2026-10-29) (O-R23); validation_plan #33.
- Method: T, A
- Verified by: VER-65, VER-103
- Phase: EVT
- Status: APPROVED

### REQ-PRD-06: Size
- Level: product
- Board: esp32s3
- Statement: The closed case shall measure at most 78.9 x 156.64 mm in plan and 10.7 mm thick at the screen, 7.7 mm at the button ends, each +0.3 mm for print tolerance.
- Rationale: a DS Lite's feel with the width the buttons set and the length the 3.5 in panel sets (hardware/ref_design/PLAN.md, Case).
- Source: hardware/ref_design/README.md; case.scad (case_w, case_l, case_h, deck_face).
- Method: I, A
- Verified by: VER-48, VER-20
- Phase: CASE
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): its numbers (78.9 x 156.64 mm, 10.7 mm at the screen) were the ESP32-S3 board's case, raised for the module and grown for its microSD slot; REQ-PRD-17 states the size from DESIGN.md and next_spin_requirements.md
- Notes: restated at 39cc07f from 77.4 x 156.64 x 9.6 mm: Alex raised the glass 1.1 mm for the module under it (glass_gap 3.5, 2026-10-27: 'raising the glass seems fine') and moved the left wall 1.5 mm out for the microSD slot (grow_l, 2026-10-28); the restated numbers are PROPOSED like the rest. VER-190 reads the outline with grow_l.

### REQ-PRD-07: Single-fault safety
- Level: product
- Board: both
- Subsystem: all
- Statement: No single fault in the electronics (one open, short, adjacent-pin bridge, wrong part or wrong rotation) shall cause fire, cell venting, a part above its material limit, or an accessible surface above IEC 62368-1's single-fault limit for its class (TS2, clause to confirm), with 1 or more independent protection layers left working after the fault.
- Rationale: a Li-ion cell in a printed case held in the hands; the standard sets touch limits for normal operation (REQ-THM-01) and looser ones under a single fault.
- Source: IEC 62368-1 Annex B.4 (single fault conditions) and clause 9 (to confirm); validation_plan #89, #151.
- Method: A, T
- Verified by: VER-98, VER-97, VER-168
- Phase: EVT
- Status: APPROVED
- Notes: the phase moved to EVT with VER-98 (O-R9: the DFMEA review is advisory at ORDER and binding at G-EVT-16); nothing proves a single-fault limit before that.

### REQ-PRD-08: Recoverable firmware
- Level: product
- Board: both
- Subsystem: soc usbc storage
- Statement: A unit with blank, corrupt, failed-update or hung firmware shall be recovered with the case closed, 3 out of 3 tries each, and shall take a debugger with the case open.
- Rationale: bring-up and field recovery must not need the case opened or a soldering iron.
- Source: DESIGN.md:64 ('There is no separate reset button.') and REQ-PRD-08.B.
- Method: D, T
- Verified by: allocated: REQ-PRD-08.B (Was: REQ-PRD-08.R)
- Phase: EVT
- Status: APPROVED
- Notes: Reworded on 2026-10-29 by the artifact audit (analysis/artifact_audit_2026-10-29.md): the old statement ('enter the USB boot loader ... 2 out of 2') described the RP board's BOOTSEL button and contradicted its allocation REQ-PRD-08.B (the hard reset, Select at power-on, the failed-update fall back, 3 of 3). Alex's approval A-011 covered the old text and no longer counts: Alex must re-approve this row.

### REQ-PRD-08.R: Recoverable firmware (ref_design)
- Level: product
- Board: esp32s3
- Statement: A unit with blank, corrupt or hung firmware shall take a new image over USB-C (the ESP32-S3's USB Serial/JTAG) with the case closed, and with the case open shall enter the ROM's download mode through J7's BOOT and EN pads, 3 out of 3 tries.
- Rationale: the BOOT and RESET buttons and their pinholes left at 39cc07f (the microSD slot took the left bezel; docs/living/esp32s3_build.md); J7 keeps 3V3, EN, BOOT and GND on pogo pads on the back. An image that turns USB off can only be recovered with the case open (DEC-41).
- Source: docs/living/esp32s3_build.md; esp32-s3-wroom-1.txt:810-817 (boot mode); design.py J7.
- Method: D, T
- Verified by: VER-212
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3's USB Serial/JTAG and J7's BOOT and EN pads; REQ-PRD-08.B holds the RP2350B board's recovery

### REQ-PRD-08.B: Recoverable firmware (rp2350b)
- Level: product
- Board: rp2350b
- Subsystem: soc usbc storage power buttons
- Statement: Holding seat 1's Start for about 8 s shall reset and turn off the unit whatever the firmware is doing. Holding seat 1's Select while turning the power on shall enter the RP2350's USB boot mode whatever state the main firmware is in. A blank unit shall enter USB boot mode by itself, a corrupted image or partition table shall fail the boot ROM's hash check and drop to USB boot mode by itself, and a failed update shall fall back to the previous firmware. All of this with the case closed, 3 out of 3 tries each.
- Rationale: there is no reset button and no BOOT button on the case, and a hung main firmware must not need the case opened. Four layers, each a design fact: (1) the hard reset is a TI TPS3421 push-button controller (LCSC C2067262, $0.36 at 30) on seat 1's Start's buffered line, driving the RP2350's RUN pin low after the hold; a reset releases PWR_HOLD, so the unit turns off, and with USB in it restarts; the exact delay depends on the variant and its delay-select pin, and 'about 8 s' means the nearest available delay, to confirm from its datasheet (VER-258). (2) A small bootloader of our own lives in its own flash partition, write-protected with the GD25Q128's block protection; it reads seat 1's Select first thing (GPIO only, before the display, sound or USB start), enters USB boot mode if it is held, and otherwise starts the main firmware (REQ-SOC-10). (3) The main firmware sits in A/B slots with the boot ROM's try-before-you-buy, plus a hash in each image; a failed-boot counter in a watchdog scratch register sends the unit to USB boot mode after 3 failed boots (REQ-SOC-10). (4) With the case open, a BOOTSEL test pad on QSPI SS forces USB boot. The firmware itself turns the unit off on a 1 to 2 s hold of seat 1's Start (REQ-PWR-21), so the 8 s hold is only for a unit that does not answer. A new image goes in by dragging a .uf2 file to the unit's USB drive (REQ-PRD-19).
- Source: DESIGN.md:64 ('There is no separate reset button.'); DESIGN.md:65 (hold seat 1's Start about 8 s); Alex, 2026-10-29, in chat (rewritten and approved: the statement is his text); the RP2350 datasheet's boot sequence, partition table, hash check and try-before-you-buy (to download: VER-233; VER-259 reads them); the TPS3421 datasheet (VER-258).
- Method: D, T
- Verified by: VER-241, VER-252, VER-258, VER-259
- Phase: EVT
- Status: APPROVED
- Notes: Reworded on 2026-10-29 at Alex's decision (the 8 s hold is on seat 1's Start, not the dropped power slider); only that phrase of the statement changed, and it needs Alex's re-approval. Rewritten on 2026-10-29 at Alex's review. It was: a blank or corrupt image through the USB boot mode and a hung one reset by the watchdog within 2 s, 2 of 2 tries; the watchdog stays (REQ-SOC-04) but is no longer the recovery's only path. To confirm from the datasheets: the TPS3421 variant and delay-select pin that give the nearest delay to 8 s (VER-258), and how the boot ROM's hash check and try-before-you-buy behave with a corrupted partition table (VER-259). Open question for Alex: block protection on the bootloader's partition and a recovery .uf2 that rewrites the bootloader (REQ-PRD-19) pull against each other (DF-263). 'about 8 s' is read as 6 s to 10 s (Alex, 2026-10-29); REQ-SOC-04 carries the number.

### REQ-PRD-09: Cost
- Level: product
- Board: both
- Subsystem: all
- Statement: The all-in cost for Alex to hold one fully assembled unit shall be $60 or less at 10 units and $40 or less at 50 units, all-in being the board, parts, JLC assembly, panel, cell, speaker, silicone, screws, tape, the home-printed case, every shipping leg, and US tariffs and duties.
- Rationale: this is Alex's definition of cost (2026-10-29): what it costs him to hold one finished unit, not the board's price. The two limits are Alex's (2026-10-29). Claude's estimate of what it comes to, TPS3421 included, is about $58 a unit at 10 and about $38.50 at 50, inside both.
- Source: Alex's definition of cost and his limits, $60 at 10 and $40 at 50, 2026-10-29, in chat; Claude's estimate (VER-117 re-runs it: hardware/ref_design/board/tools/cost.py, JLC's instant quote, the parts, every shipping leg, the US tariff and duty rate on each).
- Method: I
- Verified by: VER-117
- Phase: EVT
- Status: APPROVED
- Notes: Restated on 2026-10-29: it was $45 or less at 10 units (board, assembly, shipping, panel, cell, speaker, silicone, caps, screws, filament, the microSD card); the new definition adds tape, tariffs and duties and every shipping leg, and drops the card, which left the design. Alex set the limits himself on 2026-10-29: $60 or less at 10 units and $40 or less at 50 units, with the all-in definition kept exactly as worded; they are his numbers, not a proposal. Claude's estimate before that was $57 and $38 (a proposal, 2026-10-29 morning); with the TPS3421 hard-reset controller (REQ-PRD-08.B) it is about $58 and about $38.50.

### REQ-PRD-10: Serviceable cell and panel
- Level: product
- Board: both
- Subsystem: cell display case
- Statement: Alex shall replace the cell or the panel with a screwdriver and no soldering in 10 min or less each.
- Rationale: cells age and glass breaks; J2 is a plug and J3 a ZIF so both swap.
- Source: validation_plan #194, #203 (EU Regulation 2023/1542 Art. 11 if sold in the EU).
- Method: D
- Verified by: VER-91
- Phase: EVT
- Status: APPROVED

### REQ-PRD-11: Survives drops
- Level: product
- Board: both
- Subsystem: case
- Statement: After 3 drops from 1.0 m onto hardwood (face, end, corner) the device shall still pass the self-test, with no crack that exposes the cell or a board.
- Rationale: a handheld is dropped.
- Source: IEC 62368-1 Annex T.7 (drop test, 1 m for hand-held equipment, to confirm); validation_plan #50.
- Method: T
- Verified by: VER-93
- Phase: DVT
- Status: APPROVED

### REQ-PRD-12: GBA games at full speed
- Level: product
- Board: f1c200s
- Subsystem: soc display audio
- Statement: Pokemon FireRed under the gpSP emulator shall run at 59.7 fps or more averaged over 10 min (the GBA's own 59.73 Hz), with no frame skipped and sound on, and no 1 s window under 57 fps, in the overworld and in a battle.
- Rationale: goal 4: the console plays GBA games at full speed. 59.7 fps and the 57 fps floor are Claude's proposal: DESIGN.md says 'at full speed', next_spin_requirements.md '60 fps'; Alex sets them.
- Source: DESIGN.md:50 ('The console plays GBA (Game Boy Advance) games at full speed, with Pokemon FireRed as the test game. (2026-10-28)'); next_spin_requirements.md:15 ('GBA emulation at 60 fps (Pokemon FireRed as the test)').
- Method: T
- Verified by: VER-221, VER-247
- Phase: ORDER
- Status: DROPPED
- Notes: The chip gate (gates.md NM-03) measures it on a dev board before any board design; a fail sends the project to the V3s (DESIGN.md:77).
- Dropped: DROPPED with f1c200s (tag vv-f1c200s-final): GBA left the design on 2026-10-29 (DESIGN.md: goal 4 removed); its statement is at the tag

### REQ-PRD-13: GBA link play over a USB-C cable
- Level: product
- Board: f1c200s
- Subsystem: usbc soc
- Statement: Two consoles joined by one USB-C to USB-C cable shall link Pokemon FireRed through the emulator's GBA Wireless Adapter support, one console the USB host and the other the device as the FUSB303B's role detection decides, for a trade and a battle, with 0 disconnects in 30 min linked.
- Rationale: goal 4: two consoles link with a USB-C cable to play GBA games together. 30 min and 0 disconnects are Claude's proposal.
- Source: DESIGN.md:51 ('Two consoles can link with a USB-C cable to play GBA games together. (2026-10-28)'); DESIGN.md:92 ('GBA link play goes over the USB-C cable between two consoles, using the emulator's GBA Wireless Adapter support.'); next_spin_requirements.md:16.
- Method: T
- Verified by: VER-225, VER-226
- Phase: EVT
- Status: DROPPED
- Notes: next_spin_requirements.md:16 says the emulator's link support is 'not yet checked' (DF-243).
- Dropped: DROPPED with f1c200s (tag vv-f1c200s-final): GBA link play and USB host mode left the design on 2026-10-29; its statement is at the tag

### REQ-PRD-14: Online play as a Showdown guest through a phone
- Level: product
- Board: both
- Subsystem: usbc soc
- Statement: With the console on one USB-C cable to an Android phone, a web page in the phone's Chrome browser (WebUSB, the bridge page) shall pass the battle between the console and Pokemon Showdown's own server, joined as a guest with no login, and finish 5 of 5 random battles, Gen 1 and Gen 3, each accepted by Showdown's server, with no disconnect.
- Rationale: goal 3: the same console takes on players online. The console is a USB device and has no Wi-Fi: the phone carries the network. 5 of 5 was Claude's proposal; Alex approved it as written on 2026-10-29.
- Source: DESIGN.md:49 ('Players can play against others online, as a guest on Pokemon Showdown (the free online Pokemon battle site) with no login. (2026-10-26)'); DESIGN.md:84 (a guest only, Showdown's own servers, none of ours); DESIGN.md:86 ('Online play goes through a phone: the console plugs into an Android phone over USB, and a web page in the phone's Chrome browser passes the battle between the console and Showdown. (2026-10-29)'); next_spin_requirements.md:16.
- Method: T
- Verified by: VER-251, VER-227
- Phase: ORDER
- Status: APPROVED
- Notes: Rewritten on 2026-10-29. It was the F1C200s board's: Linux with a phone's USB tether or a USB Wi-Fi dongle, the console the USB host (tag vv-f1c200s-final). The chip gate (gates.md NM-03) runs the phone bridge test on the stripboard Pico 2 (RP2350A) before any board design. Alex reviewed this row in chat on 2026-10-29: approved as written.

### REQ-PRD-15: A match within seconds of power-on
- Level: product
- Board: both
- Subsystem: soc storage power buttons
- Statement: Within 0.5 s of pressing seat 1's Start the screen shall show a 'keep holding' bar; the lobby shall show within 1.0 s after the 2.5 s hold completes; 10 of 10 power-ons. Released before the hold completes, the console shall turn fully off.
- Rationale: goal 1: a match within seconds. DESIGN.md:87 gives 'within about a second' and a 2 to 3 s hold to turn on (DESIGN.md:60), so a bump in a pocket does not switch it on; the 1.0 s limit and the 10 of 10 power-ons were Claude's proposal, and the re-timed statement is Alex's approved text of 2026-10-29. The 1.0 s after the hold is VER-228's pass limit; VER-223 measures the supply to the lobby, which is the stricter number.
- Source: DESIGN.md:50 ('It starts fast: a match within seconds of power-on. (2026-10-28)'); DESIGN.md:60 (hold seat 1's Start 2 to 3 s to turn on); DESIGN.md:87 ('At power-on the Pokemon battler starts on its own within about a second: our own Rust program, with no operating system under it. (2026-10-29)'); next_spin_requirements.md:17.
- Method: T
- Verified by: VER-223, VER-228
- Phase: ORDER
- Status: APPROVED
- Notes: Re-timed on 2026-10-29 at Alex's decision: the statement is the text he approved in chat, and it needs his re-approval in the register. It reverses the earlier 'no hold-to-latch' change of the same day: pressing Start powers the regulator, the firmware latches PWR_HOLD only after Start has been held 2.5 s, and an early release drops the power (REQ-PWR-21, VER-228, VER-240). Earlier: the Linux part (a 10 s boot to the GBA menu on a held button) left the design with Linux (tag vv-f1c200s-final).

### REQ-PRD-16: Games and saves over USB or SD
- Level: product
- Board: f1c200s
- Subsystem: storage usbc
- Statement: A 16 MB GBA ROM and its save shall load onto the console either on the microSD card written in a computer, or over USB-C with the console a USB mass-storage device of the computer, and the game shall then run with its save; a 16 MB copy over USB shall take 60 s or less.
- Rationale: games and saves come from a computer; 16 MB is FireRed's ROM. The 60 s is Claude's proposal.
- Source: DESIGN.md:53 ('Games and save files load from a computer over USB, or from an SD card. (2026-10-28)'); DESIGN.md:93; next_spin_requirements.md:18 ('room for a 16 MB ROM plus saves').
- Method: T
- Verified by: VER-225, VER-229
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with f1c200s (tag vv-f1c200s-final): loading games and saves, and the microSD card, left the design on 2026-10-29; its statement is at the tag

### REQ-PRD-17: About today's size or smaller
- Level: product
- Board: both
- Subsystem: case
- Statement: The closed case shall measure at most 78.9 x 156.48 mm in plan and 9.6 mm thick at the screen, each +0.3 mm for print tolerance, and each spin shall record a size-minimization pass.
- Rationale: goal 4: it fits in a pocket and every spin is as small as it can be. The numbers are today's case as next_spin_requirements.md gives it; the +0.3 mm for print tolerance is Claude's proposal (next_spin_requirements.md says 'no growth'); Alex sets it.
- Source: DESIGN.md:61 ('It stays about today's size or smaller, never bigger than a smartphone in its case, and every new spin is made as small as it can be. (2026-10-28)'); next_spin_requirements.md:18 ('no growth over today's case (78.9 x 156.48 mm in plan, 9.6 mm thick at the screen)').
- Method: I, A
- Verified by: VER-230, VER-48, VER-20
- Phase: CASE
- Status: APPROVED
- Notes: Was: REQ-PRD-06 gave 78.9 x 156.64 mm and 10.7 mm for the ESP32-S3 board's case; next_spin_requirements.md gives 156.48 and 9.6 as today's.

### REQ-PRD-18: A GBA-style sound engine from the 16 MB flash
- Level: product
- Board: both
- Subsystem: audio storage soc
- Statement: Music shall play from note sequences on sampled instruments and 4 GBA-style synthesized channels (two square waves, a wave channel and noise); cries and sound effects shall play from compressed samples; all of it read from the 16 MB flash. The mixer shall run at least 8 channels at 32 kHz with 0 audible dropouts (buffer underruns) in 30 min of battling at the full display rate. The program shall take 4 MB or less and the sound data shall have 2 MB or more of room.
- Rationale: Alex: "we need external spi flash to do sound right". The engine follows the design of Nintendo's M4A ('Sappy') GBA sound engine: sequences are a few KB, instruments are short samples, the four synthesized channels cost no flash, and only the cries and effects are long samples, compressed. About 2 MB covers sequenced sound, so the 2 MB of room is enough; the 16 MB flash is kept by Alex's choice. The program is 1.4 MB today (next_spin_requirements.md:13). 8 channels, 32 kHz, 4 MB, 2 MB and 30 min are the numbers of Alex's rewrite (2026-10-29).
- Source: Alex, 2026-10-29 ('we need external spi flash to do sound right'; the statement is his text, approved on his review); DESIGN.md:74 ('a 16 MB flash memory chip beside it, which holds the program and the sound'); next_spin_requirements.md:15 ('Sound done right: room for sound samples: a 16 MB flash chip the chip reads as memory').
- Method: T, A
- Verified by: VER-256, VER-252, VER-255
- Phase: ORDER
- Status: APPROVED
- Notes: New on 2026-10-29; rewritten the same day for a GBA-style engine (it was samples in 8 MB or more of the flash, with 4 MB, 8 MB and 30 min as Claude's proposal). The sound content must be our own or licensed, not data taken from FireRed (its sequences, instruments and cries), if units are sold: the engine follows M4A's design, the content does not come with it (DF-265, VER-261). The 16 MB flash is kept by Alex's choice, even though about 2 MB covers sequenced sound; the rest is room (A/B firmware slots, REQ-SOC-10, the settings and the debug log).

### REQ-PRD-19: Update by dragging a file to a USB drive
- Level: product
- Board: both
- Subsystem: soc usbc storage
- Statement: In update mode the unit shall show up as a USB drive on Windows, macOS and Linux with no driver to install. Copying one recovery .uf2 file shall restore a unit from any flash state (erased or corrupted), with the partition table, bootloader, firmware and sound included, within 3 min. Copying a firmware .uf2 shall install that firmware alone within 30 s. 3 out of 3 tries each.
- Rationale: a player or Alex updates or rescues a unit with a computer and no tools, and no software to install. This is the RP2350 boot ROM's UF2 mass-storage mode (USB boot mode), the same one REQ-PRD-08.B drops a blank or corrupt unit into. A UF2 file carries each block's flash address, so one file can hold everything. The 3 min and the 30 s are Alex's (2026-10-29). For scale: a 1.4 MB firmware in 30 s needs about 47 KB/s of payload through the drive (VER-260 measures it).
- Source: Alex, 2026-10-29, in chat (new and approved: the statement is his text); the RP2350 datasheet's USB boot mode and UF2 download (to download: VER-233; VER-259 reads it).
- Method: T, D
- Verified by: VER-260, VER-259
- Phase: EVT
- Status: APPROVED
- Notes: New on 2026-10-29. The flash layout proposal (Claude's, REQ-SOC-10): partition table 4 KB, bootloader 64 KB (locked), firmware A 2 MB, firmware B 2 MB, sound data, and spare for settings and the debug log. To confirm against the RP2350 datasheet (VER-259), then on a dev board (VER-260): how a dragged firmware .uf2 picks the A or B slot, and whether the boot ROM accepts a whole-flash (absolute) UF2 over a valid partition table. If it does not, the recovery step is `picotool erase` and then the one file; the statement says 'copying one recovery .uf2 file', so that extra step would need Alex's decision (DF-263 holds the related question of the locked bootloader).

### REQ-PRD-20: Sleep a local battle
- Level: product
- Board: both
- Subsystem: soc storage power
- Statement: During a local battle, choosing Sleep from the pause menu shall save the battle to flash and turn the console off within 1 s; the next power-on shall return to the same battle, both players at their choice screens, with every Pokemon's state identical (HP, status, boosts, field), 10 of 10 tries, including after the cell has been unplugged. Online battles shall not offer Sleep. A save made by different firmware shall be discarded at power-on (the save carries the firmware's build ID; a mismatch, or a save that fails its check, starts at the lobby instead), 3 of 3 after an update.
- Rationale: DESIGN.md:42 (pause a local battle with Sleep; online battles cannot be paused). The statement is Alex's approved text of 2026-10-29. The save is a snapshot of the battle state, a few KB, in the flash's spare area (REQ-SOC-11); there is no input-log replay, so it does not depend on the engine being deterministic across firmware, and a build-ID mismatch discards it.
- Source: DESIGN.md:42; Alex, 2026-10-29, in chat (new: the statement is his text; the snapshot in the spare area, no input-log replay).
- Method: T, D
- Verified by: VER-263, VER-264
- Phase: EVT
- Status: APPROVED
- Notes: New on 2026-10-29; it needs Alex's approval in the register. Claude's assumption, to confirm: update mode (Select held at power-on) is entered before the save is read, so an update never resumes an old battle.

## Subsystem: cell and charging (BAT)

### REQ-BAT-01: Charge voltage
- Level: subsystem
- Board: both
- Subsystem: cell
- Statement: The charger shall stop the cell at 4.20 V +-1% (4.158-4.242 V at the cell terminals at charge-done) and never above the cell maker's maximum charge voltage.
- Rationale: overcharge is the main cell hazard.
- Source: BQ24073 datasheet SLUS810N, VBAT(REG) (U2 as on the RP board; design.py and pinmap.md quote 4.2 V +-1%); IEC 62133-2 (upper limit charging voltage set by the cell maker).
- Method: T, A
- Verified by: VER-67, VER-134, VER-14
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-02: Charge current
- Level: subsystem
- Board: both
- Subsystem: cell
- Statement: The constant-current charge shall not exceed 0.5 C of the cell's rated capacity or the cell maker's standard charge current, whichever is lower.
- Rationale: 0.5 C is the usual standard rate for small pouch cells.
- Source: IEC 62133-2 (the cell maker's charging limits); validation_plan #28, #128.
- Method: T, I, A
- Verified by: allocated: REQ-BAT-02.B (Was: REQ-BAT-02.R)
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-02.R: Charge current (ref_design)
- Level: subsystem
- Board: esp32s3
- Statement: The constant-current charge shall measure 242-295 mA (R5 = 3.3k against the BQ24073's KISET of 890 A-ohm) and shall not exceed 0.5 C of the 403450's rated capacity or its maker's standard charge current, whichever is lower; cell.json holds the cell's numbers and an assert recomputes the limit whenever R5 or the cell changes.
- Rationale: R5 was once kept from the 503450 (DEF-11); the 403450's own limit is not known yet.
- Source: pin_review/u2.json pin 16 (ICHG = 890 / R5).
- Method: T, I, A
- Verified by: VER-66, VER-37, VER-160
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): R5 against the BQ24073's KISET was the ESP32-S3 board's charger; REQ-BAT-02.B holds the RP2350B board's
- Notes: proposed: DEFERRED to EVT pending VER-37 (F-33); only Alex sets it.

### REQ-BAT-02.B: Charge current (rp2350b)
- Level: subsystem
- Board: rp2350b
- Subsystem: cell
- Statement: The constant-current charge shall measure at most 0.5 C of the chosen cell's rated capacity and at most its maker's standard charge current, at the charger's and its set resistor's tolerance.
- Rationale: the charger is not chosen yet; its set resistor follows the cell's spec sheet (carryover C-4).
- Source: the cell's spec sheet (VER-37); carryover.md C-4.
- Method: T, A
- Verified by: VER-66, VER-37, VER-160
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-03: Charge temperature window
- Level: subsystem
- Board: both
- Subsystem: cell
- Statement: At 99.9% of units, charging shall pause when the cell is above 45 C or below 0 C, or outside the cell maker's charge range if that is narrower: the hot edge at or under 45 C and the cold edge at or over 0 C (one-sided limits), and resume when back in range.
- Rationale: charging a Li-ion cell outside its range plates lithium (cold) or speeds failure (hot); TH1 (10k B4460, 0805, C525375) has its resistance at 25 C and its B each +-1% (tctr0805.txt:260, the part number's two F's), as on the RP board.
- Source: IEC 62133-2 (charging within the cell maker's temperature limits); the BQ24073's TS thresholds from SLUS810N: V_HOT 0.270-0.330 V and V_COLD 2.0-2.2 V at I_TS 72-78 uA (bq24074.txt:823-826, quoted by VER-34); validation_plan #30.
- Method: A, T
- Verified by: VER-32, VER-69, VER-135
- Phase: EVT
- Status: PROPOSED
- Notes: TH1 is a 10k B4460 0805 (TCTR0805F10K0F4460T, C525375), the highest B among the 10k NTCs JLC stocks, its resistance and B each +-1% (tctr0805.txt:260): at the charger's worst corner charging runs to a hot edge of 48.2 C (the RP board's design.py comment gives 4.6 C to 48.2 C; `design_numbers.py tempwin` recomputes the window on the RP2350B board's files, VER-32). Was: B3950, 1.4 C to 51.5 C at be437f8. The worst corner bounds every unit, so it is the 99.9% figure. A 45 C hot edge at the worst corner needs B 5130 or more; no stocked 10k part has it; no series or parallel resistor fixes both edges. PLAN.md decision DEC-05: accept the 48.2 C hot edge for EVT against the cell's datasheet (DF-217), or restate this requirement to the cell's own range if VER-37 shows it runs to 50 C.

### REQ-BAT-04: Cell temperature during charge
- Level: subsystem
- Board: both
- Subsystem: cell
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
- Subsystem: cell
- Statement: 100% of cells shall be checked with a meter before first mating and the result recorded per lot.
- Rationale: small cell connectors have no battery polarity standard; a reversed cell can destroy the charger.
- Source: assembly practice; IEC 62133-2.
- Method: I
- Verified by: allocated: REQ-BAT-05.B (Was: REQ-BAT-05.R)
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-05.R: Cell polarity (ref_design)
- Level: subsystem
- Board: esp32s3
- Statement: 100% of cells shall be checked with a meter before first mating, red lead on J2 pin 1 (silk '+'), and every plug shall latch in a 53261-0271 sample, the results recorded per lot.
- Rationale: PicoBlade has no battery polarity standard; a reversed cell destroys the BQ24073 and shorts the cell through it (DF-55); generic MX1.25 housings may not latch (DF-113).
- Source: pin_review/j2.json; assembly.md section 2.
- Method: I
- Verified by: VER-49, VER-54
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): J2 pin 1 was the ESP32-S3 board's cell header; REQ-BAT-05.B holds the RP2350B board's

### REQ-BAT-05.B: Cell polarity (rp2350b)
- Level: subsystem
- Board: rp2350b
- Subsystem: cell
- Statement: 100% of cells shall be checked with a meter before first mating, red lead on the cell header's pin 1 (marked '+' on the silk), and the result recorded per lot.
- Rationale: a reversed cell is a severity 10 failure (carryover C-7). Derivation (VER-189 fix, 2026-10-29): a design choice for the cell's safety: 100% of cells, not a sample, because one reversed cell is a severity 10 failure and a meter check costs seconds a cell. Earlier cite (not a primary source): carryover.md C-7.
- Source: Alex's decision (to sign)
- Method: I
- Verified by: VER-49, VER-54
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-06: Cell protection
- Level: subsystem
- Board: both
- Subsystem: cell
- Statement: Every cell shall carry a protection circuit with over-voltage at 4.35 V or less, under-voltage at 2.4 V or more and an over-current trip, each measured on 1 spare cell per lot.
- Rationale: the PCM is the independent layer behind every charge-path fault (REQ-BAT-09).
- Source: validation_plan #32; the cell's spec sheet once obtained (VER-37).
- Method: T, I
- Verified by: VER-88, VER-37, VER-140
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-07: Room to swell
- Level: subsystem
- Board: rp2350b
- Subsystem: cell case
- Statement: The closed case shall leave 0.4 mm or more of free height above the cell at worst-case tolerance (a 4.5 mm cell), under everything that lies over its pocket.
- Rationale: pouch cells grow 8-10% in thickness over life; 0.4 mm for a 4.0 mm cell. Derivation (VER-189 fix, 2026-10-29): 0.4 mm / 4.0 mm = 10%, the top of the 8-10% pouch growth the rationale gives; the worst-case cell is 4.5 mm, so the free height is held at 0.4 mm at that size. Serves the cell's life (REQ-BAT-08 is the same pouch). Earlier cite (not a primary source): validation_plan #26; case.scad (cell_top 1.6 over the board; the panel's back 2.4, the panel frame's walls 2.8, the button ends' underside 1.9 where they reach).
- Source: Alex's decision (to sign)
- Method: A, I
- Verified by: VER-182, VER-20, VER-48
- Phase: CASE
- Status: PROPOSED
- Notes: at be437f8 the cell lies under the glass's seat-2 half and no button end reaches it: 0.80 mm under the panel's back (VER-182 PASS); it was 0.30 mm under a deck at 684a609 (DEF-51).

### REQ-BAT-08: Nothing hard against the pouch
- Level: subsystem
- Board: rp2350b
- Subsystem: cell case
- Statement: No routed board edge, part body, plug or rib shall press on the cell's faces: 0.5 mm or more between a worst-case cell and the board's hole on every face it can rest against (1.5 mm at its lead end), after the fab, print and fastening tolerances; the tub's pocket at least 0.1 mm a side larger than a worst-case cell after the print's tolerance (0.4 mm in each direction); and no back-side copper within 1 mm of the hole.
- Rationale: a pierced pouch shorts internally and can burn; a pouch pressed into a pocket too small for it is stressed the same way (round 3's O-S1, 2026-10-27: the fit clause, a PROPOSED number). Derivation (VER-189 fix, 2026-10-29): a design choice: 0.5 mm (1.5 mm at the lead end) is the margin kept over the fab and print tolerance stack so no edge touches the pouch; 0.1 mm a side is the pocket's allowance over a worst-case cell. No datasheet sets these; the cell maker's own limit is not on disk. Earlier cite (not a primary source): validation_plan #27; placement.py CUTOUTS (the hole 37.8 x 54.8 mm since 71737ae); case.scad cell, cell_pocket_box.
- Source: Alex's decision (to sign)
- Method: A, I
- Verified by: VER-181, VER-06, VER-48, VER-109, VER-164
- Phase: ORDER
- Status: PROPOSED
- Notes: at 71737ae the gaps are met exactly and the fit is not (VER-181 FAIL): the hole is 37.8 x 54.8 mm with the cell along the board, the pocket 0.9 mm inside it on the long sides and the seat-1 end and 1.9 mm at the lead end, so a worst-case cell is 0.50 mm from the edge (1.50 mm at its lead end) after tolerances wherever it lies, and the pocket's walls are the stop (no tape); but the pocket is 52.0 x 36.0 mm, the worst-case cell's own size, with 0.00 mm spare against the fit clause's 0.40. What passes (design_numbers.py cellgap): a cell of at most 51.6 x 35.6 mm with its PCM by its spec sheet (no copper), or a 55.2 x 38.2 mm hole (copper). Was: at be437f8 the pocket sat 0.25 mm inside a 54.0 x 37.0 mm hole, -0.15 mm after tolerances. PLAN.md decision DEC-27 before the order, with DEC-25.

### REQ-BAT-09: Charge path single faults
- Level: subsystem
- Board: rp2350b
- Subsystem: cell power
- Statement: No single fault in the charger, its current-set and thermistor parts, the cell-side capacitors or the cell's leads shall charge the cell above 4.25 V or above its maker's standard charge current, or let it charge outside its temperature window; the cell's PCM stays the second layer.
- Rationale: a BQ24073 whose CV or battery FET fails, or whose TS is held in range (DF-216, DF-217, (Was: DF-151)), charges the cell with only the PCM left.
- Source: validation_plan #89; IEC 62368-1 Annex B.4; IEC 62133-2.
- Method: A, T
- Verified by: VER-98, VER-133, VER-134, VER-135, VER-169, VER-82, VER-88, VER-168
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final. the phase moved to EVT with VER-98 (O-R9).

### REQ-BAT-10: Deep discharge
- Level: subsystem
- Board: both
- Subsystem: cell power
- Statement: After the firmware shuts down at its cutoff, the board's off-state drain plus the cell's PCM drain and its self-discharge shall keep the cell above its PCM's under-voltage trip for 2 months or more at 25 C, and for the period Alex sets at 45 C storage.
- Rationale: a cell left flat in a drawer should still charge. Between 3.50 V and 3.0 V there is only about 5-10% of the capacity (35-70 mAh). 30 uA of board drain alone is 21.6 mAh a month; a DW01-class PCM adds 3-6 uA that never passes JP5; self-discharge is 1-3% a month (7-21 mAh); VSYS stays live through U2's battery FET when off, so U7's shutdown current is in the board's drain, and the TPS3421, fed from 3V3, adds nothing (REQ-PWR-07). With both terms the figure may be nearer 1 month: Alex sets the number knowing them. Derivation (VER-189 fix, 2026-10-29): the paper arithmetic, per month at 25 C on a 700 mAh cell (REQ-PRD-03.B): board drain 30 uA x 720 h = 21.6 mAh; the PCM 3 to 6 uA x 720 h = 2.2 to 4.3 mAh; self-discharge 1 to 3% = 7 to 21 mAh; total 31 to 47 mAh a month. The usable band between 3.50 V and 3.0 V is 35 to 70 mAh (5 to 10% of 700 mAh). So 2 months needs 62 to 94 mAh and is not met on paper at the high end of the drain and self-discharge; 1 month (31 to 47 mAh) mostly is. The 2 months stays as written; Alex should decide whether it is 2 months or 1 month, or the drain figure to hold. Earlier cite (not a primary source): Claude's estimate; validation_plan #33.
- Source: Alex's decision (to sign)
- Method: A, T
- Verified by: VER-103, VER-65, VER-146
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-11: Transport documents
- Level: subsystem
- Board: both
- Subsystem: cell
- Statement: Before any unit with a cell is posted or sold, the UN38.3 test summary and the MSDS for the exact cell model shall be on file (2 documents).
- Rationale: carriers refuse lithium cells without them.
- Source: UN Manual of Tests and Criteria, Part III, 38.3; validation_plan #34.
- Method: I
- Verified by: VER-38
- Phase: SALE
- Status: PROPOSED

### REQ-BAT-12: Cutoff accuracy
- Level: subsystem
- Board: rp2350b
- Subsystem: cell soc
- Statement: At 25 C ambient, the firmware's low-cell cutoff shall act at a true cell voltage of 3.50 V +0.05/-0.00 V on every unit after its per-unit calibration at 25 C (REQ-TST-06), at the highest load the firmware draws (the battler with sound playing, backlight at full).
- Rationale: the RP2350's ADC (GPIO40 to 47) reads VBAT/2 from R17 and R18 (470k each, 235 kohm) behind U10, an MCP6001 unity buffer from 3V3, as the RP board drew it. The RP2350's ADC error is not on disk yet (VER-233), and a one-point gain set at charge-done (the charger's CV is +-1%) cannot be shown to meet the window without its offset and gain split, so the per-unit point at 3.50 V does (VER-138, DF-225). U10 buffers the divider, so the pad's leakage (1 uA at most) flows from U10's output and not through R17 and R18, where it would be 0.47 V of VBAT, nine times this window. Off, U10 is unpowered and the pin reads 0 V.
- Source: design.py R17, R18, U10 (the RP board's, kept on the RP2350B board); the RP2350 datasheet's ADC chapter (to download: VER-233); mcp6001.txt:135-142; Claude's analysis (O-23, O-49).
- Method: A, T
- Verified by: VER-138, VER-139, VER-67
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final. U10 (MCP6001) buffers VBAT_DIV onto the ADC pin, so the pad's leakage flows from U10's output: the temperature-varying error is +-0.87 mV of VBAT with pessimistic stand-ins for U10's bias and drift, whose datasheet gives typical figures only (VER-138). The per-unit calibration at 3.50 V (REQ-TST-06) takes out the ADC's offset and gain; its drift with temperature is VER-139's and VER-67's. Re-grounded on 2026-10-29 by the artifact audit.

### REQ-BAT-13: Charge timeout
- Level: subsystem
- Board: both
- Subsystem: cell
- Statement: A charge that has not terminated shall stop within 5 h +-20% of fast charge (and 30 min of pre-charge under 3.0 V), and the firmware shall show a charge fault until USB is re-plugged.
- Rationale: a soft-shorted or damaged cell must not charge without end; the BQ24073's safety timers do it with TMR open, the firmware reads CHG at 2 Hz as the fault.
- Source: BQ24073 datasheet (pin_review/u2.json pins 9 and 14); Claude's proposal (O-50).
- Method: T
- Verified by: VER-145
- Phase: EVT
- Status: PROPOSED

### REQ-BAT-14: Mating the cell
- Level: subsystem
- Board: rp2350b
- Subsystem: cell power
- Statement: Plugging a 4.2 V cell into the cell's header with no USB shall not trip the PCM (the cell's voltage present after each plug), shall leave the device off, and shall peak the system rail at 4.5 V or less, 10 plugs out of 10.
- Rationale: with no USB, U2's battery FET is on, so the plug connects C3 (10 uF) directly and C2 and C5 (14.7 uF) more on VSYS through the FET: a peak of roughly 10-15 A for microseconds (DF-226). Derivation (VER-189 fix, 2026-10-29): 4.5 V is the 4.2 V full cell plus 0.3 V of ringing margin on VSYS; 10 of 10 because a plug is a repeated event and the first fail is the PCM tripping. A design choice: no datasheet gives the plug's inrush. Earlier cite (not a primary source): Claude's analysis (O-42).
- Source: Alex's decision (to sign)
- Method: A, T
- Verified by: VER-140, VER-141
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-BAT-15: Cell leads protected
- Level: subsystem
- Board: rp2350b
- Subsystem: cell case
- Statement: Both cell leads shall be sleeved (heat-shrink or a printed grommet) where they run along the board's routed hole edge to the cell's header, with 0 bare lead insulation against the FR4 edge, seen in the FAI and after the drop test.
- Rationale: the routed edge wears the insulation and shorts CELL_P to the GND pour, with only the PCM left (DF-214). Derivation (VER-189 fix, 2026-10-29): a design choice for the cell's safety: 0 bare insulation against the FR4 edge, because one worn lead shorts CELL_P to the GND pour with only the PCM left. Earlier cite (not a primary source): Claude's review (F-14).
- Source: Alex's decision (to sign)
- Method: I
- Verified by: VER-109, VER-93
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-BAT-16: Usable capacity per lot
- Level: subsystem
- Board: rp2350b
- Subsystem: cell
- Statement: One cell per lot shall deliver 600 mAh or more from charge-done to the chosen cutoff at 140 mA +-10% (126 to 154 mA), with its capacity to 3.50 V and to 3.60 V recorded.
- Rationale: capacity on low-cost cells is often overstated; play time (REQ-PRD-03.B) and the cutoff choice (DF-225) depend on it. Derivation (VER-189 fix, 2026-10-29): the arithmetic: REQ-PRD-03.B's 4.0 h x 140 mA = 560 mAh is what a session uses, so 600 mAh leaves 40 mAh (7%) of margin; Claude's estimate for the unit is 100 to 120 mA, so 140 mA is 17% to 40% more load than that. The 140 mA is set as 140 mA +-10% (126 to 154 mA) so a bench supply can be read to it. Earlier cite (not a primary source): Claude's proposal (O-58): Alex sets the lot limit.
- Source: Alex's decision (to sign)
- Method: T
- Verified by: VER-153
- Phase: EVT
- Status: PROPOSED

## Subsystem: power path (PWR)

### REQ-PWR-01: USB input range: run
- Level: subsystem
- Board: both
- Subsystem: power usbc
- Statement: With a cell in, the device shall run (3V3 in regulation) from 4.40-5.50 V at the USB-C connector, both plug orientations; with no cell, from 4.75-5.50 V; and it shall not limit-cycle on a source that folds back at 500 mA.
- Rationale: USB 2.0 sources give 4.75-5.25 V; cables drop more; under the BQ24073's VIN-DPM (4.5 V) the input current is cut back and the cell makes up the rest, which a board with no cell cannot.
- Source: USB 2.0 cl. 7.2.1; pin_review/u2.json pin 5 (VIN-DPM 4.5 V on USB500); validation_plan #51, #80.
- Method: T
- Verified by: VER-80, VER-83
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-16: USB input range: charge
- Level: subsystem
- Board: rp2350b
- Subsystem: power usbc
- Statement: From 4.75-5.25 V at the USB-C connector the cell shall charge to done within REQ-PRD-04.B's time; between 4.40 V and 4.75 V the input current, whether the cell supplements the system, and whether the charge completes shall be measured and recorded.
- Rationale: the BQ24073 cuts its input current back at 4.5 V (VIN-DPM), so a weak source charges slowly or not at all.
- Source: Claude's analysis (O-24, F-9); pin_review/u2.json.
- Method: T
- Verified by: VER-80, VER-102
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final. decision for Alex after VER-80: raise the lower bound, or accept (PLAN.md decision DEC-15).

### REQ-PWR-02: Hot-plug peaks
- Level: subsystem
- Board: rp2350b
- Subsystem: power usbc
- Statement: At a live plug-in from an always-on USB-A port with a 2 m cable and no cell, VBUS shall peak at or under the charger's over-voltage minimum less 0.2 V, and every pin fed from VBUS (the charger's, the ESD array's, the CC pull-downs') shall stay 0.3 V or more under its absolute maximum.
- Rationale: REQ-PWR-14's transient rule: EN1 0.3 V under its 7 V absolute maximum (6.7 V), U3's EN (6 V absolute) with margin; the OVP limit from its minimum, not its 6.6 V nominal (O-R18); VSYS is U2's regulated OUT.
- Source: pin_review/u2.json (IN 28 V absolute, OVP 6.6 V, EN1 7 V), pin_review/u3.json (EN 6 V).
- Method: A, T
- Verified by: VER-12, VER-71
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final. bringup.md 5.6 still describes the old charger's simulation (DEF-21; Was: the TP4056 design's). u2.json quotes only OVP's 6.6 V nominal: VER-34 takes SLUS810N's minimum into it (pin_review is outside hardware/vv) before VER-12 is built.

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
- Subsystem: power
- Statement: 3V3 shall stay at 3.30 V +-3% (3.20-3.40 V) for the system rail's range on the cell and on USB and for loads from 0 to the 3V3 budget of REQ-PWR-20.
- Rationale: the RP2350's IOVDD range at 3.3 V (to read from its datasheet once it is on disk: VER-233) and the panel's logic supply, 3.3 V maximum; U3 is the 3V3 regulator (C47773), whose accuracy and load regulation are read from its datasheet and re-derived in VER-235 against the 3V3 budget of REQ-PWR-20.
- Source: the RP2350 datasheet's IOVDD range (to download: VER-233); U3's datasheet DS9013 (accuracy and load regulation); ER-TFT035IPS-6 VCI 2.5-3.3 V recommended (er-tft035ips-6.txt:747-748: up to 3.350 V is 50 mV over, DF-227).
- Method: T
- Verified by: VER-83, VER-235
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-PWR-05: 3V3 through the unplug
- Level: subsystem
- Board: rp2350b
- Subsystem: power
- Statement: Through the USB-to-cell hand-over at full load (backlight 100%, sound playing, the battler running), every rail shall stay inside its range (REQ-PWR-20) and the device shall not reset.
- Rationale: the BQ24073 joins OUT to BAT through its battery FET with no gap, so 3V3 is the cell less the FET, VSYS's copper to U3 and U3's dropout, against the RP2350's and the panel's minimum (VER-235, DF-210).
- Source: bq24074.txt:724; design.py U2, U3; U3's datasheet dropout and the RP2350 datasheet's IOVDD minimum (VER-235; to download: VER-233).
- Method: A, T
- Verified by: VER-70
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

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
- Subsystem: power cell
- Statement: With the device off and no USB, the cell current shall be 30 uA or less at 25 C and 60 uA or less at 45 C, for a cell at 3.7-4.2 V, the hard reset's controller (the TPS3421) being fed from 3V3 and so drawing nothing.
- Rationale: designed under 30 uA at 25 C from the RP board's circuits, kept on the RP2350B board: U2 6.5, R17 into U10's unpowered input 7.9 at a 4.2 V cell, U7 8.4 on a stand-in, the FETs' leakage 3.0, and U3 (C47773) at the shutdown current its datasheet gives, summed in VER-103; leakage rises with temperature (O-51). The TPS3421 (the hard reset, REQ-PRD-08.B) runs from 3V3, not VSYS or the cell (Alex, 2026-10-29): 3V3 is off while the unit is off, so it draws nothing then, and the 30 uA budget and REQ-PRD-05's shelf life are unchanged. It watches seat 1's Start through the 3V3-side buffered power-key line (the RP board's PWR_BTN_N: Q1 with R16 to 3V3), so no VSYS or cell voltage reaches its unpowered input (DF-266, VER-159). Seat 1's Start (the power key) still turns the unit on through its own latch path (PWR_EN, the diode OR of D2), which needs no TPS3421. The hard reset works whenever the unit is on, with USB in or not.
- Source: bringup.md step 5.1; design.py U2, U3, U10, R17, Q1, R16, D2 (the RP board's); Alex, 2026-10-29, in chat (the TPS3421 on 3V3); the 45 C number is Claude's estimate (O-51).
- Method: T
- Verified by: VER-65, VER-146
- Phase: EVT
- Status: PROPOSED
- Notes: VER-189 (2026-10-29): the rationale still said the DS Lite slider turns the unit on; the slider is dropped (DESIGN.md:60), so the sentence now names seat 1's Start. The statement is unchanged.

### REQ-PWR-08: Turn-on margin
- Level: subsystem
- Board: esp32s3
- Statement: With 1 kohm in series with the power key SW24 and the cell at 3.50 V, pressing it shall bring 3V3 to 3.20 V or more within 50 ms and PWR_BTN_N shall read low, with Q1's gate margin over its maximum threshold reported in volts.
- Rationale: a dirty or worn contact adds resistance: at VSYS 3.0 V Q1's gate sees 2.88 V through 1 kohm and SW24's 500 mOhm against a 2N7002 Vgs(th) of up to 2.5 V (VER-33; 2n7002_cj.txt:55, rocpu_tps850.txt:137). Was: either Start dome, through Q1 and Q7.
- Source: design.py SW24, R13; 2n7002_cj.txt:55; rocpu_tps850.txt:20, 137; validation_plan section 1 'Latch replica'.
- Method: T, A
- Verified by: VER-211, VER-33
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the turn-on margin of the power key SW24 through Q1; the RP2350B board's power key is seat 1's Start (REQ-PWR-21, DESIGN.md:60)

### REQ-PWR-09: Turn-off after a reset or crash
- Level: subsystem
- Board: rp2350b
- Subsystem: power soc
- Statement: On the cell with no USB, after a watchdog reset or a panic, the power hold shall release and every rail fall below 0.5 V within 2 s of seat 1's Start being released, unless the firmware takes the hold again in that time.
- Rationale: a crashed unit must turn itself off, not drain the cell: PWR_HOLD (GPIO44) resets as an input, and after RP2350-E9 a released pad can sit near 2 V, so an external pull-down holds it low (R42 4.7k, E9 needs 8.2k or less; analysis/errata_rp2350.md, VER-239, VER-243). Derivation (VER-189 fix, 2026-10-29): a design choice: 2 s matches the watchdog's 2 s in REQ-SOC-04, so a crashed unit takes no longer to turn off than a hung one takes to reset; 0.5 V is the level counted as off. The pull-down is R42 4.7k against RP2350-E9's 8.2k limit (analysis/errata_rp2350.md). Earlier cite (not a primary source): design.py R42 (PWR_HOLD's pull-down); pinmap.md Power control; analysis/errata_rp2350.md.
- Source: Alex's decision (to sign)
- Method: T
- Verified by: VER-243, VER-105
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-PWR-10: No hang keeps it on
- Level: subsystem
- Board: both
- Subsystem: power soc
- Statement: Without USB, no firmware hang shall keep the device on for more than 10 s; every image, the self-test included, shall enable the watchdog before any blocking init.
- Rationale: hardware alone cannot tell a hang with PWR_HOLD high from normal play. Derivation (VER-189 fix, 2026-10-29): 10 s is the watchdog's 2 s (REQ-SOC-04) plus the release of REQ-PWR-09's 2 s, which is 4 s, with 2.5 times margin (10 / 4) for a slow image to reach its first watchdog enable. A design choice. Earlier cite (not a primary source): validation_plan gate item 3 and Fable's note 1; pinmap.md.
- Source: Alex's decision (to sign)
- Method: I, T
- Verified by: VER-104, VER-105
- Phase: EVT
- Status: PROPOSED

### REQ-PWR-11: No turn-on from leakage
- Level: subsystem
- Board: rp2350b
- Subsystem: power buttons
- Statement: With 1 Mohm across seat 1's Start's contacts and the cell at 4.2 V, the device shall stay off (every rail below 0.5 V) for 10 min.
- Rationale: seat 1's Start's contact carries a rail on one pad (VSYS through a diode to PWR_EN); sweat, flux or a pressed cap can bridge it (DF-242). Only seat 1's Start is on the power path (Was: both Start domes on the RP board).
- Source: design.py R13, R15 and Q1 on the power key's line (the RP board's, kept on the RP2350B board, now fed from seat 1's Start); 2n7002_cj.txt:55; VER-239's output.
- Method: T, A
- Verified by: VER-240
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-PWR-12: USB input current
- Level: subsystem
- Board: rp2350b
- Subsystem: power usbc
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
- Board: rp2350b
- Subsystem: power usbc
- Statement: The board shall present 10 uF or less on VBUS and take 50 uC or less at a plug-in at 5.25 V (USB 2.0 cl. 7.2.4.1), each capacitor at its tolerance and its DC bias, when the console is a sink.
- Rationale: USB 2.0's inrush limit is 10 uF or 50 uC at the plug; the old power path's 265 uC came from its diode charging VSYS (Was: the TP4056 design's D5, which left in 2d1124d).
- Source: USB 2.0 cl. 7.2.4.1; validation_plan #125; design.py C1, U2.
- Method: A
- Verified by: VER-99
- Phase: ORDER
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final. no deviation is needed at be437f8 (VER-99). U2's current limit while it brings VSYS up is a datasheet claim; VER-71's scope picture shows it.

### REQ-PWR-14: Derating
- Level: subsystem
- Board: rp2350b
- Subsystem: all
- Statement: In every mode (off, cell, USB, charging, hot plug, unplug): each IC's steady-state stress shall be at or below its recommended operating maximum and each transient at least 0.3 V below its absolute maximum; each passive and discrete (MLCC voltage, FET Vds and Vgs, diode VR and IF, resistor power) shall be at 80% or less of its rating, 90% for transients under 10 us; every exception named here.
- Rationale: the old rule (80% of every absolute maximum) failed by construction on every 3.3 V part, so it could not tell an intended stress from an oversight (O-22). Derivation (VER-189 fix, 2026-10-29): a design rule: the 80% (90% under 10 us) and the 0.3 V below absolute maximum are the margins Claude proposed in O-22 for passives and ICs; no standard is cited. The panel's VCI/VDDI exception is named in the Notes. Earlier cite (not a primary source): validation_plan #5, #123; Claude's proposal, revised (O-22).
- Source: Alex's decision (to sign)
- Method: A
- Verified by: VER-31, VER-36
- Phase: EVT
- Status: PROPOSED
- Notes: the phase moved to EVT with VER-36 (F-R2(d)). Named exception: (2) the panel's VCI and VDDI at up to 3.333 V (3.350 V at TJ 125 C) against 3.3 V recommended (DF-227). Exception (1), U10's input at VBAT/2 with VDD at 0 V, is inside the MCP6001's own limits since its datasheet is on disk (2 mA, VDD + 1.0 V: mcp6001.txt:115-117, VER-31).

### REQ-PWR-15: 3V3 budget
- Level: subsystem
- Board: esp32s3
- Statement: The 3V3 load in a Wi-Fi TX burst shall be 80% or less of U3's 1 A, U3's current limit shall be over the module's 0.5 A supply minimum, and U3's junction shall meet REQ-THM-04's per-mode limits.
- Rationale: since 39cc07f the module's Wi-Fi peaks at 355 mA (esp32-s3-wroom-1.txt:1662) and Espressif asks a 0.5 A supply (1597); the TLV75733P is rated 1 A (tlv757p.txt:209) with a 1.2 A current limit (267); its heat is VER-144's (DF-31).
- Source: tlv757p.txt:209, 267; esp32-s3-wroom-1.txt:1597, 1662-1741.
- Method: A, T
- Verified by: VER-31, VER-83, VER-144
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the 3V3 budget in a Wi-Fi burst against the TLV75733P's 1 A; the RP2350B board has no Wi-Fi, and its rails are REQ-PWR-20

### REQ-PWR-17: A faulty USB source
- Level: subsystem
- Board: rp2350b
- Subsystem: power usbc
- Statement: A source putting 6-20 V on VBUS (a broken charger, a PD source stuck high) shall over-stress no part: the lowest VBUS that takes a part over its absolute maximum shall be 20 V or more, or the value Alex accepts.
- Rationale: U2 blocks the fault from VSYS (OVP 6.6 V, 28 V absolute), but PWR_EN through R14/D3 takes U3's EN (6 V absolute) over at 9.1 V and EN1 through R43 takes U2's EN1 (7 V) over at 12.4 V (VER-177, with R14 47k and R43 220k; Was: 6.9 V and 9.5 V with 10k and 100k).
- Source: pin_review/u2.json, u3.json; Claude's analysis (O's sign-off objection on the cell path, DF-222).
- Method: A, T
- Verified by: VER-177, VER-97
- Phase: EVT
- Status: PROPOSED
- Notes: DEC-17's BOM half is in design.py at 71737ae (R14 47k, R43 220k: 9.1 V, DEF-50 FIXED); 20 V needs a clamp (copper); proposed: Alex accepts 9.1 V for EVT as this statement's value. The decision is made before the order, because DEF-50 keeps Gate ORDER and closes on it; the check (VER-177) binds at G-EVT-04, so an acceptance for EVT does not leave a failing ORDER check behind (O-R9; F-R2(f) wanted VER-177 at ORDER, but DEF-50's Gate already holds the BOM decision there).

### REQ-PWR-18: The power key
- Level: subsystem
- Board: esp32s3
- Statement: The power key SW24 shall turn the device on with a press held 1 s, leave it off after a press under 1 s, and turn it off with a long press; its cap shall sit 0.4-0.6 mm under the deck's face and need 300 gf or less, and nothing pressing the case face down shall press it.
- Rationale: the dedicated key since 39cc07f (Alex, 2026-10-28): a ROCPU TPS850 J14K (250 +-50 gf, 0.2 +0.1 mm, rocpu_tps850.txt:20, 33) under a printed cap (case.scad pwr_cap); pinmap.md's Power control gives the hold rule.
- Source: design.py SW24; case.scad pwr_*; rocpu_tps850.txt:17-33; pinmap.md Power control.
- Method: T
- Verified by: VER-211, VER-213, VER-33
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the power key SW24; the RP2350B board's power key is seat 1's Start (REQ-PWR-21)

### REQ-PWR-19: A jig on J7 does not back-feed U3
- Level: subsystem
- Board: esp32s3
- Statement: No procedure shall drive J7's 3V3 pad while JP1 is bridged and VSYS is 0 V; the bring-up and jig procedures shall say how J7 is powered.
- Rationale: U3's OUT over its IN + 0.3 V is over the TLV75733P's absolute maximum and runs current back through its body diode (tlv757p.txt:180, 1137-1138); design.py's J7 note invites exactly that (DF-189).
- Source: tlv757p.txt:180, 1129-1142; design.py J7; VER-159.
- Method: A, I
- Verified by: VER-159, VER-212
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): J7's 3V3 pogo pad back-feeding U3; the lesson carries as DF-241 on the RP2350B board's debug connector

### REQ-PWR-20: The RP2350B's rails
- Level: subsystem
- Board: rp2350b
- Subsystem: power soc storage
- Statement: The board shall supply the RP2350B's IO and USB and ADC rails and the flash's VCC at 3.3 V, and its 1.1 V core from the chip's on-chip regulator with the inductor, capacitors and VREG AVDD filter of Raspberry Pi's minimal design, each inside the RP2350 datasheet's and the GD25Q128 datasheet's recommended range at the chip's peak load and the regulator's tolerance.
- Rationale: the RP2350B has no flash inside, so the flash is a second load on the 3.3 V rail; its core is made on the chip. The ranges are the datasheets' (not on disk: VER-233).
- Source: next_spin_requirements.md:26 ('Raspberry Pi's 2-layer RP2350B minimal design, flash included, copied as drawn'); the RP2350 datasheet and the GD25Q128 datasheet (to download: VER-233).
- Method: A, T
- Verified by: VER-235, VER-231, VER-83, VER-249
- Phase: ORDER
- Status: PROPOSED
- Notes: Rewritten on 2026-10-29. It was the F1C200s's core, DRAM and IO rails and their order (tag vv-f1c200s-final).

### REQ-PWR-21: The power key: seat 1's Start
- Level: subsystem
- Board: rp2350b
- Subsystem: power buttons case
- Statement: Seat 1's Start shall be the power key: pressing it shall power the regulator, the firmware shall latch PWR_HOLD only after Start has been held 2.5 s (2 to 3 s), and if it is released earlier the power shall drop and the unit be fully off; with the unit on, the firmware shall turn it off on a 1 to 2 s hold. There shall be no power slider, slide switch, case slot for one or board-edge switch.
- Rationale: Alex, 2026-10-29: the DS Lite power slider is dropped and seat 1's Start is the power key. Start's own membrane contact also feeds the power-latch path as the RP board's SW21 did (Start joins VSYS to PWR_EN through a diode; the MCU reads it through the buffered line). The 2 to 3 s hold to turn on is so a bump in a pocket does not switch it on (DESIGN.md:60); this reverses the 'no hold-to-latch' change made earlier the same day (REQ-PRD-15). The 1 to 2 s hold to turn off is the firmware's; about 8 s is the hard reset of REQ-SOC-04, which works whatever the firmware is doing.
- Source: DESIGN.md:60 ('Power is seat 1's Start button: hold it for 2 to 3 seconds to turn on, so a bump in a pocket doesn't switch it on, and hold it to turn off. (2026-10-29)'); Alex, 2026-10-29, in chat (the 2.5 s hold, the 1 to 2 s off hold, no slider).
- Method: A, T
- Verified by: VER-239, VER-240
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-29 at Alex's decision. It was the DS Lite power slider, with seat 1's Start as the fallback and no hold to turn on.

### REQ-PWR-22: VBUS out when the console is host
- Level: subsystem
- Board: f1c200s
- Subsystem: power usbc
- Statement: As USB host the console shall source 5.0 V +-5% on VBUS at 500 mA or more from the cell, switched on only after the FUSB303B sees a sink, and shall limit a short on VBUS to 1.5 A or less with no damage.
- Rationale: a phone tether, a dongle and the other console of a link need VBUS from the host. 500 mA and 1.5 A are Claude's proposal.
- Source: next_spin_requirements.md:16-17 ('a USB port that can be host or device ... so one console hosts the other'; 'USB host for a phone tether or a Wi-Fi dongle').
- Method: A, T
- Verified by: VER-234, VER-225
- Phase: ORDER
- Status: DROPPED
- Dropped: DROPPED with f1c200s (tag vv-f1c200s-final): USB host mode left the design on 2026-10-29: the console is a USB device and never sources VBUS; its statement is at the tag

## Subsystem: core (MCU), the ESP32-S3 module: DROPPED with the ESP32-S3 board

### REQ-MCU-01: Boot from blank
- Level: subsystem
- Board: esp32s3
- Statement: A blank board on USB with no cell shall enumerate as the ESP32-S3's USB Serial/JTAG within 5 s and take an image over it.
- Rationale: proves the module, its supply, EN and USB in one step (the module carries flash, PSRAM and crystal).
- Source: esp32-s3-wroom-1.txt:810-837 (boot modes); bringup.md step 1.
- Method: T
- Verified by: VER-55
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 module's requirement; the F1C200s's are REQ-SOC-01 to -08

### REQ-MCU-02: USB full speed
- Level: subsystem
- Board: esp32s3
- Statement: D+ and D- shall each pass a series resistor of 22-33 ohm placed within 5 mm of the module's pin, stubs shall be 3.5 mm or shorter, and the device shall enumerate on 3 different hosts.
- Rationale: Espressif's guidelines: 'initial value can be 22/33', 'close to the chip' (esp32-s3-hw-design-guidelines.txt:873-875, 1307); at 39cc07f they sit 11.5 mm away (VER-191, DF-198). Was: the RP2350's 27 ohm.
- Source: esp32-s3-hw-design-guidelines.txt:873-875, 1304-1313; pin_review/j1.json.
- Method: A, T
- Verified by: VER-191, VER-108, VER-80
- Phase: ORDER
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 module's requirement; the F1C200s's are REQ-SOC-01 to -08

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
- Board: esp32s3
- Statement: Firmware shall flash and log over USB-C through the ESP32-S3's USB Serial/JTAG at 1 line or more a second, and J7's pads (3V3, EN, BOOT, GND) shall reset the module and enter its download mode from a pogo jig.
- Rationale: bring-up and recovery (Was: SWD on J5 and J7, the UART header); the module's USB Serial/JTAG carries both the flash and the log (docs/living/esp32s3_board_proposal.md item 7).
- Source: docs/living/esp32s3_board_proposal.md; design.py J7; esp32-s3-wroom-1.txt:860-871.
- Method: T
- Verified by: VER-212, VER-02
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 module's requirement; the F1C200s's are REQ-SOC-01 to -08

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
- Board: esp32s3
- Statement: Every module pin shall carry only a function that pin has in the module's Table 3-1, ADC nets on ADC1 channels (Wi-Fi on), USB on IO19 and IO20, IO35-IO37 unconnected, and no load-switching output on a pin with a high-level power-up glitch.
- Rationale: a pin that cannot do its job is a respin; ADC2 cannot run with Wi-Fi (esp32-s3.txt:3415); the N16R8's PSRAM owns IO35-IO37 (esp32-s3-wroom-1.txt:691-693); Table 2-2 lists the glitching pins (esp32-s3.txt:958-1006). Was: the RP2350's function table and PWM slices, VER-15.
- Source: esp32-s3-wroom-1.txt:629-696; esp32-s3.txt:958-1006, 3415.
- Method: A
- Verified by: VER-203
- Phase: ORDER
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 module's requirement; the F1C200s's are REQ-SOC-01 to -08
- Notes: the module pin, GPIO and net table is VER-203's evidence; the GPIO matrix routes the panel, the SD bus and the LEDC to any pin, so only ADC1 and USB are fixed (docs/living/esp32s3_board_proposal.md 'Pin map').

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
- Board: esp32s3
- Statement: Each unit shall report an ESP32-S3 with 16 MB of flash and 8 MB of PSRAM at 3.3 V VDD_SPI (the N16R8), and its chip revision, MAC and eFuse summary shall be recorded at first boot (1 record per unit).
- Rationale: a substituted N16R16VA runs its PSRAM at 1.8 V and a WROOM-1 has a PCB antenna; the chip revision decides which errata apply (VER-214).
- Source: esp32-s3-wroom-1.txt:130-171.
- Method: T
- Verified by: VER-210
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 module's requirement; the F1C200s's are REQ-SOC-01 to -08

### REQ-MCU-09: Core supply
- Level: subsystem
- Board: esp32s3
- Statement: The module's 3V3 pin shall have 22 uF and 0.1 uF within 5 mm, EN a 10k and 1 uF RC that rises past VIH_nRST after the rails, and 3V3 at the pin shall stay inside VDD33's 3.0-3.6 V through a Wi-Fi TX burst at the cutoff.
- Rationale: Espressif's peripheral schematic and guidelines (esp32-s3-wroom-1.txt:2875; esp32-s3-hw-design-guidelines.txt:299-303, 338); at 39cc07f C10 sits 23.6 mm from pin 2 and 3V3 reaches 2.976 V in a burst at 3.50 V (VER-204, DF-187). Was: the RP2350's 1V1 core supply.
- Source: esp32-s3-wroom-1.txt:931, 1596, 2875; esp32-s3-hw-design-guidelines.txt:299-303, 316-338.
- Method: A, T
- Verified by: VER-204, VER-208
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 module's requirement; the F1C200s's are REQ-SOC-01 to -08

### REQ-MCU-10: Flash budget
- Level: subsystem
- Board: esp32s3
- Statement: The firmware image shall fit its app partition in the module's 16 MB flash at 80% or less, with a written partition table (bootloader, app, settings, battle log, game data), checked in scripts/check.sh on every change.
- Rationale: the 2 MB of the MCU before it was the reason for the change (docs/living/esp32s3_board_proposal.md); the partition table is what can still run out.
- Source: docs/living/esp32s3_board_proposal.md; esp32-s3-wroom-1.txt:130-150.
- Method: A
- Verified by: VER-216, VER-185
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 module's requirement; the F1C200s's are REQ-SOC-01 to -08
- Notes: the phase moved from ORDER to EVT at 39cc07f: with 16 MB the order freezes no flash decision (G-ORD-23 retired).

### REQ-MCU-11: A bring-up image for the ordered chip
- Level: subsystem
- Board: esp32s3
- Statement: Before the boards arrive, a self-test image shall build for the ESP32-S3 (esp-hal) from pin definitions generated from design.py, with 0 hand-typed GPIO numbers.
- Rationale: the image is needed the day the boards arrive (round 2's proportionality objection); the GPIO choices that copper freezes are VER-203's.
- Source: docs/living/esp32s3_board_proposal.md 'Firmware with Wi-Fi'; Claude's review (F-44, O-37, O-56).
- Method: A
- Verified by: VER-149, VER-215
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 module's requirement; the F1C200s's are REQ-SOC-01 to -08

### REQ-MCU-12: Strapping pins at reset
- Level: subsystem
- Board: esp32s3
- Statement: At every reset the module shall read IO0 high (SPI boot), IO45 low (VDD_SPI 3.3 V for the N16R8's flash and PSRAM), IO46 low, and IO3 driven, from the board's resistors and drivers alone, with no eFuse burnt that changes them.
- Rationale: a high IO45 runs the 3.3 V flash and PSRAM at 1.8 V: no boot; IO0 low is the download mode (esp32-s3-wroom-1.txt:714-754, 810-899); AMP_EN and CHG_OFF share IO45 and IO46.
- Source: esp32-s3-wroom-1.txt:748-754, 816-817, 849, 876-899; design.py R37, R44.
- Method: A, T
- Verified by: VER-203, VER-55
- Phase: ORDER
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 module's requirement; the F1C200s's are REQ-SOC-01 to -08

### REQ-MCU-13: The module's ambient
- Level: subsystem
- Board: esp32s3
- Statement: The air round the module shall stay at or under 65 C (the N16R8's ambient limit) in every mode at 40 C outside, or the firmware shall enable the PSRAM's ECC (85 C).
- Rationale: the N16R8 and N16R16V modules' octal PSRAM limits them to 65 C ambient, 85 C with ECC on and 1/16 of the PSRAM lost (esp32-s3-wroom-1.txt:102-105, 1598); at 39cc07f VER-193 puts it at 84.9 C on a stand-in (DF-193).
- Source: esp32-s3-wroom-1.txt:102-105, 1598.
- Method: A, T
- Verified by: VER-193, VER-86
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 module's requirement; the F1C200s's are REQ-SOC-01 to -08

### REQ-MCU-14: The module's errata
- Level: subsystem
- Board: esp32s3
- Statement: Every erratum Espressif lists for the ESP32-S3's chip revision on the board shall be answered for this design: applies or not, and its fix or firmware rule.
- Rationale: the errata table of the MCU before it set two BOM values (Was: VER-16); the ESP32-S3's errata may set firmware rules.
- Source: Espressif's ESP32-S3 errata (not on disk: VER-214 BLOCKED).
- Method: I
- Verified by: VER-214
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 module's requirement; the F1C200s's are REQ-SOC-01 to -08

## Subsystem: the chip (SOC), the RP2350B and its flash, since 2026-10-29

### REQ-SOC-01: Run in place from the external flash
- Level: subsystem
- Board: rp2350b
- Subsystem: soc storage
- Statement: At every power-on the RP2350B's boot ROM shall find the bare-metal image in the GD25Q128 on its QSPI pins and run it in place (XIP), 20 of 20 power-ons, with the whole 16 MB read back through the XIP window matching the image's CRC and the flash's JEDEC ID reading as the GD25Q128's.
- Rationale: the RP2350B has no flash inside; the one 16 MB flash holds the program and the sound. The GD25Q128's ID is 0xC84018 from memory (to confirm from its datasheet: VER-233).
- Source: DESIGN.md:74; next_spin_requirements.md:13, :24-25 (the chip, the flash).
- Method: T
- Verified by: VER-252, VER-237
- Phase: ORDER
- Status: PROPOSED
- Notes: Rewritten on 2026-10-29. It was the F1C200s's boot from the microSD card (tag vv-f1c200s-final). VER-189 (2026-10-29): DESIGN.md line numbers corrected after the Sleep line moved them (this row and REQ-SOC-02, -08, -USB-01, -USB-02, -ASM-01.B).

### REQ-SOC-02: The bare-metal Rust core on the RP2350B
- Level: subsystem
- Board: rp2350b
- Subsystem: soc
- Statement: The no_std Rust battler core shall build for the RP2350B's Cortex-M33 cores and run from the flash in place with its data in the chip's 520 KB of SRAM, with today's 420 KB of RAM and 1.4 MB of code and data, and 100 KB of SRAM or more free.
- Rationale: a bare-metal Rust boot is required; the core's size on the RP2350 today (the stripboard Pico 2, an RP2350A with the same cores and SRAM). The 100 KB free is Claude's proposal.
- Source: next_spin_requirements.md:13 ('about 420 KB of RAM and 1.4 MB of code and data on the RP2350 today'); DESIGN.md:87.
- Method: T, A
- Verified by: VER-252, VER-237
- Phase: ORDER
- Status: PROPOSED
- Notes: Rewritten on 2026-10-29. It was the F1C200s's ARM9 running from its DRAM (tag vv-f1c200s-final).

### REQ-SOC-03: The crystal
- Level: subsystem
- Board: rp2350b
- Subsystem: soc
- Statement: The RP2350B's 12 MHz crystal shall be oscillating and stable within 10 ms of 3V3 valid at every power-up across the cell's range and on USB, with the load capacitors and series resistor of Raspberry Pi's minimal design.
- Rationale: the clock the chip and its USB derive from; 12 MHz is the RP2350's reference clock (to confirm: the datasheet is not on disk). Derivation (VER-189 fix, 2026-10-29): neither the RP2350 datasheet nor the crystal's is on disk, so the start-up time is a design choice: the boot budget is 1.0 s after the hold (REQ-PRD-15) and 10 ms is 1% of it (10 ms / 1000 ms). VER-237 measures it on the first unit. The frequency tolerance is the crystal part's, chosen at NM-04, and is not set here. Earlier cite (not a primary source): the RP2350 datasheet and the minimal design (to download: VER-233).
- Source: Alex's decision (to sign)
- Method: A, T
- Verified by: VER-198, VER-237
- Phase: EVT
- Status: PROPOSED

### REQ-SOC-04: Reset with no reset button
- Level: subsystem
- Board: rp2350b
- Subsystem: soc power
- Statement: With no reset button, the RP2350B shall come out of reset by its power-on reset at every power-up, a hung firmware shall reset by the watchdog within 2 s, and holding seat 1's Start for 8 s (6 s to 10 s: Alex, 2026-10-29) shall pull the RP2350B's RUN pin low through a push-button controller fed from 3V3 and so reset and turn off the unit, whatever the firmware is doing, whenever the unit is on, with USB in or not.
- Rationale: there is no reset button; the watchdog and a long hold of seat 1's Start are the ways out of a hang (REQ-PWR-10). Start's 8 s hold is the hard reset of REQ-PRD-08.B: a TI TPS3421 (LCSC C2067262) drives RUN low after the hold; a reset releases PWR_HOLD, so the unit turns off, and with USB in it restarts. The TPS3421 is fed from 3V3 (Alex, 2026-10-29), so it draws nothing while the unit is off (REQ-PWR-07), and reads Start through the 3V3-side buffered power-key line; Start turns the unit on through its own latch path, so the hard reset is needed only while the unit is on. A hung image that has stopped the watchdog is the case the watchdog cannot answer.
- Source: DESIGN.md:64 ('There is no separate reset button. (2026-10-28)'); DESIGN.md:65 (hold seat 1's Start about 8 s); Alex, 2026-10-29, in chat (the hard reset, REQ-PRD-08.B; 'about 8 s' read as 6 s to 10 s); the TPS3421 datasheet (VER-258).
- Method: A, T
- Verified by: VER-198, VER-105, VER-258, VER-241
- Phase: EVT
- Status: PROPOSED
- Notes: VER-189 (2026-10-29) found 'about 8 s' with no tolerance. Alex decided on 2026-10-29: 6 s to 10 s. This row carries the number; REQ-PRD-08.B (signed) keeps his words 'about 8 s' and reads them as this range. VER-258 checks that a TPS3421 variant has its delay inside 6 s to 10 s over temperature and supply.

### REQ-SOC-05: The pin plan
- Level: subsystem
- Board: rp2350b
- Subsystem: soc display storage usbc audio buttons
- Statement: Every function shall sit on an RP2350B pin that has it in the datasheet's pin table: the display bus and its control lines, the audio output, the 20 contacts (seat 1's Start among them, also the power key), the cell sense on an ADC pin, a UART or the SWD pins for the debug connector, with USB on the chip's own USB pins and the flash on the QSPI pins, 0 mux conflicts, and no function left on a GPIO the datasheet reserves.
- Rationale: a QFN-80 has 48 GPIOs and fixed special pins (QSPI, USB, ADC on GPIO40 to 47); a function on a pin without it is a respin. The 20 inputs take 20 of the 48 GPIOs, read directly (Alex, 2026-10-29; the power slider is dropped, so one input fewer); VER-262 counts the budget, and the 74HC165 chain is the first thing to change if it runs short.
- Source: next_spin_requirements.md:12, :24-25; the RP2350 datasheet (to download: VER-233).
- Method: A, I
- Verified by: VER-232, VER-262
- Phase: ORDER
- Status: PROPOSED

### REQ-SOC-06: Chip and flash identity per unit
- Level: board
- Board: rp2350b
- Subsystem: soc storage
- Statement: Each unit shall report an RP2350B in a QFN-80 and a GD25Q128 by its JEDEC ID, with its 16 MB reading back without error, logged against its serial.
- Rationale: a substituted or remarked chip or flash (a 4 MB or 8 MB flash, a part with its own flash inside, a QFN-60 RP2350A) is caught on the bench.
- Source: next_spin_requirements.md:24-25 (the chosen chip and flash).
- Method: T
- Verified by: VER-242
- Phase: EVT
- Status: PROPOSED

### REQ-SOC-07: The chip's heat
- Level: subsystem
- Board: rp2350b
- Subsystem: soc case
- Statement: The RP2350B's temperature shall stay inside its datasheet limit in every mode at 40 C ambient in the closed case.
- Rationale: the limit is the datasheet's (not on disk).
- Source: the RP2350 datasheet (to download: VER-233); IEC 62368-1 for the skin (REQ-THM-01).
- Method: A, T
- Verified by: VER-238, VER-86, VER-257
- Phase: EVT
- Status: PROPOSED

### REQ-SOC-08: Debug access and the battle log
- Level: subsystem
- Board: rp2350b
- Subsystem: soc
- Statement: With the case open, the debug connector shall give the RP2350B's SWD pins and a UART console and the rails' test pads; debug builds shall keep every battle line in a log region of the flash and show the log on the next boot, and production builds shall have it off.
- Rationale: bring-up needs a console; the battle log is Alex's debug aid.
- Source: DESIGN.md:67 ('The board has test pads and a debug connector for bring-up.'); DESIGN.md:68 ('Debug builds can log every battle line to flash and show the log on the next boot. Production builds have this switched off.').
- Method: D
- Verified by: VER-158, VER-241
- Phase: EVT
- Status: PROPOSED

### REQ-SOC-09: The flash's footprint and wiring follow the minimal design
- Level: subsystem
- Board: rp2350b
- Subsystem: soc storage
- Statement: The GD25Q128's land pattern shall match its package drawing, and its wiring (QSPI SCLK, QSPI SS, QSPI SD0 to SD3 to the RP2350B's QSPI pins of the same name, its VCC and decoupling, and the QSPI SS line's pull-up and BOOTSEL pad) shall match Raspberry Pi's 2-layer RP2350B minimal design net for net, with 0 differences, and no QSPI track shall be more than 2 mm longer than the minimal design's longest.
- Rationale: the core layout is copied from Raspberry Pi's minimal design, flash included (Alex, 2026-10-29); a swapped SD1 and SD2, a wrong land or a long QSPI track is a board that does not boot. The 2 mm is Claude's proposal.
- Source: next_spin_requirements.md:26 ('Raspberry Pi's 2-layer RP2350B minimal design, flash included, copied as drawn'); hardware/ref_design/board/lib/battler.pretty/RP2350B_QFN80_RPi.kicad_mod (the chip's footprint from it, under RPI-RP2350B-MINIMAL_LICENSE.txt); the GD25Q128 datasheet (to download: VER-233).
- Method: A, I
- Verified by: VER-253, VER-254, VER-125
- Phase: ORDER
- Status: PROPOSED
- Notes: New on 2026-10-29. The earlier RP board's copied core is at tag vv-rp2354b-final (its in-package flash needed none of this).

### REQ-SOC-10: Bootloader, partitions and A/B firmware in the flash
- Level: subsystem
- Board: rp2350b
- Subsystem: soc storage
- Statement: The 16 MB flash shall hold a partition table, a bootloader of our own in its own partition write-protected with the GD25Q128's block protection, two firmware slots A and B with a hash in each image, the sound data and a spare region for settings and the debug log. At power-on the bootloader shall read seat 1's Select by GPIO alone, before the display, sound or USB start, and enter the RP2350's USB boot mode if it is held, and otherwise start the main firmware. A failed update shall fall back to the previous firmware through the boot ROM's try-before-you-buy, and a failed-boot counter in a watchdog scratch register shall send the unit to USB boot mode after 3 failed boots. With the case open, a BOOTSEL test pad on QSPI SS shall force USB boot.
- Rationale: the design facts under REQ-PRD-08.B and REQ-PRD-19. The layout is Claude's proposal: partition table 4 KB, bootloader 64 KB (locked), firmware A 2 MB, firmware B 2 MB (a program is 1.4 MB today), sound data (REQ-PRD-18: 2 MB or more of room), and the spare. Reading Select by GPIO alone before anything else starts means no display, sound or USB fault can stop the recovery.
- Source: Alex, 2026-10-29, in chat (REQ-PRD-08.B and -19); the RP2350 datasheet's partition table, hash check and try-before-you-buy (to download: VER-233; VER-259 reads them); the GD25Q128 datasheet's block protection (VER-259).
- Method: A, T, D
- Verified by: VER-259, VER-241, VER-260
- Phase: EVT
- Status: PROPOSED
- Notes: New on 2026-10-29. Open to confirm: how the boot ROM picks slot A or B for a dragged firmware .uf2; whether the recovery .uf2 can rewrite a bootloader locked by block protection (DF-263); that a normal power-off or the hard reset does not count as a failed boot (DF-264).

### REQ-SOC-11: The Sleep save in the flash's spare area
- Level: subsystem
- Board: rp2350b
- Subsystem: soc storage
- Statement: The flash's spare region shall hold one battle snapshot of 16 KB or less carrying the firmware's build ID and a check value, written within 1 s of Sleep being chosen; a power cut at any moment of the write shall leave either the previous valid save or no valid save, never a half-written one that resumes; at power-on a save with another build ID, or one that fails its check, shall be discarded and the unit start at the lobby.
- Rationale: REQ-PRD-20 needs the battle to survive the cell being unplugged, so it goes to flash; the spare region is REQ-SOC-10's, beside the settings and the debug log. The 16 KB is Claude's margin over Alex's 'a few KB'. The firmware's build ID is what makes an older snapshot's layout unreadable by newer firmware; the check value catches a torn write. The write must not wear the log's sectors out (DF-259): Sleep is rare, so the save rotates over a few sectors of the region (DF-267).
- Source: Alex, 2026-10-29, in chat (REQ-PRD-20: a snapshot of a few KB in the spare area, the build ID, the discard rules); the GD25Q128 datasheet's sector erase time, program time and endurance (VER-264 reads them).
- Method: A, T
- Verified by: VER-264, VER-263
- Phase: EVT
- Status: PROPOSED
- Notes: New on 2026-10-29. The 16 KB, the rotation over sectors and the all-or-nothing write are Claude's proposals.

## Subsystem: USB-C device (USB), since 2026-10-29

### REQ-USB-01: USB-C as a device
- Level: subsystem
- Board: rp2350b
- Subsystem: usbc
- Statement: The USB-C port shall attach as a device (a sink, Rd 5.1 kohm on CC1 and on CC2, no role controller) to an Android phone, a computer or a charger, in both plug orientations, 20 of 20 plug-ins each way, enumerate as the console's WebUSB device, and never source VBUS.
- Rationale: the console is a USB device only (DESIGN.md:86); the phone is the host and carries the network.
- Source: DESIGN.md:86; next_spin_requirements.md:16 ('a USB device port'); USB Type-C 1.x (Rd on each CC, to confirm the tolerance).
- Method: T, A
- Verified by: VER-251, VER-195, VER-80
- Phase: ORDER
- Status: PROPOSED
- Notes: Rewritten on 2026-10-29. It was the FUSB303B's host and device roles and the OTG ID pin (tag vv-f1c200s-final).

### REQ-USB-02: The USB data pair
- Level: subsystem
- Board: rp2350b
- Subsystem: usbc
- Statement: D+ and D- shall run side by side from the receptacle to the RP2350B over unbroken ground on the 2-layer board, through the series resistors of Raspberry Pi's minimal design, length-matched within 2 mm with stubs of 3.5 mm or less, for full speed (12 Mbps).
- Rationale: the RP2350's USB is full speed, so no 480 Mbps impedance target is claimed; the pair is still kept tight and over ground for ESD and noise. The 2 mm and 3.5 mm are Claude's proposal from USB 2.0.
- Source: USB 2.0 cl. 7.1; DESIGN.md:62 (two layers); the RP2350 datasheet and the minimal design (to download: VER-233).
- Method: A
- Verified by: VER-236, VER-108
- Phase: ORDER
- Status: PROPOSED
- Notes: Rewritten on 2026-10-29. It was the F1C200s's high-speed pair at 90 ohm (tag vv-f1c200s-final).

## Subsystem: display (DSP)

### REQ-DSP-01: Panel interface
- Level: subsystem
- Board: rp2350b
- Subsystem: display soc
- Statement: The panel shall run its 8-bit 8080 bus (IM2-0 = 011, CSX and RDX as the bus needs) from the RP2350B through a PIO state machine and DMA, or from GPIO if the chip gate shows PIO cannot (VER-224).
- Rationale: proves the interface straps and the bus: the panel is on the 8-bit 8080 bus (er-tft035ips-6.txt:718; docs/living/display_parallel_proposal.md). Was: 4-wire SPI.
- Source: er-tft035ips-6.txt:709-722; design.py PANEL.
- Method: T
- Verified by: VER-224, VER-60
- Phase: ORDER
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-DSP-02: Write speed
- Level: subsystem
- Board: rp2350b
- Subsystem: display soc
- Statement: 8080 writes at the firmware's write clock shall draw 10 full 320 x 480 frames of a checker pattern with 0 corrupted pixels, the clock inside the ILI9488's write cycle with its tolerance.
- Rationale: the ILI9488's parallel write cycle is 40 ns (twc, ili9488.txt:16159), so 25 MHz is the limit with 0 ns spare (VER-199, DF-206); RGB565 is two bytes a pixel over the 8-bit bus.
- Source: ili9488.txt:16159-16172; pinmap.md Display.
- Method: T
- Verified by: VER-61, VER-199
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-DSP-03: Panel pin map
- Level: subsystem
- Board: rp2350b
- Subsystem: display
- Statement: J3 pin j shall carry panel pin 51 - j for all 50 contacts, so LED_A (panel pin 1) is on J3 pin 50.
- Rationale: the folded FPC lands its contact 1 at J3's far end; a reversed map puts VSYS on a logic pin.
- Source: design.py display section (J3's map, the same on the RP board); pin_review/panel.json; the ER-TFT035IPS-6's map since 79de1f7.
- Method: A, T
- Verified by: VER-02, VER-136
- Phase: ORDER
- Status: PROPOSED
- Notes: the map is a fold, not a mirror: once the contacts face the connector's contacts, the tail's contact strip can meet a bottom-contact FH12-50S in only one way, and the product's and the loose breakout's panel-side pairing stays 51-j (F-R1). What is not proved is the bought breakout's header numbering: VER-224 finds it by diode test (which header pin is panel 1), checks the GND pins' continuity, records the header pin each panel pin lands on and wires the dev board by the panel's own pin names (1 LEDA, 2-3 LEDK, 7-9 IM, 10 RESET, 33 SDO, 34 SDA, 36 SCL, 37 D/CX, 38 CSX, 40-42 VDDI/VCI, 11-32 GND). VER-136 adds the fold's reach and seating, not a different map.

### REQ-DSP-04: FPC fit
- Level: subsystem
- Board: rp2350b
- Subsystem: display case
- Statement: The panel's FPC tail shall be 0.30 +-0.05 mm thick (the ER-TFT035IPS-6's drawing: 0.3 +-0.03) and 25.5 +-0.07 mm wide, and the folded FPC shall reach J3 with 0.5-3 mm of slack and no crease.
- Rationale: the FH12-50S's FPC specification; too short cannot mate, too long bows under the panel.
- Source: Hirose FH12 catalog p.2, p.11 (pin_review/j3.json); validation_plan #215, #160.
- Method: I
- Verified by: VER-44, VER-106, VER-219
- Phase: ORDER
- Status: PROPOSED

### REQ-DSP-05: Backlight current
- Level: subsystem
- Board: rp2350b
- Subsystem: display
- Statement: At the firmware's highest duty the six LEDs shall draw 120 mA or less in total at worst-case tolerance of the backlight driver's parts and the system rail's range on the cell and on USB.
- Rationale: the panel's 120 mA (6 x 20 mA); with R61 3.74k (BL_REF's divider, DEC-31) the worst case at 100% is 117.7 mA (VER-137). Was: 122.4 mA with R61 3.9k at be437f8.
- Source: ER-TFT035IPS-6 section 4.4 (pin_review/panel.json); design.py backlight section (U6, Q5, Q6, R60, R61: the RP board's, kept); Claude's analysis (F-7, O-21).
- Method: T, A
- Verified by: VER-62, VER-137, VER-05
- Phase: ORDER
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-DSP-06: Backlight transient
- Level: subsystem
- Board: rp2350b
- Subsystem: display
- Statement: The LED current shall exceed 120 mA in total for 5 us or less at any event, a USB plug-in while lit included (VSYS from the cell's 3.5-4.2 V up to 4.4 V).
- Rationale: the panel publishes no pulse rating; the committed simulation models R61 3.9k (0.124 V, 56 mA a pin), not the 3.74k the RP board drew on BL_PWM (the RP2350B's PWM slice: 0.119 V, 54.1 mA a pin, 117.2 mA worst case for the six LEDs), and reads nothing from design.py (DEF-45, VER-05); it is re-run with the board's R61. Derivation (VER-189 fix, 2026-10-29): the 120 mA is the panel's rated maximum forward current (the datasheet quote above), so the LED current may not exceed it. The panel publishes no pulse rating, so the 5 us allowed above it is a design choice (Claude's), and is on the to-sign list as that number alone. Earlier cite (not a primary source): board/sim/backlight_results.txt (to re-run).
- Source: hardware/ref_design/datasheets/er-tft035ips-6.pdf p.11, section 4.4 Backlight Characteristics (er-tft035ips-6.txt:713-715): forward current, normal, 6-chip parallel, max 120 mA (typ 110 mA).
- Method: A, T
- Verified by: VER-05, VER-62
- Phase: EVT
- Status: PROPOSED

### REQ-DSP-07: Backlight headroom
- Level: subsystem
- Board: rp2350b
- Subsystem: display
- Statement: At the firmware's maximum duty and VSYS 3.5 V the LED current shall drop by 5% or less from its value at 4.2 V for LED Vf up to 3.2 V, and by 10% or less for Vf up to 3.4 V (the newer datasheet's maximum).
- Rationale: brightness should not sag until the cutoff; at 18.0 mA an LED (54.1 mA a pin over three, nominal, R61 3.74k) its Vf sits near its 3.2 V typical (rated at 20 mA), and the sink needs about 0.13 V.
- Source: pin_review/panel.json; design_review.md Backlight.
- Method: A, T
- Verified by: VER-05, VER-137, VER-62
- Phase: ORDER
- Status: PROPOSED

### REQ-DSP-08: Backlight loop stability
- Level: subsystem
- Board: esp32s3
- Statement: Each sink's loop shall have 45 degrees of phase margin or more at every simulated corner, and an 18% to 95% duty step shall show no ringing on the scope.
- Rationale: an op-amp driving a FET gate can oscillate.
- Source: board/sim/backlight.py (72.5 degrees or more over 720 corners, run for R61 3.9k and 56 mA a pin; to re-run for 71737ae's R61 3.74k and 54.1 mA a pin once it reads design.py: DEF-45, VER-05).
- Method: A, T
- Verified by: VER-05, VER-62
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the loop stability of the ESP32-S3 board's two discrete backlight sinks; the RP2350B board's backlight driver is not chosen (REQ-DSP-05 to -07 and -09 hold the panel's limits)

### REQ-DSP-09: Backlight off when 3V3 is off
- Level: subsystem
- Board: rp2350b
- Subsystem: display
- Statement: With 3V3 off, a VSYS hot plug shall produce less than 5 mA of LED current for less than 10 us.
- Rationale: no flash at plug-in while the device is off. Derivation (VER-189 fix, 2026-10-29): a design choice: 5 mA is 4% of the panel's 120 mA maximum backlight current (5 / 120; er-tft035ips-6.txt:714-715), so no visible flash is expected for under 10 us. Earlier cite (not a primary source): board/sim/backlight_results.txt (4.7 mA peak, an older circuit); since 2026-10-28 VER-05 prints only an estimate (the gate's kick through Crss turns on the AO3400A's capacitance curve, a figure, and on how far the unlit LEDs let the drain move), so VER-62 measures it.
- Source: Alex's decision (to sign)
- Method: T
- Verified by: VER-62
- Phase: EVT
- Status: PROPOSED

### REQ-DSP-10: Tearing-effect signal
- Level: subsystem
- Board: rp2350b
- Subsystem: display soc
- Statement: With TEON set, the panel's TE pin shall reach an RP2350B input and show one TE pulse per frame at 60 Hz +-10%.
- Rationale: firmware starts frames on it to avoid tearing. Derivation (VER-189 fix, 2026-10-29): a design choice: 60 Hz is the frame rate the battler's display loop targets (Claude's reading; Alex to confirm), and 10% is 54 to 66 Hz, wide enough to read a TE pulse on a scope and narrow enough to tell a missed one. Earlier cite (not a primary source): bringup.md display step 6.
- Source: Alex's decision (to sign)
- Method: T
- Verified by: VER-63
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-DSP-11: Redraw time
- Level: subsystem
- Board: rp2350b
- Subsystem: display soc
- Statement: A full-screen 320 x 480 RGB565 redraw over the 8-bit 8080 bus shall take 16.7 ms or less (60 fps).
- Rationale: the 8080 bus at 25 MHz moves an RGB565 frame in 12.3 ms of bus time (18.4 ms in RGB666, VER-199); the RP2350B streams the frame from its 520 KB of SRAM over PIO, with no frame store beyond it (VER-224). Was: 220 ms over SPI.
- Source: ili9488.txt:16159; docs/living/display_parallel_proposal.md; Claude's estimate (Alex sets the numbers: DEC-13).
- Method: A, T
- Verified by: VER-224, VER-61, VER-199
- Phase: ORDER
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final. PLAN.md decision DEC-13 was taken in the design: the 8080 8-bit bus (docs/living/display_parallel_proposal.md, 39cc07f); Alex sets the two numbers.

### REQ-DSP-12: Backlight halves match at low duty
- Level: subsystem
- Board: esp32s3
- Statement: At the lowest duty the firmware uses (18%, pinmap.md), each sink's current shall be within +-25% of its nominal; the minimum duty shall be written in pinmap.md.
- Rationale: U6's 4.5 mV offset is +-20.2% of BL_REF's 22.3 mV there (VER-137); the two halves light one diffuser, so a 25% band per half is not visible as two halves.
- Source: Claude's analysis (O-21); pinmap.md Backlight.
- Method: A, T
- Verified by: VER-137, VER-62
- Phase: ORDER
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the match of the ESP32-S3 board's two backlight sinks at low duty; as REQ-DSP-08

## Subsystem: audio (AUD)

### REQ-AUD-01: Loudness
- Level: subsystem
- Board: rp2350b
- Subsystem: audio
- Statement: A full-scale 1 kHz sine shall deliver 0.35 W or more into the 8 ohm speaker (4.75 V peak-to-peak across it or more) on a 3.8 V cell.
- Rationale: designed 0.4-0.5 W on the cell, about 0.5 W on USB (VSYS 4.4 V); it clips below a 3.7 V cell.
- Source: design_review.md Audio; pin_review/u7.json.
- Method: T
- Verified by: VER-64
- Phase: EVT
- Status: PROPOSED

### REQ-AUD-02: Carrier rejection
- Level: subsystem
- Board: esp32s3
- Statement: The 312.5 kHz PWM carrier at the speaker shall be 40 dB or more below the full-scale 1 kHz level.
- Rationale: the LEDC runs from the 80 MHz APB clock: 8 bits is 312.5 kHz (pinmap.md Audio); R32 and R33 1.5k (39cc07f) give 44.5 dB nominal and 42.6 dB at the worst corner (VER-100). Was: 586 kHz from the RP2350.
- Source: design_review.md Audio; ad291a8.
- Method: T, A
- Verified by: VER-64, VER-100
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board's 312.5 kHz PWM audio carrier; the RP2350B board's audio source is its own codec (to confirm at NM-04)

### REQ-AUD-03: Bandwidth
- Level: subsystem
- Board: rp2350b
- Subsystem: audio
- Statement: The audio chain shall be -3 dB at 10 kHz or higher and at 50 Hz or lower.
- Rationale: a 15 mm speaker reproduces little above 10 kHz, so the chain need not pass more; the chosen source's filter (PWM or I2S, still open) is held to this at its component corners in VER-100. The RP board's two-stage RC (1k and 4.7 nF, then 220 nF into 20k) was -3 dB at about 14 kHz loaded: the starting point if the source is PWM. Derivation (VER-189 fix, 2026-10-29): a design choice: 10 kHz is under half of the 32 kHz mixer rate (REQ-PRD-18; 32 / 2 = 16 kHz Nyquist) and near what a 15 mm speaker can reproduce; 50 Hz is the low end. Earlier cite (not a primary source): design_review.md Audio.
- Source: Alex's decision (to sign)
- Method: A, T
- Verified by: VER-100, VER-64
- Phase: EVT
- Status: PROPOSED

### REQ-AUD-04: Amplifier heat
- Level: subsystem
- Board: rp2350b
- Subsystem: audio
- Statement: After 10 min of a continuous full-scale 1 kHz tone on USB at 25 C, the amplifier's top shall be 85 C or less.
- Rationale: about 0.54 W in an SOIC-8; the case skin limit is REQ-THM-01. Derivation (VER-189 fix, 2026-10-29): the amplifier's top at 85 C or less is the datasheet's maximum operating temperature; the 10 min of full-scale 1 kHz at 25 C is the test that heats it (0.54 W in an SOIC-8, REQ-THM-01 for the skin limit). Earlier cite (not a primary source): design_review.md Audio; bringup.md step 4.
- Source: hardware/ref_design/datasheets/fm8002a.pdf p.3, absolute maximum table (fm8002a.txt:110): operating temperature TOPR -40 to 85 C.
- Method: T
- Verified by: VER-64, VER-200
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-AUD-05: Amplifier off and wake
- Level: subsystem
- Board: rp2350b
- Subsystem: audio soc
- Statement: With the amplifier's enable off the amplifier shall draw 10 uA or less and be silent; with it on it shall reach full level within 50 ms with no pop over 10 mV at the speaker.
- Rationale: 4.2 uA shutdown typical; about 100 ms to charge the bypass capacitor.
- Source: FM8002A datasheet p.3 (pin_review/u7.json); design.py U7 and Q3 with R36 and R37 (shutdown held until the MCU enables it; the RP board's, kept on the RP2350B board); pinmap.md Audio.
- Method: T
- Verified by: VER-64, VER-196
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-AUD-06: Speaker rating
- Level: subsystem
- Board: rp2350b
- Subsystem: audio
- Statement: The firmware shall cap full scale to the speaker's rating: 0.5 W into 8 ohm for the GSPK151035PN, or the rating of the speaker the release names.
- Rationale: U7 gives about 0.5 W into 8 ohm on USB at full scale and more once it clips toward a square wave (DF-213). Derivation (VER-189 fix, 2026-10-29): the speaker's 0.5 W into 8 ohm is the INGHAi GSPK151035PN's rating as bom.md records it; its datasheet is not in hardware/ref_design/datasheets, so it is not cited as one. Capping at the rating keeps U7's 0.5 W on USB (more when clipped) inside it. Earlier cite (not a primary source): bom.md (INGHAi GSPK151035PN, C530539, 0.5 W; the RP board's, the speaker stays); Claude's review (O-31).
- Source: Alex's decision (to sign)
- Method: I
- Verified by: VER-128
- Phase: EVT
- Status: PROPOSED

### REQ-AUD-07: The audio source plays the mixer at 32 kHz, carrier 40 dB down
- Level: subsystem
- Board: rp2350b
- Subsystem: audio soc
- Statement: The audio source, a PWM or I2S stream the RP2350B generates because the chip has no DAC, shall play the mixer's output at a sample rate of 32 kHz or more, and any carrier or image it leaves at the speaker shall be 40 dB or more below the full-scale 1 kHz level at the worst component corner.
- Rationale: REQ-PRD-18 sets the mixer at 8 channels and 32 kHz, and the amplifier (U7) takes an analogue input, so the stream must be turned into one. VER-64 and VER-100 still test a 40 dB carrier with no live requirement behind it, the row that held it being dropped with the ESP32-S3 board (found by VER-189; Was: REQ-AUD-02). The source (PWM or I2S) is chosen at NM-04 (VER-100).
- Source: Alex's decision (to sign): the 40 dB carrier limit is Claude's engineering number, kept by Alex's decision of 2026-10-29; the 32 kHz is REQ-PRD-18 (Alex, 2026-10-29: 8 channels at 32 kHz).
- Method: A, T
- Verified by: VER-100, VER-64
- Phase: EVT
- Status: PROPOSED
- Notes: New on 2026-10-29, found by VER-189 (a function with no live requirement).

## Subsystem: buttons (BTN)

### REQ-BTN-01: Contact closes
- Level: subsystem
- Board: both
- Subsystem: buttons
- Statement: A DS Lite pill pressed on a contact shall measure under 1 kohm on ENIG, and each input shall still read pressed up to 15 kohm of pill resistance, against whatever pull-up and input threshold the RP2350B board reads it through.
- Rationale: carbon pill on gold; the contacts are read directly on RP2350B GPIOs through the 47k pull-up arrays (RN1-RN4, DEC-24). A pill of 15 kohm against a 47k pull-up at -5% (44.65k) puts 3.3 V x 15 / (15 + 44.65) = 0.83 V on the pad, so the GPIO's VIL must be 0.83 V or more; the RP2350's VIL is read from its datasheet (VER-233, VER-232), and 0.94 V (a 74HC165's at 3.3 V) would read a pill pressed up to about 17 kohm. If VIL is lower, the pull-up falls or the 15 kohm is restated by Alex (DF-230). Was: the RP2350's 50k internal pull-up allowed 10 kohm; 3.7 kohm with 10k at be437f8.
- Source: validation_plan gate item 5, #42, #137; the RP2350 datasheet's pad input levels (to download: VER-233).
- Method: T
- Verified by: VER-45, VER-56, VER-248
- Phase: EVT
- Status: PROPOSED
- Notes: measured on a bare board from the same JLC order (F-49); a coupon ordered now is advisory.

### REQ-BTN-02: Contact stays open
- Level: subsystem
- Board: both
- Subsystem: buttons
- Statement: Leakage between a contact's two combs shall be 100 Mohm or more at 4.2 V, clean and after 24 h at high humidity.
- Rationale: seat 1's Start's contact carries a rail on one pad (the diode into PWR_EN), so leakage across it turns the device on (REQ-PWR-11); on the others it reads as a press. Derivation (VER-189 fix, 2026-10-29): 100 Mohm at 4.2 V is 4.2 V / 100 Mohm = 42 nA at most across a contact; the 47k pull-up arrays (REQ-PRD-01.B) are about 2100 times lower in resistance (100M / 47k), so a leak this size reads as open. A design choice. Earlier cite (not a primary source): validation_plan gate item 5, #185.
- Source: Alex's decision (to sign)
- Method: T
- Verified by: VER-46, VER-248
- Phase: EVT
- Status: PROPOSED

### REQ-BTN-03: Comb geometry
- Level: subsystem
- Board: both
- Subsystem: buttons
- Statement: The combs shall keep a copper gap of 0.20 mm or more; the worst bridging spot shall be 1.60 mm or less for the 4-ring under the 3.5 mm D-pad and A/B/X/Y pill; one 4.8 mm mask opening shall uncover all comb copper with no paste or silk.
- Rationale: the DS Lite's measured comb; 0.20 mm sits above JLC's minimum spacing. Start and Select are metal domes since 2d1124d (REQ-BTN-08). Derivation (VER-189 fix, 2026-10-29): a design choice: 0.20 mm is above the minimum spacing JLC lists for 2 layers (REQ-FAB-03; its page is not on disk); the 1.60 mm bridging spot against the 3.5 mm pill leaves (3.5 - 1.6) / 2 = 0.95 mm of registration (REQ-BTN-06). Earlier cite (not a primary source): hardware/ref_design/board/membrane/README.md; check_contact_fp.py; case/DSLITE.md.
- Source: Alex's decision (to sign)
- Method: A
- Verified by: VER-08, VER-132, VER-162
- Phase: ORDER
- Status: PROPOSED

### REQ-BTN-04: Keep-out under the pill
- Level: subsystem
- Board: both
- Subsystem: buttons
- Statement: No via, pour or other copper shall lie within 2.9 mm of a contact's centre except its own two 0.2 mm stubs.
- Rationale: a tented via under a pill lifts it. Derivation (VER-189 fix, 2026-10-29): a design choice: 2.9 mm is the 4.8 mm mask opening's radius of 2.4 mm plus 0.5 mm; no copper inside it lifts the pill. Earlier cite (not a primary source): design_review.md Membrane contacts.
- Source: Alex's decision (to sign)
- Method: A
- Verified by: VER-132
- Phase: ORDER
- Status: PROPOSED

### REQ-BTN-05: Feel
- Level: subsystem
- Board: both
- Subsystem: buttons case
- Statement: Each button's actuation force shall be within +-30% of a genuine DS Lite's, and its travel within +-0.2 mm, measured the same way.
- Rationale: the point of DS Lite parts is the DS Lite feel; a relative limit avoids inventing an absolute number. Derivation (VER-189 fix, 2026-10-29): a design choice: the DS Lite feel is the aim (DESIGN.md:57), so the limits are relative to a genuine one, +-30% force and +-0.2 mm travel, with no absolute number invented. Earlier cite (not a primary source): validation_plan #43; case/DSLITE.md.
- Source: Alex's decision (to sign)
- Method: T
- Verified by: VER-50
- Phase: CASE
- Status: PROPOSED

### REQ-BTN-06: Pill registration
- Level: subsystem
- Board: both
- Subsystem: buttons case
- Statement: Each pill's centre shall land within 0.9 mm of its comb's centre at worst-case tolerance.
- Rationale: (3.5 mm pill - 1.6 mm bridging spot) / 2 is about 0.95 mm. Derivation (VER-189 fix, 2026-10-29): (3.5 mm pill - 1.6 mm bridging spot) / 2 = 0.95 mm is the most the centre may miss by before the pill no longer bridges; 0.9 mm leaves 0.05 mm of margin. Earlier cite (not a primary source): validation_plan #197.
- Source: Alex's decision (to sign)
- Method: A, I
- Verified by: VER-174, VER-51
- Phase: ORDER
- Status: PROPOSED

### REQ-BTN-07: Button life
- Level: subsystem
- Board: both
- Subsystem: buttons
- Statement: Each contact shall keep its closed resistance under 1 kohm for 100,000 presses.
- Rationale: Claude's estimate for a toy used daily for a few years. Derivation (VER-189 fix, 2026-10-29): 100,000 presses over 3 years is 100,000 / (3 x 365) = about 91 presses a day, a toy used daily; the 1 kohm closed resistance is well under the 47k pull-up (about 2% of it, 1k / 47k), so the line still reads low. A design choice. Earlier cite (not a primary source): Claude's estimate; validation_plan #159.
- Source: Alex's decision (to sign)
- Method: T
- Verified by: VER-94
- Phase: DVT
- Status: PROPOSED

### REQ-BTN-08: Start/Select domes
- Level: subsystem
- Board: rp2350b
- Subsystem: buttons case
- Statement: Each 4 mm metal dome (HYP 600-415S-260) shall sit with its rim on its footprint's ring all round at 0.1 mm of placement error, close under its cap's full travel, and rest open with its plunger 0.05 mm or more clear of its apex at the measured Start/Select strip.
- Rationale: a dome pressed at rest under seat 1's Start, the power key (REQ-PWR-21), turns the device on in the case for good (DF-242); the strip's plunger height is an estimate (ss_lift). Derivation (VER-189 fix, 2026-10-29): a design choice: 0.1 mm of placement error and 0.05 mm of plunger clearance keep a dome from being pressed at rest, which would hold seat 1's Start, the power key, on. The dome part is HYP 600-415S-260; its drawing (dome_600-415s-260.pdf) has no text to cite. Earlier cite (not a primary source): design.py SW20-SW23 (Dome_4mm_Round); case.scad ss_metal_dome, ss_lift; hardware/ref_design/PLAN.md row 4.
- Source: Alex's decision (to sign)
- Method: A, I
- Verified by: VER-178, VER-180, VER-48
- Phase: CASE
- Status: PROPOSED

### REQ-BTN-09: The 24-bit chain
- Level: subsystem
- Board: esp32s3
- Statement: The three 74HC165s (U8, U9, U11) shall read all 24 bits in pinmap.md's order with every input pulled up or tied, at an SR_CLK of 1 MHz or less with 0 errors in a million reads.
- Rationale: U11 joined the chain at 39cc07f with the Starts, the Selects, the power key's read and the charger's status (pinmap.md Buttons); SR_CLK runs 185.8 mm to three loads (VER-199).
- Source: design.py U8, U9, U11; sn74hc165.txt:371-504; pinmap.md Buttons.
- Method: A, T
- Verified by: VER-198, VER-199, VER-56, VER-179
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board's 24-bit chain of three 74HC165s; REQ-BTN-10 states the scan for the RP2350B board

### REQ-BTN-10: Every input read every 2 ms
- Level: subsystem
- Board: rp2350b
- Subsystem: buttons soc
- Statement: Every contact shall be read at least every 2 ms, so a press of 10 ms or more is never missed.
- Rationale: the contacts are read directly on GPIOs (Alex, 2026-10-29), so the read is a poll of 20 pad levels and its rate is the firmware's; behind the 74HC165 chain (the pin-relief option) it would be a shift-register scan. Claude's proposal.
- Source: Claude's estimate; Alex to set it.
- Method: T, A
- Verified by: VER-56, VER-199
- Phase: EVT
- Status: PROPOSED

## Subsystem: mechanics (MEC)

### REQ-MEC-01: Heights inside the case
- Level: subsystem
- Board: rp2350b
- Subsystem: case
- Statement: Every part on the board shall clear its local case limit by 0.2 mm or more, using heights from each part's drawing or a measurement, with 0 parts on guessed heights.
- Rationale: check_fit guesses package heights today (DF-229). Derivation (VER-189 fix, 2026-10-29): a design choice: 0.2 mm of margin on a drawn or measured height, and 0 parts on guessed heights. Earlier cite (not a primary source): validation_plan #96, #130, gate item 7.
- Source: Alex's decision (to sign)
- Method: A
- Verified by: VER-06, VER-112
- Phase: ORDER
- Status: PROPOSED

### REQ-MEC-02: Stack closes
- Level: subsystem
- Board: rp2350b
- Subsystem: case
- Statement: The Z stack (the tub's floor and pockets, cell, board, parts, speaker, panel, tape, the one-piece top) shall close with 0.2 mm or more of margin at worst-case tolerance including 0.4 mm of cell swelling.
- Rationale: nominal fits fail at tolerance. Derivation (VER-189 fix, 2026-10-29): a design choice: 0.2 mm margin after the 0.4 mm of cell swelling that REQ-BAT-07 holds; nominal fits fail at tolerance. Earlier cite (not a primary source): validation_plan #25, #132, gate item 7.
- Source: Alex's decision (to sign)
- Method: A
- Verified by: VER-20
- Phase: CASE
- Status: PROPOSED

### REQ-MEC-03: Buttons work under the shell
- Level: subsystem
- Board: rp2350b
- Subsystem: case buttons
- Statement: Every cap shall sit and press its full dome travel (the D-pad rocked in 8 directions) with 0 contact with the shell, and at rest touch only its dome.
- Rationale: a cap that drags or is preloaded feels wrong or sticks; seat 1's Start cap (the power key) held down keeps the device on. Derivation (VER-189 fix, 2026-10-29): a design choice: a cap that touches the shell drags or sticks, and seat 1's Start held by a dragging cap keeps the device on (REQ-PWR-21). Earlier cite (not a primary source): case/check_buttons.py.
- Source: Alex's decision (to sign)
- Method: A, D
- Verified by: VER-07, VER-48
- Phase: CASE
- Status: PROPOSED

### REQ-MEC-04: Assembly keyed
- Level: subsystem
- Board: rp2350b
- Subsystem: case
- Statement: The board shall fit the tub in 1 orientation only, and each A/B/X/Y cap in 1 hole and 1 rotation only.
- Rationale: no wrong assembly by hand. Derivation (VER-189 fix, 2026-10-29): a design choice: one orientation and one rotation, so hand assembly (REQ-ASM-01) cannot go wrong. Earlier cite (not a primary source): assembly.md sections 3 and 5.
- Source: Alex's decision (to sign)
- Method: I
- Verified by: VER-48
- Phase: CASE
- Status: PROPOSED

### REQ-MEC-05: Opening and closing
- Level: subsystem
- Board: rp2350b
- Subsystem: case
- Statement: The case shall survive 20 open/close cycles with its six M2 screws, with 0 cracked keys or grip ribs, 0 stripped bosses and 0 torn cell leads.
- Rationale: the first PLA print's hooks snapped (DEF-15) and the one-piece top's joints are thin printed features (DF-232); service needs opening, and the cell comes out with the tub on its leads (DF-239). Derivation (VER-189 fix, 2026-10-29): a design choice: 20 cycles covers a few services and bring-up opens per unit; the panel's connector is limited separately (REQ-TST-07). Earlier cite (not a primary source): validation_plan #194; assembly.md steps 9-10.
- Source: Alex's decision (to sign)
- Method: T
- Verified by: VER-92
- Phase: DVT
- Status: PROPOSED

### REQ-MEC-06: Parts stay put
- Level: subsystem
- Board: rp2350b
- Subsystem: case cell
- Statement: After the drop test the cell shall have moved 1 mm or less, the cell plug and the panel FPC shall still be seated, and the panel shall be uncracked.
- Rationale: a loose cell or plug in a dropped handheld is a hazard and a fault. Derivation (VER-189 fix, 2026-10-29): a design choice: 1 mm of cell movement, with the plugs and the panel still seated and uncracked after the drop. Earlier cite (not a primary source): validation_plan #183.
- Source: Alex's decision (to sign)
- Method: T
- Verified by: VER-93
- Phase: DVT
- Status: PROPOSED

### REQ-MEC-07: USB-C plugs seat
- Level: subsystem
- Board: both
- Subsystem: case usbc
- Statement: 5 different USB-C cables shall seat fully with the case closed.
- Rationale: overmolds vary; the opening was widened once already (c35514f). Derivation (VER-189 fix, 2026-10-29): a design choice: 5 different cables is a sample of the overmolds sold; the opening was already widened once (c35514f). Earlier cite (not a primary source): validation_plan #51.
- Source: Alex's decision (to sign)
- Method: T
- Verified by: VER-80
- Phase: EVT
- Status: PROPOSED

### REQ-MEC-08: No unintended openings
- Level: subsystem
- Board: rp2350b
- Subsystem: case
- Statement: The printed top and tub shall have 0 through-openings other than the listed ones (USB-C, cap holes, speaker grille, screw holes).
- Rationale: screw pilots and a notch once came through the face (DEF-19). Derivation (VER-189 fix, 2026-10-29): a design choice: any through-opening other than the listed ones lets in dust and light; DEF-19 was one such. Earlier cite (not a primary source): 177123c.
- Source: Alex's decision (to sign)
- Method: A, I
- Verified by: VER-113, VER-48
- Phase: CASE
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-MEC-09: Screws engage
- Level: subsystem
- Board: rp2350b
- Subsystem: case
- Statement: Each of the four side screws (H1-H4, under the screen band) shall engage 4.0 mm or more of its pilot above the board and end 0.5 mm or more short of the pilot's end; each of the two end screws (H5, H6, under the button ends) shall engage 3.0 mm or more and end 0.1 mm or more short.
- Rationale: self-tapping screws in printed plastic want 2 to 2.5 x d where the pilot allows it; the end pilots stop 0.6 mm under the ends' face and hold 3.30 mm at most, so one number for all six could never pass (F's objection). M2 x 6 gives 3.2 mm in both today (VER-147); M2 x 7 in the side pilots gives 4.2 mm with the tip 0.8 mm short (DEC-18, DEF-31). Derivation (VER-189 fix, 2026-10-29): self-tapping screws want 2 to 2.5 x d in printed plastic: M2 gives 4.0 to 5.0 mm (the side screws take 4.0 mm); the end pilots hold at most 3.30 mm, so 3.0 mm = 1.5 x d with 0.1 mm to the end. A design choice. Earlier cite (not a primary source): case.scad; Claude's analysis (O-52).
- Source: Alex's decision (to sign)
- Method: A, T
- Verified by: VER-147, VER-92
- Phase: CASE
- Status: PROPOSED

### REQ-MEC-10: Panel stays bonded
- Level: subsystem
- Board: rp2350b
- Subsystem: case display
- Statement: The panel shall stay bonded in the top's pocket, with no mura and no new contact marks, after 100 presses of 20 N at the glass centre and 24 h at 50 C, on a tape whose part number the release record names.
- Rationale: two players point at and press the shared screen; the panel lies on 0.15 mm tape 0.8 mm over the cell and 0.4 mm over J3 (DF-229). Derivation (VER-189 fix, 2026-10-29): a design choice: two players press the shared screen, so 100 presses of 20 N and 24 h at 50 C with no mura. Earlier cite (not a primary source): Claude's review (O-59).
- Source: Alex's decision (to sign)
- Method: T
- Verified by: VER-154
- Phase: EVT
- Status: PROPOSED

### REQ-MEC-11: Connector access
- Level: subsystem
- Board: rp2350b
- Subsystem: case
- Statement: Every connector's mouth (USB-C with a mated plug, the cell's plug and wires, the panel's FPC lead-in, the speaker's plug and its leads) shall be clear of parts taller than 1 mm and of every case wall but its own opening.
- Rationale: the USB-C opening was fixed twice (VER-165, DF-233). Was: J5 with a JST SH cable, and the pinholes. Derivation (VER-189 fix, 2026-10-29): a design choice: 1 mm of height clear at each connector mouth; the USB-C opening was fixed twice (VER-165). Earlier cite (not a primary source): Claude's review (F-28).
- Source: Alex's decision (to sign)
- Method: A
- Verified by: VER-165
- Phase: CASE
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final. the phase moved to CASE with VER-165 (F-R9: the connector mouths are a case matter; a case defect does not hold the order back).

### REQ-MEC-12: MLCCs clear of the screw bosses
- Level: subsystem
- Board: rp2350b
- Subsystem: case
- Statement: Every MLCC's pad centre shall be 4.0 mm or more from a screw hole's centre (the tub's 6.0 mm boss plus 1 mm), and 6.0 mm or more for an 0603 or larger.
- Rationale: the six screws clamp the board between bosses; a cracked MLCC on VSYS or VBAT shorts the cell inside the closed case (an MLCC on VSYS sat 0.9 mm from a boss at 76955d2; Was: C9). Derivation (VER-189 fix, 2026-10-29): the tub's boss is 6.0 mm across: 6.0 / 2 + 1.0 = 4.0 mm from the hole's centre to a pad; 0603 and larger parts are longer, so 6.0 mm. A design choice. Earlier cite (not a primary source): Claude's review (O-43); case.scad tub_boss_d.
- Source: Alex's decision (to sign)
- Method: A
- Verified by: VER-142
- Phase: ORDER
- Status: PROPOSED

## Subsystem: thermal (THM)

### REQ-THM-01: Touch temperature
- Level: subsystem
- Board: both
- Subsystem: case
- Statement: In normal operation, every outer surface a player holds shall stay at or below 43 C (target) at 25 C ambient, and the measured rise plus the rated maximum ambient shall stay at or below 48 C, while charging and playing with the backlight at 100% and audio at full scale.
- Rationale: IEC 62368-1's TS1 limit for plastic held 10 min to 8 h is 48 C (43 C beyond 8 h, to confirm); a handheld is held for hours. Single faults have their own, looser limit (REQ-PRD-07).
- Source: IEC 62368-1 cl. 9.3, Table 38 (to confirm); validation_plan #93.
- Method: T, A
- Verified by: VER-86, VER-144
- Phase: EVT
- Status: PROPOSED

### REQ-THM-02: Thermistor tracks the cell
- Level: subsystem
- Board: rp2350b
- Subsystem: cell
- Statement: During a full charge in the closed case, the thermistor's reading shall stay within 5 C of the cell's hotter point (its centre or its end facing the thermistor).
- Rationale: TH1 sits on the board 1.05 mm outside the cell hole's right edge (an 0805 since 71737ae), 1.95 mm or more from the cell's long side across the FR4 edge and an air gap, the pocket keeping the cell 0.9 mm inside the edge (DF-218); the window in REQ-BAT-03 is only as good as this. Derivation (VER-189 fix, 2026-10-29): a design choice: 5 C of difference between the thermistor and the cell's hotter point; the charger's own thresholds at TS are in bq24074.txt:824-827 (hot 300 mV, cold 2100 mV typ). Earlier cite (not a primary source): placement.py TH1, CUTOUTS; case.scad; validation_plan #30.
- Source: Alex's decision (to sign)
- Method: T, A
- Verified by: VER-87, VER-164
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-THM-03: No pause at room temperature
- Level: subsystem
- Board: rp2350b
- Subsystem: cell
- Statement: At 25 C ambient a full charge in the closed case shall complete with 0 temperature pauses (the charger's thermistor input inside its window with the cell under 35 C).
- Rationale: the board's own heat (U2, U3, the backlight's FETs) must not reach TH1. Derivation (VER-189 fix, 2026-10-29): the charger pauses outside its TS thresholds (bq24074.txt:824 VHOT 300 mV typ, :826 VCOLD 2100 mV typ). 35 C on a 25 C ambient is 10 C of rise, and 15 C under the 50 C top of the 0 to 50 C window the datasheet gives for a 103AT-2 thermistor (bq24074.txt:1968); TH1 is another thermistor, and VER-87 checks its own trips. A design choice. Earlier cite (not a primary source): bringup.md step 5.4.
- Source: Alex's decision (to sign)
- Method: T
- Verified by: VER-68
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-THM-04: Heat budget per mode
- Level: subsystem
- Board: rp2350b
- Subsystem: power soc case
- Statement: Charging while playing (5.5 V source, 3.0 V cell, backlight and audio at full scale) shall put 2.0 W or less inside the case, and every part shall stay under its temperature limit at 40 C ambient.
- Rationale: the worst mode is charging while playing: U2 (BQ24073), U3 and the RP2350B each dissipate, and VER-144's table is redone for them; the 2.0 W stays Claude's proposal until VER-144 gives the real sum, and the table decides a firmware rule or copper (DEF-87). Derivation (VER-189 fix, 2026-10-29): a design choice and Claude's proposal until VER-144 gives the real sum. Earlier cite (not a primary source): Claude's analysis (O-47, F-48).
- Source: Alex's decision (to sign)
- Method: A
- Verified by: VER-144, VER-238
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

## Subsystem: ESD and EMC (EMC)

### REQ-EMC-01: ESD
- Level: subsystem
- Board: both
- Subsystem: all
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
- Subsystem: all
- Statement: Before any sale, the device shall pass FCC Part 15 Class B and CISPR 32 Class B radiated and conducted emissions at a test house, its USB-C full-speed lines in use; the board has no radio.
- Rationale: the board has no radio, so it is an unintentional radiator and needs the Part 15 Subpart B declaration only, with no modular grant and no antenna type to keep; its USB is full speed (12 Mbit/s).
- Source: FCC 47 CFR Part 15 Subpart B; CISPR 32; validation_plan #67, #162.
- Method: T
- Verified by: VER-96, VER-118
- Phase: DVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

## Subsystem: radio (RF), since 39cc07f: DROPPED with the ESP32-S3 board

### REQ-RF-01: The antenna's keep-out on the board
- Level: subsystem
- Board: esp32s3
- Statement: The board shall keep a copper-free rule area on both layers under the flex antenna's plate (no track, via, pour or footprint): 0 copper items inside, and the area no more than 1.0 mm short of the plate's outline at any end on the board's plane.
- Rationale: copper under an antenna detunes it (esp32-s3-hw-design-guidelines.txt:1290-1299); at 39cc07f the rule area is x board edge to 71.4, y 62.6-107.0, 0 copper inside, 0.5 and 0.9 mm short of the plate's ends (VER-205).
- Source: esp32-s3-hw-design-guidelines.txt:1281-1301; placement.py ANTENNA_KEEPOUT; docs/living/esp32s3_build.md.
- Method: A
- Verified by: VER-205
- Phase: ORDER
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board's radio; the RP2350B board has no Wi-Fi (DESIGN.md:74, next_spin_requirements.md:30)

### REQ-RF-02: The antenna fitted and clear of metal
- Level: subsystem
- Board: esp32s3
- Statement: The flex antenna and its U.FL cable shall be bought with the boards, fixed on the case's left wall as assembly.md says, and kept the clearance its maker's datasheet gives from metal (the panel, the cell, the screws), 15 mm until that datasheet is on disk.
- Rationale: a WROOM-1U has no antenna of its own (esp32-s3-wroom-1.txt:100); at 39cc07f the panel's outline is 1.3 mm from the plate, the cell 10.6 mm, H2's boss over its end, and bom.md buys no antenna (VER-205, VER-167; DF-192, DF-197).
- Source: esp32-s3-wroom-1.txt:100; esp32-s3-hw-design-guidelines.txt:1299; the antenna's datasheet (not on disk).
- Method: A, I
- Verified by: VER-205, VER-167, VER-48
- Phase: CASE
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board's radio; the RP2350B board has no Wi-Fi (DESIGN.md:74, next_spin_requirements.md:30)

### REQ-RF-03: Wi-Fi range
- Level: subsystem
- Board: esp32s3
- Statement: In its closed case with a cell, held or face up, the device shall keep an RSSI of -70 dBm or better from an access point 10 m away line of sight, within 10 dB of the same antenna in free air, and hold a Showdown battle for 30 min with no dropped connection.
- Rationale: Wi-Fi is the reason for the module (docs/living/esp32s3_board_proposal.md 'Radio'); the numbers are Claude's proposal for Alex to set.
- Source: Claude's estimate; docs/living/esp32s3_board_proposal.md.
- Method: T
- Verified by: VER-207
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board's radio; the RP2350B board has no Wi-Fi (DESIGN.md:74, next_spin_requirements.md:30)

### REQ-RF-04: The radio's grant
- Level: subsystem
- Board: esp32s3
- Statement: Before any unit is sold or given away, the antenna fitted shall be of a type and gain the WROOM-1U's FCC grant lists, and the label shall carry the module's FCC ID.
- Rationale: an intentional radiator relies on the module's modular grant only with a listed antenna (docs/living/esp32s3_board_proposal.md 'Open checks' 5).
- Source: docs/living/esp32s3_board_proposal.md; the module's FCC grant (not on disk).
- Method: I
- Verified by: VER-217
- Phase: SALE
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board's radio; the RP2350B board has no Wi-Fi (DESIGN.md:74, next_spin_requirements.md:30)

## Subsystem: storage (STO), the microSD slot, since 39cc07f: DROPPED with the F1C200s board (the storage subsystem is now the 16 MB flash: REQ-SOC-01, REQ-SOC-09, REQ-PRD-18)

### REQ-STO-01: The microSD bus
- Level: subsystem
- Board: f1c200s
- Subsystem: storage soc
- Statement: The microSD slot shall read and write a card on the F1C200s's SD0 at 4 bits, and the boot ROM shall boot from it (REQ-SOC-01).
- Rationale: Was: on the ESP32-S3 board (tag vv-esp32s3-final), to redo for the RP2350B board: the slot is new at 39cc07f (docs/living/esp32s3_board_proposal.md item 5); Espressif asks pull-ups on the SDIO lines (esp32-s3-hw-design-guidelines.txt:864-865); the DM3AT's detect switch is normally open (hirose_dm3_catalog_C114218.txt:27).
- Source: Was: on the ESP32-S3 board (tag vv-esp32s3-final), to redo for the RP2350B board: design.py J8, R79-R84; esp32-s3-hw-design-guidelines.txt:861-865; hirose_dm3_catalog_C114218.txt:26-27.
- Method: A, T
- Verified by: VER-209, VER-198, VER-222
- Phase: EVT
- Status: DROPPED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.
- Dropped: DROPPED with f1c200s (tag vv-f1c200s-final): the microSD slot left the design on 2026-10-29 (DESIGN.md:74: no SD card); its statement is at the tag; the flash's requirements are REQ-SOC-01 and REQ-SOC-09

### REQ-STO-02: A card plugged in while running
- Level: subsystem
- Board: f1c200s
- Subsystem: storage power
- Statement: Inserting a microSD card while the device runs shall leave the card's rail and the F1C200s's IO rail inside their ranges and cause no reset.
- Rationale: Was: on the ESP32-S3 board (tag vv-esp32s3-final), to redo for the RP2350B board: J8's VDD comes straight off 3V3 through JP6 with no load switch: a card over 1.8 uF of its own dips 3V3 under 3.0 V at once (VER-206 on stand-ins, DF-245).
- Source: Was: on the ESP32-S3 board (tag vv-esp32s3-final), to redo for the RP2350B board: esp32-s3-wroom-1.txt:1596; tlv757p.txt:267; VER-206.
- Method: A, T
- Verified by: VER-206, VER-209
- Phase: EVT
- Status: DROPPED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.
- Dropped: DROPPED with f1c200s (tag vv-f1c200s-final): the microSD slot left the design on 2026-10-29 (DESIGN.md:74: no SD card); its statement is at the tag; the flash's requirements are REQ-SOC-01 and REQ-SOC-09

### REQ-STO-03: The SD opening in the case
- Level: subsystem
- Board: f1c200s
- Subsystem: storage case
- Statement: With the case closed a card shall go in and latch, and come out, 20 of 20 times, with the slot's body clear of the left wall's opening by 0.2 mm a side.
- Rationale: Was: on the ESP32-S3 board (tag vv-esp32s3-final), to redo for the RP2350B board: the opening is 14.0 mm over the slot's 13.85 mm body at 39cc07f (VER-165, DF-233).
- Source: case.scad sd_y, sd_z; hirose_dm3_catalog_C114218.pdf.
- Method: A, T
- Verified by: VER-165, VER-48
- Phase: CASE
- Status: DROPPED
- Dropped: DROPPED with f1c200s (tag vv-f1c200s-final): the microSD slot left the design on 2026-10-29 (DESIGN.md:74: no SD card); its statement is at the tag; the flash's requirements are REQ-SOC-01 and REQ-SOC-09

## Board: manufacture (FAB)

### REQ-FAB-01: Design round trip
- Level: board
- Board: rp2350b
- Subsystem: all
- Statement: KiCad's netlist shall match design.py with 0 differences, ERC shall report 0 errors, and DRC 0 errors, 0 unconnected and 0 parity issues.
- Rationale: the generated KiCad files must say what design.py means. Derivation (VER-189 fix, 2026-10-29): a design rule: 0 differences and 0 errors, because any ERC, DRC or parity issue is a drawn fault. Earlier cite (not a primary source): validation_plan #75, #76, #173.
- Source: Alex's decision (to sign)
- Method: A
- Verified by: VER-01, VER-03, VER-04
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-02: Pins against datasheets
- Level: board
- Board: rp2350b
- Subsystem: all
- Statement: Every assembled part's every pin shall be reviewed against its current datasheet with a page citation, with 0 unresolved issues, 0 unknown ratings and 0 open questions.
- Rationale: ERC and parity trust design.py; only the datasheet can say a pin is wrong. Derivation (VER-189 fix, 2026-10-29): a design rule: 0 unknown ratings, because only the datasheet can say a pin is wrong. Earlier cite (not a primary source): validation_plan #1, #113, gate item 1.
- Source: Alex's decision (to sign)
- Method: A, I
- Verified by: VER-02, VER-36, VER-35
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-03: JLC capability
- Level: board
- Board: rp2350b
- Subsystem: all
- Statement: The board shall pass DRC against JLC's published 2-layer limits, copied with the capability page's date into a data file the build reads (track, space, the minimum via pad for a 0.3 mm hole, annular ring, copper to the edge 0.30 mm or more), with 0 errors; where the board's numbers are below the page's, JLC confirms them in writing before the order.
- Rationale: KiCad's defaults do not encode JLC's limits; the old 0.13 mm ring, written from memory, failed every via on the board (F-12).
- Source: validation_plan #10, #118, #120; JLC capabilities page, dated on order day.
- Method: A, I
- Verified by: VER-17, VER-18, VER-28
- Phase: ORDER
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final. the copper to the edge is met in the data at 71737ae: 0.308 mm at the closest, KiCad's edge clearance 0.31 mm and a 0.30 mm copper-to-NPTH rule (battler.kicad_dru), VER-18 PASS; at be437f8 GND pours sat 0.20 mm from both holes. As built, with the 0.2 mm outline tolerance (ROUTE_TOL), the closest is 0.108 mm, and the narrowest FR4 web 1.00 mm can be 0.60 mm (O-R2): whether 0.30 mm is a data rule or an as-built one is the dated page's to say (VER-17, DEF-56). The dated page and the vias' 0.10 mm ring are still open.

### REQ-FAB-04: Holes
- Level: board
- Board: rp2350b
- Subsystem: all
- Statement: Every drilled hole shall be 0.30 mm or larger, and plated and unplated holes shall be in separate drill files.
- Rationale: smaller holes cost about $40 an order at JLC (a fee, a 4-wire test, TG155); a mixed file may plate J1's pegs (DEF-32).
- Source: design_review.md Board; assembly.md section 1; JLC's KiCad guide.
- Method: A
- Verified by: VER-114, VER-131
- Phase: ORDER
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-FAB-05: Order options
- Level: board
- Board: both
- Subsystem: all
- Statement: The order shall be 2 layers, 1.6 mm, ENIG, with the order number's location specified ('JLCJLCJLCJLC' on the back silk), edge rails off (or routed removal), and every option recorded line by line in the release record (1 record per order).
- Rationale: the pills press on bare combs, so the finish must be gold; a V-cut rail leaves a ridge the case slot is not sized for; a stray order number can land on a comb (DF-234). Derivation (VER-189 fix, 2026-10-29): a design choice: 2 layers (DESIGN.md:62), 1.6 mm, ENIG because the pills press on bare combs, and no V-cut rail. Earlier cite (not a primary source): assembly.md section 1; validation_plan #189.
- Source: Alex's decision (to sign)
- Method: I
- Verified by: VER-42, VER-28
- Phase: ORDER
- Status: PROPOSED
- Notes: at 71737ae silk.py places the board's name, BOARD_REV and the build line on clear spots on the back (VER-18: every text on the board), but the marker is still in neither battler.kicad_pcb nor silk.py (O-R19), so JLC would place its number anywhere (DF-234, DEF-55). PLAN.md decision DEC-28: the marker in silk.py (a design change and a new baseline), or JLC's paid 'remove order number' option with this statement restated. VER-132 (k) checks it.

### REQ-FAB-06: Part identity
- Level: board
- Board: both
- Subsystem: all
- Statement: Every LCSC number shall resolve to the MPN, package, pin count and rating design.py intends, with 0 substitutions on the order.
- Rationale: the right family in the wrong variant passes every other check. Derivation (VER-189 fix, 2026-10-29): a design rule: 0 substitutions, because the right family in the wrong variant passes every other check. Earlier cite (not a primary source): validation_plan #2, #122.
- Source: Alex's decision (to sign)
- Method: A, I
- Verified by: VER-14, VER-186, VER-130, VER-28, VER-41
- Phase: ORDER
- Status: PROPOSED
- Notes: VER-14 is the by-hand pass at ORDER (every line of JLC's matched-parts page against design.py, saved in VER-28's evidence); the script over the parts database is VER-186's, at PVT (F-R9).

### REQ-FAB-07: Orientation
- Level: board
- Board: both
- Subsystem: all
- Statement: Every polarised part's pin 1 in the fab house's placement preview shall match the board's pin 1, and every part shall sit on its pads, with 0 unexplained differences.
- Rationale: a CAD footprint's zero and origin can differ from the fab house's. Derivation (VER-189 fix, 2026-10-29): a design rule: pin 1 and the origin can differ between CAD and the fab house, so a preview check is needed. Earlier cite (not a primary source): validation_plan #9, #116.
- Source: Alex's decision (to sign)
- Method: I, A
- Verified by: allocated: REQ-FAB-07.B (Was: REQ-FAB-07.R)
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-07.R: Orientation and centroids (ref_design)
- Level: board
- Board: esp32s3
- Statement: Every polarised part's pin 1 in JLC's placement preview shall match the board's pin 1, U1, U2, U3, U4, U8, U9 and J3's pin-1 end included, and every CPL position shall be within 0.1 mm of the part's pad centre, or the reference's origin offset shall be recorded in a JLC_FIXED entry that names the placement preview screenshot and the offset JLC's library applies (J1, J2, J3 and J4: 1.50, 0.95, 0.95 and 0.23 mm, which may be zero), with 0 unexplained differences.
- Rationale: KiCad's footprint zero differs from JLC's for some parts (DEF-14), and J1's origin is 1.50 mm, J2's and J4's 0.95 mm and J3's 0.23 mm off their pads' centre (VER-176, DEF-33). Whether JLC's own library places them off their pads is unknown until the placement preview (O-R7): no CPL row moves before the dry run, and a per-reference correction table in fab.py is filled only from a JLC_FIXED entry that names the preview screenshot and the offset JLC's library applies, which may be zero. Such an entry counts only when its evidence file is committed and names the designator (F-R4).
- Source: validation_plan #9, #116; 1fd0956.
- Method: I, A
- Verified by: VER-28, VER-29, VER-176
- Phase: ORDER
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): it named the ESP32-S3 board's parts (U1-U9, J1-J4) and their JLC offsets; REQ-FAB-07.B holds the RP2350B board's

### REQ-FAB-07.B: Orientation and centroids (rp2350b)
- Level: board
- Board: rp2350b
- Subsystem: all
- Statement: Every polarised part's pin 1 in JLC's placement preview shall match the board's pin 1, the RP2350B's and the flash's included, and every CPL position shall be within 0.1 mm of the part's pad centre, or its origin offset recorded in a JLC_FIXED entry that names the placement preview screenshot.
- Rationale: a part placed 90 or 180 degrees off kills the board (carryover K-3, K-4).
- Source: JLC's placement preview; carryover.md K-3, K-4.
- Method: I, A
- Verified by: VER-28, VER-29, VER-176
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-08: Reproducible outputs
- Level: board
- Board: rp2350b
- Subsystem: all
- Statement: The gerbers, drill, BOM and CPL shall come from the released commit by one release run from a clean worktree; two runs of fab.py shall give identical copper and drill files once their creation dates are stripped; and the uploaded zip's sha256 shall be in the release record.
- Rationale: the board ordered must be the board checked; Alex's main checkout had an uncommitted KiCad GUI change to battler.kicad_pro on 2026-10-24. Derivation (VER-189 fix, 2026-10-29): a design rule: the board ordered must be the board checked; two runs identical once dates are stripped. Earlier cite (not a primary source): validation_plan #12, #88, #121, gate item 11.
- Source: Alex's decision (to sign)
- Method: A, I
- Verified by: VER-143, VER-42, VER-132
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-09: Independent connectivity
- Level: board
- Board: both
- Subsystem: all
- Statement: JLC shall e-test 100% of the boards against an IPC-D-356 netlist exported from the ordered board with each net-tie pair merged into one net (0 pairs of differently named nets joined by copper), or the order shall go without a netlist.
- Rationale: a check from outside KiCad's own reading; JP1-JP5 would read as 5 shorts (DF-237). Derivation (VER-189 fix, 2026-10-29): a design choice: an independent e-test from outside KiCad's own reading, or none; DVT, not ORDER. Earlier cite (not a primary source): validation_plan #103.
- Source: Alex's decision (to sign)
- Method: I
- Verified by: VER-26
- Phase: DVT
- Status: PROPOSED
- Notes: the phase moved to DVT with VER-26 (O-R9: the e-test is advisory at ORDER, with the default "send no netlist", and binding at G-DVT-01). JLC e-tests against its own Gerber netlist, and an IPC-356 file without merged net ties would only trigger false shorts (F-R9).

### REQ-FAB-10: Stock
- Level: board
- Board: both
- Subsystem: all
- Statement: On order day every BOM line shall have stock of 1.1 x the order quantity plus attrition, or be pre-ordered or consigned; the RP2350B (C42415655), the GD25Q128 (C2758105) and the TPS3421 (C2067262) shall never be substituted.
- Rationale: the new parts' JLC class and stock must be read on order day; a substituted flash can be smaller or on another interface (DF-238), and the TPS3421 comes in variants with different reset delays (DF-262), so a substitute breaks the hard reset of REQ-PRD-08.B. Derivation (VER-189 fix, 2026-10-29): a design choice: 1.1 x the order quantity is 10% over, plus attrition; the three named parts are never substituted because each has variants (DF-238, DF-262). Earlier cite (not a primary source): DEF-12, DEF-13, DEF-29.
- Source: Alex's decision (to sign)
- Method: I
- Verified by: VER-41
- Phase: ORDER
- Status: PROPOSED

### REQ-FAB-11: Flatness
- Level: board
- Board: both
- Subsystem: all
- Statement: Each board's bow and twist shall be 0.75% or less of its length (the Edge.Cuts outline).
- Rationale: the case clamps the board flat; a bowed board cracks MLCCs.
- Source: IPC-6012 Class 2 (bow and twist for SMT boards); validation_plan #78.
- Method: T
- Verified by: VER-90
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-FAB-12: Workmanship
- Level: board
- Board: both
- Subsystem: all
- Statement: Every assembled board shall meet IPC-A-610 Class 2 under the microscope, with 0 bridges, tombstones, lifted leads or missing parts.
- Rationale: Economic PCBA has no X-ray; the loupe is the inspection.
- Source: IPC-A-610 Class 2; validation_plan #53, #157.
- Method: I
- Verified by: VER-89
- Phase: EVT
- Status: PROPOSED

### REQ-FAB-13: No indicator lights
- Level: board
- Board: rp2350b
- Subsystem: all
- Statement: The board shall carry 0 indicator lights: 0 LED parts in the BOM and on the board, the panel's own backlight LEDs excepted.
- Rationale: Alex's spec; a light also costs a part, a pin and a few mA.
- Source: DESIGN.md:63 ('There are no indicator lights on the board.', Alex, 2026-09-24).
- Method: I
- Verified by: VER-265
- Phase: ORDER
- Status: PROPOSED
- Notes: New on 2026-10-29, found by VER-189 (a spec line with no requirement).

## Board: hand assembly (ASM)

### REQ-ASM-01: Hand assembly
- Level: board
- Board: both
- Subsystem: all
- Statement: Alex shall assemble 1 unit from its board's assembly instructions in 30 min or less, on an ESD mat with a strap, damaging 0 parts.
- Rationale: 30 units by hand. Derivation (VER-189 fix, 2026-10-29): 30 units by hand at 30 min each is 30 x 30 = 900 min = 15 h. A design choice. Earlier cite (not a primary source): validation_plan #86, #161.
- Source: Alex's decision (to sign)
- Method: D
- Verified by: allocated: REQ-ASM-01.B (Was: REQ-ASM-01.R)
- Phase: EVT
- Status: PROPOSED

### REQ-ASM-01.R: Hand assembly (ref_design)
- Level: board
- Board: esp32s3
- Statement: Alex shall assemble 1 ref_design unit from assembly.md in 30 min or less with a screwdriver and a knife and no iron (nothing is soldered at assembly), on an ESD mat with a strap, damaging 0 parts, in the order J3 latched, the cell into the tub's pocket, J2 then J4 plugged, and on opening with J2 unplugged before the tub lifts away.
- Rationale: 30 units by hand; the speaker plugs into J4 since 7da6a90, and the opening order keeps the cell's leads off the hole's edge (DF-169, DF-170).
- Source: docs/assembly.md; validation_plan #86, #161.
- Method: D
- Verified by: VER-91
- Phase: EVT
- Status: DROPPED
- Dropped: DROPPED with esp32s3 (tag vv-esp32s3-final): assembly.md's order for the ESP32-S3 board; REQ-ASM-01.B holds the RP2350B board's

### REQ-ASM-01.B: Hand assembly (rp2350b)
- Level: board
- Board: rp2350b
- Subsystem: all
- Statement: Alex shall assemble 1 RP2350B unit from its assembly doc in 30 min or less with a screwdriver and a knife and no iron (nothing is soldered at assembly), on an ESD mat with a strap, damaging 0 parts, the cell plugged last and unplugged first.
- Rationale: nothing is soldered at assembly (DESIGN.md:71); work next to a live cell is the hazard (carryover, DEF-41 at tag vv-esp32s3-final).
- Source: DESIGN.md:71 ('Nothing is soldered when a unit is put together: the battery and the speaker both plug in.').
- Method: D
- Verified by: VER-91
- Phase: EVT
- Status: PROPOSED

### REQ-ASM-02: Panel seated
- Level: board
- Board: rp2350b
- Subsystem: display
- Statement: The panel's FPC shall be fully inserted, square and latched, and pass the display test after 10 case closures.
- Rationale: a half-seated FPC gives intermittent lines. Derivation (VER-189 fix, 2026-10-29): a design choice: a half-seated FPC gives intermittent lines, so it is checked after 10 closures. Earlier cite (not a primary source): assembly.md step 4; validation_plan #160.
- Source: Alex's decision (to sign)
- Method: I, T
- Verified by: VER-106
- Phase: EVT
- Status: PROPOSED

## Board: test and traceability (TST)

### REQ-TST-01: Test access
- Level: board
- Board: rp2350b
- Subsystem: all
- Statement: Every measurement in the RP2350B board's bring-up doc shall have a probe point (a test pad, a jumper pad or a named part pad), with 0 measurements that need a lifted part.
- Rationale: bring-up must not need rework to measure. Derivation (VER-189 fix, 2026-10-29): a design rule: bring-up must not need rework to measure. Earlier cite (not a primary source): validation_plan #21, #155.
- Source: Alex's decision (to sign)
- Method: I
- Verified by: VER-158
- Phase: ORDER
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-TST-02: Self-test
- Level: board
- Board: both
- Subsystem: all
- Statement: A self-test image shall check the 20 contacts, pin neighbour bridges, the panel's ID and fills, the backlight, an audio tone, the flash's JEDEC ID and its 16 MB readback, the USB-C device enumeration, the cell reading and the charger's states, print PASS or FAIL per item and take 5 min or less a unit.
- Rationale: 30 units need a repeatable test; OTP writes can brick a unit. Derivation (VER-189 fix, 2026-10-29): 30 units x 5 min = 150 min = 2.5 h of test for the 30-board run; a design choice. Earlier cite (not a primary source): validation_plan #57, #100, #174, #202.
- Source: Alex's decision (to sign)
- Method: T
- Verified by: VER-76
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-TST-03: Traceability
- Level: board
- Board: both
- Subsystem: soc
- Statement: Every unit shall carry a serial number linked in 1 record to its chip ID, cell lot, test log and release.
- Rationale: a field fault must lead back to its lot and its tests. Derivation (VER-189 fix, 2026-10-29): a design rule: a field fault must lead back to its lot and tests; one record per serial. Earlier cite (not a primary source): validation_plan #195.
- Source: Alex's decision (to sign)
- Method: I
- Verified by: VER-110, VER-242
- Phase: PVT
- Status: PROPOSED

### REQ-TST-04: First article
- Level: board
- Board: both
- Subsystem: all
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
- Subsystem: all
- Statement: The PVT lot's first-pass yield on the self-test shall be 90% or more, with every failure logged as a defect.
- Rationale: below that the process, not the unit, is the problem. Derivation (VER-189 fix, 2026-10-29): 90% of the 30-board PVT lot is 27 units (0.9 x 30) passing first time; below that the process is the problem. A design choice. Earlier cite (not a primary source): Claude's estimate.
- Source: Alex's decision (to sign)
- Method: I
- Verified by: VER-111
- Phase: PVT
- Status: PROPOSED

### REQ-TST-06: Per-unit VBAT calibration
- Level: board
- Board: rp2350b
- Subsystem: cell
- Statement: Each unit's cell-reading gain shall be set in production test at a true 3.50 V from the bench supply (no USB, the self-test at its highest load, a DMM checked that session against a 4.096 V reference) and lie within 0.95-1.05; a re-read at 3.50 V shall be within 10 mV; the firmware shall store no gain from a no-cell end-of-charge edge.
- Rationale: the charger's CV is only +-1% (+-42 mV), and with no cell the BQ24073's battery detection toggles CHG (CHG_STAT_N), so a gain from an end-of-charge edge can be off by the cutoff's whole margin (DF-225). Derivation (VER-189 fix, 2026-10-29): the charger's CV is +-1% of 4.2 V = +-42 mV (REQ-BAT-01); a re-read within 10 mV is about a quarter of that (10 / 42), and a gain of 0.95 to 1.05 is +-5%, five times the CV's +-1%. A design choice. Earlier cite (not a primary source): Claude's review (O-63, O-60).
- Source: Alex's decision (to sign)
- Method: T
- Verified by: VER-155, VER-156
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-TST-07: Connector mating budget
- Level: board
- Board: rp2350b
- Subsystem: all
- Statement: No shipped unit shall have any connector (the cell's, the panel's, the speaker's) mated more than 50% of its rated cycles, counted per unit.
- Rationale: wire-to-board headers and the FH12 are rated for roughly 10-30 matings (confirm per maker), bring-up alone can use them up, and the speaker's plug comes out at every opening (DF-233). Derivation (VER-189 fix, 2026-10-29): the FH12 is rated for 20 insertion cycles (hirose_fh12.txt:58, 'Durability (Insertion/withdrawal)... 20 cycles'); 50% of that is 10 matings a unit, so the panel's connector keeps half its life after bring-up. The other plugs' ratings are not on disk (confirm per maker); 50% is a design choice. Earlier cite (not a primary source): Claude's review (O-61).
- Source: Alex's decision (to sign)
- Method: I
- Verified by: VER-157
- Phase: EVT
- Status: PROPOSED
- Notes: Restated on 2026-10-28 for the RP2350B board; the ESP32-S3 board's statement is at tag vv-esp32s3-final.

### REQ-TST-08: Per-unit findings
- Level: board
- Board: both
- Subsystem: all
- Statement: Every finding on a built unit shall be recorded with its serial, how it was found, its disposition (rework, scrap or use as is) and its re-test; a finding on 2 units shall become a DEF with a check.
- Rationale: defects.md is for design defects; rework without a re-test, or a recurring process fault never raised, ships bad units. Derivation (VER-189 fix, 2026-10-29): a design rule: rework without a re-test, or a recurring process fault never raised, ships bad units; 2 units is the smallest count that shows a pattern. Earlier cite (not a primary source): Claude's review (O-36).
- Source: Alex's decision (to sign)
- Method: I
- Verified by: VER-110
- Phase: EVT
- Status: PROPOSED

## Board: prod_board (PBD), RETIRED

Alex's board, its case and their checks: all RETIRED in 574c94c, which deleted
hardware/prod_board@611ebe3 and hardware/prod_case@611ebe3 from the repository (Alex: not needed;
611ebe3 is the last commit with them). The IDs stay. They covered 16 contacts placed from prod_case and an empty
schematic; prod_case used a 10 mm 103450 cell (about 2000 mAh, a 19.1 mm case).

### REQ-PBD-01: Contacts where the case says
- Level: board
- Board: prod_board
- Statement: Each of the 16 contact footprints shall sit within 0.05 mm of its position in hardware/prod_case/layout.scad@611ebe3.
- Rationale: the case is the source of truth for prod_board.
- Source: hardware/prod_case/README.md@611ebe3; hardware/prod_board/tools/place_buttons.py@611ebe3.
- Method: A
- Verified by: VER-119
- Phase: ORDER
- Status: RETIRED
- Retired: RETIRED in 574c94c: prod_board deleted from the repo, Alex: not needed

### REQ-PBD-02: Contact footprint
- Level: board
- Board: prod_board
- Statement: The DSL_button_4ring footprint in prod.pretty shall keep a 0.20 mm or larger copper gap and a 1.60 mm or smaller bridging spot.
- Rationale: the same comb as ref_design.
- Source: hardware/ref_design/board/membrane/README.md.
- Method: A
- Verified by: VER-10
- Phase: ORDER
- Status: RETIRED
- Retired: RETIRED in 574c94c: prod_board deleted from the repo, Alex: not needed

### REQ-PBD-03: Design checks
- Level: board
- Board: prod_board
- Statement: Before an order, prod_board's ERC shall report 0 errors and its DRC 0 errors, 0 unconnected and 0 parity issues, and the board shall fit prod_case with 0 collisions.
- Rationale: the same gate ref_design passes.
- Source: this register.
- Method: A
- Verified by: VER-120, VER-121, VER-122
- Phase: ORDER
- Status: RETIRED
- Retired: RETIRED in 574c94c: prod_board deleted from the repo, Alex: not needed
