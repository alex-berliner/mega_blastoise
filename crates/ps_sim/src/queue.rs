//! The action queue (design 5.2): PS's `BattleQueue`, its `orders` table,
//! `resolveAction`, `insertChoice` and `sort`, and `getActionSpeed`.
//!
//! An [`Action`] is PS's `Action` union flattened into one struct: the
//! fields a kind does not use stay at their defaults. The list holds
//! [`QUEUE_CAP`] actions; overflow panics (a dropped action would be a
//! divergence).
//!
//! PS: sim/battle-queue.ts (whole), sim/battle.ts `getActionSpeed`.

use core::cmp::Ordering;

use ps_dex::MoveId;

use crate::effect::{EffectId, EventId};
use crate::events::{compare_priority, speed_sort, SortKey};
use crate::relay::Relay;
use crate::state::{Battle, Holder, MonRef};

/// Most actions one turn queues: two moves (each with a `beforeTurnMove`),
/// two switches with their `runSwitch`es, `beforeTurn`, `residual`, and
/// mid-turn instaswitches.
pub const QUEUE_CAP: usize = 16;

/// PS's `action.choice`, restricted to what gens 1 and 3 singles queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    /// `team`: one Pokemon of a team preview pick, in the picked order.
    Team,
    Start,
    InstaSwitch,
    BeforeTurn,
    BeforeTurnMove,
    RunSwitch,
    Switch,
    Move,
    /// `event`: run `action.event` on `action.pokemon`.
    Event,
    Residual,
    /// `pass` (resolved to nothing).
    Pass,
}

impl ActionKind {
    /// PS's `orders` table in `resolveAction`.
    pub fn order(self) -> u16 {
        match self {
            ActionKind::Team => 1,
            ActionKind::Start => 2,
            ActionKind::InstaSwitch => 3,
            ActionKind::BeforeTurn => 4,
            ActionKind::BeforeTurnMove => 5,
            ActionKind::RunSwitch => 101,
            ActionKind::Switch => 103,
            ActionKind::Residual => 300,
            ActionKind::Move | ActionKind::Event | ActionKind::Pass => 200,
        }
    }
}

/// The move of a move action: a dex move, or one of the ids PS puts in
/// `moveid` that name no move (`getActiveMove` of them is an empty move
/// object with that id).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ActionMove {
    #[default]
    None,
    Move(MoveId),
    /// `recharge` (a `LockMove` answer).
    Recharge,
    /// Gen 1's `fight` button.
    Fight,
    /// Gen 1's `nomove` (`lastSelectedMove` before any selection).
    NoMove,
    /// Gen 1's `cannotmove`.
    CannotMove,
}

impl ActionMove {
    pub fn id(self) -> Option<MoveId> {
        match self {
            ActionMove::Move(m) => Some(m),
            _ => None,
        }
    }

    pub fn is_some(self) -> bool {
        self != ActionMove::None
    }

    /// The move as an effect (`EffectId::None` for an id with no move).
    pub fn effect(self) -> EffectId {
        self.id().map_or(EffectId::None, EffectId::Move)
    }
}

/// One queued action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Action {
    pub choice: ActionKind,
    pub order: u16,
    pub priority: i16,
    /// `fractionalPriority`, as the `FractionalPriority` event's integer
    /// answer (no gen 1-3 effect answers it). Actions have no `subOrder`.
    pub fractional_priority: i8,
    pub speed: i32,
    pub pokemon: Option<MonRef>,
    pub target_loc: i8,
    pub original_target: Option<MonRef>,
    pub move_: ActionMove,
    /// A switch's incoming Pokemon.
    pub target: Option<MonRef>,
    /// `sourceEffect` (a switch's `[from]` move).
    pub source_effect: EffectId,
    /// An `event` action's event.
    pub event: Option<EventId>,
    /// A `team` action's `index`: its place in the picked order.
    pub index: u8,
}

impl Action {
    /// A fresh `{choice}` with nothing resolved (`order` 0 is unset).
    pub const fn new(choice: ActionKind) -> Action {
        Action {
            choice,
            order: 0,
            priority: 0,
            fractional_priority: 0,
            speed: 0,
            pokemon: None,
            target_loc: 0,
            original_target: None,
            move_: ActionMove::None,
            target: None,
            source_effect: EffectId::None,
            event: None,
            index: 0,
        }
    }

