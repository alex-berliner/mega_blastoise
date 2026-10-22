#!/usr/bin/env python3
"""The 3D print package for a Bambu Lab P1S: the same plates as print_package.py, sliced.

    BAMBU_STUDIO=/path/to/BambuStudio python3 case/print_package_p1s.py [plate ...]

after case/export.py --stl and case/fitkit.py. BAMBU_STUDIO is Bambu Studio's executable (an
AppImage's AppRun, extracted with --appimage-extract, works headless). Each plate is laid out
(print_package's packing) and sliced by Bambu Studio's command line into a project with its G-code in it, ready to send.

PLA only, as the U1 package (Alex, 2026-10-20): Bambu PLA Basic, one filament, no prime tower.
The profiles are in print/p1s/ (Bambu's system presets for a P1S, 0.4 mm nozzle, 0.20 mm
Standard, flattened), supports off. Supports are per object, as in the U1 package: the parts
overhangs.py says need them get print_package.OBJECT_SUPPORT (tree supports from the build plate)
in the project, so each plate is sliced in two steps: Bambu Studio writes the plate as a project,
the parts' own settings go into it, and Bambu Studio slices that. Then support_check.py reads the
G-code and the build fails if a part that needs no supports got some (a plate's supports filled
the tub's screw counterbores, 2026-10-21). A plate that doesn't fit the P1S's 256 mm bed is split.
Bambu Studio's CLI lists no changed settings in its projects, so each project then names its
process preset (P1S_PRESETS, the one at its layer height; Bambu's X1C presets are the P1S's) and
lists what it changes from it (print_package.mark_presets), or Bambu Studio would load the preset's.

Writes print/package_P1S/ and print/mega_blastoise_print_P1S.zip (build output, not in git).
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
Standard, Bambu PLA Basic), arranged and already sliced: open it and send it, or re-slice it after
changing anything. PLA only, one filament, no prime tower. (`mega_blastoise_print_U1.zip` has the
same plates for the Snapmaker U1.) Set the plate type in Bambu Studio to the plate you use.

| Project | Supports | PLA | Time | Parts |
|---|---|---|---|---|
<!-- PLATES -->
"""


BED = 256.0
EXCLUDE = (0, 0, 18, 28)       # the P1S's no-print corner, front left (machine.json bed_exclude_area)


SYSTEM = os.path.join(PROFILES, "system")      # Bambu's process presets, flattened (print_package --flatten)
P1S_PRESET = "0.20mm Standard @BBL X1C"
P1S_PRESETS = {"0.20": P1S_PRESET, "0.16": "0.16mm Optimal @BBL X1C", "0.12": "0.12mm Fine @BBL X1C",
               "0.08": "0.08mm Extra Fine @BBL X1C"}


def mark_presets(project, plate):
    """Name the plate's process preset in a Bambu project and list the settings it changes."""
    with zipfile.ZipFile(project) as z:
        files = {n: z.read(n) for n in z.namelist()}
    cfg = json.loads(files["Metadata/project_settings.config"])
    own = print_package.PLATE_SETTINGS.get(plate, {})
    preset = P1S_PRESETS[own.get("layer_height", "0.20")]
    print_package.mark_presets(cfg, SYSTEM, preset, own, P1S_PRESET)
    files["Metadata/project_settings.config"] = json.dumps(cfg, indent=4).encode()
    with zipfile.ZipFile(project, "w", zipfile.ZIP_DEFLATED) as z:
        for k, v in files.items():
            z.writestr(k, v)


def layout(parts):
    """Each part placed on the bed: print_package's shelf packing, longest first, from the back
    (Bambu's own --arrange leaves parts it can't nest on a second plate, and the slice still
    passes). Returns [(part, mesh moved into place)], or None if the parts don't fit."""
    import trimesh
    meshes = [(obj, trimesh.load(path, force="mesh")) for obj, path in parts]
    sizes = [tuple(m.extents[:2]) for _, m in meshes]
    x0, y0, x1, y1 = EXCLUDE
    try:                                           # packed front to back, then turned back to front
        spots = print_package.pack(sizes, [(x0, BED - y1, x1, BED - y0)], BED)
    except SystemExit:
        return None
    for (obj, m), (x, y) in zip(meshes, spots):
        lo, hi = m.bounds
        m.apply_translation([x - (lo[0] + hi[0]) / 2, (BED - y) - (lo[1] + hi[1]) / 2, -lo[2]])
    return meshes


