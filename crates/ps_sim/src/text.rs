//! The pieces the `ps_protocol` text codec needs from a battle: the dex's
//! name tables and the battle's player and Pokemon names. The tests and
//! `ps_sim_cli` render the server's output through these.

use alloc::string::{String, ToString};

use ps_dex::{Dex, SpeciesId, Type};
use ps_protocol::text::names::{Cosmetic, Tables};
use ps_protocol::{BattleNames, SideId};

use crate::state::Battle;

/// The codec's dex names from a `ps_dex` dex: index `i` of each column is
/// the dex's id `i` (`MoveNum(i)` is `MoveId(i)`, `TypeNum(i)` is
/// `Type::ALL[i]`), and the cosmetic formes in `cosmeticFormes` order.
pub fn dex_tables(dex: &Dex) -> Tables {
    let mut t = Tables::new();
    let id = |name: &str| -> String { name.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect() };
    for m in dex.moves {
        t.moves.push(&id(m.name.as_str()), m.name.as_str());
    }
    for (i, s) in dex.species.iter().enumerate() {
        t.species.push(&id(s.name.as_str()), s.name.as_str());
        let base = ps_protocol::SpeciesNum(i as u16);
        for (k, name) in dex.cosmetic_formes(SpeciesId(i as u16)).iter().enumerate() {
            let (cid, name) = (id(name.as_str()), name.as_str().to_string());
            t.cosmetic.push(Cosmetic { id: cid, name, base, forme: k as u8 + 1 });
        }
    }
    for i in dex.items {
        t.items.push(&id(i.name.as_str()), i.name.as_str());
    }
    for a in dex.abilities {
        t.abilities.push(&id(a.name.as_str()), a.name.as_str());
    }
    for ty in Type::ALL {
        t.types.push(&id(ty.name()), ty.name());
    }
    t
}

/// The codec's battle names: what `>player` and the teams set.
pub struct Names<'a>(pub &'a Battle);

impl BattleNames for Names<'_> {
    fn player_name(&self, side: SideId) -> &str {
        self.0.sides[side.index()].name.as_str()
    }
    fn player_avatar(&self, side: SideId) -> &str {
        self.0.sides[side.index()].avatar.as_str()
    }
    fn player_rating(&self, side: SideId) -> &str {
        self.0.sides[side.index()].rating.as_str()
    }
    fn nickname(&self, side: SideId, slot: u8) -> &str {
        self.0.sides[side.index()].mons[slot as usize].name.as_str()
    }
}
