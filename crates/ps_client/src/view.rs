//! `BattleView`: what a PS client knows, built only from protocol messages
//! and requests, and the [`BoardEvent`]s each message means.
//!
//! PS: `sim/SIM-PROTOCOL.md` (the line kinds), and the client side of
//! `|split|` (`sim/battle.ts` `addSplit`: a secret line, then a shared one).
//!
//! The view keeps per side: the active mon, every revealed team member's
//! details, HP (exact where the stream gives it), status, boosts and known
//! moves with PP, the active mon's visible volatiles, the side conditions,
//! plus weather and the turn. [`BattleView::apply`] updates it from one
//! [`Msg`] and returns the board events that message means;
//! [`BattleView::apply_request`] learns a side's team from its `|request|`
//! and returns that seat's prompt events.
//!
//! Event mapping (the old runners' vocabulary; see `board.rs`):
//!
//! | PS line | events |
//! |---|---|
//! | `start` | `BattleStart` |
//! | `switch` / `drag` | `SwitchIn` (moves and Speed when a request has told the view) |
//! | `turn` | `Turn` |
//! | `move` | `Move`, then the user's `MovesUpdate` (PP deducted as a PS client does) |
//! | `cant` | `Cant` |
//! | `faint` | `Faint` |
//! | `-damage` | `Damage`; first `Cant{confusion}` for a `[from] confusion` self-hit |
//! | `-heal` | `Heal` |
//! | `-sethp` | `Damage` or `Heal`, by direction |
//! | `-status` / `-curestatus` | `SetStatus` / `CureStatus` |
//! | `-boost` / `-unboost` / `-setboost` | `StatChange` (none for a 0 change; gen 1's `spd` twin of `spa` is folded into "Special") |
//! | `-clearallboost` | `EffectStart{haze}` |
//! | `-crit`, `-supereffective`, `-resisted`, `-immune`, `-miss`, `-fail` | the matching event |
//! | `-start` / `-end` | `EffectStart` / `EffectEnd` |
//! | `-sidestart` / `-sideend` | `EffectStart` / `EffectEnd` on that side's active mon |
//! | `-weather` | `EffectStart` when weather begins, `EffectEnd` when it stops |
//! | `-singleturn`, `-singlemove` | `EffectStart` |
//! | `-activate` `trapped` / a binding move | `EffectStart{trapped}` / `EffectStart{wrap}` |
//! | `-prepare` | `EffectStart{charging}` |
//! | `-transform` | `EffectStart{transform}`, `ActiveMonUpdate`, `MovesUpdate` |
//! | `-formechange`, `detailschange` | `ActiveMonUpdate` |
//! | `win` / `tie` | `Win` / `Tie` |
//!
//! Everything else updates state only.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ps_dex::{Dex, MoveCategory, Type};
use ps_protocol::{
    to_id, AbilityNum, Arg, ArgValue, BattleNames, Details, Health, Hp, ItemNum, Msg, MoveNum, PokemonIdent,
    Request, RequestMoveName, SideId, SideMoveId, SpeciesNum, Stat, Status, TypeNum, Weather,
    Word, MAX_MOVES, MAX_TEAM,
};
use ps_protocol::{BoundedStr, BoundedVec, Name};

use crate::board::{health_field, mon_field, BoardEvent, BoardEvents, MoveSlot, PromptKind};

/// Most visible volatiles tracked on one active mon.
pub const MAX_VOLATILES: usize = 8;
/// Most side conditions tracked on one side.
pub const MAX_SIDE_CONDITIONS: usize = 4;

/// A move the view knows a mon has, with what it knows of its PP.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KnownMove {
    pub mv: MoveNum,
    /// Hidden Power's type, from the request.
    pub ty: Option<TypeNum>,
    /// A request-given base power (Hidden Power, Return, Frustration).
    pub power: Option<u8>,
    pub pp: u8,
    pub maxpp: u8,
}

/// One team member as the view knows it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MonView {
    /// From the last `switch` / `drag` / `detailschange` or request.
    pub details: Option<Details>,
    /// The species shown while transformed or in another forme, if not the
    /// mon's own.
    pub shown_species: Option<SpeciesNum>,
    /// HP as the stream last reported it. `max == 0` until reported.
    pub hp: Hp,
    /// Non-volatile status, `None` when healthy. Never `Fnt`.
    pub status: Option<Status>,
    pub fainted: bool,
    /// PS boost order: atk, def, spa, spd, spe, accuracy, evasion.
    pub boosts: [i8; 7],
    pub moves: BoundedVec<KnownMove, MAX_MOVES>,
    /// The mon's own moves while Transform shows the target's (PS
    /// `baseMoveSlots`, restored on switch-out by `clearVolatile`).
    pub base_moves: Option<BoundedVec<KnownMove, MAX_MOVES>>,
    /// atk, def, spa, spd, spe, from a request.
    pub stats: Option<[u16; 5]>,
    pub item: Option<ItemNum>,
    /// From `-ability` or the request's `baseAbility`.
    pub ability: Option<AbilityNum>,
    /// A protocol line (not only a request) has shown this mon.
    pub revealed: bool,
}

/// One side as the view knows it.
#[derive(Clone, Debug, Default)]
pub struct SideView {
    /// Team slot of the active mon.
    pub active: Option<u8>,
    pub mons: [MonView; MAX_TEAM],
    /// Nicknames by team slot; empty means "not known, use the species".
    pub nicks: [Name; MAX_TEAM],
    /// From `|teamsize|` or the request.
    pub team_size: u8,
    /// Visible volatiles on the active mon (`-start` until `-end` or a switch).
    pub volatiles: [Option<Arg>; MAX_VOLATILES],
    /// `-sidestart` until `-sideend`.
    pub conditions: [Option<Arg>; MAX_SIDE_CONDITIONS],
    /// The active mon's `SwitchIn` went out without its Speed (no request
    /// had arrived yet); the next request sends it as an `ActiveMonUpdate`.
    stale_switch_in: bool,
}

