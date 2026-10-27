//! Packed-team codec: PS `sim/teams.ts` `pack` / `unpack`, field by field.
//!
//! [`PackedSet`] keeps PS's packed text (names stay as `packName` left
//! them), so this is pure syntax: [`unpack`] splits a packed string into its
//! fields the way `unpack` walks it, and [`pack`] writes them back the way
//! `pack` does. A packed string from PS's `Teams.pack` re-packs byte for byte.
//! Resolving the names (PS `unpackName`, the `0` / `1` / `H` / `S` ability
//! slots) needs the dex and is the sim's job (`crate::team`).

use alloc::format;
use alloc::string::{String, ToString};

use crate::ident::Gender;
use crate::team::{PackedMisc, PackedSet, PackedTeam, ShortName, StatTable};
use crate::{BoundedStr, Name};

use super::line::CodecError;

/// PS `pack`: the sets joined by `]`.
pub fn pack(team: &PackedTeam) -> String {
    let mut buf = String::new();
    for (i, set) in team.iter().enumerate() {
        if i > 0 {
            buf.push(']');
        }
        push_set(&mut buf, set);
    }
    buf
}

/// PS `pack` for one set.
pub fn pack_set(set: &PackedSet) -> String {
    let mut buf = String::new();
    push_set(&mut buf, set);
    buf
}

fn push_set(buf: &mut String, set: &PackedSet) {
    // name, species (empty when it packs like the name), item, ability
    buf.push_str(&set.name);
    buf.push('|');
    buf.push_str(&set.species);
    buf.push('|');
    buf.push_str(&set.item);
    buf.push('|');
    buf.push_str(&set.ability);
    // moves
    buf.push('|');
    for (i, m) in set.moves.iter().enumerate() {
        if i > 0 {
            buf.push(',');
        }
        buf.push_str(m);
    }
    // nature
    buf.push('|');
    buf.push_str(&set.nature);
    // evs: `set.evs[stat] || ''`, and empty when every value is
    buf.push('|');
    if let Some(evs) = set.evs {
        let s = stats_str(evs, 0);
        if s != ",,,,," {
            buf.push_str(&s);
        }
    }
    // gender
    buf.push('|');
    if let Some(g) = set.gender {
        buf.push_str(match g {
            Gender::M => "M",
            Gender::F => "F",
            Gender::N => "N",
        });
    }
    // ivs: 31 written as empty, and empty when every value is
    buf.push('|');
    if let Some(ivs) = set.ivs {
        let s = stats_str(ivs, 31);
        if s != ",,,,," {
            buf.push_str(&s);
        }
    }
    // shiny
    buf.push('|');
    if set.shiny {
        buf.push('S');
    }
    // level
    buf.push('|');
    if set.level != 100 && set.level != 0 {
        buf.push_str(&set.level.to_string());
    }
    // happiness
    buf.push('|');
    if set.happiness != 255 {
        buf.push_str(&set.happiness.to_string());
    }
    if let Some(misc) = &set.misc {
        buf.push(',');
        buf.push_str(&misc.hp_type);
        buf.push(',');
        buf.push_str(&misc.pokeball);
        buf.push(',');
        if misc.gigantamax {
            buf.push('G');
        }
        buf.push(',');
        if misc.dynamax_level != 10 {
            buf.push_str(&misc.dynamax_level.to_string());
        }
        buf.push(',');
        buf.push_str(&misc.tera_type);
    }
}

/// Six stats joined by `,`, each written empty when it equals `blank`.
fn stats_str(t: StatTable, blank: u16) -> String {
    let mut s = String::new();
    for (i, v) in t.to_array().into_iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        if v != blank {
            s.push_str(&v.to_string());
        }
    }
    s
}

/// PS `unpack`: one [`PackedSet`] per `]`-separated set, at most
/// [`crate::MAX_TEAM`].
pub fn unpack(buf: &str) -> Result<PackedTeam, CodecError> {
    if buf.is_empty() {
        return Err(bad("empty team"));
    }
    let mut team = PackedTeam::new();
    for set in buf.split(']') {
        team.push(unpack_set(set)?).map_err(|_| bad("more than 6 sets"))?;
    }
    Ok(team)
}

