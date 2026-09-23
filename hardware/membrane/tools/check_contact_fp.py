"""Check a membrane contact footprint's copper, read straight from the .kicad_mod.

Independent of the generator on purpose: it parses what KiCad actually saved.

    python3 check_contact_fp.py <footprint.kicad_mod> [min_gap_mm]

Reports, per pad: that its shapes form one connected piece of copper and that
the anchor sits on it. Then, between pads: the minimum copper gap, whether the
mask opening uncovers all the copper, and what a ray from the centre crosses.
Exits non-zero if the gap is below min_gap or any pad falls apart.
"""
import math
import re
import sys

SAMPLES_PER_MM = 200


def arc_centre(s, m, e):
    (ax, ay), (bx, by), (cx, cy) = s, m, e
    d = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by))
    ux = ((ax**2 + ay**2) * (by - cy) + (bx**2 + by**2) * (cy - ay) + (cx**2 + cy**2) * (ay - by)) / d
    uy = ((ax**2 + ay**2) * (cx - bx) + (bx**2 + by**2) * (ax - cx) + (cx**2 + cy**2) * (bx - ax)) / d
    return (ux, uy), math.hypot(ax - ux, ay - uy)


def sample_arc(s, m, e):
    c, r = arc_centre(s, m, e)
    a0 = math.atan2(s[1] - c[1], s[0] - c[0])
    am = math.atan2(m[1] - c[1], m[0] - c[0])
    a1 = math.atan2(e[1] - c[1], e[0] - c[0])
    tau = 2 * math.pi
    sweep = (a1 - a0) % tau
    if (am - a0) % tau > sweep:          # mid lies the other way round
        sweep -= tau
    n = max(8, int(abs(sweep) * r * SAMPLES_PER_MM))
    return [(c[0] + r * math.cos(a0 + sweep * k / n), c[1] + r * math.sin(a0 + sweep * k / n))
            for k in range(n + 1)], c, r, abs(math.degrees(sweep))


def sample_line(s, e):
    n = max(4, int(math.dist(s, e) * SAMPLES_PER_MM))
    return [(s[0] + (e[0] - s[0]) * k / n, s[1] + (e[1] - s[1]) * k / n) for k in range(n + 1)]


def parse(path):
    text = open(path).read()
    pads = {}
    for pm in re.finditer(r'\(pad "([^"]+)" smd custom\s*\(at ([-\d.]+) ([-\d.]+)(?: ([-\d.]+))?\)(.*?)\n\t\)', text, re.S):
        num, px, py, body = pm.group(1), float(pm.group(2)), float(pm.group(3)), pm.group(5)
        # primitives are stored relative to the pad AND in the pad's own rotation
        rot = math.radians(float(pm.group(4) or 0))
        cr, sr = math.cos(rot), math.sin(rot)

        def place(x, y, px=px, py=py, cr=cr, sr=sr):
            # KiCad angles turn counter-clockwise on screen, where Y points down
            return (px + x * cr + y * sr, py - x * sr + y * cr)
        shapes = []
        width = None
        for m in re.finditer(
                r'\((gr_arc|gr_line)\s*\(start ([-\d.]+) ([-\d.]+)\)\s*(?:\(mid ([-\d.]+) ([-\d.]+)\)\s*)?'
                r'\(end ([-\d.]+) ([-\d.]+)\)\s*\(width ([\d.]+)\)', body):
            s = place(float(m.group(2)), float(m.group(3)))
            e = place(float(m.group(6)), float(m.group(7)))
            width = float(m.group(8))
            if m.group(1) == "gr_arc":
                mid = place(float(m.group(4)), float(m.group(5)))
                pts, c, r, sweep = sample_arc(s, mid, e)
                shapes.append({"kind": "arc", "pts": pts, "r": r, "c": c, "sweep": sweep})
            else:
                shapes.append({"kind": "step", "pts": sample_line(s, e),
                               "r0": math.hypot(*s), "r1": math.hypot(*e)})
        pads[num] = {"anchor": (px, py), "shapes": shapes, "width": width}
    mask = re.search(r'\(fp_circle\s*\(center ([-\d.]+) ([-\d.]+)\)\s*\(end ([-\d.]+) ([-\d.]+)\)'
                     r'(?:(?!\(fp_).)*?\(layer "F\.Mask"\)', text, re.S)
    mask_r = None
    if mask:
        mask_r = math.hypot(float(mask.group(3)) - float(mask.group(1)),
                            float(mask.group(4)) - float(mask.group(2)))
    return pads, mask_r


