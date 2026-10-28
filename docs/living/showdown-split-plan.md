# Showdown-compatible split: battle server and client

Status: accepted, 2026-10-23 (decisions in section 11; task breakdown in
`showdown-split-execution.md`). Replaces `crates/gen1_battle`,
`crates/gen3_battle` and `crates/battle_types`.

## 1. Why

Both engines are hand-written imperative re-implementations of Pokemon Showdown
(PS). Validation against the real sim (`tools/showdown-parity`, the fuzz suites)
took Gen 3 from 2.5% to about 0.4% failing wide-fuzz seeds and Gen 1 to about
0.07%, but the curve has flattened, and the remaining Gen 3 bugs are almost all
about ORDER and TIMING rather than individual moves:

- which `BeforeMove` handler answers first (flinch 8, Disable 7, Taunt 5,
  Imprison 4, confusion 3, attraction 2, paralysis 1, Bide 0);
- residual order across and within a mon (`speedSort` is a selection sort; a
  mon's volatiles are gathered in insertion order);
- when `eachEvent('Update')` runs (per strike, after an OHKO, at the end of
  `runAction`);
- whether an action reached `AfterMove` (`runMove` vs `useMove`);
- faints settled once (`faintMessages`), replacements asked mid-turn
  (`checkFainted` when the next action is a move or the residual).

PS gets all of these for free from one event pipeline. Our engines encode each
one as a flag (`hit_this_turn`, `faint_announced`, `vol_stamp`,
`acting_move_id`, `struggling_at_request`, ...), the flags interact, and two
fixes this round caused regressions caught only by re-running old seeds.

The fix is structural: mirror PS's sim architecture and protocol, so mechanics
port nearly line by line and the device can one day talk to a real PS server.

## 2. Goals and non-goals

Goals
1. A FAITHFUL re-implementation of the PS battle server, `no_std`: action
   queue, `runAction`, `runMove` / `useMove` / `tryMoveHit` / `spreadMoveHit`,
   `runEvent` with PS's handler ordering, `faintMessages` / `checkFainted`,
   residual. Gen 1 and Gen 3 are mods over one core, as in PS. The measure is
   complete parity of outcomes, not "close enough".
2. The server speaks the PS sim protocol (section 3) and produces the same log
   lines as PS for the same inputs and seed.
3. A `no_std` CLIENT that builds its battle view only from protocol messages and
   feeds the existing UI layer (`oled_ctl`, `choice_collect`) unchanged in
   behaviour (ZERO-DRIFT, `CLAUDE.md`). The client is protocol-exact and
   transport-agnostic, so a future board with a radio can talk to a real PS
   server natively with no client changes.
4. One generic runner in `mega_blastoise_core` for both gens (today: two, 568 and
   1259 lines).
5. Fits the RP2350 comfortably: fixed RAM per battle, no per-turn heap, dex and
   handlers in flash (section 6).
6. RNG-exact parity: same seed in, same log out, for whole random battles, not
   only scripted ones.

Non-goals (for this rewrite)
- Gens 2 and 4+, doubles, team preview UI, Dynamax/Tera/Mega.
- Online play of any kind: no WebSocket transport, accounts, ladders or
  matchmaking. The old phase 5 (web build against a real server) is deferred;
  only the `Transport` seam (section 7) is kept so it can be added later.
- Changing what the player sees or presses, except the new Draw end screen
  (section 7). Message ORDER may change where PS orders differently; that
  follows the standing rule "Showdown decides".

## 3. What "Showdown-compatible" means

The subset of the PS sim protocol (`sim/SIM-PROTOCOL.md` and the package's
root `PROTOCOL.md` in the vendored package) we implement exactly:

Input (BattleStream commands)
- `>start {"formatid":"gen3customgame","seed":[a,b,c,d]}` (four 16-bit
  integers; a numeric seed selects PS's `Gen5RNG`)
- `>player p1 {"name":"...","team":"<packed team>"}` (PS packed format)
- `>p1 move N`, `>p1 switch N`, `>p1 pass`, `>p1 default`, `>p1 undo`
  (1-based, PS grammar; `move N` may carry a target only in doubles: unused)
- `>forcewin`, `>forcetie` (debug only)

Output
- `update` blocks of `|`-separated lines, `sideupdate\npN\n|request|{json}`,
  `|split|pN` secret/public pairs for exact vs percentage HP.
- Header lines: `|t:|`, `|gametype|`, `|player|`, `|teamsize|`, `|gen|`,
  `|tier|`, `|rule|`, `|start`.
- Major actions: `|move|`, `|switch|`, `|drag|`, `|detailschange|`, `|cant|`,
  `|faint|`, `|turn|`, `|win|`, `|tie|`, `|upkeep|`.
- Minor actions used by Gen 1/3: `-damage`, `-heal`, `-sethp`, `-status`,
  `-curestatus`, `-cureteam`, `-boost`, `-unboost`, `-setboost`,
  `-clearallboost`, `-crit`, `-supereffective`, `-resisted`, `-immune`,
  `-miss`, `-fail`, `-notarget`, `-activate`, `-start`, `-end`, `-weather`,
  `-sidestart`, `-sideend`, `-fieldstart`, `-fieldend`, `-item`, `-enditem`,
  `-ability`, `-transform`, `-mustrecharge`, `-prepare`, `-singleturn`,
  `-singlemove`, `-hitcount`, `-ohko`, `-message`, `-hint`.
- `|request|` JSON with PS's shape: `active[].moves[]` (`move`, `id`, `pp`,
  `maxpp`, `target`, `disabled`), `active[].trapped` / `maybeTrapped`,
  `forceSwitch`, `wait`, `side.pokemon[]` (`ident`, `details`, `condition`,
  `active`, `stats`, `moves`, `baseAbility`, `item`), `rqid`. The server's
  request text is byte-exact (same key order) so a real PS client could read it.

Formats: `gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod` and
`gen3customgame`, the two the harness already uses. Their rule tables, as PS
resolves them: Gen 1 is HP Percentage Mod, Cancel Mod, Desync Clause Mod,
Sleep Clause Mod, Freeze Clause Mod; Gen 3 is HP Percentage Mod and Cancel Mod
only (no sleep clause). Every battle-affecting rule is ported as a format rule
with handlers. Our deliberate deviations from stock PS become named format
rules, never silent differences:
- Gen 1 cartridge truncation (stat rollover past 1023 via `trunc(x, 8)`, PP
  underflow to 63 via `trunc(pp, 6)`). This is stock PS behaviour in every
  non-customgame Gen 1 format (`Dex.trunc`); only the customgame formats swap
  in `Math.trunc` for Level 9999 support. The server implements `trunc` per
  format, and the local Gen 1 format turns the bit-width version back on as a
  rule `Cartridge Trunc`. The new harness applies the same one-line
  `battle.trunc` restore the old one does (`cartridgeTrunc`); it is a format
  property, not outcome scripting.

Ties: PS emits `|tie|` (via `win()` with no side) in three ways that reach our
formats: a double KO of both last mons in gens 1 to 4 (`checkWin`), Gen 1's
no-progress check in `maybeTriggerEndlessBattleClause` (runs in Gen 1 whether
or not the rule is on), and the turn-1000 limit (every format). The server
emits it exactly there and the client shows one Draw end screen (section 7).

Internally nothing is text: the server emits typed messages (`Msg` enum) and
takes typed commands; text encode/decode exists only at the edges (harness,
debug log, any future network transport) behind a `text` feature.

## 4. Crate layout

All `no_std` + `alloc` unless noted. New crates live beside the old ones until
each gen passes its gate (section 9).

| Crate | Role |
|---|---|
| `ps_protocol` | `Msg`, `Request`, `Command`, `PokemonIdent`, packed-team types; `text` feature: PS line encoder/decoder and request JSON (serde-free hand codec, bounded) |
| `ps_dex` | build.rs-generated static tables per gen from PS dumps (species, moves, items, abilities, typechart, conditions), mod inheritance resolved at dump time; ids are `u16`; owns the `Type` enum (the old `battle_types` crate is folded in) |
| `ps_sim` | the server core: state, queue, events, PRNG, damage, protocol emitter; `gen1` and `gen3` features select the mods |
| `ps_sim::mods::{gen1,gen3}` | per-gen handler code and pipeline overrides (section 5.4) |
| `ps_client` | battle view from protocol, request to choice model, no UI |
| `mega_blastoise_core::runner` | one generic runner over a `Transport` (in-process server, or text stream) |
| `ps_harness` (std, test-only) | `ps_sim_cli` (the server over stdin/stdout, BattleStream text) and the cargo test wrapper; the driver itself is `tools/ps-diff/diff.js`, which feeds identical commands to PS's `BattleStream` and to `ps_sim_cli` |

`battle_types` (the `Type` enum, 96 lines) becomes `ps_dex::Type` in phase 1;
the old crate is a re-export shim until the Gen 3 engine, its last user, is
deleted.

Not reused: the `battler` submodule (jackson-nestelroad/battler). It is
Showdown-inspired and `no_std`, but data-driven through runtime JSON and the
`fxlang` interpreter (1.9 MB of engine source), speaks its own `key:value` log
format, and the project already moved off it once for size and fidelity. Ideas
from `battler-state` (client view) are worth reading, not depending on. Note
that the move off it is not complete: `mega_blastoise_core` still links
`battler-data` (`DataStore` in `data_store.rs`, `runner/gen1.rs`,
`device_runner.rs`), and the fw, test and web crates list `battler` /
`battler-data` path deps. Those go with the Gen 1 engine.

## 5. Server design

### 5.1 State
- `Battle { field, sides: [Side; 2], queue, prng, turn, log, rqid, ... }`,
  fixed capacity, no `Vec` in the hot path.
- `Side { mons: [Pokemon; 6], active: [u8; 1], side_conditions: EffectTable<4>,
  choice, request_state }`.
- `Pokemon` mirrors PS fields that mechanics read: `species`, `base_species`,
  `level`, `gender`, `stored_stats`, `boosts [i8; 7]`, `hp`, `maxhp`, `status` +
  `status_state`, `volatiles: EffectTable<12>`, `move_slots [MoveSlot; 4]`,
  `base_move_slots`, `item`, `ability`, `last_move`, `last_damage`,
  `attacked_by: RingBuf<AttackRecord, 4>`, `ability_state`, `item_state`, and
  the gen-specific bits (gen 1 move registers, `subFainted`, Toxic counter).
- `EffectTable<N>` = up to N `(EffectId, EffectState)` in INSERTION order (PS
  iterates `pokemon.volatiles` by insertion, which decides residual ties).
- `EffectState` = fixed struct: `duration: i8`, `effect_order: u16`,
  `source: Option<MonRef>`, `source_slot`, `move: Option<MoveId>`, `counter`,
  `layers`, `hit_count`, `true_duration`, `data: [i16; 4]` for the rare extra.
  Every PS condition's `this.effectState.*` fields map onto these by name in a
  per-condition doc comment.

### 5.2 Action queue
PS `BattleQueue` with its `orders` table (`team 1, start 2, instaswitch 3,
beforeTurn 4, beforeTurnMove 5, runSwitch 101, switch 103, move 200, residual
300`), `priority`, `speed`, `subOrder`, and the same `sort()` (a `speedSort`
with PRNG tie shuffles). Capacity 16. `turnLoop` / `runAction` ported as is,
including `checkFainted` and `faintMessages` semantics (gen <= 3: faints checked
when the next action is a move or residual; a faint cancels remaining actions in
gen <= 3 singles).

### 5.3 Events
PS's `runEvent` / `singleEvent` / `eachEvent` / `fieldEvent` /
`findEventHandlers` / `resolvePriority` / `speedSort`, with:
- `EventId: u8` enum (about 120 events: `BeforeMove`, `ModifyAtk`, `Damage`,
  `Residual`, `Update`, `AfterMove`, `DisableMove`, `LockMove`, ...).
- Static handler tables: each effect definition (move, condition, item,
  ability, species, format rule) is `&'static EffectDef { handlers:
  &'static [Handler] , events: EventMask(u128) }` where `Handler { event,
  prefix (none/source/foe/ally/any), order, priority, sub_order, f: fn(...) }`.
  The `events` bitmask lets gathering skip effects that do not answer.
- Gathering writes into a stack array (cap 32) of `(handler, state ref,
  effect_holder, order, priority, speed, sub_order, effect_order)` and sorts with
  PS's `comparePriority` via the same selection sort, shuffling exact ties with
  the PRNG exactly where PS does (`speedSort` calls `prng.shuffle` on each run
  of equal entries, so one tie costs one or more PRNG calls).
- Relay values are a small enum (`Int(i32)`, `Bool`, `Move(MoveId)`,
  `Fail`, `NotFail`, `Null`, `HitSubstitute`) so PS's `false` / `null` /
  `0` / `undefined` distinctions survive (several Gen 3 bugs were exactly that).
- Fixed-point maths: `modify` / `chainModify` / `finalModify` in 4096ths and
  `tr()` truncation as PS (already proven in `gen3_battle::ability::modify`).

### 5.4 Gens as mods
PS overrides core flow per gen in `data/mods/genN/scripts.ts`, and each mod
inherits the next one up (`gen1 -> gen2 -> gen3 -> gen4 -> gen5 -> base`):
- gen1 overrides actions `runMove`, `useMove`, `useMoveInner`, `tryMoveHit`,
  `moveHit`, `getDamage`; pokemon `deductPP`, `getStat`, `modifyStat`,
  `boostBy`, `clearBoosts`; battle `boost` (843 lines). Everything else it
  INHERITS, notably gen4's `runSwitch` and gen3's `getActionSpeed`. Gen 2's
  overrides (`runMove`, `tryMoveHit`, `moveHit`, `getDamage`, `getStat`,
  `boostBy`) are all re-overridden by gen 1, but gen 1's data (moves,
  conditions) still inherits gen 2's data mods, which `Dex.mod('gen1')`
  resolves.
