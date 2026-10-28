# The plan: from here to 5 boards, then 30

**The register targets the next spin: the Allwinner F1C200s board** (docs/static/DESIGN.md:77 and
docs/static/next_spin_requirements.md, Alex, 2026-10-28): the F1C200s (QFN-88, 64 MB of DRAM in the
package), microSD only, the FUSB303B for the USB-C roles, today's ILI9488 panel on its 8-bit 8080
bus, no Wi-Fi; the bare-metal battler boots by default and a held button boots Linux (GBA through
gpSP, online play as a Showdown guest, GBA link play over USB-C). The V3s is the fallback if a dev
board cannot hold FireRed at about 60 fps. There is no F1C200s schematic or layout yet: the KiCad
files on main are still the ESP32-S3 board's, so every F1C200s check is NOT_RUN or NOT_BUILT and
the design baseline names the dropped board (README, '- Baseline board: `esp32s3`').

**Both earlier boards are dropped.** The ESP32-S3 board's register, with every round's ledger, its
decisions DEC-40 to DEC-49 and its order checklist, is at tag `vv-esp32s3-final` (c556556); its last
results after main's path moves are at 30131db. The RP2354B board's is at tag `vv-rp2354b-final`
(c8ac9dd). Their rows here are DROPPED (README, Rules): neither PASS nor CLOSED, out of every gate of
the next spin, each naming its board and tag. HISTORY.md keeps the earlier rounds.

Contents: (a) the system on one page; (b) what the re-target changed; (c) where the register is on
the new-MCU path; (d) before the 5-board order; (e) the case gate, EVT, DVT and PVT; (f) decisions
for Alex; (h) what this plan still cannot catch; (i) the gate today; (k) the coverage grid.

## (a) The system on one page

