//! Encoder and decoder for PS sim-protocol lines.
//!
//! Source of truth: `sim/SIM-PROTOCOL.md`, `sim/battle.ts` (`add`, `addSplit`,
//! `attrLastMove`, `getRequests`), `sim/pokemon.ts` (`getHealth`,
//! `getUpdatedDetails`), `sim/teams.ts` (`pack`/`unpack`).
//!
//! Per-line round-trip: feed every recorded line to [`decode_line`]; the
//! resulting [`Msg`] re-encoded by [`encode_msg`] must be identical bytes.
//!
//! The text codec is hand-rolled: PS's JSON output keeps a specific key
//! order, the line grammar moves kwargs in PS's call order, and the names
//! (`move`, `item`, etc.) come from the dex we play with (`ps_dex`, later).

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use crate::command::{FormatId, RuleId};
use crate::ident::{
    AbilityNum, Details, Gender, Health, Hp, PokemonIdent, SideId, SpeciesNum, Stat, Status,
};
use crate::msg::{
    Arg, ArgValue, GameType, Kw, KwFlags, KwPart, Msg, Text, TextArg, TextArgKind, TextId, Weather, Word,
};
use crate::{BattleNames, DexNames, Name};

/// A codec error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodecError {
    Empty,
    UnknownKind(String),
    BadTrailingArg(usize),
    BadArg(String),
    BadHp(String),
    BadStatus(String),
    BadStat(String),
    BadCmd(String),
    BadTeam(String),
    UnsupportedLine(String),
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodecError::Empty => f.write_str("empty line"),
            CodecError::UnknownKind(k) => write!(f, "unknown line kind {:?}", k),
            CodecError::BadTrailingArg(i) => write!(f, "trailing arg {} not in field count", i),
            CodecError::BadArg(s) => write!(f, "bad arg: {}", s),
            CodecError::BadHp(s) => write!(f, "bad hp: {:?}", s),
            CodecError::BadStatus(s) => write!(f, "bad status: {:?}", s),
            CodecError::BadStat(s) => write!(f, "bad stat: {:?}", s),
            CodecError::BadCmd(s) => write!(f, "bad command: {:?}", s),
            CodecError::BadTeam(s) => write!(f, "bad team: {}", s),
            CodecError::UnsupportedLine(s) => write!(f, "unsupported line {:?}", s),
        }
    }
}

impl core::error::Error for CodecError {}

// ============================================================================
// DEX + BATTLE SHIMS
// ============================================================================

fn render_text_to_str(t: crate::msg::Text, dex: &dyn DexNames, names: &dyn BattleNames) -> String {
    
    let tmpl = t.id.template();
    let mut out = String::with_capacity(tmpl.len());
    let mut last = 0usize;
    let mut i = 0usize;
    while let Some(pos) = tmpl[last..].find("{}") {
        out.push_str(&tmpl[last..last + pos]);
        out.push_str(&render_textarg(t.args.get(i).copied().unwrap_or(crate::msg::TextArg::NONE), dex, names));
        last = last + pos + 2;
        i += 1;
    }
    out.push_str(&tmpl[last..]);
    out
}

fn render_textarg(a: crate::msg::TextArg, dex: &dyn DexNames, names: &dyn BattleNames)
    -> String
{
    use crate::msg::TextArgValue as V;
    match a.value() {
        V::None => String::new(),
        V::Int(n) => n.to_string(),
        V::MoveId(m) => dex.move_id(m).to_string(),
        V::Move(m) => dex.move_name(m).to_string(),
        V::Mon(m) => mon_str(m, names),
        V::Nickname(m) => nickname_only(m, names),
        V::Status(s) => s.as_str().to_string(),
        V::Word(w) => w.as_str().to_string(),
        V::Rule(r) => r.name().to_string(),
        V::Repeal(r) => alloc::format!("!{}", r.name()),
    }
}



/// A `DexNames` and `BattleNames` rolled into one, for unit tests and the
/// round-trip harness (`ps_dex` provides the real one).
pub struct CodecState<N: DexNames>(pub N, pub Battle);

impl<N: DexNames> CodecState<N> {
    pub fn new(dex: N) -> Self {
        Self(dex, Battle::new())
    }
    pub fn dex(&self) -> &N { &self.0 }
    pub fn battle(&self) -> &Battle { &self.1 }
    pub fn battle_mut(&mut self) -> &mut Battle { &mut self.1 }
    /// The dex and the mutable name table at once (for [`super::json::decode_request`]).
    pub fn split_mut(&mut self) -> (&N, &mut Battle) { (&self.0, &mut self.1) }
}

impl<N: DexNames> DexNames for CodecState<N> {
    fn move_name(&self, m: crate::ident::MoveNum) -> &str { self.0.move_name(m) }
    fn move_id(&self, m: crate::ident::MoveNum) -> &str { self.0.move_id(m) }
    fn move_from_id(&self, id: &str) -> Option<crate::ident::MoveNum> { self.0.move_from_id(id) }
    fn species_name(&self, s: SpeciesNum) -> &str { self.0.species_name(s) }
    fn species_from_id(&self, id: &str) -> Option<SpeciesNum> { self.0.species_from_id(id) }
    fn cosmetic_forme_name(&self, s: SpeciesNum, forme: u8) -> &str { self.0.cosmetic_forme_name(s, forme) }
    fn cosmetic_forme_from_id(&self, id: &str) -> Option<(SpeciesNum, u8)> { self.0.cosmetic_forme_from_id(id) }
    fn item_name(&self, i: crate::ident::ItemNum) -> &str { self.0.item_name(i) }
    fn item_id(&self, i: crate::ident::ItemNum) -> &str { self.0.item_id(i) }
    fn item_from_id(&self, id: &str) -> Option<crate::ident::ItemNum> { self.0.item_from_id(id) }
    fn ability_name(&self, a: AbilityNum) -> &str { self.0.ability_name(a) }
    fn ability_id(&self, a: AbilityNum) -> &str { self.0.ability_id(a) }
    fn ability_from_id(&self, id: &str) -> Option<AbilityNum> { self.0.ability_from_id(id) }
    fn type_name(&self, t: crate::ident::TypeNum) -> &str { self.0.type_name(t) }
    fn type_from_id(&self, id: &str) -> Option<crate::ident::TypeNum> { self.0.type_from_id(id) }
    fn nature_name(&self, n: crate::ident::NatureNum) -> &str { self.0.nature_name(n) }
    fn nature_from_id(&self, id: &str) -> Option<crate::ident::NatureNum> { self.0.nature_from_id(id) }
}

// ============================================================================
// Encoder
// ============================================================================

/// Encode one [`Msg`] to a single line.
pub fn encode_msg(msg: &Msg, dex: &dyn DexNames, names: &dyn BattleNames) -> String {
    let mut out = String::new();
    push_kind(&msg.kind(), &mut out);
    push_positional_args(msg, dex, names, &mut out);
    push_kwargs(msg.kw_or_empty(), dex, names, &mut out);
    out
}

/// An `update` block's lines as `BattleStream` writes them, joined by `\n`:
/// a split line as `|split|pN`, the secret form, then the shared form; a
/// one-sided hint as `|split|pN`, the hint, then an empty line (PS
/// `sim/battle.ts` `addSplit`, which `BattleStream` passes on unchanged).
pub fn encode_log(msgs: &[Msg], dex: &dyn DexNames, names: &dyn BattleNames) -> String {
    let enc = |m: &Msg| encode_msg(m, dex, names);
    let mut out: Vec<String> = Vec::new();
    for m in msgs {
        match m.split_side() {
            Some(side) => {
                out.push(format!("|split|{}", side.as_str()));
                match m {
                    Msg::Hint { text, .. } => {
                        out.push(enc(&Msg::Hint { text: *text, split: None }));
                        out.push(String::new());
                    }
                    _ => {
                        out.push(enc(&with_health(m, Health::secret)));
                        out.push(enc(&with_health(m, Health::shared)));
                    }
                }
            }
            None => out.push(enc(m)),
        }
    }
    out.join("\n")
}

