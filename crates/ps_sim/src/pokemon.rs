//! The item and ability machinery of `sim/pokemon.ts`: `eatItem`,
//! `useItem`, `takeItem`, `setItem`, `clearItem`, `setAbility`,
//! `clearAbility`, and `getWeight`. (`hasItem`, `hasAbility`,
//! `ignoringItem` and `ignoringAbility` live with the other state
//! accessors in `state.rs`.)
//!
//! Item and ability states are [`Pokemon::item_state`] and
//! [`Pokemon::ability_state`]. Where PS keeps the old state object for a
//! handler (`setItem`'s `End` on the item it replaces, `setAbility`'s `End`
//! on the old ability), the old state is buried ([`Battle::bury`]) so the
//! handler still finds it by serial; where PS clears the object in place
//! (`clearEffectState`), the serial is kept.
//!
//! PS's `source` and `sourceEffect` defaults (`this.battle.event?.target`,
//! `this.battle.effect`) come from the running event, as in `state.rs`.
//!
//! PS: sim/pokemon.ts eatItem, useItem, takeItem, setItem, clearItem,
//! setAbility, clearAbility, getWeight.

use ps_dex::{AbilityId, ItemId, MoveId};
use ps_protocol::{AbilityNum, Arg, ArgValue, ItemNum, Kw, KwFlags, MoveNum, Msg};

use crate::effect::{EffectId, EventId};
use crate::relay::Relay;
use crate::state::{Battle, Holder, MonRef, Staleness, StateRef, Trapped};

/// PS's `RESTORATIVE_BERRIES` (sim/pokemon.ts), by item name.
pub(crate) const RESTORATIVE_BERRIES: [&str; 9] =
    ["Leppa Berry", "Aguav Berry", "Enigma Berry", "Figy Berry", "Iapapa Berry", "Mago Berry", "Sitrus Berry", "Wiki Berry", "Oran Berry"];

impl Battle {
    /// An effect's `fullname` as a `[from]` argument (`item: Leftovers`,
    /// `ability: Trace`, `move: Skill Swap`).
    pub(crate) fn effect_fullname(&self, effect: EffectId) -> Arg {
        match effect {
            EffectId::Move(m) => Arg::new(ArgValue::MoveEffect(MoveNum(m.0))),
            EffectId::Item(i) => Arg::new(ArgValue::ItemEffect(ItemNum(i.0))),
            EffectId::Ability(a) => Arg::new(ArgValue::AbilityEffect(AbilityNum(a.0))),
            // A move's own condition (`Moves.x.condition`) is built with
            // the move's name and no `fullname`, so its fullname is the
            // bare name (`Ingrain`).
            EffectId::Condition(c) if self.condition_move(c).is_some() => {
                Arg::new(ArgValue::Move(MoveNum(self.condition_move(c).expect("checked").0)))
            }
            other => panic!("unported: the fullname of {other:?}"),
        }
    }

    /// `if (!source && this.battle.event?.target) source = event.target`
    /// (a Pokemon; a side or field target is no Pokemon to compare with).
    fn item_source(&self, source: Option<MonRef>) -> Option<MonRef> {
        source.or(self.event.target.mon())
    }

    /// `if (!sourceEffect && this.battle.effect) sourceEffect = this.battle.effect`.
    fn item_source_effect(&self, source_effect: EffectId) -> EffectId {
        if source_effect.is_none() {
            self.frame.effect
        } else {
            source_effect
        }
    }

    /// `sourceEffect?.effectType === 'Item' && this.item !== sourceEffect.id
    /// && source === this`: an item telling the holder to eat (use) it
    /// while holding another one.
    fn wrong_item_told(&self, m: MonRef, source: Option<MonRef>, source_effect: EffectId) -> bool {
        match source_effect {
            EffectId::Item(i) => self.mon(m).item != i && source == Some(m),
            _ => false,
        }
    }

    /// PS `side.foes()` (also `pokemon.foes()` and, in singles,
    /// `pokemon.adjacentFoes()`): the foe side's active Pokemon with HP.
    // PS: sim/side.ts foes, sim/pokemon.ts foes, adjacentFoes
    pub fn foes(&self, side: u8) -> ([MonRef; 1], usize) {
        self.allies(1 - side)
    }

