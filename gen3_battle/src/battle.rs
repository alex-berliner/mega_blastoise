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
use crate::data::{move_by_id, species_by_id, Boost, MoveEntry, SecondaryEffect, SpeciesEntry, Status};
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
    /// Accuracy and evasion stages, which live outside the five stats.
    pub acc_stage: i8,
    pub eva_stage: i8,
    pub moves: Vec<MoveSlot>,
    pub status: Option<Status>,
    /// Turns badly poisoned, driving Toxic's growing residual. Meaningless
    /// unless `status` is [`Status::Toxic`]; resets when the mon leaves.
    pub toxic_n: u8,
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
            acc_stage: 0,
            eva_stage: 0,
            moves: moves.iter().filter_map(|m| MoveSlot::new(m)).collect(),
            status: None,
            toxic_n: 0,
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

    pub fn burned(&self) -> bool {
        self.status == Some(Status::Burn)
    }

    /// Speed after paralysis: quartered in Gen 3.
    pub fn effective_speed(&self) -> u16 {
        let spe = crate::stats::apply_stage(self.spe, self.stages[Stat::Spe as usize]);
        if self.status == Some(Status::Paralysis) { spe / 4 } else { spe }
    }

    /// Whether `status` can land on this mon under Gen 3 rules: nothing
    /// stacks on an existing status, Fire types cannot burn, Ice types
    /// cannot freeze, and Poison/Steel types cannot be poisoned.
    pub fn can_receive(&self, status: Status) -> bool {
        if self.status.is_some() || self.fainted() {
            return false;
        }
        let has = |t: Type| self.types().0 == t || self.types().1 == t;
        match status {
            Status::Burn => !has(Type::Fire),
            Status::Freeze => !has(Type::Ice),
            Status::Poison | Status::Toxic => !has(Type::Poison) && !has(Type::Steel),
            Status::Paralysis | Status::Sleep => true,
        }
    }

    /// Move one stage by `delta`, clamped to ±6 like every stage is.
    pub fn apply_boost(&mut self, boost: Boost, delta: i8) {
        let slot = match boost {
            Boost::Atk => &mut self.stages[Stat::Atk as usize],
            Boost::Def => &mut self.stages[Stat::Def as usize],
            Boost::Spe => &mut self.stages[Stat::Spe as usize],
            Boost::SpAtk => &mut self.stages[Stat::SpAtk as usize],
            Boost::SpDef => &mut self.stages[Stat::SpDef as usize],
            Boost::Acc => &mut self.acc_stage,
            Boost::Eva => &mut self.eva_stage,
        };
        *slot = (*slot + delta).clamp(-6, 6);
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
    /// The move's secondary effect procs this turn.
    pub secondary: bool,
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
    /// A status landed on `side`'s active mon.
    Statused { side: u8, status: Status },
    /// A secondary moved one of `side`'s active mon's stat stages.
    Boosted { side: u8, boost: Boost, delta: i8 },
    /// `side`'s mon could not move: frozen solid or fast asleep.
    Cant { side: u8, status: Status },
    /// End-of-turn burn or poison damage.
    Residual { side: u8, amount: u16, status: Status },
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

        // Switches resolve before any move, in side order. Leaving the field
        // resets a Toxic count: the poison stays, the clock starts over.
        for side in 0..2 {
            if let Choice::Switch(idx) = choices[side] {
                if idx < self.sides[side].party.len() && !self.sides[side].party[idx].fainted() {
                    self.sides[side].mon_mut().toxic_n = 0;
                    self.sides[side].active = idx;
                    events.push(Event::Switched { side: side as u8 + 1, party_index: idx });
                }
            }
        }

        // Then moves: priority bracket first, Speed inside a bracket.
        let first = self.first_mover(&choices);
        for side in [first, 1 - first] {
            if self.over() {
                break;
            }
            if let Choice::Move(index) = choices[side] {
                self.use_move(side, index, script.seats[side], &mut events);
            }
        }

        // End of turn: burn and poison tick 1/8 max HP, Toxic a sixteenth
        // times the turns it has held, faster side first — the same order the
        // games resolve residuals in.
        let first = self.faster_side();
        for side in [first, 1 - first] {
            if self.over() {
                break;
            }
            let mon = self.sides[side].mon();
            if mon.fainted() {
                continue;
            }
            let status = match mon.status {
                Some(s @ (Status::Burn | Status::Poison | Status::Toxic)) => s,
                _ => continue,
            };
            let amount = if status == Status::Toxic {
                let mon = self.sides[side].mon_mut();
                mon.toxic_n = (mon.toxic_n + 1).min(15);
                (mon.max_hp / 16).max(1) * mon.toxic_n as u16
            } else {
                (self.sides[side].mon().max_hp / 8).max(1)
            };
            let mon = self.sides[side].mon_mut();
            let amount = amount.min(mon.hp);
            mon.hp -= amount;
            events.push(Event::Residual { side: side as u8 + 1, amount, status });
            if mon.fainted() {
                events.push(Event::Fainted { side: side as u8 + 1 });
                if let Some(next) = self.sides[side].first_healthy() {
                    self.sides[side].active = next;
                    events.push(Event::Switched { side: side as u8 + 1, party_index: next });
                }
            }
        }

        if let Some(win) = self.winner() {
            events.push(Event::Win { side: win });
        }
        events
    }

    /// Who moves first given the chosen moves: higher priority bracket, then
    /// [`Battle::faster_side`] within it. A switch resolves before moves and
    /// takes no bracket.
    fn first_mover(&mut self, choices: &[Choice; 2]) -> usize {
        let prio = |side: usize| match choices[side] {
            Choice::Move(i) => {
                self.sides[side].mon().moves.get(i).map(|s| s.entry.priority).unwrap_or(0)
            }
            Choice::Switch(_) => 0,
        };
        let (p0, p1) = (prio(0), prio(1));
        match p0.cmp(&p1) {
            core::cmp::Ordering::Greater => 0,
            core::cmp::Ordering::Less => 1,
            core::cmp::Ordering::Equal => self.faster_side(),
        }
    }

    /// Which side moves first this turn: higher Speed (paralysis included),
    /// RNG on a tie.
    fn faster_side(&mut self) -> usize {
        let s0 = self.sides[0].mon().effective_speed();
        let s1 = self.sides[1].mon().effective_speed();
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
        // Frozen solid or fast asleep: the turn is spent, no PP moves. Thaw
        // and wake chances are turn-level RNG the scripts pin off, matching
        // the reference runs. Flame Wheel and Sacred Fire are the era's two
        // moves usable while frozen: they cure the user and proceed.
        if let Some(s @ (Status::Freeze | Status::Sleep)) = self.sides[side].mon().status {
            let self_thaw = s == Status::Freeze
                && self.sides[side].mon().moves.get(index).is_some_and(|slot| {
                    matches!(slot.entry.id, "flamewheel" | "sacredfire")
                });
            if self_thaw {
                self.sides[side].mon_mut().status = None;
            } else {
                events.push(Event::Cant { side: side as u8 + 1, status: s });
                return;
            }
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
        // Unscripted rolls fold in the accuracy/evasion stages the Gen 3
        // way: one combined stage, (3+s)/3 above zero and 3/(3-s) below.
        let acc = slot.entry.accuracy;
        let hit = match script {
            Some(s) => acc == 0 || s.hit,
            None => {
                acc == 0 || {
                    let s = (self.sides[side].mon().acc_stage - self.sides[foe].mon().eva_stage)
                        .clamp(-6, 6) as i32;
                    let eff = if s >= 0 {
                        acc as u32 * (3 + s as u32) / 3
                    } else {
                        acc as u32 * 3 / (3 - s) as u32
                    };
                    self.rng.below(100) < eff
                }
            }
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
        // The move's secondary, decided by the script (or the RNG in play).
        // This runs BEFORE the thaw below, matching the reference sim: a
        // Fire move's burn chance is blocked by the freeze it is about to
        // cure, because the target still carries frz when secondaries apply.
        // A 100% secondary (Zap Cannon's paralysis) is a certainty, not a
        // roll, so the script has no say in it — matching the reference sim,
        // whose sub-certain roll can't come up short of 100.
        let certain = slot.entry.secondary.is_some_and(|sec| sec.chance >= 100);
        let proc = certain
            || match script {
                Some(s) => s.secondary,
                None => slot
                    .entry
                    .secondary
                    .map(|sec| self.rng.below(100) < sec.chance as u32)
                    .unwrap_or(false),
            };
        if proc {
            match slot.entry.secondary.map(|sec| sec.effect) {
                Some(SecondaryEffect::Status(status)) => {
                    if self.sides[foe].mon().can_receive(status) {
                        let target = self.sides[foe].mon_mut();
                        target.status = Some(status);
                        if status == Status::Toxic {
                            target.toxic_n = 0;
                        }
                        events.push(Event::Statused { side: foe as u8 + 1, status });
                    }
                }
                Some(SecondaryEffect::Boosts(list)) => {
                    if !self.sides[foe].mon().fainted() {
                        for &(boost, delta) in list {
                            self.sides[foe].mon_mut().apply_boost(boost, delta);
                            events.push(Event::Boosted { side: foe as u8 + 1, boost, delta });
                        }
                    }
                }
                None => {}
            }
        }

        // A Fire-type hit thaws a frozen target — game rule, not RNG. A
        // knocked-out target keeps its freeze; there is nothing left to thaw.
        if self.sides[foe].mon().status == Some(Status::Freeze)
            && slot.move_type() == Type::Fire
            && !self.sides[foe].mon().fainted()
        {
            self.sides[foe].mon_mut().status = None;
        }

        let target = self.sides[foe].mon_mut();
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
                burned: a.burned(),
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
