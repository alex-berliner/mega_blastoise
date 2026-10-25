#!/usr/bin/env node
'use strict';
// tools/ps-diff/diff.js
//
// Protocol-diff driver for the new `ps_sim` server. Plays the same
// command stream on (a) a real PS BattleStream and (b) either a second
// real PS BattleStream (`--self`) or the long-lived Rust `ps_sim_cli`
// subprocess. Compares every line and the `|request|` payloads; the
// first divergence writes both logs, the seed, the line number and
// context, and both PRNG call counts to `.scratch/ps-diff/`.
//
// PS is sourced at 0.11.11 (`tools/ps-diff/node_modules/pokemon-showdown`).
// Stream interface: PS's custom stream uses `await stream.next()` to
// receive `${type}\n${data}` chunks; `stream.write(cmd)` feeds inputs.
//
// Modes:
//   --self             PS-vs-PS (the harness's own self-test).
//   --ps-cli <path>    one PS, one rust.
//   Either is required.
//
//   --replays DIR      the replay corpus (V-3, V-4): every replay in DIR
//                      is played twice with the choices its `inputlog`
//                      recorded, once on the pinned PS and once on
//                      `ps_sim_cli`, and the two logs are compared line
//                      for line. `ps_sim` gets the team PS generated
//                      (packed), since it has no team generator; the
//                      `>player` seed would make PS and `ps_sim` play
//                      different teams. A choice `ps_sim` rejects is a
//                      diff. `--validated` keeps only the replays
//                      `replays.js validate` kept (DIR is a cache root:
//                      `/root/replay-cache` for the numeric-seed era,
//                      `/root/replay-cache/current` for the `sodium,`
//                      one).
//
// Battle loop:
//   - `--teams random` -> PS's own genNrandombattle generator.
//   - `--teams wide`   -> the showdown_fuzz wide fuzz space; limited to
//                         the species/moves/items/abilities list from
//                         `ps_sim_cli --ported <gen>` when run against
//                         the rust side, otherwise the whole gen.
//   - `--teams file:<path>` -> two packed teams from a file.
//   - `--trunc cartridge|math` (Gen 1 only, default `cartridge`) -> the
//                         Gen 1 rule set: `math` names `!Cartridge Trunc`
//                         in the `>start` formatid both sides read
//                         (`rules.js`).
//   - `--carried gen1` -> the old engines' pinned battles
//                         (`crates/ps_harness/carried/gen1/*.json`, written
//                         by `scripts/carried-dump.sh`): every team pair over
//                         battle seeds 0..19 (`--carried-pair <file#case>`
//                         narrows to one pair, `--seed S` to one seed).
//   - `--targeted gen1` -> the targeted battles
//                         (`crates/ps_harness/targeted/gen1.json`): hand-made
//                         team pairs, each with its own driver seed and a
//                         list of fixed choices (random legal choices after
//                         it runs out), that reach the paths the random
//                         sweeps never do (`--targeted-name N` plays one).
//
// Exit codes: 0 green; 1 diff / crash / unported hit (unported does not
// count with --allow-unported); 2 misuse or empty
// `wide --ps-cli`. A battle where PS itself throws (a PS bug) and ps_sim
// crashes too is not comparable: it counts as `ps_crashes=K` on the
// summary line and does not fail the run; a PS throw that ps_sim plays
// through is a crash.
//
// Important behaviour:
//   * `--jobs N` (default 14) splits a run into N node child processes
//     by seed range. Each child prints its own summary line; the parent
//     reads them all, sums to one merged summary line, and exits with
//     the bytewise OR of the children's exit codes.
//   * `--tamper-line K` (negative = left log, positive = right log) is
//     a test hook: after both logs are normalized, that line index has
//     a sentinel `|tampered|mark|line=<K>|` written into it. The diff
//     is the regression the cargo test pins at a known line.
//   * `--focus <ids>` makes every team carry at least one of those ids
//     among its moves (in gen 3 also as a mon's ability or held item;
//     teams.js `splitFocus`: `move:`/`ability:`/`item:` prefixes, a bare id
//     is a move when the dex has that move, else an ability, else an item).
//     Used by the cargo test for replay.
//   * `--hits <file>` is passed on to every `ps_sim_cli` (a `hit-count`
//     build): it writes its per-handler hit counts there, `{pid}`
//     replaced by its process id and start time.
//     `scripts/ps-coverage.sh` sums them.
//   * `--quiet` collapses the per-run trace to one summary line at the
//     end. Without it, the diff's seed, line number, left/right context
//     and PRNG call counts are written, plus we print per-battle progress
//     milestones for long runs.

const fs = require('fs');
const path = require('path');
const { spawn, spawnSync } = require('child_process');
const { BattleStream, PRNG, Teams } = require('pokemon-showdown');
const teams = require('./teams');
const { spawnPorted } = require('./ported');
const rules = require('./rules');
const { parseInputLog, takeForces } = require('./replays');

const DEFAULT_TIMEOUT_MS = 30000;
const DEFAULT_JOBS = 14;
// Battle seeds per carried team pair.
const CARRIED_SEEDS = 20;

function parseArgs(argv) {
  const opts = {
    gen: 3, n: 200,
    format: 'all',
    seed: undefined,
    teamsMode: 'random',
    focus: null,
    self: false,
    pinned: false, pinnedFile: undefined,
    quiet: false,
    seedStart: 0, seedEnd: undefined,
    psCli: undefined,
    hits: undefined,
    scratch: undefined,
    timeoutMs: DEFAULT_TIMEOUT_MS,
    jobs: DEFAULT_JOBS,
    tamper: undefined,
    allowUnported: false,
    slice: undefined,
    carried: undefined, carriedPair: undefined,
    targeted: undefined, targetedName: undefined,
    trunc: undefined,
    replays: undefined,
    replaySlice: undefined,
    validated: false,
    limit: undefined,
  };
  let i = 0;
  while (i < argv.length) {
    const a = argv[i++];
    switch (a) {
      case '--gen': opts.gen = parseInt(argv[i++], 10); break;
      case '--n': opts.n = parseInt(argv[i++], 10); break;
      case '--seed': opts.seed = parseInt(argv[i++], 10); break;
      case '--teams': opts.teamsMode = argv[i++]; break;
      case '--focus': opts.focus = argv[i++].split(',').filter(Boolean); break;
      case '--self': opts.self = true; break;
      case '--pinned': opts.pinned = true; break;
      case '--pinned-file': opts.pinnedFile = argv[i++]; break;
      case '--quiet': opts.quiet = true; break;
      case '--seed-start': opts.seedStart = parseInt(argv[i++], 10); break;
      case '--seed-end': opts.seedEnd = parseInt(argv[i++], 10); break;
      case '--ps-cli': opts.psCli = argv[i++]; break;
      case '--hits': opts.hits = argv[i++]; break;
      case '--scratch': opts.scratch = argv[i++]; break;
      case '--per-battle-timeout': opts.timeoutMs = parseInt(argv[i++], 10); break;
      case '--jobs': opts.jobs = parseInt(argv[i++], 10); break;
      case '--tamper-line': opts.tamper = parseInt(argv[i++], 10); break;
      case '--allow-unported': opts.allowUnported = true; break;
      case '--carried': {
        const m = /^(?:gen)?([0-9]+)$/.exec(argv[i++] || '');
        if (!m) {
          process.stderr.write('ps-diff: --carried takes a gen (gen1)\n');
          process.exit(2);
        }
        opts.carried = parseInt(m[1], 10);
        break;
      }
      case '--carried-pair': opts.carriedPair = argv[i++]; break;
      case '--targeted': {
        const m = /^(?:gen)?([0-9]+)$/.exec(argv[i++] || '');
        if (!m) {
          process.stderr.write('ps-diff: --targeted takes a gen (gen1)\n');
          process.exit(2);
        }
        opts.targeted = parseInt(m[1], 10);
        break;
      }
      case '--targeted-name': opts.targetedName = argv[i++]; break;
      case '--trunc': opts.trunc = argv[i++]; break;
      case '--slice': opts.slice = argv[i++].split(':').map((x) => parseInt(x, 10)); break;
      case '--replays': opts.replays = argv[i++]; break;
      case '--format': opts.format = argv[i++]; break;
      case '--replay-slice': opts.replaySlice = argv[i++].split(':').map((x) => parseInt(x, 10)); break;
      case '--limit': opts.limit = parseInt(argv[i++], 10); break;
      case '--validated': opts.validated = true; break;
      case '--help': case '-h':
        console.log(
          'Usage: node diff.js (--self|--ps-cli P) [--gen 1|3] [--n N] '
          + '[--seed S] [--seed-start X --seed-end Y] '
          + '[--teams random|wide|file:<p>] [--focus ids] '
          + '[--jobs N] [--tamper-line K] [--pinned] [--carried gen1 [--carried-pair F#C]] '
          + '[--targeted gen1 [--targeted-name N]] '
          + '[--trunc cartridge|math] [--quiet] [--allow-unported] [--hits FILE] '
          + '[--replays DIR [--validated]] [--format id[,id]] [--limit N]'
        );
        process.exit(0);
      default:
        if (a.startsWith('--')) throw new Error(`unknown option ${a}`);
    }
  }
  if (opts.carried !== undefined) {
    if (opts.carried !== 1) {
      process.stderr.write(`ps-diff: no carried corpus for gen ${opts.carried} (only gen1)\n`);
      process.exit(2);
    }
    opts.gen = opts.carried;
    opts.teamsMode = `carried:gen${opts.carried}`;
  }
  if (opts.targeted !== undefined) {
    if (opts.targeted !== 1) {
      process.stderr.write(`ps-diff: no targeted battles for gen ${opts.targeted} (only gen1)\n`);
      process.exit(2);
    }
    opts.gen = opts.targeted;
    opts.teamsMode = `targeted:gen${opts.targeted}`;
  }
  if (![1, 3].includes(opts.gen)) {
    process.stderr.write(`ps-diff: gen must be 1 or 3 (got ${opts.gen})\n`);
    process.exit(2);
  }
  if (opts.trunc !== undefined && !['cartridge', 'math'].includes(opts.trunc)) {
    process.stderr.write(`ps-diff: --trunc is cartridge or math (got ${opts.trunc})\n`);
    process.exit(2);
  }
  if (opts.trunc !== undefined && opts.gen !== 1 && !opts.pinned) {
    process.stderr.write('ps-diff: --trunc is a Gen 1 rule set\n');
    process.exit(2);
  }
  return opts;
}

