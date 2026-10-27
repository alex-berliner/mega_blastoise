#!/usr/bin/env bash
# The battle server's memory and speed budget (task P2-4; design section 6;
# docs/architecture/showdown-split-budget.md). Prints one table.
#
# (1) Host: `cargo test --release -p ps_sim --test budget` (struct sizes
#     against their ceilings, zero allocations inside a turn over 500
#     seeded battles per gen, host time per turn and the stack high-water
#     mark over the bench's 200 battles per gen).
# (2) Flash: builds `sim_bench` release for thumbv8m, sums the
#     `llvm-nm --print-size` sizes of the ps_sim, ps_dex, ps_protocol and
#     ps_client symbols, and takes the .text / .rodata delta against the
#     same bench built with `--features sim-bench-baseline` (the battles
#     compiled out). The shipped firmware's ps_* symbols are listed too.
# (3) Stack frames: ps_sim built with `-Z emit-stack-sizes` for thumbv8m
#     and for the host; the thumb/host ratio of the frames scales the host
#     stack depth into a thumb estimate.
#
# There is no board yet: every number is a host proxy and every budget is
# a soft gate (the on-chip run is `probe-rs run --preset pico2` of the
# sim_bench ELF, deferred). The script fails only when the host test fails
# (a size over its ceiling, or an allocation inside a turn) or a build does.
#
# Usage: scripts/sim-budget.sh [--quick]   (--quick skips the stack-frame builds)
# Outputs: .scratch/sim-budget/ (the ELFs, the raw test output, table.md).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FW_DIR="$ROOT/crates/mega_blastoise_fw"
OUT="$ROOT/.scratch/sim-budget"
TRIPLE=thumbv8m.main-none-eabihf
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
QUICK=0
for arg in "$@"; do
  case "$arg" in
    --quick) QUICK=1 ;;
    *) echo "usage: $0 [--quick]" >&2; exit 2 ;;
  esac
done
mkdir -p "$OUT"

SYSROOT=$(rustc --print sysroot)
HOST=$(rustc -vV | awk '/^host:/{print $2}')
LLVM="$SYSROOT/lib/rustlib/$HOST/bin"
[[ -x "$LLVM/llvm-nm" ]] || { echo "sim-budget: no llvm-nm (rustup component add llvm-tools-preview)" >&2; exit 1; }

status=0

echo "sim-budget: (1) host test (release)" >&2
if ! (cd "$ROOT" && cargo test --release -p ps_sim --test budget -- --nocapture --test-threads 1 </dev/null) >"$OUT/host.txt" 2>&1; then
  echo "sim-budget: the host budget test FAILED (tail of $OUT/host.txt):" >&2
  tail -20 "$OUT/host.txt" >&2
  status=1
fi

echo "sim-budget: (2) thumbv8m builds (release)" >&2
ELF="$TARGET_DIR/$TRIPLE/release"
# From the fw directory, so its .cargo/config.toml (target, link args) applies.
(cd "$FW_DIR" && cargo build --release --bin sim_bench --features sim-bench-baseline </dev/null) >"$OUT/build.txt" 2>&1 \
  || { tail -20 "$OUT/build.txt" >&2; exit 1; }
cp "$ELF/sim_bench" "$OUT/sim_bench_baseline.elf"
(cd "$FW_DIR" && cargo build --release --bin sim_bench </dev/null) >>"$OUT/build.txt" 2>&1 \
  || { tail -20 "$OUT/build.txt" >&2; exit 1; }
cp "$ELF/sim_bench" "$OUT/sim_bench.elf"
(cd "$FW_DIR" && cargo build --release --bin mega-blastoise-fw </dev/null) >>"$OUT/build.txt" 2>&1 \
  || { tail -20 "$OUT/build.txt" >&2; exit 1; }
cp "$ELF/mega-blastoise-fw" "$OUT/firmware.elf"
for e in sim_bench sim_bench_baseline firmware; do
  "$LLVM/llvm-size" -A "$OUT/$e.elf" >"$OUT/$e.size"
  "$LLVM/llvm-nm" --print-size --demangle --defined-only "$OUT/$e.elf" >"$OUT/$e.nm"
done

if [[ $QUICK == 0 ]]; then
  echo "sim-budget: (3) ps_sim stack frames (-Z emit-stack-sizes, thumbv8m and host)" >&2
  SS="$OUT/stack-target"
  for t in "$TRIPLE" "$HOST"; do
    (cd "$ROOT" && RUSTFLAGS="-Z emit-stack-sizes" CARGO_TARGET_DIR="$SS" \
      cargo build --release -p ps_sim --target "$t" </dev/null) >>"$OUT/build.txt" 2>&1 \
      || { tail -20 "$OUT/build.txt" >&2; exit 1; }
    "$LLVM/llvm-readobj" --stack-sizes --demangle "$SS/$t/release/libps_sim.rlib" >"$OUT/frames-$t.txt" 2>/dev/null
  done
  cp "$OUT/frames-$TRIPLE.txt" "$OUT/frames-thumb.txt"
  cp "$OUT/frames-$HOST.txt" "$OUT/frames-host.txt"
