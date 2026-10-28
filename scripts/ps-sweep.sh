#!/usr/bin/env bash
# The continuous parity sweep (V-2, docs/living/showdown-split-execution.md).
#
# A loop for tmux on the worker machine. It builds a private copy of
# `ps_sim_cli` from the tip of the integration worktree (rebuilding when
# the tip moves), then plays protocol-diff batches of fresh seeds against
# PS, four lanes per round, and files every failure into a bucket named
# after the first differing line.
#
# The state lives outside every worktree, in --state-dir (default
# /root/pv/sweep): the seed counter, the one-line status, the bucket
# ledger and the failure buckets themselves. The repo it builds from is
# only ever read; the build happens in a private source copy.
#
# Usage:
#   scripts/ps-sweep.sh                    # the loop: rounds until killed
#   scripts/ps-sweep.sh --once             # one round (four lanes), then stop
#   scripts/ps-sweep.sh --rounds 3         # three rounds, then stop
#   scripts/ps-sweep.sh status             # print the status line
#   scripts/ps-sweep.sh status --ack       # print it, advance the read cursor
#   scripts/ps-sweep.sh build              # rebuild the private binary only
#
# Options:
#   --repo PATH          the repo to build from (default /root/wt/ps-split)
#   --state-dir PATH     the sweep's state (default /root/pv/sweep)
#   --n N                battles per lane per round (default 2000)
#   --jobs N             diff.js --jobs (default 8)
#   --nice N             niceness of the batches (default 19)
#   --seed-start N       the counter's first seed (default 1000000)
#   --lanes LIST         the lanes to run (default: all four)
#   --tamper-line K      plant a bug: diff.js --tamper-line K
#   --max-per-bucket N   logs kept per bucket (default 5)
#   --max-disk-mb N      the cap on the failure buckets (default 512)
#   --sleep S            seconds between rounds (default 0)
#   --build-jobs N       cargo jobs for the private build (default 4)
#
# Exit codes: 0 the loop finished (or the status was printed); 2 a usage
# or setup error (including another sweep holding the state dir's lock).
set -euo pipefail

. /root/.cargo/env 2>/dev/null || true
export NVM_DIR=/root/.nvm
# shellcheck disable=SC1091
. "$NVM_DIR/nvm.sh" 2>/dev/null || true

REPO=/root/wt/ps-split
STATE=/root/pv/sweep
N=2000
JOBS=8
NICE=19
SEED_START=1000000
LANES="gen1-wide gen3-wide gen1-random gen3-random"
TAMPER=""
MAX_PER_BUCKET=5
MAX_DISK_MB=512
SLEEP=0
BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
ROUNDS=0
ONCE=0
ACTION=loop
ACK=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo) REPO="$2"; shift 2 ;;
    --state-dir) STATE="$2"; shift 2 ;;
    --n) N="$2"; shift 2 ;;
    --jobs) JOBS="$2"; shift 2 ;;
    --nice) NICE="$2"; shift 2 ;;
    --seed-start) SEED_START="$2"; shift 2 ;;
    --lanes) LANES="$2"; shift 2 ;;
    --tamper-line) TAMPER="$2"; shift 2 ;;
    --max-per-bucket) MAX_PER_BUCKET="$2"; shift 2 ;;
    --max-disk-mb) MAX_DISK_MB="$2"; shift 2 ;;
    --sleep) SLEEP="$2"; shift 2 ;;
    --build-jobs) BUILD_JOBS="$2"; shift 2 ;;
    --rounds) ROUNDS="$2"; shift 2 ;;
    --once) ONCE=1; shift ;;
    status) ACTION=status; shift ;;
    --ack) ACK=1; shift ;;
    build) ACTION=build; shift ;;
    -h|--help) awk 'NR > 1 && !/^#/ { exit } NR > 1 { sub(/^# ?/, ""); print }' "$0"; exit 0 ;;
    *) echo "ps-sweep: unknown option $1" >&2; exit 2 ;;
  esac
done