/// Derive the 4 u16 battle seed, 4 u16 team seed and 4 u16 choice seed
/// from one driver seed (PS's `Gen5RNG` needs all four: a 2-element seed
/// leaves the rest `undefined` and `random()` returns 0 forever). Root seed is `[drv[0..16], (drv >> 16), drv ^
/// 0xa5a5, 0]` -- fits PS's `Gen5RNG` shape (PRNG([u16; 4])) without a
/// re-key. Same driver seed -> identical battle in both sides.
function deriveSeeds(driverSeed) {
  const drv = driverSeed >>> 0;
  const root = new PRNG([
    drv & 0xffff,
    (drv >>> 16) & 0xffff,
    (drv ^ 0xa5a5) & 0xffff,
    0,
  ]);
  const battle = [
    root.random(0x10000),
    root.random(0x10000),
    root.random(0x10000),
    root.random(0x10000),
  ];
  const team = [root.random(0x10000), root.random(0x10000), root.random(0x10000), root.random(0x10000)];
  const choice = [root.random(0x10000), root.random(0x10000), root.random(0x10000), root.random(0x10000)];
  return { battle, team, choice };
}

/// The `>start` formatid. `trunc` (Gen 1 only) is the rule set:
/// `cartridge` (the default) or `math` (`@@@!Cartridge Trunc`).
function formatId(gen, mode, trunc) {
  // Carried and file teams are custom sets: a custom game, like wide.
  const base = mode === 'random' ? `gen${gen}randombattle` : `gen${gen}customgame`;
  return rules.withTrunc(base, gen, trunc || 'cartridge');
}

/// Build the `>start`/`>player pN` envelope for one driver seed.
/// `pair` (a carried team pair) replaces the team builder; the battle and
/// choice seeds still come from the driver seed.
function driverCommands({ gen, mode, driverSeed, focus, self, psCli, pair, trunc }) {
  const { battle, team, choice } = deriveSeeds(driverSeed);
  // The wide/ps-cli empty-list check is hoisted to main() so the gate
  // fails before any battle runs; this is the per-battle path.
  const built = pair || teams.buildPair({
    gen, mode, seed: team, focus, self, psCli,
  });
  const fmt = formatId(gen, mode, trunc);
  const cmds = [];
  cmds.push(`>start ${JSON.stringify({ formatid: fmt, seed: battle })}`);
  cmds.push(`>player p1 ${JSON.stringify({ name: 'p1', team: built.packed[0] })}`);
  cmds.push(`>player p2 ${JSON.stringify({ name: 'p2', team: built.packed[1] })}`);
  return { cmds, built, battleSeed: battle, teamSeed: team, choiceSeed: choice };
}

/// Pick a random legal choice from a request, bit-exactly the same way
/// PS's own random_player_ai would. Reads PS's request JSON by hand; the
/// full typed codec is wired next (P1-6). The choices feed both sides
/// the same byte sequence -- both sides answer the same `>pN ...`.
function pickChoice(req, choicePrng) {
  if (!req || req.wait) return null;
  if (req.forceSwitch) {
    const list = req.forceSwitch;
    const arr = req.side.pokemon;
    const chosen = [];
    const out = [];
    for (let i = 0; i < list.length; i++) {
      if (!list[i]) { out.push('pass'); continue; }
      const slots = [];
      for (let j = 1; j <= 6; j++) {
        const mon = arr[j - 1];
        if (!mon) continue;
        if (mon.active) continue;
        if (chosen.includes(`switch ${j}`)) continue;
        const cond = mon.condition || '';
        if (cond.endsWith(' fnt')) continue;
        if (arr[i].reviving && !cond.endsWith(' fnt')) continue;
        slots.push(j);
      }
      if (!slots.length) out.push('pass');
      else {
        const slot = slots[choicePrng.random(slots.length)];
        chosen.push(`switch ${slot}`);
        out.push(`switch ${slot}`);
      }
    }
    return out.join(', ');
  }
  if (req.teamPreview) return 'default';
  if (req.active) {
    const out = [];
    for (let i = 0; i < req.active.length; i++) {
      const a = req.active[i];
      const monData = req.side.pokemon[i];
      if (monData.condition && monData.condition.endsWith(' fnt')) {
        out.push('pass'); continue;
      }
      const moves = a.moves || [];
      const usable = moves
        .map((m, idx) => ({ slot: idx + 1, move: m }))
        .filter((m) => !m.move.disabled);
      const canSwitch = [];
      for (let j = 1; j <= 6; j++) {
        const mon = req.side.pokemon[j - 1];
        if (!mon) continue;
        if (mon.active) continue;
        if (mon.condition && mon.condition.endsWith(' fnt')) continue;
        canSwitch.push(j);
      }
      if (canSwitch.length && (!usable.length || choicePrng.random() < 0.25) && !a.trapped) {
        const idx = choicePrng.random(canSwitch.length);
        out.push(`switch ${canSwitch[idx]}`);
        continue;
      }
      if (usable.length) {
        const idx = choicePrng.random(usable.length);
        out.push(`move ${usable[idx].slot}`);
      } else {
        out.push('pass');
      }
    }
    return out.join(', ');
  }
  return null;
}

/// Answers every `|request|` of one battle: the next fixed choice of that
/// side's `script` list while there is one (a targeted battle), otherwise a
/// random legal one from the choice PRNG. A `wait` request takes no choice.
/// `script` is a list of `[p1, p2]` rows, `null` for a side that does not
/// choose in that row; each side's choices are used in order.
function makeChooser(choiceSeed, script) {
  const prng = new PRNG(choiceSeed);
  const queues = { p1: [], p2: [] };
  for (const row of script || []) {
    if (row[0] !== null) queues.p1.push(row[0]);
    if (row[1] !== null) queues.p2.push(row[1]);
  }
  return (side, req) => {
    if (!req || req.wait) return null;
    if (queues[side].length) return queues[side].shift();
    return pickChoice(req, prng);
  };
}

