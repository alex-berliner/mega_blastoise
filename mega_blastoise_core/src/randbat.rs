extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use gen1_battle::{MonData, MoveSlot};

pub struct RandBatEntry {
    pub species: &'static str,
    pub level: u8,
    pub moves: &'static [&'static str],
}

include!(concat!(env!("OUT_DIR"), "/roster.rs"));

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Pick `count` distinct Pokémon at random from `RANDBAT_POOL` and return them
/// as a team ready for `gen1_battle::PublicCoreBattle::update_team`.
pub fn draw_randbat_team(seed: u64, count: usize) -> Vec<MonData> {
    let n = RANDBAT_POOL.len();
    let take = count.min(n);
    let mut rng = if seed == 0 { 0xdeadbeef } else { seed };

    // Partial Fisher-Yates: swap `take` elements to the front of an index array.
    let mut indices: Vec<usize> = (0..n).collect();
    let mut team = Vec::with_capacity(take);
    for i in 0..take {
        let j = i + (xorshift64(&mut rng) as usize % (n - i));
        indices.swap(i, j);
        let e = &RANDBAT_POOL[indices[i]];
        team.push(MonData {
            // Leave the name empty: the engine fills in the species' canonical
            // display name, which the sprite table and ASCII OLED font are
            // keyed on. Roster strings carry typographic characters (curly
            // apostrophes) that would miss the sprite and render as '?'.
            name: String::new(),
            species: String::from(e.species),
            ability: Some(String::from("No Ability")),
            moves: e
                .moves
                .iter()
                .map(|&m| MoveSlot {
                    name: String::from(m),
                    id: String::from(m),
                    typ: String::new(),
                    pp: 0,
                    max_pp: 0,
                    disabled: false,
                    target: 0,
                })
                .collect(),
            level: e.level,
            ..Default::default()
        });
    }
    team
}

/// Canonical two-team seed offset (golden ratio × 2⁶⁴).
/// Add to one seed to get a well-separated second seed.
pub const TEAM_SEED_SALT: u64 = 0x9e3779b97f4a7c15;

/// Draw two independent random-battle teams from a single seed.
/// Returns `(red_team, blue_team)`.
pub fn draw_two_randbat_teams(seed: u64, count: usize) -> (Vec<MonData>, Vec<MonData>) {
    (
        draw_randbat_team(seed, count),
        draw_randbat_team(seed.wrapping_add(TEAM_SEED_SALT), count),
    )
}
