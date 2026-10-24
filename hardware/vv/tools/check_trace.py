#!/usr/bin/env python3
"""The trace gate for hardware/vv: requirements, DFMEA, checks, defects, approvals and gates.

    python3 hardware/vv/tools/check_trace.py                     # structure + report
    python3 hardware/vv/tools/check_trace.py --gate order        # exit 1 unless the order gate is met
    python3 hardware/vv/tools/check_trace.py --gate case         # the printed-case gate
    python3 hardware/vv/tools/check_trace.py --gate evt --board prod_board
    python3 hardware/vv/tools/check_trace.py --shopping [--gate evt]   # what to buy, longest lead first
    python3 hardware/vv/tools/check_trace.py --verbose           # every ID in every list

Default mode exits 1 only on a structural error. Structure means: IDs well formed, unique
and defined wherever they are referenced; rows and fields well formed; the action priority
agrees with the AIAG-VDA table; a detection rating is no better than its 'detects' checks
allow; an occurrence of 2 or less is backed by built hardware (or, at 1, by a stated
physical exclusion); a PASS or FAIL carries a commit, a date and an evidence file whose
header matches it; a 'covers' pattern matches a tracked file and, with a regex, a line in
it; a check marked 'both' covers both boards; every check is reached by some gate,
requirement or high-priority risk no later than its own phase; a status of APPROVED,
REJECTED, DEFERRED, ACCEPTED or a signed waiver has a matching line in
releases/APPROVALS.md that Alex committed; an approved requirement's statement has not
changed since; two release records of one board with different BOMs have different
BOARD_REVs; no defect is CLOSED without a check. Everything else is a report.

--gate PHASE exits 1 unless every criterion of that gate in gates.md is met for the board
(default ref_design), plus these rules at every gate:
  R1  no requirement in scope is still PROPOSED;
  R2  every requirement in scope lists a check for the board, and every listed check whose
      phase is at or before the gate has PASSED on fresh evidence;
  R3  every H-priority DFMEA row in scope is ACCEPTED for this phase or a later one, or
      DONE with every one of its 'detects' checks at or before the gate PASSED, fresh;
  R4  no defect of the board is OPEN (order, case, EVT); every one is CLOSED (DVT, PVT);
  R5  no PASSED check covers a file with uncommitted changes;
  R6  every commit since the newest release record that touches the design is named in a
      change record (ref_design);
  R7  the design files at HEAD are the ones README.md's design baseline names (ref_design).

Scope: for ref_design, requirements and DFMEA rows marked ref_design or both; for
prod_board, only those marked prod_board (REQ-PBD-* and the .P allocations Alex writes).
A product requirement split per board (REQ-X and REQ-X.R / REQ-X.P) is verified through
the child for the board.

Evidence is fresh when its commit is an ancestor of HEAD, nothing it covers changed since,
and it is younger than the check's valid_days. 'covers' tokens:
  path/glob[::regex]   a file changed (with a regex: a changed line matching \\b(regex)\\b)
  part:R52,R53,C10-C22 a listed part's value, LCSC number, footprint or pins in design.py
  release:REL-...      a bench result on built units: stale only when a change record
                       made after it names the check under 'What to re-run'

Python 3 standard library only.
"""
import argparse
import csv
import datetime
import os
import re
import subprocess
import sys
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
VV = os.path.dirname(HERE)              # replaced by --vv (tools/seed_faults.py uses it)

PHASES = ["ORDER", "CASE", "EVT", "DVT", "PVT", "SALE"]
GATE_ARG = {"order": "ORDER", "case": "CASE", "evt": "EVT", "dvt": "DVT", "pvt": "PVT"}
GATE_PREFIX = {"ORDER": "G-ORD", "CASE": "G-CASE", "EVT": "G-EVT", "DVT": "G-DVT", "PVT": "G-PVT"}
PREFIX_PHASE = {v: k for k, v in GATE_PREFIX.items()}
BOARDS = ("ref_design", "prod_board")
BOARD_VALUES = ("ref_design", "prod_board", "both")
BOARD_ROOTS = {"ref_design": ("hardware/ref_design/",),
               "prod_board": ("hardware/prod_board/", "hardware/prod_case/")}
SHARED_ROOTS = ("hardware/vv/", "hardware/membrane/")
METHODS = set("TAID")
LEVELS = ("product", "subsystem", "board")
REQ_STATUS = ("PROPOSED", "APPROVED", "REJECTED", "DEFERRED")
VER_RESULT = ("PASS", "FAIL", "NOT_RUN", "NOT_BUILT", "CUT")
DF_STATUS = ("OPEN", "DONE", "ACCEPTED")
DEF_STATUS = ("OPEN", "FIXED", "CLOSED")
VP_STATUS = ("yes", "partly", "no", "simulated")
VP_METHODS = 218                         # validation_plan.md is archived with 218 methods
DESIGN_PY = "hardware/ref_design/board/design.py"
DESIGN_PATHS = ("hardware/ref_design/board/design.py", "hardware/ref_design/board/placement.py",
                "hardware/ref_design/board/lib/", "hardware/ref_design/case/case.scad")
BASELINE_FOR = {"ORDER": ("board/design.py", "board/placement.py"), "CASE": ("case/case.scad",)}
EVIDENCE_DIR = "hardware/vv/evidence/"
CLAUDE_TRAILER = re.compile(r"^(Co-Authored-By: Claude|Claude-Session:)", re.M | re.I)

RE = {
    "REQ": r"REQ-[A-Z]{2,4}-\d{2}(?:\.[RP])?",
    "DF": r"DF-\d{2,3}",
    "VER": r"VER-\d{2,3}",
    "DEF": r"DEF-\d{2,3}",
    "CHG": r"CHG-\d{2,3}",
    "REL": r"REL-[A-Z0-9]+(?:-[A-Z0-9]+)+",
    "G": r"G-(?:ORD|CASE|EVT|DVT|PVT)-\d{2}",
    "W": r"W-\d{2}",
    "A": r"A-\d{3}",
}
ANY_ID = re.compile(r"(?<![\w-])(" + "|".join(RE[k] for k in ("REQ", "DF", "VER", "DEF", "CHG", "G", "W"))
                    + r"|REL-[A-Z0-9]+(?:-[A-Z0-9]+)+)(?![\w-])")
VER_ID = re.compile(r"(?<![\w-])VER-\d{2,3}(?![\w-])")
REQ_ID = re.compile(r"(?<![\w-])" + RE["REQ"] + r"(?![\w-])")
DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
TODAY = datetime.date.today()

ERRORS, WARNINGS = [], []


def err(where, msg):
    ERRORS.append(f"{where}: {msg}")


def warn(where, msg):
    WARNINGS.append(f"{where}: {msg}")


# ---- git -------------------------------------------------------------------------
def _toplevel(path):
    return subprocess.run(["git", "-C", path, "rev-parse", "--show-toplevel"],
                          capture_output=True, text=True).stdout.strip()


ROOT = _toplevel(HERE)
if not ROOT:
    sys.exit("check_trace: not inside a git work tree")


def git(*args):
    r = subprocess.run(["git", "-C", ROOT] + list(args), capture_output=True, text=True)
    return r.returncode, r.stdout


TRACKED = set(git("ls-files", "--cached", "--others", "--exclude-standard")[1].split("\n")) - {""}
HEAD = git("rev-parse", "HEAD")[1].strip()
_resolved, _diff, _lines, _anc, _msg, _blame, _design = {}, {}, {}, {}, {}, {}, {}


def dirty_files():
    out = git("status", "--porcelain", "--untracked-files=no")[1]
    return {line[3:].split(" -> ")[-1] for line in out.splitlines() if line.strip()}


def resolve(commit):
    if commit not in _resolved:
        rc, out = git("rev-parse", "--verify", "--quiet", commit + "^{commit}")
        _resolved[commit] = out.strip() if rc == 0 else None
    return _resolved[commit]


def is_ancestor(a, b="HEAD"):
    if (a, b) not in _anc:
        _anc[(a, b)] = git("merge-base", "--is-ancestor", a, b)[0] == 0
    return _anc[(a, b)]


def changed_since(commit):
    if commit not in _diff:
        _diff[commit] = set(git("diff", "--name-only", commit, "HEAD")[1].split("\n")) - {""}
    return _diff[commit]