- gen3 overrides `useMoveInner`, `tryMoveHit`, `modifyDamage`,
  `getActionSpeed`; inherits gen4's `runSwitch`, `hitStepAccuracy`,
  `hitStepInvulnerabilityEvent`, `applyRecoilDamage`, and the rest from base.

So the queue, events, state, protocol and residual are shared; the MOVE
PIPELINE is per gen. Design: `trait GenScripts` with those functions, two
implementations (`Gen1Scripts`, `Gen3Scripts`, each folding in what it
inherits at port time), chosen at battle creation by an enum dispatch (no trait
objects in the hot path). Data tables are per gen from `ps_dex`.

### 5.5 PRNG
Implement PS's `PRNG` API (`random`, `randomChance`, `sample`, `shuffle`) over
its `Gen5RNG` (the 64-bit LCG PS uses for numeric seeds) bit-exactly, and call
it in the same places and order as PS. That includes calls outside move
resolution: `Pokemon`'s constructor samples a gender (`sample(['M','F'])`) for
every species without a fixed one, `speedSort` shuffles ties, and Gen 3's
`endTurn` rolls Quick Claw (`randomChance(1, 5)`) every turn whether or not
anyone holds one. The harness starts PS with the same `[a,b,c,d]` seed and
compares whole battles with random choices, no monkeypatching.

