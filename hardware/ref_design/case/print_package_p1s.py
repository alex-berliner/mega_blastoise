#!/usr/bin/env python3
"""The 3D print package for a Bambu Lab P1S: the same plates as print_package.py, sliced.

    BAMBU_STUDIO=/path/to/BambuStudio python3 case/print_package_p1s.py [plate ...]

after case/export.py --stl and case/fitkit.py. BAMBU_STUDIO is Bambu Studio's executable (an
AppImage's AppRun, extracted with --appimage-extract, works headless). Each plate is laid out
(print_package's packing) and sliced by Bambu Studio's command line into a project with its G-code in it, ready to send.

Two filaments since the one-piece top (Alex, 2026-10-25), as the U1 package: Bambu PLA Basic in AMS slot 1 for every part
and Bambu Support For PLA in slot 2 (print/p1s/support.json), as the interface of the top's supports only. The profiles
are in print/p1s/ (Bambu's system presets for a P1S, 0.4 mm nozzle, 0.20 mm Standard, flattened), plate-level supports
off. Supports are per object, as in the U1 package: the parts overhangs.py says need them get print_package.OBJECT_SUPPORT
(plain PLA tree supports from the build plate; the board mock's USB-C stand-in) and the top, with the coupons cut from it
that have its slope or rounds (print_package.TOP_PARTS), print_package.TOP_SUPPORT (tree supports whose body is PLA and
whose interface, the layers touching the part, is the support filament; top Z distance 0) with a prime tower, so each plate
is sliced in two steps: Bambu Studio writes the plate as a project, the parts' own settings go into it, and Bambu Studio
slices that. Then support_check.py reads the G-code and the build fails if a part that needs no supports got some (a plate's
supports filled the tub's screw counterbores, 2026-10-21) or if the support filament prints anything but interface and the
tower. A plate that doesn't fit the P1S's 256 mm bed is split.
Bambu Studio's CLI lists no changed settings in its projects, so each project then names its
process preset (P1S_PRESETS, the one at its layer height; Bambu's X1C presets are the P1S's) and
lists what it changes from it (print_package.mark_presets), or Bambu Studio would load the preset's.

Writes print/package_P1S/ and print/mega_blastoise_print_P1S.zip (build output, not in git); the
printer tests (print_package.TEST_PLATES) go to print/tests_P1S/ and print/overhang_kit.zip instead.
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import zipfile
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import print_package  # noqa: E402
import support_check  # noqa: E402

SLICERS = max(1, (os.cpu_count() or 4) // 4)      # Bambu Studios slicing at once

PROFILES = os.path.join(HERE, "print", "p1s")
OUT = os.path.join(HERE, "print", "package_P1S")
ZIP = os.path.join(HERE, "print", "mega_blastoise_print_P1S.zip")
README = os.path.join(HERE, "print_package_README.md")    # its <!-- printer --> part replaced
PRINTER = """Each `.3mf` is a Bambu Studio project for a Bambu Lab P1S (0.4 mm nozzle, 0.20 mm
Standard), arranged and already sliced: open it and send it, or re-slice it after
changing anything. Filaments as above: **Bambu PLA Basic in AMS slot 1** (every part) and **Bambu Support For PLA in AMS slot 2**
(only on the plates with the top or a coupon cut from it, where it is the layers of the supports that touch the part, with a prime
tower at the front right of the bed); the other plates are PLA only. (`mega_blastoise_print_U1.zip` has the same plates for the
Snapmaker U1.) Set the plate type in Bambu Studio to the plate you use.

Weights are what the slicer reports: the part and its supports, then the **purge** (the flush at each change and the prime
tower, with the start line; it is part of the weight shown, not added to it) and the number of **filament changes**.