/// Where the view is inside a `|split|` pair (text transport only: the
/// typed path carries both forms in one `Msg`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SplitState {
    #[default]
    None,
    /// `|split|pN` seen: the next HP line of that side is the secret form.
    Secret(SideId),
    /// The secret form applied: the next HP line of that side is its shared
    /// twin, already accounted for.
    Shared(SideId),
}

/// How the battle ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Win(SideId),
    Tie,
}

/// The client's view of one battle.
#[derive(Clone, Debug)]
pub struct BattleView {
    dex: &'static Dex,
    pub sides: [SideView; 2],
    pub turn: u16,
    pub weather: Weather,
    pub started: bool,
    pub outcome: Option<Outcome>,
    split: SplitState,
    hidden_power: Option<MoveNum>,
    pressure: Option<AbilityNum>,
}

fn add_arg<const N: usize>(list: &mut [Option<Arg>; N], a: Arg) {
    if list.iter().any(|x| *x == Some(a)) {
        return;
    }
    if let Some(free) = list.iter_mut().find(|x| x.is_none()) {
        *free = Some(a);
    }
}

fn remove_arg<const N: usize>(list: &mut [Option<Arg>; N], a: Arg) {
    for x in list.iter_mut() {
        if *x == Some(a) {
            *x = None;
        }
    }
}

impl BattleView {
    /// An empty view for a battle played with `dex` (`&ps_dex::GEN1` or
    /// `&ps_dex::GEN3`). Protocol numbers index it: `MoveNum(n)` is the dex's
    /// move `n`, `SpeciesNum(n)` its species `n`, `TypeNum(n)` is
    /// `ps_dex::Type::ALL[n]`.
    pub fn new(dex: &'static Dex) -> Self {
        let pressure = dex.ability_id("pressure").map(|a| AbilityNum(a.0));
        Self {
            dex,
            sides: [SideView::default(), SideView::default()],
            turn: 0,
            weather: Weather::None,
            started: false,
            outcome: None,
            split: SplitState::None,
            hidden_power: dex.move_id("hiddenpower").map(|m| MoveNum(m.0)),
            pressure,
        }
    }