/// Drive a single PS BattleStream with the given envelope and random
/// legal choices. Gen 1's truncation is the formatid's `Cartridge Trunc`
/// rule (`rules.js`; the old harness's `cartridgeTrunc` restore in
/// `tools/showdown-parity/harness.js`, as a rule). Returns the joined log
/// and the choice PRNG call count (caller uses this for diagnostics).
async function drivePS({ cmds, choiceSeed, script }) {
  rules.install();
  const stream = new BattleStream({ keepAlive: false });
  const choose = makeChooser(choiceSeed, script);
  let prngCalls = 0;
  stream.write(cmds.join('\n') + '\n');
  const log = [];
  const onRequest = (side, json) => {
    let req;
    try { req = JSON.parse(json); } catch (_) { return; }
    prngCalls += 1;
    const c = choose(side, req);
    if (c !== null) stream.write(`>${side} ${c}\n`);
  };
  while (true) {
    const r = await stream.next();
    if (r.done) break;
    const msg = r.value;
    if (msg === 'end' || msg.startsWith('end\n')) break;
    const nlIdx = msg.indexOf('\n');
    if (nlIdx < 0) continue;
    const type = msg.slice(0, nlIdx);
    const data = msg.slice(nlIdx + 1);
    if (type !== 'update' && type !== 'sideupdate') continue;
    log.push(data);
    // A choice PS rejects stalls the battle (PS waits for a valid one): a
    // targeted battle's fixed choice that no longer fits its request.
    // It is the battle's fault, not PS's: a crash, never `ps_crashes`.
    if (data.includes('|error|[Invalid choice]')) {
      const e = new Error(`PS rejected a choice: ${data.slice(data.indexOf('|error|'))}`);
      e.rejectedChoice = true;
      throw e;
    }
    const lines = data.split('\n');
    let pendingSide = null;
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const m = line.match(/^\|request\|(p[12])\|(.*)$/);
      if (m) {
        onRequest(m[1], m[2]);
        pendingSide = null;
        continue;
      }
      const sideOnly = line.match(/^(p[12])$/);
      if (sideOnly) {
        pendingSide = sideOnly[1];
        continue;
      }
      if (line.startsWith('|request|') && pendingSide) {
        const json = line.slice('|request|'.length);
        onRequest(pendingSide, json);
        pendingSide = null;
        continue;
      }
      if (line.length === 0) pendingSide = null;
    }
    if (data.includes('|win|') || data.includes('|tie|')) {
      try { stream.writeEnd(); } catch (_) {}
    }
  }
  return { out: log.join('\n'), prngCalls };
}

// `normaliseLines` keeps every line PS emits (including the standalone
// `|` separators between phase chunks). The only normalisation is the
// `|t:|<n>` timestamp line, where two PS instances started in different
// seconds would otherwise drift on the same byte sequence.
function normaliseLines(out) {
  return out.split('\n')
    .map((l) => l.replace(/^\|t:\|[0-9]+$/, '|t:||'));
}

function firstDiff(a, b) {
  const n = Math.min(a.length, b.length);
  for (let i = 0; i < n; i++) if (a[i] !== b[i]) return i;
  if (a.length !== b.length) return n;
  return -1;
}

function ensureDir(d) {
  if (!fs.existsSync(d)) fs.mkdirSync(d, { recursive: true });
}

/// A carried battle's log is named after its pair as well as its seed:
/// every pair replays the same 20 seeds.
function logName(rec) {
  if (!rec.pair) return `${rec.seed}`;
  const kind = String(rec.teams).startsWith('targeted') ? 'targeted' : 'carried';
  return `${kind}-${rec.pair.replace(/[^A-Za-z0-9_]+/g, '-')}-${rec.seed}`;
}

function pairNote(rec) {
  return rec.pair ? ` pair ${rec.pair}` : '';
}

function writeDivergence(scratch, rec) {
  ensureDir(scratch);
  const ctxBefore = rec.ctxBefore || [];
  const ctxAfter = rec.ctxAfter || [];
  fs.writeFileSync(path.join(scratch, `ps-diff-${logName(rec)}.log`),
    `seed ${rec.seed} gen ${rec.gen} teams ${rec.teams}${rec.trunc ? ` trunc ${rec.trunc}` : ''}${pairNote(rec)} diff @ line ${rec.at}\n`
    + `prngCallsA=${rec.prngCallsA} prngCallsB=${rec.prngCallsB}\n`
    + `--- context (left: line ${rec.at - ctxBefore.length + 1}..${rec.at + ctxAfter.length}) ---\n`
    + ctxBefore.map((l, i) => `  [${rec.at - ctxBefore.length + 1 + i}] L: ${l}`).join('\n')
    + `\n >> [${rec.at}] L: ${rec.leftAt} | R: ${rec.rightAt} <<\n`
    + ctxAfter.map((l, i) => `  [${rec.at + 1 + i}] L: ${l}`).join('\n')
    + '\n'
    + `--- left ---\n${rec.left}\n`
    + `--- right ---\n${rec.right}\n`);
}

/// A targeted battle names the PS lines it is there to reach (`expect`):
/// if PS's log lacks one, the battle no longer reaches its path (a changed
/// seed, team or PS), which fails the run like a diff.
function expectLines(out, opts, result, scratch, seed) {
  const missing = (opts.expect || []).filter((e) => !out.includes(e));
  if (!missing.length) return;
  result.diff = 1;
  ensureDir(scratch);
  const rec = { seed, teams: opts.teamsMode, pair: opts.pairName };
  fs.writeFileSync(path.join(scratch, `ps-diff-${logName(rec)}-expect.log`),
    `seed ${seed} gen ${opts.gen} teams ${opts.teamsMode}${pairNote(rec)}: PS's log lacks the expected\n`
    + missing.map((e) => `  ${e}\n`).join('') + `--- PS ---\n${out}\n`);
}

function writeError(scratch, rec, err) {
  ensureDir(scratch);
  fs.writeFileSync(path.join(scratch, `ps-diff-${logName(rec)}-crash.log`),
    `seed ${rec.seed} gen ${rec.gen} teams ${rec.teams}${rec.trunc ? ` trunc ${rec.trunc}` : ''}${pairNote(rec)}\n`
    + `${err && err.stack ? err.stack : String(err)}\n`);
}

// The rust side: one long-lived `ps_sim_cli` per (child) process, playing
// one battle after another. It writes what PS's BattleStream pushes, each
// message as its type line, its data and a newline (`update`,
// `sideupdate`, `end`), plus two messages that end a battle early:
// `unported` + the effect id, and `crash` + the panic message. The data of
// every `update` / `sideupdate` goes in the log exactly as drivePS keeps
// PS's, and each `|request|` is answered with the same choice PRNG.
const MESSAGE_TYPES = new Set(['update', 'sideupdate', 'end', 'unported', 'crash']);
const TERMINAL_TYPES = new Set(['end', 'unported', 'crash']);

