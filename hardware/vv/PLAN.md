# The plan: from here to 5 boards, then 30

**The register targets the next spin: the RP2350B board** (docs/static/DESIGN.md:71 and
docs/static/next_spin_requirements.md, Alex, 2026-10-29): the Raspberry Pi RP2350B (QFN-80, no flash inside,
C42415655) with one GigaDevice GD25Q128 16 MB QSPI flash (C2758105) that holds the program and the sound, the
core laid out as Raspberry Pi's 2-layer RP2350B minimal design draws it, flash included; today's ILI9488 panel;
no GBA, no Linux, no SD card, no Wi-Fi, no USB host mode; online play through an Android phone's Chrome (WebUSB)
as a Showdown guest; the bare-metal Rust battler on screen within about a second; recovery through the RP2350's
USB boot ROM over USB-C. The stripboard Pico 2 (an RP2350A) already runs the battler. There is no RP2350B
schematic or layout yet: the KiCad files on main are still the ESP32-S3 board's (`hardware/ref_design`), so
every RP2350B check is NOT_RUN or NOT_BUILT and the design baseline names the dropped board (README,
'- Baseline board: `esp32s3`'). Restoring the RP design is a later stage.

**Three earlier boards are dropped.** The ESP32-S3 board's register, with every round's ledger, its decisions
DEC-40 to DEC-49 and its order checklist, is at tag `vv-esp32s3-final` (c556556); its last results after
main's path moves are at 30131db. The RP2354B board's is at tag `vv-rp2354b-final` (c8ac9dd; its order gate
failed 21 of 29 with 21 open defects). The F1C200s board's, written on 2026-10-28 and dropped on 2026-10-29
when Alex changed the spin, is at tag `vv-f1c200s-final` (cffe55b). Their rows here are DROPPED (README,
Rules): neither PASS nor CLOSED, out of every gate of the next spin, each naming its board and tag.
HISTORY.md keeps the earlier rounds.

Contents: (a) the system on one page; (b) what the re-target changed; (c) where the register is on the
new-MCU path; (d) before the 5-board order; (e) the case gate, EVT, DVT and PVT; (f) decisions for Alex;
(h) what this plan still cannot catch; (i) the gate today; (k) the coverage grid.

## (a) The system on one page

