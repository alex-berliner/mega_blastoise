# Production case: preliminary layout

The case decides where things go; the PCB follows. `layout.scad` is the single
source of truth for the board outline, the glass window and the centre of all
16 membrane contacts. KiCad copies these, never the other way round.

Replaces nothing: `cad/` holds the earlier dual-OLED device and is left as is.

## Files

| File | What | Regenerate with |
|---|---|---|
| `layout.scad` | parametric layout, every number named at the top | (source) |
| `board.dxf` | board outline, for KiCad **Edge.Cuts** | `openscad -o board.dxf -D 'part="board"' layout.scad` |
| `pads.dxf` | contact centres + outline, for a KiCad **User** layer to snap to | `openscad -o pads.dxf -D 'part="pads"' layout.scad` |
| `template.svg` | 1:1 paper template: outline, glass, silicone, caps | `openscad -o template.svg -D 'part="template"' layout.scad` |

Coordinates: origin at the case centre, +Y towards seat 2. The echo block
prints every contact in this frame and in KiCad's (centre at 100,100, Y down).
Run `openscad -o /dev/null layout.scad` to see them.

## Current numbers (preliminary)

- Case 90 x 157 mm, board 85 x 152 mm (2 mm wall + 0.5 mm clearance)
- Glass 54.5 x 83 centred; each seat's clusters at y = +/-59.5
- D-pad on each player's left, ABXY on the right; seat 2 is seat 1 rotated 180
- Thickness estimate 19.1 mm, dominated by the 10 mm 103450 cell

| Ref | Button | KiCad X, Y |
|---|---|---|
| SW1 | P1 up | 74, 152.5 |
| SW2 | P1 down | 74, 166.5 |
| SW3 | P1 left | 67, 159.5 |
| SW4 | P1 right | 81, 159.5 |
| SW5 | P1 A | 133.5, 159.5 |
| SW6 | P1 B | 126, 167 |
| SW7 | P1 X | 126, 152 |
| SW8 | P1 Y | 118.5, 159.5 |
| SW9 | P2 up | 126, 47.5 |
| SW10 | P2 down | 126, 33.5 |
| SW11 | P2 left | 133, 40.5 |
| SW12 | P2 right | 119, 40.5 |
| SW13 | P2 A | 66.5, 40.5 |
| SW14 | P2 B | 74, 33 |
| SW15 | P2 X | 74, 48 |
| SW16 | P2 Y | 81.5, 40.5 |

## Estimates still standing in for measurements

| Parameter | Now | Replace with |
|---|---|---|
| `cluster_spacing` | 52 mm, Game Boy-like | the paper template test below |
| `piece_d` | 26 mm | calipers on a real DS Lite D-pad and ABXY silicone |
| cap sizes | 6 mm face, 18 mm cross | calipers on real caps |
| `aa_offset_y` | 0, glass assumed centred | the chosen panel's mechanical drawing |
| `usb_*`, `pwr_*` | rough | the chosen parts' footprints |
| above-board height | 4.0 mm | silicone + cap + glass stack from real parts |

## Doing it professionally

1. **One source of truth.** Positions live here. If a button moves, it moves
   in `layout.scad`, the files are regenerated, and KiCad follows.
2. **Shared datum.** Case centre = board centre. In KiCad put the grid origin
   at (100,100) so typed coordinates match this file.
3. **Import, don't redraw.** Edge.Cuts comes from `board.dxf`; contacts are
   placed at the table's coordinates, checked against `pads.dxf` on a User
   layer, then locked.
4. **Measure before committing copper.** Every ESTIMATE above gets a real
   number before the board is ordered.
5. **Keep-out zones.** Next pass: no top-side parts under the silicone, a
   height limit under the glass, and the cell's footprint on the bottom.
6. **Mounting.** Screw bosses are decided by the case, and the board gets
   holes to match. Not placed yet.
7. **Close the loop.** Once the board exists, export it from KiCad in 3D and
   check it inside the case model before ordering.
8. **Physical checks, cheapest first.** Paper template now; a printed
   faceplate with real membranes before the board order.

## The paper template test

Print `template.svg` at **100% scale**, no "fit to page". Measure the 50 mm
bar first. Put the real silicone and caps on it, sit across a table from
someone, and play. Adjust `cluster_spacing` until it feels right.
