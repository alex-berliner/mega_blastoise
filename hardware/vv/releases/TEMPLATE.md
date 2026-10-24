# REL-<BOARD>-R<rev>-<PHASE>

Copy to `releases/REL-<BOARD>-R<rev>-<PHASE>.md` (for example `REL-REFA-R1-EVT.md`), make the
first heading the same ID, and fill every field before the order is paid. The release run
(`hardware/vv/tools/release.py`, VER-143, to be written) fills most of it from a clean,
tagged worktree; Alex adds the JLC order number and the file JLC returns. `check_trace.py`
reads the fields below by their exact names: keep them as `- Name: value` lines. It checks
that every ID named here exists, that a bench result covering this release names its fab zip
sha256, and that two release records of one board with different BOMs carry different
BOARD_REVs (a BOM-only change bumps the strap R48-R51).

## Identity

- Board: ref_design
- Revision: R<n>
- BOARD_REV: <n>
- Phase: EVT | DVT | PVT
- Branch:
- Commit (full hash):
- Tag: `rel/<board>-r<rev>-<phase>` (annotated, on the commit above)
- Design baseline: the git blob hashes of board/design.py, board/placement.py and case/case.scad at the commit above (README.md's baseline must equal them)
- Date ordered:
- JLC order number:
- JLC capability page date: (the dated limits VER-17 checked)

## Files ordered

- Fab zip sha256:
- BOM sha256:
- CPL sha256:

| File | sha256 | Made by |
|---|---|---|
| battler_gerbers.zip | | the release run at the commit above |
| battler_bom_jlc.csv | | same run |
| battler_cpl_jlc.csv | | same run, centroids corrected (VER-29) |
| IPC-D-356 netlist | | same run, net ties merged (VER-26), or "none sent" |
| JLC's returned placement file | | downloaded after JLC's engineering review: JLC edits rotations on their side, so this, not our CPL, is what was placed |

Tools: KiCad (kicad-cli version), Freerouting version, OpenSCAD version, Python version.

## Order options checklist

VER-28's screenshots must match each line.

| Option | Value | Why |
|---|---|---|
| Layers | 2 | |
| Thickness | 1.6 mm | the case's crush ribs and slot |
| Surface finish | ENIG | the pills press on bare combs |
| Min hole | 0.3 mm | no small-hole fee (REQ-FAB-04) |
| Via tenting | | |
| Colour | | |
| Order number | 'Specify a location' (the JLCJLCJLCJLC marker on B.SilkS by the REV text) | never on a comb or the REV text (DF-125) |
| Edge rails | no (or routed removal if JLC insists for J5/J6 at 0.12 mm) | a V-cut ridge binds in the USB tab slot |
| E-test | against the IPC-D-356 netlist with net ties merged, or none | DF-126 |
| Confirm production file | yes | see the CPL before placement |
| PCBA type and side | Economic, top side | |
| Do not substitute | ticked on every line | the no-substitution list |
| Quantity assembled / bare | 5 / 5 from the same order | the bare boards test the combs (G-EVT-12) |

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
| U1 | SRV05-4.TCT | C13612 | the JLC footprint orientation is settled for this part only (DF-68) | |
| D2 | BAT54C,215 | C37704 | BAT54A/S/single share the package and break the latch (DF-20) | |
| J3 | FH12-50S-0.5SH(55) | C202116 | bottom contact; a top-contact part does not meet the FPC (DF-58) | |
| TH1 | NCP15XH103F03RC | C77131 | the B value sets the charge temperature window (DF-05) | |

## Case

The printed case is part of the release: fit, hook strength and the lid over J2 depend on
how it is printed (VER-173). VER-48 and VER-92 evidence name this section's STL hashes.

- case.scad commit:
- Case rev (embossed inside the tub):
- Material and brand:
- Printer:
- Slicer profile: layer height, walls, infill, the 100% modifier over J2
- Panel tape part number:
- Screws: M2 x <length> (VER-147)

| Part | File | sha256 |
|---|---|---|
| tub | | |
| lid | | |
| deck inserts | | |

## Gate results

Paste the output of `python3 hardware/vv/tools/check_trace.py --gate <phase>` run at the
commit above, unedited.

```
```

## Waivers and acceptances in force

Signed by Alex in releases/APPROVALS.md; list their A- ids here.

| Approval | Item | Decision | Holds through |
|---|---|---|---|

## Changes since the previous release

CHG records included in this release:
