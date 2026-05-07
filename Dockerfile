# ── Stage 1: Build ───────────────────────────────────────────────────────────
FROM docker.io/library/rust:slim-bookworm AS builder

# serialport links against libudev on Linux
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libudev-dev \
    && rm -rf /var/lib/apt/lists/*

# The workspace uses nightly features (edition 2024 in battler, etc.); rust-toolchain.toml
# is excluded from the build context so we install nightly explicitly here.
RUN rustup toolchain install nightly --profile minimal && rustup default nightly

WORKDIR /build

# ── Workspace manifests (layer-cache deps before source) ─────────────────────
COPY Cargo.toml Cargo.lock ./
COPY patches/ patches/

# firmware Cargo.toml (workspace member, not compiled for x86; stub source added below)
COPY mega_blastoise_fw/Cargo.toml mega_blastoise_fw/Cargo.toml

# battler sub-workspace (excluded from the main workspace; referenced via path deps)
COPY battler/Cargo.toml battler/Cargo.toml
COPY battler/battler/Cargo.toml       battler/battler/Cargo.toml
COPY battler/battler-data/Cargo.toml  battler/battler-data/Cargo.toml
COPY battler/battler-choice/Cargo.toml battler/battler-choice/Cargo.toml
COPY battler/battler-prng/Cargo.toml  battler/battler-prng/Cargo.toml

COPY mega_blastoise_core/Cargo.toml mega_blastoise_core/Cargo.toml
COPY mega_blastoise_test/Cargo.toml mega_blastoise_test/Cargo.toml

# Stub entry-point for fw so Cargo can discover it as a workspace member
# without needing the full embedded source tree.
RUN mkdir -p mega_blastoise_fw/src \
 && printf 'fn main() {}\n' > mega_blastoise_fw/src/main.rs

# ── Sources ───────────────────────────────────────────────────────────────────
COPY mega_blastoise_core/ mega_blastoise_core/
COPY mega_blastoise_test/ mega_blastoise_test/

COPY battler/battler/src/      battler/battler/src/
COPY battler/battler-data/src/ battler/battler-data/src/
COPY battler/battler-choice/src/ battler/battler-choice/src/
COPY battler/battler-prng/src/ battler/battler-prng/src/

# JSON data used by mega_blastoise_core/build.rs
COPY battler/battle-data/ battler/battle-data/

RUN cargo build --release -p mega-blastoise-test --bin mega-blastoise-test

# ── Stage 2: Runtime ─────────────────────────────────────────────────────────
FROM docker.io/debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    libudev1 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder \
    /build/target/release/mega-blastoise-test \
    /usr/local/bin/mega-blastoise-test

ENTRYPOINT ["mega-blastoise-test"]
