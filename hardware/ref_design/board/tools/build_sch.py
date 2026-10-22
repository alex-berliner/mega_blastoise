"""Write the reference schematic from design.py.

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
text on text and parts off the page. tools/schwrite.py then writes each sheet
whole, in KiCad 10's own format.

The result is checked again by check_netlist.py (KiCad's netlist against
design.py) and by KiCad's ERC. Rewrites the .kicad_sch files; nothing else in
the folder. (Until 2026-10-20 the sheets were written through Konnect:
docs/konnect_notes.md has what that ran into.)
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD = os.path.dirname(HERE)
sys.path.insert(0, HERE)
sys.path.insert(0, BOARD)

import design  # noqa: E402
import schwrite  # noqa: E402
from sch_sheets import LAYOUTS  # noqa: E402

ROOT_FILE = schwrite.PROJECT + ".kicad_sch"
COMPANY = "Two-player handheld: reference design (REF-A)"
TITLES = {  # title block of each sheet
    ROOT_FILE: "Sheets and mechanical parts",
    "usb_charger.kicad_sch": "USB-C, charger, load sharing, cell connector",
    "power.kicad_sch": "3V3 regulator, power latch, cell voltage sense",
    "mcu.kicad_sch": "RP2354B",
    "mcu_power.kicad_sch": "RP2354B supply: decoupling and core regulator",
    "debug.kicad_sch": "SWD, UART, debug pads, BOOTSEL and RESET",
    "display.kicad_sch": "Panel connector and backlight sinks",
    "buttons.kicad_sch": "Membrane contacts",
}


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
    writer = schwrite.Writer(BOARD, TITLES, COMPANY)
    for f, sh in sheets.items():
        writer.emit(sh, str(design.BOARD_REV))
    print("power symbols:", writer.pwr)


if __name__ == "__main__":
    main()
