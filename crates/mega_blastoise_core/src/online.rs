//! A real Pokemon Showdown server, reached through the phone bridge: the
//! [`Transport`] the runner ([`crate::runner::ps`]) plays an online battle
//! over (`docs/living/phone_bridge.md`).
//!
//! The bridge page in the phone's browser owns the websocket and passes it
//! through as lines, so everything that decides anything stays here, once.
//!
//! Page to board (one line each, `\n` ended):
//! - `hello <page text>`: the page opened the board. Answered `hello mb1 fw=<firmware commit>`.
//! - `play 1` / `play 3`, optionally followed by ` auto`: search for a
//!   Gen 1 / Gen 3 random battle. `auto` lets the board's random AI choose
//!   (the VER-251 soak), as the lobby's AI seat does.
//! - `cancel`: stop searching.
//! - `frame`: a websocket message begins (resets the room the lines belong to).
//! - `< <line>`: one line of that message, verbatim (`>battle-gen1randombattle-7`
//!   headers included).
//! - `down`: the websocket closed.
//!
//! Board to page:
//! - `> <text>`: send `<text>` on the websocket, verbatim.
//! - `hello mb1 fw=<commit>`, `info <text>`, `result <key=value ...>`: for the page's log.
//!
//! What the board does with the server's lines: the room header names the
//! battle; `|request|` is the JSON decoded to a [`Request`]; `|error|` is the
//! server refusing a choice (counted: VER-251 wants none); every other line
//! goes through the text codec to a [`Msg`] for `ps_client`'s view. Lines the
//! board has no use for (chat, joins, timers) are dropped before decoding.

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::format;
use alloc::string::{String, ToString};

use ps_client::DexView;
use ps_protocol::text::command::encode_command;
use ps_protocol::text::json::decode_request;
use ps_protocol::text::line::{decode_line, CodecState};
use ps_protocol::{Command, Msg, SideId};

use crate::device_runner::DevicePort;
use crate::transport::{Incoming, Transport, TransportError};

/// What the link told the board, for the page's log and the VER-251 verdict.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OnlineStats {
    /// `|error|` lines: choices the server refused.
    pub rejected: u32,
    /// Server lines the codec could not read (skipped).
    pub undecoded: u32,
    /// The first such line, cut to 120 bytes, for the log.
    pub first_undecoded: Option<String>,
    /// Choices sent.
    pub choices: u32,
    /// `Some(true)` once the board's side won, `Some(false)` if it lost.
    pub won: Option<bool>,
    /// The `|tie|` ended it.
    pub tied: bool,
    /// The link dropped before the battle ended.
    pub dropped: bool,
}

/// The server's kinds of line the board never needs (chat, who is in the
/// room, the timer's warnings, HTML panels): dropped before the codec.
fn ignored(kind: &str) -> bool {
    matches!(
        kind,
        "" | "init" | "rated" | "title" | "j" | "J" | "l" | "L" | "n" | "N" | "c" | "c:" | "chat" | "raw" | "html"
            | "uhtml" | "uhtmlchange" | "inactive" | "inactiveoff" | "split" | "deinit" | "popup"
    )
}

pub struct OnlineTransport<'a, P: DevicePort> {
    port: &'a P,
    codec: CodecState<DexView>,
    /// The battle's room id, once a `>battle-...` header has come.
    room: Option<String>,
    /// The room header of the message being read.
    frame_room: Option<String>,
    /// The board's side, from the first request.
    side: Option<SideId>,
    /// The request the board is answering, to ask again if the server refuses.
    last_request: Option<ps_protocol::Request>,
    inbox: VecDeque<Incoming>,
    stats: OnlineStats,
    ended: bool,
}

impl<'a, P: DevicePort> OnlineTransport<'a, P> {
    pub fn new(port: &'a P, dex: &'static ps_dex::Dex) -> Self {
        Self {
            port,
            codec: CodecState::new(DexView(dex)),
            room: None,
            frame_room: None,
            side: None,
            last_request: None,
            inbox: VecDeque::new(),
            stats: OnlineStats::default(),
            ended: false,
        }
    }

    /// The battle room (`battle-gen1randombattle-7`), once the server made one.
    pub fn room(&self) -> Option<&str> {
        self.room.as_deref()
    }

