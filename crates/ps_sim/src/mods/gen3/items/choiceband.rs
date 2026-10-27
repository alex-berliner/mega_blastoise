//! `choiceband`: Choice Band, the holder's attack times 1.5, and the lock
//! (`choicelock`) on the move it used, added after the move.
//!
//! PS: data/items.ts choiceband (onModifyAtk), data/mods/gen4/items.ts
//! choiceband (onAfterMove)

use ps_dex::gen3::conditions;

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifyAtk(atk, pokemon)`: `if (pokemon.volatiles["dynamax"])
/// return; return this.chainModify(1.5)` (no Dynamax in gen 3).
// PS: data/items.ts choiceband.onModifyAtk
fn on_modify_atk(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    b.chain_modify_f(1.5);
    None
}

/// `onAfterMove(pokemon)`: `pokemon.addVolatile("choicelock")`.
// PS: data/mods/gen4/items.ts choiceband.onAfterMove
fn on_after_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.add_volatile(pokemon, conditions::CHOICELOCK, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::ModifyAtk, on_modify_atk).priority(1),
    Handler::on(EventId::AfterMove, on_after_move),
]);
