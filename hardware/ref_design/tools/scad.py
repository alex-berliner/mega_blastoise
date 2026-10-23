"""Which OpenSCAD the build uses, and how many renders run at once.

    import scad                      # (tools/ on sys.path)
    subprocess.run([*scad.cmd(), "-o", out, ..., "case.scad"])
    with ThreadPoolExecutor(scad.jobs()) as pool: ...

OpenSCAD: $OPENSCAD if set (a path, or "openscad" for the system one), else the newest
OpenSCAD AppImage in ~/Applications (a development snapshot from openscad.org), else the system
openscad. A build that has the Manifold geometry backend renders with it (minutes become
seconds). Its parts match 2021.01's to 0.01 mm in volume, bounding box and surface, except
engraved text, which newer OpenSCADs lay out a few tenths of a mm differently with either
backend; a few sub-micron slivers keep some Manifold meshes from being watertight, which the
slicers don't mind. OPENSCAD_BACKEND=CGAL picks the old backend again.
2021.01, the system openscad, has only CGAL and gets no --backend.

Renders at once: $REF_JOBS if set, else the machine's cores less two (fewer if memory is short).
The outputs never depend on how many run at once: each render is its own process and file.
"""
import functools
import glob
import os
import re
import shutil
import subprocess


@functools.lru_cache(None)
def binary():
    if os.environ.get("OPENSCAD"):
        return os.environ["OPENSCAD"]
    snaps = glob.glob(os.path.expanduser("~/Applications/OpenSCAD-*.AppImage"))
    # OpenSCAD-2026.10.18-x86_64.AppImage: the dates sort as text
    snaps = [p for p in snaps if os.access(p, os.X_OK)]
    return max(snaps) if snaps else "openscad"


@functools.lru_cache(None)
def backend():
    """'Manifold' or 'CGAL' when the binary takes --backend, else None (CGAL, the only one)."""
    try:
        r = subprocess.run([binary(), "--help"], capture_output=True, text=True, timeout=60)
        help_text = r.stdout + r.stderr          # (it prints its help on stderr)
    except (OSError, subprocess.TimeoutExpired):
        return None
    if "--backend" not in help_text:
        return None
    want = os.environ.get("OPENSCAD_BACKEND", "Manifold")
    return {"manifold": "Manifold", "cgal": "CGAL"}.get(want.lower(), want)


def cmd(out=""):
    """The command to start an OpenSCAD render with: the binary and its backend option. A 2D
    export (out = *.dxf, *.svg) stays with the system openscad when there is one: the backend
    only does 3D, and the snapshots write DXF in a newer dialect (LWPOLYLINE, not LINE), so
    board.dxf keeps its bytes."""
    if out.endswith((".dxf", ".svg")) and not os.environ.get("OPENSCAD") and shutil.which("openscad"):
        return ["openscad"]
    b = backend()
    return [binary()] + ([f"--backend={b}"] if b else [])


@functools.lru_cache(None)
def version():
    r = subprocess.run([binary(), "--version"], capture_output=True, text=True)
    m = re.search(r"version (\S+)", r.stdout + r.stderr)
    return m.group(1) if m else "?"


def describe():
    """One line for logs and build stamps: which OpenSCAD, which version, which backend."""
    return f"{binary()} {version()} {backend() or 'CGAL'}"


def jobs(cpu_only=False):
    """How many renders to run at once: $REF_JOBS, else the cores less two, and no more than
    the free memory holds (a render of the tub takes about 0.5 GB with Manifold, 1 GB with
    CGAL; the budget doubles that). cpu_only: for small jobs (kicad-cli exports), cores only."""
    n = os.environ.get("REF_JOBS")
    if n:
        return max(1, int(n))
    n = max(1, (os.cpu_count() or 4) - 2)
    if cpu_only:
        return n
    try:
        avail = next(int(line.split()[1]) for line in open("/proc/meminfo") if line.startswith("MemAvailable"))
    except (OSError, StopIteration, ValueError):
        return n
    per = (1.0 if backend() == "Manifold" else 2.0) * 1024 * 1024        # kB
    return max(1, min(n, int(avail // per)))


if __name__ == "__main__":
    print(describe(), f"(jobs: {jobs()})")