    /// The board's side, once the server's first request named it.
    pub fn side(&self) -> Option<SideId> {
        self.side
    }

    pub fn stats(&self) -> &OnlineStats {
        &self.stats
    }

    /// The battle ended (`|win|`, `|tie|`) or the link dropped.
    pub fn ended(&self) -> bool {
        self.ended
    }

    /// A battle room exists and the first request has named the board's
    /// side (or the link ended): the answer to a search is in.
    pub fn found(&self) -> bool {
        self.ended || (self.room.is_some() && self.side.is_some())
    }

    async fn wait_line(&mut self) {
        let line = self.port.link_recv().await;
        self.link_line(&line);
    }

    /// One line from the bridge page.
    pub fn link_line(&mut self, line: &str) {
        if let Some(rest) = line.strip_prefix('<') {
            self.server_line(rest.strip_prefix(' ').unwrap_or(rest));
        } else if line == "frame" {
            self.frame_room = None;
        } else if line == "down" && !self.ended {
            self.stats.dropped = true;
            self.ended = true;
            self.inbox.push_back(Incoming::End);
        }
    }

    fn undecoded(&mut self, line: &str) {
        self.stats.undecoded += 1;
        let cut = (0..=line.len().min(120)).rev().find(|&i| line.is_char_boundary(i)).unwrap_or(0);
        if self.stats.first_undecoded.is_none() {
            self.stats.first_undecoded = Some(line[..cut].to_string());
        }
        if self.stats.undecoded <= 5 {
            self.port.link_send(&format!("info undecoded: {}", &line[..cut]));
        }
    }

    /// The server refused a choice (`|error|[Invalid choice] ...`, sent only
    /// to the chooser). Counted before anything is decoded: the text is free
    /// form and the count is the point. An invalid choice leaves the request
    /// open with no new one, so the board is asked again.
    fn refused(&mut self, l: &str) {
        let text = l.strip_prefix("|error|").unwrap_or("");
        let invalid = text.starts_with("[Invalid choice]");
        if !invalid && !text.starts_with("[Unavailable choice]") {
            self.undecoded(l);
            return;
        }
        self.stats.rejected += 1;
        self.port.link_send(&format!("info rejected: {l}"));
        let side = self.side.unwrap_or(SideId::P1);
        if let Ok(Msg::Error { unavailable, text, .. }) = decode_line(l, &mut self.codec) {
            self.inbox.push_back(Incoming::Side(side, Msg::Error { side, unavailable, text }));
        }
        if invalid {
            if let Some(r) = self.last_request.clone() {
                self.inbox.push_back(Incoming::Request(r));
            }
        }
    }

    fn server_line(&mut self, l: &str) {
        if let Some(room) = l.strip_prefix('>') {
            self.frame_room = Some(room.trim().to_string());
            return;
        }
        let kind = l.strip_prefix('|').and_then(|r| r.split('|').next()).unwrap_or("");
        // The battle is the room the server initialises for this search; a
        // late line from an earlier battle's room (its leave, its ratings)
        // must not be taken for it.
        if kind == "init" && self.room.is_none() && !self.ended {
            if let Some(room) = self.frame_room.as_deref().filter(|r| r.starts_with("battle-")) {
                self.room = Some(room.to_string());
            }
        }
        if self.ended || self.frame_room.is_none() || self.frame_room != self.room {
            return; // not this battle's room: the lobby, a search update
        }
        if ignored(kind) {
            return;
        }
        if let Some(json) = l.strip_prefix("|request|") {
            if json.is_empty() || json == "null" {
                return;
            }
            let (dex, names) = self.codec.split_mut();
            match decode_request(json, dex, names) {
                Ok(req) => {
                    self.side = Some(req.side().id);
                    self.last_request = Some(req.clone());
                    self.inbox.push_back(Incoming::Request(req));
                }
                Err(_) => self.undecoded(l),
            }
            return;
        }
        if kind == "error" {
            self.refused(l);
            return;
        }
        match decode_line(l, &mut self.codec) {
            Ok(msg) => {
                let mine = self.side;
                match &msg {
                    Msg::Win(side) => {
                        self.stats.won = mine.map(|m| m == *side);
                        self.ended = true;
                        self.inbox.push_back(Incoming::Log(msg));
                        self.inbox.push_back(Incoming::End);
                    }
                    Msg::Tie => {
                        self.stats.tied = true;
                        self.ended = true;
                        self.inbox.push_back(Incoming::Log(msg));
                        self.inbox.push_back(Incoming::End);
                    }
                    _ => self.inbox.push_back(Incoming::Log(msg)),
                }
            }
            Err(_) => self.undecoded(l),
        }
    }
}

