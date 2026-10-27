//! `foresight`: Foresight's and Odor Sleuth's volatile. Normal and Fighting
//! moves hit the Ghost holder, and its raised evasion is ignored. No
//! effect state of its own.
//!
//! PS: data/moves.ts foresight.condition

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;
use ps_dex::Type;

/// `onStart(pokemon)`: `-start|MON|Foresight`.
// PS: data/moves.ts foresight.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    let effect = Arg::new(ArgValue::Move(MoveNum(moves::FORESIGHT.0)));
    b.add(Msg::StartEffect { mon, effect, a: None, b: None, kw: Kw::NONE });
    None
}

/// `onNegateImmunity(pokemon, type)`: a Ghost loses its Normal and
/// Fighting immunity. The type comes as the relay (see
/// `Battle::run_type_immunity`).
// PS: data/moves.ts foresight.condition.onNegateImmunity
fn on_negate_immunity(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let Some(Relay::Types(t)) = a.relay else { panic!("NegateImmunity without its type: {:?}", a.relay) };
    if b.has_type(pokemon, Type::Ghost) && matches!(t.0[0], Type::Normal | Type::Fighting) {
        return Some(Relay::FAIL);
    }
    None
}

/// `onModifyBoost(boosts)`: a positive evasion stage counts as 0.
// PS: data/moves.ts foresight.condition.onModifyBoost
fn on_modify_boost(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let Some(Relay::Boosts(mut boosts)) = a.relay else { panic!("ModifyBoost without boosts: {:?}", a.relay) };
    if boosts[6] > 0 {
        boosts[6] = 0;
        return Some(Relay::Boosts(boosts));
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::NegateImmunity, on_negate_immunity),
    Handler::on(EventId::ModifyBoost, on_modify_boost),
]);