else
  rm -f "$OUT/frames-thumb.txt" "$OUT/frames-host.txt"
fi

HOST_MHZ=$(awk -F: '/^cpu MHz/{printf "%d", $2; exit}' /proc/cpuinfo 2>/dev/null || echo 0)
HOST_CPU=$(awk -F': ' '/^model name/{print $2; exit}' /proc/cpuinfo 2>/dev/null || echo unknown)

# Other builds on the machine slow the host timing; the table shows the load.
LOAD=$(cut -d' ' -f1-3 /proc/loadavg 2>/dev/null || echo unknown)

python3 -I - "$OUT" "$HOST_MHZ" "$HOST_CPU" "$LOAD" <<'PY'
import os, re, struct, sys

out, host_mhz, host_cpu, load = sys.argv[1], int(sys.argv[2] or 0), sys.argv[3], sys.argv[4]
KB = 1024
# Design section 6.
BATTLE_BUDGET = 16 * KB
FLASH_BUDGET = 250 * 1000
TURN_BUDGET_MS = 2.0
STACK_BUDGET = 16 * KB
M33_MHZ = 150
# Host cycles to Cortex-M33 cycles: a Zen 2 core retires about 2 to 4 times
# the instructions per cycle an M33 running from cached XIP flash does on
# this branchy code. The midpoint is the estimate, the ends its range.
K_LO, K_MID, K_HI = 2.0, 3.0, 4.0

def read(name):
    with open(os.path.join(out, name)) as f:
        return f.read()

# ---- (1) the host test's BUDGET lines
budget = {}
for line in read("host.txt").splitlines():
    # libtest may print `test NAME ... ` ahead of a test's first line.
    i = line.find("BUDGET ")
    if i >= 0:
        f = line[i:].split()
        budget[tuple(f[1:3])] = f[3:]

# ---- (2) ELF sections, symbols, the thumb sizes
def sections(name):
    s = {}
    for line in read(name + ".size").splitlines():
        f = line.split()
        if len(f) >= 3 and f[0].startswith("."):
            s[f[0]] = int(f[1])
    return s

PS = ("ps_sim", "ps_dex", "ps_protocol", "ps_client")
CRATE = re.compile(r"([A-Za-z_][A-Za-z0-9_]*)(?:\[[0-9a-f]+\])?::")

def owner(sym):
    crates = CRATE.findall(sym)
    for c in crates:
        if c in PS:
            return c
    return crates[0] if crates else "(other)"

def symbols(name):
    """Bytes per (crate, section kind) from llvm-nm --print-size."""
    acc = {}
    for line in read(name + ".nm").splitlines():
        f = line.split(None, 3)
        if len(f) < 4:
            continue
        _, size, kind, sym = f
        kind = {"t": "text", "r": "rodata", "d": "data", "b": "bss"}.get(kind.lower())
        if kind is None:
            continue
        key = (owner(sym), kind)
        acc[key] = acc.get(key, 0) + int(size, 16)
    return acc

def elf_u32s(path, symbol, n):
    """n little-endian u32 at `symbol` in a 32-bit ELF (its PT_LOAD image)."""
    addr = None
    for line in read("sim_bench.nm").splitlines():
        f = line.split(None, 3)
        if len(f) == 4 and f[3] == symbol:
            addr = int(f[0], 16)
    if addr is None:
        return None
    data = open(path, "rb").read()
    phoff, = struct.unpack_from("<I", data, 0x1C)
    phentsize, phnum = struct.unpack_from("<HH", data, 0x2A)
    for i in range(phnum):
        p_type, p_offset, p_vaddr, _, p_filesz = struct.unpack_from("<IIIII", data, phoff + i * phentsize)
        if p_type == 1 and p_vaddr <= addr < p_vaddr + p_filesz:
            return list(struct.unpack_from("<%dI" % n, data, p_offset + addr - p_vaddr))
    return None

bench, base, fw = sections("sim_bench"), sections("sim_bench_baseline"), sections("firmware")
bench_syms, fw_syms = symbols("sim_bench"), symbols("firmware")
thumb = elf_u32s(os.path.join(out, "sim_bench.elf"), "SIM_BENCH_SIZES", 4)

# ---- (3) stack frames
def frames(name):
    path = os.path.join(out, name)
    if not os.path.exists(path):
        return None
    fns, cur = {}, None
    for line in read(name).splitlines():
        m = re.search(r"Functions: \[(.*)\]", line)
        if m:
            cur = m.group(1)
            continue
        m = re.search(r"Size: (0x[0-9A-Fa-f]+|\d+)", line)
        if m and cur:
            fns[cur] = max(fns.get(cur, 0), int(m.group(1), 0))
            cur = None
    return fns

ft, fh = frames("frames-thumb.txt"), frames("frames-host.txt")
ratio = None
if ft and fh:
    common = [k for k in ft if k in fh]
    ratio = sum(ft[k] for k in common) / max(1, sum(fh[k] for k in common))

# ---- the table
rows = []
def row(measure, value, budget_txt, ok):
    status = "-" if ok is None else ("within" if ok else "OVER (soft)")
    rows.append((measure, value, budget_txt, status))

