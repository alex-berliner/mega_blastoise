//! The generic runner: one battle loop for every generation, over a
//! [`Transport`] to a Showdown battle server (design section 7).
//!
//! The loop: send the commands, read what comes back, put every message
//! through the client's [`BattleView`] and hand the [`BoardEvent`]s it
//! yields to the board; on a request, the client's [`slot_options`]
//! become the neutral [`SlotOptions`] on the [`InputBus`], and each answer
//! goes back through [`ps_client::choice`] as a `>pN` command. Gen 1 and
//! Gen 3 share it because both speak the same protocol.
//!
//! Nothing here decides what the player sees or presses (ZERO-DRIFT,
//! `CLAUDE.md`): the client only translates, `choice_collect` owns input and
//! `oled_ctl` owns the screens, exactly as with the legacy runners.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::future::Future;

use embassy_futures::select::{select, Either};
use ps_client::{slot_options, BattleView, KnownMove, MoveSlot, SeatOptions};
use ps_protocol::{
    Choice, Command, FormatSpec, MoveNum, Name, PackedTeam, PokemonIdent, Request, RequestMoveName, SideId,
};

use crate::battle_effects::BoardEffects;
use crate::battle_input::{ActivePrompt, InputBus};
use crate::board_event::BoardEvent;
use crate::choice_collect::SlotOptions;
use crate::display::PartySlotData;
use crate::transport::{Incoming, Transport};

/// What `>start` and the two `>player` commands carry.
#[derive(Clone, Debug)]
pub struct BattleSetup {
    pub format: FormatSpec,
    /// PS's four 16-bit seed words (a numeric seed: `Gen5RNG`).
    pub seed: [u16; 4],
    /// Player names, p1 then p2.
    pub names: [&'static str; 2],
    /// Teams, p1 then p2, in `>player` order.
    pub teams: [PackedTeam; 2],
}

impl BattleSetup {
    /// The local formats: Gen 1's harness format (`gen1customgame` with
    /// Sleep and Freeze Clause) or `gen3customgame`, the two the protocol
    /// diff checks `ps_sim` against. Gen 1 plays the default rule set:
    /// `Cartridge Trunc` is in effect (no `!Cartridge Trunc` repeal).
    pub fn local(gen: u8, seed: u64, teams: [PackedTeam; 2]) -> Self {
        BattleSetup {
            format: if gen == 1 { FormatSpec::gen1_harness() } else { FormatSpec::gen3_harness() },
            seed: seed_words(seed),
            names: ["White", "Red"],
            teams,
        }
    }

    pub fn dex(&self) -> &'static ps_dex::Dex {
        if self.format.base.gen() == 1 {
            &ps_dex::GEN1
        } else {
            &ps_dex::GEN3
        }
    }
}

/// A 64-bit device seed as PS's four seed words, high word first.
pub fn seed_words(seed: u64) -> [u16; 4] {
    [(seed >> 48) as u16, (seed >> 32) as u16, (seed >> 16) as u16, seed as u16]
}

/// The client's seat options as the collector's [`SlotOptions`], field for
/// field. `prompt_text` stays empty, as the Gen 3 runner leaves it.
pub fn slot_options_from_seat(seat: SeatOptions) -> SlotOptions {
    let mut s = SlotOptions::blank(seat.player_num, seat.player_id);
    s.n_moves = seat.n_moves;
    s.usable = seat.usable;
    s.trapped = seat.trapped;
    s.active_slot = seat.active_slot;
    s.party_ok = seat.party_ok;
    s.forced_switch = seat.forced_switch;
    s.auto = seat.auto;
    s.party = seat
        .party
        .into_iter()
        .map(|p| PartySlotData {
            name: p.name,
            active: p.active,
            level: p.level,
            hp: p.hp,
            max_hp: p.max_hp,
            status: p.status,
            atk: p.atk,
            def: p.def,
            spe: p.spe,
            spc: p.spc,
            types: p.types,
            moves: p.moves,
            boost_atk: p.boost_atk,
            boost_def: p.boost_def,
            boost_spe: p.boost_spe,
            boost_spc: p.boost_spc,
            item: p.item,
        })
        .collect();
    s
}

/// Plays one battle to its end. `inputs` is the input pump (the device's
/// collector loop, or a test's robot); it runs alongside the battle loop
/// and is dropped when the battle ends.
pub async fn run_battle<T, E, F>(
    transport: &mut T,
    setup: &BattleSetup,
    bus: &InputBus,
    inputs: F,
    effects: &mut E,
) where
    T: Transport,
    E: BoardEffects,
    F: Future<Output = ()>,
{
    match select(battle_loop(transport, setup, bus, effects), inputs).await {
        Either::First(()) | Either::Second(()) => {}
    }
}

/// One seat waiting on an answer.
struct Seat {
    side: SideId,
    request: Request,
    events: Vec<BoardEvent>,
    options: SlotOptions,
}

