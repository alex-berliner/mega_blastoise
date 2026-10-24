#!/usr/bin/env bash
# The coverage gate (V-1, docs/architecture/showdown-split-execution.md):
# plays a protocol-diff sweep against an instrumented ps_sim_cli and prints
#
#   1. per-handler hit counts: every handler the gen's registry holds, the
#      number of battles it ran in and its calls (`ps_sim` feature
#      `hit-count`, `ps_sim_cli --hits`), summed over the sweep;
#   2. line coverage (`cargo llvm-cov`) of `crates/ps_sim/src/mods/genN/**`
#      and the core files (`crates/ps_sim/src/*.rs`), with the lines never
#      run.
#
# The gate passes when every registered handler ran in at least
# --min-battles battles and every line of mods/genN ran or is listed in
# crates/ps_harness/coverage-allow.txt. Under-hit handlers print as a
# `--focus` id list for a targeted sweep (`--teams wide --focus <ids>`).
#
#   scripts/ps-coverage.sh                       # gen 1, the gate sweep:
#                                                # 10,000 random + 10,000 wide
#                                                # from seed 1000000
#   scripts/ps-coverage.sh --gen 3               # gen 3 (information)
#   scripts/ps-coverage.sh --n 500 --jobs 8      # a quick look
#   scripts/ps-coverage.sh --teams wide --focus bide,rage --n 2000
#   scripts/ps-coverage.sh --no-lines            # hit counts only (no llvm-cov)
#   scripts/ps-coverage.sh --report-only         # re-print the tables of the
#                                                # last run (after editing
#                                                # coverage-allow.txt)
#
# Coverage runs at phase gates, not in scripts/check.sh. The builds go to
# their own target dirs (target/llvm-cov, target/hit-count), so the normal
# debug and release binaries are untouched. Reports land in
# .scratch/coverage/gen<N>/ (hits.tsv, lines.txt, lcov.info).
#
# Needs cargo-llvm-cov (`cargo install cargo-llvm-cov --locked`,
# `rustup component add llvm-tools-preview`) unless --no-lines.
#
# Exit codes: 0 gate passes, 1 gate fails (or the sweep had diffs or
# crashes), 2 usage or build error.
set -euo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$PWD"

. /root/.cargo/env 2>/dev/null || true
export NVM_DIR=/root/.nvm
# shellcheck disable=SC1091
. "$NVM_DIR/nvm.sh" 2>/dev/null || true

GEN=1
N=10000
SEED_START=1000000
JOBS=8
TEAMS="random wide"
FOCUS=""
MIN_BATTLES=20
LINES=1
OUT=""
REPORT_ONLY=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --gen) GEN="$2"; shift 2 ;;
    --n) N="$2"; shift 2 ;;
    --seed-start) SEED_START="$2"; shift 2 ;;
    --jobs) JOBS="$2"; shift 2 ;;
    --teams) TEAMS="$2"; shift 2 ;;
    --focus) FOCUS="$2"; shift 2 ;;
    --min-battles) MIN_BATTLES="$2"; shift 2 ;;
    --no-lines) LINES=0; shift ;;
    --report-only) REPORT_ONLY=1; shift ;;
    --out) OUT="$2"; shift 2 ;;
    -h|--help) awk 'NR > 1 && !/^#/ { exit } NR > 1 { sub(/^# ?/, ""); print }' "$0"; exit 0 ;;
    *) echo "ps-coverage: unknown option $1" >&2; exit 2 ;;
  esac
done
case "$GEN" in 1|3) ;; *) echo "ps-coverage: --gen 1 or 3" >&2; exit 2 ;; esac
OUT="${OUT:-$ROOT/.scratch/coverage/gen$GEN}"
mkdir -p "$OUT"
HITS_DIR="$OUT/hits"
FAILED=0
SWEEP=""
if [[ "$REPORT_ONLY" == 1 ]]; then
  BIN="$ROOT/target/llvm-cov/release/ps_sim_cli"
  [[ -f "$OUT/lcov.info" ]] || { LINES=0; BIN="$ROOT/target/hit-count/release/ps_sim_cli"; }
  [[ -x "$BIN" && -d "$HITS_DIR" ]] || { echo "ps-coverage: no earlier run in ${OUT#"$ROOT"/}" >&2; exit 2; }
  SWEEP="$(cat "$OUT/sweep.txt" 2>/dev/null || true)"
