# mega_blastoise

**[▶ Play live](https://alex-berliner.github.io/mega_blastoise/)** — browser-based Gen 1 random battle simulator

Workspace layout: `mega_blastoise_core` (no_std + battle data), `mega_blastoise_fw` (RP2040), `mega_blastoise_test` (PC).

Architecture overview: see [architecture/README.md](./architecture/README.md).

## Documentation

- [DESIGN.md](./DESIGN.md) — project overview, gameplay, hardware summary, and status
- [ELECTRONICS.md](./ELECTRONICS.md) — GPIO map, wiring, power budget, physical layout
- [TECHNICAL.md](./TECHNICAL.md) — software internals, crate layout, interfaces, data flow
- [specs.md](./specs.md) — full specification and architecture reference
- [architecture/](./architecture/README.md) — deep-dive architecture docs (7 pages)

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

### PN532 readers (firmware)

Two **PN532** modules on separate I²C buses (`I2C0` + `I2C1`), default address **0x24** each:

| Bus | SCL (GP) | SDA (GP) |
|-----|----------|----------|
| I2C0 (reader 0) | GP17 | GP16 |
| I2C1 (reader 1) | GP19 | GP18 |

Startup spawns two embassy tasks that periodically exchange **GetFirmwareVersion** and discard the reply — bring-up traffic only until NFC handling lands.

### Breadboard power (PN532 + Pico)

PN532 boards draw noticeable current when the RF field is active; pulling both from the Pico’s **3V3** pin alone can brown out the Pico regulator or cause flaky I²C.

**Plan for the breadboard stage:**

1. **External brick** — Use a small DC supply (e.g. barrel jack wall wart into a **buck module** or **LM2596-style** board) that exposes stable **5&nbsp;V** and/or **3.3&nbsp;V** rails at enough current for **both** readers plus headroom (hundreds of mA total is a comfortable target for NFC bring-up).

2. **Power the readers from that rail**, not from Pico **3V3**:
   - Many PN532 breakouts accept **5&nbsp;V** on **VIN** and regulate down on-board; follow your module’s silkscreen.
   - If the breakout is **3.3&nbsp;V only**, power it from the external **3.3&nbsp;V** output of the same supply family (not from the Pico pin).

3. **Single ground reference** — Tie **GND** from the barrel/supply module, **Pico GND**, and **both reader GNDs** on one common ground net (breadboard negative rail). I²C only works if grounds are shared.

4. **Pico supply during development** — Easiest: power the Pico over **USB** for flashing/debug while the **external supply feeds only the NFC modules** (still with common GND). Alternatively, feed **5&nbsp;V** into **VSYS** through an appropriate Schottky diode arrangement per Pico docs if you want fully standalone power (more breadboard care).

5. **I²C levels** — RP2040 GPIO is **3.3&nbsp;V** logic. PN532 I²C on common breakouts is 3.3&nbsp;V tolerant; if you ever used a 5&nbsp;V‑only I²C module you would need level shifting (most PN532 boards are 3.3&nbsp;V I²C).

Keep high‑current paths short on the breadboard and avoid routing reader supply current through the Pico pins.
