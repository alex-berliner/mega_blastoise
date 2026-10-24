#!/usr/bin/env bash
# The old engines' pinned corpus, carried over to the protocol-diff harness.
#
#   scripts/carried-dump.sh [gen] [--check]
#
# Every `fuzz_gen<gen>_battles` entry of
# crates/mega_blastoise_test/pinned_seeds.txt names the battles of one fuzz
# seed (or one case of it). This dumps each entry's team pairs, through the
# suite's own `FUZZ_DUMP`, to
#
#   crates/ps_harness/carried/gen<gen>/<suite>_w<wide>_s<seed>[_c<case>][_only].json
#
# (the log names of scripts/pinned-seeds.sh), which
# `scripts/ps-diff.sh --carried gen<gen>` replays against ps_sim, every pair
# over 20 battle seeds. The dump is deterministic: the same corpus and the
# same generator write the same files.
#
# `--check` dumps to .scratch/carried/ instead and fails if any file differs
# from the committed corpus (a missing, stale or extra file).
#
# Env:
#   FUZZ_N   cases per suite (default: 200, the volume the seeds were found at)
#
# The dump needs no reference: it runs without tools/showdown-parity.
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

GEN=1
CHECK=0
for a in "$@"; do
  case "$a" in
    --check) CHECK=1 ;;
    1|gen1) GEN=1 ;;
    *)
      echo "carried dump: usage: scripts/carried-dump.sh [1|gen1] [--check]"
      echo "              (only the gen 1 battle suite dumps its teams so far)"
      exit 2
      ;;
  esac
done

SUITE=fuzz_gen${GEN}_battles
FILE=crates/mega_blastoise_test/pinned_seeds.txt
CORPUS=crates/ps_harness/carried/gen$GEN
SCRATCH=.scratch/carried
CASES=${FUZZ_N:-200}
OUT=$CORPUS
(( CHECK )) && OUT=$SCRATCH/gen$GEN

mkdir -p "$SCRATCH"
BUILD_LOG=$SCRATCH/build.log
echo "── carried dump: building the fuzz binary ──"
if ! cargo test -p mega-blastoise-test --test showdown_fuzz --no-run < /dev/null > "$BUILD_LOG" 2>&1; then
  echo "carried dump: the build failed:"
  tail -20 "$BUILD_LOG"
  exit 1
fi
BIN=$(sed -n 's/.*Executable .*(\(.*\)).*/\1/p' "$BUILD_LOG" | tail -1)
if [[ -z "$BIN" || ! -x "$BIN" ]]; then
  echo "carried dump: no showdown_fuzz binary came out of the build"
  exit 1
fi

rm -rf "$OUT"
mkdir -p "$OUT"
n=0
failed=0
while IFS= read -r line || [[ -n "$line" ]]; do
  line=${line%%#*}
  [[ -z "${line//[[:space:]]/}" ]] && continue
  read -r -a fields <<<"$line"
  [[ "${fields[0]}" != "$SUITE" ]] && continue
  wide=${fields[1]}
  seed=${fields[2]}
  case_k=-
  only=-
  for opt in "${fields[@]:3}"; do
    case "$opt" in
      case=*) case_k=${opt#case=} ;;
      only=*) only=${opt#only=} ;;
      *) echo "carried dump: unknown option '$opt' in: $line"; exit 1 ;;
    esac
  done
  name="${SUITE}_w${wide}_s${seed}"
  [[ "$case_k" != "-" ]] && name="${name}_c${case_k}"
  [[ "$only" != "-" ]] && name="${name}_only"
  envs=("FUZZ_SEED=$seed" "FUZZ_WIDE=$wide" "FUZZ_N=$CASES" "FUZZ_DUMP=$PWD/$OUT/$name.json")
  [[ "$case_k" != "-" ]] && envs+=("FUZZ_CASE=$case_k")
  [[ "$only" != "-" ]] && envs+=("FUZZ_ONLY=$only")
  log=$SCRATCH/$name.log
  env -u FUZZ_SEED -u FUZZ_WIDE -u FUZZ_N -u FUZZ_CASE -u FUZZ_ONLY -u FUZZ_DUMP "${envs[@]}" \
    "$BIN" --exact "$SUITE" --nocapture < /dev/null > "$log" 2>&1
  rc=$?
  if (( rc != 0 )) || [[ ! -s "$OUT/$name.json" ]]; then
    echo "FAIL  $name (exit $rc, log $log)"
    tail -5 "$log" | sed 's/^/      /'
    failed=$((failed + 1))
  fi
  n=$((n + 1))
done < "$FILE"

if (( n == 0 )); then
  echo "carried dump: no $SUITE entries in $FILE"
  exit 1
fi
if (( failed > 0 )); then
  echo "carried dump: $failed of $n entries FAILED"
  exit 1
fi
if (( CHECK )); then
  if ! diff -r "$CORPUS" "$OUT" > "$SCRATCH/check.diff"; then
    echo "carried dump: the committed corpus in $CORPUS is not what the generator dumps:"
    head -20 "$SCRATCH/check.diff"
    exit 1
  fi
  echo "carried dump: $n entries, identical to $CORPUS"
  exit 0
fi
echo "carried dump: $n entries written to $OUT"
