# Number sources, batch 3 (2026-10-30)

Queue issue #89, batch 3: the `- Numbers:` field for the live rows of REQ-PRD, REQ-FAB, REQ-ASM, REQ-TST and REQ-EMC. The format and the tags are those of analysis/number_sources_2026-10-30.md and README.md, 'Where a requirement's numbers come from'. No Statement was touched (`git diff` adds lines only).

## 1. Scope

47 live rows tagged. Rows with no number in the Statement (REQ-FAB-08) carry no field. The REQ-MCU, REQ-RF, REQ-STO and REQ-PBD rows are all DROPPED or RETIRED (no live row, nothing to tag). REQ-PRD-01.R to -04.R, -06, -08.R, -12, -13, -16, REQ-FAB-07.R and REQ-ASM-01.R are DROPPED and were skipped. The signed REQ-PRD rows got the Numbers field only.

## 2. Counts

124 numbers in the Statements, by the tag that covers them (a number written twice counts twice):

| Tag | Numbers | Share |
|---|---|---|
| datasheet | 7 | 6% |
| standard | 2 | 2% |
| spec | 31 | 25% |
| derived | 5 | 4% |
| judgment | 40 | 32% |
| count | 39 | 31% |

103 entries, plus 7 `in:` inputs for derived numbers (5 spec, 1 datasheet, 1 derived). The text of IEC 62368-1 edition 1 (2010) was read for REQ-PRD-11 (Annex T.7, drop test: three impacts, 1 000 mm ± 10 mm for hand-held equipment, hardwood). No other standard's text is on disk (USB 2.0, IEC 61000-4-2, EN 55035, FCC Part 15, CISPR 32, IPC-6012, IPC-A-610, AS9102), so the numbers they set are `judgment` with `unread` in the entry, for the later web job (`grep -n unread requirements.md`).

## 3. The `judgment:` numbers

One line each; the row's Numbers field has what looser or tighter would cost.

