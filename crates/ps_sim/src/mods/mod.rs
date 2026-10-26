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

use crate::effect::{parse_handler_key, EffectId, EventId, Prefix, Registry};
use ps_dex::{ConditionId, Dex, MoveId};

/// The ported effects for `gen`: species, moves, items, abilities, conditions
/// and rules the new `ps_sim` has handlers for. The protocol-diff harness
/// (P1-7) consumes this via `ps_sim_cli --ported <gen>` so the wide
/// generator draws only from what a port already covers. Each list has
/// every move [`move_is_ported`] accepts but Struggle (no team carries it;
/// the pipeline uses it when nothing else is left) and Hidden Power's typed
/// placeholders (their move is `hiddenpower`), and every species (PS species
/// have no handlers). Gen 3's also has every ability [`is_ported`] accepts
/// and every item [`item_is_ported`] accepts (an item with no handlers, a
/// Poke Ball or an evolution stone, is ported as it is); gen 1 has neither
/// (its dex's No Ability is not listed, and it has no items). The
/// `ported_lists_are_the_dex_derivation` test derives each list from the dex
/// and the registry and fails, printing the new list, when they differ.
///
/// Each entry is `<kind>:<id>` where `kind` is one of `move`, `species`,
/// `item`, `ability`, `condition`, `rule`. The protocol-diff harness
/// parses the kind prefix and routes the id to the appropriate pool. An
/// empty list is a valid output: `--teams wide` against `ps_sim_cli`
/// exits 2 with "no ported pipeline for gen N" when the list is empty,
/// while `--self --teams wide` draws from the whole wide fuzz space
/// regardless of the list.
pub fn ported(gen: u8) -> &'static [&'static str] {
    match gen {
        1 => gen1::PORTED,
        3 => gen3::PORTED,
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

/// The handler a move's secondary has for `event`, when this gen ports
/// it. PS's secondaries have no id of their own (their
/// handlers sit in the move's data), so they are named here: gen 3's only
/// one is Tri Attack's secondary `onHit`.
pub fn secondary_handler(dex: &Dex, mv: MoveId, event: EventId) -> Option<&'static crate::effect::Handler> {
    match dex.gen {
        3 => crate::mods::gen3::moves::secondary_handler(mv, event),
        _ => None,
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
            // A move's `self` effect has no id of its own: its handlers live
            // in the move's [`EffectDef::self_def`]; a secondary's are the
            // gen's `secondary_handler`. A secondary's own `self` has
            // nowhere to go, so a move with handlers there is not ported.
            let self_free = |s: Option<&ps_dex::SelfEffect>| s.is_none_or(|s| s.handlers.is_empty());
            let self_in = def.self_effect().is_none_or(|s| {
                s.handlers
                    .iter()
                    .all(|h| parse_handler_key(h.name()).is_some_and(|(p, e)| registry.self_handler(id, p, e).is_some()))
            });
            all_in(def.handlers)
                && self_in
                && def.secondaries().iter().all(|s| secondary_ported(dex, m, s) && self_free(s.self_effect))
        }
        _ => dex_handlers(dex, id).is_none_or(all_in),
    }
}

/// A secondary's handlers are ported when the gen's [`secondary_handler`]
/// has one for each of them (gen 3: Tri Attack's secondary `onHit`).
fn secondary_ported(dex: &Dex, m: MoveId, s: &ps_dex::Secondary) -> bool {
    s.handlers.iter().all(|h| {
        parse_handler_key(h.name()).is_some_and(|(p, event)| p == Prefix::None && secondary_handler(dex, m, event).is_some())
    })
}

/// Every condition a move's data can put on the field: its status,
/// volatile status, own condition, side, slot, weather and pseudo-weather
/// conditions, and those of its `self` effect and secondaries; plus the
/// ones the gen's pipeline adds for it ([`pipeline_conditions`]).
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
    .chain(pipeline_conditions(dex, m))
    .filter(|c| !c.is_none())
}

/// Conditions a gen's pipeline adds for a move beyond what its data names:
/// gen 1's `tryMoveHit` confuses a Thrash or Petal Dance user when its
/// `lockedmove` runs out.
// PS: data/mods/gen1/scripts.ts tryMoveHit
fn pipeline_conditions(dex: &Dex, m: MoveId) -> impl Iterator<Item = ConditionId> {
    let locks = dex.gen == 1
        && dex.move_def(m).self_effect().is_some_and(|s| Some(s.volatile_status) == dex.condition_id("lockedmove"));
    locks.then(|| dex.condition_id("confusion")).flatten().into_iter()
}

