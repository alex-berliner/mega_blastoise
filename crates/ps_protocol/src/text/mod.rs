//! PS sim protocol text codec.
//!
//! Enabled with the `text` feature flag (the brief's deliverable lives behind
//! it). The codec is `no_std + alloc`: everything but the JSON parser and TSV
//! loader works on the device; those run only in tests / harness.
//!
//! Public surface:
//! - [`line::encode_msg`] / [`line::decode_line`]: the per-line codec.
//! - [`line::encode_log`]: an `update` block's lines with `BattleStream`'s
//!   `|split|pN` framing.
//! - [`json::encode_request`] / [`json::decode_request`]: the request JSON,
//!   in PS's insertion order, byte-exact.
//! - [`command::encode_command`] / [`command::decode_command`]: `BattleStream`
//!   commands.
//! - [`team::pack`] / [`team::unpack`]: PS's packed team format.
//! - [`names::Tables`] / [`names::Battle`]: the two `name` traits this crate
//!   needs; a TSV loader and a `[player|switch]` parser fill them in.
//!
//! The protocol types in [`crate`] do not depend on this module, so the device
//! can compile `ps_protocol` without `alloc` for a small `Msg`-only build.

#![cfg(feature = "text")]

extern crate alloc;

pub mod command;
pub mod json;
pub mod line;
pub mod names;
pub mod team;

pub use line::encode_log;
