//! Conversion: the user takes the target's types.
//!
//! No effect state.
//!
//! PS: data/mods/gen1/moves.ts conversion.onHit (line 138; gen 1's own handler;
//! the base move has none)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::moves;
use ps_protocol::{Arg, ArgValue, Kw, Msg, MoveNum, TypeNum, Word};

/// `onHit(target, source)`.
// PS: data/mods/gen1/moves.ts conversion.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    // The event's target is the move's target and its source the user.
    let target = a.target.mon()?;
    let source = a.source?;
    // `source.setType(target.getTypes(true))`: the user takes the
    // target's types (the added type of a Forest's Curse does not count,
    // and gen 1 has none).
    let types = b.get_types(target);
    b.mon_mut(source).types = types;
    // `this.add('-start', source, 'typechange', source.types.join('/'),
    // '[from] move: Conversion', `[of] ${target}`)`.
    let mon = b.ident(source);
    let effect = Arg::new(ArgValue::Word(Word::TypeChange));
    let types_arg = match types.as_slice() {
        [one] => Arg::ty(TypeNum(one.index())),
        [one, two] => Arg::type_pair(TypeNum(one.index()), TypeNum(two.index())),
        // A Pokemon with no type at all: PS's `getTypes` answers `???`.
        _ => Arg::ty(TypeNum(ps_dex::Type::None.index())),
    };
    let kw = Kw::from_of(Arg::move_effect(MoveNum(moves::CONVERSION.0)), b.ident(target));
    b.add(Msg::StartEffect { mon, effect, a: Some(types_arg), b: None, kw });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
