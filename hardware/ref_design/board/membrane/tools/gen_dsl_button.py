"""Generate the 4-ring DS Lite membrane contact footprint.

Two interleaved combs, one per net, identical but rotated 180 degrees. Each comb
is a stepped spiral: an arc on ring 1, a radial step out, an arc on ring 2, and
so on, alternating halves so that walking outward from the centre always
crosses signal, common, signal, common (or the reverse). Neither comb ever
crosses the other, so every pad is one connected piece of copper.

Ring radii and conductor width are the DS Lite breakout's, measured from its
gerbers (hardware/ref_design/board/membrane/README.md). The step angles are ours: each ring's
gap between nets is sized so the two combs never come closer than CLEARANCE,
which is why the gap is wider on the inner rings than the outer.

    python3 gen_dsl_button.py <library.pretty> [name]

Uses KiCad's own pcbnew module, so the file is written by KiCad's serialiser,
not by hand.
"""
import math
import sys

import pcbnew

RINGS = [0.720, 1.260, 1.720, 2.180]   # centreline radii, mm (measured)
WIDTH = 0.1524                         # conductor width, mm (measured)
CLEARANCE = 0.20                       # minimum copper gap between the nets, mm
MASK_DIA = 4.80                        # one clean solder mask opening, mm
COURTYARD_DIA = 5.30


def gap_angle(r):
    """Angle (deg) between the nets at radius r that leaves CLEARANCE of copper gap.

    The tightest spot is not the chord between the two arc ends on the ring: it
    is the perpendicular from the other net's arc end onto the radial step, which
    lands just inside the ring. That distance is r*sin(g), so g = asin(...)."""
    return math.degrees(math.asin((CLEARANCE + WIDTH) / r))


def comb(rot):
    """One net's copper as ('arc', r, a0, a1) and ('step', a, r0, r1) items.

    Angles are degrees CCW from +X in maths convention; rot turns the whole
    comb, so the other net is comb(180)."""
    g = [gap_angle(r) for r in RINGS]
    # centre ring 1's arc on the top of the pad
    t12 = 90 + (180 - g[0]) / 2
    t23 = t12 + 180 - g[1]
    t34 = t23 + 180 - g[2]
    items = [
        ("arc", RINGS[0], t12 - (180 - g[0]), t12),
        ("step", t12, RINGS[0], RINGS[1]),
        ("arc", RINGS[1], t12, t23),
        ("step", t23, RINGS[1], RINGS[2]),
        ("arc", RINGS[2], t23, t34),
        ("step", t34, RINGS[2], RINGS[3]),
        ("arc", RINGS[3], t34, t34 + 180 - g[3]),
    ]
    out = []
    for it in items:
        if it[0] == "arc":
            out.append(("arc", it[1], it[2] + rot, it[3] + rot))
        else:
            out.append(("step", it[1] + rot, it[2], it[3]))
    return out


def pt(r, deg):
    # KiCad's Y axis points down; flip so the footprint reads like the diagram.
    a = math.radians(deg)
    return (r * math.cos(a), -r * math.sin(a))


def v(x, y):
    return pcbnew.VECTOR2I(pcbnew.FromMM(x), pcbnew.FromMM(y))


