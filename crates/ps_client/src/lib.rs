//! Pokemon Showdown protocol client.
//!
//! Per `docs/architecture/showdown-split-plan.md` section 7: `BattleView`
//! holds what a PS client knows (per-side active mon details + HP, status,
//! boosts, visible volatiles, known moves + PP, field and side conditions,
//! turn), `apply(&Msg)` updates it and yields zero or more `BoardEvent`s.
//! `Request` -> `choice_collect::SlotOptions` -> player input -> PS choice
//! string -> `Command`. The client is protocol-exact and
//! transport-agnostic.
//!
//! ZERO-DRIFT (CLAUDE.md): this crate only TRANSLATES between protocol
//! and the existing UI layer. Prompts, validation, accept/reject,
//! waiting screen and end screens stay in `oled_ctl` / `choice_collect`.
//!
//! This crate starts empty in P1-0 (scaffold only); P2-1 and P2-2 fill
//! in the view + choice model.

#![no_std]

extern crate alloc;
