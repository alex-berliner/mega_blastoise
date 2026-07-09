## ZERO-DRIFT RULE: shared display & input semantics (MANDATORY)

The pico firmware and the web client MUST be indistinguishable in everything
the player sees and does during a battle. This is enforced structurally:

- `mega_blastoise_core/src/oled_ctl.rs` — the ONLY place that decides what a
  screen shows (`OledController` + `oled_cmds_for_event`).
- `mega_blastoise_core/src/choice_collect.rs` — the ONLY place that decides
  how input behaves (`ChoiceCollector`: prompts, accept/reject + messages,
  waiting screen, tap/type-to-unready, the 1 s both-ready grace, invalid
  flashes, long-press detail views, typed-line grammar).

Platforms (mega_blastoise_fw, mega_blastoise_web) may contain ONLY raw IO:
press classification (tap vs 500 ms hold), typed-line transport, rendering
`OledCmd`s, printing collector `Effect` lines, and a monotonic-ms clock for
`tick()`. If a change makes you write battle UX behavior in fw- or web-only
code, STOP — put it in core and drive it from both platforms. When behavior
between targets conflicts, the pico is the source of truth.

---

## Project

A physical two-player board game implementation of Pokémon Gen 1 combat. Hardware-driven feedback (NeoPixels, e-ink). Runs entirely on a Raspberry Pi Pico (RP2040). No phone, no PC, no network — fully self-contained.

---

# project directories
battler/ checkout of https://github.com/jackson-nestelroad/battler
mega_blastoise/ rust base project

## Hardware

| Component | Part | Notes |
|-----------|------|-------|
| MCU | Raspberry Pi Pico (RP2040, 264 KB SRAM, 2 MB flash) | On hand |
| LEDs | WS2812B NeoPixel strip (~30 LEDs target, exact count TBD) | On hand |
| Display | Waveshare 2.9" e-ink (SPI) | To be ordered |
| Inputs | 2× button clusters (4 directional + confirm per player) | Off-the-shelf tactile buttons, debounced in firmware |
| Power | USB or LiPo battery (decision later) | |

The RP2040 is chosen specifically because its PIO state machines drive WS2812B perfectly without CPU or DMA acrobatics.

---

## Language and Toolchain

- **Rust**, `no_std`
- Target: `thumbv6m-none-eabi`
- Probe-based flashing/debugging: `probe-rs` (preferred) or `picotool` for USB-mass-storage flashing
- Logging: `defmt` + `defmt-rtt`
- Allocator: `embedded-alloc` or `linked-list-allocator` — battler crate likely needs heap allocation

### Recommended Crate Stack

| Concern | Crate | Why |
|---------|-------|-----|
| Async runtime + HAL | `embassy-rp` | Modern async runtime for RP2040, well-maintained |
| NeoPixels | `ws2812-pio` | Drives WS2812B via RP2040 PIO — no CPU overhead |
| E-ink display | `epd-waveshare` | Has Waveshare 2.9" support out of the box |
| SPI / GPIO | `embedded-hal` (via embassy-rp) | Standard traits |
| Battle logic | `battler` (see below) | Full Gen 1 battle engine, no_std-compatible core |
| Logging | `defmt`, `defmt-rtt` | Embedded-friendly structured logging |

---

## Battle Engine: `battler` crate

**Use these sub-crates (all `no_std`-compatible):**
- `battler` — core battle engine
- `battler-data` — data types
- `battler-state` — battle state representation
- `battler-prng` — deterministic RNG for battles
- `battler-choice` — turn choice parsing

**Do NOT use these (require `std`):**
- `battler-ai` — AI opponent. AI is explicitly out of scope for v1.
- `battler-service`, `battler-multiplayer-service` — networking, not relevant.
- `battler-client`, `battler-calc` — verify no_std support before pulling in. Skip if heavy.

**Integration approach:**
1. Use `battler` to drive turn resolution. Both players' move choices feed into the engine; the engine returns the resolved turn (damage, status changes, who fainted).
2. Hardware layer is purely presentation + input. It does not implement any game rules.
3. The Gen 1 dataset (Pokémon stats, moves, type chart) comes from `battler-data` if it provides Gen 1 fixtures, or is loaded from a `const` table baked into flash.

**Memory constraint:** 264 KB SRAM is the budget. The battler crate may have nontrivial allocations. Profile early — if heap usage is unbounded, may need to constrain the Pokémon dataset or use a fixed-size arena allocator.

