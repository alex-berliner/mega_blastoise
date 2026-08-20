#!/usr/bin/env python3
"""Draw the stripboard into the KiCad board file, using KiCad's own pcbnew API.

Everything it draws comes from layout.py, so the board can never drift from
BUILD.md or the SVG. Run it again after editing layout.py and the board is
rebuilt from scratch.

    python3 build_board.py

What lands on which layer:
    B.Cu          the copper strips, already split at every cut
    F.Cu          the jumper wires, which really do run on the component side
    F.Silkscreen  part outlines and references, plus an X at every cut
    User.Drawings the display module and the LiPo cell footprints
"""

import importlib.util
import os
import sys

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD = os.path.join(HERE, "mega_blastoise_stripboard.kicad_pcb")

spec = importlib.util.spec_from_file_location("layout", os.path.join(HERE, "layout.py"))
L = importlib.util.module_from_spec(spec)
spec.loader.exec_module(L)

FP_DIRS = {
    "Module": "/usr/share/kicad/footprints/Module.pretty",
    "Connector_PinHeader_2.54mm": "/usr/share/kicad/footprints/Connector_PinHeader_2.54mm.pretty",
    "Diode_THT": "/usr/share/kicad/footprints/Diode_THT.pretty",
    "Resistor_THT": "/usr/share/kicad/footprints/Resistor_THT.pretty",
    "Capacitor_THT": "/usr/share/kicad/footprints/Capacitor_THT.pretty",
    "mb_stripboard": os.path.join(HERE, "footprints.pretty"),
}

STRIP_WIDTH = 1.9      # real 0.1 inch stripboard copper is about 2 mm wide
JUMPER_WIDTH = 0.6


def mm(v):
    return pcbnew.FromMM(float(v))


def pt(x, y):
    return pcbnew.VECTOR2I(mm(x), mm(y))


def hole_pt(col, row):
    return pt(*L.page(col, row))


# Where each footprint's pads must land comes from the layout itself.
WANT = L.expected_pads()


def pad_error(fp, ref):
    """Total mm between where this footprint's pads are and where they belong."""
    want = WANT.get(ref)
    if not want:
        return 0.0
    got = {}
    for pad in fp.Pads():
        got.setdefault(pad.GetNumber(), []).append(pad.GetPosition())
    total = 0.0
    for number, holes in want.items():
        here = sorted(got.get(number, []), key=lambda p: (p.x, p.y))
        there = sorted((hole_pt(c, r) for c, r in holes), key=lambda p: (p.x, p.y))
        if len(here) != len(there):
            return 1e9
        for h, t in zip(here, there):
            total += pcbnew.ToMM(int(((h.x - t.x) ** 2 + (h.y - t.y) ** 2) ** 0.5))
    return total


def new_board():
    """Start from an empty board every run, so the file is purely generated.

    This build's SWIG wrapper cannot iterate an existing board's tracks, and
    regenerating is cleaner than editing in place anyway.
    """
    board = pcbnew.NewBoard(BOARD)
    ox, oy = L.ORIGIN_X, L.ORIGIN_Y
    rect(board, ox, oy, L.BOARD_W, L.BOARD_H, pcbnew.Edge_Cuts, 0.1)
    return board


def place(board):
    problems = []
    for ref, fpid, col, row, rot, note in L.PLACEMENTS:
        lib, name = fpid.split(":")
        best = None
        for candidate in (rot, 0, 90, 180, 270):
            fp = pcbnew.FootprintLoad(FP_DIRS[lib], name)
            if fp is None:
                raise SystemExit(f"cannot load {fpid}")
            fp.SetReference(ref)
            fp.SetPosition(hole_pt(col, row))
            fp.SetOrientationDegrees(candidate)
            err = pad_error(fp, ref)
            if best is None or err < best[0]:
                best = (err, candidate, fp)
            if err < 0.05:
                break
        err, angle, fp = best
        # the origin pad is not always pad 1, so nudge the whole part onto grid
        # 0.1 mm covers the 5.00 vs 5.08 and 7.60 vs 7.62 pitch parts,
        # whose leads bend onto the grid without complaint
        if err > 0.1:
            problems.append((ref, round(err, 3), angle))
        fp.Value().SetVisible(False)
        board.Add(fp)
    return problems