class RustProc {
  constructor(psCli, hits) {
    this.psCli = psCli;
    this.args = hits ? ['--hits', hits] : [];
    this.child = null;
    this.battle = null;   // the battle being played (see runBattle)
  }
  _spawn() {
    const child = spawn(this.psCli, this.args, { stdio: ['pipe', 'pipe', 'pipe'], env: process.env });
    this.child = child;
    this.type = null;     // the message being read
    this.lines = [];
    this.partial = '';
    child.stdout.on('data', (chunk) => {
      if (this.child !== child) return;
      const text = this.partial + chunk.toString();
      const parts = text.split('\n');
      this.partial = parts.pop();
      for (const line of parts) this._onLine(line);
    });
    child.stderr.on('data', (d) => { if (this.battle) this.battle.stderr += d.toString(); });
    const gone = (why) => {
      if (this.child !== child) return;
      this.child = null;
      if (this.battle) this._finish('crash', why);
    };
    child.on('error', (e) => gone(`spawn: ${e.message}`));
    child.on('exit', (code, sig) => gone(`ps_sim_cli exited (code ${code}, signal ${sig})`));
    child.stdin.on('error', () => {});
  }
  _endMessage() {
    if (this.battle && (this.type === 'update' || this.type === 'sideupdate')) {
      this.battle.log.push(this.lines.join('\n'));
    }
    this.type = null;
    this.lines = [];
  }
  _onLine(line) {
    if (TERMINAL_TYPES.has(this.type)) {
      // The terminal message's one payload line.
      const type = this.type;
      this.type = null;
      this._finish(type, line);
      return;
    }
    if (MESSAGE_TYPES.has(line)) {
      this._endMessage();
      this.type = line;
      return;
    }
    if (this.type === null) return;
    this.lines.push(line);
    const b = this.battle;
    if (!b || this.type !== 'sideupdate' || this.lines.length !== 2) return;
    const side = this.lines[0];
    if (!/^p[12]$/.test(side)) return;
    if (line.startsWith('|error|')) {
      // A rejected choice: PS does not ask again, the player sends the
      // next one. The recording has it.
      if (!b.choices) return;
      const q = b.choices[side];
      if (!q || !q.length) return;
      const retry = q.shift();
      b.sent += 1;
      if (this.child) this.child.stdin.write(`>${side} ${retry}\n`);
      return;
    }
    if (!line.startsWith('|request|')) return;
    let req;
    try { req = JSON.parse(line.slice('|request|'.length)); } catch (_) { return; }
    b.prngCalls += 1;
    if (b.choices) {
      // The replay mode: the choice the recording made, in the recording's
      // order. PS's own grammar (`move submission`) goes through as it
      // stands; `ps_sim` resolves the name like PS does.
      if (req.wait) return;
      // A forced loss ends the battle on its own.
      const due = takeForces(b.forces, b.sent);
      if (due.length) {
        for (const f of due) {
          if (this.child) this.child.stdin.write(`${f.cmd}\n`);
        }
        b.forced = true;
        return;
      }
      if (b.forced) return;
      const q = b.choices[side];
      if (!q || !q.length) {
        // The recording has no choice left for this side. PS's `default`
        // (the first move) keeps the battle going.
        if (this.child) this.child.stdin.write(`>${side} default\n`);
        return;
      }
      const recorded = q.shift();
      b.sent += 1;
      if (this.child) this.child.stdin.write(`>${side} ${recorded}\n`);
      return;
    }
    const choice = b.choose(side, req);
    if (choice !== null && this.child) this.child.stdin.write(`>${side} ${choice}\n`);
  }
  _finish(status, detail) {
    const b = this.battle;
    if (!b) return;
    this._endMessage();
    this.battle = null;
    clearTimeout(b.timer);
    b.resolve({
      out: b.log.join('\n'), prngCalls: b.prngCalls, status, detail, stderr: b.stderr,
    });
  }
  /// Plays one battle: `cmds` are the `>start` / `>player` lines. Resolves
  /// with { out, prngCalls, status: end|unported|crash|timeout, detail }.
  /// `script` (optional) scripts the random choices; `choices` (the
  /// replay mode) is the per-side queue of the recorded choices to play,
  /// and `forces` the recording's `>forcelose` commands.
  runBattle(cmds, choiceSeed, timeoutMs, script, choices, forces) {
    if (!this.child) this._spawn();
    return new Promise((resolve) => {
      this.battle = {
        log: [], prngCalls: 0, choose: makeChooser(choiceSeed, script), stderr: '', resolve,
        choices: choices ? { p1: choices.p1.slice(), p2: choices.p2.slice() } : null,
        forces: forces ? forces.slice() : [],
        sent: 0,
        forced: false,
        timer: setTimeout(() => this.kill('timeout'), timeoutMs),
      };
      this.type = null;
      this.lines = [];
      for (const line of cmds) this.child.stdin.write(line + '\n');
    });
  }
  /// Gives up on the battle being played (the process may be stuck): the
  /// next battle starts a new process.
  kill(status) {
    const child = this.child;
    this.child = null;
    if (child) { try { child.kill('SIGKILL'); } catch (_) {} }
    this._finish(status, `no end within the time limit`);
  }
  /// Waits for the battle at most `ms` more (the PS side is done).
  graceUntil(ms) {
    const b = this.battle;
    if (!b) return;
    clearTimeout(b.timer);
    b.timer = setTimeout(() => this.kill('timeout'), ms);
  }
  close() {
    if (this.child) { try { this.child.stdin.end(); } catch (_) {} }
  }
}

let rustProc = null;
function rustFor(psCli, hits) {
  if (!rustProc || rustProc.psCli !== psCli) {
    if (rustProc) rustProc.close();
    rustProc = new RustProc(psCli, hits);
  }
  return rustProc;
}

// How long the rust side may still take once PS has finished the battle.
const RUST_GRACE_MS = 5000;

// Apply the test hook: --tamper-line K (positive: right log;
// negative: left log in absolute terms). The mutation is in a known
// shape (`|tampered|mark|line=<K>|`) so the diff report names both
// sides and the line is reproducible. Fast on purpose -- the
// `cargo test -p ps_harness --test protocol_diff` tweets one battle,
// pins it to a line, and moves on.
function applyTamper(testLog, k) {
  if (!k) return testLog;
  const lines = testLog.split('\n');
  const idx = Math.abs(k);
  if (idx >= 0 && idx < lines.length) {
    lines[idx] = `${lines[idx]}|tampered|mark|line=${idx}|`;
  }
  return lines.join('\n');
}

async function runOneBattle(driverSeed, opts, scratch) {
  const result = { diff: 0, crash: 0, unported: 0, psCrash: 0 };
  let info;
  try {
    info = driverCommands({
      gen: opts.gen, mode: opts.teamsMode, driverSeed,
      focus: opts.focus, self: opts.self, psCli: opts.psCli,
      pair: opts.pair, trunc: opts.trunc,
    });
  } catch (e) {
    // The empty-ported-list check is hoisted to main() so the gate fails
    // before any battle. A team-builder collision here is a programming
    // error and we want the diff driver to die loudly.
    throw e;
  }
  if (opts.self) {
    try {
      const aPromise = drivePS({ cmds: info.cmds, choiceSeed: info.choiceSeed, script: opts.script });
      const bPromise = drivePS({ cmds: info.cmds, choiceSeed: info.choiceSeed, script: opts.script });
      const [a, b] = await Promise.all([aPromise, bPromise]);
      expectLines(a.out, opts, result, scratch, driverSeed);
      let aLog = applyTamper(a.out, opts.tamper < 0 ? opts.tamper : 0);
      let bLog = applyTamper(b.out, opts.tamper > 0 ? opts.tamper : 0);
      const aLines = normaliseLines(aLog);
      const bLines = normaliseLines(bLog);
      const at = firstDiff(aLines, bLines);
      if (at >= 0) {
        result.diff = 1;
        const ctxBefore = aLines.slice(Math.max(0, at - 3), at);
        const ctxAfter = aLines.slice(at + 1, Math.min(aLines.length, at + 4));
        writeDivergence(scratch, {
          seed: driverSeed, gen: opts.gen, teams: opts.teamsMode, trunc: opts.trunc, pair: opts.pairName, at,
          left: aLog, right: bLog,
          prngCallsA: a.prngCalls, prngCallsB: b.prngCalls,
          leftAt: aLines[at], rightAt: bLines[at],
          ctxBefore, ctxAfter,
        });
      } else if (aLines.length === 0) {
        result.crash = 1;
      }
    } catch (e) {
      // PS itself threw: a PS bug (both sides are PS), not comparable.
      if (e && e.rejectedChoice) result.crash = 1;
      else result.psCrash = 1;
      writeError(scratch, { seed: driverSeed, gen: opts.gen, teams: opts.teamsMode, trunc: opts.trunc, pair: opts.pairName }, e);
    }
    return result;
  }
  // PS on the left, ps_sim_cli on the right.
  let rust = null;
  let bPromise = null;
  try {
    rust = rustFor(opts.psCli, opts.hits);
    bPromise = rust.runBattle(info.cmds, info.choiceSeed, opts.timeoutMs, opts.script);
    const a = await drivePS({ cmds: info.cmds, choiceSeed: info.choiceSeed, script: opts.script });
    expectLines(a.out, opts, result, scratch, driverSeed);
    rust.graceUntil(RUST_GRACE_MS);
    const bRun = await bPromise;
    const aLog = applyTamper(a.out, opts.tamper < 0 ? opts.tamper : 0);
    const bLog = applyTamper(bRun.out, opts.tamper > 0 ? opts.tamper : 0);
    const aLines = normaliseLines(aLog);
    let bLines = normaliseLines(bLog);
    // A battle that stopped early (unported, crash) is compared up to
    // where it stopped: everything it did write must be PS's.
    const stoppedEarly = bRun.status !== 'end';
    if (stoppedEarly && bLog === '') bLines = [];
    const at = stoppedEarly
      ? firstDiff(aLines.slice(0, bLines.length), bLines)
      : firstDiff(aLines, bLines);
    if (at >= 0) {
      result.diff = 1;
      const ctxBefore = aLines.slice(Math.max(0, at - 3), at);
      const ctxAfter = aLines.slice(at + 1, Math.min(aLines.length, at + 4));
      writeDivergence(scratch, {
        seed: driverSeed, gen: opts.gen, teams: opts.teamsMode, trunc: opts.trunc, pair: opts.pairName, at,
        left: aLog, right: bLog,
        prngCallsA: a.prngCalls, prngCallsB: bRun.prngCalls,
        leftAt: aLines[at], rightAt: bLines[at],
        ctxBefore, ctxAfter,
      });
    }
    if (bRun.status === 'unported') {
      result.unported = 1;
      result.unportedId = bRun.detail;
    } else if (bRun.status !== 'end') {
      result.crash = 1;
      writeError(scratch, { seed: driverSeed, gen: opts.gen, teams: opts.teamsMode, trunc: opts.trunc, pair: opts.pairName },
        new Error(`rust side: ${bRun.status}: ${bRun.detail}\n${bRun.stderr}--- right (partial) ---\n${bLog}`));
    }
  } catch (e) {
    // PS itself threw (a PS bug, such as gen 1's `moveSlot is null` after
    // Transform). ps_sim ports PS faithfully, so it must stop with a crash
    // in the same battle: such a battle is not comparable and counts as
    // `ps_crashes`, not as a failure. If ps_sim did not crash, that is a
    // real crash. Either way the rust process must not carry this battle
    // into the next one (graceUntil kills it if it is still playing).
    if (bPromise) {
      rust.graceUntil(RUST_GRACE_MS);
      const bRun = await bPromise;
      if (bRun.status === 'crash' && !(e && e.rejectedChoice)) {
        result.psCrash = 1;
        writeError(scratch, { seed: driverSeed, gen: opts.gen, teams: opts.teamsMode, trunc: opts.trunc, pair: opts.pairName },
          new Error(`PS threw and ps_sim crashed too (not comparable)\nps_sim: ${bRun.detail}\nPS: ${e && e.stack ? e.stack : e}`));
        return result;
      }
      if (rust.child) rust.kill('ps-crash');
    }
    result.crash = 1;
    writeError(scratch, { seed: driverSeed, gen: opts.gen, teams: opts.teamsMode, trunc: opts.trunc, pair: opts.pairName }, e);
  }
  return result;
}