BIN="$STATE/bin/ps_sim_cli"
SRC="$STATE/src"
TARGET="$STATE/target"
DIFF_JS="$REPO/tools/ps-diff/diff.js"
CURRENT_PID=""
LEAKED=0

mkdir -p "$STATE/bin" "$STATE/src" "$STATE/target" "$STATE/scratch"

# ---------------------------------------------------------------- python ----
# One helper, three subcommands: `collect` files the failures of one lane
# into buckets, `status` renders the one-line status, `prune` caps the
# disk. The helper is pure text and never touches the repo.
py() {
  python3 - "$@" <<'PYEOF'
import os, re, shutil, sys, time

def now_iso():
    return time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())

def slug(text, limit=44):
    text = (text or '').strip().lower()
    text = re.sub(r'[0-9]+', 'n', text)
    text = re.sub(r'[^a-z0-9]+', '-', text)
    text = re.sub(r'-{2,}', '-', text).strip('-')
    return text[:limit].strip('-') or 'empty'

def kind_of(line):
    """The PS line's kind (`|move|`, `-damage`, ...) and its rest."""
    s = (line or '').strip()
    if not s:
        return 'empty', ''
    if s.startswith('|'):
        parts = s.split('|')
        kind = parts[1] if len(parts) > 1 and parts[1] else 'line'
        return kind, '|'.join(parts[2:])
    if s.startswith('-'):
        rest = s[1:]
        kind = rest.split(' ')[0].split('|')[0]
        return kind, rest[len(kind):]
    return 'line', s

def bucket_name(gen, teams, kind, rest):
    name = 'gen%s-%s-%s-%s' % (gen, teams, kind, slug(rest))
    return name[:96].rstrip('-')

def first_diff_line(text):
    """The first differing line of a divergence log (PS's line wins)."""
    m = re.search(r'^ >> \[\d+\] L: (.*?) \| R: (.*?) <<$', text, re.M)
    if not m:
        return None
    left, right = m.group(1).strip(), m.group(2).strip()
    if '|tampered|mark|' in right:
        right = right.split('|tampered|mark|')[0].strip()
    if left and left != '|':
        return left
    return right or left

def crash_line(text):
    """The first meaningful line of a crash log."""
    lines = [l for l in text.split('\n')[1:] if l.strip()]
    if not lines:
        return 'unknown crash'
    line = lines[0].strip()
    # A JS stack: the message is the first line, the frames follow.
    line = re.sub(r'^\s*at\s+.*$', '', line)
    return line

def read_kv_tsv(path):
    out = {}
    if not os.path.exists(path):
        return out
    for raw in open(path, errors='replace'):
        f = raw.rstrip('\n').split('\t')
        if len(f) >= 2:
            out[f[0]] = f[1:]
    return out

def read_count_tsv(path):
    """`<lane>\\t<id>\\t<count>` rows, keyed by the whole prefix."""
    out = {}
    if not os.path.exists(path):
        return out
    for raw in open(path, errors='replace'):
        f = raw.rstrip('\n').split('\t')
        if len(f) >= 3:
            out['\t'.join(f[:-1])] = f[-1]
    return out

def write_atomic(path, text):
    tmp = path + '.tmp'
    with open(tmp, 'w') as f:
        f.write(text)
    os.replace(tmp, path)