def build_pad(fp, number, items):
    # anchor on the outer ring's midpoint: that is where a trace will land,
    # so the ratsnest points somewhere sensible
    outer = items[-1]
    ax, ay = pt(outer[1], (outer[2] + outer[3]) / 2)

    pad = pcbnew.PAD(fp)
    pad.SetNumber(number)
    pad.SetAttribute(pcbnew.PAD_ATTRIB_SMD)
    pad.SetShape(pcbnew.F_Cu, pcbnew.PAD_SHAPE_CUSTOM)
    pad.SetAnchorPadShape(pcbnew.F_Cu, pcbnew.PAD_SHAPE_CIRCLE)
    pad.SetSize(pcbnew.F_Cu, v(WIDTH, WIDTH))  # hides inside the conductor
    layers = pcbnew.LSET()
    layers.AddLayer(pcbnew.F_Cu)                 # no paste, no pad mask
    pad.SetLayerSet(layers)
    pad.SetPosition(v(ax, ay))

    for it in items:
        if it[0] == "arc":
            _, r, a0, a1 = it
            s, m, e = pt(r, a0), pt(r, (a0 + a1) / 2), pt(r, a1)
            sh = pcbnew.PCB_SHAPE(None, pcbnew.SHAPE_T_ARC)
            sh.SetArcGeometry(v(s[0] - ax, s[1] - ay), v(m[0] - ax, m[1] - ay),
                              v(e[0] - ax, e[1] - ay))
        else:
            _, a, r0, r1 = it
            s, e = pt(r0, a), pt(r1, a)
            sh = pcbnew.PCB_SHAPE(None, pcbnew.SHAPE_T_SEGMENT)
            sh.SetStart(v(s[0] - ax, s[1] - ay))
            sh.SetEnd(v(e[0] - ax, e[1] - ay))
        sh.SetWidth(pcbnew.FromMM(WIDTH))
        # the pad takes ownership of the pointer without copying it; if Python
        # also owns it, the shape is freed and its memory reused under the pad
        sh.thisown = False
        pad.AddPrimitiveShape(pcbnew.F_Cu, sh)
    pad.thisown = False
    fp.Add(pad)


def circle(fp, layer, dia, width, filled):
    c = pcbnew.PCB_SHAPE(fp, pcbnew.SHAPE_T_CIRCLE)
    c.SetLayer(layer)
    c.SetCenter(v(0, 0))
    c.SetEnd(v(dia / 2, 0))
    c.SetWidth(pcbnew.FromMM(width))
    c.SetFilled(filled)
    c.thisown = False
    fp.Add(c)


def main():
    lib = sys.argv[1]
    name = sys.argv[2] if len(sys.argv) > 2 else "DSL_button_4ring"

    board = pcbnew.BOARD()
    fp = pcbnew.FOOTPRINT(board)
    fp.SetFPID(pcbnew.LIB_ID("", name))
    fp.SetLibDescription(
        "Membrane contact for a DS Lite silicone pill: 4-ring interleaved combs, "
        "%.4f mm conductors, %.2f mm min gap. Bare copper: needs ENIG or hard gold, "
        "too fine for carbon ink. Not a placed part." % (WIDTH, CLEARANCE))
    fp.SetKeywords("membrane contact carbon pill keypad DS Lite")
    # copper only: nothing to buy and nothing for the pick-and-place machine
    fp.SetAttributes(pcbnew.FP_EXCLUDE_FROM_BOM | pcbnew.FP_EXCLUDE_FROM_POS_FILES)

    fp.Reference().SetText("SW**")
    fp.Reference().SetLayer(pcbnew.F_Fab)
    fp.Reference().SetPosition(v(0, -3.0))
    fp.Value().SetText(name)
    fp.Value().SetLayer(pcbnew.F_Fab)
    fp.Value().SetPosition(v(0, 3.0))

    build_pad(fp, "1", comb(0))      # signal
    build_pad(fp, "2", comb(180))    # common

    circle(fp, pcbnew.F_Mask, MASK_DIA, 0, True)
    circle(fp, pcbnew.F_CrtYd, COURTYARD_DIA, 0.05, False)
    circle(fp, pcbnew.F_Fab, 2 * (RINGS[-1] + WIDTH / 2), 0.1, False)

    io = pcbnew.PCB_IO_MGR.FindPlugin(pcbnew.PCB_IO_MGR.KICAD_SEXP)
    io.FootprintSave(lib, fp)
    print("wrote %s/%s.kicad_mod" % (lib, name), flush=True)
    for r in RINGS:
        print("  ring r=%.3f  gap between nets %.1f deg" % (r, gap_angle(r)), flush=True)


if __name__ == "__main__":
    main()
