//! How the runner reaches a Showdown battle server: the [`Transport`] seam
//! (design section 7).
//!
//! The runner ([`crate::runner::ps`]) only ever sends [`Command`]s and reads
//! [`Incoming`] messages, so it cannot tell where the server is:
//! - [`LocalTransport`]: `ps_sim` in-process, typed end to end (no text, no
//!   JSON). The device and the web build use it.
//! - [`TextTransport`] (feature `ps_text`): PS's `BattleStream` text over any
//!   byte stream ([`ByteLink`]). The tests use it against `ps_sim` behind
//!   the text codec; a radio or network link to a real PS server would be
//!   one more [`ByteLink`], with no change to the runner or the client.
//!
//! What comes back is what `BattleStream` writes after each command, in its
//! order (PS: `sim/battle.ts` `sendUpdates`, `sim/battle-stream.ts`
//! `_write`): `sideupdate`s first (an `|error|` and the request it
//! re-sends), then the `update` block's log lines, then, after a new
//! request, both sides' requests; `end` once the battle is over.

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::cell::RefCell;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use ps_protocol::{Command, Msg, Request, SideId};
use ps_sim::{Battle, Sink};

/// One message from the server.
#[derive(Clone, Debug)]
pub enum Incoming {
    /// One line of an `update` block (the omniscient log, `|split|` pairs
    /// carried as one `Msg` on the typed path and as the `|split|pN` line
    /// plus both forms on the text path; the client handles either).
    Log(Msg),
    /// A `sideupdate` line for one side (`|error|`, gen 1-4's
    /// `|-activate|...|move: Struggle`).
    Side(SideId, Msg),
    /// A `sideupdate` `|request|`; the request names its side.
    Request(Request),
    /// `end`: the battle is over (after its `|win|` or `|tie|`), or the
    /// server dropped it.
    End,
}

/// Why a command did not reach the battle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportError {
    /// A command before `>start` made a battle.
    NoBattle,
    /// The server refused a `>player` team.
    Team(ps_sim::TeamError),
    /// The command has no text form.
    Encode,
}

/// A connection to a battle server: commands out, messages in.
///
/// `poll` is non-blocking: `None` means nothing more has arrived. Both
/// transports here answer a command before `send` returns, so after a send
/// the runner reads until `None` and has the whole answer.
pub trait Transport {
    fn send(&mut self, cmd: &Command) -> Result<(), TransportError>;
    fn poll(&mut self) -> Option<Incoming>;
}

// ── LocalTransport ───────────────────────────────────────────────────────────

/// Lines a full battle log handed over in the middle of a command (`ps_sim`'s
/// `LogSpill` is a plain `fn`, so they wait here), tagged with the battle's
/// address so two battles in one process (parallel host tests) keep apart.
static SPILLED: Mutex<CriticalSectionRawMutex, RefCell<Vec<(usize, Msg)>>> =
    Mutex::new(RefCell::new(Vec::new()));

fn battle_key(b: &Battle) -> usize {
    b as *const Battle as usize
}

/// The battle's `LogSpill`: the lines go in front of the next `update`.
fn spill(b: &Battle, lines: &[Msg]) {
    let key = battle_key(b);
    SPILLED.lock(|s| s.borrow_mut().extend(lines.iter().map(|m| (key, *m))));
}

/// Takes the spilled lines of the battle at `key`, in order.
fn take_spilled(key: usize) -> Vec<Msg> {
    SPILLED.lock(|s| {
        let mut s = s.borrow_mut();
        let mine = s.iter().filter(|(k, _)| *k == key).map(|(_, m)| *m).collect();
        s.retain(|(k, _)| *k != key);
        mine
    })
}

/// `ps_sim` in the same process. `>start` makes the battle; every other
/// command is [`Battle::step`] followed by [`Battle::send_updates`], whose
/// output waits for [`Transport::poll`] (what `ps_sim_cli` writes to stdout,
/// without the text).
#[derive(Default)]
pub struct LocalTransport {
    battle: Option<Box<Battle>>,
    inbox: VecDeque<Incoming>,
    ended_sent: bool,
}

/// [`Sink`] into the inbox, the spilled lines first.
struct Inbox<'a> {
    inbox: &'a mut VecDeque<Incoming>,
    key: usize,
}

