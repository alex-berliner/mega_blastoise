# `tools/ps-diff/`: Showdown protocol-diff harness

A driver that replays the same Pokemon Showdown BattleStream command
sequence on two implementations, compares every line and request, and
reports the first divergence. P1-7 (`docs/architecture/showdown-split-execution.md`
section P1-7) builds this; the rest of the ps_sim port uses it as the
parity fixture at every change.

## Modes

`--self` runs two `pokemon-showdown` `BattleStream`s in parallel sharing
the same choice seed, treating PS as its own reference. This is the
self-test for the harness; every line has to agree.

`--ps-cli <path>` runs one PS `BattleStream` on the left and a long-lived
`ps_sim_cli` (the same BattleStream protocol over stdin/stdout) on
the right. Same commands in, line-by-line diff out. The diff is what
tells working ports from broken ones. Each job process keeps one
`ps_sim_cli` for all its battles (a new one only after a timeout).

`--pinned` reads `crates/ps_harness/pinned.txt`
(`<gen> <teams> <seed> [<trunc>]  # <commit> <note>`; the optional
`<trunc>` is a Gen 1 entry's rule set, `cartridge` or `math`, below) and
replays each entry as a
self-contained battle: against `ps_sim_cli` with `--ps-cli`, as a
`--self` battle without it. Pinned seeds are the ones a previous
regression already pinned to a known-good output; if a future port of
the same logic breaks one of them, the harness fails at that exact
seed with full divergence context. Empty / comment-only files print one
summary line and exit 0 (the green path for "no pinned regressions yet").

`--carried gen1` replays the carried corpus: the old engines' pinned
battles (below), every team pair over battle seeds 0..19.

`--targeted gen1` plays the targeted battles (below): hand-made team
pairs with fixed choices that reach the Gen 1 paths random teams never
do.

`--trunc cartridge|math` picks the Gen 1 rule set (below); every mode
takes it, `--self` included.

`--replays DIR` replays the public replay corpus (V-3, V-4) with the
choices each replay's `inputlog` recorded: the pinned PS on the left,
`ps_sim_cli` on the right with the team PS generated (packed). A choice
`ps_sim` rejects is a diff. `--validated` keeps only the replays
`replays.js validate` kept; `--format` takes one format id or a comma list.
The corpus comes from `replays.js` (below, Replays).

`--hits FILE` (against a `ps_sim_cli` built with the `hit-count`
feature) has every `ps_sim_cli` write its per-handler hit counts to
FILE, `{pid}` replaced by `<pid>-<start ns>` so each process, respawns
included, writes its own; `--jobs` children get the flag too.
`scripts/ps-coverage.sh` sums the files (Coverage gate, below).

## Usage

```
scripts/ps-diff.sh --self --gen 1 --n 200
scripts/ps-diff.sh --self --gen 3 --n 200 --teams random
scripts/ps-diff.sh --self --gen 1 --n 50 --teams wide
scripts/ps-diff.sh --self --seed 0 --gen 1
scripts/ps-diff.sh --pinned
scripts/ps-diff.sh --carried gen1 --ps-cli "$PWD/target/release/ps_sim_cli" --allow-unported --jobs 6
scripts/ps-diff.sh --targeted gen1 --ps-cli "$PWD/target/release/ps_sim_cli" --trunc math
scripts/ps-diff.sh --ps-cli "$PWD/target/release/ps_sim_cli" --gen 3 --n 200
scripts/ps-diff.sh --ps-cli "$PWD/target/release/ps_sim_cli" --gen 1 --n 200 --teams wide --trunc math
scripts/ps-diff.sh --replays /root/replay-cache/current --validated --format gen1randombattle --ps-cli "$PWD/target/release/ps_sim_cli" --jobs 6
```

## Replays

`replays.js` keeps the corpus of public replays from
`replay.pokemonshowdown.com` (one request per second, backoff on 429 and
5xx, resumable, never downloads a replay twice):

```
node tools/ps-diff/replays.js fetch    --cache /root/replay-cache --era current --format gen1randombattle,gen3randombattle --target 1000
node tools/ps-diff/replays.js validate --cache /root/replay-cache --era current --jobs 6
node tools/ps-diff/replays.js report   --cache /root/replay-cache
```

The seed picks the era. `gen5` (numeric seeds, `Gen5RNG`, before 2025) lives
in `<cache>/<format>/`; `current` (`sodium,<hex>` seeds, `SodiumRNG`) in
`<cache>/current/<format>/`. `validate` plays each replay's input log on the
pinned PS and keeps it when PS's omniscient log matches the replay's `log`
(the server's own lines ignored), writing `validated/<format>.json` with the
drop reasons. The OU formats have no input log (the server keeps one only for
generated teams), so only their logs are used, by the `ps_client` parse test
(`crates/ps_client/tests/replay_logs.rs`). A committed sample lives in
`crates/ps_harness/replays/` (`<format>/` for the numeric-seed era,
`current/<format>/` for 20 validated `sodium,` replays per random format).
`--replays` walks DIR recursively, so `/root/replay-cache` covers both eras
and `/root/replay-cache/current` only the current one.

`--self` or `--ps-cli` is required; both can be combined with `--pinned`
(pinned runs are PS-vs-PS unless a `--ps-cli` is given). `check.sh`
runs `--pinned --ps-cli ... --allow-unported`. The smoke runs in `scripts/check.sh` use
`--self --n 20 --teams random|wide`, the deep sweep uses
`--gen 1|3 --n 200 --teams random`.

## Gen 1 rule sets (`--trunc`)

Gen 1 has two rule sets, and the harness diffs both. `cartridge` (the
default) is the cartridge's truncation: `trunc(x, bits)` wraps (PS's
`Dex.trunc`, `num >>> 0` then `% 2 ** bits`), so a stat past 1023 rolls
over in the damage formula and a PP underflow lands on 63. `math` is PS's
Custom Game math: `Math.trunc`, which ignores `bits`. Stock PS decides
this per format (every Custom Game sets `battle: {trunc: Math.trunc}`,
Random Battle keeps `Dex.trunc`); here it is a format rule,
`Cartridge Trunc` (design section 3), that the `>start` formatid names:

| `--trunc` | wide, carried, file | random |
|---|---|---|
| `cartridge` | `gen1customgame` | `gen1randombattle` |
| `math` | `gen1customgame@@@!Cartridge Trunc` | `gen1randombattle@@@!Cartridge Trunc` |

`rules.js` installs the rule in PS before the first battle: it gives both
Gen 1 formats `battle: {trunc: Math.trunc}`, appends `Cartridge Trunc` to
their rulesets, and the rule's `onBegin` sets `this.trunc =
this.dex.trunc`. With the rule in effect `gen1randombattle` is exactly
stock PS's; `!Cartridge Trunc` repeals it with PS's own repeal logic
(`getRuleTable`), and PS writes the custom-rule infobox
(`1 custom rule: !Cartridge Trunc`). `ps_sim` reads the same formatid:
`Cartridge Trunc` ends every Gen 1 ruleset (`battle.rs rule_table`) and
picks `Battle::trunc`; the device plays the default. Both sides read the
one formatid, so `--self` and `--ps-cli` runs need nothing else. `--trunc`
on another gen exits 2; Gen 3 takes no rule. Gen 1 summary lines end in
`trunc=<mode>`, and a divergence log's first line names it.

## Seed derivation

All three seeds (battle, team, choice) come from one driver seed via a
Gen5RNG rooted at `[driver[0..16], (driver >> 16), driver ^ 0xa5a5, 0]`,
four u16 values laid out to fit `PRNG([u16; 4])` without a re-key. The
battle seed is four u16, exactly PS's `seed` shape. The team seed and
choice seed are four u16 each (PS's `Gen5RNG` returns 0 forever from
a shorter seed). Same driver seed => identical battle in
both drivers, no second chance to drift independently.

## Team builders

`teams.js` exposes three builders; the harness picks one with
`--teams`:

- `random` -- `Teams.generate('genNrandombattle')` for the chosen
  generation. Listed in `PS/data/random-battles/gen<N>/`. Uses PS's
  own generator; the only gen-agnostic path. p1's team comes from the
  team seed, p2's from a seed drawn from it.
- `wide` -- `buildWide` enumerates species by `dex.species.all()`,
  picks 1-6 mons per side, 1-4 distinct moves each from the gen's
  move pool, level 40-100, IVs 0-31, EVs <= 510 with a rotating
  starting stat. Gen 3 adds item and ability whitelists. The pool is:
  - `--self`: the whole dex (no restriction), 1-6 mons, every move the
    gen has. The draw is identical between the two PS sides because
    the same seed lands on the same draws; this is the harness's
    own self-test.
  - `--ps-cli`: restricted to the `kind:id` lines
    `ps_sim_cli --ported <gen>` prints; builds call exit 2 with
    "no ported pipeline for gen N" if the list is empty.
    A gen 3 mon whose species has no ported ability draws one from
    the ported pool: a packed empty ability is not "none", PS's
    `Teams.unpack` turns it into the species' first ability.
  - `--focus ids`: a team that carries none of the ids gets one: a move
    id replaces a random move of a random mon; in gen 3 an ability or
    item id becomes that mon's ability or item (custom games do not
    validate the set). A move-only focus draws as it always has.
  Gen 3 items: 60% of mons hold none, the rest one drawn from the
  `item:` lines. `ps_sim_cli --ported 3` lists every item `item_is_ported`
  accepts: each handler PS's data gives it is registered, or it has
  none (Poke Balls, evolution stones, fossils, Quick Claw, which gen 3
  reads in `getActionSpeed`), so unfocused wide teams carry those too.
  An item whose handlers add a condition is listed once that condition
  is ported too (`item_conditions` in `ps_sim/src/mods/mod.rs`: Lansat
  Berry waits for `focusenergy`).
- `file:<path>` -- `buildFromFile` reads the file as PS packed teams
  (one per line), unpacking with PS's own `Teams.unpack`.

`--focus ids` (wide teams) makes every team carry at least one of the
move ids among its moves and, in gen 3, at least one of the item ids as
a held item (`splitFocus` in `teams.js`). `move:<id>` and `item:<id>`
name the kind; a bare id is a move when the gen's dex has that move,
else an item when it has that item (so `--focus sitrusberry,lumberry`
puts those berries on the teams). A moves-only focus draws exactly what
it drew before item ids were accepted. An unported focus item stops
the battle `unported: <id>` like any other effect.

## Carried corpus

`crates/ps_harness/carried/gen1/*.json` carries the old engines' pinned
corpus over to this harness. Each file is one `fuzz_gen1_battles` entry of
`crates/mega_blastoise_test/pinned_seeds.txt`, named the way
`scripts/pinned-seeds.sh` names its logs
(`fuzz_gen1_battles_w<wide>_s<seed>[_c<case>][_only].json`), holding every
case's two teams (one case with `case=`) as PS sets: a header line naming
the run (`suite`, `gen`, `seed`, `wide`, `fuzz_n`, `case`, `only`), then one
`{"case", "p1", "p2", "status"}` object a line. `status` is the case's
starting statuses, which are not a set field; the carried run does not
apply them. Gen 1 sets name `noability` (an empty ability unpacks to the
species' later-gen one).

The files come from the fuzz suite itself: `FUZZ_DUMP=<path>` makes
`fuzz_gen1_battles` play every case's own side (the generator stream is
unchanged) and write the teams instead of asking the reference.
`scripts/carried-dump.sh gen1` builds the fuzz binary once and dumps every
entry; `scripts/carried-dump.sh gen1 --check` dumps to `.scratch/carried/`
and fails unless the result is byte-identical to the committed files (the
dump is deterministic). Re-dump after adding a gen 1 battle entry to
`pinned_seeds.txt`.

`--carried gen1` (PS against itself without `--ps-cli`, like `--pinned`)
reads the files in name order and plays each distinct team pair (a `case=`
entry inside a whole-seed one is played once, under the first name) as a
`gen1customgame` over driver seeds 0..19: the battle and choice seeds come
from the driver seed as usual, the team seed is unused. `--seed S` plays
only seed S of every pair; `--carried-pair <file>#<case>` (for example
`fuzz_gen1_battles_w1_s7132#4`) only that pair. A carried log is named
after its pair as well as its seed:
`ps-diff-carried-<file>-<case>-<seed>.log`, and its first line names the
pair. One divergence replays with

```
scripts/ps-diff.sh --carried gen1 --ps-cli "$PWD/target/release/ps_sim_cli" \
  --carried-pair 'fuzz_gen1_battles_w1_s7132#4' --seed 3 --jobs 1
```

The carried run is part of the Gen 1 phase gate (after G1-7), not of
`scripts/check.sh` yet.

## Targeted battles

`crates/ps_harness/targeted/gen1.json` holds battles built to reach one
path each that the random and wide sweeps never run (Recover's 255/511
failure, the `nomove` Fissure, a defense that rolls over to 0, and so on).
It is a JSON array, one entry a line:

```
{"name", "covers", "seed", "trunc"?, "p1", "p2", "choices", "expect"}
```

- `covers`: the `ps_sim` lines the battle is there for, and the PS source.
- `p1`, `p2`: lists of PS sets. A set gives `species` and `moves`, and may
  give `level`, `evs`, `ivs`; the rest defaults to level 100, every EV 255,
  every IV 30 and ability `noability` (the carried sets' defaults).
- `seed`: the driver seed. The battle and choice seeds come from it as
  usual (Seed derivation); the team seed is unused.
- `choices`: `[p1, p2]` rows, `null` for a side that does not choose in
  that row. Each side's choices answer its requests in order (a `wait`
  request takes none); once a side's list runs out, it chooses at random
  from the choice seed, as every other battle does. A choice PS rejects
  (`[Invalid choice]`) stalls a battle, so the PS side throws and the
  battle counts as a crash.
- `expect`: lines (substrings) PS's log must hold, the protocol trace of
  the path (a hint, a `-start`, a `|debug|` line). If one is missing the
  battle no longer reaches its path (a changed seed, team or PS) and
  counts as a diff, with `ps-diff-targeted-<name>-<seed>-expect.log`.
- `trunc`: an entry that only exists in one rule set names it (the
  defense rollover is the cartridge's 8-bit truncation); it plays that
  one whatever `--trunc` says. The others play the run's `--trunc`.

Each battle is a `gen1customgame` (no clauses). `--targeted-name <name>`
plays one entry; with `--seed S` it plays that entry at driver seed S,
which is how a seed was found for each: play the entry over a seed range
on PS alone and keep one whose log has every `expect` line. Divergence
logs are `ps-diff-targeted-<name>-<seed>.log`.

`scripts/check.sh` plays the targeted battles against `ps_sim_cli` in
both rule sets, and the coverage gate's sweep includes them (both rule
sets), so the lines they reach count as covered. A line that only a
targeted battle runs needs an entry here; a line Showdown cannot reach in
Gen 1 goes in `crates/ps_harness/coverage-allow.txt` instead.

## Coverage gate

`scripts/ps-coverage.sh` (V-1) builds `ps_sim_cli` with the `hit-count`
feature under `cargo llvm-cov` and plays the gate sweep against it: by
default 10,000 `--teams random` and 10,000 `--teams wide` Gen 1 battles
from seed 1000000, plus (Gen 1) the targeted battles in both rule sets
(`--no-targeted` leaves them out). Every `ps_sim_cli` writes its hit
counts with `--hits`. It then prints the per-handler hit table and the
line coverage of `crates/ps_sim/src/mods/genN/**`, and passes when every
registered handler ran in at least 20 battles and every line ran or is
in `crates/ps_harness/coverage-allow.txt` (each entry with its reason and
the PS line). `--report-only` re-prints the tables after an allow-list
edit; reports land in `.scratch/coverage/gen<N>/`.

### The allow-list format

`coverage-allow.txt` names lines by their source text, not their line
numbers, so an entry survives edits elsewhere in the file (V-1b; the
re-sync to c0845f33d moved most line-number entries by 5). The full
format is at the top of the file; in short:

```
<path under crates/ps_sim/src>  <selector>  <reason>; PS: <file>:<line>
handler <gen> <kind:id> <key>  <reason>; PS: <file>:<line>

mods/gen1/scripts.rs  `return 0;`  ...                       one line, its text unique in the file
mods/gen1/scripts.rs  `}`#3  ...                             the 3rd occurrence
mods/gen1/scripts.rs  `}`@120  ...                           the occurrence nearest line 120
mods/gen1/scripts.rs  `s.counter = 0;` > `}`  ...            the first `}` after that line
mods/gen1/moves/dig.rs  `fn on_prepare_hit(...) {` .. `}`  ...  a range, to the first `}` after the start
```

Each anchor is a whole source line, trimmed, in backticks. A repeated
line needs `#n`, `@n` or a `>` chain (closing braces are best chained off
the unique line above them); otherwise the entry is an error. The PS line
is at the pin (`node_modules/pokemon-showdown` here).

The gate reports two kinds of stale entry, without failing on them: an
entry whose text is no longer in the file, and an entry none of whose
lines is unrun (the code now runs). `scripts/coverage-allow.py --check`
(run by `scripts/check.sh`) checks that every entry still resolves, so an
edit that breaks an entry fails the landing, not only the next coverage
run; `--show` prints the lines each entry resolves to now.

## What the driver does

For each entry, `driverCommands` builds three lines (the PS BattleStream
envelope, in order):

- `>start {formatid, seed}`
- `>player p1 {name, team}`
- `>player p2 {name, team}`

then feeds them to both drivers. On every `|request|pN|{json}` line the
driver picks a random legal choice from PS's request using the SAME
choice seed: both drivers deterministically choose the same byte
sequence. Gen 1's truncation is the formatid's rule set (above).

After both drivers return, `firstDiff` walks both logs at the LINE
level. `|t:|N` is normalised to `|t:||` (Date.now() drift). Every
other line, including standalone `|` separators between phase chunks,
is part of the diff so a `--self` run stops matching the moment PS's
internal bookkeeping actually diverges.

If any line at index `i` differs:

```
.scratch/ps-diff/
  ps-diff-<seed>.log
    seed N gen 1 teams random diff @ line 5
    prngCallsA=124 prngCallsB=124
    --- context (left: line 3..8) ---
    [3] L: |player|p1|p1||
    [4] L: |player|p2|p2||
    [5] L: |gen|1
    >> [5] L: |tier|[Gen 1] Random Battle | R: |tier|[Gen 1] Random Battle|tampered|mark|line=5| <<
    [6] L: |rule|Desync Clause Mod: ...
    [7] L: |rule|HP Percentage Mod: HP is shown in percentages
    [8] L: |rule|Endless Battle Clause: ...
    --- left ---
    <full normalised log from the left driver>
    --- right ---
    <full normalised log from the right driver>
```

Pinned runs also save the parse-able log. Crashes (a `crash` message
from `ps_sim_cli`, the process dying, a timeout, an exception during
drain) land in the same dir under `ps-diff-<seed>-crash.log` with the
rust side's partial log, and set the run's exit code.

## The PRNG trace

`PS_RNG_TRACE=1` turns the PRNG trace on. Every PRNG call of both sides
is recorded with its index, its method, its arguments, its result and
the call site, and a divergence report writes the two traces side by
side around the first call where they differ. The trace never changes
what a battle does: PS's PRNG methods are wrapped (the wrappers call
the originals) and `ps_sim`'s trace is compiled in.

The PS side (in `diff.js`): `PRNG.prototype.random`, `randomChance`,
`sample` and `shuffle` are wrapped, and the underlying RNG's `next` is
wrapped too (a direct `next()` is a call of its own; the `next` inside
a `random()` belongs to that `random`). One entry per public call:

| Method | Arguments | Result |
|---|---|---|
| `random` | none, `[n]` or `[m, n]` (the overload PS picks) | the number |
| `random_chance` | `[numerator, denominator]` | true / false |
| `sample` | `[length]` | the index it picked |
| `shuffle` | `[length, start, end]` | the indices it picked |
| `next` | none | the 32-bit draw |

The call site is the first stack frame outside `prng.js` (the frames of
`diff.js` itself are the tracing, so they are skipped); the report also
prints the `origin`, the first frame outside `prng.js` and `battle.js`
as well: the code that asked the battle for a number.

The `ps_sim` side: the `rng-trace` feature of `ps_sim` (off by
default; the firmware never enables it) records the same calls in
`Prng::trace`, with the caller location from `#[track_caller]`. The
feature is propagated through `ps_harness`, so a traced binary is

```
cargo build -p ps_harness --features rng-trace --bin ps_sim_cli
```

`ps_sim_cli --rng-trace` then sends the battle's calls as an
`rngtrace` message (one JSON line, before the battle's `end`):

```
rngtrace
{"calls":[{"args":[0,2],"index":0,"method":"random","result":0,"site":"crates/ps_sim/src/queue.rs:423"}, ...]}
```

`diff.js` passes `--rng-trace` to `ps_sim_cli` when `PS_RNG_TRACE=1` is
set, and reads the `rngtrace` message. A divergence report then ends
with the two traces side by side:

```
--- PRNG trace (first differing call: 5) ---
  #  | PS (pokemon-showdown) | ps_sim (rust)
     4 | random_chance args=[191,256] result=true site=dist/sim/battle.js:213 origin=dist/data/mods/gen1/scripts.js:414 | random_chance args=[191,256] result=true site=crates/ps_sim/src/mods/gen1/scripts.rs:552
>>   5 | random args=[] result=0.4077296678442508 site=diff.js (injected extra random) | shuffle args=[2,0,2] result=[0] site=crates/ps_sim/src/events.rs:103
     6 | shuffle args=[2,0,2] result=[1] site=dist/sim/battle.js:288 origin=dist/sim/pokemon.js:1267 | random args=[1,8] result=4 site=crates/ps_sim/src/mods/gen1/conditions/slp.rs:34
```

`PS_RNG_TRACE_EXTRA=N` is the test hook: the PS side draws one extra
`random()` before the call at index N (the extra call is the call at
index N), so the report shows where the two sides' PRNG streams part
ways. `crates/ps_harness/tests/rng_trace.rs` plays one Gen 1 battle
with it and checks that the report names the injected call's index and
both call sites.

## Unported effects

When `ps_sim` reaches an effect with no port it panics `unported: <id>`;
`ps_sim_cli` catches that per battle, writes an `unported` message with
the id, and goes on with the next `>start`. The harness counts the
battle as `unported` and still diffs everything the rust side wrote
before the stop against the same stretch of PS's log (a mismatch there
is a diff). Every summary line carries the counts per id
(`unported_ids=brn:3,...`) and the `--jobs` parent prints the merged
list as `ps-diff: unported ids: ...`, most frequent first.

`--allow-unported` keeps unported battles from failing the run (diffs
and crashes still do). Use it with `--teams random`, whose sets carry
moves and statuses that are not ported yet; `--teams wide` against
`ps_sim_cli` draws only from the ported list and should see none.

## `--self` mode end-to-end

PS is its own truth. Two BattleStream drives on the same commands with
the same choice seed, and the diff between them is the engine noise of
running the same battle twice. The smoke test is
`--self --gen 1 --n 200` returning 0. The more interesting smoke test
is `--tamper-line K`: a known edit at line K surfaces a divergence at
that exact line, with the `|tampered|mark|line=K|` sentinel showing
up in the right log and the divergence report naming the line.

## Exit codes

`diff.js` exits non-zero whenever `diffs + crashes + unported > 0`
(`diffs + crashes` with `--allow-unported`). The
wide/ps-cli lane with an empty `ps_sim_cli --ported` list exits 2 with
"no ported pipeline for gen N", which is the gate's signal that a port
has not landed yet for that gen. Sidestep checks (no `--self` and no
`--ps-cli`, malformed `--teams`, unknown gen, missing `ps_sim_cli`)
also exit 2.

## Jobs

`--jobs N` (default 14) splits a run into N node child processes, each
playing a `--slice a:b` of the same driver list (seeds or pinned
entries). Each child prints one summary line on stdout; the parent
prints the merged `ps-diff: ran=R diffs=D crashes=C unported=U gen=G
teams=T mode=... jobs=N`. Without `--quiet`, the children's other
output is forwarded too. The gate runs always use 14 jobs.

## Cargo path

`crates/ps_harness/src/bin/ps_sim_cli.rs` is the server-side binary the
diff calls from `--ps-cli`: `ps_sim` behind the BattleStream protocol.
It reads `>start`, `>player pN`, `>pN <choice>` and writes each
BattleStream message as its type line (`update`, `sideupdate`, `end`),
its data and a newline, rendered by the `ps_protocol` text codec
(`encode_log` for the `|split|` framing, `encode_request`). `unported`
and `crash` (each with one payload line) end a battle early. It also
exposes `--ported <gen>` (ps_sim mods `ported(gen)` is the single
const list per gen) so the diff harness never carries its own whitelist
of effect ids. Gen 1 lists `move:<id>` for every handler-free move and
`species:<id>` for every species (the wide pools need both).

`crates/ps_harness/tests/protocol_diff.rs` is the integration test
ensuring `pinned.txt` parses, `ps_sim_cli --ported` matches the contract
on gens 1/3 and exits 2 on others, and that the driver-side
divergence-writing path lands the `ps-diff-*.log` it claims.

## The continuous sweep (V-2)

`scripts/ps-sweep.sh` is the background sweep: a loop for tmux on the
worker machine that plays protocol-diff batches of fresh seeds against
PS, forever, and files every failure into a bucket named after the first
differing line. It is the lane that finds the rare handlers the random
gate sweeps miss.

```
tmux new -d -s ps-sweep 'scripts/ps-sweep.sh --n 2000 --jobs 8 --nice 19'
scripts/ps-sweep.sh --once                # one round, then stop
scripts/ps-sweep.sh --rounds 3            # three rounds, then stop
scripts/ps-sweep.sh status                # print the one-line status
scripts/ps-sweep.sh status --ack          # print it, advance the read cursor
scripts/ps-sweep.sh build                 # rebuild the private binary only
```

The sweep builds its own copy of `ps_sim_cli` from the tip of the
integration worktree (`--repo`, default `/root/wt/ps-split`), rebuilds
when the tip moves, and runs the driver from that same repo, so the PS
side, the Rust side and the dex all come from one commit. The repo is
only ever read: the sources are copied into a private workspace
(`<state>/src`) and built into a private target dir (`<state>/target`),
and the binary is copied to `<state>/bin/ps_sim_cli` before every batch.

The state lives outside every worktree, in `--state-dir` (default
`/root/pv/sweep`):

| Path | What |
|---|---|
| `counter` | the next seed. It only ever grows, so a restart never replays a seed |
| `status` | the one-line status (below) |
| `buckets.tsv` | the ledger: one row per bucket (count, first round, last seed, lanes) |
| `counters.tsv` | per lane: battles, diffs, crashes, unported, ps_crashes |
| `unported.tsv` | per lane and per effect id: how many battles stopped there |
| `<date>/<bucket>/` | the failures: the `ps-diff-*.log` files and `repro.txt` |
| `src/`, `target/`, `bin/` | the private build |
| `log` | the sweep's own round log |

### The four lanes

One round plays four lanes, 2,000 battles each, every lane with its own
fresh block of seeds:

| Lane | Command |
|---|---|
| `gen1-wide` | `--gen 1 --teams wide` |
| `gen3-wide` | `--gen 3 --teams wide` |
| `gen1-random` | `--gen 1 --teams random --allow-unported` |
| `gen3-random` | `--gen 3 --teams random --allow-unported` |

The wide lanes draw only from `ps_sim_cli --ported`, so they should see
no unported stop; an unported stop there means the ported list is out of
date. The random lanes use PS's own random-battle teams, which carry
effects that are not ported yet, so most of those battles stop early.
Those stops are progress data, not failures: they are counted per effect
id in `unported.tsv` and in the status line. Only a diff and a crash
make a bucket. A battle where PS itself throws and `ps_sim` stops too is
a PS bug both sides share: the harness counts it as `ps_crashes`, and the
sweep does not bucket it.

### The status line

One line, rewritten after every lane:

```
ps-sweep: ts=<utc> round=<n> tip=<sha> counter=<next seed> battles=<n> \
diffs=<n> crashes=<n> unported=<n> unported_ids=<id:count,...> \
lanes=<lane:battles/diffs/crashes/unported,...> buckets_total=<n> \
buckets_new=<n> buckets_new_list=<name,...> ps_crashes=<n> leaked=<n> \
disk_mb=<n>
```

`buckets_new` counts the buckets first seen after the last read cursor.
The cursor moves only when the reader asks for it
(`scripts/ps-sweep.sh status --ack`), so a bucket is reported as new
exactly once and never lost if nobody reads it.

### The buckets

A bucket is named after the first differing line, the way G1-7's table
lists them: `gen<gen>-<teams>-<kind>-<slug>`, where `kind` is the PS
line's kind (`move`, `request`, `damage`, `faint`, ...) and `slug` is
the rest of the line with every number replaced by `n`. A crash gets the
kind `crash` and the slug of the error's first line. Each bucket holds
up to five `ps-diff-*.log` files (the divergence report with both logs)
and a `repro.txt` with the seed, the tip and the command that replays
it. The command points at the binary of the tip that failed
(`bin/ps_sim_cli-<tip>`, kept next to the current one) and writes its
divergence log outside the repo:

```
seed 1000220 gen 1 teams wide lane gen1-wide tip 8fdaac4
  node /root/wt/ps-split/tools/ps-diff/diff.js --ps-cli \
    /root/pv/sweep/bin/ps_sim_cli-8fdaac4 --gen 1 --n 1 --seed 1000220 \
    --teams wide --jobs 1 --scratch /root/pv/sweep/scratch/repro
```

### Planted bugs and hygiene

`--tamper-line K` passes the driver's own tamper hook to the batch, so
a planted divergence shows up as a bucket. The driver does not forward
the flag to its `--jobs` children, so a tampered lane runs with one job.

The sweep kills every `ps_sim_cli` that outlives its batch (by PID, never
by pattern kill) and counts the ones it had to kill in `leaked`. The
failure buckets are capped (`--max-disk-mb`, default 512) and each
bucket keeps at most five logs (`--max-per-bucket`); the oldest go
first.
