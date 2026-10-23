# Showdown-compatible split: battle server and client

Status: proposal, 2026-10-23. Replaces `crates/gen1_battle` and `crates/gen3_battle`.

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
1. A `no_std` battle SERVER whose control flow mirrors PS: action queue,
   `runAction`, `runMove` / `useMove` / `tryMoveHit` / `spreadMoveHit`,
   `runEvent` with PS's handler ordering, `faintMessages` / `checkFainted`,
   residual. Gen 1 and Gen 3 are mods over one core, as in PS.
2. The server speaks the PS sim protocol (section 3) and produces the same log
   lines as PS for the same inputs and seed.
3. A `no_std` CLIENT that builds its battle view only from protocol messages and
   feeds the existing UI layer (`oled_ctl`, `choice_collect`) unchanged in
   behaviour (ZERO-DRIFT, `CLAUDE.md`).
4. One generic runner in `mega_blastoise_core` for both gens (today: two, 568 and
   1259 lines).
5. Fits the RP2350 comfortably: fixed RAM per battle, no per-turn heap, dex and
   handlers in flash (section 6).
6. RNG-exact parity: same seed in, same log out, for whole random battles, not
   only scripted ones.

Non-goals (for this rewrite)
- Gens 2 and 4+, doubles, team preview UI, Dynamax/Tera/Mega.
- Hosting real online play on the firmware (the web build is the first place a
  real-server connection makes sense, section 8).
- Changing what the player sees or presses. Message ORDER may change where PS
  orders differently; that follows the standing rule "Showdown decides".

## 3. What "Showdown-compatible" means

The subset of the PS sim protocol (`sim/SIM-PROTOCOL.md`, `PROTOCOL.md` in the
vendored package) we implement exactly:

Input (BattleStream commands)
- `>start {"formatid":"gen3customgame","seed":[a,b,c,d]}`
- `>player p1 {"name":"...","team":"<packed team>"}` (PS packed format)
- `>p1 move N`, `>p1 switch N`, `>p1 pass`, `>p1 default`, `>p1 undo`
  (1-based, PS grammar; `move N` may carry a target only in doubles: unused)
- `>forcewin`, `>forcetie` (debug only)

Output
- `update` blocks of `|`-separated lines, `sideupdate\npN\n|request|{json}`,
  `|split|pN` secret/public pairs for exact vs percentage HP.
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
  `active`, `stats`, `moves`, `baseAbility`, `item`), `rqid`.

Formats: `gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod` and
`gen3customgame`, the two the harness already uses. Our deliberate deviations
from stock PS become named format rules, never silent differences:
- Gen 1 PP underflow to 63 (`trunc(pp, 6)`): the harness already patches the
  sim this way (`cartridgeTrunc`); it becomes a rule `Cartridge PP Underflow`.
- Endless Battle Clause has no tie in our API today; the server WILL emit
  `|tie|` and the client/UI decides how to show it (open question Q4).

Internally nothing is text: the server emits typed messages (`Msg` enum) and
takes typed commands; text encode/decode exists only at the edges (harness,
network, debug log) behind a `text` feature.

## 4. Crate layout

All `no_std` + `alloc` unless noted. New crates live beside the old ones until
each gen passes its gate (section 9).

| Crate | Role |
|---|---|
| `ps_protocol` | `Msg`, `Request`, `Command`, `PokemonIdent`, packed-team types; `text` feature: PS line encoder/decoder and request JSON (serde-free hand codec, bounded) |
| `ps_dex` | build.rs-generated static tables per gen from PS dumps (species, moves, items, abilities, typechart, conditions), mod inheritance resolved at build time; ids are `u16` |
| `ps_sim` | the server core: state, queue, events, PRNG, damage, protocol emitter; `gen1` and `gen3` features select the mods |
| `ps_sim::mods::{gen1,gen3}` | per-gen handler code and pipeline overrides (section 5.4) |
| `ps_client` | battle view from protocol, request to choice model, no UI |
| `mega_blastoise_core::runner` | one generic runner over a `Transport` (in-process server, or text stream) |
| `ps_harness` (std, test-only) | protocol-diff driver against the real sim |

The old `battle_types` crate shrinks to whatever the UI still needs (types and
type chart for colours) or is folded into `ps_dex`.

