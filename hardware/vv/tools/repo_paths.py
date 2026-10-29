#!/usr/bin/env python3
"""Find repository paths the register names that were moved or deleted.

    python3 hardware/vv/tools/repo_paths.py              # exit 1 on any moved or deleted path
    python3 hardware/vv/tools/repo_paths.py --vv <copy>  # another copy of hardware/vv (seed_faults.py)
    python3 hardware/vv/tools/repo_paths.py --verbose    # also list the names never committed (planned files)

On 2026-10-28 main moved or deleted about 180 of the paths this register named (docs/living,
docs/reference, hardware/ref_design/board/membrane; Alex's board and case and eleven datasheets
deleted in 574c94c), and nothing failed: a 'covers' glob is checked, a path in a procedure, a
source or a tool is not. This scan reads every file under hardware/vv but the saved outputs
(evidence/*.txt: what a run printed, never edited) and finds each repository path in it: a token
that starts with a top-level directory (hardware/, docs/, crates/, scripts/, site/, tools/) or with a
directory of hardware/, hardware/ref_design or hardware/vv (ref_design/, board/, case/, docs/,
lib/, datasheets/, tools/, evidence/, releases/, analysis/ ...), with a glob's '*' allowed and a
'covers' '::regex' cut off. A token is resolved as written, against the file's own directory, and
under hardware/, hardware/ref_design/, hardware/ref_design/board/ and hardware/vv/. Then:

  ok        it is in HEAD's tree (a file or a directory; a glob matches a file), or git ignores it
            (a generated output such as case/positions.json);
  pinned    it is written 'path@<commit>' and that commit has it: history, read where it was
            (a deleted board's files, a deleted datasheet, an evidence file a later run replaced);
  MOVED or  it is not in HEAD's tree but HEAD's history had it: the error names the commit that
  DELETED   removed it and the last commit that had it, so the fix is to point at the new path or
            to pin the old one;
  planned   no commit on HEAD's history ever had it, and something says it will exist: it is named
            on a verification.csv row that is NOT_BUILT (the tool or file that row will build) or
            gone (RETIRED, DROPPED or CUT: history), on a HISTORY.md line, or it matches PLANNED
            below (a file a check or a release will write: analysis/cell.json, evidence/...);
            listed with --verbose;
  UNKNOWN   never committed and nothing says it will be: a misspelt path, or a planned file to add
            to PLANNED with what writes it (F's audit, 2026-10-28: a typo in a live path passed as
            planned).

A pinned path whose commit lacks it is an error too. tools/seed_faults.py is not read: the moved
and wrongly pinned paths in it are the faults it seeds (VER-129 runs them). Template names ('{today}', '<date>') are not
read. This is VER-220; check_trace.py's default mode runs it, so a move on main fails the register's
structure at once. Python 3 standard library and git.
"""
import csv
import fnmatch
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
TOP = r"hardware|docs|crates|scripts|site|tools|patches"
SUB = (r"board|case|lib|datasheets|evidence|releases|analysis|viewer|membrane|pin_review|living|reference|static|"
       r"archive|architecture|ds_lite|prod_board|prod_case|stripboard_v2|ref_design|vv")
TOKEN = re.compile(r"(?<![\w./@-])((?:\.\./)*(?:" + TOP + "|" + SUB + r")/[\w./*-]*[\w*])(@[0-9a-f]{7,40})?")
UNDER = ("", "hardware/", "hardware/ref_design/", "hardware/ref_design/board/", "hardware/vv/")
# Files never committed that the register names on purpose, each with what will write it (globs allowed). A path
# that is none of these, and is named on no NOT_BUILT or gone row of verification.csv, is UNKNOWN: an error.
PLANNED = {
    "hardware/vv/evidence/*": "a saved output a check will write (check_trace checks every PASS's evidence file)",
    "hardware/vv/releases/REL*": "a release record, from releases/TEMPLATE.md",
    "hardware/vv/releases/CHG*": "a change record, from releases/CHANGE_TEMPLATE.md",
    "hardware/vv/analysis/cell.json": "the cell's spec-sheet numbers, written when the spec sheet arrives (VER-37)",
    "hardware/ref_design/board/cell.json": "the same numbers beside design.py, for VER-160's assert (VER-37)",
    "hardware/vv/analysis/jlc_capabilities.md": "JLC's dated capability page, saved by VER-17",
    "hardware/vv/tools/release.py": "the release run VER-143 builds (VER-42 reads its output)",
    "hardware/ref_design/board/pin_review/ds": "DEF-21's finding: the folder the pin review cited, which never existed",
    "hardware/ref_design/datasheets/ER-TFT035IPS-6_Datasheet.pdf": "the panel module's datasheet, copied in by Claude from the panel_bus scratchpad (REQ-PRD-02.B)",
    "hardware/ref_design/datasheets/ILI9488.pdf": "the controller's datasheet, copied in by Claude from the panel_bus scratchpad (REQ-PRD-02.B)",
}


def _root(start):
    r = subprocess.run(["git", "-C", start, "rev-parse", "--show-toplevel"], capture_output=True, text=True)
    return r.stdout.strip()


ROOT = _root(HERE)


def git(*args, inp=None):
    r = subprocess.run(["git", "-C", ROOT] + list(args), capture_output=True, text=True, input=inp)
    return r.returncode, r.stdout


def _dirs(paths):
    out = set()
    for p in paths:
        while "/" in p:
            p = p.rsplit("/", 1)[0]
            out.add(p)
    return out


_cache = {}


