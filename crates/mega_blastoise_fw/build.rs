use std::{env, fs, path::PathBuf};

fn main() {
    // The RP2350 boot ROM scans the first 4K of flash for the IMAGE_DEF
    // block; memory-rp2350.x places that section.
    let memory: &[u8] = include_bytes!("memory-rp2350.x");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("memory.x"), memory).unwrap();
    println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rerun-if-changed=memory-rp2350.x");
    // The commit this image was built from, for the phone bridge's log
    // (VER-251 logs the firmware commit); "+dirty" marks uncommitted changes.
    let git = |args: &[&str]| {
        std::process::Command::new("git").args(args).output().ok().filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let commit = git(&["rev-parse", "--short=10", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty = git(&["status", "--porcelain", "--untracked-files=no"]).is_some_and(|s| !s.is_empty());
    println!("cargo:rustc-env=MB_FW_COMMIT={commit}{}", if dirty { "+dirty" } else { "" });
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=build.rs");
}
