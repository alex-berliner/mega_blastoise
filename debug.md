# Debug Loop for mega_blastoise_fw

## Key insight: two distinct modes

The `.cargo/config.toml` runner is already `probe-rs run`, which flashes **and** holds RTT in one command.
This creates two natural modes that should never be mixed in the same session — they compete for the probe.

---

## Mode A — Flash + RTT (recommended for defmt debugging)

```bash
pkill -9 -f "probe-rs" || true && sleep 1
cargo run -C mega_blastoise_fw 2>&1 | tee /tmp/rtt.log
```

- Flashes the ELF, resets, and streams defmt RTT to stdout indefinitely.
- The probe stays attached the whole time.
- In a **second terminal**, interact via serial:
  ```
  picocom --baud 115200 /dev/ttyACM1
  ```
- `fw_test` still works in a third terminal — its internal `probe-rs attach` will fail gracefully
  ("RTT unavailable") but serial/USB interaction is unaffected.
- Kill with `Ctrl-C` when done. The probe releases immediately.

**Use this mode when**: adding `defmt::debug!` instrumentation and you want to watch bytes/events in real time.

---

## Mode B — Flash-only + fw_test (recommended for automated testing)

```bash
pkill -9 -f "probe-rs" || true && sleep 1
cargo build -C mega_blastoise_fw

ELF=target/thumbv6m-none-eabi/debug/mega-blastoise-fw

probe-rs download --preset pico "$ELF"
probe-rs reset --preset pico

sleep 3
timeout 90 cargo run -p mega-blastoise-test --bin fw_test
```

- `download` + `reset` release the probe immediately after flashing.
- `fw_test` then spawns its own `probe-rs attach` for RTT.
- Add `--preverify` to `download` to skip re-flashing when the ELF hasn't changed.

---

## Useful flags reference

| Flag | Command | Effect |
|------|---------|--------|
| `--preverify` | `download` / `run` | Skip flash if target already matches ELF |
| `--verify` | `download` / `run` | Readback-verify after flashing |
| `--no-timestamps` | `run` / `attach` | Cleaner RTT output |
| `--no-location` | `run` / `attach` | Suppress file:line from defmt |
| `--log-format oneline` | `run` / `attach` | Compact single-line defmt format (already in runner) |
| `--target-output-file defmt=rtt.log` | `run` | Save RTT to file while streaming to stdout |
| `--connect-under-reset` | any | Assert nRESET during attach — helps if firmware is locked up |
| `--rtt-scan-memory` | `run` / `attach` | Scan full RAM for RTT block if normal detection fails |

---

## Diagnosing the mystery serial input bug

The firmware receives `[EVT] === Battle` text as input inside `read_line()`.

**Step 1** — add byte logging to `usb_input.rs` `read_line()`:
```rust
b if b >= 0x20 => {
    defmt::debug!("rx byte 0x{:02x} '{}'", b, b as char);
    self.partial.push(b as char);
    let _ = self.sender.write_packet(&[b]).await;
}
```

**Step 2** — flash and watch with Mode A:
```bash
pkill -9 -f "probe-rs" || true && sleep 1
cargo run -C mega_blastoise_fw 2>&1 | tee /tmp/rtt.log
```

**Step 3** — in a second terminal, run fw_test (RTT will show "unavailable", that's fine):
```bash
sleep 3 && timeout 90 cargo run -p mega-blastoise-test --bin fw_test
```

**Step 4** — watch `/tmp/rtt.log` for unexpected bytes arriving in `read_line()`.
The bug will show up as `rx byte` lines containing the ASCII values of `[`, `E`, `V`, `T`, `]`, etc.
That tells you whether the source is USB loopback, a second CDC ACM write path, or something else.

**Step 5** — remove the byte logging once the source is identified.

---

## Avoiding probe contention (the main source of flaky failures)

- Always `pkill -9 -f "probe-rs"` before any `probe-rs` command.
- Never run two `probe-rs` commands simultaneously against the same probe.
- `probe-rs run` (Mode A) keeps the probe; `probe-rs download`+`reset` (Mode B) releases it.
- If `fw_test` hangs at "waiting for battle prompts", the previous `probe-rs attach` subprocess
  is still alive — kill it and re-run.
