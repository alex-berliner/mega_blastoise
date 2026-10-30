//! The codec's dex names read straight from a `ps_dex` dex, with no heap.
//!
//! `ps_sim::text::dex_tables` builds the same numbering as `String` tables
//! (about 80 KB for Gen 3), which the device cannot spare beside its frame.
//! Here the numbering is the dex's own index (`MoveNum(i)` is `MoveId(i)`,
//! `TypeNum(i)` is `Type::ALL[i]`, as `dex_tables` has it) and the lookups are
//! linear scans over the static name pool, which a per-line decode can afford.
//!
//! Decoding never asks for an id string (`move_id` and the like are the
//! encoder's), so those return the display name.

use ps_dex::{Dex, SpeciesId, Type};
use ps_protocol::{AbilityNum, DexNames, ItemNum, MoveNum, NatureNum, SpeciesNum, TypeNum};

/// A [`DexNames`] over one generation's dex.
#[derive(Clone, Copy)]
pub struct DexView(pub &'static Dex);

fn id_eq(name: &str, id: &str) -> bool {
    let mut a = name.bytes().filter(u8::is_ascii_alphanumeric).map(|b| b.to_ascii_lowercase());
    let mut b = id.bytes();
    loop {
        match (a.next(), b.next()) {
            (None, None) => return true,
            (Some(x), Some(y)) if x == y => {}
            _ => return false,
        }
    }
}

impl DexNames for DexView {
    fn move_name(&self, m: MoveNum) -> &str {
        self.0.moves.get(m.0 as usize).map_or("", |d| d.name.as_str())
    }
    fn move_id(&self, m: MoveNum) -> &str {
        self.move_name(m)
    }
    fn move_from_id(&self, id: &str) -> Option<MoveNum> {
        self.0.moves.iter().position(|d| id_eq(d.name.as_str(), id)).map(|i| MoveNum(i as u16))
    }
    fn species_name(&self, s: SpeciesNum) -> &str {
        self.0.species.get(s.0 as usize).map_or("", |d| d.name.as_str())
    }
    fn species_from_id(&self, id: &str) -> Option<SpeciesNum> {
        self.0.species.iter().position(|d| id_eq(d.name.as_str(), id)).map(|i| SpeciesNum(i as u16))
    }
    fn cosmetic_forme_name(&self, s: SpeciesNum, forme: u8) -> &str {
        let names = self.0.cosmetic_formes(SpeciesId(s.0));
        names.get((forme as usize).wrapping_sub(1)).map_or("", |n| n.as_str())
    }
    fn cosmetic_forme_from_id(&self, id: &str) -> Option<(SpeciesNum, u8)> {
        self.0.cosmetic_forme_id(id).map(|(s, k)| (SpeciesNum(s.0), k))
    }
    fn item_name(&self, i: ItemNum) -> &str {
        self.0.items.get(i.0 as usize).map_or("", |d| d.name.as_str())
    }
    fn item_id(&self, i: ItemNum) -> &str {
        self.item_name(i)
    }
    fn item_from_id(&self, id: &str) -> Option<ItemNum> {
        self.0.items.iter().position(|d| id_eq(d.name.as_str(), id)).map(|i| ItemNum(i as u16))
    }
    fn ability_name(&self, a: AbilityNum) -> &str {
        self.0.abilities.get(a.0 as usize).map_or("", |d| d.name.as_str())
    }
    fn ability_id(&self, a: AbilityNum) -> &str {
        self.ability_name(a)
    }
    fn ability_from_id(&self, id: &str) -> Option<AbilityNum> {
        self.0.abilities.iter().position(|d| id_eq(d.name.as_str(), id)).map(|i| AbilityNum(i as u16))
    }
    fn type_name(&self, t: TypeNum) -> &str {
        Type::ALL.get(t.0 as usize).map_or("", |t| t.name())
    }
    fn type_from_id(&self, id: &str) -> Option<TypeNum> {
        Type::ALL.iter().position(|t| id_eq(t.name(), id)).map(|i| TypeNum(i as u8))
    }
    fn nature_name(&self, _: NatureNum) -> &str {
        ""
    }
    fn nature_from_id(&self, _: &str) -> Option<NatureNum> {
        None
    }
}
