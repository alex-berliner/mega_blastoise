# Two-player handheld: reference design (REF-A)

A complete, orderable design for the production version of the handheld:
schematic, 2-layer board, JLCPCB fab and assembly files, a printable case, and
the documents to build, bring up and review it.

This board was first routed with `route.py 80`; 20 passes do as
well (2026-10-21: Freerouting leaves the same connections open after 15 as after 80).

**This is the 2-layer build**, on `main` since 2026-09-24: the first, 4-layer
design (kept at tag `archive/ref-design-4layer`) with every cost change Alex approved
that day, on two copper layers with JLC's standard 0.3 mm holes. Built by Claude.

![board](board/fab/battler_iso.png)

## What it is

- RP2354B (2 MB flash inside), USB-C (charging + firmware + debug console)
- 3.5in 320x480 IPS panel (EastRising ER-TFT035IPS-6, ILI9488, SPI; the 2.4in ER-TFT024IPS-3 from 2026-10-21 to 2026-10-26), its FPC in a 50-pin ZIF connector (no soldering; its datasheet was not re-read when the panel came back); a
  constant-current backlight sink per LED cathode pin
- 20 membrane contacts (bare gold combs) under DS Lite silicone: a D-pad, four face buttons,
  Start and Select at each end
- PWM audio through an RC filter into an FM8002A amplifier and a 15 x 10 mm speaker that plugs in,
  lies in a hole in the board beside the screen and plays up through a grille in the face
- One 403450 Li-ion cell (about 700 mAh) in a hole in the board, its bottom in a pocket in the tub's floor, BQ24073 power-path charger,
  RT9013 3V3 LDO, soft power on seat 1's Start button with MCU hold
- Case 77.4 x 156.64 mm, 9.6 mm at the screen and 7.7 mm at the button ends
  (genuine DS Lite caps stand 1.41 mm proud there): a tub and one top piece, the whole face and sides, whose screen
  band runs from wall to wall and ends in the DS Lite's own slope; the top curves over the sides to one seam and is built upside down, printed
  face down on Bambu Support For PLA (a second filament) as its supports' interface, the tub without supports, held by six M2
  screws as a DS Lite ([case/SHELL.md](case/SHELL.md)); the board, 72.4 x 151.64 mm, is placed to
  the case's positions (`case/positions.json`); the speaker lies sunk in a hole in the board like the cell

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
| `case/*.stl`, `case/board.dxf`, `case/positions.json` | printable parts (tub, top), board outline, positions for the board scripts: build output from `case/export.py`, not in git (run it first) |
| `case/SHELL.md` | the shell's design (the one-piece top, 2026-10-25) and why |
| `case/fitkit/` | a printable fit kit: board mock, part dummies and case coupons, to test the real parts before ordering ([its README](case/fitkit/README.md)) |
| `docs/` | [BOM](docs/bom.md) (factory and self-assembled), [assembly](docs/assembly.md), [bring-up](docs/bringup.md), [pin map](docs/pinmap.md), [design review](docs/design_review.md) |
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

## Placement search (2026-10-27)

`board/tools/place_search.py` looks for a placement that routes clean without routing every candidate: zones first, nudges second, a quick
score instead of Freerouting, then the best candidates routed on several machines. The zones, which group may use which, the turns, the
speaker's grid and the nudge limits are data in `board/placement_zones.py`; `placement.py`'s own table is each group's home. The search's
result is `board/picks.json` (explicit poses, the core's centre and turn, the cell hole's and the speaker hole's shifts, J1's slide along
the right wall); `placement.py` applies it on top of its table, so without that file (or `PLACE_PICKS`) a build is the committed placement.

```sh
cd hardware/ref_design/board
python3 tools/place_search.py zones-png OUT.png                  # the zones and which groups may use each
python3 tools/place_search.py score ['{"poses": {...}}']       # one placement: validity and score (home by default)
python3 tools/place_search.py stage1 --out=DIR --jobs=15       # beam over the groups' zones (about 12 min on 15 cores); DIR/assign.json
python3 tools/place_search.py stage2 --out=DIR --top=40        # nudges of 0.5 to 2 mm and quarter turns; DIR/candidates.json
python3 tools/place_search.py route DIR --hosts=il:8,desktop-ubuntu:8,xps13:4 --run=NAME --prefix=P_   # route.py once per candidate, in parallel
python3 tools/place_search.py collect DIR; python3 tools/place_search.py report DIR   # re-measure the boards, list them by score
python3 tools/place_search.py stop                             # kill every run, by PID, on every host
```