    pub fn with_pokemon(mut self, m: MonRef) -> Action {
        self.pokemon = Some(m);
        self
    }

    /// The fields `comparePriority` reads (actions have no `effectOrder`).
    pub fn key(&self) -> SortKey {
        SortKey {
            order: i64::from(self.order),
            priority: i32::from(self.priority) + i32::from(self.fractional_priority),
            speed: self.speed,
            sub_order: 0,
            effect_order: 0,
        }
    }
}

/// PS's `BattleQueue.list`.
#[derive(Clone, Copy, Debug)]
pub struct BattleQueue {
    len: u8,
    list: [Action; QUEUE_CAP],
}

impl Default for BattleQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl BattleQueue {
    pub const fn new() -> BattleQueue {
        BattleQueue { len: 0, list: [Action::new(ActionKind::Pass); QUEUE_CAP] }
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[Action] {
        &self.list[..self.len()]
    }

    pub fn as_mut_slice(&mut self) -> &mut [Action] {
        let n = self.len();
        &mut self.list[..n]
    }

    /// `shift()`.
    pub fn shift(&mut self) -> Option<Action> {
        if self.len == 0 {
            return None;
        }
        let a = self.list[0];
        self.remove(0);
        Some(a)
    }

    /// `peek()`.
    pub fn peek(&self) -> Option<&Action> {
        self.as_slice().first()
    }

    /// `peek(true)`.
    pub fn peek_end(&self) -> Option<&Action> {
        self.as_slice().last()
    }

    /// `push(action)`.
    pub fn push(&mut self, a: Action) {
        let n = self.len();
        assert!(n < QUEUE_CAP, "action queue full ({QUEUE_CAP})");
        self.list[n] = a;
        self.len += 1;
    }

    /// `unshift(action)`.
    pub fn unshift(&mut self, a: Action) {
        self.insert(0, a);
    }

    /// `list.splice(i, 0, a)`.
    pub fn insert(&mut self, i: usize, a: Action) {
        let n = self.len();
        assert!(n < QUEUE_CAP, "action queue full ({QUEUE_CAP})");
        self.list.copy_within(i..n, i + 1);
        self.list[i] = a;
        self.len += 1;
    }

    /// `list.splice(i, 1)`.
    pub fn remove(&mut self, i: usize) -> Action {
        let n = self.len();
        let a = self.list[i];
        self.list.copy_within(i + 1..n, i);
        self.len -= 1;
        a
    }

    /// `clear()`.
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// `willAct()`: the first move or switch.
    pub fn will_act(&self) -> Option<&Action> {
        self.as_slice()
            .iter()
            .find(|a| matches!(a.choice, ActionKind::Move | ActionKind::Switch | ActionKind::InstaSwitch))
    }

    /// `willSwitch(pokemon)`.
    pub fn will_switch(&self, m: MonRef) -> Option<&Action> {
        self.as_slice()
            .iter()
            .find(|a| matches!(a.choice, ActionKind::Switch | ActionKind::InstaSwitch) && a.pokemon == Some(m))
    }

    /// `cancelAction(pokemon)`: drops every action of `pokemon` but its
    /// `runSwitch`.
    // PS: sim/battle-queue.ts cancelAction
    pub fn cancel_action(&mut self, m: MonRef) -> bool {
        let old = self.len();
        let mut i = 0;
        while i < self.len() {
            if self.list[i].pokemon == Some(m) && self.list[i].choice != ActionKind::RunSwitch {
                self.remove(i);
            } else {
                i += 1;
            }
        }
        self.len() != old
    }

    /// `getSwitches(side)`: the queued `switch` / `instaswitch` actions,
    /// of one side or (`None`) both.
    // PS: sim/battle-queue.ts getSwitches
    pub fn get_switches(&self, side: Option<u8>) -> impl Iterator<Item = &Action> + '_ {
        self.as_slice().iter().filter(move |a| {
            matches!(a.choice, ActionKind::Switch | ActionKind::InstaSwitch)
                && side.is_none_or(|s| a.pokemon.is_some_and(|p| p.side == s))
        })
    }

