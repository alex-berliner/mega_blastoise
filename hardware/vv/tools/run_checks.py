#!/usr/bin/env python3
"""Run the quick scripted checks, save their evidence with a header, record the result.

    python3 hardware/vv/tools/run_checks.py               # every quick check below
    python3 hardware/vv/tools/run_checks.py VER-01 VER-04 # some of them
    python3 hardware/vv/tools/run_checks.py --dry-run     # run and print; write nothing

For each check it runs the command from the repository root, writes
hardware/vv/evidence/<date>_<VER>_<name>.txt with the header check_trace.py demands
(COMMIT, TOOLS, the command, its output, its exit code, RESULT), and writes PASS or FAIL,
the evidence path, the commit and the date into verification.csv. It refuses a check whose
covered files have uncommitted changes: evidence describes a commit, so commit first.

Only quick, read-only checks belong here. The long pipelines (route.py, check_buttons.py,
board/sim/, the fit kit, case/export.py --stl) keep a hand-run evidence file with the same
header. Add a check here once its script exists (check_bom_cpl, check_fab_content, ...). A
check that FAILs is recorded as FAIL with its evidence: a finding the order gate shows. A
script that tests only part of its row's pass limit does not belong here: split the row first
(VER-20 and VER-182, O-R21), so an exit 0 is a whole PASS.

Python 3 standard library only.
"""
import csv
import datetime
import io
import os
import platform
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import check_trace as ct  # noqa: E402

ROOT = ct.ROOT
REG = os.path.join(ct.VV, "verification.csv")

DN = "hardware/vv/tools/design_numbers.py"

# VER id: (evidence name, command, uses kicad-cli)
CHECKS = {
    "VER-01": ("check_netlist", ["python3", "hardware/ref_design/board/tools/check_netlist.py"], True),
    "VER-02": ("check_pins", ["python3", "hardware/ref_design/board/tools/check_pins.py"], False),
    "VER-03": ("erc", ["kicad-cli", "sch", "erc", "--severity-all", "--exit-code-violations", "-o", "{tmp}/erc.rpt",
                       "hardware/ref_design/board/battler.kicad_sch"], True),
    "VER-04": ("drc", ["python3", "hardware/ref_design/board/tools/check_pcb.py"], True),
    "VER-08": ("check_contact_fp", ["python3", "hardware/membrane/tools/check_contact_fp.py",
                                    "hardware/ref_design/board/lib/battler.pretty/DSL_button_4ring.kicad_mod"], False),
    "VER-10": ("check_contact_fp", ["python3", "hardware/membrane/tools/check_contact_fp.py",
                                    "hardware/prod_board/prod_board/prod.pretty/DSL_button_4ring.kicad_mod"], False),
    # the pre-order numbers (design_numbers.py); pcbnew ones record KiCad's version
    "VER-182": ("swell", ["python3", DN, "swell"], False),
    "VER-181": ("cellgap", ["python3", DN, "cellgap"], False),
    "VER-99": ("inrush", ["python3", DN, "inrush"], False),
    "VER-114": ("holes", ["python3", DN, "holes"], True),
    "VER-131": ("drill", ["python3", DN, "drill"], True),
    "VER-137": ("backlight", ["python3", DN, "backlight"], False),
    "VER-138": ("cutoff", ["python3", DN, "cutoff"], False),
    "VER-142": ("mlcc", ["python3", DN, "mlcc"], True),
    "VER-147": ("screws", ["python3", DN, "screws"], False),
    "VER-164": ("th1", ["python3", DN, "th1"], True),
    "VER-167": ("bom", ["python3", DN, "bom"], False),
    "VER-176": ("cpl", ["python3", DN, "cpl"], True),
    "VER-177": ("vbus", ["python3", DN, "vbus"], False),
    "VER-129": ("seed_faults", ["python3", "hardware/vv/tools/seed_faults.py"], False),
}


def tool_versions(kicad):
    out = [f"python {platform.python_version()}"]
    if kicad:
        r = subprocess.run(["kicad-cli", "--version"], capture_output=True, text=True)
        out.append(f"kicad-cli {r.stdout.strip() or 'unknown'}")
    return "; ".join(out)


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    dry = "--dry-run" in sys.argv
    want = args or list(CHECKS)
    rows = list(csv.reader(open(REG, newline="", encoding="utf-8")))
    head = rows[0]
    byid = {r[0]: r for r in rows[1:] if r}
    dirty = ct.dirty_files()
    commit = ct.HEAD
    today = datetime.date.today().isoformat()
    bad = 0
    for vid in want:
        if vid not in CHECKS:
            sys.exit(f"run_checks: {vid} is not a quick scripted check here")
        name, cmd, kicad = CHECKS[vid]
        row = dict(zip(head, byid[vid]))
        cov = ct.parse_covers(row["covers"])
        hit = sorted({f for k, s, _ in cov if k == "glob" for f in dirty if s[0].match(f)}
                     | ({ct.DESIGN_PY} & dirty if any(k == "part" for k, _, _ in cov) else set()))
        if hit and not dry:
            print(f"{vid}: refused: uncommitted changes under its covers: {' '.join(hit)}")
            bad += 1
            continue
        with tempfile.TemporaryDirectory() as tmp:
            argv = [a.replace("{tmp}", tmp) for a in cmd]
            r = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True)
            report = ""
            if vid == "VER-03" and os.path.exists(os.path.join(tmp, "erc.rpt")):
                report = "--- report\n" + open(os.path.join(tmp, "erc.rpt"), encoding="utf-8").read()
        result = "PASS" if r.returncode == 0 else "FAIL"
        shown = " ".join(a if "\n" not in a else "'<inline script below>'" for a in cmd)
        body = io.StringIO()
        body.write(f"# {vid} {name}\nDATE: {today}\nCOMMIT: {commit}\n")
        body.write(f"TREE: {'clean under its covers' if not hit else 'DIRTY: ' + ' '.join(hit)}\n")
        body.write(f"TOOLS: {tool_versions(kicad)}\n$ {shown}\n")
        if any("\n" in a for a in cmd):
            body.write("--- inline script\n" + next(a for a in cmd if "\n" in a).strip() + "\n--- output\n")
        body.write(r.stdout + (("\n--- stderr\n" + r.stderr) if r.stderr.strip() else ""))
        body.write(("\n" if not body.getvalue().endswith("\n") else "") + report)
        body.write(f"\nexit {r.returncode}\nRESULT: {result}\n")
        rel = f"hardware/vv/evidence/{today}_{vid}_{name}.txt"
        print(f"{vid} {name}: {result}" + ("" if not dry else " (dry run)"))
        if r.returncode != 0:
            bad += 1
        if dry:
            print(body.getvalue()[-800:])
            continue
        open(os.path.join(ROOT, rel), "w", encoding="utf-8").write(body.getvalue())
        rr = byid[vid]
        rr[head.index("last_result")] = result
        rr[head.index("evidence")] = rel
        rr[head.index("commit")] = commit[:7]
        rr[head.index("date")] = today
    if not dry:
        buf = io.StringIO()
        csv.writer(buf, lineterminator="\n").writerows(rows)
        open(REG, "w", encoding="utf-8").write(buf.getvalue())
    print(f"{len(want)} checks, {bad} not passing or refused")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
