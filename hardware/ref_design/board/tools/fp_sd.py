"""battler:microSD_DM3AT-SF-PEJM5: KiCad's Connector_Card footprint with the two front shell-tab pads (the ones at the card's mouth, +y)
shortened so their copper ends 0.43 sooner (to y 7.75 from the origin instead of 8.325 / 8.32): the slot's front sits at the board's
edge in the left wall's opening, and the tabs' ends would hang over it. The tabs' inner ends and the rest are KiCad's. Unverified
against Hirose's land pattern (its PCB mounting pattern, catalog p3, has no dimension for these tabs' ends)."""
import os

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.join(os.path.dirname(HERE), "lib", "battler.pretty")
SRC = "/usr/share/kicad/footprints/Connector_Card.pretty"
NAME = "microSD_HC_Hirose_DM3AT-SF-PEJM5"
FRONT = 7.75


def main():
    fp = pcbnew.FootprintLoad(SRC, NAME)
    mm = pcbnew.ToMM
    n = 0
    for p in fp.Pads():
        if p.GetNumber() != "SH":
            continue
        y, h = mm(p.GetY()), mm(p.GetSizeY())
        if y + h / 2 > FRONT + 0.05 and y > 5:
            top = y - h / 2
            p.SetSizeY(pcbnew.FromMM(FRONT - top))
            p.SetY(pcbnew.FromMM((FRONT + top) / 2))
            n += 1
    fp.SetFPID(pcbnew.LIB_ID("battler", NAME))
    pcbnew.GetPluginForPath(LIB).FootprintSave(LIB, fp)
    print("saved", NAME, "front pads shortened:", n)


if __name__ == "__main__":
    main()
