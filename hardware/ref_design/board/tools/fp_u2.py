"""battler:VQFN-16-1EP_3x3mm_RGT0016C: KiCad's VQFN-16 with the pads cut to TI's RGT0016C land pattern (bq24074.pdf, EXAMPLE BOARD
LAYOUT: 16 pads 0.24 x 0.6, centre lines 2.8 apart, so 0.26 from the 1.68 mm exposed pad's edge; KiCad's 0.25 x 0.825 pads
at 1.462 left 0.21: a bridge from a BAT pin to the pad shorts the cell, VER-125 / DEF-72)."""
import os

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.join(os.path.dirname(HERE), "lib", "battler.pretty")
SRC = "/usr/share/kicad/footprints/Package_DFN_QFN.pretty"
NAME = "VQFN-16-1EP_3x3mm_RGT0016C"


def main():
    fp = pcbnew.FootprintLoad(SRC, "VQFN-16-1EP_3x3mm_P0.5mm_EP1.68x1.68mm")
    mm, fr = pcbnew.ToMM, pcbnew.FromMM
    for p in fp.Pads():
        if p.GetNumber() == "17":
            continue
        x, y = mm(p.GetX()), mm(p.GetY())
        if abs(abs(x) - 1.462) < 0.01:                  # left / right: long in x
            p.SetSize(pcbnew.VECTOR2I(fr(0.6), fr(0.24)))
            p.SetX(fr(1.4 if x > 0 else -1.4))
        else:                                           # top / bottom: long in y
            p.SetSize(pcbnew.VECTOR2I(fr(0.24), fr(0.6)))
            p.SetY(fr(1.4 if y > 0 else -1.4))
    fp.SetFPID(pcbnew.LIB_ID("battler", NAME))
    pcbnew.GetPluginForPath(LIB).FootprintSave(LIB, fp)
    print("saved", NAME)


if __name__ == "__main__":
    main()
