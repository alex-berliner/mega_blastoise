//! `kingsrock`: King's Rock, a 10% flinch added to the moves on gen 3's
//! list.
//!
//! PS pushes the flinch onto the move in flight (`move.secondaries.push`),
//! after the move's own secondaries; [`crate::actions::ActiveMove`]'s
//! `added_secondary` is that push, and the base `secondaries` rolls it
//! last. It is on the same array the abilities see: Shield Dust filters
//! it out and Serene Grace (whose `ModifyMove` runs after this one)
//! doubles its chance.
//!
//! PS: data/mods/gen3/items.ts kingsrock

use ps_dex::gen3::{conditions, moves as m};
use ps_dex::{ConditionId, MoveId, Secondary};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `{chance: 10, volatileStatus: "flinch"}`.
static FLINCH: Secondary = Secondary {
    chance: Some(10),
    status: ConditionId::NONE,
    volatile_status: conditions::FLINCH,
    boosts: None,
    self_effect: None,
    handlers: &[],
};

/// `affectedByKingsRock`, in PS's order.
const AFFECTED: [MoveId; 130] = [
    m::AERIALACE, m::AEROBLAST, m::AIRCUTTER, m::ARMTHRUST, m::BARRAGE, m::BEATUP, m::BIDE, m::BIND,
    m::BLASTBURN, m::BONERUSH, m::BONEMERANG, m::BOUNCE, m::BRICKBREAK, m::BULLETSEED, m::CLAMP,
    m::COMETPUNCH, m::CRABHAMMER, m::CROSSCHOP, m::CUT, m::DIG, m::DIVE, m::DOUBLEKICK,
    m::DOUBLESLAP, m::DOUBLEEDGE, m::DRAGONBREATH, m::DRAGONCLAW, m::DRAGONRAGE, m::DRILLPECK,
    m::EARTHQUAKE, m::EGGBOMB, m::ENDEAVOR, m::ERUPTION, m::EXPLOSION, m::EXTREMESPEED,
    m::FALSESWIPE, m::FEINTATTACK, m::FIRESPIN, m::FLAIL, m::FLY, m::FRENZYPLANT, m::FRUSTRATION,
    m::FURYATTACK, m::FURYCUTTER, m::FURYSWIPES, m::GUST, m::HIDDENPOWER, m::HIGHJUMPKICK,
    m::HORNATTACK, m::HYDROCANNON, m::HYDROPUMP, m::HYPERBEAM, m::ICEBALL, m::ICICLESPEAR,
    m::JUMPKICK, m::KARATECHOP, m::LEAFBLADE, m::LOWKICK, m::MACHPUNCH, m::MAGICALLEAF,
    m::MAGNITUDE, m::MEGAKICK, m::MEGAPUNCH, m::MEGAHORN, m::METEORMASH, m::MUDSHOT, m::MUDDYWATER,
    m::NIGHTSHADE, m::OUTRAGE, m::OVERHEAT, m::PAYDAY, m::PECK, m::PETALDANCE, m::PINMISSILE,
    m::POISONTAIL, m::POUND, m::PSYCHOBOOST, m::PSYWAVE, m::QUICKATTACK, m::RAGE, m::RAPIDSPIN,
    m::RAZORLEAF, m::RAZORWIND, m::RETURN, m::REVENGE, m::REVERSAL, m::ROCKBLAST, m::ROCKTHROW,
    m::ROLLINGKICK, m::ROLLOUT, m::SANDTOMB, m::SCRATCH, m::SEISMICTOSS, m::SELFDESTRUCT,
    m::SHADOWPUNCH, m::SHOCKWAVE, m::SIGNALBEAM, m::SILVERWIND, m::SKULLBASH, m::SKYATTACK,
    m::SKYUPPERCUT, m::SLAM, m::SLASH, m::SNORE, m::SOLARBEAM, m::SONICBOOM, m::SPIKECANNON,
    m::SPITUP, m::STEELWING, m::STRENGTH, m::STRUGGLE, m::SUBMISSION, m::SURF, m::SWIFT, m::TACKLE,
    m::TAKEDOWN, m::THRASH, m::TRIPLEKICK, m::TWISTER, m::UPROAR, m::VISEGRIP, m::VINEWHIP,
    m::VITALTHROW, m::VOLTTACKLE, m::WATERGUN, m::WATERPULSE, m::WATERFALL, m::WEATHERBALL,
    m::WHIRLPOOL, m::WINGATTACK, m::WRAP,
];

/// `onModifyMove(move)`: `if (affectedByKingsRock.includes(move.id)) { if
/// (!move.secondaries) move.secondaries = []; move.secondaries.push(...) }`.
/// A typed Hidden Power's id is `hiddenpower`.
// PS: data/mods/gen3/items.ts kingsrock.onModifyMove
fn on_modify_move(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let id = b.active_move()?.id;
    if id.is_none() {
        return None;
    }
    let placeholder = b.dex.move_def(id).placeholder_for;
    let id = if placeholder.is_none() { id } else { placeholder };
    if AFFECTED.contains(&id) {
        let mv = b.active_move_mut()?;
        // Serene Grace (priority -2) doubles what is on the list when it
        // runs, so it must not have run yet.
        assert!(!mv.secondary_chances_doubled, "unported: King's Rock after Serene Grace");
        mv.added_secondary = Some(&FLINCH);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifyMove, on_modify_move).priority(-1)]);
