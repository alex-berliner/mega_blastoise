#!/usr/bin/env python3
"""The 3D print package: Snapmaker Orca projects ready to slice, the STLs, and a README.

    python3 case/print_package.py      # after case/export.py --stl and case/fitkit.py

Each project is built from Alex's own fit_test_v5.3mf (print/), so it keeps his printer,
filament, toolhead and plate settings (Snapmaker U1, 0.20 mm Standard, PLA on toolhead 2,
PETG support interface on toolhead 1, textured PEI): only the objects and where they sit on
the plate change. Orca's CLI can't load U1 projects, so the projects are written the way
Orca writes them (one object file per part, mesh centred on its own height, placed by the
build item) and the STLs ship beside them in case a project won't open.

Writes print/package/ and print/mega_blastoise_print_package.zip (build output, not in git).
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
OUT = os.path.join(HERE, "print", "package")
ZIP = os.path.join(HERE, "print", "mega_blastoise_print_package.zip")
BED = 270.0
GAP = 6.0
PLA = 2                      # the template's toolhead for PLA parts

KIT = os.path.join(HERE, "fitkit")
PRINT_TUB = os.path.join(HERE, "print", "tub_print.stl")

# Supports, changed from the template (Alex: keep the PETG, but touch the decks as little and as
# cleanly as possible): organic trees in PLA from the build plate only, so nothing grows inside the
# lid; PETG only for the 2 solid interface layers right under the decks (fewer toolhead swaps and
# purges), no gap and ironed, so the PLA deck face prints onto a smooth surface instead of
# sagging between sparse lines
SUPPORT = {"enable_support": "1", "support_type": "tree(auto)", "support_style": "organic",
           "support_filament": "2", "support_interface_filament": "1", "support_interface_not_for_body": "1",
           "support_interface_top_layers": "2", "support_interface_spacing": "0", "support_top_z_distance": "0",
           "support_ironing": "1", "support_on_build_plate_only": "1"}

# plate name: [(object name, STL), ...]; the first plate keeps v5's own layout for its parts
PLATES = {
    "0_buttons_end": [(n, f"{KIT}/coupon_{n}.stl") for n in ("buttons_tub", "buttons_board", "buttons_lid")],
    "1_case": [("tub_print", PRINT_TUB), ("lid", os.path.join(HERE, "lid.stl")),
               ("board_mock", f"{KIT}/board_mock.stl"), ("panel_dummy", f"{KIT}/panel_dummy.stl"),
               ("speaker_dummy", f"{KIT}/speaker_dummy.stl"), ("cell_dummy", f"{KIT}/cell_dummy.stl"),
               ("cell_plug_dummy", f"{KIT}/cell_plug_dummy.stl")],
    "2_buttons_quick": [("tune_openings", f"{KIT}/tune_openings.stl")] +
        [(f"tune_stack_{c}_board", f"{KIT}/tune_stack_{c}_board.stl") for c in ("dpad", "abxy", "ss")] +
        [(f"tune_stack_{c}_lid_dome1.75", f"{KIT}/tune_stack_{c}_lid_dome1.75.stl") for c in ("dpad", "abxy", "ss")],
    "3_buttons_heights": [(f"tune_stack_{c}_lid_dome{d}", f"{KIT}/tune_stack_{c}_lid_dome{d}.stl")
                          for c in ("dpad", "abxy", "ss") for d in ("1.6", "1.9")],
    "4_fit_coupons": [(n, f"{KIT}/coupon_{n}.stl") for n in
                      ("usb_tub", "usb_lid", "usb_board", "snap_tub", "snap_lid", "frame_lid",
                       "buttons_tub", "buttons_board", "buttons_lid")],
    # EXPERIMENT (case.scad: deck_inserts): seat 1's end with its deck as a separate insert, no supports
    "5_inserts_experiment": [(n, f"{KIT}/coupon_{n}.stl") for n in
                             ("buttons_tub_inserts", "buttons_board", "buttons_lid_inserts", "buttons_insert")],
}
NO_SUPPORT = {"5_inserts_experiment"}   # plates that print without any supports
V5_NAMES = {"tub_print": "tub_print.stl", "lid": "lid.stl", "board_mock": "board_mock.stl",
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
    """The prime tower's corner of the plate (two toolheads: PLA, and PETG for support interface).
    Orca puts it at wipe_tower_x/y with prime_tower_width; its depth depends on the purge, so keep
    a generous 45 mm, plus the brim and ribs round it."""
    cfg = json.loads(z.read("Metadata/project_settings.config"))
    if str(cfg.get("enable_prime_tower", "0")) != "1":
        return []
    x, y = float(cfg["wipe_tower_x"][0]), float(cfg["wipe_tower_y"][0])
    w = float(cfg.get("prime_tower_width", 30))
    m = float(cfg.get("prime_tower_brim_width", 5)) + 3
    return [(x - m, y - m, x + w + m, y + 45 + m)]


def pack(sizes, taken):
    """Shelf-pack footprints (w, l) onto the bed, avoiding the rectangles already taken."""
    spots, x, y, row = [], GAP, GAP, 0.0
    for w, l in sizes:
        while True:
            if x + w > BED - GAP:
                x, y, row = GAP, y + row + GAP, 0.0
            if y + l > BED - GAP:
                raise SystemExit("plate overflows the bed: split it")
            box = (x, y, x + w, y + l)
            hit = next((t for t in taken if box[0] < t[2] + GAP and t[0] < box[2] + GAP and
                        box[1] < t[3] + GAP and t[1] < box[3] + GAP), None)
            if hit:
                x = hit[2] + GAP
                continue
            spots.append((x + w / 2, y + l / 2))
            taken.append(box)
            x += w + GAP
            row = max(row, l)
            break
    return spots


def mesh_xml(oid, uuid, m):
    v = m.vertices
    rows = "\n".join(f'     <vertex x="{a:.5g}" y="{b:.5g}" z="{c:.5g}"/>' for a, b, c in v)
    tris = "\n".join(f'     <triangle v1="{a}" v2="{b}" v3="{c}"/>' for a, b, c in m.faces)
    return (f' <resources>\n  <object id="{oid}" p:UUID="{uuid}" type="model">\n   <mesh>\n    <vertices>\n{rows}\n'
            f'    </vertices>\n    <triangles>\n{tris}\n    </triangles>\n   </mesh>\n  </object>\n </resources>\n'
            f' <build/>\n</model>\n')


def build(plate, parts, z):
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
        settings.append(f'  <object id="{oid}">\n    <metadata key="name" value="{stl_name}"/>\n'
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
                for k, v in (dict(SUPPORT, enable_support="0") if plate in NO_SUPPORT else SUPPORT).items():
                    if k not in cfg:
                        sys.exit(f"{k} isn't a setting in the template's Orca: check the name")
                    cfg[k] = v
                data = json.dumps(cfg, indent=4).encode()
            if n == "_rels/.rels":
                data = re.sub(rb'\s*<Relationship Target="/Metadata/[^"]+\.png"[^>]*/>', b"", data)
            if n == "3D/3dmodel.model":
                continue
            o.writestr(n, data)
        for n, text in files.items():
            o.writestr(n, text)
    return out, [(n, s) for n, m, s in meshes]


def main():
    for plate, parts in PLATES.items():
        if sys.argv[1:] and plate not in sys.argv[1:]:
            continue
        for _, p in parts:
            if not os.path.exists(p):
                sys.exit(f"missing {p}: run case/export.py --stl, the print_variants tub and case/fitkit.py first")
    shutil.rmtree(OUT, ignore_errors=True)
    os.makedirs(os.path.join(OUT, "stl"))
    z = zipfile.ZipFile(TEMPLATE)
    only = sys.argv[1:]                 # plate names to build (default: all)
    for plate, parts in PLATES.items():
        if only and plate not in only:
            continue
        path, sizes = build(plate, parts, z)
        print(f"{os.path.basename(path):24s} " + ", ".join(n for n, _ in sizes))
        for n, p in parts:
            shutil.copy(p, os.path.join(OUT, "stl", os.path.basename(p)))
    for extra in ("lid.stl", "tub.stl"):
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
