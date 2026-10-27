//! Focus Punch: before the turn the user focuses (the `focuspunch`
//! volatile); if a damaging hit broke the focus, the punch fails with
//! `cant|MON|Focus Punch|Focus Punch`. No effect state.
//!
//! PS: data/mods/gen4/moves.ts focuspunch (`beforeTurnCallback`, `onTry`)

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::mods::gen3::conditions::focuspunch::LOST_FOCUS;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{conditions, moves};

/// `beforeTurnCallback(pokemon)`: `pokemon.addVolatile('focuspunch')`.
// PS: data/mods/gen4/moves.ts focuspunch.beforeTurnCallback
fn before_turn_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.add_volatile(pokemon, conditions::FOCUSPUNCH, None, EffectId::None);
    None
}

/// `onTry(pokemon)`: the punch fails once the focus is lost.
// PS: data/mods/gen4/moves.ts focuspunch.onTry
fn on_try(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let lost = b
        .mon(pokemon)
        .volatiles
        .get(EffectId::Condition(conditions::FOCUSPUNCH))
        .is_some_and(|st| st.data[LOST_FOCUS] != 0);
    if lost {
        b.attr_last_move_still();
        let mon = b.ident(pokemon);
        let name = Arg::new(ArgValue::Move(MoveNum(moves::FOCUSPUNCH.0)));
        b.add(Msg::Cant { mon, reason: name, mv: Some(name), kw: Kw::NONE });
        return Some(Relay::Null);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::BeforeTurnCallback, before_turn_callback),
    Handler::on(EventId::Try, on_try),
]);
