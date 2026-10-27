"""Does the populated board fit the case?

    python3 check_fit.py            # after board/tools/fab.py or any board change

Each part's 3D model is exported on its own (kicad-cli pcb export stl
--component-filter), so its real extent is known, and checked against what
the case allows at that spot:

  under the glass          top of part <= glass_limit above the board
  side bezels, the band    <= bezel_limit (the top's underside at the screen's height)
  around the clusters      <= retainer_limit (the top comes down to the cap flanges)
  button decks             <= deck_limit (the ends step down so genuine caps stand proud; the
                              band under the slope's wall too)
  the board's back         <= back_limit (the board rests on the tub's floor ribs)
  under a silicone piece   nothing at all (the clusters and the Start/Select membranes)
  the cell's hole          nothing reaching over it
  the speaker              nothing under it or its keep-out (it lies on the board's top: a racetrack
                           of speaker.size, plus speaker.keep)
  screw bosses             nothing within the boss diameter (four beside the glass, one at each end)
  crush ribs               nothing within 1 mm of the board's edge at a rib
  top's panel frame        its band round the panel: <= frame limit (it hangs lower)
  the ends' strip          the strip between the clusters under the band's end, which positions.json's
                           decks zones leave out and where the top has the ends' ceiling: <= the ends'
                           limit, less U6, which has a pocket of its own (case.scad's TOP echo line,
                           2026-10-25; the island's joint zones and the arms' hooks are gone)
  folded FPC, J3's range   nothing but J3 under the panel's FPC, over the whole range J3 may move
  edge parts               the USB-C face reaches the wall
  USB-C opening            J1's mouth (its model within 1 mm of its face) centred in the wall's
                           opening (case.scad's USBC echo line) with room round it, and a mated
                           plug's overmold (the shortest plug the USB-IF allows) clear of the wall
  through-hole leads       every lead poking out of the board's back lands where the floor's
                           ribs are cut away (rib_free), not on a rib

Bottom-side parts (test pads, the SWD pogo pads) are flat and only checked
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
MOUTH = {}                                      # J1's mouth: its model's extent within 1 mm of its face
USB_ROOM, USB_OFF = 0.15, 0.1                   # the opening round the mouth, at least; its centre off the mouth's, at most


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
        if any(s in name for s in ("MountingHole", "Fiducial", "TestPoint", "DSL_button", "Dome_", "Speaker_Wire", "_Solder", "Pogo_Pads")):
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
    if ref == "J1":
        front = [p for p in pts if p[0] > max(xs) - 1.0]
        fx, fy, fz = zip(*front)
        MOUTH[ref] = (min(fx), -max(fy), max(fx), -min(fy), min(fz) - PCB_T, max(fz) - PCB_T)
    # STL frame: KiCad X, -KiCad Y, z = 0 at the board's bottom face
    return ref, (min(xs), -max(ys), max(xs), -min(ys), min(zs) - PCB_T, max(zs) - PCB_T)


def extents(refs):
    """{ref: extent(ref)[1]} for every ref, several exports at once, each in its own folder."""
    with tempfile.TemporaryDirectory() as tmp, ThreadPoolExecutor(scad.jobs(cpu_only=True)) as pool:
        return dict(pool.map(lambda ref: extent(ref, tempfile.mkdtemp(dir=tmp)), refs))


def case_echo():
    """case.scad's lines for this check that aren't in positions.json: {"USBC": the USB-C opening and the
    plug it is checked with, "TOP": the one-piece top's lower ceilings}, each None if missing."""
    with tempfile.TemporaryDirectory() as tmp:
        out = os.path.join(tmp, "case.echo")
        subprocess.run([*scad.cmd(out), "-o", out, "-D", 'part="none"', os.path.join(HERE, "case.scad")],
                       capture_output=True, text=True)
        text = open(out).read() if os.path.exists(out) else ""
    found = {}
    for key in ("USBC", "TOP", "ISLAND"):
        m = re.search(r'ECHO: "' + key + r' (\{.*\})"', text)
        found[key] = json.loads(m.group(1).replace('\\"', '"')) if m else None
    return found


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


def island_ceiling(isl, tops, x, y):
    """The inside's ceiling at a point (case frame): case.scad's uni_ceil (the island's slope, the ears, the ends)."""
    def rect_d(r):
        return math.hypot(max(0, r[0] - x, x - r[2]), max(0, r[1] - y, y - r[3]))
    ex, ey = isl["edge"]
    t = rect_d((-ex, -ey, ex, ey))
    for side, y0, y1 in isl["ears"]:
        t = min(t, rect_d((isl["ear_xa"], y0, 99, y1) if side > 0 else (-99, y0, -isl["ear_xa"], y1)))
    if t >= isl["run"]:
        return isl["z_deck_under"]
    rs = isl["r"] + isl["skin"]
    sn = (isl["run"] - t) / rs
    lim = isl["deck_face"] + isl["r"] - rs * math.sqrt(1 - sn * sn)
    if sn >= math.sin(math.radians(isl["phi"])):
        lim = 99
    return min(isl["z_ceil"], lim)


def box_ceiling(isl, ceilings, box):
    """The least ceiling over a box's footprint (KiCad frame box), its pocket (U6, J5) lifting it where there is one."""
    best = 99
    n = 6
    for i in range(n + 1):
        for j in range(n + 1):
            kx = box[0] + (box[2] - box[0]) * i / n
            ky = box[1] + (box[3] - box[1]) * j / n
            x, y = kx - 100, 100 - ky
            z = island_ceiling(isl, None, x, y)
            for p in ceilings["pockets"]:
                if min(p[0][0], p[1][0]) <= kx <= max(p[0][0], p[1][0]) and min(p[0][1], p[1][1]) <= ky <= max(p[0][1], p[1][1]):
                    z = max(z, isl["deck_face"] - isl["pocket_skin"])
            best = min(best, z)
    return best


def near_speaker(box):
    """Does the box reach into the speaker's racetrack (along y, KiCad frame) grown by its keep-out?"""
    sp = POS["speaker"]
    (cx, cy), (w, l), keep = sp["c"], sp["size"], sp["keep"]
    r, half = w / 2 + keep, (l - w) / 2
    return near_circle(box, (cx, cy - half), r) or near_circle(box, (cx, cy + half), r) or \
        inside_rect(box, ((cx - r, cy - half), (cx + r, cy + half)))


def check_part(ref, box, is_flipped, echoed, problems, under):
    """One part's box against the case (appends to problems, and to under for the ends' strip)."""
    ceilings = echoed["TOP"]
    glass = POS["glass"]
    top = box[5]
    if is_flipped:
        height = -box[4] - PCB_T          # below the board
        for s in POS["pins"]:
            if near_circle(box, s, POS["boss_d"] / 2 + 0.5):
                problems.append(f"{ref} (back) sits on a tub pin boss")
        if height > POS["back_limit"] - 0.1:
            problems.append(f"{ref} (back) is {height:.2f} mm tall; the floor's ribs leave {POS['back_limit']} mm")
        return
    for i, c in enumerate(POS["pieces"]):
        if near_circle(box, c, POS["piece_d"] / 2):
            problems.append(f"{ref} is under a silicone piece (cluster {i})")
    for i, r in enumerate(POS["ss_pieces"]):
        if inside_rect(box, r):
            problems.append(f"{ref} is under a Start/Select membrane (seat {i + 1})")
    for s in POS["pins"]:
        if near_circle(box, s, POS["boss_d"] / 2 + 0.3):
            problems.append(f"{ref} collides with the screw boss at {s}")
    for c in POS["crush"]:
        if near_circle(box, c, 1.0):
            problems.append(f"{ref} is at a crush rib at {c}")
    if inside_rect(box, POS["cell_hole"]):
        problems.append(f"{ref} reaches over the cell's hole")
    if near_speaker(box):
        problems.append(f"{ref} is in the speaker's hole (or within its keep-out)")
    isl = echoed["ISLAND"]
    limit, where = box_ceiling(isl, ceilings, box) - isl["z_top_cu"] - 0.3, "island's slope, ear or the ends' ceiling"
    if inside_rect(box, glass):
        limit, where = POS["glass_limit"], "glass"
    elif any(near_circle(box, c, POS["piece_d"] / 2 + 1.5) for c in POS["pieces"]) or \
            any(inside_rect(box, r, 1.5) for r in POS["ss_pieces"]):
        limit, where = min(limit, POS["retainer_limit"]), "retainer"
    if any(inside_rect(box, d) for d in POS["decks"]) and POS["deck_limit"] < limit:
        limit, where = POS["deck_limit"], "button deck"      # the stepped-down ends
        if ceilings and any(within(box, p) for p in ceilings["pockets"]):
            limit, where = ceilings["pocket_limit"], "button deck, in a pocket"
    frame = POS["frame"]                    # the top's panel frame hangs lower than the rest of it
    notched = any(inside_rect(box, n) for n in frame["notches"])
    if inside_rect(box, frame["outer"]) and not within(box, frame["inner"]) and frame["limit"] < limit and not notched:
        limit, where = frame["limit"], "top's panel frame"
    if top > limit + 1e-3:
        problems.append(f"{ref} is {top:.2f} mm tall; the {where} allows {limit:.2f}")
    if ceilings and any(inside_rect(box, z) for z in ceilings["strips"]):      # the strip between the clusters
        pocket = any(within(box, p) for p in ceilings["pockets"])               # (U6 has its pocket)
        cap = ceilings["pocket_limit"] if pocket else ceilings["limit"]
        under.append((ref, top))
        if top > cap + 1e-3:
            problems.append(f"{ref} is {top:.2f} mm tall in the strip between the clusters under the top's ends, which allows {cap:.2f}"
                            + (" (in a pocket)" if pocket else ""))
    tail, body = POS["fpc_zones"]
    if ref.startswith("J3"):
        pass
    elif inside_rect(box, tail, -0.2) and top > 0.3:
        problems.append(f"{ref} is under the display's folded FPC (its tail: nothing but J3)")
    elif inside_rect(box, body, -0.2):
        neck = inside_rect(box, ((body[0][0], POS["fpc_neck_y"]), (body[1][0], body[1][1])), -0.2)
        if neck and top > 0.3:
            problems.append(f"{ref} is under the display's FPC neck (the fold round the glass's edge)")
        elif top > POS["fpc_low"] + 1e-3:
            problems.append(f"{ref} is {top:.2f} mm tall under the display's folded FPC, which lies on the glass's back: {POS['fpc_low']} at most")
    if ref.startswith("J3") and not within(box, POS["j3_zone"]):
        problems.append(f"J3 lies outside the range the case keeps for it ({POS['j3_zone']})")


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
    echoed, under = case_echo(), []
    ceilings = echoed["TOP"]
    if not ceilings:
        problems.append("no TOP line in case.scad's echo to check the one-piece top's ceilings with")
    for ref, box in sorted(boxes.items()):
        if box is None:
            notes.append(f"{ref}: no 3D model exported")
            continue
        check_part(ref, box, flipped[ref], echoed, problems, under)
    if ceilings:
        notes.append(f"the strip between the clusters under the top's ends: {len({u[0] for u in under})} parts in it, the tallest "
                     f"{max((u[1] for u in under), default=0):.2f} (it allows {ceilings['limit']:.2f}; {ceilings['pocket_limit']:.2f} in a pocket)")
    for ref, x, y in tht_leads():
        if not any(min(a[0], b[0]) <= x <= max(a[0], b[0]) and min(a[1], b[1]) <= y <= max(a[1], b[1])
                   for a, b in POS["rib_free"]):
            problems.append(f"{ref}'s through-hole lead at ({x:.2f}, {y:.2f}) lands on a floor rib")
    usb = boxes.get("J1")
    if usb and abs(usb[2] - POS["usb"]["front_x"]) > 0.3:
        problems.append(f"USB-C face at x={usb[2]:.2f}, the case expects {POS['usb']['front_x']:.2f}")
    mouth, hole = MOUTH.get("J1"), echoed["USBC"]
    if not mouth or not hole:
        problems.append("USB-C: no " + ("J1 model" if not mouth else "USBC line in case.scad's echo") + " to check the opening with")
    else:
        (y0, y1), (z0, z1) = hole["hole_y"], hole["hole_z"]
        room = {"-y": mouth[1] - y0, "+y": y1 - mouth[3], "-z": mouth[4] - z0, "+z": z1 - mouth[5]}
        off = ((mouth[1] + mouth[3] - y0 - y1) / 2, (mouth[4] + mouth[5] - z0 - z1) / 2)
        # mated, the plug's tip is hole["mate"] inside J1's face and its overmold plug_len_min out from the tip
        stand = mouth[2] - hole["mate"] + hole["plug_len_min"] - hole["wall_x"][1]
        if min(room.values()) < USB_ROOM:
            problems.append(f"USB-C opening leaves {min(room.values()):.2f} round J1's mouth ({', '.join(f'{k} {v:.2f}' for k, v in room.items())}), "
                            f"needs {USB_ROOM}")
        if max(map(abs, off)) > USB_OFF:
            problems.append(f"USB-C opening is off J1's mouth by y {off[0]:.2f}, z {off[1]:.2f}")
        if not hole["wall_x"][0] <= mouth[2] <= hole["wall_x"][1]:
            problems.append(f"J1's face at x={mouth[2]:.2f} is not in the wall ({hole['wall_x']})")
        if stand < 0:
            problems.append(f"a mated USB-C plug's overmold runs {-stand:.2f} into the wall")
        notes.append(f"USB-C: J1's mouth {mouth[3] - mouth[1]:.2f} x {mouth[5] - mouth[4]:.2f}, the opening round it "
                     f"{', '.join(f'{k} {v:.2f}' for k, v in room.items())}, off centre y {off[0]:.2f} z {off[1]:.2f}; "
                     f"a mated plug's overmold {stand:.2f} off the wall")
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
