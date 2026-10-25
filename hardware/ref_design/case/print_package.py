#!/usr/bin/env python3
"""The 3D print package: Snapmaker Orca projects ready to slice, the STLs, and a README.

    python3 case/print_package.py      # after case/export.py --stl and case/fitkit.py

Each project is built from Alex's own fit_test_v5.3mf, so it keeps his printer,
filament and plate settings (Snapmaker U1, 0.20 mm Standard, textured PEI). Two filaments since the one-piece top
(Alex, 2026-10-25): PLA on toolhead 1 (Alex, 2026-10-20: no PETG; 2026-10-22: the PLA is loaded in bay 1, so the
template's PETG slot 1 takes slot 2's PLA settings) and Bambu Support For PLA on toolhead 2, as the interface of the top's
supports only (SUPPORT_TOOL). The top prints face down: its slope and side rounds hang over the bed, so it (and the
coupons cut from it that include them, TOP_PARTS) get tree supports whose interface layers, touching the part, are the
second filament and whose body is PLA, with a prime tower; the tub and every other part print as before, without
supports. Only the objects, where they sit on the plate, and the supports change. Orca's CLI can't load U1 projects, so
the projects are written the way Orca writes them (one object file per part, mesh centred on its own height, placed by
the build item) and the STLs ship beside them in case a project won't open.

Supports are on only for the parts that need them, measured by overhangs.py: per object, not
per plate (a plate's supports filled the tub's screw counterbores, 2026-10-21). Only the top and its coupons are
meant to have them (and the board mock's USB-C stand-in, in plain PLA): any other part overhangs.py flags fails the build.

Each project names its slicer's system process preset and lists, in different_settings_to_system,
every setting it changes from it (mark_presets): on loading a project Orca and Bambu Studio take
only the listed settings from it and the rest from the named preset (PresetBundle::
load_config_file_config), so an unlisted change is lost (the 0.12 mm plate opened at 0.20,
2026-10-22). The presets are compared as print/u1_system/ and print/p1s/system/ have them:
flattened from the slicers' own profiles (Snapmaker Orca 2.4.0, Bambu Studio 2.08) with
    python3 case/print_package.py --flatten <the slicer's resources/profiles/<vendor>> <dir> <preset> ...
print_package_p1s.py builds the same plates for a Bambu Lab P1S.

Writes print/package_U1/ and print/mega_blastoise_print_U1.zip (build output, not in git); the
printer tests (TEST_PLATES) go to print/tests_U1/ instead and, with print_package_p1s.py's
print/tests_P1S/, into print/overhang_kit.zip (tests_zip; `--tests` packs it again alone).
"""
import os
import json
import re
import shutil
import sys
import zipfile

import numpy as np
import trimesh

HERE = os.path.dirname(os.path.abspath(__file__))
TEMPLATE = os.path.join(HERE, "print", "fit_test_v5.3mf")
OUT = os.path.join(HERE, "print", "package_U1")
ZIP = os.path.join(HERE, "print", "mega_blastoise_print_U1.zip")
BED = 270.0
GAP = 6.0
PLA = 1                      # the toolhead for every part: Alex's PLA is in bay 1 (2026-10-22)
PLA_SLOT = 2                 # the template's slot with the PLA filament settings (its slot 1 is PETG)
SUPPORT_TOOL = 2             # Bambu Support For PLA: toolhead 2 (U1), AMS slot 2 (P1S): the top's support interface only
NOT_FILAMENT = {"printable_area"}   # per-slot-length lists that aren't filament settings

KIT = os.path.join(HERE, "fitkit")
PRINT_TUB = os.path.join(HERE, "tub.stl")    # prints flat as exported (no feet since 2026-10-20)

# The plate-level support settings are all off (no plate's supports): each part that needs them gets its own (OBJECT_SUPPORT,
# TOP_SUPPORT below), and there is a prime tower only on a plate with a top (a second filament to change to). They say what
# kind of support, with the usual 0.2 mm gap for plain PLA on PLA (fused with none), and bridges aren't supported
SUPPORT = {"enable_support": "1", "support_type": "tree(auto)", "support_style": "organic",
           "support_filament": str(PLA), "support_interface_filament": str(PLA),
           "support_interface_not_for_body": "0", "support_interface_top_layers": "2",
           "support_interface_spacing": "0.5", "support_top_z_distance": "0.2", "support_ironing": "0",
           "support_on_build_plate_only": "1", "bridge_no_support": "1", "enable_prime_tower": "0"}
# Supports are per object, not per plate: with the plate's on, the slicer also put them in every
# part's small cavities (the tub's six screw counterbores, 4.4 mm wide and 1.0 deep at the bed,
# came out plugged: Alex's print, 2026-10-21). So the plate's are off (SUPPORT's other keys stay,
# they only say what kind) and each part overhangs.py says needs them gets these. The threshold
# and keeping small overhangs are overhangs.py's own measure: the slicers count the threshold from
# the bed, so case.scad's `overhang` (from vertical) is 90 less it (45 on the main line, 30 on the
# 60 degree variant). At the slicers' defaults (30 degrees, small ones dropped) Bambu Studio gave the
# board mock's USB-C stand-in nothing at all: keeping the small ones is what counts
def _threshold():
    import overhangs
    return f"{90 - overhangs.OVERHANG:g}"


