# Phase gates

Four gates. Each is a table of criteria; each criterion names the IDs that must be met.
`python3 hardware/vv/tools/check_trace.py --gate <order|evt|dvt|pvt> [--board ref_design|prod_board]`
evaluates a gate and exits 1 unless every criterion is met. What "met" means for each ID:

- VER-nn: last_result PASS, on evidence that is fresh (its commit is an ancestor of HEAD
  and nothing it covers changed since), with no uncommitted change under what it covers.
- REQ-...: APPROVED, and every listed check at or before this gate passed as above.
- DF-nn: ACCEPTED by name and date, or DONE (its action taken) with a detection check that passed as above.
- DEF-nn: CLOSED.
- PHASE:P: every check of the board whose phase is P or earlier passed as above.
- DEFECTS:CLOSED: every defect CLOSED.

A check for the other board is skipped; a criterion left with nothing that applies to the
board fails, so prod_board needs its own checks before it can pass a gate.

On top of its table, every gate applies five rules (see the README): R1 no PROPOSED
requirement; R2 every requirement has a check for the board and its checks up to this phase
pass; R3 every H-priority DFMEA row is accepted, or done with a passing detection up to this phase;
R4 no OPEN defect (order, EVT) or every defect CLOSED (DVT, PVT); R5 no uncommitted change
under passed evidence.

A criterion that the evidence cannot meet may be waived in the table at the end, by Alex
only, with a reason and a date. A waiver row with no name or date does not count.

## Order gate: before the 5-board EVT order

Criteria 1-13 are validation_plan.md section 2, the 13-point go/no-go list, in its order;
14-21 are what this register adds.

