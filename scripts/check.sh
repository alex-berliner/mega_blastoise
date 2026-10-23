#!/usr/bin/env bash
# The whole pre-commit gate, one command. Exists because two test binaries
# rotted silently this month: the gen3 engine's own tests missed a struct
# field for days, and a refactor shipped with host_device_stubs broken —
# both because nothing routinely built every test in the workspace. The
# firmware crate is excluded from the test run (its cortex-m asm does not
# build for the host) and checked for its real target instead.
#
#   scripts/check.sh          # everything below
#   scripts/check.sh --fast   # skip the wasm build and the headless probe
set -euo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "── workspace tests (host) ──"
cargo test --quiet --workspace --exclude mega-blastoise-fw

echo "── firmware check (thumbv8m, the v2 board's default build) ──"
(cd crates/mega_blastoise_fw && cargo check --quiet)

echo "── no_std proof: ps_sim, ps_client, ps_dex, ps_protocol for thumbv8m ──"
cargo check --quiet -p ps_sim -p ps_client -p ps_dex -p ps_protocol --target thumbv8m.main-none-eabihf

if [[ "${1:-}" != "--fast" ]]; then
  echo "── wasm build + headless seat probe ──"
  (cd crates/mega_blastoise_web && wasm-pack build --target web --dev >/dev/null 2>&1)
  scripts/ui-probe.sh --skip-build
fi

echo "check: OK"
