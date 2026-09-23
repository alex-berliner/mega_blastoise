"""Place the 16 membrane contacts where the case says they go.

Positions come from hardware/prod_case/layout.scad itself (its echo block), not
from a copied table, so the case stays the single source of truth. Re-run after
changing the case to move every button to its new home.

    python3 place_buttons.py            # close the board in KiCad first

Existing SW footprints are moved and renamed rather than duplicated; missing
ones are loaded from the prod library. All end up locked.
"""
import os
import re
import subprocess
import sys
import tempfile

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD = os.path.join(HERE, "..", "prod_board", "prod_board.kicad_pcb")
LIB = os.path.join(HERE, "..", "prod_board", "prod.pretty")
FOOTPRINT = "DSL_button_4ring"
CASE = os.path.join(HERE, "..", "..", "prod_case", "layout.scad")


def case_positions():
    # OpenSCAD 2021 writes its echo block to a file when the output is *.echo
    with tempfile.TemporaryDirectory() as tmp:
        echo = os.path.join(tmp, "layout.echo")
        subprocess.run(["openscad", "-o", echo, CASE], capture_output=True, text=True)
        text = open(echo).read() if os.path.exists(echo) else ""
    pos = {}
    for ref, x, y in re.findall(r'"(SW\d+)\s.*?KiCad \(([-\d.]+), ([-\d.]+)\)', text):
        pos[ref] = (float(x), float(y))
    if len(pos) != 16:
        sys.exit("expected 16 button positions from the case, got %d" % len(pos))
    return pos


def main():
    if os.path.exists(os.path.join(os.path.dirname(BOARD), "~prod_board.kicad_pcb.lck")):
        sys.exit("the board is open in KiCad; close it first or KiCad will overwrite this")
    pos = case_positions()
    board = pcbnew.LoadBoard(BOARD)

    existing = {}
    placeholders = []
    for fp in board.GetFootprints():
        # KiCad hands back its own string type; compare as str or nothing ever matches
        if str(fp.GetFPID().GetLibItemName()) != FOOTPRINT:
            continue
        ref = fp.GetReference()
        if ref in pos:
            existing[ref] = fp
        else:
            placeholders.append(fp)     # e.g. an unnamed SW** dropped by hand

    for ref in sorted(pos, key=lambda r: int(r[2:])):
        fp = existing.get(ref)
        if fp is None and placeholders:
            fp = placeholders.pop(0)
        if fp is None:
            fp = pcbnew.FootprintLoad(LIB, FOOTPRINT)
            fp.SetFPID(pcbnew.LIB_ID("prod", FOOTPRINT))
            fp.thisown = False
            board.Add(fp)
        fp.SetReference(ref)
        x, y = pos[ref]
        fp.SetPosition(pcbnew.VECTOR2I(pcbnew.FromMM(x), pcbnew.FromMM(y)))
        fp.SetOrientationDegrees(0)
        fp.SetLocked(True)
        print("%-5s at (%.2f, %.2f)" % (ref, x, y))

    for fp in placeholders:
        print("left alone, not a known button:", fp.GetReference())
    pcbnew.SaveBoard(BOARD, board)
    print("saved", os.path.normpath(BOARD))


if __name__ == "__main__":
    main()
