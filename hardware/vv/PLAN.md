# The plan: from here to 5 boards, then 30

**Not ready to order.** The order gate fails on 26 of its 29 counted criteria (section i).
This is the single plan that came out of the V&V register (commit 4af820b) after two models,
Opus 5.5 (O) and Fable 5.1 (F), attacked it, cross-examined each other and agreed. Every
agreed idea is applied below and in the register files; none was dropped and none is still
disputed. Everything here describes the design at **76955d2** (TP4056, ME6211, the 3.5in
ER-TFT035IPS-6). Alex has not yet said that this is the commit he will order (decision D1).

Contents: (a) the system on one page; (b) before the 5-board order; (c) EVT, DVT and PVT;
(d) the scripts to write; (e) the past bugs and what now catches them; (f) decisions for
Alex; (g) dropped ideas; (h) what this plan still cannot catch; (i) the gate today.

## (a) The system on one page

**What it proves.** Each board works when it is built as designed (verified), the design
is the right one for the numbers (validated), it is safe (the Li-ion cell, touch
temperature), JLC can make it, it fits the case, and it survives use.

**The chain.** Every number the product must meet is a requirement (requirements.md,
129, all PROPOSED). Every way a part, interface or assembly step can fail is a DFMEA row
(dfmea.csv, 143 rows: 42 H, 65 M, 36 L), and its `detects` column names the checks that can
actually see that failure. Every check is a row in verification.csv (174 checks), with a
procedure, a numeric pass limit, what it covers, its result and its evidence. Defects found
so far are in defects.md (46). Gates (gates.md) name what each phase needs. Alex's
decisions live in one file, releases/APPROVALS.md, which only he edits.

**Five gates.** ORDER (before the 5-board order: only what the copper and the BOM freeze),
CASE (before the printed case is released for the EVT units), EVT (the 5 boards), DVT (the
revised spin), PVT (the 30-board run). SALE items wait until a unit leaves Alex's hands.

**What check_trace.py enforces** (`python3 hardware/vv/tools/check_trace.py`):

- Every ID exists; every check is reached by a gate, a requirement or an H row no later
  than its own phase, so the order gate cannot pass without an ORDER check (O-1).
- A PASS rests on a saved output with a COMMIT, TOOLS and RESULT header, never on a commit
  message (O-4, F-3). `tools/run_checks.py` writes these for the quick scripts.
- Evidence goes stale when what it covers changes: a file (regexes as whole words, F-2),
  a part in design.py (`part:R52,R53`, O-5), a release of built units (`release:REL-...`:
  stale only when a change record asks for a re-run, O-6), or its age (valid_days, F-5).
- A DFMEA row is DONE only when all its `detects` checks pass; its detection rating cannot
  be better than they allow; an occurrence of 2 or less needs a passing test on built
  hardware, and 1 needs a stated physical exclusion (O-2, O-26, F-13).
- Requirement approvals, risk acceptances and waivers count only from lines Alex committed
  in APPROVALS.md (git blame; a Claude trailer is an error); an acceptance holds only up to
  its phase; an approved statement edited later needs approving again (O-3, O-45).
- A BOM-only change bumps BOARD_REV (O-35); once a release exists, a design commit with no
  change record fails every gate (F-31); a gate fails when HEAD's design files are not the
  README's design baseline (F-1); every assembled part and contact is named in the DFMEA
  (O-55: 0 missing today).
- Warnings: a pass band narrower than 4 x its instrument's accuracy (F-38: VER-67, VER-88,
  VER-134, VER-139 today); a 'needs' entry with no lead time.
- `--shopping --gate evt` lists what the unrun checks still need, longest lead first (F-50).

`tools/seed_faults.py` seeds 29 faults into a copy of the register and shows each one fails
(VER-129, PASS). The register's own holes that the review found are DEF-35, now CLOSED.

**Both boards.** prod_board's gate reads only REQ-PBD-* and the `.P` allocations Alex writes
or opts into (O-7). Product requirements that named ref_design's parts are split into a
board-neutral parent and a `.R` child. Gate criteria carry a board column; the others show
N/A (F-6).

## (b) Before the 5-board order

In order. Effort is Claude's unless it says Alex. The parts lead time (up to about 21 days)
runs in parallel with steps 1-9, so start step 0 today.

