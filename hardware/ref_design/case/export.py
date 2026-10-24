"""Regenerate everything derived from case.scad.

    python3 export.py          # positions.json, board.dxf
    python3 export.py --stl    # also tub.stl, top.stl (the island) and deck.stl (one deck, printed twice),
                               # each in its print pose

With --stl it also renders seat 2's deck as case.scad builds it (part="deck_seat2", turned round into
deck.stl's frame) and stops unless it is deck.stl to within DECK_TOL mm3 each way: the two decks are one
part (2026-10-24), and the kits carry deck.stl twice.

positions.json is what the board scripts read: button centres, screw holes,
connector positions and height limits, all in KiCad coordinates. It comes from
the JSON line in case.scad's echo block, so the case stays the only source.
"""
import json
import os
import re
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
SCAD = os.path.join(HERE, "case.scad")
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "tools"))
import scad  # noqa: E402  (which OpenSCAD, which backend, how many at once)


def openscad(out, part, extra=()):
    r = subprocess.run([*scad.cmd(out), "-o", out, "-D", f'part="{part}"', *extra, SCAD],
                       capture_output=True, text=True)
    if r.returncode or "ERROR" in r.stderr or "WARNING: Assert" in r.stderr:
        sys.exit(f"openscad failed for {part}:\n{r.stderr[-2000:]}")
    return r.stderr


DECK_TOL = 0.05                     # mm3 of seat 2's deck outside seat 1's (or the other way): Manifold's slivers


def deck_mismatch(a, b, tmp):
    """The volume (mm3) of a less b plus b less a, two STLs in one frame."""
    import trimesh
    total = 0.0
    for x, y in ((a, b), (b, a)):
        scad_path, out = os.path.join(tmp, "diff.scad"), os.path.join(tmp, "diff.stl")
        open(scad_path, "w").write(f'difference() {{ import("{x}"); import("{y}"); }}\n')
        r = subprocess.run([*scad.cmd(out), "-o", out, scad_path], capture_output=True, text=True)
        if r.returncode == 0 and os.path.exists(out) and os.path.getsize(out):
            m = trimesh.load(out, force="mesh")
            total += abs(m.volume) if len(m.faces) else 0.0
            os.remove(out)
    return total


def main():
    with tempfile.TemporaryDirectory() as tmp:
        echo = os.path.join(tmp, "case.echo")
        openscad(echo, "none")
        text = open(echo).read()
    m = re.search(r'ECHO: "JSON (\{.*\})"', text)
    if not m:
        sys.exit("no JSON line in the echo block")
    data = json.loads(m.group(1).replace('\\"', '"'))
    with open(os.path.join(HERE, "positions.json"), "w") as f:
        json.dump(data, f, indent=1)
    print("positions.json:", len(data["buttons"]), "buttons,", len(data["pins"]), "screw holes")
    pool, renders = ThreadPoolExecutor(scad.jobs()), []    # the renders run at once, reported in order
    for part in ("board",):
        renders.append((f"{part}.dxf", pool.submit(openscad, os.path.join(HERE, f"{part}.dxf"), part)))
    if "--stl" in sys.argv:
        for part in ("tub", "top", "deck"):
            renders.append((f"{part}.stl", pool.submit(openscad, os.path.join(HERE, f"{part}.stl"), part)))
    with tempfile.TemporaryDirectory() as tmp:
        seat2 = os.path.join(tmp, "deck_seat2.stl")
        if "--stl" in sys.argv:
            renders.append(("(seat 2's deck)", pool.submit(openscad, seat2, "deck_seat2")))
        for name, render in renders:
            render.result()
            print(name)
        if "--stl" in sys.argv:
            v = deck_mismatch(os.path.join(HERE, "deck.stl"), seat2, tmp)
            if v > DECK_TOL:
                sys.exit(f"seat 2's deck isn't deck.stl turned round: {v:.3f} mm3 differ")
            print(f"seat 2's deck is deck.stl turned round ({v:.3f} mm3 differ)")


if __name__ == "__main__":
    main()
