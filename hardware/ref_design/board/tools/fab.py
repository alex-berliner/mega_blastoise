"""Manufacturing outputs for JLCPCB, all from the saved board and schematic.

    python3 tools/fab.py        # writes board/fab/

  gerbers/ + battler_gerbers.zip   copper x4, mask, paste, silk, outline, drills
  battler_bom_jlc.csv              Comment, Designator, Footprint, LCSC Part #
  battler_cpl_jlc.csv              Designator, Mid X, Mid Y, Layer, Rotation
  battler.step                     board with parts, for the case check
  battler_top.png, battler_bottom.png, battler_iso.png
  battler_schematic.pdf, battler_assembly.pdf

Parts without an LCSC number (membrane contacts, test pads, speaker pads,
holes, fiducials) are left out of the BOM and the CPL: nothing to place.
CPL rotations are KiCad's; check them in JLC's placement preview, which is
where the few footprints whose zero angle differs from LCSC's get corrected.
"""
import csv
import os
import subprocess
import sys
import zipfile
from collections import defaultdict

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD_DIR = os.path.dirname(HERE)
PCB = os.path.join(BOARD_DIR, "battler.kicad_pcb")
SCH = os.path.join(BOARD_DIR, "battler.kicad_sch")
OUT = os.path.join(BOARD_DIR, "fab")
LAYERS = "F.Cu,In1.Cu,In2.Cu,B.Cu,F.Mask,B.Mask,F.Paste,B.Paste,F.Silkscreen,B.Silkscreen,Edge.Cuts"


def run(*args):
    r = subprocess.run(["kicad-cli", *args], capture_output=True, text=True)
    if r.returncode:
        sys.exit(f"kicad-cli {' '.join(args[:3])} failed:\n{r.stderr[-1500:]}")
    return r.stdout


def gerbers():
    g = os.path.join(OUT, "gerbers")
    os.makedirs(g, exist_ok=True)
    for f in os.listdir(g):
        os.remove(os.path.join(g, f))
    run("pcb", "export", "gerbers", "--layers", LAYERS, "--subtract-soldermask", "--use-drill-file-origin",
        "-o", g + "/", PCB)
    run("pcb", "export", "drill", "--format", "excellon", "--drill-origin", "absolute", "--excellon-units", "mm",
        "--generate-map", "--map-format", "gerberx2", "-o", g + "/", PCB)
    z = os.path.join(OUT, "battler_gerbers.zip")
    with zipfile.ZipFile(z, "w", zipfile.ZIP_DEFLATED) as zf:
        for f in sorted(os.listdir(g)):
            zf.write(os.path.join(g, f), f)
    return sorted(os.listdir(g))


def bom_cpl():
    board = pcbnew.LoadBoard(PCB)
    groups = defaultdict(list)
    rows = []
    skipped = []
    for fp in sorted(board.GetFootprints(), key=lambda f: f.GetReference()):
        ref = fp.GetReference()
        lcsc = fp.GetFieldText("LCSC") if fp.HasField("LCSC") else ""
        if not lcsc:
            skipped.append(ref)
            continue
        fpname = str(fp.GetFPID().GetLibItemName())
        groups[lcsc].append((ref, fpname, fp.GetValue()))
        pos = fp.GetPosition()
        side = "Bottom" if fp.IsFlipped() else "Top"
        # JLC: Y up, origin anywhere; mirror Y so the CPL matches the gerbers' orientation
        rows.append([ref, f"{pcbnew.ToMM(pos.x):.3f}mm", f"{-pcbnew.ToMM(pos.y):.3f}mm", side,
                     f"{fp.GetOrientationDegrees() % 360:.1f}"])
    key = lambda r: (r[0].rstrip("0123456789"), int("0" + "".join(c for c in r[0] if c.isdigit())))
    with open(os.path.join(OUT, "battler_bom_jlc.csv"), "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["Comment", "Designator", "Footprint", "LCSC Part #"])
        # one line per part number: the core's passives use Raspberry Pi's land patterns, the
        # rest KiCad's, and JLC matches on the LCSC number
        for lcsc, items in sorted(groups.items(), key=lambda kv: key([kv[1][0][0]])):
            refs = sorted((r for r, _, _ in items), key=lambda r: key([r]))
            fps = sorted({f for _, f, _ in items})
            values = sorted({v for _, _, v in items})
            w.writerow([" / ".join(values), ",".join(refs), " / ".join(fps), lcsc])
    with open(os.path.join(OUT, "battler_cpl_jlc.csv"), "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["Designator", "Mid X", "Mid Y", "Layer", "Rotation"])
        for r in sorted(rows, key=key):
            w.writerow(r)
    return len(groups), len(rows), skipped


def pictures():
    run("pcb", "export", "step", "--subst-models", "--force", "-o", os.path.join(OUT, "battler.step"), PCB)
    for side, extra in (("top", []), ("bottom", []), ("iso", ["--rotate", "-45,0,45"])):
        view = "top" if side == "iso" else side
        run("pcb", "render", "--side", view, "--quality", "high", "--width", "1600", "--height", "1600",
            "--background", "opaque", *extra, "-o", os.path.join(OUT, f"battler_{side}.png"), PCB)
    run("sch", "export", "pdf", "-o", os.path.join(OUT, "battler_schematic.pdf"), SCH)
    run("pcb", "export", "pdf", "--layers", "F.Fab,Edge.Cuts", "--mode-single", "--include-border-title", "--sp",
        "-o", os.path.join(OUT, "battler_assembly.pdf"), PCB)


def main():
    os.makedirs(OUT, exist_ok=True)
    files = gerbers()
    print("gerbers:", len(files), "files")
    n_lines, n_parts, skipped = bom_cpl()
    print(f"BOM: {n_lines} lines, CPL: {n_parts} placements; not assembled: {', '.join(skipped)}")
    if "--no-pictures" not in sys.argv:
        pictures()
        print("STEP, renders, schematic and assembly PDFs written")


if __name__ == "__main__":
    main()