async fn battle_loop<T: Transport, E: BoardEffects>(
    transport: &mut T,
    setup: &BattleSetup,
    bus: &InputBus,
    effects: &mut E,
) {
    let mut view = BattleView::new(setup.dex());
    for (i, team) in setup.teams.iter().enumerate() {
        let side = if i == 0 { SideId::P1 } else { SideId::P2 };
        for (slot, set) in team.iter().enumerate() {
            let nick = if set.name.is_empty() { set.species } else { set.name };
            view.set_nickname(side, slot as u8, &nick);
        }
    }

    let start = Command::Start { format: setup.format, seed: setup.seed.into() };
    if transport.send(&start).is_err() {
        return;
    }
    for (i, team) in setup.teams.iter().enumerate() {
        let cmd = Command::Player {
            side: if i == 0 { SideId::P1 } else { SideId::P2 },
            name: Name::from_str(setup.names[i]).unwrap_or_default(),
            team: *team,
        };
        if transport.send(&cmd).is_err() {
            return;
        }
    }

    let mut opening = true;
    loop {
        // Everything the server said since the last command.
        let mut lines = Vec::new();
        let mut requests: Vec<Request> = Vec::new();
        let mut ended = false;
        while let Some(m) = transport.poll() {
            match m {
                Incoming::Log(msg) | Incoming::Side(_, msg) => lines.push(msg),
                Incoming::Request(r) => requests.push(r),
                Incoming::End => ended = true,
            }
        }

        // PS sends the first requests after the start update. The view
        // reads them first, so the leads' `SwitchIn` carries their moves
        // and Speed and needs no catch-up `ActiveMonUpdate`; their prompt
        // events still go out after the update, like every turn's.
        let mut prompt_events: Vec<Vec<BoardEvent>> = Vec::new();
        // A team preview (a Picked Team Size format) comes before the start
        // update: the opening is the first move requests after it.
        let preview = requests.iter().any(|r| matches!(r, Request::TeamPreview(_)));
        if opening && !requests.is_empty() && !preview {
            opening = false;
            prompt_events = requests.iter().map(|r| prompt_events_for(&mut view, r)).collect();
        }
        for msg in &lines {
            for ev in view.apply(msg) {
                effects.on_event(ev).await;
            }
        }
        if prompt_events.len() != requests.len() {
            prompt_events = requests.iter().map(|r| prompt_events_for(&mut view, r)).collect();
        }
        if ended {
            return;
        }

        let mut seats: Vec<Seat> = Vec::new();
        for (request, events) in requests.into_iter().zip(prompt_events) {
            match slot_options(&view, &request) {
                Some(seat) => seats.push(Seat {
                    side: request.side().id,
                    options: slot_options_from_seat(seat),
                    request,
                    events,
                }),
                // A wait request: nothing to choose, but its events (a
                // late Speed) still reach the board.
                None => {
                    for ev in events {
                        effects.on_event(ev).await;
                    }
                }
            }
        }
        if seats.is_empty() {
            // No end and nobody to ask: nothing anyone could press would
            // move the battle on (both transports answer a command before
            // `send` returns, so there is nothing more to wait for).
            return;
        }

        // Prompt every seat before taking any answer, so both players
        // choose at once.
        let batch_total = seats.len();
        for seat in seats.iter_mut() {
            for ev in core::mem::take(&mut seat.events) {
                effects.on_event(ev).await;
            }
            bus.prompt
                .send(ActivePrompt {
                    player_id: seat.side.as_str().to_string(),
                    slot: seat.options.clone(),
                    batch_total,
                })
                .await;
        }

        // One answer per prompted seat, routed by its id tag.
        let mut pending: Vec<Seat> = seats;
        while !pending.is_empty() {
            let submitted = bus.choices.receive().await;
            let Some(pos) = pending.iter().position(|s| s.side.as_str() == submitted.player_id) else {
                // A stray or duplicate answer: that seat has no request.
                continue;
            };
            let seat = pending.remove(pos);
            let choice = translate(&seat.request, &submitted.choice);
            let _ = transport.send(&Command::Choose { side: seat.side, choice });
        }
    }
}

/// The view's prompt events for `req`, with a single-move request's menu
/// cut down to that move. PS offers one move without PP (Recharge, a lock
/// such as Thrash or a charging move, Struggle, Gen 1's `Fight` for a
/// frozen, asleep or partially trapped mon; `getMoveRequestData`), and the
/// seat's options say so ([`slot_options`]: one move, or a forced `move 0`).
/// The view names Recharge itself; for the others it repeats the mon's four
/// moves, which would show a menu of moves the request does not offer. The
/// seat's screen shows the one move it does, named.
fn prompt_events_for(view: &mut BattleView, req: &Request) -> Vec<BoardEvent> {
    let mut events: Vec<BoardEvent> = view.apply_request(req).into_iter().collect();
    let Some(slot) = single_move_slot(view, req) else { return events };
    for ev in events.iter_mut() {
        if let BoardEvent::MovesUpdate { moves, .. } = ev {
            *moves = alloc::vec![slot.clone()];
        }
    }
    events
}

