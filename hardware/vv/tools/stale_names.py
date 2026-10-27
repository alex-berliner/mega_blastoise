#!/usr/bin/env python3
"""Find live register rows that name a part, a part number or a net the design no longer has.

    python3 hardware/vv/tools/stale_names.py              # exit 1 on any stale name
    python3 hardware/vv/tools/stale_names.py --vv <copy>  # another copy of hardware/vv (seed_faults.py)

What 'the design' is: design.py at the baseline commit README.md names ('Order branch: ... from
`<commit>`'). What 'left' means: a name some earlier design.py on that commit's history had and
the baseline's does not. Three kinds of name, read from every design.py on the history:

  ref     a reference designator (Q2, D5, J6, R48-R51 as a range). R6, R7 and R8 written as
          'R6 at', 'R8 fails' and so on are check_trace's gate rules, and D0-D7 after "U8's" or
          "U9's" are the 74HC165's inputs: neither is a part. A ref no design.py on the history
          ever had (a typo: R66, TP17) fails too when its letters are a prefix the baseline uses
          (R, C, U, Q, D, J, JP, TP, SW, H, RN, L, X, TH, FID); a range's ends are read, not the
          numbers between, and a number 0 is a pin name (the 74HC165's 'D0 11'), not a ref.
  net     a net name with an underscore or a digit (LS_GATE, CHG_PROG, REV0). Bare words (TS, ILIM,
          ISET) are also the BQ24073's pin names, so they are not read.
  mpn     a part number's stem, its leading letters and digits (TP4056 from TP4056-42-ESOP8,
          ME6211 from ME6211C33M5G-N), when no baseline part's value or part number has that stem.
          RP2350 is the chip family's datasheet name and stays.

Where: requirements.md (every block not RETIRED, every field but 'Retired'), dfmea.csv (rows not
RETIRED, every column) and verification.csv (rows not RETIRED or CUT: title, procedure,
pass_limit, covers, needs). Text inside a 'Was: ...' clause is history and is not read (the
rule check_trace applies to retired IDs, O-R9).

This is VER-188. check_trace.py's default mode runs it too, so a row written after its evidence
fails the register's structure at once. Python 3 standard library and git.
"""
import csv
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DESIGN_PY = "hardware/ref_design/board/design.py"
WAS = re.compile(r"\bWas:[^.;|]*")
REF_RX = re.compile(r"(?<![\w-])([A-Z]{1,3})(\d{1,3})(?:\s*-\s*([A-Z]{1,3})?(\d{1,3}))?(?![\w])")
GATE_RULE = re.compile(r"(?<![\w-])R[1-8]\b(?=\s+(?:at|fails|holds|counts|passes|reads|compares|of|and|in|is|"
                       r"refuses|wants|asks|follows|\(|,|;|:|\)))")
SR_PIN = re.compile(r"\bU[89]'s D[0-7]\b")
MPN_KEEP = {"RP2350"}                      # the chip family: Raspberry Pi's datasheet and minimal design


def _root(start):
    r = subprocess.run(["git", "-C", start, "rev-parse", "--show-toplevel"], capture_output=True, text=True)
    return r.stdout.strip()


ROOT = _root(HERE)


def git(*args):
    r = subprocess.run(["git", "-C", ROOT] + list(args), capture_output=True, text=True)
    return r.returncode, r.stdout


def baseline_commit(vv):
    s = open(os.path.join(vv, "README.md"), encoding="utf-8").read()
    m = re.search(r"^- Order branch: `[^`]+` from `([0-9a-f]{7,40})`", s, re.M)
    return m.group(1) if m else None


_parts = {}


def parts_at(sha):
    if sha not in _parts:
        rc, src = git("show", f"{sha}:{DESIGN_PY}")
        ps = None
        if rc == 0:
            ns = {"__name__": "design_snapshot"}
            try:
                exec(compile(src, DESIGN_PY, "exec"), ns)
                ps = ns.get("PARTS", [])
            except Exception:                  # an old design.py that will not run is skipped
                ps = None
        _parts[sha] = ps
    return _parts[sha]


def stem(s):
    m = re.match(r"[A-Z]+\d+", s or "")
    return m.group(0) if m else None


def names(sha):
    """(refs, nets, mpn stems) of design.py at a commit."""
    ps = parts_at(sha) or []
    refs = {p["ref"] for p in ps if not p["ref"].startswith("#")}
    nets = {n for p in ps for n in p["pins"].values() if n}
    stems = {stem(str(v)) for p in ps for v in (p.get("mpn"), p.get("value")) if v} - {None}
    return refs, nets, stems


def history(base):
    """The commits on base's history that changed design.py, newest first."""
    rc, out = git("log", "--format=%H", base, "--", DESIGN_PY)
    return out.split()


def ever_refs(base):
    """(every reference designator any design.py on base's history had, the letter prefixes base uses)."""
    refs = set(names(base)[0])
    for sha in history(base):
        refs |= names(sha)[0]
    prefixes = {m.group(1) for m in (re.fullmatch(r"([A-Z]+)\d+", r) for r in names(base)[0]) if m}
    return refs, prefixes


