//! Output messages: one [`Msg`] per PS protocol line (an HP-bearing line's
//! `|split|` pair is one `Msg`).
//!
//! # Line kinds
//!
//! Every line kind of design section 3, plus the ones PS 0.11.11 emits for
//! gens 1 and 3 that the section's list missed (`debug`, `raw`, `-anim`,
//! `-formechange`, `-clearnegativeboost`, `-fieldactivate`, `-nothing`, `error`;
//! found by surveying 600 random battles' omniscient output).
//!
//! | PS line | Variant | Notes |
//! |---|---|---|
//! | `\|` | [`Msg::Spacer`] | the blank line before each turn's actions |
//! | `\|t:\|TIME` | [`Msg::Timestamp`] | the one field the diff normalises |
//! | `\|gametype\|singles` | [`Msg::GameType`] | |
//! | `\|player\|p1\|NAME\|AVATAR\|RATING` | [`Msg::Player`] | text from [`crate::BattleNames`] |
//! | `\|teamsize\|p1\|N` | [`Msg::TeamSize`] | |
//! | `\|gen\|N` | [`Msg::Gen`] | |
//! | `\|tier\|NAME` | [`Msg::Tier`] | |
//! | `\|rule\|RULE: DESC` | [`Msg::Rule`] | |
//! | `\|raw\|HTML` | [`Msg::Raw`] | the custom-rules infobox |
//! | `\|start` | [`Msg::Start`] | |
//! | `\|upkeep` | [`Msg::Upkeep`] | |
//! | `\|turn\|N` | [`Msg::Turn`] | |
//! | `\|win\|NAME` | [`Msg::Win`] | |
//! | `\|tie` | [`Msg::Tie`] | |
//! | `\|debug\|TEXT` | [`Msg::Debug`] | custom games run in debug mode |
//! | `\|message\|TEXT` | [`Msg::Notice`] | the turn-limit line (no dash) |
//! | `\|bigerror\|TEXT` | [`Msg::BigError`] | the auto-tie warnings |
//! | `\|error\|[Invalid choice] TEXT` | [`Msg::Error`] | a `sideupdate` line |
//! | `\|move\|USER\|MOVE\|TARGET` | [`Msg::Move`] | `[from]`, `[still]`, `[miss]`, `[notarget]` in [`Kw`] |
//! | `\|switch\|MON\|DETAILS\|HP` | [`Msg::Switch`] | split pair; `[from] Baton Pass` (PS `switchIn`) |
//! | `\|drag\|MON\|DETAILS\|HP` | [`Msg::Drag`] | split pair |
//! | `\|detailschange\|MON\|DETAILS` | [`Msg::DetailsChange`] | no HP in PS 0.11.11 |
//! | `\|-formechange\|MON\|SPECIES` | [`Msg::FormeChange`] | `[msg]` precedes `[from]` |
//! | `\|cant\|MON\|REASON\|MOVE` | [`Msg::Cant`] | |
//! | `\|faint\|MON` | [`Msg::Faint`] | |
//! | `\|-damage\|MON\|HP` | [`Msg::Damage`] | split pair |
//! | `\|-heal\|MON\|HP` | [`Msg::Heal`] | split pair; `[wisher] NAME` |
//! | `\|-sethp\|MON\|HP` | [`Msg::SetHp`] | split pair |
//! | `\|-status\|MON\|STATUS` | [`Msg::SetStatus`] | |
//! | `\|-curestatus\|MON\|STATUS` | [`Msg::CureStatus`] | |
//! | `\|-cureteam\|MON` | [`Msg::CureTeam`] | |
//! | `\|-boost\|MON\|STAT\|N` | [`Msg::Boost`] | |
//! | `\|-unboost\|MON\|STAT\|N` | [`Msg::Unboost`] | |
//! | `\|-setboost\|MON\|STAT\|N` | [`Msg::SetBoost`] | |
//! | `\|-clearallboost` | [`Msg::ClearAllBoost`] | |
//! | `\|-clearboost\|MON` | [`Msg::ClearBoost`] | |
//! | `\|-clearnegativeboost\|MON` | [`Msg::ClearNegativeBoost`] | |
//! | `\|-copyboost\|SRC\|TGT` | [`Msg::CopyBoost`] | |
//! | `\|-crit\|MON` | [`Msg::Crit`] | |
//! | `\|-supereffective\|MON` | [`Msg::SuperEffective`] | |
//! | `\|-resisted\|MON` | [`Msg::Resisted`] | |
//! | `\|-immune\|MON` | [`Msg::Immune`] | `[ohko]`, `[from]` |
//! | `\|-miss\|SRC\|TGT` | [`Msg::Miss`] | gen 1 has no target |
//! | `\|-fail\|MON\|WHAT\|WHAT2` | [`Msg::Fail`] | |
//! | `\|-block\|MON\|EFFECT\|MOVE\|ATTACKER` | [`Msg::Block`] | |
//! | `\|-notarget\|MON` | [`Msg::NoTarget`] | gen 1 has no Pokemon |
//! | `\|-activate\|MON\|EFFECT\|A\|B` | [`Msg::Activate`] | |
//! | `\|-start\|MON\|EFFECT\|A\|B` | [`Msg::StartEffect`] | |
//! | `\|-end\|MON\|EFFECT` | [`Msg::EndEffect`] | |
//! | `\|-weather\|WEATHER` | [`Msg::Weather`] | `[upkeep]` |
//! | `\|-fieldstart\|EFFECT` | [`Msg::FieldStart`] | |
//! | `\|-fieldend\|EFFECT` | [`Msg::FieldEnd`] | |
//! | `\|-fieldactivate\|EFFECT` | [`Msg::FieldActivate`] | |
//! | `\|-sidestart\|SIDE\|EFFECT` | [`Msg::SideStart`] | |
//! | `\|-sideend\|SIDE\|EFFECT` | [`Msg::SideEnd`] | |
//! | `\|-item\|MON\|ITEM` | [`Msg::Item`] | |
//! | `\|-enditem\|MON\|ITEM` | [`Msg::EndItem`] | `[eat]`; `[silent]` before `[from]` for Trick |
//! | `\|-ability\|MON\|ABILITY\|OLD` | [`Msg::Ability`] | `OLD` is Trace's old ability, or Intimidate's `boost` |
//! | `\|-endability\|MON` | [`Msg::EndAbility`] | |
//! | `\|-transform\|MON\|TARGET` | [`Msg::Transform`] | |
//! | `\|-mustrecharge\|MON` | [`Msg::MustRecharge`] | |
//! | `\|-prepare\|MON\|MOVE\|TARGET` | [`Msg::Prepare`] | |
//! | `\|-anim\|MON\|MOVE\|TARGET` | [`Msg::Anim`] | |
//! | `\|-singleturn\|MON\|EFFECT` | [`Msg::SingleTurn`] | |
//! | `\|-singlemove\|MON\|EFFECT` | [`Msg::SingleMove`] | |
//! | `\|-hitcount\|MON\|N` | [`Msg::HitCount`] | |
//! | `\|-ohko` | [`Msg::Ohko`] | |
//! | `\|-nothing` | [`Msg::Nothing`] | gen 2 data's Splash, inherited by gen 1 |
//! | `\|-message\|TEXT` | [`Msg::Message`] | |
//! | `\|-hint\|TEXT` | [`Msg::Hint`] | may be a one-sided split |
//! | `\|split\|pN` | (none) | an encoding detail of [`Health`] and [`Msg::Hint`] |
//! | `\|request\|JSON` | (none) | [`crate::Request`], sent as `sideupdate` |
//!
//! Not covered (unused by gens 1 and 3 singles): `poke`, `clearpoke`,
//! `teampreview`, `rated`, `inactive`, `replace`, `swap`, `-swapboost`,
//! `-invertboost`, `-clearpositiveboost`, `-swapsideconditions`, `-mega`,
//! `-primal`, `-burst`, `-zpower`, `-zbroken`, `-center`, `-combine`,
//! `-waiting`, `message` (without the dash; only the timer tiebreak uses it).
//!
//! # Encoding conventions
//!
//! - A line is `|KIND|ARG|ARG...`, then its kwargs ([`Kw::encode_order`]).
//! - An optional positional argument that is `None` and has no later argument
//!   is left off; `Msg::Move`'s target is the exception (always present, empty
//!   when `None`, as `attrLastMove('[still]')` blanks it).
//! - Names come from [`crate::DexNames`] and [`crate::BattleNames`].

use crate::command::{CustomRule, FormatId, RuleId};
use crate::ident::{
    AbilityNum, Details, Health, ItemNum, MoveNum, PokemonIdent, SideId, SpeciesNum, Stat, Status,
    TypeNum,
};

