//! Gen 3's move pipeline: PS's `data/mods/gen3/scripts.ts` over
//! `data/mods/gen4/scripts.ts` over `sim/battle-actions.ts`, ported with
//! its control flow.
//!
//! Gen 3 overrides `useMoveInner`, `tryMoveHit` and `modifyDamage`
//! (actions), `getActionSpeed` (pokemon, in `stats.rs`) and `checkFainted`
//! (battle, [`check_fainted`]). From gen 4 it inherits
//! `applyRecoilDamage` (`actions.rs`)
//! and gen 4's `modifyDamage`, `hitStepAccuracy` and
//! `hitStepInvulnerabilityEvent`, which gen 3's own overrides replace or
//! never reach: gen 3's `useMoveInner` calls its `tryMoveHit` for every
//! target type, so base `trySpreadMoveHit` and its `hitStep*` steps run
//! only for a future move's hit (`futuremove`'s `onEnd`,
//! [`Gen3Scripts::try_spread_move_hit`]). Gens 5 to 8 override no actions. Everything else is base
//! (`crate::base_actions`): `runMove`, `useMove`, `moveHit` with
//! `spreadMoveHit`, `getDamage`, `deductPP`, `boost`, `clearBoosts`.
//!
//! Gen 3's `init` (every damaging move's category by its type) is applied
//! in the dex dump (`Dex.mod('gen3')` resolves it).
//!
//! An effect whose handlers are not ported panics `unported: <id>` when the
//! pipeline reaches it ([`Battle::require_ported`]).
//!
//! PS: data/mods/gen3/scripts.ts `useMoveInner`, `tryMoveHit`,
//! `modifyDamage`, `checkFainted`.

use ps_dex::gen3::{abilities, conditions};
use ps_dex::{MoveCategory, MoveId, MoveTarget, Type};
use ps_protocol::{Kw, KwFlags, MoveNum, Msg};

use crate::actions::{GenScripts, MoveData, MoveRef, MoveResult};
use crate::base_actions::DamageStep;
use crate::effect::{EffectId, EventId};
use crate::queue::ActionMove;
use crate::relay::Relay;
use crate::state::{Battle, Holder, MonRef, StateRef};

/// Gen 3's `checkFainted`: replacements can be requested while other
/// switches are still queued, so a fainted active with a queued switch-out
/// is left alone; when only one side has fainted actives, one at a time.
// PS: data/mods/gen3/scripts.ts checkFainted
pub fn check_fainted(b: &mut Battle) {
    let mut candidates = [[None::<MonRef>; 1]; 2];
    let mut sides_with = 0;
    for s in 0..2u8 {
        if b.can_switch(s) == 0 {
            continue;
        }
        let mut found = false;
        for pos in 0..b.sides[s as usize].active.len() as u8 {
            let Some(m) = b.active_at(s, pos) else { continue };
            if b.mon(m).fainted && !b.queue.get_switches(None).any(|a| a.pokemon == Some(m)) {
                candidates[s as usize][pos as usize] = Some(m);
                found = true;
            }
        }
        if found {
            sides_with += 1;
        }
    }
    let one_per_side = sides_with == 1;
    for side in &candidates {
        for &m in side.iter().flatten() {
            let mon = b.mon_mut(m);
            mon.status = ps_dex::ConditionId::NONE;
            mon.switch_flag = crate::state::SwitchFlag::Yes;
            if one_per_side {
                break;
            }
        }
    }
}

/// Gen 3's scripts. See the module doc.
pub struct Gen3Scripts;

/// `tryMoveHit`'s `boostTable`: accuracy and evasion stages 0 to 6.
const BOOST_TABLE: [f64; 7] = [1.0, 4.0 / 3.0, 5.0 / 3.0, 2.0, 7.0 / 3.0, 8.0 / 3.0, 3.0];

impl Battle {
    /// Runs an event whose relay is a JS number that may carry a fraction
    /// (`move.accuracy` after the stage multipliers, the STAB `1.5`), or
    /// `true` (`None`). The number travels as a float relay, so PS's final
    /// modifier reaches it only when it is a whole number (PS:
    /// `relayVar === Math.abs(Math.floor(relayVar))`).
    fn number_event(
        &mut self,
        event: EventId,
        target: MonRef,
        source: Option<MonRef>,
        effect: EffectId,
        value: Option<f64>,
        priority: bool,
    ) -> Option<f64> {
        let relay = match value {
            None => Relay::TRUE,
            Some(v) => Relay::float(v),
        };
        let r = if priority {
            self.priority_event(event, Holder::Mon(target), source, effect, Some(relay), false)
        } else {
            self.run_event(event, Holder::Mon(target), source, effect, Some(relay))
        };
        if r == relay {
            return value;
        }
        match r {
            Relay::TRUE => None,
            Relay::Float(bits) => Some(f64::from_bits(bits)),
            Relay::Int(n) if value.is_none_or(|v| v == (v as i64) as f64) => Some(f64::from(n)),
            other => panic!("unported: {event:?} changed a fractional relay ({other:?})"),
        }
    }

    /// `this.battle.runEvent(event, ...)` of a damage number.
    fn damage_event(&mut self, event: EventId, pokemon: MonRef, target: MonRef, effect: EffectId, value: i64, priority: bool) -> i64 {
        let relay = Some(Relay::Int(value as i32));
        let r = if priority {
            self.priority_event(event, Holder::Mon(pokemon), Some(target), effect, relay, false)
        } else {
            self.run_event(event, Holder::Mon(pokemon), Some(target), effect, relay)
        };
        match r {
            Relay::Int(n) => i64::from(n),
            other => panic!("{event:?} answered {other:?}"),
        }
    }

