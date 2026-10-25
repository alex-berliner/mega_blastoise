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
own check_trace.py.

A structural fault must exit 1 and print its message; a report fault must exit 0 with its
text in the report; a gate fault must exit 1 with its text in the gate's output. The clean
copy and the clean clone must exit 0. This is VER-129: the trace gate's own seeded-fault test
(DEF-35, DEF-47).

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
TODAY = datetime.date.today().isoformat()
DESIGN_BLOB = subprocess.run(["git", "-C", ROOT, "rev-parse", "HEAD:hardware/ref_design/board/design.py"],
                             capture_output=True, text=True).stdout.strip()


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
"""

# (name, mode, expected text, mutation). mode: 'structure' wants exit 1 and the text;
# 'report' wants exit 0 and the text in the report; 'gate' wants the text in the gate output
# (--gate order unless the mutation returns another gate).
FAULTS = [
    ("duplicate VER id", "structure", "duplicate ID VER-01",
     lambda d: csv_edit(d, "verification.csv", "VER-01", _dup=True)),
    ("dangling DF id in a gate", "structure", "DF-999 is referenced but not defined",
     lambda d: text_edit(d, "gates.md", "| VER-98 VER-148 DFMEA:COVERAGE |", "| VER-98 VER-148 DFMEA:COVERAGE DF-999 |")),
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
    ("a test (T) PASS with no saved output", "structure", "VER-43: an analysis, a test or a bench result needs its saved output",
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
    ("a gate criterion naming a retired check", "structure", "G-ORD-08: needs VER-11, which is RETIRED",
     lambda d: text_edit(d, "gates.md", "| VER-30 VER-12 |", "| VER-30 VER-12 VER-11 |")),
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
    ("an ORDER check no gate reaches", "structure", "that gate can pass without it",
     lambda d: (text_edit(d, "gates.md", "| PHASE:ORDER | both |", "| VER-01 | both |"),
                text_edit(d, "gates.md", "| PHASE:CASE | both |", "| VER-01 | both |"),
                text_edit(d, "gates.md", "| PHASE:EVT | both |", "| VER-01 | both |"))),
    ("criterion with a bad board", "structure", "board 'nowhere' not in",
     lambda d: text_edit(d, "gates.md", "| PHASE:ORDER | both |", "| PHASE:ORDER | nowhere |")),
    ("new BOM under an old BOARD_REV", "structure", "BOARD_REV is still",
     lambda d: (write(d, "releases/REL-REFA-R1-EVT.md", REL.format(id="REL-REFA-R1-EVT", bom="a" * 64, commit="")),
                write(d, "releases/REL-REFA-R2-DVT.md", REL.format(id="REL-REFA-R2-DVT", bom="b" * 64, commit="")))),
    ("design baseline differs from HEAD", "structure", "design baseline differs from HEAD",
     lambda d: text_edit(d, "README.md", DESIGN_BLOB, "0" * 40)),
    ("design baseline leaves out the routed board", "structure",
     "the design baseline does not name hardware/ref_design/board/battler.kicad_pcb",
     lambda d: text_edit(d, "README.md", "- `hardware/ref_design/board/battler.kicad_pcb`", "- `hardware/ref_design/board/battler_old.kicad_pcb`")),
    ("an advisory-only check made binding", "gate", "VER-40 NOT_RUN",
     lambda d: text_edit(d, "gates.md", "| ADVISORY VER-40 |", "| VER-40 |")),
]


# ---- faults that need git history: run in a scratch clone ----------------------------
def git(repo, *args, env=None):
    r = subprocess.run(["git", "-C", repo] + list(args), capture_output=True, text=True, env=env)
    if r.returncode:
        raise SystemExit(f"seed_faults: git {' '.join(args[:3])} failed in the clone: {r.stderr[-400:]}")
    return r.stdout.strip()


def commit(repo, msg):
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "--no-verify", "-m", msg)
    return git(repo, "rev-parse", "HEAD")


def rebaseline(repo):
    """Write HEAD's object hashes into the clone's README baseline (after a design commit)."""
    p = os.path.join(repo, "hardware/vv/README.md")
    s = open(p, encoding="utf-8").read()
    for path in re.findall(r"^- `(hardware/\S+)` [0-9a-f]{40}\s*$", s, re.M):
        s = re.sub(r"^- `" + re.escape(path) + r"` [0-9a-f]{40}", f"- `{path}` " + git(repo, "rev-parse", f"HEAD:{path}"),
                   s, flags=re.M)
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
        REL.format(id="REL-REFA-R1-EVT", bom="a" * 64, commit=base))
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
    approve(os.path.join(repo, "hardware/vv"), "| A-001 | W-01 | WAIVED | ORDER | the latch replica is cut | Alex | 2026-10-25 |")
    commit(repo, "waive W-01")
    return "order"


def g_claude_waiver(repo, base):
    approve(os.path.join(repo, "hardware/vv"), "| A-001 | W-01 | WAIVED | ORDER | the latch replica is cut | Alex | 2026-10-25 |")
    commit(repo, "waive W-01\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>")
    return None


def g_part_stale(repo, base):
    reopen_closed(repo)
    fake_pass(repo, "VER-99", base)
    commit(repo, "VER-99 passes")
    p = os.path.join(repo, "hardware/ref_design/board/design.py")
    s = open(p).read()
    s = s.replace('C("C1", S, "10u 25V", "VBUS_C1")', 'C("C1", S, "22u 25V", "VBUS_C1")', 1)
    open(p, "w").write(s)
    commit(repo, "C1 to 22u")
    rebaseline(repo)
    commit(repo, "baseline")
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
        REL.format(id="REL-REFA-R1-EVT", bom="a" * 64, commit=base) + f"- Fab zip sha256: {sha}\n")
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
]


def run(d, *extra, trace=TRACE):
    r = subprocess.run([sys.executable, trace] + (["--vv", d] if d else []) + list(extra), capture_output=True,
                       text=True)
    return r.returncode, r.stdout + r.stderr


def fresh_copy(tmp, name):
    d = os.path.join(tmp, re.sub(r"\W+", "_", name))
    shutil.copytree(VV, d, ignore=shutil.ignore_patterns("__pycache__"))
    return d


def judge(mode, rc, out, want):
    if mode == "structure":
        return rc == 1 and want in out
    if mode == "report":
        return rc == 0 and want in out
    return rc == 1 and want in out


def main():
    if "--list" in sys.argv:
        for name, mode, want, _ in FAULTS + GIT_FAULTS:
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
        for name, mode, want, mutate in FAULTS:
            n += 1
            d = fresh_copy(tmp, name)
            mutate(d)
            rc, out = run(d, "--gate", "order") if mode == "gate" else run(d)
            ok = judge(mode, rc, out, want)
            print(f"{'ok' if ok else 'MISSED'}  {mode:9} {name}")
            if not ok:
                missed.append(name)
                print("          exit", rc, "; output tail:", out[-600:].replace("\n", "\n          "))

        # the git-history faults, in a scratch clone of this repository
        repo = os.path.join(tmp, "clone")
        subprocess.run(["git", "clone", "-q", "--shared", ROOT, repo], check=True)
        git(repo, "config", "user.name", "Alex")
        git(repo, "config", "user.email", "alex@example.invalid")
        shutil.rmtree(os.path.join(repo, "hardware/vv"))
        shutil.copytree(VV, os.path.join(repo, "hardware/vv"), ignore=shutil.ignore_patterns("__pycache__"))
        base = commit(repo, "the working register, as a base for seeded faults")
        trace = os.path.join(repo, "hardware/vv/tools/check_trace.py")
        rc, out = run(None, trace=trace)
        clone_ok = rc == 0
        print(f"{'ok' if clone_ok else 'FAIL'}      clean clone exits 0" + ("" if clone_ok else f" (exit {rc})"))
        if not clone_ok:
            print(out[-2000:])
            missed.append("clean clone")
        for name, mode, want, mutate in GIT_FAULTS:
            n += 1
            git(repo, "reset", "-q", "--hard", base)
            git(repo, "clean", "-q", "-fdx", "--", "hardware")
            gate = mutate(repo, base)
            rc, out = run(None, *(["--gate", gate] if gate else []), trace=trace)
            ok = judge(mode, rc, out, want)
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
