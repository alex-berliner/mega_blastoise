#!/usr/bin/env python3
"""Footprints as placed against the makers' land patterns (VER-125).

    python3 hardware/vv/tools/check_land.py          # exit 1 on a difference, 2 when every on-disk part passes
                                                     # but some parts' drawings are not on disk (BLOCKED)
    python3 hardware/vv/tools/check_land.py --seed   # U2's exposed pad shrunk to 1.4 mm in memory: must exit 1

For each part whose maker's drawing is in hardware/ref_design/datasheets, it reads the footprint from the
routed board (pcbnew), turns it to 0 degrees at the origin, and compares the pad count, the pitch, the
exposed pad's size, pin 1's corner and the numbering's direction with the drawing (the numbers below,
each with its page or line). The pad width passes within 0.05 mm of the drawing's (it sets the gap
between neighbours), the row-to-row span within 0.30 mm, and a QFN's pad-to-exposed-pad gap no smaller
than the drawing's (DF-148's bridge: on U2 the exposed pad is GND and pins 2 and 3 are the cell); the
pad length is printed only: IPC-7351's nominal
pads (KiCad's) run longer than a maker's example, and either solders.

Every other part is listed with the drawing it waits for (the URL pin_review records): those footprints
are BLOCKED here, not passed.

Python 3 standard library and pcbnew.
"""
import json
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import design_numbers as dn  # noqa: E402

SPAN_TOL, WIDTH_TOL, PITCH_TOL, EP_TOL = 0.30, 0.05, 0.01, 0.05   # mm

# (refs, source, pad count without EP, pitch, (pad length, width) or None, row span (centre to centre) or a
# (min, max) band, EP (x, y) or None, numbering)
LAND = [
    (("U2",), "SLUS810N RGT0016C example board layout, bq24074.txt:3061-3100 (16X (0.6) x (0.24), 12X (0.5), (2.8), "
              "EP (1.68))", 16, 0.5, (0.6, 0.24), 2.8, (1.68, 1.68), "quad"),
    (("U4",), "rp2350.pdf p.1330 Figure 146, the QFN-80 recommended footprint (10.573 and 9.013 across, 0.18 pads at "
              "0.40, EP 3.40)", 80, 0.4, ((10.573 - 9.013) / 2, 0.18), (10.573 + 9.013) / 2, (3.40, 3.40), "quad"),
    (("U8", "U9"), "sn74hc165.pdf p.34, PW0016A example board layout (16X (1.5) x (0.45), 14X (0.65), (5.8))", 16, 0.65,
     (1.5, 0.45), 5.8, None, "dual"),
    (("U3",), "rt9013.pdf p.11, SOT-23-5 package (e 0.838-1.041, lead tips C 2.591-2.997); pins 1-3 along one side, "
              "4-5 along the other (p.2, u3.json)", 5, (0.838, 1.041), None, (2.591 - 0.6, 2.997), None, "sot5"),
]


def local_pads(pcb, fp):
    f = pcb.FOOTPRINT(fp)
    if f.IsFlipped():
        f.Flip(f.GetPosition(), pcb.FLIP_DIRECTION_TOP_BOTTOM)
    f.SetOrientationDegrees(0)
    f.SetPosition(pcb.VECTOR2I(0, 0))
    mm = pcb.ToMM
    out = {}
    for p in f.Pads():
        if not p.GetNumber():
            continue
        sx, sy = mm(p.GetSize().x), mm(p.GetSize().y)
        if round(p.GetOrientationDegrees()) % 180 == 90:
            sx, sy = sy, sx
        out.setdefault(p.GetNumber(), []).append((mm(p.GetPosition().x), mm(p.GetPosition().y), sx, sy))
    return out


