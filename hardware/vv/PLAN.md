# The plan: from here to 5 boards, then 30

**The 5-board JLC order is the ESP32-S3 board at 39cc07f** (branch `ref-design-esp32s3`, Alex's
choice of 2026-10-28 over the RP2354B board): `hardware/ref_design` with an ESP32-S3-WROOM-1U-N16R8
module on an external flex antenna, the TLV75733P, the panel on its 8-bit 8080 bus, a microSD slot, a
third 74HC165, the power key SW24 and a case 1.5 mm wider on the left. On 2026-10-28 this register
was re-seeded for it: the baseline is 39cc07f (README), every row for a part or net that left is
RETIRED with that commit, the new parts have their requirements, DFMEA rows and checks, every row
of [carryover.md](carryover.md) has a requirement, a check and a verdict with its number, and every
scripted check was re-run against 39cc07f (evidence `evidence/2026-10-28_*`). The default mode
passes; the order gate fails, as it should (section i).

**Not ready to order.** Five things on the board itself need a decision before the copper is
frozen: the module's 22 uF 23.6 mm from its 3V3 pin and 3V3 at 2.976 V in a Wi-Fi burst at the
cutoff (DEF-82, DEF-86), the flex antenna 1.3 mm from the panel's outline (DEF-80), U4's thermal pad
3.9 mm against the module's 3.7 (DEF-84), the microSD slot 0.175 mm into the folded FPC's zone
(DEF-90), and the heat with Wi-Fi on paper (DEF-87). The rest are BOM values, accepted values and
bench checks (section d).

Contents: (a) the system on one page; (b) what changed in the design (71737ae to 39cc07f); (c) the
board at 39cc07f: the carry-over verdicts and what is new; (d) before the 5-board order; (e) the case
gate, EVT, DVT and PVT; (f) decisions for Alex; (h) what this plan still cannot catch; (i) the gate
today; (j) past bugs and what catches them; (k) the coverage grid. The RP2354B board's sections (b),
(c), (d), (f), (g), (i), (k) and (l) at fd1f6e8 are in [HISTORY.md](HISTORY.md), section (o).

## (a) The system on one page

**What it proves.** Each board works when it is built as designed (verified), the design
is the right one for the numbers (validated), it is safe (the Li-ion cell, touch
temperature), JLC can make it, it fits the case, and it survives use. It covers
`hardware/ref_design` and Alex's `hardware/prod_board` with `hardware/prod_case`, each with
its own evidence.

**The chain.** Requirements (requirements.md) -> DFMEA rows (dfmea.csv) -> checks
(verification.csv) -> gates (gates.md: ORDER, CASE, EVT, DVT, PVT) -> release and change
records (releases/) -> defects (defects.md). Alex's decisions live in one file,
releases/APPROVALS.md, which only he writes lines in, in commits signed with the key README
pins. The counts are in section (i).

**What check_trace.py enforces** (`python3 hardware/vv/tools/check_trace.py`):

- Every ID exists; every check is reached by a gate, a requirement or an H row no later than
  its own phase (O-1). A check that only ADVISORY criteria name is advisory and PHASE:<p>
  does not count it (VER-40).
- Every ORDER check is named by a numbered step row of section (d) below (the 'what moved
  where' table does not count: O-R11), so the checklist and the gate cannot drift apart (round
  2, O). A requirement has a live check no later than its Phase, so a
  check moved later cannot leave it behind (round 2, F).
- Every PASS or FAIL rests on a saved output under evidence/ with a COMMIT and RESULT
  header: TOOLS for an analysis or a test, WHO and ARTEFACT for an inspection or a demo
  (O-4, F-3, O-R3). Only VER-104 and VER-124 point at the design file their covers name.
  Evidence counts at a gate only when committed and unchanged since, and the gate fails while
  anything under hardware/vv is uncommitted (O-R2).
- Evidence goes stale when what it covers changes: a committed file (regexes as whole words),
  a part in design.py, a release of built units (stale only when a change record asks for a
  re-run), or its age. At DVT and PVT a check of phase EVT or later that is a test or covers a
  release must cover a release record of that phase (O-R11). A change record's 'What to
  re-run' must list every bench test tied to a part its commits changed (O-R21).
- A DFMEA row is DONE only when all its 'detects' checks pass; its detection rating cannot be
  better than they allow (2 or 1 needs two or more checks: round 2, F); an occurrence of 3 needs
  'Similar: <board and part>' (O-R8), 2 or less built hardware, 1 a stated exclusion.
- Approvals, acceptances, deferrals and waivers count only from lines in a commit signed with
  the one key README pins, whose pin line was itself committed signed by that key (round 2,
  O): never a merge, a Claude commit, an unsigned commit or another allowed key. A bulk
  'REQ-ALL@<commit>' or 'DF-H@<commit>' line covers what is unchanged since that commit, each
  ID checked on its own. An acceptance, a waiver and a deferral hold only up to their phase.
  A signed 'GATE@<commit>' line freezes every check's phase, defect's Gate, criterion's Needs
  and requirement's Phase and Verified by: moving one later, deleting it, retiring it or making
  it advisory fails the gate (G1) until a newer signed line (O-R5); until one is signed, the
  order gate lists every such move since README's 'Gate moves listed since' commit (G0, O-S5).
  At the order gate the
  pinned key must be a hardware (sk-) key unless the gate is run with --accept-software-key,
  which prints and goes into the release record (K1, O-R12).
- RETIRED: a requirement, risk, check, defect or gate criterion whose subject left the design
  keeps its ID and names the commit that removed it; no live row may lean on a retired check
  or cite a retired id outside 'Was: ...' (O-R9, F-R14).
- Each defect names its Gate; R4 holds it there and at every later gate, not before (O-R5).
  Since 2026-10-28 the one way to move a defect later without fixing it is Status ACCEPTED with
  Alex's signed `DF-ACCEPT:DEF-nn` line naming the gate it moves to and why: before that gate R4
  lists it and every criterion, G-ORD-22 included, lets through the checks it names, shown as
  ACCEPTED (not PASS) there and in grid.md. An unsigned ACCEPTED fails the default mode.
- A check's 'sources' names the datasheet files it relies on; a PASS whose sources are not in
  hardware/ref_design/datasheets is BLOCKED at every gate and in the grid (2026-10-28).