/// One protocol line. Fixed size, `Copy`, no heap: at most 24 bytes (a
/// compile-time assertion below holds on every target).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Msg {
    // ---- header and turn structure -------------------------------------
    /// `|`
    Spacer,
    /// `|t:|SECONDS` (Unix time; the diff normalises it).
    Timestamp(u32),
    /// `|gametype|singles`
    GameType(GameType),
    /// `|player|p1|NAME|AVATAR|RATING`, all three from [`crate::BattleNames`]
    /// (`|player|p1|A||` when only a name was given).
    Player(SideId),
    /// `|teamsize|p1|N`
    TeamSize { side: SideId, size: u8 },
    /// `|split|pN` written by [`crate::text::encode_msg`] before each pair of
    /// HP-bearing lines (the secret line and the shared line, when they
    /// differ). The decoder emits one of these between secret and shared
    /// pairs the rebuild of PS's log, and the codec includes it for byte-exact
    /// round-trip with the harness recordings. Not an end message a PS server
    /// forwards to the public stream (the omniscient viewer only sees the
    /// secret form).
    Split(SideId),
    /// `|gen|N`
    Gen(u8),
    /// `|tier|[Gen 3] Custom Game` ([`FormatId::name`]).
    Tier(FormatId),
    /// `|rule|NAME: DESCRIPTION` ([`RuleId::line`]).
    Rule(RuleId),
    /// `|raw|HTML`
    Raw(Text),
    /// `|start`
    Start,
    /// `|upkeep`
    Upkeep,
    /// `|turn|N`
    Turn(u16),
    /// `|win|NAME` (the winning player's name).
    Win(SideId),
    /// `|tie`
    Tie,
    /// `|debug|TEXT`
    Debug(Text),
    /// `|message|TEXT` (no dash): PS's `add('message', ...)`, which the
    /// turn limit uses.
    Notice(Text),
    /// `|bigerror|TEXT`: the auto-tie warnings past turn 500.
    BigError(Text),
    /// `|error|[Invalid choice] TEXT` or `|error|[Unavailable choice] TEXT`,
    /// sent to `side` only (PS `emitChoiceError`, a `sideupdate`).
    Error { side: SideId, unavailable: bool, text: Text },

    // ---- major actions -------------------------------------------------
    /// `|move|USER|MOVE|TARGET` then `kw` (PS `addMove` then `attrLastMove`):
    /// `[from] EFFECT` first, then the flags `[still]`, `[miss]`,
    /// `[notarget]` in call order. `target` renders empty when `None`.
    /// Hidden Power's name is plain `Hidden Power` (the move's dex name).
    Move { user: PokemonIdent, mv: MoveNum, target: Option<PokemonIdent>, kw: Kw },
    /// `|switch|MON|DETAILS|HP STATUS` (split pair), `[from] Baton Pass` in `from`.
    Switch { mon: PokemonIdent, details: Details, health: Health, from: Option<Arg> },
    /// `|drag|MON|DETAILS|HP STATUS` (split pair). PS `switchIn` writes
    /// `switch` and `drag` from one call site, so `drag` may carry `[from]` too.
    Drag { mon: PokemonIdent, details: Details, health: Health, from: Option<Arg> },
    /// `|detailschange|MON|DETAILS`
    DetailsChange { mon: PokemonIdent, details: Details, kw: Kw },
    /// `|-formechange|MON|SPECIES` then `kw` (Forecast: `[msg]|[from] ability: Forecast`).
    FormeChange { mon: PokemonIdent, species: SpeciesNum, kw: Kw },
    /// `|cant|MON|REASON` or `|cant|MON|REASON|MOVE`. Reasons seen: a status
    /// (`par`), a word (`flinch`, `recharge`, `nopp`), a move (`Focus Punch`,
    /// `Disable`, `Attract`), an ability effect (`ability: Truant`). The move is
    /// a name (`Focus Punch`) or, after `nopp`, an id (`hydropump`).
    Cant { mon: PokemonIdent, reason: Arg, mv: Option<Arg> },
    /// `|faint|MON`
    Faint(PokemonIdent),

    // ---- minor actions: HP and status ----------------------------------
    /// `|-damage|MON|HP STATUS` (split pair).
    Damage { mon: PokemonIdent, health: Health, kw: Kw },
    /// `|-heal|MON|HP STATUS` (split pair); Wish's `[wisher] NAME` via [`KwFlags::WISHER`].
    Heal { mon: PokemonIdent, health: Health, kw: Kw },
    /// `|-sethp|MON|HP STATUS` (split pair).
    SetHp { mon: PokemonIdent, health: Health, kw: Kw },
    /// `|-status|MON|STATUS`
    SetStatus { mon: PokemonIdent, status: Status, kw: Kw },
    /// `|-curestatus|MON|STATUS`
    CureStatus { mon: PokemonIdent, status: Status, kw: Kw },
    /// `|-cureteam|MON`
    CureTeam { mon: PokemonIdent, kw: Kw },

    // ---- minor actions: boosts -----------------------------------------
    /// `|-boost|MON|STAT|N`
    Boost { mon: PokemonIdent, stat: Stat, amount: u8, kw: Kw },
    /// `|-unboost|MON|STAT|N` (`N` positive, as PS writes it).
    Unboost { mon: PokemonIdent, stat: Stat, amount: u8, kw: Kw },
    /// `|-setboost|MON|STAT|N`
    SetBoost { mon: PokemonIdent, stat: Stat, amount: i8, kw: Kw },
    /// `|-clearallboost`
    ClearAllBoost { kw: Kw },
    /// `|-clearboost|MON`
    ClearBoost { mon: PokemonIdent, kw: Kw },
    /// `|-clearnegativeboost|MON`
    ClearNegativeBoost { mon: PokemonIdent, kw: Kw },
    /// `|-copyboost|SOURCE|TARGET`
    CopyBoost { source: PokemonIdent, target: PokemonIdent, kw: Kw },

    // ---- minor actions: hit results ------------------------------------
    /// `|-crit|MON`
    Crit(PokemonIdent),
    /// `|-supereffective|MON`
    SuperEffective(PokemonIdent),
    /// `|-resisted|MON`
    Resisted(PokemonIdent),
    /// `|-immune|MON`, with `[ohko]` or `[from] ability: Wonder Guard` in `kw`.
    Immune { mon: PokemonIdent, kw: Kw },
    /// `|-miss|SOURCE` (gen 1) or `|-miss|SOURCE|TARGET`.
    Miss { source: PokemonIdent, target: Option<PokemonIdent> },
    /// `|-fail|MON`, `|-fail|MON|WHAT`, `|-fail|MON|WHAT|WHAT2`
    /// (`-fail|MON|unboost|Attack|[from] ability: Hyper Cutter|[of] MON`).
    Fail { mon: PokemonIdent, what: Option<Arg>, what2: Option<Arg>, kw: Kw },
    /// `|-block|MON|EFFECT|MOVE|ATTACKER`
    Block {
        mon: PokemonIdent,
        effect: Arg,
        mv: Option<MoveNum>,
        attacker: Option<PokemonIdent>,
        kw: Kw,
    },
    /// `|-notarget|MON`, or bare `|-notarget` (gen 1).
    NoTarget(Option<PokemonIdent>),
    /// `|-hitcount|MON|N`
    HitCount { mon: PokemonIdent, count: u8 },
    /// `|-ohko`
    Ohko,
    /// `|-nothing`
    Nothing,

    // ---- minor actions: effects ----------------------------------------
    /// `|-activate|MON|EFFECT|A|B` (`-activate|MON|move: Spite|Thunderbolt|4`).
    /// `mon` is `None` only for PS calls that pass no Pokemon (an empty field).
    Activate { mon: Option<PokemonIdent>, effect: Arg, a: Option<Arg>, b: Option<Arg>, kw: Kw },
    /// `|-start|MON|EFFECT|A|B` (`-start|MON|typechange|Fire`, `-start|MON|Mimic|Surf`).
    StartEffect { mon: PokemonIdent, effect: Arg, a: Option<Arg>, b: Option<Arg>, kw: Kw },
    /// `|-end|MON|EFFECT`
    EndEffect { mon: PokemonIdent, effect: Arg, kw: Kw },
    /// `|-singleturn|MON|EFFECT`
    SingleTurn { mon: PokemonIdent, effect: Arg, kw: Kw },
    /// `|-singlemove|MON|EFFECT`
    SingleMove { mon: PokemonIdent, effect: Arg, kw: Kw },
    /// `|-mustrecharge|MON`
    MustRecharge(PokemonIdent),
    /// `|-prepare|MON|MOVE` or `|-prepare|MON|MOVE|TARGET`
    Prepare { mon: PokemonIdent, mv: MoveNum, target: Option<PokemonIdent> },
    /// `|-anim|MON|MOVE|TARGET`
    Anim { mon: PokemonIdent, mv: MoveNum, target: Option<PokemonIdent> },
    /// `|-transform|MON|TARGET`
    Transform { mon: PokemonIdent, into: PokemonIdent, kw: Kw },

    // ---- minor actions: field and sides --------------------------------
    /// `|-weather|WEATHER` (`[upkeep]`, `[from] ability: Drizzle|[of] MON`).
    Weather { weather: Weather, kw: Kw },
    /// `|-fieldstart|EFFECT`
    FieldStart { effect: Arg, kw: Kw },
    /// `|-fieldend|EFFECT`
    FieldEnd { effect: Arg, kw: Kw },
    /// `|-fieldactivate|EFFECT`
    FieldActivate { effect: Arg, kw: Kw },
    /// `|-sidestart|p1: NAME|EFFECT`
    SideStart { side: SideId, effect: Arg, kw: Kw },
    /// `|-sideend|p1: NAME|EFFECT`
    SideEnd { side: SideId, effect: Arg, kw: Kw },

    // ---- minor actions: items and abilities ----------------------------
    /// `|-item|MON|ITEM`
    Item { mon: PokemonIdent, item: ItemNum, kw: Kw },
    /// `|-enditem|MON|ITEM`
    EndItem { mon: PokemonIdent, item: ItemNum, kw: Kw },
    /// `|-ability|MON|ABILITY`, then a third argument: the old ability's name
    /// when `old` is set (PS `setAbility`: `-ability|MON|NEW|OLD|[from]
    /// ability: Trace|[of] SOURCE`), or `boost` when `boost` is (Intimidate).
    /// PS never writes both; `old` wins if both are set.
    Ability { mon: PokemonIdent, ability: AbilityNum, old: Option<AbilityNum>, boost: bool, kw: Kw },
    /// `|-endability|MON`
    EndAbility { mon: PokemonIdent, kw: Kw },

    // ---- text ----------------------------------------------------------
    /// `|-message|TEXT`
    Message(Text),
    /// `|-hint|TEXT`. With `split: Some(side)` PS sent it with
    /// `addSplit(side, ['-hint', TEXT])`: `|split|pN`, the hint, then an empty
    /// line (the shared form is nothing).
    Hint { text: Text, split: Option<SideId> },
}

const _: () = assert!(core::mem::size_of::<Msg>() <= 24);

