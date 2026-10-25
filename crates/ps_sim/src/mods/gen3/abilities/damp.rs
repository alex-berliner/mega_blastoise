//! Damp: while the holder is out, nobody can use Explosion or Self-Destruct
//! (`TryMove` of any Pokemon): `cant|HOLDER|ability: Damp|MOVE|[of] USER`,
//! the move line marked `[still]`. Its Aftermath guard (`onAnyDamage`) names
//! a gen 4 ability, so in gen 3 it never refuses.
//!
//! No ability state.
//!
//! PS: data/abilities.ts damp

use ps_protocol::{Arg, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::Battle;

/// `onAnyTryMove(target, source, effect)`.
// PS: data/abilities.ts damp.onAnyTryMove
fn on_any_try_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let EffectId::Move(mv) = a.effect else { return None };
    // `mindblown` (gen 7) and `mistyexplosion` (gen 8) are not in gen 3.
    let named = |b: &Battle, id: &str| b.dex.move_id(id) == Some(mv);
    if !["explosion", "mindblown", "mistyexplosion", "selfdestruct"].iter().any(|id| named(b, id)) {
        return None;
    }
    let target = a.target.mon()?;
    b.attr_last_move_still();
    let holder = super::holder(b)?;
    let mon = b.ident(holder);
    let reason = super::this_ability_arg(b);
    let kw = Kw { of: Some(b.ident(target)), ..Kw::NONE };
    b.add(Msg::Cant { mon, reason, mv: Some(Arg::mv(MoveNum(mv.0))), kw });
    Some(Relay::FAIL)
}

/// `onAnyDamage(damage, target, source, effect)`: `false` for Aftermath.
// PS: data/abilities.ts damp.onAnyDamage
fn on_any_damage(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let name = match a.effect {
        EffectId::Ability(id) => b.dex.ability_def(id).name.as_str(),
        _ => "",
    };
    (name == "Aftermath").then_some(Relay::FAIL)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryMove, on_any_try_move).prefix(Prefix::Any),
    Handler::on(EventId::Damage, on_any_damage).prefix(Prefix::Any),
]);
