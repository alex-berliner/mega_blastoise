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

use crate::effect::{parse_handler_key, EffectId, Registry};
use ps_dex::{ConditionId, Dex, MoveId};

/// The ported effects for `gen`: species, moves, items, abilities, conditions
/// and rules the new `ps_sim` has handlers for. The protocol-diff harness
/// (P1-7) consumes this via `ps_sim_cli --ported <gen>` so the wide
/// generator draws only from what a port already covers. Gen 1 lists the
/// moves [`move_is_ported`] accepts (P1-8); gen 3's list is empty until its
/// pipeline lands.
///
/// Each entry is `<kind>:<id>` where `kind` is one of `move`, `species`,
/// `item`, `ability`, `condition`, `rule`. The protocol-diff harness
/// parses the kind prefix and routes the id to the appropriate pool. An
/// empty list is a valid output: `--teams wide` against `ps_sim_cli`
/// exits 2 with "no ported pipeline for gen N" when the list is empty,
/// while `--self --teams wide` draws from the whole showdown_fuzz space
/// regardless of the list.
pub fn ported(gen: u8) -> &'static [&'static str] {
    match gen {
        1 => gen1::PORTED,
        3 => &gen3::PORTED,
        _ => &[],
    }
}

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
        EffectId::Species(_) | EffectId::Rule(_) | EffectId::Pseudo(_) | EffectId::None => None,
    }
}

/// True when `registry` holds every handler PS's data gives `id` in `dex`
/// (a move's secondaries and `self` effects included): an effect without
/// handlers is ported as it is. The pipelines panic `unported: <id>` when
/// they reach an effect that is not ([`crate::state::Battle::require_ported`]).
pub fn is_ported(dex: &Dex, registry: &Registry, id: EffectId) -> bool {
    let all_in = |hs: &[ps_dex::Handler]| {
        hs.iter().all(|h| parse_handler_key(h.name()).is_some_and(|(p, e)| registry.handler(id, p, e).is_some()))
    };
    match id {
        EffectId::Move(m) => {
            let def = dex.move_def(m);
            // Secondaries and `self` effects have no id to register under.
            let self_free = |s: Option<&ps_dex::SelfEffect>| s.is_none_or(|s| s.handlers.is_empty());
            all_in(def.handlers)
                && self_free(def.self_effect())
                && def.secondaries().iter().all(|s| s.handlers.is_empty() && self_free(s.self_effect))
        }
        _ => dex_handlers(dex, id).is_none_or(all_in),
    }
}

/// Every condition a move's data can put on the field: its status,
/// volatile status, own condition, side, slot, weather and pseudo-weather
/// conditions, and those of its `self` effect and secondaries.
pub fn move_conditions(dex: &Dex, m: MoveId) -> impl Iterator<Item = ConditionId> {
    let def = dex.move_def(m);
    let selfs = |s: Option<&'static ps_dex::SelfEffect>| {
        s.into_iter().flat_map(|s| [s.status, s.volatile_status, s.side_condition, s.pseudo_weather])
    };
    [
        def.status,
        def.volatile_status,
        def.condition,
        def.side_condition(),
        def.slot_condition(),
        def.weather(),
        def.pseudo_weather(),
    ]
    .into_iter()
    .chain(selfs(def.self_effect()))
    .chain(def.secondaries().iter().flat_map(move |s| [s.status, s.volatile_status].into_iter().chain(selfs(s.self_effect))))
    .filter(|c| !c.is_none())
}

/// A move whose use reaches no unported effect: it is ported and so is
/// every condition its data names.
pub fn move_is_ported(dex: &Dex, registry: &Registry, m: MoveId) -> bool {
    is_ported(dex, registry, EffectId::Move(m))
        && move_conditions(dex, m).all(|c| is_ported(dex, registry, EffectId::Condition(c)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Callback;
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
                        (Callback::Const(Relay::Recharge), ps_dex::HandlerValue::Id(_)) => {
                            assert_eq!(d.constant_id(), Some("recharge"));
                        }
                        (c, v) => panic!("gen {} {id:?} {}: {c:?} vs PS {v:?}", dex.gen, d.name()),
                    }
                }
            }
        }
    }

    /// The `ported(gen)` lists are exactly what the dex and the registry
    /// allow, sorted: `move:<id>` for every [`move_is_ported`] move but
    /// Struggle, and `species:<id>` for every species (PS species have no
    /// handlers; the harness's wide teams draw from these).
    #[test]
    fn ported_lists_are_the_dex_derivation() {
        extern crate std;
        use std::string::String;
        use std::vec::Vec;
        for dex in [&ps_dex::GEN1, &ps_dex::GEN3] {
            let gen = dex.gen;
            if gen == 3 {
                // The gen 3 pipeline is P1-9's.
                assert!(ported(gen).is_empty());
                continue;
            }
            let reg = registry_for(dex);
            let moves = (0..dex.moves.len())
                .map(|i| MoveId(i as u16))
                .filter(|&m| move_is_ported(dex, reg, m))
                .map(|m| ps_id(dex.move_def(m).name.as_str()))
                .filter(|id| id != "struggle")
                .map(|id| alloc::format!("move:{id}"));
            let species = dex.species.iter().filter(|s| s.num > 0).map(|s| alloc::format!("species:{}", ps_id(s.name.as_str())));
            let mut derived: Vec<String> = moves.chain(species).collect();
            derived.sort();
            let listed: Vec<String> = ported(gen).iter().map(|s| String::from(*s)).collect();
            assert_eq!(listed, derived, "gen {gen}: ported list is stale; derived:\n{derived:?}");
        }
    }

    /// PS `toID`.
    fn ps_id(name: &str) -> alloc::string::String {
        name.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect()
    }
}

/// A name as PS's `toID` prints it, without allocating.
pub struct PsId<'a>(pub &'a str);

impl core::fmt::Display for PsId<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for c in self.0.chars().filter(char::is_ascii_alphanumeric) {
            core::fmt::Write::write_char(f, c.to_ascii_lowercase())?;
        }
        Ok(())
    }
}
