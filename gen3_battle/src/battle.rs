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
use crate::data::{move_by_id, species_by_id, Boost, FixedDamage, MoveEntry, SecondaryEffect, SideCondition, SpeciesEntry, Status, StatusAction, Weather};
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
    /// Substitute hit points; 0 means no substitute. Cleared by switching
    /// out. While up, foe damage and effects land here instead.
    pub sub_hp: u16,
    /// Foresight landed: Ghost immunity to Normal/Fighting lifted, evasion
    /// stages ignored against this mon.
    pub identified: bool,
    /// Lock-On: the next move cannot miss.
    pub sure_hit: bool,
    /// Charge: the next Electric move doubles.
    pub charged_elec: bool,
    /// Grudge is armed until this mon's next action.
    pub grudged: bool,
    /// Torment: the same move twice in a row is refused.
    pub tormented: bool,
    /// True only for the remainder of the turn Torment landed in: the
    /// victim's already-chosen move still goes through that turn.
    pub torment_fresh: bool,
    /// Rage is rolling: hits taken raise Attack.
    pub raging: bool,
    /// Fury Cutter's consecutive-hit ramp (0..=4).
    pub fury_n: u8,
    /// The last move slot this mon successfully USED (for Spite/Torment).
    pub last_used: Option<u8>,
    /// The last move this mon used MISSED (Mirror Move refuses those).
    pub last_missed: bool,
    /// Mimic's overlay: the original slot to restore on faint or switch.
    pub mimic_backup: Option<(u8, MoveSlot)>,
    /// Transform overlay: the pre-transform moveset, restored when the
    /// copy ends (faint or switch) — the corpse shows its real moves.
    pub transform_backup: Option<Vec<MoveSlot>>,
    /// Bide: damage stored and turns of storing left.
    pub bide: Option<(u16, u8)>,
    /// Rollout/Ice Ball: consecutive uses so far (0..=4).
    pub rolling: Option<u8>,
    /// Defense Curl was used: Rollout and Ice Ball double.
    pub curled: bool,
    /// Encore clock: repeats the last move while above zero.
    pub encore_n: u8,
    /// Disable: which slot is sealed, and for how long. The turn it lands
    /// the victim's already-chosen move is just a lost turn.
    pub disabled_slot: Option<u8>,
    pub disable_n: u8,
    pub disable_fresh: bool,
    /// Imprison is up: the foe cannot use moves this mon also knows.
    pub imprisoning: bool,
    /// True only for the remainder of the turn Imprison landed in: the
    /// foe's already-chosen sealed move is a lost turn, not a Struggle.
    pub imprison_fresh: bool,
    /// Camouflage/Conversion retyping.
    pub type_override: Option<(Type, Type)>,
    /// Protect's stall gamble: 0 fresh, then the era's 2-4-8 ladder.
    pub stall_counter: u8,
    /// Untouchable this turn (Protect/Detect). Cleared when the turn ends.
    pub protected: bool,
    /// Whatever lands this turn leaves 1 HP (Endure). Cleared at turn end.
    pub enduring: bool,
    /// Taunt clock: while above zero, status moves are refused.
    pub taunt_n: u8,
    /// Nightmare rides this mon's sleep, a quarter of max HP per turn.
    pub nightmared: bool,
    /// Stockpile charges banked (0..=3).
    pub stockpile_n: u8,
    /// Yawn clock: at zero-after-decrement the drowsy mon falls asleep.
    pub yawn_n: u8,
    /// Perish Song clock: fainting at zero. 0 also means "not singing".
    pub perish_n: u8,
    /// Destiny Bond is armed until this mon's next action.
    pub destiny: bool,
    /// Mean Look and kin: cannot switch while the gazer stays.
    pub mean_looked: bool,
    /// Mud/Water Sport: which attack type this mon's field-hum halves.
    pub sport: Option<Type>,
    /// Tightening focus for a Focus Punch this turn: refuses the flinch
    /// volatile outright. Set as the turn starts, cleared as it ends.
    pub focusing: bool,
    /// Focus Energy is up: crits start two stages higher. Cleared by
    /// switching out.
    pub focused: bool,
    /// Minimized: the stomping moves land doubled. Cleared by switching out.
    pub minimized: bool,
    /// Leech Seed planted on this mon. Cleared by switching out.
    pub seeded: bool,
    /// Partial-trap clock (Wrap and kin): while above zero the mon cannot
    /// switch and takes a sixteenth of max HP after each surviving tick.
    /// Released when it runs out or the trapper leaves the field.
    pub trapped_n: u8,
    /// Mid two-turn move: the slot charged last turn, releasing this turn.
    /// Any Cant loses the charge. Cleared by switching out.
    pub charging: Option<u8>,
    /// Thrash-family rampage: (slot, attacks left AFTER this turn's).
    /// Ends in fatigue confusion if it runs its course; any disruption
    /// (a Cant, a miss) ends it quietly. Cleared by switching out.
    pub rampage: Option<(u8, u8)>,
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
            identified: false,
            sure_hit: false,
            charged_elec: false,
            grudged: false,
            tormented: false,
            torment_fresh: false,
            raging: false,
            fury_n: 0,
            last_used: None,
            last_missed: false,
            mimic_backup: None,
            transform_backup: None,
            bide: None,
            rolling: None,
            curled: false,
            encore_n: 0,
            disabled_slot: None,
            disable_n: 0,
            disable_fresh: false,
            imprisoning: false,
            imprison_fresh: false,
            type_override: None,
            stall_counter: 0,
            protected: false,
            enduring: false,
            taunt_n: 0,
            nightmared: false,
            stockpile_n: 0,
            yawn_n: 0,
            perish_n: 0,
            destiny: false,
            mean_looked: false,
            sport: None,
            sub_hp: 0,
            focusing: false,
            focused: false,
            minimized: false,
            seeded: false,
            trapped_n: 0,
            charging: None,
            rampage: None,
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
    /// modify() rounding, which is round-half-DOWN: 146 quarters to 36
    /// (36.5 down), 239 to 60 (59.75 up). (2·spe+3)/8 is that arithmetic
    /// exactly; the fuzzer caught both directions.
    pub fn effective_speed(&self) -> u16 {
        let spe = crate::stats::apply_stage(self.spe, self.stages[Stat::Spe as usize]);
        if self.status == Some(Status::Paralysis) {
            ((spe as u32 * 2 + 3) / 8) as u16
        } else {
            spe
        }
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
        self.type_override.unwrap_or(self.species.types)
    }

    fn stage(&self, s: Stat) -> i8 {
        self.stages[s as usize]
    }
}

/// One player's party, plus the team-wide conditions protecting it. Each
/// counter is turns remaining; zero means down.
#[derive(Clone, Debug)]
pub struct Side {
    pub party: Vec<Mon>,
    pub active: usize,
    pub reflect_n: u8,
    pub light_screen_n: u8,
    pub safeguard_n: u8,
    pub mist_n: u8,
    /// Spikes layers on THIS side's floor (0..=3).
    pub spikes: u8,
    /// Wish clock and the amount that arrives when it hits zero.
    pub wish_n: u8,
    pub wish_amount: u16,
    /// A Future Sight/Doom Desire aimed at THIS side: the countdown and
    /// which of the two moves it was. The hit itself is recomputed in
    /// full at resolution from the launcher's then-current stats.
    pub incoming: Option<(u8, &'static str)>,
}

impl Side {
    pub fn new(party: Vec<Mon>) -> Side {
        Side {
            party,
            active: 0,
            reflect_n: 0,
            light_screen_n: 0,
            safeguard_n: 0,
            mist_n: 0,
            spikes: 0,
            wish_n: 0,
            wish_amount: 0,
            incoming: None,
        }
    }

