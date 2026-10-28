#!/usr/bin/env python3
"""Case-side checks of classes 2, 13 and 14, from case.scad, its echo block and the routed board.

    python3 hardware/vv/tools/case_checks.py <check>           # exit 1 when the check's limit fails, 2 BLOCKED
    python3 hardware/vv/tools/case_checks.py <check> --seed    # the same check on a seeded fault: must exit 1
    python3 hardware/vv/tools/case_checks.py --list

  arch      VER-190  each case-side requirement mapped to the case.scad feature and the nominal number that
                     meets it; each part of assembly.md's order named with what holds it in the closed case

positions() is case/positions.json as case/export.py makes it (the JSON line of case.scad's echo block), but
run into a temp dir: positions.json is gitignored and nothing is written under hardware/ref_design. scad() is
design_numbers.scad(): case.scad's top-level assignments evaluated in Python.

Python 3 standard library, the OpenSCAD tools/scad.py names, and pcbnew for the checks that read the board.
"""
import json
import math
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import design_numbers as dn  # noqa: E402

REF = dn.REF
CASE = os.path.join(REF, "case")
REQS = os.path.join(os.path.dirname(HERE), "requirements.md")


def echo(seed_defs=()):
    """(positions dict, the other echo lines) from case.scad's echo block (part 'none', about half a second)."""
    sys.path.insert(0, os.path.join(REF, "tools"))
    import scad  # noqa: E402  (ref_design/tools/scad.py: which OpenSCAD, which backend)
    with tempfile.TemporaryDirectory() as tmp:
        out = os.path.join(tmp, "case.echo")
        defs = [a for k, v in seed_defs for a in ("-D", f"{k}={v}")]
        r = subprocess.run([*scad.cmd(out), "-o", out, "-D", 'part="none"', *defs, os.path.join(CASE, "case.scad")],
                           capture_output=True, text=True)
        text = open(out, encoding="utf-8").read() if os.path.exists(out) else ""
    m = re.search(r'ECHO: "JSON (\{.*\})"', text)
    if not m:
        if seed_defs:                        # a nudged value case.scad's asserts refuse: the caller says so
            return None, (r.stderr or "")[-300:].splitlines()
        print(f"BLOCKED: openscad failed: {r.stderr[-400:]}\nRESULT: BLOCKED")
        sys.exit(2)
    pos = json.loads(m.group(1).replace('\\"', '"'))
    return pos, [ln for ln in text.splitlines() if "JSON" not in ln]


def echo_json(lines, tag):
    for ln in lines:
        m = re.match(r'ECHO: "' + tag + r' (\{.*\})"', ln)
        if m:
            return json.loads(m.group(1).replace('\\"', '"'))
    return None


class Rows:
    def __init__(self):
        self.bad = []

    def __call__(self, good, text):
        print(("  ok    " if good else "  FAIL  ") + text)
        if not good:
            self.bad.append(text)
        return good


# ---- VER-190: the case's architecture against its requirements ----------------------------------------------------
# the case-side requirements (VER-190's procedure): REQ-MEC-*, REQ-PRD-06, REQ-ASM-*, REQ-BAT-07, REQ-BAT-08 and the
# case items of REQ-BTN-* (05 feel, 06 registration, 08 the domes; 01-04 and 07 are the board's copper and contacts)
CASE_REQ = re.compile(r"^### (REQ-(?:MEC-\d+|PRD-06|ASM-\d+(?:\.R)?|BAT-0[78]|BTN-0[568]))\b", re.M)
SEED_ARCH = {"cell_pocket_box": [[-17.0, -5.9], [17.0, 46.1]]}      # a pocket narrower than a nominal cell plus its fit


