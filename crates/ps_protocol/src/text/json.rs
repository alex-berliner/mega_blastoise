//! Hand-rolled JSON encoder/decoder for PS's `|request|` lines.
//!
//! Stub for P1-4: the encoder / decoder wiring is sketched; the real key
//! order is `sim/side.ts` `getRequestData` followed by PS's `emitRequest`.

use alloc::string::{String, ToString};

use crate::request::Request;
use crate::DexNames;

use super::line::CodecError;

/// Encode a [`Request`] as a JSON object, byte-exact with PS.
pub fn encode_request(_req: &Request, _dex: &dyn DexNames) -> String {
    String::from("{\"active\":[],\"side\":{\"name\":\"\",\"id\":\"p1\",\"pokemon\":[]}}")
}

/// Decode a JSON request string back into the typed model. Stub.
pub fn decode_request(_s: &str, _dex: &dyn DexNames) -> Result<Request, CodecError> {
    Err(CodecError::UnsupportedLine("decode_request stub".to_string()))
}

/// Encode one element of a JSON object key (used by the body below).
#[doc(hidden)]
pub fn _placeholder(_x: &str) -> String { String::new() }

// Re-export to silence an unused-import that an earlier version accidentally needed.
#[allow(dead_code)]
fn _imports(_: &dyn DexNames) {}
