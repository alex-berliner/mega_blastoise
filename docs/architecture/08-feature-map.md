# Feature Map: Core vs. Target-Specific

This document lists every significant feature in the workspace and which crate or
runtime owns it.  Use it when deciding where new code belongs.

---

## mega_blastoise_core  (`no_std`, shared by all targets)

### battle_runner

| Feature | Symbol |
|---------|--------|
| Battle loop | `run_battle` |
| Battle options setup | `battle_options_with_seed`, `demo_battle_options`, `demo_engine_opts` |
| Player setup | `make_player` |
| Log → `BoardEvent` parsing + dispatch | `enrich_and_dispatch` (private) |
| PP refresh from battle RAM (no full CBOR decode) | `MoveSlotCache` (private) |
| Move slot full query on SwitchIn | `query_active_moves` (private) |
| Team slot lookup by name | `query_team_slot_by_name` (private) |
| `team_slot` enrichment on Faint/SwitchIn | inside `enrich_and_dispatch` |
| Idle-to-demo delay shared constant | `LOBBY_DEMO_DELAY_MS` |

### battle_effects

| Feature | Symbol |
|---------|--------|
| Hardware output trait | `BoardEffects` (async `on_event`) |
| No-op sink | `NoopBoardEffects` |
| Typed event queue with split-dedup | `BoardEventQueue` |
| Convenience log-line processor | `process_new_log_lines` |
| Shared animation delay constants | `anim::{MOVE_MS, DAMAGE_MS, SWITCH_IN_MS, FAINT_MS, WIN_MS, EFFECT_MS, BRIEF_MS}` |

### board_event

| Feature | Symbol |
|---------|--------|
| All typed events | `BoardEvent` (Damage, Heal, Faint, SwitchIn, Move, Win, Tie, Prompt, …) |
| Log line parser | `parse_log_line` |
| Raw log line key/value accessor | `ParsedBattleLogLine` |
| Prompt event constructor | `board_prompt_event` |
| Move slot snapshot | `MoveSlot` |
| Prompt kind enum | `PromptKind` |
| Player/side display names | `player_display_name`, `side_display_name` |
| Mon display name (strips player prefix) | `mon_display_name` |
| Mon player-id extractor | `mon_player_id` |

### hp_bar

| Feature | Symbol |
|---------|--------|
| HP string parser (e.g. `"42/75"`) | `HpBarState` (`parse`, `pct`) |

### battle_input

| Feature | Symbol |
|---------|--------|
| Shared multi-source input bus | `InputBus` (choices / prompt / log channels) |
| Per-prompt payload | `ActivePrompt` (player_id, request, player_data, batch_total) |
| Input source trait | `InputSource` |
| Null input source | `NoInput` |
| Button-event source trait | `ButtonSource` (on_prompt, on_choice_pending, wait_action, wait_switch, …) |
| Button orchestrator with undo window | `ButtonController` |
| Two-player parallel choice collection | `ButtonController::run_parallel` |
| Choice string helpers | `format_move_choice`, `format_switch_choice`, `join_choice_parts`, … |
| Button result type | `PlayerAction` (Move / Switch) |

### cli_parse

| Feature | Symbol |
|---------|--------|
| USB turn-prompt parser | `parse_turn_line` → `TurnChoice` |
| USB forced-switch parser | `parse_switch_line` → `usize` |
| USB lobby command parser | `parse_lobby_cmd` → `LobbyCmd` |
| Web in-game command parser | `parse_web_game_cmd` → `WebGameInput` |

### display

| Feature | Symbol |
|---------|--------|
| Party slot snapshot | `PartySlotData` |
| `MonBattleData` → `PartySlotData` | `party_slot_from_mon` |
| Battle screen renderer | `render_player_screen` |
| Stats screen page 1 | `render_pokemon_stats` |
| Stats screen page 2 | `render_pokemon_stats_page2` |
| Move detail screen | `render_move_detail` |
| Forced-switch party screen | `render_switch_screen` |
| Win/tie message screen | `render_win_screen` |
| Lobby ready-state screen | `render_lobby_screen` |
| Battle event narration flash (word-wrapped) | `render_event_text` |
| Invalid-selection flash ("Already fainted!") | `render_invalid_selection` |
| Choice-submitted waiting overlay | `render_waiting_screen` |
| Waiting-for-opponent overlay | `render_waiting_for_opponent` |

### prompt_fmt

| Feature | Symbol |
|---------|--------|
| Full battle state summary (per-turn `on_turn` callback) | `format_active_state` |
| Single-player state text | `format_player_state` |
| Move/switch prompt text for CLI display | `format_prompt` |

### data_store

| Feature | Symbol |
|---------|--------|
| Static battle data (Pokémon, moves) implementing `battler::DataStore` | `FlashDataStore` |

### randbat / demo_teams / rng / random_ai

| Feature | Symbol |
|---------|--------|
| Random battle team draw | `draw_randbat_team` |
| Fixed test teams | `demo_team_red`, `demo_team_blue` |
| Simple LCG | `SimpleRng` |
| Random choice AI | `RandomAi` |

---