OBJECT_SUPPORT = {"enable_support": "1", "support_threshold_angle": _threshold(), "support_remove_small_overhang": "0",
                  "support_on_build_plate_only": "1", "bridge_no_support": "1"}
# The top (and the coupons cut from it that have its slope or side rounds): printed face down, so those hang over the
# bed. Tree supports from the plate or from the part, the body in PLA and the layers that touch the part (3 under it, 2 on
# top of the part below) in Bambu Support For PLA, top Z distance 0 so the interface sits right on the surface and
# peels off; as sliced and measured for the one-piece top (case/SHELL.md)
TOP_SUPPORT = {"enable_support": "1", "support_type": "tree(auto)", "support_style": "default", "support_threshold_angle": _threshold(),
               "support_filament": str(PLA), "support_interface_filament": str(SUPPORT_TOOL), "support_interface_not_for_body": "1",
               "support_top_z_distance": "0", "support_bottom_z_distance": "0", "support_interface_top_layers": "3",
               "support_interface_bottom_layers": "2", "support_interface_spacing": "0", "support_on_build_plate_only": "0",
               "independent_support_layer_height": "0"}
# where the prime tower stands on a plate with a top (its coordinates: x, y from the bed's front left): the corner the
# packing reaches last, so the parts (which pack from one corner, in rows) don't have to move for it. The U1's parts pack
# from the front left, the P1S's from the back left (print_package_p1s.layout), so the tower is at the back right of the U1's bed
# and the front right of the P1S's. 35 wide and about 50 deep with its brim. The P1S's is at x 200, not 210 (2026-10-25): with 0.08 mm
# layers it flushes more per layer and sliced at 210 the G-code left the bed
TOWER_W, TOWER_D = 35, 50
TOWER_AT = {"U1": (223.0, 206.0), "P1S": (200.0, 8.0)}

# plate name: [(object name, STL), ...]; the first plate keeps v5's own layout for its parts
TOP_STL = os.path.join(HERE, "top.stl")
PLATES = {
    "0_buttons_end": [(n, f"{KIT}/coupon_{n}.stl") for n in ("buttons_tub", "buttons_board", "buttons_top")],
    "1_case": [("tub_print", PRINT_TUB), ("top", TOP_STL)],
    "2_buttons_quick": [("tune_openings", f"{KIT}/tune_openings.stl")] +
        [(f"tune_stack_{c}_board", f"{KIT}/tune_stack_{c}_board.stl") for c in ("dpad", "abxy", "ss")] +
        [(f"tune_stack_{c}_lid_dome1.75", f"{KIT}/tune_stack_{c}_lid_dome1.75.stl") for c in ("dpad", "abxy", "ss")],
    "3_buttons_heights": [(f"tune_stack_{c}_lid_dome{d}", f"{KIT}/tune_stack_{c}_lid_dome{d}.stl")
                          for c in ("dpad", "abxy", "ss") for d in ("1.6", "1.9")],
    "4_fit_coupons": [(n, f"{KIT}/coupon_{n}.stl") for n in
                      ("usb_tub", "usb_top", "usb_board", "seam_tub", "seam_top", "frame_top",
                       "buttons_tub", "buttons_board", "buttons_top")],
    # one whole unit and the dummies to fit in it (Alex, 2026-10-22): the board mock, panel, speaker, cell and its plug
    "5_unit": [("tub_print", PRINT_TUB), ("top", TOP_STL),
               ("board_mock", f"{KIT}/board_mock.stl"), ("panel_dummy", f"{KIT}/panel_dummy.stl"),
               ("speaker_dummy", f"{KIT}/speaker_dummy.stl"), ("cell_dummy", f"{KIT}/cell_dummy.stl"),
               ("cell_plug_dummy", f"{KIT}/cell_plug_dummy.stl")],
    # questions for the shell's redesign, no supports: openings, overhangs, screw holes, a column's clearance
    "6_tests": [("tune_openings", f"{KIT}/tune_openings.stl"), ("test_overhangs", f"{KIT}/test_overhangs.stl"),
                ("screws_tub", f"{KIT}/coupon_screws_tub.stl"), ("screws_top", f"{KIT}/coupon_screws_top.stl"),
                ("test_column", f"{KIT}/test_column.stl"), ("test_rings", f"{KIT}/test_rings.stl")],
    # the overhang kit, no supports: the steepest clean overhang, straight and round, at three
    # layer heights (the height on each part's pad); only the layer height differs from the rest
    **{f"7_overhangs_{h}": [(f"overhang_ladder_{h}", f"{KIT}/test_overhang_ladder_{h}.stl"),
                            (f"overhang_rounds_{h}", f"{KIT}/test_overhang_rounds_{h}.stl")]
       for h in ("020", "016", "012", "008")},
    # the tub's bottom edge at two first layer heights (a plate's setting in both slicers), two
    # elephant's-foot compensations on each (an object's): the pair that leaves no lip
    **{f"8_first_layer_0{l}": [(f"first_layer_L{l}_E{e}", f"{KIT}/test_first_layer_L{l}_E{e}.stl") for e in ("15", "25")]
       for l in ("20", "12")},
    # the overhang ladder at 0.20 twice: the slicer's overhang settings as they are (DEF) and the
    # best it has (TUNED, each printer's TUNED below), side by side
    "9_overhang_settings": [(f"overhang_ladder_{t}", f"{KIT}/test_overhang_ladder_{t}.stl") for t in ("DEF", "TUNED")],
}
# parts turned a quarter round on their plate (about z) so it packs
TURN = {"5_unit": {"cell_dummy"}}
# the parts printed face down on support filament: the top and the coupons cut from it that include its slope or side
# rounds (overhangs.py flags exactly these among the top's; the others cut from it are flat plates and need none)
TOP_PARTS = {TOP_STL, f"{KIT}/coupon_seam_top.stl", f"{KIT}/coupon_usb_top.stl", f"{KIT}/coupon_buttons_top.stl"}
# and the only other parts with supports, plain PLA trees: the board mock's USB-C stand-in (front stands out past its edge)
STANDIN_PARTS = {f"{KIT}/board_mock.stl", f"{KIT}/coupon_usb_board.stl"}


