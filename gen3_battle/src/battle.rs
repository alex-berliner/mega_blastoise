//! A Gen 3 singles turn loop.
//!
//! Deliberately small. This resolves the parts of a turn that the arithmetic
//! core already knows how to answer: who moves first, what a hit takes off,
//! who faints, and when the battle is over. Abilities, held items and weather
//! are not here yet, and the module says so rather than pretending: see the
//! `Unsupported` note on [`Battle::step`].
//!
//! Nothing in here is shared with `gen1_battle`. A Gen 1 battle runs that
//! engine start to finish and never enters this module, which is what keeps
//! Gen 1 behaviour exactly as it was.

extern crate alloc;

use alloc::vec::Vec;

use crate::damage::{crit_denominator, damage, Attacker, Defender, MoveUse, Roll};
use crate::data::{move_by_id, species_by_id, MoveEntry, SpeciesEntry};
use crate::stats::{hp_stat, other_stat, Invest, Nature, Stat};
use crate::types::Type;

/// A tiny deterministic PRNG, so a battle can be replayed from its seed.
/// xorshift64*, which is plenty for damage rolls and crits.
#[derive(Clone, Copy, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // A zero state would stay zero forever.
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        ((x.wrapping_mul(0x2545F4914F6CDD1D)) >> 32) as u32
    }

    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }
}

/// One move on a mon, with its remaining PP.
#[derive(Clone, Copy, Debug)]
pub struct MoveSlot {
    pub entry: &'static MoveEntry,
    pub pp: u8,
    /// Set by the mon rather than the move. Only Hidden Power uses it, whose
    /// type in Gen 3 comes from the holder's IVs.
    pub typed_as: Option<Type>,
}

impl MoveSlot {
    pub fn new(id: &str) -> Option<MoveSlot> {
        let entry = move_by_id(id)?;
        Some(MoveSlot { entry, pp: entry.pp, typed_as: None })
    }

    /// A slot whose type overrides the move's own.
    pub fn typed(entry: &'static MoveEntry, typed_as: Option<Type>) -> MoveSlot {
        MoveSlot { entry, pp: entry.pp, typed_as }
    }

    /// The type this slot actually attacks with.
    pub fn move_type(&self) -> Type {
        self.typed_as.unwrap_or(self.entry.move_type)
    }
}

/// A battle-ready mon: species, level and investment resolved into stats.
#[derive(Clone, Debug)]
pub struct Mon {
    pub species: &'static SpeciesEntry,
    pub level: u8,
    pub nature: Nature,
    pub hp: u16,
    pub max_hp: u16,
    pub atk: u16,
    pub def: u16,
    pub spa: u16,
    pub spd: u16,
    pub spe: u16,
    /// Stat stages in [`Stat`] order.
    pub stages: [i8; 5],
    pub moves: Vec<MoveSlot>,
    pub burned: bool,
}

impl Mon {
    /// Build a mon at `level` with uniform investment. Per-stat IVs and EVs
    /// come later with the team builder; this is enough to fight.
    pub fn new(species_id: &str, level: u8, nature: Nature, inv: Invest, moves: &[&str]) -> Option<Mon> {
        let species = species_by_id(species_id)?;
        let b = species.base;
        let max_hp = hp_stat(b.hp, inv, level);
        let stat = |base: u16, s: Stat| other_stat(base, inv, level, nature, s);
        Some(Mon {
            species,
            level,
            nature,
            hp: max_hp,
            max_hp,
            atk: stat(b.atk, Stat::Atk),
            def: stat(b.def, Stat::Def),
            spa: stat(b.spa, Stat::SpAtk),
            spd: stat(b.spd, Stat::SpDef),
            spe: stat(b.spe, Stat::Spe),
            stages: [0; 5],
            moves: moves.iter().filter_map(|m| MoveSlot::new(m)).collect(),
            burned: false,
        })
    }

    /// Same, but with move slots already built — what the drafter uses, since
    /// a random-battle set can specify a move's type.
    pub fn with_moves(
        species_id: &str,
        level: u8,
        nature: Nature,
        inv: Invest,
        moves: Vec<MoveSlot>,
    ) -> Option<Mon> {
        let mut mon = Mon::new(species_id, level, nature, inv, &[])?;
        mon.moves = moves;
        Some(mon)
    }

    pub fn fainted(&self) -> bool {
        self.hp == 0
    }

    pub fn types(&self) -> (Type, Type) {
        self.species.types
    }