There is no scripted mode in the new server and no Showdown-side scripting in
the new harness. A test-only outcome override (forcing the next N PRNG results)
exists for Rust unit tests only, behind `cfg(test)` or a `test-override`
feature the firmware never enables. The old scripted fuzz suites keep guarding
the old engines until each gen switches. Later PS versions default to the
`sodium` (ChaCha20) PRNG for generated seeds but still accept numeric seeds; the
re-sync pass (section 9) confirms that before relying on it.

### 5.6 Output
Typed `Msg` into a bounded ring (cap 256 per update; a full ring flushes to the
consumer, never reallocates). `split` secret/public variants carried as one
`Msg` with both HP forms. Request built on demand from state, not stored.

## 6. Memory and speed budget

Measured today (firmware release build on .167, commit 481bfdc):
- FLASH 4 MB; `.text` 233 KB, `.rodata` 1.14 MB (mostly sprites).
- RAM 512 KB; `.bss` 419 KB of which the heap is 384 KB.
- Engine code: gen3_battle about 48 KB, gen1_battle about 23 KB of `.text`;
  gen1 tables about 10 KB.
- State: `gen3_battle::Battle` 440 B plus party on the heap (`Mon` 416 B, up to
  12); `gen1_battle::PublicCoreBattle` 3.7 KB plus heap.

