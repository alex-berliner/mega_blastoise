# Phase gates

Five gates, and before them the new-MCU path that leads to them. Each gate is a table of
criteria; each criterion names the IDs that must be met and the board it applies to. `python3
hardware/vv/tools/check_trace.py --gate <order|case|evt|dvt|pvt>` evaluates a gate for f1c200s
(the Allwinner F1C200s board, the next spin) and exits 1 unless every criterion that applies to
it is met. What "met" means for each need:

- VER-nn: last_result PASS, on evidence that is fresh (see README) and committed, with no
  uncommitted change under what it covers. At DVT and PVT a check of phase EVT or later that is
  a test (T) or covers a release, whatever its method, must also cover a release record of that
  phase (REL-...-DVT, REL-...-PVT): EVT units do not speak for the revised spin, and an EVT
  first article does not inspect the first PVT unit (O-R11, F-R1). A CUT, RETIRED or DROPPED
  check never meets a criterion: name the check that replaced it.
- REQ-...: APPROVED in releases/APPROVALS.md, and every listed check at or before this gate
  passed as above (a split requirement through its allocation for the board).
- DF-nn: ACCEPTED by Alex for this phase or a later one, or DONE with every one of its
  'detects' checks at or before this gate and passed as above (a detects check of a later
  phase cannot pass this gate: such a row needs an acceptance).
- DEF-nn: CLOSED.
- PHASE:P: every check of the board whose phase is P or earlier passed as above (CUT, RETIRED,
  DROPPED and advisory-only checks excluded).