impl Sink for Inbox<'_> {
    fn update(&mut self, lines: &[Msg]) {
        for m in take_spilled(self.key) {
            self.inbox.push_back(Incoming::Log(m));
        }
        self.inbox.extend(lines.iter().map(|m| Incoming::Log(*m)));
    }
    fn side_line(&mut self, side: SideId, line: &Msg) {
        self.inbox.push_back(Incoming::Side(side, *line));
    }
    fn request(&mut self, _side: SideId, request: &Request) {
        self.inbox.push_back(Incoming::Request(request.clone()));
    }
}

impl LocalTransport {
    pub fn new() -> Self {
        Self::default()
    }

    /// The battle, once `>start` has made it (tests and debug output).
    pub fn battle(&self) -> Option<&Battle> {
        self.battle.as_deref()
    }

    /// PS `sendUpdates`, then `end` once the battle is over.
    fn flush(&mut self) {
        let Some(b) = self.battle.as_mut() else { return };
        let key = battle_key(b);
        b.send_updates(&mut Inbox { inbox: &mut self.inbox, key });
        if b.ended && !self.ended_sent {
            self.ended_sent = true;
            self.inbox.push_back(Incoming::End);
        }
    }
}

impl Drop for LocalTransport {
    fn drop(&mut self) {
        if let Some(b) = self.battle.as_deref() {
            let _ = take_spilled(battle_key(b));
        }
    }
}

impl Transport for LocalTransport {
    fn send(&mut self, cmd: &Command) -> Result<(), TransportError> {
        match cmd {
            Command::Start { format, seed } => {
                if let Some(old) = self.battle.as_deref() {
                    let _ = take_spilled(battle_key(old));
                }
                // `|t:|` carries a wall clock; 0 keeps the log a function of
                // the seed and the choices.
                let mut b = Box::new(Battle::from_start(*format, *seed, 0));
                b.spill = Some(spill);
                self.battle = Some(b);
                self.ended_sent = false;
            }
            cmd => {
                let b = self.battle.as_mut().ok_or(TransportError::NoBattle)?;
                b.step(cmd).map_err(TransportError::Team)?;
            }
        }
        self.flush();
        Ok(())
    }

    fn poll(&mut self) -> Option<Incoming> {
        self.inbox.pop_front()
    }
}

// ── TextTransport ────────────────────────────────────────────────────────────

/// Any byte stream to a `BattleStream`-speaking server (a pipe, a UART, a
/// socket). Both calls are non-blocking.
pub trait ByteLink {
    /// Writes all of `bytes`.
    fn write(&mut self, bytes: &[u8]);
    /// Reads what has arrived into `buf`; returns the count (0: nothing).
    fn read(&mut self, buf: &mut [u8]) -> usize;
}

#[cfg(any(test, feature = "ps_text"))]
pub use text::TextTransport;

#[cfg(any(test, feature = "ps_text"))]
mod text {
    use alloc::collections::VecDeque;
    use alloc::string::String;
    use alloc::vec::Vec;

    use ps_protocol::text::command::encode_command;
    use ps_protocol::text::json::decode_request;
    use ps_protocol::text::line::{decode_line, CodecState};
    use ps_protocol::text::names::Tables;
    use ps_protocol::{Command, SideId};

    use super::{ByteLink, Incoming, Transport, TransportError};

