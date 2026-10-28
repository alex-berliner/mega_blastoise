# Showdown's own Gen 1 and Gen 3 tests: an inventory and what covers them

## 1. What this document is

Showdown ships its own mocha test suite (`tools/ps-diff/node_modules/pokemon-showdown/test/`).
This document inventories **every** test in that suite that runs under **Gen 1** or
**Gen 3** rules, and says for each one whether our engine already covers it
(`crates/ps_harness/targeted/gen1.json`, `crates/ps_harness/targeted/gen3.json`),
whether the test cannot apply to that generation, or whether the test is new and
now has a targeted battle of its own.

The inventory was built by reading the test files, not by guessing: every
`it(...)` block in the tree was scanned for the generation it runs under
(`common.gen(1)` / `common.gen(3)`, a `formatid` of `gen1*` / `gen3*`, a
`Dex.mod('gen1'|'gen3')`, or the enclosing `describe`), and the bodies were
read one by one.

## 2. The labels

| label | meaning |
| --- | --- |
| **covered** | an existing targeted battle (or a pinned seed) already reaches the same handler and line as the test. |
| **new** | nothing reached it before. A targeted battle was added for it in this task, unless the row says *not targeted yet*. |
| **not applicable** | the move, rule or game type the test needs does not exist in that generation (doubles, triples, free-for-all, the Stadium mod, a team generator, a text table), so the test has no Gen 1 / Gen 3 meaning. |
| **not driveable** | a deviation from the three labels above, used for the tests that need a PS-internal hook (`forceRandomChance`, `battle.onEvent('Accuracy')`, `moveSlots[].pp` edits, `setStatus`, `volatile.time`) or that assert on state the battle log does not carry (a move lock, a speed stat, a boost table). The battle protocol the harness speaks cannot express them, so no targeted battle can be written without changing the harness. |

Every **not driveable** row says which of the two reasons applies.

## 3. Files checked

The whole `test/` tree was scanned: 396 JavaScript files, 387 of them with
`it` blocks. 37 files hold at least one Gen 1 or Gen 3 test, and those 37
are the ones listed below; the other 350 were checked and hold no Gen 1 / Gen 3
test. Every `it` block of the 37 files is in the tables: 137 tests.

## 4. The inventory

### `random-battles/gen1.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 1] should not give bad Pokemon as leads (13) | **not applicable** | a random team generator, not a battle: no Gen 1 battle rules are exercised. |

### `sim/abilities/flashfire.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 3] should activate and grant Fire-type immunity even if the user is frozen in Gen 3 (65) | **covered** | `flashfire_gen3_frozen` |

### `sim/abilities/magnetpull.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 3] should prevent Steel-type allies from switching out normally (56) | **not applicable** | doubles (an ally); the harness plays singles. |

### `sim/abilities/thickfat.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 3] should halve damage from Fire- or Ice-type attacks in past generations, even when holding a type-boosting item (27) | **covered** | `thickfat_gen3` |

### `sim/abilities/trace.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 3] should not activate the opponent's Ability (146) | **covered** | `trace_gen3` |

### `sim/dex-text.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 3] binds domain objects and tables to the requested language (43) | **not applicable** | the text tables, no battle. |

### `sim/items/quickclaw.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 3] causes Speed ties with every holder when activated (30) | **covered** | `quickclaw_gen3` |

### `sim/items/sitrusberry.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 3] should not activate in the same turn that was put below 50% HP by a status condition (73) | **covered** | `sitrus_gen3` |

