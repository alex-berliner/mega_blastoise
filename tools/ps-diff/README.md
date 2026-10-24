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
```

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
- `file:<path>` -- `buildFromFile` reads the file as PS packed teams
  (one per line), unpacking with PS's own `Teams.unpack`. A `focus`
  filter on `--focus ids` makes every team carry at least one of
  those ids among its moves.

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
