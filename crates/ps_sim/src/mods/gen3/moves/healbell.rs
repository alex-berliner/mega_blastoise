//! Heal Bell: cures the status of every Pokemon on the user's side,
//! benched ones too, except those with Soundproof. No effect state.
//!
//! PS: data/mods/gen4/moves.ts healbell.onHit

use ps_protocol::{AbilityNum, Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{abilities, moves};

/// `onHit(target, source)`: `-activate|SOURCE|move: Heal Bell`, then each
/// of `target.side.pokemon` (no ally side in singles) is cured silently,
/// unless it has Soundproof (an active one says so).
// PS: data/mods/gen4/moves.ts healbell.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source?;
    let mon = b.ident(source);
    let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::HEALBELL.0)));
    b.add(Msg::Activate { mon: Some(mon), effect, a: None, b: None, kw: Kw::NONE });
    let side = target.side;
    for i in 0..b.sides[side as usize].team_size as usize {
        let ally = b.pokemon_at(side, i);
        if b.has_ability(ally, abilities::SOUNDPROOF) && !b.suppressing_ability(Some(ally)) {
            if b.mon(ally).is_active {
                let mon = b.ident(ally);
                let from = Arg::new(ArgValue::AbilityEffect(AbilityNum(abilities::SOUNDPROOF.0)));
                b.add(Msg::Immune { mon, what: None, kw: Kw::from(from) });
            }
            continue;
        }
        b.cure_status(ally, true);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
