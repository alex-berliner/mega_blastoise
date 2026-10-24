//! Hand-rolled JSON encoder and decoder for PS's `|request|` lines.
//!
//! PS writes a request with `JSON.stringify`, so its bytes are the keys in
//! insertion order: `sim/battle.ts` `getRequests` (the top level and
//! `noCancel`), `sim/side.ts` `getRequestData` (`side`), `sim/pokemon.ts`
//! `getMoveRequestData`, `getMoves` and `getSwitchRequestData` (the entries).
//! The key order of every object is documented on its type in
//! [`crate::request`]; [`encode_request`] writes exactly that, and
//! [`decode_request`] accepts only the keys the types model, so a request
//! that decodes re-encodes byte for byte or the round-trip test says why.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::ident::{Hp, HpStatus, PokemonIdent, SideId, Status, TypeNum};
use crate::request::{
    ActiveRequest, MoveRequest, MoveTarget, Pokeball, Request, RequestMove, RequestMoveName,
    SideMoveId, SidePokemon, SideRequest, SwitchRequest, WaitRequest,
};
use crate::{to_id, BattleNames, BoundedVec, DexNames, Name};

use super::line::{details_str, parse_details, Battle, CodecError};

// ============================================================================
// Encoder
// ============================================================================

/// Encodes a [`Request`] as PS's JSON, byte for byte.
pub fn encode_request(req: &Request, dex: &dyn DexNames, names: &dyn BattleNames) -> String {
    let mut out = String::with_capacity(2048);
    out.push('{');
    match req {
        Request::Move(r) => {
            out.push_str("\"active\":[");
            for (i, a) in r.active.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                push_active(&mut out, a, dex);
            }
            out.push_str("],\"side\":");
            push_side(&mut out, &r.side, dex, names);
            push_tail(&mut out, r.no_cancel, r.update, r.rqid);
        }
        Request::Switch(r) => {
            out.push_str("\"forceSwitch\":[");
            for (i, f) in r.force_switch.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(if *f { "true" } else { "false" });
            }
            out.push_str("],\"side\":");
            push_side(&mut out, &r.side, dex, names);
            push_tail(&mut out, r.no_cancel, r.update, r.rqid);
        }
        Request::Wait(r) => {
            out.push_str("\"wait\":true,\"side\":");
            push_side(&mut out, &r.side, dex, names);
            push_tail(&mut out, false, false, r.rqid);
        }
    }
    out.push('}');
    out
}

fn push_tail(out: &mut String, no_cancel: bool, update: bool, rqid: Option<u32>) {
    if no_cancel {
        out.push_str(",\"noCancel\":true");
    }
    if update {
        out.push_str(",\"update\":true");
    }
    if let Some(n) = rqid {
        out.push_str(",\"rqid\":");
        out.push_str(&n.to_string());
    }
}

/// PS `getMoveRequestData`.
fn push_active(out: &mut String, a: &ActiveRequest, dex: &dyn DexNames) {
    out.push_str("{\"moves\":[");
    for (i, m) in a.moves.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_request_move(out, m, dex);
    }
    out.push(']');
    if a.maybe_disabled {
        out.push_str(",\"maybeDisabled\":true");
    }
    if a.maybe_locked {
        out.push_str(",\"maybeLocked\":true");
    }
    if a.trapped {
        out.push_str(",\"trapped\":true");
    }
    if a.maybe_trapped {
        out.push_str(",\"maybeTrapped\":true");
    }
    out.push('}');
}

/// One entry of PS `getMoves`.
fn push_request_move(out: &mut String, m: &RequestMove, dex: &dyn DexNames) {
    let (name, id) = request_move_name(m.name, dex);
    out.push_str("{\"move\":");
    push_json_str(out, &name);
    out.push_str(",\"id\":");
    push_json_str(out, &id);
    if let Some(pp) = m.pp {
        out.push_str(",\"pp\":");
        out.push_str(&pp.to_string());
    }
    if let Some(maxpp) = m.maxpp {
        out.push_str(",\"maxpp\":");
        out.push_str(&maxpp.to_string());
    }
    if let Some(t) = m.target {
        out.push_str(",\"target\":");
        push_json_str(out, t.as_str());
    }
    if let Some(d) = m.disabled {
        out.push_str(",\"disabled\":");
        out.push_str(if d { "true" } else { "false" });
    }
    out.push('}');
}