def own_supports(project, objs):
    """Give the named objects of an unsliced Bambu project their own support settings."""
    import re
    with zipfile.ZipFile(project) as z:
        files = {n: z.read(n) for n in z.namelist()}
    keys = "".join(f'    <metadata key="{k}" value="{v}"/>\n' for k, v in print_package.OBJECT_SUPPORT.items())
    cfg, n = re.subn(r'(  <object id="\d+">\n    <metadata key="name" value="(%s)\.stl"/>\n)'
                     % "|".join(map(re.escape, objs)), lambda m: m.group(1) + keys,
                     files["Metadata/model_settings.config"].decode())
    if n != len(objs):
        sys.exit(f"{project}: found {n} of the {len(objs)} objects that need supports")
    files["Metadata/model_settings.config"] = cfg.encode()
    with zipfile.ZipFile(project, "w", zipfile.ZIP_DEFLATED) as z:
        for k, v in files.items():
            z.writestr(k, v)


def slice_plate(name, parts, supported, tmp):
    """Place and slice one plate (supported: the STLs that get supports); returns (3mf path,
    grams, seconds, {object: mm of support under it}) or None if it doesn't fit."""
    placed = layout(parts)
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
    own = print_package.PLATE_SETTINGS.get(plate_name)
    if own:                                        # the plate's own settings (a layer height) over the profile's
        cfg = json.load(open(process))
        if any(k not in cfg for k in own):
            sys.exit(f"{name}: {', '.join(k for k in own if k not in cfg)} isn't a setting in the P1S process")
        process = os.path.join(work, "process.json")
        json.dump(dict(cfg, **own), open(process, "w"), indent=4)
    subprocess.run([os.environ["BAMBU_STUDIO"],
                    "--load-settings", f"{PROFILES}/machine.json;{process}",
                    "--load-filaments", f"{PROFILES}/filament.json", "--arrange", "0",
                    "--export-3mf", os.path.basename(project), "--outputdir", work, *stls],
                   capture_output=True, text=True, timeout=600)
    if not os.path.exists(project):
        print(f"  {name}: Bambu Studio wrote no project")
        return None
    with_supports = [obj for obj, path in parts if path in supported]
    if with_supports:
        own_supports(project, with_supports)
    mark_presets(project, plate_name)
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
    mark_presets(out, plate_name)                  # the slice writes the list empty again
    on_plate = json.loads(zipfile.ZipFile(out).read("Metadata/plate_1.json"))["bbox_objects"]
    if len(on_plate) != len(parts):
        sys.exit(f"{name}: only {len(on_plate)} of its {len(parts)} parts are on the plate")
    grams = sum(f["main_used_g"] for f in plate["filaments"])
    report, bad, mm = support_check.check(out, {obj for obj, path in parts if path not in supported})
    if bad:
        sys.exit(f"{name}: support under {', '.join(bad)}, which need none:\n" + "\n".join(report))
    return out, grams, plate["total_predication"], {o: mm.get(o, 0) for o in with_supports}


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
    shutil.rmtree(OUT, ignore_errors=True)
    os.makedirs(os.path.join(OUT, "stl"))
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
        queue = [(n, p) for n, p in print_package.PLATES.items() if not only or n in only]
        # the plates slice at once (each Bambu Studio uses a few cores); listed in the plates' order
        for name, parts, done in [r for rs in pool.map(lambda q: plate(*q), queue) for r in rs]:
            path, grams, secs, sup = done
            shutil.copy(path, os.path.join(OUT, os.path.basename(path)))
            for _, p in parts:
                shutil.copy(p, os.path.join(OUT, "stl", os.path.basename(p)))
            rows.append((name, sup, grams, secs, [o for o, _ in parts]))
            print(f"{name + '.3mf':24s} {grams:6.1f} g  {int(secs // 3600)} h {int(secs % 3600 // 60):02d} min  "
                  + ", ".join(o for o, _ in parts) + "".join(f"\n{'':24s} supports under {o}: {v:.0f} mm of extrusion"
                                                            for o, v in sup.items()))
    for extra in ("top.stl", "deck.stl", "tub.stl"):
        if os.path.exists(os.path.join(HERE, extra)) and not only:
            shutil.copy(os.path.join(HERE, extra), os.path.join(OUT, "stl", extra))
    table = "\n".join(f"| `{n}.3mf` | {'under ' + ', '.join(s) if s else 'none'} | {g:.1f} g | "
                      f"{int(t // 3600)} h {int(t % 3600 // 60):02d} min | {', '.join(objs)} |"
                      for n, s, g, t, objs in rows)
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