function readPinned(pinnedFile) {
  const file = pinnedFile
    || path.join(__dirname, '..', '..', 'crates', 'ps_harness', 'pinned.txt');
  if (!fs.existsSync(file)) return [];
  return fs.readFileSync(file, 'utf8').split('\n')
    .filter((l) => l.trim() && !l.trim().startsWith('#'))
    .map((line) => {
      const stripped = line.replace(/\s*#.*$/, '').trim();
      const parts = stripped.split(/\s+/);
      // An optional fourth field names the Gen 1 rule set (`math`).
      const entry = { gen: parseInt(parts[0], 10), teams: parts[1], seed: parts[2], trunc: parts[3] };
      if (entry.trunc !== undefined && (entry.gen !== 1 || !['cartridge', 'math'].includes(entry.trunc))) {
        throw new Error(`pinned entry \`${stripped}\`: the fourth field is a Gen 1 \`cartridge\` or \`math\``);
      }
      return entry;
    });
}

/// `--carried gen<N>`: every team pair of the carried corpus, each over
/// battle seeds 0..CARRIED_SEEDS-1 (or the one `--seed`), pair by pair.
/// Every `--jobs` child rebuilds the same list and plays its slice.
function carriedDrivers(opts) {
  const dir = path.join(__dirname, '..', '..', 'crates', 'ps_harness', 'carried', `gen${opts.carried}`);
  let pairs = teams.readCarried(dir);
  if (opts.carriedPair) {
    pairs = pairs.filter((p) => p.name === opts.carriedPair);
    if (!pairs.length) {
      process.stderr.write(`ps-diff: no carried pair ${opts.carriedPair} in ${dir}\n`);
      process.exit(2);
    }
  }
  if (!pairs.length) {
    process.stderr.write(`ps-diff: the carried corpus in ${dir} is empty\n`);
    process.exit(2);
  }
  const seeds = opts.seed !== undefined
    ? [opts.seed]
    : Array.from({ length: CARRIED_SEEDS }, (_, s) => s);
  const drivers = [];
  for (const pair of pairs) {
    for (const s of seeds) {
      drivers.push({ gen: opts.carried, teams: opts.teamsMode, seed: String(s), pair });
    }
  }
  return drivers;
}

/// `--targeted gen<N>`: the targeted battles of
/// `crates/ps_harness/targeted/gen<N>.json`, each at its own driver seed
/// (`--seed S` replaces it), with its fixed choices and the PS lines it
/// must reach. Every `--jobs` child rebuilds the same list.
function targetedDrivers(opts) {
  const file = path.join(__dirname, '..', '..', 'crates', 'ps_harness', 'targeted', `gen${opts.targeted}.json`);
  let list = teams.readTargeted(file);
  if (opts.targetedName) {
    list = list.filter((t) => t.name === opts.targetedName);
    if (!list.length) {
      process.stderr.write(`ps-diff: no targeted battle ${opts.targetedName} in ${file}\n`);
      process.exit(2);
    }
  }
  return list.map((t) => ({
    gen: opts.targeted, teams: opts.teamsMode,
    seed: String(opts.seed !== undefined ? opts.seed : t.seed),
    pair: t, script: t.choices, expect: t.expect, trunc: t.trunc,
  }));
}

// `--jobs N`: spawn N child node processes, each with a slice of the
// driver seed range, and merge their summary lines. The caller passes
// `--jobs 1` in each child so the child runs its slice sequentially.
//
// The child always prints one summary line to stdout, regardless of
// quiet. The parent decides whether to forward children's summaries
// verbatim (not quiet) or to suppress them and merge (quiet).
async function runWithJobs(drivers, opts) {
  const n = Math.max(1, Math.min(opts.jobs, drivers.length));
  const per = Math.ceil(drivers.length / n);
  const slices = [];
  for (let i = 0; i < n; i++) {
    const start = i * per;
    const end = Math.min(start + per, drivers.length);
    if (start >= end) break;
    slices.push({ start, end });
  }
  const childArgs = [];
  childArgs.push('--gen', String(opts.gen));
  if (opts.psCli) childArgs.push('--ps-cli', opts.psCli);
  if (opts.hits) childArgs.push('--hits', opts.hits);
  if (opts.self) childArgs.push('--self');
  if (opts.pinned) childArgs.push('--pinned');
  if (opts.pinnedFile) childArgs.push('--pinned-file', opts.pinnedFile);
  if (opts.carried !== undefined) childArgs.push('--carried', `gen${opts.carried}`);
  if (opts.trunc !== undefined) childArgs.push('--trunc', opts.trunc);
  if (opts.carriedPair) childArgs.push('--carried-pair', opts.carriedPair);
  if (opts.targeted !== undefined) childArgs.push('--targeted', `gen${opts.targeted}`);
  if (opts.targetedName) childArgs.push('--targeted-name', opts.targetedName);
  if ((opts.carried !== undefined || opts.targeted !== undefined) && opts.seed !== undefined) {
    childArgs.push('--seed', String(opts.seed));
  }
  if (opts.focus) childArgs.push('--focus', opts.focus.join(','));
  if (opts.teamsMode !== 'random' && opts.carried === undefined && opts.targeted === undefined) childArgs.push('--teams', opts.teamsMode);
  if (opts.scratch) childArgs.push('--scratch', opts.scratch);
  if (opts.timeoutMs !== DEFAULT_TIMEOUT_MS) childArgs.push('--per-battle-timeout', String(opts.timeoutMs));
  if (opts.allowUnported) childArgs.push('--allow-unported');
  childArgs.push('--seed-start', String(opts.seedStart), '--n', String(opts.n));
  if (opts.seedEnd !== undefined) childArgs.push('--seed-end', String(opts.seedEnd));
  childArgs.push('--jobs', '1');

  // The children run at once (one node process each, each with its own
  // ps_sim_cli); their output is processed in slice order afterwards.
  const runs = slices.map((slice) => new Promise((resolve) => {
    const aargs = childArgs.concat(['--slice', `${slice.start}:${slice.end}`]);
    const child = spawn(process.execPath, [__filename, ...aargs], { stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (d) => { stdout += d; });
    child.stderr.on('data', (d) => { stderr += d; });
    child.on('error', (e) => resolve({ stdout, stderr: stderr + e.message + '\n', status: 2 }));
    child.on('close', (status) => resolve({ stdout, stderr, status }));
  }));
  const results = await Promise.all(runs);
  const summaries = [];
  let exitCode = 0;
  for (const r of results) {
    // Children print one summary line on stdout. Each child gets its own
    // quiet flag inherited from opts (the parent strips it during
    // merging: see below).
    if (r.stdout) {
      // Forward non-summary lines (crash dumps etc.) and parse summary
      // lines for the merger.
      for (const line of r.stdout.split('\n')) {
        if (!line.trim()) continue;
        if (line.startsWith('ps-diff: ran=')) {
          summaries.push(line);
        } else if (!opts.quiet) {
          process.stdout.write(line + '\n');
        }
      }
    }
    if (r.stderr && !opts.quiet) process.stderr.write(r.stderr);
    if (r.status !== 0) exitCode = r.status || 1;
  }
  let totalRun = 0, totalDiff = 0, totalCrash = 0, totalUn = 0, totalPsCrash = 0;
  const ids = new Map();
  for (const line of summaries) {
    const parsed = parseSummary(line);
    if (!parsed) continue;
    totalRun += parsed.ran; totalDiff += parsed.diff;
    totalCrash += parsed.crash; totalUn += parsed.unported; totalPsCrash += parsed.psCrash;
    for (const [id, k] of parsed.unportedIds) ids.set(id, (ids.get(id) || 0) + k);
  }
  const total = `ps-diff: ran=${totalRun} diffs=${totalDiff} crashes=${totalCrash} unported=${totalUn} gen=${opts.gen} teams=${opts.teamsMode} mode=${opts.self ? 'self' : 'ps-vs-rust'} jobs=${n}${truncNote(opts)}`
    + (totalPsCrash ? ` ps_crashes=${totalPsCrash}` : '');
  process.stdout.write(total + '\n');
  if (ids.size) process.stdout.write(`ps-diff: unported ids: ${formatIds(ids)}\n`);
  return exitCode;
}

/// ` trunc=<mode>` on a Gen 1 summary (pinned entries carry their own).
function truncNote(opts) {
  return opts.gen === 1 && !opts.pinned ? ` trunc=${opts.trunc || 'cartridge'}` : '';
}

/// `id:count` pairs, most frequent first.
function formatIds(ids) {
  return [...ids.entries()]
    .sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : 1))
    .map(([id, k]) => `${id}:${k}`)
    .join(',');
}