def strips(board):
    layer = pcbnew.B_Cu
    n = 0
    for col, r0, r1 in L.strip_segments():
        n += 1
        t = pcbnew.PCB_TRACK(board)
        t.SetStart(hole_pt(col, r0))
        t.SetEnd(hole_pt(col, r1))
        t.SetWidth(mm(STRIP_WIDTH))
        t.SetLayer(layer)
        board.Add(t)
    return n


def jumpers(board):
    n = 0
    for name, pts in L.JUMPERS:
        n += 1
        for a, b in zip(pts, pts[1:]):
            t = pcbnew.PCB_TRACK(board)
            t.SetStart(hole_pt(*a))
            t.SetEnd(hole_pt(*b))
            t.SetWidth(mm(JUMPER_WIDTH))
            t.SetLayer(pcbnew.F_Cu)
            board.Add(t)
    return n


def line(board, x1, y1, x2, y2, layer, width=0.15):
    s = pcbnew.PCB_SHAPE(board)
    s.SetShape(pcbnew.SHAPE_T_SEGMENT)
    s.SetStart(pt(x1, y1))
    s.SetEnd(pt(x2, y2))
    s.SetLayer(layer)
    s.SetWidth(mm(width))
    board.Add(s)


def rect(board, x, y, w, h, layer, width=0.2):
    line(board, x, y, x + w, y, layer, width)
    line(board, x + w, y, x + w, y + h, layer, width)
    line(board, x + w, y + h, x, y + h, layer, width)
    line(board, x, y + h, x, y, layer, width)


def text(board, s, x, y, layer, size=2.0, rot=0):
    t = pcbnew.PCB_TEXT(board)
    t.SetText(s)
    t.SetPosition(pt(x, y))
    t.SetLayer(layer)
    t.SetTextSize(pcbnew.VECTOR2I(mm(size), mm(size)))
    t.SetTextThickness(mm(size / 7.0))
    t.SetTextAngleDegrees(rot)
    board.Add(t)


def cut_marks(board):
    d = 1.0
    for col, row in L.CUTS:
        x, y = L.page(col, row)
        line(board, x - d, y - d, x + d, y + d, pcbnew.F_SilkS, 0.25)
        line(board, x - d, y + d, x + d, y - d, pcbnew.F_SilkS, 0.25)


def annotations(board):
    ox, oy = L.ORIGIN_X, L.ORIGIN_Y
    dx, dy, dw, dh = L.display_outline()
    rect(board, ox + dx, oy + dy, dw, dh, pcbnew.Dwgs_User, 0.3)
    text(board, "ST7796 3.5in module, 10 mm standoffs",
         ox + dx + 2, oy + dy + 4, pcbnew.Dwgs_User, 2.0)
    bx, by, bw, bh = L.battery_outline()
    rect(board, ox + bx, oy + by, bw, bh, pcbnew.Dwgs_User, 0.3)
    text(board, "LiPo 103450 on the BACK, foam pad over the joints",
         ox + bx + 2, oy + by + bh / 2, pcbnew.Dwgs_User, 2.0)

    text(board, "PLAYER 2", ox + L.BOARD_W / 2, oy + 3.0, pcbnew.F_SilkS, 3.0)
    text(board, "PLAYER 1", ox + L.BOARD_W / 2, oy + L.BOARD_H - 3.0, pcbnew.F_SilkS, 3.0)
    text(board, "mega blastoise - single screen stripboard (single 24x56)",
         ox + 4, oy + L.BOARD_H + 6, pcbnew.Cmts_User, 2.5)
    text(board, "B.Cu = copper strips (already split at each cut). "
                "F.Cu = jumper wires on the component side. "
                "X on F.Silkscreen = cut this hole.",
         ox + 4, oy + L.BOARD_H + 11, pcbnew.Cmts_User, 2.0)


def main():
    board = new_board()
    problems = place(board)
    n_strips = strips(board)
    n_jumpers = jumpers(board)
    cut_marks(board)
    annotations(board)
    pcbnew.SaveBoard(BOARD, board)

    print(f"footprints:     {len(L.PLACEMENTS)}")
    print(f"strip segments: {n_strips}")
    print(f"jumper runs:    {n_jumpers}")
    print(f"cuts:           {len(L.CUTS)}")
    if problems:
        print("PADS OFF GRID:")
        for ref, err, angle in problems:
            print(f"  {ref}: {err} mm at {angle} deg")
        return 1
    print("every pad landed on its hole")
    return 0


if __name__ == "__main__":
    sys.exit(main())
