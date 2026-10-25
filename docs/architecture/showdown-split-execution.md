# Showdown split: execution plan

Status: 2026-10-23. Executes `showdown-split-plan.md` (the design; read its
section 11 for the decisions this follows). Phases 0 to 4 plus the re-sync
pass R, as 43 tasks, each sized for one worker run (one headless session,
about 2 hours, own worktree, no commits, leaves `WORKER_REPORT.md`).

Conventions used below:
- `PS/` is `tools/showdown-parity/node_modules/pokemon-showdown/` (0.11.11).
  Workers read the TypeScript (`PS/sim/*.ts`, `PS/data/**/*.ts`); `PS/dist/` is
  what actually runs. For any effect, read the RESOLVED definition, not the
  base file: `node tools/ps-diff/show-effect.js <gen> <kind> <id>` (task P1-3
  generalises `show-ability.js` into it).
- Tier `C2` is Claude 2 (Opus). Tier `MM` is MiniMax M3 or Hyper.
- `||` marks tasks that may run in parallel with the listed ones once their
  own dependencies are met.
- "diff N" means `scripts/ps-diff.sh --gen G --n N` (built in P1-7) reports 0
  diffs, 0 crashes, 0 unported hits. "pinned" means
  `scripts/ps-diff.sh --pinned` is green. "check" means `scripts/check.sh`.
- Every ported function carries a doc comment naming its PS source
  (`// PS: sim/battle-actions.ts useMoveInner`).

## 1. How the orchestrator runs this

Branches. `ps-split` is the integration branch, cut from `main` once this plan
lands. Each run gets `w/<task-id>` in `/root/wt/<task-id>`:

    git worktree add /root/wt/<id> -b w/<id> ps-split
    cd /root/wt/<id>
    rmdir battler && ln -s /root/mega_blastoise/battler battler   # core still links battler-data
    cp -al /root/mega_blastoise/tools/showdown-parity/node_modules tools/showdown-parity/
    cp -al /root/mega_blastoise/tools/ps-diff/node_modules tools/ps-diff/   # from P1-0 on

At most three worker runs at once, each with `CARGO_BUILD_JOBS=5`. Big sweeps
(2,000 or more battles) run only when at most one worker is building, with
`JOBS=14`. `FUZZ_CACHE_DIR=/root/fuzzcache` is shared.

The brief is the task's section below, verbatim, plus a fixed preamble: the
env line (`. /root/.cargo/env; export NVM_DIR=/root/.nvm; . $NVM_DIR/nvm.sh`),
the ZERO-DRIFT rule from `CLAUDE.md`, "do not commit, do not push, never
`pkill -f`, scratch in `./.scratch/`", "do not edit files outside the task's
list without saying why in the report", "do not edit the harness, fixtures or
pinned lists unless the task says so", and the report format: what changed,
every acceptance command with its last lines of output, deviations from the
brief, open issues, and anything the next task must know.

Landing, per run:
1. Read `WORKER_REPORT.md`, then the whole diff.
2. Re-run every acceptance command yourself. Report numbers are claims.
3. Review against the checklist below.
4. Commit on `w/<id>` in the repo's message style with the attribution lines,
   rebase onto `ps-split`, run check, fast-forward `ps-split`.
5. Partial result: land the green part and re-brief the rest as `<id>b`, or
   discard. Never land red. A MM task that fails twice goes to C2.
6. Any diff seed fixed by the landing is appended to the pinned list in the
   same commit.
7. `ps-split` merges to `main` at each phase exit. The old engines stay the
   default until a gen's cut-over, so `main` stays shippable throughout.

Review checklist:
- ZERO-DRIFT: no battle UX in fw or web code; `ps_client` and the runner only
  translate; prompts, validation and screens stay in `choice_collect` /
  `oled_ctl`.
- Faithful: control flow follows the cited PS function; no "simplified"
  branches; PS's `false` / `null` / `0` / `undefined` kept apart; PRNG calls in
  PS's order and count.
- `ps_sim` hot path: no `Vec`, `String`, `Box` or `format!` (`cargo test -p
  ps_sim --test budget` is the referee); no `unsafe`; no trait objects.
- Tests: new behaviour has a test; no test, fixture or pinned entry weakened or
  deleted; the old engines untouched (phase-0 freeze) unless the task says so.
- Scope: files match the task; registry edits are additions in sorted order.

