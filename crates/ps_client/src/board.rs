//! The board events: the vocabulary the UI speaks.
//!
//! This is the ONE definition of `BoardEvent`, `MoveSlot` and `PromptKind`.
//! `mega_blastoise_core::board_event` re-exports them and adds what the UI
//! does with them (narration, animation pacing) as its `BoardEventExt`
//! trait. They live here because `ps_client` sits below
//! `mega_blastoise_core` in the crate graph: the core depends on this crate,
//! never the other way round.
//!
//! The string conventions are the old runners', which `oled_ctl` and the
//! narration parse:
//! - a `mon` field is a position string `"NAME,pN,0"` (the Gen 1 battler's
//!   form; `board_event::mon_player_id` reads the seat back out of it);
//! - `health` is `"CUR/MAX"`, what `hp_bar::HpBarState::parse` reads;
//! - `Win::side` is the winner's side index, `"0"` or `"1"`, what
//!   core's `BoardEventExt::win_player_num` reads;
//! - `SetStatus` / `CureStatus` carry PS's status id (`"par"`);
//! - `Cant::reason` and `EffectStart::what` use the Gen 1 runner's words
//!   where it had one (`"confusion"`, `"trapped"`, `"seeded"`, ...), since
//!   core's `BoardEventExt::description` narrates exactly those.
//!
//! No UI decisions here: which screen an event leads to is `oled_ctl`'s
//! business.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// Why the engine is waiting on a player (lights, whose turn on the board).
/// PS requests only ever produce the first two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    ChooseMove,
    ChooseSwitch,
    TeamPreview,
    LearnMove,
}

/// One move slot carried in [`BoardEvent::SwitchIn`] and
/// [`BoardEvent::MovesUpdate`]. The screens render straight from these
/// fields without any further battle queries.
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

/// Something the board should represent (sound, LEDs, prompts, screens).
///
/// The client never yields `Split`, `SwitchOut` or `Raw`: the view consumes
/// PS's `|split|` itself, PS has no switch-out line, and a line with no
/// board meaning yields nothing rather than a raw dump. Those three are the
/// old Gen 1 runner's, whose battler log parser
/// (`mega_blastoise_core::board_event::parse_log_line`) still makes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardEvent {
    /// The battler's `split|side:N`: whose private/public log pair follows.
    Split {
        side: String,
    },
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
    /// A move was used.
    Move {
        /// The user's name, no seat suffix.
        user: Option<String>,
        /// `"p1"` / `"p2"`.
        player_id: Option<String>,
        name: String,
    },
    /// A lead or a bench mon coming in (`switch` / `drag`).
    SwitchIn {
        /// Nickname (PS's `NAME` in `p1a: NAME`).
        name: String,
        species: Option<String>,
        player_id: Option<String>,
        /// Team slot (0-based); `None` if unknown.
        team_slot: Option<u8>,
        /// Empty when the runner has no move list for the mon yet.
        moves: Vec<MoveSlot>,
        /// The mon's Speed stat when known; drives the sprite bob rate.
        speed: Option<u16>,
    },
    /// A mon left the field (the old battler log only).
    SwitchOut {
        name: String,
    },
    Turn {
        n: u32,
    },
    BattleStart,
    Win {
        side: Option<String>,
    },
    Tie,
    /// Waiting for input: the board cues this player before blocking.
    Prompt {
        player_id: String,
        kind: PromptKind,
    },
    SuperEffective { mon: String },
    Resisted { mon: String },
    Immune { mon: String },
    Miss { mon: String },
    CriticalHit { mon: String },
    /// A status condition inflicted (PS's status id, `"par"`).
    SetStatus { mon: String, status: String },
    /// A status condition cured.
    CureStatus { mon: String, status: String },
    /// Can't move this turn.
    Cant { mon: String, reason: String },
    /// A stat stage change.
    StatChange { mon: String, stat: String, delta: i8 },
    /// A volatile effect began: seeded, confusion, substitute, wrap,
    /// disable, reflect, lightscreen, mist, focusenergy, conversion,
    /// transform, mimic, charging, bide, haze, ...
    EffectStart { mon: String, what: String, detail: Option<String> },
    /// A volatile effect ended.
    EffectEnd { mon: String, what: String },
    /// The active mon's display identity changed in place (Transform, a
    /// forme change), or its Speed became known after its switch-in.
    /// Display-only, not narrated.
    ActiveMonUpdate { mon: String, name: String, speed: Option<u16> },
    Fail { mon: String },
    /// An ability showed itself (PS's `[NAME's ABILITY]` banner and the
    /// ability's own line). `how` says why, as a PS id:
    /// - `""`: the `-ability` line itself (Intimidate, Pressure);
    /// - `"activate"`: another line's `[from] ability:`, or
    ///   `-activate|MON|ability: ...` (Water Absorb, Shed Skin);
    /// - `"trace"` / `"roleplay"`: the ability was copied from `other`.
    Ability { mon: String, ability: String, how: String, other: Option<String> },
    /// A held item arrived (`-item`). `how` is the PS id of the effect
    /// that brought it (`"trick"`, `"thief"`, `"covet"`, `"recycle"`);
    /// `other` is the mon it was taken from, for a theft.
    ItemGained { mon: String, item: String, how: String, other: Option<String> },
    /// A held item was used up or lost (`-enditem`). `how` is `"eat"` for
    /// a berry, the PS id of the effect that took it (`"knockoff"`), or
    /// empty when the item used itself up (White Herb, Mental Herb);
    /// `other` is the mon behind it (Knock Off's user).
    ItemEnded { mon: String, item: String, how: String, other: Option<String> },
    /// An effect acted without starting or ending anything the board
    /// keeps (`-activate`, `-fieldactivate`, `-hitcount`, `-ohko`,
    /// `-cureteam`, an `[upkeep]` `-start`, a stat drop an ability
    /// refused). `what` is the effect's PS name (`"Protect"`,
    /// `"confusion"`, `"Focus Band"`, `"Perish Song"`) or one of the
    /// words `"hitcount"`, `"ohko"`, `"nothing"`, `"unboost"`;
    /// `detail` its argument (Spite's move, the stat);
    /// `other` the second mon it names, `n` its number (Spite's PP, the
    /// hit count). `mon` is empty for a field-wide line.
    Activate { mon: String, what: String, detail: Option<String>, other: Option<String>, n: Option<i16> },
    /// The active mon's moves refreshed (PP change, switch-in). Not
    /// narrated.
    MovesUpdate {
        player_id: String,
        moves: Vec<MoveSlot>,
    },
    /// An old battler log line no variant matched (the old Gen 1 runner
    /// only).
    Raw(String),
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
            | BoardEvent::Fail { mon }
            | BoardEvent::Ability { mon, .. }
            | BoardEvent::ItemGained { mon, .. }
            | BoardEvent::ItemEnded { mon, .. }
            | BoardEvent::Activate { mon, .. } => mon.split(',').nth(1),
            BoardEvent::Move { player_id, .. } | BoardEvent::SwitchIn { player_id, .. } => {
                player_id.as_deref()
            }
            BoardEvent::Prompt { player_id, .. } | BoardEvent::MovesUpdate { player_id, .. } => {
                Some(player_id.as_str())
            }
            BoardEvent::Turn { .. }
            | BoardEvent::BattleStart
            | BoardEvent::Win { .. }
            | BoardEvent::Tie
            | BoardEvent::Split { .. }
            | BoardEvent::SwitchOut { .. }
            | BoardEvent::Raw(_) => None,
        }
    }
}