    /// `move.accuracy` with the user's accuracy and the target's evasion
    /// stages (`ModifyBoost` first): `tryMoveHit`'s block, also run again
    /// for each `multiaccuracy` hit.
    fn gen3_stage_accuracy(&mut self, pokemon: MonRef, target: MonRef, mv: MoveRef) -> Option<f64> {
        let mut accuracy = self.mv(mv).accuracy_x256.map(|a| f64::from(a) / 256.0)?;
        let def = self.dex.move_def(self.mv(mv).id);
        if !def.bools.has("ignoreAccuracy") {
            // `runEvent('ModifyBoost', pokemon, null, null, {...boosts})`.
            let boosts = self.mon(pokemon).boosts;
            let boost = self.modify_boost_event(pokemon, boosts)[5].clamp(-6, 6);
            if boost > 0 {
                accuracy *= BOOST_TABLE[boost as usize];
            } else {
                accuracy /= BOOST_TABLE[(-boost) as usize];
            }
        }
        if !def.bools.has("ignoreEvasion") {
            let boosts = self.mon(target).boosts;
            let boost = self.modify_boost_event(target, boosts)[6].clamp(-6, 6);
            if boost > 0 {
                accuracy /= BOOST_TABLE[boost as usize];
            } else if boost < 0 {
                accuracy *= BOOST_TABLE[(-boost) as usize];
            }
        }
        Some(accuracy)
    }

    /// `this.battle.randomChance(accuracy, 100)` of a JS number:
    /// `random(100) < accuracy`.
    fn accuracy_roll(&mut self, accuracy: f64) -> bool {
        f64::from(self.prng.random_n(100)) < accuracy
    }

    /// Gen 3's `modifyDamage`: burn, the phase 1 modifiers, weather, `+ 2`,
    /// the crit, phase 2, STAB, type effectiveness, the final modifier and
    /// the random factor last.
    // PS: data/mods/gen3/scripts.ts modifyDamage
    fn gen3_modify_damage(&mut self, base_damage: i64, pokemon: MonRef, target: MonRef, mv: MoveRef, suppress: bool) -> MoveResult {
        // `if (!move.type) move.type = '???'`: `Type::None` is `???`.
        let move_type = self.mv(mv).type_;
        let category = self.mv(mv).category;
        let effect = self.mv(mv).effect();
        let mut base_damage = base_damage;

        // Burn.
        if self.mon(pokemon).status == conditions::BRN
            && base_damage != 0
            && category == MoveCategory::Physical
            && !self.has_ability(pokemon, abilities::GUTS)
        {
            base_damage = self.modify_f(base_damage, 0.5);
        }

        // Other modifiers (Reflect, Light Screen, ...).
        base_damage = self.damage_event(EventId::ModifyDamagePhase1, pokemon, target, effect, base_damage, false);

        // The double battle spread modifier: `spreadHit` is never set in
        // singles.

        // Weather.
        base_damage = self.damage_event(EventId::WeatherModifyDamage, pokemon, target, effect, base_damage, true);

        if category == MoveCategory::Physical && base_damage == 0 {
            base_damage = 1;
        }

        base_damage += 2;

        let is_crit = self.mv(mv).crit;
        if is_crit {
            // `move.critModifier || 2`: no gen 3 move has a `critModifier`.
            base_damage = self.modify(base_damage, 2, 1);
        }

        // Phase 2 (floored after all the multipliers).
        base_damage = self.damage_event(EventId::ModifyDamagePhase2, pokemon, target, effect, base_damage, false);

        // STAB: `???` never gets it.
        if move_type != Type::None {
            let force_stab = self.dex.move_def(self.mv(mv).id).bools.has("forceSTAB");
            let stab = if force_stab || self.has_type(pokemon, move_type) { 1.5 } else { 1.0 };
            let stab = self.number_event(EventId::ModifySTAB, pokemon, Some(target), effect, Some(stab), false).unwrap_or(1.0);
            base_damage = self.modify_f(base_damage, stab);
        }

        // Types.
        let type_mod = self.run_effectiveness(target, mv).clamp(-6, 6);
        self.mv_mut(mv).type_mod = type_mod as i8;
        if type_mod > 0 {
            if !suppress {
                let mon = self.ident(target);
                self.add(Msg::SuperEffective(mon));
            }
            for _ in 0..type_mod {
                base_damage *= 2;
            }
        }
        if type_mod < 0 {
            if !suppress {
                let mon = self.ident(target);
                self.add(Msg::Resisted(mon));
            }
            for _ in type_mod..0 {
                base_damage /= 2;
            }
        }

        if is_crit && !suppress {
            let mon = self.ident(target);
            self.add(Msg::Crit(mon));
        }

        // The final modifier.
        base_damage = self.damage_event(EventId::ModifyDamage, pokemon, target, effect, base_damage, false);

        // Not a modifier.
        base_damage = self.randomizer(base_damage);

        if base_damage == 0 {
            return MoveResult::Int(1);
        }
        MoveResult::Int(base_damage as i32)
    }
}

