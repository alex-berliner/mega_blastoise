//! Gen 1's move pipeline: PS's `data/mods/gen1/scripts.ts`, ported line by
//! line with its control flow.
//!
//! Gen 1 overrides `runMove`, `useMove`, `useMoveInner`, `tryMoveHit`,
//! `moveHit` and `getDamage` (actions), `deductPP`, `getStat`,
//! `modifyStat`, `boostBy` and `clearBoosts` (pokemon) and `boost`
//! (battle). Gen 2's overrides of the same functions are all replaced, and
//! everything else is inherited: gen 4's `runSwitch` and
//! `applyRecoilDamage`, gen 3's `getActionSpeed` (see `battle.rs`,
//! `stats.rs` and `actions.rs`).
//!
//! The pipeline reads several conditions by id (`lockedmove`, `rage`,
//! `partialtrappinglock`, `twoturnmove`, `substitute`, `mustrecharge`,
//! `focusenergy`, `reflect`, `lightscreen`). Their ports (tasks G1-1 to
//! G1-6) own their state; the fields this file reads are documented where
//! it reads them, and the condition ports must store them there:
//! - `lockedmove`: `time` in `data[0]`, `accuracy` in `data[1]`;
//! - `rage`: `accuracy` in `data[1]`;
//! - `partialtrappinglock`: `locked` in `data[0]` (as `endTurn` already
//!   reads it, see [`EffectState::data_mon`](crate::state::EffectState)),
//!   `damage` in `data[1]`.
//!
//! An effect whose handlers are not ported panics `unported: <id>` when the
//! pipeline reaches it ([`Battle::require_ported`]).
//!
//! PS: data/mods/gen1/scripts.ts (whole).

use ps_dex::{FixedDamage, MoveCategory, MoveId, Type};
use ps_protocol::{Arg, ArgValue, Kw, KwFlags, Msg, MoveNum, TextArg, TextId};

use crate::actions::{
    data_boosts, data_pseudo_weather, data_self, data_side_condition, data_status, data_volatile, ActiveMove, FakeMove,
    GenScripts, MoveData, MoveRef, MoveResult,
};
use crate::choice::SelectedMove;
use crate::effect::{EffectId, EventId};
use crate::queue::ActionMove;
use crate::relay::Relay;
use crate::state::{Battle, Holder, MonRef, StateRef, SwitchFlag};
use crate::stats::Stat;
use ps_dex::gen1::{conditions, moves};

/// PS's `TWO_TURN_MOVES`.
const TWO_TURN_MOVES: [MoveId; 6] = [moves::DIG, moves::FLY, moves::RAZORWIND, moves::SKULLBASH, moves::SKYATTACK, moves::SOLARBEAM];

/// PS's `SKIP_LASTDAMAGE`.
const SKIP_LASTDAMAGE: [MoveId; 24] = [
    moves::CONFUSERAY,
    moves::CONVERSION,
    moves::COUNTER,
    moves::FOCUSENERGY,
    moves::GLARE,
    moves::HAZE,
    moves::LEECHSEED,
    moves::LIGHTSCREEN,
    moves::MIMIC,
    moves::MIST,
    moves::POISONGAS,
    moves::POISONPOWDER,
    moves::RECOVER,
    moves::REFLECT,
    moves::REST,
    moves::SOFTBOILED,
    moves::SPLASH,
    moves::STUNSPORE,
    moves::SUBSTITUTE,
    moves::SUPERSONIC,
    moves::TELEPORT,
    moves::THUNDERWAVE,
    moves::TOXIC,
    moves::TRANSFORM,
];

/// Gen 1's accuracy and evasion table, in percent, for stages -6 to +6.
const BOOST_TABLE: [i32; 13] = [25, 28, 33, 40, 50, 66, 100, 150, 200, 250, 300, 350, 400];

/// Gen 1's scripts. See the module doc.
pub struct Gen1Scripts;

impl Battle {
    /// Gen 1's `pokemon.modifyStat(statName, modifier)`: `modifier` is the
    /// JS number PS multiplies by.
    // PS: data/mods/gen1/scripts.ts modifyStat
    pub fn modify_stat(&mut self, m: MonRef, stat: usize, modifier: f64) {
        // `Math.floor` of a non-negative double: the cast truncates.
        let v = f64::from(self.mon(m).modified_stats[stat]) * modifier;
        self.mon_mut(m).modified_stats[stat] = (v as i64).max(1) as u16;
    }

    /// Gen 1's `pokemon.boostBy(boost)` for one stat (`boost` gives every
    /// stat but one a 0): PS's `false`, `true`, or `0` for a stat already
    /// at 999 (or at 1).
    // PS: data/mods/gen1/scripts.ts boostBy
    pub fn boost_by_gen1(&mut self, m: MonRef, boost: [i8; 7]) -> MoveResult {
        let mut changed = MoveResult::False;
        for i in 0..7 {
            let delta = boost[i];
            if delta == 0 {
                continue;
            }
            let b = self.mon(m).boosts[i];
            if delta > 0 && b >= 6 {
                continue;
            }
            if delta < 0 && b <= -6 {
                continue;
            }
            if i >= 5 {
                // Accuracy and evasion.
                let mon = self.mon_mut(m);
                mon.boosts[i] = (mon.boosts[i] + delta).clamp(-6, 6);
                changed = MoveResult::True;
                continue;
            }
            // Not accuracy or evasion: change `modifiedStats` too.
            if delta > 0 {
                let mon = self.mon_mut(m);
                if mon.modified_stats[i] == 999 {
                    // The intended maximum.
                    mon.boosts[i] = (mon.boosts[i] + delta).min(6);
                    mon.boosts[i] -= 1;
                    changed = MoveResult::Int(0);
                } else {
                    mon.boosts[i] = (mon.boosts[i] + delta).min(6);
                    changed = MoveResult::True;
                }
            }
            if delta < 0 {
                let mon = self.mon_mut(m);
                if mon.modified_stats[i] == 1 {
                    // The minimum.
                    mon.boosts[i] = (mon.boosts[i] + delta).max(-6);
                    mon.boosts[i] += 1;
                    changed = MoveResult::Int(0);
                } else {
                    mon.boosts[i] = (mon.boosts[i] + delta).max(-6);
                    changed = MoveResult::True;
                }
            }
            if changed.truthy() {
                // Recalculate the modified stat.
                let mon = self.mon_mut(m);
                mon.modified_stats[i] = mon.stored_stats[i];
                let b = mon.boosts[i];
                if b >= 0 {
                    self.modify_stat(m, i, [1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0][b as usize]);
                } else {
                    let pct = [100.0, 66.0, 50.0, 40.0, 33.0, 28.0, 25.0][(-b) as usize];
                    self.modify_stat(m, i, pct / 100.0);
                }
                if delta > 0 && self.mon(m).modified_stats[i] > 999 {
                    // Cap the stat at 999.
                    self.mon_mut(m).modified_stats[i] = 999;
                }
            }
        }
        changed
    }

}

