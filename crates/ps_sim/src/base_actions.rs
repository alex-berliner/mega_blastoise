//! PS's base move pipeline: the `sim/battle-actions.ts` functions (and the
//! `sim/battle.ts` / `sim/pokemon.ts` methods they call) that a gen
//! inherits instead of overriding.
//!
//! Gen 1 overrides every pipeline function it reaches, so today only gen 3
//! runs this file: its `scripts.ts` overrides `useMoveInner`, `tryMoveHit`
//! and `modifyDamage` (gen 4 adds `hitStepAccuracy`,
//! `hitStepInvulnerabilityEvent`, `applyRecoilDamage` and `runSwitch`) and
//! inherits `runMove`, `useMove`, `moveHit` with `spreadMoveHit` and its
//! steps, `getDamage`, `deductPP` and `boost` from base, all here. Each
//! function takes the gen's [`GenScripts`] as a type parameter where PS
//! calls back through `this.` into something a gen may override.
//!
//! Singles only: a move has one target, so PS's `targets` and `damage`
//! arrays have one entry ([`HitTarget`], [`MoveResult`]) and `spreadHit`
//! is never set.
//!
//! An effect whose handlers are not ported panics `unported: <id>`
//! ([`Battle::require_ported`]).
//!
//! PS: sim/battle-actions.ts `runMove`, `useMove`, `spreadMoveHit`,
//! `tryPrimaryHitEvent`, `getSpreadDamage`, `runMoveEffects`, `selfDrops`,
//! `secondaries`, `forceSwitch`, `moveHit`, `getDamage`; sim/battle.ts
//! `boost`, `spreadDamage` (non-number entries), `randomizer`,
//! `retargetLastMove`; sim/pokemon.ts `deductPP`, `getMoveData`,
//! `getCappedBoost`, `boostBy`, `clearBoosts`, `getMoveTargets`,
//! `isAdjacent`, `isSemiInvulnerable`, `trySetStatus`.

use ps_dex::{MoveCategory, MoveId, MoveTarget};
use ps_protocol::{Arg, ArgValue, Kw, Msg, MoveNum, Word};

use crate::actions::{
    data_boosts, data_force_switch, data_heal, data_pseudo_weather, data_secondaries, data_self, data_self_switch,
    data_selfdestruct, data_side_condition, data_slot_condition, data_status, data_volatile, data_weather, FakeMove,
    GenScripts, MoveData, MoveRef, MoveResult,
};
use crate::effect::{EffectId, EventId};
use crate::queue::ActionMove;
use crate::relay::Relay;
use crate::state::{Battle, Holder, MonRef, StateRef, SwitchFlag};
use crate::stats::Stat;

/// One entry of `spreadMoveHit`'s `targets`: a Pokemon, `null` (a
/// Substitute took the hit) or `false` (no longer hit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitTarget {
    Mon(MonRef),
    Null,
    False,
}

impl HitTarget {
    pub fn mon(self) -> Option<MonRef> {
        match self {
            HitTarget::Mon(m) => Some(m),
            _ => None,
        }
    }
}

/// Up to two Pokemon, in order (a field move reaches the user and the foe).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MonList {
    list: [Option<MonRef>; 2],
    len: u8,
}