/// The `move` and `id` of a request move (PS `getMoves`: Hidden Power's type
/// and power, Return's and Frustration's power, gen 1's `Fight`).
fn request_move_name(n: RequestMoveName, dex: &dyn DexNames) -> (String, String) {
    match n {
        RequestMoveName::Move(m) => (dex.move_name(m).to_string(), dex.move_id(m).to_string()),
        RequestMoveName::HiddenPower { ty, power } => {
            (format!("Hidden Power {} {}", dex.type_name(ty), power), "hiddenpower".to_string())
        }
        RequestMoveName::WithPower { mv, power } => {
            (format!("{} {}", dex.move_name(mv), power), dex.move_id(mv).to_string())
        }
        RequestMoveName::Recharge => ("Recharge".to_string(), "recharge".to_string()),
        RequestMoveName::Fight => ("Fight".to_string(), "fight".to_string()),
    }
}

/// PS `getRequestData`.
fn push_side(out: &mut String, side: &SideRequest, dex: &dyn DexNames, names: &dyn BattleNames) {
    out.push_str("{\"name\":");
    push_json_str(out, &side.name);
    out.push_str(",\"id\":");
    push_json_str(out, side.id.as_str());
    out.push_str(",\"pokemon\":[");
    for (i, p) in side.pokemon.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_side_pokemon(out, p, dex, names);
    }
    out.push_str("]}");
}

/// PS `getSwitchRequestData`.
fn push_side_pokemon(out: &mut String, p: &SidePokemon, dex: &dyn DexNames, names: &dyn BattleNames) {
    out.push_str("{\"ident\":");
    let ident = format!("{}: {}", p.ident.side.as_str(), names.nickname(p.ident.side, p.ident.slot));
    push_json_str(out, &ident);
    out.push_str(",\"details\":");
    push_json_str(out, &details_str(p.details, dex, names));
    out.push_str(",\"condition\":");
    push_json_str(out, &hp_status_str(p.condition));
    out.push_str(",\"active\":");
    out.push_str(if p.active { "true" } else { "false" });
    out.push_str(",\"stats\":{");
    for (i, (key, v)) in STAT_KEYS.iter().zip(p.stats).enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_json_str(out, key);
        out.push(':');
        out.push_str(&v.to_string());
    }
    out.push_str("},\"moves\":[");
    for (i, m) in p.moves.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_json_str(out, &side_move_id(*m, dex));
    }
    out.push_str("],\"baseAbility\":");
    push_json_str(out, p.base_ability.map(|a| dex.ability_id(a)).unwrap_or(""));
    out.push_str(",\"item\":");
    push_json_str(out, p.item.map(|i| dex.item_id(i)).unwrap_or(""));
    out.push_str(",\"pokeball\":");
    push_json_str(
        out,
        match p.pokeball {
            Pokeball::Default => "pokeball",
            Pokeball::Item(i) => dex.item_id(i),
        },
    );
    out.push('}');
}

const STAT_KEYS: [&str; 5] = ["atk", "def", "spa", "spd", "spe"];

/// One id of `side.pokemon[].moves` (PS `getSwitchRequestData`: Hidden Power
/// gets its type's id, Return and Frustration their power).
fn side_move_id(m: SideMoveId, dex: &dyn DexNames) -> String {
    match m {
        SideMoveId::Move(m) => dex.move_id(m).to_string(),
        SideMoveId::HiddenPower(t) => format!("hiddenpower{}", type_id(t, dex)),
        SideMoveId::WithPower { mv, power } => format!("{}{}", dex.move_id(mv), power),
    }
}

fn type_id(t: TypeNum, dex: &dyn DexNames) -> String {
    to_id::<16>(dex.type_name(t)).map(|s| s.as_str().to_string()).unwrap_or_default()
}

/// `HP STATUS` as PS `getHealth().secret` writes it: `0 fnt`, `46/294`,
/// `174/258 par`.
fn hp_status_str(hs: HpStatus) -> String {
    let mut s = hs.hp.cur.to_string();
    if hs.hp.max != 0 {
        s.push('/');
        s.push_str(&hs.hp.max.to_string());
    }
    if let Some(st) = hs.status {
        s.push(' ');
        s.push_str(st.as_str());
    }
    s
}

/// A JSON string as `JSON.stringify` writes it: `"` and `\` escaped, the
/// short escapes for backspace, form feed, newline, carriage return and tab,
/// `\u00XX` for the other control characters, everything else as is.
pub(crate) fn push_json_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

// ============================================================================
// Decoder
// ============================================================================

