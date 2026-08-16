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

use crate::ability;
use crate::item;
use crate::damage::{crit_denominator, damage, Attacker, Defender, MoveUse, Roll};
use crate::data::{
    move_by_id, species_by_id, Boost, FixedDamage, MoveEntry, SecondaryEffect, SideCondition,
    SpeciesEntry, Status, StatusAction, Weather,
};
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
        Some(MoveSlot {
            entry,
            pp: entry.pp,
            typed_as: None,
        })
    }

    /// A slot whose type overrides the move's own.
    pub fn typed(entry: &'static MoveEntry, typed_as: Option<Type>) -> MoveSlot {
        MoveSlot {
            entry,
            pp: entry.pp,
            typed_as,
        }
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
    /// Turns of sleep skipped by Snore or Sleep Talk since the last action
    /// that was simply lost to sleep. Gen 3 hands them back on switch-in,
    /// so attacking out of sleep and then retreating costs nothing.
    pub sleep_skipped: u8,
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
    /// Lock-On/Mind Reader: 2 while armed (cast turn + the next); the
    /// next move consumes it, the end-of-turn clock expires it.
    /// Aim taken BY THIS MON (Lock-On, Mind Reader): while it is above zero
    /// this mon cannot miss the mon it sighted, and cannot be dodged by it.
    /// The sim keeps the volatile on the user, so leaving the field ends the
    /// lock, and a replacement inherits nothing.
    pub sure_hit: u8,
    /// Which party slot on the other side the aim was taken at. The sim
    /// remembers the mon itself, so a lock survives that mon switching out
    /// and back but does not transfer to whoever stands in for it.
    pub sure_hit_on: u8,
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
    /// The move id this mon was last HIT by (Mirror Move's source).
    pub last_hit_by: Option<&'static str>,
    /// WHICH party slot on the other side landed that hit. The sim keeps the
    /// attacker itself in its attacked-by book and reads `source.lastMove`
    /// off it — and a mon that switches out has its `lastMove` cleared, so a
    /// Mirror Move aimed back at a since-swapped attacker finds nothing.
    pub last_hit_by_slot: Option<usize>,
    /// The id of the move this mon last used — survives Transform/Mimic
    /// rewriting the slots, which is exactly when the slot index lies.
    pub last_used_id: Option<&'static str>,
    /// The last move slot this mon successfully USED (for Spite/Torment).
    pub last_used: Option<u8>,
    /// The last move this mon used MISSED (Mirror Move refuses those).
    pub last_missed: bool,
    /// Mimic's overlay: the original slot to restore on faint or switch.
    pub mimic_backup: Option<(u8, MoveSlot)>,
    /// Transform overlay: the pre-transform moveset, restored when the
    /// copy ends (faint or switch) — the corpse shows its real moves.
    pub transform_backup: Option<Vec<MoveSlot>>,
    /// What Transform borrowed besides the move set: the five battle stats
    /// and the type line. The sim keeps these in `baseStoredStats` and
    /// restores them with the volatile, so a transformed mon that leaves the
    /// field is its own species again.
    pub transform_stats: Option<([u16; 5], Option<(Type, Type)>)>,
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
    /// The seal landed after its victim had already moved, so this turn's
    /// end does not spend one of the two actions it blocks.
    pub disable_skip_tick: bool,
    /// Imprison is up: the foe cannot use moves this mon also knows.
    pub imprisoning: bool,
    /// True only for the remainder of the turn Imprison landed in: the
    /// foe's already-chosen sealed move is a lost turn, not a Struggle.
    pub imprison_fresh: bool,
    /// Camouflage/Conversion retyping.
    pub type_override: Option<(Type, Type)>,
    /// Ghost-Curse victim: a quarter of max HP every end of turn.
    pub cursed: bool,
    /// Ingrain roots: a sixteenth of max HP back every end of turn.
    pub ingrained: bool,
    /// Whether this mon has taken any move action yet (Fake Out's window).
    pub acted: bool,
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
    /// The charge went up THIS turn, so it survives the end-of-turn sweep.
    /// The sim's `twoturnmove` volatile has duration 2: a release that never
    /// happens — a faint cancelled the action, say — lets the charge lapse
    /// rather than holding the mon forever.
    pub charge_fresh: bool,
    /// The rolling lock swung THIS turn. The sim refreshes Rollout's and Ice
    /// Ball's one-turn volatile from inside the base-power callback, which
    /// only runs when the move actually goes off — so a turn the user never
    /// got (a faint cancelled it, say) lets the lock lapse.
    pub rolling_fresh: bool,
    /// Mid two-turn move: the slot charged last turn, releasing this turn.
    /// Any Cant loses the charge. Cleared by switching out.
    pub charging: Option<u8>,
    /// The uproar swung its last turn just now: the din still blocks sleep
    /// until this turn's residuals finish (the sim removes the volatile at
    /// residual order 28, after every action).
    pub uproar_ending: bool,
    /// What a multi-turn lock (rampage, Rollout, Bide, a charge move) is
    /// actually swinging: normally the slot's own move, but a lock taken
    /// through Mirror Move carries the CALLED move, and the follow-up turns
    /// must run it directly (one announced line, no re-call).
    pub locked_move: Option<&'static str>,
    /// Thrash-family rampage: (slot, turns of lock still owed). For Uproar
    /// this is a plain countdown spent at the end of each din; for the
    /// Thrash family it is the sim's `trueDuration`, and the lock itself
    /// runs on `rampage_dur` below. Cleared by switching out.
    pub rampage: Option<(u8, u8)>,
    /// The Thrash-family lock's own clock, which the sim keeps separate
    /// from the count of swings owed: it is re-armed to 2 every time the
    /// move actually goes off, and expiring is what brings on the fatigue.
    pub rampage_dur: u8,
    /// Hyper Beam landed last turn: this action is spent recharging.
    pub must_recharge: bool,
    /// This mon's ability, as a lookup id, or empty for none. Trace and
    /// Role Play overwrite it, so it is not simply read off the species.
    pub ability: &'static str,
    /// Flash Fire has caught: this mon's Fire moves are half again as strong
    /// until it leaves the field.
    pub flash_fire: bool,
    /// Turns this mon has spent on the field since it came in. Speed Boost
    /// wants at least one before it starts climbing.
    pub active_turns: u8,
    /// Truant's toggle: true on the turn the mon loafs about.
    pub loafing: bool,
    /// The item this mon is holding, as a lookup id, or empty for none.
    /// Eating a berry empties it; Trick and Knock Off move it about.
    pub item: &'static str,
    /// A Choice Band holder is stuck with whatever it swung first. Cleared
    /// by leaving the field.
    pub choice_locked: Option<&'static str>,
    /// The last item this mon used up, which is what Recycle brings back.
    /// Having one TAKEN does not count; only spending it does.
    pub last_item: &'static str,
}

impl Mon {
    /// The slice of this mon the ability rules read. Handing the rules a
    /// small copy rather than the mon itself keeps them pure, and keeps the
    /// borrow checker out of the middle of a damage calculation.
    /// The slice of this mon the item rules read.
    pub fn holder(&self) -> crate::item::Holder {
        crate::item::Holder {
            item: self.item,
            species: self.species.id,
            transformed: self.transform_backup.is_some(),
            hp: self.hp,
            max_hp: self.max_hp,
        }
    }

    pub fn bearer(&self) -> ability::Bearer {
        ability::Bearer {
            ability: self.ability,
            types: self.types(),
            status: self.status,
            hp: self.hp,
            max_hp: self.max_hp,
        }
    }