Validity (a candidate is built in memory with pcbnew, no board per candidate): new courtyard overlaps (a footprint's box covers its pads),
copper 0.31 mm from the outline, the cutouts and NPTH holes, nothing in the speaker's keep-out or the cell hole, the core's escape tracks
(4.4 mm round U4's pads) inside the edge, and the case's height/zone rules (`case/check_fit.py`'s `check_part` on each moved part's cached
3D extent). `route` runs `route.py --preflight` on each candidate first and drops the ones with a placement error.
Score (lower is better): the nets that must cross each bottleneck (the gap between the hole and J3, the strips beside the hole, the board's
top end) against what fits there; the ratsnest length weighted by track width; a separate term for the power nets (VBUS, VBAT, VSYS, the LDO's
output, the backlight, the speaker); a soft penalty when U2 is more than 15 mm from J1 or J2 or the LDO more than 20 mm from the core; a
congestion count. Hosts: a host without KiCad's footprint libraries, a full JRE (Freerouting's GUI needs AWT) or xdotool gets a copy of il's
under `~/mb_route` (no install, no sudo). The case is not changed by the search itself: a candidate that moves the speaker, J1 along its wall,
the cell hole or BOOT/RESET needs a case respin (`case/case.scad`) before it is adopted.

Speed: `tools/scad.py` picks OpenSCAD. With an OpenSCAD development snapshot AppImage
in `~/Applications` (openscad.org, Development Snapshots) it renders with the Manifold
backend, minutes down to seconds; otherwise the system `openscad` (CGAL). `OPENSCAD=...`
picks a binary, `OPENSCAD_BACKEND=CGAL` the old backend. Renders and kicad-cli exports
run several at once (`REF_JOBS` sets how many). `route.py` first runs a pre-flight (the placed board with the pre-routes on it through the DRC, about 4 s; it aborts with the
list when an error is not the router's: `--no-preflight` skips it, `--preflight` only runs it), then routes copies of the
board at once, each handed to Freerouting in another order (`--seed=`: 1.9 routes one file the same way every time, so
identical copies all end the same way): half the cores, at most 8 (`--attempts=N` or `ROUTE_ATTEMPTS`, 1 routes in place; a
run holds about 1.5 cores and 1 GB). The first copy with a clean DRC is kept and the others killed; one with 2 or fewer open
and 3 or fewer errors gets `--finish` on a copy while the rest keep going; anything still running at twice the first
finisher's time is killed. `--exhaustive` is the old policy (4 at once, the next seed when one ends with something open,
`--tries=M` caps the seeds, default 2N).
Freerouting gets 20 passes (it gains nothing after about 15), no 20 s start-up dialog and
no optimizer pass (`--optimize` puts it back); about 4.5 min for a clean board where three
80-pass attempts took over 10.

Waiting on long jobs (routing, slicing): wait on the job's PID or on a done-marker file, not
on its name. `pgrep -f "print_package.py 1_case"` inside `bash -c "..."` matches the waiting
shell's own command line, so the loop only ends at its cap (8.5 min lost on 2026-10-26).

```sh
python3 case/print_package.py --slice > /tmp/pk.log 2>&1 & echo $! > /tmp/pk.pid
tools/wait_done.sh --pid "$(cat /tmp/pk.pid)" --timeout 1800     # 0 done, 124 timed out
(python3 tools/build.py; touch /tmp/build.done) &                 # or end the job with a marker
tools/wait_done.sh --file /tmp/build.done --timeout 1800
```

If `pgrep -f` is unavoidable, bracket the first letter so the pattern doesn't match itself:
`pgrep -f "[p]rint_package.py"`.

Print kits: `python3 tools/build.py` only writes them (the release zips hold the case plates, `1_case` and `5_unit`;
the P1S splits `5_unit` into a and b), in seconds. `python3 tools/build.py --slice` (or `RELEASE=1`) slices them too, U1
and P1S at the same time (`SLICE_JOBS` slicers each, default cores/8), and keeps each sliced plate in `case/print/.slice_cache/`
(not in git; key: its STLs, the profiles, the packagers' sources and the slicer), so a second run with nothing changed
takes seconds. By hand: `python3 case/print_package.py [--slice] [--no-cache] [plate ...]` and `case/print_package_p1s.py`
likewise; no plate named is the release kit, named plates (or `--all`) build instead, the printer tests go to
`overhang_kit.zip` and the other plates (fit coupons, button tunings) to `case/print/extras_U1/` and `extras_P1S/`.

The stages, by hand:

```sh
cd hardware/ref_design
python3 case/export.py --stl                 # case -> positions.json, DXF, tub/top STLs
python3 case/check_buttons.py                # D-pad, A/B/X/Y, Start/Select with their silicone under the shell
cd board
python3 tools/build_sch.py                   # design.py -> schematic (tools/schwrite.py writes the sheets)
python3 tools/check_netlist.py               # KiCad's netlist == design.py ?
python3 tools/check_pins.py                  # every part's pins against its datasheet review (pin_review/)
python3 sim/models.py && python3 sim/backlight.py   # backlight sink stability (ngspice, about 40 min; only when it changes)
kicad-cli sch erc battler.kicad_sch          # 0 errors expected
RP2350B_REF=.../RPI-RP2350B-MINIMAL_R4-S1.kicad_pcb python3 tools/build_pcb.py
FREEROUTING_JAR=.../freerouting-1.9.0.jar python3 tools/route.py   # about 7 min on 16 cores, 8 seeded attempts at once, stops at the first clean one
python3 tools/route.py --finish              # only if route.py left a connection open
python3 tools/silk.py
python3 tools/sync_pcb.py                    # only for a redrawn schematic on a routed board: relink, rename nets
python3 tools/check_pcb.py                   # DRC with schematic parity
python3 tools/fab.py                         # gerbers, BOM, CPL, STEP, renders
python3 ../case/check_fit.py                 # board inside the case?
python3 ../case/fitkit.py                    # the printable fit kit (under a minute with Manifold)
python3 ../case/check_manifold.py            # every printed part's mesh manifold (each edge on two faces)?
python3 ../case/check_wall.py                # the top's wall under its side rounds thick enough (about 2 min)?
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

## IP and regulatory stance

Decided so far (2026-09-17, [10-production-board.md](../../docs/living/10-production-board.md)
section 6):

- **Rename before selling.** "Mega Blastoise" is a Pokemon name; that includes the USB
  manufacturer string the firmware reports.
- **No Pokemon markings on the device.**
- **No preloaded firmware.** Units ship blank and the owner downloads the game, which keeps
  species names, sprites and move text off the product. Today's builds fetch those assets at
  compile time (docs/reference/architecture/01-system-overview.md) and the web build publishes them; how a buyer would get them is open.
- **Real USB IDs.** The firmware uses a placeholder VID:PID; Raspberry Pi allocates product IDs
  under its vendor ID for products on its silicon.
- **Engines.** `gen1_battle` and `gen3_battle` are written in-house from the published mechanics
  and Showdown's behaviour; `gen3_battle` takes no code from `battler`.
- **Hardware sources.** The RP2350B core copy is MIT-licensed (its licence is in
  `hardware/ref_design/datasheets/`). The vendored membrane footprints are public domain. DS Lite
  cap numbers come from open models, with no geometry copied. Licence traps (share-alike and
  non-commercial footprints and models) are listed in the membrane README and the DS Lite dossier.
- **DS Lite look.** The US design patent (D554,194) expired in 2021; an EU registered design, if
  one was filed and renewed, could last until 2031, so an EUIPO search is due before selling
  anything that looks like a DS Lite.
- **Certification.** FCC Part 15B and CE as an unintentional radiator (no radio keeps it there),
  UN38.3 test documents from the cell supplier, and marketing to adults (a children's product
  triggers CPSIA testing in the US).

The validation plan defers a full IP audit, regulatory marks and EMC testing until a sale is in
view. The repository itself has no top-level licence file.