/// The one move of a single-move request, as the board's move slot: the
/// mon's own slot for a move it knows (its PP and type), else the move
/// from the dex (Struggle), or the synthetic `Recharge` / `Fight` button.
fn single_move_slot(view: &BattleView, req: &Request) -> Option<MoveSlot> {
    let Request::Move(r) = req else { return None };
    let a = r.active.first()?;
    if a.moves.len() != 1 || a.moves[0].pp.is_some() {
        return None;
    }
    let (mv, ty, power) = match a.moves[0].name {
        RequestMoveName::Recharge => return Some(BattleView::recharge_slot()),
        RequestMoveName::Fight => {
            return Some(MoveSlot { name: String::from("Fight"), ..BattleView::recharge_slot() })
        }
        RequestMoveName::Move(mv) => (mv, None, None),
        RequestMoveName::HiddenPower { ty, power } => {
            (view.dex().move_id("hiddenpower").map(|m| MoveNum(m.0)).unwrap_or_default(), Some(ty), Some(power))
        }
        RequestMoveName::WithPower { mv, power } => (mv, None, Some(power)),
    };
    let maxpp = view.dex().moves.get(mv.0 as usize).map_or(0, |d| d.pp);
    let named = view.move_slot(&KnownMove { mv, ty, power, pp: maxpp, maxpp });
    let side = req.side();
    let own = side
        .pokemon
        .iter()
        .find(|p| p.active)
        .and_then(|p| view.mon(PokemonIdent::inactive(side.id, p.ident.slot)))
        .and_then(|m| m.moves.iter().map(|k| view.move_slot(k)).find(|s| s.name == named.name));
    Some(own.unwrap_or(named))
}

/// The device-grammar answer as a PS choice. The collector only lets a
/// player commit what the seat options allow, so this falls back to PS's
/// `default` only for an answer the request cannot express.
fn translate(request: &Request, answer: &str) -> Choice {
    let answer: String = if answer.is_empty() { String::from("pass") } else { answer.to_string() };
    ps_client::choice(request, &answer).unwrap_or(Choice::Default)
}

#[cfg(test)]
mod tests {
    //! The same seeded Gen 1 battles over [`LocalTransport`] (typed) and
    //! over [`TextTransport`] (`BattleStream` text through a byte link to
    //! `ps_sim` behind the text codec, as `ps_sim_cli` serves it) must give
    //! the board the same event stream, and every battle must reach its
    //! `|win|` or `|tie|`.
    //!
    //! Teams come from [`crate::randbat::draw_two_ps_teams`], which keeps
    //! to `ps_sim::mods::ported(1)`, so no battle reaches an unported
    //! effect while the Gen 1 handler batches are still landing.

    extern crate critical_section;
    extern crate std;

    use super::*;
    use crate::choice_collect::SlotOptions;
    use crate::rng::SimpleRng;
    use crate::transport::{ByteLink, LocalTransport, TextTransport};
    use alloc::boxed::Box;
    use alloc::format;
    use alloc::vec;
    use core::cell::RefCell;
    use std::thread_local;

    use ps_protocol::text::encode_log;
    use ps_protocol::text::json::encode_request;
    use ps_protocol::text::line::encode_msg;
    use ps_protocol::text::team::unpack;
    use ps_protocol::Msg;
    use ps_sim::text::{dex_tables, Names};
    use ps_sim::{Battle, Sink};