/// `m`, an HP-bearing line, with its HP in one form, as one viewer gets it.
fn with_health(m: &Msg, form: impl Fn(&Health) -> crate::ident::HpStatus) -> Msg {
    let mut m = *m;
    match &mut m {
        Msg::Switch { health, .. }
        | Msg::Drag { health, .. }
        | Msg::Damage { health, .. }
        | Msg::Heal { health, .. }
        | Msg::SetHp { health, .. } => *health = Health::plain(form(health)),
        _ => unreachable!("only HP lines split"),
    }
    m
}

fn push_kind(kind: &str, buf: &mut String) {
    buf.push('|');
    buf.push_str(kind);
}

fn push_positional_args(msg: &Msg, dex: &dyn DexNames, names: &dyn BattleNames, buf: &mut String) {
    match msg {
        Msg::Spacer | Msg::Start | Msg::Upkeep | Msg::Tie
        | Msg::Ohko | Msg::Nothing => {}

        Msg::Timestamp(n) => push_arg(buf, &n.to_string()),
        Msg::GameType(g) => push_arg(buf, g.as_str()),
        Msg::Player(side) => {
            push_arg(buf, side.as_str());
            push_arg(buf, names.player_name(*side));
            push_arg(buf, names.player_avatar(*side));
            push_arg(buf, names.player_rating(*side));
        }
        Msg::TeamSize { side, size } => {
            push_arg(buf, side.as_str());
            push_arg(buf, &size.to_string());
        }
        Msg::Split(side) => push_arg(buf, side.as_str()),
        Msg::Gen(n) => push_arg(buf, &n.to_string()),
        Msg::Tier(f) => push_arg(buf, f.name()),
        Msg::Rule(r) => if let Some(s) = r.line() { push_arg(buf, s); },
        Msg::Raw(t) => push_arg(buf, &render_text_to_str(*t,  dex, names)),
        Msg::Turn(n) => push_arg(buf, &n.to_string()),
        Msg::Win(side) => push_arg(buf, names.player_name(*side)),
        Msg::Debug(t) => push_arg(buf, &render_text_to_str(*t,  dex, names)),
        Msg::Notice(t) | Msg::BigError(t) => push_arg(buf, &render_text_to_str(*t, dex, names)),
        Msg::Error { unavailable, text, .. } => {
            let prefix = if *unavailable { "[Unavailable choice] " } else { "[Invalid choice] " };
            push_arg(buf, &format!("{}{}", prefix, render_text_to_str(*text, dex, names)));
        }

        Msg::Move { user, mv, target, kw: _ } => {
            push_arg(buf, &mon_str(*user, names));
            push_arg(buf, dex.move_name(*mv));
            push_arg(buf, &target.map(|t| mon_str(t, names)).unwrap_or_default());
        }
        Msg::Switch { mon, details, health, from }
        | Msg::Drag { mon, details, health, from } => {
            push_arg(buf, &mon_str(*mon, names));
            push_arg(buf, &details_str(*details, dex, names));
            push_arg(buf, &hp_status_str(*health, false));
            // `[from] EFFECT` is part of the positional layout (PS writes it
            // before any flags).
            if let Some(f) = from {
                push_arg(buf, &arg_kw_str(*f, dex, names));
            }
        }
        Msg::DetailsChange { mon, details, kw: _ } => {
            push_arg(buf, &mon_str(*mon, names));
            push_arg(buf, &details_str(*details, dex, names));
        }
        Msg::FormeChange { mon, species, kw: _ } => {
            push_arg(buf, &mon_str(*mon, names));
            push_arg(buf, dex.species_name(*species));
        }
        Msg::Cant { mon, reason, mv } => {
            push_arg(buf, &mon_str(*mon, names));
            push_arg(buf, &arg_pos_str(*reason, dex));
            if let Some(m) = mv {
                push_arg(buf, &arg_pos_str(*m, dex));
            }
        }
        Msg::Faint(mon) => push_arg(buf, &mon_str(*mon, names)),

        Msg::Damage { mon, health, kw: _ }
        | Msg::Heal { mon, health, kw: _ }
        | Msg::SetHp { mon, health, kw: _ } => {
            push_arg(buf, &mon_str(*mon, names));
            push_arg(buf, &hp_status_str(*health, true));
        }
        Msg::SetStatus { mon, status, kw: _ } | Msg::CureStatus { mon, status, kw: _ } => {
            push_arg(buf, &mon_str(*mon, names));
            push_arg(buf, status.as_str());
        }
        Msg::CureTeam { mon, kw: _ } => push_arg(buf, &mon_str(*mon, names)),

        Msg::Boost { mon: bmon, stat, amount, kw: _ } => {
            push_arg(buf, &mon_str(*bmon, names));
            push_arg(buf, stat.abbr());
            push_arg(buf, &amount.to_string());
        }
        Msg::Unboost { mon, stat, amount, kw: _ } => {
            push_arg(buf, &mon_str(*mon, names));
            push_arg(buf, stat.abbr());
            push_arg(buf, &amount.to_string());
        }
        Msg::SetBoost { mon, stat, amount, kw: _ } => {
            push_arg(buf, &mon_str(*mon, names));
            push_arg(buf, stat.abbr());
            push_arg(buf, &amount.to_string());
        }
        Msg::ClearAllBoost { kw: _ } => {}
        Msg::ClearBoost { mon, kw: _ } | Msg::ClearNegativeBoost { mon, kw: _ } => {
            push_arg(buf, &mon_str(*mon, names));
        }
        Msg::CopyBoost { source, target, kw: _ } => {
            push_arg(buf, &mon_str(*source, names));
            push_arg(buf, &mon_str(*target, names));
        }

        Msg::Crit(mon) | Msg::SuperEffective(mon) | Msg::Resisted(mon) => {
            push_arg(buf, &mon_str(*mon, names));
        }
        Msg::Immune { mon, kw: _ } => push_arg(buf, &mon_str(*mon, names)),
        Msg::Miss { source, target } => {
            push_arg(buf, &mon_str(*source, names));
            if let Some(t) = target {
                push_arg(buf, &mon_str(*t, names));
            }
        }
        Msg::Fail { mon, what, what2, kw: _ } => {
            push_arg(buf, &mon_str(*mon, names));
            if let Some(w) = what {
                push_arg(buf, &arg_pos_str(*w, dex));
            }
            if let Some(w) = what2 {
                push_arg(buf, &arg_pos_str(*w, dex));
            }
        }
        Msg::Block { mon, effect, mv, attacker, kw: _ } => {
            push_arg(buf, &mon_str(*mon, names));
            push_arg(buf, &arg_pos_str(*effect, dex));
            if let Some(m) = mv {
                push_arg(buf, dex.move_name(*m));
            }
            if let Some(a) = attacker {
                push_arg(buf, &mon_str(*a, names));
            }
        }
        Msg::NoTarget(mon) => {
            if let Some(m) = mon {
                push_arg(buf, &mon_str(*m, names));
            }
        }
        Msg::HitCount { mon, count } => {
            push_arg(buf, &mon_str(*mon, names));
            push_arg(buf, &count.to_string());
        }

        Msg::Activate { mon, effect, a, b, kw: _ } => {
            if let Some(m) = mon {
                push_arg(buf, &mon_str(*m, names));
            }
            push_arg(buf, &arg_pos_str(*effect, dex));
            if let Some(a) = a {
                push_arg(buf, &arg_pos_str(*a, dex));
            }
            if let Some(b) = b {
                push_arg(buf, &arg_pos_str(*b, dex));
            }
        }
        Msg::StartEffect { mon, effect, a, b, kw: _ } => {
            push_arg(buf, &mon_str(*mon, names));
            push_arg(buf, &arg_pos_str(*effect, dex));
            if let Some(a) = a {
                push_arg(buf, &arg_pos_str(*a, dex));
            }
            if let Some(b) = b {
                push_arg(buf, &arg_pos_str(*b, dex));
            }
        }
        Msg::EndEffect { mon, effect, kw: _ } => {
            push_arg(buf, &mon_str(*mon, names));
            push_arg(buf, &arg_pos_str(*effect, dex));
        }
        Msg::SingleTurn { mon, effect, kw: _ } | Msg::SingleMove { mon, effect, kw: _ } => {
            push_arg(buf, &mon_str(*mon, names));
            push_arg(buf, &arg_pos_str(*effect, dex));
        }
        Msg::MustRecharge(mon) => push_arg(buf, &mon_str(*mon, names)),
        Msg::Prepare { mon, mv, target } | Msg::Anim { mon, mv, target } => {
            push_arg(buf, &mon_str(*mon, names));
            push_arg(buf, dex.move_name(*mv));
            if let Some(t) = target {
                push_arg(buf, &mon_str(*t, names));
            }
        }
        Msg::Transform { mon, into, kw: _ } => {
            push_arg(buf, &mon_str(*mon, names));
            push_arg(buf, &mon_str(*into, names));
        }

        Msg::Weather { weather, kw: _ } => push_arg(buf, weather.as_str()),
        Msg::FieldStart { effect, kw: _ }
        | Msg::FieldEnd { effect, kw: _ }
        | Msg::FieldActivate { effect, kw: _ } => {
            push_arg(buf, &arg_pos_str(*effect, dex));
        }
        Msg::SideStart { side, effect, kw: _ } | Msg::SideEnd { side, effect, kw: _ } => {
            push_arg(buf, &side_full_name(*side, names));
            push_arg(buf, &arg_pos_str(*effect, dex));
        }

        Msg::Item { mon, item, kw: _ } | Msg::EndItem { mon, item, kw: _ } => {
            push_arg(buf, &mon_str(*mon, names));
            push_arg(buf, dex.item_name(*item));
        }
        Msg::Ability { mon, ability, old, boost, kw: _ } => {
            push_arg(buf, &mon_str(*mon, names));
            push_arg(buf, dex.ability_name(*ability));
            if let Some(old) = old {
                push_arg(buf, dex.ability_name(*old));
            } else if *boost {
                push_arg(buf, Word::Boost.as_str());
            }
        }
        Msg::EndAbility { mon, kw: _ } => push_arg(buf, &mon_str(*mon, names)),

        Msg::Message(text) => push_arg(buf, &render_text_to_str(*text,  dex, names)),
        Msg::Hint { text, split: _ } => push_arg(buf, &render_text_to_str(*text,  dex, names)),
    }
}

