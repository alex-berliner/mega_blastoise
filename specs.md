# Mega Blastoise — Project Specification

## Purpose

Mega Blastoise is a self-contained two-player physical board game that runs Pokémon Generation 1 combat. Two players face off across a physical board equipped with NeoPixel LED strips, an e-ink display, and button clusters. No phone, PC, or network is required — the game runs entirely on a Raspberry Pi Pico (RP2040).

The battle rules are fully handled by the `battler` open-source battle engine. The firmware and host test crate wrap that engine with a hardware presentation layer (LEDs, display, input) and a structured event pipeline.

---

## Repository Layout

```
mega_blastoise/
  battler/                  # upstream battler engine (git checkout)
  mega_blastoise/           # Rust workspace
    mega_blastoise_core/    # no_std shared library: engine integration, event bus
    mega_blastoise_fw/      # RP2040 firmware binary
    mega_blastoise_test/    # PC host binary + integration tests
    patches/                # patched once_cell for no_std compatibility
```

---

## Hardware

| Component | Part | Interface | Notes |
|-----------|------|-----------|-------|
| MCU | Raspberry Pi Pico (RP2040) | — | 264 KB SRAM, 2 MB flash |
| LEDs | WS2812B NeoPixel strip (~30 LEDs) | PIO (GPIO) | Driven by RP2040 PIO state machine |
| Display | Waveshare 2.9" e-ink | SPI | Not yet integrated |
| Move buttons | 4× tactile per player | GPIO (GP6–9, P2 TBD) | Pull-up, active-low, debounced in firmware |
| Switch buttons | 6× tactile per player | GPIO (GP10–15, P2 TBD) | Party slot 1–6 |
| NFC readers | 2× PN532 | I²C | I2C0: GP16/17, I2C1: GP18/19, addr 0x24 |
| Debug probe | Picoprobe / CMSIS-DAP | SWD | probe-rs flashing and RTT logging |
| USB serial | Built-in RP2040 USB | USB CDC-ACM | Development CLI; picocom at /dev/ttyACM1 |

### Power Notes

PN532 modules draw significant current when the RF field is active. Power them from an external 3.3 V or 5 V supply, **not** from the Pico's 3V3 pin. Share a common GND with the Pico. I²C signal levels are 3.3 V on both the RP2040 and most PN532 breakouts — no level shifting needed.

---

## Software Architecture

### Crate Dependency Graph

```
battler  (upstream engine, no_std core)
    ↑
mega_blastoise_core   (no_std, shared by fw and test)
    ↑                ↑
mega_blastoise_fw   mega_blastoise_test
(RP2040 binary)     (PC binary + tests)
```

### `mega_blastoise_core`

Platform-independent library compiled for both `thumbv6m-none-eabi` and the host. Contains:

- **`battle_runner`** — `run_battle()` entry point; drives the `battler` engine to completion
- **`battle_input`** — `InputBus`, `InputSource` trait, choice string helpers
- **`battle_effects`** — `BoardEffects` trait, `BoardEventQueue`, log parsing
- **`board_event`** — typed `BoardEvent` enum, `ParsedBattleLogLine`, `parse_log_line()`
- **`data_store`** — `FlashDataStore` implementing `battler_data::DataStore` from baked-in const tables
- **`demo_teams`** — hardcoded demo rosters for p1 (Red) and p2 (Blue)

### `mega_blastoise_fw`

`no_std` Embassy async binary for RP2040. Modules:

- **`main`** — embassy entry point; instantiates all subsystems and calls `run_battle()`
- **`subsystems/usb`** — USB CDC-ACM init; spawns `usb_device_task`; returns `UsbBattleInput`
- **`subsystems/nfc`** — I²C + PN532 init; spawns `pn532_task_i2c0/1`
- **`usb_input`** — `UsbBattleInput`: USB CDC battle CLI with local echo, backspace, move/switch prompts
- **`pico_battle_input`** — `PicoBattleInput`: physical GPIO button matrix (stub, not yet active)
- **`board_effects`** — `BattleEffects`: forwards `BoardEvent` to RTT log and optionally to `InputBus.log`
- **`pn532`** — PN532 I²C driver (`GetFirmwareVersion` ping loop; NFC card read not yet implemented)
- **`usb_cdc_line`** — CDC line helpers: `write_crlf`, `log_usb_rx_line_str_to_rtt`
- **`mem_profile`** — heap snapshot utility (gated on `mem-profile` feature)

Cargo features:

| Feature | Default | Effect |
|---------|---------|--------|
| `usb` | ✓ | Compile USB subsystem and `UsbBattleInput` |
| `nfc` | ✓ | Compile NFC subsystem and PN532 tasks |
| `mem-profile` | ✓ | Emit `heap_snapshot()` calls at boot and after major allocs |

### `mega_blastoise_test`

Standard Rust binary for PC development and CI. Modules:

- **`harness`** — `run_interactive()`: sets up a battle and runs it over stdin
- **`stdin_input`** — `StdinBattleInput`: synchronous stdin move/switch prompts
- **`board_game_effects`** — `BoardGameEffects`: `println!`-based `BoardEffects` sink for host
- **`main`** — calls `run_interactive()`
- **`tests/`** — integration tests: board event parsing, scripted effect pipeline, demo battle smoke test

