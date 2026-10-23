//! `|request|` payloads, PS's shape.
//!
//! PS: `sim/battle.ts` `getRequests`, `sim/side.ts` `getRequestData` and
//! `emitRequest`, `sim/pokemon.ts` `getMoveRequestData`, `getMoves` and
//! `getSwitchRequestData`.
//!
//! The JSON is byte-exact with PS, so every type documents its keys in PS's
//! insertion order and when each optional key is present. Only the keys gens
//! 1 to 3 produce are modelled: no `teamPreview`, `ally`, `canMegaEvo`,
//! `canZMove`, `canDynamax`, `maxMoves`, `canTerastallize`, and no
//! `ability` / `commanding` / `reviving` / `teraType` per Pokemon (gen 7+).

use crate::ident::{
    AbilityNum, Details, HpStatus, ItemNum, MoveNum, PokemonIdent, SideId, TypeNum,
};
use crate::{BoundedVec, Name, MAX_ACTIVE, MAX_MOVES, MAX_TEAM};

/// One side's request (PS `ChoiceRequest`), sent as
/// `sideupdate\npN\n|request|JSON`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// `{"active":[...],"side":{...}}` then `noCancel`, `update`, `rqid`.
    Move(MoveRequest),
    /// `{"forceSwitch":[true],"side":{...}}` then `noCancel`, `update`, `rqid`.
    Switch(SwitchRequest),
    /// `{"wait":true,"side":{...}}` then `rqid`.
    Wait(WaitRequest),
}

impl Request {
    pub fn side(&self) -> &SideRequest {
        match self {
            Request::Move(r) => &r.side,
            Request::Switch(r) => &r.side,
            Request::Wait(r) => &r.side,
        }
    }

    pub fn side_mut(&mut self) -> &mut SideRequest {
        match self {
            Request::Move(r) => &mut r.side,
            Request::Switch(r) => &mut r.side,
            Request::Wait(r) => &mut r.side,
        }
    }

    /// `rqid`, which the PS server (not the sim) appends last.
    pub fn rqid(&self) -> Option<u32> {
        match self {
            Request::Move(r) => r.rqid,
            Request::Switch(r) => r.rqid,
            Request::Wait(r) => r.rqid,
        }
    }

    /// Whether this side has a choice to make.
    pub fn needs_choice(&self) -> bool {
        !matches!(self, Request::Wait(_))
    }
}

/// A move request. Keys: `active`, `side`, then `noCancel` (when
/// `no_cancel`; PS sets it unless Cancel Mod is on and both sides have a
/// request), `update` (when `update`; PS `emitRequest(..., true)` after an
/// `[Unavailable choice]`), `rqid` (when `Some`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveRequest {
    /// One entry per active position, `side.active` order.
    pub active: BoundedVec<ActiveRequest, MAX_ACTIVE>,
    pub side: SideRequest,
    pub no_cancel: bool,
    pub update: bool,
    pub rqid: Option<u32>,
}

/// A forced-switch request. Keys: `forceSwitch`, `side`, then `noCancel`,
/// `update`, `rqid` as on [`MoveRequest`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchRequest {
    /// One flag per active position (PS `!!pokemon.switchFlag`).
    pub force_switch: BoundedVec<bool, MAX_ACTIVE>,
    pub side: SideRequest,
    pub no_cancel: bool,
    pub update: bool,
    pub rqid: Option<u32>,
}

/// The request of a side with nothing to choose. Keys: `wait` (always
/// `true`), `side`, then `rqid`. PS never sets `noCancel` on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaitRequest {
    pub side: SideRequest,
    pub rqid: Option<u32>,
}

/// One active Pokemon's move data, PS `getMoveRequestData`. Keys: `moves`,
/// then each flag only when true, in the order `maybeDisabled`,
/// `maybeLocked`, `trapped`, `maybeTrapped`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ActiveRequest {
    pub moves: BoundedVec<RequestMove, MAX_MOVES>,
    pub maybe_disabled: bool,
    pub maybe_locked: bool,
    pub trapped: bool,
    pub maybe_trapped: bool,
}

/// One entry of `active[].moves`, PS `getMoves`. Keys in order `move`,
/// `id`, then each `Some` field: `pp`, `maxpp`, `target`, `disabled`. PS
/// produces three shapes: a full slot (all four), a locked move or Gen 1's
/// `Fight` button (`move` and `id` only), and Struggle (`target`
/// `randomNormal` and `disabled` `false`, no PP).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RequestMove {
    pub name: RequestMoveName,
    pub pp: Option<u8>,
    pub maxpp: Option<u8>,
    pub target: Option<MoveTarget>,
    /// PS's `disabled` after `getMoves` resolves `'hidden'`; a plain bool for
    /// every effect gens 1 to 3 have.
    pub disabled: Option<bool>,
}

