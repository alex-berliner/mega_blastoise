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
use crate::data::{move_by_id, species_by_id, Boost, FixedDamage, MoveEntry, SecondaryEffect, SpeciesEntry, Status, StatusAction};
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
    /// Sleep clock: decremented before each action while asleep, waking at
    /// zero (and acting that turn). Set when sleep lands — 2 under a script,
    /// matching the pinned reference roll; 2..=5 in play.
    pub sleep_n: u8,
    /// Flinched this turn and loses its action if it has not moved yet.
    /// Cleared when the turn ends.
    pub flinched: bool,
    /// Confusion clock: 0 means not confused. Decremented before each
    /// action; at zero the confusion lifts and the move proceeds. Set when
    /// confusion lands — 2 under a script (the sim's pinned floor), 2..=5
    /// in play. Cleared by switching out.
    pub confusion_n: u8,
    /// Mid two-turn move: the slot charged last turn, releasing this turn.
    /// Any Cant loses the charge. Cleared by switching out.
    pub charging: Option<u8>,
    /// Hyper Beam landed last turn: this action is spent recharging.
    pub must_recharge: bool,
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
            sleep_n: 0,
            flinched: false,
            confusion_n: 0,
            charging: None,
            must_recharge: false,
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

    /// Speed after paralysis: quartered in Gen 3 — with the reference sim's
    /// round-to-nearest modify(), not a plain floor. (spe+2)/4 is exactly
    /// that arithmetic; the fuzzer caught a Deoxys at 239 landing on 60,
    /// not 59, and winning a "tie" it was never in.
    pub fn effective_speed(&self) -> u16 {
        let spe = crate::stats::apply_stage(self.spe, self.stages[Stat::Spe as usize]);
        if self.status == Some(Status::Paralysis) { (spe + 2) / 4 } else { spe }
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

    /// Which semi-invulnerable move this mon is mid-way through, if any:
    /// the charge turn of Fly, Dig, Bounce or Dive puts it out of reach.
    pub fn semi_invulnerable(&self) -> Option<&'static str> {
        let i = self.charging? as usize;
        let id = self.moves.get(i)?.entry.id;
        matches!(id, "fly" | "dig" | "bounce" | "dive").then_some(id)
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
    /// Full paralysis: the mon is paralyzed and this turn's 25% "can't move"
    /// roll comes up against it.
    pub immobile: bool,
    /// Strike count for a 2-5 multi-hit move; 0 lets the battle roll it.
    /// Fixed-count moves (Double Kick's 2) ignore it.
    pub hits: u8,
    /// A confused mon's coin comes up "hit yourself" this action.
    pub selfhit: bool,
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
    /// `side` spent the turn charging a two-turn move.
    Charging { side: u8 },
    /// `side` spent the turn recharging after Hyper Beam and kin.
    Recharging { side: u8 },
    /// `side`'s mon flinched and lost its action.
    Flinched { side: u8 },
    /// `side`'s mon became confused.
    ConfusionStarted { side: u8 },
    /// `side`'s mon hurt itself in confusion.
    ConfusedHit { side: u8, amount: u16 },
    /// `side`'s mon snapped out of confusion.
    ConfusionEnded { side: u8 },
    /// `side` healed `amount` by draining the damage it just dealt.
    Drained { side: u8, amount: u16 },
    /// `side` took `amount` recoil from its own move.
    Recoil { side: u8, amount: u16 },
    /// `side` healed itself for `amount` (Recover and kin).
    Healed { side: u8, amount: u16 },
    /// `side`'s paralyzed mon was fully paralyzed and lost its action.
    FullyParalyzed { side: u8 },
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
                    let out = self.sides[side].mon_mut();
                    out.toxic_n = 0;
                    out.confusion_n = 0;
                    out.charging = None;
                    out.must_recharge = false;
                    self.sides[side].active = idx;
                    events.push(Event::Switched { side: side as u8 + 1, party_index: idx });
                }
            }
        }

        // Then moves: priority bracket first, Speed inside a bracket.
        let scripted = script.seats.iter().any(|s| s.is_some());
        let first = self.first_mover(&choices, scripted);
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
        let first = self.faster_side(scripted);
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

        // A flinch lasts exactly the turn it landed in.
        for side in 0..2 {
            self.sides[side].mon_mut().flinched = false;
        }

        if let Some(win) = self.winner() {
            events.push(Event::Win { side: win });
        }
        events
    }

    /// Who moves first given the chosen moves: higher priority bracket, then
    /// [`Battle::faster_side`] within it. A switch resolves before moves and
    /// takes no bracket.
    fn first_mover(&mut self, choices: &[Choice; 2], scripted: bool) -> usize {
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
            core::cmp::Ordering::Equal => self.faster_side(scripted),
        }
    }

    /// Which side moves first this turn: higher Speed (paralysis included),
    /// RNG on a tie in play. Under a script a tie goes to player 1, matching
    /// the reference sim with its tie-shuffle pinned to insertion order.
    fn faster_side(&mut self, scripted: bool) -> usize {
        let s0 = self.sides[0].mon().effective_speed();
        let s1 = self.sides[1].mon().effective_speed();
        match s0.cmp(&s1) {
            core::cmp::Ordering::Greater => 0,
            core::cmp::Ordering::Less => 1,
            core::cmp::Ordering::Equal if scripted => 0,
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
        // Recharging after Hyper Beam and kin: the whole action is spent,
        // gated even above sleep, matching the games' priority order.
        if self.sides[side].mon().must_recharge {
            self.sides[side].mon_mut().must_recharge = false;
            events.push(Event::Recharging { side: side as u8 + 1 });
            return;
        }
        // Fast asleep: the sleep clock ticks down before each action, and at
        // zero the mon wakes and moves that same turn.
        if self.sides[side].mon().status == Some(Status::Sleep) {
            let mon = self.sides[side].mon_mut();
            mon.sleep_n = mon.sleep_n.saturating_sub(1);
            if mon.sleep_n == 0 {
                mon.status = None;
            } else {
                mon.charging = None;
                events.push(Event::Cant { side: side as u8 + 1, status: Status::Sleep });
                return;
            }
        }
        // Frozen solid: a 1-in-5 thaw each action in play (scripts pin it
        // off, matching the reference runs). Flame Wheel and Sacred Fire are
        // the era's two moves usable while frozen: they cure the user.
        if self.sides[side].mon().status == Some(Status::Freeze) {
            let self_thaw = self.sides[side].mon().moves.get(index).is_some_and(|slot| {
                matches!(slot.entry.id, "flamewheel" | "sacredfire")
            });
            let lucky = match script {
                Some(_) => false,
                None => self.rng.below(5) == 0,
            };
            if self_thaw || lucky {
                self.sides[side].mon_mut().status = None;
            } else {
                self.sides[side].mon_mut().charging = None;
                events.push(Event::Cant { side: side as u8 + 1, status: Status::Freeze });
                return;
            }
        }
        // Flinch: the hit that caused it resolved earlier this turn, so a
        // flinched mon that has not moved yet loses its action. Freeze and
        // sleep outrank it in the games' gate order, hence checking second.
        if self.sides[side].mon().flinched {
            self.sides[side].mon_mut().charging = None;
            events.push(Event::Flinched { side: side as u8 + 1 });
            return;
        }
        // Confusion: the clock ticks before the action; at zero it lifts
        // and the move proceeds. Otherwise a coin (the script's selfhit
        // knob) decides between acting and the 40 BP typeless self-hit.
        if self.sides[side].mon().confusion_n > 0 {
            self.sides[side].mon_mut().confusion_n -= 1;
            if self.sides[side].mon().confusion_n == 0 {
                events.push(Event::ConfusionEnded { side: side as u8 + 1 });
            } else {
                let (selfhit, random) = match script {
                    Some(s) => (s.selfhit, s.random),
                    None => (self.rng.below(2) == 0, 85 + self.rng.below(16) as u8),
                };
                if selfhit {
                    self.sides[side].mon_mut().charging = None;
                    let amount = self.confusion_self_hit(side, random);
                    events.push(Event::ConfusedHit { side: side as u8 + 1, amount });
                    let mon = self.sides[side].mon();
                    if mon.fainted() {
                        events.push(Event::Fainted { side: side as u8 + 1 });
                        if let Some(next) = self.sides[side].first_healthy() {
                            self.sides[side].active = next;
                            events.push(Event::Switched { side: side as u8 + 1, party_index: next });
                        }
                    }
                    return;
                }
            }
        }
        // Full paralysis: a quarter of a paralyzed mon's actions, decided by
        // the script under test and the RNG in play. No PP is spent.
        if self.sides[side].mon().status == Some(Status::Paralysis) {
            let immobile = match script {
                Some(s) => s.immobile,
                None => self.rng.below(4) == 0,
            };
            if immobile {
                self.sides[side].mon_mut().charging = None;
                events.push(Event::FullyParalyzed { side: side as u8 + 1 });
                return;
            }
        }
        // Mid two-turn move: the release is forced to the charged slot and
        // its PP was already paid on the charge turn.
        let releasing = self.sides[side].mon().charging.is_some();
        let index = match self.sides[side].mon().charging {
            Some(i) => {
                self.sides[side].mon_mut().charging = None;
                i as usize
            }
            None => index,
        };
        let Some(slot) = self.sides[side].mon().moves.get(index).copied() else {
            events.push(Event::Failed { side: side as u8 + 1 });
            return;
        };
        if slot.pp == 0 && !releasing {
            events.push(Event::Failed { side: side as u8 + 1 });
            return;
        }
        if !releasing {
            self.sides[side].mon_mut().moves[index].pp -= 1;
        }
        events.push(Event::Used { side: side as u8 + 1, move_index: index });

        // The charge turn of a two-turn move: announce, tuck the slot away,
        // and stop. Skull Bash's era perk raises Defense on the way down.
        if slot.entry.charge && !releasing {
            events.push(Event::Charging { side: side as u8 + 1 });
            if slot.entry.id == "skullbash" {
                self.sides[side].mon_mut().apply_boost(Boost::Def, 1);
                events.push(Event::Boosted { side: side as u8 + 1, boost: Boost::Def, delta: 1 });
            }
            self.sides[side].mon_mut().charging = Some(index as u8);
            return;
        }

        // Explosion/Self-Destruct: the user faints ON USE, before the hit
        // resolves — a miss or an immune target changes nothing about that.
        let boom = slot.entry.selfdestruct;
        if boom {
            self.sides[side].mon_mut().hp = 0;
        }

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
        // A semi-invulnerable target (mid Fly/Dig/Bounce/Dive) dodges
        // everything aimed at it — no accuracy roll happens — except its
        // pierce moves, which land and double their power. Self-targeted
        // actions ignore the dodge; they never aim at the foe.
        let mut pierce_mult: u16 = 1;
        let self_targeted = matches!(
            slot.entry.status_action,
            Some(StatusAction::BoostSelf(_) | StatusAction::HealHalf)
        );
        if !self_targeted {
            if let Some(via) = self.sides[foe].mon().semi_invulnerable() {
                let pierces = match via {
                    "fly" | "bounce" => {
                        matches!(slot.entry.id, "gust" | "twister" | "thunder" | "skyuppercut")
                    }
                    "dig" => matches!(slot.entry.id, "earthquake" | "magnitude"),
                    "dive" => matches!(slot.entry.id, "surf" | "whirlpool"),
                    _ => false,
                };
                if !pierces {
                    if boom {
                        self.resolve_faints(side, foe, events);
                    }
                    return;
                }
                if matches!(slot.entry.id, "gust" | "twister" | "earthquake" | "magnitude" | "surf")
                {
                    pierce_mult = 2;
                }
            }
        }

        // One-hit KO: fails outright against a higher-level target, is
        // stopped by type immunity, and otherwise its hit IS the KO.
        if slot.entry.ohko {
            let eff =
                crate::types::effectiveness_against(slot.move_type(), self.sides[foe].mon().types());
            if eff == 0 {
                events.push(Event::Damage { side: foe as u8 + 1, amount: 0, effectiveness: 0, crit: false });
                return;
            }
            if self.sides[foe].mon().level > self.sides[side].mon().level {
                events.push(Event::Failed { side: side as u8 + 1 });
                return;
            }
            if !hit {
                return;
            }
            let amount = self.sides[foe].mon().hp;
            self.sides[foe].mon_mut().hp = 0;
            events.push(Event::Damage { side: foe as u8 + 1, amount, effectiveness: 100, crit: false });
            self.resolve_faints(side, foe, events);
            return;
        }

        // Fixed damage skips the formula but not the type chart: Seismic
        // Toss still bounces off a Ghost in this era.
        if let Some(kind) = slot.entry.fixed {
            if !hit {
                return;
            }
            let eff =
                crate::types::effectiveness_against(slot.move_type(), self.sides[foe].mon().types());
            if eff == 0 {
                events.push(Event::Damage { side: foe as u8 + 1, amount: 0, effectiveness: 0, crit: false });
                return;
            }
            let amount = match kind {
                FixedDamage::Flat(n) => n,
                FixedDamage::Level => self.sides[side].mon().level as u16,
                FixedDamage::Half => (self.sides[foe].mon().hp / 2).max(1),
            };
            let target = self.sides[foe].mon_mut();
            let amount = amount.min(target.hp);
            target.hp -= amount;
            events.push(Event::Damage { side: foe as u8 + 1, amount, effectiveness: 100, crit: false });
            self.resolve_faints(side, foe, events);
            return;
        }

        // A zero-power move is its status action, nothing more.
        if slot.entry.power == 0 {
            self.status_move(side, foe, &slot, hit, script.is_some(), events);
            return;
        }
        if !hit {
            if boom {
                self.resolve_faints(side, foe, events);
            }
            return;
        }

        let (crit, random) = match script {
            Some(s) => (s.crit, s.random),
            None => (
                self.rng.below(crit_denominator(slot.entry.high_crit as u8)) == 0,
                85 + self.rng.below(16) as u8,
            ),
        };
        // How many times this move strikes. The 2-5 spread is the games'
        // weighted table (2 and 3 hits three-eighths each, 4 and 5 an eighth
        // each); a script pins the count for the tests.
        let hits = match slot.entry.multihit {
            None => 1,
            Some((lo, hi)) if lo == hi => lo,
            Some(_) => match script {
                Some(s) if s.hits > 0 => s.hits as u16,
                _ => [2u16, 2, 2, 3, 3, 3, 4, 5][self.rng.below(8) as usize],
            },
        };

        let mut total = 0u16;
        for _ in 0..hits {
            let (attacker, defender) = self.attack_pair(side);
            let m = MoveUse {
                move_type: slot.move_type(),
                power: slot.entry.power * pierce_mult,
                halve_def: slot.entry.selfdestruct,
            };
            let dealt = damage(&attacker, &defender, &m, Roll { crit, random });
            if dealt == 0 {
                break; // immune: later strikes land no better
            }

            let eff =
                crate::types::effectiveness_against(m.move_type, self.sides[foe].mon().types());
            let target = self.sides[foe].mon_mut();
            let amount = (dealt as u16).min(target.hp);
            target.hp -= amount;
            total += amount;
            events.push(Event::Damage {
                side: foe as u8 + 1,
                amount,
                effectiveness: eff,
                crit,
            });
            // Drain heals off the damage actually dealt: floor, but at least 1.
            if let Some((num, den)) = slot.entry.drain {
                let heal = (amount * num / den).max(1);
                let user = self.sides[side].mon_mut();
                let heal = heal.min(user.max_hp - user.hp);
                if heal > 0 {
                    user.hp += heal;
                    events.push(Event::Drained { side: side as u8 + 1, amount: heal });
                }
            }
            self.hit_effects(side, foe, &slot, script, events);
            if self.sides[foe].mon().fainted() {
                break;
            }
        }
        if total == 0 {
            if boom {
                self.resolve_faints(side, foe, events);
            }
            return;
        }

        // This runs BEFORE the thaw below, matching the reference sim: a
        // Fire move's burn chance is blocked by the freeze it is about to
        // cure, because the target still carries frz when secondaries apply.
        // Recoil comes off the damage actually dealt: floored (this era's
        // rule; the fuzzer rejected round-to-nearest), but at least 1 — and
        // it can knock the user out.
        if let Some((num, den)) = slot.entry.recoil {
            let hurt = (total * num / den).max(1);
            let user = self.sides[side].mon_mut();
            let hurt = hurt.min(user.hp);
            user.hp -= hurt;
            events.push(Event::Recoil { side: side as u8 + 1, amount: hurt });
        }

        // A landed Hyper Beam costs the next action.
        if slot.entry.recharge {
            self.sides[side].mon_mut().must_recharge = true;
        }

        self.resolve_faints(side, foe, events);
    }

    /// Announce and replace whoever is down — target first, then the user
    /// (recoil can faint it too). A side that already replaced this turn has
    /// a healthy active here, so nothing double-fires. A real forced-switch
    /// prompt belongs to the caller; this keeps the battle legal.
    fn resolve_faints(&mut self, side: usize, foe: usize, events: &mut Vec<Event>) {
        for who in [foe, side] {
            if self.sides[who].mon().fainted() {
                events.push(Event::Fainted { side: who as u8 + 1 });
                if let Some(next) = self.sides[who].first_healthy() {
                    self.sides[who].active = next;
                    events.push(Event::Switched { side: who as u8 + 1, party_index: next });
                }
            }
        }
    }


    /// Land `status` on `foe`'s active mon if Gen 3 rules allow it, setting
    /// the clocks that come with it: Toxic restarts its count, sleep draws
    /// its duration (pinned to the reference sim's floor under a script).
    fn inflict(&mut self, foe: usize, status: Status, scripted: bool, events: &mut Vec<Event>) {
        if !self.sides[foe].mon().can_receive(status) {
            return;
        }
        let sleep = match status {
            Status::Sleep if scripted => 2,
            Status::Sleep => 2 + self.rng.below(4) as u8,
            _ => 0,
        };
        let target = self.sides[foe].mon_mut();
        target.status = Some(status);
        target.toxic_n = 0;
        target.sleep_n = sleep;
        events.push(Event::Statused { side: foe as u8 + 1, status });
    }

    /// Confuse `foe`'s active mon if it can be: not fainted, not already
    /// confused. The clock is 2 under a script (the sim's pinned floor),
    /// 2..=5 in play.
    fn confuse(&mut self, foe: usize, scripted: bool, events: &mut Vec<Event>) {
        let target = self.sides[foe].mon();
        if target.fainted() || target.confusion_n > 0 {
            return;
        }
        let n = if scripted { 2 } else { 2 + self.rng.below(4) as u8 };
        self.sides[foe].mon_mut().confusion_n = n;
        events.push(Event::ConfusionStarted { side: foe as u8 + 1 });
    }

    /// The confusion self-hit: 40 base power, typeless, physical, against
    /// the mon's own Defense — stages and burn apply, nothing else does.
    fn confusion_self_hit(&mut self, side: usize, random: u8) -> u16 {
        let mon = self.sides[side].mon();
        let atk = crate::stats::apply_stage(mon.atk, mon.stages[Stat::Atk as usize]) as u32;
        let def = crate::stats::apply_stage(mon.def, mon.stages[Stat::Def as usize]).max(1) as u32;
        let mut dmg = ((2 * mon.level as u32 / 5 + 2) * 40 * atk / def) / 50;
        if mon.burned() {
            dmg /= 2;
        }
        if dmg == 0 {
            dmg = 1;
        }
        dmg += 2;
        dmg = (dmg * random.clamp(85, 100) as u32) / 100;
        let dmg = (dmg.max(1) as u16).min(mon.hp);
        self.sides[side].mon_mut().hp -= dmg;
        dmg
    }

    /// Resolve a zero-power move. `hit` already includes the accuracy roll;
    /// self-targeted actions cannot miss (their table accuracy is 0, the
    /// never-miss sentinel). A status move with no modelled action — Splash —
    /// does nothing, honestly.
    fn status_move(
        &mut self,
        side: usize,
        foe: usize,
        slot: &MoveSlot,
        hit: bool,
        scripted: bool,
        events: &mut Vec<Event>,
    ) {
        let Some(action) = slot.entry.status_action else {
            return;
        };
        match action {
            StatusAction::BoostSelf(list) => {
                for &(boost, delta) in list {
                    self.sides[side].mon_mut().apply_boost(boost, delta);
                    events.push(Event::Boosted { side: side as u8 + 1, boost, delta });
                }
            }
            StatusAction::HealHalf => {
                let mon = self.sides[side].mon_mut();
                let heal = (mon.max_hp / 2).min(mon.max_hp - mon.hp);
                if heal > 0 {
                    mon.hp += heal;
                    events.push(Event::Healed { side: side as u8 + 1, amount: heal });
                }
            }
            StatusAction::Confuse => {
                let immune = slot.entry.respects_immunity
                    && crate::types::effectiveness_against(
                        slot.move_type(),
                        self.sides[foe].mon().types(),
                    ) == 0;
                if hit && !immune {
                    self.confuse(foe, scripted, events);
                }
            }
            StatusAction::Inflict(status) => {
                let immune = slot.entry.respects_immunity
                    && crate::types::effectiveness_against(
                        slot.move_type(),
                        self.sides[foe].mon().types(),
                    ) == 0;
                if hit && !immune {
                    self.inflict(foe, status, scripted, events);
                }
            }
            StatusAction::BoostFoe(list) => {
                if hit && !self.sides[foe].mon().fainted() {
                    for &(boost, delta) in list {
                        self.sides[foe].mon_mut().apply_boost(boost, delta);
                        events.push(Event::Boosted { side: foe as u8 + 1, boost, delta });
                    }
                }
            }
        }
    }

    /// The per-strike aftermath of a landed hit: the secondary (script or
    /// RNG-decided), then the Fire thaw. Runs once per strike of a
    /// multi-hit move, which is also how the reference sim rolls it.
    fn hit_effects(
        &mut self,
        side: usize,
        foe: usize,
        slot: &MoveSlot,
        script: Option<SeatScript>,
        events: &mut Vec<Event>,
    ) {
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
                    self.inflict(foe, status, script.is_some(), events);
                }
                Some(SecondaryEffect::Boosts(list)) => {
                    if !self.sides[foe].mon().fainted() {
                        for &(boost, delta) in list {
                            self.sides[foe].mon_mut().apply_boost(boost, delta);
                            events.push(Event::Boosted { side: foe as u8 + 1, boost, delta });
                        }
                    }
                }
                Some(SecondaryEffect::Flinch) => {
                    if !self.sides[foe].mon().fainted() {
                        self.sides[foe].mon_mut().flinched = true;
                    }
                }
                Some(SecondaryEffect::Confuse) => {
                    self.confuse(foe, script.is_some(), events);
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

    fn scripted(script: [SeatScript; 2]) -> TurnScript {
        TurnScript { seats: [Some(script[0]), Some(script[1])] }
    }

    const PLAIN: SeatScript = SeatScript {
        hit: true,
        crit: false,
        random: 100,
        secondary: false,
        immobile: false,
        hits: 0,
        selfhit: false,
    };

    #[test]
    fn a_flinched_mon_loses_its_action_for_exactly_one_turn() {
        // Blaziken outspeeds and Headbutt's flinch procs: Snorlax never moves.
        // (Snorlax rather than Treecko so two Headbutts cannot KO it.)
        let mut b = battle(mon("blaziken", 50, &["headbutt"]), mon("snorlax", 50, &["pound"]));
        assert!(b.sides[0].mon().spe > b.sides[1].mon().spe);
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([SeatScript { secondary: true, ..PLAIN }, PLAIN]),
        );
        assert!(events.iter().any(|e| matches!(e, Event::Flinched { side: 2 })));
        assert!(!events.iter().any(|e| matches!(e, Event::Used { side: 2, .. })));
        // The flinch does not leak into the next turn.
        let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        assert!(events.iter().any(|e| matches!(e, Event::Used { side: 2, .. })));
    }

    #[test]
    fn sleep_lasts_its_clock_and_the_mon_acts_the_turn_it_wakes() {
        let mut b = battle(mon("blaziken", 50, &["sing"]), mon("snorlax", 50, &["pound"]));
        // Turn 1: Sing lands (clock 2), and slower Snorlax's own action
        // already ticks it to 1 — a Cant the very turn it fell asleep.
        let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Statused { side: 2, status: Status::Sleep }
        )));
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Cant { side: 2, status: Status::Sleep })));
        assert_eq!(b.sides[1].mon().sleep_n, 1);
        // Turn 2: 1 -> 0, it wakes and moves that same turn. The turn's
        // earlier Sing could not re-land — Snorlax still carried slp when it
        // resolved — so the wake leaves it clean.
        let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        assert!(events.iter().any(|e| matches!(e, Event::Used { side: 2, .. })));
        assert_eq!(b.sides[1].mon().status, None);
    }

    #[test]
    fn thunder_wave_respects_ground_immunity() {
        let mut b = battle(mon("pikachu", 50, &["thunderwave"]), mon("golem", 50, &["splash"]));
        let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        assert!(!events.iter().any(|e| matches!(e, Event::Statused { .. })));
        assert_eq!(b.sides[1].mon().status, None, "a Ground type shrugs off Thunder Wave");
    }

    #[test]
    fn confusion_ticks_selfhits_and_lifts() {
        // Gengar confuses Snorlax; the scripted coin says "hit yourself".
        let mut b = battle(mon("gengar", 50, &["confuseray"]), mon("snorlax", 50, &["pound"]));
        let hp = b.sides[1].mon().hp;
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, SeatScript { selfhit: true, ..PLAIN }]),
        );
        assert!(events.iter().any(|e| matches!(e, Event::ConfusionStarted { side: 2 })));
        assert!(events.iter().any(|e| matches!(e, Event::ConfusedHit { side: 2, .. })));
        assert!(!events.iter().any(|e| matches!(e, Event::Used { side: 2, .. })));
        assert!(b.sides[1].mon().hp < hp, "the self-hit landed");
        // Next turn the clock hits zero: confusion lifts and Snorlax acts,
        // and the re-Confuse Ray fails against the still-confused target
        // (it resolved before the clock ticked).
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, SeatScript { selfhit: true, ..PLAIN }]),
        );
        assert!(events.iter().any(|e| matches!(e, Event::ConfusionEnded { side: 2 })));
        assert!(events.iter().any(|e| matches!(e, Event::Used { side: 2, .. })));
    }

    #[test]
    fn full_paralysis_spends_the_turn_but_no_pp() {
        let mut b = battle(mon("blaziken", 50, &["ember"]), mon("treecko", 50, &["pound"]));
        b.sides[0].party[0].status = Some(Status::Paralysis);
        let pp = b.sides[0].mon().moves[0].pp;
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([SeatScript { immobile: true, ..PLAIN }, PLAIN]),
        );
        assert!(events.iter().any(|e| matches!(e, Event::FullyParalyzed { side: 1 })));
        assert!(!events.iter().any(|e| matches!(e, Event::Used { side: 1, .. })));
        assert_eq!(b.sides[0].mon().moves[0].pp, pp, "full paralysis spends no PP");
    }

    #[test]
    fn toxic_ticks_grow_and_reset_on_switching_out() {
        let side1 = Side::new(alloc::vec![mon("snorlax", 50, &["pound"]), mon("mudkip", 50, &["pound"])]);
        let side2 = Side::new(alloc::vec![mon("treecko", 50, &["pound"])]);
        let mut b = Battle::new(side1, side2, 3);
        b.sides[0].party[0].status = Some(Status::Toxic);
        let max = b.sides[0].mon().max_hp;
        let hp0 = b.sides[0].mon().hp;
        let miss = SeatScript { hit: false, ..PLAIN };
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([miss, miss]));
        let tick1 = hp0 - b.sides[0].mon().hp;
        assert_eq!(tick1, (max / 16).max(1), "first tick is one sixteenth");
        let hp1 = b.sides[0].mon().hp;
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([miss, miss]));
        assert_eq!(hp1 - b.sides[0].mon().hp, tick1 * 2, "second tick doubles");
        // Switching out resets the clock; the turn Snorlax comes back in,
        // its tick is a sixteenth again rather than a third multiple.
        b.step_with([Choice::Switch(1), Choice::Move(0)], &scripted([miss, miss]));
        let hp = b.sides[0].party[0].hp;
        b.step_with([Choice::Switch(0), Choice::Move(0)], &scripted([miss, miss]));
        assert_eq!(hp - b.sides[0].party[0].hp, tick1, "the counter restarted");
    }

    #[test]
    fn drain_heals_half_the_damage_and_recoil_floors_a_third() {
        let mut b = battle(mon("blaziken", 50, &["doubleedge"]), mon("snorlax", 50, &["gigadrain"]));
        b.sides[1].party[0].hp -= 40; // room to heal into
        let (hp1, hp2) = (b.sides[0].mon().hp, b.sides[1].mon().hp);
        let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        let dealt = |events: &[Event], to: u8| {
            events
                .iter()
                .find_map(|e| match e {
                    Event::Damage { side, amount, .. } if *side == to => Some(*amount),
                    _ => None,
                })
                .unwrap()
        };
        let (to_snorlax, to_blaziken) = (dealt(&events, 2), dealt(&events, 1));
        // Blaziken: hit by Giga Drain, and its own Double-Edge recoil.
        assert_eq!(b.sides[0].mon().hp, hp1 - to_blaziken - (to_snorlax / 3).max(1));
        // Snorlax: hit by Double-Edge, healed half of its own Giga Drain.
        assert_eq!(b.sides[1].mon().hp, hp2 - to_snorlax + (to_blaziken / 2).max(1));
        assert!(events.iter().any(|e| matches!(e, Event::Recoil { side: 1, .. })));
        assert!(events.iter().any(|e| matches!(e, Event::Drained { side: 2, .. })));
    }

    #[test]
    fn a_multi_hit_move_strikes_the_scripted_count() {
        // Fury Attack at 4 scripted strikes: four damage events, one PP.
        let mut b = battle(mon("blaziken", 50, &["furyattack"]), mon("snorlax", 50, &["pound"]));
        let miss = SeatScript { hit: false, ..PLAIN };
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([SeatScript { hits: 4, ..PLAIN }, miss]),
        );
        let strikes =
            events.iter().filter(|e| matches!(e, Event::Damage { side: 2, .. })).count();
        assert_eq!(strikes, 4);
        assert_eq!(b.sides[0].mon().moves[0].pp, b.sides[0].mon().moves[0].entry.pp - 1);

        // Double Kick is a fixed two: the script's count does not move it.
        let mut b = battle(mon("blaziken", 50, &["doublekick"]), mon("snorlax", 50, &["pound"]));
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([SeatScript { hits: 5, ..PLAIN }, miss]),
        );
        let strikes =
            events.iter().filter(|e| matches!(e, Event::Damage { side: 2, .. })).count();
        assert_eq!(strikes, 2);
    }

    #[test]
    fn status_moves_inflict_boost_and_heal() {
        // Thunder Wave: paralysis lands through the status-move path.
        let mut b = battle(mon("blaziken", 50, &["thunderwave"]), mon("snorlax", 50, &["pound"]));
        let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Statused { side: 2, status: Status::Paralysis }
        )));
        assert_eq!(b.sides[1].mon().status, Some(Status::Paralysis));

        // Swords Dance doubles the next physical hit: +2 Attack stages.
        let mut b = battle(mon("blaziken", 50, &["swordsdance", "doublekick"]), mon("snorlax", 50, &["splash"]));
        b.step_with([Choice::Move(0), Choice::Move(1)], &scripted([PLAIN, PLAIN]));
        assert_eq!(b.sides[0].mon().stages[Stat::Atk as usize], 2);
        let hp_before = b.sides[1].mon().hp;
        b.step_with([Choice::Move(1), Choice::Move(1)], &scripted([PLAIN, PLAIN]));
        let boosted = hp_before - b.sides[1].mon().hp;
        let mut plain = battle(mon("blaziken", 50, &["doublekick"]), mon("snorlax", 50, &["splash"]));
        let hp_before = plain.sides[1].mon().hp;
        plain.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        let unboosted = hp_before - plain.sides[1].mon().hp;
        // Not exactly double: the flat +2 and the floors sit outside the
        // stage multiply. Meaningfully bigger is the claim.
        assert!(boosted > unboosted * 3 / 2, "+2 Atk hits harder: {boosted} vs {unboosted}");

        // Recover heals half of max, capped at full.
        let mut b = battle(mon("blaziken", 50, &["recover"]), mon("snorlax", 50, &["splash"]));
        let max = b.sides[0].mon().max_hp;
        b.sides[0].party[0].hp = 1;
        let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        assert_eq!(b.sides[0].mon().hp, 1 + max / 2);
        assert!(events.iter().any(|e| matches!(e, Event::Healed { side: 1, .. })));

        // A scripted miss keeps Growl off the target's stages.
        let mut b = battle(mon("blaziken", 50, &["growl"]), mon("snorlax", 50, &["splash"]));
        let miss = SeatScript { hit: false, ..PLAIN };
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([miss, PLAIN]));
        assert_eq!(b.sides[1].mon().stages[Stat::Atk as usize], 0);
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        assert_eq!(b.sides[1].mon().stages[Stat::Atk as usize], -1);
    }

    #[test]
    fn charge_moves_take_two_turns_and_recharge_costs_one() {
        // Solar Beam: turn 1 charges (one PP, no damage), turn 2 releases.
        let mut b = battle(mon("venusaur", 50, &["solarbeam"]), mon("snorlax", 50, &["splash"]));
        let hp = b.sides[1].mon().hp;
        let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        assert!(events.iter().any(|e| matches!(e, Event::Charging { side: 1 })));
        assert!(!events.iter().any(|e| matches!(e, Event::Damage { side: 2, .. })));
        assert_eq!(b.sides[0].mon().moves[0].pp, b.sides[0].mon().moves[0].entry.pp - 1);
        let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        assert!(events.iter().any(|e| matches!(e, Event::Damage { side: 2, .. })));
        assert!(b.sides[1].mon().hp < hp);
        assert_eq!(
            b.sides[0].mon().moves[0].pp,
            b.sides[0].mon().moves[0].entry.pp - 1,
            "the release costs no second PP"
        );

        // Hyper Beam: the landed hit costs the next action. (Snorlax's own
        // bulk keeps the target alive to see the recharge.)
        let mut b = battle(mon("snorlax", 50, &["hyperbeam"]), mon("snorlax", 50, &["splash"]));
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        assert!(events.iter().any(|e| matches!(e, Event::Recharging { side: 1 })));
        assert!(!events.iter().any(|e| matches!(e, Event::Used { side: 1, .. })));
        // And the turn after, it attacks again.
        let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        assert!(events.iter().any(|e| matches!(e, Event::Used { side: 1, .. })));
    }

    #[test]
    fn semi_invulnerability_dodges_and_earthquake_pierces_dig_doubled() {
        // Mid-Dig, Tackle whiffs without even rolling accuracy.
        let mut b = battle(mon("sandslash", 50, &["dig"]), mon("snorlax", 50, &["tackle"]));
        let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        assert!(events.iter().any(|e| matches!(e, Event::Charging { side: 1 })));
        assert!(!events.iter().any(|e| matches!(e, Event::Damage { side: 1, .. })));
        assert_eq!(b.sides[0].mon().hp, b.sides[0].mon().max_hp);

        // Mid-Dig, Earthquake connects — at double power.
        let mut plain = battle(mon("snorlax", 50, &["earthquake"]), mon("sandslash", 50, &["splash"]));
        let hp = plain.sides[1].mon().hp;
        plain.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        let normal_hit = hp - plain.sides[1].mon().hp;

        let mut b = battle(mon("sandslash", 50, &["dig"]), mon("snorlax", 50, &["earthquake"]));
        // Snorlax is slower: Sandslash digs, then Earthquake lands doubled.
        assert!(b.sides[0].mon().spe > b.sides[1].mon().spe);
        let hp = b.sides[0].mon().hp;
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        let pierced = hp - b.sides[0].mon().hp;
        assert!(pierced > normal_hit * 3 / 2, "doubled: {pierced} vs {normal_hit}");
    }

    #[test]
    fn recoil_can_knock_the_user_out() {
        let side1 = Side::new(alloc::vec![mon("blaziken", 100, &["doubleedge"]), mon("mudkip", 50, &["pound"])]);
        let side2 = Side::new(alloc::vec![mon("snorlax", 100, &["pound"])]);
        let mut b = Battle::new(side1, side2, 3);
        b.sides[0].party[0].hp = 1;
        let miss = SeatScript { hit: false, ..PLAIN };
        let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, miss]));
        assert!(events.iter().any(|e| matches!(e, Event::Fainted { side: 1 })));
        assert_eq!(b.sides[0].active, 1, "the bench replaced the recoil faint");
    }

    #[test]
    fn a_fire_hit_thaws_the_target_but_its_burn_chance_is_blocked() {
        let mut b = battle(mon("blaziken", 50, &["ember"]), mon("treecko", 50, &["pound"]));
        b.sides[1].party[0].status = Some(Status::Freeze);
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([SeatScript { secondary: true, ..PLAIN }, PLAIN]),
        );
        assert_eq!(b.sides[1].mon().status, None, "the freeze thawed");
        assert!(
            !events.iter().any(|e| matches!(e, Event::Statused { side: 2, .. })),
            "the burn chance was blocked by the freeze it cured"
        );
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