| Project | Supports | PLA | Support filament | Purge | Changes | Time | Parts |
|---|---|---|---|---|---|---|---|
<!-- PLATES -->
"""


BED = 256.0
EXCLUDE = (0, 0, 18, 28)       # the P1S's no-print corner, front left (machine.json bed_exclude_area)
SUPPORT_PROFILE = os.path.join(PROFILES, "support.json")    # Bambu Support For PLA @BBL X1C, flattened
TOWER_KEYS = {"enable_prime_tower": "1", "prime_tower_width": str(print_package.TOWER_W), "prime_tower_brim_width": "3",
              "wipe_tower_x": str(print_package.TOWER_AT["P1S"][0]), "wipe_tower_y": str(print_package.TOWER_AT["P1S"][1])}


SYSTEM = os.path.join(PROFILES, "system")      # Bambu's process presets, flattened (print_package --flatten)
P1S_PRESET = "0.20mm Standard @BBL X1C"
P1S_PRESETS = {"0.20": P1S_PRESET, "0.16": "0.16mm Optimal @BBL X1C", "0.12": "0.12mm Fine @BBL X1C",
               "0.08": "0.08mm Extra Fine @BBL X1C"}


# The overhang settings Bambu Studio 2.08 has beyond its defaults (wiki.bambulab.com, Bambu Studio's
# speed and cooling settings): walls over an overhang slower (50 and 30 mm/s at 25 to 50 and 50 to 75 %
# of a line overhanging, now 30 and 15) and thinner bridge lines (bridge flow 1 to 0.85: the steepest
# fins print as bridges). Detect overhang wall and smoothing the overhang walls' speed changes are on
# already, the fan at 100 % all the time for PLA. Bambu Studio has no Reverse on even, extra overhang
# perimeters or slowing for curled perimeters (OrcaSlicer's). (Two values: its two extruder variants)
P1S_OBJECT_SETTINGS = {"overhang_ladder_TUNED": {
    "overhang_2_4_speed": "30,30", "overhang_3_4_speed": "15,15", "bridge_flow": "0.85"}}


# Two-filament plates (the top's interface in Bambu Support For PLA): the flush at each change, tried on 1_case. The purge goes into
# the support (flush_into_support, on in the profile too: it made no difference to the weights), not into the objects or their
# infill: there the flush of the support filament printed it as walls and infill of the part, which support_check.py's rule
# (the support filament is interface and tower only) refuses. So what is left is the multiplier on the flush volumes (a project
# setting: 230 mm3 from PLA to the support filament and 700 back, the profile's, at 1): 0.7, in Alex's 0.6 to 0.8. Narrowing the
# prime tower (35 to 25 mm) changed nothing, so it stays TOWER_W: the tower's weight is the flush's
P1S_FLUSH = {"flush_into_support": "1", "flush_into_infill": "0", "flush_into_objects": "0"}
P1S_FLUSH_MULTIPLIER = "0.7"
# Elephant's foot compensation (the top's sharp screen edge and the tub's back edge sit on the bed): Alex's first-layer test
# (2026-10-22, the printer tests' 8_first_layer_*, PLAN.md) found 0.15 left a lip and 0.25 didn't, so it stays the preset's 0.25
# and isn't lowered to the 0.1 to 0.15 tried for this list. (The U1's template has 0.15 until a U1 print shows a lip.)
P1S_ELEPHANT_FOOT = "0.25"


def mark_presets(project, plate, extra=(), project_cfg=None):
    """Name the plate's process preset in a Bambu project and list the settings it changes (extra: more that it sets;
    project_cfg: project settings, not the preset's, that it gets as they are)."""
    with zipfile.ZipFile(project) as z:
        files = {n: z.read(n) for n in z.namelist()}
    cfg = json.loads(files["Metadata/project_settings.config"])
    cfg.update(project_cfg or {})
    own = print_package.PLATE_SETTINGS.get(plate, {})
    preset = P1S_PRESETS[own.get("layer_height", "0.20")]
    print_package.mark_presets(cfg, SYSTEM, preset, set(own) | set(extra), P1S_PRESET)
    files["Metadata/project_settings.config"] = json.dumps(cfg, indent=4).encode()
    with zipfile.ZipFile(project, "w", zipfile.ZIP_DEFLATED) as z:
        for k, v in files.items():
            z.writestr(k, v)


def fine_layers(project, plate, same):
    """Give the objects with a slope or round rising from the bed their fine layers (print_package.FINE_LAYERS), as Bambu
    Studio's own height range modifier; all of a plate's objects the same if it has a prime tower (same)."""
    with zipfile.ZipFile(project) as z:
        files = {n: z.read(n) for n in z.namelist()}
    import re
    names = re.findall(r'<object id="\d+">\s*<metadata key="name" value="([^"]+?)\.stl"',
                       files["Metadata/model_settings.config"].decode())
    ids = dict(enumerate(names, 1))          # (the ranges file counts its objects from 1, whatever their ids)
    xml = print_package.layer_ranges_xml(plate, ids, same)
    if xml:
        files["Metadata/layer_config_ranges.xml"] = xml.encode()
        with zipfile.ZipFile(project, "w", zipfile.ZIP_DEFLATED) as z:
            for k, v in files.items():
                z.writestr(k, v)