    /// PS `side.randomFoe()`: `battle.sample(foes)`, one PRNG call when
    /// there is a foe.
    // PS: sim/side.ts randomFoe
    pub fn random_foe(&mut self, side: u8) -> Option<MonRef> {
        let (foes, n) = self.foes(side);
        if n == 0 {
            return None;
        }
        Some(*self.prng.sample(&foes[..n]))
    }

    /// PS `pokemon.setType(newType, enforce)` of one type. Stellar,
    /// Arceus and Silvally (gens 4+) and Terastallization are not in gen 3,
    /// so only the assignment and `knownType = true` are left; `addedType`
    /// and `apparentType` are not kept (no protocol line reads them).
    // PS: sim/pokemon.ts setType
    pub fn set_type(&mut self, m: MonRef, t: ps_dex::Type, _enforce: bool) -> bool {
        assert!(self.gen <= 3, "unported: setType's gen {} refusals", self.gen);
        let mon = self.mon_mut(m);
        mon.types = crate::relay::TypeList([t, ps_dex::Type::None]);
        mon.known_type = true;
        true
    }

    /// PS `pokemon.formeChange(speciesId, source, isPermanent, abilitySlot,
    /// message)` for a temporary forme change (Forecast): `setSpecies`, then
    /// the `-formechange` line, `[from] ability: NAME` when an ability did
    /// it. `message` is the line's flag (`[msg]`), written before `[from]`.
    // PS: sim/pokemon.ts formeChange
    pub fn forme_change(
        &mut self,
        m: MonRef,
        species: ps_dex::SpeciesId,
        source: EffectId,
        is_permanent: bool,
        message: KwFlags,
    ) -> bool {
        // The permanent branch (`detailschange`, Megas, Primals, the
        // ability slot) has no gen 3 caller.
        assert!(!is_permanent, "unported: a permanent formeChange");
        // `setSpecies` returns the species: no gen 1-3 `ModifySpecies`
        // handler can clear it.
        self.set_species(m, species, false);
        if self.gen <= 2 {
            return true;
        }
        let mon = self.ident(m);
        let num = ps_protocol::SpeciesNum(species.0);
        let kw = match source {
            EffectId::Ability(_) => {
                Kw { from: Some(self.effect_fullname(source)), of: None, flags: message | KwFlags::FLAGS_FIRST, repeat_miss: 0 }
            }
            _ => Kw::flags(message),
        };
        self.add(Msg::FormeChange { mon, species: num, kw });
        true
    }

    /// PS `pokemon.tryTrap(isHidden)`.
    // PS: sim/pokemon.ts tryTrap
    pub fn try_trap(&mut self, m: MonRef, is_hidden: bool) -> bool {
        let Some(trapped) = self.dex.condition_id("trapped") else { return false };
        if !self.run_status_immunity(m, trapped, false) {
            return false;
        }
        if self.mon(m).trapped.truthy() && is_hidden {
            return true;
        }
        self.mon_mut(m).trapped = if is_hidden { Trapped::Hidden } else { Trapped::Yes };
        true
    }

    /// PS `pokemon.hasMove(moveid)` (a typed Hidden Power is
    /// `hiddenpower`).
    // PS: sim/pokemon.ts hasMove
    pub fn has_move(&self, m: MonRef, mv: MoveId) -> bool {
        if mv.is_none() {
            return false;
        }
        let base = |id: MoveId| {
            let p = self.dex.move_def(id).placeholder_for;
            if p.is_none() { id } else { p }
        };
        let want = base(mv);
        self.mon(m).move_slots.iter().any(|s| !s.id.is_none() && base(s.id) == want)
    }

    /// The move an effect names when it prints `moveSlot.move`: the slot's
    /// own move, or `Hidden Power TYPE` (the `hiddenpowerTYPE` placeholder
    /// entry, whose name that is) for a slot `transformInto` copied.
    // PS: sim/pokemon.ts transformInto (moveName)
    pub fn slot_effect_move(&self, m: MonRef, slot: &crate::state::MoveSlot) -> MoveId {
        // Only `transformInto` renames a slot (`moveName = 'Hidden Power ' +
        // this.hpType`); Mimic and Sketch refuse a transformed user, so every
        // slot of a transformed Pokemon is one of its copies.
        if !self.mon(m).transformed || self.dex.move_def(slot.id).name.as_str() != "Hidden Power" {
            return slot.id;
        }
        const IDS: [&str; 17] = [
            "hiddenpowernormal", "hiddenpowerfire", "hiddenpowerwater", "hiddenpowerelectric", "hiddenpowergrass",
            "hiddenpowerice", "hiddenpowerfighting", "hiddenpowerpoison", "hiddenpowerground", "hiddenpowerflying",
            "hiddenpowerpsychic", "hiddenpowerbug", "hiddenpowerrock", "hiddenpowerghost", "hiddenpowerdragon",
            "hiddenpowerdark", "hiddenpowersteel",
        ];
        let ty = self.mon(m).hp_type.index() as usize;
        // Normal has no placeholder entry: `Hidden Power Normal` is not a name.
        IDS.get(ty).and_then(|id| self.dex.move_id(id)).unwrap_or(slot.id)
    }

