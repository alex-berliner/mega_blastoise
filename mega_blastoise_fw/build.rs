use std::{env, fs, path::PathBuf};

fn main() {
    // One memory map per chip: the RP2040 needs the BOOT2 window carved out of
    // flash, the RP2350 instead needs the IMAGE_DEF sections the boot ROM
    // scans the first 4K for. The chip is a cargo feature, so pick here.
    let memory: &[u8] = if env::var("CARGO_FEATURE_RP2350").is_ok() {
        include_bytes!("memory-rp2350.x")
    } else {
        include_bytes!("memory-rp2040.x")
    };
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("memory.x"), memory).unwrap();
    println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rerun-if-changed=memory-rp2040.x");
    println!("cargo:rerun-if-changed=memory-rp2350.x");
    println!("cargo:rerun-if-changed=build.rs");
}