def applied(project, own):
    """Fail unless the unsliced project has each of the plate's own settings as set: a name Bambu Studio doesn't know is dropped
    without a word."""
    with zipfile.ZipFile(project) as z:
        cfg = json.loads(z.read("Metadata/project_settings.config"))
    bad = [k for k, v in own.items() if k not in cfg or not print_package.same(cfg[k], v)]
    if bad:
        sys.exit(f"{project}: Bambu Studio didn't take {', '.join(f'{k}={own[k]} (it has {cfg.get(k)})' for k in bad)}")


def layout(parts, plate=""):
    """Each part placed on the bed: print_package's shelf packing, longest first, from the back
    (Bambu's own --arrange leaves parts it can't nest on a second plate, and the slice still
    passes). Returns [(part, mesh moved into place)], or None if the parts don't fit."""
    import trimesh
    meshes = [(obj, print_package.load_part(plate, obj, path)) for obj, path in parts]
    sizes = [tuple(m.extents[:2]) for _, m in meshes]
    x0, y0, x1, y1 = EXCLUDE
    taken = [(x0, BED - y1, x1, BED - y0)] + [(a, BED - d, c, BED - b) for a, b, c, d in print_package.tower_keepout(parts, "P1S")]
    try:                                           # packed front to back, then turned back to front
        spots = print_package.pack(sizes, taken, BED)
    except SystemExit:
        return None
    for (obj, m), (x, y) in zip(meshes, spots):
        lo, hi = m.bounds
        m.apply_translation([x - (lo[0] + hi[0]) / 2, (BED - y) - (lo[1] + hi[1]) / 2, -lo[2]])
    return meshes


def own_settings(project, objs):
    """Give the named objects of an unsliced Bambu project their own settings ({object: {key: value}})."""
    import re
    with zipfile.ZipFile(project) as z:
        files = {n: z.read(n) for n in z.namelist()}
    cfg, n = re.subn(r'(  <object id="\d+">\n    <metadata key="name" value="(%s)\.stl"/>\n)'
                     % "|".join(map(re.escape, objs)),
                     lambda m: m.group(1) + "".join(f'    <metadata key="{k}" value="{v}"/>\n'
                                                    for k, v in objs[m.group(2)].items()),
                     files["Metadata/model_settings.config"].decode())
    if n != len(objs):
        sys.exit(f"{project}: found {n} of the {len(objs)} objects with their own settings")
    files["Metadata/model_settings.config"] = cfg.encode()
    with zipfile.ZipFile(project, "w", zipfile.ZIP_DEFLATED) as z:
        for k, v in files.items():
            z.writestr(k, v)


