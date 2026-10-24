# Phase gates

Five gates. Each is a table of criteria; each criterion names the IDs that must be met and
the board it applies to. `python3 hardware/vv/tools/check_trace.py --gate
<order|case|evt|dvt|pvt> [--board ref_design|prod_board]` evaluates a gate and exits 1 unless
every criterion that applies to the board is met. What "met" means for each need:

- VER-nn: last_result PASS, on evidence that is fresh (see README), with no uncommitted change
  under what it covers. A CUT check never meets a criterion: waive the criterion or name the
  check that replaced it.
- REQ-...: APPROVED in releases/APPROVALS.md, and every listed check at or before this gate
  passed as above (a split requirement through its allocation for the board).
- DF-nn: ACCEPTED by Alex for this phase or a later one, or DONE with every one of its
  'detects' checks at or before this gate passed as above.
- DEF-nn: CLOSED.
- PHASE:P: every check of the board whose phase is P or earlier passed as above (CUT checks
  excluded).
- DEFECTS:CLOSED: every defect of the board CLOSED.
- DFMEA:COVERAGE: every assembled part and every contact in design.py is named in a DFMEA row.
- ADVISORY: the criterion is printed but never fails the gate.

A criterion whose board column does not match the board is N/A. A criterion left with
nothing that applies to the board fails, so prod_board needs its own checks.

On top of its table every gate applies seven rules (check_trace.py's docstring has them in
full): R1 no PROPOSED requirement in scope; R2 every requirement in scope verified up to this
phase; R3 every H-priority DFMEA row in scope ACCEPTED for this phase or DONE with all its
'detects' checks passing; R4 no OPEN defect (order, case, EVT) or every defect CLOSED (DVT,
PVT); R5 no uncommitted change under passed evidence; R6 every design commit since the
newest release record named in a change record (ref_design); R7 the design files at HEAD are
README's design baseline (ref_design: the order gate reads design.py and placement.py, the
case gate case.scad, later gates all three).

A criterion that the evidence cannot meet may be waived by Alex alone: the waiver row below
proposes the reason; it counts only when he signs it in releases/APPROVALS.md (WAIVED, with
the last phase it holds for).

## Order gate: before the 5-board EVT order

Only what the copper and the BOM freeze blocks the order (O-9). The printed case has its own
gate (G-CASE) and depends on nothing here except the positions the board takes from it
(VER-19). Criteria 1-13 are validation_plan.md section 2, the 13-point go/no-go list, in its
order; 14 onwards are this register's.

