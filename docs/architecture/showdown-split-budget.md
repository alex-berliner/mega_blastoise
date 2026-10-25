# Showdown split: memory and speed budget

How the new battle server (`ps_sim`, `ps_dex`, `ps_protocol`, `ps_client`)
is measured against design section 6 of `showdown-split-plan.md`, and what
the measurement says. Task P2-4 of `showdown-split-execution.md`.

**Every number in this file is a host proxy, and every budget is a soft
gate until an RP2350 board exists** (Alex, 2026-10-23). The on-chip run
(`sim_bench` over RTT) is written and builds, but it is deferred, not
dropped. When a board is available, run it and add its numbers beside the
proxies below.

## The budget (design section 6)

| Budget | Limit |
|---|---|
| RAM per battle | at most 16 KB, fixed capacity, zero heap during a turn |
| Stack | at most 2 KB per event dispatch; a whole turn under 16 KB |
| Flash | at most 250 KB for sim code, handlers and both dexes |
| Speed | a full turn under 2 ms at 150 MHz |

## How to measure

- `scripts/sim-budget.sh` does everything that runs without a board and
  prints the table below (about 15 s warm, a few minutes cold). The ELFs,
  the raw test output and `table.md` are written to `.scratch/sim-budget/`.
  `--quick` skips the stack-frame builds. The script fails only if the host
  test fails (a size over its ceiling, or an allocation inside a turn) or a
  build fails. Budgets over their limit print `OVER (soft)`.
- `cargo test -p ps_sim --test budget` (`crates/ps_sim/tests/budget.rs`) is
  the host test. It also runs in `check.sh`'s workspace tests. In a debug
  build the timing and stack probes play 20 battles per gen as a smoke run.
- `crates/mega_blastoise_fw/src/bin/sim_bench.rs` is the board bench. Build
  it from `crates/mega_blastoise_fw` with `cargo build --release --bin sim_bench`,
  and run it with `probe-rs run --preset pico2 <sim_bench elf>` (or
  `cargo run --release --bin sim_bench`). Feature `sim-bench-baseline`
  compiles the battles out; the flash delta is taken against that build.

Both the host test and the board bench play the same battles, from one file:
`crates/ps_sim/tests/common/bench_driver.rs`. That file holds 200 seeded
battles per ported gen, in each gen's custom game (the protocol diff's
formats). Teams are six distinct species with four distinct moves each,
from `ps_sim::mods::ported(gen)` (the `--teams wide` pools). Gen 3 sets
also get a random ported ability; nothing carries an item. Choices are
random: an enabled move, or a living benched Pokemon (one time in eight,
and always on a forced switch). A rejected choice is retried as `default`.

