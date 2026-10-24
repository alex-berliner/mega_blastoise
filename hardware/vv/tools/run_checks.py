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
header. Add a check here once its script exists (check_bom_cpl, check_fab_content, ...).

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

INRUSH = r"""
import re, sys
sys.path.insert(0, 'hardware/ref_design/board')
import design
UNIT = {'p': 1e-12, 'n': 1e-9, 'u': 1e-6}
def farads(v):
    m = re.match(r'([\d.]+)([pnu])', v)
    return float(m.group(1)) * UNIT[m.group(2)]
caps = {p['ref']: (p['pins'], farads(p['value'])) for p in design.PARTS if p['lib'] == 'Device:C' and p['bom']}
vbus = sum(c for pins, c in caps.values() if set(pins.values()) & {'VBUS', 'VBUS_C1'})
# with no cell, D5 charges VSYS and, through JP3 (bridged), AMP_VDD at plug-in
vsys = sum(c for pins, c in caps.values() if set(pins.values()) & {'VSYS', 'AMP_VDD'})
q = vbus * 5.0 + vsys * 4.6
print(f'VBUS capacitance {vbus * 1e6:.1f} uF (limit 10 uF): {vbus * 5e6:.0f} uC at 5.0 V')
print(f'VSYS + AMP_VDD capacitance {vsys * 1e6:.1f} uF: {vsys * 4.6e6:.0f} uC at 4.6 V through D5')
print(f'plug-in charge with no cell {q * 1e6:.0f} uC (USB 2.0 cl. 7.2.4.1: 50 uC)')
req = open('hardware/vv/requirements.md').read()
blk = req[req.index('### REQ-PWR-13:'):]
blk = blk[:blk.index('\n### ', 5)]
quoted = f'{q * 1e6:.0f} uC' in blk
print('REQ-PWR-13 carries the deviation with this number:', quoted)
sys.exit(0 if vbus <= 10e-6 * 1.001 and quoted else 1)
"""

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
    "VER-99": ("inrush", ["python3", "-c", INRUSH], False),
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