else
rm -rf "$HITS_DIR"
mkdir -p "$HITS_DIR"

# ---- build -------------------------------------------------------------------
if [[ "$LINES" == 1 ]]; then
  if ! cargo llvm-cov --version >/dev/null 2>&1; then
    echo "ps-coverage: cargo-llvm-cov is not installed (or pass --no-lines)" >&2
    exit 2
  fi
  export CARGO_TARGET_DIR="$ROOT/target/llvm-cov"
  # shellcheck disable=SC1090
  source <(cargo llvm-cov show-env --sh 2>/dev/null)
  # Drops the old .profraw files (and the workspace's instrumented builds).
  cargo llvm-cov clean --workspace </dev/null >/dev/null 2>&1
  echo "── build: instrumented ps_sim_cli (hit-count, llvm-cov) ──"
  cargo build --quiet --release -p ps_harness --bin ps_sim_cli --features hit-count </dev/null
  BIN="$CARGO_TARGET_DIR/release/ps_sim_cli"
else
  echo "── build: ps_sim_cli (hit-count) ──"
  cargo build --quiet --release -p ps_harness --bin ps_sim_cli --features hit-count \
    --target-dir "$ROOT/target/hit-count" </dev/null
  BIN="$ROOT/target/hit-count/release/ps_sim_cli"
fi

# ---- sweep -------------------------------------------------------------------
for teams in $TEAMS; do
  args=(--ps-cli "$BIN" --gen "$GEN" --n "$N" --seed-start "$SEED_START" --teams "$teams"
        --jobs "$JOBS" --quiet --allow-unported --hits "$HITS_DIR/$teams-{pid}.tsv")
  [[ -n "$FOCUS" ]] && args+=(--focus "$FOCUS")
  echo "── sweep: gen $GEN teams=$teams n=$N seeds $SEED_START.. ${FOCUS:+focus=$FOCUS} ──"
  set +e
  node tools/ps-diff/diff.js "${args[@]}" </dev/null | tee "$OUT/sweep-$teams.txt"
  status=${PIPESTATUS[0]}
  set -e
  [[ "$status" != 0 ]] && FAILED=1
  SWEEP+="$teams=$N "
done

# Each ps_sim_cli writes its hit file and .profraw when its stdin closes,
# which can be just after diff.js exits: wait for them (read-only check).
for _ in $(seq 1 120); do
  pgrep -f -- "$BIN" >/dev/null || break
  sleep 0.5
done

if [[ "$LINES" == 1 ]]; then
  cargo llvm-cov report --release --lcov --output-path "$OUT/lcov.info" </dev/null >/dev/null 2>&1
else
  rm -f "$OUT/lcov.info"
fi
echo "$SWEEP" > "$OUT/sweep.txt"
fi

# ---- tables ------------------------------------------------------------------
# (An instrumented binary writes a .profraw at exit: not this one.)
LLVM_PROFILE_FILE=/dev/null "$BIN" --handlers "$GEN" > "$OUT/handlers.tsv"
set +e
python3 - "$GEN" "$MIN_BATTLES" "$OUT" "$HITS_DIR" "$LINES" "$ROOT" "$SWEEP" <<'PY'
import os, re, sys
from collections import defaultdict

gen, min_battles, out, hits_dir, lines_on, root, sweep = sys.argv[1:]
gen = int(gen); min_battles = int(min_battles); lines_on = lines_on == '1'
fail = False