def slice_plate(name, parts, supported, tmp):
    """Place and slice one plate (supported: the STLs that get plain PLA supports; the top's go with the support
    filament); returns (3mf path, {"pla": g, "support": g, "purge": [PLA g, support g], "changes": n}, seconds,
    {object: mm of support under it}) or None if it doesn't fit."""
    placed = layout(parts, name)
    if placed is None:
        print(f"  {name}: doesn't fit the bed")
        return None
    work = os.path.join(tmp, name)
    os.makedirs(os.path.join(work, "out"))
    stls = []
    for obj, m in placed:                          # one file per object: two of a part are two files
        dst = os.path.join(work, obj + ".stl")
        m.export(dst)
        stls.append(dst)
    # the plate as a project (no slicing yet), its parts' support settings, then the slice
    project = os.path.join(work, name + "_unsliced.3mf")
    process = os.path.join(PROFILES, "process.json")
    plate_name = name.removesuffix("_a").removesuffix("_b")
    own = dict(print_package.PLATE_SETTINGS.get(plate_name, {}))
    two = print_package.has_top(parts)             # a top: the support filament is loaded, with a prime tower
    if two:
        own.update(TOWER_KEYS)
    if print_package.case_plate(plate_name):       # the case kit's print quality; the printer tests keep the preset's
        own.update(print_package.CASE_SETTINGS, elefant_foot_compensation=P1S_ELEPHANT_FOOT, **(P1S_FLUSH if two else {}))
    if own:                                        # the plate's own settings (a layer height, the tower) over the profile's
        cfg = json.load(open(process))
        process = os.path.join(work, "process.json")
        json.dump(dict(cfg, **own), open(process, "w"), indent=4)
    subprocess.run([os.environ["BAMBU_STUDIO"],
                    "--load-settings", f"{PROFILES}/machine.json;{process}",
                    "--load-filaments", f"{PROFILES}/filament.json" + (f";{SUPPORT_PROFILE}" if two else ""), "--arrange", "0",
                    "--export-3mf", os.path.basename(project), "--outputdir", work, *stls],
                   capture_output=True, text=True, timeout=600)
    if not os.path.exists(project):
        print(f"  {name}: Bambu Studio wrote no project")
        return None
    with_supports = [obj for obj, path in parts if path in supported]
    top_objs = [obj for obj, path in parts if path in print_package.TOP_PARTS]
    own_objs = {obj: dict(print_package.OBJECT_SUPPORT if obj in with_supports else print_package.TOP_SUPPORT if obj in top_objs else {},
                          **print_package.OBJECT_SETTINGS.get(obj, {}), **P1S_OBJECT_SETTINGS.get(obj, {}))
                for obj, _ in parts}
    own_objs = {obj: own for obj, own in own_objs.items() if own}
    if own_objs:
        own_settings(project, own_objs)
    if print_package.case_plate(plate_name):
        fine_layers(project, plate_name, two)
    applied(project, own)
    project_cfg = {"flush_multiplier": [P1S_FLUSH_MULTIPLIER]} if two and print_package.case_plate(plate_name) else {}
    mark_presets(project, plate_name, set(own), project_cfg)
    cmd = [os.environ["BAMBU_STUDIO"], "--slice", "0", "--export-3mf", name + ".3mf",
           "--outputdir", os.path.join(work, "out"), project]
    subprocess.run(cmd, capture_output=True, text=True, timeout=1800)
    try:
        result = json.load(open(os.path.join(work, "out", "result.json")))
    except (OSError, ValueError):
        result = {"error_string": "no result.json"}
    if result.get("return_code") != 0 or not result.get("sliced_plates"):
        print(f"  {name}: {result.get('error_string', '?').strip()}")
        return None
    plate = result["sliced_plates"][0]
    out = os.path.join(work, "out", name + ".3mf")
    mark_presets(out, plate_name, set(own), project_cfg)      # the slice writes the list empty again
    on_plate = json.loads(zipfile.ZipFile(out).read("Metadata/plate_1.json"))["bbox_objects"]
    on_plate = [o for o in on_plate if "wipe_tower" not in str(o.get("name", ""))]       # (the prime tower counts as an object)
    if len(on_plate) != len(parts):
        sys.exit(f"{name}: only {len(on_plate)} of its {len(parts)} parts are on the plate: {[o.get('name') for o in on_plate]}")
    fil = sorted(plate["filaments"], key=lambda f: f["id"])
    grams = {"pla": fil[0]["total_used_g"], "support": fil[1]["total_used_g"] if len(fil) > 1 else 0.0,
             "purge": [f["total_used_g"] - f["main_used_g"] for f in fil], "changes": plate.get("filament_change_times", 0)}
    report, bad, mm, filament = support_check.check(out, {obj for obj, path in parts if path not in supported and path not in print_package.TOP_PARTS}, set(top_objs))
    if bad or filament:
        sys.exit(f"{name}: " + ("; ".join(([f"support under {', '.join(bad)}, which need none"] if bad else []) + filament)) + ":\n" + "\n".join(report))
    return out, grams, plate["total_predication"], {o: mm.get(o, 0) for o in [*with_supports, *top_objs]}


def halves(parts):
    """Split a plate's parts into two of about equal footprint, biggest first."""
    import trimesh
    area = {p: (lambda b: (b[1][0] - b[0][0]) * (b[1][1] - b[0][1]))(trimesh.load(p, force="mesh").bounds)
            for _, p in parts}
    a, b = [], []
    for part in sorted(parts, key=lambda q: -area[q[1]]):
        (a if sum(area[p] for _, p in a) <= sum(area[p] for _, p in b) else b).append(part)
    return a, b