fn push_arg(buf: &mut String, s: &str) {
    buf.push('|');
    buf.push_str(s);
}

fn push_kwargs(kw: Option<&Kw>, dex: &dyn DexNames, names: &dyn BattleNames, buf: &mut String) {
    let Some(kw) = kw else { return; };
    kw.encode_order(|part| match part {
        KwPart::From(a) => push_arg(buf, &arg_kw_str(a, dex, names)),
        KwPart::Of(mon) => push_arg(buf, &format!("[of] {}", mon_str(mon, names))),
        KwPart::Wisher(mon) => push_arg(buf, &format!("[wisher] {}", nickname_only(mon, names))),
        KwPart::Flag(text) => push_arg(buf, text),
    });
}

fn arg_pos_str(a: Arg, dex: &dyn DexNames) -> String {
    let mut s = String::new();
    arg_value_to_str(a.value(), dex, &mut s, /* for_kw */ false);
    s
}

fn arg_kw_str(a: Arg, dex: &dyn DexNames, names: &dyn BattleNames) -> String {
    let mut s = String::from("[from] ");
    arg_value_to_str(a.value(), dex, &mut s, /* for_kw */ true);
    if let Some(mon) = match a.value() {
        ArgValue::Mon(m) => Some(mon_active(m)),
        _ => None,
    } {
        // If we have a Mon, render fully as after `[from]`.
        s.push_str(&mon_str(mon, names));
    }
    s
}

fn mon_active(m: PokemonIdent) -> PokemonIdent {
    PokemonIdent { active: true, ..m }
}

fn mon_str(m: PokemonIdent, names: &dyn BattleNames) -> String {
    let mut s = String::with_capacity(32);
    s.push_str(m.side.as_str());
    if m.active {
        s.push('a');
    }
    s.push_str(": ");
    s.push_str(names.nickname(m.side, m.slot));
    s
}

fn nickname_only(m: PokemonIdent, names: &dyn BattleNames) -> String {
    names.nickname(m.side, m.slot).to_string()
}

fn side_full_name(side: SideId, names: &dyn BattleNames) -> String {
    format!("{}: {}", side.as_str(), names.player_name(side))
}

/// PS `getUpdatedDetails`.
pub(crate) fn details_str(d: Details, dex: &dyn DexNames, names: &dyn BattleNames) -> String {
    let mut s = String::new();
    if d.forme == 0 {
        s.push_str(dex.species_name(d.species));
    } else {
        s.push_str(dex.cosmetic_forme_name(d.species, d.forme));
    }
    if d.level != 100 {
        s.push_str(", L");
        s.push_str(&{ d.level }.to_string());
    }
    if let Some(g) = d.gender.details_letter() {
        s.push_str(", ");
        s.push_str(g);
    }
    if d.shiny {
        s.push_str(", shiny");
    }
    let _ = names;
    s
}

/// Format an `HpStatus`-like form: `HP` if HP is empty (0 fnt), else
/// `HP/MAX` and an optional space + status.
fn hp_status_str(h: Health, _is_split_form: bool) -> String {
    if h.secret == Hp::FAINTED {
        // At 0 HP PS writes `0 fnt` (no max).
        return "0 fnt".to_string();
    }
    let mut s = format!("{}/{}", h.secret.cur, h.secret.max);
    if let Some(st) = h.status {
        s.push(' ');
        s.push_str(st.as_str());
    }
    s
}

fn arg_value_to_str(v: ArgValue, dex: &dyn DexNames, s: &mut String, _for_kw: bool) {
    match v {
        ArgValue::Move(m) => s.push_str(dex.move_name(m)),
        ArgValue::MoveId(m) => s.push_str(dex.move_id(m)),
        ArgValue::MoveEffect(m) => {
            // MoveEffect is `move: <name>` regardless of context.
            s.push_str("move: ");
            s.push_str(dex.move_name(m));
        }
        ArgValue::Item(i) => s.push_str(dex.item_name(i)),
        ArgValue::ItemEffect(i) => {
            s.push_str("item: ");
            s.push_str(dex.item_name(i));
        }
        ArgValue::Ability(a) => s.push_str(dex.ability_name(a)),
        ArgValue::AbilityEffect(a) => {
            s.push_str("ability: ");
            s.push_str(dex.ability_name(a));
        }
        ArgValue::Species(sp) => s.push_str(dex.species_name(sp)),
        ArgValue::Type(t) => s.push_str(dex.type_name(t)),
        ArgValue::TypePair(a, b) => {
            s.push_str(dex.type_name(a));
            s.push('/');
            s.push_str(dex.type_name(b));
        }
        ArgValue::Status(st) => s.push_str(st.as_str()),
        ArgValue::Stat(st) => s.push_str(st.abbr()),
        ArgValue::StatName(st) => s.push_str(st.name()),
        ArgValue::Word(w) => s.push_str(w.as_str()),
        ArgValue::Mon(m) => s.push_str(&mon_str(m, &NoopNames)),
        ArgValue::Side(side) => s.push_str(side.as_str()),
        ArgValue::Int(n) => s.push_str(&n.to_string()),
    }
}

#[allow(dead_code)]
struct NoopNames;
impl BattleNames for NoopNames {
    fn player_name(&self, _: SideId) -> &str { "" }
    fn player_avatar(&self, _: SideId) -> &str { "" }
    fn player_rating(&self, _: SideId) -> &str { "" }
    fn nickname(&self, _: SideId, _: u8) -> &str { "" }
}

