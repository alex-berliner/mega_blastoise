#!/usr/bin/env python3
"""The trace gate for hardware/vv: requirements, DFMEA, checks, defects and gates.

    python3 hardware/vv/tools/check_trace.py                     # structure + report
    python3 hardware/vv/tools/check_trace.py --gate order        # exit 1 unless the order gate is met
    python3 hardware/vv/tools/check_trace.py --gate evt --board prod_board
    python3 hardware/vv/tools/check_trace.py --verbose           # every ID in every list

Default mode exits 1 only on a structural error: a duplicate or malformed ID, an ID that
is referenced but defined nowhere, a malformed row or field, an action priority that does
not match the AIAG-VDA table, a check marked PASS or FAIL without a commit and date, a
'covers' pattern that matches no tracked file, a validation-plan row that disagrees with
docs/validation_plan.md, or a defect marked CLOSED without a check. Everything else is a
report: requirements with no check, high-priority risks with no detection, checks that
have not passed, evidence that has gone stale, validation-plan methods left unmapped.

--gate PHASE exits 1 unless every criterion of that gate in gates.md is met for the board
(default ref_design), plus these rules that hold at every gate:
  R1  no requirement for the board is still PROPOSED;
  R2  every requirement lists at least one check for the board, and every listed check
      whose phase is at or before the gate has PASSED on fresh evidence;
  R3  every H-priority DFMEA row is ACCEPTED (by name and date), or DONE (its action taken)
      with a detection check at or before the gate that has PASSED on fresh evidence;
  R4  no defect is OPEN (order, EVT); every defect is CLOSED (DVT, PVT);
  R5  no PASSED check covers a file with uncommitted changes.

Evidence is fresh when its commit is an ancestor of HEAD and no file matching the check's
'covers' patterns changed between that commit and HEAD. A pattern is a repo-relative glob
('*' stays inside one directory, '**' crosses them), optionally 'path::regex': then only a
changed line (added or removed) matching the regex makes the evidence stale.

Python 3 standard library only.
"""
import argparse
import csv
import os
import re
import subprocess
import sys
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
VV = os.path.dirname(HERE)

PHASES = ["ORDER", "EVT", "DVT", "PVT", "SALE"]
GATE_ARG = {"order": "ORDER", "evt": "EVT", "dvt": "DVT", "pvt": "PVT"}
GATE_PREFIX = {"ORDER": "G-ORD", "EVT": "G-EVT", "DVT": "G-DVT", "PVT": "G-PVT"}
BOARDS = ("ref_design", "prod_board")
BOARD_VALUES = ("ref_design", "prod_board", "both")
METHODS = set("TAID")
LEVELS = ("product", "subsystem", "board")
REQ_STATUS = ("PROPOSED", "APPROVED", "REJECTED", "DEFERRED")
VER_RESULT = ("PASS", "FAIL", "NOT_RUN", "NOT_BUILT")
DF_STATUS = ("OPEN", "DONE", "ACCEPTED")
DEF_STATUS = ("OPEN", "FIXED", "CLOSED")
VP_STATUS = ("yes", "partly", "no", "simulated")

RE = {
    "REQ": r"REQ-[A-Z]{2,4}-\d{2}",
    "DF": r"DF-\d{2,3}",
    "VER": r"VER-\d{2,3}",
    "DEF": r"DEF-\d{2,3}",
    "CHG": r"CHG-\d{2,3}",
    "REL": r"REL-[A-Z0-9]+(?:-[A-Z0-9]+)+",
    "G": r"G-(?:ORD|EVT|DVT|PVT)-\d{2}",
    "W": r"W-\d{2}",
}
ANY_ID = re.compile(r"(?<![\w-])(" + "|".join(RE[k] for k in ("REQ", "DF", "VER", "DEF", "CHG", "G", "W"))
                    + r"|REL-[A-Z0-9]+(?:-[A-Z0-9]+)+)(?![\w-])")
VER_ID = re.compile(r"(?<![\w-])VER-\d{2,3}(?![\w-])")
DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
ACCEPT = re.compile(r"^Accepted by \S+ \d{4}-\d{2}-\d{2}:")

ERRORS = []


def err(where, msg):
    ERRORS.append(f"{where}: {msg}")


