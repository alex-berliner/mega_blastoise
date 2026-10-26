# REL-<BOARD>-R<rev>-<PHASE>

Copy to `releases/REL-<BOARD>-R<rev>-<PHASE>.md` (for example `REL-REFA-R1-EVT.md`), make the
first heading the same ID, and fill every field before the order is paid. The release run
(`hardware/vv/tools/release.py`, VER-143, to be written) fills most of it from a clean,
tagged worktree; Alex adds the JLC order number and the file JLC returns. `check_trace.py`
reads the fields below by their exact names: keep them as `- Name: value` lines. It checks
that every ID named here exists, that a bench result covering this release names its fab zip
sha256, and that two release records of one board with different BOMs carry different
BOARD_REVs (a BOM-only change bumps BOARD_REV in design.py, printed on the silkscreen).

## Identity

- Board: ref_design
- Revision: R<n>
- BOARD_REV: <n>
- Phase: EVT | DVT | PVT
- Branch:
- Commit (full hash):
- Tag: `rel/<board>-r<rev>-<phase>` (annotated, on the commit above)
- Design baseline: the git object hashes of every design path at the commit above (design.py, placement.py, board/lib, battler.kicad_pcb, battler.kicad_pro, the schematics, the board/tools tree, case.scad, case/export.py); README.md's baseline must equal them, and the order branch README names has no design commit after it (R8)
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
| case/positions.json | | case/export.py at the commit above (gitignored, so not in the baseline: F-R13); placement.py read it |
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
| Order number | 'Specify a location' (the JLCJLCJLCJLC marker on B.SilkS by the BOARD_REV text) | never on a comb, a dome pad or the BOARD_REV text (DF-125) |
| Edge rails | no (or routed removal if JLC insists for J5 at the left edge) | a V-cut ridge binds in the USB tab slot |
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
| U4 | RP2354B (2 MB flash inside) | C39843328 | an RP2350B has no flash; another package does not fit (DF-69); out at JLC and LCSC on 2026-10-21 | pre-order or consign (PLAN.md decision D2) |
| L2 | Abracon AOTA-B201610S3R3-101-T | C42411119 | the RP2350 hardware guide names this inductor for the core switcher; its polarity dot matters (DF-93); out on 2026-10-21 | pre-order or consign (D2) |
| U2 | BQ24073RGTR | C15220 | a BQ24075 puts up to 5.5 V on VSYS, a BQ24079 charges to 4.1 V, a BQ24072/74 differs on pin 15 and the timers (DF-144) | |
| U3 | RT9013-33GB | C47773 | another output voltage of the family (1.2-3.3 V) runs the chip low or puts the panel over its VCI (DF-155) | |
| U1 | SRV05-4.TCT | C13612 | the JLC footprint orientation is settled for this part only: KiCad's SOT-23-6 turned 180 degrees (DF-68) | |
| U8, U9 | SN74HC165PWR | C43420 | a 74HCT165 wants 4.5-5.5 V and misreads at 3.3 V (DF-160) | |
| TH1 | SDNT1005X103F3950FTF (10k, B3950, 1%) | C95939 | its B value sets the charge window (3.5-47.2 C); a fixed 10k in its place disables the temperature pause (DF-146, DF-147) | |
| D2 | BAT54C,215 | C37704 | BAT54A/S/single share the package and break the latch (DF-20) | |
| J3 | FH12-50S-0.5SH(55) | C202116 | bottom contact; a top-contact part does not meet the FPC (DF-58) | |
| J2, J4 | Molex 53261-0271 | C177225 | the cell's and the speaker's PicoBlade plugs must latch in it (DF-113) | |
| J5 | SM03B-SRSS-TB(LF)(SN) | C160403 | the Megastar clone (C7430445) has another land pattern and is taller (DF-64) | |
| X1 | ABM8-272-T3 | C20625731 | Raspberry Pi's core layout and its 15 pF load caps are drawn for it (DF-92) | |

## Case

The printed case is part of the release: fit, hook strength, the arms' laps and the screws'
hold depend on how it is printed (VER-173). VER-48 and VER-92 evidence name this section's STL hashes.

- case.scad commit:
- Case rev (embossed inside the tub):
- Material and brand:
- Printer:
- Slicer profile: layer height, walls, infill, the overhang the parts were exported at (45 or 60: case/print/case_45_60.zip)
- Panel tape part number:
- Screws: M2 x <length> in the island's pilots, M2 x <length> in the decks' (VER-147)

| Part | File | sha256 |
|---|---|---|
| tub | | |
| top (the island) | | |
| deck (printed twice) | | |
| build bars | | |

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
