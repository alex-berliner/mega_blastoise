//! Effects and their handlers (design 5.3).
//!
//! Every PS effect (a move, a condition, a status, a weather, an ability,
//! an item, a species, a format rule) is an [`EffectId`]. What an effect
//! does in the event system is its [`EffectDef`]: a `&'static` list of
//! [`Handler`]s, each answering one `on{Prefix}{Event}` key, plus an
//! [`EventMask`] so gathering can skip effects that do not answer.
//!
//! A [`Registry`] maps ids to definitions: one per gen, a sorted list of
//! `(EffectId, &EffectDef)` pairs assembled in `mods/<gen>/mod.rs` from one
//! file per effect (`mods/<gen>/{moves,conditions,items,abilities,rules}/`).
//! An effect with no entry has no handlers, like PS's data entries without
//! `on*` keys.
//!
//! The handler signature ([`HandlerFn`]) is frozen at phase-1 exit. A
//! handler is PS's `callback.apply(this, args)`: `&mut Battle` is `this`
//! (with `this.effect`, `this.effectState` and `this.event` in
//! [`Battle::frame`](crate::state::Battle) and [`Battle::event`]),
//! [`EventArgs`] holds `[relayVar,] target, source, sourceEffect`, and the
//! return value is the handler's return: `None` is `undefined`.
//!
//! PS: sim/battle.ts (`runEvent`, `resolvePriority`), sim/dex-conditions.ts,
//! data/rulesets.ts.

use ps_dex::{AbilityId, ConditionId, ConditionType, Dex, ItemId, MoveId, SpeciesId};

use crate::relay::Relay;
use crate::state::{Battle, Holder, MonRef};

/// A format rule, or the format itself, as an effect. PS adds every rule
/// with event handlers to the field as a pseudo-weather
/// (`Battle` constructor), and the format's own handlers run through
/// `findBattleEventHandlers`.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rule {
    /// The battle's format (`gen1customgame`, `gen3randombattle`, ...).
    Format = 0,
    HpPercentageMod,
    CancelMod,
    DesyncClauseMod,
    SleepClauseMod,
    FreezeClauseMod,
    /// Our named deviation: Gen 1's bit-width `trunc` (design section 3).
    CartridgeTrunc,
    EndlessBattleClause,
    /// Validator rules that only announce themselves in battle.
    SpeciesClause,
    OhkoClause,
    EvasionMovesClause,
    BeatUpNicknamesMod,
    SwitchPriorityClauseMod,
    EvasionItemsClause,
}

/// Any effect. The derived order (variant, then id) is the registry's
/// sort order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum EffectId {
    /// PS's empty effect (`''`, `EMPTY_CONDITION`): no handlers.
    #[default]
    None,
    Move(MoveId),
    /// An entry of the gen's condition table: a status, a weather, a
    /// volatile, a side or slot condition (moves' nested `condition`s
    /// included, as `dex.conditions.getByID` finds them).
    Condition(ConditionId),
    Ability(AbilityId),
    Item(ItemId),
    Species(SpeciesId),
    Rule(Rule),
    /// An id PS looks up with `dex.conditions.getByID` that names no data
    /// entry: a condition with no handlers but its own `id` and `fullname`.
    Pseudo(Pseudo),
}

/// The handler-less conditions PS makes from a bare id string
/// (`battle.damage(n, pokemon, pokemon, 'recoil')`, `heal(..., 'drain')`).
/// Handlers and messages tell them apart by `effect.id`.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Pseudo {
    /// `'recoil'` (fullname `Recoil`): `applyRecoilDamage`.
    Recoil,
    /// `'drain'` (fullname `Drain`): `spreadDamage`'s drain heal.
    Drain,
}

impl Pseudo {
    /// `effect.id`.
    pub const fn id(self) -> &'static str {
        match self {
            Pseudo::Recoil => "recoil",
            Pseudo::Drain => "drain",
        }
    }
}

impl EffectId {
    pub fn is_none(self) -> bool {
        self == EffectId::None
    }

    /// The condition id, if this is a condition.
    pub fn condition(self) -> Option<ConditionId> {
        match self {
            EffectId::Condition(c) => Some(c),
            _ => None,
        }
    }
}

/// PS's `effect.effectType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectType {
    /// The empty effect (PS's `EMPTY_CONDITION` is a `Condition` with no
    /// handlers; nothing reads its type).
    None,
    Move,
    Condition,
    Weather,
    Status,
    Ability,
    Item,
    /// A species (`Pokemon` in PS).
    Pokemon,
    Rule,
    Format,
}