def nearest(p, pts):
    return min(math.dist(p, q) for q in pts)


def connected(shapes, tol):
    n = len(shapes)
    seen, todo = {0}, [0]
    while todo:
        i = todo.pop()
        ends_i = [shapes[i]["pts"][0], shapes[i]["pts"][-1]]
        for j in range(n):
            if j in seen:
                continue
            ends_j = [shapes[j]["pts"][0], shapes[j]["pts"][-1]]
            # joined if either shape's end lands on the other's centreline
            if any(nearest(p, shapes[j]["pts"]) < tol for p in ends_i) or \
               any(nearest(p, shapes[i]["pts"]) < tol for p in ends_j):
                seen.add(j)
                todo.append(j)
    return len(seen) == n


def main():
    path = sys.argv[1]
    min_gap = float(sys.argv[2]) if len(sys.argv) > 2 else 0.20
    pads, mask_r = parse(path)
    ok = True

    for num, pad in sorted(pads.items()):
        w = pad["width"]
        pts = [p for s in pad["shapes"] for p in s["pts"]]
        print("pad %s: %d shapes, anchor (%.3f, %.3f)" % (num, len(pad["shapes"]), *pad["anchor"]))
        for s in pad["shapes"]:
            if s["kind"] == "arc":
                print("    arc   r=%.4f  sweep %5.1f deg  centre (%+.4f, %+.4f)" % (s["r"], s["sweep"], *s["c"]))
            else:
                print("    step  r %.4f -> %.4f" % (s["r0"], s["r1"]))
        one = connected(pad["shapes"], tol=0.005)
        on = nearest(pad["anchor"], pts) < w / 2
        print("    one connected piece: %s   anchor on copper: %s" % ("yes" if one else "NO", "yes" if on else "NO"))
        ok &= one and on

    names = sorted(pads)
    w = pads[names[0]]["width"]
    gaps = []
    for i in range(len(names)):
        for j in range(i + 1, len(names)):
            a = [p for s in pads[names[i]]["shapes"] for p in s["pts"]]
            b = [p for s in pads[names[j]]["shapes"] for p in s["pts"]]
            d, where = min((nearest(p, b), p) for p in a)
            gaps.append((d - w, names[i], names[j], where))
    for g, x, y, where in gaps:
        verdict = "OK" if g >= min_gap - 1e-4 else "TOO TIGHT"
        print("\nmin copper gap pad %s <-> pad %s: %.4f mm (need >= %.2f)  %s  near (%.3f, %.3f)"
              % (x, y, g, min_gap, verdict, *where))
        ok &= g >= min_gap - 1e-4

    rmax = max(math.hypot(*p) for pad in pads.values() for s in pad["shapes"] for p in s["pts"]) + w / 2
    if mask_r:
        print("mask opening r=%.3f vs outermost copper r=%.3f: %s"
              % (mask_r, rmax, "covers it" if mask_r > rmax else "COPPER UNDER MASK"))
        ok &= mask_r > rmax

    # The physical question: a pill's contact spot is a disc. Wherever it lands,
    # does it touch both nets? For every spot centre on the pad, the smallest
    # disc that reaches both nets has radius max(d1, d2), d = distance to that
    # net's copper edge. The worst centre sets the smallest spot that bridges
    # everywhere. (A ray test is misleading: a ray along a radial step walks
    # inside one net's copper.)
    rmax_cu = rmax - w / 2
    coarse = {}
    for num, pad in pads.items():
        pts = [q for sh in pad["shapes"] for q in sh["pts"]]
        coarse[num] = pts[::4]          # ~0.02 mm spacing is plenty here
    worst, where, step = 0.0, None, 0.05
    n = int(rmax_cu / step)
    for ix in range(-n, n + 1):
        for iy in range(-n, n + 1):
            c = (ix * step, iy * step)
            if math.hypot(*c) > rmax_cu:
                continue
            need = max(nearest(c, coarse[k]) - w / 2 for k in coarse)
            if need > worst:
                worst, where = need, c
    pill_r = 1.75                       # smallest DS Lite pill, ~3.5 mm across
    print("\nsmallest contact spot that bridges both nets anywhere on the pad: %.2f mm across"
          % (2 * worst))
    print("    worst spot centred at (%.2f, %.2f); a %.1f mm pill clears it %s"
          % (*where, 2 * pill_r, "comfortably" if worst < pill_r else "NOT"))
    ok &= worst < pill_r
    print("\nRESULT:", "PASS" if ok else "FAIL")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
