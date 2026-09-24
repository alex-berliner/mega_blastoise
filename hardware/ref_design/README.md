# Two-player handheld: reference design (REF-A)

A complete, orderable design for the production version of the handheld:
schematic, 4-layer board, JLCPCB fab and assembly files, a printable case, and
the documents to build, bring up and review it. Built by Claude in parallel with
Alex's own hand-made board (`hardware/prod_board/`, `hardware/prod_case/`),
which this folder never touches.

![board](board/fab/battler_iso.png)

## What it is

- RP2350B, 16 MB flash, USB-C (charging + firmware + debug console)
- 3.5in 320x480 IPS panel (Displaytech DT035CTFT-IPS-SHB, ST7796, SPI) with a
  PWM-dimmed backlight driver
- 16 membrane contacts (bare gold combs) under DS Lite silicone: a D-pad and
  four face buttons at each end
- MAX98357A I2S amplifier into a 20 mm speaker
- One 103450 Li-ion cell, BQ24074 power-path charger, TPS63001 buck-boost 3V3,
  soft power button with MCU hold
- Case 75.4 x 156.2 x 20.0 mm, two printed parts, six M2 screws

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
| `case/*.stl`, `case/*.dxf`, `case/positions.json` | printable parts, lens and board outlines, positions for the board scripts |
| `docs/` | [assembly](docs/assembly.md), [bring-up](docs/bringup.md), [pin map](docs/pinmap.md), [design review](docs/design_review.md), [Konnect notes](docs/konnect_notes.md) |
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
FREEROUTING_JAR=.../freerouting-1.9.0.jar python3 tools/route.py
python3 tools/silk.py
python3 tools/check_pcb.py                   # DRC with schematic parity
python3 tools/fab.py                         # gerbers, BOM, CPL, STEP, renders
python3 ../case/check_fit.py                 # board inside the case?
python3 tools/pinmap.py                      # docs/pinmap.md
```

Needs KiCad 10 (kicad-cli and its `pcbnew` Python module), OpenSCAD, Java,
Freerouting 1.9.0, Konnect, and Raspberry Pi's RP2350B minimal KiCad design
(document RP-010329-CA) for the core layout.

## Status

Designed and checked, not yet built. What was checked, what wasn't, and what
must be measured before ordering: [docs/design_review.md](docs/design_review.md).