impl RequestMove {
    /// A full move slot.
    pub fn slot(
        name: RequestMoveName,
        pp: u8,
        maxpp: u8,
        target: MoveTarget,
        disabled: bool,
    ) -> Self {
        Self {
            name,
            pp: Some(pp),
            maxpp: Some(maxpp),
            target: Some(target),
            disabled: Some(disabled),
        }
    }

    /// A locked move (or `Recharge`, or gen 1's `Fight`): name and id only.
    pub fn locked(name: RequestMoveName) -> Self {
        Self { name, pp: None, maxpp: None, target: None, disabled: None }
    }

    /// PS's Struggle entry, given when no move is usable.
    pub fn struggle(struggle: MoveNum) -> Self {
        Self {
            name: RequestMoveName::Move(struggle),
            pp: None,
            maxpp: None,
            target: Some(MoveTarget::RandomNormal),
            disabled: Some(false),
        }
    }
}

/// The `move` and `id` pair of a request move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RequestMoveName {
    /// `move` is the dex name, `id` the dex id.
    Move(MoveNum),
    /// `Hidden Power Fire 70` / `hiddenpower` (gens below 6 append the power).
    HiddenPower { ty: TypeNum, power: u8 },
    /// `Return 102` / `return`: Return and Frustration show their power.
    WithPower { mv: MoveNum, power: u8 },
    /// `Recharge` / `recharge`
    Recharge,
    /// Gen 1's `Fight` / `fight` (frozen, asleep or partially trapped).
    Fight,
}

impl Default for RequestMoveName {
    fn default() -> Self {
        RequestMoveName::Move(MoveNum(0))
    }
}

/// A move's `target`, PS `MoveTarget`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum MoveTarget {
    AdjacentAlly,
    AdjacentAllyOrSelf,
    AdjacentFoe,
    All,
    AllAdjacent,
    AllAdjacentFoes,
    Allies,
    AllySide,
    AllyTeam,
    Any,
    FoeSide,
    #[default]
    Normal,
    RandomNormal,
    Scripted,
    SelfTarget,
}

impl MoveTarget {
    pub const ALL: [MoveTarget; 15] = [
        MoveTarget::AdjacentAlly,
        MoveTarget::AdjacentAllyOrSelf,
        MoveTarget::AdjacentFoe,
        MoveTarget::All,
        MoveTarget::AllAdjacent,
        MoveTarget::AllAdjacentFoes,
        MoveTarget::Allies,
        MoveTarget::AllySide,
        MoveTarget::AllyTeam,
        MoveTarget::Any,
        MoveTarget::FoeSide,
        MoveTarget::Normal,
        MoveTarget::RandomNormal,
        MoveTarget::Scripted,
        MoveTarget::SelfTarget,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            MoveTarget::AdjacentAlly => "adjacentAlly",
            MoveTarget::AdjacentAllyOrSelf => "adjacentAllyOrSelf",
            MoveTarget::AdjacentFoe => "adjacentFoe",
            MoveTarget::All => "all",
            MoveTarget::AllAdjacent => "allAdjacent",
            MoveTarget::AllAdjacentFoes => "allAdjacentFoes",
            MoveTarget::Allies => "allies",
            MoveTarget::AllySide => "allySide",
            MoveTarget::AllyTeam => "allyTeam",
            MoveTarget::Any => "any",
            MoveTarget::FoeSide => "foeSide",
            MoveTarget::Normal => "normal",
            MoveTarget::RandomNormal => "randomNormal",
            MoveTarget::Scripted => "scripted",
            MoveTarget::SelfTarget => "self",
        }
    }

    pub fn parse(s: &str) -> Option<MoveTarget> {
        MoveTarget::ALL.into_iter().find(|t| t.as_str() == s)
    }
}

/// `side` of a request, PS `getRequestData`. Keys: `name`, `id`, `pokemon`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SideRequest {
    pub name: Name,
    pub id: SideId,
    /// PS `side.pokemon` order: the current order, which switches permute
    /// (the active Pokemon first in singles).
    pub pokemon: BoundedVec<SidePokemon, MAX_TEAM>,
}