def has_top(parts):
    """Does a plate have a part printed on support filament (a second filament, a prime tower)?"""
    return any(p in TOP_PARTS for _, p in parts)


def load_part(plate, name, path):
    """A plate's part as trimesh loads it, turned a quarter round if TURN says so."""
    m = trimesh.load(path, force="mesh")
    if name in TURN.get(plate, ()):
        m.apply_transform(trimesh.transformations.rotation_matrix(np.pi / 2, [0, 0, 1]))
    return m


LAYERS = {f"7_overhangs_{h}": f"0.{h[1:]}" for h in ("020", "016", "012", "008")}
# the printer tests: not the case's parts, so a kit of their own (overhang_kit.zip), not in the case kits
TEST_PLATES = [p for p in PLATES if p.startswith(("7_", "8_", "9_"))]
TESTS = {"P1S": os.path.join(HERE, "print", "tests_P1S"), "U1": os.path.join(HERE, "print", "tests_U1")}
TESTS_ZIP = os.path.join(HERE, "print", "overhang_kit.zip")
TESTS_README = os.path.join(HERE, "print_tests_README.md")
# overhang_kit.zip's plates, in the order to print them (each answers less than the one before; Alex
# prints down the list and stops when he likes), as named there: {name in the zip: plate}.
# 7_overhangs_020 stays out: 9_overhang_settings' DEF ladder is the same print
TESTS_ORDER = {"1_first_layer_020": "8_first_layer_020", "2_overhang_settings": "9_overhang_settings",
               "3_overhangs_016": "7_overhangs_016", "4_overhangs_012": "7_overhangs_012",
               "5_first_layer_012": "8_first_layer_012", "6_overhangs_008": "7_overhangs_008"}
