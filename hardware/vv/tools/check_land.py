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
    # 39cc07f: the ESP32-S3-WROOM-1U's own land pattern (Figure 11-2: 40 x 1.5 x 0.9 pads at 1.27, the thermal pad 3.7,
    # the columns 17.5 apart, esp32-s3-wroom-1.txt:3422-3471); the TLV75733P's DBV0005A example layout (5X (1.1) x (0.6),
    # 2X (0.95), (2.6), tlv757p.txt:2100-2122); U11 is the third PW0016A
    (("U4",), "esp32-s3-wroom-1.pdf Figure 11-2, the WROOM-1U recommended land pattern (40 x1.5, 40 x0.9, 1.27, EP 3.7, "
              "17.5; esp32-s3-wroom-1.txt:3422-3471)", 40, 1.27, (1.5, 0.9), 17.5, (3.7, 3.7), "module"),
    (("U8", "U9", "U11"), "sn74hc165.pdf p.34, PW0016A example board layout (16X (1.5) x (0.45), 14X (0.65), (5.8))", 16,
     0.65, (1.5, 0.45), 5.8, None, "dual"),
    (("U3",), "tlv757p.pdf DBV0005A example board layout (5X (1.1) x 5X (0.6), 2X (0.95), (2.6); tlv757p.txt:2100-2122); "
              "pins 1-3 along one side, 4-5 along the other", 5, (0.94, 0.96), None, (2.6 - SPAN_TOL, 2.6 + SPAN_TOL), None,
     "sot5"),
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
    if kind == "module":   # 14 down one side, 12 along the end, 14 up the other (the WROOM's castellations)
        p1, p2 = sig["1"], sig["2"]
        got = math.hypot(p2[0] - p1[0], p2[1] - p1[1])
        if abs(got - pitch) > PITCH_TOL:
            problems.append(f"pitch {got:.3f} mm, the drawing {pitch}")
        lx, ly = sorted((p1[2], p1[3]), reverse=True)
        if abs(lx - pad[0]) > WIDTH_TOL + 1e-6 or abs(ly - pad[1]) > WIDTH_TOL + 1e-6:
            problems.append(f"pad {lx:.2f} x {ly:.2f} mm, the drawing {pad[0]} x {pad[1]} (limit +-{WIDTH_TOL})")
        opp = sig[str(count)]
        got_span = math.hypot(opp[0] - p1[0], opp[1] - p1[1])
        if abs(got_span - span) > SPAN_TOL:
            problems.append(f"pin 1 to pin {count} {got_span:.2f} mm, the drawing's columns {span:.2f} apart")
        notes.append(f"{count} pads at {got:.3f} mm ({pitch}); pad {lx:.2f} x {ly:.2f} ({pad[0]} x {pad[1]}); columns "
                     f"{got_span:.2f} apart ({span})")
        if problems:
            problems += notes[-1:]
        return problems, notes
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
            a, b, c3, d, e = ((q[1], q[0], q[3], q[2]) for q in (a, b, c3, d, e))
        got = abs(b[0] - a[0])
        if not pitch[0] <= got <= pitch[1]:
            problems.append(f"pitch {got:.3f} mm outside e {pitch}")
        row = all(abs(q[1] - a[1]) < 0.01 for q in (b, c3)) and abs(d[1] - e[1]) < 0.01 and (d[1] - a[1]) * a[1] < 0
        order = (a[0] < b[0] < c3[0] or a[0] > b[0] > c3[0]) and abs(d[0] - c3[0]) < 0.01 and abs(e[0] - a[0]) < 0.01
        if not (row and order):
            problems.append("pins 1-3 not along one side with 4 opposite 3 and 5 opposite 1")
        got_span = abs(d[1] - a[1])
        # 39cc07f: against TI's example layout, the pads' outer extent (centres plus a pad's length across the row): KiCad's
        # IPC-7351 pads run further in under the body than TI's, so their centres sit closer, and either solders
        plen = a[3]
        outer = got_span + plen
        if not span[0] + 1.1 <= outer <= span[1] + 1.1:
            problems.append(f"the rows' outer extent {outer:.2f} mm outside {span[0] + 1.1:.2f}-{span[1] + 1.1:.2f} "
                            "(TI's (2.6) centres plus its 1.1 mm pads, +-0.3)")
        notes.append(f"5 pads at {got:.3f} mm (e {pitch[0]}-{pitch[1]}); centres {got_span:.2f} apart, pads {plen:.2f} long, "
                     f"outer extent {outer:.2f} (TI's 3.70); 1-2-3 along one side, 4 opposite 3, 5 opposite 1")
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
    # 2026-10-28: 20 datasheets reached hardware/ref_design/datasheets; a part whose drawing is on disk is listed as not yet
    # compared by this script, not as BLOCKED
    on_disk = {"D2": "bat54c.pdf", "D3": "bat54ws.pdf", "J1": "hro_type-c-31-m-12.pdf", "J2": "molex_53261.pdf",
               "J3": "hirose_fh12.pdf", "J4": "jst_sh.pdf", "J8": "hirose_dm3_catalog_C114218.pdf", "Q1": "2n7002_cj.pdf",
               "Q3": "2n7002_cj.pdf", "Q5": "ao3400a.pdf", "Q6": "ao3400a.pdf", "RN1": "uniroyal_4d02.pdf",
               "RN2": "uniroyal_4d02.pdf", "RN3": "uniroyal_4d02.pdf", "RN4": "uniroyal_4d02.pdf", "TH1": "tctr0805.pdf",
               "U1": "srv05-4.pdf", "U10": "mcp6001.pdf", "U6": "mcp6001.pdf", "U7": "fm8002a.pdf"}
    have = {k: v for k, v in on_disk.items() if os.path.isfile(os.path.join(dn.REF, "datasheets", v))}
    print("not compared by this script, the drawing on disk (by hand, or a LAND row to add):")
    for k, refs in sorted(waiting.items(), key=lambda kv: kv[1][0]):
        if all(r in have for r in refs):
            print(f"  {', '.join(refs[:12])}: {have[refs[0]]}")
    print("BLOCKED, the maker's drawing not in hardware/ref_design/datasheets:")
    for k, refs in sorted(waiting.items(), key=lambda kv: kv[1][0]):
        if not all(r in have for r in refs):
            print(f"  {', '.join(refs[:12])}{' ...' if len(refs) > 12 else ''}: {k}")
    print("the SOT pin numbering (D2, Q1, Q3, Q5, Q6, U1, U3, D3) is pin_review's per-pin table, which VER-02 "
          "(check_pins.py) reads; U3's is checked above")
    print("drawings on disk but not compared by this script (by hand at VER-28's preview): J8 (hirose_dm3_catalog_C114218.pdf: "
          "its land and the two front shell tabs the build note calls unverified), SW24 (rocpu_tps850.pdf, 3.65 over all)")
    if bad:
        print("RESULT: FAIL")
        return 1
    print("RESULT: BLOCKED (every footprint with a drawing on disk passes)")
    return 2


if __name__ == "__main__":
    sys.exit(main())
