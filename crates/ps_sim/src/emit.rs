//! Output: what `BattleStream` would write after a command (design 5.6).
//!
//! PS sends two kinds of message. `sideupdate`s go out the moment they are
//! made (a choice error, gen 1-4's Struggle notice, a re-sent request);
//! they wait in the [`Outbox`] here. The `update` block (the log since the
//! last flush) and, after a new `makeRequest`, both sides' requests go out
//! when the command is done (`sendUpdates`). [`Battle::send_updates`] does
//! both, in PS's order, into a [`Sink`]: the CLI renders text, the device
//! hands typed messages to the client. Requests are built when sent
//! ([`Battle::request`]). A log that fills in the middle of a command hands
//! its oldest lines to the driver's [`crate::state::LogSpill`] first.
//!
//! PS: sim/battle.ts `sendUpdates`, `add`, `addSplit`, `hint`;
//! sim/side.ts `send`, `emitRequest`, `emitChoiceError`;
//! sim/battle-stream.ts `_write`.

use ps_protocol::{Msg, Request, SideId, Text, TextId};

use crate::state::Battle;

/// Where a flush goes. Generic, so the device path has no trait objects.
pub trait Sink {
    /// One `update` block: the log lines since the last flush, less those a
    /// full log already handed to [`crate::state::LogSpill`] (which come
    /// first; `lines` may then be empty).
    fn update(&mut self, lines: &[Msg]);
    /// A `sideupdate` line for `side` (an `|error|`, gen 1-4's
    /// `|-activate|...|move: Struggle`).
    fn side_line(&mut self, side: SideId, line: &Msg);
    /// A `sideupdate` `|request|` for `side`.
    fn request(&mut self, side: SideId, request: &Request);
}

/// One waiting `sideupdate`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SideOut {
    Line(Msg),
    /// `emitRequest`: the side's `activeRequest` as it is at flush time.
    Request,
}

/// `sideupdate`s one command can make: a choice error and the request it
/// re-sends, or the Struggle notice (a command is one side's).
pub const OUTBOX_CAP: usize = 4;

/// The `sideupdate`s made since the last flush, in order.
#[derive(Clone, Copy, Debug)]
pub struct Outbox {
    len: u8,
    items: [(SideId, SideOut); OUTBOX_CAP],
}

impl Default for Outbox {
    fn default() -> Self {
        Self::new()
    }
}

impl Outbox {
    pub const fn new() -> Outbox {
        Outbox { len: 0, items: [(SideId::P1, SideOut::Request); OUTBOX_CAP] }
    }

    fn push(&mut self, side: SideId, out: SideOut) {
        let n = self.len as usize;
        assert!(n < OUTBOX_CAP, "sideupdate outbox full ({OUTBOX_CAP})");
        self.items[n] = (side, out);
        self.len += 1;
    }

    /// PS `side.send(...)`: one line to `side`.
    pub fn push_side(&mut self, side: SideId, line: Msg) {
        self.push(side, SideOut::Line(line));
    }

    pub fn as_slice(&self) -> &[(SideId, SideOut)] {
        &self.items[..self.len as usize]
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl Battle {
    /// PS `side.emitRequest(activeRequest, updatedRequest)`: re-sends the
    /// side's request now, with `update: true` when asked.
    // PS: sim/side.ts emitRequest
    pub fn emit_request(&mut self, side: u8, updated: bool) {
        if updated {
            match &mut self.sides[side as usize].active_request {
                Some(crate::request::SideRequestState::Move { update, .. })
                | Some(crate::request::SideRequestState::Switch { update, .. }) => *update = true,
                _ => {}
            }
        }
        let id = self.sides[side as usize].id();
        self.outbox.push(id, SideOut::Request);
    }

    /// PS `hint(hint, once, side)`.
    // PS: sim/battle.ts hint
    pub fn hint(&mut self, text: TextId, once: bool, side: Option<SideId>) {
        self.hint_text(Text::new(text), once, side);
    }

    /// PS `hint(hint, once, side)` of a hint with arguments (the `once`
    /// set holds the whole text).
    // PS: sim/battle.ts hint
    pub fn hint_text(&mut self, text: Text, once: bool, side: Option<SideId>) {
        if self.hints.iter().flatten().any(|&h| h == (text, side)) {
            return;
        }
        self.add(Msg::Hint { text, split: side });
        if once {
            let slot = self.hints.iter_mut().find(|h| h.is_none()).expect("hint set full");
            *slot = Some((text, side));
        }
    }

    /// PS `debug(...)`: a `|debug|` line in debug mode (both custom games).
    // PS: sim/battle.ts debug
    pub fn debug(&mut self, text: Text) {
        if self.debug_mode {
            self.add(Msg::Debug(text));
        }
    }

    /// PS `sendUpdates()` after a command, with `BattleStream`'s ordering:
    /// the `sideupdate`s made while the command ran, then the `update`
    /// block, then (after a new request) both sides' requests.
    // PS: sim/battle.ts sendUpdates
    pub fn send_updates<S: Sink>(&mut self, sink: &mut S) {
        let outbox = self.outbox;
        self.outbox.clear();
        for &(side, out) in outbox.as_slice() {
            match out {
                SideOut::Line(m) => sink.side_line(side, &m),
                SideOut::Request => {
                    // PS sends `JSON.stringify(null)` for a side without a
                    // request; no command re-sends one then.
                    if let Some(r) = self.request(side.index() as u8) {
                        sink.request(side, &r);
                    }
                }
            }
        }
        if self.log.as_slice().is_empty() && !self.spilled {
            return;
        }
        sink.update(self.log.as_slice());
        self.log.clear();
        self.spilled = false;
        // The lines are sent; `attrLastMove` can no longer reach them.
        self.last_move_line = None;
        if !self.sent_requests {
            for s in 0..2u8 {
                if let Some(r) = self.request(s) {
                    sink.request(self.sides[s as usize].id(), &r);
                }
            }
            self.sent_requests = true;
        }
    }
}
