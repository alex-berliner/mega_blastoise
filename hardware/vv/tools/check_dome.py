#!/usr/bin/env python3
"""The dome contact's footprint against the 4 mm dome's drawing (VER-178).

    python3 hardware/vv/tools/check_dome.py          # exit 1 on any failure
    python3 hardware/vv/tools/check_dome.py --seed   # a 4.6 mm dome in place of the 4.0 mm one: must exit 1

The dome is HYP's 600-415S-260 (LCSC C256258), hand-placed on Dome_4mm_Round (SW20-SW23; SW24 is a TPS850 tact switch since 39cc07f) under a
Kapton dot (board/tools/fp_dome.py). Its drawing, hardware/ref_design/datasheets/dome_600-415s-260.pdf, is a
scanned family drawing with no text layer (pdftotext gives 0 lines), so its numbers are read by eye from page 1's
part performance table, row '600-4***-***', and named below with that place; the script prints the PDF's sha256
so the evidence names the drawing it was read from. The footprint is read from board/lib/battler.pretty with
pcbnew (VER-162 checks that the placed copies equal it). It checks:
  1. the rim: the dome's edge (D/2, at the drawing's -0.05 and +0) lands on the ring pad all round with 0.2 mm or
     more to spare inside and outside at 0.1 mm of placement error;
  2. the apex: the dome's centre, at the hand placement's 0.3 mm of error, keeps a contact spot of STAND_SPOT_R
     inside the centre pad (the drawing gives no spot size: a stand-in);
  3. the copper gap from the centre pad to the ring, 0.20 mm or more; the vent's 0.3 mm to both pads (DEF-56);
  4. no paste and no silk on the pads (the footprint's own layers);
  5. the Kapton dot (fp_dome.py's 6 mm) covers the dome at 0.3 mm of error each, and sticks to solder mask beyond the
     4.8 mm opening by 0.2 mm or more.

Python 3 standard library and pcbnew.
"""
import hashlib
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import design_numbers as dn  # noqa: E402

LIB = os.path.join(dn.BOARD, "lib", "battler.pretty")
NAME = "Dome_4mm_Round"
DRAWING = os.path.join(dn.REF, "datasheets", "dome_600-415s-260.pdf")
# read by eye from dome_600-415s-260.pdf page 1, the part performance table's row '600-4***-***' (note 6)
DOME_D = (3.95, 4.0)             # mm: D 'o4 +0 -0.05'
DOME_H = (0.15, 0.25)            # mm: H '0.2 +-0.05'
DOME_TEST_ACT = 2.0              # mm: the test actuator's diameter, Fig. 1 ('o2.0')
# from fp_dome.py's docstring (the design's own words)
KAPTON_D = 6.0                   # mm: the dot over each dome
PLACE_RIM = 0.1                  # mm: VER-178's placement error for the rim
PLACE_HAND = 0.3                 # mm: the hand placement's error (fp_dome.py, VER-178)
SPARE = 0.2                      # mm: the rim's spare to each edge of the ring
GAP_MIN = 0.20                   # mm: copper gap
HOLE_GAP = 0.30                  # mm: copper to an unplated hole (DEF-56)
TAPE_LAND = 0.2                  # mm: the dot's adhesive on solder mask beyond the opening, at its worst placement
STAND_SPOT_R = 0.2               # mm: the radius of the apex's contact spot (the drawing gives none; a stand-in)


def geometry(pcb):
    """(centre pad radius, ring inner and outer radius, vent (r, hole radius), mask opening radius, pad layers)."""
    mm = pcb.ToMM
    fp = pcb.FootprintLoad(LIB, NAME)
    pads = {p.GetNumber(): p for p in fp.Pads()}
    c = pads["1"]
    rc = mm(c.GetSize().x) / 2
    ring = pads["2"]
    poly = ring.GetEffectivePolygon(pcb.F_Cu, pcb.ERROR_INSIDE)
    rs = []
    for i in range(poly.OutlineCount()):
        o = poly.Outline(i)
        rs += [math.hypot(mm(o.CPoint(k).x), mm(o.CPoint(k).y)) for k in range(o.PointCount())]
        for h in range(poly.HoleCount(i)):
            hh = poly.Hole(i, h)
            rs += [math.hypot(mm(hh.CPoint(k).x), mm(hh.CPoint(k).y)) for k in range(hh.PointCount())]
    vent = pads[""]
    vr = math.hypot(mm(vent.GetPosition().x), mm(vent.GetPosition().y))
    mask = [g for g in fp.GraphicalItems() if g.GetLayer() == pcb.F_Mask]
    rmask = max(mm(g.GetRadius()) for g in mask) if mask else None
    layers = {n: [pcb.BOARD.GetStandardLayerName(lay) for lay in p.GetLayerSet().Seq()] for n, p in pads.items() if n}
    return rc, min(rs), max(rs), (vr, mm(vent.GetDrillSize().x) / 2), rmask, layers