---

## Core Interfaces

### `InputBus`

Defined in `mega_blastoise_core::battle_input`. The single shared-memory bus between the battle runner and all input sources.

```
InputBus {
    choices: Channel<NoopRawMutex, String, 4>,   // input → runner
    prompt:  Signal<NoopRawMutex, ActivePrompt>,  // runner → input sources
    log:     Channel<NoopRawMutex, String, 8>,    // effects → USB output sink
}
```

- `prompt` is a `Signal` (last-write-wins): the runner writes the current `ActivePrompt` before blocking on `choices`.
- `choices` is an async MPSC channel: input sources send a choice string (e.g. `"move 0"`, `"switch 2"`); the runner receives one per request.
- `log` carries human-readable battle event descriptions from `BattleEffects` to `UsbBattleInput` for display over the CDC serial line.

### `ActivePrompt`

```rust
pub struct ActivePrompt {
    pub player_id: String,   // "p1" or "p2"
    pub request: Request,    // battler::Request (Turn | Switch | TeamPreview | LearnMove)
}
```

Broadcast by the runner before blocking. Input sources read this to know what to display and what format of choice to submit.

### `InputSource` trait

```rust
pub trait InputSource {
    async fn run(&mut self, bus: &InputBus);
}
```

Implemented by `UsbBattleInput`, `PicoBattleInput`, `StdinBattleInput`, and the no-op `NoInput`. `run_battle()` joins the battle loop with `input.run(bus)` so both progress cooperatively via embassy's cooperative scheduler.

### `BoardEffects` trait

```rust
pub trait BoardEffects {
    fn on_event(&mut self, event: BoardEvent);
}
```

Implemented by `BattleEffects` (firmware) and `BoardGameEffects` (host test). Called synchronously by `BoardEventQueue::dispatch_all()` inside `battle_loop` for each typed event.

### `BoardEvent` enum

Typed events produced by parsing engine log lines and injected prompts. Variants:

| Variant | Trigger | Intended hardware action |
|---------|---------|--------------------------|
| `BattleStart` | Engine `battlestart` log | LED startup animation, full HP display |
| `Turn { n }` | Engine `turn` log | Turn marker blink |
| `Move { name }` | Engine `move`/`animatemove` log | Move flash on attacker's LED strip |
| `Damage { mon, health }` | Engine `damage` log | Hit sound, HP bar update |
| `Heal { mon, health }` | Engine `heal` log | Heal sound, HP bar update |
| `Faint { mon }` | Engine `faint` log | KO animation, extinguish that Pokémon's LEDs |
| `SwitchIn { name, species, player_id }` | Engine `switch`/`drag`/`appear` | Switch animation, light new slot |
| `SwitchOut` | Engine `switchout` | Dim outgoing Pokémon's lights |
| `Split { side }` | Engine `split` | Internal log routing marker (not a gameplay cue) |
| `Win { side }` | Engine `win` | Win animation on winning side |
| `Tie` | Engine `tie` | Neutral end animation |
| `Prompt { player_id, kind }` | Before blocking on input | Light that player's input controls |

### `BoardEventQueue`

FIFO queue for `BoardEvent` values. Handles the battler `split` → private row → public row pattern by skipping the private row after each `split`. Filled by `push_log_lines()` and drained to a `BoardEffects` sink by `dispatch_all()`.

### Choice String Protocol

All choice strings are plain ASCII passed through `InputBus.choices`:

| Situation | String format | Example |
|-----------|---------------|---------|
| Use move in slot N (0-based) | `move N` | `move 0` |
| Switch to party index N (0-based) | `switch N` | `switch 2` |
| Multiple slots (doubles / multi-switch) | Joined with `;` | `move 0;move 2` |
| Pass (no moves available) | `pass` | `pass` |
| Team preview (random) | `random` | `random` |

Helpers in `mega_blastoise_core::battle_input`: `format_move_choice(slot)`, `format_switch_choice(team_index)`, `join_choice_parts(&[String])`.

### `FlashDataStore`

Implements `battler_data::DataStore` using const tables baked into flash at build time by `build.rs`. The build script reads JSON data files from the `battler` data directory and emits Rust `static` arrays of `(&str, &str)` pairs (id → JSON). Lookup is linear scan; the `battler` Dex layer caches results after first access.

---

## Data Flow

### Battle Turn (firmware)

```
UsbBattleInput::run()
  ├─ select(bus.prompt.wait(), bus.log.receive())
  │   ├─ log line arrives → writeln over USB CDC
  │   └─ prompt arrives → handle()
  │         ├─ display battle state (allies, bench, moves) over USB CDC
  │         ├─ read_line() [local echo, backspace, CRLF]
  │         └─ validate input → format_move_choice() / format_switch_choice()
  └─ bus.choices.send(choice_string)

battle_loop()  [runs concurrently via embassy join]
  ├─ battle.active_requests().next()
  ├─ queue.push_event(board_prompt_event()) → dispatch_all(effects)
  ├─ bus.prompt.signal(ActivePrompt { player_id, request })
  ├─ bus.choices.receive().await   ← blocks here until input sends
  └─ battle.set_player_choice(player_id, choice_string)
       └─ process_new_log_lines() → BoardEventQueue → BattleEffects
             ├─ defmt::info! (RTT)
             └─ bus.log.try_send(description)  ← picked up by UsbBattleInput
```