def cmd_collect(argv):
    (scratch, state, date, lane, gen, teams, repro_tmpl, max_per_bucket, round_no,
     tip) = argv
    ledger_path = os.path.join(state, 'buckets.tsv')
    ledger = read_kv_tsv(ledger_path)
    diffs = crashes = ps_crashes = 0
    touched = []
    if not os.path.isdir(scratch):
        print('collect: diffs=0 crashes=0 ps_crashes=0 buckets=')
        return 0
    for name in sorted(os.listdir(scratch)):
        m = re.match(r'^ps-diff-(.+?)(-crash)?\.log$', name)
        if not m:
            continue
        path = os.path.join(scratch, name)
        text = open(path, errors='replace').read()
        seed = m.group(1)
        if name.endswith('-crash.log'):
            if 'not comparable' in text:
                # PS itself threw and ps_sim stopped too: a PS bug both
                # sides share, not a ps_sim failure (the harness counts it
                # as ps_crashes).
                ps_crashes += 1
                os.remove(path)
                continue
            kind, rest = 'crash', crash_line(text)
            crashes += 1
        else:
            line = first_diff_line(text)
            if line is None:
                kind, rest = 'unknown', 'no divergence line'
            else:
                kind, rest = kind_of(line)
                if not rest:
                    rest = line
            diffs += 1
        bucket = bucket_name(gen, teams, kind, rest)
        if bucket not in touched:
            touched.append(bucket)
        bdir = os.path.join(state, date, bucket)
        os.makedirs(bdir, exist_ok=True)
        shutil.move(path, os.path.join(bdir, name))
        repro = repro_tmpl.replace('{seed}', seed)
        with open(os.path.join(bdir, 'repro.txt'), 'a') as f:
            f.write('seed %s gen %s teams %s lane %s tip %s\n  %s\n'
                    % (seed, gen, teams, lane, tip, repro))
        # Keep the repro list short: the ledger holds the full count.
        rp = os.path.join(bdir, 'repro.txt')
        lines = open(rp, errors='replace').readlines()
        if len(lines) > 200:
            write_atomic(rp, ''.join(lines[-200:]))
        # Keep at most max_per_bucket logs per bucket.
        logs = sorted([f for f in os.listdir(bdir) if f.endswith('.log')],
                      key=lambda f: os.path.getmtime(os.path.join(bdir, f)))
        for old in logs[:max(0, len(logs) - int(max_per_bucket))]:
            os.remove(os.path.join(bdir, old))
        entry = ledger.get(bucket)
        if entry is None:
            entry = [ '0', str(round_no), now_iso(), '', '' ]
            ledger[bucket] = entry
        entry[0] = str(int(entry[0]) + 1)
        entry[3] = seed
        lanes = set(x for x in entry[4].split(',') if x)
        lanes.add(lane)
        entry[4] = ','.join(sorted(lanes))
    body = ''.join('\t'.join([b] + ledger[b]) + '\n' for b in sorted(ledger))
    write_atomic(ledger_path, body)
    print('collect: diffs=%d crashes=%d ps_crashes=%d buckets=%s'
          % (diffs, crashes, ps_crashes, ','.join(touched)))
    return 0