Port size (counted with `Dex.mod(genN)` resolution in PS 0.11.11, standard
entries only, nested `condition` / `secondary` / `self` functions included):
- Gen 1: 165 moves, 40 with handlers (66 functions); about 17 conditions in play
  (48 functions); 151 species; no items or abilities; 5 format rules with handlers.
- Gen 3: 370 moves, 129 with handlers (296 functions); 106 items, 60 with
  handlers (99 functions); 76 abilities, 68 with handlers (96 functions); about
  20 conditions in play (69 functions); 392 species.
- About 670 handler functions in all, plus the two move pipelines.

Budget for the new code (both gens linked):
- RAM per battle: at most 16 KB, all fixed capacity, zero heap during a turn.
  Rough: 12 `Pokemon` at about 400 B, 2 `Side` at about 200 B, queue 16 x 32 B,
  log ring 256 x 24 B, field and request scratch about 1 KB.
- Stack per event dispatch: at most 2 KB (handler array 32 x 40 B plus frames);
  nesting depth of `runEvent` is bounded in practice (about 6), so a whole turn
  stays under 16 KB of stack.
- Flash: at most 250 KB for sim code, handlers and both dexes.
- Speed: a full turn in under 2 ms at 150 MHz (measured on the board in
  phase 2 with the DWT cycle counter; the budget is a gate, not a guess).
