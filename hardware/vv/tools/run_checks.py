#!/usr/bin/env python3
"""Run the quick scripted checks, save their evidence with a header, record the result.

    python3 hardware/vv/tools/run_checks.py               # every quick check below
    python3 hardware/vv/tools/run_checks.py VER-01 VER-04 # some of them
    python3 hardware/vv/tools/run_checks.py --dry-run     # run and print; write nothing

For each check it runs the command from the repository root, writes
hardware/vv/evidence/<date>_<VER>_<name>.txt with the header check_trace.py demands
(COMMIT, TOOLS, the command, its output, its exit code, RESULT), and writes PASS or FAIL,
the evidence path, the commit and the date into verification.csv, then rewrites grid.md (tools/grid.py)
so the coverage grid shows the new results. It refuses a check whose
covered files have uncommitted changes: evidence describes a commit, so commit first.

Only quick, read-only checks belong here. The long pipelines (route.py, check_buttons.py,
board/sim/, the fit kit, case/export.py --stl) keep a hand-run evidence file with the same
header. Add a check here once its script exists (check_fab_content, ...). A script that exits 2
is BLOCKED (an input it needs is not on disk): nothing is written for it. A
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
PC = "hardware/vv/tools/paper_checks.py"
CC = "hardware/vv/tools/circuit_checks.py"
CS = "hardware/vv/tools/case_checks.py"
LC = "hardware/vv/tools/layout_checks.py"
TH = "hardware/vv/tools/thermal.py"

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
    "VER-32": ("tempwin", ["python3", DN, "tempwin"], False),
    "VER-138": ("cutoff", ["python3", DN, "cutoff"], False),
    "VER-142": ("mlcc", ["python3", DN, "mlcc"], True),
    "VER-147": ("screws", ["python3", DN, "screws"], False),
    "VER-164": ("th1", ["python3", DN, "th1"], True),
    "VER-167": ("bom", ["python3", DN, "bom"], False),
    "VER-176": ("cpl", ["python3", DN, "cpl"], True),
    "VER-177": ("vbus", ["python3", DN, "vbus"], False),
    "VER-18": ("edge", ["python3", DN, "edge"], True),
    "VER-184": ("plugs", ["python3", DN, "plugs"], True),
    "VER-188": ("stale_names", ["python3", "hardware/vv/tools/stale_names.py"], False),
    # the paper checks of classes 4, 6, 9 and 11 (2026-10-27); exit 2 is BLOCKED: nothing is recorded
    "VER-33": ("latch", ["python3", PC, "latch"], False),
    "VER-191": ("usb", ["python3", PC, "usb"], True),
    "VER-100": ("audio", ["python3", PC, "audio"], False),
    "VER-13": ("copper", ["python3", PC, "copper"], True),
    "VER-103": ("shelf", ["python3", PC, "shelf"], False),
    "VER-30": ("mlccbias", ["python3", PC, "mlccbias"], False),
    "VER-197": ("core", ["python3", PC, "core"], True),
    "VER-12": ("hotplug", ["python3", PC, "hotplug"], False),
    "VER-196": ("ampstate", ["python3", PC, "ampstate"], False),
    "VER-140": ("mating", ["python3", PC, "mating"], False),
    "VER-159": ("check_backpower", ["python3", "hardware/vv/tools/check_backpower.py"], False),
    "VER-162": ("check_placed_contacts", ["python3", "hardware/vv/tools/check_placed_contacts.py"], True),
    "VER-125": ("check_land", ["python3", "hardware/vv/tools/check_land.py"], True),
    "VER-130": ("check_bom_cpl", ["python3", "hardware/vv/tools/check_bom_cpl.py"], True),
    # the checks of classes 2, 3, 5, 7, 10 and 12-15 (2026-10-27); exit 2 is BLOCKED: nothing is recorded
    "VER-198": ("appcircuit", ["python3", CC, "appcircuit"], True),
    "VER-190": ("arch", ["python3", CS, "arch"], False),
    "VER-15": ("gpio", ["python3", CC, "gpio"], False),
    "VER-36": ("pinreview", ["python3", CC, "pinreview"], False),
    "VER-149": ("fwpins", ["python3", CC, "fwpins"], False),
    "VER-31": ("ratings", ["python3", CC, "ratings"], False),
    "VER-195": ("usbfault", ["python3", CC, "usbfault"], False),
    "VER-148": ("bridges", ["python3", LC, "bridges"], True),
    "VER-132": ("fab", ["python3", LC, "fab"], True),
    "VER-17": ("jlc", ["python3", LC, "jlc"], True),
    "VER-21": ("ret", ["python3", LC, "ret"], True),
    "VER-108": ("usbpair", ["python3", LC, "usbpair"], True),
    "VER-199": ("timing", ["python3", CC, "timing"], True),
    "VER-06": ("check_fit", ["python3", CS, "fit"], True),
    "VER-19": ("placement", ["python3", CS, "placement"], True),
    "VER-20": ("stack", ["python3", CS, "stack"], True),
    "VER-152": ("estimates", ["python3", CS, "estimates"], False),
    "VER-165": ("mouths", ["python3", CS, "mouths"], True),
    "VER-174": ("xy", ["python3", CS, "xy"], False),
    "VER-144": ("heat_modes", ["python3", TH, "modes"], False),
    "VER-193": ("heat_board", ["python3", TH, "board"], False),
    "VER-200": ("heat_amp", ["python3", TH, "amp"], False),
    # last, and on its own after the others are written (seed_faults copies the working register)
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
        if r.returncode == 2:
            print(f"{vid} {name}: BLOCKED (exit 2): nothing recorded\n" + r.stdout[-600:])
            continue
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
        # grid.md shows each cell's worst status: it is generated, so rewrite it with the new results
        g = subprocess.run([sys.executable, os.path.join(HERE, "grid.py")], capture_output=True, text=True)
        print(g.stdout.strip() or g.stderr.strip())
    print(f"{len(want)} checks, {bad} not passing or refused")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