impl MonList {
    /// Adds `m` when there is one.
    pub fn push(&mut self, m: Option<MonRef>) {
        if let Some(m) = m {
            self.list[self.len as usize] = Some(m);
            self.len += 1;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = MonRef> + '_ {
        self.list[..self.len as usize].iter().flatten().copied()
    }

    pub fn first(&self) -> Option<MonRef> {
        self.iter().next()
    }

    pub fn last(&self) -> Option<MonRef> {
        self.iter().last()
    }

    pub fn contains(&self, m: MonRef) -> bool {
        self.iter().any(|x| x == m)
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// `pokemon.getMoveTargets(move, target)`: `targets` and `pressureTargets`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MoveTargets {
    pub targets: MonList,
    pub pressure: MonList,
}

/// The `[from]` of a `-heal` or `-fail` keyword: `heal`.
fn word_heal() -> Arg {
    Arg::new(ArgValue::Word(Word::Heal))
}

impl Battle {
    // ---- runMove / useMove -------------------------------------------

    /// The main move function: `BeforeMove`, PP, `useMove`, `AfterMove`.
    /// No Z-move, Max Move, Dancer or `externalMove` in gens 1-3.
    // PS: sim/battle-actions.ts runMove
    pub fn base_run_move<S: GenScripts>(&mut self, action_move: ActionMove, pokemon: MonRef, target_loc: i8, source_effect: EffectId) {
        self.mon_mut(pokemon).active_move_actions += 1;
        let mut source_effect = source_effect;
        // No `originalTarget` reaches a gen 1-3 move (no `tracksTarget`).
        let mut target = self.get_target(pokemon, action_move, target_loc, None);
        let mut base_move = self.get_active_move(action_move);
        // `move.id === 'struggle'` (a fake move is no dex move).
        let is_struggle = |b: &Battle, mv: MoveRef| {
            let id = b.mv(mv).id;
            !id.is_none() && b.dex.move_def(id).name.as_str() == "Struggle"
        };
        // `priority` and `pranksterBoosted` ride on the move object.
        if !is_struggle(self, base_move) {
            let effect = self.mv(base_move).effect();
            let changed = self.run_event(EventId::OverrideAction, Holder::Mon(pokemon), target, effect, None);
            match changed {
                Relay::Move(id) => {
                    base_move = self.new_active_move(crate::actions::ActiveMove::from_dex(self.dex, id));
                    target = self.get_random_target(pokemon, ActionMove::Move(id));
                }
                r if !r.truthy() || r == Relay::TRUE => {}
                other => panic!("OverrideAction answered {other:?}"),
            }
        }
        let mv = base_move;
        // `move.isExternal = externalMove` (undefined).

        self.set_active_move(Some(mv), Some(pokemon), target);

        let effect = self.mv(mv).effect();
        if self.mv(mv).fake == FakeMove::None {
            self.require_ported(effect);
        }
        let will_try_move = self.run_event(EventId::BeforeMove, Holder::Mon(pokemon), target, effect, None);
        if !will_try_move.truthy() {
            self.run_event(EventId::MoveAborted, Holder::Mon(pokemon), target, effect, None);
            self.clear_active_move(true);
            // `false` counts as a failed move (Stomping Tantrum), `null` not.
            self.mon_mut(pokemon).move_this_turn_result = Some(will_try_move);
            return;
        }

        // `cantusetwice`: no gen 1-3 move has the flag.

        if self.move_callback(mv, EventId::BeforeMoveCallback, pokemon, target).is_some_and(Relay::truthy) {
            self.clear_active_move(true);
            self.mon_mut(pokemon).move_this_turn_result = Some(Relay::FAIL);
            return;
        }
        self.mon_mut(pokemon).last_damage = 0;
        // Not `externalMove`.
        if self.get_locked_move(pokemon).is_none() {
            let id = self.mv(base_move).id;
            if S::deduct_pp(self, pokemon, id, 0) == 0 && !is_struggle(self, mv) {
                let mon = self.ident(pokemon);
                let mv_arg = Arg::new(ArgValue::Move(MoveNum(self.mv(mv).id.0)));
                self.add(Msg::Cant { mon, reason: Arg::new(ArgValue::Word(Word::NoPp)), mv: Some(mv_arg) });
                self.clear_active_move(true);
                self.mon_mut(pokemon).move_this_turn_result = Some(Relay::FAIL);
                return;
            }
        } else {
            let lockedmove = self.dex.condition_id("lockedmove").expect("lockedmove in the dex");
            source_effect = EffectId::Condition(lockedmove);
        }
        self.move_used(pokemon, mv, target_loc);

        let old_active_move = mv;

        let move_did_something = self.use_move_gen::<S>(mv, pokemon, Some(target), source_effect);
        self.last_successful_move_this_turn =
            if move_did_something { self.active_move().map_or(MoveId::NONE, |m| m.id) } else { MoveId::NONE };
        let mv = self.active_move.unwrap_or(mv);
        let effect = self.mv(mv).effect();
        self.single_event(EventId::AfterMove, effect, StateRef::Scratch, 0, Holder::Mon(pokemon), target, effect, None);
        self.run_event(EventId::AfterMove, Holder::Mon(pokemon), target, effect, None);

        // No `cantusetwice` or `dance` move in gens 1-3.
        self.faint_messages(false, false, true);
        self.check_win(None);

        if self.gen <= 4 {
            // In gen 4, the outermost move is the last move for Copycat.
            self.active_move = Some(old_active_move);
        }
    }

    /// `useMove` with the gen's `useMoveInner`.
    fn use_move_gen<S: GenScripts>(
        &mut self,
        mv: MoveRef,
        pokemon: MonRef,
        target: Option<Option<MonRef>>,
        source_effect: EffectId,
    ) -> bool {
        S::use_move(self, mv, pokemon, target, source_effect)
    }

    /// The inside move caller: `useMoveInner` and its `moveThisTurnResult`.
    // PS: sim/battle-actions.ts useMove
    pub fn base_use_move<S: GenScripts>(
        &mut self,
        mv: MoveRef,
        pokemon: MonRef,
        target: Option<Option<MonRef>>,
        source_effect: EffectId,
    ) -> bool {
        self.mon_mut(pokemon).move_this_turn_result = None;
        let old_move_result = self.mon(pokemon).move_this_turn_result;
        let move_result = S::use_move_inner(self, mv, pokemon, target, source_effect);
        if old_move_result == self.mon(pokemon).move_this_turn_result {
            self.mon_mut(pokemon).move_this_turn_result = Some(Relay::Bool(move_result));
        }
        move_result
    }

    // ---- moveHit and spreadMoveHit -------------------------------------

    /// One hit of a move (or of its secondary or `self` effect): `true`
    /// becomes `undefined`.
    // PS: sim/battle-actions.ts moveHit
    pub fn base_move_hit<S: GenScripts>(
        &mut self,
        target: Option<MonRef>,
        pokemon: MonRef,
        mv: MoveRef,
        data: MoveData,
        is_secondary: bool,
        is_self: bool,
    ) -> MoveResult {
        let t = target.map_or(HitTarget::Null, HitTarget::Mon);
        let (ret, _) = self.spread_move_hit::<S>(t, pokemon, mv, data, is_secondary, is_self);
        if ret == MoveResult::True {
            MoveResult::Undefined
        } else {
            ret
        }
    }

    /// The hit itself: `TryHit`, Substitute, damage, effects, self drops,
    /// secondaries, phazing, `DamagingHit`.
    // PS: sim/battle-actions.ts spreadMoveHit
    pub fn spread_move_hit<S: GenScripts>(
        &mut self,
        target: HitTarget,
        pokemon: MonRef,
        mv: MoveRef,
        data: MoveData,
        is_secondary: bool,
        is_self: bool,
    ) -> (MoveResult, HitTarget) {
        // Hardcoded for single-target purposes (no spread move has any kind
        // of `onTryHit` handler).
        let first = target;
        let mut damage = MoveResult::True;
        let mut target = target;
        let id = self.mv(mv).id;
        let def = self.dex.move_def(id);
        let move_target = self.mv(mv).target;
        let effect = self.mv(mv).effect();
        let side_target = matches!(move_target, MoveTarget::FoeSide | MoveTarget::AllySide | MoveTarget::AllyTeam);
        let holder = first.mon().map_or(Holder::None, Holder::Mon);
        let mut hit_result = Relay::TRUE;
        if move_target == MoveTarget::All && !is_self {
            hit_result = self.move_data_event(EventId::TryHitField, data, mv, holder, Some(pokemon), None);
        } else if side_target && !is_self {
            hit_result = self.move_data_event(EventId::TryHitSide, data, mv, holder, Some(pokemon), None);
        } else if let Some(t) = first.mon() {
            hit_result = self.move_data_event(EventId::TryHit, data, mv, Holder::Mon(t), Some(pokemon), None);
        }
        if !hit_result.truthy() {
            if hit_result == Relay::FAIL {
                let mon = self.ident(pokemon);
                self.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
                self.attr_last_move_still();
            }
            return (MoveResult::False, target);
        }

        // 0. Check for Substitute.
        if !is_secondary && !is_self && !matches!(move_target, MoveTarget::All | MoveTarget::AllyTeam | MoveTarget::AllySide | MoveTarget::FoeSide) {
            damage = self.try_primary_hit_event(damage, target, pokemon, mv, data);
        }

        if damage.relay() == Some(Relay::HIT_SUBSTITUTE) {
            damage = MoveResult::True;
            target = HitTarget::Null;
        }
        if target.mon().is_some() && is_secondary && data_self(data, def).is_none() {
            damage = MoveResult::True;
        }
        if !damage.truthy() {
            target = HitTarget::False;
        }
        // 1. The damage calculation.
        damage = self.get_spread_damage::<S>(damage, target, pokemon, mv, data, is_secondary, is_self);
        if damage == MoveResult::False {
            target = HitTarget::False;
        }

        // 2. The damage.
        damage = self.spread_damage_one(damage, target, Some(pokemon), effect);
        if damage == MoveResult::False {
            target = HitTarget::False;
        }

        // 3. The `onHit` events.
        damage = self.run_move_effects::<S>(damage, target, pokemon, mv, data, is_secondary, is_self);
        if !damage.truthy() && damage != MoveResult::Int(0) {
            target = HitTarget::False;
        }

        // Steps 4 and 5 can change `activeTarget`, which Dancer needs.
        let active_target = self.active_target;

        // 4. Self drops (from here on `targets[i] === false` is checked).
        if data_self(data, def).is_some() && !self.mv(mv).self_dropped {
            self.self_drops::<S>(target, pokemon, mv, data, is_secondary);
        }

        // 5. Secondary effects.
        if !data_secondaries(data, def).is_empty() {
            self.secondaries::<S>(target, pokemon, mv, data, is_self);
        }

        self.active_target = active_target;

        // 6. Force switch.
        if data_force_switch(data, def) {
            damage = self.force_switch(damage, target, pokemon, mv);
        }

        if !damage.truthy() && damage != MoveResult::Int(0) {
            target = HitTarget::False;
        }

        let damaged = match (damage, target) {
            (MoveResult::Int(d), HitTarget::Mon(t)) => Some((t, d)),
            _ => None,
        };
        let pokemon_original_hp = self.mon(pokemon).hp;
        if let (Some((t, d)), false, false) = (damaged, is_secondary, is_self) {
            // `onAfterHit` (gens 1-4: before `DamagingHit`).
            if self.registry.handler(effect, crate::effect::Prefix::None, EventId::AfterHit).is_some() && self.mon(pokemon).hp != 0 {
                self.move_data_event(EventId::AfterHit, data, mv, Holder::Mon(t), Some(pokemon), None);
            }
            if self.gen < 5 {
                self.run_event_spread(EventId::DamagingHit, &[t], Some(pokemon), effect, Some(&[Relay::Int(d)]));
            }
            let mon = self.mon(pokemon);
            let half = u32::from(mon.maxhp);
            if mon.hp != 0 && 2 * u32::from(mon.hp) <= half && 2 * u32::from(pokemon_original_hp) > half {
                self.run_event(EventId::EmergencyExit, Holder::Mon(pokemon), None, EffectId::None, None);
            }
        }

        (damage, target)
    }

    /// `TryPrimaryHit` (Substitute answers it).
    // PS: sim/battle-actions.ts tryPrimaryHitEvent
    fn try_primary_hit_event(&mut self, damage: MoveResult, target: HitTarget, pokemon: MonRef, mv: MoveRef, _data: MoveData) -> MoveResult {
        let Some(t) = target.mon() else { return damage };
        // `runEvent('TryPrimaryHit', target, pokemon, moveData)`: a secondary
        // or `self` effect never gets here, so `moveData` is the move.
        let effect = self.mv(mv).effect();
        MoveResult::from_relay(self.run_event(EventId::TryPrimaryHit, Holder::Mon(t), Some(pokemon), effect, None))
    }

    /// `getDamage` for the target, with its `false` / `null` meaning
    /// "interrupted".
    // PS: sim/battle-actions.ts getSpreadDamage
    #[allow(clippy::too_many_arguments)]
    fn get_spread_damage<S: GenScripts>(
        &mut self,
        damage: MoveResult,
        target: HitTarget,
        source: MonRef,
        mv: MoveRef,
        data: MoveData,
        _is_secondary: bool,
        _is_self: bool,
    ) -> MoveResult {
        let Some(t) = target.mon() else { return damage };
        self.active_target = Some(t);
        // `damage[i] = undefined`: so the `-fail` below (for `damage[i] ===
        // false && !isSecondary && !isSelf`) never shows.
        let cur_damage = S::get_damage(self, source, t, mv, data, false);
        // A number deals that much damage (0 too), `false` fails ("But it
        // failed!"), `null` ends the move silently, `undefined` deals none
        // and goes on.
        if matches!(cur_damage, MoveResult::False | MoveResult::Null) {
            self.debug(ps_protocol::Text::new(ps_protocol::TextId::DamageCalculationInterrupted));
            return MoveResult::False;
        }
        cur_damage
    }

    /// `spreadDamage` of one entry: numbers go to [`Battle::damage`],
    /// anything else comes back as it is.
    // PS: sim/battle.ts spreadDamage
    fn spread_damage_one(&mut self, damage: MoveResult, target: HitTarget, source: Option<MonRef>, effect: EffectId) -> MoveResult {
        if !(damage.truthy() || damage == MoveResult::Int(0)) {
            return damage;
        }
        let Some(t) = target.mon() else { return MoveResult::Int(0) };
        match damage {
            MoveResult::Int(n) => self.damage(n, Some(t), source, effect),
            // `true` only stands with a target that is `null` or `false`.
            other => panic!("spreadDamage of {other:?} against a target"),
        }
    }

    /// The move's effects on the target: boosts, heal, status, volatiles,
    /// conditions, phazing, the `Hit` events, self-destruct, `selfSwitch`.
    // PS: sim/battle-actions.ts runMoveEffects
    #[allow(clippy::too_many_arguments)]
    fn run_move_effects<S: GenScripts>(
        &mut self,
        damage: MoveResult,
        target: HitTarget,
        source: MonRef,
        mv: MoveRef,
        data: MoveData,
        is_secondary: bool,
        is_self: bool,
    ) -> MoveResult {
        let mut damage = damage;
        let mut did_anything = damage;
        let id = self.mv(mv).id;
        let def = self.dex.move_def(id);
        let effect = self.mv(mv).effect();
        let move_target = self.mv(mv).target;
        'target: {
            if target == HitTarget::False {
                break 'target;
            }
            let mut did_something = MoveResult::Undefined;

            if let HitTarget::Mon(t) = target {
                if let Some(boosts) = data_boosts(data, def) {
                    if !self.mon(t).fainted {
                        let hit_result = S::boost(self, boosts.as_array(), t, Some(source), effect, is_secondary, is_self);
                        did_something = MoveResult::combine(did_something, hit_result);
                    }
                }
                if let Some(heal) = data_heal(data, def) {
                    if !self.mon(t).fainted {
                        if self.mon(t).hp >= self.mon(t).maxhp {
                            let mon = self.ident(t);
                            self.add(Msg::Fail { mon, what: Some(word_heal()), what2: None, kw: Kw::NONE });
                            self.attr_last_move_still();
                            damage = MoveResult::combine(damage, MoveResult::False);
                            did_anything = MoveResult::combine(did_anything, MoveResult::Null);
                            break 'target;
                        }
                        // `Math.floor` before gen 5 (the amount is not negative).
                        let amount = i32::from(self.mon(t).base_maxhp) * i32::from(heal.num) / i32::from(heal.den);
                        let d = self.heal(amount, Some(t), Some(source), effect);
                        if !d.truthy() && d != MoveResult::Int(0) {
                            if d != MoveResult::Null {
                                let mon = self.ident(source);
                                self.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
                                self.attr_last_move_still();
                            }
                            self.debug(ps_protocol::Text::new(ps_protocol::TextId::HealInterrupted));
                            damage = MoveResult::combine(damage, MoveResult::False);
                            did_anything = MoveResult::combine(did_anything, MoveResult::Null);
                            break 'target;
                        }
                        did_something = MoveResult::True;
                    }
                }
                let status = data_status(data, def);
                if !status.is_none() {
                    self.require_ported(EffectId::Condition(status));
                    // No gen 1-3 `moveData.ability`.
                    let hit_result = MoveResult::from_relay(self.try_set_status(t, status, Some(source), effect));
                    if !hit_result.truthy() && !def.status.is_none() {
                        damage = MoveResult::combine(damage, MoveResult::False);
                        did_anything = MoveResult::combine(did_anything, MoveResult::Null);
                        break 'target;
                    }
                    did_something = MoveResult::combine(did_something, hit_result);
                }
                // `forceStatus`: no gen 1-3 move has one.
                let volatile = data_volatile(data, def);
                if !volatile.is_none() {
                    self.require_ported(EffectId::Condition(volatile));
                    let hit_result = MoveResult::from_relay(self.add_volatile(t, volatile, Some(source), effect));
                    did_something = MoveResult::combine(did_something, hit_result);
                }
                let side_condition = data_side_condition(data, def);
                if !side_condition.is_none() {
                    self.require_ported(EffectId::Condition(side_condition));
                    let hit_result = MoveResult::from_relay(self.add_side_condition(t.side, side_condition, Some(source), effect));
                    did_something = MoveResult::combine(did_something, hit_result);
                }
                let slot_condition = data_slot_condition(data, def);
                if !slot_condition.is_none() {
                    self.require_ported(EffectId::Condition(slot_condition));
                    let position = self.mon(t).position;
                    let hit_result =
                        MoveResult::from_relay(self.add_slot_condition(t.side, position, slot_condition, Some(source), effect));
                    did_something = MoveResult::combine(did_something, hit_result);
                }
                let weather = data_weather(data, def);
                if !weather.is_none() {
                    self.require_ported(EffectId::Condition(weather));
                    panic!("unported: setWeather");
                }
                // `terrain`: gen 6.
                let pseudo_weather = data_pseudo_weather(data, def);
                if !pseudo_weather.is_none() {
                    self.require_ported(EffectId::Condition(pseudo_weather));
                    let hit_result =
                        MoveResult::from_relay(self.add_pseudo_weather(EffectId::Condition(pseudo_weather), Some(source), effect));
                    did_something = MoveResult::combine(did_something, hit_result);
                }
                if data_force_switch(data, def) {
                    let hit_result = MoveResult::from_bool(self.can_switch(t.side) != 0);
                    did_something = MoveResult::combine(did_something, hit_result);
                }
                // The hit events, like the `TryHit` ones without `FieldHit`.
                if move_target == MoveTarget::All && !is_self {
                    if self.move_data_has(data, mv, EventId::HitField) {
                        let r = self.move_data_event(EventId::HitField, data, mv, Holder::Mon(t), Some(source), None);
                        did_something = MoveResult::combine(did_something, MoveResult::from_relay(r));
                    }
                } else if matches!(move_target, MoveTarget::FoeSide | MoveTarget::AllySide) && !is_self {
                    if self.move_data_has(data, mv, EventId::HitSide) {
                        let r = self.move_data_event(EventId::HitSide, data, mv, Holder::Side(t.side), Some(source), None);
                        did_something = MoveResult::combine(did_something, MoveResult::from_relay(r));
                    }
                } else {
                    if self.move_data_has(data, mv, EventId::Hit) {
                        let r = self.move_data_event(EventId::Hit, data, mv, Holder::Mon(t), Some(source), None);
                        did_something = MoveResult::combine(did_something, MoveResult::from_relay(r));
                    }
                    if !is_self && !is_secondary {
                        self.run_event(EventId::Hit, Holder::Mon(t), Some(source), effect, None);
                    }
                }
            }
            if data_selfdestruct(data, def) == Some(ps_dex::SelfDestruct::IfHit) && damage != MoveResult::False {
                self.faint(source, Some(source), effect);
            }
            if data_self_switch(data, def).is_some() {
                // No Commander in gens 1-3.
                if self.can_switch(source.side) != 0 {
                    did_something = MoveResult::True;
                } else {
                    did_something = MoveResult::combine(did_something, MoveResult::False);
                }
            }
            // The move did not fail because it did not try to do anything.
            if did_something == MoveResult::Undefined {
                did_something = MoveResult::True;
            }
            let as_damage = if did_something == MoveResult::Null { MoveResult::False } else { did_something };
            damage = MoveResult::combine(damage, as_damage);
            did_anything = MoveResult::combine(did_anything, did_something);
        }

        if !did_anything.truthy()
            && did_anything != MoveResult::Int(0)
            && data_self(data, def).is_none()
            && data_selfdestruct(data, def).is_none()
        {
            if !is_self && !is_secondary && did_anything == MoveResult::False {
                let mon = self.ident(source);
                self.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
                self.attr_last_move_still();
            }
            self.debug(ps_protocol::Text::new(ps_protocol::TextId::MoveFailedNothing));
        } else if def.self_switch().is_some() && self.mon(source).hp != 0 {
            self.mon_mut(source).switch_flag = SwitchFlag::Move(id);
        }
        damage
    }

    /// Whether `moveData` has its own handler for `event` (PS's
    /// `if (moveData.onHit)`).
    fn move_data_has(&self, data: MoveData, mv: MoveRef, event: EventId) -> bool {
        match data {
            MoveData::Move => self.registry.handler(self.mv(mv).effect(), crate::effect::Prefix::None, event).is_some(),
            MoveData::Secondary(s) => !s.handlers.is_empty(),
            MoveData::SelfEffect(s) => !s.handlers.is_empty(),
            MoveData::Damage(_) => false,
        }
    }

    /// The `self` effect: a stat drop rolls once per move (`selfDropped`).
    // PS: sim/battle-actions.ts selfDrops
    fn self_drops<S: GenScripts>(&mut self, target: HitTarget, source: MonRef, mv: MoveRef, data: MoveData, is_secondary: bool) {
        if target == HitTarget::False {
            return;
        }
        let def = self.dex.move_def(self.mv(mv).id);
        let Some(self_effect) = data_self(data, def) else { return };
        if self.mv(mv).self_dropped {
            return;
        }
        if !is_secondary && self_effect.boosts.is_some() {
            let secondary_roll = self.prng.random_n(100);
            if self_effect.chance.is_none_or(|c| secondary_roll < u32::from(c)) {
                S::move_hit(self, Some(source), source, mv, MoveData::SelfEffect(self_effect), is_secondary, true);
            }
            if def.multihit().is_none() {
                self.mv_mut(mv).self_dropped = true;
            }
        } else {
            S::move_hit(self, Some(source), source, mv, MoveData::SelfEffect(self_effect), is_secondary, true);
        }
    }

    /// The secondary effects, each rolled out of 100.
    // PS: sim/battle-actions.ts secondaries
    fn secondaries<S: GenScripts>(&mut self, target: HitTarget, source: MonRef, mv: MoveRef, data: MoveData, is_self: bool) {
        let def = self.dex.move_def(self.mv(mv).id);
        let secondaries = data_secondaries(data, def);
        if secondaries.is_empty() || target == HitTarget::False {
            return;
        }
        // `runEvent('ModifySecondaries', target, source, moveData,
        // secondaries.slice())`: Shield Dust (gen 3's only handler) would
        // bring the list relay.
        let effect = self.mv(mv).effect();
        let holder = target.mon().map_or(Holder::None, Holder::Mon);
        let r = self.run_event(EventId::ModifySecondaries, holder, Some(source), effect, Some(Relay::TRUE));
        assert_eq!(r, Relay::TRUE, "ModifySecondaries handler: the secondaries relay is not ported");
        for secondary in secondaries {
            let secondary_roll = self.prng.random_n(100);
            // Boosts and self effects overflow past 255 in gens 8 and before.
            let overflow = (secondary.boosts.is_some() || secondary.self_effect.is_some()) && self.gen <= 8;
            let hit = match secondary.chance {
                None => true,
                Some(c) => {
                    let chance = if overflow { u32::from(c) % 256 } else { u32::from(c) };
                    secondary_roll < chance
                }
            };
            if hit {
                S::move_hit(self, target.mon(), source, mv, MoveData::Secondary(secondary), true, is_self);
            }
        }
    }

    /// Phazing (Roar, Whirlwind): `DragOut`, then `forceSwitchFlag`.
    // PS: sim/battle-actions.ts forceSwitch
    fn force_switch(&mut self, damage: MoveResult, target: HitTarget, source: MonRef, mv: MoveRef) -> MoveResult {
        let mut damage = damage;
        if let HitTarget::Mon(t) = target {
            if self.mon(t).hp > 0 && self.mon(source).hp > 0 && self.can_switch(t.side) != 0 {
                let effect = self.mv(mv).effect();
                let hit_result = self.run_event(EventId::DragOut, Holder::Mon(t), Some(source), effect, None);
                if hit_result.truthy() {
                    self.mon_mut(t).force_switch_flag = true;
                } else if hit_result == Relay::FAIL && self.mv(mv).category == MoveCategory::Status {
                    let mon = self.ident(source);
                    self.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
                    self.attr_last_move_still();
                    damage = MoveResult::False;
                }
            }
        }
        damage
    }

    // ---- getDamage ----------------------------------------------------

    /// The damage formula up to the gen's `modifyDamage`: base power, the
    /// crit roll, the stats and the base damage. `None` from `modify`
    /// means the gen's `modifyDamage` is called by the caller (PS's
    /// `this.modifyDamage`, which gens override).
    // PS: sim/battle-actions.ts getDamage
    pub fn base_get_damage(&mut self, source: MonRef, target: MonRef, mv: MoveRef, data: MoveData, suppress: bool) -> DamageStep {
        // A secondary or `self` effect has no type, no fixed damage and no
        // base power: `undefined`.
        if !matches!(data, MoveData::Move) {
            return DamageStep::Done(MoveResult::Undefined);
        }
        if !self.run_immunity(target, Some(mv), !suppress) {
            return DamageStep::Done(MoveResult::False);
        }
        if self.mv(mv).ohko {
            let hp = if self.gen == 3 { self.mon(target).hp } else { self.mon(target).maxhp };
            return DamageStep::Done(MoveResult::Int(i32::from(hp)));
        }
        if let Some(r) = self.move_callback(mv, EventId::DamageCallback, source, Some(target)) {
            return DamageStep::Done(MoveResult::from_relay(r));
        }
        match self.mv(mv).damage {
            Some(ps_dex::FixedDamage::Level) => return DamageStep::Done(MoveResult::Int(i32::from(self.mon(source).level))),
            Some(ps_dex::FixedDamage::Hp(n)) if n != 0 => return DamageStep::Done(MoveResult::Int(i32::from(n))),
            _ => {}
        }

        // `getCategory(move)`: the move object's own category.
        let category = self.mv(mv).category;

        let mut base_power = MoveResult::Int(i32::from(self.mv(mv).base_power));
        if let Some(r) = self.move_callback(mv, EventId::BasePowerCallback, source, Some(target)) {
            base_power = MoveResult::from_relay(r);
        }
        let base_power = match base_power {
            MoveResult::Int(0) => return DamageStep::Done(MoveResult::Undefined),
            MoveResult::Int(n) => n.max(1),
            other if !other.truthy() => return DamageStep::Done(other),
            other => panic!("basePowerCallback answered {other:?}"),
        };

        let id = self.mv(mv).id;
        let def = self.dex.move_def(id);
        let effect = self.mv(mv).effect();
        let crit_ratio = i32::from(def.crit_ratio);
        let crit_ratio =
            match self.run_event(EventId::ModifyCritRatio, Holder::Mon(source), Some(target), effect, Some(Relay::Int(crit_ratio))) {
                Relay::Int(n) => n,
                other => panic!("ModifyCritRatio answered {other:?}"),
            };
        let (crit_ratio, crit_mult): (i32, &[u32]) = if self.gen <= 5 {
            (crit_ratio.clamp(0, 5), &[0, 16, 8, 4, 3, 2])
        } else if self.gen == 6 {
            (crit_ratio.clamp(0, 4), &[0, 16, 8, 2, 1])
        } else {
            (crit_ratio.clamp(0, 4), &[0, 24, 8, 2, 1])
        };

        // `moveHit.crit = move.willCrit || false`, rolled when `willCrit` is
        // undefined (no gen 1-3 move sets it).
        let mut crit = def.bools.has("willCrit");
        if !crit && crit_ratio != 0 {
            crit = self.prng.random_chance(1, crit_mult[crit_ratio as usize]);
        }
        if crit {
            crit = self.run_event(EventId::CriticalHit, Holder::Mon(target), None, effect, None).truthy();
        }
        self.mv_mut(mv).crit = crit;

        // After the crit roll.
        let base_power =
            match self.run_event_full(EventId::BasePower, Holder::Mon(source), Some(target), effect, Some(Relay::Int(base_power)), true, false) {
                Relay::Int(n) => n,
                r if !r.truthy() => 0,
                other => panic!("BasePower answered {other:?}"),
            };
        if base_power == 0 {
            return DamageStep::Done(MoveResult::Int(0));
        }
        let base_power = base_power.max(1);
        // No Max Moves or Terastallization in gens 1-3.

        let level = i64::from(self.mon(source).level);
        // No gen 1-3 move overrides the offensive or defensive Pokemon or
        // stat.
        let (attacker, defender) = (source, target);
        let is_physical = category == MoveCategory::Physical;
        let attack_stat = if is_physical { Stat::Atk } else { Stat::Spa };
        let defense_stat = if is_physical { Stat::Def } else { Stat::Spd };

        let mut atk_boosts = self.mon(attacker).boosts[attack_stat as usize];
        let mut def_boosts = self.mon(defender).boosts[defense_stat as usize];

        let mut ignore_negative_offensive = def.bools.has("ignoreNegativeOffensive");
        let mut ignore_positive_defensive = def.bools.has("ignorePositiveDefensive");
        if crit {
            ignore_negative_offensive = true;
            ignore_positive_defensive = true;
        }
        let ignore_offensive = self.mv(mv).ignore_offensive || (ignore_negative_offensive && atk_boosts < 0);
        let ignore_defensive = self.mv(mv).ignore_defensive || (ignore_positive_defensive && def_boosts > 0);
        if ignore_offensive {
            self.debug(ps_protocol::Text::new(ps_protocol::TextId::NegatingAtkBoost));
            atk_boosts = 0;
        }
        if ignore_defensive {
            self.debug(ps_protocol::Text::new(ps_protocol::TextId::NegatingDefBoost));
            def_boosts = 0;
        }

        let attack = self.calculate_stat(attacker, attack_stat, atk_boosts, Some(source));
        let defense = self.calculate_stat(defender, defense_stat, def_boosts, Some(target));

        // The stat modifiers.
        let atk_event = if category == MoveCategory::Physical { EventId::ModifyAtk } else { EventId::ModifySpA };
        let def_event = if defense_stat == Stat::Def { EventId::ModifyDef } else { EventId::ModifySpD };
        let attack = match self.run_event(atk_event, Holder::Mon(source), Some(target), effect, Some(Relay::Int(attack))) {
            Relay::Int(n) => i64::from(n),
            other => panic!("{atk_event:?} answered {other:?}"),
        };
        let mut defense = match self.run_event(def_event, Holder::Mon(target), Some(source), effect, Some(Relay::Int(defense))) {
            Relay::Int(n) => i64::from(n),
            other => panic!("{def_event:?} answered {other:?}"),
        };

        if self.gen <= 4 && defense_stat == Stat::Def {
            let name = def.name.as_str();
            if name == "Explosion" || name == "Self-Destruct" {
                defense = (defense / 2).max(1);
            }
        }

        // `int(int(int(2 * L / 5 + 2) * A * P / D) / 50)`.
        let tr = self.trunc;
        let a = tr.ratio(2 * level + 10, 5, 0);
        let base_damage = tr.ratio(tr.ratio(tr.int(a * i64::from(base_power) * attack, 0), defense, 0), 50, 0);
        DamageStep::Modify(base_damage)
    }

    /// PS `randomizer(baseDamage)`: 85% to 100%.
    // PS: sim/battle.ts randomizer
    pub fn randomizer(&mut self, base_damage: i64) -> i64 {
        let tr = self.trunc;
        let r = i64::from(self.prng.random_n(16));
        tr.ratio(tr.int(base_damage * (100 - r), 0), 100, 0)
    }

    // ---- boosts ---------------------------------------------------------

    /// PS `battle.boost(boost, target, source, effect, isSecondary, isSelf)`.
    // PS: sim/battle.ts boost
    #[allow(clippy::too_many_arguments)]
    pub fn base_boost(
        &mut self,
        boost: [i8; 7],
        target: MonRef,
        source: Option<MonRef>,
        effect: EffectId,
        is_secondary: bool,
        is_self: bool,
    ) -> MoveResult {
        let source = source.or(self.event.source);
        let effect = if effect.is_none() { self.frame.effect } else { effect };
        if self.mon(target).hp == 0 {
            return MoveResult::Int(0);
        }
        if !self.mon(target).is_active {
            return MoveResult::False;
        }
        // `runEvent('ChangeBoost' / 'TryBoost', ..., {...boost})`: the boosts
        // table relay arrives with the first handler (Mist, Clear Body).
        let r = self.run_event(EventId::ChangeBoost, Holder::Mon(target), source, effect, Some(Relay::TRUE));
        assert_eq!(r, Relay::TRUE, "ChangeBoost handler: the boosts relay is not ported");
        let boost = self.get_capped_boost(target, boost);
        let r = self.run_event(EventId::TryBoost, Holder::Mon(target), source, effect, Some(Relay::TRUE));
        assert_eq!(r, Relay::TRUE, "TryBoost handler: the boosts relay is not ported");
        let mut success = MoveResult::Null;
        let effect_type = self.effect_type(effect);
        let boosted = is_secondary;
        for (i, &b) in boost.iter().enumerate() {
            // `getCappedBoost` keeps every stat it was given: `Some` here.
            let Some(b) = b else { continue };
            let mut current = [0i8; 7];
            current[i] = b;
            let mut boost_by = self.boost_by(target, current);
            let mut unboost = false;
            if b < 0 || self.mon(target).boosts[i] == -6 {
                unboost = true;
                boost_by = -boost_by;
            }
            let mon = self.ident(target);
            let stat = ps_protocol::Stat::ALL[i];
            let line = |amount: i32| {
                let amount = amount.unsigned_abs() as u8;
                if unboost {
                    Msg::Unboost { mon, stat, amount, kw: Kw::NONE }
                } else {
                    Msg::Boost { mon, stat, amount, kw: Kw::NONE }
                }
            };
            if boost_by != 0 {
                success = MoveResult::True;
                match effect {
                    // `bellydrum`, `angerpoint`, `zpower`: none here.
                    EffectId::None => {}
                    _ => match effect_type {
                        crate::effect::EffectType::Move => self.add(line(boost_by)),
                        crate::effect::EffectType::Item => panic!("unported: the [from] item line of a boost ({effect:?})"),
                        _ => {
                            if effect_type == crate::effect::EffectType::Ability && !boosted {
                                panic!("unported: the -ability line of a boost from {effect:?}");
                            }
                            self.add(line(boost_by));
                        }
                    },
                }
                self.run_event(EventId::AfterEachBoost, Holder::Mon(target), source, effect, Some(Relay::TRUE));
            } else if effect_type == crate::effect::EffectType::Ability {
                if is_secondary || is_self {
                    self.add(line(boost_by));
                }
            } else if !is_secondary && !is_self {
                self.add(line(boost_by));
            }
        }
        self.run_event(EventId::AfterBoost, Holder::Mon(target), source, effect, Some(Relay::TRUE));
        if success == MoveResult::True {
            if boost.iter().any(|b| b.is_some_and(|x| x > 0)) {
                self.mon_mut(target).stats_raised_this_turn = true;
            }
            if boost.iter().any(|b| b.is_some_and(|x| x < 0)) {
                self.mon_mut(target).stats_lowered_this_turn = true;
            }
        }
        success
    }

    /// PS `pokemon.getCappedBoost(boosts)`: each nonzero stat, capped to
    /// stay within -6 and 6 (`None` for a stat left out).
    // PS: sim/pokemon.ts getCappedBoost
    pub fn get_capped_boost(&self, m: MonRef, boosts: [i8; 7]) -> [Option<i8>; 7] {
        let current = self.mon(m).boosts;
        core::array::from_fn(|i| (boosts[i] != 0).then(|| (current[i] + boosts[i]).clamp(-6, 6) - current[i]))
    }

    /// PS `pokemon.boostBy(boosts)`: the last stat's change.
    // PS: sim/pokemon.ts boostBy
    pub fn boost_by(&mut self, m: MonRef, boosts: [i8; 7]) -> i32 {
        let capped = self.get_capped_boost(m, boosts);
        let mut delta = 0;
        for (i, c) in capped.iter().enumerate() {
            let Some(d) = *c else { continue };
            delta = i32::from(d);
            self.mon_mut(m).boosts[i] += d;
        }
        delta
    }

    /// PS `pokemon.clearBoosts()`.
    // PS: sim/pokemon.ts clearBoosts
    pub fn base_clear_boosts(&mut self, m: MonRef) {
        self.mon_mut(m).boosts = [0; 7];
    }

    // ---- Pokemon methods ----------------------------------------------

    /// PS `pokemon.deductPP(move, amount)`: from the move's own slot.
    /// `amount` 0 is PS's `null` (1).
    // PS: sim/pokemon.ts deductPP, getMoveData
    pub fn base_deduct_pp(&mut self, m: MonRef, mv: MoveId, amount: i32) -> i32 {
        // `getMoveData(move)`: the first slot with the move's id.
        let Some(slot) = self.mon(m).move_slots.iter().position(|s| !s.id.is_none() && s.id == mv) else { return 0 };
        let mon = self.mon_mut(m);
        mon.move_slots[slot].used = true;
        // `moveSlots` share their objects with `baseMoveSlots` until
        // Transform or Mimic replaces a slot.
        let shared = !mon.move_slots[slot].virtual_ && !mon.transformed;
        if shared {
            mon.base_move_slots[slot].used = true;
        }
        if mon.move_slots[slot].pp == 0 {
            return 0;
        }
        let mut amount = if amount == 0 { 1 } else { amount };
        let mut pp = i32::from(mon.move_slots[slot].pp) - amount;
        if pp < 0 {
            amount += pp;
            pp = 0;
        }
        mon.move_slots[slot].pp = pp as u8;
        if shared {
            mon.base_move_slots[slot].pp = pp as u8;
        }
        amount
    }

    /// PS `pokemon.trySetStatus(status, source, sourceEffect)`.
    // PS: sim/pokemon.ts trySetStatus
    pub fn try_set_status(&mut self, m: MonRef, status: ps_dex::ConditionId, source: Option<MonRef>, effect: EffectId) -> Relay {
        let current = self.mon(m).status;
        let status = if current.is_none() { status } else { current };
        self.set_status(m, status, source, effect, false)
    }

    /// PS `pokemon.isAdjacent(pokemon2)` in singles.
    // PS: sim/pokemon.ts isAdjacent
    pub fn is_adjacent(&self, a: MonRef, b: MonRef) -> bool {
        if self.mon(a).fainted || self.mon(b).fainted {
            return false;
        }
        a != b
    }

    /// PS `pokemon.isSemiInvulnerable()`. Sky Drop is gen 5.
    // PS: sim/pokemon.ts isSemiInvulnerable
    pub fn is_semi_invulnerable(&self, m: MonRef) -> bool {
        ["fly", "bounce", "dive", "dig", "phantomforce", "shadowforce"].iter().any(|k| self.has_volatile_key(m, k))
    }

    /// PS `pokemon.getMoveTargets(move, target)` in singles (one active
    /// per side, no ally).
    // PS: sim/pokemon.ts getMoveTargets
    pub fn get_move_targets(&mut self, pokemon: MonRef, mv: MoveRef, target: MonRef) -> MoveTargets {
        let move_target = self.mv(mv).target;
        // `side.allies(all)` / `side.foes(all)`: the active Pokemon, only
        // those with HP unless `all`.
        let foe = self.active_at(1 - pokemon.side, 0);
        let alive = |b: &Battle, m: Option<MonRef>| m.filter(|&m| b.mon(m).hp != 0);
        let mut targets = MonList::default();
        match move_target {
            MoveTarget::All | MoveTarget::FoeSide | MoveTarget::AllySide | MoveTarget::AllyTeam => {
                if !matches!(move_target, MoveTarget::FoeSide) {
                    targets.push(alive(self, Some(pokemon)));
                }
                if !matches!(move_target, MoveTarget::AllySide | MoveTarget::AllyTeam) {
                    targets.push(foe);
                }
                if let Some(last) = targets.last() {
                    if !targets.contains(target) {
                        self.retarget_last_move(last);
                    }
                }
            }
            MoveTarget::AllAdjacent | MoveTarget::AllAdjacentFoes => {
                // `adjacentAllies()` is empty in singles; `adjacentFoes()`.
                targets.push(alive(self, foe));
                if let Some(last) = targets.last() {
                    if !targets.contains(target) {
                        self.retarget_last_move(last);
                    }
                }
            }
            MoveTarget::Allies => {
                targets.push(alive(self, Some(pokemon)));
            }
            _ => {
                let selected = target;
                let mut target = target;
                if self.mon(target).fainted && target.side != pokemon.side {
                    // A fainted foe: the move is retargeted.
                    let am = self.action_move_of(mv);
                    match self.get_random_target(pokemon, am) {
                        None => return MoveTargets::default(),
                        Some(t) => target = t,
                    }
                }
                // `activePerHalf` 1: no `RedirectTarget`; no `smartTarget`.
                targets.push(Some(target));
                let futuremove = !self.mv(mv).id.is_none() && self.dex.move_def(self.mv(mv).id).flags.has("futuremove");
                if self.mon(target).fainted && !futuremove {
                    return MoveTargets::default();
                }
                if selected != target {
                    self.retarget_last_move(target);
                }
            }
        }
        // Pressure's apparent targets.
        let mut pressure = targets;
        if move_target == MoveTarget::FoeSide {
            pressure = MonList::default();
        }
        if !self.mv(mv).id.is_none() && self.dex.move_def(self.mv(mv).id).flags.has("mustpressure") {
            pressure = MonList::default();
            pressure.push(alive(self, foe));
        }
        MoveTargets { targets, pressure }
    }

    /// PS `retargetLastMove(newTarget)`: the `|move|` line's target.
    // PS: sim/battle.ts retargetLastMove
    pub fn retarget_last_move(&mut self, new_target: MonRef) {
        let Some(i) = self.last_move_line else { return };
        let ident = self.ident(new_target);
        if let Msg::Move { target, .. } = &mut self.log.as_mut_slice()[i as usize] {
            *target = Some(ident);
        }
    }
}

/// What [`Battle::base_get_damage`] leaves to the gen's `modifyDamage`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageStep {
    /// `getDamage` returned before the formula.
    Done(MoveResult),
    /// The base damage, for `modifyDamage`.
    Modify(i64),
}