def cmd_status(argv):
    state = argv[0]
    ack = '--ack' in argv[1:]
    counters = read_kv_tsv(os.path.join(state, 'counters.tsv'))
    unported = read_count_tsv(os.path.join(state, 'unported.tsv'))
    ledger = read_kv_tsv(os.path.join(state, 'buckets.tsv'))
    def read_int(path, default=0):
        try:
            return int(open(path).read().strip())
        except Exception:
            return default
    round_no = read_int(os.path.join(state, 'round'))
    cursor = read_int(os.path.join(state, 'read_cursor'))
    tip = ''
    if os.path.exists(os.path.join(state, 'tip')):
        tip = open(os.path.join(state, 'tip')).read().strip()
    counter = read_int(os.path.join(state, 'counter'))
    leaked = read_int(os.path.join(state, 'leaked'))
    disk = read_int(os.path.join(state, 'disk_bytes'))
    totals = [0, 0, 0, 0, 0]
    lane_bits = []
    for lane in sorted(counters):
        vals = counters[lane]
        nums = [int(x) for x in (vals + ['0'] * 5)[:5]]
        for i, v in enumerate(nums):
            totals[i] += v
        lane_bits.append('%s:%d/%d/%d/%d' % (lane, nums[0], nums[1], nums[2], nums[3]))
    merged = {}
    for key, count in unported.items():
        ident = key.split('\t', 1)[1] if '\t' in key else key
        merged[ident] = merged.get(ident, 0) + int(count)
    ids = sorted(merged.items(), key=lambda kv: (-kv[1], kv[0]))
    id_text = ','.join('%s:%d' % (k, v) for k, v in ids[:40])
    if len(ids) > 40:
        id_text += ',+%d-more' % (len(ids) - 40)
    new = sorted(b for b, e in ledger.items() if int(e[1]) > cursor)
    line = ('ps-sweep: ts=%s round=%d tip=%s counter=%d battles=%d diffs=%d crashes=%d '
            'unported=%d unported_ids=%s lanes=%s buckets_total=%d buckets_new=%d '
            'buckets_new_list=%s ps_crashes=%d leaked=%d disk_mb=%d'
            % (now_iso(), round_no, tip or '-', counter, totals[0], totals[1], totals[2],
               totals[3], id_text, ','.join(lane_bits), len(ledger), len(new),
               ','.join(new), totals[4], leaked, disk // (1024 * 1024)))
    print(line)
    write_atomic(os.path.join(state, 'status'), line + '\n')
    if ack:
        write_atomic(os.path.join(state, 'read_cursor'), '%d\n' % round_no)
    return 0

def cmd_prune(argv):
    """Cap the disk the failure buckets use. Only the dated bucket dirs
    are pruned; the counter, the status, the ledger and the log stay."""
    state, max_disk_mb = argv[0], int(argv[1])
    limit = max_disk_mb * 1024 * 1024
    files = []
    total = 0
    for name in sorted(os.listdir(state)) if os.path.isdir(state) else []:
        if not re.match(r'^[0-9]{4}-[0-9]{2}-[0-9]{2}$', name):
            continue
        for root, dirs, names in os.walk(os.path.join(state, name)):
            for n in names:
                p = os.path.join(root, n)
                try:
                    size = os.path.getsize(p)
                except OSError:
                    continue
                files.append((os.path.getmtime(p), size, p))
                total += size
    # The whole state dir (the build included) is reported, so the
    # orchestrator can see the disk the sweep uses.
    all_bytes = 0
    for root, dirs, names in os.walk(state):
        for n in names:
            try:
                all_bytes += os.path.getsize(os.path.join(root, n))
            except OSError:
                pass
    write_atomic(os.path.join(state, 'disk_bytes'), '%d\n' % all_bytes)
    if total <= limit:
        print('prune: %d bytes used, cap %d' % (total, limit))
        return 0
    files.sort()
    for mtime, size, p in files:
        if total <= limit:
            break
        try:
            os.remove(p)
            total -= size
        except OSError:
            pass
    print('prune: %d bytes used after pruning, cap %d' % (total, limit))
    return 0

cmd = sys.argv[1]
if cmd == 'collect':
    sys.exit(cmd_collect(sys.argv[2:]))
if cmd == 'status':
    sys.exit(cmd_status(sys.argv[2:]))
if cmd == 'prune':
    sys.exit(cmd_prune(sys.argv[2:]))
sys.stderr.write('ps-sweep: unknown helper command %s\n' % cmd)
sys.exit(2)
PYEOF
}

# ------------------------------------------------------------------ misc ----
case "$ACTION" in
  status)
    if [[ "$ACK" == 1 ]]; then
      py status "$STATE" --ack
    else
      py status "$STATE"
    fi
    exit $?
    ;;
esac

# One sweep per state dir: two of them would take the same seed block.
exec 9>"$STATE/lock"
if ! flock -n 9; then
  echo "ps-sweep: another sweep holds $STATE/lock" >&2
  exit 2
fi

say() {
  local line
  line="$(date -u '+%Y-%m-%dT%H:%M:%SZ') $*"
  echo "$line"
  printf '%s\n' "$line" >> "$STATE/log"
}

kill_strays() {
  # Any ps_sim_cli of ours that outlived its batch: kill it by PID.
  local pids p
  pids="$(pgrep -f -- "^$BIN" 2>/dev/null || true)"
  [[ -z "$pids" ]] && return 0
  for p in $pids; do
    [[ "$p" == "$$" ]] && continue
    kill -TERM "$p" 2>/dev/null || true
  done
  sleep 1
  pids="$(pgrep -f -- "^$BIN" 2>/dev/null || true)"
  for p in $pids; do
    [[ "$p" == "$$" ]] && continue
    kill -KILL "$p" 2>/dev/null || true
    LEAKED=$((LEAKED + 1))
  done
  return 0
}