def changed_lines(commit, path):
    key = (commit, path)
    if key not in _lines:
        out = git("diff", "-U0", commit, "HEAD", "--", path)[1]
        _lines[key] = [ln[1:] for ln in out.splitlines()
                       if ln[:1] in "+-" and not ln.startswith("+++") and not ln.startswith("---")]
    return _lines[key]


def commit_message(sha):
    if sha not in _msg:
        _msg[sha] = git("log", "-1", "--format=%B", sha)[1]
    return _msg[sha]


def blame(relpath):
    """{line number: commit sha} for a tracked file; uncommitted lines get 40 zeros."""
    if relpath not in _blame:
        out = {}
        rc, text = git("blame", "--porcelain", "--", relpath)
        if rc == 0:
            sha = None
            for ln in text.splitlines():
                m = re.match(r"^([0-9a-f]{40}) \d+ (\d+)", ln)
                if m:
                    sha, final = m.group(1), int(m.group(2))
                elif ln.startswith("\t") and sha:
                    out[final] = sha
        _blame[relpath] = out
    return _blame[relpath]


def vv_rel(*parts):
    """Repo-relative path of a file in the register directory, or None when --vv points
    outside the repository."""
    p = os.path.relpath(os.path.join(VV, *parts), ROOT)
    return None if p.startswith("..") else p


def glob_re(pat):
    i, out = 0, ""
    while i < len(pat):
        if pat.startswith("**/", i):
            out, i = out + "(?:.*/)?", i + 3
        elif pat.startswith("**", i):
            out, i = out + ".*", i + 2
        elif pat[i] == "*":
            out, i = out + "[^/]*", i + 1
        elif pat[i] == "?":
            out, i = out + "[^/]", i + 1
        else:
            out, i = out + re.escape(pat[i]), i + 1
    return re.compile(out + r"\Z")


def expand_refs(spec):
    out = []
    for tok in spec.split(","):
        tok = tok.strip()
        m = re.fullmatch(r"([A-Z]+)(\d+)-([A-Z]+)?(\d+)", tok)
        if m and m.group(3) in (None, m.group(1)) and int(m.group(2)) <= int(m.group(4)):
            out += [f"{m.group(1)}{i}" for i in range(int(m.group(2)), int(m.group(4)) + 1)]
        elif tok:
            out.append(tok)
    return out


def parse_covers(text):
    """[(kind, spec, original token)]. kind glob: spec (path regex, line regex or None);
    part: a list of refs; release: a REL id. Quotes around a regex are stripped and the
    regex is matched as whole words, so 'R5' does not match R52."""
    out = []
    for tok in text.split():
        tok = tok.strip("\"'")
        if tok.startswith("part:"):
            out.append(("part", expand_refs(tok[5:]), tok))
        elif tok.startswith("release:"):
            out.append(("release", tok[8:], tok))
        else:
            path, _, rx = tok.partition("::")
            rx = rx.strip("\"'")
            try:
                crx = re.compile(r"\b(?:" + rx + r")\b") if rx else None
            except re.error as e:
                err("verification.csv", f"covers {tok!r}: bad regex ({e})")
                crx = None
            out.append(("glob", (glob_re(path), crx), tok))
    return out


def design_parts(commit=None):
    """{ref: part dict} from design.py at a commit (None: the working tree), or None."""
    if commit not in _design:
        src = None
        if commit is None:
            p = os.path.join(ROOT, DESIGN_PY)
            src = open(p, encoding="utf-8").read() if os.path.exists(p) else None
        else:
            rc, out = git("show", f"{commit}:{DESIGN_PY}")
            src = out if rc == 0 else None
        parts = None
        if src:
            ns = {"__name__": "design_snapshot"}
            try:
                exec(compile(src, DESIGN_PY, "exec"), ns)
                parts = {p["ref"]: p for p in ns.get("PARTS", [])}
            except Exception:                      # an old design.py that will not run
                parts = None
        _design[commit] = parts
    return _design[commit]


def part_key(p):
    if p is None:
        return None
    return (p.get("value"), p.get("lcsc"), p.get("fp"), p.get("mpn"), p.get("dnp"), p.get("bom"),
            tuple(sorted((str(k), v or "") for k, v in p.get("pins", {}).items())))


# ---- AIAG-VDA action priority (DFMEA) -----------------------------------------------
def action_priority(s, o, d):
    """The AIAG-VDA FMEA handbook (2019) action-priority table, design FMEA."""
    if s >= 9:
        if o >= 6:
            return "H"
        if o >= 4:
            return "H" if d >= 2 else "M"
        if o >= 2:
            return "H" if d >= 7 else ("M" if d >= 5 else "L")
        return "L"
    if s >= 7:
        if o >= 8:
            return "H"
        if o >= 6:
            return "H" if d >= 2 else "M"
        if o >= 4:
            return "H" if d >= 7 else "M"
        if o >= 2:
            return "M" if d >= 5 else "L"
        return "L"
    if s >= 4:
        if o >= 8:
            return "H" if d >= 5 else "M"
        if o >= 6:
            return "M" if d >= 2 else "L"
        if o >= 4:
            return "M" if d >= 7 else "L"
        return "L"
    if s >= 2:
        if o >= 8:
            return "M" if d >= 5 else "L"
        return "L"
    return "L"


# ---- parsers ---------------------------------------------------------------------
def read_csv(name, columns):
    path = os.path.join(VV, name)
    if not os.path.exists(path):
        err(name, "missing")
        return []
    with open(path, newline="", encoding="utf-8") as f:
        rows = list(csv.reader(f))
    if not rows:
        err(name, "empty")
        return []
    if rows[0] != columns:
        err(name, f"header is {rows[0]}, want {columns}")
        return []
    out = []
    for n, r in enumerate(rows[1:], 2):
        if not any(c.strip() for c in r):
            continue
        if len(r) != len(columns):
            err(f"{name}:{n}", f"{len(r)} fields, want {len(columns)}")
            continue
        d = {k: v.strip() for k, v in zip(columns, r)}
        d["_where"] = f"{name}:{n}"
        out.append(d)
    return out


def parse_blocks(name, prefix_re, fields_required):
    """Markdown blocks: '### ID: title' then '- Field: value' lines."""
    path = os.path.join(VV, name)
    if not os.path.exists(path):
        err(name, "missing")
        return {}
    blocks, cur = {}, None
    head_re = re.compile(r"^### (\S+?):? (.+)$")
    for n, line in enumerate(open(path, encoding="utf-8"), 1):
        line = line.rstrip("\n")
        if line.startswith("#"):
            cur = None
            m = head_re.match(line)
            if m and re.match(prefix_re.split("-")[0] + "-", m.group(1)):
                bid = m.group(1)
                if not re.fullmatch(prefix_re, bid):
                    err(f"{name}:{n}", f"malformed ID {bid!r}")
                    continue
                if bid in blocks:
                    err(f"{name}:{n}", f"duplicate ID {bid}")
                    continue
                cur = blocks[bid] = {"id": bid, "title": m.group(2), "_where": f"{name}:{n}", "f": {}, "lines": {}}
            continue
        if cur is not None:
            m = re.match(r"^- ([A-Za-z][A-Za-z ]*?): (.*)$", line)
            if m:
                cur["f"][m.group(1)] = m.group(2).strip()
                cur["lines"][m.group(1)] = [n]
            elif line.startswith("  ") and cur["f"]:
                last = list(cur["f"])[-1]
                cur["f"][last] += " " + line.strip()
                cur["lines"][last].append(n)
    for b in blocks.values():
        for k in fields_required:
            if not b["f"].get(k):
                err(b["_where"], f"{b['id']}: field '{k}' missing or empty")
    return blocks


def table_rows(path):
    """(line number, cells) for every markdown table row of a file."""
    if not os.path.exists(path):
        return []
    out = []
    for n, line in enumerate(open(path, encoding="utf-8"), 1):
        if line.startswith("|"):
            out.append((n, [c.strip() for c in line.strip().strip("|").split("|")]))
    return out


