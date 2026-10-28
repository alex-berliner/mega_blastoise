# The plan: from here to 5 boards, then 30

**Decision (Alex, 2026-10-28): the 5-board JLC order is the ESP32-S3-WROOM-1-N16R8 board** in
`hardware/ref_design/docs/esp32s3_board_proposal.md` on main (dcb6a76, e79f251), not the RP2354B
board below. It is not designed yet: no design.py exists for it. This register will be re-seeded
for it once its design exists. The RP2354B content (sections b to l) is frozen at tag
`vv-rp2354b-final` (c8ac9dd) and stays as history; nothing in it is the plan for the next order.
What it found that the new board must also satisfy is [carryover.md](carryover.md). On 2026-10-28
the framework was fixed for the next board, after both sign-off reviewers declined
vv-rp2354b-final: an ACCEPTED defect status with a signed DF-ACCEPT line (README, Rules), a
'sources' column that makes a PASS on a missing datasheet BLOCKED, R8 failing the order gate on a
proposal doc changed after the baseline, and the grid wording below (sections d and k).

**Not ready to order (RP2354B, history).** The board that was to be ordered is `hardware/ref_design` at **71737ae** (main,
2026-10-27: the board spin 40281b8 on ref-design-35in, merged as bfe9609, then a SOURCES.md
fix). On 2026-10-27 this register, which described be437f8, was re-seeded for it: every
scripted check and every check that failed at be437f8 was re-run against 71737ae's files
(section c), the rows for parts and nets that changed were rewritten, retired or added
(section b), and round 3's twelve sign-off objections were answered (section g). The default
mode passes; the order gate fails, as it should (section i). The review of 2026-10-27 and
where each of its 16 findings landed is section (l).

Contents: (a) the system on one page; (b) what changed in the design (be437f8 to 71737ae);
(c) the board at 71737ae: the 13 problems sent to the board session, and what is new; (d)
before the 5-board order; (e) the case gate, EVT, DVT and PVT; (f) decisions for Alex; (g)
round 3's sign-off objections and where each landed; (h) what this plan still cannot catch;
(i) the gate today; (j) past bugs and what catches them; (k) the coverage grid; (l) the
review of 2026-10-27, its 16 findings and where each landed. The earlier
rounds' ledgers (round 2's objections, the first run's and the second review's ID ledgers,
round 3's 23 findings, the grid's audit, be437f8's findings) are in [HISTORY.md](HISTORY.md)
(O-S9).

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
  be437f8 failed it at once, as designed, and the rebase onto 71737ae on 2026-10-27 did the same;
  F-R3: a hash rewritten from HEAD fails too). README names the order branch (main since
  71737ae): a design commit on it after the baseline warns in the default mode and
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

## (b) What changed in the design (be437f8 to 71737ae)

`git diff --stat be437f8 71737ae -- hardware/ref_design`: 68 files; most are the routed board,
the route session, the placement search's tools, the print kits and the datasheets (now
committed in hardware/ref_design/datasheets with SOURCES.md). What touches this register:

