//! `magiccoat`: Magic Coat's one-turn volatile. The first reflectable move
//! aimed at its holder by another Pokemon is used back at that Pokemon
//! instead (a new move object with `hasBounced`, used by the holder with
//! the condition as its source effect: `|move|...|[from] Magic Coat`).
//!
//! Effect state: `pranksterBoosted` (gen 7) is not kept.
//!
//! PS: data/moves.ts magiccoat.condition.onStart; data/mods/gen4/moves.ts
//! magiccoat.condition.onTryHit

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::actions::ActiveMove;
use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{conditions, moves};

/// `onStart(target, source, effect)`: `-singleturn|MON|move: Magic Coat`.
// PS: data/moves.ts magiccoat.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::MAGICCOAT.0)));
    b.add(Msg::SingleTurn { mon, effect, kw: Kw::NONE });
    None
}

/// `onTryHit(target, source, move)`: bounces a reflectable move from
/// another Pokemon, then `null`.
// PS: data/mods/gen4/moves.ts magiccoat.condition.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source?;
    let mv = *b.active_move().expect("TryHit runs for a move");
    if target == source || mv.has_bounced || mv.id.is_none() || !b.dex.move_def(mv.id).flags.has("reflectable") {
        return None;
    }
    b.remove_volatile(target, conditions::MAGICCOAT);
    let new_move = b.new_active_move(ActiveMove { has_bounced: true, ..ActiveMove::from_dex(b.dex, mv.id) });
    b.use_move(new_move, target, Some(Some(source)), EffectId::None);
    Some(Relay::Null)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::TryHit, on_try_hit).priority(2),
]);
