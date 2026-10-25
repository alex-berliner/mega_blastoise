//! Natural Cure: the holder's status goes when it switches out.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts naturalcure (onSwitchOut), data/mods/gen4/abilities.ts
//! naturalcure (onSwitchOut)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::{Kw, KwFlags, Msg};

/// `onSwitchOut(pokemon)`: the `-curestatus` line, then the status goes.
fn on_switch_out(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let status = b.mon(pokemon).status;
    // `if (!pokemon.status || pokemon.status === "fnt") return;`
    if status.is_none() {
        return None;
    }
    let key = b.dex.condition_def(status).key.as_str();
    let Some(status) = ps_protocol::Status::parse(key) else {
        panic!("unported: Natural Cure of the {key} status");
    };
    let mon = b.ident(pokemon);
    let kw = Kw::from(b.effect_fullname(b.frame.effect)).with(KwFlags::SILENT);
    b.add(Msg::CureStatus { mon, status, kw });
    // `pokemon.clearStatus()`.
    b.clear_status(pokemon);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::SwitchOut, on_switch_out)]);
