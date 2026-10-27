//! Pain Split: the user's and the target's HP both become their average.
//! No effect state.
//!
//! PS: data/moves.ts painsplit.onHit

use ps_protocol::{Arg, ArgValue, Kw, KwFlags, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `onHit(target, pokemon)`: the target first (`[silent]`), then the user.
/// `getUndynamaxedHP()` is the HP (no Dynamax in gen 3).
// PS: data/moves.ts painsplit.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let pokemon = a.source?;
    let target_hp = i32::from(b.mon(target).hp);
    let averagehp = match (target_hp + i32::from(b.mon(pokemon).hp)) / 2 {
        0 => 1,
        n => n,
    };
    let target_change = target_hp - averagehp;
    b.set_hp(target, target_hp - target_change);
    let from = Arg::new(ArgValue::MoveEffect(MoveNum(moves::PAINSPLIT.0)));
    let (mon, health) = (b.ident(target), b.health(target));
    b.add(Msg::SetHp { mon, health, kw: Kw::from(from).with(KwFlags::SILENT) });
    b.set_hp(pokemon, averagehp);
    let (mon, health) = (b.ident(pokemon), b.health(pokemon));
    b.add(Msg::SetHp { mon, health, kw: Kw::from(from) });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