    /// `cancelMove(pokemon)`: drops `pokemon`'s first move.
    pub fn cancel_move(&mut self, m: MonRef) -> bool {
        if let Some(i) = self.as_slice().iter().position(|a| a.choice == ActionKind::Move && a.pokemon == Some(m)) {
            self.remove(i);
            return true;
        }
        false
    }
}

/// The actions one `resolveAction` produces (a move and its
/// `beforeTurnMove`).
pub struct Resolved {
    pub len: usize,
    pub actions: [Action; 2],
}

impl Resolved {
    pub fn as_slice(&self) -> &[Action] {
        &self.actions[..self.len]
    }
}

impl Battle {
    /// PS `queue.willMove(pokemon)`.
    // PS: sim/battle-queue.ts willMove
    pub fn will_move(&self, m: MonRef) -> Option<&Action> {
        if self.mon(m).fainted {
            return None;
        }
        self.queue.as_slice().iter().find(|a| a.choice == ActionKind::Move && a.pokemon == Some(m))
    }

    /// PS `queue.resolveAction(action, midTurn)`.
    // PS: sim/battle-queue.ts resolveAction
    pub fn resolve_action(&mut self, action: Action, mid_turn: bool) -> Resolved {
        let mut out = Resolved { len: 0, actions: [Action::new(ActionKind::Pass); 2] };
        if action.choice == ActionKind::Pass {
            return out;
        }
        let mut action = action;
        if action.order == 0 {
            action.order = action.choice.order();
        }
        if !mid_turn {
            if action.choice == ActionKind::Move {
                let has_before_turn = action.move_.id().is_some_and(|mv| {
                    ps_dex::find_handler(self.dex.move_def(mv).handlers, "beforeTurnCallback").is_some()
                });
                if has_before_turn {
                    let mut btm = Action::new(ActionKind::BeforeTurnMove);
                    btm.pokemon = action.pokemon;
                    btm.move_ = action.move_;
                    btm.target_loc = action.target_loc;
                    let r = self.resolve_action(btm, false);
                    out.actions[..r.len].copy_from_slice(r.as_slice());
                    out.len = r.len;
                }
                // No mega, Z, Dynamax, Tera or priorityChargeCallback in
                // gens 1-3.
                let pokemon = action.pokemon.expect("move action without a pokemon");
                let r = self.run_event(
                    EventId::FractionalPriority,
                    Holder::Mon(pokemon),
                    None,
                    action.move_.effect(),
                    Some(Relay::Int(0)),
                );
                action.fractional_priority = match r {
                    Relay::Int(n) => n as i8,
                    other => panic!("FractionalPriority returned {other:?}"),
                };
            } else if matches!(action.choice, ActionKind::Switch | ActionKind::InstaSwitch) {
                let pokemon = action.pokemon.expect("switch action without a pokemon");
                if let crate::state::SwitchFlag::Move(mv) = self.mon(pokemon).switch_flag {
                    action.source_effect = EffectId::Move(mv);
                }
                self.mon_mut(pokemon).switch_flag = crate::state::SwitchFlag::No;
            }
        }
        if action.move_.is_some() {
            let pokemon = action.pokemon.expect("move action without a pokemon");
            if action.target_loc == 0 {
                if let Some(target) = self.get_random_target(pokemon, action.move_) {
                    action.target_loc = self.get_loc_of(pokemon, target);
                }
            }
            action.original_target = self.get_at_loc(pokemon, action.target_loc);
        }
        self.get_action_speed(&mut action);
        out.actions[out.len] = action;
        out.len += 1;
        out
    }

    /// PS `queue.addChoice(choices)`.
    // PS: sim/battle-queue.ts addChoice
    pub fn add_choice(&mut self, choice: Action) {
        let r = self.resolve_action(choice, false);
        for a in r.as_slice() {
            self.queue.push(*a);
        }
    }

