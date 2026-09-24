"""The landing for the display panel's solder FPC, into lib/battler.pretty.

    python3 tools/fp_panel.py

QD3525 (3.5in IPS, ST7796U): 48 contacts at 0.8 mm pitch, 0.40 mm wide, at the
tip of a 43.21 mm wide tail (panel maker's drawing, QD3525 Specifications rev A).
The tail folds under the panel, so its contacts face the board with contact 1 on
the left seen from above when the tail comes from +Y (the panel's FPC edge).

Pads are longer than the contacts (2.6 mm against about 1.7) for a hand or hot-bar
joint and a little placement slack; JLC's paste leaves them tinned. F.Fab shows the
tail up to the fold, whose distance from the pads is case.scad's fpc_front.
"""
import json
import os
import sys

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.join(os.path.dirname(HERE), "lib", "battler.pretty")
POS = json.load(open(os.path.join(os.path.dirname(os.path.dirname(HERE)), "case", "positions.json")))
NAME = "QD3525_FPC_1x48_P0.80mm_Solder"
N, PITCH = 48, 0.80
PAD_W, PAD_L = 0.45, 2.6
TAIL_W = 43.21
TIP = 1.0                 # FPC tip past the pad row centre (its contacts are 2.0 long from the tip)


def mm(v):
    return pcbnew.FromMM(v)


def V(x, y):
    return pcbnew.VECTOR2I(mm(x), mm(y))


def main():
    io = pcbnew.GetPluginForPath(LIB)
    fold = POS["fpc_tail"][1][1] - POS["fpc_conn"]["front_y"] - 1.5    # pad row to the panel edge
    fp = pcbnew.FOOTPRINT(None)
    fp.SetFPID(pcbnew.LIB_ID("battler", NAME))
    fp.SetReference("REF**")
    fp.SetValue(NAME)
    fp.SetAttributes(pcbnew.FP_SMD)
    fp.SetLibDescription(f"QD3525 3.5in IPS panel, solder FPC landing: {N} pads, {PITCH} mm pitch; the tail "
                         f"comes from +Y (folded under the panel), its tip at -Y; contact 1 at -X")
    fp.SetKeywords("FPC solder hot-bar LCD QD3525 ST7796")
    for n in range(1, N + 1):
        p = pcbnew.PAD(fp)
        p.SetNumber(str(n))
        p.SetAttribute(pcbnew.PAD_ATTRIB_SMD)
        p.SetShape(pcbnew.PAD_SHAPE_RECT)
        p.SetSize(V(PAD_W, PAD_L))
        p.SetLayerSet(pcbnew.LSET(p.SMDMask()))
        p.SetFPRelativePosition(V((n - (N + 1) / 2) * PITCH, 0))
        fp.Add(p)
    half = (N - 1) / 2 * PITCH + PAD_W / 2

    def seg(layer, a, b, w):
        s = pcbnew.PCB_SHAPE(fp, pcbnew.SHAPE_T_SEGMENT)
        s.SetStart(V(*a))
        s.SetEnd(V(*b))
        s.SetLayer(layer)
        s.SetWidth(mm(w))
        fp.Add(s)
        s.thisown = False

    def rect(layer, x0, y0, x1, y1, w):
        for a, b in (((x0, y0), (x1, y0)), ((x1, y0), (x1, y1)), ((x1, y1), (x0, y1)), ((x0, y1), (x0, y0))):
            seg(layer, a, b, w)
    # courtyard around the pads; the tail itself is flat and lies over the board
    rect(pcbnew.F_CrtYd, -half - 0.25, -PAD_L / 2 - 0.25, half + 0.25, PAD_L / 2 + 0.25, 0.05)
    # fab: the tail from its tip to the fold at the panel edge
    rect(pcbnew.F_Fab, -TAIL_W / 2, -TIP, TAIL_W / 2, fold, 0.1)
    seg(pcbnew.F_Fab, (-TAIL_W / 2 - 1.0, fold), (TAIL_W / 2 + 1.0, fold), 0.1)
    # silk: contact 1, and the tail's edges for lining it up by eye
    seg(pcbnew.F_SilkS, (-half, -PAD_L / 2 - 0.45), (-half + 0.8, -PAD_L / 2 - 0.45), 0.15)
    for sx in (-1, 1):
        x = sx * (TAIL_W / 2 + 0.2)
        seg(pcbnew.F_SilkS, (x, -TIP), (x, PAD_L / 2 + 0.6), 0.12)
    fp.Reference().SetPosition(V(-half - 1.8, 0))
    fp.Reference().SetLayer(pcbnew.F_SilkS)
    fp.Value().SetPosition(V(0, fold + 1.0))
    fp.Value().SetLayer(pcbnew.F_Fab)
    io.FootprintSave(LIB, fp)
    print(f"saved {NAME}: {N} pads, fold {fold:.2f} mm from the pad row")


if __name__ == "__main__":
    sys.exit(main())
