//! Knock Off: after the hit the target loses its item (gens 3-4: it is
//! only knocked off, `itemKnockedOff`, so it is lost again on every
//! switch-in, `switchIn`). No effect state.
//!
//! PS: data/mods/gen4/moves.ts knockoff.onAfterHit

use ps_protocol::{Arg, ArgValue, ItemNum, Kw, MoveNum, Msg, TextId};

use crate::effect::{EffectId, EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, Holder};
use ps_dex::gen3::moves;

/// `onAfterHit(target, source, move)`: if `TakeItem` lets it go, `item =
/// ''` (no `End`, no cleared state), `itemKnockedOff = true` and
/// `-enditem|TARGET|ITEM|[from] move: Knock Off|[of] SOURCE`.
// PS: data/mods/gen4/moves.ts knockoff.onAfterHit
fn on_after_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source.expect("Knock Off's onAfterHit without a source");
    let item = b.mon(target).item;
    if item.is_none() {
        return None;
    }
    let relay = Some(Relay::Effect(EffectId::Item(item)));
    if b.run_event(EventId::TakeItem, Holder::Mon(target), Some(source), a.effect, relay).truthy() {
        let mon = b.mon_mut(target);
        mon.item = ps_dex::ItemId::NONE;
        mon.item_knocked_off = true;
        let tgt = b.ident(target);
        let src = b.ident(source);
        let from = Arg::new(ArgValue::MoveEffect(MoveNum(moves::KNOCKOFF.0)));
        b.add(Msg::EndItem { mon: tgt, item: ItemNum(item.0), kw: Kw::from_of(from, src) });
        if b.gen == 4 {
            b.hint(TextId::HintGens3To4KnockOff, true, None);
        }
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::AfterHit, on_after_hit)]);
