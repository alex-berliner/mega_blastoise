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

