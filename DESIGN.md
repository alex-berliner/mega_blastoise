# Mega Blastoise: design document

The whole project on one page: what it is, why it is built this way, how the pieces fit, and
where the details live. It is current as of 2026-10-21. Detailed specifications (pin tables,
part values, BOM lines, layouts) stay in the documents this one links to; where a number appears
here it is to explain a decision, and the linked document is the authority.

Owner: Alex Berliner. Much of the software and the hardware reference design were written with
Claude (Anthropic's model) working in this repository.

## Contents

1. [Purpose and goals](#1-purpose-and-goals)
2. [Product concept and player experience](#2-product-concept-and-player-experience)
3. [History and generations](#3-history-and-generations)
4. [Decisions and their reasons](#4-decisions-and-their-reasons)
5. [Software architecture](#5-software-architecture)
6. [Hardware](#6-hardware)
7. [Manufacturing and cost](#7-manufacturing-and-cost)
8. [IP and regulatory stance](#8-ip-and-regulatory-stance)
9. [Verification and tooling](#9-verification-and-tooling)
10. [Status, known gaps and open decisions](#10-status-known-gaps-and-open-decisions)
11. [Where things live](#11-where-things-live)
12. [Glossary](#12-glossary)

---

## 1. Purpose and goals

Mega Blastoise is a two-player handheld Pokemon battler. The device lies flat on a table between
two people sitting across from each other. One shared colour screen in the middle is split
between them, and each player has a DS Lite style D-pad and face buttons at their own end. They
pick random teams and fight a full singles battle under Gen 1 or Gen 3 rules. Everything runs on
one microcontroller: no phone, no PC, no network.

The same game also runs in a web browser, built from the same Rust code, and is published on
GitHub Pages: [play it here](https://alex-berliner.github.io/mega_blastoise/battler/device.html).

Goals, in the order the project has held them:

1. **Two strangers can sit down and play.** The first device was aimed at demoing at a
   convention (Open Sauce 2026, per [the v1 overview](docs/archive/v1/DESIGN.md)): no setup,
   no loose pieces, a first match within seconds. An attract demo and a "hold A to fight the AI"
   option exist for a lone visitor.
2. **It must feel like a purpose-built game device**, not an emulator or a phone app. Since
   2026-10-19 the explicit target is that the case should feel like holding the bottom half of a
   Nintendo DS Lite ([hardware/ds_lite/](hardware/ds_lite/README.md)).
3. **The battle rules must be verifiably the same as a community-accepted reference.** Since
   2026-08-16 the goal is a complete recreation of Pokemon Showdown's Gen 1 and Gen 3 singles
   combat, abilities and held items included. Showdown wins every disagreement, and the engines
   are checked against it by differential tests.
4. **The browser build and the device must be indistinguishable** in everything a player sees
   and does (the zero-drift rule, [section 5.3](#53-the-zero-drift-rule)). The browser is where
   the game is developed and tested; the device is the source of truth when they disagree.
5. **The hardware should become something that could be batch-produced and sold.** Since
   2026-09-17 the hand-wired prototype is being turned into a factory-assembled PCB with a
   printed case.

Out of scope for now: radios (no Wi-Fi, no Bluetooth), doubles battles, per-move attack
animations beyond a small shared vocabulary, and shipping any Pokemon assets on the product.

## 2. Product concept and player experience

### The table, the screen and the seats

The console is never picked up or turned. Two seats face each other across it, so no screen may
assume a single reader. The panel is portrait and split into two halves, one per seat, and the
far half is drawn rotated 180 degrees so both players read their own half upright. There is
exactly one arrangement and no orientation setting.

During a battle the two halves become one shared scene: both active Pokemon stand side by side in
a band across the middle of the panel, each upright to its owner and facing the other, with each
seat's HP plate under its own mon and the narration drawn in both halves. When either half shows
something private (move choice, party list, battle log, options), a divider returns between the
halves. Each private half is framed in its seat's colour (red or white), so the whole panel reads
as a Poke Ball. The full rules are in
[docs/architecture/09-single-screen.md](docs/architecture/09-single-screen.md) section 3, and the
visual spec is [crates/mega_blastoise_web/www/ui_flow.html](crates/mega_blastoise_web/www/ui_flow.html).

The screen needs an IPS panel: a TN panel shows inverted contrast to whichever seat views it from
the far side.

### Controls

Each seat has the same controls, placed with rotational symmetry so both players get an identical
hand position. The reference design follows the DS Lite: a D-pad, A/B/X/Y in a diamond, and Start
and Select side by side between the clusters.

| Control | Meaning |
|---|---|
| D-pad | Moves the cursor (the web page repeats a held direction; the firmware does not yet) |
| A | Confirms. Holding A in the lobby summons the AI as the opponent |
| B | Backs out; every state has a back path. On a locked-in choice it takes the choice back |
| X (INFO, shown as `?`) | Explains what the cursor is on (move details, party stats); outside a choice it opens the battle log |
| Y (MENU) | Currently behaves as B on both platforms; a real menu action is still to come |
| Start, Select | Present on the hardware and the web page, with no game meaning in core yet. On the reference design, seat 1's Start is also the power button |

The cursor model replaced the first device's scheme, where a physical corner button meant the move
drawn at that corner and a stray tap committed a choice with no way back.

### A session

1. **Attract demo.** After 15 s idle in the lobby, the device plays an AI-versus-AI battle across
   both halves.
2. **Gen picker and options.** Gen 1 or Gen 3 (the Gen 3 entry is labelled "preview"), drawn into
   both halves; either player can drive it. Options open per seat over shared settings: team size
   (3v3 or 6v6), text speed, and a tutorial shown before the first game.
3. **Lobby.** Each player presses to ready up; holding A fights the AI instead.
4. **Teams** are a fresh random draft every battle, rematches included, from Showdown's random
   battle sets for the chosen generation.
5. **Choosing.** Each player picks a move or a switch on their own half with the cursor. A
   committed player sees a waiting screen and can take the choice back; once both are ready the
   turn fires after a 1 s grace window.
6. **Playback.** Narration, HP drain, the sprite bob and a small set of attack effects play over
   the shared scene. The battle log (the last 50 lines, with damage numbers and the reasons for
   crits and effectiveness) is available on `?` and pauses only the reader.
7. **Faints and switches** ask the owning player for the next mon, showing the rival's side.
8. **Result and recap**, then back to the lobby.

### Open information

Information is open: each player can see the rival's active mon, its HP and status. Only the move
a player has locked in stays hidden until playback. The v1 device had a "concealed" mode with
per-player screens; it was deleted in the single-screen redesign (2026-08-06), whose one screen
sits in view of both players.

## 3. History and generations

The repository starts on 2026-04-28. Four hardware generations exist; only the first two were
built.

| Generation | Dates | What it was | Where it is now |
|---|---|---|---|
| **v1: two OLEDs** | 2026-04 to 2026-08 | Raspberry Pi Pico (RP2040), one 128x64 mono OLED per player, WS2812B LED strips for HP and party, a button matrix with move buttons at the screen corners. Built first on breadboard and stripboard, then as a fabricated PCB designed by a collaborator (Eddy Zheng; files committed 2026-07-13), which ran in a printed case on 2xAA cells by early August | Removed from the tree on 2026-10-19; tag `v1-archive` holds its board, stripboard and case files; notes in [docs/archive/v1/](docs/archive/v1/DESIGN.md) |
| **Stripboard v2: single screen** | designed 2026-08-19/20, running 2026-08-26 | Pico 2 (RP2350), one 3.5 in ST7796 320x480 panel, 16 direct-GPIO tactile switches, a LiPo with a TP4056 module, all on one 24x56 stripboard | Built and working; the firmware's current target ([hardware/stripboard_v2/](hardware/stripboard_v2/BUILD.md)) |
| **prod_board / prod_case: Alex's production board** | from 2026-09-23 | Alex's own KiCad board and OpenSCAD case, made by hand to learn the tools | In progress ([hardware/prod_case/](hardware/prod_case/README.md), `hardware/prod_board/`) |
| **ref_design (REF-A): the reference design** | from 2026-09-24 | Claude's complete, orderable version: RP2354B, 3.5 in IPS panel in a ZIF connector, DS Lite membranes, one Li-ion cell, speaker, 2-layer JLCPCB-assembled board, printed DS Lite style case | Designed and checked, not yet ordered ([hardware/ref_design/](hardware/ref_design/README.md)) |

Milestones:

| Date | Milestone |
|---|---|
| 2026-04-28 | First commit: Pico firmware wrapping the open-source `battler` engine, a typed event pipeline, a host test harness |
| 2026-05-09 | Web build (WASM) published on GitHub Pages; lobby with AI demo and ready-up |
| 2026-05-15 | Hand-written `gen1_battle` replaces `battler` on the device: about 40 times less heap |
| 2026-07-09 | `ChoiceCollector` and the shared OLED controller move all input and display semantics into core; the zero-drift rule is written down |
| 2026-07-13 | `gen1_battle` ports Showdown's Gen 1 mechanics and format rules |
| 2026-08-06 | Single-screen redesign specified ([09-single-screen.md](docs/architecture/09-single-screen.md)) and a working web client built; tag `mono-renderer-final` marks the last mono renderer |
| 2026-08-15 to 08-19 | `gen3_battle` added; Showdown parity and fuzz suites; Gen 3 moves, abilities and held items brought in under the fuzzer |
| 2026-08-20 | Stripboard v2 generated end to end from one layout script |
| 2026-08-26 to 08-28 | The game runs on the stripboard's panel; the game loop moves into core as `DeviceRunner`; dirty-tile updates |
| 2026-09-17 to 09-21 | Production board planned ([10-production-board.md](docs/architecture/10-production-board.md)): DS Lite membranes, bare RP2350B, 3.5 in IPS class |
| 2026-09-24 | Reference design started; on the same day its 4-layer first version (tag `archive/ref-design-4layer`) became a cheaper 2-layer build on `main` |
| 2026-09-25/26 | Three audit rounds and a two-model debate settle the open circuit decisions; printable fit kit |
| 2026-10-05 | First case print and fixes; joint validation plan; datasheet pin-review gate; exploded 3D viewer on Pages |
| 2026-10-07 | Backlight charge pump out of stock, replaced by discrete current sinks |
| 2026-10-09 | The cell moves under the board (the cell-on-board layout stays on branch `ref-design-cell-on-board`) |
| 2026-10-19 | Start and Select added per seat, power moved from seat 1's Y to seat 1's Start; screen in a lid pocket; debug pads; DS Lite shell; v1 removed from the tree; crates moved under `crates/`; Pages front page; DS Lite research packet |
| 2026-10-20 | Schematic written directly in KiCad 10 format; speaker plays out of the side wall on a plug-in header; Alex's silicone pad measurements folded in; print kits |
| 2026-10-21 | BOM audit: per-unit cost updated, and the RP2354B and its core inductor found out of stock (see [section 7](#7-manufacturing-and-cost)) |

Tags worth knowing: `v1-archive` (the v1 hardware), `mono-renderer-final` (last two-OLED
renderer), `pre-collector-refactor`, `archive/ref-design-4layer`,
`ref-design-battery-under-board` (the case as first printed).

## 4. Decisions and their reasons

The decisions that shape the project, newest last. Each links to where it is argued in full.

| Date | Decision | Why | Recorded in |
|---|---|---|---|
| 2026-05-15 | Own Gen 1 engine instead of `battler` on the device | `battler` held about 13 KB per Pokemon and exhausted the RP2040's 128 KB heap in 3v3 battles; `gen1_battle` keeps a 6v6 battle in about 800 B of RAM with flash-resident tables | [crates/gen1_battle/SPEC.md](crates/gen1_battle/SPEC.md) |
| 2026-07-09 | All display and input semantics in core; platforms do raw IO only | The browser and the device had drifted (different screens, different input rules); structure, not discipline, keeps them identical | [CLAUDE.md](CLAUDE.md) |
| 2026-08-06 | One shared colour screen, cursor input, open information | The old scheme let a stray tap commit a move with no way back; an audit of the assembled v1 build (2026-08-05) also found cross-player ghost presses and the LED strips over the power budget | [09-single-screen.md](docs/architecture/09-single-screen.md) |
| 2026-08-06 | RP2350, not ESP32-S3 | ESP32-S3 would mean a new toolchain and a rewrite of the embassy-rp IO layer, and losing probe-rs, defmt and RTT; RP2350 has the RAM and flash | 09-single-screen.md section 7 |
| 2026-08-11 | The battle is one head-to-head scene across the middle | Drawing each mon in its own half staggered the pair; a shared band reads as one field | 09-single-screen.md section 3 |
| 2026-08-15 | Gen 3 as a sibling engine (`gen3_battle`), written in-house | Gen 1 stays untouched; the new crate is `no_std` and allocation-free on the hot path, which upstream `battler` is not | [crates/gen3_battle/src/lib.rs](crates/gen3_battle/src/lib.rs) |
| 2026-08-16 | Showdown is the reference; read its source before implementing, fuzz to confirm | The project's value is being verifiably the same as an accepted reference; guessing by probing cost repeated rework | [tools/showdown-parity/README.md](tools/showdown-parity/README.md) |
| 2026-08-19 | Stripboard v2: 16 switches straight to GPIO, no matrix | The v1 matrix ghosted presses across players; direct GPIO cannot | [hardware/stripboard_v2/BUILD.md](hardware/stripboard_v2/BUILD.md) |
| 2026-08-19 | 3.5 in 320x480 panel instead of the 2.8 in 240x320 in the redesign spec | The 3.5 in ST7796 module was what was bought; the UI was regrown to 320x480 native on 2026-08-27 | BUILD.md; firmware history |
| 2026-08-27 | The game loop (`DeviceRunner`) lives in core | The first single-screen web client kept lobby, battle wiring and pacing in web-only code, so the firmware had a UI with no battles behind it | [crates/mega_blastoise_core/src/device_runner.rs](crates/mega_blastoise_core/src/device_runner.rs) |
| 2026-09-17 | No radio; one USB-C; audio added | No radio keeps certification on the unintentional-radiator path; the stripboard had two USB ports and no sound | [10-production-board.md](docs/architecture/10-production-board.md) |
| 2026-09-21 | DS Lite silicone membranes on bare PCB contacts | Half the footprint of SNES parts (15 mm face-button diamond), sold as repair spares; one D-pad piece and one ABXY piece per seat | [hardware/membrane/README.md](hardware/membrane/README.md) |
| 2026-09-21 | Bare RP2350B family chip, not a module | No RP2350B module exists that JLCPCB will place; the Pico family is placeable but has too few GPIO | 10-production-board.md section 8 |
| 2026-09-21 | 3.5 in 320x480 IPS class | The only size with datasheet-backed, stocked IPS supply; panels hold their own frame memory, so no framebuffer is needed | 10-production-board.md section 9 |
| 2026-09-23 | The case owns every position; the board follows | One source of truth for outline, window and contact centres, imported into KiCad | [hardware/prod_case/README.md](hardware/prod_case/README.md) |
| 2026-09-24 | Reference design built by Claude in parallel with Alex's hand-made board | Alex keeps learning on his own board while a complete, orderable version exists | [hardware/ref_design/PLAN.md](hardware/ref_design/PLAN.md) |
| 2026-09-24 | 2-layer board with Alex's approved cost changes: TP4056 with load sharing, ME6211 LDO, RP2354B (flash inside), PWM audio into an FM8002A, no cover lens, every hole 0.3 mm or larger | Each change was costed and approved by Alex; for example the LDO saves $1.40 a unit against the buck-boost, and the bare board fell from $9.74 to $3.34 a unit at 10 | PLAN.md decisions table |
| 2026-09-24 | EastRising ER-TFT035IPS-6 panel in a ZIF connector | Published datasheet and 3D model, sold by its maker, plugs in instead of being soldered | PLAN.md; [docs/design_review.md](hardware/ref_design/docs/design_review.md) |
| 2026-09-25/26 | Keep a Schottky across the cell switch, a bleeder on VBUS, a series resistor on the USB input | Simulated unplug and hot-plug events dipped 3V3 under the flash's minimum or rang VSYS near the regulator's limit | design_review.md |
| 2026-10-07 | Discrete backlight current sinks replace the AP3156 | The AP3156 ran out at JLC and LCSC; the sinks use only Basic or Preferred parts and were simulated before layout | design_review.md; [board/sim/](hardware/ref_design/board/sim/) |
| 2026-10-09 | Cell under the board, on the tub floor | Alex's call. The alternative, thinner at the ends, put the cell over the MCU core, which the validation plan flagged for heat; it stays on branch `ref-design-cell-on-board` | PLAN.md |
| 2026-10-19 | Start and Select per seat; seat 1's Start is the power button | Matches the DS Lite layout; no game button doubles as power | PLAN.md |
| 2026-10-20 | Speaker on a plug-in header, playing out of the side wall | A floor grille faces the table; no soldering at assembly | PLAN.md |

## 5. Software architecture

### 5.1 Shape

One `no_std` core holds the battle engines and the whole game: menus, lobby, input rules, screen
decisions and rendering. Three thin platforms wrap it.

```
            gen1_battle   gen3_battle      (battle engines, no_std)
                   \         /
                 battle_types              (the type chart's alphabet)
                        |
               mega_blastoise_core         (game loop, input rules, screens, renderer)
              /         |          \
 mega_blastoise_fw  mega_blastoise_web  mega_blastoise_test
  (RP2350 device)    (browser, WASM)     (host CLI and tests)
```

| Crate | Role |
|---|---|
| [`gen1_battle`](crates/gen1_battle/SPEC.md) | Gen 1 singles engine, cartridge-accurate as Showdown models it, about 800 B of RAM per battle |
| `gen3_battle` | Gen 3 singles engine: Special split, natures, IVs and EVs, 17 types, weather, abilities, held items |
| `battle_types` | The type enum shared by both engines |
| `mega_blastoise_core` | Everything the device does, shared verbatim by the platforms |
| `mega_blastoise_fw` | Firmware for the stripboard v2 board (Embassy on RP2350) |
| `mega_blastoise_web` | The same device in a browser page; its own WASM workspace |
| `mega_blastoise_test` | Interactive stdin battle, unit and integration tests, the Showdown parity and fuzz suites, a renderer that dumps every screen to image files |

More detail: [docs/architecture/](docs/architecture/README.md) and the
"where does new code go" guide in [08-feature-map.md](docs/architecture/08-feature-map.md).
Toolchain: nightly Rust pinned in [rust-toolchain.toml](rust-toolchain.toml).

### 5.2 The battle engines

**History.** The project began on upstream [`battler`](https://github.com/jackson-nestelroad/battler),
an open-source Rust Pokemon engine, checked out as a git submodule (Alex's fork) at `battler/`. On
the RP2040 it ran out of heap, so `gen1_battle` was written as a drop-in replacement on
2026-05-15. `battler` is still a dependency: core's build script reads its Gen 1 data files and
the data store uses its types, but no battle runs through it.

**Gen 1** (`gen1_battle`, about 4,400 lines) models Gen 1 random battles as Showdown's Gen 1 mod
does, including the cartridge's glitches. Its [SPEC.md](crates/gen1_battle/SPEC.md) documents
memory layout, formulas, turn structure and the deliberate gaps. By 2026-08-16, 164 of its 165
moves were in the fuzz pool.

**Gen 3** (`gen3_battle`, about 11,400 lines) was added on 2026-08-15 as a sibling crate, so a Gen 1
battle never enters it. By 2026-08-16 it modelled 75 of the 76 Gen 3 abilities and the first 47
held items (more followed on 08-17), with weather, screens, multi-turn moves and the residual
order as Showdown runs them. Abilities and items are answered by small modules
([ability.rs](crates/gen3_battle/src/ability.rs), [item.rs](crates/gen3_battle/src/item.rs)) that
each pipeline stage asks, rather than an event bus.

**The reference.** Pokemon Showdown is the authority. Its source is installed locally by
`tools/showdown-parity` and read before a mechanic is implemented; the fuzzer then confirms. The
last engine commit (2026-08-19) widened the fuzz pool to every damaging move, found a backlog of
480 failing seeds in 600, and brought it to 5, which are not yet explained.

**On the device**, both engines speak one protocol to core: a request becomes generation-neutral
slot options for the input layer, and the chosen move or switch goes back as a command string.
Teams come from Showdown's random battle sets ([randbat data](https://pkmn.github.io/randbats/)
for Gen 1; the vendored Gen 3 sets for Gen 3). The device's Gen 3 draft does not yet hand out
abilities, held items or EV spreads ([draft.rs](crates/gen3_battle/src/draft.rs)), so device
battles do not yet exercise what the engine models.

### 5.3 The zero-drift rule

The firmware and the web client must be indistinguishable in everything a player sees and does.
This is the project's most important software property, and it is enforced by where code is
allowed to live ([CLAUDE.md](CLAUDE.md)):

- **Core decides.** What any screen shows, how any input behaves, and what the device does next
  are decided only in `mega_blastoise_core`.
- **Platforms only do raw IO**: classifying a press as a tap or a 500 ms hold, carrying typed
  console lines, presenting rendered frames, printing log lines, and supplying a monotonic clock,
  sleeps and entropy.
- **The device wins.** When the two targets disagree, the firmware on the Pico is the source of
  truth.

If a change needs battle UX behaviour written in firmware- or web-only code, it belongs in core
instead, driven from both platforms. The rule was written on 2026-07-09 after the two targets had
drifted, and extended on 2026-08-27 to orchestration after the first single-screen web client
turned out to hold the whole game loop.

Which core module owns what:

| Module | Owns |
|---|---|
| [`oled_ctl.rs`](crates/mega_blastoise_core/src/oled_ctl.rs) | `OledController` and the `Screen` enum: which screen each seat is on, flashes, narration timing, the redraw rule (the name dates from the two-OLED device) |
| [`choice_collect.rs`](crates/mega_blastoise_core/src/choice_collect.rs) | `ChoiceCollector`: prompts, accept and reject with messages, the waiting screen, take-back, the 1 s both-ready grace, invalid flashes, long-press detail views, the typed-line grammar |
| [`device_session.rs`](crates/mega_blastoise_core/src/device_session.rs) | Outside a turn: which menu owns which half, shared options, each seat's cursor and overlays, what a press means in each state |
| [`device_runner.rs`](crates/mega_blastoise_core/src/device_runner.rs) | The game loop (attract, lobby, ready sequence, battle, recap) and the `DevicePort` trait a platform implements |
| [`cursor_nav.rs`](crates/mega_blastoise_core/src/cursor_nav.rs), [`menu.rs`](crates/mega_blastoise_core/src/menu.rs), [`battle_log.rs`](crates/mega_blastoise_core/src/battle_log.rs) | D-pad cursor, Gen picker and options, the scrollable log |
| [`display_color.rs`](crates/mega_blastoise_core/src/display_color.rs), [`device_view.rs`](crates/mega_blastoise_core/src/device_view.rs), [`device_ui.rs`](crates/mega_blastoise_core/src/device_ui.rs), `move_anim.rs` | Rendering one half, composing two halves (far one rotated) into the 320x480 frame, the shared battle band and attack effects |
| [`cli_parse.rs`](crates/mega_blastoise_core/src/cli_parse.rs) | The console grammar, shared by the USB console and the web console |

### 5.4 How a turn flows

1. The engine writes battle log lines; core parses them into typed `BoardEvent`s (damage, faint,
   switch-in, status and so on) and turns the engine's request into a prompt.
2. `OledController` decides each seat's screen from those events; `DeviceRunner` paces narration.
3. The renderer draws each half from the `Screen` plus per-seat context (foe HUD, cursor) and
   composes the 320x480 frame. The platform's `present` hook only moves pixels: the firmware
   sends the 16x16 tiles that changed; the web page blits the frame to a canvas.
4. A press comes in from the platform as a classified `DeviceEvent`. `DeviceSession` and
   `CursorNav` turn it into a pad event; `ChoiceCollector` validates it and, once both seats are
   ready and the grace window has passed, hands the engine its command strings.

### 5.5 Data and assets

Core's [build script](crates/mega_blastoise_core/build.rs) generates static tables at compile
time, so nothing is loaded at run time and there is no file system: Gen 1 data from `battler`'s
data files, the random battle roster, move descriptions from Showdown's text data, 1-bit sprites
for the old mono renderer, and colour sprites for every Gen 3 species (Emerald art from the
PokeAPI sprite repository, packed losslessly as 4 bits per pixel with a per-species palette).
Downloads are cached under `crates/mega_blastoise_core/vendor/` so later builds are offline.
Gen 3 art is used for both generations. Move icons for the attack effects come from
[scripts/gen_move_sprites.py](scripts/gen_move_sprites.py).

### 5.6 Firmware

`mega_blastoise_fw` runs on the Pico 2 in the stripboard v2 board. A plain `cargo build` in the
crate builds the device; [03-firmware-runtime.md](docs/architecture/03-firmware-runtime.md) has
the startup and task list. In short: Embassy async tasks for the ST7796 panel (PIO SPI with DMA),
the 16 buttons (debounce, A tap versus hold), the USB CDC console, battery sensing and the sprite
bob, and [`board_v2.rs`](crates/mega_blastoise_fw/src/board_v2.rs) implements core's
`DevicePort` and runs `DeviceRunner` forever. A panic lights the Pico's GP25 LED and its message
survives a reset. Features: `input-debug` (held-key overlay), `mem-profile`, `trace`.

There is no firmware yet for the reference design: it needs the `rp235xb` chip feature, its pin
map, an ILI9488 panel driver (18-bit colour over SPI, unlike the ST7796), audio and power control.
Bring-up tools (an RP2350 blink and a recovery-reset helper) are in
[firmware/bringup/](firmware/bringup/rp2350_blink/README.md).

### 5.7 Web build and site

`mega_blastoise_web` compiles core to WASM. [`src/lib.rs`](crates/mega_blastoise_web/src/lib.rs)
is the web's `DevicePort` plus exports that turn pointer and keyboard input into `DeviceEvent`s;
[`www/device.html`](crates/mega_blastoise_web/www/device.html) draws a DS Lite style console with
both seats in one window and an in-page console. URL parameters such as `?demo=1`, `?ai=1` and
`?seed=N` drive it for testing.

[`.github/workflows/pages.yml`](.github/workflows/pages.yml) builds and deploys on every push to
`main`:

| Path on Pages | What |
|---|---|
| `/` | Front page, from [site/index.html](site/index.html) |
| `/battler/device.html` | The game |
| `/battler/ui_flow.html` | The single-screen UI design notes |
| `/assembly/` | Exploded 3D view of the reference design, built locally by [hardware/ref_design/viewer/](hardware/ref_design/viewer/) and kept fresh by a pre-push hook |
| `/assembly-cell-on-board/` | The same view of the earlier cell-on-board layout |

### 5.8 Host harness

`cargo run -p mega-blastoise-test` plays a battle over stdin through the same pipeline as the
device. `cargo run -p mega-blastoise-test --bin screens -- <dir>` renders every screen to PPM
files with no hardware or browser.

## 6. Hardware

### 6.1 The generations compared

| | v1 PCB | Stripboard v2 | Reference design (REF-A) |
|---|---|---|---|
| MCU | Pico (RP2040) | Pico 2 (RP2350A, 4 MB flash) | RP2354B on the board (2 MB flash inside) |
| Display | Two 1.3 in 128x64 mono OLEDs | One 3.5 in ST7796 320x480 module | One 3.5 in ER-TFT035IPS-6 IPS panel, ILI9488, in a ZIF connector |
| Inputs | Button matrix, move buttons at screen corners | 16 tactile switches, direct GPIO, 9 mm printed caps | 20 membrane contacts under genuine-style DS Lite silicone and caps |
| Per seat | 4 moves, 3 party buttons | D-pad cross and A, B, INFO, MENU cross | D-pad, A/B/X/Y, Start, Select |
| Feedback | LED strips for HP | Screen only | Screen and a speaker |
| Power | 2xAA in the final case | 2000 mAh LiPo, TP4056 module, slide switch | About 700 mAh Li-ion cell, on-board charger with USB load sharing, soft power |
| Built | Yes | Yes | No |

### 6.2 Stripboard v2: the board that runs today

One 24x56 stripboard (about 61 x 142 mm) carries the Pico 2, the panel standing portrait on a
socket in the middle, and a band of switches at each end: a D-pad cross on the player's left and
an action cross on their right. Every switch has its own GPIO with the internal pull-up, so there
is no ghosting. A protected TP4056 module charges a 103450 LiPo mounted on the back; a Schottky
diode lets the charger's USB and the Pico's USB be plugged in together, and a divider lets the
firmware read the cell. There is no buzzer.

Everything physical is generated by `layout.py`: the build guide, the BOM, the KiCad schematic and
board, and solid models. The guide is [BUILD.md](hardware/stripboard_v2/BUILD.md) (its section 10
is the firmware pin map), the staged build with a test after each stage is
[ASSEMBLY.md](hardware/stripboard_v2/ASSEMBLY.md), and the models are in
[model/](hardware/stripboard_v2/model/README.md). The cluster spacing is cramped (about 30 mm),
which Alex accepted for a prototype.

### 6.3 Requirements for a sellable board

[10-production-board.md](docs/architecture/10-production-board.md) (2026-09-17 to 09-21) records
what changes when the device becomes a product: no 2.54 mm grid or stripboard outline, ergonomics
re-decided from scratch, membranes instead of tactile switches, a bare MCU placed by the fab, a
proper charger and protected cell, one USB-C for charge and data, no radio, an IPS panel, and
audio for the first time. Several of its first choices (a power-path charger IC, a buck-boost
regulator, an I2S amplifier, an anti-glare cover lens) were later replaced in the reference design
by cheaper parts on 2026-09-24; the reference design's [PLAN.md](hardware/ref_design/PLAN.md) is
the current word.

### 6.4 Alex's production board

`hardware/prod_board/` and [`hardware/prod_case/`](hardware/prod_case/README.md) are Alex's own
learning project in KiCad and OpenSCAD. The case's `layout.scad` owns the outline, glass window
and every contact centre, and a script places the board's 16 contacts from it. On `main` the board
has its outline and 16 contacts placed; the schematic is not started. This folder is Alex's:
other work reads it and never edits it.

### 6.5 The reference design (REF-A)

A complete design package to order a first prototype: schematic, routed 2-layer board, JLCPCB
fab and assembly files, printable case, fit kit, and documents to build, bring up and review it.
It is generated: [`board/design.py`](hardware/ref_design/board/design.py) holds every part, value,
LCSC number and net; [`board/placement.py`](hardware/ref_design/board/placement.py) places them;
[`case/case.scad`](hardware/ref_design/case/case.scad) is the source of every mechanical position.
Scripts build the schematic, board, routing, checks, fab outputs and case from those three files
(the pipeline is in the [README](hardware/ref_design/README.md)). Never edit the generated KiCad
files by hand.

Where to read more: [PLAN.md](hardware/ref_design/PLAN.md) (decisions and to-do),
[design_review.md](hardware/ref_design/docs/design_review.md) (each block and why, what was and
was not checked), [pinmap.md](hardware/ref_design/docs/pinmap.md),
[bom.md](hardware/ref_design/docs/bom.md), [assembly.md](hardware/ref_design/docs/assembly.md),
[bringup.md](hardware/ref_design/docs/bringup.md),
[validation_plan.md](hardware/ref_design/docs/validation_plan.md),
[konnect_notes.md](hardware/ref_design/docs/konnect_notes.md), and
[datasheets/SOURCES.md](hardware/ref_design/datasheets/SOURCES.md).

**Power and battery.** One 403450 Li-ion cell (about 700 mAh) lies on the tub's floor under the
board and plugs in on a Molex PicoBlade connector. A TP4056 charges it from USB-C while USB also
feeds the system directly (load sharing), with a board thermistor over the cell limiting charging
to a safe temperature window. An ME6211 LDO makes 3V3; because the ADC's reference is the 3V3 rail,
the firmware must shut down at a computed 3.5 V. Power is a latch: pressing seat 1's Start
connects the battery to the regulator, the MCU then holds itself on, and releasing the hold turns
the device off. A reset or crash drops the hold, so on battery a crash turns the device off, which
is the intended safe failure. Off-state drain is about 20 uA. The unplug hand-over and hot-plug
behaviour were simulated, not yet measured. Battery life has not been measured.

**MCU.** The RP2354B is the RP2350B die with 2 MB of flash stacked inside, so there is no flash
chip to route. Its core circuit and layout, including the copper around the internal 1.1 V
switcher, are copied from Raspberry Pi's RP2350B minimal design (MIT licence), so the most
layout-sensitive part of the board is theirs. Pins are assigned so each faces where its net goes,
which is what lets two layers route completely.

**Display and backlight.** The 3.5 in 320x480 IPS panel's FPC folds under it into a bottom-contact
ZIF connector that JLC places, so no hand soldering and a cracked panel swaps in a minute. Its tear
signal goes to a GPIO. Its six backlight LEDs are driven by two op-amp current sinks set by a
filtered PWM, so the LED current is DC and stays constant down to the cutoff. The case is built
around a fixed panel bay, so another panel of the same glass class changes only the panel numbers,
the connector and its pin map.

**Audio.** PWM from a GPIO through a two-stage RC filter into an FM8002A amplifier and a 15 x 10 mm
speaker, which plugs into a header and plays out of the case's left wall. The target is GBA-like
sound. A mixer, when written, lives in core so the browser sounds like the device.

**Buttons.** 20 membrane contacts (two interleaved gold combs each) under DS Lite silicone and caps:
per seat a D-pad, A/B/X/Y, Start and Select, each on its own GPIO with no matrix. The contact
footprints are generated and checked by the tools in [hardware/membrane/](hardware/membrane/README.md).

**Debug.** SWD and a UART on JST SH connectors in the Raspberry Pi Debug Probe pinout, SWD again on
six pogo pads on the back, BOOTSEL and RESET buttons under pinholes in the lid, probe pads on every
rail and on key signals, cut jumpers to isolate each power block, and revision-strap resistors.
No LEDs (an approved decision).

**Board.** 2 layers, 1.6 mm, ENIG (the silicone pills press straight on the combs), every hole
0.3 mm or larger to stay on JLC's standard process, assembled on one side.

**Case and printing.** About 75 x 156 mm, 13.4 mm thick at the screen and 11.1 mm at the button
decks: close to a DS Lite's lower half in width (73.9 mm) and thickness (about 13.5 mm), and
longer than it (133 mm). A tub, a lid with the panel in a pocket, and one deck
insert printed twice, held by six M2 screws as on a DS Lite, with four hooks holding the lid while
it is screwed. The shell follows the DS Lite: rounded back, 4.3 mm plan corners, the seam on the
face. Genuine-pattern DS Lite caps stand 1.3 mm proud, using numbers measured from open models
([case/DSLITE.md](hardware/ref_design/case/DSLITE.md)). Parts print without supports where
possible, in PLA, on Alex's Snapmaker U1 (a Bambu P1S package exists too):
[print_package_README.md](hardware/ref_design/case/print_package_README.md). A printable
[fit kit](hardware/ref_design/case/fitkit/README.md) (board mock, part dummies, coupons) tests the
real parts before boards are ordered. The case modelling method (render, cross-section and
assert, headless) is in [docs/hardware/openscad-methodology.md](docs/hardware/openscad-methodology.md).

### 6.6 Membranes and the DS Lite research

The membrane decision is documented in [hardware/membrane/README.md](hardware/membrane/README.md):
the donors surveyed (SNES, DS Lite, others), the DS Lite geometry measured from a breakout board's
gerbers, the contact pattern, licence traps in other projects' footprints, and an open question:
fine gold combs need ENIG or hard gold, while carbon ink (more durable) needs a coarser pattern.

[hardware/ds_lite/](hardware/ds_lite/README.md) is a research packet on how the DS Lite is built and
why it feels as it does, every number sourced and labelled by how it was obtained. Its key points
for this project: density (a DS Lite works out to about 1.03 g/cm3, so about 160 g for a case this
size), a 13.5 mm lower half, rounded edges, and the seam on the face. Its recommendations are
inputs, not decisions. Alex owns a genuine DS Lite for scale and caliper checks; the dossier's
"Measure your own unit" section lists them.

## 7. Manufacturing and cost

**Route.** JLCPCB makes the 2-layer board and assembles one side (Economic PCBA). The panel,
cell, speaker, DS Lite silicone and caps, screws and tape are bought separately and fitted by hand.
The case is printed at home on FDM. [assembly.md](hardware/ref_design/docs/assembly.md) is the
order and build procedure; [tools/quotes/](hardware/ref_design/tools/quotes/README.md) re-runs
JLCPCB's and PCBWay's instant-quote forms headlessly when the design changes.

**Cost, with the date of each figure:**

| Figure | Per unit | Date | Source |
|---|---|---|---|
| v1 parts, excluding enclosure | about $40 | spring 2026 | [ELECTRONICS.md](docs/archive/v1/ELECTRONICS.md) |
| Stripboard v2 parts, bought in hobby packs (one unit) | about $125 | 2026-08 | [BUILD.md](hardware/stripboard_v2/BUILD.md) |
| Reference design bare board, 4-layer with 0.25 mm holes, at 10 | $9.74 | 2026-09-24 | quotes |
| Reference design bare board, 2-layer, at 10 | $3.34 | 2026-09-24 | quotes |
| Reference design, factory part (board, parts, assembly fees, shipping) at 10 | about $16.47 | 2026-09-24, parts repriced 2026-10-20 | [bom.md](hardware/ref_design/docs/bom.md) |
| Reference design, all in, at 10 | about $39-40 | same | bom.md |
| **Reference design, latest, at 10** | **$42.52** | **2026-10-21** | BOM audit |
| **Reference design, latest, at 50** | **$27.98** | **2026-10-21** | BOM audit |

The bom.md figures exclude import duties (not yet known), JLC's new-account coupons and hand
labour. The panel is the
largest single item (about $14 at 10, under $10 at 50); the case is about $1.50 of filament.

**Supply risks.** The 2026-10-21 BOM audit found the RP2354B and its core inductor (L2) out of
stock at JLC and LCSC. **That blocks ordering** until stock returns or an alternative is designed
in. Stock has bitten before: on 2026-10-07 both the AP3156 backlight driver (replaced by discrete
sinks) and the first USB ESD array (replaced by another maker's part) ran out. The panel has a
single maker; its glass class has other sources that would need only case and pin-map changes.

**Order plan.** The validation plan proposes 3 to 5 assembled boards plus 5 bare ones from the same
order (for sectioning, rework practice and the contact test), and a dry run of JLC's order flow
with screenshots of every polarised part before paying.

## 8. IP and regulatory stance

Decided so far (2026-09-17, [10-production-board.md](docs/architecture/10-production-board.md)
section 6):

- **Rename before selling.** "Mega Blastoise" is a Pokemon name; that includes the USB
  manufacturer string the firmware reports.
- **No Pokemon markings on the device.**
- **No preloaded firmware.** Units ship blank and the owner downloads the game, which keeps
  species names, sprites and move text off the product. Today's builds fetch those assets at
  compile time (section 5.5) and the web build publishes them; how a buyer would get them is open.
- **Real USB IDs.** The firmware uses a placeholder VID:PID; Raspberry Pi allocates product IDs
  under its vendor ID for products on its silicon.
- **Engines.** `gen1_battle` and `gen3_battle` are written in-house from the published mechanics
  and Showdown's behaviour; `gen3_battle` takes no code from `battler`.
- **Hardware sources.** The RP2350B core copy is MIT-licensed (its licence is in
  `hardware/ref_design/datasheets/`). The vendored membrane footprints are public domain. DS Lite
  cap numbers come from open models, with no geometry copied. Licence traps (share-alike and
  non-commercial footprints and models) are listed in the membrane README and the DS Lite dossier.
- **DS Lite look.** The US design patent (D554,194) expired in 2021; an EU registered design, if
  one was filed and renewed, could last until 2031, so an EUIPO search is due before selling
  anything that looks like a DS Lite.
- **Certification.** FCC Part 15B and CE as an unintentional radiator (no radio keeps it there),
  UN38.3 test documents from the cell supplier, and marketing to adults (a children's product
  triggers CPSIA testing in the US).

The validation plan defers a full IP audit, regulatory marks and EMC testing until a sale is in
view. The repository itself has no top-level licence file.

## 9. Verification and tooling

### Software

- **The gate:** [scripts/check.sh](scripts/check.sh) runs before every code commit: all host
  tests, a firmware `cargo check` for its real target, the WASM build, and the headless UI probe
  (`--fast` skips the last two). It exists because two test binaries once rotted silently.
- **Showdown parity:** [tools/showdown-parity/](tools/showdown-parity/README.md) runs the real
  Showdown simulator under Node. `showdown_parity` compares hand-written scenarios and
  `showdown_fuzz` compares seeded random ones (`FUZZ_N`, `FUZZ_SEED`, `FUZZ_ONLY`), with hit,
  crit and damage roll scripted on both sides so every branch is compared. Unarmed checkouts skip
  these suites loudly. Engine changes get a 2000-seed sweep before they are pushed.
- **Headless UI probe:** [scripts/ui-probe.sh](scripts/ui-probe.sh) boots the web page in headless
  Chromium on a pinned seed, plays real turns in both generations through the same exports the
  touch controls use, and asserts on a per-seat state trace. It catches wiring bugs that unit
  tests cannot see.
- **Screens:** the `screens` binary renders every UI state to image files on a host.
- **Device debugging:** [scripts/mb-console.py](scripts/mb-console.py) merges RTT and USB console
  output and forwards typed lines; [scripts/fw-build.sh](scripts/fw-build.sh) reports flash and RAM
  use; `scripts/commands.sh` defines shell helpers; `.claude/commands/fwdebug.md` is the firmware
  debug loop for Claude Code.

### Hardware

- **Reference design checks**, all scripted ([design_review.md](hardware/ref_design/docs/design_review.md),
  "Checks run"): every assembled part's every pin reviewed against its current datasheet
  (`check_pins.py`), schematic netlist equal to `design.py`, ERC and DRC clean with schematic
  parity, hole sizes, the membrane contact pattern, every part's 3D model inside the case
  (`check_fit.py`), the button stacks under the shell (`check_buttons.py`), and an ngspice
  simulation of the backlight sinks.
- **Physical checks so far:** fit-test prints (first on 2026-10-05, again on 2026-10-19), Alex's
  caliper measurements of the DS Lite silicone pads (2026-10-20), and the stripboard's per-stage
  bring-up testers.
- **Validation plan:** [validation_plan.md](hardware/ref_design/docs/validation_plan.md) (2026-10-05)
  is a joint plan from two models' cross-examination, with a 13-item go/no-go gate for ordering:
  datasheet and errata tables, real panel lit through a breakout, contact resistance on a test
  coupon, a latch replica, measured stack heights, a JLC dry run, an outside review, and live stock
  on order day. Several of its board changes are already in `design.py`; the physical items are
  still listed as open in PLAN.md.
- **Bring-up:** [bringup.md](hardware/ref_design/docs/bringup.md) is the step-by-step first power
  of a new board, ending with the USB hand-over and power switch.

### Working practices

- Generated files are never hand-edited: the stripboard from `layout.py`, the reference design
  from `design.py`, `placement.py` and `case.scad`, the pin map from `pinmap.py`.
- For engine work, read Showdown's source first, then fuzz to confirm.
- For case work, render and assert headless before printing.

## 10. Status, known gaps and open decisions

### Status on 2026-10-21

| Area | Status |
|---|---|
| Gen 1 engine | 164 of the 165 moves in its random battle pool under parity and fuzz tests |
| Gen 3 engine | Moves, abilities and held items modelled; 5 of 600 seeds in the widest fuzz sweep unexplained (2026-08-19) |
| Web build | Live on Pages; both generations playable |
| Firmware | Runs the full game on the stripboard v2 |
| Stripboard v2 | Built and working |
| Alex's prod_board | Contacts placed; schematic not started |
| Reference design | Designed, routed and checked; fit-test prints done; not ordered |
| Ordering | Blocked: RP2354B and L2 out of stock (2026-10-21) |

### Known gaps

- No firmware for the reference design (RP2354B pin map, ILI9488 driver, audio, power latch,
  charger status, battery cutoff).
- No sound anywhere yet: no mixer in core and no web audio. The options menu dropped its sound and
  turn-timer rows because nothing was behind them.
- No turn timer, so a stalled player can still hold up a game.
- Start and Select have no meaning in core; MENU acts as B.
- The device's Gen 3 draft gives no abilities, items or EV spreads.
- A team reveal screen and sprite compression for flash are not built.
- D-pad auto-repeat exists only in the web page's input code; the firmware does not repeat yet.
- Whether the firmware and its assets fit the RP2354B's 2 MB of flash has not been measured.
- No persistent battle log in flash for post-mortems (a long-standing to-do).
- The reference design has no LED: there is no visible error indicator beyond the screen, and a
  player's only reset is power-cycling (RESET sits under a pinhole).

### Open decisions and to-dos

From [PLAN.md](hardware/ref_design/PLAN.md) and the validation plan:

- **Before ordering:** a source for the RP2354B and L2, or an alternative; the measurements in
  PLAN.md's "Measure before ordering" (the Start/Select pad, the D-pad collar, whether dome tops
  take the cap's stem, the real panel against its drawing and the FPC's reach into the connector,
  the cell's length and plug polarity, the printed snap fit, a sample speaker plug in a genuine
  header); the go/no-go gate.
- **Buttons:** the A/B/X/Y stems sit on the domes at rest, the D-pad's pivot needs modelling on
  its centre collar, and the Start/Select wells are estimated; `check_buttons.py` fails until these
  settle.
- **Case:** narrow it 1 mm a side once a real silicone piece measures 26 mm or less; the mismatched
  end bezels (tabled); whether to adopt the DS Lite dossier's recommendations (mass, edges).
- **Disputed in the validation plan:** 3 or 5 assembled boards; whether a latch replica gates the
  order; a hardware long-press kill footprint.
- **Contact finish:** ENIG combs (chosen) may drift over a product's life; carbon ink needs a
  coarser pattern.
- **Enclosure at volume:** printing serves prototypes; 10-production-board.md names injection
  moulding for volume, and nothing is planned yet.
- **Product:** the new name, USB IDs, how buyers get the firmware and assets, the EUIPO check.

### Unknown

- Battery life of the reference design: not measured or estimated in the repo.
- The finished unit's mass against the about 160 g target: not measured.

## 11. Where things live

| Path | What |
|---|---|
| [README.md](README.md) | Build and run instructions |
| [CLAUDE.md](CLAUDE.md) | The zero-drift rule and project rules for Claude Code |
| [docs/architecture/](docs/architecture/README.md) | Software architecture pages; [09](docs/architecture/09-single-screen.md) single-screen design, [10](docs/architecture/10-production-board.md) production board |
| [docs/TECHNICAL.md](docs/TECHNICAL.md) | Software internals overview |
| [docs/archive/v1/](docs/archive/v1/DESIGN.md) | The v1 device: design, electronics, firmware interactions, original plan |
| [docs/hardware/openscad-methodology.md](docs/hardware/openscad-methodology.md) | How the cases are modelled and checked |
| `crates/gen1_battle/`, `crates/gen3_battle/`, `crates/battle_types/` | Battle engines; [Gen 1 spec](crates/gen1_battle/SPEC.md) |
| `crates/mega_blastoise_core/` | The shared game: loop, input, screens, renderer, generated data |
| `crates/mega_blastoise_fw/` | Firmware for the stripboard v2 |
| `crates/mega_blastoise_web/` | Browser build; `www/` holds the pages |
| `crates/mega_blastoise_test/` | Host CLI, tests, parity and fuzz suites, screen dumper |
| `battler/` | Git submodule: the upstream engine, now a data source |
| `patches/once_cell/` | A patched dependency for `no_std` targets |
| [tools/showdown-parity/](tools/showdown-parity/README.md) | Node harness around the real Showdown |
| `scripts/` | The gate, UI probe, console, build and publish helpers |
| `firmware/bringup/` | Standalone bring-up crates |
| `site/` | The Pages front page |
| [hardware/README.md](hardware/README.md) | Hardware index |
| [hardware/stripboard_v2/](hardware/stripboard_v2/BUILD.md) | The built board: generator, build guide, assembly stages, models |
| [hardware/ref_design/](hardware/ref_design/README.md) | The reference design: `board/`, `case/`, `docs/`, `datasheets/`, `tools/quotes/`, `viewer/` |
| `hardware/prod_board/`, [hardware/prod_case/](hardware/prod_case/README.md) | Alex's own board and case (read only for others) |
| [hardware/membrane/](hardware/membrane/README.md) | Membrane contact footprints and their generator and checker |
| [hardware/ds_lite/](hardware/ds_lite/README.md) | DS Lite construction research |

## 12. Glossary

| Term | Meaning |
|---|---|
| Seat | One player's end of the device: seat 1 and seat 2 (P1 and P2 in code) |
| Half | One seat's part of the panel; the far half is drawn rotated 180 degrees |
| Zero-drift rule | Firmware and web must look and behave identically; all decisions live in core |
| Core | `mega_blastoise_core`, the `no_std` crate shared by every platform |
| Platform | The firmware, web or host crate: raw IO around core |
| `BoardEvent` | A typed battle event parsed from an engine's log |
| `OledController` / `Screen` | Core's per-seat screen state machine and its list of screens |
| `ChoiceCollector` | Core's battle-input state machine |
| `DeviceSession` | Core's state outside a turn: menus, cursors, overlays |
| `DeviceRunner` / `DevicePort` | Core's game loop, and the trait a platform implements for it |
| Showdown | Pokemon Showdown, the open-source battle simulator used as the reference |
| Parity / fuzz | Differential tests of the engines against Showdown: fixed scenarios and seeded random ones |
| Randbat | A random battle team drawn from Showdown's curated sets |
| Attract demo | The AI-versus-AI battle the device plays while idle |
| REF-A | The reference design in `hardware/ref_design/` |
| RP2354B | Raspberry Pi's RP2350B microcontroller with 2 MB of flash in the package |
| FPC / ZIF | The panel's flexible cable, and the zero-insertion-force connector it plugs into |
| Membrane, pill, comb | The silicone button pad, the carbon pill under each dome, and the interleaved copper contact it bridges |
| ENIG | A gold-over-nickel PCB finish; needed because the pills press on bare copper |
| Load sharing | USB powers the device directly while the charger fills the cell |
| Latch / PWR_HOLD | The soft power circuit: a button press powers up, the MCU holds itself on |
| Deck insert | The printed part carrying one end's buttons, the same part at both ends |
| Fit kit | Printed board mock, part dummies and case coupons to test real parts before ordering |
| JLC / LCSC | JLCPCB (board fab and assembly) and its parts distributor |
| Basic / Preferred / Extended | JLC part classes; Extended parts add a per-order loading fee |
| UN38.3 | The transport test standard for lithium cells |