| # | Step | IDs | Effort |
|---|---|---|---|
| 0.1 | **Alex names the order candidate.** If it is not 76955d2, re-seed the power path, panel and button rows and the baseline first. Clean the main checkout's uncommitted battler.kicad_pro before any fab run (O-34). | DEF-30, G-ORD-19, R7 | Alex: 5 min; re-seed half a day |
| 0.2 | **Order everything with a lead time** (`--shopping --gate evt`): RP2354B and L2 pre-order at JLC; cells and the 403450 spec sheet; the panel, an FH12 breakout and an RP2350B dev board; speaker samples; calipers, a 4.096 V reference, a J2 sample, clip leads, a LiPo bag, a USB-C breakout, 82 ohm and 10 ohm power resistors. | VER-41, VER-37, VER-49, VER-153, VER-43, VER-136, VER-128, VER-116 | Alex: 1 h, about $100-150 |
| 0.3 | Run the register on every commit: a hardware step in scripts/check.sh (check_trace, run_checks' quick scripts), and print what a staged design change makes stale. | O-48 | 30 min |
| 1.1 | **TH1 and the cell**: script the cell rectangle and TH1's lag; decide a B.Cu patch under TH1 with vias, or a silicone thermal pad for the foam; TP11-TP14 off the pouch. | VER-164, DF-04, DEF-37 | 2 h, then a copper decision |
| 1.2 | **C9 against H3's boss**: the MLCC keep-out script; move C9, or Alex accepts it for EVT. | VER-142, DF-38, DEF-38 | 1 h |
| 1.3 | **The cutoff**: the calibrated-reading analysis; decide a voltage reference (copper), a firmware rule, or a 3.6 V cutoff. | VER-138, DF-116, DEF-40, REQ-BAT-12 | 2 h, then a decision |
| 1.4 | **The panel against the firmware**: light it on the dev board through the breakout with the J3 map, time a full RGB565-to-666 redraw; fold the real FPC into a breakout in J3's place; measure the tail. If the redraw fails, change the panel now. | VER-43, VER-136, VER-44, REQ-DSP-11, DF-120, DF-57 | an evening once the parts arrive |
| 1.5 | **VBUS over-voltage**: accept for EVT, or add an OVP switch or an SMBJ6.0A footprint. | DF-101 | decision |
| 1.6 | **Copper set by estimates**: the ESTIMATE ledger with calipers on the cell plug, the FPC and the Start/Select membrane; the Start/Select pill measured and VER-09 re-run with it. | VER-152, VER-09, DF-123, DF-124 | 2 h plus calipers |
| 1.7 | **The board against the case**: placement drift, the XY chain, connector mouths, check_fit re-run on heights from drawings, the pill prints on the fit kit. | VER-19, VER-174, VER-165, VER-06, VER-112, VER-51, DF-61 | about a day |
| 1.8 | **Copper review**: the adjacent-pad bridge table, the copied core diff, return paths, IR drop, USB stubs. | VER-148, VER-23, VER-21, VER-13, VER-108, DF-106 | about a day |
| 2 | **Power-path analyses**, committed under board/sim/: latch.py at 10 kohm; the temperature window (and the R39/R40 decision, D5); the ratings table under the new derating rule; DC bias and the unplug and hot-plug ODEs; back-power paths (DEF-01's check); backlight tolerance (and the duty cap or R61 3.6k, D12); the cell-mating peak; the heat table and its firmware rule; the shelf estimate; the audio chain; the backlight sim re-run with its defaults asserted. VER-99 (inrush) is done. | VER-33, VER-32, VER-31, VER-30, VER-11, VER-12, VER-159, VER-137, VER-140, VER-144, VER-103, VER-100, VER-05 | about 3 days |
| 3 | **Datasheets and part identity**: the pin gate reads both rating schemas and resolves its 9 open ratings; the datasheet archive; the errata table; the application-circuit diff; the GPIO mux; LCSC identity; BOM, CPL and PCB fields against design.py; land patterns; the rotation and centroid table (U1, J1, J2). | VER-36, VER-35, VER-16, VER-34, VER-15, VER-14, VER-130, VER-125, VER-29, DEF-14, DEF-26, DEF-33 | 2-3 days |
| 4 | **Fab outputs**: split the drill file; check the fab files' content; JLC's dated limits in a data file (never a hand-written .kicad_dru); edges; the netlist with net ties merged; reproducible outputs; the gerbv look; holes; contacts as placed; check_pcb failing on non-silk warnings; the JLC order-number marker in silk.py and the order options. | VER-131, VER-132, VER-17, VER-18, VER-26, VER-24, VER-27, VER-114, VER-162, VER-04, DEF-32, DEF-36, DF-125 | about 1.5 days |
| 5 | **Firmware for the ordered chip**: pins generated from design.py; a self-test that builds and links for rp235xb at 2048K; the game image measured against a flash budget. | VER-149, VER-150, VER-151, DEF-34 | half a day to a day |
| 6 | **Docs and BOM**: fix DEF-21's list in the ref_design docs (bring-up step 0 limits, a no-cell screen in step 1, 5.6 at 5.7 V, Start not Y, the cell-emulation steps; the safe assembly order; M2 x 11 screws; Start/Select strips and caps; pinmap.md's calibration, heat rule, flash map and minimum duty); the claims script; the stale-doc sweep re-run; test access; BOM coverage. | VER-161, VER-39, VER-115, VER-158, VER-167, DEF-21, DEF-31, DEF-41, DEF-42, DEF-46 | about 1.5 days |
| 7 | **The cell**: the spec sheet; cell.json and its assert; usable capacity on one sample; the incoming procedure (polarity and the plug's latch). Recompute REQ-PRD-03.R, REQ-PRD-04.R and REQ-BAT-02.R. | VER-37, VER-160, VER-153, VER-49 | 1 h plus 5 h unattended |
| 8 | **Prove the checkers fail**: extend seed_faults.py to check_netlist, check_pins, check_pcb, check_fit and check_contact_fp. | VER-22 (VER-129 done) | half a day |
| 9 | **Reviews and readiness**: the DFMEA review (Alex and two models); the gear list from --shopping; the speaker's rating; the clean-room review (advisory). | VER-98, VER-116, VER-128, VER-40 | Alex: half a day |
| 10 | **Alex's decisions** in APPROVALS.md: the requirements; the H rows (13 can be DONE once their checks pass; 29 can only be detected on built units, so Alex accepts them for ORDER, see D19); waivers W-01 and W-02. | R1, R3, G-ORD-06, G-ORD-13 | Alex: 1-2 h |
| 11 | **Release and order**: the release run from a clean tag; the release record (BOARD_REV, BOM and CPL hashes, the options checklist); the dry run (valid 7 days); stock on order day (valid 1 day); `check_trace.py --gate order` exits 0; order 5 assembled and 5 bare boards. | VER-143, VER-42, VER-28, VER-41, G-ORD-11, G-ORD-21 | 1 day |

Rough total: about 12-15 working days of Claude's work, a few hours of Alex's, and up to
three weeks of parts lead time in parallel. Steps 1.1-1.6 come first because each can move
copper; the rest re-check whatever they move.

## (c) EVT, DVT and PVT

### Case gate (between the order and EVT assembly)

Z stack with swelling (VER-20), buttons under the shell (VER-07), no openings (VER-113),
screws that engage 4 mm (VER-147), the printed case with real parts and its feel (VER-48,
VER-50), incoming silicone and caps (VER-126), and the case released with STL hashes,
material and slicer profile (VER-173). Exit: `--gate case` exits 0 (G-CASE-01 to G-CASE-05).

### EVT: the 5 assembled boards

Day one, before any assembled unit is powered: pill resistance and comb leakage on a bare
board from the same order (G-EVT-12). Then unit 1's first article inspection from the
generated sheet (VER-166, VER-109) before any other unit is powered.

Per unit, in this order: unpowered resistances and diode tests (VER-52, VER-53); first
power from a 100 mA bench supply with JP1 cut (VER-74, VER-55); the charge-path screen with
USB in and no cell (VER-133: TP3 <= 4.25 V, TP2 - TP3 >= 0.3 V, TP7 >= TP1 - 0.4 V); the
cell-current reading with charging off (VER-169); CV with an 82 ohm load (VER-134); TEMP by
resistors (VER-135). Only then a real cell, attended, in a LiPo bag (VER-172). Clip leads
only on TP3 and TP7 (DF-114, DF-115).

Unit allocation (gates.md): unit 1 FAI, full bring-up and golden readings; units 2 and 3
bring-up, soak and thermal; unit 4 abuse, ESD screen, near-field, drop, button life and
fault injection (it never ships); unit 5 spare. The screens that decide the DVT changes run
here, on unit 4, so the DVT spin is made knowing them (O-8).

Exit: `--gate evt` exits 0. In words: FAI done; every unit through first power, bring-up
steps 1-6 and the per-unit safety screens; power-path margins measured with the bench
supply held at 3.50 V (CHG_OFF high, O-19); touch temperature within TS1 with the heat rule
in the image; the latch at 10 kohm; the self-test on every unit; mechanics with real parts;
play time, charge time and cost; the unit 4 screens; unit records with findings,
calibration, golden readings and mating counts; every check up to EVT fresh (G-EVT-01 to
G-EVT-14).

### DVT: the revised spin

Candidates, decided from EVT's results. BOM-only (each bumps BOARD_REV): R39/R40 for the
temperature window (DF-05); 4.7k pull-downs on AMP_EN and BL_PWM (DF-45, DF-47); R18 56k
(DF-95); R5 once the cell is known (DF-07); R61 3.6k unless firmware caps the duty (DF-130).
Copper: reverse-polarity protection or a keyed connector on CELL_P (DF-55); series-resistor
footprints on the button nets and SW21 (DF-100); a second element beside Q2 (DF-11); input
OVP (DF-101); TH1's thermal path, if not done before the order (DF-04).

Exit: `--gate dvt` exits 0: every check up to DVT re-run where a change record asks; open
and close, drop, button life and abuse on the DVT spin; ESD to IEC 61000-4-2 with a real
gun (VER-170) and the EMC pre-scan; every defect CLOSED; the checkers shown to fail.

### PVT: the 30-board run

Exit: `--gate pvt` exits 0: a complete release record (tag, baseline, hashes, BOARD_REV,
locked BOM, no-substitution list, case section); FAI on the first PVT unit; a record per
unit with its calibration and mating counts; every cell inspected; first-pass yield 90% or
more with every failure logged; golden readings re-derived for PVT; every check fresh
against the PVT release. Before any unit is sold or given away: UN38.3 and MSDS, emissions,
marking, and the fire-enclosure decision (G-PVT-06, DF-87).

## (d) The scripts to write, in priority order

Highest severity caught per hour first, and order blockers before EVT tooling.

| # | Script | Check | What it catches | Effort |
|---|---|---|---|---|
| 1 | scripts/check.sh hardware step | O-48 | a register broken by an edit, found only on order day | 30 min |
| 2 | board/tools/check_bom_cpl.py, run inside fab.py | VER-130 | a stale or wrong LCSC number on the order (a 5.0 V U3, a BAT54 variant) while every schematic check passes | 1 h |
| 3 | fab.py --excellon-separate-th and a drill parser | VER-131 | J1's pegs plated: no power and no USB on every board | 1 h |
| 4 | board/tools/check_rotation.py with the centroid table | VER-29 | U1 reversed (3V3 shorted), J1 placed 1.46 mm off its pads | 2 h |
| 5 | board/tools/check_backpower.py | VER-159 | DEF-01's class: an unpowered 3V3 fed through an I/O | half a day |
| 6 | board/sim/latch.py | VER-33 | a latch that cannot start on a worn pill near the cutoff | 2 h |
| 7 | board/sim/temp_window.py | VER-32 | a charge window that reaches 49 C | 1 h |
| 8 | board/sim/cutoff.py | VER-138 | a cutoff that never fires on a high-Vreg unit | 2 h |
| 9 | board/sim/backlight_tol.py and backlight.py asserting its defaults | VER-137, VER-05 | LEDs over 120 mA at worst case; a sim that silently ignores a design change | 2 h |
| 10 | check_pins.py: both rating schemas, open questions, passives | VER-36 | a 6.3 V cap on VBUS, an UNKNOWN rating passing the datasheet gate | 1 h, then a day of reviews |
| 11 | board/tools/check_bridges.py and bridges.csv | VER-148 | a VBAT-to-GND bridge under U2 that shorts the first cell | 2 h |
| 12 | check_fit: cell rectangle, MLCC boss keep-out, connector mouths | VER-164, VER-142, VER-165 | TH1 reading the board edge; C9 cracked by the screws; a cable that cannot reach | half a day |
| 13 | case/check_estimates.py | VER-152 | a cell plug that will not pass the board, an FPC that cannot reach J3 | 1 h |
| 14 | board/tools/check_placement.py, case/check_xy.py | VER-19, VER-174 | the board drifting from the case positions; pills off their combs | half a day |
| 15 | check_fab_content.py (reusing membrane/tools/gerb.py) | VER-132 | paste on a comb, a 0.25 mm drill, an aux origin shifting the drills | half a day |
| 16 | pinmap.py writing board_ref_a.rs, a host test, the rp235xb build and a size check | VER-149, VER-150, VER-151 | a bring-up image on the wrong pin map; a game that does not fit 2 MB | half a day |
| 17 | cell.json and its assert | VER-160 | DEF-11 again at the next cell change | 1 h |
| 18 | tools/check_claims.py | VER-161 | docs and register rows quoting values the design no longer has (12 times so far) | half a day |
| 19 | check_test_access.py, bom_coverage.py | VER-158, VER-167 | a per-unit screen whose pads were removed; no Start/Select caps bought | 2 h |
| 20 | hardware/vv/tools/release.py, board/tools/check_fab.py | VER-143, VER-24 | gerbers ordered from a different tree or tool version than the one checked | half a day |
| 21 | check_lcsc.py, check_land.py, check_gpio.py, check_edge.py, the JLC limits file, the merged IPC-D-356 | VER-14, VER-125, VER-15, VER-18, VER-17, VER-26 | wrong variants, wrong pads, a pin without its function, an edge or rail surprise, an e-test hold | 2 days |
| 22 | the unplug and hot-plug ODEs, DC bias, check_copper, check_return, check_usb, check_core, the audio sim | VER-11, VER-12, VER-30, VER-13, VER-21, VER-108, VER-23, VER-100 | analyses quoted in docs that nobody can re-run (DEF-28) | 3 days |
| 23 | check_pcb.py on non-silk warnings; contacts as placed; check_contact_fp --pill | VER-04, VER-162, VER-163, VER-09 | drills too close passing DRC; a placed footprint unlike the checked one; a dead power button | 2 h |
| 24 | seed_faults.py extended to the design checkers | VER-22 | a checker that silently stopped failing | half a day |
| 25 | EVT tooling: fai_sheet.py; the self-test image (generated neighbour pairs, timer build, calibration) | VER-166, VER-76, VER-145, VER-155, VER-156 | a wrong reel on unit 1; adjacent GPIOs bridged; an endless charge; a wrong stored gain | 2-3 days, before the boards arrive |

## (e) Past bugs and what now catches them

| Defect | What happened | Now caught by | Gate | When |
|---|---|---|---|---|
| DEF-01 | U1's REF on VBUS back-fed PWR_EN | VER-159 (seeded with this fault), VER-127 | G-ORD-18, G-EVT-07 | before the order |
| DEF-02 | R15 at 10k cut the pill margin | VER-33 at 10 kohm, VER-78 | G-ORD-18, G-EVT-07 | before the order |
| DEF-03 | the hand-over took tens of ms | VER-11, VER-70 | G-ORD-08, G-EVT-04 | before the order |
| DEF-04 | 3V3 under 2.97 V on unplugs | VER-11, VER-70, VER-138 | G-ORD-08, G-ORD-18 | before the order |
| DEF-05 | hot plug over the ME6211's 6.5 V | VER-12, VER-71 | G-ORD-08, G-EVT-04 | before the order |
| DEF-06 | VSYS over U7's 6.0 V; bring-up checks 6.5 V | VER-71 at 5.7 V, VER-31, VER-36, VER-161 | G-ORD-18, G-ORD-17 | before the order |
| DEF-07 | J6 TX and RX swapped | VER-02 | G-ORD-01 | every change (CLOSED) |
| DEF-08 | USB resistors 22 ohm, not 27 | VER-02 | G-ORD-01 | every change (CLOSED) |
| DEF-09 | CE above VIN at a live plug-in | VER-72, VER-12 | G-EVT-04 | EVT, analysed before |
| DEF-10 | no external PWR_HOLD pull-down | VER-02, VER-77, VER-16 | G-ORD-01, G-EVT-05 | before the order |
| DEF-11 | R5 kept from the bigger cell | VER-160, VER-37, VER-66 | G-ORD-03 | before the order |
| DEF-12 | AP3156 out of stock | VER-41 (valid 1 day), VER-14 | G-ORD-11 | order day |
| DEF-13 | ProTek SRV05-4 out of stock | VER-41, VER-14 | G-ORD-11 | order day |
| DEF-14 | U1's JLC rotation in doubt | VER-29, VER-28 | G-ORD-10 | before the order |
| DEF-15 | first PLA print's hooks snapped | VER-48, VER-92 | G-CASE-02, G-DVT-02 | case gate |
| DEF-16 | D-pad opening too tight | VER-126 | G-CASE-02 | case gate |
| DEF-17 | deck insert would not seat | VER-07 (re-run with saved evidence) | G-CASE-01 | case gate |
| DEF-18 | Start/Select wells hit the pockets | VER-07 | G-CASE-01 | case gate |
| DEF-19 | holes through the lid | VER-113 | G-CASE-01 | case gate |
| DEF-20 | J6's mouth faced J5's back | VER-165 | G-ORD-07 | before the order |
| DEF-21 | docs disagree with the design (18 items) | VER-161, VER-39 | G-ORD-17 | every change once VER-161 is in check.sh |
| DEF-22 | the validation plan's statuses went stale | the plan archived; statuses in vp_map.csv and verification.csv | none | fixed |
| DEF-23 | JST PH J2 too tall | VER-06, VER-112 | G-ORD-07 | before the order |
| DEF-24 | audio numbers wrong in the docs | VER-100, VER-64 | G-ORD-22, G-EVT-03 | before the order |
| DEF-25 | lid window too tight | VER-06, VER-20 | G-ORD-07, G-CASE-01 | before the order |
| DEF-26 | the pin gate passes with open questions | VER-36 | G-ORD-01 | before the order |
| DEF-27 | check_fit guesses heights | VER-112, VER-22 | G-ORD-07, G-ORD-24 | before the order |
| DEF-28 | analyses not in the repo | VER-11, VER-12, VER-13, VER-100; the evidence rule | G-ORD-08, G-ORD-22 | before the order |
| DEF-29 | RP2354B and L2 out of stock | VER-41 (needs: the pre-order) | G-ORD-11 | order day |
| DEF-30 | the order candidate not recorded | VER-42, R7 (the design baseline) | G-ORD-19, every gate | before the order |
| DEF-31 | screws engage 1.4 mm | VER-147, VER-92 | G-CASE-01 | case gate |
| DEF-32 | drill file mixes PTH and NPTH | VER-131 | G-ORD-09 | before the order |
| DEF-33 | CPL off the pads for J1 and J2 | VER-29, VER-28 | G-ORD-10 | before the order |
| DEF-34 | no firmware for the ordered chip | VER-149, VER-150, VER-151 | G-ORD-23 | before the order |
| DEF-35 | the trace gate's own holes | VER-129 | G-ORD-24, G-DVT-05 | every change (CLOSED) |
| DEF-36 | check_pcb passes non-silk warnings | VER-04 fixed, VER-22 | G-ORD-14, G-ORD-24 | before the order |
| DEF-37 | TH1 at the cell's edge; bare pads face the pouch | VER-164, VER-87 | G-ORD-07, G-EVT-05 | before the order |
| DEF-38 | C9 0.9 mm from a boss | VER-142, VER-92 | G-ORD-07 | before the order |
| DEF-39 | backlight over 120 mA at worst case | VER-137, VER-62 | G-ORD-18, G-EVT-03 | before the order |
| DEF-40 | the cutoff may never fire | VER-138, VER-139, VER-155, VER-156 | G-ORD-18, G-EVT-05 | before the order |
| DEF-41 | assembly next to a live cell | VER-91 | G-EVT-09 | EVT (doc fix before) |
| DEF-42 | Start/Select pills and caps not bought | VER-167 | G-ORD-17 | before the order |
| DEF-43 | inrush over USB 2.0's 50 uC | VER-99 (PASS) with REQ-PWR-13's deviation | G-ORD-18 | before the order |
| DEF-44 | requirements that failed by construction | VER-161 for their numbers | G-ORD-17 | before the order |
| DEF-45 | the backlight sim cannot follow design.py | VER-05 (part covers and asserts) | G-ORD-22 | before the order |
| DEF-46 | cell emulation cannot hold 3.50 V | VER-70, VER-67 revised | G-EVT-04 | EVT |

## (f) Decisions for Alex

Each goes into releases/APPROVALS.md (requirements, acceptances, waivers) or into a commit.
Claude's proposal follows each question.

**The design and the order**

- **D1. Which commit is the order candidate?** This register describes 76955d2. origin/main
  (56933a5) has the BQ24073, RT9013, the 2.4in ER-TFT024IPS-3 and 74HC165s;
  ref-design-levers (86bf694) removes J6, C41 and R48-R51, which O-18, O-35 and the
  self-test's UART need (O-62). Proposed: name it first; every other decision depends on it.
- **D2. Are the RP2354B and L2 pre-orders placed?** Both are on the no-substitution list.
- **D3. Will any of the 30 be sold or given away?** That decides the SALE items: UN38.3 and
  MSDS, emissions, marking, and the printed case not being a fire enclosure (DF-87).

**Requirement numbers that need your value** (all 129 are PROPOSED; approve, reject or
defer each in APPROVALS.md)

- **D4.** Play time 4 h or more; charge time 3 h or less; 100,000 presses of button life;
  $45 or less a unit at 10. Proposed: DEFERRED for REQ-PRD-03.R, REQ-PRD-04.R and
  REQ-BAT-02.R until the cell's spec sheet (VER-37) and a capacity sample (VER-153) exist
  (F-33).
- **D5.** The charge window reaches 49 C at worst case. Change R39 and R40 (BOM only) so the
  top edge stays at or below 45 C? Proposed: yes, from VER-32's sweep (DF-05).
- **D6.** Must 3V3 stay above 3.135 V (VREG_AVDD, USB_OTP_VDD) through the unplug dip, or is
  2.97 V enough? Proposed: decide with VER-138 and VER-153 in hand, since a 3.6 V cutoff
  costs play time (DF-34).
- **D7.** The cutoff: 3.50 V +0.05/-0.00 V after calibration (REQ-BAT-12), with a bench
  calibration at 3.50 V on every unit (REQ-TST-06)? A voltage reference needs copper.
- **D8.** Legacy USB hosts (REQ-PWR-12): accept charging at up to about 600 mA before
  enumeration, or add a firmware rule? USB 2.0 caps an unconfigured device at 100 mA.
- **D9.** The derating rule is rewritten (REQ-PWR-14): ICs at or below the recommended
  maximum, transients 0.3 V under the absolute maximum, passives 80% (90% for transients).
  Accept its two named exceptions: U7 at 5.7 V of 6.0 V, and GPIO47 below.
- **D10.** GPIO47 sees VBAT/2 with IOVDD at 0 V on every unit. Take R18 = 56k (BOM only,
  0.447 V at 4.2 V, 8 uA of divider drain), or name the waiver? Proposed: R18 56k (F-46).
- **D11.** RP2350-E9: 4.7k pull-downs on AMP_EN (R37) and BL_PWM (R61 path), BOM only, or rely
  on firmware and the stepping (DF-45, DF-47)? Proposed: wait for VER-75's stepping.
- **D12.** Backlight: cap the duty at 95% in firmware, or R61 3.6k (BOM only)? (REQ-DSP-05)
- **D13.** Redraw time: 120 ms or less for a full screen and 50 ms for battle text
  (REQ-DSP-11)? If the breakout misses it, which fallback panel?
- **D14.** Flash budget 1.6 MB of 2 MB (REQ-MCU-10)? Off current 60 uA or less at 45 C
  (REQ-PWR-07)? The 45 C deep-discharge period (REQ-BAT-10)? Lot capacity 600 mAh to the
  cutoff (REQ-BAT-16)? Charge timeout 4 h and 30 min under 3.0 V (REQ-BAT-13)?
- **D15.** USB input low end: run from 4.40 V (REQ-PWR-01) but charge to done only from 4.75
  V (REQ-PWR-16)? Decide after VER-80's characterisation.
- **D16.** prod_board: which product requirements apply to your board? Write the `.P`
  allocations you want; until then your gate reads REQ-PBD-01 to 03 only.

**Hardware decisions before the order**

- **D17.** TH1: a B.Cu copper patch under it with vias, or a silicone thermal pad for the foam
  (DF-04)? C9: move it from H3's boss, or accept for EVT (DF-38)? VBUS: accept no OVP for
  EVT, or add an OVP switch or TVS footprint (DF-101)?
- **D18.** Cell swelling: keep the 0.5 mm gap with a 0.45 mm foam pad, or redesign for 0.4 mm
  of free height (DF-84, a CASE item)? Screws: change to M2 x 11 (DEF-31)?

**Risk acceptances and waivers**

- **D19.** 29 H rows can only be detected on built units, so the order gate needs your
  acceptance for ORDER (it expires at EVT, where each must be DONE): DF-04, DF-05, DF-11,
  DF-13, DF-14, DF-26, DF-38, DF-55, DF-67, DF-69, DF-70, DF-72, DF-74, DF-79, DF-84, DF-86,
  DF-87, DF-88, DF-100, DF-101, DF-102, DF-103, DF-104, DF-106, DF-108, DF-112, DF-115,
  DF-116, DF-117. Take each row's pre-order action first (for example DF-04's copper
  decision). DF-87 (the printed case is not a fire enclosure) needs a scope of its own:
  proposed through DVT, as hobby units kept by their builder. DF-38 and DF-100 need one
  through EVT (their last check runs at DVT). The other 13 H rows can be DONE before the
  order once their checks pass.
- **D20.** Sign W-01 (G-ORD-06, the latch replica) and W-02 (G-ORD-13, the perfboard)?
  Both models agreed to cut them (O-10, F-32); the proposed reasons are in gates.md.
- **D21.** DVT: reverse-polarity protection or a keyed connector on CELL_P (DF-55, the
  highest-risk row)? Decide after VER-97 on unit 4.

**Disputed ideas.** None. Every point the two models disagreed on was settled in
cross-examination; there are no O and F positions left to put side by side.

## (g) Dropped ideas

None were dropped. Where both models revised the same idea, the texts were reconciled:

- F-50 (shopping list): one 'needs' column ending in a '<N>d' lead time; a missing day token
  is a warning, not a gate failure; `--shopping` sorts by the longest lead.
- F-51 (golden unit): the analysis limit stays the only pass/fail limit; one golden.csv,
  replaced by the median of units 1-4 at EVT exit and re-derived at DVT and PVT; bands of 2%
  for rails and 20% for currents, and a factor of 10 for resistances and leakage.
- F-52 (probe slip): two DFMEA rows, without a cell (DF-114, S5) and with one (DF-115,
  S9); their detection checks are not built yet, so both rate D8, not D6.
- O-63 (calibration at 3.50 V): the two texts agree; the expected ADC count follows the
  released R17 and R18.
- O-60 and O-63: the bench calibration replaces the CHG_DONE_N calibration; what is left of
  O-60 is VER-156 (no gain stored from a no-cell edge). The production band is O-63's
  0.95-1.05.
- O-9 and F-29: the case gate holds O-9's members; the order gate keeps what copper fixes.
- F-18: TH1 open and R39 shorted are safe failures (the unit never charges). R40 open is not:
  it moves the hot edge up by about 1 C, so DF-111 rates it as a small hazard.
- Duplicates merged: O-10 with F-32, O-25 with F-45, O-27 with F-16, O-28 with F-15, O-30
  with F-36, O-41 with F-35, O-13 with F-47, O-15 with F-20 and F-21, O-4 with F-3, O-12 with
  F-27, O-21 with F-7, O-26 with F-13, O-33 with F-22, O-37 with F-44.

Applied as work items rather than edits here, because this task writes only the register:
the ref_design doc fixes (step 6), the scripts/check.sh step (step 0.3), the silk.py marker
(step 4) and every script in section d.

## (h) What this plan still cannot catch

- **Small numbers.** Five boards, then 30, cannot show a failure rate under a few percent.
  A rare cell defect, a marginal joint or a weak PCM lot can pass every test here.
- **Time.** Cell ageing and swelling over years, ENIG wear past the life test, tape creep
  over summers, flux that migrates slowly. The DVT life tests are short stand-ins.
- **Faults nobody listed.** The DFMEA names every part, but only the failure modes someone
  thought of. Seeded faults prove a checker fails on the faults seeded, not on all faults.
- **Bench honesty.** A bench result is typed from a meter. The evidence header ties it to a
  release and to units, but nothing can prove the reading was taken.
- **Approvals.** The blame check trusts git authorship: a line Alex commits by hand counts,
  whoever wrote it.
- **Standards.** The IEC, USB and UN clause numbers and the 62368-1 touch limits are from
  memory and marked "to confirm". Pre-scans are not compliance tests. ESD is screened with an
  uncalibrated igniter until a real gun is borrowed at DVT.
- **JLC's side.** What JLC changes after its engineering review is caught only by its
  returned placement file and the FAI; a passive substituted without notice may pass both.
- **Firmware beyond the self-test.** The game's own behaviour (a hang path, a heavy screen,
  a flash write in an unplug) is tested only where a requirement names it.