| ID | Criterion | Needs |
|---|---|---|
| G-ORD-01 | vp 1: the current datasheets are in hand with hashes; every IC's pin table is signed off with quotes; the errata table is done; no open question or unknown rating is left in the pin reviews | VER-02 VER-35 VER-16 VER-36 |
| G-ORD-02 | vp 2: the LCSC identity check, the land-pattern overlay and the SOT pin numbering show zero unexplained differences; FM8002A shutdown polarity confirmed | VER-14 VER-125 VER-124 |
| G-ORD-03 | vp 3: PWR_HOLD pull-down 8.2k or less (R42 4.7k) and CE on a GPIO (Q4) are in design.py; R5 is set from the cell's spec sheet; the cell's foam and gap are decided by the stack; the watchdog rule is written | VER-02 VER-37 VER-20 VER-104 |
| G-ORD-04 | vp 4: the real panel lights through a confirmed bottom-contact breakout using design.py's J3 map; the tail thickness is within the FH12 spec | VER-43 VER-44 |
| G-ORD-05 | vp 5: a pill on matched ENIG reads under 1 kohm; comb leakage is above 100 Mohm (this now means SW21, seat 1's Start, whose comb carries VSYS; the plan wrote P1_Y) | VER-45 VER-46 |
| G-ORD-06 | vp 6 (disputed: Fable only, Opus does not accept it as a gate): the latch replica holds and releases at VSYS 3.5 V and 4.2 V and starts with 1 kohm in series | VER-47 |
| G-ORD-07 | vp 7: check_fit and the Z stack pass on measured heights, swelling included, with 0.2 mm or more of margin; the buttons pass under the shell | VER-06 VER-20 VER-112 VER-07 |
| G-ORD-08 | vp 8: C9 and C1 derating is done and the unplug and hot-plug simulations are re-run and committed | VER-30 VER-11 VER-12 |
| G-ORD-09 | vp 9: JLC DRU clean, no edge or rail surprises, the IPC-D-356 netlist exported | VER-17 VER-18 VER-26 |
| G-ORD-10 | vp 10: dry-run screenshots of every polarised part signed off (U1 and D7 included), the rotation table done, the production-file review clean | VER-28 VER-29 |
| G-ORD-11 | vp 11: every check fresh against the gerber hash being ordered, tools recorded; live stock confirmed on order day; RP2354B and L2 pre-ordered or consigned, no substitution | VER-24 VER-41 VER-42 |
| G-ORD-12 | vp 12: the clean-room review replies have been read and answered | VER-40 |
| G-ORD-13 | vp 13 (Opus only, conditional): if the perfboard was built, it never showed 3V3 under 2.97 V | VER-123 |
| G-ORD-14 | the design round trip is fresh: netlist, ERC, DRC with parity, both contact footprints | VER-01 VER-03 VER-04 VER-08 VER-09 |
| G-ORD-15 | the DFMEA is reviewed by Alex and two other models; every severity 9-10 row names its layers of protection | VER-98 |
| G-ORD-16 | the cell is known: spec sheet in hand, incoming-inspection procedure ready | VER-37 VER-49 |
| G-ORD-17 | the docs match the design, bring-up procedure included (step 0 limits, the 5.6 limit, D7 in the lists) | VER-39 VER-115 |
| G-ORD-18 | the power-path analyses are written and pass: latch table, temperature window, ratings and 3V3 budget, inrush | VER-33 VER-32 VER-31 VER-99 |
| G-ORD-19 | the order candidate is named in a release record (branch, commit, tag): this register describes 76955d2, and origin/main has a different REF-A (DEF-30) | VER-42 DEF-30 |
| G-ORD-20 | bench readiness: gear on hand or bought for every EVT check; the speaker chosen | VER-116 VER-128 |
| G-ORD-21 | order contents recorded: 5 assembled, 5 bare boards from the same order, rework spares (vp order size, #72, #193) | VER-42 |

## EVT exit: on the 5 assembled boards, before the DVT changes are frozen

Evidence for a per-unit check names the units it ran on (serials). "Every unit" means all
boards that will be kept; a sacrificial board may skip the safety soak but not the FAI.

| ID | Criterion | Needs |
|---|---|---|
| G-EVT-01 | first article inspection on unit 1 before any other unit is powered | VER-109 |
| G-EVT-02 | first power done safely on every unit: JP1-cut first power, unpowered checks, cell polarity | VER-74 VER-52 VER-53 VER-54 |
| G-EVT-03 | bring-up steps 1-6 pass on every unit | VER-55 VER-56 VER-57 VER-58 VER-59 VER-60 VER-61 VER-62 VER-63 VER-64 VER-65 VER-66 VER-67 VER-68 VER-69 VER-73 |
| G-EVT-04 | power-path margins measured: unplug, hot plug, CE, 3V3 regulation and budget, 1V1 | VER-70 VER-71 VER-72 VER-83 VER-84 |
| G-EVT-05 | cell safety on built units: latent-fault screen, temperature pause, cell temperature in a full charge, TH1 tracking, PCM trip, PWR_HOLD release, hang test | VER-82 VER-69 VER-85 VER-87 VER-88 VER-77 VER-105 |
| G-EVT-06 | touch temperature within the IEC 62368-1 TS1 limit | VER-86 |
| G-EVT-07 | latch and USB margins: 1 kohm start, leakage immunity, sources and cables, input current, back-power | VER-78 VER-79 VER-80 VER-81 VER-127 |
| G-EVT-08 | the self-test image runs on every unit; chip ID and stepping logged | VER-76 VER-75 |
| G-EVT-09 | mechanics with real parts: case fit, FPC and ZIF, recovery through the pinholes, assembly demo, flatness, microscope inspection | VER-48 VER-106 VER-107 VER-91 VER-90 VER-89 |
| G-EVT-10 | play time and charge time | VER-101 VER-102 |
| G-EVT-11 | every ORDER-phase check still passes, fresh, against the design going into DVT | PHASE:ORDER |

## DVT exit: on the revised spin

| ID | Criterion | Needs |
|---|---|---|
| G-DVT-01 | every ORDER and EVT check re-run on the DVT design and passing on fresh evidence | PHASE:EVT |
| G-DVT-02 | durability: open/close, drop, button life, margins and abuse | VER-92 VER-93 VER-94 VER-97 |
| G-DVT-03 | ESD and EMC screens | VER-95 VER-96 |
| G-DVT-04 | every defect CLOSED, each with a passing check | DEFECTS:CLOSED |
| G-DVT-05 | the checkers are shown to fail on seeded faults | VER-22 |

## PVT exit: the 30-board run

| ID | Criterion | Needs |
|---|---|---|
| G-PVT-01 | release record complete: tag, commit, fab zip sha256, locked BOM with LCSC numbers, no-substitution list (L2 and the RP2354B included), gate output, waivers | VER-42 |
| G-PVT-02 | first article inspection on the first PVT unit | VER-109 |
| G-PVT-03 | a production record with serial for every unit, and incoming inspection of every cell | VER-110 VER-49 |
| G-PVT-04 | first-pass yield 90% or more, every failure logged as a defect | VER-111 |
| G-PVT-05 | every check up to PVT passes on fresh evidence against the PVT release | PHASE:PVT |
| G-PVT-06 | only if any unit is sold or given away: cell transport documents and emissions | VER-38 VER-118 |

## Waivers

Only Alex fills these. A criterion with a valid waiver counts as met; the gate output says so.

| ID | Criterion | Reason | Approver | Date |
|---|---|---|---|---|
| W-01 | G-ORD-06 | disputed in the validation plan (section 4); Alex to decide whether the latch replica gates the order | - | - |
| W-02 | G-ORD-13 | applies only if the perfboard is built | - | - |
| W-03 | G-PVT-06 | applies only if a unit leaves Alex's hands | - | - |