| ID | Criterion | Needs | Board |
|---|---|---|---|
| G-ORD-01 | vp 1: the current datasheets are in hand with hashes; every IC's pin table is signed off with quotes; the errata table is done; no open question or unknown rating left in the pin reviews (both rating schemas read) | VER-02 VER-35 VER-16 VER-36 | ref_design |
| G-ORD-02 | vp 2: the LCSC identity check, the land-pattern overlay and the SOT pin numbering show zero unexplained differences; the BOM, CPL and PCB fields equal design.py; FM8002A shutdown polarity confirmed | VER-14 VER-125 VER-124 VER-130 | ref_design |
| G-ORD-03 | vp 3: PWR_HOLD pull-down 8.2k or less (R42 4.7k) and CE on a GPIO (Q4) are in design.py; R5 is set from the cell's spec sheet and an assert ties it to the cell; the watchdog rule is written | VER-02 VER-37 VER-160 VER-104 | ref_design |
| G-ORD-04 | vp 4: the real panel lights through a confirmed bottom-contact breakout using design.py's J3 map, driven from an RP2350B dev board at the redraw time REQ-DSP-11 sets; the folded FPC proves the 51-j map; the tail thickness is within the FH12 spec | VER-43 VER-136 VER-44 | ref_design |
| G-ORD-05 | vp 5 (moved to G-EVT-12, F-49): a pill on ENIG reads under 1 kohm and comb leakage is above 100 Mohm, on a bare board from the same order; a coupon ordered now is advisory | ADVISORY VER-45 VER-46 | ref_design |
| G-ORD-06 | vp 6 (agreed cut, O-10 and F-32): the latch replica is replaced by latch.py (VER-33, in G-ORD-18) before the order and VER-70, VER-73, VER-77, VER-78 on unit 1; met only by Alex's waiver W-01 | VER-47 | ref_design |
| G-ORD-07 | vp 7, what the copper fixes: part heights from drawings under the case's limits; the pill-to-comb XY chain and its measurement; the positions the board takes from the case (J1, J2 and its cutout, J3, H1-H6, the combs, the pinholes); the connector mouths; the MLCC keep-out from the bosses; the cell rectangle (TH1, bare pads); every copper-setting ESTIMATE measured; J3 against the FPC fold | VER-06 VER-112 VER-174 VER-51 VER-19 VER-165 VER-142 VER-164 VER-152 DF-61 | ref_design |
| G-ORD-08 | vp 8: C9 and C1 derating is done and the unplug and hot-plug simulations are re-run and committed | VER-30 VER-11 VER-12 | ref_design |
| G-ORD-09 | vp 9: JLC limits from the dated capability page, no edge or rail surprises, the netlist sent with the net ties merged; the drill files split and parsed; the fab files' content checked | VER-17 VER-18 VER-26 VER-131 VER-132 | ref_design |
| G-ORD-10 | vp 10: dry-run screenshots of every polarised part signed off (U1, D7, U4, J3 included), the rotation and centroid table done, the production-file review clean | VER-28 VER-29 | ref_design |
| G-ORD-11 | vp 11: every check fresh against the gerber hash being ordered, from one release run in a clean worktree, tools recorded; live stock confirmed on order day; RP2354B and L2 pre-ordered or consigned, no substitution | VER-24 VER-143 VER-41 VER-42 | ref_design |
| G-ORD-12 | vp 12 (advisory, F-32): the clean-room review replies read; the release record lists which | ADVISORY VER-40 | ref_design |
| G-ORD-13 | vp 13 (agreed cut, O-10 and F-32): the perfboard is replaced by VER-11 before the order and VER-70 on unit 1; met only by Alex's waiver W-02 | VER-123 | ref_design |
| G-ORD-14 | the design round trip is fresh: netlist, ERC, DRC with parity, both contact footprints in the library and as placed | VER-01 VER-03 VER-04 VER-08 VER-09 VER-162 | ref_design |
| G-ORD-15 | the DFMEA is reviewed by Alex and two other models; every severity 9-10 row names its layers of protection; every assembled part and contact is rated; every adjacent pad pair is classed | VER-98 VER-148 DFMEA:COVERAGE | ref_design |
| G-ORD-16 | the cell is known: spec sheet in hand, usable capacity measured on a sample, incoming inspection ready (polarity and the plug's latch) | VER-37 VER-153 VER-49 | ref_design |
| G-ORD-17 | the docs match the design: the stale-doc sweep, the claims script, every bring-up probe point present, every contact class bought | VER-39 VER-161 VER-115 VER-158 VER-167 | ref_design |
| G-ORD-18 | the power-path analyses are written and pass: latch table (10 kohm), temperature window, ratings and 3V3 budget, inrush, back-power paths, backlight tolerance, cutoff accuracy, cell mating peak, heat per mode | VER-33 VER-32 VER-31 VER-99 VER-159 VER-137 VER-138 VER-140 VER-144 | ref_design |
| G-ORD-19 | the order candidate is named in a release record (branch, commit, tag, design baseline): this register describes 76955d2, and origin/main has a different REF-A (DEF-30) | VER-42 DEF-30 | ref_design |
| G-ORD-20 | bench readiness: gear on hand or bought for every EVT check (`check_trace.py --shopping --gate evt` lists what is still needed, longest lead first); the speaker chosen and its rating known | VER-116 VER-128 | ref_design |
| G-ORD-21 | order contents recorded: 5 assembled, 5 bare boards from the same order, rework spares, every order option (vp order size, #72, #189, #193) | VER-42 | ref_design |
| G-ORD-22 | every ORDER-phase check of the board passes, fresh (O-1: nothing the order needs can hide behind a later gate) | PHASE:ORDER | both |
| G-ORD-23 | firmware for the ordered chip: generated pins, a self-test that builds and links for rp235xb at 2048K, the game image inside the flash budget | VER-149 VER-150 VER-151 | ref_design |
| G-ORD-24 | the trace gate and the checkers are shown to fail on seeded faults | VER-129 VER-22 | ref_design |
| G-ORD-25 | prod_board: its own round trip and contacts (as library footprints, as placed, where prod_case says) | VER-10 VER-163 VER-119 VER-120 VER-121 VER-122 | prod_board |

## Case gate: before the printed case is released for the EVT units

The case changes daily while the board freezes, so its checks gate the print run, not the
board order (F-29).

| ID | Criterion | Needs | Board |
|---|---|---|---|
| G-CASE-01 | the Z stack closes with swelling, the buttons work under the shell, no unintended openings, the screws engage | VER-20 VER-07 VER-113 VER-147 | ref_design |
| G-CASE-02 | the printed case with real parts, its feel against a genuine DS Lite, incoming silicone and caps | VER-48 VER-50 VER-126 | ref_design |
| G-CASE-03 | the case is released: case.scad commit, STL hashes, material, slicer profile, tape part number | VER-173 | ref_design |
| G-CASE-04 | case-level requirements: size, keyed assembly, no openings, room to swell | REQ-PRD-06 REQ-MEC-04 REQ-MEC-08 REQ-BAT-07 | ref_design |
| G-CASE-05 | every check up to CASE passes, fresh | PHASE:CASE | both |

## EVT exit: on the 5 assembled boards, before the DVT changes are frozen

Evidence for a per-unit check names the units it ran on (`UNITS:` in its header). "Every
unit" means all boards that will be kept; a sacrificial board may skip the safety soak but
not the FAI.

Unit allocation (O-8): the screens whose results decide the DVT changes run here, on a
sacrificial unit, so the DVT spin is made knowing them.

| Unit | Use | Ships? |
|---|---|---|
| 1 | FAI (VER-109), full bring-up, golden readings (VER-171), cell mating (VER-141), PWR_HOLD and hang tests | no (FAI) |
| 2 | bring-up, soak, thermal (VER-85, VER-86, VER-87), warm box (VER-146), panel bond (VER-154) | yes, if its connectors are within budget (VER-157) |
| 3 | bring-up, soak, play and charge time (VER-101, VER-102), cutoff sweep (VER-139) | yes, as unit 2 |
| 4 | abuse and margins (VER-97), ESD screen (VER-95), near-field (VER-96), drop (VER-93), button life (VER-94), fault injection (VER-168) | never |
| 5 | spare; replaces any unit lost before its tests finish | yes, as unit 2 |

| ID | Criterion | Needs | Board |
|---|---|---|---|
| G-EVT-01 | first article inspection on unit 1, from the generated sheet, before any other unit is powered | VER-166 VER-109 | ref_design |
| G-EVT-02 | first power done safely on every unit: JP1-cut first power from a 100 mA bench supply, unpowered checks, cell polarity, the charge-path screen with no cell | VER-74 VER-52 VER-53 VER-54 VER-133 | ref_design |
| G-EVT-03 | bring-up steps 1-6 pass on every unit | VER-55 VER-56 VER-57 VER-58 VER-59 VER-60 VER-61 VER-62 VER-63 VER-64 VER-65 VER-66 VER-67 VER-68 VER-69 VER-73 | ref_design |
| G-EVT-04 | power-path margins measured: unplug at a held 3.50 V, hot plug, CE, 3V3 regulation and budget, 1V1 | VER-70 VER-71 VER-72 VER-83 VER-84 | ref_design |
| G-EVT-05 | cell safety on built units, the bench charging rules kept first: per-unit CV, TEMP and cell-current screens; temperature pause; cell temperature in a full charge; TH1 tracking; PCM trip; cell mating; charge timeout; D5 leakage; cutoff accuracy and calibration; PWR_HOLD release; hang test | VER-172 VER-134 VER-135 VER-169 VER-82 VER-69 VER-85 VER-87 VER-88 VER-141 VER-145 VER-146 VER-139 VER-155 VER-156 VER-77 VER-105 | ref_design |
| G-EVT-06 | touch temperature within the IEC 62368-1 TS1 limit, with the heat rule in the image | VER-86 | ref_design |
| G-EVT-07 | latch and USB margins: 10 kohm start, leakage immunity, sources and cables (with the 4.40-4.60 V characterisation), input current, back-power with a probe attached | VER-78 VER-79 VER-80 VER-81 VER-127 | ref_design |
| G-EVT-08 | the self-test image runs on every unit; chip ID and stepping logged | VER-76 VER-75 | ref_design |
| G-EVT-09 | mechanics with real parts: FPC and ZIF, recovery through the pinholes, assembly demo in the safe order, flatness, microscope inspection, panel bond | VER-106 VER-107 VER-91 VER-90 VER-89 VER-154 | ref_design |
| G-EVT-10 | play time, charge time and cost | VER-101 VER-102 VER-117 | ref_design |
| G-EVT-11 | every check up to EVT (ORDER, CASE and EVT phases) passes, fresh, against the design going into DVT | PHASE:EVT | both |
| G-EVT-12 | vp 5 on a bare board from the order: pill resistance and comb leakage, day one, before any assembled unit is powered (F-49) | VER-45 VER-46 | ref_design |
| G-EVT-13 | the DVT-deciding screens on unit 4: abuse with a reversed cell, ESD screen at the combs, near-field, drop, button life, fault injection (every credited detection trips) | VER-97 VER-95 VER-96 VER-93 VER-94 VER-168 | ref_design |
| G-EVT-14 | unit records: serials, findings with disposition and re-test, golden readings, connector mating counts | VER-110 VER-171 VER-157 | ref_design |

## DVT exit: on the revised spin

| ID | Criterion | Needs | Board |
|---|---|---|---|
| G-DVT-01 | every check up to DVT re-run where a change record asks, and passing on fresh evidence | PHASE:DVT | both |
| G-DVT-02 | durability on the DVT spin: open/close, drop, button life, margins and abuse | VER-92 VER-93 VER-94 VER-97 | ref_design |
| G-DVT-03 | ESD to IEC 61000-4-2 with a real gun, and the EMC pre-scan | VER-170 VER-96 | ref_design |
| G-DVT-04 | every defect CLOSED, each with a passing check | DEFECTS:CLOSED | both |
| G-DVT-05 | the checkers are shown to fail on seeded faults | VER-22 VER-129 | ref_design |

## PVT exit: the 30-board run

| ID | Criterion | Needs | Board |
|---|---|---|---|
| G-PVT-01 | release record complete: tag, commit, design baseline, fab zip, BOM and CPL sha256, BOARD_REV (bumped for any BOM change), locked BOM with LCSC numbers, no-substitution list (L2 and the RP2354B included), case section, gate output, waivers | VER-42 | ref_design |
| G-PVT-02 | first article inspection on the first PVT unit | VER-109 | ref_design |
| G-PVT-03 | a production record with serial for every unit, its calibration and mating counts, and incoming inspection of every cell | VER-110 VER-155 VER-157 VER-49 | ref_design |
| G-PVT-04 | first-pass yield 90% or more, every failure logged; golden readings re-derived for PVT | VER-111 VER-171 | ref_design |
| G-PVT-05 | every check up to PVT passes on fresh evidence against the PVT release | PHASE:PVT | both |
| G-PVT-06 | only if any unit is sold or given away: cell transport documents and emissions | VER-38 VER-118 | ref_design |

## Waivers

Each row proposes a waiver and its reason. It counts only when Alex signs it in
[releases/APPROVALS.md](releases/APPROVALS.md) with decision WAIVED and the last phase it
holds for; check_trace refuses a signature in a commit that carries a Claude trailer.

| ID | Criterion | Proposed reason |
|---|---|---|
| W-01 | G-ORD-06 | superseded by VER-33 before the order and VER-70, VER-73, VER-77, VER-78 on unit 1; every latch part reworks on an EVT unit, and USB bypasses the latch for bring-up |
| W-02 | G-ORD-13 | superseded by VER-11 before the order and VER-70 on unit 1; a perfboard does not reproduce the layout parasitics or C9's DC bias the unplug depends on |
| W-03 | G-PVT-06 | applies only if a unit leaves Alex's hands |