# ---- coverage-allow.txt ----
# `<path under crates/ps_sim/src>:<first>[-<last>]  <reason>` or
# `handler <gen> <kind:id> <key>  <reason>`
allow = defaultdict(list)  # path -> [[first, last, reason, used]]
allow_handlers = {}        # (effect, key) -> [reason, used]
allow_file = os.path.join(root, 'crates/ps_harness/coverage-allow.txt')
for raw in open(allow_file):
    s = raw.strip()
    if not s or s.startswith('#'):
        continue
    m = re.match(r'handler\s+(\d+)\s+(\S+)\s+(\S+)\s+(.*)', s)
    if m:
        if int(m.group(1)) == gen:
            allow_handlers[(m.group(2), m.group(3))] = [m.group(4), False]
        continue
    m = re.match(r'(\S+):(\d+)(?:-(\d+))?\s+(.*)', s)
    if not m:
        print(f'coverage-allow.txt: bad line: {raw.rstrip()}')
        fail = True
        continue
    a, b = int(m.group(2)), int(m.group(3) or m.group(2))
    allow[m.group(1)].append([a, b, m.group(4), False])

# ---- 1. handler hits ----
battles = 0
hits = defaultdict(lambda: [0, 0])  # (effect, key) -> [calls, battles]
for name in sorted(os.listdir(hits_dir)):
    if not name.endswith('.tsv'):
        continue
    for line in open(os.path.join(hits_dir, name)):
        f = line.rstrip('\n').split('\t')
        if f[0] == 'battles' and int(f[1]) == gen:
            battles += int(f[2])
        elif f[0] == 'hit' and int(f[1]) == gen:
            h = hits[(f[2], f[3])]
            h[0] += int(f[4]); h[1] += int(f[5])

handlers = []  # (effect, key, focus)
for line in open(os.path.join(out, 'handlers.tsv')):
    effect, key, focus = line.rstrip('\n').split('\t')
    handlers.append((effect, key, [] if focus == '-' else focus.split(',')))
registered = {(e, k) for e, k, _ in handlers}

rows = []
for effect, key, focus in handlers:
    calls, nb = hits.get((effect, key), (0, 0))
    rows.append((nb, calls, effect, key, focus))
rows.sort(key=lambda r: (r[0], r[2], r[3]))
under = []
for r in rows:
    if r[0] >= min_battles:
        continue
    a = allow_handlers.get((r[2], r[3]))
    if a:
        a[1] = True
    else:
        under.append(r)

w = max([len(f'{r[2]} {r[3]}') for r in rows] + [20])
table = [f'handler hits, gen {gen}: {battles} battles ({sweep.strip()}), {len(rows)} registered handlers, gate {min_battles} battles',
         f'{"battles":>8} {"calls":>9}  {"handler":<{w}}  status']
for nb, calls, effect, key, focus in rows:
    status = 'ok' if nb >= min_battles else 'allowed' if (effect, key) in allow_handlers else 'UNDER'
    table.append(f'{nb:>8} {calls:>9}  {effect + " " + key:<{w}}  {status}')
stray = sorted(set(hits) - registered)
for e, k in stray:
    table.append(f'{hits[(e, k)][1]:>8} {hits[(e, k)][0]:>9}  {e + " " + k:<{w}}  (not in the registry)')
focus_ids = sorted({m for r in under for m in r[4]})
no_focus = [f'{r[2]} {r[3]}' for r in under if not r[4]]
n_allowed = sum(1 for r in rows if r[0] < min_battles) - len(under)
table.append(f'under-hit: {len(under)} of {len(rows)} handlers ran in fewer than {min_battles} battles'
             f' (and {n_allowed} more are allowed by coverage-allow.txt)')
if under:
    fail = True
    table.append('--focus ' + (','.join(focus_ids) if focus_ids else '(none)'))
    if no_focus:
        table.append('no focus move reaches: ' + ', '.join(no_focus))
stale_h = [(e, k) for (e, k), a in allow_handlers.items() if not a[1]]
if stale_h:
    table.append('coverage-allow.txt handler entries that pass the gate or name no handler (stale): '
                 + ', '.join(f'{e} {k}' for e, k in stale_h))
text = '\n'.join(table)
print('\n== 1. handler hits ==')
print(text)
open(os.path.join(out, 'hits.txt'), 'w').write(text + '\n')

# ---- 2. line coverage ----
if not lines_on:
    print('\n== 2. line coverage: skipped (--no-lines) ==')
    sys.exit(1 if fail else 0)