# each plate's own settings over the project's (the slicers' names; the same in Orca and Bambu Studio)
PLATE_SETTINGS = {plate: {"layer_height": h} for plate, h in LAYERS.items()}
PLATE_SETTINGS.update({f"8_first_layer_0{l}": {"initial_layer_print_height": f"0.{l}"} for l in ("20", "12")})
# The case kit's print quality (Alex's list, 2026-10-25), over the slicers' 0.20 mm Standard on every case plate (not the printer
# tests, TEST_PLATES, which ask what the preset does). The names and values are the same in Orca and Bambu Studio (two values
# where a setting has one per extruder variant); the filament change's own settings are each printer's script's.
CASE_SETTINGS = {
    # the outer wall slow for the finish (the P1S profile's is 200, the U1's 200 and 500) and the inner wall at 120 (the profile's
    # 300 and up, which PLA's 21 mm3/s melt limit capped anyway): Alex's 60 and 120, 2026-10-25. 100 and the profile's inner wall
    # were compared (print_package_README.md)
    "outer_wall_speed": ["60", "60"], "inner_wall_speed": ["120", "120"],
    # three walls (was 2): the fits (the tongue in its groove, the button holes) and the thin top's walls are all walls, so a third
    # wall makes them stiffer and truer than infill would; the fill stays 15 %, now gyroid (was grid): no crossings to bump the
    # walls, and it's the stronger one per gram in every direction
    "wall_loops": "3", "sparse_infill_pattern": "gyroid", "sparse_infill_density": "15%",
    # inner wall, outer wall, inner wall (was inner, outer): the outer wall is printed between two walls that hold it, so it
    # keeps its size at the fits and has no bulge from the inner wall pushing it; the support interface's surface is the outer
    # wall's, which a wall order doesn't touch
    "wall_sequence": "inner-outer-inner wall",
    # the seam at the back of each object (was aligned: a line wherever the layers lined up, down the long walls): the back of a
    # plate is the end of a part, and the visible long walls stay clean
    "seam_position": "back",
    # small perimeters (the screw pilots and counterbores, round openings: loops shorter than 15 mm) at half
    # the outer wall's speed (was: no loop counted as small, threshold 0)
    "small_perimeter_speed": ["50%", "50%"], "small_perimeter_threshold": ["15", "15"],
}
# Variable layer height where the surface rises from the bed in a slope or round, whose steps a 0.20 mm layer makes visible (a
# face 30 degrees from the bed steps 0.35 mm a layer): fine layers over the first FINE_LAYERS[object] mm (the top's DS Lite
# curve and side rounds rise for 4.5 mm face down, the tub's back round for 2.5 mm), 0.20 above. Bambu Studio's and Orca's own
# per-object height range modifiers (Metadata/layer_config_ranges.xml), as adaptive layer height isn't a setting of a project. The
# coupons cut from the top and the tub follow the part they are cut from; measured as the sloped face area by height above the bed.
# A plate with a prime tower gives every object the same range (the tower needs one layer height profile)
# Per plate (Alex, 2026-10-25): the release plates, the case itself and the whole unit, at 0.08 mm (the P1S profile's
# min_layer_height, the lowest it takes); the test plates (the fit coupons, the end) at 0.12, which prints sooner
RELEASE_PLATES = {"1_case", "5_unit"}
FINE_LAYER_RELEASE, FINE_LAYER_TEST = "0.08", "0.12"
FINE_LAYERS = {**{o: 4.5 for o in ("top", "usb_top", "seam_top", "buttons_top")},
               **{o: 2.5 for o in ("tub_print", "usb_tub", "seam_tub", "buttons_tub")}}


def case_plate(plate):
    """Is it a plate of the case kit (given the case settings), not one of the printer tests?"""
    return plate.removesuffix("_a").removesuffix("_b") not in TEST_PLATES


def layer_ranges_xml(plate, ids, same=False):
    """Metadata/layer_config_ranges.xml: the fine layer height (FINE_LAYER_RELEASE on a RELEASE_PLATES plate, else FINE_LAYER_TEST) over each object's first FINE_LAYERS mm; ids: {object's number in the project, from 1: name}.
    same: every object of the plate gets the highest range of them (Bambu Studio won't slice a prime tower unless all the
    plate's objects have the same variable layer height). None if no object of the plate has one."""
    top = max((FINE_LAYERS[n] for n in ids.values() if n in FINE_LAYERS), default=0)
    ranges = {i: (top if same else FINE_LAYERS.get(n, 0)) for i, n in ids.items()}
    objs = "".join(f' <object id="{i}">\n  <range min_z="0" max_z="{h}">\n   <option opt_key="layer_height">{FINE_LAYER_RELEASE if plate in RELEASE_PLATES else FINE_LAYER_TEST}'
                   f'</option>\n  </range>\n </object>\n' for i, h in ranges.items() if h)
    return f'<?xml version="1.0" encoding="utf-8"?>\n<objects>\n{objs}</objects>\n' if objs else None


# each object's own settings (kept as they are: a project lists no preset for an object's)
OBJECT_SETTINGS = {f"first_layer_L{l}_E{e}": {"elefant_foot_compensation": f"0.{e}"}
                   for l in ("20", "12") for e in ("15", "25")}
# The overhang settings Snapmaker Orca 2.4.0 has beyond its defaults (the OrcaSlicer wiki, Quality >
# Overhangs and Speed > Overhang speed): walls over an overhang slower (50 and 30 mm/s at 25 to 50 and
# 50 to 75 % of a line overhanging, now 30 and 15), slower where perimeters may curl, every other
# layer's overhanging walls the other way round (Reverse on even), and extra overhang walls where a
# bridge has no anchor. Detect overhang wall and the fan's overhang boost are on already, the fan at
# 100 % all the time for PLA, so no fan setting would change anything. Not used: Make overhang
# printable (it reshapes the model, which `overhang` in case.scad does already). (Two values: the
# U1's two extruder variants)
U1_OBJECT_SETTINGS = {"overhang_ladder_TUNED": {
    "overhang_2_4_speed": "30,30", "overhang_3_4_speed": "15,15", "slowdown_for_curled_perimeters": "1,1",
    "overhang_reverse": "1", "extra_perimeters_on_overhangs": "1"}}
# the system process preset each plate's project names: the one at its layer height
U1_PRESET = "0.20mm Standard @Snapmaker U1 (0.4 nozzle)"
U1_PLATE_PRESETS = {plate: f"{h}mm Standard @Snapmaker U1 (0.4 nozzle)" for plate, h in LAYERS.items()}
U1_SYSTEM = os.path.join(HERE, "print", "u1_system")
# the overhang tests overhang on purpose (they ask how far the printer goes without supports)
NEVER_SUPPORTED = {f"{KIT}/test_overhangs.stl"} | {p for plate in [*LAYERS, "9_overhang_settings"] for _, p in PLATES[plate]}


