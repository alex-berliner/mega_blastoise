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
// Battle loop:
//   - `--teams random` -> PS's own genNrandombattle generator.
//   - `--teams wide`   -> the showdown_fuzz wide fuzz space; limited to
//                         the species/moves/items/abilities list from
//                         `ps_sim_cli --ported <gen>` when run against
//                         the rust side, otherwise the whole gen.
//   - `--teams file:<path>` -> two packed teams from a file.
//
// Exit codes: 0 green; 1 diff / crash / unported hit (unported does not
// count with --allow-unported); 2 misuse or empty
// `wide --ps-cli`.
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
//     among its moves. Used by the cargo test for replay.
//   * `--quiet` collapses the per-run trace to one summary line at the
//     end. Without it, the diff's seed, line number, left/right context
//     and PRNG call counts are written, plus we print per-battle progress
//     milestones for long runs.

const fs = require('fs');
const path = require('path');
const { spawn, spawnSync } = require('child_process');
const { BattleStream, PRNG } = require('pokemon-showdown');
const teams = require('./teams');
const { spawnPorted } = require('./ported');

const DEFAULT_TIMEOUT_MS = 30000;
const DEFAULT_JOBS = 14;

function parseArgs(argv) {
  const opts = {
    gen: 3, n: 200,
    seed: undefined,
    teamsMode: 'random',
    focus: null,
    self: false,
    pinned: false, pinnedFile: undefined,
    quiet: false,
    seedStart: 0, seedEnd: undefined,
    psCli: undefined,
    scratch: undefined,
    timeoutMs: DEFAULT_TIMEOUT_MS,
    jobs: DEFAULT_JOBS,
    tamper: undefined,
    allowUnported: false,
    slice: undefined,
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
      case '--scratch': opts.scratch = argv[i++]; break;
      case '--per-battle-timeout': opts.timeoutMs = parseInt(argv[i++], 10); break;
      case '--jobs': opts.jobs = parseInt(argv[i++], 10); break;
      case '--tamper-line': opts.tamper = parseInt(argv[i++], 10); break;
      case '--allow-unported': opts.allowUnported = true; break;
      case '--slice': opts.slice = argv[i++].split(':').map((x) => parseInt(x, 10)); break;
      case '--help': case '-h':
        console.log(
          'Usage: node diff.js (--self|--ps-cli P) [--gen 1|3] [--n N] '
          + '[--seed S] [--seed-start X --seed-end Y] '
          + '[--teams random|wide|file:<p>] [--focus ids] '
          + '[--jobs N] [--tamper-line K] [--pinned] [--quiet] [--allow-unported]'
        );
        process.exit(0);
      default:
        if (a.startsWith('--')) throw new Error(`unknown option ${a}`);
    }
  }
  if (![1, 3].includes(opts.gen)) {
    process.stderr.write(`ps-diff: gen must be 1 or 3 (got ${opts.gen})\n`);
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

function formatId(gen, mode) {
  if (mode === 'random') return `gen${gen}randombattle`;
  return `gen${gen}customgame`;
}

/// Build the `>start`/`>player pN` envelope for one driver seed.
function driverCommands({ gen, mode, driverSeed, focus, self, psCli }) {
  const { battle, team, choice } = deriveSeeds(driverSeed);
  // The wide/ps-cli empty-list check is hoisted to main() so the gate
  // fails before any battle runs; this is the per-battle path.
  const built = teams.buildPair({
    gen, mode, seed: team, focus, self, psCli,
  });
  const fmt = formatId(gen, mode);
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

/// Cartridge truncation, restored for Gen 1 (one PS-side adaptation).
/// See `tools/showdown-parity/harness.js:cartridgeTrunc`.
function applyCartridgeTrunc(battle) {
  battle.trunc = (num, bits = 0) => (bits ? (num >>> 0) % (2 ** bits) : num >>> 0);
}

/// Drive a single PS BattleStream with the given envelope, random legal
/// choices, and cartridge trunc for gen 1. Returns the joined log and
/// the choice PRNG call count (caller uses this for diagnostics).
async function drivePS({ gen, cmds, choiceSeed, doCartridge }) {
  const stream = new BattleStream({ keepAlive: false });
  const prng = new PRNG(choiceSeed);
  let prngCalls = 0;
  if (doCartridge) {
    const origWriteLines = stream._writeLines.bind(stream);
    stream._writeLines = (chunk) => {
      const r = origWriteLines(chunk);
      if (stream.battle) applyCartridgeTrunc(stream.battle);
      return r;
    };
  }
  stream.write(cmds.join('\n') + '\n');
  const log = [];
  const onRequest = (side, json) => {
    let req;
    try { req = JSON.parse(json); } catch (_) { return; }
    prngCalls += 1;
    const c = pickChoice(req, prng);
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

function writeDivergence(scratch, rec) {
  ensureDir(scratch);
  const ctxBefore = rec.ctxBefore || [];
  const ctxAfter = rec.ctxAfter || [];
  fs.writeFileSync(path.join(scratch, `ps-diff-${rec.seed}.log`),
    `seed ${rec.seed} gen ${rec.gen} teams ${rec.teams} diff @ line ${rec.at}\n`
    + `prngCallsA=${rec.prngCallsA} prngCallsB=${rec.prngCallsB}\n`
    + `--- context (left: line ${rec.at - ctxBefore.length + 1}..${rec.at + ctxAfter.length}) ---\n`
    + ctxBefore.map((l, i) => `  [${rec.at - ctxBefore.length + 1 + i}] L: ${l}`).join('\n')
    + `\n >> [${rec.at}] L: ${rec.leftAt} | R: ${rec.rightAt} <<\n`
    + ctxAfter.map((l, i) => `  [${rec.at + 1 + i}] L: ${l}`).join('\n')
    + '\n'
    + `--- left ---\n${rec.left}\n`
    + `--- right ---\n${rec.right}\n`);
}

function writeError(scratch, rec, err) {
  ensureDir(scratch);
  fs.writeFileSync(path.join(scratch, `ps-diff-${rec.seed}-crash.log`),
    `seed ${rec.seed} gen ${rec.gen} teams ${rec.teams}\n`
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
  constructor(psCli) {
    this.psCli = psCli;
    this.child = null;
    this.battle = null;   // the battle being played (see runBattle)
  }
  _spawn() {
    const child = spawn(this.psCli, [], { stdio: ['pipe', 'pipe', 'pipe'], env: process.env });
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
    if (!/^p[12]$/.test(side) || !line.startsWith('|request|')) return;
    let req;
    try { req = JSON.parse(line.slice('|request|'.length)); } catch (_) { return; }
    b.prngCalls += 1;
    const choice = pickChoice(req, b.prng);
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
  runBattle(cmds, choiceSeed, timeoutMs) {
    if (!this.child) this._spawn();
    return new Promise((resolve) => {
      this.battle = {
        log: [], prngCalls: 0, prng: new PRNG(choiceSeed), stderr: '', resolve,
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
function rustFor(psCli) {
  if (!rustProc || rustProc.psCli !== psCli) {
    if (rustProc) rustProc.close();
    rustProc = new RustProc(psCli);
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
  const result = { diff: 0, crash: 0, unported: 0 };
  let info;
  try {
    info = driverCommands({
      gen: opts.gen, mode: opts.teamsMode, driverSeed,
      focus: opts.focus, self: opts.self, psCli: opts.psCli,
    });
  } catch (e) {
    // The empty-ported-list check is hoisted to main() so the gate fails
    // before any battle. A team-builder collision here is a programming
    // error and we want the diff driver to die loudly.
    throw e;
  }
  const doCartridge = opts.gen === 1;
  if (opts.self) {
    try {
      const aPromise = drivePS({ gen: opts.gen, cmds: info.cmds, choiceSeed: info.choiceSeed, doCartridge });
      const bPromise = drivePS({ gen: opts.gen, cmds: info.cmds, choiceSeed: info.choiceSeed, doCartridge });
      const [a, b] = await Promise.all([aPromise, bPromise]);
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
          seed: driverSeed, gen: opts.gen, teams: opts.teamsMode, at,
          left: aLog, right: bLog,
          prngCallsA: a.prngCalls, prngCallsB: b.prngCalls,
          leftAt: aLines[at], rightAt: bLines[at],
          ctxBefore, ctxAfter,
        });
      } else if (aLines.length === 0) {
        result.crash = 1;
      }
    } catch (e) {
      result.crash = 1;
      writeError(scratch, { seed: driverSeed, gen: opts.gen, teams: opts.teamsMode }, e);
    }
    return result;
  }
  // PS on the left, ps_sim_cli on the right.
  try {
    const rust = rustFor(opts.psCli);
    const bPromise = rust.runBattle(info.cmds, info.choiceSeed, opts.timeoutMs);
    const a = await drivePS({ gen: opts.gen, cmds: info.cmds, choiceSeed: info.choiceSeed, doCartridge });
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
        seed: driverSeed, gen: opts.gen, teams: opts.teamsMode, at,
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
      writeError(scratch, { seed: driverSeed, gen: opts.gen, teams: opts.teamsMode },
        new Error(`rust side: ${bRun.status}: ${bRun.detail}\n${bRun.stderr}--- right (partial) ---\n${bLog}`));
    }
  } catch (e) {
    result.crash = 1;
    writeError(scratch, { seed: driverSeed, gen: opts.gen, teams: opts.teamsMode }, e);
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
      return { gen: parseInt(parts[0], 10), teams: parts[1], seed: parts[2] };
    });
}

// `--jobs N`: spawn N child node processes, each with a slice of the
// driver seed range, and merge their summary lines. The caller passes
// `--jobs 1` in each child so the child runs its slice sequentially.
//
// The child always prints one summary line to stdout, regardless of
// quiet. The parent decides whether to forward children's summaries
// verbatim (not quiet) or to suppress them and merge (quiet).
function runWithJobs(drivers, opts) {
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
  if (opts.self) childArgs.push('--self');
  if (opts.pinned) childArgs.push('--pinned');
  if (opts.pinnedFile) childArgs.push('--pinned-file', opts.pinnedFile);
  if (opts.focus) childArgs.push('--focus', opts.focus.join(','));
  if (opts.teamsMode !== 'random') childArgs.push('--teams', opts.teamsMode);
  if (opts.scratch) childArgs.push('--scratch', opts.scratch);
  if (opts.timeoutMs !== DEFAULT_TIMEOUT_MS) childArgs.push('--per-battle-timeout', String(opts.timeoutMs));
  if (opts.allowUnported) childArgs.push('--allow-unported');
  childArgs.push('--seed-start', String(opts.seedStart), '--n', String(opts.n));
  if (opts.seedEnd !== undefined) childArgs.push('--seed-end', String(opts.seedEnd));
  childArgs.push('--jobs', '1');

  const summaries = [];
  let exitCode = 0;
  for (const slice of slices) {
    // Each child builds the same driver list and plays its slice of it.
    const aargs = childArgs.concat(['--slice', `${slice.start}:${slice.end}`]);
    // Children print one summary line on stdout. Each child gets its own
    // quiet flag inherited from opts (the parent strips it during
    // merging: see below).
    const r = spawnSync(process.execPath, [__filename, ...aargs], { encoding: 'utf8' });
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
  let totalRun = 0, totalDiff = 0, totalCrash = 0, totalUn = 0;
  const ids = new Map();
  for (const line of summaries) {
    const parsed = parseSummary(line);
    if (!parsed) continue;
    totalRun += parsed.ran; totalDiff += parsed.diff;
    totalCrash += parsed.crash; totalUn += parsed.unported;
    for (const [id, k] of parsed.unportedIds) ids.set(id, (ids.get(id) || 0) + k);
  }
  const total = `ps-diff: ran=${totalRun} diffs=${totalDiff} crashes=${totalCrash} unported=${totalUn} gen=${opts.gen} teams=${opts.teamsMode} mode=${opts.self ? 'self' : 'ps-vs-rust'} jobs=${n}`;
  process.stdout.write(total + '\n');
  if (ids.size) process.stdout.write(`ps-diff: unported ids: ${formatIds(ids)}\n`);
  return exitCode;
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
  return { ran: +m[1], diff: +m[2], crash: +m[3], unported: +m[4], unportedIds };
}

async function main() {
  const opts = parseArgs(process.argv.slice(2));
  if (opts.pinned && !opts.psCli) opts.self = true;
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

  if (opts.jobs > 1 && drivers.length > 1 && opts.seed === undefined) {
    // Forward to the child-process distributer.
    const code = runWithJobs(drivers, opts);
    process.exit(code);
  }

  let nRun = 0, nDiff = 0, nCrash = 0, nUnported = 0;
  const ids = new Map();
  for (const entry of drivers) {
    const seedInt = parseInt(entry.seed, 10) >>> 0;
    const battleOpts = { ...opts, gen: entry.gen, teamsMode: entry.teams };
    const r = await runOneBattle(seedInt, battleOpts, scratch);
    nRun++; nDiff += r.diff; nCrash += r.crash; nUnported += r.unported;
    if (r.unportedId) ids.set(r.unportedId, (ids.get(r.unportedId) || 0) + 1);
  }
  if (rustProc) rustProc.close();
  // Always one summary line: the --jobs parent merges them.
  process.stdout.write(
    `ps-diff: ran=${nRun} diffs=${nDiff} crashes=${nCrash} unported=${nUnported}`
    + ` gen=${opts.gen} teams=${opts.teamsMode} mode=${opts.self ? 'self' : 'ps-vs-rust'}`
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
  normaliseLines, firstDiff, runOneBattle,
};