impl<P: DevicePort> Transport for OnlineTransport<'_, P> {
    fn send(&mut self, cmd: &Command) -> Result<(), TransportError> {
        // The server made the battle and the teams: only choices go out.
        if let Command::Choose { .. } = cmd {
            let text = encode_command(cmd).map_err(|_| TransportError::Encode)?;
            let choice = text.split_once(' ').map_or("", |(_, c)| c);
            let room = self.room.as_deref().ok_or(TransportError::NoBattle)?;
            self.port.link_send(&format!("> {room}|/choose {choice}"));
            self.stats.choices += 1;
        }
        Ok(())
    }

    fn poll(&mut self) -> Option<Incoming> {
        while let Some(line) = self.port.link_try_recv() {
            self.link_line(&line);
        }
        self.inbox.pop_front()
    }

    fn remote(&self) -> bool {
        true
    }

    async fn wait(&mut self) {
        while self.inbox.is_empty() {
            self.wait_line().await;
        }
    }
}

#[cfg(test)]
mod tests {
    //! The transport on lines a real Showdown server wrote (captured from a
    //! local server running `smogon/pokemon-showdown`, Gen 1 random battle).
    extern crate std;

    use alloc::string::String;
    use alloc::vec::Vec;
    use core::cell::RefCell;

    use super::*;

    #[derive(Default)]
    struct Port {
        sent: RefCell<Vec<String>>,
    }

    impl DevicePort for Port {
        fn now_ms(&self) -> u64 {
            0
        }
        async fn sleep_ms(&self, _: u32) {}
        fn line_out(&self, _: &str) {}
        fn present(&self, _: &crate::device_session::DeviceSession, _: &crate::OledController, _: bool) {}
        fn seed(&self) -> u64 {
            1
        }
        fn link_send(&self, line: &str) {
            self.sent.borrow_mut().push(String::from(line));
        }
    }

    const OPEN: &str = include_str!("../tests/fixtures/online_gen1_open.txt");

    #[test]
    fn a_real_opening_decodes_to_requests_and_log() {
        let port = Port::default();
        let mut t = OnlineTransport::new(&port, &ps_dex::GEN1);
        for l in OPEN.lines() {
            t.link_line(l);
        }
        assert_eq!(t.stats().undecoded, 0, "first: {:?}", t.stats().first_undecoded);
        assert_eq!(t.room(), Some("battle-gen1randombattle-6"));
        assert_eq!(t.side(), Some(SideId::P2), "the first request names the board's side");
        assert!(t.found());
        let (mut logs, mut reqs) = (0, 0);
        while let Some(m) = t.poll() {
            match m {
                Incoming::Log(_) => logs += 1,
                Incoming::Request(_) => reqs += 1,
                other => panic!("unexpected {other:?}"),
            }
        }
        assert_eq!(reqs, 2, "the opening move request and the forced switch");
        assert!(logs >= 10, "{logs}");
    }

    #[test]
    fn a_late_line_from_an_earlier_room_is_not_the_battle() {
        let port = Port::default();
        let mut t = OnlineTransport::new(&port, &ps_dex::GEN1);
        t.link_line("frame");
        t.link_line("< >battle-gen1randombattle-5");
        t.link_line("< |raw|Guest 1's rating: 1000");
        assert_eq!(t.room(), None);
        for l in OPEN.lines() {
            t.link_line(l);
        }
        assert_eq!(t.room(), Some("battle-gen1randombattle-6"));
        assert!(t.found());
    }

