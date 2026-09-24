"""Does the populated board fit the case?

    python3 check_fit.py            # after board/tools/fab.py or any board change

Each part's 3D model is exported on its own (kicad-cli pcb export stl
--component-filter), so its real extent is known, and checked against what
the case allows at that spot:

  under the glass          top of part <= glass_limit above the board
  side bezels, seat bands  <= bezel_limit (the lid's underside)
  around the clusters      <= retainer_limit (the lid comes down to the cap flanges)
  under a silicone piece   nothing at all
  lid pin bosses           nothing within the boss diameter
  lid snap hooks           nothing taller than hook_clear under a hook
  J2 pocket                the battery connector may reach into the lid pocket
  edge parts               the USB-C face reaches the wall

Bottom-side parts (speaker pads, test pads) are flat and only checked
against the tub's bosses and ribs. A whole-board CGAL intersection in
OpenSCAD would be exact but takes hours on these models; boxes are enough for
parts this blocky.
"""
import json
import math
import os
import re
import struct
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
PCB = os.path.join(HERE, "..", "board", "battler.kicad_pcb")
POS = json.load(open(os.path.join(HERE, "positions.json")))
PCB_T = POS["board"]["t"]


def stl_points(path):
    data = open(path, "rb").read()
    if data[:5] == b"solid" and b"facet" in data[:400]:
        return [tuple(float(v) for v in m.groups()) for m in re.finditer(rb"vertex\s+(\S+)\s+(\S+)\s+(\S+)", data)]
    n = struct.unpack("<I", data[80:84])[0]
    pts = []
    for i in range(n):
        v = struct.unpack("<12f", data[84 + i * 50:84 + i * 50 + 48])
        pts += [v[3:6], v[6:9], v[9:12]]
    return pts


# Parts whose 3D model doesn't show their fitted height, and packages whose model
# this KiCad install lacks: height above the board in mm
KNOWN_HEIGHT = {}
PACKAGE_HEIGHT = {"SOIC-8": 1.75, "QFN": 1.0, "SON": 1.0, "DRC0010J": 1.0, "SOT-23": 1.2, "SOD-323": 1.1, "0402": 0.5, "0603": 0.9,
                  "1008": 1.2, "0806": 1.0}


def refs():
    import pcbnew
    b = pcbnew.LoadBoard(PCB)
    out = []
    for fp in b.GetFootprints():
        name = str(fp.GetFPID().GetLibItemName())
        if any(s in name for s in ("MountingHole", "Fiducial", "TestPoint", "DSL_button", "Speaker_Wire", "_Solder")):
            continue                                  # flat copper, nothing to collide (the panel's FPC lands on J3)
        cy = fp.GetCourtyard(pcbnew.B_CrtYd if fp.IsFlipped() else pcbnew.F_CrtYd).BBox()
        court = (pcbnew.ToMM(cy.GetX()), pcbnew.ToMM(cy.GetY()), pcbnew.ToMM(cy.GetRight()), pcbnew.ToMM(cy.GetBottom()))
        out.append((fp.GetReference(), fp.IsFlipped(), str(fp.GetFPID().GetLibItemName()), court))
    return out


def extent(ref, tmp):
    # one at a time: parallel kicad-cli exports clobber each other's temporary files
    out = os.path.join(tmp, ref + ".stl")
    r = subprocess.run(["kicad-cli", "pcb", "export", "stl", "--no-board-body", "--force", "--subst-models",
                        "--component-filter", ref, "-o", out, PCB], capture_output=True, text=True)
    if r.returncode or not os.path.exists(out):
        return ref, None
    pts = stl_points(out)
    if not pts:
        return ref, None
    xs, ys, zs = zip(*pts)
    # STL frame: KiCad X, -KiCad Y, z = 0 at the board's bottom face
    return ref, (min(xs), -max(ys), max(xs), -min(ys), min(zs) - PCB_T, max(zs) - PCB_T)


def inside_rect(box, rect, margin=0.0):
    (x0, y0), (x1, y1) = rect
    return box[2] > min(x0, x1) - margin and box[0] < max(x0, x1) + margin and \
        box[3] > min(y0, y1) - margin and box[1] < max(y0, y1) + margin


