//! Dex names PS moves, items, abilities, etc. come from.
//!
//! [`Tables`] is the indexable form: one `Names` column per id kind,
//! indexed by PS's `dex.moves.all()`-order dex number. It plays two roles:
//!
//! 1. Test/dev: build it directly from a TSV (one row per entry: kind, id,
//!    display name).
//! 2. Build-time: `ps_dex` (the project's gen tables task) emits one TSV per
//!    gen and `build.rs` parses it into a `Tables` baked into the firmware.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use core::str::FromStr;

use crate::ident::{AbilityNum, ItemNum, MoveNum, NatureNum, SpeciesNum, TypeNum};

/// One column of the dex: an array of (id, display name) pairs.
#[derive(Clone, Debug, Default)]
pub struct Names {
    inner: Vec<(String, String)>,
}

impl Names {
    pub const fn new() -> Self { Self { inner: Vec::new() } }
    pub fn push(&mut self, id: &str, name: &str) {
        self.inner.push((id.to_string(), name.to_string()));
    }
    pub fn len(&self) -> usize { self.inner.len() }
    pub fn is_empty(&self) -> bool { self.inner.is_empty() }
    pub fn get(&self, idx: usize) -> Option<&(String, String)> { self.inner.get(idx) }
    pub fn from_id(&self, id: &str) -> Option<usize> {
        self.inner.iter().position(|(i, _)| i == id)
    }
    pub fn from_name(&self, name: &str) -> Option<usize> {
        self.inner.iter().position(|(_, n)| n == name)
    }
}

/// One cosmetic forme (PS `species.cosmeticFormes`): its id and name, the
/// base species, and its 1-based index in the base's list
/// ([`crate::Details::forme`]).
#[derive(Clone, Debug)]
pub struct Cosmetic {
    pub id: String,
    pub name: String,
    pub base: SpeciesNum,
    pub forme: u8,
}

/// All six columns of the dex, and the cosmetic formes.
#[derive(Clone, Debug, Default)]
pub struct Tables {
    pub moves: Names,
    pub species: Names,
    pub items: Names,
    pub abilities: Names,
    pub types: Names,
    pub natures: Names,
    pub cosmetic: Vec<Cosmetic>,
}

impl Tables {
    pub fn new() -> Self { Self::default() }
    /// Reads a TSV of `<kind>\t<id>\t<display>` rows, and
    /// `cosmetic\t<id>\t<display>\t<base species id>` rows (each after its
    /// base species, in `cosmeticFormes` order).
    pub fn from_tsv(input: &str) -> Self {
        let mut t = Self::new();
        for line in input.lines() {
            let mut parts = line.split('\t');
            let kind = match parts.next() { Some(k) => k, None => continue };
            let id = parts.next().unwrap_or("");
            let name = parts.next().unwrap_or("");
            if kind == "cosmetic" {
                let Some(base) = parts.next().and_then(|b| t.species_from_id(b)) else { continue };
                let forme = t.cosmetic.iter().filter(|c| c.base == base).count() as u8 + 1;
                t.cosmetic.push(Cosmetic { id: id.to_string(), name: name.to_string(), base, forme });
                continue;
            }
            let col: &mut Names = match kind {
                "move" => &mut t.moves,
                "species" => &mut t.species,
                "item" => &mut t.items,
                "ability" => &mut t.abilities,
                "type" => &mut t.types,
                "nature" => &mut t.natures,
                _ => continue,
            };
            col.push(id, name);
        }
        t
    }
}

impl core::fmt::Display for Tables {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (id, name) in &self.moves.inner { writeln!(f, "move\t{}\t{}", id, name)?; }
        for (id, name) in &self.species.inner { writeln!(f, "species\t{}\t{}", id, name)?; }
        for (id, name) in &self.items.inner { writeln!(f, "item\t{}\t{}", id, name)?; }
        for (id, name) in &self.abilities.inner { writeln!(f, "ability\t{}\t{}", id, name)?; }
        for (id, name) in &self.types.inner { writeln!(f, "type\t{}\t{}", id, name)?; }
        for (id, name) in &self.natures.inner { writeln!(f, "nature\t{}\t{}", id, name)?; }
        for c in &self.cosmetic {
            writeln!(f, "cosmetic\t{}\t{}\t{}", c.id, c.name, self.species_id(c.base).unwrap_or(""))?;
        }
        Ok(())
    }
}

impl FromStr for Tables {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> { Ok(Tables::from_tsv(s)) }
}

