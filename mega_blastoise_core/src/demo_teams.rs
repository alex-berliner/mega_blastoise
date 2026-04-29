//! Stock demo teams (singles, multiple bench Pokémon) shared by host test binary and firmware.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use battler::MonData;

fn mon(name: &str, species: &str, moves: [&str; 4]) -> MonData {
    MonData {
        name: name.into(),
        species: species.into(),
        ability: "No Ability".into(),
        moves: moves.iter().map(|s| (*s).into()).collect(),
        level: 50,
        ..Default::default()
    }
}

/// Red: four Pokémon — lead Charizard, then bench Venusaur, Pikachu, Gyarados.
pub fn demo_team_red() -> Vec<MonData> {
    vec![
        mon(
            "Charizard",
            "Charizard",
            ["Thunder Wave", "Earthquake", "Slash", "Wing Attack"],
        ),
        mon(
            "Venusaur",
            "Venusaur",
            ["Thunder Wave", "Earthquake", "Slash", "Wing Attack"],
        ),
        mon(
            "Pikachu",
            "Pikachu",
            ["Thunder Wave", "Earthquake", "Slash", "Wing Attack"],
        ),
        mon(
            "Gyarados",
            "Gyarados",
            ["Thunder Wave", "Ice Beam", "Body Slam", "Submission"],
        ),
    ]
}

/// Blue: four Pokémon — lead Blastoise, then bench Lapras, Jolteon, Machamp.
pub fn demo_team_blue() -> Vec<MonData> {
    vec![
        mon(
            "Blastoise",
            "Blastoise",
            ["Thunder Wave", "Ice Beam", "Body Slam", "Submission"],
        ),
        mon(
            "Lapras",
            "Lapras",
            ["Thunder Wave", "Ice Beam", "Body Slam", "Submission"],
        ),
        mon(
            "Jolteon",
            "Jolteon",
            ["Thunder Wave", "Earthquake", "Slash", "Wing Attack"],
        ),
        mon(
            "Machamp",
            "Machamp",
            ["Thunder Wave", "Earthquake", "Slash", "Wing Attack"],
        ),
    ]
}
