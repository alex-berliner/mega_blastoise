#!/usr/bin/env python3
"""The contact footprints as placed on battler.kicad_pcb (VER-162).

    python3 hardware/vv/tools/check_placed_contacts.py          # exit 1 on any failure
    python3 hardware/vv/tools/check_placed_contacts.py --seed   # moves SW7's pad 2 by 0.1 mm in memory: must exit 1

For every membrane contact on the board (SW1-SW16, DSL_button_4ring) and every dome contact (SW20-SW23,
Dome_4mm_Round) it takes the footprint as the board holds it, turns it back to 0 degrees at the origin,
and:
  1. compares every pad (number, shape, size, position, angle, layers, and each custom pad's primitives)
     with the library footprint in board/lib/battler.pretty, within 1 um: the board carries the library's
     copper, not an older copy;
  2. saves it to a scratch .kicad_mod and runs hardware/membrane/tools/check_contact_fp.py on it for the
     4-ring contacts (copper gap >= 0.20 mm, bridging spot <= 1.60 mm, mask covers the comb, each pad one piece);
  3. checks that no contact pad has paste and that no silk crosses a contact's mask opening.
The dome's own rule (the dome's rim and dimple against the pads) is VER-178's: its drawing is not on disk.

Python 3 standard library and pcbnew.
"""
import math
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import design_numbers as dn  # noqa: E402

LIB = os.path.join(dn.BOARD, "lib", "battler.pretty")
CHECK_FP = os.path.join(dn.ROOT, "hardware", "membrane", "tools", "check_contact_fp.py")
TOL = 0.001                      # mm


def pad_sig(pcb, pad):
    mm = pcb.ToMM
    prims = []
    lay = pcb.F_Cu if pad.IsOnLayer(pcb.F_Cu) else pcb.B_Cu
    poly = pad.GetEffectivePolygon(lay, pcb.ERROR_INSIDE)     # the copper as KiCad fills it, custom shapes included
    for i in range(poly.OutlineCount()):
        o = poly.Outline(i)
        prims.append((round(o.Area() / 1e12, 4),                         # nm^2 to mm^2
                      tuple(sorted((round(mm(o.CPoint(k).x), 3), round(mm(o.CPoint(k).y), 3))
                                   for k in range(o.PointCount())))))
    return dict(shape=int(pad.GetShape()), size=(mm(pad.GetSize().x), mm(pad.GetSize().y)),
                pos=(mm(pad.GetPosition().x), mm(pad.GetPosition().y)), angle=pad.GetOrientationDegrees() % 360,
                layers=pad.GetLayerSet().FmtHex(), prims=sorted(prims))


def normalised(pcb, fp):
    f = pcb.FOOTPRINT(fp)
    if f.IsFlipped():
        f.Flip(f.GetPosition(), pcb.FLIP_DIRECTION_TOP_BOTTOM)
    f.SetOrientationDegrees(0)
    f.SetPosition(pcb.VECTOR2I(0, 0))
    return f


def diff(a, b):
    out = []
    for k in ("shape", "layers"):
        if a[k] != b[k]:
            out.append(f"{k} {a[k]} != {b[k]}")
    for k in ("size", "pos"):
        if max(abs(x - y) for x, y in zip(a[k], b[k])) > TOL:
            out.append(f"{k} {tuple(round(x, 3) for x in a[k])} != {tuple(round(x, 3) for x in b[k])}")
    if abs((a["angle"] - b["angle"] + 180) % 360 - 180) > 0.01:
        out.append(f"angle {a['angle']} != {b['angle']}")
    if a["prims"] != b["prims"]:
        out.append("custom primitives differ")
    return out


