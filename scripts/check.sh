#!/usr/bin/env bash
# The whole pre-commit gate, one command. Exists because two test binaries
# rotted silently this month: the gen3 engine's own tests missed a struct
# field for days, and a refactor shipped with host_device_stubs broken,
# both because nothing routinely built every test in the workspace. The
# firmware crate is excluded from the test run (its cortex-m asm does not
# build for the host) and checked for its real target instead.
#
#   scripts/check.sh          # everything below
#   scripts/check.sh --fast   # skip the wasm build and the headless probe
#                             # AND the protocol-diff sweeps (the smoke
#                             # builds and tests still run)
set -euo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Path to the rust side. Used in the ps-cli lane of the protocol-diff
# sweep (the cargo build earlier in this script keeps it current).
PS_CLI_BIN="$PWD/target/debug/ps_sim_cli"

echo "── workspace tests (host) ──"
cargo test --quiet --workspace --exclude mega-blastoise-fw

echo "── firmware check (thumbv8m, the v2 board's default build) ──"
(cd crates/mega_blastoise_fw && cargo check --quiet)

echo "── no_std proof: ps_sim, ps_client, ps_dex, ps_protocol for thumbv8m ──"
cargo check --quiet -p ps_sim -p ps_client -p ps_dex -p ps_protocol --target thumbv8m.main-none-eabihf

# The ps_harness binary and integration tests ride every mode (fast and
# full): the integration tests are <2s and they pin the
# pinned.txt / ps_sim_cli contract that the protocol-diff sweep depends
# on. Building ps_sim_cli now means the rust side cannot disappear
# between this gate and the diff's --ps-cli step.
echo "── ps_sim_cli binary: build + protocol-diff integration test ──"
cargo build --quiet -p ps_harness --bin ps_sim_cli
cargo test --quiet -p ps_harness --test protocol_diff

# The coverage gate itself runs at phase gates (scripts/ps-coverage.sh);
# here only its allow-list: every entry parses and its source text is
# still in the tree, so an edit that breaks an entry fails the landing.
echo "── coverage-allow.txt resolves against the source ──"
python3 -I scripts/coverage-allow.py --check

if [[ "${1:-}" != "--fast" ]]; then
  echo "── pinned seeds (the legacy Gen 3 engine's corpus) ──"
  scripts/pinned-seeds.sh

  echo "── protocol-diff harness: per-gen sweeps + pinned corpus ──"
  # Smoke run: --self mode, 20 per gen for random and wide. Validates
  # the harness's own self-test against a PS-vs-PS run; no PS-cli
  # dependency, so it always runs regardless of how many ports exist.
  scripts/ps-diff.sh --self --gen 1 --n 20 --teams random --quiet --jobs 14
  scripts/ps-diff.sh --self --gen 3 --n 20 --teams random --quiet --jobs 14
  scripts/ps-diff.sh --self --gen 1 --n 20 --teams wide   --quiet --jobs 14
  scripts/ps-diff.sh --self --gen 3 --n 20 --teams wide   --quiet --jobs 14
  scripts/ps-diff.sh --self --gen 1 --n 20 --teams wide   --quiet --jobs 14 --trunc math

  # For each gen whose ps_sim_cli --ported list is non-empty, run the
  # 200-battle parity sweep against the rust side. An empty list is
  # the "no ported pipeline yet" signal and that gen is skipped with a
  # one-line notice; once a port lands, the same gate runs.
  for gen in 1 3; do
    ported_out=$("$PS_CLI_BIN" --ported "$gen")
    if [[ -n "$ported_out" ]]; then
      echo "── protocol-diff harness: ps-sim side gen $gen n=200 jobs=14 ──"
      scripts/ps-diff.sh --ps-cli "$PS_CLI_BIN" --gen "$gen" --n 200 --teams wide --quiet --jobs 14
      if [[ "$gen" == 1 ]]; then
        # Gen 1's other rule set: plain math (`!Cartridge Trunc`).
        echo "── protocol-diff harness: ps-sim side gen 1 n=200 jobs=14 trunc=math ──"
        scripts/ps-diff.sh --ps-cli "$PS_CLI_BIN" --gen 1 --n 200 --teams wide --quiet --jobs 14 --trunc math
      fi
    else
      echo "── protocol-diff harness: skipped gen $gen ps-sim (empty --ported list) ──"
    fi
  done

  # Pinned seeds against the rust side. A battle may stop at an effect
  # that has no port yet (`--allow-unported`); what it wrote up to there
  # must still be PS's, line for line.
  scripts/ps-diff.sh --pinned --ps-cli "$PS_CLI_BIN" --allow-unported --quiet

  # Gen 1's targeted battles (crates/ps_harness/targeted/gen1.json): fixed
  # teams and choices that reach the paths random teams never do, in both
  # rule sets. A battle that no longer reaches its path fails like a diff.
  echo "── protocol-diff harness: gen 1 targeted battles (cartridge, math) ──"
  scripts/ps-diff.sh --targeted gen1 --ps-cli "$PS_CLI_BIN" --quiet --jobs 14
  scripts/ps-diff.sh --targeted gen1 --ps-cli "$PS_CLI_BIN" --quiet --jobs 14 --trunc math
  # Gen 3's (crates/ps_harness/targeted/gen3.json), the same way.
  echo "── protocol-diff harness: gen 3 targeted battles ──"
  scripts/ps-diff.sh --targeted gen3 --ps-cli "$PS_CLI_BIN" --quiet --jobs 14

  echo "── wasm build + headless seat probe ──"
  (cd crates/mega_blastoise_web && wasm-pack build --target web --dev >/dev/null 2>&1)
  # The default engine (PS for both generations).
  scripts/ui-probe.sh --skip-build
  # The legacy Gen 3 engine, until G3-9 deletes it.
  scripts/ui-probe.sh --skip-build --engine legacy
fi

echo "check: OK"
