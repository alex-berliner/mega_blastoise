fn main() {
    // Human-readable EST build time via `date`.
    let datetime = std::process::Command::new("date")
        .env("TZ", "America/New_York")
        .args(["+%Y-%m-%d %H:%M EST"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=BUILD_DATETIME={}", datetime.trim());

    // Short git hash.
    let hash = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "?".into());
    println!("cargo:rustc-env=GIT_HASH={}", hash.trim());

    // Re-run when a new commit lands.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
}
