#!/usr/bin/env python3
"""The 3D print package: Snapmaker Orca projects ready to slice, the STLs, and a README.

    python3 case/print_package.py      # after case/export.py --stl and case/fitkit.py

Each project is built from Alex's own fit_test_v5.3mf (print/), so it keeps his printer,
filament and plate settings (Snapmaker U1, 0.20 mm Standard, textured PEI), PLA only on
toolhead 2 (Alex, 2026-10-20: no PETG, no second toolhead, no prime tower): only the objects,
where they sit on the plate, and the supports change. Orca's CLI can't load U1 projects, so the projects are written the way
Orca writes them (one object file per part, mesh centred on its own height, placed by the
build item) and the STLs ship beside them in case a project won't open.

Supports are on only for the parts that need them, measured by overhangs.py: per object, not
per plate (a plate's supports filled the tub's screw counterbores, 2026-10-21).
print_package_p1s.py builds the same plates for a Bambu Lab P1S.

Writes print/package_U1/ and print/mega_blastoise_print_U1.zip (build output, not in git).
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
PLA = 2                      # the template's toolhead for PLA parts

KIT = os.path.join(HERE, "fitkit")
PRINT_TUB = os.path.join(HERE, "tub.stl")    # prints flat as exported (no feet since 2026-10-20)

# PLA only (Alex, 2026-10-20): every object, support and interface on the PLA toolhead, and no
# prime tower (one filament never changes). The faces print on the bed, not on supports,
# so where a plate does need supports they're plain PLA organic trees from the build plate only,
# with the usual 0.2 mm gap (PLA on PLA with none would fuse), and bridges aren't supported
SUPPORT = {"enable_support": "1", "support_type": "tree(auto)", "support_style": "organic",
           "support_filament": str(PLA), "support_interface_filament": str(PLA),
           "support_interface_not_for_body": "0", "support_interface_top_layers": "2",
           "support_interface_spacing": "0.5", "support_top_z_distance": "0.2", "support_ironing": "0",
           "support_on_build_plate_only": "1", "bridge_no_support": "1", "enable_prime_tower": "0"}
# Supports are per object, not per plate: with the plate's on, the slicer also put them in every
# part's small cavities (the tub's six screw counterbores, 4.4 mm wide and 1.0 deep at the bed,
# came out plugged: Alex's print, 2026-10-21). So the plate's are off (SUPPORT's other keys stay,
# they only say what kind) and each part overhangs.py says needs them gets these. The 45 degree
# threshold and keeping small overhangs are overhangs.py's own measure: at the slicers' defaults
# (30 degrees, small ones dropped) Bambu Studio gave the board mock's USB-C stand-in nothing
OBJECT_SUPPORT = {"enable_support": "1", "support_threshold_angle": "45", "support_remove_small_overhang": "0",
                  "support_on_build_plate_only": "1", "bridge_no_support": "1"}

# plate name: [(object name, STL), ...]; the first plate keeps v5's own layout for its parts
PLATES = {
    "0_buttons_end": [(n, f"{KIT}/coupon_{n}.stl") for n in ("buttons_tub", "buttons_board", "buttons_top", "buttons_deck")],
    "1_case": [("tub_print", PRINT_TUB), ("top", os.path.join(HERE, "top.stl")),
               ("board_mock", f"{KIT}/board_mock.stl"), ("panel_dummy", f"{KIT}/panel_dummy.stl"),
               ("speaker_dummy", f"{KIT}/speaker_dummy.stl"), ("cell_dummy", f"{KIT}/cell_dummy.stl"),
               ("cell_plug_dummy", f"{KIT}/cell_plug_dummy.stl")],
    "1_case_decks": [(n, os.path.join(HERE, "deck.stl")) for n in ("deck_a", "deck_b")],   # one part, two of it
    "2_buttons_quick": [("tune_openings", f"{KIT}/tune_openings.stl")] +
        [(f"tune_stack_{c}_board", f"{KIT}/tune_stack_{c}_board.stl") for c in ("dpad", "abxy", "ss")] +
        [(f"tune_stack_{c}_lid_dome1.75", f"{KIT}/tune_stack_{c}_lid_dome1.75.stl") for c in ("dpad", "abxy", "ss")],
    "3_buttons_heights": [(f"tune_stack_{c}_lid_dome{d}", f"{KIT}/tune_stack_{c}_lid_dome{d}.stl")
                          for c in ("dpad", "abxy", "ss") for d in ("1.6", "1.9")],
    "4_fit_coupons": [(n, f"{KIT}/coupon_{n}.stl") for n in
                      ("usb_tub", "usb_top", "usb_board", "seam_tub", "seam_top", "frame_top",
                       "buttons_tub", "buttons_board", "buttons_top", "buttons_deck")],
    "5_unit": [("tub_print", PRINT_TUB), ("top", os.path.join(HERE, "top.stl")),     # one whole unit, no dummies
               ("deck_a", os.path.join(HERE, "deck.stl")), ("deck_b", os.path.join(HERE, "deck.stl")),
               ("board_mock", f"{KIT}/board_mock.stl")],
    # questions for the shell's redesign, no supports: openings, overhangs, screw holes, a column's clearance
    "6_tests": [("tune_openings", f"{KIT}/tune_openings.stl"), ("test_overhangs", f"{KIT}/test_overhangs.stl"),
                ("screws_tub", f"{KIT}/coupon_screws_tub.stl"), ("screws_top", f"{KIT}/coupon_screws_top.stl"),
                ("test_column", f"{KIT}/test_column.stl"), ("test_rings", f"{KIT}/test_rings.stl")],
    # the overhang kit, no supports: the steepest clean overhang, straight and round, at three
    # layer heights (the height on each part's pad); only the layer height differs from the rest
    **{f"7_overhangs_{h}": [(f"overhang_ladder_{h}", f"{KIT}/test_overhang_ladder_{h}.stl"),
                            (f"overhang_rounds_{h}", f"{KIT}/test_overhang_rounds_{h}.stl")]
       for h in ("020", "016", "012")},
}
# each plate's own settings over the project's (the slicers' names; the same in Orca and Bambu Studio)
PLATE_SETTINGS = {f"7_overhangs_{h}": {"layer_height": f"0.{h[1:]}"} for h in ("020", "016", "012")}
# the overhang tests overhang on purpose (they ask how far the printer goes without supports)
NEVER_SUPPORTED = {f"{KIT}/test_overhangs.stl"} | {p for h in ("020", "016", "012") for _, p in PLATES[f"7_overhangs_{h}"]}


def support_parts():
    """The STLs of the parts that need supports (overhangs.py), after printing every part's verdict."""
    import overhangs
    from concurrent.futures import ProcessPoolExecutor
    paths = sorted({p for parts in PLATES.values() for _, p in parts})
    with ProcessPoolExecutor() as ex:
        found = dict(zip(paths, ex.map(overhangs.check, paths)))
    for p in paths:
        print(f"  {os.path.basename(p):38s} " + ("no supports" if not found[p] else
              "; ".join(f"{a:.1f} mm2: {why}" for a, z, xy, why in found[p]))
              + (" (on purpose: printed without)" if found[p] and p in NEVER_SUPPORTED else ""))
    return {p for p in paths if found[p] and p not in NEVER_SUPPORTED}


