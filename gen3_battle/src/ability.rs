//! Gen 3 abilities.
//!
//! The reference sim expresses every ability as a handler on one of its
//! events, and the events are the same pipeline stages this engine already
//! runs: pick a stat, pick a base power, roll accuracy, roll a crit, try a
//! status. So rather than build an event bus, each stage asks this module a
//! question — "does anything change the attack here?" — and the answers live
//! together, where the Gen 3 wording of each ability can be read in one go.
//!
//! Gen 3 is not the modern wording. Blaze and its kin boost BASE POWER here,
//! not the attacking stat; Thick Fat likewise cuts base power rather than the
//! stat; Flash Fire's boost is a damage-stage modifier; Volt Absorb lets Thunder
//! Wave through. Each of those is a difference the parity suite would otherwise
//! find one seed at a time.

use crate::data::{Boost, Status};
use crate::types::Type;

/// The sim's fixed-point modifier chain. Every multiplier is a numerator over
/// 4096, they compose with a rounding step between them, and the result is
/// applied to the value once at the end — which is why two modifiers on one
/// stage are not the same as applying each in turn.
#[derive(Clone, Copy, Debug)]
pub struct Chain(u32);

/// ×1.5, the sim's `chainModify(1.5)`.
pub const X1_5: u32 = 6144;
/// ×2.
pub const X2: u32 = 8192;
/// ×0.5.
pub const X0_5: u32 = 2048;
/// ×1.3, truncated the way the sim truncates it.
pub const X1_3: u32 = 5324;
/// ×0.8.
pub const X0_8: u32 = 3276;
/// Hustle's accuracy cut, which the sim writes out as a raw 3277/4096 rather
/// than as 0.8 — one part in 4096 away from Sand Veil's, and not the same.
pub const HUSTLE_ACC: u32 = 3277;

impl Default for Chain {
    fn default() -> Self {
        Chain::new()
    }
}

impl Chain {
    pub const fn new() -> Chain {
        Chain(4096)
    }

    /// Fold another multiplier in, rounding as the sim rounds.
    pub fn mul(&mut self, numerator: u32) {
        self.0 = (self.0 * numerator + 2048) >> 12;
    }

    /// Fold a whole second chain in, as the sim does when two effects answer
    /// the same event.
    pub fn extend(&mut self, other: Chain) {
        self.mul(other.0);
    }

    /// True while nothing has been folded in.
    pub fn is_identity(&self) -> bool {
        self.0 == 4096
    }

    /// Apply the accumulated chain to a value.
    pub fn apply(&self, value: u32) -> u32 {
        (value * self.0 + 2047) / 4096
    }
}

/// Everything about a mon that an ability question needs to see. Passing this
/// rather than the whole `Mon` keeps the module free of the battle state and
/// makes each answer a pure function of what the sim's handler actually reads.
#[derive(Clone, Copy, Debug)]
pub struct Bearer {
    pub ability: &'static str,
    pub types: (Type, Type),
    pub status: Option<Status>,
    pub hp: u16,
    pub max_hp: u16,
}

impl Bearer {
    fn pinched(&self) -> bool {
        self.hp as u32 * 3 <= self.max_hp as u32
    }

    fn has(&self, id: &str) -> bool {
        self.ability == id
    }
}

/// A pinch ability's half-again on its own type, once the user is down to a
/// third of its health. Gen 3 puts this on BASE POWER, where the modern game
/// puts it on the attacking stat.
pub fn pinch_boost(user: &Bearer, move_type: Type) -> bool {
    let pinch = match user.ability {
        "blaze" => Type::Fire,
        "overgrow" => Type::Grass,
        "torrent" => Type::Water,
        "swarm" => Type::Bug,
        _ => Type::None,
    };
    pinch != Type::None && move_type == pinch && user.pinched()
}

/// Thick Fat halves Ice and Fire, also at base power in this era.
pub fn thick_fat_cut(target: &Bearer, move_type: Type) -> bool {
    target.has("thickfat") && matches!(move_type, Type::Ice | Type::Fire)
}