Techniques: `u16` ids, `u8` event ids, event bitmasks, `&'static` tables,
no `String` in state, no JSON on the device (typed in-process path), text only
under the `text` feature. The measurement method is task P2-4 in the execution
plan.

## 7. Client

`ps_client::BattleView` holds what a PS client knows: per side the active mon
(ident, details, HP as exact for own side and percentage or exact for the foe
per `|split|`), status, boosts, visible volatiles, known moves and PP, field and
side conditions, turn. `apply(&Msg)` updates it and yields zero or more
`BoardEvent`s, the boundary the UI already consumes.

The request path: `Request` -> `choice_collect::SlotOptions` (usable moves,
trapped, forced switch, party validity) -> player input -> PS choice string
(`move N` / `switch N`, 1-based) -> `Command`. `choice_collect` and `oled_ctl`
stay the only places that decide prompts, validation and screens (ZERO-DRIFT);
the client only translates.

The generic runner replaces `runner/gen1.rs` and `runner/gen3.rs`: loop
`transport.poll()` -> `view.apply()` -> board events; on a request ->
`SlotOptions` -> `InputBus` -> `transport.send(choice)`. Both gens use the same
runner because both speak the same protocol.

`Transport`: `Local` (in-process `ps_sim`, typed, no text; the device and the
web default), `Text` (PS line protocol over any byte stream; the harness today,
and the seam for a future radio or network link). The client never assumes the
server is local: everything it shows comes from protocol messages, so pointing
`Text` at a real PS server is a transport change, not a client change.

