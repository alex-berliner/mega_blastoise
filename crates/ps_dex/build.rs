//! Placeholder build script for the dex crate.
//!
//! Real implementation lands in P1-3: read per-gen JSON dumps under
//! `data/{gen1,gen3}/*.json` (produced by `tools/ps-diff/dump-dex.js`)
//! and emit `&'static` lookup tables. Until then this just keeps Cargo
//! honest that `build = "build.rs"` points at something.

fn main() {
    // Tell Cargo to rerun this build script if the data folder changes
    // (so adding dump files in P1-3 rebuilds without manual touches).
    println!("cargo:rerun-if-changed=data");
    println!("cargo:rerun-if-changed=build.rs");
}