/// The attacking stat, after stat stages. Huge Power and Pure Power double it
/// outright; Guts and Hustle add half again, Guts only while something ails
/// the user.
pub fn attack_chain(user: &Bearer, physical: bool) -> Chain {
    let mut chain = Chain::new();
    if !physical {
        return chain;
    }
    match user.ability {
        "hugepower" | "purepower" => chain.mul(X2),
        "hustle" => chain.mul(X1_5),
        "guts" if user.status.is_some() => chain.mul(X1_5),
        _ => {}
    }
    chain
}

/// The defending stat, after stat stages. Marvel Scale thickens a sick mon's
/// hide, and only its physical one.
pub fn defence_chain(target: &Bearer, physical: bool) -> Chain {
    let mut chain = Chain::new();
    if physical && target.has("marvelscale") && target.status.is_some() {
        chain.mul(X1_5);
    }
    chain
}

/// Guts shrugs off burn's Attack cut as well as taking its boost, so a burned
/// Guts mon hits harder than a healthy one.
pub fn ignores_burn_drop(user: &Bearer) -> bool {
    user.has("guts")
}

/// The accuracy stage. Compound Eyes sharpens every roll, Hustle blunts the
/// physical ones — and Gen 3 decides "physical" by the move's TYPE, since the
/// category follows the type in this era.
pub fn accuracy_chain(user: &Bearer, target: &Bearer, move_type: Type, sand: bool) -> Chain {
    let mut chain = Chain::new();
    // In the sim's handler order: Compound Eyes, then Sand Veil, then
    // Hustle. They chain, so the order decides the rounding.
    if user.has("compoundeyes") {
        chain.mul(X1_3);
    }
    if target.has("sandveil") && sand {
        chain.mul(X0_8);
    }
    if user.has("hustle") && physical_type(move_type) {
        chain.mul(HUSTLE_ACC);
    }
    chain
}

/// The nine types Hustle's accuracy cut names. The sim writes them out as a
/// list rather than as "not special", and Struggle's typeless hit is not on
/// it — so this is NOT the same question as the category one below.
pub fn physical_type(t: Type) -> bool {
    matches!(
        t,
        Type::Normal
            | Type::Fighting
            | Type::Flying
            | Type::Poison
            | Type::Ground
            | Type::Rock
            | Type::Bug
            | Type::Ghost
            | Type::Steel
    )
}

/// Whether a move of this type is PHYSICAL in Gen 3, which the era decides by
/// excluding the eight special types. A typeless hit — Struggle, a delayed
/// Future Sight — is physical, so a Choice Band swings it half again as hard.
pub fn physical_category(t: Type) -> bool {
    !matches!(
        t,
        Type::Fire
            | Type::Water
            | Type::Grass
            | Type::Ice
            | Type::Electric
            | Type::Dark
            | Type::Psychic
            | Type::Dragon
    )
}

/// Battle Armor and Shell Armor refuse critical hits outright.
pub fn blocks_crit(target: &Bearer) -> bool {
    matches!(target.ability, "battlearmor" | "shellarmor")
}

/// The status a mon simply cannot catch. Toxic counts as poison here, and a
/// mon already immune by type is handled elsewhere.
pub fn blocks_status(target: &Bearer, status: Status) -> bool {
    match target.ability {
        "immunity" => matches!(status, Status::Poison | Status::Toxic),
        "insomnia" | "vitalspirit" => status == Status::Sleep,
        "limber" => status == Status::Paralysis,
        "waterveil" => status == Status::Burn,
        "magmaarmor" => status == Status::Freeze,
        _ => false,
    }
}

/// Yawn never takes hold on a mon that cannot sleep in the first place.
pub fn blocks_yawn(target: &Bearer) -> bool {
    matches!(target.ability, "insomnia" | "vitalspirit")
}

/// Own Tempo keeps a clear head; Inner Focus does not flinch.
pub fn blocks_confusion(target: &Bearer) -> bool {
    target.has("owntempo")
}

pub fn blocks_flinch(target: &Bearer) -> bool {
    target.has("innerfocus")
}

/// A stat drop coming from the other side. Clear Body and White Smoke refuse
/// all of them, Hyper Cutter guards Attack, Keen Eye guards accuracy. A mon
/// lowering its own stats is never stopped.
pub fn blocks_drop(target: &Bearer, stat: Drop) -> bool {
    match target.ability {
        "clearbody" | "whitesmoke" => true,
        "hypercutter" => stat == Drop::Attack,
        "keeneye" => stat == Drop::Accuracy,
        _ => false,
    }
}