    /// PS `pokemon.disableMove(moveid, isHidden, sourceEffect)`.
    /// `disabledSource` is not kept: only the request's error text reads
    /// it, and the request here says `disabled` alone.
    // PS: sim/pokemon.ts disableMove
    pub fn disable_move(&mut self, m: MonRef, mv: MoveId, is_hidden: bool, _source_effect: EffectId) {
        for slot in self.mon_mut(m).move_slots.iter_mut() {
            if slot.id == mv && slot.disabled != 1 {
                slot.disabled = if is_hidden { 2 } else { 1 };
            }
        }
    }

    /// PS `pokemon.eatItem(force, source, sourceEffect)`.
    // PS: sim/pokemon.ts eatItem
    pub fn eat_item(&mut self, m: MonRef, force: bool, source: Option<MonRef>, source_effect: EffectId) -> bool {
        let item = self.mon(m).item;
        if item.is_none() {
            return false;
        }
        let name = self.dex.item_def(item).name.as_str();
        let (hp, active) = (self.mon(m).hp, self.mon(m).is_active);
        if (hp == 0 && name != "Jaboca Berry" && name != "Rowap Berry") || !active {
            return false;
        }
        let source_effect = self.item_source_effect(source_effect);
        let source = self.item_source(source);
        if self.wrong_item_told(m, source, source_effect) {
            return false;
        }
        let relay = Some(Relay::Effect(EffectId::Item(item)));
        if !self.run_event(EventId::UseItem, Holder::Mon(m), None, EffectId::None, relay).truthy() {
            return false;
        }
        if !force && !self.run_event(EventId::TryEatItem, Holder::Mon(m), None, EffectId::None, relay).truthy() {
            return false;
        }
        let mon = self.ident(m);
        self.add(Msg::EndItem { mon, item: ItemNum(item.0), kw: Kw::flags(KwFlags::EAT) });

        let serial = self.mon(m).item_state.serial;
        // PS `pokemon.eatItem` returns false if `singleEvent('Eat', ...)`
        // answers false (a flavor berry's `onEat: false`): the `-enditem
        // [eat]` line is already printed above.
        if !self.single_event(EventId::Eat, EffectId::Item(item), StateRef::Item(m), serial, Holder::Mon(m), source, source_effect, None).truthy() {
            return false;
        }
        self.run_event(EventId::EatItem, Holder::Mon(m), source, source_effect, relay);

        if RESTORATIVE_BERRIES.contains(&name) {
            let mon = self.mon_mut(m);
            match mon.pending_staleness {
                Staleness::Internal => {
                    if mon.staleness != Staleness::External {
                        mon.staleness = Staleness::Internal;
                    }
                }
                Staleness::External => mon.staleness = Staleness::External,
                Staleness::None => {}
            }
            mon.pending_staleness = Staleness::None;
        }

        let mon = self.mon_mut(m);
        mon.last_item = item;
        mon.item = ItemId::NONE;
        Battle::clear_effect_state(&mut mon.item_state);
        mon.used_item_this_turn = true;
        mon.ate_berry = true;
        self.run_event(EventId::AfterUseItem, Holder::Mon(m), None, EffectId::None, relay);
        true
    }

