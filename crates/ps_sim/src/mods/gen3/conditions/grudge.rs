//! `grudge`: Grudge's volatile. If a move makes the holder faint, that
//! move of the attacker loses all its PP; it goes before the holder's next
//! move. No effect state of its own.
//!
//! PS: data/moves.ts grudge.condition

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg, Text, TextId};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{conditions, moves};

/// `onStart(pokemon)`: `-singlemove|MON|Grudge`.
// PS: data/moves.ts grudge.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::SingleMove { mon, effect: Arg::new(ArgValue::Move(MoveNum(moves::GRUDGE.0))), kw: Kw::NONE });
    None
}

/// `onFaint(target, source, effect)`: a move (not a future move) empties
/// the PP of the attacker's last move: `-activate|SOURCE|move:
/// Grudge|MOVE` per slot (no Max moves in gen 3).
// PS: data/moves.ts grudge.condition.onFaint
fn on_faint(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let source = a.source?;
    if b.mon(source).hp == 0 || a.effect.is_none() {
        return None;
    }
    let EffectId::Move(id) = a.effect else { return None };
    let last = b.mon(source).last_move;
    if b.dex.move_def(id).flags.has("futuremove") || last.is_none() {
        return None;
    }
    for i in 0..4 {
        let slot = b.mon(source).move_slots[i];
        if slot.id.is_none() || slot.id != last {
            continue;
        }
        // `moveSlot.pp = 0` on the slot object, which `baseMoveSlots`
        // shares until Transform or Mimic replaces it (as `deductPP`).
        let mon = b.mon_mut(source);
        mon.move_slots[i].pp = 0;
        if !mon.move_slots[i].virtual_ && !mon.transformed {
            mon.base_move_slots[i].pp = 0;
        }
        let mon = b.ident(source);
        let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::GRUDGE.0)));
        let mv = Arg::new(ArgValue::Move(MoveNum(last.0)));
        b.add(Msg::Activate { mon: Some(mon), effect, a: Some(mv), b: None, kw: Kw::NONE });
    }
    None
}

/// `onBeforeMove(pokemon)`: removed before the holder's next move.
// PS: data/moves.ts grudge.condition.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.debug(Text::new(TextId::RemovingGrudge));
    b.remove_volatile(pokemon, conditions::GRUDGE);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Faint, on_faint),
    Handler::on(EventId::BeforeMove, on_before_move).priority(100),
]);