    pub fn dex(&self) -> &'static Dex {
        self.dex
    }

    /// Sets the nickname of team slot `slot` (the `NAME` of `p1a: NAME`).
    pub fn set_nickname(&mut self, side: SideId, slot: u8, nick: &str) {
        if let Some(n) = self.sides[side.index()].nicks.get_mut(slot as usize) {
            *n = BoundedStr::from_str(nick).unwrap_or_default();
        }
    }

    /// Copies every nickname `names` knows for team slots 0 to 5.
    pub fn learn_names(&mut self, names: &dyn BattleNames) {
        for side in [SideId::P1, SideId::P2] {
            for slot in 0..MAX_TEAM as u8 {
                let nick = names.nickname(side, slot);
                if !nick.is_empty() {
                    self.set_nickname(side, slot, nick);
                }
            }
        }
    }

    pub fn side(&self, side: SideId) -> &SideView {
        &self.sides[side.index()]
    }

    pub fn mon(&self, ident: PokemonIdent) -> Option<&MonView> {
        self.sides[ident.side.index()].mons.get(ident.slot as usize)
    }

    /// The active mon of `side`, with its team slot.
    pub fn active(&self, side: SideId) -> Option<(u8, &MonView)> {
        let s = &self.sides[side.index()];
        let slot = s.active?;
        Some((slot, &s.mons[slot as usize]))
    }

    // ---- names ---------------------------------------------------------

    fn species_name(&self, s: SpeciesNum) -> &'static str {
        self.dex.species.get(s.0 as usize).map(|d| d.name.as_str()).unwrap_or("")
    }

    fn move_name(&self, m: MoveNum) -> &'static str {
        self.dex.moves.get(m.0 as usize).map(|d| d.name.as_str()).unwrap_or("")
    }

    fn type_of(t: TypeNum) -> Type {
        Type::ALL.get(t.0 as usize).copied().unwrap_or(Type::None)
    }

    /// The mon's nickname, or its species when no nickname is known (PS's
    /// default nickname is the species name).
    pub fn nickname(&self, ident: PokemonIdent) -> String {
        let s = &self.sides[ident.side.index()];
        let slot = ident.slot as usize;
        match s.nicks.get(slot) {
            Some(n) if !n.is_empty() => n.as_str().to_string(),
            _ => s
                .mons
                .get(slot)
                .and_then(|m| m.details)
                .map(|d| self.species_name(d.species).to_string())
                .unwrap_or_default(),
        }
    }

    /// `"NAME,pN,0"`.
    pub fn mon_str(&self, ident: PokemonIdent) -> String {
        mon_field(&self.nickname(ident), ident.side.as_str())
    }

    fn active_ident(&self, side: SideId) -> Option<PokemonIdent> {
        self.sides[side.index()].active.map(|slot| PokemonIdent::active(side, slot))
    }

    fn active_str(&self, side: SideId) -> String {
        self.active_ident(side).map(|i| self.mon_str(i)).unwrap_or_default()
    }

    fn health_str(&self, ident: PokemonIdent) -> String {
        let m = self.mon(ident).copied().unwrap_or_default();
        health_field(m.hp.cur, m.hp.max)
    }

    /// The display name of an effect or reason argument.
    pub fn arg_name(&self, a: Arg) -> String {
        match a.value() {
            ArgValue::Move(m) | ArgValue::MoveEffect(m) => self.move_name(m).to_string(),
            ArgValue::MoveId(m) => {
                self.dex.moves.get(m.0 as usize).map(|d| id_of(d.name.as_str())).unwrap_or_default()
            }
            ArgValue::Item(i) | ArgValue::ItemEffect(i) => {
                self.dex.items.get(i.0 as usize).map(|d| d.name.as_str()).unwrap_or("").to_string()
            }
            ArgValue::Ability(x) | ArgValue::AbilityEffect(x) => {
                self.dex.abilities.get(x.0 as usize).map(|d| d.name.as_str()).unwrap_or("").to_string()
            }
            ArgValue::Species(s) => self.species_name(s).to_string(),
            ArgValue::Type(t) => Self::type_of(t).name().to_string(),
            ArgValue::TypePair(a, b) => {
                format!("{}/{}", Self::type_of(a).name(), Self::type_of(b).name())
            }
            ArgValue::Status(s) => s.as_str().to_string(),
            ArgValue::Stat(s) => s.abbr().to_string(),
            ArgValue::StatName(s) => s.name().to_string(),
            ArgValue::Word(w) => w.as_str().to_string(),
            ArgValue::Mon(m) => self.nickname(m),
            ArgValue::Side(s) => s.as_str().to_string(),
            ArgValue::Int(n) => n.to_string(),
        }
    }

    /// The `what` of an `EffectStart` / `EffectEnd`: the Gen 1 runner's word
    /// where it had one (so the narration reads as before), else PS's name.
    fn effect_what(&self, a: Arg) -> String {
        let word = match a.value() {
            ArgValue::Word(Word::Confusion) => Some("confusion"),
            ArgValue::Word(Word::TypeChange) => Some("conversion"),
            ArgValue::Word(Word::PartiallyTrapped) => Some("wrap"),
            ArgValue::Move(m) | ArgValue::MoveEffect(m) => {
                match id_of(self.move_name(m)).as_str() {
                    "substitute" => Some("substitute"),
                    "leechseed" => Some("seeded"),
                    "reflect" => Some("reflect"),
                    "lightscreen" => Some("lightscreen"),
                    "mist" => Some("mist"),
                    "focusenergy" => Some("focusenergy"),
                    "bide" => Some("bide"),
                    "disable" => Some("disable"),
                    "mimic" => Some("mimic"),
                    "transform" => Some("transform"),
                    "yawn" => Some("drowsy"),
                    "wrap" | "bind" | "clamp" | "firespin" | "whirlpool" | "sandtomb" => Some("wrap"),
                    _ => None,
                }
            }
            _ => None,
        };
        match word {
            Some(w) => w.to_string(),
            None => self.arg_name(a),
        }
    }

    /// The `stat` of a `StatChange`, as each old engine labelled it.
    fn stat_label(&self, s: Stat) -> &'static str {
        match s {
            Stat::Atk => "Attack",
            Stat::Def => "Defense",
            Stat::Spa if self.dex.gen == 1 => "Special",
            Stat::Spd if self.dex.gen == 1 => "Special",
            Stat::Spa => "Sp. Atk",
            Stat::Spd => "Sp. Def",
            Stat::Spe => "Speed",
            Stat::Accuracy => "accuracy",
            Stat::Evasion => "evasiveness",
        }
    }

    // ---- move slots ----------------------------------------------------

    /// One known move as the board's `MoveSlot`.
    pub fn move_slot(&self, k: &KnownMove) -> MoveSlot {
        let Some(def) = self.dex.moves.get(k.mv.0 as usize) else {
            return MoveSlot {
                name: String::new(),
                type_name: String::new(),
                category: String::new(),
                power: None,
                accuracy: None,
                pp: k.pp,
                max_pp: k.maxpp,
            };
        };
        let ty = k.ty.map(Self::type_of).unwrap_or(def.type_);
        // Gens 1 to 3 split physical and special by type (PS's gen 3 mod
        // rewrites every move's category at load); a request-given Hidden
        // Power type needs the same rule.
        let category = if self.dex.gen <= 3 && def.category != MoveCategory::Status {
            match ty {
                Type::Fire | Type::Water | Type::Grass | Type::Electric | Type::Ice | Type::Psychic
                | Type::Dark | Type::Dragon => MoveCategory::Special,
                _ => MoveCategory::Physical,
            }
        } else {
            def.category
        };
        let power = k.power.map(u32::from).or(if def.base_power > 0 {
            Some(def.base_power as u32)
        } else {
            None
        });
        MoveSlot {
            name: def.name.as_str().to_string(),
            type_name: ty.name().to_string(),
            category: category.name().to_string(),
            power,
            accuracy: if def.accuracy == 0 { None } else { Some(def.accuracy) },
            pp: k.pp,
            max_pp: k.maxpp,
        }
    }

    /// The board's move list for one mon.
    pub fn move_slots(&self, ident: PokemonIdent) -> Vec<MoveSlot> {
        match self.mon(ident) {
            Some(m) => m.moves.iter().map(|k| self.move_slot(k)).collect(),
            None => Vec::new(),
        }
    }

    /// The synthetic single slot a recharging mon shows (the Gen 1 runner's).
    pub fn recharge_slot() -> MoveSlot {
        MoveSlot {
            name: "Recharge".to_string(),
            type_name: "Normal".to_string(),
            category: MoveCategory::Status.name().to_string(),
            power: None,
            accuracy: None,
            pp: 1,
            max_pp: 1,
        }
    }

    fn moves_update(&self, ident: PokemonIdent, out: &mut BoardEvents) {
        let moves = self.move_slots(ident);
        if !moves.is_empty() {
            out.push(BoardEvent::MovesUpdate { player_id: ident.side.as_str().to_string(), moves });
        }
    }

    // ---- state helpers -------------------------------------------------

    fn mon_mut(&mut self, ident: PokemonIdent) -> Option<&mut MonView> {
        self.sides[ident.side.index()].mons.get_mut(ident.slot as usize)
    }

    /// Applies an HP-bearing line's health: the secret (exact) form when the
    /// message carries both.
    fn set_health(&mut self, ident: PokemonIdent, h: &Health) {
        let hp = if h.split { h.secret } else { h.shared };
        let Some(m) = self.mon_mut(ident) else { return };
        m.revealed = true;
        if h.status == Some(Status::Fnt) || (hp.cur == 0 && hp.max == 0) {
            m.fainted = true;
            m.hp.cur = 0;
            m.status = None;
        } else {
            m.fainted = false;
            m.hp = hp;
            m.status = h.status;
        }
    }

    /// Text transport: is this HP line the shared twin of a secret line just
    /// applied? Advances the split state either way.
    fn split_twin(&mut self, side: SideId) -> bool {
        match self.split {
            SplitState::Secret(s) if s == side => {
                self.split = SplitState::Shared(s);
                false
            }
            SplitState::Shared(s) if s == side => {
                self.split = SplitState::None;
                true
            }
            _ => {
                self.split = SplitState::None;
                false
            }
        }
    }

    /// `baton`: the line carries `[from] Baton Pass` (PS `switchIn` writes
    /// its `sourceEffect`; `copyVolatileFrom` runs for a `selfSwitch:
    /// 'copyvolatile'` move, which in gens 1 to 3 is Baton Pass alone), so
    /// boosts and volatiles pass to the new mon.
    fn switch_in(
        &mut self,
        ident: PokemonIdent,
        details: Details,
        health: &Health,
        baton: bool,
        out: &mut BoardEvents,
    ) {
        let side = ident.side;
        let s = &mut self.sides[side.index()];
        let mut passed = [0i8; 7];
        if let Some(old) = s.active {
            // PS: switching out clears boosts and volatiles.
            if let Some(m) = s.mons.get_mut(old as usize) {
                passed = m.boosts;
                m.boosts = [0; 7];
                m.shown_species = None;
                if let Some(base) = m.base_moves.take() {
                    m.moves = base;
                }
            }
        }
        if !baton {
            s.volatiles = [None; MAX_VOLATILES];
            passed = [0; 7];
        }
        s.active = Some(ident.slot);
        if let Some(m) = s.mons.get_mut(ident.slot as usize) {
            m.details = Some(details);
            m.boosts = passed;
            m.shown_species = None;
        }
        self.set_health(ident, health);
        let m = self.mon(ident).copied().unwrap_or_default();
        let speed = m.stats.map(|st| st[4]);
        self.sides[side.index()].stale_switch_in = speed.is_none();
        out.push(BoardEvent::SwitchIn {
            name: self.nickname(ident),
            species: Some(self.species_name(details.species).to_string()),
            player_id: Some(side.as_str().to_string()),
            team_slot: Some(ident.slot),
            moves: self.move_slots(ident),
            speed,
        });
    }

    fn boost(&mut self, ident: PokemonIdent, stat: Stat, delta: i8, out: &mut BoardEvents) {
        if let Some(m) = self.mon_mut(ident) {
            let b = &mut m.boosts[stat as usize];
            *b = (*b + delta).clamp(-6, 6);
        }
        // Gen 1 Special is one stat that PS reports as `spa` then `spd`; the
        // Gen 1 runner said it once.
        if delta == 0 || (self.dex.gen == 1 && stat == Stat::Spd) {
            return;
        }
        out.push(BoardEvent::StatChange {
            mon: self.mon_str(ident),
            stat: self.stat_label(stat).to_string(),
            delta,
        });
    }

    /// PS client `rememberMove`: a move used on its own (no `[from]`) costs
    /// one PP, one more when it targets a foe known to have Pressure (PS:
    /// `useMoveInner`, `pressureTargets`). A Pressure the view has not been
    /// told of, and `pressureTarget: 'foeSide'` moves (Spikes), are not
    /// counted; the next request corrects the PP either way.
    fn deduct_pp(&mut self, ident: PokemonIdent, mv: MoveNum, target: Option<PokemonIdent>) {
        let pressure = target.is_some_and(|t| {
            t.side != ident.side && self.pressure.is_some() && self.mon(t).and_then(|m| m.ability) == self.pressure
        });
        let cost = if pressure { 2 } else { 1 };
        // `|move|` names Hidden Power plainly; the request may list it as
        // its typed entry (`Hidden Power Flying`).
        let dex = self.dex;
        let Some(i) = self.mon(ident).and_then(|m| m.moves.iter().position(|k| same_move(dex, k.mv, mv))) else {
            return;
        };
        if let Some(m) = self.mon_mut(ident) {
            m.moves[i].pp = m.moves[i].pp.saturating_sub(cost);
        }
    }

    // ---- the protocol --------------------------------------------------

    /// Updates the view from one protocol message and returns the board
    /// events it means (often none).
    pub fn apply(&mut self, msg: &Msg) -> BoardEvents {
        let mut out = BoardEvents::new();
        // A `|split|` covers exactly the next two HP lines of its side.
        let hp_side = match msg {
            Msg::Switch { mon, .. }
            | Msg::Drag { mon, .. }
            | Msg::Damage { mon, .. }
            | Msg::Heal { mon, .. }
            | Msg::SetHp { mon, .. } => Some(mon.side),
            _ => None,
        };
        match (msg, hp_side) {
            (Msg::Split(side), _) => {
                self.split = SplitState::Secret(*side);
                return out;
            }
            (_, Some(side)) => {
                if self.split_twin(side) {
                    return out;
                }
            }
            _ => self.split = SplitState::None,
        }

        match *msg {
            Msg::TeamSize { side, size } => self.sides[side.index()].team_size = size,
            Msg::Start => {
                self.started = true;
                out.push(BoardEvent::BattleStart);
            }
            Msg::Turn(n) => {
                self.turn = n;
                out.push(BoardEvent::Turn { n: n as u32 });
            }
            Msg::Win(side) => {
                self.outcome = Some(Outcome::Win(side));
                out.push(BoardEvent::Win { side: Some(side.index().to_string()) });
            }
            Msg::Tie => {
                self.outcome = Some(Outcome::Tie);
                out.push(BoardEvent::Tie);
            }

            Msg::Switch { mon, details, health, from } | Msg::Drag { mon, details, health, from } => {
                let baton = from.is_some_and(|a| match a.value() {
                    ArgValue::Move(m) | ArgValue::MoveEffect(m) => id_of(self.move_name(m)) == "batonpass",
                    _ => false,
                });
                self.switch_in(mon, details, &health, baton, &mut out);
            }
            Msg::DetailsChange { mon, details, .. } => {
                if let Some(m) = self.mon_mut(mon) {
                    m.details = Some(details);
                    m.shown_species = None;
                }
                self.active_mon_update(mon, details.species, &mut out);
            }
            Msg::FormeChange { mon, species, .. } => {
                if let Some(m) = self.mon_mut(mon) {
                    m.shown_species = Some(species);
                }
                self.active_mon_update(mon, species, &mut out);
            }

            Msg::Move { user, mv, target, kw } => {
                if kw.from.is_none() {
                    self.deduct_pp(user, mv, target);
                }
                out.push(BoardEvent::Move {
                    user: Some(self.nickname(user)),
                    player_id: Some(user.side.as_str().to_string()),
                    name: self.move_name(mv).to_string(),
                });
                self.moves_update(user, &mut out);
            }
            Msg::Cant { mon, reason, .. } => {
                out.push(BoardEvent::Cant { mon: self.mon_str(mon), reason: self.cant_reason(reason) });
            }
            Msg::Faint(mon) => {
                if let Some(m) = self.mon_mut(mon) {
                    m.fainted = true;
                    m.hp.cur = 0;
                    m.status = None;
                }
                out.push(BoardEvent::Faint { mon: self.mon_str(mon), team_slot: Some(mon.slot) });
            }

            Msg::Damage { mon, health, kw } => {
                self.set_health(mon, &health);
                // The Gen 1 runner narrated a confusion self-hit as a `cant`
                // before its damage; PS says it with `[from] confusion`.
                if kw.from.map(|a| a.value()) == Some(ArgValue::Word(Word::Confusion)) {
                    out.push(BoardEvent::Cant { mon: self.mon_str(mon), reason: "confusion".to_string() });
                }
                out.push(BoardEvent::Damage { mon: self.mon_str(mon), health: self.health_str(mon) });
            }
            Msg::Heal { mon, health, .. } => {
                self.set_health(mon, &health);
                out.push(BoardEvent::Heal { mon: self.mon_str(mon), health: self.health_str(mon) });
            }
            Msg::SetHp { mon, health, .. } => {
                let before = self.mon(mon).map(|m| m.hp.cur).unwrap_or(0);
                self.set_health(mon, &health);
                let after = self.mon(mon).map(|m| m.hp.cur).unwrap_or(0);
                let (m, h) = (self.mon_str(mon), self.health_str(mon));
                out.push(if after < before {
                    BoardEvent::Damage { mon: m, health: h }
                } else {
                    BoardEvent::Heal { mon: m, health: h }
                });
            }
            Msg::SetStatus { mon, status, .. } => {
                if let Some(m) = self.mon_mut(mon) {
                    m.status = Some(status);
                }
                out.push(BoardEvent::SetStatus { mon: self.mon_str(mon), status: status.as_str().to_string() });
            }
            Msg::CureStatus { mon, status, .. } => {
                if let Some(m) = self.mon_mut(mon) {
                    m.status = None;
                }
                out.push(BoardEvent::CureStatus { mon: self.mon_str(mon), status: status.as_str().to_string() });
            }
            Msg::CureTeam { mon, .. } => {
                for m in self.sides[mon.side.index()].mons.iter_mut() {
                    m.status = None;
                }
            }

            Msg::Boost { mon, stat, amount, .. } => self.boost(mon, stat, amount as i8, &mut out),
            Msg::Unboost { mon, stat, amount, .. } => self.boost(mon, stat, -(amount as i8), &mut out),
            Msg::SetBoost { mon, stat, amount, .. } => {
                let old = self.mon(mon).map(|m| m.boosts[stat as usize]).unwrap_or(0);
                self.boost(mon, stat, amount - old, &mut out);
            }
            Msg::ClearAllBoost { .. } => {
                for s in self.sides.iter_mut() {
                    for m in s.mons.iter_mut() {
                        m.boosts = [0; 7];
                    }
                }
                out.push(BoardEvent::EffectStart { mon: String::new(), what: "haze".to_string(), detail: None });
            }
            Msg::ClearBoost { mon, .. } => {
                if let Some(m) = self.mon_mut(mon) {
                    m.boosts = [0; 7];
                }
            }
            Msg::ClearNegativeBoost { mon, .. } => {
                if let Some(m) = self.mon_mut(mon) {
                    for b in m.boosts.iter_mut() {
                        *b = (*b).max(0);
                    }
                }
            }
            Msg::CopyBoost { source, target, .. } => {
                let b = self.mon(source).map(|m| m.boosts).unwrap_or_default();
                if let Some(m) = self.mon_mut(target) {
                    m.boosts = b;
                }
            }

            Msg::Crit(mon) => out.push(BoardEvent::CriticalHit { mon: self.mon_str(mon) }),
            Msg::SuperEffective(mon) => out.push(BoardEvent::SuperEffective { mon: self.mon_str(mon) }),
            Msg::Resisted(mon) => out.push(BoardEvent::Resisted { mon: self.mon_str(mon) }),
            Msg::Immune { mon, .. } => out.push(BoardEvent::Immune { mon: self.mon_str(mon) }),
            Msg::Miss { source, target } => {
                // The board's miss names the DEFENDER ("The attack missed
                // X!"); gen 1's `-miss` names only the attacker, whose one
                // foe is the defender.
                let mon = match target {
                    Some(t) => self.mon_str(t),
                    None => self.active_str(source.side.foe()),
                };
                out.push(BoardEvent::Miss { mon });
            }
            Msg::Fail { mon, .. } => out.push(BoardEvent::Fail { mon: self.mon_str(mon) }),
            Msg::NoTarget(Some(mon)) => out.push(BoardEvent::Fail { mon: self.mon_str(mon) }),

            Msg::StartEffect { mon, effect, a, .. } => {
                let what = self.effect_what(effect);
                if what != "transform" {
                    if let Some(s) = self.sides.get_mut(mon.side.index()) {
                        add_arg(&mut s.volatiles, effect);
                    }
                }
                let detail = a.map(|x| self.arg_name(x));
                if what == "mimic" {
                    self.mimic(mon, a);
                }
                out.push(BoardEvent::EffectStart { mon: self.mon_str(mon), what: what.clone(), detail });
                if what == "mimic" {
                    self.moves_update(mon, &mut out);
                }
            }
            Msg::EndEffect { mon, effect, .. } => {
                remove_arg(&mut self.sides[mon.side.index()].volatiles, effect);
                out.push(BoardEvent::EffectEnd { mon: self.mon_str(mon), what: self.effect_what(effect) });
            }
            Msg::SingleTurn { mon, effect, .. } | Msg::SingleMove { mon, effect, .. } => {
                out.push(BoardEvent::EffectStart { mon: self.mon_str(mon), what: self.arg_name(effect), detail: None });
            }
            Msg::Prepare { mon, mv, .. } => {
                out.push(BoardEvent::EffectStart {
                    mon: self.mon_str(mon),
                    what: "charging".to_string(),
                    detail: Some(self.move_name(mv).to_string()),
                });
            }
            Msg::Transform { mon, into, .. } => self.transform(mon, into, &mut out),

            Msg::SideStart { side, effect, .. } => {
                add_arg(&mut self.sides[side.index()].conditions, effect);
                out.push(BoardEvent::EffectStart { mon: self.active_str(side), what: self.effect_what(effect), detail: None });
            }
            Msg::SideEnd { side, effect, .. } => {
                remove_arg(&mut self.sides[side.index()].conditions, effect);
                out.push(BoardEvent::EffectEnd { mon: self.active_str(side), what: self.effect_what(effect) });
            }
            Msg::Weather { weather, kw } => {
                let upkeep = kw.flags.contains(ps_protocol::KwFlags::UPKEEP);
                let before = self.weather;
                self.weather = weather;
                if weather == Weather::None {
                    if before != Weather::None {
                        out.push(BoardEvent::EffectEnd { mon: String::new(), what: weather_label(before).to_string() });
                    }
                } else if !upkeep {
                    out.push(BoardEvent::EffectStart {
                        mon: String::new(),
                        what: weather_label(weather).to_string(),
                        detail: None,
                    });
                }
            }

            Msg::Activate { mon: Some(mon), effect, .. } => {
                // Two `-activate`s start a condition the Gen 3 runner
                // narrated (PS `data/conditions.ts`): `trapped` `onStart`
                // (Mean Look, Spider Web, Block) and `partiallytrapped`
                // `onStart` (`move: Bind|[of] SOURCE`). The rest (confusion's
                // per-turn notice, Struggle, a Substitute taking a hit, a
                // Protect blocking) the old runners did not narrate.
                let what = match effect.value() {
                    ArgValue::Word(Word::Trapped) => Some("trapped".to_string()),
                    ArgValue::Move(_) | ArgValue::MoveEffect(_) => {
                        Some(self.effect_what(effect)).filter(|w| w == "wrap")
                    }
                    _ => None,
                };
                if let Some(what) = what {
                    out.push(BoardEvent::EffectStart { mon: self.mon_str(mon), what, detail: None });
                }
            }
            Msg::Item { mon, item, .. } => {
                if let Some(m) = self.mon_mut(mon) {
                    m.item = Some(item);
                }
            }
            Msg::Ability { mon, ability, .. } => {
                if let Some(m) = self.mon_mut(mon) {
                    m.ability = Some(ability);
                }
            }
            Msg::EndItem { mon, .. } => {
                if let Some(m) = self.mon_mut(mon) {
                    m.item = None;
                }
            }

            // State the board does not show, or lines with no board meaning.
            Msg::Spacer
            | Msg::Timestamp(_)
            | Msg::GameType(_)
            | Msg::Player(_)
            | Msg::Split(_)
            | Msg::Gen(_)
            | Msg::Tier(_)
            | Msg::Rule(_)
            | Msg::Raw(_)
            | Msg::Upkeep
            | Msg::Debug(_)
            | Msg::Error { .. }
            | Msg::Activate { mon: None, .. }
            | Msg::Block { .. }
            | Msg::NoTarget(None)
            | Msg::HitCount { .. }
            | Msg::Ohko
            | Msg::Nothing
            | Msg::MustRecharge(_)
            | Msg::Anim { .. }
            | Msg::FieldStart { .. }
            | Msg::FieldEnd { .. }
            | Msg::FieldActivate { .. }
            | Msg::EndAbility { .. }
            | Msg::Message(_)
            | Msg::Notice(_)
            | Msg::BigError(_)
            | Msg::Hint { .. } => {}
        }
        out
    }

    /// `Cant::reason` in the old runners' words.
    fn cant_reason(&self, reason: Arg) -> String {
        match reason.value() {
            ArgValue::Status(s) => s.as_str().to_string(),
            ArgValue::Word(Word::PartiallyTrapped) => "trapped".to_string(),
            ArgValue::Word(w) => w.as_str().to_string(),
            ArgValue::Move(m) | ArgValue::MoveEffect(m) => match id_of(self.move_name(m)).as_str() {
                "disable" => "disabled".to_string(),
                "attract" => "love".to_string(),
                id => id.to_string(),
            },
            _ => id_of(&self.arg_name(reason)),
        }
    }

    fn active_mon_update(&mut self, mon: PokemonIdent, species: SpeciesNum, out: &mut BoardEvents) {
        if self.sides[mon.side.index()].active != Some(mon.slot) {
            return;
        }
        let speed = self.mon(mon).and_then(|m| m.stats).map(|s| s[4]);
        out.push(BoardEvent::ActiveMonUpdate {
            mon: self.mon_str(mon),
            name: self.species_name(species).to_string(),
            speed,
        });
    }

    /// `-start|MON|Mimic|MOVE`: the Mimic slot becomes the copied move,
    /// keeping its PP (PS gen 1 and 3 `mimic` `onHit`).
    fn mimic(&mut self, mon: PokemonIdent, a: Option<Arg>) {
        let Some(ArgValue::Move(copied)) = a.map(|x| x.value()) else { return };
        let mimic = self.dex.move_id("mimic").map(|m| MoveNum(m.0));
        if let Some(m) = self.mon_mut(mon) {
            if let Some(k) = m.moves.iter_mut().find(|k| Some(k.mv) == mimic) {
                k.mv = copied;
                k.ty = None;
                k.power = None;
            }
        }
    }

    /// `-transform|MON|TARGET`: PS `transformInto` copies the target's
    /// species, boosts and moves; each copied move gets `min(5, move.pp)` PP
    /// and, before gen 5, `calculatePP(move, this.ppUps[i] || 0)` max PP,
    /// where `ppUps` are the TRANSFORMER's own (3 per own move, PS's
    /// default; 0 past its move count). Board: the Gen 1 runner's `start` +
    /// `activemon` + move refresh.
    fn transform(&mut self, mon: PokemonIdent, into: PokemonIdent, out: &mut BoardEvents) {
        let target = self.mon(into).copied().unwrap_or_default();
        let species = target.shown_species.or(target.details.map(|d| d.species)).unwrap_or_default();
        let dex = self.dex;
        if let Some(m) = self.mon_mut(mon) {
            m.shown_species = Some(species);
            m.boosts = target.boosts;
            let own = m.base_moves.unwrap_or(m.moves);
            let mut moves = target.moves;
            for (i, k) in moves.iter_mut().enumerate() {
                let base = dex.moves.get(k.mv.0 as usize).map(|d| d.pp).unwrap_or(0);
                k.pp = base.min(5);
                k.maxpp = calculate_pp(dex, k.mv, if i < own.len() { 3 } else { 0 });
            }
            m.base_moves = Some(own);
            m.moves = moves;
        }
        let name = self.species_name(species).to_string();
        out.push(BoardEvent::EffectStart {
            mon: self.mon_str(mon),
            what: "transform".to_string(),
            detail: Some(name.clone()),
        });
        out.push(BoardEvent::ActiveMonUpdate {
            mon: self.mon_str(mon),
            name,
            speed: target.stats.map(|s| s[4]),
        });
        self.moves_update(mon, out);
    }

    // ---- requests ------------------------------------------------------

    /// Learns a side's team from its `|request|` (HP, status, stats, moves
    /// and, for the active mon, PP) and returns that seat's prompt events:
    /// an `ActiveMonUpdate` if its active mon's switch-in went out before
    /// the view knew its Speed, then for a move request the active mon's
    /// `MovesUpdate`, then the `Prompt` (none for a wait request). This is
    /// the Gen 1 runner's per-seat prompt sequence.
    pub fn apply_request(&mut self, req: &Request) -> BoardEvents {
        let mut out = BoardEvents::new();
        let side_req = req.side();
        let side = side_req.id;
        let hp_move = self.hidden_power;
        let dex = self.dex;
        {
            let s = &mut self.sides[side.index()];
            s.team_size = s.team_size.max(side_req.pokemon.len() as u8);
            for p in side_req.pokemon.iter() {
                let Some(m) = s.mons.get_mut(p.ident.slot as usize) else { continue };
                m.details = Some(p.details);
                m.stats = Some(p.stats);
                m.item = p.item;
                if p.base_ability.is_some() {
                    m.ability = p.base_ability;
                }
                let c = p.condition;
                if c.status == Some(Status::Fnt) || c.hp.max == 0 {
                    m.fainted = true;
                    m.hp.cur = 0;
                    m.status = None;
                } else {
                    m.fainted = false;
                    m.hp = c.hp;
                    m.status = c.status;
                }
                // The side list names the moves but not their PP: keep what
                // the view knows of a move it already had, else assume full
                // PP with PS's default 3 PP Ups (`Pokemon` constructor).
                let mut moves: BoundedVec<KnownMove, MAX_MOVES> = BoundedVec::new();
                for (i, id) in p.moves.iter().enumerate() {
                    let (mv, ty, power) = match *id {
                        SideMoveId::Move(mv) => (mv, None, None),
                        SideMoveId::HiddenPower(t) => (hp_move.unwrap_or_default(), Some(t), None),
                        SideMoveId::WithPower { mv, power } => (mv, None, Some(power)),
                    };
                    let known = m.moves.get(i).copied().filter(|k| same_move(dex, k.mv, mv));
                    let k = match known {
                        Some(mut k) => {
                            k.ty = ty.or(k.ty);
                            k.power = power.or(k.power);
                            k
                        }
                        None => {
                            let maxpp = calculate_pp(dex, mv, 3);
                            KnownMove { mv, ty, power, pp: maxpp, maxpp }
                        }
                    };
                    let _ = moves.push(k);
                }
                // PS `getSwitchRequestData` lists `moves`, the current slots,
                // so a transformed mon's list is the copied one; its own come
                // back from `base_moves` when it switches out.
                m.moves = moves;
                if p.active {
                    s.active = Some(p.ident.slot);
                }
            }
        }

        let Some(active) = self.active_ident(side) else {
            if let Request::Switch(_) = req {
                out.push(BoardEvent::Prompt { player_id: side.as_str().to_string(), kind: PromptKind::ChooseSwitch });
            }
            return out;
        };

        let mut recharge = false;
        if let Request::Move(r) = req {
            if let Some(a) = r.active.first() {
                recharge = a.moves.len() == 1 && a.moves[0].name == RequestMoveName::Recharge;
                // Exact PP for the active mon (absent on a locked or
                // Struggle-only request, which lists one move without PP).
                if a.moves.iter().all(|x| x.pp.is_some()) && !a.moves.is_empty() {
                    let mut moves: BoundedVec<KnownMove, MAX_MOVES> = BoundedVec::new();
                    for x in a.moves.iter() {
                        let (mv, ty, power) = match x.name {
                            RequestMoveName::Move(mv) => (mv, None, None),
                            RequestMoveName::HiddenPower { ty, power } => {
                                (hp_move.unwrap_or_default(), Some(ty), Some(power))
                            }
                            RequestMoveName::WithPower { mv, power } => (mv, None, Some(power)),
                            RequestMoveName::Recharge | RequestMoveName::Fight => continue,
                        };
                        let _ = moves.push(KnownMove {
                            mv,
                            ty,
                            power,
                            pp: x.pp.unwrap_or(0),
                            maxpp: x.maxpp.unwrap_or(0),
                        });
                    }
                    if let Some(m) = self.mon_mut(active) {
                        m.moves = moves;
                    }
                }
            }
        }

        if self.sides[side.index()].stale_switch_in {
            if let Some(speed) = self.mon(active).and_then(|m| m.stats).map(|s| s[4]) {
                self.sides[side.index()].stale_switch_in = false;
                out.push(BoardEvent::ActiveMonUpdate {
                    mon: self.mon_str(active),
                    name: self.nickname(active),
                    speed: Some(speed),
                });
            }
        }

        let pid = side.as_str().to_string();
        match req {
            Request::Move(_) => {
                if recharge {
                    out.push(BoardEvent::MovesUpdate { player_id: pid.clone(), moves: alloc::vec![Self::recharge_slot()] });
                } else {
                    self.moves_update(active, &mut out);
                }
                out.push(BoardEvent::Prompt { player_id: pid, kind: PromptKind::ChooseMove });
            }
            Request::Switch(_) => out.push(BoardEvent::Prompt { player_id: pid, kind: PromptKind::ChooseSwitch }),
            Request::Wait(_) => {}
        }
        out
    }
}

