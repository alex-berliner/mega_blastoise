//! The event pipeline (design 5.3): `speedSort`, handler gathering, and the
//! dispatch family `runEvent`, `priorityEvent`, `singleEvent`, `eachEvent`
//! and `fieldEvent`, each following PS's control flow line by line.
//!
//! Gathering writes [`Listener`]s into a stack array of [`LISTENER_CAP`]
//! (PS's `EventListener`s), in PS's order: target, its allies and foes with
//! their prefixes, the source's `onSource`, the sides, the field, the
//! battle. A list is then sorted with PS's comparator through the same
//! selection sort, whose exact ties cost PRNG calls (`prng.shuffle` over
//! each run of equal entries), so the battle's random stream stays in step
//! with PS's.
//!
//! PS: sim/battle.ts `comparePriority`, `compareRedirectOrder`,
//! `compareLeftToRightOrder`, `speedSort`, `eachEvent`, `fieldEvent`,
//! `singleEvent`, `runEvent`, `priorityEvent`, `resolvePriority`,
//! `getCallback`, `findEventHandlers`, `findPokemonEventHandlers`,
//! `findBattleEventHandlers`, `findFieldEventHandlers`,
//! `findSideEventHandlers`.

use core::cmp::Ordering;

use crate::effect::{Callback, EffectId, EffectType, EventArgs, EventId, Handler, Prefix};
use crate::prng::Prng;
use crate::relay::Relay;
use crate::state::{Battle, EffectFrame, EffectState, EventFrame, Holder, MonRef, StateRef, MAX_EVENT_DEPTH};

/// Most listeners one event gathers (design 5.3). Overflow panics: a
/// silently dropped handler would be a divergence.
pub const LISTENER_CAP: usize = 32;
/// Most targets of a spread `runEvent` (singles: one foe, at most two).
pub const SPREAD_CAP: usize = 4;

/// PS's `order || 4294967296`: an absent (or 0) order sorts last.
const ORDER_LAST: i64 = 4_294_967_296;

/// The fields PS's comparators read. `0` is "absent" for each, as PS's
/// `|| 0` reads them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SortKey {
    /// `order`, 0 for `false` / `undefined` (sorted as 2^32).
    pub order: i64,
    pub priority: i32,
    /// `speed`. Only compared, never shown, so listeners store it doubled:
    /// `resolvePriority` subtracts half a point on `SwitchIn` events
    /// (`index / (activePerHalf * 2)` in singles), which stays integral.
    pub speed: i32,
    pub sub_order: i32,
    pub effect_order: i64,
}

fn order_or_last(o: i64) -> i64 {
    if o == 0 {
        ORDER_LAST
    } else {
        o
    }
}

/// PS `comparePriority`: order low to high, priority high to low, speed
/// high to low, subOrder low to high, effectOrder low to high.
/// `Less` means `a` comes first (PS's negative return).
pub fn compare_priority(a: &SortKey, b: &SortKey) -> Ordering {
    order_or_last(a.order)
        .cmp(&order_or_last(b.order))
        .then(b.priority.cmp(&a.priority))
        .then(b.speed.cmp(&a.speed))
        .then(a.sub_order.cmp(&b.sub_order))
        .then(a.effect_order.cmp(&b.effect_order))
}

/// PS `speedSort(list, comparator)`: a selection sort that moves each run
/// of entries tied for first to the front and shuffles it with the PRNG.
// PS: sim/battle.ts speedSort
pub fn speed_sort<T>(prng: &mut Prng, list: &mut [T], mut cmp: impl FnMut(&T, &T) -> Ordering) {
    if list.len() < 2 {
        return;
    }
    let mut sorted = 0;
    let mut next = [0usize; LISTENER_CAP];
    while sorted + 1 < list.len() {
        next[0] = sorted;
        let mut n = 1;
        for i in sorted + 1..list.len() {
            match cmp(&list[next[0]], &list[i]) {
                Ordering::Less => continue,
                Ordering::Greater => {
                    next[0] = i;
                    n = 1;
                }
                Ordering::Equal => {
                    assert!(n < LISTENER_CAP, "speedSort tie run over {LISTENER_CAP}");
                    next[n] = i;
                    n += 1;
                }
            }
        }
        for (i, &index) in next[..n].iter().enumerate() {
            if index != sorted + i {
                list.swap(sorted + i, index);
            }
        }
        if n > 1 {
            prng.shuffle(list, sorted, sorted + n);
        }
        sorted += n;
    }
}

/// A stable insertion sort, as JS's `Array.prototype.sort` is stable.
fn stable_sort<T: Copy>(list: &mut [T], mut cmp: impl FnMut(&T, &T) -> Ordering) {
    for i in 1..list.len() {
        let mut j = i;
        while j > 0 && cmp(&list[j - 1], &list[j]) == Ordering::Greater {
            list.swap(j - 1, j);
            j -= 1;
        }
    }
}

/// What `fieldEvent` calls when a residual duration runs out (PS's
/// `handler.end`), bound to its holder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// `end: null`.
    None,
    /// Species' `end() {}`.
    Noop,
    ClearStatus(MonRef),
    RemoveVolatile(MonRef),
    ClearAbility(MonRef),
    ClearItem(MonRef),
    RemoveSlotCondition(u8, u8),
    RemoveSideCondition(u8),
    RemovePseudoWeather,
    ClearWeather,
}

/// PS's `EventListener`.
#[derive(Clone, Copy, Debug)]
pub struct Listener {
    pub effect: EffectId,
    /// The matched handler; `None` for a listener gathered only for its
    /// `duration` (`getKey`).
    pub handler: Option<&'static Handler>,
    /// The event the handler key answers (`SideResidual` for
    /// `onSideResidual`).
    pub event: EventId,
    pub state: StateRef,
    /// The state's identity at gathering.
    pub serial: u16,
    pub end: End,
    pub holder: Holder,
    pub key: SortKey,
    /// Spread events: the target this listener was gathered for, and its
    /// index in the target list.
    pub target: Option<MonRef>,
    pub index: Option<u8>,
}

impl Listener {
    const EMPTY: Listener = Listener {
        effect: EffectId::None,
        handler: None,
        event: EventId::Residual,
        state: StateRef::Scratch,
        serial: 0,
        end: End::None,
        holder: Holder::None,
        key: SortKey { order: 0, priority: 0, speed: 0, sub_order: 0, effect_order: 0 },
        target: None,
        index: None,
    };
}

/// A fixed-capacity listener list.
pub struct Listeners {
    len: usize,
    items: [Listener; LISTENER_CAP],
}

impl Default for Listeners {
    fn default() -> Self {
        Self::new()
    }
}

impl Listeners {
    pub const fn new() -> Listeners {
        Listeners { len: 0, items: [Listener::EMPTY; LISTENER_CAP] }
    }

    pub fn push(&mut self, l: Listener) {
        assert!(self.len < LISTENER_CAP, "more than {LISTENER_CAP} event listeners");
        self.items[self.len] = l;
        self.len += 1;
    }

    /// `handlers.unshift(l)`.
    pub fn push_front(&mut self, l: Listener) {
        assert!(self.len < LISTENER_CAP, "more than {LISTENER_CAP} event listeners");
        self.items.copy_within(0..self.len, 1);
        self.items[0] = l;
        self.len += 1;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[Listener] {
        &self.items[..self.len]
    }

    pub fn as_mut_slice(&mut self) -> &mut [Listener] {
        &mut self.items[..self.len]
    }
}

/// The per-target results of a spread `runEvent` (PS returns
/// `targetRelayVars`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpreadRelays {
    pub len: usize,
    pub relays: [Relay; SPREAD_CAP],
}

impl SpreadRelays {
    pub fn as_slice(&self) -> &[Relay] {
        &self.relays[..self.len]
    }
}

/// Which `findXEventHandlers` options are in play: PS's `getKey` (only
/// `'duration'`, for Residual) and `customHolder`.
#[derive(Clone, Copy)]
struct Find {
    prefix: Prefix,
    event: EventId,
    duration: bool,
    custom_holder: Option<MonRef>,
}

