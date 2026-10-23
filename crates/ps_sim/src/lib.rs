//! Pokemon Showdown-compatible battle server.
//!
//! Per `docs/architecture/showdown-split-plan.md` section 5: this crate owns
//! the battle state, the action queue (`BattleQueue` with PS's `orders`
//! table, priority, speed, subOrder, `speedSort` tie-shuffle), the event
//! pipeline (`runEvent`, `singleEvent`, `eachEvent`, `fieldEvent`,
//! `findEventHandlers`, `resolvePriority`), the `PRNG` (PS's `Gen5RNG`
//! bit-exactly, called in PS's order), damage formulas, and the protocol
//! emitter for the line set in design section 3.
//!
//! Per-gen overrides live in `mods::{gen1, gen3}` behind a `GenScripts`
//! enum dispatch (section 5.4). The text codec lives behind the `text`
//! feature so the on-device path stays typed. The PRNG outcome override
//! for Rust unit tests lives behind `test-override`.
//!
//! This crate starts empty in P1-0 (scaffold only); P1-2 brings the PRNG,
//! P1-3 brings the dex tables, later phases fill in state, queue and
//! events.

#![no_std]

extern crate alloc;
