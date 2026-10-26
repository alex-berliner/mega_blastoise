# The plan: from here to 5 boards, then 30

**Not ready to order.** The board being ordered is `hardware/ref_design` at **684a609**
(branch ref-design-island-45, 2026-10-24 12:49). On 2026-10-25 this register was re-seeded
for it: the first run (2026-10-24) had described 76955d2, a design two days older (DEF-47).
The same day two models attacked the re-seed; their 39 accepted findings (O-R1 to O-R23,
F-R1 to F-R16, none disputed) are applied (section l). The order gate fails 27 of its 31 counted
criteria (section i); the default mode passes.

Contents: (a) the system on one page; (b) what changed in the design; (c) the first run's
pre-order findings, re-checked; (d) before the 5-board order; (e) the case gate, EVT, DVT and
PVT; (f) decisions for Alex; (g) the first run's sign-off objections and where each landed;
(h) what this plan still cannot catch; (i) the gate today; (j) past bugs and what catches
them; (k) the ID ledger of the first run's debate; (l) the second review's ID ledger.

## (a) The system on one page

**What it proves.** Each board works when it is built as designed (verified), the design
is the right one for the numbers (validated), it is safe (the Li-ion cell, touch
temperature), JLC can make it, it fits the case, and it survives use. It covers
`hardware/ref_design` and Alex's `hardware/prod_board` with `hardware/prod_case`, each with
its own evidence.

**The chain.** Requirements (requirements.md: 133, of which 130 live and PROPOSED, 3
RETIRED) -> DFMEA rows (dfmea.csv: 177, of which 145 live: 53 H, 65 M, 27 L; 32 RETIRED) ->
checks (verification.csv: 182: 14 PASS, 9 FAIL, 52 NOT_RUN, 102
NOT_BUILT, 2 CUT, 3 RETIRED) -> gates (gates.md: ORDER, CASE, EVT, DVT, PVT) ->
release and change records (releases/) -> defects (defects.md: 55: 22 OPEN, 20 FIXED, 6 CLOSED, 7 RETIRED). Alex's decisions
live in one file, releases/APPROVALS.md, which only he edits, in commits signed with his key.

**What check_trace.py enforces** (`python3 hardware/vv/tools/check_trace.py`):