    #[test]
    fn a_choice_goes_out_as_a_choose_line_for_the_room() {
        let port = Port::default();
        let mut t = OnlineTransport::new(&port, &ps_dex::GEN1);
        for l in OPEN.lines() {
            t.link_line(l);
        }
        t.send(&Command::Choose { side: SideId::P2, choice: ps_protocol::Choice::Move(2) }).unwrap();
        t.send(&Command::Choose { side: SideId::P2, choice: ps_protocol::Choice::Switch(3) }).unwrap();
        assert_eq!(
            *port.sent.borrow(),
            ["> battle-gen1randombattle-6|/choose move 2", "> battle-gen1randombattle-6|/choose switch 3"]
        );
        assert_eq!(t.stats().choices, 2);
    }

    #[test]
    fn lines_outside_the_battle_room_and_chat_are_ignored() {
        let port = Port::default();
        let mut t = OnlineTransport::new(&port, &ps_dex::GEN1);
        t.link_line("frame");
        t.link_line(r#"< |updatesearch|{"searching":["gen1randombattle"],"games":null}"#);
        t.link_line("frame");
        t.link_line("< >battle-gen1randombattle-1");
        t.link_line("< |c|☆Someone|hello");
        t.link_line("< |j|☆Someone");
        t.link_line("< |raw|<div>rating</div>");
        assert_eq!(t.stats().undecoded, 0);
        assert!(t.poll().is_none());
        assert!(!t.found(), "a room alone is not a battle: no request yet");
    }

    #[test]
    fn a_refused_choice_is_counted_and_win_ends_the_battle() {
        let port = Port::default();
        let mut t = OnlineTransport::new(&port, &ps_dex::GEN1);
        for l in OPEN.lines() {
            t.link_line(l);
        }
        while t.poll().is_some() {}
        t.link_line("frame");
        t.link_line("< >battle-gen1randombattle-6");
        t.link_line("< |error|[Invalid choice] Can't move: Your Onix doesn't have a move matching Nope");
        t.link_line("< |win|Guest 10");
        assert_eq!(t.stats().rejected, 1);
        assert_eq!(t.stats().undecoded, 0);
        assert_eq!(t.stats().won, Some(true), "Guest 10 is p2, the board's side");
        assert!(t.ended());
        let mut all = Vec::new();
        while let Some(m) = t.poll() {
            all.push(m);
        }
        assert!(matches!(all[0], Incoming::Request(_)), "asked again after the refusal");
        assert!(matches!(all.last(), Some(Incoming::End)));
    }

    #[test]
    fn the_link_dropping_ends_the_battle() {
        let port = Port::default();
        let mut t = OnlineTransport::new(&port, &ps_dex::GEN1);
        t.link_line("down");
        assert!(t.stats().dropped && t.ended());
        assert!(matches!(t.poll(), Some(Incoming::End)));
    }

    #[test]
    fn the_static_dex_numbering_is_the_tables() {
        use ps_protocol::DexNames;
        for dex in [&ps_dex::GEN1, &ps_dex::GEN3] {
            let tables = ps_sim::text::dex_tables(dex);
            let view = DexView(dex);
            for i in 0..dex.moves.len() as u16 {
                assert_eq!(view.move_name(ps_protocol::MoveNum(i)), tables.move_name(ps_protocol::MoveNum(i)).unwrap());
            }
            for i in 0..dex.species.len() as u16 {
                let n = ps_protocol::SpeciesNum(i);
                assert_eq!(view.species_name(n), tables.species_name(n).unwrap());
                let id: String = view.species_name(n).chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect();
                assert_eq!(view.species_from_id(&id), tables.species_from_id(&id), "{id}");
            }
            for i in 0..dex.items.len() as u16 {
                assert_eq!(view.item_name(ps_protocol::ItemNum(i)), tables.item_name(ps_protocol::ItemNum(i)).unwrap());
            }
            for i in 0..dex.abilities.len() as u16 {
                let n = ps_protocol::AbilityNum(i);
                assert_eq!(view.ability_name(n), tables.ability_name(n).unwrap());
            }
            for i in 0..ps_dex::Type::ALL.len() as u8 {
                let n = ps_protocol::TypeNum(i);
                assert_eq!(view.type_name(n), tables.type_name(n).unwrap());
            }
        }
    }
}
