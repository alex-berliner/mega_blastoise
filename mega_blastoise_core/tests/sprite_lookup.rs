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