impl EffectType {
    /// The effect type of `id` in `dex`.
    pub fn of(dex: &Dex, id: EffectId) -> EffectType {
        match id {
            EffectId::None => EffectType::None,
            EffectId::Move(_) => EffectType::Move,
            EffectId::Condition(c) => match dex.condition_def(c).effect_type {
                ConditionType::Condition => EffectType::Condition,
                ConditionType::Weather => EffectType::Weather,
                ConditionType::Status => EffectType::Status,
            },
            EffectId::Ability(_) => EffectType::Ability,
            EffectId::Item(_) => EffectType::Item,
            EffectId::Species(_) => EffectType::Pokemon,
            EffectId::Rule(Rule::Format) => EffectType::Format,
            EffectId::Rule(_) => EffectType::Rule,
            EffectId::Pseudo(_) => EffectType::Condition,
        }
    }
}

macro_rules! events {
    ($($name:ident),* $(,)?) => {
        /// An event, PS's `eventid` (the key of a handler without `on` and
        /// without its prefix). `Side*` and `Field*` events are their own
        /// ids, as PS's `fieldEvent` and `singleEvent` treat them. Keys that
        /// are not events but are dispatched the same way (`basePowerCallback`,
        /// `durationCallback`, ...) are ids too, named in upper camel case.
        #[repr(u8)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum EventId {
            $($name),*
        }

        impl EventId {
            /// Every event, in declaration order.
            pub const ALL: &'static [EventId] = &[$(EventId::$name),*];

            /// The event's name as PS spells it after `on`.
            pub const fn name(self) -> &'static str {
                match self {
                    $(EventId::$name => stringify!($name)),*
                }
            }
        }
    };
}

events! {
    Accuracy,
    AfterBoost,
    AfterEachBoost,
    AfterFaint,
    AfterHit,
    AfterMega,
    AfterMove,
    AfterMoveSecondary,
    AfterMoveSecondarySelf,
    AfterMoveSelf,
    AfterSetStatus,
    AfterSubDamage,
    AfterSwitchInSelf,
    AfterTakeItem,
    AfterUseItem,
    Attract,
    BasePower,
    BasePowerCallback,
    BattleStart,
    BeforeFaint,
    BeforeMove,
    BeforeMoveCallback,
    BeforeSwitchIn,
    BeforeSwitchOut,
    BeforeTurn,
    BeforeTurnCallback,
    Begin,
    ChangeBoost,
    ChargeMove,
    CriticalHit,
    Damage,
    DamageCallback,
    DamagingHit,
    DeductPP,
    DisableMove,
    DragOut,
    DurationCallback,
    Eat,
    EatItem,
    Effectiveness,
    EmergencyExit,
    End,
    EntryHazard,
    Faint,
    FieldEnd,
    FieldResidual,
    FieldRestart,
    FieldStart,
    Flinch,
    FractionalPriority,
    Heal,
    Hit,
    HitField,
    HitSide,
    Immunity,
    Invulnerability,
    LockMove,
    MaybeTrapPokemon,
    ModifyAccuracy,
    ModifyAtk,
    ModifyBoost,
    ModifyCritRatio,
    ModifyDamage,
    ModifyDamagePhase1,
    ModifyDamagePhase2,
    ModifyDef,
    ModifyMove,
    ModifyPriority,
    ModifySecondaries,
    ModifySpA,
    ModifySpD,
    ModifySpe,
    ModifySpecies,
    ModifySTAB,
    ModifyTarget,
    ModifyType,
    ModifyWeight,
    MoveAborted,
    MoveFail,
    NegateImmunity,
    OverrideAction,
    PrepareHit,
    PseudoWeatherChange,
    RedirectTarget,
    Residual,
    Restart,
    SemiLockMove,
    SetAbility,
    SetStatus,
    SetWeather,
    SideConditionStart,
    SideEnd,
    SideResidual,
    SideRestart,
    SideStart,
    StallMove,
    Start,
    SwitchIn,
    SwitchOut,
    TakeItem,
    TerrainChange,
    TrapPokemon,
    Try,
    TryAddVolatile,
    TryBoost,
    TryEatItem,
    TryFieldHit,
    TryHeal,
    TryHit,
    TryHitField,
    TryHitSide,
    TryImmunity,
    TryMove,
    TryPrimaryHit,
    TrySecondaryHit,
    TryTerrain,
    Type,
    Update,
    Use,
    UseItem,
    UseMoveMessage,
    Weather,
    WeatherChange,
    WeatherModifyDamage,
}

const _: () = assert!(EventId::ALL.len() <= 128, "EventMask is a u128");

impl EventId {
    /// Looks an event up by its name after `on` (`"ModifyAtk"`).
    pub fn from_name(name: &str) -> Option<EventId> {
        EventId::ALL.iter().copied().find(|e| e.name() == name)
    }

    /// True for the callbacks named `xyzCallback` in PS's data rather than
    /// `onXyz`.
    pub fn is_callback_key(self) -> bool {
        matches!(
            self,
            EventId::BasePowerCallback
                | EventId::BeforeMoveCallback
                | EventId::BeforeTurnCallback
                | EventId::DamageCallback
                | EventId::DurationCallback
        )
    }
}