/// Parses one JSON document with PS's reader.
///
/// The `>start` and `>player` payloads are JSON too, so they read with
/// the same reader as the request JSON (the command codec calls this).
pub(crate) fn parse_value(s: &str) -> Result<Value, CodecError> {
    parse_value_as(s, "json")
}

/// Parses one JSON document, with the reader's errors prefixed by `what`.
fn parse_value_as(s: &str, what: &'static str) -> Result<Value, CodecError> {
    let mut p = Parser { s: s.as_bytes(), pos: 0, what };
    let v = p.value()?;
    p.ws();
    if p.pos != p.s.len() {
        return Err(p.bad("trailing bytes after the value"));
    }
    Ok(v)
}

/// Decodes a request JSON (the text after `|request|`). Nicknames in
/// `ident` resolve to team slots through `names`, as the line decoder's do.
pub fn decode_request(s: &str, dex: &dyn DexNames, names: &mut Battle) -> Result<Request, CodecError> {
    let v = parse_value_as(s, "request")?;
    let top = v.obj("request")?;
    let first = top.first().map(|(k, _)| k.as_str()).unwrap_or("");
    let side_v = field(top, "side")?;
    let side = decode_side(side_v, dex, names)?;
    let flag = |k: &str| -> Result<bool, CodecError> {
        match top.iter().find(|(key, _)| key == k) {
            Some((_, v)) => v.boolean(k),
            None => Ok(false),
        }
    };
    let no_cancel = flag("noCancel")?;
    let update = flag("update")?;
    let rqid = match top.iter().find(|(k, _)| k == "rqid") {
        Some((_, v)) => Some(v.uint("rqid")? as u32),
        None => None,
    };
    let req = match first {
        "active" => {
            only_keys(top, &["active", "side", "noCancel", "update", "rqid"])?;
            let mut active = BoundedVec::new();
            for a in field(top, "active")?.arr("active")? {
                active.push(decode_active(a, dex)?).map_err(|_| bad("too many active"))?;
            }
            Request::Move(MoveRequest { active, side, no_cancel, update, rqid })
        }
        "forceSwitch" => {
            only_keys(top, &["forceSwitch", "side", "noCancel", "update", "rqid"])?;
            let mut force_switch = BoundedVec::new();
            for f in field(top, "forceSwitch")?.arr("forceSwitch")? {
                force_switch.push(f.boolean("forceSwitch")?).map_err(|_| bad("too many forceSwitch"))?;
            }
            Request::Switch(SwitchRequest { force_switch, side, no_cancel, update, rqid })
        }
        "wait" => {
            only_keys(top, &["wait", "side", "rqid"])?;
            if !field(top, "wait")?.boolean("wait")? {
                return Err(bad("wait is not true"));
            }
            Request::Wait(WaitRequest { side, rqid })
        }
        other => return Err(bad(&format!("request starts with {:?}", other))),
    };
    Ok(req)
}

fn decode_active(v: &Value, dex: &dyn DexNames) -> Result<ActiveRequest, CodecError> {
    let o = v.obj("active[]")?;
    only_keys(o, &["moves", "maybeDisabled", "maybeLocked", "trapped", "maybeTrapped"])?;
    let mut a = ActiveRequest::default();
    for m in field(o, "moves")?.arr("moves")? {
        a.moves.push(decode_request_move(m, dex)?).map_err(|_| bad("too many moves"))?;
    }
    for (k, v) in o {
        let flag = match k.as_str() {
            "maybeDisabled" => &mut a.maybe_disabled,
            "maybeLocked" => &mut a.maybe_locked,
            "trapped" => &mut a.trapped,
            "maybeTrapped" => &mut a.maybe_trapped,
            _ => continue,
        };
        // PS only writes these flags when true.
        if !v.boolean(k)? {
            return Err(bad(&format!("{} is false", k)));
        }
        *flag = true;
    }
    Ok(a)
}

fn decode_request_move(v: &Value, dex: &dyn DexNames) -> Result<RequestMove, CodecError> {
    let o = v.obj("move")?;
    only_keys(o, &["move", "id", "pp", "maxpp", "target", "disabled"])?;
    let name = field(o, "move")?.string("move")?;
    let id = field(o, "id")?.string("id")?;
    let mut m = RequestMove::locked(decode_request_move_name(name, id, dex)?);
    for (k, v) in o {
        match k.as_str() {
            "pp" => m.pp = Some(small(v.uint("pp")?, "pp")?),
            "maxpp" => m.maxpp = Some(small(v.uint("maxpp")?, "maxpp")?),
            "target" => {
                let t = v.string("target")?;
                m.target = Some(MoveTarget::parse(t).ok_or_else(|| bad(&format!("target {:?}", t)))?);
            }
            "disabled" => m.disabled = Some(v.boolean("disabled")?),
            _ => {}
        }
    }
    Ok(m)
}

