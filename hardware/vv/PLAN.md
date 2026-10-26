# The plan: from here to 5 boards, then 30

**Not ready to order.** The board being ordered is `hardware/ref_design` at **be437f8**
(branch ref-design-35in, 2026-10-26; the board itself is final at 62b01f2, the later commits
are docs and print kits). On 2026-10-26 this register, which described 684a609, was
re-seeded for it, the board-level checks that found real problems were re-run, round 2's 18
sign-off objections were answered (section g) and the order gate was trimmed to what the
copper, the BOM and the fab files freeze (section d). The default mode passes; the order gate
fails, as it should (section i).

Contents: (a) the system on one page; (b) what changed in the design; (c) the board at
be437f8: what the checks find; (d) before the 5-board order, and what moved where; (e) the
case gate, EVT, DVT and PVT; (f) decisions for Alex; (g) round 2's sign-off objections and
where each landed; (h) what this plan still cannot catch; (i) the gate today; (j) past bugs and
what catches them; (k) the ID ledger of the first run's debate; (l) the second review's ID
ledger; (m) the first run's sign-off objections.

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
- Every ORDER check is named by a step of section (d) below, so the checklist and the gate
  cannot drift apart (round 2, O). A requirement has a live check no later than its Phase, so a
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
- RETIRED: a requirement, risk, check, defect or gate criterion whose subject left the design
  keeps its ID and names the commit that removed it; no live row may lean on a retired check
  or cite a retired id outside 'Was: ...' (O-R9, F-R14).
- Each defect names its Gate; R4 holds it there and at every later gate, not before (O-R5).
- The design baseline in README.md names every design path and must equal HEAD, or the
  default mode fails (DEF-47: on 2026-10-26 the rebase onto be437f8 failed it at once, as
  designed). README names the order branch: a design commit on it after be437f8 warns in the
  default mode and fails the order gate (R8, O-R13).

`tools/seed_faults.py` seeds faults one at a time and catches every one (VER-129; the count is
in section i). `tools/design_numbers.py` recomputes the pre-order numbers this plan quotes; its
CPL and drill checks run fab.py's own export into a temp dir and read the files it writes
(O-R1). `tools/run_checks.py` runs it and the other quick checks and writes their evidence.

## (b) What changed in the design (684a609 to be437f8)

`git diff --stat 684a609 be437f8 -- hardware/ref_design`: 118 files, most of them the case's
images, print kits and the viewer. What touches this register:

| Change | Commits | Register effect |
|---|---|---|
| The panel back to the ER-TFT035IPS-6 (ILI9488, 320 x 480, six LEDs on two cathode pins, 120 mA); J3's map restored (SCK on panel pin 36, D/C on 37, IM2-0 high, panel pins 4-6 not connected); R61 2.4k to 3.9k (C51721) | 79de1f7, ada53ea | REQ-PRD-02.R, REQ-DSP-01, 02, 05-09, 11, 12 rewritten; DF-44, 57, 59, 60, 130, 159 rewritten; DF-180 new (the 18-bit redraw; Was: DF-120); VER-43, 60, 61, 62, 137 rewritten; DEF-39 reopened; vp #58 un-cut |
| The board re-placed: 72.4 x 151.64 mm (was 127.7); the cell's 54.0 x 37.0 mm hole under the glass's seat-2 half; TH1 at (127.8, 79.0); the shift registers and the backlight's sinks in the ends' spines; the speaker sunk in an 11.0 x 16.0 mm hole at the right edge; J4's mouth towards the speaker | 9cff0d3, afa322e | DF-04, 62, 97, 156, 165, 170, 171 rewritten; DF-178 (the 1.0 mm rim), DF-179 (copper at the routed edges) new; VER-18 built; VER-90, 166 rewritten; DEF-56, DEF-57 new |
| Routed: 0 unconnected, DRC 0 errors, with schematic parity; route.py's maze reaches 40 mm; one GND via by U3's NC pad nudged 0.08 mm by hand in the board file (a rebuild brings it back) | 62b01f2 | VER-01 to 04 re-run; the hand edit is in (c) and (h) |
| The case: the one-piece top (the island, the decks and their arm hooks gone), 77.4 x 156.64 x 9.6 mm (7.7 at the button ends); the cell under the panel; the D-pad opening 0.60 | 2c73cea, d23d5c1, ec7859c, 382870c | REQ-PRD-06, MEC-02, 05, 08, 09, 10 rewritten; DF-84, 107, 119 rewritten; DF-89 RETIRED; DF-181 new; VER-07, 48, 92, 113, 147, 182 rewritten; DEF-51 CLOSED; DEF-31 restated |
| Docs: bom.md (the panel, M2 x 6, 44 part numbers, 118 placements, the board's size), assembly.md (the one-piece top), pinmap.md (113 mA, an 18% minimum duty), design_review.md | e672f90, 03df0f2 | DEF-21's list re-checked: bringup.md's display steps are still the 2.4 in panel's (n) |

What stayed: every other part, value, LCSC number and net (design.py's diff is the panel's
comment, its pin map and R61), board/lib, fab.py and silk.py. So the drill, CPL and
order-number findings stand as they were.

## (c) The board at be437f8: what the checks find

Every number below comes from `tools/design_numbers.py` run on be437f8's design files
(evidence `evidence/2026-10-26_*`, recorded in section i's commit) unless it says otherwise.
The four checks the brief asked to re-run first:

| Check | At be437f8 | Verdict |
|---|---|---|
| The cell's gap, every side (VER-181) | the tub's pocket is still 0.25 mm inside the 54.0 x 37.0 mm hole on every side, so a worst-case 52 x 36 cell is 0.25 mm from the routed edge at the pushed end and on both long sides (-0.15 mm after 0.40 mm of routing, print and screw tolerances), and 1.75 mm at its lead end (1.35 after, against 1.5); a nominal 50 x 34 cell fails the same three faces at -0.15 mm and passes its lead end (3.35 mm). What passes: the pocket 0.9 mm inside the hole with a cell of at most 51.2 x 35.2 mm, or a hole of at least 54.8 x 37.8 mm | still failing (DEF-53, DEC-27) |
| J1's pick-and-place offset (VER-176) | the CPL fab.py writes still places J1 1.50 mm from its pads' centre; J2 and J4 0.95 mm, J3 0.23 mm; 44 BOM lines, 118 CPL rows | still failing (DEF-33) |
| The drill file (VER-131) | one MixedPlating Excellon of 628 holes and 4 slots: J1's four 0.60 mm plated slots and every via sit with the unplated SW20-SW23 vents (0.5 mm), J1's pegs (0.65 mm) and H1-H6 (2.2 mm) | still failing (DEF-32) |
| The cell's plug into J4 (DF-170, VER-184 new) | J2 and J4 are still both the 53261-0271 (C177225). They now sit on opposite long edges, 64.2 mm apart centre to centre, and the speaker's 35 mm leads cannot reach J2 (67.1 mm), but with the case open the cell can be anywhere | risk lower, cause unchanged: VER-184 FAIL (a DVT check), DEC-33 |

What else the re-run finds, new or changed at be437f8:

| Finding | Numbers | Where |
|---|---|---|
| **The backlight is over the panel's 120 mA again** | 122.4 mA at 100% duty at worst case (61.2 mA a pin, 20.4 mA an LED on average): 79de1f7 brought back the first run's finding with the panel; a duty cap at 97.9% or R61 3.74k passes | VER-137 FAIL, DEF-39 reopened, DF-130, DEC-31 |
| **The board's name, REV 1 and build text are routed away** | silk.py writes them at y 57.6-63.2 on the back; the cell's hole runs y 57.0-94.0 there. BOOT and RST are written at negative coordinates (off the board); the speaker's label sits in its hole. KiCad's DRC called the first three 'cosmetic' silk warnings | VER-18 FAIL, DEF-57, DEC-28 |
| **Copper 0.20 mm from the routed holes** | GND pours on both layers 0.20 mm from the cell's and the speaker's holes; J2's mounting pad 0.275 mm from the left edge; J2's and J4's courtyards 0.22 and 0.12 mm past the outline; REQ-FAB-03 asks 0.30 mm (JLC's figure to confirm) | VER-18 FAIL, DEF-56, DF-179, DEC-32 |
| A 1.00 mm FR4 bar beside the speaker's hole | the speaker's hole leaves 1.00 mm to the board's right edge over 16 mm (case.scad asserts 0.9) | DF-178 |
| The 18-bit redraw | the ILI9488 takes only 18-bit colour over SPI: 461 kB a frame, 92 ms at 40 MHz, 184 ms at 20 MHz | DF-180 (H), REQ-DSP-11, DEC-13 |
| The panel's mechanics are estimates | its outline, active area, thickness and FPC shape are the old 3.5 in design's (its datasheet could not be downloaded on 2026-10-26); they set J3's place | DF-61, DF-123, VER-136, VER-152 |
| A hand edit in the board file | one GND via moved 0.08 mm by hand; the next build_pcb run undoes it | (h); VER-143 must rebuild and re-run DRC |
| The order branch moved | 2ddb7fe (2026-10-26 16:48), after be437f8, engraves button labels 0.3 mm deep on the top's inside: case.scad only, no board file | R8 now holds the case gate, not the board order; VER-113 and VER-07 re-check it before the case release |
| Swelling room | 0.80 mm under the panel's back (0.30 under a deck at 684a609): passes | VER-182 PASS, DEF-51 CLOSED |
| Screws | M2 x 6 gives 3.2 mm of thread in the side pilots (want 4.0; M2 x 7 passes, its tip 0.8 mm short) and 3.2 mm of the end pilots' 3.30 (ok) | VER-147 FAIL, DEF-31, DEC-18 |
| Unchanged | cutoff 3.521 V at the worst corner (VER-138 FAIL); VBUS fault reach 6.9 V (VER-177 FAIL); bom.md's domes and Start/Select caps (VER-167 FAIL); TH1 0.80 mm from the hole's edge, no back pad within 1 mm (nearest TP3, 5.5 mm: VER-164 PASS); nearest MLCC C53, 11.40 mm from H5 (VER-142 PASS); every hole 0.30 mm or more (VER-114 PASS); VBUS 10 uF (VER-99 PASS) | as named |

## (d) Before the 5-board order

Round 2's two reviewers called the order gate (67 checks, 10-12 days) too heavy for a 5-board
test order. It now holds only what the copper, the BOM and the fab files freeze, or what would
make the five boards useless or unsafe. Every ORDER check is named in a step below
(check_trace fails otherwise). Effort is Claude's unless it says Alex.

| # | Step | IDs | Effort |
|---|---|---|---|
| 0.1 | **Alex confirms be437f8** as the order candidate (DEC-01). A later commit to ref-design-35in that touches a design path makes the default mode warn and the order gate fail (R8) until the register is re-checked and the baseline moved. Clean the main checkout's uncommitted battler.kicad_pro before any fab run (O-34). | DEF-30 | Alex: 5 min |
| 0.2 | **Order everything with a lead time** (`--shopping --gate evt`): RP2354B and L2 pre-order at JLC; cells from a seller with a datasheet (DEC-23); the ER-TFT035IPS-6, an FH12 breakout, an RP2350B dev board; two GSPK151035PN and a 53261-0271; domes and Start/Select strips; calipers, a 4.096 V reference, clip leads, a LiPo bag, a USB-C breakout, 133 ohm resistors. | VER-41, VER-37, VER-43, VER-49 | Alex: 1 h, about $100-150 |
| 0.3 | **Run the register on every commit**: one line in scripts/check.sh, `python3 hardware/vv/tools/check_trace.py` (outside hardware/vv, so it waits for Alex's OK). It would have caught DEF-39's return in 79de1f7. | DEF-47 | 15 min |
| 0.4 | **Alex pins a signing key** (README, 'Approvals': a hardware key or a passphrase key never in an agent). Until then no APPROVALS.md line counts (DEC-30). | | Alex: 15 min |
| 1 | **Copper, silk and BOM decisions**, then one design pass in ref_design (outside this register) and a re-route where copper moves: DEC-28 silk.py (the board's name, BOARD_REV and labels out of the holes, the JLC marker), DEC-32 the pours' edge clearance to 0.3 mm and J2 0.1 mm in, DEC-27 the cell's gap (a cell spec and the pocket, or the hole), DEC-31 the backlight's full scale, DEC-05 TH1's B, DEC-17 the VBUS reach, DEC-24 the pull-ups, DEC-22 the fuse and DEC-33 J4 (copper now, or accepted for EVT and done at DVT). | VER-18, VER-181, VER-137, VER-32, VER-177 | Alex: 1 h; the design pass half a day, a re-route about 2 h if J2, J4 or a fuse move |
| 2 | **Re-run the scripted checks** after step 1 (`run_checks.py`, about 10 min, then VER-129 on its own): netlist, pin gate, ERC, DRC, contact footprints, inrush, holes, MLCC, TH1, CPL, drill, VBUS, cutoff, backlight, cell gap, edges. | VER-01, VER-02, VER-03, VER-04, VER-08, VER-99, VER-114, VER-142, VER-164, VER-176, VER-131, VER-177, VER-138, VER-137, VER-181, VER-18, VER-129 | 30 min |
| 3 | **Fab outputs** (VER-131 and VER-176 read fab.py's real output and fail today): separate drill files and a centroid table in fab.py; the fab content check over the exported files (the combs' paste, mask and keep-outs, the plated slots, the vents, the JLC marker, every silk text on the board); JLC's dated limits; the IPC-D-356 netlist with the net ties merged; BOM, CPL and PCB fields against design.py; the rotation table; land patterns; contacts as placed; the dome footprint against its drawing; the release run that makes the files twice and compares them. | VER-131, VER-176, VER-132, VER-17, VER-26, VER-130, VER-29, VER-125, VER-162, VER-178, VER-143 | 2 days |
| 4 | **Layout reviews** (round 2: these were in no step): the copied core against Raspberry Pi's RP2350B minimal design (with the second 4.7 uF on 1V1); the USB pair's stubs; the GPIO mux and PWM slices; the back-power path table. | VER-23, VER-108, VER-15, VER-159 | 1 day |
| 5 | **Power-path analyses that can move copper or the BOM**: the latch table (both Starts); the charge window from SLUS810N's TS limits; the cutoff with the pad leakage; the mating peak against the PCM (and the fuse); the VBUS fault reach; the backlight; inrush. | VER-33, VER-32, VER-138, VER-140, VER-177, VER-137, VER-99 | 1.5 days |
| 6 | **Datasheets and part identity**: the pin gate reads both rating schemas, its open findings and waivers, and IC identity (DEF-26, O-R20); the application circuits and every number this register quotes from memory (RP2350 leakage, OVP minimum, I_TS, pre-charge and termination, the ILI9488's SPI cycles); LCSC identity; the watchdog rule and FM8002A polarity (both pass). | VER-36, VER-34, VER-14, VER-104, VER-124 | 1.5 days |
| 7 | **The board against the case**: heights from drawings and check_fit; the XY chain and the pill print on the fit kit; placement drift; connector mouths; the MLCC keep-out and TH1 (both pass); the ESTIMATE ledger, the panel's first. | VER-112, VER-06, VER-174, VER-51, VER-19, VER-165, VER-142, VER-164, VER-152 | 1 day plus calipers |
| 8 | **The panel and the cell, parts in hand**: the ER-TFT035IPS-6 lit through the breakout with J3's map and its redraw timed; the FPC folded into a breakout in J3's place; the tail's thickness; the cell's spec sheet, cell.json and the incoming inspection ready. | VER-43, VER-136, VER-44, VER-37, VER-160, VER-49 | Alex and Claude: an evening each once parts arrive |
| 9 | **Test access and firmware size**: every bring-up probe point has copper (the probes table and its script); the game inside 2 MB with a flash map. | VER-158, VER-151 | half a day |
| 10 | **Reviews**: the DFMEA review (Alex and two models); the advisory clean-room, return-path and gerbv reviews if time allows (VER-40, VER-21, VER-27). | VER-98 | Alex: half a day |
| 11 | **Alex's decisions** in APPROVALS.md, in signed commits: one `REQ-ALL@<commit>` line for the requirements he agrees with (round 2: not 130 lines), deferrals by ID; the H rows that only built units can detect, accepted for ORDER (`--gate order --verbose` lists them; one `DF-H@<commit>` line or one per row); u4 pin 58's waiver (DEC-29). | R1, R3 | Alex: 1 h |
| 12 | **Release and order**: the release record; the dry run (valid 7 days); stock on order day (valid 1 day); `check_trace.py --gate order` exits 0; order 5 assembled and 5 bare boards. | VER-42, VER-28, VER-41 | 1 day |

prod_board's own order list (`--gate order --board prod_board`, Alex's board, evaluated
apart): VER-10, VER-119, VER-120, VER-121, VER-122, VER-163.

Rough total: about 8.5 working days of Claude's work (it was 10-12), 3-4 hours of Alex's, a
design pass of half a day in ref_design, and up to three weeks of parts lead time in parallel.
The ORDER checks for ref_design went from 67 not passing to 45 (12 pass); a further 23 run
on the first boards or the DVT spin instead.

**What moved where on 2026-10-26** (round 2's proportionality objection, O and F):

| Check | Was | Now | Why |
|---|---|---|---|
| VER-05 backlight sim, VER-100 audio sim, VER-35 datasheet archive, VER-116 gear, VER-128 speaker rating | ORDER | EVT (G-EVT-16) | BOM values or bench readiness; the bench sees the same things on unit 1 |
| VER-12 hot-plug sim, VER-30 DC bias, VER-13 copper IR drop, VER-31 ratings and the 3V3 budget | ORDER | EVT (G-EVT-04) | each could change a BOM value or a DVT clamp, not the five boards; VER-13 was reached by nothing but PHASE:ORDER (F) |
| VER-144 heat per mode | ORDER | EVT (G-EVT-06) | it sets a firmware rule before the touch test |
| VER-148 bridge table, VER-149 pin file, VER-150 rp235xb self-test, VER-16 errata | ORDER | EVT (G-EVT-08) | test tooling and the stepping, needed when the boards arrive |
| VER-153 capacity per lot, VER-103 shelf estimate | ORDER | EVT (G-EVT-10) | no copper or BOM decision depends on them (O); R5 follows the spec sheet |
| VER-161 claims script | ORDER | CASE (G-CASE-06) | it keeps quoted values honest; this re-seed read every row against be437f8 |
| VER-22 seeded faults for the design checkers | ORDER | DVT (G-DVT-05) | the checkers must be proven before the DVT copper freezes |
| VER-21 return paths, VER-27 gerbv view | ORDER | advisory (G-ORD-12) | their scripted siblings (VER-04, VER-131, VER-132) are binding (F) |
| VER-24 reproducibility, VER-25 combs in the gerbers, VER-115 test points by hand | ORDER | CUT, folded into VER-143, VER-132 (b), VER-158 | the same file read twice |
| VER-34 inherited claims | ORDER, reached by nothing | ORDER, in G-ORD-01 | it supplies the constants design_numbers.py quotes from memory (F) |

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
with charging off (VER-169); CV into a 133 ohm load (VER-134: 4.158-4.242 V; the window from
`design_numbers.py cvload`); TS by a finger and a 4.7k (VER-135). Only then a real cell,
attended, in a LiPo bag (VER-172).

Unit allocation (gates.md): unit 1 FAI, full bring-up, golden readings, the charge timer into
a load (VER-145) and SR_CLK (VER-179, now 176 mm of track across the board); units 2 and 3
bring-up, soak and thermal; unit 4 abuse, ESD screen, near-field, drop, button life and fault
injection (it never ships); unit 5 spare.

Exit: `--gate evt` exits 0 (G-EVT-01 to G-EVT-16; G-EVT-15 is prod_board's).

### DVT: the revised spin

Candidates, decided from EVT's results. BOM-only (each bumps BOARD_REV): 4.7k pull-downs on
AMP_EN and BL_PWM (DF-45, DF-47); R18 56k (DF-95, with DEC-07); RN1-RN4 47k if not done
before (DF-164); R14 47k and R43 220k if not done before (DF-101); R61 3.74k if the firmware
cap was chosen for EVT (DF-130). Copper: J4 keyed apart from J2 (DF-170, G-DVT-06, if not
done before the order); reverse-polarity protection or a keyed connector on CELL_P (DF-55);
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
waivers) or into a commit. Claude's proposal follows each. The decisions are DEC-nn since
2026-10-26: round 2 (O) found 'D2' or 'D8' read as the diodes D2 and D8. The numbers are the
old ones (D5 is DEC-05).

**The design and the order**

- **DEC-01. Is be437f8 the order candidate?** Proposed: yes (this brief). It goes into the
  release record (DEF-30).
- **DEC-02. Are the RP2354B and L2 pre-ordered?** Both were out at JLC and LCSC on 2026-10-21
  and are on the no-substitution list.
- **DEC-03. Will any of the 30 be sold or given away?** That decides the SALE items (UN38.3,
  emissions, marking, DF-87).

**Before the order: copper, silk or BOM**

- **DEC-28. silk.py: the board's texts and JLC's marker** (DEF-57, DEF-55). The board's name,
  'REV 1' and the build line sit in the cell's hole; BOOT and RST are off the board; the
  speaker's label is in its hole. Proposed: move them in silk.py to a clear patch on the back
  (VER-18 then checks every text is on the board) and add the 'JLCJLCJLCJLC' marker there. No
  copper moves; a rebuild and a new baseline. The alternative for the marker is JLC's paid
  removal option with REQ-FAB-05 restated; there is none for BOARD_REV, which the FAI and every
  BOM-only change rely on.
- **DEC-32. Copper at the routed edges** (DEF-56, DF-179): the pours 0.20 mm from both holes,
  J2's mounting pad 0.275 mm from the left edge. Proposed: raise the pours' edge clearance to
  0.3 mm in build_pcb.py and move J2 0.1 mm in, with the next rebuild; or show JLC's dated page
  accepts 0.2 mm and restate REQ-FAB-03.
- **DEC-27. The cell's gap on every side** (DEF-53, DF-85, VER-181). A worst-case 52 x 36 cell
  is -0.15 mm from the edge after tolerances at the pushed end and both long sides, and 1.35 mm
  at its lead end. Widening one end cannot help (the pocket follows the hole: seeded in
  VER-129). Options, re-costed: (a) buy a cell whose spec sheet guarantees 51.2 x 35.2 mm or
  less with its PCM (VER-37, DEC-23), set case.scad's worst case to it and move the pocket 0.9
  mm inside the hole: no copper; (b) a 54.8 x 37.8 mm hole with the pocket 0.9 mm inside it:
  copper, TH1 moves 0.4 mm and the strips lose 0.4 mm each; (c) restate REQ-BAT-08 to count the
  printed pocket wall as the barrier: it does not hold either, since the FR4 edge can stand
  0.15 mm inside the wall's line. Proposed: (a), with (b) at DVT if no such cell is sold.
- **DEC-31. The backlight's full scale** (DEF-39, DF-130, DF-159): 122.4 mA worst case against
  the panel's 120 mA. Proposed: R61 3.74k (BOM only: 117.7 mA worst case, 108 mA nominal,
  barely dimmer), or a 97% cap written in pinmap.md (no BOM change; VER-137 reads it).
- **DEC-05. The charge window's hot edge is 47.2 C nominal** (TH1 against U2's fixed TS
  thresholds; DEF-48, DF-147). Proposed: (a) a BOM-only TH1 change to an 0402 10k NTC with B
  near 4250 if JLC stocks one (same pads, about 4.9-45.5 C); else (b) accept 47.2 C against the
  cell's datasheet. REQ-BAT-03 and VER-32 are one-sided.
- **DEC-22. A fuse in the cell path** (DF-172). A short downstream of J2 ends only at the cell's
  PCM. Proposed for this order: accept the PCM as the only layer for the five EVT units, which
  stay with Alex and charge attended in a LiPo bag (VER-172); a 1206 fast fuse of about 2 A in
  CELL_P between J2 and JP5 in the DVT copper. If copper is re-routed anyway (DEC-33 now), add
  the fuse in the same pass.
- **DEC-33. J4 keyed apart from J2** (DF-170, VER-184): only copper removes the cause (round 2,
  O). J4 as a JST SH SM02B-SRSS-TB (2-pin, 1.0 mm) with the speaker's leads re-terminated, or
  another 2-pin header the cell's PicoBlade plug cannot enter. Proposed: accept the procedural
  layer for EVT (J2 and J4 on opposite edges, 64.2 mm apart; assembly.md plugs J2 while J4 is
  empty; the five units are Alex's) and change J4 in the DVT copper (G-DVT-06). If Alex prefers
  it now, it costs a local re-route by J4.
- **DEC-17. A faulty USB source reaches U3's EN at 6.9 V** (DEF-50, DF-101). Proposed: R14 47k
  and R43 220k (BOM only): the reach goes to about 9.1 V (EN) and 12.4 V (EN1); 20 V needs a
  clamp (copper). Or accept for EVT.
- **DEC-07. The cutoff and R18, decided together.** The cutoff can fire 35 mV low after a CV
  calibration and never fire on a high-3V3 unit (VER-138: 3.521 V); GPIO47's pad leakage moves
  VBAT's reading 0.47 V per uA through 235 kohm. Proposed: decide after VER-34 quotes the
  RP2350's leakage: keep 470k/470k, the bench calibration at 3.50 V (REQ-TST-06) and a
  firmware dropout rule if the leakage allows it; else a lower divider, a buffer, or
  REQ-BAT-12 stated at 25 C. A voltage reference needs copper.
- **DEC-24. The pill margin** (DEF-49, DF-164): RN1-RN4 at 47k (BOM only) restores about 18
  kohm of margin.
- **DEC-11. E9 pull-downs**: 4.7k on AMP_EN (R37) and BL_PWM, BOM only, or rely on firmware
  and the stepping (VER-75). Proposed: wait for the stepping.

**Requirement numbers** (all live ones are PROPOSED; one signed `REQ-ALL@<commit>` line
approves those Alex agrees with)

- **DEC-04.** Play time 4 h; charge time 4.0 h from 0.5 A (the BQ24073's USB500 limit is 475
  mA); 100,000 presses; $45 a unit. Proposed: DEFERRED with scope EVT for REQ-PRD-03.R,
  REQ-PRD-04.R and REQ-BAT-02.R until the cell's spec sheet and a capacity sample exist.
- **DEC-08.** Legacy USB hosts: U2 charges from plug-in inside 475 mA (REQ-PWR-12). Accept.
- **DEC-09.** The derating rule's two exceptions: GPIO47 (with DEC-07) and the panel's VCI and
  VDDI at up to 3.366 V against 3.3 V (DF-96). Accept both.
- **DEC-12.** The backlight halves at the minimum duty: each sink within +-25% at 18%
  (+-20.2% today). Accept.
- **DEC-13. The redraw time** (back with the ILI9488; DF-180, REQ-DSP-11): a full 18-bit frame
  is 92 ms at 40 MHz. Proposed: REQ-DSP-11 at 120 ms full-screen and 50 ms for a text update,
  measured on the breakout before the order (VER-43); the battle screens redraw in parts.
- **DEC-14.** Flash budget 1.6 MB; off current 60 uA at 45 C; lot capacity 600 mAh; the
  BQ24073's 5 h timer as the charge timeout (REQ-BAT-13).
- **DEC-15.** USB input low end: run from 4.40 V with a cell, 4.75 V without. Decide after
  VER-80.
- **DEC-16.** prod_board: which product requirements apply to your board (the .P allocations).

**Before the case release**

- **DEC-18. Screws** (DEF-31, VER-147): REQ-MEC-09 is now per pilot (round 2, F). Proposed: M2
  x 7 in the four side pilots (4.2 mm of thread, the tip 0.8 mm short), M2 x 6 in the two end
  pilots (3.2 mm of their 3.30); case.scad gets screw_l_side and screw_l_end, bom.md two
  lines. Or accept 3.2 mm at the sides for EVT and count stripped bosses in VER-92.
- **DEC-25. TH1's coupling to the cell** (DF-04): a 0.5 mm silicone thermal pad bridging the
  hole's edge to the cell's end, or accept the lag for EVT and measure it (VER-87).
- **DEC-26. Opening the case** (DF-169): change assembly.md so J2 is unplugged before the tub
  lifts away.

**The cell**

- **DEC-23. A cell sold with a datasheet**: a 403450 whose seller publishes its size with the
  PCM (DEC-27 needs 51.2 x 35.2 mm or less), the PCM's thresholds and a UN38.3 summary.

**Risk acceptances, waivers and signing**

- **DEC-19.** The H rows that only built units can detect, or that wait for a decision, need
  Alex's acceptance for ORDER (R3); `check_trace.py --gate order --verbose` lists them. Take
  each row's pre-order action first. DF-87 (the printed case is not a fire enclosure):
  proposed through DVT, as hobby units kept by their builder. DF-170 and DF-172: for EVT, with
  DEC-33 and DEC-22. DF-180: decide with DEC-13.
- **DEC-20.** W-01 and W-02 are no longer needed: G-ORD-06 names VER-33 and G-ORD-13 is
  RETIRED (round 2, O).
- **DEC-21.** DVT: reverse-polarity protection or a keyed connector on CELL_P (DF-55).
- **DEC-29. u4.json pin 58's waiver** (GPIO47 above IOVDD when off) was written by the design
  session, not by Alex. Once VER-36 is built it fails until an A-NNN line of his cites it.
  Decide with DEC-07.
- **DEC-30. A pinned signing key** (round 2, O): pin a key Claude cannot use (README,
  'Approvals'). Until then no approval counts.

Retired decisions: DEC-06 (3V3 through the unplug: no gap with the BQ24073), DEC-10 (folded
into DEC-07). DEC-13 came back with the ILI9488.

## (g) Round 2's sign-off objections and where each landed

Both reviewers of the 684a609 register declined sign-off with these 18. All 18 are answered
here; where a fix is partial or the objection did not hold, the row says why.

| # | Objection | What was done | State |
|---|---|---|---|
| O1 major | the cell's gap understated: it fails on every side, and widening the right end would leave VER-181 failing | DEF-53, DF-85, REQ-BAT-08, VER-181, G-ORD-07 and DEC-27 restated on every side; cellgap prints what passes; DEC-27's options re-costed (a cell spec and the pocket, or a 54.8 x 37.8 mm hole); seeded fault: only the right end widened keeps VER-181 failing | fixed |
| O2 major | 'G' only proves some key in a user-global file; README told Alex to sign with an unprotected key | check_trace counts a line only when `%GF` equals the fingerprint README pins and the pin line is itself signed by that key; it warns while the pinned key is a software key; README asks for an ed25519-sk key or a passphrase key never in an agent; seeded fault: a line signed by another allowed key does not count | fixed as far as a file can: a process running as alex that holds an unlocked key can still sign; only a touch key closes that, and Alex chooses it (DEC-30) |
| O3 major | five ORDER checks (VER-13, 21, 23, 25, 108) in no step; prod_board's list missing | section (d) names every ORDER check (VER-23 and VER-108 in step 4; VER-13 moved to EVT; VER-21 advisory; VER-25 folded into VER-132) and prod_board's list; check_trace fails on an ORDER check no step names; seeded fault | fixed |
| O4 minor | APPROVALS.md's header described the old rule | header rewritten: signing with the pinned key is required; unsigned, other-key and merged lines are pending or errors | fixed |
| O5 minor | REQ-TST-06 named CHG_DONE_N; make it VER-161's first seeded case | REQ-TST-06 says CHG_STAT_N | fixed; the seeded case waits for VER-161, which is not built |
| O6 minor | PLAN statements disagreeing with the tool: step 0.1, section (c)'s source, the 129 count, the template's marker spot | step 0.1 says the default mode warns and R8 fails; (c) names its evidence dates; counts come from the tool (section i); the template's marker goes in the clear patch silk.py names, and DEF-57 moves the BOARD_REV text out of the cell's hole first | fixed; the marker's coordinates wait for DEC-28 |
| O7 minor | per-unit screens a good unit can fail: TS at 25 C only; the 130 ohm window | VER-133, VER-135 and VER-68 take the band from `design_numbers.py tsband` at the measured ambient (0.70-1.04 V at 22 C); VER-134 uses 133 ohm from `cvload`'s window (128-141 ohm; 130 sat 0.6 ohm from its edge) | fixed; I_TS, pre-charge and termination are still quoted from memory until VER-34 |
| O8 minor | DF-170 offered only a procedure | DEC-33 offers the copper (J4 a JST SH 2-pin, now or at DVT); VER-184 (FAIL) shows the cause; G-DVT-06 makes the DVT spin fix it | fixed; Alex chooses when |
| O9 minor | G-ORD-13 and G-ORD-06 needed signatures that prove nothing | G-ORD-13 RETIRED in 2d1124d (check_trace now knows a RETIRED criterion); G-ORD-06 names VER-33; W-01 and W-02 kept as IDs, not needed | fixed |
| O10 minor | decision IDs collide with diodes | D1-D30 renamed DEC-01 to DEC-30 in every file | fixed |
| O11 minor | VER-153 at ORDER; 127 signatures for a 5-board order | VER-153 at EVT; bulk lines `REQ-ALL@<commit>` and `DF-H@<commit>`, each ID still checked on its own (seeded fault: an edited statement is not covered); the order gate trimmed (section d) | fixed |
| O12 minor | bring-up docs not gated before first power | VER-39 in G-EVT-02 and stated as an EVT day-one precondition (e); bringup.md's display section is the 2.4 in panel's, logged as DEF-21 (n) | fixed in the register; bringup.md itself is outside hardware/vv |
| F1 major | as O1 | as O1 | fixed |
| F2 minor | REQ-MCU-06: GPIO28 is PWM2 A, GPIO31 PWM3 B | not changed: the RP2350's table maps GPIO 0-31 to slices (n >> 1) & 7 and 32-47 to 8 + ((n >> 1) & 3), so GPIO28 is slice 6 A and GPIO31 slice 7 B (`grep -n "PIN_28\|PIN_31" ~/.cargo/registry/src/*/embassy-rp-0.8.0/src/pwm.rs`: 'PWM_SLICE6, ChannelAPin', 'PWM_SLICE7, ChannelBPin'). VER-15 now derives each slice from the table, as asked | objection did not hold; its request applied |
| F3 minor | stale names: REQ-TST-06, vp 212's C41, placement.py and design.py comments | REQ-TST-06 and vp 212 fixed; the comments (outside hardware/vv) logged in DEF-21 (p); VER-39 now covers placement.py | fixed in the register |
| F4 minor | gate logic: a check re-phased leaves its requirement; no DFMEA:COVERAGE fault; DF-95 and DF-96 rated 2 on one check | a requirement must have a live check no later than its Phase; DFMEA:COVERAGE seeded (R64 unnamed); D of 2 or 1 needs two checks, DF-95 and DF-96 now D4 and DF-95's prevention says the condition is by design | fixed, three seeded faults |
| F5 minor | REQ-MEC-09 unmeetable for the end screws | split per pilot: 4.0 mm in the side pilots, 3.0 mm in the end pilots; M2 x 7 at the sides passes | fixed |
| F6 minor | VER-13, 21, 27, 34 reached only by PHASE:ORDER; effort | VER-13 to EVT (G-EVT-04); VER-21 and VER-27 advisory; VER-34 in G-ORD-01; the gate trimmed | fixed |

## (h) What this plan still cannot catch

- **Small numbers.** Five boards, then 30, cannot show a failure rate under a few percent.
- **Time.** Cell ageing and swelling over years, ENIG wear, tape creep, slow flux migration.
- **Faults nobody listed.** The DFMEA names every part, but only the failure modes someone
  thought of; seeded faults prove a checker fails on the faults seeded.
- **Numbers this register quotes from memory.** design_numbers.py names them (the RP2350's
  leakage, I_TS, pre-charge and termination shares, V_LOWV, the ILI9488's SPI cycles, JLC's
  0.30 mm and 0.2 mm tolerances); VER-34 and VER-17 must replace each with a quoted page.
- **The panel's mechanics.** Its datasheet could not be downloaded on 2026-10-26, so its
  outline, thickness and FPC shape are the old design's numbers; only a real panel (VER-43,
  VER-136, VER-152) can show J3's place is right.
- **A hand edit in the board file.** 62b01f2 moved one GND via by hand; any rebuild (a silk.py
  or rule change, DEC-28, DEC-32) undoes it. The release run must rebuild, re-run DRC and
  record the result, not trust the committed file.
- **A key that a process can use.** A pinned software key is only as safe as its passphrase
  habit (DEC-30).
- **Bench honesty, standards, JLC's side, firmware beyond the self-test, use**: as the first
  run listed them.
- **A design that moves again.** Another session may commit to ref-design-35in. Any change to a
  design path fails the default mode and every gate until its rows are re-checked; the
  register cannot re-check them itself.
- **Quoted values until VER-161 exists.** A GPIO, a count or a dimension quoted in a row can
  still drift from design.py unseen (DEF-54).

## (i) The gate today

At 0c1571a (2026-10-26), whose evidence was made at 807d0e0 (the scripted checks) and 1bc504b (VER-129):
requirements 133 (130 PROPOSED, 3 RETIRED); DFMEA rows 181 (148 live: 54 H, 68 M, 26 L; 33
RETIRED); checks 183 (14 PASS, 11 FAIL, 51 NOT_RUN, 99 NOT_BUILT, 5 CUT, 3 RETIRED); defects
57 (24 OPEN, 20 FIXED, 6 CLOSED, 7 RETIRED); seeded faults 71 of 71 caught (VER-129: 45 on a
copy of the register, 26 in a scratch clone with signed commits, three of them design_numbers.py
runs).

```
$ python3 hardware/vv/tools/check_trace.py
STRUCTURE: OK (5 warnings)
  approvals by Alex: 0 counted (signed), 0 pending; commit signing not set up, so none can
  count yet; README.md pins no signing key
  order branch: ref-design-35in has design commits after be437f8: 2ddb7fe ref_design case:
  button labels engraved 0.3 deep on the top's inside
  design baseline: matches HEAD
  warnings: the order branch's case commit; the pass bands of VER-67, VER-88, VER-134 and
  VER-139 are 1.4-2.2 x their DMM's accuracy (want 4 x)
exit 0

$ python3 hardware/vv/tools/check_trace.py --gate order
  PASS  G-ORD-24 (VER-129), R5, R6 (no release yet), R7, R8 (board files)
  INFO  G-ORD-05, G-ORD-08, G-ORD-12, G-ORD-20, G-ORD-26 (advisory, four of them moved);
        RETIRED G-ORD-13; N/A G-ORD-25 (prod_board)
  FAIL  G-ORD-01 to G-ORD-04, G-ORD-06, G-ORD-07, G-ORD-09 to G-ORD-11, G-ORD-14 to G-ORD-19,
        G-ORD-21, G-ORD-23 (the FAILs among them: VER-18 edges, VER-131 drill, VER-176 CPL,
        VER-181 the cell's gap, VER-137 backlight, VER-138 cutoff, VER-177 VBUS)
  FAIL  G-ORD-22: 45 ORDER checks not passing (it was 67)
  FAIL  R1 127 PROPOSED; R2 42 requirements with an ORDER check not passing;
        R3 53 H rows neither accepted nor DONE; R4 18 OPEN defects with Gate ORDER
RESULT: FAIL (22 of 27 not met); 1 N/A, 5 advisory, 1 retired
```

The case gate fails 11 of 14 (R8 among them: 2ddb7fe's engraved labels are a case.scad change
after be437f8, to re-check with VER-113 and VER-07 before the case release), EVT 19 of 22, DVT
10 of 13, PVT 10 of 13, and prod_board's order gate 5 of 7 (24 criteria N/A). The checks that
pass: the netlist, the pin gate, ERC, DRC with parity, both 4-ring contact footprints, inrush,
holes, the MLCC keep-out, TH1's geometry, swelling room (new), two inspections (VER-104,
VER-124) and the seeded faults (VER-129).

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
| DEF-32, 33 | the drill file; the CPL offsets | VER-131, VER-176 (both FAIL) | before the order |
| DEF-34 | no firmware for the ordered chip | VER-149, VER-150 (EVT), VER-151 | EVT, before the order |
| DEF-35, 52 | the trace gate's own holes | VER-129 | every change |
| DEF-36 | check_pcb passes non-silk warnings, and called the routed-away BOARD_REV cosmetic | VER-04, VER-22, VER-18 | before the order |
| DEF-37, 48 | TH1's coupling; the 47 C hot edge | VER-164, VER-87, VER-32 | before the order and EVT |
| DEF-39 | backlight over 120 mA: closed with the 2.4 in panel, back with the 3.5 in | VER-137 (FAIL) | before the order |
| DEF-40 | the cutoff may never fire | VER-138 (FAIL), VER-139, VER-155, VER-156 | before the order |
| DEF-41, 42 | assembly next to a live cell; Start/Select parts not bought | VER-91, VER-167 (FAIL) | case gate |
| DEF-43, 51 | inrush; swelling room | VER-99, VER-182 (both CLOSED) | every change |
| DEF-45 | the backlight sim does not read design.py | VER-05 | EVT |
| DEF-46 | bench cell emulation | VER-70, VER-67 | EVT |
| DEF-49, 50 | the pill margin; the VBUS fault reach | VER-45, VER-177 (FAIL) | before the order |
| DEF-53 | a worst-case cell 0.25 mm from the FR4 edge on every side | VER-181 (FAIL) | before the order |
| DEF-54 | rows still written for an older design after a re-seed | VER-161 (not built), the retired-citation rule | case gate |
| DEF-55, 57 | JLC's marker not on the board; the board's name and BOARD_REV in the cell's hole | VER-132 (k), VER-18 (FAIL) | before the order |
| DEF-56 | copper 0.20 mm from the routed holes | VER-18 (FAIL) | before the order |

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
| O-18 | named in DEC-01 as needing J6 or the strap (R48-R51), which left in 86bf694 | PLAN (f) | moot at 684a609: RTT/USB CDC logs (REQ-MCU-04) |
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
| O-62 | the self-test's UART needs J6 (DEC-01) | PLAN (f) | moot: J6 left in 86bf694; logs over RTT or USB CDC |
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
| F-33 | defer play time, charge time and charge current | REQ-PRD-03.R, REQ-PRD-04.R, REQ-BAT-02.R, PLAN (f) | PLAN decision DEC-04; DEFERRED now needs a scope |
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
| F-46 | R18 = 56k for GPIO47 | DF-95, REQ-PWR-14, PLAN (f) | PLAN decision DEC-07 (with the cutoff) |
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
| O-R4 (revised) | approvals counted from any commit without a Claude trailer | a line counts only from a commit git shows as signed (%G? = G), never a merge; pending where signing is not set up; README 'Approvals'; DEC-30; the clone signs with a throwaway key; two seeded faults |
| O-R5 | R4 at ORDER blocked by case and docs defects | every defect has a Gate; DEF-21, 31, 37, 42, 51 are CASE; two seeded faults |
| O-R6 | DEC-05's footprints cannot give 0-45 C | DEC-05 rewritten (a higher-B TH1 or accept 47.2 C); REQ-BAT-03 and VER-32 one-sided; DF-147, DEF-48 |
| O-R7 | the pushed cell's end 0.25 mm from the FR4 edge | design_numbers.py cellgap, VER-181 (FAIL); DF-85 H; DEF-53; DEC-27 |
| O-R8 | occurrence 3 on parts never built | rows on new parts re-rated 4-8; O = 3 needs 'Similar:' (check_trace rule, seeded); 17 rows newly H |
| O-R9 | checks still written for 76955d2 | VER-29, 33, 36, 77, 89, 130, 132, 148, 159, 166 rewritten; counts and outline derived, not typed; a live row citing a RETIRED id fails; DEF-54 |
| O-R10 | latch checks covered only seat 1's Start | VER-33 (both paths, D8, Q7, R62, R63, SW23 in covers), VER-78 and VER-79 on SW21 and SW23 through 1 kohm |
| O-R11 | DVT and PVT counted EVT inspections and demos | the release-of-phase rule applies to every check that covers a release; seeded (an EVT FAI at PVT) |
| O-R12 | prod_board's gate evaluated no DFMEA row | df_scope includes 'both'; ref_design rows relabelled; DF-174 to DF-177 for prod_board's contacts; VER-183; G-EVT-15 |
| O-R13 (revised) | the design-hash rule saw only this branch | README's parsed 'Order branch' line; default mode warns and R8 fails the order gate on a design commit after 684a609; battler.kicad_pro in the baseline; VER-143 refuses a drifted tree; seeded |
| O-R14 | VER-164 saw 2 of 11 back-side footprints | th1 reads every flipped footprint and TH1 from the board (pcbnew); seeded (TP11 moved next to the hole fails) |
| O-R15 | the cutoff analysis left out GPIO47's leakage | cutoff prints the leakage band (0.47 V per uA through 235 kohm); DEC-07 waits for VER-34's number |
| O-R16 | DF-159's detection cannot see one LED | VER-43 measures each LED by its own cathode pin; design_numbers backlight says so; DF-159 detection reworded |
| O-R17 (revised) | latch and TS numbers off the circuit | REQ-PWR-11 cites VER-33's computed bands; REQ-THM-03 and VER-68 0.49 V; VER-52 TP1 8.5-10 kohm from the full network |
| O-R18 | EN1 at its absolute maximum; OVP nominal | REQ-PWR-02, VER-12, VER-71: EN1 <= 6.7 V, VBUS under OVP's minimum less 0.2 V (VER-34 takes the minimum into u2.json) |
| O-R19 | the JLC marker is not on the board | REQ-FAB-05 note; VER-132 (k); DEF-55; DEC-28 |
| O-R20 (revised) | the pin gate accepts anyone's waiver, never checks identity | VER-36: waivers cite an A-NNN line, lcsc and mpn compared with design.py, reviews of removed parts fail, acceptance list generated; DEC-29 |
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
| F-R12 (revised) | charge-time source current | REQ-PRD-04 and VER-102 at 0.5 A; the 3.0 vs 4.0 h choice is Alex's (DEC-04) |
| F-R13 (revised) | R6 and the baseline left out schematics and scripts | the schematics, battler.kicad_pro, the board/tools tree and case/export.py in DESIGN_PATHS and the baseline; positions.json's sha256 in the release record |
| F-R14 (revised) | VER-148 and DF-144 leaned on retired or wrong examples | VER-148 names DF-146 and DF-148; DF-144's variant list corrected; VER-109 reads U2's marking; the retired-citation rule (seeded) |
| F-R15 | VER-52's TP1 band ignored the second path | as O-R17 |
| F-R16 (revised) | smaller stale statements | REQ-PRD-09 cites VER-117; VER-29 cites VER-176's evidence; vp_map row 1 37 parts, 322 pins; bom.md:16 and :55 in DEF-21 and VER-39 |

Not done here, by design: the new check scripts (check_claims.py, check_bom_cpl.py,
check_fab_content.py, latch.py, temp_window.py, release.py, the extended check_pins.py) are
work items in section (d); design changes (the drill flag, the marker, the hole) wait for
Alex's decisions; files outside hardware/vv (pin_review, docs) are logged under DEF-21.

## (m) The first run's sign-off objections and where each landed (history)

| Objection (O: Opus, F: Fable) | What was done |
|---|---|
| O blocker: the register described a superseded design | Re-seeded at 684a609 (this commit range); the baseline names every design path and drift fails the default mode; DEF-47 records why nothing caught it; a realistic estimate (about a day for the re-seed) in (d) |
| O major: R6 and R7 missed the copper and the fab scripts | DESIGN_PATHS and the baseline cover battler.kicad_pcb, last_route.ses, build_pcb.py, route.py, silk.py, fab.py and board/lib; seeded fault: a silk.py change after a release fails R6 |
| O major: DVT and PVT accepted EVT bench results | ver_ok at DVT and PVT needs a test of phase EVT or later to cover a release of that phase; gates.md's text matches; seeded fault |
| O major: DEFERRED never expired | DEFERRED needs a scope in APPROVALS.md and ends after it (R1 says so); seeded faults for a missing scope and an ended deferral |
| O major: the order gate broke O-9 | VER-40 is advisory-only (named only by G-ORD-12's ADVISORY), so PHASE:ORDER skips it; the doc sweep (VER-39), the claims script (VER-161) and the BOM coverage (VER-167) moved to G-CASE-06, before assembly; DEF-41 is FIXED and held by R4 at the case gate. Not moved: VER-115 and VER-158 (test access) stay at ORDER because a missing probe pad is a copper change |
| O major: no decision on a fuse in the cell path | DEC-22 with a proposal; DF-172 (H) |
| O major: 'nothing dropped' could not be verified | The ID ledger in (k): all 115 IDs, where each landed, and 30 that cannot be traced (19 never named, 11 named only as a merge) |
| O minor: VER-99 presented as done | At 684a609 there is no deviation to present (section f, 'REQ-PWR-13 and VER-99'): VER-99 now checks the limit itself and passes; DEF-43 CLOSED with that changed check; REQ-PWR-13 still awaits Alex |
| O minor: D10 needs D7 (now DEC-10, DEC-07) | DEC-07 and DEC-10 decided together, with the ADC numbers |
| O minor: the cell source is weak | DEC-23 |
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