def support_parts():
    """The STLs of the parts that get plain PLA supports (overhangs.py flags them: STANDIN_PARTS), after printing every
    part's verdict. Fails when a part that is neither one of those nor a top (TOP_PARTS, which carry support filament)
    needs supports, or a top part's flag is gone (then it isn't one)."""
    import overhangs
    from concurrent.futures import ProcessPoolExecutor
    paths = sorted({p for parts in PLATES.values() for _, p in parts})
    with ProcessPoolExecutor() as ex:
        found = dict(zip(paths, ex.map(overhangs.check, paths)))
    for p in paths:
        print(f"  {os.path.basename(p):38s} " + ("no supports" if not found[p] else
              "; ".join(f"{a:.1f} mm2: {why}" for a, z, xy, why in found[p]))
              + (" (on purpose: printed without)" if found[p] and p in NEVER_SUPPORTED else ""))
    stale = [os.path.basename(p) for p in sorted(TOP_PARTS) if p in found and not found[p]]
    if stale:
        sys.exit(f"{', '.join(stale)} need no supports any more: take them out of TOP_PARTS")
    wrong = [os.path.basename(p) for p in paths if found[p] and p not in NEVER_SUPPORTED | TOP_PARTS | STANDIN_PARTS]
    if wrong:
        sys.exit(f"{', '.join(wrong)} need supports and aren't designed for them (only the top and the stand-in parts are)")
    return {p for p in paths if found[p] and p not in NEVER_SUPPORTED | TOP_PARTS}


V5_NAMES = {"tub_print": "tub_print.stl", "board_mock": "board_mock.stl",
            "panel_dummy": "panel_dummy.stl", "speaker_dummy": "speaker_dummy.stl"}
NS = "http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"


def flatten(profiles, out, names):
    """Write each named preset with everything it inherits, as the slicer resolves it (a vendor's
    profiles, then Orca's shared filament library beside them)."""
    import glob
    found = {}
    for d in (os.path.join(os.path.dirname(profiles), "OrcaFilamentLibrary"), profiles):
        for f in glob.glob(os.path.join(d, "**", "*.json"), recursive=True):
            try:
                p = json.load(open(f, encoding="utf-8-sig"))     # some carry a byte order mark
            except ValueError:
                continue
            if isinstance(p, dict) and "name" in p:
                found[p["name"]] = p

    def resolve(n):
        p = found[n]
        return dict(resolve(p["inherits"]) if p.get("inherits") else {}, **p)
    os.makedirs(out, exist_ok=True)
    for n in names:
        p = {k: v for k, v in resolve(n).items() if k != "inherits"}
        json.dump(p, open(os.path.join(out, n + ".json"), "w"), indent=4)
        print(f"  {n}: {len(p)} settings")


# a preset's own bookkeeping, never a setting a project changes
PRESET_META = {"name", "from", "inherits", "type", "setting_id", "instantiation", "description", "version",
               "renamed_from", "compatible_printers", "compatible_printers_condition", "compatible_prints",
               "compatible_prints_condition", "print_settings_id", "printer_settings_id", "filament_settings_id",
               "filament_id", "inherits_group", "different_settings_to_system"}


def same(a, b):
    """Two settings' values equal, as the slicers write them ("100%" and "100", "1.0" and "1")."""
    def norm(v):
        out = []
        for x in (v if isinstance(v, list) else [v]):
            try:
                out.append(float(str(x).rstrip("%")))
            except ValueError:
                out.append(str(x))
        return out
    return norm(a) == norm(b)


def pla_in_slot_1(cfg):
    """Slot 1 (bay 1) gets the template's PLA filament settings from PLA_SLOT: every per-filament
    list, and the filament's entry in different_settings_to_system (process, filaments, printer)."""
    n, s = len(cfg["filament_settings_id"]), PLA_SLOT - 1
    for k, v in cfg.items():
        if isinstance(v, list) and len(v) == n and k not in NOT_FILAMENT:
            v[0] = v[s]
    cfg["different_settings_to_system"][1] = cfg["different_settings_to_system"][1 + s]
    assert set(cfg["filament_type"]) == {"PLA"}, cfg["filament_type"]


def support_in_slot_2(cfg):
    """Slot SUPPORT_TOOL (bay 2) is Bambu Support For PLA: the template's PLA settings there, marked as a support filament
    (a PLA support filament, density 1.30, white), the changes listed in its entry of different_settings_to_system."""
    s = SUPPORT_TOOL - 1
    cfg["filament_is_support"][s], cfg["filament_density"][s], cfg["filament_colour"][s] = "1", "1.30", "#F2F2F2"
    listed = set(cfg["different_settings_to_system"][1 + s].split(";")) - {""}
    cfg["different_settings_to_system"][1 + s] = ";".join(sorted(listed | {"filament_is_support", "filament_density", "filament_colour"}))