impl Gen3Scripts {
    /// Base `trySpreadMoveHit` with gen 3's step order and gen 4's
    /// `hitStepInvulnerabilityEvent` and `hitStepAccuracy`, which gen 3
    /// inherits: the only caller in gen 3 is a future move's hit
    /// (`futuremove`'s `onEnd`, `notActive` true), since gen 3's
    /// `useMoveInner` calls its own `tryMoveHit`. One target (singles).
    // PS: sim/battle-actions.ts trySpreadMoveHit
    pub fn try_spread_move_hit(b: &mut Battle, target: MonRef, pokemon: MonRef, mv: MoveRef, not_active: bool) -> bool {
        // One target: no `spreadHit`.
        if not_active {
            b.set_active_move(Some(mv), Some(pokemon), Some(target));
        }
        let effect = b.mv(mv).effect();
        let own = b.mv(mv).handler_effect();

        let mut hit_result = b.single_event(EventId::Try, own, StateRef::Scratch, 0, Holder::Mon(pokemon), Some(target), effect, None);
        if hit_result.truthy() {
            hit_result =
                b.single_event(EventId::PrepareHit, own, StateRef::Scratch, 0, Holder::Mon(target), Some(pokemon), effect, None);
        }
        if hit_result.truthy() {
            hit_result = b.run_event(EventId::PrepareHit, Holder::Mon(pokemon), Some(target), effect, None);
        }
        if !hit_result.truthy() {
            if hit_result == Relay::FAIL {
                let mon = b.ident(pokemon);
                b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
                b.attr_last_move_still();
            }
            return hit_result == Relay::NotFail;
        }

        // Gens 5 and earlier swap steps 1 and 2 (type immunity before
        // `TryHit`); gen 4's further swap is gen 4 only.
        let steps: [fn(&mut Battle, MonRef, MonRef, MoveRef) -> MoveResult; 6] = [
            Self::hit_step_invulnerability_event,
            Self::hit_step_type_immunity,
            Self::hit_step_try_hit_event,
            Self::hit_step_try_immunity,
            Self::hit_step_accuracy,
            // `hitStepBreakProtect` and `hitStepStealBoosts`: no gen 3 move
            // `breaksProtect` or `stealsBoosts`.
            Self::hit_step_move_hit_loop,
        ];
        let mut targets = Some(target);
        let mut at_least_one_failure = false;
        for step in steps {
            let Some(t) = targets else { break };
            let hit_result = step(b, t, pokemon, mv);
            if !(hit_result.truthy() || hit_result == MoveResult::Int(0)) {
                targets = None;
            }
            at_least_one_failure = at_least_one_failure || hit_result == MoveResult::False;
            // No `smartTarget` in gen 3.
        }

        // `move.hitTargets` is read by nothing in gen 3.
        let move_result = targets.is_some();
        if !move_result && !at_least_one_failure {
            b.mon_mut(pokemon).move_this_turn_result = Some(Relay::Null);
        }
        move_result
    }

    /// Gen 4's `hitStepInvulnerabilityEvent`.
    // PS: data/mods/gen4/scripts.ts hitStepInvulnerabilityEvent
    fn hit_step_invulnerability_event(b: &mut Battle, target: MonRef, pokemon: MonRef, mv: MoveRef) -> MoveResult {
        let effect = b.mv(mv).effect();
        let hit_result = b.run_event(EventId::Invulnerability, Holder::Mon(target), Some(pokemon), effect, None);
        if hit_result == Relay::FAIL {
            b.attr_last_move(KwFlags::MISS);
            let source = b.ident(pokemon);
            let tgt = b.ident(target);
            b.add(Msg::Miss { source, target: Some(tgt) });
        }
        MoveResult::from_relay(hit_result)
    }

    /// `hitStepTypeImmunity`: `ignoreImmunity` is set on every move object
    /// (`new Dex.Move` defaults it).
    // PS: sim/battle-actions.ts hitStepTypeImmunity
    fn hit_step_type_immunity(b: &mut Battle, target: MonRef, _pokemon: MonRef, mv: MoveRef) -> MoveResult {
        // `runImmunity(move, !move.smartTarget)`.
        MoveResult::from_bool(b.run_immunity(target, Some(mv), true))
    }

    /// `hitStepTryHitEvent`.
    // PS: sim/battle-actions.ts hitStepTryHitEvent
    fn hit_step_try_hit_event(b: &mut Battle, target: MonRef, pokemon: MonRef, mv: MoveRef) -> MoveResult {
        let effect = b.mv(mv).effect();
        let hit_result = b.run_event(EventId::TryHit, Holder::Mon(target), Some(pokemon), effect, None);
        // `!hitResults.includes(true) && hitResults.includes(false)`.
        if hit_result == Relay::FAIL {
            let mon = b.ident(pokemon);
            b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
            b.attr_last_move_still();
        }
        if hit_result != Relay::NotFail && !hit_result.truthy() {
            return MoveResult::False;
        }
        MoveResult::from_relay(hit_result)
    }

    /// `hitStepTryImmunity` (powder and Prankster immunities are gens 6
    /// and 7).
    // PS: sim/battle-actions.ts hitStepTryImmunity
    fn hit_step_try_immunity(b: &mut Battle, target: MonRef, pokemon: MonRef, mv: MoveRef) -> MoveResult {
        let effect = b.mv(mv).effect();
        let own = b.mv(mv).handler_effect();
        if !b.single_event(EventId::TryImmunity, own, StateRef::Scratch, 0, Holder::Mon(target), Some(pokemon), effect, None).truthy() {
            let mon = b.ident(target);
            b.add(Msg::Immune { mon, what: None, kw: Kw::NONE });
            return MoveResult::False;
        }
        MoveResult::True
    }

