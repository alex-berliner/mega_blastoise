# Verification and validation

How we know a board works before and after we order it. One page; the files do the rest.
It covers both boards: `hardware/ref_design` (REF-A, Claude's) and `hardware/prod_board` with
`hardware/prod_case` (Alex's). Each board supplies its own evidence. Nothing here edits either
board.

"Works" means all of: built as designed (verified), the right design for the numbers
(validated), safe (the Li-ion cell, touch temperature), made by JLC without surprises, fits
the case, survives use.

## The files

| File | What | ID |
|---|---|---|
| [requirements.md](requirements.md) | what the device must do, each a number with a tolerance, a reason, a source and the checks that prove it | REQ-<AREA>-NN (PRD, BAT, PWR, MCU, DSP, AUD, BTN, MEC, THM, EMC, FAB, ASM, TST, PBD) |
| [dfmea.csv](dfmea.csv) | how each part, interface and assembly step can fail, how bad that is, what prevents it and what detects it (AIAG-VDA) | DF-NN |
| [verification.csv](verification.csv) | every check: procedure, pass limit, what it covers, last result, evidence, commit, date | VER-NN |
| [vp_map.csv](vp_map.csv) | each of validation_plan.md's 218 methods: the IDs that carry it, or CUT with a reason; its status checked against the repo | plan #n |
| [gates.md](gates.md) | the order, EVT, DVT and PVT gates, each criterion naming IDs; Alex's waivers | G-ORD-NN, G-EVT-NN, G-DVT-NN, G-PVT-NN, W-NN |
| [defects.md](defects.md) | the corrective-action log | DEF-NN |
| [releases/](releases/) | release records and change records, from the two templates | REL-<BOARD>-R<rev>-<PHASE>, CHG-NN |
| [evidence/](evidence/) | the saved output of each check run, named `<date>_<VER>_<check>.txt` | |
| [tools/check_trace.py](tools/check_trace.py) | the trace gate (Python 3 standard library) | |

## How they link

```
requirement  --Verified by-->  check (VER)  <--detection--  DFMEA row
     ^                             |  ^                          ^
     |                             |  +-- defect: "check added or changed"
     +---- gate criterion ---------+-----------------------------+
validation_plan method --maps_to--> REQ / DF / VER, or CUT
```

A requirement is proven by its checks. A risk is caught by its detection checks. A defect
names the check that now catches it. A gate names the checks, requirements, risks and
defects it needs. Every ID written anywhere in these files must exist, or the trace gate fails.

## Running the gate

```sh
python3 hardware/vv/tools/check_trace.py                  # structure (exit 1 on an error) + report
python3 hardware/vv/tools/check_trace.py --gate order     # exit 1 unless the order gate is met
python3 hardware/vv/tools/check_trace.py --gate evt --board prod_board
python3 hardware/vv/tools/check_trace.py --verbose        # every ID in every list
```

Default mode fails only on structure: duplicate or malformed IDs, dangling references,
malformed rows, an action priority that disagrees with the AIAG-VDA table, PASS or FAIL
without a commit and date, a 'covers' pattern that matches no file, a vp_map row that
disagrees with validation_plan.md, a CLOSED defect with no check. Then it reports what is
not yet true: requirements with no check, H-priority risks with no passing detection,
checks that are FAIL, NOT_RUN or NOT_BUILT, stale evidence, unmapped plan methods.

## Passing a gate

1. Alex approves the requirements (Status APPROVED, REJECTED or DEFERRED with a reason).
2. Run every check the gate needs at one commit, save its output under `evidence/`, and
   write PASS or FAIL, the evidence path, the commit and the date into verification.csv.
3. Each H-priority DFMEA row is acted on: take the action, re-rate S, O and D (a passing
   check that really detects the failure lowers D), and mark it DONE with its detection
   check passing; or Alex accepts it in its action field, `Accepted by Alex YYYY-MM-DD:
   reason`, status ACCEPTED. A row still rated H after its action needs the acceptance.
4. Fix or close the defects the gate asks for.
5. Run `check_trace.py --gate <phase>` until it exits 0, paste its output into the
   release record, tag the commit, then order.

Phases: ORDER (before the 5-board EVT order), EVT (the 5 boards), DVT (the revised spin),
PVT (the 30-board run), SALE (before any unit is sold or given away).

## Rules

- **Requirements stay PROPOSED until Alex approves them.** Claude may propose, re-word and
  add; only Alex sets APPROVED, REJECTED or DEFERRED.
- **No defect closes without a new or changed check.** CLOSED needs a VER in "Check added
  or changed" that passes on fresh evidence. A design fix with no check is FIXED, not CLOSED.
- **Evidence goes stale when what it covers changes.** Each check lists the files it covers
  (globs; `path::regex` limits it to changed lines that match). If any of them changed
  between the evidence's commit and HEAD, or the commit is not an ancestor of HEAD, the
  result no longer counts. Re-run it; do not edit the commit field to make it fresh.
- **A check that can be a script is a script that can fail.** Show it failing once (VER-22).
- **A number quoted in a doc needs its script in the repo**, or it is NOT_BUILT (DEF-28).
- Every change to the design, a part, the case or a check gets a CHG record listing the
  REQ, DF and VER it touches and what to re-run.
- **Each board supplies its own evidence.** A check row holds one result, so a test of
  built hardware belongs to one board. Mark a check `both` only when one result serves both
  boards (the comb coupon, the DFMEA review, the bench gear, shared DS Lite parts). As
  prod_board grows it gets its own VER rows; until then the report lists the requirements
  that have no prod_board check.
- prod_board and prod_case are Alex's: read them, never write them. KiCad files are never
  written by hand; ref_design's are generated from design.py.

## Ratings (DFMEA)

Severity: 10 fire or cell venting without warning; 9 a hazard or a regulatory breach with
some warning; 8 the device or board is destroyed; 7 a main function lost (display, charge,
a seat's input) or rework needed; 6 a function badly degraded; 5 degraded with a workaround;
4 most users notice; 3 a careful user notices; 2 barely; 1 none.

Occurrence (for this design, before it is built): 10 new and nothing prevents it; 8 new,
prevention not yet checked; 6 prevented by reasoning only; 4-5 prevented by a datasheet value
and a design rule, or checked by a script; 3 proven on similar hardware (Raspberry Pi's copied
core); 2 proven on this design's hardware; 1 cannot happen by design.

Detection: 10 no check; 8 a check is planned but not built; 7 a check exists but is late or
manual and has not run; 6 a bench test on the built board, run once per unit; 5 a recorded
inspection; 3-4 a script that can fail, run on every change; 2 two independent checks; 1 the
failure cannot pass unseen.

Action priority comes from S, O and D by the AIAG-VDA handbook's table, encoded in
`check_trace.py` (`action_priority`): check it against the handbook once.

## Gear

Alex has an oscilloscope, a current-limited bench supply and a microscope or loupe. Each check
that needs more names the cheapest option in its procedure (a DMM with a thermocouple input,
calipers, feeler gauges, an H-field probe set, a borrowed ESD gun). No IR camera and no power
profiler are assumed: the off-current checks read a DMM across a cut jumper (JP5).

## Sources borrowed from

ISO 13485 cl. 7.3 (design inputs, outputs, verification, validation, transfer, changes),
ISO 14971 (risk), AIAG-VDA FMEA handbook (DFMEA, action priority), AS9102 (first article),
IPC-A-610 Class 2 (workmanship), IPC-7351 (land patterns), IPC-2221 and IPC-2152 (spacing,
current), IPC-6012 (bow and twist), IEC 62368-1 (single faults, touch temperature, drop),
IEC 62133-2 and UN38.3 (the cell), USB 2.0 cl. 7.2 (power), IEC 61000-4-2 and EN 55035 (ESD).
Clause numbers quoted here are from memory: check them against the purchased standards.

## Status on 2026-10-24

Seeded by Claude at commit 76955d2 (branch vv-plan) for two other models to attack. Every
requirement is PROPOSED; the order gate fails, as it should. The statuses that
`docs/validation_plan.md` gives its methods are now kept in vp_map.csv.
