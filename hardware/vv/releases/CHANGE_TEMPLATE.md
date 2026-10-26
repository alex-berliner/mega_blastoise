# CHG-NN: <one-line title>

Copy to `releases/CHG-NN-<slug>.md` with the next free number, keep the first heading as
`# CHG-NN: title`, and fill it in before the change is committed. One record per change that
touches the design, a part, a footprint, the case, the fab outputs or a check. Pure doc
fixes need no record.

`check_trace.py` reads two fields: the commits on the `- Commit(s):` line, and the VER ids in
the "What to re-run" section. Once a release record exists, every commit after it that
touches the design (board/design.py, board/placement.py, board/lib/, battler.kicad_pcb,
battler.kicad_pro, the board/*.kicad_sch schematics, the board/tools tree, case/case.scad or
case/export.py) must be named on some change record's Commit(s) line, or every gate fails
(R6). A bench result on built units (covers `release:REL-...`) goes stale only when a change
record made after it names the check under "What to re-run". The list is not hand-picked:
check_trace refuses a record that omits a bench test (method T, a release cover) whose DFMEA
row or requirement names a part its commits changed in design.py (O-R21).

A BOM-only change (a resistor value, a part for stock) still bumps BOARD_REV in design.py,
so the next release record differs from the last (check_trace refuses the same BOARD_REV on
a different BOM). If the change moves any design path, update README's design baseline in
the same commit: the default mode fails until it matches (DEF-47).

- Date:
- Author:
- Commit(s):
- Reason: (a defect, DEF-nn; a requirement, REQ-...; a stock-out; Alex's request)

## What changed

Files and the exact change: part values, LCSC numbers, nets, placement, copper, case
parameters, check limits.

## What it touches

| Kind | IDs | How |
|---|---|---|
| Requirements | REQ-... | statement or limit changed? (a changed statement needs Alex's approval again) |
| DFMEA rows | DF-.. | re-rate S, O, D; new failure modes? |
| Checks | VER-.. | limits or procedure changed? |
| Defects | DEF-.. | does this close one? which check proves it? |

## What to re-run

`python3 hardware/vv/tools/check_trace.py` lists every analysis check whose evidence this
change made stale. Name here every check that must be re-run, including each bench check on
built units that the change affects: naming it here is what makes its result stale.

| Check | Why | New result | Commit | Date |
|---|---|---|---|---|

## Release

Goes into release: REL-...