def mark_presets(cfg, system_dir, preset, keys=(), base=None):
    """Name the system process preset `preset` in a project's settings (cfg, changed in place) and
    list in different_settings_to_system[0] every process setting whose value isn't the preset's,
    with the ones already listed and `keys` (the settings the packages set). base: the preset the
    project's settings came from, whose settings the named preset may not set at all (Snapmaker's
    0.20 mm sets eight its other heights leave to the defaults). Returns the list."""
    def load(n):
        return json.load(open(os.path.join(system_dir, n + ".json")))
    system = load(preset)
    process = set(system) | set(load(base) if base else ())
    listed = set(cfg.get("different_settings_to_system", [""])[0].split(";")) - {""}
    diff = {k for k in system if k not in PRESET_META and k in cfg and not same(system[k], cfg[k])}
    diff |= {k for k in process - set(system) if k not in PRESET_META and k in cfg}
    diff |= {k for k in listed | set(keys) if k in process or k in cfg and k not in PRESET_META}   # (a setting the preset leaves at its default isn't in its file)
    cfg["print_settings_id"] = preset
    cfg["different_settings_to_system"][0] = ";".join(sorted(diff))
    return sorted(diff)


def plate_dir(plate, case_dir, printer):
    """Where a plate's project goes: the case kit's folder, or the printer's tests folder."""
    return TESTS[printer] if plate in TEST_PLATES else case_dir


def fresh(dirs):
    """Empty each folder, with an stl/ in it."""
    for d in dirs:
        shutil.rmtree(d, ignore_errors=True)
        os.makedirs(os.path.join(d, "stl"))


def tests_zip():
    """overhang_kit.zip: both printers' test plates (P1S/, U1/) as far as they're built, renamed and in
    TESTS_ORDER's order, and their README."""
    with zipfile.ZipFile(TESTS_ZIP, "w", zipfile.ZIP_DEFLATED) as o:
        for printer, d in TESTS.items():
            for name, plate in TESTS_ORDER.items():
                if os.path.exists(os.path.join(d, plate + ".3mf")):
                    o.write(os.path.join(d, plate + ".3mf"), f"overhang_kit/{printer}/{name}.3mf")
        o.write(TESTS_README, "overhang_kit/README.md")
        n = sum(1 for i in o.namelist() if i.endswith(".3mf"))
    print(f"{TESTS_ZIP} ({n} plates, {os.path.getsize(TESTS_ZIP) // 1024} kB)")


def tower_keepout(parts, printer="U1"):
    """The prime tower's corner of a plate with a top (a second filament to change to): TOWER_AT, TOWER_W wide and about
    TOWER_D deep, plus its brim and a margin. None on a plate without one (a single filament has no tower)."""
    if not has_top(parts):
        return []
    x, y = TOWER_AT[printer]
    return [(x - 8, y - 8, x + TOWER_W + 8, y + TOWER_D + 8)]


def pack(sizes, taken, bed=BED):
    """Shelf-pack footprints (w, l) onto the bed, longest first, avoiding the rectangles already
    taken; returns their centres in the given order."""
    spots, x, y, row = {}, GAP, GAP, 0.0
    for i in sorted(range(len(sizes)), key=lambda i: -sizes[i][1]):
        w, l = sizes[i]
        while True:
            if x + w > bed - GAP:
                x, y, row = GAP, y + row + GAP, 0.0
            if y + l > bed - GAP:
                raise SystemExit("plate overflows the bed: split it")
            box = (x, y, x + w, y + l)
            hit = next((t for t in taken if box[0] < t[2] + GAP and t[0] < box[2] + GAP and
                        box[1] < t[3] + GAP and t[1] < box[3] + GAP), None)
            if hit:
                x = hit[2] + GAP
                continue
            spots[i] = (x + w / 2, y + l / 2)
            taken.append(box)
            x += w + GAP
            row = max(row, l)
            break
    return [spots[i] for i in range(len(sizes))]


def mesh_xml(oid, uuid, m):
    v = m.vertices
    rows = "\n".join(f'     <vertex x="{a:.5g}" y="{b:.5g}" z="{c:.5g}"/>' for a, b, c in v)
    tris = "\n".join(f'     <triangle v1="{a}" v2="{b}" v3="{c}"/>' for a, b, c in m.faces)
    return (f' <resources>\n  <object id="{oid}" p:UUID="{uuid}" type="model">\n   <mesh>\n    <vertices>\n{rows}\n'
            f'    </vertices>\n    <triangles>\n{tris}\n    </triangles>\n   </mesh>\n  </object>\n </resources>\n'
            f' <build/>\n</model>\n')


