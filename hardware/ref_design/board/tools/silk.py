"""Silkscreen: legible references where they help, labels where people look.

    python3 tools/silk.py

- references of chip passives, diodes and LEDs are hidden (the assembly PDF
  on F.Fab carries them); ICs, connectors, switches, test pads keep theirs
- every visible reference is tried beside its part (above, below, left,
  right) until it clears pads, holes, the board edge and other references;
  one that finds no clear spot is hidden
- labels: board name and revision (back), test pad names, connector names\n  and the battery's + pin
Run after routing; it only touches silkscreen text.
"""
import os
import sys

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD_DIR = os.path.dirname(HERE)
sys.path.insert(0, BOARD_DIR)
from placement import CASE  # noqa: E402

PCB = os.path.join(BOARD_DIR, "battler.kicad_pcb")
SMALL = ("R_0402", "C_0402", "R_0603", "C_0603", "L_0603", "LED_0603", "D_SOD-323", "L_1008",
         "Fiducial", "MountingHole", "DSL_button", "AOTA", "ABM8")
TEXT_H = 0.8        # mm; JLC's minimum legible silk is about 0.8 mm tall, 0.15 mm line


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
    shown = hidden = 0
    for fp in sorted(board.GetFootprints(), key=lambda f: f.GetReference()):
        ref = fp.Reference()
        name = str(fp.GetFPID().GetLibItemName())
        side = pcbnew.B_SilkS if fp.IsFlipped() else pcbnew.F_SilkS
        if any(s in name for s in SMALL):
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

    # the board's name, revision and build on the back, over the cell
    back = pcbnew.B_SilkS
    text("TWO-PLAYER HANDHELD  REF-A", 100, 116.0, back, h=1.2)
    text("REV 2026-09-24", 100, 118.2, back)
    text("4 LAYERS  1.6 MM  ENIG", 100, 119.8, back)
    # test pads (back, above the speaker pads), one name under each
    for i, name in enumerate(["VBUS", "VSYS", "VBAT", "3V3", "GND"]):
        tp = board.FindFootprintByReference(f"TP{i + 1}")
        x, y = pcbnew.ToMM(tp.GetPosition().x), pcbnew.ToMM(tp.GetPosition().y)
        text(name, x, y + 1.6, back)
    j4 = board.FindFootprintByReference("J4")
    text("SPEAKER", pcbnew.ToMM(j4.GetPosition().x), pcbnew.ToMM(j4.GetPosition().y) + 2.7, back)
    # front: connectors and buttons people solder to or press during bring-up
    j2 = board.FindFootprintByReference("J2")
    p1 = next(p for p in j2.Pads() if p.GetNumber() == "1")
    b1 = box(p1)
    text("+", pcbnew.ToMM(p1.GetPosition().x), b1[3] + 0.9, h=1.0)
    jb = box(j2)
    text("BATT", jb[0] - 2.4, (jb[1] + jb[3]) / 2)
    text("SWD", CASE["swd"][0] + 2.9, CASE["swd"][1])
    text("UART", CASE["uart"][0] + 3.1, CASE["uart"][1])
    text("BOOT", CASE["pinholes"][0][0] + 3.0, CASE["pinholes"][0][1])
    text("RESET", CASE["pinholes"][1][0] + 3.2, CASE["pinholes"][1][1])
    if clashes:
        print("labels over pads or other text:", clashes)
    pcbnew.SaveBoard(PCB, board)
    print(f"references shown {shown}, hidden {hidden}")


if __name__ == "__main__":
    main()
