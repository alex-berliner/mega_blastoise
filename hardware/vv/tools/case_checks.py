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
        if r.returncode or not os.path.exists(out):
            print(f"BLOCKED: openscad failed: {r.stderr[-400:]}\nRESULT: BLOCKED")
            sys.exit(2)
        text = open(out, encoding="utf-8").read()
    m = re.search(r'ECHO: "JSON (\{.*\})"', text)
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


CHECKS = {"arch": (arch, SEED_ARCH)}


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
