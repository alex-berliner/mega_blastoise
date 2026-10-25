//! Psych Up: the user copies the target's stat stages. No effect state.
//!
//! Gen 3 inherits gen 5's `onHit` (every stage, then `-copyboost`).
//!
//! PS: data/mods/gen5/moves.ts psychup.onHit

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `onHit(target, source)`.
// PS: data/mods/gen5/moves.ts psychup.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon().expect("Psych Up hits a Pokemon");
    let source = a.source.expect("Psych Up has a user");
    let boosts = b.mon(target).boosts;
    b.mon_mut(source).boosts = boosts;
    let (src, tgt) = (b.ident(source), b.ident(target));
    let from = Arg::new(ArgValue::MoveEffect(MoveNum(moves::PSYCHUP.0)));
    b.add(Msg::CopyBoost { source: src, target: tgt, kw: Kw::from(from) });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
