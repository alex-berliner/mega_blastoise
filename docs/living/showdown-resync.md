# Showdown re-sync survey (R-1)

Status: survey, 2026-10-24. Input to R-2a / R-2b
(`showdown-split-execution.md`, phase R). Nothing here is ported yet.

## 1. Target

| | |
|---|---|
| Old pin | `pokemon-showdown` 0.11.11 from npm (`tools/ps-diff`), released 2026-07-28. Its `sim/` and `data/` are byte-identical to the git tag `v0.11.11` = commit `739a5e1fe` (2026-07-27), checked with `diff -rq`. |
| New pin | upstream `master` at `c0845f33d381402dfcb18ce324c20a94f18af075` (2026-10-23, "Force of the Fallen: Update bans"). 225 commits after `v0.11.11`. |
| Why a commit | There is no npm release after 0.11.11 (`npm view pokemon-showdown versions`), and master's `package.json` still says `0.11.11`. The live server runs master: a public gen1randombattle replay fetched on 2026-10-24 starts `>version 3da4dd39a2e6...`, and `c0845f33d` is `3da4dd39a` plus three ban-list commits (`config/formats.ts`, Gen 3 tiers in `data/mods/gen3/formats-data.ts`), no sim or battle-data change. |

The pin is in `tools/ps-diff/package.json` as a codeload tarball URL with a
`postinstall` that runs PS's `build` (master has no `dist/` in git), on the
`w/R-1` branch only. See section 9.

## 2. Method

1. `git diff v0.11.11 c0845f33d` read in full for `sim/*.ts`,
   `data/mods/gen{1,2,3,4}/*`, and base
   `data/{moves,items,abilities,conditions,rulesets}.ts`; every hunk assigned
   to a commit (`git log -S` / `--stat` per commit).
2. A RESOLVED-entry diff: both installs loaded side by side, every entry of
   `Dex.mod('gen1')` and `Dex.mod('gen3')` (moves, items, abilities, species,
   conditions) compared field by field with handler source text
   (`.scratch/tools/resolved-diff.js`). This catches base-file and gen4/gen2
   changes that a gen inherits, and proves the rest unchanged. Field
   `desc`/`shortDesc` (moved to a text table), `nonGhostTarget` (removed) and
   tiers are reported separately.
3. Rule tables of `gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod`,
   `gen1customgame`, `gen1randombattle`, `gen1ou`, `gen3customgame`,
   `gen3randombattle`, `gen3ou` compared rule by rule, handler source included
   (`.scratch/tools/rules-diff.js`).
4. The current Gen 1 server (`ps_sim_cli` at `7d9c2ca`) diffed against a
   second PS install built from `c0845f33d` (section 8), and the divergences
   traced to the entries below.

Resolved-entry result in one line: for Gen 1, no move, species or rule
handler changed except `tox` (G1-A) and dead-code conditions; for Gen 3, 22
moves/items/abilities and 10 conditions changed (section 4.3).

## 3. Scope: `git diff --stat v0.11.11 c0845f33d`

The listed paths:

```
 data/abilities.ts                |  81 ++++++------
 data/conditions.ts               |  34 ++---
 data/items.ts                    | 148 +++++++++++++---------
 data/mods/gen1/conditions.ts     |   6 +-
 data/mods/gen1/formats-data.ts   |   2 +-
 data/mods/gen1/scripts.ts        |   9 +-
 data/mods/gen2/conditions.ts     |  16 +--
 data/mods/gen2/items.ts          |   9 +-
 data/mods/gen2/moves.ts          |  37 +++++-
 data/mods/gen2/rulesets.ts       |   2 +-
 data/mods/gen2/scripts.ts        |   7 +-
 data/mods/gen3/abilities.ts      |  47 +++++++
 data/mods/gen3/conditions.ts     |   4 +-
 data/mods/gen3/formats-data.ts   |  34 ++---
 data/mods/gen3/moves.ts          |  15 ++-
 data/mods/gen3/scripts.ts        |  29 ++++-
 data/mods/gen4/abilities.ts      |  58 +++++++--
 data/mods/gen4/conditions.ts     |  10 +-
 data/mods/gen4/items.ts          |  18 ++-
 data/mods/gen4/moves.ts          |  52 ++++++--
 data/mods/gen4/scripts.ts        |  52 ++------
 data/moves.ts                    | 160 ++++++++++++------------
 data/rulesets.ts                 | 395 ++++++++++++++++++++++++++++-------------------------------
 sim/battle-actions.ts            | 170 ++++++++++++++++---------
 sim/battle-queue.ts              |  13 +-
 sim/battle-stream.ts             |   4 +-
 sim/battle.ts                    | 199 +++++++++++++++++++-----------
 sim/dex-abilities.ts             |   8 +-
 sim/dex-conditions.ts            |   8 +-
 sim/dex-data.ts                  |  35 +++---
 sim/dex-formats.ts               |  68 +++++-----
 sim/dex-items.ts                 |  16 +--
 sim/dex-moves.ts                 |  26 ++--
 sim/dex-species.ts               |   6 +-
 sim/dex-text.ts                  | 390 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 sim/dex.ts                       | 161 ++++++++++++++++--------
 sim/field.ts                     |   4 +-
 sim/global-types.ts              | 147 +++++++++++++++-------
 sim/pokemon.ts                   |  51 +++++---
 sim/prng.ts                      |   4 +-
 sim/side.ts                      |  29 +++--
 sim/state.ts                     |   4 +-
 sim/team-validator.ts            | 105 +++++++++++-----
 sim/tools/exhaustive-runner.ts   |   2 +-
 sim/tools/multi-random-runner.ts |   6 +-
 sim/tools/random-player-ai.ts    |   4 +-
 46 files changed, 1792 insertions(+), 893 deletions(-)
```

Also read, because the harness or the dex dump reads them:

```
 data/formats-data.ts               |  68 +++++++++++++++++----------------
 data/learnsets.ts                  | 117 +++++++++++++++++++++++++++++++++++++++++++++++++++++----
 data/pokedex.ts                    |  43 ++++++++++++++-------
 data/random-battles/gen1/data.json |   2 +-
 data/random-battles/gen3/sets.json |  86 ++++++++++++++++++++++--------------------
 data/random-battles/gen3/teams.ts  |  29 +++++++++++---
 data/tags.ts                       |  77 ++++++++++++++++++-------------------
 config/formats.ts                  | 1074 (format definitions; ours unchanged, see C-21)
```

`sim/teams.ts` (packed team format), `PROTOCOL.md`, `data/typechart.ts` and
`data/scripts.ts` are unchanged.

## 4. Change list

Tags: **core** (shared `ps_sim` machinery), **gen 1**, **gen 3**,
**protocol** (wire text or JSON), **PRNG**. "Gen 1 effect" says what a
current `ps_sim` Gen 1 battle sees; "none" means no observable output or PRNG
difference for Gen 1 (port it anyway where the code structure follows PS).

### 4.1 Core

The one large change is `437926906` "Overhaul 'SwitchIn' event for Gens 3
and 4" (#11185, 2026-10-18; 434+ / 167-, 22 files). Entries C-1 to C-9 are
its parts.