/// The same move slot: equal ids, or two forms of Hidden Power (`|move|`
/// and a request's `active` list say `hiddenpower`, the side list
/// `hiddenpowerice`).
fn same_move(dex: &Dex, a: MoveNum, b: MoveNum) -> bool {
    let hp = |m: MoveNum| dex.moves.get(m.0 as usize).is_some_and(|d| d.name.as_str().starts_with("Hidden Power"));
    a == b || (hp(a) && hp(b))
}

/// PS `Battle.calculatePP`: base PP raised by `pp_ups` fifths, except
/// that gens 1 and 2 give 40-PP moves one less per PP Up (61, not 64), and
/// `noPPBoosts` moves take none. The constructor's default is 3 PP Ups.
fn calculate_pp(dex: &Dex, mv: MoveNum, pp_ups: u8) -> u8 {
    let Some(def) = dex.moves.get(mv.0 as usize) else { return 0 };
    if def.bools.has("noPPBoosts") {
        return def.pp;
    }
    let mut pp = def.pp as u16 * (5 + pp_ups as u16) / 5;
    if dex.gen <= 2 && def.pp == 40 {
        pp -= pp_ups as u16;
    }
    pp as u8
}

/// PS `toID`.
pub(crate) fn id_of(s: &str) -> String {
    to_id::<64>(s).map(|b| b.as_str().to_string()).unwrap_or_default()
}

/// The Gen 3 runner's weather words.
fn weather_label(w: Weather) -> &'static str {
    match w {
        Weather::RainDance => "rain",
        Weather::SunnyDay => "harsh sunlight",
        Weather::Sandstorm => "sandstorm",
        Weather::Hail => "hail",
        Weather::None => "",
    }
}