    /// PS `queue.insertChoice(choice, midTurn)`: resolves and inserts at
    /// its place in the (sorted) queue, a random place among exact ties.
    // PS: sim/battle-queue.ts insertChoice
    pub fn insert_choice(&mut self, choice: Action, mid_turn: bool) {
        if let Some(m) = choice.pokemon {
            self.update_mon_speed(m);
        }
        let r = self.resolve_action(choice, mid_turn);
        let Some(first) = r.as_slice().first().copied() else {
            // PS indexes `actions[0]` of an empty list here (a `pass`);
            // `comparePriority(undefined, ...)` throws.
            panic!("insertChoice of a pass");
        };
        let mut first_index = None;
        let mut last_index = None;
        for (i, cur) in self.queue.as_slice().iter().enumerate() {
            let compared = compare_priority(&first.key(), &cur.key());
            if compared != Ordering::Greater && first_index.is_none() {
                first_index = Some(i);
            }
            if compared == Ordering::Less {
                last_index = Some(i);
                break;
            }
        }
        match first_index {
            None => {
                for a in r.as_slice() {
                    self.queue.push(*a);
                }
            }
            Some(first_index) => {
                let last_index = last_index.unwrap_or(self.queue.len());
                let index = if first_index == last_index {
                    first_index
                } else {
                    self.prng.random_range(first_index as u32, last_index as u32 + 1) as usize
                };
                for (k, a) in r.as_slice().iter().enumerate() {
                    self.queue.insert(index + k, *a);
                }
            }
        }
    }

    /// PS `queue.prioritizeAction(action, sourceEffect)`: moves `action` (or
    /// adds it) to the front with order 3.
    // PS: sim/battle-queue.ts prioritizeAction
    pub fn prioritize_action(&mut self, action: Action, source_effect: EffectId) {
        if let Some(i) = self.queue.as_slice().iter().position(|a| *a == action) {
            self.queue.remove(i);
        }
        let mut action = action;
        action.source_effect = source_effect;
        action.order = 3;
        self.queue.unshift(action);
    }

    /// PS `queue.changeAction(pokemon, action)`.
    // PS: sim/battle-queue.ts changeAction
    pub fn change_action(&mut self, m: MonRef, action: Action) {
        self.queue.cancel_action(m);
        let mut action = action;
        if action.pokemon.is_none() {
            action.pokemon = Some(m);
        }
        self.insert_choice(action, false);
    }

    /// PS `queue.sort()`: `speedSort` with `comparePriority`.
    // PS: sim/battle-queue.ts sort
    pub fn sort_queue(&mut self) {
        let mut list = [Action::new(ActionKind::Pass); QUEUE_CAP];
        let n = self.queue.len();
        list[..n].copy_from_slice(self.queue.as_slice());
        speed_sort(&mut self.prng, &mut list[..n], |a, b| compare_priority(&a.key(), &b.key()));
        self.queue.as_mut_slice().copy_from_slice(&list[..n]);
    }

    /// PS `getActionSpeed(action)`: a move's priority (with
    /// `ModifyPriority`) and the actor's speed. No Z or Max moves.
    // PS: sim/battle.ts getActionSpeed
    pub fn get_action_speed(&mut self, action: &mut Action) {
        if action.choice == ActionKind::Move {
            let pokemon = action.pokemon.expect("move action without a pokemon");
            // `dex.moves.get(move.id).priority`: 0 for an id with no move.
            let mut priority = action.move_.id().map_or(0, |mv| i32::from(self.dex.move_def(mv).priority));
            let target = self.get_target(pokemon, action.move_, action.target_loc, None);
            let effect = action.move_.effect();
            let r = self.single_event(
                EventId::ModifyPriority,
                effect,
                crate::state::StateRef::Scratch,
                0,
                Holder::Mon(pokemon),
                target,
                EffectId::None,
                Some(Relay::Int(priority)),
            );
            priority = r.as_int().unwrap_or_else(|| panic!("ModifyPriority returned {r:?}"));
            let target_holder = Holder::Mon(pokemon);
            let r = self.run_event(EventId::ModifyPriority, target_holder, target, effect, Some(Relay::Int(priority)));
            priority = r.as_int().unwrap_or_else(|| panic!("ModifyPriority returned {r:?}"));
            action.priority = priority as i16;
            // `priority + fractionalPriority` is kept apart in
            // `Action::key`.
        }
        match action.pokemon {
            None => action.speed = 1,
            Some(m) => {
                // Gen <= 4 reverses a negative fractional priority; none
                // exists in gens 1-3, so this is `getActionSpeed()`.
                if self.gen <= 4 && action.choice == ActionKind::Move && action.fractional_priority < 0 {
                    action.speed = -self.get_stat(m, crate::stats::Stat::Spe, false, false);
                } else {
                    action.speed = self.get_mon_action_speed(m);
                }
            }
        }
    }
}