Not reused: the `battler` submodule (jackson-nestelroad/battler). It is
Showdown-inspired and `no_std`, but data-driven through runtime JSON and the
`fxlang` interpreter (1.9 MB of engine source), speaks its own `key:value` log
format, and the project already moved off it once for size and fidelity. Ideas
from `battler-state` (client view) are worth reading, not depending on.

## 5. Server design

### 5.1 State
- `Battle { field, sides: [Side; 2], queue, prng, turn, log, rqid, ... }`,
  fixed capacity, no `Vec` in the hot path.
- `Side { mons: [Pokemon; 6], active: [u8; 1], side_conditions: EffectTable<4>,
  choice, request_state }`.
- `Pokemon` mirrors PS fields that mechanics read: `species`, `base_species`,
  `level`, `stored_stats`, `boosts [i8; 7]`, `hp`, `maxhp`, `status` +
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
  PS's comparator via the same selection sort, shuffling exact ties with the
  PRNG exactly where PS does.
- Relay values are a small enum (`Int(i32)`, `Bool`, `Move(MoveId)`,
  `Fail`, `NotFail`, `Null`, `HitSubstitute`) so PS's `false` / `null` /
  `0` / `undefined` distinctions survive (several Gen 3 bugs were exactly that).
- Fixed-point maths: `modify` / `chainModify` / `finalModify` in 4096ths and
  `tr()` truncation as PS (already proven in `gen3_battle::ability::modify`).

### 5.4 Gens as mods
PS overrides core flow per gen in `data/mods/genN/scripts.js`:
- gen1 overrides `runMove`, `useMove`, `useMoveInner`, `tryMoveHit`,
  `moveHit`, `getDamage`, `deductPP`, `getStat`, `modifyStat`, `boostBy`,
  `clearBoosts` (843 lines);
- gen2 overrides `runMove`, `tryMoveHit`, `moveHit`, `getDamage`, `getStat`;
- gen3 overrides `useMoveInner`, `tryMoveHit`, `modifyDamage`,
  `getActionSpeed`; inherits gen4's `runSwitch`, `hitStepAccuracy`,
  `hitStepInvulnerabilityEvent`, `applyRecoilDamage`, `modifyDamage`.

So the queue, events, state, protocol and residual are shared; the MOVE
PIPELINE is per gen. Design: `trait GenScripts` with those functions, two
implementations (`Gen1Scripts`, `Gen3Scripts`, the latter folding in what it
inherits from gen 4/5 at port time), chosen at compile time by feature or at
battle creation by an enum dispatch (no trait objects in the hot path). Data
tables are per gen from `ps_dex`.

### 5.5 PRNG
Implement PS's `Gen5RNG` (the legacy 64-bit LCG PS still accepts for numeric
seeds) bit-exactly, and call it in the same places and order as PS (`random`,
`randomChance`, `sample`, `shuffle`, speed-tie shuffles in `speedSort`). The
harness then starts PS with the same `[a,b,c,d]` seed and compares whole
battles with random choices, no monkeypatching. The existing scripted seams
(hit, crit, roll, secondary, ...) stay available behind a `scripted` feature for
targeted tests. Optional later: PS's default `sodium` (ChaCha20) PRNG.

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

Port size (counted from the vendored PS data): Gen 1 has 165 moves, 40 with
handlers (66 handler functions), 151 species, no items or abilities. Gen 3 has
370 moves, 144 with handlers (341 handler functions), 106 items, 76 abilities,
392 species.

Budget for the new code (both gens linked):
- RAM per battle: at most 16 KB, all fixed capacity, zero heap during a turn.
  Rough: 12 `Pokemon` at about 400 B, 2 `Side` at about 200 B, queue 16 x 32 B,
  log ring 256 x 24 B, field and request scratch about 1 KB.
- Stack per event dispatch: at most 2 KB (handler array 32 x 40 B plus frames);
  nesting depth of `runEvent` is bounded in practice (about 6).
- Flash: at most 250 KB for sim code, handlers and both dexes.
- Speed: a full turn in under 2 ms at 150 MHz (to be measured on the board in
  phase 2; the budget is a gate, not a guess).