impl GenScripts for Gen1Scripts {
    /// The main move function: the `BeforeMove` event and gen 1's PP
    /// deduction.
    // PS: data/mods/gen1/scripts.ts runMove
    fn run_move(b: &mut Battle, action_move: ActionMove, pokemon: MonRef, target_loc: i8, source_effect: EffectId) {
        let mut source_effect = source_effect;
        let target = b.get_target(pokemon, action_move, target_loc, None);
        let mut mv = b.get_active_move(action_move);
        let struggle = |b: &Battle, mv: MoveRef| b.mv(mv).id == moves::STRUGGLE;
        if !struggle(b, mv) {
            let effect = b.mv(mv).effect();
            let changed = b.run_event(EventId::OverrideAction, Holder::Mon(pokemon), target, effect, None);
            match changed {
                Relay::Move(id) => mv = b.new_active_move(ActiveMove::from_dex(b.dex, id)),
                r if !r.truthy() || r == Relay::TRUE => {}
                other => panic!("OverrideAction answered {other:?}"),
            }
        }

        // `abortMove`.
        let abort = |b: &mut Battle, mv: MoveRef| {
            b.clear_active_move(true);
            let effect = b.mv(mv).effect();
            b.run_event(EventId::AfterMoveSelf, Holder::Mon(pokemon), target, effect, None);
        };

        if b.mv(mv).fake == FakeMove::CannotMove {
            if b.mon(pokemon).status == conditions::SLP {
                b.hint(TextId::HintGen1CannotMoveSleep, false, None);
            } else if b.get_locked_move(pokemon).is_some() {
                b.hint(TextId::HintGen1HazeLockedMove, false, None);
            } else if b.get_semi_locked_move(pokemon, false).is_some() {
                b.hint(TextId::HintGen1HazeBide, false, None);
            }
            abort(b, mv);
            return;
        }
        if b.mv(mv).fake == FakeMove::None {
            b.require_ported(b.mv(mv).effect());
        }

        if let Some(t) = target {
            if b.mon(t).sub_fainted {
                b.mon_mut(t).sub_fainted = false;
            }
        }

        b.set_active_move(Some(mv), Some(pokemon), target);

        let effect = b.mv(mv).effect();
        if !b.mon(pokemon).move_this_turn.is_none()
            || !b.run_event(EventId::BeforeMove, Holder::Mon(pokemon), target, effect, None).truthy()
        {
            abort(b, mv);
            return;
        }
        if b.move_callback(mv, EventId::BeforeMoveCallback, pokemon, target).is_some_and(Relay::truthy) {
            abort(b, mv);
            return;
        }

        if !struggle(b, mv) {
            let locked = b.get_locked_move(pokemon).is_some() || b.get_semi_locked_move(pokemon, false).is_some();
            if locked {
                source_effect = b.mv(mv).effect();
            }

            // Locked moves don't deduct PP; two-turn moves deduct it on
            // their second turn.
            let id = b.mv(mv).id;
            let two_turn = TWO_TURN_MOVES.contains(&id);
            if (!locked && !two_turn) || b.has_vol(pokemon, conditions::TWOTURNMOVE) {
                let slot = b.sides[pokemon.side as usize].last_selected_move_slot;
                if b.get_move_slot(pokemon, slot).is_some() {
                    Self::deduct_pp(b, pokemon, id, 0);
                }
            }

            let same = match b.sides[pokemon.side as usize].last_enemy_selected_move {
                SelectedMove::Move(m) => b.mv(mv).fake == FakeMove::None && m == id,
                SelectedMove::NoMove => b.mv(mv).fake == FakeMove::NoMove,
                SelectedMove::CannotMove => b.mv(mv).fake == FakeMove::CannotMove,
            };
            if !locked && !same {
                b.hint(TextId::DesyncClauseActivated, false, None);
                b.hint(TextId::HintGen1DesyncDefault, false, None);
            }
        }

        if b.mv(mv).fake == FakeMove::NoMove {
            b.hint(TextId::HintGen1NoMove, false, None);
            // Fissure's animation, 102 base power, `???` type, Special,
            // `100 * 81 / 256` accuracy (the name stays Fissure's).
            let am = ActiveMove {
                base_power: 102,
                ohko: false,
                type_: Type::None,
                category: MoveCategory::Special,
                accuracy_x256: Some(100 * 81),
                ..ActiveMove::from_dex(b.dex, moves::FISSURE)
            };
            mv = b.new_active_move(am);
        }

        Self::use_move(b, mv, pokemon, Some(target), source_effect);
    }

