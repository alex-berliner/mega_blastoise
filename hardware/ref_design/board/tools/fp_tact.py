"""battler:Tact_ROCPU_TPS850: ROCPU's TPS 850 series, top actuated, 3.5 x 2.75 x 1.4 mm (datasheets/rocpu_tps850.pdf, 'P.C.B LAND DIMENSION'):
four corner pads 0.4 wide, 3.65 over all and 2.85 between inner edges, 2.25 high over all with 1.1 between the pairs. The top pair is
pin 1 (the switch's contact 1 and 3 are one side), the bottom pair pin 2. The middle pads of the drawing (hold-downs) are not
connected on this part and left out. The actuator (dia 2.0, 0.4 over the body) is not drawn: the case's deck pocket takes it."""
import os

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.join(os.path.dirname(HERE), "lib", "battler.pretty")
NAME = "Tact_ROCPU_TPS850"


def mm(v):
    return pcbnew.FromMM(v)


def line(fp, a, b, layer, w):
    s = pcbnew.PCB_SHAPE(fp, pcbnew.SHAPE_T_SEGMENT)
    s.SetStart(pcbnew.VECTOR2I(mm(a[0]), mm(a[1])))
    s.SetEnd(pcbnew.VECTOR2I(mm(b[0]), mm(b[1])))
    s.SetLayer(layer)
    s.SetWidth(mm(w))
    fp.Add(s)


def main():
    fp = pcbnew.FOOTPRINT(None)
    fp.SetFPID(pcbnew.LIB_ID("battler", NAME))
    fp.SetLibDescription("ROCPU TPS850 top-actuated tact switch 3.5 x 2.75 x 1.4, J-lead pads from the datasheet's land drawing")
    fp.SetKeywords("tact switch top actuated")
    fp.SetAttributes(pcbnew.FP_SMD)
    fp.Reference().SetPosition(pcbnew.VECTOR2I(0, mm(-2.4)))
    fp.Reference().SetLayer(pcbnew.F_Fab)
    fp.Value().SetPosition(pcbnew.VECTOR2I(0, mm(2.4)))
    fp.Value().SetLayer(pcbnew.F_Fab)
    lset = pcbnew.LSET()
    for l in (pcbnew.F_Cu, pcbnew.F_Mask, pcbnew.F_Paste):
        lset.AddLayer(l)
    for num, x, y in (("1", -1.625, -0.8), ("1", 1.625, -0.8), ("2", -1.625, 0.8), ("2", 1.625, 0.8)):
        pad = pcbnew.PAD(fp)
        pad.SetNumber(num)
        pad.SetAttribute(pcbnew.PAD_ATTRIB_SMD)
        pad.SetShape(pcbnew.PAD_SHAPE_RECTANGLE)
        pad.SetSize(pcbnew.VECTOR2I(mm(0.4), mm(0.575)))
        pad.SetLayerSet(lset)
        pad.SetPosition(pcbnew.VECTOR2I(mm(x), mm(y)))
        fp.Add(pad)
    for a, b in (((-1.75, -1.375), (1.75, -1.375)), ((1.75, -1.375), (1.75, 1.375)), ((1.75, 1.375), (-1.75, 1.375)), ((-1.75, 1.375), (-1.75, -1.375))):
        line(fp, a, b, pcbnew.F_Fab, 0.1)
    for a, b in (((-2.1, -1.7), (2.1, -1.7)), ((2.1, -1.7), (2.1, 1.7)), ((2.1, 1.7), (-2.1, 1.7)), ((-2.1, 1.7), (-2.1, -1.7))):
        line(fp, a, b, pcbnew.F_CrtYd, 0.05)
    pcbnew.GetPluginForPath(LIB).FootprintSave(LIB, fp)
    print("saved", NAME)


if __name__ == "__main__":
    main()