def main():
    seed = "--seed" in sys.argv
    pcb, _ = dn.board()
    sha = hashlib.sha256(open(DRAWING, "rb").read()).hexdigest()
    print(f"[dome_600-415s-260.pdf sha256 {sha}: a scanned drawing, no text layer; its numbers read by eye from "
          "page 1's part performance table, row '600-4***-***']")
    d = (4.55, 4.6) if seed else DOME_D
    if seed:
        print(f"SEEDED: a {d[1]} mm dome")
    print(f"the dome: D {d[0]}-{d[1]} mm, H {DOME_H[0]}-{DOME_H[1]} mm (drawing); the test actuator o{DOME_TEST_ACT} "
          "(Fig. 1)")
    rc, ri, ro, (vr, vh), rmask, layers = geometry(pcb)
    print(f"{NAME} (board/lib/battler.pretty): centre pad r {rc:.3f}, ring r {ri:.3f} to {ro:.3f}, vent o{2 * vh:.2f} at "
          f"r {vr:.2f}, mask opening r {rmask:.2f} mm")
    ok = True

    def row(good, text):
        nonlocal ok
        ok &= good
        print(("  ok    " if good else "  FAIL  ") + text)

    inner = d[0] / 2 - PLACE_RIM - ri
    outer = ro - (d[1] / 2 + PLACE_RIM)
    row(min(inner, outer) >= SPARE - 1e-9,
        f"the rim (r {d[0] / 2:.3f}-{d[1] / 2:.3f}) at {PLACE_RIM} mm of placement error: {inner:.3f} mm inside the ring's "
        f"inner edge, {outer:.3f} mm inside its outer edge (limit {SPARE} each)")
    on_ring = d[0] / 2 - PLACE_HAND - ri
    print(f"  info  at the hand placement's {PLACE_HAND} mm: the rim's near side {on_ring:+.3f} mm from the ring's inner "
          f"edge ({'still on copper' if on_ring >= 0 else 'off the ring there; the far side stays on it'}), the far side "
          f"{ro - (d[1] / 2 + PLACE_HAND):+.3f} mm inside its outer edge")
    spot = rc - PLACE_HAND - STAND_SPOT_R
    row(spot >= -1e-9, f"the apex at {PLACE_HAND} mm of hand-placement error keeps a {STAND_SPOT_R} mm contact spot "
                       f"(stand-in) {spot:.3f} mm inside the {2 * rc:.1f} mm centre pad")
    gap = ri - rc
    row(gap >= GAP_MIN, f"copper gap centre pad to ring {gap:.3f} mm (limit {GAP_MIN})")
    vg = min(vr - vh - rc, ri - (vr + vh))
    row(vg >= HOLE_GAP - 1e-9, f"the vent {vg:.3f} mm from the nearer pad (limit {HOLE_GAP}, DEF-56)")
    bad_layers = {n: [x for x in ls if "Paste" in x or "Silk" in x] for n, ls in layers.items()}
    row(not any(bad_layers.values()), f"pad layers {layers}: no paste, no silk")
    tape_cover = KAPTON_D / 2 - PLACE_HAND - (d[1] / 2 + PLACE_HAND)
    land = KAPTON_D / 2 - PLACE_HAND - rmask
    row(tape_cover >= 0 and land >= TAPE_LAND - 1e-9,
        f"the {KAPTON_D} mm Kapton dot (fp_dome.py) at {PLACE_HAND} mm of error over a dome at {PLACE_HAND} mm: covers it "
        f"with {tape_cover:.2f} mm to spare, and lies {land:.2f} mm on solder mask beyond the o{2 * rmask:.1f} mm opening "
        f"(limit {TAPE_LAND})")
    print("RESULT: " + ("PASS" if ok else "FAIL"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