impl Find {
    fn on(prefix: Prefix, event: EventId) -> Find {
        Find { prefix, event, duration: false, custom_holder: None }
    }
}

/// Events run through `eachEvent` (and `Weather` / `TerrainChange`), which
/// gather no prefixed handlers.
fn prefixed_handlers(event: EventId) -> bool {
    !matches!(
        event,
        EventId::BeforeTurn | EventId::Update | EventId::Weather | EventId::WeatherChange | EventId::TerrainChange
    )
}

impl Battle {
    /// `speedSort` with `comparePriority` over listeners.
    pub fn speed_sort_listeners(&mut self, list: &mut [Listener]) {
        speed_sort(&mut self.prng, list, |a, b| compare_priority(&a.key, &b.key));
    }

    // ---- gathering --------------------------------------------------------

    /// PS `getCallback(target, effect, callbackName)`. Abilities and items
    /// start at different times during `SwitchIn`, so on a Pokemon one
    /// without `onSwitchIn` (or `onAnySwitchIn`) answers it with `onStart`.
    /// In gen 3 an ability with `onAfterEntryHazard` answers `SwitchIn` with
    /// it on turn 0 and not at all later (`switchIn` runs it then).
    // PS: sim/battle.ts getCallback
    fn get_callback(&self, target: Holder, effect: EffectId, prefix: Prefix, event: EventId) -> Option<&'static Handler> {
        let mut key = (prefix, event);
        if matches!(target, Holder::Mon(_)) && prefix == Prefix::None && event == EventId::SwitchIn {
            // No innate abilities or items (`ability:` / `item:` statuses)
            // in gens 1-3.
            let etype = self.effect_type(effect);
            if !self.has_dex_handler(effect, Prefix::None, EventId::SwitchIn)
                && !self.has_dex_handler(effect, Prefix::Any, EventId::SwitchIn)
                && matches!(etype, EffectType::Ability | EffectType::Item)
            {
                key = (Prefix::None, EventId::Start);
            }
            if self.gen == 3 && etype == EffectType::Ability && self.has_dex_handler(effect, Prefix::None, EventId::AfterEntryHazard) {
                if self.turn != 0 {
                    return None;
                }
                key = (Prefix::None, EventId::AfterEntryHazard);
            }
        }
        let callback = self.registry.handler(effect, key.0, key.1);
        if callback.is_none() {
            self.require_handler_ported(effect, key.0, key.1);
        }
        callback
    }

    /// Whether the dex lists a `prefix` + `event` handler for `effect`
    /// (PS's `effect[callbackName] !== undefined`), ported or not.
    fn has_dex_handler(&self, effect: EffectId, prefix: Prefix, event: EventId) -> bool {
        if self.registry.handler(effect, prefix, event).is_some() {
            return true;
        }
        crate::mods::dex_handlers(self.dex, effect)
            .is_some_and(|hs| hs.iter().any(|h| crate::effect::parse_handler_key(h.name()) == Some((prefix, event))))
    }

    /// The `unported` stop for event dispatch: PS would call `effect`'s
    /// `prefix` + `event` handler here, and the registry does not carry it
    /// (an item or ability with handlers, or any other effect whose port
    /// has not landed). Without this the handler would be skipped silently.
    pub(crate) fn require_handler_ported(&self, effect: EffectId, prefix: Prefix, event: EventId) {
        #[cfg(any(test, feature = "test-override"))]
        if self.lax_handlers {
            return;
        }
        let Some(hs) = crate::mods::dex_handlers(self.dex, effect) else { return };
        if hs.iter().any(|h| crate::effect::parse_handler_key(h.name()) == Some((prefix, event))) {
            self.require_ported(effect);
            // Ported means every dex handler is registered, so a miss here
            // is a registry bug.
            panic!("unported: {effect:?} {prefix:?} {event:?} is in the dex but not the registry");
        }
    }

    /// The `target` stored on the state a listener points at.
    fn state_target(&self, r: StateRef, serial: u16) -> Holder {
        self.state_at(r, serial).map_or(Holder::None, |s| s.target)
    }

    /// PS `resolvePriority(handler, callbackName)`.
    // PS: sim/battle.ts resolvePriority
    fn resolve_priority(&self, l: &mut Listener, prefix: Prefix) {
        let (mut order, mut priority, mut sub_order) = match l.handler {
            Some(h) => (h.order, h.priority, h.sub_order),
            None => (0, 0, 0),
        };
        // A condition gathered for its `duration` alone still has the
        // residual's keys read for it (`effect.onResidualOrder` with no
        // `onResidual`: gen 3 Taunt, Yawn, Wish, the screens).
        if let (None, EffectId::Condition(c)) = (l.handler, l.effect) {
            if let Some(r) = self.dex.condition_def(c).residual_order {
                let event = match r.kind {
                    ps_dex::ResidualKind::Residual => EventId::Residual,
                    ps_dex::ResidualKind::SideResidual => EventId::SideResidual,
                    ps_dex::ResidualKind::FieldResidual => EventId::FieldResidual,
                };
                if l.event == event && prefix == Prefix::None {
                    (order, priority, sub_order) = (r.keys.order, r.keys.priority, r.keys.sub_order);
                }
            }
        }
        // `effect[`${callbackName}Priority`]` reads the keys of the event
        // being run: an ability or item answering `onSwitchIn` with its
        // `onStart` (or gen 3's `onAfterEntryHazard`, `getCallback`) is
        // ordered by its `onSwitchIn` keys, not by those of the handler.
        if l.event == EventId::SwitchIn && prefix == Prefix::None {
            let keys = match l.effect {
                EffectId::Ability(a) => Some(self.dex.ability_def(a).switch_in_order),
                EffectId::Item(i) => Some(self.dex.item_def(i).switch_in_order),
                _ => None,
            };
            if let Some(k) = keys {
                (order, priority, sub_order) = (k.order, k.priority, k.sub_order);
            }
        }
        l.key.order = i64::from(order);
        l.key.priority = i32::from(priority);
        l.key.sub_order = i32::from(sub_order);
        let etype = self.effect_type(l.effect);
        if l.key.sub_order == 0 {
            l.key.sub_order = match etype {
                EffectType::Condition => 2,
                EffectType::Weather | EffectType::Format | EffectType::Rule => 5,
                EffectType::Ability => 7,
                EffectType::Item => 8,
                _ => 0,
            };
            if etype == EffectType::Condition {
                match self.state_target(l.state, l.serial) {
                    Holder::Side(_) => {
                        l.key.sub_order = if matches!(l.state, StateRef::SlotCondition(..)) { 3 } else { 4 };
                    }
                    Holder::Field => l.key.sub_order = 5,
                    _ => {}
                }
            } else if let EffectId::Ability(a) = l.effect {
                match self.dex.ability_def(a).name.as_str() {
                    "Poison Touch" | "Perish Body" => l.key.sub_order = 6,
                    "Stall" => l.key.sub_order = 9,
                    _ => {}
                }
            }
        }
        let switch_in = l.event == EventId::SwitchIn;
        if switch_in || l.event == EventId::RedirectTarget {
            l.key.effect_order = self.state_at(l.state, l.serial).map_or(0, |s| i64::from(s.effect_order));
        }
        if let Holder::Mon(m) = l.holder {
            l.key.speed = 2 * self.mon(m).speed;
            if let EffectId::Ability(a) = l.effect {
                // Magic Bounce's `onAllyTryHitSide` reads the unmodified
                // speed stat; Magic Bounce is a gen 5 ability.
                assert!(
                    !(self.dex.ability_def(a).name.as_str() == "Magic Bounce"
                        && prefix == Prefix::Ally
                        && l.event == EventId::TryHitSide),
                    "Magic Bounce speed: not ported (no such ability in gens 1-3)"
                );
            }
            if switch_in {
                let pos = self.field_position_value(m);
                let index = self.speed_order.iter().position(|&p| p == pos).map_or(-1, |i| i as i32);
                // speed - index / (activePerHalf * 2), doubled.
                l.key.speed -= index;
            }
        }
    }

    fn push_listener(
        &self,
        out: &mut Listeners,
        f: Find,
        effect: EffectId,
        handler: Option<&'static Handler>,
        state: StateRef,
        end: End,
        holder: Holder,
    ) {
        let serial = self.serial_at(state);
        let mut l = Listener { effect, handler, event: f.event, state, serial, end, holder, ..Listener::EMPTY };
        self.resolve_priority(&mut l, f.prefix);
        out.push(l);
    }

    /// PS `findPokemonEventHandlers(pokemon, callbackName, getKey)`.
    // PS: sim/battle.ts findPokemonEventHandlers
    fn find_pokemon_handlers(&self, m: MonRef, f: Find, out: &mut Listeners) {
        let holder = Holder::Mon(m);
        let mon = self.mon(m);

        let status = mon.status_effect();
        let cb = self.get_callback(holder, status, f.prefix, f.event);
        if cb.is_some() || (f.duration && mon.status_state.duration != 0) {
            self.push_listener(out, f, status, cb, StateRef::Status(m), End::ClearStatus(m), holder);
        }
        for (id, st) in mon.volatiles.iter() {
            // A move-keyed volatile is a condition PS made from a bare move
            // id (`Battle::move_volatile_key`): no handlers.
            let cb = if matches!(id, EffectId::Move(_)) { None } else { self.get_callback(holder, id, f.prefix, f.event) };
            if cb.is_some() || (f.duration && st.duration != 0) {
                self.push_listener(out, f, id, cb, StateRef::Volatile(m, id), End::RemoveVolatile(m), holder);
            }
        }
        let ability = mon.ability_effect();
        let cb = self.get_callback(holder, ability, f.prefix, f.event);
        if cb.is_some() || (f.duration && mon.ability_state.duration != 0) {
            self.push_listener(out, f, ability, cb, StateRef::Ability(m), End::ClearAbility(m), holder);
        }
        let item = mon.item_effect();
        let cb = self.get_callback(holder, item, f.prefix, f.event);
        if cb.is_some() || (f.duration && mon.item_state.duration != 0) {
            self.push_listener(out, f, item, cb, StateRef::Item(m), End::ClearItem(m), holder);
        }
        let species = if mon.base_species.is_none() { EffectId::None } else { EffectId::Species(mon.base_species) };
        let cb = self.get_callback(holder, species, f.prefix, f.event);
        if cb.is_some() {
            self.push_listener(out, f, species, cb, StateRef::Species(m), End::Noop, holder);
        }
        let side = &self.sides[m.side as usize];
        let pos = mon.position;
        // A benched Pokemon's `position` is its team index, past the active
        // slots: `for (const id in side.slotConditions[pokemon.position])`
        // iterates `undefined`, which is nothing (gen 3 Yawn's `setStatus`
        // with a source that has switched out asks it for `onSource`; a future move's benched user as the hit's source).
        let Some(slot_conditions) = side.slot_conditions.get(pos as usize) else { return };
        for (id, st) in slot_conditions.iter() {
            let cb = self.get_callback(holder, id, f.prefix, f.event);
            if cb.is_some() || (f.duration && st.duration != 0) {
                self.push_listener(
                    out,
                    f,
                    id,
                    cb,
                    StateRef::SlotCondition(m.side, pos, id),
                    End::RemoveSlotCondition(m.side, pos),
                    holder,
                );
            }
        }
    }

    /// PS `findBattleEventHandlers(callbackName, getKey, customHolder)`.
    /// The formats here attach no handlers with `onEvent` (`this.events`).
    // PS: sim/battle.ts findBattleEventHandlers
    fn find_battle_handlers(&self, f: Find, out: &mut Listeners) {
        let holder = f.custom_holder.map_or(Holder::Battle, Holder::Mon);
        let cb = self.get_callback(Holder::Battle, self.format, f.prefix, f.event);
        if cb.is_some() || (f.duration && self.format_data.duration != 0) {
            self.push_listener(out, f, self.format, cb, StateRef::FormatData, End::None, holder);
        }
    }

    /// PS `findFieldEventHandlers(field, callbackName, getKey, customHolder)`.
    /// No terrain in gens 1-3.
    // PS: sim/battle.ts findFieldEventHandlers
    fn find_field_handlers(&self, f: Find, out: &mut Listeners) {
        let holder = f.custom_holder.map_or(Holder::Field, Holder::Mon);
        let custom = f.custom_holder.is_some();
        for (id, st) in self.field.pseudo_weather.iter() {
            let cb = self.get_callback(Holder::Field, id, f.prefix, f.event);
            if cb.is_some() || (f.duration && st.duration != 0) {
                let end = if custom { End::None } else { End::RemovePseudoWeather };
                self.push_listener(out, f, id, cb, StateRef::PseudoWeather(id), end, holder);
            }
        }
        let weather = self.field.weather_effect();
        let cb = self.get_callback(Holder::Field, weather, f.prefix, f.event);
        if cb.is_some() || (f.duration && self.field.weather_state.duration != 0) {
            let end = if custom { End::None } else { End::ClearWeather };
            self.push_listener(out, f, weather, cb, StateRef::Weather, end, holder);
        }
    }

    /// PS `findSideEventHandlers(side, callbackName, getKey, customHolder)`.
    // PS: sim/battle.ts findSideEventHandlers
    fn find_side_handlers(&self, side: u8, f: Find, out: &mut Listeners) {
        let holder = f.custom_holder.map_or(Holder::Side(side), Holder::Mon);
        for (id, st) in self.sides[side as usize].side_conditions.iter() {
            let cb = self.get_callback(Holder::Side(side), id, f.prefix, f.event);
            if cb.is_some() || (f.duration && st.duration != 0) {
                let end = if f.custom_holder.is_some() { End::None } else { End::RemoveSideCondition(side) };
                self.push_listener(out, f, id, cb, StateRef::SideCondition(side, id), end, holder);
            }
        }
    }

    /// PS `findEventHandlers(target, eventName, source)` for one target.
    // PS: sim/battle.ts findEventHandlers
    pub fn find_event_handlers(&self, target: Holder, event: EventId, source: Option<MonRef>, out: &mut Listeners) {
        let should_bubble_down = matches!(target, Holder::Side(_));
        let prefixed = prefixed_handlers(event);
        let mut target = target;
        if let Holder::Mon(m) = target {
            let source_active = source.is_some_and(|s| self.mon(s).is_active);
            if self.mon(m).is_active || source_active {
                self.find_pokemon_handlers(m, Find::on(Prefix::None, event), out);
                if prefixed {
                    let (allies, n) = self.allies(m.side);
                    for &ally in &allies[..n] {
                        self.find_pokemon_handlers(ally, Find::on(Prefix::Ally, event), out);
                        self.find_pokemon_handlers(ally, Find::on(Prefix::Any, event), out);
                    }
                    let (foes, n) = self.allies(1 - m.side);
                    for &foe in &foes[..n] {
                        self.find_pokemon_handlers(foe, Find::on(Prefix::Foe, event), out);
                        self.find_pokemon_handlers(foe, Find::on(Prefix::Any, event), out);
                    }
                }
                target = Holder::Side(m.side);
            }
        }
        if let Some(s) = source {
            if prefixed {
                self.find_pokemon_handlers(s, Find::on(Prefix::Source, event), out);
            }
        }
        if let Holder::Side(t) = target {
            for side in 0..2u8 {
                if should_bubble_down {
                    for pos in 0..self.sides[side as usize].active.len() as u8 {
                        let Some(active) = self.active_at(side, pos) else { continue };
                        if side == t {
                            self.find_pokemon_handlers(active, Find::on(Prefix::None, event), out);
                        } else if prefixed {
                            self.find_pokemon_handlers(active, Find::on(Prefix::Foe, event), out);
                        }
                        if prefixed {
                            self.find_pokemon_handlers(active, Find::on(Prefix::Any, event), out);
                        }
                    }
                }
                // `side.n < 2 || !side.allySide`: always, in singles.
                if side == t {
                    self.find_side_handlers(side, Find::on(Prefix::None, event), out);
                } else if prefixed {
                    self.find_side_handlers(side, Find::on(Prefix::Foe, event), out);
                }
                if prefixed {
                    self.find_side_handlers(side, Find::on(Prefix::Any, event), out);
                }
            }
        }
        self.find_field_handlers(Find::on(Prefix::None, event), out);
        self.find_battle_handlers(Find::on(Prefix::None, event), out);
    }

    // ---- dispatch ---------------------------------------------------------

    fn check_depth(&self, event: EventId) {
        // PS adds "STACK LIMIT EXCEEDED" lines, then throws.
        assert!(
            (self.event_depth as usize) < MAX_EVENT_DEPTH,
            "Stack overflow: event {event:?} at depth {} (parent {:?})",
            self.event_depth,
            self.event.id
        );
    }

    /// Feature `hit-count`: counts one call of `effect`'s handler `h`
    /// (`hits.rs`). The call sites are `cfg`'d out with the feature, so a
    /// build without it (the firmware) is unchanged.
    #[cfg(feature = "hit-count")]
    pub(crate) fn count_hit(&mut self, effect: EffectId, h: &Handler) {
        self.hits.count(effect, h.prefix, h.event);
    }

    /// Runs one handler with `this.effect` / `this.effectState` set as
    /// `runEvent` does (`this.effectState.target = effectHolder`).
    fn call_listener(&mut self, l: &Listener, args: &EventArgs) -> Option<Relay> {
        let h = l.handler?;
        #[cfg(feature = "hit-count")]
        self.count_hit(l.effect, h);
        match h.callback {
            Callback::Const(v) => Some(v),
            Callback::Fn(f) => {
                let parent = self.frame;
                if matches!(l.state, StateRef::Scratch | StateRef::Species(_)) {
                    // `handler.state || this.initEffectState({})`
                    self.scratch[self.event_depth as usize] = EffectState::EMPTY;
                }
                self.frame = EffectFrame { effect: l.effect, state: l.state, serial: l.serial };
                self.effect_state().target = l.holder;
                let r = f(self, args);
                self.frame = parent;
                r
            }
        }
    }

    /// The per-handler effect checks `runEvent` makes; true to skip.
    fn run_event_skips(&mut self, event: EventId, l: &Listener) -> bool {
        let etype = self.effect_type(l.effect);
        let holder_mon = l.holder.mon();
        if etype == EffectType::Status {
            if let Some(m) = holder_mon {
                if self.mon(m).status_effect() != l.effect {
                    return true;
                }
            }
        }
        if let EffectId::Ability(a) = l.effect {
            // The `!effect.num` branch after this one is dead in PS: a
            // breakable ability always `continue`s here first.
            if self.dex.ability_def(a).flags.has("breakable") && self.suppressing_ability(holder_mon) {
                // Its `|debug|` line: Mold Breaker is gen 4.
                panic!("unported: {event:?} handler suppressed by Mold Breaker");
            }
        }
        if !matches!(event, EventId::Start | EventId::SwitchIn | EventId::TakeItem)
            && etype == EffectType::Item
            && holder_mon.is_some_and(|m| self.ignoring_item(m))
        {
            // Embargo, Klutz and Magic Room are gen 4 and later.
            panic!("unported: {event:?} handler suppressed by Embargo, Klutz or Magic Room");
        } else if event != EventId::End
            && etype == EffectType::Ability
            && holder_mon.is_some_and(|m| self.ignoring_ability(m))
        {
            // Gastro Acid and Neutralizing Gas are gen 4 and later, and no
            // gen 1-3 ability has the `notransform` flag.
            panic!("unported: {event:?} handler suppressed by Gastro Acid or Neutralizing Gas");
        }
        if (etype == EffectType::Weather || event == EventId::Weather)
            && event != EventId::Residual
            && event != EventId::End
            && self.suppressing_weather()
        {
            self.debug_suppressed_by_air_lock(event);
            return true;
        }
        false
    }

    /// `this.debug(eventid + ' handler suppressed by Air Lock')`.
    fn debug_suppressed_by_air_lock(&mut self, event: EventId) {
        use ps_protocol::{Text, TextArg, TextId, Word};
        let word = match event {
            EventId::Immunity => Word::EventImmunity,
            EventId::Weather => Word::EventWeather,
            EventId::WeatherModifyDamage => Word::EventWeatherModifyDamage,
            other => panic!("unported: the Air Lock debug line of {other:?}"),
        };
        self.debug(Text::with1(TextId::HandlerSuppressedByAirLock, TextArg::word(word)));
    }

    /// PS `runEvent(eventid, target, source, sourceEffect, relayVar)`.
    /// `target` `Holder::None` is the battle (`if (!target) target = this`);
    /// a `relay` of `None` or `Null` is "no relay variable".
    pub fn run_event(
        &mut self,
        event: EventId,
        target: Holder,
        source: Option<MonRef>,
        effect: EffectId,
        relay: Option<Relay>,
    ) -> Relay {
        self.run_event_full(event, target, source, effect, relay, false, false)
    }

    /// PS `priorityEvent`: `runEvent` with `fastExit`, which stops at the
    /// first handler that returns anything.
    // PS: sim/battle.ts priorityEvent
    pub fn priority_event(
        &mut self,
        event: EventId,
        target: Holder,
        source: Option<MonRef>,
        effect: EffectId,
        relay: Option<Relay>,
        on_effect: bool,
    ) -> Relay {
        self.run_event_full(event, target, source, effect, relay, on_effect, true)
    }

    /// PS `runEvent` with every argument.
    // PS: sim/battle.ts runEvent
    #[allow(clippy::too_many_arguments)]
    pub fn run_event_full(
        &mut self,
        event: EventId,
        target: Holder,
        source: Option<MonRef>,
        effect: EffectId,
        relay: Option<Relay>,
        on_effect: bool,
        fast_exit: bool,
    ) -> Relay {
        self.check_depth(event);
        let target = if target == Holder::None { Holder::Battle } else { target };
        let mut handlers = Listeners::new();
        self.find_event_handlers(target, event, source, &mut handlers);
        if on_effect {
            assert!(!effect.is_none(), "onEffect passed without an effect");
            let h = self.registry.handler(effect, Prefix::None, event);
            if h.is_none() {
                self.require_handler_ported(effect, Prefix::None, event);
            }
            if let Some(h) = h {
                let mut l = Listener {
                    effect,
                    handler: Some(h),
                    event,
                    state: StateRef::Scratch,
                    holder: target,
                    ..Listener::EMPTY
                };
                self.resolve_priority(&mut l, Prefix::None);
                handlers.push_front(l);
            }
        }
        self.sort_handlers(event, fast_exit, handlers.as_mut_slice());
        // `relayVar === undefined || relayVar === null`: no relay variable.
        let relay = relay.filter(|r| *r != Relay::Null);
        self.run_handlers(event, target, source, effect, relay, fast_exit, &handlers, None)
    }

    fn sort_handlers(&mut self, event: EventId, fast_exit: bool, list: &mut [Listener]) {
        if matches!(event, EventId::Invulnerability | EventId::TryHit | EventId::DamagingHit | EventId::EntryHazard) {
            // PS `compareLeftToRightOrder`.
            stable_sort(list, |a, b| {
                order_or_last(a.key.order)
                    .cmp(&order_or_last(b.key.order))
                    .then(b.key.priority.cmp(&a.key.priority))
                    .then(a.index.unwrap_or(0).cmp(&b.index.unwrap_or(0)))
            });
        } else if fast_exit {
            // PS `compareRedirectOrder`.
            let ability_order = |b: &Battle, l: &Listener| l.holder.mon().map(|m| b.mon(m).ability_state.effect_order);
            let mut keyed = [(Listener::EMPTY, None::<u32>); LISTENER_CAP];
            for (k, l) in keyed.iter_mut().zip(list.iter()) {
                *k = (*l, ability_order(self, l));
            }
            let keyed = &mut keyed[..list.len()];
            stable_sort(keyed, |(a, ao), (b, bo)| {
                b.key.priority.cmp(&a.key.priority).then(b.key.speed.cmp(&a.key.speed)).then(match (ao, bo) {
                    (Some(x), Some(y)) => x.cmp(y),
                    _ => Ordering::Equal,
                })
            });
            for (l, (k, _)) in list.iter_mut().zip(keyed.iter()) {
                *l = *k;
            }
        } else {
            self.speed_sort_listeners(list);
        }
    }

    /// The handler loop of `runEvent`, shared by single and spread targets.
    #[allow(clippy::too_many_arguments)]
    fn run_handlers(
        &mut self,
        event: EventId,
        target: Holder,
        source: Option<MonRef>,
        effect: EffectId,
        relay: Option<Relay>,
        fast_exit: bool,
        handlers: &Listeners,
        spread: Option<&mut SpreadRelays>,
    ) -> Relay {
        let has_relay = relay.is_some();
        let mut relay = relay.unwrap_or(Relay::TRUE);
        let mut args = EventArgs { id: event, relay: if has_relay { Some(relay) } else { None }, target, source, effect };

        let parent_event = self.event;
        self.event = EventFrame {
            id: Some(event),
            target: if spread.is_some() { Holder::None } else { target },
            source,
            effect,
            modifier: Some(4096),
        };
        self.event_depth += 1;

        let mut spread = spread;
        for l in handlers.as_slice() {
            if let (Some(index), Some(sp)) = (l.index, spread.as_deref_mut()) {
                let tr = sp.relays[index as usize];
                if !tr.truthy() && !(tr == Relay::Int(0) && event == EventId::DamagingHit) {
                    continue;
                }
                if let Some(t) = l.target {
                    args.target = Holder::Mon(t);
                    self.event.target = Holder::Mon(t);
                }
                if has_relay {
                    args.relay = Some(tr);
                }
            }
            if self.run_event_skips(event, l) {
                continue;
            }
            let ret = self.call_listener(l, &args);
            if let Some(r) = ret {
                relay = r;
                if !relay.truthy() || fast_exit {
                    if let (Some(index), Some(sp)) = (l.index, spread.as_deref_mut()) {
                        sp.relays[index as usize] = relay;
                        if sp.as_slice().iter().all(|v| !v.truthy()) {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                if has_relay {
                    args.relay = Some(relay);
                }
            }
        }

        self.event_depth -= 1;
        // PS: `if (typeof relayVar === 'number' && relayVar ===
        // Math.abs(Math.floor(relayVar))) relayVar = this.modify(relayVar,
        // this.event.modifier)`. A whole number that is not negative gets
        // the event's modifier; a fraction does not.
        match relay {
            Relay::Int(n) => {
                if n >= 0 {
                    let m = i64::from(self.event.modifier.unwrap_or(0));
                    relay = Relay::Int(crate::modify::modify_4096(self.trunc, i64::from(n), m) as i32);
                }
            }
            Relay::Float(bits) => {
                let v = f64::from_bits(bits);
                if v >= 0.0 && v.is_finite() && v % 1.0 == 0.0 {
                    let m = i64::from(self.event.modifier.unwrap_or(0));
                    let out = crate::modify::modify_4096(self.trunc, v as i64, m);
                    relay = Relay::float(out as f64);
                }
            }
            _ => {}
        }
        self.event = parent_event;
        relay
    }

    /// PS `runEvent` over a list of Pokemon (`targets` is an array): each
    /// target gathers its own handlers, and the result is one relay per
    /// target. `relays` is PS's array `relayVar`, `None` for none.
    // PS: sim/battle.ts runEvent, findEventHandlers (array branch)
    pub fn run_event_spread(
        &mut self,
        event: EventId,
        targets: &[MonRef],
        source: Option<MonRef>,
        effect: EffectId,
        relays: Option<&[Relay]>,
    ) -> SpreadRelays {
        self.check_depth(event);
        assert!(targets.len() <= SPREAD_CAP, "spread event over {SPREAD_CAP} targets");
        let mut handlers = Listeners::new();
        for (i, &t) in targets.iter().enumerate() {
            let start = handlers.len();
            self.find_event_handlers(Holder::Mon(t), event, source, &mut handlers);
            for l in &mut handlers.as_mut_slice()[start..] {
                l.target = Some(t);
                l.index = Some(i as u8);
            }
        }
        self.sort_handlers(event, false, handlers.as_mut_slice());
        let mut out = SpreadRelays { len: targets.len(), relays: [Relay::TRUE; SPREAD_CAP] };
        if let Some(r) = relays {
            out.relays[..r.len()].copy_from_slice(r);
        }
        // An array relayVar is present (and truthy); each handler gets its
        // target's entry in place of it.
        let relay = relays.map(|_| Relay::TRUE);
        self.run_handlers(event, Holder::None, source, effect, relay, false, &handlers, Some(&mut out));
        out
    }

    /// PS `singleEvent(eventid, effect, state, target, source, sourceEffect,
    /// relayVar)`: runs one effect's own handler. `state` / `serial` name
    /// the effect's state (`StateRef::Scratch` for PS's `{}` or `null`).
    // PS: sim/battle.ts singleEvent
    #[allow(clippy::too_many_arguments)]
    pub fn single_event(
        &mut self,
        event: EventId,
        effect: EffectId,
        state: StateRef,
        serial: u16,
        target: Holder,
        source: Option<MonRef>,
        source_effect: EffectId,
        relay: Option<Relay>,
    ) -> Relay {
        self.single_event_with(event, effect, state, serial, target, source, source_effect, relay, None)
    }

    /// `singleEvent` with PS's `customCallback`.
    #[allow(clippy::too_many_arguments)]
    pub fn single_event_with(
        &mut self,
        event: EventId,
        effect: EffectId,
        state: StateRef,
        serial: u16,
        target: Holder,
        source: Option<MonRef>,
        source_effect: EffectId,
        relay: Option<Relay>,
        custom: Option<&'static Handler>,
    ) -> Relay {
        self.check_depth(event);
        let has_relay = relay.is_some();
        let relay = relay.unwrap_or(Relay::TRUE);
        let etype = self.effect_type(effect);
        let target_mon = target.mon();

        if etype == EffectType::Status {
            if let Some(m) = target_mon {
                if self.mon(m).status_effect() != effect {
                    return relay;
                }
            }
        }
        // The Mold Breaker, Embargo and Gastro Acid suppressions (and their
        // `|debug|` lines) cannot happen in gens 1-3; see `run_event_skips`.
        if event == EventId::SwitchIn {
            if let EffectId::Ability(a) = effect {
                if self.dex.ability_def(a).flags.has("breakable") && self.suppressing_ability(target_mon) {
                    panic!("unported: {event:?} handler suppressed by Mold Breaker");
                }
            }
        }
        if !matches!(event, EventId::Start | EventId::TakeItem | EventId::SetAbility)
            && etype == EffectType::Item
            && target_mon.is_some_and(|m| self.ignoring_item(m))
        {
            panic!("unported: {event:?} handler suppressed by Embargo, Klutz or Magic Room");
        }
        if event != EventId::End && etype == EffectType::Ability && target_mon.is_some_and(|m| self.ignoring_ability(m)) {
            panic!("unported: {event:?} handler suppressed by Gastro Acid or Neutralizing Gas");
        }
        if etype == EffectType::Weather
            && !matches!(event, EventId::FieldStart | EventId::FieldResidual | EventId::FieldEnd)
            && self.suppressing_weather()
        {
            self.debug_suppressed_by_air_lock(event);
            return relay;
        }

        let Some(h) = custom.or_else(|| self.registry.handler(effect, Prefix::None, event)) else {
            self.require_handler_ported(effect, Prefix::None, event);
            return relay;
        };

        let parent_frame = self.frame;
        let parent_event = self.event;
        self.event = EventFrame { id: Some(event), target, source, effect: source_effect, modifier: None };
        self.event_depth += 1;
        if matches!(state, StateRef::Scratch | StateRef::Species(_)) {
            self.scratch[self.event_depth as usize] = EffectState::EMPTY;
        }
        self.frame = EffectFrame { effect, state, serial };

        let args = EventArgs {
            id: event,
            relay: if has_relay { Some(relay) } else { None },
            target,
            source,
            effect: source_effect,
        };
        #[cfg(feature = "hit-count")]
        self.count_hit(effect, h);
        let ret = match h.callback {
            Callback::Fn(f) => f(self, &args),
            Callback::Const(v) => Some(v),
        };

        self.event_depth -= 1;
        self.frame = parent_frame;
        self.event = parent_event;
        ret.unwrap_or(relay)
    }

    /// PS `eachEvent(eventid, effect, relayVar)`: `runEvent` with no source
    /// on each active Pokemon, fastest first.
    // PS: sim/battle.ts eachEvent
    pub fn each_event(&mut self, event: EventId, effect: EffectId, relay: Option<bool>) {
        let (mut actives, n) = self.all_active(false);
        // `if (!effect && this.effect) effect = this.effect`: `this.effect`
        // is always an object in PS.
        let effect = if effect.is_none() { self.frame.effect } else { effect };
        let speeds: [i32; 2] = core::array::from_fn(|i| if i < n { self.mon(actives[i]).speed } else { 0 });
        let mut order = [(actives[0], speeds[0]), (actives[1], speeds[1])];
        // PS's comparator `(a, b) => b.speed - a.speed`.
        speed_sort(&mut self.prng, &mut order[..n], |a, b| b.1.cmp(&a.1));
        for (i, (m, _)) in order[..n].iter().enumerate() {
            actives[i] = *m;
        }
        for &m in &actives[..n] {
            self.run_event(event, Holder::Mon(m), None, effect, relay.map(Relay::Bool));
        }
        if event == EventId::Weather && self.gen >= 7 {
            self.each_event(EventId::Update, EffectId::None, None);
        }
    }

    /// PS `fieldEvent(eventid, targets)`: the field, side and Pokemon
    /// handlers of `Residual` and `SwitchIn`, sorted together, with
    /// residual durations counted down and expired effects ended.
    // PS: sim/battle.ts fieldEvent
    pub fn field_event(&mut self, event: EventId, targets: Option<&[MonRef]>) {
        let duration = event == EventId::Residual;
        let mut handlers = Listeners::new();
        let field_event = match event {
            EventId::Residual => Some(EventId::FieldResidual),
            _ => None,
        };
        let side_event = match event {
            EventId::Residual => Some(EventId::SideResidual),
            _ => None,
        };
        let get = |prefix, ev, custom_holder| Find { prefix, event: ev, duration, custom_holder };
        // `onField${eventid}` / `onSide${eventid}`: only Residual has such
        // events in PS's data for gens 1-3.
        if let Some(fe) = field_event {
            self.find_field_handlers(get(Prefix::None, fe, None), &mut handlers);
        }
        for side in 0..2u8 {
            if let Some(se) = side_event {
                self.find_side_handlers(side, get(Prefix::None, se, None), &mut handlers);
            }
            for pos in 0..self.sides[side as usize].active.len() as u8 {
                let Some(active) = self.active_at(side, pos) else { continue };
                if event == EventId::SwitchIn {
                    self.find_pokemon_handlers(active, Find::on(Prefix::Any, event), &mut handlers);
                }
                if targets.is_some_and(|t| !t.contains(&active)) {
                    continue;
                }
                self.find_pokemon_handlers(active, get(Prefix::None, event, None), &mut handlers);
                let custom = Find { prefix: Prefix::None, event, duration: false, custom_holder: Some(active) };
                self.find_side_handlers(side, custom, &mut handlers);
                self.find_field_handlers(custom, &mut handlers);
                self.find_battle_handlers(Find { duration, ..custom }, &mut handlers);
            }
        }
        self.speed_sort_listeners(handlers.as_mut_slice());

        for l in handlers.as_slice() {
            if let Holder::Mon(m) = l.holder {
                if self.mon(m).fainted && !matches!(l.state, StateRef::SlotCondition(..)) {
                    continue;
                }
            }
            // `handler.state?.duration`: the state object as gathered, live
            // or detached; a scratch (species) state has none.
            if event == EventId::Residual
                && l.end != End::None
                && !matches!(l.state, StateRef::Scratch | StateRef::Species(_))
            {
                let st = self.resolve_state_mut(l.state, l.serial);
                if st.duration != 0 {
                    st.duration -= 1;
                    if st.duration == 0 {
                        self.run_end(l);
                        if self.ended {
                            return;
                        }
                        continue;
                    }
                }
            }

            // The effect may have been removed by an earlier handler.
            if !self.state_still_attached(l) {
                continue;
            }

            if l.handler.is_some() {
                // `singleEvent(handlerEventid, effect, handler.state,
                // handler.effectHolder, null, null, undefined, callback)`.
                self.single_event_with(
                    l.event,
                    l.effect,
                    l.state,
                    l.serial,
                    l.holder,
                    None,
                    EffectId::None,
                    None,
                    l.handler,
                );
            }

            self.resolve_faints(crate::battle::Faints::DEFAULT);
            if self.ended {
                return;
            }
        }
    }

    /// `handler.end.call(...endCallArgs)` for an expired residual effect.
    fn run_end(&mut self, l: &Listener) {
        match l.end {
            End::None | End::Noop => {}
            End::ClearStatus(m) => {
                self.clear_status(m);
            }
            End::RemoveVolatile(m) => {
                if let Some(c) = l.effect.condition() {
                    self.remove_volatile(m, c);
                }
            }
            // `clearAbility` / `clearItem` are `setAbility('')` /
            // `setItem('')`; no gen 1-3 ability or item has a duration.
            End::ClearAbility(_) | End::ClearItem(_) => {
                panic!("residual duration on {:?}: clearAbility/clearItem not ported", l.effect)
            }
            End::RemoveSlotCondition(side, pos) => {
                if let Some(c) = l.effect.condition() {
                    self.remove_slot_condition(side, pos, c);
                }
            }
            End::RemoveSideCondition(side) => {
                if let Some(c) = l.effect.condition() {
                    self.remove_side_condition(side, c);
                }
            }
            End::RemovePseudoWeather => {
                self.remove_pseudo_weather(l.effect);
            }
            End::ClearWeather => {
                self.clear_weather();
            }
        }
    }

    /// `fieldEvent`'s check that the handler's state is still where its
    /// `target` says (`expectedStateLocation !== handler.state`). PS only
    /// checks states whose `target` is set; slot conditions are exempt.
    fn state_still_attached(&self, l: &Listener) -> bool {
        let Some(target) = self.state_target_any(l) else { return true };
        let etype = self.effect_type(l.effect);
        let expected = match target {
            Holder::Mon(m) => match etype {
                EffectType::Ability => Some(self.mon(m).ability_state.serial),
                EffectType::Item => Some(self.mon(m).item_state.serial),
                EffectType::Status => Some(self.mon(m).status_state.serial),
                _ => self.mon(m).volatiles.get(l.effect).map(|s| s.serial),
            },
            Holder::Side(s) => {
                if matches!(l.state, StateRef::SlotCondition(..)) {
                    return true;
                }
                self.sides[s as usize].side_conditions.get(l.effect).map(|st| st.serial)
            }
            Holder::Field => match etype {
                EffectType::Weather => Some(self.field.weather_state.serial),
                _ => self.field.pseudo_weather.get(l.effect).map(|s| s.serial),
            },
            Holder::None | Holder::Battle => return true,
        };
        expected == Some(l.serial)
    }

    /// The `target` of the state object a listener holds, live or detached.
    fn state_target_any(&self, l: &Listener) -> Option<Holder> {
        if l.serial == 0 {
            return None;
        }
        let st = self
            .state_at(l.state, l.serial)
            .or_else(|| self.graveyard.iter().find(|s| s.serial == l.serial))?;
        Some(st.target)
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::effect::{EffectDef, Registry};
    use crate::modify::Trunc;
    use crate::state::{MonRef, Pokemon};
    use core::cell::RefCell;
    use ps_dex::gen3::conditions as c;
    use std::vec::Vec;

    std::thread_local! {
        static CALLS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    }

    fn calls() -> Vec<&'static str> {
        CALLS.with(|c| c.borrow_mut().drain(..).collect())
    }

    macro_rules! rec {
        ($name:ident, $tag:literal) => {
            fn $name(_: &mut Battle, _: &EventArgs) -> Option<Relay> {
                CALLS.with(|c| c.borrow_mut().push($tag));
                None
            }
        };
    }

    // Prefix probes: one effect per side, each answering every prefix of
    // ModifyAtk.
    rec!(a_on, "A.on");
    rec!(a_ally, "A.onAlly");
    rec!(a_any, "A.onAny");
    rec!(a_foe, "A.onFoe");
    rec!(a_source, "A.onSource");
    rec!(b_on, "B.on");
    rec!(b_ally, "B.onAlly");
    rec!(b_any, "B.onAny");
    rec!(b_foe, "B.onFoe");
    rec!(b_source, "B.onSource");
    // Side and insertion-order probes.
    rec!(spikes_switch, "spikes");
    rec!(reflect_switch, "reflect");
    rec!(lightscreen_switch, "lightscreen");
    rec!(mist_any, "mist.onAny");
    rec!(mist_foe, "mist.onFoe");
    rec!(safeguard_on, "safeguard.on");
    rec!(encore_res, "encore");
    rec!(disable_res, "disable");
    rec!(taunt_res, "taunt");

    fn boost_half(b: &mut Battle, _: &EventArgs) -> Option<Relay> {
        b.chain_modify(3, 2);
        None
    }
    fn fail(_: &mut Battle, _: &EventArgs) -> Option<Relay> {
        CALLS.with(|c| c.borrow_mut().push("fail"));
        Some(Relay::FAIL)
    }
    fn count(b: &mut Battle, _: &EventArgs) -> Option<Relay> {
        b.effect_state().counter += 1;
        None
    }

    const ATK: EventId = EventId::ModifyAtk;
    static CONFUSION: EffectDef = EffectDef::new(&[
        Handler::on(ATK, a_on),
        Handler::on(ATK, a_ally).prefix(Prefix::Ally),
        Handler::on(ATK, a_any).prefix(Prefix::Any),
        Handler::on(ATK, a_foe).prefix(Prefix::Foe),
        Handler::on(ATK, a_source).prefix(Prefix::Source),
        Handler::on(EventId::Update, a_on),
        Handler::on(EventId::Update, a_any).prefix(Prefix::Any),
    ]);
    static DISABLE: EffectDef = EffectDef::new(&[
        Handler::on(EventId::Residual, disable_res).order(10),
        Handler::on(EventId::ModifySpe, boost_half),
    ]);
    static ENCORE: EffectDef = EffectDef::new(&[
        Handler::on(EventId::Residual, encore_res).order(10),
        Handler::on(EventId::ModifySpe, boost_half),
    ]);
    static TAUNT: EffectDef = EffectDef::new(&[
        Handler::on(EventId::Residual, taunt_res).order(10),
        Handler::on(EventId::Try, fail),
        Handler::on(EventId::AfterMove, count),
    ]);
    static LEECHSEED: EffectDef = EffectDef::new(&[
        Handler::on(ATK, b_on),
        Handler::on(ATK, b_ally).prefix(Prefix::Ally),
        Handler::on(ATK, b_any).prefix(Prefix::Any),
        Handler::on(ATK, b_foe).prefix(Prefix::Foe),
        Handler::on(ATK, b_source).prefix(Prefix::Source),
        Handler::on(EventId::Try, fail),
    ]);
    static LIGHTSCREEN: EffectDef = EffectDef::new(&[
        Handler::on(EventId::SwitchIn, lightscreen_switch),
        Handler::on(EventId::ModifyDef, lightscreen_switch),
    ]);
    static MIST: EffectDef = EffectDef::new(&[
        Handler::on(ATK, mist_any).prefix(Prefix::Any),
        Handler::on(ATK, mist_foe).prefix(Prefix::Foe),
    ]);
    static REFLECT: EffectDef = EffectDef::new(&[
        Handler::on(EventId::SwitchIn, reflect_switch),
        Handler::on(EventId::ModifyDef, reflect_switch),
    ]);
    static SAFEGUARD: EffectDef = EffectDef::new(&[Handler::on(ATK, safeguard_on)]);
    static SPIKES: EffectDef = EffectDef::new(&[
        Handler::on(EventId::SwitchIn, spikes_switch),
        Handler::on(EventId::ModifyDef, spikes_switch),
    ]);

    static REG: Registry = Registry {
        entries: &[
            (EffectId::Condition(c::CONFUSION), &CONFUSION),
            (EffectId::Condition(c::DISABLE), &DISABLE),
            (EffectId::Condition(c::ENCORE), &ENCORE),
            (EffectId::Condition(c::TAUNT), &TAUNT),
            (EffectId::Condition(c::LEECHSEED), &LEECHSEED),
            (EffectId::Condition(c::LIGHTSCREEN), &LIGHTSCREEN),
            (EffectId::Condition(c::MIST), &MIST),
            (EffectId::Condition(c::REFLECT), &REFLECT),
            (EffectId::Condition(c::SAFEGUARD), &SAFEGUARD),
            (EffectId::Condition(c::SPIKES), &SPIKES),
        ],
    };

    const A: MonRef = MonRef { side: 0, index: 0 };
    const B: MonRef = MonRef { side: 1, index: 0 };

    /// A gen 3 battle with one active Pokemon per side, both at speed 100.
    fn battle() -> std::boxed::Box<Battle> {
        calls();
        let mut b = std::boxed::Box::new(Battle::new(&ps_dex::GEN3, &REG, [1, 2, 3, 4], Trunc::Math));
        b.lax_handlers = true;
        for side in 0..2u8 {
            let mut p = Pokemon::EMPTY;
            p.exists = true;
            p.species = ps_dex::gen3::species::PIKACHU;
            p.base_species = p.species;
            p.types = crate::relay::TypeList([ps_dex::Type::Electric, ps_dex::Type::None]);
            p.hp = 100;
            p.maxhp = 100;
            p.speed = 100;
            p.is_active = true;
            b.sides[side as usize].mons[0] = p;
            b.sides[side as usize].active[0] = Some(0);
        }
        b
    }

    fn gathered(b: &Battle, target: Holder, event: EventId, source: Option<MonRef>) -> Vec<(EffectId, Prefix)> {
        let mut out = Listeners::new();
        b.find_event_handlers(target, event, source, &mut out);
        out.as_slice()
            .iter()
            .map(|l| (l.effect, l.handler.map_or(Prefix::None, |h| h.prefix)))
            .collect()
    }

    fn cond(id: ps_dex::ConditionId) -> EffectId {
        EffectId::Condition(id)
    }

    #[test]
    fn registry_is_sorted() {
        assert!(REG.is_sorted());
    }

    #[test]
    fn effect_table_keeps_insertion_order() {
        let mut t: EffectTable<4> = EffectTable::new();
        for id in [c::TAUNT, c::CONFUSION, c::ENCORE] {
            t.insert(cond(id), EffectState::EMPTY);
        }
        assert_eq!(t.ids(), &[cond(c::TAUNT), cond(c::CONFUSION), cond(c::ENCORE)]);
        // Re-assigning a present key keeps its place, as `obj[k] = v` does.
        t.insert(cond(c::TAUNT), EffectState { counter: 7, ..EffectState::EMPTY });
        assert_eq!(t.ids()[0], cond(c::TAUNT));
        // `delete` then re-add goes last.
        assert!(t.remove(cond(c::TAUNT)).is_some());
        t.insert(cond(c::TAUNT), EffectState::EMPTY);
        assert_eq!(t.ids(), &[cond(c::CONFUSION), cond(c::ENCORE), cond(c::TAUNT)]);
    }

    use crate::state::EffectTable;

    #[test]
    fn volatiles_gather_in_insertion_order() {
        let mut b = battle();
        for id in [c::TAUNT, c::DISABLE, c::ENCORE] {
            assert_eq!(b.add_volatile(A, id, None, EffectId::None), Relay::TRUE);
        }
        assert!(b.remove_volatile(A, c::DISABLE));
        assert_eq!(b.add_volatile(A, c::DISABLE, None, EffectId::None), Relay::TRUE);
        let mut out = Listeners::new();
        b.find_pokemon_handlers(A, Find { prefix: Prefix::None, event: EventId::Residual, duration: true, custom_holder: None }, &mut out);
        let ids: Vec<EffectId> = out.as_slice().iter().map(|l| l.effect).collect();
        assert_eq!(ids, [cond(c::TAUNT), cond(c::ENCORE), cond(c::DISABLE)]);
        // effectOrder follows creation: taunt 0, disable 1, encore 2, disable again 3.
        let orders: Vec<u32> = b.mon(A).volatiles.iter().map(|(_, s)| s.effect_order).collect();
        assert_eq!(orders, [0, 2, 3]);
    }

    #[test]
    fn effect_order_breaks_switch_in_ties_only() {
        let mut b = battle();
        // Created in this order: reflect, spikes, lightscreen.
        for id in [c::REFLECT, c::SPIKES, c::LIGHTSCREEN] {
            assert_eq!(b.add_side_condition(0, id, Some(A), EffectId::None), Relay::TRUE);
        }
        // SwitchIn: identical order, priority, speed and subOrder; the
        // effectOrder decides, so no PRNG call.
        let before = b.prng.calls;
        b.run_event(EventId::SwitchIn, Holder::Mon(A), None, EffectId::None, None);
        assert_eq!(b.prng.calls, before);
        assert_eq!(calls(), ["reflect", "spikes", "lightscreen"]);
        // Any other event: a full tie, shuffled with the PRNG (2 calls for 3).
        b.run_event(EventId::ModifyDef, Holder::Mon(A), None, EffectId::None, Some(Relay::Int(10)));
        assert_eq!(b.prng.calls, before + 2);
        let mut got = calls();
        got.sort();
        assert_eq!(got, ["lightscreen", "reflect", "spikes"]);
    }

    #[test]
    fn prefixes_reach_the_right_holders() {
        let mut b = battle();
        b.add_volatile(A, c::CONFUSION, None, EffectId::None);
        b.add_volatile(B, c::LEECHSEED, None, EffectId::None);
        b.add_side_condition(0, c::SAFEGUARD, Some(A), EffectId::None);
        b.add_side_condition(1, c::MIST, Some(B), EffectId::None);
        // runEvent('ModifyAtk', A, B): A's own `on`, A as its own ally
        // (`onAlly`, `onAny`), B as foe (`onFoe`, `onAny`), B as source
        // (`onSource`); A's side `on`, B's side `onFoe` and both `onAny`.
        let got = gathered(&b, Holder::Mon(A), ATK, Some(B));
        let a = cond(c::CONFUSION);
        let l = cond(c::LEECHSEED);
        assert_eq!(
            got,
            [
                (a, Prefix::None),
                (a, Prefix::Ally),
                (a, Prefix::Any),
                (l, Prefix::Foe),
                (l, Prefix::Any),
                (l, Prefix::Source),
                (cond(c::SAFEGUARD), Prefix::None),
                (cond(c::MIST), Prefix::Foe),
                (cond(c::MIST), Prefix::Any),
            ]
        );
        // Without a source, no onSource.
        let got = gathered(&b, Holder::Mon(A), ATK, None);
        assert!(!got.contains(&(l, Prefix::Source)));
        // Events run through eachEvent take no prefixed handlers.
        let got = gathered(&b, Holder::Mon(A), EventId::Update, Some(B));
        assert_eq!(got, [(a, Prefix::None)]);
        // A Side target bubbles down: its actives' `on`, the foe's `onFoe`.
        let got = gathered(&b, Holder::Side(0), ATK, None);
        assert_eq!(
            got,
            [
                (a, Prefix::None),
                (a, Prefix::Any),
                (cond(c::SAFEGUARD), Prefix::None),
                (l, Prefix::Foe),
                (l, Prefix::Any),
                (cond(c::MIST), Prefix::Foe),
                (cond(c::MIST), Prefix::Any),
            ]
        );
        // And they all run.
        b.run_event(ATK, Holder::Mon(A), Some(B), EffectId::None, Some(Relay::Int(50)));
        let mut ran = calls();
        ran.sort();
        assert_eq!(
            ran,
            ["A.on", "A.onAlly", "A.onAny", "B.onAny", "B.onFoe", "B.onSource", "mist.onAny", "mist.onFoe", "safeguard.on"]
        );
    }

    #[test]
    fn inactive_target_without_active_source_skips_holders() {
        let mut b = battle();
        b.add_volatile(A, c::CONFUSION, None, EffectId::None);
        b.mon_mut(A).is_active = false;
        let got = gathered(&b, Holder::Mon(A), ATK, None);
        assert!(got.is_empty());
        // An active source brings the target's handlers back.
        let got = gathered(&b, Holder::Mon(A), ATK, Some(B));
        assert_eq!(got[0], (cond(c::CONFUSION), Prefix::None));
    }

    #[test]
    fn falsy_return_stops_and_modifier_applies() {
        let mut b = battle();
        b.add_volatile(A, c::TAUNT, None, EffectId::None);
        b.add_volatile(B, c::LEECHSEED, None, EffectId::None);
        // Both answer `Try` with false; the first one stops the event.
        let r = b.run_event(EventId::Try, Holder::Mon(A), None, EffectId::None, None);
        assert_eq!(r, Relay::FAIL);
        assert_eq!(calls(), ["fail"]);
        // chainModify(1.5) twice: (6144 * 6144 + 2048) >> 12 = 9216 4096ths.
        b.add_volatile(A, c::ENCORE, None, EffectId::None);
        b.add_volatile(A, c::DISABLE, None, EffectId::None);
        let r = b.run_event(EventId::ModifySpe, Holder::Mon(A), None, EffectId::None, Some(Relay::Int(100)));
        assert_eq!(r, Relay::Int(crate::modify::modify_4096(Trunc::Math, 100, 9216) as i32));
        assert_eq!(r, Relay::Int(225));
        // No relay variable: runEvent returns true when nothing answers.
        assert_eq!(b.run_event(EventId::Hit, Holder::Mon(A), None, EffectId::None, None), Relay::TRUE);
    }

    #[test]
    fn handlers_see_their_own_state() {
        let mut b = battle();
        b.add_volatile(A, c::TAUNT, None, EffectId::None);
        b.run_event(EventId::AfterMove, Holder::Mon(A), None, EffectId::None, None);
        b.run_event(EventId::AfterMove, Holder::Mon(A), None, EffectId::None, None);
        let st = b.mon(A).volatiles.get(cond(c::TAUNT)).unwrap();
        assert_eq!(st.counter, 2);
        assert_eq!(st.target, Holder::Mon(A));
    }

    #[test]
    fn residual_counts_durations_down_and_ends() {
        let mut b = battle();
        b.add_volatile(A, c::TAUNT, None, EffectId::None);
        b.mon_mut(A).volatiles.get_mut(cond(c::TAUNT)).unwrap().duration = 2;
        b.field_event(EventId::Residual, None);
        assert_eq!(calls(), ["taunt"]);
        assert_eq!(b.mon(A).volatiles.get(cond(c::TAUNT)).unwrap().duration, 1);
        b.field_event(EventId::Residual, None);
        // Expired: removed instead of run.
        assert!(calls().is_empty());
        assert!(!b.mon(A).volatiles.contains(cond(c::TAUNT)));
    }

    #[test]
    fn single_event_skips_a_changed_status() {
        let mut b = battle();
        // `par` is not `A`'s status, so its handler is called off.
        let r = b.single_event(
            EventId::Start,
            cond(c::PAR),
            StateRef::Status(A),
            0,
            Holder::Mon(A),
            None,
            EffectId::None,
            Some(Relay::Int(3)),
        );
        assert_eq!(r, Relay::Int(3));
    }

    #[test]
    fn speed_sort_ties_cost_prng_calls() {
        let mut prng = Prng::new([1, 2, 3, 4]);
        let mut list = [3, 1, 3, 2, 3];
        speed_sort(&mut prng, &mut list, |a: &i32, b: &i32| b.cmp(a));
        assert_eq!(&list[3..], &[2, 1]);
        // One run of three 3s: shuffle(0, 3) draws twice.
        assert_eq!(prng.calls, 2);
    }
}