def parse_gates():
    path = os.path.join(VV, "gates.md")
    crit, waivers = {}, []
    if not os.path.exists(path):
        err("gates.md", "missing")
        return crit, waivers
    for n, cells in table_rows(path):
        if re.match(r"G-", cells[0]):
            if not re.fullmatch(RE["G"], cells[0]):
                err(f"gates.md:{n}", f"malformed criterion ID {cells[0]!r}")
                continue
            if len(cells) < 4:
                err(f"gates.md:{n}", f"{cells[0]}: want | ID | criterion | needs | board |")
                continue
            if cells[0] in crit:
                err(f"gates.md:{n}", f"duplicate ID {cells[0]}")
                continue
            if cells[3] not in BOARD_VALUES:
                err(f"gates.md:{n}", f"{cells[0]}: board {cells[3]!r} not in {BOARD_VALUES}")
            phase = PREFIX_PHASE[cells[0].rsplit("-", 1)[0]]
            crit[cells[0]] = {"id": cells[0], "text": cells[1], "needs": cells[2].split(), "board": cells[3],
                              "phase": phase, "advisory": "ADVISORY" in cells[2].split(), "_where": f"gates.md:{n}"}
        elif re.match(r"W-", cells[0]):
            if not re.fullmatch(RE["W"], cells[0]) or len(cells) < 3:
                err(f"gates.md:{n}", "waiver row wants | W-NN | G-... | reason |")
                continue
            waivers.append({"id": cells[0], "crit": cells[1], "reason": cells[2], "_where": f"gates.md:{n}"})
    return crit, waivers


def parse_records():
    """releases/REL-*.md and releases/CHG-*.md: '# ID' or '# ID: title' as the first heading."""
    d = os.path.join(VV, "releases")
    recs = {}
    if not os.path.isdir(d):
        return recs
    for fn in sorted(os.listdir(d)):
        if not fn.endswith(".md") or "TEMPLATE" in fn or fn == "APPROVALS.md":
            continue
        text = open(os.path.join(d, fn), encoding="utf-8").read()
        m = re.search(r"^# (\S+?)(?::|\s|$)", text, re.M)
        rid = m.group(1) if m else None
        if not rid or not (re.fullmatch(RE["REL"], rid) or re.fullmatch(RE["CHG"], rid)):
            err(f"releases/{fn}", "first heading must be '# REL-...' or '# CHG-NN: title'")
            continue
        if rid in recs:
            err(f"releases/{fn}", f"duplicate ID {rid}")
            continue

        def field(name):
            fm = re.search(r"^- " + re.escape(name) + r":[ \t]*(.*)$", text, re.M)
            return fm.group(1).strip() if fm else ""
        rec = {"id": rid, "text": text, "_where": f"releases/{fn}", "kind": rid.split("-")[0],
               "board": field("Board"), "commit": field("Commit (full hash)"),
               "fab_sha": field("Fab zip sha256"), "bom_sha": field("BOM sha256"), "board_rev": field("BOARD_REV"),
               "commits": re.findall(r"\b[0-9a-f]{7,40}\b", field("Commit(s)"))}
        sect = re.search(r"^## What to re-run\s*$(.*?)(?=^## |\Z)", text, re.M | re.S)
        rec["reruns"] = set(VER_ID.findall(sect.group(1))) if sect else set()
        recs[rid] = rec
    return recs


def parse_approvals():
    """releases/APPROVALS.md rows: | A-NNN | item | decision | scope | reason | by | date |.
    Only lines Alex committed count: a line whose commit carries a Claude trailer is an error,
    and an uncommitted line is pending."""
    path = os.path.join(VV, "releases", "APPROVALS.md")
    rel = vv_rel("releases", "APPROVALS.md")
    bl = blame(rel) if rel and rel in TRACKED else {}
    rows, seen = [], set()
    for n, cells in table_rows(path):
        if not re.match(r"A-", cells[0]):
            continue
        w = f"releases/APPROVALS.md:{n}"
        if not re.fullmatch(RE["A"], cells[0]) or len(cells) != 7:
            err(w, "approval row wants | A-NNN | item | decision | scope | reason | by | date |")
            continue
        aid, item, dec, scope, reason, who, date = cells
        if aid in seen:
            err(w, f"duplicate ID {aid}")
            continue
        seen.add(aid)
        kind = item.split("-")[0]
        want = {"REQ": ("APPROVED", "REJECTED", "DEFERRED"), "DF": ("ACCEPTED",), "W": ("WAIVED",)}.get(kind)
        if not want:
            err(w, f"{aid}: item {item!r} is not a REQ, DF or W id")
            continue
        if dec not in want:
            err(w, f"{aid}: decision {dec!r} for {item}: want one of {want}")
        if kind in ("DF", "W") and scope not in PHASES:
            err(w, f"{aid}: {item} needs a scope phase (the last gate it holds for): one of {PHASES}")
        if not who or who == "-" or "claude" in who.lower():
            err(w, f"{aid}: 'by' must name the person who decided (only Alex signs)")
        if not DATE.match(date):
            err(w, f"{aid}: date {date!r} is not YYYY-MM-DD")
        if dec in ("REJECTED", "DEFERRED", "ACCEPTED", "WAIVED") and (not reason or reason == "-"):
            err(w, f"{aid}: {dec} needs a reason")
        sha = bl.get(n, "0" * 40)
        committed = not set(sha) <= {"0"}
        if committed and CLAUDE_TRAILER.search(commit_message(sha)):
            err(w, f"{aid}: written in commit {sha[:7]}, which carries a Claude trailer: only Alex signs approvals")
            committed = False
        rows.append({"id": aid, "item": item, "decision": dec, "scope": scope, "reason": reason, "who": who,
                     "date": date, "sha": sha, "committed": committed, "_where": w})
    eff = {}
    for r in rows:
        if r["committed"]:
            eff[r["item"]] = r                     # a later line supersedes an earlier one
    return rows, eff


def parse_baseline():
    """README.md's design baseline: lines '- `path` <40-hex git blob>'."""
    path = os.path.join(VV, "README.md")
    out = {}
    if not os.path.exists(path):
        return out
    text = open(path, encoding="utf-8").read()
    m = re.search(r"^## Design baseline\s*$(.*?)(?=^## |\Z)", text, re.M | re.S)
    if m:
        for p, h in re.findall(r"^- `(hardware/\S+)` ([0-9a-f]{40})\s*$", m.group(1), re.M):
            out[p] = h
    return out


# ---- load and validate ------------------------------------------------------------
REQ_FIELDS = ["Level", "Board", "Statement", "Rationale", "Source", "Method", "Verified by", "Phase", "Status"]
DF_COLS = ["id", "board", "item", "function", "failure_mode", "effect", "severity", "cause", "prevention",
           "detection", "detects", "occurrence", "detection_rating", "action_priority", "action", "status"]
VER_COLS = ["id", "title", "method", "board", "procedure", "pass_limit", "covers", "last_result", "evidence",
            "commit", "date", "phase", "valid_days", "instrument", "accuracy", "needs"]
VP_COLS = ["num", "title", "maps_to", "cut_reason", "status_file", "status_now", "evidence"]
DEF_FIELDS = ["Found", "Symptom", "Root cause", "Why missed", "Check added or changed", "Updated", "Status"]
UNITS = {"mv": ("V", 1e-3), "v": ("V", 1.0), "ua": ("A", 1e-6), "ma": ("A", 1e-3), "a": ("A", 1.0),
         "c": ("C", 1.0), "ohm": ("ohm", 1.0), "kohm": ("ohm", 1e3), "mm": ("mm", 1.0)}


def in_board(path, board):
    return any(path.startswith(r) for r in BOARD_ROOTS[board])


