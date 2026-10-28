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
  R8       a proposal doc (docs/*proposal*.md) changed on the order branch after the baseline: the default
           mode warns and the order gate fails.
  grid     a GRID line signed with no ACCEPTED_GAP row leaves its cell a GAP, and R9 says why.

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
                        "hardware/vv/evidence/2026-10-24_VER-01_check_netlist.txt")),
    ("a commit message as evidence", "structure", "a commit message is not evidence",
     lambda d: csv_edit(d, "verification.csv", "VER-01", last_result="PASS", commit=HEAD7, date=TODAY,
                        evidence="git show 76955d2")),
    ("an analysis PASS with no saved output", "structure", "needs its saved output under",
     lambda d: csv_edit(d, "verification.csv", "VER-01", last_result="PASS", commit=HEAD7, date=TODAY,
                        evidence="hardware/vv/README.md")),
    ("a test (T) PASS with no saved output", "structure", "VER-43: needs its saved output under",
     lambda d: csv_edit(d, "verification.csv", "VER-43", last_result="PASS", commit=HEAD7, date=TODAY,
                        evidence="hardware/ref_design/docs/assembly.md")),
    ("CLOSED defect with no check", "structure", "CLOSED with no VER",
     lambda d: (block_field(d, "defects.md", "DEF-07", "Check added or changed", "none"),
                block_field(d, "defects.md", "DEF-07", "Status", "CLOSED (seeded)"))),
    ("covers glob matching nothing", "structure", "matches no tracked file",
     lambda d: csv_edit(d, "verification.csv", "VER-01", covers=lambda c: c + " hardware/nothing/*.py")),
    ("covers regex matching no line", "structure", "matches no current line",
     lambda d: csv_edit(d, "verification.csv", "VER-104", covers="hardware/ref_design/docs/pinmap.md::nosuchword")),
    ("covers part not in design.py", "structure", "R999 not in design.py",
     lambda d: csv_edit(d, "verification.csv", "VER-12", covers="part:R999")),
    ("a 'both' check covering one board", "structure", "a 'both' check must cover each board",
     lambda d: csv_edit(d, "verification.csv", "VER-01", board="both")),
    ("CUT check with no reason", "structure", "CUT needs its reason",
     lambda d: csv_edit(d, "verification.csv", "VER-47", evidence="")),
    ("RETIRED check with no commit", "structure", "RETIRED needs 'RETIRED in <commit>",
     lambda d: csv_edit(d, "verification.csv", "VER-09", evidence="RETIRED: the 3-ring is gone")),
    ("a live requirement verified by a retired check", "structure", "REQ-PRD-05: verified by VER-11, which is RETIRED",
     lambda d: block_field(d, "requirements.md", "REQ-PRD-05", "Verified by", "VER-65, VER-11")),
    ("a live risk detected by a retired check", "structure", "DF-35: detects VER-11, which is RETIRED",
     lambda d: csv_edit(d, "dfmea.csv", "DF-35", detects=lambda x: x + " VER-11", detection=lambda x: x + "; VER-11")),
    ("a gate criterion naming a retired check", "structure", "G-EVT-04: needs VER-11, which is RETIRED",
     lambda d: text_edit(d, "gates.md", "| VER-12 VER-30 VER-13 VER-31 ", "| VER-11 VER-12 VER-30 VER-13 VER-31 ")),
    ("vp_map status changed with no evidence", "structure", "with no evidence",
     lambda d: csv_edit(d, "vp_map.csv", "4", status_now="no")),
    ("requirement with no Verified by", "structure", "field 'Verified by' missing or empty",
     lambda d: block_field(d, "requirements.md", "REQ-PRD-05", "Verified by", "")),
    ("requirement APPROVED with no APPROVALS line", "structure", "has no committed line for it",
     lambda d: block_field(d, "requirements.md", "REQ-PRD-05", "Status", "APPROVED")),
    ("allocation on the wrong board", "structure", "a .R allocation is Board: ref_design",
     lambda d: block_field(d, "requirements.md", "REQ-PRD-01.R", "Board", "both")),
    ("risk ACCEPTED with no APPROVALS line", "structure", "ACCEPTED but releases/APPROVALS.md",
     lambda d: csv_edit(d, "dfmea.csv", "DF-87", status="ACCEPTED")),
    ("approval line signed by Claude", "structure", "only Alex signs",
     lambda d: approve(d, "| A-001 | REQ-PRD-05 | APPROVED | - | - | Claude | 2026-10-24 |")),
    ("a deferral with no scope phase", "structure", "REQ-PRD-05 DEFERRED needs a scope phase",
     lambda d: approve(d, "| A-001 | REQ-PRD-05 | DEFERRED | - | waiting for the cell | Alex | 2026-10-25 |")),
    ("detects names a check the text does not", "structure", "which the detection text does not describe",
     lambda d: csv_edit(d, "dfmea.csv", "DF-70", detects=lambda x: x + " VER-02",
                        detection=lambda x: x.replace("VER-02", "the pin gate"))),
    ("detection rating better than its checks", "structure", "must be 8 or more",
     lambda d: csv_edit(d, "dfmea.csv", "DF-152", detection_rating="6", action_priority="M")),
    ("occurrence 2 with no built hardware", "structure", "occurrence 2 needs a passing T check",
     lambda d: csv_edit(d, "dfmea.csv", "DF-20", occurrence="2", action_priority="M")),
    ("DONE with no detects", "structure", "DONE needs at least one 'detects' check",
     lambda d: csv_edit(d, "dfmea.csv", "DF-87", status="DONE")),
    # every ORDER check is now also named by a criterion or a requirement (2026-10-26), so dropping PHASE:ORDER
    # alone no longer strands one: with it, an advisory check taken out of its advisory criterion is reached by nothing
    ("an ORDER check no gate reaches", "structure", "VER-21: phase ORDER, but no gate criterion",
     lambda d: (text_edit(d, "gates.md", "| ADVISORY VER-40 VER-21 VER-27 |", "| ADVISORY VER-40 VER-27 |"),
                text_edit(d, "gates.md", "| PHASE:ORDER | both |", "| VER-01 | both |"))),
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
     lambda d: text_edit(d, "gates.md", "| ADVISORY VER-40 VER-21 VER-27 |", "| VER-40 VER-21 VER-27 |")),
    # the second review (2026-10-25)
    ("O-R3: an inspection PASS on a design file it does not cover", "structure", "VER-28: needs its saved output under",
     lambda d: csv_edit(d, "verification.csv", "VER-28", last_result="PASS", commit=HEAD7, date=TODAY,
                        evidence="hardware/ref_design/README.md")),
    ("O-R3: an inspection's evidence with no WHO or ARTEFACT", "structure", "has no 'WHO:' line",
     lambda d: borrow_evidence(d, "VER-28", "VER-99")),
    ("O-R5: a defect with no Gate", "structure", "DEF-07: Gate ''",
     lambda d: block_field(d, "defects.md", "DEF-07", "Gate", "")),
    ("O-R5: a CASE defect moved to the order gate blocks it", "gate", "OPEN (Gate ORDER or earlier): DEF-14 DEF-21 ",
     lambda d: block_field(d, "defects.md", "DEF-21", "Gate", "ORDER")),
    ("O-R8: occurrence 3 with no 'Similar:'", "structure", "DF-92: occurrence 3 needs 'Similar:",
     lambda d: csv_edit(d, "dfmea.csv", "DF-92", prevention=lambda x: x.replace("Similar:", "Like:"))),
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
    ("round 2 (F): a part no DFMEA row names fails DFMEA:COVERAGE", "gate", "parts in no DFMEA row: R64",
     lambda d: csv_edit(d, "dfmea.csv", "DF-153", item="the ILIM resistor", failure_mode="the ILIM resistor open")),
    ("round 2 (F): detection rating 2 on a single 'detects' check", "structure",
     "DF-96: detection_rating 2 needs two or more independent 'detects' checks",
     lambda d: csv_edit(d, "dfmea.csv", "DF-96", detection_rating="2", action_priority="L")),
    # the coverage grid and the stale-names scan (2026-10-27)
    ("VER-188: a live row names a part number that left design.py (TP4056 outside 'Was:')", "structure",
     f"DF-147: names what design.py at {BASE7} does not have: TP4056",
     lambda d: csv_edit(d, "dfmea.csv", "DF-147", cause=lambda x: x.replace("(Was: the TP4056's,", "(the TP4056's,"))),
    ("VER-188: a live check names a net that left design.py (LS_GATE)", "structure",
     f"VER-21: names what design.py at {BASE7} does not have: LS_GATE (net",
     lambda d: csv_edit(d, "verification.csv", "VER-21", pass_limit=lambda x: x + ", LS_GATE")),
    ("VER-188: a live requirement names a part that left design.py (J6 outside 'Was:')", "structure",
     f"REQ-MCU-04: names what design.py at {BASE7} does not have: J6 (part",
     lambda d: text_edit(d, "requirements.md", "since 86bf694 (Was: J6).", "since 86bf694 (J6).")),
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
    # 8 since 2026-10-28: VER-144 and VER-193 are BLOCKED (their PASS rests on ao3400a.pdf, srv05-4.pdf and mcp6001.pdf,
    # not on disk), so only VER-13 (an analysis that ran: FAIL) clears class 15, for power and cell; the other six cells
    # are planned.  VER-13 not run leaves all eight planned; counting a NOT_RUN analysis would keep the count at 6.
    # (5 before, with VER-193 not run while VER-144 and VER-193 cleared the rest.)
    ("grid: a class 15 analysis that has not run is no paper estimate", "report", "15 Thermal 8",
     lambda d: csv_edit(d, "verification.csv", "VER-13", last_result="NOT_RUN", evidence="", commit="", date="")),
    ("VER-188: a live check names a ref no design.py ever had (R666, a typo)", "structure",
     "R666 (part, never in design.py)",
     lambda d: csv_edit(d, "verification.csv", "VER-21", pass_limit=lambda x: x + ", R666")),
    # the review's findings applied at 71737ae (2026-10-27)
    ("F-R1: a defect marked FIXED while its own Status text says it stays OPEN", "structure",
     "DEF-50: Status is FIXED but its own text says 'stays OPEN'",
     lambda d: text_edit(d, "defects.md", "- Status: OPEN (R14 47k and R43 220k",
                         "- Status: FIXED (It stays OPEN, and the order gate fails on it. R14 47k and R43 220k")),
    ("F-R3: an analysis that reads pinmap.md without naming it in covers", "structure",
     "VER-138: its procedure reads pinmap.md, but covers does not name",
     lambda d: csv_edit(d, "verification.csv", "VER-138",
                        covers=lambda x: x.replace("hardware/ref_design/docs/pinmap.md::[Cc]alibrat\\w* ", ""))),
    ("F-R5: a live row pairs J4 with the PicoBlade family (DF-48 before its rewrite)", "structure",
     "DF-48: names what design.py at " + BASE7 + " does not have: PicoBlade (PicoBlade) beside J4",
     lambda d: csv_edit(d, "dfmea.csv", "DF-48", prevention=lambda x: x.replace(
         "the speaker's leads end in a JST SH SHR-02V-S housing", "the speaker's leads end in a PicoBlade plug"))),
    ("O-R8: a live check names a gone net in KiCad's path form (/USB_Charger/VBUS_D5)", "structure",
     "VER-21: names what design.py at " + BASE7 + " does not have: VBUS_D5 (net",
     lambda d: csv_edit(d, "verification.csv", "VER-21", pass_limit=lambda x: x + ", /USB_Charger/VBUS_D5")),
    ("O-R6: an analysis that has not run does not clear a class 1-14 cell (display, class 4)", "gate",
     "4 display planned: VER-05 VER-137",
     lambda d: (csv_edit(d, "verification.csv", "VER-137", last_result="NOT_RUN", evidence="", commit="", date=""),
                block_field(d, "defects.md", "DEF-39", "Status", "FIXED (seeded: VER-137 not run)"))),
    # the framework fixes after vv-rp2354b-final's sign-off (2026-10-28)
    ("accept: a defect ACCEPTED with no DF-ACCEPT line at all", "structure",
     "DEF-62: ACCEPTED but releases/APPROVALS.md has no signed 'DF-ACCEPT:DEF-62' line",
     lambda d: block_field(d, "defects.md", "DEF-62", "Status", "ACCEPTED (to EVT, seeded)")),
    ("sources: a PASS whose datasheet is not on disk is BLOCKED at the order gate", "gate",
     "VER-99 BLOCKED: its PASS rests on nosuch_datasheet.pdf",
     lambda d: csv_edit(d, "verification.csv", "VER-99", sources="nosuch_datasheet.pdf")),
    ("sources: the default mode reports a PASS on a datasheet not on disk as BLOCKED", "report",
     "    VER-99: nosuch_datasheet.pdf",
     lambda d: csv_edit(d, "verification.csv", "VER-99", sources="nosuch_datasheet.pdf")),
    ("sources: a source that is a path, not a file in the datasheets directory", "structure",
     "VER-99: sources token 'datasheets/bq24074.pdf' is not a file name",
     lambda d: csv_edit(d, "verification.csv", "VER-99", sources="datasheets/bq24074.pdf")),
]


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
    """Record a PASS for vid at commit base with a header that check_trace accepts."""
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
    approve(os.path.join(repo, "hardware/vv"), "| A-001 | DF-04 | ACCEPTED | ORDER | for the order only | Alex | 2026-10-25 |")
    csv_edit(os.path.join(repo, "hardware/vv"), "dfmea.csv", "DF-04", status="ACCEPTED")
    commit(repo, "accept DF-04 for ORDER")
    return "evt"


def g_signed_waiver(repo, base):
    # VER-33 (G-ORD-06's check) passes since 2026-10-27: undo it in the clone so the criterion fails and the waiver acts
    csv_edit(os.path.join(repo, "hardware/vv"), "verification.csv", "VER-33", last_result="NOT_BUILT", evidence="",
             commit="", date="")
    commit(repo, "VER-33 not built in this clone")
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
    approve(os.path.join(repo, "hardware/vv"), "| A-001 | REQ-PRD-06 | APPROVED | - | - | Alex | 2026-10-25 |")
    block_field(os.path.join(repo, "hardware/vv"), "requirements.md", "REQ-PRD-06", "Status", "APPROVED")
    commit(repo, "approve REQ-PRD-06", sign=False)
    return None


def g_merge_approval(repo, base):
    git(repo, "checkout", "-q", "-b", "seeded-side")
    with open(os.path.join(repo, "hardware/vv/PLAN.md"), "a") as f:
        f.write("\nside\n")
    commit(repo, "side")
    git(repo, "checkout", "-q", "-")
    git(repo, "merge", "-q", "--no-ff", "--no-commit", "seeded-side")
    approve(os.path.join(repo, "hardware/vv"), "| A-001 | REQ-PRD-06 | APPROVED | - | - | Alex | 2026-10-25 |")
    block_field(os.path.join(repo, "hardware/vv"), "requirements.md", "REQ-PRD-06", "Status", "APPROVED")
    commit(repo, "Merge branch 'seeded-side'")
    git(repo, "branch", "-q", "-D", "seeded-side")
    return None


def g_uncommitted_register(repo, base):
    """O-R2's attack: VER-138 FAIL to PASS, and its evidence's RESULT with it, left uncommitted."""
    d = os.path.join(repo, "hardware/vv")
    row = next(r for r in csv.DictReader(open(os.path.join(d, "verification.csv"), encoding="utf-8")) if r["id"] == "VER-138")
    p = os.path.join(repo, row["evidence"].split()[0])
    s = open(p).read()
    open(p, "w").write(re.sub(r"^RESULT: FAIL", "RESULT: PASS", s, flags=re.M))
    csv_edit(d, "verification.csv", "VER-138", last_result="PASS")
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
    approve(os.path.join(repo, "hardware/vv"), "| A-001 | REQ-PRD-06 | APPROVED | - | - | Alex | 2026-10-26 |")
    block_field(os.path.join(repo, "hardware/vv"), "requirements.md", "REQ-PRD-06", "Status", "APPROVED")
    commit(repo, "approve REQ-PRD-06 (signed with the other key)", key=KEYS["other"])
    return None


def g_bulk_edited(repo, base):
    """Round 2: a bulk 'REQ-ALL@<commit>' line covers a requirement only while its statement is unchanged."""
    approve(os.path.join(repo, "hardware/vv"), f"| A-001 | REQ-ALL@{base[:7]} | APPROVED | - | - | Alex | 2026-10-26 |")
    block_field(os.path.join(repo, "hardware/vv"), "requirements.md", "REQ-PRD-06", "Status", "APPROVED")
    commit(repo, "approve every requirement as worded at the base")
    block_field(os.path.join(repo, "hardware/vv"), "requirements.md", "REQ-PRD-06", "Statement",
                "The closed case shall measure at most 80 x 160 mm in plan.")
    commit(repo, "loosen REQ-PRD-06")
    return None


def g_cellgap_right_end(repo, base):
    """Round 2: widening only the hole's right side (the first proposed fix; since 71737ae the cell lies along y, so
    its long side) leaves VER-181 failing: the pocket follows the hole, and the cell's length still has no fit."""
    p = os.path.join(repo, "hardware/ref_design/case/case.scad")
    s = open(p).read()
    old = "cell_hole = [[-18.9, -7.8 + cell_dy], [18.9, 47.0 + cell_dy]]"
    if old not in s:
        raise SystemExit("seed_faults: case.scad's cell_hole line moved")
    open(p, "w").write(s.replace(old, "cell_hole = [[-18.9, -7.8 + cell_dy], [19.4, 47.0 + cell_dy]]", 1))
    return ("dn", [["cellgap"]])


def g_cell_fit(repo, base):
    """Round 3's sign-off (O-S1): a cell 0.1 mm too long for its pocket's fit fails VER-181, though every gap to the
    FR4 passes: the pocket printed 0.1 mm small must still take it with 0.1 mm a side."""
    p = os.path.join(repo, "hardware/ref_design/case/case.scad")
    s = open(p).read()
    old = "cell = [52, 36, 4.5];"
    if old not in s:
        raise SystemExit("seed_faults: case.scad's cell line moved")
    open(p, "w").write(s.replace(old, "cell = [51.7, 35.6, 4.5];", 1))
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
    block_field(os.path.join(repo, "hardware/vv"), "defects.md", "DEF-14", "Status", "FIXED (seeded)")
    commit(repo, "DEF-14 FIXED")
    return "order"


def g_status_freeze(repo, base):
    """O-R1: a defect taken from OPEN to FIXED after Alex's signed GATE@ line fails the order gate (G1)."""
    approve(os.path.join(repo, "hardware/vv"),
            f"| A-001 | GATE@{base} | FROZEN | ORDER | the order gate state at this commit | Alex | {TODAY} |")
    commit(repo, "freeze the gate state")
    block_field(os.path.join(repo, "hardware/vv"), "defects.md", "DEF-14", "Status", "FIXED (seeded)")
    commit(repo, "DEF-14 FIXED")
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
    text_edit(os.path.join(repo, "hardware/vv"), "gates.md", "| VER-33 VER-99 VER-137 ", "| ADVISORY:VER-33 VER-99 VER-137 ")
    commit(repo, "VER-33 advisory at the order gate")
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
    approve(vv, f"| A-001 | DF-ACCEPT:DEF-62 | ACCEPTED | EVT | DEC-27 decided at EVT | Alex | {TODAY} |")
    block_field(vv, "defects.md", "DEF-62", "Status", "ACCEPTED (to EVT, A-001)")
    commit(repo, "DEF-62 accepted to EVT, unsigned", sign=False)
    return None


def g_accept_signed(repo, base):
    """1. (control): Alex's signed DF-ACCEPT line moves DEF-62 (Gate ORDER) to EVT: R4 lists it beside its result,
    not in the OPEN list, G-ORD-07 lets VER-181 (a check DEF-62 names) through as ACCEPTED, and G0 does not list
    the move (it is his own signed one)."""
    vv = os.path.join(repo, "hardware/vv")
    approve(vv, f"| A-001 | DF-ACCEPT:DEF-62 | ACCEPTED | EVT | DEC-27 decided at EVT | Alex | {TODAY} |")
    block_field(vv, "defects.md", "DEF-62", "Status", "ACCEPTED (to EVT, A-001)")
    commit(repo, "DEF-62 accepted to EVT")
    return "order"


def g_accept_outside(repo, base):
    """1.: a signed acceptance lets through only the checks its defect names.  DEF-37 (VER-164, VER-87) accepted to
    EVT: G-ORD-07 shows VER-164 ACCEPTED, and VER-181, which DEF-37 does not name, still FAIL."""
    vv = os.path.join(repo, "hardware/vv")
    approve(vv, f"| A-001 | DF-ACCEPT:DEF-37 | ACCEPTED | EVT | DEC-25's pad is a case part | Alex | {TODAY} |")
    block_field(vv, "defects.md", "DEF-37", "Status", "ACCEPTED (to EVT, A-001)")
    commit(repo, "DEF-37 accepted to EVT")
    return "order"


def g_proposal_branch(repo, base):
    """3. (2026-10-28): a commit on the order branch after the baseline that touches docs/*proposal*.md (dcb6a76
    chose another MCU there): the default mode warns, the order gate fails R8."""
    b = re.search(r"^- Order branch: `[^`]+` from `([0-9a-f]+)`", open(os.path.join(repo, "hardware/vv/README.md")).read(),
                  re.M).group(1)
    wt = os.path.join(os.path.dirname(repo), "order_wt_proposal")
    if not os.path.isdir(wt):
        git(repo, "worktree", "add", "-q", "-b", "seeded-proposal", wt, b)
        with open(os.path.join(wt, "hardware/ref_design/docs/next_revision_proposal.md"), "a") as f:
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
    ("an acceptance used past its scope", "gate", "DF-04 accepted only up to ORDER", g_expired_accept),
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
     "CHG-01: 'What to re-run' omits VER-52", g_chg_omits),
    ("O-R1: a comment in fab.py does not pass the CPL or drill check", "dn", "RESULT: FAIL", g_fab_comment),
    ("O-R14: a back-side test point next to the cell's hole fails th1", "dn",
     "back-side pads within 1 mm of the hole: TP11", g_tp_near_hole),
    # round 2's sign-off objections (2026-10-26)
    ("round 2 (O): an approval signed by another key in the allowed-signers file", "structure",
     "A-001 pending: signed with another key", g_other_key),
    ("round 2 (O): a bulk REQ-ALL line does not cover a statement edited after it", "structure",
     "REQ-PRD-06: Status APPROVED but releases/APPROVALS.md has no committed line for it", g_bulk_edited),
    ("round 2 (O, F): widening only the cell hole's right end keeps VER-181 failing", "dn", "RESULT: FAIL",
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
     "G-ORD-18 gained ADVISORY:VER-33", g_gate_freeze_advisory),
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
     "DEF-14 went from OPEN to FIXED", g_status_listed),
    ("O-R1: a defect taken from OPEN to FIXED after Alex's signed GATE@ line fails the order gate (G1)", "gate",
     "DEF-14 went from OPEN to FIXED", g_status_freeze),
    # the framework fixes after vv-rp2354b-final's sign-off (2026-10-28)
    ("accept: a defect ACCEPTED on an unsigned DF-ACCEPT line", "structure",
     "DEF-62: ACCEPTED but releases/APPROVALS.md has no signed 'DF-ACCEPT:DEF-62' line naming the gate it moves to "
     "and why (A-001 pending: not signed with Alex's key)", g_accept_unsigned),
    ("accept (control): a signed DF-ACCEPT line moves DEF-62 past the order gate and lets VER-181 through", "gate_no",
     "VER-181 ACCEPTED (DEF-62 to EVT, A-001", g_accept_signed, "DEF-62 went from OPEN"),
    ("accept: a check outside the accepted defect's list stays FAIL (DEF-37 names VER-164, not VER-181)", "gate_no",
     "VER-164 ACCEPTED (DEF-37 to EVT, A-001", g_accept_outside, "VER-181 ACCEPTED"),
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