    /// Gen 4's `hitStepAccuracy`: the stages, `ModifyAccuracy`, `Accuracy`
    /// and the roll (OHKO moves by level).
    // PS: data/mods/gen4/scripts.ts hitStepAccuracy
    fn hit_step_accuracy(b: &mut Battle, target: MonRef, pokemon: MonRef, mv: MoveRef) -> MoveResult {
        b.active_target = Some(target);
        let effect = b.mv(mv).effect();
        let mut accuracy = b.mv(mv).accuracy_x256.map(|a| f64::from(a) / 256.0);
        if b.mv(mv).ohko {
            // Bypasses the accuracy modifiers.
            if !b.is_semi_invulnerable(target) {
                if b.mon(pokemon).level < b.mon(target).level {
                    let mon = b.ident(target);
                    b.add(Msg::Immune { mon, what: None, kw: Kw::flags(KwFlags::OHKO) });
                    return MoveResult::False;
                }
                accuracy = Some(30.0 + f64::from(b.mon(pokemon).level - b.mon(target).level));
            }
        } else {
            accuracy = b.gen3_stage_accuracy(pokemon, target, mv);
            accuracy = b.number_event(EventId::ModifyAccuracy, target, Some(pokemon), effect, accuracy, false);
        }
        let always_hit = b.mv(mv).fake != crate::actions::FakeMove::Data && b.dex.move_def(b.mv(mv).id).bools.has("alwaysHit");
        if always_hit {
            accuracy = None;
        } else {
            accuracy = b.number_event(EventId::Accuracy, target, Some(pokemon), effect, accuracy, false);
        }
        if let Some(acc) = accuracy {
            if !b.accuracy_roll(acc) {
                b.attr_last_move(KwFlags::MISS);
                let source = b.ident(pokemon);
                let tgt = b.ident(target);
                b.add(Msg::Miss { source, target: Some(tgt) });
                return MoveResult::False;
            }
        }
        MoveResult::True
    }

    /// `hitStepMoveHitLoop`: each hit's `spreadMoveHit`, then the hit
    /// count, recoil, `gotAttacked` and the secondary events. One target.
    // PS: sim/battle-actions.ts hitStepMoveHitLoop
    fn hit_step_move_hit_loop(b: &mut Battle, target: MonRef, pokemon: MonRef, mv: MoveRef) -> MoveResult {
        let effect = b.mv(mv).effect();
        let own = b.mv(mv).handler_effect();
        let data = b.mv(mv).hit_data();
        let id = b.mv(mv).id;
        let mut damage = MoveResult::Int(0);
        let mut total_damage = 0;
        b.mon_mut(pokemon).last_damage = 0;
        let target_hits = match b.mv(mv).multihit {
            None => 1,
            Some([lo, hi]) if lo == hi => u32::from(lo),
            Some([2, 5]) => *b.prng.sample(&[2u32, 2, 2, 3, 3, 3, 4, 5]),
            Some([lo, hi]) => b.prng.random_range(u32::from(lo), u32::from(hi) + 1),
        };
        let mut null_damage = true;
        let mut move_damage = MoveResult::Undefined;
        // Sleep Talk can only be used asleep: no recursive check.
        let plain = b.mv(mv).fake == crate::actions::FakeMove::Data;
        let bools = |b: &Battle, name: &str| !plain && b.dex.move_def(id).bools.has(name);
        let source_sleep_usable = match b.mv(mv).source_effect {
            EffectId::Move(m) => b.dex.move_def(m).bools.has("sleepUsable"),
            _ => false,
        };
        let is_sleep_usable = bools(b, "sleepUsable") || source_sleep_usable;
        let mut targets_copy = crate::base_actions::HitTarget::Mon(target);
        let mut hit = 1;
        while hit <= target_hits {
            if damage == MoveResult::False {
                break;
            }
            if hit > 1 && b.mon(pokemon).status == conditions::SLP && !is_sleep_usable {
                break;
            }
            if b.mon(target).hp == 0 {
                break;
            }
            let am = b.mv_mut(mv);
            am.hit = hit as u8;
            am.last_hit = hit == target_hits;
            targets_copy = crate::base_actions::HitTarget::Mon(target);
            // No `smartTarget` in gen 3.

            if bools(b, "multiaccuracy") && hit > 1 {
                let mut accuracy = b.gen3_stage_accuracy(pokemon, target, mv);
                accuracy = b.number_event(EventId::ModifyAccuracy, target, Some(pokemon), effect, accuracy, false);
                if !bools(b, "alwaysHit") {
                    accuracy = b.number_event(EventId::Accuracy, target, Some(pokemon), effect, accuracy, false);
                    if let Some(acc) = accuracy {
                        if !b.accuracy_roll(acc) {
                            break;
                        }
                    }
                }
            }

            let (md, t) = b.spread_move_hit::<Self>(targets_copy, pokemon, mv, data, false, false);
            move_damage = md;
            targets_copy = t;
            if move_damage == MoveResult::False {
                break;
            }
            null_damage = false;
            // Each hit's damage counts on its own (Counter, Mirror Coat).
            damage = MoveResult::Int(if move_damage == MoveResult::True { 0 } else { move_damage.or_zero() });
            total_damage += damage.or_zero();
            b.each_event(EventId::Update, EffectId::None, None);
            if b.mon(pokemon).hp == 0 {
                hit += 1;
                break;
            }
            hit += 1;
        }
        // `hit` is one more than the hits made.
        if hit == 1 {
            return MoveResult::False;
        }
        if null_damage {
            damage = MoveResult::False;
        }
        if b.mv(mv).multihit.is_some() {
            let mon = b.ident(target);
            b.add(Msg::HitCount { mon, count: (hit - 1) as u8 });
        }

        if total_damage != 0 {
            b.apply_recoil_damage(total_damage, mv, pokemon);
        }

        if let Some(t) = targets_copy.mon() {
            if pokemon != t {
                b.got_attacked(t, id, move_damage, pokemon);
            }
        }

        if b.mv(mv).ohko && b.mon(target).hp == 0 {
            b.add(Msg::Ohko);
        }

        if !(damage.truthy() || damage == MoveResult::Int(0)) {
            return damage;
        }

        b.each_event(EventId::Update, EffectId::None, None);

        if !b.suppressing_secondaries() {
            // `afterMoveSecondaryEvent(targetsCopy.filter(val => !!val))`.
            if let Some(t) = targets_copy.mon() {
                b.single_event(EventId::AfterMoveSecondary, own, StateRef::Scratch, 0, Holder::Mon(t), Some(pokemon), effect, None);
                b.run_event(EventId::AfterMoveSecondary, Holder::Mon(t), Some(pokemon), effect, None);
            }
            // `move.totalDamage` is always a number with one target.
            if b.mon(target).hp != 0 {
                let before = i32::from(b.mon(target).hurt_this_turn.unwrap_or(0)) + total_damage;
                b.run_event(EventId::EmergencyExit, Holder::Mon(target), Some(pokemon), EffectId::None, Some(Relay::Int(before)));
            }
        }

        damage
    }
}