# ---- git -------------------------------------------------------------------------
def git(*args):
    r = subprocess.run(["git", "-C", ROOT] + list(args), capture_output=True, text=True)
    return r.returncode, r.stdout


ROOT = subprocess.run(["git", "-C", VV, "rev-parse", "--show-toplevel"],
                      capture_output=True, text=True).stdout.strip()
if not ROOT:
    sys.exit("check_trace: not inside a git work tree")
TRACKED = set(git("ls-files", "--cached", "--others", "--exclude-standard")[1].split("\n")) - {""}
HEAD = git("rev-parse", "HEAD")[1].strip()


def dirty_files():
    out = git("status", "--porcelain", "--untracked-files=no")[1]
    return {line[3:].split(" -> ")[-1] for line in out.splitlines() if line.strip()}


_resolved, _diff, _lines = {}, {}, {}


def resolve(commit):
    if commit not in _resolved:
        rc, out = git("rev-parse", "--verify", "--quiet", commit + "^{commit}")
        _resolved[commit] = out.strip() if rc == 0 else None
    return _resolved[commit]


def is_ancestor(commit):
    return git("merge-base", "--is-ancestor", commit, "HEAD")[0] == 0


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


def parse_covers(text):
    """[(glob regex, line regex or None, original token)]"""
    out = []
    for tok in text.split():
        path, _, rx = tok.partition("::")
        out.append((glob_re(path), re.compile(rx) if rx else None, tok))
    return out


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
        rd = csv.reader(f)
        rows = list(rd)
    if not rows:
        err(name, "empty")
        return []
    head = rows[0]
    if head != columns:
        err(name, f"header is {head}, want {columns}")
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
                cur = blocks[bid] = {"id": bid, "title": m.group(2), "_where": f"{name}:{n}", "f": {}}
            continue
        if cur is not None:
            m = re.match(r"^- ([A-Za-z][A-Za-z ]*?): (.*)$", line)
            if m:
                cur["f"][m.group(1)] = m.group(2).strip()
            elif line.startswith("  ") and cur["f"]:
                last = list(cur["f"])[-1]
                cur["f"][last] += " " + line.strip()
    for b in blocks.values():
        for k in fields_required:
            if not b["f"].get(k):
                err(b["_where"], f"{b['id']}: field '{k}' missing or empty")
    return blocks


def parse_validation_plan():
    path = os.path.join(ROOT, "hardware/ref_design/docs/validation_plan.md")
    out = {}
    if not os.path.exists(path):
        err("validation_plan.md", "missing")
        return out
    for line in open(path, encoding="utf-8"):
        m = re.match(r"^(\d+)\. \*\*(.+?)\*\* \(", line)
        if not m or "*Done:*" not in line:
            continue
        done = line.split("*Done:*", 1)[1].strip()
        s = re.match(r"(yes|no|partly|simulated)\b", done)
        out[int(m.group(1))] = (m.group(2), s.group(1) if s else "?")
    return out


def parse_gates():
    path = os.path.join(VV, "gates.md")
    crit, waivers = {}, []
    if not os.path.exists(path):
        err("gates.md", "missing")
        return crit, waivers
    for n, line in enumerate(open(path, encoding="utf-8"), 1):
        cells = [c.strip() for c in line.strip().strip("|").split("|")] if line.startswith("|") else []
        if not cells:
            continue
        if re.match(r"G-", cells[0]):
            if not re.fullmatch(RE["G"], cells[0]):
                err(f"gates.md:{n}", f"malformed criterion ID {cells[0]!r}")
                continue
            if len(cells) < 3:
                err(f"gates.md:{n}", f"{cells[0]}: want | ID | criterion | needs |")
                continue
            if cells[0] in crit:
                err(f"gates.md:{n}", f"duplicate ID {cells[0]}")
                continue
            crit[cells[0]] = {"id": cells[0], "text": cells[1], "needs": cells[2].split(), "_where": f"gates.md:{n}"}
        elif re.match(r"W-", cells[0]):
            if not re.fullmatch(RE["W"], cells[0]) or len(cells) < 5:
                err(f"gates.md:{n}", f"waiver row wants | W-NN | G-... | reason | approver | date |")
                continue
            waivers.append({"id": cells[0], "crit": cells[1], "reason": cells[2], "who": cells[3],
                            "date": cells[4], "_where": f"gates.md:{n}"})
    return crit, waivers


