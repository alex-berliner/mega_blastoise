use std::{env, fs, path::PathBuf};

fn main() {
    // The RP2350 boot ROM scans the first 4K of flash for the IMAGE_DEF
    // block; memory-rp2350.x places that section.
    let memory: &[u8] = include_bytes!("memory-rp2350.x");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("memory.x"), memory).unwrap();
    println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rerun-if-changed=memory-rp2350.x");
    println!("cargo:rerun-if-changed=build.rs");
}
