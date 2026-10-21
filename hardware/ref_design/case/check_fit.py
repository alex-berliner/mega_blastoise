"""Does the populated board fit the case?

    python3 check_fit.py            # after board/tools/fab.py or any board change

Each part's 3D model is exported on its own (kicad-cli pcb export stl
--component-filter), so its real extent is known, and checked against what
the case allows at that spot:

  under the glass          top of part <= glass_limit above the board
  side bezels, seat bands  <= bezel_limit (the lid's underside)
  around the clusters      <= retainer_limit (the lid comes down to the cap flanges)
  button decks             <= deck_limit (the ends step down so genuine caps stand proud)
  lid's stiffening ribs    <= rib_limit (beside the screen, under the lid)
  the board's back         <= back_limit (the board rests on ribs; the cell and speaker lie under it)
  under a silicone piece   nothing at all (the clusters and the Start/Select membranes)
  lid pin bosses           nothing within the boss diameter
  lid snap hooks           nothing taller than hook_clear under a hook
  lid's panel frame        its band round the panel: <= frame limit (it hangs lower)
  J2 pocket                the battery connector may reach into the lid pocket
  edge parts               the USB-C face reaches the wall
  through-hole leads       every lead poking out of the board's back lands where the floor's
                           ribs are cut away (rib_free), not on a rib

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
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "tools"))
import scad  # noqa: E402  (scad.jobs(): how many exports at once)
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
                  "1008": 1.2, "0806": 1.0, "SolderJumper": 0.05}


def refs():
    import pcbnew
    b = pcbnew.LoadBoard(PCB)
    out = []
    for fp in b.GetFootprints():
        name = str(fp.GetFPID().GetLibItemName())
        if any(s in name for s in ("MountingHole", "Fiducial", "TestPoint", "DSL_button", "Speaker_Wire", "_Solder", "Pogo_Pads")):
            continue                                  # flat copper, nothing to collide
        cy = fp.GetCourtyard(pcbnew.B_CrtYd if fp.IsFlipped() else pcbnew.F_CrtYd).BBox()
        court = (pcbnew.ToMM(cy.GetX()), pcbnew.ToMM(cy.GetY()), pcbnew.ToMM(cy.GetRight()), pcbnew.ToMM(cy.GetBottom()))
        out.append((fp.GetReference(), fp.IsFlipped(), str(fp.GetFPID().GetLibItemName()), court))
    return out


def tht_leads():
    """Leads that poke out of the board's back: plated holes of parts, not mounting holes or vias in pads."""
    import pcbnew
    b = pcbnew.LoadBoard(PCB)
    out = []
    for fp in b.GetFootprints():
        if fp.GetReference().startswith("H"):
            continue
        for pad in fp.Pads():
            if pad.HasHole() and max(pcbnew.ToMM(pad.GetDrillSizeX()), pcbnew.ToMM(pad.GetDrillSizeY())) > 0.45:
                p = pad.GetPosition()
                out.append((fp.GetReference(), pcbnew.ToMM(p.x), pcbnew.ToMM(p.y)))
    return out


def extent(ref, tmp):
    # in tmp: kicad-cli writes its scratch file ($tempfile$.stl) into its working directory, so
    # exports running at once each need a folder of their own (extents() gives them one)
    out = os.path.join(tmp, ref + ".stl")
    r = subprocess.run(["kicad-cli", "pcb", "export", "stl", "--no-board-body", "--force", "--subst-models",
                        "--component-filter", ref, "-o", out, PCB], capture_output=True, text=True, cwd=tmp)
    if r.returncode or not os.path.exists(out):
        return ref, None
    pts = stl_points(out)
    if not pts:
        return ref, None
    xs, ys, zs = zip(*pts)
    # STL frame: KiCad X, -KiCad Y, z = 0 at the board's bottom face
    return ref, (min(xs), -max(ys), max(xs), -min(ys), min(zs) - PCB_T, max(zs) - PCB_T)


def extents(refs):
    """{ref: extent(ref)[1]} for every ref, several exports at once, each in its own folder."""
    with tempfile.TemporaryDirectory() as tmp, ThreadPoolExecutor(scad.jobs(cpu_only=True)) as pool:
        return dict(pool.map(lambda ref: extent(ref, tempfile.mkdtemp(dir=tmp)), refs))


def inside_rect(box, rect, margin=0.0):
    (x0, y0), (x1, y1) = rect
    return box[2] > min(x0, x1) - margin and box[0] < max(x0, x1) + margin and \
        box[3] > min(y0, y1) - margin and box[1] < max(y0, y1) + margin


def within(box, rect):
    (x0, y0), (x1, y1) = rect
    return box[0] >= min(x0, x1) and box[2] <= max(x0, x1) and box[1] >= min(y0, y1) and box[3] <= max(y0, y1)


def near_circle(box, c, r):
    cx = min(max(c[0], box[0]), box[2])
    cy = min(max(c[1], box[1]), box[3])
    return math.hypot(cx - c[0], cy - c[1]) < r


def main():
    parts = refs()
    boxes = extents([r[0] for r in parts])
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
            if height > POS["back_limit"] - 0.1:
                problems.append(f"{ref} (back) is {height:.2f} mm tall; the ribs, cell and speaker leave {POS['back_limit']} mm")
            continue
        for i, c in enumerate(POS["pieces"]):
            if near_circle(box, c, POS["piece_d"] / 2):
                problems.append(f"{ref} is under a silicone piece (cluster {i})")
        for i, r in enumerate(POS["ss_pieces"]):
            if inside_rect(box, r):
                problems.append(f"{ref} is under a Start/Select membrane (seat {i + 1})")
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
        limit = POS["pocket_limit"] - 0.2 if in_pocket else POS["bezel_limit"]
        where = "bezel"
        if inside_rect(box, glass):
            limit, where = POS["glass_limit"], "glass"
        elif any(near_circle(box, c, POS["piece_d"] / 2 + 1.5) for c in POS["pieces"]) or \
                any(inside_rect(box, r, 1.5) for r in POS["ss_pieces"]):
            limit, where = min(limit, POS["retainer_limit"]), "retainer"
        if any(inside_rect(box, d) for d in POS["decks"]) and POS["deck_limit"] < limit:
            limit, where = POS["deck_limit"], "button deck"      # the stepped-down ends
        if any(inside_rect(box, r) for r in POS["lid_ribs"]) and POS["rib_limit"] < limit:
            limit, where = POS["rib_limit"], "lid's stiffening rib"
        frame = POS["frame"]                    # the lid's panel frame hangs lower than the rest of the lid
        if inside_rect(box, frame["outer"]) and not within(box, frame["inner"]) and frame["limit"] < limit:
            limit, where = frame["limit"], "lid's panel frame"
        if top > limit + 1e-3:
            problems.append(f"{ref} is {top:.2f} mm tall; the {where} allows {limit:.2f}")
        for zone in POS["fpc_zones"]:
            if inside_rect(box, zone, -0.2) and top > 0.3 and not ref.startswith("J3"):
                problems.append(f"{ref} is under the display's folded FPC")
    for ref, x, y in tht_leads():
        if not any(min(a[0], b[0]) <= x <= max(a[0], b[0]) and min(a[1], b[1]) <= y <= max(a[1], b[1])
                   for a, b in POS["rib_free"]):
            problems.append(f"{ref}'s through-hole lead at ({x:.2f}, {y:.2f}) lands on a floor rib")
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
