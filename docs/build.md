# Build & Run

## PC test (host)

```bash
cargo run -p mega-blastoise-test          # interactive stdin battle
cargo run -p mega-blastoise-test --release
cargo test -p mega-blastoise-test         # automated tests
```

| Mode | Command | What it does |
|------|---------|-------------|
| Interactive (default) | `cargo run -p mega-blastoise-test` | Full battle over stdin — pick moves/switches when prompted |
| Automated tests | `cargo test -p mega-blastoise-test` | Board-event parsing, scripted effect pipeline, demo battle init |

## RP2040 firmware

Install the target once:

```bash
rustup target add thumbv6m-none-eabi
```

Build from the workspace root (pass target explicitly — `.cargo/config.toml` inside `mega_blastoise_fw/` is not applied for `-p` builds):

```bash
cargo build -p mega-blastoise-fw --target thumbv6m-none-eabi
cargo build -p mega-blastoise-fw --target thumbv6m-none-eabi --release
```

Or `cd` into the crate so its `.cargo/config.toml` sets the target automatically:

```bash
cd mega_blastoise_fw
cargo build --release
```

### Firmware size stats

Requires `llvm-size` (`rustup component add llvm-tools-preview`) or `arm-none-eabi-size`:

```bash
./scripts/fw-build.sh            # debug build + section totals
./scripts/fw-build.sh --release  # release build + stats (use this for flash/RAM checks)
```

Debug binaries include large `.debug_*` sections; use `--release` when checking against Pico flash/RAM limits.

## Flashing

Install probe-rs:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.sh | sh
source ~/.bashrc
```

Flash with a debug probe connected (Picoprobe, CMSIS-DAP, etc.):

```bash
cd mega_blastoise_fw
cargo run --release
```
