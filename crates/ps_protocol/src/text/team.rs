//! Packed-team codec: PS `sim/teams.ts` `pack` / `unpack`.
//!
//! Stub for P1-4: deliver a complete implementation in a follow-up; this
//! stub exposes only the placeholder types so the rest of the crate
//! compiles.

use alloc::string::String;
use alloc::vec::Vec;

use crate::team::{PackedSet, PackedTeam, PokemonSet};
use super::line::CodecError;

/// Encodes a packed team back to its wire form (`]|]...`).
pub fn pack(_team: &PackedTeam) -> Result<String, CodecError> {
    Err(CodecError::BadTeam("not implemented".into()))
}

/// Decodes a packed team string into a packed-team structure.
pub fn unpack(_s: &str) -> Result<PackedTeam, CodecError> {
    Err(CodecError::BadTeam("not implemented".into()))
}

/// Decodes a packed team into the typed form the sim uses.
pub fn unpack_typed(_team: &PackedTeam) -> Result<Vec<PokemonSet>, CodecError> {
    Err(CodecError::BadTeam("not implemented".into()))
}

/// Encodes one [`PackedSet`] to a single wire field.
pub fn pack_set(_set: &PackedSet) -> Result<String, CodecError> {
    Err(CodecError::BadTeam("not implemented".into()))
}
