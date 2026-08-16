//! Gen 3 held items.
//!
//! Same shape as [`crate::ability`]: the reference sim hangs each item off one
//! of its events, those events are stages this engine already runs, and this
//! module answers a question at each one. The table is written out here rather
//! than vendored because most of what matters lives inside the handler body in
//! the reference data — which type a booster boosts, which species an item only
//! works for — and a hand-written line per item is both shorter and easier to
//! check against the source than a scraper would be.
//!
//! Gen 3 specifics worth naming: a type booster raises the attacking STAT by a
//! tenth, not the base power. Oran and Sitrus heal a flat ten and thirty, and
//! neither is eaten the moment the holder is hurt — the berries wait for the
//! residual phase, which is why a mon can be knocked out with one in hand.

use crate::ability::{Chain, X0_5, X1_5, X2};
use crate::data::{Boost, Status};
use crate::types::Type;

/// A tenth again, which is what most type boosters are worth.
pub const X1_1: u32 = 4505;
/// A twentieth again: Sea Incense alone is weaker than the rest.
pub const X1_05: u32 = 4300;

/// What an item question needs to know about its holder.
#[derive(Clone, Copy, Debug)]
pub struct Holder {
    pub item: &'static str,
    pub species: &'static str,
    /// Whether this mon has copied someone else — Metal Powder stops working
    /// the moment Ditto stops being Ditto.
    pub transformed: bool,
    pub hp: u16,
    pub max_hp: u16,
}

impl Holder {
    fn has(&self, id: &str) -> bool {
        self.item == id
    }

    fn is(&self, species: &str) -> bool {
        self.species == species
    }
}

/// The type a booster answers to, and whether that type is a physical one in
/// this era — which is the same question as which stat it raises.
fn boosted_type(item: &str) -> Option<Type> {
    Some(match item {
        "blackbelt" => Type::Fighting,
        "blackglasses" => Type::Dark,
        "charcoal" => Type::Fire,
        "dragonfang" => Type::Dragon,
        "hardstone" => Type::Rock,
        "magnet" => Type::Electric,
        "metalcoat" => Type::Steel,
        "miracleseed" => Type::Grass,
        "mysticwater" => Type::Water,
        "seaincense" => Type::Water,
        "nevermeltice" => Type::Ice,
        "poisonbarb" => Type::Poison,
        "sharpbeak" => Type::Flying,
        "silkscarf" => Type::Normal,
        "silverpowder" => Type::Bug,
        "softsand" => Type::Ground,
        "spelltag" => Type::Ghost,
        "twistedspoon" => Type::Psychic,
        _ => return None,
    })
}

/// The attacking stat, after its stages. `physical` says which stat the move
/// is actually using, so a booster only speaks for the one it belongs to.
pub fn attack_chain(user: &Holder, move_type: Type, physical: bool) -> Chain {
    let mut chain = Chain::new();
    if boosted_type(user.item) == Some(move_type) {
        chain.mul(if user.has("seaincense") { X1_05 } else { X1_1 });
    }
    if physical {
        match user.item {
            "choiceband" => chain.mul(X1_5),
            "thickclub" if user.is("cubone") || user.is("marowak") => chain.mul(X2),
            _ => {}
        }
    } else {
        match user.item {
            "lightball" if user.is("pikachu") => chain.mul(X2),
            "deepseatooth" if user.is("clamperl") => chain.mul(X2),
            "souldew" if user.is("latios") || user.is("latias") => chain.mul(X1_5),
            _ => {}
        }
    }
    chain
}

/// The defending stat, after its stages.
pub fn defence_chain(target: &Holder, physical: bool) -> Chain {
    let mut chain = Chain::new();
    if physical {
        if target.has("metalpowder") && target.is("ditto") && !target.transformed {
            chain.mul(X2);
        }
    } else {
        match target.item {
            "deepseascale" if target.is("clamperl") => chain.mul(X2),
            "souldew" if target.is("latios") || target.is("latias") => chain.mul(X1_5),
            _ => {}
        }
    }
    chain
}