- A commit on the order branch after the baseline that touches docs/*proposal*.md warns in the
  default mode and fails R8 at the order gate (2026-10-28).
- The design baseline in README.md names every design path and must equal HEAD and the
  baseline commit's own blobs, or the default mode fails (DEF-47: on 2026-10-26 the rebase onto
  be437f8 failed it at once, as designed, and the rebases onto 71737ae on 2026-10-27 and onto 39cc07f on 2026-10-28 did the same;
  F-R3: a hash rewritten from HEAD fails too). README names the order branch (ref-design-esp32s3 since
  39cc07f): a design commit on it after the baseline warns in the default mode and
  fails the order gate (R8, O-R13). A case-only commit passes R8 at the order gate only when
  positions.json, regenerated by case/export.py at the branch's tip, hashes the same as at the
  baseline; both hashes print (O-R4).
- A CPL row off its pads passes only through a JLC_FIXED entry whose evidence file is committed
  and names the designator (F-R4).
- The coverage grid (section k, [grid.md](grid.md)): every live check names the classes and
  subsystems it targets; at the order gate R9 fails while a cell of classes 1-15 has no built
  check that runs before the boards exist (class 15: an analysis that has run), no NOT_NEEDED
  reason and no acceptance Alex signed. No live row may name a part, part
  number or net that design.py at the baseline no longer has, outside 'Was: ...' (VER-188).

`tools/seed_faults.py` seeds faults one at a time and catches every one (VER-129; the count is
in section i). `tools/design_numbers.py` recomputes the pre-order numbers this plan quotes; its
CPL and drill checks run fab.py's own export into a temp dir and read the files it writes
(O-R1). `tools/run_checks.py` runs it and the other quick checks and writes their evidence.

## (b) What changed in the design (71737ae to 39cc07f)

`git diff --stat 71737ae 39cc07f -- hardware/ref_design`: 108 files. What touches this register:

| Change | Where | Register effect |
|---|---|---|
| U4 the RP2354B, its flash, X1, L2 and the 1V1 parts (C15-C28, R19, R20, R25, R26) out; the ESP32-S3-WROOM-1U-N16R8 (C3013946) in, its antenna a flex plate on the case's left wall on a 40 mm U.FL cable | design.py, placement.py, lib (fp_esp32.py) | VER-15, VER-16, VER-23, VER-84, VER-187, VER-197 RETIRED; VER-203 (pins, strapping), VER-204 (supply, EN), VER-205 (antenna keep-out), VER-210, VER-214; REQ-MCU-01..11 rewritten, REQ-MCU-03 and -07 RETIRED, REQ-MCU-12..14, REQ-RF-01..04 new; DF-72, 77, 92-94, 139 RETIRED; DF-185, 186, 187, 192, 193, 197, 199, 200 new |
| U3 RT9013 to TLV75733PDBVR (C485517), the same SOT-23-5 pinout, 1 A | design.py | VER-12, 31, 103, 137, 138, 144, 195, 198 read tlv757p.pdf; REQ-PWR-04, -05, -15 restated; DF-31, 32, 155, 158 rewritten |
| The panel on its 8-bit 8080 bus (IM 011, R70-R78 33 ohm, CSX low, RDX high); the SPI nets and R28, R29 out | design.py PANEL | VER-21, 43, 60, 61, 199 rewritten; REQ-DSP-01, -02, -11 restated; DF-141, 180 RETIRED; DF-190, 191 new |
| A microSD slot J8 (Hirose DM3AT-SF-PEJM5, C114218), its pull-ups R79-R84, JP6, C60, C61; the left wall 1.5 mm out (grow_l) and an opening (sd_y) | design.py, storage.kicad_sch, case.scad | the grid's 'storage' subsystem; VER-206, VER-209 new; VER-165 reads the SD opening; REQ-STO-01..03 new; DF-194, 195, 196 new |
| A third 74HC165 U11 (the Starts, the Selects, PWR_BTN_N, the charger's status) and R86-R88; a 24-bit chain | design.py | VER-56, 198, 199 rewritten; REQ-BTN-09 new; DF-163 |
| The power key SW24 (ROCPU TPS850 J14K, C2759975) and its printed cap; Q7, D8, R62 out; both Starts plain buttons to GND | design.py, case.scad pwr_* | VER-33 rewritten for SW24 (PASS); VER-78, VER-79 RETIRED; VER-211, VER-213 new; REQ-PWR-08, -11 restated, REQ-PWR-18 new; DF-79, 166 RETIRED, DF-188 new |
| BOOT and RESET buttons (SW18, SW19) and their pinholes out; J5 (SWD) out; J7's pads 3V3, EN, BOOT, GND | design.py, case.scad pin_pos | VER-58, 107, 127 RETIRED; VER-212 new; REQ-PRD-08.R, REQ-MCU-04 restated; REQ-PWR-19 and DF-189 (J7's 3V3 back-feeds U3) new |
| USB R21/R22 27 to 22 ohm, 11.5 mm from the module (e20cfa6) | design.py, placement.py | VER-191 rewritten (FAIL on the distance); REQ-MCU-02 restated; DF-198 new |
| The cell's hole 37.8 x 54.8 to 38.2 x 55.2 mm (the 0.4 mm on its left side) | case.scad, placement.py | VER-181 PASS: DEF-53, DEF-58, DEF-62 CLOSED |
| The glass 1.1 mm higher (glass_gap 3.5); the case 78.9 x 156.48 x 10.70 mm | case.scad | REQ-PRD-06 restated (Alex's two decisions); VER-182 1.87 mm |
| U2 on TI's RGT0016C land (fp_u2.py) | lib | VER-125's U2 line ok (DEF-72 FIXED) |
| Audio filter R32, R33 1k to 1.5k for the LEDC's 312.5 kHz | design.py | VER-100 FAIL on 8.2 kHz (DEF-93) |
| check_pcb.py fails every non-cosmetic DRC type; the JLC marker on the back silk; the comb keep-out 2.9 mm across its flats | board/tools | DEF-36 FIXED, DEF-55 and DEF-74 CLOSED |
| placement_new.json (place_new.py's spots for the new parts), applied by placement.py | board/ | added to the design baseline (check_trace's BOARD_FILES) |
| 20 datasheets on disk (55be3cb's), plus the ESP32-S3's, the TLV757P's, the DM3's and the TPS850's | datasheets/ | the 9 PASSes BLOCKED at fd1f6e8 re-run on them (section i) |

What stayed: the charger, its TH1, the cell's connector J2, the panel and J3's map, the backlight
circuit, the amplifier, the speaker and J4, U1, U10, the latch's D2, D3, Q1, R13-R16, R42.

## (c) The board at 39cc07f: what the checks find

Every number below is from the evidence of 2026-10-28 at 39cc07f's design files.

**The 46 carry-over rows** ([carryover.md](carryover.md), with each row's requirement, check and
number): FIXED 16, STILL FAILING 25, NEEDS ALEX 5. The rows the board session marked met or changed
were checked against the evidence, not taken from its status file: C2, P8 and B2, which it had not
recomputed, are FIXED on the numbers (0.50/1.50 mm after tolerances; 47.3 uC on C1's DC-bias curve;
sure on to 5.0 kohm); B4, F3, F5, F8, F10 and F11, which it marked met, changed or 'see the final
report', still fail.

**What is new at 39cc07f** (each a defect with its gate):

| Finding | Numbers | Where |
|---|---|---|
| **3V3 sags under the module's 3.0 V in a Wi-Fi burst at the cutoff** | at VBAT 3.50 V and a TX burst (3V3 0.62 A): U2's FET 124 mV, VSYS's 0.24 mm track to U3 137 mV (221 mohm), U3's dropout 263 mV: 2.976 V against 3.0 V; C10 (the module's 22 uF) 23.6 mm from its pin | DEF-82, DEF-86, VER-204, VER-13 |
| **The flex antenna sits close to metal** | its plate 1.3 mm from the panel's outline, 10.6 mm from the cell, H2's screw boss over its end, against the guidelines' 15 mm (for the module's own antenna: the flex antenna's datasheet is not on disk); 0 copper in the board's keep-out | DEF-80, VER-205 |
| **No antenna or U.FL cable is bought, and the case does not hold it** | bom.md names neither; case.scad has no mount | DEF-85, VER-167 |
| **Heat with Wi-Fi on** | 3.31 W inside in the worst mode (2.0 W budget): the case's air 62.6 C at 40 C, the module's 84.9 C against its 65 C (N16R8), U3 163 C on JEDEC's figure (106 C on the EVM's), the cell 47.6 C at 25 C, U7 144.7 C on a stand-in theta | DEF-87, VER-144, VER-193, VER-200 |
| **U4's thermal pad 3.9 mm against the module's 3.7** | the 40 castellations match Figure 11-2 | DEF-84, VER-125 |
| **J8 under the folded FPC** | its body to x 78.225, the FPC's zone from 78.05; 1.675 mm tall against fpc_low 1.05 (an ESTIMATE) less 0.2 | DEF-90, VER-20 |
| **The SD opening has 0.075 mm a side** | the slot's 13.85 mm body in a 14.0 mm opening, against 0.2 mm of print and float | DEF-89, VER-165 |
| **C3 under its minimum at DC bias** | 3.11 uF at 4.2 V on its Samsung curve against BAT's 4.7 uF | DEF-83, VER-30 |
| **A card plugged in while running** | the rail's 21.4 uF against a card's own: over 1.8 uF takes 3V3 under 3.0 V; a 10 uF card (stand-in) to 2.21 V for about 38 us | DEF-91, VER-206 |
| **J2 at 1.24 A in a burst** | 3V3 0.62 A + the backlight 117.7 mA + the amplifier's 0.5 A against the PicoBlade's 1 A (a stand-in) | DEF-95, VER-31 |
| **u3.json is the RT9013's review** | VER-02 passes on nets; VER-36's identity rule fires; seven orphan review files | DEF-88, VER-36 |
| **J7's 3V3 pad back-feeds U3** | design.py's note invites powering 3V3 there with VSYS at 0 V: over U3's VIN + 0.3 V | DEF-92, VER-159 |
| **The audio bandwidth** | 9.7 kHz nominal, 8.2 kHz at the worst corner against 10 kHz | DEF-93, VER-100 |
| **USB's resistors 11.5 mm from the module** | 18.2 and 20.9 mm of track before them | DEF-94, VER-191 |
| **The new parts' stock and class unverified** | the module, the TLV75733P, the DM3AT, the TPS850; Konnect's JLC snapshot of 2026-07-13 has all four as designed, Extended, stock 18078, 30038, 9158 and 2979 (VER-186's evidence): order day still decides | DEF-81, VER-41 |
| **BL_PWM low leaves the backlight glowing** | U6's +4.5 mV offset holds 2.07 mA a sink, 4.13 mA in all (0.69 mA an LED) with BL_REF at 0 V; under VER-05's 5 mA, against pinmap.md's 'low = off' | DEF-96, VER-05 |
| **The smallest dome's rim near the ring's inner edge** | D 4.0 +0/-0.05 mm (the drawing): the rim 0.171 mm inside the ring at 0.1 mm of placement error against 0.2 | DEF-97, VER-178 |
| **design.py's maker for four capacitors** | C37, C38, C48, C49 (Fenghua 0402B...) named Samsung; JLC's BOM has no maker column, so the order is right | DEF-98, VER-186 |
| **The case grew** | 78.9 x 156.48 x 10.70 mm against REQ-PRD-06's 77.4 x 156.64 x 9.6 before: Alex's two decisions; the requirement restated | VER-190 |

Unchanged and still failing: the charge window 48.2 C (DEF-48), TH1's distance (DEF-37), the VBUS
reach 9.1 V (DEF-50), the CC and data fault reach (DEF-73), the screws 3.2 mm (DEF-31), the speaker's
plug (DEF-64), the domes not bought (DEF-42), the docs (DEF-21, DEF-79), the ESTIMATEs (DEF-78), the
shelf estimate (DEF-69), the ground under the fast nets and the USB pair (DEF-75), SR_CLK's limits in
pinmap.md (DEF-76).

## (d) Before the 5-board order

**What "classes 1-15 pass before the order" means** (2026-10-28, for every board this register
describes): each cell of classes 1-15 in grid.md is covered (R9: a check that can clear it has
run, or a NOT_NEEDED reason, or an accepted gap), and each ORDER check in it passes or is ACCEPTED
with a signed DF-ACCEPT line (G-ORD-22). A FAIL in the cell from a check of a later phase (an EVT
analysis such as VER-103) does not hold the order: its defect's Gate does. An accepted gap needs
both a grid_reasons.csv ACCEPTED_GAP row and Alex's signed GRID-<class>-<subsystem> line.

**How a decision to accept moves the gate** (2026-10-28): "Alex accepts X for EVT" means a signed
`DF-ACCEPT:DEF-nn` line with scope EVT and the defect's Status ACCEPTED. R4 then lists the defect
instead of failing on it, and the checks the defect names under 'Check added or changed' pass the
ORDER criteria as ACCEPTED. A check the defect does not name still fails. A DFMEA row's ACCEPTED
line (DF-nn) covers R3 only; a requirement's DEFERRED line covers R1 and R2 only.

The order gate holds only what the copper, the BOM and the fab files freeze, or what would make
the five boards useless or unsafe. Every ORDER check is named in a numbered step below (check_trace
fails otherwise). Effort is Claude's unless it says Alex. Step 1 lists the decisions in the order
they must be made, those that can move copper first.

| # | Step | IDs | Effort |
|---|---|---|---|
| 0.1 | **Alex confirms 39cc07f as the base** the register describes (DEC-01). If step 1 changes a design file, the order candidate is the commit after that pass (DEC-35), and the baseline moves to it. A later commit on ref-design-esp32s3 that touches a design path makes the default mode warn and the order gate fail (R8); the board session still commits renders and docs there. It already has: 589c6b5 and c8d5744 (C10 a 0603 22 uF 4.28 mm from the module's pin 2, the board re-routed with tracks 0.5 mm from the edge) and e872140 (the power key's pocket a cone, the SD opening 0.1 mm into the cavity), so R8 fails today; the next re-seed moves the baseline to the branch's tip and re-runs VER-204, VER-165, VER-20, VER-18 and VER-213. | DEF-30 | Alex: 5 min |
| 0.2 | **Order everything with a lead time** (`--shopping --gate evt`): cells from a seller with a spec sheet that gives the size with the PCM and names the plug (VER-37, VER-184); the ER-TFT035IPS-6, an FH12 breakout and an ESP32-S3 dev board (VER-43, VER-136, VER-44); the flex antenna and its U.FL cable (DEF-85); two GSPK151035PN and SH leads (DEF-64); domes and Start/Select strips; three microSD cards (VER-209); a 4-pin pogo jig (VER-212). **Download** into hardware/ref_design/datasheets: the antenna's datasheet (VER-205, VER-34), Samsung's CL21A226MAQNNNE DC-bias curve (VER-30), Espressif's ESP32-S3 errata (VER-214), JLC's dated capabilities page as jlc_capabilities.md (VER-17). JLC's parts database on order day for the four new parts (VER-41, DEF-81). The ESTIMATE ledger's measurements need calipers (VER-152). | VER-41, VER-37, VER-184, VER-43, VER-34, VER-152, VER-178 | Alex: 1 h, about $150-200 |
| 0.3 | **Run the register on every commit**: one line in scripts/check.sh, `python3 hardware/vv/tools/check_trace.py` (outside hardware/vv, so it waits for Alex's OK). | DEF-47 | 15 min |
| 0.4 | **Alex pins a signing key** (README, 'Approvals'). Until then no APPROVALS.md line counts (DEC-30). | | Alex: 15 min |
| 0.5 | **prod_board's own order list** (`--gate order --board prod_board`, Alex's board, evaluated apart). | VER-10, VER-119, VER-120, VER-121, VER-122, VER-163 | Alex: 1 h |
| 1.1 | **DEC-43, 3V3 in a Wi-Fi burst** (can move copper): C10 within 5 mm of the module's pin 2 and VSYS to U3 at 0.5 mm, or a firmware rule (TX power capped near the cutoff, or the cutoff at 3.6 V) accepted for EVT with VER-208. Proposed: the copper, if any other step-1 item re-routes; else the firmware rule. | VER-204, DEF-82, DEF-86 | Alex: 10 min; the copper a half-day pass |
| 1.2 | **DEC-40, the antenna's place** (can move copper and the case): accept the left wall for EVT with VER-207's range test, or move the module and its antenna. Proposed: accept for EVT, read the antenna's datasheet first. | VER-205, DEF-80 | Alex: 10 min |
| 1.3 | **DEC-47, U4's thermal pad**: read Figure 11-2 by eye; 3.7 mm in fp_esp32.py, or the paste windowed. | VER-125, DEF-84 | the design pass, 30 min |
| 1.4 | **DEC-48, J8 against the FPC and the SD opening**: J8 0.2 mm left (copper) or fpc_low measured on a real panel (VER-136); sd_y 0.4 mm wider (case only). | VER-152, DEF-90, DEF-89 | Alex: 5 min |
| 1.5 | **DEC-44, the heat with Wi-Fi**: a firmware heat rule (modem sleep, a lower CPU clock, the PSRAM's ECC for 85 C, CHG_OFF while hot) accepted for EVT with VER-86 measuring. Proposed: yes. | VER-144, VER-193, VER-200, DEF-87 | Alex: 5 min |
| 1.6 | **DEC-45, C3 at DC bias** (BOM): a second 10 uF or a 22 uF 0805 on VBAT, or accept for EVT (the cell sits across it). Proposed: the BOM change with any pass. | DEF-83 | Alex: 5 min |
| 1.7 | **DEC-42, the audio bandwidth** (BOM or a restated number): REQ-AUD-03 to 8 kHz, or the filter's values. Proposed: restate. | DEF-93 | Alex: 5 min |
| 1.8 | **DEC-46, USB's series resistors 11.5 mm from the module**: proposed accept for EVT (a 20 mm stub is 0.13 ns at full speed); under the pins at DVT. | VER-191, DEF-94 | Alex: 5 min |
| 1.9 | **DEC-05, the charge window's 48.2 C hot edge**. Proposed: accept DF-147 for EVT (attended charging), or restate REQ-BAT-03 to the cell's own range. | DEF-48 | Alex: 5 min |
| 1.10 | **DEC-17, REQ-PWR-17's accepted value**: 9.1 V (U3's EN 6.0 V absolute, tlv757p.txt:179), 20 V needs a clamp. Proposed: 9.1 V for EVT. | DEF-50 | Alex: 5 min |
| 1.11 | **DEC-09, the panel's VCI and VDDI at 3.333 V** (3.350 V at TJ 125 C). Proposed: accept. | DEF-61 | Alex: 5 min |
| 1.12 | **DEC-07, the cutoff**: the per-unit calibration at 3.50 V (REQ-TST-06) as the reference for the ADC's +-50 mV. Proposed: yes, by approving REQ-BAT-12. | DEF-40 | Alex: 5 min |
| 1.13 | **DEC-25, TH1's coupling** (1.95 mm from the cell): a 0.5 mm silicone pad (case side). | VER-164 | Alex: 5 min |
| 1.14 | **DEC-24, the pill margin with 47k pull-ups**: about 17.8 kohm on paper against 15; VER-45 on a bare board. Proposed: accept DF-164 for EVT. | DEF-49 | Alex: 5 min |
| 1.15 | **DEC-41, recovery with no BOOT or RESET button**: USB-C with the case closed, J7's pogo pads with it open (VER-212). Proposed: accept. | DEF-92 | Alex: 5 min |
| 1.16 | **DEC-35, the order candidate**: the commit after any step-1 pass; route.py --reuse for a rebuild that moves no copper. | DEF-30 | Alex: 5 min |
| 2 | **Re-run the scripted checks** after any step-1 pass (`run_checks.py`, about 15 min, then VER-129 on its own): netlist, pin gate, ERC, DRC, contact footprints, inrush, holes, MLCC, TH1, CPL, drill, backlight, cell gap, edges, net classes, the stale-names scan, the latch, the module's pins and supply, the antenna. | VER-01, VER-02, VER-03, VER-04, VER-08, VER-99, VER-114, VER-142, VER-164, VER-176, VER-131, VER-137, VER-181, VER-18, VER-201, VER-129, VER-188, VER-33, VER-203, VER-204, VER-205 | 30 min |
| 3 | **Fab outputs**: drill files and CPL (pass); no CPL row moves before the placement preview (J1-J4, J8, U4, SW24); the fab content check; JLC's dated limits, which also decide DEF-56; land patterns (U4's thermal pad: step 1.3; J8's and SW24's by hand); contacts as placed; the dome against its drawing (not built; the drawing is on disk); the release run (VER-143). The order sends no netlist; VER-26 and VER-29 bind at G-DVT-01. | VER-131, VER-176, VER-132, VER-17, VER-125, VER-162, VER-178, VER-143 | 1.5 days |
| 4 | **Layout reviews**: the USB pair's stubs and length (FAIL: 3.60 mm); the return paths (advisory: SD_D3 14.22 mm); the independent gerber view (advisory); the antenna's keep-out (step 2). | VER-108, VER-21, VER-27 | half a day |
| 5 | **Power-path analyses that can move copper or the BOM**: the latch table through SW24 (passes); the backlight (passes); inrush (passes). Alex reads by hand the nine back-power lines VER-159 lists (EVT), the jig on J7's 3V3 first. | VER-33, VER-137, VER-99 | 2 hours |
| 6 | **Datasheets and part identity**: the application circuits and every cited constant from the datasheets on disk (VER-198: FAIL only on the panel's VCI, step 1.11); the module's pins and strapping (VER-203: PASS); the hand audit of the parts the scripts do not read, u3.json rewritten (DEF-88); every line of JLC's matched-parts page by hand; the watchdog rule and the FM8002A's polarity (fm8002a.txt:75-76). | VER-34, VER-198, VER-203, VER-14, VER-104, VER-124 | 1 day |
| 7 | **The board against the case**: the XY chain; placement drift (with placement_new.json); check_fit; the MLCC keep-out; TH1; the ESTIMATE ledger (J8 under the FPC: step 1.4); the cell's gap (passes). | VER-174, VER-06, VER-19, VER-142, VER-164, VER-152, VER-181 | half a day plus calipers |
| 8 | **The panel and the cell, parts in hand**: the panel lit on its 8080 bus from an ESP32-S3 dev board, its map by diode and continuity tests, its RGB565 redraw timed at 25 MHz; the FPC folded into a breakout in J3's place; the tail's thickness; the cell's spec sheet and its plug against J2. | VER-43, VER-136, VER-44, VER-37, VER-184 | Alex and Claude: an evening each once parts arrive |
| 9 | **Test access**: every bring-up probe point has copper (TP1-TP16, JP1-JP6, J7's pads). | VER-158 | 2 hours |
| 9.5 | **The coverage grid's gaps** ([grid.md](grid.md), R9): each subsystem's requirements against its functions (VER-189, not built), the case's architecture (VER-190: FAIL on the screws only), the USB data path (VER-191), the USB fault reach (VER-195), the amplifier's states (VER-196), the module's supply (VER-204), the heat (VER-144, VER-193, VER-200), the clocked buses (VER-199). R9 also holds every cell whose only clearing checks are NOT_BUILT or BLOCKED: build or unblock them, or accept the gap (Claude's ACCEPTED_GAP row plus Alex's signed GRID line). | VER-189, VER-190, VER-191, VER-195, VER-196, VER-204, VER-144, VER-193, VER-199, VER-200 | 1 day; or Alex's acceptances: 30 min |
| 10 | **Reviews** (advisory here): the clean-room review if time allows; the DFMEA review binds at G-EVT-16 (VER-98). | VER-40 | Alex: half a day, optional |
| 11 | **Alex's decisions** in APPROVALS.md, in signed commits: one `REQ-ALL@<commit>` line; the H rows only built units can detect, accepted for ORDER (`--gate order --verbose` lists them); each step-1 acceptance as its defect's `DF-ACCEPT:DEF-nn` line with the defect's Status ACCEPTED; one `GATE@<commit>` line after reading G0's list of moves. | R1, R3, G1 | Alex: 1 h |
| 12 | **Release and order**: the release record; the dry run (valid 7 days); stock on order day (valid 1 day); `check_trace.py --gate order` exits 0; order 5 assembled and 5 bare boards, typing the Gerbers' extent (75.0 x 151.58 mm over the Edge.Cuts line; 73.9 x 151.48 without J1's tab), not bom.md's (DEF-79). | VER-42, VER-28, VER-41 | 1 day |

Rough total: about 5 working days of Claude's work, 3-4 hours of Alex's, a half-day design pass if
step 1.1, 1.3 or 1.4 changes copper, and up to three weeks of parts lead time in parallel.

**What moved where at this re-seed** (G0 lists them for Alex before he signs GATE@):

| Check | Was | Now | Why |
|---|---|---|---|
| VER-15, VER-197, VER-23, VER-187, VER-16, VER-84 | ORDER, EVT, DVT | RETIRED | the RP2354B's core and errata left; VER-203 and VER-204 are their successors at ORDER |
| VER-151, VER-150 | ORDER, EVT | RETIRED | the RP2354B's 2 MB and its image left; VER-216 and VER-215 at EVT (16 MB freezes no flash decision) |
| VER-58, VER-75, VER-78, VER-79, VER-107, VER-127 | EVT | RETIRED | SWD, picotool, the Starts' power path and the pinholes left; VER-212, VER-210, VER-211 at EVT |
| G-ORD-23 | ORDER (VER-151) | RETIRED | as VER-151 |

## (e) The case gate, EVT, DVT and PVT

### Case gate (before the printed case is released and the EVT units are assembled)

The Z stack (VER-20) with swelling (VER-182 passes: 0.80 mm under the panel), the buttons
under the shell (VER-07), no openings (VER-113), the screws per pilot (VER-147: the side
screws fail at 3.2 mm), the Start/Select strip against the domes (VER-180), the printed case
with real parts and its feel (VER-48, VER-50), incoming silicone, caps and domes (VER-126), the
case release (VER-173). Before any unit is assembled (G-CASE-06): the docs fixed (DEF-21's
list; VER-39 and the claims script VER-161), every contact class bought (VER-167), and the
opening steps changed so J2 is unplugged before the tub lifts (DF-169). Exit: `--gate case`
exits 0.

### EVT: the 5 assembled boards

Day one, before any unit is powered: bringup.md matches the design (VER-39 in G-EVT-02; today
it still describes the RP2354B board, DEF-21), the gear is on the bench (G-EVT-16), and pill resistance and comb leakage are measured
on a bare board from the same order (G-EVT-12). Then unit 1's first article inspection
(VER-166, VER-109) before any other unit is powered.

Per unit, in this order: unpowered resistances and diode tests (VER-52, VER-53: TP1 8.5-10
kohm from R38 with the R14/D3/R15 and R43/EN1 paths in parallel, TP3 above 1 kohm rules out
U2's BAT bridged to its pad); first power from a 100 mA bench supply with JP1 cut (VER-74);
the charge-path screen with USB in and no cell (VER-133: TP2 4.30-4.50 V, TP3 peak 4.25 V or
less, TP7 within `design_numbers.py tsband`'s band at the measured ambient); the cell current
with charging off (VER-169); CV into the TL431 load, 160 ohm in parallel with a 3.5 V TL431 shunt and 22 ohm
(VER-134: 4.158-4.242 V; `design_numbers.py cvload` shows no resistor alone can hold CV on
every unit, and this load does); TS by a finger and a 4.7k (VER-135). Only then a real cell,
attended, in a LiPo bag (VER-172).

Unit allocation (gates.md): unit 1 FAI, full bring-up, golden readings, the charge timer into
a load (VER-145, 6.5 h: the timer is 4-6 h) and SR_CLK (VER-179, now over U8, U9 and U11 on the 39cc07f board); units 2 and 3
bring-up, soak and thermal; unit 4 abuse, ESD screen, near-field, drop, button life and fault
injection (it never ships); unit 5 spare.

Exit: `--gate evt` exits 0 (G-EVT-01 to G-EVT-16; G-EVT-15 is prod_board's).

### DVT: the revised spin

Candidates, decided from EVT's results. BOM-only (each bumps BOARD_REV): C3's value (DEF-83, if
not done before the order); U10's part or its input's protection if VER-34 or VER-139
finds them short (DF-182, DEC-07); the BOM changes the plan asked for (R61, R14, R43, RN1-RN4)
and J4 keyed apart from J2 are in at 71737ae. Copper: a VBUS clamp for 20 V (DF-101, if
REQ-PWR-17 keeps 9.1 V for EVT); a bigger hole if incoming cells crowd the 55.2 x 38.2 mm
one (DF-85); J2's '+' if it is still under its label; C10 at the module's pin and VSYS wider if
DEC-43 took the firmware rule (DEF-82, DEF-86); the antenna moved if VER-207 fails (DEF-80);
reverse-polarity protection or a keyed connector on CELL_P (DF-55);
series-resistor footprints on the button nets and the Start domes (DF-100); a fuse in CELL_P
if accepted away for EVT (DF-172); the cell's hole or pocket if DEC-27 chose a stopgap; a
wider rim beside the speaker (DF-178); a cell with an NTC lead (DF-04). No resistor footprint
on TS can fix the charge window (O-R6), so DF-147 is TH1's B value or the cell's own limit.
Exit: `--gate dvt` exits 0: every test of phase EVT or later re-run on the DVT spin (its
result covers a REL-...-DVT record), durability, ESD with a real gun, every defect CLOSED or
RETIRED.

### PVT: the 30-board run

Exit: `--gate pvt` exits 0: a complete release record (tag, baseline, hashes, BOARD_REV,
locked BOM, the no-substitution list in releases/TEMPLATE.md, which now names the hand-fitted
panel, cell, speaker and domes too, the case section); FAI on the first PVT unit; a record per
unit; every cell inspected; first-pass yield 90% or more; every test re-run on the PVT units.
Before any unit is sold or given away: UN38.3 and MSDS, emissions, marking, the
fire-enclosure decision (G-PVT-06, DF-87).

## (f) Decisions for Alex

Each goes into releases/APPROVALS.md (requirements, acceptances, deferrals with a scope, waivers)
or into a commit. Claude's proposal follows each. Step 1 of section (d) lists the ones before the
order in the order to make them. The RP2354B board's list (DEC-01 to DEC-39 at fd1f6e8) is in
HISTORY.md section (o); the IDs it shares with this board keep their numbers.

**New at 39cc07f**

- **DEC-40. The antenna's place** (DEF-80, DF-192): the plate 1.3 mm from the panel's outline, 10.6 mm from the cell, H2's boss over its end. Proposed: accept for EVT; VER-207's range test on unit 1 decides; read the antenna's datasheet first.
- **DEC-41. Recovery without BOOT and RESET buttons** (REQ-PRD-08.R): USB-C with the case closed, J7's pogo pads with it open; an image that turns USB off needs the case open. Proposed: accept.
- **DEC-42. The audio bandwidth** (DEF-93): 8.2 kHz at the worst corner. Proposed: REQ-AUD-03 to 8 kHz.
- **DEC-43. 3V3 in a Wi-Fi burst** (DEF-82, DEF-86): 2.976 V at the cutoff. Proposed: the copper with any re-route; else TX power capped below a 3.6 V cell, for EVT with VER-208.
- **DEC-44. The heat with Wi-Fi** (DEF-87): 3.31 W inside, the module's air over 65 C on paper. Proposed: a firmware heat rule for EVT, VER-86 measuring; the PSRAM's ECC if the module's air is over 65 C on unit 2.
- **DEC-45. C3 at DC bias** (DEF-83): 3.11 uF against 4.7. Proposed: a 22 uF 0805 or a second 10 uF on VBAT.
- **DEC-46. USB's series resistors' place** (DEF-94). Proposed: accept for EVT.
- **DEC-47. U4's thermal pad** (DEF-84): 3.9 against 3.7 mm. Proposed: 3.7 mm in fp_esp32.py if the drawing says so.
- **DEC-48. J8 and the FPC, J8 and the SD opening** (DEF-90, DEF-89). Proposed: sd_y 0.4 mm wider now (case); J8 0.2 mm left only if VER-136 shows the fold reaches it.

**Carried from the RP2354B board, restated at 39cc07f**

- **DEC-01. Is 39cc07f the base the order is made from?** Proposed: yes (DEC-35 for any later pass).
- **DEC-03. Will any of the 30 be sold or given away?** That decides the SALE items, now with the radio's grant (VER-217).
- **DEC-05. The charge window** (DEF-48): 3.9-48.2 C. Proposed: accept DF-147 for EVT.
- **DEC-07. The cutoff** (DEF-40): the per-unit calibration at 3.50 V. Proposed: yes.
- **DEC-09. The panel's VCI and VDDI at 3.333 V** (DEF-61). Proposed: accept.
- **DEC-13. The display bus**: taken in the design (the 8080 bus); Alex sets REQ-DSP-11's numbers (20 ms a frame, 5 ms a text update proposed).
- **DEC-17. The VBUS reach** (DEF-50): 9.1 V. Proposed: accept for EVT; a clamp at DVT.
- **DEC-18. Screws** (DEF-31): M2 x 7 or 8 in the three side pilots.
- **DEC-22. A fuse in the cell path** (DF-172): the PCM for EVT; a fuse at DVT.
- **DEC-24. The pill margin** (DEF-49): accept DF-164 for EVT.
- **DEC-25. TH1's coupling** (DEF-37): a 0.5 mm silicone pad.
- **DEC-30. A pinned signing key**; **DEC-34. Freeze the gate state** (GATE@); **DEC-35. The order candidate after a design pass**.
- **DEC-37. Datasheets still missing**: the flex antenna's, Samsung's CL21A226MAQNNNE curve, the ESP32-S3 errata, JLC's capabilities page (step 0.2).
- **DEC-04, DEC-08, DEC-12, DEC-14, DEC-15, DEC-16, DEC-19, DEC-21, DEC-26, DEC-38, DEC-39**: as HISTORY.md section (o) states them; DEC-14's flash budget is REQ-MCU-10's partition rule now, DEC-38 (REQ-BAT-08's fit clause) is met by the 55.2 x 38.2 mm hole.

## (h) What this plan still cannot catch

- **Small numbers.** Five boards, then 30, cannot show a failure rate under a few percent.
- **Time.** Cell ageing and swelling over years, ENIG wear, tape creep, slow flux migration.
- **Faults nobody listed.** The DFMEA names every part, but only the failure modes someone
  thought of; seeded faults prove a checker fails on the faults seeded.
- **Numbers this register quotes from memory.** The charger's, the ILI9488's, the ESP32-S3's and the TLV757P's
  come from the files in hardware/ref_design/datasheets (committed with SOURCES.md since
  71737ae); JLC's 0.30 mm and 0.2 mm tolerances (VER-17) and the cell's size and PCM (VER-37)
  still do not, and the flex antenna has no datasheet on disk (DEC-37). The input leakage
  in design_numbers.py is a 1 uA stand-in.
- **The panel's mechanics.** case.scad takes the outline, neck and border from the drawing
  since 71737ae, but the FPC's folded shape and fpc_low are still estimates: only a real panel
  (VER-43, VER-136, VER-152) can show J3's place is right.
- **The cell's fit.** The pocket's walls hold the cell off every edge since 71737ae, but only a
  cell under its spec sheet's maximum fits without pressing (DEF-62); a seller's spec sheet is
  a claim until VER-49 measures the lot.
- **The bought breakout's header numbering.** The product's panel-side pairing is 51-j, and
  the fold changes no contact pairing, but the loose breakout's own header numbering against
  the FH12's pads is unknown until VER-43's diode and continuity tests (F-R1).
- **A board the scripts cannot rebuild the same way twice.** The board's copper came from route.py's
  search; route.py's Freerouting race picks its winner by
  timing, so only route.py --reuse with the committed last_route.ses can give the same copper
  (O-S2). Until VER-143 shows two --reuse rebuilds agree, a rebuild is a new board.
- **A gate state nobody has signed.** The register's own phases are in force until Alex
  signs a `GATE@<commit>` line; anything moved later without a newer signed line fails the
  order gate (G1), so a phase cannot be moved quietly.
- **A key that a process can use.** A pinned software key is only as safe as its passphrase
  habit (DEC-30). The order gate now fails on one, unless it is run with
  `--accept-software-key`, which prints in the gate result and has to be copied into the
  release record's signing section (K1). A process running as alex could also pass that flag:
  the release record is where Alex sees it.
- **JLC's placement.** fab.py writes the pads' centre since 40281b8; whether JLC's own library
  origins for J1-J4, J8, U4 and SW24 agree, only the dry run's preview shows (O-R7).
- **The antenna's range.** No paper check gives the flex antenna's range in the closed case
  beside the panel and the cell; VER-207 on unit 1 does (DEF-80).
- **Bench honesty, standards, JLC's side, firmware beyond the self-test, use**: as the first
  run listed them.
- **A design that moves again.** Another session may commit to ref-design-esp32s3. Any change to a
  design path fails the default mode and every gate until its rows are re-checked; the
  register cannot re-check them itself.
- **Quoted values until VER-161 exists.** A GPIO, a count or a dimension quoted in a row can
  still drift from design.py unseen (DEF-54).

## (i) The gate today

**Run at c564e99 (2026-10-28)**, the re-seed for 39cc07f. Every scripted check ran against
39cc07f's files (evidence `evidence/2026-10-28_*`), the 9 PASSes BLOCKED at fd1f6e8 among them: with
the datasheets on disk none is BLOCKED now. VER-33, VER-137, VER-124 and VER-196 pass; VER-31, VER-100,
VER-144, VER-191 and VER-193 fail on their numbers (section c). Requirements 146 (141 PROPOSED, 5
RETIRED); DFMEA rows 201 (66 H, 94 M, 41 L); checks 216 (31 PASS, 33 FAIL, 51 NOT_RUN, 79 NOT_BUILT,
5 CUT, 17 RETIRED); defects 95 (43 OPEN, 23 FIXED, 16 CLOSED, 13 RETIRED); seeded faults 117 of 117
caught (VER-129 at 0eabbd4, nine seeds moved to live rows). VER-36 now counts A-numbers from
APPROVALS.md's table only (0 counted of 0 rows), not the README's example.

```
$ python3 hardware/vv/tools/check_trace.py
STRUCTURE: OK (5 warnings: four DMM pass bands under 4 x the meter's accuracy; R8, the order
branch has design commits after 39cc07f: e872140, 589c6b5, c8d5744)
exit 0

$ python3 hardware/vv/tools/check_trace.py --gate order
  PASS  G-ORD-06, G-ORD-15, G-ORD-18, G-ORD-24, R5, R6 (no release yet), R7
  INFO  G-ORD-05, -08, -12, -20, -26 (advisory); G0: 47 moves since fd1f6e8 (the 14 RETIRED
        checks, 6 RETIRED and 6 FIXED or CLOSED defects, the gate criteria and requirements that
        no longer name them, REQ-MCU-10 to EVT)
  RETIRED G-ORD-13, G-ORD-23; N/A G-ORD-25 (prod_board)
  FAIL  G-ORD-01 to -04, -07, -09 to -11, -14, -16, -17, -19, -21, -27
  FAIL  G-ORD-22: 29 ORDER checks not passing
  FAIL  R1 138 PROPOSED; R2 28 requirements with an ORDER check not passing; R3 52 H rows
        neither accepted nor DONE
  FAIL  R4 24 OPEN defects due at ORDER: DEF-14, 30, 47, 48, 49, 50, 56, 61, 66, 68, 75, 78,
        79, 80, 81, 82, 83, 84, 86, 87, 88, 90, 93, 94; 12 FIXED ones listed beside it
  FAIL  R8 ref-design-esp32s3 has design commits after 39cc07f (e872140, 589c6b5, c8d5744);
        positions.json moved (case commit e872140)
  FAIL  R9 12 cells planned: class 1 for all nine subsystems (VER-189), 2/audio (VER-34,
        VER-40), 2/cell (VER-37, VER-40), 5/case (VER-192)
RESULT: FAIL (21 of 28 not met); 1 N/A, 6 advisory, 2 retired
```

R8's proposal warning for dcb6a76 and e79f251 is gone: both are ancestors of 39cc07f. R8 fails
instead on three commits the board session made on the order branch after 39cc07f (step 0.1):
they move C10 beside the module's pin and re-route the board, which is DEF-86's fix, so the next
re-seed starts from the branch's tip.

The coverage grid, classes 1-15 by the worst status in each cell (135 cells, nine subsystems):
PASS 10, FAIL 93, NOT_RUN 10, NOT_BUILT 1, ACCEPTED 0, BLOCKED 0, planned 12, not needed 9. No
defect is ACCEPTED and G1 and K1 print nothing: no key is pinned, so no line in APPROVALS.md can
count yet.

The RP2354B board's gate history (fd1f6e8 and before) is in HISTORY.md section (o).

## (j) Past bugs and what catches them

| Defect | What happened | Caught now by | When |
|---|---|---|---|
| DEF-01 | U1's REF on VBUS back-fed PWR_EN | VER-159 | before the order |
| DEF-02 | R15 at 10k cut the pill margin | VER-33 | before the order |
| DEF-03-06, 09 | the TP4056 power path's hand-over, hot plug and CE limits | RETIRED with those parts (2d1124d, 1fd0956); the BQ24073's are VER-12, VER-70-72 | EVT |
| DEF-07, 08 | J6 swapped; 22 ohm USB resistors | VER-02 (CLOSED) | every change |
| DEF-10 | no PWR_HOLD pull-down | RETIRED in 39cc07f with the old latch | |
| DEF-11 | R5 kept from the bigger cell | VER-160, VER-37, VER-66 | before the order |
| DEF-12, 13, 29 | parts out of stock | VER-41 (valid 1 day), VER-14 | order day |
| DEF-14 | U1's JLC rotation | VER-29, VER-28 | before the order |
| DEF-15-19, 25 | case fits and openings | VER-48, VER-92, VER-126, VER-07, VER-113, VER-20 | case gate |
| DEF-21 | docs disagree with the design (bringup.md's display and power steps among them) | VER-39 (FAIL), VER-161 | case gate; bringup.md before first power |
| DEF-26-28 | the pin gate's blind spots; guessed heights; analyses not in the repo | VER-36, VER-112, design_numbers.py | before the order |
| DEF-30, 47 | the order candidate not recorded; the register drifted | VER-42, the baseline rule, VER-129 | every change once check.sh runs it |
| DEF-31 | screws engage 3.2 mm | VER-147 (FAIL, per pilot) | case gate |
| DEF-32, 33 | the drill file; the CPL offsets | VER-131, VER-176 (both PASS at 71737ae; DEF-32 CLOSED, DEF-33 FIXED until VER-28's preview) | before the order |
| DEF-34 | no firmware for the ordered chip | VER-149, VER-215 (EVT) | EVT |
| DEF-35, 52 | the trace gate's own holes | VER-129 | every change |
| DEF-36 | check_pcb passes non-silk warnings, and called the routed-away BOARD_REV cosmetic | VER-04, VER-22, VER-18 | before the order |
| DEF-37, 48 | TH1's coupling; the 47 C hot edge | VER-164, VER-87, VER-32 | before the order and EVT |
| DEF-39 | backlight over 120 mA: closed with the 2.4 in panel, back with the 3.5 in | VER-137 (PASS at 71737ae, CLOSED) | before the order |
| DEF-40 | the cutoff may never fire | VER-138 (FAIL), VER-139, VER-155, VER-156 | before the order |
| DEF-41, 42 | assembly next to a live cell; Start/Select parts not bought | VER-91, VER-167 (FAIL) | case gate |
| DEF-43, 51 | inrush; swelling room | VER-99, VER-182 (both CLOSED) | every change |
| DEF-45 | the backlight sim does not read design.py | VER-05 (paper_checks.py backlight since 2026-10-28: reads design.py; PASS; CLOSED) | every change |
| DEF-96-98 | the backlight's glow at BL_PWM low; the dome's rim margin; four capacitors' maker field | VER-05, VER-178 (FAIL), VER-186 (FAIL) | before the order (DEF-97), EVT |
| DEF-46 | bench cell emulation | VER-70, VER-67 | EVT |
| DEF-49, 50 | the pill margin; the VBUS fault reach | VER-45, VER-177 (FAIL at 9.1 V; both OPEN: the BOM half in, the decisions Alex's) | before the order |
| DEF-53, 62 | a worst-case cell 0.25 mm from the FR4 edge; then a pocket with no fit | VER-181 (PASS at 39cc07f; both CLOSED) | before the order |
| DEF-54 | rows still written for an older design after a re-seed | VER-161 (not built), the retired-citation rule | case gate |
| DEF-55, 57, 66 | JLC's marker not on the board; the board's name and BOARD_REV in the cell's hole; J2's '+' over its label | VER-132 (k), VER-18 (PASS), VER-04's warnings read by hand | before the order |
| DEF-56 | copper 0.20 mm from the routed holes; then 0.108 mm as built | VER-18 (PASS in the data), VER-17 (JLC's dated page; reopened, O-R2) | before the order |
| DEF-63 | U10 added without its datasheet | VER-34, VER-198 (mcp6001.pdf on disk since 39cc07f) | before the order |
| DEF-64 | the speaker's plug no longer fits J4 | VER-202 (FAIL), VER-128 | case gate |
| DEF-68 | CELL_P and AMP_VDD in no net class | VER-13, VER-201 (both FAIL) | before the order |
| DEF-79 | bom.md's order line 0.16 mm long and its cost table one part number short | VER-39, VER-130 | before the order |
| DEF-65 | the swelling check used the old decks' line | VER-182 (reads uni_ceil; CLOSED) | every change |
| DEF-80-95 | the ESP32-S3 board's new findings (section c) | VER-204, VER-205, VER-206, VER-144, VER-193, VER-125 and others | before the order, case gate, EVT |

## (k) The coverage grid

[grid.md](grid.md) is generated by tools/grid.py from verification.csv and grid_reasons.csv: classes 1-18
by nine subsystems ('storage' added at 39cc07f for the microSD slot, its bus, its supply and its
opening), each cell COVERED with its checks and worst status, planned, GAP or NOT_NEEDED. At the
order gate R9 holds every cell of classes 1-15 that no built pre-order check has cleared. The counts
of 2026-10-28 are in section (i); the earlier grid's audit and its history are HISTORY.md's.

Last updated: 2026-10-28 13:30 EDT
