#!/usr/bin/env python3
"""Build the printable fit kit (case/fitkit/): stand-ins for the board and the loose parts, and quick
coupons cut from the real tub, top piece and deck, to test the case with the real parts before ordering boards.

    python3 case/fitkit.py            # after case/export.py --stl and a routed board

It asks check_fit.py for every board part's envelope (from the part's 3D model), writes them to
fitkit_parts.scad, then renders each kit part from fitkit.scad (which includes case.scad).
"""
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "fitkit")
sys.path.insert(0, HERE)
import check_fit  # noqa: E402
import scad  # noqa: E402  (tools/, on the path through check_fit)

KIT = [  # (kit name, file, what it tests)
    ("board", "board_mock.stl", "board stand-in: outline, locating holes, part envelopes, contact and J3 marks"),
    ("panel", "panel_dummy.stl", "ER-TFT024IPS-3 at its thickest, active area engraved"),
    ("cell", "cell_dummy.stl", "403450 cell (nominal 50 x 34 x 4), for the board mock's hole"),
    ("plug", "cell_plug_dummy.stl", "PicoBlade 51021-0200 plug with a cable stub (approximate)"),
    ("speaker", "speaker_dummy.stl", "GSPK151035PN speaker, 10 x 15 x 3.5 mm, face up, for its pocket under the grille"),
    ("seam_tub", "coupon_seam_tub.stl", "the left wall of the tub at the seam: its tongue and a crush rib"),
    ("seam_top", "coupon_seam_top.stl", "the top piece's skirt over it"),
    ("buttons_tub", "coupon_buttons_tub.stl", "seat 1's end of the tub: ribs, bosses, crush ribs"),
    ("buttons_board", "coupon_buttons_board.stl", "seat 1's end of the board mock"),
    ("buttons_top", "coupon_buttons_top.stl", "seat 1's end of the top piece: the step, the spine, the shoulders and the undercut"),
    ("buttons_deck", "coupon_buttons_deck.stl", "seat 1's deck (the real part): cap holes, walls, wedge, printed face down"),
    ("usb_tub", "coupon_usb_tub.stl", "the USB-C opening's lower half, in the tub's tab"),
    ("usb_top", "coupon_usb_top.stl", "its upper half, in the top piece's skirt"),
    ("usb_board", "coupon_usb_board.stl", "the board's tab with a USB-C shell stand-in, between the two halves"),
    ("tune_openings", "tune_openings.stl", "D-pad openings 0.55/0.65/0.75, a keyed A/B/X/Y cluster (8.0 holes), holes 7.6/7.8/8.2"),
    ("tune_stack_dpad_board", "tune_stack_dpad_board.stl", "seat 1's D-pad: board slice with the contact rings"),
    ("tune_stack_abxy_board", "tune_stack_abxy_board.stl", "seat 1's A/B/X/Y: board slice with the contact rings"),
    ("tune_stack_ss_board", "tune_stack_ss_board.stl", "seat 1's Start/Select: board slice with the two small contacts"),
] + [(f"tune_stack_{c}_lid", f"tune_stack_{c}_lid_dome{d}.stl", f"its deck slice on legs, silicone height {d} mm",
       {"dome_h": d, "tune": "true"}) for c in ("dpad", "abxy", "ss") for d in ("1.6", "1.75", "1.9")] + [
    # (a fourth element: case.scad settings to render that coupon with, as openscad -D name=value)
    ("frame_top", "coupon_frame_top.stl", "the top piece's window, pocket and panel frame"),
    # the test plate (6_tests): questions for the shell's redesign
    ("test_overhangs", "test_overhangs.stl", "overhang ladder: fins at 45 to 75 degrees from vertical, a convex R6 round"),
    ("test_screws_tub", "coupon_screws_tub.stl", "two of the tub's screw holes (counterbore, clearance, boss), printed flat"),
    ("test_screws_top", "coupon_screws_top.stl", "one of the top piece's bosses with its 1.6 pilot, printed face down"),
    ("test_column", "test_column.stl", "a 5.0 column, as the screw bosses"),
    ("test_rings", "test_rings.stl", "rings 0.2/0.3/0.4/0.5 a side over the 5.0 column"),
] + [
    # the overhang kit (7_overhangs_*): one of each part per layer height, the height on its pad
    (f"overhang_{p}", f"test_overhang_{p}_{h.replace('.', '')}.stl", f"{what}, for {h} mm layers", {"oh_layer": f'"{h}"'})
    for h in ("0.20", "0.16", "0.12", "0.08") for p, what in (
        ("ladder", "overhang ladder: double fins 30 to 80 degrees from vertical (2.5 apart from 50 to 70), front and back"),
        ("rounds", "convex rounds R10, R6, R3 from 80 to 25 degrees from vertical, ticked on their end faces"))
] + [
    # the overhang settings (9_overhang_settings): the 0.20 ladder, its pad saying which settings
    ("overhang_ladder", f"test_overhang_ladder_{t}.stl", f"overhang ladder at 0.20, printed with the {what} overhang settings",
     {"oh_tag": f'"{t}"'}) for t, what in (("DEF", "default"), ("TUNED", "tuned"))
] + [
    # the first layer (8_first_layer_*): the tub's bottom edge at a first layer height and an
    # elephant's-foot compensation, engraved (the slicer settings are the plate's and the object's)
    ("first_layer", f"test_first_layer_L{l}_E{e}.stl", f"a corner of the tub's bottom edge, for first layer 0.{l}, elephant's foot 0.{e}",
     {"fl_label": f'"L{l} E{e}"'}) for l in ("20", "12") for e in ("15", "25")
]


