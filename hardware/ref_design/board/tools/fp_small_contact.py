"""The Start/Select membrane contact: hardware/ref_design/board/membrane's DS Lite comb with its outer
ring left off (3 rings: copper 3.6 mm across, mask 3.9), for the smaller pills on the Start/Select membrane.
Written into lib/battler.pretty through the same generator, so the comb geometry,
clearances and attributes are the 4-ring contact's.

    python3 tools/fp_small_contact.py

ESTIMATE: the pill's size on a real Start/Select membrane hasn't been measured; the
3-ring comb's 3.6 mm of copper suits a pill about 3 mm across or more.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
sys.path.insert(0, os.path.join(REPO, "hardware", "membrane", "tools"))
import gen_dsl_button as g  # noqa: E402

_comb4 = g.comb
g.comb = lambda rot: _comb4(rot)[:5]      # rings 1-3: drop the step out to ring 4 and its arc
g.MASK_DIA = 3.90
g.COURTYARD_DIA = 4.40
LIB = os.path.join(os.path.dirname(HERE), "lib", "battler.pretty")

if __name__ == "__main__":
    sys.argv = [sys.argv[0], LIB, "DSL_button_3ring"]
    g.main()
    # one mask opening over both combs is the point (as on the 4-ring contact): tell DRC
    io = g.pcbnew.PCB_IO_MGR.FindPlugin(g.pcbnew.PCB_IO_MGR.KICAD_SEXP)
    fp = io.FootprintLoad(LIB, "DSL_button_3ring")
    fp.SetAllowSolderMaskBridges(True)
    io.FootprintSave(LIB, fp)