impl GenScripts for Gen3Scripts {
    // PS: sim/battle-actions.ts runMove
    fn run_move(b: &mut Battle, mv: ActionMove, pokemon: MonRef, target_loc: i8, source_effect: EffectId) {
        b.base_run_move::<Self>(mv, pokemon, target_loc, source_effect);
    }

    // PS: sim/battle-actions.ts useMove
    fn use_move(b: &mut Battle, mv: MoveRef, pokemon: MonRef, target: Option<Option<MonRef>>, source_effect: EffectId) -> bool {
        b.base_use_move::<Self>(mv, pokemon, target, source_effect)
    }

    /// Uses the move: `ModifyMove`, the `|move|` line, Pressure's PP, `TryMove`,
    /// then `tryMoveHit` on the target(s).
    // PS: data/mods/gen3/scripts.ts useMoveInner
    fn use_move_inner(
        b: &mut Battle,
        mv: MoveRef,
        pokemon: MonRef,
        target: Option<Option<MonRef>>,
        source_effect: EffectId,
    ) -> bool {
        let mut source_effect = source_effect;
        if source_effect.is_none() && !b.frame.effect.is_none() {
            source_effect = b.frame.effect;
        }
        // `instruct` is gen 7.
        if b.mv(mv).fake == crate::actions::FakeMove::None {
            b.require_ported(b.mv(mv).effect());
        }

        // `getActiveMove(move)`: the caller passes a move object (a new one
        // for a move called by id). `move.priority = activeMove.priority` is
        // not kept: nothing reads a move object's priority after the queue
        // sorted the action.
        let (id, type_) = (b.mv(mv).id, b.mv(mv).type_);
        let mon = b.mon_mut(pokemon);
        mon.last_move_used = id;
        mon.last_move_used_type = type_;

        let base_target = b.mv(mv).target;
        let mut target = match target {
            Some(t) => t,
            None => {
                let am = b.action_move_of(mv);
                b.get_random_target(pokemon, am)
            }
        };
        if matches!(b.mv(mv).target, MoveTarget::Self_ | MoveTarget::Allies) {
            target = Some(pokemon);
        }
        if !source_effect.is_none() {
            let am = b.mv_mut(mv);
            am.source_effect = source_effect;
            am.ignore_ability = false;
        }
        let mut move_result = false;

        b.set_active_move(Some(mv), Some(pokemon), target);

        let effect = b.mv(mv).effect();
        let relay = b.mv(mv).relay();
        b.single_event(EventId::ModifyMove, effect, StateRef::Scratch, 0, Holder::Mon(pokemon), target, effect, Some(relay));
        if base_target != b.mv(mv).target {
            // The target changed in `ModifyMove`: adjust it before the next
            // event sees it (from the move object's new target).
            let t = b.mv(mv).target;
            target = b.get_random_target_of(pokemon, Some(t));
        }
        let r = b.run_event(EventId::ModifyMove, Holder::Mon(pokemon), target, effect, Some(relay));
        // `if (!move) return false`: a handler could only clear it.
        assert!(matches!(r, Relay::Move(_)), "ModifyMove answered {r:?}");
        // `lastMoveUsed` is this object: it sees what `ModifyMove` changed
        // (Hidden Power's and Weather Ball's type).
        b.mon_mut(pokemon).last_move_used_type = b.mv(mv).type_;
        if base_target != b.mv(mv).target {
            let t = b.mv(mv).target;
            target = b.get_random_target_of(pokemon, Some(t));
        }
        if b.mon(pokemon).fainted {
            return false;
        }

        // `movename`: Hidden Power's is its own name already.
        let kw = if source_effect.is_none() { Kw::NONE } else { Kw::from(b.move_from(source_effect)) };
        let user = b.ident(pokemon);
        // `${target!}${attrs}`: no target is written `null`.
        let tgt = Some(target.map_or(ps_protocol::PokemonIdent::JS_NULL, |t| b.ident(t)));
        let id = b.mv(mv).id;
        b.add_move(Msg::Move { user, mv: MoveNum(id.0), target: tgt, kw });

        let Some(t) = target else {
            b.attr_last_move(KwFlags::NOTARGET);
            let mon = b.ident(pokemon);
            b.add(Msg::NoTarget(Some(mon)));
            return false;
        };

        let targets = b.get_move_targets(pokemon, mv, t);

        // `callerMoveForPressure`: a calling move (any `sourceEffect` with
        // `pp`, Pursuit's switch-out use included) pays Pressure's PP.
        let caller_move_for_pressure = match source_effect {
            EffectId::Move(m) if b.dex.move_def(m).pp != 0 => Some(m),
            _ => None,
        };
        if source_effect.is_none() || caller_move_for_pressure.is_some() {
            let mut extra_pp = 0;
            for source in targets.pressure.iter() {
                let pp_drop = b.run_event(EventId::DeductPP, Holder::Mon(source), Some(pokemon), effect, None);
                if pp_drop != Relay::TRUE {
                    extra_pp += pp_drop.as_int().unwrap_or(0);
                }
            }
            if extra_pp > 0 {
                Self::deduct_pp(b, pokemon, caller_move_for_pressure.unwrap_or(id), extra_pp);
            }
        }

        if !b.single_event(EventId::TryMove, effect, StateRef::Scratch, 0, Holder::Mon(pokemon), target, effect, None).truthy()
            || !b.run_event(EventId::TryMove, Holder::Mon(pokemon), target, effect, None).truthy()
        {
            return false;
        }

        b.single_event(EventId::UseMoveMessage, effect, StateRef::Scratch, 0, Holder::Mon(pokemon), target, effect, None);

        // `move.ignoreImmunity` is never undefined: the dex sets it.

        let def = b.dex.move_def(id);
        if def.selfdestruct() == Some(ps_dex::SelfDestruct::Always) {
            b.faint(pokemon, Some(pokemon), effect);
        }

        let mut damage = MoveResult::False;
        let mut target = target;
        match b.mv(mv).target {
            MoveTarget::All | MoveTarget::FoeSide | MoveTarget::AllySide | MoveTarget::AllyTeam => {
                damage = Self::try_move_hit(b, t, pokemon, mv);
                if damage == MoveResult::NotFail {
                    b.mon_mut(pokemon).move_this_turn_result = Some(Relay::Null);
                }
                if damage.succeeded() {
                    move_result = true;
                }
            }
            MoveTarget::AllAdjacent | MoveTarget::AllAdjacentFoes => {
                if targets.targets.is_empty() {
                    b.attr_last_move(KwFlags::NOTARGET);
                    let mon = b.ident(pokemon);
                    b.add(Msg::NoTarget(Some(mon)));
                    return false;
                }
                // One target in singles: `spreadHit` stays unset and no
                // `[spread]` goes on the line.
                for source in targets.targets.iter() {
                    let hit_result = Self::try_move_hit(b, source, pokemon, mv);
                    if hit_result.succeeded() {
                        move_result = true;
                    }
                    if damage.truthy() {
                        damage = MoveResult::Int(damage.or_zero() + hit_result.or_zero());
                    } else if damage != MoveResult::False || hit_result != MoveResult::NotFail {
                        damage = hit_result;
                    }
                    if damage == MoveResult::NotFail {
                        b.mon_mut(pokemon).move_this_turn_result = Some(Relay::Null);
                    }
                }
            }
            _ => {
                target = targets.targets.first();
                let mut lacks_target = target.is_none_or(|t| b.mon(t).fainted);
                if let Some(t) = target {
                    if !lacks_target
                        && matches!(
                            b.mv(mv).target,
                            MoveTarget::AdjacentFoe | MoveTarget::AdjacentAlly | MoveTarget::Normal | MoveTarget::RandomNormal
                        )
                    {
                        lacks_target = !b.is_adjacent(t, pokemon);
                    }
                }
                if lacks_target && !def.flags.has("futuremove") {
                    b.attr_last_move(KwFlags::NOTARGET);
                    let mon = b.ident(pokemon);
                    b.add(Msg::NoTarget(Some(mon)));
                    return false;
                }
                let t = target.expect("a future move without a target");
                damage = Self::try_move_hit(b, t, pokemon, mv);
                if damage == MoveResult::NotFail {
                    b.mon_mut(pokemon).move_this_turn_result = Some(Relay::Null);
                }
                if damage.succeeded() {
                    move_result = true;
                }
            }
        }
        // `selfBoost`: no gen 1-3 move.
        if b.mon(pokemon).hp == 0 {
            b.faint(pokemon, Some(pokemon), effect);
        }

        if !move_result {
            let holder = target.map_or(Holder::None, Holder::Mon);
            b.single_event(EventId::MoveFail, effect, StateRef::Scratch, 0, holder, Some(pokemon), effect, None);
            return false;
        }

        if !b.suppressing_secondaries() {
            b.single_event(EventId::AfterMoveSecondarySelf, effect, StateRef::Scratch, 0, Holder::Mon(pokemon), target, effect, None);
            b.run_event(EventId::AfterMoveSecondarySelf, Holder::Mon(pokemon), target, effect, None);
        }
        true
    }