def load():
    reqs = parse_blocks("requirements.md", RE["REQ"], REQ_FIELDS)
    for r in reqs.values():
        f, w = r["f"], r["_where"]
        if f.get("Level") and f["Level"] not in LEVELS:
            err(w, f"{r['id']}: Level {f['Level']!r} not in {LEVELS}")
        if f.get("Board") and f["Board"] not in BOARD_VALUES:
            err(w, f"{r['id']}: Board {f['Board']!r} not in {BOARD_VALUES}")
        meth = set(re.split(r"[,\s/]+", f.get("Method", ""))) - {""}
        if not meth or not meth <= METHODS:
            err(w, f"{r['id']}: Method {f.get('Method')!r}: use T, A, I, D")
        if f.get("Phase") and f["Phase"] not in PHASES:
            err(w, f"{r['id']}: Phase {f['Phase']!r} not in {PHASES}")
        if f.get("Status") and f["Status"] not in REQ_STATUS:
            err(w, f"{r['id']}: Status {f['Status']!r} not in {REQ_STATUS}")
        vb = f.get("Verified by", "")
        r["vers"] = VER_ID.findall(vb)
        r["children"] = [c for c in REQ_ID.findall(vb) if c.startswith(r["id"] + ".")]
        if vb and vb.lower() != "none" and not r["vers"] and not r["children"]:
            err(w, f"{r['id']}: 'Verified by' names no VER id and no allocation (write 'none' if there is none yet)")
        if "." in r["id"]:
            parent, suffix = r["id"].rsplit(".", 1)
            if parent not in reqs:
                err(w, f"{r['id']}: allocation of {parent}, which does not exist")
            want = {"R": "ref_design", "P": "prod_board"}[suffix]
            if f.get("Board") != want:
                err(w, f"{r['id']}: a .{suffix} allocation is Board: {want}")
    for r in reqs.values():
        r["alloc"] = {reqs[c]["f"].get("Board"): c for c in reqs if c.startswith(r["id"] + ".")}
        for c in r["children"]:
            if c not in reqs:
                err(r["_where"], f"{r['id']}: allocated to {c}, which does not exist")

    vers = {}
    for v in read_csv("verification.csv", VER_COLS):
        w = v["_where"]
        if not re.fullmatch(RE["VER"], v["id"]):
            err(w, f"malformed ID {v['id']!r}")
            continue
        if v["id"] in vers:
            err(w, f"duplicate ID {v['id']}")
            continue
        meth = set(v["method"].split("/")) - {""}
        if not meth or not meth <= METHODS:
            err(w, f"{v['id']}: method {v['method']!r}: use T, A, I, D or e.g. I/D")
        if v["board"] not in BOARD_VALUES:
            err(w, f"{v['id']}: board {v['board']!r}")
        if v["phase"] not in PHASES:
            err(w, f"{v['id']}: phase {v['phase']!r}")
        if v["last_result"] not in VER_RESULT:
            err(w, f"{v['id']}: last_result {v['last_result']!r} not in {VER_RESULT}")
        for k in ("title", "procedure", "pass_limit"):
            if not v[k]:
                err(w, f"{v['id']}: {k} is empty")
        if v["last_result"] in ("PASS", "FAIL"):
            for k in ("evidence", "commit", "date", "covers"):
                if not v[k]:
                    err(w, f"{v['id']}: {v['last_result']} needs {k}")
            if v["date"] and not DATE.match(v["date"]):
                err(w, f"{v['id']}: date {v['date']!r} is not YYYY-MM-DD")
            if v["commit"] and not resolve(v["commit"]):
                err(w, f"{v['id']}: commit {v['commit']!r} is not in this repository")
        elif v["commit"] or v["date"]:
            err(w, f"{v['id']}: {v['last_result']} must not carry a commit or date")
        if v["last_result"] == "CUT" and not v["evidence"]:
            err(w, f"{v['id']}: CUT needs its reason in the evidence column")
        if v["valid_days"] and not re.fullmatch(r"\d+", v["valid_days"]):
            err(w, f"{v['id']}: valid_days {v['valid_days']!r} is not a whole number of days")
        if v["needs"] and not re.search(r"\b\d+d\b", v["needs"]):
            warn(w, f"{v['id']}: 'needs' has no lead time token like '10d'")
        if v["needs"] and v["last_result"] == "PASS":
            warn(w, f"{v['id']}: PASS but 'needs' still lists {v['needs'][:40]!r}: clear it")
        v["cov"] = parse_covers(v["covers"])
        v["release"] = [s for k, s, _ in v["cov"] if k == "release"]
        for kind, spec, tok in v["cov"]:
            if kind == "glob":
                g, rx = spec
                files = [t for t in TRACKED if g.match(t)]
                if not files:
                    err(w, f"{v['id']}: covers {tok!r} matches no tracked file")
                elif rx is not None and not any(
                        rx.search(ln) for f_ in files if os.path.exists(os.path.join(ROOT, f_))
                        for ln in open(os.path.join(ROOT, f_), encoding="utf-8", errors="replace")):
                    err(w, f"{v['id']}: covers {tok!r}: the regex matches no current line of its file")
            elif kind == "part":
                cur = design_parts(None) or {}
                missing = [r for r in spec if r not in cur]
                if missing:
                    err(w, f"{v['id']}: covers {tok!r}: {', '.join(missing)} not in design.py")
            elif kind == "release":
                if not re.fullmatch(RE["REL"], spec):
                    err(w, f"{v['id']}: covers {tok!r}: not a REL id")
        if v["board"] == "both":
            paths = [t for k, s, _ in v["cov"] if k == "glob" for t in TRACKED if s[0].match(t)]
            shared = any(p.startswith(SHARED_ROOTS) for p in paths)
            if not shared and not all(any(in_board(p, b) for p in paths) for b in BOARDS):
                err(w, f"{v['id']}: a 'both' check must cover each board's files (or a shared file under "
                       f"{' or '.join(SHARED_ROOTS)})")
        for tok in v["evidence"].split():
            if re.match(r"^hardware/\S+\.\w+$", tok) and not os.path.exists(os.path.join(ROOT, tok)):
                err(w, f"{v['id']}: evidence file {tok} does not exist")
        tur_check(v)
        vers[v["id"]] = v

    dfs = {}
    for d in read_csv("dfmea.csv", DF_COLS):
        w = d["_where"]
        if not re.fullmatch(RE["DF"], d["id"]):
            err(w, f"malformed ID {d['id']!r}")
            continue
        if d["id"] in dfs:
            err(w, f"duplicate ID {d['id']}")
            continue
        if d["board"] not in BOARD_VALUES:
            err(w, f"{d['id']}: board {d['board']!r}")
        try:
            s, o, dt = int(d["severity"]), int(d["occurrence"]), int(d["detection_rating"])
            if not all(1 <= x <= 10 for x in (s, o, dt)):
                raise ValueError
        except ValueError:
            err(w, f"{d['id']}: severity, occurrence and detection_rating must be integers 1-10")
            continue
        d["S"], d["O"], d["D"] = s, o, dt
        want = action_priority(s, o, dt)
        if d["action_priority"] not in ("H", "M", "L"):
            err(w, f"{d['id']}: action_priority {d['action_priority']!r}")
        elif d["action_priority"] != want:
            err(w, f"{d['id']}: action_priority {d['action_priority']} but S{s} O{o} D{dt} gives {want} (AIAG-VDA table)")
        if d["status"] not in DF_STATUS:
            err(w, f"{d['id']}: status {d['status']!r} not in {DF_STATUS}")
        for k in ("item", "function", "failure_mode", "effect", "cause"):
            if not d[k]:
                err(w, f"{d['id']}: {k} is empty")
        d["vers"] = VER_ID.findall(d["detection"])
        d["det"] = d["detects"].split()
        for t in d["det"]:
            if not VER_ID.fullmatch(t):
                err(w, f"{d['id']}: detects token {t!r} is not a VER id")
            elif t not in d["vers"]:
                err(w, f"{d['id']}: detects {t}, which the detection text does not describe")
        dfs[d["id"]] = d

    # DFMEA against the checks (O-2, O-26)
    for d in dfs.values():
        w, live = d["_where"], [t for t in d["det"] if t in vers and vers[t]["last_result"] != "CUT"]
        if not live and d["D"] != 10:
            err(w, f"{d['id']}: no live 'detects' check, so detection_rating must be 10 (it is {d['D']})")
        elif live and all(vers[t]["last_result"] == "NOT_BUILT" for t in live) and d["D"] < 8:
            err(w, f"{d['id']}: every 'detects' check is NOT_BUILT, so detection_rating must be 8 or more (it is {d['D']})")
        if d["status"] == "DONE" and not live:
            err(w, f"{d['id']}: DONE needs at least one 'detects' check")
        if d["O"] <= 2:
            built = any("T" in vers[t]["method"] and vers[t]["release"] and vers[t]["last_result"] == "PASS"
                        for t in live)
            excluded = d["O"] == 1 and "Excluded:" in d["prevention"]
            if not built and not excluded:
                err(w, f"{d['id']}: occurrence {d['O']} needs a passing T check on built hardware in 'detects', "
                       f"or occurrence 1 with 'Excluded: <why the cause cannot happen>' in prevention")

    plan_n = set()
    vps = {}
    for p in read_csv("vp_map.csv", VP_COLS):
        w = p["_where"]
        try:
            num = int(p["num"])
        except ValueError:
            err(w, f"num {p['num']!r} is not a number")
            continue
        if num in vps:
            err(w, f"duplicate method #{num}")
            continue
        if not 1 <= num <= VP_METHODS:
            err(w, f"#{num}: validation_plan.md (archived) has methods 1-{VP_METHODS}")
        plan_n.add(num)
        if p["status_now"] not in VP_STATUS:
            err(w, f"#{num}: status_now {p['status_now']!r} not in {VP_STATUS}")
        if p["status_now"] != p["status_file"] and not p["evidence"]:
            err(w, f"#{num}: status changed from {p['status_file']} to {p['status_now']} with no evidence")
        toks = p["maps_to"].split()
        p["cut"] = toks == ["CUT"]
        if p["cut"] and not p["cut_reason"]:
            err(w, f"#{num}: CUT needs a cut_reason")
        if not p["cut"]:
            for t in toks:
                if t.startswith("DOC:"):
                    if not os.path.exists(os.path.join(VV, t[4:])):
                        err(w, f"#{num}: {t} does not exist")
                elif not ANY_ID.fullmatch(t):
                    err(w, f"#{num}: maps_to token {t!r} is not an ID, DOC:<file> or CUT")
        p["ids"] = [t for t in toks if ANY_ID.fullmatch(t)]
        vps[num] = p
    missing = sorted(set(range(1, VP_METHODS + 1)) - plan_n)
    if missing:
        err("vp_map.csv", f"methods missing: {missing}")

    defs = parse_blocks("defects.md", RE["DEF"], DEF_FIELDS)
    for d in defs.values():
        st = d["f"].get("Status", "").split()[0] if d["f"].get("Status") else ""
        d["status"] = st
        d["board"] = d["f"].get("Board", "ref_design")
        if d["board"] not in BOARD_VALUES:
            err(d["_where"], f"{d['id']}: Board {d['board']!r} not in {BOARD_VALUES}")
        if st not in DEF_STATUS:
            err(d["_where"], f"{d['id']}: Status must start with one of {DEF_STATUS}")
        d["vers"] = VER_ID.findall(d["f"].get("Check added or changed", ""))
        if st == "CLOSED" and not d["vers"]:
            err(d["_where"], f"{d['id']}: CLOSED with no VER in 'Check added or changed' (no defect closes without a check)")

    crit, waivers = parse_gates()
    recs = parse_records()
    approvals, eff = parse_approvals()
    db = dict(reqs=reqs, dfs=dfs, vers=vers, vps=vps, defs=defs, crit=crit, waivers=waivers, recs=recs,
              approvals=approvals, eff=eff, baseline=parse_baseline())

    for v in vers.values():
        check_evidence(v, recs)
    for d in defs.values():
        if d["status"] == "CLOSED":
            for vid in d["vers"]:
                if vid in vers and (vers[vid]["last_result"] != "PASS" or staleness(vers[vid], db)):
                    err(d["_where"], f"{d['id']}: CLOSED but {vid} has not passed on fresh evidence")

    cross_check_approvals(db)
    check_reach(db)
    check_releases(db)

    # every ID referenced anywhere must be defined somewhere
    defined = (set(reqs) | set(dfs) | set(vers) | set(defs) | set(crit) | set(recs) | {w["id"] for w in waivers})
    texts = []
    for r in reqs.values():
        texts += [(r["_where"], v) for v in r["f"].values()]
    for d in dfs.values():
        texts += [(d["_where"], d[k]) for k in DF_COLS if k != "id"]
    for v in vers.values():
        texts += [(v["_where"], v[k]) for k in ("title", "procedure", "pass_limit", "evidence")]
    for p in vps.values():
        texts += [(p["_where"], p["maps_to"]), (p["_where"], p["evidence"])]
    for d in defs.values():
        texts += [(d["_where"], v) for v in d["f"].values()]
    for c in crit.values():
        texts += [(c["_where"], c["text"])]
        for t in c["needs"]:
            if not (ANY_ID.fullmatch(t) or re.fullmatch(r"PHASE:(" + "|".join(PHASES) + ")", t)
                    or t in ("DEFECTS:CLOSED", "DFMEA:COVERAGE", "ADVISORY")):
                err(c["_where"], f"{c['id']}: needs token {t!r} is not an ID, PHASE:<phase>, "
                                 "DEFECTS:CLOSED, DFMEA:COVERAGE or ADVISORY")
            texts.append((c["_where"], t))
    for w in waivers:
        texts.append((w["_where"], w["crit"] + " " + w["reason"]))
    for rr in recs.values():
        texts.append((rr["_where"], rr["text"]))
    for a in approvals:
        texts.append((a["_where"], a["item"]))
    seen_bad = set()
    for where, t in texts:
        for i in ANY_ID.findall(t):
            if i not in defined and (where, i) not in seen_bad:
                seen_bad.add((where, i))
                err(where, f"{i} is referenced but not defined")
    for w in waivers:
        if w["crit"] not in crit:
            err(w["_where"], f"{w['id']}: waives {w['crit']!r}, which is not a criterion")
    return db