impl Msg {
    /// The PS line kind (`"-damage"`), the text between the first two `|`.
    pub const fn kind(&self) -> &'static str {
        match self {
            Msg::Spacer => "",
            Msg::Timestamp(_) => "t:",
            Msg::GameType(_) => "gametype",
            Msg::Player(_) => "player",
            Msg::TeamSize { .. } => "teamsize",
            Msg::Split(_) => "split",
            Msg::Gen(_) => "gen",
            Msg::Tier(_) => "tier",
            Msg::Rule(_) => "rule",
            Msg::Raw(_) => "raw",
            Msg::Start => "start",
            Msg::Upkeep => "upkeep",
            Msg::Turn(_) => "turn",
            Msg::Win(_) => "win",
            Msg::Tie => "tie",
            Msg::Debug(_) => "debug",
            Msg::Notice(_) => "message",
            Msg::BigError(_) => "bigerror",
            Msg::Error { .. } => "error",
            Msg::Move { .. } => "move",
            Msg::Switch { .. } => "switch",
            Msg::Drag { .. } => "drag",
            Msg::DetailsChange { .. } => "detailschange",
            Msg::FormeChange { .. } => "-formechange",
            Msg::Cant { .. } => "cant",
            Msg::Faint(_) => "faint",
            Msg::Damage { .. } => "-damage",
            Msg::Heal { .. } => "-heal",
            Msg::SetHp { .. } => "-sethp",
            Msg::SetStatus { .. } => "-status",
            Msg::CureStatus { .. } => "-curestatus",
            Msg::CureTeam { .. } => "-cureteam",
            Msg::Boost { .. } => "-boost",
            Msg::Unboost { .. } => "-unboost",
            Msg::SetBoost { .. } => "-setboost",
            Msg::ClearAllBoost { .. } => "-clearallboost",
            Msg::ClearBoost { .. } => "-clearboost",
            Msg::ClearNegativeBoost { .. } => "-clearnegativeboost",
            Msg::CopyBoost { .. } => "-copyboost",
            Msg::Crit(_) => "-crit",
            Msg::SuperEffective(_) => "-supereffective",
            Msg::Resisted(_) => "-resisted",
            Msg::Immune { .. } => "-immune",
            Msg::Miss { .. } => "-miss",
            Msg::Fail { .. } => "-fail",
            Msg::Block { .. } => "-block",
            Msg::NoTarget(_) => "-notarget",
            Msg::HitCount { .. } => "-hitcount",
            Msg::Ohko => "-ohko",
            Msg::Nothing => "-nothing",
            Msg::Activate { .. } => "-activate",
            Msg::StartEffect { .. } => "-start",
            Msg::EndEffect { .. } => "-end",
            Msg::SingleTurn { .. } => "-singleturn",
            Msg::SingleMove { .. } => "-singlemove",
            Msg::MustRecharge(_) => "-mustrecharge",
            Msg::Prepare { .. } => "-prepare",
            Msg::Anim { .. } => "-anim",
            Msg::Transform { .. } => "-transform",
            Msg::Weather { .. } => "-weather",
            Msg::FieldStart { .. } => "-fieldstart",
            Msg::FieldEnd { .. } => "-fieldend",
            Msg::FieldActivate { .. } => "-fieldactivate",
            Msg::SideStart { .. } => "-sidestart",
            Msg::SideEnd { .. } => "-sideend",
            Msg::Item { .. } => "-item",
            Msg::EndItem { .. } => "-enditem",
            Msg::Ability { .. } => "-ability",
            Msg::EndAbility { .. } => "-endability",
            Msg::Message(_) => "-message",
            Msg::Hint { .. } => "-hint",
        }
    }

    /// The side whose `|split|pN` precedes this line, if PS splits it: the
    /// side of the Pokemon an HP-bearing line is about, or a one-sided hint's.
    pub fn split_side(&self) -> Option<SideId> {
        match self {
            Msg::Switch { mon, health, .. }
            | Msg::Drag { mon, health, .. }
            | Msg::Damage { mon, health, .. }
            | Msg::Heal { mon, health, .. }
            | Msg::SetHp { mon, health, .. } => health.split.then_some(mon.side),
            Msg::Hint { split, .. } => *split,
            _ => None,
        }
    }

    // ---- constructors for the common lines -----------------------------

    /// `|move|USER|MOVE|TARGET`
    pub fn move_used(user: PokemonIdent, mv: MoveNum, target: Option<PokemonIdent>) -> Msg {
        Msg::Move { user, mv, target, kw: Kw::NONE }
    }

    /// `|switch|MON|DETAILS|HP STATUS`
    pub fn switch(mon: PokemonIdent, details: Details, health: Health) -> Msg {
        Msg::Switch { mon, details, health, from: None }
    }

    /// `|-damage|MON|HP STATUS`
    pub fn damage(mon: PokemonIdent, health: Health) -> Msg {
        Msg::Damage { mon, health, kw: Kw::NONE }
    }

    /// `|-heal|MON|HP STATUS`
    pub fn heal(mon: PokemonIdent, health: Health) -> Msg {
        Msg::Heal { mon, health, kw: Kw::NONE }
    }

    /// `|-status|MON|STATUS`
    pub fn status(mon: PokemonIdent, status: Status) -> Msg {
        Msg::SetStatus { mon, status, kw: Kw::NONE }
    }

    /// `|-boost|MON|STAT|N` or `|-unboost|MON|STAT|N` by the sign of `delta`
    /// (PS `boost()` picks the kind the same way; 0 is a `-boost` of 0).
    pub fn boost_by(mon: PokemonIdent, stat: Stat, delta: i8) -> Msg {
        if delta < 0 {
            Msg::Unboost { mon, stat, amount: delta.unsigned_abs(), kw: Kw::NONE }
        } else {
            Msg::Boost { mon, stat, amount: delta as u8, kw: Kw::NONE }
        }
    }

    /// `|-start|MON|EFFECT`
    pub fn start_effect(mon: PokemonIdent, effect: Arg) -> Msg {
        Msg::StartEffect { mon, effect, a: None, b: None, kw: Kw::NONE }
    }

    /// `|-end|MON|EFFECT`
    pub fn end_effect(mon: PokemonIdent, effect: Arg) -> Msg {
        Msg::EndEffect { mon, effect, kw: Kw::NONE }
    }

    /// `|-activate|MON|EFFECT`
    pub fn activate(mon: PokemonIdent, effect: Arg) -> Msg {
        Msg::Activate { mon: Some(mon), effect, a: None, b: None, kw: Kw::NONE }
    }

    /// `|-fail|MON`
    pub fn fail(mon: PokemonIdent) -> Msg {
        Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE }
    }

    /// The same line with `kw` replaced, for lines that carry kwargs; other
    /// lines come back unchanged.
    pub fn with_kw(mut self, new: Kw) -> Msg {
        if let Some(kw) = self.kw_mut() {
            *kw = new;
        }
        self
    }

    /// The line's kwargs, if its kind carries any.
    pub fn kw(&self) -> Option<&Kw> {
        match self {
            Msg::Move { kw, .. }
            | Msg::DetailsChange { kw, .. }
            | Msg::FormeChange { kw, .. }
            | Msg::Damage { kw, .. }
            | Msg::Heal { kw, .. }
            | Msg::SetHp { kw, .. }
            | Msg::SetStatus { kw, .. }
            | Msg::CureStatus { kw, .. }
            | Msg::CureTeam { kw, .. }
            | Msg::Boost { kw, .. }
            | Msg::Unboost { kw, .. }
            | Msg::SetBoost { kw, .. }
            | Msg::ClearAllBoost { kw }
            | Msg::ClearBoost { kw, .. }
            | Msg::ClearNegativeBoost { kw, .. }
            | Msg::CopyBoost { kw, .. }
            | Msg::Immune { kw, .. }
            | Msg::Fail { kw, .. }
            | Msg::Block { kw, .. }
            | Msg::Activate { kw, .. }
            | Msg::StartEffect { kw, .. }
            | Msg::EndEffect { kw, .. }
            | Msg::SingleTurn { kw, .. }
            | Msg::SingleMove { kw, .. }
            | Msg::Transform { kw, .. }
            | Msg::Weather { kw, .. }
            | Msg::FieldStart { kw, .. }
            | Msg::FieldEnd { kw, .. }
            | Msg::FieldActivate { kw, .. }
            | Msg::SideStart { kw, .. }
            | Msg::SideEnd { kw, .. }
            | Msg::Item { kw, .. }
            | Msg::EndItem { kw, .. }
            | Msg::Ability { kw, .. }
            | Msg::EndAbility { kw, .. } => Some(kw),
            _ => None,
        }
    }

    /// Mutable [`Msg::kw`], for PS's `attrLastMove` on a stored `|move|`.
    pub fn kw_mut(&mut self) -> Option<&mut Kw> {
        match self {
            Msg::Move { kw, .. }
            | Msg::DetailsChange { kw, .. }
            | Msg::FormeChange { kw, .. }
            | Msg::Damage { kw, .. }
            | Msg::Heal { kw, .. }
            | Msg::SetHp { kw, .. }
            | Msg::SetStatus { kw, .. }
            | Msg::CureStatus { kw, .. }
            | Msg::CureTeam { kw, .. }
            | Msg::Boost { kw, .. }
            | Msg::Unboost { kw, .. }
            | Msg::SetBoost { kw, .. }
            | Msg::ClearAllBoost { kw }
            | Msg::ClearBoost { kw, .. }
            | Msg::ClearNegativeBoost { kw, .. }
            | Msg::CopyBoost { kw, .. }
            | Msg::Immune { kw, .. }
            | Msg::Fail { kw, .. }
            | Msg::Block { kw, .. }
            | Msg::Activate { kw, .. }
            | Msg::StartEffect { kw, .. }
            | Msg::EndEffect { kw, .. }
            | Msg::SingleTurn { kw, .. }
            | Msg::SingleMove { kw, .. }
            | Msg::Transform { kw, .. }
            | Msg::Weather { kw, .. }
            | Msg::FieldStart { kw, .. }
            | Msg::FieldEnd { kw, .. }
            | Msg::FieldActivate { kw, .. }
            | Msg::SideStart { kw, .. }
            | Msg::SideEnd { kw, .. }
            | Msg::Item { kw, .. }
            | Msg::EndItem { kw, .. }
            | Msg::Ability { kw, .. }
            | Msg::EndAbility { kw, .. } => Some(kw),
            _ => None,
        }
    }
}

/// `|gametype|...`. Only singles is in scope.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GameType {
    #[default]
    Singles,
}

impl GameType {
    pub const fn as_str(self) -> &'static str {
        match self {
            GameType::Singles => "singles",
        }
    }
}

/// The `WEATHER` of `-weather`: PS's weather condition names, or `none`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Weather {
    #[default]
    None,
    RainDance,
    SunnyDay,
    Sandstorm,
    Hail,
}

