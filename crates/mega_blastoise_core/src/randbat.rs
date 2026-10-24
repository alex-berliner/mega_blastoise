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

// ── Teams for the PS engine ──────────────────────────────────────────────────

/// Whether `ps_sim` has ported `kind:id` for `gen` (`ps_sim::mods::ported`).
fn ps_ported(gen: u8, kind: &str, id: &str) -> bool {
    ps_sim::mods::ported(gen)
        .iter()
        .any(|e| e.split_once(':') == Some((kind, id)))
}

/// One team for the PS engine, in the protocol's packed form (what
/// `>player` carries, on both transports): `count` distinct mons drawn from
/// `RANDBAT_POOL` exactly as [`draw_randbat_team`] draws them, each with its
/// randbat level and moves.
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
        let j = i + (xorshift64(&mut rng) as usize % (n - i));
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
            let pick = pool[xorshift64(&mut rng) as usize % pool.len()];
            if !moves.contains(&pick) {
                moves.push(pick);
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

/// Two PS-engine teams from one seed, as [`draw_two_randbat_teams`] salts
/// it. Returns `[p1, p2]`.
pub fn draw_two_ps_teams(dex: &'static ps_dex::Dex, seed: u64, count: usize) -> [ps_protocol::PackedTeam; 2] {
    [draw_ps_team(dex, seed, count), draw_ps_team(dex, seed.wrapping_add(TEAM_SEED_SALT), count)]
}