/// Macho Brace drags its holder's Speed down by half.
pub fn speed_chain(mon: &Holder) -> Chain {
    let mut chain = Chain::new();
    if mon.has("machobrace") {
        chain.mul(X0_5);
    }
    chain
}

/// Extra critical-hit stages the holder's item is worth.
pub fn crit_stages(user: &Holder) -> u8 {
    match user.item {
        "scopelens" => 1,
        "luckypunch" if user.is("chansey") => 2,
        "stick" if user.is("farfetchd") => 2,
        _ => 0,
    }
}

/// Bright Powder and Lax Incense blur the holder. These are plain multiplies
/// in the sim rather than modifier chains, so they are written that way here.
pub fn accuracy_after_item(target: &Holder, accuracy: u32) -> u32 {
    match target.item {
        "brightpowder" => accuracy * 9 / 10,
        "laxincense" => accuracy * 95 / 100,
        _ => accuracy,
    }
}

/// Leftovers' sixteenth, at the sim's residual order 10, subOrder 4.
pub fn leftovers(mon: &Holder) -> bool {
    mon.has("leftovers")
}

/// Shell Bell hands back an eighth of what the move dealt.
pub fn shell_bell(mon: &Holder) -> bool {
    mon.has("shellbell")
}

/// What a status-curing berry answers to. These are `onUpdate` items: they
/// are eaten the moment the status lands, not at the end of the turn.
pub fn cures_status(mon: &Holder, status: Status) -> bool {
    match mon.item {
        "cheriberry" => status == Status::Paralysis,
        "chestoberry" => status == Status::Sleep,
        "pechaberry" => matches!(status, Status::Poison | Status::Toxic),
        "rawstberry" => status == Status::Burn,
        "aspearberry" => status == Status::Freeze,
        "lumberry" => true,
        _ => false,
    }
}

/// Persim and Lum also clear a muddled head.
pub fn cures_confusion(mon: &Holder) -> bool {
    matches!(mon.item, "persimberry" | "lumberry")
}

/// What a berry does when the residual phase finds its holder low enough.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ripe {
    None,
    /// Heal this many HP flat.
    Heal(u16),
    /// Heal an eighth of maximum, and muddle a mon whose nature dislikes the
    /// flavour — which this engine does not track, so it never muddles.
    HealEighth,
    /// Raise this stat one stage.
    Boost(Boost),
    /// Starf raises a random stat two stages; the sim samples the list, and a
    /// pinned sample takes the first that is not already maxed.
    StarfBoost,
    /// Lansat sharpens the holder's aim for crits.
    FocusEnergy,
}

/// Whether the residual phase eats this berry, and what happens if it does.
/// The healing berries wait until the holder is at half, the pinch ones until
/// a quarter.
pub fn ripens(mon: &Holder) -> Ripe {
    let half = mon.hp as u32 * 2 <= mon.max_hp as u32;
    let quarter = mon.hp as u32 * 4 <= mon.max_hp as u32;
    match mon.item {
        "oranberry" if half => Ripe::Heal(10),
        "sitrusberry" if half => Ripe::Heal(30),
        "figyberry" | "wikiberry" | "magoberry" | "aguavberry" | "iapapaberry" if half => {
            Ripe::HealEighth
        }
        "liechiberry" if quarter => Ripe::Boost(Boost::Atk),
        "ganlonberry" if quarter => Ripe::Boost(Boost::Def),
        "salacberry" if quarter => Ripe::Boost(Boost::Spe),
        "petayaberry" if quarter => Ripe::Boost(Boost::SpAtk),
        "apicotberry" if quarter => Ripe::Boost(Boost::SpDef),
        "starfberry" if quarter => Ripe::StarfBoost,
        "lansatberry" if quarter => Ripe::FocusEnergy,
        _ => Ripe::None,
    }
}

/// Whether an item is one of the berries, which is what Trick and a few other
/// effects need to know.
pub fn is_berry(item: &str) -> bool {
    item.ends_with("berry")
}
