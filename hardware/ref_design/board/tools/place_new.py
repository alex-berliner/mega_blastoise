"""Nudge the ESP32-S3 build's new parts to the nearest legal spot (no courtyard overlap, inside the board 0.4 from its edge and the cutouts,
out of the antenna's keepout, inside the part's region), writing board/placement_new.json, which placement.py applies on top of its table.

    python3 tools/build_pcb.py && python3 tools/place_new.py && python3 tools/build_pcb.py

Greedy, in the order of ORDER; each part tries its preferred pose and then the others of its turns, from the nearest grid point out.
"""
import json
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD = os.path.dirname(HERE)
sys.path.insert(0, BOARD)
import pcbnew  # noqa: E402
import placement as pl  # noqa: E402

mm = pcbnew.ToMM
STEP = 0.1
RADIUS = 9.0
EDGE = 0.36
REGIONS = {   # name: (x0, y0, x1, y1) KiCad; a part's box must lie inside its region
    "left_strip": (62.3, 109.0, 64.3, 126.7),
    "above_module": (72.0, 99.0, 81.0, 107.6),
    "channel": (81.5, 107.3, 117.0, 113.5),
    "fpc": (78.1, 127.2, 126.9, 142.8),            # under the panel's FPC: 1.05 mm
    "spine": (88.0, 143.6, 107.5, 150.5),
    "any_low": (64.0, 108.0, 135.0, 175.0),
}
ORDER = [   # ref, region, preferred (x, y, rot) from placement.P unless given, turns
    ("C11", "left_strip", (0, 90)), ("C12", "left_strip", (0, 90)), ("R23", "left_strip", (0, 90)),
    ("C10", "above_module", (0, 90)),
    *[(f"R{70 + k}", "above_module", (90, 0)) for k in range(7)],
    ("R77", "channel", (0, 90)), ("R78", "channel", (0, 90)), ("R65", "channel", (0, 90)),
    ("R21", "fpc", (90, 0)), ("R22", "fpc", (90, 0)), ("C13", "fpc", (90, 0)), ("C14", "fpc", (90, 0)),
    *[(f"R{79 + k}", "fpc", (90, 0)) for k in range(6)],
    ("JP6", "fpc", (0, 90)), ("C60", "fpc", (0, 90)), ("C61", "fpc", (0, 90)),
    ("U11", "spine", (0, 90)), ("C62", "spine", (0, 90)), ("R86", "spine", (90, 0)), ("R87", "spine", (90, 0)),
]


def box_of(fp):
    lay = pcbnew.B_CrtYd if fp.IsFlipped() else pcbnew.F_CrtYd
    bb = fp.GetCourtyard(lay).BBox()
    if bb.GetWidth() == 0:
        bb = fp.GetBoundingBox(False)
    # pads too (the courtyard can be inside the copper)
    pb = fp.GetBoundingBox(False)
    x0, y0 = min(mm(bb.GetX()), mm(pb.GetX())), min(mm(bb.GetY()), mm(pb.GetY()))
    x1, y1 = max(mm(bb.GetRight()), mm(pb.GetRight())), max(mm(bb.GetBottom()), mm(pb.GetBottom()))
    return (x0, y0, x1, y1)


def hit(a, b, m=0.0):
    return a[2] + m > b[0] and b[2] + m > a[0] and a[3] + m > b[1] and b[3] + m > a[1]


def main():
    b = pcbnew.LoadBoard(os.path.join(BOARD, "battler.kicad_pcb"))
    fps = {f.GetReference(): f for f in b.GetFootprints()}
    boxes = {r: box_of(f) for r, f in fps.items() if not f.IsFlipped()}
    movable = {o[0] for o in ORDER}
    others = {r: bx for r, bx in boxes.items() if r not in movable}
    keep = [pl.ANTENNA_KEEPOUT] + [(c["box"][0] - EDGE, c["box"][1] - EDGE, c["box"][2] + EDGE, c["box"][3] + EDGE) for c in pl.CUTOUTS]
    sp = pl.SPEAKER
    keep.append((sp["at"][0] - sp["size"][0] / 2 - 0.5, sp["at"][1] - sp["size"][1] / 2 - 0.5, sp["at"][0] + sp["size"][0] / 2 + 0.5, sp["at"][1] + sp["size"][1] / 2 + 0.5))
    x0, x1, y0, y1 = pl.BOARD_X0 + EDGE, pl.BOARD_X1 - EDGE, 100 - pl.CASE["board"]["l"] / 2 + EDGE, 100 + pl.CASE["board"]["l"] / 2 - EDGE
    placed, fails = {}, []
    for ref, region, turns in ORDER:
        fp = fps[ref]
        pref = pl.P[ref]
        rel = {}
        for rot in turns:                 # box relative to the origin, per turn
            fp.SetPosition(pcbnew.VECTOR2I(0, 0)); fp.SetOrientationDegrees(rot)
            rel[rot] = box_of(fp)
        best = None
        rg = REGIONS[region]
        n = int(RADIUS / STEP)
        cands = []
        for rot in turns:
            for i in range(-n, n + 1):
                for j in range(-n, n + 1):
                    d = math.hypot(i, j) * STEP + (0.0 if rot == pref[2] else 0.4)
                    if d <= RADIUS:
                        cands.append((d, pref[0] + i * STEP, pref[1] + j * STEP, rot))
        cands.sort()
        for d, x, y, rot in cands:
            r = rel[rot]
            bx = (x + r[0], y + r[1], x + r[2], y + r[3])
            if not (rg[0] <= bx[0] and bx[2] <= rg[2] and rg[1] <= bx[1] and bx[3] <= rg[3]):
                continue
            if not (x0 <= bx[0] and bx[2] <= x1 and y0 <= bx[1] and bx[3] <= y1):
                continue
            if any(hit(bx, k) for k in keep):
                continue
            if any(hit(bx, o, 0.0) for o in others.values()):
                continue
            best = (x, y, rot, bx)
            break
        if best is None:
            fails.append(ref)
            continue
        placed[ref] = [round(best[0], 3), round(best[1], 3), best[2], "F"]
        others[ref] = best[3]
    json.dump(placed, open(os.path.join(BOARD, "placement_new.json"), "w"), indent=1)
    print("placed", len(placed), "of", len(ORDER), "failed:", fails)


if __name__ == "__main__":
    main()