    fn condition_n(&mut self, cond: SideCondition) -> &mut u8 {
        match cond {
            SideCondition::Reflect => &mut self.reflect_n,
            SideCondition::LightScreen => &mut self.light_screen_n,
            SideCondition::Safeguard => &mut self.safeguard_n,
            SideCondition::Mist => &mut self.mist_n,
        }
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
    /// The consecutive Protect/Endure gamble comes up heads.
    pub stall: bool,
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
    /// A five-turn team condition went up on `side`.
    SideStarted { side: u8, condition: SideCondition },
    /// Leech Seed took root on `side`'s active mon.
    Seeded { side: u8 },
    WeatherStarted { weather: Weather },
    WeatherEnded { weather: Weather },
    /// Sandstorm or hail chipped `side`'s mon.
    WeatherDamage { side: u8, amount: u16 },
    /// Haze wiped every stat stage on both actives.
    HazeCleared,
    /// `side` tucked behind Protect (or braced with Endure).
    Protected { side: u8 },
    /// `side`'s mon grew drowsy (Yawn).
    Drowsy { side: u8 },
    /// The perish count on `side`'s mon: 3, 2, 1 — and 0 is the faint.
    PerishCount { side: u8, n: u8 },
    /// `side`'s mon armed Destiny Bond.
    DestinyArmed { side: u8 },
    /// `side`'s mon can no longer escape (Mean Look and kin).
    NoEscape { side: u8 },
    /// A layer of Spikes scattered on `side`'s floor.
    SpikesLaid { side: u8 },
    /// `side`'s switch-in stepped on Spikes for `amount`.
    SpikesDamage { side: u8, amount: u16 },
    /// Leech Seed drained `amount` from `side`'s mon to the other active.
    SeedDrain { side: u8, amount: u16 },
    /// `side`'s mon was bound by Wrap and kin.
    Trapped { side: u8 },
    /// `side`'s mon is getting pumped (Focus Energy).
    Focused { side: u8 },
    /// `side` put up a substitute.
    SubStarted { side: u8 },
    /// `side`'s substitute soaked `amount`.
    SubDamage { side: u8, amount: u16 },
    /// `side`'s substitute broke.
    SubBroke { side: u8 },
    /// `side`'s mon went to sleep with Rest.
    Rested { side: u8 },
    /// The bind chipped `side`'s mon.
    TrapDamage { side: u8, amount: u16 },
    /// The bind on `side`'s mon ran out.
    TrapEnded { side: u8 },
    /// `side`'s team condition ran out.
    SideEnded { side: u8, condition: SideCondition },
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
    /// Active weather and its remaining end-of-turn ticks.
    pub weather: Option<Weather>,
    pub weather_n: u8,
    /// Last damage each side's mon took THIS turn from the foe's move,
    /// split physical/special — what Counter and Mirror Coat bounce.
    taken_physical: [u16; 2],
    taken_special: [u16; 2],
}

impl Battle {
    pub fn new(side1: Side, side2: Side, seed: u64) -> Battle {
        Battle {
            sides: [side1, side2],
            rng: Rng::new(seed),
            turn: 0,
            weather: None,
            weather_n: 0,
            taken_physical: [0; 2],
            taken_special: [0; 2],
        }
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
        self.taken_physical = [0; 2];
        self.taken_special = [0; 2];

        // A seat that chose Focus Punch starts tightening its focus before
        // anything else happens this turn (the sim's priority charge step);
        // while focusing, the flinch volatile is refused outright.
        for side in 0..2 {
            if let Choice::Move(i) = choices[side] {
                if self.sides[side].mon().moves.get(i).is_some_and(|m| m.entry.id == "focuspunch")
                    && !self.sides[side].mon().fainted()
                {
                    self.sides[side].mon_mut().focusing = true;
                }
            }
        }

        // Switches resolve before any move, in side order. Leaving the field
        // resets a Toxic count: the poison stays, the clock starts over.
        for side in 0..2 {
            if let Choice::Switch(idx) = choices[side] {
                if (self.sides[side].mon().trapped_n > 0 || self.sides[side].mon().mean_looked)
                    && !self.sides[side].mon().fainted()
                {
                    // Bound or gazed: switching is refused; the turn is forfeit.
                    continue;
                }
                if idx < self.sides[side].party.len() && !self.sides[side].party[idx].fainted() {
                    // The trapper/gazer leaving the field releases its
                    // victim; a sport leaves with its hummer (handled by
                    // the outgoing mon's own field reset below).
                    self.sides[1 - side].mon_mut().trapped_n = 0;
                    self.sides[1 - side].mon_mut().mean_looked = false;
                    let out = self.sides[side].mon_mut();
                    out.toxic_n = 0;
                    out.confusion_n = 0;
                    out.identified = false;
                    out.sure_hit = false;
                    out.charged_elec = false;
                    out.grudged = false;
                    out.tormented = false;
                    out.torment_fresh = false;
                    out.raging = false;
                    out.fury_n = 0;
                    out.last_used = None;
                    out.last_missed = false;
                    if let Some((i, orig)) = out.mimic_backup.take() {
                        out.moves[i as usize] = orig;
                    }
                    if let Some(orig) = out.transform_backup.take() {
                        out.moves = orig;
                    }
                    out.bide = None;
                    out.rolling = None;
                    out.curled = false;
                    out.encore_n = 0;
                    out.disabled_slot = None;
                    out.disable_n = 0;
                    out.disable_fresh = false;
                    out.imprisoning = false;
                    out.imprison_fresh = false;
                    out.type_override = None;
                    out.stall_counter = 0;
                    out.protected = false;
                    out.enduring = false;
                    out.taunt_n = 0;
                    out.nightmared = false;
                    out.stockpile_n = 0;
                    out.yawn_n = 0;
                    out.perish_n = 0;
                    out.destiny = false;
                    out.mean_looked = false;
                    out.sport = None;
                    out.sub_hp = 0;
                    out.focused = false;
                    out.minimized = false;
                    out.seeded = false;
                    out.trapped_n = 0;
                    out.charging = None;
                    out.rampage = None;
                    out.must_recharge = false;
                    self.sides[side].active = idx;
                    events.push(Event::Switched { side: side as u8 + 1, party_index: idx });
                    self.spikes_greet(side, &mut events);
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

        // The whole end-of-turn phase is skipped once the battle is decided
        // — but a wipe DURING the phase does not cut it short (the sim
        // finishes both weather chips even when the first one ends it).
        if !self.over() {
        // Weather chips first — the games' upkeep slot: sandstorm hits
        // anything not Rock/Ground/Steel, hail anything not Ice, a
        // sixteenth each, faster side first.
        // Wish arrives at the end of the turn after it was made — at the
        // games' order 4, BEFORE the weather chips and the status ticks
        // (the sim logs the heal ahead of the hail) — healing whoever is
        // active then.
        for side in 0..2 {
            if self.sides[side].wish_n > 0 {
                self.sides[side].wish_n -= 1;
                if self.sides[side].wish_n == 0 {
                    let amount = self.sides[side].wish_amount;
                    let mon = self.sides[side].mon_mut();
                    if !mon.fainted() {
                        let heal = amount.min(mon.max_hp - mon.hp);
                        if heal > 0 {
                            mon.hp += heal;
                            events.push(Event::Healed { side: side as u8 + 1, amount: heal });
                        }
                    }
                }
            }
        }

        if matches!(self.weather, Some(Weather::Sandstorm | Weather::Hail)) {
            let sand = self.weather == Some(Weather::Sandstorm);
            let first = self.faster_side(scripted);
            for side in [first, 1 - first] {
                let mon = self.sides[side].mon();
                if mon.fainted() {
                    continue;
                }
                let (t1, t2) = mon.types();
                let immune = if sand {
                    [t1, t2].iter().any(|t| {
                        matches!(t, Type::Rock | Type::Ground | Type::Steel)
                    })
                } else {
                    t1 == Type::Ice || t2 == Type::Ice
                };
                if immune {
                    continue;
                }
                // Underground or underwater shelters from the weather;
                // mid-Fly/Bounce does not.
                if matches!(mon.semi_invulnerable(), Some("dig" | "dive")) {
                    continue;
                }
                let amount = (self.sides[side].mon().max_hp / 16).max(1);
                let mon = self.sides[side].mon_mut();
                let amount = amount.min(mon.hp);
                mon.hp -= amount;
                events.push(Event::WeatherDamage { side: side as u8 + 1, amount });
                self.faint_and_replace(side, &mut events);
            }
        }

        // End of turn: residuals run per MON in speed order, each mon
        // resolving all of its own effects before the next mon's — Leech
        // Seed (the games' order 8) before the status tick (order 9). The
        // fuzzer caught the wrong shape: looping per effect let a poisoned
        // seeder tick first and then heal itself back off its victim.
        let first = self.faster_side(scripted);
        for side in [first, 1 - first] {
            // Unlike the field-wide weather handler, the per-mon residuals
            // stop the moment the battle is decided.
            if self.over() {
                break;
            }
            // Leech Seed bleeds an eighth of max HP to the opposing active.
            if self.sides[side].mon().seeded && !self.sides[side].mon().fainted() {
                let drain = (self.sides[side].mon().max_hp / 8).max(1);
                let mon = self.sides[side].mon_mut();
                let drain = drain.min(mon.hp);
                mon.hp -= drain;
                events.push(Event::SeedDrain { side: side as u8 + 1, amount: drain });
                let foe = self.sides[1 - side].mon_mut();
                if !foe.fainted() {
                    let heal = drain.min(foe.max_hp - foe.hp);
                    if heal > 0 {
                        foe.hp += heal;
                        events.push(Event::Healed { side: (1 - side) as u8 + 1, amount: heal });
                    }
                }
                self.faint_and_replace(side, &mut events);
            }
            // Burn and poison tick 1/8 max HP, Toxic a growing sixteenth.
            let mon = self.sides[side].mon();
            if mon.fainted() {
                continue;
            }
            if let Some(status @ (Status::Burn | Status::Poison | Status::Toxic)) = mon.status {
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
                self.faint_and_replace(side, &mut events);
            }
            // Nightmare rides the sleep: a quarter per turn while it lasts.
            if self.sides[side].mon().nightmared && !self.sides[side].mon().fainted() {
                if self.sides[side].mon().status == Some(Status::Sleep) {
                    let mon = self.sides[side].mon_mut();
                    let amount = (mon.max_hp / 4).max(1).min(mon.hp);
                    mon.hp -= amount;
                    events.push(Event::Residual {
                        side: side as u8 + 1,
                        amount,
                        status: Status::Sleep,
                    });
                    self.faint_and_replace(side, &mut events);
                } else {
                    self.sides[side].mon_mut().nightmared = false;
                }
            }
            // The bind (order 10, after the status tick): the clock ticks
            // down, and every surviving tick chips a sixteenth.
            if self.sides[side].mon().trapped_n > 0 && !self.sides[side].mon().fainted() {
                let mon = self.sides[side].mon_mut();
                mon.trapped_n -= 1;
                if mon.trapped_n > 0 {
                    // A substitute soaks the bind's chip (invisible to the
                    // battle state we track); the clock still runs down.
                    if mon.sub_hp == 0 {
                        let amount = ((mon.max_hp / 16).max(1)).min(mon.hp);
                        mon.hp -= amount;
                        events.push(Event::TrapDamage { side: side as u8 + 1, amount });
                        self.faint_and_replace(side, &mut events);
                    }
                } else {
                    events.push(Event::TrapEnded { side: side as u8 + 1 });
                }
            }
        }

        // A Future Sight lands at the end of its third turn, computed from
        // the launcher's snapshot against the target now standing.
        for side in 0..2 {
            if self.over() {
                break;
            }
            if let Some((n, id)) = self.sides[side].incoming {
                if n > 1 {
                    self.sides[side].incoming = Some((n - 1, id));
                } else {
                    self.sides[side].incoming = None;
                    let mon = self.sides[side].mon();
                    if !mon.fainted() {
                        let (move_type, power) = if id == "doomdesire" {
                            (Type::Steel, 120)
                        } else {
                            (Type::Psychic, 80)
                        };
                        // A target mid Fly/Dig/Bounce/Dive when the hit
                        // arrives dodges it like any other attack.
                        if mon.semi_invulnerable().is_some() {
                            continue;
                        }
                        let defender = crate::damage::Defender {
                            def: mon.def,
                            sp_def: mon.spd,
                            def_stage: mon.stages[Stat::Def as usize],
                            sp_def_stage: mon.stages[Stat::SpDef as usize],
                            types: mon.types(),
                            reflect: false,
                            light_screen: false,
                        };
                        // The hit is recomputed IN FULL at resolution — the
                        // launcher's CURRENT stats and stages, not a launch
                        // snapshot (Feather Dance between launch and landing
                        // shrinks it).
                        let (attacker, _) = self.attack_pair(1 - side);
                        let m = MoveUse { move_type, power, halve_def: false, weather: 0 };
                        // Accuracy is rolled at RESOLUTION too — 90 for
                        // Future Sight, 85 for Doom Desire — off the
                        // launcher's seat script for that turn. A miss
                        // simply drops the delayed hit.
                        let landed = match script.seats[1 - side] {
                            Some(sc) => sc.hit,
                            None => {
                                self.rng.below(100)
                                    < if id == "doomdesire" { 85 } else { 90 }
                            }
                        };
                        if !landed {
                            continue;
                        }
                        // The damage roll — and the crit — happen at
                        // RESOLUTION, off the launcher's seat script for
                        // that turn.
                        let (random, crit) = match script.seats[1 - side] {
                            Some(sc) => (sc.random, sc.crit),
                            None => (85 + self.rng.below(16) as u8, self.rng.below(16) == 0),
                        };
                        let dealt =
                            damage(&attacker, &defender, &m, Roll { crit, random }) as u16;
                        if dealt > 0 {
                            let mon = self.sides[side].mon_mut();
                            let hit_sub = mon.sub_hp > 0;
                            if hit_sub {
                                let amount = dealt.min(mon.sub_hp);
                                mon.sub_hp -= amount;
                                events.push(Event::SubDamage { side: side as u8 + 1, amount });
                                if self.sides[side].mon().sub_hp == 0 {
                                    events.push(Event::SubBroke { side: side as u8 + 1 });
                                }
                            } else {
                                let cap = if mon.enduring {
                                    mon.hp.saturating_sub(1)
                                } else {
                                    mon.hp
                                };
                                let amount = dealt.min(cap);
                                mon.hp -= amount;
                                events.push(Event::Damage {
                                    side: side as u8 + 1,
                                    amount,
                                    effectiveness: 100,
                                    crit: false,
                                });
                                self.faint_and_replace(side, &mut events);
                            }
                        }
                    }
                }
            }
        }

        // Yawn drops the drowsy at the end of the turn AFTER it landed.
        for side in 0..2 {
            if self.over() {
                break;
            }
            if self.sides[side].mon().yawn_n > 0 && !self.sides[side].mon().fainted() {
                self.sides[side].mon_mut().yawn_n -= 1;
                if self.sides[side].mon().yawn_n == 0 {
                    self.inflict(side, Status::Sleep, scripted, &mut events);
                }
            }
        }

        // The perish count falls; at zero the song collects.
        for side in 0..2 {
            if self.over() {
                break;
            }
            if self.sides[side].mon().perish_n > 0 && !self.sides[side].mon().fainted() {
                self.sides[side].mon_mut().perish_n -= 1;
                let n = self.sides[side].mon().perish_n;
                events.push(Event::PerishCount { side: side as u8 + 1, n });
                if n == 0 {
                    self.sides[side].mon_mut().hp = 0;
                    self.faint_and_replace(side, &mut events);
                }
            }
        }

        // Team conditions run out: five end-of-turn ticks including the
        // turn they went up.
        for side in 0..2 {
            for cond in [
                SideCondition::Reflect,
                SideCondition::LightScreen,
                SideCondition::Safeguard,
                SideCondition::Mist,
            ] {
                let n = self.sides[side].condition_n(cond);
                if *n > 0 {
                    *n -= 1;
                    if *n == 0 {
                        events.push(Event::SideEnded { side: side as u8 + 1, condition: cond });
                    }
                }
            }
        }

        // Weather runs out on the same five-tick clock.
        if let Some(weather) = self.weather {
            self.weather_n = self.weather_n.saturating_sub(1);
            if self.weather_n == 0 {
                self.weather = None;
                events.push(Event::WeatherEnded { weather });
            }
        }

        // Taunt, Encore and Disable wear off on their own short clocks.
        for side in 0..2 {
            let mon = self.sides[side].mon_mut();
            if mon.taunt_n > 0 {
                mon.taunt_n -= 1;
            }
            if mon.encore_n > 0 {
                mon.encore_n -= 1;
            }
            if mon.disable_n > 0 {
                mon.disable_n -= 1;
                mon.disable_fresh = false;
                if mon.disable_n == 0 {
                    mon.disabled_slot = None;
                }
            }
        }

        }

        // A flinch lasts exactly the turn it landed in — as do Protect's
        // shield and Endure's brace.
        for side in 0..2 {
            let mon = self.sides[side].mon_mut();
            mon.flinched = false;
            mon.protected = false;
            mon.enduring = false;
            mon.torment_fresh = false;
            mon.imprison_fresh = false;
            mon.focusing = false;
        }

        if let Some(win) = self.winner() {
            events.push(Event::Win { side: win });
        }
        events
    }

    /// Who moves first given the chosen moves: higher priority bracket, then
    /// [`Battle::faster_side`] within it. A switch resolves before moves and
    /// takes no bracket.
    /// True when this seat's chosen move can only come out as Struggle —
    /// decided at CHOICE time, so turn order uses Struggle's priority 0
    /// instead of the dead move's (the sim's request step offers only
    /// Struggle, so the queued action never carries the old priority).
    fn struggles_at_choice(&self, side: usize, i: usize) -> bool {
        let mon = self.sides[side].mon();
        let locked = mon.charging.is_some()
            || mon.rampage.is_some()
            || mon.bide.is_some()
            || mon.rolling.is_some();
        if locked {
            return false;
        }
        let Some(slot) = mon.moves.get(i) else { return false };
        let status_movish = slot.entry.power == 0
            && slot.entry.fixed.is_none()
            && !slot.entry.ohko
            && !matches!(slot.entry.id, "counter" | "mirrorcoat" | "spitup");
        let foe_mon = self.sides[1 - side].mon();
        slot.pp == 0
            || mon.disabled_slot == Some(i as u8)
            || (mon.tormented && mon.last_used == Some(i as u8))
            || (mon.taunt_n == 1 && status_movish)
            || (foe_mon.imprisoning
                && foe_mon.moves.iter().any(|m| m.entry.id == slot.entry.id))
    }

    fn first_mover(&mut self, choices: &[Choice; 2], scripted: bool) -> usize {
        let prio = |side: usize| match choices[side] {
            Choice::Move(i) => {
                if self.struggles_at_choice(side, i) {
                    0
                } else {
                    self.sides[side].mon().moves.get(i).map(|s| s.entry.priority).unwrap_or(0)
                }
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
        // In Gen 3 a rampage broken by flinch or full paralysis still ends
        // in fatigue confusion, on the spot. (A miss or a protecting target
        // ends it quietly — those sites clear `rampage` directly.)
        fn break_rampage(
            b: &mut Battle,
            side: usize,
            scripted: bool,
            events: &mut Vec<Event>,
        ) {
            if let Some((slot_i, _)) = b.sides[side].mon().rampage {
                let uproar = b.sides[side]
                    .mon()
                    .moves
                    .get(slot_i as usize)
                    .is_some_and(|m| m.entry.id == "uproar");
                let n = if scripted { 2 } else { 2 + b.rng.below(4) as u8 };
                let mon = b.sides[side].mon_mut();
                mon.rampage = None;
                if !uproar && mon.confusion_n == 0 && !mon.fainted() {
                    mon.confusion_n = n;
                    events.push(Event::ConfusionStarted { side: side as u8 + 1 });
                }
            }
        }
        // Destiny Bond and Grudge last exactly until the user's next action.
        self.sides[side].mon_mut().destiny = false;
        self.sides[side].mon_mut().grudged = false;

        // Recharging after Hyper Beam and kin: the whole action is spent,
        // gated even above sleep, matching the games' priority order.
        if self.sides[side].mon().must_recharge {
            self.sides[side].mon_mut().must_recharge = false;
            self.sides[side].mon_mut().stall_counter = 0;
            events.push(Event::Recharging { side: side as u8 + 1 });
            return;
        }
        // Fast asleep: the sleep clock ticks down before each action, and at
        // zero the mon wakes and moves that same turn.
        let mut asleep_now = false;
        if self.sides[side].mon().status == Some(Status::Sleep) {
            let snoring = self.sides[side].mon().moves.get(index).is_some_and(|m| {
                matches!(m.entry.id, "snore" | "sleeptalk")
            });
            let mon = self.sides[side].mon_mut();
            mon.sleep_n = mon.sleep_n.saturating_sub(1);
            if mon.sleep_n == 0 {
                mon.status = None;
            } else if snoring {
                // Snore attacks straight out of sleep.
                asleep_now = true;
                events.push(Event::Cant { side: side as u8 + 1, status: Status::Sleep });
            } else {
                mon.charging = None;
                mon.rampage = None;
                mon.rolling = None;
                mon.fury_n = 0;
                mon.stall_counter = 0;
                events.push(Event::Cant { side: side as u8 + 1, status: Status::Sleep });
                return;
            }
        }
        // Frozen solid: a 1-in-5 thaw each action in play (scripts pin it
        // off, matching the reference runs). Flame Wheel and Sacred Fire
        // pass THROUGH this gate — but the cure itself only lands when the
        // move actually executes, so a flinch or full paralysis after this
        // gate leaves the user frozen.
        if self.sides[side].mon().status == Some(Status::Freeze) {
            let defrost = self.sides[side].mon().moves.get(index).is_some_and(|slot| {
                matches!(slot.entry.id, "flamewheel" | "sacredfire")
            });
            let lucky = match script {
                Some(_) => false,
                None => self.rng.below(5) == 0,
            };
            if lucky {
                self.sides[side].mon_mut().status = None;
            } else if !defrost {
                self.sides[side].mon_mut().charging = None;
                self.sides[side].mon_mut().rampage = None;
                self.sides[side].mon_mut().rolling = None;
                self.sides[side].mon_mut().fury_n = 0;
                self.sides[side].mon_mut().stall_counter = 0;
                events.push(Event::Cant { side: side as u8 + 1, status: Status::Freeze });
                return;
            }
        }
        // Flinch: the hit that caused it resolved earlier this turn, so a
        // flinched mon that has not moved yet loses its action. Freeze and
        // sleep outrank it in the games' gate order, hence checking second.
        if self.sides[side].mon().flinched {
            self.sides[side].mon_mut().charging = None;
            break_rampage(self, side, script.is_some(), events);
            self.sides[side].mon_mut().rolling = None;
            self.sides[side].mon_mut().fury_n = 0;
            self.sides[side].mon_mut().stall_counter = 0;
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
                    self.sides[side].mon_mut().rampage = None;
                    self.sides[side].mon_mut().rolling = None;
                    self.sides[side].mon_mut().fury_n = 0;
                    self.sides[side].mon_mut().stall_counter = 0;
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
                break_rampage(self, side, script.is_some(), events);
                self.sides[side].mon_mut().rolling = None;
                self.sides[side].mon_mut().fury_n = 0;
                self.sides[side].mon_mut().stall_counter = 0;
                events.push(Event::FullyParalyzed { side: side as u8 + 1 });
                return;
            }
        }
        // Encore overrides the choice with the last move used.
        let index = match (self.sides[side].mon().encore_n > 0, self.sides[side].mon().last_used) {
            (true, Some(i)) => i as usize,
            _ => index,
        };
        // Mid two-turn move: the release is forced to the charged slot and
        // its PP was already paid on the charge turn. A rampage forces its
        // slot the same way. Bide and a rolling Rollout force theirs too.
        let ramping = self.sides[side].mon().rampage.is_some()
            || self.sides[side].mon().bide.is_some()
            || self.sides[side].mon().rolling.is_some();
        let releasing = self.sides[side].mon().charging.is_some() || ramping;
        let index = match self.sides[side].mon().charging {
            Some(i) => {
                self.sides[side].mon_mut().charging = None;
                i as usize
            }
            None => match self.sides[side].mon().rampage {
                Some((i, _)) => i as usize,
                None => index,
            },
        };
        let Some(slot) = self.sides[side].mon().moves.get(index).copied() else {
            events.push(Event::Failed { side: side as u8 + 1 });
            return;
        };
        // A storing Bide sits silent — the games log nothing for the two
        // storing turns — and cannot be re-chosen out of.
        if let Some((stored, left)) = self.sides[side].mon().bide {
            if left > 0 {
                self.sides[side].mon_mut().bide = Some((stored, left - 1));
                return;
            }
        }

        // Anything but another Protect/Endure resets the stall gamble.
        if !matches!(
            slot.entry.status_action,
            Some(StatusAction::Protect | StatusAction::Endure)
        ) {
            self.sides[side].mon_mut().stall_counter = 0;
        }
        // Taunt semantics split on WHEN it landed: the turn it lands, the
        // victim's already-chosen status move is simply blocked (a lost
        // turn); a status move CHOSEN while taunted becomes Struggle —
        // typeless 50 power, a quarter recoil — as does running out of PP.
        let status_movish = slot.entry.power == 0
            && slot.entry.fixed.is_none()
            && !slot.entry.ohko
            && !matches!(slot.entry.id, "counter" | "mirrorcoat" | "spitup");
        if self.sides[side].mon().taunt_n == 2 && status_movish {
            events.push(Event::Failed { side: side as u8 + 1 });
            return;
        }
        let taunted_out = self.sides[side].mon().taunt_n == 1 && status_movish;
        // Torment: the same move twice in a row becomes Struggle. So does
        // a Disabled slot, or a move the imprisoning foe also knows.
        let tormented_out = self.sides[side].mon().tormented
            && !self.sides[side].mon().torment_fresh
            && self.sides[side].mon().last_used == Some(index as u8)
            && !releasing;
        if self.sides[side].mon().disabled_slot == Some(index as u8)
            && self.sides[side].mon().disable_fresh
            && !releasing
        {
            // Disabled mid-turn: the chosen move is simply lost.
            self.sides[side].mon_mut().disable_fresh = false;
            self.sides[side].mon_mut().stall_counter = 0;
            events.push(Event::Failed { side: side as u8 + 1 });
            return;
        }
        let disabled_out =
            self.sides[side].mon().disabled_slot == Some(index as u8) && !releasing;
        let sealed = self.sides[foe].mon().imprisoning
            && self.sides[foe]
                .mon()
                .moves
                .iter()
                .any(|m| m.entry.id == slot.entry.id);
        if sealed && self.sides[foe].mon().imprison_fresh && !releasing {
            // Imprison landed earlier this same turn: the chosen move is
            // simply lost — no move line, no PP, no Struggle.
            self.sides[side].mon_mut().stall_counter = 0;
            events.push(Event::Failed { side: side as u8 + 1 });
            return;
        }
        let imprisoned_out = sealed && !releasing;
        let struggling = (taunted_out
            || tormented_out
            || disabled_out
            || imprisoned_out
            || (slot.pp == 0 && !releasing))
            && !releasing;
        let slot = if struggling {
            MoveSlot { entry: &crate::data::STRUGGLE, pp: 1, typed_as: None }
        } else {
            slot
        };
        if !releasing && !struggling {
            self.sides[side].mon_mut().moves[index].pp -= 1;
        }
        // A defrosting move thaws its user the moment it actually goes off.
        if self.sides[side].mon().status == Some(Status::Freeze)
            && matches!(slot.entry.id, "flamewheel" | "sacredfire")
        {
            self.sides[side].mon_mut().status = None;
        }
        events.push(Event::Used { side: side as u8 + 1, move_index: index });
        {
            let mon = self.sides[side].mon_mut();
            mon.last_used = if struggling { None } else { Some(index as u8) };
            mon.last_missed = false;
            // Using any move ends an ongoing rage; Rage itself re-arms it
            // only once it actually lands (a missed Rage never rages).
            mon.raging = false;
            if slot.entry.id != "furycutter" {
                mon.fury_n = 0;
            }
        }

        // Snore only works out of a snore-filled sleep.
        if slot.entry.id == "snore" && !asleep_now {
            events.push(Event::Failed { side: side as u8 + 1 });
            return;
        }

        // Nature Power becomes Swift in the sim's default arena; Hidden
        // Power under the fuzz's uniform maxed IVs is Dark 70.
        let slot = if slot.entry.id == "naturepower" {
            // Nature Power rolls its own 95 accuracy; a miss stops before
            // Swift is even called (one log line, not two).
            let np_hit = match script {
                Some(s) => s.hit,
                None => self.rng.below(100) < 95,
            };
            if !np_hit {
                return;
            }
            // The games announce both: Nature Power, then the called Swift.
            events.push(Event::Used { side: side as u8 + 1, move_index: index });
            MoveSlot { entry: move_by_id("swift").expect("swift"), pp: 1, typed_as: None }
        } else if slot.entry.id == "hiddenpower" && slot.typed_as.is_none() {
            MoveSlot { entry: slot.entry, pp: slot.pp, typed_as: Some(Type::Dark) }
        } else {
            slot
        };

        // The charge turn of a two-turn move: announce, tuck the slot away,
        // and stop. Skull Bash's era perk raises Defense on the way down.
        let instant_solar = slot.entry.id == "solarbeam" && self.weather == Some(Weather::Sun);
        if slot.entry.charge && !releasing && !instant_solar {
            events.push(Event::Charging { side: side as u8 + 1 });
            if slot.entry.id == "skullbash" {
                self.sides[side].mon_mut().apply_boost(Boost::Def, 1);
                events.push(Event::Boosted { side: side as u8 + 1, boost: Boost::Def, delta: 1 });
            }
            self.sides[side].mon_mut().charging = Some(index as u8);
            return;
        }

        // Rollout and Ice Ball lock five doubling uses; Bide stores for two
        // turns; Uproar rolls like a rampage without the hangover.
        if matches!(slot.entry.id, "rollout" | "iceball") && self.sides[side].mon().rolling.is_none()
        {
            self.sides[side].mon_mut().rolling = Some(0);
        }
        if slot.entry.id == "bide" && self.sides[side].mon().bide.is_none() {
            self.sides[side].mon_mut().bide = Some((0, 2));
            events.push(Event::Charging { side: side as u8 + 1 });
            return;
        }
        // The unleash: double everything stored, typeless, at the foe.
        if slot.entry.id == "bide" {
            let (stored, _) = self.sides[side].mon().bide.unwrap();
            self.sides[side].mon_mut().bide = None;
            let amount = stored.saturating_mul(2);
            if amount == 0 {
                events.push(Event::Failed { side: side as u8 + 1 });
                return;
            }
            let target = self.sides[foe].mon_mut();
            if target.sub_hp > 0 {
                let amount = amount.min(target.sub_hp);
                target.sub_hp -= amount;
                events.push(Event::SubDamage { side: foe as u8 + 1, amount });
                if self.sides[foe].mon().sub_hp == 0 {
                    events.push(Event::SubBroke { side: foe as u8 + 1 });
                }
                return;
            }
            let cap = if target.enduring { target.hp.saturating_sub(1) } else { target.hp };
            let amount = amount.min(cap);
            target.hp -= amount;
            self.taken_physical[foe] = amount;
            events.push(Event::Damage { side: foe as u8 + 1, amount, effectiveness: 100, crit: false });
            self.resolve_faints(side, foe, events);
            return;
        }
        // The thrash family locks in: the games roll 2..3 total attacks
        // (a script pins the floor). The lock starts on first use. Uproar
        // rides the same lock for its pinned 2 (2..5 in play) turns.
        if matches!(slot.entry.id, "thrash" | "petaldance" | "outrage" | "uproar") && !ramping {
            let total: u8 = match script {
                Some(_) => 2,
                None if slot.entry.id == "uproar" => 2 + self.rng.below(4) as u8,
                None => 2 + self.rng.below(2) as u8,
            };
            self.sides[side].mon_mut().rampage = Some((index as u8, total - 1));
        }

        // Spit Up with an empty bank simply fails; otherwise the bank is
        // spent whatever happens next.
        if slot.entry.id == "spitup" {
            if self.sides[side].mon().stockpile_n == 0 {
                events.push(Event::Failed { side: side as u8 + 1 });
                return;
            }
        }

        // Focus Punch loses its focus — and the turn — if anything hit the
        // user before it moved. The sim checks in the move's own onTry,
        // after every gate has passed and the PP is already spent.
        if slot.entry.id == "focuspunch"
            && (self.taken_physical[side] > 0 || self.taken_special[side] > 0)
        {
            events.push(Event::Failed { side: side as u8 + 1 });
            return;
        }

        // Brick Break smashes the target's screens before it hits, unless
        // the target is outright immune.
        if slot.entry.id == "brickbreak"
            && crate::types::effectiveness_against(slot.move_type(), self.sides[foe].mon().types())
                != 0
        {
            for cond in [SideCondition::Reflect, SideCondition::LightScreen] {
                let n = self.sides[foe].condition_n(cond);
                if *n > 0 {
                    *n = 0;
                    events.push(Event::SideEnded { side: foe as u8 + 1, condition: cond });
                }
            }
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
        // Thunder rides the weather: unmissable in rain, halved in sun.
        let acc = if slot.entry.id == "thunder" {
            match self.weather {
                Some(Weather::Rain) => 0,
                Some(Weather::Sun) => 50,
                _ => slot.entry.accuracy,
            }
        } else {
            slot.entry.accuracy
        };
        let sure = self.sides[side].mon().sure_hit;
        if sure {
            self.sides[side].mon_mut().sure_hit = false;
        }
        let hit = sure || match script {
            Some(s) => acc == 0 || s.hit,
            None => {
                acc == 0 || {
                    let eva = if self.sides[foe].mon().identified {
                        0
                    } else {
                        self.sides[foe].mon().eva_stage
                    };
                    let s = (self.sides[side].mon().acc_stage - eva).clamp(-6, 6) as i32;
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
            Some(
                StatusAction::BoostSelf(_)
                    | StatusAction::HealHalf
                    | StatusAction::Team(_)
                    | StatusAction::SetWeather(_)
                    | StatusAction::Protect
                    | StatusAction::Endure
                    | StatusAction::Focus
                    | StatusAction::Rest
                    | StatusAction::BellyDrum
                    | StatusAction::Stockpile
                    | StatusAction::Swallow
                    | StatusAction::WeatherHeal
                    | StatusAction::Refresh
                    | StatusAction::Wish
                    | StatusAction::DestinyBond
                    | StatusAction::Grudge
                    | StatusAction::ChargeUp
                    | StatusAction::Sport(_)
                    | StatusAction::Spikes
                    | StatusAction::Haze
                    | StatusAction::PerishSong
                    | StatusAction::Minimize
                    | StatusAction::PsychUp
            )
        );
        // Sketch carries no protect flag: it works through a shield.
        if !self_targeted && slot.entry.id != "sketch" && self.sides[foe].mon().protected {
            // A shielded target disrupts a rampage: no fatigue, fresh start.
            self.sides[side].mon_mut().rampage = None;
            events.push(Event::Failed { side: side as u8 + 1 });
            // The kicks crash into the shield all the same.
            if matches!(slot.entry.id, "highjumpkick" | "jumpkick") {
                self.kick_crash(side, foe, &slot, script, events);
                return;
            }
            if boom {
                self.resolve_faints(side, foe, events);
            }
            return;
        }
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
                    // A dodge IS a miss: same bookkeeping, and the kicks
                    // still crash for half what they would have dealt.
                    self.sides[side].mon_mut().last_missed = true;
                    self.sides[side].mon_mut().rampage = None;
                    self.sides[side].mon_mut().fury_n = 0;
                    self.sides[side].mon_mut().rolling = None;
                    if matches!(slot.entry.id, "highjumpkick" | "jumpkick") {
                        self.kick_crash(side, foe, &slot, script, events);
                        return;
                    }
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
            if self.sides[foe].mon().sub_hp > 0 {
                let amount = self.sides[foe].mon().sub_hp;
                self.sides[foe].mon_mut().sub_hp = 0;
                events.push(Event::SubDamage { side: foe as u8 + 1, amount });
                events.push(Event::SubBroke { side: foe as u8 + 1 });
                return;
            }
            let mon = self.sides[foe].mon_mut();
            let amount = if mon.enduring { mon.hp.saturating_sub(1) } else { mon.hp };
            mon.hp -= amount;
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
            if target.sub_hp > 0 {
                let amount = amount.min(target.sub_hp);
                target.sub_hp -= amount;
                events.push(Event::SubDamage { side: foe as u8 + 1, amount });
                if self.sides[foe].mon().sub_hp == 0 {
                    events.push(Event::SubBroke { side: foe as u8 + 1 });
                }
                return;
            }
            let cap = if target.enduring { target.hp.saturating_sub(1) } else { target.hp };
            let amount = amount.min(cap);
            target.hp -= amount;
            match crate::types::category_of(slot.move_type()) {
                crate::types::Category::Physical => self.taken_physical[foe] = amount,
                _ => self.taken_special[foe] = amount,
            }
            events.push(Event::Damage { side: foe as u8 + 1, amount, effectiveness: 100, crit: false });
            // Fixed damage still stokes a raging target and banks in a Bide.
            if let Some((stored, left)) = self.sides[foe].mon().bide {
                self.sides[foe].mon_mut().bide = Some((stored.saturating_add(amount), left));
            }
            if self.sides[foe].mon().raging && !self.sides[foe].mon().fainted() {
                self.sides[foe].mon_mut().apply_boost(Boost::Atk, 1);
                events.push(Event::Boosted { side: foe as u8 + 1, boost: Boost::Atk, delta: 1 });
            }
            self.resolve_faints(side, foe, events);
            return;
        }

        // Endeavor drags the target's HP down to the user's — through the
        // chart, never a substitute, and failing upward.
        if slot.entry.id == "endeavor" {
            if !hit {
                return;
            }
            if crate::types::effectiveness_against(slot.move_type(), self.sides[foe].mon().types())
                == 0
            {
                events.push(Event::Damage { side: foe as u8 + 1, amount: 0, effectiveness: 0, crit: false });
                return;
            }
            let (uhp, thp) = (self.sides[side].mon().hp, self.sides[foe].mon().hp);
            if self.sides[foe].mon().sub_hp > 0 || uhp >= thp {
                events.push(Event::Failed { side: side as u8 + 1 });
                return;
            }
            let amount = thp - uhp;
            self.sides[foe].mon_mut().hp = uhp;
            self.taken_physical[foe] = amount;
            events.push(Event::Damage { side: foe as u8 + 1, amount, effectiveness: 100, crit: false });
            self.resolve_faints(side, foe, events);
            return;
        }

        // Psywave's spread collapses the same way: level/2 or level*3/2.
        if slot.entry.id == "psywave" {
            if !hit {
                return;
            }
            if crate::types::effectiveness_against(slot.move_type(), self.sides[foe].mon().types())
                == 0
            {
                events.push(Event::Damage { side: foe as u8 + 1, amount: 0, effectiveness: 0, crit: false });
                return;
            }
            let level = self.sides[side].mon().level as u32;
            let i = match script {
                Some(s) => {
                    if s.secondary {
                        0
                    } else {
                        10
                    }
                }
                None => self.rng.below(11),
            };
            let amount = ((level * (10 * i as u32 + 50)) / 100).max(1) as u16;
            let target = self.sides[foe].mon_mut();
            if target.sub_hp > 0 {
                let amount = amount.min(target.sub_hp);
                target.sub_hp -= amount;
                events.push(Event::SubDamage { side: foe as u8 + 1, amount });
                if self.sides[foe].mon().sub_hp == 0 {
                    events.push(Event::SubBroke { side: foe as u8 + 1 });
                }
                return;
            }
            let cap = if target.enduring { target.hp.saturating_sub(1) } else { target.hp };
            let amount = amount.min(cap);
            target.hp -= amount;
            self.taken_special[foe] = amount;
            events.push(Event::Damage { side: foe as u8 + 1, amount, effectiveness: 100, crit: false });
            self.resolve_faints(side, foe, events);
            return;
        }

        // Counter and Mirror Coat bounce back double the last hit this mon
        // took this turn — physical for Counter, special for Mirror Coat —
        // through the type chart (a Ghost shrugs Counter off) and into a
        // substitute if one stands. Nothing taken means they fail.
        if matches!(slot.entry.id, "counter" | "mirrorcoat") {
            if !hit {
                return;
            }
            let taken = if slot.entry.id == "counter" {
                self.taken_physical[side]
            } else {
                self.taken_special[side]
            };
            if taken == 0 {
                events.push(Event::Failed { side: side as u8 + 1 });
                return;
            }
            let eff =
                crate::types::effectiveness_against(slot.move_type(), self.sides[foe].mon().types());
            if eff == 0 {
                events.push(Event::Damage { side: foe as u8 + 1, amount: 0, effectiveness: 0, crit: false });
                return;
            }
            let amount = taken.saturating_mul(2);
            let target = self.sides[foe].mon_mut();
            if target.sub_hp > 0 {
                let amount = amount.min(target.sub_hp);
                target.sub_hp -= amount;
                events.push(Event::SubDamage { side: foe as u8 + 1, amount });
                if self.sides[foe].mon().sub_hp == 0 {
                    events.push(Event::SubBroke { side: foe as u8 + 1 });
                }
                return;
            }
            let cap = if target.enduring { target.hp.saturating_sub(1) } else { target.hp };
            let amount = amount.min(cap);
            target.hp -= amount;
            match crate::types::category_of(slot.move_type()) {
                crate::types::Category::Physical => self.taken_physical[foe] = amount,
                _ => self.taken_special[foe] = amount,
            }
            events.push(Event::Damage { side: foe as u8 + 1, amount, effectiveness: 100, crit: false });
            self.resolve_faints(side, foe, events);
            return;
        }

        // Future Sight and Doom Desire: aim a delayed hit two turns out.
        if matches!(slot.entry.id, "futuresight" | "doomdesire") {
            if self.sides[foe].incoming.is_some() {
                events.push(Event::Failed { side: side as u8 + 1 });
                return;
            }
            self.sides[foe].incoming = Some((3, slot.entry.id));
            events.push(Event::Charging { side: side as u8 + 1 });
            return;
        }

        // Mirror Move plays back the foe's last move (both get announced);
        // Mimic and Sketch write it into the slot instead.
        let slot = if matches!(slot.entry.id, "mirrormove" | "mimic" | "sketch") {
            let foe_last = self.sides[foe].mon().last_used.and_then(|i| {
                self.sides[foe].mon().moves.get(i as usize).map(|m| m.entry)
            });
            match (slot.entry.id, foe_last) {
                (_, None) => {
                    events.push(Event::Failed { side: side as u8 + 1 });
                    return;
                }
                (_, Some(e)) if e.id == slot.entry.id => {
                    events.push(Event::Failed { side: side as u8 + 1 });
                    return;
                }
                ("mirrormove", Some(_)) if self.sides[foe].mon().last_missed => {
                    events.push(Event::Failed { side: side as u8 + 1 });
                    return;
                }
                ("mimic", Some(e)) => {
                    // A five-PP overlay; the original slot returns when the
                    // mon leaves the field or faints.
                    let orig = self.sides[side].mon().moves[index];
                    let mon = self.sides[side].mon_mut();
                    mon.mimic_backup = Some((index as u8, orig));
                    mon.moves[index] = MoveSlot { entry: e, pp: 5, typed_as: None };
                    return;
                }
                ("sketch", Some(e)) => {
                    self.sides[side].mon_mut().moves[index] =
                        MoveSlot { entry: e, pp: e.pp, typed_as: None };
                    return;
                }
                ("mirrormove", Some(e)) => {
                    events.push(Event::Used { side: side as u8 + 1, move_index: index });
                    MoveSlot { entry: e, pp: 1, typed_as: None }
                }
                _ => unreachable!(),
            }
        } else {
            slot
        };

        // A zero-power move is its status action, nothing more.
        if slot.entry.power == 0 {
            self.status_move(
                side,
                foe,
                &slot,
                hit,
                script.is_some(),
                script.map(|s| s.stall),
                events,
            );
            return;
        }
        // Type immunity preempts the accuracy step entirely: the sim logs
        // |-immune| without ever rolling to hit, so a scripted miss never
        // happens against an immune target — and the kicks never crash.
        {
            let move_type = if slot.entry.id == "weatherball" {
                match self.weather {
                    Some(Weather::Sun) => Type::Fire,
                    Some(Weather::Rain) => Type::Water,
                    Some(Weather::Sandstorm) => Type::Rock,
                    Some(Weather::Hail) => Type::Ice,
                    None => Type::Normal,
                }
            } else {
                slot.move_type()
            };
            let mut dtypes = self.sides[foe].mon().types();
            if self.sides[foe].mon().identified
                && matches!(move_type, Type::Normal | Type::Fighting)
            {
                let strip = |t: Type| if t == Type::Ghost { Type::None } else { t };
                dtypes = (strip(dtypes.0), strip(dtypes.1));
            }
            if crate::types::effectiveness_against(move_type, dtypes) == 0 {
                events.push(Event::Damage {
                    side: foe as u8 + 1,
                    amount: 0,
                    effectiveness: 0,
                    crit: false,
                });
                // An immune target never locks a rampage in — and breaks
                // a running one the way a miss does.
                if ramping {
                    break_rampage(self, side, script.is_some(), events);
                } else {
                    self.sides[side].mon_mut().rampage = None;
                }
                self.sides[side].mon_mut().rolling = None;
                self.sides[side].mon_mut().fury_n = 0;
                if boom {
                    self.resolve_faints(side, foe, events);
                }
                return;
            }
        }
        if !hit {
            // A first-use miss ends a rampage quietly, but a miss once the
            // lock is running (the [from] lockedmove turns) still ends in
            // fatigue confusion. Fury Cutter's ramp and a Rollout reset.
            self.sides[side].mon_mut().last_missed = true;
            if ramping {
                break_rampage(self, side, script.is_some(), events);
            } else {
                self.sides[side].mon_mut().rampage = None;
            }
            self.sides[side].mon_mut().fury_n = 0;
            self.sides[side].mon_mut().rolling = None;
            // The kicks crash for half the damage they would have dealt.
            if matches!(slot.entry.id, "highjumpkick" | "jumpkick") {
                self.kick_crash(side, foe, &slot, script, events);
                return;
            }
            if boom {
                self.resolve_faints(side, foe, events);
            }
            return;
        }

        let (crit, random) = match script {
            Some(s) => (s.crit, s.random),
            None => (
                self.rng.below(crit_denominator(
                    slot.entry.high_crit as u8 + if self.sides[side].mon().focused { 2 } else { 0 },
                )) == 0,
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
                // An unset (zero) hits knob means the table minimum, 2 —
                // the same reading the reference harness uses.
                Some(s) => if s.hits > 0 { s.hits as u16 } else { 2 },
                None => [2u16, 2, 2, 3, 3, 3, 4, 5][self.rng.below(8) as usize],
            },
        };

        let mut total = 0u16;
        for _ in 0..hits {
            let (attacker, mut defender) = self.attack_pair(side);
            // Weather Ball wears the sky: retyped and doubled under weather.
            let move_type = if slot.entry.id == "weatherball" {
                match self.weather {
                    Some(Weather::Sun) => Type::Fire,
                    Some(Weather::Rain) => Type::Water,
                    Some(Weather::Sandstorm) => Type::Rock,
                    Some(Weather::Hail) => Type::Ice,
                    None => Type::Normal,
                }
            } else {
                slot.move_type()
            };
            // Foresight lifts the Ghost immunity to Normal and Fighting.
            if self.sides[foe].mon().identified
                && matches!(move_type, Type::Normal | Type::Fighting)
            {
                let strip = |t: Type| if t == Type::Ghost { Type::None } else { t };
                defender.types = (strip(defender.types.0), strip(defender.types.1));
            }
            let mut weather_mod = match (self.weather, move_type) {
                (Some(Weather::Rain), Type::Water) | (Some(Weather::Sun), Type::Fire) => 1,
                (Some(Weather::Rain), Type::Fire) | (Some(Weather::Sun), Type::Water) => -1,
                _ => 0,
            };
            // Mud/Water Sport hum from EITHER active halves the matching type.
            if (0..2).any(|w| self.sides[w].mon().sport == Some(move_type)) {
                weather_mod = -1;
            }
            // The stomping moves land doubled on a minimized target.
            let stomp_mult: u16 = if self.sides[foe].mon().minimized
                && matches!(slot.entry.id, "stomp" | "extrasensory" | "needlearm" | "astonish")
            {
                2
            } else {
                1
            };
            // Solar Beam sputters outside the sun it was made for.
            let solar_cut = slot.entry.id == "solarbeam"
                && matches!(
                    self.weather,
                    Some(Weather::Rain | Weather::Sandstorm | Weather::Hail)
                );
            // Conditional powers the era defines by id.
            let base_power = match slot.entry.id {
                "return" => 102,   // the sim's default full happiness
                "frustration" => 1,
                "eruption" | "waterspout" => {
                    let mon = self.sides[side].mon();
                    ((150 * mon.hp as u32) / mon.max_hp as u32).max(1) as u16
                }
                "facade"
                    if matches!(
                        self.sides[side].mon().status,
                        Some(Status::Burn | Status::Poison | Status::Toxic | Status::Paralysis)
                    ) =>
                {
                    slot.entry.power * 2
                }
                "smellingsalts"
                    if self.sides[foe].mon().status == Some(Status::Paralysis) =>
                {
                    slot.entry.power * 2
                }
                "revenge"
                    if self.taken_physical[side] > 0 || self.taken_special[side] > 0 =>
                {
                    slot.entry.power * 2
                }
                "weatherball" if self.weather.is_some() => 100,
                "hiddenpower" => 70,
                "magnitude" => {
                    // The spread collapses under a script: the secondary knob
                    // picks Magnitude 4 (10) or 10 (150); play rolls the table.
                    let i = match script {
                        Some(s) => {
                            if s.secondary {
                                0
                            } else {
                                99
                            }
                        }
                        None => self.rng.below(100) as u8,
                    };
                    match i {
                        0..=4 => 10,
                        5..=14 => 30,
                        15..=34 => 50,
                        35..=64 => 70,
                        65..=84 => 90,
                        85..=94 => 110,
                        _ => 150,
                    }
                }
                "rollout" | "iceball" => {
                    let n = self.sides[side].mon().rolling.unwrap_or(0)
                        + self.sides[side].mon().curled as u8;
                    30u16 << n.min(5)
                }
                "furycutter" => {
                    let n = self.sides[side].mon().fury_n;
                    (10u16 << n).min(160)
                }
                "spitup" => 100 * self.sides[side].mon().stockpile_n as u16,
                "flail" | "reversal" => {
                    let mon = self.sides[side].mon();
                    let p = 48 * mon.hp as u32 / mon.max_hp as u32;
                    match p {
                        0..=1 => 200,
                        2..=4 => 150,
                        5..=9 => 100,
                        10..=16 => 80,
                        17..=32 => 40,
                        _ => 20,
                    }
                }
                _ => slot.entry.power,
            };
            // Charge doubles the next Electric move, then is spent.
            let charge_mult: u16 = if move_type == Type::Electric && self.sides[side].mon().charged_elec
            {
                self.sides[side].mon_mut().charged_elec = false;
                2
            } else {
                1
            };
            let m = MoveUse {
                move_type,
                power: base_power * pierce_mult * stomp_mult * charge_mult
                    / if solar_cut { 2 } else { 1 },
                halve_def: slot.entry.selfdestruct,
                weather: weather_mod,
            };
            let dealt = damage(&attacker, &defender, &m, Roll { crit, random });
            if dealt == 0 {
                break; // immune: later strikes land no better
            }

            let eff =
                crate::types::effectiveness_against(m.move_type, self.sides[foe].mon().types());
            let target = self.sides[foe].mon_mut();
            let hit_sub = target.sub_hp > 0;
            let amount = if hit_sub {
                let amount = (dealt as u16).min(target.sub_hp);
                target.sub_hp -= amount;
                amount
            } else {
                let cap = if slot.entry.id == "falseswipe" || target.enduring {
                    target.hp.saturating_sub(1)
                } else {
                    target.hp
                };
                let amount = (dealt as u16).min(cap);
                target.hp -= amount;
                amount
            };
            total += amount;
            if hit_sub {
                events.push(Event::SubDamage { side: foe as u8 + 1, amount });
                if self.sides[foe].mon().sub_hp == 0 {
                    events.push(Event::SubBroke { side: foe as u8 + 1 });
                }
            } else {
                match m.category() {
                    crate::types::Category::Physical => self.taken_physical[foe] = amount,
                    _ => self.taken_special[foe] = amount,
                }
                events.push(Event::Damage {
                    side: foe as u8 + 1,
                    amount,
                    effectiveness: eff,
                    crit,
                });
                // A biding target banks what it just took.
                if let Some((stored, left)) = self.sides[foe].mon().bide {
                    self.sides[foe].mon_mut().bide = Some((stored.saturating_add(amount), left));
                }
                // A raging target's Attack climbs with every hit it takes.
                if self.sides[foe].mon().raging && !self.sides[foe].mon().fainted() {
                    self.sides[foe].mon_mut().apply_boost(Boost::Atk, 1);
                    events.push(Event::Boosted { side: foe as u8 + 1, boost: Boost::Atk, delta: 1 });
                }
                // Rage's own rage state begins only once it actually lands.
                if slot.entry.id == "rage" {
                    self.sides[side].mon_mut().raging = true;
                }
            }
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
            if !hit_sub {
                self.hit_effects(side, foe, &slot, script, events);
            }
            if self.sides[foe].mon().fainted() {
                break;
            }
        }
        if slot.entry.id == "spitup" {
            self.sides[side].mon_mut().stockpile_n = 0;
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
        // A rampage counts down after each attack; running its course ends
        // in fatigue confusion (which asks nobody's permission — not
        // Safeguard's, not a substitute's).
        if let Some((slot_i, left)) = self.sides[side].mon().rampage {
            if left == 0 {
                let scripted = script.is_some();
                let fatigue = slot.entry.id != "uproar";
                let mon = self.sides[side].mon_mut();
                mon.rampage = None;
                if fatigue && mon.confusion_n == 0 && !mon.fainted() {
                    mon.confusion_n = if scripted { 2 } else { 2 + self.rng.below(4) as u8 };
                    events.push(Event::ConfusionStarted { side: side as u8 + 1 });
                }
            } else {
                self.sides[side].mon_mut().rampage = Some((slot_i, left - 1));
            }
        }

        // Fury Cutter ramps on every landed use, resetting elsewhere.
        if slot.entry.id == "furycutter" {
            let mon = self.sides[side].mon_mut();
            mon.fury_n = (mon.fury_n + 1).min(4);
        }
        // Rollout counts its landed uses and lets go after five.
        if matches!(slot.entry.id, "rollout" | "iceball") {
            let mon = self.sides[side].mon_mut();
            let n = mon.rolling.unwrap_or(0) + 1;
            mon.rolling = if n >= 5 { None } else { Some(n) };
        }
        // Defense Curl primes Rollout.
        if slot.entry.id == "defensecurl" {
            self.sides[side].mon_mut().curled = true;
        }

        // Rapid Spin flings off the user's own bind and Leech Seed.
        if slot.entry.id == "rapidspin" {
            let user = self.sides[side].mon_mut();
            user.seeded = false;
            user.trapped_n = 0;
        }

        // Superpower and kin always pay their stat bill on a landed hit.
        if let Some(list) = slot.entry.self_drop {
            if !self.sides[side].mon().fainted() {
                for &(boost, delta) in list {
                    self.sides[side].mon_mut().apply_boost(boost, delta);
                    events.push(Event::Boosted { side: side as u8 + 1, boost, delta });
                }
            }
        }

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

    /// Announce and replace one side's active if it just fainted.
    fn faint_and_replace(&mut self, side: usize, events: &mut Vec<Event>) {
        if self.sides[side].mon().fainted() {
            if let Some((i, orig)) = self.sides[side].mon_mut().mimic_backup.take() {
                self.sides[side].mon_mut().moves[i as usize] = orig;
            }
            if let Some(orig) = self.sides[side].mon_mut().transform_backup.take() {
                self.sides[side].mon_mut().moves = orig;
            }
            // A fainted trapper/gazer releases its victim.
            self.sides[1 - side].mon_mut().trapped_n = 0;
            self.sides[1 - side].mon_mut().mean_looked = false;
            events.push(Event::Fainted { side: side as u8 + 1 });
            if let Some(next) = self.sides[side].first_healthy() {
                self.sides[side].active = next;
                events.push(Event::Switched { side: side as u8 + 1, party_index: next });
                self.spikes_greet(side, events);
            }
        }
    }

    /// Spikes bite a grounded switch-in: an eighth, a sixth, a quarter for
    /// one, two, three layers. Flying types float over.
    fn spikes_greet(&mut self, side: usize, events: &mut Vec<Event>) {
        let layers = self.sides[side].spikes;
        if layers == 0 {
            return;
        }
        let mon = self.sides[side].mon();
        let (t1, t2) = mon.types();
        if t1 == Type::Flying || t2 == Type::Flying {
            return;
        }
        let max = mon.max_hp;
        let amount = match layers {
            1 => max / 8,
            2 => max / 6,
            _ => max / 4,
        }
        .max(1);
        let mon = self.sides[side].mon_mut();
        let amount = amount.min(mon.hp);
        mon.hp -= amount;
        events.push(Event::SpikesDamage { side: side as u8 + 1, amount });
        self.faint_and_replace(side, events);
    }

    /// Announce and replace whoever is down — target first, then the user
    /// (recoil can faint it too). A side that already replaced this turn has
    /// a healthy active here, so nothing double-fires. A real forced-switch
    /// prompt belongs to the caller; this keeps the battle legal.
    fn resolve_faints(&mut self, side: usize, foe: usize, events: &mut Vec<Event>) {
        // Destiny Bond: KO the bonded target with a move, go down with it.
        if self.sides[foe].mon().fainted() && self.sides[foe].mon().destiny {
            self.sides[side].mon_mut().hp = 0;
        }
        // Grudge: the killing move's PP drains to nothing.
        if self.sides[foe].mon().fainted() && self.sides[foe].mon().grudged {
            if let Some(slot_i) = self.sides[side].mon().last_used {
                if let Some(ms) = self.sides[side].mon_mut().moves.get_mut(slot_i as usize) {
                    ms.pp = 0;
                }
            }
        }
        for who in [foe, side] {
            if self.sides[who].mon().fainted() {
                if let Some((i, orig)) = self.sides[who].mon_mut().mimic_backup.take() {
                    self.sides[who].mon_mut().moves[i as usize] = orig;
                }
                if let Some(orig) = self.sides[who].mon_mut().transform_backup.take() {
                    self.sides[who].mon_mut().moves = orig;
                }
                self.sides[1 - who].mon_mut().trapped_n = 0;
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
        // Safeguard shields the whole team from foe-inflicted statuses.
        if self.sides[foe].safeguard_n > 0 {
            return;
        }
        // No one sleeps through an Uproar.
        if status == Status::Sleep
            && (0..2).any(|w| {
                self.sides[w].mon().rampage.is_some_and(|(i, _)| {
                    self.sides[w].mon().moves.get(i as usize).is_some_and(|m| m.entry.id == "uproar")
                })
            })
        {
            return;
        }
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
        // Safeguard blocks foe-inflicted confusion too in this era.
        if self.sides[foe].safeguard_n > 0 {
            return;
        }
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
        script_stall: Option<bool>,
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
            StatusAction::Seed => {
                if self.sides[foe].mon().sub_hp > 0 {
                    return;
                }
                let target = self.sides[foe].mon();
                let grass = target.types().0 == Type::Grass || target.types().1 == Type::Grass;
                if hit && !grass && !target.fainted() && !target.seeded {
                    self.sides[foe].mon_mut().seeded = true;
                    events.push(Event::Seeded { side: foe as u8 + 1 });
                }
            }
            StatusAction::BoostConfuse(list) => {
                if self.sides[foe].mon().sub_hp > 0 {
                    return;
                }
                // Swagger and Flatter: the gift lands (Mist still blocks
                // drops, not raises), then the confusion (Safeguard's job).
                if hit && !self.sides[foe].mon().fainted() {
                    for &(boost, delta) in list {
                        self.sides[foe].mon_mut().apply_boost(boost, delta);
                        events.push(Event::Boosted { side: foe as u8 + 1, boost, delta });
                    }
                    self.confuse(foe, scripted, events);
                }
            }
            StatusAction::Focus => {
                if !self.sides[side].mon().focused {
                    self.sides[side].mon_mut().focused = true;
                    events.push(Event::Focused { side: side as u8 + 1 });
                }
            }
            StatusAction::Rest => {
                let mon = self.sides[side].mon();
                if mon.hp < mon.max_hp {
                    let mon = self.sides[side].mon_mut();
                    mon.hp = mon.max_hp;
                    mon.status = Some(Status::Sleep);
                    // The games' Rest sleeps two full turns: clock of 3,
                    // the same shape as the sim's pinned setStatus.
                    mon.sleep_n = 3;
                    mon.toxic_n = 0;
                    events.push(Event::Rested { side: side as u8 + 1 });
                }
            }
            StatusAction::Minimize => {
                self.sides[side].mon_mut().minimized = true;
                self.sides[side].mon_mut().apply_boost(Boost::Eva, 1);
                events.push(Event::Boosted { side: side as u8 + 1, boost: Boost::Eva, delta: 1 });
            }
            StatusAction::WeatherHeal => {
                let mult = match self.weather {
                    None => (1, 2),
                    Some(Weather::Sun) => (2, 3),
                    Some(_) => (1, 4),
                };
                let mon = self.sides[side].mon_mut();
                let heal = ((mon.max_hp as u32 * mult.0 / mult.1) as u16)
                    .min(mon.max_hp - mon.hp);
                if heal > 0 {
                    mon.hp += heal;
                    events.push(Event::Healed { side: side as u8 + 1, amount: heal });
                }
            }
            StatusAction::Refresh => {
                let mon = self.sides[side].mon_mut();
                if matches!(
                    mon.status,
                    Some(Status::Burn | Status::Paralysis | Status::Poison | Status::Toxic)
                ) {
                    mon.status = None;
                    mon.toxic_n = 0;
                }
            }
            StatusAction::BellyDrum => {
                let mon = self.sides[side].mon();
                let cost = mon.max_hp / 2;
                if mon.hp > cost && mon.stages[Stat::Atk as usize] < 6 {
                    let mon = self.sides[side].mon_mut();
                    mon.hp -= cost;
                    mon.stages[Stat::Atk as usize] = 6;
                    events.push(Event::Boosted { side: side as u8 + 1, boost: Boost::Atk, delta: 6 });
                } else {
                    events.push(Event::Failed { side: side as u8 + 1 });
                }
            }
            StatusAction::PsychUp => {
                let stages = self.sides[foe].mon().stages;
                let acc = self.sides[foe].mon().acc_stage;
                let eva = self.sides[foe].mon().eva_stage;
                let mon = self.sides[side].mon_mut();
                mon.stages = stages;
                mon.acc_stage = acc;
                mon.eva_stage = eva;
            }
            StatusAction::Yawn => {
                let target = self.sides[foe].mon();
                if hit
                    && target.sub_hp == 0
                    && target.status.is_none()
                    && target.yawn_n == 0
                    && self.sides[foe].safeguard_n == 0
                    && !target.fainted()
                {
                    self.sides[foe].mon_mut().yawn_n = 2;
                    events.push(Event::Drowsy { side: foe as u8 + 1 });
                }
            }
            StatusAction::Wish => {
                if self.sides[side].wish_n == 0 {
                    self.sides[side].wish_n = 2;
                    self.sides[side].wish_amount = self.sides[side].mon().max_hp / 2;
                }
            }
            StatusAction::PerishSong => {
                for who in 0..2 {
                    let mon = self.sides[who].mon_mut();
                    if mon.perish_n == 0 && !mon.fainted() {
                        mon.perish_n = 4;
                    }
                }
            }
            StatusAction::DestinyBond => {
                self.sides[side].mon_mut().destiny = true;
                events.push(Event::DestinyArmed { side: side as u8 + 1 });
            }
            StatusAction::MeanLook => {
                let target = self.sides[foe].mon();
                if hit && target.sub_hp == 0 && !target.mean_looked && !target.fainted() {
                    self.sides[foe].mon_mut().mean_looked = true;
                    events.push(Event::NoEscape { side: foe as u8 + 1 });
                }
            }
            StatusAction::Sport(kind) => {
                self.sides[side].mon_mut().sport = Some(kind);
            }
            StatusAction::Spikes => {
                if self.sides[foe].spikes < 3 {
                    self.sides[foe].spikes += 1;
                    events.push(Event::SpikesLaid { side: foe as u8 + 1 });
                } else {
                    events.push(Event::Failed { side: side as u8 + 1 });
                }
            }
            StatusAction::Memento => {
                if !hit || self.sides[foe].mon().sub_hp > 0 || self.sides[foe].mon().fainted() {
                    events.push(Event::Failed { side: side as u8 + 1 });
                    return;
                }
                let misted = self.sides[foe].mist_n > 0;
                if !misted {
                    for (boost, delta) in [(Boost::Atk, -2i8), (Boost::SpAtk, -2)] {
                        self.sides[foe].mon_mut().apply_boost(boost, delta);
                        events.push(Event::Boosted { side: foe as u8 + 1, boost, delta });
                    }
                }
                self.sides[side].mon_mut().hp = 0;
                self.faint_and_replace(side, events);
            }
            StatusAction::PainSplit => {
                if !hit || self.sides[foe].mon().sub_hp > 0 || self.sides[foe].mon().fainted() {
                    events.push(Event::Failed { side: side as u8 + 1 });
                    return;
                }
                let avg = (self.sides[side].mon().hp as u32 + self.sides[foe].mon().hp as u32) / 2;
                for who in [side, foe] {
                    let mon = self.sides[who].mon_mut();
                    mon.hp = (avg as u16).min(mon.max_hp);
                }
                events.push(Event::Healed { side: side as u8 + 1, amount: 0 });
            }
            StatusAction::Protect | StatusAction::Endure => {
                let counter = self.sides[side].mon().stall_counter;
                let ok = counter == 0
                    || match script_stall {
                        Some(stall) => stall,
                        None => self.rng.below(counter as u32) == 0,
                    };
                if ok {
                    let mon = self.sides[side].mon_mut();
                    if matches!(action, StatusAction::Protect) {
                        mon.protected = true;
                    } else {
                        mon.enduring = true;
                    }
                    mon.stall_counter = if counter == 0 { 2 } else { (counter * 2).min(8) };
                    events.push(Event::Protected { side: side as u8 + 1 });
                } else {
                    self.sides[side].mon_mut().stall_counter = 0;
                    events.push(Event::Failed { side: side as u8 + 1 });
                }
            }
            StatusAction::Identify => {
                if hit && self.sides[foe].mon().sub_hp == 0 && !self.sides[foe].mon().fainted() {
                    self.sides[foe].mon_mut().identified = true;
                }
            }
            StatusAction::LockOn => {
                if hit && !self.sides[foe].mon().fainted() {
                    self.sides[side].mon_mut().sure_hit = true;
                }
            }
            StatusAction::ChargeUp => {
                self.sides[side].mon_mut().charged_elec = true;
            }
            StatusAction::Spite => {
                // Spite reaches through a substitute in this era.
                let drained = if hit {
                    if let Some(slot_i) = self.sides[foe].mon().last_used {
                        let mon = self.sides[foe].mon_mut();
                        if let Some(ms) = mon.moves.get_mut(slot_i as usize) {
                            if ms.pp > 0 {
                                // The games shave 2..5 PP; a script pins 2.
                                let cut = if scripted { 2 } else { 2 + self.rng.below(4) as u8 };
                                ms.pp = ms.pp.saturating_sub(cut);
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };
                if !drained {
                    events.push(Event::Failed { side: side as u8 + 1 });
                }
            }
            StatusAction::Grudge => {
                self.sides[side].mon_mut().grudged = true;
            }
            StatusAction::Torment => {
                if hit && !self.sides[foe].mon().tormented {
                    self.sides[foe].mon_mut().tormented = true;
                    // The landing turn is free: an already-chosen repeat
                    // still runs; only choices made under Torment struggle.
                    self.sides[foe].mon_mut().torment_fresh = true;
                } else {
                    events.push(Event::Failed { side: side as u8 + 1 });
                }
            }
            StatusAction::Encore => {
                let target = self.sides[foe].mon();
                if hit
                    && target.encore_n == 0
                    && target.last_used.is_some()
                    && !target.fainted()
                {
                    // The games run 3..6 encored turns; a script pins 3.
                    let n = if scripted { 3 } else { 3 + self.rng.below(4) as u8 };
                    self.sides[foe].mon_mut().encore_n = n;
                } else {
                    events.push(Event::Failed { side: side as u8 + 1 });
                }
            }
            StatusAction::Disable => {
                // Disable pierces a substitute in this era, but fails when
                // the target's last move has no PP left to seal.
                let target = self.sides[foe].mon();
                let last_has_pp = target
                    .last_used
                    .and_then(|i| target.moves.get(i as usize))
                    .is_some_and(|m| m.pp > 0);
                if hit
                    && target.disabled_slot.is_none()
                    && last_has_pp
                    && !target.fainted()
                {
                    // Four sealed turns pinned; 4..7 in play.
                    let slot_i = target.last_used;
                    let n = if scripted { 4 } else { 4 + self.rng.below(4) as u8 };
                    let mon = self.sides[foe].mon_mut();
                    mon.disabled_slot = slot_i;
                    mon.disable_n = n;
                    mon.disable_fresh = true;
                } else {
                    events.push(Event::Failed { side: side as u8 + 1 });
                }
            }
            StatusAction::Camouflage => {
                self.sides[side].mon_mut().type_override = Some((Type::Normal, Type::None));
            }
            StatusAction::Conversion => {
                // Only a type the user does not already have is eligible;
                // with no eligible move type, Conversion fails.
                let cur = self.sides[side].mon().types();
                let ty = self
                    .sides[side]
                    .mon()
                    .moves
                    .iter()
                    .map(|m| m.move_type())
                    .find(|&t| t != Type::None && t != cur.0 && t != cur.1);
                match ty {
                    Some(t) => {
                        self.sides[side].mon_mut().type_override = Some((t, Type::None));
                    }
                    None => events.push(Event::Failed { side: side as u8 + 1 }),
                }
            }
            StatusAction::Imprison => {
                // Fails unless the foe actually shares a move with the user
                // — the sim refuses a sealless Imprison outright.
                let shares = self.sides[side].mon().moves.iter().any(|m| {
                    self.sides[foe].mon().moves.iter().any(|f| f.entry.id == m.entry.id)
                });
                if shares && !self.sides[side].mon().imprisoning {
                    self.sides[side].mon_mut().imprisoning = true;
                    self.sides[side].mon_mut().imprison_fresh = true;
                } else {
                    events.push(Event::Failed { side: side as u8 + 1 });
                }
            }
            StatusAction::MirrorMove | StatusAction::Mimic | StatusAction::Sketch => {
                // Handled before the status path; unreachable here.
            }
            StatusAction::Transform => {
                let foe_mon = self.sides[foe].mon().clone();
                if foe_mon.fainted() {
                    events.push(Event::Failed { side: side as u8 + 1 });
                } else {
                    let mon = self.sides[side].mon_mut();
                    if mon.transform_backup.is_none() {
                        mon.transform_backup = Some(mon.moves.clone());
                    }
                    mon.atk = foe_mon.atk;
                    mon.def = foe_mon.def;
                    mon.spa = foe_mon.spa;
                    mon.spd = foe_mon.spd;
                    mon.spe = foe_mon.spe;
                    mon.stages = foe_mon.stages;
                    mon.acc_stage = foe_mon.acc_stage;
                    mon.eva_stage = foe_mon.eva_stage;
                    mon.type_override = Some(foe_mon.types());
                    mon.moves = foe_mon
                        .moves
                        .iter()
                        .map(|ms| MoveSlot {
                            entry: ms.entry,
                            // Five PP per copied move, but never more than
                            // the move's own maximum (a copied Sketch has 1).
                            pp: 5.min(ms.entry.pp),
                            typed_as: ms.typed_as,
                        })
                        .collect();
                }
            }
            StatusAction::NoopFail => {
                events.push(Event::Failed { side: side as u8 + 1 });
            }
            StatusAction::NaturePower => {
                // Handled before the status path; unreachable here.
            }
            StatusAction::Taunt => {
                // Taunt is one of the era's sub-piercers.
                let target = self.sides[foe].mon();
                if hit && target.taunt_n == 0 && !target.fainted() {
                    self.sides[foe].mon_mut().taunt_n = 2;
                }
            }
            StatusAction::Nightmare => {
                let target = self.sides[foe].mon();
                if hit
                    && target.sub_hp == 0
                    && target.status == Some(Status::Sleep)
                    && !target.nightmared
                {
                    self.sides[foe].mon_mut().nightmared = true;
                } else {
                    events.push(Event::Failed { side: side as u8 + 1 });
                }
            }
            StatusAction::Stockpile => {
                let mon = self.sides[side].mon_mut();
                if mon.stockpile_n < 3 {
                    mon.stockpile_n += 1;
                } else {
                    events.push(Event::Failed { side: side as u8 + 1 });
                }
            }
            StatusAction::Swallow => {
                let n = self.sides[side].mon().stockpile_n;
                if n == 0 {
                    events.push(Event::Failed { side: side as u8 + 1 });
                } else {
                    let mon = self.sides[side].mon_mut();
                    mon.stockpile_n = 0;
                    let heal = match n {
                        1 => mon.max_hp / 4,
                        2 => mon.max_hp / 2,
                        _ => mon.max_hp,
                    }
                    .min(mon.max_hp - mon.hp);
                    if heal > 0 {
                        mon.hp += heal;
                        events.push(Event::Healed { side: side as u8 + 1, amount: heal });
                    }
                }
            }
            StatusAction::Haze => {
                for who in 0..2 {
                    let mon = self.sides[who].mon_mut();
                    mon.stages = [0; 5];
                    mon.acc_stage = 0;
                    mon.eva_stage = 0;
                }
                events.push(Event::HazeCleared);
            }
            StatusAction::Substitute => {
                let mon = self.sides[side].mon();
                let cost = mon.max_hp / 4;
                if mon.sub_hp == 0 && mon.hp > cost {
                    let mon = self.sides[side].mon_mut();
                    mon.hp -= cost;
                    mon.sub_hp = cost;
                    events.push(Event::SubStarted { side: side as u8 + 1 });
                } else {
                    events.push(Event::Failed { side: side as u8 + 1 });
                }
            }
            StatusAction::SetWeather(weather) => {
                if self.weather != Some(weather) {
                    self.weather = Some(weather);
                    self.weather_n = 5;
                    events.push(Event::WeatherStarted { weather });
                }
            }
            StatusAction::Team(cond) => {
                let n = self.sides[side].condition_n(cond);
                if *n == 0 {
                    *n = 5;
                    events.push(Event::SideStarted { side: side as u8 + 1, condition: cond });
                }
            }
            StatusAction::Confuse => {
                if self.sides[foe].mon().sub_hp > 0 {
                    return;
                }
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
                if self.sides[foe].mon().sub_hp > 0 {
                    return;
                }
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
                if self.sides[foe].mon().sub_hp > 0 {
                    return;
                }
                let misted = self.sides[foe].mist_n > 0;
                if hit && !self.sides[foe].mon().fainted() {
                    for &(boost, delta) in list {
                        if misted && delta < 0 {
                            continue;
                        }
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
                    // Mist blocks foe-caused stat drops for the whole team.
                    let misted = self.sides[foe].mist_n > 0;
                    if !self.sides[foe].mon().fainted() {
                        for &(boost, delta) in list {
                            if misted && delta < 0 {
                                continue;
                            }
                            self.sides[foe].mon_mut().apply_boost(boost, delta);
                            events.push(Event::Boosted { side: foe as u8 + 1, boost, delta });
                        }
                    }
                }
                Some(SecondaryEffect::Flinch) => {
                    // A mon tightening its focus cannot be flinched at all —
                    // the volatile is refused, not merely out-prioritized.
                    if !self.sides[foe].mon().fainted() && !self.sides[foe].mon().focusing {
                        self.sides[foe].mon_mut().flinched = true;
                    }
                }
                Some(SecondaryEffect::Confuse) => {
                    self.confuse(foe, script.is_some(), events);
                }
                Some(SecondaryEffect::TriAttack) => {
                    let status = match script {
                        Some(_) => Status::Burn, // the sim's pinned sample
                        None => *[Status::Burn, Status::Paralysis, Status::Freeze]
                            .iter()
                            .nth(self.rng.below(3) as usize)
                            .unwrap(),
                    };
                    self.inflict(foe, status, script.is_some(), events);
                }
                Some(SecondaryEffect::SelfBoosts(list)) => {
                    if !self.sides[side].mon().fainted() {
                        for &(boost, delta) in list {
                            self.sides[side].mon_mut().apply_boost(boost, delta);
                            events.push(Event::Boosted { side: side as u8 + 1, boost, delta });
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

        // Smelling Salts spends the paralysis it fed on — unless the hit
        // was also the KO; a corpse keeps its status.
        if slot.entry.id == "smellingsalts"
            && self.sides[foe].mon().status == Some(Status::Paralysis)
            && !self.sides[foe].mon().fainted()
        {
            self.sides[foe].mon_mut().status = None;
        }

        // Wrap and kin bind the target: the games roll 3..6 end-of-turn
        // ticks (a script pins the floor), and a fresh bind cannot stack
        // on a running one.
        if slot.entry.trap
            && !self.sides[foe].mon().fainted()
            && self.sides[foe].mon().trapped_n == 0
        {
            let n = match script {
                Some(_) => 3,
                None => 3 + self.rng.below(4) as u8,
            };
            self.sides[foe].mon_mut().trapped_n = n;
            events.push(Event::Trapped { side: foe as u8 + 1 });
        }
    }



    /// The kicks crash for half the damage they would have dealt — full
    /// calc, including the crit the seat was due.
    fn kick_crash(
        &mut self,
        side: usize,
        foe: usize,
        slot: &MoveSlot,
        script: Option<SeatScript>,
        events: &mut Vec<Event>,
    ) {
        let (random, crit) = match script {
            Some(s) => (s.random, s.crit),
            None => (85 + self.rng.below(16) as u8, self.rng.below(16) == 0),
        };
        let (attacker, defender) = self.attack_pair(side);
        let m = MoveUse {
            move_type: slot.move_type(),
            power: slot.entry.power,
            halve_def: false,
            weather: 0,
        };
        let would = damage(&attacker, &defender, &m, Roll { crit, random });
        // The sim clamps the crash into [1, target's max HP / 2].
        let cap = (self.sides[foe].mon().max_hp / 2).max(1);
        let crash = ((would / 2) as u16).max(1).min(cap);
        let user = self.sides[side].mon_mut();
        let crash = crash.min(user.hp);
        user.hp -= crash;
        events.push(Event::Recoil { side: side as u8 + 1, amount: crash });
        self.resolve_faints(side, foe, events);
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
                reflect: self.sides[1 - side].reflect_n > 0,
                light_screen: self.sides[1 - side].light_screen_n > 0,
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
    fn a_move_with_no_pp_struggles_instead() {
        let mut b = battle(mon("blaziken", 50, &["ember"]), mon("treecko", 50, &["pound"]));
        b.sides[0].party[0].moves[0].pp = 0;
        let hp = b.sides[1].mon().hp;
        let before = b.sides[0].mon().hp;
        let events = b.step([Choice::Move(0), Choice::Move(0)]);
        assert!(events.iter().any(|e| matches!(e, Event::Used { side: 1, .. })));
        assert!(b.sides[1].mon().hp < hp, "Struggle landed");
        assert!(b.sides[0].mon().hp < before, "and recoiled");
        assert_eq!(b.sides[0].mon().moves[0].pp, 0, "no PP moved");
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
        stall: false,
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
    fn screens_halve_safeguard_shields_and_mist_holds_stages() {
        // Reflect roughly halves a physical hit.
        let mut plain = battle(mon("snorlax", 50, &["tackle"]), mon("chansey", 50, &["splash"]));
        let hp = plain.sides[1].mon().hp;
        plain.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        let open_hit = hp - plain.sides[1].mon().hp;

        let mut b = battle(mon("snorlax", 50, &["tackle"]), mon("chansey", 50, &["reflect"]));
        assert!(b.sides[1].mon().spe > b.sides[0].mon().spe, "chansey screens first");
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        let hp = b.sides[1].mon().hp;
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        let screened = hp - b.sides[1].mon().hp;
        assert!(screened < open_hit * 2 / 3, "reflected: {screened} vs {open_hit}");

        // Safeguard blocks Thunder Wave for the whole team. (Snorlax is
        // slower than Chansey, so the shield is up before the wave.)
        let mut b = battle(mon("snorlax", 50, &["thunderwave"]), mon("chansey", 50, &["safeguard"]));
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        assert_eq!(b.sides[1].mon().status, None);

        // Mist holds Growl off.
        let mut b = battle(mon("snorlax", 50, &["growl"]), mon("chansey", 50, &["mist"]));
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        assert_eq!(b.sides[1].mon().stages[Stat::Atk as usize], 0);
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