on_exit() {
  local rc="$1"
  trap - EXIT INT TERM
  if [[ -n "$CURRENT_PID" ]] && kill -0 "$CURRENT_PID" 2>/dev/null; then
    kill -TERM "$CURRENT_PID" 2>/dev/null || true
    wait "$CURRENT_PID" 2>/dev/null || true
  fi
  kill_strays || true
  exit "$rc"
}
trap 'on_exit $?' EXIT
trap 'on_exit 130' INT TERM

# ----------------------------------------------------------------- build ----
tip_of() {
  local sha dirty
  sha="$(git -C "$REPO" rev-parse --short HEAD 2>/dev/null || echo none)"
  dirty="$(git -C "$REPO" status --porcelain -- crates 2>/dev/null | head -1 || true)"
  if [[ -n "$dirty" ]]; then
    echo "$sha+dirty"
  else
    echo "$sha"
  fi
}

ensure_binary() {
  local tip want
  if [[ ! -f "$DIFF_JS" ]]; then
    echo "ps-sweep: $DIFF_JS is missing (the repo to build from?)" >&2
    exit 2
  fi
  want="$(tip_of)"
  if [[ "$want" == "none" ]]; then
    echo "ps-sweep: $REPO is not a git repository" >&2
    exit 2
  fi
  if [[ -x "$BIN" && -f "$STATE/tip" && "$(cat "$STATE/tip")" == "$want" ]]; then
    return 0
  fi
  say "build: ps_sim_cli from $REPO at $want"
  rm -rf "$SRC/crates"
  mkdir -p "$SRC/crates"
  local c
  for c in ps_sim ps_protocol ps_client ps_dex ps_harness; do
    if [[ ! -d "$REPO/crates/$c" ]]; then
      echo "ps-sweep: $REPO/crates/$c is missing" >&2
      exit 2
    fi
    cp -a "$REPO/crates/$c" "$SRC/crates/$c"
  done
  cp "$REPO/Cargo.lock" "$SRC/Cargo.lock" 2>/dev/null || true
  cp "$REPO/rust-toolchain.toml" "$SRC/rust-toolchain.toml" 2>/dev/null || true
  # A private workspace with only the crates the CLI needs: the build
  # never sees the rest of the repo and never writes into it.
  cat > "$SRC/Cargo.toml" <<'EOF'
[workspace]
resolver = "2"
members = [
    "crates/ps_client",
    "crates/ps_dex",
    "crates/ps_harness",
    "crates/ps_protocol",
    "crates/ps_sim",
]
EOF
  ( cd "$SRC" && CARGO_BUILD_JOBS="$BUILD_JOBS" cargo build --release \
      -p ps_harness --bin ps_sim_cli --target-dir "$TARGET" </dev/null ) \
    || { echo "ps-sweep: the private build failed" >&2; exit 2; }
  cp "$TARGET/release/ps_sim_cli" "$BIN.new"
  mv "$BIN.new" "$BIN"
  printf '%s\n' "$want" > "$STATE/tip"
  say "build: $BIN is at $want"
}

# The batch always runs the binary that was just built, never a stale one.
# The copy is also kept per tip, so a bucket's repro command replays the
# battle against the engine that failed.
refresh_binary() {
  [[ -f "$TARGET/release/ps_sim_cli" ]] || return 0
  cp "$TARGET/release/ps_sim_cli" "$BIN.new"
  mv "$BIN.new" "$BIN"
  local tip
  tip="$(cat "$STATE/tip" 2>/dev/null || echo unknown)"
  tip="${tip//[^A-Za-z0-9]/-}"
  cp "$TARGET/release/ps_sim_cli" "$STATE/bin/ps_sim_cli-$tip"
}

# ----------------------------------------------------------------- seeds ----
# The counter only ever grows, so a restart never replays a seed. The
# block is taken before the batch runs: a crash mid-batch burns the
# seeds instead of reusing them.
next_seed_block() {
  local want="$1" cur
  cur="$(cat "$STATE/counter" 2>/dev/null || echo "$SEED_START")"
  [[ -z "$cur" ]] && cur="$SEED_START"
  printf '%s\n' "$((cur + want))" > "$STATE/counter.tmp"
  mv "$STATE/counter.tmp" "$STATE/counter"
  echo "$cur"
}