def build(plate, parts, z, supported):
    """One plate's project; supported: the STLs that get supports (support_parts)."""
    model = z.read("3D/3dmodel.model").decode()
    head = model[:model.index("<resources>")]
    some_part = next(n for n in z.namelist() if n.startswith("3D/Objects/"))
    part_xml = z.read(some_part).decode()
    part_head = part_xml[:part_xml.index("<resources>")]
    meshes = []
    for name, path in parts:
        m = load_part(plate, name, path)
        m.merge_vertices()
        lo, hi = m.bounds
        m.apply_translation(-(lo + hi) / 2)          # centred on its footprint and its height
        meshes.append((name, m, hi - lo))
    taken, spots = tower_keepout(parts), {}
    for (n, s), xy in zip([(n, s) for n, m, s in meshes], pack([(s[0], s[1]) for n, m, s in meshes], taken)):
        spots[n] = xy
    files, resources, items, settings, instances, rels = {}, [], [], [], [], []
    for i, (name, m, size) in enumerate(meshes, 1):
        mid, oid = 2 * i - 1, 2 * i
        path = f"/3D/Objects/{name}.stl_{i}.model"
        files[path.lstrip("/")] = part_head + mesh_xml(mid, f"{i:08x}-81cb-4c03-9d28-80fed5dfa1dc", m)
        resources.append(f'  <object id="{oid}" p:UUID="{i:08x}-61cb-4c03-9d28-80fed5dfa1dc" type="model">\n'
                         f'   <components>\n    <component p:path="{path}" objectid="{mid}" '
                         f'p:UUID="{i:04x}0000-b206-40ff-9872-83e8017abed1" transform="1 0 0 0 1 0 0 0 1 0 0 0"/>\n'
                         f'   </components>\n  </object>')
        x, y = spots[name]
        items.append(f'  <item objectid="{oid}" p:UUID="{oid:08x}-b1ec-4553-aec9-835e5b724bb4" '
                     f'transform="1 0 0 0 1 0 0 0 1 {x:.4f} {y:.4f} {size[2] / 2:.4f}" printable="1"/>')
        stl_name = V5_NAMES.get(name, name + ".stl")
        stl = dict(parts)[name]
        own = dict(OBJECT_SUPPORT if stl in supported else TOP_SUPPORT if stl in TOP_PARTS else {}, **OBJECT_SETTINGS.get(name, {}),
                   **U1_OBJECT_SETTINGS.get(name, {}))
        own = "".join(f'    <metadata key="{k}" value="{v}"/>\n' for k, v in own.items())
        settings.append(f'  <object id="{oid}">\n    <metadata key="name" value="{stl_name}"/>\n{own}'
                        f'    <metadata key="extruder" value="{PLA}"/>\n    <part id="{mid}" subtype="normal_part">\n'
                        f'      <metadata key="name" value="{stl_name}"/>\n'
                        f'      <metadata key="matrix" value="1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1"/>\n'
                        f'      <metadata key="source_file" value="{stl_name}"/>\n'
                        f'      <metadata key="source_object_id" value="0"/>\n      <metadata key="source_volume_id" value="0"/>\n'
                        f'      <metadata key="source_offset_x" value="0"/>\n      <metadata key="source_offset_y" value="0"/>\n'
                        f'      <metadata key="source_offset_z" value="{size[2] / 2:.4f}"/>\n'
                        f'      <mesh_stat edges_fixed="0" degenerate_facets="0" facets_removed="0" facets_reversed="0" backwards_edges="0"/>\n'
                        f'    </part>\n  </object>')
        instances.append(f'    <model_instance>\n      <metadata key="object_id" value="{oid}"/>\n'
                         f'      <metadata key="instance_id" value="0"/>\n      <metadata key="identify_id" value="{800 + 11 * i}"/>\n'
                         f'    </model_instance>')
        rels.append(f' <Relationship Target="{path}" Id="rel-{i}" Type="{NS}"/>')
    files["3D/3dmodel.model"] = (head + "<resources>\n" + "\n".join(resources) + "\n </resources>\n <build>\n"
                                 + "\n".join(items) + "\n </build>\n</model>\n")
    files["3D/_rels/3dmodel.model.rels"] = ('<?xml version="1.0" encoding="UTF-8"?>\n<Relationships xmlns='
                                            '"http://schemas.openxmlformats.org/package/2006/relationships">\n'
                                            + "\n".join(rels) + "\n</Relationships>")
    ranges = layer_ranges_xml(plate, {i: n for i, (n, _) in enumerate(parts, 1)}, has_top(parts)) if case_plate(plate) else None
    if ranges:
        files["Metadata/layer_config_ranges.xml"] = ranges
    cfg = z.read("Metadata/model_settings.config").decode()
    plate_xml = cfg[cfg.index("  <plate>"):cfg.index("  </plate>")]
    plate_xml = plate_xml[:plate_xml.index("    <model_instance>")] + "\n".join(instances) + "\n"
    files["Metadata/model_settings.config"] = ('<?xml version="1.0" encoding="UTF-8"?>\n<config>\n' + "\n".join(settings)
                                               + "\n" + plate_xml + "  </plate>\n</config>\n")
    out = os.path.join(plate_dir(plate, OUT, "U1"), f"{plate}.3mf")
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as o:
        for n in z.namelist():
            if n.startswith("3D/") or n == "Metadata/model_settings.config":
                continue
            # the template's plate pictures show v5: leave them out, Orca draws new ones on save
            if n.endswith(".png"):
                continue
            data = z.read(n)
            if n == "Metadata/project_settings.config":
                cfg = json.loads(data)
                pla_in_slot_1(cfg)
                support_in_slot_2(cfg)
                tower = {"enable_prime_tower": "1", "prime_tower_width": str(TOWER_W), "prime_tower_brim_width": "3"} if has_top(parts) else {}
                case = dict(CASE_SETTINGS) if case_plate(plate) else {}      # (the print quality of the case kit)
                for k, v in dict(SUPPORT, enable_support="0", **tower, **PLATE_SETTINGS.get(plate, {}), **case).items():   # supports per object
                    if k not in cfg:
                        sys.exit(f"{k} isn't a setting in the template's Orca: check the name")
                    cfg[k] = v
                if tower:                                  # (lists: one value per extruder variant)
                    cfg["wipe_tower_x"], cfg["wipe_tower_y"] = [str(TOWER_AT["U1"][0])], [str(TOWER_AT["U1"][1])]
                for k in [*OBJECT_SUPPORT, *TOP_SUPPORT, *(k for o in [*OBJECT_SETTINGS.values(), *U1_OBJECT_SETTINGS.values()] for k in o)]:
                    if k not in cfg:
                        sys.exit(f"{k} isn't a setting in the template's Orca: check the name")
                mark_presets(cfg, U1_SYSTEM, U1_PLATE_PRESETS.get(plate, U1_PRESET),
                             set(SUPPORT) | set(tower) | {"wipe_tower_x", "wipe_tower_y"} | set(PLATE_SETTINGS.get(plate, {})) | set(case), U1_PRESET)
                data = json.dumps(cfg, indent=4).encode()
            if n == "_rels/.rels":
                data = re.sub(rb'\s*<Relationship Target="/Metadata/[^"]+\.png"[^>]*/>', b"", data)
            if n == "3D/3dmodel.model":
                continue
            o.writestr(n, data)
        for n, text in files.items():
            o.writestr(n, text)
    return out, [(n, s) for n, m, s in meshes]