def main():
    pcb, b = dn.board()
    parts, _ = dn.design()
    seed = "--seed" in sys.argv
    bad = 0
    tmp = tempfile.mkdtemp(prefix="vv_contacts_") + "/placed.pretty"
    io = pcb.PCB_IO_KICAD_SEXPR()
    io.CreateLibrary(tmp)
    fps = {f.GetReference(): f for f in b.GetFootprints()}
    refs = sorted((r for r, p in parts.items() if p["fp"] in ("battler:DSL_button_4ring", "battler:Dome_4mm_Round")),
                  key=lambda r: int(r[2:]))
    lib_fp = {}
    for ref in refs:
        name = parts[ref]["fp"].split(":")[1]
        if name not in lib_fp:
            lib_fp[name] = pcb.FootprintLoad(LIB, name)
        placed = normalised(pcb, fps[ref])
        if seed and ref == "SW7":
            p2 = next(p for p in placed.Pads() if p.GetNumber() == "2")
            p2.SetPosition(p2.GetPosition() + pcb.VECTOR2I(pcb.FromMM(0.1), 0))
        lp = {(p.GetNumber(), i): pad_sig(pcb, p) for i, p in enumerate(lib_fp[name].Pads())}
        pp = {(p.GetNumber(), i): pad_sig(pcb, p) for i, p in enumerate(placed.Pads())}
        problems = []
        if set(lp) != set(pp):
            problems.append(f"pads {sorted(pp)} != library {sorted(lp)}")
        for k in sorted(set(lp) & set(pp)):
            for d in diff(pp[k], lp[k]):
                problems.append(f"pad {k[0] or 'NPTH'}: {d}")
        paste = [p.GetNumber() for p in placed.Pads() if p.IsOnLayer(pcb.F_Paste) or p.IsOnLayer(pcb.B_Paste)]
        if paste:
            problems.append(f"paste on pads {paste}")
        line = f"{ref} {name} at ({pcb.ToMM(fps[ref].GetPosition().x):.2f}, {pcb.ToMM(fps[ref].GetPosition().y):.2f}) " \
               f"{fps[ref].GetOrientationDegrees():.0f} deg, {'B' if fps[ref].IsFlipped() else 'F'}: "
        if name == "DSL_button_4ring":
            placed.SetFPID(pcb.LIB_ID("", f"{ref}_placed"))
            io.FootprintSave(tmp, placed)
            path = os.path.join(tmp, f"{ref}_placed.kicad_mod")
            r = subprocess.run([sys.executable, CHECK_FP, path], capture_output=True, text=True)
            tail = [x for x in r.stdout.splitlines() if "min copper gap" in x or "smallest contact spot" in x
                    or "mask opening" in x]
            if r.returncode != 0:
                problems.append("check_contact_fp failed: " + " | ".join(tail))
            line += " | ".join(t.strip() for t in tail) + "; "
        # silk over the contact: any silk item of the board whose box meets the mask opening's circle
        cx, cy = pcb.ToMM(fps[ref].GetPosition().x), pcb.ToMM(fps[ref].GetPosition().y)
        rmask = max(pcb.ToMM(max(p.GetBoundingBox().GetWidth(), p.GetBoundingBox().GetHeight())) / 2
                    for p in fps[ref].Pads() if p.GetNumber())
        silk = []
        for d in list(b.GetDrawings()) + [g for f in b.GetFootprints() for g in f.GraphicalItems()] + \
                [t for f in b.GetFootprints() for t in (f.Reference(), f.Value()) if t.IsVisible()]:
            if d.GetLayer() not in (pcb.F_SilkS,) or (hasattr(d, "GetParentFootprint") and d.GetParentFootprint() is fps[ref]
                                                     and d.GetClass() != "PCB_FIELD"):
                continue
            bb = d.GetBoundingBox()
            x0, y0, x1, y1 = (pcb.ToMM(bb.GetLeft()), pcb.ToMM(bb.GetTop()), pcb.ToMM(bb.GetRight()), pcb.ToMM(bb.GetBottom()))
            nx, ny = min(max(cx, x0), x1), min(max(cy, y0), y1)
            if math.hypot(nx - cx, ny - cy) < rmask:
                silk.append(d.GetClass())
        if silk and not fps[ref].IsFlipped():
            problems.append(f"silk over the contact: {silk}")
        line += "same pads as the library, no paste" if not problems else "; ".join(problems)
        print(("  ok    " if not problems else "  FAIL  ") + line)
        bad += bool(problems)
    print(f"{len(refs)} contacts, {bad} failing; the dome footprint's rule against the dome's drawing is VER-178 "
          "(BLOCKED: the HYP 600-415S-260 drawing, LCSC C256258, is not on disk)")
    print("RESULT: " + ("PASS" if not bad else "FAIL"))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