/// PS `unpack` for one set (the text between two `]`).
pub fn unpack_set(s: &str) -> Result<PackedSet, CodecError> {
    let mut f = s.splitn(12, '|');
    let mut next = |what: &str| f.next().ok_or_else(|| bad(&format!("no {} field in {:?}", what, s)));
    let name = name_of(next("name")?)?;
    let species = name_of(next("species")?)?;
    let item = name_of(next("item")?)?;
    let ability = name_of(next("ability")?)?;
    let moves_s = next("moves")?;
    let nature = short(next("nature")?)?;
    let evs = stats(next("evs")?, 0)?;
    let gender = match next("gender")? {
        "" => None,
        "M" => Some(Gender::M),
        "F" => Some(Gender::F),
        "N" => Some(Gender::N),
        g => return Err(bad(&format!("gender {:?}", g))),
    };
    let ivs = stats(next("ivs")?, 31)?;
    let shiny = match next("shiny")? {
        "" => false,
        "S" => true,
        x => return Err(bad(&format!("shiny {:?}", x))),
    };
    let level = match next("level")? {
        "" => 100,
        l => not_default(canonical_int::<u16>(l, "level")?, 100, "level")?,
    };
    let rest = next("happiness")?;
    let mut set = PackedSet { name, species, item, ability, nature, evs, gender, ivs, shiny, level, ..Default::default() };
    if !moves_s.is_empty() {
        for m in moves_s.split(',') {
            set.moves.push(name_of(m)?).map_err(|_| bad("more than 4 moves"))?;
        }
    }
    let mut misc = rest.split(',');
    set.happiness = match misc.next().unwrap_or("") {
        "" => 255,
        h => not_default(canonical_int::<u8>(h, "happiness")?, 255, "happiness")?,
    };
    if rest.contains(',') {
        let mut part = |what: &str| misc.next().ok_or_else(|| bad(&format!("no {} in {:?}", what, rest)));
        let hp_type = short(part("hpType")?)?;
        let pokeball = short(part("pokeball")?)?;
        let gigantamax = match part("gigantamax")? {
            "" => false,
            "G" => true,
            x => return Err(bad(&format!("gigantamax {:?}", x))),
        };
        let dynamax_level = match part("dynamaxLevel")? {
            "" => 10,
            d => not_default(canonical_int::<u8>(d, "dynamaxLevel")?, 10, "dynamaxLevel")?,
        };
        let tera_type = short(part("teraType")?)?;
        if misc.next().is_some() {
            return Err(bad(&format!("misc field {:?}", rest)));
        }
        set.misc = Some(PackedMisc { hp_type, pokeball, gigantamax, dynamax_level, tera_type });
    }
    Ok(set)
}

/// A stats field: empty for none, else six comma-separated values with
/// `blank` written as empty.
fn stats(s: &str, blank: u16) -> Result<Option<StatTable>, CodecError> {
    if s.is_empty() {
        return Ok(None);
    }
    let mut a = [blank; 6];
    let mut n = 0;
    for (i, v) in s.split(',').enumerate() {
        if i >= 6 {
            return Err(bad(&format!("stats {:?}", s)));
        }
        if !v.is_empty() {
            a[i] = canonical_int::<u16>(v, "stat")?;
            if a[i] == blank {
                // PS never writes the blank value out, so it would not re-pack.
                return Err(bad(&format!("stats {:?} spell out {}", s, blank)));
            }
        }
        n = i + 1;
    }
    if n != 6 {
        return Err(bad(&format!("stats {:?}", s)));
    }
    Ok(Some(StatTable::from_array(a)))
}

/// A packed number PS would have written as empty is refused: it would not
/// re-pack.
fn not_default<T: PartialEq + core::fmt::Display>(n: T, default: T, what: &str) -> Result<T, CodecError> {
    if n == default {
        return Err(bad(&format!("{} {} is written as empty", what, n)));
    }
    Ok(n)
}

/// A decimal integer exactly as JavaScript's `toString` writes it.
fn canonical_int<T: core::str::FromStr + ToString>(s: &str, what: &str) -> Result<T, CodecError> {
    let n: T = s.parse().map_err(|_| bad(&format!("{} {:?}", what, s)))?;
    if n.to_string() != s {
        return Err(bad(&format!("{} {:?}", what, s)));
    }
    Ok(n)
}

fn name_of(s: &str) -> Result<Name, CodecError> {
    Name::from_str(s).map_err(|_| bad(&format!("name {:?} too long", s)))
}

fn short(s: &str) -> Result<ShortName, CodecError> {
    BoundedStr::from_str(s).map_err(|_| bad(&format!("field {:?} too long", s)))
}

fn bad(s: &str) -> CodecError {
    CodecError::BadTeam(s.to_string())
}
