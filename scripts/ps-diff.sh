#!/usr/bin/env bash
# Showdown split protocol-diff driver.
#
#   scripts/ps-diff.sh --self --gen 1 --n 200          # 200 PS-vs-PS battles
#   scripts/ps-diff.sh --self --gen 3 --n 200          # gen 3 random
#   scripts/ps-diff.sh --self --gen 1 --teams wide --n 50
#   scripts/ps-diff.sh --pinned                        # run crates/ps_harness/pinned.txt
#   scripts/ps-diff.sh --self --seed 0 --gen 1         # one specific seed
#   scripts/ps-diff.sh --self --gen 1 --teams wide --trunc math   # Gen 1 plain math
#   scripts/ps-diff.sh --carried gen1 --ps-cli "$PWD/target/release/ps_sim_cli" \
#     --allow-unported --jobs 6                        # the carried old corpus
#   scripts/ps-diff.sh --carried gen3 --ps-cli "$PWD/target/release/ps_sim_cli" --jobs 6
#   scripts/ps-diff.sh --targeted gen3 --ps-cli "$PWD/target/release/ps_sim_cli"
#
# Thin wrapper around tools/ps-diff/diff.js. The actual BattleStream
# plumbing, request parsing, line-by-line diff and cargo path are all
# there; this script just makes the command runnable from anywhere and
# fixes the project-root working dir.
#
# Exit codes:
#   0  every battle passed (parity, no crash, no unported)
#   1  at least one diff / crash / unported hit; see .scratch/ps-diff/
#   2  usage error (gen missing, unknown --teams, etc.)
set -euo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

. /root/.cargo/env 2>/dev/null || true
export NVM_DIR=/root/.nvm
# shellcheck disable=SC1091
. "$NVM_DIR/nvm.sh" 2>/dev/null || true

node tools/ps-diff/diff.js "$@"