def tur_check(v):
    """F-38: warn when a pass band is narrower than 4 x the instrument's stated accuracy."""
    if not v["accuracy"]:
        return
    a = re.search(r"\+-\s*([\d.]+)\s*(mV|V|uA|mA|A|C|ohm|kohm|mm)\b", v["accuracy"])
    b = re.search(r"([\d.]+)\s*-\s*([\d.]+)\s*(mV|V|uA|mA|A|C|ohm|kohm|mm)\b", v["pass_limit"])
    if not a or not b:
        return
    ua, ub = UNITS[a.group(2).lower()], UNITS[b.group(3).lower()]
    if ua[0] != ub[0]:
        return
    acc = float(a.group(1)) * ua[1]
    half = (float(b.group(2)) - float(b.group(1))) * ub[1] / 2
    if half > 0 and half < 4 * acc:
        warn(v["_where"], f"{v['id']}: pass band +-{half:g} {ub[0]} is {half / acc:.1f} x the instrument's "
                          f"+-{acc:g} {ua[0]} (want 4 x): use a better instrument or a reference")


def check_evidence(v, recs):
    """O-4, F-3, O-6: a PASS or FAIL rests on a saved output whose header matches the row."""
    if v["last_result"] not in ("PASS", "FAIL"):
        return
    w, ev = v["_where"], v["evidence"]
    if re.search(r"\bgit (show|log)\b", ev):
        err(w, f"{v['id']}: a commit message is not evidence: save the check's output under {EVIDENCE_DIR}")
        return
    first = ev.split()[0] if ev.split() else ""
    if not first.startswith("hardware/") or not os.path.isfile(os.path.join(ROOT, first)):
        err(w, f"{v['id']}: evidence must start with a file in the repository")
        return
    if "A" not in v["method"] and not v["release"]:
        return
    if not first.startswith(EVIDENCE_DIR):
        err(w, f"{v['id']}: an analysis or bench result needs its saved output under {EVIDENCE_DIR}")
        return
    text = open(os.path.join(ROOT, first), encoding="utf-8", errors="replace").read()
    m = re.search(r"^COMMIT: ([0-9a-f]{7,40})\b", text, re.M)
    if not m or not (m.group(1).startswith(v["commit"]) or v["commit"].startswith(m.group(1))):
        err(w, f"{v['id']}: {first} has no 'COMMIT: {v['commit']}' line")
    if not re.search(r"^TOOLS: \S", text, re.M):
        err(w, f"{v['id']}: {first} has no 'TOOLS:' line (tool versions)")
    res = re.findall(r"^RESULT: (PASS|FAIL)\b", text, re.M)
    if not res or res[-1] != v["last_result"]:
        err(w, f"{v['id']}: the row says {v['last_result']} but {first} ends "
               f"{'RESULT: ' + res[-1] if res else 'with no RESULT line'}")
    for r in v["release"]:
        if not re.search(r"^RELEASE: " + re.escape(r) + r"\b", text, re.M):
            err(w, f"{v['id']}: {first} has no 'RELEASE: {r}' line")
        if not re.search(r"^UNITS: \S", text, re.M):
            err(w, f"{v['id']}: {first} has no 'UNITS:' line (the serials it ran on)")
        rr = recs.get(r)
        if rr is None:
            err(w, f"{v['id']}: covers {r}, which has no release record")
        elif rr["fab_sha"] and rr["fab_sha"] not in text:
            err(w, f"{v['id']}: {first} does not name {r}'s fab zip sha256")