    /// `AfterMoveSelf` and the partial trapping details after the move.
    // PS: data/mods/gen1/scripts.ts useMove
    fn use_move(b: &mut Battle, mv: MoveRef, pokemon: MonRef, target: Option<Option<MonRef>>, source_effect: EffectId) -> bool {
        let mut source_effect = source_effect;
        if source_effect.is_none() && !b.frame.effect.is_none() {
            source_effect = b.frame.effect;
        }
        if b.mv(mv).fake == FakeMove::None {
            b.require_ported(b.mv(mv).effect());
        }
        // `baseMove = dex.moves.get(move)` and `getActiveMove(baseMove)`:
        // both are `move` itself, an `ActiveMove` already.
        let mut target = match target {
            Some(t) => t,
            None => {
                let am = b.action_move_of(mv);
                b.get_random_target(pokemon, am)
            }
        };
        if b.move_targets_self(mv) {
            target = Some(pokemon);
        }
        if !source_effect.is_none() {
            b.mv_mut(mv).source_effect = source_effect;
        }

        let source_key = match source_effect {
            EffectId::Move(m) => Some(m),
            _ => None,
        };
        if source_key.is_some_and(|m| m == moves::METRONOME || m == moves::MIRRORMOVE) {
            let id = b.mv(mv).id;
            let slot = b.sides[pokemon.side as usize].last_selected_move_slot;
            if TWO_TURN_MOVES.contains(&id) && b.get_move_slot(pokemon, slot).is_some() {
                Self::deduct_pp(b, pokemon, id, -1);
            }
            let count = b.mon(pokemon).base_move_slots.iter().filter(|s| !s.id.is_none()).count();
            if b.mon(pokemon).transformed && (slot as usize) < count {
                let ms = &mut b.mon_mut(pokemon).base_move_slots[slot as usize];
                ms.pp = (ms.pp + 1) % 64;
            }
        }

        let effect = b.mv(mv).effect();
        let relay = b.mv(mv).relay();
        b.single_event(EventId::ModifyMove, effect, StateRef::Scratch, 0, Holder::Mon(pokemon), target, effect, Some(relay));
        // `baseMove.target !== move.target`: the same object.
        let r = b.run_event(EventId::ModifyMove, Holder::Mon(pokemon), target, effect, Some(relay));
        assert!(matches!(r, Relay::Move(_)), "ModifyMove answered {r:?}");

        let id = b.mv(mv).id;
        b.sides[pokemon.side as usize].last_move = id;
        b.sides[pokemon.side as usize].last_enemy_move = id;
        // The charging turn of a two-turn move does not update `lastMove`.
        if !TWO_TURN_MOVES.contains(&id) || b.has_vol(pokemon, conditions::TWOTURNMOVE) {
            b.move_used(pokemon, mv, 0);
        }

        let move_result = Self::use_move_inner(b, mv, pokemon, Some(target), source_effect);

        if id != moves::METRONOME {
            let foe_last = b.active_at(1 - pokemon.side, 0).map_or(MoveId::NONE, |f| b.mon(f).last_move);
            if id != moves::MIRRORMOVE || foe_last.is_none() || foe_last == moves::MIRRORMOVE {
                b.run_event(EventId::AfterMove, Holder::Mon(pokemon), target, effect, None);
                if target.is_none_or(|t| b.mon(t).hp > 0) {
                    b.run_event(EventId::AfterMoveSelf, Holder::Mon(pokemon), target, effect, None);
                }
            }
        }
        move_result
    }

    /// Uses the move (`ModifyMove` again), then its after effects.
    // PS: data/mods/gen1/scripts.ts useMoveInner
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
        let mut target = match target {
            Some(t) => t,
            None => {
                let am = b.action_move_of(mv);
                b.get_random_target(pokemon, am)
            }
        };
        if b.move_targets_self(mv) {
            target = Some(pokemon);
        }
        if !source_effect.is_none() {
            b.mv_mut(mv).source_effect = source_effect;
        }

        b.set_active_move(Some(mv), Some(pokemon), target);

        let effect = b.mv(mv).effect();
        let relay = b.mv(mv).relay();
        b.single_event(EventId::ModifyMove, effect, StateRef::Scratch, 0, Holder::Mon(pokemon), target, effect, Some(relay));
        let r = b.run_event(EventId::ModifyMove, Holder::Mon(pokemon), target, effect, Some(relay));
        // `if (!move) return false`: a handler could only clear it.
        assert!(matches!(r, Relay::Move(_)), "ModifyMove answered {r:?}");

        if b.mon(pokemon).fainted {
            return false;
        }

        let kw = if source_effect.is_none() { Kw::NONE } else { Kw::from(b.move_from(source_effect)) };
        let user = b.ident(pokemon);
        let tgt = target.map(|t| b.ident(t));
        let id = b.mv(mv).id;
        b.add_move(Msg::Move { user, mv: MoveNum(id.0), target: tgt, kw });

        if !b.single_event(EventId::Try, effect, StateRef::Scratch, 0, Holder::Mon(pokemon), target, effect, None).truthy() {
            return true;
        }
        if !b.single_event(EventId::TryMove, effect, StateRef::Scratch, 0, Holder::Mon(pokemon), target, effect, None).truthy()
            || !b.run_event(EventId::TryMove, Holder::Mon(pokemon), target, effect, None).truthy()
        {
            return true;
        }

        // `move.ignoreImmunity` is never undefined: the dex sets it.
        let Some(target) = target.filter(|&t| !b.mon(t).fainted) else {
            b.attr_last_move(KwFlags::NOTARGET);
            b.add(Msg::NoTarget(None));
            return true;
        };
        // Store 0 damage for last damage if the move is not in the list.
        if !SKIP_LASTDAMAGE.contains(&id) {
            b.last_damage = 0;
        }

        let damage = Self::try_move_hit(b, target, pokemon, mv);

        // Disable and Self-Destruct/Explosion build Rage, hit or miss.
        let selfdestruct = b.dex.move_def(id).selfdestruct().is_some();
        if b.mon(target).boosts[0] < 6 && (selfdestruct || id == moves::DISABLE) && b.has_vol(target, conditions::RAGE) {
            let rage = conditions::RAGE;
            b.boost([1, 0, 0, 0, 0, 0, 0], target, Some(pokemon), EffectId::Condition(rage));
            b.hint_text(ps_protocol::Text::with1(TextId::HintGen1RageBuild, TextArg::mv(MoveNum(id.0))), true, None);
        }

        if damage == MoveResult::False {
            b.single_event(EventId::MoveFail, effect, StateRef::Scratch, 0, Holder::Mon(target), Some(pokemon), effect, None);
            return true;
        }