names = ["Battle", "Side", "Pokemon", "EffectState"]
for i, n in enumerate(names):
    b = budget.get(("size", n))
    if not b:
        row("size_of %s" % n, "missing (host test)", "", False)
        continue
    host, ceiling = int(b[0]), int(b[1])
    t = thumb[i] if thumb else None
    tt = "%d B thumb, " % t if t is not None else ""
    limit = "%d B (16 KB, design 6)" % BATTLE_BUDGET if n == "Battle" else "%d B ceiling (budget.rs)" % ceiling
    row("size_of %s" % n, "%s%d B host" % (tt, host), limit, max(host, t or 0) <= ceiling)

for gen in ("gen1", "gen3"):
    a = budget.get(("allocs", gen))
    if a:
        row("allocations inside a turn, %s" % gen, "%s over %s commands (%s battles)" % (a[0], a[1], a[2]), "0", int(a[0]) == 0)

for gen in ("gen1", "gen3"):
    t = budget.get(("time", gen))
    if not t:
        continue
    prof, turns, p50, p99, mx = t[0], int(t[1]), int(t[2]), int(t[3]), int(t[4])
    row("host time per turn, %s (%s, %d turns)" % (gen, prof, turns),
        "p50 %d ns, p99 %d ns, max %d ns" % (p50, p99, mx), "-", None)
    if host_mhz:
        est = lambda ns, k: ns * host_mhz / 1000.0 * k / (M33_MHZ * 1000.0)
        row("est. M33 time per turn at %d MHz, %s" % (M33_MHZ, gen),
            "p50 %.1f ms (%.1f to %.1f), p99 %.1f ms, max %.1f ms" % (
                est(p50, K_MID), est(p50, K_LO), est(p50, K_HI), est(p99, K_MID), est(mx, K_MID)),
            "%.0f ms per turn" % TURN_BUDGET_MS, est(p99, K_MID) <= TURN_BUDGET_MS)

for gen in ("gen1", "gen3"):
    s = budget.get(("stack", gen))
    if not s:
        continue
    prof, n, p50, p99, mx, setup = s[0], int(s[1]), int(s[2]), int(s[3]), int(s[4]), int(s[5])
    row("host stack per command, %s (%s, %d commands)" % (gen, prof, n),
        "p50 %d B, p99 %d B, max %d B; >player max %d B" % (p50, p99, mx, setup), "-", None)
    if ratio:
        row("est. thumb stack per command, %s" % gen,
            "max %d B, >player %d B (host x %.2f)" % (mx * ratio, setup * ratio, ratio),
            "%d B per turn" % STACK_BUDGET, max(mx, setup) * ratio <= STACK_BUDGET)
if ft:
    top = sorted(((v, k) for k, v in ft.items()), reverse=True)[:5]
    row("largest ps_sim thumb frames", "; ".join("%s %d B" % (re.sub(r"^<ps_sim::[a-z_:]+::(\w+)>::", r"\1::", k), v) for v, k in top),
        "2 KB per dispatch (design 6)", None)

for c in PS:
    t, r = bench_syms.get((c, "text"), 0), bench_syms.get((c, "rodata"), 0)
    row("sim_bench %s symbols" % c, "%d B text, %d B rodata" % (t, r), "-", None)
sym_total = sum(bench_syms.get((c, k), 0) for c in PS for k in ("text", "rodata"))
row("sim_bench ps_* symbols, total", "%d B" % sym_total, "%d B (design 6)" % FLASH_BUDGET, sym_total <= FLASH_BUDGET)
dt = bench.get(".text", 0) - base.get(".text", 0)
dr = bench.get(".rodata", 0) - base.get(".rodata", 0)
row("sim_bench minus baseline", "%d B .text + %d B .rodata = %d B" % (dt, dr, dt + dr),
    "%d B (design 6)" % FLASH_BUDGET, dt + dr <= FLASH_BUDGET)
fw_total = sum(fw_syms.get((c, k), 0) for c in PS for k in ("text", "rodata"))
row("firmware ps_* symbols (all four crates)",
    "%d B (%s)" % (fw_total, ", ".join("%s %d" % (c, fw_syms.get((c, "text"), 0) + fw_syms.get((c, "rodata"), 0)) for c in PS)),
    "-", None)
row("firmware sections", "%d B .text, %d B .rodata, %d B .bss" % (fw.get(".text", 0), fw.get(".rodata", 0), fw.get(".bss", 0)), "-", None)

lines = [
    "Host proxies, soft gates until a board exists. Host: %s at %d MHz, load average %s at the end of the run;" % (host_cpu, host_mhz, load),
    "M33 estimate = host ns x host MHz x %.0f (range %.0f to %.0f) / %d MHz." % (K_MID, K_LO, K_HI, M33_MHZ),
    "",
    "| Measure | Value | Budget | Status |",
    "|---|---|---|---|",
]
lines += ["| %s | %s | %s | %s |" % r for r in rows]
text = "\n".join(lines)
open(os.path.join(out, "table.md"), "w").write(text + "\n")
print(text)
PY

exit $status
