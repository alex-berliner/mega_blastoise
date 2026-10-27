# Verification and validation

How we know a board works before and after we order it. [PLAN.md](PLAN.md) is the single
plan: what must be true before the order, then EVT, DVT and PVT. This page is the system.
It covers both boards: `hardware/ref_design` (REF-A, Claude's) and `hardware/prod_board` with
`hardware/prod_case` (Alex's). Each board supplies its own evidence. Nothing here edits either
board.

"Works" means all of: built as designed (verified), the right design for the numbers
(validated), safe (the Li-ion cell, touch temperature), made by JLC without surprises, fits
the case, survives use.

## The files

| File | What | ID |
|---|---|---|
| [PLAN.md](PLAN.md) | the single plan: the order checklist, the phase exits, the scripts to write, the past bugs, Alex's decisions | |
| [requirements.md](requirements.md) | what the device must do, each a number with a tolerance, a reason, a source and the checks that prove it; split per board where a board's parts or numbers enter | REQ-<AREA>-NN, REQ-<AREA>-NN.R / .P |
| [dfmea.csv](dfmea.csv) | how each part, interface and assembly step can fail, how bad that is, what prevents it, what detects it ('detects' names the checks that can see the failure) | DF-NN |
| [verification.csv](verification.csv) | every check: procedure, pass limit, what it covers, last result, evidence, commit, date, phase, how long a result stays valid, the instrument and its accuracy, what still has to be bought, and the check classes and subsystems it targets | VER-NN |
| [grid.md](grid.md) | the coverage grid, generated (never typed): check classes 1-18 by subsystem, each cell COVERED with its checks and worst status, planned, GAP or NOT_NEEDED; the gaps with their proposed checks; the FAILs | |
| [grid_reasons.csv](grid_reasons.csv) | the cells that need no check: class, subsystem, NOT_NEEDED (or ACCEPTED_GAP, which counts only once Alex signs GRID-<class>-<subsystem>) and why | |
| [vp_map.csv](vp_map.csv) | a one-time map of the archived validation_plan.md's 218 methods to IDs, or CUT with a reason | plan #n |
| [gates.md](gates.md) | the order, case, EVT, DVT and PVT gates, each criterion naming IDs and a board; the unit allocation; proposed waivers | G-ORD-NN, G-CASE-NN, G-EVT-NN, G-DVT-NN, G-PVT-NN, W-NN |
| [defects.md](defects.md) | the corrective-action log (design defects; per-unit findings go in evidence/findings.csv) | DEF-NN |
| [releases/APPROVALS.md](releases/APPROVALS.md) | Alex's decisions: approvals, acceptances (with the last phase they hold for) and waivers | A-NNN |
| [releases/](releases/) | release and change records, from the two templates | REL-<BOARD>-R<rev>-<PHASE>, CHG-NN |
| [evidence/](evidence/) | the saved output of each check run, named `<date>_<VER>_<check>.txt`; units.csv and findings.csv for built units | |
| [tools/check_trace.py](tools/check_trace.py) | the trace gate (Python 3 standard library) | |
| [tools/run_checks.py](tools/run_checks.py) | runs the quick scripted checks, writes their evidence with a header and records the result | |
| [tools/seed_faults.py](tools/seed_faults.py) | proves check_trace can fail: seeds one fault at a time into a copy, and the faults that need git history into a scratch clone (VER-129) | |
| [tools/design_numbers.py](tools/design_numbers.py) | recomputes the pre-order numbers from the design files (inrush, backlight, cutoff, screws per pilot, MLCC keep-out, TH1, the cell's gap on every side, VBUS fault reach, BOM coverage, holes, swelling, copper and silk against the routed edges, the cell's plug against J4, the charge window TH1 gives against U2's TS limits) and prints two bench numbers (the TS band at an ambient, and the CV load that holds the charge without terminating: there is no resistive window); the CPL and drill checks run fab.py's own export into a temp dir and read what it writes | |
| [tools/grid.py](tools/grid.py) | writes grid.md from verification.csv and grid_reasons.csv; check_trace's R9 reads the same grid | |
| [tools/stale_names.py](tools/stale_names.py) | fails on a live row naming a part, part number or net that design.py at the baseline no longer has (VER-188); check_trace's default mode runs it | |
| [tools/datasheet_hashes.py](tools/datasheet_hashes.py) | records each datasheet extract's sha256, byte size and the drawing's date under evidence/, so the numbers this register quotes can be checked without committing the extracts themselves | |

## How they link

```
requirement  --Verified by-->  check (VER)  <--detects--  DFMEA row
  (parent --allocated--> .R / .P)  |  ^                        ^
     ^                             |  +-- defect: "check added or changed"
     +---- gate criterion ---------+--------------------------+
Alex's decisions (APPROVALS.md) --> requirement status, DFMEA acceptance, waiver
```

A requirement is proven by its checks. A risk is caught by its 'detects' checks. A defect
names the check that now catches it. A gate names the checks, requirements, risks and
defects it needs. Every ID written anywhere in these files must exist, or the trace gate fails.

## Running the gate

```sh
python3 hardware/vv/tools/check_trace.py                  # structure (exit 1 on an error) + report
python3 hardware/vv/tools/check_trace.py --gate order     # exit 1 unless the order gate is met
python3 hardware/vv/tools/check_trace.py --gate order --accept-software-key   # only if Alex pinned a software key; the gate says so
python3 hardware/vv/tools/check_trace.py --gate case      # the printed-case gate
python3 hardware/vv/tools/check_trace.py --gate evt --board prod_board
python3 hardware/vv/tools/check_trace.py --shopping --gate evt   # what is still to buy, longest lead first
python3 hardware/vv/tools/run_checks.py                   # re-run the quick scripted checks, save evidence
python3 hardware/vv/tools/seed_faults.py                  # prove the trace gate fails on seeded faults
python3 hardware/vv/tools/grid.py                         # rewrite grid.md from the register (--check: is it current?)
python3 hardware/vv/tools/stale_names.py                  # live rows naming a part or net design.py lost
python3 hardware/vv/tools/design_numbers.py --list        # the scripted pre-order numbers
```

Default mode fails only on structure; check_trace.py's docstring lists every rule. A design
baseline that differs from HEAD, or from the baseline commit's own blobs, is a structure error
(DEF-47, F-R3): the register no longer describes the design until its rows are re-checked and
the baseline updated. Then it
reports what is not yet true: requirements with no check, H-priority risks whose 'detects'
checks have not passed, checks that are FAIL, NOT_RUN or NOT_BUILT, stale evidence, parts no
DFMEA row names, and warnings (a pass band narrower than 4 x its instrument's accuracy, a
'needs' entry with no lead time, an approval not yet counted, evidence not yet committed, a
design commit on the order branch after the baseline).

## Passing a gate

1. Alex decides the requirements in releases/APPROVALS.md (APPROVED, REJECTED, or DEFERRED
   with a reason and the last phase it holds for) and copies the decision into each Status.
2. Run every check the gate needs, save its output under `evidence/` with the header below,
   and record PASS or FAIL, the evidence path, the commit and the date in verification.csv.
   `run_checks.py` does this for the quick scripted checks.
3. Each H-priority DFMEA row is acted on: take the action, re-rate S, O and D, and mark it
   DONE once every one of its 'detects' checks passes; or Alex accepts it in APPROVALS.md
   for a phase. An acceptance holds only up to its phase.
4. Fix or close the defects the gate asks for.
5. Run `check_trace.py --gate <phase>` until it exits 0, paste its output into the release
   record, tag the commit, then order.

Phases: ORDER (before the 5-board EVT order: what the copper and the BOM freeze), CASE
(before the printed case is released for the EVT units), EVT (the 5 boards), DVT (the
revised spin), PVT (the 30-board run), SALE (before any unit is sold or given away).

## Rules

- **Only Alex decides.** Requirements stay PROPOSED, risks unaccepted and waivers unsigned
  until he commits a line in releases/APPROVALS.md in a commit signed with his key (Approvals,
  below). Every commit in this repository has the same author, so authorship proves nothing
  (O-R4), and a good signature alone proves only some key in a user-global allowed-signers
  file that any process running as alex can edit (round 2's objection). check_trace counts a
  line only when its blame commit is signed (`%G?` = `G`) by the one key this README pins
  (`%GF` equal to the '- Signing key:' line below), and that pin line was itself committed in a
  commit signed by the same key; a line from an unsigned commit, another key, or anywhere the
  key is not pinned is pending; a line from a merge or a commit with a Claude trailer is an
  error; an approved statement edited since needs approving again. A bulk line
  ('REQ-ALL@<commit>' APPROVED, 'DF-H@<commit>' ACCEPTED) covers every requirement, or every
  H row, unchanged since that commit, each checked on its own.
- **No defect closes without a new or changed check.** CLOSED needs a VER in "Check added
  or changed" that passes on fresh evidence. A design fix with no check is FIXED, not CLOSED.
- **Evidence is a saved output, not a typed word.** A PASS or FAIL names an evidence file
  under evidence/ with a header: `COMMIT: <hash>` and a last `RESULT: PASS|FAIL` equal to the
  row; an analysis (A) or a test (T) adds `TOOLS: <versions>`; an inspection or demonstration
  (I, D) adds `WHO: <reviewer>` and `ARTEFACT: <what was inspected: a screenshot folder with
  sha256s, a spec sheet's sha256, a dated page>` (O-R3); a bench result adds `RELEASE: <REL
  id>`, `UNITS: <serials>` and the release's fab zip sha256. Only an I or D check with no
  release may point at a design file instead, and only one its own covers name (VER-104,
  VER-124). Evidence counts at a gate only when it is committed and unchanged since, and the
  gate fails while anything under hardware/vv is uncommitted (R5, O-R2). A commit message is
  never evidence.
- **Evidence goes stale when what it covers changes.** A check's 'covers' lists globs
  (`path::regex` limits it to changed lines matching the regex as whole words), parts
  (`part:R52,R53,C10-C22`: stale when a listed part's value, LCSC number, footprint or pins
  change in design.py), or a release (`release:REL-...`: a bench result on built units,
  stale only when a later change record names the check under "What to re-run"; check_trace
  refuses a change record that omits a bench test whose DFMEA row or requirement names a part
  the recorded commits changed in design.py, O-R21). It is also
  stale when its commit is not an ancestor of HEAD, or when it is older than its valid_days
  (stock, quotes, the dry run). Re-run it; never edit the commit field to make it fresh. At
  DVT and PVT a check of phase EVT or later that is a test or covers a release, whatever its
  method, counts only if it covers a release record of that phase: the revised spin is
  re-tested and re-inspected, not vouched for by the EVT units (O-R11, F-R1). A 'covers' glob
  matches only files committed to git.
- **A deferral ends.** DEFERRED names the last phase it holds for; after it the requirement
  is PROPOSED again at the gate (R1).
- **Retired rows keep their IDs.** A requirement, DFMEA row, check or defect whose subject left
  the design is RETIRED with 'RETIRED in <commit>: <why>'; no live row may lean on a retired
  check, and no live row's text may cite a RETIRED requirement, row or check except inside a
  'Was: ...' clause (O-R9, F-R14). RETIRED is not a way to close a defect whose part is still on
  the board.
- **A defect names its gate.** Each defect's `Gate:` is the first gate it must be fixed by; R4
  counts it there and at every later gate, so a docs or case defect does not hold the order back
  (O-R5).
- **Advisory checks stay advisory.** A check that only ADVISORY criteria name is printed but
  not counted by PHASE:<p> (VER-40). One check can be advisory on its own: a criterion's Needs
  cell may name it as `ADVISORY:VER-nn`, which makes that one check advisory at that gate and
  binding at the gate it moved to, while the criterion's other checks stay binding.
- **Alex freezes the gate state.** A signed `GATE@<commit>` line in releases/APPROVALS.md
  freezes every check's phase, every defect's Gate, every criterion's Needs and every
  requirement's Phase and Verified-by as they stand at that commit. The newest signed line
  wins, and anything moved later than it fails the order gate (the gate result G1). Until
  Alex signs one, the register's own phases are in force.
- **The order gate wants a hardware key.** The pinned key's type in the allowed-signers file
  must be `sk-ssh-ed25519@openssh.com` or `sk-ecdsa-sha2-nistp256@openssh.com`, or the gate
  is run with `--accept-software-key`, whose use is printed in the gate result and has to be
  copied into the release record's signing section. No line in APPROVALS.md can grant the
  exception (the gate result K1).
- **A case commit may not move the board.** R8 at the order gate compares
  `case/positions.json`, regenerated by case/export.py at the commit on the order branch, with
  the one regenerated at the baseline commit. Both sha256s go into the gate output and the
  release record, and they must match.
- **The baseline is checked against the baseline commit too.** The default mode fails when a
  design path's hash in README differs from HEAD's blob for that path, or from the blob at the
  baseline commit (`be437f8`), so a baseline rewritten from HEAD cannot hide a design edit.
- **Detection is only what can see the failure.** A DFMEA row's 'detects' lists the checks
  that must all pass before it is DONE. Its detection rating is 10 with none, and 8 or more
  while every one is NOT_BUILT. An occurrence of 3 needs `Similar: <the board and part>` in its
  prevention (O-R8); 2 or less needs a passing test on built hardware; 1 needs `Excluded:` and
  the reason in its prevention.
- **Every check says what it covers in the grid.** A live check names its classes (1-20) and
  subsystems (power, cell, mcu, display, audio, buttons, usb_debug, case; 'register' for 19 and
  20), tagged by what its procedure and pass limit really target. A cell no check covers needs a
  NOT_NEEDED reason in grid_reasons.csv or a NOT_BUILT check, and the order gate (R9) holds every
  cell of classes 1-15 until a built check covers it (2026-10-27).
- **Nothing hides behind a later gate.** Every check is reached by a gate criterion, a
  requirement or an H-priority risk no later than its own phase.
- **A check that can be a script is a script that can fail.** Show it failing once (VER-22,
  VER-129).
- **A number quoted in a doc needs its script in the repo**, or it is NOT_BUILT (DEF-28).
- **Every design change gets a change record.** Once a release record exists, a commit that
  touches the design without one fails every gate (R6). The design is design.py,
  placement.py, board/lib/, battler.kicad_pcb, battler.kicad_pro, the schematics, the
  board/tools tree, case.scad and case/export.py: the copper, its rules and the scripts that
  make the fab files, not only the data. A BOM-only change bumps BOARD_REV (on the silkscreen).
  A change to the design files also updates the design baseline below.
- **Each board supplies its own evidence.** A check row holds one result, so a test of
  built hardware belongs to one board. Mark a check `both` only when one result serves both
  boards and it covers each board's files (or a shared file under hardware/vv or
  hardware/membrane). A product requirement that names one board's parts is split into
  .R and .P allocations; prod_board's gate reads only its own (REQ-PBD-* and the .P ones
  Alex writes).
- prod_board and prod_case are Alex's: read them, never write them. KiCad files are never
  written by hand; ref_design's are generated from design.py.

## Approvals: signing with Alex's key

check_trace counts an APPROVALS.md line only from a commit signed with the key pinned below.
Use a key a Claude session running as alex cannot use: an ed25519-sk key on a security key
that wants a touch (best), or a passphrase key that is never loaded into an agent while a
Claude session runs. Not ~/.ssh/id_ed25519 without a passphrase: any process running as alex
can read it. One-time setup in this worktree (SSH signing):

```sh
ssh-keygen -t ed25519-sk -f ~/.ssh/alex_sign_sk          # a touch per signature (or: -t ed25519 with a passphrase)
git config gpg.format ssh
git config user.signingkey ~/.ssh/alex_sign_sk.pub
echo "<Alex's commit e-mail> namespaces=\"git\" $(cat ~/.ssh/alex_sign_sk.pub)" >> ~/.config/git/allowed_signers
git config gpg.ssh.allowedSignersFile ~/.config/git/allowed_signers
ssh-keygen -lf ~/.ssh/alex_sign_sk.pub                   # its SHA256:... fingerprint
# add the line "- Signing key: `SHA256:...`" under 'Pinned key' below, then:
git commit -S -m "vv: pin Alex's signing key" hardware/vv/README.md
git commit -S -m "approvals: ..." hardware/vv/releases/APPROVALS.md   # one signed commit per decision set
git log -1 --format='%G? %GF'                            # G and the pinned fingerprint: check_trace counts it
```

Pinned key (Alex writes the line, in a commit signed with that key; the order gate fails while
it is a software key unless run with --accept-software-key, K1):

Until a key is pinned every line is pending, and the report says so. Claude never signs,
never pins a key, and never writes a line in APPROVALS.md's table (it may correct the
header's wording, as round 2 asked).

## Design baseline

The design files this register describes, as git object hashes (`git rev-parse
HEAD:<path>`; board/lib and board/tools are trees). The default mode fails when any differs
from HEAD, and every ref_design gate fails with "gate evaluated against a design it does not
describe" (the order gate reads the board's files, the case gate case.scad and export.py,
later gates all). Re-seeded on 2026-10-26 at be437f8 (branch ref-design-35in), the board Alex
is ordering: the board itself is final at 62b01f2, the later commits are docs and print kits.
The earlier seeds are kept on branches vv-plan-684a609 (2026-10-25, ref-design-island-45) and
vv-plan-76955d2 (2026-10-24). The second review (F-R13) added the schematics,
battler.kicad_pro (its DRC rules, which VER-04 relies on), the whole board/tools tree (the
route, silk and fab scripts and last_route.ses; cost.py and the like churn it too, accepted)
and case/export.py (it writes case/positions.json, which placement.py reads; positions.json is
gitignored, so its sha256 goes in the release record instead).

The order branch below is read by check_trace (O-R13): a commit on it after the baseline commit
that touches any of these paths makes the default mode warn and the order gate fail (R8), even
before it is merged here. Every hash below equals the same path at be437f8.

- Order branch: `ref-design-35in` from `be437f8`
- `hardware/ref_design/board/design.py` 1b2ce08a3f966375a9babc9777abff73fc01f89e
- `hardware/ref_design/board/placement.py` 65f219bc34795b8ec697a86203f5db8ff88dae12
- `hardware/ref_design/board/lib` 50f371cbf7df2fae7db3746b134434297c38a4fa
- `hardware/ref_design/board/battler.kicad_pcb` 1094c8d8347db6ffee35c37c48f0d12fdf35b909
- `hardware/ref_design/board/battler.kicad_pro` 3633f0dd64d37be8ed692d95dfb9c3d86589e6bc
- `hardware/ref_design/board/tools` 7ace29d2c562703c2fe97ec908c84432f2f1212a
- `hardware/ref_design/board/audio.kicad_sch` c26fffa4213c87a1ae70dd9cef9354229cdfba03
- `hardware/ref_design/board/battler.kicad_sch` d5ed7b835ef3668f9be177a601270a0c16692406
- `hardware/ref_design/board/buttons.kicad_sch` 058ae4c0a8e9039fda0591dcb321200670296dc5
- `hardware/ref_design/board/debug.kicad_sch` 967425f19e2c54b7737201fafd07f60db1b25670
- `hardware/ref_design/board/display.kicad_sch` 22130b7609cffdbba4cb541199a447416b37ca45
- `hardware/ref_design/board/mcu.kicad_sch` 080a7e2143d4b999192fa458f4c550112e933c52
- `hardware/ref_design/board/mcu_power.kicad_sch` b994e45467e2b21e4fd9085c86b90b034d78c351
- `hardware/ref_design/board/power.kicad_sch` fe37406611536d354edfc64c0210c89698bad0df
- `hardware/ref_design/board/usb_charger.kicad_sch` 48d861d4ac36d8852018858b4d54e1e4229fb958
- `hardware/ref_design/case/case.scad` af69698eb5f95d7cb61e4fb9bc8dbe32b3d0ed55
- `hardware/ref_design/case/export.py` 41fdd1f0834187e938832421a83506032ccec9a5

## Ratings (DFMEA)

Severity: 10 fire or cell venting without warning; 9 a hazard or a regulatory breach with
some warning; 8 the device or board is destroyed; 7 a main function lost (display, charge,
a seat's input) or rework needed; 6 a function badly degraded; 5 degraded with a workaround;
4 most users notice; 3 a careful user notices; 2 barely; 1 none.

Occurrence (for this design, before it is built): 10 new and nothing prevents it; 8 new,
prevention not yet checked; 6 prevented by reasoning only; 4-5 prevented by a datasheet value
and a design rule, or checked by a script; 3 proven on similar hardware (Raspberry Pi's copied
core); 2 proven on this design's hardware (a passing test on built units); 1 cannot happen by
design (and prevention says why).

Detection: 10 no check; 8 a check is planned but not built; 7 a check exists but is late or
manual and has not run; 6 a bench test on the built board, run once per unit; 5 a recorded
inspection; 3-4 a script that can fail, run on every change; 2 two independent checks; 1 the
failure cannot pass unseen. Rated from the 'detects' checks only.

Action priority comes from S, O and D by the AIAG-VDA handbook's table, encoded in
`check_trace.py` (`action_priority`): check it against the handbook once.

## Gear

Alex has an oscilloscope, a current-limited bench supply and a microscope or loupe. Each check
that needs more says so in its 'needs' column with the cheapest option and a lead time
(`--shopping` lists them): a DMM with a thermocouple input, calipers, clip leads, a 4.096 V
reference, a LiPo bag, a USB-C breakout, a piezo igniter, a borrowed ESD gun. No IR camera
and no power profiler are assumed: the off-current checks read a DMM across a cut jumper (JP5).

## Sources borrowed from

ISO 13485 cl. 7.3 (design inputs, outputs, verification, validation, transfer, changes),
ISO 14971 (risk), AIAG-VDA FMEA handbook (DFMEA, action priority), AS9102 (first article),
IPC-A-610 Class 2 (workmanship), IPC-7351 (land patterns), IPC-2221 and IPC-2152 (spacing,
current), IPC-6012 (bow and twist), IEC 62368-1 (single faults, touch temperature, drop),
IEC 62133-2 and UN38.3 (the cell), USB 2.0 cl. 7.2 (power), IEC 61000-4-2 and EN 55035 (ESD),
ANSI/NCSL Z540.3 (test uncertainty ratio). Clause numbers quoted here are from memory: check
them against the purchased standards.

## Status on 2026-10-26

Seeded by Claude at 76955d2 on 2026-10-24, attacked by two models, re-seeded on 2026-10-25 for
684a609 and attacked again (39 findings applied, PLAN.md section l); both reviewers then
declined sign-off with 18 objections. On 2026-10-26 the register was re-seeded for be437f8,
the board being ordered (the 3.5 in panel back, the new placement, the one-piece top), the
board-level checks were re-run, the 18 objections answered (PLAN.md section g) and the order
gate trimmed to what the copper, the BOM and the fab files freeze. On 2026-10-26 a third
round added 23 findings (O-R1 to O-R12, F-R1 to F-R11; PLAN.md section n): the SPI numbers
corrected, the charge window and the CV load computed from the datasheets on disk, the
panel's mechanics read from its drawing, the phases re-cut by what the order freezes, and
new gate rules (the frozen gate state G1, the hardware signing key K1, the case's positions in
R8, the baseline against its own commit, JLC_FIXED only with committed evidence). Every
requirement is PROPOSED and APPROVALS.md is empty; the order gate fails, as it should.
