# Where the numbers come from: the Numbers field and its first batch (2026-10-30)

Why: Alex asked how many requirements carry numbers Claude made up with no source, and whether the register sources its reasoning systematically. It did not: tools/check_trace.py checked only that a row had a Source field. REQ-PWR-14 was one of the unsourced ones (analysis/derating_rule_2026-10-30.md shows what a sourced number looks like). This file records the fix: a per-requirement `- Numbers:` field, a checker for it, and the first batch of rows tagged.

## 1. The format

After Source, each live requirement carries `- Numbers:`, one entry per number in its Statement, separated by `; `, each `<value as written in the Statement> = <tag>: <detail>`. Tags: `datasheet` (file.txt:line and a quote, verified against the PDF by tools/ds.py's rule: on the line or the next), `standard`, `spec` (DESIGN.md or next_spin_requirements.md line, or Alex's words with their date and where recorded), `derived` (arithmetic whose every input is tagged), `judgment` (Claude's choice, with what looser or tighter would cost), `count` (sample sizes and test counts). The full rules, including what is not a number (IDs, designators, part numbers, dates, line cites, clause cites, seat and pin labels, list numbering), are in README.md, 'Where a requirement's numbers come from'.

Three conventions the batch needed: an `in: <value>` entry tags an input a derived number uses that the Statement does not state; a number set by a standard whose text is not on disk is tagged `judgment` and the standard's name carries the word `unread` (so `grep -n unread requirements.md` lists the numbers the later web job should check against the real text, and re-tag as `standard`); no `standard:` tag is used in this batch, because no standard's text is on disk.

## 2. Approvals

The field does not touch any approval. check_trace.py matches an approval to a row by the git blame of the Statement's own lines (cross_check_approvals, `r["lines"]["Statement"]`) and, for a `REQ-ALL@<commit>` line, by the Statement text alone (statements_at). A new `- Numbers:` line is a different field on different lines, so a signed row stays signed. The batch touches no Statement (`git diff` shows only added lines in requirements.md). It also added no line to releases/APPROVALS.md.

## 3. The checker

check_trace.py (`check_numbers`, run from `load`): for every live requirement (Status not DROPPED or RETIRED, Board rp2350b or both) it extracts the Statement's numbers and, for a row with a Numbers field, requires each to be inside some entry's value, each entry's value to be written in the Statement, the tag to be known, a `datasheet:` quote to be on its line (tools/ds.py `on_line`), a `spec:` line to exist, a `derived:` input to be tagged in the row or in a named row, and a `judgment:` to say looser or tighter; each fault is a structure error. A live row with no field is counted: the report prints the untagged numbers per row and in total, as a warning, with `NUMBERS_REQUIRED = False` in tools/check_trace.py to be set to True when every row is tagged. tools/seed_faults.py has five seeded faults for it (an untagged number, a wrong datasheet quote, an unknown tag, a judgment with no cost, a derived input nobody tagged).

## 4. Counts for this batch (REQ-BAT, REQ-PWR, REQ-SOC, REQ-USB)

45 live rows with numbers in their Statement (REQ-SOC-08 has none), 142 numbers, by the tag that covers them:

| Tag | Numbers | Share |
|---|---|---|
| datasheet | 13 | 9% |
| standard | 0 | 0% |
| spec | 19 | 13% |
| derived | 13 | 9% |
| judgment | 82 | 58% |
| count | 15 | 11% |

124 entries, 4 of them `in:` inputs for derived numbers (3 judgment, 1 spec). `judgment` is the biggest tag: most of these rows' numbers were Claude's choice and the register now says so.

## 5. The `judgment:` numbers

Row, value as written, the one-line reason. Each has the full reason, with what looser or tighter would cost, in its row's Numbers field. Numbers marked unread name a standard believed to set them whose text is not on disk.

| Row | Value | Reason |
|---|---|---|
| REQ-BAT-01 | +-1% | the charger's own VBAT(REG) spread is 4.16 to 4.23 V (bq24074.txt:773 "4.16 4.20 4.23"), -0.95% to +0.71% of 4.20 V, so +-1% holds every part inside its datasheet |
| REQ-BAT-02 | 0.5 C | Claude's ceiling, 'the usual standard rate for small pouch cells' (Rationale); the cell maker's own standard charge current (IEC 62133-2, unread) still caps it |
| REQ-BAT-02.B | 0.5 C | the same ceiling as REQ-BAT-02 (Claude's choice, no source on disk; the cell's spec sheet is not on disk, VER-37) |
| REQ-BAT-03 | 99.9% | Claude's population claim: the worst corner of the TS thresholds and of TH1's +-1% bounds every unit (Notes), so 99.9% is the figure that corner supports |
| REQ-BAT-03 | 45 C | the usual Li-ion charge ceiling, named by IEC 62133-2 as the cell maker's charge limit (standard unread; the cell's spec sheet is not on disk, VER-37) |
| REQ-BAT-03 | 0 C | the usual Li-ion charge floor (lithium plating below it), IEC 62133-2 names the cell maker's limit (unread) |
| REQ-BAT-04 | 45 C | the cell's charge ceiling as in REQ-BAT-03 (IEC 62133-2 unread, validation_plan #31) |
| REQ-BAT-04 | 25 C | room temperature, the usual bench condition |
| REQ-BAT-04 | 35 C | Claude's warm-room figure for the second test |
| REQ-BAT-06 | 4.35 V | Claude's limit, 0.12 V over the charger's 4.23 V maximum (bq24074.txt:773 "4.16 4.20 4.23") so the PCM only acts on a charger fault; the cell's spec sheet is not on disk (VER-37,... |
| REQ-BAT-06 | 2.4 V | Claude's under-voltage floor, a DW01-class PCM figure (REQ-BAT-10 Rationale), well under the 3.50 V cutoff (REQ-BAT-12) so the PCM is the second layer |
| REQ-BAT-07 | 0.4 mm | 0.4 mm / 4.0 mm = 10%, the top of the 8-10% pouch growth over life that the Rationale gives (no source for the 8-10% is on disk) |
| REQ-BAT-07 | 4.5 mm | the worst-case cell thickness, a 4.0 mm nominal plus tolerance; the chosen cell's spec sheet is not on disk (VER-37) |
| REQ-BAT-08 | 0.5 mm | the margin over the fab, print and fastening tolerance stack so no edge touches the pouch (Rationale: 'a design choice', no datasheet sets it) |
| REQ-BAT-08 | 1.5 mm | the lead end's larger margin, where the leads leave and the pouch is stiffest and flexes least (Claude's choice, no source on disk) |
| REQ-BAT-08 | 0.1 mm | the pocket's allowance per side over a worst-case cell after the print's tolerance (Rationale) |
| REQ-BAT-08 | 0.4 mm | the fit clause's allowance in each direction, a PROPOSED number from round 3's O-S1 (2026-10-27; Rationale) |
| REQ-BAT-08 | 1 mm | back-side copper kept 1 mm from the hole so a pierced pouch has nothing to short against (Claude's choice, no source on disk) |
| REQ-BAT-09 | 4.25 V | 20 mV above the BQ24073's own maximum VBAT(REG) of 4.23 V (bq24074.txt:773 "4.16 4.20 4.23") so a charger inside its datasheet is no fault; the reason for the exact 4.25 is not recorded... |
| REQ-BAT-10 | 2 months | Claude's estimate for a unit left in a drawer; the Rationale's paper arithmetic shows it is not met at the high end of drain and self-discharge while 1 month mostly is, and Alex is asked... |
| REQ-BAT-10 | 25 C | room temperature, the usual storage condition |
| REQ-BAT-12 | 25 C | room temperature, where the per-unit calibration is done (REQ-TST-06) |
| REQ-BAT-12 | 3.50 V | Claude's cutoff choice (DF-225, carryover P-9): high enough to stay far above the PCM's 2.4 V trip, low enough to keep the usable band (only 5-10% of the capacity lies between 3.50 V and... |
| REQ-BAT-12 | +0.05/-0.00 V | one-sided: the cutoff may be up to 50 mV late-high but never below 3.50 V; 50 mV is what an ADC and reference of +-50 mV (DF-225) can hold after per-unit calibration (VER-138) |
| REQ-BAT-14 | in: 0.3 V ringing margin | the margin for ringing on VSYS at the plug (Rationale: 'a design choice'; no datasheet gives the plug's inrush) |
| REQ-BAT-15 | 0 bare lead insulation | zero, because one worn lead shorts CELL_P to the GND pour with only the PCM left (Rationale, DF-214) |
| REQ-BAT-16 | in: 40 mAh | 7% margin over the 560 mAh a session uses (Rationale) |
| REQ-BAT-16 | 140 mA | Claude's estimate of the unit's draw is 100 to 120 mA, so 140 mA is 17% to 40% more than that (Rationale) |
| REQ-BAT-16 | +-10% | so a bench supply can be read to it (Rationale) |
| REQ-BAT-16 | 3.50 V | the cutoff REQ-BAT-12 chooses (DF-225); recorded so the choice can be made on measured capacity |
| REQ-BAT-16 | 3.60 V | the alternative, higher cutoff the cutoff choice (DF-225) weighs against 3.50 V |
| REQ-PWR-01 | 4.40 | the lowest VBUS at which the unit with a cell is claimed to run: just under the BQ24073's VIN-DPM typical 4.5 V (bq24074.txt:739 "4.35 4.5 4.63") so the cell makes up the input current... |
| REQ-PWR-01 | 5.50 V | the upper edge, 0.25 V over USB 2.0's 5.25 V (unread) to take in a high charger, and under the charger's input over-voltage threshold |
| REQ-PWR-01 | 4.75 | without a cell nothing makes up the input current, so the floor sits above the VIN-DPM maximum 4.63 V (bq24074.txt:739 "4.35 4.5 4.63") and at USB 2.0's source floor (cl. 7.2.1, unread) |
| REQ-PWR-01 | 500 mA | the 500 mA of a standard USB 2.0 port (USB 2.0 cl. 7.2.1 names it but the standard's text is not on disk (unread)), the charger's USB500 limit reads 450 475 500 mA (bq24074.txt:733 "EN1... |
| REQ-PWR-16 | 4.75-5.25 V | the voltage range of a USB 2.0 source at the connector (cl. 7.2.1, text unread) |
| REQ-PWR-16 | 4.40 V | the lower edge of REQ-PWR-01's run range, so the measurement covers the whole gap between run and charge; DEC-15 lets Alex raise it or accept |
| REQ-PWR-16 | 4.75 V | the bottom of USB 2.0's source range (cl. 7.2.1, text unread) and above the VIN-DPM maximum 4.63 V (bq24074.txt:739 "4.35 4.5 4.63") |
| REQ-PWR-02 | 2 m | a long cable that rings harder than a short one at a live plug |
| REQ-PWR-02 | 0.2 V | the margin under the charger's over-voltage minimum, which the Rationale takes from the minimum and not the 6.6 V nominal (O-R18) |
| REQ-PWR-04 | +-3% | a window U3 can meet on paper: output accuracy -2 to +2 % at 10 mA (rt9013.txt:161 "−2 0 +2 %") plus load regulation up to 1 % (the same table) leaves no margin for line regulation, so... |
| REQ-PWR-04 | from 0 to | the load range starts at no load, where an LDO's output is highest |
| REQ-PWR-05 | backlight 100% | full brightness is the heaviest load on the rails through the hand-over |
| REQ-PWR-07 | 30 uA | the budget REQ-PRD-05 states (2 years from a full cell, Claude's estimate); the RP board's circuits sum to about 26 uA (U2 6.5, R17 into U10 7.9, U7 8.4, FETs 3.0, U3 at its shutdown... |
| REQ-PWR-07 | 25 C | room temperature, the usual condition for a leakage figure |
| REQ-PWR-07 | 60 uA | Claude's estimate that leakage doubles from 25 C to 45 C (O-51), no source on disk |
| REQ-PWR-07 | 45 C | Claude's estimate of a hot pocket or car dashboard (O-51) |
| REQ-PWR-07 | 3.7-4.2 V | 3.7 V is the Li-ion nominal and 4.2 V is a full cell (bq24074.txt:773 "4.16 4.20 4.23"), the range the cell spends most of its life in |
| REQ-PWR-09 | 0.5 V | the level counted as off (Rationale: 'a design choice'), below any rail's logic threshold |
| REQ-PWR-09 | 2 s | matches the watchdog's 2 s in REQ-SOC-04, so a crashed unit takes no longer to turn off than a hung one takes to reset (Rationale) |
| REQ-PWR-09 | 12 s | the hold may run past the TPS3421's maximum delay of 9 s (tps3421.txt:308 "TPS3421Ey, TPS3422Ey: TS = GND 6 7.5 9") so the case where the firmware reboots with Start still held is tested |
| REQ-PWR-10 | in: 2.5 times margin | 2.5 times the 4 s the two steps take, for a slow image to reach its first watchdog enable (Rationale: 'a design choice') |
| REQ-PWR-11 | 1 Mohm | Claude's model of sweat, flux or a pressed cap bridging the contact (DF-242) |
| REQ-PWR-11 | 10 min | long enough for a latch that needs a slow leak to build up to show itself |
| REQ-PWR-12 | 0.25-0.61 V | the CC voltage band of a default-power USB Type-C source (the Type-C specification, unread) |
| REQ-PWR-12 | 500 mA | the 500 mA of a standard USB 2.0 port (USB 2.0 cl. 7.2.1 names it but the standard's text is not on disk (unread)); the charger's USB500 limit is 475 mA typical, 500 mA maximum... |
| REQ-PWR-12 | 2 s | Claude's allowance for the charger to settle after plug-in |
| REQ-PWR-13 | 10 uF | USB 2.0's capacitance limit at a plug (cl. 7.2.4.1, text unread) |
| REQ-PWR-13 | 50 uC | USB 2.0's charge limit at a plug (cl. 7.2.4.1, text unread) |
| REQ-PWR-13 | 5.25 V | the top of USB 2.0's source range (cl. 7.2.1, text unread), the worst case for the charge a capacitor takes |
| REQ-PWR-14 | 0.3 V | Claude's margin below absolute maximum for an IC transient (O-22); analysis/derating_rule_2026-10-30.md (section 1) finds no standard sets a fixed volts figure, and proposes replacing it... |
| REQ-PWR-14 | 80% | matches ECSS-Q-ST-30-11C Table 6-34 (VDS 80%) and Table 6-29 (chip resistor voltage 80%) and Murata's consumer MLCC 80% as quoted in analysis/derating_rule_2026-10-30.md, but not the... |
| REQ-PWR-14 | 90% | Claude's figure for a transient under 10 us (O-22); ECSS 5.4.2e (analysis/derating_rule_2026-10-30.md) allows the steady-state limit plus 10% or 80%, whichever is lower, for repeated... |
| REQ-PWR-14 | 10 us | Claude's boundary between a transient and steady state (O-22); no standard on disk sets a time; the Statement is about to change |
| REQ-PWR-17 | 6-20 V | 6 V just over the USB source ceiling and 20 V the top of a USB PD source (the PD specification, unread) |
| REQ-PWR-21 | 2 s or more | Claude's decision, 2026-10-30 (Rationale): 2 s plus the longest erase or write (under 2 s) ends by 4 s, 2 s before the TPS3421's 6 s minimum delay (tps3421.txt:308 "TPS3421Ey, TPS3422Ey:... |
| REQ-SOC-03 | 10 ms | 1% of the 1.0 s boot budget after the hold (REQ-PRD-15), 10 ms / 1000 ms; neither the RP2350 datasheet's nor the crystal's start-up time is read here, VER-237 measures it |
| REQ-SOC-04 | 2 s | Claude's watchdog timeout; the RP2350's watchdog LOAD register reaches about 16 s so 2 s is settable (rp2350.pdf section 12.9, Notes) |
| REQ-SOC-07 | 40 C | Claude's warm-room and pocket figure; the datasheet limit it is held against is not read here (VER-233) |
| REQ-SOC-09 | 2 mm | the Rationale: 'the 2 mm is Claude's proposal', no source on disk |
| REQ-SOC-10 | 3 failed boots | Claude's count: one failed boot can be a power cut, three in a row mean the image is bad; the Statement gives no source |
| REQ-SOC-11 | 16 KB | the Rationale: 'Claude's margin over Alex's a few KB' |
| REQ-USB-01 | 5.1 kohm | the Rd of a USB Type-C device (the Type-C specification, unread; the Source says 'to confirm the tolerance') |
| REQ-USB-02 | 2 mm | Claude's proposal 'from USB 2.0' (Rationale; the standard is unread) |
| REQ-USB-02 | 3.5 mm | Claude's proposal from USB 2.0 (Rationale; unread) |

## 6. The `count:` numbers

| Row | Value | Reason |
|---|---|---|
| REQ-BAT-05 | 100% | every cell, not a sample: a reversed cell can destroy the charger (Rationale) and a meter check costs seconds; no standard sets a sample size here |
| REQ-BAT-05.B | 100% | every cell, not a sample, because one reversed cell is a severity 10 failure and a meter check costs seconds a cell (Rationale, VER-49); no standard sets a sample size |
| REQ-BAT-06 | 1 spare cell | one cell per lot is destroyed measuring the trips; no standard sets it; more cells cost lot yield, fewer leave a lot unmeasured |
| REQ-BAT-10 | 3 of 3 units | three units of the 5-board EVT build, no standard sets it; fewer cannot show a spread between units, more costs EVT boards |
| REQ-BAT-11 | 2 documents | the UN38.3 test summary and the MSDS that the Statement names, one each; the UN Manual of Tests and Criteria Part III 38.3 (unread) is what asks for the test summary |
| REQ-BAT-14 | 10 plugs out of 10 | a plug is a repeated event and the first fail is the PCM tripping (Rationale); no standard sets the count; fewer cannot show a repeat, more costs cells |
| REQ-PWR-09 | 20 of 20 | twenty tries; no standard sets it; fewer cannot show an intermittent release, more costs bench time |
| REQ-SOC-01 | 20 of 20 | twenty power-ons; no standard sets it; fewer cannot show an intermittent boot failure, more costs bench time |
| REQ-SOC-05 | 0 mux conflicts | none allowed: a function on a pin without it is a respin (Rationale), so zero by definition, no sample |
| REQ-USB-01 | 20 of 20 | twenty plug-ins each way; no standard sets it; fewer cannot show an intermittent attach, more costs bench time |

## 7. Wrong cites found

Every `<file>.txt:<line>` cite in the 45 rows was read against the datasheet's `pdftotext -layout` text. One is wrong:

- REQ-PWR-04, Source: `er-tft035ips-6.txt:747-748` for the panel's VCI 2.5-3.3 V. Those lines are "-APPEARANCE INSPECTION" and "-PACKING SPECIFICTION". The values are at `er-tft035ips-6.txt:650` ("VCI High voltage power supply for analog circuit blocks (2.5~3.3V)") and `:703` ("Power Supply Voltage VCI 2.5 2.8 3.3 V"). The datasheet on disk is rev 2.0 (2026-10-29, datasheets/SOURCES.md); the cite predates it. The Source text is left as written; no Numbers entry relies on it.

The other `.txt:N` cites hold what the rows say: bq24074.txt:724 (VDO(BAT-OUT) 50/100 mV, the battery FET), bq24074.txt:823-826 (INTC, VHOT, VHYS(HOT), VCOLD), tctr0805.txt:260 (F=+-1% in the part-number key), mcp6001.txt:115-117 and :135-142, 2n7002_cj.txt:55 (Vth(GS) 1 / 1.6 / 2.5 V). Page cites (rp2350.pdf p.427-428, p.1358; tps3421.pdf p.5, p.11) were spot-checked only for the TPS3421's tTIMER and Table 2 and for the watchdog's LOAD register; they were not all re-read.

Two observations that are not bad cites:

- REQ-BAT-01's Source quotes the charger as "4.2 V +-1%"; the datasheet gives 4.16 / 4.20 / 4.23 V (bq24074.txt:773), so the +-1% (4.158 to 4.242 V) is wider than the part. It is tagged `judgment` with that comparison.
- REQ-SOC-02's Rationale calls the 100 KB free "Claude's proposal"; it equals 520 KB - 420 KB, so it is tagged `derived` (no more RAM than today is used).

## 8. REQ-PWR-14

Tagged on its current Statement (0.3 V, 80%, 90%, 10 us, all `judgment`), with each entry saying the Statement is about to change. The proposal in analysis/derating_rule_2026-10-30.md replaces the 0.3 V with the accuracy of the check and splits the percentages by part type. When the Statement changes, REQ-PWR-14's Numbers entries need redoing, and so does REQ-PWR-02's `0.3 V` entry, which is `derived` from REQ-PWR-14's (the checker will fail it if the number goes).

## 9. Left to tag

`python3 hardware/vv/tools/check_trace.py` prints the rest: live rows with a number in the Statement and no Numbers field, per row, and the total. Set `NUMBERS_REQUIRED = True` in tools/check_trace.py when it reaches 0. Next: the `judgment` rows above, one by one, against the real standard or datasheet (the web job).

Last updated: 2026-10-30 11:56 EDT