Regression sweeps:
- Every landing: check (it runs diff 200 per ported gen and the pinned lists).
- After any change to `ps_sim` core files (`battle.rs`, `queue.rs`,
  `events.rs`, `prng.rs`, `actions.rs`, a mod's `scripts.rs`): diff 2,000 per
  ported gen.
- Full sweep (phase exits, before a cut-over or deletion, after the re-sync,
  weekly while porting): diff 20,000 per ported gen with both team generators,
  the pinned lists, `scripts/pinned-seeds.sh` for the old engines while they
  exist, and `scripts/sim-budget.sh`.

## 2. Parity gates

| Gate | Gen 1 (end of phase 3) | Gen 3 (end of phase 4) |
|---|---|---|
| Protocol diff | 0 failures over 20,000 battles (10,000 random-battle teams, 10,000 wide teams), PS 0.11.11 | same, against the re-synced pin |
| Pinned | diff pinned list and old-engine corpus green | same |
| Coverage | every registered handler fires in at least 20 battles of the gate sweep; every line of `mods/gen1` runs or is allow-listed with a reason (V-1) | same, `mods/gen3` |
| Replays | every validated Gen5RNG-era replay replays clean (V-3) | same, plus current-era replays against the new pin (V-4) |
| Background sweep | no open bucket from the continuous sweep (V-2) | same |
| check | green, UI probe green with the new engine | same |
| Budget | `sim-budget.sh` within design section 6, on-board turn time under 2 ms | same, both gens linked |
| Device | Alex plays a smoke battle on the board, Draw screen seen once (forced tie seed) | same |
| Then | G1-10 deletes the Gen 1 engine | G3-9 deletes the Gen 3 engine |

Phase 1 exit: diff 2,000 per gen restricted to handler-free effects. Phase 2
exit: P2-2 and P2-4 acceptance, firmware builds with the new path compiled in.
Phase R exit: Gen 1 gate green against the new pin.

## 3. Tasks

### Phase 0: freeze and pin

**P0-1 Pinned seed corpus for the old engines** · MM (mechanical script and
test-filter work with an exact check) · depends: none · || everything in phase 1
- Touches: `crates/mega_blastoise_test/tests/showdown_fuzz.rs` (filter only),
  new `crates/mega_blastoise_test/pinned_seeds.txt`, new
  `scripts/pinned-seeds.sh`, `scripts/check.sh`.
- Read first: `showdown_fuzz.rs` (`Fuzz::new`, `fuzz_n`, `fuzz_wide`, how each
  suite draws and records choices), `/root/pv/sweep.sh`.
- Deliverable: (a) `FUZZ_CASE=k` makes a suite run every case's Rust side as
  today (so the generator stream is unchanged, including choices drawn while
  the engine plays) but send only case `k` to the reference and compare only
  it. (b) `pinned_seeds.txt`, one entry per line:
  `<suite> <wide 0|1|2> <seed> [case=<k>] [only=<ids>]  # <fix commit> <note>`;
  the orchestrator supplies the ~90 entries from `/root/pv/open_seeds.txt`
  history and the `showdown-validation` log; the worker seeds the file with
  the entries named in commits b3ec4de, ffa67f2 and 2a9a052 to prove the
  format. (c) `scripts/pinned-seeds.sh [file]` builds the `showdown_fuzz`
  binary once, runs entries with `xargs -P ${JOBS:-nproc}`, logs to
  `.scratch/pinned/`, prints failing lines, exits non-zero on any failure, and
  skips loudly (exit 0) when `node_modules` is missing. (d) check.sh runs it
  outside `--fast`.
- Accept: `scripts/pinned-seeds.sh` green on HEAD; in a scratch worktree at
  `2a9a052^`, the entries pinned for 2a9a052 fail; one entry with `case=` runs
  at least 5x faster than without; wall time for the full list on 16 cores
  recorded in the report (target under 5 minutes); check green.

The freeze itself is a landing rule, not a task: from P0-1 on, the old engines
change only for bugs a player hits, each fix pinned.

### Phase 1: protocol, dex, core

**P1-0 Workspace scaffold** · MM (exact file list, no design) · depends: none
- Touches: root `Cargo.toml`; new `crates/{ps_protocol,ps_dex,ps_sim,ps_client,ps_harness}`
  with `Cargo.toml` and an empty `lib.rs` (`#![no_std]` + `extern crate alloc`
  except `ps_harness`); new `tools/ps-diff/package.json` pinning
  `pokemon-showdown` 0.11.11 exactly; `scripts/check.sh`.
- Deliverable: the crates build; `ps_sim` has features `gen1`, `gen3`,
  `text`, `test-override`; check.sh gains
  `cargo check -p ps_sim -p ps_client -p ps_dex -p ps_protocol --target thumbv8m.main-none-eabihf`
  (the `no_std` proof).
- Accept: check green; `npm ci` in `tools/ps-diff` installs 0.11.11.

**P1-1 Protocol types** · C2 (every later crate builds on these types) ·
depends: P1-0 · || P1-2, P1-3
- Touches: `crates/ps_protocol/src/{lib,msg,command,request,ident,team}.rs`.
- Read first: `PS/sim/SIM-PROTOCOL.md`, `PS/PROTOCOL.md`, `PS/sim/battle.ts`
  (`add`, `addSplit`, `getRequests`), `PS/sim/side.ts` (`getRequestData`,
  `choose*`), `PS/sim/pokemon.ts` (`getDetails`, `getHealth`,
  `getMoveRequestData`, `getSwitchRequestData`), `PS/sim/teams.ts`
  (`pack`, `unpack`), design section 3.
- Deliverable: `Msg` (one variant per line kind in design section 3, `Split`
  carrying both HP forms, kwargs like `[from]`, `[of]`, `[still]`, `[miss]` as
  typed fields), `Command`, `Request` (PS shape, bounded arrays), `PokemonIdent`,
  `Details`, `HpStatus`, `PokemonSet` and packed-team types. Fixed capacity, no
  heap in `Msg`. A doc table mapping each PS line kind to its variant.
- Accept: `cargo test -p ps_protocol` (constructors and size tests:
  `size_of::<Msg>() <= 24` or the report says why), thumbv8m check.

**P1-2 PRNG port and the test override** · MM (bit-level port, exact fixture) ·
depends: P1-0 · || P1-1, P1-3
- Touches: `crates/ps_sim/src/prng.rs`, `crates/ps_sim/tests/prng.rs`,
  `crates/ps_sim/tests/fixtures/prng.json`, new `tools/ps-diff/prng-fixtures.js`.
- Read first: `PS/sim/prng.ts` (whole file), `PS/sim/battle.ts` (`random`,
  `randomChance`, `sample`, `speedSort`).
- Deliverable: `Prng` with `Gen5RNG` (`multiplyAdd`, `nextFrame`, `next`) and
  PS's `random()` (as `f64` = `next / 2^32`), `random(n)`, `random(m, n)`
  (`floor(result * n / 2^32)` done in `u64`), `random_chance`, `sample`,
  `shuffle(start, end)`, `get_seed`, and a call counter. The fixture script
  drives PS's own `PRNG` for 100 seeds through a fixed mixed script of 1,000
  calls (every method, edge args 1, 2, 2^16) and records results and
  `getSeed()` every 100 calls. Test-only `OutcomeOverride` (queue of forced
  next results) behind `cfg(any(test, feature = "test-override"))`.
- Accept: `cargo test -p ps_sim --test prng` matches all 100,000 calls;
  re-running the fixture script reproduces the JSON byte for byte; thumbv8m
  build of `ps_sim` without `test-override` contains no override symbol
  (`llvm-nm | grep -c Override` is 0).

**P1-3 Dex dump and tables, `battle_types` folded in** · MM (data plumbing to
a fixed schema) · depends: P1-0 · || P1-1, P1-2
- Touches: new `tools/ps-diff/dump-dex.js`, new `tools/ps-diff/show-effect.js`,
  `crates/ps_dex/{build.rs,data/gen1/*.json,data/gen3/*.json,src/*.rs}`,
  `crates/battle_types/src/lib.rs` (becomes `pub use ps_dex::Type;` plus
  whatever the old engines still call), `crates/battle_types/Cargo.toml`.
- Read first: `PS/sim/dex.ts` (`mod`, `loadData`), `PS/sim/dex-moves.ts`,
  `dex-species.ts`, `dex-items.ts`, `dex-abilities.ts`, `dex-conditions.ts`,
  `PS/data/mods/gen1/*.ts`, `PS/data/mods/gen3/*.ts`,
  `tools/showdown-parity/vendor-species.js` (the existing pattern),
  `crates/battle_types/src/lib.rs`.
- Deliverable: dumps of the RESOLVED `Dex.mod('gen1')` / `Dex.mod('gen3')`:
  species (stats, types, abilities, gender ratio), moves (every data field the
  sim reads, flags, secondaries, and the list of handler names it has), items,
  abilities, typechart, conditions (handler names, priorities and orders as
  data). `build.rs` emits `&'static` tables with `u16` ids in PS's iteration
  order, `MoveId`/`SpeciesId`/... newtypes, `ps_dex::Type` with the same
  variants and `name()`/`abbr()` as today. `show-effect.js <gen> <kind> <id>`
  prints the resolved definition with handler source.
- Accept: `cargo test -p ps_dex` (every gen 1 and gen 3 move, species, item and
  ability round-trips against the JSON; counts equal design section 6); the
  workspace still builds with `battle_types` as a shim; `.rodata` of both
  dexes reported from a thumbv8m build (target under 60 KB).

**P1-4 Text codec with round trips against real logs** · MM (codec against a
corpus, binary pass/fail) · depends: P1-1 · || P1-5, P2-1
- Touches: `crates/ps_protocol/src/text/*.rs` (feature `text`), new
  `tools/ps-diff/record-logs.js`, `crates/ps_protocol/tests/{roundtrip.rs,fixtures/}`.
- Read first: `PS/sim/battle-stream.ts`, `PS/sim/SIM-PROTOCOL.md`,
  `PS/sim/teams.ts`, the P1-1 types.
- Deliverable: line encoder and decoder for every `Msg`, request JSON
  encoder/decoder (hand-written, bounded, PS key order), command parser and
  printer, packed-team codec. `record-logs.js` plays 60 gen 1 and 60 gen 3
  `BattleStream` battles (random-battle teams, random legal choices, fixed
  seeds) and saves the raw output; commit 20 of each (gzip if over 1 MB total).
- Accept: `cargo test -p ps_protocol --features text`: every recorded line
  decodes and re-encodes byte for byte (unknown kinds fail the test), every
  request JSON re-encodes byte for byte, every team packs back to the same
  string; the recorder regenerates identical logs for the same seeds.

**P1-5 Event core: state, effects, dispatch, `speedSort`** · C2 (the
architecture every handler depends on) · depends: P1-1, P1-2, P1-3 · || P1-4, P1-7
- Touches: `crates/ps_sim/src/{state,effect,events,relay,modify}.rs`,
  `tests/speed_sort.rs`, new `tools/ps-diff/speedsort-fixtures.js`.
- Read first: `PS/sim/battle.ts` (`comparePriority`, `speedSort`,
  `eachEvent`, `fieldEvent`, `singleEvent`, `runEvent`, `priorityEvent`,
  `resolvePriority`, `findEventHandlers`, `findPokemonEventHandlers`,
  `findSideEventHandlers`, `findFieldEventHandlers`, `findBattleEventHandlers`,
  `initEffectState`, `chainModify`, `modify`, `finalModify`), `PS/sim/pokemon.ts`
  (`addVolatile`, `removeVolatile`, `setStatus`, `getStatus`, state fields),
  `PS/sim/side.ts`, `PS/sim/field.ts`, `PS/sim/dex-conditions.ts`, design 5.1
  and 5.3.
- Deliverable: `Battle`, `Side`, `Pokemon`, `Field`, `EffectTable<N>`,
  `EffectState`, `EventId`, `Handler`, `EffectDef`, `EventMask`, the relay enum,
  the dispatch family with PS's prefixes (`onSource`, `onFoe`, `onAlly`,
  `onAny`), `effectOrder` assignment, and the effect registry
  (`EffectId -> &'static EffectDef`, one file per effect under
  `mods/<gen>/{moves,conditions,items,abilities,rules}/`, a sorted registry
  list per gen). The handler signature is frozen at phase-1 exit.
  Fixtures: the script builds a PS `Battle` with a fixed seed and calls its
  `speedSort` on 500 synthetic lists (2 to 12 entries, small value ranges to
  force ties, missing `order` and `order: 0`, which PS treats alike via
  `order || 2^32`), recording the permutation and `prng.getSeed()` after.
