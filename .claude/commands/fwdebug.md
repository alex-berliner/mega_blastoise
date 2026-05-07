# Debug Loop for mega_blastoise_fw

## Key insight: two distinct modes

The `.cargo/config.toml` runner is already `probe-rs run`, which flashes **and** holds RTT in one command.
This creates two natural modes that should never be mixed in the same session — they compete for the probe.

---

## Commands
run `source $MB_BASE/mega_blastoise/commands.sh`
`cat $MB_BASE/mega_blastoise/commands.sh` to discover the commands

## fw_test (recommended for automated testing)

```bash
# 1) first build and flash
mb_build
mb_download
mb_reset

# 2) read initial state of rtt and usb
mb_rttpoll
mb_usbpoll

# 3) then send your command over usb
mb_usb_send "<your data>"

# 4) read updated state of rtt and usb
mb_rttpoll
mb_usbpoll

# repeat 2-4 until you have determined the test has passed or failed
```

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
## Avoiding probe contention (the main source of flaky failures)

- Before any `probe-rs` command, kill existing probe-rs processes **only if they exist**:
  `pgrep -f "probe-rs" && pkill -9 -f "probe-rs" || true`
  Do NOT unconditionally `pkill` — if no process exists, pkill can hang or error and stall the loop.
- Never run two `probe-rs` commands simultaneously against the same probe.
- `probe-rs run` (Mode A) keeps the probe; `probe-rs download`+`reset` (Mode B) releases it.
- If `fw_test` hangs at "waiting for battle prompts", the previous `probe-rs attach` subprocess
  is still alive — kill it and re-run.

## Aggressive timeouts on hardware reads (mandatory)

Any command that reads from hardware (`probe-rs attach`, `cat /dev/ttyACM*`) **must** be wrapped
in `timeout <N>` — these will hang forever if the device is quiet or disconnected.

- RTT poll: `timeout 2 probe-rs attach ...`
- USB poll: `timeout 2 cat /dev/ttyACM1`
- stty setup: `timeout 1 stty -F /dev/ttyACM1 ...`

Do NOT add timeouts to flash/download commands (`mb_build`, `mb_download`, `mb_reset`) —
those are long-running by design and should be left to complete naturally.