        b.single_event(
            EventId::AfterMoveSecondarySelf,
            effect,
            StateRef::Scratch,
            0,
            Holder::Mon(pokemon),
            Some(target),
            effect,
            None,
        );
        b.run_event(EventId::AfterMoveSecondarySelf, Holder::Mon(pokemon), Some(target), effect, None);
        true
    }

    /// The hit attempt: partial trapping details, gen 1's accuracy (and its
    /// bugs), multi-hit.
    // PS: data/mods/gen1/scripts.ts tryMoveHit
    fn try_move_hit(b: &mut Battle, target: MonRef, pokemon: MonRef, mv: MoveRef) -> MoveResult {
        let mut damage = MoveResult::Int(0);
        let id = b.mv(mv).id;
        let def = b.dex.move_def(id);
        let effect = b.mv(mv).effect();

        // Thrash starts before the move does damage; its last turn confuses.
        let lockedmove = conditions::LOCKEDMOVE;
        if def.self_effect().is_some_and(|s| s.volatile_status == lockedmove) {
            b.require_ported(EffectId::Condition(lockedmove));
            if let Some(st) = b.volatile_state_mut(pokemon, conditions::LOCKEDMOVE) {
                // `time` (data[0]).
                st.data[0] -= 1;
                if st.data[0] == 0 {
                    // Confusion begins even if already confused.
                    let confusion = conditions::CONFUSION;
                    b.require_ported(EffectId::Condition(confusion));
                    b.mon_mut(pokemon).volatiles.remove(EffectId::Condition(confusion));
                    b.add_volatile(pokemon, confusion, Some(pokemon), EffectId::Condition(lockedmove));
                }
            } else {
                b.add_volatile(pokemon, lockedmove, Some(pokemon), effect);
            }
        }

        let selfdestruct = def.selfdestruct().is_some();
        // Semi-invulnerable targets first.
        let hit_result = b.run_event(EventId::Invulnerability, Holder::Mon(target), Some(pokemon), effect, None);
        if hit_result == Relay::FAIL {
            b.attr_last_move(KwFlags::MISS);
            let source = b.ident(pokemon);
            b.add(Msg::Miss { source, target: None });
            if selfdestruct {
                b.faint(pokemon, Some(pokemon), effect);
            }
            return MoveResult::False;
        }

        // Then the target's immunity to the move.
        if !b.run_immunity(target, Some(mv), true) {
            if selfdestruct {
                b.faint(pokemon, Some(pokemon), effect);
            }
            return MoveResult::False;
        }
        let hit_result =
            b.single_event(EventId::TryImmunity, effect, StateRef::Scratch, 0, Holder::Mon(target), Some(pokemon), effect, None);
        if hit_result == Relay::FAIL {
            let mon = b.ident(target);
            b.add(Msg::Immune { mon, kw: Kw::NONE });
            return MoveResult::False;
        }

        // Accuracy: `None` is `true`.
        let mut accuracy: Option<i32> = b.mv(mv).accuracy_x256.map(i32::from);

        // A sleep move against a Pokemon that must recharge always hits.
        let slp = conditions::SLP;
        if def.status == slp && b.has_vol(target, conditions::MUSTRECHARGE) {
            accuracy = None;
        }

        // OHKO moves only hit a target no faster than the user.
        if b.mv(mv).ohko && b.get_stat(target, Stat::Spe, false, false) > b.get_stat(pokemon, Stat::Spe, false, false) {
            let mon = b.ident(target);
            b.add(Msg::Immune { mon, kw: Kw::flags(KwFlags::OHKO) });
            return MoveResult::False;
        }

        // Gen 1's accuracy is out of 255, with the stat table for accuracy
        // and evasion.
        if let Some(acc_x256) = accuracy {
            let mut acc = acc_x256 * 255 / (100 * 256);
            // The Rage and Thrash / Petal Dance accuracy bug (`accuracy` in
            // data[1]).
            if let Some(st) = b.volatile_state(pokemon, conditions::LOCKEDMOVE) {
                acc = i32::from(st.data[1]);
            }
            if let Some(st) = b.volatile_state(pokemon, conditions::RAGE) {
                acc = i32::from(st.data[1]);
            }
            if !def.bools.has("ignoreAccuracy") {
                let stage = i32::from(b.mon(pokemon).boosts[5]);
                acc = (f64::from(acc) * (f64::from(BOOST_TABLE[(stage + 6) as usize]) / 100.0)) as i32;
            }
            if !def.bools.has("ignoreEvasion") {
                let stage = i32::from(b.mon(target).boosts[6]);
                acc = (f64::from(acc) * (f64::from(BOOST_TABLE[(-stage + 6) as usize]) / 100.0)) as i32;
            }
            // (Both products are non-negative: the casts are `Math.floor`.)
            acc = acc.clamp(1, 255);
            if let Some(st) = b.volatile_state_mut(pokemon, conditions::LOCKEDMOVE) {
                st.data[1] = acc as i16;
            }
            if let Some(st) = b.volatile_state_mut(pokemon, conditions::RAGE) {
                st.data[1] = acc as i16;
            }
            accuracy = Some(acc);
        }
        let relay = accuracy.map_or(Relay::TRUE, Relay::Int);
        accuracy = match b.run_event(EventId::Accuracy, Holder::Mon(target), Some(pokemon), effect, Some(relay)) {
            Relay::Bool(true) => None,
            Relay::Int(n) => Some(n),
            other => panic!("Accuracy answered {other:?}"),
        };
        // Moves that target the user skip the 1/256 miss.
        if b.move_targets_self(mv) {
            if let Some(a) = accuracy.as_mut() {
                *a += 1;
            }
        }
        // The 1/256 miss.
        if let Some(acc) = accuracy {
            if !b.prng.random_chance(acc.max(0) as u32, 256) {
                b.attr_last_move(KwFlags::MISS);
                let source = b.ident(pokemon);
                b.add(Msg::Miss { source, target: None });
                if acc == 255 {
                    b.hint(TextId::HintGen1Miss, false, None);
                }
                damage = MoveResult::False;
                b.last_damage = 0;
            }
        }

        // It hit: the damage.
        if damage != MoveResult::False {
            if let Some([lo, hi]) = def.multihit() {
                // A fixed count is a number in PS (no PRNG call).
                let hits = if lo == hi {
                    u32::from(lo)
                } else if lo == 2 && hi == 5 {
                    *b.prng.sample(&[2u32, 2, 2, 3, 3, 3, 4, 5])
                } else {
                    b.prng.random_range(u32::from(lo), u32::from(hi) + 1)
                };
                // Every hit of a gen 1 multi-hit move deals the first one's
                // damage.
                let mut i = 0;
                while i < hits && b.mon(target).hp != 0 && b.mon(pokemon).hp != 0 {
                    let am = b.mv_mut(mv);
                    am.hit = (i + 1) as u8;
                    am.last_hit = i + 1 == hits;
                    let move_damage = Self::move_hit(b, Some(target), pokemon, mv, MoveData::Move, false, false);
                    if move_damage == MoveResult::False {
                        break;
                    }
                    damage = MoveResult::Int(move_damage.or_zero());
                    if i == 0 {
                        b.mv_mut(mv).damage = Some(FixedDamage::Hp(damage.or_zero() as u16));
                    }
                    if b.mon(target).sub_fainted {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                b.mv_mut(mv).damage = None;
                if i == 0 {
                    return MoveResult::Int(1);
                }
                let mon = b.ident(target);
                b.add(Msg::HitCount { mon, count: i as u8 });
            } else {
                damage = Self::move_hit(b, Some(target), pokemon, mv, MoveData::Move, false, false);
            }
        }

        if b.mv(mv).category != MoveCategory::Status {
            b.got_attacked(target, id, damage, pokemon);
        }

        if selfdestruct {
            if !b.mon(target).sub_fainted {
                b.faint(pokemon, Some(pokemon), effect);
            } else if id == moves::EXPLOSION {
                b.hint(TextId::HintGen1ExplosionSub, false, None);
            } else {
                b.hint(TextId::HintGen1SelfDestructSub, false, None);
            }
        }

        // The move missed.
        if damage == MoveResult::False {
            // The partial trap lock goes (a plain `delete`).
            b.mon_mut(pokemon).volatiles.remove(EffectId::Condition(conditions::PARTIALTRAPPINGLOCK));
            return MoveResult::False;
        }

        if b.mv(mv).ohko {
            b.add(Msg::Ohko);
        }

        b.single_event(
            EventId::AfterMoveSecondary,
            effect,
            StateRef::Scratch,
            0,
            Holder::Mon(target),
            Some(pokemon),
            effect,
            None,
        );
        b.run_event(EventId::AfterMoveSecondary, Holder::Mon(target), Some(pokemon), effect, None);
        damage
    }

    /// One hit: damage and effects, gen 1's Substitute behaviour.
    // PS: data/mods/gen1/scripts.ts moveHit
    fn move_hit(
        b: &mut Battle,
        target: Option<MonRef>,
        pokemon: MonRef,
        mv: MoveRef,
        data: MoveData,
        is_secondary: bool,
        is_self: bool,
    ) -> MoveResult {
        let mut damage = MoveResult::Int(0);
        let mut target = target;
        if !is_secondary && !is_self {
            b.set_active_move(Some(mv), Some(pokemon), target);
        }
        let mut hit_result;
        let effect = b.mv(mv).effect();
        let id = b.mv(mv).id;
        let def = b.dex.move_def(id);
        // `move.ignoreImmunity` is never undefined: the dex sets it.

        // Whether the target had a Substitute before the hit.
        let target_had_sub = target.is_some_and(|t| b.has_vol(t, conditions::SUBSTITUTE));
        let mut target_has_sub: Option<bool> = None;

        if let Some(t) = target {
            hit_result = b.move_data_event(EventId::TryHit, data, mv, Holder::Mon(t), Some(pokemon), None);
            if !hit_result.truthy() {
                if hit_result == Relay::FAIL {
                    let mon = b.ident(t);
                    b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
                }
                return MoveResult::False;
            }

            // The hit events run for the hit itself, not the secondary or
            // self hits.
            if !is_self && !is_secondary {
                hit_result = b.run_event(EventId::TryHit, Holder::Mon(t), Some(pokemon), effect, None);
                if !hit_result.truthy() {
                    if hit_result == Relay::FAIL {
                        let mon = b.ident(t);
                        b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
                    }
                    // The special Substitute hit flag.
                    if hit_result != Relay::HIT_SUBSTITUTE {
                        return MoveResult::False;
                    }
                }
                if !b.run_event(EventId::TryFieldHit, Holder::Mon(t), Some(pokemon), effect, None).truthy() {
                    return MoveResult::False;
                }
            } else if is_secondary && data_self(data, def).is_none() {
                // `runEvent('TrySecondaryHit', target, pokemon, moveData)`:
                // the move stands for its secondary as the source effect.
                hit_result = b.run_event(EventId::TrySecondaryHit, Holder::Mon(t), Some(pokemon), effect, None);
            }

            if hit_result == Relay::HIT_SUBSTITUTE {
                target_has_sub = Some(b.has_vol(t, conditions::SUBSTITUTE));
                target = None;
            } else if !hit_result.truthy() {
                if hit_result == Relay::FAIL {
                    let mon = b.ident(t);
                    b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
                }
                return MoveResult::False;
            }
        }

        if let Some(t) = target {
            let mut did_something = false;

            damage = Self::get_damage(b, pokemon, t, mv, data, false);

            // `getDamage` returns a number (that much damage, 0 included),
            // `false` (it failed), `null` (it ended, already explained) or
            // `undefined` (no damage, the move goes on).
            if let MoveResult::Int(n) = damage {
                if n > i32::from(b.mon(t).hp) {
                    damage = MoveResult::Int(i32::from(b.mon(t).hp));
                }
            }
            if let (MoveResult::Int(n), false) = (damage, b.mon(t).fainted) {
                damage = b.damage(n, Some(t), Some(pokemon), effect);
                if let MoveResult::Int(d) = damage {
                    if d != 0 {
                        b.apply_recoil_damage(d, mv, pokemon);
                    }
                }
                if !damage.is_number() {
                    return MoveResult::False;
                }
                did_something = true;
            }
            // `damage === false && typeof hitResult === 'undefined'`: the
            // relay is never undefined.
            if matches!(damage, MoveResult::False | MoveResult::Null) {
                return MoveResult::False;
            }
            if let Some(boosts) = data_boosts(data, def) {
                if b.mon(t).hp != 0 {
                    let will_boost = b.boost(boosts.as_array(), t, Some(pokemon), effect);
                    if !will_boost.truthy() {
                        let mon = b.ident(t);
                        b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
                        return MoveResult::False;
                    }
                    did_something = true;
                    // The Pokemon whose turn it is not: a paralysis or burn
                    // penalty applies again.
                    if let Some(foe) = b.active_at(1 - pokemon.side, 0) {
                        let status = b.mon(foe).status;
                        if status == conditions::PAR {
                            b.modify_stat(foe, Stat::Spe as usize, 0.25);
                        }
                        if status == conditions::BRN {
                            b.modify_stat(foe, Stat::Atk as usize, 0.5);
                        }
                    }
                }
            }
            if let (MoveData::Move, Some(heal)) = (data, def.heal()) {
                if !b.mon(t).fainted {
                    let amount = i32::from(b.mon(t).maxhp) * i32::from(heal.num) / i32::from(heal.den);
                    let d = b.heal_mon(t, amount);
                    if d == 0 {
                        let mon = b.ident(t);
                        b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
                        return MoveResult::False;
                    }
                    let mon = b.ident(t);
                    let health = b.health(t);
                    b.add(Msg::Heal { mon, health, kw: Kw::NONE });
                    did_something = true;
                }
            }
            let status = data_status(data, def);
            if !status.is_none() {
                b.require_ported(EffectId::Condition(status));
                // A target that must recharge falls asleep whatever its
                // status; sleep clears the recharge (unless Sleep Clause
                // stops it). The paralysis and burn drops stay.
                if status == conditions::SLP && b.has_vol(t, conditions::MUSTRECHARGE) {
                    if b.set_status(t, status, Some(pokemon), effect, false).truthy() {
                        let mr = conditions::MUSTRECHARGE;
                        b.remove_volatile(t, mr);
                        b.hint(TextId::HintGen1HyperBeamSleep, false, None);
                    }
                } else if b.mon(t).status.is_none() {
                    if b.set_status(t, status, Some(pokemon), effect, false).truthy() {
                        // The burn and paralysis drops go on the stats here.
                        if status == conditions::BRN {
                            b.modify_stat(t, Stat::Atk as usize, 0.5);
                        }
                        if status == conditions::PAR {
                            b.modify_stat(t, Stat::Spe as usize, 0.25);
                        }
                    }
                } else if !is_secondary {
                    let mon = b.ident(t);
                    let what = if b.mon(t).status == status {
                        ps_protocol::Status::parse(b.dex.condition_def(status).key.as_str()).map(|s| Arg::new(ArgValue::Status(s)))
                    } else {
                        None
                    };
                    b.add(Msg::Fail { mon, what, what2: None, kw: Kw::NONE });
                }
                did_something = true;
            }
            // `forceStatus`: no gen 1 move has one.
            let volatile = data_volatile(data, def);
            if !volatile.is_none() {
                b.require_ported(EffectId::Condition(volatile));
                if b.add_volatile(t, volatile, Some(pokemon), effect).truthy() {
                    did_something = true;
                }
            }
            let side_condition = data_side_condition(data, def);
            if !side_condition.is_none() {
                b.require_ported(EffectId::Condition(side_condition));
                if b.add_side_condition(t.side, side_condition, Some(pokemon), effect).truthy() {
                    did_something = true;
                }
            }
            let pseudo_weather = data_pseudo_weather(data, def);
            if !pseudo_weather.is_none() {
                b.require_ported(EffectId::Condition(pseudo_weather));
                if b.add_pseudo_weather(EffectId::Condition(pseudo_weather), Some(pokemon), effect).truthy() {
                    did_something = true;
                }
            }
            // The hit events.
            hit_result = b.move_data_event(EventId::Hit, data, mv, Holder::Mon(t), Some(pokemon), None);
            if !is_self && !is_secondary {
                b.run_event(EventId::Hit, Holder::Mon(t), Some(pokemon), effect, None);
            }
            if !hit_result.truthy() && !did_something {
                if hit_result == Relay::FAIL {
                    let mon = b.ident(t);
                    b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
                }
                return MoveResult::False;
            }
        }
        let target_has_sub = match target_has_sub {
            Some(s) => s,
            None => target.is_some_and(|t| b.has_vol(t, conditions::SUBSTITUTE)),
        };

        // The self effects.
        let do_self = (target_had_sub && target_has_sub) || !target_had_sub;
        if let Some(self_effect) = data_self(data, def) {
            let lockedmove = conditions::LOCKEDMOVE;
            if self_effect.volatile_status != lockedmove
                && (do_self || self_effect.volatile_status == conditions::PARTIALTRAPPINGLOCK)
            {
                Self::move_hit(b, Some(pokemon), pokemon, mv, MoveData::SelfEffect(self_effect), is_secondary, true);
            }
        }

        // The secondaries.
        if let (MoveData::Move, Some(t)) = (data, target) {
            if b.mon(t).hp > 0 {
                let multihit = def.multihit().is_some();
                for secondary in def.secondaries() {
                    // A multi-hit move rolls for its secondaries once.
                    if multihit && !b.mv(mv).last_hit {
                        continue;
                    }
                    // A paralysis, burn or freeze chance is not rolled
                    // against a target of the move's type.
                    if [conditions::PAR, conditions::BRN, conditions::FRZ].contains(&secondary.status) {
                        let move_type = b.mv(mv).type_;
                        if b.has_type(t, move_type) {
                            continue;
                        }
                    }
                    match secondary.chance {
                        None => {
                            Self::move_hit(b, Some(t), pokemon, mv, MoveData::Secondary(secondary), true, is_self);
                        }
                        Some(chance) => {
                            let mut secondary_chance = (u32::from(chance) * 256).div_ceil(100);
                            // Confusion's numerator is one less (10% is
                            // 25/256, not 26/256).
                            if secondary.volatile_status == conditions::CONFUSION {
                                secondary_chance -= 1;
                            }
                            if b.prng.random_chance(secondary_chance, 256) {
                                Self::move_hit(b, Some(t), pokemon, mv, MoveData::Secondary(secondary), true, is_self);
                            }
                        }
                    }
                }
            }
        }
        if let Some(ss) = def.self_switch() {
            if b.mon(pokemon).hp != 0 {
                // `copyvolatile` is the flag's id; no gen 1 move has it.
                b.mon_mut(pokemon).switch_flag = match ss {
                    ps_dex::SelfSwitch::True => SwitchFlag::Yes,
                    ps_dex::SelfSwitch::CopyVolatile => SwitchFlag::Move(id),
                };
            }
        }
        damage
    }

    /// Gen 1's damage formula, step by step as the cartridge does it.
    // PS: data/mods/gen1/scripts.ts getDamage
    fn get_damage(b: &mut Battle, source: MonRef, target: MonRef, mv: MoveRef, data: MoveData, suppress: bool) -> MoveResult {
        let is_move = matches!(data, MoveData::Move);
        let effect = b.mv(mv).effect();
        let id = b.mv(mv).id;
        let def = b.dex.move_def(id);

        // Immunity (a secondary or self effect has no type: never immune).
        if is_move && !b.run_immunity(target, Some(mv), true) {
            return MoveResult::False;
        }
        // `{damage: N}` has no type, OHKO or callback: `move.damage`.
        if let MoveData::Damage(n) = data {
            return MoveResult::Int(n);
        }
        if is_move {
            if b.mv(mv).ohko {
                return MoveResult::Int(65535);
            }
            if let Some(r) = b.move_callback(mv, EventId::DamageCallback, source, Some(target)) {
                return MoveResult::from_relay(r);
            }
            match b.mv(mv).damage {
                Some(FixedDamage::Level) => return MoveResult::Int(i32::from(b.mon(source).level)),
                Some(FixedDamage::Hp(n)) => return MoveResult::Int(i32::from(n)),
                None => {}
            }
            // The first hit of a Normal partial trapping move hits a Ghost
            // for 0.
            if def.self_effect().is_some_and(|s| s.volatile_status == conditions::PARTIALTRAPPINGLOCK)
                && b.mv(mv).type_ == Type::Normal
                && b.has_type(target, Type::Ghost)
            {
                return MoveResult::Int(0);
            }
        }
        // In the middle of a partial trap sequence, the previous damage
        // (`locked` in data[0], `damage` in data[1]).
        if let Some(st) = b.volatile_state(source, conditions::PARTIALTRAPPINGLOCK) {
            if st.data_mon(0) == Some(target) {
                return MoveResult::Int(i32::from(st.data[1]));
            }
        }
        if !is_move {
            // No `basePower`: `return undefined`.
            return MoveResult::Undefined;
        }

        let category = b.mv(mv).category;
        let move_type = b.mv(mv).type_;

        let mut base_power = MoveResult::Int(i32::from(b.mv(mv).base_power));
        if let Some(r) = b.move_callback(mv, EventId::BasePowerCallback, source, Some(target)) {
            base_power = MoveResult::from_relay(r);
        }
        let mut base_power = match base_power {
            MoveResult::Int(0) => return MoveResult::Undefined,
            MoveResult::Int(n) => n.max(1),
            other => return other,
        };

        // Crits: gen 1's chance comes from base Speed.
        let mut is_crit = def.bools.has("willCrit");
        if !is_crit {
            let base_spe = i32::from(b.dex.species_def(b.mon(source).base_species).base_stats[5]);
            let mut crit_chance = base_spe / 2;
            if b.has_vol(source, conditions::FOCUSENERGY) {
                crit_chance /= 2;
            } else {
                crit_chance = (crit_chance * 2).clamp(1, 255);
            }
            match def.crit_ratio {
                1 => crit_chance /= 2,
                2 => crit_chance = (crit_chance * 4).clamp(1, 255),
                _ => {}
            }
            // There is always a 1/256 chance of no crit.
            if crit_chance > 0 {
                is_crit = b.prng.random_chance(crit_chance as u32, 256);
            }
        }
        if is_crit {
            b.mv_mut(mv).crit = true;
        }

        // After the crit roll.
        match b.run_event(EventId::BasePower, Holder::Mon(source), Some(target), effect, Some(Relay::Int(base_power))) {
            Relay::Int(n) => base_power = n,
            r if !r.truthy() => base_power = 0,
            other => panic!("BasePower answered {other:?}"),
        }
        // No gen 1 move has `basePowerModifier`.
        if base_power == 0 {
            return MoveResult::Int(0);
        }
        base_power = base_power.max(1);

        // The stats. No gen 1 move overrides the offensive or defensive
        // Pokemon or stat.
        let mut level = i32::from(b.mon(source).level);
        let (attacker, defender) = (source, target);
        let is_physical = category == MoveCategory::Physical;
        let atk_type = if is_physical { Stat::Atk } else { Stat::Spa };
        let def_type = if is_physical { Stat::Def } else { Stat::Spd };

        let mut attack = b.get_stat(attacker, atk_type, false, false);
        let mut defense = b.get_stat(defender, def_type, false, false);

        // Gen 1's screens apply here.
        if (def_type == Stat::Def && b.has_vol(defender, conditions::REFLECT))
            || (def_type == Stat::Spd && b.has_vol(defender, conditions::LIGHTSCREEN))
        {
            b.debug(ps_protocol::Text::new(TextId::ScreenDoublingDef));
            defense *= 2;
        }

        // A crit ignores the stat changes (boosts and screens) and doubles
        // the level.
        if is_crit {
            let am = b.mv_mut(mv);
            am.ignore_offensive = true;
            am.ignore_defensive = true;
            level *= 2;
            if !suppress {
                let mon = b.ident(target);
                b.add(Msg::Crit(mon));
            }
        }
        level = b.trunc.int(i64::from(level), 8) as i32;

        if b.mv(mv).ignore_offensive {
            b.debug(ps_protocol::Text::new(TextId::NegatingAtkBoost));
            attack = b.get_stat(attacker, atk_type, true, false);
        }
        if b.mv(mv).ignore_defensive {
            b.debug(ps_protocol::Text::new(TextId::NegatingDefBoost));
            // No screens.
            defense = b.get_stat(target, def_type, true, false);
        }

        // At 256 or more, both are divided by 4, and roll over past 1023.
        if attack >= 256 || defense >= 256 {
            if attack >= 1024 || defense >= 1024 {
                b.hint(TextId::HintGen1StatRollover, false, None);
            }
            attack = (b.trunc.int(i64::from(attack / 4), 8) as i32).max(1);
            // The cartridge does not check defense; the sim avoids / 0.
            defense = b.trunc.int(i64::from(defense / 4), 8) as i32;
            if defense == 0 {
                b.hint(TextId::HintDefenseDivZero, false, None);
                defense = 1;
            }
        }

        // Self-destruct moves halve defense here.
        if def.selfdestruct().is_some() && def_type == Stat::Def {
            defense = (defense / 2).max(1);
        }

        // The formula, step by step. JS doubles hold every intermediate
        // exactly.
        let mut damage = i64::from(level) * 2;
        damage /= 5;
        damage += 2;
        damage *= i64::from(base_power);
        damage *= i64::from(attack);
        damage /= i64::from(defense);
        damage = (damage / 50).clamp(0, 997);
        damage += 2;

        // STAB (`???` never gets it).
        if move_type != Type::None && b.has_type(source, move_type) {
            damage += damage / 2;
        }

        // Type effectiveness, against each of the target's types in turn.
        let types = b.mon(target).types;
        for &target_type in types.as_slice() {
            let type_mod = b.type_effectiveness(move_type, target_type);
            // `runEvent('Effectiveness', this.battle, targetType, move, typeMod)`.
            let type_mod = match b.run_event(EventId::Effectiveness, Holder::Battle, None, effect, Some(Relay::Int(type_mod))) {
                Relay::Int(n) => n,
                other => panic!("Effectiveness answered {other:?}"),
            };
            if type_mod > 0 {
                damage *= 20;
                damage /= 10;
            }
            if type_mod < 0 {
                damage *= 5;
                damage /= 10;
            }
        }
        let total_type_mod = b.run_effectiveness(target, mv);
        if total_type_mod > 0 && !suppress {
            let mon = b.ident(target);
            b.add(Msg::SuperEffective(mon));
        }
        if total_type_mod < 0 && !suppress {
            let mon = b.ident(target);
            b.add(Msg::Resisted(mon));
        }

        // 0 damage misses: 2 or 3 before STAB and type, against a double
        // resist.
        if damage == 0 {
            return MoveResult::Int(0);
        }

        // The random factor, above 1.
        if damage > 1 {
            damage *= i64::from(b.prng.random_range(217, 256));
            damage /= 255;
        }
        MoveResult::Int(damage as i32)
    }

    /// PP from `side.lastSelectedMoveSlot` (not `move`'s slot), with gen
    /// 1's underflow to 63. `amount` 0 is PS's `null` (1).
    // PS: data/mods/gen1/scripts.ts deductPP
    fn deduct_pp(b: &mut Battle, pokemon: MonRef, _mv: MoveId, amount: i32) -> i32 {
        let side = pokemon.side as usize;
        let slot = b.sides[side].last_selected_move_slot as usize;
        if b.get_move_slot(pokemon, slot as u8).is_none() {
            return 0;
        }
        let amount = if amount == 0 { 1 } else { amount };
        b.mon_mut(pokemon).move_slots[slot].used = true;
        let pp = i32::from(b.mon(pokemon).move_slots[slot].pp) - amount;
        if pp < 0 {
            let id = b.sides[side].id();
            b.hint(TextId::HintGen1PpUnderflow, false, Some(id));
        }
        let pp = b.trunc.int(i64::from(pp), 6) as i8;
        let mon = b.mon_mut(pokemon);
        mon.move_slots[slot].pp = pp;
        if mon.move_slots[slot].virtual_ && !mon.transformed {
            // Mimic's slot (or Metronome / Mirror Move calling Mimic).
            mon.base_move_slots[slot].pp = pp;
        }
        if !mon.move_slots[slot].virtual_ && !mon.transformed {
            // PS's `moveSlots` is a shallow copy of `baseMoveSlots`: the
            // slot is the same object until Transform or Mimic replaces it.
            mon.base_move_slots[slot].pp = pp;
            mon.base_move_slots[slot].used = true;
        }
        amount
    }

    /// Gen 1's boosts change the modified stats; a stat at 999 (or 1)
    /// bounces back.
    // PS: data/mods/gen1/scripts.ts boost
    /// (Gen 1's override takes no `isSecondary` / `isSelf`.)
    fn boost(
        b: &mut Battle,
        boosts: [i8; 7],
        target: MonRef,
        source: Option<MonRef>,
        effect: EffectId,
        _is_secondary: bool,
        _is_self: bool,
    ) -> MoveResult {
        let source = source.or(b.event.source);
        let effect = if effect.is_none() { b.frame.effect } else { effect };
        if b.mon(target).hp == 0 {
            return MoveResult::Int(0);
        }
        let mut success = MoveResult::Null;
        // `boost = runEvent('TryBoost', target, source, effect, {...boost})`:
        // a handler may drop entries (Mist drops the drops); a dropped
        // entry is 0, which the loop skips as PS skips a missing key.
        let mut boosts = match b.run_event(EventId::TryBoost, Holder::Mon(target), source, effect, Some(Relay::Boosts(boosts))) {
            Relay::Boosts(t) => t,
            other => panic!("TryBoost answered {other:?}"),
        };
        let kw = match effect {
            EffectId::None | EffectId::Move(_) => Kw::NONE,
            // `'[from] ' + effect.fullname`: a move's own condition (Rage)
            // has the move's name.
            EffectId::Condition(c) if b.dex.condition_def(c).key.as_str() == "rage" => {
                Kw::from(Arg::new(ArgValue::Move(MoveNum(moves::RAGE.0))))
            }
            other => panic!("unported: [from] {other:?} on a boost"),
        };
        for i in 0..7 {
            if boosts[i] == 0 {
                continue;
            }
            let mut current = [0i8; 7];
            current[i] = boosts[i];
            let result = b.boost_by_gen1(target, current);
            let stat = ps_protocol::Stat::ALL[i];
            let mon = b.ident(target);
            if result.truthy() {
                success = MoveResult::True;
                let amount = boosts[i].unsigned_abs();
                b.add(if boosts[i] < 0 {
                    Msg::Unboost { mon, stat, amount, kw }
                } else {
                    Msg::Boost { mon, stat, amount, kw }
                });
                b.run_event(EventId::AfterEachBoost, Holder::Mon(target), source, effect, Some(Relay::Boosts(current)));
            }
            // A boost at 999 or a drop at 1: no success, so no paralysis
            // or burn penalty after.
            if result == MoveResult::Int(0) {
                let amount = boosts[i].unsigned_abs();
                if boosts[i] < 0 {
                    b.add(Msg::Unboost { mon, stat, amount, kw });
                    b.add(Msg::Boost { mon, stat, amount: 1, kw: Kw::NONE });
                    b.hint(TextId::HintGen1DropAt1, true, None);
                } else {
                    b.add(Msg::Boost { mon, stat, amount, kw });
                    b.add(Msg::Unboost { mon, stat, amount: 1, kw: Kw::NONE });
                    b.hint(TextId::HintGen1BoostAt999, true, None);
                }
                b.run_event(EventId::AfterEachBoost, Holder::Mon(target), source, effect, Some(Relay::Boosts(current)));
            }
            // `boost[i] = -boost[i]` for a shown drop: `AfterBoost` sees
            // the amounts as printed.
            if (result.truthy() || result == MoveResult::Int(0)) && boosts[i] < 0 {
                boosts[i] = -boosts[i];
            }
        }
        b.run_event(EventId::AfterBoost, Holder::Mon(target), source, effect, Some(Relay::Boosts(boosts)));
        success
    }

    /// Gen 1's `clearBoosts`: the modified stats go back to the stored
    /// ones.
    // PS: data/mods/gen1/scripts.ts clearBoosts
    fn clear_boosts(b: &mut Battle, pokemon: MonRef) {
        let mon = b.mon_mut(pokemon);
        mon.boosts = [0; 7];
        mon.modified_stats = mon.stored_stats;
    }
}