- Accept: `cargo test -p ps_sim --test speed_sort` reproduces all 500
  permutations and post-seeds; unit tests for insertion-order iteration,
  `effectOrder` ties and each prefix; `size_of::<Battle>() <= 16384` asserted;
  thumbv8m check.

**P1-6 Queue, turn loop, switching, faints, requests, end of battle** · C2
(control flow PS gets right only as a whole) · depends: P1-5, P1-4
- Touches: `crates/ps_sim/src/{queue,battle,choice,request,emit}.rs`.
- Read first: `PS/sim/battle-queue.ts` (whole), `PS/sim/battle.ts`
  (`start`, `setPlayer`, `getTeam`, `turnLoop`, `runAction`, `endTurn`,
  `nextTurn`, `checkFainted`, `faintMessages`, `checkWin`, `win`, `tie`,
  `maybeTriggerEndlessBattleClause`, `makeRequest`, `getRequests`, `choose`,
  `commitChoices`, `undoChoice`, `switchIn`, `dragIn`, `getRandomTarget`),
  `PS/sim/side.ts` (`choose`, `chooseMove`, `chooseSwitch`, `isChoiceDone`),
  `PS/sim/pokemon.ts` constructor (the gender `sample`),
  `PS/data/mods/gen4/scripts.ts` `runSwitch`, `PS/data/rulesets.ts` (HP
  Percentage Mod, Cancel Mod, Desync / Sleep / Freeze Clause Mod), design 5.2.
- Deliverable: the server from `>start` to `|win|` / `|tie|` with a stub move
  pipeline (moves fail with a `todo` panic the CLI turns into "unported"); all
  header lines; the three tie paths of design section 3; format rules with
  handlers ported; `Cartridge Trunc` as a rule.
- Accept: unit tests: switch-only battles and forced-switch requests match the
  recorded P1-4 fixtures message for message up to the first move; tie tests
  for each tie path (turn 1001, Gen 1 no-progress, double KO through the test
  override); request JSON for every fixture request state matches byte for byte.

**P1-7 Protocol-diff harness** · MM (tooling with a crisp self-test) ·
depends: P1-4 · || P1-5, P1-6
- Touches: new `tools/ps-diff/{diff.js,teams.js,README.md}`,
  `crates/ps_harness/src/bin/ps_sim_cli.rs`, `crates/ps_harness/tests/protocol_diff.rs`,
  new `scripts/ps-diff.sh`, new `crates/ps_harness/pinned.txt`, `scripts/check.sh`.