    /// PS `pokemon.useItem(source, sourceEffect)`.
    // PS: sim/pokemon.ts useItem
    pub fn use_item(&mut self, m: MonRef, source: Option<MonRef>, source_effect: EffectId) -> bool {
        let item = self.mon(m).item;
        let is_gem = !item.is_none() && self.dex.item_def(item).is_gem;
        if (self.mon(m).hp == 0 && !is_gem) || !self.mon(m).is_active {
            return false;
        }
        if item.is_none() {
            return false;
        }
        let source_effect = self.item_source_effect(source_effect);
        let source = self.item_source(source);
        if self.wrong_item_told(m, source, source_effect) {
            return false;
        }
        let relay = Some(Relay::Effect(EffectId::Item(item)));
        if !self.run_event(EventId::UseItem, Holder::Mon(m), None, EffectId::None, relay).truthy() {
            return false;
        }
        let def = self.dex.item_def(item);
        // `case 'redcard'` and the gems' `[from] gem|[move] MOVE` are gen 5
        // items: the default line is the only one gens 1-3 reach.
        assert!(!is_gem && def.name.as_str() != "Red Card", "unported: the -enditem line of {}", def.name.as_str());
        let mon = self.ident(m);
        self.add(Msg::EndItem { mon, item: ItemNum(item.0), kw: Kw::NONE });
        if let Some(b) = def.boosts {
            let boosts = [b.atk, b.def, b.spa, b.spd, b.spe, b.accuracy, b.evasion];
            self.boost(boosts, m, source, EffectId::Item(item));
        }

        let serial = self.mon(m).item_state.serial;
        self.single_event(EventId::Use, EffectId::Item(item), StateRef::Item(m), serial, Holder::Mon(m), source, source_effect, None);

        let mon = self.mon_mut(m);
        mon.last_item = item;
        mon.item = ItemId::NONE;
        Battle::clear_effect_state(&mut mon.item_state);
        mon.used_item_this_turn = true;
        self.run_event(EventId::AfterUseItem, Holder::Mon(m), None, EffectId::None, relay);
        true
    }

    /// PS `pokemon.takeItem(source)`. PS returns the item taken, `false`
    /// when refused, or `undefined` with no item to take: here `Ok(item)`,
    /// `Err(Relay::FAIL)` and `Err(Relay::Null)`.
    // PS: sim/pokemon.ts takeItem
    pub fn take_item(&mut self, m: MonRef, source: Option<MonRef>) -> Result<ItemId, Relay> {
        let source = source.unwrap_or(m);
        if self.gen == 4 && (self.mon(m).item_knocked_off || self.mon(source).item_knocked_off) {
            return Err(Relay::FAIL);
        }
        let item = self.mon(m).item;
        if item.is_none() {
            return Err(Relay::Null);
        }
        let relay = Some(Relay::Effect(EffectId::Item(item)));
        if !self.run_event(EventId::TakeItem, Holder::Mon(m), Some(source), EffectId::None, relay).truthy() {
            return Err(Relay::FAIL);
        }
        let mon = self.mon_mut(m);
        mon.item = ItemId::NONE;
        // `oldItemState` is the same object, cleared before `End` sees it.
        Battle::clear_effect_state(&mut mon.item_state);
        mon.pending_staleness = Staleness::None;
        let serial = mon.item_state.serial;
        self.single_event(EventId::End, EffectId::Item(item), StateRef::Item(m), serial, Holder::Mon(m), None, EffectId::None, None);
        self.run_event(EventId::AfterTakeItem, Holder::Mon(m), None, EffectId::None, relay);
        Ok(item)
    }

    /// PS `pokemon.setItem(item, source, effect)`; `ItemId::NONE` is `''`.
    // PS: sim/pokemon.ts setItem
    pub fn set_item(&mut self, m: MonRef, item: ItemId, source: Option<MonRef>, effect: EffectId) -> bool {
        if self.mon(m).hp == 0 || !self.mon(m).is_active {
            return false;
        }
        // `RESTORATIVE_BERRIES.has('leppaberry')` is always true.
        let effect_name = match self.frame.effect {
            EffectId::Move(mv) => self.dex.move_def(mv).name.as_str(),
            _ => "",
        };
        let inflicted = effect_name == "Trick" || effect_name == "Switcheroo";
        // `source.isAlly(this)`: the same side, in singles.
        let external = inflicted && source.is_some_and(|s| s.side != m.side);
        self.mon_mut(m).pending_staleness = if external { Staleness::External } else { Staleness::Internal };

        let old_item = self.mon(m).item;
        let old_state = self.mon(m).item_state;
        let st = self.init_effect_state(!item.is_none(), Holder::Mon(m));
        let mon = self.mon_mut(m);
        mon.item = item;
        mon.item_state = st;
        self.bury(old_state);
        // `if (oldItem.exists)`.
        if !old_item.is_none() {
            self.single_event(EventId::End, EffectId::Item(old_item), StateRef::Item(m), old_state.serial, Holder::Mon(m), None, EffectId::None, None);
        }
        if !item.is_none() {
            self.single_event(EventId::Start, EffectId::Item(item), StateRef::Item(m), st.serial, Holder::Mon(m), source, effect, None);
        }
        true
    }