Techniques: `u16` ids, `u8` event ids, event bitmasks, `&'static` tables,
no `String` in state, no JSON on the device (typed in-process path), text only
under the `text` feature.

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
web default), `Text` (PS line protocol over any byte stream; the harness and
later the network).

Real server path: the web build adds a WebSocket `Text` transport to a PS
server (guest login, `/challenge`, `/choose ...|rqid`). The device stays
offline; a later USB or BLE bridge could reuse the same transport.

## 8. Testing

1. Protocol diff (primary gate). Feed identical command streams (seed, packed
   teams, choices) to PS's `BattleStream` and to `ps_sim` through the `Text`
   codec; normalise only what PS randomises outside the PRNG or timestamps
   (`|t:|`); diff line by line, first divergence reported with both logs. This
   replaces end-state diffs and the many harness seams.
2. RNG-exact random battles: random teams (PS random battle sets, or the
   current wide generator's space: 1-6 mons, 1-4 moves, natures, IVs/EVs, 2-30
   turns) with random legal choices, same seed both sides.
3. Corpus: every seed fixed during validation (about 70, listed in
   `/root/pv/open_seeds.txt` and the commit log) becomes a pinned regression
   set, run by `scripts/check.sh` from phase 0 on.
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
| 2 | `ps_client`, generic runner behind the existing runner entry points, per-gen switch (feature or runtime) between old and new | UI probe green on the new path; firmware builds; RAM, flash, turn time measured against section 6 | about 1 week |
| 3 | Pilot gen complete (all moves and conditions) | per-gen gate (section 8); then delete that gen's old engine and runner | 1-2 weeks (Gen 1) |
| 4 | Other gen: moves, then abilities, then items | per-gen gate; delete the old engine | 2-3 weeks (Gen 3) |
| 5 | Web build: real-server transport | can play a ladderless challenge against a PS server | about 1 week |

Pilot gen recommendation: Gen 1. It is the smaller port (66 move handler
functions, no items or abilities) so it shakes out the core, the protocol, the
client and the device integration fastest, and its current engine is already
near parity, which makes the gate easy to read. Gen 3 is where the value is,
and could go first if Alex prefers (Q1).

During phases 1-4 nothing outside `mega_blastoise_core::runner` and the new
crates changes; fw and web keep building against the same core API, so the rest
of the code is not fractured. `display.rs`, `display_color.rs`, `move_anim.rs`,
`cli_parse.rs`, `randbat.rs` and friends currently import `gen1_battle` types
(move tables, types, `MonData`); they move to `ps_dex` / `ps_protocol` types in
the same phase that deletes `gen1_battle`.

## 10. Risks

- Porting volume: about 400 handler functions plus the gen pipelines. Mitigated
  by porting mechanically from the PS source with the protocol diff as the
  referee, one effect at a time.
- Faithful event ordering costs cycles; the bitmask skip and fixed arrays should
  keep it well inside budget, but phase 2 measures it before the bulk port.
- RNG-call-order parity is strict: one extra or missing `random()` desyncs the
  rest of the battle. That is a feature (it localises bugs) but early on most
  diffs will be RNG-order diffs. The scripted seam mode is the fallback.
- PS data or behaviour can change between versions; pin PS 0.11.11 (current) and
  regenerate dex dumps only deliberately.
- Model capacity: this is architecture work and needs a strong model; the
  flash-tier workers used for spot fixes are not enough on their own.

## 11. Open questions for Alex

- Q1. Pilot gen: Gen 1 first (recommended, smaller) or Gen 3 first (more value)?
- Q2. PS version: pin 0.11.11 (what the harness uses), or move to current PS
  first?
- Q3. Keep the scripted RNG seams as a supported mode, or go RNG-exact only?
- Q4. How should the device show a PS `|tie|` (Endless Battle Clause)? Today the
  engine picks a winner by remaining HP.
- Q5. Real-server goal: one player on the web build against an online opponent,
  spectating, or both? (The device's two-players-one-screen model maps to two PS
  accounts.)
- Q6. Who does the work: Claude 2 on .167, this session directly, or a stronger
  paid model on .167? Flash-tier workers alone are not enough for phases 1-2.
- Q7. Keep `battle_types` as a UI-only crate, or fold it into `ps_dex`?
