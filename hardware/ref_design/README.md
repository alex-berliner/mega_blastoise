# Two-player handheld: reference design (REF-A)

A complete, orderable design for the production version of the handheld:
schematic, 2-layer board, JLCPCB fab and assembly files, a printable case, and
the documents to build, bring up and review it.

**The cell lies under the board** (on the tub's floor) since 2026-10-09; the layout with
it on the board under the screen (13.9 mm thick, 7.9 at the ends) is on branch
`ref-design-cell-on-board`. This board was first routed with `route.py 80`; 20 passes do as
well (2026-10-21: Freerouting leaves the same connections open after 15 as after 80).

**This is the 2-layer build**, on `main` since 2026-09-24: the first, 4-layer
design (kept at tag `archive/ref-design-4layer`) with every cost change Alex approved
that day, on two copper layers with JLC's standard 0.3 mm holes. Built by Claude in parallel with
Alex's own hand-made board (`hardware/prod_board/`, `hardware/prod_case/`),
which this folder never touches.

![board](board/fab/battler_iso.png)

## What it is

- RP2354B (2 MB flash inside), USB-C (charging + firmware + debug console)
- 2.4in 240x320 IPS panel (EastRising ER-TFT024IPS-3, ST7789V, SPI; the 3.5in ER-TFT035IPS-6
  until 2026-10-21) with a published
  datasheet, its FPC in a 50-pin ZIF connector (no soldering); a
  constant-current backlight sink per LED group
- 20 membrane contacts (bare gold combs) under DS Lite silicone: a D-pad, four face buttons,
  Start and Select at each end
- PWM audio through an RC filter into an FM8002A amplifier and a 15 x 10 mm speaker that plugs in,
  lies on the board's top beside the screen and plays up through a grille in the face
- One 403450 Li-ion cell (about 700 mAh) in a hole in the board, its bottom in a pocket in the tub's floor, BQ24073 power-path charger,
  RT9013 3V3 LDO, soft power on seat 1's Start button with MCU hold
- Case 77.4 x 132.7 mm (a DS Lite's length), 9.55 mm at the screen and 7.7 mm at the button decks
  (genuine DS Lite caps stand 1.41 mm proud there): a tub, a top piece and a deck at each end that
  curve over the sides to one seam, built upside down, all printed without supports, held by six M2
  screws as a DS Lite ([case/SHELL.md](case/SHELL.md)); the board, 72.4 x 127.7 mm, is placed to
  its constraints ([case/CELLCUT_FOR_CASE.md](case/CELLCUT_FOR_CASE.md))

## Folder

| Path | What |
|---|---|
| `board/design.py` | **the design as data**: every part, value, LCSC number, footprint and net. Edit this. |
| `board/placement.py` | where parts go on the board (the case supplies the rest) |
| `board/battler.kicad_pro` / `.kicad_sch` / `*.kicad_sch` / `.kicad_pcb` | the KiCad 10 project (generated) |
| `board/lib/` | project footprints and 3D models |
| `board/fab/` | gerbers zip, JLC BOM and CPL, STEP, renders, schematic and assembly PDFs: build output from `tools/fab.py`, not in git, except the renders the docs show |
| `board/tools/` | the scripts that build and check all of it (below) |
| `board/sim/` | ngspice simulation of the backlight current sinks (loop stability, accuracy, transients); `backlight_results.txt` is the last run |
| `case/case.scad` | the case, and the source of every mechanical position |
| `case/*.stl`, `case/board.dxf`, `case/positions.json` | printable parts (tub, top, deck), board outline, positions for the board scripts: build output from `case/export.py`, not in git (run it first) |
| `case/SHELL.md`, `case/BOARD_OUTLINE.md` | the shell's design (2026-10-21) and what it asks of the board |
| `case/fitkit/` | a printable fit kit: board mock, part dummies and case coupons, to test the real parts before ordering ([its README](case/fitkit/README.md)) |
| `docs/` | [BOM](docs/bom.md) (factory and self-assembled), [assembly](docs/assembly.md), [bring-up](docs/bringup.md), [pin map](docs/pinmap.md), [design review](docs/design_review.md), [Konnect notes](docs/konnect_notes.md) |
| `tools/quotes/` | JLCPCB and PCBWay instant-quote scripts for the bare board and assembly |
| `datasheets/SOURCES.md` | where each datasheet comes from |

## Rebuilding

Everything in `board/` except `design.py`, `placement.py`, `lib/` and `tools/`
is generated. After changing the design, run the pipeline:

```sh
cd hardware/ref_design
python3 tools/build.py                       # every stage whose inputs changed since its last good run
python3 tools/build.py --list                # the stages and which are out of date
python3 tools/build.py --dry-run             # what a run would do
python3 tools/build.py --only check_fit,fitkit   # just these, always
python3 tools/build.py --from route          # route again, then whatever that changes
python3 tools/build.py --keep-going          # a failing check doesn't stop the run
python3 tools/build.py --touch               # a fresh checkout: take the committed files as built
```

It skips a stage whose command, tools and input contents match its last good run
(`.build/state.json`, not committed; logs in `.build/logs/`), so a case change that
leaves `positions.json` as it was doesn't reroute the board. The end-of-round stages
(fit kit, print packages, viewer) run only when their inputs changed; the backlight
simulation, `route.py --finish` and `sync_pcb.py` only when named with `--only`.
`tools/build.py`'s docstring has the details.

Speed: `tools/scad.py` picks OpenSCAD. With an OpenSCAD development snapshot AppImage
in `~/Applications` (openscad.org, Development Snapshots) it renders with the Manifold
backend, minutes down to seconds; otherwise the system `openscad` (CGAL). `OPENSCAD=...`
picks a binary, `OPENSCAD_BACKEND=CGAL` the old backend. Renders and kicad-cli exports
run several at once (`REF_JOBS` sets how many). `route.py` routes four copies of the
board at once, each handed to Freerouting in another order (`--seed=`: 1.9 routes one file
the same way every time, so identical copies all end the same way), starts the next seed
when one ends with something open, and keeps the first with a clean DRC (`--attempts=N` or
`ROUTE_ATTEMPTS`, 1 routes in place as before; `--tries=M` caps the seeds, default 2N).
Freerouting gets 20 passes (it gains nothing after about 15), no 20 s start-up dialog and
no optimizer pass (`--optimize` puts it back); about 4.5 min for a clean board where three
80-pass attempts took over 10.

The stages, by hand:

```sh
cd hardware/ref_design
python3 case/export.py --stl                 # case -> positions.json, DXF, tub/top/deck STLs
python3 case/check_buttons.py                # D-pad, A/B/X/Y, Start/Select with their silicone under the shell
cd board
python3 tools/build_sch.py                   # design.py -> schematic (tools/schwrite.py writes the sheets)
python3 tools/check_netlist.py               # KiCad's netlist == design.py ?
python3 tools/check_pins.py                  # every part's pins against its datasheet review (pin_review/)
python3 sim/models.py && python3 sim/backlight.py   # backlight sink stability (ngspice, about 40 min; only when it changes)
kicad-cli sch erc battler.kicad_sch          # 0 errors expected
RP2350B_REF=.../RPI-RP2350B-MINIMAL_R4-S1.kicad_pcb python3 tools/build_pcb.py
FREEROUTING_JAR=.../freerouting-1.9.0.jar python3 tools/route.py   # about 4.5 min, 4 seeded attempts at once
python3 tools/route.py --finish              # only if route.py left a connection open
python3 tools/silk.py
python3 tools/sync_pcb.py                    # only for a redrawn schematic on a routed board: relink, rename nets
python3 tools/check_pcb.py                   # DRC with schematic parity
python3 tools/fab.py                         # gerbers, BOM, CPL, STEP, renders
python3 ../case/check_fit.py                 # board inside the case?
python3 ../case/fitkit.py                    # the printable fit kit (under a minute with Manifold)
python3 ../case/check_manifold.py            # every printed part's mesh manifold (each edge on two faces)?
python3 tools/pinmap.py                      # docs/pinmap.md
python3 tools/strip_load.py                  # nets through each strip beside the cell's hole, ratsnest length (no routing)
python3 ../viewer/build.py                   # the exploded 3D view (GitHub Pages, /assembly/)
```

The exploded view is published at https://alex-berliner.github.io/mega_blastoise/assembly/
whenever main is pushed. Its parts are built here (GitHub's runners lack KiCad's 3D
library and OpenSCAD) and committed; `viewer/pre-push`, installed as the repo's pre-push
hook, rebuilds and commits them when main would go out with stale ones.

Needs KiCad 10 (kicad-cli and its `pcbnew` Python module), OpenSCAD, Java,
Freerouting 1.9.0, and Raspberry Pi's RP2350B minimal KiCad design
(document RP-010329-CA) for the core layout. The project library in `lib/` is
already built; `tools/lib_from_ref.py` (the core's land patterns, from that
design) and `tools/sym_lib.py` (symbols KiCad lacks) rebuild it.

## Status

Designed and checked, not yet built. What was checked, what wasn't, and what
must be measured before ordering: [docs/design_review.md](docs/design_review.md).