def parse_records():
    """releases/REL-*.md and releases/CHG-*.md: '# ID' or '# ID: title' as the first heading."""
    d = os.path.join(VV, "releases")
    recs = {}
    if not os.path.isdir(d):
        return recs
    for fn in sorted(os.listdir(d)):
        if not fn.endswith(".md") or "TEMPLATE" in fn:
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
        recs[rid] = {"id": rid, "text": text, "_where": f"releases/{fn}"}
    return recs


# ---- load and validate ------------------------------------------------------------
REQ_FIELDS = ["Level", "Board", "Statement", "Rationale", "Source", "Method", "Verified by", "Phase", "Status"]
DF_COLS = ["id", "board", "item", "function", "failure_mode", "effect", "severity", "cause", "prevention",
           "detection", "occurrence", "detection_rating", "action_priority", "action", "status"]
VER_COLS = ["id", "title", "method", "board", "procedure", "pass_limit", "covers", "last_result", "evidence",
            "commit", "date", "phase"]
VP_COLS = ["num", "title", "maps_to", "cut_reason", "status_file", "status_now", "evidence"]
DEF_FIELDS = ["Found", "Symptom", "Root cause", "Why missed", "Check added or changed", "Updated", "Status"]


def load():
    reqs = parse_blocks("requirements.md", RE["REQ"], REQ_FIELDS)
    for r in reqs.values():
        f, w = r["f"], r["_where"]
        if f.get("Level") and f["Level"] not in LEVELS:
            err(w, f"{r['id']}: Level {f['Level']!r} not in {LEVELS}")
        if f.get("Board") and f["Board"] not in BOARD_VALUES:
            err(w, f"{r['id']}: Board {f['Board']!r} not in {BOARD_VALUES}")
        meth = set(re.split(r"[,\s/]+", f.get("Method", "")) ) - {""}
        if not meth or not meth <= METHODS:
            err(w, f"{r['id']}: Method {f.get('Method')!r}: use T, A, I, D")
        if f.get("Phase") and f["Phase"] not in PHASES:
            err(w, f"{r['id']}: Phase {f['Phase']!r} not in {PHASES}")
        if f.get("Status") and f["Status"] not in REQ_STATUS:
            err(w, f"{r['id']}: Status {f['Status']!r} not in {REQ_STATUS}")
        vb = f.get("Verified by", "")
        r["vers"] = VER_ID.findall(vb)
        if vb and vb.lower() != "none" and not r["vers"]:
            err(w, f"{r['id']}: 'Verified by' lists no VER id (write 'none' if there is none yet)")

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
        if d["status"] == "ACCEPTED" and not ACCEPT.match(d["action"]):
            err(w, f"{d['id']}: ACCEPTED needs an action starting 'Accepted by <name> <YYYY-MM-DD>:'")
        for k in ("item", "function", "failure_mode", "effect", "cause"):
            if not d[k]:
                err(w, f"{d['id']}: {k} is empty")
        d["vers"] = VER_ID.findall(d["detection"])
        dfs[d["id"]] = d

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
        v["cov"] = parse_covers(v["covers"])
        for g, _, tok in v["cov"]:
            if not any(g.match(t) for t in TRACKED):
                err(w, f"{v['id']}: covers {tok!r} matches no tracked file")
        for tok in v["evidence"].split():
            if re.match(r"^hardware/\S+\.\w+$", tok) and not os.path.exists(os.path.join(ROOT, tok)):
                err(w, f"{v['id']}: evidence file {tok} does not exist")
        vers[v["id"]] = v

    plan = parse_validation_plan()
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
        if num not in plan:
            err(w, f"#{num} is not a method in validation_plan.md")
        else:
            t, s = plan[num]
            if p["title"] != t:
                err(w, f"#{num}: title differs from validation_plan.md ({t!r})")
            if p["status_file"] != s:
                err(w, f"#{num}: status_file {p['status_file']!r} but validation_plan.md says {s!r}")
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
    missing = sorted(set(plan) - set(vps))
    if missing:
        err("vp_map.csv", f"methods missing: {missing}")

    defs = parse_blocks("defects.md", RE["DEF"], DEF_FIELDS)
    for d in defs.values():
        st = d["f"].get("Status", "").split()[0] if d["f"].get("Status") else ""
        d["status"] = st
        if st not in DEF_STATUS:
            err(d["_where"], f"{d['id']}: Status must start with one of {DEF_STATUS}")
        d["vers"] = VER_ID.findall(d["f"].get("Check added or changed", ""))
        if st == "CLOSED" and not d["vers"]:
            err(d["_where"], f"{d['id']}: CLOSED with no VER in 'Check added or changed' (no defect closes without a check)")

    crit, waivers = parse_gates()
    recs = parse_records()

    # every ID referenced anywhere must be defined somewhere
    defined = set(reqs) | set(dfs) | set(vers) | set(defs) | set(crit) | set(recs) | {w["id"] for w in waivers}
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
                    or t == "DEFECTS:CLOSED"):
                err(c["_where"], f"{c['id']}: needs token {t!r} is not an ID, PHASE:<phase> or DEFECTS:CLOSED")
            texts.append((c["_where"], t))
    for w in waivers:
        texts.append((w["_where"], w["crit"] + " " + w["reason"]))
    for rr in recs.values():
        texts.append((rr["_where"], rr["text"]))
    seen_bad = set()
    for where, t in texts:
        for i in ANY_ID.findall(t):
            if i not in defined and (where, i) not in seen_bad:
                seen_bad.add((where, i))
                err(where, f"{i} is referenced but not defined")
    for w in waivers:
        if w["crit"] not in crit:
            err(w["_where"], f"{w['id']}: waives {w['crit']!r}, which is not a criterion")
    return dict(reqs=reqs, dfs=dfs, vers=vers, vps=vps, defs=defs, crit=crit, waivers=waivers, recs=recs)


