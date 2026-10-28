# History: the register's earlier rounds

The ledgers that used to sit inside [PLAN.md](PLAN.md), moved here on 2026-10-27 (round 3's sign-off, O-S9):
PLAN.md is the plan; this file is what earlier rounds found and where each finding landed. The section
letters are PLAN.md's as they were at be437f8, so older commit messages and defects still find them.
Nothing here is read by check_trace.py. The design these sections describe is be437f8 unless they say
otherwise; the board being ordered is 71737ae (PLAN.md section b).

Contents: (b) and (c) at be437f8: what changed from 684a609 and what the checks found; (i) the gate after
round 3; (g) round 2's sign-off objections; (k) the first run's ID ledger; (l) the second review's ID
ledger; (m) the first run's sign-off objections; (n) round 3's 23 findings; the grid's audit (2026-10-27).

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

## (c) The board at be437f8: what the checks found

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
| The panel's mechanics are estimates | its outline, active area, thickness and FPC shape in case.scad are the old 3.5 in design's. The datasheet is on disk in the main checkout (hardware/ref_design/datasheets/er-tft035ips-6.pdf and .txt, REV 1.0 of Dec-15-2022, its drawing drawn Dec-20-22; untracked, their sha256 in evidence/2026-10-26_VER-34_datasheets.txt): 54.56 x 82.84 +-0.2 mm, a thickness of 2.1 (table) or 2.15 +-0.15 mm (side view) and a 4.47 mm neck, while case.scad uses 54.50 x 83.00 x 2.45 mm and 4.59 mm. The fold makes the outline an estimate either way, and these numbers set J3's place (O-R6, F-R7) | DF-61, DF-123, DEF-59, VER-136, VER-152 |
| A hand edit in the board file | one GND via moved 0.08 mm by hand at 62b01f2; the next build_pcb run undoes it, and every step-1 change rebuilds | (h), DEF-60 (Gate ORDER); VER-143 now rebuilds, re-runs the DRC and diffs the copper; 7d35933 may fix the cause |
| The order branch moved | after be437f8: 2ddb7fe engraves button labels on the top's inside (case.scad only; positions.json regenerated there hashes the same as at be437f8, 179f81ed..., so it moves no board position: O-R4); 7d35933 changes board/tools/route.py (a pre-flight DRC, and core vias that fail clearance are dropped: it may be the generator fix DEF-60 asks for, unverified); the rest are print-kit and docs commits | R8 fails at the order gate on 7d35933 until VER-01 to VER-04 are re-run on a board route.py rebuilds and the baseline moves (DEC-01) |
| Swelling room | 0.80 mm under the panel's back (0.30 under a deck at 684a609): passes | VER-182 PASS, DEF-51 CLOSED |
| Screws | M2 x 6 gives 3.2 mm of thread in the side pilots (want 4.0; M2 x 7 passes, its tip 0.8 mm short) and 3.2 mm of the end pilots' 3.30 (ok) | VER-147 FAIL, DEF-31, DEC-18 |
| Two pin-review findings that set the BOM | the panel's VCI and VDDI at up to 3.366 V against the module's 3.3 V (absolute 4.6 V; the ILI9488 itself runs to 3.6 V: er-tft035ips-6.txt:734, :747-748); the second 4.7 uF on +1V1 | DEF-61 (split from DEF-26), VER-34, VER-23, DEC-09 |
| Unchanged | cutoff 3.521 V at the worst corner (VER-138 FAIL, now binding at EVT); VBUS fault reach 6.9 V (VER-177 FAIL, now binding at EVT); bom.md's domes and Start/Select caps (VER-167 FAIL); TH1 0.80 mm from the hole's edge, no back pad within 1 mm (nearest TP3, 5.5 mm: VER-164 PASS); nearest MLCC C53, 11.40 mm from H5 (VER-142 PASS); every hole 0.30 mm or more (VER-114 PASS); VBUS 10 uF (VER-99 PASS) | as named |

## (i) The gate after round 3 (at c4b92e6, 2026-10-26)

**The run after round 3**, at c4b92e6 (2026-10-26); the scripted checks' evidence was made at
fec2fc4 and VER-129's at b187fcf: requirements 133 (130 PROPOSED, 3 RETIRED); DFMEA rows 181
(64 H, 84 M, 33 L by action priority; 33 RETIRED); checks 186 (14 PASS, 12 FAIL, 51 NOT_RUN,
101 NOT_BUILT, 5 CUT, 3 RETIRED); defects 61 (28 OPEN, 20 FIXED, 6 CLOSED, 7 RETIRED); seeded
faults 82 of 82 caught (VER-129).

```
$ python3 hardware/vv/tools/check_trace.py
STRUCTURE: OK (5 warnings)
  approvals by Alex: 0 counted (signed), 0 pending; README.md pins no signing key
  order branch: ref-design-35in has design commits after be437f8: 7d35933 (route.py),
  2ddb7fe (case.scad's engraved labels)
  design baseline: matches HEAD (and be437f8)
  warnings: the order branch's design commits; the pass bands of VER-67, VER-88, VER-134 and
  VER-139 are 1.4-2.2 x their DMM's accuracy (want 4 x)
exit 0

$ python3 hardware/vv/tools/check_trace.py --gate order
  PASS  G-ORD-15 (VER-98 advisory, DFMEA:COVERAGE), G-ORD-24 (VER-129), R5, R6 (no release
        yet), R7
  INFO  G-ORD-05, G-ORD-08, G-ORD-12, G-ORD-20, G-ORD-26 (advisory); RETIRED G-ORD-13;
        N/A G-ORD-25 (prod_board)
  FAIL  G-ORD-01 to G-ORD-04, G-ORD-06, G-ORD-07, G-ORD-09 to G-ORD-11, G-ORD-14, G-ORD-16
        to G-ORD-19, G-ORD-21, G-ORD-23 (the FAILs among them: VER-18 edges, VER-131 drill,
        VER-176 CPL, VER-181 the cell's gap, VER-137 backlight)
  FAIL  G-ORD-22: 32 ORDER checks not passing (45 before round 3, 67 before round 2); 12 pass
  FAIL  R1 127 PROPOSED; R2 30 requirements with an ORDER check not passing;
        R3 53 H rows neither accepted nor DONE; R4 15 OPEN defects with Gate ORDER (DEF-14,
        29, 30, 32, 33, 39, 48, 49, 50, 53, 55, 56, 57, 60, 61)
  FAIL  R8 7d35933 (route.py) on ref-design-35in after be437f8; case positions:
        positions.json sha256 179f81ed... at dd54d0f and at be437f8, so 2ddb7fe moves no
        board position
RESULT: FAIL (22 of 27 not met); 1 N/A, 5 advisory, 1 retired
```

G1 and K1 print nothing yet: no `GATE@` line is signed and no key is pinned. The case gate
fails 11 of 14, EVT 19 of 22, DVT 10 of 13, PVT 10 of 13, and prod_board's order gate 5 of 7
(24 criteria N/A). The checks that pass: the netlist, the pin gate, ERC, DRC with parity, both
4-ring contact footprints, inrush, holes, the MLCC keep-out, TH1's geometry, swelling room,
two inspections (VER-104, VER-124) and the seeded faults (VER-129). VER-32 (the charge window)
and VER-177 (the VBUS reach) FAIL and print at the order gate as advisory; they bind at
G-EVT-04.

What the order gate holds after round 3, beyond the copper, BOM and fab checks: the BOM
decisions through their defects (DEF-39, 48, 49, 50, 61), the hand edit through DEF-60, the
order branch through R8 (7d35933 today), a frozen gate state through G1 once Alex signs a
`GATE@` line, and a hardware signing key through K1 once he pins one.

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

## The grid's audit (2026-10-27)

Another model audited the grid: 10 findings. Each was checked against the register.

| # | Finding | Landed |
|---|---|---|
| 1 | a NOT_RUN bench test of a later phase cleared a cell of classes 1-14 (1/audio, 1/buttons, 6/cell, 6/audio, 6/buttons, 7/usb_debug, 14/mcu) | applied: grid.py's can_clear(); all seven now read planned, and 6/audio had no clearing check at all, so VER-196 (the amplifier's states on paper, ORDER) was added for it; seeded (VER-70 re-tagged into 3/case still leaves R9 holding it) |
| 2 | 7/usb_debug rested on VER-98, and no check analyses VBUS on J1's CC and data pins | applied in part: VER-195 (A, ORDER, NOT_BUILT) is the USB connector's fault reach, a new row and not an extension of VER-177, because the SRV05-4's datasheet is not on disk, so its limits cannot be quoted yet; VER-195 adds the step to design_numbers.py once it is |
| 3 | VER-144's pass limit named no number for U7, Q5 and Q6 | applied: VER-144's limit now holds U2, U3, U7, Q5 and Q6 to their junction maximum less 20 C at 40 C ambient, each thermal resistance quoted; VER-193 keeps its own parts (it extends VER-144's table, so tagging it display and audio would claim parts it leaves to VER-144) |
| 4 | the class 15 estimates were EVT checks, so nothing needed their number before the order | applied: VER-144 and VER-193 are ORDER (G-ORD-22 needs them, step 9.5 names them); class 15 clears only on a PASS or FAIL, seeded; VER-192 says VER-193 runs first; VER-13 stays EVT (section d's table) |
| 5 | no paper check on the shift-register chain's signals | applied: VER-21 names SR_CLK, SR_PL and SR_DATA and is tagged buttons; 12/buttons is planned on it. The optional edge-time step (R65 into U8 and U9) is not added: VER-21 and VER-179 cover the cell between them |
| 6 | 11/case's reason said VER-147 checks bom.md, which it did not | applied both ways: the reason names VER-167, VER-147 and VER-173 for what each checks, and design_numbers.py screws now reads the length bom.md's Screws line names and fails when it is not case.scad's (M2 x 6 on both today); VER-147 covers bom.md |
| 7 | VER-191's capacitance limit had no number | applied: 100 pF on D+ and D- (CINUB, USB 2.0 Table 7-7), with the README's note on clauses quoted from memory |
| 8 | stale_names.py passed a ref that never existed (a typo) | applied: a ref with a baseline prefix that no design.py on the history had fails; a range's ends are read and a number 0 is a pin name (the 74HC165's 'D0 11' in DF-163, the one hit); seeded with R666 |
| 9 | 6/mcu rests on VER-104, a documentation check | rejected: VER-104 is an ORDER inspection of the mcu's reset after a hang (the watchdog releases PWR_HOLD), which is class 6 for the mcu, so it clears the cell; tagging it power only would hide what it targets. The depth (power-up, brown-out, E9) is VER-16, VER-77, VER-105 and VER-159's, which round 3 moved to EVT on purpose |
| 10 | a planned cell printed bench tests as if they could clear it | applied: a planned or GAP cell prints the checks that can clear it first, then "measures, does not clear:" and the rest |

Rejected in part, finding 2: VER-98 keeps class 7. Its pass limit reads every subsystem's fault
rows, which is what class 7 is; class 19 is the right-baseline class, which it does not check.
After finding 1 it no longer clears any cell before the order (it is an EVT inspection).

The run after the audit: the scripted checks re-run at c1c61f0 give the same numbers as at
fec2fc4 (VER-147 adds its bom.md line: M2 x 6 on both sides), VER-188 passes with the new
never-existed rule, and VER-129 at 6289d2a catches 95 of 95 seeded faults, so DEF-35 and DEF-52
are CLOSED again. The default mode exits 0; the order gate fails, R9 among its criteria.

## (d) What moved where on 2026-10-26 (from PLAN.md section d at be437f8)

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
| VER-144 heat per mode, VER-193 board-wide heat (2026-10-27) | EVT | ORDER | Alex's grid rule puts class 15's paper estimate before the order, and an estimate exists only once it has run, so G-ORD-22 now needs their result; VER-13 (copper IR drop) stays at EVT: VER-144 and VER-193 together reach all eight subsystems |

## (o) PLAN.md at fd1f6e8 (the RP2354B board, moved 2026-10-28)

The plan's header and sections (b), (c), (d), (f), (g), (i), (k) and (l) as they stood at fd1f6e8,
for the RP2354B board at 71737ae. Their headings are one level down here.

### The header at fd1f6e8

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

### (b) What changed in the design (be437f8 to 71737ae)

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

### (c) The board at 71737ae: what the checks find

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

### (d) Before the 5-board order

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

### (f) Decisions for Alex

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

### (g) Round 3's sign-off objections and where each landed

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

### (i) The gate today

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

### (k) The coverage grid (2026-10-27)

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

### (l) The review of 2026-10-27: 16 findings and where each landed

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
