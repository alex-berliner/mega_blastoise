//! Intimidate: on switch-in, every adjacent foe's Attack drops by one. In
//! gen 3 it does not activate at all when every foe has a Substitute, and a
//! foe behind one is immune. `onSwitchInPriority: 2` (before Trace, after
//! the weather abilities): the dex's `switch_in_order`.
//!
//! No ability state.
//!
//! PS: data/mods/gen3/abilities.ts intimidate

use ps_dex::gen3::conditions as c;
use ps_protocol::{AbilityNum, Kw, Msg, TextId};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onStart(pokemon)`.
// PS: data/mods/gen3/abilities.ts intimidate.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let (foes, n) = b.foes(pokemon.side);
    let activated = foes[..n].iter().any(|&t| !b.has_vol(t, c::SUBSTITUTE));
    if !activated {
        let side = b.ident(pokemon).side;
        b.hint(TextId::HintGen3IntimidateSub, false, Some(side));
        return None;
    }
    let EffectId::Ability(id) = b.frame.effect else { unreachable!("Intimidate's own handler") };
    let mon = b.ident(pokemon);
    b.add(Msg::Ability { mon, ability: AbilityNum(id.0), old: None, boost: true, kw: Kw::NONE, split: None });
    // `pokemon.adjacentFoes()` again: the same list in singles.
    let (foes, n) = b.foes(pokemon.side);
    for &target in &foes[..n] {
        if b.has_vol(target, c::SUBSTITUTE) {
            let mon = b.ident(target);
            b.add(Msg::Immune { mon, what: None, kw: Kw::NONE });
        } else {
            // `this.boost({atk: -1}, target, pokemon, null, true)`.
            b.boost_full([-1, 0, 0, 0, 0, 0, 0], target, Some(pokemon), EffectId::None, true, false);
        }
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Start, on_start)]);