# ---- freshness ---------------------------------------------------------------------
def staleness(v, dirty=None):
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
    for g, rx, tok in v["cov"]:
        for f in sorted(changed):
            if g.match(f) and (rx is None or any(rx.search(ln) for ln in changed_lines(c, f))):
                hits.append(f)
    if dirty:
        for g, _, tok in v["cov"]:
            hits += [f + " (uncommitted)" for f in sorted(dirty) if g.match(f)]
    if hits:
        uniq = sorted(set(hits))
        return "changed: " + ", ".join(uniq[:4]) + (f" (+{len(uniq) - 4})" if len(uniq) > 4 else "")
    return None


def for_board(item_board, board):
    return item_board in (board, "both")


def phase_le(a, b):
    return PHASES.index(a) <= PHASES.index(b)


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


def report(db, verbose):
    reqs, dfs, vers, vps, defs = db["reqs"], db["dfs"], db["vers"], db["vps"], db["defs"]
    print(f"trace report at {HEAD[:7]} ({ROOT})")
    print(f"  requirements {len(reqs)} " + str(dict(Counter(r['f'].get('Status') for r in reqs.values()))))
    print(f"  DFMEA rows {len(dfs)}, action priority " + str(dict(sorted(Counter(d['action_priority'] for d in dfs.values()).items()))))
    print(f"  checks {len(vers)} " + str(dict(Counter(v['last_result'] for v in vers.values()))))
    cut = [p for p in vps.values() if p["cut"]]
    print(f"  validation-plan methods {len(vps)}: {len(vps) - len(cut)} mapped, {len(cut)} cut")
    print(f"  defects {len(defs)} " + str(dict(Counter(d['status'] for d in defs.values()))))
    print()
    print("requirements")
    for b in BOARDS:
        show(f"no check for {b}", [r for r in reqs if not any(for_board(vers[v]["board"], b)
                                                             for v in reqs[r]["vers"] if v in vers)
                                   and for_board(reqs[r]["f"].get("Board", ""), b)], verbose)
    show("no listed check has passed", [r for r in reqs if reqs[r]["vers"] and not any(
        vers.get(v, {}).get("last_result") == "PASS" for v in reqs[r]["vers"])], verbose)
    show("statement has no number (INCOSE: state a value and tolerance)",
         [r for r in reqs if not re.search(r"\d", reqs[r]["f"].get("Statement", ""))], verbose)
    print("DFMEA")
    hi = [d for d in dfs.values() if d["action_priority"] == "H"]
    show("H rows with no detection check", [d["id"] for d in hi if not d["vers"]], verbose)
    show("H rows whose detection has not passed on fresh evidence",
         [d["id"] for d in hi if d["vers"] and not any(
             vers.get(v, {}).get("last_result") == "PASS" and not staleness(vers[v]) for v in d["vers"])
          and d["status"] != "ACCEPTED"], verbose)
    show("severity 9-10 rows", [d["id"] for d in dfs.values() if d["S"] >= 9], verbose)
    print("checks")
    for res in ("FAIL", "NOT_RUN", "NOT_BUILT"):
        show(res, [v for v in vers if vers[v]["last_result"] == res], verbose)
    stale = [(v, staleness(vers[v])) for v in vers]
    stale = [(v, s) for v, s in stale if s]
    print(f"  stale evidence: {len(stale)}")
    for v, s in stale:
        print(f"    {v} ({vers[v]['last_result']}, {vers[v]['commit']}): {s}")
    used = set()
    for r in reqs.values():
        used |= set(r["vers"])
    for d in dfs.values():
        used |= set(d["vers"])
    for p in vps.values():
        used |= set(p["ids"])
    for d in defs.values():
        used |= set(d["vers"])
    for c in db["crit"].values():
        used |= {t for t in c["needs"] if t.startswith("VER-")}
    show("not referenced by any requirement, risk, gate, defect or plan method",
         [v for v in vers if v not in used], verbose)
    print("validation plan")
    show("unmapped methods", [f"#{n}" for n, p in sorted(vps.items()) if not p["cut"] and not p["ids"]
                              and not p["maps_to"].startswith("DOC:")], verbose)
    changed = [f"#{n}:{p['status_file']}->{p['status_now']}" for n, p in sorted(vps.items())
               if p["status_now"] != p["status_file"]]
    show("statuses corrected against the repo", changed, verbose, limit=12)
    print("defects")
    show("OPEN", [d for d in defs if defs[d]["status"] == "OPEN"], verbose)
    show("FIXED but no check yet passes (cannot close)", [d for d in defs if defs[d]["status"] == "FIXED"], verbose)