    fn stage(&self, s: Stat) -> i8 {
        self.stages[s as usize]
    }
}

/// One player's party.
#[derive(Clone, Debug)]
pub struct Side {
    pub party: Vec<Mon>,
    pub active: usize,
}

impl Side {
    pub fn new(party: Vec<Mon>) -> Side {
        Side { party, active: 0 }
    }

    pub fn mon(&self) -> &Mon {
        &self.party[self.active]
    }

    fn mon_mut(&mut self) -> &mut Mon {
        &mut self.party[self.active]
    }

    pub fn defeated(&self) -> bool {
        self.party.iter().all(|m| m.fainted())
    }

    /// First living party member that is not already out.
    pub fn first_healthy(&self) -> Option<usize> {
        self.party.iter().position(|m| !m.fainted())
    }
}

/// A scripted set of random outcomes for one seat's move, used by the parity
/// tests to force the engine down the same branch a reference simulator was
/// forced down. `None` for a seat means "use the battle's own RNG".
#[derive(Clone, Copy, Debug)]
pub struct SeatScript {
    pub hit: bool,
    pub crit: bool,
    /// 85..=100.
    pub random: u8,
}

/// Per-turn RNG script; see [`Battle::step_with`].
#[derive(Clone, Copy, Debug, Default)]
pub struct TurnScript {
    pub seats: [Option<SeatScript>; 2],
}

/// What a player does with their turn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Choice {
    /// Index into the active mon's move list.
    Move(usize),
    /// Index into the party.
    Switch(usize),
}

/// What happened, in order. The caller narrates these; nothing here formats
/// text, so the device and the tests read the same events.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
    Switched { side: u8, party_index: usize },
    Used { side: u8, move_index: usize },
    /// The move could not be used: no PP, or nothing to use.
    Failed { side: u8 },
    Damage { side: u8, amount: u16, effectiveness: u32, crit: bool },
    Fainted { side: u8 },
    /// 1 or 2, or 0 for a draw.
    Win { side: u8 },
}

/// Two sides and the RNG that decides the rolls.
#[derive(Clone, Debug)]
pub struct Battle {
    pub sides: [Side; 2],
    pub rng: Rng,
    pub turn: u32,
}

impl Battle {
    pub fn new(side1: Side, side2: Side, seed: u64) -> Battle {
        Battle { sides: [side1, side2], rng: Rng::new(seed), turn: 0 }
    }

    pub fn over(&self) -> bool {
        self.sides[0].defeated() || self.sides[1].defeated()
    }

    /// Resolve one turn.
    ///
    /// Order is switches first, then moves by Speed with the faster mon going
    /// first and ties broken by the RNG, which is how Gen 3 does it.
    ///
    /// Unsupported so far, and silently absent rather than wrongly modelled:
    /// abilities, held items, weather, move priority, secondary effects, and
    /// multi-turn moves.
    pub fn step(&mut self, choices: [Choice; 2]) -> Vec<Event> {
        self.step_with(choices, &TurnScript::default())
    }

    /// [`Battle::step`], with any scripted seats forced down their scripted
    /// branch instead of rolling. The play path always passes an empty script;
    /// the parity tests pass the same script they gave the reference sim.
    pub fn step_with(&mut self, choices: [Choice; 2], script: &TurnScript) -> Vec<Event> {
        let mut events = Vec::new();
        self.turn += 1;

        // Switches resolve before any move, in side order.
        for side in 0..2 {
            if let Choice::Switch(idx) = choices[side] {
                if idx < self.sides[side].party.len() && !self.sides[side].party[idx].fainted() {
                    self.sides[side].active = idx;
                    events.push(Event::Switched { side: side as u8 + 1, party_index: idx });
                }
            }
        }

        // Then moves, faster first.
        let first = self.faster_side();
        for side in [first, 1 - first] {
            if self.over() {
                break;
            }
            if let Choice::Move(index) = choices[side] {
                self.use_move(side, index, script.seats[side], &mut events);
            }
        }

        if let Some(win) = self.winner() {
            events.push(Event::Win { side: win });
        }
        events
    }

    /// Which side moves first this turn: higher Speed, RNG on a tie.
    fn faster_side(&mut self) -> usize {
        let s0 = crate::stats::apply_stage(self.sides[0].mon().spe, self.sides[0].mon().stage(Stat::Spe));
        let s1 = crate::stats::apply_stage(self.sides[1].mon().spe, self.sides[1].mon().stage(Stat::Spe));
        match s0.cmp(&s1) {
            core::cmp::Ordering::Greater => 0,
            core::cmp::Ordering::Less => 1,
            core::cmp::Ordering::Equal => self.rng.below(2) as usize,
        }
    }