    /// PS `pokemon.clearItem()`: `setItem('')`.
    // PS: sim/pokemon.ts clearItem
    pub fn clear_item(&mut self, m: MonRef) -> bool {
        self.set_item(m, ItemId::NONE, None, EffectId::None)
    }

    /// PS `pokemon.setAbility(ability, source, sourceEffect,
    /// isFromFormeChange, isTransform)`. Returns PS's return value: the old
    /// ability's id ([`Relay::Effect`], `Effect(None)` for `''`), `false`,
    /// or what `SetAbility` answered when it is falsy.
    // PS: sim/pokemon.ts setAbility
    pub fn set_ability(
        &mut self,
        m: MonRef,
        ability: AbilityId,
        source: Option<MonRef>,
        source_effect: EffectId,
        is_from_forme_change: bool,
        is_transform: bool,
    ) -> Relay {
        if self.mon(m).hp == 0 {
            return Relay::FAIL;
        }
        let source_effect = if source_effect.is_none() { self.frame.effect } else { source_effect };
        let old_ability = self.mon(m).ability;
        let cantsuppress = |b: &Battle, a: AbilityId| !a.is_none() && b.dex.ability_def(a).flags.has("cantsuppress");
        if !is_from_forme_change && (cantsuppress(self, ability) || cantsuppress(self, old_ability)) {
            return Relay::FAIL;
        }
        if !is_from_forme_change && !is_transform {
            let relay = Some(Relay::Effect(if ability.is_none() { EffectId::None } else { EffectId::Ability(ability) }));
            let r = self.run_event(EventId::SetAbility, Holder::Mon(m), source, source_effect, relay);
            if !r.truthy() {
                return r;
            }
        }
        let old_effect = if old_ability.is_none() { EffectId::None } else { EffectId::Ability(old_ability) };
        let old_state = self.mon(m).ability_state;
        self.single_event(EventId::End, old_effect, StateRef::Ability(m), old_state.serial, Holder::Mon(m), source, EffectId::None, None);
        let st = self.init_effect_state(!ability.is_none(), Holder::Mon(m));
        let mon = self.mon_mut(m);
        mon.ability = ability;
        mon.ability_state = st;
        self.bury(old_state);
        if !source_effect.is_none() && !is_from_forme_change && !is_transform {
            // `case 'mummy': case 'lingeringaroma'` are gen 5 and 9.
            let mon = self.ident(m);
            let from = self.effect_fullname(source_effect);
            let kw = match source {
                Some(s) => Kw::from_of(from, self.ident(s)),
                None => Kw::from(from),
            };
            let old = (!old_ability.is_none()).then_some(AbilityNum(old_ability.0));
            self.add(Msg::Ability { mon, ability: AbilityNum(ability.0), old, boost: false, kw, split: None });
        }
        if !ability.is_none() && self.gen > 3 && (!is_transform || old_ability != ability || self.gen <= 4) {
            self.single_event(EventId::Start, EffectId::Ability(ability), StateRef::Ability(m), st.serial, Holder::Mon(m), source, EffectId::None, None);
        }
        Relay::Effect(old_effect)
    }

    /// PS `pokemon.clearAbility()`: `setAbility('')`.
    // PS: sim/pokemon.ts clearAbility
    pub fn clear_ability(&mut self, m: MonRef) -> Relay {
        self.set_ability(m, AbilityId::NONE, None, EffectId::None, false, false)
    }

    /// PS `pokemon.getWeight()`: `ModifyWeight` over `weighthg`, at least 1.
    // PS: sim/pokemon.ts getWeight
    pub fn get_weight(&mut self, m: MonRef) -> i32 {
        let w = i32::from(self.mon(m).weighthg);
        let r = self.run_event(EventId::ModifyWeight, Holder::Mon(m), None, EffectId::None, Some(Relay::Int(w)));
        match r {
            Relay::Int(n) => n.max(1),
            other => panic!("ModifyWeight answered {other:?}"),
        }
    }
}

#[cfg(test)]
#[path = "pokemon_tests.rs"]
mod tests;