End of battle: `|win|` keeps today's result screens. `|tie|` shows one Draw end
screen for both seats, designed in core (`oled_ctl` decides it, the colour
renderer draws it); local play and any future online play use the same screen.
There is no HP tiebreak anywhere. (Today Gen 3 maps its tie to the generic
result screen with the text "TIE!", and Gen 1's engine picks a winner by
remaining HP in `endless_battle_winner`, which is itself a divergence from PS.)

Out of scope: the WebSocket transport, guest login, `/challenge`, accounts,
ladders, matchmaking (deferred with the old phase 5).

## 8. Testing

1. Protocol diff (primary gate). `tools/ps-diff/diff.js` feeds identical
   command streams (seed, packed teams, choices) to PS's `BattleStream` and to
   `ps_sim_cli` through the `Text` codec, and diffs the omniscient output line
   by line, requests included. The only normalisation is the `|t:|` timestamp.
   Gender letters need none: they come from the battle PRNG. The first
   divergence is reported with both logs, the seed and the PRNG call count.
   This replaces end-state diffs and the many harness seams. The new harness
   has its own PS install (`tools/ps-diff/`, pinned to 0.11.11) so the re-sync
   can move it without touching the old suites' pin.
2. RNG-exact random battles: teams from PS's own random-battle generator and
   from a wide generator (the current fuzz space: 1-6 mons, 1-4 moves, levels,
   natures, DVs/IVs/EVs, items and abilities in Gen 3), random legal choices
   drawn by the driver from PS's request, same seed both sides, whole battles
   to the end (turn cap 300 for runtime).
3. Corpus: every seed fixed during validation (about 90, listed in
   `/root/pv/open_seeds.txt` history and the commit messages of
   `showdown-validation`) becomes a pinned regression set for the OLD engines,
   run by `scripts/check.sh` from phase 0 on. Every protocol-diff seed that
   finds a bug in the new server is pinned the same way.
4. The existing fuzz suites keep running against the OLD engines until each gen
   switches, so nothing regresses meanwhile.
5. Device gates: RAM and flash budget from the linker map, turn time measured
   on the board, UI probe (`scripts/ui-probe.sh`) green on the new runner.

Per-gen exit gate: 0 protocol-diff failures over 20,000 random battles plus the
pinned corpus, check.sh green, budgets met, on-device smoke battle.

## 9. Migration

| Phase | Work | Exit criteria | Rough effort |
|---|---|---|---|
| 0 | Pin the fixed seeds into check.sh; freeze the old engines except for real bugs | corpus green in check.sh | 1-2 days |
| 1 | `ps_protocol`, `ps_dex` generation, `ps_sim` core (state, queue, events, PRNG, damage), `Text` codec, protocol-diff harness | a 1v1 battle of plain damaging moves matches PS line for line, RNG-exact, both gens' damage formulas | 1-2 weeks |
| 2 | `ps_client`, generic runner behind the existing runner entry points, per-gen runtime switch between old and new, Draw screen | UI probe green on the new path; firmware builds; RAM, flash, turn time measured against section 6 | about 1 week |
| 3 | Gen 1 complete (all moves, conditions, rules) | per-gen gate (section 8); then delete the Gen 1 engine, its runner and `battler` / `battler-data` | 1-2 weeks |
| R | Re-sync to current PS: diff 0.11.11 against the current release, port the changes that touch the core or Gen 1, re-pin `tools/ps-diff` | Gen 1 gate green against the new pin | 2-4 days |
| 4 | Gen 3, ported from the re-synced pin: infrastructure, conditions, moves, items, abilities | per-gen gate; delete the Gen 3 engine, its runner and `battle_types` | 2-3 weeks |