// ============================================================================
// DECODER
// ============================================================================

/// Decode one PS line to one [`Msg`].
pub fn decode_line(line: &str, state: &mut CodecState<impl DexNames>) -> Result<Msg, CodecError> {
    let mut parts = line.split('|');
    parts.next(); // empty before the leading bar
    let kind = parts.next().ok_or(CodecError::Empty)?.to_string();
    let args: Vec<&str> = parts.collect();
    // Disjoint-field borrows: `state.0` is the dex (immutable via the trait),
    // `state.1` is the Battle (mutable for nicknames). The compiler splits this
    // because the type `N` does not itself borrow from `state`.
    let dex = &state.0 as &dyn DexNames;
    let names = &mut state.1;
    decode_kind(&kind, &args, dex, names)
}

fn decode_kind(kind: &str, args: &[&str], dex: &dyn DexNames, names: &mut Battle) -> Result<Msg, CodecError> {
    match kind {
        "" => Ok(Msg::Spacer),
        "t:" => Ok(Msg::Timestamp(parse_u32(arg(args, 0)?, "t:")?)),
        "gametype" => Ok(Msg::GameType(parse_game_type(arg(args, 0)?)?)),
        "player" => {
            let side = parse_side(arg(args, 0)?)?;
            let pname = arg(args, 1)?;
            let avatar = args.get(2).copied().unwrap_or("");
            let rating = args.get(3).copied().unwrap_or("");
            names.set_player(side, pname, avatar, rating);
            Ok(Msg::Player(side))
        }
        "teamsize" => Ok(Msg::TeamSize { side: parse_side(arg(args, 0)?)?, size: parse_u8(arg(args, 1)?, "teamsize")? }),
        "split" => Ok(Msg::Split(parse_side(arg(args, 0)?)?)),
        "gen" => Ok(Msg::Gen(parse_u8(arg(args, 0)?, "gen")?)),
        "tier" => Ok(Msg::Tier(parse_tier(arg(args, 0)?)?)),
        "rule" => Ok(Msg::Rule(parse_rule(arg(args, 0)?)?)),
        "raw" => Ok(Msg::Raw(decode_text(&text_arg(args)?, dex, names)?)),
        "start" => Ok(Msg::Start),
        "upkeep" => Ok(Msg::Upkeep),
        "turn" => Ok(Msg::Turn(parse_u16(arg(args, 0)?, "turn")?)),
        "win" => Ok(Msg::Win(parse_win_side(arg(args, 0)?, names))),
        "tie" => Ok(Msg::Tie),
        "debug" => Ok(Msg::Debug(decode_text(&text_arg(args)?, dex, names)?)),
        "message" => Ok(Msg::Notice(decode_text(&text_arg(args)?, dex, names)?)),
        "bigerror" => Ok(Msg::BigError(decode_text(&text_arg(args)?, dex, names)?)),
        // The line does not name its side: PS sends it in a `sideupdate pN`
        // block, so the caller that read the header sets `side`.
        "error" => {
            let all = text_arg(args)?;
            let (unavailable, rest) = if let Some(rest) = all.strip_prefix("[Invalid choice] ") {
                (false, rest)
            } else if let Some(rest) = all.strip_prefix("[Unavailable choice] ") {
                (true, rest)
            } else {
                return Err(CodecError::BadArg(format!("error prefix {:?}", all)));
            };
            Ok(Msg::Error { side: SideId::P1, unavailable, text: decode_text(rest, dex, names)? })
        }
        k if k.starts_with('-') => decode_minor(k, args, dex, names),
        _ => decode_major(kind, args, dex, names),
    }
}

