//! Aromatherapy: cures the status of every Pokemon on the user's side,
//! benched ones too, with one `-cureteam` line. No effect state.
//!
//! PS: data/mods/gen4/moves.ts aromatherapy.onHit

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `onHit(target, source)`: `-cureteam|SOURCE|[from] move: Aromatherapy`,
/// then `clearStatus()` on each of `target.side.pokemon` (no ally side in
/// singles).
// PS: data/mods/gen4/moves.ts aromatherapy.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source?;
    let mon = b.ident(source);
    let from = Arg::new(ArgValue::MoveEffect(MoveNum(moves::AROMATHERAPY.0)));
    b.add(Msg::CureTeam { mon, kw: Kw::from(from) });
    let side = target.side;
    for i in 0..b.sides[side as usize].team_size as usize {
        let ally = b.pokemon_at(side, i);
        b.clear_status(ally);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
