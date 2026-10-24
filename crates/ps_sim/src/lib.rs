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
//! (`modify`) and the per-gen effect registries (`mods`). P1-6 brings the
//! control flow from `>start` to `|win|` / `|tie|`: the action queue
//! (`queue`), the turn loop, switching and faints (`battle`), choices
//! (`choice`), requests (`request`) and the output (`emit`), with stats
//! for sorting and requests (`stats`).

#![no_std]

extern crate alloc;

pub mod actions;
pub mod base_actions;
pub mod battle;
pub mod choice;
pub mod effect;
pub mod emit;
pub mod events;
pub mod mods;
pub mod modify;
pub mod prng;
pub mod queue;
pub mod relay;
pub mod request;
pub mod state;
pub mod stats;
#[cfg(any(test, feature = "text"))]
pub mod text;

pub use effect::{EffectDef, EffectId, Pseudo, EventArgs, EventId, EventMask, Handler, HandlerFn, Prefix, Registry};
pub use relay::Relay;
pub use battle::{MoveHook, SwitchResult, TeamError};
pub use emit::Sink;
pub use state::{Battle, EffectState, EffectTable, Field, Holder, MonRef, Pokemon, Side};