    /// Build a mon at `level` with uniform investment. Per-stat IVs and EVs
    /// come later with the team builder; this is enough to fight.
    pub fn new(
        species_id: &str,
        level: u8,
        nature: Nature,
        inv: Invest,
        moves: &[&str],
    ) -> Option<Mon> {
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
            sleep_skipped: 0,
            flinched: false,
            confusion_n: 0,
            identified: false,
            sure_hit: 0,
            sure_hit_on: 0,
            charged_elec: false,
            grudged: false,
            tormented: false,
            torment_fresh: false,
            raging: false,
            fury_n: 0,
            last_used: None,
            last_used_id: None,
            last_hit_by: None,
            last_hit_by_slot: None,
            last_missed: false,
            mimic_backup: None,
            transform_backup: None,
            transform_stats: None,
            bide: None,
            rolling: None,
            curled: false,
            encore_n: 0,
            disabled_slot: None,
            disable_n: 0,
            disable_fresh: false,
            disable_skip_tick: false,
            imprisoning: false,
            imprison_fresh: false,
            type_override: None,
            cursed: false,
            ingrained: false,
            acted: false,
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
            charge_fresh: false,
            rolling_fresh: false,
            charging: None,
            uproar_ending: false,
            locked_move: None,
            rampage: None,
            rampage_dur: 0,
            must_recharge: false,
            ability: "",
            flash_fire: false,
            active_turns: 0,
            loafing: false,
            item: "",
            choice_locked: None,
            last_item: "",
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
        self.charging?;
        // What is actually being charged, which is not always what the slot
        // holds: a Mirror-Moved Bounce charges out of the Mirror Move's own
        // slot, and reading the slot would have the mon standing on the
        // ground while the sim has it in the air.
        let id = match self.locked_move {
            Some(id) => id,
            None => self.moves.get(self.charging? as usize)?.entry.id,
        };
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
    /// The sim's own `side.pokemon` ordering, which is NOT the party order:
    /// every switch swaps the incoming mon into slot 0 and pushes the
    /// outgoing one into the slot it came from. Anything that picks a mon at
    /// random — Roar and Whirlwind dragging one in — samples this list, so
    /// the order is observable and has to be kept.
    pub order: Vec<usize>,
    pub reflect_n: u8,
    pub light_screen_n: u8,
    pub safeguard_n: u8,
    pub mist_n: u8,
    /// Spikes layers on THIS side's floor (0..=3).
    pub spikes: u8,
    /// Wish clock and the amount that arrives when it hits zero.
    pub wish_n: u8,
    /// Unused in this era: the wish heals half the RECIPIENT's maximum, so
    /// there is nothing to bank at casting time. Kept so the field's shape
    /// does not change under callers.
    pub wish_amount: u16,
    /// A Future Sight/Doom Desire aimed at THIS side: the countdown, the
    /// damage locked in at launch, and which of the two moves it was.
    pub incoming: Option<(u8, u16, &'static str)>,
}

impl Side {
    pub fn new(party: Vec<Mon>) -> Side {
        let order = (0..party.len()).collect();
        Side {
            party,
            active: 0,
            order,
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

    /// Record a switch the way the sim does: the incoming mon trades places
    /// with whoever was at the front.
    fn reorder_for_switch(&mut self, incoming: usize) {
        if let Some(p) = self.order.iter().position(|&i| i == incoming) {
            self.order.swap(0, p);
        }
    }

    /// Everyone the sim would consider dragging in: its `possibleSwitches`
    /// walks `side.pokemon` past the active slot and keeps the unfainted, so
    /// the answer comes back in ITS order, not the party's.
    pub fn draggable(&self) -> Vec<usize> {
        self.order
            .iter()
            .skip(1)
            .copied()
            .filter(|&i| !self.party[i].fainted())
            .collect()
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
    Switched {
        side: u8,
        party_index: usize,
    },
    Used {
        side: u8,
        move_index: usize,
    },
    /// The move could not be used: no PP, or nothing to use.
    Failed {
        side: u8,
    },
    Damage {
        side: u8,
        amount: u16,
        effectiveness: u32,
        crit: bool,
    },
    /// A status landed on `side`'s active mon.
    Statused {
        side: u8,
        status: Status,
    },
    /// A secondary moved one of `side`'s active mon's stat stages.
    Boosted {
        side: u8,
        boost: Boost,
        delta: i8,
    },
    /// `side` spent the turn charging a two-turn move.
    Charging {
        side: u8,
    },
    /// A five-turn team condition went up on `side`.
    SideStarted {
        side: u8,
        condition: SideCondition,
    },
    /// Leech Seed took root on `side`'s active mon.
    Seeded {
        side: u8,
    },
    WeatherStarted {
        weather: Weather,
    },
    WeatherEnded {
        weather: Weather,
    },
    /// Sandstorm or hail chipped `side`'s mon.
    WeatherDamage {
        side: u8,
        amount: u16,
    },
    /// Haze wiped every stat stage on both actives.
    HazeCleared,
    /// `side` tucked behind Protect (or braced with Endure).
    Protected {
        side: u8,
    },
    /// `side`'s mon grew drowsy (Yawn).
    Drowsy {
        side: u8,
    },
    /// The perish count on `side`'s mon: 3, 2, 1 — and 0 is the faint.
    PerishCount {
        side: u8,
        n: u8,
    },
    /// `side`'s mon armed Destiny Bond.
    DestinyArmed {
        side: u8,
    },
    /// `side`'s mon can no longer escape (Mean Look and kin).
    NoEscape {
        side: u8,
    },
    /// A layer of Spikes scattered on `side`'s floor.
    SpikesLaid {
        side: u8,
    },
    /// `side`'s switch-in stepped on Spikes for `amount`.
    SpikesDamage {
        side: u8,
        amount: u16,
    },
    /// Leech Seed drained `amount` from `side`'s mon to the other active.
    SeedDrain {
        side: u8,
        amount: u16,
    },
    /// `side`'s mon was bound by Wrap and kin.
    Trapped {
        side: u8,
    },
    /// `side`'s mon is getting pumped (Focus Energy).
    Focused {
        side: u8,
    },
    /// `side` put up a substitute.
    SubStarted {
        side: u8,
    },
    /// `side`'s substitute soaked `amount`.
    SubDamage {
        side: u8,
        amount: u16,
    },
    /// `side`'s substitute broke.
    SubBroke {
        side: u8,
    },
    /// `side`'s mon went to sleep with Rest.
    Rested {
        side: u8,
    },
    /// The bind chipped `side`'s mon.
    TrapDamage {
        side: u8,
        amount: u16,
    },
    /// The bind on `side`'s mon ran out.
    TrapEnded {
        side: u8,
    },
    /// `side`'s team condition ran out.
    SideEnded {
        side: u8,
        condition: SideCondition,
    },
    /// `side` spent the turn recharging after Hyper Beam and kin.
    Recharging {
        side: u8,
    },
    /// `side`'s mon flinched and lost its action.
    Flinched {
        side: u8,
    },
    /// `side`'s mon became confused.
    ConfusionStarted {
        side: u8,
    },
    /// `side`'s mon hurt itself in confusion.
    ConfusedHit {
        side: u8,
        amount: u16,
    },
    /// `side`'s mon snapped out of confusion.
    ConfusionEnded {
        side: u8,
    },
    /// `side` healed `amount` by draining the damage it just dealt.
    Drained {
        side: u8,
        amount: u16,
    },
    /// `side` took `amount` recoil from its own move.
    Recoil {
        side: u8,
        amount: u16,
    },
    /// `side` healed itself for `amount` (Recover and kin).
    Healed {
        side: u8,
        amount: u16,
    },
    /// `side`'s paralyzed mon was fully paralyzed and lost its action.
    FullyParalyzed {
        side: u8,
    },
    /// `side`'s mon could not move: frozen solid or fast asleep.
    Cant {
        side: u8,
        status: Status,
    },
    /// End-of-turn burn or poison damage.
    Residual {
        side: u8,
        amount: u16,
        status: Status,
    },
    Fainted {
        side: u8,
    },
    /// 1 or 2, or 0 for a draw.
    Win {
        side: u8,
    },
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
    /// Whether each seat's CHOSEN slot was already out of PP at choice
    /// time — that (and only that) is what Struggle substitution reads;
    /// a mid-turn Spite draining it to zero is a silent lost turn.
    pp0_at_choice: [bool; 2],
    /// Which sides have already taken their action this turn. Disable needs
    /// it: its clock counts the target's next N ACTIONS, so landing on a mon
    /// that has already moved does not spend one of them.
    acted_this_turn: [bool; 2],
    /// A side whose active was dragged off this turn by Roar or Whirlwind.
    /// Its queued action goes with the mon that left, exactly as a faint's
    /// does, and the replacement does not inherit it.
    dragged: [bool; 2],
    /// Whether an action is still queued behind the one resolving now. The
    /// sim's Protect and Endure both start with `!!this.queue.willAct()`, so
    /// the last mon to act in a turn cannot shield: a foe that switched has
    /// already spent its action, and a foe that moved first has too.
    will_act: bool,
    /// A Pursuit is being fired at a mon on its way out: doubled power and
    /// no accuracy roll, per the move's own callbacks.
    pursuing: bool,
    /// A switch this side chose that a Pursuit KO pushed to the back of the
    /// turn. Gen 2-4 still honour it (the sim re-queues at priority -101),
    /// so the slot stays empty until the move phase is over.
    deferred_switch: [Option<usize>; 2],
    /// Which side this turn's speeds put first, read once as the turn opens
    /// and held for the rest of it.
    speed_first: Option<usize>,
}

impl Battle {
    pub fn new(side1: Side, side2: Side, seed: u64) -> Battle {
        let mut battle = Battle {
            sides: [side1, side2],
            rng: Rng::new(seed),
            turn: 0,
            weather: None,
            weather_n: 0,
            taken_physical: [0; 2],
            taken_special: [0; 2],
            pp0_at_choice: [false; 2],
            acted_this_turn: [false; 2],
            dragged: [false; 2],
            will_act: false,
            pursuing: false,
            deferred_switch: [None; 2],
            speed_first: None,
        };
        // The battle's opening switch-ins are switch-ins, and the sim runs
        // them as part of starting: Intimidate cows, Trace copies, a weather
        // ability lays its sky down, all in speed order. They belong here
        // rather than on the first turn, because what a player is allowed to
        // CHOOSE on turn one already depends on them — an Arena Trap staring
        // across the field takes the switch off the menu before anyone has
        // moved.
        let mut opening = Vec::new();
        let first = battle.faster_side(true);
        for side in [first, 1 - first] {
            battle.greet(side, false, &mut opening);
        }
        // A status a mon was handed before the battle is still subject to the
        // sky those openers just laid down: nothing freezes under sun, and
        // the sim refuses the status rather than thawing it later.
        // A status a mon was handed before the battle is still subject to
        // what those openers just did. Nothing freezes under sun, and an
        // ability that refuses a status refuses it here too — the sim never
        // applies it in the first place, which is why a Limber mon handed
        // paralysis is at full Speed on turn one rather than curing it a
        // moment later. A BERRY, by contrast, is not eaten yet: those wait
        // for the first update of the turn, after the speeds are read.
        let sun = battle.effective_weather() == Some(Weather::Sun);
        for w in 0..2 {
            let active = battle.sides[w].active;
            for (i, mon) in battle.sides[w].party.iter_mut().enumerate() {
                let Some(st) = mon.status else { continue };
                // The sky reaches the whole party; an ability speaks only for
                // the mon holding it, and only while that mon is out.
                let refused = i == active && ability::blocks_status(&mon.bearer(), st);
                if (sun && st == Status::Freeze) || refused {
                    mon.status = None;
                    mon.sleep_n = 0;
                }
            }
        }
        battle
    }

    /// Whether `side`'s active mon may switch out at all. The sim marks a
    /// mon `trapped` for two separate reasons and both belong here: an
    /// effect holding it in place (a bind, Mean Look), and a move holding
    /// its own turn — `getLockedMove` covers a rampage, a rolling
    /// Rollout/Ice Ball, a storing Bide, an Uproar, a charge turn and a
    /// Hyper Beam recharge, and every one of those sets `trapped = true`.
    /// The sim's `onUpdate`, which runs between actions and is where the
    /// refusing abilities do their tidying: they do not merely block a
    /// status arriving, they shed one already there. A mon that walks into
    /// the battle asleep with Insomnia is awake before it has to act.
    fn ability_update(&mut self, side: usize) {
        let mon = self.sides[side].mon();
        if mon.fainted() {
            return;
        }
        let bearer = mon.bearer();
        if mon.status.is_some_and(|st| ability::blocks_status(&bearer, st)) {
            let mon = self.sides[side].mon_mut();
            mon.status = None;
            mon.sleep_n = 0;
            mon.sleep_skipped = 0;
            mon.toxic_n = 0;
        }
        if ability::blocks_confusion(&bearer) {
            self.sides[side].mon_mut().confusion_n = 0;
        }
        // The curing berries are `onUpdate` items too: they are eaten the
        // moment the status lands, not at the end of the turn like the
        // healing ones.
        let holder = self.sides[side].mon().holder();
        let cure_status = self
            .sides[side]
            .mon()
            .status
            .is_some_and(|st| item::cures_status(&holder, st));
        let cure_confusion =
            self.sides[side].mon().confusion_n > 0 && item::cures_confusion(&holder);
        if cure_status || cure_confusion {
            let mon = self.sides[side].mon_mut();
            mon.last_item = mon.item;
            mon.item = "";
            if cure_status {
                mon.status = None;
                mon.sleep_n = 0;
                mon.sleep_skipped = 0;
                mon.toxic_n = 0;
            }
            if cure_confusion {
                mon.confusion_n = 0;
            }
        }
    }

    /// The weather as the field actually feels it. Air Lock and Cloud Nine
    /// do not clear the sky — the weather keeps running its clock down — but
    /// while either is out nothing reads it.
    pub fn effective_weather(&self) -> Option<Weather> {
        if (0..2).any(|w| {
            !self.sides[w].mon().fainted()
                && ability::suppresses_weather(&self.sides[w].mon().bearer())
        }) {
            return None;
        }
        self.weather
    }

    /// A mon's Speed for the turn order, with the abilities that read the
    /// sky folded in.
    fn turn_speed(&self, side: usize) -> u16 {
        let mon = self.sides[side].mon();
        let spe = item::speed_chain(&mon.holder()).apply(mon.effective_speed() as u32) as u16;
        let sky = self.effective_weather();
        if ability::speed_doubles(
            &mon.bearer(),
            sky == Some(Weather::Sun),
            sky == Some(Weather::Rain),
        ) {
            spe.saturating_mul(2)
        } else {
            spe
        }
    }

    pub fn can_switch(&self, side: usize) -> bool {
        let mon = self.sides[side].mon();
        if mon.fainted() {
            return true; // a replacement is always allowed in
        }
        // Ingrain's roots hold it as surely as a bind does: the condition's
        // onTrapPokemon calls tryTrap, so the request comes back trapped.
        if mon.trapped_n > 0 || mon.mean_looked || mon.ingrained {
            return false;
        }
        // The other side's ability can hold it in place too. Being off the
        // ground is what gets past Arena Trap.
        let foe = self.sides[1 - side].mon();
        let (t1, t2) = mon.types();
        let grounded =
            t1 != Type::Flying && t2 != Type::Flying && mon.ability != "levitate";
        if !foe.fainted() && ability::traps(&foe.bearer(), &mon.bearer(), grounded) {
            return false;
        }
        !(mon.rampage.is_some()
            || mon.rolling.is_some()
            || mon.bide.is_some()
            || mon.charging.is_some()
            || mon.must_recharge)
    }

    /// Base Attack of everyone Beat Up can call: unfainted and unstatused,
    /// the active first (the sim's `side.pokemon` keeps it at the front) and
    /// the rest in party order. Order only decides which strike lands first,
    /// since every ally gets one.
    fn beatup_allies(&self, side: usize) -> Vec<u16> {
        let s = &self.sides[side];
        let ok = |m: &Mon| !m.fainted() && m.status.is_none();
        let mut out = Vec::new();
        if ok(s.mon()) {
            out.push(s.mon().species.base.atk as u16);
        }
        for (i, m) in s.party.iter().enumerate() {
            if i != s.active && ok(m) {
                out.push(m.species.base.atk as u16);
            }
        }
        out
    }

    /// Move slots the sim would actually offer this side: PP left and not
    /// shut off by Disable, Taunt, Torment or Imprison. The sim rejects a
    /// choice outside this set outright rather than substituting anything,
    /// so a caller picking a move should pick from here.
    pub fn selectable_moves(&self, side: usize) -> Vec<usize> {
        let mon = self.sides[side].mon();
        // Imprison seals every move the imprisoner itself knows.
        let foe = self.sides[1 - side].mon();
        let sealed = |id: &str| {
            foe.imprisoning && foe.moves.iter().any(|m| m.entry.id == id)
        };
        (0..mon.moves.len())
            .filter(|&i| {
                let slot = &mon.moves[i];
                // "Status move" is the CATEGORY, not zero base power:
                // Dragon Rage, Seismic Toss, Night Shade, Psywave, the OHKOs
                // and Counter all sit at power 0 and Taunt lets them through.
                let status_move = slot.entry.power == 0
                    && slot.entry.fixed.is_none()
                    && !slot.entry.ohko
                    && !matches!(slot.entry.id, "counter" | "mirrorcoat" | "spitup");
                slot.pp > 0
                    && !sealed(slot.entry.id)
                    && mon.disabled_slot != Some(i as u8)
                    && !(mon.taunt_n > 0 && status_move)
                    && !(mon.tormented && mon.last_used == Some(i as u8))
                    // Encore greys out everything BUT the encored move.
                    && !(mon.encore_n > 0 && mon.last_used.is_some_and(|u| u != i as u8))
                    // A Choice Band greys out everything but the first swing.
                    && !mon
                        .choice_locked
                        .is_some_and(|id| id != slot.entry.id)
            })
            .collect()
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
        // The sim sorts this turn's actions as the choices come in, which is
        // before the turn's first update(). A Cheri Berry eaten a moment
        // later cures the paralysis but does not reorder the turn, so the
        // speeds are read here and held.
        let scripted_now = script.seats.iter().any(|s| s.is_some());
        self.speed_first = Some(self.faster_side(scripted_now));
        for side in 0..2 {
            self.ability_update(side);
            if !self.sides[side].mon().fainted() {
                let mon = self.sides[side].mon_mut();
                mon.active_turns = mon.active_turns.saturating_add(1);
            }
        }
        self.turn += 1;
        self.acted_this_turn = [false; 2];
        self.dragged = [false; 2];
        self.taken_physical = [0; 2];
        self.taken_special = [0; 2];

        for side in 0..2 {
            self.pp0_at_choice[side] = match choices[side] {
                Choice::Move(i) => self.sides[side]
                    .mon()
                    .moves
                    .get(i)
                    .is_some_and(|m| m.pp == 0),
                _ => false,
            };
        }
        // A seat that chose Focus Punch starts tightening its focus before
        // anything else happens this turn (the sim's priority charge step);
        // while focusing, the flinch volatile is refused outright.
        for side in 0..2 {
            if let Choice::Move(i) = choices[side] {
                if self.sides[side]
                    .mon()
                    .moves
                    .get(i)
                    .is_some_and(|m| m.entry.id == "focuspunch")
                    && !self.sides[side].mon().fainted()
                {
                    self.sides[side].mon_mut().focusing = true;
                }
            }
        }

        // Pursuit fires at a mon on its way out, before the switch happens
        // and at double power. The user's own action is spent doing it (the
        // sim cancels the queued move), and if the strike lands a KO the
        // chosen switch is not cancelled — this era re-queues it for the end
        // of the turn, so the slot simply stays empty until then.
        // Whether each side MAY switch is settled once, before anything
        // moves. The sim decides it when it builds the turn's request, so
        // two mons can trade places even though each would hold the other
        // in place once it arrived — re-asking mid-turn let the first
        // switch-in's Shadow Tag cancel the second side's answer.
        let may_switch = [self.can_switch(0), self.can_switch(1)];
        let mut pursued = [false; 2];
        self.deferred_switch = [None; 2];
        for side in 0..2 {
            let Choice::Switch(idx) = choices[side] else {
                continue;
            };
            if !may_switch[side] || self.sides[side].mon().fainted() {
                continue;
            }
            let foe = 1 - side;
            let Choice::Move(mi) = choices[foe] else {
                continue;
            };
            // The chosen slot only counts if the mon is free to use it: one
            // locked into a charge, a rampage, a Rollout, a Bide or a
            // recharge is swinging that instead, and the sim never offers
            // Pursuit in its request to begin with.
            let locked = {
                let m = self.sides[foe].mon();
                m.must_recharge
                    || m.charging.is_some()
                    || m.rampage.is_some()
                    || m.rolling.is_some()
                    || m.bide.is_some()
            };
            let is_pursuit = !locked
                && self.sides[foe]
                    .mon()
                    .moves
                    .get(mi)
                    .is_some_and(|m| m.entry.id == "pursuit");
            let able = !self.sides[foe].mon().fainted()
                && !matches!(
                    self.sides[foe].mon().status,
                    Some(Status::Freeze | Status::Sleep)
                );
            if !is_pursuit || !able {
                continue;
            }
            self.pursuing = true;
            self.use_move(foe, mi, script.seats[foe], &mut events);
            self.pursuing = false;
            pursued[foe] = true;
            if self.sides[side].mon().fainted() {
                self.deferred_switch[side] = Some(idx);
            }
        }

        // Switches resolve before any move, in side order. Leaving the field
        // resets a Toxic count: the poison stays, the clock starts over.
        // Switches resolve in SPEED order — the speed of the mon leaving —
        // and each arrival greets the field the moment it lands, before the
        // other side has moved. That is what decides who an Intimidate cows
        // and what a Trace finds standing opposite: on a double switch the
        // slower side's newcomer is greeted by a field that has already
        // changed, and the faster side's by one that has not.
        let switch_first = self.faster_side(scripted_now);
        for side in [switch_first, 1 - switch_first] {
            if self.deferred_switch[side].is_some() {
                continue;
            }
            if let Choice::Switch(idx) = choices[side] {
                if !may_switch[side] {
                    // Held in place, or locked into a move of its own:
                    // switching is refused and the turn is forfeit.
                    continue;
                }
                if idx < self.sides[side].party.len() && !self.sides[side].party[idx].fainted() {
                    self.switch_out_reset(side);
                    self.sides[side].reorder_for_switch(idx);
                    self.sides[side].active = idx;
                    events.push(Event::Switched {
                        side: side as u8 + 1,
                        party_index: idx,
                    });
                    self.switch_in_greet(side, &mut events);
                }
            }
        }

        // A switch-in that dropped to Spikes is replaced before anyone moves.
        self.replace_fainted(&mut events);

        // Then moves: priority bracket first, Speed inside a bracket.
        let scripted = script.seats.iter().any(|s| s.is_some());
        let first = self.first_mover(&choices, scripted);
        // Going down cancels that side's queued action outright, and the
        // replacement does not inherit it — so the cancellation is recorded
        // BEFORE anyone is swapped in, while the slot is still empty.
        let mut cancelled = [false; 2];
        let already_down = (0..2).any(|s| self.sides[s].mon().fainted());
        for side in 0..2 {
            cancelled[side] = pursued[side] || already_down;
        }
        for side in [first, 1 - first] {
            if self.over() {
                break;
            }
            if !cancelled[side] {
                if let Choice::Move(index) = choices[side] {
                    self.acted_this_turn[side] = true;
                    // Whoever is left in the order still has an action; the
                    // second mover has nobody behind it.
                    let foe = 1 - side;
                    self.will_act =
                        side == first && !cancelled[foe] && matches!(choices[foe], Choice::Move(_));
                    self.use_move(side, index, script.seats[side], &mut events);
                }
            }
            // ANY faint stops the rest of the turn dead in this era. The
            // sim's faintMessages runs `cancelAction` over every active mon
            // when `gen <= 3 && singles`, not just over the one that went
            // down — so the survivor's queued move is thrown away too. A
            // one-mon battle could never show this: the faint ended it.
            if (0..2).any(|s| self.sides[s].mon().fainted()) {
                cancelled = [true; 2];
            }
            // A mon dragged off by Roar or Whirlwind takes its action with
            // it; whoever the drag brought in does not get to use it.
            for s in 0..2 {
                cancelled[s] |= self.dragged[s];
            }
            // The sim checks for faints at every action boundary in this
            // era (`gen <= 3` in checkFainted's guard), so a replacement is
            // already on the field when the residual phase runs.
            self.replace_fainted(&mut events);
        }

        // The switch a Pursuit KO pushed back takes its turn now, once the
        // moves are done and BEFORE the residuals — the sim re-queues it at
        // priority -101, so the mon it brings in is on the field to take its
        // own poison tick.
        for side in 0..2 {
            if let Some(idx) = self.deferred_switch[side].take() {
                if idx < self.sides[side].party.len() && !self.sides[side].party[idx].fainted() {
                    self.sides[side].mon_mut().status = None;
                    self.sides[side].reorder_for_switch(idx);
                    self.sides[side].active = idx;
                    events.push(Event::Switched {
                        side: side as u8 + 1,
                        party_index: idx,
                    });
                    self.switch_in_greet(side, &mut events);
                }
            }
        }
        self.replace_fainted(&mut events);

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
                        // Half of whoever CATCHES it, not half of whoever
                        // made it: `target.baseMaxhp / 2` in the sim's onEnd.
                        // Gen 5 moved it to the wisher; this era did not.
                        let amount = self.sides[side].mon().max_hp / 2;
                        let mon = self.sides[side].mon_mut();
                        if !mon.fainted() {
                            let heal = amount.min(mon.max_hp - mon.hp);
                            if heal > 0 {
                                mon.hp += heal;
                                events.push(Event::Healed {
                                    side: side as u8 + 1,
                                    amount: heal,
                                });
                            }
                        }
                    }
                }
            }

            // The clock runs down first: the sim's field residual decrements the
            // weather's duration and clears it before `onWeather` would chip, so
            // a five-turn sandstorm lands FOUR ticks, not five. Nothing under a
            // three-turn fuzz could ever have noticed.
            if let Some(weather) = self.weather {
                self.weather_n = self.weather_n.saturating_sub(1);
                if self.weather_n == 0 {
                    self.weather = None;
                    events.push(Event::WeatherEnded { weather });
                }
            }

            if matches!(self.effective_weather(), Some(Weather::Sandstorm | Weather::Hail)) {
                let sand = self.effective_weather() == Some(Weather::Sandstorm);
                let first = self.faster_side(scripted);
                for side in [first, 1 - first] {
                    let mon = self.sides[side].mon();
                    if mon.fainted() {
                        continue;
                    }
                    let (t1, t2) = mon.types();
                    let immune = if sand {
                        [t1, t2]
                            .iter()
                            .any(|t| matches!(t, Type::Rock | Type::Ground | Type::Steel))
                            || ability::immune_to_sandstorm(&mon.bearer())
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
                    events.push(Event::WeatherDamage {
                        side: side as u8 + 1,
                        amount,
                    });
                    self.announce_faint(side, &mut events);
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
                // Ingrain sips a sixteenth of max HP back (the games' order 7,
                // ahead of Leech Seed).
                if self.sides[side].mon().ingrained && !self.sides[side].mon().fainted() {
                    let mon = self.sides[side].mon_mut();
                    let amount = ((mon.max_hp / 16).max(1)).min(mon.max_hp - mon.hp);
                    if amount > 0 {
                        mon.hp += amount;
                        events.push(Event::Healed {
                            side: side as u8 + 1,
                            amount,
                        });
                    }
                }
                // Rain Dish sips while it rains; Shed Skin gets a third of a
                // chance at shrugging a status off; Speed Boost climbs a
                // stage for every turn spent on the field. All three sit at
                // the sim's residual order 10, subOrder 3.
                if !self.sides[side].mon().fainted() {
                    let bearer = self.sides[side].mon().bearer();
                    if ability::rain_dish(&bearer) && self.effective_weather() == Some(Weather::Rain)
                    {
                        let mon = self.sides[side].mon_mut();
                        let amount = ((mon.max_hp / 16).max(1)).min(mon.max_hp - mon.hp);
                        if amount > 0 {
                            mon.hp += amount;
                            events.push(Event::Healed {
                                side: side as u8 + 1,
                                amount,
                            });
                        }
                    }
                    // A scripted run pins this roll off: it is not one of
                    // the scenario's knobs, and the reference harness leaves
                    // a 33-in-100 alone the same way.
                    if ability::sheds_skin(&bearer)
                        && self.sides[side].mon().status.is_some()
                        && !scripted
                        && self.rng.below(100) < 33
                    {
                        let mon = self.sides[side].mon_mut();
                        mon.status = None;
                        mon.sleep_n = 0;
                        mon.sleep_skipped = 0;
                        mon.toxic_n = 0;
                    }
                    if ability::speed_boosts(&bearer) && self.sides[side].mon().active_turns > 0 {
                        self.sides[side].mon_mut().apply_boost(Boost::Spe, 1);
                        events.push(Event::Boosted {
                            side: side as u8 + 1,
                            boost: Boost::Spe,
                            delta: 1,
                        });
                    }
                    // Truant loafs every other turn, and the toggle flips
                    // here whether or not the mon acted.
                    if ability::truant(&bearer) {
                        let mon = self.sides[side].mon_mut();
                        mon.loafing = !mon.loafing;
                    }
                    // Then the items, at subOrder 4. Gen 3 berries wait for
                    // this phase rather than firing the moment the holder is
                    // hurt, which is why a mon can be knocked out with a
                    // Sitrus still in hand.
                    let holder = self.sides[side].mon().holder();
                    if item::leftovers(&holder) {
                        let mon = self.sides[side].mon_mut();
                        let amount = ((mon.max_hp / 16).max(1)).min(mon.max_hp - mon.hp);
                        if amount > 0 {
                            mon.hp += amount;
                            events.push(Event::Healed {
                                side: side as u8 + 1,
                                amount,
                            });
                        }
                    }
                    self.ripen(side, &mut events);
                }
                // Leech Seed bleeds an eighth of max HP to the opposing active.
                // A seed with nobody to feed does nothing at all: the sim
                // bails on "Nothing to leech into" before it takes a point,
                // so a seeder that just fainted spares its victim entirely.
                if self.sides[side].mon().seeded
                    && !self.sides[side].mon().fainted()
                    && !self.sides[1 - side].mon().fainted()
                {
                    let drain = (self.sides[side].mon().max_hp / 8).max(1);
                    let mon = self.sides[side].mon_mut();
                    let drain = drain.min(mon.hp);
                    mon.hp -= drain;
                    events.push(Event::SeedDrain {
                        side: side as u8 + 1,
                        amount: drain,
                    });
                    let foe = self.sides[1 - side].mon_mut();
                    if !foe.fainted() {
                        let heal = drain.min(foe.max_hp - foe.hp);
                        if heal > 0 {
                            foe.hp += heal;
                            events.push(Event::Healed {
                                side: (1 - side) as u8 + 1,
                                amount: heal,
                            });
                        }
                    }
                    self.announce_faint(side, &mut events);
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
                    events.push(Event::Residual {
                        side: side as u8 + 1,
                        amount,
                        status,
                    });
                    self.announce_faint(side, &mut events);
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
                        self.announce_faint(side, &mut events);
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
                            events.push(Event::TrapDamage {
                                side: side as u8 + 1,
                                amount,
                            });
                            self.announce_faint(side, &mut events);
                        }
                    } else {
                        events.push(Event::TrapEnded {
                            side: side as u8 + 1,
                        });
                    }
                }
                // Ghost-Curse chips a quarter of max HP in this mon's slot.
                if self.sides[side].mon().cursed && !self.sides[side].mon().fainted() {
                    let mon = self.sides[side].mon_mut();
                    let amount = ((mon.max_hp / 4).max(1)).min(mon.hp);
                    mon.hp -= amount;
                    events.push(Event::Residual {
                        side: side as u8 + 1,
                        amount,
                        status: Status::Poison,
                    });
                    self.announce_faint(side, &mut events);
                }
                // Yawn rides THIS mon's residual slot, right after its bind:
                // a faster mon's yawn resolves before a slower mon's poison,
                // and a battle already decided leaves the drowsy awake.
                if self.sides[side].mon().yawn_n > 0 && !self.sides[side].mon().fainted() {
                    self.sides[side].mon_mut().yawn_n -= 1;
                    if self.sides[side].mon().yawn_n == 0 {
                        self.inflict(side, Status::Sleep, scripted, &mut events);
                    }
                }
                // The Thrash-family lock ticks last of all, carrying no
                // residual order of its own. When its two-turn clock runs
                // out the mon is confused whatever it did with the turn;
                // falling asleep calms it only if the sleep arrives while
                // the clock still has time on it.
                if let Some((slot_i, owed)) = self.sides[side].mon().rampage {
                    let uproar = self.sides[side]
                        .mon()
                        .moves
                        .get(slot_i as usize)
                        .is_some_and(|m| m.entry.id == "uproar");
                    if !uproar {
                        let n = if scripted {
                            2
                        } else {
                            2 + self.rng.below(4) as u8
                        };
                        let mon = self.sides[side].mon_mut();
                        mon.rampage_dur = mon.rampage_dur.saturating_sub(1);
                        if mon.rampage_dur == 0 {
                            mon.rampage = None;
                            if owed <= 1 && mon.confusion_n == 0 && !mon.fainted() {
                                mon.confusion_n = n;
                                events.push(Event::ConfusionStarted {
                                    side: side as u8 + 1,
                                });
                            }
                        } else if mon.status == Some(Status::Sleep) {
                            mon.rampage = None;
                        } else {
                            mon.rampage = Some((slot_i, owed.saturating_sub(1)));
                        }
                    }
                }
            }

            // A Future Sight lands at the end of its third turn, computed from
            // the launcher's snapshot against the target now standing.
            for side in 0..2 {
                if self.over() {
                    break;
                }
                if let Some((n, dealt, id)) = self.sides[side].incoming {
                    if n > 1 {
                        self.sides[side].incoming = Some((n - 1, dealt, id));
                    } else {
                        self.sides[side].incoming = None;
                        let mon = self.sides[side].mon();
                        if !mon.fainted() {
                            // A target mid Fly/Dig/Bounce/Dive when the hit
                            // arrives dodges it like any other attack.
                            if mon.semi_invulnerable().is_some() {
                                continue;
                            }
                            // Only ACCURACY waits for the landing — 90 for
                            // Future Sight, 85 for Doom Desire — off the
                            // launcher's seat script for that turn. A miss
                            // simply drops the delayed hit. The damage itself
                            // was locked in at launch; the sim even strips the
                            // target's Endure before it lands.
                            let landed = match script.seats[1 - side] {
                                Some(sc) => sc.hit,
                                None => {
                                    self.rng.below(100) < if id == "doomdesire" { 85 } else { 90 }
                                }
                            };
                            if !landed {
                                continue;
                            }
                            if dealt > 0 {
                                let mon = self.sides[side].mon_mut();
                                let hit_sub = mon.sub_hp > 0;
                                if hit_sub {
                                    let amount = dealt.min(mon.sub_hp);
                                    mon.sub_hp -= amount;
                                    events.push(Event::SubDamage {
                                        side: side as u8 + 1,
                                        amount,
                                    });
                                    if self.sides[side].mon().sub_hp == 0 {
                                        events.push(Event::SubBroke {
                                            side: side as u8 + 1,
                                        });
                                    }
                                } else {
                                    let amount = dealt.min(mon.hp);
                                    mon.hp -= amount;
                                    events.push(Event::Damage {
                                        side: side as u8 + 1,
                                        amount,
                                        effectiveness: 100,
                                        crit: false,
                                    });
                                    self.announce_faint(side, &mut events);
                                }
                            }
                        }
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
                    events.push(Event::PerishCount {
                        side: side as u8 + 1,
                        n,
                    });
                    if n == 0 {
                        self.sides[side].mon_mut().hp = 0;
                        self.announce_faint(side, &mut events);
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
                            events.push(Event::SideEnded {
                                side: side as u8 + 1,
                                condition: cond,
                            });
                        }
                    }
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
                    mon.disable_fresh = false;
                    if mon.disable_skip_tick {
                        mon.disable_skip_tick = false;
                        continue;
                    }
                    mon.disable_n -= 1;
                    if mon.disable_n == 0 {
                        mon.disabled_slot = None;
                    }
                }
            }
        }

        // The residual phase is one action as far as the queue is concerned:
        // whoever it knocked out is replaced when it finishes, not between
        // its individual ticks.
        self.replace_fainted(&mut events);

        // A flinch lasts exactly the turn it landed in — as do Protect's
        // shield and Endure's brace.
        for side in 0..2 {
            let mon = self.sides[side].mon_mut();
            mon.flinched = false;
            mon.protected = false;
            mon.enduring = false;
            mon.torment_fresh = false;
            mon.imprison_fresh = false;
            mon.uproar_ending = false;
            // A rolling lock that did not swing this turn is over.
            if mon.rolling.is_some() {
                if mon.rolling_fresh {
                    mon.rolling_fresh = false;
                } else {
                    mon.rolling = None;
                    mon.locked_move = None;
                }
            }
            // A charge only gets the one turn to come down.
            if mon.charging.is_some() {
                if mon.charge_fresh {
                    mon.charge_fresh = false;
                } else {
                    mon.charging = None;
                    mon.charge_fresh = false;
                    mon.locked_move = None;
                }
            }
            mon.focusing = false;
            mon.sure_hit = mon.sure_hit.saturating_sub(1);
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
        let Some(slot) = mon.moves.get(i) else {
            return false;
        };
        let status_movish = slot.entry.power == 0
            && slot.entry.fixed.is_none()
            && !slot.entry.ohko
            && !matches!(slot.entry.id, "counter" | "mirrorcoat" | "spitup");
        let foe_mon = self.sides[1 - side].mon();
        slot.pp == 0
            || mon.disabled_slot == Some(i as u8)
            || (mon.tormented && mon.last_used_id == Some(slot.entry.id))
            || (mon.taunt_n == 1 && status_movish)
            || (foe_mon.imprisoning && foe_mon.moves.iter().any(|m| m.entry.id == slot.entry.id))
    }

    fn first_mover(&mut self, choices: &[Choice; 2], scripted: bool) -> usize {
        let prio = |side: usize| match choices[side] {
            Choice::Move(i) => {
                if self.struggles_at_choice(side, i) {
                    0
                } else {
                    // A locked mon swings the LOCK, whatever the player
                    // picked, so the bracket is the lock's. Getting this
                    // from the chosen slot instead hands a mon mid-Ice-Ball
                    // the +3 of an Endure it will never use.
                    let mon = self.sides[side].mon();
                    let locked = mon
                        .locked_move
                        .filter(|_| {
                            mon.rampage.is_some()
                                || mon.rolling.is_some()
                                || mon.bide.is_some()
                                || mon.charging.is_some()
                        })
                        .and_then(crate::data::move_by_id);
                    match locked {
                        Some(e) => e.priority,
                        None if mon.must_recharge => 0,
                        None => mon.moves.get(i).map(|s| s.entry.priority).unwrap_or(0),
                    }
                }
            }
            Choice::Switch(_) => 0,
        };
        let (p0, p1) = (prio(0), prio(1));
        match p0.cmp(&p1) {
            core::cmp::Ordering::Greater => 0,
            core::cmp::Ordering::Less => 1,
            core::cmp::Ordering::Equal => match self.speed_first {
                // The order was settled as the turn opened; a berry eaten
                // since then cures the paralysis without reshuffling it.
                Some(first) => first,
                None => self.faster_side(scripted),
            },
        }
    }

    /// Which side moves first this turn: higher Speed (paralysis included),
    /// RNG on a tie in play. Under a script a tie goes to player 1, matching
    /// the reference sim with its tie-shuffle pinned to insertion order.
    fn faster_side(&mut self, scripted: bool) -> usize {
        let s0 = self.turn_speed(0);
        let s1 = self.turn_speed(1);
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
        // Fake Out's window is ACTIONS, not moves: a sleep-lost turn still
        // burns it (the sim counts every action taken from the queue).
        let had_acted = self.sides[side].mon().acted;
        self.sides[side].mon_mut().acted = true;
        // In Gen 3 a rampage broken by flinch or full paralysis still ends
        // in fatigue confusion, on the spot. (A miss or a protecting target
        // ends it quietly — those sites clear `rampage` directly.)
        fn break_rampage(b: &mut Battle, side: usize, scripted: bool, events: &mut Vec<Event>) {
            if let Some((slot_i, _)) = b.sides[side].mon().rampage {
                let uproar = b.sides[side]
                    .mon()
                    .moves
                    .get(slot_i as usize)
                    .is_some_and(|m| m.entry.id == "uproar");
                let n = if scripted {
                    2
                } else {
                    2 + b.rng.below(4) as u8
                };
                let mon = b.sides[side].mon_mut();
                mon.rampage = None;
                if !uproar && mon.confusion_n == 0 && !mon.fainted() {
                    mon.confusion_n = n;
                    events.push(Event::ConfusionStarted {
                        side: side as u8 + 1,
                    });
                }
            }
        }
        // Destiny Bond and Grudge last exactly until the user's next action.
        self.sides[side].mon_mut().destiny = false;
        self.sides[side].mon_mut().grudged = false;

        // An intercepting Pursuit skips every can't-move gate below. The sim
        // fires it from the switch-out hook through `useMove`, which is the
        // inside caller: it never runs the BeforeMove event, so a paralysed
        // or flinched user still gets its strike in. (Sleep and freeze are
        // refused earlier, by the condition's own guard.)
        let mut asleep_now = false;
        if !self.pursuing {

        // Recharging after Hyper Beam and kin: the whole action is spent,
        // gated even above sleep, matching the games' priority order.
        if self.sides[side].mon().must_recharge {
            self.sides[side].mon_mut().must_recharge = false;
            self.sides[side].mon_mut().stall_counter = 0;
            events.push(Event::Recharging {
                side: side as u8 + 1,
            });
            return;
        }
        // Fast asleep: the sleep clock ticks down before each action, and at
        // zero the mon wakes and moves that same turn.
        if self.sides[side].mon().status == Some(Status::Sleep) {
            let snoring = self.sides[side]
                .mon()
                .moves
                .get(index)
                .is_some_and(|m| matches!(m.entry.id, "snore" | "sleeptalk"));
            let mon = self.sides[side].mon_mut();
            if ability::early_bird(&mon.bearer()) {
                mon.sleep_n = mon.sleep_n.saturating_sub(1);
            }
            mon.sleep_n = mon.sleep_n.saturating_sub(1);
            if mon.sleep_n == 0 {
                mon.status = None;
                mon.sleep_skipped = 0;
            } else if snoring {
                // Snore attacks straight out of sleep, and Gen 3 refunds the
                // turn on switch-in rather than counting it.
                mon.sleep_skipped += 1;
                asleep_now = true;
                events.push(Event::Cant {
                    side: side as u8 + 1,
                    status: Status::Sleep,
                });
            } else {
                mon.charging = None;
                mon.charge_fresh = false;
                mon.sleep_skipped = 0;
                // A Thrash-family lock is NOT dropped here: the sim lets the
                // sleeper keep it and settles the matter in the residual
                // phase, which is why a mon slept out of its final swing
                // still wakes up confused.
                if mon.rampage.is_some_and(|(slot_i, _)| {
                    mon.moves
                        .get(slot_i as usize)
                        .is_some_and(|m| m.entry.id == "uproar")
                }) {
                    mon.rampage = None;
                }
                mon.rolling = None;
                mon.fury_n = 0;
                mon.raging = false;
                mon.stall_counter = 0;
                events.push(Event::Cant {
                    side: side as u8 + 1,
                    status: Status::Sleep,
                });
                return;
            }
        }
        // Truant: every other turn is spent loafing about, and the turn it
        // arrives counts as one unless the battle has not started.
        if self.sides[side].mon().loafing
            && ability::truant(&self.sides[side].mon().bearer())
        {
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        // Frozen solid: a 1-in-5 thaw each action in play (scripts pin it
        // off, matching the reference runs). Flame Wheel and Sacred Fire
        // pass THROUGH this gate — but the cure itself only lands when the
        // move actually executes, so a flinch or full paralysis after this
        // gate leaves the user frozen.
        if self.sides[side].mon().status == Some(Status::Freeze) {
            let defrost = self.sides[side]
                .mon()
                .moves
                .get(index)
                .is_some_and(|slot| matches!(slot.entry.id, "flamewheel" | "sacredfire"));
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
                self.sides[side].mon_mut().raging = false;
                self.sides[side].mon_mut().stall_counter = 0;
                events.push(Event::Cant {
                    side: side as u8 + 1,
                    status: Status::Freeze,
                });
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
            self.sides[side].mon_mut().raging = false;
            self.sides[side].mon_mut().stall_counter = 0;
            events.push(Event::Flinched {
                side: side as u8 + 1,
            });
            return;
        }
        // Confusion: the clock ticks before the action; at zero it lifts
        // and the move proceeds. Otherwise a coin (the script's selfhit
        // knob) decides between acting and the 40 BP typeless self-hit.
        if self.sides[side].mon().confusion_n > 0 {
            self.sides[side].mon_mut().confusion_n -= 1;
            if self.sides[side].mon().confusion_n == 0 {
                events.push(Event::ConfusionEnded {
                    side: side as u8 + 1,
                });
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
                    self.sides[side].mon_mut().raging = false;
                    self.sides[side].mon_mut().stall_counter = 0;
                    let amount = self.confusion_self_hit(side, random);
                    events.push(Event::ConfusedHit {
                        side: side as u8 + 1,
                        amount,
                    });
                    self.announce_faint(side, events);
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
                self.sides[side].mon_mut().raging = false;
                self.sides[side].mon_mut().stall_counter = 0;
                events.push(Event::FullyParalyzed {
                    side: side as u8 + 1,
                });
                return;
            }
        }
        }
        // Encore overrides the choice with the last move used.
        let index = match (
            self.sides[side].mon().encore_n > 0,
            self.sides[side].mon().last_used,
        ) {
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
        let was_charging = self.sides[side].mon().charging.is_some();
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
        // Any other lock forces its own slot too — a rolling Rollout or Ice
        // Ball, a storing Bide. It has to be the SLOT and not just the move
        // entry, because the PP comes off it and Grudge and Spite both read
        // `last_used` to decide what to drain.
        let index = if releasing {
            match self.sides[side].mon().locked_move {
                Some(id) => self.sides[side]
                    .mon()
                    .moves
                    .iter()
                    .position(|m| m.entry.id == id)
                    .unwrap_or(index),
                None => index,
            }
        } else {
            index
        };
        // A Disabled locked move cannot continue — a mid-charge release, a
        // rolling Rollout or a rampage all lose the turn silently (the
        // sim's disable cant; a broken rampage still fatigues).
        if releasing && self.sides[side].mon().disabled_slot == Some(index as u8) {
            let _ = was_charging;
            break_rampage(self, side, script.is_some(), events);
            self.sides[side].mon_mut().rolling = None;
            self.sides[side].mon_mut().stall_counter = 0;
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        let Some(slot) = self.sides[side].mon().moves.get(index).copied() else {
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        };
        // A rampage locked in through Mirror Move keeps swinging the CALLED
        // move on its follow-up turns — run it directly, one announced line.
        let slot = if releasing {
            match self.sides[side].mon().locked_move {
                Some(id) if id != slot.entry.id => match crate::data::move_by_id(id) {
                    Some(e) => MoveSlot {
                        entry: e,
                        pp: 1,
                        typed_as: None,
                    },
                    None => slot,
                },
                _ => slot,
            }
        } else {
            slot
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
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        let taunted_out = self.sides[side].mon().taunt_n == 1 && status_movish;
        // Torment: the same move twice in a row becomes Struggle. So does
        // a Disabled slot, or a move the imprisoning foe also knows.
        let tormented_out = self.sides[side].mon().tormented
            && !self.sides[side].mon().torment_fresh
            && self.sides[side].mon().last_used_id == Some(slot.entry.id)
            && !releasing;
        if self.sides[side].mon().disabled_slot == Some(index as u8)
            && self.sides[side].mon().disable_fresh
            && !releasing
        {
            // Disabled mid-turn: the chosen move is simply lost.
            self.sides[side].mon_mut().disable_fresh = false;
            self.sides[side].mon_mut().stall_counter = 0;
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        let disabled_out = self.sides[side].mon().disabled_slot == Some(index as u8) && !releasing;
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
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        let imprisoned_out = sealed && !releasing;
        if slot.pp == 0
            && !releasing
            && !self.pp0_at_choice[side]
            && !(taunted_out || tormented_out || disabled_out || imprisoned_out)
        {
            // Drained to zero AFTER the choice was made: the sim's runMove
            // hits "cant: nopp" — a silent lost turn, no Struggle.
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        let struggling = (taunted_out
            || tormented_out
            || disabled_out
            || imprisoned_out
            || (slot.pp == 0 && !releasing && self.pp0_at_choice[side]))
            && !releasing;
        let slot = if struggling {
            MoveSlot {
                entry: &crate::data::STRUGGLE,
                pp: 1,
                typed_as: None,
            }
        } else {
            slot
        };
        if !releasing && !struggling {
            // Pressure charges an extra point for anything aimed across the
            // field. A move the user turns on itself is free of it.
            let cost = if ability::pressure(&self.sides[1 - side].mon().bearer())
                && !self.sides[1 - side].mon().fainted()
                && slot.entry.pressured
                && (slot.entry.id != "curse" || {
                    let (t1, t2) = self.sides[side].mon().types();
                    t1 == Type::Ghost || t2 == Type::Ghost
                })
            {
                2
            } else {
                1
            };
            let pp = &mut self.sides[side].mon_mut().moves[index].pp;
            *pp = pp.saturating_sub(cost);
        }
        // A defrosting move thaws its user the moment it actually goes off.
        if self.sides[side].mon().status == Some(Status::Freeze)
            && matches!(slot.entry.id, "flamewheel" | "sacredfire")
        {
            self.sides[side].mon_mut().status = None;
        }
        // A Choice Band clamps shut behind the move it commits to, whatever
        // then becomes of it: a miss or a failure spends the choice too.
        if self.sides[side].mon().item == "choiceband" {
            let id = slot.entry.id;
            self.sides[side].mon_mut().choice_locked = Some(id);
        }
        events.push(Event::Used {
            side: side as u8 + 1,
            move_index: index,
        });
        {
            let mon = self.sides[side].mon_mut();
            mon.last_used = if struggling { None } else { Some(index as u8) };
            mon.last_used_id = Some(slot.entry.id);
            mon.last_missed = false;
            // Using any move ends an ongoing rage; Rage itself re-arms it
            // only once it actually lands (a missed Rage never rages).
            mon.raging = false;
        }

        // No living foe to aim at: the sim logs the move and stops there
        // (`-notarget`), PP already spent. Self- and field-aimed moves go
        // off regardless of whether the other slot is empty.
        if slot.entry.needs_target
            && self.sides[foe].mon().fainted()
            && !matches!(slot.entry.id, "futuresight" | "doomdesire")
        {
            return;
        }

        // Nature Power becomes Swift in the sim's default arena; Hidden
        // Power under the fuzz's uniform maxed IVs is Dark 70.
        let slot = if slot.entry.id == "metronome" {
            // The sim samples the num-sorted eligible list; pinned, that is
            // its first entry — Pound. Play keeps a real random pick.
            let called: &'static MoveEntry = match script {
                Some(_) => move_by_id("pound").expect("pound"),
                None => {
                    let mut pick = move_by_id("pound").expect("pound");
                    for _ in 0..16 {
                        let i = self.rng.below(crate::data::MOVES.len() as u32) as usize;
                        let cand = &crate::data::MOVES[i];
                        if !matches!(
                            cand.id,
                            "metronome" | "struggle" | "mirrormove" | "sketch" | "mimic"
                        ) {
                            pick = cand;
                            break;
                        }
                    }
                    pick
                }
            };
            // Both lines are announced: Metronome, then the call.
            events.push(Event::Used {
                side: side as u8 + 1,
                move_index: index,
            });
            MoveSlot {
                entry: called,
                pp: 1,
                typed_as: None,
            }
        } else if slot.entry.id == "sleeptalk" {
            // Only out of a sleep, and it calls one of the user's OWN moves:
            // anything flagged nosleeptalk or charge is skipped, and a
            // pinned sample lands on the first survivor. A pick with no PP
            // left is a silent lost turn rather than a call.
            if !asleep_now {
                events.push(Event::Failed { side: side as u8 + 1 });
                return;
            }
            let pick = self.sides[side]
                .mon()
                .moves
                .iter()
                .find(|m| !m.entry.no_sleep_talk && !m.entry.charge)
                .copied();
            match pick {
                None => {
                    events.push(Event::Failed { side: side as u8 + 1 });
                    return;
                }
                Some(m) if m.pp == 0 => return,
                Some(m) => {
                    events.push(Event::Used { side: side as u8 + 1, move_index: index });
                    MoveSlot { entry: m.entry, pp: 1, typed_as: None }
                }
            }
        } else if slot.entry.id == "assist" {
            // Assist rummages through the REST of the party, in the sim's
            // own `side.pokemon` order, and calls the first move that is not
            // on the noassist list.
            let called = {
                let s = &self.sides[side];
                s.order
                    .iter()
                    .copied()
                    .filter(|&i| i != s.active)
                    .flat_map(|i| s.party[i].moves.iter())
                    .find(|m| !m.entry.no_assist)
                    .map(|m| m.entry)
            };
            match called {
                None => {
                    events.push(Event::Failed { side: side as u8 + 1 });
                    return;
                }
                Some(e) => {
                    events.push(Event::Used { side: side as u8 + 1, move_index: index });
                    MoveSlot { entry: e, pp: 1, typed_as: None }
                }
            }
        } else if slot.entry.id == "naturepower" {
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
            events.push(Event::Used {
                side: side as u8 + 1,
                move_index: index,
            });
            MoveSlot {
                entry: move_by_id("swift").expect("swift"),
                pp: 1,
                typed_as: None,
            }
        } else if slot.entry.id == "hiddenpower" && slot.typed_as.is_none() {
            MoveSlot {
                entry: slot.entry,
                pp: slot.pp,
                typed_as: Some(Type::Dark),
            }
        } else {
            slot
        };

        // A called move can itself be a caller: Assist and Sleep Talk both
        // reach Nature Power, and the sim's useMove simply recurses. Run the
        // one chain that exists in this era rather than stopping a link
        // short, which left the call announced and nothing coming out of it.
        let slot = if slot.entry.id == "naturepower" {
            let np_hit = match script {
                Some(s) => s.hit,
                None => self.rng.below(100) < 95,
            };
            if !np_hit {
                return;
            }
            events.push(Event::Used {
                side: side as u8 + 1,
                move_index: index,
            });
            MoveSlot {
                entry: move_by_id("swift").expect("swift"),
                pp: 1,
                typed_as: None,
            }
        } else {
            slot
        };

        // Mirror Move plays back the move the foe last HIT this mon with
        // (the sim's attacked-by book, damaging or status alike) — refusing
        // the noMirror set and a move the foe no longer knows. It swaps in
        // EARLY, before every gate, so the called move runs the whole
        // pipeline: its own accuracy, protect, immunity, fixed-damage arms.
        let slot = if slot.entry.id == "mirrormove" {
            // The attacker has to still BE there with a move to its name:
            // the sim reads `lastAttackedBy.source.lastMove`, and switching
            // out wipes that, so a mon that hit and left leaves nothing to
            // mirror even after it comes back.
            let same_attacker = self.sides[side].mon().last_hit_by_slot
                == Some(self.sides[foe].active)
                && self.sides[foe].mon().last_used_id.is_some();
            let hit_by = self.sides[side].mon().last_hit_by.filter(|_| same_attacker);
            let callable = hit_by
                .filter(|id| {
                    !matches!(
                        *id,
                        "assist"
                            | "curse"
                            | "doomdesire"
                            | "focuspunch"
                            | "futuresight"
                            | "magiccoat"
                            | "metronome"
                            | "mimic"
                            | "mirrormove"
                            | "naturepower"
                            | "psychup"
                            | "roleplay"
                            | "sketch"
                            | "sleeptalk"
                            | "spikes"
                            | "spitup"
                            | "taunt"
                            | "teeterdance"
                            | "transform"
                    ) && self.sides[foe]
                        .mon()
                        .moves
                        .iter()
                        .any(|m| m.entry.id == *id)
                })
                .and_then(crate::data::move_by_id);
            match callable {
                None => {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                Some(e) => {
                    // Both lines are announced: Mirror Move, then the call.
                    events.push(Event::Used {
                        side: side as u8 + 1,
                        move_index: index,
                    });
                    MoveSlot {
                        entry: e,
                        pp: 1,
                        typed_as: None,
                    }
                }
            }
        } else {
            slot
        };

        // Fury Cutter's count belongs to the move that ACTUALLY goes off,
        // not the one announced: a Mirror Move or a Sleep Talk that plays
        // Fury Cutter back keeps the ramp climbing, which is why this sits
        // after the call substitutions rather than with the other
        // announced-move bookkeeping.
        if slot.entry.id != "furycutter" {
            self.sides[side].mon_mut().fury_n = 0;
        }

        // The charge turn of a two-turn move: announce, tuck the slot away,
        // and stop. Skull Bash's era perk raises Defense on the way down.
        let instant_solar = slot.entry.id == "solarbeam" && self.effective_weather() == Some(Weather::Sun);
        if slot.entry.charge && !releasing && !instant_solar {
            events.push(Event::Charging {
                side: side as u8 + 1,
            });
            if slot.entry.id == "skullbash" {
                self.sides[side].mon_mut().apply_boost(Boost::Def, 1);
                events.push(Event::Boosted {
                    side: side as u8 + 1,
                    boost: Boost::Def,
                    delta: 1,
                });
            }
            self.sides[side].mon_mut().charging = Some(index as u8);
            self.sides[side].mon_mut().charge_fresh = true;
            self.sides[side].mon_mut().locked_move = Some(slot.entry.id);
            return;
        }

        // Rollout and Ice Ball lock five doubling uses; Bide stores for two
        // turns; Uproar rolls like a rampage without the hangover.
        if matches!(slot.entry.id, "rollout" | "iceball")
            && self.sides[side].mon().rolling.is_none()
        {
            self.sides[side].mon_mut().rolling = Some(0);
            self.sides[side].mon_mut().locked_move = Some(slot.entry.id);
        }
        if slot.entry.id == "bide" && self.sides[side].mon().bide.is_none() {
            self.sides[side].mon_mut().bide = Some((0, 2));
            self.sides[side].mon_mut().locked_move = Some(slot.entry.id);
            events.push(Event::Charging {
                side: side as u8 + 1,
            });
            return;
        }
        // The unleash: double everything stored, typeless, at the foe.
        if slot.entry.id == "bide" {
            let (stored, _) = self.sides[side].mon().bide.unwrap();
            self.sides[side].mon_mut().bide = None;
            let amount = stored.saturating_mul(2);
            if amount == 0 {
                events.push(Event::Failed {
                    side: side as u8 + 1,
                });
                return;
            }
            let target = self.sides[foe].mon_mut();
            if target.sub_hp > 0 {
                let amount = amount.min(target.sub_hp);
                target.sub_hp -= amount;
                events.push(Event::SubDamage {
                    side: foe as u8 + 1,
                    amount,
                });
                if self.sides[foe].mon().sub_hp == 0 {
                    events.push(Event::SubBroke {
                        side: foe as u8 + 1,
                    });
                }
                return;
            }
            let cap = if target.enduring {
                target.hp.saturating_sub(1)
            } else {
                target.hp
            };
            let amount = amount.min(cap);
            target.hp -= amount;
            self.taken_physical[foe] = amount;
            events.push(Event::Damage {
                side: foe as u8 + 1,
                amount,
                effectiveness: 100,
                crit: false,
            });
            self.sides[foe].mon_mut().last_hit_by = Some(slot.entry.id);
self.sides[foe].mon_mut().last_hit_by_slot = Some(self.sides[side].active);
            self.resolve_faints(side, foe, events);
            return;
        }
        // The thrash family locks in: the games roll 2..3 total attacks
        // (a script pins the floor). The lock starts on first use. Uproar
        // rides the same lock for its pinned 2 (2..5 in play) turns.
        if matches!(
            slot.entry.id,
            "thrash" | "petaldance" | "outrage" | "uproar"
        ) {
            if ramping {
                // Every swing that actually goes off re-arms the lock's own
                // two-turn clock, but only while swings are still owed: the
                // last one lets the clock run out, and that is what fatigues.
                let mon = self.sides[side].mon_mut();
                if mon.rampage.is_some_and(|(_, owed)| owed >= 2) {
                    mon.rampage_dur = 2;
                }
            } else {
                let total: u8 = match script {
                    Some(_) => 2,
                    None if slot.entry.id == "uproar" => 2 + self.rng.below(4) as u8,
                    None => 2 + self.rng.below(2) as u8,
                };
                let uproar = slot.entry.id == "uproar";
                let mon = self.sides[side].mon_mut();
                // Uproar keeps counting its own turns down as it attacks;
                // the Thrash family hands its countdown to the residual
                // phase and stores the swings owed here instead.
                mon.rampage = Some((index as u8, if uproar { total - 1 } else { total }));
                mon.rampage_dur = 2;
                mon.locked_move = Some(slot.entry.id);
            }
        }

        // Spit Up with an empty bank simply fails; otherwise the bank is
        // spent whatever happens next.
        if slot.entry.id == "spitup" {
            if self.sides[side].mon().stockpile_n == 0 {
                events.push(Event::Failed {
                    side: side as u8 + 1,
                });
                return;
            }
        }

        // Focus Punch loses its focus — and the turn — if anything hit the
        // user before it moved. The sim checks in the move's own onTry,
        // after every gate has passed and the PP is already spent.
        if slot.entry.id == "focuspunch"
            && (self.taken_physical[side] > 0 || self.taken_special[side] > 0)
        {
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }

        // Fake Out only works on the user's first action on the field.
        if slot.entry.id == "fakeout" && had_acted {
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        // Beat Up rallies every healthy party member — the sim builds
        // `move.allies` from everyone unfainted and unstatused — and fails
        // only when that leaves nobody at all.
        if slot.entry.id == "beatup" && self.beatup_allies(side).is_empty() {
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        // Dream Eater only bites a sleeping target.
        if slot.entry.id == "dreameater" && self.sides[foe].mon().status != Some(Status::Sleep) {
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        // Present: the sim's random(10) picks heal or a power tier; the
        // pinned roll routes to the secondary knob — floor is the HEAL
        // branch (a quarter of the target's max, failing at full HP).
        if slot.entry.id == "present" {
            let heal_branch = match script {
                Some(sc) => sc.secondary,
                None => self.rng.below(10) < 2,
            };
            if heal_branch {
                let target = self.sides[foe].mon_mut();
                let amount = (target.max_hp / 4).max(1).min(target.max_hp - target.hp);
                if amount == 0 {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                } else {
                    target.hp += amount;
                    events.push(Event::Healed {
                        side: foe as u8 + 1,
                        amount,
                    });
                }
                return;
            }
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
                    events.push(Event::SideEnded {
                        side: foe as u8 + 1,
                        condition: cond,
                    });
                }
            }
        }

        // Explosion/Self-Destruct: the user faints ON USE, before the hit
        // resolves — a miss or an immune target changes nothing about that.
        if slot.entry.selfdestruct
            && ability::damp_present(
                &self.sides[side].mon().bearer(),
                &self.sides[foe].mon().bearer(),
            )
        {
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
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
            match self.effective_weather() {
                Some(Weather::Rain) => 0,
                Some(Weather::Sun) => 50,
                _ => slot.entry.accuracy,
            }
        } else {
            slot.entry.accuracy
        };
        // Abilities move the accuracy BEFORE the stages, in one chain: the
        // sim runs Compound Eyes, Sand Veil and Hustle on the same event and
        // applies the result once. A move that cannot miss is left alone —
        // the sim's accuracy is `true` there, not a number to modify.
        let acc = if acc == 0 {
            0
        } else {
            let chain = ability::accuracy_chain(
                &self.sides[side].mon().bearer(),
                &self.sides[foe].mon().bearer(),
                slot.move_type(),
                self.effective_weather() == Some(Weather::Sandstorm),
            );
            let after = chain.apply(acc as u32);
            item::accuracy_after_item(&self.sides[foe].mon().holder(), after).clamp(1, 100) as u8
        };
        // Pursuit's onModifyMove sets accuracy true against a mon that is
        // already leaving, so the interception cannot whiff.
        let sure = (self.sides[side].mon().sure_hit > 0
            && self.sides[side].mon().sure_hit_on as usize == self.sides[foe].active)
            || (self.pursuing && slot.entry.id == "pursuit");
        // Nothing consumes the lock: the sim's volatile simply runs out its
        // two-turn duration, so clearing it here both let a second Mind
        // Reader re-apply it and cut it short by a turn.
        let hit = sure
            || match script {
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
        let mut pierce_power_mult: u16 = 1;
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
                    | StatusAction::Camouflage
                    | StatusAction::Conversion
                    | StatusAction::Imprison
                    | StatusAction::Substitute
                    | StatusAction::Ingrain
                    | StatusAction::HealBell
                    | StatusAction::NoopSuccess
                    | StatusAction::BatonPass
                    | StatusAction::SleepTalk
                    | StatusAction::Assist
            )
        ) || (matches!(slot.entry.status_action, Some(StatusAction::Curse))
            && {
                // Non-Ghost Curse retargets SELF (the sim's nonGhostTarget):
                // no shield and no semi-invulnerable foe can stop it.
                let (t1, t2) = self.sides[side].mon().types();
                t1 != Type::Ghost && t2 != Type::Ghost
            });
        // A shield only stops what carries the protect flag — Sketch,
        // Transform and the delayed hits all go straight through one.
        if !self_targeted && slot.entry.protectable && self.sides[foe].mon().protected {
            // A shielded target breaks a rampage the way a miss does —
            // quietly on first use, with fatigue confusion once the lock
            // is running — and a rolling Rollout resets to a fresh choice.
            if ramping {
                break_rampage(self, side, script.is_some(), events);
            } else {
                self.sides[side].mon_mut().rampage = None;
            }
            self.sides[side].mon_mut().rolling = None;
            self.sides[side].mon_mut().fury_n = 0;
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
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
        // A delayed hit is aimed at a slot, not a mon: the sim exempts
        // anything flagged `futuremove` from both the no-target check and
        // the invulnerability one, so it launches at a foe that is
        // underground and at an empty slot alike.
        let futuremove = matches!(slot.entry.id, "futuresight" | "doomdesire");
        if !self_targeted && !futuremove {
            if let Some(via) = self.sides[foe].mon().semi_invulnerable() {
                let pierces = match via {
                    "fly" | "bounce" => {
                        matches!(
                            slot.entry.id,
                            "gust" | "twister" | "thunder" | "skyuppercut"
                        )
                    }
                    "dig" => matches!(slot.entry.id, "earthquake" | "magnitude"),
                    "dive" => matches!(slot.entry.id, "surf" | "whirlpool"),
                    _ => false,
                };
                // A taken aim (Mind Reader, Lock-On) reaches a mon that is
                // not even on the field: the sim's lockon condition answers
                // the Invulnerability event as well as the accuracy one.
                if !pierces && !sure {
                    // A dodge IS a miss: same bookkeeping — including the
                    // fatigue confusion of a broken rampage lock — and the
                    // kicks still crash for half what they would have dealt.
                    self.sides[side].mon_mut().last_missed = true;
                    if ramping {
                        break_rampage(self, side, script.is_some(), events);
                    } else {
                        self.sides[side].mon_mut().rampage = None;
                    }
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
                // WHERE the pierce doubles depends on the hideout: Bounce
                // doubles gust/twister at BASE POWER, while Fly, Dig and
                // Dive all double at the sim's ModifyDamage stage.
                if via == "bounce" && matches!(slot.entry.id, "gust" | "twister") {
                    pierce_power_mult = 2;
                } else if matches!(
                    slot.entry.id,
                    "gust" | "twister" | "earthquake" | "magnitude" | "surf" | "whirlpool"
                ) {
                    pierce_mult = 2;
                }
            }
        }

        // One-hit KO: fails outright against a higher-level target, is
        // stopped by type immunity, and otherwise its hit IS the KO.
        if slot.entry.ohko && ability::blocks_ohko(&self.sides[foe].mon().bearer()) {
            events.push(Event::Damage {
                side: foe as u8 + 1,
                amount: 0,
                effectiveness: 0,
                crit: false,
            });
            return;
        }
        if slot.entry.ohko {
            let eff = crate::types::effectiveness_against(
                slot.move_type(),
                self.sides[foe].mon().types(),
            );
            if eff == 0
                || ability::immune_to_type(&self.sides[foe].mon().bearer(), slot.move_type())
            {
                events.push(Event::Damage {
                    side: foe as u8 + 1,
                    amount: 0,
                    effectiveness: 0,
                    crit: false,
                });
                return;
            }
            if self.sides[foe].mon().level > self.sides[side].mon().level {
                events.push(Event::Failed {
                    side: side as u8 + 1,
                });
                return;
            }
            if !hit {
                return;
            }
            if self.sides[foe].mon().sub_hp > 0 {
                let amount = self.sides[foe].mon().sub_hp;
                self.sides[foe].mon_mut().sub_hp = 0;
                events.push(Event::SubDamage {
                    side: foe as u8 + 1,
                    amount,
                });
                events.push(Event::SubBroke {
                    side: foe as u8 + 1,
                });
                return;
            }
            let mon = self.sides[foe].mon_mut();
            let amount = if mon.enduring {
                mon.hp.saturating_sub(1)
            } else {
                mon.hp
            };
            mon.hp -= amount;
            mon.last_hit_by = Some(slot.entry.id);
            let who = self.sides[side].active;
            self.sides[foe].mon_mut().last_hit_by_slot = Some(who);
            events.push(Event::Damage {
                side: foe as u8 + 1,
                amount,
                effectiveness: 100,
                crit: false,
            });
            self.resolve_faints(side, foe, events);
            return;
        }

        // Fixed damage skips the formula but not the type chart: Seismic
        // Toss still bounces off a Ghost in this era.
        if let Some(kind) = slot.entry.fixed {
            if !hit {
                return;
            }
            let eff = crate::types::effectiveness_against(
                slot.move_type(),
                self.sides[foe].mon().types(),
            );
            if eff == 0 {
                events.push(Event::Damage {
                    side: foe as u8 + 1,
                    amount: 0,
                    effectiveness: 0,
                    crit: false,
                });
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
                events.push(Event::SubDamage {
                    side: foe as u8 + 1,
                    amount,
                });
                if self.sides[foe].mon().sub_hp == 0 {
                    events.push(Event::SubBroke {
                        side: foe as u8 + 1,
                    });
                }
                return;
            }
            let cap = if target.enduring {
                target.hp.saturating_sub(1)
            } else {
                target.hp
            };
            let amount = amount.min(cap);
            target.hp -= amount;
            match crate::types::category_of(slot.move_type()) {
                crate::types::Category::Physical => self.taken_physical[foe] = amount,
                _ => self.taken_special[foe] = amount,
            }
            events.push(Event::Damage {
                side: foe as u8 + 1,
                amount,
                effectiveness: 100,
                crit: false,
            });
            self.sides[foe].mon_mut().last_hit_by = Some(slot.entry.id);
self.sides[foe].mon_mut().last_hit_by_slot = Some(self.sides[side].active);
            // Fixed damage still stokes a raging target and banks in a Bide.
            if let Some((stored, left)) = self.sides[foe].mon().bide {
                self.sides[foe].mon_mut().bide = Some((stored.saturating_add(amount), left));
            }
            if self.sides[foe].mon().raging && !self.sides[foe].mon().fainted() {
                self.sides[foe].mon_mut().apply_boost(Boost::Atk, 1);
                events.push(Event::Boosted {
                    side: foe as u8 + 1,
                    boost: Boost::Atk,
                    delta: 1,
                });
            }
            self.on_damaged(side, foe, &slot, slot.move_type(), script, events);
            self.shell_bell(side, amount, events);
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
                events.push(Event::Damage {
                    side: foe as u8 + 1,
                    amount: 0,
                    effectiveness: 0,
                    crit: false,
                });
                return;
            }
            let (uhp, thp) = (self.sides[side].mon().hp, self.sides[foe].mon().hp);
            if self.sides[foe].mon().sub_hp > 0 || uhp >= thp {
                if self.sides[foe].mon().sub_hp == 0 {
                    self.sides[foe].mon_mut().last_hit_by = Some(slot.entry.id);
self.sides[foe].mon_mut().last_hit_by_slot = Some(self.sides[side].active);
                }
                events.push(Event::Failed {
                    side: side as u8 + 1,
                });
                return;
            }
            let amount = thp - uhp;
            self.sides[foe].mon_mut().hp = uhp;
            self.sides[foe].mon_mut().last_hit_by = Some(slot.entry.id);
self.sides[foe].mon_mut().last_hit_by_slot = Some(self.sides[side].active);
            self.taken_physical[foe] = amount;
            events.push(Event::Damage {
                side: foe as u8 + 1,
                amount,
                effectiveness: 100,
                crit: false,
            });
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
                events.push(Event::Damage {
                    side: foe as u8 + 1,
                    amount: 0,
                    effectiveness: 0,
                    crit: false,
                });
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
                events.push(Event::SubDamage {
                    side: foe as u8 + 1,
                    amount,
                });
                if self.sides[foe].mon().sub_hp == 0 {
                    events.push(Event::SubBroke {
                        side: foe as u8 + 1,
                    });
                }
                return;
            }
            let cap = if target.enduring {
                target.hp.saturating_sub(1)
            } else {
                target.hp
            };
            let amount = amount.min(cap);
            target.hp -= amount;
            self.taken_special[foe] = amount;
            events.push(Event::Damage {
                side: foe as u8 + 1,
                amount,
                effectiveness: 100,
                crit: false,
            });
            self.sides[foe].mon_mut().last_hit_by = Some(slot.entry.id);
self.sides[foe].mon_mut().last_hit_by_slot = Some(self.sides[side].active);
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
                // Nothing recorded against the target: with nothing to
                // bounce back the sim fails this in the move's own `onTry`,
                // which runs before the hit step ever writes the
                // attacked-by book. A Mirror Move aimed back finds nothing.
                events.push(Event::Failed {
                    side: side as u8 + 1,
                });
                return;
            }
            let eff = crate::types::effectiveness_against(
                slot.move_type(),
                self.sides[foe].mon().types(),
            );
            if eff == 0 {
                events.push(Event::Damage {
                    side: foe as u8 + 1,
                    amount: 0,
                    effectiveness: 0,
                    crit: false,
                });
                return;
            }
            let amount = taken.saturating_mul(2);
            let target = self.sides[foe].mon_mut();
            if target.sub_hp > 0 {
                let amount = amount.min(target.sub_hp);
                target.sub_hp -= amount;
                events.push(Event::SubDamage {
                    side: foe as u8 + 1,
                    amount,
                });
                if self.sides[foe].mon().sub_hp == 0 {
                    events.push(Event::SubBroke {
                        side: foe as u8 + 1,
                    });
                }
                return;
            }
            let cap = if target.enduring {
                target.hp.saturating_sub(1)
            } else {
                target.hp
            };
            let amount = amount.min(cap);
            target.hp -= amount;
            match crate::types::category_of(slot.move_type()) {
                crate::types::Category::Physical => self.taken_physical[foe] = amount,
                _ => self.taken_special[foe] = amount,
            }
            events.push(Event::Damage {
                side: foe as u8 + 1,
                amount,
                effectiveness: 100,
                crit: false,
            });
            self.sides[foe].mon_mut().last_hit_by = Some(slot.entry.id);
self.sides[foe].mon_mut().last_hit_by_slot = Some(self.sides[side].active);
            self.on_damaged(side, foe, &slot, slot.move_type(), script, events);
            self.shell_bell(side, amount, events);
            self.resolve_faints(side, foe, events);
            return;
        }

        // Future Sight and Doom Desire: aim a delayed hit two turns out.
        // The DAMAGE is computed now, at launch — typeless (no STAB, no
        // chart), never a crit, the launch turn's roll, today's stats and
        // screens — and stored; only accuracy waits for the landing.
        if matches!(slot.entry.id, "futuresight" | "doomdesire") {
            if self.sides[foe].incoming.is_some() {
                events.push(Event::Failed {
                    side: side as u8 + 1,
                });
                return;
            }
            let (mut attacker, mut defender) = self.attack_pair(side);
            let power = if slot.entry.id == "doomdesire" {
                120
            } else {
                // Future Sight is special: route the special stats through
                // the physical slots, since a typeless move reads those.
                attacker.atk = attacker.sp_atk;
                attacker.atk_stage = attacker.sp_atk_stage;
                attacker.burned = false;
                defender.def = defender.sp_def;
                defender.def_stage = defender.sp_def_stage;
                defender.reflect = defender.light_screen;
                80
            };
            let random = match script {
                Some(s) => s.random,
                None => 85 + self.rng.below(16) as u8,
            };
            let m = MoveUse {
                move_type: Type::None,
                power,
                halve_def: false,
                late_mult: 1,
                special: false,
                weather: 0,
                phase1: ability::Chain::new(),
            };
            let dealt = damage(
                &attacker,
                &defender,
                &m,
                Roll {
                    crit: false,
                    random,
                },
            ) as u16;
            self.sides[foe].incoming = Some((3, dealt, slot.entry.id));
            events.push(Event::Charging {
                side: side as u8 + 1,
            });
            return;
        }

        // Mimic and Sketch write the foe's last move into the slot — after
        // the dodge and protect gates above, where the sim's copy happens.
        if matches!(slot.entry.id, "mimic" | "sketch") {
            // These aim at the foe and so go in its attacked-by book, the
            // same as any other move that got this far — which matters
            // because both sit in Mirror Move's noMirror list, and a Sketch
            // landing after a Crabhammer is what makes the reply fail.
            if hit && self.sides[foe].mon().sub_hp == 0 {
                self.sides[foe].mon_mut().last_hit_by = Some(slot.entry.id);
self.sides[foe].mon_mut().last_hit_by_slot = Some(self.sides[side].active);
            }
            // The foe's last move BY ID — a Transform that rewrote its
            // slots doesn't change what it last used.
            let foe_last = self.sides[foe]
                .mon()
                .last_used_id
                .filter(|&i| i != "struggle")
                .and_then(crate::data::move_by_id);
            match (slot.entry.id, foe_last) {
                (_, None) => {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                // The user must not be transformed, must not already know
                // the move, and Sketch refuses the nosketch set (itself and
                // Struggle) just as Mimic refuses its own failmimic set.
                (_, Some(e))
                    if e.id == slot.entry.id
                        || self.sides[side].mon().transform_backup.is_some()
                        || self.sides[side].mon().moves.iter().any(|m| m.entry.id == e.id)
                        || (slot.entry.id == "sketch"
                            && matches!(e.id, "sketch" | "struggle")) =>
                {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                ("mimic", Some(e)) => {
                    // A five-PP overlay; the original slot returns when the
                    // mon leaves the field or faints. A substitute blocks
                    // the copy outright (the sim's, flags notwithstanding),
                    // a TRANSFORMED user cannot Mimic at all, and the
                    // failmimic set (Mimic, Metronome, Sketch, Struggle)
                    // refuses to be copied.
                    if self.sides[foe].mon().sub_hp > 0
                        || self.sides[side].mon().transform_backup.is_some()
                        || matches!(e.id, "mimic" | "metronome" | "sketch" | "struggle")
                    {
                        events.push(Event::Failed {
                            side: side as u8 + 1,
                        });
                        return;
                    }
                    let orig = self.sides[side].mon().moves[index];
                    let mon = self.sides[side].mon_mut();
                    mon.mimic_backup = Some((index as u8, orig));
                    mon.moves[index] = MoveSlot {
                        entry: e,
                        pp: 5,
                        typed_as: None,
                    };
                    return;
                }
                ("sketch", Some(e)) => {
                    self.sides[side].mon_mut().moves[index] = MoveSlot {
                        entry: e,
                        pp: e.pp,
                        typed_as: None,
                    };
                    return;
                }
                _ => unreachable!(),
            }
        }

        // Snore only works out of a snore-filled sleep — checked HERE, after
        // the call substitution, because Assist and Metronome can hand it to
        // a wide-awake mon and the gate has to see the move that will
        // actually go off.
        if slot.entry.id == "snore" && !asleep_now {
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }

        // A zero-power move is its status action, nothing more. A
        // foe-aimed one that gets this far still goes in the target's
        // attacked-by book (the sim records at the hit loop, whether or
        // not the effect then succeeds) — that is what Mirror Move reads.
        if slot.entry.power == 0 {
            // …except one the target is outright IMMUNE to (Leech Seed on
            // Grass, a chart-zero Thunder Wave or Glare): the sim filters
            // those at the type-immunity step, before the book is written.
            let immune = match slot.entry.id {
                "leechseed" => {
                    let (t1, t2) = self.sides[foe].mon().types();
                    t1 == Type::Grass || t2 == Type::Grass
                }
                "thunderwave" | "glare" => {
                    crate::types::effectiveness_against(
                        slot.move_type(),
                        self.sides[foe].mon().types(),
                    ) == 0
                }
                _ => false,
            };
            // ...and a handful of self-aimed moves ride the NoopFail action
            // without being in the self-target list: they never touch the foe.
            let self_aimed = self_targeted
                || matches!(
                    slot.entry.id,
                    "batonpass" | "assist" | "sleeptalk" | "recycle"
                );
            // The try-hit abilities answer a status move too: Growl is a
            // sound move whatever its power, and Will-O-Wisp is a Fire one.
            if !self_aimed {
                match ability::absorbs(
                    &self.sides[foe].mon().bearer(),
                    slot.entry.id,
                    slot.move_type(),
                    slot.entry.sound,
                ) {
                    ability::Absorb::None => {}
                    ability::Absorb::FlashFire => {
                        self.sides[foe].mon_mut().flash_fire = true;
                        return;
                    }
                    ability::Absorb::Drain => {
                        let mon = self.sides[foe].mon_mut();
                        let amount = (mon.max_hp / 4).max(1).min(mon.max_hp - mon.hp);
                        if amount > 0 {
                            mon.hp += amount;
                            events.push(Event::Healed {
                                side: foe as u8 + 1,
                                amount,
                            });
                        }
                        return;
                    }
                    ability::Absorb::Immune => {
                        events.push(Event::Damage {
                            side: foe as u8 + 1,
                            amount: 0,
                            effectiveness: 0,
                            crit: false,
                        });
                        return;
                    }
                }
            }
            if !self_aimed && hit && !immune && self.sides[foe].mon().sub_hp == 0 {
                self.sides[foe].mon_mut().last_hit_by = Some(slot.entry.id);
self.sides[foe].mon_mut().last_hit_by_slot = Some(self.sides[side].active);
            }
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
                match self.effective_weather() {
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
            let foe_b = self.sides[foe].mon().bearer();
            // Levitate is part of the immunity step itself in the sim, not a
            // try-hit handler: it makes the mon ungrounded, and Ground has
            // nothing to stand on.
            let chart_immune = crate::types::effectiveness_against(move_type, dtypes) == 0
                || ability::immune_to_type(&foe_b, move_type);
            // Then the try-hit abilities, which gen 3 runs AFTER the chart
            // rather than before it. Wonder Guard is asked here because this
            // is the only place that knows what the chart said.
            let effective = crate::types::effectiveness_against(move_type, dtypes) > 100;
            let absorb = if chart_immune {
                ability::Absorb::None
            } else if foe_b.ability == "wonderguard" && !effective && move_type != Type::None {
                ability::Absorb::Immune
            } else {
                ability::absorbs(&foe_b, slot.entry.id, move_type, slot.entry.sound)
            };
            match absorb {
                ability::Absorb::Drain => {
                    let mon = self.sides[foe].mon_mut();
                    let amount = (mon.max_hp / 4).max(1).min(mon.max_hp - mon.hp);
                    if amount > 0 {
                        mon.hp += amount;
                        events.push(Event::Healed {
                            side: foe as u8 + 1,
                            amount,
                        });
                    } else {
                        events.push(Event::Damage {
                            side: foe as u8 + 1,
                            amount: 0,
                            effectiveness: 0,
                            crit: false,
                        });
                    }
                }
                ability::Absorb::FlashFire => {
                    self.sides[foe].mon_mut().flash_fire = true;
                }
                ability::Absorb::Immune => {
                    events.push(Event::Damage {
                        side: foe as u8 + 1,
                        amount: 0,
                        effectiveness: 0,
                        crit: false,
                    });
                }
                ability::Absorb::None => {}
            }
            if chart_immune || absorb != ability::Absorb::None {
                if chart_immune {
                    events.push(Event::Damage {
                        side: foe as u8 + 1,
                        amount: 0,
                        effectiveness: 0,
                        crit: false,
                    });
                }
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

        // Uproar's din wakes every active sleeper — the gen 3 sim cures in
        // moveHit's TryHit, which only runs once the move got PAST immunity
        // and the accuracy roll: an immune target or a miss wakes nobody.
        if slot.entry.id == "uproar" {
            for w in 0..2 {
                if self.sides[w].mon().status == Some(Status::Sleep) {
                    let mon = self.sides[w].mon_mut();
                    mon.status = None;
                    mon.sleep_n = 0;
                    mon.nightmared = false;
                }
            }
        }

        let (crit, random) = match script {
            Some(s) => (s.crit, s.random),
            None => (
                self.rng.below(crit_denominator(
                    slot.entry.high_crit as u8
                        + if self.sides[side].mon().focused { 2 } else { 0 }
                        + item::crit_stages(&self.sides[side].mon().holder()),
                )) == 0,
                85 + self.rng.below(16) as u8,
            ),
        };
        // Battle Armor and Shell Armor refuse the critical hit outright,
        // however it was rolled.
        let crit = crit && !ability::blocks_crit(&self.sides[foe].mon().bearer());
        // How many times this move strikes. The 2-5 spread is the games'
        // weighted table (2 and 3 hits three-eighths each, 4 and 5 an eighth
        // each); a script pins the count for the tests.
        // Beat Up strikes once per rallied ally.
        let beatup_allies = if slot.entry.id == "beatup" {
            self.beatup_allies(side)
        } else {
            Vec::new()
        };
        let hits = if slot.entry.id == "beatup" {
            beatup_allies.len() as u16
        } else if slot.entry.id == "triplekick" {
            // Each kick re-rolls accuracy in the sim; under a script the
            // follow-up rolls read the secondary knob — false stops after
            // the first kick, true lands all three.
            match script {
                Some(s) => {
                    if s.secondary {
                        3
                    } else {
                        1
                    }
                }
                None => 3,
            }
        } else {
            match slot.entry.multihit {
                None => 1,
                Some((lo, hi)) if lo == hi => lo,
                Some(_) => match script {
                    // An unset (zero) hits knob means the table minimum, 2 —
                    // the same reading the reference harness uses.
                    Some(s) => {
                        if s.hits > 0 {
                            s.hits as u16
                        } else {
                            2
                        }
                    }
                    None => [2u16, 2, 2, 3, 3, 3, 4, 5][self.rng.below(8) as usize],
                },
            }
        };

        let mut total = 0u16;
        for hit_i in 0..hits {
            let (mut attacker, mut defender) = self.attack_pair(side);
            // Beat Up strikes with each ally's BASE Attack against the
            // target's BASE Defence — no stages, no burn — as a typeless
            // SPECIAL hit (Light Screen counts, and a zero base stays zero).
            if slot.entry.id == "beatup" {
                // Each strike swings the NEXT ally's base Attack; the sim
                // shifts them off `move.allies` one per hit.
                attacker.sp_atk = beatup_allies[hit_i as usize % beatup_allies.len()];
                attacker.sp_atk_stage = 0;
                defender.sp_def = self.sides[foe].mon().species.base.def as u16;
                defender.sp_def_stage = 0;
            }
            // Weather Ball wears the sky: retyped and doubled under weather.
            let move_type = if slot.entry.id == "weatherball" {
                match self.effective_weather() {
                    Some(Weather::Sun) => Type::Fire,
                    Some(Weather::Rain) => Type::Water,
                    Some(Weather::Sandstorm) => Type::Rock,
                    Some(Weather::Hail) => Type::Ice,
                    None => Type::Normal,
                }
            } else if slot.entry.id == "beatup" {
                Type::None // typeless: no STAB, no effectiveness
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
            let mut weather_mod = match (self.effective_weather(), move_type) {
                (Some(Weather::Rain), Type::Water) | (Some(Weather::Sun), Type::Fire) => 1,
                (Some(Weather::Rain), Type::Fire) | (Some(Weather::Sun), Type::Water) => -1,
                _ => 0,
            };
            // Mud/Water Sport hum from EITHER active halves the matching
            // type at BASE POWER (the sim's onBasePower chain), not at the
            // damage stage — the floor lands one point differently.
            let sport_div: u16 = if (0..2).any(|w| self.sides[w].mon().sport == Some(move_type)) {
                2
            } else {
                1
            };
            // The stomping moves land doubled on a minimized target.
            let stomp_mult: u16 = if self.sides[foe].mon().minimized
                && matches!(
                    slot.entry.id,
                    "stomp" | "extrasensory" | "needlearm" | "astonish"
                ) {
                2
            } else {
                1
            };
            // Solar Beam sputters outside the sun it was made for.
            let solar_cut = slot.entry.id == "solarbeam"
                && matches!(
                    self.effective_weather(),
                    Some(Weather::Rain | Weather::Sandstorm | Weather::Hail)
                );
            // Conditional powers the era defines by id.
            let base_power = match slot.entry.id {
                "triplekick" => 10 * (hit_i + 1),
                "present" => 120,
                "beatup" => 10,
                "pursuit" if self.pursuing => slot.entry.power * 2,
                "lowkick" => {
                    // Weight tiers, in hectograms.
                    let w = self.sides[foe].mon().species.weight_hg;
                    match w {
                        0..=99 => 20,
                        100..=249 => 40,
                        250..=499 => 60,
                        500..=999 => 80,
                        1000..=1999 => 100,
                        _ => 120,
                    }
                }
                "return" => 102, // the sim's default full happiness
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
                "smellingsalts" if self.sides[foe].mon().status == Some(Status::Paralysis) => {
                    slot.entry.power * 2
                }
                "revenge" if self.taken_physical[side] > 0 || self.taken_special[side] > 0 => {
                    slot.entry.power * 2
                }
                "weatherball" if self.effective_weather().is_some() => 100,
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
            let charge_mult: u16 =
                if move_type == Type::Electric && self.sides[side].mon().charged_elec {
                    self.sides[side].mon_mut().charged_elec = false;
                    2
                } else {
                    1
                };
            // Every base-power modifier goes through ONE chain and is
            // applied once, in the sim's handler order: Charge's doubling,
            // then a Sport's hum, then a pinch ability, then Thick Fat, then
            // Solar Beam's weather sulk. Halving and then boosting in turn
            // is not the same arithmetic as doing both at once, and the
            // difference shows up as a point of damage.
            let user_b = self.sides[side].mon().bearer();
            let foe_b = self.sides[foe].mon().bearer();
            let mut bp = ability::Chain::new();
            if charge_mult == 2 {
                bp.mul(ability::X2);
            }
            if sport_div == 2 {
                bp.mul(ability::X0_5);
            }
            if ability::pinch_boost(&user_b, move_type) {
                bp.mul(ability::X1_5);
            }
            if ability::thick_fat_cut(&foe_b, move_type) {
                bp.mul(ability::X0_5);
            }
            if solar_cut {
                bp.mul(ability::X0_5);
            }
            let power = bp
                .apply((base_power * pierce_power_mult) as u32)
                .max(1)
                .min(u16::MAX as u32) as u16;
            // The stats, once their stages are in. Gen 3 reads the category
            // off the move's type, so that is what decides whether an
            // Attack ability speaks at all.
            let physical = slot.entry.id != "beatup" && ability::physical_category(move_type);
            let user_i = self.sides[side].mon().holder();
            let foe_i = self.sides[foe].mon().holder();
            attacker.stat_mod = ability::attack_chain(&user_b, physical);
            attacker.stat_mod
                .extend(item::attack_chain(&user_i, move_type, physical));
            attacker.ignores_burn = ability::ignores_burn_drop(&user_b);
            defender.stat_mod = ability::defence_chain(&foe_b, physical);
            defender.stat_mod.extend(item::defence_chain(&foe_i, physical));
            let mut phase1 = ability::Chain::new();
            if self.sides[side].mon().flash_fire && move_type == Type::Fire {
                phase1.mul(ability::X1_5);
            }
            let m = MoveUse {
                move_type,
                power,
                halve_def: slot.entry.selfdestruct,
                weather: weather_mod,
                // Pierce and stomp double at the sim's ModifyDamage stage,
                // just before the roll — not on base power.
                late_mult: pierce_mult * stomp_mult,
                special: slot.entry.id == "beatup",
                phase1,
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
                events.push(Event::SubDamage {
                    side: foe as u8 + 1,
                    amount,
                });
                if self.sides[foe].mon().sub_hp == 0 {
                    events.push(Event::SubBroke {
                        side: foe as u8 + 1,
                    });
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
                // What just hit this mon (Mirror Move's playback source).
                self.sides[foe].mon_mut().last_hit_by = Some(slot.entry.id);
self.sides[foe].mon_mut().last_hit_by_slot = Some(self.sides[side].active);
                // A biding target banks what it just took.
                if let Some((stored, left)) = self.sides[foe].mon().bide {
                    self.sides[foe].mon_mut().bide = Some((stored.saturating_add(amount), left));
                }
                // A raging target's Attack climbs with every hit it takes.
                if self.sides[foe].mon().raging && !self.sides[foe].mon().fainted() {
                    self.sides[foe].mon_mut().apply_boost(Boost::Atk, 1);
                    events.push(Event::Boosted {
                        side: foe as u8 + 1,
                        boost: Boost::Atk,
                        delta: 1,
                    });
                }
                // Rage's own rage state begins only once it actually lands.
                if slot.entry.id == "rage" {
                    self.sides[side].mon_mut().raging = true;
                }
                self.resolve_faints(side, foe, events);
            }
            // Drain heals off the damage actually dealt: floor, but at
            // least 1 — EXCEPT off a substitute, where the sim's sub hook
            // heals with a CEILING instead.
            if let Some((num, den)) = slot.entry.drain {
                let heal = if hit_sub {
                    ((amount * num + den - 1) / den).max(1)
                } else {
                    (amount * num / den).max(1)
                };
                // Liquid Ooze turns the sip into a swig of poison: the same
                // number, taken off the drainer instead of given to it.
                if ability::ooze_reverses_drain(&self.sides[foe].mon().bearer(), slot.entry.id) {
                    let user = self.sides[side].mon_mut();
                    let hurt = heal.min(user.hp);
                    user.hp -= hurt;
                    events.push(Event::Recoil {
                        side: side as u8 + 1,
                        amount: hurt,
                    });
                    self.resolve_faints(side, foe, events);
                    continue;
                }
                let user = self.sides[side].mon_mut();
                let heal = heal.min(user.max_hp - user.hp);
                if heal > 0 {
                    user.hp += heal;
                    events.push(Event::Drained {
                        side: side as u8 + 1,
                        amount: heal,
                    });
                }
            }
            if !hit_sub {
                // The move's own secondary lands FIRST, and only then does
                // the target answer having been hit. Colour Change turning a
                // mon Fire the instant a Blaze Kick lands would otherwise
                // make it immune to the burn that same Blaze Kick is about
                // to inflict.
                self.hit_effects(side, foe, &slot, script, events);
                self.on_damaged(side, foe, &slot, move_type, script, events);
                self.resolve_faints(side, foe, events);
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
        // An Uproar counts its turns down as it goes; the Thrash family's
        // lock is spent in the residual phase instead, so that a swing lost
        // to sleep or a flinch still runs the clock out on schedule.
        if slot.entry.id == "uproar" {
            if let Some((slot_i, left)) = self.sides[side].mon().rampage {
                let mon = self.sides[side].mon_mut();
                if left == 0 {
                    mon.rampage = None;
                    mon.uproar_ending = true;
                } else {
                    mon.rampage = Some((slot_i, left - 1));
                }
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
            mon.rolling_fresh = true;
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
            // It sweeps the floor as well as the user: the sim's onHit runs
            // removeSideCondition('spikes') before it touches the volatiles,
            // so every later switch-in on this side walks in clean.
            self.sides[side].spikes = 0;
        }

        // Superpower and kin always pay their stat bill on a landed hit.
        if let Some(list) = slot.entry.self_drop {
            if !self.sides[side].mon().fainted() {
                for &(boost, delta) in list {
                    self.sides[side].mon_mut().apply_boost(boost, delta);
                    events.push(Event::Boosted {
                        side: side as u8 + 1,
                        boost,
                        delta,
                    });
                }
            }
        }

        // Recoil comes off the damage actually dealt: floored (this era's
        // rule; the fuzzer rejected round-to-nearest), but at least 1 — and
        // it can knock the user out.
        if let Some((num, den)) = slot.entry.recoil
            .filter(|_| !ability::ignores_recoil(&self.sides[side].mon().bearer(), slot.entry.id))
        {
            let hurt = (total * num / den).max(1);
            let user = self.sides[side].mon_mut();
            let hurt = hurt.min(user.hp);
            user.hp -= hurt;
            events.push(Event::Recoil {
                side: side as u8 + 1,
                amount: hurt,
            });
        }

        // Thief and Covet pocket what the target was holding, but only if
        // the thief's own hands are empty; Knock Off simply strikes it away.
        // Sticky Hold keeps hold of it either way, and in this era Knock Off
        // gets no extra power for the trouble.
        if total > 0 && !self.sides[foe].mon().fainted() && !self.sides[side].mon().fainted() {
            let theirs = self.sides[foe].mon().item;
            let held = self.sides[foe].mon().ability == "stickyhold";
            match slot.entry.id {
                "thief" | "covet"
                    if self.sides[side].mon().item.is_empty() && !theirs.is_empty() && !held =>
                {
                    self.sides[foe].mon_mut().item = "";
                    self.sides[side].mon_mut().item = theirs;
                }
                "knockoff" if !theirs.is_empty() && !held => {
                    self.sides[foe].mon_mut().item = "";
                }
                _ => {}
            }
        }
        self.shell_bell(side, total, events);


        // A landed Hyper Beam costs the next action.
        if slot.entry.recharge {
            self.sides[side].mon_mut().must_recharge = true;
        }

        self.resolve_faints(side, foe, events);
    }



    /// Announce and replace one side's active if it just fainted.
    fn announce_faint(&mut self, side: usize, events: &mut Vec<Event>) {
        if self.sides[side].mon().fainted() {
            if let Some((i, orig)) = self.sides[side].mon_mut().mimic_backup.take() {
                self.sides[side].mon_mut().moves[i as usize] = orig;
            }
            if let Some(orig) = self.sides[side].mon_mut().transform_backup.take() {
                self.sides[side].mon_mut().moves = orig;
            }
            if let Some((stats, types)) = self.sides[side].mon_mut().transform_stats.take() {
                let mon = self.sides[side].mon_mut();
                mon.atk = stats[0];
                mon.def = stats[1];
                mon.spa = stats[2];
                mon.spd = stats[3];
                mon.spe = stats[4];
                mon.type_override = types;
            }
            // A fainted trapper/gazer releases its victim.
            self.sides[1 - side].mon_mut().trapped_n = 0;
            self.sides[1 - side].mon_mut().mean_looked = false;
            events.push(Event::Fainted {
                side: side as u8 + 1,
            });
        }
    }


    /// Everything the sim's `clearVolatile` drops when a mon leaves the
    /// field, applied to `side`'s current active. Shared by the voluntary
    /// switch and by Roar and Whirlwind dragging one off.
    fn switch_out_reset(&mut self, side: usize) {
            // The trapper/gazer leaving the field releases its
            // victim; a sport leaves with its hummer (handled by
            // the outgoing mon's own field reset below).
            self.sides[1 - side].mon_mut().trapped_n = 0;
            self.sides[1 - side].mon_mut().mean_looked = false;
            let out = self.sides[side].mon_mut();
            if ability::cures_on_switch_out(&out.bearer()) && !out.fainted() {
                out.status = None;
                out.sleep_n = 0;
            }
            out.flash_fire = false;
            out.choice_locked = None;
            out.active_turns = 0;
            out.toxic_n = 0;
            out.confusion_n = 0;
            out.identified = false;
            out.sure_hit = 0;
            out.charged_elec = false;
            out.grudged = false;
            out.tormented = false;
            out.torment_fresh = false;
            out.raging = false;
            out.fury_n = 0;
            out.last_used = None;
            out.last_used_id = None;
            out.last_hit_by = None;
            out.last_hit_by_slot = None;
            out.last_missed = false;
            if let Some((i, orig)) = out.mimic_backup.take() {
                out.moves[i as usize] = orig;
            }
            if let Some(orig) = out.transform_backup.take() {
                out.moves = orig;
            }
            if let Some((stats, types)) = out.transform_stats.take() {
                out.atk = stats[0];
                out.def = stats[1];
                out.spa = stats[2];
                out.spd = stats[3];
                out.spe = stats[4];
                out.type_override = types;
            }
            out.bide = None;
            out.rolling = None;
            out.curled = false;
            out.encore_n = 0;
            out.disabled_slot = None;
            out.disable_n = 0;
            out.disable_fresh = false;
            out.disable_skip_tick = false;
            out.imprisoning = false;
            out.imprison_fresh = false;
            out.type_override = None;
            out.cursed = false;
            out.ingrained = false;
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
            // The sim's clearVolatile also wipes every stat stage and
            // the accuracy/evasion pair, drops any lock the mon was
            // under, and zeroes its action count — which is why Fake
            // Out works again on a mon that left and came back.
            out.stages = Default::default();
            out.acc_stage = 0;
            out.eva_stage = 0;
            out.flinched = false;
            out.focusing = false;
            out.uproar_ending = false;
            out.locked_move = None;
            out.acted = false;
            out.sport = None;
            out.sub_hp = 0;
            out.focused = false;
            out.minimized = false;
            out.seeded = false;
            out.trapped_n = 0;
            out.charging = None;
            out.charge_fresh = false;
            out.charge_fresh = false;
            out.rolling_fresh = false;
            out.rampage = None;
            out.must_recharge = false;
    }

    /// Send in replacements for whoever is down. The sim asks for these only
    /// AFTER the residual phase (`fieldEvent("Residual")`, then
    /// `checkFainted`), so a mon that faints mid-turn stays in its slot for
    /// the rest of the turn: the other side's move finds no target, and the
    /// residuals tick against an empty slot rather than against the
    /// replacement. The loop repeats because Spikes can drop the incoming
    /// mon too, and the sim just asks again.
    fn replace_fainted(&mut self, events: &mut Vec<Event>) {
        // A decided battle asks for nothing: the sim's checkWin runs inside
        // faintMessages, ahead of checkFainted, so the loser's last mon is
        // simply left lying where it fell — status and all.
        if self.over() {
            return;
        }
        for side in 0..2 {
            if self.deferred_switch[side].is_some() {
                continue; // its own switch is still coming
            }
            let mut guard = 0;
            while self.sides[side].mon().fainted() && guard < 8 {
                guard += 1;
                let Some(next) = self.sides[side].first_healthy() else {
                    break;
                };
                self.sides[side].mon_mut().status = None;
                self.sides[side].reorder_for_switch(next);
                self.sides[side].active = next;
                events.push(Event::Switched {
                    side: side as u8 + 1,
                    party_index: next,
                });
                self.switch_in_greet(side, events);
            }
        }
    }

    /// What greets a mon as it comes in. Gen 3 hands back the sleep it spent
    /// on Snore or Sleep Talk right before retreating, so a sleeper can
    /// attack, switch out and come back no closer to waking. Then Spikes
    /// bite a grounded arrival: an eighth, a sixth, a quarter for one, two,
    /// three layers. Flying types float over.
    fn switch_in_greet(&mut self, side: usize, events: &mut Vec<Event>) {
        self.greet(side, true, events)
    }

    fn greet(&mut self, side: usize, tidy: bool, events: &mut Vec<Event>) {
        // Truant counts the turn it arrives on, unless the battle has not
        // started: the sim keys that off its own turn counter.
        let turn = self.turn;
        self.sides[side].mon_mut().loafing = turn > 0;
        // What this mon walks in WITH is what greets the field. Trace copies
        // at the same moment, but an ability handed over in this era is never
        // started — the sim gates that on gen > 3 — so a traced Intimidate
        // cows nobody and a traced Drizzle brings no rain.
        let own = self.sides[side].mon().ability;
        let foe_ability = self.sides[1 - side].mon().ability;
        if ability::traces(&self.sides[side].mon().bearer())
            && !self.sides[1 - side].mon().fainted()
        {
            self.sides[side].mon_mut().ability = foe_ability;
        }
        let greeter = ability::Bearer {
            ability: own,
            ..self.sides[side].mon().bearer()
        };
        // A weather ability lays its sky down on arrival. Gen 3 gives it no
        // clock at all: it holds until something else sets the weather.
        if let Some(sky) = ability::weather_on_entry(&greeter) {
            let weather = match sky {
                "rain" => Weather::Rain,
                "sun" => Weather::Sun,
                _ => Weather::Sandstorm,
            };
            if self.weather != Some(weather) {
                self.weather = Some(weather);
                self.weather_n = u8::MAX;
                events.push(Event::WeatherStarted { weather });
            }
        }
        // Intimidate cows whatever is standing across the field — but not
        // through a substitute, and in this era not at all if the only
        // target has one up.
        if ability::intimidates(&greeter)
            && !self.sides[1 - side].mon().fainted()
            && self.sides[1 - side].mon().sub_hp == 0
            && !ability::blocks_drop(&self.sides[1 - side].mon().bearer(), ability::Drop::Attack)
            && self.sides[1 - side].mist_n == 0
        {
            self.sides[1 - side].mon_mut().apply_boost(Boost::Atk, -1);
            events.push(Event::Boosted {
                side: (1 - side) as u8 + 1,
                boost: Boost::Atk,
                delta: -1,
            });
        }
        if tidy {
            self.ability_update(side);
        }
        let mon = self.sides[side].mon_mut();
        if mon.status == Some(Status::Sleep) {
            mon.sleep_n += mon.sleep_skipped;
        }
        mon.sleep_skipped = 0;
        let layers = self.sides[side].spikes;
        if layers == 0 {
            return;
        }
        let mon = self.sides[side].mon();
        let (t1, t2) = mon.types();
        // Spikes only bite what stands on the ground, and Levitate is the
        // other way to be off it.
        if t1 == Type::Flying
            || t2 == Type::Flying
            || self.sides[side].mon().ability == "levitate"
        {
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
        events.push(Event::SpikesDamage {
            side: side as u8 + 1,
            amount,
        });
        self.announce_faint(side, events);
    }

    /// Announce whoever is down — target first, then the user (recoil can
    /// faint it too). Replacements are NOT sent in here: the sim only asks
    /// for one at an action boundary, so the empty slot has to survive to
    /// the end of the current move.
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
            self.announce_faint(who, events);
        }
    }

    /// Land `status` on `foe`'s active mon if Gen 3 rules allow it, setting
    /// the clocks that come with it: Toxic restarts its count, sleep draws
    /// its duration (pinned to the reference sim's floor under a script).
    fn inflict(&mut self, foe: usize, status: Status, scripted: bool, events: &mut Vec<Event>) {
        let before = self.sides[foe].mon().status;
        self.inflict_inner(foe, status, scripted, events);
        // The sim calls update() after an action resolves, which is where
        // the curing berries and the refusing abilities do their tidying.
        // A Rawst eaten the instant the burn lands is a whole turn's chip
        // that never happens.
        self.ability_update(foe);
        // Synchronize passes what it just caught back across the field. It
        // fires on the status ACTUALLY taking hold, so a blocked or refused
        // one bounces nothing.
        if self.sides[foe].mon().status != before {
            if let Some(back) =
                ability::synchronizes(&self.sides[foe].mon().bearer(), status)
            {
                let source = 1 - foe;
                if !self.sides[source].mon().fainted() {
                    self.inflict_inner(source, back, scripted, events);
                    self.ability_update(source);
                }
            }
        }
    }

    fn inflict_inner(
        &mut self,
        foe: usize,
        status: Status,
        scripted: bool,
        events: &mut Vec<Event>,
    ) {
        // Safeguard shields the whole team from foe-inflicted statuses.
        if self.sides[foe].safeguard_n > 0 {
            return;
        }
        // No one sleeps through an Uproar.
        if status == Status::Sleep
            && (0..2).any(|w| {
                // NOT `uproar_ending`: the din's own residual sits at
                // subOrder 11 and Yawn's at 19, so by the time Yawn puts its
                // victim under, the Uproar has already ended and has nothing
                // left to say about it.
                self.sides[w].mon().rampage.is_some_and(|(i, _)| {
                        self.sides[w]
                            .mon()
                            .moves
                            .get(i as usize)
                            .is_some_and(|m| m.entry.id == "uproar")
                    })
                    || (self.sides[w].mon().rampage.is_some()
                        && self.sides[w].mon().locked_move == Some("uproar"))
            })
        {
            return;
        }
        if !self.sides[foe].mon().can_receive(status) {
            return;
        }
        // An ability that simply refuses this status.
        if ability::blocks_status(&self.sides[foe].mon().bearer(), status) {
            return;
        }
        // Bright sunlight prevents freezing outright (the sim's Sunny Day
        // onImmunity hook).
        if status == Status::Freeze && self.effective_weather() == Some(Weather::Sun) {
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
        target.sleep_skipped = 0;
        if status == Status::Sleep {
            // A fresh sleep shakes off a Nightmare.
            target.nightmared = false;
        }
        events.push(Event::Statused {
            side: foe as u8 + 1,
            status,
        });
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
        // Own Tempo simply refuses the volatile.
        if ability::blocks_confusion(&self.sides[foe].mon().bearer()) {
            return;
        }
        let n = if scripted {
            2
        } else {
            2 + self.rng.below(4) as u8
        };
        self.sides[foe].mon_mut().confusion_n = n;
        self.ability_update(foe);
        events.push(Event::ConfusionStarted {
            side: foe as u8 + 1,
        });
    }

    /// The confusion self-hit: 40 base power, typeless, physical, against
    /// the mon's own Defense — stages and burn apply, nothing else does.
    /// What the target's ability does about having just been hit. Color
    /// Change takes on the move's type; Rough Skin grazes whatever touched
    /// it; the status ones each get a third of a chance, Effect Spore a
    /// tenth. A scripted run pins those rolls off — they are not one of the
    /// scenario's knobs, and the reference harness leaves a denominator of
    /// three or ten alone.
    fn on_damaged(
        &mut self,
        side: usize,
        foe: usize,
        slot: &MoveSlot,
        move_type: Type,
        script: Option<SeatScript>,
        events: &mut Vec<Event>,
    ) {
        let hit_b = self.sides[foe].mon().bearer();
        if ability::color_change(&hit_b) && !self.sides[foe].mon().fainted() && move_type != Type::None
        {
            // Only a type the target does NOT already have takes hold: a
            // dual-type that already counts as this type keeps both halves.
            let (t1, t2) = self.sides[foe].mon().types();
            if t1 != move_type && t2 != move_type {
                self.sides[foe].mon_mut().type_override = Some((move_type, Type::None));
            }
        }
        if !slot.entry.contact {
            return;
        }
        if ability::rough_skin(&hit_b) {
            let attacker = self.sides[side].mon_mut();
            let graze = ((attacker.max_hp / 16).max(1)).min(attacker.hp);
            attacker.hp -= graze;
            events.push(Event::Recoil {
                side: side as u8 + 1,
                amount: graze,
            });
        }
        let (touch, odds) = ability::on_touch(&hit_b);
        let proc = match script {
            Some(_) => false,
            None => touch != ability::OnTouch::None && self.rng.below(odds) == 0,
        };
        if proc {
            match touch {
                ability::OnTouch::Status(st) => self.inflict(side, st, script.is_some(), events),
                // The sim samples sleep, paralysis and poison in that order.
                ability::OnTouch::Spore => {
                    let st = match self.rng.below(3) {
                        0 => Status::Sleep,
                        1 => Status::Paralysis,
                        _ => Status::Poison,
                    };
                    self.inflict(side, st, script.is_some(), events);
                }
                // Attract is not modelled yet, so Cute Charm has nothing to
                // do with its third of a chance.
                ability::OnTouch::Attract | ability::OnTouch::None => {}
            }
        }
    }

    /// Eat the held berry if the residual phase finds the holder low enough.
    fn ripen(&mut self, side: usize, events: &mut Vec<Event>) {
        if self.sides[side].mon().fainted() {
            return;
        }
        let ripe = item::ripens(&self.sides[side].mon().holder());
        if ripe == item::Ripe::None {
            return;
        }
        let eaten = self.sides[side].mon().item;
        self.sides[side].mon_mut().item = "";
        self.sides[side].mon_mut().last_item = eaten;
        match ripe {
            item::Ripe::Heal(flat) => {
                let mon = self.sides[side].mon_mut();
                let amount = flat.min(mon.max_hp - mon.hp);
                if amount > 0 {
                    mon.hp += amount;
                    events.push(Event::Healed {
                        side: side as u8 + 1,
                        amount,
                    });
                }
            }
            item::Ripe::HealEighth => {
                let mon = self.sides[side].mon_mut();
                let amount = ((mon.max_hp / 8).max(1)).min(mon.max_hp - mon.hp);
                if amount > 0 {
                    mon.hp += amount;
                    events.push(Event::Healed {
                        side: side as u8 + 1,
                        amount,
                    });
                }
            }
            item::Ripe::Boost(boost) => {
                self.sides[side].mon_mut().apply_boost(boost, 1);
                events.push(Event::Boosted {
                    side: side as u8 + 1,
                    boost,
                    delta: 1,
                });
            }
            // The sim samples the stats that are not already maxed, and a
            // pinned sample takes the first — which is Attack.
            item::Ripe::StarfBoost => {
                let order = [Boost::Atk, Boost::Def, Boost::SpAtk, Boost::SpDef, Boost::Spe];
                let pick = order.iter().copied().find(|b| {
                    let i = match b {
                        Boost::Atk => Stat::Atk,
                        Boost::Def => Stat::Def,
                        Boost::SpAtk => Stat::SpAtk,
                        Boost::SpDef => Stat::SpDef,
                        _ => Stat::Spe,
                    };
                    self.sides[side].mon().stage(i) < 6
                });
                if let Some(boost) = pick {
                    self.sides[side].mon_mut().apply_boost(boost, 2);
                    events.push(Event::Boosted {
                        side: side as u8 + 1,
                        boost,
                        delta: 2,
                    });
                }
            }
            item::Ripe::FocusEnergy => {
                self.sides[side].mon_mut().focused = true;
            }
            item::Ripe::None => {}
        }
    }

    /// Shell Bell hands back an eighth of everything the move dealt, rounded
    /// down, at the sim's after-secondary-self stage — which is after the
    /// recoil, so a full-health attacker still gets part of its kick back.
    fn shell_bell(&mut self, side: usize, dealt: u16, events: &mut Vec<Event>) {
        // Nothing heals a corpse: the sim's heal() refuses a target with no
        // HP left, and an Explosion that pays its user back to life keeps a
        // decided battle running.
        if dealt == 0
            || self.sides[side].mon().fainted()
            || !item::shell_bell(&self.sides[side].mon().holder())
        {
            return;
        }
        // The sim's heal() rounds a fraction of one UP: anything above zero
        // gives back at least a point, so a four-damage hit still pays one.
        let mon = self.sides[side].mon_mut();
        let amount = (dealt / 8).max(1).min(mon.max_hp - mon.hp);
        if amount > 0 {
            mon.hp += amount;
            events.push(Event::Healed {
                side: side as u8 + 1,
                amount,
            });
        }
    }

    fn confusion_self_hit(&mut self, side: usize, random: u8) -> u16 {
        let mon = self.sides[side].mon();
        // The sim runs this through its ordinary damage call, which means
        // the Attack and Defence abilities both speak — a confused Huge
        // Power mon hits itself twice as hard.
        let bearer = mon.bearer();
        let holder = mon.holder();
        // A typeless forty-power physical hit, which means the held items
        // speak as well as the abilities: a confused Choice Band swings half
        // again as hard at itself.
        let mut atk_chain = ability::attack_chain(&bearer, true);
        atk_chain.extend(item::attack_chain(&holder, Type::None, true));
        let mut def_chain = ability::defence_chain(&bearer, true);
        def_chain.extend(item::defence_chain(&holder, true));
        let atk = atk_chain
            .apply(crate::stats::apply_stage(mon.atk, mon.stages[Stat::Atk as usize]) as u32);
        let def = def_chain
            .apply(crate::stats::apply_stage(mon.def, mon.stages[Stat::Def as usize]) as u32)
            .max(1);
        let mut dmg = ((2 * mon.level as u32 / 5 + 2) * 40 * atk / def) / 50;
        if mon.burned() && !ability::ignores_burn_drop(&bearer) {
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
                    events.push(Event::Boosted {
                        side: side as u8 + 1,
                        boost,
                        delta,
                    });
                }
            }
            StatusAction::HealHalf => {
                let mon = self.sides[side].mon_mut();
                let heal = (mon.max_hp / 2).min(mon.max_hp - mon.hp);
                if heal > 0 {
                    mon.hp += heal;
                    events.push(Event::Healed {
                        side: side as u8 + 1,
                        amount: heal,
                    });
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
                    events.push(Event::Seeded {
                        side: foe as u8 + 1,
                    });
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
                        events.push(Event::Boosted {
                            side: foe as u8 + 1,
                            boost,
                            delta,
                        });
                    }
                    self.confuse(foe, scripted, events);
                }
            }
            StatusAction::Focus => {
                if !self.sides[side].mon().focused {
                    self.sides[side].mon_mut().focused = true;
                    events.push(Event::Focused {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Rest => {
                // Nothing sleeps through an Uproar, a self-inflicted sleep
                // included: the din answers the sim's SetStatus event
                // wherever the sleep came from.
                let uproar = (0..2).any(|w| {
                    let m = self.sides[w].mon();
                    m.uproar_ending
                        || (m.rampage.is_some() && m.locked_move == Some("uproar"))
                        || m.rampage.is_some_and(|(i, _)| {
                            m.moves.get(i as usize).is_some_and(|s| s.entry.id == "uproar")
                        })
                });
                let mon = self.sides[side].mon();
                if mon.hp < mon.max_hp && !uproar {
                    let mon = self.sides[side].mon_mut();
                    mon.hp = mon.max_hp;
                    mon.status = Some(Status::Sleep);
                    // The games' Rest sleeps two full turns: clock of 3,
                    // the same shape as the sim's pinned setStatus. A fresh
                    // sleep also shakes off a Nightmare.
                    mon.sleep_n = 3;
                    mon.sleep_skipped = 0;
                    mon.toxic_n = 0;
                    mon.nightmared = false;
                    events.push(Event::Rested {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Minimize => {
                self.sides[side].mon_mut().minimized = true;
                self.sides[side].mon_mut().apply_boost(Boost::Eva, 1);
                events.push(Event::Boosted {
                    side: side as u8 + 1,
                    boost: Boost::Eva,
                    delta: 1,
                });
            }
            StatusAction::WeatherHeal => {
                let mult = match self.effective_weather() {
                    None => (1, 2),
                    Some(Weather::Sun) => (2, 3),
                    Some(_) => (1, 4),
                };
                let mon = self.sides[side].mon_mut();
                let heal = ((mon.max_hp as u32 * mult.0 / mult.1) as u16).min(mon.max_hp - mon.hp);
                if heal > 0 {
                    mon.hp += heal;
                    events.push(Event::Healed {
                        side: side as u8 + 1,
                        amount: heal,
                    });
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
                    events.push(Event::Boosted {
                        side: side as u8 + 1,
                        boost: Boost::Atk,
                        delta: 6,
                    });
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
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
                    && !ability::blocks_yawn(&target.bearer())
                {
                    self.sides[foe].mon_mut().yawn_n = 2;
                    events.push(Event::Drowsy {
                        side: foe as u8 + 1,
                    });
                }
            }
            StatusAction::Wish => {
                if self.sides[side].wish_n == 0 {
                    self.sides[side].wish_n = 2;
                    self.sides[side].wish_amount = self.sides[side].mon().max_hp / 2;
                }
            }
            StatusAction::PerishSong => {
                // The song sweeps the field, but its onHitField loop still
                // runs the Invulnerability event per target: a mon that is
                // underground or in the air is missed and never picks up a
                // count, however loud the singing.
                for who in 0..2 {
                    if self.sides[who].mon().semi_invulnerable().is_some() {
                        continue;
                    }
                    // The song is a sound, and it sweeps the singer's own
                    // side too, so a Soundproof singer is deaf to it.
                    if self.sides[who].mon().ability == "soundproof" {
                        continue;
                    }
                    let mon = self.sides[who].mon_mut();
                    if mon.perish_n == 0 && !mon.fainted() {
                        mon.perish_n = 4;
                    }
                }
            }
            StatusAction::DestinyBond => {
                self.sides[side].mon_mut().destiny = true;
                events.push(Event::DestinyArmed {
                    side: side as u8 + 1,
                });
            }
            StatusAction::MeanLook => {
                let target = self.sides[foe].mon();
                if hit && target.sub_hp == 0 && !target.mean_looked && !target.fainted() {
                    self.sides[foe].mon_mut().mean_looked = true;
                    events.push(Event::NoEscape {
                        side: foe as u8 + 1,
                    });
                }
            }
            StatusAction::Sport(kind) => {
                self.sides[side].mon_mut().sport = Some(kind);
            }
            StatusAction::Spikes => {
                if self.sides[foe].spikes < 3 {
                    self.sides[foe].spikes += 1;
                    events.push(Event::SpikesLaid {
                        side: foe as u8 + 1,
                    });
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Memento => {
                if !hit || self.sides[foe].mon().sub_hp > 0 || self.sides[foe].mon().fainted() {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                let misted = self.sides[foe].mist_n > 0;
                if !misted {
                    for (boost, delta) in [(Boost::Atk, -2i8), (Boost::SpAtk, -2)] {
                        self.sides[foe].mon_mut().apply_boost(boost, delta);
                        events.push(Event::Boosted {
                            side: foe as u8 + 1,
                            boost,
                            delta,
                        });
                    }
                }
                self.sides[side].mon_mut().hp = 0;
                self.announce_faint(side, events);
            }
            StatusAction::PainSplit => {
                if !hit || self.sides[foe].mon().sub_hp > 0 || self.sides[foe].mon().fainted() {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                let avg = (self.sides[side].mon().hp as u32 + self.sides[foe].mon().hp as u32) / 2;
                for who in [side, foe] {
                    let mon = self.sides[who].mon_mut();
                    mon.hp = (avg as u16).min(mon.max_hp);
                }
                events.push(Event::Healed {
                    side: side as u8 + 1,
                    amount: 0,
                });
            }
            StatusAction::Protect | StatusAction::Endure => {
                // Nothing left to shield against: the sim refuses before it
                // ever reaches the stall gamble.
                if !self.will_act {
                    self.sides[side].mon_mut().stall_counter = 0;
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
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
                    mon.stall_counter = if counter == 0 {
                        2
                    } else {
                        (counter * 2).min(8)
                    };
                    events.push(Event::Protected {
                        side: side as u8 + 1,
                    });
                } else {
                    self.sides[side].mon_mut().stall_counter = 0;
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Identify => {
                if hit && self.sides[foe].mon().sub_hp == 0 && !self.sides[foe].mon().fainted() {
                    self.sides[foe].mon_mut().identified = true;
                }
            }
            StatusAction::LockOn => {
                // Taking aim at a mon already in your sights does nothing:
                // addVolatile refuses a lock that is still up, and the move
                // fails rather than refreshing it.
                if hit && !self.sides[foe].mon().fainted() && self.sides[side].mon().sure_hit == 0 {
                    let aim = self.sides[foe].active as u8;
                    let mon = self.sides[side].mon_mut();
                    mon.sure_hit = 2;
                    mon.sure_hit_on = aim;
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::ChargeUp => {
                self.sides[side].mon_mut().charged_elec = true;
            }
            // Handled by the call substitution above, before the status
            // dispatch is ever reached; these arms only exist so the match
            // stays exhaustive.
            StatusAction::SleepTalk | StatusAction::Assist => {}
            StatusAction::BatonPass => {
                // The user leaves and hands over its boosts and most of its
                // volatiles. The sim refuses the ones flagged noCopy —
                // Disable, Encore, Foresight, Nightmare, Stockpile, Imprison,
                // Minimize, Torment, Toxic's counter, Yawn, Destiny Bond,
                // Defense Curl — so the incoming mon starts clean of those.
                let incoming = (0..self.sides[side].party.len())
                    .find(|&i| i != self.sides[side].active && !self.sides[side].party[i].fainted());
                match incoming {
                    None => events.push(Event::Failed {
                        side: side as u8 + 1,
                    }),
                    Some(next) => {
                        let passed = {
                            let m = self.sides[side].mon();
                            (
                                m.stages,
                                m.acc_stage,
                                m.eva_stage,
                                m.sub_hp,
                                m.seeded,
                                m.confusion_n,
                                m.perish_n,
                                m.cursed,
                                m.ingrained,
                                m.focused,
                                m.mean_looked,
                                m.trapped_n,
                                m.charged_elec,
                                m.taunt_n,
                            )
                        };
                        self.switch_out_reset(side);
                        self.sides[side].reorder_for_switch(next);
                        self.sides[side].active = next;
                        {
                            let m = self.sides[side].mon_mut();
                            m.stages = passed.0;
                            m.acc_stage = passed.1;
                            m.eva_stage = passed.2;
                            m.sub_hp = passed.3;
                            m.seeded = passed.4;
                            m.confusion_n = passed.5;
                            m.perish_n = passed.6;
                            m.cursed = passed.7;
                            m.ingrained = passed.8;
                            m.focused = passed.9;
                            m.mean_looked = passed.10;
                            m.trapped_n = passed.11;
                            m.charged_elec = passed.12;
                            m.taunt_n = passed.13;
                        }
                        events.push(Event::Switched {
                            side: side as u8 + 1,
                            party_index: next,
                        });
                        self.switch_in_greet(side, events);
                    }
                }
            }
            StatusAction::ForceSwitch => {
                // Roar and Whirlwind drag a benched mon out of the other
                // side. The sim samples `possibleSwitches`, which is its own
                // `side.pokemon` order past the active slot; a pinned sample
                // lands on that list's first entry. With nobody to drag in,
                // or the target already down, the move simply fails.
                // Ingrain's roots hold against a drag as well as against a
                // voluntary switch — it is the one thing in this era with an
                // onDragOut — and the sim announces the refusal rather than
                // failing quietly. A bind or a Mean Look does NOT stop one:
                // those only forbid leaving of your own accord.
                let rooted = self.sides[foe].mon().ingrained
                    || ability::blocks_drag(&self.sides[foe].mon().bearer());
                let bench = self.sides[foe].draggable();
                match bench.first().copied() {
                    Some(next) if hit && !rooted && !self.sides[foe].mon().fainted() => {
                        self.switch_out_reset(foe);
                        self.sides[foe].reorder_for_switch(next);
                        self.sides[foe].active = next;
                        self.dragged[foe] = true;
                        events.push(Event::Switched {
                            side: foe as u8 + 1,
                            party_index: next,
                        });
                        self.switch_in_greet(foe, events);
                    }
                    _ => events.push(Event::Failed {
                        side: side as u8 + 1,
                    }),
                }
            }
            StatusAction::Spite => {
                // Spite reaches through a substitute in this era. It drains
                // the slot carrying the last-used move's ID — a Transform
                // that rewrote the slots leaves nothing to drain, and fails.
                let drained = if hit {
                    if let Some(id) = self.sides[foe]
                        .mon()
                        .last_used_id
                        .filter(|&i| i != "struggle")
                    {
                        let mon = self.sides[foe].mon_mut();
                        if let Some(ms) = mon.moves.iter_mut().find(|m| m.entry.id == id) {
                            if ms.pp > 0 {
                                // The games shave 2..5 PP; a script pins 2.
                                let cut = if scripted {
                                    2
                                } else {
                                    2 + self.rng.below(4) as u8
                                };
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
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
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
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Encore => {
                let target = self.sides[foe].mon();
                // The sim reads the move that was actually USED, then looks
                // its slot up by id: a `failencore` move is refused, and so
                // is one the target no longer carries — which is what makes
                // Encore fail on a mon that just Transformed, since the
                // transform rewrote every slot out from under it.
                let encorable = target.last_used_id.is_some_and(|id| {
                    !matches!(
                        id,
                        "encore" | "mimic" | "mirrormove" | "sketch" | "struggle" | "transform"
                    ) && target
                        .moves
                        .iter()
                        .any(|m| m.entry.id == id && m.pp > 0)
                });
                if hit && target.encore_n == 0 && encorable && !target.fainted() {
                    // The games run 3..6 encored turns; a script pins 3.
                    let n = if scripted {
                        3
                    } else {
                        3 + self.rng.below(4) as u8
                    };
                    self.sides[foe].mon_mut().encore_n = n;
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Disable => {
                // Disable pierces a substitute in this era, but fails when
                // the target's last move has no PP left to seal — and it
                // finds that move BY ID, so a Transform that rewrote the
                // slots leaves nothing to disable.
                let target = self.sides[foe].mon();
                let slot_by_id = target
                    .last_used_id
                    .filter(|&i| i != "struggle")
                    .and_then(|id| target.moves.iter().position(|m| m.entry.id == id));
                let last_has_pp = slot_by_id
                    .and_then(|i| target.moves.get(i))
                    .is_some_and(|m| m.pp > 0);
                if hit && target.disabled_slot.is_none() && last_has_pp && !target.fainted() {
                    // This era's clock is `random(2, 6)`, and it counts the
                    // victim's next N ACTIONS rather than N turns. Measured
                    // against the sim: a Disable from a FASTER mon blocks the
                    // victim's move that same turn and the next, and ends; a
                    // Disable from a slower one lands after the victim has
                    // moved and blocks the following two turns instead. (The
                    // modern four-to-seven belongs to a later generation.)
                    let slot_i = slot_by_id.map(|i| i as u8);
                    let n = if scripted {
                        2
                    } else {
                        2 + self.rng.below(4) as u8
                    };
                    let already_moved = self.acted_this_turn[foe];
                    let mon = self.sides[foe].mon_mut();
                    mon.disabled_slot = slot_i;
                    mon.disable_n = n;
                    mon.disable_fresh = true;
                    mon.disable_skip_tick = already_moved;
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Camouflage => {
                // Fails on anything already carrying Normal (either slot).
                let (t1, t2) = self.sides[side].mon().types();
                if t1 == Type::Normal || t2 == Type::Normal {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                self.sides[side].mon_mut().type_override = Some((Type::Normal, Type::None));
            }
            StatusAction::Conversion => {
                // Only a type the user does not already have is eligible;
                // with no eligible move type, Conversion fails.
                let cur = self.sides[side].mon().types();
                let ty = self.sides[side]
                    .mon()
                    .moves
                    .iter()
                    .map(|m| m.move_type())
                    .find(|&t| t != Type::None && t != cur.0 && t != cur.1);
                match ty {
                    Some(t) => {
                        self.sides[side].mon_mut().type_override = Some((t, Type::None));
                    }
                    None => events.push(Event::Failed {
                        side: side as u8 + 1,
                    }),
                }
            }
            StatusAction::Imprison => {
                // Fails unless the foe actually shares a move with the user
                // — the sim refuses a sealless Imprison outright.
                let shares = self.sides[side].mon().moves.iter().any(|m| {
                    self.sides[foe]
                        .mon()
                        .moves
                        .iter()
                        .any(|f| f.entry.id == m.entry.id)
                });
                if shares && !self.sides[side].mon().imprisoning {
                    self.sides[side].mon_mut().imprisoning = true;
                    self.sides[side].mon_mut().imprison_fresh = true;
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::MirrorMove | StatusAction::Mimic | StatusAction::Sketch => {
                // Handled before the status path; unreachable here.
            }
            StatusAction::Transform => {
                let foe_mon = self.sides[foe].mon().clone();
                // Copying a copy fails: a transformed target refuses it.
                if foe_mon.fainted() || foe_mon.transform_backup.is_some() {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                } else {
                    let mon = self.sides[side].mon_mut();
                    if mon.transform_backup.is_none() {
                        mon.transform_backup = Some(mon.moves.clone());
                        mon.transform_stats = Some((
                            [mon.atk, mon.def, mon.spa, mon.spd, mon.spe],
                            mon.type_override,
                        ));
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
                    // The copy is complete enough to include the ability,
                    // which is how a paralysed mon that copies a Limber one
                    // walks away cured.
                    mon.ability = foe_mon.ability;
                    mon.flash_fire = false;
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
                    self.ability_update(side);
                }
            }
            StatusAction::Trick => {
                // Sticky Hold refuses the trade outright, and two empty
                // hands have nothing to trade.
                let mine = self.sides[side].mon().item;
                let theirs = self.sides[foe].mon().item;
                if !hit
                    || self.sides[foe].mon().ability == "stickyhold"
                    || (mine.is_empty() && theirs.is_empty())
                    || self.sides[foe].mon().fainted()
                {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                self.sides[side].mon_mut().item = theirs;
                self.sides[foe].mon_mut().item = mine;
                // A Choice Band that changes hands lets go of both.
                self.sides[side].mon_mut().choice_locked = None;
                self.sides[foe].mon_mut().choice_locked = None;
                self.ability_update(side);
                self.ability_update(foe);
            }
            StatusAction::Recycle => {
                let back = self.sides[side].mon().last_item;
                if !self.sides[side].mon().item.is_empty() || back.is_empty() {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                let mon = self.sides[side].mon_mut();
                mon.item = back;
                mon.last_item = "";
                self.ability_update(side);
            }
            StatusAction::SkillSwap | StatusAction::RolePlay => {
                // Wonder Guard refuses to be traded or copied, and this era
                // also refuses a swap between two mons that already share an
                // ability. Role Play alone insists the two differ; Skill
                // Swap checks the same thing under gen 5 and below.
                let mine = self.sides[side].mon().ability;
                let theirs = self.sides[foe].mon().ability;
                let unswappable = mine == "wonderguard" || theirs == "wonderguard";
                if !hit
                    || unswappable
                    || mine == theirs
                    || self.sides[foe].mon().fainted()
                    || self.sides[side].mon().fainted()
                {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                let swapping = matches!(action, StatusAction::SkillSwap);
                self.sides[side].mon_mut().ability = theirs;
                if swapping {
                    self.sides[foe].mon_mut().ability = mine;
                }
                // Losing Flash Fire loses what it caught: the sim ends the
                // old ability, and ending Flash Fire drops its volatile.
                for w in [side, foe] {
                    if self.sides[w].mon().ability != "flashfire" {
                        self.sides[w].mon_mut().flash_fire = false;
                    }
                }
                // A traded-in ability tidies up straight away — a frozen mon
                // handed Magma Armor thaws on the spot.
                self.ability_update(side);
                self.ability_update(foe);
            }
            StatusAction::NoopFail => {
                events.push(Event::Failed {
                    side: side as u8 + 1,
                });
            }
            StatusAction::NoopSuccess => {}
            StatusAction::HealBell => {
                // The chime reaches the WHOLE party, bench included — the
                // sim walks `side.pokemon`, not just the active slot.
                // …but it never reaches a Soundproof one, on the field or
                // on the bench. The chime is a sound, and that ability is
                // deaf to it whoever rang it.
                let chimes = slot.entry.sound;
                for mon in self.sides[side].party.iter_mut() {
                    if chimes && mon.ability == "soundproof" {
                        continue;
                    }
                    mon.status = None;
                    mon.toxic_n = 0;
                    mon.sleep_n = 0;
                    mon.sleep_skipped = 0;
                    mon.nightmared = false;
                }
            }
            StatusAction::Ingrain => {
                if self.sides[side].mon().ingrained {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                } else {
                    self.sides[side].mon_mut().ingrained = true;
                }
            }
            StatusAction::Conversion2 => {
                // Retype to the FIRST type (the sim's pinned sample walks
                // the dex order) that resists the foe's last-used move.
                // The sim walks dex.types.names() — the typechart file's
                // key order — and the pinned sample takes the first.
                const DEX_ORDER: [Type; 17] = [
                    Type::Electric,
                    Type::Ghost,
                    Type::Grass,
                    Type::Steel,
                    Type::Dark,
                    Type::Bug,
                    Type::Dragon,
                    Type::Fighting,
                    Type::Fire,
                    Type::Flying,
                    Type::Ground,
                    Type::Ice,
                    Type::Normal,
                    Type::Poison,
                    Type::Psychic,
                    Type::Rock,
                    Type::Water,
                ];
                let last = self.sides[foe]
                    .mon()
                    .last_used_id
                    .and_then(crate::data::move_by_id)
                    .map(|m| m.move_type);
                match last {
                    Some(atk_type) if atk_type != Type::None => {
                        let pick = DEX_ORDER
                            .iter()
                            .copied()
                            .find(|&t| crate::types::effectiveness(atk_type, t) < 10);
                        match pick {
                            Some(t) => {
                                self.sides[side].mon_mut().type_override = Some((t, Type::None));
                            }
                            None => events.push(Event::Failed {
                                side: side as u8 + 1,
                            }),
                        }
                    }
                    _ => events.push(Event::Failed {
                        side: side as u8 + 1,
                    }),
                }
            }
            StatusAction::Curse => {
                let (t1, t2) = self.sides[side].mon().types();
                if t1 == Type::Ghost || t2 == Type::Ghost {
                    // The Ghost pays half its max HP to lay the curse.
                    if self.sides[foe].mon().cursed || self.sides[foe].mon().fainted() {
                        events.push(Event::Failed {
                            side: side as u8 + 1,
                        });
                    } else {
                        let cost = (self.sides[side].mon().max_hp / 2).max(1);
                        let user = self.sides[side].mon_mut();
                        let cost = cost.min(user.hp);
                        user.hp -= cost;
                        self.sides[foe].mon_mut().cursed = true;
                        self.announce_faint(side, events);
                    }
                } else {
                    let mon = self.sides[side].mon_mut();
                    mon.apply_boost(Boost::Atk, 1);
                    mon.apply_boost(Boost::Def, 1);
                    mon.apply_boost(Boost::Spe, -1);
                }
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
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Stockpile => {
                let mon = self.sides[side].mon_mut();
                if mon.stockpile_n < 3 {
                    mon.stockpile_n += 1;
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Swallow => {
                let n = self.sides[side].mon().stockpile_n;
                if n == 0 {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
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
                        events.push(Event::Healed {
                            side: side as u8 + 1,
                            amount: heal,
                        });
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
                    events.push(Event::SubStarted {
                        side: side as u8 + 1,
                    });
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
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
                    events.push(Event::SideStarted {
                        side: side as u8 + 1,
                        condition: cond,
                    });
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
                        // Clear Body and the single-stat guards refuse a
                        // drop that came from the other side.
                        if delta < 0
                            && ability::blocks_drop(
                                &self.sides[foe].mon().bearer(),
                                ability::drop_kind(boost),
                            )
                        {
                            continue;
                        }
                        self.sides[foe].mon_mut().apply_boost(boost, delta);
                        events.push(Event::Boosted {
                            side: foe as u8 + 1,
                            boost,
                            delta,
                        });
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
        // Shield Dust refuses every secondary a move turns on ITS TARGET.
        // The self-aimed drops (Overheat's own Special Attack) are a
        // different field entirely and are not touched.
        // Shield Dust filters the secondaries aimed at its bearer and keeps
        // the ones the move turns on the ATTACKER — Steel Wing still raises
        // its own Defence through it.
        let dusted = ability::blocks_secondary(&self.sides[foe].mon().bearer())
            && !matches!(
                slot.entry.secondary.map(|sec| sec.effect),
                Some(SecondaryEffect::SelfBoosts(_))
            );
        // Serene Grace doubles the printed chance before it is rolled.
        let doubled = ability::doubles_secondary(&self.sides[side].mon().bearer());
        let chance =
            |sec: crate::data::Secondary| (sec.chance as u32 * if doubled { 2 } else { 1 }).min(255);
        let certain = slot.entry.secondary.is_some_and(|sec| chance(sec) >= 100);
        let proc = !dusted
            && (certain
                || match script {
                    Some(s) => s.secondary,
                    None => slot
                        .entry
                        .secondary
                        .map(|sec| self.rng.below(100) < chance(sec))
                        .unwrap_or(false),
                });
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
                            if delta < 0
                                && ability::blocks_drop(
                                    &self.sides[foe].mon().bearer(),
                                    ability::drop_kind(boost),
                                )
                            {
                                continue;
                            }
                            self.sides[foe].mon_mut().apply_boost(boost, delta);
                            events.push(Event::Boosted {
                                side: foe as u8 + 1,
                                boost,
                                delta,
                            });
                        }
                    }
                }
                Some(SecondaryEffect::Flinch) => {
                    // A mon tightening its focus cannot be flinched at all —
                    // the volatile is refused, not merely out-prioritized.
                    if !self.sides[foe].mon().fainted()
                        && !self.sides[foe].mon().focusing
                        && !ability::blocks_flinch(&self.sides[foe].mon().bearer())
                    {
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
                            events.push(Event::Boosted {
                                side: side as u8 + 1,
                                boost,
                                delta,
                            });
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
            events.push(Event::Trapped {
                side: foe as u8 + 1,
            });
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
        let (mut attacker, mut defender) = self.attack_pair(side);
        let user_b = self.sides[side].mon().bearer();
        let foe_b = self.sides[foe].mon().bearer();
        let physical = ability::physical_category(slot.move_type());
        attacker.stat_mod = ability::attack_chain(&user_b, physical);
        attacker.ignores_burn = ability::ignores_burn_drop(&user_b);
        defender.stat_mod = ability::defence_chain(&foe_b, physical);
        let m = MoveUse {
            move_type: slot.move_type(),
            power: slot.entry.power,
            halve_def: false,
            late_mult: 1,
            special: false,
            weather: 0,
            phase1: ability::Chain::new(),
        };
        let would = damage(&attacker, &defender, &m, Roll { crit, random });
        // The sim clamps the crash into [1, target's max HP / 2].
        let cap = (self.sides[foe].mon().max_hp / 2).max(1);
        let crash = ((would / 2) as u16).max(1).min(cap);
        let user = self.sides[side].mon_mut();
        let crash = crash.min(user.hp);
        user.hp -= crash;
        events.push(Event::Recoil {
            side: side as u8 + 1,
            amount: crash,
        });
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
                stat_mod: ability::Chain::new(),
                ignores_burn: false,
            },
            Defender {
                def: d.def,
                sp_def: d.spd,
                def_stage: d.stage(Stat::Def),
                sp_def_stage: d.stage(Stat::SpDef),
                types: d.types(),
                reflect: self.sides[1 - side].reflect_n > 0,
                light_screen: self.sides[1 - side].light_screen_n > 0,
                stat_mod: ability::Chain::new(),
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
        let mut b = battle(
            mon("blaziken", 50, &["ember"]),
            mon("treecko", 50, &["pound"]),
        );
        let before = b.sides[1].mon().hp;
        let pp_before = b.sides[0].mon().moves[0].pp;
        let events = b.step([Choice::Move(0), Choice::Move(0)]);

        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Used { side: 1, .. })));
        assert!(
            b.sides[1].mon().hp < before,
            "treecko should have taken a hit"
        );
        assert_eq!(b.sides[0].mon().moves[0].pp, pp_before - 1);
    }

    #[test]
    fn the_faster_mon_moves_first() {
        // Base Speed 80 against 70, so Blaziken moves first. Asserted against
        // the stats rather than a memory of who is fast.
        let mut b = battle(
            mon("blaziken", 50, &["ember"]),
            mon("treecko", 50, &["pound"]),
        );
        let faster = if b.sides[0].mon().spe > b.sides[1].mon().spe {
            1
        } else {
            2
        };
        assert_ne!(
            b.sides[0].mon().spe,
            b.sides[1].mon().spe,
            "the tie-break is a different test"
        );
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
        let mut b = battle(
            mon("blaziken", 50, &["ember"]),
            mon("treecko", 50, &["pound"]),
        );
        let events = b.step([Choice::Move(0), Choice::Move(0)]);
        let eff = events
            .iter()
            .find_map(|e| match e {
                Event::Damage {
                    side: 2,
                    effectiveness,
                    ..
                } => Some(*effectiveness),
                _ => None,
            })
            .expect("treecko took damage");
        assert_eq!(eff, 200);
    }

    #[test]
    fn a_battle_ends_when_a_side_is_out() {
        // A level 100 attacker against a level 5 defender: one hit, one win.
        let mut b = battle(
            mon("blaziken", 100, &["ember"]),
            mon("treecko", 5, &["pound"]),
        );
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
        let side2 = Side::new(alloc::vec![
            mon("treecko", 5, &["pound"]),
            mon("mudkip", 50, &["pound"])
        ]);
        let mut b = Battle::new(side1, side2, 7);
        let events = b.step([Choice::Move(0), Choice::Move(0)]);
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Fainted { side: 2 })));
        assert_eq!(b.sides[1].active, 1, "the next mon steps in");
        assert!(!b.over(), "the side still has a mon");
    }

    #[test]
    fn switching_happens_before_anyone_attacks() {
        let side1 = Side::new(alloc::vec![
            mon("blaziken", 50, &["ember"]),
            mon("mudkip", 50, &["pound"])
        ]);
        let side2 = Side::new(alloc::vec![mon("treecko", 50, &["pound"])]);
        let mut b = Battle::new(side1, side2, 3);
        let events = b.step([Choice::Switch(1), Choice::Move(0)]);
        let switched = events
            .iter()
            .position(|e| matches!(e, Event::Switched { side: 1, .. }));
        let used = events.iter().position(|e| matches!(e, Event::Used { .. }));
        assert!(switched.is_some() && used.is_some());
        assert!(switched < used, "the switch resolves first");
        assert_eq!(b.sides[0].active, 1);
    }

    #[test]
    fn a_move_with_no_pp_struggles_instead() {
        let mut b = battle(
            mon("blaziken", 50, &["ember"]),
            mon("treecko", 50, &["pound"]),
        );
        b.sides[0].party[0].moves[0].pp = 0;
        let hp = b.sides[1].mon().hp;
        let before = b.sides[0].mon().hp;
        let events = b.step([Choice::Move(0), Choice::Move(0)]);
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Used { side: 1, .. })));
        assert!(b.sides[1].mon().hp < hp, "Struggle landed");
        assert!(b.sides[0].mon().hp < before, "and recoiled");
        assert_eq!(b.sides[0].mon().moves[0].pp, 0, "no PP moved");
    }

    fn scripted(script: [SeatScript; 2]) -> TurnScript {
        TurnScript {
            seats: [Some(script[0]), Some(script[1])],
        }
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
        let mut b = battle(
            mon("blaziken", 50, &["headbutt"]),
            mon("snorlax", 50, &["pound"]),
        );
        assert!(b.sides[0].mon().spe > b.sides[1].mon().spe);
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([
                SeatScript {
                    secondary: true,
                    ..PLAIN
                },
                PLAIN,
            ]),
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Flinched { side: 2 })));
        assert!(!events
            .iter()
            .any(|e| matches!(e, Event::Used { side: 2, .. })));
        // The flinch does not leak into the next turn.
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Used { side: 2, .. })));
    }

    #[test]
    fn sleep_lasts_its_clock_and_the_mon_acts_the_turn_it_wakes() {
        let mut b = battle(
            mon("blaziken", 50, &["sing"]),
            mon("snorlax", 50, &["pound"]),
        );
        // Turn 1: Sing lands (clock 2), and slower Snorlax's own action
        // already ticks it to 1 — a Cant the very turn it fell asleep.
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Statused {
                side: 2,
                status: Status::Sleep
            }
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Cant {
                side: 2,
                status: Status::Sleep
            }
        )));
        assert_eq!(b.sides[1].mon().sleep_n, 1);
        // Turn 2: 1 -> 0, it wakes and moves that same turn. The turn's
        // earlier Sing could not re-land — Snorlax still carried slp when it
        // resolved — so the wake leaves it clean.
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Used { side: 2, .. })));
        assert_eq!(b.sides[1].mon().status, None);
    }

    #[test]
    fn thunder_wave_respects_ground_immunity() {
        let mut b = battle(
            mon("pikachu", 50, &["thunderwave"]),
            mon("golem", 50, &["splash"]),
        );
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        assert!(!events.iter().any(|e| matches!(e, Event::Statused { .. })));
        assert_eq!(
            b.sides[1].mon().status,
            None,
            "a Ground type shrugs off Thunder Wave"
        );
    }

    #[test]
    fn confusion_ticks_selfhits_and_lifts() {
        // Gengar confuses Snorlax; the scripted coin says "hit yourself".
        let mut b = battle(
            mon("gengar", 50, &["confuseray"]),
            mon("snorlax", 50, &["pound"]),
        );
        let hp = b.sides[1].mon().hp;
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([
                PLAIN,
                SeatScript {
                    selfhit: true,
                    ..PLAIN
                },
            ]),
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::ConfusionStarted { side: 2 })));
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::ConfusedHit { side: 2, .. })));
        assert!(!events
            .iter()
            .any(|e| matches!(e, Event::Used { side: 2, .. })));
        assert!(b.sides[1].mon().hp < hp, "the self-hit landed");
        // Next turn the clock hits zero: confusion lifts and Snorlax acts,
        // and the re-Confuse Ray fails against the still-confused target
        // (it resolved before the clock ticked).
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([
                PLAIN,
                SeatScript {
                    selfhit: true,
                    ..PLAIN
                },
            ]),
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::ConfusionEnded { side: 2 })));
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Used { side: 2, .. })));
    }

    #[test]
    fn full_paralysis_spends_the_turn_but_no_pp() {
        let mut b = battle(
            mon("blaziken", 50, &["ember"]),
            mon("treecko", 50, &["pound"]),
        );
        b.sides[0].party[0].status = Some(Status::Paralysis);
        let pp = b.sides[0].mon().moves[0].pp;
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([
                SeatScript {
                    immobile: true,
                    ..PLAIN
                },
                PLAIN,
            ]),
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::FullyParalyzed { side: 1 })));
        assert!(!events
            .iter()
            .any(|e| matches!(e, Event::Used { side: 1, .. })));
        assert_eq!(
            b.sides[0].mon().moves[0].pp,
            pp,
            "full paralysis spends no PP"
        );
    }

    #[test]
    fn toxic_ticks_grow_and_reset_on_switching_out() {
        let side1 = Side::new(alloc::vec![
            mon("snorlax", 50, &["pound"]),
            mon("mudkip", 50, &["pound"])
        ]);
        let side2 = Side::new(alloc::vec![mon("treecko", 50, &["pound"])]);
        let mut b = Battle::new(side1, side2, 3);
        b.sides[0].party[0].status = Some(Status::Toxic);
        let max = b.sides[0].mon().max_hp;
        let hp0 = b.sides[0].mon().hp;
        let miss = SeatScript {
            hit: false,
            ..PLAIN
        };
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([miss, miss]));
        let tick1 = hp0 - b.sides[0].mon().hp;
        assert_eq!(tick1, (max / 16).max(1), "first tick is one sixteenth");
        let hp1 = b.sides[0].mon().hp;
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([miss, miss]));
        assert_eq!(hp1 - b.sides[0].mon().hp, tick1 * 2, "second tick doubles");
        // Switching out resets the clock; the turn Snorlax comes back in,
        // its tick is a sixteenth again rather than a third multiple.
        b.step_with(
            [Choice::Switch(1), Choice::Move(0)],
            &scripted([miss, miss]),
        );
        let hp = b.sides[0].party[0].hp;
        b.step_with(
            [Choice::Switch(0), Choice::Move(0)],
            &scripted([miss, miss]),
        );
        assert_eq!(hp - b.sides[0].party[0].hp, tick1, "the counter restarted");
    }

    #[test]
    fn drain_heals_half_the_damage_and_recoil_floors_a_third() {
        let mut b = battle(
            mon("blaziken", 50, &["doubleedge"]),
            mon("snorlax", 50, &["gigadrain"]),
        );
        b.sides[1].party[0].hp -= 40; // room to heal into
        let (hp1, hp2) = (b.sides[0].mon().hp, b.sides[1].mon().hp);
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
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
        assert_eq!(
            b.sides[0].mon().hp,
            hp1 - to_blaziken - (to_snorlax / 3).max(1)
        );
        // Snorlax: hit by Double-Edge, healed half of its own Giga Drain.
        assert_eq!(
            b.sides[1].mon().hp,
            hp2 - to_snorlax + (to_blaziken / 2).max(1)
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Recoil { side: 1, .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Drained { side: 2, .. })));
    }

    #[test]
    fn a_multi_hit_move_strikes_the_scripted_count() {
        // Fury Attack at 4 scripted strikes: four damage events, one PP.
        let mut b = battle(
            mon("blaziken", 50, &["furyattack"]),
            mon("snorlax", 50, &["pound"]),
        );
        let miss = SeatScript {
            hit: false,
            ..PLAIN
        };
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([SeatScript { hits: 4, ..PLAIN }, miss]),
        );
        let strikes = events
            .iter()
            .filter(|e| matches!(e, Event::Damage { side: 2, .. }))
            .count();
        assert_eq!(strikes, 4);
        assert_eq!(
            b.sides[0].mon().moves[0].pp,
            b.sides[0].mon().moves[0].entry.pp - 1
        );

        // Double Kick is a fixed two: the script's count does not move it.
        let mut b = battle(
            mon("blaziken", 50, &["doublekick"]),
            mon("snorlax", 50, &["pound"]),
        );
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([SeatScript { hits: 5, ..PLAIN }, miss]),
        );
        let strikes = events
            .iter()
            .filter(|e| matches!(e, Event::Damage { side: 2, .. }))
            .count();
        assert_eq!(strikes, 2);
    }

    #[test]
    fn status_moves_inflict_boost_and_heal() {
        // Thunder Wave: paralysis lands through the status-move path.
        let mut b = battle(
            mon("blaziken", 50, &["thunderwave"]),
            mon("snorlax", 50, &["pound"]),
        );
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        assert!(events.iter().any(|e| matches!(
            e,
            Event::Statused {
                side: 2,
                status: Status::Paralysis
            }
        )));
        assert_eq!(b.sides[1].mon().status, Some(Status::Paralysis));

        // Swords Dance doubles the next physical hit: +2 Attack stages.
        let mut b = battle(
            mon("blaziken", 50, &["swordsdance", "doublekick"]),
            mon("snorlax", 50, &["splash"]),
        );
        b.step_with(
            [Choice::Move(0), Choice::Move(1)],
            &scripted([PLAIN, PLAIN]),
        );
        assert_eq!(b.sides[0].mon().stages[Stat::Atk as usize], 2);
        let hp_before = b.sides[1].mon().hp;
        b.step_with(
            [Choice::Move(1), Choice::Move(1)],
            &scripted([PLAIN, PLAIN]),
        );
        let boosted = hp_before - b.sides[1].mon().hp;
        let mut plain = battle(
            mon("blaziken", 50, &["doublekick"]),
            mon("snorlax", 50, &["splash"]),
        );
        let hp_before = plain.sides[1].mon().hp;
        plain.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        let unboosted = hp_before - plain.sides[1].mon().hp;
        // Not exactly double: the flat +2 and the floors sit outside the
        // stage multiply. Meaningfully bigger is the claim.
        assert!(
            boosted > unboosted * 3 / 2,
            "+2 Atk hits harder: {boosted} vs {unboosted}"
        );

        // Recover heals half of max, capped at full.
        let mut b = battle(
            mon("blaziken", 50, &["recover"]),
            mon("snorlax", 50, &["splash"]),
        );
        let max = b.sides[0].mon().max_hp;
        b.sides[0].party[0].hp = 1;
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        assert_eq!(b.sides[0].mon().hp, 1 + max / 2);
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Healed { side: 1, .. })));

        // A scripted miss keeps Growl off the target's stages.
        let mut b = battle(
            mon("blaziken", 50, &["growl"]),
            mon("snorlax", 50, &["splash"]),
        );
        let miss = SeatScript {
            hit: false,
            ..PLAIN
        };
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([miss, PLAIN]));
        assert_eq!(b.sides[1].mon().stages[Stat::Atk as usize], 0);
        b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        assert_eq!(b.sides[1].mon().stages[Stat::Atk as usize], -1);
    }

    #[test]
    fn charge_moves_take_two_turns_and_recharge_costs_one() {
        // Solar Beam: turn 1 charges (one PP, no damage), turn 2 releases.
        let mut b = battle(
            mon("venusaur", 50, &["solarbeam"]),
            mon("snorlax", 50, &["splash"]),
        );
        let hp = b.sides[1].mon().hp;
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Charging { side: 1 })));
        assert!(!events
            .iter()
            .any(|e| matches!(e, Event::Damage { side: 2, .. })));
        assert_eq!(
            b.sides[0].mon().moves[0].pp,
            b.sides[0].mon().moves[0].entry.pp - 1
        );
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Damage { side: 2, .. })));
        assert!(b.sides[1].mon().hp < hp);
        assert_eq!(
            b.sides[0].mon().moves[0].pp,
            b.sides[0].mon().moves[0].entry.pp - 1,
            "the release costs no second PP"
        );

        // Hyper Beam: the landed hit costs the next action. (Snorlax's own
        // bulk keeps the target alive to see the recharge.)
        let mut b = battle(
            mon("snorlax", 50, &["hyperbeam"]),
            mon("snorlax", 50, &["splash"]),
        );
        b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Recharging { side: 1 })));
        assert!(!events
            .iter()
            .any(|e| matches!(e, Event::Used { side: 1, .. })));
        // And the turn after, it attacks again.
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Used { side: 1, .. })));
    }

    #[test]
    fn semi_invulnerability_dodges_and_earthquake_pierces_dig_doubled() {
        // Mid-Dig, Tackle whiffs without even rolling accuracy.
        let mut b = battle(
            mon("sandslash", 50, &["dig"]),
            mon("snorlax", 50, &["tackle"]),
        );
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Charging { side: 1 })));
        assert!(!events
            .iter()
            .any(|e| matches!(e, Event::Damage { side: 1, .. })));
        assert_eq!(b.sides[0].mon().hp, b.sides[0].mon().max_hp);

        // Mid-Dig, Earthquake connects — at double power.
        let mut plain = battle(
            mon("snorlax", 50, &["earthquake"]),
            mon("sandslash", 50, &["splash"]),
        );
        let hp = plain.sides[1].mon().hp;
        plain.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        let normal_hit = hp - plain.sides[1].mon().hp;

        let mut b = battle(
            mon("sandslash", 50, &["dig"]),
            mon("snorlax", 50, &["earthquake"]),
        );
        // Snorlax is slower: Sandslash digs, then Earthquake lands doubled.
        assert!(b.sides[0].mon().spe > b.sides[1].mon().spe);
        let hp = b.sides[0].mon().hp;
        b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        let pierced = hp - b.sides[0].mon().hp;
        assert!(
            pierced > normal_hit * 3 / 2,
            "doubled: {pierced} vs {normal_hit}"
        );
    }

    #[test]
    fn screens_halve_safeguard_shields_and_mist_holds_stages() {
        // Reflect roughly halves a physical hit.
        let mut plain = battle(
            mon("snorlax", 50, &["tackle"]),
            mon("chansey", 50, &["splash"]),
        );
        let hp = plain.sides[1].mon().hp;
        plain.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        let open_hit = hp - plain.sides[1].mon().hp;

        let mut b = battle(
            mon("snorlax", 50, &["tackle"]),
            mon("chansey", 50, &["reflect"]),
        );
        assert!(
            b.sides[1].mon().spe > b.sides[0].mon().spe,
            "chansey screens first"
        );
        b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        let hp = b.sides[1].mon().hp;
        b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        let screened = hp - b.sides[1].mon().hp;
        assert!(
            screened < open_hit * 2 / 3,
            "reflected: {screened} vs {open_hit}"
        );

        // Safeguard blocks Thunder Wave for the whole team. (Snorlax is
        // slower than Chansey, so the shield is up before the wave.)
        let mut b = battle(
            mon("snorlax", 50, &["thunderwave"]),
            mon("chansey", 50, &["safeguard"]),
        );
        b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        assert_eq!(b.sides[1].mon().status, None);

        // Mist holds Growl off.
        let mut b = battle(
            mon("snorlax", 50, &["growl"]),
            mon("chansey", 50, &["mist"]),
        );
        b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        assert_eq!(b.sides[1].mon().stages[Stat::Atk as usize], 0);
    }

    #[test]
    fn recoil_can_knock_the_user_out() {
        let side1 = Side::new(alloc::vec![
            mon("blaziken", 100, &["doubleedge"]),
            mon("mudkip", 50, &["pound"])
        ]);
        let side2 = Side::new(alloc::vec![mon("snorlax", 100, &["pound"])]);
        let mut b = Battle::new(side1, side2, 3);
        b.sides[0].party[0].hp = 1;
        let miss = SeatScript {
            hit: false,
            ..PLAIN
        };
        let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, miss]));
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::Fainted { side: 1 })));
        assert_eq!(b.sides[0].active, 1, "the bench replaced the recoil faint");
    }

    #[test]
    fn a_fire_hit_thaws_the_target_but_its_burn_chance_is_blocked() {
        let mut b = battle(
            mon("blaziken", 50, &["ember"]),
            mon("treecko", 50, &["pound"]),
        );
        b.sides[1].party[0].status = Some(Status::Freeze);
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([
                SeatScript {
                    secondary: true,
                    ..PLAIN
                },
                PLAIN,
            ]),
        );
        assert_eq!(b.sides[1].mon().status, None, "the freeze thawed");
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::Statused { side: 2, .. })),
            "the burn chance was blocked by the freeze it cured"
        );
    }

    #[test]
    fn the_same_seed_replays_the_same_battle() {
        let run = || {
            let mut b = battle(
                mon("blaziken", 50, &["ember"]),
                mon("treecko", 50, &["pound"]),
            );
            let mut all = Vec::new();
            for _ in 0..4 {
                all.extend(b.step([Choice::Move(0), Choice::Move(0)]));
            }
            all
        };
        assert_eq!(run(), run(), "a seeded battle has to be reproducible");
    }
}