fn decode_request_move_name(name: &str, id: &str, dex: &dyn DexNames) -> Result<RequestMoveName, CodecError> {
    match (name, id) {
        ("Recharge", "recharge") => return Ok(RequestMoveName::Recharge),
        ("Fight", "fight") => return Ok(RequestMoveName::Fight),
        _ => {}
    }
    let mv = dex.move_from_id(id).ok_or_else(|| bad(&format!("move id {:?}", id)))?;
    if dex.move_name(mv) == name {
        return Ok(RequestMoveName::Move(mv));
    }
    // `NAME POWER`: Hidden Power's `Hidden Power TYPE POWER`, or Return's
    // and Frustration's `NAME POWER`.
    let (head, power) = name.rsplit_once(' ').ok_or_else(|| bad(&format!("move {:?}", name)))?;
    let power: u8 = power.parse().map_err(|_| bad(&format!("move power {:?}", name)))?;
    if id == "hiddenpower" {
        if let Some(ty) = head.strip_prefix("Hidden Power ") {
            let t = dex.type_from_id(&id_of(ty)).filter(|&t| dex.type_name(t) == ty);
            if let Some(ty) = t {
                return Ok(RequestMoveName::HiddenPower { ty, power });
            }
        }
    } else if dex.move_name(mv) == head {
        return Ok(RequestMoveName::WithPower { mv, power });
    }
    Err(bad(&format!("move {:?} / {:?}", name, id)))
}

fn decode_side(v: &Value, dex: &dyn DexNames, names: &mut Battle) -> Result<SideRequest, CodecError> {
    let o = v.obj("side")?;
    only_keys(o, &["name", "id", "pokemon"])?;
    let name = field(o, "name")?.string("name")?;
    let id = field(o, "id")?.string("id")?;
    let side = SideId::parse(id).ok_or_else(|| bad(&format!("side id {:?}", id)))?;
    let mut out = SideRequest {
        name: Name::from_str(name).map_err(|_| bad("side name too long"))?,
        id: side,
        pokemon: BoundedVec::new(),
    };
    for p in field(o, "pokemon")?.arr("pokemon")? {
        out.pokemon.push(decode_side_pokemon(p, dex, names)?).map_err(|_| bad("too many pokemon"))?;
    }
    Ok(out)
}

fn decode_side_pokemon(v: &Value, dex: &dyn DexNames, names: &mut Battle) -> Result<SidePokemon, CodecError> {
    let o = v.obj("pokemon[]")?;
    only_keys(
        o,
        &["ident", "details", "condition", "active", "stats", "moves", "baseAbility", "item", "pokeball"],
    )?;
    let ident = field(o, "ident")?.string("ident")?;
    let (side, nick) = ident.split_once(": ").ok_or_else(|| bad(&format!("ident {:?}", ident)))?;
    let side = SideId::parse(side).ok_or_else(|| bad(&format!("ident {:?}", ident)))?;
    let slot = names.set_nickname(side, nick);
    let details = parse_details(field(o, "details")?.string("details")?, dex, names)?;
    let condition = parse_hp_status(field(o, "condition")?.string("condition")?)?;
    let active = field(o, "active")?.boolean("active")?;
    let stats_o = field(o, "stats")?.obj("stats")?;
    if stats_o.len() != 5 {
        return Err(bad("stats"));
    }
    let mut stats = [0u16; 5];
    for (i, key) in STAT_KEYS.iter().enumerate() {
        let n = field(stats_o, key)?.uint(key)?;
        stats[i] = u16::try_from(n).map_err(|_| bad("stat"))?;
    }
    let mut moves = BoundedVec::new();
    for m in field(o, "moves")?.arr("moves")? {
        moves.push(decode_side_move(m.string("moves[]")?, dex)?).map_err(|_| bad("too many moves"))?;
    }
    let ability = field(o, "baseAbility")?.string("baseAbility")?;
    let base_ability = if ability.is_empty() {
        None
    } else {
        Some(dex.ability_from_id(ability).ok_or_else(|| bad(&format!("ability {:?}", ability)))?)
    };
    let item = field(o, "item")?.string("item")?;
    let item = if item.is_empty() {
        None
    } else {
        Some(dex.item_from_id(item).ok_or_else(|| bad(&format!("item {:?}", item)))?)
    };
    let ball = field(o, "pokeball")?.string("pokeball")?;
    let pokeball = if ball == "pokeball" {
        Pokeball::Default
    } else {
        Pokeball::Item(dex.item_from_id(ball).ok_or_else(|| bad(&format!("pokeball {:?}", ball)))?)
    };
    Ok(SidePokemon {
        ident: PokemonIdent::inactive(side, slot),
        details,
        condition,
        active,
        stats,
        moves,
        base_ability,
        item,
        pokeball,
    })
}