# ------------------------------------------------------------------ lane ----
lane_gen() {
  case "$1" in
    gen1-*) echo 1 ;;
    gen3-*) echo 3 ;;
    *) echo 0 ;;
  esac
}

lane_teams() {
  case "$1" in
    *-wide) echo wide ;;
    *-random) echo random ;;
    *) echo wide ;;
  esac
}

run_lane() {
  local lane="$1" gen teams extra seed_start scratch out rc summary jobs
  gen="$(lane_gen "$lane")"
  teams="$(lane_teams "$lane")"
  if [[ "$gen" != 1 && "$gen" != 3 ]]; then
    echo "ps-sweep: unknown lane '$lane' (gen1-wide, gen3-wide, gen1-random, gen3-random)" >&2
    exit 2
  fi
  extra=""
  [[ "$teams" == "random" ]] && extra="--allow-unported"
  # The harness does not forward --tamper-line to its --jobs children, so
  # a tampered lane runs with one job: the planted bug really happens.
  jobs="$JOBS"
  [[ -n "$TAMPER" ]] && jobs=1
  seed_start="$(next_seed_block "$N")"
  refresh_binary
  scratch="$STATE/scratch/$lane"
  rm -rf "$scratch"
  mkdir -p "$scratch"
  out="$STATE/scratch/$lane.out"
  local -a args=(--ps-cli "$BIN" --gen "$gen" --n "$N" --seed-start "$seed_start"
                 --teams "$teams" --jobs "$jobs" --quiet --scratch "$scratch")
  [[ -n "$extra" ]] && args+=("$extra")
  [[ -n "$TAMPER" ]] && args+=(--tamper-line "$TAMPER")
  say "lane $lane: gen=$gen teams=$teams n=$N seeds=$seed_start..$((seed_start + N - 1)) jobs=$jobs nice=$NICE"
  set +e
  nice -n "$NICE" node "$DIFF_JS" "${args[@]}" </dev/null > "$out" 2> "$out.err" &
  CURRENT_PID=$!
  wait "$CURRENT_PID"
  rc=$?
  CURRENT_PID=""
  set -e
  summary="$(grep -E '^ps-diff: ran=' "$out" | tail -1 || true)"
  if [[ -z "$summary" ]]; then
    say "lane $lane: no summary line (exit $rc): $(tail -2 "$out.err" 2>/dev/null | tr '\n' ' ')"
    summary="ps-diff: ran=0 diffs=0 crashes=0 unported=0"
  fi
  say "lane $lane: ${summary#ps-diff: } (exit $rc)"
  # The failure logs of this lane become buckets.
  # The repro replays one seed against the binary of the tip that failed,
  # and writes its divergence log outside the repo.
  local tip tip_bin
  tip="$(cat "$STATE/tip" 2>/dev/null || echo unknown)"
  tip="${tip//[^A-Za-z0-9]/-}"
  tip_bin="$STATE/bin/ps_sim_cli-$tip"
  [[ -x "$tip_bin" ]] || tip_bin="$BIN"
  local repro="node $DIFF_JS --ps-cli $tip_bin --gen $gen --n 1 --seed {seed} --teams $teams"
  [[ -n "$extra" ]] && repro="$repro $extra"
  [[ -n "$TAMPER" ]] && repro="$repro --tamper-line $TAMPER"
  repro="$repro --jobs $jobs --scratch $STATE/scratch/repro"
  local round_no
  round_no="$(cat "$STATE/round" 2>/dev/null || echo 0)"
  py collect "$scratch" "$STATE" "$(date -u '+%Y-%m-%d')" "$lane" "$gen" "$teams" \
     "$repro" "$MAX_PER_BUCKET" "$round_no" "$(cat "$STATE/tip" 2>/dev/null || echo unknown)" \
     | tee -a "$STATE/log"
  # The counters.
  python3 - "$STATE" "$lane" "$summary" "$out" <<'PYEOF'
import re, sys
state, lane, summary, out = sys.argv[1:5]
m = re.search(r'ran=(\d+)\s+diffs=(\d+)\s+crashes=(\d+)\s+unported=(\d+)', summary)
if not m:
    sys.exit(0)
ran, diffs, crashes, unported = (int(x) for x in m.groups())
pc = re.search(r'ps_crashes=(\d+)', summary)
ps_crashes = int(pc.group(1)) if pc else 0
ids = {}
tail = re.search(r'unported_ids=(\S+)', summary)
if tail:
    for pair in tail.group(1).split(','):
        if ':' in pair:
            k, v = pair.rsplit(':', 1)
            ids[k] = int(v)
else:
    try:
        for line in open(out, errors='replace'):
            if line.startswith('ps-diff: unported ids: '):
                for pair in line.strip().split('unported ids: ', 1)[1].split(','):
                    if ':' in pair:
                        k, v = pair.rsplit(':', 1)
                        ids[k] = int(v)
    except OSError:
        pass
def read(path):
    rows = {}
    try:
        for raw in open(path):
            f = raw.rstrip('\n').split('\t')
            if len(f) >= 2:
                rows[f[0]] = f[1:]
    except OSError:
        pass
    return rows
def read_counts(path):
    """`<lane>\t<id>\t<count>` rows, keyed by the whole prefix."""
    rows = {}
    try:
        for raw in open(path):
            f = raw.rstrip('\n').split('\t')
            if len(f) >= 3:
                rows['\t'.join(f[:-1])] = [f[-1]]
    except OSError:
        pass
    return rows
counters = read(state + '/counters.tsv')
cur = counters.get(lane, ['0', '0', '0', '0', '0'])
cur = [str(int(cur[i]) + v) for i, v in enumerate([ran, diffs, crashes, unported, ps_crashes])]
counters[lane] = cur
body = ''.join('\t'.join([k] + counters[k]) + '\n' for k in sorted(counters))
open(state + '/counters.tsv.tmp', 'w').write(body)
import os
os.replace(state + '/counters.tsv.tmp', state + '/counters.tsv')
unp = read_counts(state + '/unported.tsv')
for ident, count in ids.items():
    key = lane + '\t' + ident
    old = int(unp.get(key, ['0'])[0])
    unp[key] = [str(old + count)]
body = ''.join('\t'.join([k] + unp[k]) + '\n' for k in sorted(unp))
open(state + '/unported.tsv.tmp', 'w').write(body)
os.replace(state + '/unported.tsv.tmp', state + '/unported.tsv')
PYEOF
  kill_strays || true
  printf '%s\n' "$LEAKED" > "$STATE/leaked"
  py status "$STATE" | tee -a "$STATE/log"
}

