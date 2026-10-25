//! The seeded random battles of the budget measurement (task P2-4,
//! `docs/architecture/showdown-split-budget.md`), shared by the host probe
//! (`crates/ps_sim/tests/budget.rs`) and the board bench
//! (`crates/mega_blastoise_fw/src/bin/sim_bench.rs`, which `#[path]`-includes
//! this file), so both measure the same battles.
//!
//! `no_std`, no allocation: the battle is the caller's (it decides where
//! the 15 KB live), the sink keeps the last request per side in place, and
//! the spill only counts lines. Teams draw from `ps_sim::mods::ported(gen)`
//! (the protocol diff's `--teams wide` pools): six distinct species, four
//! distinct moves each, a random ported ability in gen 3, no items. Choices
//! are random over the request: an enabled move, or (one time in eight,
//! and always on a forced switch) a living benched Pokemon; a choice the
//! server rejects is retried as `default`.
//!
//! The driver brackets every command (`Battle::step` plus
//! `Battle::send_updates`) with [`Probe::enter`] / [`Probe::exit`]. A turn
//! is every command that answers one round of requests: the commands up to
//! and including the one after which new requests went out or the battle
//! ended ([`Exit::Turn`]); the ones before it only record a choice
//! ([`Exit::Pending`]). The two `>player` commands (team setup, `start`,
//! the leads' switch-in) are [`Exit::Setup`].

#![allow(dead_code)]

use core::sync::atomic::{AtomicU32, Ordering};

use ps_protocol::{Choice, Command, FormatSpec, Msg, Name, PackedSet, PackedTeam, Request, SideId};
use ps_sim::{Battle, Sink};

/// Seeded battles per ported gen.
pub const BATTLES_PER_GEN: u32 = 200;

/// Commands after which a battle counts as stuck (PS's turn limit ends a
/// battle at turn 1000; a random battle never comes near).
pub const COMMAND_LIMIT: u32 = 20_000;

/// The gens the bench plays: Gen 1 (every effect ported) and Gen 3 (the
/// move pipeline and the ported pools; partial).
pub const GENS: [u8; 2] = [1, 3];

/// What a command was, for the probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    /// A `>player` command.
    Setup,
    /// A choice was recorded (or rejected); the turn has not run.
    Pending,
    /// The command ran the turn: new requests went out, or the battle ended.
    Turn,
}

/// Brackets each command. Static dispatch only (a generic, no trait object).
pub trait Probe {
    fn enter(&mut self);
    fn exit(&mut self, exit: Exit);
}

/// How one battle went.
#[derive(Clone, Copy, Debug, Default)]
pub struct Outcome {
    pub turns: u32,
    pub commands: u32,
    pub rejected: u32,
    /// `Battle::turn` at the end.
    pub battle_turn: u16,
    pub ended: bool,
}

/// xorshift64, the bench's own randomness (teams and choices; the battle's
/// randomness is its PRNG, seeded by [`seed_words`]).
#[derive(Clone, Copy, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15 | 1)
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform in `0..n` (`n > 0`; the modulo bias is irrelevant here).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// The format: each gen's custom game, as the protocol diff plays it.
pub fn format(gen: u8) -> FormatSpec {
    if gen == 1 {
        FormatSpec::gen1_harness()
    } else {
        FormatSpec::gen3_harness()
    }
}

/// A battle seed as PS's four seed words, high word first.
pub fn seed_words(seed: u64) -> [u16; 4] {
    [(seed >> 48) as u16, (seed >> 32) as u16, (seed >> 16) as u16, seed as u16]
}

/// The battle seed of battle `i` of `gen`.
pub fn battle_seed(gen: u8, i: u32) -> u64 {
    0x5EED_0000_0000_0000 | (gen as u64) << 32 | i as u64
}

/// The ids of `kind` in the gen's ported list.
fn pool(gen: u8, kind: &str) -> impl Iterator<Item = &'static str> + Clone + '_ {
    ps_sim::mods::ported(gen).iter().filter_map(move |e| {
        let (k, id) = e.split_once(':')?;
        (k == kind).then_some(id)
    })
}

/// A random entry of the pool, `None` when it is empty.
fn draw(gen: u8, kind: &str, rng: &mut Rng) -> Option<&'static str> {
    let n = pool(gen, kind).count();
    if n == 0 {
        return None;
    }
    pool(gen, kind).nth(rng.below(n))
}

/// Six sets from the gen's ported pools.
pub fn team(gen: u8, rng: &mut Rng) -> PackedTeam {
    let mut team = PackedTeam::new();
    let mut species: [&str; 6] = [""; 6];
    let mut n = 0;
    while n < 6 {
        let Some(s) = draw(gen, "species", rng) else { break };
        if species[..n].contains(&s) {
            continue;
        }
        species[n] = s;
        n += 1;
        let mut set = PackedSet {
            species: Name::from_str(s).unwrap_or_default(),
            level: 100 - rng.below(30) as u16,
            happiness: 255,
            ..Default::default()
        };
        if let Some(a) = draw(gen, "ability", rng) {
            set.ability = Name::from_str(a).unwrap_or_default();
        }
        let mut moves: [&str; 4] = [""; 4];
        let mut m = 0;
        let mut tries = 0;
        while m < 4 && tries < 64 {
            tries += 1;
            let Some(mv) = draw(gen, "move", rng) else { break };
            if !moves[..m].contains(&mv) {
                moves[m] = mv;
                m += 1;
            }
        }
        for mv in &moves[..m] {
            let _ = set.moves.push(Name::from_str(mv).unwrap_or_default());
        }
        let _ = team.push(set);
    }
    team
}

