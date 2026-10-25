//! Pokemon Showdown protocol client.
//!
//! Per `docs/architecture/showdown-split-plan.md` section 7: [`BattleView`]
//! holds what a PS client knows (per-side active mon details + HP, status,
//! boosts, visible volatiles, known moves + PP, field and side conditions,
//! turn), [`BattleView::apply`] updates it from one `Msg` and yields zero or
//! more [`BoardEvent`]s. `Request` -> [`slot_options`] (the seat's
//! `choice_collect::SlotOptions`) -> player input -> [`choice_string`] (PS
//! choice) -> `Command`. The client is protocol-exact and transport-agnostic.
//!
//! ZERO-DRIFT (CLAUDE.md): this crate only TRANSLATES between protocol
//! and the existing UI layer. Prompts, validation, accept/reject,
//! waiting screen and end screens stay in `oled_ctl` / `choice_collect`.
//!
//! The runner's loop, per update: every `Msg` through [`BattleView::apply`];
//! every `|request|` through [`BattleView::apply_request`] (prompt events),
//! then [`slot_options`] for the seat; the answer through [`choice`] /
//! [`choice_string`].

#![no_std]

extern crate alloc;

pub mod board;
pub mod request;
pub mod view;

pub use board::{BoardEvent, BoardEvents, MoveSlot, PromptKind};
pub use request::{choice, choice_string, slot_options, PartyMon, SeatOptions};
pub use view::{BattleView, KnownMove, MonView, Outcome, SideView};