fn decode_side_move(id: &str, dex: &dyn DexNames) -> Result<SideMoveId, CodecError> {
    if let Some(m) = dex.move_from_id(id) {
        return Ok(SideMoveId::Move(m));
    }
    if let Some(t) = id.strip_prefix("hiddenpower").and_then(|ty| dex.type_from_id(ty)) {
        return Ok(SideMoveId::HiddenPower(t));
    }
    let digits = id.len() - id.bytes().rev().take_while(u8::is_ascii_digit).count();
    let (head, power) = id.split_at(digits);
    if let (Some(mv), Ok(n)) = (dex.move_from_id(head), power.parse::<u8>()) {
        if n.to_string() == power {
            return Ok(SideMoveId::WithPower { mv, power: n });
        }
    }
    Err(bad(&format!("side move {:?}", id)))
}

fn parse_hp_status(s: &str) -> Result<HpStatus, CodecError> {
    let (hp, status) = match s.split_once(' ') {
        Some((hp, st)) => (hp, Some(Status::parse(st).ok_or_else(|| CodecError::BadStatus(st.into()))?)),
        None => (s, None),
    };
    let num = |x: &str| x.parse::<u16>().map_err(|_| CodecError::BadHp(s.into()));
    let hp = match hp.split_once('/') {
        Some((cur, max)) => Hp::new(num(cur)?, num(max)?),
        None => Hp::new(num(hp)?, 0),
    };
    Ok(HpStatus { hp, status })
}

fn id_of(s: &str) -> String {
    to_id::<32>(s).map(|b| b.as_str().to_string()).unwrap_or_default()
}

fn small(n: u64, what: &str) -> Result<u8, CodecError> {
    u8::try_from(n).map_err(|_| bad(&format!("{} {}", what, n)))
}

fn bad(s: &str) -> CodecError {
    CodecError::BadArg(format!("request: {}", s))
}

fn field<'a>(o: &'a [(String, Value)], key: &str) -> Result<&'a Value, CodecError> {
    o.iter().find(|(k, _)| k == key).map(|(_, v)| v).ok_or_else(|| bad(&format!("missing {:?}", key)))
}

fn only_keys(o: &[(String, Value)], allowed: &[&str]) -> Result<(), CodecError> {
    for (k, _) in o {
        if !allowed.contains(&k.as_str()) {
            return Err(bad(&format!("unmodelled key {:?}", k)));
        }
    }
    Ok(())
}

// ---- a small JSON reader ---------------------------------------------------

/// A parsed JSON value. Objects keep their keys in order; numbers are the
/// non-negative integers a request holds.
#[derive(Debug)]
pub(crate) enum Value {
    Null,
    Bool(bool),
    Num(u64),
    Str(String),
    Arr(Vec<Value>),
    Obj(Vec<(String, Value)>),
}

impl Value {
    /// The value of `key` in an object, `None` when the object has no such
    /// key (or when the value is not an object at all).
    pub(crate) fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(o) => o.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub(crate) fn obj(&self, what: &str) -> Result<&[(String, Value)], CodecError> {
        match self {
            Value::Obj(o) => Ok(o),
            _ => Err(bad(&format!("{} is not an object", what))),
        }
    }
    pub(crate) fn arr(&self, what: &str) -> Result<&[Value], CodecError> {
        match self {
            Value::Arr(a) => Ok(a),
            _ => Err(bad(&format!("{} is not an array", what))),
        }
    }
    pub(crate) fn string(&self, what: &str) -> Result<&str, CodecError> {
        match self {
            Value::Str(s) => Ok(s),
            _ => Err(bad(&format!("{} is not a string", what))),
        }
    }
    fn boolean(&self, what: &str) -> Result<bool, CodecError> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(bad(&format!("{} is not a bool", what))),
        }
    }
    pub(crate) fn uint(&self, what: &str) -> Result<u64, CodecError> {
        match self {
            Value::Num(n) => Ok(*n),
            Value::Null => Err(bad(&format!("{} is null", what))),
            _ => Err(bad(&format!("{} is not a number", what))),
        }
    }
}