Requirements (requirements.md, each allocated to the F1C200s board's subsystems) -> DFMEA rows
(dfmea.csv) -> checks (verification.csv) -> the new-MCU path and the phase gates (gates.md) ->
release and change records (releases/) -> defects (defects.md). Alex's decisions live in one file,
releases/APPROVALS.md, which only he writes lines in, in commits signed with the key README pins.
`tools/check_trace.py` enforces the chain; its docstring and README's Rules list every rule.
`tools/seed_faults.py` proves each rule can fail (VER-129). `tools/grid.py` writes the coverage grid.

## (b) What the re-target changed (2026-10-28)

| What | Register effect |
|---|---|
| The ESP32-S3 board dropped (its files still on main) | 85 defects, 120 DFMEA rows, 25 checks, 18 requirements and every REQ-MCU and REQ-RF row DROPPED with esp32s3; the live checks of its design files reset to NOT_RUN (the tool exists) or NOT_BUILT (it reads that board's circuit, or its bench step is in that board's bring-up doc), their last results at 30131db |
| The F1C200s board, the next spin | board `f1c200s`; subsystems power, cell, soc, storage, usbc, display, audio, buttons, case (grid.md); every live requirement names its subsystems; `.F` allocations (the `.R` ones were the ESP32-S3 board's) |
| DESIGN.md's goals for the spin | REQ-PRD-12 to -17 (GBA at full speed, link play over USB-C, Showdown as a guest, a match within seconds, games and saves over USB or SD, today's size or smaller), each quoting its DESIGN.md line; REQ-SOC-01 to -08, REQ-USB-01, -02, REQ-PWR-20 to -22, REQ-BTN-10 |
| The carryover lessons (carryover.md) | DF-213 to DF-247 on the F1C200s board, with the checks that carry them; the new chip's risks DF-203 to DF-212 |
| The new-MCU path | gates.md NM-01 to NM-10; VER-221 to VER-225 the chip gate on a dev board (queue issue #11); check_trace prints the stage |

## (c) Where the register is on the new-MCU path

`python3 hardware/vv/tools/check_trace.py` prints it. Today: **NM-01, Requirements** (Alex has not
approved REQ-PRD), with NM-02, Chip selection, met out of order (queue issue #4, the 'Chosen' table
of next_spin_requirements.md). NM-03, the chip gate, waits on Alex's dev board order (issue #11). A
FAIL there sends the project to the V3s: the board becomes `v3s` and the chip's requirements and
checks are re-written for it before NM-04.

## (d) Before the 5-board order

**What "classes 1-15 pass before the order" means:** each cell of classes 1-15 in grid.md is
covered (R9: a check that can clear it has run, or a NOT_NEEDED reason, or an accepted gap), and
each ORDER check in it passes or is ACCEPTED with a signed DF-ACCEPT line (G-ORD-22). An accepted
gap needs both a grid_reasons.csv ACCEPTED_GAP row and Alex's signed GRID-<class>-<subsystem> line.

The order gate holds only what the copper, the BOM and the fab files freeze, or what would make the
five boards useless or unsafe. Every ORDER check is named in a numbered step below (check_trace
fails otherwise). Effort is Claude's unless it says Alex. A carried check's procedure may still name
the ESP32-S3 board's parts: it is re-written for the F1C200s design before it runs (stale_names.py
fails it once that design.py lands without them).

| # | Step | IDs | Effort |
|---|---|---|---|
| 0.1 | **Alex approves the requirements (NM-01)**: REQ-PRD-12 to -17 and their proposed numbers (59.7 fps, 30 min linked, 5 of 5 battles, 1.0 s and 10 s, 60 s a 16 MB copy, 78.9 x 156.48 x 9.6 mm); Claude reviews each subsystem's requirements against its functions first. | VER-189 | Claude: half a day; Alex: 1 h |
| 0.2 | **Alex pins a signing key** (README, 'Approvals'). Until then no APPROVALS.md line counts. | | Alex: 15 min |
| 0.3 | **Download** the F1C200s datasheet and user manual and the FUSB303B datasheet into hardware/ref_design/datasheets, with their hashes. | VER-233 | 30 min |
| 1 | **The chip gate (NM-03), queue issue #11**: Alex orders two F1C200s dev boards and two FUSB303B breakouts; then FireRed under gpSP, boot from the card with the image's own DRAM init, boot time, the panel on the 8080 bus, the USB-C roles (and a gpSP link over USB between the two boards, DF-243). A FAIL: the V3s. | VER-221, VER-222, VER-223, VER-224, VER-225 | Alex: the order, 14 days' lead; then 3-5 days on the bench |
| 2 | **Architecture and pin plan (NM-04)**: the pin plan, the power budget, the rails and their order, the FUSB303B and the VBUS switch, the chip's heat; the backlight driver, the cell sense and the USB ESD array chosen; the power slider measured against a board switch. | VER-232, VER-231, VER-235, VER-234, VER-238, VER-137, VER-144, VER-193, VER-239 | 2-3 days |
| 3 | **Schematic (NM-05)**: the F1C200s design.py replaces the ESP32-S3 board's on main; the netlist, ERC, the pin gate, the application circuits, the datasheet audit, the parts' identity, the watchdog rule; then README's baseline moves to that commit and its baseline board to `f1c200s`. | VER-01, VER-03, VER-02, VER-34, VER-198, VER-14, VER-104 | the board session; Claude: 1 day of checks |
| 4 | **The power path and the buses on paper**: inrush, the USB-C fault reach, the amplifier's states and heat, the clocked buses, the cell's chain, the thermistor's place, the contacts' pull-ups against the pill. | VER-99, VER-195, VER-196, VER-200, VER-199, VER-218, VER-164, VER-248 | 1 day |
| 5 | **Layout and the board against the case (NM-06)**: DRC, check_fit, placement drift, the XY chain, the MLCC keep-out, the ESTIMATE ledger, the cell's gap, the USB pair (high speed on two layers), return paths, net classes, the contacts and the dome as placed, the panel's tail, the case's architecture, the regulators' switching loops. | VER-04, VER-06, VER-19, VER-174, VER-142, VER-152, VER-181, VER-236, VER-108, VER-21, VER-201, VER-08, VER-162, VER-178, VER-219, VER-190, VER-249 | 2 days |
| 6 | **Fab outputs**: edges, holes, the drill files, the CPL, the fab content, the gerbers read on their own (each exposed pad's paste: DF-219), JLC's dated limits, land patterns (rows for the F1C200s and the FUSB303B), the release run. | VER-18, VER-114, VER-131, VER-176, VER-132, VER-27, VER-17, VER-125, VER-143 | 1.5 days |
| 7 | **Parts in hand**: the cell's spec sheet and its plug against the cell's and the speaker's headers; the panel's tail and its map on the fit-kit board. | VER-37, VER-184, VER-44, VER-136 | an evening each once parts arrive |
| 8 | **Docs and test access (NM-07)**: bring-up, assembly and the BOM re-written for the F1C200s board; every probe point has copper. | VER-158 | 2 hours, after the docs |
| 9 | **The register**: seeded faults, stale names, repository paths. | VER-129, VER-188, VER-220 | 30 min |
| 9.5 | **The coverage grid's gaps** ([grid.md](grid.md), R9): build or run what clears each cell of classes 1-15, or accept the gap (Claude's ACCEPTED_GAP row plus Alex's signed GRID line). | VER-189, VER-190, VER-195, VER-196, VER-236, VER-144, VER-193, VER-238, VER-199, VER-200, VER-218 | 1 day; or Alex's acceptances: 30 min |
| 10 | **Reviews** (advisory here): the clean-room review; the DFMEA review binds at G-EVT-16. | VER-40 | Alex: half a day, optional |
| 11 | **Alex's decisions** in APPROVALS.md, in signed commits: one `REQ-ALL@<commit>` line; the H rows only built units can detect, accepted for ORDER; each acceptance as its defect's `DF-ACCEPT:DEF-nn` line; one `GATE@<commit>` line after reading G0's list of moves. | | Alex: 1 h |
| 12 | **Release and order (NM-08)**: the release record; the dry run (valid 7 days); stock on order day (valid 1 day); `check_trace.py --gate order` exits 0; 5 assembled and 5 bare boards. | VER-42, VER-28, VER-41 | 1 day |

Rough total once the dev boards arrive: about 3 weeks of Claude's work around the board session's
design, 4-5 hours of Alex's, and the parts' lead times in parallel.

**What moved where at this re-seed** (G0 lists every move for Alex before he signs GATE@):

| Check | Was | Now | Why |
|---|---|---|---|
| VER-33, VER-43, VER-55, VER-57, VER-72, VER-73, VER-77, VER-124, VER-179, VER-185, VER-191, VER-194, VER-203 to VER-205, VER-207, VER-208, VER-210 to VER-217 | ORDER to SALE | DROPPED | the ESP32-S3 board's parts (the module, its radio, its latch and power key, its chain, its flash); each names its successor |
| every other check of the ESP32-S3 board's files | PASS, FAIL or NOT_RUN | NOT_RUN or NOT_BUILT for f1c200s | no F1C200s design exists to check yet; the results are at 30131db |

## (e) The case gate, EVT, DVT and PVT

The gates are gates.md's, unchanged in shape: the case gate (G-CASE: the stack, the shell, the
screws, the print, the size against today's with the spin's size pass, VER-230), EVT on the 5 boards
(bring-up from a bring-up doc written for the F1C200s board, the cell's safety screens first, then
the new functions: FireRed on unit 1 (VER-247), link play (VER-226), Showdown (VER-227), boot time
(VER-228), games and saves (VER-229)), DVT on the revised spin, PVT on 30 boards. Exit for each:
`check_trace.py --gate <case|evt|dvt|pvt>` exits 0.

## (f) Decisions for Alex

1. **Approve or restate the new requirements** (REQ-PRD-12 to -17 first; every number marked
   'Claude's proposal' is his to set). Proposed: approve, with the numbers as written.
2. **Order the dev boards for the chip gate** (issue #11): two boards, so the USB-C roles and the
   link can be tested between them. Proposed: two, plus two FUSB303B breakouts.
3. **The power key**: the DS Lite slider if a board switch fits it, else seat 1's Start (DESIGN.md
   says which). Proposed: measure the slider first (VER-239).
4. **The case material** before any sale (DF-247). Proposed: decide at PVT.

The ESP32-S3 board's decisions (DEC-40 to DEC-49) and the RP2354B board's (DEC-01 to DEC-39) are
at their tags; none carries, since each named that board's parts.

## (h) What this plan still cannot catch

- The F1C200s's numbers are from memory until its datasheet and user manual are on disk (VER-233):
  the rails, the pin-mux, the TCON's CPU mode, the boot order and the ADCs. Every check that reads
  them names f1c200s.pdf in 'sources', so a PASS before the download is BLOCKED.
- Whether gpSP's GBA link runs over a USB cable at all (next_spin_requirements.md: 'not yet
  checked'): only two dev boards or two units show it (DF-243).
- A carried check whose procedure still names the ESP32-S3 board's parts looks built; its result is
  NOT_RUN until it runs on the F1C200s design, and stale_names.py fails it if that design lacks the
  parts it names.
- The register cannot see a chip decision made outside docs/static or a proposal doc; R8 sees only
  docs/living/*proposal*.md.

## (i) The gate today

Filled in by the run at the end of this re-seed (below).

## (k) The coverage grid

[grid.md](grid.md) is generated by tools/grid.py from verification.csv, grid_reasons.csv and each
requirement's subsystems: classes 1-18 by the nine F1C200s subsystems, each cell COVERED with its
checks and worst status, planned, GAP or NOT_NEEDED, and a table of the requirements per subsystem.
At the order gate R9 holds every cell of classes 1-15 that no built pre-order check has cleared:
today every cell but the NOT_NEEDED ones, since no F1C200s design exists to run a check on.

Last updated: 2026-10-28 (step 2 of the F1C200s re-target)
