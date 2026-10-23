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
//! for Rust unit tests lives behind `test-override` (or `cfg(test)`).
//!
//! P1-2 brought the PRNG (`prng`). P1-5 brings the event core: battle state
//! (`state`), effects and their handlers (`effect`), the dispatch family and
//! `speedSort` (`events`), relay values (`relay`), fixed-point modifiers
//! (`modify`) and the per-gen effect registries (`mods`).

#![no_std]

extern crate alloc;

pub mod effect;
pub mod events;
pub mod mods;
pub mod modify;
pub mod prng;
pub mod relay;
pub mod state;

pub use effect::{EffectDef, EffectId, EventArgs, EventId, EventMask, Handler, HandlerFn, Prefix, Registry};
pub use relay::Relay;
pub use state::{Battle, EffectState, EffectTable, Field, Holder, MonRef, Pokemon, Side};
