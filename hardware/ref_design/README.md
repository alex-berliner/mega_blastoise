# Two-player handheld: reference design (REF-A)

A complete, orderable design for the production version of the handheld:
schematic, 2-layer board, JLCPCB fab and assembly files, a printable case, and
the documents to build, bring up and review it.

**This is the 2-layer build**, on `main` since 2026-09-24: the first, 4-layer
design (kept on branch `ref-design-4layer`) with every cost change Alex approved
that day, on two copper layers with JLC's standard 0.3 mm holes. Built by Claude in parallel with
Alex's own hand-made board (`hardware/prod_board/`, `hardware/prod_case/`),
which this folder never touches.

![board](board/fab/battler_iso.png)

## What it is

- RP2354B (2 MB flash inside), USB-C (charging + firmware + debug console)
- 3.5in 320x480 IPS panel (EastRising ER-TFT035IPS-6, ILI9488, SPI) with a published
  datasheet and 3D model, its FPC in a 50-pin ZIF connector (no soldering); a
  constant-current backlight driver
- 16 membrane contacts (bare gold combs) under DS Lite silicone: a D-pad and
  four face buttons at each end
- PWM audio through an RC filter into an FM8002A amplifier and a 20 mm speaker
- One 503450 Li-ion cell (1000 mAh), TP4056 charger with USB load sharing,
  ME6211 3V3 LDO, soft power on seat 1's Y button with MCU hold
- Case 75.4 x 156.2 x 15.0 mm, two printed parts that snap together

## Folder

| Path | What |
|---|---|
| `board/design.py` | **the design as data**: every part, value, LCSC number, footprint and net. Edit this. |
| `board/placement.py` | where parts go on the board (the case supplies the rest) |
| `board/battler.kicad_pro` / `.kicad_sch` / `*.kicad_sch` / `.kicad_pcb` | the KiCad 10 project (generated) |
| `board/lib/` | project footprints and 3D models |
| `board/fab/` | gerbers zip, JLC BOM and CPL, STEP, renders, schematic and assembly PDFs |
| `board/tools/` | the scripts that build and check all of it (below) |
| `case/case.scad` | the case, and the source of every mechanical position |
| `case/*.stl`, `case/board.dxf`, `case/positions.json` | printable parts, board outline, positions for the board scripts |
| `docs/` | [BOM](docs/bom.md) (factory and self-assembled), [assembly](docs/assembly.md), [bring-up](docs/bringup.md), [pin map](docs/pinmap.md), [design review](docs/design_review.md), [Konnect notes](docs/konnect_notes.md) |
| `tools/quotes/` | JLCPCB and PCBWay instant-quote scripts for the bare board and assembly |
| `datasheets/SOURCES.md` | where each datasheet comes from |

## Rebuilding

Everything in `board/` except `design.py`, `placement.py`, `lib/` and `tools/`
is generated. After changing the design:

```sh
cd hardware/ref_design
python3 case/export.py --stl                 # case -> positions.json, DXFs, STLs
cd board
python3 tools/build_sch.py                   # design.py -> schematic (through Konnect)
python3 tools/check_netlist.py               # KiCad's netlist == design.py ?
kicad-cli sch erc battler.kicad_sch          # 0 errors expected
RP2350B_REF=.../RPI-RP2350B-MINIMAL_R4-S1.kicad_pcb python3 tools/build_pcb.py
FREEROUTING_JAR=.../freerouting-1.9.0.jar python3 tools/route.py   # about 10 min
python3 tools/route.py --finish              # only if route.py left a connection open
python3 tools/silk.py
python3 tools/check_pcb.py                   # DRC with schematic parity
python3 tools/fab.py                         # gerbers, BOM, CPL, STEP, renders
python3 ../case/check_fit.py                 # board inside the case?
python3 tools/pinmap.py                      # docs/pinmap.md
```

Needs KiCad 10 (kicad-cli and its `pcbnew` Python module), OpenSCAD, Java,
Freerouting 1.9.0, Konnect, and Raspberry Pi's RP2350B minimal KiCad design
(document RP-010329-CA) for the core layout. The project library in `lib/` is
already built; `tools/lib_from_ref.py` (the core's land patterns, from that
design) and `tools/sym_lib.py` (symbols KiCad lacks) rebuild it.

## Status

Designed and checked, not yet built. What was checked, what wasn't, and what
must be measured before ordering: [docs/design_review.md](docs/design_review.md).