def _trees():
    if not _cache:
        head = set(git("ls-tree", "-r", "--name-only", "HEAD")[1].split("\n")) - {""}
        hist = set(git("log", "--no-renames", "--format=", "--name-only", "HEAD")[1].split("\n")) - {""}
        _cache.update(head=head, head_dirs=_dirs(head), hist=hist, hist_dirs=_dirs(hist))
    return _cache["head"], _cache["head_dirs"], _cache["hist"], _cache["hist_dirs"]


def _match(cand, files, dirs):
    if "*" in cand:
        return any(fnmatch.fnmatch(t, cand) for t in files)
    return cand in files or cand in dirs


def candidates(tok, base):
    """The repository paths a token can mean, as written first."""
    out = []
    for c in [os.path.normpath(os.path.join(base, tok))] + [u + tok for u in UNDER]:
        c = os.path.normpath(c).rstrip("/")
        if not c.startswith("..") and c not in out:
            out.append(c)
    return out


def ignored(paths):
    """The paths git ignores (generated outputs: case/positions.json, the fab zips)."""
    if not paths:
        return set()
    r = subprocess.run(["git", "-C", ROOT, "check-ignore", "--no-index", "--stdin"], capture_output=True,
                       text=True, input="\n".join(paths) + "\n")
    return set(r.stdout.split("\n")) - {""}


def gone_where(path):
    """(the commit that removed it, the last commit that had it), short hashes; ('', '') if unknown."""
    spec = path if "*" not in path else ":(glob)" + path
    rc, out = git("log", "-1", "--no-renames", "--diff-filter=D", "--format=%h", "HEAD", "--", spec)
    gone = out.strip()
    if not gone:
        return "", ""
    rc, par = git("rev-parse", "--short", gone + "^")
    return gone, par.strip()


def scan_files(vv):
    for d, subs, fs in os.walk(vv):
        subs[:] = sorted(s for s in subs if s != "__pycache__")
        for f in sorted(fs):
            rel = os.path.relpath(os.path.join(d, f), vv).replace(os.sep, "/")
            if rel.startswith("evidence/") and rel.endswith(".txt"):
                continue            # a saved output: what a run printed then, never edited
            if rel == "tools/seed_faults.py":
                continue            # its moved and wrongly pinned paths are the faults it seeds (VER-129)
            if not f.endswith((".md", ".csv", ".py", ".txt")):
                continue
            yield rel


def planned_ok(rel, line, cands):
    """Is a never-committed path one the register means to exist? (see the docstring: planned or UNKNOWN)"""
    if rel == "HISTORY.md":
        return True
    if rel == "verification.csv":
        row = next(csv.reader([line]), [])
        if len(row) > 7 and row[7] in ("NOT_BUILT", "RETIRED", "DROPPED", "CUT"):
            return True
    return any(fnmatch.fnmatch(c, p) or c == p for c in cands for p in PLANNED)


def findings(vv):
    """(where, path, message) for every moved, deleted, wrongly pinned or unknown path; and the planned names."""
    head, head_dirs, hist, hist_dirs = _trees()
    bad, planned, seen = [], [], {}
    raw = []
    for rel in scan_files(vv):
        base = "hardware/vv/" + os.path.dirname(rel)
        for n, line in enumerate(open(os.path.join(vv, rel), encoding="utf-8", errors="replace"), 1):
            for m in TOKEN.finditer(line):
                tok, pin = m.group(1).rstrip("."), m.group(2)
                if "{" in line[m.start():m.end() + 1] or line[m.end():m.end() + 1] == "<" or tok.endswith("/"):
                    continue                # a template name: '{today}', 'samsung_<mpn>'
                raw.append((f"{rel}:{n}", tok, pin[1:] if pin else None, base, rel, line))
    ign = ignored(sorted({c for _, tok, pin, base, _, _ in raw if not pin for c in candidates(tok, base)
                          if "*" not in c}))
    for where, tok, pin, base, rel, line in raw:
        cands = candidates(tok, base)
        if pin:
            if not any(git("cat-file", "-e", f"{pin}:{c}")[0] == 0 or git("ls-tree", "-d", pin, c)[1].strip()
                       for c in cands if "*" not in c):
                bad.append((where, tok, f"pinned at {pin}, which does not have it"))
            continue
        key = (tok, base)
        if key not in seen:
            if any(_match(c, head, head_dirs) or c in ign for c in cands):
                seen[key] = None
            else:
                was = next((c for c in cands if _match(c, hist, hist_dirs)), None)
                if was:
                    gone, last = gone_where(was)
                    seen[key] = (f"{was} is not in HEAD's tree: moved or deleted"
                                 + (f" in {gone} (last in {last})" if gone else "")
                                 + ": point at its new path, or pin the old one as path@<commit>")
                else:
                    seen[key] = "planned"
        if seen[key] == "planned":
            if planned_ok(rel, line, cands):
                planned.append((where, tok))
            else:
                bad.append((where, tok, f"{tok} was never committed, and no NOT_BUILT check and no entry of "
                                        "repo_paths.PLANNED says it will be: a misspelt path, or a planned file to list"))
        elif seen[key]:
            bad.append((where, tok, seen[key]))
    return bad, planned


def main():
    vv = os.path.dirname(HERE)
    if "--vv" in sys.argv:
        vv = os.path.abspath(sys.argv[sys.argv.index("--vv") + 1])
    bad, planned = findings(vv)
    print(f"repository paths named under {os.path.relpath(vv, ROOT) if vv.startswith(ROOT) else vv} at "
          f"{git('rev-parse', '--short', 'HEAD')[1].strip()}: {len(bad)} moved, deleted or wrongly pinned; "
          f"{len(planned)} planned (never committed)")
    for where, tok, msg in bad:
        print(f"  {where}: {msg}")
    if "--verbose" in sys.argv:
        for where, tok in planned:
            print(f"  planned  {where}: {tok}")
    print(f"RESULT: {'FAIL' if bad else 'PASS'}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
