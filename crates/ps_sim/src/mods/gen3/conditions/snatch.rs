//! `snatch`: Snatch's one-turn volatile. The next move with the `snatch`
//! flag that any Pokemon prepares is used by the holder instead (with the
//! condition as its source effect: `|move|...|[from] Snatch`). A Pressure
//! holder whose move is snatched costs the Snatch user extra PP.
//!
//! Effect state: `source` (the Snatch user) and `sourceEffect` (Snatch).
//!
//! PS: data/moves.ts snatch.condition.onStart; data/mods/gen4/moves.ts
//! snatch.condition.onAnyPrepareHit

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::actions::ActiveMove;
use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::{Battle, Holder};
use ps_dex::gen3::{conditions, moves};

/// `onStart(pokemon)`: `-singleturn|MON|Snatch`.
// PS: data/moves.ts snatch.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    let effect = Arg::new(ArgValue::Move(MoveNum(moves::SNATCH.0)));
    b.add(Msg::SingleTurn { mon, effect, kw: Kw::NONE });
    None
}

/// `onAnyPrepareHit(source, target, move)`: steals a `snatch` move, then
/// `null`. Sky Drop is gen 5; no gen 3 move is a Z or Max move.
// PS: data/mods/gen4/moves.ts snatch.condition.onAnyPrepareHit
fn on_any_prepare_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let source = a.target.mon()?;
    let st = *b.effect_state();
    let snatch_user = st.source.expect("Snatch's volatile has a source");
    let mv = *b.active_move()?;
    if mv.id.is_none() || !b.dex.move_def(mv.id).flags.has("snatch") {
        return None;
    }
    b.remove_volatile(snatch_user, conditions::SNATCH);
    let (mon, of) = (b.ident(snatch_user), b.ident(source));
    let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::SNATCH.0)));
    b.add(Msg::Activate { mon: Some(mon), effect, a: None, b: None, kw: Kw { of: Some(of), ..Kw::NONE } });
    let pp_drop = b.run_event(EventId::DeductPP, Holder::Mon(source), Some(snatch_user), st.source_effect, None);
    let extra_pp = if pp_drop == Relay::TRUE { 0 } else { pp_drop.as_int().unwrap_or(0) };
    if extra_pp > 0 {
        let snatch = match st.source_effect {
            EffectId::Move(m) => m,
            other => panic!("Snatch's volatile came from {other:?}"),
        };
        b.deduct_pp(snatch_user, snatch, extra_pp);
    }
    let new_move = b.new_active_move(ActiveMove::from_dex(b.dex, mv.id));
    b.use_move(new_move, snatch_user, None, EffectId::None);
    Some(Relay::Null)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::PrepareHit, on_any_prepare_hit).prefix(Prefix::Any).priority(-1),
]);