---

## Game Scope (v1, for the demo)

### In scope
- ~20 Gen 1 Pokémon with type-coverage representation
- Standard turn-based 1v1 battle: select move, resolve turn, repeat until KO
- Status effects: burn, freeze, sleep, paralysis, poison
- Type effectiveness, STAB, critical hits, RNG damage variance
- 2-player local play (both players on the same board)
- NeoPixel feedback: HP bars, attack flash on hit, faint animation
- E-ink display: current turn state, HP, active Pokémon, move list

### Explicitly out of scope (v1)
- AI opponent
- Full 151-Pokémon roster
- Full move list (only the moves the chosen Pokémon use)
- Sound
- Wireless multiplayer
- Pokémon switching mid-battle (optional v2 if time permits)
- Items, abilities (Gen 1 abilities don't exist anyway)

---

## Architecture

Recommended high-level structure:

```
src/
  main.rs              # embassy_executor entry, spawns tasks
  hw/
    neopixels.rs       # PIO-driven WS2812B driver wrapper
    eink.rs            # e-ink draw routines (HP bar, text, sprites)
    input.rs           # button matrix, debouncing
  game/
    state.rs           # battle state, integration with battler crate
    presentation.rs    # game state -> NeoPixel + e-ink updates
    flow.rs            # menu, move selection, turn loop
  data/
    pokemon.rs         # Gen 1 Pokémon stats (const tables)
    moves.rs           # move definitions
```

### Concurrency model
Two embassy tasks:
1. **Game task** — runs the battle loop, calls into `battler` for turn resolution, queues display/LED updates
2. **Hardware task** — drives e-ink + NeoPixels, polls inputs

Communication via embassy channels.

---

## First Milestones (in order)

### Milestone 1: Hardware bring-up (Week 1–2)
- Blink an LED on the Pico from Rust
- Drive a NeoPixel strip — solid color, then a simple animation, using `ws2812-pio`
- Display "Hello World" on the e-ink display via `epd-waveshare`
- Read a button press over GPIO with debouncing

**Done when:** all four peripherals work in isolation. No game logic yet.

### Milestone 2: Battle engine integration (Week 3–4)
- Pull in `battler` crate, get it compiling for `thumbv6m-none-eabi`
- Set up an allocator if needed
- Hardcode a 1v1 battle (Charizard vs. Blastoise, both with fixed movesets)
- Run a turn through the engine in a unit test or `defmt`-printed simulation
- Verify damage numbers match Gen 1 expectations

**Done when:** the engine resolves a full battle correctly with output going to RTT logs. No display yet.

### Milestone 3: Input + display flow (Week 5–6)
- Move-selection UI on e-ink
- HP bars on NeoPixels (color shift green → yellow → red)
- Attack flash animation on hit
- Faint animation
- Two-player input working — players alternate selecting moves

**Done when:** a full battle is playable end-to-end on the hardware with both players using buttons.

### Milestone 4: Roster + polish (Week 7–10)
- All ~20 Pokémon implemented as const data tables
- Per-Pokémon move sets
- Status effect visual feedback (different LED colors / icons)
- Edge cases: PP exhaustion, sleep counter, recharge turns, etc.

### Milestone 5: Enclosure + demo prep (Week 11–12)
- Enclosure (3D printed or laser cut)
- Battery if going untethered
- Stress test — survive 30 minutes of strangers playing
- Clear instructions printed on the board

---

## Constraints / Rules of the Road

- **No premature features.** AI, switching, full roster — all explicit v2. Keep them out.
- **No reimplementing battle logic.** That's what the `battler` crate is for. If something seems easier to write from scratch, it's probably because the integration isn't understood yet.
- **Profile memory early.** Heap exhaustion on RP2040 manifests as silent crashes — instrument allocations.
- **Hardware-in-the-loop testing only.** The whole point is the physical demo. Don't get sucked into pure-software refinement.
- **Comment policy:** match the rest of this repo — no commentary, only WHY-comments for genuinely subtle code.

---

## Open Questions for Implementation
- Does `battler` ship Gen 1 datasets, or do we need to build the Pokémon/move tables ourselves?
- What's the memory footprint of `battler` on `thumbv6m-none-eabi` once linked?
- Will `battler-state` serialization fit our needs, or do we need a custom representation?
- Does `epd-waveshare` support the exact 2.9" model we're ordering, or do we need a fork?
- Is one e-ink display enough for a 2-player game, or do we need two (one per side)?