impl Weather {
    pub const ALL: [Weather; 5] =
        [Weather::None, Weather::RainDance, Weather::SunnyDay, Weather::Sandstorm, Weather::Hail];

    pub const fn as_str(self) -> &'static str {
        match self {
            Weather::None => "none",
            Weather::RainDance => "RainDance",
            Weather::SunnyDay => "SunnyDay",
            Weather::Sandstorm => "Sandstorm",
            Weather::Hail => "Hail",
        }
    }

    pub fn parse(s: &str) -> Option<Weather> {
        Weather::ALL.into_iter().find(|w| w.as_str() == s)
    }
}

/// A fixed literal PS writes as an effect or reason, where it is not a move,
/// item, ability, status or type name: condition ids (`confusion`,
/// `partiallytrapped`), pseudo-effects (`Recoil`, `drain`) and `cant` /
/// `-fail` reasons (`nopp`, `heal`). Later tasks add entries as PS handlers
/// need them; the text is exactly what PS writes.
macro_rules! words {
    ($($variant:ident => $text:literal,)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        #[repr(u16)]
        pub enum Word { $($variant,)* }

        impl Word {
            pub const ALL: &'static [Word] = &[$(Word::$variant,)*];

            pub const fn as_str(self) -> &'static str {
                match self { $(Word::$variant => $text,)* }
            }

            pub fn parse(s: &str) -> Option<Word> {
                match s { $($text => Some(Word::$variant),)* _ => None }
            }

            fn from_u16(n: u16) -> Option<Word> {
                Word::ALL.get(n as usize).copied()
            }
        }
    };
}

words! {
    Boost => "boost",
    Confusion => "confusion",
    Drain => "drain",
    // Event names in `|debug|EVENT handler suppressed by Air Lock` (PS:
    // sim/battle.ts runEvent, singleEvent): the ones gen 3's weathers and
    // weather abilities answer.
    EventImmunity => "Immunity",
    EventWeather => "Weather",
    EventWeatherModifyDamage => "WeatherModifyDamage",
    Flinch => "flinch",
    Heal => "heal",
    LockedMove => "lockedmove",
    Move => "move",
    MustRecharge => "mustrecharge",
    NoPp => "nopp",
    PartiallyTrapped => "partiallytrapped",
    Perish0 => "perish0",
    Perish1 => "perish1",
    Perish2 => "perish2",
    Perish3 => "perish3",
    Recharge => "recharge",
    Recoil => "Recoil",
    StealEat => "stealeat",
    StruggleRecoil => "recoil",
    Stockpile1 => "stockpile1",
    Stockpile2 => "stockpile2",
    Stockpile3 => "stockpile3",
    Switch => "switch",
    // PS: data/mods/gen1/moves.ts haze.onHit (`-end|MON|Toxic counter|[silent]`)
    ToxicCounter => "Toxic counter",
    Trapped => "trapped",
    TypeChange => "typechange",
    Unboost => "unboost",
}

/// The value of a positional argument or `[from]`, as PS writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArgValue {
    /// A move's name: `Thunderbolt` (also conditions named after their move:
    /// `Substitute`, `Leech Seed`, `Reflect`).
    Move(MoveNum),
    /// A move's `fullname`: `move: Thunderbolt`.
    MoveEffect(MoveNum),
    /// A move's id: `thunderbolt` (`cant|MON|nopp|hydropump`).
    MoveId(MoveNum),
    /// An item's name: `Leftovers`.
    Item(ItemNum),
    /// An item's `fullname`: `item: Leftovers`.
    ItemEffect(ItemNum),
    /// An ability's name: `Pressure`.
    Ability(AbilityNum),
    /// An ability's `fullname`: `ability: Pressure`.
    AbilityEffect(AbilityNum),
    /// A species' name: `Castform-Sunny`.
    Species(SpeciesNum),
    /// A type's name: `Fire`.
    Type(TypeNum),
    /// Two type names joined with a slash: `Water/Flying` (Conversion's
    /// `-start` line prints `source.types.join('/')`).
    TypePair(TypeNum, TypeNum),
    /// A status id: `brn`.
    Status(Status),
    /// A stat's abbreviation: `atk`.
    Stat(Stat),
    /// A stat's handler-written name: `Attack` ([`Stat::name`]).
    StatName(Stat),
    /// A fixed literal.
    Word(Word),
    /// A Pokemon: `p1a: NAME`.
    Mon(PokemonIdent),
    /// A side: `p1: NAME`.
    Side(SideId),
    /// A number (`-activate|MON|move: Spite|Thunderbolt|4`).
    Int(i16),
}

/// [`ArgValue`] packed into 3 bytes with alignment 1, so the variants of
/// [`Msg`] that carry several of them stay inside 24 bytes. Build one with
/// the constructors or `From<ArgValue>`, read it with [`Arg::value`].
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Arg {
    kind: ArgKind,
    data: [u8; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
enum ArgKind {
    Move,
    MoveEffect,
    MoveId,
    Item,
    ItemEffect,
    Ability,
    AbilityEffect,
    Species,
    Type,
    TypePair,
    Status,
    Stat,
    StatName,
    Word,
    Mon,
    Side,
    Int,
}

fn pack_ident(m: PokemonIdent) -> [u8; 2] {
    [m.slot, m.side as u8 | (m.active as u8) << 1]
}

fn unpack_ident(d: [u8; 2]) -> PokemonIdent {
    PokemonIdent {
        side: if d[1] & 1 == 0 { SideId::P1 } else { SideId::P2 },
        slot: d[0],
        active: d[1] & 2 != 0,
    }
}

const STATUS_ALL: [Status; 7] =
    [Status::Brn, Status::Par, Status::Slp, Status::Frz, Status::Psn, Status::Tox, Status::Fnt];

impl Arg {
    pub fn new(v: ArgValue) -> Arg {
        let (kind, data) = match v {
            ArgValue::Move(m) => (ArgKind::Move, m.0.to_le_bytes()),
            ArgValue::MoveEffect(m) => (ArgKind::MoveEffect, m.0.to_le_bytes()),
            ArgValue::MoveId(m) => (ArgKind::MoveId, m.0.to_le_bytes()),
            ArgValue::Item(i) => (ArgKind::Item, i.0.to_le_bytes()),
            ArgValue::ItemEffect(i) => (ArgKind::ItemEffect, i.0.to_le_bytes()),
            ArgValue::Ability(a) => (ArgKind::Ability, a.0.to_le_bytes()),
            ArgValue::AbilityEffect(a) => (ArgKind::AbilityEffect, a.0.to_le_bytes()),
            ArgValue::Species(s) => (ArgKind::Species, s.0.to_le_bytes()),
            ArgValue::Type(t) => (ArgKind::Type, [t.0, 0]),
            ArgValue::TypePair(a, b) => (ArgKind::TypePair, [a.0, b.0]),
            ArgValue::Status(s) => (ArgKind::Status, [s as u8, 0]),
            ArgValue::Stat(s) => (ArgKind::Stat, [s as u8, 0]),
            ArgValue::StatName(s) => (ArgKind::StatName, [s as u8, 0]),
            ArgValue::Word(w) => (ArgKind::Word, (w as u16).to_le_bytes()),
            ArgValue::Mon(m) => (ArgKind::Mon, pack_ident(m)),
            ArgValue::Side(s) => (ArgKind::Side, [s as u8, 0]),
            ArgValue::Int(n) => (ArgKind::Int, n.to_le_bytes()),
        };
        Arg { kind, data }
    }

    pub fn value(self) -> ArgValue {
        let u = u16::from_le_bytes(self.data);
        let b = self.data[0];
        match self.kind {
            ArgKind::Move => ArgValue::Move(MoveNum(u)),
            ArgKind::MoveEffect => ArgValue::MoveEffect(MoveNum(u)),
            ArgKind::MoveId => ArgValue::MoveId(MoveNum(u)),
            ArgKind::Item => ArgValue::Item(ItemNum(u)),
            ArgKind::ItemEffect => ArgValue::ItemEffect(ItemNum(u)),
            ArgKind::Ability => ArgValue::Ability(AbilityNum(u)),
            ArgKind::AbilityEffect => ArgValue::AbilityEffect(AbilityNum(u)),
            ArgKind::Species => ArgValue::Species(SpeciesNum(u)),
            ArgKind::Type => ArgValue::Type(TypeNum(b)),
            ArgKind::TypePair => ArgValue::TypePair(TypeNum(self.data[0]), TypeNum(self.data[1])),
            // Only `new` writes these bytes, from a valid discriminant.
            ArgKind::Status => ArgValue::Status(STATUS_ALL[b as usize % STATUS_ALL.len()]),
            ArgKind::Stat => ArgValue::Stat(Stat::ALL[b as usize % Stat::ALL.len()]),
            ArgKind::StatName => ArgValue::StatName(Stat::ALL[b as usize % Stat::ALL.len()]),
            ArgKind::Word => ArgValue::Word(Word::from_u16(u).unwrap_or(Word::ALL[0])),
            ArgKind::Mon => ArgValue::Mon(unpack_ident(self.data)),
            ArgKind::Side => ArgValue::Side(if b == 0 { SideId::P1 } else { SideId::P2 }),
            ArgKind::Int => ArgValue::Int(i16::from_le_bytes(self.data)),
        }
    }

    pub fn mv(m: MoveNum) -> Arg {
        Arg::new(ArgValue::Move(m))
    }
    pub fn move_effect(m: MoveNum) -> Arg {
        Arg::new(ArgValue::MoveEffect(m))
    }
    pub fn move_id(m: MoveNum) -> Arg {
        Arg::new(ArgValue::MoveId(m))
    }
    pub fn item(i: ItemNum) -> Arg {
        Arg::new(ArgValue::Item(i))
    }
    pub fn item_effect(i: ItemNum) -> Arg {
        Arg::new(ArgValue::ItemEffect(i))
    }
    pub fn ability(a: AbilityNum) -> Arg {
        Arg::new(ArgValue::Ability(a))
    }
    pub fn ability_effect(a: AbilityNum) -> Arg {
        Arg::new(ArgValue::AbilityEffect(a))
    }
    pub fn species(s: SpeciesNum) -> Arg {
        Arg::new(ArgValue::Species(s))
    }
    pub fn ty(t: TypeNum) -> Arg {
        Arg::new(ArgValue::Type(t))
    }
    /// Two types, printed joined with a slash: `Water/Flying`.
    pub fn type_pair(a: TypeNum, b: TypeNum) -> Arg {
        Arg::new(ArgValue::TypePair(a, b))
    }
    pub fn status(s: Status) -> Arg {
        Arg::new(ArgValue::Status(s))
    }
    pub fn stat(s: Stat) -> Arg {
        Arg::new(ArgValue::Stat(s))
    }
    pub fn stat_name(s: Stat) -> Arg {
        Arg::new(ArgValue::StatName(s))
    }
    pub fn word(w: Word) -> Arg {
        Arg::new(ArgValue::Word(w))
    }
    pub fn mon(m: PokemonIdent) -> Arg {
        Arg::new(ArgValue::Mon(m))
    }
    pub fn side(s: SideId) -> Arg {
        Arg::new(ArgValue::Side(s))
    }
    pub fn int(n: i16) -> Arg {
        Arg::new(ArgValue::Int(n))
    }
}

impl From<ArgValue> for Arg {
    fn from(v: ArgValue) -> Arg {
        Arg::new(v)
    }
}

impl core::fmt::Debug for Arg {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.value(), f)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Arg {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "Arg({}, {=[u8]})", self.kind, self.data)
    }
}

