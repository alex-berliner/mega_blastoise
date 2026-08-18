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

/// The single-screen view draws BOTH generations from the colour tables, so a
/// Gen 1 species has to resolve there too. It caught a real one: Showdown
/// spells Farfetch'd with a curly apostrophe and the Gen 1 data with a
/// straight one, so rebuilding the tables off the Gen 3 dex quietly took that
/// species' art away.
#[test]
fn every_gen1_species_name_has_a_color_sprite() {
    use mega_blastoise_core::{mon_back_sprite_color, mon_sprite_color};
    let mut missing = Vec::new();
    for entry in gen1_battle::SPECIES {
        if mon_sprite_color(entry.name).is_none() {
            missing.push((entry.name, "front"));
        }
        if mon_back_sprite_color(entry.name).is_none() {
            missing.push((entry.name, "back"));
        }
    }
    assert!(missing.is_empty(), "no colour sprite for: {missing:?}");
}

/// A forme keeps its own art but answers to the base name. Both halves of
/// that broke at once: the name buffer cut "Deoxys-Attack" to twelve bytes,
/// which matched no sprite, and the plate spent its width on the suffix.
#[test]
fn formes_use_their_own_art_under_the_base_name() {
    use core::ops::Not;
    use mega_blastoise_core::oled_ctl::name_buf;
    use mega_blastoise_core::sprites_color::{mon_sprite_color, species_display_name};

    let (buf, len) = name_buf("Deoxys-Attack");
    let round_tripped = core::str::from_utf8(&buf[..len as usize]).unwrap();
    assert_eq!(round_tripped, "Deoxys-Attack", "the name was cut in transit");
    assert!(mon_sprite_color(round_tripped).is_some(), "no art for the forme");
    let forme = mon_sprite_color("Deoxys-Attack").unwrap();
    let base = mon_sprite_color("Deoxys").unwrap();
    assert!(
        core::ptr::eq(forme, base).not(),
        "the forme must not fall back to the base art",
    );

    assert_eq!(species_display_name("Deoxys-Attack"), "Deoxys");
    assert_eq!(species_display_name("Castform-Sunny"), "Castform");
    // Hyphens that are part of the name itself stay put.
    for name in ["Ho-Oh", "Nidoran-F", "Nidoran-M", "Porygon2"] {
        assert_eq!(species_display_name(name), name, "{name}");
    }
}

/// Every Gen 3 species the drafter can send out needs art, under the name the
/// engine actually uses.
#[test]
fn every_gen3_species_name_has_color_art() {
    use mega_blastoise_core::sprites_color::mon_sprite_color;
    let missing: Vec<&str> = gen3_battle::SPECIES
        .iter()
        .map(|s| s.name)
        .filter(|n| mon_sprite_color(n).is_none())
        .collect();
    assert!(missing.is_empty(), "no color art for: {missing:?}");
}