Requirements (requirements.md, each allocated to the RP2350B board's subsystems) -> DFMEA rows
(dfmea.csv) -> checks (verification.csv) -> the new-MCU path and the phase gates (gates.md) ->
release and change records (releases/) -> defects (defects.md). Alex's decisions live in one file,
releases/APPROVALS.md, which only he writes lines in, in commits signed with the key README pins.
`tools/check_trace.py` enforces the chain; its docstring and README's Rules list every rule.
`tools/seed_faults.py` proves each rule can fail (VER-129). `tools/grid.py` writes the coverage grid.

## (b) What the re-target changed (2026-10-29)

| What | Register effect |
|---|---|
| Alex's change of the next spin (DESIGN.md:71, next_spin_requirements.md) | board `rp2350b`; allocations `.B` (the `.F` ones are DROPPED, the F1C200s board's); the F1C200s board is DROPPED with tag `vv-f1c200s-final` |
| Dropped with the F1C200s board | REQ-PRD-12, -13, -16 (GBA, link play, games and saves), REQ-PWR-22 (VBUS out as host), REQ-STO-01 to -03 (the microSD slot); DF-203, -204, -207, -208, -209, -243, -244, -245; checks VER-206, -209, -221, -222, -225, -226, -229, -234, -247, -250 |
| Rewritten for the RP2350B | REQ-PRD-14 (the phone web bridge), REQ-PRD-15 (no Linux part), REQ-PRD-09 (Alex's all-in cost and limits), REQ-PRD-03.B, -08.B (recovery over the RP2350 USB boot ROM), REQ-PWR-20, REQ-SOC-01 to -08, REQ-USB-01, -02; the chip gate's checks VER-223, -224 and the checks that named the F1C200s |
| New | REQ-PRD-18 (a GBA-style sound engine from the 16 MB flash), REQ-PRD-19 (update by a USB drive), REQ-SOC-10 (bootloader, partitions, A/B), REQ-SOC-09 (the flash's footprint and wiring follow the minimal design); checks VER-251 (the phone bridge test), VER-252 (XIP from the 16 MB flash), VER-253 (the flash's wiring), VER-254 (the footprints), VER-255 (sound on a unit), VER-256 (the flash's budget), VER-258 to VER-261 (the hard reset, the boot ROM's recovery, the drive update, the sound's provenance); DF-252 to DF-265 |
| Carried from the RP2354B board (tag `vv-rp2354b-final`) | the errata analysis (analysis/errata_rp2350.md, VER-245), the copied core's lessons as DF-256 to DF-261; its 21 open defects stay at the tag: they were findings on that board's files, and the lessons that apply to any board are carryover.md's |
| The new-MCU path | gates.md NM-01 to NM-11; NM-03 is the chip gate: the phone bridge test plus XIP from the 16 MB flash (VER-251, VER-252); check_trace prints the stage |

## (b2) Alex's review of the REQ-PRD rows (2026-10-29, in chat)

Written into the register in branch vv-review-1; every row stays PROPOSED until Alex signs it in APPROVALS.md.
Approved as written: REQ-PRD-02.B (the 8-bit 8080 bus confirmed, evidence in its Notes and VER-199), -03.B (4.0 h,
the 700 mAh cell stays), -14, -15 (and the RP board's hold-to-latch at power-on dropped: REQ-PWR-21, VER-240).
Approved and rewritten: -08.B (recovery in four layers: the hard reset, Select at power-on, the boot ROM's hash check
and fallback, the BOOTSEL pad; REQ-SOC-04, REQ-SOC-10, VER-241, VER-258, VER-259), -18 (a GBA-style sound engine;
VER-252, VER-255, VER-256, VER-261, DF-265). New: -19 (update by dragging a .uf2 to a USB drive; VER-260).
Approved with Alex's numbers: -09 ($60 at 10 units, $40 at 50). Unchanged, approved at c3aec22: -01, -01.B, -02, -03,
-04, -04.B, -05, -07, -08, -10, -11, -17. Open for Alex: DF-263 (block protection on the bootloader against a
recovery .uf2 that rewrites it).

## (b3) The artifact audit applied (2026-10-29)

analysis/artifact_audit_2026-10-29.md found leftovers of the dropped boards inside live rows (36 REWORD, 2 ASK, 345 KEEP).
All 36 REWORD rows carry its corrected text, and the 34 KEEP rows with an old rationale are re-grounded on the RP2350B
board (the RP board's circuits at tag vv-rp2354b-final, the RP2350B, the GD25Q128, the TPS3421). Alex's two answers:
(1) the TPS3421 runs from 3V3, so it draws nothing while the unit is off (REQ-PWR-07, DF-266 for the back-power
path, VER-258, VER-159); (2) the 20 contacts are read directly on GPIOs, with no 74HC165s (U8, U9
and U11 go; REQ-PRD-01.B, VER-232, VER-262): the 74HC165 chain (about $0.95 a unit at 10, freeing about 18 pins)
is the designated pin-relief option, the first thing to change if the pin plan runs short. REQ-PRD-08's statement
changed, so Alex's approval A-011 no longer counts: the row is PROPOSED until he approves the new text. VER-188
cannot catch leftovers until the RP2350B design.py replaces the baseline (DEF-47, VER-188's procedure).

## (c) Where the register is on the new-MCU path

`python3 hardware/vv/tools/check_trace.py` prints it. Today: **NM-01, Requirements** (Alex approved the REQ-PRD
rows in chat, as worded at c3aec22 or at his review of 2026-10-29 (section b2), but nothing is signed in APPROVALS.md), with NM-02, Chip selection, met out of order (next_spin_requirements.md,
'Chosen (Alex, 2026-10-29)'). NM-03, the chip gate, waits on the datasheets on disk (VER-233, its entry need) and
on the two tests: the phone bridge test on the stripboard Pico 2 (VER-251) and XIP from a 16 MB flash on an
RP2350B (VER-252: it needs a dev board with that flash, or the first board). Both cover only
next_spin_requirements.md, so they can PASS while the baseline is still the ESP32-S3 board's files. A fail of
the bridge test stops the board design: online play is a must-have.

## (d) Before the 5-board order

**What "classes 1-15 pass before the order" means:** each cell of classes 1-15 in grid.md is
covered (R9: a check that can clear it has run, or a NOT_NEEDED reason, or an accepted gap), and
each ORDER check in it passes or is ACCEPTED with a signed DF-ACCEPT line (G-ORD-22). An accepted
gap needs both a grid_reasons.csv ACCEPTED_GAP row and Alex's signed GRID-<class>-<subsystem> line.

The order gate holds only what the copper, the BOM and the fab files freeze, or what would make the
five boards useless or unsafe. Every ORDER check is named in a numbered step below (check_trace
fails otherwise). Effort is Claude's unless it says Alex. A carried check's procedure may still name
the ESP32-S3 board's parts: it is re-written for the RP2350B design before it runs (stale_names.py
fails it once that design.py lands without them).

| # | Step | IDs | Effort |
|---|---|---|---|
| 0.1 | **Alex approves the requirements (NM-01)**: REQ-PRD-01 to -19 as he reviewed them in chat (at c3aec22, and on 2026-10-29: section b2) and their numbers (5 of 5 battles, 1.0 s and 10 of 10, 8 channels at 32 kHz with 4 MB for the program and 2 MB of room for the sound and 30 min, $60 at 10 and $40 at 50, 78.9 x 156.48 x 9.6 mm each +0.3 mm); Claude reviews each subsystem's requirements against its functions first. | VER-189 | Claude: half a day; Alex: 1 h |
| 0.2 | **Alex pins a signing key** (README, 'Approvals'). Until then no APPROVALS.md line counts. | | Alex: 15 min |
| 0.3 | **Download** the RP2350 datasheet (its hardware design chapter holds the minimal design's drawing) and the GD25Q128 datasheet into hardware/ref_design/datasheets, with their hashes. | VER-233 | 30 min |
| 1 | **The chip gate (NM-03)**: the phone bridge test on the stripboard Pico 2 (our bridge page in Android Chrome, Showdown's server accepting a battle, 5 Gen 1 and 5 Gen 3); XIP from a 16 MB flash on an RP2350B (Alex orders a dev board with that flash, or it waits for the first board), with a sound sample played from the top of the flash; boot time to the lobby and the real panel on a Pico 2 on the side. A fail of the bridge test stops the board design. | VER-251, VER-252, VER-223, VER-224 | 2-3 days on the bench; Alex: the dev board order, 7 to 14 days' lead |
| 2 | **Architecture and pin plan (NM-04)**: the pin plan and the pin budget (48 GPIOs against every function; the 74HC165 chain is the designated relief), the power budget, the rails (the flash's VCC among them), the flash's budget and read bandwidth, the chip's heat, the hard reset's delay and the boot ROM's recovery rules, the Sleep save's size and write (REQ-SOC-11); the backlight driver, the cell sense and the USB ESD array chosen; the power key's states on paper (seat 1's Start; the slider is dropped). | VER-232, VER-262, VER-231, VER-235, VER-256, VER-238, VER-137, VER-144, VER-193, VER-239, VER-258, VER-259, VER-264 | 2-3 days |
| 3 | **Schematic (NM-05)**: the RP2350B design.py replaces the ESP32-S3 board's on main; the netlist, ERC, the pin gate, the application circuits, the flash's wiring against the minimal design, the datasheet audit, the parts' identity, the watchdog rule; then README's baseline moves to that commit and its baseline board to `rp2350b`. | VER-01, VER-03, VER-02, VER-34, VER-198, VER-253, VER-14, VER-104 | the board session; Claude: 1 day of checks |
| 4 | **The power path and the buses on paper**: inrush, the USB-C connector's fault reach, the amplifier's states and heat, the clocked buses (the panel's and the flash's), the cell's chain, the thermistor's place, the contacts' pull-ups against the pill. | VER-99, VER-195, VER-196, VER-200, VER-199, VER-218, VER-164, VER-248 | 1 day |
| 5 | **Layout and the board against the case (NM-06; the case gate itself is NM-09, after the order)**: DRC, check_fit, placement drift, the XY chain, the MLCC keep-out, the ESTIMATE ledger, the cell's gap, the USB pair, return paths (the QSPI flash's among them), net classes, the contacts and the dome as placed, the panel's tail, the case's architecture, the regulators' switching loops. | VER-04, VER-06, VER-19, VER-174, VER-142, VER-152, VER-181, VER-236, VER-108, VER-21, VER-201, VER-08, VER-162, VER-178, VER-219, VER-190, VER-249 | 2 days |
| 6 | **Fab outputs**: edges, holes, the drill files, the CPL, the fab content, the gerbers read on their own (each exposed pad's paste: DF-219), JLC's dated limits, land patterns (rows for the RP2350B and the GD25Q128, VER-254), the release run. | VER-18, VER-114, VER-131, VER-176, VER-132, VER-27, VER-17, VER-125, VER-254, VER-143 | 1.5 days |
| 7 | **Parts in hand**: the cell's spec sheet and its plug against the cell's and the speaker's headers; the panel's tail and its map on the fit-kit board. | VER-37, VER-184, VER-44, VER-136 | an evening each once parts arrive |
| 8 | **Docs and test access (NM-07)**: bring-up, assembly and the BOM re-written for the RP2350B board; every probe point has copper. | VER-158, VER-265 | 2 hours, after the docs |
| 9 | **The register**: seeded faults, stale names, repository paths. | VER-129, VER-188, VER-220 | 30 min |
| 9.5 | **The coverage grid's gaps** ([grid.md](grid.md), R9): build or run what clears each cell of classes 1-15, or accept the gap (Claude's ACCEPTED_GAP row plus Alex's signed GRID line). | VER-189, VER-190, VER-195, VER-196, VER-236, VER-144, VER-193, VER-238, VER-199, VER-200, VER-218, VER-256 | 1 day; or Alex's acceptances: 30 min |
| 10 | **Reviews** (advisory here): the clean-room review; the DFMEA review binds at G-EVT-16. | VER-40 | Alex: half a day, optional |
| 11 | **Alex's decisions** in APPROVALS.md, in signed commits: one `REQ-ALL@<commit>` line; the H rows only built units can detect, accepted for ORDER; each acceptance as its defect's `DF-ACCEPT:DEF-nn` line; one `GATE@<commit>` line after reading G0's list of moves. | | Alex: 1 h |
| 12 | **Release and order (NM-08)**: the release record; the dry run (valid 7 days); stock on order day (valid 1 day); `check_trace.py --gate order` exits 0; 5 assembled and 5 bare boards. | VER-42, VER-28, VER-41 | 1 day |

Rough total once the dev board arrives: about 3 weeks of Claude's work around the board session's
design, 4-5 hours of Alex's, and the parts' lead times in parallel.

**What moved where at this re-seed** (G0 lists every move for Alex before he signs GATE@):

| Check | Was | Now | Why |
|---|---|---|---|
| VER-206, VER-209, VER-221, VER-222, VER-225, VER-226, VER-229, VER-234, VER-247, VER-250 | ORDER or EVT (NOT_BUILT) | DROPPED | the F1C200s board's GBA, Linux, microSD card and USB host mode; each names its tag |
| VER-221's place at the chip gate | the GBA test on an F1C200s dev board | VER-251 and VER-252 | the phone bridge test and XIP from the 16 MB flash (NM-03) |
| every other check of the F1C200s board's files | NOT_RUN or NOT_BUILT for f1c200s | NOT_RUN or NOT_BUILT for rp2350b | no RP2350B design exists to check yet |

## (e) The case gate, EVT, DVT and PVT

The gates are gates.md's, unchanged in shape: the case gate (NM-09, after the order gate; G-CASE: the stack, the shell, the
screws, the print, the size against today's with the spin's size pass, VER-230), EVT on the 5 boards
(bring-up from a bring-up doc written for the RP2350B board, the cell's safety screens first, then
the new functions: the bring-up image running in place from the flash (VER-237), recovery over the USB boot ROM
(VER-241), online battles through the phone bridge (VER-227), boot time (VER-228), sound from the flash
(VER-255)), DVT on the revised spin, PVT on 30 boards. Exit for each:
`check_trace.py --gate <case|evt|dvt|pvt>` exits 0.

## (f) Decisions for Alex

1. **Sign the REQ-PRD approvals** you gave in chat (at c3aec22 and on 2026-10-29, section b2) in
   APPROVALS.md. REQ-PRD-09 is yours: $60 or less at 10 units and $40 or less at 50 (Claude's estimate about
   $58 and about $38.50, the TPS3421 included); VER-117 re-quotes it with the tariffs and every shipping leg.
   Also read DF-263: block protection on the bootloader against a recovery .uf2 that rewrites it.
2. **Order the dev board for the chip gate's XIP test** (VER-252): an RP2350B with 16 MB of QSPI
   flash (Pimoroni's Pico Plus 2 is one, to confirm). The phone bridge test (VER-251) needs only the
   stripboard Pico 2 and an Android phone. Proposed: run VER-251 first, since a fail stops the design.
3. **The power key**: decided by Alex on 2026-10-29: seat 1's Start, held 2.5 s to turn on
   (DESIGN.md:60); the DS Lite slider is dropped. Proposed: work the power path's states on paper (VER-239).
4. **The case material** before any sale (DF-247). Proposed: decide at PVT.

The ESP32-S3 board's decisions (DEC-40 to DEC-49), the RP2354B board's (DEC-01 to DEC-39) and the
F1C200s board's (the V3s fallback, queue issue #11's dev boards) are at their tags or moot; none
carries, since each named that board's parts.

## (h) What this plan still cannot catch

- The RP2350's and the GD25Q128's numbers are from memory until their datasheets and the minimal
  design's drawing are on disk (VER-233): the rails, the pin table, the QSPI pins, the flash's ID and
  timing. Every check that reads them names rp2350.pdf or gd25q128.pdf in 'sources', so a PASS before
  the download is BLOCKED.
- The minimal design's exact nets, values and the BOOTSEL pad are not on disk: VER-253 and VER-254
  compare the board with them only once VER-233 has put them there. The 2 mm QSPI limit is Claude's proposal;
  the sound's 4 MB, 2 MB and 30 min are Alex's. The TPS3421's delay variant and the boot ROM's A/B slot rule,
  its handling of a whole-flash UF2 and the block protection against a recovery file are not read yet (VER-258, VER-259).
- Whether Showdown's server accepts a battle from our bridge page (VER-251) is shown only by the
  test; Chrome's WebUSB behaviour on the phones Alex has is not known until it runs (DF-254).
- A carried check whose procedure still names the ESP32-S3 board's parts looks built; its result is
  NOT_RUN until it runs on the RP2350B design, and stale_names.py fails it if that design lacks the
  parts it names.
- The register cannot see a chip decision made outside docs/static or a proposal doc; R8 sees only
  docs/living/*proposal*.md.

## (i) The gate today

**Run at 816179a (2026-10-29), after the RP2350B re-seed.** Default mode: STRUCTURE OK, 4 warnings (the
DMM pass bands), the register at NM-01 (NM-02 met out of order). seed_faults.py: 148 of 148 seeded faults
caught (its seeds now point at rows of the RP2350B board). run_checks.py ran the register's own checks only:
VER-129, VER-188 and VER-220 PASS at 0bc51d6; DEF-35 and DEF-52 went FIXED while VER-129 was stale and are
CLOSED again on its PASS. It ran no RP2350B check: the seven that may run before the RP2350B design
(VER-189, VER-223, VER-224, VER-233, VER-251, VER-252, VER-256) are NOT_BUILT or NOT_RUN, and every other one reads the
dropped board's files. Counts: requirements 177 (129 PROPOSED, 40 DROPPED, 8 RETIRED); checks 255 (3
PASS, 58 NOT_RUN, 130 NOT_BUILT, 35 DROPPED, 24 RETIRED, 5 CUT); DFMEA rows 261 (51 live, 26 of them H);
defects 102 (85 DROPPED, 13 RETIRED, 2 CLOSED, 1 FIXED, 1 OPEN). The grid: 125 cells of classes 1-15 planned,
0 bare GAP, 10 NOT_NEEDED. repo_paths.py: 0 moved, deleted or wrongly pinned.

Order gate: FAIL, 25 of 29 not met (G-ORD-24, R5, R6 and R8 pass). It fails as it should: there is no
RP2350B design yet, so every RP2350B check is unrun (G-ORD-22, R2, R9), R7 fails on the dropped board's
baseline, the chip gate has not run (G-ORD-28: VER-251 and VER-252), no requirement is approved (R1) and
DEF-47 is open (R4). The EVT, DVT and PVT gates are not evaluated.

## (k) The coverage grid

[grid.md](grid.md) is generated by tools/grid.py from verification.csv, grid_reasons.csv and each
requirement's subsystems: classes 1-18 by the nine RP2350B subsystems, each cell COVERED with its
checks and worst status, planned, GAP or NOT_NEEDED, and a table of the requirements per subsystem.
At the order gate R9 holds every cell of classes 1-15 that no built pre-order check has cleared:
today every cell but the NOT_NEEDED ones, since no RP2350B design exists to run a check on.

Last updated: 2026-10-29 13:10:42 EDT (the artifact audit applied, branch vv-audit-apply)
