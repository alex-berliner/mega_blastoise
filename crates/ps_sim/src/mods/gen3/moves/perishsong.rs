//! Perish Song: every active Pokemon without the count gets it (3 turns
//! left), except one that is out of reach or whose `TryHit` stops it.
//!
//! PS: data/moves.ts perishsong.onHitField

use ps_protocol::{Arg, ArgValue, Kw, KwFlags, MoveNum, Msg, Word};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, Holder};
use ps_dex::gen3::{conditions, moves};

/// `onHitField(target, source, move)`.
// PS: data/moves.ts perishsong.onHitField
fn on_hit_field(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let source = a.source.expect("perishsong without a source");
    let effect = a.effect;
    let mut result = false;
    let mut message = false;
    let (actives, n) = b.all_active(false);
    for &pokemon in &actives[..n] {
        if b.run_event(EventId::Invulnerability, Holder::Mon(pokemon), Some(source), effect, None) == Relay::FAIL {
            let s = b.ident(source);
            let t = b.ident(pokemon);
            b.add(Msg::Miss { source: s, target: Some(t) });
            result = true;
        } else if b.run_event(EventId::TryHit, Holder::Mon(pokemon), Some(source), effect, None) == Relay::Null {
            result = true;
        } else if !b.has_vol(pokemon, conditions::PERISHSONG) {
            b.add_volatile(pokemon, conditions::PERISHSONG, None, EffectId::None);
            let mon = b.ident(pokemon);
            let perish3 = Arg::new(ArgValue::Word(Word::Perish3));
            b.add(Msg::StartEffect { mon, effect: perish3, a: None, b: None, kw: Kw::flags(KwFlags::SILENT) });
            result = true;
            message = true;
        }
    }
    if !result {
        return Some(Relay::FAIL);
    }
    if message {
        let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::PERISHSONG.0)));
        b.add(Msg::FieldActivate { effect, kw: Kw::NONE });
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::HitField, on_hit_field)]);
