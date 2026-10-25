//! Baton Pass: the user switches out (`selfSwitch: 'copyvolatile'`), and
//! the replacement keeps its boosts and volatiles
//! (`Battle::copy_volatile_from`). It fails with no Pokemon to switch to.
//! Its `self` effect skips Pursuit for that switch-out. No effect state.
//!
//! PS: data/moves.ts batonpass (`onHit`, `self.onHit`)

use ps_protocol::{Kw, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onHit(target)`: with no Pokemon to switch to (no Commander in gen 3),
/// `[still]`, `-fail|MON` and `NOT_FAIL`.
// PS: data/moves.ts batonpass.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    if b.can_switch(target.side) == 0 {
        b.attr_last_move_still();
        let mon = b.ident(target);
        b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
        return Some(Relay::NOT_FAIL);
    }
    None
}

/// `self.onHit(source)`: `source.skipPursuit = true`.
// PS: data/moves.ts batonpass.self.onHit
fn self_on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let source = a.target.mon()?;
    b.mon_mut(source).skip_pursuit = true;
    None
}

static SELF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, self_on_hit)]);

pub static DEF: EffectDef = EffectDef::with_self(&[Handler::on(EventId::Hit, on_hit)], &SELF);