| Change | Where | Register effect |
|---|---|---|
| R61 3.9k to 3.74k (C15172) | design.py | VER-137 PASS; DEF-39 CLOSED; DF-130 re-rated L; REQ-DSP-05's nominal 108 mA |
| R14 10k to 47k (C25792), R43 100k to 220k (C25767) | design.py | VER-177 still FAIL (9.1 V against 20 V); DEF-50 OPEN (the BOM half is in, Alex's value is not: O-R1); DF-101, REQ-PWR-17; VER-52's TP1 band (9.19-9.81 kohm) |
| RN1-RN4 10k to 47k (C2028, 4D02WGJ0473TCE, 5%) | design.py | DEF-49 OPEN (VER-45 needs a bare board; DF-164's acceptance is Alex's: F-R1); DF-164; REQ-BTN-01 now 15 kohm |
| TH1 0402 B3950 (C95939) to 0805 B4460 (TCTR0805F10K0F4460T, C525375) | design.py | VER-32 still FAIL (48.2 C); DEF-48 OPEN; DF-147; REQ-BAT-03, REQ-THM-03 (0.46 V), VER-68; tempwin reads TH1 from design.py now |
| U10 (MCP6001T-I/OT, C116490) and C54 (100n) new; net VBAT_DIV new; VBAT_SENSE is now U10's output | design.py, pin_review/u10.json | the cutoff's leakage term fixed (VER-138's leakage line); DF-182 new (H); DF-95 RETIRED in 40281b8; DEF-63 new; DF-98, DF-138, REQ-BAT-12, REQ-PWR-07, VER-65, VER-159; u4.json pin 58's waiver rewritten for U10 (DEC-29) |
| J4 PicoBlade 53261-0271 to JST SH SM02B-SRSS-TB (C160402) | design.py, pin_review/j4.json, case.scad | the headers keyed apart by footprint; VER-184 (the cell's plug) BLOCKED on the cell's spec sheet, VER-202 (the speaker's plug) FAIL (F-R2); DF-170 re-rated M; DF-183 and DEF-64 new (the speaker's SH plug, by hand, not bought); VER-128, VER-89, DF-118 |
| The cell's hole rotated and grown: 54.0 x 37.0 to 37.8 x 54.8 mm, the cell along the board, 1 mm towards seat 2; the pocket 0.9 mm inside it (1.9 mm at the lead end, cell_pocket_box) | placement.py CUTOUTS, case.scad | VER-181 FAIL on the fit alone (gaps exactly met); DEF-53 and DEF-58 FIXED; DEF-62 new; DF-85, DF-04; REQ-BAT-08's fit clause (PROPOSED); cellgap, swell and th1 read the new geometry |
| J2 to the right strip by the hole's seat-2 end (131.55, 69.7); TH1 to (119.95, 79.0); JP5, TP3 moved with the hole | placement.py ADOPTED | VER-164 FAIL (TH1 1.05 mm from the edge, 1.95 mm from the cell); DEF-37; VER-18's J2 courtyard (0.08 mm past, pads and body inside) |
| The power latch (Q1, R16, D2, R13, R42) to seat 1's end; seat 2's Start row (Q7, D8, R62, R15, C47, R63, TP6) to end2_l; PWR_BTN_RAW pre-routed by the maze (117 mm) | placement.py, route.py | netlist unchanged (VER-01 PASS); DF-184 new (the long, 33k-loaded PWR_BTN_RAW); VER-33 (the latch table) and VER-79 detect it, VER-179 reads the new lengths when built |
| The core moved 2.2 mm left (CORE_HOME 73.8), R28/R29, TP11-TP14, U3, JP1, C1, R41, C10 and others re-placed | placement.py | VER-142 PASS (nearest MLCC C47, 8.45 mm from H2); VER-23, VER-108, VER-158 when built |
| The board's copper: edge clearance 0.31 mm, a 0.30 mm copper-to-NPTH rule (battler.kicad_dru, new), routed again (0 DRC errors, 0 unconnected) | build_pcb.py, battler.kicad_pro, battler.kicad_dru | VER-04 PASS (6 silk warnings read by hand: DEF-66); VER-18 PASS in the data (0.308 mm), 0.108 mm with the 0.2 mm outline tolerance: DEF-56 reopened (O-R2); DEF-60 FIXED; battler.kicad_dru added to the baseline |
| fab.py: separate PTH and NPTH drill files; the CPL carries each footprint's pads' centre | board/tools/fab.py | VER-131 PASS, DEF-32 CLOSED; VER-176 PASS, DEF-33 FIXED (JLC's preview still the reference: O-R7); F-S3's fab.py edits came with the board pass |
| silk.py: the board's name, BOARD_REV and build line on clear spots; BOOT and RST at the pinholes; nothing in the speaker's hole | board/tools/silk.py | VER-18 PASS; DEF-57 FIXED (closes on VER-109); the JLC marker still missing (DEF-55) |
| The dome footprint's centre pad 1.6 to 1.2 mm, the vent at r 1.15 (0.30 mm from both pads) | board/lib, fp_dome.py | DF-168, VER-178 (BLOCKED on the dome's drawing) |
| case.scad: the panel's outline 54.56 x 82.84, neck 4.47, active-area border 2.22 from the drawing; fpc_low marked ESTIMATE; J4's body 3.0 mm tall | case.scad | DEF-59 FIXED; VER-152; O-S8 |
| tools/build.py and placement_zones.py (the placement search) | tools/build.py, board/placement_zones.py | both in the design baseline now (O-S4), with battler.kicad_dru, fp-lib-table and sym-lib-table |
| bom.md's changes table; the speaker's SH plug 'not yet ordered'; the panel still 54.50 x 83.00 x 2.30 | docs/bom.md | DEF-21 (u), DEF-64 |
| The board 151.64 to 151.48 mm long (the panel's 82.84 mm outline through bay_l and board_l), 73.4 mm wide across J1's tab, the case 156.48 mm | case.scad, Edge.Cuts | REQ-FAB-11, REQ-PRD-09, VER-90 and VER-166 at 151.48 (O-R3); bom.md's order line and cost table DEF-79; SHELL.md, CELLCUT_FOR_CASE.md and ref_design/README.md DEF-21 (y)-(aa) (O-R4) |

What stayed: the charger, the LDO, the panel and its J3 map (the display bus is still SPI:
DEC-13), the cell connector J2's part, the backlight circuit but R61, the audio chain, the
shift registers, J1 and its ESD array, the copied core's parts.

## (c) The board at 71737ae: what the checks find

Every number below comes from `tools/design_numbers.py` and the board's own checkers run on
71737ae's design files (evidence `evidence/2026-10-27_*` at 737e54e) unless it says otherwise.

**The 13 problems the register sent to the board session at be437f8:**

| # | Problem at be437f8 | At 71737ae | Verdict |
|---|---|---|---|
| 1 | Cell fit: -0.15 mm after tolerances on every side | gaps exactly met: 0.50 mm on the long sides and the seat-1 end, 1.50 mm at the lead end, after 0.40 mm of tolerances; but the pocket is 52.00 x 36.00 mm, the worst-case cell's own size: 0.00 mm spare against 0.40 (O-S1). This hole takes a cell of at most 51.60 x 35.60 mm; the worst-case cell needs 55.20 x 38.20 mm (VER-181 FAIL) | STILL FAILING (on the fit, DEF-62) |
| 2 | Charge window reaches 51.5 C; needs B >= 5130 | TH1 B4460: 3.9-48.2 C at the worst corner, 5.8-44.5 C nominal; B 5130 still needed and no stocked 10k NTC has it (VER-32 FAIL) | STILL FAILING (48.2 C against 45 C) |
| 3 | Cutoff can fail from GPIO47 pad leakage | U10 buffers VBAT_DIV: the temperature-varying error is +-0.87 mV of VBAT (was +-0.470 V), on pessimistic stand-ins until the MCP6001's datasheet is read. VER-138 still fails on its other two terms, unchanged: the CV calibration (3.465 V) and the dropout knee (3.521 V) | FIXED (the leakage term) |
| 4 | Drill file mixes plated and unplated holes | battler-PTH.drl 738 holes and 4 slots, battler-NPTH.drl 12 holes; all 750 board holes in the right file (VER-131 PASS) | FIXED |
| 5 | CPL offsets J1 1.50, J2 0.95, J3 0.95, J4 0.23 mm (the brief's order; at be437f8 it was J3 0.23, J4 0.95) | fab.py writes the pads' centre: 120 rows, none over 0.10 mm (VER-176 PASS); JLC's preview still decides (VER-28, O-R7) | FIXED |
| 6 | Copper 0.20 mm from routed holes | 0 features under 0.30 mm in the data; the closest 0.308 mm (a GND pour by the speaker's hole), 0.108 mm with the 0.2 mm outline tolerance; J2's courtyard 0.08 mm past the edge with its pads 0.43 mm and body 0.78 mm inside (VER-18 PASS) | FIXED in the data; DEF-56 reopened until JLC's dated page says which the rule is (O-R2) |
| 7 | Silk in holes; BOARD_REV in the cell hole | every silk text on the board, none in a hole (VER-18 PASS); two legibility warnings remain (DEF-66) and the JLC marker is missing (DEF-55) | FIXED |
| 8 | Backlight 122.4 mA against 120 mA | R61 3.74k: 117.7 mA worst case, 108.2 mA nominal, 19.6 mA an LED (VER-137 PASS) | FIXED |
| 9 | The cell plug fits J4 | J4 is a JST SH SM02B-SRSS-TB (1.00 mm), J2 the PicoBlade (1.25 mm): keyed apart by footprint. The cell's plug is the seller's (VER-184 BLOCKED on the spec sheet); the speaker's 1.25 mm plug does not fit J4 and its SH refit is not bought (VER-202 FAIL, DEF-64) | FIXED on the board; the plugs are open (F-R2) |
| 10 | VBUS above 6.9 V overstresses U3's EN | R14 47k, R43 220k: U3's EN at 9.1 V, U2's EN1 at 12.4 V, against 20 V (VER-177 FAIL); EN1 still high down to VBUS 3.6 V | STILL FAILING (9.1 V against 20 V; Alex sets the accepted value) |
| 11 | SPI caps a full frame at about 190 ms | not changed, as agreed: 184 ms at 20 MHz, 197 ms at 18.75 MHz (DF-180, REQ-DSP-11) | STILL FAILING (not being fixed: DEC-13) |
| 12 | Panel supply up to 3.366 V against 3.3 V | not changed: 3V3 through JP4 (DEF-61, DF-96) | STILL FAILING (proposed: accept, DEC-09) |
| 13 | case.scad's panel numbers differ from the drawing | outline 54.56 x 82.84, neck 4.47, border 2.22 now the drawing's; panel_t 2.45 kept on purpose (the drawing's 2.30 maximum plus 0.15); bom.md still says 54.50 x 83.00 x 2.30 (DEF-21 (u)) | FIXED in case.scad (bom.md stale) |

**What else the re-run finds at 71737ae:**

| Finding | Numbers | Where |
|---|---|---|
| **The pocket has no fit for a worst-case cell** (problem 1's new form) | 52.00 x 36.00 mm pocket for a 52 x 36 cell; a cell of at most 51.60 x 35.60 mm by its spec sheet, or a 55.20 x 38.20 mm hole (copper) | DEF-62, DF-85, VER-181, DEC-27 |
| **U10 went on the board without its datasheet** | u10.json's pin table is KiCad's symbol (UNVERIFIED); off, its input sources about 6.8 uA into the unpowered 3V3 and R17 draws 7.9 uA (was 4.5) | DEF-63, DF-182 (H), VER-34 BLOCKED |
| **The speaker's plug no longer fits J4** | the GSPK151035PN's 1.25 mm plug must be cut off and an SH 2-pin plug fitted by hand; not bought | DEF-64, DF-183, VER-128 |
| **TH1 sits further from the cell** | the 0805 is 1.05 mm from the hole's edge (VER-164's limit 1.0) and 1.95 mm or more from the cell's face (the pocket's 0.9 mm inset); at be437f8 0.80 and about 1.05 mm | VER-164 FAIL, DEF-37, DF-04, DEC-25 |
| **J2's '+' overlaps its 'BATT' label; 'PANEL' sits on U4's pads 45-47** | kicad-cli DRC's silk_overlap at (132.2, 73.8) and three silk_over_copper on U4 | DEF-66, VER-04 |
| The JLC order-number marker | still in neither silk.py nor the board | DEF-55, DEC-28 |
| The dome's centre pad is smaller | 1.2 mm (1.6 before): 0.3 mm less margin for the dimple under hand placement | DF-168, VER-178 |
| The swelling check's model was stale | it put the ends' 1.9 mm ceiling over the cell's seat-2 end; case.scad's uni_ceil puts the slope's inside 3.40 mm over the board there. With the real ceiling: 0.80 mm under the panel (PASS) | DEF-65 CLOSED, VER-182 |
| Unchanged | screws M2 x 6 give 3.2 mm in the side pilots (VER-147 FAIL, CASE); bom.md's domes and Start/Select parts (VER-167 FAIL, CASE); 21 of 22 doc items stale (VER-39 FAIL, CASE); inrush 10 uF (VER-99 PASS); every hole 0.30 mm or more (VER-114 PASS); nearest MLCC 8.45 mm from H2 (VER-142 PASS); swelling 0.80 mm (VER-182 PASS) | as named |

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

The order gate holds only what the copper, the BOM and the fab files freeze, or what would
make the five boards useless or unsafe. A BOM decision Alex may accept for EVT is held here by
its defect, while its check binds at EVT. Every ORDER check is named in a numbered step below
(check_trace fails otherwise). Effort is Claude's unless it says Alex. Since 71737ae the board
session has made eight of the BOM and fab changes this plan asked for (section c), so step 1 is
now short: one line per decision, in the order they must be made, those that can move copper
first (O-S9).

| # | Step | IDs | Effort |
|---|---|---|---|
| 0.1 | **Alex confirms 71737ae as the base** the register describes (DEC-01). If step 1 changes a design file (1.1 (b), 1.2 (b), 1.4), the order candidate is the commit after that pass, named by DEC-35 and the release record, and the baseline moves to it (O-S3). A later commit to main that touches a design path makes the default mode warn and the order gate fail (R8). | DEF-30 | Alex: 5 min |
| 0.2 | **Order everything with a lead time** (`--shopping --gate evt`): RP2354B and L2 pre-order at JLC; cells from a seller with a datasheet that guarantees 51.6 x 35.6 mm or less (DEC-23) and names the plug, a PicoBlade (MX1.25) for J2, recorded in analysis/cell.json for VER-184 (F-R2); the ER-TFT035IPS-6, an FH12 breakout, an RP2350B dev board; two GSPK151035PN, an SM02B-SRSS-TB sample and pre-crimped SH 2-pin leads (or SHR-02V-S housings, SSH-003T-P0.2 contacts and a crimper: DEF-64); domes and Start/Select strips; calipers, a 4.096 V reference, clip leads, a LiPo bag, a USB-C breakout, a DMM thermocouple; the CV load (160 ohm and 22 ohm 0.25 W 1%, a TL431 in TO-92, 10k and 24.9k 1%). **Download four datasheets** into hardware/ref_design/datasheets: MCP6001 (U10), TCTR0805 (TH1), JST SH (J4), HYP 600-415S-260 (the domes); VER-34 and VER-178 are BLOCKED until then (DEC-37). | VER-41, VER-37, VER-184, VER-43, VER-34, VER-152 | Alex: 1 h, about $120-170 |
| 0.3 | **Run the register on every commit**: one line in scripts/check.sh, `python3 hardware/vv/tools/check_trace.py` (outside hardware/vv, so it waits for Alex's OK). | DEF-47 | 15 min |
| 0.4 | **Alex pins a signing key** (README, 'Approvals'). Until then no APPROVALS.md line counts (DEC-30). | | Alex: 15 min |
| 0.5 | **prod_board's own order list** (`--gate order --board prod_board`, Alex's board, evaluated apart). | VER-10, VER-119, VER-120, VER-121, VER-122, VER-163 | Alex: 1 h |
| 1.1 | **DEC-27 with DEC-23, the cell and its pocket** (can move copper): (a) buy a cell whose spec sheet guarantees 51.6 x 35.6 mm or less with its PCM, no copper; (b) the hole to 55.2 x 38.2 mm, a re-route by the hole. Proposed: (a) if a seller publishes it, else (b). | VER-181, VER-37 | Alex: 15 min; (b) a design pass and a re-route, about half a day |
| 1.2 | **DEC-13, the display bus** (can move copper): SPI with REQ-DSP-11 at 220 ms, or the 8080 bus on J3's pins 19-26. Proposed: SPI for EVT. | VER-43 | Alex: 10 min |
| 1.3 | **DEC-22, a fuse in CELL_P** (copper if now). Proposed: the PCM alone for the five attended EVT units; a 1206 fuse at DVT, or now if 1.1 (b) re-routes anyway. | | Alex: 5 min |
| 1.4 | **DEC-28 with DEF-66, one silk.py pass** (no copper; a rebuild and a new baseline): the 'JLCJLCJLCJLC' marker on a clear back spot, J2's '+' 0.15 mm off 'BATT', 'PANEL' off U4's pads. Proposed: yes, with any 1.1 (b) or 1.3 pass. | VER-18 | the design pass, 1 h |
| 1.5 | **DEC-25, TH1's coupling** (worse at 71737ae: 1.95 mm from the cell): a 0.5 mm silicone pad across the gap (case side), or TH1 0.1 mm towards the edge in a design pass. Proposed: the pad, decided with 1.1. | VER-164 | Alex: 5 min |
| 1.6 | **DEC-05, the charge window's 48.2 C hot edge**: no stocked part reaches 45 C. Proposed: accept DF-147 for EVT (attended charging in the LiPo bag), or REQ-BAT-03 restated to the cell's range if VER-37 shows 50 C or more. | DEF-48 | Alex: 5 min |
| 1.7 | **DEC-17, REQ-PWR-17's accepted value**: 9.1 V with the BOM change in, 20 V needs a clamp. Proposed: 9.1 V for EVT, with an ACCEPTED line for DF-101 scoped EVT; DEF-50 is OPEN and holds the order until then (O-R1, F-R1). | DEF-50 | Alex: 5 min |
| 1.8 | **DEC-09, the panel's VCI and VDDI at 3.366 V**. Proposed: accept (absolute 4.6 V, the ILI9488 to 3.6 V). | DEF-61 | Alex: 5 min |
| 1.9 | **DEC-07, the cutoff's two remaining terms**: REQ-TST-06's bench calibration at 3.50 V and a firmware dropout rule (both EVT). Proposed: yes, recorded by approving REQ-BAT-12. | DEF-40 | Alex: 5 min |
| 1.10 | **DEC-35, the order candidate**: if 1.1 (b), 1.3 or 1.4 changed files, the commit after that pass; route.py --reuse and the committed last_route.ses for any rebuild that moves no copper (O-S2). | DEF-30 | Alex: 5 min |
| 1.11 | **DEC-24, the pill margin with 47k pull-ups**: RN1-RN4 47k are in (about 17 kohm against REQ-BTN-01's 15 kohm), and VER-45 needs a bare board from the order. Proposed: accept DF-164 for EVT, so DEF-49 stops holding the order (O-R1, F-R1: it is OPEN again). | DEF-49 | Alex: 5 min |
| 2 | **Re-run the scripted checks** after any step-1 pass (`run_checks.py`, about 10 min, then VER-129 on its own): netlist, pin gate, ERC, DRC with every warning type and count read by hand (DEF-36, DEF-66), contact footprints, inrush, holes, MLCC, TH1, CPL, drill, VBUS, cutoff, backlight, charge window, cell gap, edges, the net classes (VER-201, O-R8: FAIL today on DEF-68's root cause) and the stale-names scan. | VER-01, VER-02, VER-03, VER-04, VER-08, VER-99, VER-114, VER-142, VER-164, VER-176, VER-131, VER-137, VER-181, VER-18, VER-201, VER-129, VER-188 | 30 min |
| 3 | **Fab outputs**: the drill files and the CPL now pass (VER-131, VER-176); no CPL row moves before the dry run's placement preview, the reference for J1-J5's origins (O-R7), and any correction goes through a JLC_FIXED entry naming its committed evidence (F-R4); the fab content check over the exported files (the combs' paste, mask and keep-outs, the plated slots, the vents, the JLC marker, every silk text legible); JLC's dated limits, which also decide DEF-56 (is 0.30 mm to the edge a data rule or an as-built one: O-R2); land patterns; contacts as placed; the dome footprint against its drawing (BLOCKED on it); the release run that rebuilds the board twice with route.py --reuse, re-runs the DRC, diffs the copper and makes the files twice (VER-143: O-S2, F-S2). The order sends no netlist; VER-26 and VER-29 are advisory here and bind at G-DVT-01. | VER-131, VER-176, VER-132, VER-17, VER-125, VER-162, VER-178, VER-143 | 1.5 days |
| 4 | **Layout reviews**: the copied core laid over Raspberry Pi's RP2350B minimal design by hand (the second 4.7 uF on +1V1 in writing; the core moved 2.2 mm left at 71737ae); the USB pair's stubs; the GPIO mux and PWM slices, its evidence printing the whole slice table (F-R11). | VER-23, VER-108, VER-15 | half a day |
| 5 | **Power-path analyses that can move copper or the BOM**: the latch table over both Starts (the latch now at seat 1's end, PWR_BTN_RAW 117 mm long); the backlight (passes); inrush (passes). Alex reads by hand the four back-power paths DF-98 names (VBAT_DIV into U10's input, SWD, the panel's SPI with JP4 cut, D+); the script is VER-159, at EVT. | VER-33, VER-137, VER-99 | half a day |
| 6 | **Datasheets and part identity**: the application circuits and every constant this register quotes, from the files in hardware/ref_design/datasheets (the on-disk ICs by script since 2026-10-27, VER-198), U10, TH1 and J4 first once downloaded (DEC-37); the panel's VCI and VDDI in writing (DEF-61); every line of JLC's matched-parts page against design.py, by hand (it also covers what VER-130's script would: O-S6); the watchdog rule and FM8002A polarity (both pass). | VER-34, VER-198, VER-14, VER-104, VER-124 | 1 day |
| 7 | **The board against the case**: the XY chain from positions.json; placement drift against placement.py and positions.json (VER-19 stays: VER-143 compares two builds of the same scripts, so a placement or positions error passes it: O-S6); check_fit; the MLCC keep-out (passes) and TH1 (fails at 1.05 mm, step 1.5); the ESTIMATE ledger with fpc_low in it (O-S8), the cell from its spec sheet (F-S1). | VER-174, VER-06, VER-19, VER-142, VER-164, VER-152 | 1 day plus calipers |
| 8 | **The panel and the cell, parts in hand**: the ER-TFT035IPS-6 lit through the breakout, its header numbering by diode and continuity tests (F-R1); ID4 on SDO, or SDA if SDO is silent (F-R8); its redraw timed at an in-spec clock; the FPC folded into a breakout in J3's place; the tail's thickness; the cell's spec sheet. cell.json and its assert (VER-160) moved to EVT (O-S6); the cell's incoming inspection (VER-49) is EVT's. | VER-43, VER-136, VER-44, VER-37 | Alex and Claude: an evening each once parts arrive |
| 9 | **Test access and firmware size**: every bring-up probe point has copper (the probes moved at 71737ae: TP3, TP6, TP11-TP14); the game's llvm-size inside 2 MB. | VER-158, VER-151 | 2 hours |
| 9.5 | **The coverage grid's gaps** ([grid.md](grid.md), R9): the pre-order checks the grid planned: each subsystem's requirements read against its functions (VER-189), the case's architecture (VER-190), the USB data path's corners (VER-191), the USB connector's fault reach (VER-195), the amplifier's states (VER-196), the copied core's values against rp2350.pdf's own rules (VER-197, built and run 2026-10-27: FAIL, DEF-67), and the paper heat estimate run to a number (VER-144, VER-193), the clocked buses' timing (VER-199, new 2026-10-27). R9 also holds every cell whose only clearing checks are NOT_BUILT or BLOCKED: build or unblock them, or accept the gap, which needs two things: Claude adds the cell's ACCEPTED_GAP row with its reason to grid_reasons.csv, and Alex signs the GRID-<class>-<subsystem> line. Either alone leaves the cell a GAP (grid.py lists a signed line with no row). | VER-189, VER-190, VER-191, VER-195, VER-196, VER-197, VER-144, VER-193, VER-199, VER-200 | 2 days; the later-phase analyses R9 holds, about 3 days more, or Alex's acceptances: 30 min |
| 10 | **Reviews** (advisory here): the DFMEA review binds at G-EVT-16 (VER-98); the clean-room, return-path and gerbv reviews if time allows (VER-40, VER-21, VER-27). | VER-98 | Alex: half a day, optional |
| 11 | **Alex's decisions** in APPROVALS.md, in signed commits: one `REQ-ALL@<commit>` line; the H rows only built units can detect, accepted for ORDER (`--gate order --verbose` lists them); DF-147, DF-101 and DF-164 for EVT (steps 1.6, 1.7, 1.11), each with its defect's `DF-ACCEPT:DEF-nn` line to EVT (DEF-48, DEF-50, DEF-49) and that defect's Status ACCEPTED, since a DF-nn line alone moves no defect and no check (2026-10-28); u4 pin 58's waiver, now for U10's input (DEC-29); one `GATE@<commit>` line after reading G0's list of moves (O-S5). | R1, R3, G1 | Alex: 1 h |
| 12 | **Release and order**: the release record; the dry run (valid 7 days); stock on order day (valid 1 day); `check_trace.py --gate order` exits 0; order 5 assembled and 5 bare boards, typing the Gerbers' extent, 73.4 x 151.48 mm (72.4 wide without J1's tab), not bom.md's 151.64 (DEF-79, O-R3). | VER-42, VER-28, VER-41 | 1 day |

Rough total: about 5.5 working days of Claude's work (6 before this re-seed: the drill, CPL,
copper and silk fixes are in, and O-S6 moved VER-130 and VER-160 to EVT), 3-4 hours of Alex's,
half a day of design pass if step 1.1 (b) or 1.4 is taken, and up to three weeks of parts lead
time in parallel.

**What moved where on 2026-10-27** (earlier moves: [HISTORY.md](HISTORY.md), section d):

| Check | Was | Now | Why |
|---|---|---|---|
| VER-130 the BOM, CPL and PCB fields script | ORDER (G-ORD-02) | EVT (G-EVT-08) | round 3's O-S6: VER-14 by hand, VER-28's matched-parts page and VER-132 (d) see the same fields before the order |
| VER-160 cell.json and its assert | ORDER (G-ORD-03) | EVT (G-EVT-05) | O-S6: the only value the order freezes is R5's, which VER-37 checks against the spec sheet |
| VER-19 placement drift, VER-162 contacts as placed | ORDER | ORDER (kept) | O-S6 proposed folding them into VER-143; VER-143 compares two builds of the same scripts, so an error in placement.py or positions.json passes it, and VER-162 reads the combs as placed |

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
its display steps are the 2.4 in panel's and its power steps the TP4056's, DEF-21 (a)-(d),
(n)), the gear is on the bench (G-EVT-16), and pill resistance and comb leakage are measured
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
a load (VER-145, 6.5 h: the timer is 4-6 h) and SR_CLK (VER-179, its track length read off the 71737ae board when it is built); units 2 and 3
bring-up, soak and thermal; unit 4 abuse, ESD screen, near-field, drop, button life and fault
injection (it never ships); unit 5 spare.

Exit: `--gate evt` exits 0 (G-EVT-01 to G-EVT-16; G-EVT-15 is prod_board's).

### DVT: the revised spin

Candidates, decided from EVT's results. BOM-only (each bumps BOARD_REV): 4.7k pull-downs on
AMP_EN and BL_PWM (DF-45, DF-47); U10's part or its input's protection if VER-34 or VER-139
finds them short (DF-182, DEC-07); the BOM changes the plan asked for (R61, R14, R43, RN1-RN4)
and J4 keyed apart from J2 are in at 71737ae. Copper: a VBUS clamp for 20 V (DF-101, if
REQ-PWR-17 keeps 9.1 V for EVT); the 55.2 x 38.2 mm hole if DEC-27 chose a cell spec and
incoming cells crowd it (DF-85); J2's '+' and the JLC marker if not done in step 1.4;
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

Each goes into releases/APPROVALS.md (requirements, acceptances, deferrals with a scope,
waivers) or into a commit. Claude's proposal follows each. Step 1 of section (d) lists the
ones before the order in the order to make them.

**Done in the board spin (40281b8), recorded here so the IDs stay:** DEC-31 (R61 3.74k),
DEC-24 (RN1-RN4 47k), DEC-32 (0.31 mm to the edge, 0.30 mm to NPTH, J2 moved), DEC-33 (J4 a
JST SH), DEC-17's BOM half (R14 47k, R43 220k), DEC-07's copper half (U10), DEC-28's texts
(moved out of the holes; the marker is not in: 1.4), DEC-05's part search (B4460, the highest
stocked: 1.6).

**The design and the order**

- **DEC-01. Is 71737ae the base the order is made from?** Proposed: yes; the order candidate
  is 71737ae itself, or the commit after the last step-1 pass (DEC-35, DEF-30, O-S3).
- **DEC-02. Are the RP2354B and L2 pre-ordered?** Both were out at JLC and LCSC on 2026-10-21
  and are on the no-substitution list.
- **DEC-03. Will any of the 30 be sold or given away?** That decides the SALE items.
- **DEC-35. The order candidate after a design pass**: any pass that moves no copper rebuilds
  with route.py --reuse and the committed last_route.ses, so VER-143 can compare copper; a pass
  that moves copper commits a new last_route.ses with the board (O-S2).
- **DEC-37. Four datasheets** (new, 2026-10-27): MCP6001 DS20001733L
  (https://ww1.microchip.com/downloads/en/DeviceDoc/MCP6001-1R-1U-2-4-1-MHz-Low-Power-Op-Amp-DS20001733L.pdf),
  Resistor.Today TCTR0805F10K0F4460T (https://www.lcsc.com/product-detail/C525375.html), JST SH
  (https://www.lcsc.com/datasheet/lcsc_datasheet_2304140030_JST-SM02B-SRSS-TB-LF-SN_C160402.pdf)
  and the HYP 600-415S-260 dome (LCSC C256258) into hardware/ref_design/datasheets. They cannot
  be downloaded from this session; VER-34 and VER-178 are BLOCKED on them.

**Before the order: copper, silk or BOM**

- **DEC-27 with DEC-23. The cell's pocket** (DEF-62, DF-85, VER-181): the gaps are met
  exactly; the pocket has no fit. (a) a cell of 51.6 x 35.6 mm or less with its PCM by its
  spec sheet; (b) a 55.2 x 38.2 mm hole. Proposed: (a) if sold, else (b) in the same pass as
  1.4.
- **DEC-13. The display bus** (DF-180, REQ-DSP-11): unchanged. Proposed: SPI for EVT,
  REQ-DSP-11 at 220 ms for a full frame and 50 ms for a text update.
- **DEC-22. A fuse in the cell path** (DF-172). Proposed: the PCM for EVT; a fuse at DVT.
- **DEC-28 with DEF-66. silk.py**: the JLC marker, J2's '+' apart from 'BATT', 'PANEL' off
  U4's pads. Proposed: one pass, no copper.
- **DEC-25. TH1's coupling** (DF-04, DEF-37): 1.95 mm from the cell now. Proposed: a 0.5 mm
  silicone pad bridging the hole's edge to the cell's long side (case side), decided with
  DEC-27.
- **DEC-05. The charge window** (DEF-48, DF-147): 3.9-48.2 C at the worst corner with B4460.
  Proposed: accept DF-147 for EVT, charging attended in the LiPo bag; at DVT a cell with an NTC
  lead.
- **DEC-17. The VBUS reach** (DEF-50, DF-101): 9.1 V with the BOM change. Proposed: 9.1 V as
  REQ-PWR-17's accepted value for EVT; a clamp only if the DVT copper moves anyway.
- **DEC-07. The cutoff** (DEF-40, REQ-BAT-12): the leakage term is fixed by U10; the CV
  calibration and the dropout knee remain. Proposed: REQ-TST-06's bench calibration at 3.50 V
  and a firmware rule that a reading which stops falling under a steady load is dropout.
- **DEC-11. E9 pull-downs**: 4.7k on AMP_EN (R37) and BL_PWM, or rely on firmware and the
  stepping. Proposed: wait for the stepping.

**Requirement numbers** (all live ones are PROPOSED; one signed `REQ-ALL@<commit>` line
approves those Alex agrees with)

- **DEC-04.** Play time 4 h; charge time 4.0 h from 0.5 A; 100,000 presses; $45 a unit.
  Proposed: DEFERRED with scope EVT for REQ-PRD-03.R, REQ-PRD-04.R and REQ-BAT-02.R.
- **DEC-08.** Legacy USB hosts: U2 charges from plug-in inside 475 mA (REQ-PWR-12). Accept.
- **DEC-09.** The derating rule's exceptions: the panel's VCI and VDDI at up to 3.366 V (DF-96,
  DEF-61), and U10's input above its unpowered VDD when off (DF-182, once VER-34 reads its
  limits). Accept both.
- **DEC-12.** The backlight halves at the minimum duty: each sink within +-25% at 18% (+-21.0%
  at 71737ae). Accept.
- **DEC-14.** Flash budget 1.6 MB; off current 60 uA at 45 C; lot capacity 600 mAh; the
  BQ24073's 5 h timer as the charge timeout (REQ-BAT-13).
- **DEC-15.** USB input low end: run from 4.40 V with a cell, 4.75 V without. Decide after
  VER-80.
- **DEC-16.** prod_board: which product requirements apply to your board (the .P allocations).
- **DEC-38.** REQ-BAT-08's fit clause (new, 2026-10-27, O-S1): the pocket 0.1 mm a side
  larger than a worst-case cell after the print's 0.1 mm, 0.4 mm in each direction. Proposed:
  approve with REQ-ALL.

**Before the case release**

- **DEC-18. Screws** (DEF-31, VER-147): M2 x 7 in the four side pilots, M2 x 6 in the two end
  pilots. Or accept 3.2 mm at the sides for EVT.
- **DEC-26. Opening the case** (DF-169): assembly.md unplugs J2 before the tub lifts away
  (still not so at 71737ae: DEF-21 (j)).
- **DEC-39. The speaker's plug** (DEF-64, DF-183, new): pre-crimped SH leads joined to the
  speaker's, or SH housings crimped by hand. Proposed: pre-crimped leads, bought with the
  speakers.

**Risk acceptances, waivers and signing**

- **DEC-19.** The H rows that only built units can detect need Alex's acceptance for ORDER
  (R3); `check_trace.py --gate order --verbose` lists them. DF-87 through DVT; DF-147 and
  DF-101 for EVT with DEC-05 and DEC-17; DF-180 with DEC-13; DF-182 once VER-34 has read U10.
- **DEC-21.** DVT: reverse-polarity protection or a keyed connector on CELL_P (DF-55).
- **DEC-29. u4.json pin 58's waiver** now covers U10's input (the device off, VIN+ at VBAT/2
  through 235 kohm); it was written by the design session, not by Alex. Decide with DEC-09.
- **DEC-30. A pinned signing key**: an ed25519-sk key that wants a touch.
- **DEC-34. Freeze the gate state**: one signed `GATE@<commit>` line after reading G0's list of
  moves since b9b38a9 (O-S5).

Retired decisions: DEC-06 (no gap with the BQ24073), DEC-10 (folded into DEC-07), DEC-20
(W-01 and W-02 not needed). DEC-24, DEC-31, DEC-32 and DEC-33 are done (above).

## (g) Round 3's sign-off objections and where each landed

O declined (2 major, 7 minor) and F approved with 3 minors at be437f8. Each is fixed below or
the row says why not.

| # | Objection | What was done | State |
|---|---|---|---|
| O-S1 major | DEC-27's passing geometry left the cell no fit in its pocket | design_numbers.py cellgap now checks the fit: the pocket 0.4 mm larger than the cell in each direction (0.1 mm print and 0.1 mm fit a side), REQ-BAT-08's new fit clause (PROPOSED, DEC-38); seeded twice (a 51.7 mm cell fails, 51.6 passes). The board at 71737ae built exactly the flaw O named (a 52.0 x 36.0 mm pocket): VER-181 FAIL, DEF-62, DEC-27 re-asked with 51.6 x 35.6 mm or a 55.2 x 38.2 mm hole | fixed in the register; the board's fix is DEC-27 |
| O-S2 major | VER-143 could not pass: a rebuild runs a timed Freerouting race | VER-143 rebuilds twice with route.py --reuse and the committed last_route.ses, compares the two and the committed copper; if --reuse proves not repeatable, the limit falls back to positions, netlist connectivity and DRC, recorded here; DEC-35 asks every copper pass to commit its last_route.ses | fixed (the script itself is still NOT_BUILT) |
| O-S3 minor | the order candidate described two ways | 71737ae is the base the register describes; the order candidate is 71737ae or the commit after the last step-1 pass, named by DEC-35 and the release record (step 0.1, G-ORD-19, DEF-30); the DEC-01 citation that meant DEC-35 is gone with the old section (c) | fixed |
| O-S4 minor | tools/build.py, fp-lib-table and pin_review outside the watched paths | BOARD_FILES now holds tools/build.py, battler.kicad_dru (new at 71737ae), fp-lib-table, sym-lib-table and placement_zones.py; README's baseline names them; a pin_review change stales VER-02, which covers it, so it is not a design path; seeded (the baseline missing build.py fails) | fixed |
| O-S5 minor | a Gate or phase moved later before any GATE@ line is seen by no one | G0 at the order gate lists every check, defect, criterion or requirement moved later since README's 'Gate moves listed since' commit (b9b38a9, the state round 3 read), until Alex signs GATE@; seeded (VER-138 moved to DVT is listed). The first option (a signed DF acceptance per moved defect) is what step 11 asks Alex to sign anyway | fixed (the listing option) |
| O-S6 minor | about 11 new scripts, some overlapping | VER-130 and VER-160 moved to EVT (gates.md, section d's table); VER-19 and VER-162 kept, because VER-143 compares two builds of the same scripts and cannot see an error in placement.py or positions.json or in the combs as placed | applied in part, reasons in section d |
| O-S7 minor | DEF-14 pointed at a step the plan forbids | DEF-14 closes on VER-28's screenshot of U1, any correction only through a JLC_FIXED entry | fixed |
| O-S8 minor | fpc_low tracked by no check | case.scad marks fpc_low ESTIMATE at 71737ae; VER-152's pass limit names it | fixed |
| O-S9 minor | PLAN.md 848 lines, half ledgers; step 1 one cell of 12 decisions | the ledgers moved to HISTORY.md; step 1 is ten numbered lines in the order to decide, those that can move copper first, each with Claude's proposal | fixed |
| F-S1 minor | VER-152 needed cells in hand at ORDER | the cell's size is met at ORDER from its spec sheet (VER-37); the calipers on real cells moved beside VER-49 at EVT; 'cells (21d)' left VER-152's needs | fixed |
| F-S2 minor | VER-143 named route.py, not route.py --reuse | as O-S2; section d step 3 names --reuse | fixed |
| F-S3 minor | step 3's fab.py edits came after step 2's re-run; bom.md's third panel thickness | the board session made the fab.py edits in the board pass (40281b8), and step 2 re-ran on them (VER-131, VER-176 PASS); bom.md's 54.50 x 83.00 x 2.30 is DEF-21 (u) and named in DEF-59 | fixed in the register; bom.md is outside it |

## (h) What this plan still cannot catch

- **Small numbers.** Five boards, then 30, cannot show a failure rate under a few percent.
- **Time.** Cell ageing and swelling over years, ENIG wear, tape creep, slow flux migration.
- **Faults nobody listed.** The DFMEA names every part, but only the failure modes someone
  thought of; seeded faults prove a checker fails on the faults seeded.
- **Numbers this register quotes from memory.** The charger's, the ILI9488's and the RP2350's
  come from the files in hardware/ref_design/datasheets (committed with SOURCES.md since
  71737ae); JLC's 0.30 mm and 0.2 mm tolerances (VER-17) and the cell's size and PCM (VER-37)
  still do not, and four parts have no datasheet on disk at all: U10, TH1's TCTR0805, J4's SH
  header and the domes (DEC-37). U10's numbers in design_numbers.py are pessimistic stand-ins.
- **The panel's mechanics.** case.scad takes the outline, neck and border from the drawing
  since 71737ae, but the FPC's folded shape and fpc_low are still estimates: only a real panel
  (VER-43, VER-136, VER-152) can show J3's place is right.
- **The cell's fit.** The pocket's walls hold the cell off every edge since 71737ae, but only a
  cell under its spec sheet's maximum fits without pressing (DEF-62); a seller's spec sheet is
  a claim until VER-49 measures the lot.
- **The bought breakout's header numbering.** The product's panel-side pairing is 51-j, and
  the fold changes no contact pairing, but the loose breakout's own header numbering against
  the FH12's pads is unknown until VER-43's diode and continuity tests (F-R1).
- **A board the scripts cannot rebuild the same way twice.** The 71737ae board came from the
  placement search's route (seed 32 of 20-55); route.py's Freerouting race picks its winner by
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
  origins for J1-J5 agree, only the dry run's preview shows (O-R7).
- **Bench honesty, standards, JLC's side, firmware beyond the self-test, use**: as the first
  run listed them.
- **A design that moves again.** Another session may commit to main. Any change to a
  design path fails the default mode and every gate until its rows are re-checked; the
  register cannot re-check them itself.
- **Quoted values until VER-161 exists.** A GPIO, a count or a dimension quoted in a row can
  still drift from design.py unseen (DEF-54).

## (i) The gate today

**Re-run at 2b3bfc3 (2026-10-28)**, after the framework fixes (top of this page). Default mode:
STRUCTURE OK, exit 0, 7 warnings (the proposal commits among them). Seeded faults: 117 of 117
caught (VER-129 at 086da04). `--gate order`: FAIL, 24 of 29 not met (21 before). Three more fail:
G-ORD-06 and G-ORD-18, because VER-33 and VER-137 are now BLOCKED (their PASS rests on
2n7002_cj.pdf, bat54c.pdf, bat54ws.pdf and mcp6001.pdf, not on disk here), and R8, on dcb6a76 and
e79f251. G-ORD-24 passes again. 9 PASSes are BLOCKED (VER-31, 33, 100, 124, 137, 144, 191, 193,
196); G-ORD-22 counts 37 ORDER checks not passing; R9 holds 22 cells (12 before: the BLOCKED
PASSes no longer clear 4/buttons, 4/display, 4/usb_debug, 6/audio, 6/buttons and 15 for buttons,
case, display, mcu and usb_debug). No defect is
ACCEPTED: no key is pinned, so no DF-ACCEPT line can count yet. The RP2354B snapshot below stays as
it was.

**Run at 95a9d26 (2026-10-27)**, after the review's 16 findings (section l). The scripted checks'
evidence was made at 5bedcef, VER-129's at 69e9c0c. Requirements 133 (130 PROPOSED, 3 RETIRED);
DFMEA rows 184 (63 H, 86 M, 35 L); checks 201 (35 PASS, 27 FAIL, 54 NOT_RUN, 77 NOT_BUILT, 5 CUT,
3 RETIRED); defects 79 (38 OPEN, 24 FIXED, 10 CLOSED, 7 RETIRED); seeded faults 107 of 107
caught (VER-129). The re-seed's snapshot (c4fdd45) is in that commit's PLAN.md.

```
$ python3 hardware/vv/tools/check_trace.py
STRUCTURE: OK (5 warnings: four DMM pass bands under 4 x the meter's accuracy; VER-196's
'needs' still names the FM8002A's and 2N7002's datasheets, which VER-34 waits for)
exit 0

$ python3 hardware/vv/tools/check_trace.py --gate order
  PASS  G-ORD-06, G-ORD-15, G-ORD-18, G-ORD-24, R5, R6 (no release yet), R7, R8
  INFO  G-ORD-05, -08, -12, -20, -26 (advisory); G0: 12 moves since b9b38a9 (VER-130 and
        VER-160 to EVT; DEF-32 and DEF-39 OPEN to CLOSED; DEF-33, 53, 57, 58, 59 and 60 OPEN
        to FIXED, listed since O-R1; G-ORD-02 and G-ORD-03 without VER-130 and VER-160)
  RETIRED G-ORD-13; N/A G-ORD-25 (prod_board)
  FAIL  G-ORD-01 to -04, -07, -09 to -11, -14, -16, -17, -19, -21, -23, -27
  FAIL  G-ORD-22: 30 ORDER checks not passing
  FAIL  R1 127 PROPOSED; R2 22 requirements with an ORDER check not passing; R3 52 H rows
        neither accepted nor DONE
  FAIL  R4 21 OPEN defects due at ORDER: DEF-14, 29, 30, 47, 48, 49, 50, 55, 56, 61, 62, 63,
        66, 67, 68, 70, 72, 74, 75, 78, 79 (DEF-47, 49, 50 and 56 OPEN again, DEF-79 new);
        13 FIXED ones listed beside it (DEF-01, 02, 10, 11, 12, 13, 22, 33, 44, 53, 54, 57, 60)
  FAIL  R9 12 cells planned: class 1 for all eight subsystems (VER-189), 2/audio (VER-34,
        VER-40), 2/cell (VER-37, VER-40), 5/case (VER-192), 15/audio (VER-200)
RESULT: FAIL (21 of 29 not met); 1 N/A, 6 advisory, 1 retired
```

The coverage grid, classes 1-15 by the worst status in each cell (120 cells): PASS 15, FAIL 80,
NOT_RUN 2, NOT_BUILT 1, planned 12, not needed 10. Before this pass: PASS 15, FAIL 80, NOT_RUN 4,
NOT_BUILT 2, planned 9. O-R6 moved 2/audio and 2/cell to planned (their clearing analyses have
not run); 14/buttons is NOT_BUILT (VER-178, VER-180). G1 and K1 print nothing: no `GATE@` line
is signed and no key is pinned.

New results this pass: VER-201 FAIL (build_pcb.py's PWR class names VBUS_D5, no net of the
board, and leaves CELL_P and AMP_VDD out: DEF-68); VER-202 FAIL (the speaker's 1.25 mm plug does
not fit J4 and its SH refit is not bought: DEF-64; its 35 mm leads reach J4 at 16.4 mm and not
J2 at 54.8 mm); VER-184 BLOCKED (the headers are keyed apart, the cell's plug waits for its
spec sheet); VER-18 PASS in the data, the closest copper 0.108 mm from the edge as built
(DEF-56); VER-188 PASS with the connector-family rule.

## (j) Past bugs and what catches them

| Defect | What happened | Caught now by | When |
|---|---|---|---|
| DEF-01 | U1's REF on VBUS back-fed PWR_EN | VER-159, VER-127 | before the order, EVT |
| DEF-02 | R15 at 10k cut the pill margin | VER-33, VER-78 | before the order |
| DEF-03-06, 09 | the TP4056 power path's hand-over, hot plug and CE limits | RETIRED with those parts (2d1124d, 1fd0956); the BQ24073's are VER-12, VER-70-72 | EVT |
| DEF-07, 08 | J6 swapped; 22 ohm USB resistors | VER-02 (CLOSED) | every change |
| DEF-10 | no PWR_HOLD pull-down | VER-02, VER-77, VER-16 | EVT |
| DEF-11 | R5 kept from the bigger cell | VER-160, VER-37, VER-66 | before the order |
| DEF-12, 13, 29 | parts out of stock | VER-41 (valid 1 day), VER-14 | order day |
| DEF-14 | U1's JLC rotation | VER-29, VER-28 | before the order |
| DEF-15-19, 25 | case fits and openings | VER-48, VER-92, VER-126, VER-07, VER-113, VER-20 | case gate |
| DEF-21 | docs disagree with the design (bringup.md's display and power steps among them) | VER-39 (FAIL), VER-161 | case gate; bringup.md before first power |
| DEF-26-28 | the pin gate's blind spots; guessed heights; analyses not in the repo | VER-36, VER-112, design_numbers.py | before the order |
| DEF-30, 47 | the order candidate not recorded; the register drifted | VER-42, the baseline rule, VER-129 | every change once check.sh runs it |
| DEF-31 | screws engage 3.2 mm | VER-147 (FAIL, per pilot) | case gate |
| DEF-32, 33 | the drill file; the CPL offsets | VER-131, VER-176 (both PASS at 71737ae; DEF-32 CLOSED, DEF-33 FIXED until VER-28's preview) | before the order |
| DEF-34 | no firmware for the ordered chip | VER-149, VER-150 (EVT), VER-151 | EVT, before the order |
| DEF-35, 52 | the trace gate's own holes | VER-129 | every change |
| DEF-36 | check_pcb passes non-silk warnings, and called the routed-away BOARD_REV cosmetic | VER-04, VER-22, VER-18 | before the order |
| DEF-37, 48 | TH1's coupling; the 47 C hot edge | VER-164, VER-87, VER-32 | before the order and EVT |
| DEF-39 | backlight over 120 mA: closed with the 2.4 in panel, back with the 3.5 in | VER-137 (PASS at 71737ae, CLOSED) | before the order |
| DEF-40 | the cutoff may never fire | VER-138 (FAIL), VER-139, VER-155, VER-156 | before the order |
| DEF-41, 42 | assembly next to a live cell; Start/Select parts not bought | VER-91, VER-167 (FAIL) | case gate |
| DEF-43, 51 | inrush; swelling room | VER-99, VER-182 (both CLOSED) | every change |
| DEF-45 | the backlight sim does not read design.py | VER-05 | EVT |
| DEF-46 | bench cell emulation | VER-70, VER-67 | EVT |
| DEF-49, 50 | the pill margin; the VBUS fault reach | VER-45, VER-177 (FAIL at 9.1 V; both OPEN: the BOM half in, the decisions Alex's) | before the order |
| DEF-53, 62 | a worst-case cell 0.25 mm from the FR4 edge; then a pocket with no fit | VER-181 (FAIL on the fit) | before the order |
| DEF-54 | rows still written for an older design after a re-seed | VER-161 (not built), the retired-citation rule | case gate |
| DEF-55, 57, 66 | JLC's marker not on the board; the board's name and BOARD_REV in the cell's hole; J2's '+' over its label | VER-132 (k), VER-18 (PASS), VER-04's warnings read by hand | before the order |
| DEF-56 | copper 0.20 mm from the routed holes; then 0.108 mm as built | VER-18 (PASS in the data), VER-17 (JLC's dated page; reopened, O-R2) | before the order |
| DEF-63 | U10 added without its datasheet | VER-34 (BLOCKED on it) | before the order |
| DEF-64 | the speaker's plug no longer fits J4 | VER-202 (FAIL), VER-128 | case gate |
| DEF-68 | CELL_P and AMP_VDD in no net class | VER-13, VER-201 (both FAIL) | before the order |
| DEF-79 | bom.md's order line 0.16 mm long and its cost table one part number short | VER-39, VER-130 | before the order |
| DEF-65 | the swelling check used the old decks' line | VER-182 (reads uni_ceil; CLOSED) | every change |

## (k) The coverage grid (2026-10-27)

[grid.md](grid.md) is the check classes (rows) by the subsystems (columns) for ref_design.
`tools/grid.py` generates it from each check's `classes` and `subsystems` in verification.csv
and from grid_reasons.csv; nothing in it is typed. A check is tagged only with the classes it
targets for every subsystem it names (the tags are a cross product). The order gate's R9 fails
while a cell of classes 1-15 has no built check that can clear it (below), no NOT_NEEDED
reason and no accepted gap; a cell whose clearing checks
are all NOT_BUILT (or, for class 15, not yet run) is "planned" and still a gap. `check_trace.py` reports the count; run_checks.py
rewrites grid.md after each run. On the reworked board: re-tag the rows the rework touches,
then run `python3 hardware/vv/tools/grid.py`.

Said plainly (2026-10-28): "classes 1-15 pass before the order" means each cell's ORDER checks
pass, or are ACCEPTED through a defect Alex moved with a signed DF-ACCEPT line (G-ORD-22), and the
cell is covered (R9). R9 is coverage, not results: it clears a cell on a FAIL too, so the grid can
read FAIL in a cell on the day the order gate passes. Such a FAIL comes from a check of a later
phase (VER-103 and VER-138 at EVT, say); its defect's Gate holds it, not the order. An accepted gap
is two things, both needed: an ACCEPTED_GAP row in grid_reasons.csv with its reason, and Alex's
signed `GRID-<class>-<subsystem>` line; grid.py and the gate list a signed line with no row. A PASS
whose 'sources' name a datasheet not on disk shows BLOCKED and clears nothing; a check an accepted
defect lets through shows ACCEPTED, never PASS.

Which check can clear a cell (tools/grid.py, after the grid's audit, now in HISTORY.md): for classes 1-14 a built
check that runs before the boards exist, an ORDER check or an analysis (A); for class 15 an
analysis that has run (PASS or FAIL), since a paper estimate exists only once it has a number. A
bench test, inspection or demonstration of a later phase still shows in its cell, marked
"measures, does not clear". The counts per class and each held cell with the checks that would
clear it are grid.md's own sections ("Counts per class", "Gaps"), generated from the register;
read them there, not here.

The first build, at be437f8: 84 cells of classes 1-15 covered, 10 not needed, 26 held by R9.
Before it 22 cells had no check: 10 are NOT_NEEDED with a reason in grid_reasons.csv (8 case
cells, and class 12 for power and the cell), and 12 were planned through six new NOT_BUILT rows,
VER-189 to VER-194. After the audit, at the same baseline: 77 covered, 10 not needed, 33 held
by R9 (all planned); classes 16-18: 16 covered, 8 planned. The analyses R9 waits for that belong
to a later phase (VER-30, VER-100, VER-103, VER-140, VER-148, VER-159, VER-165, VER-187,
VER-192) have to be built before the order (written and runnable); they still run in their own
phase. The class 15 estimates (VER-144, VER-193) have to run before the order: they are ORDER
checks now (VER-13, at EVT, would also clear 15/power and 15/cell once it has run).

False coverage removed first: 12 live rows named a part or part number design.py no longer has
(TP4056, D5, J6, C9, AP3156, BQ24074, and SW17 inside DF-82's and DF-100's 'SW1-SW23'). None
claimed a check of a part that left, so none was retired: REQ-PRD-05, REQ-PWR-02, REQ-PWR-13,
REQ-MCU-04, REQ-MEC-11, REQ-MEC-12, REQ-FAB-10, DF-147 and VER-103 keep that history inside
'Was: ...'; DF-82 and DF-100 now name SW1-SW16 and SW20-SW23 (SW18 and SW19 are the BOOTSEL and
RESET switches, not contacts); DF-144 names the BQ24074 as a wrong variant, "a '74".
`tools/stale_names.py` (VER-188) now fails on any such row, and check_trace's default mode
runs it. The order branch moved again while this ran (307eefd, 64ef18d): the refresh round
tags the new commit.

At 71737ae (2026-10-27), with the rows the board spin touched re-read: 77 covered, 10 not
needed, 33 planned (no cell moved between states: the spin changed results, not tags); by
the worst status in each covered cell 0 PASS, 10 FAIL, 6 NOT_RUN, 61 NOT_BUILT (section i).

Classes 4, 6, 9 and 11 built and run (2026-10-27, at 71737ae): four new scripts and one analysis.
`tools/paper_checks.py` (latch VER-33, usb VER-191, audio VER-100, copper VER-13, shelf VER-103,
mlccbias VER-30, core VER-197 (new), hotplug VER-12, ampstate VER-196, mating VER-140),
`tools/check_backpower.py` (VER-159), `tools/check_placed_contacts.py` (VER-162),
`tools/check_land.py` (VER-125), `tools/check_bom_cpl.py` (VER-130) and
`analysis/errata_rp2350.md` (VER-16). Each script has a `--seed` that must fail, and exit 2 means
BLOCKED (an input not on disk): run_checks.py records nothing for it.

- PASS: VER-33, VER-191, VER-100, VER-12, VER-196, VER-159, VER-16, VER-162.
- FAIL, each a defect: VER-197 (DEF-67: DVDD pin 51 and IOVDD pins 41 and 50 have no capacitor
  within 5 mm, the far DVDD pin has no 4.7 uF), VER-13 (DEF-68: CELL_P is 0.15 mm, VSYS to U3
  229 mohm), VER-103 (DEF-69: 0.8 months from the cutoff to the PCM's UVP at the worst case),
  VER-30 (DEF-70: C24 under 4.7 uF at its own tolerance), VER-125 (DEF-72: U2's pads 0.21 mm
  from the GND pad against TI's 0.26), VER-130 (DEF-21 (x): assembly.md's 117 and 49 against
  120 and 48). DEF-71 carries three firmware rules from the errata.
- BLOCKED: VER-140 (the cell's PCM data, VER-37), VER-178 (the dome's drawing), VER-23 (RP-010329),
  VER-160 (the cell's spec sheet), VER-29, VER-14 and VER-28 (Alex's JLC dry run), VER-186 (JLC's
  parts database), VER-117 (web quotes), VER-05 (ngspice is not installed and the run is a long
  pipeline); VER-125 covers only the five footprints whose drawings are on disk, and VER-30 only
  the minimums that do not need Samsung's curves.

Grid after this round, classes 1-15 by the worst status in each cell (120 cells): PASS 2, FAIL 37,
NOT_RUN 10, NOT_BUILT 38, planned 23, not needed 10.

Classes 2, 3, 5, 7, 8, 10 and 12-15 built and run (2026-10-27, at 71737ae): four new scripts and a
helper. `tools/ds.py` reads every datasheet number from the PDF itself (a quote not on its cited line
fails the check). `tools/circuit_checks.py` (appcircuit VER-198 (new), gpio VER-15, pinreview VER-36,
fwpins VER-149, ratings VER-31, usbfault VER-195, timing VER-199 (new)), `tools/layout_checks.py`
(bridges VER-148 with `bridges.csv`, fab VER-132, jlc VER-17, ret VER-21, usbpair VER-108),
`tools/case_checks.py` (arch VER-190, fit VER-06, placement VER-19, stack VER-20, estimates VER-152,
mouths VER-165, xy VER-174; the ones that need positions.json run in a temp copy of hardware/ref_design)
and `tools/thermal.py` (modes VER-144, board VER-193, amp VER-200 (new)). Each has a `--seed` that was
run once and fails (a check that already fails needs none).

- PASS: VER-15 (every GPIO's job on a pin that has it; 48 of 48 PWM slices agree with the table and
  embassy-rp), VER-31 (every part inside its rating in every mode; 3V3 65 mA on stand-ins, U3 0.08 W),
  VER-148 (167 close net pairs all listed, 20 dangerous with a live DF row), VER-06 (check_fit: no
  collisions), VER-19 (158 parts where placement.py puts them), VER-20 (0.27 mm under the glass, 0.20 mm
  under the ends at worst case), VER-174 (pill to comb 0.48 mm worst case against 0.9), VER-144 (U2
  0.80 W reaches 90.8 C at 40 C ambient against 105; it fails above 62 C/W, so VER-68 matters), VER-193
  (the cell 40.3 C charging while playing at 25 C ambient, against 45).
- FAIL, each a defect: VER-198 (the panel at 3.366 V, DEF-61), VER-190 (M2 x 6 engages 3.2 mm, DEF-31),
  VER-36 (37 pin-review items: orphans, stale bases, open questions, DEF-26), VER-149 (no REF-A pin file,
  DEF-34), VER-195 (VBUS on CC or D+/D-: the pads from 3.9 V, R1/R2 at 17.9 V, the 3V3 domain from 4 V,
  DEF-73), VER-132 (the GND pour 2.875 mm from every comb against 2.9, DEF-74; no JLC marker, DEF-55),
  VER-21 and VER-108 (the USB pair 38.7 mm over no ground and 3.64 mm apart; SR_CLK 6.9 mm, DEF-75),
  VER-199 (the timing closes at 18.75 MHz; pinmap.md states no clock limits, DEF-76), VER-152 (seven
  copper-setting ESTIMATEs and the cell unmeasured, DEF-78), VER-165 (J5's mouth at the wall, DEF-77).
- BLOCKED (exit 2 or not runnable here, nothing recorded): VER-17 (JLC's dated capability page),
  VER-200 (fm8002a.pdf), VER-192 (the case material's datasheet), VER-151 (the battler submodule is not
  checked out), VER-150 and VER-185 (firmware changes outside hardware/vv), VER-143 (route.py --reuse),
  VER-27 (gerbv not installed), VER-07 (check_buttons.py, a long pipeline), VER-113 (the case's STLs),
  VER-112, VER-178, VER-180 and VER-44 (drawings, calipers, the panel), VER-23 and VER-187 (RP-010329),
  VER-34 (U10, TH1, J4 and the other off-disk datasheets), VER-37 (the cell's spec sheet), VER-40 (a
  human reviewer), VER-43 and VER-136 (the panel in hand), VER-28 (Alex's dry run).

The grid now judges a cell of classes 1-15 by the checks that can clear it (tools/grid.py): a bench test
that cannot exist before the boards no longer holds a cell at NOT_BUILT, and it is listed as "measures,
does not clear". Classes 1-15 after this round (120 cells): PASS 15, FAIL 80, NOT_RUN 4, NOT_BUILT 2,
planned 9, not needed 10. R9 holds 9 cells: class 1's seven (VER-189, Alex's), 5/case (VER-192) and
15/audio (VER-200). The order gate fails 23 of 29 criteria; G-ORD-22 counts 31 ORDER checks not passing
and R4 16 OPEN defects due at ORDER (DEF-74, DEF-75 and DEF-78 new). Class 4: buttons and usb_debug PASS; power,
cell, mcu and audio FAIL; display NOT_RUN (VER-05). Class 6: power and cell FAIL (VER-103); the
others show their bench tests (NOT_RUN or NOT_BUILT), their paper checks all pass. Classes 9 and 11
read FAIL in every subsystem because VER-125 and VER-130 name all seven. VER-13's run also clears
class 15 for power and cell.

## (l) The review of 2026-10-27: 16 findings and where each landed

O and F reviewed the register after the two gap-check passes (e09ec9b). A first integrate pass
(another model, no shell) left its edits uncommitted and check_trace crashing; this pass re-read
each of its edits against 71737ae, kept what was right and redid the rest (the commit message
lists what was not kept). The REVISED fix is the one applied.

| # | Finding | What was done | State |
|---|---|---|---|
| O-R1 major | DEF-50 FIXED, so nothing at the order gate held the VBUS overstress | DEF-50 and DEF-49 OPEN again, each naming what closes it (Alex's ACCEPTED line for DF-101 or DF-164 with scope EVT, a DVT clamp, VER-45); G0 and G1 now count a defect leaving OPEN as a move (seeded twice); REQ-PWR-17 stays PROPOSED at EVT | fixed |
| O-R2 | VER-18's PASS is nominal: 0.308 mm is 0.108 mm with the 0.2 mm outline tolerance | edge prints each distance beside its worst case and the web's; VER-18 still compares the data (not flipped to FAIL); DEF-56 reopened until JLC's dated page says whether 0.30 mm is a data rule or an as-built one; REQ-FAB-03's Notes carry the numbers | fixed; VER-17 BLOCKED on the page |
| O-R3 | the board is 151.48 mm long, the register said 151.64 | the preamble, REQ-PRD-09, REQ-FAB-11, VER-90 and VER-166 at 151.48; DEF-21 (k) corrected; DEF-79 (bom.md's order line, Gate ORDER); section (b)'s row. The Gerbers' extent is 73.4 x 151.48 mm: the outline has J1's 1.0 mm tab, so '72.5 x 151.58 over the outline' was wrong in width | fixed in the register; bom.md is the design session's |
| O-R4 | SHELL.md and CELLCUT_FOR_CASE.md quote the old case, board, hole and panel; no check reads case/*.md | case/*.md added to VER-39's and VER-161's scope; DEF-21 (y), (z) and (aa) (ref_design/README.md too) | fixed in the register; the docs are the design session's |
| O-R5 | the backlight's 3.9k, 56 mA, 103.9k and GPIO44 still live | VER-05, DF-44, DF-45, DF-59, REQ-PRD-03.R, REQ-MCU-07, REQ-DSP-06, REQ-DSP-07, REQ-DSP-08 and REQ-THM-04 rewritten to R61 3.74k on GPIO28, 0.119 V, 54.1 mA a pin, 108.2 mA nominal, 117.7 mA worst case, 103.74k; design.py's comments and the sim's defaults are DEF-21 (ab), (ac); VER-05 and DEF-45 stay | fixed in the register |
| O-R6 | an unrun EVT analysis cleared the display's class 6 cell | grid.py: a clearing check of classes 1-14 counts once it has run (PASS or FAIL), as class 15 already did; a clearing check not of phase ORDER prints its phase (VER-05@EVT); seeded (VER-137 not run leaves 4/display planned) | fixed |
| O-R7 | section (i) was an old snapshot | section (i) rewritten at this commit | fixed |
| O-R8 | G-ORD-24 claimed more than VER-188 reads; path-form nets unseen; build_pcb.py's dead class entry | G-ORD-24 narrowed to the three files; the net pattern reads '/Sheet/NET' (seeded); VER-201 (design_numbers.py netclass, new, ORDER): every class net is a board net and every net of 0.25 A or more sits in a class; FAIL on VBUS_D5's entry, CELL_P and AMP_VDD (DEF-68) | fixed; the copper is DEF-68's |
| O-R9 | DEF-47 FIXED while scripts/check.sh does not run check_trace | DEF-47 OPEN at Gate ORDER; the check.sh line is Alex's (step 0.3), or he moves the Gate to CASE | fixed in the register |
| O-R10 | README's tool table missed five scripts | paper_checks.py, check_backpower.py, check_bom_cpl.py, check_land.py and check_placed_contacts.py added from their own docstrings | fixed |
| O-R11 | the review ran nothing | this pass ran check_trace, grid.py, seed_faults.py, run_checks.py and --gate order, and read git status: the integrate pass's design.py and build_pcb.py edits had already been reverted | done |
| F-R1 major | DEF-49 and DEF-50's keyword FIXED and their text 'stays OPEN' | both OPEN; a structure rule fails a Status keyword other than OPEN whose text says it stays OPEN or holds a gate (seeded; it found DEF-39, reworded); R4 lists the FIXED defects due by the gate beside its result; DEF-23 CLOSED on VER-06's PASS. No separate 'holds this gate' marker: OPEN is that marker, and the rule keeps the two from disagreeing | fixed |
| F-R2 | VER-184 compared BOM lines, not plugs | plugs reads each header's family and pitch from its footprint name and the cell's plug from analysis/cell.json (VER-37): VER-184 is ORDER and BLOCKED until the spec sheet; the speaker's half is VER-202 (CASE): its plug from bom.md's row, its leads' reach to J4 and J2 asserted; FAIL while the SH refit is not ordered (DEF-64). The plug families are not put in design.py: the register does not edit it | fixed; BLOCKED and FAIL on the parts |
| F-R3 | a doc edit stales nothing | a structure rule: an analysis whose procedure reads assembly.md, bom.md, bringup.md or pinmap.md names that doc in covers (seeded); VER-138 now covers pinmap.md's calibration lines, VER-184 and VER-202 bom.md's rows; README states the rule | fixed |
| F-R4 | bom.md's feeder row says J4 reuses J2's part number | DEF-79 (the whole factory block re-derived by the design session); VER-39 reads the cost rows; VER-117 replaces the snapshot | fixed in the register |
| F-R5 | DF-48's prevention named a PicoBlade plug for the SH header | DF-48 rewritten for the SH housing and its crimp (DF-183 carries the bad crimp); DF-112's old cause moved inside 'Was:'; stale_names.py's connector-family rule reads each header's family from its footprint name and fails DF-48's old text (seeded) while DF-170 and VER-128 pass | fixed |