def build_plate(job):
    """build() for one plate in a worker process, with its own handle on the template."""
    plate, parts, supported = job
    with zipfile.ZipFile(TEMPLATE) as z:
        return build(plate, parts, z, supported)


def main():
    if sys.argv[1:2] == ["--flatten"]:
        return flatten(sys.argv[2], sys.argv[3], sys.argv[4:])
    if sys.argv[1:] == ["--tests"]:
        return tests_zip()
    for plate, parts in PLATES.items():
        if sys.argv[1:] and plate not in sys.argv[1:]:
            continue
        for _, p in parts:
            if not os.path.exists(p):
                sys.exit(f"missing {p}: run case/export.py --stl, the print_variants tub and case/fitkit.py first")
    print("supports, measured part by part:")
    supported = support_parts()
    only = sys.argv[1:]                 # plate names to build (default: all)
    jobs = [(plate, parts, supported) for plate, parts in PLATES.items() if not only or plate in only]
    case = any(plate not in TEST_PLATES for plate, _, _ in jobs)      # the case kit or the tests, or both
    tests = any(plate in TEST_PLATES for plate, _, _ in jobs)
    fresh(([OUT] if case else []) + ([TESTS["U1"]] if tests else []))
    from concurrent.futures import ProcessPoolExecutor
    with ProcessPoolExecutor() as ex:   # the plates build at once, listed in order
        built = list(ex.map(build_plate, jobs))
    for (plate, parts, _), (path, sizes) in zip(jobs, built):
        print(f"{os.path.basename(path):24s} " + ", ".join(n + (" (supports)" if p in supported else "")
                                                           for n, p in parts))
        for n, p in parts:
            shutil.copy(p, os.path.join(plate_dir(plate, OUT, "U1"), "stl", os.path.basename(p)))
    if tests:
        shutil.copy(TESTS_README, os.path.join(TESTS["U1"], "README.md"))
        tests_zip()
    if not case:
        return
    for extra in ("top.stl", "tub.stl"):
        if os.path.exists(os.path.join(HERE, extra)) and not sys.argv[1:]:
            shutil.copy(os.path.join(HERE, extra), os.path.join(OUT, "stl", extra))
    shutil.copy(os.path.join(HERE, "print_package_README.md"), os.path.join(OUT, "README.md"))
    with zipfile.ZipFile(ZIP, "w", zipfile.ZIP_DEFLATED) as o:
        for root, _, names in os.walk(OUT):
            for n in names:
                p = os.path.join(root, n)
                o.write(p, os.path.join("mega_blastoise_print", os.path.relpath(p, OUT)))
    print(f"{ZIP} ({os.path.getsize(ZIP) // 1024} kB)")


if __name__ == "__main__":
    main()
