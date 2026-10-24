//! `leechseed`: Leech Seed's volatile. In gen 1 it drains after each of
//! the holder's moves (`AfterMoveSelf`), not at the end of the turn, and the
//! Toxic counter multiplies the drain. The seeder is whoever now stands in
//! the seeder's slot.
//!
//! Effect state: `sourceSlot` is `source_slot`, which `addVolatile` sets.
//! The Toxic counter is `residualdmg`'s `counter`.
//!
//! PS: data/moves.ts leechseed.condition.onStart (lines 10216-10218);
//! data/mods/gen1/moves.ts leechseed.condition.onAfterMoveSelf (lines
//! 441-462)

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg, Text, TextId};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::{conditions, moves};

/// `onStart(target)`: `-start|MON|move: Leech Seed`.
// PS: data/moves.ts leechseed.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::LEECHSEED.0)));
    b.add(Msg::StartEffect { mon, effect, a: None, b: None, kw: Kw::NONE });
    None
}

/// `onAfterMoveSelf(pokemon)`: the holder loses `baseMaxhp / 16` (times the
/// Toxic counter) and the seeder heals that much, however little the holder
/// had left.
// PS: data/mods/gen1/moves.ts leechseed.condition.onAfterMoveSelf
fn on_after_move_self(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    // `this.getAtSlot(pokemon.volatiles['leechseed'].sourceSlot)`.
    let slot = b.effect_state().source_slot.expect("leechseed without a sourceSlot");
    let leecher = b.active_at(slot.side, slot.position);
    let Some(leecher) = leecher.filter(|&l| !b.mon(l).fainted && b.mon(l).hp > 0) else {
        b.debug(Text::new(TextId::NothingToLeechInto));
        return None;
    };
    let mut toxic_counter = 1;
    let has_residualdmg = b.has_vol(pokemon, conditions::RESIDUALDMG);
    if has_residualdmg {
        // `residualdmg.counter++`.
        super::bump_residualdmg_counter(b, pokemon);
        toxic_counter = super::residualdmg_counter(b, pokemon);
    }
    let to_leech = (i32::from(b.mon(pokemon).base_maxhp) / 16).max(1) * toxic_counter;
    // `this.damage(toLeech, pokemon, leecher)`: the effect is this volatile.
    let damage = b.damage(to_leech, Some(pokemon), Some(leecher), EffectId::None);
    if has_residualdmg {
        b.hint(TextId::HintGen1LeechSeedToxic, true, None);
    }
    if !damage.truthy() || to_leech > damage.or_zero() {
        b.hint(TextId::HintGen1LeechSeedRecovery, true, None);
    }
    b.heal(to_leech, Some(leecher), Some(pokemon), EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::AfterMoveSelf, on_after_move_self).priority(1),
]);
