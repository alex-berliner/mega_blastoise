# Solid models

Two STLs come out of here, and both are downstream of one file:

```
../layout.py  ->  *.scad  ->  renders/*.png  ->  stl/*.stl
   geometry       shape       look at it        the thing
```

`layout.py` is the only place the board is described. It emits `assembled.scad`
and `cuts_stencil.scad` with the coordinates already baked in, so a model can
never disagree with `BUILD.md`, the schematic or the cut guide. **Do not edit
the `.scad` files.** Anything you change there is gone the next time anyone
runs `layout.py`.

| File | What it is | For |
|---|---|---|
| `stl/cuts_stencil.stl` | 1 mm mask with an opening over each of the 43 cut holes | printing, then marking the board |
| `stl/assembled.stl` | the finished board, every part at its true height | checking against the enclosure, or just looking |
| `stl/assembled_plain.stl` | the same without the 1344 drilled holes | quick fit checks |

## Build

```bash
./build.sh           # regenerate, render the contact sheet, export the STLs
./build.sh render    # contact sheet only, seconds
./build.sh stl       # STLs only
```

## The loop

Same one as [docs/reference/openscad-methodology.md](../../../docs/reference/openscad-methodology.md), which this follows:

1. **Change the geometry in `../layout.py`.** Never in the `.scad`.
2. **`./build.sh render`** and **actually look at every PNG.** A model that
   compiles is not a model that is right; the display floating a millimetre
   above its standoffs looks fine in the build log.
3. **Read the `ECHO:` lines.** Every derived dimension is printed: total stack
   height, deck height, cap height, part counts.
4. **Let the `assert()`s fail loudly.** They encode the constraints that are
   easy to break by nudging a number:
   - sixteen caps, four standoffs
   - no cap stands taller than the display deck
   - the charger fits under the deck, foam included
   - the Pico's USB shell fits under the deck
   - no cap hangs off the edge of the board
5. **Fix, repeat.** One parameter group at a time.

## Parameters

Both files are parametric; override from the command line rather than editing.

```bash
openscad -o out.stl -D 'part="display"'     assembled.scad
openscad -o out.stl -D 'show_holes=false'   assembled.scad   # seconds not minutes
openscad -o out.stl -D 'sit_on_bed=false'   assembled.scad   # board face at z=0
```

`part` is `"all"`, `"board"`, `"parts"`, `"display"`, `"cell"` or `"wires"`.

For the stencil, **measure your own board first** and set `board_w` and
`board_l`. Stripboard outlines vary by a millimetre between vendors, and the
pocket is cut to fit whatever you put there.

## Why the board is built the way it is

This is OpenSCAD 2021.01, so CGAL, so no manifold backend. 1344 drilled holes
will hang for minutes if you write them the obvious way. Three things keep the
export in the seconds:

- **Drill in 2D, extrude once.** A 2D difference against 1344 circles is cheap;
  the 3D equivalent is not.
- **Cache the repeat.** The board is one drilled band that `render()` bakes
  once and 56 copies reuse. The copper is one drilled cell reused 1301 times.
  Drilling each copper strip to its own length instead means 21 separate CGAL
  renders and a union of 67 different polygons: minutes, for the same shape.
- **Keep the copper off the board.** `copper_gap` holds it 0.01 mm clear. Two
  meshes with 1300 holes each meeting on a coplanar face is the single most
  expensive thing in the file, and the gap is invisible at 0.05 mm copper.

If you change how the board or copper is built, time it before you commit it.

## What is not checked

Nothing here has been test printed. The assertions are about the model
agreeing with itself and with `layout.py`; they say nothing about whether the
stencil pocket fits your board or whether your printer holds 0.84 mm walls.
Print a corner of the stencil first.
