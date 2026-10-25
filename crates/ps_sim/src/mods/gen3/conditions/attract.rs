//! `attract`: Attract's volatile (infatuation). The holder fails to move
//! half the time while its source stays in.
//!
//! Effect state: `source` is the Pokemon it is attracted to.
//!
//! PS: data/moves.ts attract.condition

use ps_protocol::{Arg, ArgValue, Kw, KwFlags, MoveNum, Msg, Text, TextArg, TextId};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, Holder};
use ps_dex::gen3::{abilities, conditions, moves};
use ps_dex::Gender;

fn attract_arg() -> Arg {
    Arg::new(ArgValue::Move(MoveNum(moves::ATTRACT.0)))
}

/// `onStart(pokemon, source, effect)`: opposite genders only, then the
/// `Attract` event; `-start|MON|Attract`, with `[from] ability: Cute
/// Charm|[of] SOURCE` from Cute Charm (Destiny Knot is gen 4).
// PS: data/moves.ts attract.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let source = a.source.expect("attract without a source");
    let (pg, sg) = (b.mon(pokemon).gender, b.mon(source).gender);
    if !(pg == Gender::Male && sg == Gender::Female) && !(pg == Gender::Female && sg == Gender::Male) {
        b.debug(Text::new(TextId::IncompatibleGender));
        return Some(Relay::FAIL);
    }
    if !b.run_event(EventId::Attract, Holder::Mon(pokemon), Some(source), EffectId::None, None).truthy() {
        b.debug(Text::new(TextId::AttractEventFailed));
        return Some(Relay::FAIL);
    }
    let kw = if a.effect == EffectId::Ability(abilities::CUTECHARM) {
        Kw::from_of(b.effect_fullname(a.effect), b.ident(source))
    } else {
        Kw::NONE
    };
    let mon = b.ident(pokemon);
    b.add(Msg::StartEffect { mon, effect: attract_arg(), a: None, b: None, kw });
    None
}

/// `onUpdate(pokemon)`: ends once its source is no longer active.
// PS: data/moves.ts attract.condition.onUpdate
fn on_update(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let Some(source) = b.effect_state().source else { return None };
    if !b.mon(source).is_active && b.has_vol(pokemon, conditions::ATTRACT) {
        let mon = b.ident(pokemon);
        b.debug(Text::with1(TextId::RemovingAttractVolatileOn, TextArg::mon(mon)));
        b.remove_volatile(pokemon, conditions::ATTRACT);
    }
    None
}

/// `onBeforeMove(pokemon, target, move)`: `-activate|MON|move:
/// Attract|[of] SOURCE`, then half the time `cant|MON|Attract`.
// PS: data/moves.ts attract.condition.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let source = b.effect_state().source.expect("attract without a source");
    let mon = b.ident(pokemon);
    let of = b.ident(source);
    let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::ATTRACT.0)));
    b.add(Msg::Activate { mon: Some(mon), effect, a: None, b: None, kw: Kw { of: Some(of), ..Kw::NONE } });
    if b.prng.random_chance(1, 2) {
        b.add(Msg::Cant { mon, reason: attract_arg(), mv: None, kw: Kw::NONE });
        return Some(Relay::FAIL);
    }
    None
}

/// `onEnd(pokemon)`: `-end|MON|Attract|[silent]`.
// PS: data/moves.ts attract.condition.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::EndEffect { mon, effect: attract_arg(), kw: Kw::flags(KwFlags::SILENT) });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Update, on_update),
    Handler::on(EventId::BeforeMove, on_before_move).priority(2),
    Handler::on(EventId::End, on_end),
]);