/// A move whose use reaches no unported effect: it is ported and so is
/// every condition its data names. Metronome also calls any move with the
/// `metronome` flag (gen 1's pool; Metronome itself has no flag), so it
/// counts only once every one of those does. Mirror Move calls the foe's
/// last move, which a team of ported moves keeps ported.
pub fn move_is_ported(dex: &Dex, registry: &Registry, m: MoveId) -> bool {
    let own = is_ported(dex, registry, EffectId::Move(m))
        && move_conditions(dex, m).all(|c| is_ported(dex, registry, EffectId::Condition(c)));
    if !own || dex.move_def(m).name.as_str() != "Metronome" {
        return own;
    }
    (0..dex.moves.len()).map(|i| MoveId(i as u16)).filter(|&c| dex.move_def(c).flags.has("metronome")).all(|c| move_is_ported(dex, registry, c))
}

/// Conditions an item's handlers add, which its data does not name: Choice
/// Band's `onAfterMove` adds `choicelock`, King's Rock's flinch secondary
/// `flinch`, Lansat Berry's `onEat` `focusenergy`.
// PS: data/mods/gen4/items.ts choiceband onAfterMove, data/mods/gen3/items.ts
// kingsrock onModifyMove, data/items.ts lansatberry onEat
pub fn item_conditions(dex: &Dex, i: ps_dex::ItemId) -> impl Iterator<Item = ConditionId> {
    let adds = match dex.item_def(i).name.as_str() {
        "Choice Band" => "choicelock",
        "King's Rock" => "flinch",
        "Lansat Berry" => "focusenergy",
        _ => "",
    };
    (!adds.is_empty()).then(|| dex.condition_id(adds)).flatten().into_iter()
}

/// An item whose use reaches no unported effect: it is ported and so is
/// every condition it adds ([`item_conditions`]).
pub fn item_is_ported(dex: &Dex, registry: &Registry, i: ps_dex::ItemId) -> bool {
    is_ported(dex, registry, EffectId::Item(i)) && item_conditions(dex, i).all(|c| is_ported(dex, registry, EffectId::Condition(c)))
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
                if let Some(sd) = def.self_def {
                    assert_eq!(sd.events, crate::effect::EventMask::of(sd.handlers), "{id:?} self mask");
                }
                let Some(dex_hs) = dex_handlers(dex, *id) else { continue };
                let check = |h: &crate::effect::Handler, dex_hs: &'static [ps_dex::Handler]| {
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
                };
                for h in def.handlers {
                    check(h, dex_hs);
                }
                // A move's `self` effect has its own handlers in PS's data.
                if let EffectId::Move(m) = *id {
                    let self_hs = dex.move_def(m).self_effect().map(|s| s.handlers).unwrap_or(&[]);
                    for h in def.self_def.map_or(&[][..], |sd| sd.handlers) {
                        check(h, self_hs);
                    }
                }
            }
        }
    }

    /// The `kind:id` list [`ported`] holds for `dex`, derived from the dex and
    /// `registry` (allocates: test only).
    pub fn derive_ported(dex: &Dex, registry: &Registry) -> alloc::vec::Vec<alloc::string::String> {
        use alloc::format;
        let mut out = alloc::vec::Vec::new();
        for i in 0..dex.moves.len() {
            let m = MoveId(i as u16);
            let def = dex.move_def(m);
            let id = format!("{}", PsId(def.name.as_str()));
            if id == "struggle" || !def.placeholder_for.is_none() || !move_is_ported(dex, registry, m) {
                continue;
            }
            out.push(format!("move:{id}"));
        }
        for i in 0..dex.abilities.len() {
            let a = ps_dex::AbilityId(i as u16);
            if dex.gen < 3 {
                break;
            }
            // `noability` is PS's 'Past' entry, in the gen 3 dex only for
            // custom games (`dump-dex.js`): not an ability a team draws.
            if is_ported(dex, registry, EffectId::Ability(a)) && dex.ability_def(a).name.as_str() != "No Ability" {
                out.push(format!("ability:{}", PsId(dex.ability_def(a).name.as_str())));
            }
        }
        for i in 0..dex.items.len() {
            let it = ps_dex::ItemId(i as u16);
            if dex.gen < 3 {
                break;
            }
            if item_is_ported(dex, registry, it) {
                out.push(format!("item:{}", PsId(dex.item_def(it).name.as_str())));
            }
        }
        for s in dex.species.iter().filter(|s| s.num > 0) {
            out.push(format!("species:{}", PsId(s.name.as_str())));
        }
        out.sort();
        out
    }

    /// The `ported(gen)` lists are exactly what the dex and the registry
    /// allow ([`derive_ported`]).
    #[test]
    fn ported_lists_are_the_dex_derivation() {
        for (gen, dex) in [(1, &ps_dex::GEN1), (3, &ps_dex::GEN3)] {
            let derived = derive_ported(dex, registry_for(dex));
            let listed: alloc::vec::Vec<alloc::string::String> = ported(gen).iter().map(|s| alloc::string::String::from(*s)).collect();
            assert_eq!(listed, derived, "gen {gen}: ported list is stale; derived:\n{derived:#?}");
        }
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
