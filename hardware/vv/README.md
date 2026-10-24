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
| [verification.csv](verification.csv) | every check: procedure, pass limit, what it covers, last result, evidence, commit, date, phase, how long a result stays valid, the instrument and its accuracy, and what still has to be bought | VER-NN |
| [vp_map.csv](vp_map.csv) | a one-time map of the archived validation_plan.md's 218 methods to IDs, or CUT with a reason | plan #n |
| [gates.md](gates.md) | the order, case, EVT, DVT and PVT gates, each criterion naming IDs and a board; the unit allocation; proposed waivers | G-ORD-NN, G-CASE-NN, G-EVT-NN, G-DVT-NN, G-PVT-NN, W-NN |
| [defects.md](defects.md) | the corrective-action log (design defects; per-unit findings go in evidence/findings.csv) | DEF-NN |
| [releases/APPROVALS.md](releases/APPROVALS.md) | Alex's decisions: approvals, acceptances (with the last phase they hold for) and waivers | A-NNN |
| [releases/](releases/) | release and change records, from the two templates | REL-<BOARD>-R<rev>-<PHASE>, CHG-NN |
| [evidence/](evidence/) | the saved output of each check run, named `<date>_<VER>_<check>.txt`; units.csv and findings.csv for built units | |
| [tools/check_trace.py](tools/check_trace.py) | the trace gate (Python 3 standard library) | |
| [tools/run_checks.py](tools/run_checks.py) | runs the quick scripted checks, writes their evidence with a header and records the result | |
| [tools/seed_faults.py](tools/seed_faults.py) | proves check_trace can fail: seeds one fault at a time into a copy (VER-129) | |

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
python3 hardware/vv/tools/check_trace.py --gate case      # the printed-case gate
python3 hardware/vv/tools/check_trace.py --gate evt --board prod_board
python3 hardware/vv/tools/check_trace.py --shopping --gate evt   # what is still to buy, longest lead first
python3 hardware/vv/tools/run_checks.py                   # re-run the quick scripted checks, save evidence
python3 hardware/vv/tools/seed_faults.py                  # prove the trace gate fails on seeded faults
```

Default mode fails only on structure; check_trace.py's docstring lists every rule. Then it
reports what is not yet true: requirements with no check, H-priority risks whose 'detects'
checks have not passed, checks that are FAIL, NOT_RUN or NOT_BUILT, stale evidence, parts no
DFMEA row names, and warnings (a pass band narrower than 4 x its instrument's accuracy, a
'needs' entry with no lead time, an approval not yet committed).

## Passing a gate

1. Alex decides the requirements in releases/APPROVALS.md (APPROVED, REJECTED, or DEFERRED
   with a reason) and copies the decision into each Status.
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
  until he commits a line in releases/APPROVALS.md. check_trace refuses a status with no such
  line, a line from a commit with a Claude trailer, and an approved statement edited since.
- **No defect closes without a new or changed check.** CLOSED needs a VER in "Check added
  or changed" that passes on fresh evidence. A design fix with no check is FIXED, not CLOSED.
- **Evidence is a saved output, not a typed word.** A PASS or FAIL names an evidence file.
  For an analysis (method A) or a bench result it lives under evidence/ and carries a header:
  `COMMIT: <hash>`, `TOOLS: <versions>`, and a last `RESULT: PASS|FAIL` equal to the row;
  a bench result adds `RELEASE: <REL id>`, `UNITS: <serials>` and the release's fab zip
  sha256. A commit message is never evidence.
- **Evidence goes stale when what it covers changes.** A check's 'covers' lists globs
  (`path::regex` limits it to changed lines matching the regex as whole words), parts
  (`part:R52,R53,C10-C22`: stale when a listed part's value, LCSC number, footprint or pins
  change in design.py), or a release (`release:REL-...`: a bench result on built units,
  stale only when a later change record names the check under "What to re-run"). It is also
  stale when its commit is not an ancestor of HEAD, or when it is older than its valid_days
  (stock, quotes, the dry run). Re-run it; never edit the commit field to make it fresh.
- **Detection is only what can see the failure.** A DFMEA row's 'detects' lists the checks
  that must all pass before it is DONE. Its detection rating is 10 with none, and 8 or more
  while every one is NOT_BUILT. An occurrence of 2 or less needs a passing test on built
  hardware; an occurrence of 1 needs `Excluded:` and the reason in its prevention.
- **Nothing hides behind a later gate.** Every check is reached by a gate criterion, a
  requirement or an H-priority risk no later than its own phase.
- **A check that can be a script is a script that can fail.** Show it failing once (VER-22,
  VER-129).
- **A number quoted in a doc needs its script in the repo**, or it is NOT_BUILT (DEF-28).
- **Every design change gets a change record.** Once a release record exists, a commit that
  touches design.py, placement.py, board/lib/ or case.scad without one fails every gate. A
  BOM-only change bumps BOARD_REV. A change to the design files also updates the design
  baseline below.
- **Each board supplies its own evidence.** A check row holds one result, so a test of
  built hardware belongs to one board. Mark a check `both` only when one result serves both
  boards and it covers each board's files (or a shared file under hardware/vv or
  hardware/membrane). A product requirement that names one board's parts is split into
  .R and .P allocations; prod_board's gate reads only its own (REQ-PBD-* and the .P ones
  Alex writes).
- prod_board and prod_case are Alex's: read them, never write them. KiCad files are never
  written by hand; ref_design's are generated from design.py.

## Design baseline

The design files this register describes, as git blob hashes (`git rev-parse HEAD:<path>`).
Every ref_design gate fails with "gate evaluated against a design it does not describe" when
HEAD differs (the order gate reads design.py and placement.py, the case gate case.scad,
later gates all three). Seeded at 76955d2 on 2026-10-24; origin/main (56933a5) and
ref-design-levers (86bf694) are different designs.

- `hardware/ref_design/board/design.py` 08d17297ca629ef9fbc19ac4a91c4002948d1703
- `hardware/ref_design/board/placement.py` 266133b2e18e3649c994c97291d9a84ef0fc7055
- `hardware/ref_design/case/case.scad` 7b8a78b7e1e5bd077f053b0feebbf040a545bb9d

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

## Status on 2026-10-24

Seeded by Claude at commit 76955d2 (branch vv-plan), attacked by two models, and revised into
PLAN.md. Every requirement is PROPOSED and APPROVALS.md is empty; the order gate fails, as
it should. validation_plan.md is archived.
