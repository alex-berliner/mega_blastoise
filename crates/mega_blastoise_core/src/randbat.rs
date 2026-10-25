extern crate alloc;

pub mod gen3;

use alloc::string::String;
use alloc::vec::Vec;

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

/// Canonical two-team seed offset (golden ratio × 2⁶⁴).
/// Add to one seed to get a well-separated second seed.
pub const TEAM_SEED_SALT: u64 = 0x9e3779b97f4a7c15;

// ── Teams for the PS engine ──────────────────────────────────────────────────

/// An index below `n` from the next draw, the way the device takes it: the
/// RP2350 and wasm32 have a 32-bit `usize`, so `draw as usize` keeps the low
/// 32 bits there and all 64 on the host. Taking the low 32 bits everywhere
/// makes one seed draw the same team on the pico, the web build and the
/// host (the pico is the source of truth).
fn pick(rng: &mut u64, n: usize) -> usize {
    xorshift64(rng) as u32 as usize % n
}

/// Whether `ps_sim` has ported `kind:id` for `gen` (`ps_sim::mods::ported`).
fn ps_ported(gen: u8, kind: &str, id: &str) -> bool {
    ps_sim::mods::ported(gen)
        .iter()
        .any(|e| e.split_once(':') == Some((kind, id)))
}

/// One team for the PS engine, in the protocol's packed form (what
/// `>player` carries, on both transports): `count` distinct mons drawn from
/// `RANDBAT_POOL` by a partial Fisher-Yates shuffle on `xorshift64`, each with
/// its randbat level and moves.
///
/// While `ps_sim` is still being ported, a battle that reaches an effect
/// with no port stops (the sim panics `unported: <id>`). So each mon keeps
/// only the moves `ps_sim::mods::ported(gen)` lists, topped back up to its
/// randbat move count with ported moves drawn from the same seed, as the
/// protocol diff's `--teams wide` restricts its pools. Once a generation is
/// fully ported the filter keeps every move and the teams are the plain
/// randbat sets.
///
/// Stat investment follows PS's random battles (`data/random-battles/gen1/teams.ts`,
/// `gen3/teams.ts` `randomSet`): 255 EVs per stat in gens 1
/// and 2, 85 in gen 3; IVs are left at PS's default (31, which gens 1
/// and 2 read as DV 15).
pub fn draw_ps_team(dex: &'static ps_dex::Dex, seed: u64, count: usize) -> ps_protocol::PackedTeam {
    use ps_protocol::{Name, PackedSet, PackedTeam, StatTable};

    let gen = dex.gen;
    let id = |s: &str| -> String { s.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect() };
    let n = RANDBAT_POOL.len();
    let take = count.min(n).min(ps_protocol::MAX_TEAM);
    let mut rng = if seed == 0 { 0xdeadbeef } else { seed };
    // The ported move pool, in dex order (for the top-up draws).
    let pool: Vec<&'static str> = dex
        .moves
        .iter()
        .map(|m| m.name.as_str())
        .filter(|name| ps_ported(gen, "move", &id(name)))
        .collect();

    let mut indices: Vec<usize> = (0..n).collect();
    let mut team = PackedTeam::new();
    for i in 0..take {
        let j = i + pick(&mut rng, n - i);
        indices.swap(i, j);
        let e = &RANDBAT_POOL[indices[i]];
        let Some(species) = dex.species_id(&id(e.species)) else { continue };
        let mut set = PackedSet {
            // The dex's canonical name: the sprite table and the font are
            // keyed on it (roster strings carry curly apostrophes).
            name: Name::from_str(dex.species_def(species).name.as_str()).unwrap_or_default(),
            level: e.level as u16,
            happiness: 255,
            evs: Some(StatTable::splat(if gen <= 2 { 255 } else { 85 })),
            ..Default::default()
        };
        let mut moves: Vec<&str> = e
            .moves
            .iter()
            .filter_map(|m| dex.move_id(&id(m)).map(|mv| dex.move_def(mv).name.as_str()))
            .filter(|name| ps_ported(gen, "move", &id(name)))
            .collect();
        let want = e.moves.len().clamp(1, ps_protocol::MAX_MOVES);
        let mut tries = 0;
        while moves.len() < want && !pool.is_empty() && tries < 64 {
            tries += 1;
            let m = pool[pick(&mut rng, pool.len())];
            if !moves.contains(&m) {
                moves.push(m);
            }
        }
        for m in moves.iter().take(ps_protocol::MAX_MOVES) {
            let _ = set.moves.push(Name::from_str(m).unwrap_or_default());
        }
        if set.moves.is_empty() {
            continue;
        }
        let _ = team.push(set);
    }
    team
}

/// Two PS-engine teams from one seed, the second drawn from the seed plus
/// [`TEAM_SEED_SALT`]. Returns `[p1, p2]`.
pub fn draw_two_ps_teams(dex: &'static ps_dex::Dex, seed: u64, count: usize) -> [ps_protocol::PackedTeam; 2] {
    [draw_ps_team(dex, seed, count), draw_ps_team(dex, seed.wrapping_add(TEAM_SEED_SALT), count)]
}

/// The two teams a device battle of `dex`'s generation plays: Gen 1 draws
/// [`draw_two_ps_teams`] (the randbat roster), Gen 3 draws Showdown's
/// `gen3randombattle` sets, with their items and abilities ([`gen3`]).
/// Returns `[p1, p2]`.
pub fn draw_device_teams(dex: &'static ps_dex::Dex, seed: u64, count: usize) -> [ps_protocol::PackedTeam; 2] {
    if dex.gen == 3 {
        gen3::draw_two_teams(seed, count)
    } else {
        draw_two_ps_teams(dex, seed, count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(team: &ps_protocol::PackedTeam) -> Vec<&str> {
        team.iter().map(|s| s.name.as_str()).collect()
    }

    /// One seed, one pair of teams, on every target. The expected teams are
    /// what the web build (wasm32, a 32-bit `usize` like the RP2350) printed
    /// for these seeds' 3-on-3 Gen 1 battles; seed 1 is the UI probe's.
    /// Before [`pick`] the host kept all 64 bits of each draw and drew
    /// other teams.
    #[test]
    fn a_seed_draws_the_devices_teams_on_the_host() {
        let cases: [(u64, [&str; 3], [&str; 3]); 3] = [
            (1, ["Nidoran-M", "Zapdos", "Omastar"], ["Sandshrew", "Fearow", "Clefable"]),
            (2, ["Dragonair", "Beedrill", "Fearow"], ["Doduo", "Dragonair", "Dodrio"]),
            (7777, ["Drowzee", "Articuno", "Venusaur"], ["Lapras", "Psyduck", "Arcanine"]),
        ];
        for (seed, white, red) in cases {
            let [p1, p2] = draw_two_ps_teams(&ps_dex::GEN1, seed, 3);
            assert_eq!(names(&p1), white, "seed {seed}, White");
            assert_eq!(names(&p2), red, "seed {seed}, Red");
        }
    }

    /// The draw reads the low 32 bits of each xorshift step on every target.
    #[test]
    fn pick_keeps_the_low_32_bits() {
        let mut a = 0x1234_5678_9abc_def0u64;
        let mut b = a;
        for n in [7usize, 151, 1000, 65_537] {
            let raw = xorshift64(&mut b);
            assert_eq!(pick(&mut a, n), (raw & 0xffff_ffff) as usize % n);
        }
    }
}
