//! Swallow: heals a quarter, half or all of the user's HP for one, two or
//! three `stockpile` layers, then the layers go. Fails without them. No
//! effect state.
//!
//! PS: data/mods/gen4/moves.ts swallow.onTry, data/mods/gen7/moves.ts
//! swallow.onHit

use ps_protocol::{Arg, ArgValue, Kw, Msg, Word};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::mods::gen3::moves::stockpile::layers;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `onTry(source)`: `!!source.volatiles['stockpile']`.
// PS: data/mods/gen4/moves.ts swallow.onTry
fn on_try(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let source = a.target.mon()?;
    Some(Relay::Bool(b.mon(source).volatiles.contains(EffectId::Condition(conditions::STOCKPILE))))
}

/// `onHit(pokemon)`: the heal, `-fail|MON|heal` when nothing healed, and
/// the layers go.
// PS: data/mods/gen7/moves.ts swallow.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let layers = match layers(b, pokemon) {
        0 => 1,
        n => n,
    };
    let fraction = [0.25, 0.5, 1.0][usize::from(layers) - 1];
    let amount = b.modify_f(i64::from(b.mon(pokemon).maxhp), fraction) as i32;
    let success = b.heal(amount, None, None, EffectId::None).truthy();
    if !success {
        let mon = b.ident(pokemon);
        b.add(Msg::Fail { mon, what: Some(Arg::new(ArgValue::Word(Word::Heal))), what2: None, kw: Kw::NONE });
    }
    b.remove_volatile(pokemon, conditions::STOCKPILE);
    Some(if success { Relay::TRUE } else { Relay::Null })
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Try, on_try),
    Handler::on(EventId::Hit, on_hit),
]);
