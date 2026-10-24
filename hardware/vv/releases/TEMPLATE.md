# REL-<BOARD>-R<rev>-<PHASE>

Copy to `releases/REL-<BOARD>-R<rev>-<PHASE>.md` (for example `REL-REFA-R1-EVT.md`), make the
first heading the same ID, and fill every field before the order is paid. `check_trace.py`
checks that every ID named in a release record exists. VER-42 passes when this record is
complete and its hashes match the files ordered.

## Identity

- Board: ref_design | prod_board
- Revision: R<n> (BOARD_REV strap value: <n>)
- Phase: EVT | DVT | PVT
- Branch:
- Commit (full hash):
- Tag: `rel/<board>-r<rev>-<phase>` (annotated, on the commit above)
- Date ordered:
- JLC order number:

## Files ordered

| File | sha256 | Made by |
|---|---|---|
| battler_gerbers.zip | | `python3 hardware/ref_design/board/tools/fab.py` at the commit above |
| battler_bom_jlc.csv | | same run |
| battler_cpl_jlc.csv | | same run |
| IPC-D-356 netlist | | same run (VER-26) |

Tools: KiCad (kicad-cli version), Freerouting version, OpenSCAD version, Python version.

## Order options

Layers 2, thickness 1.6 mm, finish ENIG, min hole 0.3 mm, via tenting, colour, remove order
number (or where JLC prints it), e-test against the IPC-D-356 netlist, "confirm production
file", PCBA type, assembly side, quantity assembled, quantity bare, edge rails (yes or no and why).

## Locked BOM

Every assembled line, as ordered. "Locked" means the order says "do not substitute" for it.

| Designators | Value | MPN | LCSC | Stock on order day | Locked |
|---|---|---|---|---|---|
| | | | | | |

## No-substitution list

These must arrive as designed or the order stops. Pre-ordered or consigned parts say so.

| Designator | Part | LCSC | Why it cannot change | Pre-ordered or consigned |
|---|---|---|---|---|
| U4 | RP2354B (2 MB flash inside) | C39843328 | an RP2350B has no flash; another package does not fit | |
| L2 | Abracon AOTA-B201610S3R3-101-T | C42411119 | the RP2350 hardware guide names this inductor for the core switcher; its polarity dot matters | |
| U2 | TP4056-42-ESOP8 | C16581 | the -42 variant sets 4.2 V; other variants overcharge (DF-01) | |
| U3 | ME6211C33M5G-N | C82942 | another output voltage destroys the chip at first power (DF-29) | |
| U1 | SRV05-4.TCT | C13612 | the JLC footprint orientation was settled for this part only (DF-68) | |
| D2 | BAT54C,215 | C37704 | BAT54A/S/single share the package and break the latch (DF-20) | |
| J3 | FH12-50S-0.5SH(55) | C202116 | bottom contact; a top-contact part does not meet the FPC (DF-58) | |
| TH1 | NCP15XH103F03RC | C77131 | the B value sets the charge temperature window (DF-05) | |

## Gate results

Paste the output of `python3 hardware/vv/tools/check_trace.py --gate <phase>` run at the
commit above, unedited.

```
```

## Waivers in force

| Waiver | Criterion | Reason | Approver | Date |
|---|---|---|---|---|

## Changes since the previous release

CHG records included in this release:
