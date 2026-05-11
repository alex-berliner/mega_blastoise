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
| HP fraction → lit LED count (8-bar) | `hp_bar_count` |
| HP fraction → RGB color tuple | `hp_bar_color` |
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
| Lobby ready-status line (USB/CLI) | `format_lobby_status` |

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

## mega_blastoise_fw  (RP2040, Embassy, `no_std`)

### main.rs — startup and game loop

- Hardware peripheral init (Embassy HAL)
- Heap initialization and RTT/defmt setup
- Game loop: lobby → battle → lobby
- Feature-gated subsystem spawn (see table below)

### lobby.rs — lobby state machine

| Feature | Detail |
|---------|--------|
| `LobbyInput` trait | Abstracts USB+button vs. button-only |
| `LobbyEvent` enum | Typed lobby inputs (P1/P2/Ready/Unready/AI/Demo/Stop) |
| `run_lobby_inner` | Demo-race loop → ready-up phase → countdown |
| Demo AI battle | `run_demo_battle` races AI vs. AI until interrupted |
| USB + button implementation | `UsbButtonLobbyInput` |
| Button-only implementation | `ButtonOnlyLobbyInput` (no `usb` feature) |

### battle_effects.rs — `BoardEffects` implementation

Implements the `BoardEffects` trait for the firmware.  Routes events to hardware
subsystems and writes narration strings to `bus.log` for USB display.

| Event | Action |
|-------|--------|
| Damage / Heal | HP% → OLED + LED HP bar; buzzer Hit |
| SwitchIn | Mon name → OLED; move list → OLED; LED party slot on |
| Faint | HP=0 → OLED; LED party slot off + status clear; buzzer Faint |
| Win / Tie | OLED win screen; LED win animation; buzzer Win |
| SuperEffective | Buzzer SuperEffective |
| CriticalHit | Buzzer Crit |
| SetStatus / CureStatus | LED status dot |
| Prompt | OLED restore (clears any long-press detail overlay) |
| MovesUpdate | OLED move list update |

Animation delay applied after every event per `anim::*` constants (skippable via `:anim off`).

### pico_battle_input.rs — GPIO button matrix

| Feature | Detail |
|---------|--------|
| `ButtonMatrix` | 4×4 scan, 12 µs settle, active-LOW with pull-ups |
| `PicoBattleInput` | `ButtonSource` impl; routes rows 0–1 to P1, rows 2–3 to P2 |
| `wait_move` | Waits for a usable move button (respects disabled/PP=0 filter) |
| `wait_switch` | Waits for a party button |
| `wait_lobby_press` | Short vs. long press detection for lobby AI trigger |

### usb_input.rs — USB CDC battle CLI

| Feature | Detail |
|---------|--------|
| `UsbBattleInput` | `InputSource` impl over USB CDC ACM |
| Line editor | Backspace, CRLF normalization, Enter-to-repeat last line |
| Turn prompt | Shows move list, accepts move slot or `switch N`; validates disabled/PP |
| Switch prompt | Accepts party slot 1–6 |
| AI auto-choice | `RandomAi` for AI-controlled players |
| Meta commands | `:reset`, `:anim on/off` valid at any prompt |
| Lobby CLI | `read_lobby_cmd` (delegates to `parse_lobby_cmd` from core) |
| `last_typed_line` | Persists last non-empty input; cleared by `set_ai_players` |

### subsystems/buzzer.rs — piezo buzzer  (`buzzer` feature)

- Embassy async task; PWM slice 2 channel B (GP21)
- `BuzzerCmd` channel (capacity 4); `buzz()` enqueue helper
- Tones: Hit, SuperEffective, Crit, Faint, Win, CountdownBeep

### subsystems/led.rs — WS2812B NeoPixels  (`leds` feature)

- Embassy async task; PIO0 / DMA_CH0 / GP20; 24 LEDs
- `LedCmd` channel (capacity 8); `send()` helper
- Per-player state: HP bar (8 LEDs, green→yellow→red), party slots (3 LEDs, slot-indexed bool), status dot
- Lobby animations: LobbyIdle (breathing blue-purple), LobbyWaiting, LobbyCountdown
- Win animation: gold winner / dim loser

### subsystems/oled.rs — dual SSD1306 OLEDs  (`oled` feature)

- Embassy async task; I2C0 (GP16/17, P1) and I2C1 (GP18/19, P2)
- Async I2C (interrupt-driven, no blocking flush)
- `OledCmd` channel (capacity 8); `send()` helper
- Commands: HpUpdate, ActiveMon, MovesUpdate, Faint, Win, RestoreScreen, LobbyState

### lib.rs (fw library crate)

- Panic handler (drives GP25 error LED, logs via defmt, UDF)
- defmt panic handler
- `HpBarState::parse` — parses `"current/max"` health strings
- `usb_cdc_line` — USB line helpers (RTT mirror, CRLF write)
- `mem_profile` — heap snapshot logging (peak/current, `mem-profile` feature)

### Cargo feature flags

| Flag | What it enables |
|------|-----------------|
| `usb` (default) | USB CDC stack, `UsbBattleInput`, USB lobby input |
| `oled` (default) | Dual SSD1306 async task |
| `buzzer` (default) | PWM piezo task |
| `leds` | WS2812B NeoPixel task |
| `mem-profile` (default) | Heap snapshot logging |

---

## mega_blastoise_web  (WASM, std)

### lib.rs — WASM entry and global state

| Feature | Detail |
|---------|--------|
| Game loop | `run_game_loop`: lobby → battle → lobby |
| Button queues | Per-player `VecDeque<ButtonEvent>` + `Waker`; `press_move` / `press_switch` WASM exports |
| `submit_text` | Parses text-box input: global cmds → lobby cmds → `parse_web_game_cmd` from core |
| Lobby commands | `:ready`, `:ready p1/p2`, `:ready ai`, `:demo` |
| Lobby button handling | Any press readies that player; long-press triggers AI |
| Demo mode | Auto-restart loop; interrupted by any button |
| Global commands | `:reset` (page reload), `:anim on/off` |
| OLED pixel exports | `get_p1_pixels`, `get_p2_pixels` (RGBA) |
| LED state export | `get_led_state` (24 packed RGB u32s) |
| Flash export | `get_flash_state` (super-effective / crit indicators) |
| Battle transition export | `consume_battle_transitions` (bitmask, clears on read) |
| Long-press overlays | `wasm_show_move_detail`, `wasm_show_pokemon_stats`, `wasm_restore_screen` |
| Detail overlay guard | `P_IN_DETAIL` suppresses `update_pixels` while overlay is active |
| Party LED sync | `sync_party_leds` — status-colored from `P_PARTY` data |
| HP LED update | `update_hp_leds` — writes 8 HP-bar LEDs per player |
| Active mon name cache | `P_MON_NAME` — shown on waiting screen |
| AI pause | `wasm_toggle_ai_pause` — blocks AI `wait_action` without stopping the loop |

### web_controller.rs — `WebButtonSource`

- `ButtonSource` impl backed by `PlayerButtonFuture` (waker-based async)
- `on_prompt`: renders `format_prompt` to log, calls `restore_screen`, `update_party`, `show_switch_screen`
- `on_choice_pending`: shows waiting screen (skipped for AI players)
- `on_waiting_for_other_player`: shows opponent-waiting screen
- `wait_cancel_window`: 1-second window; any button press cancels
- `wait_action`: loops on `PlayerButtonFuture`; validates party slot alive before Switch
- `wait_switch`: loops on `PlayerButtonFuture`; validates party slot alive

### web_effects.rs — `WebBattleEffects`

`BoardEffects` impl for the web target.  Mirrors `battle_effects.rs` functionality using
web globals instead of hardware channels.

| Event | Action |
|-------|--------|
| Damage / Heal | Redraws OLED via `render_player_screen`; updates HP LEDs; flash for SE/crit |
| SwitchIn | Redraws OLED; updates `P_MON_NAME`; updates moves + party |
| Faint | Sets `slot.hp = 0` in `P_PARTY`; patches HP bar; syncs party LEDs |
| Win / Tie | Full LED fill (gold winner / dim loser); win screen on OLED |
| SetStatus / CureStatus | Patches `P_PARTY` status field; syncs party LEDs |
| MovesUpdate | Updates `P_MOVES`; redraws OLED |
| Prompt | Restores OLED screen |

### web_display.rs — `WasmDisplay`

- Implements `embedded_graphics::DrawTarget` over a 128×64 1-bit pixel buffer
- `to_rgba()` converts to a 4-byte-per-pixel RGBA `Vec<u8>` for canvas transfer

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
| A display renderer using `embedded-graphics` | `mega_blastoise_core/src/display.rs` |
| CLI input parsing (USB syntax or web syntax) | `mega_blastoise_core/src/cli_parse.rs` |
| A hardware peripheral task (I2C, PWM, PIO) | `mega_blastoise_fw/src/subsystems/` |
| Firmware-side `BoardEffects` routing | `mega_blastoise_fw/src/battle_effects.rs` |
| GPIO input driver | `mega_blastoise_fw/src/pico_battle_input.rs` |
| A WASM export or JS-facing global | `mega_blastoise_web/src/lib.rs` |
| Web-side `BoardEffects` rendering | `mega_blastoise_web/src/web_effects.rs` |
| A stub for testing hardware behaviour | `mega_blastoise_test/src/` |
| A regression test | `mega_blastoise_test/tests/` |