    fn use_move(
        &mut self,
        side: usize,
        index: usize,
        script: Option<SeatScript>,
        events: &mut Vec<Event>,
    ) {
        let foe = 1 - side;
        if self.sides[side].mon().fainted() {
            return;
        }
        let Some(slot) = self.sides[side].mon().moves.get(index).copied() else {
            events.push(Event::Failed { side: side as u8 + 1 });
            return;
        };
        if slot.pp == 0 {
            events.push(Event::Failed { side: side as u8 + 1 });
            return;
        }
        self.sides[side].mon_mut().moves[index].pp -= 1;
        events.push(Event::Used { side: side as u8 + 1, move_index: index });

        // Accuracy: 0 in the table means the move never misses. A scripted
        // seat's hit/miss is decided by the script, but only for moves that
        // CAN miss, matching how the reference sim's accuracy step works.
        let acc = slot.entry.accuracy;
        let hit = match script {
            Some(s) => acc == 0 || s.hit,
            None => acc == 0 || self.rng.below(100) < acc as u32,
        };
        if !hit {
            return;
        }

        let (crit, random) = match script {
            Some(s) => (s.crit, s.random),
            None => (
                self.rng.below(crit_denominator(0)) == 0,
                85 + self.rng.below(16) as u8,
            ),
        };
        let (attacker, defender) = self.attack_pair(side);
        let m = MoveUse { move_type: slot.move_type(), power: slot.entry.power };
        let dealt = damage(&attacker, &defender, &m, Roll { crit, random });
        if dealt == 0 {
            return;
        }

        let eff = crate::types::effectiveness_against(m.move_type, self.sides[foe].mon().types());
        let target = self.sides[foe].mon_mut();
        let amount = (dealt as u16).min(target.hp);
        target.hp -= amount;
        events.push(Event::Damage {
            side: foe as u8 + 1,
            amount,
            effectiveness: eff,
            crit,
        });
        if target.fainted() {
            events.push(Event::Fainted { side: foe as u8 + 1 });
            // The next living party member steps in. A real forced-switch
            // prompt belongs to the caller; this keeps the battle legal.
            if let Some(next) = self.sides[foe].first_healthy() {
                self.sides[foe].active = next;
                events.push(Event::Switched { side: foe as u8 + 1, party_index: next });
            }
        }
    }

    fn attack_pair(&self, side: usize) -> (Attacker, Defender) {
        let a = self.sides[side].mon();
        let d = self.sides[1 - side].mon();
        (
            Attacker {
                level: a.level,
                atk: a.atk,
                sp_atk: a.spa,
                atk_stage: a.stage(Stat::Atk),
                sp_atk_stage: a.stage(Stat::SpAtk),
                types: a.types(),
                burned: a.burned,
            },
            Defender {
                def: d.def,
                sp_def: d.spd,
                def_stage: d.stage(Stat::Def),
                sp_def_stage: d.stage(Stat::SpDef),
                types: d.types(),
                reflect: false,
                light_screen: false,
            },
        )
    }

