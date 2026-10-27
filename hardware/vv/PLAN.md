# The plan: from here to 5 boards, then 30

**Not ready to order.** The board being ordered is `hardware/ref_design` at **be437f8**
(branch ref-design-35in, 2026-10-26; the board itself is final at 62b01f2, the later commits
are docs, print kits, 2ddb7fe's engraved case labels and 7d35933's route.py change). On
2026-10-26 this register, which described 684a609, was re-seeded for it, the board-level
checks that found real problems were re-run, round 2's 18 sign-off objections were answered
(section g) and the order gate was trimmed to what the copper, the BOM and the fab files
freeze (section d). The same day a third review's 23 findings (O-R1 to O-R12, F-R1 to F-R11)
were applied and checked (section n). The default mode passes; the order gate fails, as it
should (section i).

Contents: (a) the system on one page; (b) what changed in the design; (c) the board at
be437f8: what the checks find; (d) before the 5-board order, and what moved where; (e) the
case gate, EVT, DVT and PVT; (f) decisions for Alex; (g) round 2's sign-off objections and
where each landed; (h) what this plan still cannot catch; (i) the gate today; (j) past bugs and
what catches them; (k) the ID ledger of the first run's debate; (l) the second review's ID
ledger; (m) the first run's sign-off objections; (n) round 3's 23 findings and where each
landed.

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
  it advisory fails the gate (G1) until a newer signed line (O-R5). At the order gate the
  pinned key must be a hardware (sk-) key unless the gate is run with --accept-software-key,
  which prints and goes into the release record (K1, O-R12).
- RETIRED: a requirement, risk, check, defect or gate criterion whose subject left the design
  keeps its ID and names the commit that removed it; no live row may lean on a retired check
  or cite a retired id outside 'Was: ...' (O-R9, F-R14).
- Each defect names its Gate; R4 holds it there and at every later gate, not before (O-R5).
- The design baseline in README.md names every design path and must equal HEAD and the
  baseline commit's own blobs, or the default mode fails (DEF-47: on 2026-10-26 the rebase onto
  be437f8 failed it at once, as designed; F-R3: a hash rewritten from HEAD fails too). README
  names the order branch: a design commit on it after be437f8 warns in the default mode and
  fails the order gate (R8, O-R13). A case-only commit passes R8 at the order gate only when
  positions.json, regenerated by case/export.py at the branch's tip, hashes the same as at the
  baseline; both hashes print (O-R4).
- A CPL row off its pads passes only through a JLC_FIXED entry whose evidence file is committed
  and names the designator (F-R4).

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
| The cell's gap, every side (VER-181) | the tub's pocket is still 0.25 mm inside the 54.0 x 37.0 mm hole on every side, so a worst-case 52 x 36 cell is 0.25 mm from the routed edge at the pushed end and on both long sides (-0.15 mm after 0.40 mm of routing, print and screw tolerances), and 1.75 mm at its lead end (1.35 after, against 1.5); a nominal 50 x 34 cell fails the same three faces at -0.15 mm and passes its lead end (3.35 mm). The lead end of an **unpushed** cell has only the pocket's fit (0.25 mm today, -0.15 mm after the tolerances; 0.5 mm with DEC-27(a)), because assembly.md step 5 holds the cell pushed with tape and nothing in the design does (O-R8). What passes: the pocket 0.9 mm inside the hole at the right end and both long sides and 1.9 mm inside it at the lead end (that wall is the stop DEF-58 asks for, so no tape is needed) with a cell of at most 51.2 x 35.2 mm, or a hole of at least 54.8 x 37.8 mm | still failing (DEF-53, DEF-58, DEC-27) |
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
| The 18-bit redraw | the ILI9488 takes only 18-bit colour over SPI: 461 kB a frame, 184 ms at 20 MHz (twc 50 ns) and 197 ms at 18.75 MHz, the fastest in-spec clock the board can make from its 150 MHz clk_peri (SSPCPSR); reads cap at 6.7 MHz (trc 150 ns). The 40 MHz figure this section quoted was out of spec and is gone (O-R1) | DF-180 (H), REQ-DSP-11, DEC-13, O-R1 |
| **The charge window's hot edge** | TH1 (B3950 10k 1%) against U2's TS thresholds: charging runs from 3.5 C to 47.2 C at nominal values, 1.8 C to 51.0 C at SLUS810N's limits (the hot trip is 3.46 kohm: V_HOT 0.270 V over I_TS 78 uA) and 1.4 C to 51.5 C at the worst corner with TH1's own 1%. A 45 C hot edge needs B 5130 or more (cold edge 6.5 C); B4250 gives 49.5 C and B4350 48.9 C. `design_numbers.py tempwin` is the check (O-R3) | VER-32 FAIL (binds at EVT), DEF-48, DF-147, DEC-05 |
| The CV load's window is empty | SLUS810N's KPRECHG 70-106 A-ohm (pre-charge 21.0-32.4 mA with R5) and ITERM 0.09-0.11 x ICHG (termination up to 32.8 mA) leave no resistive window: a resistor must exceed 147.6 ohm to leave pre-charge and stay under 126.7 ohm never to terminate. The bench load is 160 ohm in parallel with a TL431 set to 3.5 V and 22 ohm: 3.31 V or more in pre-charge, 52 mA or more at 4.16 V, at every corner `design_numbers.py cvload` checks. A 3.3 V zener (too soft) or three diodes (about 2 V) would not do it (O-R2) | VER-134, VER-145 |
| The breakout's header numbering | the product's panel-side pairing is 51-j and the fold changes no contact pairing; what is not known is the bought breakout's own header numbering, so VER-43 finds it by diode and continuity tests, checks it equals 51 minus the FH12 pad number, and wires the dev board by the panel's own pin names (F-R1) | VER-43, VER-136, DF-57 |
| The panel's mechanics are estimates | its outline, active area, thickness and FPC shape in case.scad are the old 3.5 in design's. The datasheet is on disk in the main checkout (hardware/ref_design/datasheets/er-tft035ips-6.pdf and .txt, REV 1.0 of Dec-15-2022, its drawing drawn Dec-20-22; untracked, their sha256 in evidence/2026-10-26_datasheets.txt): 54.56 x 82.84 +-0.2 mm, a thickness of 2.1 (table) or 2.15 +-0.15 mm (side view) and a 4.47 mm neck, while case.scad uses 54.50 x 83.00 x 2.45 mm and 4.59 mm. The fold makes the outline an estimate either way, and these numbers set J3's place (O-R6, F-R7) | DF-61, DF-123, DEF-59, VER-136, VER-152 |
| A hand edit in the board file | one GND via moved 0.08 mm by hand at 62b01f2; the next build_pcb run undoes it, and every step-1 change rebuilds | (h), DEF-60 (Gate ORDER); VER-143 now rebuilds, re-runs the DRC and diffs the copper; 7d35933 may fix the cause |
| The order branch moved | after be437f8: 2ddb7fe engraves button labels on the top's inside (case.scad only; positions.json regenerated there hashes the same as at be437f8, 179f81ed..., so it moves no board position: O-R4); 7d35933 changes board/tools/route.py (a pre-flight DRC, and core vias that fail clearance are dropped: it may be the generator fix DEF-60 asks for, unverified); the rest are print-kit and docs commits | R8 fails at the order gate on 7d35933 until VER-01 to VER-04 are re-run on a board route.py rebuilds and the baseline moves (DEC-01) |
| Swelling room | 0.80 mm under the panel's back (0.30 under a deck at 684a609): passes | VER-182 PASS, DEF-51 CLOSED |
| Screws | M2 x 6 gives 3.2 mm of thread in the side pilots (want 4.0; M2 x 7 passes, its tip 0.8 mm short) and 3.2 mm of the end pilots' 3.30 (ok) | VER-147 FAIL, DEF-31, DEC-18 |
| Two pin-review findings that set the BOM | the panel's VCI and VDDI at up to 3.366 V against the module's 3.3 V (absolute 4.6 V; the ILI9488 itself runs to 3.6 V: er-tft035ips-6.txt:734, :747-748); the second 4.7 uF on +1V1 | DEF-61 (split from DEF-26), VER-34, VER-23, DEC-09 |
| Unchanged | cutoff 3.521 V at the worst corner (VER-138 FAIL, now binding at EVT); VBUS fault reach 6.9 V (VER-177 FAIL, now binding at EVT); bom.md's domes and Start/Select caps (VER-167 FAIL); TH1 0.80 mm from the hole's edge, no back pad within 1 mm (nearest TP3, 5.5 mm: VER-164 PASS); nearest MLCC C53, 11.40 mm from H5 (VER-142 PASS); every hole 0.30 mm or more (VER-114 PASS); VBUS 10 uF (VER-99 PASS) | as named |