/// Valueless kwargs (`|[still]`, `|[silent]`, ...) and two encoding controls.
///
/// Render order of the flags is the bit order below, which is PS's call
/// order wherever several meet on one line (`-end|MON|Wrap|[partiallytrapped]|[silent]`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct KwFlags(pub u16);

impl KwFlags {
    pub const STILL: KwFlags = KwFlags(1 << 0);
    pub const MISS: KwFlags = KwFlags(1 << 1);
    pub const NOTARGET: KwFlags = KwFlags(1 << 2);
    pub const MSG: KwFlags = KwFlags(1 << 3);
    pub const EAT: KwFlags = KwFlags(1 << 4);
    pub const WEAK: KwFlags = KwFlags(1 << 5);
    pub const DAMAGE: KwFlags = KwFlags(1 << 6);
    pub const UPKEEP: KwFlags = KwFlags(1 << 7);
    pub const PARTIALLYTRAPPED: KwFlags = KwFlags(1 << 8);
    pub const SILENT: KwFlags = KwFlags(1 << 9);
    pub const OHKO: KwFlags = KwFlags(1 << 10);
    pub const FATIGUE: KwFlags = KwFlags(1 << 11);
    pub const CONSUMED: KwFlags = KwFlags(1 << 12);
    pub const INTERRUPT: KwFlags = KwFlags(1 << 13);
    /// Control: [`Kw::of`] is Wish's `[wisher] NAME` (the nickname alone),
    /// not `[of] MON`.
    pub const WISHER: KwFlags = KwFlags(1 << 14);
    /// Control: the flags come before `[from]` / `[of]` (PS writes
    /// `-formechange|...|[msg]|[from] ...` and `-enditem|...|[silent]|[from] move: Trick`).
    pub const FLAGS_FIRST: KwFlags = KwFlags(1 << 15);

    /// The valueless kwargs in render order, with their text.
    pub const RENDERED: [(KwFlags, &'static str); 14] = [
        (KwFlags::STILL, "[still]"),
        (KwFlags::MISS, "[miss]"),
        (KwFlags::NOTARGET, "[notarget]"),
        (KwFlags::MSG, "[msg]"),
        (KwFlags::EAT, "[eat]"),
        (KwFlags::WEAK, "[weak]"),
        (KwFlags::DAMAGE, "[damage]"),
        (KwFlags::UPKEEP, "[upkeep]"),
        (KwFlags::PARTIALLYTRAPPED, "[partiallytrapped]"),
        (KwFlags::SILENT, "[silent]"),
        (KwFlags::OHKO, "[ohko]"),
        (KwFlags::FATIGUE, "[fatigue]"),
        (KwFlags::CONSUMED, "[consumed]"),
        (KwFlags::INTERRUPT, "[interrupt]"),
    ];

    pub const fn empty() -> KwFlags {
        KwFlags(0)
    }

    pub const fn contains(self, other: KwFlags) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn union(self, other: KwFlags) -> KwFlags {
        KwFlags(self.0 | other.0)
    }

    pub fn insert(&mut self, other: KwFlags) {
        self.0 |= other.0;
    }

    pub fn remove(&mut self, other: KwFlags) {
        self.0 &= !other.0;
    }
}

impl core::ops::BitOr for KwFlags {
    type Output = KwFlags;
    fn bitor(self, rhs: KwFlags) -> KwFlags {
        self.union(rhs)
    }
}

/// The kwargs of a line: `[from] EFFECT`, `[of] MON` (or `[wisher] NAME`),
/// and the valueless flags.
///
/// Encoding order ([`Kw::encode_order`]): `[from]`, `[of]`, then the flags in
/// [`KwFlags::RENDERED`] order; with [`KwFlags::FLAGS_FIRST`] the flags come
/// first. On `|move|` lines `[from]` is written by `addMove` and the flags by
/// `attrLastMove` afterwards, which is the same order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Kw {
    pub from: Option<Arg>,
    pub of: Option<PokemonIdent>,
    pub flags: KwFlags,
}

/// One rendered kwarg, in [`Kw::encode_order`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KwPart {
    /// `[from] EFFECT`
    From(Arg),
    /// `[of] MON`
    Of(PokemonIdent),
    /// `[wisher] NAME`
    Wisher(PokemonIdent),
    /// A valueless flag, its text from [`KwFlags::RENDERED`].
    Flag(&'static str),
}

impl Kw {
    pub const NONE: Kw = Kw { from: None, of: None, flags: KwFlags(0) };

    /// `[from] EFFECT`
    pub fn from(effect: Arg) -> Kw {
        Kw { from: Some(effect), ..Kw::NONE }
    }

    /// `[from] EFFECT|[of] MON`
    pub fn from_of(effect: Arg, of: PokemonIdent) -> Kw {
        Kw { from: Some(effect), of: Some(of), flags: KwFlags(0) }
    }

    /// Only flags.
    pub fn flags(flags: KwFlags) -> Kw {
        Kw { flags, ..Kw::NONE }
    }

    /// Adds flags.
    pub fn with(mut self, flags: KwFlags) -> Kw {
        self.flags.insert(flags);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.from.is_none() && self.of.is_none() && self.flags.0 & !KwFlags::FLAGS_FIRST.0 == 0
    }

    /// Calls `f` on each kwarg in the order PS writes them.
    pub fn encode_order(&self, mut f: impl FnMut(KwPart)) {
        let flags_first = self.flags.contains(KwFlags::FLAGS_FIRST);
        if flags_first {
            self.each_flag(&mut f);
        }
        if let Some(from) = self.from {
            f(KwPart::From(from));
        }
        if let Some(of) = self.of {
            if self.flags.contains(KwFlags::WISHER) {
                f(KwPart::Wisher(of));
            } else {
                f(KwPart::Of(of));
            }
        }
        if !flags_first {
            self.each_flag(&mut f);
        }
    }

    fn each_flag(&self, f: &mut impl FnMut(KwPart)) {
        for (flag, text) in KwFlags::RENDERED {
            if self.flags.contains(flag) {
                f(KwPart::Flag(text));
            }
        }
    }
}

/// A text line's content: a fixed template with up to two arguments, for
/// `debug`, `-hint`, `-message`, `raw` and `error`. PS builds these strings
/// from a handful of templates per handler; a template's `{}` is replaced by
/// the next argument in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Text {
    pub id: TextId,
    pub args: [TextArg; 3],
}

impl Text {
    pub const fn new(id: TextId) -> Text {
        Text { id, args: [TextArg::NONE; 3] }
    }

    pub const fn with1(id: TextId, a: TextArg) -> Text {
        Text { id, args: [a, TextArg::NONE, TextArg::NONE] }
    }

    pub const fn with2(id: TextId, a: TextArg, b: TextArg) -> Text {
        Text { id, args: [a, b, TextArg::NONE] }
    }

    pub const fn with3(id: TextId, a: TextArg, b: TextArg, c: TextArg) -> Text {
        Text { id, args: [a, b, c] }
    }
}

/// The value substituted for one `{}` of a [`TextId`] template.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextArgValue {
    None,
    /// A decimal number.
    Int(i32),
    /// A move's id (`before turn callback: focuspunch`).
    MoveId(MoveNum),
    /// A move's name.
    Move(MoveNum),
    /// A Pokemon ID (`p1a: NAME`).
    Mon(PokemonIdent),
    /// A Pokemon's nickname alone (`Your NAME doesn't have a move 5`).
    Nickname(PokemonIdent),
    /// A status id (`set status [tox] interrupted`).
    Status(Status),
    /// A fixed literal.
    Word(Word),
    /// A format rule's name (`Sleep Clause Mod`).
    Rule(RuleId),
    /// A custom rule's repeal, `!` and the name (`!Cartridge Trunc`).
    Repeal(RuleId),
}

/// [`TextArgValue`] packed into 5 bytes with alignment 1 (see [`Arg`]).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextArg {
    kind: u8,
    data: [u8; 4],
}

impl TextArg {
    pub const NONE: TextArg = TextArg { kind: 0, data: [0; 4] };

    pub fn new(v: TextArgValue) -> TextArg {
        let mut data = [0u8; 4];
        let kind = match v {
            TextArgValue::None => 0,
            TextArgValue::Int(n) => {
                data = n.to_le_bytes();
                1
            }
            TextArgValue::MoveId(m) => {
                data[..2].copy_from_slice(&m.0.to_le_bytes());
                2
            }
            TextArgValue::Move(m) => {
                data[..2].copy_from_slice(&m.0.to_le_bytes());
                3
            }
            TextArgValue::Mon(m) => {
                data[..2].copy_from_slice(&pack_ident(m));
                4
            }
            TextArgValue::Nickname(m) => {
                data[..2].copy_from_slice(&pack_ident(m));
                5
            }
            TextArgValue::Status(s) => {
                data[0] = s as u8;
                6
            }
            TextArgValue::Word(w) => {
                data[..2].copy_from_slice(&(w as u16).to_le_bytes());
                7
            }
            TextArgValue::Rule(r) => {
                data[0] = r as u8;
                8
            }
            TextArgValue::Repeal(r) => {
                data[0] = r as u8;
                9
            }
        };
        TextArg { kind, data }
    }

