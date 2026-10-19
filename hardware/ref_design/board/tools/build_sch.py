"""Write the reference schematic from design.py, through Konnect.

    python3 tools/build_sch.py            # from hardware/ref_design/board/
    python3 tools/build_sch.py --check    # lay out and check the sheets, write nothing

Every sheet is drawn by hand as functional blocks (tools/sch_sheets.py): parts
wired to each other, decoupling next to its pins, series parts in a chain, net
labels only where a net leaves its block and global labels only for nets that
cross sheets. design.py stays the source of every part, value and net; the
layout only says where things go and which pins a wire joins.

tools/schdraw.py holds the drawing kit and its checks: before anything is
written, each sheet's wiring is traced and compared with design.py (every
group of joined pins one net, every net named as design.py names it, nothing
left dangling), and its geometry is checked for wires through parts or text,
text on text and parts off the page. Konnect's own quirks, and how the kit
works around them, are described there and in docs/konnect_notes.md.

The result is checked again by check_netlist.py (KiCad's netlist against
design.py) and by KiCad's ERC. Deletes and regenerates the .kicad_sch files;
nothing else in the folder.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD = os.path.dirname(HERE)
sys.path.insert(0, HERE)
sys.path.insert(0, BOARD)

import design  # noqa: E402
import schdraw  # noqa: E402
from konnect_client import Konnect  # noqa: E402
from sch_sheets import LAYOUTS, SHEET_BOX  # noqa: E402

PROJECT = "battler"
ROOT_FILE = PROJECT + ".kicad_sch"


def drawn():
    """Lay out and check every sheet. Returns ({file: Sheet}, error count)."""
    out, bad = {}, 0
    for f in [ROOT_FILE] + [s for s, _ in design.SHEETS]:
        sh = LAYOUTS[f]()
        sh.finish()
        errors, warns = sh.check()
        print(f"{f}: {len(sh.sym)} parts, {len(sh.wires)} wires, {len(sh.labels)} labels, "
              f"{len(sh.power)} power symbols; {len(errors)} errors, {len(warns)} warnings")
        for e in errors:
            print("  ERROR:", e)
        for w in warns:
            print("  warn:", w)
        bad += len(errors)
        out[f] = sh
    return out, bad


def main():
    sheets, bad = drawn()
    if "--check" in sys.argv:
        return
    if bad and "--force" not in sys.argv:
        raise SystemExit(f"{bad} drawing errors: not written")
    # Konnect looks for libraries in KICAD10_SYMBOL_DIR before KiCad's own folder; the
    # project's symbols (tools/sym_lib.py) live in lib/, not in the project's library table
    os.environ["KICAD10_SYMBOL_DIR"] = os.path.join(BOARD, "lib")
    k = Konnect()
    for ts in ("project", "sch_components", "sch_wiring", "sch_hierarchy", "library", "sch_batch"):
        k.call("load_toolset", name=ts)
    for f in sheets:
        path = os.path.join(BOARD, f)
        if os.path.exists(path):
            os.remove(path)
    if not os.path.exists(os.path.join(BOARD, PROJECT + ".kicad_pro")):
        k.call("create_project", path=BOARD, name=PROJECT)
    else:
        k.call("create_schematic", path=os.path.join(BOARD, ROOT_FILE))
    k.call("register_footprint_library", library_path=os.path.join(BOARD, "lib", "battler.pretty"),
           nickname="battler", project=os.path.join(BOARD, PROJECT + ".kicad_pro"), scope="project")
    root = os.path.join(BOARD, ROOT_FILE)
    for i, (f, name) in enumerate(design.SHEETS):
        x, y, w, h = SHEET_BOX(i)
        k.call("add_hierarchical_sheet", schematic=root, sheet_file=f, sheet_name=name,
               x=x, y=y, width=w, height=h, project_name=PROJECT)
    writer = schdraw.Writer(k, BOARD)
    for f, sh in sheets.items():
        writer.emit(sh)
    k.close()
    print("power symbols:", writer.pwr)


if __name__ == "__main__":
    main()