    /// The hit attempt: `PrepareHit`, `Try`, invulnerability, immunity,
    /// gen 3's accuracy (OHKO by level), `TryHit`, multi-hit, recoil.
    // PS: data/mods/gen3/scripts.ts tryMoveHit
    fn try_move_hit(b: &mut Battle, target: MonRef, pokemon: MonRef, mv: MoveRef) -> MoveResult {
        b.set_active_move(Some(mv), Some(pokemon), Some(target));
        let mut natural_immunity = false;
        let mut acc_pass = true;
        let effect = b.mv(mv).effect();
        // The move's own handlers: none on a plain `moveData` object
        // (Bide's unleash), whose hits carry only its fixed damage.
        let own = b.mv(mv).handler_effect();
        let data = b.mv(mv).hit_data();
        let id = b.mv(mv).id;
        let def = b.dex.move_def(id);

        // `singleEvent('PrepareHit', ...) && runEvent('PrepareHit', ...)`.
        let mut hit_result =
            b.single_event(EventId::PrepareHit, own, StateRef::Scratch, 0, Holder::Mon(target), Some(pokemon), effect, None);
        if hit_result.truthy() {
            hit_result = b.run_event(EventId::PrepareHit, Holder::Mon(pokemon), Some(target), effect, None);
        }
        if !hit_result.truthy() {
            if hit_result == Relay::FAIL {
                let mon = b.ident(pokemon);
                b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
                b.attr_last_move_still();
            }
            return MoveResult::False;
        }

        if !b.single_event(EventId::Try, own, StateRef::Scratch, 0, Holder::Mon(pokemon), Some(target), effect, None).truthy() {
            return MoveResult::False;
        }

        let move_target = b.mv(mv).target;
        if matches!(move_target, MoveTarget::All | MoveTarget::FoeSide | MoveTarget::AllySide | MoveTarget::AllyTeam) {
            let event = if move_target == MoveTarget::All { EventId::TryHitField } else { EventId::TryHitSide };
            let hit_result = b.run_event(event, Holder::Mon(target), Some(pokemon), effect, None);
            if !hit_result.truthy() {
                if hit_result == Relay::FAIL {
                    let mon = b.ident(pokemon);
                    b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
                    b.attr_last_move_still();
                }
                return MoveResult::False;
            }
            return Self::move_hit(b, Some(target), pokemon, mv, data, false, false);
        }

        let hit_result = b.run_event(EventId::Invulnerability, Holder::Mon(target), Some(pokemon), effect, None);
        if hit_result == Relay::FAIL {
            // `spreadHit` is never set in singles.
            b.attr_last_move(KwFlags::MISS);
            let source = b.ident(pokemon);
            let tgt = b.ident(target);
            b.add(Msg::Miss { source, target: Some(tgt) });
            return MoveResult::False;
        }

        // `move.ignoreImmunity === undefined` (a plain `moveData` object):
        // `category === 'Status'`, which the object's field already is.

        if !b.run_immunity(target, Some(mv), false) {
            natural_immunity = true;
        } else {
            let hit_result =
                b.single_event(EventId::TryImmunity, own, StateRef::Scratch, 0, Holder::Mon(target), Some(pokemon), effect, None);
            if hit_result == Relay::FAIL {
                natural_immunity = true;
            }
        }

        // The true accuracy (`None` is `true`).
        let mut accuracy = b.gen3_stage_accuracy(pokemon, target, mv);
        if b.mv(mv).ohko {
            // OHKO moves bypass the accuracy modifiers.
            if !b.is_semi_invulnerable(target) {
                // Gen 3's `ohko` is always `true` (no typed OHKO).
                if b.mon(pokemon).level >= b.mon(target).level {
                    accuracy = Some(30.0 + f64::from(b.mon(pokemon).level - b.mon(target).level));
                } else {
                    let mon = b.ident(target);
                    b.add(Msg::Immune { mon, what: None, kw: Kw::flags(KwFlags::OHKO) });
                    return MoveResult::False;
                }
            }
        } else {
            accuracy = b.number_event(EventId::ModifyAccuracy, target, Some(pokemon), effect, accuracy, false);
        }
        if def.bools.has("alwaysHit") {
            // Bypasses the OHKO accuracy too.
            accuracy = None;
        } else {
            accuracy = b.number_event(EventId::Accuracy, target, Some(pokemon), effect, accuracy, false);
        }
        if let Some(acc) = accuracy {
            if !b.accuracy_roll(acc) {
                acc_pass = false;
            }
        }

        if acc_pass {
            let hit_result = b.run_event(EventId::TryHit, Holder::Mon(target), Some(pokemon), effect, None);
            if !hit_result.truthy() {
                if hit_result == Relay::FAIL {
                    let mon = b.ident(pokemon);
                    b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
                    b.attr_last_move_still();
                }
                return MoveResult::False;
            } else if natural_immunity {
                let mon = b.ident(target);
                b.add(Msg::Immune { mon, what: None, kw: Kw::NONE });
                return MoveResult::False;
            }
        } else {
            if natural_immunity {
                let mon = b.ident(target);
                b.add(Msg::Immune { mon, what: None, kw: Kw::NONE });
            } else {
                b.attr_last_move(KwFlags::MISS);
                let source = b.ident(pokemon);
                let tgt = b.ident(target);
                b.add(Msg::Miss { source, target: Some(tgt) });
            }
            return MoveResult::False;
        }

        let mut total_damage = MoveResult::Int(0);
        let mut damage;
        b.mon_mut(pokemon).last_damage = 0;
        if let Some([lo, hi]) = b.mv(mv).multihit {
            // A fixed count is a number in PS (no PRNG call).
            let hits = if lo == hi {
                u32::from(lo)
            } else if lo == 2 && hi == 5 {
                *b.prng.sample(&[2u32, 2, 2, 3, 3, 3, 4, 5])
            } else {
                b.prng.random_range(u32::from(lo), u32::from(hi) + 1)
            };
            let mut null_damage = true;
            damage = MoveResult::Int(0);
            // Sleep Talk can only be used asleep: no recursive check.
            let source_sleep_usable = match b.mv(mv).source_effect {
                EffectId::Move(m) => b.dex.move_def(m).bools.has("sleepUsable"),
                _ => false,
            };
            let is_sleep_usable = def.bools.has("sleepUsable") || source_sleep_usable;
            let mut i = 0;
            while i < hits && b.mon(target).hp != 0 && b.mon(pokemon).hp != 0 {
                if b.mon(pokemon).status == conditions::SLP && !is_sleep_usable {
                    break;
                }
                let am = b.mv_mut(mv);
                am.hit = (i + 1) as u8;
                am.last_hit = i + 1 == hits;

                if def.bools.has("multiaccuracy") && i > 0 {
                    let mut accuracy = b.gen3_stage_accuracy(pokemon, target, mv);
                    accuracy = b.number_event(EventId::ModifyAccuracy, target, Some(pokemon), effect, accuracy, false);
                    if !def.bools.has("alwaysHit") {
                        accuracy = b.number_event(EventId::Accuracy, target, Some(pokemon), effect, accuracy, false);
                        if let Some(acc) = accuracy {
                            if !b.accuracy_roll(acc) {
                                break;
                            }
                        }
                    }
                }

                let move_damage = Self::move_hit(b, Some(target), pokemon, mv, data, false, false);
                if move_damage == MoveResult::False {
                    break;
                }
                if null_damage && move_damage.succeeded() {
                    null_damage = false;
                }
                // Each hit's damage counts on its own (Counter, Mirror Coat).
                damage = MoveResult::Int(move_damage.or_zero());
                total_damage = MoveResult::Int(total_damage.or_zero() + damage.or_zero());
                b.each_event(EventId::Update, EffectId::None, None);
                i += 1;
            }
            if i == 0 {
                return MoveResult::False;
            }
            if null_damage {
                damage = MoveResult::False;
            }
            let mon = b.ident(target);
            b.add(Msg::HitCount { mon, count: i as u8 });
        } else {
            damage = Self::move_hit(b, Some(target), pokemon, mv, data, false, false);
            total_damage = damage;
        }

        // `move.totalDamage` (Shell Bell reads it).
        b.mv_mut(mv).total_damage = total_damage.or_zero();
        if total_damage.truthy() {
            b.apply_recoil_damage(total_damage.or_zero(), mv, pokemon);
        }

        if pokemon != target {
            b.got_attacked(target, id, damage, pokemon);
        }

        if b.mv(mv).ohko && b.mon(target).hp == 0 {
            b.add(Msg::Ohko);
        }

        if !damage.truthy() && damage != MoveResult::Int(0) {
            return damage;
        }

        b.each_event(EventId::Update, EffectId::None, None);

        b.single_event(EventId::AfterMoveSecondary, own, StateRef::Scratch, 0, Holder::Mon(target), Some(pokemon), effect, None);
        b.run_event(EventId::AfterMoveSecondary, Holder::Mon(target), Some(pokemon), effect, None);

        damage
    }

