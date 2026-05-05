# Todos

## 1. Rewrite debug.md with timeout-based debug loop

Current debug.md uses picocom and relies on probe-rs holding RTT open continuously.
Replace with a Claude-friendly loop that uses timeout-bounded commands so Claude can
drive the debug cycle autonomously.

**Debug loop shape:**
```
1. pkill picocom; pkill -9 -f probe-rs
2. flash: probe-rs download --preset pico <elf> && probe-rs reset --preset pico
3. read USB init:  stty -F /dev/ttyACM1 raw -echo min 0 time 10 && timeout 2 cat /dev/ttyACM1
4. write to USB:   echo -ne "1\r\n" > /dev/ttyACM1
5. read USB reply: stty -F /dev/ttyACM1 raw -echo min 0 time 5 && timeout 1 cat /dev/ttyACM1
6. read RTT:       timeout 2 probe-rs attach --preset pico --rtt-scan-memory (or equivalent)
7. assert expected output; iterate
```

Commands to standardize:
- USB read:  `stty -F /dev/ttyACM1 raw -echo min 0 time 10 && timeout N cat /dev/ttyACM1`
- USB write: `echo -ne "message\r\n" > /dev/ttyACM1`
- RTT read:  `timeout N probe-rs attach --preset pico` (figure out exact flags)

---

## 2. Give the firmware target a name and alias it in config.toml

The target triple `thumbv6m-none-eabi` is spelled out everywhere. Give it a named alias
in `.cargo/config.toml` so commands like `cargo build` and `cargo run` don't require
`--target thumbv6m-none-eabi` and the target is unambiguous.

Look into `[alias]` entries and whether `build.target` in config.toml already covers this
for the fw crate — and whether the workspace-level config is interfering.

---

## 3. Fix `cargo build` failing at workspace root

`cargo build` from `~/Code/mega_blastoise/mega_blastoise/` pulls in crates that don't
compile for the host target (firmware-only crates, `#![no_std]`, etc.). Works fine from
within `mega_blastoise_fw/`. 

Fix options:
- Exclude fw crates from the workspace default members (`default-members` in root `Cargo.toml`)
- Or gate them with a target-specific member so they only build when targeting thumbv6m

Goal: `cargo build` at the workspace root should compile only host-side crates cleanly.

---

## 4. Default binary for `probe-rs attach` (no need to pass ELF path every time)

Currently `probe-rs attach` requires an explicit ELF path. Find a way to default this:
- Check if probe-rs supports a default binary in `.probe-rs.toml` presets
- Or add a `Makefile` / cargo alias like `cargo attach` that injects the last-built ELF path
- Or a shell alias/script in the repo

Goal: `probe-rs attach --preset pico` (no ELF arg) works and attaches to the correct binary.
