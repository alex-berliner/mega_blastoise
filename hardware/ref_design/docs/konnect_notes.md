# Konnect: what this design ran into

**History.** Since 2026-10-20 `board/tools/build_sch.py` writes the sheets itself
(`board/tools/schwrite.py`, whole files in KiCad 10's format, round-trip checked
against KiCad's own writer), because what the sheets needed next was exactly what
Konnect couldn't write: hidden power-symbol references (the `#PWR` fields parked
at the page's top-left below), field positions, label justification and sheet
name fields. What follows is the record of the Konnect-written version.

The schematic here was written entirely through Konnect (`board/tools/build_sch.py`
drove it over stdio) and checked by exporting KiCad's own netlist. These are
the problems that check and KiCad's renderer turned up, with the workaround
the generator uses. KiCad 10.0.6, Konnect from `/home/alex/Konnect` as of
2026-09-24.

## Connectivity (these break netlists)

| Tool | What happens | Workaround here |
|---|---|---|
| `add_schematic_component` | A derived symbol (one whose library entry `extends` another, e.g. `Regulator_Switching:TPS63001`, `Memory_Flash:W25Q128JVP`, `Transistor_FET:2N7002`, `Power_Protection:USBLC6-2SC6`) is embedded unflattened as `(symbol "Lib:Child" (extends "Lib:Parent"))`. KiCad then reads the placed symbol with no pins: every wire to it is silently unconnected. ERC doesn't flag it. | Place the root symbol (TPS63000, W25Q32JVZP, Q_NMOS_GSD, USBLC6-2P6) with the real part as the value. |
| `add_power_symbol` | For a name with no symbol in `power.kicad_sym` (e.g. `VSYS`) it writes `lib_id "power:VSYS"` with no embedded definition. | Use `power:VCC` through `add_schematic_component` with value `VSYS` (KiCad names the net after the value). |
| `batch_connect_to_net` | Writes `(net_label ...)`, which KiCad can't load. | `connect_to_net` / `add_schematic_net_label` per pin. |
| pin location queries | Report a pin's root, not its connection tip, so a label placed there never connects. | `tools/kisym.py` reads the library and computes tips. |
| `delete_schematic_net_label` | Given a name and coordinates, deleted a global label of that name elsewhere, not the local label at the coordinates. | Not used. |
| `add_schematic_component` | Only finds libraries in `KICAD10_SYMBOL_DIR` and `/usr/share/kicad/symbols`: a project library in the project's `sym-lib-table` is not looked at, and the symbol is placed with no embedded definition (no pins again). | `build_sch.py` sets `KICAD10_SYMBOL_DIR` to `board/lib` before starting Konnect (KiCad's own folder is still searched after it). |

## Drawing (these make sheets hard to read)

| Tool | What happens | Workaround here |
|---|---|---|
| labels (`connect_to_net`, `add_schematic_net_label`) | No `(justify ...)` is written, so a label at 180 degrees draws its text to the right of its anchor and one at 270 draws upwards: back over its own wire and into the symbol. A global label's box always lies right of (or above) its anchor. | Local labels sit at the far end of a wire with their text lying on it. A global label fed from the right is drawn as a hook: the label a row above (or below) the wire, joined at its left end; up-facing pins get vertical global labels. |
| `rotate_schematic_label` | Says it updates the justify; it only rewrites `(at ...)`, and it can't find local labels at all ("Label block not found": it looks for `(net_label`, Konnect writes `(label`). | Not used. |
| `add_power_symbol` | Reference and Value have no `(at ...)`, so KiCad draws them at the sheet origin (top-left corner), and the reference is visible. | Power symbols go in with `add_schematic_component` (below). |
| power symbols through `add_schematic_component` | Fields can't be hidden, so every GND and rail symbol would show its `#PWR` reference next to it. | Both fields are parked off the printed page (top-left, outside the paper; KiCad's editor shows them there) and the rail's name is written as plain text by its arrow. GND symbols carry no text. |
| `add_hierarchical_sheet` | Sheetname and Sheetfile have no position: drawn at the page corner, not on the sheet box. | Text items next to each box on the root sheet. |
| `add_schematic_component` | Reference always at y-3.81 and value at y+3.81 from the origin, whatever the library says, turned with the symbol. On a vertical resistor both land on the pin tips. | `move_schematic_component` moves the symbol and leaves its fields where they are: each part is placed at the point its fields should straddle, then moved to where it goes (`tools/schdraw.py`). |
| `add_wire` / `batch_add_wire` | Adds a junction at every T in the sheet on every call, so earlier T's get one more each time. | The drawing kit splits wires where they meet, so no wire ends in another's middle, and places the junctions itself with `batch_add_junction`. |
| instances | Symbols in sub-sheets get `(project "" (path "/" ...))`; KiCad falls back to the Reference property. Fine while each sheet is used once. | None needed yet. |

## Libraries

| Tool | What happens | Workaround here |
|---|---|---|
| `create_symbol` | Writes the pins only: no body rectangle, and pin names and numbers hidden. On a sheet the part is a cluster of loose pin stubs. | `tools/sym_lib.py` writes the symbol and has `kicad-cli sym upgrade` re-save it through KiCad. |
| `register_symbol_library` | The `sym-lib-table` it writes starts with `(fp_lib_table`, so KiCad rejects the table. | `sym-lib-table` written by hand (it is not a design file). |

## Missing

- `edit_schematic_component` / `batch_edit_schematic_components` `fields` only update fields that already exist ("Field 'LCSC' not found"); new ones need `add_component_annotation`.
- No way to set: paper size (A4 is hard-coded), title block, `in_bom` / `on_board` / DNP, field position (other than the move above) or visibility, label or text justification, mirroring, graphic lines or boxes (to frame a block).
- Konnect's PCB tools need a running KiCad (IPC); the board here is built with KiCad's own `pcbnew` Python module instead.
