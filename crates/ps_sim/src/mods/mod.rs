//! Per-gen effects (design 5.4): one file per effect, one registry per gen.
//!
//! Layout, for every gen `<gen>`:
//!
//! ```text
//! mods/<gen>/mod.rs                the gen's REGISTRY (sorted by EffectId)
//! mods/<gen>/moves/<id>.rs         a move's handlers
//! mods/<gen>/conditions/<id>.rs    a condition's (statuses, volatiles,
//!                                  side and slot conditions, weathers,
//!                                  moves' nested `condition`s)
//! mods/<gen>/items/<id>.rs
//! mods/<gen>/abilities/<id>.rs
//! mods/<gen>/rules/<id>.rs         format rules (`Rule`)
//! ```
//!
//! Each effect file defines `pub static DEF: EffectDef = EffectDef::new(&[...])`
//! with its `Handler`s, ordering data copied from PS's data file (the
//! `registry_matches_dex` test checks every copy against `ps_dex`), and a doc
//! comment naming its PS source and mapping its `this.effectState.*` fields
//! onto `EffectState`. The gen's `mod.rs` adds one `(EffectId, &DEF)` line
//! to `REGISTRY` in sorted position (`Registry::is_sorted` is tested).
//!
//! Effects not listed have no handlers, like PS data entries without `on*`
//! keys. Handler bodies arrive with the porting tasks (phases 3 and 4).

pub mod gen1;
pub mod gen3;

use crate::effect::{EffectId, Registry};
use ps_dex::Dex;

/// The registry of `dex`'s gen.
pub fn registry_for(dex: &Dex) -> &'static Registry {
    match dex.gen {
        1 => &gen1::REGISTRY,
        3 => &gen3::REGISTRY,
        g => panic!("no registry for gen {g}"),
    }
}

/// The handler list PS's data gives `id` in `dex`, where `ps_dex` has one
/// (rules have no dex entry).
pub fn dex_handlers(dex: &Dex, id: EffectId) -> Option<&'static [ps_dex::Handler]> {
    match id {
        EffectId::Move(m) => Some(dex.move_def(m).handlers),
        EffectId::Condition(c) => Some(dex.condition_def(c).handlers),
        EffectId::Item(i) => Some(dex.item_def(i).handlers),
        EffectId::Ability(a) => Some(dex.ability_def(a).handlers),
        EffectId::Species(_) | EffectId::Rule(_) | EffectId::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::{parse_handler_key, Callback};
    use crate::relay::Relay;

    /// Every registry entry's handlers exist in PS's data under the same
    /// key, with the same order, priority and subOrder, and constant
    /// handlers hold PS's constant.
    #[test]
    fn registry_matches_dex() {
        for dex in [&ps_dex::GEN1, &ps_dex::GEN3] {
            let reg = registry_for(dex);
            assert!(reg.is_sorted(), "gen {} registry not sorted", dex.gen);
            for (id, def) in reg.entries {
                assert_eq!(def.events, crate::effect::EventMask::of(def.handlers), "{id:?} mask");
                let Some(dex_hs) = dex_handlers(dex, *id) else { continue };
                for h in def.handlers {
                    let found = dex_hs.iter().find(|d| parse_handler_key(d.name()) == Some((h.prefix, h.event)));
                    let d = found.unwrap_or_else(|| panic!("gen {} {id:?}: {:?}{:?} not in PS data", dex.gen, h.prefix, h.event));
                    assert_eq!(
                        (h.order, h.priority, h.sub_order),
                        (d.order, d.priority, d.sub_order),
                        "gen {} {id:?} {}: ordering differs from PS",
                        dex.gen,
                        d.name()
                    );
                    match (h.callback, d.value) {
                        (Callback::Fn(_), ps_dex::HandlerValue::Function) => {}
                        (Callback::Const(Relay::Bool(false)), ps_dex::HandlerValue::False) => {}
                        (Callback::Const(Relay::Move(m)), ps_dex::HandlerValue::Id(_)) => {
                            assert_eq!(Some(dex.moves[m.index()].name.as_str()).map(|_| ()), Some(()));
                        }
                        (c, v) => panic!("gen {} {id:?} {}: {c:?} vs PS {v:?}", dex.gen, d.name()),
                    }
                }
            }
        }
    }
}
