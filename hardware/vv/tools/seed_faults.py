#!/usr/bin/env python3
"""Prove check_trace.py can fail: seed one fault at a time into a copy of hardware/vv.

    python3 hardware/vv/tools/seed_faults.py            # every fault; exit 1 if any is missed
    python3 hardware/vv/tools/seed_faults.py --list

Each fault copies hardware/vv to a scratch directory, changes one thing, and runs
check_trace.py --vv <copy> on it. A structural fault must exit 1 and print its message; a
freshness fault must show in the report; a gate fault must show in the gate's output. The
clean copy must exit 0. This is VER-129: the trace gate's own seeded-fault test (DEF-35).
Faults that need git history (an approval line in a commit with a Claude trailer, a
statement edited after its approval) are not seeded here: they need a scratch repository.

Python 3 standard library only.
"""
import csv
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
    blk = re.sub(r"^- " + re.escape(field) + r": .*$", f"- {field}: {value}", m.group(0), count=1, flags=re.M)
    open(path, "w", encoding="utf-8").write(s[:m.start()] + blk + s[m.end():])


REL = """# {id}

- Board: ref_design
- BOARD_REV: 1
- BOM sha256: {bom}
"""

# (name, mode, expected text, mutation). mode: 'structure' wants exit 1 and the text;
# 'report' wants exit 0 and the text in the report; 'gate' wants the text in the gate output.
FAULTS = [
    ("duplicate VER id", "structure", "duplicate ID VER-01",
     lambda d: csv_edit(d, "verification.csv", "VER-01", _dup=True)),
    ("dangling DF id in a gate", "structure", "DF-999 is referenced but not defined",
     lambda d: text_edit(d, "gates.md", "| VER-98 VER-148 DFMEA:COVERAGE |", "| VER-98 VER-148 DFMEA:COVERAGE DF-999 |")),
    ("action priority off the AIAG-VDA table", "structure", "(AIAG-VDA table)",
     lambda d: csv_edit(d, "dfmea.csv", "DF-04", action_priority="L")),
    ("PASS without a commit", "structure", "PASS needs commit",
     lambda d: csv_edit(d, "verification.csv", "VER-01", commit="")),
    ("evidence header names another commit", "structure", "has no 'COMMIT: 56933a5' line",
     lambda d: csv_edit(d, "verification.csv", "VER-01", commit="56933a5")),
    ("row result disagrees with its evidence", "structure", "the row says FAIL",
     lambda d: csv_edit(d, "verification.csv", "VER-01", last_result="FAIL")),
    ("a commit message as evidence", "structure", "a commit message is not evidence",
     lambda d: csv_edit(d, "verification.csv", "VER-01", evidence="git show 76955d2")),
    ("an analysis PASS with no saved output", "structure", "needs its saved output under",
     lambda d: csv_edit(d, "verification.csv", "VER-01", evidence="hardware/vv/README.md")),
    ("evidence past its valid_days", "report", "expired",
     lambda d: csv_edit(d, "verification.csv", "VER-01", valid_days="30", date="2026-01-01")),
    ("CLOSED defect with no check", "structure", "CLOSED with no VER",
     lambda d: block_field(d, "defects.md", "DEF-07", "Check added or changed", "none")),
    ("covers glob matching nothing", "structure", "matches no tracked file",
     lambda d: csv_edit(d, "verification.csv", "VER-01", covers=lambda c: c + " hardware/nothing/*.py")),
    ("covers regex matching no line", "structure", "matches no current line",
     lambda d: csv_edit(d, "verification.csv", "VER-104", covers="hardware/ref_design/docs/pinmap.md::nosuchword")),
    ("covers part not in design.py", "structure", "R999 not in design.py",
     lambda d: csv_edit(d, "verification.csv", "VER-11", covers="part:R999")),
    ("a 'both' check covering one board", "structure", "a 'both' check must cover each board",
     lambda d: csv_edit(d, "verification.csv", "VER-01", board="both")),
    ("CUT check with no reason", "structure", "CUT needs its reason",
     lambda d: csv_edit(d, "verification.csv", "VER-47", evidence="")),
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
     lambda d: text_edit(d, "releases/APPROVALS.md", "|---|---|---|---|---|---|---|",
                         "|---|---|---|---|---|---|---|\n| A-001 | REQ-PRD-05 | APPROVED | - | - | Claude | 2026-10-24 |")),
    ("detects names a check the text does not", "structure", "which the detection text does not describe",
     lambda d: csv_edit(d, "dfmea.csv", "DF-70", detects=lambda x: x + " VER-02",
                        detection=lambda x: x.replace("VER-02", "the pin gate"))),
    ("detection rating better than its checks", "structure", "must be 8 or more",
     lambda d: csv_edit(d, "dfmea.csv", "DF-04", detection_rating="6", action_priority="H")),
    ("occurrence 2 with no built hardware", "structure", "occurrence 2 needs a passing T check",
     lambda d: csv_edit(d, "dfmea.csv", "DF-10", occurrence="2", action_priority="M")),
    ("DONE with no detects", "structure", "DONE needs at least one 'detects' check",
     lambda d: csv_edit(d, "dfmea.csv", "DF-87", status="DONE")),
    ("an ORDER check no gate reaches", "structure", "that gate can pass without it",
     lambda d: (text_edit(d, "gates.md", "| PHASE:ORDER | both |", "| VER-01 | both |"),
                text_edit(d, "gates.md", "| PHASE:CASE | both |", "| VER-01 | both |"),
                text_edit(d, "gates.md", "| PHASE:EVT | both |", "| VER-01 | both |"))),
    ("criterion with a bad board", "structure", "board 'nowhere' not in",
     lambda d: text_edit(d, "gates.md", "| PHASE:ORDER | both |", "| PHASE:ORDER | nowhere |")),
    ("new BOM under an old BOARD_REV", "structure", "BOARD_REV is still",
     lambda d: (write(d, "releases/REL-REFA-R1-EVT.md", REL.format(id="REL-REFA-R1-EVT", bom="a" * 64)),
                write(d, "releases/REL-REFA-R2-DVT.md", REL.format(id="REL-REFA-R2-DVT", bom="b" * 64)))),
    ("design baseline differs from HEAD", "gate", "gate evaluated against a design it does not describe",
     lambda d: text_edit(d, "README.md", "08d17297ca629ef9fbc19ac4a91c4002948d1703", "0" * 40)),
]