## mega_blastoise_fw  (RP2350, Embassy, `no_std`)

The stripboard v2 board: Pico 2, one ST7796 320x480 panel (the 240x320 UI
centred on it), 16 direct buttons.
Default features `rp2350` + `board_v2`; default target `thumbv8m.main-none-eabihf`.

| File | Feature |
|------|---------|
| `main.rs` | Peripheral init, heap + RTT, task spawns, hands over to `board_v2::run` |
| `board_v2.rs` | `DevicePort` impl (clock, timers, console, frame presentation, entropy, battery); runs core's `DeviceRunner` |
| `subsystems/st7796.rs` | Panel driver: PIO SPI + DMA, one frame in flight |
| `subsystems/buttons_v2.rs` | 16-key debounce, A tap vs. hold, `DeviceEvent::Button` pushes; held-key masks for `input-debug` |
| `subsystems/usb_v2.rs` | USB CDC console: typed lines to the runner, `:reset`, `:bl`, `:panic`, `:anim` |
| `subsystems/battery.rs` | Cell voltage sampling for the status bar |
| `lib.rs` | IMAGE_DEF block, panic/HardFault handlers (GP25 LED, crash message kept across reset), `mem_profile`, `usb_cdc_line` |
| `bin/gpio_probe.rs`, `bin/usb_loopback.rs` | Bring-up tools: pin discovery, CDC loopback |

### Cargo feature flags

| Flag | What it enables |
|------|-----------------|
| `rp2350` (default) | The RP2350 chip in embassy-rp |
| `board_v2` (default) | The stripboard v2 drivers (panel, buttons, console, battery) |
| `input-debug` | Held-button overlay on the panel (`cargo bv2` / `cargo rv2`) |
| `mem-profile` | Heap snapshot logging |
| `trace` | Core's trace logging |

---

## mega_blastoise_web  (WASM, std)

The same `DeviceRunner` in a browser page (`www/device.html`, `device.js`).
`src/lib.rs` is raw IO only: a `DevicePort` impl (JS clock, timeouts, console
log, entropy, the 240x320 pixel cache the page polls) and exports that turn
pointer/keyboard input into `DeviceEvent`s (`nav_dpad`, `nav_a`, `nav_a_hold`,
`nav_b`, `nav_info`, `nav_menu`, `panel_tap`, `submit_text`, plus `press_move`,
`press_switch`, `nav_cancel` and `screen_state` for the headless probe).

---

## mega_blastoise_test  (host/std, test harness)

### src/

| File | Feature |
|------|---------|
| `host_battle_effects.rs` | `HostBattleEffects`: `BoardEffects` impl; tracks HP state; delegates to host stubs |
| `host_battle_controller.rs` | `HostBattleController`: `InputSource` impl with pre-fed button queue; auto-picks on switch |
| `host_buzzer.rs` | Stub buzzer: records faint/win/hit/crit calls |
| `host_led.rs` | Stub LED: records HP%, faint, party state |
| `host_oled.rs` | Stub OLED: records active mon name, move list |
| `host_hp_bar.rs` | `HostHpBarState`: parses and stores HP |
| `host_hw_object.rs` | `HostHwObject`: retained for existing tests; wraps hp bar state |
| `stdin_input.rs` | Interactive stdin `InputSource` (used by the `bin/main.rs` interactive harness) |
| `harness.rs` | Shared test helpers |
| `turn_timing.rs` | Turn processing time benchmark |

### tests/

| File | What it covers |
|------|----------------|
| `cli_parsing.rs` | All four parsing surfaces: `parse_turn_line`, `parse_switch_line`, `parse_lobby_cmd`, `parse_web_game_cmd` — 40 tests |
| `host_device_stubs.rs` | `HostBattleEffects` HP tracking; full battle completion (with/without bus.log); button-press choice path |
| `board_events_and_queue.rs` | `parse_log_line`, `BoardEventQueue` split-dedup, `ParsedBattleLogLine` |
| `scripted_effects.rs` | `BoardEventQueue::dispatch_all` order |
| `demo_battle_smoke.rs` | Battle initializes with correct team sizes |

---

## Decision guide: where does new code go?

| It is… | Put it in… |
|--------|-----------|
| Pure battle logic, event parsing, or choice formatting | `mega_blastoise_core` |
| A display renderer using `embedded-graphics` | `crates/mega_blastoise_core/src/display_color.rs` (the single screen) |
| CLI input parsing (USB syntax or web syntax) | `crates/mega_blastoise_core/src/cli_parse.rs` |
| A hardware peripheral task (I2C, PWM, PIO) | `crates/mega_blastoise_fw/src/subsystems/` |
| Firmware-side `DevicePort` plumbing | `crates/mega_blastoise_fw/src/board_v2.rs` |
| GPIO input driver | `crates/mega_blastoise_fw/src/subsystems/buttons_v2.rs` |
| A WASM export or JS-facing global | `crates/mega_blastoise_web/src/lib.rs` |
| A stub for testing hardware behaviour | `crates/mega_blastoise_test/src/` |
| A regression test | `crates/mega_blastoise_test/tests/` |