def main():
    if not os.environ.get("BAMBU_STUDIO"):
        sys.exit("set BAMBU_STUDIO to Bambu Studio's executable")
    only = sys.argv[1:]
    print("supports, measured part by part:")
    supported = print_package.support_parts()
    queue = [(n, p) for n, p in print_package.PLATES.items() if not only or n in only]
    case = any(n not in print_package.TEST_PLATES for n, _ in queue)  # the case kit or the tests, or both
    tests = any(n in print_package.TEST_PLATES for n, _ in queue)
    tests_dir = print_package.TESTS["P1S"]
    print_package.fresh(([OUT] if case else []) + ([tests_dir] if tests else []))
    rows = []
    with tempfile.TemporaryDirectory() as tmp, ThreadPoolExecutor(SLICERS) as pool:
        def plate(name, parts):
            """Slice a plate, or its halves if it doesn't fit: [(name, parts, sliced)], in order."""
            done = slice_plate(name, parts, supported, tmp)
            if done is None:
                if len(parts) < 2:
                    sys.exit(f"{name}: one part that doesn't slice on a P1S")
                a, b = halves(parts)
                print(f"  {name}: split into {name}_a and {name}_b")
                return plate(name + "_a", a) + plate(name + "_b", b)
            return [(name, parts, done)]
        # the plates slice at once (each Bambu Studio uses a few cores); listed in the plates' order
        for name, parts, done in [r for rs in pool.map(lambda q: plate(*q), queue) for r in rs]:
            path, grams, secs, sup = done
            where = print_package.plate_dir(name.removesuffix("_a").removesuffix("_b"), OUT, "P1S")
            shutil.copy(path, os.path.join(where, os.path.basename(path)))
            for _, p in parts:
                shutil.copy(p, os.path.join(where, "stl", os.path.basename(p)))
            if where == OUT:
                rows.append((name, parts, sup, grams, secs))
            print(f"{name + '.3mf':24s} PLA {grams['pla']:5.1f} g, support filament {grams['support']:4.1f} g, purge "
                  f"{grams['purge'][0]:.1f} + {grams['purge'][1] if len(grams['purge']) > 1 else 0:.1f} g, {grams['changes']} changes, "
                  f"{int(secs // 3600)} h {int(secs % 3600 // 60):02d} min  " + ", ".join(o for o, _ in parts)
                  + "".join(f"\n{'':24s} supports under {o}: {v:.0f} mm of extrusion" for o, v in sup.items()))
    if tests:
        shutil.copy(print_package.TESTS_README, os.path.join(tests_dir, "README.md"))
        print_package.tests_zip()
    if not case:
        return
    for extra in ("top.stl", "tub.stl"):
        if os.path.exists(os.path.join(HERE, extra)) and not only:
            shutil.copy(os.path.join(HERE, extra), os.path.join(OUT, "stl", extra))
    def supports(parts, sup):
        top = [o for o, p in parts if p in print_package.TOP_PARTS]
        plain = [o for o in sup if o not in top]
        return "; ".join(([f"{', '.join(top)}: tree, interface in slot 2" if len(top) > 1 else f"{top[0]}: tree, interface in slot 2"] if top else [])
                         + ([f"{', '.join(plain)}: plain PLA"] if plain else [])) or "none"

    def purge(g):
        return f"{g['purge'][0]:.1f} g PLA + {g['purge'][1]:.1f} g support" if g["support"] else "none"
    table = "\n".join(f"| `{n}.3mf` | {supports(parts, sup)} | {g['pla']:.1f} g | {g['support']:.1f} g | {purge(g)} | {g['changes']} | "
                      f"{int(t // 3600)} h {int(t % 3600 // 60):02d} min | {', '.join(o for o, _ in parts)} |"
                      for n, parts, sup, g, t in rows)
    text = open(README).read()
    a, b = text.index("<!-- printer -->"), text.index("<!-- /printer -->") + len("<!-- /printer -->")
    text = text[:a] + PRINTER.replace("<!-- PLATES -->", table) + text[b:]
    open(os.path.join(OUT, "README.md"), "w").write(text)
    with zipfile.ZipFile(ZIP, "w", zipfile.ZIP_DEFLATED) as o:
        for root, _, names in os.walk(OUT):
            for n in names:
                p = os.path.join(root, n)
                o.write(p, os.path.join("mega_blastoise_print_P1S", os.path.relpath(p, OUT)))
    print(f"{ZIP} ({os.path.getsize(ZIP) // 1024} kB)")


if __name__ == "__main__":
    main()
