# mega_blastoise

**[▶ Play live](https://alex-berliner.github.io/mega_blastoise/)** — browser-based Gen 1 random battle simulator

Workspace layout: `mega_blastoise_core` (no_std + battle data), `mega_blastoise_fw` (RP2040), `mega_blastoise_test` (PC).

Architecture overview: see [architecture/README.md](./architecture/README.md).

## Documentation

- [TECHNICAL.md](./TECHNICAL.md): software internals, crate layout, interfaces, data flow
- [architecture/](./architecture/README.md): deep-dive architecture docs
- [hardware/ref_design/](./hardware/ref_design/README.md): the reference design (board, case, docs)
- [docs/hardware/openscad-methodology.md](./docs/hardware/openscad-methodology.md): how the OpenSCAD cases are built and checked
- [docs/archive/v1/](./docs/archive/v1/): the v1 device (Pico, two OLEDs, button matrix): its design and electronics notes.
  Its board, stripboard and case files were removed from the tree; tag `v1-archive` has them.

## Docker / Podman

Run the interactive battle without installing Rust or any native deps:

```bash
# Build once
podman build -t mega-blastoise-test .   # or: docker build ...

# Play
podman run -i --rm mega-blastoise-test
```

`-i` is required — the binary reads moves from stdin.

## Run modes (`mega-blastoise-test`)

The host binary is **interactive only**: stdin battle with the same **typed `BoardEvent` → `BoardEffects`** path as firmware (queue, prompts, combat log).

| Mode | Command | What it does |
|------|-----------|----------------|
| **Interactive** (default) | `cargo run -p mega-blastoise-test`<br>`cargo run -p mega-blastoise-test --release` | Full battle over stdin (pick moves / switches when prompted). |
| **Automated tests** | `cargo test -p mega-blastoise-test` | Runs Rust tests (board-event parsing, scripted effect pipeline, demo battle init). Pass/fail exit status. |

Firmware (`mega-blastoise-fw`) has no CLI modes; flash and run as usual (see below).

## Compile

From this directory (`mega_blastoise/mega_blastoise/`, where the workspace `Cargo.toml` lives):

**PC test (host):**

```bash
cargo build -p mega-blastoise-test
cargo run -p mega-blastoise-test
```

Add `--release` to `cargo run` for a faster binary. See **Run modes** for `cargo test`.

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

Flash/run with `probe-rs` as configured in `mega_blastoise_fw/.cargo/config.toml`.

**Install `probe-rs`:** download and run the latest installer from the [probe-rs releases page](https://github.com/probe-rs/probe-rs/releases), then reload your shell:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.sh | sh
source ~/.bashrc   # or ~/.zshrc
```

Then flash with a debug probe connected (e.g. Picoprobe, CMSIS-DAP):

```bash
cd mega_blastoise_fw
cargo run --release
```