### Engine Log Processing

```
battle.new_log_entries()  →  process_new_log_lines()
  └─ BoardEventQueue::push_log_lines()
       ├─ ParsedBattleLogLine::parse(line)  [title|key:value|…]
       ├─ split handling: skip private row after split
       └─ parse_log_line() → Some(BoardEvent) or None
  └─ dispatch_all(sink)
       └─ BoardEffects::on_event(event)
```

---

## Concurrency Model

The firmware runs under `embassy-executor` with a single-threaded cooperative async model.

- **`battle_loop`** and **`input.run(bus)`** execute as joined futures — neither is a separate embassy task; they interleave on the single executor thread via `await` yield points.
- **`usb_device_task`** is a true embassy task, spawned separately, and handles USB protocol frames independently.
- **`pn532_task_i2c0/1`** are embassy tasks, one per I²C bus, polling NFC readers in the background.
- All shared state crosses future boundaries only through embassy-sync primitives (`Channel`, `Signal`) with `NoopRawMutex` (safe for single-core).

---

## Build System

### Toolchain

- Rust nightly (`rust-toolchain.toml` in workspace)
- Target `thumbv6m-none-eabi` for firmware
- `probe-rs` for flashing and RTT attach

### Build Commands

```bash
# PC test binary
cargo build -p mega-blastoise-test
cargo run -p mega-blastoise-test
cargo test -p mega-blastoise-test

# Firmware
cargo build -p mega-blastoise-fw --target thumbv6m-none-eabi --release
cd mega_blastoise_fw && cargo run --release   # flash via probe-rs

# Firmware without USB or NFC
cargo build -p mega-blastoise-fw --target thumbv6m-none-eabi \
  --no-default-features
```

### `build.rs` (core)

At compile time, `mega_blastoise_core/build.rs` reads the battler data directory and emits `battle_data.rs` into `$OUT_DIR`, included by `data_store.rs`. This bakes Pokémon species, moves, abilities, conditions, type chart, and aliases into flash as static string pairs.

---

## Game Scope (v1)

### In Scope

- ~20 Gen 1 Pokémon with type-coverage representation
- Standard singles 1v1: select move → resolve turn → repeat
- Status effects: burn, freeze, sleep, paralysis, poison
- Type effectiveness, STAB, critical hits, RNG damage roll
- Two-player local play (both players share one board)
- NeoPixel feedback: HP bars, attack flash, faint animation
- E-ink display: turn state, HP, active Pokémon, move list

### Explicitly Out of Scope (v1)

- AI opponent
- Full 151-Pokémon roster
- Sound hardware
- Wireless multiplayer
- Items (Gen 1 has none in battle)
- Pokémon abilities (Gen 1 has none)

---

## PlayerStation Architecture

Two symmetric `PlayerStation` structs, one per player. Each owns all player-side hardware:

```rust
pub struct PlayerStation<'d> {
    player_id: PlayerId,        // P1 | P2
    nfc: NfcReader<'d>,
    screen: EinkDisplay<'d>,
    buttons: PicoBattleInput<'d>,
    leds: LedStrip<'d>,
}
```

`BattleController` owns both stations and routes prompts/choices between them and the engine.

### Presentation Pipeline

Battle engine events are dispatched asynchronously; the main loop paces them via a per-event `delay` field:

1. Main loop receives event from engine
2. Sends event into each `PlayerStation`'s channel (non-blocking)
3. Sleeps `event.delay` before pulling the next event
4. Each station has one long-running async task draining its channel

```rust
pub struct BoardEvent {
    pub kind: BoardEventKind,
    pub delay: Duration,   // floor: main loop waits this long before next event
}
```

Properties:
- **P1 and P2 animate in parallel** — independent tasks, no cross-player coordination
- **Per-player hardware is serialized** — single consumer task per station prevents display/LED conflicts
- **Overlap is a per-event knob** — `delay=0` max overlap, `delay≥animation_duration` sequential
- **Channel backpressure** (capacity ~4–8) — if presentation falls behind, non-blocking sends drop low-priority events or the main loop stalls naturally
- Events affecting both players (turn start, weather) are sent to both channels and animate in parallel

---

## Open Work

| Area | Status |
|------|--------|
| USB battle CLI (picocom) | Working |
| Physical button input (`PicoBattleInput`) | Stub — awaiting wiring |
| NFC card read (PN532) | Bring-up only (firmware version ping loop) |
| NeoPixel HP bars | Not started |
| E-ink display | Not started |
| Pokémon switching mid-battle | Deferred to v2 |
| Per-Pokémon move limits / PP enforcement | Engine handles PP; UI validation in USB input |