def arch(seed=None):
    e = dn.scad()
    if seed:
        e.update(seed)
    pos, lines = echo()
    usbc = echo_json(lines, "USBC") or {}
    stack = next((ln for ln in lines if "STACK" in ln), "")
    row = Rows()
    pocket = e["cell_pocket_box"]
    pw, pl = pocket[1][0] - pocket[0][0], pocket[1][1] - pocket[0][1]
    nom = e["cell_nom"]
    stack_sum = sum(float(x) for x in re.findall(r"\d+(?:\.\d+)?", stack.split("=")[0])) if stack else 0.0
    side_eng = e["screw_cb"][1] + e["screw_l"] - e["z_top_cu"]
    # (requirement, the case's feature, its nominal number, meets it, the check that holds the worst case)
    m = [
        ("REQ-PRD-06", "outline2d (case_w, case_l), case_h, deck_face",
         f"{e['case_w']:.2f} x {e['case_l']:.2f} x {e['case_h']:.2f} mm, ends {e['deck_face']:.2f} against 77.4 x 156.64 x 9.6, 7.7",
         e["case_w"] <= 77.4 + 1e-6 and e["case_l"] <= 156.64 + 1e-6 and e["case_h"] <= 9.6 + 1e-6 and e["deck_face"] <= 7.7 + 1e-6,
         "VER-20, VER-48"),
        ("REQ-BAT-07", "glass_gap over the board, cell_top (the worst cell's top over the board)",
         f"{e['glass_gap'] - e['cell_top']:.2f} mm free above the cell under the panel against 0.4",
         e["glass_gap"] - e["cell_top"] >= 0.4, "VER-182 (the slope's ceiling too), VER-20"),
        ("REQ-BAT-08", "cell_pocket_box in the tub's floor, cell_hole2d in the board",
         f"pocket {pw:.2f} x {pl:.2f} mm against a nominal cell {nom[1]} x {nom[0]} plus 0.4 (the fit clause, PROPOSED)",
         pw >= nom[1] + 0.4 - 1e-6 and pl >= nom[0] + 0.4 - 1e-6, "VER-181 at the worst-case cell (FAIL: DEF-62)"),
        ("REQ-MEC-01", "positions.json's height zones (glass_limit, bezel_limit, retainer_limit, deck_limit, back_limit)",
         f"glass {pos['glass_limit']}, bezel {pos['bezel_limit']}, retainer {pos['retainer_limit']}, decks {pos['deck_limit']}, "
         f"back {pos['back_limit']} mm, every zone named and positive",
         all(pos[k] > 0 for k in ("glass_limit", "bezel_limit", "retainer_limit", "deck_limit", "back_limit")),
         "VER-06 (check_fit), VER-112"),
        ("REQ-MEC-02", "the STACK echo line: floor, ribs, PCB, under the panel, panel, tape, top over the panel",
         f"{stack_sum:.2f} mm against case_h {e['case_h']:.2f}: closes at nominal ({stack.split(':')[-1].strip()[:60] if stack else 'no STACK line'}...)",
         abs(stack_sum - e["case_h"]) <= 0.01, "VER-20 (tolerances)"),
        ("REQ-MEC-03", "caps in the ends' pockets over the silicone: cap_proud, dome_travel, dpad_gap",
         f"cap stands {e['cap_proud']:.2f} mm proud, travels {e['dome_travel']:.2f}: {e['cap_proud'] - e['dome_travel']:.2f} left pressed "
         f"(case.scad wants 0.4); D-pad gap {e['dpad_gap']:.2f} in 0.3-1.0",
         e["cap_proud"] - e["dome_travel"] >= 0.4 and 0.3 <= e["dpad_gap"] <= 1.0, "VER-07, VER-48"),
        ("REQ-MEC-04", "the board's USB-C tab into the tub's notch (usb_tab, usb_front at +x only); abxy_square on abxy_boss",
         f"tab {e['usb_tab']:.1f} x {e['usb_tab_w']:.1f} mm at x = +{e['board_w'] / 2:.1f} only; ABXY slot {e['abxy_square'][0]:.2f} "
         f"over the cap's {e['abxy_boss'][0]:.2f} boss",
         e["usb_front"] > 0 and e["abxy_square"][0] >= e["abxy_boss"][0] + 0.1, "VER-48"),
        ("REQ-MEC-05", "six M2 screws (positions.json pins), the tongue (deck_tongue_h), keys, grip ribs",
         f"{len(pos['pins'])} screws; tongue {e['deck_tongue_h']:.2f} mm (case.scad wants 2.5)",
         len(pos["pins"]) == 6 and e["deck_tongue_h"] >= 2.5, "VER-92 (DVT)"),
        ("REQ-MEC-06", "cell_pocket walls (cell_fit_side, cell_fit_lead); the panel frame (frame_wall); J3's latch",
         f"pocket {e['cell_pocket']:.2f} mm deep in the floor with walls {e['cell_fit_side']:.2f}/{e['cell_fit_lead']:.2f} mm inside the hole; "
         f"frame wall {e['frame_wall']:.2f} mm round the panel",
         e["cell_pocket"] > 0 and e["frame_wall"] > 0, "VER-93 (DVT)"),
        ("REQ-MEC-07", "the USB-C opening (usb_hole) and the receptacle's recess (usb_recess)",
         f"opening {e['usb_hole'][0]:.2f} x {e['usb_hole'][1]:.2f} for a {e['usb_w']:.2f} x {e['usb_h']:.2f} receptacle; recess "
         f"{e['usb_recess']:.2f} mm against plug_len_min - mate - 0.1 = {usbc.get('plug_len_min', 0) - usbc.get('mate', 0) - 0.1:.2f}",
         e["usb_recess"] <= usbc.get("plug_len_min", 0) - usbc.get("mate", 0) - 0.1 + 1e-6, "VER-80, VER-165"),
        ("REQ-MEC-08", "the listed openings: USB-C, two pinholes (pin_pos), the grille (grille_x, grille_slot), six screw counterbores",
         f"{len(e['pin_pos'])} pinholes, grille {e['grille_x'][0]}-{e['grille_x'][1]} mm, {len(pos['pins'])} screws, 1 USB-C opening",
         len(e["pin_pos"]) == 2 and len(pos["pins"]) == 6, "VER-113 (NOT_BUILT), VER-48"),
        ("REQ-MEC-09", "screw_l in the pilots above the board (z_top_cu, screw_cb, side_engage, deck_engage)",
         f"M2 x {e['screw_l']}: {side_eng:.2f} mm above the board in every pilot against 4.0 at the sides and 3.0 at the ends",
         side_eng >= 4.0 - 1e-6, "VER-147 (FAIL: DEF-31, DEC-18)"),
        ("REQ-MEC-10", "the panel's pocket (bay_w, bay_l, panel_fit) and its tape (tape_t)",
         f"bay {e['bay_w']:.2f} x {e['bay_l']:.2f} for a {e['panel_w']} x {e['panel_l']} panel; tape {e['tape_t']} mm",
         e["panel_w"] + e["panel_fit"] <= e["bay_w"] + 1e-6 and e["tape_t"] > 0, "VER-154 (EVT)"),
        ("REQ-MEC-11", "connector mouths: usb, j2_plug, j4_plug, fpc_conn, swd, pinholes against the bosses",
         f"J2's plug box {e['j2_plug']}, J4's {e['j4_plug']}, SWD at {pos['swd']}, pinholes {pos['pinholes']}",
         all(k in e for k in ("j2_plug", "j4_plug")) and bool(pos.get("swd")), "VER-165"),
        ("REQ-MEC-12", "the tub's bosses (tub_boss_d)",
         f"largest boss {max(e['tub_boss_d']):.1f} mm: {max(e['tub_boss_d']) / 2 + 1:.1f} mm keep-out, the statement's 4.0",
         abs(max(e["tub_boss_d"]) / 2 + 1 - 4.0) < 1e-6, "VER-142 (PASS)"),
        ("REQ-BTN-05", "the DS Lite silicone over the combs, held by the ends' ribs (sil_rib_gap) without crushing its web",
         f"rib gap {e['sil_rib_gap']:.2f} mm over a {e['sil_web_t']:.2f} mm web; the force is relative to a genuine DS Lite",
         e["sil_rib_gap"] >= e["sil_web_t"], "VER-50"),
        ("REQ-BTN-06", "positions.json pieces (silicone centres) against the comb centres (buttons)",
         "each piece's centre at its cluster's comb centroid at nominal", None, "VER-174"),
        ("REQ-BTN-08", "ss_metal_dome in ss_well; ss_plunger_gap and ss_lift over the dome",
         f"well {e['ss_well'][0] - 2 * e['ss_well'][1]:.2f} mm for a {e['ss_metal_dome'][0]} mm dome (+0.3); plunger "
         f"{e['ss_plunger_gap'] + e['ss_lift'] - 0.35:.2f} mm over a {e['ss_metal_dome'][1]} mm dome (+0.05)",
         e["ss_well"][0] - 2 * e["ss_well"][1] >= e["ss_metal_dome"][0] + 0.3 - 1e-6
         and e["ss_plunger_gap"] + e["ss_lift"] - 0.35 >= e["ss_metal_dome"][1] + 0.05 - 1e-9, "VER-178, VER-180 (ss_lift is an estimate)"),
        ("REQ-ASM-01", "allocated to REQ-ASM-01.R", "the holds below", None, "VER-91"),
        ("REQ-ASM-01.R", "assembly.md's order: each part dropped straight down and held by a feature (the holds below)",
         "every part held", None, "VER-91"),
        ("REQ-ASM-02", "J3's latch with fpc_slack in the FPC's fold", f"slack {e['fpc_slack']} mm in the fold", e["fpc_slack"] > 0,
         "VER-106"),
    ]
    # REQ-BTN-06 at nominal: each silicone piece centred on its cluster
    import statistics
    clusters = {}
    for sw, (x, y) in pos["buttons"].items():
        n = int(sw[2:])
        clusters.setdefault((n - 1) // 4, []).append((x, y))
    offs = []
    for pc in pos["pieces"]:
        best = min(clusters.values(), key=lambda pts: (statistics.mean(p[0] for p in pts) - pc[0]) ** 2
                   + (statistics.mean(p[1] for p in pts) - pc[1]) ** 2)
        offs.append(((statistics.mean(p[0] for p in best) - pc[0]) ** 2 + (statistics.mean(p[1] for p in best) - pc[1]) ** 2) ** 0.5)
    m = [(r, f, (f"pieces off their comb centroid by at most {max(offs):.2f} mm (D-pads; the A/B/X/Y diamonds centre the same way)"
                 if r == "REQ-BTN-06" else t), (max(offs) <= 0.5 if r == "REQ-BTN-06" else ok), c) for r, f, t, ok, c in m]
    # the holds, in assembly.md's order: (part, what holds it, the case.scad name that must exist)
    holds = [
        ("the speaker", "its pocket in the top's right bezel centres it; the board lies over it (spk_top_over)", "spk_top_over"),
        ("A/B/X/Y caps, D-pad, Start/Select", "the ends' pockets and retainers (pocket_d, retainer_z)", "retainer_z"),
        ("the silicone pads", "the walls on the ends' underside (sil_rim) and the board over them", "sil_rim"),
        ("the panel", "the top's frame centres it (frame_wall, bay_w); the tape bonds it in Z (REQ-MEC-10 names the tape)", "bay_w"),
        ("the cell", "the tub's pocket walls (cell_pocket_box, DEF-58's stop); the board's hole and the panel over it in Z; the "
                     "double-sided tape only locates", "cell_pocket_box"),
        ("the FPC", "J3's latch (fpc_conn), folded under the panel with fpc_slack", "fpc_slack"),
        ("the cell's plug", "J2's PicoBlade latch, its box j2_plug clear beside the glass", "j2_plug"),
        ("the speaker's plug", "J4's SH friction lock, its box j4_plug clear of the speaker's hole", "j4_plug"),
        ("the Start/Select domes", "the strip's wells (ss_well) round each 4 mm dome", "ss_well"),
        ("the board", "six M2 screws through it, clamped between the bosses (tub_boss_d); the crush ribs centre it", "tub_boss_d"),
        ("the tub", "its tongue in the top's skirt, keys, grip ribs (grip_reach), then the screws", "grip_reach"),
    ]
    want = sorted(set(CASE_REQ.findall(open(REQS, encoding="utf-8").read())))
    print("case-side requirements in requirements.md: " + " ".join(want))
    mapped = {r for r, *_ in m}
    for r in want:
        if r not in mapped:
            row(False, f"{r}: not mapped to a case feature")
    print("requirement -> the case's feature -> its nominal number (the worst case is the named check's):")
    for r, feat, num, ok, chk in m:
        if r not in want:
            continue
        if ok is None:
            ok = all(h[2] in e for h in holds) if r.startswith("REQ-ASM-01") else True
        row(ok, f"{r}: {feat}: {num} [worst case: {chk}]")
    print("what holds each part in the closed case (assembly.md section 5):")
    for part, how, name in holds:
        row(name in e, f"{part}: {how}" + ("" if name in e else f": {name} is not in case.scad"))
    sym = all([-k[0], -k[1]] in e["keys"] for k in e["keys"])
    print(f"note: the tub's four keys are {'symmetric under a half turn' if sym else 'asymmetric'}: "
          f"{'the USB-C tab alone keys the tub to the top' if sym else 'they key the tub'}")
    print(f"{len(row.bad)} not met")
    return dn.result(not row.bad)


# ---- the reference design in a temp dir, with positions.json, for the scripts that read it from case/ -----------------
import contextlib
import shutil


@contextlib.contextmanager
def ref_copy():
    """hardware/ref_design copied into a temp dir with case/positions.json written from case.scad's echo, so placement.py
    and case/check_fit.py run on the committed files without writing under hardware/ref_design (positions.json is
    gitignored there). The datasheets, fab outputs, prints and pictures are left out; the 3D models are kept."""
    with tempfile.TemporaryDirectory(prefix="vv_ref_") as tmp:
        dst = os.path.join(tmp, "ref_design")
        shutil.copytree(REF, dst, ignore=shutil.ignore_patterns("fab", "print", "images", "img", "fitkit", "datasheets",
                                                                  "*.stl", "__pycache__", "last_route.ses"))
        pos, lines = echo()
        json.dump(pos, open(os.path.join(dst, "case", "positions.json"), "w"), indent=1)
        yield dst, pos, lines


def load_module(path, name):
    import importlib.util
    sys.path.insert(0, os.path.dirname(path))
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


# ---- VER-06: the board inside the case (case/check_fit.py) -------------------------------------------------------------
def fit(seed=None):
    with ref_copy() as (dst, pos, _):
        if seed:
            pos["glass_limit"] = 0.5            # a ceiling under the glass lower than J3: check_fit must fail
            json.dump(pos, open(os.path.join(dst, "case", "positions.json"), "w"), indent=1)
        r = subprocess.run([sys.executable, os.path.join(dst, "case", "check_fit.py")], cwd=os.path.join(dst, "case"),
                           capture_output=True, text=True)
        print("case/check_fit.py run in a temp copy of hardware/ref_design with positions.json from case.scad's echo")
        print(r.stdout.strip())
        rep = os.path.join(dst, "case", "fit_report.txt")
        if os.path.exists(rep):
            print("--- fit_report.txt\n" + open(rep, encoding="utf-8").read().strip())
    return 0 if r.returncode == 0 else 1


# ---- VER-19: placement drift ------------------------------------------------------------------------------------------
PLACE_MM, PLACE_DEG = 0.05, 0.1


def placement(seed=None):
    pcb, b = dn.board()
    mm = pcb.ToMM
    row = Rows()
    with ref_copy() as (dst, pos, _):
        pl = load_module(os.path.join(dst, "board", "placement.py"), "placement_vv")
        P = dict(pl.P)
    if seed:
        k = next(iter(sorted(P)))
        x, y, r, s = P[k]
        P[k] = (x + 0.1, y, r, s)
    fps = {f.GetReference(): f for f in b.GetFootprints()}
    worst, bad, n = (0.0, ""), [], 0
    for ref, (x, y, rot, side) in sorted(P.items()):
        f = fps.get(ref)
        if not f:
            bad.append(f"{ref}: placed by placement.py, not on the board")
            continue
        n += 1
        d = math.hypot(mm(f.GetPosition().x) - x, mm(f.GetPosition().y) - y)
        want = rot if side == "F" else 180 - rot       # KiCad shows a flipped footprint's turn mirrored: 0 on the back reads 180
        dr = abs((f.GetOrientationDegrees() - want + 180) % 360 - 180)
        flip_ok = f.IsFlipped() == (side == "B")
        if d > worst[0]:
            worst = (d, ref)
        if d > PLACE_MM or dr > PLACE_DEG or not flip_ok:
            bad.append(f"{ref}: board ({mm(f.GetPosition().x):.3f}, {mm(f.GetPosition().y):.3f}, {f.GetOrientationDegrees():g} deg, "
                       f"{'B' if f.IsFlipped() else 'F'}) against placement.py ({x:.3f}, {y:.3f}, {rot:g}, {side})")
    unplaced = sorted(r for r in fps if r not in P and not r.startswith(("FID", "TP", "#")))
    for t in bad[:12]:
        row(False, t)
    row(not bad, f"{n} parts placed by placement.py (from positions.json and its own rules) against the routed board: worst "
        f"{worst[0]:.4f} mm ({worst[1]}); limits {PLACE_MM} mm and {PLACE_DEG} deg")
    # the case's own positions, straight from positions.json
    # 39cc07f: no pinholes (BOOT and RESET left with the RP2354B's SW18 and SW19) and no J5; the power key's SW24 is the
    # case's (case.scad pwr_pos) and J8's mouth is the left wall's opening (sd_y)
    case_pts = {**{k: v for k, v in pos["buttons"].items()}, **pos["ss_buttons"],
                **{f"H{i + 1}": p for i, p in enumerate(pos["pins"])},
                **{ref: p for ref, p in zip(("SW18", "SW19"), pos.get("pinholes", []))},
                "J3": [pos["fpc_conn"]["x"], pos["fpc_conn"]["y"]]}
    if "power" in pos:
        case_pts["SW24"] = pos["power"]["c"]
    off = []
    for ref, (x, y) in sorted(case_pts.items()):
        if ref not in fps:
            off.append(f"{ref} not on the board")
            continue
        f = fps[ref]
        d = math.hypot(mm(f.GetPosition().x) - x, mm(f.GetPosition().y) - y)
        if d > PLACE_MM:
            off.append(f"{ref} {d:.3f} mm")
    row(not off, f"{len(case_pts)} parts the case positions (the 20 contacts, H1-H6, J3, the power key SW24) at "
        "positions.json's spot within 0.05 mm" + (f": {off}" if off else ""))
    if "sd" in pos:
        j8 = fps["J8"]
        y0, y1 = sorted(pos["sd"]["y"])
        print(f"  J8 at ({mm(j8.GetPosition().x):.2f}, {mm(j8.GetPosition().y):.2f}); the left wall's opening y {y0:.2f}-{y1:.2f} "
              f"(case.scad sd_y): its centre {(y0 + y1) / 2:.2f}")
        row(abs(mm(j8.GetPosition().y) - (y0 + y1) / 2) <= PLACE_MM, f"J8 centred on the SD opening within {PLACE_MM} mm")
    j1 = fps["J1"]
    usb_y = pos["usb"]["y"]
    row(abs(mm(j1.GetPosition().y) - usb_y) <= PLACE_MM, f"J1 at y {mm(j1.GetPosition().y):.3f} against the USB slot's {usb_y}")
    print(f"not placed by placement.py (the scripts route or pour them, or they are test pads): {len(unplaced)}")
    return dn.result(not row.bad)


# ---- VER-20: the Z stack ------------------------------------------------------------------------------------------------
PCB_TOL = 0.16          # mm: a 1.6 mm board +-10% (JLC's standard, from memory: VER-17's page confirms)
TAPE_TOL = 0.03         # mm: the panel tape's thickness tolerance (stand-in until VER-173 names the tape)
MARGIN = 0.2


def part_heights(dst):
    """{ref: (x0, y0, x1, y1, z0, z1)} from case/check_fit.py's own STL extents, in the temp copy."""
    cf = load_module(os.path.join(dst, "case", "check_fit.py"), "check_fit_vv")
    import pcbnew
    b = pcbnew.LoadBoard(cf.PCB)
    refs = [f.GetReference() for f in b.GetFootprints() if not f.IsFlipped() and not f.GetReference().startswith(("H", "FID"))]
    return {r: e for r, e in cf.extents(refs).items() if e}


def rect_hit(box, rect):
    (x0, y0), (x1, y1) = rect
    return box[2] > min(x0, x1) and box[0] < max(x0, x1) and box[3] > min(y0, y1) and box[1] < max(y0, y1)


def stack(seed=None):
    e = dn.scad()
    row = Rows()
    with ref_copy() as (dst, pos, lines):
        ext = part_heights(dst)
    stk = next((ln for ln in lines if "STACK" in ln), "")
    terms = [float(x) for x in re.findall(r"\d+(?:\.\d+)?", stk.split("=")[0])]
    print(f"case.scad's STACK line: {stk.split(':', 1)[-1].strip()}")
    row(abs(sum(terms) - e["case_h"]) < 0.01, f"the terms sum to {sum(terms):.2f} mm, case_h {e['case_h']:.2f}")
    print(f"the board's top face is clamped to the top's bosses by the six screws, so the space over it is the top's own "
          f"(print +-{dn.PRINT_TOL} mm) and the board's thickness (+-{PCB_TOL}, stand-in) moves the tub: the seam steps by up to "
          f"{PCB_TOL} mm, inside the {e['deck_tongue_h']:.2f} mm tongue")
    glass = pos["glass"]
    under = [(r, x[5]) for r, x in ext.items() if rect_hit(x, glass)]
    hmax, href = max((h, r) for r, h in under) if under else (0.0, "")
    if seed:
        hmax += 0.5
    tol = [dn.PRINT_TOL, TAPE_TOL]
    m_nom = e["glass_gap"] - hmax
    m_wc, m_rss = m_nom - sum(tol), m_nom - math.sqrt(sum(t * t for t in tol))
    row(m_wc >= MARGIN, f"under the glass: {e['glass_gap']} mm over the board, the tallest part {href} {hmax:.2f} mm (its 3D model): "
        f"{m_nom:.2f} nominal, {m_wc:.2f} worst case, {m_rss:.2f} RSS (print {dn.PRINT_TOL}, tape {TAPE_TOL}) against {MARGIN}")
    cell_free = e["glass_gap"] - e["cell_top"]
    c_wc = cell_free - 0.4 - sum(tol)
    row(c_wc >= 0.0, f"over a 4.5 mm cell under the panel: {cell_free:.2f} mm free, {cell_free - 0.4:.2f} after REQ-BAT-07's 0.4 mm "
        f"of swelling, {c_wc:.2f} at worst case: the swelling room holds (VER-182 reads the slope's ceiling too)")
    decks = pos["decks"]
    top = echo_json(lines, "TOP") or {}
    pockets = top.get("pockets", [])
    p_room = top.get("pocket_limit", 0) + 0.3          # case.scad:688 cuts each pocket 0.3 over its limit, as the decks
    room = e["z_deck_under"] - e["z_top_cu"]
    worst = (1e9, "", 0.0, 0.0)
    for r, x in ext.items():
        if not any(rect_hit(x, z) for z in decks):
            continue
        here = p_room if any(rect_hit(x, p) for p in pockets) else room
        if here - x[5] < worst[0]:
            worst = (here - x[5], r, x[5], here)
    m, dref, dmax, here = worst
    d_wc = m - dn.PRINT_TOL
    row(d_wc >= MARGIN, f"under the button ends: {room:.2f} mm ({p_room:.2f} in the top's pockets, case.scad's TOP line); the least "
        f"room over a part: {dref} {dmax:.2f} mm under {here:.2f}: {m:.2f} nominal, {d_wc:.2f} worst case against {MARGIN}")
    fz = [(x[5], r) for r, x in ext.items() if any(rect_hit(x, z) for z in pos["fpc_zones"]) and r != "J3"]
    fmax, fref = max(fz) if fz else (0.0, "none")
    row(fmax <= pos["fpc_low"] - MARGIN + 1e-9 or not fz,
        f"under the folded FPC: the tallest part but J3 {fref} {fmax:.2f} mm against fpc_low {pos['fpc_low']} (an ESTIMATE, VER-152) "
        f"less {MARGIN}")
    print(f"{len(row.bad)} failing")
    return dn.result(not row.bad)


# ---- VER-174: the XY chain, pill to comb and cap to hole ------------------------------------------------------------
JLC_REG = 0.075         # mm: copper to drilled hole registration (JLC, from memory: VER-17's page confirms)
PILL_TOL = 0.1          # mm: the pill's place in the DS Lite silicone (moulded; stand-in until VER-51 measures it)
PILL_LIMIT = 0.9


def xy(seed=None):
    e = dn.scad()
    pos, _ = echo()
    row = Rows()
    float_screw = (e["hole_d"] - 2.0) / 2 + (0.5 if seed else 0.0)
    terms = [("the comb's copper to the board's holes (JLC registration, stand-in)", JLC_REG),
             (f"the board on its M2 screws in {e['hole_d']} mm holes", float_screw),
             ("the top's pilots to its button walls (one print)", dn.PRINT_TOL),
             (f"the silicone's rim in the top's walls (rim_clear)", e["rim_clear"]),
             ("the pill in the silicone (stand-in)", PILL_TOL)]
    for t, v in terms:
        print(f"  +-{v:.3f} mm  {t}")
    nom = 0.0
    import statistics
    for pc in pos["pieces"]:
        pts = [p for k, p in pos["buttons"].items() if math.hypot(p[0] - pc[0], p[1] - pc[1]) < 10]
        if pts:
            nom = max(nom, math.hypot(statistics.mean(p[0] for p in pts) - pc[0], statistics.mean(p[1] for p in pts) - pc[1]))
    wc = nom + sum(v for _, v in terms)
    rss = nom + math.sqrt(sum(v * v for _, v in terms))
    row(wc <= PILL_LIMIT, f"pill to comb: nominal {nom:.2f} mm (positions.json's pieces over their combs), worst case {wc:.3f} mm, "
        f"RSS {rss:.3f} mm against {PILL_LIMIT} mm (REQ-BTN-06)")
    print(f"  cap to hole: the D-pad's opening clears its cap by {e['dpad_gap']} mm a side (case.scad dpad_gap); a cap rides on its "
          "silicone, so the same chain moves it: worst case "
          f"{wc:.2f} mm {'inside' if wc < e['dpad_gap'] else 'PAST'} that gap")
    row(wc < e["dpad_gap"], f"cap to hole: the chain's {wc:.2f} mm against the D-pad's {e['dpad_gap']} mm gap")
    return dn.result(not row.bad)


# ---- VER-152: the ESTIMATE ledger ---------------------------------------------------------------------------------------
MEASURED = os.path.join(os.path.dirname(HERE), "evidence")


def estimates(seed=None):
    e = dn.scad()
    row = Rows()
    src = open(os.path.join(CASE, "case.scad"), encoding="utf-8").read().splitlines()
    marked = []
    for i, ln in enumerate(src):
        if "ESTIMATE" not in ln or ln.lstrip().startswith("// ESTIMATE marks"):
            continue
        comment_only = ln.lstrip().startswith("//")
        # a code line: its own assignments; a comment at the margin: the line under it; an indented comment line: the
        # continuation of the trailing comment on the line above it
        j = i if not comment_only else (i + 1 if not ln.startswith(" " * 20) else i - 1)
        for m in re.finditer(r"\b([a-z_][a-z0-9_]*)\s*=", src[j].split("//")[0]):
            if m.group(1) in e and m.group(1) not in marked:
                marked.append(m.group(1))
        if comment_only and "ESTIMATE:" in ln:     # names listed after 'ESTIMATE:' in the comment itself
            for name in re.findall(r"\b([a-z][a-z0-9_]+)\b", ln.split("ESTIMATE:", 1)[1]):
                if name in e and name not in marked:
                    marked.append(name)
    base, _ = echo()
    moves = {}
    refused = {}
    for name in marked:
        v = e[name]
        for f in (1.1, 1.02, 0.98):                  # case.scad's asserts may refuse a 10% nudge: try 2% either way
            if isinstance(v, (int, float)):
                nv = v * f
            elif isinstance(v, list) and all(isinstance(x, (int, float)) for x in v):
                nv = [x * f for x in v]
            else:
                break
            new, err = echo(((name, json.dumps(nv)),))
            if new is not None:
                moves[name] = sorted(k for k in base if json.dumps(base[k], sort_keys=True) != json.dumps(new.get(k), sort_keys=True))
                break
            refused[name] = next((ln for ln in err if "assert" in ln.lower()), err[-1] if err else "")[:120]
    # the cell's size is VER-37's (its spec sheet), not marked in case.scad but it sets the hole: listed with them
    measured = set()
    for f in os.listdir(MEASURED):
        if "_VER-152_" in f and "measure" in f:
            measured |= set(re.findall(r"^([a-z_][a-z0-9_]*)\s*=", open(os.path.join(MEASURED, f)).read(), re.M))
    copper = []
    print("each ESTIMATE in case.scad, nudged by 10% through openscad -D, and the positions.json keys (the board's inputs) it moves:")
    for name in marked:
        ks = moves.get(name)
        if ks is None and name in refused:
            print(f"  {name} = {e[name]}: every nudge refused by case.scad's asserts ({refused[name]}): it is held tight by "
                  "what it fits, so it is counted as moving the board")
            copper.append(name)
            continue
        print(f"  {name} = {e[name]}: " + ("not numeric, read by hand" if ks is None else (", ".join(ks) or "moves nothing the board reads")))
        if ks:
            copper.append(name)
    if seed:
        moves["seeded_estimate"] = ["seeded"]
        copper.append("seeded_estimate")
    for name in copper:
        row(name in measured, f"{name}: moves {', '.join(moves.get(name) or ['(asserts)'])} and has no measurement "
                              f"(evidence/<date>_VER-152_measured.txt)" if name not in measured else f"{name}: measured")
    row("cell" in measured, f"cell = {e['cell']}: sets the hole's size; from the cell's spec sheet (VER-37, BLOCKED), "
                            "measured at EVT (VER-49)")
    am = open(os.path.join(REF, "docs", "assembly.md"), encoding="utf-8").read()
    sizes = re.findall(r"(\d+(?:\.\d+)?)\s*x\s*(\d+(?:\.\d+)?)\s*mm", am)
    hole = dn.cutouts()
    hw, hl = round(hole[2] - hole[0], 2), round(hole[3] - hole[1], 2)
    print(f"assembly.md's sizes: {sizes or 'none'}; the cell's hole in placement.py {hw} x {hl} mm: "
          + ("assembly.md states no hole size, nothing to disagree" if not sizes else "compare by hand"))
    print(f"{len(row.bad)} not measured")
    return dn.result(not row.bad)


# ---- VER-165: connector mouths and pinholes ----------------------------------------------------------------------------
TALL = 1.0


def mouths(seed=None):
    import pcbnew
    e = dn.scad()
    row = Rows()
    with ref_copy() as (dst, pos, _):
        ext = part_heights(dst)
    b = pcbnew.LoadBoard(os.path.join(dn.BOARD, "battler.kicad_pcb"))
    mm = pcbnew.ToMM
    fps = {f.GetReference(): f for f in b.GetFootprints()}
    to_k = lambda box: [[100 + box[0][0], 100 - box[1][1]], [100 + box[1][0], 100 - box[0][1]]]   # noqa: E731 case -> KiCad

    def mouth_side(ref, depth):
        """the box in front of a side-entry connector: on its mounting-pad side, away from its signal pads"""
        f = fps[ref]
        sig = [p for p in f.Pads() if p.GetNumber() not in ("MP", "")]
        mp = [p for p in f.Pads() if p.GetNumber() == "MP"]
        sx = sum(mm(p.GetPosition().x) for p in sig) / len(sig)
        sy = sum(mm(p.GetPosition().y) for p in sig) / len(sig)
        mx = sum(mm(p.GetPosition().x) for p in mp) / len(mp)
        my = sum(mm(p.GetPosition().y) for p in mp) / len(mp)
        fab = [g.GetBoundingBox() for g in f.GraphicalItems() if g.GetLayer() == pcbnew.F_Fab]
        x0, y0 = min(mm(g.GetLeft()) for g in fab), min(mm(g.GetTop()) for g in fab)
        x1, y1 = max(mm(g.GetRight()) for g in fab), max(mm(g.GetBottom()) for g in fab)
        if abs(mx - sx) > abs(my - sy):
            return [[x0 - depth, y0], [x0, y1]] if mx < sx else [[x1, y0], [x1 + depth, y1]]
        return [[x0, y0 - depth], [x1, y0]] if my < sy else [[x0, y1], [x1, y1 + depth]]
    boxes = {"J2": (to_k(e["j2_plug"]), "the cell's plug (case.scad j2_plug)"),
             "J4": (to_k(e["j4_plug"]), "the speaker's SH plug (case.scad j4_plug)")}
    # 39cc07f: J8's mouth is the left wall's opening, not a box inside the tub: its body must pass the opening with the
    # print's and the board's float on each side (PRINT_TOL + SCREW_FLOAT)
    if "J8" in ext and "sd" in pos:
        y0, y1 = sorted(pos["sd"]["y"])
        jx = ext["J8"]
        side = min(jx[1] - y0, y1 - jx[3])
        need = dn.PRINT_TOL + dn.SCREW_FLOAT
        z0, z1 = pos["sd"]["z"]
        top = jx[5]
        row(side >= need and top <= z1 - dn.PRINT_TOL,
            f"J8's body y {jx[1]:.3f}-{jx[3]:.3f} (its 3D model) in the left wall's opening y {y0:.2f}-{y1:.2f}: {side:.3f} mm a "
            f"side against the print's {dn.PRINT_TOL} and the board's float {dn.SCREW_FLOAT} ({need:.1f} mm); its top {top:.2f} mm "
            f"under the opening's {z1:.2f} mm over the board")
    if seed:
        boxes["J4"] = (mouth_side("J2", 8.0), "seeded: J2's mouth box made 8 mm deep")
    inner = [[100 - e["in_x"], 100 - (e["case_l"] / 2 - e["wall"])], [100 + e["in_x"], 100 + (e["case_l"] / 2 - e["wall"])]]
    for ref, (box, what) in boxes.items():
        (x0, y0), (x1, y1) = box
        tall = sorted(f"{r} {x[5]:.2f} mm" for r, x in ext.items() if r != ref and x[5] > TALL and rect_hit(x, box))
        row(not tall, f"{ref} mouth box {x0:.2f}-{x1:.2f} x {y0:.2f}-{y1:.2f} ({what}): parts over {TALL} mm in it: " + (", ".join(tall) or "none"))
        inside_wall = inner[0][0] <= min(x0, x1) and max(x0, x1) <= inner[1][0] and inner[0][1] <= min(y0, y1) and max(y0, y1) <= inner[1][1]
        gap = min(min(x0, x1) - inner[0][0], inner[1][0] - max(x0, x1))
        row(inside_wall, f"{ref} mouth box inside the tub's inner wall (x {inner[0][0]:.2f}-{inner[1][0]:.2f}): "
            + ("yes" if inside_wall else f"crosses it by {-gap:.2f} mm: a plug cannot go in with the board in the tub"))
    print("J1 (a mated plug through the wall's opening) and J3 (the FPC's lead-in under the panel) are check_fit's (VER-06): "
          "its USB-C and folded-FPC items")
    bad = []
    for p in pos["pinholes"]:
        for i, h in enumerate(pos["pins"]):
            d = math.hypot(p[0] - h[0], p[1] - h[1])
            if d < pos["boss_d"] / 2 + 1.0:
                bad.append(f"pinhole {p} {d:.2f} mm from H{i + 1}")
    if pos["pinholes"]:
        near = min(math.hypot(p[0] - h[0], p[1] - h[1]) for p in pos["pinholes"] for h in pos["pins"])
        row(not bad, f"no pinhole inside a boss circle (boss {pos['boss_d']} mm + 1 mm): the nearest {near:.2f} mm"
            + (f": {bad}" if bad else ""))
    else:
        print("no pinholes in the case (case.scad pin_pos is empty since 39cc07f): the boss rule has nothing to read")
    return dn.result(not row.bad)


CHECKS = {"arch": (arch, SEED_ARCH), "fit": (fit, True), "placement": (placement, True), "stack": (stack, True),
          "xy": (xy, True), "estimates": (estimates, True), "mouths": (mouths, True)}


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if "--list" in sys.argv or not args:
        print(__doc__)
        return 0
    fn, seed = CHECKS[args[0]]
    if "--seed" in sys.argv:
        if not seed:
            sys.exit(f"case_checks: {args[0]} has no seed")
        print(f"SEEDED: {seed}")
        return fn(seed)
    return fn()


if __name__ == "__main__":
    sys.exit(main())