src = os.path.join(root, 'crates/ps_sim/src') + '/'
cov = {}  # rel path -> {line: count}
cur = None
for line in open(os.path.join(out, 'lcov.info')):
    line = line.rstrip('\n')
    if line.startswith('SF:'):
        p = line[3:]
        cur = cov.setdefault(p[len(src):], {}) if p.startswith(src) else None
    elif line.startswith('DA:') and cur is not None:
        ln, n = line[3:].split(',')[:2]
        cur[int(ln)] = cur.get(int(ln), 0) + int(n)

def ranges(lines):
    out, start, prev = [], None, None
    for l in lines:
        if start is None:
            start = prev = l
        elif l == prev + 1:
            prev = l
        else:
            out.append((start, prev)); start = prev = l
    if start is not None:
        out.append((start, prev))
    return out

def fmt(rs):
    return ','.join(f'{a}' if a == b else f'{a}-{b}' for a, b in rs)

mods = f'mods/gen{gen}/'
report = []
gate_rows, core_rows = [], []
unallowed_total = 0
for path in sorted(cov):
    data = cov[path]
    unrun = sorted(l for l, n in data.items() if n == 0)
    total, ran = len(data), len(data) - len(unrun)
    if path.startswith(mods):
        bad = []
        for l in unrun:
            ok = False
            for e in allow.get(path, []):
                if e[0] <= l <= e[1]:
                    e[3] = True; ok = True
            if not ok:
                bad.append(l)
        unallowed_total += len(bad)
        gate_rows.append((path, total, ran, unrun, bad))
    elif '/' not in path:
        core_rows.append((path, total, ran, unrun))

def pct(ran, total):
    return f'{100.0 * ran / total:5.1f}%' if total else '  -  '

report.append(f'line coverage, gen {gen}: crates/ps_sim/src/{mods}** (gate) and the core files, {battles} battles')
report.append(f'{"lines":>6} {"run":>6} {"unrun":>6} {"cover":>6} {"not allowed":>11}  file')
t_total = t_ran = 0
for path, total, ran, unrun, bad in gate_rows:
    t_total += total; t_ran += ran
    report.append(f'{total:>6} {ran:>6} {len(unrun):>6} {pct(ran, total)} {len(bad):>11}  {path}')
report.append(f'{t_total:>6} {t_ran:>6} {t_total - t_ran:>6} {pct(t_ran, t_total)} {unallowed_total:>11}  (mods/gen{gen} total)')
report.append('')
report.append(f'unrun lines in mods/gen{gen} not in coverage-allow.txt:')
for path, total, ran, unrun, bad in gate_rows:
    if bad:
        report.append(f'  {path}: {fmt(ranges(bad))}')
if unallowed_total == 0:
    report.append('  (none)')
else:
    fail = True
stale = [(p, e) for p, es in allow.items() if p.startswith(mods) for e in es if not e[3]]
if stale:
    report.append('coverage-allow.txt entries that match no unrun line (stale or now run):')
    for p, e in stale:
        report.append(f'  {p}:{e[0]}-{e[1]}  {e[2]}')
report.append('')
report.append('core files (information, not gated):')
report.append(f'{"lines":>6} {"run":>6} {"unrun":>6} {"cover":>6}  file: unrun lines')
for path, total, ran, unrun in core_rows:
    report.append(f'{total:>6} {ran:>6} {len(unrun):>6} {pct(ran, total)}  {path}: {fmt(ranges(unrun))}')
text = '\n'.join(report)
print('\n== 2. line coverage ==')
print(text)
open(os.path.join(out, 'lines.txt'), 'w').write(text + '\n')
sys.exit(1 if fail else 0)
PY
GATE=$?
set -e
echo
if [[ "$FAILED" != 0 ]]; then
  echo "ps-coverage: the sweep had diffs or crashes (see above); FAIL"
  exit 1
fi
if [[ "$GATE" == 0 ]]; then
  echo "ps-coverage: gen $GEN gate PASS (reports in ${OUT#"$ROOT"/})"
else
  echo "ps-coverage: gen $GEN gate FAIL (reports in ${OUT#"$ROOT"/})"
fi
exit "$GATE"