fn decode_minor(k: &str, args: &[&str], dex: &dyn DexNames, names: &mut Battle) -> Result<Msg, CodecError> {
    match k {
        "-damage" => Ok(Msg::Damage { mon: parse_mon(arg(args, 0)?, names)?, health: parse_health(arg(args, 1)?)?, kw: parse_kwargs(&args[2..], dex, names)? }),
        "-heal" => Ok(Msg::Heal { mon: parse_mon(arg(args, 0)?, names)?, health: parse_health(arg(args, 1)?)?, kw: parse_kwargs(&args[2..], dex, names)? }),
        "-sethp" => Ok(Msg::SetHp { mon: parse_mon(arg(args, 0)?, names)?, health: parse_health(arg(args, 1)?)?, kw: parse_kwargs(&args[2..], dex, names)? }),
        "-status" => Ok(Msg::SetStatus { mon: parse_mon(arg(args, 0)?, names)?, status: parse_status(arg(args, 1)?)?, kw: parse_kwargs(&args[2..], dex, names)? }),
        "-curestatus" => Ok(Msg::CureStatus { mon: parse_mon(arg(args, 0)?, names)?, status: parse_status(arg(args, 1)?)?, kw: parse_kwargs(&args[2..], dex, names)? }),
        "-cureteam" => Ok(Msg::CureTeam { mon: parse_mon(arg(args, 0)?, names)?, kw: parse_kwargs(&args[1..], dex, names)? }),
        "-boost" => Ok(Msg::Boost { mon: parse_mon(arg(args, 0)?, names)?, stat: parse_stat(arg(args, 1)?)?, amount: parse_u8(arg(args, 2)?, "-boost")?, kw: parse_kwargs(&args[3..], dex, names)? }),
        "-unboost" => Ok(Msg::Unboost { mon: parse_mon(arg(args, 0)?, names)?, stat: parse_stat(arg(args, 1)?)?, amount: parse_u8(arg(args, 2)?, "-unboost")?, kw: parse_kwargs(&args[3..], dex, names)? }),
        "-setboost" => Ok(Msg::SetBoost { mon: parse_mon(arg(args, 0)?, names)?, stat: parse_stat(arg(args, 1)?)?, amount: parse_i8(arg(args, 2)?, "-setboost")?, kw: parse_kwargs(&args[3..], dex, names)? }),
        "-clearallboost" => Ok(Msg::ClearAllBoost { kw: parse_kwargs(args, dex, names)? }),
        "-clearboost" => Ok(Msg::ClearBoost { mon: parse_mon(arg(args, 0)?, names)?, kw: parse_kwargs(&args[1..], dex, names)? }),
        "-clearnegativeboost" => Ok(Msg::ClearNegativeBoost { mon: parse_mon(arg(args, 0)?, names)?, kw: parse_kwargs(&args[1..], dex, names)? }),
        "-copyboost" => Ok(Msg::CopyBoost { source: parse_mon(arg(args, 0)?, names)?, target: parse_mon(arg(args, 1)?, names)?, kw: parse_kwargs(&args[2..], dex, names)? }),
        "-crit" => Ok(Msg::Crit(parse_mon(arg(args, 0)?, names)?)),
        "-supereffective" => Ok(Msg::SuperEffective(parse_mon(arg(args, 0)?, names)?)),
        "-resisted" => Ok(Msg::Resisted(parse_mon(arg(args, 0)?, names)?)),
        "-immune" => Ok(Msg::Immune { mon: parse_mon(arg(args, 0)?, names)?, kw: parse_kwargs(&args[1..], dex, names)? }),
        "-miss" => Ok(Msg::Miss { source: parse_mon(arg(args, 0)?, names)?, target: parse_opt_mon(arg_opt(args, 1), names) }),
        "-fail" => {
            let mon = parse_mon(arg(args, 0)?, names)?;
            let mut i = 1;
            let what = if let Some(s) = args.get(i).copied() {
                if s.is_empty() {
                    None
                } else {
                    let (a, used) = parse_arg_simple(s, dex)?;
                    if !used { None } else { Some(a) }
                }
            } else { None };
            if what.is_some() { i += 1; }
            let what2 = if let Some(s) = args.get(i).copied() {
                if s.is_empty() {
                    None
                } else {
                    let (a, used) = parse_arg_simple(s, dex)?;
                    if !used { None } else { Some(a) }
                }
            } else { None };
            if what2.is_some() { i += 1; }
            Ok(Msg::Fail { mon, what, what2, kw: parse_kwargs(&args[i..], dex, names)? })
        }
        "-block" => {
            let mon = parse_mon(arg(args, 0)?, names)?;
            let effect = parse_arg_simple(arg(args, 1)?, dex)?.0;
            let mut i = 2;
            let mv = if let Some(s) = args.get(i).copied() {
                if !s.is_empty() { i += 1; Some(dex.move_from_id(&id_str(s)).ok_or(CodecError::BadArg(format!("-block move {:?}", s)))?) } else { None }
            } else { None };
            let attacker = if let Some(s) = args.get(i).copied() {
                if !s.is_empty() { Some(parse_mon(s, names)?) } else { None }
            } else { None };
            if attacker.is_some() { i += 1; }
            Ok(Msg::Block { mon, effect, mv, attacker, kw: parse_kwargs(&args[i..], dex, names)? })
        }
        "-notarget" => Ok(Msg::NoTarget(if let Some(s) = args.first().copied() { if s.is_empty() { None } else { Some(parse_mon(s, names)?) } } else { None })),
        "-hitcount" => Ok(Msg::HitCount { mon: parse_mon(arg(args, 0)?, names)?, count: parse_u8(arg(args, 1)?, "-hitcount")? }),
        "-ohko" => Ok(Msg::Ohko),
        "-nothing" => Ok(Msg::Nothing),
        "-activate" => {
            let mut i = 0;
            let mon = if let Some(s) = args.first().copied() {
                if s.is_empty() { None } else { i = 1; Some(parse_mon(s, names)?) }
            } else { None };
            let effect = parse_arg_simple(arg(args, i)?, dex)?.0;
            i += 1;
            // Details name moves (`-activate|MON|move: Spite|Psychic|4`), never types.
            let a = if let Some(s) = args.get(i).copied() {
                let (ta, used) = parse_detail_arg(s, dex)?;
                if used { i += 1; Some(ta) } else { None }
            } else { None };
            let b = if let Some(s) = args.get(i).copied() {
                let (tb, used) = parse_detail_arg(s, dex)?;
                if used { i += 1; Some(tb) } else { None }
            } else { None };
            Ok(Msg::Activate { mon, effect, a, b, kw: parse_kwargs(&args[i..], dex, names)? })
        }
        "-start" => {
            let mon = parse_mon(arg(args, 0)?, names)?;
            let effect = parse_arg_simple(arg(args, 1)?, dex)?.0;
            // Only `typechange` details are types (`-start|MON|typechange|Psychic`);
            // other details are moves (`-start|MON|Mimic|Psychic`).
            let types = effect.value() == ArgValue::Word(Word::TypeChange);
            let detail = |s: &str| -> Result<(Arg, bool), CodecError> {
                if types { parse_arg_simple(s, dex) } else { parse_detail_arg(s, dex) }
            };
            let mut i = 2;
            let a = if let Some(s) = args.get(i).copied() {
                let (ta, used) = detail(s)?;
                if used { i += 1; Some(ta) } else { None }
            } else { None };
            let b = if let Some(s) = args.get(i).copied() {
                let (tb, used) = detail(s)?;
                if used { i += 1; Some(tb) } else { None }
            } else { None };
            Ok(Msg::StartEffect { mon, effect, a, b, kw: parse_kwargs(&args[i..], dex, names)? })
        }
        "-end" => Ok(Msg::EndEffect { mon: parse_mon(arg(args, 0)?, names)?, effect: parse_arg_simple(arg(args, 1)?, dex)?.0, kw: parse_kwargs(&args[2..], dex, names)? }),
        "-singleturn" => Ok(Msg::SingleTurn { mon: parse_mon(arg(args, 0)?, names)?, effect: parse_arg_simple(arg(args, 1)?, dex)?.0, kw: parse_kwargs(&args[2..], dex, names)? }),
        "-singlemove" => Ok(Msg::SingleMove { mon: parse_mon(arg(args, 0)?, names)?, effect: parse_arg_simple(arg(args, 1)?, dex)?.0, kw: Kw::NONE }),
        "-mustrecharge" => Ok(Msg::MustRecharge(parse_mon(arg(args, 0)?, names)?)),
        "-prepare" | "-anim" => {
            let mon = parse_mon(arg(args, 0)?, names)?;
            let mv = parse_move_name(arg(args, 1)?, dex)?;
            let target = if let Some(s) = args.get(2).copied() { if !s.is_empty() { Some(parse_mon(s, names)?) } else { None } } else { None };
            if k == "-prepare" { Ok(Msg::Prepare { mon, mv, target }) } else { Ok(Msg::Anim { mon, mv, target }) }
        }
        "-transform" => Ok(Msg::Transform { mon: parse_mon(arg(args, 0)?, names)?, into: parse_mon(arg(args, 1)?, names)?, kw: parse_kwargs(&args[2..], dex, names)? }),
        "-weather" => Ok(Msg::Weather { weather: parse_weather(arg(args, 0)?)?, kw: parse_kwargs(&args[1..], dex, names)? }),
        "-fieldstart" => Ok(Msg::FieldStart { effect: parse_arg_simple(arg(args, 0)?, dex)?.0, kw: parse_kwargs(&args[1..], dex, names)? }),
        "-fieldend" => Ok(Msg::FieldEnd { effect: parse_arg_simple(arg(args, 0)?, dex)?.0, kw: parse_kwargs(&args[1..], dex, names)? }),
        "-fieldactivate" => Ok(Msg::FieldActivate { effect: parse_arg_simple(arg(args, 0)?, dex)?.0, kw: parse_kwargs(&args[1..], dex, names)? }),
        "-sidestart" => {
            let (side, _name) = parse_side_ident(arg(args, 0)?, names)?;
            Ok(Msg::SideStart { side, effect: parse_arg_simple(arg(args, 1)?, dex)?.0, kw: parse_kwargs(&args[2..], dex, names)? })
        }
        "-sideend" => {
            let (side, _name) = parse_side_ident(arg(args, 0)?, names)?;
            Ok(Msg::SideEnd { side, effect: parse_arg_simple(arg(args, 1)?, dex)?.0, kw: parse_kwargs(&args[2..], dex, names)? })
        }
        "-item" => Ok(Msg::Item { mon: parse_mon(arg(args, 0)?, names)?, item: parse_item_name(arg(args, 1)?, dex)?, kw: parse_kwargs(&args[2..], dex, names)? }),
        "-enditem" => Ok(Msg::EndItem { mon: parse_mon(arg(args, 0)?, names)?, item: parse_item_name(arg(args, 1)?, dex)?, kw: parse_kwargs(&args[2..], dex, names)? }),
        "-ability" => {
            let mon = parse_mon(arg(args, 0)?, names)?;
            let ability = parse_ability_name(arg(args, 1)?, dex)?;
            // The third argument, if not a kwarg: `boost` (Intimidate) or the
            // old ability's name (PS `setAbility`).
            let (mut old, mut boost, mut i) = (None, false, 2);
            if let Some(s) = args.get(2).copied().filter(|s| !s.starts_with('[')) {
                if s == Word::Boost.as_str() {
                    boost = true;
                } else {
                    old = Some(parse_ability_name(s, dex)?);
                }
                i = 3;
            }
            Ok(Msg::Ability { mon, ability, old, boost, kw: parse_kwargs(&args[i..], dex, names)? })
        }
        "-endability" => Ok(Msg::EndAbility { mon: parse_mon(arg(args, 0)?, names)?, kw: parse_kwargs(&args[1..], dex, names)? }),
        "-formechange" => {
            let mon = parse_mon(arg(args, 0)?, names)?;
            let species = parse_move_id_internal_species(arg(args, 1)?, dex)?;
            Ok(Msg::FormeChange { mon, species, kw: parse_kwargs(&args[2..], dex, names)? })
        }
        "-message" => Ok(Msg::Message(decode_text(&text_arg(args)?, dex, names)?)),
        // A one-sided hint's `|split|pN` comes on the line before, as its own
        // `Msg::Split`; this line alone is the unsplit form.
        "-hint" => Ok(Msg::Hint { text: decode_text(&text_arg(args)?, dex, names)?, split: None }),
        other => Err(CodecError::UnknownKind(other.to_string())),
    }
}