def part_envelopes():
    """Every top-side part's box in case coordinates (x, y from the case centre, z above the board)."""
    parts = check_fit.refs()
    boxes = check_fit.extents([ref for ref, *_ in parts])
    rows = []
    for ref, flipped, name, court in parts:
        b = boxes.get(ref)
        if ref in check_fit.KNOWN_HEIGHT and b:
            b = (b[0], b[1], b[2], b[3], 0.0, check_fit.KNOWN_HEIGHT[ref])
        if b is None:                                # no 3D model here: courtyard at the package height
            h = next((v for k, v in check_fit.PACKAGE_HEIGHT.items() if k in name), 1.5)
            b = (court[0], court[1], court[2], court[3], 0.0, h)
        if flipped:
            continue                                 # the bottom carries nothing tall (pads only)
        x0, y0, x1, y1, z0, z1 = b
        rows.append((ref, x0 - 100, 100 - y1, x1 - 100, 100 - y0, z0, z1))   # KiCad -> case coordinates
    return rows


def main():
    os.makedirs(OUT, exist_ok=True)
    rows = part_envelopes()
    with open(os.path.join(HERE, "fitkit_parts.scad"), "w") as f:
        f.write("// generated by fitkit.py from the board's 3D models: [ref, x0, y0, x1, y1, z0, z1]\n")
        f.write("KIT_PARTS = [\n" + ",\n".join(
            f'  ["{r[0]}", {r[1]:.3f}, {r[2]:.3f}, {r[3]:.3f}, {r[4]:.3f}, {r[5]:.3f}, {r[6]:.3f}]' for r in rows) + "\n];\n")
    print(f"{len(rows)} part envelopes")
    only = sys.argv[1:]                 # names to build (default: all)
    todo = [k for k in KIT if not only or any(o in k[1] for o in only)]
    with ThreadPoolExecutor(scad.jobs()) as pool:      # the renders run at once, listed in KIT's order
        for line in pool.map(lambda k: render(*k), todo):
            print(line)


def render(kit, name, what, *defs):
    out = os.path.join(OUT, name)
    extra = [a for k, v in (defs[0] if defs else {}).items() for a in ("-D", f"{k}={v}")]
    r = subprocess.run([*scad.cmd(), "-o", out, "-D", 'part="kit"', "-D", f'kit="{kit}"', *extra,
                        os.path.join(HERE, "fitkit.scad")], capture_output=True, text=True)
    if r.returncode or not os.path.exists(out) or "ERROR" in r.stderr or "WARNING: Assert" in r.stderr:
        sys.exit(f"openscad failed for {kit}:\n{r.stderr[-2000:]}")
    return f"  {name:28s} {os.path.getsize(out) // 1024:5d} kB  {what}"


if __name__ == "__main__":
    main()
