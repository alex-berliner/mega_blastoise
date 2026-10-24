# CHG-NN: <one-line title>

Copy to `releases/CHG-NN-<slug>.md` with the next free number, keep the first heading as
`# CHG-NN: title`, and fill it in before the change is committed. One record per change that
touches the design, a part, a footprint, the case, the fab outputs or a check. Pure doc
fixes need no record. `check_trace.py` checks that every ID named here exists.

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
| Requirements | REQ-... | statement or limit changed? |
| DFMEA rows | DF-.. | re-rate S, O, D; new failure modes? |
| Checks | VER-.. | limits or procedure changed? |
| Defects | DEF-.. | does this close one? which check proves it? |

## What to re-run

`python3 hardware/vv/tools/check_trace.py` lists every check whose evidence this change made
stale. Re-run each one and record the new result, commit and date in verification.csv. List
here any check the trace gate cannot see (a bench test whose 'covers' does not include the
changed file) and why it must be re-run anyway.

| Check | Why | New result | Commit | Date |
|---|---|---|---|---|

## Release

Goes into release: REL-...