def run(d, *extra):
    r = subprocess.run([sys.executable, TRACE, "--vv", d] + list(extra), capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


def fresh_copy(tmp, name):
    d = os.path.join(tmp, re.sub(r"\W+", "_", name))
    shutil.copytree(VV, d, ignore=shutil.ignore_patterns("__pycache__"))
    return d


def main():
    if "--list" in sys.argv:
        for name, mode, want, _ in FAULTS:
            print(f"{mode:9} {name}: expects {want!r}")
        return 0
    missed = []
    with tempfile.TemporaryDirectory() as tmp:
        rc, out = run(fresh_copy(tmp, "clean"))
        clean_ok = rc == 0
        print(f"{'ok' if clean_ok else 'FAIL'}      clean copy exits 0" + ("" if clean_ok else f" (exit {rc})"))
        if not clean_ok:
            print(out[-2000:])
            missed.append("clean")
        for name, mode, want, mutate in FAULTS:
            d = fresh_copy(tmp, name)
            mutate(d)
            rc, out = run(d, "--gate", "order") if mode == "gate" else run(d)
            if mode == "structure":
                ok = rc == 1 and want in out
            elif mode == "report":
                ok = rc == 0 and want in out
            else:
                ok = rc == 1 and want in out
            print(f"{'ok' if ok else 'MISSED'}  {mode:9} {name}")
            if not ok:
                missed.append(name)
                print("          exit", rc, "; output tail:", out[-600:].replace("\n", "\n          "))
    print(f"{len(FAULTS)} seeded faults, {len(FAULTS) - len([m for m in missed if m != 'clean'])} caught")
    print("RESULT: " + ("PASS" if not missed else "FAIL"))
    return 1 if missed else 0


if __name__ == "__main__":
    sys.exit(main())