impl Tables {
    // moves ----
    pub fn move_name(&self, m: MoveNum) -> Option<&str> {
        self.moves.get(m.0 as usize).map(|(_, name)| name.as_str())
    }
    pub fn move_id(&self, m: MoveNum) -> Option<&str> {
        self.moves.get(m.0 as usize).map(|(id, _)| id.as_str())
    }
    pub fn move_from_id(&self, id: &str) -> Option<MoveNum> {
        self.moves.from_id(id).map(|i| MoveNum(i as u16))
    }
    // species ----
    pub fn species_name(&self, s: SpeciesNum) -> Option<&str> {
        self.species.get(s.0 as usize).map(|(_, name)| name.as_str())
    }
    pub fn species_id(&self, s: SpeciesNum) -> Option<&str> {
        self.species.get(s.0 as usize).map(|(id, _)| id.as_str())
    }
    pub fn species_from_id(&self, id: &str) -> Option<SpeciesNum> {
        self.species.from_id(id).map(|i| SpeciesNum(i as u16))
    }
    pub fn cosmetic_forme_name(&self, s: SpeciesNum, forme: u8) -> Option<&str> {
        self.cosmetic.iter().find(|c| c.base == s && c.forme == forme).map(|c| c.name.as_str())
    }
    pub fn cosmetic_forme_from_id(&self, id: &str) -> Option<(SpeciesNum, u8)> {
        self.cosmetic.iter().find(|c| c.id == id).map(|c| (c.base, c.forme))
    }
    // items ----
    pub fn item_name(&self, i: ItemNum) -> Option<&str> {
        self.items.get(i.0 as usize).map(|(_, name)| name.as_str())
    }
    pub fn item_id(&self, i: ItemNum) -> Option<&str> {
        self.items.get(i.0 as usize).map(|(id, _)| id.as_str())
    }
    pub fn item_from_id(&self, id: &str) -> Option<ItemNum> {
        self.items.from_id(id).map(|i| ItemNum(i as u16))
    }
    // abilities ----
    pub fn ability_name(&self, a: AbilityNum) -> Option<&str> {
        self.abilities.get(a.0 as usize).map(|(_, name)| name.as_str())
    }
    pub fn ability_id(&self, a: AbilityNum) -> Option<&str> {
        self.abilities.get(a.0 as usize).map(|(id, _)| id.as_str())
    }
    pub fn ability_from_id(&self, id: &str) -> Option<AbilityNum> {
        self.abilities.from_id(id).map(|i| AbilityNum(i as u16))
    }
    // types ----
    pub fn type_name(&self, t: TypeNum) -> Option<&str> {
        self.types.get(t.0 as usize).map(|(_, name)| name.as_str())
    }
    pub fn type_id(&self, t: TypeNum) -> Option<&str> {
        self.types.get(t.0 as usize).map(|(id, _)| id.as_str())
    }
    pub fn type_from_id(&self, id: &str) -> Option<TypeNum> {
        self.types.from_id(id).map(|i| TypeNum(i as u8))
    }
    // natures ----
    pub fn nature_name(&self, n: NatureNum) -> Option<&str> {
        self.natures.get(n.0 as usize).map(|(_, name)| name.as_str())
    }
    pub fn nature_id(&self, n: NatureNum) -> Option<&str> {
        self.natures.get(n.0 as usize).map(|(id, _)| id.as_str())
    }
    pub fn nature_from_id(&self, id: &str) -> Option<NatureNum> {
        self.natures.from_id(id).map(|i| NatureNum(i as u8))
    }
}

/// `DexNames` impl on `Tables` so it forwards into a `CodecState`.
impl crate::DexNames for Tables {
    fn move_name(&self, n: MoveNum) -> &str { self.move_name(n).unwrap_or("") }
    fn move_id(&self, n: MoveNum) -> &str { self.move_id(n).unwrap_or("") }
    fn move_from_id(&self, id: &str) -> Option<MoveNum> { self.move_from_id(id) }
    fn species_name(&self, n: SpeciesNum) -> &str { self.species_name(n).unwrap_or("") }
    fn species_from_id(&self, id: &str) -> Option<SpeciesNum> { self.species_from_id(id) }
    fn cosmetic_forme_name(&self, s: SpeciesNum, forme: u8) -> &str {
        self.cosmetic_forme_name(s, forme).unwrap_or("")
    }
    fn cosmetic_forme_from_id(&self, id: &str) -> Option<(SpeciesNum, u8)> {
        self.cosmetic_forme_from_id(id)
    }
    fn item_name(&self, n: ItemNum) -> &str { self.item_name(n).unwrap_or("") }
    fn item_id(&self, n: ItemNum) -> &str { self.item_id(n).unwrap_or("") }
    fn item_from_id(&self, id: &str) -> Option<ItemNum> { self.item_from_id(id) }
    fn ability_name(&self, n: AbilityNum) -> &str { self.ability_name(n).unwrap_or("") }
    fn ability_id(&self, n: AbilityNum) -> &str { self.ability_id(n).unwrap_or("") }
    fn ability_from_id(&self, id: &str) -> Option<AbilityNum> { self.ability_from_id(id) }
    fn type_name(&self, n: TypeNum) -> &str { self.type_name(n).unwrap_or("") }
    fn type_from_id(&self, id: &str) -> Option<TypeNum> { self.type_from_id(id) }
    fn nature_name(&self, n: NatureNum) -> &str { self.nature_name(n).unwrap_or("") }
    fn nature_from_id(&self, id: &str) -> Option<NatureNum> { self.nature_from_id(id) }
}