fn decode_major(kind: &str, args: &[&str], dex: &dyn DexNames, names: &mut Battle) -> Result<Msg, CodecError> {
    match kind {
        "move" => {
            let user = parse_mon(arg(args, 0)?, names)?;
            let mv = parse_move_name(arg(args, 1)?, dex)?;
            let target = parse_opt_mon(arg_opt(args, 2), names);
            let kw = parse_kwargs(&args[3..], dex, names)?;
            Ok(Msg::Move { user, mv, target, kw })
        }
        "switch" | "drag" => {
            let mon = parse_mon(arg(args, 0)?, names)?;
            let details = parse_details(arg(args, 1)?, dex, names)?;
            let health = parse_health(arg(args, 2)?)?;
            // `[from] EFFECT` may follow HP. Anything else is a kwarg.
            let from = if let Some(s) = args.get(3).copied() {
                if let Some(rest) = s.strip_prefix("[from] ") {
                    Some(parse_arg_simple(rest, dex)?.0)
                } else { None }
            } else { None };
            if kind == "switch" {
                Ok(Msg::Switch { mon, details, health, from })
            } else {
                Ok(Msg::Drag { mon, details, health, from })
            }
        }
        "detailschange" => {
            let mon = parse_mon(arg(args, 0)?, names)?;
            let details = parse_details(arg(args, 1)?, dex, names)?;
            Ok(Msg::DetailsChange { mon, details, kw: parse_kwargs(&args[2..], dex, names)? })
        }
        "cant" => {
            let mon = parse_mon(arg(args, 0)?, names)?;
            let reason = parse_arg_simple(arg(args, 1)?, dex)?.0;
            let mv = if let Some(_) = args.get(2) {
                Some(parse_detail_arg(arg(args, 2)?, dex)?.0)
            } else { None };
            Ok(Msg::Cant { mon, reason, mv })
        }
        "faint" => Ok(Msg::Faint(parse_mon(arg(args, 0)?, names)?)),
        other => Err(CodecError::UnknownKind(other.to_string())),
    }
}

// ============================================================================
// argument helpers
// ============================================================================

