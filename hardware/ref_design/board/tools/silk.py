"""Silkscreen: legible references where they help, labels where people look.

    python3 tools/silk.py

- references of chip passives, diodes and LEDs are hidden (the assembly PDF
  on F.Fab carries them); ICs, connectors, switches, test pads keep theirs
- every visible reference is tried beside its part (above, below, left,
  right) until it clears pads, holes, the board edge and other references;
  one that finds no clear spot is hidden
- labels: board name and revision (back), test pad names, connector names
  and the battery's + pin
Run after routing; it only touches silkscreen text, and can be run again.
"""
import os
import sys

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD_DIR = os.path.dirname(HERE)
sys.path.insert(0, BOARD_DIR)
from placement import CASE, CUTOUTS, PINHOLES, SPEAKER  # noqa: E402

PCB = os.path.join(BOARD_DIR, "battler.kicad_pcb")
SMALL = ("R_0402", "C_0402", "R_0603", "C_0603", "L_0603", "LED_0603", "D_SOD-323", "D_SOD-123", "C_0805", "L_1008",
         "Fiducial", "MountingHole", "DSL_button", "AOTA", "ABM8")
TEXT_H = 0.8        # mm; JLC's minimum legible silk is about 0.8 mm tall, 0.15 mm line
# parts whose label below says more than their reference would
LABELLED = {"J2", "J3", "J5", "J7", "SW18", "SW19", "TP1", "TP2", "TP3", "TP4", "TP5",
            "TP9", "TP10", "TP11", "TP12", "TP13", "TP14", "TP15", "TP16"}
# the probe pads added for bring-up (2026-10-19): the name printed beside each
PROBE_NAMES = {"TP9": "1V1", "TP10": "BLREF", "TP11": "SCK", "TP12": "MOSI", "TP13": "CS", "TP14": "DC",
               "TP15": "GND", "TP16": "GND"}


def mm(v):
    return pcbnew.FromMM(v)


def V(x, y):
    return pcbnew.VECTOR2I(mm(x), mm(y))


def box(item, grow=0.0):
    b = item.GetBoundingBox()
    return (pcbnew.ToMM(b.GetX()) - grow, pcbnew.ToMM(b.GetY()) - grow,
            pcbnew.ToMM(b.GetRight()) + grow, pcbnew.ToMM(b.GetBottom()) + grow)


def overlap(a, b):
    return not (a[2] <= b[0] or b[2] <= a[0] or a[3] <= b[1] or b[3] <= a[1])