## (d) Before the 5-board order

Round 2's two reviewers called the order gate (67 checks, 10-12 days) too heavy for a 5-board
test order, and round 3 trimmed it again. It now holds only what the copper, the BOM and the
fab files freeze, or what would make the five boards useless or unsafe. A BOM decision that
Alex may accept for EVT is held here by its defect (DEF-39, 48, 49, 50: FIXED by a value in
design.py, or moved to EVT by his acceptance), while its check binds at EVT, so an acceptance
does not leave a failing ORDER check behind. Every ORDER check is named in a numbered step
below (check_trace fails otherwise). Effort is Claude's unless it says Alex.

| # | Step | IDs | Effort |
|---|---|---|---|
| 0.1 | **Alex confirms be437f8** as the order candidate (DEC-01). A later commit to ref-design-35in that touches a design path makes the default mode warn and the order gate fail (R8) until the register is re-checked and the baseline moved: 7d35933 (route.py) does so today. Clean the main checkout's uncommitted battler.kicad_pro before any fab run (O-34). | DEF-30 | Alex: 5 min |
| 0.2 | **Order everything with a lead time** (`--shopping --gate evt`): RP2354B and L2 pre-order at JLC; cells from a seller with a datasheet (DEC-23); the ER-TFT035IPS-6, an FH12 breakout, an RP2350B dev board; two GSPK151035PN and a 53261-0271; domes and Start/Select strips; calipers, a 4.096 V reference, clip leads, a LiPo bag, a USB-C breakout, a DMM thermocouple; the CV load (160 ohm and 22 ohm 0.25 W 1%, a TL431 in TO-92, 10k and 24.9k 1%: O-R2). The datasheets are on disk in the main checkout (F-R7: not in pin_review/ds/, which does not exist, but in hardware/ref_design/datasheets); `tools/datasheet_hashes.py` records their sha256 and sizes, and the extracts stay out of git (O-R6). | VER-41, VER-37, VER-43, VER-34, VER-152 | Alex: 1 h, about $100-150 |
| 0.3 | **Run the register on every commit**: one line in scripts/check.sh, `python3 hardware/vv/tools/check_trace.py` (outside hardware/vv, so it waits for Alex's OK). It would have caught DEF-39's return in 79de1f7. | DEF-47 | 15 min |
| 0.4 | **Alex pins a signing key** (README, 'Approvals': a hardware key or a passphrase key never in an agent). Until then no APPROVALS.md line counts (DEC-30). | | Alex: 15 min |
| 0.5 | **prod_board's own order list** (`--gate order --board prod_board`, Alex's board, evaluated apart): its netlist, pin gate, ERC, holes and probe table. | VER-10, VER-119, VER-120, VER-121, VER-122, VER-163 | Alex: 1 h |
| 1 | **Copper, silk and BOM decisions**, then one design pass in ref_design (outside this register) and a re-route where copper moves: DEC-28 silk.py (the board's name, BOARD_REV and labels out of the holes, the JLC marker), DEC-32 the pours' edge clearance to 0.3 mm and J2 0.1 mm in, DEC-27 the cell's gap (a cell spec and the pocket, with the pocket's lead-end wall as the stop, or the hole), DEC-31 the backlight's full scale, DEC-05 TH1's B (B 5130 or more, after a JLC stock check), DEC-17 the VBUS reach, DEC-24 the pull-ups, DEC-09 the panel's VCI and VDDI (DEF-61), DEC-22 the fuse and DEC-33 J4 (copper now, or accepted for EVT and done at DVT); DEC-13 the display bus (SPI at 220 ms a frame, or the 8080 bus on J3's pins 19-26 with a local re-route and PIO firmware: a copper decision, not a firmware tweak); and DEF-60's generator fix (7d35933 may be it), so no hand edit survives the next rebuild. Each BOM value either goes into design.py (its defect FIXED) or Alex accepts it for EVT (its defect's Gate moves). | VER-18, VER-181, VER-137 | Alex: 1 h; the design pass half a day, a re-route about 2 h if J2, J4, J3's bus or a fuse move |
| 2 | **Re-run the scripted checks** after step 1 (`run_checks.py`, about 10 min, then VER-129 on its own): netlist, pin gate, ERC, DRC with every warning type and count read by hand (DEF-36), contact footprints, inrush, holes, MLCC, TH1, CPL, drill, VBUS, cutoff, backlight, charge window, cell gap, edges. | VER-01, VER-02, VER-03, VER-04, VER-08, VER-99, VER-114, VER-142, VER-164, VER-176, VER-131, VER-137, VER-181, VER-18, VER-129 | 30 min |
| 3 | **Fab outputs** (VER-131 and VER-176 read fab.py's real output and fail today): separate drill files; no CPL row moves before the dry run's placement preview, which is the reference for J1-J4's origin offsets (O-R7), and fab.py's per-ref correction table is filled only from JLC_FIXED entries that name the committed preview evidence (F-R4); the fab content check over the exported files (the combs' paste, mask and keep-outs, the plated slots, the vents, the JLC marker, every silk text on the board); JLC's dated limits; BOM, CPL and PCB fields against design.py; land patterns; contacts as placed; the dome footprint against its drawing; the release run that rebuilds the board, re-runs the DRC, diffs the copper and makes the files twice (VER-143). The order sends no netlist; VER-26 (the merged netlist) and VER-29 (the rotation script) are advisory here and bind at G-DVT-01. | VER-131, VER-176, VER-132, VER-17, VER-130, VER-125, VER-162, VER-178, VER-143 | 1.5 days |
| 4 | **Layout reviews**: the copied core laid over Raspberry Pi's RP2350B minimal design by hand, answering the second 4.7 uF on +1V1 in writing (the script is VER-187, DVT); the USB pair's stubs; the GPIO mux and PWM slices, its evidence printing the whole slice table (F-R11). | VER-23, VER-108, VER-15 | half a day |
| 5 | **Power-path analyses that can move copper or the BOM**: the latch table over both Starts (it decides whether any unit turns on); the backlight; inrush. Alex reads by hand the four back-power paths DF-98 names (VBAT_SENSE, SWD, the panel's SPI with JP4 cut, D+); the script is VER-159, at EVT. The charge window (VER-32), the cutoff (VER-138), the mating peak (VER-140) and the VBUS reach (VER-177) are built or scripted and bind at EVT (G-EVT-04); their decisions (DEC-05, DEC-07, DEC-17) are made in step 1. | VER-33, VER-137, VER-99 | half a day |
| 6 | **Datasheets and part identity**: the application circuits and every constant this register quotes, from the files on disk (the ILI9488's twc 50 ns and trc 150 ns and its IM=111 read pin, SLUS810N's limits, the RP2350's IIN; O-R6), with the panel's VCI and VDDI answered in writing (DEF-61); every line of JLC's matched-parts page against design.py, by hand (the script is VER-186, PVT); the watchdog rule and FM8002A polarity (both pass). The pin gate's tooling (VER-36, DEF-26) moved to EVT. | VER-34, VER-14, VER-104, VER-124 | 1 day |
| 7 | **The board against the case**: the XY chain from positions.json (the comb's 1.60 mm spot is copper, and the chain needs no parts: O-R9); placement drift; the MLCC keep-out and TH1 (both pass); the ESTIMATE ledger, the panel's numbers first, read off the drawing on disk. The heights, the pill print on the fit kit and the connector mouths are the case gate's (VER-112, VER-51, VER-165 at G-CASE-02: F-R9). | VER-174, VER-06, VER-19, VER-142, VER-164, VER-152 | 1 day plus calipers |
| 8 | **The panel and the cell, parts in hand**: the ER-TFT035IPS-6 lit through the breakout, the breakout's header numbering found by diode and continuity tests and the dev board wired by the panel's own pin names (F-R1); ID4 read on SDO, or on SDA if SDO is silent (F-R8); its redraw timed at an in-spec clock (18.75 MHz from 150 MHz, or 20 MHz from a 120 MHz clk_peri: O-R1); the FPC folded into a breakout in J3's place (assembly.md section 5 step 6); the tail's thickness; the cell's spec sheet and cell.json. The cell's incoming inspection (VER-49) is EVT's, before the first cell is plugged (G-EVT-02, F-R9). | VER-43, VER-136, VER-44, VER-37, VER-160 | Alex and Claude: an evening each once parts arrive |
| 9 | **Test access and firmware size**: every bring-up probe point has copper (the probes table and its script); the game's llvm-size inside 2 MB (the flash map and the check.sh step are VER-185, EVT). | VER-158, VER-151 | 2 hours |
| 10 | **Reviews** (all advisory here): the DFMEA review by Alex and two models binds at G-EVT-16 (VER-98, O-R9); the clean-room, return-path and gerbv reviews if time allows (VER-40, VER-21, VER-27). | VER-98 | Alex: half a day, optional before the order |
| 11 | **Alex's decisions** in APPROVALS.md, in signed commits: one `REQ-ALL@<commit>` line for the requirements he agrees with (round 2: not 130 lines), deferrals by ID; the H rows that only built units can detect, accepted for ORDER (`--gate order --verbose` lists them; one `DF-H@<commit>` line or one per row); the BOM values he accepts for EVT (DF-130, DF-147, DF-164, DF-101); u4 pin 58's waiver (DEC-29); one `GATE@<commit>` line that freezes the phases he agrees with (O-R5). | R1, R3, G1 | Alex: 1 h |
| 12 | **Release and order**: the release record; the dry run (valid 7 days); stock on order day (valid 1 day); `check_trace.py --gate order` exits 0; order 5 assembled and 5 bare boards. | VER-42, VER-28, VER-41 | 1 day |

prod_board's own order list has a row of its own (step 0.5).

Rough total: about 6 working days of Claude's work (it was 8.5 before round 3, 10-12 before
round 2), 3-4 hours of Alex's, a design pass of half a day in ref_design, and up to three weeks
of parts lead time in parallel. Round 3 moved 13 checks out of ORDER (VER-26, 29, 32, 36, 49,
51, 98, 112, 138, 140, 159, 165, 177), cut VER-14 and VER-23 to a by-hand pass at ORDER (their
scripts are VER-186 at PVT and VER-187 at DVT) and VER-151 to its llvm-size figure (the rest
is VER-185 at EVT). The order gate's counts are in section (i).

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
| VER-49 the cell's incoming inspection | ORDER (G-ORD-16) | EVT (G-EVT-02) | it needs cells in hand (21 days); DEC-27 needs only the spec sheet (VER-37) (F-R9) |
| VER-138 the cutoff, VER-140 the mating peak, VER-159 the back-power table, VER-32 the charge window, VER-177 the VBUS reach | ORDER | EVT (G-EVT-04) | each sets a BOM value or a firmware rule; the BOM decisions (DEC-05, DEC-07, DEC-17) stay before the order through DEF-48, DEF-40's REQ-BAT-12 and DEF-50, and the four back-power paths are read by hand (O-R9, F-R2(b), F-R2(c)) |
| VER-26 the netlist, VER-29 the rotation table | ORDER | DVT: advisory at ORDER, binding at G-DVT-01 | the order sends no netlist, so JLC's e-test uses the gerbers; the dry run's preview is the reference for the rotations (O-R9, F-R9) |
| VER-51 the fit kit, VER-112 the heights, VER-165 the connector | ORDER | CASE (G-CASE-02) | the case gate has the drawings and the printed parts (O-R9, F-R9) |
| VER-98 the DFMEA review | ORDER | EVT: advisory at ORDER, binding at G-EVT-16 | two rounds have already read the DFMEA (O-R9) |
| VER-14 the matched parts, VER-23 the copied core | ORDER (scripts) | ORDER (by hand); the scripts at PVT and DVT | a wrong C-number puts the wrong part on all five boards, and the +1V1 capacitor question is copper; the scripts become VER-186 and VER-187 (F-R9) |
| VER-151 the game image | ORDER | ORDER, cut to the llvm-size figure | the flash map and the check.sh step are VER-185's, at EVT (F-R2(f)) |
| VER-36 the pin gate's tooling | ORDER | EVT | it changes no copper; the two findings that set the BOM are DEF-61, at ORDER (F-R2(d)) |
| VER-174 the XY chain | ORDER | ORDER (kept) | the comb's spot is copper and the chain needs no parts (O-R9; F-R9 wanted it at CASE) |

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
a load (VER-145, 6.5 h: the timer is 4-6 h) and SR_CLK (VER-179, now 176 mm of track across the board); units 2 and 3
bring-up, soak and thermal; unit 4 abuse, ESD screen, near-field, drop, button life and fault
injection (it never ships); unit 5 spare.

Exit: `--gate evt` exits 0 (G-EVT-01 to G-EVT-16; G-EVT-15 is prod_board's).

### DVT: the revised spin

Candidates, decided from EVT's results. BOM-only (each bumps BOARD_REV): 4.7k pull-downs on
AMP_EN and BL_PWM (DF-45, DF-47); a lower VBAT divider or a reference if VER-139's drift
needs it (DF-95, DEC-07); RN1-RN4 47k if not done
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
- **DEC-27. The cell's gap on every side** (DEF-53, DF-85, DEF-58, VER-181). A worst-case 52 x 36
  cell is -0.15 mm from the edge after tolerances at the pushed end and both long sides, and
  1.35 mm at its lead end. An **unpushed** cell is worse at the lead end: only the pocket's fit
  (0.25 mm, -0.15 mm after tolerances), because assembly.md step 5's tape is all that holds it
  pushed (O-R8). Widening one end of the hole cannot help (the pocket follows the hole: seeded
  in VER-129). Options: (a) buy a cell whose spec sheet guarantees 51.2 x 35.2 mm or less with
  its PCM (VER-37, DEC-23), set case.scad's worst case to it and put the pocket's walls 0.9 mm
  inside the hole at the right end and the long sides and 1.9 mm inside it at the lead end:
  the lead-end wall is the stop, every cell that fits meets every gap, no tape and no copper;
  (b) a 54.8 x 37.8 mm hole with the same pocket: copper, TH1 moves 0.4 mm and the strips lose
  0.4 mm each; (c) restate REQ-BAT-08 to count the printed pocket wall as the barrier: it does
  not hold, since the FR4 edge can stand 0.15 mm inside the wall's line. Proposed: (a), with
  (b) at DVT if no such cell is sold. (a)'s pocket moves the pushed cell 0.9 mm from TH1's edge,
  so DEC-25 is decided with it.
- **DEC-31. The backlight's full scale** (DEF-39, DF-130, DF-159): 122.4 mA worst case against
  the panel's 120 mA (er-tft035ips-6.txt:760). Proposed: R61 3.74k (BOM only: 117.7 mA worst
  case, 108 mA nominal, barely dimmer); DEF-39 is then FIXED and VER-137 closes it. The other
  option, a 97% duty cap in firmware, needs Alex's acceptance of DF-130 for EVT, and DEF-39
  then closes on VER-62's measured current on unit 1, not on VER-137 reading a line in
  pinmap.md (F-R2(a), F-R5).
- **DEC-05. The charge window's hot edge is 51.5 C at the worst corner** (TH1 against U2's
  fixed TS thresholds; DEF-48, DF-147, O-R3; `design_numbers.py tempwin`, VER-32). Charging
  runs from 3.5 C to 47.2 C at nominal values, 1.8 C to 51.0 C at SLUS810N's limits (the hot
  trip is 3.46 kohm: V_HOT 0.270 V over I_TS 78 uA) and 1.4 C to 51.5 C with TH1's own 1% R25
  and B. B4250 gives 49.5 C and B4350 48.9 C at the worst corner. A 45 C hot edge needs B 5130
  or more for a 10k 1% part; its cold edge is then 6.5 C, inside REQ-BAT-03's 0 C. Proposed:
  (a) a BOM-only TH1 change to an 0402 10k 1% NTC with B 5130 or more **if JLC stocks one**
  (Claude checks the stock before this is put to Alex; such a high B at 10k is rare); else (b)
  accept the hot edge for EVT (attended charging in the LiPo bag, VER-172) and set it against
  the cell's own charge range once VER-37 has its spec sheet; a cell whose range runs to 50 C
  or more would pass REQ-BAT-03 as written.
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
  clamp (copper). Or accept DF-101 for EVT (the five units charge from Alex's own chargers).
  DEF-50 holds this decision at the order gate; VER-177 binds at EVT.
- **DEC-07. The cutoff and the VBAT divider** (DEF-40, REQ-BAT-12). The cutoff can fire 35 mV
  low after a CV calibration and never fire on a high-3V3 unit in dropout (VER-138: 3.521 V).
  GPIO47's pad leakage moves VBAT's reading 0.47 V per uA through 235 kohm, and the RP2350's
  IIN is at most 1 uA (rp2350.txt:101199): nine times the +0.05 V window at the limit, so no
  one-point calibration holds over temperature with 470k/470k. **Proposed (O-R6): keep
  470k/470k for EVT; REQ-BAT-12 restated at 25 C (a calibration at 25 C takes in the leakage
  at 25 C); a firmware rule for the dropout case; VER-139 measures the drift from 25 C to 40 C
  on unit 1; a lower divider, a buffer or a reference at DVT if the drift is over the window.**
  Alex records it by approving REQ-BAT-12's new statement, which the order gate needs anyway.
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
  VDDI at up to 3.366 V against the module's 3.3 V maximum (DF-96, DEF-61; its absolute
  maximum is 4.6 V and the ILI9488's own range runs to 3.6 V, er-tft035ips-6.txt:734, :747-748).
  Accept both. DEF-61 closes on VER-34's written answer and VER-23's +1V1 overlay.
- **DEC-12.** The backlight halves at the minimum duty: each sink within +-25% at 18%
  (+-20.2% today). Accept.
- **DEC-13. The redraw time, and the display bus** (back with the ILI9488; DF-180,
  REQ-DSP-11, O-R1). A full 18-bit frame is 461 kB: 184 ms at 20 MHz (the ILI9488's twc of
  50 ns is the limit) and 197 ms at 18.75 MHz, the fastest in-spec clock this board can make
  from its 150 MHz clk_peri; reads cap at 6.7 MHz (trc 150 ns). A 120 ms full frame cannot be
  met in spec over SPI. **This is a copper decision in step 1, with three options: (a) keep SPI,
  REQ-DSP-11 at 220 ms for a full frame (197 ms of bytes at 18.75 MHz plus 23 ms for the commands
  and the colour expansion) and 50 ms for a text update, the battle screens redrawn in parts: no
  copper; (b) the 8080 8-bit bus on J3's pins 19-26 with WRX, RGB565 at
  about 12 ms a frame: a local re-route by J3 and PIO firmware; (c) clocks over 20 MHz: not
  offered, they are out of spec.** A 120 MHz clk_peri gives exactly 20 MHz with CPSDVSR 6, if
  the firmware chooses it. VER-43 times the redraw at the in-spec clock before the order.
  Proposed: (a) for EVT; (b) only if the battle screens feel slow on the breakout, since its
  re-route by J3 must happen before the order or wait for DVT.
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
  'Approvals'). Until then no approval counts. The order gate now fails on a software key (K1,
  O-R12) unless it is run with `--accept-software-key`, which prints and goes into the release
  record. Proposed: an ed25519-sk key on a security key that wants a touch.
- **DEC-34. Freeze the gate state** (O-R5): one signed `GATE@<commit>` line in APPROVALS.md
  after Alex agrees with the phases in section (d); from then on a check, defect, criterion or
  requirement moved later, deleted, retired or made advisory fails the gate (G1) until a newer
  signed line.
- **DEC-35. The order branch moved after be437f8** (7d35933, route.py): re-check VER-01 to
  VER-04 on a board route.py rebuilds, then move the baseline, or keep be437f8's committed board
  and its hand-moved via and let VER-143 show what a rebuild changes (DEF-60). Proposed: let the
  step-1 design pass rebuild with 7d35933's route.py, so DEF-60's fix and the new baseline come
  in one pass.

**The phase conflicts between the round-3 revisions, decided by Claude** (O-R9 and F-R2(f) or
F-R9 disagreed; each choice follows the rule in section d: the order gate keeps what the copper,
the BOM and the fab files freeze)

- **VER-32, the charge window: EVT** (F-R2(c)). Built now, so DEC-05 has its numbers (O-R9's
  point); DEF-48 holds the BOM decision at the order gate. At ORDER a FAIL would block the order
  even after Alex accepted the edge for EVT.
- **VER-177, the VBUS fault reach: EVT** (O-R9). F-R2(f) kept it at ORDER because the fix is two
  BOM values; DEF-50 holds that decision at the order gate the same way.
- **VER-174, the XY chain: ORDER** (O-R9). The comb's spot is copper and the chain is a script
  over positions.json that needs no parts; F-R9 grouped it with the fit-kit measurement (VER-51),
  which moved to CASE.

Retired decisions: DEC-06 (3V3 through the unplug: no gap with the BQ24073), DEC-10 (folded
into DEC-07). DEC-13 came back with the ILI9488.

## (g) Round 2's sign-off objections and where each landed

Both reviewers of the 684a609 register declined sign-off with these 18. All 18 are answered
here; where a fix is partial or the objection did not hold, the row says why.

The next review (round 3, 2026-10-26) raised 23 findings against this register: O-R1 to O-R12 and
F-R1 to F-R11. Where each one landed is section (n)'s ledger; the ones that moved a phase
also appear in section (d)'s "What moved where" table, and the three phases the two reviewers'
revisions disagreed about are decided in section (f).

| # | Objection | What was done | State |
|---|---|---|---|
| O1 major | the cell's gap understated: it fails on every side, and widening the right end would leave VER-181 failing | DEF-53, DF-85, REQ-BAT-08, VER-181, G-ORD-07 and DEC-27 restated on every side; cellgap prints what passes; DEC-27's options re-costed (a cell spec and the pocket, or a 54.8 x 37.8 mm hole); seeded fault: only the right end widened keeps VER-181 failing | fixed |
| O2 major | 'G' only proves some key in a user-global file; README told Alex to sign with an unprotected key | check_trace counts a line only when `%GF` equals the fingerprint README pins and the pin line is itself signed by that key; it warns while the pinned key is a software key; README asks for an ed25519-sk key or a passphrase key never in an agent; seeded fault: a line signed by another allowed key does not count | fixed as far as a file can: a process running as alex that holds an unlocked key can still sign; only a touch key closes that, and Alex chooses it (DEC-30) |
| O3 major | five ORDER checks (VER-13, 21, 23, 25, 108) in no step; prod_board's list missing | section (d) names every ORDER check (VER-23 and VER-108 in step 4; VER-13 moved to EVT; VER-21 advisory; VER-25 folded into VER-132) and prod_board's list; check_trace fails on an ORDER check no step names; seeded fault | fixed |
| O4 minor | APPROVALS.md's header described the old rule | header rewritten: signing with the pinned key is required; unsigned, other-key and merged lines are pending or errors | fixed |
| O5 minor | REQ-TST-06 named CHG_DONE_N; make it VER-161's first seeded case | REQ-TST-06 says CHG_STAT_N | fixed; the seeded case waits for VER-161, which is not built |
| O6 minor | PLAN statements disagreeing with the tool: step 0.1, section (c)'s source, the 129 count, the template's marker spot | step 0.1 says the default mode warns and R8 fails; (c) names its evidence dates; counts come from the tool (section i); the template's marker goes in the clear patch silk.py names, and DEF-57 moves the BOARD_REV text out of the cell's hole first | fixed; the marker's coordinates wait for DEC-28 |
| O7 minor | per-unit screens a good unit can fail: TS at 25 C only; the 130 ohm window | VER-133, VER-135 and VER-68 take the band from `design_numbers.py tsband` at the measured ambient, now with I_TS at 72-78 uA; VER-134 used 133 ohm from a window round 3 found empty (O-R2); VER-134 and VER-145 now use 160 ohm in parallel with a 3.5 V TL431 shunt and 22 ohm, which `cvload` checks at every corner | fixed in round 3 (O-R2): every constant now comes from SLUS810N on disk |
| O8 minor | DF-170 offered only a procedure | DEC-33 offers the copper (J4 a JST SH 2-pin, now or at DVT); VER-184 (FAIL) shows the cause; G-DVT-06 makes the DVT spin fix it | fixed; Alex chooses when |
| O9 minor | G-ORD-13 and G-ORD-06 needed signatures that prove nothing | G-ORD-13 RETIRED in 2d1124d (check_trace now knows a RETIRED criterion); G-ORD-06 names VER-33; W-01 and W-02 kept as IDs, not needed | fixed |
| O10 minor | decision IDs collide with diodes | D1-D30 renamed DEC-01 to DEC-30 in every file | fixed |
| O11 minor | VER-153 at ORDER; 127 signatures for a 5-board order | VER-153 at EVT; bulk lines `REQ-ALL@<commit>` and `DF-H@<commit>`, each ID still checked on its own (seeded fault: an edited statement is not covered); the order gate trimmed (section d) | fixed |
| O12 minor | bring-up docs not gated before first power | VER-39 in G-EVT-02 and stated as an EVT day-one precondition (e); bringup.md's display section is the 2.4 in panel's, logged as DEF-21 (n) | fixed in the register; bringup.md itself is outside hardware/vv |
| F1 major | as O1 | as O1 | fixed |
| F2 minor | REQ-MCU-06: GPIO28 is PWM2 A, GPIO31 PWM3 B | not changed (round 3's F-R11 agreed; VER-15's evidence will print the whole table): the RP2350's table maps GPIO 0-31 to slices (n >> 1) & 7 and 32-47 to 8 + ((n >> 1) & 3), so GPIO28 is slice 6 A and GPIO31 slice 7 B (`grep -n "PIN_28\|PIN_31" ~/.cargo/registry/src/*/embassy-rp-0.8.0/src/pwm.rs`: 'PWM_SLICE6, ChannelAPin', 'PWM_SLICE7, ChannelBPin'). VER-15 now derives each slice from the table, as asked | objection did not hold; its request applied |
| F3 minor | stale names: REQ-TST-06, vp 212's C41, placement.py and design.py comments | REQ-TST-06 and vp 212 fixed; the comments (outside hardware/vv) logged in DEF-21 (p); VER-39 now covers placement.py | fixed in the register |
| F4 minor | gate logic: a check re-phased leaves its requirement; no DFMEA:COVERAGE fault; DF-95 and DF-96 rated 2 on one check | a requirement must have a live check no later than its Phase; DFMEA:COVERAGE seeded (R64 unnamed); D of 2 or 1 needs two checks, DF-95 and DF-96 now D4 and DF-95's prevention says the condition is by design | fixed, three seeded faults |
| F5 minor | REQ-MEC-09 unmeetable for the end screws | split per pilot: 4.0 mm in the side pilots, 3.0 mm in the end pilots; M2 x 7 at the sides passes | fixed |
| F6 minor | VER-13, 21, 27, 34 reached only by PHASE:ORDER; effort | VER-13 to EVT (G-EVT-04); VER-21 and VER-27 advisory; VER-34 in G-ORD-01; the gate trimmed | fixed |

## (h) What this plan still cannot catch

- **Small numbers.** Five boards, then 30, cannot show a failure rate under a few percent.
- **Time.** Cell ageing and swelling over years, ENIG wear, tape creep, slow flux migration.
- **Faults nobody listed.** The DFMEA names every part, but only the failure modes someone
  thought of; seeded faults prove a checker fails on the faults seeded.
- **Numbers this register quotes from memory.** The charger's, the ILI9488's and the RP2350's
  now come from the files on disk with their line numbers (O-R6); JLC's 0.30 mm and 0.2 mm
  tolerances (VER-17) and the cell's size and PCM (VER-37) still do not. The datasheet files
  are untracked: their sha256 is the only proof of which copy was read.
- **The panel's mechanics.** Its datasheet was downloaded 2026-10-26 to the main checkout
  (untracked; the extracts stay out of git), but case.scad's outline, thickness and FPC shape
  are still the old 3.5 in design's numbers and differ from the drawing (DEF-59). The fold
  makes the outline an estimate either way: only a real panel (VER-43, VER-136, VER-152) can
  show J3's place is right.
- **The cell's lead end.** Nothing but a piece of tape holds the cell pushed (DEF-58): until
  the pocket's lead-end wall moves in (DEC-27(a)), an unpushed cell's lead end has only the
  pocket's fit.
- **The bought breakout's header numbering.** The product's panel-side pairing is 51-j, and
  the fold changes no contact pairing, but the loose breakout's own header numbering against
  the FH12's pads is unknown until VER-43's diode and continuity tests (F-R1).
- **A hand edit in the board file.** 62b01f2 moved one GND via by hand; any rebuild (a silk.py
  or rule change, DEC-28, DEC-32) undoes it, and the fix belongs to the design pass
  (DEF-60). The release run must rebuild, re-run DRC and diff the copper, not trust the
  committed file (VER-143).
- **A gate state nobody has signed.** The register's own phases are in force until Alex
  signs a `GATE@<commit>` line; anything moved later without a newer signed line fails the
  order gate (G1), so a phase cannot be moved quietly.
- **A key that a process can use.** A pinned software key is only as safe as its passphrase
  habit (DEC-30). The order gate now fails on one, unless it is run with
  `--accept-software-key`, which prints in the gate result and has to be copied into the
  release record's signing section (K1). A process running as alex could also pass that flag:
  the release record is where Alex sees it.
- **JLC's placement.** Whether JLC places J1-J4 by their body centres, as their origins are, or
  by something else, only the dry run's preview shows (O-R7).
- **Bench honesty, standards, JLC's side, firmware beyond the self-test, use**: as the first
  run listed them.
- **A design that moves again.** Another session may commit to ref-design-35in. Any change to a
  design path fails the default mode and every gate until its rows are re-checked; the
  register cannot re-check them itself.
- **Quoted values until VER-161 exists.** A GPIO, a count or a dimension quoted in a row can
  still drift from design.py unseen (DEF-54).

## (i) The gate today

**The last recorded run** was at 0c1571a (2026-10-26), whose evidence was made at 807d0e0 (the scripted checks) and 1bc504b (VER-129):
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

**This round changed the register** (23 findings: sections (g) and (n)). Ten checks moved
phase, three checks were added (VER-185 at EVT, VER-186 at PVT, VER-187 at DVT), two were cut
to a by-hand pass at ORDER (VER-14, VER-23), three defects were logged (DEF-58, DEF-59,
DEF-60), two tools changed and the trace gate grew three rules. The transcript above predates
all of it, so its counts are stale. It has not been re-run: this session had no shell, and
`python3 hardware/vv/tools/check_trace.py`, `python3 hardware/vv/tools/check_trace.py --gate
order` and `python3 hardware/vv/tools/seed_faults.py` are still to be run on the committed
tree.

**What the order gate does now.** It still fails, as it should: the copper, the BOM and the
fab files are not frozen, Alex's decisions are not signed, and the H rows that only built units
can detect are not accepted. Three gate results are new:

- **G1, the gate state Alex froze.** A signed `GATE@<commit>` line freezes each check's phase,
  each defect's Gate, each criterion's Needs and each requirement's Phase and Verified-by.
  The newest committed line wins, and anything moved later than it fails the order gate.
  There is no such line yet, so the register's own phases are in force.
- **K1, the pinned signing key is a hardware key.** The order gate fails when the pinned
  key's type in the allowed-signers file is not an sk- key, unless the gate is run with
  `--accept-software-key`; the flag's use is printed in the gate result and has to be copied
  into the release record's signing section. An in-repo exception line does not count.
- **R8 now holds the case too.** A case-only commit on the order branch passes only when
  `case/positions.json`, regenerated at that commit, has the same sha256 as the one
  regenerated at the baseline; both hashes are printed in the gate output and go into the
  release record.

The case gate, EVT, DVT, PVT and prod_board's order gate move with the phases in (d).

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

## (n) Round 3's findings and where each landed (2026-10-26)

Two models attacked the re-seeded register and judged each other's findings: 23 findings,
O-R1 to O-R12 and F-R1 to F-R11, applied as the judge revised them. A first integrate pass
(another model, no shell) left the register failing its own default mode; this pass reviewed
it finding by finding, corrected it, ran every tool and committed. Where the revision itself
was wrong, the row says what was done instead. Seven findings carry seeded faults in
`tools/seed_faults.py` (VER-129).

| ID | The finding | Where it landed |
|---|---|---|
| O-R1 | the redraw numbers assumed an out-of-spec clock | twc 50 ns and trc 150 ns quoted from ili9488.txt:16258, :16262 in REQ-DSP-11, DF-180, VER-34, VER-43, VER-61 and (c): 184 ms at 20 MHz, 197 ms at 18.75 MHz; VER-61's pass band capped at 20 MHz, the fastest clock only an observation; DEC-13 a copper decision in step 1 with REQ-DSP-11 proposed at 220 ms (200 ms would leave 3 ms over the bytes) or the 8080 bus |
| O-R2 | no load resistor gives a CV window | `cvload` computes pre-charge from KPRECHG/R5 and termination from ITERM x ICHG and prints "no resistive window" (147.6 against 126.7 ohm); I_TS_TOL 0.04. Corrected: the revision's branch, "a 3.3 V zener or three series diodes", does not work (three diodes conduct at about 2 V, under V_LOWV; a 3.3 V zener's soft knee starves the CV current), so VER-134 and VER-145 use a 3.5 V TL431 shunt with 22 ohm beside 160 ohm, which `cvload` checks at every corner; VER-145 waits 6.5 h, since SLUS810N's timer is 4-6 h (it waited 5.5 h) |
| O-R3 | B4250 still lets charging run to 49 C | VER-32 built as `design_numbers.py tempwin` (FAIL): 3.5-47.2 C nominal, 1.4-51.5 C at the worst corner; B 5130 or more holds 45 C, cold edge 6.5 C. Corrected: the integrate pass called 51.0 C "nominal" (it is the charger's limit with TH1 nominal), put the cold edge's worst corner at a high B (it is a low B) and asked for B "about 5030" (that ignores TH1's own 1%); its Monte Carlo is gone, since every term is a datasheet limit and the worst corner bounds every unit |
| O-R4 | a case-only commit passed R8 at ORDER | R8 regenerates positions.json at the order branch's tip and at the baseline and fails on "case commit moves board positions"; both sha256s print even on a PASS and go into the release record (TEMPLATE, check_releases). Today 2ddb7fe and be437f8 both give 179f81ed..., so the labels move nothing. Seeded: a ss_pitch change fails, 2ddb7fe passes |
| O-R5 | the gate state could be weakened by unsigned commits | a signed `GATE@<commit>` line (FROZEN, with a scope phase) freezes each check's phase, defect's Gate, criterion's Needs and requirement's Phase and Verified by (G1). Corrected: the integrate pass flagged a criterion losing ADVISORY (a tightening) and missed one gaining it; G1 now also catches a deleted row, a check turned CUT or RETIRED, a criterion gaining ADVISORY, ADVISORY:VER-nn or RETIRED, and a check dropped from a Verified by, and holds only up to its scope. Seeded: a check re-phased after GATE@ fails (DEC-34) |
| O-R6 | the datasheets were on disk but treated as missing | VER-34 names every constant's file and line; `tools/datasheet_hashes.py` records each .txt and .pdf's sha256 and size in evidence (the extracts stay out of git); DF-61, DF-123, VER-152 and (c), (h) corrected; DEF-59 (Gate CASE) for case.scad's panel numbers (drawing drawn Dec-20-22: 54.56 x 82.84 +-0.2, 2.1 or 2.15 +-0.15, a 4.47 mm neck); DEC-07 decided as proposed and REQ-BAT-12 restated at 25 C. Corrected: the integrate pass's statement put the leakage "inside the window" at "0.235 V of VBAT" (it is 0.47 V of VBAT, nine times the window); the statement is now simply at 25 C and the rationale shows the term |
| O-R7 | correcting the CPL could create J1's offset | the preview is the reference: VER-176 keeps 0.10 mm and passes J1-J4 only through committed JLC_FIXED evidence; VER-29 and step 3 move no CPL row before the preview. Corrected: DEF-33 said "the fix is fab.py writing the pads' centroid", which O-R7 overruled; it now states the offsets and that JLC's placement is unknown until the preview |
| O-R8 | the lead end depends on tape | `cellgap` prints and fails the unpushed lead end; DEF-58 (Gate CASE); DF-04, DF-85, REQ-BAT-08. Added: the fix is the pocket's own lead-end wall 1.9 mm inside the hole (the rest 0.9 mm), so every cell that fits meets every gap with no tape; cellgap prints it and DEC-27(a) carries it with DEC-25 |
| O-R9 | about eight ORDER checks freeze nothing | VER-138, VER-140, VER-159 to G-EVT-04; VER-51 to G-CASE-02; VER-174 kept at ORDER; VER-26, VER-29 and VER-98 advisory at ORDER (ADVISORY:VER-nn) and binding at G-DVT-01 and G-EVT-16; VER-32 and VER-177 also to G-EVT-04 (section f); step efforts re-costed, about 6 Claude days. DF-101 and DF-103's acceptance for EVT is Alex's (step 11) |
| O-R10 | the hand-moved via had no defect | DEF-60 (Gate ORDER); VER-143 rebuilds, re-runs the DRC and diffs the copper. 7d35933 on the order branch may be the generator fix; DEC-35 |
| O-R11 | step parsing counted the whole section | `check_plan_steps` reads only numbered step rows; prod_board's list is step 0.5; seeded: an ORDER check named only in the table fails |
| O-R12 | a software key only warned | K1 at the order gate: an sk- key, or `--accept-software-key`, whose use always prints ("--accept-software-key used") for the release record's new signing section; a `KEY@` line in APPROVALS.md is warned about and ignored. Seeded: a software key fails, the flag passes and prints, an APPROVALS line does not count |
| F-R1 | VER-43 wired by the fold map | VER-43 finds the breakout's header numbering by diode and continuity tests, checks it equals 51 minus the FH12 pad, and wires by the panel's pin names; VER-136's reference is section 5 step 6; DF-57 and REQ-DSP-03's note say the fold changes no pairing |
| F-R2 | BOM and firmware tunings held the order | (a) DEF-39, 48, 49, 50 keep Gate ORDER and leave OPEN only by a value in design.py (FIXED, then their check closes them) or by Alex's acceptance for EVT (the Gate moves under his next GATE@ line); corrected: the integrate pass wrote that all four "no longer close on VER-32 passing", which was true of DEF-48 only. (b) DEF-40 at EVT, its decision recorded as REQ-BAT-12's approval. (c) VER-32, VER-140, VER-159 to EVT; the four back-power paths read by hand (step 5). (d) DEF-26 split: the tooling (VER-36) to EVT, the two BOM findings are DEF-61 at ORDER (the integrate pass had moved all of DEF-26 and left VER-36 at ORDER). (e) DEF-27 to CASE, DEF-28 and DEF-36 to EVT, VER-04's warning list read by hand (step 2). (f) VER-33 and VER-137 at ORDER; VER-177 to EVT (section f); VER-151 cut to llvm-size, VER-185 at EVT |
| F-R3 | the baseline was compared only with HEAD | `baseline_diff` also compares each hash with the baseline commit's blob; a real re-seed moves the baseline commit too, so the seeded helper does; seeded: a hash rewritten from HEAD after a design.py edit fails |
| F-R4 | JLC_FIXED passed on a word | an entry counts only with a committed evidence file naming the designator, in check_trace and in `cpl`; seeded twice (no file; an uncommitted file). O-R7 overrules its "fab.py writes centroids" |
| F-R5 | VER-137 could close DEF-39 on a doc line | DEF-39 and DEC-31: with a duty cap it closes on VER-62 on unit 1 |
| F-R6 | the LED test needed a high diode-test voltage | VER-136: 3.3 V at 5 mA through 330 ohm, 2.4-3.0 V across the pins, then OL reversed; instrument and accuracy filled (0.02 V) |
| F-R7 | panel.json and case.scad disagree about the datasheet | DEF-21 (s): pin_review/ds/ does not exist, but hardware/ref_design/datasheets/ holds the PDF and its .txt; panel.json's J3 at (100, 120.89) against the board's (100.0, 117.59) |
| F-R8 | the ID4 read's pin | VER-43 reads on SDO, then on SDA, and records which answered; VER-34 quotes the IM=111 table; REQ-DSP-01 restated only if the read is on SDA |
| F-R9 | parts in hand should not hold the order | VER-49 to G-EVT-02; VER-14 and VER-23 by hand at ORDER, their scripts VER-186 (PVT) and VER-187 (DVT); VER-112, VER-165, VER-51 to CASE; VER-174 kept (section f); VER-26 advisory, the order sends no netlist |
| F-R10 | DF-90 named the old lid | DF-90 is the one-piece top and the tub; it no longer cites the retired DF-89 (the integrate pass's text did, a structure error) |
| F-R11 | record the PWM slice table | VER-15's pass limit asks its evidence to print the whole table and the pwm.rs lines; the row's commas had broken the CSV and are quoted |

What the integrate pass broke and this pass repaired: VER-15, VER-43 and DF-09 had unquoted
commas that shifted their fields (the "deleted" rows), DF-90 cited a retired row, gates.md lost
the rule that a moved criterion stays as ADVISORY, and VER-36's phase left REQ-PWR-14 with no
ORDER check; the seeded faults' release record lacked the case-positions fields, and their
rebaseline helper did not move the baseline commit. PLAN (d) and (i) claimed the counts could
not be run; they are run below.
