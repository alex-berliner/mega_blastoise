# Mega Blastoise: design document

The whole project on one page: what it is, why it is built this way, how the pieces fit, and
where the details live. It is current as of 2026-10-21. Detailed specifications ([pin tables](#g-pin-map),
part values, [BOM](#g-bom) lines, layouts) stay in the documents this one links to; where a number appears
here it is to explain a decision, and the linked document is the authority.

Owner: Alex Berliner. Much of the software and the hardware [reference design](#g-ref-a) were written with
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
between them, and each player has a [DS Lite](#g-ds-lite) style D-pad and face buttons at their own end. They
pick random teams and fight a full [singles battle](#g-singles) under [Gen 1](#g-gen) or Gen 3 rules. Everything runs on
one [microcontroller](#g-mcu): no phone, no PC, no network.

The same game also runs in a web browser, built from the same Rust code, and is published on
[GitHub Pages](#g-github-pages): [play it here](https://alex-berliner.github.io/mega_blastoise/battler/device.html).

Goals, in the order the project has held them:

1. **Two strangers can sit down and play.** The first device was aimed at demoing at a
   convention (Open Sauce 2026, per [the v1 overview](docs/archive/v1/DESIGN.md)): no setup,
   no loose pieces, a first match within seconds. An [attract demo](#g-attract-demo) and a "hold A to fight the AI"
   option exist for a lone visitor.
2. **It must feel like a purpose-built game device**, not an emulator or a phone app. Since
   2026-10-19 the explicit target is that the case should feel like holding the bottom half of a
   Nintendo DS Lite ([hardware/ds_lite/](hardware/ds_lite/README.md)).
3. **The battle rules must be verifiably the same as a community-accepted reference.** Since
   2026-08-16 the goal is a complete recreation of [Pokemon Showdown](#g-showdown)'s Gen 1 and Gen 3 singles
   combat, [abilities](#g-ability) and [held items](#g-held-item) included. Showdown wins every disagreement, and the [engines](#g-battle-engine)
   are checked against it by [differential tests](#g-differential).
4. **The browser build and the device must be indistinguishable** in everything a player sees
   and does (the [zero-drift rule](#g-zero-drift), [section 5.3](#53-the-zero-drift-rule)). The browser is where
   the game is developed and tested; the device is the source of truth when they disagree.
5. **The hardware should become something that could be batch-produced and sold.** Since
   2026-09-17 the hand-wired prototype is being turned into a factory-assembled [PCB](#g-pcb) with a
   printed case.

Out of scope for now: radios (no Wi-Fi, no Bluetooth), doubles battles, per-move attack
animations beyond a small shared vocabulary, and shipping any Pokemon assets on the product.

## 2. Product concept and player experience

### The table, the screen and the seats

The console is never picked up or turned. Two [seats](#g-seat) face each other across it, so no screen may
assume a single reader. The [panel](#g-panel) is portrait and split into two [halves](#g-half), one per seat, and the
far half is drawn rotated 180 degrees so both players read their own half upright. There is
exactly one arrangement and no orientation setting.

During a battle the two halves become one shared scene: both active Pokemon stand side by side in
a band across the middle of the panel, each upright to its owner and facing the other, with each
seat's [HP](#g-hp) plate under its own [mon](#g-mon) and the narration drawn in both halves. When either half shows
something private (move choice, party list, battle log, options), a divider returns between the
halves. Each private half is framed in its seat's colour (red or white), so the whole panel reads
as a Poke Ball. The full rules are in
[docs/archive/architecture/09-single-screen.md](docs/archive/architecture/09-single-screen.md) section 3, and the
visual spec is [crates/mega_blastoise_web/www/ui_flow.html](crates/mega_blastoise_web/www/ui_flow.html).

The screen needs an [IPS panel](#g-ips): a [TN panel](#g-tn) shows inverted contrast to whichever seat views it from
the far side.

### Controls

Each seat has the same controls, placed with rotational symmetry so both players get an identical
hand position. The [reference design](#g-ref-a) follows the [DS Lite](#g-ds-lite): a D-pad, A/B/X/Y in a diamond, and Start
and Select side by side between the clusters.

| Control | Meaning |
|---|---|
| D-pad | Moves the cursor (the web page repeats a held direction; the [firmware](#g-firmware) does not yet) |
| A | Confirms. Holding A in the lobby summons the AI as the opponent |
| B | Backs out; every state has a back path. On a locked-in choice it takes the choice back |
| X (INFO, shown as `?`) | Explains what the cursor is on (move details, party stats); outside a choice it opens the battle log |
| Y (MENU) | Currently behaves as B on both [platforms](#g-platform); a real menu action is still to come |
| Start, Select | Present on the hardware and the web page, with no game meaning in [core](#g-core) yet. On the reference design, seat 1's Start is also the power button |

The cursor model replaced the first device's scheme, where a physical corner button meant the move
drawn at that corner and a stray tap committed a choice with no way back.

### A session

1. **Attract demo.** After 15 s idle in the lobby, the device plays an AI-versus-AI battle across
   both halves.
2. **Gen picker and options.** [Gen 1](#g-gen) or Gen 3 (the Gen 3 entry is labelled "preview"), drawn into
   both halves; either player can drive it. Options open per seat over shared settings: team size
   (3v3 or 6v6), text speed, and a tutorial shown before the first game.
3. **Lobby.** Each player presses to ready up; holding A fights the AI instead.
4. **Teams** are a fresh random draft every battle, rematches included, from [Showdown](#g-showdown)'s [random
   battle sets](#g-randbat) for the chosen generation.
5. **Choosing.** Each player picks a move or a switch on their own half with the cursor. A
   committed player sees a waiting screen and can take the choice back; once both are ready the
   turn fires after a 1 s [grace window](#g-grace).
6. **Playback.** Narration, HP drain, the [sprite bob](#g-sprite) and a small set of attack effects play over
   the shared scene. The battle log (the last 50 lines, with damage numbers and the reasons for
   [crits](#g-crit) and [effectiveness](#g-type)) is available on `?` and pauses only the reader.
7. **Faints and switches** ask the owning player for the next mon, showing the rival's side.
8. **Result and recap**, then back to the lobby.

### Open information

Information is open: each player can see the rival's active mon, its HP and status. Only the move
a player has locked in stays hidden until playback. The [v1](#g-v1) device had a "concealed" mode with
per-player screens; it was deleted in the single-screen redesign (2026-08-06), whose one screen
sits in view of both players.

## 3. History and generations

The repository starts on 2026-04-28. Four hardware generations exist; only the first two were
built.

| Generation | Dates | What it was | Where it is now |
|---|---|---|---|
| **[v1](#g-v1): two [OLEDs](#g-oled)** | 2026-04 to 2026-08 | [Raspberry Pi Pico](#g-pico) ([RP2040](#g-rp2040)), one 128x64 mono OLED per player, [WS2812B](#g-ws2812b) LED strips for [HP](#g-hp) and party, a [button matrix](#g-button-matrix) with move buttons at the screen corners. Built first on [breadboard](#g-breadboard) and [stripboard](#g-stripboard), then as a fabricated [PCB](#g-pcb) designed by a collaborator (Eddy Zheng; files committed 2026-07-13), which ran in a printed case on 2xAA cells by early August | Removed from the tree on 2026-10-19; tag `v1-archive` holds its board, stripboard and case files; notes in [docs/archive/v1/](docs/archive/v1/DESIGN.md) |
| **[Stripboard v2](#g-stripboard-v2): single screen** | designed 2026-08-19/20, running 2026-08-26 | Pico 2 ([RP2350](#g-rp2350)), one 3.5 in [ST7796](#g-ili9488) 320x480 [panel](#g-panel), 16 direct-GPIO [tactile switches](#g-tactile), a [LiPo](#g-lipo) with a [TP4056](#g-tp4056) [module](#g-module), all on one 24x56 stripboard | Built and working; the [firmware](#g-firmware)'s current target ([hardware/stripboard_v2/](hardware/stripboard_v2/BUILD.md)) |
| **[prod_board](#g-prod-board) / prod_case: Alex's production board** | from 2026-09-23 | Alex's own [KiCad](#g-kicad) board and [OpenSCAD](#g-openscad) case, made by hand to learn the tools | In progress ([hardware/prod_case/](hardware/prod_case/README.md), `hardware/prod_board/`) |
| **ref_design ([REF-A](#g-ref-a)): the reference design** | from 2026-09-24 | Claude's complete, orderable version: [RP2354B](#g-rp2354b), 3.5 in [IPS panel](#g-ips) in a [ZIF connector](#g-zif), [DS Lite](#g-ds-lite) [membranes](#g-membrane), one [Li-ion cell](#g-li-ion), speaker, [2-layer](#g-layers) JLCPCB-assembled board, printed DS Lite style case | Designed and checked, not yet ordered ([hardware/ref_design/](hardware/ref_design/README.md)) |

Milestones:

| Date | Milestone |
|---|---|
| 2026-04-28 | First commit: Pico firmware wrapping the open-source [`battler`](#g-battler) [engine](#g-battle-engine), a typed event pipeline, a [host](#g-host) test harness |
| 2026-05-09 | Web build ([WASM](#g-wasm)) published on [GitHub Pages](#g-github-pages); lobby with AI demo and ready-up |
| 2026-05-15 | Hand-written `gen1_battle` replaces `battler` on the device: about 40 times less [heap](#g-heap) |
| 2026-07-09 | [`ChoiceCollector`](#g-choicecollector) and the shared OLED controller move all input and display semantics into [core](#g-core); the [zero-drift rule](#g-zero-drift) is written down |
| 2026-07-13 | `gen1_battle` ports [Showdown](#g-showdown)'s [Gen 1](#g-gen) mechanics and format rules |
| 2026-08-06 | Single-screen redesign specified ([09-single-screen.md](docs/archive/architecture/09-single-screen.md)) and a working web client built; tag `mono-renderer-final` marks the last mono renderer |
| 2026-08-15 to 08-19 | `gen3_battle` added; [Showdown parity](#g-parity) and [fuzz suites](#g-fuzz); Gen 3 moves, [abilities](#g-ability) and [held items](#g-held-item) brought in under the fuzzer |
| 2026-08-20 | Stripboard v2 generated end to end from one layout script |
| 2026-08-26 to 08-28 | The game runs on the stripboard's panel; the game loop moves into core as [`DeviceRunner`](#g-devicerunner); [dirty-tile updates](#g-dirty-tiles) |
| 2026-09-17 to 09-21 | Production board planned ([10-production-board.md](docs/archive/architecture/10-production-board.md)): DS Lite membranes, bare RP2350B, 3.5 in [IPS class](#g-glass-class) |
| 2026-09-24 | Reference design started; on the same day its 4-layer first version (tag `archive/ref-design-4layer`) became a cheaper 2-layer build on `main` |
| 2026-09-25/26 | Three audit rounds and a two-model debate settle the open circuit decisions; printable [fit kit](#g-fit-kit) |
| 2026-10-05 | First case print and fixes; joint [validation plan](#g-validation-plan); [datasheet pin-review gate](#g-pin-review); [exploded 3D viewer](#g-exploded) on Pages |
| 2026-10-07 | Backlight [charge pump](#g-charge-pump) out of stock, replaced by [discrete](#g-discrete) [current sinks](#g-current-sink) |
| 2026-10-09 | The cell moves under the board (the cell-on-board layout stays on branch `ref-design-cell-on-board`) |
| 2026-10-19 | Start and Select added per [seat](#g-seat), power moved from seat 1's Y to seat 1's Start; screen in a [lid](#g-tub-lid) pocket; [debug pads](#g-probe-pad); DS Lite shell; v1 removed from the tree; [crates](#g-crate) moved under `crates/`; Pages front page; DS Lite research packet |
| 2026-10-20 | [Schematic](#g-schematic) written directly in KiCad 10 format; speaker plays out of the side wall on a plug-in [header](#g-header); Alex's silicone pad measurements folded in; print kits |
| 2026-10-21 | [BOM](#g-bom) audit: per-unit cost updated, and the RP2354B and its core [inductor](#g-inductor) found out of stock (see [section 8](#8-manufacturing-and-cost)) |

Tags worth knowing: `v1-archive` (the v1 hardware), `mono-renderer-final` (last two-OLED
renderer), `pre-collector-refactor`, `archive/ref-design-4layer`,
`ref-design-battery-under-board` (the case as first printed).

## 4. Decisions and their reasons

The decisions that shape the project, newest last. Each links to where it is argued in full.

| Date | Decision | Why | Recorded in |
|---|---|---|---|
| 2026-05-15 | Own [Gen 1](#g-gen) [engine](#g-battle-engine) instead of [`battler`](#g-battler) on the device | `battler` held about 13 KB per Pokemon and exhausted the [RP2040](#g-rp2040)'s 128 KB [heap](#g-heap) in 3v3 battles; `gen1_battle` keeps a 6v6 battle in about 800 B of RAM with flash-resident tables | [crates/gen1_battle/SPEC.md](crates/gen1_battle/SPEC.md) |
| 2026-07-09 | All display and input semantics in [core](#g-core); [platforms](#g-platform) do raw IO only | The browser and the device had drifted (different screens, different input rules); structure, not discipline, keeps them identical | [CLAUDE.md](CLAUDE.md) |
| 2026-08-06 | One shared colour screen, cursor input, [open information](#g-open-information) | The old scheme let a stray tap commit a move with no way back; an audit of the assembled [v1](#g-v1) build (2026-08-05) also found cross-player [ghost presses](#g-ghosting) and the LED strips over the power budget | [09-single-screen.md](docs/archive/architecture/09-single-screen.md) |
| 2026-08-06 | [RP2350](#g-rp2350), not [ESP32-S3](#g-esp32) | ESP32-S3 would mean a new toolchain and a rewrite of the [embassy-rp](#g-embassy) IO layer, and losing [probe-rs](#g-probe-rs), [defmt](#g-defmt) and [RTT](#g-rtt); RP2350 has the RAM and flash | 09-single-screen.md section 7 |
| 2026-08-11 | The battle is one head-to-head scene across the middle | Drawing each [mon](#g-mon) in its own half staggered the pair; a shared band reads as one field | 09-single-screen.md section 3 |
| 2026-08-15 | Gen 3 as a sibling engine (`gen3_battle`), written in-house | Gen 1 stays untouched; the new [crate](#g-crate) is [`no_std`](#g-no-std) and allocation-free on the [hot path](#g-hot-path), which upstream `battler` is not | [crates/gen3_battle/src/lib.rs](crates/gen3_battle/src/lib.rs) |
| 2026-08-16 | [Showdown](#g-showdown) is the reference; read its source before implementing, [fuzz](#g-fuzz) to confirm | The project's value is being verifiably the same as an accepted reference; guessing by probing cost repeated rework | [tools/showdown-parity/README.md](tools/showdown-parity/README.md) |
| 2026-08-19 | [Stripboard v2](#g-stripboard-v2): 16 switches straight to [GPIO](#g-gpio), no [matrix](#g-button-matrix) | The v1 matrix ghosted presses across players; direct GPIO cannot | [hardware/stripboard_v2/BUILD.md](hardware/stripboard_v2/BUILD.md) |
| 2026-08-19 | 3.5 in 320x480 [panel](#g-panel) instead of the 2.8 in 240x320 in the redesign spec | The 3.5 in [ST7796](#g-ili9488) [module](#g-module) was what was bought; the UI was regrown to 320x480 native on 2026-08-27 | BUILD.md; [firmware](#g-firmware) history |
| 2026-08-27 | The game loop ([`DeviceRunner`](#g-devicerunner)) lives in core | The first single-screen web client kept lobby, battle wiring and pacing in web-only code, so the firmware had a UI with no battles behind it | [crates/mega_blastoise_core/src/device_runner.rs](crates/mega_blastoise_core/src/device_runner.rs) |
| 2026-09-17 | No radio; one [USB-C](#g-usb-c); audio added | No radio keeps certification on the [unintentional-radiator](#g-unintentional) path; the [stripboard](#g-stripboard) had two USB ports and no sound | [10-production-board.md](docs/archive/architecture/10-production-board.md) |
| 2026-09-21 | [DS Lite](#g-ds-lite) silicone [membranes](#g-membrane) on bare [PCB](#g-pcb) contacts | Half the footprint of [SNES](#g-snes) parts (15 mm face-button diamond), sold as repair spares; one D-pad piece and one ABXY piece per [seat](#g-seat) | [hardware/membrane/README.md](hardware/membrane/README.md) |
| 2026-09-21 | Bare RP2350B family chip, not a module | No RP2350B module exists that [JLCPCB](#g-jlc) will place; the Pico family is placeable but has too few GPIO | 10-production-board.md section 8 |
| 2026-09-21 | 3.5 in 320x480 [IPS class](#g-glass-class) | The only size with datasheet-backed, stocked [IPS](#g-ips) supply; panels hold their own [frame memory](#g-framebuffer), so no framebuffer is needed | 10-production-board.md section 9 |
| 2026-09-23 | The case owns every position; the board follows | One source of truth for [outline](#g-board-outline), window and contact centres, imported into [KiCad](#g-kicad) | [hardware/prod_case/README.md](hardware/prod_case/README.md) |
| 2026-09-24 | [Reference design](#g-ref-a) built by Claude in parallel with Alex's hand-made board | Alex keeps learning on his own board while a complete, orderable version exists | [hardware/ref_design/PLAN.md](hardware/ref_design/PLAN.md) |
| 2026-09-24 | [2-layer](#g-layers) board with Alex's approved cost changes: [TP4056](#g-tp4056) with [load sharing](#g-load-sharing), [ME6211](#g-me6211) [LDO](#g-ldo), [RP2354B](#g-rp2354b) (flash inside), [PWM](#g-pwm) audio into an [FM8002A](#g-fm8002a), no [cover lens](#g-cover-lens), every hole 0.3 mm or larger | Each change was costed and approved by Alex; for example the LDO saves $1.40 a unit against the [buck-boost](#g-buck-boost), and the [bare board](#g-bare-board) fell from $9.74 to $3.34 a unit at 10 | PLAN.md decisions table |
| 2026-09-24 | EastRising ER-TFT035IPS-6 panel in a [ZIF connector](#g-zif) | Published [datasheet](#g-datasheet) and [3D model](#g-3d-model), sold by its maker, plugs in instead of being soldered | PLAN.md; [docs/design_review.md](hardware/ref_design/docs/design_review.md) |
| 2026-09-25/26 | Keep a [Schottky](#g-schottky) across the cell switch, a [bleeder](#g-bleeder) on [VBUS](#g-vbus), a series resistor on the USB input | Simulated unplug and [hot-plug](#g-hot-plug) events dipped [3V3](#g-3v3) under the flash's minimum or [rang](#g-ringing) [VSYS](#g-vsys) near the [regulator](#g-regulator)'s limit | design_review.md |
| 2026-10-07 | Discrete [backlight](#g-backlight) [current sinks](#g-current-sink) replace the [AP3156](#g-ap3156) | The AP3156 ran out at JLC and [LCSC](#g-lcsc); the sinks use only [Basic or Preferred parts](#g-basic-ext) and were [simulated](#g-ngspice) before [layout](#g-layout) | design_review.md; [board/sim/](hardware/ref_design/board/sim/) |
| 2026-10-09 | Cell under the board, on the [tub](#g-tub-lid) floor | Alex's call. The alternative, thinner at the ends, put the cell over the [MCU](#g-mcu) core, which the [validation plan](#g-validation-plan) flagged for heat; it stays on branch `ref-design-cell-on-board` | PLAN.md |
| 2026-10-19 | Start and Select per seat; seat 1's Start is the power button | Matches the DS Lite layout; no game button doubles as power | PLAN.md |
| 2026-10-20 | Speaker on a plug-in [header](#g-header), playing out of the side wall | A floor grille faces the table; no soldering at assembly | PLAN.md |

## 5. Software architecture

### 5.1 Shape

One [`no_std`](#g-no-std) [core](#g-core) holds the [battle engines](#g-battle-engine) and the whole game: menus, lobby, input rules, screen
decisions and rendering. Three thin [platforms](#g-platform) wrap it.

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
| [`gen1_battle`](crates/gen1_battle/SPEC.md) | [Gen 1](#g-gen) [singles](#g-singles) engine, cartridge-accurate as [Showdown](#g-showdown) models it, about 800 B of RAM per battle |
| `gen3_battle` | Gen 3 singles engine: [Special split](#g-special-split), [natures](#g-ev-iv-nature), IVs and EVs, [17 types](#g-type), [weather](#g-weather), [abilities](#g-ability), [held items](#g-held-item) |
| `battle_types` | The type enum shared by both engines |
| `mega_blastoise_core` | Everything the device does, shared verbatim by the platforms |
| `mega_blastoise_fw` | [Firmware](#g-firmware) for the [stripboard v2](#g-stripboard-v2) board ([Embassy](#g-embassy) on [RP2350](#g-rp2350)) |
| `mega_blastoise_web` | The same device in a browser page; its own [WASM](#g-wasm) [workspace](#g-workspace) |
| `mega_blastoise_test` | Interactive stdin battle, unit and integration tests, the [Showdown parity](#g-parity) and [fuzz suites](#g-fuzz), a renderer that dumps every screen to image files |

More detail: [docs/architecture/](docs/architecture/README.md) and the
"where does new code go" guide in [08-feature-map.md](docs/architecture/08-feature-map.md).
Toolchain: [nightly Rust](#g-nightly) pinned in [rust-toolchain.toml](rust-toolchain.toml).

### 5.2 The battle engines

**History.** The project began on upstream [`battler`](https://github.com/jackson-nestelroad/battler),
an open-source Rust Pokemon engine, checked out as a [git submodule](#g-submodule) (Alex's fork) at `battler/`. On
the [RP2040](#g-rp2040) it ran out of [heap](#g-heap), so `gen1_battle` was written as a drop-in replacement on
2026-05-15. [`battler`](#g-battler) is still a dependency: core's [build script](#g-build-script) reads its Gen 1 data files and
the data store uses its types, but no battle runs through it.

**Gen 1** (`gen1_battle`, about 4,400 lines) models Gen 1 [random battles](#g-randbat) as Showdown's Gen 1 mod
does, including the cartridge's glitches. Its [SPEC.md](crates/gen1_battle/SPEC.md) documents
memory layout, formulas, turn structure and the deliberate gaps. By 2026-08-16, 164 of its 165
moves were in the fuzz pool.

**Gen 3** (`gen3_battle`, about 11,400 lines) was added on 2026-08-15 as a sibling [crate](#g-crate), so a Gen 1
battle never enters it. By 2026-08-16 it modelled 75 of the 76 Gen 3 abilities and the first 47
held items (more followed on 08-17), with weather, screens, multi-turn moves and the [residual
order](#g-residual) as Showdown runs them. Abilities and items are answered by small modules
([ability.rs](crates/gen3_battle/src/ability.rs), [item.rs](crates/gen3_battle/src/item.rs)) that
each pipeline stage asks, rather than an [event bus](#g-event-bus).

**The reference.** Pokemon Showdown is the authority. Its source is installed locally by
`tools/showdown-parity` and read before a mechanic is implemented; the fuzzer then confirms. The
last engine commit (2026-08-19) widened the fuzz pool to every damaging move, found a backlog of
480 failing [seeds](#g-seed) in 600, and brought it to 5, which are not yet explained.

**On the device**, both engines speak one protocol to core: a request becomes generation-neutral
slot options for the input layer, and the chosen move or switch goes back as a command string.
Teams come from Showdown's random battle sets ([randbat data](https://pkmn.github.io/randbats/)
for Gen 1; the [vendored](#g-vendored) Gen 3 sets for Gen 3). The device's Gen 3 draft does not yet hand out
abilities, held items or EV spreads ([draft.rs](crates/gen3_battle/src/draft.rs)), so device
battles do not yet exercise what the engine models.

### 5.3 The zero-drift rule

The firmware and the web client must be indistinguishable in everything a player sees and does.
This is the project's most important software property, and it is enforced by where code is
allowed to live ([CLAUDE.md](CLAUDE.md)):

- **Core decides.** What any screen shows, how any input behaves, and what the device does next
  are decided only in `mega_blastoise_core`.
- **Platforms only do raw IO**: classifying a press as a tap or a 500 ms hold, carrying typed
  console lines, presenting rendered frames, printing log lines, and supplying a [monotonic clock](#g-monotonic),
  sleeps and [entropy](#g-entropy).
- **The device wins.** When the two targets disagree, the firmware on the [Pico](#g-pico) is the source of
  truth.

If a change needs battle UX behaviour written in firmware- or web-only code, it belongs in core
instead, driven from both platforms. The rule was written on 2026-07-09 after the two targets had
drifted, and extended on 2026-08-27 to orchestration after the first single-screen web client
turned out to hold the whole game loop.

Which core module owns what:

| Module | Owns |
|---|---|
| [`oled_ctl.rs`](crates/mega_blastoise_core/src/oled_ctl.rs) | [`OledController`](#g-oledcontroller) and the `Screen` enum: which screen each [seat](#g-seat) is on, flashes, narration timing, the redraw rule (the name dates from the two-OLED device) |
| [`choice_collect.rs`](crates/mega_blastoise_core/src/choice_collect.rs) | [`ChoiceCollector`](#g-choicecollector): prompts, accept and reject with messages, the waiting screen, take-back, the 1 s both-ready [grace](#g-grace), invalid flashes, long-press detail views, the typed-line grammar |
| [`device_session.rs`](crates/mega_blastoise_core/src/device_session.rs) | Outside a turn: which menu owns which [half](#g-half), shared options, each seat's cursor and overlays, what a press means in each state |
| [`device_runner.rs`](crates/mega_blastoise_core/src/device_runner.rs) | The game loop ([attract](#g-attract-demo), lobby, ready sequence, battle, recap) and the [`DevicePort`](#g-devicerunner) [trait](#g-trait) a platform implements |
| [`cursor_nav.rs`](crates/mega_blastoise_core/src/cursor_nav.rs), [`menu.rs`](crates/mega_blastoise_core/src/menu.rs), [`battle_log.rs`](crates/mega_blastoise_core/src/battle_log.rs) | D-pad cursor, Gen picker and options, the scrollable log |
| [`display_color.rs`](crates/mega_blastoise_core/src/display_color.rs), [`device_view.rs`](crates/mega_blastoise_core/src/device_view.rs), [`device_ui.rs`](crates/mega_blastoise_core/src/device_ui.rs), `move_anim.rs` | Rendering one half, composing two halves (far one rotated) into the 320x480 frame, the shared battle band and attack effects |
| [`cli_parse.rs`](crates/mega_blastoise_core/src/cli_parse.rs) | The console grammar, shared by the USB console and the web console |

### 5.4 How a turn flows

1. The engine writes battle log lines; core parses them into typed [`BoardEvent`](#g-boardevent)s (damage, faint,
   switch-in, status and so on) and turns the engine's request into a prompt.
2. `OledController` decides each seat's screen from those events; `DeviceRunner` paces narration.
3. The renderer draws each half from the `Screen` plus per-seat context (foe [HUD](#g-hud), cursor) and
   composes the 320x480 frame. The platform's `present` hook only moves pixels: the firmware
   sends the [16x16 tiles](#g-dirty-tiles) that changed; the web page [blits](#g-blit) the frame to a canvas.
4. A press comes in from the platform as a classified [`DeviceEvent`](#g-deviceevent). [`DeviceSession`](#g-devicesession) and
   `CursorNav` turn it into a pad event; `ChoiceCollector` validates it and, once both seats are
   ready and the grace window has passed, hands the engine its command strings.

### 5.5 Data and assets

Core's [build script](crates/mega_blastoise_core/build.rs) generates static tables at compile
time, so nothing is loaded at run time and there is no file system: Gen 1 data from `battler`'s
data files, the random battle roster, move descriptions from Showdown's text data, 1-bit [sprites](#g-sprite)
for the old mono renderer, and colour sprites for every Gen 3 species (Emerald art from the
[PokeAPI](#g-pokeapi) sprite repository, packed losslessly as 4 bits per pixel with a per-species [palette](#g-palette)).
Downloads are cached under `crates/mega_blastoise_core/vendor/` so later builds are offline.
Gen 3 art is used for both generations. Move icons for the attack effects come from
[scripts/gen_move_sprites.py](scripts/gen_move_sprites.py).

### 5.6 Firmware

`mega_blastoise_fw` runs on the Pico 2 in the stripboard v2 board. A plain `cargo build` in the
crate builds the device; [03-firmware-runtime.md](docs/architecture/03-firmware-runtime.md) has
the startup and task list. In short: Embassy [async tasks](#g-async-task) for the [ST7796](#g-ili9488) [panel](#g-panel) (PIO [SPI](#g-spi) with [DMA](#g-dma)),
the 16 buttons ([debounce](#g-debounce), A tap versus hold), the [USB CDC](#g-usb-cdc) console, battery sensing and the sprite
bob, and [`board_v2.rs`](crates/mega_blastoise_fw/src/board_v2.rs) implements core's
`DevicePort` and runs `DeviceRunner` forever. A [panic](#g-panic) lights the Pico's GP25 LED and its message
survives a reset. [Features](#g-feature): `input-debug` (held-key overlay), `mem-profile`, `trace`.

There is no firmware yet for the [reference design](#g-ref-a): it needs the `rp235xb` chip feature, its [pin
map](#g-pin-map), an ILI9488 panel driver ([18-bit colour](#g-18-bit) over SPI, unlike the ST7796), audio and power control.
[Bring-up](#g-bring-up) tools (an RP2350 blink and a recovery-reset helper) are in
[firmware/bringup/](firmware/bringup/rp2350_blink/README.md).

### 5.7 Web build and site

`mega_blastoise_web` compiles core to WASM. [`src/lib.rs`](crates/mega_blastoise_web/src/lib.rs)
is the web's `DevicePort` plus exports that turn pointer and keyboard input into `DeviceEvent`s;
[`www/device.html`](crates/mega_blastoise_web/www/device.html) draws a [DS Lite](#g-ds-lite) style console with
both seats in one window and an in-page console. URL parameters such as `?demo=1`, `?ai=1` and
`?seed=N` drive it for testing.

[`.github/workflows/pages.yml`](.github/workflows/pages.yml) builds and deploys on every push to
`main`:

| Path on [Pages](#g-github-pages) | What |
|---|---|
| `/` | Front page, from [site/index.html](site/index.html) |
| `/battler/device.html` | The game |
| `/battler/ui_flow.html` | The single-screen UI design notes |
| `/assembly/` | [Exploded 3D view](#g-exploded) of the reference design, built locally by [hardware/ref_design/viewer/](hardware/ref_design/viewer/) and kept fresh by a [pre-push hook](#g-pre-push) |
| `/assembly-cell-on-board/` | The same view of the earlier cell-on-board layout |

### 5.8 Host harness

`cargo run -p mega-blastoise-test` plays a battle over stdin through the same pipeline as the
device. `cargo run -p mega-blastoise-test --bin screens -- <dir>` renders every screen to [PPM](#g-ppm)
files with no hardware or browser.

## 6. Hardware

### 6.1 The generations compared

| | [v1](#g-v1) [PCB](#g-pcb) | [Stripboard v2](#g-stripboard-v2) | [Reference design](#g-ref-a) (REF-A) |
|---|---|---|---|
| [MCU](#g-mcu) | [Pico](#g-pico) ([RP2040](#g-rp2040)) | Pico 2 ([RP2350A](#g-rp2350), 4 MB flash) | [RP2354B](#g-rp2354b) on the board (2 MB flash inside) |
| Display | Two 1.3 in 128x64 mono [OLEDs](#g-oled) | One 3.5 in [ST7796](#g-ili9488) 320x480 [module](#g-module) | One 3.5 in ER-TFT035IPS-6 [IPS panel](#g-ips), ILI9488, in a [ZIF connector](#g-zif) |
| Inputs | Button [matrix](#g-button-matrix), move buttons at screen corners | 16 [tactile switches](#g-tactile), direct [GPIO](#g-gpio), 9 mm printed caps | 20 [membrane](#g-membrane) contacts under genuine-style [DS Lite](#g-ds-lite) silicone and caps |
| Per [seat](#g-seat) | 4 moves, 3 party buttons | D-pad cross and A, B, INFO, MENU cross | D-pad, A/B/X/Y, Start, Select |
| Feedback | LED strips for [HP](#g-hp) | Screen only | Screen and a speaker |
| Power | 2xAA in the final case | 2000 mAh [LiPo](#g-lipo), [TP4056](#g-tp4056) module, slide switch | About 700 mAh [Li-ion cell](#g-li-ion), on-board charger with [USB load sharing](#g-load-sharing), [soft power](#g-latch) |
| Built | Yes | Yes | No |

### 6.2 Stripboard v2: the board that runs today

One 24x56 [stripboard](#g-stripboard) (about 61 x 142 mm) carries the Pico 2, the [panel](#g-panel) standing portrait on a
socket in the middle, and a band of switches at each end: a D-pad cross on the player's left and
an action cross on their right. Every switch has its own GPIO with the internal [pull-up](#g-pull-up), so there
is no [ghosting](#g-ghosting). A [protected](#g-protection) TP4056 module charges a [103450](#g-403450) LiPo mounted on the back; a [Schottky
diode](#g-schottky) lets the charger's USB and the Pico's USB be plugged in together, and a [divider](#g-divider) lets the
[firmware](#g-firmware) read the cell. There is no buzzer.

Everything physical is generated by `layout.py`: the build guide, the [BOM](#g-bom), the [KiCad](#g-kicad) [schematic](#g-schematic) and
board, and solid models. The guide is [BUILD.md](hardware/stripboard_v2/BUILD.md) (its section 10
is the firmware [pin map](#g-pin-map)), the staged build with a test after each stage is
[ASSEMBLY.md](hardware/stripboard_v2/ASSEMBLY.md), and the models are in
[model/](hardware/stripboard_v2/model/README.md). The cluster spacing is cramped (about 30 mm),
which Alex accepted for a prototype.

### 6.3 Requirements for a sellable board

[10-production-board.md](docs/archive/architecture/10-production-board.md) (2026-09-17 to 09-21) records
what changes when the device becomes a product: no [2.54 mm grid](#g-pitch) or stripboard [outline](#g-board-outline), ergonomics
re-decided from scratch, membranes instead of tactile switches, a bare MCU placed by the [fab](#g-fab), a
proper charger and protected cell, one [USB-C](#g-usb-c) for charge and data, no radio, an IPS panel, and
audio for the first time. Several of its first choices (a power-path charger [IC](#g-ic), a [buck-boost
regulator](#g-buck-boost), an [I2S](#g-i2s) amplifier, an [anti-glare cover lens](#g-cover-lens)) were later replaced in the reference design
by cheaper parts on 2026-09-24; the reference design's [PLAN.md](hardware/ref_design/PLAN.md) is
the current word.

### 6.4 Alex's production board

`hardware/prod_board/` and [`hardware/prod_case/`](hardware/prod_case/README.md) are Alex's own
learning project in KiCad and [OpenSCAD](#g-openscad). The case's `layout.scad` owns the outline, glass window
and every contact centre, and a script places the board's 16 contacts from it. On `main` the board
has its outline and 16 contacts placed; the schematic is not started. This folder is Alex's:
other work reads it and never edits it.

### 6.5 The reference design (REF-A)

The reference design now has a section of its own: [section 7](#7-the-reference-design-ref-a).

### 6.6 Membranes and the DS Lite research

The membrane decision is documented in [hardware/membrane/README.md](hardware/membrane/README.md):
the donors surveyed ([SNES](#g-snes), DS Lite, others), the DS Lite geometry measured from a [breakout board](#g-breakout)'s
[gerbers](#g-gerbers), the contact pattern, licence traps in other projects' [footprints](#g-footprint), and an open question:
fine [gold combs](#g-comb) need [ENIG](#g-enig) or [hard gold](#g-hard-gold), while [carbon ink](#g-carbon-ink) (more durable) needs a coarser pattern.

[hardware/ds_lite/](hardware/ds_lite/README.md) is a research packet on how the DS Lite is built and
why it feels as it does, every number sourced and labelled by how it was obtained. Its key points
for this project: density (a DS Lite works out to about 1.03 g/cm3, so about 160 g for a case this
size), a 13.5 mm lower half, rounded edges, and the [seam](#g-seam) on the face. Its recommendations are
inputs, not decisions. Alex owns a genuine DS Lite for scale and [caliper](#g-caliper) checks; the [dossier](#g-dossier)'s
"Measure your own unit" section lists them.

## 7. The reference design (REF-A)

A complete design package to order a first prototype: [schematic](#g-schematic), [routed](#g-routing) [2-layer](#g-layers) board, [JLCPCB](#g-jlc)
[fab](#g-fab) and assembly files, printable case, [fit kit](#g-fit-kit), and documents to build, bring up and review it.
It is generated: [`board/design.py`](hardware/ref_design/board/design.py) holds every part, value,
[LCSC](#g-lcsc) number and [net](#g-net); [`board/placement.py`](hardware/ref_design/board/placement.py) places them;
[`case/case.scad`](hardware/ref_design/case/case.scad) is the source of every mechanical position.
Scripts build the schematic, board, routing, checks, fab outputs and case from those three files
(the pipeline is in the [README](hardware/ref_design/README.md)). Never edit the generated [KiCad](#g-kicad)
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
schematic [sheet](#g-sheet) by sheet, in the schematic's own order. Where design.py and another document
disagree, this section follows design.py.

### 7.1 Overview

**Power and battery.** One [403450](#g-403450) [Li-ion cell](#g-li-ion) (about 700 mAh) lies on the [tub](#g-tub-lid)'s floor under the
board and plugs in on a [Molex PicoBlade](#g-picoblade) connector. A [TP4056](#g-tp4056) charges it from [USB-C](#g-usb-c) while USB also
feeds the system directly ([load sharing](#g-load-sharing)), with a board [thermistor](#g-thermistor) over the cell limiting charging
to a safe temperature window. An [ME6211](#g-me6211) [LDO](#g-ldo) makes [3V3](#g-3v3); because the [ADC](#g-adc)'s reference is the 3V3 [rail](#g-rail),
the [firmware](#g-firmware) must shut down at a computed 3.5 V. Power is a [latch](#g-latch): pressing [seat](#g-seat) 1's Start
turns the [regulator](#g-regulator) on, the [MCU](#g-mcu) then holds itself on, and releasing the hold turns the device
off. A reset or crash drops the hold, so on battery a crash turns the device off, which is the
intended safe failure. Off-state drain is about 20 [uA](#g-units). The unplug hand-over and [hot-plug](#g-hot-plug)
behaviour were [simulated](#g-ngspice), not yet measured. Battery life has not been measured.

**MCU.** The [RP2354B](#g-rp2354b) is the [RP2350B](#g-rp2350) [die](#g-die) with 2 MB of flash stacked inside, so there is no flash
chip to route. Its core circuit and [layout](#g-layout), including the copper around the internal 1.1 V
[switcher](#g-switcher), are copied from Raspberry Pi's RP2350B [minimal design](#g-minimal-design) ([MIT licence](#g-mit)), so the most
layout-sensitive part of the board is theirs. Pins are assigned so each faces where its net goes,
which is what lets two layers route completely.

**Display and backlight.** The 3.5 in 320x480 [IPS panel](#g-ips)'s [FPC](#g-fpc) folds under it into a bottom-contact
[ZIF connector](#g-zif) that JLC places, so no hand soldering and a cracked [panel](#g-panel) swaps in a minute. Its [tear
signal](#g-tearing) goes to a [GPIO](#g-gpio). Its six [backlight](#g-backlight) LEDs are driven by two [op-amp](#g-op-amp) [current sinks](#g-current-sink) set by a
[filtered](#g-low-pass) [PWM](#g-pwm), so the LED current is DC and stays constant down to the cutoff. The case is built
around a fixed panel bay, so another panel of the same [glass class](#g-glass-class) changes only the panel numbers,
the connector and its [pin map](#g-pin-map).

**Audio.** PWM from a GPIO through a two-stage RC filter into an [FM8002A](#g-fm8002a) amplifier and a 15 x 10 mm
speaker, which plugs into a [header](#g-header) and plays out of the case's left wall. The target is [GBA-like](#g-gba)
sound. A [mixer](#g-mixer), when written, lives in [core](#g-core) so the browser sounds like the device.

**Buttons.** 20 [membrane](#g-membrane) contacts (two interleaved [gold combs](#g-comb) each) under [DS Lite](#g-ds-lite) silicone and caps:
per seat a D-pad, A/B/X/Y, Start and Select, each on its own GPIO with no [matrix](#g-button-matrix). The contact
[footprints](#g-footprint) are generated and checked by the tools in [hardware/membrane/](hardware/membrane/README.md).

**Debug.** [SWD](#g-swd) and a [UART](#g-uart) on [JST SH](#g-jst) connectors in the [Raspberry Pi Debug Probe](#g-debug-probe) pinout, SWD again on
six [pogo pads](#g-pogo) on the back, [BOOTSEL](#g-bootsel) and RESET buttons under pinholes in the lid, [probe pads](#g-probe-pad) on every
rail and on key signals, [cut jumpers](#g-solder-jumper) to isolate each power block, and [revision-strap](#g-strap) resistors.
No LEDs (an approved decision).

**Board.** 2 layers, 1.6 mm, [ENIG](#g-enig) (the silicone [pills](#g-pill) press straight on the combs), every hole
0.3 mm or larger to stay on JLC's [standard process](#g-standard-process), assembled on one side.

### 7.2 Case and printing

About 75 x 156 mm, 13.4 mm thick at the screen and 11.1 mm at the button
decks: close to a DS Lite's lower half in width (73.9 mm) and thickness (about 13.5 mm), and
longer than it (133 mm). A tub, a lid with the panel in a pocket, and one [deck
insert](#g-deck-insert) printed twice, held by six [M2](#g-m2) screws as on a DS Lite, with four hooks holding the lid while
it is screwed. The shell follows the DS Lite: rounded back, 4.3 mm [plan corners](#g-plan-view), the [seam](#g-seam) on the
face. Genuine-pattern DS Lite caps stand 1.3 mm [proud](#g-proud), using numbers measured from open models
([case/DSLITE.md](hardware/ref_design/case/DSLITE.md)). Parts print without [supports](#g-supports) where
possible, in [PLA](#g-pla), on Alex's Snapmaker U1 (a Bambu P1S package exists too):
[print_package_README.md](hardware/ref_design/case/print_package_README.md). A printable
[fit kit](hardware/ref_design/case/fitkit/README.md) (board mock, part dummies, [coupons](#g-coupon)) tests the
real parts before boards are ordered. The case modelling method (render, cross-section and
assert, [headless](#g-headless)) is in [docs/hardware/openscad-methodology.md](docs/hardware/openscad-methodology.md).

### 7.3 The schematic: how the sheets connect

The schematic is a [root sheet](#g-root-sheet) and eight [sub-sheets](#g-sub-sheet) in KiCad 10's format. Since 2026-10-20
[`board/tools/build_sch.py`](hardware/ref_design/board/tools/build_sch.py) writes them directly
from design.py (before that they went through [Konnect](#g-konnect)), and
[`sch_sheets.py`](hardware/ref_design/board/tools/sch_sheets.py) lays each sheet out by hand as
functional blocks: parts wired to each other, [decoupling](#g-decoupling) beside its pins, series parts in a chain.
design.py decides how each net is drawn: a power rail gets a [power symbol](#g-power-symbol), a net used on more than
one sheet gets a [global label](#g-global-label), and a net used on one sheet only gets a [local label](#g-local-label) (KiCad prefixes
it with the sheet's name). The drawing is traced and compared with design.py before it is
written; afterwards `check_netlist.py` compares KiCad's own [netlist](#g-netlist) with design.py, and KiCad's
[ERC](#g-erc) runs.

In numbers: design.py has 173 [reference designators](#g-ref-des), of which JLC places 118. The other 55 are
copper features or [empty footprints](#g-unfitted): the 20 contacts, 16 test pads, 5 solder jumpers, J7's pads,
6 holes, 3 [fiducials](#g-fiducial), and 4 unfitted footprints (C41-C43, R48). Its 101 named nets are 6 power
rails, 48 global labels and 47 local nets. Eight [`#FLG`](#g-power-flag) power flags are not parts: they tell ERC
that a rail fed through a jumper, diode or [FET](#g-fet) is driven.

The sheets, with links to their walk-throughs:

| Sheet (file) | [Title block](#g-title-block) | Walk-through |
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

**Power rails.** Six nets get power symbols. [VSYS](#g-vsys) and [VBAT](#g-vbat) use KiCad's VCC [symbol](#g-symbol) renamed through
its value.

| Rail | Made by | Level | Used by |
|---|---|---|---|
| [VBUS](#g-vbus) | the USB-C receptacle J1 | 5 V from the source; a simulated hot plug peaks at 6.7 V | the charger U2 and its TEMP and [CE](#g-enable) networks, D5 and D6 (load sharing), the [bleeder](#g-bleeder) R38, and R14 (USB holds the device on) |
| VSYS | D5 from VBUS, or Q2 and D7 from the cell | about 4.6 V on USB, the cell's voltage otherwise | the 3V3 regulator U3, the backlight [anode](#g-anode) (through JP2), the amplifier (through JP3), the power contact SW21 |
| VBAT | the cell, through J2 and JP5; U2 charges it | 4.2 V full; the firmware shuts down at a computed 3.50 V | Q2, D7, the [divider](#g-divider) R17/R18 |
| +3V3 | U3, through JP1, only while PWR_EN is high | 3.3 V | U4, the panel's logic (through JP4), the backlight op-amp U6, U1's reference, the [pull-ups](#g-pull-up), J7 |
| +1V1 | U4's internal switcher, through L2 | 1.1 V | U4's core ([DVDD](#g-iovdd)) only |
| [GND](#g-ground) | | | every sub-sheet; [poured](#g-pour) on both layers and stitched by [vias](#g-via) on a 5 mm grid |

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
| USB_Charger | USB_DP, USB_DM | USB data (the series resistors are on the MCU sheet, the [ESD array](#g-esd-array) on USB_Charger) |
| | CC1_SENSE, CC2_SENSE | the USB-C [CC lines](#g-cc) through 10k, into ADC6 and ADC5 |
| | CHG_STAT_N, CHG_DONE_N, CHG_OFF | the charger's two status outputs, and the MCU's charge pause |
| Power | PWR_HOLD, PWR_BTN_N, VBAT_SENSE | the MCU's hold on the latch, the power button as the MCU sees it, the cell voltage halved |
| MCU_Power | VREG_AVDD, VREG_LX | the core switcher's filtered analogue supply and its [inductor node](#g-lx) |
| Debug | SWCLK, SWDIO, [RUN](#g-run), QSPI_SS, UART_TX, UART_RX | SWD, reset, the BOOTSEL strap, the UART console |
| Display | LCD_SCK_MCU, LCD_MOSI_MCU, LCD_MISO, LCD_CS, LCD_DC, LCD_RST, LCD_TE, BL_PWM | the panel's [SPI](#g-spi) bus and control lines, and the backlight's set point |
| Audio | AUDIO_PWM, AMP_EN | the audio PWM, and the amplifier's enable |
| Buttons | P1_UP to P1_SELECT (9), P2_UP to P2_START (10) | one net per contact; seat 1's Start is PWR_BTN_RAW instead |

SCK and MOSI change name at their series resistors on the Display sheet: `_MCU` on the chip's side,
plain on the panel's.

### 7.4 Sheets and mechanical parts (root sheet)

The root sheet holds a [title note](#g-title-note), the eight [sheet symbols](#g-sheet-symbol) (each with a [four-line note](#g-sheet-note) of the
blocks inside it) and the parts that have no circuit. Neither kind of mechanical part is in JLC's
[BOM](#g-bom) or [placement file](#g-placement-file): there is nothing to place.

| Parts | Role | Why |
|---|---|---|
| H1-H6 | 2.2 mm holes for M2 screws | the case is held by six M2 screws through the board, which clamp tub and lid as on a DS Lite; the positions come from case.scad (through `positions.json` and placement.py), so the board always matches the case |
| FID1-FID3 | 1 mm fiducials with a 2 mm [mask opening](#g-solder-mask) | optical alignment marks for JLC's [placement machines](#g-pick-place); the router keeps 0.6 mm of copper clear around each |

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

- **USB in:** VSYS is VBUS less D5's [drop](#g-forward-voltage), about 4.6 V ([bring-up](#g-bring-up) expects 4.5 to 4.7 V). LS_GATE
  sits at VBUS less D6's drop, at or above VSYS, so Q2 is off; Q2's [body diode](#g-body-diode) and D7 are [reverse
  biased](#g-reverse-bias).
- **USB out:** R6 pulls LS_GATE to ground and Q2's channel joins the cell to VSYS (at most
  85 mOhm with 2.5 V of [gate](#g-gate) drive).
- **The unplug.** At an unplug C1 holds VBUS up, and LS_GATE can only fall as fast as VBUS does.
  R38 (1k) drains VBUS, so Q2's channel takes over 1.5 to 3.3 ms after the cable leaves (about 2 to
  3 ms in design.py's comment; tens of ms without R38). Until then D7 carries VSYS at the cell less
  about 0.34 V, and C9 covers the step.

**Parts.**

| Parts | Role | Why this part or value |
|---|---|---|
| J1 (HRO TYPE-C-31-M-12) | USB 2.0 Type-C receptacle: VBUS, CC1 and CC2, [D+ and D-](#g-dplus) (both rows joined), shield to GND; the SBU pins are left open | one USB-C for charging, firmware and console (the [stripboard](#g-stripboard) had two USB ports) |
| R1, R2 (5.1k) | Rd pull-downs on CC1 and CC2 | they mark the board as a [power sink](#g-source-sink): a Type-C source turns VBUS on only when it sees them. There is no [USB PD](#g-usb-pd) |
| R3, R4 (10k); C42, C43 (empty) | CC1 and CC2 into ADC6 and ADC5 (GPIO46, GPIO45) | firmware reads the source's current offer; the 10k keeps a CC line shorted to VBUS off the pin. C42 and C43 are empty [1206](#g-imperial) footprints for filter capacitors, fitted by hand only if bring-up shows noise |
| U1 (SRV05-4A, [SOT-23-6](#g-sot23)) | ESD steering array on D+ and D-; its other two lines are unused | 1.0 pF, 15 kV contact and air discharge, next to the connector. Its reference pin is on 3V3, not VBUS (see Protection). hongjiacheng's part since 2026-10-20 (Alex): [Preferred](#g-basic-ext) at JLC, so no loading fee. Before it: Semtech's Extended SRV05-4.TCT from 2026-10-07, when ProTek's part ran out |
| R41 (2.2 ohm 0603), C1 (10 uF 25 V) | the VBUS [bulk capacitor](#g-bulk-cap), behind a series resistor | 10 uF is USB's limit for bulk capacitance on VBUS. The resistor is the TP4056 [datasheet](#g-datasheet)'s fix for a [ceramic input capacitor](#g-ceramic-cap) [ringing](#g-ringing) when a live source is plugged in (it asks 1.5 ohm; 2.2 is the nearest Basic part). Added 2026-09-26 |
| R38 (1k 0603) | VBUS bleeder | it drains VBUS after an unplug so Q2 takes over in milliseconds, and it holds D5's [leakage](#g-leakage) away from PWR_EN (see Protection). It costs 5 mA (25-28 mW) with USB in, so it is an 0603 at 25-30% of its rating. 10k on 2026-09-25, 1k after the third audit round the same day, 0603 on 2026-09-26 |
| U2 (TP4056, [ESOP-8](#g-soic)) | [linear Li-ion charger](#g-linear-charger) to 4.2 V | a fifth of the BQ24074's price with no stock risk (2026-09-24). Its [thermal pad](#g-thermal-pad) has six 0.3 mm vias into the ground pours on both layers; it lowers its own current when its die passes about 145 C, so heat slows the charge rather than hurting the chip |
| R5 (3.3k) | sets the charge current | 1100 or 1200 V (by datasheet revision) / 3.3k = 333-364 mA, about [0.5 C](#g-c-rate) for the 700 mAh cell, the usual standard rate for small pouch cells. It gave 460-500 mA until 2026-10-05, which the [validation plan](#g-validation-plan) judged too high for this cell |
| R39 (5.6k), R40 (150k), TH1 (10k NTC, B3380); C41 (empty) | the charger's TEMP input | the TP4056 charges only while TEMP sits between 45% and 80% of VBUS. With TH1 in parallel with R40 that is 1 to 46 C (-3 to 49 C across the chip's threshold spread). The 403450 has no thermistor wire, so TH1 sits on the board over the cell, reading it through the board, about 45 mm from U2 so that U2's 0.65 W while charging doesn't trip it. Fed from VBUS: about 0.6 mA from USB, nothing from the cell. Added 2026-09-25 |
| R43 (100k), Q4 (2N7002), R44 (4.7k) | the charger's enable (CE) | R43 pulls CE to VBUS, so the charger charges whenever USB is in and nothing drives Q4, even with a blank or crashed MCU. CHG_OFF (GPIO19) high turns Q4 on and pauses charging (a weak USB port, a warm case). R44 holds Q4 off through any processor state, at 4.7k because [erratum](#g-erratum) RP2350-E9 needs 8.2k or less. Added 2026-10-05 |
| R9, R10 (100k to 3V3) | pull-ups on the [open-drain](#g-open-drain) STDBY (CHG_DONE_N, GPIO18) and CHRG (CHG_STAT_N, GPIO17) | the MCU reads the charger's state; there are no charge LEDs (an approved decision) |
| R47 (0.1 ohm 1206) | damper in D5's path | a live plug-in rings through D5 into the VSYS capacitors: with no cell (or a [tripped pack](#g-protection)) VSYS peaked at about 6.6 V against the FM8002A's 6 V maximum, about 5.7 V with R47. It costs 50 mV of VSYS at 0.5 A. Added 2026-10-05 |
| D5 (B5819W, 1 A [Schottky](#g-schottky)) | USB into VSYS | a Schottky keeps the drop small, so VSYS sits near 4.6 V on USB |
| D6 (BAT54WS), R6 (10k) | Q2's gate follows VBUS | with USB in the gate is high and Q2 off; without, R6 pulls it low |
| Q2 (AO3401A) | cell into VSYS | P-channel: with LS_GATE at ground its gate sees the whole cell voltage (about -3.4 V at a 3.5 V cell), more than the -2.5 V at which its [on-resistance](#g-on-resistance) is rated 85 mOhm or less |
| D7 (B5819W) | across Q2 | carries VSYS through the unplug gap at 0.34 V (at 0.25 A) against the body diode's 0.6 V. Added 2026-09-26 after two independent model reviews |
| C9 (22 uF 25 V), C2 (4.7 uF); C3 (10 uF) | VSYS bulk; the charger's output capacitor on VBAT | C9 covers the step at an unplug |
| J2 (Molex PicoBlade 53261-0271), JP5 | the cell connector; a cut jumper from CELL_P to VBAT | J2 is 3.4 mm tall: the 5.5 mm JST PH it replaced on 2026-10-19 needed a pad on the lowered lid. Cells come with PicoBlade ("MX1.25") plugs. Cutting JP5 and bridging it with an [ammeter](#g-ammeter) reads everything the cell gives or takes |

**Protection and failure cases.**

- **Hot plug.** The worst case is an A-to-C cable from an always-on USB-A port: a USB-C source
  turns VBUS on only after it sees the Rd resistors, and ramps it. Simulated with a 2 m cable into a
  board with no cell, R41 brings the peaks from 7.1 to 6.7 V on VBUS (U2's maximum is 8 V) and from
  6.7 to 6.2 V on VSYS (the ME6211's maximum is 6.5 V). R47, added later, brings VSYS from about
  6.6 V to about 5.7 V in design.py's own figures, under the FM8002A's 6 V. With a cell in, the
  peaks are lower. There is no [TVS](#g-tvs) on VBUS: an SMF5.0A's 5.0 V working voltage is below USB's
  5.5 V, and it would barely conduct at these peaks.
- **The unplug gap.** Without D7, Q2's body diode would carry the gap at about 0.6 V (0.25 A, full
  backlight): 3V3 would then be the cell less about 0.71 V, under the internal flash's 2.97 V
  minimum (QSPI_IOVDD) on every unplug with the cell below about 3.7 V. With D7, 3V3 stays above
  2.97 V down to a cell of about 3.45 V (design.py; design_review.md gives 3.42 V, or 3.49 V with
  0.25 A of audio as well), inside the 3.5 V cutoff. This matters because the RP2350 has no 3V3
  [supervisor](#g-brown-out) (its brown-out detector watches the 1.1 V core): a dip would not reset it cleanly, it
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
  threshold, a small [calendar-ageing](#g-calendar-ageing) cost backstopped by the pack's overvoltage cut-off.
- **Charging outside 1 to 46 C** pauses by itself and resumes when TH1 is back in range. CHRG and
  STDBY then both read high, as with no USB; firmware tells the two apart by the CC lines.
- **A weak USB port.** The TP4056 has no input current limit, so a 500 mA default-USB port can be
  asked for the charge current plus the system's draw. Firmware reads the source's offer on the CC
  lines and dims the backlight on a weak one.
- **A reversed cell destroys the charger.** PicoBlade plugs have no standard [polarity](#g-polarity); the red lead
  must land on J2's pin 1 ([silkscreen](#g-silkscreen) `+`), checked before plugging in
  ([assembly.md](hardware/ref_design/docs/assembly.md)).

**Bring-up hooks and firmware.** Probe pads: TP1 (VBUS), TP2 (VSYS) and TP3 (VBAT) are drawn on the
Power sheet, TP7 (LS_GATE) here, and TP15 is a [ground clip](#g-scope) by the charger. JP5 reads the cell's
current; C41-C43 take hand-fitted filter capacitors. Bring-up step 1 (USB, no cell) checks VBUS
and VSYS; step 5 covers the charger, the temperature cutoff with a hair dryer and a cold pack, the
scoped unplug with the cell at 3.5 V, the hot plug with no cell, and PWR_EN at plug-in. Two of
step 0's unpowered expectations predate D7 and R38: the [diode test](#g-diode-test) from TP3 to TP2 reads D7's drop
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
  shut-off, the ME6211's [dropout](#g-dropout), the routed copper; checked by two model reviews), not measured.
- C9's [DC-bias derating](#g-dc-bias), with the unplug and hot-plug simulations re-run, is item 8 of the
  validation plan's go/no-go gate; no document records it as done.
- D5's surge rating and R47's short-time overload against the plug-in [inrush](#g-inrush) are marked unknown in
  the [pin review](#g-pin-review) ([pin_review/d5.json](hardware/ref_design/board/pin_review/d5.json),
  [r47.json](hardware/ref_design/board/pin_review/r47.json)); the TP4056 datasheet misprints the
  [absolute maximum](#g-abs-max) for BAT and the status pins ([u2.json](hardware/ref_design/board/pin_review/u2.json)).
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

**The latch.** U3's enable (PWR_EN) is the [OR](#g-diode-or) of three sources, each through its own Schottky
diode so none can [back-feed](#g-back-power) another:

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
| U3 (ME6211C33, SOT-23-5); C5, C6, C7 (10 uF) | 3V3 LDO from VSYS, enabled by PWR_EN; input and output capacitors | $1.40 a unit cheaper than the [buck-boost regulator](#g-buck-boost) it replaced (2026-09-24). 500 mA maximum, 120 mV dropout at 100 mA: it holds 3.3 V down to a cell of about 3.42 V, and below that 3V3 follows the cell. The load is under 100 mA (the RP2354B up to about 50 mA, the panel's logic about 10 mA), at most 0.13 W in the LDO on USB. The cost: roughly 5-10% of the cell goes unused against the buck-boost |
| JP1 | joins the regulator's output (3V3_LDO) to +3V3 | cut at bring-up to check the regulator alone before the rest of the board sees 3V3, or to feed the rail from a [bench supply](#g-bench-supply) on TP4 |
| D2 (BAT54C, common cathode) | ORs the button and PWR_HOLD into PWR_EN | two Schottky diodes in one SOT-23 |
| R14 (10k), D3 (BAT54WS) | USB into the OR | keeps the device on while USB is in |
| R15 (100k) | PWR_EN pull-down | 100k, not 10k: a 10k cut by about five times how much resistance the Start contact's pill may have before Q1 misses a press. It was 10k for part of 2026-09-25, until the R38 bleeder took over the leakage job |
| C47 (100 nF) | slows PWR_EN's rise from VBUS to about 1 ms (R14 x C47) | at a live plug-in VSYS lags behind R47 and D5, and the ME6211's enable may not go more than 0.3 V above its input (its absolute maximum). Found by the datasheet pin review, 2026-10-05 |
| R42 (4.7k) | PWR_HOLD pull-down | erratum RP2350-E9 can hold a released pad near 2 V; through D2 that would keep the regulator on after a crash and drain the cell in a bag. E9 needs 8.2k or less. Added 2026-10-05 |
| R13 (33k) | PWR_BTN_RAW pull-down | 33k, not 100k: the node tolerates about twice the surface leakage (sweat, condensation) across the Start contact before it lifts PWR_EN (2026-10-05) |
| Q1 (2N7002), R16 (100k to 3V3) | button sense: gate on PWR_BTN_RAW, drain on PWR_BTN_N (GPIO3) | the MCU reads the power button (low = pressed) and no MCU pin ever touches VSYS |
| R17, R18 (470k each), C8 (100 nF) | the cell's voltage halved, into ADC7 (GPIO47) | 470k keeps the divider's drain to 4.5 uA. 100k/100k would add about 30 uA, most of it into the unpowered chip's [clamp](#g-clamp), and still not guarantee the reading against the pad's worst-case leakage. C8 smooths the 235k source (23.5 ms [time constant](#g-time-constant)) |
| TP1-TP5; TP6; TP15, TP16 | probe pads | VBUS, VSYS, VBAT, 3V3 and GND on the back; PWR_EN on top; two GND pads on top for scope ground clips (2026-10-19) |

**Protection and failure cases.**

- **A crash or reset turns the device off on battery.** The reset releases GPIO2 and R42 pulls
  PWR_HOLD low; once Start is up, PWR_EN falls. On USB it stays on. This is the intended safe
  failure: a device that crashes in a bag turns itself off instead of draining the cell.
- **A hang with PWR_HOLD high** keeps the device on with no way to turn it off but unplugging the
  cell, so the firmware enables the [watchdog](#g-watchdog) before any blocking initialisation.
- **The enable above its input at plug-in** (C47) and **leakage lifting PWR_EN** (R38 on the
  USB_Charger sheet, R13 here) are covered above.
- **Off-state drain**, about 18-22 uA in all: the ME6211 off; the amplifier in shutdown (4.2 uA);
  the backlight sinks off; the charger drawing about 2 uA from the cell; the divider 4.5 uA (up to
  9 uA, because with the chip unpowered GPIO47 sits above IOVDD and its ESD diode takes a few uA,
  as on Raspberry Pi's [Pico](#g-pico); accepted); and D5's reverse leakage from VSYS into R38 (1-6 uA at
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
- The validation plan's power-off order: [DISPOFF](#g-panel-commands), then SLPIN, then drop the hold.
- Cell voltage: VBAT = 2 x ADC7 x 3.3 V / 4096, times a gain calibrated once when CHG_DONE_N goes
  low (the cell is then at 4.2 V +-1%). Keep the ADC channel for the whole program (in [embassy-rp](#g-embassy)
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
  [respin](#g-respin) fallback if bring-up step 5 shows 3V3 under 2.97 V at a 3.5 V cell.
- [pinmap.md](hardware/ref_design/docs/pinmap.md) and [bringup.md](hardware/ref_design/docs/bringup.md)
  step 6 still name seat 1's Y as the power button; design.py moved it to Start on 2026-10-19.

### 7.7 RP2354B (sheet MCU)

**What it does.** This sheet holds U4 and what it needs besides power: the [crystal](#g-crystal), the USB series
resistors, the QSPI_SS boot strap's pull-up, the board revision straps, and a label on every GPIO
that leaves for another sheet. Supplies are on the MCU_Power sheet and debug parts on the Debug
sheet.

**The chip.** U4 is an RP2354B in [QFN-80](#g-qfn): the RP2350B die (48 GPIOs) with 2 MB of flash stacked in
the package. Only the flash's [chip select](#g-chip-select), QSPI_SS, comes out, as the BOOTSEL strap; the five [QSPI](#g-qspi)
data and clock pins are left open. Raspberry Pi's RP2350B minimal design (MIT) supplies the core
circuit and its layout, turned 90 degrees.

**Pin assignment.** With two layers there is no room for routes to cross, so each pin faces where
its net goes and, along an edge, the pins are in the order their nets [fan out](#g-escape): seat 1 below the
chip, seat 2 above, the charger and USB to the right, the panel's connector below. Seat 1's cross
and the panel's TE leave down the left edge on GPIO28-32; seat 1's face buttons, the panel's reset,
the backlight and the ADC inputs run along the bottom edge on GPIO40-47. Raspberry Pi's own pin
escapes are copied with the core, so every used pin leaves the chip on their track. GPIO0, 1, 16,
26 and 27 are spare. The full table is [pinmap.md](hardware/ref_design/docs/pinmap.md).

**Parts.**

| Parts | Role | Why this part or value |
|---|---|---|
| U4 (RP2354B) | the microcontroller | no flash chip to route (2026-09-24); a bare chip because no RP2350B [module](#g-module) exists that JLC will place, and the Pico family has too few GPIOs (2026-09-21). $1.43 at 10 |
| X1 (Abracon ABM8-272-T3, 12 MHz); C27, C28 (15 pF); R20 (1k) | the crystal, its [load capacitors](#g-load-cap), and a series resistor on XOUT | as in Raspberry Pi's design: two 15 pF in series make 7.5 pF, plus about 3 pF of stray, 10.5 pF against the crystal's 10 pF. R20 limits the drive into the crystal, per Raspberry Pi's hardware guide |
| R21, R22 (27 ohm) | series resistors on D+ and D- | the RP2350 datasheet requires 27 ohm. They sit in Raspberry Pi's own 0402 spots (a 27 ohm 0603 doesn't fit the copied layout; the 27 ohm 0402 is an Extended part). 22 ohm until the datasheet pin review of 2026-10-05 |
| R23 (10k to 3V3) | QSPI_SS pull-up | holds the internal flash's chip select high at power-up (the reference's value) |
| R48-R51 (1k, or unfitted) | 4-bit board revision on GPIO22-25 | a 1k to GND reads 0; an unfitted bit reads 1 through the pin's pull-up. This board is revision 1, so R48 is unfitted. JLC fits the 0 bits, so a new revision changes the BOM, not the copper; a resistor rather than a hard tie in case firmware ever drives a pin high. Added 2026-10-05 |

**Protection and failure cases.** RUN has an internal pull-up, so it needs no external one (a
reviewer's claim otherwise was rejected on 2026-09-25). Erratum E9 is covered by 4.7k external
pull-downs on PWR_HOLD (R42) and CHG_OFF (R44), under E9's 8.2k limit. AMP_EN and BL_PWM rely on
larger pull-downs (100k, 103.9k), which the pin review finds safe while they are outputs or at
reset ([input buffer](#g-input-buffer) off), not if firmware releases them as inputs with the buffer on. Buttons and
REV straps use pull-ups, which E9 doesn't affect.

**Bring-up hooks and firmware.** A blank chip boots into its USB [boot loader](#g-boot-loader), so the first sign of
life is a drive called RP2350 on the computer (bring-up step 1): that proves the chip, the crystal,
USB and the 1.1 V core at once. Step 2.5 reads the revision straps. Firmware:

- embassy-rp with the `rp235xb` [feature](#g-feature) (the current firmware builds for `rp235xa`), and a [memory
  map](#g-memory-map) with 2048K of flash. The validation plan proposes generating `pins.rs` from design.py.
- Every GPIO resets with its pull-down on. Turn on the pull-ups of the buttons and REV0-REV3, turn
  off the pull-downs of PWR_BTN_N, CHG_STAT_N and CHG_DONE_N, and turn off both the input buffer
  and the pull-down on the ADC pins.
- Erratum E12: firmware using USB must run clk_sys at 1.1 times clk_usb or more.

**Open items and risks.**

- **The RP2354B was out of stock at JLC and LCSC in the 2026-10-21 BOM audit, which blocks ordering**
  (section 8).
- The chip's [stepping](#g-stepping) is unknown. Errata E9 and E16 apply only to the A2 stepping, and which one
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
| L2 (Abracon AOTA-B201610S3R3, 3.3 uH) | the switcher's [inductor](#g-inductor) | meets the RP2350 datasheet's requirements: 3.3 uH +-20%, [DCR](#g-dcr) at most 250 mOhm (this part: 140 max), [saturation](#g-saturation) at least 1.5 A (this part: 2.4 A). Polarity-marked: fit as drawn. $0.27 at 10 |
| TP9 | +1V1 probe pad, on the core's own via left of the chip | added 2026-10-19 |

**Layout.** The copied part of the board keeps what Raspberry Pi's layout depends on:
`build_pcb.py` copies their routing, vias and the small copper areas around the switcher, the
0402 passives use their land patterns, and over the chip the top layer is a 3V3 pour joined by
short stubs from the 3V3 pins. [Clearances](#g-clearance) inside the copied core go down to 0.1 mm (0.15 mm on the
rest of the board), and the copied vias keep 0.6 mm pads with 0.3 mm holes, so the whole board
stays on JLC's standard process.

**Protection and failure cases.** The chip's brown-out detector watches the 1.1 V core, not 3V3,
so nothing on the board resets the chip cleanly if 3V3 dips. That is why the unplug gap (D7) and the
3.5 V cutoff matter. On the A2 stepping only, erratum E16 says a disturbed USB_OTP_VDD can corrupt
[OTP](#g-otp) reads; the pin review flags that the unplug dips toward 2.97 V go below that pin's 3.135 V
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
| J7 (six bare pads at 2.54 mm, on the back) | 3V3, SWCLK, SWDIO, RUN, BOOTSEL, GND | for pogo pins in a [jig](#g-jig), a clip or soldered wires: flash and debug without J5's cable or USB. Not assembled: it is copper only |
| SW18 (BOOTSEL), R25 (1k) | pulls QSPI_SS to GND through 1k | holding it through a reset enters the USB boot loader; the reference design's strap |
| SW19 (RESET), R26 (1k) | pulls RUN to GND through 1k | resets the chip |

Both buttons are XKB TS-1088 [tactile switches](#g-tactile) under pinholes in the lid. There are no LEDs (an
approved decision).

**Protection and failure cases.** J7's 3V3 pad feeds the 3V3 rail directly, so power the board
through it only with the cell out and nothing on USB. RESET on battery turns the device off, since
the reset drops PWR_HOLD; that is the intended behaviour, and it means a player's only reset is
power-cycling.

**Bring-up and firmware.** Blank flash always lands in the boot loader. With firmware on it, hold
SW18 and tap SW19 to get back there; on J7, short BOOT to GND while tapping RUN to GND. Bring-up
step 2 attaches [probe-rs](#g-probe-rs) or [openocd](#g-openocd) through J5 and then J7, and reads the bring-up firmware's
report on J6 at 115200 [baud](#g-baud).

**Open items.** No LED means no visible error indicator beyond the screen (section 11).

### 7.10 Panel connector and backlight sinks (sheet Display)

**What it does.** The panel is an EastRising ER-TFT035IPS-6: 3.5 in IPS, 320x480, an [ILI9488](#g-ili9488)
controller on 4-wire SPI at 3.3 V. Its 50-contact FPC (0.5 mm [pitch](#g-pitch)) leaves the panel's seat-1
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

The op-amp drives Q5's gate until the [sense voltage](#g-sense-resistor) on R52 equals BL_REF: 0.124 V at 100% [duty](#g-duty) is
56 mA a pin, 113 mA for all six LEDs. LED_K2 has the same sink built from the other half of U6, Q6,
R53, R55, R57, R59 and C49.

**Parts.**

| Parts | Role | Why this part or value |
|---|---|---|
| J3 (Hirose FH12-50S-0.5SH(55)) | 50-pin, 0.5 mm, bottom-contact ZIF connector | JLC places it, so no hand soldering per unit, and a cracked panel swaps in a minute (2026-09-24, when this panel replaced the solder-tail QD3525). It sits where the folded FPC's tip reaches. $0.81 at 10 |
| JP4; C29 (1 uF), C30 (100 nF) | the panel's logic supply (LCD_3V3) from 3V3; decoupling at the connector | JP4 is cut to run the board with the panel unpowered, or to read the panel's logic current |
| R28, R29 (33 ohm) | [source terminations](#g-source-termination) on the SPI clock and data (SCK, MOSI) | fitted next to the MCU, the source end of each line |
| TP11-TP14 | SCK, MOSI, CS, DC | four [logic-analyser](#g-logic-analyser) channels; the panel is the part most likely to need one (2026-10-19) |
| U6 (MCP6002); C51 (100 nF) | dual op-amp, one half per cathode pin; its supply capacitor | replaced the [AP3156](#g-ap3156) [charge pump](#g-charge-pump), out of stock at JLC and LCSC on 2026-10-07; every sink part is Basic or Preferred. $0.24 at 10 |
| Q5, Q6 (AO3400A) | the [pass transistors](#g-pass-transistor), run in their linear region | the pin review checked the simulation's AO3400 model against this part's datasheet ([threshold](#g-threshold), on-resistance) |
| R52, R53 (2.2 ohm 0603) | sense resistors | 2.2 ohm x 56 mA = 0.124 V; the sink needs only about 0.13 V across it, so the current holds from a full cell (or USB) down to the 3.5 V cutoff with the panel's 3.2 V maximum Vf |
| R54, R55 (330 ohm); R58, R59 (10k); C48, C49 (1 nF) | gate resistors; local feedback loop | the 330 ohm isolates the FET's gate capacitance from the op-amp, and the 10k with 1 nF closes a local loop: 72.5 degrees of [phase margin](#g-phase-margin) or more at all 720 [corners](#g-corner) simulated |
| R56, R57 (100k) | gate pull-downs | the sinks are off whenever 3V3 is (U6 runs from 3V3) |
| R60 (100k), R61 (3.9k), C50 (1 uF) | turns GPIO44's PWM into the DC reference BL_REF | 3.3 V x 3.9 / 103.9 = 0.124 V at 100% duty. Time constant 3.75 ms: 0.3% [ripple](#g-ripple) at 36.6 kHz. The LED current is DC, so there is no PWM in the LEDs for the amplifier to pick up |
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

- The ILI9488 needs a new driver ([18-bit colour](#g-18-bit) over SPI, unlike the stripboard's ST7796).
  Pulse LCD_RST, send the init sequence over SPI0 at 10-20 MHz (the datasheet rates serial writes
  to 20 MHz), and read ID4 (0xD3) back on LCD_MISO at 5 MHz or less: expect 0x94 0x88.
- Over SPI the ILI9488 takes 18-bit colour only (COLMOD 0x66): three bytes a pixel, 461 kB a full
  frame, so redraw only what changes. Enable TE (TEON, 0x35) and start frames on GPIO32's pulse,
  about 60 Hz, to avoid tearing.
- Backlight: PWM10 A on GPIO44, for example 12 bits at 36.6 kHz. Current is proportional to duty,
  so apply a [gamma curve](#g-gamma); keep the lowest level at about 18% duty, where the op-amp's [offset](#g-offset) (up to
  4.5 mV against 22 mV) could make the two halves visibly unequal. A change settles in about 15 ms.
  Duty 0 is off.

**Open items and risks.**

- The panel has one maker, EastRising, selling direct. Another 3.5 in panel that fits the
  56.0 x 86.24 x 4.9 mm bay needs only case.scad's panel numbers, J3 and its pin map changed.
- The FPC's reach into J3 and the reversed pin order (contact 1 on J3's pin 50) are checked only
  on paper. Lighting a real panel through a bottom-contact [breakout](#g-breakout) with design.py's mapping is
  item 4 of the validation plan's gate. The model errs long; if the tail bows, J3 moves 1.5-2 mm
  towards seat 1.
- An LED at the older datasheet's 3.4 V Vf would lose 6% of its current at 100% on a 3.5 V cell.
- Accuracy is set by the op-amp's offset: +-3.6% at full, +-20% a pin at 18% duty.
- The pin review notes that the panel's VCI and VDDI are rated to 3.3 V, and the ME6211's
  tolerance allows up to 3.366 V: minor, well inside the absolute maximum.
- The validation plan also proposed a spare filter footprint on LCD_RST; design.py has none.

### 7.11 PWM audio and amplifier (sheet Audio)

**What it does.** A GPIO's PWM is filtered into an audio signal and played by a [bridge-tied](#g-btl)
amplifier into an 8 ohm speaker. The cost route was approved on 2026-09-24: PWM instead of an [I2S](#g-i2s)
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
| R32, R33 (1k); C37, C38 (4.7 nF) | two RC low-pass stages | with the PWM at 8 bits (586 kHz) the [carrier](#g-carrier) comes out about 49 [dB](#g-db) down, and the audio band is -3 dB at about 14 kHz loaded, about what a 15 mm speaker reproduces. 4.7 nF is a JLC Basic part; 3.9 nF, which would give 17 kHz, is only Extended. Changed from 10 nF on 2026-09-25 |
| C39 (220 nF) | [DC block](#g-dc-block) | blocks the PWM's DC (it idles at half scale); with R34 it sets a 36 Hz low corner |
| R34, R35 (20k) | input and feedback resistors | gain 2 x R35 / R34 = 2, the datasheet's circuit; with the filter's 2k in front of R34 the chain's gain is about 1.8 |
| C40 (1 uF) | the bypass capacitor: a half-supply reference on +IN | the datasheet's value; charging it is why the amplifier takes about 100 ms to wake |
| U7 (FM8002A, SOIC-8) | [class AB](#g-class-ab) bridge-tied amplifier from VSYS, rated 1.1 W | the cheapest way to keep a speaker, approved over the class-D NS4168, which is 50 cents a unit more and cleaner. Class AB turns about half its battery draw into sound at high volume, against 80-90% for class D |
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

- PWM4 A on GPIO8 at 8 bits (586 kHz). Not 10 bits: 146 kHz would come out only 26 dB down. [Dither](#g-dither)
  or noise-shape for more resolution.
- Idle the PWM at half scale so turning on doesn't pop. Keep AMP_EN high while the device is awake
  (about 4.4 mA), or raise it at least 150 ms before a sound.
- Full scale is about 0.55 W on USB and 0.4-0.5 W on the cell, and it [clips](#g-clipping) below a 3.7 V cell:
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

- **One GPIO per contact, no matrix.** [v1](#g-v1)'s button matrix [ghosted presses](#g-ghosting) across players; direct
  GPIO cannot (2026-08-19, kept here).
- **DS Lite silicone on bare contacts** (2026-09-21): half the footprint of [SNES](#g-snes) parts, sold as
  repair spares, one D-pad piece and one ABXY piece per seat. ENIG because the pills press straight
  on the copper.
- **Start and Select per seat, power on seat 1's Start** (Alex, 2026-10-19): it matches the DS Lite
  layout, and no game button doubles as power (the power button was seat 1's Y before). The
  contacts sit between the clusters near the case's end, under the DS Lite kit's Start/Select
  membrane.

**Protection and failure cases.** The power contact is the only one on VSYS. Its other side reaches
the MCU only through Q1, so no GPIO ever sees VSYS, and R13 (33k) keeps surface leakage across the
contact from lifting PWR_EN (section 7.6). There are no [ESD](#g-esd) parts on the contact lines; ESD has not
been tested (design_review.md, "Not checked").

**Bring-up and firmware.** Bring-up step 2.1 prints all 20 buttons while each contact's combs are
shorted with tweezers. Firmware enables the internal pull-ups (the pins reset with pull-downs on);
a contact reads low when pressed, and seat 1's Start reads on GPIO3 (PWR_BTN_N), also low when
pressed. Tap versus 500 ms hold is the [platform](#g-platform)'s job; what a press means is core's (section 5.3).
Start and Select have no game meaning in core yet.

**Open items and risks.**

- The validation plan's gate asks for pill resistance under 1 kohm on a matched ENIG coupon, and
  leakage above 100 Mohm on the power contact at 4.2 V (it names P1_Y, the power contact before
  2026-10-19).
- ENIG combs may drift over a product's life; [carbon ink](#g-carbon-ink) would last longer but needs a coarser
  pattern.
- The Start/Select pad is not measured, so the 3-ring footprint and its wells are estimates; the
  A/B/X/Y stems against the [domes](#g-dome) and the D-pad's pivot are open, and `check_buttons.py` fails until
  they settle (section 11).

## 8. Manufacturing and cost

**Route.** [JLCPCB](#g-jlc) makes the [2-layer](#g-layers) board and assembles one side ([Economic PCBA](#g-pcba)). The [panel](#g-panel),
cell, speaker, [DS Lite](#g-ds-lite) silicone and caps, screws and tape are bought separately and fitted by hand.
The case is printed at home on [FDM](#g-fdm). [assembly.md](hardware/ref_design/docs/assembly.md) is the
order and build procedure; [tools/quotes/](hardware/ref_design/tools/quotes/README.md) re-runs
JLCPCB's and PCBWay's instant-quote forms [headlessly](#g-headless) when the design changes.

**Cost, with the date of each figure:**

| Figure | Per unit | Date | Source |
|---|---|---|---|
| [v1](#g-v1) parts, excluding enclosure | about $40 | spring 2026 | [ELECTRONICS.md](docs/archive/v1/ELECTRONICS.md) |
| [Stripboard v2](#g-stripboard-v2) parts, bought in hobby packs (one unit) | about $125 | 2026-08 | [BUILD.md](hardware/stripboard_v2/BUILD.md) |
| [Reference design](#g-ref-a) [bare board](#g-bare-board), 4-layer with 0.25 mm holes, at 10 | $9.74 | 2026-09-24 | quotes |
| Reference design bare board, 2-layer, at 10 | $3.34 | 2026-09-24 | quotes |
| Reference design, factory part (board, parts, assembly fees, shipping) at 10 | about $16.47 | 2026-09-24, parts repriced 2026-10-20 | bom.md, until the audit replaced it |
| Reference design, all in, at 10 | about $39-40 | same | same |
| **Reference design, latest, at 10** | **$42.52** | **2026-10-21** | [BOM](#g-bom) audit, now in [bom.md](hardware/ref_design/docs/bom.md) |
| **Reference design, latest, at 50** | **$27.98** | **2026-10-21** | same |

The bom.md figures exclude import duties (not yet known), JLC's new-account coupons and hand
labour. The panel is the
largest single item (about $14 at 10, under $10 at 50); the case is about $1.50 of filament.

**Supply risks.** The 2026-10-21 BOM audit found the [RP2354B](#g-rp2354b) and its core [inductor](#g-inductor) (L2) out of
stock at JLC and [LCSC](#g-lcsc). **That blocks ordering** until stock returns or an alternative is designed
in. Stock has bitten before: on 2026-10-07 both the [AP3156](#g-ap3156) [backlight](#g-backlight) driver (replaced by [discrete](#g-discrete)
[sinks](#g-current-sink)) and the first USB [ESD array](#g-esd-array) (replaced by another maker's part) ran out. The panel has a
single maker; its [glass class](#g-glass-class) has other sources that would need only case and pin-map changes.

**Order plan.** The [validation plan](#g-validation-plan) proposes 3 to 5 assembled boards plus 5 bare ones from the same
order (for [sectioning](#g-sectioning), [rework](#g-rework) practice and the contact test), and a dry run of JLC's order flow
with screenshots of every [polarised](#g-polarity) part before paying.

## 9. IP and regulatory stance

Decided so far (2026-09-17, [10-production-board.md](docs/archive/architecture/10-production-board.md)
section 6):

- **Rename before selling.** "Mega Blastoise" is a Pokemon name; that includes the USB
  manufacturer string the [firmware](#g-firmware) reports.
- **No Pokemon markings on the device.**
- **No preloaded firmware.** Units ship blank and the owner downloads the game, which keeps
  species names, [sprites](#g-sprite) and move text off the product. Today's builds fetch those assets at
  compile time (section 5.5) and the web build publishes them; how a buyer would get them is open.
- **Real USB IDs.** The firmware uses a placeholder [VID:PID](#g-vid-pid); Raspberry Pi allocates product IDs
  under its vendor ID for products on its silicon.
- **Engines.** `gen1_battle` and `gen3_battle` are written in-house from the published mechanics
  and [Showdown](#g-showdown)'s behaviour; `gen3_battle` takes no code from [`battler`](#g-battler).
- **Hardware sources.** The [RP2350B](#g-rp2350) core copy is [MIT-licensed](#g-mit) (its licence is in
  `hardware/ref_design/datasheets/`). The [vendored](#g-vendored) [membrane](#g-membrane) [footprints](#g-footprint) are public domain. [DS Lite](#g-ds-lite)
  cap numbers come from open models, with no geometry copied. Licence traps ([share-alike](#g-share-alike) and
  non-commercial footprints and models) are listed in the membrane README and the DS Lite [dossier](#g-dossier).
- **DS Lite look.** The US [design patent](#g-design-patent) (D554,194) expired in 2021; an EU [registered design](#g-registered-design), if
  one was filed and renewed, could last until 2031, so an [EUIPO](#g-euipo) search is due before selling
  anything that looks like a DS Lite.
- **Certification.** [FCC Part 15B](#g-fcc) and [CE](#g-ce-marking) as an [unintentional radiator](#g-unintentional) (no radio keeps it there),
  [UN38.3](#g-un383) test documents from the cell supplier, and marketing to adults (a children's product
  triggers [CPSIA](#g-cpsia) testing in the US).

The [validation plan](#g-validation-plan) defers a full [IP](#g-ip) audit, regulatory marks and [EMC](#g-emc) testing until a sale is in
view. The repository itself has no top-level licence file.

## 10. Verification and tooling

### Software

- **The gate:** [scripts/check.sh](scripts/check.sh) runs before every code commit: all [host](#g-host)
  tests, a [firmware](#g-firmware) `cargo check` for its real target, the [WASM](#g-wasm) build, and the [headless](#g-headless) [UI probe](#g-ui-probe)
  (`--fast` skips the last two). It exists because two test binaries once rotted silently.
- **Showdown parity:** [tools/showdown-parity/](tools/showdown-parity/README.md) runs the real
  [Showdown](#g-showdown) simulator under [Node](#g-node). `showdown_parity` compares hand-written scenarios and
  `showdown_fuzz` compares [seeded](#g-seed) random ones (`FUZZ_N`, `FUZZ_SEED`, `FUZZ_ONLY`), with hit,
  [crit](#g-crit) and [damage roll](#g-damage-roll) scripted on both sides so every branch is compared. Unarmed checkouts skip
  these suites loudly. Engine changes get a 2000-seed [sweep](#g-fuzz) before they are pushed.
- **Headless UI probe:** [scripts/ui-probe.sh](scripts/ui-probe.sh) boots the web page in headless
  Chromium on a pinned seed, plays real turns in both generations through the same exports the
  touch controls use, and asserts on a per-seat state trace. It catches wiring bugs that unit
  tests cannot see.
- **Screens:** the `screens` binary renders every UI state to image files on a host.
- **Device debugging:** [scripts/mb-console.py](scripts/mb-console.py) merges [RTT](#g-rtt) and USB console
  output and forwards typed lines; [scripts/fw-build.sh](scripts/fw-build.sh) reports flash and RAM
  use; `scripts/commands.sh` defines shell helpers; `.claude/commands/fwdebug.md` is the firmware
  debug loop for Claude Code.

### Hardware

- **[Reference design](#g-ref-a) checks**, all scripted ([design_review.md](hardware/ref_design/docs/design_review.md),
  "Checks run"): every assembled part's every pin reviewed against its current [datasheet](#g-datasheet)
  (`check_pins.py`), [schematic](#g-schematic) [netlist](#g-netlist) equal to `design.py`, [ERC](#g-erc) and [DRC](#g-drc) clean with [schematic
  parity](#g-schematic-parity), hole sizes, the [membrane](#g-membrane) contact pattern, every part's [3D model](#g-3d-model) inside the case
  (`check_fit.py`), the [button stacks](#g-button-stack) under the shell (`check_buttons.py`), and an [ngspice](#g-ngspice)
  simulation of the [backlight](#g-backlight) [sinks](#g-current-sink).
- **Physical checks so far:** fit-test prints (first on 2026-10-05, again on 2026-10-19), Alex's
  [caliper](#g-caliper) measurements of the [DS Lite](#g-ds-lite) silicone pads (2026-10-20), and the [stripboard](#g-stripboard)'s per-stage
  [bring-up](#g-bring-up) testers.
- **Validation plan:** [validation_plan.md](hardware/ref_design/docs/validation_plan.md) (2026-10-05)
  is a joint plan from two models' cross-examination, with a 13-item [go/no-go gate](#g-validation-plan) for ordering:
  datasheet and [errata](#g-erratum) tables, real [panel](#g-panel) lit through a [breakout](#g-breakout), contact resistance on a test
  [coupon](#g-coupon), a [latch](#g-latch) replica, measured stack heights, a [JLC](#g-jlc) dry run, an outside review, and live stock
  on order day. Several of its board changes are already in `design.py`; the physical items are
  still listed as open in PLAN.md.
- **Bring-up:** [bringup.md](hardware/ref_design/docs/bringup.md) is the step-by-step first power
  of a new board, ending with the USB hand-over and power switch.

### Working practices

- Generated files are never hand-edited: the stripboard from `layout.py`, the reference design
  from `design.py`, `placement.py` and `case.scad`, the [pin map](#g-pin-map) from `pinmap.py`.
- For [engine](#g-battle-engine) work, read Showdown's source first, then fuzz to confirm.
- For case work, render and assert headless before printing.

## 11. Status, known gaps and open decisions

### Status on 2026-10-21

| Area | Status |
|---|---|
| [Gen 1](#g-gen) [engine](#g-battle-engine) | 164 of the 165 moves in its [random battle pool](#g-randbat) under [parity](#g-parity) and [fuzz tests](#g-fuzz) |
| Gen 3 engine | Moves, [abilities](#g-ability) and [held items](#g-held-item) modelled; 5 of 600 [seeds](#g-seed) in the widest fuzz sweep unexplained (2026-08-19) |
| Web build | Live on [Pages](#g-github-pages); both generations playable |
| [Firmware](#g-firmware) | Runs the full game on the [stripboard v2](#g-stripboard-v2) |
| Stripboard v2 | Built and working |
| Alex's [prod_board](#g-prod-board) | Contacts placed; [schematic](#g-schematic) not started |
| [Reference design](#g-ref-a) | Designed, [routed](#g-routing) and checked; fit-test prints done; not ordered |
| Ordering | Blocked: [RP2354B](#g-rp2354b) and L2 out of stock (2026-10-21) |

### Known gaps

- No firmware for the reference design (RP2354B [pin map](#g-pin-map), [ILI9488](#g-ili9488) driver, audio, [power latch](#g-latch),
  charger status, battery cutoff).
- No sound anywhere yet: no [mixer](#g-mixer) in [core](#g-core) and no web audio. The options menu dropped its sound and
  turn-timer rows because nothing was behind them.
- No turn timer, so a stalled player can still hold up a game.
- Start and Select have no meaning in core; MENU acts as B.
- The device's Gen 3 draft gives no abilities, items or [EV spreads](#g-ev-iv-nature).
- A team reveal screen and [sprite](#g-sprite) compression for flash are not built.
- D-pad auto-repeat exists only in the web page's input code; the firmware does not repeat yet.
- Whether the firmware and its assets fit the RP2354B's 2 MB of flash has not been measured.
- No persistent battle log in flash for post-mortems (a long-standing to-do).
- The reference design has no LED: there is no visible error indicator beyond the screen, and a
  player's only reset is power-cycling (RESET sits under a pinhole).

### Open decisions and to-dos

From [PLAN.md](hardware/ref_design/PLAN.md) and the [validation plan](#g-validation-plan):

- **Before ordering:** a source for the RP2354B and L2, or an alternative; the measurements in
  PLAN.md's "Measure before ordering" (the Start/Select pad, the D-pad collar, whether [dome](#g-dome) tops
  take the cap's stem, the real [panel](#g-panel) against its drawing and the [FPC](#g-fpc)'s reach into the connector,
  the cell's length and plug [polarity](#g-polarity), the printed [snap fit](#g-snap-fit), a sample speaker plug in a genuine
  [header](#g-header)); the go/no-go gate.
- **Buttons:** the A/B/X/Y stems sit on the domes at rest, the D-pad's pivot needs modelling on
  its centre collar, and the Start/Select wells are estimated; `check_buttons.py` fails until these
  settle.
- **Case:** narrow it 1 mm a side once a real silicone piece measures 26 mm or less; the mismatched
  end [bezels](#g-bezel) (tabled); whether to adopt the [DS Lite](#g-ds-lite) [dossier](#g-dossier)'s recommendations (mass, edges).
- **Disputed in the validation plan:** 3 or 5 assembled boards; whether a latch replica gates the
  order; a hardware long-press kill [footprint](#g-footprint).
- **Contact finish:** [ENIG](#g-enig) [combs](#g-comb) (chosen) may drift over a product's life; [carbon ink](#g-carbon-ink) needs a
  coarser pattern.
- **Enclosure at volume:** printing serves prototypes; 10-production-board.md names [injection
  moulding](#g-injection) for volume, and nothing is planned yet.
- **Product:** the new name, [USB IDs](#g-vid-pid), how buyers get the firmware and assets, the [EUIPO](#g-euipo) check.

### Unknown

- Battery life of the reference design: not measured or estimated in the repo.
- The finished unit's mass against the about 160 g target: not measured.

## 12. Where things live

| Path | What |
|---|---|
| [README.md](README.md) | Build and run instructions |
| [CLAUDE.md](CLAUDE.md) | The [zero-drift rule](#g-zero-drift) and project rules for Claude Code |
| [docs/architecture/](docs/architecture/README.md) | Software architecture pages |
| [docs/archive/architecture/](docs/archive/architecture/09-single-screen.md) | Superseded plans: [09](docs/archive/architecture/09-single-screen.md) single-screen redesign, [10](docs/archive/architecture/10-production-board.md) production board |
| [docs/TECHNICAL.md](docs/TECHNICAL.md) | Software internals overview |
| [docs/archive/v1/](docs/archive/v1/DESIGN.md) | The [v1](#g-v1) device: design, electronics, [firmware](#g-firmware) interactions, original plan |
| [docs/hardware/openscad-methodology.md](docs/hardware/openscad-methodology.md) | How the cases are modelled and checked |
| `crates/gen1_battle/`, `crates/gen3_battle/`, `crates/battle_types/` | Battle [engines](#g-battle-engine); [Gen 1 spec](crates/gen1_battle/SPEC.md) |
| `crates/mega_blastoise_core/` | The shared game: loop, input, screens, renderer, generated data |
| `crates/mega_blastoise_fw/` | Firmware for the [stripboard v2](#g-stripboard-v2) |
| `crates/mega_blastoise_web/` | Browser build; `www/` holds the pages |
| `crates/mega_blastoise_test/` | Host CLI, tests, [parity](#g-parity) and [fuzz suites](#g-fuzz), screen dumper |
| `battler/` | Git [submodule](#g-submodule): the upstream engine, now a data source |
| `patches/once_cell/` | A patched dependency for [`no_std`](#g-no-std) targets |
| [tools/showdown-parity/](tools/showdown-parity/README.md) | [Node](#g-node) harness around the real [Showdown](#g-showdown) |
| `scripts/` | [The gate](#g-the-gate), [UI probe](#g-ui-probe), console, build and publish helpers |
| `firmware/bringup/` | Standalone [bring-up](#g-bring-up) [crates](#g-crate) |
| `site/` | The [Pages](#g-github-pages) front page |
| [hardware/README.md](hardware/README.md) | Hardware index |
| [hardware/stripboard_v2/](hardware/stripboard_v2/BUILD.md) | The built board: generator, build guide, assembly stages, models |
| [hardware/ref_design/](hardware/ref_design/README.md) | The [reference design](#g-ref-a): `board/`, `case/`, `docs/`, `datasheets/`, `tools/quotes/`, `viewer/` |
| `hardware/prod_board/`, [hardware/prod_case/](hardware/prod_case/README.md) | Alex's own board and case (read only for others) |
| [hardware/membrane/](hardware/membrane/README.md) | Membrane contact [footprints](#g-footprint) and their generator and checker |
| [hardware/ds_lite/](hardware/ds_lite/README.md) | [DS Lite](#g-ds-lite) construction research |

## 13. Glossary

Terms of art used in this document, in alphabetical order. In each top-level section, the first use of a term links to its entry here. The link at the end of an entry leads to a fuller public reference, or to the project file that is the authority.

- <a id="g-imperial"></a>**0402, 0603, 1206**: Size codes for chip resistors and capacitors in hundredths of an inch: 0402 is 1.0 x 0.5 mm, 0603 is 1.6 x 0.8 mm and 1206 is 3.2 x 1.6 mm. Bigger parts take more power and are easier to fit by hand. More: [Wikipedia](https://en.wikipedia.org/wiki/Surface-mount_technology).
- <a id="g-18-bit"></a>**18-bit colour**: Six bits each of red, green and blue per pixel, sent as three bytes. Over [SPI](#g-spi) the [ILI9488](#g-ili9488) takes only this, so a full [REF-A](#g-ref-a) frame is 461 kB. More: [Wikipedia](https://en.wikipedia.org/wiki/Color_depth).
- <a id="g-layers"></a>**2-layer, 4-layer**: How many copper layers a board has. Two are cheapest; REF-A fits on two because each pin faces where its [net](#g-net) goes; its first 4-layer version's [bare board](#g-bare-board) cost about three times as much. More: [Wikipedia](https://en.wikipedia.org/wiki/Printed_circuit_board).
- <a id="g-3d-model"></a>**3D model**: A part's solid shape, which [KiCad](#g-kicad) shows on the board and `check_fit.py` tests against the case. More: [KiCad docs](https://docs.kicad.org/10.0/en/pcbnew/pcbnew.html#navigating_the_3d_view).
- <a id="g-3v3"></a>**3V3, 1V1**: Rail names that put the V where the decimal point goes: 3V3 is 3.3 V and 1V1 is 1.1 V. On REF-A, +3V3 exists only while the device is on, and +1V1 is the chip's core supply; the + is part of the [power symbol](#g-power-symbol)'s name.
- <a id="g-403450"></a>**403450, 103450**: Size codes for flat [lithium cells](#g-li-ion): thickness, width and length, so a 403450 is about 4.0 x 34 x 50 mm and a 103450 about 10 x 34 x 50 mm.
- <a id="g-ability"></a>**Ability**: A Pokemon's built-in passive effect, such as Levitate or Intimidate. Abilities arrived in [Gen 3](#g-gen), so Gen 1 has none; gen3_battle models 75 of the 76. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Ability).
- <a id="g-abs-max"></a>**Absolute maximum rating**: The limit in a [datasheet](#g-datasheet) beyond which a part may be damaged, as against the range it is meant to work in. The design checks its peaks against these, for example [VSYS](#g-vsys) against the [FM8002A](#g-fm8002a)'s 6 V.
- <a id="g-adc"></a>**ADC**: Analogue-to-digital converter: measures a voltage as a number. REF-A reads the cell (ADC7) and the two [USB-C](#g-usb-c) [CC lines](#g-cc) (ADC6, ADC5); readings are relative to [3V3](#g-3v3), the ADC's reference. More: [Wikipedia](https://en.wikipedia.org/wiki/Analog-to-digital_converter).
- <a id="g-ammeter"></a>**Ammeter**: A current meter. It goes in series with the circuit, which is why REF-A's [cut jumpers](#g-solder-jumper) can be bridged with one. More: [Wikipedia](https://en.wikipedia.org/wiki/Ammeter).
- <a id="g-anode"></a>**Anode and cathode**: A diode's or LED's two ends: current flows in at the anode and out at the cathode (the end marked with a bar). Common anode means several LEDs share one anode connection, as the [backlight](#g-backlight)'s do on VSYS. More: [Wikipedia](https://en.wikipedia.org/wiki/Diode).
- <a id="g-ap3156"></a>**AP3156**: A charge-pump backlight driver REF-A first used. It ran out of stock at [JLC](#g-jlc) and [LCSC](#g-lcsc) on 2026-10-07, and [op-amp](#g-op-amp) [current sinks](#g-current-sink) replaced it.
- <a id="g-async-task"></a>**Async task**: A function that can pause while it waits (for a timer, a [DMA](#g-dma) transfer, a pin) and let other tasks run, all on a single CPU with no operating system. The [firmware](#g-firmware) is a set of [Embassy](#g-embassy) tasks. More: [Embassy book](https://embassy.dev/book/).
- <a id="g-attract-demo"></a>**Attract demo**: The AI-versus-AI battle the device plays across both [halves](#g-half) while idle, after 15 s in the lobby.
- <a id="g-back-power"></a>**Back-powering**: A chip or [rail](#g-rail) fed through a path never meant to supply it, such as a signal pin's protection diode, while its real supply is off. REF-A had one through USB [D+](#g-dplus) until U1's reference pin moved to 3V3.
- <a id="g-backlight"></a>**Backlight**: The LEDs behind an LCD that make its light; the LCD only filters it. REF-A's [panel](#g-panel) has six white LEDs, driven by two current sinks. More: [Wikipedia](https://en.wikipedia.org/wiki/Backlight).
- <a id="g-bare-board"></a>**Bare board**: A [PCB](#g-pcb) as the [fab](#g-fab) makes it, with no parts fitted.
- <a id="g-basic-ext"></a>**Basic, Preferred and Extended parts**: JLC part classes. Basic and Preferred parts stay loaded on the assembly machines; each Extended part type adds a per-order loading fee. More: [JLCPCB parts](https://jlcpcb.com/parts).
- <a id="g-battle-engine"></a>**Battle engine**: The [crate](#g-crate) that holds one generation's rules: `gen1_battle` or `gen3_battle`. It takes each side's choice and writes the turn as log lines; [core](#g-core) does everything else.
- <a id="g-battler"></a>**`battler`**: The upstream open-source Rust Pokemon [engine](#g-battle-engine) the project began on, checked out as a [git submodule](#g-submodule) (Alex's fork). It ran out of [heap](#g-heap) on the [RP2040](#g-rp2040), so it is now only a data source for core's [build script](#g-build-script). More: [GitHub](https://github.com/jackson-nestelroad/battler).
- <a id="g-baud"></a>**Baud**: Symbols per second on a serial line; on a [UART](#g-uart), bits per second. More: [Wikipedia](https://en.wikipedia.org/wiki/Baud).
- <a id="g-bench-supply"></a>**Bench supply**: An adjustable laboratory power supply with a current limit, used to feed a board safely during [bring-up](#g-bring-up). More: [Wikipedia](https://en.wikipedia.org/wiki/Power_supply).
- <a id="g-bezel"></a>**Bezel**: The rim of the case around a screen or a group of buttons. More: [Wikipedia](https://en.wikipedia.org/wiki/Bezel).
- <a id="g-bleeder"></a>**Bleeder**: A resistor that drains a node when nothing else does. R38 (1k) bleeds [VBUS](#g-vbus) after an unplug so Q2 takes over in milliseconds, not tens of milliseconds. More: [Wikipedia](https://en.wikipedia.org/wiki/Bleeder_resistor).
- <a id="g-blit"></a>**Blit**: To copy a block of pixels in one go. The web page blits each finished frame onto a canvas. More: [Wikipedia](https://en.wikipedia.org/wiki/Bit_blit).
- <a id="g-board-outline"></a>**Board outline**: The shape the board is cut to. On Alex's board it comes from the case's `layout.scad`, so the board always fits the case. More: [KiCad docs](https://docs.kicad.org/10.0/en/pcbnew/pcbnew.html#board-outlines).
- <a id="g-boardevent"></a>**`BoardEvent`**: A typed battle event (damage, faint, switch-in, status and so on) parsed from an engine's log.
- <a id="g-body-diode"></a>**Body diode**: The diode every power [MOSFET](#g-fet) has built in between drain and source. In Q2 it would carry the cell to VSYS at an unplug, but with a 0.6 V drop, which is why D7 sits beside it. More: [Wikipedia](https://en.wikipedia.org/wiki/Power_MOSFET).
- <a id="g-bom"></a>**BOM**: Bill of materials: every part with its quantity, value and supplier number. JLC's assembly order takes a BOM and a [placement file](#g-placement-file); REF-A's `bom.md` carries the costs. More: [Wikipedia](https://en.wikipedia.org/wiki/Bill_of_materials).
- <a id="g-boot-loader"></a>**Boot loader**: Code that runs at power-up and loads or receives the main program. The [RP2350](#g-rp2350)'s is in its ROM: a blank chip, or one reset with [BOOTSEL](#g-bootsel) held, appears on a computer as a USB drive that takes a firmware file. More: [Wikipedia](https://en.wikipedia.org/wiki/Bootloader).
- <a id="g-bootsel"></a>**BOOTSEL**: The RP2350's boot-select input: holding QSPI_SS low through a reset enters the USB [boot loader](#g-boot-loader) instead of the firmware. On REF-A that is SW18 under a pinhole, or BOOT to [GND](#g-ground) on J7. More: [RP2350 datasheet](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf).
- <a id="g-breadboard"></a>**Breadboard**: A plug-in board for trying circuits without soldering; [v1](#g-v1) began on one. More: [Wikipedia](https://en.wikipedia.org/wiki/Breadboard).
- <a id="g-breakout"></a>**Breakout board**: A small adapter board that brings a part's fine-pitch pins out to easy pads or [headers](#g-header), used to try a part before the real board exists. More: [Wikipedia](https://en.wikipedia.org/wiki/Printed_circuit_board#Breakout_boards).
- <a id="g-btl"></a>**Bridge-tied load (BTL)**: An amplifier output that drives both speaker wires, in opposite directions, doubling the swing from one supply. Neither speaker pin may touch ground. More: [Wikipedia](https://en.wikipedia.org/wiki/Bridge-tied_load).
- <a id="g-bring-up"></a>**Bring-up**: The first power-on of a new board, step by step with a check after each stage, so a fault shows up before it can do damage. REF-A's procedure is `bringup.md`. More: [bringup.md](hardware/ref_design/docs/bringup.md).
- <a id="g-brown-out"></a>**Brown-out detector, supervisor**: A circuit that resets a chip when its supply sags, so it never runs at a voltage too low for its logic or flash. The RP2350's brown-out detector watches only the 1.1 V core, and REF-A has no supervisor on 3V3, which is why the unplug dip and the 3.5 V cutoff matter. More: [Wikipedia](https://en.wikipedia.org/wiki/Power-on_reset).
- <a id="g-buck-boost"></a>**Buck-boost regulator**: A [switching regulator](#g-switcher) that can step a voltage down or up, so it gives 3.3 V from a cell anywhere between full and flat. REF-A uses a cheaper [LDO](#g-ldo) instead and gives up roughly 5-10% of the cell. More: [Wikipedia](https://en.wikipedia.org/wiki/Buck%E2%80%93boost_converter).
- <a id="g-build-script"></a>**Build script**: A Rust program (`build.rs`) that Cargo runs before compiling a crate. Core's generates the static data tables and [sprites](#g-sprite) at compile time. More: [Cargo book](https://doc.rust-lang.org/cargo/reference/build-scripts.html).
- <a id="g-bulk-cap"></a>**Bulk capacitor**: A larger capacitor (microfarads) that holds a rail up through steps in load or supply, as against the small [decoupling capacitors](#g-decoupling) at each pin. USB limits bulk capacitance on VBUS to 10 uF. More: [Wikipedia](https://en.wikipedia.org/wiki/Decoupling_capacitor).
- <a id="g-button-matrix"></a>**Button matrix**: Buttons wired in rows and columns so a few pins read many buttons, at the cost of [ghosting](#g-ghosting). v1 used one; [stripboard v2](#g-stripboard-v2) and REF-A give every button its own [GPIO](#g-gpio). More: [Wikipedia](https://en.wikipedia.org/wiki/Keyboard_matrix_circuit).
- <a id="g-button-stack"></a>**Button stack**: Everything in one button's column: the printed cap, the silicone [dome](#g-dome) and [pill](#g-pill), and the contact on the board. `check_buttons.py` checks the stacks under the case shell.
- <a id="g-c-rate"></a>**C rate**: A charge or discharge current as a multiple of the cell's capacity: 0.5 C for a 700 mAh cell is 350 mA. More: [Wikipedia](https://en.wikipedia.org/wiki/Battery_charger#C-rate).
- <a id="g-calendar-ageing"></a>**Calendar ageing**: The capacity a lithium cell loses with time, used or not; it is worst when the cell is kept full and warm. More: [Wikipedia](https://en.wikipedia.org/wiki/Lithium-ion_battery).
- <a id="g-caliper"></a>**Caliper**: A measuring tool with sliding jaws; a digital one reads to 0.01 mm. More: [Wikipedia](https://en.wikipedia.org/wiki/Calipers).
- <a id="g-carbon-ink"></a>**Carbon ink**: A conductive carbon print for keypad contacts: harder wearing than gold, but it needs coarser lines than the [DS Lite](#g-ds-lite) [combs](#g-comb).
- <a id="g-pill"></a>**Carbon pill**: The conductive carbon disc under each dome. A press pushes it onto the contact and it bridges the combs; REF-A's contacts need a pill of 1.6 mm or more.
- <a id="g-carrier"></a>**Carrier**: The [PWM](#g-pwm)'s own switching frequency (586 kHz for REF-A's audio), which the filter removes, leaving the slower audio signal it carried.
- <a id="g-cc"></a>**CC lines and Rd**: USB-C's two configuration channel pins. A device marks itself as a [power sink](#g-source-sink) with 5.1k Rd [pull-downs](#g-pull-up) on them, and the source then turns VBUS on; the voltage on the active CC line says how much current the source offers (default USB, 1.5 A or 3 A). More: [Wikipedia](https://en.wikipedia.org/wiki/USB-C).
- <a id="g-ce-marking"></a>**CE marking**: The mark that says a product meets EU rules; for this device mainly [EMC](#g-emc), as an [unintentional radiator](#g-unintentional). More: [Wikipedia](https://en.wikipedia.org/wiki/CE_marking).
- <a id="g-ceramic-cap"></a>**Ceramic capacitor**: The common small surface-mount capacitor: cheap and low-loss, but it can ring with a cable's inductance at a [live plug-in](#g-hot-plug), and it loses capacitance under DC voltage. More: [Wikipedia](https://en.wikipedia.org/wiki/Ceramic_capacitor).
- <a id="g-charge-pump"></a>**Charge pump**: A converter that makes a higher (or negative) voltage by switching capacitors instead of an [inductor](#g-inductor). The [AP3156](#g-ap3156) backlight driver was one. More: [Wikipedia](https://en.wikipedia.org/wiki/Charge_pump).
- <a id="g-chip-select"></a>**Chip select (CS)**: The line that tells one device on a shared SPI bus that it is being addressed. The internal flash's chip select, QSPI_SS, doubles as the BOOTSEL [strap](#g-strap). More: [Wikipedia](https://en.wikipedia.org/wiki/Serial_Peripheral_Interface).
- <a id="g-choicecollector"></a>**`ChoiceCollector`**: Core's battle-input state machine: prompts, accept and reject, the waiting screen, take-back, the [grace window](#g-grace) and the typed-line grammar.
- <a id="g-clamp"></a>**Clamp diode**: A diode inside a chip from each pin to its supply that diverts [ESD](#g-esd) and over-voltage. It also leaks: with the [RP2354B](#g-rp2354b) unpowered, GPIO47 sits above [IOVDD](#g-iovdd) and its clamp takes a few [uA](#g-units) from the cell [divider](#g-divider).
- <a id="g-class-ab"></a>**Class AB, class D**: Amplifier types. Class AB runs its output transistors linearly: simple and cheap, turning about half its battery draw into sound at high volume. Class D switches its output and reaches 80-90%. More: [Wikipedia](https://en.wikipedia.org/wiki/Power_amplifier_classes).
- <a id="g-clearance"></a>**Clearance**: The minimum gap between copper features of different nets. REF-A uses 0.15 mm, down to 0.1 mm inside the copied RP2350B core. More: [KiCad docs](https://docs.kicad.org/10.0/en/pcbnew/pcbnew.html#clearance-and-constraint-resolution).
- <a id="g-clipping"></a>**Clipping**: An amplifier's output hitting its supply limit, so the peaks of the wave are cut flat and the sound distorts. More: [Wikipedia](https://en.wikipedia.org/wiki/Clipping_(audio)).
- <a id="g-comb"></a>**Comb**: One half of a [membrane](#g-membrane) contact: two interleaved combs of bare copper (0.15 mm lines, 0.20 mm gaps) that a pill bridges when pressed.
- <a id="g-core"></a>**Core**: `mega_blastoise_core`, the [`no_std`](#g-no-std) crate shared by every [platform](#g-platform). It holds the game loop, the input rules, the screens and the renderer; see the [zero-drift rule](#g-zero-drift).
- <a id="g-corner"></a>**Corner (simulation)**: One combination of extremes (part tolerances, temperature, supply, load) that a [simulation](#g-ngspice) is run at, so the design is checked at its worst, not only its typical case. More: [Wikipedia](https://en.wikipedia.org/wiki/Process_corners).
- <a id="g-coupon"></a>**Coupon**: A small test piece that checks one thing: printed case coupons in the [fit kit](#g-fit-kit), or a bare [ENIG](#g-enig) board coupon for contact resistance.
- <a id="g-cover-lens"></a>**Cover lens**: A clear window over the panel to protect it. REF-A has none, an approved cost saving.
- <a id="g-cpsia"></a>**CPSIA**: The US Consumer Product Safety Improvement Act, which requires testing of children's products; marketing to adults avoids it. More: [Wikipedia](https://en.wikipedia.org/wiki/Consumer_Product_Safety_Improvement_Act).
- <a id="g-crate"></a>**Crate**: Rust's unit of code: one library or program. The repository holds the engines, core and one crate per platform. More: [Rust book](https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html).
- <a id="g-crit"></a>**Critical hit (crit)**: A hit that does extra damage, at random. The battle log gives the reason for each one, and the [fuzzer](#g-fuzz) scripts them on both sides so the comparison is exact. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Critical_hit).
- <a id="g-crystal"></a>**Crystal**: A quartz resonator that gives a chip an accurate clock; USB needs that accuracy. REF-A's X1 is 12 MHz, as in Raspberry Pi's design, on the chip's XIN and XOUT pins. More: [Wikipedia](https://en.wikipedia.org/wiki/Crystal_oscillator).
- <a id="g-current-sink"></a>**Current sink**: A circuit that pulls a set current to ground whatever the voltage above it. Each of REF-A's two backlight sinks is an op-amp driving a MOSFET until the voltage on a [sense resistor](#g-sense-resistor) matches a reference. More: [Wikipedia](https://en.wikipedia.org/wiki/Current_source).
- <a id="g-dplus"></a>**D+ and D-**: The USB 2.0 data pair. On REF-A it runs from J1 through the [ESD array](#g-esd-array) and 27 ohm series resistors to the RP2354B (USB_DP, USB_DM). More: [Wikipedia](https://en.wikipedia.org/wiki/USB_hardware).
- <a id="g-damage-roll"></a>**Damage roll**: The random factor in each damage calculation (in Gen 3, a multiplier from 85% to 100%). The fuzzer fixes it on both sides so every branch compares exactly. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Damage).
- <a id="g-datasheet"></a>**Datasheet**: The maker's document for a part: pins, limits, typical circuits. REF-A keeps each one in `hardware/ref_design/datasheets/` and checks every pin against it. More: [Wikipedia](https://en.wikipedia.org/wiki/Datasheet).
- <a id="g-dc-block"></a>**DC block**: A series capacitor that passes an audio swing but not the steady voltage it rides on. C39 blocks the PWM's half-scale idle level.
- <a id="g-dc-bias"></a>**DC-bias derating**: Ceramic capacitors lose capacitance as the DC voltage across them rises, small high-value parts often by half or more, so a 22 uF part on a 5 V rail may act like much less. More: [Wikipedia](https://en.wikipedia.org/wiki/Ceramic_capacitor).
- <a id="g-dcr"></a>**DCR**: An inductor's DC resistance, the plain resistance of its winding. L2 must have 250 mOhm or less. More: [Wikipedia](https://en.wikipedia.org/wiki/Inductor).
- <a id="g-debounce"></a>**Debounce**: Ignoring the rapid on-off chatter a mechanical contact makes for a few milliseconds when pressed or released, so one press counts once. More: [Wikipedia](https://en.wikipedia.org/wiki/Switch#Contact_bounce).
- <a id="g-debug-probe"></a>**Debug Probe**: Raspberry Pi's small USB adapter that speaks [SWD](#g-swd) and UART to a target board over 3-pin [JST SH](#g-jst) cables. J5 and J6 use its pinout. More: [Raspberry Pi](https://www.raspberrypi.com/documentation/microcontrollers/debug-probe.html).
- <a id="g-db"></a>**Decibel (dB)**: A logarithmic ratio: every 20 dB is a factor of ten in voltage. The PWM [carrier](#g-carrier) coming out 49 dB down means it is about 1/280 of its size. More: [Wikipedia](https://en.wikipedia.org/wiki/Decibel).
- <a id="g-deck-insert"></a>**Deck insert**: The printed part carrying one end's buttons; the same part, printed twice, serves both ends.
- <a id="g-decoupling"></a>**Decoupling capacitor**: A small capacitor (often 100 nF) right at a chip's supply pin that supplies fast current spikes locally and keeps noise off the rail. More: [Wikipedia](https://en.wikipedia.org/wiki/Decoupling_capacitor).
- <a id="g-defmt"></a>**defmt**: A compact logging format for embedded Rust: the device sends short codes and the [host](#g-host) expands them into text. More: [defmt book](https://defmt.ferrous-systems.com/).
- <a id="g-design-patent"></a>**Design patent**: A US patent on how a product looks rather than how it works. The DS Lite's (D554,194) expired in 2021. More: [Wikipedia](https://en.wikipedia.org/wiki/Design_patent).
- <a id="g-deviceevent"></a>**`DeviceEvent`**: A press as a platform hands it to core, already classified as a tap or a 500 ms hold.
- <a id="g-devicerunner"></a>**`DeviceRunner` / `DevicePort`**: Core's game loop ([attract](#g-attract-demo), lobby, ready sequence, battle, recap), and the [trait](#g-trait) a platform implements for it. It moved into core on 2026-08-27.
- <a id="g-devicesession"></a>**`DeviceSession`**: Core's state outside a turn: menus, cursors, overlays, and what a press means in each state.
- <a id="g-die"></a>**Die**: The bare piece of silicon inside a chip's package. The RP2354B is the RP2350B die with a flash die stacked in the same package. More: [Wikipedia](https://en.wikipedia.org/wiki/Die_(integrated_circuit)).
- <a id="g-differential"></a>**Differential testing**: Running the same inputs through two implementations and comparing the outputs; here the engines against [Showdown](#g-showdown). More: [Wikipedia](https://en.wikipedia.org/wiki/Differential_testing).
- <a id="g-diode-or"></a>**Diode OR**: Several sources joined to one node, each through its own diode, so the highest wins and none can feed another. REF-A's PWR_EN is the OR of the power button, [PWR_HOLD](#g-latch) and USB. More: [Wikipedia](https://en.wikipedia.org/wiki/Diode_logic).
- <a id="g-diode-test"></a>**Diode test**: A multimeter mode that pushes a small current through a part and shows the voltage across it; a [Schottky](#g-schottky) reads about 0.2-0.3 V. More: [Wikipedia](https://en.wikipedia.org/wiki/Multimeter).
- <a id="g-dirty-tiles"></a>**Dirty tiles**: Redrawing only what changed: the firmware splits the frame into 16x16 tiles and sends only those whose pixels differ, which keeps SPI traffic down.
- <a id="g-discrete"></a>**Discrete**: Built from separate standard parts (op-amp, transistors, resistors) rather than one dedicated chip, as REF-A's backlight sinks are.
- <a id="g-panel-commands"></a>**DISPOFF, SLPIN, COLMOD, TEON, ID4**: ILI9488 command names: DISPOFF blanks the display, SLPIN puts the panel to sleep, COLMOD sets the pixel format, TEON turns on the tearing-effect output, and ID4 reads back the controller's ID.
- <a id="g-dither"></a>**Dither, noise shaping**: Adding a little noise (dither), or pushing the rounding error into frequencies the ear notices less (noise shaping), to get more effective resolution from an 8-bit PWM. More: [Wikipedia](https://en.wikipedia.org/wiki/Noise_shaping).
- <a id="g-dma"></a>**DMA**: Direct memory access: hardware that moves data (here pixels to the panel's SPI) while the CPU does other work. More: [Wikipedia](https://en.wikipedia.org/wiki/Direct_memory_access).
- <a id="g-dome"></a>**Dome**: The raised bubble in the membrane over each contact. Pressed, it collapses with a click and pushes its pill down. More: [Wikipedia](https://en.wikipedia.org/wiki/Keyboard_technology).
- <a id="g-drc"></a>**DRC**: Design rule check: KiCad's check of the board against the fab's limits ([clearances](#g-clearance), hole sizes, [track](#g-routing) widths) and for unconnected nets. More: [KiCad docs](https://docs.kicad.org/10.0/en/pcbnew/pcbnew.html#design-rule-checking).
- <a id="g-dropout"></a>**Dropout**: The smallest voltage a linear [regulator](#g-regulator) needs between input and output to keep regulating. The [ME6211](#g-me6211)'s is 120 mV at 100 mA, so 3V3 holds down to a cell of about 3.42 V and then follows the cell. More: [Wikipedia](https://en.wikipedia.org/wiki/Low-dropout_regulator).
- <a id="g-ds-lite"></a>**DS Lite**: The Nintendo DS Lite (2006): REF-A's target for feel, buttons and case, and the source of its silicone membranes, sold as repair spares. More: [Wikipedia](https://en.wikipedia.org/wiki/Nintendo_DS#DS_Lite).
- <a id="g-dossier"></a>**DS Lite dossier**: `hardware/ds_lite/`: the research packet on how the DS Lite is built and why it feels as it does, with every number sourced. Its recommendations are inputs, not decisions. More: [hardware/ds_lite](hardware/ds_lite/README.md).
- <a id="g-duty"></a>**Duty cycle**: The fraction of each PWM period the output is high. Averaged by a filter, 50% duty from 3.3 V gives 1.65 V. More: [Wikipedia](https://en.wikipedia.org/wiki/Duty_cycle).
- <a id="g-embassy"></a>**Embassy**: An async framework for embedded Rust; `embassy-rp` is its layer for Raspberry Pi's chips. It lets the firmware run as cooperating tasks without an operating system. More: [embassy.dev](https://embassy.dev/).
- <a id="g-emc"></a>**EMC**: Electromagnetic compatibility: a device neither gives off interference beyond the limits nor fails under it. More: [Wikipedia](https://en.wikipedia.org/wiki/Electromagnetic_compatibility).
- <a id="g-enable"></a>**Enable (EN, CE, SHUTDOWN)**: An input that turns a chip on or off. U3's enable is PWR_EN; the [TP4056](#g-tp4056)'s is CE (it charges while CE is high); the FM8002A has the opposite, a SHUTDOWN pin that turns it off when high.
- <a id="g-enig"></a>**ENIG**: Electroless nickel, immersion gold: a flat gold-over-nickel PCB finish. Needed because the pills press on bare copper; REF-A's whole board has it. More: [Wikipedia](https://en.wikipedia.org/wiki/Electroless_nickel_immersion_gold).
- <a id="g-entropy"></a>**Entropy**: Unpredictable input used to [seed](#g-seed) random numbers (team drafts, [damage rolls](#g-damage-roll)); each platform supplies its own. More: [Wikipedia](https://en.wikipedia.org/wiki/Entropy_(computing)).
- <a id="g-erc"></a>**ERC**: Electrical rules check: KiCad's check of the [schematic](#g-schematic) for pins left unconnected, outputs driving each other, and power inputs with nothing driving them. More: [KiCad docs](https://docs.kicad.org/10.0/en/eeschema/eeschema.html#erc).
- <a id="g-erratum"></a>**Erratum**: A known chip bug, published by the maker with a number (errata in the plural). REF-A designs around RP2350-E9 (a released pin can float near 2 V, so a pull-down must be 8.2k or less to hold it low), E12 (a clock ratio for USB) and E16 ([OTP](#g-otp) reads). More: [RP2350 datasheet](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf).
- <a id="g-esd"></a>**ESD**: Electrostatic discharge: the spark from a charged finger or cable, kilovolts for nanoseconds. Chips have some protection inside; an ESD array adds more on lines a user can touch. More: [Wikipedia](https://en.wikipedia.org/wiki/Electrostatic_discharge).
- <a id="g-esd-array"></a>**ESD array**: A small chip of steering diodes that sends a strike on a data line into a supply or ground instead of the [MCU](#g-mcu). REF-A's U1 guards D+ and D-, with its reference pin on 3V3. More: [Wikipedia](https://en.wikipedia.org/wiki/Transient-voltage-suppression_diode).
- <a id="g-esp32"></a>**ESP32-S3**: Espressif's microcontroller with Wi-Fi and Bluetooth, considered and turned down in favour of the RP2350. More: [Wikipedia](https://en.wikipedia.org/wiki/ESP32).
- <a id="g-euipo"></a>**EUIPO**: The EU Intellectual Property Office, which keeps the register of EU designs and trade marks. More: [euipo.europa.eu](https://www.euipo.europa.eu/).
- <a id="g-event-bus"></a>**Event bus**: A hub where code registers for events and is called back when they fire. gen3_battle avoids one: each pipeline stage asks the [ability](#g-ability) and item code directly. More: [Wikipedia](https://en.wikipedia.org/wiki/Publish%E2%80%93subscribe_pattern).
- <a id="g-ev-iv-nature"></a>**EVs, IVs and natures**: Gen 3's hidden stat modifiers: IVs (individual values, fixed per Pokemon), EVs (effort values, earned in training; an EV spread is how they are shared out) and a nature, which raises one stat by 10% and lowers another. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Effort_values).
- <a id="g-exploded"></a>**Exploded view**: A 3D drawing with the parts pulled apart along the assembly direction so each one shows. REF-A's is published on [Pages](#g-github-pages) at `/assembly/`. More: [Wikipedia](https://en.wikipedia.org/wiki/Exploded-view_drawing).
- <a id="g-fab"></a>**Fab**: The factory that makes bare boards (and here also assembles them); the fab files or fab outputs are what is sent to it: [gerbers](#g-gerbers), drill files, the [BOM](#g-bom) and the placement file. More: [KiCad docs](https://docs.kicad.org/10.0/en/pcbnew/pcbnew.html#plotting).
- <a id="g-fcc"></a>**FCC Part 15B**: The US rules for devices that give off radio energy as a by-product of their digital circuits (unintentional radiators). More: [Wikipedia](https://en.wikipedia.org/wiki/Title_47_CFR_Part_15).
- <a id="g-fdm"></a>**FDM**: Fused deposition modelling: 3D printing by laying down melted plastic filament layer by layer. More: [Wikipedia](https://en.wikipedia.org/wiki/Fused_filament_fabrication).
- <a id="g-feature"></a>**Feature (Cargo)**: A named build option of a Rust crate. The firmware has `input-debug`, `mem-profile` and `trace`; embassy-rp picks the chip with `rp235xa` or `rp235xb`. More: [Cargo book](https://doc.rust-lang.org/cargo/reference/features.html).
- <a id="g-fiducial"></a>**Fiducial**: A bare copper dot in a [mask opening](#g-solder-mask) that the assembly machine's camera finds to know exactly where the board is. REF-A has three. More: [Wikipedia](https://en.wikipedia.org/wiki/Fiducial_marker).
- <a id="g-firmware"></a>**Firmware**: The program stored in the device's flash and run by its microcontroller; here `mega_blastoise_fw`, core plus the RP2350's raw IO. More: [Wikipedia](https://en.wikipedia.org/wiki/Firmware).
- <a id="g-fit-kit"></a>**Fit kit**: Printed board mock, part dummies and case [coupons](#g-coupon) to test real parts before boards are ordered. More: [fit kit README](hardware/ref_design/case/fitkit/README.md).
- <a id="g-fm8002a"></a>**FM8002A**: REF-A's audio amplifier (U7): [class AB](#g-class-ab), [bridge-tied](#g-btl), rated 1.1 W, run from VSYS with a 6 V maximum.
- <a id="g-footprint"></a>**Footprint**: The copper pads, with their markings, that a part solders to on the board, also called its land pattern. An [unfitted](#g-unfitted) footprint is one the BOM leaves empty. More: [KiCad docs](https://docs.kicad.org/10.0/en/pcbnew/pcbnew.html#working-with-footprints).
- <a id="g-forward-voltage"></a>**Forward voltage (Vf)**: The voltage across a diode or LED while it conducts: about 0.3 V for a Schottky, 0.6-0.7 V for a silicon diode, about 3 V for the panel's white backlight LEDs. More: [Wikipedia](https://en.wikipedia.org/wiki/Diode).
- <a id="g-fpc"></a>**FPC**: Flexible printed circuit: the panel's flat flexible cable, here 50 contacts at 0.5 mm [pitch](#g-pitch), which folds under the panel into J3. More: [Wikipedia](https://en.wikipedia.org/wiki/Flexible_electronics).
- <a id="g-framebuffer"></a>**Framebuffer**: A copy of the whole screen image in the MCU's RAM. Not needed here: the panel keeps its own frame memory, so the firmware sends only what changes. More: [Wikipedia](https://en.wikipedia.org/wiki/Framebuffer).
- <a id="g-fuzz"></a>**Fuzz testing**: Comparing the engines with Showdown on seeded [random battles](#g-randbat) (`showdown_fuzz`), with hit, [crit](#g-crit) and damage roll scripted on both sides. The fuzz pool is the set of moves it draws from; a sweep runs many seeds, 2000 before an engine change is pushed. More: [Wikipedia](https://en.wikipedia.org/wiki/Fuzzing).
- <a id="g-gamma"></a>**Gamma curve**: A non-linear mapping from a brightness setting to drive level, so that equal steps look equal to the eye. More: [Wikipedia](https://en.wikipedia.org/wiki/Gamma_correction).
- <a id="g-the-gate"></a>**The gate (`check.sh`)**: `scripts/check.sh`, run before every code commit: host tests, a firmware `cargo check`, the [WASM](#g-wasm) build and the [headless](#g-headless) [UI probe](#g-ui-probe). More: [check.sh](scripts/check.sh).
- <a id="g-gate"></a>**Gate, drain and source**: A MOSFET's three terminals. The gate's voltage, measured from the source, sets how well current flows between drain and source; the gate itself draws almost no steady current. More: [Wikipedia](https://en.wikipedia.org/wiki/MOSFET).
- <a id="g-gba"></a>**GBA**: The Game Boy Advance (2001), whose small-speaker sound is the audio target. More: [Wikipedia](https://en.wikipedia.org/wiki/Game_Boy_Advance).
- <a id="g-gen"></a>**Gen 1, Gen 3**: Generations of the Pokemon games, each with its own battle rules. Gen 1 is Red, Blue and Yellow (1996 to 1999); Gen 3 is Ruby, Sapphire, Emerald, FireRed and LeafGreen (2002 to 2004). More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Generation).
- <a id="g-gerbers"></a>**Gerbers**: The standard files a fab makes a bare board from, one per copper, mask and [silkscreen](#g-silkscreen) layer, plus drill files. More: [Wikipedia](https://en.wikipedia.org/wiki/Gerber_format).
- <a id="g-ghosting"></a>**Ghosting**: A [button matrix](#g-button-matrix) reporting a press that didn't happen, when several held buttons close a path through another position. v1's matrix ghosted presses across players. More: [Wikipedia](https://en.wikipedia.org/wiki/Keyboard_matrix_circuit).
- <a id="g-submodule"></a>**Git submodule**: A git repository checked out inside another at a pinned commit; `battler/` is one. More: [Pro Git](https://git-scm.com/book/en/v2/Git-Tools-Submodules).
- <a id="g-github-pages"></a>**GitHub Pages**: GitHub's static web hosting. `.github/workflows/pages.yml` publishes the front page, the game under `/battler/` and the assembly viewers on every push to `main`. More: [GitHub Pages](https://pages.github.com/).
- <a id="g-glass-class"></a>**Glass class**: Panels built on the same LCD glass (size, resolution, [IPS](#g-ips)) and sold by different makers with their own [FPC](#g-fpc) and pin order. REF-A's case has a fixed bay for the 3.5 in 320x480 IPS class, so a swap changes only the panel numbers, the connector and the [pin map](#g-pin-map).
- <a id="g-global-label"></a>**Global label**: A schematic label whose net is the same on every [sheet](#g-sheet). REF-A uses one for each signal that runs between sheets (48 of them). More: [KiCad docs](https://docs.kicad.org/10.0/en/eeschema/eeschema.html#labels).
- <a id="g-gpio"></a>**GPIO**: General-purpose input/output: a chip pin that firmware can read or drive. Every REF-A button has its own. More: [Wikipedia](https://en.wikipedia.org/wiki/General-purpose_input/output).
- <a id="g-grace"></a>**Grace window**: The 1 s core waits after both [seats](#g-seat) have locked in before the turn fires, so a last-moment take-back still counts.
- <a id="g-ground"></a>**Ground (GND)**: The 0 V reference every voltage is measured from. REF-A [pours](#g-pour) it on both layers and stitches the pours together with [vias](#g-via) on a 5 mm grid. More: [Wikipedia](https://en.wikipedia.org/wiki/Ground_(electricity)).
- <a id="g-half"></a>**Half**: One seat's part of the panel; the far half is drawn rotated 180 degrees so both players read their own half upright.
- <a id="g-hard-gold"></a>**Hard gold**: Thick electroplated gold for contacts that are pressed or rubbed many times; dearer than ENIG. More: [Wikipedia](https://en.wikipedia.org/wiki/Gold_plating).
- <a id="g-header"></a>**Header**: A board connector that a cable's plug mates with. The speaker plugs into J4, so nothing is soldered at assembly. More: [Wikipedia](https://en.wikipedia.org/wiki/Pin_header).
- <a id="g-headless"></a>**Headless**: Run with no window: headless Chromium plays the web build in the UI probe, and [OpenSCAD](#g-openscad) renders and checks the case from a script. More: [Wikipedia](https://en.wikipedia.org/wiki/Headless_browser).
- <a id="g-heap"></a>**Heap**: Memory handed out at run time for data whose size isn't known in advance. [`battler`](#g-battler) exhausted the RP2040's 128 KB heap; the in-house engines avoid heap allocation on the [hot path](#g-hot-path). More: [Wikipedia](https://en.wikipedia.org/wiki/Memory_management).
- <a id="g-held-item"></a>**Held item**: An item a Pokemon carries into battle, such as Leftovers. gen3_battle models them, but the device's Gen 3 draft doesn't hand them out yet. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Held_item).
- <a id="g-host"></a>**Host**: The developer's computer, as against the device. Host tests, the host CLI and the screen dumper run there with no hardware.
- <a id="g-hot-path"></a>**Hot path**: The code that runs most often, where speed and memory matter most; for the engines, every damage calculation. More: [Wikipedia](https://en.wikipedia.org/wiki/Hot_spot_(computer_programming)).
- <a id="g-hot-plug"></a>**Hot plug**: Plugging a cable into a source that is already on. The cable's inductance and the board's [ceramic capacitors](#g-ceramic-cap) ring, so the voltage can briefly peak well above 5 V (6.7 V on REF-A's VBUS in simulation). More: [Wikipedia](https://en.wikipedia.org/wiki/Hot_swapping).
- <a id="g-hp"></a>**HP**: Hit points: how much damage a Pokemon can take before it faints. Each seat's HP plate shows its own [mon](#g-mon)'s. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/HP).
- <a id="g-hud"></a>**HUD**: Heads-up display: the [HP](#g-hp) plate and status drawn over the battle scene. More: [Wikipedia](https://en.wikipedia.org/wiki/HUD_(video_games)).
- <a id="g-i2s"></a>**I2S**: A three-wire digital audio bus. The first production plan had an I2S amplifier; REF-A makes audio from a PWM pin instead, which is cheaper. More: [Wikipedia](https://en.wikipedia.org/wiki/I2S).
- <a id="g-ic"></a>**IC**: Integrated circuit: a chip. More: [Wikipedia](https://en.wikipedia.org/wiki/Integrated_circuit).
- <a id="g-ili9488"></a>**ILI9488, ST7796**: Display controllers: the chip on a panel's glass that keeps the frame in its own memory and drives the pixels, while the MCU sends commands and pixels over SPI. Stripboard v2's panel has an ST7796; REF-A's has an ILI9488, which needs a new driver.
- <a id="g-inductor"></a>**Inductor**: A coil that stores energy in a magnetic field and resists changes in current; the heart of a switching regulator. L2 is the one in the RP2354B's core switcher. More: [Wikipedia](https://en.wikipedia.org/wiki/Inductor).
- <a id="g-injection"></a>**Injection moulding**: Making plastic parts by forcing molten plastic into a steel mould: dear to tool, cheap per part, the usual route for a case at volume. More: [Wikipedia](https://en.wikipedia.org/wiki/Injection_moulding).
- <a id="g-input-buffer"></a>**Input buffer**: The digital input stage behind a GPIO pad. It is turned off on analogue pins, and pins with it on are the ones [erratum](#g-erratum) E9 affects.
- <a id="g-inrush"></a>**Inrush current**: The surge of current into a board's capacitors at the moment power arrives. More: [Wikipedia](https://en.wikipedia.org/wiki/Inrush_current).
- <a id="g-iovdd"></a>**IOVDD, DVDD**: RP2350 supply pin names: IOVDD feeds the 3.3 V I/O and DVDD the 1.1 V digital core. VREG_VIN, VREG_AVDD and VREG_LX are the internal switcher's input, [filtered](#g-low-pass) analogue supply and inductor pin. More: [RP2350 datasheet](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf).
- <a id="g-ip"></a>**IP**: Intellectual property: names, designs, code and art that others own rights in. More: [Wikipedia](https://en.wikipedia.org/wiki/Intellectual_property).
- <a id="g-ips"></a>**IPS panel**: In-plane switching LCD: colour and contrast hold from steep angles. Needed because each seat views the shared panel from its own side. More: [Wikipedia](https://en.wikipedia.org/wiki/IPS_panel).
- <a id="g-jig"></a>**Jig**: A fixture that holds a board against test pins, such as [pogo pins](#g-pogo) on J7. More: [Wikipedia](https://en.wikipedia.org/wiki/Jig_(tool)).
- <a id="g-jlc"></a>**JLCPCB (JLC)**: The board maker that fabricates and assembles REF-A; JLC for short. Its parts come from LCSC. More: [jlcpcb.com](https://jlcpcb.com/).
- <a id="g-jst"></a>**JST SH, JST PH**: Small wire-to-board connector families from JST: SH at 1.0 mm pitch (the [Debug Probe](#g-debug-probe)'s cables, J5 and J6) and PH at 2.0 mm (the cell connector REF-A dropped for its height). More: [Wikipedia](https://en.wikipedia.org/wiki/JST_connector).
- <a id="g-kicad"></a>**KiCad**: The open-source electronics design suite: schematic, board [layout](#g-layout), checks and fab outputs. REF-A uses KiCad 10 with every file generated by scripts. More: [kicad.org](https://www.kicad.org/).
- <a id="g-konnect"></a>**Konnect**: A tool that drives KiCad from scripts. REF-A's schematic was written through it until 2026-10-20; since then `build_sch.py` writes KiCad's files directly. More: [konnect_notes.md](hardware/ref_design/docs/konnect_notes.md).
- <a id="g-latch"></a>**Latch (soft power, PWR_HOLD)**: REF-A's soft power circuit: pressing seat 1's Start powers the regulator up, the MCU then holds itself on through PWR_HOLD, and dropping PWR_HOLD turns the device off once Start is released. A reset or crash drops the hold.
- <a id="g-layout"></a>**Layout**: The board design itself: where each part sits and how the copper joins them, as against the schematic, which shows only what connects to what. More: [KiCad docs](https://docs.kicad.org/10.0/en/pcbnew/pcbnew.html#placement-and-drawing-operations).
- <a id="g-lcsc"></a>**LCSC**: JLCPCB's parts distributor. Each REF-A part has an LCSC number in `design.py`. More: [lcsc.com](https://www.lcsc.com/).
- <a id="g-ldo"></a>**LDO**: Low-dropout linear regulator: makes a lower, fixed voltage by dropping the difference across a transistor as heat. Simple, quiet and cheap; REF-A's ME6211 makes 3V3 from VSYS. More: [Wikipedia](https://en.wikipedia.org/wiki/Low-dropout_regulator).
- <a id="g-leakage"></a>**Leakage**: Small unwanted current: through a [reverse-biased](#g-reverse-bias) diode (more when warm, most in Schottkys), into an unpowered chip's pins, or across a board's surface under sweat or condensation. On REF-A's high-value nodes a few uA matter. More: [Wikipedia](https://en.wikipedia.org/wiki/Leakage_(electronics)).
- <a id="g-li-ion"></a>**Li-ion cell**: A rechargeable lithium-ion cell, 4.2 V when full. REF-A uses one [403450](#g-403450) pouch cell of about 700 mAh with its own [protection board](#g-protection). More: [Wikipedia](https://en.wikipedia.org/wiki/Lithium-ion_battery).
- <a id="g-linear-charger"></a>**Linear charger**: A charger that drops the difference between USB's 5 V and the cell as heat in the chip, like an LDO: cheap and simple but warm (about 0.65 W in REF-A's TP4056 while charging). More: [Wikipedia](https://en.wikipedia.org/wiki/Battery_charger).
- <a id="g-lipo"></a>**LiPo**: A lithium-ion cell in a flat, soft pouch (lithium polymer). Stripboard v2's is a 2000 mAh 103450. More: [Wikipedia](https://en.wikipedia.org/wiki/Lithium_polymer_battery).
- <a id="g-load-cap"></a>**Load capacitance**: The capacitance a [crystal](#g-crystal) must see to run at its marked frequency. REF-A's two 15 pF capacitors in series (7.5 pF) plus about 3 pF of stray board capacitance give 10.5 pF against the crystal's 10 pF. More: [Wikipedia](https://en.wikipedia.org/wiki/Crystal_oscillator).
- <a id="g-load-sharing"></a>**Load sharing**: USB powers the device directly while the charger fills the cell. REF-A does it with D5 (USB into VSYS) and Q2 with D7 (the cell into VSYS once USB is gone); a power-path charger [IC](#g-ic) does the same inside one chip.
- <a id="g-local-label"></a>**Local label**: A schematic label that joins nets only on its own sheet; KiCad prefixes the net name with the sheet's name. REF-A has 47 local nets. More: [KiCad docs](https://docs.kicad.org/10.0/en/eeschema/eeschema.html#labels).
- <a id="g-logic-analyser"></a>**Logic analyser**: A tool that records many digital lines at once and decodes buses such as SPI. TP11 to TP14 give one the panel's four SPI lines. More: [Wikipedia](https://en.wikipedia.org/wiki/Logic_analyzer).
- <a id="g-low-pass"></a>**Low-pass filter (RC filter)**: A resistor and a capacitor that pass slow signals and remove fast ones above the corner frequency (the -3 [dB](#g-db) point, 1 / (2 pi R C)). REF-A uses them to turn PWM into a steady reference (BL_REF) or into audio. More: [Wikipedia](https://en.wikipedia.org/wiki/Low-pass_filter).
- <a id="g-lx"></a>**LX (switch node)**: The pin where a switching regulator's inductor connects. It swings hard at the switching frequency, so its copper is kept short; VREG_LX is the RP2354B's.
- <a id="g-m2"></a>**M2 screw**: A metric screw 2 mm across the thread. Six hold REF-A's case together through the board, as on a DS Lite. More: [Wikipedia](https://en.wikipedia.org/wiki/ISO_metric_screw_thread).
- <a id="g-mcu"></a>**MCU (microcontroller)**: A single chip with a processor, RAM, often flash, and peripherals such as GPIO, [ADC](#g-adc), SPI and USB; here the RP2040, RP2350 or RP2354B. More: [Wikipedia](https://en.wikipedia.org/wiki/Microcontroller).
- <a id="g-me6211"></a>**ME6211**: REF-A's 3.3 V LDO (U3): 500 mA maximum, 120 mV [dropout](#g-dropout) at 100 mA, switched on and off by PWR_EN.
- <a id="g-membrane"></a>**Membrane**: The silicone button pad: one moulded piece per button cluster, with a dome and a carbon pill over each contact. REF-A uses genuine-style DS Lite spares on bare PCB contacts. More: [hardware/membrane](hardware/membrane/README.md).
- <a id="g-memory-map"></a>**Memory map**: The table that tells the linker where flash and RAM sit and how big they are. REF-A's firmware needs one with 2048K of flash. More: [Wikipedia](https://en.wikipedia.org/wiki/Memory_map).
- <a id="g-minimal-design"></a>**Minimal design**: Raspberry Pi's published reference board for the RP2350B with only the parts it needs, described in its hardware design guide. REF-A copies its core circuit and layout under its [MIT licence](#g-mit). More: [Hardware design with RP2350](https://datasheets.raspberrypi.com/rp2350/hardware-design-with-rp2350.pdf).
- <a id="g-mit"></a>**MIT licence**: A permissive open-source licence: reuse is allowed if the copyright notice and licence text go along. The copied RP2350B core is under it. More: [Wikipedia](https://en.wikipedia.org/wiki/MIT_License).
- <a id="g-mixer"></a>**Mixer**: Code that adds several sounds into one output stream. None exists yet; when written it goes in core, so the browser sounds like the device.
- <a id="g-module"></a>**Module**: A small ready-made board carrying a chip and its support parts. The [Pico](#g-pico) is an MCU module; stripboard v2's TP4056 charger and ST7796 panel are modules too. JLC will place no RP2350B module, so REF-A uses the bare chip.
- <a id="g-mon"></a>**Mon**: Short for a Pokemon, as the code and the UI text use it.
- <a id="g-monotonic"></a>**Monotonic clock**: A clock that only counts up and is never adjusted, used for timing (the 500 ms hold, the 1 s grace). Each platform supplies one. More: [Rust docs](https://doc.rust-lang.org/std/time/struct.Instant.html).
- <a id="g-fet"></a>**MOSFET (FET)**: A transistor whose [gate](#g-gate) voltage controls the current between drain and source. N-channel ones (2N7002, AO3400A) turn on with the gate above the source; P-channel ones (AO3401A) with the gate below it. More: [Wikipedia](https://en.wikipedia.org/wiki/MOSFET).
- <a id="g-net"></a>**Net**: A set of pins that are electrically joined. Each net has a name (REF-A's `design.py` has 101 named nets) that the schematic, the [netlist](#g-netlist) and the board all share. More: [Wikipedia](https://en.wikipedia.org/wiki/Netlist).
- <a id="g-netlist"></a>**Netlist**: The list of every net and the pins on it. `check_netlist.py` compares the one KiCad exports with `design.py`. More: [KiCad docs](https://docs.kicad.org/10.0/en/eeschema/eeschema.html#netlist-export).
- <a id="g-nightly"></a>**Nightly Rust**: Rust's daily build, which allows unstable features. The project pins one in `rust-toolchain.toml` so builds repeat. More: [rustup book](https://rust-lang.github.io/rustup/concepts/channels.html).
- <a id="g-no-std"></a>**`no_std`**: A Rust crate that doesn't use the standard library (no operating system, files or threads), so it runs on a bare microcontroller. Core and the engines are `no_std`. More: [Embedded Rust book](https://docs.rust-embedded.org/book/intro/no-std.html).
- <a id="g-node"></a>**Node**: Node.js, JavaScript outside a browser. The [parity](#g-parity) harness runs the real Showdown under it. More: [nodejs.org](https://nodejs.org/).
- <a id="g-thermistor"></a>**NTC thermistor**: A resistor whose value falls as it warms; B3380 is the steepness of its curve (its B value). TH1 (10k) lets the TP4056 charge only between about 1 and 46 C. More: [Wikipedia](https://en.wikipedia.org/wiki/Thermistor).
- <a id="g-offset"></a>**Offset (op-amp)**: The small error an op-amp adds between its inputs, up to 4.5 mV for REF-A's MCP6002. At low backlight levels it is a large share of the reference, so the two LED strings can differ. More: [Wikipedia](https://en.wikipedia.org/wiki/Input_offset_voltage).
- <a id="g-oled"></a>**OLED**: Organic LED display: each pixel makes its own light. v1 had one 128x64 mono OLED per player. More: [Wikipedia](https://en.wikipedia.org/wiki/OLED).
- <a id="g-oledcontroller"></a>**`OledController` / `Screen`**: Core's per-seat screen state machine and its list of screens. The name dates from the two-OLED device.
- <a id="g-on-resistance"></a>**On-resistance**: A switched-on MOSFET's resistance from drain to source, which falls as the gate drive rises. Q2's is at most 85 mOhm with 2.5 V of gate drive. More: [Wikipedia](https://en.wikipedia.org/wiki/Power_MOSFET).
- <a id="g-op-amp"></a>**Op-amp**: Operational amplifier: a high-gain amplifier that, with feedback, drives its output until its two inputs match. In each backlight sink, U6 drives a MOSFET until the sense resistor's voltage equals BL_REF. More: [Wikipedia](https://en.wikipedia.org/wiki/Operational_amplifier).
- <a id="g-open-information"></a>**Open information**: The rule that each player sees the rival's active mon, its HP and status; only a locked-in move stays hidden until playback. More: [Section 2](#open-information).
- <a id="g-open-drain"></a>**Open-drain**: An output that can only pull low or let go; a pull-up resistor provides the high. The TP4056's CHRG and STDBY status pins are open-drain. More: [Wikipedia](https://en.wikipedia.org/wiki/Open_collector).
- <a id="g-openocd"></a>**OpenOCD**: An open-source on-chip debugger, an alternative to [probe-rs](#g-probe-rs) for SWD. More: [GitHub](https://github.com/openocd-org/openocd).
- <a id="g-openscad"></a>**OpenSCAD**: A CAD tool where solids are written as code. The cases are OpenSCAD files, rendered and checked by script. More: [openscad.org](https://openscad.org/).
- <a id="g-scope"></a>**Oscilloscope (scope)**: An instrument that plots voltage against time; scoping a signal means watching it on one. Its ground clip goes to a GND pad such as TP15. More: [Wikipedia](https://en.wikipedia.org/wiki/Oscilloscope).
- <a id="g-otp"></a>**OTP**: One-time programmable memory in the RP2350 that holds keys and settings; each bit can be written only once. More: [Wikipedia](https://en.wikipedia.org/wiki/Programmable_ROM).
- <a id="g-palette"></a>**Palette**: The short list of colours an image's pixels index into. Each Gen 3 sprite is stored at 4 bits a pixel (16 colours) with its own palette. More: [Wikipedia](https://en.wikipedia.org/wiki/Indexed_color).
- <a id="g-panel"></a>**Panel**: The display itself: LCD glass with its controller, backlight and FPC. A panel on a small carrier board with pins, as on stripboard v2, is a display [module](#g-module). More: [Wikipedia](https://en.wikipedia.org/wiki/Liquid-crystal_display).
- <a id="g-panic"></a>**Panic**: A Rust program stopping on an unrecoverable error. On the Pico 2 a panic lights GP25 and its message survives a reset. More: [Rust book](https://doc.rust-lang.org/book/ch09-01-unrecoverable-errors-with-panic.html).
- <a id="g-parity"></a>**Parity tests**: Differential tests of the engines against the real Showdown on fixed, hand-written scenarios (`showdown_parity`). The fuzzer does the same with seeded random ones. More: [tools/showdown-parity](tools/showdown-parity/README.md).
- <a id="g-pass-transistor"></a>**Pass transistor (linear region)**: A transistor in series with a load that drops the extra voltage itself. Run in its linear region, a MOSFET acts as a resistor set by its gate, which is how Q5 and Q6 hold the backlight current. More: [Wikipedia](https://en.wikipedia.org/wiki/MOSFET).
- <a id="g-pcb"></a>**PCB**: Printed circuit board: copper tracks on a fibreglass board, with parts soldered on. More: [Wikipedia](https://en.wikipedia.org/wiki/Printed_circuit_board).
- <a id="g-pcba"></a>**PCBA**: PCB assembly: the fab also buys and solders the parts. JLC's Economic PCBA, used for REF-A, assembles one side. More: [JLCPCB](https://jlcpcb.com/capabilities/pcb-assembly-capabilities).
- <a id="g-phase-margin"></a>**Phase margin**: How far a feedback loop is from oscillating, in degrees; 45 or more is usually comfortable. The backlight sinks have 72.5 or more at every simulated corner. More: [Wikipedia](https://en.wikipedia.org/wiki/Phase_margin).
- <a id="g-pico"></a>**Pico, Pico 2**: Raspberry Pi's MCU modules: the Pico carries an RP2040 and the Pico 2 an RP2350A with 4 MB of flash. v1 ran on a Pico; stripboard v2 runs on a Pico 2. More: [Raspberry Pi](https://www.raspberrypi.com/documentation/microcontrollers/pico-series.html).
- <a id="g-picoblade"></a>**PicoBlade**: Molex's 1.25 mm pitch wire-to-board connector, often sold as MX1.25. REF-A's cell (J2) and speaker (J4) plug in on it. More: [Molex](https://www.molex.com/en-us/products/connectors/wire-to-board-connectors/picoblade-connectors).
- <a id="g-escape"></a>**Pin escape (fan-out)**: Routing a track out from between a chip's pins to where there is room. REF-A copies Raspberry Pi's escapes along with the core layout.
- <a id="g-pin-map"></a>**Pin map**: The table of which chip pin carries which signal. REF-A's is `pinmap.md`; stripboard v2's is section 10 of BUILD.md. More: [pinmap.md](hardware/ref_design/docs/pinmap.md).
- <a id="g-pin-review"></a>**Pin review**: REF-A's check of every assembled part's every pin against its current datasheet (`check_pins.py`, results in `board/pin_review/`), a gate since 2026-10-05.
- <a id="g-pitch"></a>**Pitch**: The centre-to-centre spacing of pins or contacts: 0.5 mm on the panel's FPC, 2.54 mm (0.1 in) on [stripboard](#g-stripboard).
- <a id="g-pla"></a>**PLA**: Polylactic acid, the easiest common 3D-printing plastic; REF-A's case parts print in it. More: [Wikipedia](https://en.wikipedia.org/wiki/Polylactic_acid).
- <a id="g-placement-file"></a>**Placement file**: The list of each part's position, rotation and side that the assembly machine follows (JLC calls it the CPL). More: [KiCad docs](https://docs.kicad.org/10.0/en/pcbnew/pcbnew.html#component_placement_position_files).
- <a id="g-pick-place"></a>**Placement machine**: The pick-and-place machine that sets parts on boards for assembly. It finds the board by its [fiducials](#g-fiducial) and works from the placement file. More: [Wikipedia](https://en.wikipedia.org/wiki/Pick-and-place_machine).
- <a id="g-plan-view"></a>**Plan corners**: The case's corners as seen from directly above (in plan), here rounded to 4.3 mm.
- <a id="g-platform"></a>**Platform**: The firmware, web or host crate: raw IO around core (press classification, frames, log lines, a clock).
- <a id="g-pogo"></a>**Pogo pin**: A spring-loaded pin that presses on a pad for a temporary connection; J7's six pads take them. More: [Wikipedia](https://en.wikipedia.org/wiki/Pogo_pin).
- <a id="g-pokeapi"></a>**PokeAPI**: An open Pokemon data project whose sprite repository supplies the Gen 3 (Emerald) art. More: [GitHub](https://github.com/PokeAPI/sprites).
- <a id="g-polarity"></a>**Polarity**: Which way round a part or plug goes. A polarised part (a diode, a chip, L2) works only one way; [PicoBlade](#g-picoblade) cell plugs have no standard polarity, so the red lead must land on J2's pin 1.
- <a id="g-pour"></a>**Pour**: A copper area filling the free space on a layer, joined to a net, usually ground. REF-A pours ground on both layers and 3V3 over the chip. More: [KiCad docs](https://docs.kicad.org/10.0/en/pcbnew/pcbnew.html#working-with-zones).
- <a id="g-power-flag"></a>**Power flag (PWR_FLAG)**: A KiCad marker that tells [ERC](#g-erc) a net is driven by a source it can't see, such as a rail fed through a jumper, diode or FET. REF-A's eight are the `#FLG` [symbols](#g-symbol). More: [KiCad docs](https://docs.kicad.org/10.0/en/eeschema/eeschema.html#pwr-flag).
- <a id="g-power-symbol"></a>**Power symbol**: A schematic symbol that names a rail (VSYS, +3V3, GND). Every power symbol with the same name is the same net on every sheet. More: [KiCad docs](https://docs.kicad.org/10.0/en/eeschema/eeschema.html#power-symbols).
- <a id="g-ppm"></a>**PPM**: A minimal uncompressed image format, easy to write from a host program; the `screens` dumper writes it. More: [Wikipedia](https://en.wikipedia.org/wiki/Netpbm).
- <a id="g-pre-push"></a>**Pre-push hook**: A git script that runs before each push; here it rebuilds the assembly viewer. More: [Pro Git](https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks).
- <a id="g-probe-pad"></a>**Probe pad**: A bare copper pad on a net for a meter or [scope](#g-scope) probe (TP1 to TP16 on REF-A). More: [Wikipedia](https://en.wikipedia.org/wiki/Test_point).
- <a id="g-probe-rs"></a>**probe-rs**: A Rust tool that flashes and debugs chips over SWD and shows their [RTT](#g-rtt) and [defmt](#g-defmt) output. More: [probe.rs](https://probe.rs/).
- <a id="g-prod-board"></a>**prod_board, prod_case**: Alex's own production board (KiCad) and case (OpenSCAD), made by hand to learn the tools. Other work reads them and never edits them. More: [prod_case README](hardware/prod_case/README.md).
- <a id="g-protection"></a>**Protection board**: A small circuit on a lithium cell that cuts it off on over-charge, over-discharge or a short; a tripped pack has cut itself off. REF-A's cell has one; stripboard v2's TP4056 module has one built in. More: [Wikipedia](https://en.wikipedia.org/wiki/Lithium-ion_battery).
- <a id="g-proud"></a>**Proud**: Standing out above a surface: REF-A's caps stand 1.3 mm proud of the case.
- <a id="g-pull-up"></a>**Pull-up and pull-down**: A resistor from a pin to the supply (pull-up) or to ground (pull-down) that sets its level when nothing drives it. RP2350 pins have weak internal ones, and every GPIO resets with its pull-down on. More: [Wikipedia](https://en.wikipedia.org/wiki/Pull-up_resistor).
- <a id="g-pwm"></a>**PWM**: Pulse-width modulation: a fast on-off signal whose [duty cycle](#g-duty) stands for a level. Filtered, it gives a steady voltage (the backlight's set point) or audio. On the RP2350, PWM4 A means channel A of PWM slice 4. More: [Wikipedia](https://en.wikipedia.org/wiki/Pulse-width_modulation).
- <a id="g-qfn"></a>**QFN**: Quad flat no-leads: a square chip package with pads under its edges and usually a large centre pad. The RP2354B is a QFN-80. More: [Wikipedia](https://en.wikipedia.org/wiki/Flat_no-leads_package).
- <a id="g-qspi"></a>**QSPI**: Quad SPI: SPI with four data lines, used between a chip and its flash. On the RP2354B only the flash's [chip select](#g-chip-select), QSPI_SS, comes out. More: [Wikipedia](https://en.wikipedia.org/wiki/Serial_Peripheral_Interface).
- <a id="g-rail"></a>**Rail**: A supply net that feeds many parts, such as VSYS or +3V3. A sub-rail is a branch of one behind a solder jumper.
- <a id="g-randbat"></a>**Random battle (randbat)**: Showdown's format in which each player gets a random team drawn from curated sets. The device drafts a fresh team every battle, from the randbat data for Gen 1 and the [vendored](#g-vendored) sets for Gen 3. More: [pkmn randbats](https://pkmn.github.io/randbats/).
- <a id="g-ref-a"></a>**Reference design (REF-A)**: Claude's complete, orderable version of the device in `hardware/ref_design/`, generated from `design.py`, `placement.py` and `case.scad` (section 7). More: [Section 7](#7-the-reference-design-ref-a).
- <a id="g-ref-des"></a>**Reference designator**: A part's label on the schematic and board: R for resistors, C capacitors, U chips, Q transistors, D diodes, J connectors, SW switches, TP test points, JP jumpers. REF-A has 173. More: [KiCad docs](https://docs.kicad.org/10.0/en/eeschema/eeschema.html#reference-designators-and-symbol-annotation).
- <a id="g-registered-design"></a>**Registered design (EU)**: The EU's protection for how a product looks, renewable up to 25 years. An [EUIPO](#g-euipo) search would show whether the DS Lite has one. More: [Wikipedia](https://en.wikipedia.org/wiki/European_Union_design).
- <a id="g-regulator"></a>**Regulator**: A circuit that makes a steady supply voltage from a varying one. REF-A has two: U3, the ME6211 LDO that makes 3V3, and the RP2354B's internal switcher that makes 1.1 V. More: [Wikipedia](https://en.wikipedia.org/wiki/Voltage_regulator).
- <a id="g-residual"></a>**Residual order**: The fixed order in which end-of-turn effects ([weather](#g-weather) damage, Leftovers, poison and so on) resolve. gen3_battle follows Showdown's.
- <a id="g-respin"></a>**Respin**: A revised version of a board, ordered to fix what the first one got wrong.
- <a id="g-reverse-bias"></a>**Reverse bias**: A diode with its [cathode](#g-anode) above its anode, so it blocks apart from a little [leakage](#g-leakage). More: [Wikipedia](https://en.wikipedia.org/wiki/Diode).
- <a id="g-rework"></a>**Rework**: Changing an assembled board by hand: lifting, replacing or adding parts with a soldering iron. More: [Wikipedia](https://en.wikipedia.org/wiki/Rework_(electronics)).
- <a id="g-ringing"></a>**Ringing**: A voltage overshooting and oscillating after a sudden change, as a cable's inductance and a ceramic capacitor trade energy at a hot plug. A small series resistor (a damper, such as R41 or R47) absorbs it. More: [Wikipedia](https://en.wikipedia.org/wiki/Ringing_(signal)).
- <a id="g-ripple"></a>**Ripple**: The leftover wobble on a filtered or regulated voltage; BL_REF keeps 0.3% of the PWM's. More: [Wikipedia](https://en.wikipedia.org/wiki/Ripple_(electrical)).
- <a id="g-root-sheet"></a>**Root sheet**: The top page of a hierarchical KiCad schematic, holding a [sheet symbol](#g-sheet-symbol) for each [sub-sheet](#g-sub-sheet). REF-A's is `battler.kicad_sch`, with the [title note](#g-title-note), the eight sheet symbols and the mechanical parts. More: [KiCad docs](https://docs.kicad.org/10.0/en/eeschema/eeschema.html#hierarchical-schematics).
- <a id="g-routing"></a>**Routing**: Drawing the copper tracks that join each net's pads. REF-A's is done by a script (the router) and must fit two layers. More: [KiCad docs](https://docs.kicad.org/10.0/en/pcbnew/pcbnew.html#routing-tracks).
- <a id="g-rp2040"></a>**RP2040**: Raspberry Pi's first microcontroller (two Cortex-M0+ cores, 264 KB of RAM, no internal flash), on the Pico that ran v1. More: [Wikipedia](https://en.wikipedia.org/wiki/RP2040).
- <a id="g-rp2350"></a>**RP2350**: Raspberry Pi's 2024 microcontroller: two Cortex-M33 or RISC-V cores, 520 KB of RAM, no internal flash. The A variant has 30 GPIOs in a QFN-60, the B variant 48 in a [QFN-80](#g-qfn). More: [Raspberry Pi](https://www.raspberrypi.com/products/rp2350/).
- <a id="g-rp2354b"></a>**RP2354B**: Raspberry Pi's RP2350B microcontroller with 2 MB of flash in the package; REF-A's MCU (U4). More: [RP2350 datasheet](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf).
- <a id="g-rtt"></a>**RTT**: Real-time transfer: a debug channel through the SWD probe that carries log output at little cost to the running program. More: [SEGGER](https://www.segger.com/products/debug-probes/j-link/technology/about-real-time-transfer/).
- <a id="g-run"></a>**RUN**: The RP2350's reset input: held low, the chip stays in reset. It has an internal pull-up; SW19 (RESET) pulls it to ground. More: [RP2350 datasheet](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf).
- <a id="g-saturation"></a>**Saturation current**: The current at which an inductor's core saturates and its inductance collapses. L2 must handle at least 1.5 A. More: [Wikipedia](https://en.wikipedia.org/wiki/Saturation_(magnetic)).
- <a id="g-schematic"></a>**Schematic**: The circuit drawn as symbols joined by wires and labels, with no regard to where parts sit on the board. More: [Wikipedia](https://en.wikipedia.org/wiki/Schematic).
- <a id="g-schematic-parity"></a>**Schematic parity**: KiCad's check that the board and the schematic agree: the same parts, [footprints](#g-footprint) and nets. More: [KiCad docs](https://docs.kicad.org/10.0/en/pcbnew/pcbnew.html#schematic_parity_drc_checks).
- <a id="g-schottky"></a>**Schottky diode**: A diode with a low [forward voltage](#g-forward-voltage) (about 0.3 V) and fast switching, but more reverse leakage, worse when warm. REF-A uses them in the USB path (D5), the unplug gap (D7) and the power [OR](#g-diode-or) (D2, D3). More: [Wikipedia](https://en.wikipedia.org/wiki/Schottky_diode).
- <a id="g-seam"></a>**Seam**: The line where two case parts meet. On the DS Lite and REF-A it is on the face rather than around the edge.
- <a id="g-seat"></a>**Seat**: One player's end of the device: seat 1 and seat 2 (P1 and P2 in code).
- <a id="g-sectioning"></a>**Sectioning**: Cutting through a board and polishing the cut to inspect its copper, plating and solder joints under a microscope.
- <a id="g-seed"></a>**Seed**: The number that starts a pseudo-random sequence, so a random battle or test replays exactly when given the same seed. More: [Wikipedia](https://en.wikipedia.org/wiki/Random_seed).
- <a id="g-sense-resistor"></a>**Sense resistor**: A small resistor whose voltage shows the current through it (V = I x R). R52 and R53 (2.2 ohm) give 0.124 V at 56 mA. More: [Wikipedia](https://en.wikipedia.org/wiki/Shunt_(electrical)).
- <a id="g-share-alike"></a>**Share-alike, non-commercial**: Licence terms on some open designs: share-alike makes derived work carry the same licence, and non-commercial forbids selling it. Either would block a product, so the project avoids such footprints and models. More: [Wikipedia](https://en.wikipedia.org/wiki/Creative_Commons_license).
- <a id="g-sheet"></a>**Sheet**: One page of a KiCad schematic. REF-A has a [root sheet](#g-root-sheet) and eight sub-sheets, one per functional block. More: [KiCad docs](https://docs.kicad.org/10.0/en/eeschema/eeschema.html#hierarchical-schematics).
- <a id="g-sheet-note"></a>**Sheet note**: A text note drawn on a schematic sheet. On REF-A's root sheet each sheet symbol has a four-line note of the blocks inside it, and each sub-sheet carries a short note of its own. More: [KiCad docs](https://docs.kicad.org/10.0/en/eeschema/eeschema.html#graphics_and_text).
- <a id="g-sheet-symbol"></a>**Sheet symbol**: The box on a parent sheet that stands for a sub-sheet and names its file. More: [KiCad docs](https://docs.kicad.org/10.0/en/eeschema/eeschema.html#drawing-hierarchical-sheets).
- <a id="g-showdown"></a>**Showdown**: Pokemon Showdown, the open-source battle simulator used as the reference. Its source is read before a mechanic is implemented, and it wins every disagreement. More: [GitHub](https://github.com/smogon/pokemon-showdown).
- <a id="g-silkscreen"></a>**Silkscreen**: The printed lettering and markings on a board: [reference designators](#g-ref-des), pin 1 marks, the `+` by J2. More: [Wikipedia](https://en.wikipedia.org/wiki/Printed_circuit_board).
- <a id="g-ngspice"></a>**Simulation (SPICE, ngspice)**: Computing a circuit's behaviour from models of its parts before it is built. ngspice is the open-source SPICE simulator REF-A's backlight sinks were checked in. A simulation is evidence, not a measurement. More: [ngspice](https://ngspice.sourceforge.io/).
- <a id="g-singles"></a>**Singles battle**: One Pokemon active on each side at a time. Doubles, with two a side, are out of scope. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Single_Battle).
- <a id="g-snap-fit"></a>**Snap fit**: Plastic parts that click together on flexing hooks or lips, with no screws. More: [Wikipedia](https://en.wikipedia.org/wiki/Snap-fit).
- <a id="g-snes"></a>**SNES**: The Super Nintendo, whose controller membranes were the other main donor considered. More: [Wikipedia](https://en.wikipedia.org/wiki/Super_Nintendo_Entertainment_System).
- <a id="g-soic"></a>**SOIC, ESOP**: Small-outline chip packages with gull-wing leads on two sides; ESOP adds an exposed [thermal pad](#g-thermal-pad) underneath. The FM8002A is a SOIC-8, the TP4056 an ESOP-8. More: [Wikipedia](https://en.wikipedia.org/wiki/Small_outline_integrated_circuit).
- <a id="g-solder-jumper"></a>**Solder jumper (cut jumper)**: Two pads joined by a thin copper trace that can be cut with a knife and closed again with solder. REF-A's JP1 to JP5 sit in the supply paths, so cutting one isolates a block or lets a meter in series.
- <a id="g-solder-mask"></a>**Solder mask**: The thin coloured coating over a board's copper that stops solder bridging. A mask opening leaves the copper bare, as over each membrane contact. More: [Wikipedia](https://en.wikipedia.org/wiki/Solder_mask).
- <a id="g-sot23"></a>**SOT-23**: A small plastic package with three leads for transistors and diodes; SOT-23-5 and SOT-23-6 have five and six, for small chips. More: [Wikipedia](https://en.wikipedia.org/wiki/Small-outline_transistor).
- <a id="g-source-sink"></a>**Source and sink (USB-C)**: A USB-C source supplies power (a charger, a computer's port) and a sink takes it, as REF-A does. A source turns VBUS on only once it sees a sink's Rd resistors. More: [Wikipedia](https://en.wikipedia.org/wiki/USB-C).
- <a id="g-source-termination"></a>**Source termination**: A resistor in series at the driving end of a fast signal line that damps reflections and [ringing](#g-ringing); R28 and R29 (33 ohm) on the panel's SPI clock and data. More: [Wikipedia](https://en.wikipedia.org/wiki/Signal_reflection).
- <a id="g-special-split"></a>**Special split**: Gen 1 has a single Special stat; from Gen 2 on it is split into Special Attack and Special Defense, and gen3_battle models the split. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Stat).
- <a id="g-spi"></a>**SPI**: Serial Peripheral Interface: a clocked serial bus with a clock (SCK), data out (MOSI), data in (MISO) and a chip select per device. The panel's 4-wire SPI adds a data/command line (DC). More: [Wikipedia](https://en.wikipedia.org/wiki/Serial_Peripheral_Interface).
- <a id="g-sprite"></a>**Sprite**: A small picture drawn over the scene, here each Pokemon's. The sprite bob is its idle up-and-down bounce. More: [Wikipedia](https://en.wikipedia.org/wiki/Sprite_(computer_graphics)).
- <a id="g-standard-process"></a>**Standard process**: A fab's cheapest set of limits (smallest hole, track and gap); going below them costs more. REF-A keeps every hole 0.3 mm or larger to stay on JLC's. More: [JLCPCB](https://jlcpcb.com/capabilities/pcb-capabilities).
- <a id="g-stepping"></a>**Stepping**: A chip's silicon revision, such as the RP2350's A2. Some errata apply to one stepping only. More: [Wikipedia](https://en.wikipedia.org/wiki/Stepping_level).
- <a id="g-strap"></a>**Strap**: A resistor that fixes a pin's level so the chip or the firmware reads it at start-up. QSPI_SS is the BOOTSEL strap; R48 to R51 set REF-A's 4-bit board revision.
- <a id="g-stripboard"></a>**Stripboard**: Prototyping board with holes on a 2.54 mm grid and copper strips joining each row; cut the strips and solder in parts and wires to make a circuit. More: [Wikipedia](https://en.wikipedia.org/wiki/Stripboard).
- <a id="g-stripboard-v2"></a>**Stripboard v2**: The single-screen prototype that runs today: a Pico 2, a 3.5 in ST7796 panel, 16 [tactile switches](#g-tactile) and a [LiPo](#g-lipo) on one 24x56 stripboard, all generated by `layout.py`. More: [BUILD.md](hardware/stripboard_v2/BUILD.md).
- <a id="g-sub-sheet"></a>**Sub-sheet**: A schematic sheet placed inside another through a sheet symbol. REF-A's eight hang off the root sheet. More: [KiCad docs](https://docs.kicad.org/10.0/en/eeschema/eeschema.html#drawing-hierarchical-sheets).
- <a id="g-supports"></a>**Supports**: Temporary printed scaffolding under the overhanging parts of a 3D print, broken off afterwards. Parts designed to need none come out cleaner. More: [Wikipedia](https://en.wikipedia.org/wiki/Fused_filament_fabrication).
- <a id="g-swd"></a>**SWD**: Serial Wire Debug: ARM's two-wire debug port (SWCLK, SWDIO) for loading firmware, pausing the chip and reading its memory. More: [Wikipedia](https://en.wikipedia.org/wiki/JTAG#Serial_Wire_Debug).
- <a id="g-switcher"></a>**Switching regulator (switcher)**: A regulator that switches current through an inductor at high speed: efficient across a large voltage gap, but noisy and sensitive to layout. The RP2354B's internal one makes its 1.1 V core supply from 3V3 through L2. More: [Wikipedia](https://en.wikipedia.org/wiki/Switched-mode_power_supply).
- <a id="g-symbol"></a>**Symbol**: A part's drawing in the schematic, with its pins; each is tied to a footprint for the board. More: [KiCad docs](https://docs.kicad.org/10.0/en/eeschema/eeschema.html#working-with-symbols).
- <a id="g-tactile"></a>**Tactile switch**: A small push button with a click, soldered to a board. Stripboard v2 has 16; REF-A has two (BOOTSEL and RESET). More: [Wikipedia](https://en.wikipedia.org/wiki/Push-button).
- <a id="g-tearing"></a>**Tearing (TE)**: A visible split when the screen is redrawn partway through being shown. The panel's TE (tearing effect) output pulses at each refresh so the firmware can start frames in step. More: [Wikipedia](https://en.wikipedia.org/wiki/Screen_tearing).
- <a id="g-thermal-pad"></a>**Thermal pad**: A large pad under a chip that conducts heat into the board's copper. The TP4056's has six vias into the ground pours. More: [Wikipedia](https://en.wikipedia.org/wiki/Flat_no-leads_package).
- <a id="g-threshold"></a>**Threshold voltage**: The gate-to-source voltage at which a MOSFET starts to conduct. More: [Wikipedia](https://en.wikipedia.org/wiki/Threshold_voltage).
- <a id="g-time-constant"></a>**Time constant**: R x C, the time an RC circuit takes to cover about 63% of a step. The cell divider's is 23.5 ms. More: [Wikipedia](https://en.wikipedia.org/wiki/RC_time_constant).
- <a id="g-title-block"></a>**Title block**: The box in the corner of every KiCad sheet's frame that holds the sheet's title, the company or project, the date and the revision. More: [KiCad docs](https://docs.kicad.org/10.0/en/eeschema/eeschema.html#sheet-title-block).
- <a id="g-title-note"></a>**Title note**: The text at the top of REF-A's root sheet: the project's name, a two-line summary of the board, and the rule that every sheet is generated from `design.py`, so `design.py` is what gets edited.
- <a id="g-tn"></a>**TN panel**: Twisted nematic LCD: cheap, but its contrast inverts when viewed from the wrong side, so the far seat would see a negative. More: [Wikipedia](https://en.wikipedia.org/wiki/Twisted_nematic_field_effect).
- <a id="g-tp4056"></a>**TP4056**: A cheap [linear Li-ion charger](#g-linear-charger) chip that charges to 4.2 V. Stripboard v2 uses a protected TP4056 module; REF-A uses the bare chip with [load sharing](#g-load-sharing) around it. Its pins: CHRG and STDBY report charging and done, TEMP takes the [thermistor](#g-thermistor), [CE](#g-enable) enables it.
- <a id="g-trait"></a>**Trait**: A Rust interface: a set of functions a type promises to provide. Each platform implements core's [`DevicePort`](#g-devicerunner) trait. More: [Rust book](https://doc.rust-lang.org/book/ch10-02-traits.html).
- <a id="g-tub-lid"></a>**Tub and lid**: REF-A's two main case shells: the tub is the bottom, carrying the board with the cell and speaker on its floor; the lid closes over it with the panel in a pocket.
- <a id="g-tvs"></a>**TVS diode**: Transient-voltage-suppression diode: clamps spikes above its working voltage. REF-A has none on VBUS, because an SMF5.0A's 5.0 V working voltage is below USB's 5.5 V. More: [Wikipedia](https://en.wikipedia.org/wiki/Transient-voltage-suppression_diode).
- <a id="g-type"></a>**Type**: Each Pokemon and move has a type (Fire, Water and so on): 15 in Gen 1, 17 in Gen 3. The type chart sets how effective each attacking type is against each defending one; `battle_types` holds the shared list. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Type).
- <a id="g-uart"></a>**UART**: A simple serial port with one wire each way (TX out, RX in) and no clock. REF-A's console is UART1 on J6. More: [Wikipedia](https://en.wikipedia.org/wiki/Universal_asynchronous_receiver-transmitter).
- <a id="g-ui-probe"></a>**UI probe**: `scripts/ui-probe.sh`: boots the web build in headless Chromium on a pinned seed, plays real turns in both generations, and asserts on a per-seat state trace. More: [ui-probe.sh](scripts/ui-probe.sh).
- <a id="g-un383"></a>**UN38.3**: The UN transport test standard for lithium cells; the cell supplier provides the test documents. More: [Wikipedia](https://en.wikipedia.org/wiki/UN38.3).
- <a id="g-unfitted"></a>**Unfitted (DNP)**: A footprint the BOM deliberately leaves empty (do not populate), for a part fitted by hand only if needed or a strap bit that should read 1. REF-A has four: C41 to C43 and R48.
- <a id="g-unintentional"></a>**Unintentional radiator**: A device that gives off radio energy only as a by-product of its digital circuits. With no radio, the device stays in this simpler class for FCC and [CE marking](#g-ce-marking). More: [Wikipedia](https://en.wikipedia.org/wiki/Title_47_CFR_Part_15).
- <a id="g-units"></a>**Units as written here**: This document writes u for micro (uA, uF, uH), a bare k for kilohms in resistor values (10k), mOhm for milliohms, +- for plus or minus, and C after a temperature for degrees Celsius.
- <a id="g-usb-cdc"></a>**USB CDC**: The USB class for a serial port; the firmware's console shows up on the computer as one. More: [Wikipedia](https://en.wikipedia.org/wiki/USB_communications_device_class).
- <a id="g-usb-pd"></a>**USB PD**: USB Power Delivery: a negotiation over the CC line for more than 5 V or 3 A. REF-A doesn't use it. More: [Wikipedia](https://en.wikipedia.org/wiki/USB_hardware#USB_Power_Delivery).
- <a id="g-usb-c"></a>**USB-C**: The reversible USB connector. REF-A's J1 carries charging, firmware loading and the console, on USB 2.0 data; its SBU (sideband) pins are left open. More: [Wikipedia](https://en.wikipedia.org/wiki/USB-C).
- <a id="g-v1"></a>**v1**: The first device (April to August 2026): a Pico, one 128x64 [OLED](#g-oled) per player, LED strips and a button matrix. It is gone from the tree; tag `v1-archive` keeps it. More: [v1 notes](docs/archive/v1/DESIGN.md).
- <a id="g-validation-plan"></a>**Validation plan**: `validation_plan.md` (2026-10-05): a joint plan from two models' cross-examination, with a 13-item go/no-go gate to pass before boards are ordered. More: [validation_plan.md](hardware/ref_design/docs/validation_plan.md).
- <a id="g-vbat"></a>**VBAT**: REF-A's cell rail: the cell's own voltage, 4.2 V when full, behind J2 and JP5.
- <a id="g-vbus"></a>**VBUS**: USB's 5 V supply line, and REF-A's rail from J1. It is there only while a cable is in and the source has turned it on. More: [Wikipedia](https://en.wikipedia.org/wiki/USB_hardware).
- <a id="g-vendored"></a>**Vendored**: Copied into this repository from an outside source rather than fetched when needed, such as the Gen 3 random battle sets and the membrane footprints.
- <a id="g-via"></a>**Via**: A plated hole that joins copper on different layers. Stitching vias tie REF-A's two ground pours together every 5 mm. More: [Wikipedia](https://en.wikipedia.org/wiki/Via_(electronics)).
- <a id="g-vid-pid"></a>**VID and PID**: The vendor and product IDs a USB device reports. The firmware uses placeholders; Raspberry Pi gives out product IDs under its vendor ID for products on its chips. More: [Raspberry Pi usb-pid](https://github.com/raspberrypi/usb-pid).
- <a id="g-divider"></a>**Voltage divider**: Two resistors in series across a voltage, with the middle giving a fixed fraction. R17 and R18 (470k each) halve the cell voltage for ADC7. More: [Wikipedia](https://en.wikipedia.org/wiki/Voltage_divider).
- <a id="g-vsys"></a>**VSYS**: REF-A's system rail, always live with a cell in: about 4.6 V from USB through D5, otherwise the cell through Q2. The regulator, the backlight and the amplifier run from it.
- <a id="g-wasm"></a>**WASM**: WebAssembly: a compact code format that browsers run at near-native speed. The web build is core compiled to it. More: [webassembly.org](https://webassembly.org/).
- <a id="g-watchdog"></a>**Watchdog**: A hardware timer the firmware must keep restarting; if the firmware hangs, the watchdog resets the chip, which on REF-A turns the device off on battery. More: [Wikipedia](https://en.wikipedia.org/wiki/Watchdog_timer).
- <a id="g-weather"></a>**Weather**: Gen 3's field conditions (rain, sun, sandstorm, hail), which change damage and act at the end of each turn. More: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Weather).
- <a id="g-workspace"></a>**Workspace (Cargo)**: A set of crates built together. The web build is its own workspace so its WASM settings stay apart. More: [Cargo book](https://doc.rust-lang.org/cargo/reference/workspaces.html).
- <a id="g-ws2812b"></a>**WS2812B**: An RGB LED with its own controller, chained on one data line into addressable strips; v1 showed HP and party on them. More: [Adafruit](https://learn.adafruit.com/adafruit-neopixel-uberguide).
- <a id="g-zero-drift"></a>**Zero-drift rule**: Firmware and web must look and behave identically: all decisions live in core, and platforms do raw IO only. When they disagree, the device is the source of truth. More: [Section 5.3](#53-the-zero-drift-rule).
- <a id="g-zif"></a>**ZIF connector**: Zero insertion force: a connector with a hinged latch that clamps an FPC laid into it. REF-A's J3 is a bottom-contact ZIF that JLC places, so a panel plugs in and swaps without soldering. More: [Wikipedia](https://en.wikipedia.org/wiki/Zero_insertion_force).