- Every ID exists; every check is reached by a gate, a requirement or an H row no later than
  its own phase (O-1). A check that only ADVISORY criteria name is advisory and PHASE:<p>
  does not count it (VER-40, F's objection).
- Every PASS or FAIL rests on a saved output under evidence/ with a COMMIT and RESULT
  header: TOOLS for an analysis or a test, WHO and ARTEFACT for an inspection or a demo
  (O-4, F-3, O-R3). Only VER-104 and VER-124 point at the design file their covers name.
  Evidence counts at a gate only when committed and unchanged since, and the gate fails while
  anything under hardware/vv is uncommitted (O-R2).
- Evidence goes stale when what it covers changes: a committed file (regexes as whole words),
  a part in design.py, a release of built units (stale only when a change record asks for a
  re-run), or its age. A 'covers' glob matches committed files only (F's objection). At DVT
  and PVT a check of phase EVT or later that is a test or covers a release, whatever its
  method, must cover a release record of that phase (O's objection, O-R11). A change record's
  'What to re-run' must list every bench test tied to a part its commits changed (O-R21).
- A DFMEA row is DONE only when all its 'detects' checks pass; its detection rating cannot be
  better than they allow; an occurrence of 3 needs 'Similar: <board and part>' (O-R8), 2 or
  less built hardware, 1 a stated exclusion.
- Approvals, acceptances, deferrals and waivers count only from lines in a commit signed with
  Alex's key, never a merge or a Claude commit (O-R4); until signing is set up every line is
  pending. An acceptance, a waiver and a deferral hold only up to the phase they name; an
  approved statement edited later needs approving again.
- RETIRED: a requirement, risk, check or defect whose subject left the design keeps its ID and
  names the commit that removed it; no live row may lean on a retired check or cite a retired
  id outside 'Was: ...' (O-R9, F-R14).
- Each defect names its Gate; R4 holds it there and at every later gate, not before (O-R5).
- The design baseline in README.md names every design path (design.py, placement.py,
  board/lib, battler.kicad_pcb, battler.kicad_pro, the schematics, the board/tools tree,
  case.scad, case/export.py: F-R13) and must equal HEAD, or the default mode fails (DEF-47).
  After a release, a commit touching any of them needs a change record (R6). README names the
  order branch: a design commit on it after 684a609 warns in the default mode and fails the
  order gate (R8, O-R13), even before it is merged here.

`tools/seed_faults.py` seeds 63 faults and catches every one (VER-129): 41 on a copy of the
register, 22 in a scratch clone with real git history and signed commits, two of those
design_numbers.py runs (O-R1, O-R14).
`tools/design_numbers.py` recomputes the pre-order numbers this plan quotes; its CPL and
drill checks run fab.py's own export into a temp dir and read the files it writes (O-R1).
`tools/run_checks.py` runs it and the other quick checks and writes their evidence.

## (b) What changed in the design (76955d2 to 684a609)

`git diff 76955d2 684a609 -- hardware/ref_design`: 153 files. What touches this register:

| Change | Commit | Register effect |
|---|---|---|
| TP4056 and its load sharing (D5, D6, D7, Q2, Q4, R6, R39, R40, R47, C9) became a BQ24073 power path (C15220, VQFN-16): OVP 6.6 V, OUT 4.4 V, USB500 475 mA, 270 mA charge (242-295), 5 h timer, TS on TH1; R64 on ILIM; R38 1k became 10k | 2d1124d | DF-01-03, 05, 10-19, 34, 37, 38, 40, 41, 99, 104-106, 110, 111 RETIRED; DF-144-153 new; VER-11 RETIRED; DEF-03-06, 38 RETIRED; DEF-43 CLOSED; REQ-PWR-06 RETIRED |
| ME6211 became the RT9013-33GB (C47773) | 1fd0956 | DF-29, 30 RETIRED; DF-155, 157, 158 new; DEF-09 RETIRED; REQ-PWR-03 RETIRED |
| ER-TFT035IPS-6 (3.5in, ILI9488) became the ER-TFT024IPS-3 (2.4in, ST7789V, 4 LEDs, 80 mA); J3's map; R61 3.9k to 2.4k | 53e21c8 | DF-120 RETIRED; DF-159 new; REQ-DSP-01, 02, 05, 11, 12 rewritten; DEF-39 CLOSED |
| D-pads and A/B/X/Y through two chained 74HC165s (U8, U9, RN1-RN4, R65, C52, C53) | 2d1124d | DF-160-165 new; DEF-49 (the pill margin) |
| Start and Select on 4 mm metal domes (SW20-SW23 on Dome_4mm_Round); seat 2's Start also powers on (Q7, D8, R62, R63) | 2d1124d | DF-124 RETIRED; DF-166-168 new; VER-09 RETIRED; VER-178, VER-180 new; REQ-BTN-08 new |
| The cell in a 54.0 x 37.0 mm hole in the board; TH1 at the hole's right edge; the tub pocketed under the cell | 2d1124d and the case after it | DF-04, 84, 85, 108 rewritten; DF-169, 171 new; DEF-37 rewritten; DEF-51 new |
| The speaker on a PicoBlade J4 (no soldering), on the board's top in the right bezel | 7da6a90, 0c1e3ac | DF-112 RETIRED; DF-170 new |
| J6 (UART), the revision strap R48-R51 and C41 removed; debug buttons and latch parts moved; GPIO map re-picked (routing levers) | 86bf694 | DF-63, 140 RETIRED; VER-59 RETIRED; REQ-MCU-05 RETIRED; DEF-20 RETIRED; DF-122 rewritten |
| The case: 77.4 x 132.7 x 9.55 mm (7.7 at the decks); the island, two decks, the tub; six M2 x 6 screws; arm hooks | case.scad since 2026-10-21 | REQ-PRD-06, MEC-05, 09, 10 rewritten; DEF-31 rewritten |

## (c) The first run's pre-order findings, re-checked at 684a609

Every number below comes from `tools/design_numbers.py` at 4a86173 (evidence in
`evidence/2026-10-25_*`) unless it says otherwise.

| Finding (2026-10-24) | At 684a609 | Check |
|---|---|---|
| The drill file mixes plated and unplated holes | Still true: one MixedPlating file; the unplated holes are J1's two 0.65 mm pegs, SW20-SW23's four 0.5 mm holes and H1-H6 (2.2 mm). Every hole is 0.30 mm or more. VER-131 now parses the drill files fab.py's own export writes (O-R1); J1's four 0.60 mm plated slots are on its list | VER-131 FAIL, VER-114 PASS, DEF-32 |
| J1 and J2 off their pads in the CPL (1.46 and 0.60 mm) | Still true, measured against the pads' centre: J1 1.50 mm, J2 0.95, J4 0.95 (the same footprint), J3 0.23. VER-176 now reads the CPL fab.py writes, row by row (O-R1) | VER-176 FAIL, DEF-33 |
| C9 0.9 mm from a screw boss | C9 is gone (2d1124d). The nearest MLCC is C14 (100n 0402 on 3V3), its pad 4.40 mm from H4's centre; the nearest bulk cap C34, 9.16 mm from H3 | VER-142 PASS, DEF-38 RETIRED |
| TH1 at the cell's edge, not its centre | TH1 sits 0.80 mm outside the hole's right edge by design, facing the cell's end across the FR4 edge and an air gap (0-0.5 mm when the cell is pushed home). The geometry passes; the lag is unmeasured. New (O-R7): the pushed cell's end is 0.25 mm from the routed FR4 edge, -0.15 mm after routing, print and screw-float tolerances, against REQ-BAT-08's 0.5 mm | VER-164 PASS, VER-181 FAIL, VER-87 not built, DF-04 H, DF-85 H, DEF-37, DEF-53 |
| The 3.50 V cutoff may never fire | Still true with the RT9013: with 3V3 +2%, 120 mV of dropout at 150 mA and a -1% CV, the lowest reading in dropout is 3.521 V. In regulation the CV calibration alone puts the cutoff at a true 3.465-3.535 V. New (O-R15): GPIO47's pad leakage through 235 kohm moves VBAT's reading 0.47 V per uA; REQ-BAT-12's window allows about 53 nA that changes with temperature (the 1 uA maximum is quoted from memory until VER-34) | VER-138 FAIL, DEF-40 |
| Backlight 122.5 mA against 120 mA | Fixed by the panel change: 78.0 mA worst case against the new panel's 80 mA (39.0 mA a sink). New: two LEDs share each sink, so one LED's share is not controlled | VER-137 PASS, DEF-39 CLOSED, DF-159 |
| Screws engage 1.4 mm | Now M2 x 6: 3.2 mm of thread (1.6 x d) in the island's 4.95 mm pilots and the decks' 3.30 mm. bom.md still buys M2 x 8, which would run past the decks' pilots | VER-147 FAIL, DEF-31 |
| No firmware pin map for this board | pinmap.md is generated from design.py now, but no firmware is: the fw crate is rp235xa, 4096K, ST7796, and fw-ref-board (a01f6fb) predates the routing levers: PWR_HOLD on GPIO2 (P2_START here), SR_PL on GPIO28 (BL_PWM here) | VER-149-151 not built, DEF-34 |
| Start/Select pills and caps not in the BOM | Start/Select are metal domes now; bom.md still says the Start/Select strip is not used and buys no domes, Kapton dots or Start/Select caps | VER-167 FAIL, DEF-42 |
| Inrush 265 uC against 50 uC (a written deviation) | Gone: VBUS carries C1 alone (10 uF, 50 uC at 5.0 V) and U2's input switch is the only path to VSYS | VER-99 PASS, DEF-43 CLOSED |
| Charge window up to 49 C (R39/R40) | Now 3.5-47.2 C nominal from the BQ24073's fixed TS thresholds. No series or parallel resistor fixes both edges (O-R6); a 10k NTC with B near 4250 on the same pads gives about 4.9-45.5 C | VER-32 not built, DF-147 H, DEF-48 |
| 3V3 under 3.135 V through the unplug (Q2, D7) | Gone: no hand-over gap. 3V3 is about 3.33 V at a 3.50 V cell at worst case | REQ-PWR-05 rewritten, VER-70 |
| No OVP on VBUS | U2 has OVP (6.6 V, 28 V absolute), but PWR_EN through R14/D3 takes U3's EN over its 6 V above VBUS 6.9 V, and EN1 over its 7 V above 9.5 V | VER-177 FAIL, DEF-50 |
| Cell swelling: 0.5 mm gap holding a 0.45 mm foam pad | No foam: a 4.5 mm cell has 0.30 mm free under the deck, 0.80 mm under the panel | VER-182 FAIL (split from VER-20, O-R21), DEF-51 |
| U1's JLC rotation in doubt | assembly.md now states JLC's footprint is KiCad's turned 180 degrees; still to see in the dry run | VER-28, DEF-14 |
| TP11-TP14 facing the pouch | Gone: no back-side pad within 1 mm of the hole: the nearest is TP11 at 6.2 mm, now read from all 11 flipped footprints on the board, not placement.py's text (O-R14) | VER-164 PASS |
| E9 pull-downs on AMP_EN and BL_PWM; GPIO47 above IOVDD when off; legacy USB hosts; reversed cell | Still true (R37 100k; R60 + R61 102.4k; R17/R18 470k/470k; charging starts before enumeration; no polarity protection) | DF-45, 47, 95, 66, 55 |
| Docs disagree with the design | 26 stale statements in 8 files, mostly the TP4056 and the old latch in bringup.md, assembly.md and bom.md; the second review added bom.md's board size and foam pad and u4.json's old GPIO numbers | VER-39 FAIL, DEF-21 |

New at 684a609: the pill margin dropped from 10 kohm to about 3.7 kohm with the 10k pull-ups
(DEF-49, DF-164); the backlight simulation runs for the old panel (DEF-45); a fixed 10k at
TH1 would silently disable the temperature pause (DF-146); opening the case pulls the cell
on its leads (DF-169); J4 takes the cell's plug (DF-170); the strips beside the hole carry
seat 2's buttons and the latch (DF-171); a Start dome held at rest keeps the device on
(DF-167); U2's battery FET failing short leaves only the PCM (DF-151); no fuse in the cell
path (DF-172). From the second review: the cell's gap at the pushed end (DEF-53, VER-181);
JLC's order-number marker is not on the board (DEF-55, REQ-FAB-05).

## (d) Before the 5-board order

In order. Effort is Claude's unless it says Alex. The parts lead time (up to about 21 days)
runs in parallel, so step 0.2 goes first.

| # | Step | IDs | Effort |
|---|---|---|---|
| 0.1 | **Alex confirms 684a609** as the order candidate. A later commit to ref-design-island-45 that touches a design path fails the default mode and R7 until its rows are re-checked. Clean the main checkout's uncommitted battler.kicad_pro before any fab run (O-34). | DEF-30, D1 | Alex: 5 min |
| 0.2 | **Order everything with a lead time** (`--shopping --gate evt`): RP2354B and L2 pre-order at JLC; cells from a seller with a datasheet (D23); the panel, an FH12 breakout, an RP2350B dev board; two GSPK151035PN and a 53261-0271; domes and Start/Select strips; calipers, a 4.096 V reference, clip leads, a LiPo bag, a USB-C breakout, 130 ohm resistors. | VER-41, VER-37, VER-43, VER-128, VER-180 | Alex: 1 h, about $100-150 |
| 0.3 | **Run the register on every commit**: one line in scripts/check.sh, `python3 hardware/vv/tools/check_trace.py` (outside hardware/vv, so it waits for Alex's OK). This is what would have caught DEF-47. | DEF-47, O-48 | 15 min |
| 0.4 | **Alex sets up commit signing** (README, 'Approvals': SSH key, allowed-signers file). Until then no APPROVALS.md line counts, so R1, R3 and every waiver stay open (O-R4). | D30 | Alex: 10 min |
| 1 | **Copper and BOM decisions** (each can move copper, so they come first): D5 TH1's B value (BOM only), D27 the cell's gap at the pushed end (hole or pad), D28 JLC's order-number marker (silk or a paid option), D22 the cell-path fuse, D17 the VBUS fault reach, D7 the cutoff with the pad leakage (a reference is copper), D24 the pull-up arrays; then the dome footprint against its drawing. | DF-147, DF-85, DEF-53, DEF-55, DF-172, DF-101, DF-116, DF-164, VER-178 | 1 day with Alex's answers |
| 2 | **Fab outputs**: separate drill files in fab.py; a centroid table for J1-J4 (VER-131 and VER-176 already read fab.py's real output and fail today); fab content check over the exported files with the plated slots, the dome vents and the JLC marker; JLC's dated limits in a data file; edges; the IPC-D-356 netlist with the net ties merged; reproducible outputs; gerbv against the board's own hole list; contacts as placed; check_pcb failing on non-silk warnings. | VER-131, VER-176, VER-132, VER-17, VER-18, VER-26, VER-24, VER-27, VER-162, VER-04, DEF-32, DEF-33, DEF-36 | 1.5 days |
| 2.5 | **The claims script** (check_claims.py, moved to ORDER): every '<ref> <value>', GPIO, pin, test point and dimension quoted in hardware/vv, ref_design/docs and pin_review against design.py, case.scad and pin_review, run by the default mode; then its seeded fault (a GPIO changed and rebaselined must fail: F-R6). | VER-161, G-ORD-26, DEF-54 | 1 day |
| 3 | **Power-path analyses**: hot plug at U2's IN and the enables; DC bias; ratings and the 3V3 budget; the charge window from SLUS810N's TS limits; the latch table (both Starts); the mating peak; heat per mode; back-power; the backlight sim re-run on design.py's values; audio; shelf. | VER-12, VER-30, VER-31, VER-32, VER-33, VER-140, VER-144, VER-159, VER-05, VER-100, VER-103 | 2.5 days |
| 4 | **Datasheets and part identity**: the pin gate reads both rating schemas and its open findings, refuses a waiver with no A-NNN line, compares each IC's lcsc and mpn with design.py and refuses reviews of removed parts (DEF-26, O-R20); the RP2350's input leakage and the BQ24073's OVP minimum into the reviews; archive; errata; application circuits; GPIO mux; LCSC identity; BOM, CPL and PCB fields; land patterns; rotation table. | VER-36, VER-35, VER-16, VER-34, VER-15, VER-14, VER-130, VER-125, VER-29 | 2 days |
| 5 | **Firmware for the ordered chip**: pins generated from design.py; an rp235xb self-test at 2048K; the image against the flash budget. | VER-149, VER-150, VER-151, DEF-34 | 1 day |
| 6 | **The board against the case and the ESTIMATEs**: heights from drawings; the XY chain; placement drift; connector mouths; calipers on a real cell and plug, the panel's FPC and the speaker. | VER-06, VER-112, VER-174, VER-51, VER-19, VER-165, VER-152 | 1 day plus calipers |
| 7 | **The panel and the cell**: the ER-TFT024IPS-3 lit through the breakout with J3's map; the FPC fold; the tail; the cell's spec sheet, cell.json, capacity, incoming inspection. | VER-43, VER-136, VER-44, VER-37, VER-160, VER-153, VER-49 | an evening each once parts arrive, plus 5 h unattended |
| 8 | **Test access and proof the checkers fail**: every bring-up probe point has copper; seed faults into check_netlist, check_pins, check_pcb, check_fit, check_contact_fp. | VER-115, VER-158, VER-22 | 1 day |
| 9 | **Reviews**: the DFMEA review (Alex and two models); the gear list; the speaker's rating; the bridge table; the clean-room review (advisory). | VER-98, VER-116, VER-128, VER-148, VER-40 | Alex: half a day |
| 10 | **Alex's decisions** in APPROVALS.md, in signed commits: the requirements; the H rows that can only be detected on built units, accepted for ORDER (D19; 53 live H rows after the re-rating, 17 more than before); waivers W-01 and W-02; u4 pin 58's waiver (D29). | R1, R3, G-ORD-06, G-ORD-13 | Alex: 2 h |
| 11 | **Release and order**: the release run from a clean tag; the release record; the dry run (valid 7 days); stock on order day (valid 1 day); `check_trace.py --gate order` exits 0; order 5 assembled and 5 bare boards. | VER-143, VER-42, VER-28, VER-41, G-ORD-11, G-ORD-21 | 1 day |

Rough total: about 10-12 working days of Claude's work, 3-4 hours of Alex's, and up to three
weeks of parts lead time in parallel. Re-seeding the register for 684a609 took about one day
(this run), not the half day the first run estimated.

## (e) The case gate, EVT, DVT and PVT

### Case gate (before the printed case is released and the EVT units are assembled)

The Z stack with swelling (VER-20, 0.30 mm under the deck today), the buttons under the shell
(VER-07), no openings (VER-113), the screws (VER-147, 3.2 mm today), the Start/Select strip
against the domes (VER-180), the printed case with real parts and its feel (VER-48, VER-50),
incoming silicone, caps and domes (VER-126), the case release (VER-173). Before any unit is
assembled (G-CASE-06): the docs fixed (DEF-21's list; VER-39 and the claims script VER-161),
every contact class bought (VER-167), and the opening steps changed so J2 is unplugged before
the tub lifts (DF-169). Exit: `--gate case` exits 0.

### EVT: the 5 assembled boards

Day one, before any assembled unit is powered: pill resistance and comb leakage on a bare
board from the same order (G-EVT-12). Then unit 1's first article inspection (VER-166,
VER-109) before any other unit is powered.

Per unit, in this order: unpowered resistances and diode tests (VER-52, VER-53: TP1 8.5-10
kohm from R38 with the R14/D3/R15 and R43/EN1 paths in parallel, TP3 above 1 kohm rules out U2's BAT bridged to its pad); first power from a 100 mA bench
supply with JP1 cut (VER-74); the charge-path screen with USB in and no cell (VER-133: TP2
4.30-4.50 V, TP3 peak 4.25 V or less, TP7 0.70-0.80 V); the cell current with charging off
(VER-169); CV into a 130 ohm load (VER-134: 4.158-4.242 V); TS by a finger and a 4.7k
(VER-135). Only then a real cell, attended, in a LiPo bag (VER-172).

Unit allocation (gates.md): unit 1 FAI, full bring-up, golden readings, the charge timer into
a load (VER-145) and SR_CLK (VER-179); units 2 and 3 bring-up, soak and thermal; unit 4 abuse,
ESD screen, near-field, drop, button life and fault injection (it never ships); unit 5 spare.

Exit: `--gate evt` exits 0 (G-EVT-01 to G-EVT-14; G-EVT-15 is prod_board's).

### DVT: the revised spin

Candidates, decided from EVT's results. BOM-only (each bumps BOARD_REV): 4.7k pull-downs on
AMP_EN and BL_PWM (DF-45, DF-47); R18 56k (DF-95, with D7); RN1-RN4 47k if not done before
(DF-164); R14 47k and R43 220k if not done before (DF-101). Copper: reverse-polarity
protection or a keyed connector on CELL_P (DF-55); series-resistor footprints on the button
nets and the Start domes (DF-100); a wider cell hole at its right end if D27 chose the pad
for EVT (DF-85); a cell with an NTC lead (DF-04). No resistor footprint on TS can fix the
charge window (O-R6), so DF-147 is TH1's B value or the cell's own limit. Exit: `--gate dvt` exits 0: every test of phase EVT or later re-run on the
DVT spin (its result covers a REL-...-DVT record), durability, ESD with a real gun, every
defect CLOSED or RETIRED.

### PVT: the 30-board run

Exit: `--gate pvt` exits 0: a complete release record (tag, baseline, hashes, BOARD_REV,
locked BOM, the no-substitution list in releases/TEMPLATE.md, the case section); FAI on the
first PVT unit; a record per unit; every cell inspected; first-pass yield 90% or more; every
test re-run on the PVT units. Before any unit is sold or given away: UN38.3 and MSDS,
emissions, marking, the fire-enclosure decision (G-PVT-06, DF-87).

## (f) Decisions for Alex

Each goes into releases/APPROVALS.md (requirements, acceptances, deferrals with a scope,
waivers) or into a commit. Claude's proposal follows each. Numbers are kept from the first run
where the decision survives.

**The design and the order**

- **D1. Is 684a609 the order candidate?** Proposed: yes (this brief). It goes into the
  release record (DEF-30).
- **D2. Are the RP2354B and L2 pre-ordered?** Both were out at JLC and LCSC on 2026-10-21 and
  are on the no-substitution list.
- **D3. Will any of the 30 be sold or given away?** That decides the SALE items (UN38.3,
  emissions, marking, DF-87).

**Before the order: copper or BOM**

- **D5. The charge window's hot edge is 47.2 C nominal** (TH1 against U2's fixed TS
  thresholds; DEF-48, DF-147). The first proposal (two DNP footprints) cannot work (O-R6): a
  parallel resistor that brings the hot edge to 45 C drags the cold edge to -11.6 C, and a
  series one makes the hot edge hotter. Proposed: (a) a BOM-only TH1 change to a 0402 10k
  NTC with B near 4250 if JLC stocks one (same pads, about 4.9-45.5 C); else (b) accept
  47.2 C against the cell's datasheet charge limit. REQ-BAT-03 and VER-32 are now one-sided:
  hot edge at or under 45 C (or the cell's limit), cold edge at or over 0 C, at 99.9% of units.
- **D27. The cell's end is 0.25 mm from the routed FR4 edge** (O-R7; DEF-53, DF-85 now H,
  VER-181 FAIL at -0.15 mm after tolerances). Options: (a) D25's soft silicone pad fills the
  gap and also couples TH1 (no copper; the gap is then a pad, not air); (b) widen the hole's
  right end by about 0.5 mm (copper, and TH1 moves with it); (c) restate REQ-BAT-08. Proposed:
  (b) if the copper is reopened for D5 or D28 anyway, else (a) for EVT with REQ-BAT-08
  re-worded to allow a pad at the pushed end.
- **D28. JLC's order-number marker** (O-R19; DEF-55): 'JLCJLCJLCJLC' is in neither
  battler.kicad_pcb nor silk.py, so JLC prints its number where it likes (DF-125: on a comb or
  the BOARD_REV text). Proposed: add the marker to silk.py on the back, away from the combs
  (a design change and a new baseline). The alternative is JLC's paid 'remove order number'
  option with REQ-FAB-05 restated.
- **D22. A fuse in the cell path** (O's objection; DF-172). Today a short downstream of J2
  (C3 cracked, U2's BAT pin bridged to its pad, a lead worn through at the hole's edge, a
  reversed cell, U2's battery FET failed short) ends only at the cell's PCM, on an unbranded
  cell. Proposed: a 1206 fast-acting fuse of about 2 A, rated 6 V or more, in CELL_P between
  J2 and JP5, placed by JLC. It adds about 30-60 mohm (10-20 mV at 0.3 A, under the cutoff's
  margin) and passes the mating peak (roughly 10-15 A for microseconds, an I2t far below a
  2 A fuse's melting point: VER-140 checks it). A PTC is the second choice: resettable, but
  0.1-0.5 ohm and a slow trip. The alternative is to accept the PCM as the only layer for EVT,
  since the 5 units stay with Alex.
- **D17. A faulty USB source reaches U3's EN at 6.9 V** (DEF-50, DF-101). Proposed: R14 47k
  and R43 220k (BOM only): the reach goes to about 9.1 V (EN) and 12.4 V (EN1); at VBUS 4.4 V
  PWR_EN still reads 2.8 V and EN1 2.5 V, over their thresholds. 20 V needs a clamp (copper).
  Or accept for EVT.
- **D7 with D10. The cutoff and R18, decided together** (O's objection). The cutoff can fire
  35 mV low after a CV calibration and never fire on a high-3V3 unit (VER-138: 3.521 V).
  R18 = 56k would keep GPIO47 under IOVDD + 0.5 V when off (DF-95), but it cuts the ratio from
  0.5 to 0.106, so each ADC count becomes about 7.6 mV of VBAT instead of 1.6 mV, and the
  ADC's offset and INL grow 4.7 times against REQ-BAT-12's 50 mV window. Proposed: keep
  470k/470k and waive GPIO47's few microamps by name (the pin review already argues it);
  replace the CV calibration with the bench calibration at 3.50 V (REQ-TST-06); add a firmware
  rule that a reading which stops falling under a steady load is a dropout and shuts down.
  The alternative is a 3.6 V cutoff (costs play time) or a voltage reference (copper).
  New (O-R15): GPIO47's pad leakage through the divider's 235 kohm source. If the RP2350's
  maximum is 1 uA (quoted from memory; VER-34 reads the table), it moves VBAT's reading by
  0.47 V, and REQ-BAT-12's 50 mV window tolerates only about 53 nA that drifts with
  temperature. A bench calibration at 3.50 V removes the room-temperature part only. Decide
  D7 after VER-34 gives the number: if the guaranteed leakage is that large, 470k/470k cannot
  meet REQ-BAT-12 by analysis, and the choice is a lower divider (more off-state drain),
  a buffer, or REQ-BAT-12 stated at 25 C with a measured drift on unit 4.
- **D24. The pill margin** (DEF-49, DF-164): the 10k pull-ups and the 74HC165's 0.94 V VIL
  read a pill as pressed only under about 3.7 kohm. Proposed: RN1-RN4 at 47k (BOM only, a
  4 x 0402 array; its LCSC number to find): about 18 kohm of margin, and the comb leakage
  limit (100 Mohm) is unchanged.
- **D11. E9 pull-downs**: 4.7k on AMP_EN (R37) and BL_PWM, BOM only, or rely on firmware and
  the stepping (DF-45, DF-47). Proposed: wait for VER-75's stepping; both failures are
  low severity.

**Requirement numbers** (all 129 live ones are PROPOSED)

- **D4.** Play time 4 h; charge time: REQ-PRD-04 said 3.0 h from a 1.5 A source, its
  allocation REQ-PRD-04.R 4.0 h from 0.5 A (the BQ24073's USB500 limit is 475 mA). Both now
  say 0.5 A (F-R12); the time is yours to set, no tighter than the allocation. 100,000
  presses; $45 a unit (REQ-PRD-09 now cites VER-117's re-run of cost.py, not bom.md's figure).
  Proposed: DEFERRED with scope EVT for REQ-PRD-03.R, REQ-PRD-04.R and REQ-BAT-02.R until the
  cell's spec sheet and a capacity sample exist; a deferral now ends at its scope.
- **D8.** Legacy USB hosts: U2 charges from plug-in inside 475 mA, before enumeration
  (REQ-PWR-12). Proposed: accept.
- **D9.** The derating rule's two exceptions: GPIO47 (with D7) and the panel's VCI and VDDI at
  up to 3.366 V against 3.3 V (DF-96). Proposed: accept both.
- **D12.** The backlight halves at the minimum duty: each sink within +-25% of nominal at 29%
  (REQ-DSP-12, re-worded; +-20.1% today). Proposed: accept.
- **D14.** Flash budget 1.6 MB; off current 60 uA at 45 C; lot capacity 600 mAh; and the
  BQ24073's 5 h timer as the charge timeout (REQ-BAT-13).
- **D15.** USB input low end: run from 4.40 V with a cell, 4.75 V without (REQ-PWR-01, the
  BQ24073's VIN-DPM). Decide after VER-80.
- **D16.** prod_board: which product requirements apply to your board (the .P allocations).
- **REQ-PWR-13 and VER-99.** The first run carried VER-99 as a pass that only proved a
  deviation (265 uC). At 684a609 there is no deviation: VBUS holds 10 uF (50 uC at 5.0 V) and
  U2's switch is the only path to VSYS, so VER-99 passes on the limit itself and DEF-43 is
  CLOSED. REQ-PWR-13 waits for Alex's approval like every other requirement.

**Before the case release**

- **D18. Swelling room and screws.** A 4.5 mm cell has 0.30 mm free under the deck (REQ-BAT-07
  asks 0.4; VER-182): deepen the tub's pocket by 0.1-0.2 mm, or accept 0.3 mm for EVT. Screws: M2 x 7 in
  the island's four pilots (4.2 mm of thread), the decks' two stay M2 x 6 (3.2 mm, their
  pilots allow 3.3), or accept 3.2 mm for EVT and count stripped bosses in VER-92. bom.md's
  M2 x 8 must change either way.
- **D25. TH1's coupling to the cell** (DF-04): a 0.5 mm silicone thermal pad bridging the
  hole's edge to the cell's end, or accept the lag for EVT and measure it (VER-87).
- **D26. Opening the case** (DF-169, DF-170): change assembly.md so J2 is unplugged before the
  tub lifts away, and the cell sits in its pocket before its plug goes near the board.

**The cell**

- **D23. A cell sold with a datasheet** (O's objection): buy the 403450 from a seller that
  publishes a datasheet with the PCM named (DW01 or equivalent, with its thresholds and
  short-circuit delay) and a UN38.3 summary, rather than relying on an AliExpress seller's
  e-mail. It makes VER-37, VER-88, VER-140 and G-PVT-06 tractable and lowers the occurrence of
  the PCM-only rows (DF-151, DF-172).

**Risk acceptances and waivers**

- **D19.** 25 H rows can only be detected on built units (or need a decision first), so the
  order gate needs Alex's acceptance for ORDER: DF-04, DF-26, DF-55, DF-67, DF-69, DF-70,
  DF-72, DF-74, DF-79, DF-86, DF-87, DF-88, DF-100, DF-103, DF-108, DF-115, DF-117, DF-146,
  DF-147, DF-148, DF-151, DF-167, DF-169, DF-171, DF-172. Take each row's pre-order action
  first (D5, D22, D25, D26). DF-87 (the printed case is not a fire enclosure): proposed through
  DVT, as hobby units kept by their builder. The other 11 H rows (DF-07, DF-57, DF-68, DF-76,
  DF-121, DF-122, DF-123, DF-126, DF-127, DF-128, DF-129) can be DONE before the order once
  their ORDER checks pass.
- **D20.** Sign W-01 (G-ORD-06, the latch replica) and W-02 (G-ORD-13, the perfboard of a
  power path that left in 2d1124d)?
- **D29. u4.json pin 58's waiver** (GPIO47 above IOVDD when off) was written by the design
  session, not by you (O-R20). Once VER-36 is built it fails until an A-NNN line of yours
  cites it. Decide with D7.
- **D30. Commit signing** (O-R4): every commit here has the same author, so only a signature
  tells your approvals from a Claude commit without a trailer or a merge. Set it up once
  (README, 'Approvals'). Proposed: yes, before any approval line.
- **D19 update.** The second review re-rated occurrence on every part never built (O-R8: 3
  now needs 'Similar:') and four rows to severity 7 (O-R23), so 53 live rows are H: new are
  DF-06, DF-09, DF-25, DF-32, DF-54, DF-58, DF-75, DF-80, DF-85, DF-102, DF-135, DF-144,
  DF-145, DF-152, DF-155, DF-170 and prod_board's DF-176. `check_trace.py --gate order
  --verbose` lists those still to accept or close.

**Disputed findings.** The second review left none disputed: every one of its 39 findings
was accepted or revised by the other model and is applied (section l).
- **D21.** DVT: reverse-polarity protection or a keyed connector on CELL_P (DF-55).

Retired decisions: D6 (3V3 through the unplug: no gap with the BQ24073), D13 (the ILI9488's
redraw: the panel changed), D10 (folded into D7).

## (g) The first run's sign-off objections and where each landed

| Objection (O: Opus, F: Fable) | What was done |
|---|---|
| O blocker: the register described a superseded design | Re-seeded at 684a609 (this commit range); the baseline names every design path and drift fails the default mode; DEF-47 records why nothing caught it; a realistic estimate (about a day for the re-seed) in (d) |
| O major: R6 and R7 missed the copper and the fab scripts | DESIGN_PATHS and the baseline cover battler.kicad_pcb, last_route.ses, build_pcb.py, route.py, silk.py, fab.py and board/lib; seeded fault: a silk.py change after a release fails R6 |
| O major: DVT and PVT accepted EVT bench results | ver_ok at DVT and PVT needs a test of phase EVT or later to cover a release of that phase; gates.md's text matches; seeded fault |
| O major: DEFERRED never expired | DEFERRED needs a scope in APPROVALS.md and ends after it (R1 says so); seeded faults for a missing scope and an ended deferral |
| O major: the order gate broke O-9 | VER-40 is advisory-only (named only by G-ORD-12's ADVISORY), so PHASE:ORDER skips it; the doc sweep (VER-39), the claims script (VER-161) and the BOM coverage (VER-167) moved to G-CASE-06, before assembly; DEF-41 is FIXED and held by R4 at the case gate. Not moved: VER-115 and VER-158 (test access) stay at ORDER because a missing probe pad is a copper change |
| O major: no decision on a fuse in the cell path | D22 with a proposal; DF-172 (H) |
| O major: 'nothing dropped' could not be verified | The ID ledger in (k): all 115 IDs, where each landed, and 30 that cannot be traced (19 never named, 11 named only as a merge) |
| O minor: VER-99 presented as done | At 684a609 there is no deviation to present (section f, 'REQ-PWR-13 and VER-99'): VER-99 now checks the limit itself and passes; DEF-43 CLOSED with that changed check; REQ-PWR-13 still awaits Alex |
| O minor: D10 needs D7 | D7 and D10 decided together, with the ADC numbers |
| O minor: the cell source is weak | D23 |
| O minor: seeded faults missed R5, R6, acceptances, waivers, part: staleness | 12 faults in a scratch clone: R5, R6, an acceptance past its scope, a signed waiver, a waiver in a Claude commit, part: staleness, a deferral past its scope, an EVT result at DVT, an untracked covers file, a statement edited after approval, a result that disagrees with its evidence, expired evidence |
| F major: the F-32 advisory cut was not applied | As O's: advisory-only checks; seeded fault: making G-ORD-12 binding shows VER-40 failing |
| F major: G-ORD-07 could not be met (DF-61) | DF-61 out of G-ORD-07 (met through G-ORD-04's VER-43 and VER-136); gates.md and check_trace's R3 text now say a later-phase detects check needs an acceptance |
| F minor: T checks without evidence headers | Every A and T PASS or FAIL needs the header; seeded fault (VER-43) |
| F minor: TRACKED included untracked files | `git ls-files --cached` only; seeded fault in the clone |
| F minor: VER-09's evidence in the old format | VER-09 RETIRED (the 3-ring left in 2d1124d) and its file deleted; all 2026-10-24 evidence replaced by fresh runs |
| F minor: the ID table | (k) |

Not fixed: none of the objections. Two points are partial by choice: VER-115 and VER-158 stay at
ORDER (above), and DEF-47 stays FIXED, not CLOSED, until scripts/check.sh runs check_trace
(step 0.3, outside hardware/vv).

## (h) What this plan still cannot catch

- **Small numbers.** Five boards, then 30, cannot show a failure rate under a few percent.
- **Time.** Cell ageing and swelling over years, ENIG wear, tape creep, slow flux migration.
- **Faults nobody listed.** The DFMEA names every part, but only the failure modes someone
  thought of; seeded faults prove a checker fails on the faults seeded.
- **Datasheet numbers this register quotes from memory.** design_numbers.py names its
  constants (RT9013 +-2%, CV +-1%, the 74HC165's VIL, the MCP6002's offset); VER-34's source
  audit must confirm each.
- **Bench honesty, approvals, standards, JLC's side, firmware beyond the self-test, use**: as
  the first run listed them.
- **A design that moves again.** Another session may commit to ref-design-island-45. Any
  change to a design path fails the default mode and every gate until its rows are re-checked;
  the register cannot re-check them itself.
- **The first run's lost ideas.** 30 debate IDs cannot be traced to any text (k).
- **Quoted values until VER-161 exists.** The claims script is not built, so a GPIO, a count
  or a dimension quoted in a row can still drift from design.py unseen (DEF-54). Its seeded
  fault (F-R6) waits for it.
- **Approvals before signing.** Until D30 is done no approval counts, and nothing can tell an
  unsigned line of Alex's from anyone else's.
- **Typed tolerances in design_numbers.py.** The RP2350's input leakage, JLC's routing
  tolerance (0.2 mm), its minimum plated slot (0.50 mm) and the print's wall tolerance are
  quoted, not read from a dated source (VER-34, VER-17, VER-174).

## (i) The gate today

At d3f66ea, the evidence commit after fc862fa (14 PASS, 9 FAIL, 52 NOT_RUN, 102 NOT_BUILT,
2 CUT, 3 RETIRED):

```
$ python3 hardware/vv/tools/check_trace.py
STRUCTURE: OK (4 warnings)
  requirements 133 {'PROPOSED': 130, 'RETIRED': 3}
  DFMEA rows 177 (live: 53 H, 65 M, 27 L; 32 RETIRED)
  checks 182 {'PASS': 14, 'FAIL': 9, 'NOT_RUN': 52, 'NOT_BUILT': 102, 'CUT': 2, 'RETIRED': 3}
  defects 55 {'OPEN': 22, 'FIXED': 20, 'CLOSED': 6, 'RETIRED': 7}
  approvals by Alex: 0 counted (signed); commit signing not set up, so none can count yet
  order branch: ref-design-island-45 has no design commit after 684a609
  design baseline: matches HEAD
  warnings: the pass bands of VER-67, VER-88, VER-134 and VER-139 are 1.4-2.2 x their DMM's
  accuracy (want 4 x)
exit 0

$ python3 hardware/vv/tools/check_trace.py --gate order
  PASS  R5, R6 (no release yet), R7, R8
  INFO  G-ORD-05, G-ORD-12 (advisory); N/A G-ORD-25 (prod_board)
  FAIL  G-ORD-01 to G-ORD-04, G-ORD-06 to G-ORD-11, G-ORD-13 to G-ORD-24, G-ORD-26
        (the FAILs among them: VER-131 drill, VER-176 CPL, VER-181 the cell's gap,
        VER-138 cutoff, VER-177 VBUS)
  FAIL  G-ORD-22: 67 ORDER checks not passing (VER-40 excluded as advisory)
  FAIL  R1 127 PROPOSED; R2 56 requirements with an ORDER check not passing;
        R3 52 H rows neither accepted nor DONE; R4 17 OPEN defects with Gate ORDER
RESULT: FAIL (27 of 31 not met); 1 N/A, 2 advisory
```

The case gate fails 10 of 13, EVT 18 of 21, DVT 9 of 12, PVT 10 of 13, and prod_board's order
gate 5 of 7 (24 criteria N/A; R3 now reads its DFMEA rows, O-R12). The checks that pass: the
netlist, the pin gate, ERC, DRC with parity, both 4-ring contact footprints, inrush, holes,
the backlight's worst case, the MLCC keep-out, TH1's geometry (now from the board), two
inspections (VER-104, VER-124) and the seeded faults (VER-129, 63 of 63).

## (j) Past bugs and what catches them

| Defect | What happened | Caught now by | When |
|---|---|---|---|
| DEF-01 | U1's REF on VBUS back-fed PWR_EN | VER-159, VER-127 | before the order |
| DEF-02 | R15 at 10k cut the pill margin | VER-33, VER-78 | before the order |
| DEF-03-06, 09 | the TP4056 power path's hand-over, hot plug and CE limits | RETIRED with those parts (2d1124d, 1fd0956); the BQ24073's are VER-12, VER-70-72 | |
| DEF-07, 08 | J6 swapped; 22 ohm USB resistors | VER-02 (CLOSED) | every change |
| DEF-10 | no PWR_HOLD pull-down | VER-02, VER-77, VER-16 | before the order |
| DEF-11 | R5 kept from the bigger cell | VER-160, VER-37, VER-66 | before the order |
| DEF-12, 13, 29 | parts out of stock | VER-41 (valid 1 day), VER-14 | order day |
| DEF-14 | U1's JLC rotation | VER-29, VER-28 | before the order |
| DEF-15-19, 25 | case fits and openings | VER-48, VER-92, VER-126, VER-07, VER-113, VER-20 | case gate |
| DEF-20, 38 | J6's mouth; C9 by a boss | RETIRED with the parts; the classes are VER-165 and VER-142 | |
| DEF-21 | docs disagree with the design (23 statements) | VER-39 (FAIL), VER-161 | case gate |
| DEF-26-28 | the pin gate's blind spots; guessed heights; analyses not in the repo | VER-36, VER-112, VER-22, design_numbers.py | before the order |
| DEF-30, 47 | the order candidate not recorded; the register drifted | VER-42, the baseline rule, VER-129 | every change once check.sh runs it |
| DEF-31 | screws engage 3.2 mm | VER-147 (FAIL) | case gate |
| DEF-32, 33 | the drill file; the CPL offsets | VER-131, VER-176 (both FAIL) | before the order |
| DEF-34 | no firmware for the ordered chip | VER-149-151 | before the order |
| DEF-35, 52 | the trace gate's own holes; then words, typed results and unsigned approvals still passing it | VER-129 (63 seeded faults) | every change |
| DEF-36 | check_pcb passes non-silk warnings | VER-04, VER-22 | before the order |
| DEF-37, 48 | TH1's coupling; the 47 C hot edge | VER-164, VER-87, VER-32 | before the order and EVT |
| DEF-39, 43 | backlight over its limit; inrush | VER-137, VER-99 (both CLOSED) | every change |
| DEF-40 | the cutoff may never fire | VER-138 (FAIL), VER-139, VER-155, VER-156 | before the order |
| DEF-41, 42 | assembly next to a live cell; Start/Select parts not bought | VER-91, VER-167 (FAIL) | case gate |
| DEF-45 | the backlight sim runs for the old panel | VER-05 | before the order |
| DEF-46 | bench cell emulation | VER-70, VER-67 | EVT |
| DEF-49, 50, 51 | the pill margin; the VBUS fault reach; swelling room | VER-45, VER-177 (FAIL), VER-182 (FAIL) | before the order and the case gate |
| DEF-53 | the pushed cell 0.25 mm from the FR4 edge | VER-181 (FAIL) | before the order |
| DEF-54 | rows still written for 76955d2 after the re-seed | VER-161 (not built), the retired-citation rule | before the order |
| DEF-55 | JLC's order-number marker not on the board | VER-132 (k) | before the order |

## (k) The ID ledger of the first run's debate

The first run's two models raised 115 ideas (O-1 to O-63 from Opus, F-1 to F-52 from Fable)
and reported 115 agreed, 0 dropped. It saved only those counts, not the texts. This table
lists every ID, what the register says the idea was, where it landed at 76955d2 (9f25a71's
files: register IDs, PLAN.md sections, check_trace.py), and where it lives at 684a609. "Not
traceable" means no file of the first run names the ID except as a merge, so whether it was
applied cannot be checked from the repository: 19 IDs are never named, and 11 are named only
as merged with another.

| ID | The idea, as the register records it | Landed at 76955d2 | At 684a609 |
|---|---|---|---|
| O-1 | every check reached by a gate, requirement or H row no later than its phase | DEF-32, DEF-35, G-ORD-22, PLAN (a), check_trace.py | check_trace check_reach; G-ORD-22 |
| O-2 | an H row is DONE only when every check that can see its failure passes | DEF-35, PLAN (a), check_trace.py | check_trace DFMEA rules; R3 |
| O-3 | approvals count only from lines Alex committed (blame) | DEF-35, PLAN (a) | APPROVALS.md; seeded (git) faults |
| O-4 | a PASS rests on saved evidence with a header (with F-3) | DEF-35, PLAN (a), PLAN (g), check_trace.py | check_evidence; now every T check too |
| O-5 | part: covers so loop-made parts make evidence stale | DEF-35, DEF-45, PLAN (a) | parse_covers; seeded (git) fault 'part: cover' |
| O-6 | release: covers for bench results, stale only by a change record | PLAN (a), check_trace.py | staleness(); CHANGE_TEMPLATE |
| O-7 | prod_board's gate reads only its own requirements | PLAN (a), check_trace.py | req_scope() |
| O-8 | unit allocation: DVT-deciding screens on unit 4 at EVT | gates.md, PLAN (c) | gates.md unit table |
| O-9 | the order gate holds only what copper and the BOM freeze | gates.md, PLAN (g) | gates.md; G-CASE-06 now holds the docs (sign-off objection) |
| O-10 | cut the latch replica and the perfboard (with F-32) | VER-47, VER-123, G-ORD-06, G-ORD-13, PLAN (f), PLAN (g) | W-01, W-02; G-ORD-06, G-ORD-13 |
| O-11 | archive validation_plan.md; vp_map.csv is a one-time map | DEF-22 | DEF-22; vp_map.csv |
| O-12 | check_pcb.py passes non-silk warnings (with F-27) | DEF-36, PLAN (g) | DEF-36 (open) |
| O-13 | merged with F-47; the record names no content | PLAN (g) | not traceable |
| O-14 | the drill file mixes PTH and NPTH | DEF-32 | DEF-32; VER-131 (FAIL) |
| O-15 | merged with F-20 and F-21; F-20 is the export-origin item of VER-132 | PLAN (g) | VER-132 (h) |
| O-16 | the CPL uses footprint origins | DEF-33 | DEF-33; VER-176 (FAIL) |
| O-17 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| O-18 | named in D1 as needing J6 or the strap (R48-R51), which left in 86bf694 | PLAN (f) | moot at 684a609: RTT/USB CDC logs (REQ-MCU-04) |
| O-19 | bench cell emulation must hold 3.50 V: CHG_OFF high | DEF-46, PLAN (c) | DEF-46; VER-70, VER-67 |
| O-20 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| O-21 | backlight worst-case tolerance (with F-7) | REQ-DSP-05, REQ-DSP-12, DEF-39, PLAN (g) | VER-137 (PASS, 78.0 mA); DEF-39 CLOSED |
| O-22 | derating rule rewritten | REQ-PWR-14, DEF-44 | REQ-PWR-14; DEF-44 |
| O-23 | cutoff accuracy needs a tolerance | REQ-BAT-12, DEF-40 | REQ-BAT-12; VER-138 (FAIL); DEF-40 |
| O-24 | charge at low VBUS (with F-9) | REQ-PWR-16 | REQ-PWR-16 (now the BQ24073's VIN-DPM) |
| O-25 | un-cut vp #180, the redraw time (with F-45) | vp #180, PLAN (g) | REQ-DSP-11 (ILI9488 risk gone, 53e21c8) |
| O-26 | detection rating capped by the checks; O <= 2 needs built hardware (with F-13) | DEF-35, PLAN (a), PLAN (g), check_trace.py | check_trace DFMEA rules |
| O-27 | merged with F-16; the record names no content | PLAN (g) | not traceable |
| O-28 | merged with F-15; the record names no content | PLAN (g) | not traceable |
| O-29 | assembly worked next to a live cell (with F-37) | DEF-41 | DEF-41 FIXED; DF-102, DF-169, DF-170 |
| O-30 | TH1 at the cell's edge, bare pads on the pouch (with F-36) | DEF-37, PLAN (g) | DEF-37 (open), DF-04, VER-164 (PASS geometry) |
| O-31 | the speaker's rating | REQ-AUD-06, DEF-44 | REQ-AUD-06; VER-128 |
| O-32 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| O-33 | merged with F-22; the record names no content | PLAN (g) | not traceable |
| O-34 | clean Alex's main checkout's battler.kicad_pro before a fab run | step 0.1 | PLAN step 0.1 |
| O-35 | a BOM-only change bumps BOARD_REV | DF-140, PLAN (a), PLAN (f), check_trace.py | check_releases(); BOARD_REV is on the silkscreen now |
| O-36 | per-unit findings with disposition and re-test | REQ-TST-08 | REQ-TST-08; evidence/findings.csv |
| O-37 | firmware for the ordered chip (with F-44) | REQ-MCU-11, PLAN (g) | REQ-MCU-11; DEF-34 (open) |
| O-38 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| O-39 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| O-40 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| O-41 | merged with F-35; the record names no content | PLAN (g) | not traceable |
| O-42 | cell mating peak against the PCM | REQ-BAT-14 | REQ-BAT-14; DF-103; VER-140 |
| O-43 | C9 0.9 mm from a boss | DEF-38 | DEF-38 RETIRED (C9 left, 2d1124d); VER-142 (PASS), REQ-MEC-12 |
| O-44 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| O-45 | an approved statement edited later needs approval again | PLAN (a), check_trace.py | cross_check_approvals; seeded (git) fault |
| O-46 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| O-47 | heat budget per mode (with F-48) | REQ-THM-04 | REQ-THM-04; VER-144 |
| O-48 | a hardware step in scripts/check.sh | step 0.3, step 1 | PLAN step 0.3; DEF-47 |
| O-49 | the cutoff may never fire | REQ-BAT-12, DEF-40 | REQ-BAT-12; DEF-40; VER-138 (FAIL, 3.521 V) |
| O-50 | charge timeout | REQ-BAT-13 | REQ-BAT-13 (now the BQ24073's 5 h timer); VER-145 |
| O-51 | off current at 45 C | REQ-PWR-07 | REQ-PWR-07; VER-146 |
| O-52 | screws engage 1.4 mm | REQ-MEC-09, DEF-31 | DEF-31 (now 3.2 mm, M2 x 6); VER-147 (FAIL) |
| O-53 | flash budget | vp #180, REQ-MCU-10 | REQ-MCU-10; VER-151 |
| O-54 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| O-55 | every assembled part and contact in a DFMEA row | PLAN (a), check_trace.py | DFMEA:COVERAGE (0 missing) |
| O-56 | no firmware for the ordered chip | vp #56, REQ-MCU-11, DEF-34 | DEF-34 (open) |
| O-57 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| O-58 | usable capacity per lot | REQ-BAT-16 | REQ-BAT-16; VER-153 |
| O-59 | panel bond under press and heat | REQ-MEC-10 | REQ-MEC-10; VER-154 |
| O-60 | no gain from a no-cell charger edge | REQ-TST-06, DEF-40, PLAN (g) | REQ-TST-06; VER-156 |
| O-61 | connector mating budget | REQ-TST-07 | REQ-TST-07; VER-157 |
| O-62 | the self-test's UART needs J6 (D1) | PLAN (f) | moot: J6 left in 86bf694; logs over RTT or USB CDC |
| O-63 | bench calibration at 3.50 V | REQ-TST-06, PLAN (g) | REQ-TST-06; VER-155 |
| F-1 | the gate fails when HEAD's design is not the baseline | DEF-30, PLAN (a) | R7; now also a structure error (DEF-47) |
| F-2 | covers regexes match whole words | DEF-35, PLAN (a) | parse_covers() |
| F-3 | evidence is a saved output (with O-4) | DEF-35, PLAN (a), PLAN (g), check_trace.py | check_evidence() |
| F-4 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| F-5 | valid_days for external state | DEF-35, PLAN (a) | staleness(); seeded (git) fault 'expired' |
| F-6 | gate criteria carry a board column | PLAN (a) | gates.md |
| F-7 | backlight over its limit at worst case (with O-21) | REQ-DSP-05, DEF-39, PLAN (g) | VER-137 (PASS); DEF-39 CLOSED |
| F-8 | REQ-PWR-02 broke its own transient rule | DEF-06, DEF-44 | DEF-44; REQ-PWR-02 rewritten for the BQ24073 |
| F-9 | no TP4056 board charges to done at 4.40 V (with O-24) | REQ-PWR-16, DEF-44 | REQ-PWR-16 (now VIN-DPM) |
| F-10 | 1 kohm in series proves nothing: test at 10 kohm | DEF-02 | DEF-02; REQ-PWR-08 now 1 kohm through a metal dome |
| F-11 | single-fault touch limits, not normal ones | DEF-44 | REQ-PRD-07; DEF-44 |
| F-12 | the 0.13 mm ring failed every via | REQ-FAB-03, DEF-44 | REQ-FAB-03; DEF-44 |
| F-13 | DF-87 could not reach L at O = 1 (with O-26) | DF-87, DEF-35, PLAN (a), PLAN (g) | DF-87; check_trace |
| F-14 | sleeve the cell leads at the board's edge | REQ-BAT-15 | REQ-BAT-15; DF-108 |
| F-15 | merged with O-28; the record names no content | PLAN (g) | not traceable |
| F-16 | merged with O-27; the record names no content | PLAN (g) | not traceable |
| F-17 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| F-18 | TH1 open and R39 shorted are safe failures | PLAN (g) | DF-109 (R39 left; TH1 open or shorted) |
| F-19 | U1's two rotation notes are both true | DEF-14 | DEF-14 (open) |
| F-20 | export origin flags and hole-to-copper alignment (with O-15) | vp #88, PLAN (g) | VER-132 (h) |
| F-21 | merged with O-15; the record names no content | PLAN (g) | not traceable |
| F-22 | merged with O-33; the record names no content | PLAN (g) | not traceable |
| F-23 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| F-24 | plug-in inrush over 50 uC | DEF-43 | DEF-43 CLOSED (no deviation at 684a609); VER-99 (PASS) |
| F-25 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| F-26 | VER-09 with the measured Start/Select pill | VER-09 | VER-09 RETIRED (domes, 2d1124d); VER-178 |
| F-27 | check_pcb non-silk warnings (with O-12) | DEF-36, PLAN (g) | DEF-36 (open) |
| F-28 | connector access | REQ-MEC-11 | REQ-MEC-11; VER-165 |
| F-29 | the case gets its own gate | gates.md, PLAN (g) | G-CASE |
| F-30 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| F-31 | a design commit after a release needs a change record | PLAN (a) | R6, now over the copper and fab scripts too |
| F-32 | cut the latch replica and the perfboard; G-ORD-12 advisory (with O-10) | VER-47, VER-123, G-ORD-06, G-ORD-12, G-ORD-13, PLAN (f) ... | advisory-only checks (VER-40) out of PHASE: |
| F-33 | defer play time, charge time and charge current | REQ-PRD-03.R, REQ-PRD-04.R, REQ-BAT-02.R, PLAN (f) | PLAN decision D4; DEFERRED now needs a scope |
| F-34 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| F-35 | merged with O-41; the record names no content | PLAN (g) | not traceable |
| F-36 | TH1 at the cell's edge (with O-30) | DEF-37, PLAN (g) | DEF-37 (open) |
| F-37 | assembly next to a live cell (with O-29) | DEF-41 | DEF-41 FIXED |
| F-38 | test uncertainty ratio warning (4 x) | VER-116, vp #91, PLAN (a), check_trace.py | tur_check(); 4 warnings today |
| F-39 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| F-40 | Start/Select pills and caps not in the BOM | DEF-42 | DEF-42 (domes now); VER-167 (FAIL) |
| F-41 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| F-42 | no text kept: the first run saved its debate only as counts (115 agreed, 0 dropped) and no file names this ID | nowhere | not traceable |
| F-43 | a LiPo bag in the gear list | VER-116 | VER-116, VER-172 |
| F-44 | firmware for the ordered chip (with O-37) | vp #56, REQ-MCU-11, DEF-34, PLAN (g) | REQ-MCU-11; DEF-34 (open) |
| F-45 | un-cut vp #180 (with O-25) | vp #180, PLAN (g) | REQ-DSP-11 |
| F-46 | R18 = 56k for GPIO47 | DF-95, REQ-PWR-14, PLAN (f) | PLAN decision D7 (with the cutoff) |
| F-47 | merged with O-13; the record names no content | PLAN (g) | not traceable |
| F-48 | heat budget per mode (with O-47) | REQ-THM-04 | REQ-THM-04; VER-144 |
| F-49 | pill and comb tests on a bare board from the order | REQ-BTN-01, G-ORD-05, G-EVT-12 | G-EVT-12; G-ORD-05 advisory |
| F-50 | a shopping list by lead time | PLAN (a), PLAN (g), check_trace.py | check_trace --shopping |
| F-51 | golden-unit readings | PLAN (g) | VER-171 |
| F-52 | probe slip, with and without a cell | VER-116, PLAN (g) | DF-114, DF-115 |

## (l) The second review's ID ledger (2026-10-25)

Two models (O: Opus, F: Fable) attacked the re-seeded register and judged each other's
findings. All 39 were accepted or revised by the other model; none is disputed. Where the
other model revised a fix, the revision is what was applied.

| ID | Finding | Where it landed |
|---|---|---|
| O-R1 | VER-176 and VER-131 passed on a word in fab.py | design_numbers.py cpl and drill run fab.py's export into a temp dir and read the CPL and the Excellon files; seeded (a comment in fab.py keeps both at FAIL); DEF-52 |
| O-R2 | uncommitted register and evidence edits opened the gate | R5 fails on anything uncommitted under hardware/vv; evidence counts only when committed and unchanged; DEF-35 reopened with DEF-52; three seeded faults |
| O-R3 | inspections and demos passed on any repository file | every PASS needs a header under evidence/, with WHO and ARTEFACT for I and D; only VER-104 and VER-124 keep the in-file pointer; two seeded faults |
| O-R4 (revised) | approvals counted from any commit without a Claude trailer | a line counts only from a commit git shows as signed (%G? = G), never a merge; pending where signing is not set up; README 'Approvals'; D30; the clone signs with a throwaway key; two seeded faults |
| O-R5 | R4 at ORDER blocked by case and docs defects | every defect has a Gate; DEF-21, 31, 37, 42, 51 are CASE; two seeded faults |
| O-R6 | D5's footprints cannot give 0-45 C | D5 rewritten (a higher-B TH1 or accept 47.2 C); REQ-BAT-03 and VER-32 one-sided; DF-147, DEF-48 |
| O-R7 | the pushed cell's end 0.25 mm from the FR4 edge | design_numbers.py cellgap, VER-181 (FAIL); DF-85 H; DEF-53; D27 |
| O-R8 | occurrence 3 on parts never built | rows on new parts re-rated 4-8; O = 3 needs 'Similar:' (check_trace rule, seeded); 17 rows newly H |
| O-R9 | checks still written for 76955d2 | VER-29, 33, 36, 77, 89, 130, 132, 148, 159, 166 rewritten; counts and outline derived, not typed; a live row citing a RETIRED id fails; DEF-54 |
| O-R10 | latch checks covered only seat 1's Start | VER-33 (both paths, D8, Q7, R62, R63, SW23 in covers), VER-78 and VER-79 on SW21 and SW23 through 1 kohm |
| O-R11 | DVT and PVT counted EVT inspections and demos | the release-of-phase rule applies to every check that covers a release; seeded (an EVT FAI at PVT) |
| O-R12 | prod_board's gate evaluated no DFMEA row | df_scope includes 'both'; ref_design rows relabelled; DF-174 to DF-177 for prod_board's contacts; VER-183; G-EVT-15 |
| O-R13 (revised) | the design-hash rule saw only this branch | README's parsed 'Order branch' line; default mode warns and R8 fails the order gate on a design commit after 684a609; battler.kicad_pro in the baseline; VER-143 refuses a drifted tree; seeded |
| O-R14 | VER-164 saw 2 of 11 back-side footprints | th1 reads every flipped footprint and TH1 from the board (pcbnew); seeded (TP11 moved next to the hole fails) |
| O-R15 | the cutoff analysis left out GPIO47's leakage | cutoff prints the leakage band (0.47 V per uA through 235 kohm); D7 waits for VER-34's number |
| O-R16 | DF-159's detection cannot see one LED | VER-43 measures each LED by its own cathode pin; design_numbers backlight says so; DF-159 detection reworded |
| O-R17 (revised) | latch and TS numbers off the circuit | REQ-PWR-11 cites VER-33's computed bands; REQ-THM-03 and VER-68 0.49 V; VER-52 TP1 8.5-10 kohm from the full network |
| O-R18 | EN1 at its absolute maximum; OVP nominal | REQ-PWR-02, VER-12, VER-71: EN1 <= 6.7 V, VBUS under OVP's minimum less 0.2 V (VER-34 takes the minimum into u2.json) |
| O-R19 | the JLC marker is not on the board | REQ-FAB-05 note; VER-132 (k); DEF-55; D28 |
| O-R20 (revised) | the pin gate accepts anyone's waiver, never checks identity | VER-36: waivers cite an A-NNN line, lcsc and mpn compared with design.py, reviews of removed parts fail, acceptance list generated; D29 |
| O-R21 | partial scripts recorded as full PASSes | VER-20 split (VER-182 is the swell script); VER-131 now whole; MIN_DUTY read from pinmap.md; check_trace refuses a CHG record that omits a bench test of a changed part (seeded) |
| O-R22 | no check read the real fab output; VER-27, VER-28 lists | VER-131 and VER-176 read the export; VER-27 compares against the board's own hole list (slots, vents); VER-28 adds J5, SW18, SW19 |
| O-R23 | lost main functions rated 6 | DF-58, DF-109, DF-160, DF-163 at S7; VER-103 re-derives the off budget without D5; VER-102 at 0.5 A |
| F-R1 | DVT and PVT accept EVT inspections | as O-R11 |
| F-R2 (revised) | VER-77 probed GPIO2 for PWR_HOLD | VER-77 at U4 pin 55 (GPIO44) via D2 pin 2 or R42, TP6, TP4; VER-161 gets the GPIO pattern and moves to ORDER; u4.json's E9 text logged under DEF-21 (VER-39) |
| F-R3 (revised) | as O-R1 | as O-R1; the release record carries the sha256 of the uploaded zip, BOM and CPL (TEMPLATE.md) |
| F-R4 (revised) | VER-130 rejected the correct CPL (J4) | VER-130 derives the excluded set and the counts from design.PARTS and NO_ASSEMBLY; VER-132 (f) 'B.Paste empty' |
| F-R5 | the old 70.5 x 151.34 mm outline | VER-166 reads the outline (72.4 x 127.7 mm); REQ-FAB-11 and VER-90 0.96 mm; bom.md:16 in DEF-21 and VER-39 |
| F-R6 (revised) | nothing staled row text when the design moved | VER-161 at ORDER with the patterns and scope given, run by the default mode (not built: step 2.5); G-ORD-26; its seeded fault waits for it |
| F-R7 (revised) | VER-89 and VER-91 inspect a TP4056 and a soldered speaker | VER-89's list for the VQFN-16, TSSOP-16s, FH12 and the domes; VER-91 with no iron; REQ-ASM-01 split, REQ-ASM-01.R for ref_design |
| F-R8 | VER-27's drill list omitted the dome vents | as O-R22 |
| F-R9 (revised) | VER-36's acceptance list named old reviews | as O-R20; VER-159 names U2 pins 7 and 9 |
| F-R10 | units.csv counted J6 matings | mating_j4 |
| F-R11 (revised) | VER-78 said 10 kohm against 1 kohm | 1 kohm in VER-78, VER-33 and REQ-PWR-08; VER-33 reports the largest series resistance that still turns on |
| F-R12 (revised) | charge-time source current | REQ-PRD-04 and VER-102 at 0.5 A; the 3.0 vs 4.0 h choice is Alex's (D4) |
| F-R13 (revised) | R6 and the baseline left out schematics and scripts | the schematics, battler.kicad_pro, the board/tools tree and case/export.py in DESIGN_PATHS and the baseline; positions.json's sha256 in the release record |
| F-R14 (revised) | VER-148 and DF-144 leaned on retired or wrong examples | VER-148 names DF-146 and DF-148; DF-144's variant list corrected; VER-109 reads U2's marking; the retired-citation rule (seeded) |
| F-R15 | VER-52's TP1 band ignored the second path | as O-R17 |
| F-R16 (revised) | smaller stale statements | REQ-PRD-09 cites VER-117; VER-29 cites VER-176's evidence; vp_map row 1 37 parts, 322 pins; bom.md:16 and :55 in DEF-21 and VER-39 |

Not done here, by design: the new check scripts (check_claims.py, check_bom_cpl.py,
check_fab_content.py, latch.py, temp_window.py, release.py, the extended check_pins.py) are
work items in section (d); design changes (the drill flag, the marker, the hole) wait for
Alex's decisions; files outside hardware/vv (pin_review, docs) are logged under DEF-21.