    pub fn value(self) -> TextArgValue {
        let u = u16::from_le_bytes([self.data[0], self.data[1]]);
        let id2 = [self.data[0], self.data[1]];
        match self.kind {
            1 => TextArgValue::Int(i32::from_le_bytes(self.data)),
            2 => TextArgValue::MoveId(MoveNum(u)),
            3 => TextArgValue::Move(MoveNum(u)),
            4 => TextArgValue::Mon(unpack_ident(id2)),
            5 => TextArgValue::Nickname(unpack_ident(id2)),
            6 => TextArgValue::Status(STATUS_ALL[self.data[0] as usize % STATUS_ALL.len()]),
            7 => TextArgValue::Word(Word::from_u16(u).unwrap_or(Word::ALL[0])),
            8 => TextArgValue::Rule(RuleId::ALL[self.data[0] as usize % RuleId::ALL.len()]),
            9 => TextArgValue::Repeal(RuleId::ALL[self.data[0] as usize % RuleId::ALL.len()]),
            _ => TextArgValue::None,
        }
    }

    pub fn int(n: i32) -> TextArg {
        TextArg::new(TextArgValue::Int(n))
    }
    pub fn move_id(m: MoveNum) -> TextArg {
        TextArg::new(TextArgValue::MoveId(m))
    }
    pub fn mv(m: MoveNum) -> TextArg {
        TextArg::new(TextArgValue::Move(m))
    }
    pub fn mon(m: PokemonIdent) -> TextArg {
        TextArg::new(TextArgValue::Mon(m))
    }
    pub fn nickname(m: PokemonIdent) -> TextArg {
        TextArg::new(TextArgValue::Nickname(m))
    }
    pub fn status(s: Status) -> TextArg {
        TextArg::new(TextArgValue::Status(s))
    }
    pub fn word(w: Word) -> TextArg {
        TextArg::new(TextArgValue::Word(w))
    }
    pub fn rule(r: RuleId) -> TextArg {
        TextArg::new(TextArgValue::Rule(r))
    }
    /// A custom rule as the infobox writes it: the name, or `!` and the name.
    pub fn custom_rule(c: CustomRule) -> TextArg {
        TextArg::new(if c.repeal { TextArgValue::Repeal(c.rule) } else { TextArgValue::Rule(c.rule) })
    }
}

impl core::fmt::Debug for TextArg {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.value(), f)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for TextArg {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "TextArg({=u8}, {=[u8]})", self.kind, self.data)
    }
}

/// What one `{}` of a [`TextId`] template holds: which [`TextArgValue`]
/// variant fills it. The decoder reads an argument back by its kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TextArgKind {
    Int,
    MoveId,
    Move,
    Mon,
    Nickname,
    Status,
    Word,
    Rule,
}

/// The text templates. `{}` marks an argument, and the parenthesised list
/// gives each argument's [`TextArgKind`]. The strings are exactly PS's (from
/// the `this.debug` / `this.hint` / `this.add('-message', ...)` /
/// `emitChoiceError` call sites named on each group); later tasks add the
/// ones their handlers emit, keeping each group sorted by variant name.
macro_rules! texts {
    ($($variant:ident => $text:literal ($($arg:ident),*),)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        #[repr(u16)]
        pub enum TextId { $($variant,)* }

        impl TextId {
            pub const ALL: &'static [TextId] = &[$(TextId::$variant,)*];

            /// The template, `{}` for each argument.
            pub const fn template(self) -> &'static str {
                match self { $(TextId::$variant => $text,)* }
            }

            /// The kind of each `{}`, in order.
            pub const fn arg_kinds(self) -> &'static [TextArgKind] {
                match self { $(TextId::$variant => &[$(TextArgKind::$arg),*],)* }
            }
        }
    };
}

