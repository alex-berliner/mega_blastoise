//! `beatup`: Beat Up's volatile (its `onModifyMove` adds it), `duration:
//! 1`. Each strike's attack is the next ally's base Attack and the
//! target's defense its base Defense, the event's other modifiers
//! dropped (`this.event.modifier = 1`). No effect state; `move.allies` is
//! the move object's.
//!
//! PS: data/mods/gen3/moves.ts beatup.condition

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler, Prefix, Rule};
use crate::relay::Relay;
use crate::state::{Battle, MonRef};
use ps_dex::gen3::moves;

/// `this.dex.species.get(pokemon.set.species).baseStats[stat]`.
fn set_base_stat(b: &Battle, m: MonRef, stat: usize) -> i32 {
    i32::from(b.dex.species_def(b.mon(m).base_species).base_stats[stat])
}

/// `onModifySpA(atk, pokemon, defender, move)`: `-activate|MON|move: Beat
/// Up|[of] NAME` (unless Beat Up Nicknames Mod), and the next ally's base
/// Attack.
// PS: data/mods/gen3/moves.ts beatup.condition.onModifySpA
fn on_modify_spa(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let first = {
        let mv = b.active_move().expect("Beat Up's onModifySpA without a move");
        assert!(mv.allies_len != 0, "Beat Up's onModifySpA with no ally left");
        MonRef::new(pokemon.side, mv.allies[0])
    };
    if !b.has_rule(Rule::BeatUpNicknamesMod) {
        let mon = b.ident(pokemon);
        let of = b.ident(first);
        b.add(Msg::Activate {
            mon: Some(mon),
            effect: Arg::new(ArgValue::MoveEffect(MoveNum(moves::BEATUP.0))),
            a: None,
            b: None,
            kw: Kw::of_nickname(of),
        });
    }
    b.event.modifier = Some(4096);
    // `move.allies.shift()`.
    let mv = b.active_move_mut().expect("Beat Up's onModifySpA without a move");
    mv.allies.copy_within(1.., 0);
    mv.allies_len -= 1;
    Some(Relay::Int(set_base_stat(b, first, 1)))
}

/// `onFoeModifySpD(def, pokemon)`: the target's base Defense.
// PS: data/mods/gen3/moves.ts beatup.condition.onFoeModifySpD
fn on_foe_modify_spd(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.event.modifier = Some(4096);
    Some(Relay::Int(set_base_stat(b, pokemon, 2)))
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::ModifySpA, on_modify_spa).priority(-101),
    Handler::on(EventId::ModifySpD, on_foe_modify_spd).prefix(Prefix::Foe).priority(-101),
]);
