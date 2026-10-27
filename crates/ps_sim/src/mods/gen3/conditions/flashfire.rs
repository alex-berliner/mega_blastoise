//! Flash Fire's volatile (the ability's `condition`): the holder's Fire
//! moves deal 1.5x (`ModifyDamagePhase1`). `noCopy` (Baton Pass leaves it).
//! The `onModifyAtkPriority` / `onModifySpAPriority` keys have no handler
//! at the pin and are never read.
//!
//! No state of its own.
//!
//! PS: data/abilities.ts flashfire.condition (onStart, onEnd),
//! data/mods/gen4/abilities.ts flashfire.condition (onModifyDamagePhase1)

use ps_dex::Type;
use ps_protocol::{Kw, KwFlags, Msg, Text, TextId};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `ability: Flash Fire`, the volatile's name in its lines.
fn flash_fire_arg(b: &Battle) -> ps_protocol::Arg {
    let id = b.dex.ability_id("flashfire").expect("Flash Fire in the dex");
    ps_protocol::Arg::ability_effect(ps_protocol::AbilityNum(id.0))
}

/// `onStart(target)`: `-start|MON|ability: Flash Fire`.
// PS: data/abilities.ts flashfire.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    let effect = flash_fire_arg(b);
    b.add(Msg::StartEffect { mon, effect, a: None, b: None, kw: Kw::NONE });
    None
}

/// `onEnd(target)`: `-end|MON|ability: Flash Fire|[silent]`.
// PS: data/abilities.ts flashfire.condition.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    let effect = flash_fire_arg(b);
    b.add(Msg::EndEffect { mon, effect, kw: Kw::flags(KwFlags::SILENT) });
    None
}

/// `onModifyDamagePhase1(atk, attacker, defender, move)`: Fire moves 1.5x.
// PS: data/mods/gen4/abilities.ts flashfire.condition.onModifyDamagePhase1
fn on_modify_damage_phase1(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if super::event_move_type(b, a) == Type::Fire {
        b.debug(Text::new(TextId::FlashFireBoost));
        b.chain_modify_f(1.5);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::End, on_end),
    Handler::on(EventId::ModifyDamagePhase1, on_modify_damage_phase1),
]);