pub(crate) struct Parser<'a> {
    s: &'a [u8],
    pos: usize,
    /// What the JSON is: `request` for a `|request|` payload, `json` for
    /// a command's payload. It prefixes the reader's errors.
    what: &'static str,
}

impl Parser<'_> {
    /// The reader's error, prefixed with what it was reading.
    fn bad(&self, s: &str) -> CodecError {
        CodecError::BadArg(format!("{}: {}", self.what, s))
    }

    fn ws(&mut self) {
        while matches!(self.s.get(self.pos), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn eat(&mut self, lit: &str) -> Result<(), CodecError> {
        if self.s[self.pos..].starts_with(lit.as_bytes()) {
            self.pos += lit.len();
            Ok(())
        } else {
            Err(self.bad(&format!("expected {:?} at byte {}", lit, self.pos)))
        }
    }

    pub(crate) fn value(&mut self) -> Result<Value, CodecError> {
        self.ws();
        match self.s.get(self.pos) {
            Some(b'{') => {
                self.pos += 1;
                let mut o = Vec::new();
                self.ws();
                if self.s.get(self.pos) == Some(&b'}') {
                    self.pos += 1;
                    return Ok(Value::Obj(o));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.ws();
                    self.eat(":")?;
                    let v = self.value()?;
                    o.push((k, v));
                    self.ws();
                    match self.s.get(self.pos) {
                        Some(b',') => self.pos += 1,
                        Some(b'}') => {
                            self.pos += 1;
                            return Ok(Value::Obj(o));
                        }
                        _ => return Err(self.bad(&format!("bad object at byte {}", self.pos))),
                    }
                }
            }
            Some(b'[') => {
                self.pos += 1;
                let mut a = Vec::new();
                self.ws();
                if self.s.get(self.pos) == Some(&b']') {
                    self.pos += 1;
                    return Ok(Value::Arr(a));
                }
                loop {
                    a.push(self.value()?);
                    self.ws();
                    match self.s.get(self.pos) {
                        Some(b',') => self.pos += 1,
                        Some(b']') => {
                            self.pos += 1;
                            return Ok(Value::Arr(a));
                        }
                        _ => return Err(self.bad(&format!("bad array at byte {}", self.pos))),
                    }
                }
            }
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b't') => self.eat("true").map(|_| Value::Bool(true)),
            Some(b'f') => self.eat("false").map(|_| Value::Bool(false)),
            Some(b'n') => self.eat("null").map(|_| Value::Null),
            Some(b'0'..=b'9') => {
                let start = self.pos;
                while matches!(self.s.get(self.pos), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
                let digits = core::str::from_utf8(&self.s[start..self.pos]).unwrap_or("");
                if digits.len() > 1 && digits.starts_with('0') {
                    return Err(self.bad("number with a leading zero"));
                }
                digits.parse().map(Value::Num).map_err(|_| self.bad("number"))
            }
            _ => Err(self.bad(&format!("unexpected byte at {}", self.pos))),
        }
    }

    fn string(&mut self) -> Result<String, CodecError> {
        self.eat("\"")?;
        let mut out = String::new();
        loop {
            let start = self.pos;
            while !matches!(self.s.get(self.pos), Some(b'"' | b'\\') | None) {
                self.pos += 1;
            }
            out.push_str(core::str::from_utf8(&self.s[start..self.pos]).map_err(|_| self.bad("utf-8"))?);
            match self.s.get(self.pos) {
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    let c = *self.s.get(self.pos + 1).ok_or_else(|| self.bad("escape"))?;
                    self.pos += 2;
                    match c {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hex = self.s.get(self.pos..self.pos + 4).ok_or_else(|| self.bad("\\u"))?;
                            let hex = core::str::from_utf8(hex).map_err(|_| self.bad("\\u"))?;
                            let n = u32::from_str_radix(hex, 16).map_err(|_| self.bad("\\u"))?;
                            self.pos += 4;
                            out.push(char::from_u32(n).ok_or_else(|| self.bad("\\u surrogate"))?);
                        }
                        _ => return Err(self.bad("escape")),
                    }
                }
                _ => return Err(self.bad("unterminated string")),
            }
        }
    }
}