| Row | Value | Reason |
|---|---|---|
| REQ-PRD-03 | 25 C | room temperature, the usual bench condition and the one a runtime is quoted at (the cell's own sheet is not on disk, VER-37) |
| REQ-PRD-03 | 50% | Claude's mid-brightness figure for a typical session |
| REQ-PRD-03 | -12 dBFS | Claude's moderate listening level |
| REQ-PRD-03.B | 50% | the same figure as REQ-PRD-03 (Claude's mid-brightness choice) |
| REQ-PRD-03.B | -12 dBFS | the same figure as REQ-PRD-03 (Claude's moderate listening level) |
| REQ-PRD-03.B | 25 C | the same bench condition as REQ-PRD-03 |
| REQ-PRD-04 | 4.0 h | Claude's estimate ('a flat unit is ready again within an evening', Source: Alex to set the target) |
| REQ-PRD-04 | 25 C | room temperature, the usual bench condition |
| REQ-PRD-04.B | 4.0 h | the same figure as REQ-PRD-04 (Claude's estimate approved by Alex, A-008) |
| REQ-PRD-04.B | 25 C | the same bench condition as REQ-PRD-04 |
| REQ-PRD-04.B | 4.75-5.25 V | the voltage range of a USB 2.0 source at the connector (cl. 7.2.1, text unread, as REQ-PWR-16) |
| REQ-PRD-05 | 30 uA | Claude's budget: 700 mAh over 2 years of 17520 h is 40 uA, so 30 uA keeps 25% margin under it and sits just above the board's paper sum of about 26 uA (REQ-PWR-07) |
| REQ-PRD-05 | 2 years | Claude's estimate of a unit left in a drawer (Source: Alex to set), approved by Alex as worded on 2026-10-29 (A-009) |
| REQ-PRD-07 | 1 or more independent protection layers | Claude's reading of IEC 62368-1:2010 Annex B.4.1, which treats 'a basic safeguard failure or a supplementary safeguard failure' as the single fault, so one safeguard must still stand, the standard sets no count for the board's own layers |
| REQ-PRD-10 | 10 min | Claude's figure for a screwdriver swap of a plug (J2) or a ZIF (J3) |
| REQ-PRD-17 | 0.3 mm | Claude's print tolerance (Rationale: the spec says 'no growth', Alex sets it), the home printer's repeatability is not measured |
| REQ-EMC-01 | +-4 kV | IEC 61000-4-2 and EN 55035 Table 1 are believed to set the contact level (both unread, no text on disk), a protection part's own rating is a separate matter |
| REQ-EMC-01 | +-8 kV | IEC 61000-4-2 and EN 55035 Table 1 are believed to set the air level (both unread, no text on disk) |
| REQ-EMC-02 | Part 15 | the number is the name of FCC 47 CFR Part 15 Subpart B (unread, no text on disk) and sets no value in the Statement, the limit is the Class B one the test house applies |
| REQ-EMC-02 | CISPR 32 | the number is the name of CISPR 32 (unread, no text on disk) and sets no value in the Statement |
| REQ-FAB-03 | 0.3 mm hole | the smallest hole the board uses, set by FAB-04's 0.3 mm, which build_pcb.py:6 and :114 call the price break and not JLC's limit |
| REQ-FAB-03 | 0.30 mm or more | 0.1 mm over JLC's published 0.2 mm from routed edges (jlcpcb.com capabilities page read 2026-10-27, build_pcb.py:121-122, not a file on disk) |
| REQ-FAB-04 | 0.30 mm or larger | the price break at JLC, 'smaller holes cost about $40 an order' (Rationale, build_pcb.py:6), JLC's own limit is lower (build_pcb.py:114) |
| REQ-FAB-05 | 1.6 mm | the thickness build_pcb.py sets (SetBoardThickness 1.6) and the usual 2-layer one |
| REQ-FAB-07.B | 0.1 mm | Claude's figure: no JLC placement tolerance is on disk and fine-pitch parts such as the RP2350B's QFN-80 are the ones a shift hurts |
| REQ-FAB-09 | 100% | every board in the lot is e-tested, so a bad board cannot ship, JLC e-tests per board and a sample leaves untested boards |
| REQ-FAB-10 | 1.1 x | Claude's 10% over the order quantity before attrition (Rationale, Derivation) |
| REQ-FAB-11 | 0.75% | IPC-6012 Class 2's bow and twist figure for surface-mount boards (standard unread, no text on disk, Source) |
| REQ-FAB-12 | Class 2 | IPC-A-610 Class 2 (unread, no text on disk) is the usual class for dedicated-service electronics |
| REQ-ASM-01 | 30 min | Claude's figure: 30 units by hand at 30 min each is 900 min, 15 h (Rationale, Derivation) |
| REQ-ASM-01.B | 30 min | the same figure as REQ-ASM-01 (30 units by hand at 30 min each is 15 h) |
| REQ-TST-02 | 5 min | Claude's figure: 30 units at 5 min each is 150 min, 2.5 h of test for the PVT run (Rationale, Derivation) |
| REQ-TST-04 | 100% | every BOM line of the first unit (AS9102, unread, Source, is the name of the first article practice) |
| REQ-TST-05 | 90% | Claude's figure: 90% of the 30-board PVT lot is 27 units (0.9 x 30) passing first time (Rationale, Derivation), below it the process and not the unit is the problem |
| REQ-TST-06 | 4.096 V | a 4.096 V reference is a common precision value that is exact to read on a DMM (Claude's choice) |
| REQ-TST-06 | 0.95-1.05 | Claude's window: a gain of plus or minus 5% is five times the charger's +-1% constant voltage (Rationale, Derivation) |
| REQ-TST-06 | 10 mV | Claude's figure: about a quarter of the charger's +-42 mV, which is 1% of 4.20 V (REQ-BAT-01, Rationale, Derivation) |
| REQ-TST-07 | 50% | Claude's figure: half the connector's rated life, the FH12 is rated 20 cycles so 10 matings a unit (Derivation) |

## 4. The `count:` numbers

| Row | Value | Reason |
|---|---|---|
| REQ-PRD-02 | 0 missing lines or columns | none allowed, so zero by definition and no sample: a missing line or column is a visible panel or bus defect (Rationale: the shared screen is the product) |
| REQ-PRD-02.B | 0 missing lines or columns | none allowed, so zero by definition and no sample, as in REQ-PRD-02 |
| REQ-PRD-08 | 3 out of 3 | three tries each, matching REQ-PRD-08.B (the Notes say the old 2 of 2 was replaced) |
| REQ-PRD-08.B | 3 out of 3 | three tries each |
| REQ-PRD-14 | 5 of 5 | five random battles, Claude's proposal that Alex approved as written on 2026-10-29 (Rationale) |
| REQ-PRD-15 | 10 of 10 | ten power-ons, Claude's proposal approved by Alex (Rationale) |
| REQ-PRD-18 | 0 audible dropouts | none allowed, so zero by definition and no sample: a buffer underrun is audible |
| REQ-PRD-19 | 3 out of 3 | three tries each |
| REQ-PRD-20 | 10 of 10 | ten Sleep and resume tries (VER-263) |
| REQ-PRD-20 | 3 of 3 | three firmware-mismatch tries (VER-263) |
| REQ-FAB-01 | 0 differences, ERC shall report 0 errors, ... | none allowed, so zero by definition and no sample: any ERC, DRC or parity issue is a drawn fault (Rationale, Derivation) |
| REQ-FAB-02 | 0 unresolved issues, 0 unknown ratings and... | none allowed, so zero by definition and no sample: only the datasheet can say a pin is wrong (Rationale, Derivation) |
| REQ-FAB-03 | 0 errors | none allowed, so zero by definition and no sample: a DRC error is a drawn fault |
| REQ-FAB-05 | 1 record per order | one release record for each order, so the options of an order are in one place |
| REQ-FAB-06 | 0 substitutions | none allowed, so zero by definition and no sample: the right family in the wrong variant passes every other check (Rationale) |
| REQ-FAB-07 | 0 unexplained differences | none allowed, so zero by definition and no sample: a part placed off its pin 1 kills the board (Rationale, REQ-FAB-07.B) |
| REQ-FAB-09 | 0 pairs | none allowed, so zero by definition and no sample: a pair of differently named nets joined by copper is a short |
| REQ-FAB-12 | 0 bridges, tombstones, lifted leads or mis... | none allowed, so zero by definition and no sample: each is a visible assembly fault |
| REQ-ASM-01 | 1 unit | one unit per timing, the time study of VER-91 is one unit by Alex |
| REQ-ASM-01 | 0 parts | none allowed, so zero by definition and no sample: a damaged part is a lost unit |
| REQ-ASM-01.B | 1 RP2350B unit | one unit per timing, the time study of VER-91 is one unit by Alex |
| REQ-ASM-01.B | 0 parts | none allowed, so zero by definition and no sample: a damaged part is a lost unit |
| REQ-ASM-02 | 10 case closures | ten closures |
| REQ-TST-01 | 0 measurements | none allowed, so zero by definition and no sample: a measurement that needs a lifted part is rework (Rationale, Derivation) |
| REQ-TST-03 | 1 record | one record per serial, linking chip ID, cell lot, test log and release |
| REQ-TST-08 | 2 units | two units, the smallest count that shows a pattern (Rationale, Derivation) |

## 5. Wrong cites and Statement-versus-source findings

No cite to a datasheet line, a DESIGN.md line or next_spin_requirements.md line in these rows is wrong; each was read. Checked: DESIGN.md:39, :42, :43, :49, :50, :56, :59, :60, :61, :62, :63, :64, :65, :71, :74, :84, :86, :87, next_spin_requirements.md:12, :13, :15, :17, :18, :27, ILI9488 twc 40 ns (ili9488.txt:16159, section 17.4.1), the panel's IM0 to IM2 on pins 7 to 9 (er-tft035ips-6.txt:596-614), hirose_fh12.txt:58 (20 cycles), IEC 62368-1 Annex T.7 and B.4 (edition 1).

What does not line up:

- REQ-PRD-03 and REQ-PRD-04 Source say "Claude's estimate; Alex to set the target", but REQ-PRD-03.B's Rationale says the 4.0 h of play time is Alex's (2026-10-29) and releases/APPROVALS.md has A-005 to A-008 approving all four rows. The play time 4.0 h is tagged `spec`; the charge time 4.0 h stays `judgment` (nobody recorded Alex setting it, only approving it). The Source text of the parent rows is stale; left as written (a Source edit is not this job).
- REQ-PRD-04 and -04.B: "a source offering 0.5 A" against the charger's USB500 limit, which the datasheet gives as 450 / 475 / 500 mA (bq24074.txt:733). A charger at its minimum takes 450 mA from a 0.5 A source, so the 4.0 h has to hold at 450 mA less the system draw. REQ-PRD-04's Notes already say 475 mA.
- REQ-PRD-15 against VER-228: the Statement says 10 of 10 power-ons; VER-228 says "20 of 20 (REQ-PRD-15)" for the early-release part (10 releases at 1.0 s and 10 at 2.4 s), which the Statement does not state a count for.
- REQ-PRD-11: IEC 62368-1:2010 Annex T.7 gives 1 000 mm ± 10 mm; the Statement says 1.0 m with no tolerance. Not a disagreement, but the test height has a tolerance the check should hold.
- REQ-PRD-18 and VER-231 cite gd25q128.pdf for the flash; no GD25Q128 datasheet is in hardware/ref_design/datasheets, so the 16 MB rests on DESIGN.md:74.
- REQ-FAB-03: the 0.30 mm copper-to-edge figure is 0.1 mm over JLC's published 0.2 mm (build_pcb.py:121-122, a web read of 2026-10-27, no file on disk); the Notes already say the as-built closest is 0.108 mm.
- README.md's design baseline differs from HEAD (hardware/ref_design/tools/build.py, case/case.scad, case/export.py), so tools/check_trace.py prints this STRUCTURE error on main too, before and after this batch; seed_faults.py then catches 109 of 153 seeded faults on main and on this branch alike. The README is another job's.

## 6. Left to tag

Next: the REQ-BAT, REQ-PWR, REQ-SOC and REQ-USB rows were done in batch 1; REQ-DSP, AUD, BTN, MEC, THM and the BAT, PWR, SOC, USB rows are on another branch. Then `NUMBERS_REQUIRED = True` in tools/check_trace.py.

Last updated: 2026-10-30 13:49 EDT