    // PS: sim/battle-actions.ts moveHit
    fn move_hit(
        b: &mut Battle,
        target: Option<MonRef>,
        pokemon: MonRef,
        mv: MoveRef,
        data: MoveData,
        is_secondary: bool,
        is_self: bool,
    ) -> MoveResult {
        b.base_move_hit::<Self>(target, pokemon, mv, data, is_secondary, is_self)
    }

    /// Base `getDamage` (gen 3's OHKO deals the target's HP), then gen 3's
    /// `modifyDamage`.
    // PS: sim/battle-actions.ts getDamage
    fn get_damage(b: &mut Battle, source: MonRef, target: MonRef, mv: MoveRef, data: MoveData, suppress: bool) -> MoveResult {
        match b.base_get_damage(source, target, mv, data, suppress) {
            DamageStep::Done(r) => r,
            DamageStep::Modify(base_damage) => b.gen3_modify_damage(base_damage, source, target, mv, suppress),
        }
    }

    // PS: sim/pokemon.ts deductPP
    fn deduct_pp(b: &mut Battle, pokemon: MonRef, mv: MoveId, amount: i32) -> i32 {
        b.base_deduct_pp(pokemon, mv, amount)
    }

    // PS: sim/battle.ts boost
    fn boost(
        b: &mut Battle,
        boosts: [i8; 7],
        target: MonRef,
        source: Option<MonRef>,
        effect: EffectId,
        is_secondary: bool,
        is_self: bool,
    ) -> MoveResult {
        b.base_boost(boosts, target, source, effect, is_secondary, is_self)
    }

    // PS: sim/pokemon.ts clearBoosts
    fn clear_boosts(b: &mut Battle, pokemon: MonRef) {
        b.base_clear_boosts(pokemon);
    }
}