- **Use.** Water, a hot car past 60 C, drops above 1 m, a child biting the case.
- **prod_board.** Almost nothing until Alex writes his `.P` allocations; today its gate reads
  three requirements and six checks.
- **Another design.** If the order candidate is not 76955d2, this register describes the
  wrong board until it is re-seeded; R7 makes every gate say so.

## (i) The gate today

At e7a26e8 plus this commit's records (10 PASS, 1 FAIL, 53 NOT_RUN, 108 NOT_BUILT, 2 CUT):

```
$ python3 hardware/vv/tools/check_trace.py
STRUCTURE: OK (4 warnings)
  requirements 129 {'PROPOSED': 129}
  DFMEA rows 143, action priority {'H': 42, 'L': 36, 'M': 65}
  checks 174 {'PASS': 10, 'NOT_RUN': 53, 'NOT_BUILT': 108, 'FAIL': 1, 'CUT': 2}
  validation-plan methods 218: 201 mapped, 17 cut (archived plan)
  defects 46 {'FIXED': 23, 'CLOSED': 3, 'OPEN': 20}
  approvals by Alex: 0 committed, 0 pending
  design baseline: matches HEAD
  design.py parts and contacts named in no row (of 137): 0
  warnings: the pass bands of VER-67, VER-88, VER-134 and VER-139 are 1.4-2.2 x their
  DMM's accuracy (want 4 x): a 4.096 V reference or a better meter for those
exit 0

$ python3 hardware/vv/tools/check_trace.py --gate order
  PASS  R5 no uncommitted change under passed evidence
  PASS  R6 every design change since the last release has a change record (no release yet)
  PASS  R7 the register describes this design
  INFO  G-ORD-05, G-ORD-12 (advisory); N/A G-ORD-25 (prod_board)
  FAIL  G-ORD-01 to G-ORD-04, G-ORD-06 to G-ORD-11, G-ORD-13 to G-ORD-24
  FAIL  G-ORD-22: 72 of the 81 ref_design and shared ORDER checks not passing (CUT excluded)
  FAIL  R1 126 PROPOSED; R2 58 requirements with an ORDER check not passing;
        R3 42 H rows neither accepted nor DONE; R4 20 OPEN defects
RESULT: FAIL (26 of 29 not met); 1 N/A, 2 advisory
```

The case gate fails 9 of 12, EVT 18 of 21, DVT 9 of 12, PVT 10 of 13, and prod_board's order
gate 4 of 7 (23 criteria N/A). The only checks that pass today: the netlist, the pin gate,
ERC, DRC with parity, the two 4-ring contact footprints, the inrush arithmetic (VER-99),
the trace gate's seeded faults (VER-129), and two inspections (VER-104, VER-124).
