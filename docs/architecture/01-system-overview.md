# System Overview

Mega Blastoise is organized as a shared battle core plus platform adapters.

## Workspace Components

- [`mega_blastoise_core`](../../crates/mega_blastoise_core/) (`no_std`)
  - Battle runner, typed events, input trait, demo teams, and data store.
- [`mega_blastoise_fw`](../../crates/mega_blastoise_fw/) (`no_std`, RP2350 / Pico 2)
  - Embassy runtime, ST7796 panel, 16 direct buttons, USB CDC console, battery sense, defmt logging.
- [`mega_blastoise_web`](../../crates/mega_blastoise_web/) (wasm)
  - The same device in a browser page (`www/device.html`).
- [`mega_blastoise_test`](../../crates/mega_blastoise_test/) (host/std)
  - Stdin-based interactive harness and tests reusing the same core pipeline.

## Architectural Goal

One battle/event model should drive all runtimes. Platform-specific code should only handle I/O and effects.

## End-to-End Battle Lifecycle

1. Draw two teams (`randbat::draw_two_ps_teams`) and a `runner::ps::BattleSetup`.
2. Enter `runner::ps::run_battle` over a `Transport` (`LocalTransport` on the device).
3. Send `>start` and both `>player` commands to `ps_sim`.
4. Read the PS protocol back through `ps_client` into typed `BoardEvent`s.
5. Distil each `|request|` into `SlotOptions` and put the prompts on the `InputBus`.
6. Read each player's choice back from the bus (the `ChoiceCollector` decides it).
7. Send it as `>p1` / `>p2` and continue until the server says the battle ended.

(Both generations run the same loop, on the PS engine: the legacy Gen 1
and Gen 3 engines are gone.)

Continue with [Core Battle Engine Flow](./02-core-battle-flow.md).

## Shape

One `no_std` core holds the whole game: menus, lobby, input rules, screen decisions and rendering.
It drives a Showdown-compatible battle server and reads the battle back through a Showdown
protocol client, the same split as Pokemon Showdown itself, so an online battle is the same
protocol. Three thin platforms wrap the core.

```
   ps_dex (data)    ps_protocol (messages)
         \               /
   ps_sim (battle server)   ps_client (battle view from protocol)
                  \          /
               mega_blastoise_core         (game loop, input rules, screens, renderer)
              /         |          \
 mega_blastoise_fw  mega_blastoise_web  mega_blastoise_test
  (RP2350 device)    (browser, WASM)     (host CLI and tests)
```

| Crate | Role |
|---|---|
| `ps_sim` | Pokemon Showdown-compatible battle server: state, queue, events, PRNG, damage, protocol output. `no_std` + alloc |
| `ps_client` | Showdown protocol client: builds its battle view only from protocol messages and feeds `choice_collect` |
| `ps_dex` | Showdown's dex tables (species, moves, items, abilities, types, conditions), generated for each gen |
| `ps_protocol` | Showdown sim protocol types (messages, commands, requests, teams) |
| `ps_harness` | Test only: the `ps_sim_cli` binary that the protocol diff against real Showdown drives (`tools/ps-diff/`) |
| `mega_blastoise_core` | Everything the device does, shared verbatim by the platforms |
| `mega_blastoise_fw` | Firmware for the stripboard v2 board (Embassy on RP2350) |
| `mega_blastoise_web` | The same device in a browser page; its own WASM workspace |
| `mega_blastoise_test` | Interactive stdin battle, unit and integration tests, the Showdown parity and fuzz suites, a renderer that dumps every screen to image files |

More detail: [docs/architecture/](README.md) and the
"where does new code go" guide in [08-feature-map.md](08-feature-map.md).
Toolchain: nightly Rust pinned in [rust-toolchain.toml](../../rust-toolchain.toml).

## The battle engines

**History.** The project began on upstream [`battler`](https://github.com/jackson-nestelroad/battler),
an open-source Rust Pokemon engine, checked out as a git submodule (Alex's fork) at `battler/`. On
the RP2040 it ran out of heap, so `gen1_battle` was written as a drop-in replacement on
2026-05-15. `battler` is still a dependency: core's build script reads its Gen 1 data files and
the data store uses its types, but no battle runs through it.

**Gen 1** (`gen1_battle`, about 4,400 lines) models Gen 1 random battles as Showdown's Gen 1 mod
does, including the cartridge's glitches. Its SPEC.md documents
memory layout, formulas, turn structure and the deliberate gaps. By 2026-08-16, 164 of its 165
moves were in the fuzz pool.

**Gen 3** (`gen3_battle`, about 11,400 lines) was added on 2026-08-15 as a sibling crate, so a Gen 1
battle never enters it. By 2026-08-16 it modelled 75 of the 76 Gen 3 abilities and the first 47
held items (more followed on 08-17), with weather, screens, multi-turn moves and the residual
order as Showdown runs them. Abilities and items are answered by small modules
(ability.rs, item.rs) that
each pipeline stage asks, rather than an event bus.

**The reference.** Pokemon Showdown is the authority. Its source is installed locally by
`tools/showdown-parity` and read before a mechanic is implemented; the fuzzer then confirms. The
last engine commit (2026-08-19) widened the fuzz pool to every damaging move, found a backlog of
480 failing seeds in 600, and brought it to 5, which are not yet explained.

**On the device**, both engines speak one protocol to core: a request becomes generation-neutral
slot options for the input layer, and the chosen move or switch goes back as a command string.
Teams come from Showdown's random battle sets ([randbat data](https://pkmn.github.io/randbats/)
for Gen 1; the vendored Gen 3 sets for Gen 3). The device's Gen 3 draft does not yet hand out
abilities, held items or EV spreads (draft.rs), so device
battles do not yet exercise what the engine models.

## Data and assets

Core's [build script](../../crates/mega_blastoise_core/build.rs) generates static tables at compile
time, so nothing is loaded at run time and there is no file system: Gen 1 data from `battler`'s
data files, the random battle roster, move descriptions from Showdown's text data, 1-bit sprites
for the old mono renderer, and colour sprites for every Gen 3 species (Emerald art from the
PokeAPI sprite repository, packed losslessly as 4 bits per pixel with a per-species palette).
Downloads are cached under `crates/mega_blastoise_core/vendor/` so later builds are offline.
Gen 3 art is used for both generations. Move icons for the attack effects come from
[scripts/gen_move_sprites.py](../../scripts/gen_move_sprites.py).
