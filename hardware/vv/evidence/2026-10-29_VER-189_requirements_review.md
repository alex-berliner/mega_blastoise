# VER-189 requirements_review
DATE: 2026-10-29
COMMIT: 7c7f8717f9ffb02cb9526ef0e1d2ccf0711525b0
WHO: Claude (session vv-189), reading every live row of requirements.md against docs/static/DESIGN.md and docs/static/next_spin_requirements.md, both read the same day (Alex's decisions of 2026-10-29 included)
ARTEFACT: the three files at this commit, with their git blob hashes:
  hardware/vv/requirements.md c6474058910a5c40f117636954ccdda7c8da5297
  docs/static/DESIGN.md 8f3b47275556dbd6542c9e97f30fa2e2d4f68f9e
  docs/static/next_spin_requirements.md ff52aa52c41e8f3a5657c30b56e21d67bd92f095
Live rows read: 135 (REQ-PRD 21, all APPROVED by Alex; lower-level 114); a row counts once per subsystem it lists.

## Test 1: every function has a live requirement (DESIGN.md / next_spin_requirements.md function -> rows -> checks)

### power
- take USB-C power: REQ-PWR-01, -16, -02, -12, -13, -17 (VER-80, 83, 102, 12, 71, 81, 99, 177, 97)
- charge the cell: REQ-BAT-01 to -04, -09, -13 (VER-67, 134, 14, 32, 69, 135, 85, 98, 133, 145)
- the RP2350B's and flash's rails: REQ-PWR-04, -05, -20 (VER-83, 235, 70, 231, 249)
- turn on by a 2.5 s hold of seat 1's Start, off by a 1 to 2 s hold, no slider: REQ-PWR-21, -09, -10, -11, REQ-PRD-15 (VER-239, 240, 243, 105, 104, 223, 228)
- force off by an 8 s hold, TPS3421 on 3V3: REQ-SOC-04, REQ-PRD-08.B, REQ-PWR-07 (VER-258, 241, 198, 65, 146). GAP: 'about 8 s' has no tolerance (DEF-106)
- Sleep saves and turns off: REQ-PRD-20, REQ-SOC-11 (VER-263, 264)
### cell
- hold, sense, protect: REQ-BAT-05 to -08, -10 to -12, -14 to -16, REQ-THM-02, -03 (VER-49, 54, 88, 37, 140, 182, 181, 138, 139, 153)
### soc
- boot bare metal from the flash within about a second: REQ-SOC-01, -02, -03, REQ-PRD-15 (VER-252, 237, 198, 223, 228)
- recover over the boot ROM, update mode with Select at power-on: REQ-SOC-10, REQ-PRD-08.B, REQ-PRD-19 (VER-259, 241, 260, 258)
- the pin plan, the chip's heat, debug and the battle log: REQ-SOC-05, -06, -07, -08 (VER-232, 262, 242, 238, 86, 257, 158)
### storage
- the 16 MB flash holds the program and the sound: REQ-SOC-01, -09, -10, -11, REQ-PRD-18 (VER-252, 253, 254, 125, 259, 264, 256, 255)
### usbc
- a USB device to an Android phone's Chrome (WebUSB): REQ-USB-01, -02, REQ-PRD-14 (VER-251, 195, 80, 236, 108, 227)
- a USB device to a computer for recovery: REQ-PRD-08.B, REQ-PRD-19, REQ-SOC-10 (VER-259, 260). No usbc-level row of its own; the product rows carry the number and the checks.
### display
- the ILI9488 on the 8-bit 8080 bus: REQ-DSP-01 to -04, -10, -11, REQ-PRD-02.B (VER-224, 60, 61, 199, 2, 136, 44, 106, 219, 63)
- the backlight: REQ-DSP-05, -06, -07, -09 (VER-62, 137, 5)
### audio
- play sound from the flash: REQ-PRD-18, REQ-AUD-01, -03, -04, -05, -06 and, new, REQ-AUD-07 (VER-256, 255, 64, 100, 200, 196, 128)
### buttons
- each seat's D-pad, A, B, X, Y, Start and Select, read directly on GPIOs: REQ-PRD-01.B, REQ-BTN-01 to -08, -10 (VER-56, 76, 45, 46, 248, 8, 132, 162, 50, 174, 51, 94, 178, 180, 199)
### case
- hold, protect, open, cool, today's size or smaller: REQ-PRD-17, REQ-MEC-01 to -12, REQ-THM-01, -04, REQ-BAT-07, -08, -15 (VER-230, 48, 20, 6, 112, 92, 147, 165, 142, 86, 144)
### board-wide (spec lines outside the nine subsystems)
- two layers, JLCPCB batches: REQ-FAB-03 (JLC's 2-layer limits), REQ-USB-02; test pads and a debug connector: REQ-SOC-08, REQ-TST-01; nothing soldered, plug-in cell and speaker: REQ-ASM-01.B, REQ-BAT-14; $60 at 10 and $40 at 50: REQ-PRD-09 (VER-117)
- GAP, no row: 'There are no indicator lights on the board' (DESIGN.md:63). Added REQ-FAB-13 and VER-265.
- GAP, no live row behind a live check's limit: VER-64 and VER-100 test a 40 dB carrier and the audio source's 32 kHz; REQ-AUD-02 that held it was dropped with the ESP32-S3 board. Added REQ-AUD-07.

Functions without a requirement before this review: 2 (now rows, PROPOSED). Left: 0.

## Test 2: every live row has a number and tolerance, a primary Source and a live check
- A live check (not RETIRED, DROPPED or CUT, and named in the register): all 135 live rows, 0 failures (a parent row lists its allocation).
- A number and tolerance: all but 4 rows (DEF-106): REQ-SOC-04 and REQ-PRD-08.B ('about 8 s'), REQ-SOC-03 (the crystal has no start-up time), REQ-BAT-16 ('about 140 mA').
- A primary Source (a datasheet line, a standard's clause or Alex's decision; a row Alex APPROVED counts as his decision; pin_review files count as the datasheets they quote): FAILS for 54 live rows (15: the design's own files, 24: validation_plan only, 15: Claude's estimate/analysis/review). Row by row:
  - REQ-BAT-05.B [the design's own files]: carryover.md C-7.
  - REQ-BAT-07 [validation_plan only]: validation_plan #26; case.scad (cell_top 1.6 over the board; the panel's back 2.4, the pan
  - REQ-BAT-08 [validation_plan only]: validation_plan #27; placement.py CUTOUTS (the hole 37.8 x 54.8 mm since 71737ae); case.sc
  - REQ-BAT-10 [Claude's estimate/analysis/review]: Claude's estimate; validation_plan #33.
  - REQ-BAT-14 [Claude's estimate/analysis/review]: Claude's analysis (O-42).
  - REQ-BAT-15 [Claude's estimate/analysis/review]: Claude's review (F-14).
  - REQ-PWR-09 [the design's own files]: design.py R42 (PWR_HOLD's pull-down); pinmap.md Power control; analysis/errata_rp2350.md.
  - REQ-PWR-10 [validation_plan only]: validation_plan gate item 3 and Fable's note 1; pinmap.md.
  - REQ-PWR-14 [Claude's estimate/analysis/review]: validation_plan #5, #123; Claude's proposal, revised (O-22).
  - REQ-DSP-06 [the design's own files]: board/sim/backlight_results.txt (to re-run).
  - REQ-DSP-09 [the design's own files]: board/sim/backlight_results.txt (4.7 mA peak, an older circuit); since 2026-10-28 VER-05 p
  - REQ-DSP-10 [the design's own files]: bringup.md display step 6.
  - REQ-AUD-03 [the design's own files]: design_review.md Audio.
  - REQ-AUD-04 [the design's own files]: design_review.md Audio; bringup.md step 4.
  - REQ-AUD-06 [Claude's estimate/analysis/review]: bom.md (INGHAi GSPK151035PN, C530539, 0.5 W; the RP board's, the speaker stays); Claude's 
  - REQ-BTN-02 [validation_plan only]: validation_plan gate item 5, #185.
  - REQ-BTN-03 [the design's own files]: hardware/ref_design/board/membrane/README.md; check_contact_fp.py; case/DSLITE.md.
  - REQ-BTN-04 [the design's own files]: design_review.md Membrane contacts.
  - REQ-BTN-05 [validation_plan only]: validation_plan #43; case/DSLITE.md.
  - REQ-BTN-06 [validation_plan only]: validation_plan #197.
  - REQ-BTN-07 [Claude's estimate/analysis/review]: Claude's estimate; validation_plan #159.
  - REQ-BTN-08 [the design's own files]: design.py SW20-SW23 (Dome_4mm_Round); case.scad ss_metal_dome, ss_lift; hardware/ref_desig
  - REQ-MEC-01 [validation_plan only]: validation_plan #96, #130, gate item 7.
  - REQ-MEC-02 [validation_plan only]: validation_plan #25, #132, gate item 7.
  - REQ-MEC-03 [the design's own files]: case/check_buttons.py.
  - REQ-MEC-04 [the design's own files]: assembly.md sections 3 and 5.
  - REQ-MEC-05 [validation_plan only]: validation_plan #194; assembly.md steps 9-10.
  - REQ-MEC-06 [validation_plan only]: validation_plan #183.
  - REQ-MEC-07 [validation_plan only]: validation_plan #51.
  - REQ-MEC-08 [the design's own files]: 177123c.
  - REQ-MEC-09 [Claude's estimate/analysis/review]: case.scad; Claude's analysis (O-52).
  - REQ-MEC-10 [Claude's estimate/analysis/review]: Claude's review (O-59).
  - REQ-MEC-11 [Claude's estimate/analysis/review]: Claude's review (F-28).
  - REQ-MEC-12 [Claude's estimate/analysis/review]: Claude's review (O-43); case.scad tub_boss_d.
  - REQ-THM-02 [validation_plan only]: placement.py TH1, CUTOUTS; case.scad; validation_plan #30.
  - REQ-THM-03 [the design's own files]: bringup.md step 5.4.
  - REQ-THM-04 [Claude's estimate/analysis/review]: Claude's analysis (O-47, F-48).
  - REQ-FAB-01 [validation_plan only]: validation_plan #75, #76, #173.
  - REQ-FAB-02 [validation_plan only]: validation_plan #1, #113, gate item 1.
  - REQ-FAB-05 [validation_plan only]: assembly.md section 1; validation_plan #189.
  - REQ-FAB-06 [validation_plan only]: validation_plan #2, #122.
  - REQ-FAB-07 [validation_plan only]: validation_plan #9, #116.
  - REQ-FAB-08 [validation_plan only]: validation_plan #12, #88, #121, gate item 11.
  - REQ-FAB-09 [validation_plan only]: validation_plan #103.
  - REQ-FAB-10 [the design's own files]: DEF-12, DEF-13, DEF-29.
  - REQ-ASM-01 [validation_plan only]: validation_plan #86, #161.
  - REQ-ASM-02 [validation_plan only]: assembly.md step 4; validation_plan #160.
  - REQ-TST-01 [validation_plan only]: validation_plan #21, #155.
  - REQ-TST-02 [validation_plan only]: validation_plan #57, #100, #174, #202.
  - REQ-TST-03 [validation_plan only]: validation_plan #195.
  - REQ-TST-05 [Claude's estimate/analysis/review]: Claude's estimate.
  - REQ-TST-06 [Claude's estimate/analysis/review]: Claude's review (O-63, O-60).
  - REQ-TST-07 [Claude's estimate/analysis/review]: Claude's review (O-61).
  - REQ-TST-08 [Claude's estimate/analysis/review]: Claude's review (O-36).

## Test 3: live rows that cover a function the spec no longer has (flagged)
- REQ-PWR-07 rationale said the slider still turns the unit on: fixed (seat 1's Start), statement unchanged.
- Checks (not rows) that still say slider, power switch or 'slid on': VER-28, VER-65 (pass limit), VER-112, VER-167, VER-198, VER-199, VER-241, VER-262 (the slider as an input read on a GPIO, or the slider held 8 s). Not edited here (their evidence is NOT_BUILT or NOT_RUN); each needs the word changed to seat 1's Start at its next edit. VER-241's bench steps ('the power slider held about 8 s') will not run as written.
- REQ-PRD-08 cites DESIGN.md:63 for 'no separate reset button' (now :64); REQ-PRD-14 and REQ-PRD-15 cite :48, :49, :81, :83, :84 and REQ-PRD-17 cites :60, REQ-PRD-18 cites :71, REQ-PRD-02.B :55 (all moved by the Sleep line). Alex's signed rows: not edited. Their text still matches the spec; only the line numbers are off.
- REQ-BTN-01 and -10 mention the 74HC165 only as the pin-relief option: correct as written.

## Findings logged
- DEF-105: 54 live rows without a primary Source. DEF-106: 'about' with no tolerance.
- New rows: REQ-AUD-07 (VER-100, VER-64), REQ-FAB-13 (VER-265, new, NOT_BUILT). Fixed: REQ-PWR-07 rationale; DESIGN.md line cites in REQ-SOC-01, -02, -08, REQ-USB-01, -02, REQ-ASM-01.B; REQ-SOC-04 proposes 4 s to 12 s.

Pass limit: 0 functions without a requirement (met after the two rows); 0 live requirements without a number and tolerance, a primary Source and a live check (NOT met: 54 sources, 4 tolerances); each finding logged as a requirement or a DEF (met).
RESULT: FAIL