A command is one `Battle::step` plus its `Battle::send_updates`. A turn is
every command that answers one round of requests: the first side's choice
only records it, and the second side's runs the turn. The two `>player`
commands (teams, `start`, the leads' switch-in) are "setup" and are not
counted as turns. Battle construction (`Battle::from_start`) happens
outside every measured window.

### Method, per budget

1. **RAM.** `size_of` of `Battle`, `Side`, `Pokemon` and `EffectState` on
   the host is asserted in `budget.rs`: `Battle` against 16 KB, the others
   against ceilings set from this measurement with a few percent of
   headroom (`Side` 6 KB, `Pokemon` 896 B, `EffectState` 40 B), so growth
   fails before it eats the battle's margin. `sim_bench` asserts the
   thumbv8m `Battle` size at compile time and exports all four as
   `SIM_BENCH_SIZES`, which the script reads out of the ELF.
2. **Zero heap in a turn.** A counting global allocator in `budget.rs` is
   armed around every command of 500 battles per gen (`setup` included)
   and must count zero allocations. `the_counter_counts` checks the
   instrument itself. On the board, the bench compares `HEAP.used()` before
   and after every command.
3. **Flash.** `sim_bench` is built release for `thumbv8m.main-none-eabihf`,
   both gens linked. Two numbers come out of that build:
   - the sum of the `llvm-nm --print-size` sizes of the symbols owned by
     `ps_sim`, `ps_dex`, `ps_protocol` and `ps_client`. A symbol belongs to
     the first of those crates its demangled name mentions, so generic
     code instantiated on a `ps_sim` type counts as `ps_sim`;
   - the `.text` / `.rodata` delta against the baseline build.

   The symbol sum misses unnamed read-only data (string literals, the
   dex's name strings, panic locations); the section delta catches all of
   it, so the delta is the number to hold against the budget. The same
   symbol sum over the shipped firmware ELF shows `ps_client` too (the
   bench does not use it).
4. **Speed (proxy).** Wall time per turn on the host, release build, over
   the bench's 200 battles per gen: p50, p99, max in ns. The script turns
   that into an M33 estimate: host ns x host MHz gives host cycles. Those
   are multiplied by 3 (range 2 to 4) to get M33 cycles at 150 MHz. The
   factor is a judgement, not a measurement: a Zen 2 core retires roughly
   2 to 4 times the instructions per cycle a Cortex-M33 does on branchy
   code running from cached XIP flash. This machine exposes no hardware
   performance counters (WSL2) and has no Arm emulator, so instruction
   counts were not available. Other builds on the machine slow the host
   timing, so the table header records the load average.
5. **Stack (proxy).** `budget.rs` paints 256 KB below the driver's frame
   before every command, then finds the lowest overwritten word after it
   (the method the board bench uses, on the host). The window starts 512 B
   below the probe's frame, past the x86-64 red zone. A thumb estimate
   scales that depth by the ratio of `ps_sim`'s stack frames on thumbv8m
   to the same functions' frames on x86-64. Both come from
   `-Z emit-stack-sizes` builds of `ps_sim` (`llvm-readobj --stack-sizes`),
   and the ratio is 0.96.
6. **On the board (deferred).** `sim_bench` paints 32 KB below the stack
   pointer before each battle and scans it after. It reads the DWT cycle
   counter around every command and reports over RTT, per gen:
   - cycles per turn (p50, p99, max), and the same in microseconds at
     150 MHz;
   - the number of commands where `HEAP.used()` moved (must be 0);
   - the stack high-water mark.

## Results

Measured 2026-10-24 on `w/P2-4` (on top of `98bc57d`), with
`scripts/sim-budget.sh` on an AMD Ryzen 7 3700X at 3593 MHz under WSL2
(load average about 15). **All host proxies, soft gates.**

State: every Gen 1 effect is ported. **Gen 3 has only its move pipeline**:
its registry holds 5 effects, and its teams draw from 146 ported moves and
8 abilities with no items. So the Gen 3 rows measure a small part of what
Gen 3 will run, and its flash and time will grow.

| Measure | Value (host proxy) | Budget | Status |
|---|---|---|---|
| `size_of::<Battle>()` | 14,944 B thumb, 14,952 B host | 16,384 B | within (91%) |
| `size_of::<Side>()` | 5,796 B thumb and host | 6,144 B ceiling | within |
| `size_of::<Pokemon>()` | 872 B thumb and host | 896 B ceiling | within |
| `size_of::<EffectState>()` | 36 B thumb and host | 40 B ceiling | within |
| allocations inside a turn, Gen 1 | 0 over 60,939 commands (500 battles) | 0 | within |
| allocations inside a turn, Gen 3 (partial) | 0 over 55,283 commands (500 battles) | 0 | within |
| host time per turn, Gen 1 (13,223 turns) | p50 75,600 ns, p99 278,190 ns, max 517,497 ns | - | - |
| est. M33 time per turn, Gen 1 | p50 5.4 ms (3.6 to 7.2), p99 20.0 ms, max 37.2 ms | 2 ms | **OVER (soft)** |
| host time per turn, Gen 3 partial (11,100 turns) | p50 71,140 ns, p99 185,851 ns, max 294,550 ns | - | - |
| est. M33 time per turn, Gen 3 partial | p50 5.1 ms (3.4 to 6.8), p99 13.4 ms, max 21.2 ms | 2 ms | **OVER (soft)** |
| host stack per choice command, Gen 1 | p50 7,640 B, p99 12,696 B, max 18,048 B | - | - |
| host stack per choice command, Gen 3 partial | p50 7,832 B, p99 11,792 B, max 11,792 B | - | - |
| host stack, `>player` (setup) | max 22,320 B | - | - |
| est. thumb stack per command | Gen 1 max 17,268 B; Gen 3 max 11,282 B; setup 21,355 B | 16,384 B per turn | **OVER (soft)** for Gen 1 and setup |
| est. thumb stack, battle construction | `Battle::from_start` frame 15,048 B, plus the `Battle::new` it calls, 12,760 B: about 28 KB | - | see below |
| largest `ps_sim` thumb frames in a turn | `run_event_full` 4,640 B; `field_event` 4,312 B; `run_event_spread` 4,232 B | 2 KB per dispatch | **OVER (soft)** |
| `sim_bench` ps_* symbols | ps_sim 205,544 text + 1,712 rodata; ps_dex 12,374 + 52,596; ps_protocol 12,556 + 0; total 284,782 B | 250,000 B | **OVER (soft)** |
| `sim_bench` minus baseline | 256,624 B `.text` + 141,084 B `.rodata` = 397,708 B | 250,000 B | **OVER (soft)** |
| shipped firmware, ps_* symbols | 319,472 B (ps_sim 207,568; ps_dex 65,940; ps_protocol 15,342; ps_client 30,622) | - | - |
| shipped firmware sections | 571,872 B `.text`, 1,265,268 B `.rodata`, 427,900 B `.bss` | 4 MB flash | within |

The host timing is noisy at this load. Three release runs within an hour
gave a Gen 1 p50 of 56, 76 and 94 microseconds. Only about 1% of a turn's
time is `send_updates`; the rest is `step`, the turn itself (a one-off
split, not kept in the test).

## What this says

- **RAM** is inside the budget: `Battle` is 91% of 16 KB, and no turn
  allocates. P1-6's report lists cheap reductions if the margin is needed:
  `VOLATILE_CAP` 12 to 8, `EffectState` 36 to 28 B, `QUEUE_CAP` 16 to 10,
  `LOG_CAP` 64 to 32, slimmer `base_move_slots`.
- **Speed** probably misses the 2 ms turn by 2 to 4 times at p50 if the
  factor of 3 holds, and by more at p99. The host proxy is about 75 us
  per turn on a 3.6 GHz desktop core. This is the first thing the board
  must confirm. If it holds, profiling `step` is the next task: the event
  dispatch (`run_event_full`, `field_event`, `find_event_handlers`) is
  the hot path by design.
- **Stack** sits at the 16 KB per-turn line on thumb (17.3 KB estimated
  worst Gen 1 command). The dispatch frames are about 4.5 KB each against
  the design's 2 KB, which also explains the depth (three or four nested
  dispatches). Battle construction needs about 28 KB of stack because
  `Battle::new` and `from_start` each build a whole `Battle` in their
  frame. The firmware leaves about 84 KB of RAM to the stack (512 KB less
  the 428 KB `.bss`), so this fits today. In-place construction would
  remove it.
- **Flash** is over: about 398 KB for the sim, both dexes and the protocol
  with Gen 3 barely started, against 250 KB. `ps_sim`'s own code is about
  206 KB of that. The flash itself is not short (4 MB, the firmware uses
  1.8 MB), so this budget is about the design's intent, not a hard wall.

## Running it on the board (deferred)

1. `cd crates/mega_blastoise_fw && cargo build --release --bin sim_bench`.
2. `probe-rs run --preset pico2 ../../target/thumbv8m.main-none-eabihf/release/sim_bench`.
3. The bench prints the thumb sizes, then for each gen: battles and turns,
   cycles per turn (p50, p99, max) and the same in microseconds at 150 MHz,
   the number of commands where `HEAP.used()` moved (must be 0), and the
   stack high-water mark. Then `sim_bench: done`. The 400 battles take a
   few minutes if the estimate above holds.
4. Add the numbers to the table above, in a "board" column, and turn the
   speed row into a hard gate.