# ---- gates ---------------------------------------------------------------------------
def gate(db, phase, board, verbose):
    reqs, dfs, vers, defs, crit = db["reqs"], db["dfs"], db["vers"], db["defs"], db["crit"]
    dirty = dirty_files()
    waived = {w["crit"]: w for w in db["waivers"] if w["who"] not in ("", "-") and DATE.match(w["date"])}

    def ver_ok(vid):
        v = vers.get(vid)
        if v is None:
            return False, f"{vid} undefined"
        if v["last_result"] != "PASS":
            return False, f"{vid} {v['last_result']}"
        s = staleness(v, dirty)
        return (False, f"{vid} stale ({s})") if s else (True, "")

    results = []

    def need_ok(tok):
        if tok.startswith("PHASE:"):
            p = tok.split(":")[1]
            bad = [v for v in vers if for_board(vers[v]["board"], board) and phase_le(vers[v]["phase"], p)
                   and not ver_ok(v)[0]]
            return (not bad), (f"{len(bad)} checks at or before {p} not passing: " + " ".join(bad[:8])) if bad else ""
        if tok == "DEFECTS:CLOSED":
            bad = [d for d in defs if defs[d]["status"] != "CLOSED"]
            return (not bad), (f"not CLOSED: {' '.join(bad[:8])}" + (" ..." if len(bad) > 8 else "")) if bad else ""
        if tok.startswith("VER-"):
            if tok in vers and not for_board(vers[tok]["board"], board):
                return None, f"{tok} is a {vers[tok]['board']} check"
            return ver_ok(tok)
        if tok.startswith("REQ-"):
            r = reqs.get(tok)
            if r is None:
                return False, f"{tok} undefined"
            if r["f"].get("Status") != "APPROVED":
                return False, f"{tok} {r['f'].get('Status')}"
            bad = [v for v in r["vers"] if v in vers and for_board(vers[v]["board"], board)
                   and phase_le(vers[v]["phase"], phase) and not ver_ok(v)[0]]
            return (not bad), (f"{tok} checks not passing: {' '.join(bad)}" if bad else "")
        if tok.startswith("DF-"):
            d = dfs.get(tok)
            if d is None:
                return False, f"{tok} undefined"
            if d["status"] == "ACCEPTED" or (d["status"] == "DONE" and any(ver_ok(v)[0] for v in d["vers"])):
                return True, ""
            return False, f"{tok} {d['status']}, not accepted and not DONE with a passing detection"
        if tok.startswith("DEF-"):
            d = defs.get(tok)
            if d is None:
                return False, f"{tok} undefined"
            return d["status"] == "CLOSED", (f"{tok} {d['status']}" if d["status"] != "CLOSED" else "")
        return False, f"{tok}: cannot evaluate"

    prefix = GATE_PREFIX[phase]
    for cid, c in sorted(crit.items()):
        if not cid.startswith(prefix):
            continue
        checked = [need_ok(t) for t in c["needs"]]
        reasons = [r for ok, r in checked if ok is False]
        if all(ok is None for ok, _ in checked):
            reasons = [f"nothing here applies to {board}: give it its own check ("
                       + "; ".join(r for _, r in checked) + ")"]
        if reasons and cid in waived:
            w = waived[cid]
            results.append((cid, True, [f"waived by {w['who']} {w['date']} ({w['id']})"]))
        else:
            results.append((cid, not reasons, reasons))

    # R1
    mine = {k: r for k, r in reqs.items() if for_board(r["f"].get("Board", ""), board)}
    prop = [k for k, r in mine.items() if r["f"].get("Status") == "PROPOSED"]
    results.append(("R1 requirements approved", not prop, [f"{len(prop)} PROPOSED: " + " ".join(prop[:10])
                                                           + (" ..." if len(prop) > 10 else "")] if prop else []))
    # R2
    nover, notpass = [], []
    for k, r in mine.items():
        if r["f"].get("Status") in ("REJECTED", "DEFERRED"):
            continue
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
    results.append(("R2 requirements verified", not rs, rs))
    # R3
    bad = []
    for d in dfs.values():
        if d["action_priority"] != "H" or not for_board(d["board"], board) or d["status"] == "ACCEPTED":
            continue
        if d["status"] != "DONE" or not any(v in vers and phase_le(vers[v]["phase"], phase) and ver_ok(v)[0]
                                            for v in d["vers"]):
            bad.append(d["id"])
    results.append(("R3 high-priority risks covered", not bad,
                    [f"{len(bad)} H rows neither ACCEPTED nor DONE with a passing detection at or before {phase}: "
                     + " ".join(bad[:12]) + (" ..." if len(bad) > 12 else "")] if bad else []))
    # R4
    if phase in ("ORDER", "EVT"):
        bad = [d for d in defs if defs[d]["status"] == "OPEN"]
        results.append(("R4 no open defects", not bad, [f"OPEN: {' '.join(bad)}"] if bad else []))
    else:
        bad = [d for d in defs if defs[d]["status"] != "CLOSED"]
        results.append(("R4 every defect closed", not bad, [f"not CLOSED: {' '.join(bad)}"] if bad else []))
    # R5
    dirty_hits = sorted({f for v in vers.values() if v["last_result"] == "PASS"
                         for g, _, _ in v["cov"] for f in dirty if g.match(f)})
    results.append(("R5 no uncommitted change under passed evidence", not dirty_hits,
                    [" ".join(dirty_hits[:6])] if dirty_hits else []))

    print(f"gate {phase} for {board} at {HEAD[:7]}")
    for cid, ok, reasons in results:
        print(f"  {'PASS' if ok else 'FAIL'}  {cid}" + (f"  {crit[cid]['text'][:70]}" if cid in crit else ""))
        for r in reasons if (verbose or not ok) else []:
            print(f"          {r}")
    n_bad = sum(1 for _, ok, _ in results if not ok)
    print(f"RESULT: {'PASS' if not n_bad else f'FAIL ({n_bad} of {len(results)} not met)'}")
    return n_bad == 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--gate", choices=sorted(GATE_ARG), help="evaluate a phase gate (exit 1 unless met)")
    ap.add_argument("--board", choices=BOARDS, default="ref_design")
    ap.add_argument("--verbose", action="store_true")
    a = ap.parse_args()
    db = load()
    if ERRORS:
        print(f"STRUCTURE: {len(ERRORS)} error(s)")
        for e in ERRORS:
            print("  " + e)
        sys.exit(1)
    print("STRUCTURE: OK")
    if a.gate:
        sys.exit(0 if gate(db, GATE_ARG[a.gate], a.board, a.verbose) else 1)
    report(db, a.verbose)
    sys.exit(0)


if __name__ == "__main__":
    main()
