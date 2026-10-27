"""Datasheet quotes the checks read from the PDFs themselves, so a quoted number cannot drift from its file.

    import ds
    ds.quote("bq24074", 3061, "RGT")            # the line's text; exits 2 (BLOCKED) if the file is missing,
                                                # exits 1 if the tokens are not on that line (+-1 line)

text(name) is `pdftotext -layout hardware/ref_design/datasheets/<name>.pdf -`, the same extract the
register's '<name>.txt:<line>' citations count lines in (rp2350.txt's sha256 fa2cad47... and
bq24074.txt's are recorded by datasheet_hashes.py; this module prints each file's sha256 once, so the
evidence names the extract it read). A cache under the system temp dir, keyed by the PDF's sha256,
saves the second extraction. Nothing is written under the repository.

quote() is what makes a constant checkable: each one a check uses names its file, its line and the
tokens that line must hold (the number itself and a word of its label). A datasheet that is not on disk
is BLOCKED (exit 2): run_checks.py then records nothing, and the check's row names the part and URL.

Python 3 standard library, plus the pdftotext binary (poppler-utils).
"""
import hashlib
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(HERE)))
DS = os.path.join(ROOT, "hardware", "ref_design", "datasheets")
_TEXT, _SHOWN = {}, set()


def path(name):
    return os.path.join(DS, name + ".pdf")


def have(name):
    return os.path.isfile(path(name))


def blocked(what):
    """Print why and exit 2: the check cannot run until an input is on disk."""
    print(f"BLOCKED: {what}")
    print("RESULT: BLOCKED")
    sys.exit(2)


def text(name):
    if name in _TEXT:
        return _TEXT[name]
    p = path(name)
    if not os.path.isfile(p):
        blocked(f"{name}.pdf is not in hardware/ref_design/datasheets")
    sha = hashlib.sha256(open(p, "rb").read()).hexdigest()
    cache = os.path.join(tempfile.gettempdir(), f"vv_ds_{name}_{sha[:16]}.txt")
    if os.path.isfile(cache):
        t = open(cache, encoding="utf-8").read()
    else:
        r = subprocess.run(["pdftotext", "-layout", p, "-"], capture_output=True, text=True)
        if r.returncode:
            blocked(f"pdftotext failed on {name}.pdf: {r.stderr.strip()[:200]}")
        t = r.stdout
        try:
            open(cache, "w", encoding="utf-8").write(t)
        except OSError:
            pass
    _TEXT[name] = t.split("\n")
    if name not in _SHOWN:
        _SHOWN.add(name)
        print(f"[{name}.pdf sha256 {sha}; its pdftotext -layout text sha256 "
              f"{hashlib.sha256(t.encode()).hexdigest()[:16]}..., {t.count(chr(10))} lines]")
    return _TEXT[name]


def quote(name, line, *tokens):
    """The text of <name>.txt:<line>, after checking every token is on it or the next line (a table row the
    layout wrapped). A token is a plain string matched without case, or a compiled regex."""
    lines = text(name)
    win = " ".join(lines[max(0, line - 1):line + 1])
    flat = re.sub(r"\s+", " ", win)
    for t in tokens:
        ok = t.search(flat) if hasattr(t, "search") else t.lower() in flat.lower()
        if not ok:
            print(f"QUOTE WRONG: {name}.txt:{line} does not hold {getattr(t, 'pattern', t)!r}: {flat.strip()[:160]!r}")
            print("RESULT: FAIL")
            sys.exit(1)
    return re.sub(r"\s+", " ", lines[line - 1]).strip()


def find(name, pattern, start=0):
    """The first line number (1-based) at or after start whose text matches pattern (a helper for writing
    quotes; checks cite fixed lines)."""
    rx = re.compile(pattern, re.I)
    for i, ln in enumerate(text(name)[start:], start + 1):
        if rx.search(ln):
            return i
    return None
