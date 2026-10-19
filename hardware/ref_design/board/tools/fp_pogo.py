"""The debug pad row (J7): six bare round pads at 2.54 mm for pogo pins, a clip or soldered
wires, each named on the silkscreen. Written into lib/battler.pretty.

    python3 tools/fp_pogo.py

Placed on the board's back (flipped), so the names are drawn on F.SilkS here and land on
B.SilkS, mirrored to read correctly from below.
"""
import os

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.join(os.path.dirname(HERE), "lib", "battler.pretty")
NAME = "Pogo_Pads_1x06_P2.54mm"
PITCH, PAD_D = 2.54, 1.6
LABELS = ["3V3", "CLK", "DIO", "RUN", "BOOT", "GND"]


def mm(v):
    return pcbnew.FromMM(v)


def text(fp, s, x, y, size, layer):
    t = pcbnew.PCB_TEXT(fp)
    t.SetText(s)
    t.SetPosition(pcbnew.VECTOR2I(mm(x), mm(y)))
    t.SetLayer(layer)
    t.SetTextSize(pcbnew.VECTOR2I(mm(size), mm(size)))
    t.SetTextThickness(mm(size * 0.15))
    fp.Add(t)


def line(fp, a, b, layer, w):
    s = pcbnew.PCB_SHAPE(fp, pcbnew.SHAPE_T_SEGMENT)
    s.SetStart(pcbnew.VECTOR2I(mm(a[0]), mm(a[1])))
    s.SetEnd(pcbnew.VECTOR2I(mm(b[0]), mm(b[1])))
    s.SetLayer(layer)
    s.SetWidth(mm(w))
    fp.Add(s)


def main():
    io = pcbnew.GetPluginForPath(LIB)
    fp = pcbnew.FOOTPRINT(None)
    fp.SetFPID(pcbnew.LIB_ID("battler", NAME))
    fp.SetLibDescription("SWD debug pads: 3V3, SWCLK, SWDIO, RUN, BOOTSEL, GND; 1.6 mm round pads at 2.54 mm "
                         "for P75/P100 pogo pins, a clip or soldered wires (no part fitted)")
    fp.SetKeywords("SWD debug pogo pads")
    fp.SetAttributes(pcbnew.FP_SMD | pcbnew.FP_EXCLUDE_FROM_BOM | pcbnew.FP_EXCLUDE_FROM_POS_FILES)
    fp.Reference().SetPosition(pcbnew.VECTOR2I(0, mm(-2.4)))
    fp.Reference().SetLayer(pcbnew.F_Fab)
    fp.Value().SetPosition(pcbnew.VECTOR2I(0, mm(3.6)))
    fp.Value().SetLayer(pcbnew.F_Fab)
    x0 = -PITCH * (len(LABELS) - 1) / 2
    lset = pcbnew.LSET()
    lset.AddLayer(pcbnew.F_Cu)
    lset.AddLayer(pcbnew.F_Mask)
    for i, label in enumerate(LABELS):
        pad = pcbnew.PAD(fp)
        pad.SetNumber(str(i + 1))
        pad.SetAttribute(pcbnew.PAD_ATTRIB_SMD)
        pad.SetShape(pcbnew.PAD_SHAPE_RECTANGLE if i == 0 else pcbnew.PAD_SHAPE_CIRCLE)   # pin 1 square
        pad.SetSize(pcbnew.VECTOR2I(mm(PAD_D), mm(PAD_D)))
        pad.SetLayerSet(lset)
        pad.SetPosition(pcbnew.VECTOR2I(mm(x0 + i * PITCH), 0))
        fp.Add(pad)
        text(fp, label, x0 + i * PITCH, 1.9, 0.7, pcbnew.F_SilkS)
    w = PITCH * (len(LABELS) - 1) / 2 + PAD_D / 2
    for a, b in (((-w - 0.25, -1.25), (w + 0.25, -1.25)), ((w + 0.25, -1.25), (w + 0.25, 2.6)),
                 ((w + 0.25, 2.6), (-w - 0.25, 2.6)), ((-w - 0.25, 2.6), (-w - 0.25, -1.25))):
        line(fp, a, b, pcbnew.F_CrtYd, 0.05)
    line(fp, (-w - 0.15, -1.15), (-w + 0.9, -1.15), pcbnew.F_SilkS, 0.15)     # pin 1 corner mark
    line(fp, (-w - 0.15, -1.15), (-w - 0.15, -0.1), pcbnew.F_SilkS, 0.15)
    io.FootprintSave(LIB, fp)
    print("saved", NAME)


if __name__ == "__main__":
    main()
