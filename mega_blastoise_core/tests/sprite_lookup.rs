//! Every Gen 1 species display name must resolve to a sprite — both from the
//! sprite table's own keys and from the battle engine's species names.

use mega_blastoise_core::sprites::{mon_back_sprite, mon_sprite, MON_BACK_SPRITES, MON_SPRITES};

#[test]
fn every_engine_species_name_has_a_sprite() {
    let mut missing = Vec::new();
    for entry in gen1_battle::SPECIES {
        if mon_sprite(entry.name).is_none() {
            missing.push(entry.name);
        }
        if mon_back_sprite(entry.name).is_none() {
            missing.push(entry.name);
        }
    }
    assert!(missing.is_empty(), "no sprite for: {missing:?}");
}

#[test]
fn back_sprites_are_distinct_art() {
    assert_eq!(MON_BACK_SPRITES.len(), 151);
    let front = mon_sprite("Blastoise").unwrap();
    let back = mon_back_sprite("Blastoise").unwrap();
    assert_ne!(front, back, "back sprite must not be the front sprite");
}

#[test]
fn spot_checks() {
    assert!(mon_sprite("Farfetch'd").is_some(), "Farfetch'd");
    assert!(mon_sprite("Mr. Mime").is_some(), "Mr. Mime");
    assert!(mon_sprite("Nidoran-F").is_some(), "Nidoran-F");
    assert_eq!(MON_SPRITES.len(), 151);
}

/// The Gen 3 battler drafts from a 220-species random-battle pool, and the
/// colour sprite tables were built off a hand-kept list of Kanto's 151 — so
/// every mon past Mew reached the field with no art at all, and the miss was
/// silent because `front_sprite_in` just returns false and draws nothing.
/// This is the guard: the table is built from the same vendored dex the engine
/// compiles against, and this asserts the two still agree.
#[test]
fn every_gen3_species_name_has_a_color_sprite() {
    use mega_blastoise_core::{mon_back_sprite_color, mon_sprite_color};
    let mut missing = Vec::new();
    for entry in gen3_battle::data::SPECIES {
        if mon_sprite_color(entry.name).is_none() {
            missing.push((entry.name, "front"));
        }
        if mon_back_sprite_color(entry.name).is_none() {
            missing.push((entry.name, "back"));
        }
    }
    assert!(missing.is_empty(), "no colour sprite for: {missing:?}");
}