def eff_req_status(db, rid):
    a = db["eff"].get(rid)
    return a["decision"] if a else "PROPOSED"


def eff_df_accept(db, did):
    a = db["eff"].get(did)
    return a if a and a["decision"] == "ACCEPTED" else None


def cross_check_approvals(db):
    """O-45: statuses that need Alex must match a line he committed in APPROVALS.md."""
    for r in db["reqs"].values():
        st, eff = r["f"].get("Status"), eff_req_status(db, r["id"])
        if st != "PROPOSED" and st != eff:
            err(r["_where"], f"{r['id']}: Status {st} but releases/APPROVALS.md "
                             f"{'says ' + eff if eff != 'PROPOSED' else 'has no committed line for it'}")
        elif st == "PROPOSED" and eff != "PROPOSED":
            warn(r["_where"], f"{r['id']}: APPROVALS.md says {eff}: update its Status")
        if eff == "APPROVED":
            a = db["eff"][r["id"]]
            rel = vv_rel("requirements.md")
            bl = blame(rel) if rel and rel in TRACKED else {}
            for n in r["lines"].get("Statement", []):
                sha = bl.get(n, "0" * 40)
                if set(sha) <= {"0"} or not is_ancestor(sha, a["sha"]):
                    err(r["_where"], f"{r['id']}: its statement changed after Alex approved it ({a['id']}): "
                                     "he approves it again")
                    break
    for d in db["dfs"].values():
        acc = eff_df_accept(db, d["id"])
        if d["status"] == "ACCEPTED" and not acc:
            err(d["_where"], f"{d['id']}: ACCEPTED but releases/APPROVALS.md has no committed line for it")
        elif d["status"] != "ACCEPTED" and acc:
            warn(d["_where"], f"{d['id']}: APPROVALS.md accepts it for {acc['scope']}: set status ACCEPTED")
    for a in db["approvals"]:
        if not a["committed"]:
            warn(a["_where"], f"{a['id']} ({a['item']} {a['decision']}) is not committed by Alex yet: it does not count")


def check_reach(db):
    """O-1: a check must be reached by a gate, a requirement or an H row no later than its phase."""
    vers, crit = db["vers"], db["crit"]
    reached = set()
    for r in db["reqs"].values():
        if eff_req_status(db, r["id"]) not in ("REJECTED", "DEFERRED"):
            reached |= set(r["vers"])
    for d in db["dfs"].values():
        if d["action_priority"] == "H":
            reached |= set(d["det"])
    for c in crit.values():
        if c["advisory"]:
            continue
        for vid, v in vers.items():
            if c["board"] != "both" and v["board"] not in (c["board"], "both"):
                continue
            if PHASES.index(c["phase"]) > PHASES.index(v["phase"]):
                continue
            if vid in c["needs"] or any(t.startswith("PHASE:") and phase_le(v["phase"], t[6:]) for t in c["needs"]):
                reached.add(vid)
    for vid, v in vers.items():
        if v["last_result"] != "CUT" and vid not in reached:
            err(v["_where"], f"{vid}: phase {v['phase']}, but no gate criterion at or before {v['phase']}, "
                             "requirement or H-priority risk reaches it: that gate can pass without it")


def check_releases(db):
    """O-35: a release record whose BOM differs from another's must carry a new BOARD_REV."""
    rels = [r for r in db["recs"].values() if r["kind"] == "REL"]
    for i, a in enumerate(rels):
        for b in rels[i + 1:]:
            if a["board"] == b["board"] and a["bom_sha"] and b["bom_sha"] and a["board_rev"] and b["board_rev"]:
                if a["bom_sha"] != b["bom_sha"] and a["board_rev"] == b["board_rev"]:
                    err(b["_where"], f"{b['id']}: its BOM differs from {a['id']}'s but BOARD_REV is still "
                                     f"{b['board_rev']}: bump the strap (R48-R51) with the BOM change")
        if a["commit"] and not resolve(a["commit"]):
            err(a["_where"], f"{a['id']}: commit {a['commit']!r} is not in this repository")


# ---- freshness ---------------------------------------------------------------------
def staleness(v, db, dirty=None):
    """None if fresh, else a short reason."""
    if v["last_result"] not in ("PASS", "FAIL"):
        return None
    c = resolve(v["commit"])
    if not c:
        return "commit unknown"
    if not is_ancestor(c):
        return f"{v['commit']} is not an ancestor of HEAD"
    hits = []
    changed = changed_since(c)
    for kind, spec, tok in v["cov"]:
        if kind == "glob":
            g, rx = spec
            for f in sorted(changed):
                if g.match(f) and (rx is None or any(rx.search(ln) for ln in changed_lines(c, f))):
                    hits.append(f)
            if dirty:
                hits += [f + " (uncommitted)" for f in sorted(dirty) if g.match(f)]
        elif kind == "part":
            old, new = design_parts(c), design_parts(None)
            if old is None or new is None:
                hits.append("design.py could not be read at " + v["commit"])
                continue
            moved = [r for r in spec if part_key(old.get(r)) != part_key(new.get(r))]
            if moved:
                hits.append("design.py parts " + ",".join(moved[:6]))
        elif kind == "release":
            for ch in db["recs"].values():
                if ch["kind"] == "CHG" and v["id"] in ch["reruns"] and (
                        not ch["commits"] or any(not (resolve(x) and is_ancestor(resolve(x), c)) for x in ch["commits"])):
                    hits.append(f"{ch['id']} asks for a re-run")
    if v["valid_days"] and v["date"]:
        age = (TODAY - datetime.date.fromisoformat(v["date"])).days
        if age > int(v["valid_days"]):
            hits.append(f"expired: {age} days old, valid {v['valid_days']}")
    if hits:
        uniq = sorted(set(hits))
        return "changed: " + ", ".join(uniq[:4]) + (f" (+{len(uniq) - 4})" if len(uniq) > 4 else "")
    return None


def for_board(item_board, board):
    return item_board in (board, "both")


def phase_le(a, b):
    return PHASES.index(a) <= PHASES.index(b)


def req_scope(db, board):
    """Requirements a board's gate evaluates (O-7)."""
    reqs = db["reqs"]
    if board == "prod_board":
        return {k: r for k, r in reqs.items() if r["f"].get("Board") == "prod_board"}
    return {k: r for k, r in reqs.items() if r["f"].get("Board") in ("ref_design", "both")}


def df_scope(db, board):
    if board == "prod_board":
        return [d for d in db["dfs"].values() if d["board"] == "prod_board"]
    return [d for d in db["dfs"].values() if d["board"] in ("ref_design", "both")]


def coverage(db):
    """O-55: bom=True refs and every contact that no DFMEA row names (ranges like C10-C22 count)."""
    parts = design_parts(None) or {}
    want = [r for r, p in parts.items() if not r.startswith("#") and (p.get("bom") or r.startswith("SW"))]
    named = set()
    rx = re.compile(r"\b([A-Z]{1,3})(\d{1,3})(?:\s*-\s*([A-Z]{1,3})?(\d{1,3}))?\b")
    for d in db["dfs"].values():
        text = " ".join(d[k] for k in ("item", "function", "failure_mode", "effect", "cause", "prevention", "action"))
        for m in rx.finditer(text):
            a, n1, b, n2 = m.groups()
            if n2 and b in (None, a) and int(n1) <= int(n2):
                named |= {f"{a}{i}" for i in range(int(n1), int(n2) + 1)}
            else:
                named.add(a + n1)
                if n2 and b:
                    named.add(b + n2)
    return want, [r for r in want if r not in named]


# ---- report --------------------------------------------------------------------------
def show(label, ids, verbose, limit=40):
    ids = list(ids)
    print(f"  {label}: {len(ids)}")
    if not ids:
        return
    shown = ids if verbose else ids[:limit]
    line = "    "
    for i in shown:
        if len(line) + len(i) + 1 > 98:
            print(line.rstrip())
            line = "    "
        line += i + " "
    if line.strip():
        print(line.rstrip())
    if len(ids) > len(shown):
        print(f"    ... and {len(ids) - len(shown)} more (--verbose)")