/// The prefix of a handler key: `onFoeBeforeMove` is `Foe` + `BeforeMove`.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Prefix {
    None,
    Source,
    Foe,
    Ally,
    Any,
}

impl Prefix {
    pub const fn name(self) -> &'static str {
        match self {
            Prefix::None => "",
            Prefix::Source => "Source",
            Prefix::Foe => "Foe",
            Prefix::Ally => "Ally",
            Prefix::Any => "Any",
        }
    }
}

/// Splits a PS handler key (`onSourceModifyAtk`, `basePowerCallback`) into
/// its prefix and event. `None` for a key no event here answers.
pub fn parse_handler_key(key: &str) -> Option<(Prefix, EventId)> {
    if let Some(rest) = key.strip_prefix("on") {
        if let Some(e) = EventId::from_name(rest) {
            return Some((Prefix::None, e));
        }
        for p in [Prefix::Source, Prefix::Foe, Prefix::Ally, Prefix::Any] {
            if let Some(ev) = rest.strip_prefix(p.name()) {
                if let Some(e) = EventId::from_name(ev) {
                    return Some((p, e));
                }
            }
        }
        return None;
    }
    // `basePowerCallback` -> `BasePowerCallback`.
    let mut buf = [0u8; 32];
    let bytes = key.as_bytes();
    if bytes.is_empty() || bytes.len() > buf.len() {
        return None;
    }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[0] = buf[0].to_ascii_uppercase();
    let name = core::str::from_utf8(&buf[..bytes.len()]).ok()?;
    EventId::from_name(name)
        .filter(|e| e.is_callback_key())
        .map(|e| (Prefix::None, e))
}

/// What a handler receives besides the battle: PS's
/// `[relayVar,] target, source, sourceEffect`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventArgs {
    /// The event being run.
    pub id: EventId,
    /// The relay variable, `None` where PS passes none (the handler then
    /// gets `target` first).
    pub relay: Option<Relay>,
    /// `target`: in `runEvent`, the event's target (each one in turn for a
    /// spread target list); in `singleEvent`, the effect's holder.
    pub target: Holder,
    /// `source`, when it is a Pokemon.
    pub source: Option<MonRef>,
    /// `sourceEffect`.
    pub effect: EffectId,
}

/// A handler body. Returns `None` for PS's `undefined` (keep the relay
/// variable as it is), otherwise the new relay value.
pub type HandlerFn = fn(&mut Battle, &EventArgs) -> Option<Relay>;

/// What a handler key holds: a function, or a constant that PS's
/// `runEvent` returns as is (`onTry: false`, `onLockMove: 'recharge'`).
#[derive(Clone, Copy)]
pub enum Callback {
    Fn(HandlerFn),
    Const(Relay),
}

impl core::fmt::Debug for Callback {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Callback::Fn(_) => f.write_str("Fn"),
            Callback::Const(r) => write!(f, "Const({r:?})"),
        }
    }
}

/// One handler: `on{prefix}{event}` with its ordering data
/// (`on...Order`, `on...Priority`, `on...SubOrder`), copied from PS's data
/// file. 0 means "absent" for each, as `resolvePriority` reads them
/// (`order || false`, `priority || 0`, `subOrder || 0`).
#[derive(Clone, Copy, Debug)]
pub struct Handler {
    pub event: EventId,
    pub prefix: Prefix,
    pub priority: i8,
    /// 0 is PS's `false` (sorted last).
    pub order: i8,
    /// 0 lets `resolvePriority` pick the effect type's default.
    pub sub_order: i8,
    pub callback: Callback,
}

impl Handler {
    /// `on{event}: function`.
    pub const fn on(event: EventId, f: HandlerFn) -> Handler {
        Handler {
            event,
            prefix: Prefix::None,
            priority: 0,
            order: 0,
            sub_order: 0,
            callback: Callback::Fn(f),
        }
    }

    /// `on{event}: <constant>`.
    pub const fn value(event: EventId, v: Relay) -> Handler {
        Handler {
            event,
            prefix: Prefix::None,
            priority: 0,
            order: 0,
            sub_order: 0,
            callback: Callback::Const(v),
        }
    }

    pub const fn prefix(mut self, prefix: Prefix) -> Handler {
        self.prefix = prefix;
        self
    }

    pub const fn priority(mut self, priority: i8) -> Handler {
        self.priority = priority;
        self
    }

    pub const fn order(mut self, order: i8) -> Handler {
        self.order = order;
        self
    }

    pub const fn sub_order(mut self, sub_order: i8) -> Handler {
        self.sub_order = sub_order;
        self
    }
}

/// The set of events an effect has any handler for (any prefix).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct EventMask(pub u128);