def near_circle(box, c, r):
    cx = min(max(c[0], box[0]), box[2])
    cy = min(max(c[1], box[1]), box[3])
    return math.hypot(cx - c[0], cy - c[1]) < r


def main():
    parts = refs()
    with tempfile.TemporaryDirectory() as tmp:
        boxes = {r[0]: extent(r[0], tmp)[1] for r in parts}
    flipped = {r[0]: r[1] for r in parts}
    problems, notes = [], []
    for ref, _, name, court in parts:
        if ref in KNOWN_HEIGHT and boxes.get(ref):
            b = boxes[ref]
            boxes[ref] = (b[0], b[1], b[2], b[3], 0.0, KNOWN_HEIGHT[ref])
            notes.append(f"{ref}: checked at {KNOWN_HEIGHT[ref]} mm")
        if boxes.get(ref) is None:
            h = next((v for k, v in PACKAGE_HEIGHT.items() if k in name), 1.5)
            boxes[ref] = (court[0], court[1], court[2], court[3], 0.0, h)
            notes.append(f"{ref}: no 3D model in this KiCad install, checked as its courtyard at {h} mm ({name})")
    glass = POS["glass"]
    for ref, box in sorted(boxes.items()):
        if box is None:
            notes.append(f"{ref}: no 3D model exported")
            continue
        top = box[5]
        if flipped[ref]:
            height = -box[4] - PCB_T          # below the board
            for s in POS["pins"]:
                if near_circle(box, s, POS["boss_d"] / 2 + 0.5):
                    problems.append(f"{ref} (back) sits on a tub pin boss")
            continue
        for i, c in enumerate(POS["pieces"]):
            if near_circle(box, c, POS["piece_d"] / 2):
                problems.append(f"{ref} is under a silicone piece (cluster {i})")
        for s in POS["pins"]:
            if near_circle(box, s, POS["boss_d"] / 2 + 0.3):
                problems.append(f"{ref} collides with the lid pin boss at {s}")
        for (hx0, hy0), (hx1, hy1) in POS["hooks"]:
            hr = [[min(hx0, hx1), min(hy0, hy1)], [max(hx0, hx1), max(hy0, hy1)]]
            overlap = box[0] < hr[1][0] and box[2] > hr[0][0] and box[1] < hr[1][1] and box[3] > hr[0][1]
            if overlap and top > POS["hook_clear"]:
                problems.append(f"{ref} is under a lid snap hook ({top:.2f} mm; the hook leaves {POS['hook_clear']:.2f})")
        pocket = POS["batt_conn"]
        in_pocket = abs((box[0] + box[2]) / 2 - pocket[0]) < POS["pocket"][0] / 2 and \
            abs((box[1] + box[3]) / 2 - pocket[1]) < POS["pocket"][1] / 2
        limit = POS["bezel_limit"] + (1.2 if in_pocket else 0)
        where = "bezel"
        if inside_rect(box, glass):
            limit, where = POS["glass_limit"], "glass"
        elif any(near_circle(box, c, POS["piece_d"] / 2 + 1.5) for c in POS["pieces"]):
            limit, where = min(limit, POS["retainer_limit"]), "retainer"
        if top > limit + 1e-3:
            problems.append(f"{ref} is {top:.2f} mm tall; the {where} allows {limit:.2f}")
        tl = POS["fpc_tail"]
        if inside_rect(box, tl, -0.2) and top > 0.3 and not ref.startswith("J3"):
            problems.append(f"{ref} is under the display's FPC tail")
    usb = boxes.get("J1")
    if usb and abs(usb[2] - POS["usb"]["front_x"]) > 0.3:
        problems.append(f"USB-C face at x={usb[2]:.2f}, the case expects {POS['usb']['front_x']:.2f}")
    report = ["Board-in-case check (part boxes against case/positions.json)", ""]
    report += [f"checked {sum(1 for b in boxes.values() if b)} parts"]
    report += [f"  PROBLEM: {p}" for p in problems] or ["  no collisions"]
    report += [f"  note: {n}" for n in notes]
    report.append("RESULT: " + ("PASS" if not problems else f"FAIL ({len(problems)})"))
    text = "\n".join(report)
    print(text)
    open(os.path.join(HERE, "fit_report.txt"), "w").write(text + "\n")
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()