def baseline_diff(db, rels):
    out = []
    for path, blob in db["baseline"].items():
        if rels and not any(path.endswith(r) for r in rels):
            continue
        rc, now = git("rev-parse", f"HEAD:{path}")
        if rc != 0 or now.strip() != blob:
            out.append(path)
    return out


def report(db, verbose):
    reqs, dfs, vers, vps, defs = db["reqs"], db["dfs"], db["vers"], db["vps"], db["defs"]
    print(f"trace report at {HEAD[:7]} ({ROOT})")
    print(f"  requirements {len(reqs)} " + str(dict(Counter(eff_req_status(db, r) for r in reqs))))
    print(f"  DFMEA rows {len(dfs)}, action priority " + str(dict(sorted(Counter(d['action_priority'] for d in dfs.values()).items()))))
    print(f"  checks {len(vers)} " + str(dict(Counter(v['last_result'] for v in vers.values()))))
    cut = [p for p in vps.values() if p["cut"]]
    print(f"  validation-plan methods {len(vps)}: {len(vps) - len(cut)} mapped, {len(cut)} cut (archived plan)")
    print(f"  defects {len(defs)} " + str(dict(Counter(d['status'] for d in defs.values()))))
    print(f"  approvals by Alex: {sum(1 for a in db['approvals'] if a['committed'])} committed, "
          f"{sum(1 for a in db['approvals'] if not a['committed'])} pending")
    bd = baseline_diff(db, None)
    print("  design baseline: " + (("differs from HEAD: " + " ".join(bd)) if bd else "matches HEAD")
          if db["baseline"] else "  design baseline: none in README.md")
    print()
    print("requirements")
    for b in BOARDS:
        scope = req_scope(db, b)
        show(f"in scope for {b} with no check for it",
             [r for r, q in scope.items() if not q["alloc"].get(b) and not any(
                 for_board(vers[v]["board"], b) for v in q["vers"] if v in vers)], verbose)
    show("product requirements not yet allocated to prod_board (Alex writes the .P ones)",
         [r for r, q in reqs.items() if q["f"].get("Board") == "both" and "." not in r], verbose, limit=12)
    show("no listed check has passed", [r for r in reqs if reqs[r]["vers"] and not any(
        vers.get(v, {}).get("last_result") == "PASS" for v in reqs[r]["vers"])], verbose)
    show("statement has no number (INCOSE: state a value and tolerance)",
         [r for r in reqs if not re.search(r"\d", reqs[r]["f"].get("Statement", ""))], verbose)
    print("DFMEA")
    hi = [d for d in dfs.values() if d["action_priority"] == "H"]
    show("H rows", [d["id"] for d in hi], verbose)
    show("H rows with no 'detects' check", [d["id"] for d in hi if not d["det"]], verbose)
    show("H rows whose 'detects' checks have not all passed on fresh evidence",
         [d["id"] for d in hi if d["det"] and not all(
             vers.get(v, {}).get("last_result") == "PASS" and not staleness(vers[v], db) for v in d["det"])
          and not eff_df_accept(db, d["id"])], verbose)
    show("severity 9-10 rows", [d["id"] for d in dfs.values() if d["S"] >= 9], verbose)
    want, miss = coverage(db)
    show(f"design.py parts and contacts named in no row (of {len(want)})", miss, verbose)
    print("checks")
    for res in ("FAIL", "NOT_RUN", "NOT_BUILT", "CUT"):
        show(res, [v for v in vers if vers[v]["last_result"] == res], verbose)
    stale = [(v, staleness(vers[v], db)) for v in vers]
    stale = [(v, s) for v, s in stale if s]
    print(f"  stale evidence: {len(stale)}")
    for v, s in stale:
        print(f"    {v} ({vers[v]['last_result']}, {vers[v]['commit']}): {s}")
    print("validation plan (archived; vp_map.csv is a one-time mapping)")
    show("unmapped methods", [f"#{n}" for n, p in sorted(vps.items()) if not p["cut"] and not p["ids"]
                              and not p["maps_to"].startswith("DOC:")], verbose)
    print("defects")
    show("OPEN", [d for d in defs if defs[d]["status"] == "OPEN"], verbose)
    show("FIXED but no check yet passes (cannot close)", [d for d in defs if defs[d]["status"] == "FIXED"], verbose)
    if WARNINGS:
        print(f"warnings: {len(WARNINGS)}")
        for wn in WARNINGS if verbose else WARNINGS[:15]:
            print("  " + wn)
        if not verbose and len(WARNINGS) > 15:
            print(f"  ... and {len(WARNINGS) - 15} more (--verbose)")


def shopping(db, phase, board):
    """F-50: what the next gate's unrun checks still need, longest lead time first."""
    vers, crit = db["vers"], db["crit"]
    named = {t for c in crit.values() if c["phase"] == phase for t in c["needs"]}
    rows = []
    for vid, v in vers.items():
        if not v["needs"] or v["last_result"] not in ("NOT_RUN", "NOT_BUILT") or not for_board(v["board"], board):
            continue
        if phase_le(v["phase"], phase) or vid in named:
            m = re.findall(r"\b(\d+)d\b", v["needs"])
            rows.append((-max(int(x) for x in m) if m else 1, vid, v["needs"]))
    print(f"shopping for the {phase} gate ({board}): {len(rows)} checks still need something")
    for days, vid, need in sorted(rows):
        print(f"  {'?' if days == 1 else -days:>4}d  {vid}  {need}")
    if any(d == 1 for d, _, _ in rows):
        print("  (?: no lead time given: add '<N>d' to the 'needs' entry)")