/// Lines a full log handed over, all battles.
pub static SPILLED_LINES: AtomicU32 = AtomicU32::new(0);

fn spill(_: &Battle, lines: &[Msg]) {
    SPILLED_LINES.fetch_add(lines.len() as u32, Ordering::Relaxed);
}

/// The driver's [`Sink`]: the last request per side, in place.
struct Out {
    pending: [Option<Request>; 2],
    requests: u32,
    error: bool,
}

impl Sink for Out {
    fn update(&mut self, _lines: &[Msg]) {}
    fn side_line(&mut self, _side: SideId, line: &Msg) {
        if matches!(line, Msg::Error { .. }) {
            self.error = true;
        }
    }
    fn request(&mut self, side: SideId, request: &Request) {
        self.requests += 1;
        self.pending[side.index()] = Some(request.clone());
    }
}

/// A random answer to `request`.
fn pick(request: &Request, rng: &mut Rng) -> Choice {
    let side = request.side();
    let mut bench = [0u8; 6];
    let mut nb = 0;
    for (i, p) in side.pokemon.iter().enumerate() {
        if !p.active && p.condition != ps_protocol::HpStatus::FAINTED {
            bench[nb] = i as u8 + 1;
            nb += 1;
        }
    }
    match request {
        Request::Move(r) => {
            let Some(active) = r.active.first() else { return Choice::Default };
            let mut usable = [0u8; 4];
            let mut nu = 0;
            for (i, m) in active.moves.iter().enumerate() {
                if m.disabled != Some(true) {
                    usable[nu] = i as u8 + 1;
                    nu += 1;
                }
            }
            if nb > 0 && !active.trapped && (nu == 0 || rng.below(8) == 0) {
                Choice::Switch(bench[rng.below(nb)])
            } else if nu > 0 {
                Choice::Move(usable[rng.below(nu)])
            } else {
                Choice::Default
            }
        }
        Request::Switch(_) if nb > 0 => Choice::Switch(bench[rng.below(nb)]),
        _ => Choice::Default,
    }
}

/// Builds battle `i` of `gen` (call [`play`] on it). Returned by value: the
/// caller boxes it or places it.
pub fn new_battle(gen: u8, i: u32) -> Battle {
    let mut b = Battle::from_start(format(gen), seed_words(battle_seed(gen, i)), 0);
    b.spill = Some(spill);
    b
}

/// Plays battle `i` of `gen` on `b` (fresh from [`new_battle`]) to its end.
pub fn play<P: Probe>(b: &mut Battle, gen: u8, i: u32, probe: &mut P) -> Outcome {
    let mut rng = Rng::new(battle_seed(gen, i));
    let mut out = Out { pending: [None, None], requests: 0, error: false };
    let mut o = Outcome::default();
    for side in [SideId::P1, SideId::P2] {
        let name = Name::from_str(if side == SideId::P1 { "White" } else { "Red" }).unwrap_or_default();
        let cmd = Command::Player { side, name, team: team(gen, &mut rng) };
        probe.enter();
        b.step(&cmd).expect("the bench's teams are valid");
        b.send_updates(&mut out);
        probe.exit(Exit::Setup);
        o.commands += 1;
    }
    let mut fallback = [false; 2];
    while !b.ended && o.commands < COMMAND_LIMIT {
        let Some(s) = (0..2).find(|&s| out.pending[s].as_ref().is_some_and(Request::needs_choice)) else {
            panic!("battle {i} of gen {gen}: no side has a choice to make and the battle goes on");
        };
        let request = out.pending[s].take().unwrap();
        let choice = if fallback[s] { Choice::Default } else { pick(&request, &mut rng) };
        let side = if s == 0 { SideId::P1 } else { SideId::P2 };
        out.requests = 0;
        out.error = false;
        probe.enter();
        b.step(&Command::Choose { side, choice }).expect("a choice is not a team");
        b.send_updates(&mut out);
        let ran = !out.error && (out.requests > 0 || b.ended);
        probe.exit(if ran { Exit::Turn } else { Exit::Pending });
        o.commands += 1;
        if out.error {
            o.rejected += 1;
            fallback[s] = true;
            if out.pending[s].is_none() {
                out.pending[s] = Some(request);
            }
        } else {
            fallback[s] = false;
        }
        if ran {
            o.turns += 1;
            // The wait requests of this round are dropped; the next round's
            // replace the rest.
            for p in out.pending.iter_mut() {
                if p.as_ref().is_some_and(|r| !r.needs_choice()) {
                    *p = None;
                }
            }
        }
    }
    o.ended = b.ended;
    o.battle_turn = b.turn;
    o
}