/// Which guard, if any, covers this stat.
pub fn drop_kind(boost: Boost) -> Drop {
    match boost {
        Boost::Atk => Drop::Attack,
        Boost::Acc => Drop::Accuracy,
        _ => Drop::Other,
    }
}

/// Which stat a drop is aimed at, in the only granularity the guards need.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drop {
    Attack,
    Accuracy,
    Other,
}

/// What an ability does with an incoming move before it can land.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Absorb {
    /// Nothing: the move carries on.
    None,
    /// The move is refused and nothing else happens.
    Immune,
    /// The move is refused and the target heals a quarter of its maximum.
    Drain,
    /// The move is refused and the target's Fire moves are boosted from now on.
    FlashFire,
}

/// The try-hit stage, in the sim's order. Wonder Guard is checked by the
/// caller, which is the only one that knows the type chart's answer.
pub fn absorbs(target: &Bearer, move_id: &str, move_type: Type, sound: bool) -> Absorb {
    match target.ability {
        // Thunder Wave slips past Volt Absorb in this era: the sim spells the
        // exception out rather than leaning on the move being a status one.
        "voltabsorb" if move_type == Type::Electric && move_id != "thunderwave" => Absorb::Drain,
        "waterabsorb" if move_type == Type::Water => Absorb::Drain,
        "flashfire" if move_type == Type::Fire => {
            // Will-O-Wisp is waved through against a target that could not
            // have been burned anyway, and a frozen mon is never lit up.
            let wisp_noop = move_id == "willowisp"
                && (target.types.0 == Type::Fire
                    || target.types.1 == Type::Fire
                    || target.status.is_some());
            if wisp_noop || target.status == Some(Status::Freeze) {
                Absorb::None
            } else {
                Absorb::FlashFire
            }
        }
        "soundproof" if sound => Absorb::Immune,
        _ => Absorb::None,
    }
}

/// Levitate floats over Ground moves.
pub fn immune_to_type(target: &Bearer, move_type: Type) -> bool {
    target.has("levitate") && move_type == Type::Ground
}

/// Damp smothers Self-Destruct and Explosion from either side.
pub fn damp_present(a: &Bearer, b: &Bearer) -> bool {
    a.has("damp") || b.has("damp")
}

/// Sturdy in Gen 3 is only an answer to the one-hit-KO moves; the endure-a-hit
/// behaviour is a later generation's.
pub fn blocks_ohko(target: &Bearer) -> bool {
    target.has("sturdy")
}

/// Rock Head takes no recoil. Struggle's is not recoil the ability recognises.
pub fn ignores_recoil(user: &Bearer, move_id: &str) -> bool {
    user.has("rockhead") && move_id != "struggle"
}

/// Liquid Ooze turns a drain into damage. Dream Eater is exempt in Gen 3.
pub fn ooze_reverses_drain(target: &Bearer, move_id: &str) -> bool {
    target.has("liquidooze") && move_id != "dreameater"
}

/// Shield Dust shrugs off a move's secondary effect, but not one the move
/// turns on its own user.
pub fn blocks_secondary(target: &Bearer) -> bool {
    target.has("shielddust")
}

/// Serene Grace doubles every secondary chance the move carries.
pub fn doubles_secondary(user: &Bearer) -> bool {
    user.has("serenegrace")
}

/// Suction Cups holds its ground against Roar and Whirlwind.
pub fn blocks_drag(target: &Bearer) -> bool {
    target.has("suctioncups")
}

/// Natural Cure sheds status on the way out.
pub fn cures_on_switch_out(mon: &Bearer) -> bool {
    mon.has("naturalcure")
}

/// Early Bird burns sleep twice as fast: the sim decrements the clock a second
/// time before the usual one.
pub fn early_bird(mon: &Bearer) -> bool {
    mon.has("earlybird")
}

/// Pressure charges the other side an extra PP per move aimed at its bearer.
pub fn pressure(target: &Bearer) -> bool {
    target.has("pressure")
}