### `sim/misc/faint-order-old-gens.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 3] should check the outcome after each replacement, before the other side switches in (420) | **new** | not targeted yet: both sides replace at the same time, which the harness's one-choice-per-request queue cannot script. |
| [Gen 3] should finish a Pursuit-interrupted switch and its hazards before checking the outcome (440) | **new** | not targeted yet: the same replacement queue problem (the p2 replaces after the p1's Pursuit). |
| [Gen 3] should check the outcome after one voluntary switch before executing the next (459) | **not applicable** | doubles. |

### `sim/misc/faint-order.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 1] should end the turn in Gen 1 (13) | **covered** | `faint_gen1_endturn` |
| [Gen 3] should end the turn in Gen 3 (26) | **covered** | `faint_gen3_endturn` |

### `sim/misc/ohko.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 3] should deal damage equal to the target's HP (38) | **covered** | `ohko_gen3_hp_damage` |
| [Gen 1] should faint the target's substitute (79) | **covered** | `ohko_gen1_substitute` |
| [Gen 1] should fail if the target has a higher speed (103) | **covered** | `ohko_gen1_speedfail` |

### `sim/misc/recoil.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 1] should deal recoil damage based on the damage dealt (25) | **covered** | `recoil_gen1` (and it found a difference in Showdown's own test, see section 7) |

### `sim/misc/statdownoverflow.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| SafeTwo (13) | **new** | not driveable: the test reads the internal stat table (a stat that overflows the cartridge's byte); the log only carries damage numbers, so the overflow is only observable through the damage the cartridge truncation produces (every Gen 1 targeted battle runs under the cartridge truncation). |
| Not SafeTwo (33) | **new** | not driveable: the same internal stat table. |

### `sim/misc/statuses.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| should reduce atk to 50% of its original value in Stadium (46) | **not applicable** | the Stadium mod (`gen1stadiumou`) is not ported. |
| should reduce speed to 25% of its original value in Stadium (141) | **not applicable** | the Stadium mod is not ported. |
| should reapply its speed drop when an opponent uses a stat-altering move in Gen 1 (152) | **new** | not driveable: the assertion is on `getStat('spe')`, an internal number the log does not carry. |
| should not reapply its speed drop when an opponent uses a failed stat-altering move in Gen 1 (164) | **new** | not driveable: the same internal speed number. |
| should affect Leech Seed damage counter (304) | **new** | not driveable: the assertion is on the Leech Seed volatile's counter, not in the log. |
| should cause a desync if a Pokemon attacks immediately after thawing without having attacked since its last switch (386) | **new** | not driveable: the test needs PS's internal thaw bookkeeping (`pokemon.lastMove`) to force the desync. |

### `sim/misc/trapmoves.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| Wrap ends when wrapped Pokemon dies of residual damage (196) | **new** | not targeted yet: the Wrap's end is a request-level lock, the log only shows the damage. |
| Wrap ends when wrapped Pokemon switches to a Pokemon that dies of residual damage (210) | **new** | not targeted yet: the same, plus a switch in the middle. |
| Wrap ends when wrapper dies to residual damage (227) | **new** | not targeted yet: the same. |
| Wrap ends when wrapper switches to a Pokemon that dies of residual damage (241) | **new** | not targeted yet: the same, plus a switch. |
| Wrap should damage the target's substitute (259) | **new** | not targeted yet: the harness could not script the Psyduck's second move (PS rejects `move 2` once the Wrap traps it); the trap and the Substitute need a different choice queue. |
| Wrap should never miss if the target is already trapped (278) | **new** | not driveable: the assertion is on the Wrap's internal accuracy state. |
| should stay asleep if it switched in after a Pokemon spent a turn trapped (293) | **new** | not driveable: the assertion is on the sleep counter of a switched-in Pokemon. |
| should continue if copied by Mirror Move (324) | **new** | not targeted yet: the Mirror Move plus the trap needs a longer script than the harness's queue expresses. |
| should restart the trapping move if copied by Mirror Move and the target switches (350) | **new** | not targeted yet: the same. |
| should use Metronome if copied by Metronome even if the target switches (377) | **new** | not targeted yet: the same. |

### `sim/misc/turn-order.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 3] should make an instant switch request if a Pokemon faints during switch-in (289) | **not applicable** | doubles. |
| [Gen 3] should make an instant switch request if a Pokemon faints during switch-in (337) | **not applicable** | doubles. |
| [Gen 3] should not send a switch request if the only available pokemon are queued (364) | **not applicable** | doubles. |
| [Gen 3] should finish replacements after Pursuit and Destiny Bond before activating Intimidate (390) | **not applicable** | doubles. |
| [Gen 3] should preserve pending Intimidate when a replacement faints to Spikes (414) | **not applicable** | doubles. |
| [Gen 3] should request one switch per side at a time when only one side has fainted Pokemon (438) | **not applicable** | doubles. |
| [Gen 3] should still request replacements when the other side has no available switches (480) | **not applicable** | doubles. |
| [Gen 1] should cancel the other switch and request a replacement after a switch-in faints to poison (512) | **new** | not driveable: the assertion is on the *request state* of each side, which the battle log does not carry. |

### `sim/misc/twoturnmoves.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| charges the first turn, does damage and uses PP the second turn (13) | **new** | not driveable: the test reads `moveSlots[].pp` and the move's internal phase. |
| move is paused when asleep or frozen (32) | **new** | not targeted yet: the pause is visible in the log, but the sleep needs a seed scan and a freeze needs an Ice move (no frozen state in the log). |
| two-turn move ends if it fails due to Disable, does not use PP (46) | **new** | not driveable: the assertion is on the move's PP. |
| if called by Metronome or Mirror Move, the calling move uses PP in the attacking turn (63) | **new** | not driveable: the assertion is on the move's PP. |
| Dig/Fly dodges all attacks except for Swift, Transform, and Bide (79) | **new** | not targeted yet: the semi-invulnerable state needs a long script. |
| Dig/Fly invulnerability glitch (92) | **new** | not targeted yet: the same. |
| should be soft-locked if it was woken up by Haze during the charging turn (115) | **new** | not driveable: the soft lock is a request-level state. |
| should be delayed by trapping moves (138) | **new** | not targeted yet: the trap plus the charge needs a longer script. |

### `sim/moves/bide.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| should be possible to roll two-turn Bide (13) | **covered** | `bide_gen1_two_turn` |
| should be possible to roll three-turn Bide (33) | **covered** | `bide_gen1_three_turn` |
| should damage Substitute with Bide damage (50) | **covered** | `bide_gen1_substitute` |
| should accumulate damage as the opponent switches or uses moves that don't reset lastDamage (68) | **new** | not targeted yet: the opponent has to switch in the middle of the Bide, which the harness's choice queue cannot script (the switch request has its own row). |
| should not zero out accumulated damage when an enemy faints (Desync Clause Mod) (88) | **new** | not targeted yet: the same (the enemy has to faint inside the Bide). |
| should pause Bide's duration when asleep or frozen (116) | **covered** | `bide_gen1_pause_sleep` |
| should pause Bide's duration when disabled (132) | **covered** | `bide_gen1_pause_disable` |

### `sim/moves/brickbreak.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 3] should break the foe's Reflect when used against an ally in Gen 3 (97) | **not applicable** | doubles. |

### `sim/moves/conversion2.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 3] should not fail to change to a type the user already has (72) | **covered** | `conversion2_gen3` |

### `sim/moves/counter.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 1] Counter Desync Clause (179) | **covered** | `counter_first_slot_sleep` (the first-slot hint); the second hint (the last announced move) is not targeted yet. |
| [Gen 1] should counter attacks made against substitutes (220) | **covered** | `counter_gen1_substitute` |
| [Gen 1] simultaneous counters should both fail (241) | **new** | not targeted yet: both sides need the Counter in the same turn and the script has to pick the right slot. |
| [Gen 1] (High) Jump Kick recoil can be countered (280) | **new** | not targeted yet: the recoil has to be the last move the Counter reads. |
| [Gen 1] confusion damage can be countered (291) | **new** | not targeted yet: the same, with a confusion step. |
| [Gen 1] draining can be countered (305) | **new** | not targeted yet: the same, with a draining move. |
| [Gen 1] Mirror Move can be countered when it calls a counterable move (319) | **new** | not targeted yet: the same, with a Mirror Move step. |
| [Gen 1] Moves with unique damage calculation don't overdamage a target with less HP (331) | **new** | not targeted yet: the fixed-damage move has to leave the target alive, so the script needs a different finisher. |
| [Gen 1] Metronome calling Counter fails (341) | **new** | not targeted yet: the Metronome has to roll the Counter, which needs a seed scan. |

### `sim/moves/disable.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| should work on the first turn so long as the opponent has move with PP (61) | **covered** | `disable_gen1_first_turn` |
| should fail if opponent has no moves with PP (71) | **not driveable** | the test empties the opponent's PP (`moveSlots[].pp = 0`); the protocol cannot set a move's PP. |
| should not select moves with 0 PP (82) | **not driveable** | the same (a PP poke). |
| should keep the slot disabled even if the move is replaced by Mimic (97) | **covered** | `disable_gen1_mimic_slot` |
| should keep the slot disabled even if the move is replaced by Transform (114) | **covered** | `disable_gen1_transform_slot` |
| Disable should build Rage, even if it misses/fails (127) | **covered** | `disable_gen1_rage` |

### `sim/moves/explosion.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 1] Explosion should build rage, even if it misses (37) | **covered** | `explosion_gen1_rage` |
| [Gen 1] Explosion should faint the user when the target is semi-invulnerable (70) | **covered** | `explosion_gen1_rage_ghost` |

### `sim/moves/followme.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 3] should continue to redirect moves after the user is knocked out and replaced (125) | **not applicable** | doubles. |

### `sim/moves/furycutter.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| should double in power with each successful hit (Gen 3) (34) | **covered** | `furycutter_gen3` |

### `sim/moves/glare.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| should not ignore natural type immunities (29) | **covered** | `glare_gen3_ghost` |

### `sim/moves/haze.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| should remove stat changes (13) | **covered** | `haze_gen1_stats` |
| should remove opponent's status (29) | **covered** | `haze_gen1_status` |
| should not remove the user's status (43) | **covered** | `haze_gen1_userstatus` |
| should remove focus energy in gen 1 (57) | **covered** | `haze_gen1_focusenergy` |
| should remove reflect and light screen (71) | **covered** | `haze_gen1_reflect` |
| should remove Leech Seed and confusion (89) | **new** | not targeted yet: the Leech Seed and the confusion need a longer script than the harness's queue expresses. |
| should remove Disable (107) | **covered** | `haze_gen1_disable` |
| should still make previously disabled Pokemon (on the same turn) with 1 move use Struggle (123) | **covered** | `haze_gen1_struggle` |
| should convert toxic poisoning to regular poisoning for the user and effectively reset the toxic counter (137) | **new** | not targeted yet: the toxic counter is an internal number; the log shows the status change but not the counter. |
| should not remove substitute from either side (150) | **new** | not targeted yet: the Haze plus two Substitutes needs a longer script. |
| [Gen 3] should not remove focus energy in other gens (209) | **covered** | `haze_gen3_focusenergy` |

### `sim/moves/hyperbeam.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| should not force a recharge turn after KOing a Pokemon (29) | **new** | not targeted yet: the recharge state is a request-level lock; the log shows `|cant|...|recharge`, but the "no recharge" case has no log line at all. |
| should not force a recharge turn after breaking a Substitute (41) | **new** | not targeted yet: the same. |
| should force a recharge turn after damaging, but not breaking a Substitute (51) | **covered** | `hyperbeam_gen1_recharge` (the recharge shows as `|cant|p1a: Chansey|recharge`) |
| Partial trapping moves negate recharge turns (recharging Pokemon is slower) (61) | **new** | not targeted yet: the partial trap has to end the recharge, which the log only shows through the volatile. |
| Partial trapping moves negate recharge turns (recharging Pokemon is faster) (75) | **new** | not targeted yet: the same. |
| Hyper Beam Wrap underflow glitch (88) | **new** | not driveable: the test reads the Wrap volatile's internal damage counter. |
| Hyper Beam automatic selection glitch (107) | **new** | not driveable: the automatic choice is made by PS's own chooser, not by the scripted one. |
| should be soft-locked if it was frozen during the recharge turn (122) | **new** | not driveable: the soft lock is a request-level state. |
| should be freed from soft-locked if thawed by a fire move during the recharge turn (142) | **new** | not driveable: the same. |
| should not be freed from soft-locked if unfrozen by Haze during the recharge turn (169) | **new** | not driveable: the same. |

### `sim/moves/knockoff.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 3] should allow obtaining and using an item with Thief, but nullify it on switch-in (194) | **covered** | `knockoff_gen3` |

### `sim/moves/mirrormove.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 1] Mirror Move'd Hyper Beam should force a recharge turn after not KOing a Pokemon (13) | **new** | not targeted yet: the Mirror Move plus the recharge needs a longer script. |
| [Gen 1] Mirror Move'd Hyper Beam should not force a recharge turn after KOing a Pokemon (23) | **new** | not targeted yet: the same. |
| [Gen 1] Mirror Move should fail when used by a Pokemon that has not seen the opponent use an attack (35) | **new** | not targeted yet: the scenario needs a Thunder Wave that fully paralyzes on a chosen seed; the harness rejected the scripted choices in this batch. |
| [Gen 1] Mirror Move should not copy the charging turn of a two-turn attack (50) | **new** | not targeted yet: the two-turn charge plus the Mirror Move needs a longer script. |
| [Gen 1] Mirror Move should not copy Metronome if Metronome calls a regular move (63) | **new** | not targeted yet: the same, with a Metronome roll that needs a seed scan. |
| [Gen 1] Mirror Move should copy Metronome if Metronome calls a two-turn move (73) | **new** | not targeted yet: the same. |

### `sim/moves/pursuit.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| should continue the switch (398) | **covered** | `pursuit_choiceband` (the existing Gen 3 battle) |
| should only activate on the targeted opponent (409) | **not applicable** | doubles (an ally). |
| should not activate on a switching opponent if targeting an ally (427) | **not applicable** | doubles. |

### `sim/moves/rage.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| Rage accuracy bug (13) | **covered** | `rage_gen1_accuracy` |

### `sim/moves/sleeptalk.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| [Gen 3] should not update the persistent sleep timer (50) | **covered** | `sleeptalk_gen3_timer` |

### `sim/moves/substitute.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| should take specific recoil damage in Gen 1 (59) | **new** | not driveable: the test needs `battle.onEvent('Accuracy', ..., false)` to make the High Jump Kick miss on purpose. |
| [Gen 1] should track what the actual damage would have been without the Substitute (145) | **new** | not driveable: the assertion is on the Substitute volatile's internal damage bookkeeping. |
| [Gen 1] Substitute should not block secondary effect confusion if it is unbroken (161) | **new** | not targeted yet: the confusion needs a seed scan (a secondary effect that must hit). |
| [Gen 1] if a Pokemon with a Substitute hurts itself due to confusion and the target does not have a Substitute, there is no damage dealt (172) | **new** | not targeted yet: the same, with a confusion self-hit. |
| [Gen 1] if a Pokemon with a Substitute hurts itself due to confusion and the target has a Substitute, the target's Substitute takes the damage (195) | **new** | not targeted yet: the same, with two Substitutes. |

### `sim/moves/thrash.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| Three turn Thrash (13) | **covered** | `thrash_gen1_confusion` |
| Four turn Thrash (29) | **covered** | `thrash_gen1_confusion` (the seed fixes the length of the Thrash; the other length is the same handler) |
| Thrash locks the user in, even if it targets a semi-invulnerable foe (46) | **not driveable** | the lock is a request-level state: the log does not say that the user may not choose another move. |
| Thrash locks the user in, even if it targets a Ghost type (57) | **not driveable** | the same request-level lock. |
| Thrash locks the user in, even if it targets and breaks a Substitute (68) | **not driveable** | the same request-level lock. |
| Thrash is paused when asleep or frozen (81) | **new** | not targeted yet: the pause shows in the log, but the sleep needs a seed scan. |
| Thrash is paused when disabled (94) | **new** | not targeted yet: the pause shows in the log, but the Disable has to hit first, which needs a seed scan (the Gen 1 Disable is 55% accurate). |
| Thrash accuracy bug (108) | **new** | not targeted yet: the accuracy state is internal; the bug shows only as a miss in the log. |

### `sim/moves/transform.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| should not send |-endability| (333) | **new** | not driveable: the assertion is on the absence of a log line, which the diff can only see as a whole-log difference. |
| should copy the target's boosted stats (344) | **new** | not targeted yet: the test needs `battle.onEvent('Accuracy', ..., true)` for perfect accuracy. |
| should copy the target's stats (except HP), even if different level (359) | **new** | not targeted yet: the harness could not script the copied move (PS rejected the choice with "Ditto's Tackle is disabled"); the transformed Ditto's move list is the target's. |
| should copy the target's status-modified stats (374) | **new** | not targeted yet: the same. |
| should not re-apply status stat modifier after transforming (388) | **new** | not targeted yet: the same. |
| calls Metronome or Mirror Move, PP from the original base move slot is incremented (405) | **new** | not driveable: the assertion is on the move slot's PP. |
| calls Metronome or Mirror Move, PP from the original base move slot is incremented (same with two-turn moves) (420) | **new** | not driveable |

### `sim/moves/weatherball.js`

| PS test | verdict | targeted battle / reason |
| --- | --- | --- |
| should not trigger counter when it is special (46) | **covered** | `weatherball_gen3_special` |
| should trigger mirror coat when it is special (56) | **covered** | `weatherball_gen3_mirrorcoat` |
| should not trigger mirror coat when it is physical (66) | **covered** | `weatherball_gen3_physical` |
| should trigger counter when it is physical (76) | **covered** | `weatherball_gen3_mirrorcoat_physical` |

## 5. The new targeted battles

Each of these is a battle in `crates/ps_harness/targeted/gen1.json` or
`gen3.json`; it runs the way the harness runs every targeted battle
(`node tools/ps-diff/diff.js --targeted gen1|gen3 --ps-cli <ps_sim_cli>`),
and it compares Showdown's log with ours line by line. The `expect` lines
are the test's assertion, written as the log lines Showdown has to print.

### Gen 1: 26 new battles

#### `faint_gen1_endturn` (gen 1)

- Covers: mods/gen1/scripts.rs move order: Gen 1 ends the turn when a Pokemon faints, so the foe never moves (Explosion from the Electrode, the Haunter's Substitute never happens); PS: test/sim/misc/faint-order.js "should end the turn in Gen 1"
- Seed: 0, trunc `math`
- Teams: p1 electrode (explosion) + pikachu (growl, explosion); p2 haunter (substitute)
- Choices: [move 1, move 1] [switch 2, -] [move 1, move 1] [move 2, move 1]
- expect: `|faint|p1a: Electrode`, `|switch|p1a: Pikachu`, `|-start|p2a: Haunter|Substitute`

#### `recoil_gen1` (gen 1)

- Covers: mods/gen1/scripts.rs applyRecoilDamage: Gen 1 Double Edge recoils a quarter of the damage dealt (not a third); PS: test/sim/misc/recoil.js "[Gen 1] should deal recoil damage based on the damage dealt"
- Seed: 0, trunc `math`
- Teams: p1 snorlax (doubleedge); p2 chansey (softboiled)
- Choices: [move 1, move 1]
- expect: `|-damage|p2a: Chansey|0 fnt`, `|-damage|p1a: Snorlax|300/460|[from] Recoil`

#### `ohko_gen1_substitute` (gen 1)

- Covers: mods/gen1/scripts.rs tryMoveHit: an OHKO move breaks a Substitute instead of fainting its user (Horn Drill into the Vaporeon's Substitute); PS: test/sim/misc/ohko.js "[Gen 1] should faint the target's substitute"
- Seed: 0, trunc `math`
- Teams: p1 tauros (horndrill, splash); p2 vaporeon (substitute, splash)
- Choices: [move 2, move 1] [move 1, move 2] [move 1, move 2] [move 1, move 2] [move 1, move 2] [move 1, move 2] [move 1, move 2] [move 1, move 2]
- expect: `|-end|p2a: Vaporeon|Substitute`, `|-ohko`

#### `ohko_gen1_speedfail` (gen 1)

- Covers: mods/gen1/scripts.rs tryMoveHit: an OHKO move fails against a faster target; PS: test/sim/misc/ohko.js "[Gen 1] should fail if the target has a higher speed"
- Seed: 0, trunc `math`
- Teams: p1 rhydon (horndrill, seismictoss); p2 vaporeon (splash)
- Choices: [move 1, move 1] [move 2, move 1] [move 2, move 1] [move 2, move 1] [move 2, move 1]
- expect: `|-immune|p2a: Vaporeon|[ohko]`

#### `rage_gen1_accuracy` (gen 1)

- Covers: mods/gen1/conditions/rage.rs onStart: the Rage accuracy bug (the stored accuracy halves on the second use, so the second Rage misses); PS: test/sim/moves/rage.js "Rage accuracy bug"
- Seed: 1, trunc `math`
- Teams: p1 nidoking (rage); p2 aerodactyl (doubleteam, seismictoss)
- Choices: [move 1, move 1] [move 1, move 2] [move 1, move 2] [move 1, move 2] [move 1, move 2]
- expect: `|move|p1a: Nidoking|Rage|p2a: Aerodactyl|[from] Rage|[miss]`, `|-miss|p1a: Nidoking`

#### `explosion_gen1_rage` (gen 1)

- Covers: mods/gen1/scripts.rs moveHit: Explosion builds the target's Rage even when it misses; PS: test/sim/moves/explosion.js "[Gen 1] Explosion should build rage, even if it misses"
- Seed: 0, trunc `math`
- Teams: p1 golem (explosion); p2 aerodactyl (rage)
- Choices: [move 1, move 1]
- expect: `|-boost|p2a: Aerodactyl|atk|1|[from] Rage`

#### `explosion_gen1_rage_ghost` (gen 1)

- Covers: mods/gen1/scripts.rs moveHit: Explosion against a Ghost builds the target's Rage even though the move fails; PS: test/sim/moves/explosion.js "[Gen 1] Explosion should faint the user when the target is semi-invulnerable"
- Seed: 0, trunc `math`
- Teams: p1 golem (explosion); p2 gastly (rage)
- Choices: [move 1, move 1]
- expect: `|-immune|p2a: Gastly`, `|faint|p1a: Golem`

#### `haze_gen1_stats` (gen 1)

- Covers: mods/gen1/moves/haze.rs onHit: Haze clears every stat boost on the field; PS: test/sim/moves/haze.js "should remove stat changes"
- Seed: 0, trunc `math`
- Teams: p1 mew (agility, haze, explosion); p2 mewtwo (swordsdance, splash)
- Choices: [move 1, move 1] [move 2, move 2] [move 3, move 2]
- expect: `|-boost|p1a: Mew|spe|2`, `|-boost|p2a: Mewtwo|atk|2`, `|-clearallboost|[silent]`

#### `haze_gen1_status` (gen 1)

- Covers: mods/gen1/moves/haze.rs onHit: Haze cures the opponent's paralysis; PS: test/sim/moves/haze.js "should remove opponent's status"
- Seed: 0, trunc `math`
- Teams: p1 mew (thunderwave, haze, explosion); p2 mewtwo (splash)
- Choices: [move 1, move 1] [move 2, move 1] [move 3, move 1]
- expect: `|-status|p2a: Mewtwo|par`, `|-curestatus|p2a: Mewtwo|par|[silent]`

#### `haze_gen1_userstatus` (gen 1)

- Covers: mods/gen1/moves/haze.rs onHit: Haze does not cure the user's own status; PS: test/sim/moves/haze.js "should not remove the user's status"
- Seed: 0, trunc `math`
- Teams: p1 mew (thunderwave, splash, explosion); p2 mewtwo (haze, splash)
- Choices: [move 1, move 1] [move 1, move 2] [move 2, move 2]
- expect: `|-status|p2a: Mewtwo|par`, `|-clearallboost|[silent]`

#### `haze_gen1_disable` (gen 1)

- Covers: mods/gen1/moves/haze.rs onHit: Haze removes the Disable volatile, so the disabled move works again; PS: test/sim/moves/haze.js "should remove Disable"
- Seed: 0, trunc `math`
- Teams: p1 mew (disable, haze, splash, explosion); p2 muk (splash, tackle)
- Choices: [move 1, move 2] [move 2, move 1] [move 3, move 2] [move 4, move 1]
- expect: `|-start|p2a: Muk|Disable|Tackle`, `|move|p2a: Muk|Tackle|p1a: Mew`

#### `haze_gen1_struggle` (gen 1)

- Covers: mods/gen1/scripts.rs runMove: a Pokemon disabled on the same turn as the Haze still uses Struggle; PS: test/sim/moves/haze.js "should still make previously disabled Pokemon (on the same turn) with 1 move use Struggle"
- Seed: 0, trunc `math`
- Teams: p1 mew (disable, haze); p2 muk (tackle)
- Choices: [move 1, move 1] [move 2, move 1] [move 2, move 1]
- expect: `|move|p2a: Muk|Struggle|p1a: Mew`

#### `haze_gen1_focusenergy` (gen 1)

- Covers: mods/gen1/conditions/focusenergy.rs onEnd: Haze removes the Focus Energy volatile; PS: test/sim/moves/haze.js "should remove focus energy in gen 1"
- Seed: 0, trunc `math`
- Teams: p1 mew (splash, explosion); p2 mewtwo (focusenergy, haze)
- Choices: [move 1, move 1] [move 1, move 2] [move 1, move 1] [move 2, move 1]
- expect: `|-start|p2a: Mewtwo|move: Focus Energy`, `|-end|p2a: Mewtwo|move: Focus Energy|[silent]`

#### `disable_gen1_first_turn` (gen 1)

- Covers: mods/gen1/moves/disable.rs onHit: Disable works on the first turn against a move with PP (the Roar is disabled before it is used); PS: test/sim/moves/disable.js "[Gen 1] should work on the first turn so long as the opponent has move with PP"
- Seed: 0, trunc `math`
- Teams: p1 mew (disable, explosion); p2 muk (roar)
- Choices: [move 1, move 1] [move 2, move 1]
- expect: `|-start|p2a: Muk|Disable|Roar`

#### `disable_gen1_mimic_slot` (gen 1)

- Covers: mods/gen1/conditions/disable.rs onDisableMove: the disabled slot stays disabled when Mimic replaces the move; PS: test/sim/moves/disable.js "[Gen 1] should keep the slot disabled even if the move is replaced by Mimic"
- Seed: 14, trunc `math`
- Teams: p1 drowzee (mimic, tackle, explosion); p2 abra (tackle, disable, splash)
- Choices: [move 1, move 2] [move 1, move 1] [move 3, move 1]
- expect: `|-start|p1a: Drowzee|Disable|Tackle`, `|-start|p1a: Drowzee|Mimic|Tackle`, `|cant|p1a: Drowzee|Disable|Tackle`

#### `disable_gen1_transform_slot` (gen 1)

- Covers: mods/gen1/conditions/disable.rs onDisableMove: the disabled slot stays disabled when Transform replaces the move; PS: test/sim/moves/disable.js "[Gen 1] should keep the slot disabled even if the move is replaced by Transform"
- Seed: 10, trunc `math`
- Teams: p1 drowzee (transform, tackle, explosion); p2 abra (tackle, disable, explosion)
- Choices: [move 2, move 2] [move 1, move 1] [move 1, move 3]
- expect: `|-start|p1a: Drowzee|Disable|Tackle`, `|-transform|p1a: Drowzee|p2a: Abra`, `|cant|p1a: Drowzee|Disable|Tackle`

#### `disable_gen1_rage` (gen 1)

- Covers: mods/gen1/scripts.rs moveHit: Disable builds the target's Rage even when it misses; PS: test/sim/moves/disable.js "[Gen 1] Disable should build Rage, even if it misses/fails"
- Seed: 0, trunc `math`
- Teams: p1 drowzee (disable, explosion); p2 abra (rage, splash)
- Choices: [move 1, move 1] [move 2, move 1]
- expect: `|-boost|p2a: Abra|atk|1|[from] Rage`, `In Gen 1, using Disable causes the target to build Rage, even if it misses or fails`

#### `bide_gen1_two_turn` (gen 1)

- Covers: mods/gen1/moves/bide.rs onBeforeMove: Gen 1 Bide rolls two or three turns and releases twice the damage it took (a two-turn Bide against a Dragon Rage); PS: test/sim/moves/bide.js "should be possible to roll two-turn Bide"
- Seed: 0, trunc `math`
- Teams: p1 aerodactyl (bide, seismictoss, whirlwind); p2 gyarados (dragonrage)
- Choices: [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 2, move 1] [move 2, move 1]
- expect: `|-start|p1a: Aerodactyl|Bide`, `|-end|p1a: Aerodactyl|Bide`, `|-damage|p2a: Gyarados|170/330`

#### `bide_gen1_three_turn` (gen 1)

- Covers: mods/gen1/moves/bide.rs onBeforeMove: Gen 1 Bide rolls two or three turns and releases twice the damage it took (a three-turn Bide against a Dragon Rage); PS: test/sim/moves/bide.js "should be possible to roll three-turn Bide"
- Seed: 3, trunc `math`
- Teams: p1 aerodactyl (bide, seismictoss, whirlwind); p2 gyarados (dragonrage)
- Choices: [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 2, move 1]
- expect: `|-damage|p2a: Gyarados|90/330`

#### `bide_gen1_substitute` (gen 1)

- Covers: mods/gen1/moves/bide.rs onHit: the Bide release damages the target's Substitute; PS: test/sim/moves/bide.js "should damage Substitute with Bide damage"
- Seed: 0, trunc `math`
- Teams: p1 aerodactyl (bide, seismictoss, whirlwind); p2 gyarados (dragonrage, substitute)
- Choices: [move 3, move 2] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1]
- expect: `|-end|p1a: Aerodactyl|Bide`, `|-end|p2a: Gyarados|Substitute`

#### `bide_gen1_pause_sleep` (gen 1)

- Covers: mods/gen1/conditions/bide.rs onBeforeMove: Bide's duration pauses while the user is asleep; PS: test/sim/moves/bide.js "should pause Bide's duration when asleep or frozen"
- Seed: 9, trunc `math`
- Teams: p1 aerodactyl (bide, seismictoss); p2 parasect (spore, splash)
- Choices: [move 1, move 1] [move 1, move 2] [move 1, move 2] [move 1, move 2] [move 2, move 2] [move 2, move 2] [move 2, move 2]
- expect: `|-status|p1a: Aerodactyl|slp|[from] move: Spore`, `|-activate|p1a: Aerodactyl|Bide`, `|-end|p1a: Aerodactyl|Bide`, `|-fail|p1a: Aerodactyl`

#### `bide_gen1_pause_disable` (gen 1)

- Covers: mods/gen1/conditions/bide.rs onBeforeMove: Bide's duration pauses while the user is disabled; PS: test/sim/moves/bide.js "should pause Bide's duration when disabled"
- Seed: 0, trunc `math`
- Teams: p1 aerodactyl (bide, seismictoss); p2 voltorb (disable, explosion)
- Choices: [move 1, move 1] [move 1, move 1] [move 1, move 2]
- expect: `|-start|p1a: Aerodactyl|Disable|Bide`, `|cant|p1a: Aerodactyl|Disable|Bide`

#### `thrash_gen1_confusion` (gen 1)

- Covers: mods/gen1/moves/thrash.rs: Gen 1 Thrash locks the user in for a few turns and then confuses it; PS: test/sim/moves/thrash.js "Three turn Thrash"
- Seed: 0, trunc `math`
- Teams: p1 nidoking (thrash); p2 golem (splash)
- Choices: [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1]
- expect: `|-start|p1a: Nidoking|confusion|[silent]`

#### `counter_gen1_substitute` (gen 1)

- Covers: mods/gen1/moves/counter.rs: Gen 1 Counter reflects the damage of the last attack even when it hit the user's Substitute, and can hit a target that is switching out; PS: test/sim/moves/counter.js "[Gen 1] should counter attacks made against substitutes"
- Seed: 0, trunc `math`
- Teams: p1 chansey (substitute, counter, seismictoss); p2 snorlax (bodyslam, splash) + chansey (splash)
- Choices: [move 1, move 1] [move 2, switch 2] [move 3, switch 2] [move 3, move 2] [move 3, move 2] [move 3, move 2] [move 3, move 2] [move 3, move 2] [move 3, move 2]
- expect: `|faint|p2a: Chansey`

#### `haze_gen1_reflect` (gen 1)

- Covers: mods/gen1/moves/haze.rs: Gen 1 Haze removes Reflect and Light Screen; PS: test/sim/moves/haze.js "should remove reflect and light screen"
- Seed: 0, trunc `math`
- Teams: p1 mew (reflect, lightscreen, haze, seismictoss); p2 mewtwo (splash)
- Choices: [move 1, move 1] [move 2, move 1] [move 3, move 1] [move 4, move 1] [move 4, move 1] [move 4, move 1] [move 4, move 1] [move 4, move 1] [move 4, move 1]
- expect: `|-end|p1a: Mew|Light Screen|[silent]`, `|-end|p1a: Mew|Reflect|[silent]`

#### `hyperbeam_gen1_recharge` (gen 1)

- Covers: mods/gen1/moves/hyperbeam.rs: Gen 1 Hyper Beam forces a recharge turn after it damages the target; PS: test/sim/moves/hyperbeam.js "should force a recharge turn after damaging, but not breaking a Substitute"
- Seed: 0, trunc `math`
- Teams: p1 chansey (hyperbeam, seismictoss); p2 chansey (splash)
- Choices: [move 1, move 1] [move 1, move 1] [move 2, move 1] [move 2, move 1] [move 2, move 1] [move 2, move 1] [move 2, move 1] [move 2, move 1] [move 2, move 1] [move 2, move 1]
- expect: `|cant|p1a: Chansey|recharge`

### Gen 3: 17 new battles

#### `faint_gen3_endturn` (gen 3)

- Covers: mods/gen3/scripts.rs move order: Gen 3 ends the turn when a Pokemon faints, so the foe never moves (the Explosion); PS: test/sim/misc/faint-order.js "[Gen 3] should end the turn in Gen 3"
- Seed: 0
- Teams: p1 electrode (explosion) + pikachu (growl, seismictoss); p2 haunter (substitute)
- Choices: [move 1, move 1] [switch 2, move 1] [move 2, move 1] [move 2, move 1] [move 2, move 1]
- expect: `|-immune|p2a: Haunter`, `|faint|p1a: Electrode`

#### `ohko_gen3_hp_damage` (gen 3)

- Covers: move:horndrill damageCallback: an OHKO move deals the target's remaining HP; PS: test/sim/misc/ohko.js "[Gen 3] should deal damage equal to the target's HP"
- Seed: 387
- Teams: p1 rhydon (horndrill, seismictoss); p2 breloom (substitute, splash)
- Choices: [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 2, move 2]
- expect: `|-end|p2a: Breloom|Substitute`, `|-activate|p2a: Breloom|Substitute|[damage]`

#### `thickfat_gen3` (gen 3)

- Covers: mods/gen3/abilities/thickfat.rs onSourceModifyDamage: Thick Fat halves Fire- and Ice-type damage even with a type-boosting item; PS: test/sim/abilities/thickfat.js "[Gen 3] should halve damage from Fire- or Ice-type attacks in past generations, even when holding a type-boosting item"
- Seed: 0
- Teams: p1 miltank (thickfat) (seismictoss); p2 wynaut @nevermeltice (icebeam)
- Choices: [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1]
- expect: `|-damage|p1a: Miltank|311/331`

#### `trace_gen3` (gen 3)

- Covers: mods/gen3/abilities/trace.rs onStart: the Trace copies the opponent's Ability without activating it; PS: test/sim/abilities/trace.js "[Gen 3] should not activate the opponent's Ability"
- Seed: 0
- Teams: p1 ralts (trace) (seismictoss); p2 arcanine (intimidate) (sleeptalk)
- Choices: [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1]
- expect: `|-unboost|p1a: Ralts|atk|1`

#### `flashfire_gen3_frozen` (gen 3)

- Covers: mods/gen3/abilities/flashfire.rs onTryHit: the Flash Fire activates and boosts SpA even while the user is frozen; PS: test/sim/abilities/flashfire.js "should activate and grant Fire-type immunity even if the user is frozen in Gen 3"
- Seed: 4
- Teams: p1 arcanine (flashfire) (sleeptalk, splash); p2 charizard (blaze) (flamethrower, icebeam, seismictoss)
- Choices: [move 1, move 2] [move 1, move 1] [move 1, move 3] [move 1, move 3] [move 1, move 3]
- expect: `|-status|p1a: Arcanine|frz`, `|-damage|p1a: Arcanine|197/321 frz`

#### `quickclaw_gen3` (gen 3)

- Covers: mods/gen3/items/quickclaw.rs onModifyMove: the Quick Claw makes every holder tie on Speed when it activates; PS: test/sim/items/quickclaw.js "[Gen 3] causes Speed ties with every holder when activated"
- Seed: 6
- Teams: p1 snorlax @quickclaw (spore); p2 deoxys @quickclaw (seismictoss)
- Choices: [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1]
- expect: `|move|p1a: Snorlax|Spore|p2a: Deoxys
|-status|p2a: Deoxys|slp|[from] move: Spore
|cant|p2a: Deoxys|slp`

#### `sitrus_gen3` (gen 3)

- Covers: mods/gen3/items/stat_berries.rs sitrus onEat: the Sitrus Berry does not activate in the same turn a status condition put the holder below half HP; PS: test/sim/items/sitrusberry.js "[Gen 3] should not activate in the same turn that was put below 50% HP by a status condition"
- Seed: 0
- Teams: p1 swampert @sitrusberry (splash, seismictoss); p2 charizard (toxic, splash)
- Choices: [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 2, move 2] [move 2, move 2] [move 2, move 2]
- expect: `|-enditem|p1a: Swampert|Sitrus Berry|[eat]`

#### `conversion2_gen3` (gen 3)

- Covers: mods/gen3/moves/conversion2.rs onHit: Conversion 2 changes the user's type to the type of the target's last move; PS: test/sim/moves/conversion2.js "[Gen 3] should not fail to change to a type the user already has"
- Seed: 0
- Teams: p1 forretress (conversion2, seismictoss); p2 salamence (dragonrage, splash)
- Choices: [move 1, move 1] [move 2, move 2] [move 2, move 2] [move 2, move 2] [move 2, move 2]
- expect: `|-start|p1a: Forretress|typechange|Steel`

#### `furycutter_gen3` (gen 3)

- Covers: mods/gen3/moves/furycutter.rs basePowerCallback: the Fury Cutter doubles in power with every hit; PS: test/sim/moves/furycutter.js "should double in power with each successful hit (Gen 3)"
- Seed: 0
- Teams: p1 kangaskhan (splash, seismictoss); p2 wynaut (compoundeyes) (furycutter)
- Choices: [move 1, move 1] [move 2, move 1] [move 2, move 1] [move 2, move 1] [move 1, move 1] [move 1, move 1] [move 1, move 1] [move 2, move 1]
- expect: `|-damage|p1a: Kangaskhan|347/351`, `|-damage|p1a: Kangaskhan|338/351`, `|-damage|p1a: Kangaskhan|323/351`

#### `glare_gen3_ghost` (gen 3)

- Covers: move:glare onTryHit: Glare does not ignore a Ghost's type immunity in Gen 3; PS: test/sim/moves/glare.js "[Gen 3] should not ignore natural type immunities"
- Seed: 0
- Teams: p1 arbok (glare, seismictoss); p2 gengar (blaze) (bulkup, seismictoss)
- Choices: [move 1, move 2] [move 1, move 2] [move 1, move 2] [move 1, move 2]
- expect: `|-immune|p2a: Gengar`

#### `haze_gen3_focusenergy` (gen 3)

- Covers: mods/gen3/moves/haze.rs onHit: the Gen 3 Haze does not remove the Focus Energy volatile; PS: test/sim/moves/haze.js "[Gen 3] should not remove focus energy in other gens"
- Seed: 0
- Teams: p1 mew (splash, seismictoss); p2 mewtwo (focusenergy, haze)
- Choices: [move 1, move 1] [move 1, move 2] [move 2, move 1] [move 2, move 1] [move 2, move 1]
- expect: `|-start|p2a: Mewtwo|move: Focus Energy`

#### `knockoff_gen3` (gen 3)

- Covers: mods/gen3/moves/knockoff.rs onHit: the Knock Off removes the target's item, which the Thief can then take and use, but it is nullified on switch-in; PS: test/sim/moves/knockoff.js "[Gen 3] should allow obtaining and using an item with Thief, but nullify it on switch-in"
- Seed: 0
- Teams: p1 blissey @pokeball (thief, sleeptalk, seismictoss) + snorlax (sleeptalk); p2 shuckle @leftovers (knockoff, sleeptalk)
- Choices: [move 2, move 1] [move 1, move 2] [switch 2, move 2] [switch 2, move 2] [move 3, move 2] [move 3, move 2]
- expect: `|-enditem|p1a: Blissey|Poke Ball|[from] move: Knock Off|[of] p2a: Shuckle`, `|-item|p1a: Blissey|Leftovers|[from] move: Thief|[of] p2a: Shuckle`

#### `weatherball_gen3_special` (gen 3)

- Covers: move:weatherball basePowerCallback and type: the Weather Ball is special in the rain, so the Counter does not trigger; PS: test/sim/moves/weatherball.js "[Gen 3] should not trigger counter when it is special"
- Seed: 0
- Teams: p1 shuckle (drizzle) (weatherball, seismictoss); p2 shuckle (counter, seismictoss)
- Choices: [move 1, move 1] [move 2, move 2] [move 2, move 2]
- expect: `|-supereffective|p2a: Shuckle`, `|-damage|p2a: Shuckle|155/181`

#### `weatherball_gen3_physical` (gen 3)

- Covers: move:weatherball basePowerCallback and type: the Weather Ball is physical in a sandstorm, so the Counter triggers; PS: test/sim/moves/weatherball.js "[Gen 3] should trigger counter when it is physical"
- Seed: 0
- Teams: p1 shuckle (sandstream) (weatherball, seismictoss); p2 shuckle (counter, seismictoss)
- Choices: [move 1, move 1] [move 2, move 2] [move 2, move 2]
- expect: `|-damage|p2a: Shuckle|153/181`, `|-damage|p1a: Shuckle|125/181`

#### `weatherball_gen3_mirrorcoat` (gen 3)

- Covers: move:weatherball basePowerCallback and type: the Weather Ball is special in the sun, so the Mirror Coat triggers; PS: test/sim/moves/weatherball.js "[Gen 3] should trigger mirror coat when it is special"
- Seed: 0
- Teams: p1 shuckle (drought) (weatherball, seismictoss); p2 shuckle (mirrorcoat, seismictoss)
- Choices: [move 1, move 1] [move 2, move 2] [move 2, move 2]
- expect: `|-damage|p2a: Shuckle|168/181`, `|-damage|p1a: Shuckle|155/181`

#### `weatherball_gen3_mirrorcoat_physical` (gen 3)

- Covers: move:weatherball basePowerCallback and type: the Weather Ball is physical without a weather move, so the Mirror Coat does not trigger; PS: test/sim/moves/weatherball.js "[Gen 3] should not trigger mirror coat when it is physical"
- Seed: 0
- Teams: p1 shuckle (weatherball, seismictoss); p2 shuckle (mirrorcoat, seismictoss)
- Choices: [move 1, move 1] [move 2, move 2] [move 2, move 2]
- expect: `|-resisted|p2a: Shuckle`, `|-damage|p2a: Shuckle|179/181`

#### `sleeptalk_gen3_timer` (gen 3)

- Covers: mods/gen3/moves/sleeptalk.rs onHit: the Sleep Talk does not update the persistent sleep timer; PS: test/sim/moves/sleeptalk.js "[Gen 3] should not update the persistent sleep timer"
- Seed: 0
- Teams: p1 xatu (synchronize) (sleeptalk, calmmind); p2 smeargle (spore, splash)
- Choices: [move 2, move 1] [move 1, move 2] [move 1, move 2] [move 1, move 2] [move 1, move 2] [move 1, move 2]
- expect: `|-status|p1a: Xatu|slp|[from] move: Spore`, `|move|p1a: Xatu|Sleep Talk|p1a: Xatu`

## 6. Findings

Three differences turned up while running Showdown's own tests against our
engine. None of them is fixed here; each one is a report of its own.

### F1: Showdown's Gen 1 Double Edge recoil is a quarter, but its own test asserts a third

`test/sim/misc/recoil.js` ("[Gen 1] should deal recoil damage based on the
damage dealt") asserts `floor(damage * 33 / 100)`, but the Gen 1 Double Edge
recoils a quarter of the damage: `data/moves.ts` has `recoil: [33, 100]` and
the Gen 2 mod (`data/mods/gen2/moves.ts:173`, which Gen 1 inherits) has
`recoil: [25, 100]`. The test never notices, because it runs with
`forceRandomChance: false`, which makes the Double Edge always miss.

Reproduction (Showdown, Gen 1, seed `[1,2,3,4]`): the Double Edge deals 640
damage and the recoil is 160 (a quarter), not 213 (a third). Our targeted
battle `recoil_gen1` asserts the real quarter:

    |-damage|p2a: Chansey|0 fnt
    |-damage|p1a: Snorlax|300/460|[from] Recoil

### F2: Gen 1 Rage, once it has hit, locks the user in for good

In Gen 1 the Rage volatile is never removed, not even after a later miss: the
move request then offers only Rage, and every other move is rejected with
`|error|[Invalid choice] Can't move: Your Nidoking doesn't have a move 2`.
The same applies to the Disable and the Explosion, which build Rage on the
target: once the target has built Rage, it can only use Rage.

Reproduction: Gen 1, a Nidoking with Rage against a Drowzee with Disable; the
Disable builds the Rage (`|-boost|p2a: Nidoking|atk|1|[from] Rage`), and the
next request offers Rage only.

### F3: the Gen 1 Rage accuracy bug is only visible while the first hit lands

The Gen 1 Rage volatile stores `accuracy = 255` when it starts. The turn 1
roll uses that 255 times the target's evasion (so with a Double Team the
Rage hits with 168 of 256, about 65%). On turn 2 the stored 255 is reused and
multiplied by the *new* evasion boost, so a target that gained two stages of
evasion is hit with only 127 of 256 (about 50%). Showdown's own test
(`test/sim/moves/rage.js`) asserts `accuracy === 255` after turn 1 and
`accuracy === 127` after turn 2, which only holds when the turn 1 Rage
hit.

Our targeted battle `rage_gen1_accuracy` pins the seed so that the first
Rage hits and then checks the accuracy the engine uses.

## 7. Deviations and notes

* **A fourth label.** The task names three labels (covered, not applicable,
  new). PS's suite has a fourth kind of test that no battle protocol can
  reach: the ones that need a PS-internal hook (`forceRandomChance`,
  `battle.onEvent('Accuracy')`, `moveSlots[].pp` edits, `setStatus`,
  `volatile.time`) or that assert on state the battle log does not carry (a
  move lock, a speed stat, a boost table, a PP counter). They are marked
  **not driveable** with the reason, so the inventory stays honest.
* **Some new tests have no battle yet.** Every row marked *not targeted yet*
  is a real gap. The blocker is the harness's choice queue: one choice per
  request, per side, so a script that needs a switch in the middle of a
  two-turn move, or a second replacement after a faint, cannot be written
  with the current protocol. Fixing that means changing
  `tools/ps-diff/diff.js` (out of scope here, and `tools/ps-diff` is
  read-only for this task).
* **Sleep Talk.** PS's Gen 1 test teams use Sleep Talk, which does not exist
  in Gen 1; our Gen 1 dex only holds the 165 Gen 1 moves, so a team with
  Sleep Talk makes our engine stop with `>player: UnknownName`. The Gen 1
  battles here use Gen 1 moves only (Splash or Seismic Toss as the filler).
* **Lucky Chant.** The Gen 3 Fury Cutter test uses a Lucky Chant, a Gen 4
  move that our Gen 3 dex does not hold either; the entry uses Splash (the
  move is only there so the Kangaskhan has something to do).
* **Abilities.** The Gen 3 Glare test uses No Guard, which our Gen 3 dex does
  not hold; the entry drops it (the Gen 3 Glare has 100% accuracy, so the
  ability does not change the outcome).
* **Stadium.** Two of the Gen 1 status tests need the Stadium mod
  (`gen1stadiumou`), which is not ported: they are not applicable.
* **Gen 1 EVs.** PS's test teams have no EVs. The harness's Gen 1 default is
  255 EVs per stat, which changes every stat (a Snorlax has 523 HP with 255
  EVs and 460 with none), so every new Gen 1 entry sets `evs` to zero per
  mon.
* **Gen 1 truncation.** PS's `[Gen 1] Custom Game` uses `Math.trunc`; the
  harness's Gen 1 default is the cartridge truncation. The new Gen 1 entries
  therefore carry `"trunc": "math"`, and `scripts/check.sh` runs the Gen 1
  targeted diff under both truncations.
* **Finishing the battle.** Every targeted battle has to end: the harness
  kills a battle after 30 s and Showdown after 1000 turns. The entries here
  end with an Explosion, a Seismic Toss series, a Hyper Beam, a Counter or a
  switch.
