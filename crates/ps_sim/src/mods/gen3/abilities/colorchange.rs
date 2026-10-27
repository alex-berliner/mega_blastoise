//! Color Change: after a damaging hit, the holder becomes the type of the
//! move that hit it (`-start|MON|typechange|TYPE|[from] ability: Color
//! Change`), unless it already has that type, the move is typeless, or the
//! holder fainted.
//!
//! No ability state.
//!
//! PS: data/mods/gen4/abilities.ts colorchange

use ps_dex::{MoveCategory, Type};
use ps_protocol::{Arg, Kw, Msg, TypeNum, Word};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::{event_move, this_ability_arg};

/// `onDamagingHit(damage, target, source, move)`.
// PS: data/mods/gen4/abilities.ts colorchange.onDamagingHit
fn on_damaging_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let damage = a.relay.and_then(Relay::as_int).unwrap_or(0);
    if damage == 0 || b.mon(target).hp == 0 {
        return None;
    }
    let mv = event_move(b, a)?;
    let ty = mv.type_;
    if b.mon(target).is_active && mv.category != MoveCategory::Status && ty != Type::None && !b.has_type(target, ty) {
        if !b.set_type(target, ty, false) {
            return Some(Relay::FAIL);
        }
        let mon = b.ident(target);
        let kw = Kw::from(this_ability_arg(b));
        b.add(Msg::StartEffect {
            mon,
            effect: Arg::word(Word::TypeChange),
            a: Some(Arg::ty(TypeNum(ty.index()))),
            b: None,
            kw,
        });
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::DamagingHit, on_damaging_hit)]);