/// Doubles this mon's Speed under its own weather. Chlorophyll wants sun,
/// Swift Swim wants rain, and neither notices a sky that Air Lock has
/// flattened — the caller passes what the weather EFFECTIVELY is.
pub fn speed_doubles(mon: &Bearer, sun: bool, rain: bool) -> bool {
    (mon.has("chlorophyll") && sun) || (mon.has("swiftswim") && rain)
}

/// Sand Veil also shelters its bearer from the sandstorm's own chip.
pub fn immune_to_sandstorm(mon: &Bearer) -> bool {
    mon.has("sandveil")
}

/// The weather a mon lays down as it arrives.
pub fn weather_on_entry(mon: &Bearer) -> Option<&'static str> {
    match mon.ability {
        "drizzle" => Some("rain"),
        "drought" => Some("sun"),
        "sandstream" => Some("sandstorm"),
        _ => None,
    }
}

/// Air Lock and Cloud Nine flatten the sky for as long as they are out.
pub fn suppresses_weather(mon: &Bearer) -> bool {
    matches!(mon.ability, "airlock" | "cloudnine")
}

/// Intimidate cows the mon across the field as its bearer arrives.
pub fn intimidates(mon: &Bearer) -> bool {
    mon.has("intimidate")
}

/// Trace takes a copy of what it sees. Nothing in this era refuses to be
/// traced, but a mon with no ability has nothing to give.
pub fn traces(mon: &Bearer) -> bool {
    mon.has("trace")
}

/// Speed Boost climbs a stage at the end of every turn its bearer has spent
/// on the field.
pub fn speed_boosts(mon: &Bearer) -> bool {
    mon.has("speedboost")
}

/// Rain Dish sips a sixteenth back while it rains.
pub fn rain_dish(mon: &Bearer) -> bool {
    mon.has("raindish")
}

/// Shed Skin's third of a chance at shrugging a status off each turn.
pub fn sheds_skin(mon: &Bearer) -> bool {
    mon.has("shedskin")
}

/// Truant loafs about every other turn.
pub fn truant(mon: &Bearer) -> bool {
    mon.has("truant")
}

/// Rough Skin grazes whatever touches it for a sixteenth of that mon's
/// maximum.
pub fn rough_skin(mon: &Bearer) -> bool {
    mon.has("roughskin")
}

/// Color Change takes on the type of the move that just landed.
pub fn color_change(mon: &Bearer) -> bool {
    mon.has("colorchange")
}

/// What a contact ability gives back to whatever touched it, if its third
/// (or tenth, for Effect Spore) comes up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnTouch {
    None,
    Status(Status),
    /// Effect Spore picks between sleep, paralysis and poison; the sim
    /// samples that list, and a pinned sample takes the first.
    Spore,
    Attract,
}

/// The odds, out of the given denominator, that a contact ability answers.
pub fn on_touch(mon: &Bearer) -> (OnTouch, u32) {
    match mon.ability {
        "static" => (OnTouch::Status(Status::Paralysis), 3),
        "poisonpoint" => (OnTouch::Status(Status::Poison), 3),
        "flamebody" => (OnTouch::Status(Status::Burn), 3),
        "cutecharm" => (OnTouch::Attract, 3),
        "effectspore" => (OnTouch::Spore, 10),
        _ => (OnTouch::None, 1),
    }
}

/// Synchronize passes the status it just caught back to whoever gave it.
/// Sleep and freeze do not travel, and Toxic arrives as ordinary poison.
pub fn synchronizes(mon: &Bearer, status: Status) -> Option<Status> {
    if !mon.has("synchronize") {
        return None;
    }
    match status {
        Status::Sleep | Status::Freeze => None,
        Status::Toxic => Some(Status::Poison),
        other => Some(other),
    }
}

/// Whether this mon holds the other side in place, given what the other mon
/// is. Shadow Tag lets another Shadow Tag go; Magnet Pull only holds Steel;
/// Arena Trap only holds what stands on the ground.
pub fn traps(mon: &Bearer, victim: &Bearer, victim_grounded: bool) -> bool {
    match mon.ability {
        "shadowtag" => !victim.has("shadowtag"),
        "magnetpull" => victim.types.0 == Type::Steel || victim.types.1 == Type::Steel,
        "arenatrap" => victim_grounded,
        _ => false,
    }
}
