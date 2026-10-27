//! Rapid Spin: after it hits, the user is freed from Leech Seed, Spikes
//! and a partial trap. Gen 4 gives the move a `self.onHit` that does this
//! and keeps base's `onAfterHit` and `onAfterSubDamage`, so a hit runs the
//! `self` effect first and `onAfterHit` then finds nothing left to clear.
//! No effect state.
//!
//! PS: data/mods/gen4/moves.ts rapidspin.self.onHit, data/moves.ts
//! rapidspin (onAfterHit, onAfterSubDamage)

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, MonRef};
use ps_dex::gen3::{conditions, moves};

/// The body the three handlers share. `side_conditions` is the handler's
/// list (gen 4's lacks G-Max Steelsurge); of them only Spikes exists in
/// gen 3, and `removeSideCondition` of a condition the side cannot have is
/// false. `need_hp` is `onAfterSubDamage`'s `pokemon.hp &&` on each step.
/// `!move.hasSheerForce` always holds (Sheer Force is gen 5).
fn spin(b: &mut Battle, pokemon: MonRef, side_conditions: &[&str], need_hp: bool) {
    let from = || Arg::new(ArgValue::MoveEffect(MoveNum(moves::RAPIDSPIN.0)));
    let alive = |b: &Battle| !need_hp || b.mon(pokemon).hp != 0;
    if alive(b) && b.remove_volatile(pokemon, conditions::LEECHSEED) {
        let mon = b.ident(pokemon);
        let effect = Arg::new(ArgValue::Move(MoveNum(moves::LEECHSEED.0)));
        b.add(Msg::EndEffect { mon, effect, kw: Kw::from_of(from(), mon) });
    }
    let side = pokemon.side;
    for key in side_conditions {
        let Some(condition) = b.dex.condition_id(key) else { continue };
        if alive(b) && b.remove_side_condition(side, condition) {
            // `this.dex.conditions.get(condition).name`: a move's own
            // condition is named after the move.
            let m = b.dex.move_id(key).expect("a side condition named after its move");
            let mon = b.ident(pokemon);
            let effect = Arg::new(ArgValue::Move(MoveNum(m.0)));
            let side_id = b.sides[side as usize].id();
            b.add(Msg::SideEnd { side: side_id, effect, kw: Kw::from_of(from(), mon) });
        }
    }
    let trapped = EffectId::Condition(conditions::PARTIALLYTRAPPED);
    if alive(b) && b.mon(pokemon).volatiles.contains(trapped) {
        b.remove_volatile(pokemon, conditions::PARTIALLYTRAPPED);
    }
}

/// `self.onHit(pokemon)`.
// PS: data/mods/gen4/moves.ts rapidspin.self.onHit
fn self_on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    spin(b, pokemon, &["spikes", "toxicspikes", "stealthrock", "stickyweb"], false);
    None
}

/// `onAfterHit(target, pokemon, move)`.
// PS: data/moves.ts rapidspin.onAfterHit
fn on_after_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.source?;
    spin(b, pokemon, &["spikes", "toxicspikes", "stealthrock", "stickyweb", "gmaxsteelsurge"], false);
    None
}

/// `onAfterSubDamage(damage, target, pokemon, move)`. Gen 4's Substitute
/// runs `AfterSubDamage` without `onEffect`, so the move's own handler
/// never runs in gen 3; it is here because the dex has it.
// PS: data/moves.ts rapidspin.onAfterSubDamage
fn on_after_sub_damage(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.source?;
    spin(b, pokemon, &["spikes", "toxicspikes", "stealthrock", "stickyweb", "gmaxsteelsurge"], true);
    None
}

static SELF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, self_on_hit)]);

pub static DEF: EffectDef = EffectDef::with_self(
    &[Handler::on(EventId::AfterHit, on_after_hit), Handler::on(EventId::AfterSubDamage, on_after_sub_damage)],
    &SELF,
);
