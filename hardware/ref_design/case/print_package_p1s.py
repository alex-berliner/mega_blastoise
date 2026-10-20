#!/usr/bin/env python3
"""The 3D print package for a Bambu Lab P1S: the same plates as print_package.py, sliced.

    BAMBU_STUDIO=/path/to/BambuStudio python3 case/print_package_p1s.py [plate ...]

after case/export.py --stl and case/fitkit.py. BAMBU_STUDIO is Bambu Studio's executable (an
AppImage's AppRun, extracted with --appimage-extract, works headless). Each plate is laid out
(print_package's packing) and sliced by Bambu Studio's command line into a project with its G-code in it, ready to send.

PLA only, as the U1 package (Alex, 2026-10-20): Bambu PLA Basic, one filament, no prime tower.
The profiles are in print/p1s/ (Bambu's system presets for a P1S, 0.4 mm nozzle, 0.20 mm
Standard, flattened). A plate gets tree supports from the build plate only if one of its parts
needs them, measured by overhangs.py, the same decision as the U1 package; that's
process_supports.json. A plate that doesn't fit the P1S's 256 mm bed is split.

Writes print/package_P1S/ and print/mega_blastoise_print_P1S.zip (build output, not in git).
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import print_package  # noqa: E402

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


def slice_plate(name, parts, supports, tmp):
    """Place and slice one plate; returns (3mf path, grams, seconds) or None if it doesn't fit."""
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
    process = "process_supports.json" if supports else "process.json"
    cmd = [os.environ["BAMBU_STUDIO"],
           "--load-settings", f"{PROFILES}/machine.json;{PROFILES}/{process}",
           "--load-filaments", f"{PROFILES}/filament.json",
           "--arrange", "0", "--slice", "0", "--export-3mf", name + ".3mf",
           "--outputdir", os.path.join(work, "out"), *stls]
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
    on_plate = json.loads(zipfile.ZipFile(out).read("Metadata/plate_1.json"))["bbox_objects"]
    if len(on_plate) != len(parts):
        sys.exit(f"{name}: only {len(on_plate)} of its {len(parts)} parts are on the plate")
    grams = sum(f["main_used_g"] for f in plate["filaments"])
    return out, grams, plate["total_predication"]


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
    supported = print_package.support_plates()
    shutil.rmtree(OUT, ignore_errors=True)
    os.makedirs(os.path.join(OUT, "stl"))
    rows = []
    with tempfile.TemporaryDirectory() as tmp:
        queue = [(n, p, n in supported) for n, p in print_package.PLATES.items() if not only or n in only]
        while queue:
            name, parts, sup = queue.pop(0)
            done = slice_plate(name, parts, sup, tmp)
            if done is None:
                if len(parts) < 2:
                    sys.exit(f"{name}: one part that doesn't slice on a P1S")
                a, b = halves(parts)
                print(f"  {name}: split into {name}_a and {name}_b")
                queue[:0] = [(name + "_a", a, sup), (name + "_b", b, sup)]   # (a split half could drop
                # its supports if none of its parts needs them; kept simple: the plate's decision)
                continue
            path, grams, secs = done
            shutil.copy(path, os.path.join(OUT, os.path.basename(path)))
            for _, p in parts:
                shutil.copy(p, os.path.join(OUT, "stl", os.path.basename(p)))
            rows.append((name, sup, grams, secs, [o for o, _ in parts]))
            print(f"{name + '.3mf':24s} {'supports' if sup else 'no supports':12s} {grams:6.1f} g  "
                  f"{int(secs // 3600)} h {int(secs % 3600 // 60):02d} min  " + ", ".join(o for o, _ in parts))
    for extra in ("lid.stl", "tub.stl", "insert.stl"):
        if os.path.exists(os.path.join(HERE, extra)) and not only:
            shutil.copy(os.path.join(HERE, extra), os.path.join(OUT, "stl", extra))
    table = "\n".join(f"| `{n}.3mf` | {'tree supports from the plate' if s else 'none'} | {g:.1f} g | "
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