# ---- gates ---------------------------------------------------------------------------
def gate(db, phase, board, verbose):
    reqs, dfs, vers, defs, crit = db["reqs"], db["dfs"], db["vers"], db["defs"], db["crit"]
    dirty = dirty_files()
    waived = {}
    for w in db["waivers"]:
        a = db["eff"].get(w["id"])
        c = crit.get(w["crit"])
        if a and a["decision"] == "WAIVED" and c and phase_le(c["phase"], a["scope"]):
            waived[w["crit"]] = (w, a)

    def ver_ok(vid):
        v = vers.get(vid)
        if v is None:
            return False, f"{vid} undefined"
        if v["last_result"] != "PASS":
            return False, f"{vid} {v['last_result']}"
        s = staleness(v, db, dirty)
        return (False, f"{vid} stale ({s})") if s else (True, "")

    def df_ok(d):
        acc = eff_df_accept(db, d["id"])
        if acc and phase_le(phase, acc["scope"]):
            return True, ""
        if acc:
            return False, f"{d['id']} accepted only up to {acc['scope']} ({acc['id']})"
        if d["status"] != "DONE":
            return False, f"{d['id']} {d['status']}"
        bad = [t for t in d["det"] if not (t in vers and phase_le(vers[t]["phase"], phase) and ver_ok(t)[0])]
        return (not bad), (f"{d['id']} DONE but detects not passing by {phase}: {' '.join(bad)}" if bad else "")

    def need_ok(tok):
        if tok == "ADVISORY":
            return None, ""
        if tok.startswith("PHASE:"):
            p = tok.split(":")[1]
            bad = [v for v in vers if for_board(vers[v]["board"], board) and phase_le(vers[v]["phase"], p)
                   and vers[v]["last_result"] != "CUT" and not ver_ok(v)[0]]
            return (not bad), (f"{len(bad)} checks at or before {p} not passing: " + " ".join(bad[:8])
                               + (" ..." if len(bad) > 8 else "")) if bad else ""
        if tok == "DEFECTS:CLOSED":
            bad = [d for d in defs if for_board(defs[d]["board"], board) and defs[d]["status"] != "CLOSED"]
            return (not bad), (f"not CLOSED: {' '.join(bad[:8])}" + (" ..." if len(bad) > 8 else "")) if bad else ""
        if tok == "DFMEA:COVERAGE":
            if board != "ref_design":
                return None, "DFMEA:COVERAGE reads ref_design's design.py"
            _, miss = coverage(db)
            return (not miss), (f"{len(miss)} parts in no DFMEA row: " + " ".join(miss[:10])) if miss else ""
        if tok.startswith("VER-"):
            if tok in vers and not for_board(vers[tok]["board"], board):
                return None, f"{tok} is a {vers[tok]['board']} check"
            if tok in vers and vers[tok]["last_result"] == "CUT":
                return False, f"{tok} CUT ({vers[tok]['evidence'][:60]}): waive the criterion or replace the check"
            return ver_ok(tok)
        if tok.startswith("REQ-"):
            r = reqs.get(tok)
            if r is None:
                return False, f"{tok} undefined"
            if eff_req_status(db, tok) != "APPROVED":
                return False, f"{tok} {eff_req_status(db, tok)}"
            target = reqs.get(r["alloc"].get(board), r)
            bad = [v for v in target["vers"] if v in vers and for_board(vers[v]["board"], board)
                   and phase_le(vers[v]["phase"], phase) and not ver_ok(v)[0]]
            return (not bad), (f"{tok} checks not passing: {' '.join(bad)}" if bad else "")
        if tok.startswith("DF-"):
            d = dfs.get(tok)
            return (False, f"{tok} undefined") if d is None else df_ok(d)
        if tok.startswith("DEF-"):
            d = defs.get(tok)
            if d is None:
                return False, f"{tok} undefined"
            return d["status"] == "CLOSED", (f"{tok} {d['status']}" if d["status"] != "CLOSED" else "")
        return False, f"{tok}: cannot evaluate"

    results = []                                    # (id, state PASS FAIL N/A INFO, reasons)
    prefix = GATE_PREFIX[phase]
    for cid, c in sorted(crit.items()):
        if c["phase"] != phase:
            continue
        if not for_board(c["board"], board):
            results.append((cid, "N/A", [f"a {c['board']} criterion"]))
            continue
        checked = [need_ok(t) for t in c["needs"]]
        reasons = [r for ok, r in checked if ok is False]
        if all(ok is None for ok, _ in checked):
            reasons = [f"nothing here applies to {board}: give it its own check ("
                       + "; ".join(r for _, r in checked if r) + ")"]
        if c["advisory"]:
            results.append((cid, "INFO", reasons or ["met"]))
        elif reasons and cid in waived:
            w, a = waived[cid]
            results.append((cid, "PASS", [f"waived by {a['who']} {a['date']} ({w['id']}, {a['id']})"]))
        else:
            results.append((cid, "FAIL" if reasons else "PASS", reasons))

    mine = req_scope(db, board)
    prop = [k for k in mine if eff_req_status(db, k) == "PROPOSED"]
    results.append(("R1 requirements approved", "FAIL" if prop else "PASS",
                     [f"{len(prop)} PROPOSED: " + " ".join(prop[:10]) + (" ..." if len(prop) > 10 else "")] if prop else []))
    nover, notpass = [], []
    for k, r in mine.items():
        if eff_req_status(db, k) in ("REJECTED", "DEFERRED") or r["alloc"].get(board):
            continue                                   # a split requirement is checked through its child
        vs = [v for v in r["vers"] if v in vers and for_board(vers[v]["board"], board)]
        if not vs:
            nover.append(k)
            continue
        if any(phase_le(vers[v]["phase"], phase) and not ver_ok(v)[0] for v in vs):
            notpass.append(k)
    rs = []
    if nover:
        rs.append(f"{len(nover)} with no check for {board}: " + " ".join(nover[:10]) + (" ..." if len(nover) > 10 else ""))
    if notpass:
        rs.append(f"{len(notpass)} with a check at or before {phase} not passing: " + " ".join(notpass[:10])
                  + (" ..." if len(notpass) > 10 else ""))
    results.append(("R2 requirements verified", "FAIL" if rs else "PASS", rs))
    bad = [d["id"] for d in df_scope(db, board) if d["action_priority"] == "H" and not df_ok(d)[0]]
    results.append(("R3 high-priority risks covered", "FAIL" if bad else "PASS",
                    [f"{len(bad)} H rows neither ACCEPTED for {phase} nor DONE with every 'detects' check passing: "
                     + " ".join(bad[:12]) + (" ..." if len(bad) > 12 else "")] if bad else []))
    mydefs = [d for d in defs if for_board(defs[d]["board"], board)]
    if phase in ("ORDER", "CASE", "EVT"):
        bad = [d for d in mydefs if defs[d]["status"] == "OPEN"]
        results.append(("R4 no open defects", "FAIL" if bad else "PASS", [f"OPEN: {' '.join(bad)}"] if bad else []))
    else:
        bad = [d for d in mydefs if defs[d]["status"] != "CLOSED"]
        results.append(("R4 every defect closed", "FAIL" if bad else "PASS", [f"not CLOSED: {' '.join(bad)}"] if bad else []))
    dirty_hits = sorted({f for v in vers.values() if v["last_result"] == "PASS"
                         for k, s, _ in v["cov"] if k == "glob" for f in dirty if s[0].match(f)})
    results.append(("R5 no uncommitted change under passed evidence", "FAIL" if dirty_hits else "PASS",
                    [" ".join(dirty_hits[:6])] if dirty_hits else []))
    if board == "ref_design":
        rels = [r for r in db["recs"].values() if r["kind"] == "REL" and r["board"] == "ref_design"
                and r["commit"] and resolve(r["commit"]) and is_ancestor(resolve(r["commit"]))]
        unrec = []
        if rels:
            newest = min(rels, key=lambda r: int(git("rev-list", "--count", f"{resolve(r['commit'])}..HEAD")[1] or 0))
            shas = git("log", "--format=%H", f"{resolve(newest['commit'])}..HEAD", "--", *DESIGN_PATHS)[1].split()
            named = [x for ch in db["recs"].values() if ch["kind"] == "CHG" for x in ch["commits"]]
            unrec = [s for s in shas if not any(s.startswith(x) for x in named)]
        results.append(("R6 every design change since the last release has a change record",
                         "FAIL" if unrec else "PASS",
                         [f"unrecorded design change {' '.join(s[:7] for s in unrec[:8])}"] if unrec else
                         ([] if rels else ["no release record yet: nothing to compare against"])))
        which = None if phase not in BASELINE_FOR else BASELINE_FOR[phase]
        bd = baseline_diff(db, which) if db["baseline"] else ["README.md has no design baseline"]
        results.append(("R7 the register describes this design", "FAIL" if bd else "PASS",
                        ["gate evaluated against a design it does not describe: " + " ".join(bd)] if bd else []))

    print(f"gate {phase} for {board} at {HEAD[:7]}")
    for cid, state, reasons in results:
        print(f"  {state:4}  {cid}" + (f"  {crit[cid]['text'][:70]}" if cid in crit else ""))
        for r in reasons if (verbose or state in ("FAIL", "INFO")) else []:
            print(f"          {r}")
    counted = [s for _, s, _ in results if s in ("PASS", "FAIL")]
    n_bad = counted.count("FAIL")
    print(f"RESULT: {'PASS' if not n_bad else f'FAIL ({n_bad} of {len(counted)} not met)'}"
          + (f"; {sum(1 for _, s, _ in results if s == 'N/A')} N/A, "
             f"{sum(1 for _, s, _ in results if s == 'INFO')} advisory" if len(counted) < len(results) else ""))
    return n_bad == 0


def main():
    global VV
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--gate", choices=sorted(GATE_ARG), help="evaluate a phase gate (exit 1 unless met)")
    ap.add_argument("--board", choices=BOARDS, default="ref_design")
    ap.add_argument("--shopping", action="store_true", help="list what unrun checks still need, by lead time")
    ap.add_argument("--verbose", action="store_true")
    ap.add_argument("--vv", help=argparse.SUPPRESS)        # another copy of hardware/vv (seed_faults.py)
    a = ap.parse_args()
    if a.vv:
        VV = os.path.abspath(a.vv)
    db = load()
    if ERRORS:
        print(f"STRUCTURE: {len(ERRORS)} error(s)")
        for e in ERRORS:
            print("  " + e)
        sys.exit(1)
    print("STRUCTURE: OK" + (f" ({len(WARNINGS)} warnings)" if WARNINGS else ""))
    if a.shopping:
        shopping(db, GATE_ARG[a.gate] if a.gate else "ORDER", a.board)
        sys.exit(0)
    if a.gate:
        sys.exit(0 if gate(db, GATE_ARG[a.gate], a.board, a.verbose) else 1)
    report(db, a.verbose)
    sys.exit(0)


if __name__ == "__main__":
    main()