| ID | Tags | PS commit | PS files | Change | `ps_sim` files | Gen 1 effect |
|---|---|---|---|---|---|---|
| C-1 | core, gen 1, gen 3, PRNG | 437926906 | `data/mods/gen4/scripts.ts`, `sim/battle-actions.ts` | gen4's `actions.runSwitch` override is DELETED. Gen 1 and Gen 3 inherited it; they now run base `runSwitch`: `speedSort(getAllActive(true))`, `speedOrder`, `fieldEvent('SwitchIn', switchersIn)`, `isStarted`, gen 4 `substitutebroken` cleanup. The old override's work moved into base `switchIn` under `gen <= 4`: `runEvent('EntryHazard')`; gen <= 2 `lastMove = null` on all actives; gen 1 `lastSelectedMoveSlot = 0` and the Metronome partial-trap re-target; gen <= 2 `runEvent('AfterSwitchInSelf')` when `!side.faintedThisTurn && !isDrag` (replaces `draggedIn`); `if (!pokemon.hp) return false`; gen 3 `runEvent('AfterEntryHazard')` when `turn > 0`. `isDrag` now runs `runSwitch` immediately in every gen (was gen >= 5). `switchIn` returns `!!pokemon.hp`; the `'pursuitfaint'` return is gone (its handling moved into `switchIn`). Ability/item `onStart` no longer run by a `singleEvent('Start')` here (see C-7). | `battle.rs` `switch_in`, `run_switch`, `SwitchResult` | YES. `speedSort` of all actives shuffles on a speed tie, one more PRNG call per `runSwitch` with tied actives; with C-2 the start of a mirror match costs 3 calls instead of 4, in a different order (`.scratch/tools/switch-prng.js`). Bucket D-2 (section 8). AfterSwitchInSelf (psn/brn/tox switch-in damage) now runs inside the switch action, before the queued `runSwitch`. |
| C-2 | core, gen 1, gen 3, PRNG | 437926906 | `sim/battle.ts` `runAction` | Tail of `runAction`: gen < 5 `eachEvent('Update')` no longer runs after `start` or after a `switch`/`instaswitch` action; gen <= 4 `clearActiveMove()` moves before the phazing loop; faint resolution uses C-3 flags (gen 2 defers between switches, gen 4 defers win checks); gen 3 calls `checkFainted()` after every switch action as well as before moves/residual; gen 4 replacement batch rule; the early `return false` when the next action is `instaswitch` is folded into the gen >= 5 Update condition. | `battle.rs` `run_action` | YES (PRNG): `eachEvent` speed-sorts all actives, so on a tie each skipped Update was a skipped shuffle. Part of D-2. |
| C-3 | core, gen 3 | 437926906 | `sim/battle.ts`, `sim/battle-actions.ts`, `sim/global-types.ts` | `faintMessages(lastFirst, forceCheck, checkWin)` renamed `resolveFaints({lastFaintFirst, checkWinIfNoFaints, checkWin})`. Gen <= 4: the `Faint` event runs BEFORE the `faint` line (gen >= 5 after), tracked by a new `faintQueue[].faintEventDone`; `runMove` (gen <= 4) runs pending `Faint` events before `resolveFaints`. Gen <= 4: faints stay pending while any active has a string `switchFlag` (pivot), except the pivot's own blackout check. Pursuit + Destiny Bond: gen 2 processes only the target, gens 3-4 skip the win check. Base `useMoveInner` (gen <= 4) calls `resolveFaints({checkWin: false})` before `AfterMoveSecondarySelf`; `hitStepMoveHitLoop` resolves faints only in gen >= 5. | `battle.rs` `faint_messages`, `actions.rs` `run_move`, `base_actions.rs` | none (Gen 1 has no `Faint`/`BeforeFaint` handlers and overrides `useMoveInner` and `tryMoveHit`). Gen 3: Destiny Bond and Grudge `-activate` now come before `faint`. |
| C-4 | core, gen 3 | 437926906 | `sim/battle.ts`, `data/mods/gen3/scripts.ts`, `data/mods/gen4/scripts.ts` | `checkFainted` skips a side that `!canSwitch(side)`. New overridable `checkFainted`: gen 4 flags one fainted active per side; gen 3 excludes actives with a queued switch-out and, when only one side has fainted actives, flags one at a time. | `battle.rs` `check_fainted`, `mods/gen3/scripts.rs` | none (singles: a side that cannot switch has lost). |
| C-5 | core, gen 3 | 437926906 | `sim/battle-actions.ts`, `sim/battle.ts`, `sim/pokemon.ts` | `runEvent('BeforeSwitchOut')` (in `switchIn` and in `runAction`'s switch-flag loop) becomes `actions.runPursuitActivation(pokemon)`: skip if `skipPursuit`; BeforeSwitchOut; `clearActiveMove()`; gen >= 5 Update; `resolveFaints({checkWin: gen >= 5})` if Pursuit activated; then reset or set `skipPursuit`. Pokemon fields `skipBeforeSwitchOutEventFlag` and `draggedIn` removed, `pursuitActivated` and `skipPursuit` added (both cleared on switch-out). | `battle.rs`, `state.rs` (`Pokemon`) | none (no BeforeSwitchOut handlers in Gen 1; the added `clearActiveMove` has nothing to clear at a switch). |
| C-6 | core, gen 3 | 437926906 | `sim/battle-queue.ts`, `sim/side.ts`, `sim/battle.ts` | `queue.cancelAction` keeps `runSwitch` actions; new `queue.getSwitches(side?)`; `side.clearChoice` seeds `choice.switchIns` from queued switches; `possibleSwitches` excludes queued switch-in targets; forced-switch count uses `battle.canSwitch(side)`. All marked "relevant for Gen 3" (a replacement requested while switches are still queued, C-2/C-4). | `queue.rs`, `choice.rs` `clear_choice`, `request.rs` `possible_switches` | none |
| C-7 | core, gen 3 | 437926906 | `sim/battle.ts` `getCallback` | For `onSwitchIn` on a Pokemon, an Ability/Item (or innate) with no `onSwitchIn` answers with its `onStart` in EVERY gen (was gen >= 5). Gen 3 only: an ability with `onAfterEntryHazard` answers SwitchIn with it on turn 0 and not at all later (later it runs from `switchIn`, C-1). Order now comes from `onSwitchInPriority` (section 4.3). | `events.rs` `get_callback` | none (no abilities or items) |
| C-8 | core, gen 1, gen 3 | 437926906 | `sim/battle-actions.ts`, `data/mods/gen1/scripts.ts`, `data/mods/gen2/scripts.ts` | `selfSwitch` sets `switchFlag = move.id` only if `canSwitch(side)` (or Revival Blessing) and not `commanded`; gen 1 and gen 2 `moveHit` copy the same test. | `base_actions.rs`, `mods/gen1/scripts.rs` `move_hit` | none (no Gen 1 move has `selfSwitch`); Gen 3 Baton Pass with no bench no longer sets the flag. |
| C-9 | core | 437926906 | `sim/battle-actions.ts` | `switchIn`, gen <= 4: a Pokemon with `itemKnockedOff` and an item loses it on switch-in, with hint "In Gens 3-4, Knock Off makes a Pokémon lose its held item every time it switches in." (with 41898849e, see G3-12). | `battle.rs` `switch_in` | none |
| C-10 | core | a19a2fbae | `sim/pokemon.ts` | `setStatus` returns false for a non-active Pokemon when setting a status (`!this.isActive && status`). | `state.rs` `set_status` | none (Gen 1's switch-out `tox` -> `psn` writes `pokemon.status` directly) |
| C-11 | core | 41898849e | `sim/pokemon.ts` | `takeItem`'s `itemKnockedOff` refusal is gen 4 only (was gen <= 4). | `takeItem` (not ported yet; Gen 1 has no items) | none |
| C-12 | core | a7ce30c06 | `sim/battle.ts`, `sim/battle-actions.ts`, `sim/dex-conditions.ts` | `EmergencyExit` always runs, with the pre-damage HP as relay (`onEmergencyExit(originalHp, pokemon)`), the threshold test moved into the handler; new `battle.suppressingSecondaries()` (Sheer Force) checked by callers, `afterMoveSecondaryEvent` no longer checks it; Dynamax `turns` counts down. | `base_actions.rs`, `actions.rs` (if mirrored) | none (no handlers; `runEvent` with no handlers draws no PRNG) |
| C-13 | core | 437926906 | `sim/battle.ts` `directDamage` | Drops `if (target.fainted) this.faint(target)`, which was a no-op (`faint()` returns early on a fainted Pokemon). | `battle.rs` `direct_damage` | none |
| C-14 | core | 234511985, 700b4f75b | `data/moves.ts`, `sim/dex-moves.ts`, `sim/pokemon.ts`, `sim/battle-queue.ts`, `data/mods/gen4/moves.ts` | Move field `nonGhostTarget` removed; Curse and the request's move target hard-code `'self'` for a non-Ghost user (same value); a Champions-only queue branch. | `tools/ps-diff/dump-dex.js` reads `move.nonGhostTarget` (now always undefined) | none in battle; the dex dump loses the field (R-2b) |
| C-15 | core | 49875bcdd, 3da4dd39a | many | Oxlint/stylistic: `a && a.b` -> `a?.b`, `!x.lastMove` -> `x.lastMove === null` (always initialised to `null`), `${x!}` casts, `DebugOutput` casts in debug strings. | none | none |
| C-16 | core, protocol | 8020f383d | `sim/side.ts`, `sim/SIM-PROTOCOL.md` | Team Preview `team` choice: optional `[...]` brackets, forced comma split at 10+ Pokemon, "must choose exactly N" error. | none (no Team Preview in our formats) | none |
| C-17 | core | 247863645 | `sim/battle.ts` | Endless Battle Clause reads `this.gameType` instead of `this.format.gameType`. | none | none |
| C-18 | core | 02f217a35, 89905975e, 57ecb348b | `sim/battle.ts`, `sim/pokemon.ts` | Skill Swap ally hint (gen > 4), Mega Sol vs Electro Shot, Emergency Exit vs switching (Champions), doubles charge-move redirection. | none | none |
| C-19 | core | 3ab832905, 5e8ead64b, d04d59618, 92f01f8f7, 833d0da44 | `sim/dex.ts`, `sim/dex-data.ts`, new `sim/dex-text.ts`, `sim/dex-*.ts` | Text moved out of the effects: `desc`/`shortDesc` are no longer fields of `Dex` entries (`dex.text`, `loadTextData(lang)`, translated tables). `status`/`weather` fields move from `BasicEffect` to `DataMove`. | `tools/ps-diff/dump-dex.js` already drops `desc`/`shortDesc`; `crates/mega_blastoise_core/build.rs` reads its own vendored `data/text/moves.ts` (same format upstream) | none |
| C-20 | core | febe4bfdb, 234b30318, a21fdb77c | `sim/dex-formats.ts` | Complex bans stored in a `Map`; custom format strings capped at 32 KB; `type:`/`teratype:` bans; new `Legends` and `Gmax` nonstandard tags. | none | none |
| C-21 | core | (many ban/tier commits) | `config/formats.ts`, `data/formats-data.ts`, `data/mods/gen{1,3}/formats-data.ts`, `data/learnsets.ts`, `data/pokedex.ts`, `data/tags.ts`, `sim/team-validator.ts` | Format list, tiers, learnsets, validator. The definitions of `gen1customgame`, `gen1randombattle`, `gen3customgame`, `gen3randombattle` (mod, ruleset, banlist, `battle` overrides) are unchanged; no Gen 1 or Gen 3 species changed base stats or types (resolved diff: tier fields only). | none | none |

### 4.2 Gen 1

| ID | Tags | PS commit | PS files | Change | `ps_sim` files | Gen 1 effect |
|---|---|---|---|---|---|---|
| G1-A | gen 1, protocol | 437926906 | `data/mods/gen2/conditions.ts` `tox` (Gen 1 inherits) | `tox.onSwitchIn` (set `status = 'psn'` and add `-status\|POKEMON\|psn\|[silent]`) is replaced by `tox.onSwitchOut` (set `status = 'psn'`, no line). | `mods/gen1/conditions/tox.rs` (+ the `SwitchOut` run in `switch_in`) | YES. A benched ex-Toxic Pokemon shows `psn` (not `tox`) in `side.pokemon[].condition`; the switch-in no longer prints the silent `-status` line. Bucket D-1, 88% of all diffs (section 8). |
| G1-B | gen 1, gen 3, protocol | 56bc083aa | `data/rulesets.ts` | "Standardize descriptions": `Pokémon` -> `Pokemon` in rule texts. Battle-visible: the Sleep Clause Mod `-hint` ("...one of their opponent's Pokemon to sleep at a time"), `\|rule\|Species Clause: Limit one of each Pokemon` (random battle, OU), `\|rule\|Switch Priority Clause Mod: Faster Pokemon switch first` (Gen 3 random battle). Every other `Pokémon` string in `ps_protocol/src/msg.rs` is still present unchanged in master (checked string by string). | `ps_protocol/src/msg.rs` `HintSleepClause`, `ps_protocol/src/command.rs` rule lines | YES. 100/100 `--teams random` battles diverge at the `\|rule\|Species Clause` header line; Sleep Clause hints diverge too. |
| G1-C | gen 1, gen 3 | 7b02eb398 | `data/rulesets.ts` | OHKO Clause becomes a banlist (`Fissure`, `Guillotine`, `Horn Drill`, `Sheer Cold`) instead of an `onValidateSet`; the rule table of `gen1randombattle`, `gen1ou`, `gen3randombattle`, `gen3ou` gains four `-move:` entries. `\|rule\|OHKO Clause` line unchanged. Sleep Moves Clause likewise (not in our formats). | `battle.rs` `rule_table` only if it mirrors table entries | none in battle |
| G1-D | gen 1 | 437926906 | `data/mods/gen1/scripts.ts` | `moveHit` selfSwitch test (C-8). | `mods/gen1/scripts.rs` | none |
| G1-E | gen 1 | 104641c84 | `data/mods/gen4/conditions.ts` (Gen 1 and Gen 3 inherit) | New gen 4 `mustrecharge.onBeforeMove`, same as base minus `pokemon.removeVolatile('truant')`. | `mods/gen1/conditions/mustrecharge.rs` (if it ports the truant line) | none (no Truant in Gen 1); Gen 3 see G3-14 |
| G1-F | gen 1 | 49875bcdd | `data/mods/gen1/conditions.ts`, `data/mods/gen2/*` | `?.` refactors in `slp`, `frz`, `confusion`, gen 2 `lockedmove`, Counter, Lock-On; `focusband`/`berryjuice` (gen 2 items, not in Gen 1). | none | none |
| G1-G | gen 1 | 86505f664 | `data/mods/gen1/formats-data.ts` | Pinsir ZU -> ZUBL. | none | none |
| G1-H | gen 1 | db93869dc..c211813aa | `data/random-battles/gen1/data.json` | Beedrill level 81 -> 84. Changes PS's `--teams random` generator output, not the battle. | none (harness team space only) | none |
| G1-I | gen 1 | 437926906, aa17ca0fa, 40eed4238 | `data/mods/gen2/moves.ts`, `items.ts` | Gen 2 Destiny Bond, Pursuit message, berry thresholds: not in Gen 1's dex. | none | none |

Resolved Gen 1 conditions that changed but never run in a Gen 1 battle:
`futuremove`, `dynamax`, `deltastream` (style/hints, C-12, 0ac140354,
651b6fa85).

### 4.3 Gen 3 (for the phase 4 port; Gen 3 has no handler batches yet)

Every entry here is what `Dex.mod('gen3')` resolves to at the new pin. Phase 4
reads master, so these are information for the G3 briefs, not R-2 work,
except where the core already models them (C-1 to C-9).

| ID | Tags | PS commit | Resolved entry | Change | `ps_sim` area |
|---|---|---|---|---|---|
| G3-1 | gen 3 | 437926906 | abilities `drizzle`, `drought`, `sandstream` (prio 3), `airlock`, `cloudnine` (prio 4), `forecast` (prio 3, was -2) | `onSwitchIn: undefined`; `onAfterEntryHazard` calls the ability's `onStart`; `onSwitchInPriority` set. Turn 0 they answer SwitchIn (C-7); later they fire from `switchIn` before other switch-ins (C-1). | `mods/gen3/abilities/*` |
| G3-2 | gen 3 | 437926906 | abilities `intimidate` (2), `trace` (1), `pressure` (3) | `onSwitchInPriority` added (Intimidate before Trace before items). | `mods/gen3/abilities/*` |
| G3-3 | gen 3 | 437926906 | item `whiteherb` | `onAnySwitchInPriority: -2` removed (gen 4 items). | `mods/gen3/items/*` |
| G3-4 | gen 3 | 437926906 | move `spikes` condition | `onEntryHazardPriority: 2`. | `mods/gen3/moves/spikes.rs` |
| G3-5 | gen 3 | 437926906 | condition `slp` | Nightmare skipped-time bookkeeping moves from `onSwitchIn` to `onSwitchOut`. | `mods/gen3/conditions/slp.rs` |
| G3-6 | gen 3, core | 437926906 | `checkFainted` (gen 3 scripts) | C-4. | `mods/gen3/scripts.rs` |
| G3-7 | gen 3, protocol | aa17ca0fa, 437926906, 9b5c760fc | move `pursuit` condition, `useMoveInner` (gen 3 scripts) | Adds `-activate\|TARGET\|move: Pursuit` once per switching target (`pokemon.pursuitActivated`); the Pursuit `useMove` passes the stored `sourceEffect`; under Pressure the extra PP comes off the CALLER move (`callerMoveForPressure`, any `sourceEffect` with `pp`), not only `pursuit`. | `mods/gen3/moves/pursuit.rs`, `mods/gen3/scripts.rs` |
| G3-8 | gen 3 | 437926906 | move `batonpass` | `self.onHit` sets `skipPursuit` (field renamed). | `mods/gen3/moves/batonpass.rs`, `state.rs` |
| G3-9 | gen 3, protocol | 437926906 | moves `destinybond`, `grudge` conditions | `onFaint` bails if `!source?.hp`; with C-3 the `-activate` precedes the `faint` line. | `mods/gen3/moves/*` |
| G3-10 | gen 3 | 0ac140354 | move `endure` condition, condition `futuremove` | Endure does not block a future move (`!effect.flags['futuremove']`); a landing future move no longer strips Protect/Endure. | `mods/gen3/moves/endure.rs`, `conditions/futuremove.rs` |
| G3-11 | gen 3, protocol | 651b6fa85 | condition `futuremove` | The turn-254 hint only in gen >= 8; `onRestart` returns false (no hint before gen 8). | `conditions/futuremove.rs` |
| G3-12 | gen 3, protocol | 41898849e | move `knockoff`, `takeItem`, `switchIn` | Gen 3 Knock Off: the item is lost again on every switch-in (C-9, with hint), a knocked-off Pokemon CAN obtain a new item (C-11); the old hint now only in gen 4. | `mods/gen3/moves/knockoff.rs`, `takeItem` (to port), `battle.rs` |
| G3-13 | gen 3 | 40eed4238 | items `berry`, `goldberry` | Heal when `hp < maxhp / 2` (was `<=`). | `mods/gen3/items/*` |
| G3-14 | gen 3 | 104641c84 | condition `mustrecharge` | The recharge turn no longer removes the `truant` volatile (G1-E). | `conditions/mustrecharge.rs` |
| G3-15 | gen 3, protocol | bb179fbf8 | ability `hypercutter` | `-fail\|POKEMON\|unboost\|atk\|[from] ability: Hyper Cutter` (stat id, was `Attack`). | `mods/gen3/abilities/hypercutter.rs` |
| G3-16 | gen 3 | 234511985 | move `curse` | `move.target = 'self'` literal (C-14), same behaviour. | none |
| G3-17 | gen 3 | 49875bcdd | `bide`, `disable`, `encore`, `iceball`, `rollout`, `uproar`, `focusband`, `mysteryberry`, `damp`, `stickyhold`, `synchronize`, status `brn`/`frz`/`par`/`psn`/`tox` `onStart` | Style only. | none |
| G3-18 | gen 3 | 437926906 | `actions.useMoveInner` (gen 3), `suppressingSecondaries` (a7ce30c06) | Gen 3 `useMoveInner` uses `suppressingSecondaries()`; base gen <= 4 `resolveFaints({checkWin:false})` (C-3) does not reach Gen 3 (it overrides `useMoveInner`). | `mods/gen3/scripts.rs` |
| G3-19 | gen 3 | 0490edb43, db93869dc, 841c1d1be, c211813aa | `data/random-battles/gen3/{sets.json,teams.ts}` | Set changes; Dugtrio/Wobbuffet never lead (`NO_LEAD_POKEMON`, lead chosen first and `unshift`ed). Harness team space only. | none |

`data/mods/gen4/abilities.ts` also changes Truant/Slow Start (`activeTurns`,
104641c84), Leaf Guard, and others that Gen 3 overrides or lacks; the
resolved diff shows no other Gen 3 ability change.

## 5. File coverage

Every file in the section 3 stat, and where it lands:

| File | Entries |
|---|---|
| `data/abilities.ts` | G3-15 (bb179fbf8); d39cea805, f0039f571, 7340ea497, 490b7fb8c, a7ce30c06 (gen 5+ abilities, not in Gen 3's resolved table); C-19 desc removal |
| `data/conditions.ts` | C-12 (Dynamax), G3-10, G3-11, C-15 |
| `data/items.ts` | G3-13 (40eed4238); 7244fe8e2, a833737d8, 57ecb348b (new items / Champions, not in Gen 3); C-19 |
| `data/mods/gen1/conditions.ts` | G1-F |
| `data/mods/gen1/formats-data.ts` | G1-G |
| `data/mods/gen1/scripts.ts` | G1-D (C-8), C-15 |
| `data/mods/gen2/conditions.ts` | G1-A, G1-F |
| `data/mods/gen2/items.ts` | G1-I (40eed4238 Berry Juice; not in Gen 1) |
| `data/mods/gen2/moves.ts` | G1-I (Destiny Bond, Pursuit message), C-15 |
| `data/mods/gen2/rulesets.ts` | G1-B (NC 2000 desc, validator only) |
| `data/mods/gen2/scripts.ts` | C-8 (overridden by Gen 1), C-15 |
| `data/mods/gen3/abilities.ts` | G3-1, G3-2 |
| `data/mods/gen3/conditions.ts` | G3-5 |
| `data/mods/gen3/formats-data.ts` | C-21 (tiers) |
| `data/mods/gen3/moves.ts` | G3-7 |
| `data/mods/gen3/scripts.ts` | G3-6, G3-7, G3-18 |
| `data/mods/gen4/abilities.ts` | G3-1, G3-2 (inherited parts); Truant (104641c84, Gen 3 overrides) |
| `data/mods/gen4/conditions.ts` | G1-E, G3-14 |
| `data/mods/gen4/items.ts` | G3-3; Quick Powder for transformed Ditto (dbff7e8d4, gen 4 item); C-15 |
| `data/mods/gen4/moves.ts` | G3-4, G3-10 (Endure), G3-12 (Knock Off hint), C-14 (Curse); Pursuit (gen 4 only, Gen 3 has its own); Healing Wish / Lunar Dance on `EntryHazard` (gen 4 moves) |
| `data/mods/gen4/scripts.ts` | C-1 (runSwitch deleted), C-4 (gen 4 checkFainted) |
| `data/moves.ts` | C-14, G3-8, G3-9, C-5 (Pursuit/Baton Pass/Shed Tail fields); 651b6fa85 Wish hint; a9042fb1a, 247863645 (gen 5+ moves); C-19 |
| `data/rulesets.ts` | G1-B, G1-C; 954971def (One Boost Passer Clause adds Recycle, validator only, `gen3ou`); other rules not in our formats |
| `sim/battle-actions.ts` | C-1, C-3, C-5, C-8, C-9, C-12, C-15 |
| `sim/battle-queue.ts` | C-6, C-14 (Champions branch) |
| `sim/battle-stream.ts` | lint comments only |
| `sim/battle.ts` | C-1, C-2, C-3, C-4, C-5, C-6, C-7, C-12, C-13, C-17, C-18 |
| `sim/dex-abilities.ts`, `sim/dex-items.ts`, `sim/dex-species.ts` | C-19 (types for mod text), Plate fling fix (gen 4+) |
| `sim/dex-conditions.ts` | C-3 types, C-12 (`onEmergencyExit` signature), new `onAfterEntryHazard` / `onEntryHazardPriority` |
| `sim/dex-data.ts`, `sim/dex.ts`, `sim/dex-text.ts` | C-19 |
| `sim/dex-formats.ts` | C-20 |
| `sim/dex-moves.ts` | C-14, C-19 (`status`/`weather` on `DataMove`) |
| `sim/field.ts`, `sim/state.ts` | C-15 |
| `sim/global-types.ts` | C-1, C-3, C-4 (signatures), C-20 |
| `sim/pokemon.ts` | C-5, C-10, C-11, C-14, C-18, gem `-enditem` gains `[move]` (gen 5+) |
| `sim/prng.ts` | lint comments only; see section 6 |
| `sim/side.ts` | C-6, C-16 |
| `sim/team-validator.ts`, `sim/tools/*` | C-21 / not used by the harness |

## 6. PRNG

**Numeric seeds still select `Gen5RNG`.** `sim/prng.ts` changed only in two
lint comments. `PRNG.setSeed` in both versions: an array is joined with `,`
first ("compat for old inputlogs"); `sodium,<hex>` selects `SodiumRNG`;
`gen5,<16 hex>` selects `Gen5RNG` from four 16-bit hex groups; a seed whose
first character is a digit selects `Gen5RNG` from `split(',').map(Number)`;
anything else throws. With no seed, PS generates a sodium one. Checked on both
installs: `new PRNG([1,2,3,4]).rng` is `Gen5RNG` and `random(100)` is 47 on
each, and `gen5,0001000200030004` gives the same 47. The harness keeps working
unchanged (`--self` runs green on the new pin, section 8).

**SodiumRNG is already in 0.11.11** (identical in master), so R-2a ports it
from the pinned source either way. What it takes:

- Algorithm (`SodiumRNG`, a drop-in for libsodium's
  `randombytes_buf_deterministic`, implemented with `ts-chacha20` 1.2.0):
  - Seed: `sodium,<hex>`; the hex is right-padded with `0` to 64 digits and
    written into a 32-byte key (generated seeds are 32 hex digits, so the
    last 16 bytes start at zero).
  - `next()`: ChaCha20 in the RFC 7539 (IETF) form: 20 rounds, 32-bit block
    counter starting at 0, 96-bit nonce = the ASCII bytes `LibsodiumDRG`.
    Encrypt 36 zero bytes (one block function call; the rest of the 64-byte
    block is discarded). Bytes 0..31 become the new key; bytes 32..35 read
    BIG-endian are the 32-bit output.
  - `getSeed()`: `sodium,` + 64 hex digits of the current key.
  - `random`, `randomChance`, `sample`, `shuffle` sit on `next()` exactly as
    for `Gen5RNG` (`floor(next * n / 2^32)`), so `ps_sim`'s existing
    `Prng` methods and call counter carry over unchanged.
- Vectors (from the 0.11.11 install; `next()` four times, then `getSeed()`):
  - `sodium,00000000000000000000000000000000`: 4267713560 2597593732
    1237973749 687352079; seed after:
    `sodium,274564c90cb73c6453f4a4152c3fb2c3021f7950723025ec25604fce96de1ed4`
  - `sodium,d4e69dd8ca6bd83e6f23520d0c6d7984` (a real gen1randombattle
    replay's battle seed): 3452080645 121223913 3901222867 1439957731; seed
    after:
    `sodium,2855a2a44139525971584096f7c91a3b41f19ff0e519720ad918afc9adaf03c5`
  - plus the RFC 7539 section 2.3.2 block-function vector for the ChaCha20
    core itself. `tools/ps-diff/prng-fixtures.js` can emit sodium seeds the
    same way it emits Gen5 ones.
- `ps_sim` (`crates/ps_sim/src/prng.rs`): `Prng` holds an enum
  `{ Gen5(Gen5Rng), Sodium(SodiumRng) }`; `SodiumRng` is a 32-byte key plus a
  hand-written `no_std` ChaCha20 block function (about 50 lines, no crate,
  no `unsafe`). State grows from 8 to 32 bytes. Cost is one 20-round block per
  `next()`; a host proxy and the thumbv8m cycle estimate go in R-2a's report
  (the device can keep numeric seeds; sodium is for replays).
- How seeds are written. A current inputlog (public replay, 2026-10-24):

  ```
  >version 3da4dd39a2e6d535c757a740535a03fc621953b2
  >start {"formatid":"gen1randombattle","seed":"sodium,d4e69dd8ca6bd83e6f23520d0c6d7984","rated":"Rated battle"}
  >player p1 {"name":"...","avatar":"266","rating":1110,"seed":"sodium,f4d9760178039e8f23a5a21547fddfbb"}
  >player p2 {"name":"...","avatar":"265","rating":1040,"seed":"sodium,06fd6e12af13b83396eab47762aeae30"}
  ```

  `Battle`'s constructor writes `>start` with `seed: this.prngSeed`, the PRNG's
  `startingSeed` STRING, so PS 0.11.10+ inputlogs carry numeric seeds as
  `"seed":"a,b,c,d"` (a string), older ones as a JSON array. The `>player`
  `seed` is the TEAM generator seed (`Battle.getTeam` -> 
  `Teams.getGenerator(format, seed)`), not the battle PRNG; `ps_sim` does not
  run PS's team generator, so the replay driver hands it the team PS built
  (packed), as V-3 already does. `>reseed <seed>` (debug) takes the same seed
  forms.
- `ps_protocol` (`command.rs`, `text/command.rs`): `Command::Start.seed`
  becomes a seed enum; the decoder accepts a JSON array of four integers, the
  string `a,b,c,d`, `gen5,<16 hex>` and `sodium,<hex>`, and ignores `rated`;
  the `>player` decoder must ignore `avatar`, `rating`, `seed` when a `team`
  is present. The encoder keeps writing the array form for Gen5 seeds (the
  harness's form), `sodium,<hex>` for sodium.

## 7. Protocol

No wire-format change. `PROTOCOL.md` is unchanged; `sim/SIM-PROTOCOL.md`
only documents the Team Preview bracket syntax (C-16); the request builders
(`getRequestData`, `getMoveRequestData`, `getSwitchRequestData`) are
unchanged, so the `|request|` JSON has no new or renamed fields;
`sim/teams.ts` (packed teams) is unchanged; `lib/streams.ts` and
`sim/battle-stream.ts` changed only in lint comments.

Changed or new lines `ps_protocol` / `ps_client` must produce or accept for a
current server, in our gens:

| Line | Gen | Change | Entry |
|---|---|---|---|
| `\|rule\|Species Clause: Limit one of each Pokemon` | 1, 3 (random, OU) | `Pokémon` -> `Pokemon` | G1-B |
| `\|rule\|Switch Priority Clause Mod: Faster Pokemon switch first` | 3 (random, OU) | same | G1-B |
| `\|-hint\|Sleep Clause Mod prevents players from putting more than one of their opponent's Pokemon to sleep at a time` | 1, 3 | same | G1-B |
| `\|-status\|POKEMON\|psn\|[silent]` on switch-in of an ex-Toxic Pokemon | 1 | no longer sent | G1-A |
| `condition` of a benched ex-Toxic Pokemon in `\|request\|` | 1 | `NN/MM psn` (was `tox`) | G1-A |
| `\|-activate\|POKEMON\|move: Pursuit` | 3 | new, once per switching target, before Pursuit's `\|move\|` | G3-7 |
| `\|-activate\|POKEMON\|move: Destiny Bond` / Grudge | 3 | now BEFORE `\|faint\|` | C-3, G3-9 |
| `\|-fail\|POKEMON\|unboost\|atk\|[from] ability: Hyper Cutter\|[of] POKEMON` | 3 | stat id (was `Attack`) | G3-15 |
| `\|-hint\|In Gens 3-4, Knock Off makes a Pokémon lose its held item every time it switches in.` | 3 | new, at switch-in | C-9, G3-12 |
| `\|-hint\|In Gens 3-4, Knock Off only makes the target's item unusable; it cannot obtain a new item.` | 3 | no longer sent in Gen 3 | G3-12 |
| `\|-hint\|In Gen 8+, Future attacks will never resolve...` | 3 | no longer sent (gen >= 8 only) | G3-11 |
| `\|-hint\|Previously chosen switches continue in Gen 2-4 after a Pursuit target faints.` | 3 | same text, now from `switchIn` | C-1 |

`ps_client` should parse all of these already (generic `-activate`,
`-hint`, `-fail` with a stat word); R-2a confirms with a test. For a LIVE
server the room layer adds lines outside the sim (`|j|`, `|l|`, `|t:|`,
`|inactive|`, `|badge|`, `|uhtml|`, `|raw|`, chat), unchanged by this
survey's paths; V-3's validation already lists them.

## 8. Diff 100: current Gen 1 server against the new pin

Setup: `ps_sim_cli` built from this branch (`7d9c2ca`, `cargo build --release
-p ps_harness --bin ps_sim_cli`). Second PS install at
`.scratch/ps-new/node_modules/pokemon-showdown` (`git archive c0845f33d`,
`npm install --omit=dev --omit=optional`, `node build`). A mirror of the
harness at `.scratch/psn/tools/ps-diff/` (copies of the `.js` files, its
`node_modules` symlinked to the new install, `crates` symlinked to the repo),
so `diff.js` resolves the new PS without touching `tools/ps-diff`.
All runs `--jobs 8`; baseline is the same command on 0.11.11.

Raw (harness unmodified):

| run | 0.11.11 | new pin |
|---|---|---|
| `--gen 1 --n 100 --teams random` (cartridge) | 0 diffs | 100 diffs |
| `--gen 1 --n 100 --teams random --trunc math` | 0 | 100 |
| `--gen 1 --n 100 --teams wide` (cartridge) | 0 | 2 |
| `--gen 1 --n 100 --teams wide --trunc math` | 0 | 2 |

All 200 random-team failures stop at header line 11, `|rule|Species Clause`
(G1-B). To see past it, the scratch mirror's `normaliseLines` also reduced
`|rule|` lines to the rule name and mapped `Pokémon` to `Pokemon` (scratch
copy only; the real harness is untouched). Then:

| run | diffs | first-divergence buckets |
|---|---|---|
| random, cartridge | 3 | D-2: 3 (seeds 5, 32, 39) |
| random, math | 3 | D-2: 3 (same seeds) |
| wide, cartridge | 2 | D-1: 2 (seeds 26, 55) |
| wide, math | 2 | D-1: 2 |

Bigger samples with the same normalisation:

| run | 0.11.11 | new pin |
|---|---|---|
| `--pinned` | 35 ran, 0 diffs, 3 unported (Gen 3) | 1 diff (Gen 1 seed 32, D-2) |
| `--carried 1 --allow-unported` (25,320 battles) | 0 diffs (5 PS crashes, known) | 859 diffs (5 PS crashes) |

Carried buckets (`.scratch/tools/bucket.py`):

| Bucket | Count | Cause |
|---|---|---|
| D-1: `\|request\|` differs only in a benched Pokemon's `condition`, new `psn` vs ours `tox` | 757 | G1-A |
| D-2: a stream line (`move`, `-damage`, `-crit`, `split`, `cant`, `-activate`, `-supereffective`) after a speed tie between the actives earlier in the battle; same lines, different rolls | 101 | C-1 + C-2 (PRNG shuffles moved) |
| D-3: no tie found in the requests before the divergence (`fuzz_gen1_battles_w1_s421#22` seed 16, line 460, a `[miss]` flip) | 1 | probably D-2 from a tie the request scan cannot see (only one side's stats are in each request); R-2a re-checks it after porting C-1/C-2 |

D-2 is confirmed directly: through `start` of a Wartortle mirror, 0.11.11
draws 4 PRNG calls (the `insertChoice` tie plus three `eachEvent('Update')`
shuffles) and the new pin draws 3 (`insertChoice`, the new `runSwitch`
`speedSort`, one Update); a non-tie pairing draws 0 on both
(`.scratch/tools/switch-prng.js`).

The harness runs on the new pin as is: `--self --n 20` green for Gen 1
(random, wide, `--trunc math`) and Gen 3 (random, wide); `rules.js`'s
`Cartridge Trunc` install still works.

## 9. The pin

`tools/ps-diff/package.json` on `w/R-1` (not for `ps-split` before R-2):

```json
"scripts": { "postinstall": "node node_modules/pokemon-showdown/build" },
"dependencies": {
  "pokemon-showdown": "https://codeload.github.com/smogon/pokemon-showdown/tar.gz/c0845f33d381402dfcb18ce324c20a94f18af075"
}
```

- A tarball URL, not `github:`: npm records `github:` deps as `git+ssh://` in
  the lockfile, which `npm ci` cannot fetch without SSH keys. The codeload URL
  locks with an `integrity` hash.
- `postinstall` is needed because master keeps no `dist/` in git and has no
  `prepare` script. `build` needs Node >= 22 (the machine has 24.22.1) and
  the `esbuild` and `ts-chacha20` deps that npm installs with PS.
- Tested in `.scratch/pin-test` (fresh `npm install`, 1.5 min): `dist/` built,
  `sim/` and `data/` identical to the install used in section 8.
  `tools/ps-diff/package-lock.json` must change with it; the regenerated lock
  is in the same uncommitted change.
- The worker checkouts hard-link `tools/ps-diff/node_modules` from the main
  repo (`cp -al`): run `npm install` in a fresh copy, never in place, or every
  worktree's install changes at once.
- `dump-dex.js` records `require('pokemon-showdown/package.json').version`,
  which still reads `0.11.11` on master; R-2b should record the commit.

## 10. Suggested R-2 split

**R-2a, core and protocol (C2):** C-1 and C-2 (`switch_in` / `run_switch` /
`run_action` re-port from the new `battle-actions.ts` / `battle.ts`; this
clears D-2), C-3 (`resolve_faints` with its options and the gen <= 4 `Faint`
order), C-4 (`check_fainted` with `canSwitch` and the per-gen override hook),
C-5, C-6, C-7 (`get_callback`), C-8, C-10, C-11, C-13; `ps_protocol` text
G1-B; SodiumRNG and the seed enum (section 6); a `ps_client` test over the
section 7 lines. Acceptance as written (diff 20,000 Gen 1 on the new pin).

**R-2b, Gen 1 data and handlers, dex re-dump (MM):** G1-A (`tox.rs`
`onSwitchOut`, clears D-1, check the `SwitchOut` run in `switch_in`), G1-C
(rule table entries if mirrored), G1-E (`mustrecharge` truant line, a no-op in
Gen 1), the `ps_dex` re-dump on the new pin (`nonGhostTarget` gone, version
string), and repinning `tools/ps-diff` (package.json + lock). R-2b can land
before R-2a: D-1 is independent of D-2.

Then: G3 briefs cite section 4.3 for the entries they touch; G3-1 to G3-6
depend on R-2a's C-1, C-2, C-4, C-7.