    /// Where the reader is in `BattleStream`'s output: each message is its
    /// type line, then its data lines.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Reading {
        /// Between messages, or in a message this transport ignores.
        Nothing,
        /// `update`: log lines.
        Update,
        /// `sideupdate`: the side line comes next.
        SideId,
        /// `sideupdate` for this side: its one line comes next.
        SideLine(SideId),
        /// `end` (or `ps_sim_cli`'s `unported` / `crash`): its data line.
        End,
    }

    /// PS's text protocol over a [`ByteLink`]: commands encoded as
    /// `BattleStream` lines (`ps_protocol::text::command`), its messages
    /// decoded back into the same typed [`Incoming`] [`super::LocalTransport`]
    /// yields.
    ///
    /// The codec numbers moves, species, items and abilities by tables
    /// built from the format's `ps_dex` dex (`ps_sim::text::dex_tables`), so
    /// a decoded `MoveNum(n)` is the dex's move `n`, as on the typed path.
    /// Nicknames come from the `>player` teams this transport sends.
    pub struct TextTransport<L: ByteLink> {
        link: L,
        codec: Option<CodecState<Tables>>,
        rx: Vec<u8>,
        reading: Reading,
        inbox: VecDeque<Incoming>,
        undecoded: u32,
    }

    impl<L: ByteLink> TextTransport<L> {
        pub fn new(link: L) -> Self {
            Self { link, codec: None, rx: Vec::new(), reading: Reading::Nothing, inbox: VecDeque::new(), undecoded: 0 }
        }

        pub fn link(&self) -> &L {
            &self.link
        }

        pub fn link_mut(&mut self) -> &mut L {
            &mut self.link
        }

        /// Protocol lines the codec could not read (skipped, so the client
        /// never saw them). Zero against `ps_sim`; a real server may send
        /// line kinds the codec does not know yet.
        pub fn undecoded(&self) -> u32 {
            self.undecoded
        }

        /// Reads what the link has and decodes every complete line.
        fn pump(&mut self) {
            let mut buf = [0u8; 512];
            loop {
                let n = self.link.read(&mut buf);
                if n == 0 {
                    break;
                }
                self.rx.extend_from_slice(&buf[..n]);
            }
            while let Some(nl) = self.rx.iter().position(|&c| c == b'\n') {
                let raw: Vec<u8> = self.rx.drain(..=nl).collect();
                let line = String::from_utf8_lossy(&raw[..raw.len() - 1]).into_owned();
                self.line(line.trim_end_matches('\r'));
            }
        }

        /// One line of `BattleStream` output.
        fn line(&mut self, line: &str) {
            match (self.reading, line) {
                (Reading::SideId, side) => {
                    self.reading = SideId::parse(side).map_or(Reading::Nothing, Reading::SideLine);
                    return;
                }
                (Reading::SideLine(side), l) => {
                    self.reading = Reading::Nothing;
                    self.side_line(side, l);
                    return;
                }
                (Reading::End, _) => {
                    self.reading = Reading::Nothing;
                    return;
                }
                _ => {}
            }
            match line {
                "update" => self.reading = Reading::Update,
                "sideupdate" => self.reading = Reading::SideId,
                // `ps_sim_cli` ends a battle it cannot play with `unported`
                // or `crash` in place of `end`.
                "end" | "unported" | "crash" => {
                    self.reading = Reading::End;
                    self.inbox.push_back(Incoming::End);
                }
                l if l.starts_with('|') && self.reading == Reading::Update => {
                    let Some(codec) = self.codec.as_mut() else { return };
                    match decode_line(l, codec) {
                        Ok(msg) => self.inbox.push_back(Incoming::Log(msg)),
                        Err(_) => self.undecoded += 1,
                    }
                }
                // Blank lines and anything outside a known message.
                _ => {}
            }
        }

        fn side_line(&mut self, side: SideId, line: &str) {
            let Some(codec) = self.codec.as_mut() else { return };
            if let Some(json) = line.strip_prefix("|request|") {
                let (dex, names) = codec.split_mut();
                // PS sends `null` for a side with no request.
                match decode_request(json, dex, names) {
                    Ok(req) => self.inbox.push_back(Incoming::Request(req)),
                    Err(_) if json == "null" => {}
                    Err(_) => self.undecoded += 1,
                }
            } else {
                match decode_line(line, codec) {
                    Ok(msg) => self.inbox.push_back(Incoming::Side(side, msg)),
                    Err(_) => self.undecoded += 1,
                }
            }
        }
    }

    impl<L: ByteLink> Transport for TextTransport<L> {
        fn send(&mut self, cmd: &Command) -> Result<(), TransportError> {
            match cmd {
                Command::Start { format, .. } => {
                    let dex: &'static ps_dex::Dex =
                        if format.base.gen() == 1 { &ps_dex::GEN1 } else { &ps_dex::GEN3 };
                    self.codec = Some(CodecState::new(ps_sim::text::dex_tables(dex)));
                    self.rx.clear();
                    self.reading = Reading::Nothing;
                }
                Command::Player { side, name, team } => {
                    let codec = self.codec.as_mut().ok_or(TransportError::NoBattle)?;
                    let names = codec.battle_mut();
                    names.set_player(*side, name, "", "");
                    for set in team.iter() {
                        let nick = if set.name.is_empty() { set.species } else { set.name };
                        names.set_nickname(*side, &nick);
                    }
                }
                _ => {}
            }
            let mut line = encode_command(cmd).map_err(|_| TransportError::Encode)?;
            line.push('\n');
            self.link.write(line.as_bytes());
            Ok(())
        }

        fn poll(&mut self) -> Option<Incoming> {
            if self.inbox.is_empty() {
                self.pump();
            }
            self.inbox.pop_front()
        }
    }
}