def main():
    board = pcbnew.LoadBoard(PCB)
    # labels from an earlier run go first, so running this again changes nothing
    old = [d for d in board.GetDrawings() if isinstance(d, pcbnew.PCB_TEXT)
           and d.GetLayer() in (pcbnew.F_SilkS, pcbnew.B_SilkS)]
    old += [d for d in board.GetDrawings() if isinstance(d, pcbnew.PCB_SHAPE) and d.GetLayer() == pcbnew.B_SilkS]
    for d in old:
        board.Remove(d)
    bb = board.GetBoardEdgesBoundingBox()
    inner = (pcbnew.ToMM(bb.GetX()) + 0.6, pcbnew.ToMM(bb.GetY()) + 0.6,
             pcbnew.ToMM(bb.GetRight()) - 0.6, pcbnew.ToMM(bb.GetBottom()) - 0.6)
    obstacles = {pcbnew.F_SilkS: [], pcbnew.B_SilkS: []}
    for fp in board.GetFootprints():
        side = pcbnew.B_SilkS if fp.IsFlipped() else pcbnew.F_SilkS
        for p in fp.Pads():
            obstacles[side].append(box(p, 0.15))
            if p.GetAttribute() in (pcbnew.PAD_ATTRIB_PTH, pcbnew.PAD_ATTRIB_NPTH):
                obstacles[pcbnew.B_SilkS if side == pcbnew.F_SilkS else pcbnew.F_SilkS].append(box(p, 0.15))
    for t in board.GetTracks():
        if t.GetClass() == "PCB_VIA":
            obstacles[pcbnew.F_SilkS].append(box(t, 0.1))
            obstacles[pcbnew.B_SilkS].append(box(t, 0.1))
    # the cell's and the speaker's holes: no text within 0.6 of them
    for c in CUTOUTS:
        x0, y0, x1, y1 = c["box"]
        hole = (x0 - 0.6, y0 - 0.6, x1 + 0.6, y1 + 0.6)
        obstacles[pcbnew.F_SilkS].append(hole)
        obstacles[pcbnew.B_SilkS].append(hole)
    shown = hidden = 0
    for fp in sorted(board.GetFootprints(), key=lambda f: f.GetReference()):
        ref = fp.Reference()
        name = str(fp.GetFPID().GetLibItemName())
        side = pcbnew.B_SilkS if fp.IsFlipped() else pcbnew.F_SilkS
        if any(s in name for s in SMALL) or fp.GetReference() in LABELLED:
            ref.SetVisible(False)
            hidden += 1
            continue
        ref.SetTextSize(V(TEXT_H, TEXT_H))
        ref.SetTextThickness(mm(0.15))
        ref.SetTextAngleDegrees(0)
        cy = fp.GetBoundingBox(False)
        cx, cyc = pcbnew.ToMM(cy.GetCenter().x), pcbnew.ToMM(cy.GetCenter().y)
        w, h = pcbnew.ToMM(cy.GetWidth()), pcbnew.ToMM(cy.GetHeight())
        spots = [(cx, cyc - h / 2 - 0.8), (cx, cyc + h / 2 + 0.8), (cx - w / 2 - 1.6, cyc), (cx + w / 2 + 1.6, cyc),
                 (cx, cyc - h / 2 - 1.8), (cx, cyc + h / 2 + 1.8)]
        placed = False
        for x, y in spots:
            ref.SetPosition(V(x, y))
            b = box(ref, 0.05)
            if not (inner[0] <= b[0] and b[2] <= inner[2] and inner[1] <= b[1] and b[3] <= inner[3]):
                continue
            if any(overlap(b, o) for o in obstacles[side]):
                continue
            obstacles[side].append(b)
            placed = True
            break
        ref.SetVisible(placed)
        shown += placed
        hidden += not placed
    # labels
    clashes = []

    def text(s, x, y, layer=pcbnew.F_SilkS, h=0.8):
        t = pcbnew.PCB_TEXT(board)
        t.SetText(s)
        t.SetPosition(V(x, y))
        t.SetLayer(layer)
        t.SetTextSize(V(h, h))
        t.SetTextThickness(mm(0.15 if h < 1.2 else 0.2))
        t.SetMirrored(layer == pcbnew.B_SilkS)
        board.Add(t)
        b = box(t, 0.05)
        if any(overlap(b, o) for o in obstacles[layer]):
            clashes.append(s)
        obstacles[layer].append(b)

    # the board's name, revision (design.BOARD_REV)
    # and build, on the back between seat 2's clusters (the core took the middle of the back)
    import design
    back = pcbnew.B_SilkS
    # (since 2026-10-27 the cell's hole runs the whole middle: the name goes on the back at seat 2's end, on the first spot where
    # all three lines clear every pad, via and hole)
    # each line takes the clear spot nearest (100, 40) on the back, anywhere on the board, outside every hole
    grid = sorted(((x, y) for x in [66 + 1.0 * i for i in range(69)] for y in [26 + 0.5 * k for k in range(250)]),
                  key=lambda t: (t[0] - 100.0) ** 2 + (t[1] - 40.0) ** 2)
    for s_, h, w in (("TWO-PLAYER HANDHELD  REF-A", 1.2, 16.5), (f"REV {design.BOARD_REV}", 2.0, 6.5), ("2 LAYERS  1.6 MM  ENIG", 0.8, 10.0)):
        for x, y in grid:
            bx = (x - w, y - h / 2 - 0.1, x + w, y + h / 2 + 0.1)
            if inner[0] <= bx[0] and bx[2] <= inner[2] and inner[1] <= bx[1] and bx[3] <= inner[3] and \
                    not any(overlap(bx, o) for o in obstacles[back]):
                text(s_, x, y, back, h=h)
                obstacles[back].append(bx)
                break
        else:
            print("no clear spot on the back for", s_)
    # test pads (back, between seat 2's clusters), one name under each
    for i, name in enumerate(["VBUS", "VSYS", "VBAT", "3V3", "GND"]):
        tp = board.FindFootprintByReference(f"TP{i + 1}")
        x, y = pcbnew.ToMM(tp.GetPosition().x), pcbnew.ToMM(tp.GetPosition().y)
        text(name, x, y + 1.6, back)
    for ref, name in PROBE_NAMES.items():
        tp = board.FindFootprintByReference(ref)
        layer = back if tp.IsFlipped() else pcbnew.F_SilkS
        x, y = pcbnew.ToMM(tp.GetPosition().x), pcbnew.ToMM(tp.GetPosition().y)
        text(name, x, y + 1.5 if name not in ("1V1", "BLREF") else y - 1.5, layer)
    j7 = board.FindFootprintByReference("J7")
    jb = box(j7)
    text("SWD PADS", (jb[0] + jb[2]) / 2, jb[1] - 0.9, back)
    # front: connectors and buttons people solder to or press during bring-up
    j2 = board.FindFootprintByReference("J2")
    p1 = next(p for p in j2.Pads() if p.GetNumber() == "1")
    b1 = box(p1)
    text("+", pcbnew.ToMM(p1.GetPosition().x), b1[3] + 0.9, h=1.0)
    jb = box(j2)
    text("BATT", (jb[0] + jb[2]) / 2, jb[3] + 0.9)       # below it: its mouth faces the cell's hole
    j3 = board.FindFootprintByReference("J3")
    row = [xy for xy in ((pcbnew.ToMM(p.GetPosition().x), pcbnew.ToMM(p.GetPosition().y)) for p in j3.Pads())]
    text("PANEL", min(x for x, _ in row) - 5.2, row[0][1])
    for ref, name in (("J5", "SWD"),):
        jb = box(board.FindFootprintByReference(ref))
        text(name, (jb[0] + jb[2]) / 2 - 0.6, (jb[1] + jb[3]) / 2)     # on its body: right of it is the hole
    # the speaker, hand-fitted on the top (build_pcb.py draws its outline): its name inside it
    # (the speaker's hole takes no text since 2026-10-27: nothing is printed in a hole)
    # the pinholes' names (placement.PINHOLES is in the case frame, y up: KiCad's x + 100, 100 - y)
    px = [(100 + x, 100 - y) for x, y in PINHOLES]
    text("BOOT", px[0][0] + 0.0, px[0][1] - 2.3, h=0.8)   # above the upper one, below the lower one
    text("RST", px[1][0] - 1.0, px[1][1] + 2.3, h=0.8)
    if clashes:
        print("labels over pads or other text:", clashes)
    pcbnew.SaveBoard(PCB, board)
    print(f"references shown {shown}, hidden {hidden}")


if __name__ == "__main__":
    main()
