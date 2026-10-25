//! Spite: the target's last move loses 2 to 5 PP. No effect state.
//!
//! PS: data/mods/gen3/moves.ts spite.onHit

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::actions::GenScripts;
use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::scripts::Gen3Scripts;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `onHit(target)`: `roll = this.random(2, 6)` (always drawn), then
/// `-activate|TARGET|move: Spite|LASTMOVEID|ROLL` if the last move lost PP,
/// else `false`.
// PS: data/mods/gen3/moves.ts spite.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let roll = b.prng.random_range(2, 6) as i32;
    let last = b.mon(target).last_move;
    if !last.is_none() && Gen3Scripts::deduct_pp(b, target, last, roll) != 0 {
        let mon = b.ident(target);
        b.add(Msg::Activate {
            mon: Some(mon),
            effect: Arg::new(ArgValue::MoveEffect(MoveNum(moves::SPITE.0))),
            a: Some(Arg::new(ArgValue::MoveId(MoveNum(last.0)))),
            b: Some(Arg::new(ArgValue::Int(roll as i16))),
            kw: Kw::NONE,
        });
        return None;
    }
    Some(Relay::FAIL)
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
