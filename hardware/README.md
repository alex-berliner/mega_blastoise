# Hardware

The boards and cases for the two-player handheld, newest first.

| Folder | What it is | Whose | Status |
|---|---|---|---|
| [`ref_design/`](./ref_design/README.md) | The reference design (REF-A): RP2354B, 3.5in IPS panel, DS Lite buttons (D-pad, A/B/X/Y, Start, Select per seat), one Li-ion cell; the schematic, routed 2-layer board, JLCPCB fab files, printable case, fit kit and docs, all generated from `board/design.py`, `board/placement.py` and `case/case.scad` | Claude, built alongside Alex's own board | Designed and checked, not yet ordered; parts marked ESTIMATE need a real sample first ([PLAN.md](./ref_design/PLAN.md)) |
| [`prod_board/`](./prod_board/), [`prod_case/`](./prod_case/README.md) | Alex's own production board and case, made by hand in KiCad and OpenSCAD to learn the tools | Alex | In progress |
| [`membrane/`](./membrane/README.md) | DS Lite membrane contact footprints and the generator for them, shared by both boards | shared | In use |
| [`stripboard_v2/`](./stripboard_v2/BUILD.md) | The working prototype: Pico 2, ST7796 panel, 16 direct buttons on stripboard; the firmware's `board_v2` target | Alex and Claude | Built; the firmware's current target |

The v1 device (Pico, two OLEDs, button matrix: its KiCad board, stripboard and case) is out of
the tree; tag `v1-archive` has it, and [docs/archive/v1/](../docs/archive/v1/) its notes.

Each folder's own README says how to rebuild it. Build output (gerbers, STLs, STEP, renders)
is not committed where a script makes it: run the folder's pipeline.
