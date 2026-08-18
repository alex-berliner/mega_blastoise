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
use crate::data::{
    move_by_id, species_by_id, Boost, MoveEntry, SideCondition,
    SpeciesEntry, Status, Weather,
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
    /// Encore landed THIS turn, after its victim had already chosen. The
    /// forcing then happens at execution time, below every gate.
    pub encore_fresh: bool,
    /// Rage is rolling: hits taken raise Attack.
    pub raging: bool,
    /// Fury Cutter's consecutive-hit ramp (0..=4).
    pub fury_n: u8,
    /// Fury Cutter landed THIS turn, so its ramp has been refreshed.
    pub fury_fresh: bool,
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
    /// The sim's `lastMoveUsed`, which is NOT the same thing as `lastMove`:
    /// `useMoveInner` writes this one for every move that goes off, called
    /// moves included, while `lastMove` is only written by `runMove`. Gen 3
    /// has exactly one reader of it — Conversion 2 — and that is why a
    /// bounced Toxic retypes its bouncer even though a Torment would still
    /// be greying out the Magic Coat.
    pub last_move_used_id: Option<&'static str>,
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
    /// WHICH slot the Encore locked, snapshotted when it landed — the sim
    /// pins `effectState.move` at onStart and never re-derives it. Reading
    /// the live `last_used` instead let a Torment-forced Struggle (which
    /// clears `last_used`) end an Encore a turn early.
    pub encored_slot: Option<u8>,
    /// The bind's user has LEFT (fainted or switched). The sim does not free
    /// the victim on the spot: the stale volatile lingers until the victim's
    /// next residual, where it is deleted silently with no chip — and while
    /// it lingers it blocks a fresh binding move from attaching (addVolatile
    /// refuses a volatile already present, and gen 3's has no onRestart). It
    /// no longer holds the victim in, though: the trapper is gone.
    pub trap_stale: bool,
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
    /// Magic Coat is up: the next reflectable move aimed here goes back the
    /// way it came. One turn only, like Protect's shield.
    pub magic_coat: bool,
    /// Snatch is up: the next self-aimed move anyone reaches for is taken.
    pub snatching: bool,
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
    /// The recharge was set THIS turn, so its own residual has not run yet.
    /// The sim gives the volatile a duration of two, which means it expires
    /// at the end of the following turn whether or not it was ever spent.
    pub recharge_fresh: bool,
    /// This mon's ability, as a lookup id, or empty for none. Trace and
    /// Role Play overwrite it, so it is not simply read off the species.
    pub ability: &'static str,
    /// Knock Off took this mon's item. In gens 3 and 4 the item is only made
    /// UNUSABLE rather than removed, and the sim records that as a flag it
    /// never clears — not on switch-out, not on faint. `takeItem` refuses
    /// whenever either side of the exchange carries it, so a mon that has
    /// been knocked off can never steal an item nor be robbed of one again.
    pub item_knocked_off: bool,
    /// Which party slot on the other side charmed this mon, if any. The
    /// volatile ends by itself the moment that mon is no longer the one
    /// standing opposite, and a Baton Pass does not carry it.
    pub attracted_by: Option<usize>,
    /// "M", "F" or "N". Attract is the only thing in the era that reads it,
    /// and it reads it as a plain comparison. A species that can be either
    /// starts male here; callers that care set it themselves.
    pub gender: &'static str,
    /// Flash Fire has caught: this mon's Fire moves are half again as strong
    /// until it leaves the field.
    pub flash_fire: bool,
    /// What this mon was born with, stashed the first time something in the
    /// battle overwrites its ability. An ability handed over by Trace,
    /// Transform, Role Play or Skill Swap is a VOLATILE in the sim — leaving
    /// the field restores the original, which is why a Trace user copies
    /// afresh every time it comes back in.
    pub ability_backup: Option<&'static str>,
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
        let gender = match species.gender {
            "" => "M",
            fixed => fixed,
        };
        Some(Mon {
            species,
            gender,
            item_knocked_off: false,
            attracted_by: None,
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
            encore_fresh: false,
            raging: false,
            fury_n: 0,
            fury_fresh: false,
            last_used: None,
            last_used_id: None,
            last_move_used_id: None,
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
            encored_slot: None,
            trap_stale: false,
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
            magic_coat: false,
            snatching: false,
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
            recharge_fresh: false,
            ability: "",
            flash_fire: false,
            ability_backup: None,
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
    /// Speed with its stages in and NOTHING else. Paralysis, Swift Swim and
    /// a Macho Brace are all `ModifySpe` links in this era, which means they
    /// fold into one chain applied once — see `turn_speed`. Taking the
    /// quarter here on its own rounded twice and came out a point high.
    pub fn effective_speed(&self) -> u16 {
        crate::stats::apply_stage(self.spe, self.stages[Stat::Spe as usize])
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
    /// A Focus Band holder's 1-in-10 comes up, so a lethal blow this action
    /// leaves it standing at one HP.
    pub band: bool,
}

/// Per-turn RNG script; see [`Battle::step_with`].
#[derive(Clone, Copy, Debug, Default)]
pub struct TurnScript {
    pub seats: [Option<SeatScript>; 2],
    /// The turn's Quick Claw coin. One coin serves the whole field in this
    /// era, so it is a turn knob rather than a seat one: when it comes up,
    /// EVERY holder on the field moves at 65535.
    pub claw: bool,
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
    /// `side`'s mon was too taken with the other one to act.
    Infatuated {
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
    /// This turn's Quick Claw coin, flipped once for the whole field.
    claw_this_turn: bool,
    /// This turn's per-seat script, kept where anything that needs the OTHER
    /// seat's knobs can reach it — Focus Band's coin is rolled by the mon
    /// being hit, not by the one attacking.
    turn_seats: [Option<SeatScript>; 2],
    /// The move each side has just committed to, held until the move finishes
    /// so a Choice Band can clamp on it afterwards.
    committed_move: [Option<&'static str>; 2],
    /// A move handed to a mon by something other than its own choice: the
    /// move a Magic Coat threw back, or the one a Snatch took. The sim runs
    /// these through `useMove` directly, which sits inside the can't-move
    /// gates and inside the PP charge, so they answer to neither.
    forced_entry: Option<&'static MoveEntry>,
    /// Set while such a move is running.
    calling: bool,
    /// Whoever is owed one, and what: drained once the current move is done.
    pending_call: Option<(usize, &'static MoveEntry)>,
    /// The move being resolved kills its own user outright — an Explosion or
    /// a Self-Destruct — which the sim queues before the hit.
    self_destructed: bool,
    /// A Yawn is collecting right now. Safeguard blocks the drowsiness going
    /// on, and lets the sleep it delivers through — the sim writes that
    /// exemption into Safeguard's own onSetStatus as `if (effect.id ===
    /// 'yawn') return`.
    yawn_landing: bool,
    /// Whether each side's chosen slot was SEALED by an Imprison as the turn
    /// opened. The sim clears and recomputes every disable flag in
    /// `nextTurn`, before it asks for choices, and never revises them — so an
    /// imprisoner that switches out mid-turn does not give its victim its
    /// moves back until the turn after.
    sealed_at_choice: [bool; 2],
    /// A move already thrown back once. The sim's `hasBounced` — a Magic
    /// Coat cannot volley against another Magic Coat.
    bounced: bool,
}

/// How many residual buckets the end-of-turn phase walks.
const BUCKETS: usize = 13;

/// The `comparePriority` key for one residual bucket on a mon of this Speed.
///
/// The sim reads `(order, priority, speed, subOrder, effectOrder)` and folds
/// a MISSING order into 4294967296, which sends it to the very back — that is
/// where the two bare duration volatiles land, since neither Recharge nor the
/// Thrash-family lock carries a residual order of its own. Nothing in this
/// era gives a residual handler a priority, and `effectOrder` is only ever
/// assigned for `SwitchIn` and `RedirectTarget`, so both drop out and the key
/// is just the three that are left.
fn residual_key(bucket: usize, speed: u16) -> (u32, i32, i32, u32) {
    // (order, -priority, -speed, subOrder): every field now sorts ASCENDING,
    // which is what makes plain tuple comparison the same sort.
    // The sim's stand-in is 4294967296, which does not fit here; any value
    // past the largest real order (Truant's 27) sorts the same.
    const NO_ORDER: u32 = u32::MAX;
    let (order, sub): (u32, u32) = match bucket {
        0 => (10, 1),   // Ingrain
        1 => (10, 3),   // Rain Dish, Shed Skin, Speed Boost
        2 => (10, 4),   // the berries and Leftovers
        3 => (10, 5),   // Leech Seed
        4 => (10, 6),   // burn, poison and Toxic
        5 => (10, 7),   // Nightmare
        6 => (10, 8),   // Curse
        7 => (10, 9),   // the bind
        9 => (10, 11),  // Uproar
        10 => (10, 19), // Yawn
        12 => (27, 0),  // Truant
        // Recharge (8) and the Thrash-family lock (11) are plain duration
        // volatiles with no residual order at all.
        _ => (NO_ORDER, 0),
    };
    (order, 0, -(speed as i32), sub)
}

/// The sim's `speedSort`, which is a selection sort and deliberately so: it
/// is the shape that makes a speed tie easy to resolve. The important part is
/// what it does to the tied elements it is NOT currently looking at. Having
/// found the run of minima, it swaps each one into place from wherever it sat
/// — and whatever was already in that slot goes back to where the minimum
/// came from. A tied pair therefore comes out REVERSED whenever a lower-key
/// handler was sitting behind it.
///
/// The sim then shuffles each run of ties with its own PRNG. The reference
/// runs we check against pin that shuffle to a no-op, so a run is left in the
/// order the swaps put it, and this does the same.
fn speed_sort<T, K: Ord, F: Fn(&T) -> K>(list: &mut [T], key: F) {
    if list.len() < 2 {
        return;
    }
    let mut sorted = 0;
    while sorted + 1 < list.len() {
        let mut next: Vec<usize> = Vec::new();
        next.push(sorted);
        for i in (sorted + 1)..list.len() {
            match key(&list[next[0]]).cmp(&key(&list[i])) {
                core::cmp::Ordering::Less => continue,
                core::cmp::Ordering::Greater => {
                    next.clear();
                    next.push(i);
                }
                core::cmp::Ordering::Equal => next.push(i),
            }
        }
        // `next` is ascending, so no swap here can disturb a later one.
        for (i, &index) in next.iter().enumerate() {
            if index != sorted + i {
                list.swap(sorted + i, index);
            }
        }
        sorted += next.len();
    }
}

mod residual;
mod status_move;
mod switching;
mod turn;
mod use_move;

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
            claw_this_turn: false,
            turn_seats: [None; 2],
            committed_move: [None; 2],
            forced_entry: None,
            calling: false,
            pending_call: None,
            bounced: false,
            yawn_landing: false,
            sealed_at_choice: [false; 2],
            self_destructed: false,
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
        // moment later.
        //
        // A berry mostly waits for the first update of the turn, which comes
        // after the speeds are read. The Lum Berry is the exception: it is
        // the only cure berry carrying an `onAfterSetStatus`, so handing the
        // status out is itself what eats it, and it is gone before turn one
        // is sorted. That only reaches a mon standing on the field —
        // `eatItem` refuses a benched one — so a Lum on the bench waits for
        // its holder to be sent out.
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
                } else if i == active && mon.item == "lumberry" {
                    mon.last_item = mon.item;
                    mon.item = "";
                    mon.status = None;
                    mon.sleep_n = 0;
                    mon.toxic_n = 0;
                }
            }
        }
        battle
    }

    /// Wonder Guard's try-hit gate: `runEffectiveness(move) <= 0` turns away
    /// anything that is not super effective. It is not a damage-formula
    /// modifier, so it answers for every non-status move whatever way that
    /// move works out its damage — an OHKO, a Seismic Toss, a Psywave and a
    /// Dragon Rage alike. A Night Shade gets through, because Ghost beats
    /// Shedinja's Ghost half. A typeless hit is exempt.
    /// The target's types as the IMMUNITY step sees them. Foresight and Odor
    /// Sleuth hang an `onNegateImmunity` on the target that cancels a Ghost's
    /// exemption from Normal and Fighting, and the sim asks that question in
    /// exactly ONE place — `runImmunity`, called from `hitStepTypeImmunity`,
    /// which sits above every damage arm. So the fixed-damage moves, the
    /// OHKOs, Endeavor, Psywave and Counter all inherit the strip there for
    /// free, and have to inherit it here too. `getEffectiveness` is a
    /// different question that Foresight does not touch, which is why Wonder
    /// Guard still reads the raw chart.
    fn immunity_types(&self, foe: usize, move_type: Type) -> (Type, Type) {
        let types = self.sides[foe].mon().types();
        if self.sides[foe].mon().identified
            && matches!(move_type, Type::Normal | Type::Fighting)
        {
            let strip = |t: Type| if t == Type::Ghost { Type::None } else { t };
            return (strip(types.0), strip(types.1));
        }
        types
    }

    fn wonder_guard_blocks(&self, foe: usize, move_type: Type) -> bool {
        self.sides[foe].mon().ability == "wonderguard"
            && move_type != Type::None
            && crate::types::effectiveness_against(move_type, self.sides[foe].mon().types()) <= 100
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
        // Every Speed modifier in this era answers the same ModifySpe event,
        // so they accumulate into ONE chain and the sim applies it once. A
        // paralysed Swift Swim mon in rain is x2 then x0.25 composed into
        // x0.5 and taken off the boosted stat in a single step — not halved
        // and quartered in turn, which rounds twice and lands a point out.
        let mut chain = item::speed_chain(&mon.holder());
        let sky = self.effective_weather();
        if ability::speed_doubles(
            &mon.bearer(),
            sky == Some(Weather::Sun),
            sky == Some(Weather::Rain),
        ) {
            chain.mul(ability::X2);
        }
        if mon.status == Some(Status::Paralysis) {
            chain.mul(ability::X0_25);
        }
        // Quick Claw is not a priority bracket in this era. The sim's gen 3
        // `getActionSpeed` simply answers 65535 for a holder when the turn's
        // claw came up, which wins its own bracket and loses to a higher one.
        // It rolls ONE claw per turn for the whole field, so two holders both
        // answer 65535 and tie. The same number feeds the residual sort,
        // because `case 'residual'` calls `updateSpeed()` before the field
        // event and `updateSpeed` is what writes this stat.
        if self.claw_this_turn && mon.item == "quickclaw" {
            return u16::MAX;
        }
        chain.apply(mon.effective_speed() as u32) as u16
    }

    pub fn can_switch(&self, side: usize) -> bool {
        let mon = self.sides[side].mon();
        if mon.fainted() {
            return true; // a replacement is always allowed in
        }
        // Ingrain's roots hold it as surely as a bind does: the condition's
        // onTrapPokemon calls tryTrap, so the request comes back trapped.
        if (mon.trapped_n > 0 && !mon.trap_stale) || mon.mean_looked || mon.ingrained {
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
                    && !(mon.encore_n > 0 && mon.encored_slot.is_some_and(|u| u != i as u8))
                    // A Choice Band greys out everything but the first swing.
                    && !mon
                        .choice_locked
                        .is_some_and(|id| id != slot.entry.id)
            })
            .collect()
    }

}

#[cfg(test)]
mod tests;
