//! Request translation: a PS `|request|` to the seat's options, and the
//! player's answer back to a PS choice.
//!
//! PS: `sim/side.ts` (`getRequestData`, `chooseMove`, `chooseSwitch`),
//! `sim/pokemon.ts` (`getMoveRequestData`, `getMoves`: a locked or
//! Struggle-only mon lists one move without PP; a recharging one lists
//! `Recharge`), `sim/SIM-PROTOCOL.md` "Possible choices" (`move N`,
//! `switch N`, 1-based).
//!
//! [`SeatOptions`] mirrors `mega_blastoise_core::choice_collect::SlotOptions`'
//! public fields (and [`PartyMon`] its `display::PartySlotData`) so the runner
//! (task P2-2) copies them across field for field; the core types cannot be
//! built here because the core sits above this crate. Prompt text, input
//! validation and every screen stay in `choice_collect` / `oled_ctl`
//! (ZERO-DRIFT): this module only says what the request allows.
//!
//! Two numberings meet here:
//! - the device's choice grammar (`battle_input::format_move_choice` /
//!   `format_switch_choice`): `move K` with K the 0-based move slot,
//!   `switch T` with T the 0-based TEAM slot (the order `>player` gave);
//! - PS's: `move N` with N the 1-based move in the request's
//!   `active[0].moves`, `switch N` with N the 1-based position in the
//!   request's `side.pokemon`, which PS reorders on every switch (the active
//!   mon moves to the front).
//!
//! [`SeatOptions`] party arrays are indexed by team slot, and
//! [`choice`] maps team slot to request position.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use ps_dex::Type;
use ps_protocol::{Choice, PokemonIdent, Request, SideId, Status, MAX_TEAM};

use crate::view::BattleView;

/// One party member for the stats and switch screens. Mirrors core's
/// `display::PartySlotData`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartyMon {
    pub name: String,
    pub active: bool,
    pub level: u8,
    pub hp: u16,
    pub max_hp: u16,
    /// PS status id (`"par"`), `None` when healthy or fainted.
    pub status: Option<String>,
    pub atk: u16,
    pub def: u16,
    pub spe: u16,
    /// Special Attack (Gen 1's Special).
    pub spc: u16,
    pub types: Vec<Type>,
    /// Move name, PP, max PP.
    pub moves: Vec<(String, u8, u8)>,
    pub boost_atk: i8,
    pub boost_def: i8,
    pub boost_spe: i8,
    pub boost_spc: i8,
    pub item: Option<String>,
}

/// What one seat may do for one request. Mirrors the public fields of
/// core's `choice_collect::SlotOptions` (minus `is_ai` and `prompt_text`,
/// which the runner and `prompt_fmt` own).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeatOptions {
    /// 1 or 2.
    pub player_num: u8,
    /// `"p1"` / `"p2"`.
    pub player_id: String,
    pub n_moves: usize,
    pub usable: [bool; 4],
    pub trapped: bool,
    /// Team slot of the active mon.
    pub active_slot: Option<usize>,
    /// Per team slot: alive and benched.
    pub party_ok: [bool; 6],
    pub forced_switch: bool,
    /// A choice already made for the player, in the device grammar: a
    /// locked move, Struggle or Recharge is `move 0`.
    pub auto: Option<String>,
    /// By team slot.
    pub party: Vec<PartyMon>,
}

impl SeatOptions {
    fn blank(side: SideId) -> Self {
        Self {
            player_num: side.index() as u8 + 1,
            player_id: side.as_str().to_string(),
            n_moves: 0,
            usable: [false; 4],
            trapped: false,
            active_slot: None,
            party_ok: [true; 6],
            forced_switch: false,
            auto: None,
            party: Vec::new(),
        }
    }
}

