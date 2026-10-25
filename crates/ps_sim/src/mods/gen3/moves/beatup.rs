//! Beat Up: one typeless special strike for each healthy, unstatused
//! Pokemon of the user's side, each with that Pokemon's base Attack
//! against the target's base Defense (the `beatup` volatile swaps the
//! stats: see `conditions/beatup.rs`). No effect state; `move.allies`
//! lives on the move object.
//!
//! PS: data/mods/gen3/moves.ts beatup.onModifyMove,
//! data/mods/gen4/moves.ts beatup.basePowerCallback

use ps_dex::{MoveCategory, Type};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `basePowerCallback(pokemon, target, move)`: `null` with no ally left,
/// else 10.
// PS: data/mods/gen4/moves.ts beatup.basePowerCallback
fn base_power_callback(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let allies = b.active_move().expect("basePowerCallback without a move").allies_len;
    if allies == 0 {
        return Some(Relay::Null);
    }
    Some(Relay::Int(10))
}

/// `onModifyMove(move, pokemon)`: the volatile, type `???`, Special, and
/// one hit per ally (`side.pokemon` order, not fainted, no status).
// PS: data/mods/gen3/moves.ts beatup.onModifyMove
fn on_modify_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.add_volatile(pokemon, conditions::BEATUP, None, EffectId::None);
    let side = pokemon.side;
    let mut allies = [0u8; 6];
    let mut n = 0;
    for i in 0..b.sides[side as usize].team_size as usize {
        let ally = b.pokemon_at(side, i);
        if !b.mon(ally).fainted && b.mon(ally).status.is_none() {
            allies[n] = ally.index;
            n += 1;
        }
    }
    let mv = b.active_move_mut().expect("ModifyMove without an active move");
    mv.type_ = Type::None;
    mv.category = MoveCategory::Special;
    mv.allies = allies;
    mv.allies_len = n as u8;
    // `move.multihit = move.allies.length`: 0 is no multi-hit.
    mv.multihit = (n != 0).then_some([n as u8, n as u8]);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::BasePowerCallback, base_power_callback),
    Handler::on(EventId::ModifyMove, on_modify_move),
]);