function parseSummary(line) {
  // Parses "ps-diff: ran=K diffs=K crashes=K unported=K ..." emitted by
  // the child with --quiet. Returns null if the line is not a summary.
  const m = line.match(/^ps-diff\:\s+ran=(\d+)\s+diffs=(\d+)\s+crashes=(\d+)\s+unported=(\d+)/);
  if (!m) return null;
  const ids = line.match(/\sunported_ids=(\S+)/);
  const unportedIds = ids
    ? ids[1].split(',').map((p) => { const k = p.lastIndexOf(':'); return [p.slice(0, k), +p.slice(k + 1)]; })
    : [];
  const pc = line.match(/\sps_crashes=(\d+)/);
  const skipped = line.match(/\sskipped=(\d+)/);
  const reasons = line.match(/\sreasons=(\S*)/);
  const reasonPairs = reasons && reasons[1]
    ? reasons[1].split(',').filter(Boolean).map((p) => {
      const k = p.lastIndexOf(':'); return [p.slice(0, k), +p.slice(k + 1)];
    })
    : [];
  return {
    ran: +m[1], diff: +m[2], crash: +m[3], unported: +m[4], psCrash: pc ? +pc[1] : 0, unportedIds,
    skipped: skipped ? +skipped[1] : 0, reasons: reasonPairs,
  };
}

// ---- the replay corpus -----------------------------------------------------

/// The formats `ps_sim` can play (`ps_protocol::FormatId::ALL`): the
/// base format must be one of these, and every custom rule after `@@@`
/// must be one `ps_protocol::RuleId::ALL` knows, by its name.
const PS_SIM_FORMATS = ['gen1customgame', 'gen3customgame', 'gen1randombattle', 'gen3randombattle'];
const PS_SIM_RULES = [
  'HP Percentage Mod', 'Cancel Mod', 'Desync Clause Mod', 'Sleep Clause Mod',
  'Freeze Clause Mod', 'Endless Battle Clause', 'Species Clause', 'OHKO Clause',
  'Evasion Moves Clause', 'Beat Up Nicknames Mod', 'Switch Priority Clause Mod',
  'Evasion Items Clause', 'Cartridge Trunc',
];

/// Can `ps_sim` play the format a replay names?
function psSimKnowsFormat(formatid) {
  if (typeof formatid !== 'string') return false;
  const [base, rules] = formatid.split('@@@');
  if (!PS_SIM_FORMATS.includes(base)) return false;
  if (rules === undefined) return true;
  return rules.split(',').every((r) => PS_SIM_RULES.includes(r.trim()));
}

/// The pinned PS playing the choices the recording made. Returns the raw
/// update log (the bytes the ps_sim side writes, so the two compare
/// directly), the teams PS generated (packed) and the errors.
///
/// The `>start` line loses its `rated` field: `ps_sim` has no rated flag,
/// and PS's `|rated|` line would be a difference that is not about the
/// battle.
async function drivePSRecorded(p, opts) {
  rules.install();
  const stream = new BattleStream({ keepAlive: false });
  const log = [];
  const errors = [];
  const choices = { p1: p.choices.p1.slice(), p2: p.choices.p2.slice() };
  const forces = (p.forces || []).slice();
  let sent = 0;
  let forced = false;
  const start = { ...p.start };
  delete start.rated;
  const cmds = [`>start ${JSON.stringify(start)}`];
  for (const side of ['p1', 'p2']) {
    const pl = p.players[side];
    if (!pl) continue;
    cmds.push(`>player ${side} ${JSON.stringify(pl)}`);
  }
  const deadline = setTimeout(() => {
    errors.push('timeout');
    try { stream.writeEnd(); } catch (_) { /* already closed */ }
  }, opts.timeoutMs || DEFAULT_TIMEOUT_MS);
  const iter = stream[Symbol.asyncIterator]();
  // The stream does not resolve while the battle waits for a choice, so
  // the pump polls it and stops as soon as it knows the replay cannot go
  // on.
  let stopped = false;
  let pending = null;
  const nextMessage = async () => {
    for (;;) {
      if (stopped) return null;
      if (!pending) pending = iter.next();
      let timer = null;
      const timeout = new Promise((resolve) => { timer = setTimeout(() => resolve(null), 200); });
      const r = await Promise.race([pending, timeout]);
      clearTimeout(timer);
      if (r === null) continue;
      pending = null;
      return r;
    }
  };
  const pump = (async () => {
    for (;;) {
      const r = await nextMessage();
      if (!r || r.done) break;
      const value = r.value;
      const nl = value.indexOf('\n');
      if (nl < 0) continue;
      const type = value.slice(0, nl);
      const data = value.slice(nl + 1);
      if (type === 'update') {
        log.push(data);
        continue;
      }
      if (type === 'sideupdate') {
        // The side updates carry the requests; the rust side writes them
        // the same way, so both logs hold them.
        log.push(data);
        const nl2 = data.indexOf('\n');
        const side = data.slice(0, nl2);
        const body = data.slice(nl2 + 1);
        if (body.startsWith('|request|')) {
          let req;
          try { req = JSON.parse(body.slice('|request|'.length)); } catch (_) { continue; }
          if (req.wait) continue;
          const q = choices[side];
          // A forced loss ends the battle on its own.
          const due = takeForces(forces, sent);
          if (due.length) {
            for (const f of due) stream.write(`${f.cmd}\n`);
            forced = true;
            continue;
          }
          if (forced) continue;
          if (!q || !q.length) {
            errors.push(`${side}: no recorded choice left`);
            // The battle cannot go on: stop the driver instead of waiting
            // for a choice that will never come.
            stopped = true;
            try { stream.writeEnd(); } catch (_) { /* already closed */ }
            return;
          }
          const choice = q.shift();
          sent += 1;
          stream.write(`>${side} ${choice}\n`);
        } else if (body.startsWith('|error|')) {
          errors.push(`${side}: ${body}`);
          // PS does not ask again after a rejected choice: the player
          // sends the next one. The recording has it.
          const q = choices[side];
          if (q && q.length) {
            const choice = q.shift();
            sent += 1;
            stream.write(`>${side} ${choice}\n`);
          } else {
            stopped = true;
            try { stream.writeEnd(); } catch (_) { /* already closed */ }
            return;
          }
        }
        continue;
      }
      if (type === 'end') break;
    }
  })();
  try {
    stream.write(cmds.join('\n') + '\n');
    await pump;
  } catch (e) {
    clearTimeout(deadline);
    return { error: `PS threw: ${e && e.message}` };
  }
  clearTimeout(deadline);
  // The teams PS generated (packed): `ps_sim` has no team generator, so
  // it plays the team PS made. `side.team` is the list of sets; the
  // packed form is what a `>player` line carries.
  const teams = {};
  if (stream.battle) {
    for (const side of stream.battle.sides) {
      if (side) teams[side.id] = Teams.pack(side.team);
    }
  }
  return { log: log.join('\n'), errors, teams };
}

