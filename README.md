# mega_blastoise

**[▶ Play live](https://alex-berliner.github.io/mega_blastoise/battler/device.html)** — browser-based random battle simulator.
Everything published from this repo (the game, its UI notes, the hardware's 3D views) is linked from
[the site's front page](https://alex-berliner.github.io/mega_blastoise/), built from `site/`.

Layout:
- `crates/`: the Rust workspace. `ps_sim`, `ps_client`, `ps_dex`, `ps_protocol` (the Showdown
  port: server, client, dex and protocol), `ps_harness` (the protocol diff), `mega_blastoise_core` (no_std + battle data), `mega_blastoise_fw` (firmware), `mega_blastoise_test` (PC),
  and `mega_blastoise_web` (its own wasm workspace, the browser build).
- `firmware/bringup/`: standalone bring-up tools (`rp2350_blink`, `esp32_reset`).
- `hardware/`: the boards and cases ([index](hardware/README.md)). `docs/`: notes, including `docs/reference/architecture/`. `scripts/`: dev scripts.

Architecture overview: see [docs/reference/architecture/README.md](docs/reference/architecture/README.md).

## Documentation

- [docs/reference/TECHNICAL.md](docs/reference/TECHNICAL.md): software internals, crate layout, interfaces, data flow
- [docs/reference/architecture/](docs/reference/architecture/README.md): deep-dive architecture docs
- [hardware/ref_design/](hardware/ref_design/README.md): the reference design (board, case, docs)
- [docs/reference/openscad-methodology.md](docs/reference/openscad-methodology.md): how the OpenSCAD cases are built and checked
- [docs/archive/v1/](docs/archive/v1/): the v1 device (Pico, two OLEDs, button matrix): its design and electronics notes.
  Its board, stripboard and case files were removed from the tree; tag `v1-archive` has them.

## Run modes (`mega-blastoise-test`)

The host binary is **interactive only**: stdin battle with the same **typed `BoardEvent` → `BoardEffects`** path as firmware (queue, prompts, combat log).

| Mode | Command | What it does |
|------|-----------|----------------|
| **Interactive** (default) | `cargo run -p mega-blastoise-test`<br>`cargo run -p mega-blastoise-test --release` | A Gen 1 battle on the PS engine over stdin (`1`-`4` a move, `s1`-`s6` a switch, when prompted). |
| **Automated tests** | `cargo test -p mega-blastoise-test` | Runs Rust tests (board-event parsing, scripted effect pipeline, host input stubs and full automated PS battles). Pass/fail exit status. |

Firmware (`mega-blastoise-fw`) has no CLI modes; flash and run as usual (see below).

## Compile

From this directory (`mega_blastoise/mega_blastoise/`, where the workspace `Cargo.toml` lives):

**PC test (host):**

```bash
cargo build -p mega-blastoise-test
cargo run -p mega-blastoise-test
```

Add `--release` to `cargo run` for a faster binary. See **Run modes** for `cargo test`.

**Firmware (Pico 2 / RP2350, the stripboard v2 board):** the Rust target is listed in
`rust-toolchain.toml`; install it by hand if rustup did not:

```bash
rustup target add thumbv8m.main-none-eabihf
```

Build from inside the firmware crate, so its `.cargo/config.toml` applies (default target
`thumbv8m.main-none-eabihf`, the linker scripts, the probe-rs runner). The default features
(`rp2350`, `board_v2`) are the device build:

```bash
cd crates/mega_blastoise_fw
cargo build
cargo build --release
cargo bv2          # same, plus the held-button overlay on the panel (input-debug)
```

**Firmware size stats** (prints section totals after link; install `rustup component add llvm-tools-preview` for `llvm-size`, or use `arm-none-eabi-size` from the GNU Arm toolchain):

```bash
./scripts/fw-build.sh                    # debug + llvm-size -t / -A
PROFILE=release ./scripts/fw-build.sh    # release build + stats
./scripts/fw-build.sh --release          # same (cargo passes --release)
```

Debug binaries include large `.debug_*` sections, so `llvm-size` totals look huge; prefer **`./scripts/fw-build.sh --release`** when checking flash/RAM against the Pico 2.

Flash/run with `probe-rs` as configured in `crates/mega_blastoise_fw/.cargo/config.toml`.

**Install `probe-rs`:** download and run the latest installer from the [probe-rs releases page](https://github.com/probe-rs/probe-rs/releases), then reload your shell:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.sh | sh
source ~/.bashrc   # or ~/.zshrc
```

Then flash with a debug probe connected (e.g. Picoprobe, CMSIS-DAP):

```bash
cd crates/mega_blastoise_fw
cargo run --release
```