texts! {
    // ---- debug (custom games run with debug on) ------------------------
    // PS: sim/pokemon.ts addVolatile (`status.id`)
    AddVolatileInterrupted => "add volatile [{}] interrupted" (Word),
    // PS: sim/pokemon.ts runStatusImmunity
    ArtificialStatusImmunity => "artificial status immunity" (),
    // PS: the moves' basePowerCallback (data/moves.ts and data/mods/gen*/moves.ts:
    // Flail, Reversal, Hidden Power, ...)
    BasePower => "BP: {}" (Int),
    // PS: sim/battle.ts runAction `beforeTurnMove` (`action.move.id`)
    BeforeTurnCallback => "before turn callback: {}" (MoveId),
    // PS: data/mods/gen1/moves.ts bide.condition.onBeforeMove
    BideNoDamage => "Bide failed because no damage was stored" (),
    // PS: data/abilities.ts compoundeyes onSourceModifyAccuracy
    CompoundEyesAccuracy => "compoundeyes - enhancing accuracy" (),
    // PS: sim/battle-actions.ts getSpreadDamage
    DamageCalculationInterrupted => "damage calculation interrupted" (),
    // PS: sim/battle.ts spreadDamage
    DamageEventFailed => "damage event failed" (),
    // PS: data/mods/gen1/moves.ts disable.condition.onStart
    DisableDisablingMove => "Disable: disabling move {} in slot {}" (Move, Int),
    // PS: data/abilities.ts serenegrace onModifyMove
    DoublingSecondaryChance => "doubling secondary chance" (),
    // PS: sim/battle-actions.ts runMoveEffects
    HealInterrupted => "heal interrupted" (),
    // PS: sim/pokemon.ts setStatus
    ImmuneToStatus => "immune to status" (),
    // PS: sim/pokemon.ts addVolatile
    ImmuneToVolatileStatus => "immune to volatile status" (),
    // PS: sim/battle.ts runEvent, singleEvent (`eventid`)
    HandlerSuppressedByAirLock => "{} handler suppressed by Air Lock" (Word),
    // PS: data/moves.ts attract volatileStatus onStart
    IncompatibleGender => "incompatible gender" (),
    // PS: sim/battle-actions.ts runMoveEffects
    MoveFailedNothing => "move failed because it did nothing" (),
    // PS: sim/pokemon.ts runStatusImmunity
    NaturalStatusImmunity => "natural status immunity" (),
    // PS: sim/battle-actions.ts getDamage (crit ignores boosts)
    NegatingAtkBoost => "Negating (sp)atk boost/penalty." (),
    NegatingDefBoost => "Negating (sp)def boost/penalty." (),
    NotAGen1Mechanic => "not a gen 1 mechanic" (),
    NothingToLeechInto => "Nothing to leech into" (),
    // PS: data/conditions.ts raindance onWeatherModifyDamage
    RainFireSuppress => "rain fire suppress" (),
    RainWaterBoost => "rain water boost" (),
    // PS: data/moves.ts attract condition onUpdate (`${pokemon}`)
    RemovingAttractVolatileOn => "Removing Attract volatile on {}" (Mon),
    // PS: data/moves.ts destinybond condition onBeforeMove
    RemovingDestinyBond => "removing Destiny Bond before attack" (),
    // PS: data/mods/gen1/scripts.ts getDamage
    ScreenDoublingDef => "Screen doubling (Sp)Def" (),
    // PS: sim/pokemon.ts setStatus (`status.id`)
    SetStatusInterrupted => "set status [{}] interrupted" (Status),
    // PS: data/abilities.ts shielddust onModifySecondaries
    ShieldDustPrevent => "Shield Dust prevent secondary" (),
    // PS: sim/pokemon.ts setStatus (`status.id`)
    StatusStartInterrupted => "status start [{}] interrupted" (Status),
    // PS: data/conditions.ts stall onStallMove
    SuccessChance => "Success chance: {}%" (Int),
    // PS: data/conditions.ts sunnyday onWeatherModifyDamage
    SunnyDayFireBoost => "Sunny Day fire boost" (),
    SunnyDayWaterSuppress => "Sunny Day water suppress" (),
    // PS: data/abilities.ts swarm / torrent onModifyAtk, onModifySpA
    SwarmBoost => "Swarm boost" (),
    TorrentBoost => "Torrent boost" (),
    // PS: sim/battle.ts spreadDamage
    WeatherImmunity => "weather immunity" (),
    // PS: data/abilities.ts wonderguard onTryHit (`move.id`)
    WonderGuardImmunity => "Wonder Guard immunity: {}" (MoveId),
    // ---- -message (PS: sim/battle.ts maybeTriggerEndlessBattleClause;
    // data/rulesets.ts freezeclausemod, sleepclausemod) ----------------
    EndlessBattleNoProgress => "This battle cannot progress. Endless Battle Clause activated!" (),
    FreezeClauseActivated => "Freeze Clause activated." (),
    // PS: data/mods/gen1/conditions.ts invulnerability onInvulnerability
    InvulnerableTarget => "The foe {} can't be hit while invulnerable!" (Nickname),
    SleepClauseActivated => "Sleep Clause Mod activated." (),
    // ---- message and bigerror (PS: sim/battle.ts
    // maybeTriggerEndlessBattleClause, the turn limit; start, the EV check)
    AutoTieIn => "You will auto-tie if the battle doesn't end in {} turns (on turn 1000)." (Int),
    AutoTieIn1 => "You will auto-tie if the battle doesn't end in 1 turn (on turn 1000)." (),
    EvLimitWarning => "Warning: One player isn't adhering to a 510 EV limit, and the other player is." (),
    TurnLimit => "It is turn 1000. You have hit the turn limit!" (),
    // ---- -hint (PS: sim/battle.ts hint, from the handlers and gen 1's
    // scripts.ts) -------------------------------------------------------
    DesyncClauseActivated => "Desync Clause Mod activated!" (),
    HintDefenseDivZero => "Pokemon Showdown avoids division by zero by rounding defense up to 1. In game, the battle would have crashed." (),
    // PS: data/conditions.ts futuremove.onEnd (`${move.name} did not hit because the target is ...`)
    HintFutureMoveTargetFainted => "{} did not hit because the target is fainted." (Move),
    HintFutureMoveTargetUser => "{} did not hit because the target is the user." (Move),
    HintGen1BoostAt999 => "In Gen 1, boosting a stat at 999 will apply a -1 drop afterwards, and the stat remains at 999." (),
    HintGen1CannotMoveSleep => "In Gen 1, if a Pokémon spends a turn partially trapped and switches to a Pokémon that is asleep, the sleep counter will not decrease until you select a move with a different Pokémon." (),
    HintGen1Counter => "In Gen 1, from the Counter user's perspective, Counter uses the last announced move by the target's team to determine if it will succeed." (),
    HintGen1CounterSleep => "In Gen 1, if Counter is used against a target that switched in and spent the turn sleeping, from the Counter user's perspective, it will fail if the move in the target's first slot is also Counter." (),
    HintGen1DesyncDefault => "In Gen 1, if both players would see the same Pokémon using different moves, the Pokemon defaults to the move shown from the perspective of the player controlling that Pokémon." (),
    HintGen1DirectDamageNoSub => "In Gen 1, if a Pokemon with a Substitute hurts itself due to confusion or Jump Kick/Hi Jump Kick recoil and the target does not have a Substitute there is no damage dealt." (),
    HintGen1DirectDamageSub => "In Gen 1, if a Pokemon with a Substitute hurts itself due to confusion or Jump Kick/Hi Jump Kick recoil and the target has a Substitute, the target's Substitute takes the damage." (),
    HintGen1DropAt1 => "In Gen 1, dropping a stat at 1 will apply a +1 boost afterwards, and the stat remains at 1." (),
    HintGen1ExplosionSub => "In Gen 1, the user of Explosion will not take damage if it breaks a Substitute." (),
    HintGen1HazeBide => "In Gen 1, when Haze cures the sleep/freeze status of a Pokémon during Bide, the move execution will never resolve." (),
    HintGen1HazeLockedMove => "In Gen 1, when Haze cures the sleep/freeze status of a Pokémon during a multi-turn move, that Pokémon will become soft-locked." (),
    HintGen1HyperBeamSleep => "In Gen 1, if a Pokémon that has just used Hyper Beam and has yet to recharge is targeted with a sleep inducing move, any other status it may already have will be ignored and sleep will be induced regardless." (),
    HintGen1LeechSeedRecovery => "In Gen 1, Leech Seed recovery is not limited by the remaining HP of the seeded Pokemon." (),
    HintGen1LeechSeedToxic => "In Gen 1, Leech Seed's damage is affected by Toxic's counter." (),
    HintGen1Miss => "In Gen 1, moves with 100% accuracy can still miss 1/256 of the time." (),
    HintGen1NoMove => "In Gen 1, if a Pokémon is thawed from freeze and no move has been selected yet, the Pokémon will use a move with Fissure's animation, 102 base power, ??? type, Special category, and around 31.6% accuracy." (),
    HintGen1ParSemiInvulnerable => "In Gen 1, when a Dig/Fly user is fully paralyzed while semi-invulnerable, it will remain semi-invulnerable until it switches out or fully executes Dig/Fly" (),
    HintGen1PartialTrapRecharge => "In Gen 1, partial trapping moves negate the recharge turn of Hyper Beam, even if they miss." (),
    HintGen1PpUnderflow => "In Gen 1, if a Pokémon is forced to use a move with 0 PP, the move will underflow to have 63 PP." (),
    HintGen1PsywaveSoftlock => "In Gen 1, if a Pokémon at level 0, 1 or 171 uses Psywave, the game softlocks." (),
    HintGen1RageBuild => "In Gen 1, using {} causes the target to build Rage, even if it misses or fails" (Move),
    HintGen1Recovery => "In Gen 1, recovery moves fail if (user's maximum HP - user's current HP + 1) is divisible by 256, unless the current hp is also divisible by 256." (),
    HintGen1SelfDestructSub => "In Gen 1, the user of Self-Destruct will not take damage if it breaks a Substitute." (),
    HintGen1StatRollover => "In Gen 1, a stat will roll over to a small number if it is larger than 1024." (),
    HintGen1SubConfusion => "In Gen 1, moves that inflict confusion as a secondary effect can confuse targets with a Substitute, as long as the move does not break the Substitute." (),
    HintGen1ToxicCounter => "In Gen 1, Toxic's counter is retained after Rest and applies to PSN/BRN." (),
    HintGen3IntimidateSub => "In Gen 3, Intimidate does not activate if every target has a Substitute." (),
    // PS: data/mods/gen4/moves.ts knockoff (gen 4 only since 41898849e)
    HintGens3To4KnockOff => "In Gens 3-4, Knock Off only makes the target's item unusable; it cannot obtain a new item." (),
    // PS: sim/battle-actions.ts switchIn (gens <= 4, an item knocked off)
    HintGens3To4KnockOffSwitchIn => "In Gens 3-4, Knock Off makes a Pokémon lose its held item every time it switches in." (),
    // PS: sim/battle-actions.ts switchIn (a Pursuit faint, gens 2-4)
    HintPursuitSwitch => "Previously chosen switches continue in Gen 2-4 after a Pursuit target faints." (),
    // PS: data/rulesets.ts sleepclausemod onSetStatus
    HintSleepClause => "Sleep Clause Mod prevents players from putting more than one of their opponent's Pokemon to sleep at a time" (),
    // PS: sim/battle-actions.ts switchIn
    HintSwitchFailed => "A switch failed because the Pokémon trying to switch in is already in." (),
    // ---- raw (PS: sim/battle.ts start, the custom-rules infobox) -------
    CustomRules1 => "<div class=\"infobox\"><details class=\"readmore\" open><summary><strong>1 custom rule:</strong></summary> {}</details></div>" (Rule),
    CustomRules2 => "<div class=\"infobox\"><details class=\"readmore\" open><summary><strong>2 custom rules:</strong></summary> {}, {}</details></div>" (Rule, Rule),
    CustomRules3 => "<div class=\"infobox\"><details class=\"readmore\" open><summary><strong>3 custom rules:</strong></summary> {}, {}, {}</details></div>" (Rule, Rule, Rule),
    // ---- error (PS: sim/side.ts emitChoiceError, after the `[Invalid
    // choice] ` / `[Unavailable choice] ` prefix) ------------------------
    CantDoGameOver => "Can't do anything: The game is over" (),
    CantDoNotYourTurn => "Can't do anything: It's not your turn" (),
    CantMoveDisabled => "Can't move: {}'s {} is disabled" (Nickname, Move),
    CantMoveNeedResponse => "Can't move: You need a {} response" (Word),
    CantMoveNoMoveIndex => "Can't move: Your {} doesn't have a move {}" (Nickname, Int),
    CantMoveTooMany => "Can't move: You sent more choices than unfainted Pokémon." (),
    CantPassMustMove => "Can't pass: Your {} must make a move (or switch)" (Nickname),
    CantPassNotMoveOrSwitch => "Can't pass: Not a move or switch request" (),
    CantPassReplace => "Can't pass: You need to switch in a Pokémon to replace {}" (Nickname),
    CantSwitchActive => "Can't switch: You can't switch to an active Pokémon" (),
    CantSwitchFainted => "Can't switch: You can't switch to a fainted Pokémon" (),
    CantSwitchNeedResponse => "Can't switch: You need a {} response" (Word),
    CantSwitchNoName => "Can't switch: You do not have a Pokémon named \"{}\" to switch to" (Int),
    CantSwitchNoSlot => "Can't switch: You do not have a Pokémon in slot {} to switch to" (Int),
    CantSwitchOnce => "Can't switch: The Pokémon in slot {} can only switch in once" (Int),
    CantSwitchSelect => "Can't switch: You need to select a Pokémon to switch in" (),
    CantSwitchTooMany => "Can't switch: You sent more choices than unfainted Pokémon" (),
    CantSwitchTooManySwitches => "Can't switch: You sent more switches than Pokémon that need to switch" (),
    CantSwitchTrapped => "Can't switch: The active Pokémon is trapped" (),
    CantUndoLeak => "Can't undo: A trapping/disabling effect would cause undo to leak information" (),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{FormatId, RuleId};
    use core::mem::{align_of, size_of};

    #[test]
    fn msg_fits_24_bytes() {
        assert!(size_of::<Msg>() <= 24, "Msg is {} bytes", size_of::<Msg>());
        assert_eq!(align_of::<Msg>(), 4, "the u32 timestamp sets the alignment");
    }

    #[test]
    fn packed_parts_are_small() {
        assert_eq!(size_of::<Arg>(), 3);
        assert_eq!(align_of::<Arg>(), 1);
        assert_eq!(size_of::<Option<Arg>>(), 3);
        assert_eq!(size_of::<PokemonIdent>(), 3);
        assert_eq!(size_of::<Option<PokemonIdent>>(), 3);
        assert_eq!(size_of::<Kw>(), 8);
        assert_eq!(size_of::<Health>(), 10);
        assert_eq!(size_of::<TextArg>(), 5);
        // Three arguments (the three-rule custom-rules box); `Msg` stays 24.
        assert_eq!(size_of::<Text>(), 18);
        assert_eq!(size_of::<Details>(), 7);
        assert_eq!(align_of::<Details>(), 1);
    }

    #[test]
    fn arg_round_trips_every_kind() {
        let mon = PokemonIdent::active(SideId::P2, 5);
        let values = [
            ArgValue::Move(MoveNum(354)),
            ArgValue::MoveEffect(MoveNum(1)),
            ArgValue::MoveId(MoveNum(u16::MAX)),
            ArgValue::Item(ItemNum(77)),
            ArgValue::ItemEffect(ItemNum(0)),
            ArgValue::Ability(AbilityNum(12)),
            ArgValue::AbilityEffect(AbilityNum(76)),
            ArgValue::Species(SpeciesNum(386)),
            ArgValue::Type(TypeNum(17)),
            ArgValue::Status(Status::Tox),
            ArgValue::Stat(Stat::Evasion),
            ArgValue::StatName(Stat::Atk),
            ArgValue::Word(Word::Unboost),
            ArgValue::Mon(mon),
            ArgValue::Mon(PokemonIdent::inactive(SideId::P1, 0)),
            ArgValue::Side(SideId::P2),
            ArgValue::Int(-4),
            ArgValue::Int(i16::MAX),
        ];
        for v in values {
            assert_eq!(Arg::new(v).value(), v);
        }
    }

    #[test]
    fn text_arg_round_trips_every_kind() {
        let mon = PokemonIdent::active(SideId::P1, 3);
        let values = [
            TextArgValue::None,
            TextArgValue::Int(-123_456),
            TextArgValue::MoveId(MoveNum(7)),
            TextArgValue::Move(MoveNum(8)),
            TextArgValue::Mon(mon),
            TextArgValue::Nickname(mon),
            TextArgValue::Status(Status::Slp),
            TextArgValue::Word(Word::Move),
            TextArgValue::Rule(RuleId::FreezeClauseMod),
        ];
        for v in values {
            assert_eq!(TextArg::new(v).value(), v);
        }
    }

    #[test]
    fn words_parse_back() {
        for &w in Word::ALL {
            assert_eq!(Word::parse(w.as_str()), Some(w));
        }
        // Both spellings are PS's: `[from] Recoil` (applyRecoilDamage) and
        // `[from] recoil` (directDamage's Struggle recoil).
        assert_eq!(Word::parse("Recoil"), Some(Word::Recoil));
        assert_eq!(Word::parse("recoil"), Some(Word::StruggleRecoil));
        assert_eq!(Word::parse("RECOIL"), None);
    }

    #[test]
    fn templates_have_as_many_slots_as_args_fit() {
        for &t in TextId::ALL {
            let slots = t.template().matches("{}").count();
            assert!(slots <= 3, "{:?}", t);
            assert_eq!(slots, t.arg_kinds().len(), "{:?}: one kind per slot", t);
        }
    }

    #[test]
    fn templates_are_distinct() {
        for (i, &a) in TextId::ALL.iter().enumerate() {
            for &b in &TextId::ALL[i + 1..] {
                assert_ne!(a.template(), b.template(), "{:?} and {:?}", a, b);
            }
        }
    }

    #[test]
    fn kw_order_default_and_flags_first() {
        let of = PokemonIdent::active(SideId::P1, 0);
        let mut seen: [Option<KwPart>; 4] = [None; 4];
        let mut n = 0;
        let kw = Kw::from_of(Arg::status(Status::Brn), of)
            .with(KwFlags::SILENT | KwFlags::PARTIALLYTRAPPED);
        kw.encode_order(|p| {
            seen[n] = Some(p);
            n += 1;
        });
        assert_eq!(
            seen,
            [
                Some(KwPart::From(Arg::status(Status::Brn))),
                Some(KwPart::Of(of)),
                Some(KwPart::Flag("[partiallytrapped]")),
                Some(KwPart::Flag("[silent]")),
            ]
        );

        // -enditem|MON|ITEM|[silent]|[from] move: Trick
        let mut seen: [Option<KwPart>; 2] = [None; 2];
        let mut n = 0;
        let kw = Kw::from(Arg::move_effect(MoveNum(3))).with(KwFlags::SILENT | KwFlags::FLAGS_FIRST);
        kw.encode_order(|p| {
            seen[n] = Some(p);
            n += 1;
        });
        assert_eq!(seen, [Some(KwPart::Flag("[silent]")), Some(KwPart::From(Arg::move_effect(MoveNum(3))))]);

        // -heal|MON|HP|[from] move: Wish|[wisher] NAME
        let mut parts = 0;
        Kw { from: Some(Arg::move_effect(MoveNum(9))), of: Some(of), flags: KwFlags::WISHER }
            .encode_order(|p| {
                if parts == 1 {
                    assert_eq!(p, KwPart::Wisher(of));
                }
                parts += 1;
            });
        assert_eq!(parts, 2);
    }

    #[test]
    fn every_variant_has_a_distinct_kind() {
        let m = PokemonIdent::active(SideId::P1, 0);
        let f = PokemonIdent::active(SideId::P2, 1);
        let d = Details { species: SpeciesNum(1), level: 50, gender: crate::Gender::F, shiny: true, forme: 3 };
        let h = Health::split_percent(10, 20, None);
        let a = Arg::mv(MoveNum(1));
        let k = Kw::NONE;
        let t = Text::with1(TextId::BasePower, TextArg::int(70));
        let all = [
            Msg::Spacer,
            Msg::Timestamp(1_792_768_140),
            Msg::GameType(GameType::Singles),
            Msg::Player(SideId::P1),
            Msg::TeamSize { side: SideId::P2, size: 6 },
            Msg::Gen(3),
            Msg::Tier(FormatId::Gen3CustomGame),
            Msg::Rule(RuleId::HpPercentageMod),
            Msg::Raw(Text::with2(TextId::CustomRules2, TextArg::rule(RuleId::SleepClauseMod), TextArg::rule(RuleId::FreezeClauseMod))),
            Msg::Start,
            Msg::Upkeep,
            Msg::Turn(1),
            Msg::Win(SideId::P2),
            Msg::Tie,
            Msg::Debug(Text::with1(TextId::BasePower, TextArg::int(70))),
            Msg::Error { side: SideId::P1, unavailable: false, text: t },
            Msg::move_used(m, MoveNum(2), Some(f)),
            Msg::switch(m, d, h),
            Msg::Drag { mon: m, details: d, health: h, from: None },
            Msg::DetailsChange { mon: m, details: d, kw: k },
            Msg::FormeChange { mon: m, species: SpeciesNum(2), kw: k },
            Msg::Cant { mon: m, reason: Arg::status(Status::Par), mv: None },
            Msg::Faint(m),
            Msg::damage(m, h),
            Msg::heal(m, h),
            Msg::SetHp { mon: m, health: h, kw: k },
            Msg::status(m, Status::Brn),
            Msg::CureStatus { mon: m, status: Status::Slp, kw: Kw::flags(KwFlags::MSG) },
            Msg::CureTeam { mon: m, kw: k },
            Msg::boost_by(m, Stat::Atk, 1),
            Msg::boost_by(m, Stat::Atk, -1),
            Msg::SetBoost { mon: m, stat: Stat::Atk, amount: 6, kw: k },
            Msg::ClearAllBoost { kw: k },
            Msg::ClearBoost { mon: m, kw: k },
            Msg::ClearNegativeBoost { mon: m, kw: k },
            Msg::CopyBoost { source: m, target: f, kw: k },
            Msg::Crit(f),
            Msg::SuperEffective(f),
            Msg::Resisted(f),
            Msg::Immune { mon: f, kw: Kw::flags(KwFlags::OHKO) },
            Msg::Miss { source: m, target: None },
            Msg::fail(m),
            Msg::Block { mon: f, effect: a, mv: None, attacker: None, kw: k },
            Msg::NoTarget(None),
            Msg::HitCount { mon: f, count: 2 },
            Msg::Ohko,
            Msg::Nothing,
            Msg::activate(m, Arg::word(Word::Confusion)),
            Msg::start_effect(m, a),
            Msg::end_effect(m, a),
            Msg::SingleTurn { mon: m, effect: a, kw: k },
            Msg::SingleMove { mon: m, effect: a, kw: k },
            Msg::MustRecharge(m),
            Msg::Prepare { mon: m, mv: MoveNum(3), target: None },
            Msg::Anim { mon: m, mv: MoveNum(3), target: Some(f) },
            Msg::Transform { mon: m, into: f, kw: k },
            Msg::Weather { weather: Weather::RainDance, kw: Kw::flags(KwFlags::UPKEEP) },
            Msg::FieldStart { effect: a, kw: k },
            Msg::FieldEnd { effect: a, kw: k },
            Msg::FieldActivate { effect: a, kw: k },
            Msg::SideStart { side: SideId::P1, effect: a, kw: k },
            Msg::SideEnd { side: SideId::P1, effect: a, kw: k },
            Msg::Item { mon: m, item: ItemNum(1), kw: k },
            Msg::EndItem { mon: m, item: ItemNum(1), kw: Kw::flags(KwFlags::EAT) },
            Msg::Ability { mon: m, ability: AbilityNum(1), old: None, boost: true, kw: k },
            Msg::EndAbility { mon: m, kw: k },
            Msg::Message(t),
            Msg::Hint { text: t, split: None },
        ];
        for (i, x) in all.iter().enumerate() {
            for y in &all[i + 1..] {
                if x.kind() == y.kind() {
                    panic!("two variants share kind {:?}", x.kind());
                }
            }
        }
        assert_eq!(all.len(), 68);
    }

    #[test]
    fn constructors_build_the_expected_lines() {
        let me = PokemonIdent::active(SideId::P1, 0);
        let foe = PokemonIdent::active(SideId::P2, 2);
        let m = Msg::move_used(me, MoveNum(85), Some(foe));
        assert_eq!(m.kind(), "move");
        assert_eq!(m.kw(), Some(&Kw::NONE));

        let d = Msg::damage(foe, Health::split_percent(150, 300, None));
        assert_eq!(d.kind(), "-damage");
        assert_eq!(d.split_side(), Some(SideId::P2));

        let plain = Msg::damage(foe, Health::plain(crate::HpStatus::exact(50, 100, None)));
        assert_eq!(plain.split_side(), None);

        assert_eq!(
            Msg::boost_by(me, Stat::Spe, -2),
            Msg::Unboost { mon: me, stat: Stat::Spe, amount: 2, kw: Kw::NONE }
        );
        assert_eq!(
            Msg::boost_by(me, Stat::Atk, 1),
            Msg::Boost { mon: me, stat: Stat::Atk, amount: 1, kw: Kw::NONE }
        );

        let still = m.with_kw(Kw::flags(KwFlags::STILL));
        assert!(still.kw().unwrap().flags.contains(KwFlags::STILL));
        assert_eq!(Msg::Tie.with_kw(Kw::flags(KwFlags::STILL)), Msg::Tie);

        let hint = Msg::Hint { text: Text::new(TextId::HintGen1Miss), split: Some(SideId::P1) };
        assert_eq!(hint.split_side(), Some(SideId::P1));
    }
}
