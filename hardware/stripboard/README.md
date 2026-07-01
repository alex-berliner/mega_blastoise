# Button matrix - stripboard layout

Permanent stripboard (Veroboard) version of the 4x4 button matrix, replacing the
flaky breadboard build. 14x 6 mm tactile switches, soldered.

Oriented for Alex's board: **strips run vertically, one continuous rail per
letter column (A..X across the top, holes 1..55 down the side).** The layout drops
onto the top of that board, full width (letters A..X), using roughly the first
22 holes.

## Files
- **`mega_blastoise_matrix.diy`** - the matrix layout, for **DIYLC** (DIY Layout
  Creator). Authoritative source for the board: every switch, track cut, jumper.
- **`mega_blastoise_matrix_full.diy`** - the **full assembly**: the same matrix
  plus switch bodies, the Raspberry Pi Pico (a 40-pin DIL stand-in), the 8 ribbon
  wires to the correct Pico pins, and a parts list.
- **`*_preview.png`** - quick renders of each.
- **`gen_matrix_diy.py`** / **`gen_matrix_full_diy.py`** - regenerate the `.diy`s
  (edit geometry here, not the XML). The full one imports the matrix one.
- **`preview_strip.py`** / **`preview_full.py`** - regenerate the preview PNGs.
- **`verify_matrix.py`** - netlist check of the matrix.

## Opening it
DIYLC is a single jar (needs Java). From the DIYLC folder:
```
java -jar diylc.jar hardware/stripboard/mega_blastoise_matrix.diy
```
(or launch DIYLC and File -> Open). Print at 100 % scale to lay the board on top.

## How the matrix maps to copper
Strips run **vertically** (down the columns), so each vertical letter-strip is one
continuous rail. The matrix uses two kinds of strip:

- **Bus strips** (a whole row net, never cut): one letter each.
  - **GP5** (P1 moves) = strip **V** | **GP7** (P1 party) = strip **P**
  - **GP8** (P2 moves) = strip **J** | **GP9** (P2 party) = strip **D**
- **Node strips** (letters **A, G, M, S**): each carries the column-legs of one
  row net and is **cut between rows** so every column gets an isolated node. The
  four columns are then tied together by **horizontal jumper wires** (the col
  nets **GP10..GP13**), one jumper row per matrix column.

Each switch straddles a bus strip and the node strip 3 letters to its left
(A<->D, G<->J, M<->P, S<->V): both legs of the row terminal land on the bus
strip, both legs of the col terminal on the node strip.

Pico physical-pin numbers below are the RP2040 board's actual header pins
(GP5=7, GP7=9, GP8=10, GP9=11, GP10=12, GP11=13, GP12=14, GP13=15 -- all on
the left column; GP6/pin 8 is skipped).

| Matrix row | Bus strip | | Matrix col | Net | run as |
|---|---|---|---|---|---|
| P1 moves (M1-M4) | **GP5** (pin 7) | | col 0 | **GP10** (pin 12) | jumper row |
| P1 party (S1-S3) | **GP7** (pin 9) | | col 1 | **GP11** (pin 13) | jumper row |
| P2 moves (M1-M4) | **GP8** (pin 10) | | col 2 | **GP12** (pin 14) | jumper row |
| P2 party (S1-S3) | **GP9** (pin 11) | | col 3 | **GP13** (pin 15) | jumper row |

col 3 exists only on the move rows (party rows have 3 buttons).

## Build order
1. **Cut the board** to size (24 wide x ~24 tall is plenty; letters A..X).
2. **Make the track cuts** - every red X in the `.diy`. They sit on the node
   strips (A, G, M, S), between adjacent column groups, isolating each column
   node. Spot cutter or a 3 mm drill twirled by hand; verify each with a meter.
3. **Solder the column jumpers first** (blue horizontals). They run *under* the
   switch bodies, so they go down before the switches. Insulated wire on the
   component side; solder only where they cross a node strip (A/G/M/S), kept
   raised/insulated over the bus strips in between.
4. **Solder the 14 switches.** Solder **all four legs**: the two on the bus-strip
   side become the row terminal, the two on the node-strip side become the col
   terminal (the marked green/blue pair just shows one leg of each). Because both
   legs of a terminal share a strip, this is more robust than the breadboard
   "bend two diagonal legs" trick.
5. **Solder the 8 header wires** (ribbon) to the orange pads -> Pico. Rows exit on
   the bus strips (**GP5/7/8/9**, top of the board); columns exit on the node
   strips at the left (**GP10-GP13**).
6. **Continuity test**: pressing a button connects its row net to its column net
   and *nothing else*.

## Switch footprint
6 mm tactile switches have a ~4.5 x 6.5 mm leg pattern = a **0.2" x 0.3" (3 x 4
hole)** rectangle on the 0.1" grid. This layout matches that: the two legs of one
terminal sit **0.2" (2 holes)** apart along a strip, and the two terminals sit
**0.3" (3 holes / 3 letters)** apart across strips. If your switches measure
differently, edit `VSPAN` / `HSPAN` at the top of `gen_matrix_diy.py` and re-run;
everything else (cuts, jumpers, nets) is footprint-independent.

## Verification
`gen_matrix_diy.py` was checked with a netlist model (strips + cuts + jumpers +
every leg -> union-find nets): all 14 switches connect exactly their row GPIO to
their col GPIO, 8 nets total, no stray shorts.

## Firmware
Matches the shipped firmware pin map (`main.rs`): rows GP5/7/8/9, cols GP10-13.
No firmware change needed - drop-in replacement for the breadboard matrix.