Phase 5 (web build against a real server) is deferred; see section 2.

Gen 1 is the pilot. It is the smaller port (66 move handler functions, no items
or abilities) so it shakes out the core, the protocol, the client and the
device integration fastest, and its current engine is already near parity,
which makes the gate easy to read. The re-sync comes after the pilot so Gen 3,
the bulk of the port, is written once against current PS.

During phases 1-4 nothing outside `mega_blastoise_core::runner` and the new
crates changes, except the Draw screen in `oled_ctl` and its renderer; fw and
web keep building against the same core API, so the rest of the code is not
fractured. `display.rs`, `display_color.rs`, `move_anim.rs`, `cli_parse.rs`,
`randbat.rs`, `demo_teams.rs`, `prompt_fmt.rs`, `random_ai.rs`,
`device_session.rs`, `choice_collect.rs` and `board_event.rs` currently import
`gen1_battle` or `gen3_battle` types (move tables, types, `MonData`); they move
to `ps_dex` / `ps_protocol` types in the same phase that deletes the engine.

## 10. Risks

- Porting volume: about 670 handler functions plus the gen pipelines. Mitigated
  by porting mechanically from the PS source with the protocol diff as the
  referee, one batch of effects at a time.
- Faithful event ordering costs cycles; the bitmask skip and fixed arrays should
  keep it well inside budget, but phase 2 measures it before the bulk port.
- RNG-call-order parity is strict: one extra or missing `random()` desyncs the
  rest of the battle. That is a feature (it localises bugs) but early on most
  diffs will be RNG-order diffs. The driver reports the PRNG call count at the
  first divergence, and the Rust side can log every call site.
- PS data or behaviour can change between versions; pin PS 0.11.11 for phases
  0 to 3, re-sync once (phase R), and regenerate dex dumps only deliberately.
- Model capacity: the architecture work (core, pipelines, client, the subtle
  effects) goes to the strongest worker; narrow, source-guided handler batches
  go to the cheaper tier with the protocol diff as their check. The execution
  plan assigns each task.

## 11. Decisions (Alex, 2026-10-23)

1. Pilot: Gen 1 first, then Gen 3.
2. PS version: port from 0.11.11 (what the harness pins). One re-sync pass to
   current PS after the Gen 1 pilot lands (phase R), before Gen 3.
3. Parity method: an exact, bit-for-bit port of PS's PRNG called in PS's order;
   the harness diffs protocol logs line by line from the same seed. A test-only
   outcome override exists for Rust unit tests only. No Showdown-side scripting
   in the new harness. The old scripted fuzz suites keep guarding the old
   engines until each gen switches.
4. Ties: a PS `|tie|` is shown as a tie, on one Draw end screen designed in core
   (`oled_ctl`), used by local and any future online play. No HP tiebreak.
5. Goal: a faithful re-implementation of the PS server with complete parity of
   outcomes. The client stays protocol-exact and transport-agnostic so a future
   board with a radio can talk to the real PS server natively. Web online play
   (old phase 5) is deferred: keep the transport seam, plan no accounts,
   ladders or matchmaking.
6. Execution: the main Claude session orchestrates, reviews and lands. Workers
   are Claude 2 (Opus; architecture and anything subtle) and MiniMax M3 /
   Hyper (narrow, well-specified, source-guided tasks with a clear fuzz or test
   check). One worker run is one headless session of about 2 hours in its own
   git worktree off the integration branch, with a self-contained brief, no
   questions mid-run, no commits, and a WORKER_REPORT.md. Builds and sims run
   on the WSL1 build machine (16 cores).
7. `crates/battle_types` (the shared `Type` enum) is folded into `ps_dex`.
