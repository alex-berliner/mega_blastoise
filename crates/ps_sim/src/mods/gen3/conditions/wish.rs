//! `wish`: Wish's slot condition. At the end of the next turn the Pokemon
//! in the slot heals half its base maximum HP. `duration: 2`, ended by the
//! residual (`onResidualOrder: 7`, the dex's `residual_order`).
//!
//! Effect state: `source` is the wisher.
//!
//! PS: data/mods/gen4/moves.ts wish.condition (onEnd)

use ps_protocol::{Arg, ArgValue, Kw, KwFlags, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `onEnd(target)`: `this.heal(target.baseMaxhp / 2, target, target)`
/// (`battle.heal` writes no line for Wish; `heal` makes anything in (0, 1]
/// one), then `-heal|MON|HP|[from] move: Wish|[wisher] NAME` if it healed.
// PS: data/mods/gen4/moves.ts wish.condition.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    if b.mon(target).fainted {
        return None;
    }
    let source = b.effect_state().source.expect("a Wish without a wisher");
    let amount = (i32::from(b.mon(target).base_maxhp) / 2).max(1);
    let damage = b.heal(amount, Some(target), Some(target), EffectId::None);
    if damage.truthy() {
        let (mon, health) = (b.ident(target), b.health(target));
        let from = Arg::new(ArgValue::MoveEffect(MoveNum(moves::WISH.0)));
        let kw = Kw { from: Some(from), of: Some(b.ident(source)), flags: KwFlags::WISHER, repeat_miss: 0 };
        b.add(Msg::Heal { mon, health, kw });
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::End, on_end)]);