/// One entry of `side.pokemon`, PS `getSwitchRequestData`. Keys: `ident`,
/// `details`, `condition`, `active`, `stats` (`atk`, `def`, `spa`, `spd`,
/// `spe`), `moves`, `baseAbility`, `item`, `pokeball`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SidePokemon {
    /// Always rendered in the inactive form, `p1: NAME` (PS `fullname`);
    /// `active` is its own key.
    pub ident: PokemonIdent,
    pub details: Details,
    /// PS `getHealth().secret`: exact HP, `0 fnt` when fainted.
    pub condition: HpStatus,
    pub active: bool,
    /// `baseStoredStats` in key order atk, def, spa, spd, spe.
    pub stats: [u16; 5],
    pub moves: BoundedVec<SideMoveId, MAX_MOVES>,
    /// `None` renders `""` (a set with no ability). Gen 1 sets carry
    /// `noability`, which is a real ability id.
    pub base_ability: Option<AbilityNum>,
    /// `None` renders `""`.
    pub item: Option<ItemNum>,
    pub pokeball: Pokeball,
}

/// One id of `side.pokemon[].moves`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SideMoveId {
    /// The dex id (`thunderbolt`).
    Move(MoveNum),
    /// `hiddenpower` plus the type's id (`hiddenpowerfire`; gens below 6 do
    /// not append the power).
    HiddenPower(TypeNum),
    /// `return102`, `frustration102`: the id plus the current power.
    WithPower { mv: MoveNum, power: u8 },
}

impl Default for SideMoveId {
    fn default() -> Self {
        SideMoveId::Move(MoveNum(0))
    }
}

/// `pokeball`: PS `toID(set.pokeball) || 'pokeball'`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Pokeball {
    /// `pokeball`, for a set that names none.
    #[default]
    Default,
    /// The set's ball, by item id.
    Item(ItemNum),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ident::{Gender, SpeciesNum, Status};

    fn mon(slot: u8, active: bool) -> SidePokemon {
        SidePokemon {
            ident: PokemonIdent::inactive(SideId::P1, slot),
            details: Details { species: SpeciesNum(146), level: 73, gender: Gender::N, shiny: false },
            condition: HpStatus::exact(282, 282, None),
            active,
            stats: [218, 204, 255, 255, 204],
            moves: BoundedVec::from_slice(&[SideMoveId::Move(MoveNum(1)), SideMoveId::HiddenPower(TypeNum(9))])
                .unwrap(),
            base_ability: Some(AbilityNum(0)),
            item: None,
            pokeball: Pokeball::Default,
        }
    }

    fn side() -> SideRequest {
        let mut s = SideRequest { name: Name::from_str("A").unwrap(), id: SideId::P1, ..Default::default() };
        s.pokemon.push(mon(0, true)).unwrap();
        s.pokemon.push(mon(1, false)).unwrap();
        s
    }

    #[test]
    fn move_request_shapes() {
        let mut active = ActiveRequest::default();
        active
            .moves
            .push(RequestMove::slot(RequestMoveName::Move(MoveNum(53)), 8, 8, MoveTarget::Normal, false))
            .unwrap();
        active.moves.push(RequestMove::locked(RequestMoveName::Recharge)).unwrap();
        active.moves.push(RequestMove::struggle(MoveNum(165))).unwrap();
        active.trapped = true;
        let req = Request::Move(MoveRequest {
            active: BoundedVec::from_slice(&[active]).unwrap(),
            side: side(),
            no_cancel: false,
            update: false,
            rqid: None,
        });
        assert!(req.needs_choice());
        assert_eq!(req.side().pokemon.len(), 2);
        let Request::Move(m) = &req else { unreachable!() };
        assert_eq!(m.active[0].moves[1].pp, None);
        assert_eq!(m.active[0].moves[2].target, Some(MoveTarget::RandomNormal));
        assert_eq!(m.active[0].moves[2].disabled, Some(false));
    }

    #[test]
    fn switch_and_wait_requests() {
        let mut s = side();
        s.pokemon[0].condition = HpStatus::FAINTED;
        let sw = Request::Switch(SwitchRequest {
            force_switch: BoundedVec::from_slice(&[true]).unwrap(),
            side: s,
            no_cancel: true,
            update: false,
            rqid: Some(3),
        });
        assert_eq!(sw.rqid(), Some(3));
        assert_eq!(sw.side().pokemon[0].condition.status, Some(Status::Fnt));
        let w = Request::Wait(WaitRequest { side: side(), rqid: None });
        assert!(!w.needs_choice());
    }

    #[test]
    fn active_overflow_is_an_error() {
        let mut a: BoundedVec<ActiveRequest, MAX_ACTIVE> = BoundedVec::new();
        a.push(ActiveRequest::default()).unwrap();
        assert!(a.push(ActiveRequest::default()).is_err());
    }

    #[test]
    fn move_targets_parse_back() {
        for t in MoveTarget::ALL {
            assert_eq!(MoveTarget::parse(t.as_str()), Some(t));
        }
        assert_eq!(MoveTarget::SelfTarget.as_str(), "self");
    }
}
