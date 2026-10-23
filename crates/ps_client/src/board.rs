//! The board events the client yields: the vocabulary the UI already speaks.
//!
//! These mirror `mega_blastoise_core::board_event::{BoardEvent, MoveSlot,
//! PromptKind}` field for field (same variant names, same field names and
//! types, same string conventions), so the runner (task P2-2) converts them
//! with one mechanical `match`. They live here because `ps_client` sits below
//! `mega_blastoise_core` in the crate graph: the core will depend on this
//! crate, not the other way round.
//!
//! The string conventions are the old runners', which `oled_ctl` and the
//! narration parse:
//! - a `mon` field is a position string `"NAME,pN,0"` (the Gen 1 battler's
//!   form; `board_event::mon_player_id` reads the seat back out of it);
//! - `health` is `"CUR/MAX"`, what `hp_bar::HpBarState::parse` reads;
//! - `Win::side` is the winner's side index, `"0"` or `"1"`, what
//!   `BoardEvent::win_player_num` reads;
//! - `SetStatus` / `CureStatus` carry PS's status id (`"par"`);
//! - `Cant::reason` and `EffectStart::what` use the Gen 1 runner's words
//!   where it had one (`"confusion"`, `"trapped"`, `"seeded"`, ...), since
//!   `BoardEvent::description` narrates exactly those.
//!
//! No UI decisions here: which screen an event leads to is `oled_ctl`'s
//! business.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// Why the engine is waiting on a player. Mirrors core's `PromptKind`; PS
/// requests only ever produce the first two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    ChooseMove,
    ChooseSwitch,
    TeamPreview,
    LearnMove,
}

/// One move slot of the active mon. Mirrors core's `MoveSlot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveSlot {
    pub name: String,
    /// Display type name, e.g. `"Electric"`.
    pub type_name: String,
    /// `"Physical"`, `"Special"`, or `"Status"`.
    pub category: String,
    /// Base power; `None` for moves with none.
    pub power: Option<u32>,
    /// Accuracy 0 to 100; `None` for moves that always hit.
    pub accuracy: Option<u8>,
    pub pp: u8,
    pub max_pp: u8,
}

/// Something the board should represent. Mirrors core's `BoardEvent`, minus
/// `Split`, `SwitchOut` and `Raw`, which the client never yields (the view
/// consumes PS's `|split|` itself, PS has no switch-out line, and a line with
/// no board meaning yields nothing rather than a raw dump).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardEvent {
    Damage {
        mon: String,
        health: String,
    },
    Heal {
        mon: String,
        health: String,
    },
    Faint {
        mon: String,
        /// Team slot (0-based, the order `>player` gave the team in).
        team_slot: Option<u8>,
    },
    Move {
        /// The user's name, no seat suffix.
        user: Option<String>,
        /// `"p1"` / `"p2"`.
        player_id: Option<String>,
        name: String,
    },
    SwitchIn {
        /// Nickname (PS's `NAME` in `p1a: NAME`).
        name: String,
        species: Option<String>,
        player_id: Option<String>,
        team_slot: Option<u8>,
        /// Empty when the view has not seen this side's request yet.
        moves: Vec<MoveSlot>,
        /// The mon's Speed stat, when a request has told the view.
        speed: Option<u16>,
    },
    Turn {
        n: u32,
    },
    BattleStart,
    Win {
        side: Option<String>,
    },
    Tie,
    Prompt {
        player_id: String,
        kind: PromptKind,
    },
    SuperEffective { mon: String },
    Resisted { mon: String },
    Immune { mon: String },
    Miss { mon: String },
    CriticalHit { mon: String },
    SetStatus { mon: String, status: String },
    CureStatus { mon: String, status: String },
    Cant { mon: String, reason: String },
    StatChange { mon: String, stat: String, delta: i8 },
    EffectStart { mon: String, what: String, detail: Option<String> },
    EffectEnd { mon: String, what: String },
    /// The active mon's display identity changed in place (Transform, a
    /// forme change), or its Speed became known after its switch-in.
    ActiveMonUpdate { mon: String, name: String, speed: Option<u16> },
    Fail { mon: String },
    MovesUpdate {
        player_id: String,
        moves: Vec<MoveSlot>,
    },
}

/// What one protocol message or request turned into, in order.
pub type BoardEvents = Vec<BoardEvent>;

/// `"NAME,pN,0"`, the position string every `mon` field carries.
pub fn mon_field(name: &str, player_id: &str) -> String {
    format!("{name},{player_id},0")
}

/// `"CUR/MAX"`.
pub fn health_field(cur: u16, max: u16) -> String {
    format!("{cur}/{max}")
}

impl BoardEvent {
    /// The `player_id` of an event that names a seat, for tests and logs.
    pub fn seat(&self) -> Option<&str> {
        match self {
            BoardEvent::Damage { mon, .. }
            | BoardEvent::Heal { mon, .. }
            | BoardEvent::Faint { mon, .. }
            | BoardEvent::SuperEffective { mon }
            | BoardEvent::Resisted { mon }
            | BoardEvent::Immune { mon }
            | BoardEvent::Miss { mon }
            | BoardEvent::CriticalHit { mon }
            | BoardEvent::SetStatus { mon, .. }
            | BoardEvent::CureStatus { mon, .. }
            | BoardEvent::Cant { mon, .. }
            | BoardEvent::StatChange { mon, .. }
            | BoardEvent::EffectStart { mon, .. }
            | BoardEvent::EffectEnd { mon, .. }
            | BoardEvent::ActiveMonUpdate { mon, .. }
            | BoardEvent::Fail { mon } => mon.split(',').nth(1),
            BoardEvent::Move { player_id, .. } | BoardEvent::SwitchIn { player_id, .. } => {
                player_id.as_deref()
            }
            BoardEvent::Prompt { player_id, .. } | BoardEvent::MovesUpdate { player_id, .. } => {
                Some(player_id.as_str())
            }
            BoardEvent::Turn { .. } | BoardEvent::BattleStart | BoardEvent::Win { .. } | BoardEvent::Tie => None,
        }
    }
}