impl EventMask {
    pub const EMPTY: EventMask = EventMask(0);

    pub const fn with(self, e: EventId) -> EventMask {
        EventMask(self.0 | (1u128 << e as u8))
    }

    pub const fn contains(self, e: EventId) -> bool {
        self.0 & (1u128 << e as u8) != 0
    }

    /// The mask of a handler list.
    pub const fn of(handlers: &[Handler]) -> EventMask {
        let mut m = EventMask::EMPTY;
        let mut i = 0;
        while i < handlers.len() {
            m = m.with(handlers[i].event);
            i += 1;
        }
        m
    }
}

/// An effect's handlers. Build with [`EffectDef::new`] so the mask matches.
#[derive(Debug)]
pub struct EffectDef {
    pub handlers: &'static [Handler],
    pub events: EventMask,
}

impl EffectDef {
    pub const fn new(handlers: &'static [Handler]) -> EffectDef {
        EffectDef { handlers, events: EventMask::of(handlers) }
    }

    /// The handler for `on{prefix}{event}`, if this effect has one.
    pub fn find(&'static self, prefix: Prefix, event: EventId) -> Option<&'static Handler> {
        if !self.events.contains(event) {
            return None;
        }
        self.handlers.iter().find(|h| h.event == event && h.prefix == prefix)
    }
}

/// One gen's effect definitions, sorted by id.
#[derive(Debug)]
pub struct Registry {
    pub entries: &'static [(EffectId, &'static EffectDef)],
}

impl Registry {
    pub const EMPTY: Registry = Registry { entries: &[] };

    /// The definition of `id`, `None` for an effect without handlers.
    pub fn get(&self, id: EffectId) -> Option<&'static EffectDef> {
        self.entries
            .binary_search_by(|(k, _)| k.cmp(&id))
            .ok()
            .map(|i| self.entries[i].1)
    }

    /// The handler for `on{prefix}{event}` on effect `id`.
    pub fn handler(&self, id: EffectId, prefix: Prefix, event: EventId) -> Option<&'static Handler> {
        self.get(id)?.find(prefix, event)
    }

    /// True when the entries are strictly increasing (sorted, no
    /// duplicates), as `get` needs.
    pub fn is_sorted(&self) -> bool {
        self.entries.windows(2).all(|w| w[0].0 < w[1].0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_dex_handler_key_parses() {
        for dex in [&ps_dex::GEN1, &ps_dex::GEN3] {
            let mut seen = 0;
            let mut check = |hs: &'static [ps_dex::Handler]| {
                for h in hs {
                    let key = h.name();
                    let parsed = parse_handler_key(key);
                    assert!(parsed.is_some(), "gen {}: handler key {key} has no EventId", dex.gen);
                    let (p, e) = parsed.unwrap();
                    let back = if e.is_callback_key() {
                        let n = e.name();
                        let mut s = alloc::string::String::new();
                        s.push(n.as_bytes()[0].to_ascii_lowercase() as char);
                        s.push_str(&n[1..]);
                        s
                    } else {
                        alloc::format!("on{}{}", p.name(), e.name())
                    };
                    assert_eq!(back, key);
                    seen += 1;
                }
            };
            for m in dex.moves {
                check(m.handlers);
                for s in m.secondaries() {
                    check(s.handlers);
                }
                if let Some(s) = m.self_effect() {
                    check(s.handlers);
                }
            }
            for c in dex.conditions {
                check(c.handlers);
            }
            for i in dex.items {
                check(i.handlers);
            }
            for a in dex.abilities {
                check(a.handlers);
            }
            assert!(seen > 0);
        }
    }

    #[test]
    fn mask_and_find() {
        fn f(_: &mut Battle, _: &EventArgs) -> Option<Relay> {
            None
        }
        static HS: [Handler; 2] = [
            Handler::on(EventId::Residual, f).order(5),
            Handler::value(EventId::Try, Relay::FAIL).prefix(Prefix::Foe),
        ];
        static DEF: EffectDef = EffectDef::new(&HS);
        assert!(DEF.events.contains(EventId::Residual));
        assert!(DEF.events.contains(EventId::Try));
        assert!(!DEF.events.contains(EventId::Start));
        assert!(DEF.find(Prefix::None, EventId::Residual).is_some());
        assert!(DEF.find(Prefix::None, EventId::Try).is_none());
        assert!(DEF.find(Prefix::Foe, EventId::Try).is_some());
    }

    #[test]
    fn effect_id_order() {
        assert!(EffectId::None < EffectId::Move(MoveId(0)));
        assert!(EffectId::Move(MoveId(9)) < EffectId::Condition(ConditionId(0)));
        assert!(EffectId::Condition(ConditionId(1)) < EffectId::Condition(ConditionId(2)));
        assert!(EffectId::Item(ItemId(0)) < EffectId::Rule(Rule::Format));
    }
}
