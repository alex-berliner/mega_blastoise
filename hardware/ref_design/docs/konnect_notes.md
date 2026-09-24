# Konnect: what this design ran into

The schematic here is written entirely through Konnect (`board/tools/build_sch.py`
drives it over stdio) and checked by exporting KiCad's own netlist. These are
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

## Drawing (these make sheets hard to read)

| Tool | What happens | Workaround here |
|---|---|---|
| labels (`connect_to_net`, `add_schematic_net_label`) | No `(justify ...)` is written, so a label at 180 degrees draws its text to the right of its anchor and one at 270 draws upwards: back over its own wire and into the symbol. | Left- and down-facing pins get a longer stub with the label at its far end, text lying along the wire. Global labels on left-facing pins fan out below the symbol instead. |
| `add_power_symbol` | Reference and Value have no `(at ...)`, so KiCad draws them at the sheet origin (top-left corner), and the reference is visible. | Power symbols are placed once per sheet in a "rails" column; the pins carry plain labels with the rail name. |
| `add_hierarchical_sheet` | Sheetname and Sheetfile have no position: drawn at the page corner, not on the sheet box. | Text items next to each box on the root sheet. |
| `add_schematic_component` | Reference always at y-3.81 and value at y+3.81 from the origin, horizontal, whatever the library says. On a vertical resistor both land on the pin tips. | Two-pin parts are drawn horizontal, where those spots are clear of the wires. |
| instances | Symbols in sub-sheets get `(project "" (path "/" ...))`; KiCad falls back to the Reference property. Fine while each sheet is used once. | None needed yet. |

## Missing

- `edit_schematic_component` / `batch_edit_schematic_components` `fields` only update fields that already exist ("Field 'LCSC' not found"); new ones need `add_component_annotation`.
- No way to set: paper size (A4 is hard-coded), title block, `in_bom` / `on_board` / DNP, field position or visibility, mirroring.
- Konnect's PCB tools need a running KiCad (IPC); the board here is built with KiCad's own `pcbnew` Python module instead.
