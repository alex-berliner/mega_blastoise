#!/usr/bin/env python3
"""Report schematic text that lands on top of other text.

KiCad has no such check, and the generator that produced this sheet was happy
to stack a net label on the reference designator it belonged to. Boxes are
approximate: the stroke font advances about 0.95 x the text size per character.
"""
import sexp

SCH = "mega_blastoise_stripboard.kicad_sch"
ADVANCE = 0.95      # character width as a multiple of text size
PAD = 0.2           # mm of slack before two boxes count as touching


def nm(n):
    return n[0] if isinstance(n, list) and isinstance(n[0], str) else None


def get(n, k):
    for c in n:
        if nm(c) == k:
            return c


def getall(n, k):
    return [c for c in n if nm(c) == k]


def box(text, x, y, rot, size, just):
    w, h = len(text) * size * ADVANCE, size
    if just == "right":
        x0 = x - w
    elif just == "center":
        x0 = x - w / 2
    else:
        x0 = x
    off = x0 - x
    if rot % 180 == 0:
        return (x + off, x + off + w, y - h / 2, y + h / 2)
    # rotated text reads bottom-to-top, so the run extends up from the anchor
    return (x - h / 2, x + h / 2, y - off - w, y - off)


def texts():
    root = sexp.parse(open(SCH).read())
    out = []
    for n in root:
        kind = nm(n)
        if kind in ("label", "text"):
            at = get(n, "at")
            eff = get(n, "effects")
            size = float(get(get(eff, "font"), "size")[1])
            j = get(eff, "justify")
            out.append((str(n[1]), box(str(n[1]), float(at[1]), float(at[2]),
                                       float(at[3]), size, j[1] if j else "center")))
        if kind == "symbol":
            srot = float(get(n, "at")[3])
            for p in getall(n, "property"):
                if str(p[1]) not in ("Reference", "Value"):
                    continue
                at, eff = get(p, "at"), get(p, "effects")
                if at is None or eff is None:
                    continue
                if any(str(h[1]) == "yes" for h in getall(eff, "hide")):
                    continue
                size = float(get(get(eff, "font"), "size")[1])
                j = get(eff, "justify")
                out.append((f"{str(p[1])}={str(p[2])}",
                            box(str(p[2]), float(at[1]), float(at[2]),
                                (float(at[3]) + srot) % 360, size,
                                j[1] if j else "center")))
    return out


def hits(a, b):
    return (a[0] < b[1] - PAD and b[0] < a[1] - PAD
            and a[2] < b[3] - PAD and b[2] < a[3] - PAD)


items = texts()
bad = [(items[i], items[j]) for i in range(len(items)) for j in range(i + 1, len(items))
       if hits(items[i][1], items[j][1])]
for (ta, ba), (tb, bb) in bad:
    print(f"OVERLAP  {ta!r} @({ba[0]:.1f},{ba[2]:.1f})  x  {tb!r} @({bb[0]:.1f},{bb[2]:.1f})")
print(f"{len(items)} text items, {len(bad)} overlapping pairs")
