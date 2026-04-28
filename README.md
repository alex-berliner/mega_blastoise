# mega_blastoise

Workspace layout: `mega_blastoise_core` (no_std + battle data), `mega_blastoise_fw` (RP2040), `mega_blastoise_test` (PC).

## Compile

From this directory (`mega_blastoise/mega_blastoise/`, where the workspace `Cargo.toml` lives):

**PC test (host):**

```bash
cargo build -p mega-blastoise-test
cargo run -p mega-blastoise-test
```

**Pico / RP2040 firmware:** install the Rust target once:

```bash
rustup target add thumbv6m-none-eabi
```

Then either pass the target explicitly (recommended when building from the **workspace root** — Cargo does not apply `mega_blastoise_fw/.cargo/config.toml`’s default target for `-p` builds):

```bash
cargo build -p mega-blastoise-fw --target thumbv6m-none-eabi
cargo build -p mega-blastoise-fw --target thumbv6m-none-eabi --release
```

**Firmware size stats** (prints section totals after link — install `rustup component add llvm-tools-preview` for `llvm-size`, or use `arm-none-eabi-size` from the GNU Arm toolchain):

```bash
./scripts/fw-build.sh                    # debug + llvm-size -t / -A
PROFILE=release ./scripts/fw-build.sh    # release build + stats
./scripts/fw-build.sh --release          # same (cargo passes --release)
```

Debug binaries include large `.debug_*` sections, so `llvm-size` totals look huge; prefer **`./scripts/fw-build.sh --release`** when checking flash/RAM against the Pico.

Or `cd` into the firmware crate so its `.cargo/config.toml` sets `thumbv6m-none-eabi` automatically:

```bash
cd mega_blastoise_fw
cargo build
cargo build --release
```

Flash/run with your usual tool (e.g. `probe-rs` as configured in `mega_blastoise_fw/.cargo/config.toml`).