def check(pcb, fp, spec, seed):
    _, src, count, pitch, pad, span, ep, kind = spec
    pads = local_pads(pcb, fp)
    problems, notes = [], []
    sig = {n: v[0] for n, v in pads.items() if n.isdigit() and int(n) <= count}
    ex = None
    if len(sig) != count:
        problems.append(f"{len(sig)} signal pads, the drawing has {count}")
        return problems, notes
    if ep:
        epn = str(count + 1)
        if epn not in pads:
            problems.append(f"no exposed pad {epn}")
        else:
            biggest = max(pads[epn], key=lambda q: q[2] * q[3])
            ex, ey = biggest[2], biggest[3]
            if seed:
                ex, ey = 1.4, 1.4
            if abs(ex - ep[0]) > EP_TOL or abs(ey - ep[1]) > EP_TOL:
                problems.append(f"exposed pad {ex:.2f} x {ey:.2f} mm, the drawing {ep[0]} x {ep[1]}")
            notes.append(f"EP {ex:.2f} x {ey:.2f} ({ep[0]} x {ep[1]})")
    # pitch along each side
    if kind in ("quad", "dual"):
        side = count // (4 if kind == "quad" else 2)
        p1, p2 = sig["1"], sig["2"]
        got = math.hypot(p2[0] - p1[0], p2[1] - p1[1])
        if abs(got - pitch) > PITCH_TOL:
            problems.append(f"pitch {got:.3f} mm, the drawing {pitch}")
        # pin 1 at the top of the left column, numbering down it (counter-clockwise seen from the top)
        left = [sig[str(i)] for i in range(1, side + 1)]
        if not (all(abs(q[0] - left[0][0]) < 0.01 for q in left) and all(b[1] > a[1] for a, b in zip(left, left[1:]))
                and left[0][0] < 0 and left[0][1] < 0):
            problems.append("pins 1.. do not run down the left side from the top-left corner")
        lx, ly = (p1[2], p1[3])
        if abs(ly - pad[1]) > WIDTH_TOL + 1e-6:
            problems.append(f"pad width {ly:.2f} mm, the drawing {pad[1]:.2f} (limit +-{WIDTH_TOL})")
        opp = sig[str(count - side + 1 if kind == "dual" else 2 * side + 1)]
        got_span = abs(opp[0] - p1[0])
        if abs(got_span - span) > SPAN_TOL:
            problems.append(f"row span {got_span:.2f} mm, the drawing {span:.2f}")
        gap = ""
        if ep and ex:
            mine = got_span / 2 - lx / 2 - ex / 2
            theirs = span / 2 - pad[0] / 2 - ep[0] / 2
            gap = f"; pad to EP {mine:.2f} mm (the drawing's {theirs:.2f})"
            if mine < theirs - 0.01:
                problems.append(f"pad to exposed pad {mine:.2f} mm, the drawing's {theirs:.2f}: closer, more bridging (DF-148)")
        notes.append(f"{count} pads at {got:.3f} mm ({pitch}); pad {lx:.2f} x {ly:.2f} ({pad[0]:.2f} x {pad[1]:.2f}); "
                     f"span {got_span:.2f} ({span:.2f}){gap}; pin 1 top-left, counter-clockwise")
        if problems:
            problems += notes[-1:]
    else:   # SOT-23-5, its rows along x or y
        a, b, c3, d, e = (sig[str(i)] for i in range(1, 6))
        if abs(a[0] - b[0]) < 0.01:
            a, b, c3, d, e = ((q[1], q[0]) + q[2:] for q in (a, b, c3, d, e))
        got = abs(b[0] - a[0])
        if not pitch[0] <= got <= pitch[1]:
            problems.append(f"pitch {got:.3f} mm outside e {pitch}")
        row = all(abs(q[1] - a[1]) < 0.01 for q in (b, c3)) and abs(d[1] - e[1]) < 0.01 and (d[1] - a[1]) * a[1] < 0
        order = (a[0] < b[0] < c3[0] or a[0] > b[0] > c3[0]) and abs(d[0] - c3[0]) < 0.01 and abs(e[0] - a[0]) < 0.01
        if not (row and order):
            problems.append("pins 1-3 not along one side with 4 opposite 3 and 5 opposite 1")
        got_span = abs(d[1] - a[1])
        if not span[0] <= got_span <= span[1]:
            problems.append(f"row span {got_span:.2f} mm outside {span[0]:.2f}-{span[1]:.2f} (lead tips less a 0.6 mm foot)")
        notes.append(f"5 pads at {got:.3f} mm (e {pitch[0]}-{pitch[1]}); span {got_span:.2f}; 1-2-3 along one side, "
                     "4 opposite 3, 5 opposite 1")
    return problems, notes


def main():
    pcb, b = dn.board()
    parts, _ = dn.design()
    seed = "--seed" in sys.argv
    fps = {f.GetReference(): f for f in b.GetFootprints()}
    bad = 0
    done = set()
    print("compared with a drawing on disk:")
    for spec in LAND:
        for ref in spec[0]:
            problems, notes = check(pcb, fps[ref], spec, seed and ref == "U2")
            done.add(ref)
            bad += bool(problems)
            print(f"  {'ok  ' if not problems else 'FAIL'}  {ref} {parts[ref]['fp'].split(':')[1]}: "
                  + "; ".join(problems or notes) + f"  [{spec[1]}]")
    review = os.path.join(dn.BOARD, "pin_review")
    urls = {}
    for name in sorted(os.listdir(review)):
        d = json.load(open(os.path.join(review, name)))
        ds = d.get("datasheet")
        url = ds.get("url") if isinstance(ds, dict) else (ds[0] if isinstance(ds, list) and ds else None)
        for r in d.get("refs", []) or []:
            urls[r] = url
    waiting = {}
    for ref, p in parts.items():
        if ref in done or not p["bom"] or not p["fp"] or ref.startswith(("#", "TP", "H", "FID", "JP", "SW")):
            continue
        if p["lib"] in ("Device:R", "Device:C"):
            key = "chip passives (UNI-ROYAL, Samsung, FH), KiCad's IPC-7351 R/C footprints; the makers' land patterns"
            waiting.setdefault(key, []).append(ref)
            continue
        key = f"{p['mpn']} ({p['fp'].split(':')[1]}): {urls.get(ref) or 'no URL recorded'}"
        waiting.setdefault(key, []).append(ref)
    print("BLOCKED, the maker's drawing not in hardware/ref_design/datasheets:")
    for k, refs in sorted(waiting.items(), key=lambda kv: kv[1][0]):
        print(f"  {', '.join(refs[:12])}{' ...' if len(refs) > 12 else ''}: {k}")
    print("the SOT pin numbering (D2, Q1, Q3, Q5, Q6, Q7, U1, U3, D3, D8) is pin_review's per-pin table, which VER-02 "
          "(check_pins.py) reads; U3's is checked above")
    if bad:
        print("RESULT: FAIL")
        return 1
    print("RESULT: BLOCKED (every footprint with a drawing on disk passes)")
    return 2


if __name__ == "__main__":
    sys.exit(main())