- DEFECTS:CLOSED: every defect of the board CLOSED (or RETIRED: its subject left the design; a
  DROPPED defect is another board's).
- DFMEA:COVERAGE: every assembled part and every contact in design.py is named in a live DFMEA
  row. While design.py is the dropped ESP32-S3 board's (README '- Baseline board:') it fails: no
  F1C200s design.py exists to cover.
- ADVISORY: the criterion is printed but never fails the gate. A check that only advisory
  criteria name (and no requirement or H row) is advisory: PHASE:P does not count it (VER-40).
  A criterion moved to a later gate stays here as ADVISORY, saying where it went, so its ID
  keeps its history.
- ADVISORY:<id>: that one check is advisory at this gate and binding at the later gate that
  names it (O-R9): it is printed, never fails this gate, while the criterion's other needs
  stay binding.
- RETIRED (alone): the criterion's subject left the design; its text says 'RETIRED in
  <commit>: why'. It keeps its ID, is printed and never counted.
- DROPPED (alone): the criterion belonged to a dropped board; its text says 'DROPPED with <board>
  (tag <tag>): why' and its board column is that board. It keeps its ID, is printed and never
  counted.

A criterion whose board column does not match the board is N/A. A criterion left with
nothing that applies to the board fails. Since 574c94c deleted prod_board from the repository
(Alex: not needed), prod_board's criteria (G-ORD-25, G-EVT-15) are RETIRED and keep their board
column. On 2026-10-28 the ESP32-S3 board and the RP2354B board were both dropped: every live
criterion is the F1C200s board's (f1c200s), and its checks are NOT_RUN or NOT_BUILT until an
F1C200s design replaces the ESP32-S3 board's files on main (README, Design baseline). R7 fails
until then: the baseline is the dropped board's files.

On top of its table every gate applies these rules (check_trace.py's docstring has them in
full): R1 no requirement in scope PROPOSED, or DEFERRED past its scope; R2 every requirement
in scope verified up to this phase; R3 every H-priority DFMEA row in scope ACCEPTED for this
phase or DONE with all its 'detects' checks passing; R4 no OPEN defect whose Gate is this phase or earlier (order, case, EVT), or every such
defect CLOSED (DVT, PVT): a case defect does not hold the order back (O-R5); R5 nothing
uncommitted under passed evidence or anywhere in hardware/vv, and every evidence file
committed (O-R2); R6 every design commit since the newest release record named in a change
record (ref_design; the design is design.py, placement.py, lib/, battler.kicad_pcb,
battler.kicad_pro, the schematics, the board/tools tree, case.scad and case/export.py); R7
the design files at HEAD are README's design baseline (ref_design); R8 (order and case gates)
the order branch README names has no commit after the baseline commit that touches the gate's
files, the board's at the order gate and the case's at the case gate, as R7 (O-R13). At the
order gate R8 also holds the board's positions: a case-only commit on the order branch passes
only when positions.json regenerated at that commit (case/export.py without --stl) has the same
sha256 as positions.json regenerated at the baseline commit; both hashes are printed and go
into the release record (O-R4). R9 (order gate, 2026-10-27) the coverage grid (grid.md, made by
tools/grid.py from verification.csv's classes and subsystems and grid_reasons.csv) has no GAP
in classes 1-15: each subsystem has, for each class, a built check that runs before the boards
exist (classes 1-14: an ORDER check or an analysis; class 15: an analysis that has run, PASS
or FAIL), a NOT_NEEDED reason, or a gap Alex accepted with a signed
'GRID-<class>-<subsystem>' line; a cell with only later bench checks or NOT_BUILT checks is
planned and still a gap, and a grid.md that is not what the register gives fails it too. Two more gate results, both reported and never structural:
G1 "the gate state Alex froze": a signed GATE@<commit> line in releases/APPROVALS.md freezes
each check's phase, each defect's Gate, each criterion's Needs and each requirement's Phase
and Verified-by; a check, defect, criterion or requirement moved later without a newer signed
line fails the order gate (O-R5). K1 "the pinned signing key is a hardware key": the order
gate fails when the key README pins is not sk-ssh-ed25519@openssh.com or
sk-ecdsa-sha2-nistp256@openssh.com, unless it is run with --accept-software-key, whose use
must then be copied into the release record's signing section; no line in APPROVALS.md can
grant the exception (O-R12). G0 "moves later since the reviewed register state" (round 3's O-S5,
2026-10-27): until Alex signs a GATE@ line, the order gate prints, as information, every check,
defect, criterion or requirement moved later since the commit README names on its 'Gate moves
listed since' line, so he reads the list before he freezes it. The default mode already fails when
the baseline drifts (DEF-47) and warns when the order branch (main from fd66be9 since 2026-10-28)
moves.

A criterion that the evidence cannot meet may be waived by Alex alone: the waiver row below
proposes the reason; it counts only when he signs it in releases/APPROVALS.md (WAIVED, with
the last phase it holds for).

## The new-MCU path (2026-10-28)

How the register moves a new chip from Alex's requirements to the gates above. It was written for
the F1C200s, the chip after the ESP32-S3 and the RP2354B boards were dropped. Each stage has entry
and exit needs; `check_trace.py` (default mode) prints the stage the register is at: the first
stage whose exit is not met, and any later stage met out of order. A need is an ID (VER: PASS and
fresh; REQ: APPROVED by Alex; DEF: CLOSED; DF: DONE), a stage (NM-nn: its exit met),
`APPROVED:REQ-<AREA>` (every live requirement of that area APPROVED in a line Alex signed),
`DOC:<path>::<regex>` (a committed file has a matching line) or `GATE:<phase>` (that gate met; a
gate need is not evaluated while an earlier stage is open). A stage's entry names the stage before
it; a check a stage names is live. A NOT_BUILT check here is a work item: it has no script or
procedure for this chip yet.

| ID | Stage | Entry | Exit | Board |
|---|---|---|---|---|
| NM-01 | Requirements: Alex approves the REQ-PRD rows (DESIGN.md is his spec) and the requirements are reviewed per subsystem | DOC:docs/static/DESIGN.md::next.spin.uses.the.Allwinner.F1C200s | APPROVED:REQ-PRD VER-189 | f1c200s |
| NM-02 | Chip selection: done, queue issue #4 (Alex, 2026-10-28: the F1C200s, the V3s as fallback) | NM-01 | DOC:docs/static/next_spin_requirements.md::^##.Chosen.\(Alex,.2026-10-28\) DOC:docs/static/next_spin_requirements.md::claude-queue/issues/4 | f1c200s |
| NM-03 | Chip gate: on a dev board, the requirements that would kill the chip: FireRed at full speed, boot from the card, boot time, the panel on the 8080 bus, the USB-C roles; queue issue #11's results are the evidence; a fail sends the project to the V3s (DESIGN.md:77) | NM-02 VER-233 | VER-221 VER-222 VER-223 VER-224 VER-225 | f1c200s |
| NM-04 | Architecture and pin plan: the pin review, the power budget, the rails and their order, the FUSB303B's roles on paper, the chip's heat | NM-03 | VER-232 VER-231 VER-235 VER-234 VER-238 | f1c200s |
| NM-05 | Schematic: ERC, the netlist against design.py, the pin reviews, the application circuits and the datasheet checks | NM-04 | VER-01 VER-03 VER-02 VER-36 VER-34 VER-198 VER-233 | f1c200s |
| NM-06 | Layout and case: DRC, check_fit, the USB pair, the land patterns, the gerbers, and the case gate (its checks and prints) | NM-05 | VER-04 VER-06 VER-236 VER-125 VER-27 GATE:case | f1c200s |
| NM-07 | Docs: bring-up, assembly and the BOM describe the F1C200s board; every bring-up measurement has a probe point | NM-06 | VER-39 VER-161 VER-158 | f1c200s |
| NM-08 | Order gate | NM-07 | GATE:order | f1c200s |
| NM-09 | EVT | NM-08 | GATE:evt | f1c200s |
| NM-10 | DVT and PVT | NM-09 | GATE:dvt GATE:pvt | f1c200s |

## Order gate: before the 5-board EVT order

Only what the copper, the BOM and the fab files freeze blocks the order (O-9). Round 2 found
the order gate too heavy for a 5-board test order, so on 2026-10-26 every check whose result
could only change a DVT spin, a firmware rule or a bench procedure moved to the gate where it
matters (PLAN.md section (d) lists what moved where; check_trace refuses an ORDER check that no
step of section (d) names). The printed case and the assembly docs have their own gate
(G-CASE). Criteria 1-13 are validation_plan.md section 2, the 13-point go/no-go list, in its
order; 14 onwards are this register's.

| ID | Criterion | Needs | Board |
|---|---|---|---|
| G-ORD-01 | vp 1: every IC's pin table is signed off with quotes; no open question or unknown rating left in the pin reviews (both rating schemas read); the application circuits checked against the datasheets on disk; the F1C200s's pin plan and boot pins (VER-232) and its and the FUSB303B's datasheets on disk (VER-233); the ICs the scripts read by script (VER-198), the rest by VER-34 | VER-02 VER-36 VER-34 VER-198 VER-232 VER-233 | f1c200s |
| G-ORD-02 | vp 2: the LCSC identity check and the land-pattern overlay show zero unexplained differences; the BOM, CPL and PCB fields script (VER-130) is G-EVT-08's (VER-14 by hand, VER-28's matched-parts page and VER-132 see the same fields before the order). Was: the FM8002A's shutdown polarity (VER-124) on the ESP32-S3 board | VER-14 VER-125 | f1c200s |
| G-ORD-03 | vp 3: the power hold's default state is in design.py; the charge current is set from the cell's spec sheet (VER-37); the watchdog rule is written (VER-104); the assert that ties the charge current to cell.json (VER-160) is G-EVT-05's | VER-02 VER-37 VER-104 | f1c200s |
| G-ORD-04 | vp 4: the real ER-TFT035IPS-6 lights from the F1C200s dev board on its 8-bit 8080 bus through a confirmed bottom-contact breakout, and its full-screen redraw is timed (VER-224, the chip gate); the folded FPC proves the 51-j map and reaches its connector; the tail thickness is within the FH12 spec | VER-224 VER-136 VER-44 | f1c200s |
| G-ORD-05 | vp 5 (moved to G-EVT-12, F-49): a pill on ENIG reads under 1 kohm and comb leakage is above 100 Mohm, on a bare board from the same order; a coupon ordered now is advisory | ADVISORY VER-45 VER-46 | f1c200s |
| G-ORD-06 | vp 6, the power key: a board switch fits under the DS Lite power slider, or seat 1's Start is the power key (DESIGN.md:64), and its states hold on paper (VER-239). Was: the latch replica, then latch.py on the ESP32-S3 board | VER-239 | f1c200s |
| G-ORD-07 | vp 7, what the copper fixes: part heights from drawings under the case's limits (VER-112 is G-CASE-02's); the pill-to-comb XY chain (VER-174); the positions the board takes from the case (the USB-C, the cell at the hole, the panel's connector, the microSD at its opening, the power slider, the screws, the combs); the MLCC keep-out from the bosses; the thermistor against the cell's hole; a worst-case cell's gap to the routed edge on every side (VER-181); every copper-setting ESTIMATE measured | VER-06 VER-174 VER-19 VER-142 VER-164 VER-181 VER-152 | f1c200s |
| G-ORD-08 | vp 8 (moved to G-EVT-04 on 2026-10-26: the MLCCs' DC bias and the hot-plug simulation change at most BOM values or a DVT clamp, and the bench sees both on unit 1) | ADVISORY VER-30 VER-12 | f1c200s |
| G-ORD-09 | vp 9: JLC limits from the dated capability page; copper, courtyards and silk against every routed edge; the netlist sent with the net ties merged (advisory at ORDER, binding at G-DVT-01); every hole 0.30 mm or more; the drill files split and parsed; the fab files' content checked; the gerbers read on their own: every paste aperture on copper and each exposed pad's paste (VER-27: the F1C200s's QFN-88 and the charger's, DF-219) | VER-17 VER-18 ADVISORY:VER-26 VER-114 VER-131 VER-132 VER-27 | f1c200s |
| G-ORD-10 | vp 10: dry-run screenshots of every polarised part signed off (the F1C200s, the FUSB303B, the charger, each regulator, the amplifier, the USB-C, the microSD slot, the panel's connector, the power switch), the rotation table done (advisory at ORDER, binding at G-DVT-01), every CPL position on its pads (the placement preview is the reference: O-R7) | VER-28 ADVISORY:VER-29 VER-176 | f1c200s |
| G-ORD-11 | vp 11: every check fresh against the gerber hash being ordered, from one release run in a clean worktree, tools recorded; live stock and class confirmed on order day for every line, the F1C200s (C2879851), the FUSB303B (C895444) and the DM3AT (C114218) first, no substitution | VER-143 VER-41 VER-42 | f1c200s |
| G-ORD-12 | vp 12 (advisory, F-32): the clean-room review replies read; and a layout review whose scripted siblings are binding: return paths under the panel bus, the card's bus, USB and the audio (the independent gerber view, VER-27, is G-ORD-09's) | ADVISORY VER-40 VER-21 | f1c200s |
| G-ORD-13 | RETIRED in 2d1124d: vp 13's perfboard tested the TP4056 power path, which left the design; VER-70 measures the BQ24073's unplug on unit 1 (round 2: no signature for a subject that is gone) | RETIRED | ref_design |
| G-ORD-14 | the design round trip is fresh: netlist, ERC, DRC with parity, the contact footprints in the library and as placed, the dome footprint against the dome; build_pcb.py's net classes name only the board's nets and hold every net of 0.25 A or more (VER-201, O-R8, 2026-10-27) | VER-01 VER-03 VER-04 VER-08 VER-162 VER-178 VER-201 | f1c200s |
| G-ORD-15 | the DFMEA is reviewed by Alex and two other models (advisory at ORDER, binding at G-EVT-16: O-R9); every severity 9-10 row names its layers of protection; every assembled part and contact is rated (the bridge table moved to G-EVT-08: it feeds the self-test) | ADVISORY:VER-98 DFMEA:COVERAGE | f1c200s |
| G-ORD-16 | the cell is known: spec sheet in hand (the incoming inspection is G-EVT-02's, before the first cell is plugged); the capacity sample is G-EVT-10's; the plug the spec sheet gives fits the cell's header and cannot enter the speaker's (VER-184's cell half) | VER-37 VER-184 | f1c200s |
| G-ORD-17 | every bring-up probe point has copper: each measurement in bringup.md and each per-unit screen names a TP, a jumper pad or a part pad (the docs sweep and the BOM coverage are G-CASE-06's and G-EVT-02's) | VER-158 | f1c200s |
| G-ORD-18 | the power-path analyses that can move copper or the BOM: the F1C200s's rails and their order (VER-235), the power budget (VER-231), the FUSB303B's roles and the VBUS switch (VER-234), inrush (VER-99), the backlight's worst case (VER-137). The charge window (VER-32) and the VBUS fault reach (VER-177) are advisory here and bind at G-EVT-04 | VER-235 VER-231 VER-234 VER-99 VER-137 ADVISORY:VER-32 ADVISORY:VER-177 | f1c200s |
| G-ORD-19 | the order candidate is named in a release record (branch, commit, tag, design baseline): the F1C200s board's commit on main once its design replaces the ESP32-S3 board's files. Was: DEF-30, the ESP32-S3 board's candidate | VER-42 | f1c200s |
| G-ORD-20 | bench readiness (moved to G-EVT-16 on 2026-10-26: the gear is needed when the boards arrive, not to order them) | ADVISORY VER-116 VER-128 | f1c200s |
| G-ORD-21 | order contents recorded: 5 assembled, 5 bare boards from the same order, rework spares, every order option (vp order size, #72, #189, #193) | VER-42 | f1c200s |
| G-ORD-22 | every ORDER-phase check of the board passes, fresh (O-1: nothing the order needs can hide behind a later gate; advisory-only checks excluded) | PHASE:ORDER | both |
| G-ORD-23 | RETIRED in 39cc07f: the RP2354B's 2 MB left; the N16R8's 16 MB freezes no flash decision at the order, and its partition budget is G-EVT-08's (VER-216) | RETIRED | ref_design |
| G-ORD-24 | the trace gate is shown to fail on seeded faults (the design checkers' seeded faults are G-DVT-05's); no live row of requirements.md, dfmea.csv or verification.csv names a part, part number or net the baseline's design.py lacks, or pairs a connector with another connector's family (VER-188); every repository path the register names is in HEAD's tree or pinned where it was (VER-220) | VER-129 VER-188 VER-220 | f1c200s |
| G-ORD-25 | RETIRED in 574c94c: prod_board deleted from the repo, Alex: not needed. Was: prod_board's own round trip and contacts (as library footprints, as placed, where prod_case says) | RETIRED | prod_board |
| G-ORD-26 | the claims script (moved to G-CASE-06 on 2026-10-26: it keeps the register's quoted values honest, it does not freeze copper; the re-seeds read every row against be437f8, 71737ae and then 39cc07f instead) | ADVISORY VER-161 | f1c200s |
| G-ORD-27 | the coverage grid's pre-order checks (grid.md): each subsystem's requirements read against its functions, the case's architecture against its requirements, the USB-C connector's fault reach, the amplifier's states, the high-speed USB pair (VER-236), the paper heat estimates run to a number, the chip's heat (VER-238); the clocked buses' timing on paper (VER-199). R9 holds every other cell of classes 1-15 that no built pre-order check clears | VER-189 VER-190 VER-195 VER-196 VER-236 VER-144 VER-193 VER-238 VER-199 VER-200 | f1c200s |
| G-ORD-28 | the chip gate (the new-MCU path's NM-03) is exited on the F1C200s dev board: FireRed at full speed, boot from the card with the image's own DRAM init, boot time, the panel on the 8080 bus, the USB-C roles (queue issue #11's results as evidence); a fail moves the project to the V3s before any copper | VER-221 VER-222 VER-223 VER-224 VER-225 | f1c200s |

## Case gate: before the printed case is released and the EVT units are assembled

The case changes daily while the board freezes, so its checks gate the print run and the
assembly, not the board order (F-29). The docs and purchases the hand assembly needs are
here too (G-CASE-06), so they do not hold the order back.

| ID | Criterion | Needs | Board |
|---|---|---|---|
| G-CASE-01 | the Z stack closes with swelling, the buttons work under the shell, no unintended openings, the screws engage per pilot, the Start/Select strip clears the domes, the case's material is rated for its hottest surface. Was: the ESP32-S3 board's power key cap (VER-213) | VER-20 VER-182 VER-07 VER-113 VER-147 VER-180 VER-192 | f1c200s |
| G-CASE-02 | the printed case with real parts, its feel against a genuine DS Lite, incoming silicone, caps and domes; the pill-to-comb registration on the fit kit (VER-51), the part heights from drawings (VER-112) and the connector mouths (VER-165) moved here from the order gate (F-R9) | VER-48 VER-50 VER-126 VER-51 VER-112 VER-165 | f1c200s |
| G-CASE-03 | the case is released: case.scad commit, STL hashes, material, slicer profile, tape part number | VER-173 | f1c200s |
| G-CASE-04 | case-level requirements: size (today's or smaller, and the spin's size pass, VER-230), keyed assembly, no openings, room to swell, the domes | REQ-PRD-17 VER-230 REQ-MEC-04 REQ-MEC-08 REQ-BAT-07 REQ-BTN-08 | f1c200s |
| G-CASE-05 | every check up to CASE passes, fresh | PHASE:CASE | both |
| G-CASE-06 | before any unit is assembled: the docs match the design (the stale-doc sweep and the claims script), every contact class's parts are bought, the speaker's plug fits its header and its leads reach it (VER-202); R4 holds the defects whose Gate is CASE | VER-39 VER-161 VER-167 VER-202 | f1c200s |

## EVT exit: on the 5 assembled boards, before the DVT changes are frozen

Evidence for a per-unit check names the units it ran on (`UNITS:` in its header). "Every
unit" means all boards that will be kept; a sacrificial board may skip the safety soak but
not the FAI. The criteria are evaluated at EVT exit, but G-EVT-02's first item, bringup.md
matching the design (VER-39), and G-EVT-16's bench gear are preconditions of the first power
(PLAN.md section (e), EVT day one).

Unit allocation (O-8): the screens whose results decide the DVT changes run here, on a
sacrificial unit, so the DVT spin is made knowing them.

| Unit | Use | Ships? |
|---|---|---|
| 1 | FAI (VER-109), full bring-up, golden readings (VER-171), cell mating (VER-141), the power hold and hang tests (VER-243, VER-105), the charge timer into a load (VER-145), the microSD card (VER-209), the power slider (VER-240), boot time (VER-228), FireRed at full speed (VER-247), games and saves (VER-229) | no (FAI) |
| 2 | bring-up, soak, thermal (VER-85, VER-86, VER-87), warm box (VER-146), panel bond (VER-154); with unit 3, GBA link play (VER-226); online battles (VER-227) | yes, if its connectors are within budget (VER-157) |
| 3 | bring-up, soak, play and charge time (VER-101, VER-102), cutoff sweep (VER-139) | yes, as unit 2 |
| 4 | abuse and margins (VER-97), ESD screen (VER-95), near-field (VER-96), drop (VER-93), button life (VER-94), fault injection (VER-168) | never |
| 5 | spare; replaces any unit lost before its tests finish | yes, as unit 2 |

| ID | Criterion | Needs | Board |
|---|---|---|---|
| G-EVT-01 | first article inspection on unit 1, from the generated sheet, before any other unit is powered | VER-166 VER-109 | f1c200s |
| G-EVT-02 | first power done safely on every unit: bringup.md matches the design before the first unit is powered (round 2's objection: bring-up of the bare boards does not wait for the case gate), JP1-cut first power from a 100 mA bench supply, unpowered checks, cell polarity, the charge-path screen with no cell; the cell's incoming inspection (VER-49) moved here from the order gate, before the first cell is plugged (F-R9) | VER-39 VER-74 VER-52 VER-53 VER-54 VER-49 VER-133 | f1c200s |
| G-EVT-03 | bring-up on every unit: the bring-up image boots (VER-237), recovery over the card and FEL with the case closed and the debug console (VER-241), the power hold after a watchdog reset and a panic (VER-243), then the contacts, the panel, the backlight, audio and the power path | VER-237 VER-241 VER-243 VER-56 VER-60 VER-61 VER-62 VER-63 VER-64 VER-65 VER-66 VER-67 VER-68 VER-69 | f1c200s |
| G-EVT-04 | power-path margins analysed and measured: the hot-plug simulation and the MLCCs' DC bias (from G-ORD-08), copper IR drop, ratings, the unplug at a held 3.50 V, hot plug, each rail's regulation, a card's insertion (VER-206, VER-209); from the order gate: the back-power paths (VER-159), the cutoff accuracy (VER-138), the cell mating peak (VER-140), the charge window (VER-32) and the VBUS fault reach (VER-177) | VER-12 VER-30 VER-13 VER-31 VER-70 VER-71 VER-83 VER-206 VER-209 VER-138 VER-140 VER-159 VER-32 VER-177 | f1c200s |
| G-EVT-05 | cell safety on built units, the bench charging rules kept first: per-unit CV, thermistor and cell-current screens; temperature pause; cell temperature in a full charge; the thermistor tracking; PCM trip; cell mating; the charge timer; off current warm; cutoff accuracy and calibration; hang test; the charge current and window tied to cell.json (VER-160) | VER-172 VER-134 VER-135 VER-169 VER-82 VER-69 VER-85 VER-87 VER-88 VER-141 VER-145 VER-146 VER-139 VER-155 VER-156 VER-105 VER-160 | f1c200s |
| G-EVT-06 | touch temperature within the IEC 62368-1 TS1 limit, with the heat table (every part, the chip (VER-238), the cell's own heat and the case's inner surface) and its firmware rule in the image | VER-144 VER-193 VER-238 VER-86 | f1c200s |
| G-EVT-07 | the power slider on unit 1 (VER-240), USB sources and cables in both orientations and as host (with the 4.40-4.75 V characterisation), input current; GBA link play on two units (VER-226) and online battles as a Showdown guest (VER-227) | VER-240 VER-80 VER-81 VER-226 VER-227 | f1c200s |
| G-EVT-08 | the self-test image on the generated pin file runs on every unit with the bridge table it tests; the chip's identity and its 64 MB logged (VER-242) against its user manual and errata (VER-245); the bring-up image (VER-237); FireRed at full speed on unit 1 (VER-247); the BOM, CPL and PCB fields script (VER-130) | VER-149 VER-237 VER-148 VER-245 VER-76 VER-242 VER-247 VER-130 | f1c200s |
| G-EVT-09 | mechanics with real parts: FPC and ZIF, assembly and opening demo in the safe order, flatness, microscope inspection, panel bond | VER-106 VER-91 VER-90 VER-89 VER-154 | f1c200s |
| G-EVT-10 | play time, charge time, usable capacity per lot, the shelf estimate and cost; boot time on unit 1 (VER-228) and games and saves over USB and the card (VER-229) | VER-101 VER-102 VER-153 VER-103 VER-117 VER-228 VER-229 | f1c200s |
| G-EVT-11 | every check up to EVT (ORDER, CASE and EVT phases) passes, fresh, against the design going into DVT | PHASE:EVT | both |
| G-EVT-12 | vp 5 on a bare board from the order: pill resistance and comb leakage, day one, before any assembled unit is powered (F-49) | VER-45 VER-46 | f1c200s |
| G-EVT-13 | the DVT-deciding screens on unit 4: abuse with a reversed cell, ESD screen at the combs, near-field, drop, button life, fault injection (every credited detection trips) | VER-97 VER-95 VER-96 VER-93 VER-94 VER-168 | f1c200s |
| G-EVT-14 | unit records: serials, findings with disposition and re-test, golden readings, connector mating counts | VER-110 VER-171 VER-157 | f1c200s |
| G-EVT-15 | RETIRED in 574c94c: prod_board deleted from the repo, Alex: not needed. Was: prod_board on its own bare boards, every contact reads a pill under 1 kohm and leaks over 100 Mohm | RETIRED | prod_board |
| G-EVT-16 | bench readiness and the analyses that need no copper (moved from the order gate on 2026-10-26): gear on hand for every EVT check (`check_trace.py --shopping --gate evt`), the speaker's rating, the backlight and audio simulations, the hashed datasheet archive; the DFMEA review (VER-98) binds here (O-R9) | VER-116 VER-128 VER-05 VER-100 VER-35 VER-98 | f1c200s |

## DVT exit: on the revised spin

| ID | Criterion | Needs | Board |
|---|---|---|---|
| G-DVT-01 | every check up to DVT passes on fresh evidence; every test of phase EVT or later re-run on the DVT spin (its result covers a REL-...-DVT record) | PHASE:DVT | both |
| G-DVT-02 | durability on the DVT spin: open/close, drop, button life, margins and abuse; the card's endurance for saves and the battle log (VER-244); a 72 h soak of GBA play at 40 C (VER-250) | VER-92 VER-93 VER-94 VER-97 VER-244 VER-250 | f1c200s |
| G-DVT-03 | ESD to IEC 61000-4-2 with a real gun, and the EMC pre-scan | VER-170 VER-96 | f1c200s |
| G-DVT-04 | every defect CLOSED, each with a passing check | DEFECTS:CLOSED | both |
| G-DVT-05 | the checkers are shown to fail on seeded faults | VER-22 VER-129 | f1c200s |
| G-DVT-06 | the cell's plug cannot enter the speaker's header, and the speaker's cannot enter the cell's, on the DVT copper (DF-170, DEC-33): VER-184 and VER-202 re-run on the DVT spin (both halves moved earlier on 2026-10-27, F-R2) | VER-184 VER-202 | f1c200s |

## PVT exit: the 30-board run

| ID | Criterion | Needs | Board |
|---|---|---|---|
| G-PVT-01 | release record complete: tag, commit, design baseline, fab zip, BOM and CPL sha256, BOARD_REV (bumped for any BOM change), locked BOM with LCSC numbers, no-substitution list (the F1C200s, the FUSB303B and the DM3AT included), case section, gate output, waivers | VER-42 | f1c200s |
| G-PVT-02 | first article inspection on the first PVT unit (VER-109's result must cover a REL-...-PVT record: an EVT first article does not count) | VER-109 | f1c200s |
| G-PVT-03 | a production record with serial for every unit, its calibration and mating counts, and incoming inspection of every cell | VER-110 VER-155 VER-157 VER-49 | f1c200s |
| G-PVT-04 | first-pass yield 90% or more, every failure logged; golden readings re-derived for PVT | VER-111 VER-171 | f1c200s |
| G-PVT-05 | every check up to PVT passes on fresh evidence; every test of phase EVT or later re-run on the PVT units (its result covers a REL-...-PVT record) | PHASE:PVT | both |
| G-PVT-06 | only if any unit is sold or given away: cell transport documents and emissions (the board has no radio). Was: the ESP32-S3 module's radio grant (VER-217) | VER-38 VER-118 | f1c200s |

## Waivers

Each row proposes a waiver and its reason. It counts only when Alex signs it in
[releases/APPROVALS.md](releases/APPROVALS.md) with decision WAIVED and the last phase it
holds for; check_trace counts a signature only from a commit signed with the key README.md
pins (README, Approvals), and refuses one in a commit that carries a Claude trailer or in a
merge.

| ID | Criterion | Proposed reason |
|---|---|---|
| W-01 | G-ORD-06 | not needed since 2026-10-26: G-ORD-06 names VER-33, the check that replaced the latch replica (round 2's objection); kept for its ID |
| W-02 | G-ORD-13 | not needed since 2026-10-26: G-ORD-13 is RETIRED with the TP4056 power path (2d1124d); kept for its ID |
| W-03 | G-PVT-06 | applies only if a unit leaves Alex's hands |