/// The replay files under `dir`, sorted. A format filter keeps the
/// replays of one format (the cache keeps them in a directory per
/// format).
function replayFiles(dir, format) {
  const out = [];
  const walk = (d) => {
    let entries;
    try { entries = fs.readdirSync(d, { withFileTypes: true }); } catch (_) { return; }
    for (const e of entries) {
      const p = path.join(d, e.name);
      if (e.isDirectory()) {
        if (e.name !== 'validated') walk(p);
      } else if (e.name.endsWith('.json') && e.name !== 'state.json') {
        out.push(p);
      }
    }
  };
  walk(dir);
  out.sort();
  if (!format || format === 'all') return out;
  const formats = format.split(',');
  return out.filter((f) => {
    const dirName = path.basename(path.dirname(f));
    const stem = path.basename(f, '.json');
    return formats.some((fmt) => dirName === fmt || stem.startsWith(fmt + '-'));
  });
}

/// `--validated`: only the replays the validator kept
/// (`replays.js validate` writes `validated/<format>.json` next to the
/// format directories). A format without a validation report keeps
/// nothing.
function keepValidated(files) {
  const kept = new Map();
  const keptOf = (formatDir) => {
    if (!kept.has(formatDir)) {
      const report = path.join(path.dirname(formatDir), 'validated', `${path.basename(formatDir)}.json`);
      let ids = new Set();
      try { ids = new Set(JSON.parse(fs.readFileSync(report, 'utf8')).kept); } catch (_) { /* no report */ }
      kept.set(formatDir, ids);
    }
    return kept.get(formatDir);
  };
  return files.filter((f) => keptOf(path.dirname(f)).has(path.basename(f, '.json')));
}

/// The gen a replay's formatid plays.
function genOfFormat(formatid) {
  const m = /^gen([0-9])/.exec(formatid || '');
  return m ? parseInt(m[1], 10) : 1;
}

/// Play one replay on both sides and compare the logs.
async function runReplay(file, opts) {
  const result = { diff: 0, crash: 0, unported: 0, skipped: 0, reason: null, id: null };
  let replay;
  try {
    replay = JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch (e) {
    result.skipped = 1;
    result.reason = 'unreadable';
    return result;
  }
  result.id = replay.id || path.basename(file, '.json');
  const p = parseInputLog(replay.inputlog || '');
  if (!p.start) {
    result.skipped = 1;
    result.reason = 'no-inputlog';
    return result;
  }
  if (!psSimKnowsFormat(p.start.formatid)) {
    result.skipped = 1;
    result.reason = 'unsupported-format';
    return result;
  }
  for (const side of ['p1', 'p2']) {
    for (const c of p.choices[side]) {
      if (/^team\b/.test(c)) {
        // The team preview choice is not in ps_sim's choice grammar yet.
        result.skipped = 1;
        result.reason = 'team-preview';
        return result;
      }
    }
  }
  const ps = await drivePSRecorded(p, opts);
  if (ps.error) {
    result.crash = 1;
    result.reason = 'ps-error';
    result.detail = ps.error;
    return result;
  }
  if (ps.errors.length) {
    result.skipped = 1;
    // A rejected choice means the team PS generated from the seed is not
    // the team the recording played (the random team generator changed);
    // a battle that runs out of recorded choices is the same drift.
    result.reason = ps.errors.some((e) => e.includes('|error|'))
      ? 'team-drift' : 'recording-incomplete';
    result.detail = ps.errors[0];
    return result;
  }
  const start = { ...p.start };
  delete start.rated;
  const cmds = [`>start ${JSON.stringify(start)}`];
  for (const side of ['p1', 'p2']) {
    const pl = p.players[side] || {};
    const team = ps.teams[side];
    if (!team) {
      result.skipped = 1;
      result.reason = 'no-team';
      return result;
    }
    cmds.push(`>player ${side} ${JSON.stringify({
      name: pl.name || side,
      avatar: pl.avatar || '',
      rating: String(pl.rating || ''),
      team,
    })}`);
  }
  const rust = rustFor(opts.psCli);
  const bRun = await rust.runBattle(cmds, [1, 2, 3, 4], opts.timeoutMs, undefined, p.choices, p.forces);
  const aLines = normaliseLines(ps.log);
  let bLines = normaliseLines(bRun.out);
  const stoppedEarly = bRun.status !== 'end';
  if (stoppedEarly && bRun.out === '') bLines = [];
  const at = stoppedEarly
    ? firstDiff(aLines.slice(0, bLines.length), bLines)
    : firstDiff(aLines, bLines);
  // A choice ps_sim rejects is a diff: it answers with an `|error|` line.
  const rejected = bRun.out.includes('|error|');
  if (at >= 0 || rejected) {
    result.diff = 1;
    result.reason = rejected && at < 0 ? 'rejected-choice' : 'log-diff';
    writeDivergence(opts.scratch || path.join(__dirname, '..', '..', '.scratch', 'ps-diff'), {
      seed: result.id, gen: genOfFormat(p.start.formatid),
      teams: 'replay', at: at < 0 ? 0 : at,
      left: ps.log, right: bRun.out,
      prngCallsA: 0, prngCallsB: bRun.prngCalls,
      leftAt: at < 0 ? '' : aLines[at], rightAt: at < 0 ? '' : bLines[at],
      ctxBefore: [], ctxAfter: [],
    });
  }
  if (bRun.status === 'unported') {
    result.unported = 1;
    result.unportedId = bRun.detail;
  } else if (bRun.status !== 'end') {
    result.crash = 1;
    result.reason = result.reason || `rust-${bRun.status}`;
    writeError(opts.scratch || path.join(__dirname, '..', '..', '.scratch', 'ps-diff'),
      { seed: result.id, gen: genOfFormat(p.start.formatid), teams: 'replay' },
      new Error(`rust side: ${bRun.status}: ${bRun.detail}\n${bRun.stderr}`));
  }
  return result;
}

/// The `--replays DIR` mode: the corpus on both sides, one summary line.
async function runReplayMode(opts) {
  const scratch = opts.scratch || path.join(__dirname, '..', '..', '.scratch', 'ps-diff');
  let files = replayFiles(opts.replays, opts.format);
  if (opts.validated) files = keepValidated(files);
  if (opts.limit) files = files.slice(0, opts.limit);
  if (opts.replaySlice) files = files.slice(opts.replaySlice[0], opts.replaySlice[1]);
  if (opts.jobs > 1 && files.length > 1) {
    const code = await runReplaysWithJobs(files, opts);
    process.exit(code);
  }
  let nRun = 0, nDiff = 0, nCrash = 0, nUnported = 0, nSkipped = 0;
  const ids = new Map();
  const reasons = {};
  for (const file of files) {
    const r = await runReplay(file, { ...opts, scratch });
    nRun++;
    nDiff += r.diff; nCrash += r.crash; nUnported += r.unported; nSkipped += r.skipped;
    if (r.unportedId) ids.set(r.unportedId, (ids.get(r.unportedId) || 0) + 1);
    if (r.skipped && r.reason) reasons[r.reason] = (reasons[r.reason] || 0) + 1;
    if (r.diff && r.reason === 'log-diff') reasons['log-diff'] = (reasons['log-diff'] || 0) + 1;
    if (!opts.quiet && nRun % 25 === 0) {
      process.stdout.write(`ps-diff: replays ${nRun}/${files.length} (diffs ${nDiff})\n`);
    }
  }
  if (rustProc) rustProc.close();
  process.stdout.write(
    `ps-diff: ran=${nRun} diffs=${nDiff} crashes=${nCrash} unported=${nUnported} skipped=${nSkipped}`
    + ` mode=replays reasons=${formatReasons(reasons)}`
    + (ids.size ? ` unported_ids=${formatIds(ids)}` : '') + '\n');
  const failures = nDiff + nCrash + (opts.allowUnported ? 0 : nUnported);
  process.exit(failures === 0 ? 0 : 1);
}

/// `reason:count` pairs, most frequent first.
function formatReasons(reasons) {
  return Object.entries(reasons)
    .sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : 1))
    .map(([k, v]) => `${k}:${v}`)
    .join(',');
}

