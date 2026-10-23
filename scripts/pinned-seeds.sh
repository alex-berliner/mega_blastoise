#!/usr/bin/env bash
# The pinned regression corpus for the OLD engines (Gen 1 and Gen 3).
#
#   scripts/pinned-seeds.sh [file]
#
# Every entry is a seed that found a bug, kept as a test that must stay
# green. One entry per line:
#
#   <suite> <wide 0|1|2> <seed> [case=<k>] [only=<ids>]  # <fix commit> <note>
#
#   suite  the showdown_fuzz test to run (fuzz_gen1_battles, ...)
#   wide   0, 1 or 2: the FUZZ_WIDE level the seed was found at
#   seed   the FUZZ_SEED the seed was found with
#   case   optional: compare only this case of the seed (FUZZ_CASE)
#   only   optional: narrow the move pool to these ids (FUZZ_ONLY)
#
# Everything after the `#` says where the finding came from; it is a note,
# not part of the run.
#
# Env:
#   JOBS          entries to run at once (default: nproc)
#   FUZZ_N        cases per suite (default: 200, the volume the seeds were
#                 found at; a bigger number names more battles per seed)
#   FUZZ_CACHE_DIR  where the reference's answers are cached
#   PINNED_LOG_DIR   where the logs go (default: .scratch/pinned)
#
# Skips loudly (exit 0) when the reference harness is not installed.
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

FILE=${1:-crates/mega_blastoise_test/pinned_seeds.txt}
LOG_DIR=${PINNED_LOG_DIR:-.scratch/pinned}
JOBS=${JOBS:-$(nproc 2>/dev/null || echo 4)}
CASES=${FUZZ_N:-200}

if [[ ! -d tools/showdown-parity/node_modules ]]; then
  echo "SKIP: the pinned seeds need the reference harness:"
  echo "      tools/showdown-parity/node_modules is missing (npm install there)."
  exit 0
fi
if [[ ! -f "$FILE" ]]; then
  echo "pinned seeds: no corpus at $FILE"
  exit 1
fi

mkdir -p "$LOG_DIR"
STATUS_FILE=$LOG_DIR/status.txt
JOBS_FILE=$LOG_DIR/jobs.txt
BUILD_LOG=$LOG_DIR/build.log
: > "$STATUS_FILE"

# ── the corpus ────────────────────────────────────────────────────────────────
# The note after the `#` is dropped; the run needs the suite, the width and
# the seed, and takes the optional case= and only= fields.
: > "$JOBS_FILE"
while IFS= read -r line || [[ -n "$line" ]]; do
  line=${line%%#*}
  [[ -z "${line//[[:space:]]/}" ]] && continue
  read -r -a fields <<<"$line"
  if (( ${#fields[@]} < 3 )); then
    echo "pinned seeds: bad entry: $line"
    exit 1
  fi
  suite=${fields[0]}
  wide=${fields[1]}
  seed=${fields[2]}
  case_k="-"
  only="-"
  if (( ${#fields[@]} > 3 )); then
    for opt in "${fields[@]:3}"; do
      case "$opt" in
        case=*) case_k=${opt#case=} ;;
        only=*) only=${opt#only=} ;;
        *)
          echo "pinned seeds: unknown option '$opt' in: $line"
          exit 1
          ;;
      esac
    done
  fi
  printf '%s %s %s %s %s\n' "$suite" "$wide" "$seed" "$case_k" "$only" >> "$JOBS_FILE"
done < "$FILE"

total=$(wc -l < "$JOBS_FILE")
if (( total == 0 )); then
  echo "pinned seeds: the corpus at $FILE is empty"
  exit 1
fi

# ── the binary, built once ───────────────────────────────────────────────────
echo "── pinned seeds: building the fuzz binary ──"
if ! cargo test -p mega-blastoise-test --test showdown_fuzz --no-run > "$BUILD_LOG" 2>&1; then
  echo "pinned seeds: the build failed:"
  tail -20 "$BUILD_LOG"
  exit 1
fi
BIN=$(sed -n 's/.*Executable .*(\(.*\)).*/\1/p' "$BUILD_LOG" | tail -1)
if [[ -z "$BIN" || ! -x "$BIN" ]]; then
  echo "pinned seeds: no showdown_fuzz binary came out of the build"
  tail -20 "$BUILD_LOG"
  exit 1
fi
# A private copy, so a rebuild while the corpus runs cannot swap it under
# the entries that are still to come.
cp "$BIN" "$LOG_DIR/showdown_fuzz"
BIN=$LOG_DIR/showdown_fuzz

# ── the entries ──────────────────────────────────────────────────────────────
# The log's name: the entry, spelled out.
entry_name() {
  local suite=$1 wide=$2 seed=$3 case_k=$4 only=$5
  local name="${suite}_w${wide}_s${seed}"
  [[ "$case_k" != "-" ]] && name="${name}_c${case_k}"
  [[ "$only" != "-" ]] && name="${name}_only"
  echo "$name"
}

run_entry() {
  local suite=$1 wide=$2 seed=$3 case_k=$4 only=$5
  local log="$LOG_DIR/$(entry_name "$suite" "$wide" "$seed" "$case_k" "$only").log"
  local -a envs=("FUZZ_SEED=$seed" "FUZZ_WIDE=$wide" "FUZZ_N=$CASES")
  [[ "$case_k" != "-" ]] && envs+=("FUZZ_CASE=$case_k")
  [[ "$only" != "-" ]] && envs+=("FUZZ_ONLY=$only")
  env -u FUZZ_SEED -u FUZZ_WIDE -u FUZZ_N -u FUZZ_CASE -u FUZZ_ONLY "${envs[@]}" \
    timeout 1800 "$BIN" --exact "$suite" --nocapture > "$log" 2>&1
  local rc=$?
  printf '%s %s %s %s %s %s\n' "$suite" "$wide" "$seed" "$case_k" "$only" "$rc" >> "$STATUS_FILE"
  return 0
}
export -f run_entry entry_name
export BIN LOG_DIR STATUS_FILE

echo "── pinned seeds: $total entries, $JOBS at a time ──"
xargs -P "$JOBS" -n 5 bash -c 'run_entry "$0" "$1" "$2" "$3" "$4"' < "$JOBS_FILE"

# ── the report ───────────────────────────────────────────────────────────────
failed=0
ran=$(wc -l < "$STATUS_FILE")
if (( ran != total )); then
  echo "pinned seeds: only $ran of $total entries ran"
  failed=$((failed + total - ran))
fi
while read -r suite wide seed case_k only rc; do
  if [[ "$rc" != "0" ]]; then
    failed=$((failed + 1))
    echo "FAIL  $suite wide=$wide seed=$seed case=$case_k only=$only (exit $rc)"
    tail -5 "$LOG_DIR/$(entry_name "$suite" "$wide" "$seed" "$case_k" "$only").log" 2>/dev/null \
      | sed 's/^/      /'
  fi
done < "$STATUS_FILE"

if (( failed > 0 )); then
  echo "pinned seeds: $failed of $total FAILED (logs in $LOG_DIR)"
  exit 1
fi
echo "pinned seeds: $total green (logs in $LOG_DIR)"