# ----------------------------------------------------------------- round ----
run_round() {
  local round_no
  round_no="$(( $(cat "$STATE/round" 2>/dev/null || echo 0) + 1 ))"
  printf '%s\n' "$round_no" > "$STATE/round"
  say "round $round_no: start"
  ensure_binary
  local lane
  for lane in $LANES; do
    run_lane "$lane"
  done
  py prune "$STATE" "$MAX_DISK_MB" | tee -a "$STATE/log"
  say "round $round_no: done"
}

case "$ACTION" in
  build)
    ensure_binary
    py status "$STATE"
    exit 0
    ;;
esac

if [[ ! -f "$STATE/counter" ]]; then
  printf '%s\n' "$SEED_START" > "$STATE/counter"
fi

say "sweep: start (repo=$REPO state=$STATE n=$N jobs=$JOBS nice=$NICE lanes=$LANES)"
rounds_done=0
while : ; do
  run_round
  rounds_done=$((rounds_done + 1))
  if [[ "$ONCE" == 1 ]]; then
    break
  fi
  if [[ "$ROUNDS" -gt 0 && "$rounds_done" -ge "$ROUNDS" ]]; then
    break
  fi
  if [[ "$SLEEP" -gt 0 ]]; then
    sleep "$SLEEP"
  fi
done
say "sweep: stopped after $rounds_done round(s)"
py status "$STATE"