V5_NAMES = {"tub_print": "tub_print.stl", "board_mock": "board_mock.stl",
            "panel_dummy": "panel_dummy.stl", "speaker_dummy": "speaker_dummy.stl"}
NS = "http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"


def template_layout(z):
    """Where v5 put each of its parts: bed centre (x, y) of each part's footprint."""
    model = z.read("3D/3dmodel.model").decode()
    paths = dict(re.findall(r'<object id="(\d+)"[^>]*>\s*<components>\s*<component p:path="([^"]+)"', model))
    out = {}
    for oid, t in re.findall(r'<item objectid="(\d+)"[^>]*transform="([^"]+)"', model):
        tx, ty = map(float, t.split()[9:11])
        part = z.read(paths[oid].lstrip("/")).decode()
        v = np.array(re.findall(r'<vertex x="([^"]+)" y="([^"]+)" z="[^"]+"', part), float)
        cx, cy = (v.min(0) + v.max(0)) / 2
        name = os.path.basename(paths[oid]).split(".stl")[0]
        out[name] = (tx + cx, ty + cy)
    return out


def tower_keepout(z):
    """The prime tower's corner of the plate, if the projects keep one (the template's two
    toolheads did; PLA only, they don't). Orca puts it at wipe_tower_x/y with prime_tower_width;
    its depth depends on the purge, so keep a generous 45 mm, plus the brim and ribs round it."""
    cfg = dict(json.loads(z.read("Metadata/project_settings.config")), **SUPPORT)
    if str(cfg.get("enable_prime_tower", "0")) != "1":
        return []
    x, y = float(cfg["wipe_tower_x"][0]), float(cfg["wipe_tower_y"][0])
    w = float(cfg.get("prime_tower_width", 30))
    m = float(cfg.get("prime_tower_brim_width", 5)) + 3
    return [(x - m, y - m, x + w + m, y + 45 + m)]


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
    layout = template_layout(z) if plate == "1_case" else {}
    meshes = []
    for name, path in parts:
        m = trimesh.load(path, force="mesh")
        m.merge_vertices()
        lo, hi = m.bounds
        m.apply_translation(-(lo + hi) / 2)          # centred on its footprint and its height
        meshes.append((name, m, hi - lo))
    taken, spots = tower_keepout(z), {}
    for name, m, size in meshes:
        if name in layout:
            cx, cy = layout[name]
            spots[name] = (cx, cy)
            taken.append((cx - size[0] / 2, cy - size[1] / 2, cx + size[0] / 2, cy + size[1] / 2))
    rest = [(n, s) for n, m, s in meshes if n not in spots]
    for (n, s), xy in zip(rest, pack([(s[0], s[1]) for n, s in rest], taken)):
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
        own = "".join(f'    <metadata key="{k}" value="{v}"/>\n' for k, v in OBJECT_SUPPORT.items()) \
            if dict(parts)[name] in supported else ""
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
    cfg = z.read("Metadata/model_settings.config").decode()
    plate_xml = cfg[cfg.index("  <plate>"):cfg.index("  </plate>")]
    plate_xml = plate_xml[:plate_xml.index("    <model_instance>")] + "\n".join(instances) + "\n"
    files["Metadata/model_settings.config"] = ('<?xml version="1.0" encoding="UTF-8"?>\n<config>\n' + "\n".join(settings)
                                               + "\n" + plate_xml + "  </plate>\n</config>\n")
    out = os.path.join(OUT, f"{plate}.3mf")
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
                for k, v in dict(SUPPORT, enable_support="0", **PLATE_SETTINGS.get(plate, {})).items():   # supports per object
                    if k not in cfg:
                        sys.exit(f"{k} isn't a setting in the template's Orca: check the name")
                    cfg[k] = v
                for k in OBJECT_SUPPORT:
                    if k not in cfg:
                        sys.exit(f"{k} isn't a setting in the template's Orca: check the name")
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
    for plate, parts in PLATES.items():
        if sys.argv[1:] and plate not in sys.argv[1:]:
            continue
        for _, p in parts:
            if not os.path.exists(p):
                sys.exit(f"missing {p}: run case/export.py --stl, the print_variants tub and case/fitkit.py first")
    shutil.rmtree(OUT, ignore_errors=True)
    os.makedirs(os.path.join(OUT, "stl"))
    print("supports, measured part by part:")
    supported = support_parts()
    only = sys.argv[1:]                 # plate names to build (default: all)
    jobs = [(plate, parts, supported) for plate, parts in PLATES.items() if not only or plate in only]
    from concurrent.futures import ProcessPoolExecutor
    with ProcessPoolExecutor() as ex:   # the plates build at once, listed in order
        built = list(ex.map(build_plate, jobs))
    for (plate, parts, _), (path, sizes) in zip(jobs, built):
        print(f"{os.path.basename(path):24s} " + ", ".join(n + (" (supports)" if p in supported else "")
                                                           for n, p in parts))
        for n, p in parts:
            shutil.copy(p, os.path.join(OUT, "stl", os.path.basename(p)))
    for extra in ("top.stl", "deck.stl", "tub.stl"):
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