fn arg<'a>(args: &'a [&'a str], i: usize) -> Result<&'a str, CodecError> {
    args.get(i).copied().ok_or(CodecError::BadTrailingArg(i))
}

fn arg_opt<'a>(args: &'a [&'a str], i: usize) -> Option<&'a str> {
    args.get(i).copied()
}

fn parse_u8(s: &str, ctx: &str) -> Result<u8, CodecError> {
    s.parse().map_err(|_| CodecError::BadArg(format!("{}: {:?}", ctx, s)))
}

fn parse_u16(s: &str, ctx: &str) -> Result<u16, CodecError> {
    s.parse().map_err(|_| CodecError::BadArg(format!("{}: {:?}", ctx, s)))
}

fn parse_u32(s: &str, ctx: &str) -> Result<u32, CodecError> {
    s.parse().map_err(|_| CodecError::BadArg(format!("{}: {:?}", ctx, s)))
}

fn parse_i8(s: &str, ctx: &str) -> Result<i8, CodecError> {
    s.parse().map_err(|_| CodecError::BadArg(format!("{}: {:?}", ctx, s)))
}

fn parse_side(s: &str) -> Result<SideId, CodecError> {
    match s {
        "p1" => Ok(SideId::P1),
        "p2" => Ok(SideId::P2),
        _ => Err(CodecError::BadArg(format!("side {:?}", s))),
    }
}

fn parse_side_ident(s: &str, _names: &mut Battle) -> Result<(SideId, String), CodecError> {
    // `p1: NAME` or `p1a: NAME`.
    let side_str = s.get(..2).ok_or_else(|| CodecError::BadArg(format!("side ident {:?}", s)))?;
    let side = parse_side(side_str)?;
    let colon = s.find(':').ok_or_else(|| CodecError::BadArg(format!("side ident {:?}", s)))?;
    Ok((side, s[colon + 1..].trim_start().to_string()))
}

fn parse_game_type(s: &str) -> Result<GameType, CodecError> {
    match s {
        "singles" => Ok(GameType::Singles),
        _ => Err(CodecError::BadArg(format!("gametype {:?}", s))),
    }
}

fn parse_tier(s: &str) -> Result<FormatId, CodecError> {
    FormatId::ALL.into_iter().find(|f| f.name() == s).ok_or_else(|| CodecError::BadArg(format!("tier {:?}", s)))
}

fn parse_rule(s: &str) -> Result<RuleId, CodecError> {
    RuleId::ALL
        .into_iter()
        .find(|r| r.line() == Some(s))
        .ok_or_else(|| CodecError::BadArg(format!("rule {:?}", s)))
}

fn parse_status(s: &str) -> Result<Status, CodecError> {
    Status::parse(s).ok_or_else(|| CodecError::BadStatus(s.into()))
}

fn parse_stat(s: &str) -> Result<Stat, CodecError> {
    Stat::parse_abbr(s).or_else(|| Stat::parse_name(s))
        .ok_or_else(|| CodecError::BadStat(s.into()))
}

fn parse_weather(s: &str) -> Result<Weather, CodecError> {
    Weather::parse(s).ok_or_else(|| CodecError::BadArg(format!("weather {:?}", s)))
}

fn parse_health(s: &str) -> Result<Health, CodecError> {
    let mut parts = s.splitn(2, ' ');
    let hp_part = parts.next().unwrap_or("");
    let status_part = parts.next();
    let status = match status_part {
        Some(other) => Some(parse_status(other)?),
        None => None,
    };
    if hp_part == "0" {
        return Ok(Health::FAINTED_SPLIT);
    }
    let (cur_s, max_s) = hp_part.split_once('/').ok_or_else(|| CodecError::BadHp(hp_part.into()))?;
    let cur: u16 = cur_s.parse().map_err(|_| CodecError::BadHp(hp_part.into()))?;
    let max: u16 = max_s.parse().map_err(|_| CodecError::BadHp(hp_part.into()))?;
    Ok(Health { secret: Hp::new(cur, max), shared: Hp::new(cur, max), status, split: false })
}

/// Parses `p1a: NAME`. Records the nickname of slot 0 of `side`.
fn parse_mon(s: &str, names: &mut Battle) -> Result<PokemonIdent, CodecError> {
    let side_part = s.get(..2).ok_or_else(|| CodecError::BadArg(format!("ident {:?}", s)))?;
    let side = parse_side(side_part)?;
    let active = s.as_bytes().get(2) == Some(&b'a');
    let colon = s.find(':').ok_or_else(|| CodecError::BadArg(format!("ident {:?}", s)))?;
    let nick = s[colon + 1..].trim_start();
    let slot = find_or_add_slot(side, nick, names);
    Ok(PokemonIdent { side, slot, active })
}

fn find_or_add_slot(side: SideId, nick: &str, names: &mut Battle) -> u8 {
    names.set_nickname(side, nick)
}

fn parse_opt_mon<'a>(m: Option<&'a str>, names: &mut Battle) -> Option<PokemonIdent> {
    let s = m?;
    if s.is_empty() {
        return None;
    }
    match parse_mon(s, names) {
        Ok(m) => Some(m),
        Err(_) => None,
    }
}

/// Parses a details string (PS `getUpdatedDetails`). The species must be a
/// dex species or a cosmetic forme, by exact name.
pub(crate) fn parse_details(s: &str, dex: &dyn DexNames, names: &dyn BattleNames) -> Result<Details, CodecError> {
    let mut parts = s.split(", ");
    let species_name = parts.next().unwrap_or("");
    let id = id_str(species_name);
    let (species, forme) = if let Some(n) = dex.species_from_id(&id).filter(|&n| dex.species_name(n) == species_name) {
        (n, 0)
    } else if let Some((n, f)) = dex.cosmetic_forme_from_id(&id).filter(|&(n, f)| dex.cosmetic_forme_name(n, f) == species_name) {
        (n, f)
    } else {
        return Err(CodecError::BadArg(format!("species {:?}", species_name)));
    };
    let mut level = 100u16;
    let mut gender = Gender::N;
    let mut shiny = false;
    for p in parts {
        if let Some(rest) = p.strip_prefix('L') {
            level = rest.parse().map_err(|_| CodecError::BadArg(format!("level {:?}", p)))?;
        } else if p == "M" {
            gender = Gender::M;
        } else if p == "F" {
            gender = Gender::F;
        } else if p == "shiny" {
            shiny = true;
        } else {
            return Err(CodecError::BadArg(format!("details field {:?}", p)));
        }
    }
    let _ = names;
    Ok(Details { species, level, gender, shiny, forme })
}

fn parse_move_name(s: &str, dex: &dyn DexNames) -> Result<crate::ident::MoveNum, CodecError> {
    let s = normalise_move_name(s);
    if let Some(m) = dex.move_from_id(&id_str(&s)) {
        return Ok(m);
    }
    if let Some(rest) = s.strip_prefix("Hidden Power ") {
        let type_name_end = rest.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(rest.len());
        let _type_name = &rest[..type_name_end];
        // The id is `hiddenpower` (or `hiddenpowerfire` for gen 3 onward, but
        // gens 1 and 3 emit the bare id).
        return dex.move_from_id("hiddenpower")
            .ok_or_else(|| CodecError::BadArg(format!("hiddenpower {:?}", s)));
    }
    Err(CodecError::BadArg(format!("move {:?}", s)))
}

fn normalise_move_name(s: &str) -> String {
    // PS uses `Hidden Power Fire 70` (gen <6); gens 1 and 3 do this. We don't
    // need the type, just a returnable id.
    s.to_string()
}

fn parse_move_id_internal_species(s: &str, dex: &dyn DexNames) -> Result<SpeciesNum, CodecError> {
    dex.species_from_id(&id_str(s))
        .filter(|&n| dex.species_name(n) == s)
        .ok_or_else(|| CodecError::BadArg(format!("species {:?}", s)))
}

fn parse_item_name(s: &str, dex: &dyn DexNames) -> Result<crate::ident::ItemNum, CodecError> {
    if let Some(i) = dex.item_from_id(&id_str(s)) {
        return Ok(i);
    }
    (0..1000).find_map(|n| {
        let i = crate::ident::ItemNum(n);
        if dex.item_name(i) == s { Some(i) } else { None }
    }).ok_or_else(|| CodecError::BadArg(format!("item {:?}", s)))
}

fn parse_ability_name(s: &str, dex: &dyn DexNames) -> Result<AbilityNum, CodecError> {
    if let Some(a) = dex.ability_from_id(&id_str(s)) {
        return Ok(a);
    }
    (0..1000).find_map(|n| {
        let a = AbilityNum(n);
        if dex.ability_name(a) == s { Some(a) } else { None }
    }).ok_or_else(|| CodecError::BadArg(format!("ability {:?}", s)))
}

/// PS `toID`: lowercase ASCII letters and digits only.
fn id_str(s: &str) -> String {
    let id = crate::to_id::<32>(s);
    id.map(|b| b.as_str().to_string()).unwrap_or_else(|_| {
        s.to_ascii_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect()
    })
}

/// The text of a text line: everything after the kind (a text may hold a `|`).
fn text_arg(args: &[&str]) -> Result<String, CodecError> {
    if args.is_empty() {
        return Err(CodecError::BadTrailingArg(0));
    }
    Ok(args.join("|"))
}

/// Finds the [`TextId`] template `s` was rendered from and reads its
/// arguments back by their [`TextArgKind`]. Unknown text is an error.
fn decode_text(s: &str, dex: &dyn DexNames, names: &mut Battle) -> Result<Text, CodecError> {
    for &id in TextId::ALL {
        if let Some(text) = match_template(id, s, dex, names) {
            return Ok(text);
        }
    }
    Err(CodecError::UnsupportedLine(format!("no text template for {:?}", s)))
}

fn match_template(id: TextId, s: &str, dex: &dyn DexNames, names: &mut Battle) -> Option<Text> {
    let tmpl = id.template();
    let kinds = id.arg_kinds();
    let mut pieces = tmpl.split("{}");
    let mut rest = s.strip_prefix(pieces.next().unwrap_or(""))?;
    let mut args = [TextArg::NONE; 3];
    for (i, &kind) in kinds.iter().enumerate() {
        let lit = pieces.next().unwrap_or("");
        // An argument ends at the next literal piece (the end of the text
        // for a trailing argument).
        let end = if lit.is_empty() { rest.len() } else { rest.find(lit)? };
        args[i] = parse_text_arg(kind, &rest[..end], dex, names)?;
        rest = &rest[end + lit.len()..];
    }
    if !rest.is_empty() {
        return None;
    }
    Some(Text { id, args })
}

fn parse_text_arg(kind: TextArgKind, s: &str, dex: &dyn DexNames, names: &mut Battle) -> Option<TextArg> {
    Some(match kind {
        TextArgKind::Int => {
            let n: i32 = s.parse().ok()?;
            (n.to_string() == s).then_some(TextArg::int(n))?
        }
        TextArgKind::MoveId => TextArg::move_id(dex.move_from_id(s).filter(|&m| dex.move_id(m) == s)?),
        TextArgKind::Move => TextArg::mv(dex.move_from_id(&id_str(s)).filter(|&m| dex.move_name(m) == s)?),
        TextArgKind::Mon => TextArg::mon(parse_mon(s, names).ok()?),
        TextArgKind::Nickname => {
            // A bare nickname names no side; take the first side that has it.
            let side = [SideId::P1, SideId::P2]
                .into_iter()
                .find(|&side| names.find_nickname_side(side, s).is_some())
                .unwrap_or(SideId::P1);
            TextArg::nickname(PokemonIdent::inactive(side, names.set_nickname(side, s)))
        }
        TextArgKind::Status => TextArg::status(Status::parse(s)?),
        TextArgKind::Word => TextArg::word(Word::parse(s)?),
        TextArgKind::Rule => TextArg::custom_rule(crate::command::CustomRule::parse(s)?),
    })
}

fn parse_win_side(s: &str, names: &Battle) -> SideId {
    if names.player_name(SideId::P1) == s { SideId::P1 } else { SideId::P2 }
}

/// Parses `[from] EFFECT`-style kwargs into a [`Kw`].
///
/// In tolerance mode, lines that don't match a flag or `[from]` / `[of]` /
/// `[wisher]` are silently dropped (they may be positional trailing fields
/// that the typed encoder has already consumed).
fn parse_kwargs(args: &[&str], dex: &dyn DexNames, names: &mut Battle) -> Result<Kw, CodecError> {
    let mut kw = Kw::NONE;
    let mut wisher: Option<PokemonIdent> = None;
    for s in args {
        let mut matched = false;
        for (flag, text) in KwFlags::RENDERED.iter().copied() {
            if *s == text {
                kw.flags.insert(flag);
                matched = true;
                break;
            }
        }
        if matched { continue; }
        if let Some(rest) = s.strip_prefix("[from] ") {
            if let Ok((a, _)) = parse_arg_simple(rest, dex) {
                kw.from = Some(a);
            }
            continue;
        }
        if let Some(rest) = s.strip_prefix("[of] ") {
            if let Ok(m) = parse_mon(rest, names) {
                kw.of = Some(m);
            }
            continue;
        }
        if let Some(rest) = s.strip_prefix("[wisher] ") {
            // `[wisher] NICKNAME` is just the nickname (no `p1a:` side prefix).
            // Try both sides; PS looks it up on the acting side first.
            for s_idx in 0..2 {
                let side = crate::ident::SideId::from_index(s_idx).unwrap();
                if let Some(slot) = names.find_nickname_side(side, rest) {
                    wisher = Some(crate::ident::PokemonIdent {
                        side, slot, active: false,
                    });
                    break;
                }
            }
            // falling through to `kw.of = Some(some_default)` is fine if no
            // match: the encoder will fall back to slot 0 (likely wrong, but
            // better than dropping the flag).
            continue;
        }
        // Otherwise drop silently.
    }
    if let Some(m) = wisher {
        kw.of = Some(m);
        kw.flags.insert(KwFlags::WISHER);
    }
    Ok(kw)
}

// ps_protocol's `Msg` doesn't expose `kw_or_empty`; need a helper extension.
trait MsgKw {
    fn kw_or_empty(&self) -> Option<&Kw>;
}
impl MsgKw for Msg {
    fn kw_or_empty(&self) -> Option<&Kw> {
        self.kw()
    }
}

/// Parses an arg in / positional-or-/ form.
/// Returns (Arg, used) where `used = true` means at least one of our typed
/// Arg variants accepted it (so `-fail`'s positional slot should consume it).
fn parse_arg_simple(s: &str, dex: &dyn DexNames) -> Result<(Arg, bool), CodecError> {
    // Bracketed tokens are kwarg markers; treat as unrecognised so the
    // positional readers (e.g. `-activate|MON|EFFECT|A`) leave them for the
    // kwarg pass.
    if s.starts_with('[') {
        return Ok((Arg::new(ArgValue::Word(Word::Move)), false));
    }
    if let Some(rest) = s.strip_prefix("move: ") {
        if let Some(m) = dex.move_from_id(&id_str(rest)) {
            return Ok((Arg::new(ArgValue::MoveEffect(m)), true));
        }
        return Err(CodecError::BadArg(format!("move: {:?}", rest)));
    }
    if let Some(rest) = s.strip_prefix("ability: ") {
        if let Some(a) = dex.ability_from_id(&id_str(rest)) {
            return Ok((Arg::new(ArgValue::AbilityEffect(a)), true));
        }
        return Err(CodecError::BadArg(format!("ability: {:?}", rest)));
    }
    if let Some(rest) = s.strip_prefix("item: ") {
        if let Some(i) = dex.item_from_id(&id_str(rest)) {
            return Ok((Arg::new(ArgValue::ItemEffect(i)), true));
        }
        return Err(CodecError::BadArg(format!("item: {:?}", rest)));
    }
    // Two types joined with a slash (`Water/Flying`): PS's
    // `source.types.join('/')`. Anything else with a slash falls through
    // to the parsers below.
    if let Some((first, second)) = s.split_once('/') {
        if let (Some(a), Some(b)) = (dex.type_from_id(&id_str(first)), dex.type_from_id(&id_str(second))) {
            return Ok((Arg::new(ArgValue::TypePair(a, b)), true));
        }
    }
    if let Some(s) = Status::parse(s) { return Ok((Arg::new(ArgValue::Status(s)), true)); }
    if let Some(st) = Stat::parse_name(s) { return Ok((Arg::new(ArgValue::StatName(st)), true)); }
    if let Some(st) = Stat::parse_abbr(s) { return Ok((Arg::new(ArgValue::Stat(st)), true)); }
    if let Some(w) = Word::parse(s) { return Ok((Arg::new(ArgValue::Word(w)), true)); }
    if let Some(t) = dex.type_from_id(&id_str(s)) { return Ok((Arg::new(ArgValue::Type(t)), true)); }
    if let Some(m) = dex.move_from_id(&id_str(s)) { return Ok((Arg::new(ArgValue::Move(m)), true)); }
    // Unrecognised argument; return a typed placeholder.
    Ok((Arg::new(ArgValue::Word(Word::Move)), false))
}

/// [`parse_arg_simple`] for a detail that PS writes as a move's name when it
/// is a name at all (`-start|MON|Mimic|MOVE`, `-activate|MON|move: Spite|MOVE|N`,
/// `cant|MON|Disable|MOVE`). `Psychic` names both a type and a move, and
/// `parse_arg_simple` reads it as the type; here it is the move.
fn parse_detail_arg(s: &str, dex: &dyn DexNames) -> Result<(Arg, bool), CodecError> {
    let (a, used) = parse_arg_simple(s, dex)?;
    if let ArgValue::Type(_) = a.value() {
        if let Some(m) = dex.move_from_id(&id_str(s)) {
            return Ok((Arg::new(ArgValue::Move(m)), true));
        }
    }
    Ok((a, used))
}

// ============================================================================
// In-memory BattleNames implementation
// ============================================================================

/// A per-side in-memory name table. Tracks players (`>player`) and per-slot
/// nicknames (`-switch|-drag`).
#[derive(Clone, Debug, Default)]
pub struct Battle {
    players: [PlayerInfo; 2],
    nicks: [Vec<Name>; 2],
}

impl Battle {
    pub fn new() -> Self {
        Self {
            players: [PlayerInfo::default(), PlayerInfo::default()],
            nicks: [Vec::new(), Vec::new()],
        }
    }
    fn player(&self, s: SideId) -> &PlayerInfo {
        &self.players[s.index()]
    }
    fn player_mut(&mut self, s: SideId) -> &mut PlayerInfo {
        &mut self.players[s.index()]
    }
    #[allow(dead_code)]
    fn nick(&mut self, s: SideId, slot: u8) -> &str {
        if let Some(n) = self.nicks[s.index()].get(slot as usize) {
            n.as_str()
        } else {
            ""
        }
    }
    pub fn set_player(&mut self, side: SideId, name: &str, avatar: &str, rating: &str) {
        let p = self.player_mut(side);
        p.name.clear();
        p.avatar.clear();
        p.rating.clear();
        let _ = p.name.push_str(name);
        let _ = p.avatar.push_str(avatar);
        let _ = p.rating.push_str(rating);
    }
    pub fn set_nickname(&mut self, side: SideId, nick: &str) -> u8 {
        // If a slot already has this nickname, reuse it; else add. PS allows
        // duplicate nicknames, so the first match wins (matches PS's
        // behaviour when `>player` doesn't repeat them).
        let names = &mut self.nicks[side.index()];
        let slot = names.iter().position(|n| n.as_str() == nick);
        if let Some(slot) = slot {
            slot as u8
        } else {
            let mut n = Name::new();
            let _ = n.push_str(nick);
            names.push(n);
            (names.len() - 1) as u8
        }
    }
    pub fn find_nickname_side(&self, side: SideId, nick: &str) -> Option<u8> {
        self.nicks[side.index()]
            .iter()
            .position(|n| n.as_str() == nick)
            .map(|i| i as u8)
    }
}

#[derive(Clone, Debug, Default)]
struct PlayerInfo {
    name: Name,
    avatar: Name,
    rating: Name,
}

impl BattleNames for Battle {
    fn player_name(&self, side: SideId) -> &str { self.player(side).name.as_str() }
    fn player_avatar(&self, side: SideId) -> &str { self.player(side).avatar.as_str() }
    fn player_rating(&self, side: SideId) -> &str { self.player(side).rating.as_str() }
    fn nickname(&self, side: SideId, slot: u8) -> &str {
        self.nicks[side.index()].get(slot as usize).map(|n| n.as_str()).unwrap_or("")
    }
}

// Adapter for `CodecState`: `battle_mut()` requires mutable; `parse_kwargs`
// doesn't need it, but the `decode_*` paths do. We pass the state by value to
// the smaller helpers and clone where appropriate.
