#!/usr/bin/env python3
"""Prove check_trace.py can fail: seed one fault at a time and show each one is caught.

    python3 hardware/vv/tools/seed_faults.py            # every fault; exit 1 if any is missed
    python3 hardware/vv/tools/seed_faults.py --list

Two kinds of fault. Most change the register only: each copies hardware/vv to a scratch
directory, changes one thing, and runs check_trace.py --vv <copy> on it. The rest need git
history (who committed an approval line, a design change after a release, a file left
uncommitted, evidence made stale by a later commit): for those the repository is cloned once
into a scratch directory (git clone --shared, nothing written to this repository), the
working register is committed there as a base, and each fault resets to that base, commits
its change as "Alex" (or with a Claude trailer where that is the fault) and runs the clone's
own check_trace.py. The clone signs its commits with a throwaway SSH key named in a
throwaway allowed-signers file and pinned in the clone's README, so an approval line counts
there the way Alex's will (O-R4, round 2); a second allowed key signs where a fault wants
another key, and a fault that wants an unsigned commit makes one on purpose.

A third kind runs design_numbers.py in the clone (pcbnew and kicad-cli): a fault there must
leave the check at RESULT: FAIL (a comment in fab.py must not pass the CPL or drill check:
O-R1; a back-side test point moved next to the cell's hole must fail th1: O-R14).

A structural fault must exit 1 and print its message; a report fault must exit 0 with its
text in the report; a gate fault must exit 1 with its text in the gate's output; a design
number fault must print RESULT: FAIL. The clean
copy and the clean clone must exit 0. This is VER-129: the trace gate's own seeded-fault test
(DEF-35, DEF-47).

Round 3's six findings add the faults that keep the new rules honest (2026-10-26):

  F-R3  README's baseline hash rewritten from HEAD after a design.py edit: the default mode fails
        (the register describes another design than the baseline commit holds).
  F-R4  a JLC_FIXED entry with no evidence file, and one whose evidence file is not committed:
        neither counts, so the CPL check still fails on the row.
  O-R4  a case.scad commit that moves a button position fails the order gate (R8, the case's
        positions), while a case-only commit that moves no position (2ddb7fe's engraved labels)
        passes that part of R8.
  O-R5  a check moved later, or made advisory, after Alex's signed GATE@ line fails the order
        gate (G1).
  O-R11 an ORDER check named only in the plan's 'what moved where' table is not a step.
  O-R12 a software signing key fails the order gate (K1); --accept-software-key passes it and
        says so; a line in APPROVALS.md that tries to grant the exception does not count.

The coverage grid and the stale-names scan add theirs (2026-10-27):

  VER-188 a live DFMEA row, check and requirement naming a part number (TP4056), a net (LS_GATE)
        and a reference designator (J6) that left design.py, outside 'Was: ...': the default mode
        fails on each.
  grid  a live check with no classes, and a NOT_NEEDED cell a check targets: the default mode
        fails; a NOT_NEEDED reason dropped, an ACCEPTED_GAP Alex has not signed and a grid.md
        edited by hand each fail the order gate (R9); the default mode reports the GAP count; a
        control: a gap Alex accepts in a signed GRID-<class>-<subsystem> line leaves R9.
  audit a NOT_RUN bench test of phase EVT re-tagged into an empty cell leaves it a GAP at R9; a
        class 15 analysis that has not run leaves its cells planned; a ref no design.py ever had
        (R666) fails the default mode.

The review's findings applied at 71737ae add theirs (2026-10-27):

  F-R1  a defect marked FIXED whose own Status text says it stays OPEN: the default mode fails.
  F-R3  an analysis whose procedure reads pinmap.md and whose covers do not name it: the default mode fails.
  F-R5  DF-48 pairing J4 with the PicoBlade family again: the connector-family rule fails the default mode.
  O-R8  a gone net written in KiCad's path form (/USB_Charger/VBUS_D5): the stale-names scan sees it.
  O-R6  VER-137 not run: the display's class 4 cell, cleared only by an unrun analysis, is planned at R9.
  O-R1  a defect taken from OPEN to FIXED is listed by G0 with no GATE@ line and fails G1 after one.

The framework fixes after vv-rp2354b-final's sign-off add theirs (2026-10-28):

  accept   a defect ACCEPTED with no DF-ACCEPT line, and one on an unsigned line: the default mode fails;
           a signed line moves DEF-62 past the order gate and lets VER-181 through as ACCEPTED (a control);
           DEF-37 accepted lets VER-164 through and leaves VER-181, which it does not name, FAIL.
  sources  a PASS whose 'sources' names a datasheet not on disk is BLOCKED at the order gate and in the
           default report; a source written as a path fails the default mode.
  R8       a proposal doc (docs/living/*proposal*.md) changed on the order branch after the baseline: the default
           mode warns and the order gate fails.
  grid     a GRID line signed with no ACCEPTED_GAP row leaves its cell a GAP, and R9 says why.

The F1C200s re-target adds theirs (2026-10-28):

  DROPPED  a dropped check or defect that names no board and tag; a DFMEA row or a gate criterion DROPPED on the
           live board; a live check on a dropped board; a live requirement, risk or parent leaning on a DROPPED
           row; a live row citing one outside 'Was:': the default mode fails on each.
  baseline an F1C200s PASS while README's '- Baseline board:' is the dropped ESP32-S3 board's, and no such line.
  subsys   a live requirement with no '- Subsystem:', and a requirement or a check on the dropped board's mcu or
           usb_debug.
  new-MCU  a stage naming a DROPPED check, an entry that skips the stage before, a token that is no need, stages
           out of order (structure); the chip selection's evidence gone, and the stage the register is at moving
           to NM-03 when NM-01's exit is met (report).
F's audit of the re-target adds theirs (2026-10-28):

  chip gate  a dev-board check's PASS whose covers name a doc under hardware/ref_design, and one that names a part,
             while the baseline is the dropped board's: the default mode fails (only a check that reads Alex's spec,
             the datasheets or the register may run before the F1C200s design is the baseline).
  entry      a stage's entry need that is not met keeps the stage open (report).
  lessons    a dropped defect that names no live DF row its lesson carries to, one whose row is itself DROPPED, and one
             that does not say what it was there: the default mode fails.
  paths      a misspelt path on a live check (never committed, named by no NOT_BUILT row and not in
             repo_paths.PLANNED): the default mode fails.
The seeds that leaned on the ESP32-S3 board's rows (DF-35, DF-87, DEF-62, DEF-84, VER-99's PASS and the like) moved
to the F1C200s board's; where a seed needs an F1C200s PASS it takes the baseline as the live board's in its copy
(live_baseline), and where it needs a live defect it adds one (add_defect).

Python 3 standard library and git.
"""
import csv
import datetime
import hashlib
import io
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
VV = os.path.dirname(HERE)
TRACE = os.path.join(HERE, "check_trace.py")
ROOT = subprocess.run(["git", "-C", HERE, "rev-parse", "--show-toplevel"], capture_output=True, text=True).stdout.strip()
HEAD7 = subprocess.run(["git", "-C", ROOT, "rev-parse", "--short=7", "HEAD"], capture_output=True, text=True).stdout.strip()
HEAD_SHA = subprocess.run(["git", "-C", ROOT, "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
TODAY = datetime.date.today().isoformat()
DESIGN_BLOB = subprocess.run(["git", "-C", ROOT, "rev-parse", "HEAD:hardware/ref_design/board/design.py"],
                             capture_output=True, text=True).stdout.strip()
# README's order branch and baseline commit (main from 71737ae since 2026-10-27): the seeds name them from there,
# so a re-seed moves them without editing this file
_OB = re.search(r"^- Order branch: `([^`]+)` from `([0-9a-f]+)`", open(os.path.join(VV, "README.md"), encoding="utf-8").read(),
                re.M)
ORDER_BRANCH, BASE7 = (_OB.group(1), _OB.group(2)[:7]) if _OB else ("main", "?")


def name_order_branch(vv, name):
    """Point README's order-branch line at another branch (the seeded ones), whatever it names today."""
    text_edit(vv, "README.md", f"- Order branch: `{ORDER_BRANCH}`", f"- Order branch: `{name}`")


def csv_edit(d, name, rid, **fields):
    """Change fields of the row whose first column is rid (or append a copy with _dup)."""
    path = os.path.join(d, name)
    rows = list(csv.reader(open(path, newline="", encoding="utf-8")))
    head = rows[0]
    for r in rows[1:]:
        if r and r[0] == rid:
            if fields.pop("_dup", False):
                rows.append(list(r))
            for k, v in fields.items():
                r[head.index(k)] = v(r[head.index(k)]) if callable(v) else v
            break
    else:
        raise SystemExit(f"seed_faults: {rid} not in {name}")
    buf = io.StringIO()
    csv.writer(buf, lineterminator="\n").writerows(rows)
    open(path, "w", encoding="utf-8").write(buf.getvalue())


def text_edit(d, name, old, new):
    path = os.path.join(d, name)
    s = open(path, encoding="utf-8").read()
    if old not in s:
        raise SystemExit(f"seed_faults: {old!r} not in {name}")
    open(path, "w", encoding="utf-8").write(s.replace(old, new, 1))


def write(d, name, text):
    open(os.path.join(d, name), "w", encoding="utf-8").write(text)


def block_field(d, name, bid, field, value):
    """Set '- field: value' inside the '### bid:' block of a markdown file."""
    path = os.path.join(d, name)
    s = open(path, encoding="utf-8").read()
    m = re.search(r"^### " + re.escape(bid) + r": .*?(?=^### |^## |\Z)", s, re.M | re.S)
    if not m:
        raise SystemExit(f"seed_faults: {bid} not in {name}")
    blk = re.sub(r"^- " + re.escape(field) + r": .*$", lambda _: f"- {field}: {value}", m.group(0), count=1,
                 flags=re.M)
    open(path, "w", encoding="utf-8").write(s[:m.start()] + blk + s[m.end():])


def approve(d, line):
    text_edit(d, "releases/APPROVALS.md", "|---|---|---|---|---|---|---|", "|---|---|---|---|---|---|---|\n" + line)


REL = """# {id}

- Board: ref_design
- BOARD_REV: 1
- BOM sha256: {bom}
- Commit (full hash): {commit}
- Case positions sha256: {case}
- Case positions sha256 at the baseline: {case}
"""

# (name, mode, expected text, mutation). mode: 'structure' wants exit 1 and the text;
# 'report' wants exit 0 and the text in the report; 'gate' wants the text in the gate output
# (--gate order unless the mutation returns another gate).
FAULTS = [
    ("duplicate VER id", "structure", "duplicate ID VER-01",
     lambda d: csv_edit(d, "verification.csv", "VER-01", _dup=True)),
    ("dangling DF id in a gate", "structure", "DF-999 is referenced but not defined",
     lambda d: text_edit(d, "gates.md", "| ADVISORY:VER-98 DFMEA:COVERAGE |", "| ADVISORY:VER-98 DFMEA:COVERAGE DF-999 |")),
    ("action priority off the AIAG-VDA table", "structure", "(AIAG-VDA table)",
     lambda d: csv_edit(d, "dfmea.csv", "DF-04", action_priority="L")),
    ("PASS without a commit", "structure", "PASS needs commit",
     lambda d: csv_edit(d, "verification.csv", "VER-01", last_result="PASS", commit="", date=TODAY)),
    ("evidence header names another commit", "structure", "has no 'COMMIT: 56933a5' line",
     lambda d: csv_edit(d, "verification.csv", "VER-01", last_result="PASS", commit="56933a5", date=TODAY,
                        evidence=lambda e: e if e.startswith("hardware/vv/evidence/") else
                        "hardware/vv/evidence/2026-10-26_VER-01_check_netlist.txt")),
    ("a commit message as evidence", "structure", "a commit message is not evidence",
     lambda d: csv_edit(d, "verification.csv", "VER-01", last_result="PASS", commit=HEAD7, date=TODAY,
                        evidence="git show 76955d2")),
    ("an analysis PASS with no saved output", "structure", "needs its saved output under",
     lambda d: csv_edit(d, "verification.csv", "VER-01", last_result="PASS", commit=HEAD7, date=TODAY,
                        evidence="hardware/vv/README.md")),
    ("a test (T) PASS with no saved output", "structure", "VER-60: needs its saved output under",
     lambda d: csv_edit(d, "verification.csv", "VER-60", last_result="PASS", commit=HEAD7, date=TODAY,
                        evidence="hardware/ref_design/docs/assembly.md")),
    ("CLOSED defect with no check", "structure", "CLOSED with no VER",
     lambda d: (block_field(d, "defects.md", "DEF-35", "Check added or changed", "none"),
                block_field(d, "defects.md", "DEF-35", "Status", "CLOSED (seeded)"))),
    ("covers glob matching nothing", "structure", "matches no tracked file",
     lambda d: csv_edit(d, "verification.csv", "VER-01", covers=lambda c: c + " hardware/nothing/*.py")),
    ("covers regex matching no line", "structure", "matches no current line",
     lambda d: csv_edit(d, "verification.csv", "VER-104", covers="hardware/ref_design/docs/pinmap.md::nosuchword")),
    ("covers part not in design.py", "structure", "R999 not in design.py",
     lambda d: csv_edit(d, "verification.csv", "VER-12", covers="part:R999")),
    ("a 'both' check covering neither the board nor the register", "structure", "a 'both' check must cover each board",
     lambda d: csv_edit(d, "verification.csv", "VER-01", board="both",
                        covers="crates/mega_blastoise_core/src/oled_ctl.rs")),
    ("CUT check with no reason", "structure", "CUT needs its reason",
     lambda d: csv_edit(d, "verification.csv", "VER-47", evidence="")),
    ("RETIRED check with no commit", "structure", "RETIRED needs 'RETIRED in <commit>",
     lambda d: csv_edit(d, "verification.csv", "VER-09", evidence="RETIRED: the 3-ring is gone")),
    ("a live requirement verified by a retired check", "structure", "REQ-PRD-05: verified by VER-11, which is RETIRED",
     lambda d: block_field(d, "requirements.md", "REQ-PRD-05", "Verified by", "VER-65, VER-11")),
    ("a live risk detected by a retired check", "structure", "DF-214: detects VER-11, which is RETIRED",
     lambda d: csv_edit(d, "dfmea.csv", "DF-214", detects=lambda x: x + " VER-11", detection=lambda x: x + "; VER-11")),
    ("a gate criterion naming a retired check", "structure", "G-EVT-04: needs VER-11, which is RETIRED",
     lambda d: text_edit(d, "gates.md", "| VER-12 VER-30 VER-13 VER-31 ", "| VER-11 VER-12 VER-30 VER-13 VER-31 ")),
    ("vp_map status changed with no evidence", "structure", "with no evidence",
     lambda d: csv_edit(d, "vp_map.csv", "4", status_now="no")),
    ("requirement with no Verified by", "structure", "field 'Verified by' missing or empty",
     lambda d: block_field(d, "requirements.md", "REQ-PRD-05", "Verified by", "")),
    ("requirement APPROVED with no APPROVALS line", "structure", "has no committed line for it",
     lambda d: block_field(d, "requirements.md", "REQ-PRD-05", "Status", "APPROVED")),
    ("allocation on the wrong board", "structure", "a .F allocation is Board: f1c200s",
     lambda d: block_field(d, "requirements.md", "REQ-PRD-01.F", "Board", "both")),
    ("risk ACCEPTED with no APPROVALS line", "structure", "ACCEPTED but releases/APPROVALS.md",
     lambda d: csv_edit(d, "dfmea.csv", "DF-247", status="ACCEPTED")),
    ("approval line signed by Claude", "structure", "only Alex signs",
     lambda d: approve(d, "| A-001 | REQ-PRD-05 | APPROVED | - | - | Claude | 2026-10-24 |")),
    ("a deferral with no scope phase", "structure", "REQ-PRD-05 DEFERRED needs a scope phase",
     lambda d: approve(d, "| A-001 | REQ-PRD-05 | DEFERRED | - | waiting for the cell | Alex | 2026-10-25 |")),
    ("detects names a check the text does not", "structure", "which the detection text does not describe",
     lambda d: csv_edit(d, "dfmea.csv", "DF-238", detects=lambda x: x + " VER-02",
                        detection=lambda x: x.replace("VER-02", "the pin gate"))),
    ("detection rating better than its checks", "structure", "must be 8 or more",
     lambda d: csv_edit(d, "dfmea.csv", "DF-210", detection_rating="6", action_priority="H")),
    ("occurrence 2 with no built hardware", "structure", "occurrence 2 needs a passing T check",
     lambda d: csv_edit(d, "dfmea.csv", "DF-224", occurrence="2", action_priority="L")),
    ("DONE with no detects", "structure", "DONE needs at least one 'detects' check",
     lambda d: csv_edit(d, "dfmea.csv", "DF-247", status="DONE")),
    # 2026-10-28: VER-218 (the cell's chain on paper) is reached only through G-ORD-22's PHASE:ORDER, so taking that
    # need away strands it (VER-21, the earlier seed, is reached by DF-209 since the F1C200s re-seed)
    ("an ORDER check no gate reaches", "structure", "VER-218: phase ORDER, but no gate criterion",
     lambda d: text_edit(d, "gates.md", "| PHASE:ORDER | both |", "| VER-01 | both |")),
    ("criterion with a bad board", "structure", "board 'nowhere' not in",
     lambda d: text_edit(d, "gates.md", "| PHASE:ORDER | both |", "| PHASE:ORDER | nowhere |")),
    ("new BOM under an old BOARD_REV", "structure", "BOARD_REV is still",
     lambda d: (write(d, "releases/REL-REFA-R1-EVT.md", REL.format(case="b" * 64, id="REL-REFA-R1-EVT", bom="a" * 64, commit="")),
                write(d, "releases/REL-REFA-R2-DVT.md", REL.format(case="b" * 64, id="REL-REFA-R2-DVT", bom="b" * 64, commit="")))),
    ("design baseline differs from HEAD", "structure", "design baseline differs from HEAD",
     lambda d: text_edit(d, "README.md", DESIGN_BLOB, "0" * 40)),
    ("design baseline leaves out the routed board", "structure",
     "the design baseline does not name hardware/ref_design/board/battler.kicad_pcb",
     lambda d: text_edit(d, "README.md", "- `hardware/ref_design/board/battler.kicad_pcb`", "- `hardware/ref_design/board/battler_old.kicad_pcb`")),
    ("O-S4: the design baseline leaves out tools/build.py (it runs the build, route, silk and fab stages)", "structure",
     "the design baseline does not name hardware/ref_design/tools/build.py",
     lambda d: drop_line(d, "README.md", "- `hardware/ref_design/tools/build.py`")),
    ("an advisory-only check made binding", "gate", "VER-40 NOT_RUN",
     lambda d: text_edit(d, "gates.md", "| ADVISORY VER-40 VER-21 |", "| VER-40 VER-21 |")),
    # the second review (2026-10-25)
    ("O-R3: an inspection PASS on a design file it does not cover", "structure", "VER-28: needs its saved output under",
     lambda d: csv_edit(d, "verification.csv", "VER-28", last_result="PASS", commit=HEAD7, date=TODAY,
                        evidence="hardware/ref_design/README.md")),
    ("O-R3: an inspection's evidence with no WHO or ARTEFACT", "structure", "has no 'WHO:' line",
     lambda d: borrow_evidence(d, "VER-28", "VER-220")),
    ("O-R5: a defect with no Gate", "structure", "DEF-47: Gate ''",
     lambda d: block_field(d, "defects.md", "DEF-47", "Gate", "")),
    # 2026-10-28: the F1C200s board has no live defect of its own yet, so the seed adds one with Gate CASE, then moves it
    ("O-R5: a CASE defect moved to the order gate blocks it", "gate", "OPEN (Gate ORDER or earlier): DEF-47 DEF-103",
     lambda d: (add_defect(d, "DEF-103", "VER-147", "CASE"), block_field(d, "defects.md", "DEF-103", "Gate", "ORDER"))),
    ("O-R8: occurrence 3 with no 'Similar:'", "structure", "DF-224: occurrence 3 needs 'Similar:",
     lambda d: csv_edit(d, "dfmea.csv", "DF-224", occurrence="3", action_priority="L")),
    ("F-R14: a live check cites a RETIRED row", "structure", "VER-148: cites DF-106, which is RETIRED",
     lambda d: csv_edit(d, "verification.csv", "VER-148", pass_limit=lambda x: x + " (as DF-106)")),
    # round 2's sign-off objections (2026-10-26)
    ("round 2 (O): an ORDER check that no step of PLAN.md section (d) names", "structure",
     "VER-131: an ORDER check that no numbered step of PLAN.md section (d) names",
     lambda d: plan_d_drop(d, "VER-131")),
    # round 3's findings (2026-10-26)
    ("O-R11: an ORDER check named only in the plan's 'what moved where' table is not a step", "structure",
     "VER-174: an ORDER check that no numbered step of PLAN.md section (d) names",
     lambda d: plan_d_table_only(d, "VER-174")),
    ("F-R4: a JLC_FIXED entry with no evidence file does not count", "structure",
     "JLC_FIXED[J1]: the evidence file hardware/vv/evidence/2026-10-27_VER-28_jlc_preview.txt does not exist",
     lambda d: jlc_fixed_entry(d)),
    ("round 2 (F): a requirement whose only check moved to a later phase", "structure",
     "REQ-PWR-13: Phase ORDER, but its earliest live check (VER-99) is EVT",
     lambda d: csv_edit(d, "verification.csv", "VER-99", phase="EVT")),
    # 2026-10-28: with the baseline's design.py taken as the live board's, no F1C200s row names its parts: every one is
    # in no row (with the dropped board's files named as the baseline it is not counted at all: the next seed)
    ("round 2 (F): a part no DFMEA row names fails DFMEA:COVERAGE", "gate", "parts in no DFMEA row: ",
     lambda d: live_baseline(d)),
    ("2026-10-28: DFMEA:COVERAGE fails while design.py is a dropped board's", "gate",
     "DFMEA:COVERAGE reads design.py, which is the esp32s3 board's", lambda d: None),
    ("round 2 (F): detection rating 2 on a single 'detects' check", "structure",
     "DF-224: detection_rating 2 needs two or more independent 'detects' checks",
     lambda d: csv_edit(d, "dfmea.csv", "DF-224", detection_rating="2", action_priority="L")),
    # 2026-10-28: a path main moved (hardware/membrane to hardware/ref_design/board/membrane in fd66be9), written
    # back into a procedure, and a deleted board's file pinned at a commit that never had it (VER-220)
    ("VER-220: a procedure names a path main moved (hardware/membrane)", "structure",
     "hardware/membrane/tools/check_contact_fp.py is not in HEAD's tree: moved or deleted",
     lambda d: csv_edit(d, "verification.csv", "VER-08", procedure=lambda x: x.replace(
         "hardware/ref_design/board/membrane/tools/", "hardware/membrane/tools/"))),
    ("VER-220: a deleted file pinned at a commit that does not have it", "structure",
     "pinned at 574c94c, which does not have it",
     lambda d: text_edit(d, "requirements.md", "hardware/prod_case/README.md@611ebe3",
                         "hardware/prod_case/README.md@574c94c")),
    # the coverage grid and the stale-names scan (2026-10-27)
    ("VER-188: a live row names a part number that left design.py (TP4056 outside 'Was:')", "structure",
     f"DF-217: names what design.py at {BASE7} does not have: TP4056",
     lambda d: csv_edit(d, "dfmea.csv", "DF-217", cause=lambda x: x + " (as the TP4056's was)")),
    ("VER-188: a live check names a net that left design.py (LS_GATE)", "structure",
     f"VER-21: names what design.py at {BASE7} does not have: LS_GATE (net",
     lambda d: csv_edit(d, "verification.csv", "VER-21", pass_limit=lambda x: x + ", LS_GATE")),
    ("VER-188: a live requirement names a part that left design.py (D5 outside 'Was:')", "structure",
     f"REQ-PWR-13: names what design.py at {BASE7} does not have: D5 (part",
     lambda d: text_edit(d, "requirements.md", "(Was: the TP4056 design's D5, which left in 2d1124d).",
                         "(the TP4056 design's D5, which left in 2d1124d).")),
    ("grid: a live check that names no classes", "structure", "VER-99: a live check names its classes and subsystems",
     lambda d: csv_edit(d, "verification.csv", "VER-99", classes="")),
    ("grid: a NOT_NEEDED cell that a check targets", "structure", "class 4 case is NOT_NEEDED, but VER-99",
     lambda d: csv_edit(d, "verification.csv", "VER-99", subsystems="power case")),
    ("grid R9: a NOT_NEEDED reason dropped leaves a GAP the order gate holds", "gate", "3 case no check",
     lambda d: drop_line(d, "grid_reasons.csv", "3,case,")),
    ("grid R9: an ACCEPTED_GAP Alex has not signed is still a GAP", "gate", "3 case no check",
     lambda d: text_edit(d, "grid_reasons.csv", "3,case,NOT_NEEDED,", "3,case,ACCEPTED_GAP,")),
    ("grid R9: a grid.md edited by hand fails the order gate", "gate", "grid.md is not what the register gives",
     lambda d: text_edit(d, "grid.md", "| A1 Requirements |", "| A1 Requirement |")),
    ("grid: the default mode reports the GAP count", "report", "(1 with no check,",
     lambda d: drop_line(d, "grid_reasons.csv", "3,case,")),
    # the grid audit (2026-10-27)
    ("grid R9: a NOT_RUN bench test of phase EVT re-tagged into an empty cell does not clear it", "gate",
     "3 case no check",
     lambda d: (drop_line(d, "grid_reasons.csv", "3,case,"),
                csv_edit(d, "verification.csv", "VER-70", classes="3", subsystems="case"))),
    # 2026-10-28, the F1C200s re-seed: no class 15 analysis has run on an F1C200s design, so all 9 cells are planned;
    # VER-193 (soc, buttons, usbc, cell, case, storage) made NOT_RUN, a built analysis that has not run, must leave
    # them planned: counting it would give 3
    ("grid: a class 15 analysis that has not run is no paper estimate", "report", "15 Thermal 9",
     lambda d: csv_edit(d, "verification.csv", "VER-193", last_result="NOT_RUN", evidence="", commit="", date="")),
    ("VER-188: a live check names a ref no design.py ever had (R666, a typo)", "structure",
     "R666 (part, never in design.py)",
     lambda d: csv_edit(d, "verification.csv", "VER-21", pass_limit=lambda x: x + ", R666")),
    # the review's findings applied at 71737ae (2026-10-27)
    ("F-R1: a defect marked FIXED while its own Status text says it stays OPEN", "structure",
     "DEF-47: Status is FIXED but its own text says 'stays OPEN'",
     lambda d: text_edit(d, "defects.md", "- Status: OPEN (O-R9, 2026-10-27:",
                         "- Status: FIXED (It stays OPEN, and the order gate fails on it. O-R9, 2026-10-27:")),
    ("F-R3: an analysis that reads pinmap.md without naming it in covers", "structure",
     "VER-231: its procedure reads pinmap.md, but covers does not name",
     lambda d: csv_edit(d, "verification.csv", "VER-231", procedure=lambda x: x + "; the rails pinmap.md lists")),
    ("F-R5: a live row pairs J4 with the PicoBlade family", "structure",
     "DF-231: names what design.py at " + BASE7 + " does not have: PicoBlade (PicoBlade) beside J4",
     lambda d: csv_edit(d, "dfmea.csv", "DF-231", prevention=lambda x: x + "; J4 takes a PicoBlade plug")),
    ("O-R8: a live check names a gone net in KiCad's path form (/USB_Charger/VBUS_D5)", "structure",
     "VER-21: names what design.py at " + BASE7 + " does not have: VBUS_D5 (net",
     lambda d: csv_edit(d, "verification.csv", "VER-21", pass_limit=lambda x: x + ", /USB_Charger/VBUS_D5")),
    # 2026-10-28, the F1C200s re-seed: VER-190 (the case's architecture, an ORDER analysis) is built and NOT_RUN, the
    # only check of class 2 for the case besides VER-232's planned one: it must leave the cell planned, so all nine
    # class 2 cells stay GAP (counting a NOT_RUN analysis would give 8)
    ("O-R6: an analysis that has not run does not clear a class 1-14 cell (case, class 2)", "report",
     "2 Architecture fit 9",
     lambda d: csv_edit(d, "verification.csv", "VER-190", last_result="NOT_RUN", evidence="", commit="", date="")),
    # the framework fixes after vv-rp2354b-final's sign-off (2026-10-28)
    ("accept: a defect ACCEPTED with no DF-ACCEPT line at all", "structure",
     "DEF-47: ACCEPTED but releases/APPROVALS.md has no signed 'DF-ACCEPT:DEF-47' line",
     lambda d: block_field(d, "defects.md", "DEF-47", "Status", "ACCEPTED (to EVT, seeded)")),
    # 2026-10-28: VER-220 (the register's own path check, board both) is a PASS the F1C200s re-seed kept
    ("sources: a PASS whose datasheet is not on disk is BLOCKED at the order gate", "gate",
     "VER-220 BLOCKED: its PASS rests on nosuch_datasheet.pdf",
     lambda d: csv_edit(d, "verification.csv", "VER-220", sources="nosuch_datasheet.pdf")),
    ("sources: the default mode reports a PASS on a datasheet not on disk as BLOCKED", "report",
     "    VER-220: nosuch_datasheet.pdf",
     lambda d: csv_edit(d, "verification.csv", "VER-220", sources="nosuch_datasheet.pdf")),
    ("sources: a source that is a path, not a file in the datasheets directory", "structure",
     "VER-220: sources token 'datasheets/bq24074.pdf' is not a file name",
     lambda d: csv_edit(d, "verification.csv", "VER-220", sources="datasheets/bq24074.pdf")),
    # the F1C200s re-target (2026-10-28): a dropped board's rows, the subsystems, the baseline's board, the new-MCU path
    ("DROPPED: a dropped check that names no board and tag", "structure",
     "VER-203: DROPPED needs 'DROPPED with esp32s3 (tag vv-esp32s3-final): <why>'",
     lambda d: csv_edit(d, "verification.csv", "VER-203", evidence="DROPPED: the module left")),
    ("DROPPED: a dropped defect that names no board and tag", "structure",
     "DEF-01: DROPPED needs 'DROPPED with esp32s3 (tag vv-esp32s3-final): <why>'",
     lambda d: block_field(d, "defects.md", "DEF-01", "Status", "DROPPED (the board left)")),
    ("DROPPED: a DFMEA row DROPPED on the live board", "structure",
     "DF-04: DROPPED, but its board 'f1c200s' is not a dropped board",
     lambda d: csv_edit(d, "dfmea.csv", "DF-04", board="f1c200s")),
    ("DROPPED: a gate criterion DROPPED on the live board", "structure",
     "G-ORD-25: DROPPED, but its board 'f1c200s' is not a dropped board",
     lambda d: text_edit(d, "gates.md", "| RETIRED | prod_board |", "| DROPPED | f1c200s |")),
    ("DROPPED: a live check on a dropped board", "structure", "VER-01: board 'esp32s3' was dropped",
     lambda d: csv_edit(d, "verification.csv", "VER-01", board="esp32s3")),
    ("DROPPED: a live requirement verified by a dropped check", "structure",
     "REQ-PRD-05: verified by VER-203, which is DROPPED",
     lambda d: block_field(d, "requirements.md", "REQ-PRD-05", "Verified by", "VER-65, VER-203")),
    ("DROPPED: a live risk detected by a dropped check", "structure", "DF-214: detects VER-205, which is DROPPED",
     lambda d: csv_edit(d, "dfmea.csv", "DF-214", detects=lambda x: x + " VER-205", detection=lambda x: x + "; VER-205")),
    ("DROPPED: a live parent allocated to its dropped child", "structure",
     "REQ-PRD-01: allocated to REQ-PRD-01.R, which is DROPPED",
     lambda d: block_field(d, "requirements.md", "REQ-PRD-01", "Verified by", "allocated: REQ-PRD-01.R")),
    ("DROPPED: a live row cites a dropped row outside 'Was:'", "structure",
     "DF-219: cites DF-148, which is DROPPED",
     lambda d: csv_edit(d, "dfmea.csv", "DF-219", prevention=lambda x: x + "; as DF-148 asked")),
    ("baseline board: an F1C200s PASS on the dropped board's files", "structure",
     "VER-01: PASS for f1c200s, but the design baseline is the dropped esp32s3 board's files",
     lambda d: csv_edit(d, "verification.csv", "VER-01", last_result="PASS", commit="0637562", date="2026-10-28",
                        evidence="hardware/vv/evidence/2026-10-28_VER-01_check_netlist.txt")),
    ("baseline board: README names no baseline board", "structure", "the design baseline names no board",
     lambda d: drop_line(d, "README.md", "- Baseline board:")),
    ("subsystems: a live requirement that names none", "structure", "REQ-PRD-12: a live requirement names its subsystems",
     lambda d: block_field(d, "requirements.md", "REQ-PRD-12", "Subsystem", "")),
    ("subsystems: a live requirement on the dropped board's subsystem", "structure",
     "REQ-PRD-12: subsystem 'mcu' is not one of",
     lambda d: block_field(d, "requirements.md", "REQ-PRD-12", "Subsystem", "mcu display")),
    ("subsystems: a live check on the dropped board's subsystem", "structure", "VER-01: subsystem 'usb_debug' is not one of",
     lambda d: csv_edit(d, "verification.csv", "VER-01", subsystems="power usb_debug")),
    ("new-MCU path: a stage that names a dropped check", "structure", "NM-04: needs VER-203, which is DROPPED",
     lambda d: text_edit(d, "gates.md", "| VER-232 VER-231 VER-235 VER-234 VER-238 |",
                         "| VER-232 VER-203 VER-231 VER-235 VER-234 VER-238 |")),
    ("new-MCU path: a stage whose entry skips the stage before it", "structure",
     "NM-05: its entry needs the stage before it (NM-04)",
     lambda d: text_edit(d, "gates.md", "| NM-04 | VER-01 VER-03", "| NM-03 | VER-01 VER-03")),
    ("new-MCU path: a stage token that is no need", "structure", "NM-03: token 'ISSUE#11' is not an ID",
     lambda d: text_edit(d, "gates.md", "| NM-02 VER-233 |", "| NM-02 VER-233 ISSUE#11 |")),
    ("new-MCU path: the stages out of order", "structure", "number them NM-01 to NM-11 in order",
     lambda d: text_edit(d, "gates.md", "| NM-11 | DVT and PVT", "| NM-12 | DVT and PVT")),
    ("new-MCU path: the chip selection's evidence gone from next_spin_requirements.md", "report",
     "not met: docs/static/next_spin_requirements.md has no line matching",
     lambda d: text_edit(d, "gates.md", "claude-queue/issues/4 |", "claude-queue/issues/999 |")),
    ("new-MCU path: the stage the register is at moves when a stage's exit is met", "report",
     "new-MCU path (gates.md): at stage NM-03",
     lambda d: text_edit(d, "gates.md", "| APPROVED:REQ-PRD VER-189 |", "| DOC:docs/static/DESIGN.md::Mega.Blastoise |")),
    # F's audit of the re-target (2026-10-28): the chip gate's exemption, the stages' entry needs, the dropped
    # defects' lessons, the planned paths
    ("baseline board: a PASS that reads a doc under hardware/ref_design is no dev-board check", "structure",
     "VER-221: PASS for f1c200s, but the design baseline is the dropped esp32s3 board's files",
     lambda d: csv_edit(d, "verification.csv", "VER-221", last_result="PASS", commit=HEAD7, date=TODAY,
                        evidence="hardware/vv/evidence/2026-10-28_VER-221_chip_gate.txt",
                        covers="docs/static/next_spin_requirements.md hardware/ref_design/docs/bringup.md")),
    ("baseline board: a PASS that names a part is no dev-board check", "structure",
     "VER-233: PASS for f1c200s, but the design baseline is the dropped esp32s3 board's files",
     lambda d: csv_edit(d, "verification.csv", "VER-233", last_result="PASS", commit=HEAD7, date=TODAY,
                        evidence="hardware/vv/evidence/2026-10-28_VER-233_datasheets.txt",
                        covers="hardware/ref_design/datasheets/SOURCES.md part:U1")),
    ("new-MCU path: a stage's entry need not met keeps it open", "report",
     "NM-02 Chip selection: done, queue issue #4 (Alex, 2026-10-28: the F1C200s, the V3s as fallback): not met: entry:",
     lambda d: text_edit(d, "gates.md", "| NM-01 | DOC:docs/static/next_spin_requirements.md::^##.Chosen",
                         "| NM-01 DOC:docs/static/DESIGN.md::NO_SUCH_LINE | DOC:docs/static/next_spin_requirements.md::^##.Chosen")),
    ("DROPPED: a dropped defect that names no live DF row its lesson carries to", "structure",
     "DEF-07: DROPPED needs the live DF row its lesson carries to",
     lambda d: text_edit(d, "defects.md", " Its lesson carries as DF-251 (the debug UART's direction in the pin plan) on "
                         "the F1C200s board.", "")),
    ("DROPPED: a dropped defect whose lesson row is itself DROPPED", "structure",
     "DEF-07: DROPPED needs the live DF row its lesson carries to",
     lambda d: text_edit(d, "defects.md", "carries as DF-251 (the debug UART's", "carries as DF-04 (the debug UART's")),
    ("DROPPED: a dropped defect that does not say what it was there", "structure",
     "DEF-07: DROPPED needs what it was there",
     lambda d: text_edit(d, "defects.md", "dropped; it was CLOSED there, neither fixed nor closed on the F1C200s board. "
                         "Its lesson carries as DF-251", "dropped. Its lesson carries as DF-251")),
    ("repo paths: a misspelt path on a live check passes as planned no more", "structure",
     "hardware/ref_design/board/tools/check_netlst.py was never committed",
     lambda d: csv_edit(d, "verification.csv", "VER-01",
                        procedure=lambda x: x + "; then hardware/ref_design/board/tools/check_netlst.py")),
]


def live_baseline(d):
    """2026-10-28: take the baseline's design files as the live board's (README '- Baseline board: `f1c200s`'), so a
    seed can give an F1C200s check a PASS: on the real register they are the dropped ESP32-S3 board's, and the default
    mode refuses one (the seeds test the trace gate's rules, not the design)."""
    p = os.path.join(d, "README.md")
    s = open(p, encoding="utf-8").read()
    open(p, "w", encoding="utf-8").write(s.replace("- Baseline board: `esp32s3`", "- Baseline board: `f1c200s`", 1))


def add_defect(d, did, checks, gate="ORDER"):
    """A live defect of the F1C200s board for a seed (the re-seed left it none of its own)."""
    with open(os.path.join(d, "defects.md"), "a", encoding="utf-8") as f:
        f.write(f"\n### {did}: a seeded defect\n- Board: f1c200s\n- Found: seed_faults.py\n- Symptom: seeded\n"
                f"- Root cause: seeded\n- Why missed: seeded\n- Check added or changed: {checks}\n- Updated: none\n"
                f"- Gate: {gate}\n- Status: OPEN\n")


def drop_line(d, name, prefix):
    """Delete the one line of a file that starts with prefix."""
    path = os.path.join(d, name)
    lines = open(path, encoding="utf-8").read().splitlines(keepends=True)
    keep = [ln for ln in lines if not ln.startswith(prefix)]
    if len(keep) != len(lines) - 1:
        raise SystemExit(f"seed_faults: no single line starting {prefix!r} in {name}")
    open(path, "w", encoding="utf-8").write("".join(keep))


def plan_d_drop(d, vid):
    """Take one check out of PLAN.md section (d) only (it may be named elsewhere in the plan)."""
    path = os.path.join(d, "PLAN.md")
    s = open(path, encoding="utf-8").read()
    m = re.search(r"^## \(d\).*?(?=^## )", s, re.M | re.S)
    if not m or vid not in m.group(0):
        raise SystemExit(f"seed_faults: {vid} not in PLAN.md section (d)")
    sect = re.sub(r"(?<![\w-])" + re.escape(vid) + r"(?![\w-])", vid.replace("-", " "), m.group(0))
    open(path, "w", encoding="utf-8").write(s[:m.start()] + sect + s[m.end():])


def plan_d_table_only(d, vid):
    """O-R11: take one check out of the numbered step rows of PLAN.md section (d) and leave it in the
    'what moved where' table: the old rule counted any VER id in the section, so the table passed for a
    step; only a numbered row counts now."""
    path = os.path.join(d, "PLAN.md")
    s = open(path, encoding="utf-8").read()
    m = re.search(r"^## \(d\).*?(?=^## )", s, re.M | re.S)
    if not m or vid not in m.group(0):
        raise SystemExit(f"seed_faults: {vid} not in PLAN.md section (d)")
    out = []
    for line in m.group(0).splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")] if line.startswith("|") else []
        if cells and re.fullmatch(r"\d+(\.\d+)?", cells[0]):
            line = re.sub(r"(?<![\w-])" + re.escape(vid) + r"(?![\w-])", vid.replace("-", " "), line)
        out.append(line)
    # put the check back into the 'what moved where' table, so it is named only there
    hdr = [i for i, l in enumerate(out) if l.strip().startswith("| Check |")]
    if not hdr:
        raise SystemExit("seed_faults: PLAN.md section (d) has no 'what moved where' table")
    i = hdr[0]
    while i + 1 < len(out) and out[i + 1].startswith("|"):
        i += 1
    out.insert(i + 1, f"| {vid} | ORDER | ORDER | named only in this table, not in a numbered step (O-R11's seeded fault) |")
    open(path, "w", encoding="utf-8").write(s[:m.start()] + "\n".join(out) + "\n" + s[m.end():])


def jlc_fixed_entry(d):
    """F-R4: a JLC_FIXED entry that names an evidence file nobody committed: it must not silence the CPL
    check (the file does not exist)."""
    p = os.path.join(d, "tools", "design_numbers.py")
    s = open(p, encoding="utf-8").read()
    old = "JLC_FIXED = {\n"
    if old not in s:
        raise SystemExit("seed_faults: design_numbers.py's JLC_FIXED moved")
    new = (old + "    \"J1\": {\"evidence\": \"hardware/vv/evidence/2026-10-27_VER-28_jlc_preview.txt\",\n"
           "           \"offset\": \"JLC's library puts J1's origin 1.50 mm behind its pads' centre\"},\n")
    open(p, "w", encoding="utf-8").write(s.replace(old, new, 1))


def borrow_evidence(d, vid, donor):
    """Give an inspection a real evidence file whose COMMIT and RESULT fit, but which is an
    analysis's output: no WHO and no ARTEFACT line (O-R3)."""
    row = next(r for r in csv.DictReader(open(os.path.join(d, "verification.csv"), encoding="utf-8")) if r["id"] == donor)
    if row["last_result"] != "PASS":
        raise SystemExit(f"seed_faults: {donor} must PASS to lend its evidence")
    csv_edit(d, "verification.csv", vid, last_result="PASS", commit=row["commit"], date=row["date"],
             evidence=row["evidence"])


# ---- faults that need git history: run in a scratch clone ----------------------------
def git(repo, *args, env=None):
    r = subprocess.run(["git", "-C", repo] + list(args), capture_output=True, text=True, env=env)
    if r.returncode:
        raise SystemExit(f"seed_faults: git {' '.join(args[:3])} failed in the clone: {r.stderr[-400:]}")
    return r.stdout.strip()


def commit(repo, msg, sign=True, key=None):
    git(repo, "add", "-A")
    pre = ["-c", f"user.signingkey={key}"] if key else []
    git(repo, *pre, "commit", "-q", "--no-verify", "-S" if sign else "--no-gpg-sign", "-m", msg)
    return git(repo, "rev-parse", "HEAD")


KEYS = {}


def setup_signing(repo, tmp):
    """Two throwaway SSH keys in a throwaway allowed-signers file, so the clone's 'Alex' signs as Alex will:
    'alex' is the key README.md pins, 'other' is a second key the file also allows (round 2's objection:
    a good signature from it must not count)."""
    lines = []
    for name in ("alex", "other"):
        key = os.path.join(tmp, f"seed_key_{name}")
        subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-C", name, "-f", key], check=True)
        KEYS[name] = key
        lines.append(f'{name}@example.invalid namespaces="git" {open(key + ".pub").read().strip()}\n')
    allowed = os.path.join(tmp, "allowed_signers")
    open(allowed, "w").write("".join(lines))
    for k, v in (("gpg.format", "ssh"), ("user.signingkey", KEYS["alex"]), ("gpg.ssh.allowedSignersFile", allowed),
                 ("commit.gpgsign", "true")):
        git(repo, "config", k, v)


def pin_key(repo):
    """README's '- Signing key:' line for the clone's 'alex' key (committed with the base, signed by it)."""
    fp = subprocess.run(["ssh-keygen", "-lf", KEYS["alex"] + ".pub"], capture_output=True, text=True).stdout.split()[1]
    text_edit(os.path.join(repo, "hardware/vv"), "README.md", "Pinned key (Alex writes the line",
              f"- Signing key: `{fp}`\n\nPinned key (Alex writes the line")


def rebaseline(repo, move_base=True):
    """Write HEAD's object hashes into the clone's README baseline (after a design commit).  A real re-seed
    also names HEAD as the baseline commit (F-R3: the hashes must equal that commit's blobs), so move_base
    does too; the F-R3 fault leaves the old baseline commit in place, as a hash rewritten from HEAD would."""
    p = os.path.join(repo, "hardware/vv/README.md")
    s = open(p, encoding="utf-8").read()
    for path in re.findall(r"^- `(hardware/\S+)` [0-9a-f]{40}\s*$", s, re.M):
        s = re.sub(r"^- `" + re.escape(path) + r"` [0-9a-f]{40}", f"- `{path}` " + git(repo, "rev-parse", f"HEAD:{path}"),
                   s, flags=re.M)
    if move_base:
        head = git(repo, "rev-parse", "--short=7", "HEAD")
        s = re.sub(r"^(- Order branch: `[^`]+` from `)[0-9a-f]+(`)", lambda m: m.group(1) + head + m.group(2), s,
                   count=1, flags=re.M)
    open(p, "w", encoding="utf-8").write(s)


def fake_pass(repo, vid, base, extra=""):
    """Record a PASS for vid at commit base with a header that check_trace accepts (the baseline taken as the live
    board's: an F1C200s check may not pass on a dropped board's files)."""
    live_baseline(os.path.join(repo, "hardware/vv"))
    rel = f"hardware/vv/evidence/{TODAY}_{vid}_seeded.txt"
    open(os.path.join(repo, rel), "w").write(f"# {vid} seeded\nCOMMIT: {base}\nTOOLS: seed_faults\n{extra}RESULT: PASS\n")
    csv_edit(os.path.join(repo, "hardware/vv"), "verification.csv", vid, last_result="PASS", evidence=rel,
             commit=base[:7], date=TODAY, needs="")


def reopen_closed(repo):
    """Set every CLOSED defect to FIXED in the clone: a fault that makes evidence stale on
    purpose would otherwise also trip 'CLOSED but its check is stale', which is not what it tests."""
    p = os.path.join(repo, "hardware/vv/defects.md")
    s = re.sub(r"^- Status: CLOSED\b", "- Status: FIXED (seeded fault setup)", open(p, encoding="utf-8").read(), flags=re.M)
    open(p, "w", encoding="utf-8").write(s)


def g_r6(repo, base):
    open(os.path.join(repo, "hardware/vv/releases/REL-REFA-R1-EVT.md"), "w").write(
        REL.format(case="b" * 64, id="REL-REFA-R1-EVT", bom="a" * 64, commit=base))
    commit(repo, "release record")
    with open(os.path.join(repo, "hardware/ref_design/board/tools/silk.py"), "a") as f:
        f.write("# a silk change after the release\n")
    commit(repo, "silk change")
    rebaseline(repo)
    commit(repo, "baseline")
    return "order"


def g_expired_accept(repo, base):
    approve(os.path.join(repo, "hardware/vv"), "| A-001 | DF-214 | ACCEPTED | ORDER | for the order only | Alex | 2026-10-25 |")
    csv_edit(os.path.join(repo, "hardware/vv"), "dfmea.csv", "DF-214", status="ACCEPTED")
    commit(repo, "accept DF-214 for ORDER")
    return "evt"


def g_signed_waiver(repo, base):
    # G-ORD-06 needs VER-239 since 2026-10-28 (the power slider, not built), so the criterion fails and the waiver acts
    approve(os.path.join(repo, "hardware/vv"), "| A-001 | W-01 | WAIVED | ORDER | the latch replica is cut | Alex | 2026-10-25 |")
    commit(repo, "waive W-01")
    return "order"


def g_claude_waiver(repo, base):
    approve(os.path.join(repo, "hardware/vv"), "| A-001 | W-01 | WAIVED | ORDER | the latch replica is cut | Alex | 2026-10-25 |")
    commit(repo, "waive W-01\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>")
    return None


def g_part_stale(repo, base):
    """A design.py part changed after the evidence: the evidence is stale (the part: cover).

    The baseline names the design as it stands at the new commit (F-R3: the register may not be rebaselined
    from HEAD to hide a design change), so the register itself is sound and only the evidence is stale."""
    reopen_closed(repo)
    fake_pass(repo, "VER-99", base)
    # the F1C200s re-seed took the dropped board's part: covers off its carried rows; this seed puts C1's back
    csv_edit(os.path.join(repo, "hardware/vv"), "verification.csv", "VER-99", covers=lambda c: c + " part:C1")
    commit(repo, "VER-99 passes")
    p = os.path.join(repo, "hardware/ref_design/board/design.py")
    s = open(p).read()
    s = s.replace('C("C1", S, "10u 25V", "VBUS_C1")', 'C("C1", S, "22u 25V", "VBUS_C1")', 1)
    open(p, "w").write(s)
    commit(repo, "C1 to 22u")
    rebaseline(repo)
    commit(repo, "the baseline names the design as it stands at this commit")
    return None


def g_dirty(repo, base):
    fake_pass(repo, "VER-99", base)
    commit(repo, "VER-99 passes")
    with open(os.path.join(repo, "hardware/vv/tools/design_numbers.py"), "a") as f:
        f.write("# uncommitted\n")
    return "order"


def g_deferral_ends(repo, base):
    approve(os.path.join(repo, "hardware/vv"), "| A-001 | REQ-PRD-05 | DEFERRED | ORDER | the cell's numbers first | Alex | 2026-10-25 |")
    block_field(os.path.join(repo, "hardware/vv"), "requirements.md", "REQ-PRD-05", "Status", "DEFERRED")
    commit(repo, "defer REQ-PRD-05 to ORDER")
    return "evt"


def g_dvt_release(repo, base):
    sha = hashlib.sha256(b"seeded fab zip").hexdigest()
    open(os.path.join(repo, "hardware/vv/releases/REL-REFA-R1-EVT.md"), "w").write(
        REL.format(case="b" * 64, id="REL-REFA-R1-EVT", bom="a" * 64, commit=base) + f"- Fab zip sha256: {sha}\n")
    fake_pass(repo, "VER-93", base, extra=f"RELEASE: REL-REFA-R1-EVT\nUNITS: 4\nFAB: {sha}\n")
    commit(repo, "VER-93 on an EVT unit")
    return "dvt"


def g_untracked(repo, base):
    open(os.path.join(repo, "hardware/vv/evidence/untracked_note.txt"), "w").write("not committed\n")
    csv_edit(os.path.join(repo, "hardware/vv"), "verification.csv", "VER-01",
             covers=lambda c: c + " hardware/vv/evidence/untracked_note.txt")
    return None


def g_statement_edited(repo, base):
    approve(os.path.join(repo, "hardware/vv"), "| A-001 | REQ-PRD-05 | APPROVED | - | - | Alex | 2026-10-25 |")
    block_field(os.path.join(repo, "hardware/vv"), "requirements.md", "REQ-PRD-05", "Status", "APPROVED")
    commit(repo, "approve REQ-PRD-05")
    block_field(os.path.join(repo, "hardware/vv"), "requirements.md", "REQ-PRD-05", "Statement",
                "Switched off with a full cell and no USB, the board shall drain the cell by no more than 60 uA.")
    commit(repo, "loosen REQ-PRD-05")
    return None


def g_result_disagrees(repo, base):
    fake_pass(repo, "VER-99", base)
    csv_edit(os.path.join(repo, "hardware/vv"), "verification.csv", "VER-99", last_result="FAIL")
    return None


def g_expired(repo, base):
    reopen_closed(repo)
    fake_pass(repo, "VER-99", base)
    csv_edit(os.path.join(repo, "hardware/vv"), "verification.csv", "VER-99", valid_days="30", date="2026-01-01")
    return None


def g_unsigned_approval(repo, base):
    approve(os.path.join(repo, "hardware/vv"), "| A-001 | REQ-PRD-17 | APPROVED | - | - | Alex | 2026-10-25 |")
    block_field(os.path.join(repo, "hardware/vv"), "requirements.md", "REQ-PRD-17", "Status", "APPROVED")
    commit(repo, "approve REQ-PRD-17", sign=False)
    return None


def g_merge_approval(repo, base):
    git(repo, "checkout", "-q", "-b", "seeded-side")
    with open(os.path.join(repo, "hardware/vv/PLAN.md"), "a") as f:
        f.write("\nside\n")
    commit(repo, "side")
    git(repo, "checkout", "-q", "-")
    git(repo, "merge", "-q", "--no-ff", "--no-commit", "seeded-side")
    approve(os.path.join(repo, "hardware/vv"), "| A-001 | REQ-PRD-17 | APPROVED | - | - | Alex | 2026-10-25 |")
    block_field(os.path.join(repo, "hardware/vv"), "requirements.md", "REQ-PRD-17", "Status", "APPROVED")
    commit(repo, "Merge branch 'seeded-side'")
    git(repo, "branch", "-q", "-D", "seeded-side")
    return None


def g_uncommitted_register(repo, base):
    """O-R2's attack: a check's result and its evidence's RESULT edited together, left uncommitted (2026-10-28: VER-220
    PASS to FAIL; the F1C200s re-seed left no FAIL to turn into a PASS, and the rule reads either way)."""
    d = os.path.join(repo, "hardware/vv")
    row = next(r for r in csv.DictReader(open(os.path.join(d, "verification.csv"), encoding="utf-8")) if r["id"] == "VER-220")
    p = os.path.join(repo, row["evidence"].split()[0])
    s = open(p).read()
    open(p, "w").write(re.sub(r"^RESULT: PASS", "RESULT: FAIL", s, flags=re.M))
    csv_edit(d, "verification.csv", "VER-220", last_result="FAIL")
    return "order"


def g_uncommitted_evidence(repo, base):
    """A PASS whose row is committed but whose evidence file never was."""
    fake_pass(repo, "VER-99", base)
    git(repo, "add", "hardware/vv/verification.csv")
    git(repo, "commit", "-q", "--no-verify", "-m", "VER-99 passes (evidence left out)")
    return "order"


def g_edited_evidence(repo, base):
    """A PASS whose evidence was edited after its commit."""
    fake_pass(repo, "VER-99", base)
    commit(repo, "VER-99 passes")
    with open(os.path.join(repo, f"hardware/vv/evidence/{TODAY}_VER-99_seeded.txt"), "a") as f:
        f.write("# touched up afterwards\n")
    return "order"


def g_evt_fai_at_pvt(repo, base):
    sha = hashlib.sha256(b"seeded fab zip").hexdigest()
    open(os.path.join(repo, "hardware/vv/releases/REL-REFA-R1-EVT.md"), "w").write(
        REL.format(case="b" * 64, id="REL-REFA-R1-EVT", bom="a" * 64, commit=base) + f"- Fab zip sha256: {sha}\n")
    fake_pass(repo, "VER-109", base, extra=f"RELEASE: REL-REFA-R1-EVT\nUNITS: 1\nFAB: {sha}\nWHO: Alex\n"
                                           "ARTEFACT: the FAI sheet of unit 1\n")
    commit(repo, "VER-109 on EVT unit 1")
    return "pvt"


def g_order_branch_moves(repo, base):
    """O-R13: a design commit on the order branch after the baseline commit, before any merge."""
    b = re.search(r"^- Order branch: `[^`]+` from `([0-9a-f]+)`", open(os.path.join(repo, "hardware/vv/README.md")).read(),
                  re.M).group(1)
    wt = os.path.join(os.path.dirname(repo), "order_wt")
    git(repo, "worktree", "add", "-q", "-b", "seeded-order", wt, b)
    with open(os.path.join(wt, "hardware/ref_design/board/tools/silk.py"), "a") as f:
        f.write("# a silk change on the order branch\n")
    commit(wt, "silk change on the order branch")
    name_order_branch(os.path.join(repo, "hardware/vv"), "seeded-order")
    commit(repo, "README names the seeded order branch")
    return "order"


def g_order_branch_case(repo, base):
    """2026-10-26: a case.scad commit on the order branch after the baseline holds the case gate (R8, as R7)."""
    b = re.search(r"^- Order branch: `[^`]+` from `([0-9a-f]+)`", open(os.path.join(repo, "hardware/vv/README.md")).read(),
                  re.M).group(1)
    wt = os.path.join(os.path.dirname(repo), "order_wt_case")
    git(repo, "worktree", "add", "-q", "-b", "seeded-order-case", wt, b)
    with open(os.path.join(wt, "hardware/ref_design/case/case.scad"), "a") as f:
        f.write("// a case change on the order branch\n")
    commit(wt, "case change on the order branch")
    name_order_branch(os.path.join(repo, "hardware/vv"), "seeded-order-case")
    commit(repo, "README names the seeded order branch")
    return "case"


def g_chg_omits(repo, base):
    """O-R21: a change record whose 'What to re-run' leaves out a bench test tied to the part."""
    open(os.path.join(repo, "hardware/vv/releases/REL-REFA-R1-EVT.md"), "w").write(
        REL.format(case="b" * 64, id="REL-REFA-R1-EVT", bom="a" * 64, commit=base))
    commit(repo, "release record")
    p = os.path.join(repo, "hardware/ref_design/board/design.py")
    s = open(p).read()
    s = s.replace('C("C1", S, "10u 25V", "VBUS_C1")', 'C("C1", S, "22u 25V", "VBUS_C1")', 1)
    open(p, "w").write(s)
    sha = commit(repo, "C1 to 22u")
    rebaseline(repo)
    # 2026-10-28: no live F1C200s row names a part yet, so a DFMEA row ties C1 to the hot-plug bench test here
    csv_edit(os.path.join(repo, "hardware/vv"), "dfmea.csv", "DF-224", item="VBUS capacitance (C1)",
             detects=lambda x: x + " VER-71", detection=lambda x: x + "; VER-71 the hot plug on the bench")
    open(os.path.join(repo, "hardware/vv/releases/CHG-01-c1.md"), "w").write(
        f"# CHG-01: C1 to 22u\n\n- Date: {TODAY}\n- Author: Alex\n- Commit(s): {sha[:7]}\n- Reason: seeded\n\n"
        "## What to re-run\n\n| Check | Why | New result | Commit | Date |\n|---|---|---|---|---|\n"
        "| VER-99 | inrush | | | |\n")
    commit(repo, "baseline and CHG-01")
    return None


def g_fab_comment(repo, base):
    """O-R1's attack: fab.py's text cannot pass the CPL or drill check.  Since 71737ae fab.py writes the pads' centre
    and separate drill files, so both pass; here the code goes back to KiCad's origin and one mixed drill file while
    a comment claims the fixes are still in: both checks must fail, as they read what fab.py writes."""
    p = os.path.join(repo, "hardware/ref_design/board/tools/fab.py")
    s = open(p).read()
    olds = ('"--excellon-separate-th", ', 'rows.append([ref, f"{cx:.3f}mm", f"{-cy:.3f}mm", side,')
    if not all(o in s for o in olds):
        raise SystemExit("seed_faults: fab.py's drill or CPL line moved")
    s = s.replace(olds[0], "", 1).replace(
        olds[1], 'rows.append([ref, f"{pcbnew.ToMM(pos.x):.3f}mm", f"{-pcbnew.ToMM(pos.y):.3f}mm", side,', 1)
    open(p, "w").write(s + "# --excellon-separate-th is used and the CPL carries the pads' centre\n")
    return ("dn", [["cpl"], ["drill"]])


def g_tp_near_hole(repo, base):
    """O-R14: a back-side test point (made in a loop, not a literal put()) moved next to the hole,
    in memory: no KiCad file is written."""
    code = ("import sys; sys.path.insert(0, 'hardware/vv/tools'); import design_numbers as dn\n"
            "pcbnew, b = dn.board(); x0, y0, x1, y1 = dn.cutouts()\n"
            "fp = [f for f in b.GetFootprints() if f.GetReference() == 'TP11'][0]\n"
            "assert fp.IsFlipped()\n"
            "fp.SetPosition(pcbnew.VECTOR2I(pcbnew.FromMM(x1 + 0.6), pcbnew.FromMM((y0 + y1) / 2)))\n"
            "sys.exit(dn.th1(b))\n")
    return ("py", code)


def g_body_past_edge(repo, base):
    """2026-10-27: VER-18 prints a courtyard past the outline but fails a pad or a body past it: J2 moved 1.0 mm
    towards the right edge, in memory (no KiCad file is written), puts its mounting pad past the outline."""
    code = ("import sys; sys.path.insert(0, 'hardware/vv/tools'); import design_numbers as dn\n"
            "pcbnew, b = dn.board()\n"
            "fp = [f for f in b.GetFootprints() if f.GetReference() == 'J2'][0]\n"
            "p = fp.GetPosition(); fp.SetPosition(pcbnew.VECTOR2I(p.x + pcbnew.FromMM(1.0), p.y))\n"
            "sys.exit(dn.edge(b))\n")
    return ("py", code)


def g_other_key(repo, base):
    """Round 2: a commit signed by another key in the allowed-signers file gives %G? = G, and must not count."""
    approve(os.path.join(repo, "hardware/vv"), "| A-001 | REQ-PRD-17 | APPROVED | - | - | Alex | 2026-10-26 |")
    block_field(os.path.join(repo, "hardware/vv"), "requirements.md", "REQ-PRD-17", "Status", "APPROVED")
    commit(repo, "approve REQ-PRD-17 (signed with the other key)", key=KEYS["other"])
    return None


def g_bulk_edited(repo, base):
    """Round 2: a bulk 'REQ-ALL@<commit>' line covers a requirement only while its statement is unchanged."""
    approve(os.path.join(repo, "hardware/vv"), f"| A-001 | REQ-ALL@{base[:7]} | APPROVED | - | - | Alex | 2026-10-26 |")
    block_field(os.path.join(repo, "hardware/vv"), "requirements.md", "REQ-PRD-17", "Status", "APPROVED")
    commit(repo, "approve every requirement as worded at the base")
    block_field(os.path.join(repo, "hardware/vv"), "requirements.md", "REQ-PRD-17", "Statement",
                "The closed case shall measure at most 80 x 160 mm in plan.")
    commit(repo, "loosen REQ-PRD-17")
    return None


def g_cellgap_right_end(repo, base):
    """Round 2, restated at 39cc07f: the hole's left side back at 71737ae's -18.9 (the 0.4 mm on the left that C1's
    fix added lost) leaves the pocket 36.0 mm wide, the worst-case cell's own width, so VER-181 fails on the fit."""
    p = os.path.join(repo, "hardware/ref_design/case/case.scad")
    s = open(p).read()
    old = "cell_hole = [[-19.3, -8.2 + cell_dy], [18.9, 47.0 + cell_dy]]"
    if old not in s:
        raise SystemExit("seed_faults: case.scad's cell_hole line moved")
    open(p, "w").write(s.replace(old, "cell_hole = [[-18.9, -8.2 + cell_dy], [18.9, 47.0 + cell_dy]]", 1))
    return ("dn", [["cellgap"]])


def g_cell_fit(repo, base):
    """Round 3's sign-off (O-S1): a cell 0.1 mm too long for its pocket's fit fails VER-181, though every gap to the
    FR4 passes: the pocket printed 0.1 mm small must still take it with 0.1 mm a side (since 39cc07f the
    pocket is 52.4 x 36.4, so a 52.1 x 36.1 cell)."""
    p = os.path.join(repo, "hardware/ref_design/case/case.scad")
    s = open(p).read()
    old = "cell = [52, 36, 4.5];"
    if old not in s:
        raise SystemExit("seed_faults: case.scad's cell line moved")
    open(p, "w").write(s.replace(old, "cell = [52.1, 36.1, 4.5];", 1))
    return ("dn", [["cellgap"]])


def g_cell_fit_control(repo, base):
    """O-S1's control: a cell spec of 51.6 x 35.6 mm (what cellgap says this hole takes) passes VER-181."""
    p = os.path.join(repo, "hardware/ref_design/case/case.scad")
    s = open(p).read()
    old = "cell = [52, 36, 4.5];"
    if old not in s:
        raise SystemExit("seed_faults: case.scad's cell line moved")
    open(p, "w").write(s.replace(old, "cell = [51.6, 35.6, 4.5];", 1))
    return ("dn_pass", [["cellgap"]])


def g_moves_listed(repo, base):
    """Round 3's sign-off (O-S5): with no GATE@ line signed, the order gate lists a check moved later since the
    register state README names (G0), so Alex sees it before he freezes the phases."""
    csv_edit(os.path.join(repo, "hardware/vv"), "verification.csv", "VER-138", phase="DVT")
    commit(repo, "VER-138 moves to DVT")
    return "order"


def g_status_listed(repo, base):
    """O-R1: with no GATE@ line signed, a defect taken from OPEN to FIXED since the register state README names is
    listed by the order gate (G0), like a Gate move: FIXED takes it off R4 at ORDER, CASE and EVT."""
    block_field(os.path.join(repo, "hardware/vv"), "defects.md", "DEF-47", "Status", "FIXED (seeded)")
    commit(repo, "DEF-47 FIXED")
    return "order"


def g_status_freeze(repo, base):
    """O-R1: a defect taken from OPEN to FIXED after Alex's signed GATE@ line fails the order gate (G1)."""
    approve(os.path.join(repo, "hardware/vv"),
            f"| A-001 | GATE@{base} | FROZEN | ORDER | the order gate state at this commit | Alex | {TODAY} |")
    commit(repo, "freeze the gate state")
    block_field(os.path.join(repo, "hardware/vv"), "defects.md", "DEF-47", "Status", "FIXED (seeded)")
    commit(repo, "DEF-47 FIXED")
    return "order"


def g_case_move(repo, base):
    """O-R4: a case.scad commit on the order branch that moves a button position fails the order gate: the
    positions.json regenerated at that commit differs from the one at the baseline commit."""
    b = re.search(r"^- Order branch: `[^`]+` from `([0-9a-f]+)`", open(os.path.join(repo, "hardware/vv/README.md")).read(),
                  re.M).group(1)
    wt = os.path.join(os.path.dirname(repo), "order_wt_move")
    git(repo, "worktree", "add", "-q", "-b", "seeded-case-move", wt, b)
    p = os.path.join(wt, "hardware/ref_design/case/case.scad")
    s = open(p).read()
    old = "ss_pitch = 7.5;"
    if old not in s:
        raise SystemExit("seed_faults: case.scad's ss_pitch line moved")
    open(p, "w").write(s.replace(old, "ss_pitch = 7.4;", 1))
    commit(wt, "the Start/Select pills 0.05 mm closer together")
    name_order_branch(os.path.join(repo, "hardware/vv"), "seeded-case-move")
    commit(repo, "README names the seeded case branch")
    return "order"


def g_case_labels(repo, base):
    """O-R4 (control): a case-only commit that moves no position (like 2ddb7fe's engraved labels, which is history
    since the baseline moved past it: a comment in case.scad here) passes that part of R8 at the order gate: the
    two positions.json files hash the same."""
    b = re.search(r"^- Order branch: `[^`]+` from `([0-9a-f]+)`", open(os.path.join(repo, "hardware/vv/README.md")).read(),
                  re.M).group(1)
    wt = os.path.join(os.path.dirname(repo), "order_wt_labels")
    git(repo, "worktree", "add", "-q", "-b", "seeded-case-labels", wt, b)
    with open(os.path.join(wt, "hardware/ref_design/case/case.scad"), "a") as f:
        f.write("// a case-only change that moves no board position\n")
    commit(wt, "a case.scad comment on the order branch")
    name_order_branch(os.path.join(repo, "hardware/vv"), "seeded-case-labels")
    commit(repo, "README names the labels-only branch")
    return "order"


def g_gate_freeze(repo, base):
    """O-R5: a check moved after Alex's signed GATE@ line fails the order gate (G1)."""
    approve(os.path.join(repo, "hardware/vv"),
            f"| A-001 | GATE@{base} | FROZEN | ORDER | the order gate state at this commit | Alex | {TODAY} |")
    commit(repo, "freeze the gate state")
    csv_edit(os.path.join(repo, "hardware/vv"), "verification.csv", "VER-138", phase="DVT")
    commit(repo, "VER-138 moves to DVT")
    return "order"


def g_gate_freeze_advisory(repo, base):
    """O-R5: a check made advisory in a criterion after Alex's signed GATE@ line fails the order gate (G1): the
    criterion gains ADVISORY:VER-33, which weakens what the gate counts."""
    approve(os.path.join(repo, "hardware/vv"),
            f"| A-001 | GATE@{base} | FROZEN | ORDER | the order gate state at this commit | Alex | {TODAY} |")
    commit(repo, "freeze the gate state")
    text_edit(os.path.join(repo, "hardware/vv"), "gates.md", "| VER-235 VER-231 ", "| ADVISORY:VER-235 VER-231 ")
    commit(repo, "VER-235 advisory at the order gate")
    return "order"


def g_software_key(repo, base):
    """O-R12: the clone's pinned key is a plain software key, so the order gate fails K1."""
    return "order"


def g_software_key_accept(repo, base):
    """O-R12: --accept-software-key lets the gate pass K1 and prints the choice for the release record."""
    return {"gate": "order", "args": ["--accept-software-key", "--verbose"]}


def g_key_exception(repo, base):
    """O-R12: a line in APPROVALS.md that tries to grant the software-key exception does not count."""
    fp = re.search(r"^- Signing key: `(SHA256:[A-Za-z0-9+/]+)`",
                   open(os.path.join(repo, "hardware/vv/README.md")).read(), re.M).group(1)
    approve(os.path.join(repo, "hardware/vv"),
            f"| A-001 | KEY@{fp} | ACCEPTED | ORDER | a software key is fine for this order | Alex | {TODAY} |")
    commit(repo, "a line that tries to grant the software-key exception")
    return "order"


def g_baseline_rewritten(repo, base):
    """F-R3: README's baseline hash rewritten from HEAD after a design.py edit must fail: the register
    describes a design the baseline commit does not hold."""
    p = os.path.join(repo, "hardware/ref_design/board/design.py")
    s = open(p).read()
    if "SEEDED_FAULT" in s:
        raise SystemExit("seed_faults: design.py already carries the seeded marker")
    open(p, "w").write(s + "\n# SEEDED_FAULT: a design.py edit after the baseline commit\n")
    commit(repo, "a design.py edit after the baseline")
    rebaseline(repo, move_base=False)
    commit(repo, "README's baseline rewritten from HEAD")
    return None


def g_jlc_fixed_uncommitted(repo, base):
    """F-R4: a JLC_FIXED entry whose evidence file is not committed does not count."""
    p = os.path.join(repo, "hardware/vv/tools/design_numbers.py")
    s = open(p).read()
    old = "JLC_FIXED = {\n"
    if old not in s:
        raise SystemExit("seed_faults: design_numbers.py's JLC_FIXED moved")
    new = (old + "    \"J1\": {\"evidence\": \"hardware/vv/evidence/2026-10-27_VER-28_jlc_preview.txt\",\n"
           "           \"offset\": \"JLC's library puts J1's origin 1.50 mm behind its pads' centre\"},\n")
    open(p, "w").write(s.replace(old, new, 1))
    commit(repo, "JLC_FIXED names the dry run's preview")
    open(os.path.join(repo, "hardware/vv/evidence/2026-10-27_VER-28_jlc_preview.txt"), "w").write(
        "# VER-28's placement preview\nARTEFACT: the dry run's screenshot of J1 on its pads\n")
    return None


def g_grid_accepted(repo, base):
    """A gap Alex accepts with a signed GRID-<class>-<subsystem> line leaves the order gate's R9 (a control: the
    unsigned acceptance is a FAULTS entry)."""
    vv = os.path.join(repo, "hardware/vv")
    text_edit(vv, "grid_reasons.csv", "3,case,NOT_NEEDED,", "3,case,ACCEPTED_GAP,")
    approve(vv, "| A-001 | GRID-3-case | ACCEPTED | ORDER | the case has no wiring | Alex | 2026-10-27 |")
    commit(repo, "accept the case's class 3 gap")
    return "order"


def g_accept_unsigned(repo, base):
    """1. (2026-10-28): a defect ACCEPTED on a DF-ACCEPT line committed without Alex's signature fails the default
    mode: an unsigned acceptance moves nothing."""
    vv = os.path.join(repo, "hardware/vv")
    approve(vv, f"| A-001 | DF-ACCEPT:DEF-47 | ACCEPTED | EVT | decided at EVT | Alex | {TODAY} |")
    block_field(vv, "defects.md", "DEF-47", "Status", "ACCEPTED (to EVT, A-001)")
    commit(repo, "DEF-47 accepted to EVT, unsigned", sign=False)
    return None


def g_accept_signed(repo, base):
    """1. (control): Alex's signed DF-ACCEPT line moves DEF-84 (Gate ORDER) to EVT: R4 lists it beside its result,
    not in the OPEN list, G-ORD-02 lets VER-125 (a check DEF-84 names) through as ACCEPTED, and G0 does not list
    the move (it is his own signed one). Since 39cc07f DEF-62's VER-181 passes, so DEF-84 (VER-125 FAIL) stands in."""
    vv = os.path.join(repo, "hardware/vv")
    # 2026-10-28: DEF-84 was DROPPED with the ESP32-S3 board; a seeded F1C200s defect naming VER-125 stands in
    add_defect(vv, "DEF-103", "VER-125")
    approve(vv, f"| A-001 | DF-ACCEPT:DEF-103 | ACCEPTED | EVT | decided at EVT | Alex | {TODAY} |")
    block_field(vv, "defects.md", "DEF-103", "Status", "ACCEPTED (to EVT, A-001)")
    commit(repo, "DEF-103 accepted to EVT")
    return "order"


def g_accept_outside(repo, base):
    """1.: a signed acceptance lets through only the checks its defect names.  DEF-37 (VER-164, VER-87) accepted to
    EVT: G-ORD-07 shows VER-164 ACCEPTED, and VER-125, which DEF-37 does not name, still FAIL."""
    vv = os.path.join(repo, "hardware/vv")
    # 2026-10-28: DEF-37 was DROPPED with the ESP32-S3 board; a seeded F1C200s defect naming VER-164 and VER-87 stands in
    add_defect(vv, "DEF-104", "VER-164, VER-87")
    approve(vv, f"| A-001 | DF-ACCEPT:DEF-104 | ACCEPTED | EVT | the pad is a case part | Alex | {TODAY} |")
    block_field(vv, "defects.md", "DEF-104", "Status", "ACCEPTED (to EVT, A-001)")
    commit(repo, "DEF-104 accepted to EVT")
    return "order"


def g_proposal_branch(repo, base):
    """3. (2026-10-28): a commit on the order branch after the baseline that touches docs/living/*proposal*.md (dcb6a76
    chose another MCU there): the default mode warns, the order gate fails R8."""
    b = re.search(r"^- Order branch: `[^`]+` from `([0-9a-f]+)`", open(os.path.join(repo, "hardware/vv/README.md")).read(),
                  re.M).group(1)
    wt = os.path.join(os.path.dirname(repo), "order_wt_proposal")
    if not os.path.isdir(wt):
        git(repo, "worktree", "add", "-q", "-b", "seeded-proposal", wt, b)
        with open(os.path.join(wt, "docs/living/next_revision_proposal.md"), "a") as f:
            f.write("\nSeeded: the next board takes another MCU.\n")
        commit(wt, "a proposal doc changes the architecture on the order branch")
    name_order_branch(os.path.join(repo, "hardware/vv"), "seeded-proposal")
    commit(repo, "README names the seeded proposal branch")
    return "order"


def g_proposal_branch_default(repo, base):
    g_proposal_branch(repo, base)
    return None


def g_grid_signed_no_row(repo, base):
    """4. (2026-10-28, step 9.5): a GRID line Alex signed with no ACCEPTED_GAP row in grid_reasons.csv does not
    clear the cell, and the order gate says why."""
    approve(os.path.join(repo, "hardware/vv"),
            f"| A-001 | GRID-1-power | ACCEPTED | ORDER | read against the functions by hand | Alex | {TODAY} |")
    commit(repo, "accept the power subsystem's class 1 gap, with no reason row")
    return "order"


GIT_FAULTS = [
    ("row result disagrees with its evidence", "structure", "the row says FAIL", g_result_disagrees),
    ("evidence past its valid_days", "report", "expired", g_expired),
    ("R6: a routed-board script changed after a release with no change record", "gate", "unrecorded design change", g_r6),
    ("an acceptance used past its scope", "gate", "DF-214 accepted only up to ORDER", g_expired_accept),
    ("a waiver Alex signed meets its criterion", "gate", "waived by Alex", g_signed_waiver),
    ("a waiver signed in a commit with a Claude trailer", "structure", "carries a Claude trailer", g_claude_waiver),
    ("staleness through a part: cover", "report", "design.py parts C1", g_part_stale),
    ("R5: an uncommitted change under passed evidence", "gate", "FAIL  R5", g_dirty),
    ("a deferral used past its scope", "gate", "deferral ended before EVT", g_deferral_ends),
    ("an EVT unit's result at the DVT gate", "gate", "VER-93 covers no DVT release", g_dvt_release),
    ("a covers glob met only by an untracked file", "structure", "untracked_note.txt' matches no tracked file", g_untracked),
    ("a statement edited after its approval", "structure", "its statement changed after Alex approved it", g_statement_edited),
    # the second review (2026-10-25)
    ("O-R4: an approval line in an unsigned plain commit", "structure", "A-001 pending: not signed with Alex's key",
     g_unsigned_approval),
    ("O-R4: an approval line written in a merge commit", "structure", "comes from a merge commit", g_merge_approval),
    ("O-R2: uncommitted register and evidence edits (VER-138 FAIL to PASS)", "gate", "uncommitted register changes",
     g_uncommitted_register),
    ("O-R2: a PASS whose evidence was never committed", "gate", "VER-99 evidence hardware/vv/evidence/",
     g_uncommitted_evidence),
    ("O-R2: a PASS whose evidence was edited after its commit", "gate", "not committed, or changed since",
     g_edited_evidence),
    ("O-R11: an EVT first article at the PVT gate", "gate", "VER-109 covers no PVT release", g_evt_fai_at_pvt),
    ("O-R13: a design commit on the order branch after the baseline", "gate",
     "seeded-order has design commits after", g_order_branch_moves),
    ("O-R21: a change record that omits a bench test of the changed part", "structure",
     "CHG-01: 'What to re-run' omits VER-71", g_chg_omits),
    ("O-R1: a comment in fab.py does not pass the CPL or drill check", "dn", "RESULT: FAIL", g_fab_comment),
    ("O-R14: a back-side test point next to the cell's hole fails th1", "dn",
     "back-side pads within 1 mm of the hole: TP11", g_tp_near_hole),
    # round 2's sign-off objections (2026-10-26)
    ("round 2 (O): an approval signed by another key in the allowed-signers file", "structure",
     "A-001 pending: signed with another key", g_other_key),
    ("round 2 (O): a bulk REQ-ALL line does not cover a statement edited after it", "structure",
     "REQ-PRD-17: Status APPROVED but releases/APPROVALS.md has no committed line for it", g_bulk_edited),
    ("round 2 (O, F): the cell hole's left side back at 71737ae's fails VER-181 on the fit", "dn", "RESULT: FAIL",
     g_cellgap_right_end),
    ("R8: a case.scad commit on the order branch holds the case gate", "gate",
     "seeded-order-case has design commits after", g_order_branch_case),
    # round 3's findings (2026-10-26)
    ("F-R3: README's baseline hash rewritten from HEAD after a design.py edit", "structure",
     "design baseline differs from the baseline commit", g_baseline_rewritten),
    ("F-R4: a JLC_FIXED entry whose evidence file is not committed does not count", "structure",
     "JLC_FIXED[J1]: the evidence file hardware/vv/evidence/2026-10-27_VER-28_jlc_preview.txt is not committed",
     g_jlc_fixed_uncommitted),
    ("O-R4: a case.scad commit that moves a button position fails the order gate", "gate",
     "case commit moves board positions", g_case_move),
    ("O-R4: a case-only commit that moves no position passes that part of R8", "gate_no",
     "move no board position", g_case_labels, "case commit moves board positions"),
    ("O-R5: a check moved after Alex's signed GATE@ line fails the order gate", "gate",
     "moved after A-001", g_gate_freeze),
    ("O-R5: a check made advisory after Alex's signed GATE@ line fails the order gate", "gate",
     "G-ORD-18 gained ADVISORY:VER-235", g_gate_freeze_advisory),
    ("O-R12: a software signing key fails the order gate", "gate",
     "FAIL  K1 the pinned signing key is a hardware key", g_software_key),
    ("O-R12: --accept-software-key passes K1 and prints the choice", "gate_line",
     "PASS  K1 the pinned signing key is a hardware key", g_software_key_accept),
    ("O-R12: a line in APPROVALS.md that grants the exception does not count", "gate",
     "FAIL  K1 the pinned signing key is a hardware key", g_key_exception),
    # the coverage grid (2026-10-27)
    ("grid R9: a gap Alex accepts in a signed GRID line leaves R9", "gate_no",
     "R9 the coverage grid", g_grid_accepted, "3 case no check"),
    # round 3's sign-off objections, applied at the 71737ae re-seed (2026-10-27)
    ("O-S1: a cell 0.1 mm too big for its pocket's fit fails VER-181 though every FR4 gap passes", "dn",
     "in the pocket's length: 0.30 mm spare against 0.40", g_cell_fit),
    ("O-S1 (control): the largest cell cellgap says this hole takes passes VER-181", "dn_pass",
     "RESULT: PASS", g_cell_fit_control),
    ("O-S5: a check moved later with no GATE@ line is listed by the order gate (G0)", "gate_line",
     "VER-138 moved from EVT to DVT", g_moves_listed),
    ("VER-18: a connector's pad past the board's outline fails edge (a courtyard alone does not)", "dn",
     "J2 (a pad or its body past the outline)", g_body_past_edge),
    # the review's findings applied at 71737ae (2026-10-27)
    ("O-R1: a defect taken from OPEN to FIXED with no GATE@ line is listed by the order gate (G0)", "gate_line",
     "DEF-47 went from OPEN to FIXED", g_status_listed),
    ("O-R1: a defect taken from OPEN to FIXED after Alex's signed GATE@ line fails the order gate (G1)", "gate",
     "DEF-47 went from OPEN to FIXED", g_status_freeze),
    # the framework fixes after vv-rp2354b-final's sign-off (2026-10-28)
    ("accept: a defect ACCEPTED on an unsigned DF-ACCEPT line", "structure",
     "DEF-47: ACCEPTED but releases/APPROVALS.md has no signed 'DF-ACCEPT:DEF-47' line naming the gate it moves to "
     "and why (A-001 pending: not signed with Alex's key)", g_accept_unsigned),
    ("accept (control): a signed DF-ACCEPT line moves DEF-103 past the order gate and lets VER-125 through", "gate_no",
     "VER-125 ACCEPTED (DEF-103 to EVT, A-001", g_accept_signed, "DEF-103 went from OPEN"),
    ("accept: a check outside the accepted defect's list is not let through (DEF-104 names VER-164, not VER-125)",
     "gate_no", "VER-164 ACCEPTED (DEF-104 to EVT, A-001", g_accept_outside, "VER-125 ACCEPTED"),
    ("R8: a proposal doc changed on the order branch after the baseline fails the order gate", "gate",
     "seeded-proposal has architecture proposal commits after", g_proposal_branch),
    ("R8: a proposal doc changed on the order branch after the baseline warns in the default mode", "report",
     "seeded-proposal has architecture proposal commits after", g_proposal_branch_default),
    ("grid R9: a GRID line signed with no ACCEPTED_GAP row leaves the cell a GAP and says why", "gate",
     "GRID-1-power is signed but grid_reasons.csv has no ACCEPTED_GAP row", g_grid_signed_no_row),
]


def run(d, *extra, trace=TRACE):
    r = subprocess.run([sys.executable, trace] + (["--vv", d] if d else []) + list(extra), capture_output=True,
                       text=True)
    return r.returncode, r.stdout + r.stderr


def fresh_copy(tmp, name):
    d = os.path.join(tmp, re.sub(r"\W+", "_", name))
    shutil.copytree(VV, d, ignore=shutil.ignore_patterns("__pycache__"))
    return d


def judge(mode, rc, out, want, forbid=""):
    if mode == "dn":
        return rc != 0 and want in out
    if mode == "dn_pass":
        # a control: the design_numbers check must pass on the seeded change
        return rc == 0 and want in out
    if mode == "structure":
        return rc == 1 and want in out
    if mode == "report":
        return rc == 0 and want in out
    if mode == "gate_no":
        # a control: the text must be there and the fault's own message must not be
        return want in out and forbid not in out
    if mode == "gate_line":
        # a line's state in the gate's output (the gate fails for other reasons)
        return want in out
    return rc == 1 and want in out


def main():
    if "--list" in sys.argv:
        for f in FAULTS + GIT_FAULTS:
            name, mode, want = f[0], f[1], f[2]
            print(f"{mode:9} {name}: expects {want!r}")
        return 0
    missed = []
    n = 0
    with tempfile.TemporaryDirectory() as tmp:
        rc, out = run(fresh_copy(tmp, "clean"))
        clean_ok = rc == 0
        print(f"{'ok' if clean_ok else 'FAIL'}      clean copy exits 0" + ("" if clean_ok else f" (exit {rc})"))
        if not clean_ok:
            print(out[-2000:])
            missed.append("clean")
        for f in FAULTS:
            n += 1
            name, mode, want, mutate = f[:4]
            forbid = f[4] if len(f) > 4 else ""
            d = fresh_copy(tmp, name)
            mutate(d)
            rc, out = run(d, "--gate", "order") if mode in ("gate", "gate_no", "gate_line") else run(d)
            ok = judge(mode, rc, out, want, forbid)
            print(f"{'ok' if ok else 'MISSED'}  {mode:9} {name}")
            if not ok:
                missed.append(name)
                print("          exit", rc, "; output tail:", out[-600:].replace("\n", "\n          "))

        # the git-history faults, in a scratch clone of this repository
        repo = os.path.join(tmp, "clone")
        subprocess.run(["git", "clone", "-q", "--shared", ROOT, repo], check=True)
        # a clone checks out the repository's default branch, which may not be the branch this register
        # lives on: put the clone's tree on this worktree's HEAD, so every seeded fault sees the same
        # design files the register describes
        git(repo, "reset", "-q", "--hard", HEAD_SHA)
        git(repo, "config", "user.name", "Alex")
        git(repo, "config", "user.email", "alex@example.invalid")
        setup_signing(repo, tmp)
        shutil.rmtree(os.path.join(repo, "hardware/vv"))
        shutil.copytree(VV, os.path.join(repo, "hardware/vv"), ignore=shutil.ignore_patterns("__pycache__"))
        pin_key(repo)
        base = commit(repo, "the working register and the clone's pinned key, as a base for seeded faults")
        trace = os.path.join(repo, "hardware/vv/tools/check_trace.py")
        rc, out = run(None, trace=trace)
        clone_ok = rc == 0
        print(f"{'ok' if clone_ok else 'FAIL'}      clean clone exits 0" + ("" if clone_ok else f" (exit {rc})"))
        if not clone_ok:
            print(out[-2000:])
            missed.append("clean clone")
        for f in GIT_FAULTS:
            n += 1
            name, mode, want, mutate = f[:4]
            forbid = f[4] if len(f) > 4 else ""
            git(repo, "reset", "-q", "--hard", base)
            git(repo, "clean", "-q", "-fdx", "--", "hardware")
            gate = mutate(repo, base)
            extra = []
            if isinstance(gate, dict):
                extra = gate.get("args", [])
                gate = gate.get("gate", "order")
            if isinstance(gate, tuple):
                ok, out, rc = True, "", 0
                runs = ([[sys.executable, os.path.join(repo, "hardware/vv/tools/design_numbers.py")] + a for a in gate[1]]
                        if gate[0] in ("dn", "dn_pass") else [[sys.executable, "-c", gate[1]]])
                for cmd in runs:
                    r = subprocess.run(cmd, capture_output=True, text=True, cwd=repo)
                    out += r.stdout + r.stderr
                    rc = r.returncode
                    ok &= judge(mode, r.returncode, r.stdout + r.stderr, want)
            else:
                rc, out = run(None, *(["--gate", gate] if gate else []), *extra, trace=trace)
                ok = judge(mode, rc, out, want, forbid)
            print(f"{'ok' if ok else 'MISSED'}  {mode:9} {name} (git)")
            if not ok:
                missed.append(name)
                print("          exit", rc, "; output tail:", out[-800:].replace("\n", "\n          "))
    caught = n - len([m for m in missed if m not in ("clean", "clean clone")])
    print(f"{n} seeded faults, {caught} caught")
    print("RESULT: " + ("PASS" if not missed else "FAIL"))
    return 1 if missed else 0


if __name__ == "__main__":
    sys.exit(main())
