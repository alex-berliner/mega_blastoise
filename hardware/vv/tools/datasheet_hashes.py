#!/usr/bin/env python3
"""Record the datasheet extracts this register quotes: sha256, byte size, the drawing's date.

    python3 hardware/vv/tools/datasheet_hashes.py            # write hardware/vv/evidence/<date>_VER-34_datasheets.txt
    python3 hardware/vv/tools/datasheet_hashes.py --print    # print only, write nothing

The extracts are not committed: they were downloaded on 2026-10-26 to the main checkout's
hardware/ref_design/datasheets and stay out of git (the ILI9488 text may not be
redistributed). What the register needs from them is their identity, so a number quoted
from a file can be checked later without the file being in the repository: the sha256, the
byte size, the line count and the date on the drawing.

The directory comes from the first argument, then $VV_DATASHEETS, then the main checkout's
hardware/ref_design/datasheets (found through git's common dir, so a worktree finds its
main checkout). Every required file must be there: a missing one is an error, not a
silent skip, because a quoted number would then have no recorded source.

The output is a record, not a check of its own: VER-34 quotes the constants from these
files and cites this file's hashes.

Python 3 standard library only.
"""
import datetime
import hashlib
import os
import platform
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
VV = os.path.dirname(HERE)
ROOT = os.path.dirname(os.path.dirname(VV))

# the extracts the register quotes: the panel's mechanics, the ILI9488's interface, the
# charger's limits and the RP2350's leakage. The .txt is what the line numbers cite; the .pdf
# is what it was extracted from (the panel's outline drawing is a picture in it).
REQUIRED = [f"{n}.{ext}" for n in ("bq24074", "er-tft035ips-6", "ili9488", "rp2350") for ext in ("txt", "pdf")]

DATE_PAT = re.compile(
    r"\b(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*[ \-]\d{1,2}(?:[ \-]\d{2,4})?\b"
    r"|\b(?:19|20)\d{2}-\d{2}-\d{2}\b"
    r"|\b\d{1,2}/\d{1,2}/(?:19|20)?\d{2}\b")


def git(*args):
    try:
        r = subprocess.run(["git", "-C", ROOT] + list(args), capture_output=True, text=True)
        return r.stdout.strip() if r.returncode == 0 else ""
    except OSError:
        return ""


def datasheet_dir(argv):
    """Where the extracts are: the argument, then $VV_DATASHEETS, then the main checkout."""
    if argv:
        return os.path.abspath(argv[0]), "the argument"
    env = os.environ.get("VV_DATASHEETS")
    if env:
        return os.path.abspath(env), "$VV_DATASHEETS"
    common = git("rev-parse", "--git-common-dir")
    if common:
        main_root = os.path.dirname(os.path.abspath(os.path.join(ROOT, common)))
        return os.path.join(main_root, "hardware", "ref_design", "datasheets"), "the main checkout"
    return os.path.join(ROOT, "hardware", "ref_design", "datasheets"), "this worktree"


def dates(text):
    """The date-like strings in a text, with their line numbers (the drawing's date)."""
    out = []
    for n, line in enumerate(text.splitlines(), 1):
        for m in DATE_PAT.findall(line):
            m = m.strip()
            if m and (m, n) not in out:
                out.append((m, n))
    return out[:8]


def describe(path):
    data = open(path, "rb").read()
    text = data.decode("utf-8", errors="replace")
    found = dates(text) if path.endswith(".txt") else []     # a PDF's dates are read from its .txt
    return {
        "name": os.path.basename(path),
        "sha256": hashlib.sha256(data).hexdigest(),
        "bytes": len(data),
        "lines": text.count("\n") + (0 if text.endswith("\n") else 1),
        "dates": found,
    }


def tracked(name):
    """True when this worktree's git tracks the file (the extracts are not tracked)."""
    out = git("ls-files", "--", os.path.join("hardware", "ref_design", "datasheets", name))
    return bool(out)


def main():
    argv = [a for a in sys.argv[1:] if not a.startswith("--")]
    write = "--print" not in sys.argv
    directory, source = datasheet_dir(argv)
    print(f"datasheet directory: {directory} (from {source})")
    if not os.path.isdir(directory):
        print(f"not a directory: {directory}")
        print("the extracts are downloaded to the main checkout and stay out of git; "
              "pass the directory or set VV_DATASHEETS")
        return 1
    rows = []
    missing = []
    for name in REQUIRED:
        path = os.path.join(directory, name)
        if not os.path.isfile(path):
            missing.append(name)
            continue
        rows.append(describe(path))
    others = sorted(f for f in os.listdir(directory)
                    if f.endswith((".txt", ".md")) and f not in REQUIRED)
    commit = git("rev-parse", "HEAD")
    print(f"commit: {commit or 'unknown'}")
    for row in rows:
        print(f"{row['name']}: sha256 {row['sha256']}, {row['bytes']} bytes, {row['lines']} lines, "
              f"{'tracked' if tracked(row['name']) else 'untracked (an extract)'}")
        for stamp, line in row["dates"]:
            print(f"    date {stamp!r} at line {line}")
        if not row["dates"] and row["name"].endswith(".txt"):
            print("    no date-like string found")
    for name in others:
        path = os.path.join(directory, name)
        try:
            row = describe(path)
        except OSError:
            continue
        print(f"{row['name']}: sha256 {row['sha256']}, {row['bytes']} bytes, {row['lines']} lines "
              f"({'tracked' if tracked(row['name']) else 'untracked'})")
    if missing:
        print(f"missing: {' '.join(missing)}")
    body = []
    body.append("# the datasheet extracts this register quotes")
    body.append(f"# directory: {directory}")
    body.append(f"COMMIT: {commit or 'unknown'}")
    body.append(f"TOOLS: python {platform.python_version()}")
    body.append("")
    body.append("file | sha256 | bytes | lines | in git | dates")
    for row in rows:
        stamps = "; ".join(f"{s} (line {n})" for s, n in row["dates"]) or "none found"
        body.append(f"{row['name']} | {row['sha256']} | {row['bytes']} | {row['lines']} | "
                    f"{'tracked' if tracked(row['name']) else 'untracked'} | {stamps}")
    for name in others:
        path = os.path.join(directory, name)
        try:
            row = describe(path)
        except OSError:
            continue
        stamps = "; ".join(f"{s} (line {n})" for s, n in row["dates"]) or "none found"
        body.append(f"{row['name']} | {row['sha256']} | {row['bytes']} | {row['lines']} | "
                    f"{'tracked' if tracked(row['name']) else 'untracked'} | {stamps}")
    body.append("")
    body.append("the extracts themselves are not committed; the numbers this register quotes from")
    body.append("them are cited by file and line, and these hashes say which file that was")
    body.append(f"RESULT: {'FAIL' if missing else 'PASS'}")
    text = "\n".join(body) + "\n"
    if write:
        out = os.path.join(VV, "evidence", f"{datetime.date.today().isoformat()}_VER-34_datasheets.txt")
        os.makedirs(os.path.dirname(out), exist_ok=True)
        open(out, "w", encoding="utf-8").write(text)
        print(f"wrote {os.path.relpath(out, ROOT)}")
    else:
        print(text)
    return 1 if missing else 0


if __name__ == "__main__":
    sys.exit(main())
