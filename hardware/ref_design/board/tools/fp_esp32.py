"""battler:ESP32-S3-WROOM-1U, the module's footprint: KiCad's RF_Module copy minus its 12 thermal vias
(0.2 mm drills, under the 0.3 mm the fab's cheap tier allows). What stays: 40 edge pads, the 3.9 mm
ground pad 41 on F.Cu and its nine paste squares. The ground pad connects to the plane through
board vias placed around the module (board/placement.py), none under the paste.

    python3 tools/fp_esp32.py
"""
import os

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.join(os.path.dirname(HERE), "lib", "battler.pretty")
SRC = "/usr/share/kicad/footprints/RF_Module.pretty"
NAME = "ESP32-S3-WROOM-1U"


def main():
    fp = pcbnew.FootprintLoad(SRC, NAME)
    drop = [p for p in fp.Pads() if p.GetAttribute() != pcbnew.PAD_ATTRIB_SMD]
    for p in drop:
        fp.Remove(p)
    fp.SetFPID(pcbnew.LIB_ID("battler", NAME))
    pcbnew.GetPluginForPath(LIB).FootprintSave(LIB, fp)
    print("saved", NAME, "without", len(drop), "through pads;", len(list(fp.Pads())), "left")


if __name__ == "__main__":
    main()
