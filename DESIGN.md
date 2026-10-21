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
7. [The reference design (REF-A)](#7-the-reference-design-ref-a)
8. [Manufacturing and cost](#8-manufacturing-and-cost)
9. [IP and regulatory stance](#9-ip-and-regulatory-stance)
10. [Verification and tooling](#10-verification-and-tooling)
11. [Status, known gaps and open decisions](#11-status-known-gaps-and-open-decisions)
12. [Where things live](#12-where-things-live)
13. [Glossary](#13-glossary)

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
| 2026-10-21 | BOM audit: per-unit cost updated, and the RP2354B and its core inductor found out of stock (see [section 8](#8-manufacturing-and-cost)) |

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

The reference design now has a section of its own: [section 7](#7-the-reference-design-ref-a).

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

## 7. The reference design (REF-A)

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

Sections 7.1 and 7.2 summarise the design and its case. Sections 7.3 to 7.12 walk through the
schematic sheet by sheet, in the schematic's own order. Where design.py and another document
disagree, this section follows design.py.

### 7.1 Overview

**Power and battery.** One 403450 Li-ion cell (about 700 mAh) lies on the tub's floor under the
board and plugs in on a Molex PicoBlade connector. A TP4056 charges it from USB-C while USB also
feeds the system directly (load sharing), with a board thermistor over the cell limiting charging
to a safe temperature window. An ME6211 LDO makes 3V3; because the ADC's reference is the 3V3 rail,
the firmware must shut down at a computed 3.5 V. Power is a latch: pressing seat 1's Start
turns the regulator on, the MCU then holds itself on, and releasing the hold turns the device
off. A reset or crash drops the hold, so on battery a crash turns the device off, which is the
intended safe failure. Off-state drain is about 20 uA. The unplug hand-over and hot-plug
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

### 7.2 Case and printing

About 75 x 156 mm, 13.4 mm thick at the screen and 11.1 mm at the button
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

### 7.3 The schematic: how the sheets connect

The schematic is a root sheet and eight sub-sheets in KiCad 10's format. Since 2026-10-20
[`board/tools/build_sch.py`](hardware/ref_design/board/tools/build_sch.py) writes them directly
from design.py (before that they went through Konnect), and
[`sch_sheets.py`](hardware/ref_design/board/tools/sch_sheets.py) lays each sheet out by hand as
functional blocks: parts wired to each other, decoupling beside its pins, series parts in a chain.
design.py decides how each net is drawn: a power rail gets a power symbol, a net used on more than
one sheet gets a global label, and a net used on one sheet only gets a local label (KiCad prefixes
it with the sheet's name). The drawing is traced and compared with design.py before it is
written; afterwards `check_netlist.py` compares KiCad's own netlist with design.py, and KiCad's
ERC runs.

In numbers: design.py has 173 reference designators, of which JLC places 118. The other 55 are
copper features or empty footprints: the 20 contacts, 16 test pads, 5 solder jumpers, J7's pads,
6 holes, 3 fiducials, and 4 unfitted footprints (C41-C43, R48). Its 101 named nets are 6 power
rails, 48 global labels and 47 local nets. Eight `#FLG` power flags are not parts: they tell ERC
that a rail fed through a jumper, diode or FET is driven.

The sheets, with links to their walk-throughs:

| Sheet (file) | Title block | Walk-through |
|---|---|---|
| root (`battler.kicad_sch`) | Sheets and mechanical parts | [7.4](#74-sheets-and-mechanical-parts-root-sheet) |
| USB_Charger (`usb_charger.kicad_sch`) | USB-C, charger, load sharing, cell connector | [7.5](#75-usb-c-charger-load-sharing-and-cell-connector-sheet-usb_charger) |
| Power (`power.kicad_sch`) | 3V3 regulator, power latch, cell voltage sense | [7.6](#76-3v3-regulator-power-latch-and-cell-voltage-sense-sheet-power) |
| MCU (`mcu.kicad_sch`) | RP2354B | [7.7](#77-rp2354b-sheet-mcu) |
| MCU_Power (`mcu_power.kicad_sch`) | RP2354B supply: decoupling and core regulator | [7.8](#78-decoupling-and-the-11-v-core-switcher-sheet-mcu_power) |
| Debug (`debug.kicad_sch`) | SWD, UART, debug pads, BOOTSEL and RESET | [7.9](#79-swd-uart-debug-pads-bootsel-and-reset-sheet-debug) |
| Display (`display.kicad_sch`) | Panel connector and backlight sinks | [7.10](#710-panel-connector-and-backlight-sinks-sheet-display) |
| Audio (`audio.kicad_sch`) | PWM audio and amplifier | [7.11](#711-pwm-audio-and-amplifier-sheet-audio) |
| Buttons (`buttons.kicad_sch`) | Membrane contacts | [7.12](#712-membrane-contacts-sheet-buttons) |

**Power rails.** Six nets get power symbols. VSYS and VBAT use KiCad's VCC symbol renamed through
its value.

| Rail | Made by | Level | Used by |
|---|---|---|---|
| VBUS | the USB-C receptacle J1 | 5 V from the source; a simulated hot plug peaks at 6.7 V | the charger U2 and its TEMP and CE networks, D5 and D6 (load sharing), the bleeder R38, and R14 (USB holds the device on) |
| VSYS | D5 from VBUS, or Q2 and D7 from the cell | about 4.6 V on USB, the cell's voltage otherwise | the 3V3 regulator U3, the backlight anode (through JP2), the amplifier (through JP3), the power contact SW21 |
| VBAT | the cell, through J2 and JP5; U2 charges it | 4.2 V full; the firmware shuts down at a computed 3.50 V | Q2, D7, the divider R17/R18 |
| +3V3 | U3, through JP1, only while PWR_EN is high | 3.3 V | U4, the panel's logic (through JP4), the backlight op-amp U6, U1's reference, the pull-ups, J7 |
| +1V1 | U4's internal switcher, through L2 | 1.1 V | U4's core (DVDD) only |
| GND | | | every sub-sheet; poured on both layers and stitched by vias on a 5 mm grid |

VSYS is never switched: with a cell in, it is always live. "Off" means U3 is disabled, so +3V3
and +1V1 are gone. The loads that sit on VSYS directly are kept off by their own circuits: the
backlight sinks run from 3V3 (section 7.10) and the amplifier is held in shutdown (section 7.11).

```
 J1 USB-C --- VBUS --+-- R47 -- D5 ----------------------+-- VSYS
                     |                                   |
                     +-- U2 TP4056 -- VBAT -- Q2 and D7 -+
                                       |
 J2 cell --- JP5 ----------------------+

 VSYS ---+-- U3 ME6211, enabled by PWR_EN -- 3V3_LDO -- JP1 -- +3V3
         +-- JP2 -- LED_A: the panel's six backlight LEDs
         +-- JP3 -- AMP_VDD: the FM8002A amplifier
         +-- SW21: seat 1's Start contact, the power button

 +3V3 ---+-- U4 RP2354B supply pins -- U4's switcher -- L2 -- +1V1 (core)
         +-- JP4 -- LCD_3V3: the panel's logic
         +-- U6: the backlight sinks' op-amp
         +-- U1's reference, the pull-ups R9, R10, R16, R23, J7's 3V3 pad
```

Each sub-rail behind a solder jumper (3V3_LDO, LED_A, AMP_VDD, LCD_3V3, CELL_P) is a local net.
Cutting one of JP1 to JP5 isolates that block or puts a meter in series with it.

**Signals between sheets.** Every global label but one runs between the MCU sheet and one other
sheet; the exception is PWR_BTN_RAW, from the power contact on the Buttons sheet to the latch on
the Power sheet.

```
                     Debug      MCU_Power
                         \       /
      USB_Charger ------- MCU (U4) ------- Display
                         /   |    \
                    Power  Audio   Buttons
                      |               |
                      +- PWR_BTN_RAW -+
```

| Between MCU and | Global labels | What they carry |
|---|---|---|
| USB_Charger | USB_DP, USB_DM | USB data (the series resistors are on the MCU sheet, the ESD array on USB_Charger) |
| | CC1_SENSE, CC2_SENSE | the USB-C CC lines through 10k, into ADC6 and ADC5 |
| | CHG_STAT_N, CHG_DONE_N, CHG_OFF | the charger's two status outputs, and the MCU's charge pause |
| Power | PWR_HOLD, PWR_BTN_N, VBAT_SENSE | the MCU's hold on the latch, the power button as the MCU sees it, the cell voltage halved |
| MCU_Power | VREG_AVDD, VREG_LX | the core switcher's filtered analogue supply and its inductor node |
| Debug | SWCLK, SWDIO, RUN, QSPI_SS, UART_TX, UART_RX | SWD, reset, the BOOTSEL strap, the UART console |
| Display | LCD_SCK_MCU, LCD_MOSI_MCU, LCD_MISO, LCD_CS, LCD_DC, LCD_RST, LCD_TE, BL_PWM | the panel's SPI bus and control lines, and the backlight's set point |
| Audio | AUDIO_PWM, AMP_EN | the audio PWM, and the amplifier's enable |
| Buttons | P1_UP to P1_SELECT (9), P2_UP to P2_START (10) | one net per contact; seat 1's Start is PWR_BTN_RAW instead |

SCK and MOSI change name at their series resistors on the Display sheet: `_MCU` on the chip's side,
plain on the panel's.

### 7.4 Sheets and mechanical parts (root sheet)

The root sheet holds a title note, the eight sheet symbols (each with a four-line note of the
blocks inside it) and the parts that have no circuit. Neither kind of mechanical part is in JLC's
BOM or placement file: there is nothing to place.

| Parts | Role | Why |
|---|---|---|
| H1-H6 | 2.2 mm holes for M2 screws | the case is held by six M2 screws through the board, which clamp tub and lid as on a DS Lite; the positions come from case.scad (through `positions.json` and placement.py), so the board always matches the case |
| FID1-FID3 | 1 mm fiducials with a 2 mm mask opening | optical alignment marks for JLC's placement machines; the router keeps 0.6 mm of copper clear around each |

The board fits the case one way only: J1 and J2 key it (PLAN.md, Case row).

### 7.5 USB-C, charger, load sharing and cell connector (sheet USB_Charger)

**What it does.** This sheet takes 5 V from USB-C, charges the cell, and makes VSYS, the system
rail: from USB when a cable is in, from the cell otherwise. It also passes USB data on to the MCU
sheet and tells the MCU what the charger and the USB source are doing. Its blocks, as the sheet
heads them: the USB-C receptacle and CC resistors, the ESD array, the VBUS input, the charger,
the cell temperature window, load sharing, and the cell connector.

**Load sharing.** USB feeds VSYS through D5 while the TP4056 charges the cell on its own output.
Without USB, a P-channel FET (Q2) connects the cell to VSYS.

```
               R47 0.1 ohm           D5
 VBUS --+------/\/\/---------------|>|------+-----------+----- VSYS
        |                                   |           |
        |      D6                         +----+      +----+
        +------|>|------ LS_GATE ---------| Q2 |      | D7 |
        |                    |            +----+      +----+
        |                 R6 10k            |           |
        |                    |              |           |
        |                   GND             |           |
        +-- U2 TP4056 --- BAT --------------+-----------+----- VBAT -- JP5 -- J2 (cell)

 Q2: AO3401A P-channel FET: source on VSYS, drain on VBAT, gate on LS_GATE.
 D7: B5819W Schottky, anode on VBAT, cathode on VSYS: in parallel with Q2's body diode.
```

- **USB in:** VSYS is VBUS less D5's drop, about 4.6 V (bring-up expects 4.5 to 4.7 V). LS_GATE
  sits at VBUS less D6's drop, at or above VSYS, so Q2 is off; Q2's body diode and D7 are reverse
  biased.
- **USB out:** R6 pulls LS_GATE to ground and Q2's channel joins the cell to VSYS (at most
  85 mOhm with 2.5 V of gate drive).
- **The unplug.** At an unplug C1 holds VBUS up, and LS_GATE can only fall as fast as VBUS does.
  R38 (1k) drains VBUS, so Q2's channel takes over 1.5 to 3.3 ms after the cable leaves (about 2 to
  3 ms in design.py's comment; tens of ms without R38). Until then D7 carries VSYS at the cell less
  about 0.34 V, and C9 covers the step.

**Parts.**

| Parts | Role | Why this part or value |
|---|---|---|
| J1 (HRO TYPE-C-31-M-12) | USB 2.0 Type-C receptacle: VBUS, CC1 and CC2, D+ and D- (both rows joined), shield to GND; the SBU pins are left open | one USB-C for charging, firmware and console (the stripboard had two USB ports) |
| R1, R2 (5.1k) | Rd pull-downs on CC1 and CC2 | they mark the board as a power sink: a Type-C source turns VBUS on only when it sees them. There is no USB PD |
| R3, R4 (10k); C42, C43 (empty) | CC1 and CC2 into ADC6 and ADC5 (GPIO46, GPIO45) | firmware reads the source's current offer; the 10k keeps a CC line shorted to VBUS off the pin. C42 and C43 are empty 1206 footprints for filter capacitors, fitted by hand only if bring-up shows noise |
| U1 (SRV05-4A, SOT-23-6) | ESD steering array on D+ and D-; its other two lines are unused | 1.0 pF, 15 kV contact and air discharge, next to the connector. Its reference pin is on 3V3, not VBUS (see Protection). hongjiacheng's part since 2026-10-20 (Alex): Preferred at JLC, so no loading fee. Before it: Semtech's Extended SRV05-4.TCT from 2026-10-07, when ProTek's part ran out |
| R41 (2.2 ohm 0603), C1 (10 uF 25 V) | the VBUS bulk capacitor, behind a series resistor | 10 uF is USB's limit for bulk capacitance on VBUS. The resistor is the TP4056 datasheet's fix for a ceramic input capacitor ringing when a live source is plugged in (it asks 1.5 ohm; 2.2 is the nearest Basic part). Added 2026-09-26 |
| R38 (1k 0603) | VBUS bleeder | it drains VBUS after an unplug so Q2 takes over in milliseconds, and it holds D5's leakage away from PWR_EN (see Protection). It costs 5 mA (25-28 mW) with USB in, so it is an 0603 at 25-30% of its rating. 10k on 2026-09-25, 1k after the third audit round the same day, 0603 on 2026-09-26 |
| U2 (TP4056, ESOP-8) | linear Li-ion charger to 4.2 V | a fifth of the BQ24074's price with no stock risk (2026-09-24). Its thermal pad has six 0.3 mm vias into the ground pours on both layers; it lowers its own current when its die passes about 145 C, so heat slows the charge rather than hurting the chip |
| R5 (3.3k) | sets the charge current | 1100 or 1200 V (by datasheet revision) / 3.3k = 333-364 mA, about 0.5 C for the 700 mAh cell, the usual standard rate for small pouch cells. It gave 460-500 mA until 2026-10-05, which the validation plan judged too high for this cell |
| R39 (5.6k), R40 (150k), TH1 (10k NTC, B3380); C41 (empty) | the charger's TEMP input | the TP4056 charges only while TEMP sits between 45% and 80% of VBUS. With TH1 in parallel with R40 that is 1 to 46 C (-3 to 49 C across the chip's threshold spread). The 403450 has no thermistor wire, so TH1 sits on the board over the cell, reading it through the board, about 45 mm from U2 so that U2's 0.65 W while charging doesn't trip it. Fed from VBUS: about 0.6 mA from USB, nothing from the cell. Added 2026-09-25 |
| R43 (100k), Q4 (2N7002), R44 (4.7k) | the charger's enable (CE) | R43 pulls CE to VBUS, so the charger charges whenever USB is in and nothing drives Q4, even with a blank or crashed MCU. CHG_OFF (GPIO19) high turns Q4 on and pauses charging (a weak USB port, a warm case). R44 holds Q4 off through any processor state, at 4.7k because erratum RP2350-E9 needs 8.2k or less. Added 2026-10-05 |
| R9, R10 (100k to 3V3) | pull-ups on the open-drain STDBY (CHG_DONE_N, GPIO18) and CHRG (CHG_STAT_N, GPIO17) | the MCU reads the charger's state; there are no charge LEDs (an approved decision) |
| R47 (0.1 ohm 1206) | damper in D5's path | a live plug-in rings through D5 into the VSYS capacitors: with no cell (or a tripped pack) VSYS peaked at about 6.6 V against the FM8002A's 6 V maximum, about 5.7 V with R47. It costs 50 mV of VSYS at 0.5 A. Added 2026-10-05 |
| D5 (B5819W, 1 A Schottky) | USB into VSYS | a Schottky keeps the drop small, so VSYS sits near 4.6 V on USB |
| D6 (BAT54WS), R6 (10k) | Q2's gate follows VBUS | with USB in the gate is high and Q2 off; without, R6 pulls it low |
| Q2 (AO3401A) | cell into VSYS | P-channel: with LS_GATE at ground its gate sees the whole cell voltage (about -3.4 V at a 3.5 V cell), more than the -2.5 V at which its on-resistance is rated 85 mOhm or less |
| D7 (B5819W) | across Q2 | carries VSYS through the unplug gap at 0.34 V (at 0.25 A) against the body diode's 0.6 V. Added 2026-09-26 after two independent model reviews |
| C9 (22 uF 25 V), C2 (4.7 uF); C3 (10 uF) | VSYS bulk; the charger's output capacitor on VBAT | C9 covers the step at an unplug |
| J2 (Molex PicoBlade 53261-0271), JP5 | the cell connector; a cut jumper from CELL_P to VBAT | J2 is 3.4 mm tall: the 5.5 mm JST PH it replaced on 2026-10-19 needed a pad on the lowered lid. Cells come with PicoBlade ("MX1.25") plugs. Cutting JP5 and bridging it with an ammeter reads everything the cell gives or takes |

**Protection and failure cases.**

- **Hot plug.** The worst case is an A-to-C cable from an always-on USB-A port: a USB-C source
  turns VBUS on only after it sees the Rd resistors, and ramps it. Simulated with a 2 m cable into a
  board with no cell, R41 brings the peaks from 7.1 to 6.7 V on VBUS (U2's maximum is 8 V) and from
  6.7 to 6.2 V on VSYS (the ME6211's maximum is 6.5 V). R47, added later, brings VSYS from about
  6.6 V to about 5.7 V in design.py's own figures, under the FM8002A's 6 V. With a cell in, the
  peaks are lower. There is no TVS on VBUS: an SMF5.0A's 5.0 V working voltage is below USB's
  5.5 V, and it would barely conduct at these peaks.
- **The unplug gap.** Without D7, Q2's body diode would carry the gap at about 0.6 V (0.25 A, full
  backlight): 3V3 would then be the cell less about 0.71 V, under the internal flash's 2.97 V
  minimum (QSPI_IOVDD) on every unplug with the cell below about 3.7 V. With D7, 3V3 stays above
  2.97 V down to a cell of about 3.45 V (design.py; design_review.md gives 3.42 V, or 3.49 V with
  0.25 A of audio as well), inside the 3.5 V cutoff. This matters because the RP2350 has no 3V3
  supervisor (its brown-out detector watches the 1.1 V core): a dip would not reset it cleanly, it
  could corrupt a flash access. And a reset turns the device off on battery.
- **Back-powering through D+.** U1's reference pin was on VBUS until 2026-09-25. With no cable in,
  the MCU's D+ pull-up (on in BOOTSEL mode and whenever USB runs) then fed VBUS through U1's D+
  steering diode to about 2.7 V, which held PWR_EN up (the device could not turn off) and left Q2
  half on. On 3V3 the path is gone.
- **Leakage.** With the cell on VSYS, D5's reverse leakage flows into the VBUS node, and from there
  through R14 and D3 into PWR_EN. Into R15's 100k alone, about 10 uA (a warm Schottky) would reach
  the ME6211's 1.0 V enable level; with R38 on VBUS it takes about 1.3 mA. In the other direction,
  D7's reverse leakage flows into the cell while USB is in: about 5 uA at 25 C and 50 uA at 75 C. A
  full cell left on USB therefore floats near 4.2 V instead of cycling through the 4.05 V recharge
  threshold, a small calendar-ageing cost backstopped by the pack's overvoltage cut-off.
- **Charging outside 1 to 46 C** pauses by itself and resumes when TH1 is back in range. CHRG and
  STDBY then both read high, as with no USB; firmware tells the two apart by the CC lines.
- **A weak USB port.** The TP4056 has no input current limit, so a 500 mA default-USB port can be
  asked for the charge current plus the system's draw. Firmware reads the source's offer on the CC
  lines and dims the backlight on a weak one.
- **A reversed cell destroys the charger.** PicoBlade plugs have no standard polarity; the red lead
  must land on J2's pin 1 (silkscreen `+`), checked before plugging in
  ([assembly.md](hardware/ref_design/docs/assembly.md)).

**Bring-up hooks and firmware.** Probe pads: TP1 (VBUS), TP2 (VSYS) and TP3 (VBAT) are drawn on the
Power sheet, TP7 (LS_GATE) here, and TP15 is a ground clip by the charger. JP5 reads the cell's
current; C41-C43 take hand-fitted filter capacitors. Bring-up step 1 (USB, no cell) checks VBUS
and VSYS; step 5 covers the charger, the temperature cutoff with a hair dryer and a cold pack, the
scoped unplug with the cell at 3.5 V, the hot plug with no cell, and PWR_EN at plug-in. Two of
step 0's unpowered expectations predate D7 and R38: the diode test from TP3 to TP2 reads D7's drop
(about 0.25 V, per the validation plan), not Q2's body diode, and TP1 reads R38's 1k to ground.
Firmware:

- Turn off the pull-downs of CHG_STAT_N and CHG_DONE_N at boot (every GPIO resets with its
  pull-down on, which fights the 100k pull-ups). Decode the states with the table in
  [pinmap.md](hardware/ref_design/docs/pinmap.md): low/high charging, high/low done, high/high no
  USB or a temperature pause. The validation plan adds that the status pins can blink.
- Read CC1_SENSE and CC2_SENSE and take the higher: with 5.1k Rd the active line sits at
  0.25-0.61 V for default USB, 0.70-1.16 V for 1.5 A and 1.31-2.04 V for 3 A.
- Drive CHG_OFF high to pause charging; release it (R44 pulls it low) to charge.

**Open items and risks.**

- The unplug hand-over and the hot plug are simulated (the AO3401A and B5819W curves, the TP4056's
  shut-off, the ME6211's dropout, the routed copper; checked by two model reviews), not measured.
- C9's DC-bias derating, with the unplug and hot-plug simulations re-run, is item 8 of the
  validation plan's go/no-go gate; no document records it as done.
- D5's surge rating and R47's short-time overload against the plug-in inrush are marked unknown in
  the pin review ([pin_review/d5.json](hardware/ref_design/board/pin_review/d5.json),
  [r47.json](hardware/ref_design/board/pin_review/r47.json)); the TP4056 datasheet misprints the
  absolute maximum for BAT and the status pins ([u2.json](hardware/ref_design/board/pin_review/u2.json)).
- The validation plan asks for a short failure table of D5, D7, Q2 and U2: a shorted D7 or Q2
  would charge the cell straight from USB with no regulation.
- TH1 reads the board over the cell and lags it; a charge in a hot car or near 46 C may pause and
  resume.
- The cell's length with its protection board, and its plug polarity, are still to be measured.

### 7.6 3V3 regulator, power latch and cell voltage sense (sheet Power)

**What it does.** This sheet makes the 3V3 rail from VSYS and decides when it exists. It holds the
soft power latch, the circuit that lets the MCU see the power button without touching VSYS, the
cell voltage divider for the ADC, and most of the board's probe pads. Its blocks: the 3V3
regulator, the power latch, button sense, cell voltage sense, and ground pads.

**The latch.** U3's enable (PWR_EN) is the OR of three sources, each through its own Schottky
diode so none can back-feed another:

```
 VSYS --[SW21: seat 1 Start]--+-- PWR_BTN_RAW ------|>|--+   D2, first diode
                              |                          |
                       R13 33k to GND                    |
                       Q1's gate (PWR_BTN_N to GPIO3)    |
                                                         |
 GPIO2 (PWR_HOLD) ------------+---------------------|>|--+   D2, second diode
                              |                          |
                       R42 4.7k to GND                   +-- PWR_EN --> U3 (ME6211) enable
                                                         |   R15 100k and C47 100 nF to GND; TP6
 VBUS -- R14 10k -- VBUS_EN ------------------------|>|--+   D3
```

1. **Off.** R15 holds PWR_EN low; U3 is off and there is no 3V3. The cell still feeds VSYS.
2. **Press Start.** SW21 joins VSYS to PWR_BTN_RAW, D2 raises PWR_EN, U3 starts and the MCU boots.
   Q1 sees the press and pulls PWR_BTN_N (GPIO3) low.
3. **Hold.** Firmware sets PWR_HOLD high once Start has been held for about a second, so a knock in
   a bag doesn't turn the device on: let go sooner and it powers off again.
4. **Turn off.** Firmware watches PWR_BTN_N for a long press and drops PWR_HOLD; power goes when
   Start is released.
5. **USB in.** VBUS holds PWR_EN up through R14 and D3 whatever PWR_HOLD does, so the firmware runs
   while charging.

**Parts.**

| Parts | Role | Why this part or value |
|---|---|---|
| U3 (ME6211C33, SOT-23-5); C5, C6, C7 (10 uF) | 3V3 LDO from VSYS, enabled by PWR_EN; input and output capacitors | $1.40 a unit cheaper than the buck-boost regulator it replaced (2026-09-24). 500 mA maximum, 120 mV dropout at 100 mA: it holds 3.3 V down to a cell of about 3.42 V, and below that 3V3 follows the cell. The load is under 100 mA (the RP2354B up to about 50 mA, the panel's logic about 10 mA), at most 0.13 W in the LDO on USB. The cost: roughly 5-10% of the cell goes unused against the buck-boost |
| JP1 | joins the regulator's output (3V3_LDO) to +3V3 | cut at bring-up to check the regulator alone before the rest of the board sees 3V3, or to feed the rail from a bench supply on TP4 |
| D2 (BAT54C, common cathode) | ORs the button and PWR_HOLD into PWR_EN | two Schottky diodes in one SOT-23 |
| R14 (10k), D3 (BAT54WS) | USB into the OR | keeps the device on while USB is in |
| R15 (100k) | PWR_EN pull-down | 100k, not 10k: a 10k cut by about five times how much resistance the Start contact's pill may have before Q1 misses a press. It was 10k for part of 2026-09-25, until the R38 bleeder took over the leakage job |
| C47 (100 nF) | slows PWR_EN's rise from VBUS to about 1 ms (R14 x C47) | at a live plug-in VSYS lags behind R47 and D5, and the ME6211's enable may not go more than 0.3 V above its input (its absolute maximum). Found by the datasheet pin review, 2026-10-05 |
| R42 (4.7k) | PWR_HOLD pull-down | erratum RP2350-E9 can hold a released pad near 2 V; through D2 that would keep the regulator on after a crash and drain the cell in a bag. E9 needs 8.2k or less. Added 2026-10-05 |
| R13 (33k) | PWR_BTN_RAW pull-down | 33k, not 100k: the node tolerates about twice the surface leakage (sweat, condensation) across the Start contact before it lifts PWR_EN (2026-10-05) |
| Q1 (2N7002), R16 (100k to 3V3) | button sense: gate on PWR_BTN_RAW, drain on PWR_BTN_N (GPIO3) | the MCU reads the power button (low = pressed) and no MCU pin ever touches VSYS |
| R17, R18 (470k each), C8 (100 nF) | the cell's voltage halved, into ADC7 (GPIO47) | 470k keeps the divider's drain to 4.5 uA. 100k/100k would add about 30 uA, most of it into the unpowered chip's clamp, and still not guarantee the reading against the pad's worst-case leakage. C8 smooths the 235k source (23.5 ms time constant) |
| TP1-TP5; TP6; TP15, TP16 | probe pads | VBUS, VSYS, VBAT, 3V3 and GND on the back; PWR_EN on top; two GND pads on top for scope ground clips (2026-10-19) |

**Protection and failure cases.**

- **A crash or reset turns the device off on battery.** The reset releases GPIO2 and R42 pulls
  PWR_HOLD low; once Start is up, PWR_EN falls. On USB it stays on. This is the intended safe
  failure: a device that crashes in a bag turns itself off instead of draining the cell.
- **A hang with PWR_HOLD high** keeps the device on with no way to turn it off but unplugging the
  cell, so the firmware enables the watchdog before any blocking initialisation.
- **The enable above its input at plug-in** (C47) and **leakage lifting PWR_EN** (R38 on the
  USB_Charger sheet, R13 here) are covered above.
- **Off-state drain**, about 18-22 uA in all: the ME6211 off; the amplifier in shutdown (4.2 uA);
  the backlight sinks off; the charger drawing about 2 uA from the cell; the divider 4.5 uA (up to
  9 uA, because with the chip unpowered GPIO47 sits above IOVDD and its ESD diode takes a few uA,
  as on Raspberry Pi's Pico; accepted); and D5's reverse leakage from VSYS into R38 (1-6 uA at
  25 C). That is years of shelf life.
- **The ADC reads against 3V3.** Once the LDO drops out, 3V3 follows the cell and the computed cell
  voltage levels off at 3.37-3.43 V whatever the cell really is, so a cutoff below that would never
  fire. The firmware shuts down at a computed 3.50 V, which also keeps 3V3 above the flash's
  2.97 V through an unplug.

**Bring-up hooks and firmware.** Bring-up step 1 checks 3V3 on TP4 with USB only (R14 and D3 raise
PWR_EN). Step 5.1 connects the cell with USB out and expects nothing to turn on and about 20 uA
(under 30) through JP5. Step 5.7 scopes PWR_EN against VSYS at a live plug-in. Step 6 exercises the
switch: Start powers the board while held, the hold keeps it on, a long press turns it off, and
RESET on battery turns it off. Firmware:

- Turn off GPIO3's pull-down at boot. Set PWR_HOLD only after Start has been held about a second;
  drop it after a long press. On USB, show a charging screen, turn the backlight off and sleep.
- Enable the watchdog before any blocking initialisation.
- The validation plan's power-off order: DISPOFF, then SLPIN, then drop the hold.
- Cell voltage: VBAT = 2 x ADC7 x 3.3 V / 4096, times a gain calibrated once when CHG_DONE_N goes
  low (the cell is then at 4.2 V +-1%). Keep the ADC channel for the whole program (in embassy-rp
  0.8 dropping it turns the pad's digital input and pull-down back on, which drains C8), wait about
  200 ms after setting it up, read bursts of up to 16 one-shot samples at most once a second, and
  shut down only after several low bursts. Details in [pinmap.md](hardware/ref_design/docs/pinmap.md).

**Open items and risks.**

- The latch has not been built. The validation plan asks for a replica from the real parts that
  holds and releases at VSYS 3.5 and 4.2 V and still starts with 1 kohm in series with the button;
  whether that gates the order is disputed.
- A hardware long-press kill (an RC into Q1's gate, as an unfitted footprint) was proposed in the
  validation plan because a watchdog doesn't cover firmware that runs correctly but never drops
  the hold. It was not opposed, and it is not in design.py.
- The pin review notes that the SOT-23's 300 mW limit caps 3V3 at about 158 mA with VSYS at 5.2 V;
  the design's load is under 100 mA.
- A lower-dropout regulator would widen the unplug margin and use more of the cell; it is the
  respin fallback if bring-up step 5 shows 3V3 under 2.97 V at a 3.5 V cell.
- [pinmap.md](hardware/ref_design/docs/pinmap.md) and [bringup.md](hardware/ref_design/docs/bringup.md)
  step 6 still name seat 1's Y as the power button; design.py moved it to Start on 2026-10-19.

### 7.7 RP2354B (sheet MCU)

**What it does.** This sheet holds U4 and what it needs besides power: the crystal, the USB series
resistors, the QSPI_SS boot strap's pull-up, the board revision straps, and a label on every GPIO
that leaves for another sheet. Supplies are on the MCU_Power sheet and debug parts on the Debug
sheet.

**The chip.** U4 is an RP2354B in QFN-80: the RP2350B die (48 GPIOs) with 2 MB of flash stacked in
the package. Only the flash's chip select, QSPI_SS, comes out, as the BOOTSEL strap; the five QSPI
data and clock pins are left open. Raspberry Pi's RP2350B minimal design (MIT) supplies the core
circuit and its layout, turned 90 degrees.

**Pin assignment.** With two layers there is no room for routes to cross, so each pin faces where
its net goes and, along an edge, the pins are in the order their nets fan out: seat 1 below the
chip, seat 2 above, the charger and USB to the right, the panel's connector below. Seat 1's cross
and the panel's TE leave down the left edge on GPIO28-32; seat 1's face buttons, the panel's reset,
the backlight and the ADC inputs run along the bottom edge on GPIO40-47. Raspberry Pi's own pin
escapes are copied with the core, so every used pin leaves the chip on their track. GPIO0, 1, 16,
26 and 27 are spare. The full table is [pinmap.md](hardware/ref_design/docs/pinmap.md).

**Parts.**

| Parts | Role | Why this part or value |
|---|---|---|
| U4 (RP2354B) | the microcontroller | no flash chip to route (2026-09-24); a bare chip because no RP2350B module exists that JLC will place, and the Pico family has too few GPIOs (2026-09-21). $1.43 at 10 |
| X1 (Abracon ABM8-272-T3, 12 MHz); C27, C28 (15 pF); R20 (1k) | the crystal, its load capacitors, and a series resistor on XOUT | as in Raspberry Pi's design: two 15 pF in series make 7.5 pF, plus about 3 pF of stray, 10.5 pF against the crystal's 10 pF. R20 limits the drive into the crystal, per Raspberry Pi's hardware guide |
| R21, R22 (27 ohm) | series resistors on D+ and D- | the RP2350 datasheet requires 27 ohm. They sit in Raspberry Pi's own 0402 spots (a 27 ohm 0603 doesn't fit the copied layout; the 27 ohm 0402 is an Extended part). 22 ohm until the datasheet pin review of 2026-10-05 |
| R23 (10k to 3V3) | QSPI_SS pull-up | holds the internal flash's chip select high at power-up (the reference's value) |
| R48-R51 (1k, or unfitted) | 4-bit board revision on GPIO22-25 | a 1k to GND reads 0; an unfitted bit reads 1 through the pin's pull-up. This board is revision 1, so R48 is unfitted. JLC fits the 0 bits, so a new revision changes the BOM, not the copper; a resistor rather than a hard tie in case firmware ever drives a pin high. Added 2026-10-05 |

**Protection and failure cases.** RUN has an internal pull-up, so it needs no external one (a
reviewer's claim otherwise was rejected on 2026-09-25). Erratum E9 is covered by 4.7k external
pull-downs on PWR_HOLD (R42) and CHG_OFF (R44), under E9's 8.2k limit. AMP_EN and BL_PWM rely on
larger pull-downs (100k, 103.9k), which the pin review finds safe while they are outputs or at
reset (input buffer off), not if firmware releases them as inputs with the buffer on. Buttons and
REV straps use pull-ups, which E9 doesn't affect.

**Bring-up hooks and firmware.** A blank chip boots into its USB boot loader, so the first sign of
life is a drive called RP2350 on the computer (bring-up step 1): that proves the chip, the crystal,
USB and the 1.1 V core at once. Step 2.5 reads the revision straps. Firmware:

- embassy-rp with the `rp235xb` feature (the current firmware builds for `rp235xa`), and a memory
  map with 2048K of flash. The validation plan proposes generating `pins.rs` from design.py.
- Every GPIO resets with its pull-down on. Turn on the pull-ups of the buttons and REV0-REV3, turn
  off the pull-downs of PWR_BTN_N, CHG_STAT_N and CHG_DONE_N, and turn off both the input buffer
  and the pull-down on the ADC pins.
- Erratum E12: firmware using USB must run clk_sys at 1.1 times clk_usb or more.

**Open items and risks.**

- **The RP2354B was out of stock at JLC and LCSC in the 2026-10-21 BOM audit, which blocks ordering**
  (section 8).
- The chip's stepping is unknown. Errata E9 and E16 apply only to the A2 stepping, and which one
  JLC supplies isn't known; the E9 pull-downs are fitted either way.
- Whether the firmware and its assets fit 2 MB of flash has not been measured. No firmware exists
  for this board yet.
- design_review.md and pinmap.md still describe 22 ohm USB resistors, and design.py's own comment
  above R21 still argues for 22 ohm; the value is 27 ohm.

### 7.8 Decoupling and the 1.1 V core switcher (sheet MCU_Power)

**What it does.** The RP2354B runs its I/O, USB, ADC and flash interface from 3V3 and its core from
1.1 V, which its own switching regulator makes from 3V3. This sheet holds the capacitors at every
supply pin and the parts around that switcher. Everything on it is copied, value for value, from
Raspberry Pi's minimal design.

```
 +3V3 ---+------------------------- VREG_VIN (U4)     C24 4.7 uF to GND
         +-- R19 33 ohm --+-------- VREG_AVDD (U4)    C26 4.7 uF to GND

 U4's switcher: VREG_LX -- L2 3.3 uH --+-- +1V1 --> DVDD (three pins) and VREG_FB (U4)
                                       +-- C25 4.7 uF, C20-C22 100 nF to GND
                                       +-- TP9
```

**Parts.**

| Parts | Role | Why this part or value |
|---|---|---|
| C10-C19 (ten 100 nF), C23 (10 uF) | 3V3 decoupling at IOVDD (eight pins), ADC_AVDD, USB_OTP_VDD, QSPI_IOVDD and VREG_VIN; bulk | Raspberry Pi's values and placement |
| C20-C22 (three 100 nF) | DVDD decoupling on +1V1 | the same |
| C24, C25 (4.7 uF 0402) | the switcher's input (on 3V3) and output (on 1V1) | on Raspberry Pi's own wide land pattern: the switcher's LX copper runs between their pads, which KiCad's larger pads would cut |
| R19 (33 ohm), C26 (4.7 uF) | RC filter into VREG_AVDD | gives the switcher's analogue supply a filtered 3V3 |
| L2 (Abracon AOTA-B201610S3R3, 3.3 uH) | the switcher's inductor | meets the RP2350 datasheet's requirements: 3.3 uH +-20%, DCR at most 250 mOhm (this part: 140 max), saturation at least 1.5 A (this part: 2.4 A). Polarity-marked: fit as drawn. $0.27 at 10 |
| TP9 | +1V1 probe pad, on the core's own via left of the chip | added 2026-10-19 |

**Layout.** The copied part of the board keeps what Raspberry Pi's layout depends on:
`build_pcb.py` copies their routing, vias and the small copper areas around the switcher, the
0402 passives use their land patterns, and over the chip the top layer is a 3V3 pour joined by
short stubs from the 3V3 pins. Clearances inside the copied core go down to 0.1 mm (0.15 mm on the
rest of the board), and the copied vias keep 0.6 mm pads with 0.3 mm holes, so the whole board
stays on JLC's standard process.

**Protection and failure cases.** The chip's brown-out detector watches the 1.1 V core, not 3V3,
so nothing on the board resets the chip cleanly if 3V3 dips. That is why the unplug gap (D7) and the
3.5 V cutoff matter. On the A2 stepping only, erratum E16 says a disturbed USB_OTP_VDD can corrupt
OTP reads; the pin review flags that the unplug dips toward 2.97 V go below that pin's 3.135 V
minimum.

**Bring-up.** Bring-up step 1's USB drive proves the core rail; TP9 measures it.

**Open items and risks.**

- **L2 was out of stock with the RP2354B in the 2026-10-21 BOM audit** (757 in stock on 2026-10-07);
  any replacement must meet the three requirements above.
- The validation plan asks for L2's orientation to be checked in JLC's placement preview, and for
  a netlist diff of the copied core against Raspberry Pi's design file (RP-010329).

### 7.9 SWD, UART, debug pads, BOOTSEL and RESET (sheet Debug)

**What it does.** Everything needed to load and debug firmware besides USB: two 3-pin
connectors for a Raspberry Pi Debug Probe, the same SWD on pads on the back, and the BOOTSEL
and RESET buttons. J7 and the board's probe pads came from Alex's debug-readiness review on
2026-10-19.

| Parts | Role | Why this part or value |
|---|---|---|
| J5 (JST SH, SM03B-SRSS-TB) | SWD: SWCLK, GND, SWDIO | the Raspberry Pi 3-pin debug connector pinout, so the Debug Probe's cable plugs straight in |
| J6 (JST SH, SM03B-SRSS-TB) | UART console (UART1, GPIO20 and GPIO21) | the same spec (RP-003139-SP), named from the board's side: pin 1 is the board's RX, where the probe's TX arrives, and pin 3 its TX. The two were swapped until the pin review of 2026-10-05 |
| J7 (six bare pads at 2.54 mm, on the back) | 3V3, SWCLK, SWDIO, RUN, BOOTSEL, GND | for pogo pins in a jig, a clip or soldered wires: flash and debug without J5's cable or USB. Not assembled: it is copper only |
| SW18 (BOOTSEL), R25 (1k) | pulls QSPI_SS to GND through 1k | holding it through a reset enters the USB boot loader; the reference design's strap |
| SW19 (RESET), R26 (1k) | pulls RUN to GND through 1k | resets the chip |

Both buttons are XKB TS-1088 tactile switches under pinholes in the lid. There are no LEDs (an
approved decision).

**Protection and failure cases.** J7's 3V3 pad feeds the 3V3 rail directly, so power the board
through it only with the cell out and nothing on USB. RESET on battery turns the device off, since
the reset drops PWR_HOLD; that is the intended behaviour, and it means a player's only reset is
power-cycling.

**Bring-up and firmware.** Blank flash always lands in the boot loader. With firmware on it, hold
SW18 and tap SW19 to get back there; on J7, short BOOT to GND while tapping RUN to GND. Bring-up
step 2 attaches probe-rs or openocd through J5 and then J7, and reads the bring-up firmware's
report on J6 at 115200 baud.

**Open items.** No LED means no visible error indicator beyond the screen (section 11).

### 7.10 Panel connector and backlight sinks (sheet Display)

**What it does.** The panel is an EastRising ER-TFT035IPS-6: 3.5 in IPS, 320x480, an ILI9488
controller on 4-wire SPI at 3.3 V. Its 50-contact FPC (0.5 mm pitch) leaves the panel's seat-1
edge, folds under it and plugs into J3. This sheet holds J3 and the way each panel pin is tied, the
panel's logic supply, the SPI series resistors and probe pads, and the backlight: a filtered PWM
reference and two linear current sinks.

**The panel's pins.** Wired as the datasheet says: IM2-IM0 high (4-wire 8-bit SPI), RD (a parallel
read strobe) high, the parallel bus DB0-DB17 and the RGB strobes (VSYNC, HSYNC, DOTCLK, DE) low,
and the touch pins open (this panel has no touch layer). The folded FPC's contact 1 lands at the
right of J3 while J3's pin 1 is at its left, so J3 pin j carries panel pin 51 - j. TE goes to
GPIO32.

**The backlight.** Six LEDs in parallel (3.0-3.2 V, 120 mA at most), common anode on VSYS, three on
each of two cathode pins. Each cathode pin has its own current sink:

```
 GPIO44 (BL_PWM) -- R60 100k --+-- BL_REF ----------------> +IN of both halves of U6
                               |   R61 3.9k and C50 1 uF to GND; TP10

 VSYS -- JP2 -- LED_A -- three LEDs -- LED_K1 -- Q5 drain
                                                 Q5 source (BL_S1) -- R52 2.2 ohm -- GND
 U6, one half:  output -- R54 330 ohm -- Q5 gate     (R56 100k from the gate to GND)
                -IN    -- R58 10k -- BL_S1           (C48 1 nF from the output to -IN)
```

The op-amp drives Q5's gate until the sense voltage on R52 equals BL_REF: 0.124 V at 100% duty is
56 mA a pin, 113 mA for all six LEDs. LED_K2 has the same sink built from the other half of U6, Q6,
R53, R55, R57, R59 and C49.

**Parts.**

| Parts | Role | Why this part or value |
|---|---|---|
| J3 (Hirose FH12-50S-0.5SH(55)) | 50-pin, 0.5 mm, bottom-contact ZIF connector | JLC places it, so no hand soldering per unit, and a cracked panel swaps in a minute (2026-09-24, when this panel replaced the solder-tail QD3525). It sits where the folded FPC's tip reaches. $0.81 at 10 |
| JP4; C29 (1 uF), C30 (100 nF) | the panel's logic supply (LCD_3V3) from 3V3; decoupling at the connector | JP4 is cut to run the board with the panel unpowered, or to read the panel's logic current |
| R28, R29 (33 ohm) | source terminations on the SPI clock and data (SCK, MOSI) | fitted next to the MCU, the source end of each line |
| TP11-TP14 | SCK, MOSI, CS, DC | four logic-analyser channels; the panel is the part most likely to need one (2026-10-19) |
| U6 (MCP6002); C51 (100 nF) | dual op-amp, one half per cathode pin; its supply capacitor | replaced the AP3156 charge pump, out of stock at JLC and LCSC on 2026-10-07; every sink part is Basic or Preferred. $0.24 at 10 |
| Q5, Q6 (AO3400A) | the pass transistors, run in their linear region | the pin review checked the simulation's AO3400 model against this part's datasheet (threshold, on-resistance) |
| R52, R53 (2.2 ohm 0603) | sense resistors | 2.2 ohm x 56 mA = 0.124 V; the sink needs only about 0.13 V across it, so the current holds from a full cell (or USB) down to the 3.5 V cutoff with the panel's 3.2 V maximum Vf |
| R54, R55 (330 ohm); R58, R59 (10k); C48, C49 (1 nF) | gate resistors; local feedback loop | the 330 ohm isolates the FET's gate capacitance from the op-amp, and the 10k with 1 nF closes a local loop: 72.5 degrees of phase margin or more at all 720 corners simulated |
| R56, R57 (100k) | gate pull-downs | the sinks are off whenever 3V3 is (U6 runs from 3V3) |
| R60 (100k), R61 (3.9k), C50 (1 uF) | turns GPIO44's PWM into the DC reference BL_REF | 3.3 V x 3.9 / 103.9 = 0.124 V at 100% duty. Time constant 3.75 ms: 0.3% ripple at 36.6 kHz. The LED current is DC, so there is no PWM in the LEDs for the amplifier to pick up |
| JP2 | VSYS to the LED anode | cut it and bridge it with an ammeter to read the LED current, or leave it open to run the panel dark |
| TP10 | BL_REF probe pad, by U6 | the backlight's set point (2026-10-19) |

The sinks were simulated in ngspice before layout with Microchip's MCP6001/2 and Alpha & Omega's
AO3400 models ([board/sim/backlight.py](hardware/ref_design/board/sim/backlight.py), results in
`backlight_results.txt`), over duty, VSYS 3.5-5.7 V, LED Vf, FPC inductance and doubled gate
capacitance. Their efficiency is that of the AP3156 in its 1x mode (Vf / VSYS), so battery life is
unchanged.

**Protection and failure cases.**

- **The backlight is off whenever 3V3 is**, though its anode sits on the always-live VSYS: U6 runs
  from 3V3 and R56/R57 hold the gates low. GPIO44 resets low and R61 holds BL_REF at 0, so it also
  stays off until firmware starts the PWM.
- **No spike at power-up, power-down or a VSYS hot plug with 3V3 off** (0.5-4.7 mA for about a
  microsecond, simulated). A VSYS step of 1.9 V in 2 us while lit (USB plugged in) briefly
  overshoots to 133 mA a pin for about 2 us.
- **The LED limit.** 100% duty gives 113 mA against the panel's 120 mA maximum.
- **Back-powering the panel.** With JP4 open the panel's inputs would feed it from the SPI lines,
  so firmware must hold them low.

**Bring-up hooks and firmware.** Bring-up step 3 covers the panel and its backlight. Firmware:

- The ILI9488 needs a new driver (18-bit colour over SPI, unlike the stripboard's ST7796).
  Pulse LCD_RST, send the init sequence over SPI0 at 10-20 MHz (the datasheet rates serial writes
  to 20 MHz), and read ID4 (0xD3) back on LCD_MISO at 5 MHz or less: expect 0x94 0x88.
- Over SPI the ILI9488 takes 18-bit colour only (COLMOD 0x66): three bytes a pixel, 461 kB a full
  frame, so redraw only what changes. Enable TE (TEON, 0x35) and start frames on GPIO32's pulse,
  about 60 Hz, to avoid tearing.
- Backlight: PWM10 A on GPIO44, for example 12 bits at 36.6 kHz. Current is proportional to duty,
  so apply a gamma curve; keep the lowest level at about 18% duty, where the op-amp's offset (up to
  4.5 mV against 22 mV) could make the two halves visibly unequal. A change settles in about 15 ms.
  Duty 0 is off.

**Open items and risks.**

- The panel has one maker, EastRising, selling direct. Another 3.5 in panel that fits the
  56.0 x 86.24 x 4.9 mm bay needs only case.scad's panel numbers, J3 and its pin map changed.
- The FPC's reach into J3 and the reversed pin order (contact 1 on J3's pin 50) are checked only
  on paper. Lighting a real panel through a bottom-contact breakout with design.py's mapping is
  item 4 of the validation plan's gate. The model errs long; if the tail bows, J3 moves 1.5-2 mm
  towards seat 1.
- An LED at the older datasheet's 3.4 V Vf would lose 6% of its current at 100% on a 3.5 V cell.
- Accuracy is set by the op-amp's offset: +-3.6% at full, +-20% a pin at 18% duty.
- The pin review notes that the panel's VCI and VDDI are rated to 3.3 V, and the ME6211's
  tolerance allows up to 3.366 V: minor, well inside the absolute maximum.
- The validation plan also proposed a spare filter footprint on LCD_RST; design.py has none.

### 7.11 PWM audio and amplifier (sheet Audio)

**What it does.** A GPIO's PWM is filtered into an audio signal and played by a bridge-tied
amplifier into an 8 ohm speaker. The cost route was approved on 2026-09-24: PWM instead of an I2S
amplifier.

```
 GPIO8 (AUDIO_PWM) -- R32 1k --+-- R33 1k --+-- C39 220 nF -- R34 20k --+-- U7 -IN
                               |            |                           |
                          C37 4.7 nF   C38 4.7 nF                       +-- R35 20k -- U7 VO1
                             to GND       to GND

 U7 (FM8002A): +IN and BYPASS on AMP_BYP, C40 1 uF to GND
               VO1 (SPK_P) -- J4 pin 1, VO2 (SPK_N) -- J4 pin 2: the speaker, bridge-tied

 VSYS -- JP3 -- AMP_VDD (C34 10 uF, C35 100 nF) -- U7's supply
 AMP_VDD -- R36 100k -- SHUTDOWN (TP8) -- Q3 drain;  Q3 gate = AMP_EN (GPIO11), R37 100k to GND
```

**Parts.**

| Parts | Role | Why this part or value |
|---|---|---|
| R32, R33 (1k); C37, C38 (4.7 nF) | two RC low-pass stages | with the PWM at 8 bits (586 kHz) the carrier comes out about 49 dB down, and the audio band is -3 dB at about 14 kHz loaded, about what a 15 mm speaker reproduces. 4.7 nF is a JLC Basic part; 3.9 nF, which would give 17 kHz, is only Extended. Changed from 10 nF on 2026-09-25 |
| C39 (220 nF) | DC block | blocks the PWM's DC (it idles at half scale); with R34 it sets a 36 Hz low corner |
| R34, R35 (20k) | input and feedback resistors | gain 2 x R35 / R34 = 2, the datasheet's circuit; with the filter's 2k in front of R34 the chain's gain is about 1.8 |
| C40 (1 uF) | the bypass capacitor: a half-supply reference on +IN | the datasheet's value; charging it is why the amplifier takes about 100 ms to wake |
| U7 (FM8002A, SOIC-8) | class AB bridge-tied amplifier from VSYS, rated 1.1 W | the cheapest way to keep a speaker, approved over the class-D NS4168, which is 50 cents a unit more and cleaner. Class AB turns about half its battery draw into sound at high volume, against 80-90% for class D |
| JP3; C34 (10 uF), C35 (100 nF) | AMP_VDD from VSYS; supply decoupling | JP3 is cut to take a noisy or shorted amplifier off the board, or to measure its current |
| R36 (100k), Q3 (2N7002), R37 (100k) | shutdown control | R36 holds SHUTDOWN high, so the amplifier is off unless the MCU turns it on; AMP_EN high turns Q3 on and pulls SHUTDOWN low. R37 keeps it off while the MCU boots. Off: 4.2 uA. On: 4.4 mA, plus 37-57 uA through R36 |
| TP8 | AMP_SHDN probe pad | added 2026-10-05 |
| J4 (Molex PicoBlade 53261-0271) | the speaker's header | the speaker (INGHAi GSPK151035PN: 15 x 10 x 3.5 mm, 8 ohm, 0.5 W, 90 dB) comes with a 1.25 mm 2-pin plug, so nothing is soldered at assembly (Alex, 2026-10-20). It is J2's part, so it adds no part number at JLC |

The speaker lies face down on the tub floor beside the cell and plays out of the left wall's lower
edge (2026-10-20); until then it fired down through a floor grille that a table covers.

**Protection and failure cases.**

- **Off unless asked.** Through boot, a reset or a crash, R37 holds AMP_EN low and R36 keeps the
  amplifier in shutdown, drawing 4.2 uA from the cell.
- **Supply transients.** VSYS rings at a hot plug; R47 (section 7.5) keeps it at about 5.7 V in
  simulation, under the FM8002A's 6 V maximum (0.3 V of margin).
- **Bridge-tied outputs.** Neither speaker pin may touch GND. The cell's plug would also fit J4 and
  put the cell across the amplifier's outputs, so J4 sits out of the cell plug's reach.
- **Heat.** The worst case, a continuous full-scale sine on USB, is about 0.54 W in the SOIC-8.
- **No backlight whine.** The backlight current is DC, so there is no PWM in the LEDs to couple into
  the audio.

**Bring-up hooks and firmware.** Bring-up step 4 plays a 1 kHz sine, checks that the carrier is gone
at C39, listens for hiss, measures the wake delay and checks U7's temperature after 10 minutes at
full scale on USB. Firmware:

- PWM4 A on GPIO8 at 8 bits (586 kHz). Not 10 bits: 146 kHz would come out only 26 dB down. Dither
  or noise-shape for more resolution.
- Idle the PWM at half scale so turning on doesn't pop. Keep AMP_EN high while the device is awake
  (about 4.4 mA), or raise it at least 150 ms before a sound.
- Full scale is about 0.55 W on USB and 0.4-0.5 W on the cell, and it clips below a 3.7 V cell:
  back the amplitude off there, and cap it on USB if U7 runs hot.
- A mixer, when written, lives in core so the browser sounds like the device (section 5.3).

**Open items and risks.**

- Nothing here has been measured. A sample speaker's plug must still be tried in a genuine
  53261-0271 header, and how loud it is out of the side wall is unknown.
- Full scale on USB (about 0.55 W) is a little above the speaker's 0.5 W rating in bom.md; the
  documents don't reconcile the two.
- The validation plan proposed an unfitted RC footprint on the speaker outputs; design.py has none.
- AMP_EN relies on R37's 100k against erratum E9 (section 7.7): safe while GPIO11 is an output.
- The sheet's own note still says "8 ohm 1 W speaker on the J4 pads"; J4 is a header and the
  speaker is rated 0.5 W.

### 7.12 Membrane contacts (sheet Buttons)

**What it does.** 20 contacts under DS Lite silicone: per seat a D-pad, A, B, X, Y, Start and
Select. Each joins its own GPIO to GND when pressed, except seat 1's Start, which joins VSYS to
the power latch.

| Parts | Role | Footprint |
|---|---|---|
| SW1-SW8 | seat 1's Up, Down, Left, Right, A, B, X, Y | DSL_button_4ring |
| SW9-SW16 | seat 2's Up, Down, Left, Right, A, B, X, Y | DSL_button_4ring |
| SW20, SW22 | seat 1's and seat 2's Select | DSL_button_3ring |
| SW21 | seat 1's Start, and the power button: VSYS to PWR_BTN_RAW | DSL_button_3ring |
| SW23 | seat 2's Start | DSL_button_3ring |

**The contacts.** Each is two interleaved combs of bare copper with an ENIG finish (0.15 mm lines,
0.20 mm gaps) under one 4.8 mm mask opening; a carbon pill of 1.6 mm or more bridges them anywhere
on the pad. The 3-ring version is the same comb without its outer ring, for the Start/Select
membrane's smaller pills (an estimate). Each comb leaves through a 0.2 mm stub to a via 3.3 mm from
the contact's centre, with nothing else (no vias, no pour) inside 2.9 mm. The contacts are copper,
not parts: JLC places nothing here. The footprints are generated and checked by the tools in
[hardware/membrane/](hardware/membrane/README.md) (0.200 mm minimum gap, 1.60 mm bridging spot).

**Why it is built this way.**

- **One GPIO per contact, no matrix.** v1's button matrix ghosted presses across players; direct
  GPIO cannot (2026-08-19, kept here).
- **DS Lite silicone on bare contacts** (2026-09-21): half the footprint of SNES parts, sold as
  repair spares, one D-pad piece and one ABXY piece per seat. ENIG because the pills press straight
  on the copper.
- **Start and Select per seat, power on seat 1's Start** (Alex, 2026-10-19): it matches the DS Lite
  layout, and no game button doubles as power (the power button was seat 1's Y before). The
  contacts sit between the clusters near the case's end, under the DS Lite kit's Start/Select
  membrane.

**Protection and failure cases.** The power contact is the only one on VSYS. Its other side reaches
the MCU only through Q1, so no GPIO ever sees VSYS, and R13 (33k) keeps surface leakage across the
contact from lifting PWR_EN (section 7.6). There are no ESD parts on the contact lines; ESD has not
been tested (design_review.md, "Not checked").

**Bring-up and firmware.** Bring-up step 2.1 prints all 20 buttons while each contact's combs are
shorted with tweezers. Firmware enables the internal pull-ups (the pins reset with pull-downs on);
a contact reads low when pressed, and seat 1's Start reads on GPIO3 (PWR_BTN_N), also low when
pressed. Tap versus 500 ms hold is the platform's job; what a press means is core's (section 5.3).
Start and Select have no game meaning in core yet.

**Open items and risks.**

- The validation plan's gate asks for pill resistance under 1 kohm on a matched ENIG coupon, and
  leakage above 100 Mohm on the power contact at 4.2 V (it names P1_Y, the power contact before
  2026-10-19).
- ENIG combs may drift over a product's life; carbon ink would last longer but needs a coarser
  pattern.
- The Start/Select pad is not measured, so the 3-ring footprint and its wells are estimates; the
  A/B/X/Y stems against the domes and the D-pad's pivot are open, and `check_buttons.py` fails until
  they settle (section 11).

## 8. Manufacturing and cost

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

## 9. IP and regulatory stance

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

## 10. Verification and tooling

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

## 11. Status, known gaps and open decisions

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

## 12. Where things live

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

## 13. Glossary

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
| REF-A | The reference design in `hardware/ref_design/` ([section 7](#7-the-reference-design-ref-a)) |
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