- Read first: `PS/sim/battle-stream.ts`, `PS/sim/teams.ts`
  (`Teams.generate`, `getGenerator`), `PS/data/random-battles/gen1/` and
  `gen3/`, `tools/showdown-parity/harness.js` (`formatFor`, `cartridgeTrunc`),
  `crates/mega_blastoise_test/tests/showdown_fuzz.rs` (the wide
  generator's space).
- Deliverable: `diff.js` derives a battle seed (four u16), a team seed and a
  choice seed from one driver seed; builds teams with `--teams random` (PS's
  `genNrandombattle` generator) or `--teams wide` (the fuzz space, restricted to
  the ids `ps_sim_cli --ported <gen>` prints) or `--teams file:<path>`; plays PS
  `BattleStream` (restoring `cartridgeTrunc` for Gen 1, the only PS-side
  adaptation) and `ps_sim_cli` (long-lived, many battles per process) on the
  same commands; picks random legal choices from PS's request; compares every
  output line and request, `|t:|` normalised; on the first divergence writes
  both logs, the seed and both PRNG call counts to `.scratch/ps-diff/`. Battles
  end at `|win|`/`|tie|` or turn 300. `--focus <ids>` makes every team carry at
  least one listed effect. `--pinned` runs `pinned.txt`
  (`<gen> <teams> <seed>  # <commit> <note>`). Exit non-zero on any diff, crash
  or unported hit; one summary line per run. check.sh runs diff 200 per gen
  that has a ported pipeline, plus `--pinned`.
- Accept: PS against itself (a `--self` mode: PS on both sides) gives 0 diffs
  over 2,000 battles per gen; with one side's seed changed the first divergence
  is reported with context; a hand-edited log line is caught; 2,000 battles on
  14 jobs finish in under 10 minutes (report the time).

**P1-8 Gen 1 move pipeline** · C2 (843 lines of order-sensitive overrides) ·
depends: P1-6, P1-7 · || P1-9, P2-1
- Touches: `crates/ps_sim/src/mods/gen1/scripts.rs`, `src/actions.rs` (the
  `GenScripts` trait and enum dispatch), `src/stats.rs`.
- Read first: `PS/data/mods/gen1/scripts.ts` (whole), `PS/sim/battle-actions.ts`
  (what gen 1 does not override), `PS/data/mods/gen1/moves.ts` header,
  `PS/data/mods/gen1/conditions.ts` (only to know which effects to leave
  unported), design 5.4.
- Deliverable: `runMove`, `useMove`, `useMoveInner`, `tryMoveHit`, `moveHit`,
  `getDamage`, `deductPP`, `getStat`, `modifyStat`, `boostBy`, `clearBoosts`,
  `boost`, plus inherited `runSwitch` and `getActionSpeed`; data-driven effects
  (accuracy, crits, secondaries, multi-hit, recoil, drain, OHKO, fixed damage,
  status and boost moves without handlers). `ps_sim_cli --ported gen1` lists
  handler-free moves.
- Accept: diff 2,000 (gen 1, both team modes); `cargo test -p ps_sim`.

**P1-9 Gen 3 move pipeline** · C2 (gen 3 / gen 4 / base layering) ·
depends: P1-6, P1-7 · || P1-8
- Touches: `crates/ps_sim/src/mods/gen3/scripts.rs`, `src/actions.rs`,
  `src/stats.rs` (natures, IV/EV).
- Read first: `PS/data/mods/gen3/scripts.ts`, `PS/data/mods/gen4/scripts.ts`,
  `PS/sim/battle-actions.ts` (`useMove`, `useMoveInner`, `trySpreadMoveHit`,
  `hitStep*`, `spreadMoveHit`, `moveHit`, `getDamage`, `modifyDamage`,
  `getSpreadDamage`, `runMoveEffects`, `secondaries`, `selfDrops`), `PS/sim/battle.ts`
  `endTurn` (the Quick Claw roll every turn).
- Deliverable: `Gen3Scripts` for handler-free moves, items none, abilities
  restricted to handler-free ones (the CLI's ported list).
- Accept: diff 2,000 (gen 3, wide teams; random-battle teams only if the
  ported list covers them).

### Phase 2: client, runner, screen, budget

**P2-1 `ps_client`: view and request translation** · C2 (must reproduce what
both old runners narrate, ZERO-DRIFT) · depends: P1-4 · || P1-5 to P1-9
- Touches: `crates/ps_client/src/{view,request,board}.rs`, tests.
- Read first: `crates/mega_blastoise_core/src/runner/{gen1,gen3}.rs` (which
  `BoardEvent`s they emit, in what order), `board_event.rs`,
  `choice_collect.rs` (`SlotOptions`), `PS/sim/SIM-PROTOCOL.md`, design 7.
- Deliverable: `BattleView::apply(&Msg) -> BoardEvents`, `slot_options(&Request)`,
  `choice_string(...)`. No UI decisions.
- Accept: on every P1-4 fixture log the view ends with the HP, status and
  active mon PS's final request reports; golden tests of the `BoardEvent`
  stream for 3 recorded battles per gen (reviewed by the orchestrator against
  what the old runner emits for the same events).

**P2-2 Generic runner, transports, engine switch** · C2 (core seam touching
device and web wiring) · depends: P2-1, P1-8
- Touches: `crates/mega_blastoise_core/src/runner/{mod,ps}.rs`, new
  `src/transport.rs`, `device_runner.rs` (`run_gen1`/`run_gen3` choose the
  path), `randbat.rs` (teams as `PokemonSet`), `crates/mega_blastoise_web/src/lib.rs`
  (query param `engine=ps` only), `crates/mega_blastoise_fw/Cargo.toml`
  (feature `ps_engine`), `scripts/ui-probe.sh` (`--engine ps`).
- Deliverable: `Transport` trait, `LocalTransport` (typed, in-process),
  `TextTransport` (any byte stream); one `run_battle` for both gens behind the
  existing entry points; `Engine::{Legacy, Ps}` chosen at runtime, default
  `Legacy`.
- Accept: `scripts/ui-probe.sh --engine ps` green for gen 1; check green; a
  host test runs 50 seeded gen 1 battles to the end over `LocalTransport` and
  the same 50 over `TextTransport` with identical `BoardEvent` streams;
  `git grep` shows no new battle logic in fw or web.

**P2-3 Draw end screen** · MM (one screen, existing patterns, unit-testable) ·
depends: none · || anything
- Touches: `crates/mega_blastoise_core/src/{oled_ctl,board_event,device_ui,display_color}.rs`,
  `crates/gen1_battle/src/battle.rs` (`endless_battle_winner` becomes a tie, a
  PS-parity bug fix allowed under the freeze),
  `crates/mega_blastoise_test/src/host_battle_effects.rs`.
- Read first: `oled_ctl.rs` (`OledCmd::Win`, `View::Win`, `screen()`),
  `display_color.rs` `render_result`, `PS/sim/battle.ts`
  `maybeTriggerEndlessBattleClause`, design 7.
- Deliverable: `BoardEvent::Tie` drives a distinct `View::Draw` /
  `Screen::Draw` for both seats (title DRAW, both last mons' sprites, the same
  legend as the result screen), decided in `oled_ctl` only. Gen 1's old engine
  emits a tie where PS does; no HP tiebreak remains.
- Accept: `oled_ctl` unit tests (Tie gives Draw on both seats, Win unchanged);
  a gen 1 engine test for the no-progress tie; the gen 1 fuzz suites and
  `pinned-seeds.sh` still green; check green; a host-rendered PNG of the
  screen attached to the report for the orchestrator to show Alex.

**P2-4 Budget measurement** · MM (instrumentation to an exact recipe) ·
depends: P1-8 · || P2-2
- Touches: new `crates/mega_blastoise_fw/src/bin/sim_bench.rs`, new
  `crates/ps_sim/tests/budget.rs`, new `scripts/sim-budget.sh`, new
  `docs/architecture/showdown-split-budget.md`.
- Read first: `crates/mega_blastoise_fw/src/mem_profile.rs`,
  `scripts/fw-build.sh`, `.claude/commands/fwdebug.md`, design 6.
- Method: (1) host: `size_of` of `Battle`, `Side`, `Pokemon`, `EffectState`
  asserted against the budget; a counting global allocator proves zero
  allocations inside a turn over 500 battles. (2) flash: build `sim_bench`
  release for `thumbv8m.main-none-eabihf`, sum `llvm-nm --print-size` sizes of
  `ps_sim`, `ps_dex`, `ps_protocol`, `ps_client` symbols, and the `.text` /
  `.rodata` delta against the same bench with the sim compiled out. (3) on the
  board: the bench paints 32 KB below the stack pointer, runs 200 seeded
  battles per ported gen with random choices, and reports over RTT the DWT
  cycle count per turn (p50, p99, max, at 150 MHz), heap `used()` before and
  after each turn (must be equal) and the stack high-water mark.
  `sim-budget.sh` does (1) and (2) and prints a table; the orchestrator runs (3)
  with `probe-rs run --preset pico2 <sim_bench elf>` on the machine with the
  probe and records it in `showdown-split-budget.md`.
- Accept: `cargo test -p ps_sim --test budget`; `scripts/sim-budget.sh` prints
  the table; `cargo build --release --bin sim_bench` succeeds for thumbv8m.

### Phase V: verification beyond the random sweep

The protocol diff with random choices proves parity statistically (0 in 20,000
bounds the per-battle mismatch rate near 1 in 6,700), but uniform random
choices rarely play the lines strategic players do, and a rare handler can go
untested. These tasks close that gap. They run alongside the Gen 1 batches and
feed the phase gates in section 2.

**V-1 Coverage gate** · MM · depends: P1-8 · || G1-1..G1-6
- Touches: `crates/ps_sim` (a `hit-count` cargo feature, off by default and in
  the firmware), `crates/ps_harness/src/bin/ps_sim_cli.rs` (`--hits FILE`),
  `tools/ps-diff/diff.js` (only to forward the flag), new
  `scripts/ps-coverage.sh`, new `crates/ps_harness/coverage-allow.txt`.
- Deliverable: (1) per-handler hit counts: under `hit-count` the dispatcher
  counts each (effect id, event) handler call and the number of distinct
  battles it fired in; `ps_sim_cli` writes them at exit and the script sums a
  sweep. (2) line coverage: `cargo llvm-cov` over `ps_sim_cli` for the same
  sweep, reported for `crates/ps_sim/src/mods/genN/**` and the core files,
  listing lines never run. (3) the gate: every handler registered for the gen
  fires in at least 20 battles of the 20,000 sweep, and every line of
  `mods/genN` runs or is listed in `coverage-allow.txt` with a reason (dead
  under the format, unreachable in PS too). Under-hit handlers print as a
  `--focus` id list, so the gate task can aim teams at them.
- Accept: the script prints both tables for the current Gen 1 port;
  `cargo size` of the firmware is unchanged with the feature off; check.sh is
  unchanged (coverage runs at phase gates, not per commit).

**V-2 Continuous sweep** · MM · depends: P1-7, P1-8
- Touches: new `scripts/ps-sweep.sh`, a section in `tools/ps-diff/README.md`.
- Deliverable: a loop for tmux on the worker machine. It builds a private copy
  of `ps_sim_cli` from the tip of `ps-split` (rebuilding when the tip moves),
  runs batches of 2,000 per gen and team mode with fresh seeds (a counter
  file, never reused across restarts) under `nice -n 19` with `--jobs 8`, and
  writes failures to `sweep/<date>/<bucket>/`, bucketed by first differing
  line like G1-7, each with its seed and the command that reproduces it. A
  one-line status file holds battles run, diffs, crashes, unported counts by
  id, and buckets new since the last read.
- Accept: an hour unattended with no leaked processes and capped disk use;
  seeds never repeat across a restart; a planted bug (`--tamper-line`) shows
  up as a bucket.
- The orchestrator keeps it running from the first Gen 1 gate sweep on and
  reads the status after each landing; new buckets feed G1-7b/c and G3-7b/c.

**V-3 Replay corpus, Gen5RNG era** · MM · depends: P1-7, P1-4c · || G1 batches
- Background: `replay.pokemonshowdown.com` serves public replays as JSON
  (`/search.json?format=<id>&before=<uploadtime>`, `/<id>.json`). Each has an
  `inputlog`: `>version <commit>`, `>start` with the battle seed, `>player`
  with a team seed (random battles) or a packed team (OU and the like), then
  every choice the players made. Replays from before about late 2024 have
  numeric seeds, which select Gen5RNG (0.11.11 and `ps_sim`); later ones use
  `sodium,<hex>` (V-4).
- Touches: new `tools/ps-diff/replays.js`, `tools/ps-diff/diff.js`
  (`--replays DIR`), new `crates/ps_harness/replays/` (a small committed
  sample), an untracked cache on the worker machine, a `ps_client` test.
- Deliverable: (1) a polite fetcher: one request per second, an on-disk cache,
  resumable, never re-downloads; formats gen1randombattle, gen1ou,
  gen3randombattle, gen3ou; numeric seeds only. (2) validation: run each
  inputlog through PS 0.11.11 and keep a replay only when PS's spectator
  output matches the replay's `log` (ignoring the lines the server adds:
  chat, `|j|`, `|l|`, `|t:|`, `|inactive|`, `|badge|`, `|uhtml|`, `|raw|`);
  record kept and dropped counts and the main drop reasons (version drift,
  team generator changes). (3) `diff.js --replays DIR`: PS 0.11.11 against
  `ps_sim` with the choices from the inputlog instead of random ones; `ps_sim`
  gets the team PS generated (packed), since it does not run PS's team
  generator; a choice `ps_sim` rejects is a diff. (4) `ps_client` parses every
  line of every cached `log`, server-only lines included, without an error.
- Accept: at least 1,000 validated replays per gen (or all that exist); the
  replay diff runs over the Gen 1 set (stopping at unported effects allowed
  until G1-7); the `ps_client` test passes.

**V-4 Current-era replays** · MM · depends: R-2a, V-3
- After R-2a ports SodiumRNG, V-3's pipeline also takes `sodium,` seeds,
  validated against the re-synced pin: the bulk of the public corpus, recent
  human battles in every format. Accept: as V-3, both gens, against the new
  pin; the Gen 3 set gates G3-7.

### Phase 3: Gen 1 pilot

Handler batches. Each: port the listed effects (one file per effect, PS source
cited), register them, and pass diff 2,000 with `--focus <batch ids>` plus
pinned and check. Read first, for every batch: the resolved definitions via
`show-effect.js gen1 ...`, `PS/data/mods/gen1/moves.ts`,
`PS/data/mods/gen1/conditions.ts`, `PS/data/mods/gen1/scripts.ts` (where the
pipeline special-cases the ids), and `crates/gen1_battle/SPEC.md` for the
cartridge notes.

**G1-1 Status, confusion, flinch, clauses** · MM · depends: P1-8 · || G1-2..G1-6
- `brn par slp frz psn tox residualdmg confusion flinch`; rules Sleep Clause
  Mod, Freeze Clause Mod, Desync Clause Mod; moves `toxic rest dreameater`.

**G1-2 Trapping, recharge, locks, Rage, Bide** · C2 (gen 1's most entangled
volatiles) · depends: P1-8
- `bind wrap clamp firespin partiallytrapped partialtrappinglock
  fakepartiallytrapped trapped mustrecharge lockedmove rage bide` (Thrash and
  Petal Dance through `lockedmove`).

**G1-3 Two-turn moves** · MM · depends: P1-8
- `twoturnmove invulnerability dig fly razorwind skullbash skyattack solarbeam`.

**G1-4 Screens and stat volatiles** · MM · depends: P1-8
- `substitute reflect lightscreen mist haze focusenergy leechseed disable`.

**G1-5 Simple handlers** · MM · depends: P1-8
- `superfang psywave highjumpkick jumpkick recover softboiled payday thunder
  conversion splash minimize defensecurl`.

**G1-6 Copy and call moves, Counter** · C2 (move-register state and call
order) · depends: P1-8
- `transform mimic metronome mirrormove counter`.

**G1-7 Gen 1 gate sweep and fixes** · C2 (cross-effect debugging) ·
depends: G1-1..G1-6, P2-2 · repeatable as G1-7b, G1-7c
- Deliverable: run the full Gen 1 sweep (orchestrator starts it; the worker
  gets the failure directory), bucket divergences by first differing line, fix
  the buckets in priority order, pin each fixed seed.
- Accept: the fixed seeds pass; diff 2,000 on fresh seeds green; the report
  lists remaining buckets with counts. Phase gate when a fresh 20,000 is clean,
  V-1's coverage gate passes, the V-3 Gen 1 replays are clean and V-2 has no
  open bucket.

**G1-8 Old corpus carried over** · MM (plumbing between two harnesses) ·
depends: P0-1, P1-7 · || G1-1..G1-6
- Touches: `showdown_fuzz.rs` (`FUZZ_DUMP=<path>` writes each case's two teams
  as PS sets JSON), `tools/ps-diff/diff.js` (`--teams file:` already exists).
- Deliverable: every gen 1 battle-suite entry in `pinned_seeds.txt` dumped to
  `crates/ps_harness/carried/gen1/*.json`; `ps-diff.sh --carried gen1` runs each
  team pair over 20 battle seeds.
- Accept: dump is deterministic; the carried run is part of the Gen 1 gate
  (green after G1-7).

**G1-9 Cut-over: core off `gen1_battle`** · C2 (wide refactor across UI data
paths) · depends: G1-7 gate green
- Touches: `crates/mega_blastoise_core/src/{display,display_color,move_anim,cli_parse,randbat,demo_teams,prompt_fmt,random_ai,device_session,choice_collect,board_event,data_store,device_runner,lib}.rs`,
  `crates/mega_blastoise_test/src/*` and tests that use gen 1 types, web
  `lib.rs` if needed.
- Deliverable: everything imports `ps_dex` / `ps_protocol` types; gen 1 runs on
  `Engine::Ps` by default; `DataStore` from `battler-data` replaced by `ps_dex`.
- Accept: check green; `scripts/ui-probe.sh` (default engine) green;
  `cargo tree -p mega-blastoise-core | grep -E 'gen1_battle|battler'` empty;
  firmware release build size reported.

**G1-10 Delete the Gen 1 engine** · MM (mechanical deletion against a
checklist) · depends: G1-9, Alex's device smoke battle
- Deletes: `crates/gen1_battle`, `runner/gen1.rs`, the `battler` /
  `battler-data` deps in core, fw, test and web, the `fuzz_gen1_*` suites and
  gen 1 scenarios in `showdown_parity.rs`, gen 1 entries in `pinned_seeds.txt`,
  the `battler` submodule (if Alex confirms at the gate).
- Accept: `git grep -n 'gen1_battle\|battler_data' -- crates scripts tools`
  empty; check green; the Gen 1 diff gate re-run green (`--n 2000` plus pinned
  and carried); `sim-budget.sh` still within budget.

### Phase R: re-sync to current Showdown

**R-1 Re-sync survey** · C2 (judging which upstream changes matter) ·
depends: G1-10
- Touches: `tools/ps-diff/package.json` (new pin, on the branch only),
  new `docs/architecture/showdown-resync.md`.
- Read first: upstream changelog and the source diff 0.11.11 to the new release
  for `sim/*.ts`, `data/mods/gen{1,2,3,4}/*`, and base `data/{moves,items,abilities,conditions,rulesets}.ts`
  entries that gen 1 or gen 3 resolve to.
- Deliverable: a change list, each entry tagged core / gen 1 / gen 3 / protocol
  / PRNG with the PS commit and the `ps_sim` file it hits; confirmation that
  numeric seeds still select `Gen5RNG` (if not, a plan for `SodiumRNG`); a diff
  100 run of the current Gen 1 server against the new pin. Public replays since
  about late 2024 seed with `sodium,<hex>`, so R-2a ports `SodiumRNG` either
  way (V-4 needs it).
- Accept: the doc exists and covers every changed file in the listed paths
  (the worker includes the `git diff --stat`); the orchestrator splits R-2.

**R-2a Core and protocol changes** · C2 · depends: R-1 · || R-2b
**R-2b Gen 1 data and handler changes, dex re-dump** · MM · depends: R-1
- Accept (both, then together): diff 20,000 for gen 1 against the new pin, the
  pinned and carried lists, check. The old Gen 3 suites keep the 0.11.11 pin
  in `tools/showdown-parity`.

### Phase 4: Gen 3

Every Gen 3 port reads the re-synced pin. Handler batches follow the phase 3
recipe (diff 2,000 `--gen 3 --focus <ids>`, pinned, check), reading
`show-effect.js gen3 ...`, `PS/data/mods/gen3/*.ts`, `PS/data/mods/gen4/*.ts`
and the base `PS/data/*.ts` entries they inherit.

**G3-0 Gen 3 infrastructure** · C2 · depends: R-2a
- Items and abilities machinery (`setItem`, `takeItem`, `eatItem`, `useItem`,
  `setAbility`, `ignoringAbility`, item and ability state), weather as a field
  effect, the switch-in ability order, Pursuit's switch hook, `Update` points,
  Hidden Power, Struggle. Read `PS/sim/pokemon.ts`, `PS/sim/field.ts`,
  `PS/sim/battle-actions.ts` (`switchIn`, `runSwitch`), `PS/data/mods/gen4/scripts.ts`.
- Accept: diff 2,000 (gen 3, handler-free effects, both team modes) still green
  after the refactor; unit tests for each new primitive.

**G3-1 Gen 3 conditions** · C2 (residual and weather ordering) · depends: G3-0
- `slp frz brn par psn tox confusion flinch trapped partiallytrapped lockedmove
  choicelock futuremove stall twoturnmove mustrecharge raindance sunnyday
  sandstorm hail`, with the moves that only start them.

**G3-M1 Power and damage callbacks** · MM · depends: G3-1 · || all G3 batches
- `eruption waterspout flail reversal return frustration lowkick magnitude
  present facade revenge smellingsalts superfang psywave endeavor falseswipe
  triplekick hiddenpower weatherball highjumpkick jumpkick thunder triattack
  secretpower brickbreak payday`.

**G3-M2 Protection, side conditions, healing** · MM · depends: G3-1
- `protect detect endure reflect lightscreen safeguard mist spikes rapidspin
  mudsport watersport wish ingrain haze aromatherapy healbell moonlight
  morningsun synthesis rest refresh bellydrum defensecurl minimize focusenergy
  splash painsplit`.

**G3-M3 Volatile control** · C2 (the `BeforeMove` ordering cluster that
motivated the rewrite) · depends: G3-1
- `disable encore taunt torment imprison attract yawn perishsong destinybond
  grudge curse nightmare leechseed substitute foresight odorsleuth lockon
  mindreader block meanlook spiderweb helpinghand followme`.

**G3-M4 Multi-turn and delayed** · C2 · depends: G3-1
- `bide uproar rollout iceball furycutter rage charge focuspunch dig dive fly
  bounce skullbash skyattack razorwind solarbeam futuresight doomdesire
  stockpile spitup swallow pursuit beatup`.

**G3-M5 Call and copy** · C2 · depends: G3-1
- `assist metronome mirrormove sleeptalk snore naturepower mimic sketch
  transform conversion conversion2 psychup roleplay skillswap camouflage
  magiccoat snatch`.

**G3-M6 Item moves, counters, the rest** · MM · depends: G3-1, G3-I2
- `trick knockoff thief covet recycle batonpass counter mirrorcoat spite
  fakeout dreameater toxic`.

**G3-I1 Held items** · MM · depends: G3-1
- type-boost items, `souldew lightball thickclub deepseatooth deepseascale
  metalpowder stick luckypunch scopelens choiceband brightpowder laxincense
  kingsrock focusband machobrace shellbell leftovers mail`.

**G3-I2 Berries and herbs** · MM · depends: G3-1
- every `*berry` with a handler, `whiteherb mentalherb`.

**G3-A1 Modifier and contact abilities** · MM · depends: G3-1
- `blaze overgrow torrent swarm hugepower purepower hustle guts marvelscale
  thickfat swiftswim chlorophyll compoundeyes sandveil keeneye hypercutter
  clearbody whitesmoke rockhead serenegrace shielddust sturdy suctioncups
  stickyhold static flamebody poisonpoint effectspore cutecharm roughskin
  liquidooze speedboost shedskin naturalcure raindish`.

**G3-A2 Switch-in, weather, immunity and trapping abilities** · C2 (order of
switch-in triggers) · depends: G3-1
- `intimidate trace drizzle drought sandstream airlock cloudnine forecast
  flashfire voltabsorb waterabsorb lightningrod wonderguard soundproof damp
  truant pressure shadowtag arenatrap magnetpull insomnia vitalspirit immunity
  limber magmaarmor waterveil oblivious owntempo innerfocus colorchange
  synchronize plus minus`.

**G3-7 Gen 3 gate sweep and fixes** · C2 · depends: all G3 batches ·
repeatable. Same recipe as G1-7, plus G1-8's carried corpus for gen 3.

**G3-8 Cut-over: core off `gen3_battle`** · C2 · depends: G3-7 gate green
- Same shape as G1-9 for `runner/gen3.rs` users (`drafted_battle`, the draft
  tables, `display.rs` types). Accept as G1-9 with `gen3_battle`.

**G3-9 Delete the Gen 3 engine and `battle_types`** · MM · depends: G3-8,
Alex's device smoke battle
- Deletes `crates/gen3_battle`, `crates/battle_types`, `runner/gen3.rs`,
  `showdown_fuzz.rs`, `showdown_parity.rs`, `tools/showdown-parity` (its
  scripts move to `tools/ps-diff` if still used), `pinned_seeds.txt` and
  `pinned-seeds.sh`. Accept as G1-10, both gens.

### Phase N: online play through a relay (after G3-9)

Two browsers play each other over the internet. Friends open the Pages link
and type a room code; nothing to install. The relay is a small server that
Alex hosts at a public `wss://` address (Pages is https, so plain `ws://`
is blocked). Showdown's own servers, login and lobby are not involved: the
relay runs `ps_sim` and speaks the same protocol `ps_client` already reads.

**N-1 Relay server** · MM · depends: G3-9
- New `crates/ps_relay` (std, tokio or a plain WebSocket crate, not in the
  firmware build). One room per code, two seats, spectators optional. Per
  room it owns one `ps_sim` `Battle`, takes `Command`s from each seat, and
  sends each seat only its own `|request|` and the side-split protocol (PS's
  per-side `sideupdate` and the spectator stream). Room reaper, a size cap,
  a rate limit, no accounts.
- Accept: two scripted clients play a full battle in both gens; a client
  cannot see the other seat's request; a disconnected seat can rejoin with
  its room code and seat token.
- Time-boxed by default: the relay exits on its own 60 minutes after it
  starts (`--max-uptime`, default 3600 s; rooms get a 2 minute warning line
  first). It is started on demand, never restarted by a supervisor, and
  refuses new rooms during the last 5 minutes. Tested with a short uptime.

**N-2 WebSocket transport** · C2 · depends: N-1
- A `Transport` impl in the web crate over the browser WebSocket (raw IO
  only). Reconnect with backoff, rejoin with the seat token. A `?relay=`
  override for a local relay.
- Accept: a headless test drives a battle against a local relay.

**N-3 Single-seat mode in core** · C2 · depends: G3-8
- The UI assumes two seats at one device. Online there is one local seat and
  a hidden opponent. All of it lives in `oled_ctl` and `choice_collect`
  (zero-drift rule); the web build shows the one seat at full size.
  Opponent-thinking and disconnected states, a forfeit control, rematch.
- Accept: the headless seat probe plays a battle in single-seat mode in both
  gens with the same screens the two-seat mode shows for that seat.

**N-4 Lobby screen** · MM · depends: N-2, N-3
- Create room (shows the code and a share link), join by code, pick gen and
  format (random battle both gens, truncation on for Gen 1), team preview if
  the format has it.
- Accept: two headless browsers meet through a local relay and finish a
  battle.

**N-5 Home deploy and soak** · MM · depends: N-4
- Deploy on `box` (10.67.1.101, always on, Caddy on 80/443 with a wildcard
  certificate for `*.hubshouse.duckdns.org`): a Dockerfile and a compose
  service for the relay on a local port, and a Caddy route for
  `relay.hubshouse.duckdns.org` proxying WebSocket to it. Never touch the
  bare `hubshouse.duckdns.org` name (the WireGuard endpoint). The router's
  ddns service already keeps DuckDNS current.
- Alex's side, by hand or through the infra session after he approves: a
  router forward of TCP 8443 to box:8443 (a dedicated port, decided
  2026-10-25, so no other Caddy name such as qbittorrent.* is exposed), a
  Caddy site `relay.hubshouse.duckdns.org:8443` and the docker service under
  `/srv/infra` (needs `claude-admin`). The worker writes the exact request,
  never edits the router or Caddy itself.
- On demand and time-boxed: Alex (or one command) starts the container, it
  stops itself after an hour (`restart: no`), and the Caddy route and router
  forward stay in place but answer 502 while it is down. Docker sandbox:
  non-root, read-only filesystem, no capabilities, its own network with no
  route to the LAN, size, rate and per-IP limits (N-1).
- A soak run of 200 scripted battles with random disconnects.
- Accept: no leaked rooms or tasks after the soak; a client on another
  network plays through the public name; Alex plays one real battle with a
  friend.

Open choice for Alex: whether rooms are public by code only (default) or
need a passphrase.

## 4. Order and critical path

Counts: phase 0 one task (MM); phase 1 ten (5 C2, 5 MM); phase 2 four (2 C2,
2 MM); phase 3 ten (4 C2, 6 MM); phase R three (2 C2, 1 MM); phase 4 fifteen
(8 C2, 7 MM); phase V four (4 MM). In all 47: 21 C2, 26 MM, plus repeat runs
of the two gate tasks. Phase V is off the critical path, but the phase gates
wait for it.

Critical path: P1-0, P1-1, P1-5, P1-6, P1-8, G1-2 (the longest Gen 1 batch),
G1-7, G1-9, G1-10, R-1, R-2a, G3-0, G3-1, G3-M3, G3-7, G3-8, G3-9. Off the
path and parallel: P0-1, P1-2, P1-3, P1-4, P1-7, P2-1 (from P1-4 on), P2-3
(any time), P2-2 and P2-4 (alongside the Gen 1 batches), G1-8, R-2b, and all
other batches. With three concurrent runs the path is about 17 runs deep.

## 5. Risks

| Risk | Mitigation |
|---|---|
| One stray PRNG call desyncs the rest of a battle, so early diffs are noisy | First-divergence reporting with both PRNG call counts; `PS_RNG_TRACE=1` logs every call site on both sides (PS side wraps `prng.next` to record, never to change results) |
| The handler signature or dispatch is wrong and only shows after many batches | P1-8 and P1-9 exercise the core on two pipelines before any batch; the signature freezes at phase-1 exit and later changes are C2 tasks followed by a 2,000 sweep per gen |
| A MM worker ships a plausible but wrong port, or loosens a check | The orchestrator re-runs every acceptance command; batches may not touch the harness, fixtures or pinned lists; two failures move the task to C2 |
| Budget overrun (stack depth, flash, turn time) | Measured in phase 2 before the bulk port; `budget.rs` in check.sh keeps RAM honest; fallbacks: smaller caps, `#[inline(never)]` on handlers, colder data in tables |
| Parallel batches collide in registry files | One file per effect; registries are sorted lists; the orchestrator resolves conflicts at landing |
| The re-sync is larger than expected (protocol or PRNG change upstream) | R-1 sizes it first; anything beyond a few days goes to Alex before R-2 |
| Faithful port reproduces PS bugs | Intended (decision 5); each known one is noted with the PS line in `docs/architecture/ps-quirks.md` |
| UX drifts during the cut-over | Runner and client only translate (checklist); golden `BoardEvent` tests (P2-1, P2-2); UI probe on both engines until deletion |
| Board measurements need the probe, which this WSL1 machine cannot reach | On-board runs batched at phase exits on the probe machine (question to Alex) |
| Random choices rarely play strategically, so rare lines go untested | Human choices from public replays (V-3, V-4); per-handler coverage counts with focus teams aimed at the gaps (V-1); a background sweep of 100,000+ battles a day (V-2) |
| Node or WSL1 flakiness in long sweeps | Per-battle timeouts, crashes reported apart from diffs, private copies of binaries per sweep, cache for the old suites |

## Decisions taken after this plan was written (Alex, 2026-10-23)

- Formats: implement everything Showdown does for the gens we implement, every format rule included, and default to what Showdown's online Random Battles use. For Gen 1 that means shipping both the truncation (cartridge) rules and the Custom Game (plain math) rules, with truncation on by default; the diff harness covers both.
- Hardware budgets: there is no RP2350 board for now. Budget checks at the phase exits use host proxies (struct and stack sizes, `cargo size` of the thumbv8m build, a cycle-count estimate) and stay soft gates until a board is available; the on-chip measurement tasks are deferred, not dropped.
- `battler`: the git submodule is removed in G1-10, together with the last `battler-data` links. The battler repository itself is untouched.
- Verification beyond the random sweep (2026-10-24): a coverage gate, a continuous background sweep, and public Showdown replays (human choices, exact seeds) as a corpus. Phase V; all three feed the phase gates.
- Online play (2026-10-25): phase N, a small `ps_relay` server running `ps_sim` plus a WebSocket transport and a single-seat mode, instead of connecting to Showdown's servers. After G3-9.