/// The seat options for `req`, or `None` for a wait request (nothing to
/// choose). Call after [`BattleView::apply_request`], so the party reflects
/// the request.
pub fn slot_options(view: &BattleView, req: &Request) -> Option<SeatOptions> {
    let side_req = req.side();
    let side = side_req.id;
    let mut s = SeatOptions::blank(side);

    // Party, by team slot.
    let size = side_req.pokemon.iter().map(|p| p.ident.slot as usize + 1).max().unwrap_or(0).min(MAX_TEAM);
    for (slot, ok) in s.party_ok.iter_mut().enumerate() {
        *ok = side_req
            .pokemon
            .iter()
            .find(|p| p.ident.slot as usize == slot)
            .is_some_and(|p| !p.active && p.condition.status != Some(Status::Fnt) && p.condition.hp.cur > 0);
    }
    for slot in 0..size {
        let Some(p) = side_req.pokemon.iter().find(|p| p.ident.slot as usize == slot) else { continue };
        if p.active {
            s.active_slot = Some(slot);
        }
        s.party.push(party_mon(view, PokemonIdent::inactive(side, slot as u8), p.active));
    }

    match req {
        Request::Wait(_) => return None,
        Request::Switch(_) => s.forced_switch = true,
        Request::Move(r) => {
            if let Some(a) = r.active.first() {
                let n = a.moves.len().min(4);
                // A locked move, Struggle or Recharge: one move without PP.
                let locked = n == 1 && a.moves[0].pp.is_none();
                if n == 0 {
                    s.auto = Some(String::from("pass"));
                } else if locked {
                    s.auto = Some(String::from("move 0"));
                } else {
                    s.n_moves = n;
                    for i in 0..n {
                        let m = &a.moves[i];
                        s.usable[i] = m.disabled != Some(true) && m.pp != Some(0);
                    }
                    // PS sends `maybeTrapped` when the foe MIGHT trap (an
                    // unrevealed Shadow Tag); a switch is then allowed and
                    // answered with an `[Unavailable choice]` error and a new
                    // request if it is not. Only a known trap blocks here.
                    s.trapped = a.trapped;
                }
            }
        }
    }
    Some(s)
}

fn party_mon(view: &BattleView, ident: PokemonIdent, active: bool) -> PartyMon {
    let m = view.mon(ident).copied().unwrap_or_default();
    let dex = view.dex();
    let species = m.details.map(|d| d.species);
    let types = species
        .and_then(|sp| dex.species.get(sp.0 as usize))
        .map(|d| d.types.iter().copied().filter(|t| *t != Type::None).collect())
        .unwrap_or_default();
    let stats = m.stats.unwrap_or_default();
    PartyMon {
        name: view.nickname(ident),
        active,
        level: m.details.map(|d| d.level.min(255) as u8).unwrap_or(0),
        hp: if m.fainted { 0 } else { m.hp.cur },
        max_hp: m.hp.max,
        status: m.status.map(|st| st.as_str().to_string()),
        atk: stats[0],
        def: stats[1],
        spe: stats[4],
        spc: stats[2],
        types,
        moves: m
            .moves
            .iter()
            .map(|k| (view.move_slot(k).name, k.pp, k.maxpp))
            .collect(),
        boost_atk: if active { m.boosts[0] } else { 0 },
        boost_def: if active { m.boosts[1] } else { 0 },
        boost_spe: if active { m.boosts[4] } else { 0 },
        boost_spc: if active { m.boosts[2] } else { 0 },
        item: m
            .item
            .and_then(|i| dex.items.get(i.0 as usize))
            .map(|d| d.name.as_str().to_string())
            .filter(|n| !n.is_empty()),
    }
}

/// The PS choice for a device-grammar answer to `req`: `move K` (0-based
/// slot) is `move K+1`; `switch T` (team slot) is `switch P` with P the
/// 1-based position of team slot T in `req`'s `side.pokemon`; `pass` and
/// `default` pass through. `None` for anything else, or a team slot the
/// request does not list.
pub fn choice(req: &Request, answer: &str) -> Option<Choice> {
    let mut words = answer.split_whitespace();
    let verb = words.next()?;
    let n = words.next().and_then(|w| w.parse::<u8>().ok());
    match (verb, n) {
        ("move", Some(k)) => Some(Choice::Move(k.checked_add(1)?)),
        ("switch", Some(team_slot)) => {
            let pos = req.side().pokemon.iter().position(|p| p.ident.slot == team_slot)?;
            Some(Choice::Switch(pos as u8 + 1))
        }
        ("pass", None) => Some(Choice::Pass),
        ("default", None) => Some(Choice::Default),
        _ => None,
    }
}

/// [`choice`] as PS's text: `move 1`, `switch 3`, `pass`, `default`.
pub fn choice_string(req: &Request, answer: &str) -> Option<String> {
    Some(match choice(req, answer)? {
        Choice::Move(n) => alloc::format!("move {n}"),
        Choice::MoveName(name) => alloc::format!("move {}", name.as_str()),
        Choice::Switch(n) => alloc::format!("switch {n}"),
        Choice::SwitchName(name) => alloc::format!("switch {}", name.as_str()),
        Choice::Pass => String::from("pass"),
        Choice::Default => String::from("default"),
        Choice::Undo => String::from("undo"),
    })
}
