//! Hidden Power: the type and power come from the user's IVs (`hpType`,
//! `hpPower`, set by the Pokemon constructor or a typed `hiddenpowerX`
//! move in the team, and copied by Transform), and in gen 3 the category
//! follows the type like every other damaging move's.
//!
//! Gen 3's `useMoveInner` runs no `ModifyType` event, so `onModifyType`
//! (base) never fires in gen 3; it is registered because the resolved
//! move has it. `basePowerCallback` is gen 5's, `onModifyMove` gen 3's. No
//! effect state.
//!
//! PS: data/moves.ts hiddenpower.onModifyType, data/mods/gen5/moves.ts
//! hiddenpower.basePowerCallback, data/mods/gen3/moves.ts
//! hiddenpower.onModifyMove

use ps_dex::{MoveCategory, Type};
use ps_protocol::{Text, TextArg, TextId};

use crate::effect::{EventArgs, EventId, Handler, EffectDef};
use crate::relay::Relay;
use crate::state::Battle;

/// `pokemon.hpType || 'Dark'`.
fn hp_type(b: &Battle, a: &EventArgs) -> Type {
    let pokemon = a.target.mon().expect("Hidden Power without a user");
    match b.mon(pokemon).hp_type {
        Type::None => Type::Dark,
        t => t,
    }
}

/// `onModifyType(move, pokemon) { move.type = pokemon.hpType || 'Dark'; }`
// PS: data/moves.ts hiddenpower.onModifyType
fn on_modify_type(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let t = hp_type(b, a);
    b.active_move_mut().expect("ModifyType without an active move").type_ = t;
    None
}

/// `basePowerCallback(pokemon) { const bp = pokemon.hpPower || 70;
/// this.debug(`BP: ${bp}`); return bp; }`
// PS: data/mods/gen5/moves.ts hiddenpower.basePowerCallback
fn base_power_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    // PS calls the callback with the move's user first: the event's target.
    let pokemon = a.target.mon().expect("basePowerCallback without a user");
    let bp = match b.mon(pokemon).hp_power {
        0 => 70,
        p => i32::from(p),
    };
    b.debug(Text::with1(TextId::BasePower, TextArg::int(bp)));
    Some(Relay::Int(bp))
}

/// `onModifyMove(move, pokemon)`: the type, then the category by type.
// PS: data/mods/gen3/moves.ts hiddenpower.onModifyMove
fn on_modify_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let t = hp_type(b, a);
    let special = matches!(
        t,
        Type::Fire | Type::Water | Type::Grass | Type::Ice | Type::Electric | Type::Dark | Type::Psychic | Type::Dragon
    );
    let mv = b.active_move_mut().expect("ModifyMove without an active move");
    mv.type_ = t;
    mv.category = if special { MoveCategory::Special } else { MoveCategory::Physical };
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::ModifyType, on_modify_type),
    Handler::on(EventId::BasePowerCallback, base_power_callback),
    Handler::on(EventId::ModifyMove, on_modify_move),
]);
