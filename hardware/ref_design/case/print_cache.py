"""What the print packagers share: when to slice, and the cache of sliced plates.

Slicing is the slow part (about 12 min for the U1's two plates), so the packagers only slice when asked: `--slice` on the
command line (tools/build.py --slice passes it) or RELEASE=1 in the environment. Without it they write the projects, STLs and
zips in seconds.

A sliced plate is kept in print/.slice_cache/ (not in git) under a key made of its STLs' contents, the plate name, the
profile files the slicer reads, the packagers' own sources (they decide what goes into a project) and the slicer's identity.
The same key again takes the cached file instead of slicing. `--no-cache` slices again and replaces the entry.
"""
import glob
import hashlib
import os
import shutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CACHE = os.path.join(HERE, "print", ".slice_cache")
SOURCES = ("print_package.py", "print_package_p1s.py", "slice_u1.py", "support_check.py", "overhangs.py", "print_cache.py")
PROFILES = ("print/fit_test_v5.3mf", "print/u1_system/*.json", "print/p1s/*.json", "print/p1s/system/*.json")


def wanted():
    """Slice (rather than only write)? --slice or RELEASE=1."""
    return "--slice" in sys.argv or os.environ.get("RELEASE") == "1"


def cached():
    return "--no-cache" not in sys.argv


def names():
    """The plate names on the command line (everything not a --flag)."""
    return [a for a in sys.argv[1:] if not a.startswith("--")]


def _digest(h, path):
    h.update(os.path.basename(path).encode() + b"\0")
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)


def slicer_id(exe):
    """The slicer's identity: its path, size and mtime, plus those of its binary next to an AppRun (Bambu Studio's extracted AppImage)."""
    exe = os.path.realpath(exe)
    parts = [exe]
    for p in [exe, *glob.glob(os.path.join(os.path.dirname(exe), "bin", "*"))]:
        if os.path.isfile(p):
            st = os.stat(p)
            parts.append(f"{os.path.basename(p)}:{st.st_size}:{int(st.st_mtime)}")
    return "|".join(parts)


def key(kind, plate, stls, exe):
    """Cache key of one plate: kind ("U1"/"P1S"), name, the STLs' contents, profiles, sources and slicer."""
    h = hashlib.sha256()
    h.update(f"v1|{kind}|{plate}|{slicer_id(exe)}".encode() + b"\0")
    for p in sorted(set(stls)):
        _digest(h, p)
    for pat in PROFILES:
        for p in sorted(glob.glob(os.path.join(HERE, pat))):
            _digest(h, p)
    for s in SOURCES:
        _digest(h, os.path.join(HERE, s))
    return h.hexdigest()[:24]


def get(k, name):
    """The cached file `name` of key k, or None."""
    p = os.path.join(CACHE, k, name)
    return p if cached() and os.path.exists(p) else None


def put(k, *files):
    """Keep these files (paths) under key k, replacing the entry."""
    d = os.path.join(CACHE, k)
    shutil.rmtree(d, ignore_errors=True)
    os.makedirs(d)
    for f in files:
        shutil.copy(f, d)
