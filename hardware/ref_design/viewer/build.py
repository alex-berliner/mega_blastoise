#!/usr/bin/env python3
"""The assembly viewer: an exploded 3D view of the case, the board and the bought parts.

    python3 viewer/build.py           # regenerate site/parts from the board and the case
    python3 viewer/build.py --check   # exit 1 when site/parts is older than its inputs

site/ ships to GitHub Pages at /assembly/ (.github/workflows/pages.yml). GitHub's runners
have no KiCad 3D library or OpenSCAD, so the parts are built here and committed; the
pre-push hook (viewer/pre-push) rebuilds and commits them when main goes out stale.

Inputs are named by their committed blob ids, so the check reads git, not the disk. The
case's tub.stl and lid.stl (case/export.py --stl, which must run first) are build output, not
in git: case.scad stands for them.
"""
import datetime
import json
import os
import re
import shutil
import struct
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REF = os.path.dirname(HERE)
PARTS = os.path.join(HERE, "site", "parts")
MODEL = os.path.join(PARTS, "model.json")
INPUTS = ["board/battler.kicad_pcb", "case/case.scad", "viewer/pieces.scad", "viewer/build.py"]
PIECES = ["cell", "foam", "speaker", "panel", "silicone", "caps", "screws", "inserts"]


def git(*args):
    # without the pre-push hook's GIT_DIR, which in a linked worktree sends HEAD:./ paths astray
    env = {k: v for k, v in os.environ.items() if k not in ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_PREFIX")}
    return subprocess.run(["git", "-C", REF, *args], capture_output=True, text=True, env=env).stdout.strip()


def input_ids():
    return {p: git("rev-parse", f"HEAD:./{p}") for p in INPUTS}


def check():
    try:
        built = json.load(open(MODEL))["inputs"]
    except (OSError, ValueError, KeyError):
        print("assembly viewer: never built")
        return 1
    stale = [p for p, i in input_ids().items() if built.get(p) != i]
    if stale:
        print("assembly viewer: out of date with", ", ".join(stale))
        return 1
    print("assembly viewer: up to date")
    return 0


def read_stl(path):
    import numpy as np
    b = open(path, "rb").read()
    if b[:5] == b"solid" and b"facet" in b[:400]:
        v = np.array(re.findall(rb"vertex\s+(\S+)\s+(\S+)\s+(\S+)", b), dtype=np.float32)
        return v.reshape(-1, 3, 3)
    n = struct.unpack("<I", b[80:84])[0]
    rec = np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")])
    return np.frombuffer(b[84:84 + 50 * n], dtype=rec)["v"].copy()


def write_stl(v, path):
    import numpy as np
    n = np.cross(v[:, 1] - v[:, 0], v[:, 2] - v[:, 0])
    ln = np.linalg.norm(n, axis=1, keepdims=True)
    ln[ln == 0] = 1
    rec = np.zeros(len(v), dtype=np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")]))
    rec["n"], rec["v"] = n / ln, v
    with open(path, "wb") as f:
        f.write(b"\0" * 80 + struct.pack("<I", len(v)) + rec.tobytes())
    flat = v.reshape(-1, 3)
    return [round(float(flat[:, 2].min()), 2), round(float(flat[:, 2].max()), 2)]


def build():
    dirty = git("status", "--porcelain", "--", *INPUTS)
    if dirty:
        print("note: building from uncommitted inputs; the page will say so\n" + dirty)
    os.makedirs(PARTS, exist_ok=True)
    tmp = tempfile.mkdtemp()
    model = {"z": {}}

    # the board, with every fitted part's 3D model (KiCad frame centred on the case: 100, 100)
    subprocess.run(["kicad-cli", "pcb", "export", "glb", "-f", "-o", os.path.join(PARTS, "board.glb"),
                    "--user-origin", "100x100mm", "--include-silkscreen", "--include-soldermask",
                    "--no-dnp", os.path.join(REF, "board", "battler.kicad_pcb")],
                   check=True, capture_output=True)

    # the bought parts, rendered where case.scad puts them
    procs = {p: subprocess.Popen(["openscad", "-o", os.path.join(tmp, p + ".stl"), "-D", 'part="none"',
                                  "-D", f'piece="{p}"', os.path.join(HERE, "pieces.scad")],
                                 stderr=subprocess.PIPE, text=True) for p in PIECES}
    for p, proc in procs.items():
        err = proc.communicate()[1]
        if proc.returncode:
            sys.exit(f"openscad failed on {p}:\n{err}")
        m = re.search(r'ECHO: "VIEWER (\S+) (\S+)"', err)
        model["case_h"], model["z_board"] = float(m.group(1)), float(m.group(2))
        model["z"][p] = write_stl(read_stl(os.path.join(tmp, p + ".stl")), os.path.join(PARTS, p + ".stl"))

    # the printed shell: the tub as exported, the lid turned back over from its print pose
    tub = read_stl(os.path.join(REF, "case", "tub.stl"))
    model["z"]["tub"] = write_stl(tub, os.path.join(PARTS, "tub.stl"))
    lid = read_stl(os.path.join(REF, "case", "lid.stl"))
    lid[:, :, 1] *= -1
    lid[:, :, 2] = model["case_h"] - lid[:, :, 2]
    model["z"]["lid"] = write_stl(lid, os.path.join(PARTS, "lid.stl"))
    model["z"]["board"] = [model["z_board"], round(model["z_board"] + 1.6, 2)]
    flat = tub.reshape(-1, 3)
    model["size"] = [round(float(flat[:, i].max() - flat[:, i].min()), 1) for i in (0, 1)]
    shutil.rmtree(tmp)

    rev = re.search(r"^BOARD_REV = (\d+)", open(os.path.join(REF, "board", "design.py")).read(), re.M)
    model.update(rev=int(rev.group(1)), commit=git("rev-parse", "--short", "HEAD"), dirty=bool(dirty),
                 built=datetime.date.today().isoformat(), inputs=input_ids())
    json.dump(model, open(MODEL, "w"), indent=1)
    size = sum(os.path.getsize(os.path.join(PARTS, f)) for f in os.listdir(PARTS))
    print(f"assembly viewer: built from {model['commit']}{' (dirty)' if dirty else ''}, {size / 1e6:.1f} MB")


if __name__ == "__main__":
    sys.exit(check() if "--check" in sys.argv else build())
