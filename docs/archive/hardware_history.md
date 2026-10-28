# Hardware history

The device's hardware generations and the reference design's milestones, kept for history. The current design is
[docs/static/DESIGN.md](../static/DESIGN.md) and [hardware/ref_design/](../../hardware/ref_design/README.md).

## The generations compared

| | v1 PCB | Stripboard v2 | Reference design (REF-A) |
|---|---|---|---|
| MCU | Pico (RP2040) | Pico 2 (RP2350A, 4 MB flash) | RP2354B on the board (2 MB flash inside) |
| Display | Two 1.3 in 128x64 mono OLEDs | One 3.5 in ST7796 320x480 module | One 3.5 in ER-TFT035IPS-6 IPS panel, ILI9488, in a ZIF connector |
| Inputs | Button matrix, move buttons at screen corners | 16 tactile switches, direct GPIO, 9 mm printed caps | 20 membrane contacts under genuine-style DS Lite silicone and caps |
| Per seat | 4 moves, 3 party buttons | D-pad cross and A, B, INFO, MENU cross | D-pad, A/B/X/Y, Start, Select |
| Feedback | LED strips for HP | Screen only | Screen and a speaker |
| Power | 2xAA in the final case | 2000 mAh LiPo, TP4056 module, slide switch | About 700 mAh Li-ion cell, on-board charger with USB load sharing, soft power |
| Built | Yes | Yes | No |

## History and generations

The repository starts on 2026-04-28. Four hardware generations exist; only the first two were
built.

| Generation | Dates | What it was | Where it is now |
|---|---|---|---|
| **v1: two OLEDs** | 2026-04 to 2026-08 | Raspberry Pi Pico (RP2040), one 128x64 mono OLED per player, WS2812B LED strips for HP and party, a button matrix with move buttons at the screen corners. Built first on breadboard and stripboard, then as a fabricated PCB designed by a collaborator (Eddy Zheng; files committed 2026-07-13), which ran in a printed case on 2xAA cells by early August | Removed from the tree on 2026-10-19; tag `v1-archive` holds its board, stripboard and case files; notes in [docs/archive/v1/](../archive/v1/DESIGN.md) |
| **Stripboard v2: single screen** | designed 2026-08-19/20, running 2026-08-26 | Pico 2 (RP2350), one 3.5 in ST7796 320x480 panel, 16 direct-GPIO tactile switches, a LiPo with a TP4056 module, all on one 24x56 stripboard | Built and working; the firmware's current target. Its files are at tag `stripboard-v2-archive` |
| **prod_board / prod_case: Alex's production board** | from 2026-09-23 | Alex's own KiCad board and OpenSCAD case, made by hand to learn the tools | Removed from the tree on 2026-10-28 (Alex: not needed) |
| **ref_design (REF-A): the reference design** | from 2026-09-24 | Claude's complete, orderable version: RP2354B, 3.5 in IPS panel in a ZIF connector, DS Lite membranes, one Li-ion cell, speaker, 2-layer JLCPCB-assembled board, printed DS Lite style case | Designed and checked, not yet ordered ([hardware/ref_design/](../../hardware/ref_design/README.md)) |

Milestones:

| Date | Milestone |
|---|---|
| 2026-04-28 | First commit: Pico firmware wrapping the open-source `battler` engine, a typed event pipeline, a host test harness |
| 2026-05-09 | Web build (WASM) published on GitHub Pages; lobby with AI demo and ready-up |
| 2026-05-15 | Hand-written `gen1_battle` replaces `battler` on the device: about 40 times less heap |
| 2026-07-09 | `ChoiceCollector` and the shared OLED controller move all input and display semantics into core; the zero-drift rule is written down |
| 2026-07-13 | `gen1_battle` ports Showdown's Gen 1 mechanics and format rules |
| 2026-08-06 | Single-screen redesign specified ([09-single-screen.md](../reference/architecture/09-single-screen.md)) and a working web client built; tag `mono-renderer-final` marks the last mono renderer |
| 2026-08-15 to 08-19 | `gen3_battle` added; Showdown parity and fuzz suites; Gen 3 moves, abilities and held items brought in under the fuzzer |
| 2026-08-20 | Stripboard v2 generated end to end from one layout script |
| 2026-08-26 to 08-28 | The game runs on the stripboard's panel; the game loop moves into core as `DeviceRunner`; dirty-tile updates |
| 2026-09-17 to 09-21 | Production board planned ([10-production-board.md](../living/10-production-board.md)): DS Lite membranes, bare RP2350B, 3.5 in IPS class |
| 2026-09-24 | Reference design started; on the same day its 4-layer first version (tag `archive/ref-design-4layer`) became a cheaper 2-layer build on `main` |
| 2026-09-25/26 | Three audit rounds and a two-model debate settle the open circuit decisions; printable fit kit |
| 2026-10-05 | First case print and fixes; joint validation plan; datasheet pin-review gate; exploded 3D viewer on Pages |
| 2026-10-07 | Backlight charge pump out of stock, replaced by discrete current sinks |
| 2026-10-09 | The cell moves under the board |
| 2026-10-19 | Start and Select added per seat, power moved from seat 1's Y to seat 1's Start; screen in a lid pocket; debug pads; DS Lite shell; v1 removed from the tree; crates moved under `crates/`; Pages front page; DS Lite research packet |
| 2026-10-20 | Schematic written directly in KiCad 10 format; speaker plays out of the side wall on a plug-in header; Alex's silicone pad measurements folded in; print kits |
| 2026-10-21 | BOM audit: per-unit cost updated, and the RP2354B and its core inductor found out of stock (see ref_design/docs/bom.md) |

Tags worth knowing: `v1-archive` (the v1 hardware), `mono-renderer-final` (last two-OLED
renderer), `pre-collector-refactor`, `archive/ref-design-4layer`,
`ref-design-battery-under-board` (the case as first printed).
