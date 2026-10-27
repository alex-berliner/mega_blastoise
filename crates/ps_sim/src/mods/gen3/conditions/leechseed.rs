//! `leechseed`: Leech Seed's volatile. At each residual the holder loses
//! an eighth of its max HP to whoever stands in the seeder's slot.
//!
//! Effect state: `sourceSlot` is `source_slot`, which `addVolatile` sets.
//!
//! PS: data/moves.ts leechseed.condition

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg, Text, TextId};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{conditions, moves};

/// `onStart(target)`: `-start|MON|move: Leech Seed`.
// PS: data/moves.ts leechseed.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::LEECHSEED.0)));
    b.add(Msg::StartEffect { mon, effect, a: None, b: None, kw: Kw::NONE });
    None
}

/// `onResidual(pokemon)`: `damage(baseMaxhp / 8, pokemon, target)` for the
/// Pokemon at the seeder's slot, which heals what was dealt.
// PS: data/moves.ts leechseed.condition.onResidual
fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    // `this.getAtSlot(pokemon.volatiles['leechseed'].sourceSlot)`.
    let slot = b.volatile_state(pokemon, conditions::LEECHSEED).and_then(|s| s.source_slot).expect("leechseed sourceSlot");
    let target = b.active_at(slot.side, slot.position);
    let Some(target) = target.filter(|&t| !b.mon(t).fainted && b.mon(t).hp > 0) else {
        b.debug(Text::new(TextId::NothingToLeechInto));
        return None;
    };
    let d = super::fraction_of_base_maxhp(b, pokemon, 8);
    let damage = b.damage(d, Some(pokemon), Some(target), EffectId::None);
    if damage.truthy() {
        b.heal(damage.or_zero(), Some(target), Some(pokemon), EffectId::None);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Residual, on_residual).order(10).sub_order(5),
]);