    fn winner(&self) -> Option<u8> {
        match (self.sides[0].defeated(), self.sides[1].defeated()) {
            (true, true) => Some(0),
            (true, false) => Some(2),
            (false, true) => Some(1),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mon(id: &str, level: u8, moves: &[&str]) -> Mon {
        Mon::new(id, level, Nature::Hardy, Invest { iv: 31, ev: 0 }, moves)
            .unwrap_or_else(|| panic!("{id} is not in the dex"))
    }

    fn battle(a: Mon, b: Mon) -> Battle {
        Battle::new(Side::new(alloc::vec![a]), Side::new(alloc::vec![b]), 42)
    }

    #[test]
    fn a_turn_damages_and_spends_pp() {
        let mut b = battle(mon("blaziken", 50, &["ember"]), mon("treecko", 50, &["pound"]));
        let before = b.sides[1].mon().hp;
        let pp_before = b.sides[0].mon().moves[0].pp;
        let events = b.step([Choice::Move(0), Choice::Move(0)]);

        assert!(events.iter().any(|e| matches!(e, Event::Used { side: 1, .. })));
        assert!(b.sides[1].mon().hp < before, "treecko should have taken a hit");
        assert_eq!(b.sides[0].mon().moves[0].pp, pp_before - 1);
    }

    #[test]
    fn the_faster_mon_moves_first() {
        // Base Speed 80 against 70, so Blaziken moves first. Asserted against
        // the stats rather than a memory of who is fast.
        let mut b = battle(mon("blaziken", 50, &["ember"]), mon("treecko", 50, &["pound"]));
        let faster = if b.sides[0].mon().spe > b.sides[1].mon().spe { 1 } else { 2 };
        assert_ne!(b.sides[0].mon().spe, b.sides[1].mon().spe, "the tie-break is a different test");
        let events = b.step([Choice::Move(0), Choice::Move(0)]);
        let first_used = events
            .iter()
            .find_map(|e| match e {
                Event::Used { side, .. } => Some(*side),
                _ => None,
            })
            .expect("someone moved");
        assert_eq!(first_used, faster);
    }

    #[test]
    fn type_effectiveness_reaches_the_events() {
        // Ember into Treecko is super effective: Fire on Grass.
        let mut b = battle(mon("blaziken", 50, &["ember"]), mon("treecko", 50, &["pound"]));
        let events = b.step([Choice::Move(0), Choice::Move(0)]);
        let eff = events
            .iter()
            .find_map(|e| match e {
                Event::Damage { side: 2, effectiveness, .. } => Some(*effectiveness),
                _ => None,
            })
            .expect("treecko took damage");
        assert_eq!(eff, 200);
    }

    #[test]
    fn a_battle_ends_when_a_side_is_out() {
        // A level 100 attacker against a level 5 defender: one hit, one win.
        let mut b = battle(mon("blaziken", 100, &["ember"]), mon("treecko", 5, &["pound"]));
        let mut saw_win = None;
        for _ in 0..8 {
            for e in b.step([Choice::Move(0), Choice::Move(0)]) {
                if let Event::Win { side } = e {
                    saw_win = Some(side);
                }
            }
            if b.over() {
                break;
            }
        }
        assert_eq!(saw_win, Some(1), "blaziken should win");
        assert!(b.over());
    }

    #[test]
    fn a_fainted_mon_is_replaced_from_the_party() {
        let side1 = Side::new(alloc::vec![mon("blaziken", 100, &["ember"])]);
        let side2 = Side::new(alloc::vec![mon("treecko", 5, &["pound"]), mon("mudkip", 50, &["pound"])]);
        let mut b = Battle::new(side1, side2, 7);
        let events = b.step([Choice::Move(0), Choice::Move(0)]);
        assert!(events.iter().any(|e| matches!(e, Event::Fainted { side: 2 })));
        assert_eq!(b.sides[1].active, 1, "the next mon steps in");
        assert!(!b.over(), "the side still has a mon");
    }

    #[test]
    fn switching_happens_before_anyone_attacks() {
        let side1 = Side::new(alloc::vec![mon("blaziken", 50, &["ember"]), mon("mudkip", 50, &["pound"])]);
        let side2 = Side::new(alloc::vec![mon("treecko", 50, &["pound"])]);
        let mut b = Battle::new(side1, side2, 3);
        let events = b.step([Choice::Switch(1), Choice::Move(0)]);
        let switched = events.iter().position(|e| matches!(e, Event::Switched { side: 1, .. }));
        let used = events.iter().position(|e| matches!(e, Event::Used { .. }));
        assert!(switched.is_some() && used.is_some());
        assert!(switched < used, "the switch resolves first");
        assert_eq!(b.sides[0].active, 1);
    }

    #[test]
    fn a_move_with_no_pp_fails_rather_than_hitting() {
        let mut b = battle(mon("blaziken", 50, &["ember"]), mon("treecko", 50, &["pound"]));
        b.sides[0].party[0].moves[0].pp = 0;
        let events = b.step([Choice::Move(0), Choice::Move(0)]);
        assert!(events.iter().any(|e| matches!(e, Event::Failed { side: 1 })));
        assert!(!events.iter().any(|e| matches!(e, Event::Used { side: 1, .. })));
    }

    #[test]
    fn the_same_seed_replays_the_same_battle() {
        let run = || {
            let mut b = battle(mon("blaziken", 50, &["ember"]), mon("treecko", 50, &["pound"]));
            let mut all = Vec::new();
            for _ in 0..4 {
                all.extend(b.step([Choice::Move(0), Choice::Move(0)]));
            }
            all
        };
        assert_eq!(run(), run(), "a seeded battle has to be reproducible");
    }
}