def gone(base):
    """{name: (kind, the newest commit that had it)} for names earlier design.py files had and base has not."""
    refs, nets, stems = names(base)
    out_ = {}
    for sha in history(base):
        r, n, s = names(sha)
        for x in r - refs:
            out_.setdefault(x, ("ref", sha[:7]))
        for x in n - nets:
            if "_" in x or re.search(r"\d", x):
                out_.setdefault(x, ("net", sha[:7]))
        for x in s - stems - MPN_KEEP:
            if len(x) >= 5:
                out_.setdefault(x, ("mpn", sha[:7]))
    return out_


def rows(vv):
    """[(where, id, text)] of every live row's text."""
    out = []
    s = open(os.path.join(vv, "requirements.md"), encoding="utf-8").read()
    for m in re.finditer(r"^### (REQ-[A-Z]{2,4}-\d{2}(?:\.[RP])?): (.*?)(?=^### |^## |\Z)", s, re.M | re.S):
        blk = m.group(0)
        if re.search(r"^- Status: RETIRED", blk, re.M):
            continue
        line = s[:m.start()].count("\n") + 1
        text = "\n".join(ln for ln in blk.splitlines() if not ln.startswith("- Retired:"))
        out.append((f"requirements.md:{line}", m.group(1), text))
    with open(os.path.join(vv, "dfmea.csv"), newline="", encoding="utf-8") as f:
        for n, r in enumerate(csv.DictReader(f), 2):
            if r.get("status") != "RETIRED":
                out.append((f"dfmea.csv:{n}", r["id"], " | ".join(v for k, v in r.items() if k != "id")))
    with open(os.path.join(vv, "verification.csv"), newline="", encoding="utf-8") as f:
        for n, r in enumerate(csv.DictReader(f), 2):
            if r.get("last_result") not in ("RETIRED", "CUT"):
                out.append((f"verification.csv:{n}", r["id"],
                            " | ".join(r.get(k, "") for k in ("title", "procedure", "pass_limit", "covers", "needs"))))
    return out


def findings(vv):
    base = baseline_commit(vv)
    if not base:
        return [("README.md", "", "no 'Order branch: ... from `<commit>`' line: no baseline to read design.py at")]
    if parts_at(base) is None:
        return [("README.md", "", f"design.py does not load at the baseline {base}")]
    old = gone(base)
    refs = {k for k, (kind, _) in old.items() if kind == "ref"}
    nets = {k for k, (kind, _) in old.items() if kind == "net"}
    mpns = {k for k, (kind, _) in old.items() if kind == "mpn"}
    net_rx = re.compile(r"(?<![\w/])(" + "|".join(sorted(map(re.escape, nets), key=len, reverse=True)) + r")(?!\w)") \
        if nets else None
    mpn_rx = re.compile(r"(?<![\w-])(" + "|".join(sorted(map(re.escape, mpns), key=len, reverse=True)) + r")\w*") \
        if mpns else None
    ever, prefixes = ever_refs(base)
    out = []
    for where, rid, text in rows(vv):
        t = WAS.sub(" ", text)
        t = SR_PIN.sub(" ", GATE_RULE.sub(" ", t))
        hit = set()
        for m in REF_RX.finditer(t):
            a, n1, b, n2 = m.groups()
            if n2 and b in (None, a) and int(n1) <= int(n2):
                named = {f"{a}{i}" for i in range(int(n1), int(n2) + 1)}
                ends = {a + n1, a + n2}
            else:
                named = ends = {a + n1} | ({b + n2} if n2 and b else set())
            hit |= {f"{x} (part, last in {old[x][1]})" for x in named & refs}
            # a ref no design.py ever had (a typo such as R66 or TP17), with a prefix the baseline uses: only a
            # range's ends are read, so a range over a gap in the numbering (C10-C22) is not a typo; a number 0
            # is a pin name, never a ref (the 74HC165's 'D0 11' in a pin list)
            hit |= {f"{x} (part, never in design.py)" for x in ends - ever
                    if (lambda p: p.group(1) in prefixes and int(p.group(2)) > 0)(re.fullmatch(r"([A-Z]+)(\d+)", x))}
        if net_rx:
            hit |= {f"{m.group(1)} (net, last in {old[m.group(1)][1]})" for m in net_rx.finditer(t)}
        if mpn_rx:
            hit |= {f"{m.group(0)} (part number, last in {old[m.group(1)][1]})" for m in mpn_rx.finditer(t)}
        if hit:
            out.append((where, rid, "names what design.py at " + base[:7] + " does not have: " + ", ".join(sorted(hit))))
    return out


def main():
    vv = os.path.dirname(HERE)
    if "--vv" in sys.argv:
        vv = os.path.abspath(sys.argv[sys.argv.index("--vv") + 1])
    base = baseline_commit(vv)
    old = gone(base) if base and parts_at(base) is not None else {}
    print(f"baseline {base}: {sum(1 for k in old.values() if k[0] == 'ref')} refs, "
          f"{sum(1 for k in old.values() if k[0] == 'net')} nets and "
          f"{sum(1 for k in old.values() if k[0] == 'mpn')} part-number stems left design.py on its history")
    f = findings(vv)
    for where, rid, msg in f:
        print(f"  {where} {rid}: {msg}")
    print(f"{len(f)} live rows name a part, part number or net the baseline does not have")
    print("RESULT: " + ("PASS" if not f else "FAIL"))
    return 1 if f else 0


if __name__ == "__main__":
    sys.exit(main())