/// `--jobs N` for the replay mode: N child processes running at once,
/// each with a slice of the file list, merged into one summary line.
async function runReplaysWithJobs(files, opts) {
  const n = Math.max(1, Math.min(opts.jobs, files.length));
  const per = Math.ceil(files.length / n);
  const slices = [];
  for (let i = 0; i < n; i++) {
    const start = i * per;
    const end = Math.min(start + per, files.length);
    if (start >= end) break;
    slices.push({ start, end });
  }
  const summaries = [];
  let exitCode = 0;
  const runSlice = (slice) => new Promise((resolve) => {
    const aargs = ['--replays', opts.replays, '--ps-cli', opts.psCli, '--jobs', '1',
      '--replay-slice', `${slice.start}:${slice.end}`];
    if (opts.format && opts.format !== 'all') aargs.push('--format', opts.format);
    if (opts.validated) aargs.push('--validated');
    if (opts.limit) aargs.push('--limit', String(opts.limit));
    if (opts.allowUnported) aargs.push('--allow-unported');
    if (opts.scratch) aargs.push('--scratch', opts.scratch);
    aargs.push('--per-battle-timeout', String(opts.timeoutMs));
    const child = spawn(process.execPath, [__filename, ...aargs], { stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (d) => { stdout += d; });
    child.stderr.on('data', (d) => { stderr += d; });
    child.on('close', (status) => resolve({ stdout, stderr, status }));
  });
  for (const r of await Promise.all(slices.map(runSlice))) {
    if (r.stdout) {
      for (const line of r.stdout.split('\n')) {
        if (!line.trim()) continue;
        if (line.startsWith('ps-diff: ran=')) {
          summaries.push(line);
        } else if (!opts.quiet) {
          process.stdout.write(line + '\n');
        }
      }
    }
    if (r.stderr && !opts.quiet) process.stderr.write(r.stderr);
    if (r.status !== 0) exitCode = r.status || 1;
  }
  let totalRun = 0, totalDiff = 0, totalCrash = 0, totalUn = 0, totalSkipped = 0;
  const ids = new Map();
  const reasons = {};
  for (const line of summaries) {
    const parsed = parseSummary(line);
    if (!parsed) continue;
    totalRun += parsed.ran; totalDiff += parsed.diff;
    totalCrash += parsed.crash; totalUn += parsed.unported;
    totalSkipped += parsed.skipped || 0;
    for (const [id, k] of parsed.unportedIds) ids.set(id, (ids.get(id) || 0) + k);
    for (const [r, k] of parsed.reasons || []) reasons[r] = (reasons[r] || 0) + k;
  }
  process.stdout.write(
    `ps-diff: ran=${totalRun} diffs=${totalDiff} crashes=${totalCrash} unported=${totalUn}`
    + ` skipped=${totalSkipped} mode=replays reasons=${formatReasons(reasons)}`
    + (ids.size ? ` unported_ids=${formatIds(ids)}` : '') + '\n');
  return exitCode;
}

async function main() {
  const opts = parseArgs(process.argv.slice(2));
  if (opts.replays) {
    if (!opts.psCli) {
      process.stderr.write('error: --replays needs --ps-cli <path>\n');
      process.exit(2);
    }
    if (!fs.existsSync(opts.psCli)) {
      process.stderr.write(`ps-diff: ps_cli not found at ${opts.psCli}\n`);
      process.exit(2);
    }
    await runReplayMode(opts);
    return;
  }
  if ((opts.pinned || opts.carried !== undefined || opts.targeted !== undefined) && !opts.psCli) opts.self = true;
  if ([opts.pinned || undefined, opts.carried, opts.targeted].filter((x) => x !== undefined).length > 1) {
    process.stderr.write('ps-diff: --pinned, --carried and --targeted are separate runs\n');
    process.exit(2);
  }
  if (!opts.psCli && !opts.self) {
    process.stderr.write('error: --self or --ps-cli <path> is required\n');
    process.exit(2);
  }
  if (opts.psCli && !fs.existsSync(opts.psCli)) {
    process.stderr.write(`ps-diff: ps_cli not found at ${opts.psCli}\n`);
    process.exit(2);
  }
  if (opts.teamsMode === 'wide' && !opts.self) {
    // Hoist: a known-empty ported list is a fast exit 2 before any
    // battle runs, so check.sh fails the gate cleanly.
    const ported = spawnPorted.getPool({ gen: opts.gen, self: false, psCli: opts.psCli });
    if (ported.empty) {
      process.stderr.write(`no ported pipeline for gen ${opts.gen}\n`);
      process.exit(2);
    }
  }
  const scratch = opts.scratch
    || path.join(__dirname, '..', '..', '.scratch', 'ps-diff');

  let drivers;
  if (opts.pinned) {
    drivers = readPinned(opts.pinnedFile);
  } else if (opts.carried !== undefined) {
    drivers = carriedDrivers(opts);
  } else if (opts.targeted !== undefined) {
    drivers = targetedDrivers(opts);
  } else if (opts.seed !== undefined) {
    drivers = [{ gen: opts.gen, teams: opts.teamsMode, seed: String(opts.seed) }];
  } else if (opts.seedEnd !== undefined) {
    drivers = [];
    for (let s = opts.seedStart; s < opts.seedEnd; s++) {
      drivers.push({ gen: opts.gen, teams: opts.teamsMode, seed: String(s) });
    }
  } else {
    drivers = [];
    for (let s = opts.seedStart; s < opts.seedStart + opts.n; s++) {
      drivers.push({ gen: opts.gen, teams: opts.teamsMode, seed: String(s) });
    }
  }

  // If pinned with no entries, emit one summary line and exit 0. (A zero-
  // entry pinned.txt is the SMILIES.md of "the regression list is empty
  // today; tick every check.sh step anyway".)
  if (opts.pinned && drivers.length === 0) {
    if (!opts.quiet) {
      process.stdout.write(`ps-diff: pinned list empty; nothing to run (gen=${opts.gen} teams=${opts.teamsMode})\n`);
    }
    process.stdout.write(`ps-diff: ran=0 diffs=0 crashes=0 unported=0 gen=${opts.gen} teams=${opts.teamsMode} mode=pinned\n`);
    process.exit(0);
  }

  if (opts.slice) drivers = drivers.slice(opts.slice[0], opts.slice[1]);

  if (opts.jobs > 1 && drivers.length > 1
    && (opts.seed === undefined || opts.carried !== undefined || opts.targeted !== undefined)) {
    // Forward to the child-process distributer.
    const code = await runWithJobs(drivers, opts);
    process.exit(code);
  }

  let nRun = 0, nDiff = 0, nCrash = 0, nUnported = 0, nPsCrash = 0;
  const ids = new Map();
  for (const entry of drivers) {
    const seedInt = parseInt(entry.seed, 10) >>> 0;
    const battleOpts = {
      ...opts, gen: entry.gen, teamsMode: entry.teams,
      pair: entry.pair, pairName: entry.pair && entry.pair.name,
      script: entry.script, expect: entry.expect,
      trunc: entry.trunc || (entry.gen === 1 ? opts.trunc : undefined),
    };
    const r = await runOneBattle(seedInt, battleOpts, scratch);
    nRun++; nDiff += r.diff; nCrash += r.crash; nUnported += r.unported; nPsCrash += r.psCrash;
    if (r.unportedId) ids.set(r.unportedId, (ids.get(r.unportedId) || 0) + 1);
  }
  if (rustProc) rustProc.close();
  // Always one summary line: the --jobs parent merges them.
  process.stdout.write(
    `ps-diff: ran=${nRun} diffs=${nDiff} crashes=${nCrash} unported=${nUnported}`
    + ` gen=${opts.gen} teams=${opts.teamsMode} mode=${opts.self ? 'self' : 'ps-vs-rust'}${truncNote(opts)}`
    + (nPsCrash ? ` ps_crashes=${nPsCrash}` : '')
    + (ids.size ? ` unported_ids=${formatIds(ids)}` : '') + '\n');
  const failures = nDiff + nCrash + (opts.allowUnported ? 0 : nUnported);
  process.exit(failures === 0 ? 0 : 1);
}

if (require.main === module) {
  main().catch((e) => {
    process.stderr.write(e && e.stack ? e.stack : e);
    process.stderr.write('\n');
    process.exit(2);
  });
}

module.exports = {
  parseArgs, deriveSeeds, driverCommands, pickChoice,
  normaliseLines, firstDiff, runOneBattle, drivePS,
};
