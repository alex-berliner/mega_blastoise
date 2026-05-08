extern crate alloc;

use alloc::vec::Vec;
use battler::MonData;

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
/// as a team ready for `battler::PublicCoreBattle::update_team`.
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
            name: e.species.into(),
            species: e.species.into(),
            ability: "No Ability".into(),
            moves: e.moves.iter().map(|&m| m.into()).collect(),
            level: e.level,
            ..Default::default()
        });
    }
    team
}