    fn block_on<F: Future>(f: F) -> F::Output {
        use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
        fn noop(_: *const ()) {}
        fn clone(p: *const ()) -> RawWaker {
            RawWaker::new(p, &VTABLE)
        }
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
        let waker = unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) };
        let mut cx = Context::from_waker(&waker);
        let mut f = core::pin::pin!(f);
        loop {
            if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    /// Every event the board was handed, in order.
    #[derive(Default)]
    struct Record(Vec<BoardEvent>);

    impl BoardEffects for Record {
        async fn on_event(&mut self, event: BoardEvent) {
            self.0.push(event);
        }
    }

    /// The robot on both seats: a random legal choice per prompt, from
    /// the shared [`SlotOptions::random_choice`] policy.
    async fn robot(bus: &InputBus, seed: u64) {
        let mut rng = SimpleRng::new(seed);
        loop {
            let first = bus.prompt.receive().await;
            let mut batch = vec![first];
            while batch.len() < batch[0].batch_total {
                batch.push(bus.prompt.receive().await);
            }
            for p in batch {
                let mut slot: SlotOptions = p.slot;
                let choice = slot.random_choice(&mut rng);
                bus.choices.send(crate::battle_input::PlayerChoice { player_id: p.player_id, choice }).await;
            }
        }
    }

    fn play<T: Transport>(transport: &mut T, seed: u64) -> Vec<BoardEvent> {
        let teams = crate::randbat::draw_two_ps_teams(&ps_dex::GEN1, seed, 6);
        let setup = BattleSetup::local(1, seed, teams);
        let bus = InputBus::new();
        let mut rec = Record::default();
        block_on(run_battle(transport, &setup, &bus, robot(&bus, seed ^ 0x5eed), &mut rec));
        rec.0
    }

    /// Held items handed out in the Gen 3 battles: the ones that announce
    /// themselves (berries, herbs, Leftovers, Choice Band, Focus Band,
    /// Shell Bell) and a few silent ones.
    const GEN3_ITEMS: &[&str] = &[
        "Leftovers", "Choice Band", "Lum Berry", "Salac Berry", "Petaya Berry", "Liechi Berry",
        "Ganlon Berry", "Apicot Berry", "Starf Berry", "Sitrus Berry", "Oran Berry", "Chesto Berry",
        "Cheri Berry", "Pecha Berry", "Rawst Berry", "Aspear Berry", "Persim Berry", "Leppa Berry",
        "Figy Berry", "White Herb", "Mental Herb", "Focus Band", "Quick Claw", "King's Rock",
        "Shell Bell", "Bright Powder", "Scope Lens", "Macho Brace",
    ];

    /// Moves spliced into the Gen 3 teams so the battles reach the lines
    /// a random set seldom does: items changing hands, `-activate`s,
    /// field and side effects, ability copies. Unported ones are skipped.
    const GEN3_MOVES: &[&str] = &[
        "Trick", "Thief", "Covet", "Knock Off", "Heal Bell", "Aromatherapy", "Perish Song",
        "Substitute", "Protect", "Endure", "Destiny Bond", "Attract", "Mean Look", "Pursuit",
        "Spite", "Mimic", "Pain Split", "Wish", "Rapid Spin", "Spikes", "Encore", "Taunt", "Yawn",
        "Rain Dance", "Sunny Day", "Sandstorm", "Hail", "Skill Swap", "Role Play", "Recycle", "Bide",
        "Magic Coat", "Snatch", "Grudge", "Imprison", "Uproar", "Stockpile", "Curse", "Lock-On",
        "Foresight", "Camouflage", "Conversion 2", "Memento", "Baton Pass", "Rest", "Sleep Talk",
        "Toxic", "Will-O-Wisp", "Thunder Wave", "Swagger", "Belly Drum", "Refresh", "Ingrain",
        "Future Sight", "Doom Desire", "Fake Out", "Focus Punch", "Counter", "Mirror Coat",
        "Teeter Dance", "Outrage", "Hyper Beam", "Solar Beam", "Fly", "Bind", "Whirlpool",
        "Torment", "Nightmare", "Disable", "Leech Seed", "Water Sport", "Mud Sport", "Charge",
        "Transform", "Psych Up", "Haze", "Beat Up", "Block", "Smelling Salt", "Brick Break",
        "Light Screen", "Reflect", "Safeguard", "Mist", "Focus Energy", "Fissure", "Double Kick",
    ];

    /// Gen 3 teams with items and abilities in play: the device's PS draw,
    /// then for each mon one of its species' abilities, a held item (four
    /// mons in five) and one move from [`GEN3_MOVES`] in place of its last.
    fn gen3_teams(seed: u64) -> [PackedTeam; 2] {
        let dex = &ps_dex::GEN3;
        let ported = |kind: &str, name: &str| {
            let id: String = name.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect();
            ps_sim::mods::ported(3).contains(&format!("{kind}:{id}").as_str())
        };
        let mut rng = SimpleRng::new(seed ^ 0x6e3);
        let mut teams = crate::randbat::draw_two_ps_teams(dex, seed, 6);
        for team in teams.iter_mut() {
            for set in team.iter_mut() {
                let id: String = set.name.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect();
                let species = dex.species_def(dex.species_id(&id).expect("a dex species"));
                let slot = (rng.next_u64() % 2) as usize;
                let ability = species.ability(slot).or(species.ability(0)).unwrap_or("");
                set.ability = Name::from_str(ability).unwrap_or_default();
                // One mon in five holds nothing, so Thief and Covet have
                // somewhere to put what they take.
                let item = GEN3_ITEMS[(rng.next_u64() % GEN3_ITEMS.len() as u64) as usize];
                if rng.next_u64() % 5 != 0 {
                    set.item = Name::from_str(item).unwrap_or_default();
                }
                let mv = GEN3_MOVES[(rng.next_u64() % GEN3_MOVES.len() as u64) as usize];
                if ported("move", mv) && !set.moves.iter().any(|m| m.as_str() == mv) {
                    let last = set.moves.len() - 1;
                    set.moves[last] = Name::from_str(mv).unwrap_or_default();
                }
            }
        }
        teams
    }

    fn play_gen3<T: Transport>(transport: &mut T, seed: u64) -> Vec<BoardEvent> {
        let setup = BattleSetup::local(3, seed, gen3_teams(seed));
        let bus = InputBus::new();
        let mut rec = Record::default();
        block_on(run_battle(transport, &setup, &bus, robot(&bus, seed ^ 0x5eed), &mut rec));
        rec.0
    }

    // ── The text server: `ps_sim` behind `BattleStream` text ───────────

    thread_local! {
        static SPILLED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn tables(dex: &'static ps_dex::Dex) -> &'static ps_protocol::text::names::Tables {
        thread_local! {
            static TABLES: RefCell<Option<&'static ps_protocol::text::names::Tables>> = const { RefCell::new(None) };
        }
        TABLES.with(|t| *t.borrow_mut().get_or_insert_with(|| Box::leak(Box::new(dex_tables(dex)))))
    }

    fn spill(b: &Battle, lines: &[Msg]) {
        let text = encode_log(lines, tables(b.dex), &Names(b));
        SPILLED.with(|s| s.borrow_mut().push(text));
    }

    enum Block {
        Update(Vec<String>, Vec<Msg>),
        Line(SideId, Msg),
        Request(SideId, Request),
    }

    #[derive(Default)]
    struct Collect(Vec<Block>);

    impl Sink for Collect {
        fn update(&mut self, lines: &[Msg]) {
            let spilled = SPILLED.with(|s| core::mem::take(&mut *s.borrow_mut()));
            self.0.push(Block::Update(spilled, lines.to_vec()));
        }
        fn side_line(&mut self, side: SideId, line: &Msg) {
            self.0.push(Block::Line(side, *line));
        }
        fn request(&mut self, side: SideId, request: &Request) {
            self.0.push(Block::Request(side, request.clone()));
        }
    }

    /// A byte link whose far end is `ps_sim`, framed as `ps_sim_cli`
    /// writes it (`crates/ps_harness/src/bin/ps_sim_cli.rs`): each
    /// `BattleStream` message is its type line, its data and a newline.
    #[derive(Default)]
    struct SimLink {
        battle: Option<Box<Battle>>,
        ended_sent: bool,
        rx: Vec<u8>,
        tx: Vec<u8>,
        /// Every byte the client wrote, for the test's own check.
        commands: String,
    }

    impl SimLink {
        fn command(&mut self, line: &str) {
            let rest = &line[1..];
            let (kind, arg) = rest.split_once(' ').unwrap_or((rest, ""));
            match kind {
                "start" => {
                    let v: serde_json::Value = serde_json::from_str(arg).unwrap();
                    let spec = FormatSpec::parse(v["formatid"].as_str().unwrap()).unwrap();
                    let seed: Vec<u16> = v["seed"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u16).collect();
                    let mut b = Box::new(Battle::from_start(spec, <[u16; 4]>::try_from(seed).unwrap(), 0));
                    b.spill = Some(spill);
                    self.battle = Some(b);
                    self.ended_sent = false;
                }
                "player" => {
                    let (side, json) = arg.split_once(' ').unwrap();
                    let v: serde_json::Value = serde_json::from_str(json).unwrap();
                    let team = unpack(v["team"].as_str().unwrap()).unwrap();
                    let b = self.battle.as_mut().unwrap();
                    b.set_player(SideId::parse(side).unwrap(), v["name"].as_str().unwrap(), "", "", Some(&team)).unwrap();
                }
                "p1" | "p2" => {
                    let (verb, n) = arg.trim().split_once(' ').unwrap_or((arg.trim(), ""));
                    let choice = match verb {
                        "move" => Choice::Move(n.trim().parse().unwrap()),
                        "switch" => Choice::Switch(n.trim().parse().unwrap()),
                        "pass" => Choice::Pass,
                        "default" => Choice::Default,
                        other => panic!("choice {other}"),
                    };
                    let side = SideId::parse(kind).unwrap();
                    self.battle.as_mut().unwrap().step(&Command::Choose { side, choice }).unwrap();
                }
                other => panic!("command >{other}"),
            }
            self.flush();
        }

        fn message(&mut self, kind: &str, data: &str) {
            self.tx.extend_from_slice(format!("{kind}\n{data}\n").as_bytes());
        }

        fn flush(&mut self) {
            let b = self.battle.as_mut().unwrap();
            let mut c = Collect::default();
            b.send_updates(&mut c);
            let b = self.battle.as_ref().unwrap();
            let (dex, names) = (tables(b.dex), Names(b));
            let mut out: Vec<(&str, String)> = Vec::new();
            for block in c.0 {
                match block {
                    Block::Update(mut parts, lines) => {
                        if !lines.is_empty() || parts.is_empty() {
                            parts.push(encode_log(&lines, dex, &names));
                        }
                        out.push(("update", parts.join("\n")));
                    }
                    Block::Line(side, m) => out.push(("sideupdate", format!("{}\n{}", side.as_str(), encode_msg(&m, dex, &names)))),
                    Block::Request(side, r) => {
                        out.push(("sideupdate", format!("{}\n|request|{}", side.as_str(), encode_request(&r, dex, &names))))
                    }
                }
            }
            let end = b.ended && !self.ended_sent;
            for (kind, data) in out {
                self.message(kind, &data);
            }
            if end {
                self.ended_sent = true;
                self.message("end", "{}");
            }
        }
    }

    impl ByteLink for SimLink {
        fn write(&mut self, bytes: &[u8]) {
            self.rx.extend_from_slice(bytes);
            while let Some(nl) = self.rx.iter().position(|&c| c == b'\n') {
                let raw: Vec<u8> = self.rx.drain(..=nl).collect();
                let line = String::from_utf8(raw).unwrap();
                self.commands.push_str(&line);
                self.command(line.trim_end());
            }
        }
        fn read(&mut self, buf: &mut [u8]) -> usize {
            let n = buf.len().min(self.tx.len());
            buf[..n].copy_from_slice(&self.tx[..n]);
            self.tx.drain(..n);
            n
        }
    }

    // ── The tests ───────────────────────────────────────────────────────

    #[test]
    fn fifty_gen1_battles_agree_over_both_transports() {
        let mut turns = 0u32;
        let mut ties = 0;
        for seed in 0..50u64 {
            let mut local = LocalTransport::new();
            let typed = play(&mut local, seed);
            let b = local.battle().expect("the battle started");
            assert!(b.ended, "seed {seed}: the battle did not end (turn {})", b.turn);
            turns += u32::from(b.turn);

            let mut text = TextTransport::new(SimLink::default());
            let decoded = play(&mut text, seed);
            assert!(text.link().commands.starts_with(">start "), "seed {seed}: the text path sent no >start");
            assert_eq!(text.undecoded(), 0, "seed {seed}: the text path skipped lines it could not decode");

            let end = typed.iter().rev().find(|e| matches!(e, BoardEvent::Win { .. } | BoardEvent::Tie));
            assert!(end.is_some(), "seed {seed}: no Win or Tie event");
            if matches!(end, Some(BoardEvent::Tie)) {
                ties += 1;
            }
            if typed != decoded {
                let at = typed.iter().zip(&decoded).position(|(a, b)| a != b).unwrap_or(typed.len().min(decoded.len()));
                panic!(
                    "seed {seed}: the streams part at event {at} (typed {} events, text {}):\n  typed: {:?}\n  text:  {:?}",
                    typed.len(),
                    decoded.len(),
                    typed.get(at),
                    decoded.get(at)
                );
            }
        }
        std::println!("50 gen 1 battles: {turns} turns, {ties} ties, identical over both transports");
    }

    /// A Picked Team Size format asks both seats for a team preview pick
    /// before the battle; the seats answer PS's `default` (the device has
    /// no preview screen) and the battle plays on, the same over both
    /// transports (the one-sided `|poke|` lines and the preview request
    /// decode).
    #[test]
    fn a_team_preview_format_plays_over_both_transports() {
        for seed in 0..5u64 {
            let run = |transport: &mut dyn FnMut(&BattleSetup) -> Vec<BoardEvent>| {
                let teams = crate::randbat::draw_two_ps_teams(&ps_dex::GEN1, seed, 6);
                let mut setup = BattleSetup::local(1, seed, teams);
                setup.format = FormatSpec::parse("gen1customgame@@@Picked Team Size = 3").unwrap();
                transport(&setup)
            };
            let mut local = LocalTransport::new();
            let typed = run(&mut |setup| {
                let bus = InputBus::new();
                let mut rec = Record::default();
                block_on(run_battle(&mut local, setup, &bus, robot(&bus, seed ^ 0x5eed), &mut rec));
                rec.0
            });
            let b = local.battle().expect("the battle started");
            assert!(b.ended, "seed {seed}: the battle did not end (turn {})", b.turn);
            assert_eq!(b.sides[0].team_size, 3, "seed {seed}: the picked team");
            let mut text = TextTransport::new(SimLink::default());
            let decoded = run(&mut |setup| {
                let bus = InputBus::new();
                let mut rec = Record::default();
                block_on(run_battle(&mut text, setup, &bus, robot(&bus, seed ^ 0x5eed), &mut rec));
                rec.0
            });
            assert!(text.link().commands.contains(">p1 default"), "seed {seed}: no team preview answer");
            assert_eq!(text.undecoded(), 0, "seed {seed}: the text path skipped lines it could not decode");
            assert_eq!(typed, decoded, "seed {seed}: the streams part");
        }
    }

    /// Mimic and Transform rewrite move slots (Transform also the species and
    /// Speed), which the request and the protocol lines must carry alike on
    /// both transports. Plays seeds from 50 on until three battles reached
    /// each and requires identical streams for every battle played.
    #[test]
    fn copied_moves_agree_over_both_transports() {
        let start = |e: &BoardEvent, w: &str| matches!(e, BoardEvent::EffectStart { what, .. } if what == w);
        let (mut transforms, mut mimics, mut psychic) = (0, 0, false);
        let mut seed = 50u64;
        while transforms < 3 || mimics < 3 || !psychic {
            assert!(seed < 1050, "seeds 50..{seed}: {transforms} Transform and {mimics} Mimic battles");
            let mut local = LocalTransport::new();
            let typed = play(&mut local, seed);
            let mut text = TextTransport::new(SimLink::default());
            let decoded = play(&mut text, seed);
            assert_eq!(text.undecoded(), 0, "seed {seed}");
            if typed != decoded {
                let at = typed.iter().zip(&decoded).position(|(a, b)| a != b).unwrap_or(typed.len().min(decoded.len()));
                panic!("seed {seed}: the streams part at event {at}:\n  typed: {:?}\n  text:  {:?}", typed.get(at), decoded.get(at));
            }
            transforms += usize::from(typed.iter().any(|e| start(e, "transform")));
            mimics += usize::from(typed.iter().any(|e| start(e, "mimic")));
            psychic |= typed.iter().any(|e| {
                matches!(e, BoardEvent::EffectStart { what, detail: Some(d), .. } if what == "mimic" && d == "Psychic")
            });
            seed += 1;
        }
        std::println!("seeds 50..{seed}: {transforms} Transform and {mimics} Mimic battles, identical over both transports");
    }

    /// Gen 3 with items and abilities in play ([`gen3_teams`]): the
    /// item, ability and `-activate` lines must reach the board alike over
    /// both transports, and every kind of them must turn up somewhere in
    /// the seeds played. Each event the board narrates must have words
    /// (no empty description) that fit the 48-byte narration box.
    /// The device's own Gen 3 battles (G3-8): PS random-battle teams with
    /// their items and abilities ([`crate::randbat::draw_device_teams`]),
    /// three and six a side, play to an end on the PS engine, and the
    /// typed and text transports narrate them identically.
    #[test]
    fn device_gen3_battles_end_over_both_transports() {
        use crate::board_event::BoardEventExt;
        let mut turns = 0u32;
        let mut fallback = 0u32;
        for seed in 0..40u64 {
            let size = if seed % 2 == 0 { 3 } else { 6 };
            let play_dev = |transport: &mut dyn FnMut(&BattleSetup, &InputBus, &mut Record)| {
                let teams = crate::randbat::draw_device_teams(&ps_dex::GEN3, seed, size);
                assert!(teams.iter().all(|t| t.len() == size), "seed {seed}: short team");
                let setup = BattleSetup::local(3, seed, teams);
                let bus = InputBus::new();
                let mut rec = Record::default();
                transport(&setup, &bus, &mut rec);
                rec.0
            };
            let mut local = LocalTransport::new();
            let typed = play_dev(&mut |setup, bus, rec| {
                block_on(run_battle(&mut local, setup, bus, robot(bus, seed ^ 0x5eed), rec))
            });
            let b = local.battle().expect("the battle started");
            assert!(b.ended, "seed {seed}: the battle did not end (turn {})", b.turn);
            turns += u32::from(b.turn);
            let mut text = TextTransport::new(SimLink::default());
            let decoded = play_dev(&mut |setup, bus, rec| {
                block_on(run_battle(&mut text, setup, bus, robot(bus, seed ^ 0x5eed), rec))
            });
            assert_eq!(text.undecoded(), 0, "seed {seed}: undecoded lines");
            if typed != decoded {
                let at = typed.iter().zip(&decoded).position(|(a, b)| a != b).unwrap_or(typed.len().min(decoded.len()));
                panic!("seed {seed}: the streams part at event {at}:\n  typed: {:?}\n  text:  {:?}", typed.get(at), decoded.get(at));
            }
            fallback += typed.iter().filter(|e| e.description().contains("activated!)")).count() as u32;
        }
        std::println!("40 device gen 3 battles: {turns} turns, {fallback} default `activated` lines");
    }

    #[test]
    fn gen3_items_and_abilities_agree_over_both_transports() {
        use crate::board_event::BoardEventExt;
        use std::collections::BTreeMap;
        let mut seen: BTreeMap<String, u32> = BTreeMap::new();
        let mut turns = 0u32;
        for seed in 0..60u64 {
            let mut local = LocalTransport::new();
            let typed = play_gen3(&mut local, seed);
            let b = local.battle().expect("the battle started");
            assert!(b.ended, "seed {seed}: the battle did not end (turn {})", b.turn);
            turns += u32::from(b.turn);
            assert!(typed.iter().any(|e| matches!(e, BoardEvent::Win { .. } | BoardEvent::Tie)), "seed {seed}: no end");

            let mut text = TextTransport::new(SimLink::default());
            let decoded = play_gen3(&mut text, seed);
            assert_eq!(text.undecoded(), 0, "seed {seed}: the text path skipped lines it could not decode");
            if typed != decoded {
                let at = typed.iter().zip(&decoded).position(|(a, b)| a != b).unwrap_or(typed.len().min(decoded.len()));
                panic!("seed {seed}: the streams part at event {at}:\n  typed: {:?}\n  text:  {:?}", typed.get(at), decoded.get(at));
            }
            for e in &typed {
                let key = match e {
                    BoardEvent::Ability { how, .. } => format!("Ability {how}"),
                    BoardEvent::ItemGained { how, .. } => format!("ItemGained {how}"),
                    BoardEvent::ItemEnded { how, .. } => format!("ItemEnded {how}"),
                    BoardEvent::Activate { what, .. } => format!("Activate {what}"),
                    _ => continue,
                };
                let words = e.description();
                assert!(!words.is_empty() && !words.contains("activated!)"), "seed {seed}: {e:?} reads {words:?}");
                assert!(crate::oled_ctl::oled_cmds_for_event(e).iter().any(|c| matches!(c, crate::oled_ctl::OledCmd::EventFlash { .. })), "{e:?} does not flash");
                *seen.entry(key).or_default() += 1;
            }
        }
        std::println!("60 gen 3 battles: {turns} turns, identical over both transports; narrated: {seen:?}");
        for kind in [
            "Ability ",          // Intimidate, Pressure
            "Ability activate",  // a `[from] ability:` banner
            "Ability trace",
            "Ability roleplay",
            "ItemGained trick",
            "ItemGained thief",
            "ItemEnded eat",
            "ItemEnded knockoff",
            "Activate confusion",
            "Activate Spite",
            "Activate Attract",
            "Activate Trick",
            "Activate Lock-On",
            "Activate hitcount",
            "Activate Perish Song",
        ] {
            assert!(seen.keys().any(|k| k == kind), "no {kind:?} in 60 battles: {seen:?}");
        }
    }

    #[test]
    fn the_leads_switch_in_complete() {
        let mut local = LocalTransport::new();
        let events = play(&mut local, 7);
        let leads: Vec<&BoardEvent> = events.iter().filter(|e| matches!(e, BoardEvent::SwitchIn { .. })).take(2).collect();
        assert_eq!(leads.len(), 2);
        for e in leads {
            let BoardEvent::SwitchIn { moves, speed, team_slot, .. } = e else { unreachable!() };
            assert!(!moves.is_empty(), "a lead's SwitchIn carries its moves: {e:?}");
            assert!(speed.is_some(), "and its Speed: {e:?}");
            assert_eq!(*team_slot, Some(0));
        }
        let first_turn = events.iter().position(|e| matches!(e, BoardEvent::Turn { n: 1 })).unwrap();
        assert!(
            !events[..first_turn].iter().any(|e| matches!(e, BoardEvent::ActiveMonUpdate { .. })),
            "no catch-up ActiveMonUpdate before turn 1"
        );
        assert!(matches!(events.first(), Some(BoardEvent::BattleStart)), "{:?}", events.first());
    }

    #[test]
    fn ps_teams_keep_to_the_ported_moves() {
        for seed in 0..20u64 {
            for team in crate::randbat::draw_two_ps_teams(&ps_dex::GEN1, seed, 6) {
                assert_eq!(team.len(), 6);
                for set in team.iter() {
                    assert!(!set.moves.is_empty());
                    for m in set.moves.iter() {
                        let id: String = m.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect();
                        let key = format!("move:{id}");
                        assert!(ps_sim::mods::ported(1).contains(&key.as_str()), "{} has unported {m}", set.name.as_str());
                    }
                }
            }
        }
    }

    #[test]
    fn seat_options_copy_across() {
        let s = slot_options_from_seat(SeatOptions {
            player_num: 2,
            player_id: "p2".into(),
            n_moves: 3,
            usable: [true, false, true, false],
            trapped: true,
            active_slot: Some(1),
            party_ok: [true, false, true, true, false, false],
            forced_switch: false,
            auto: None,
            party: Vec::new(),
        });
        assert_eq!((s.player_num, s.player_id.as_str(), s.n_moves), (2, "p2", 3));
        assert_eq!(s.usable, [true, false, true, false]);
        assert!(s.trapped && !s.forced_switch && s.auto.is_none());
        assert_eq!(s.active_slot, Some(1));
        assert_eq!(s.party_ok, [true, false, true, true, false, false]);
    }
}
