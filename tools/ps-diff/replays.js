#!/usr/bin/env node
'use strict';
// tools/ps-diff/replays.js
//
// The replay corpus: fetch public replays from replay.pokemonshowdown.com,
// validate them against the pinned PS, and report what was kept and what
// was dropped.
//
//   node tools/ps-diff/replays.js fetch    --cache DIR --format <id> --target N [--era gen5|current]
//   node tools/ps-diff/replays.js validate --cache DIR [--format <id>] [--jobs N] [--era gen5|current]
//   node tools/ps-diff/replays.js report   --cache DIR [--era gen5|current]
//
// Sub-commands
//
//   fetch     Walk the replay server's search API backwards in time and
//             download every replay it finds until the format has
//             `--target` replays on disk.
//
//   validate  Run every cached replay's `inputlog` through the pinned PS
//             and keep the replay only when PS's spectator output matches
//             the replay's stored `log` (the server-only lines are ignored).
//             Writes `validated/<format>.json` and prints the kept and
//             dropped counts per format with the drop reasons.
//
//   report    Print the validation report (kept, dropped, reasons) per
//             format from the cache, without re-running anything.
//
// The two eras (the seed decides which PRNG the battle runs on, and the
// PRNG decides whether a battle reproduces at all):
//
//   gen5      the numeric-seed era (PS's `Gen5RNG`). The cache lives in
//             `<cache>/<format>/`.
//
//   current   the `sodium,<hex>` era (PS's `SodiumRNG`, ChaCha20). The
//             cache lives in `<cache>/current/<format>/`.
//
// Politeness (the replay server is a free service):
//   * at most one request per second, across all formats;
//   * a descriptive User-Agent;
//   * exponential backoff on 429 and 5xx;
//   * nothing is ever downloaded twice (the cache is checked first);
//   * the fetch is resumable: the search cursor and the downloaded set
//     live on disk, so a killed run continues where it stopped.
//
// Seeds: a replay's battle seed (its `>start` line) and its team seeds
// (its `>player` lines) decide the era. `gen5` keeps the numeric ones,
// `current` keeps the `sodium,` ones. A replay whose seeds are neither
// (or that has no input log at all) is not kept in either era.
//
// Input logs: the PS server stores an input log (the `>` commands) only
// for battles with a generated team (the random battle formats). The OU
// formats' replays have none: their log is kept, but they cannot be
// replayed, so they are not validated and not diffed.

const fs = require('fs');
const path = require('path');
const https = require('https');

const PS_ROOT = path.join(__dirname, 'node_modules', 'pokemon-showdown');
const { BattleStream } = require(PS_ROOT);
const { extractChannelMessages } = require(path.join(PS_ROOT, 'dist', 'sim', 'battle.js'));

const FORMATS = ['gen1randombattle', 'gen1ou', 'gen3randombattle', 'gen3ou'];
const USER_AGENT = 'mega_blastoise-ps-diff (github.com/alex-berliner/mega_blastoise)';
const REPLAY_BASE = 'https://replay.pokemonshowdown.com';
const DEFAULT_CACHE = '/root/replay-cache';
// The Gen5RNG era ends in late 2024: the replays uploaded after that use
// the `sodium,<hex>` seed, which is a different PRNG.
const DEFAULT_BEFORE = 1734000000;
// The floor of the current era: the last numeric-seed replay of the
// `gen1randombattle` corpus is from December 2024, the first sodium one
// from January 2025. The fetch stops when the cursor goes below it.
const CURRENT_FLOOR = 1735000000;
// One request per second, at most.
const MIN_INTERVAL_MS = 1000;

/// The two eras of the corpus. `dir` is the subdirectory of the cache
/// the era's replays live in, `seed` the seed kind its input logs carry
/// (`seedEra`), `before` the timestamp the search starts at (null: the
/// newest replay the server has) and `floor` the timestamp it stops at.
const ERAS = {
  gen5: { dir: '', seed: 'gen5', before: DEFAULT_BEFORE, floor: 0 },
  current: { dir: 'current', seed: 'sodium', before: null, floor: CURRENT_FLOOR },
};

/// The era whose input logs carry seeds of the kind `seed`, or null.
function eraOfSeed(seed) {
  for (const [name, e] of Object.entries(ERAS)) if (e.seed === seed) return name;
  return null;
}

/// The shared rate limiter. One request at a time, at least
/// MIN_INTERVAL_MS apart, with a promise chain so concurrent callers
/// queue up.
class RateLimiter {
  constructor(minIntervalMs) {
    this.minIntervalMs = minIntervalMs;
    this.last = 0;
    this.chain = Promise.resolve();
  }

  wait() {
    const next = this.chain.then(async () => {
      const now = Date.now();
      const due = this.last + this.minIntervalMs;
      if (due > now) await sleep(due - now);
      this.last = Date.now();
    });
    // Keep the chain alive even if one request fails.
    this.chain = next.catch(() => {});
    return next;
  }
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/// One HTTP GET with the shared rate limit and the backoff policy.
/// Resolves with { status, body }.
async function get(url, limiter, { retries = 5 } = {}) {
  let attempt = 0;
  for (;;) {
    await limiter.wait();
    let res;
    try {
      res = await rawGet(url);
    } catch (e) {
      attempt++;
      if (attempt > retries) throw new Error(`GET ${url} failed: ${e.message}`);
      await sleep(Math.min(60000, 1000 * Math.pow(2, attempt)));
      continue;
    }
    if (res.status === 200) return res;
    if (res.status === 429 || res.status >= 500) {
      attempt++;
      if (attempt > retries) {
        throw new Error(`GET ${url} gave ${res.status} after ${retries} retries`);
      }
      const wait = Math.min(60000, 2000 * Math.pow(2, attempt - 1));
      process.stderr.write(`replays: ${res.status} on ${url}; waiting ${wait} ms\n`);
      await sleep(wait);
      continue;
    }
    // 404 and friends: the caller decides what to do.
    return res;
  }
}

function rawGet(url) {
  return new Promise((resolve, reject) => {
    const req = https.get(url, {
      headers: {
        'User-Agent': USER_AGENT,
        'Accept': 'application/json',
      },
    }, (res) => {
      const chunks = [];
      res.on('data', (c) => chunks.push(c));
      res.on('end', () => {
        resolve({
          status: res.statusCode,
          body: Buffer.concat(chunks).toString('utf8'),
        });
      });
    });
    req.on('error', reject);
    req.setTimeout(30000, () => {
      req.destroy(new Error('timeout'));
    });
  });
}

function ensureDir(d) {
  fs.mkdirSync(d, { recursive: true });
}

/// The directory a format's replays of an era live in.
function formatDir(cache, format, era) {
  const e = ERAS[era] || ERAS.gen5;
  return path.join(cache, e.dir, format);
}

function statePath(cache, format, era) {
  return path.join(formatDir(cache, format, era), 'state.json');
}

function loadState(cache, format, era) {
  const p = statePath(cache, format, era);
  const e = ERAS[era] || ERAS.gen5;
  const fresh = {
    cursor: e.before, done: false, downloaded: 0, skipped: 0, otherEra: 0, noInputLog: 0, unknownSeed: [],
  };
  if (!fs.existsSync(p)) return fresh;
  try {
    const s = JSON.parse(fs.readFileSync(p, 'utf8'));
    return {
      cursor: typeof s.cursor === 'number' ? s.cursor : e.before,
      done: !!s.done,
      downloaded: s.downloaded || 0,
      skipped: s.skipped || 0,
      otherEra: s.otherEra || 0,
      noInputLog: s.noInputLog || 0,
      unknownSeed: Array.isArray(s.unknownSeed) ? s.unknownSeed : [],
    };
  } catch (_) {
    return fresh;
  }
}

function saveState(cache, format, era, state) {
  ensureDir(formatDir(cache, format, era));
  fs.writeFileSync(statePath(cache, format, era), JSON.stringify(state, null, 1) + '\n');
}

/// How many replays of a format are already on disk.
function cachedCount(cache, format, era) {
  const dir = formatDir(cache, format, era);
  if (!fs.existsSync(dir)) return 0;
  let n = 0;
  for (const f of fs.readdirSync(dir)) {
    if (f.endsWith('.json') && f !== 'state.json') n++;
  }
  return n;
}

/// The battle seed of a replay's `>start` line, or null when there is
/// none.
function battleSeedOf(inputlog) {
  for (const raw of inputlog.split('\n')) {
    const line = raw.trim();
    if (!line.startsWith('>start')) continue;
    try {
      const opts = JSON.parse(line.slice('>start'.length));
      return opts.seed;
    } catch (_) {
      return null;
    }
  }
  return null;
}

/// The team seeds of a replay's `>player` lines (null when the player
/// carries a packed team instead).
function playerSeedsOf(inputlog) {
  const out = [];
  for (const raw of inputlog.split('\n')) {
    const line = raw.trim();
    if (!line.startsWith('>player')) continue;
    try {
      const opts = JSON.parse(line.slice('>player'.length).trim().replace(/^p[12]\s+/, ''));
      out.push(opts.seed === undefined ? null : opts.seed);
    } catch (_) {
      out.push(null);
    }
  }
  return out;
}

/// Which era a seed belongs to: `gen5` (four plain numbers), `sodium`
/// (a `sodium,<hex>` string) or `unknown`.
function seedEra(seed) {
  if (typeof seed === 'string') {
    return seed.startsWith('sodium,') ? 'sodium' : 'unknown';
  }
  if (Array.isArray(seed) && seed.length === 4) {
    return seed.every((n) => typeof n === 'number') ? 'gen5' : 'unknown';
  }
  return 'unknown';
}

/// The era of a replay's `inputlog`: the era of its battle seed when the
/// team seeds agree with it, `unknown` otherwise.
function inputlogEra(inputlog) {
  const battle = seedEra(battleSeedOf(inputlog));
  if (battle === 'unknown') return 'unknown';
  for (const s of playerSeedsOf(inputlog)) {
    if (s === null) continue;
    if (seedEra(s) !== battle) return 'unknown';
  }
  return battle;
}

/// A seed is usable when it is an array of plain numbers (the Gen5RNG
/// shape) or when the player has a packed team (no seed at all).
function seedsAreNumeric(inputlog) {
  return inputlogEra(inputlog) === 'gen5';
}

/// The fetch sub-command.
async function cmdFetch(opts) {
  const limiter = new RateLimiter(MIN_INTERVAL_MS);
  const formats = opts.format && opts.format !== 'all' ? opts.format.split(',') : FORMATS;
  const eras = opts.era === 'all' ? ['gen5', 'current'] : [opts.era];
  const started = Date.now();
  for (const era of eras) {
    for (const format of formats) {
      if (!FORMATS.includes(format)) {
        process.stderr.write(`replays: unknown format ${format}\n`);
        process.exit(2);
      }
      await fetchOne({ limiter, opts, era, format, started });
    }
  }
}

/// The fetch of one format of one era.
async function fetchOne({ limiter, opts, era, format, started }) {
  const e = ERAS[era];
  const state = loadState(opts.cache, format, era);
  if (state.done && cachedCount(opts.cache, format, era) >= opts.target) {
    process.stdout.write(`replays: ${era} ${format} already has ${cachedCount(opts.cache, format, era)} replays\n`);
    return;
  }
  ensureDir(formatDir(opts.cache, format, era));
  let have = cachedCount(opts.cache, format, era);
  let rounds = 0;
  while (have < opts.target && !state.done) {
    if (e.floor && state.cursor !== null && state.cursor < e.floor) {
      // The era is over: the walk reached the other era's replays.
      state.done = true;
      break;
    }
    const url = state.cursor === null
      ? `${REPLAY_BASE}/search.json?format=${format}`
      : `${REPLAY_BASE}/search.json?format=${format}&before=${state.cursor}`;
    const res = await get(url, limiter);
    if (res.status !== 200) {
      process.stderr.write(`replays: search for ${format} gave ${res.status}; stopping this format\n`);
      break;
    }
    let entries;
    try {
      entries = JSON.parse(res.body);
    } catch (_) {
      process.stderr.write(`replays: search for ${format} is not JSON; stopping this format\n`);
      break;
    }
    if (!Array.isArray(entries) || entries.length === 0) {
      state.done = true;
      break;
    }
    // The oldest entry this round looked at: a round that stops at the
    // target leaves the rest of the page for the next run.
    let oldest = entries[0].uploadtime;
    for (const ent of entries) {
      if (have >= opts.target) break;
      if (ent.uploadtime < oldest) oldest = ent.uploadtime;
      const file = path.join(formatDir(opts.cache, format, era), `${ent.id}.json`);
      if (fs.existsSync(file)) continue;
      // Never download a replay twice: the other era's cache and the list
      // of the replays whose seeds are of no era are checked too.
      if (Object.keys(ERAS).some((o) => fs.existsSync(path.join(formatDir(opts.cache, format, o), `${ent.id}.json`)))) continue;
      if (state.unknownSeed.includes(ent.id)) continue;
      const r = await get(`${REPLAY_BASE}/${ent.id}.json`, limiter);
      if (r.status !== 200) {
        state.skipped++;
        continue;
      }
      let replay;
      try {
        replay = JSON.parse(r.body);
      } catch (_) {
        state.skipped++;
        continue;
      }
      if (replay.inputlog) {
        const thisEra = eraOfSeed(inputlogEra(replay.inputlog));
        if (thisEra !== era) {
          // The other era's replay goes into the other era's cache (it is
          // not downloaded again from there); a replay of no era is
          // remembered by its id.
          state.otherEra++;
          if (thisEra) {
            ensureDir(formatDir(opts.cache, format, thisEra));
            fs.writeFileSync(path.join(formatDir(opts.cache, format, thisEra), `${ent.id}.json`), r.body);
          } else {
            state.unknownSeed.push(ent.id);
          }
          continue;
        }
      } else {
        // The server stored no input log for this battle (the OU
        // formats). The log is still worth keeping: it is what the
        // ps_client parse test reads.
        state.noInputLog++;
      }
      fs.writeFileSync(file, r.body);
      have++;
      state.downloaded++;
      if (have % 25 === 0) {
        process.stdout.write(
          `replays: ${era} ${format} ${have}/${opts.target} (${Math.round((Date.now() - started) / 1000)} s)\n`);
      }
    }
    // `before` is exclusive, so the next round starts one second after
    // the oldest entry we looked at (it is on disk, so it is not fetched
    // again).
    state.cursor = oldest + 1;
    saveState(opts.cache, format, era, state);
    rounds++;
  }
  saveState(opts.cache, format, era, state);
  process.stdout.write(
    `replays: ${era} ${format} done with ${have} replays (${state.skipped} failed, `
    + `${state.otherEra} from the other era, `
    + `${state.noInputLog} without an input log, `
    + `${rounds} search rounds)\n`);
}

// ---- validation -------------------------------------------------------------
//
// A replay is kept when the pinned PS, fed the replay's own `inputlog`,
// plays the same battle the server recorded.
//
// What the replay's `log` is: the server saves `this.room.getLog(-1)`
// (`server/room-battle.ts`, "replay log (exact damage)"), which is PS's
// OMNISCIENT log: the `|split|` pairs are resolved to their secret side,
// so the HP is exact. The comparison therefore runs against PS's
// channel -1 output (`extractChannelMessages(data, [-1])`), not the
// spectator view.
//
// The lines the server adds on top of the sim's log never decide whether
// a replay reproduces: chat (`|c|`), the joins and leaves (`|j|`, `|l|`),
// the name changes (`|n|`), the inactive notices (`|inactive|`,
// `|inactiveoff|`), the badges (`|badge|`), the html boxes (`|uhtml|`,
// `|uhtmlchange|`, `|html|`), the timestamps (`|t:|`) and the rating
// lines (`|raw|`).
//
// Drop reasons:
//   * `team-generator`: the random teams PS generates from the replay's
//     `>player` seeds are not the teams the replay revealed (PS's random
//     battle generator changed since the replay was recorded);
//   * `version-drift`: the teams match but the battle does not reproduce
//     (PS's rules or its output changed);
//   * `choice-rejected`: the teams match but PS rejected a recorded
//     choice (the battle went another way earlier: a version drift);
//   * `header-drift`: the battle reproduces line for line, but the
//     header lines (the `|rule|` lines, the `|teamsize|` lines) are not
//     the ones the server wrote back then. The replay still reproduces
//     and is kept;
//   * `text-drift`: the battle reproduces line for line once the
//     spelling is folded (`fold`). Kept;
//   * `error`: PS refused the replay (a broken log, PS threw).

/// The lines the server adds to the sim's log. They are not part of the
/// battle, so they never decide whether a replay reproduces.
const SERVER_ONLY = /^\|(c|c:|j|l|n|t:|inactive|inactiveoff|badge|uhtml|uhtmlchange|raw|html)\|/;
// The server adds lines of its own to the log it keeps: the notice when a
// player forfeits or is dropped for inactivity, and the roster lines (the
// server logs the players as they join and leave, the sim logs them once
// at the start).
// A best-of series adds the notice that a player is ready for the next
// game after the battle ends.
const SERVER_ADDED = /^\|-message\|.* (forfeited|lost due to inactivity)\.$|^\|player\|p[12]\||^\|\|.* is ready for game [0-9]+\.$/;

/// A replay's `inputlog`, split into the parts the driver needs.
function parseInputLog(inputlog) {
  const out = {
    version: null, start: null, players: {}, choices: { p1: [], p2: [] }, chat: [],
    // The commands that end the battle on their own (`>forcelose p2`),
    // with the number of choices that came before them in the recording.
    forces: [],
  };
  let choicesSoFar = 0;
  for (const raw of inputlog.split('\n')) {
    const line = raw.trim();
    if (!line.startsWith('>')) continue;
    const sp = line.indexOf(' ');
    const kind = sp < 0 ? line.slice(1) : line.slice(1, sp);
    const rest = sp < 0 ? '' : line.slice(sp + 1);
    if (kind === 'version') {
      out.version = rest;
    } else if (kind === 'version-origin') {
      // nothing
    } else if (kind === 'start') {
      try {
        out.start = JSON.parse(rest);
      } catch (_) {
        out.start = null;
      }
    } else if (kind === 'player') {
      const m = rest.match(/^(p[12])\s+([\s\S]*)$/);
      if (!m) continue;
      try {
        out.players[m[1]] = JSON.parse(m[2]);
      } catch (_) {
        out.players[m[1]] = null;
      }
    } else if (kind === 'chat' || kind === 'chat-inputlogonly') {
      out.chat.push(rest);
    } else if (kind === 'p1' || kind === 'p2') {
      out.choices[kind].push(rest);
      choicesSoFar += 1;
    } else if (kind === 'forcelose' || kind === 'forcewin' || kind === 'forcetie') {
      out.forces.push({ cmd: `>${kind}${rest ? ' ' + rest : ''}`, after: choicesSoFar });
    }
  }
  return out;
}

/// The forced losses that are due after `sent` choices. The queue is
/// emptied as it goes.
function takeForces(forces, sent) {
  const out = [];
  while (forces.length && forces[0].after <= sent) out.push(forces.shift());
  return out;
}

/// The lines of a log, with the server's own lines removed and the
/// timestamps normalised.
function normalise(lines) {
  const out = [];
  for (const raw of lines) {
    const line = raw.replace(/\r$/, '');
    if (!line) continue;
    if (SERVER_ONLY.test(line)) continue;
    if (SERVER_ADDED.test(line)) continue;
    out.push(line.replace(/^\|t:\|[0-9]+$/, '|t:||'));
  }
  return out;
}

/// The mons a replay revealed, per side, in the order they were sent out.
function revealedMons(log) {
  const out = { p1: [], p2: [] };
  for (const line of log.split('\n')) {
    const m = line.match(/^\|switch\|(p[12])a: ([^|]+)\|/);
    if (m && !out[m[1]].includes(m[2])) out[m[1]].push(m[2]);
  }
  return out;
}

/// Play a replay's `inputlog` through PS and return the log PS produced
/// (the omniscient one, channel -1), plus the teams PS generated.
async function runPS(replay) {
  const p = parseInputLog(replay.inputlog || '');
  if (!p.start) return { error: 'no >start line' };
  const stream = new BattleStream({ keepAlive: false });
  const updates = [];
  const errors = [];
  const teams = {};
  const keepAlive = setInterval(() => {}, 500);
  // The number of choices sent so far: it says where in the recording
  // the forced losses fall.
  let sent = 0;
  // Whether a forced loss has been sent: the battle is ending then.
  let forced = false;
  // A replay that never finishes must not hang the validation run.
  const deadline = setTimeout(() => {
    errors.push('timeout');
    try { stream.writeEnd(); } catch (_) { /* already closed */ }
  }, 20000);
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
        updates.push(data);
      } else if (type === 'sideupdate') {
        const nl2 = data.indexOf('\n');
        const side = data.slice(0, nl2);
        const body = data.slice(nl2 + 1);
        if (body.startsWith('|request|')) {
          let req;
          try {
            req = JSON.parse(body.slice('|request|'.length));
          } catch (_) {
            continue;
          }
          if (req.wait) continue;
          // The teams PS generated, as they are while the battle runs
          // (the battle is torn down when the stream ends).
          if (stream.battle && !Object.keys(teams).length) {
            for (const s of stream.battle.sides) {
              if (!s) continue;
              teams[s.id] = s.pokemon.map((p2) => ({
                species: p2.species.name, level: p2.level,
                moves: p2.moveSlots.map((m) => m.id),
              }));
            }
          }
          const q = p.choices[side];
          // A forced loss ends the battle on its own.
          const due = takeForces(p.forces, sent);
          if (due.length) {
            for (const f of due) stream.write(`${f.cmd}\n`);
            forced = true;
            continue;
          }
          if (forced) {
            // The forced loss is already in: the battle is ending.
            continue;
          }
          if (!q.length) {
            errors.push(`${side}: no recorded choice left`);
            // The battle cannot go on: stop the driver instead of waiting
            // for a choice that will never come.
            stopped = true;
            try { stream.writeEnd(); } catch (_) { /* already closed */ }
            return;
          }
          // A forced loss that came before this choice in the recording.
          const choice = q.shift();
          sent += 1;
          stream.write(`>${side} ${choice}\n`);
        } else if (body.startsWith('|error|')) {
          errors.push(`${side}: ${body}`);
          // PS does not ask again after a rejected choice: the player
          // sends the next one. The recording has it.
          const q = p.choices[side];
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
      } else if (type === 'end') {
        break;
      }
    }
  })();
  const cmds = [`>start ${JSON.stringify(p.start)}`];
  for (const side of ['p1', 'p2']) {
    const pl = p.players[side];
    if (!pl) continue;
    cmds.push(`>player ${side} ${JSON.stringify(pl)}`);
  }
  try {
    stream.write(cmds.join('\n') + '\n');
    await pump;
  } catch (e) {
    clearInterval(keepAlive);
    clearTimeout(deadline);
    return { error: `PS threw: ${e && e.message}` };
  }
  clearInterval(keepAlive);
  clearTimeout(deadline);
  // The omniscient view of every update block (the replay's log is the
  // server's channel -1 log).
  const lines = [];
  for (const u of updates) {
    const cm = extractChannelMessages(u, [-1]);
    for (const l of cm[-1]) lines.push(l);
  }
  // The teams PS generated for the random-battle sides.
  return { lines, errors, teams, parsed: p };
}

/// Does the team PS generated match the mons the replay revealed?
/// The revealed mons carry their level ("Parasect, L77") when the log
/// shows it, so the levels are compared too: a changed level table is a
/// changed team generator.
function teamsMatch(revealed, generated) {
  for (const side of ['p1', 'p2']) {
    const gen = generated[side];
    if (!gen || !gen.length) continue;
    const rev = revealed[side];
    if (!rev.length) continue;
    // The revealed mons are a subset of the team's mons (a battle does not
    // always send every mon out).
    for (const mon of rev) {
      const name = mon.replace(/, L\d+$/, '').trim().toLowerCase();
      const level = /, L(\d+)$/.exec(mon);
      const hit = gen.some((g) => {
        if (String(g.species).toLowerCase() !== name) return false;
        if (!level) return true;
        return g.level === parseInt(level[1], 10);
      });
      if (!hit) return false;
    }
  }
  return true;
}

/// The `|rule|` lines of a header, as a set (PS changed their order and
/// their wording between the replay's version and ours).
function ruleSet(lines) {
  return lines.filter((l) => l.startsWith('|rule|')).sort().join('\n');
}

/// A line with its spelling folded: the pinned PS writes `Pokemon`
/// without the accent (`data/rulesets.ts`, the Species Clause text)
/// where the server's stored log has the accented e. The spelling is
/// not the battle.
function fold(line) {
  return line.replace(/\u00e9/g, 'e');
}

/// The body of a log: everything from the `|start` line on, without the
/// `|teamsize|` lines (PS moved them from the header into the start
/// action).
function body(lines) {
  const at = lines.indexOf('|start');
  if (at < 0) return null;
  return lines.slice(at).filter((l) => !l.startsWith('|teamsize|'));
}

/// The header of a log: everything before the `|start` line, without the
/// `|teamsize|` lines.
function header(lines) {
  const at = lines.indexOf('|start');
  if (at < 0) return null;
  return lines.slice(0, at).filter((l) => !l.startsWith('|teamsize|'));
}

/// Validate one cached replay file.
/// Returns { id, format, kept, reason, detail, version }.
async function validateFile(file) {
  let replay;
  try {
    replay = JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch (e) {
    return { kept: false, reason: 'error', detail: `unreadable: ${e.message}` };
  }
  const id = replay.id || path.basename(file, '.json');
  const format = replay.formatid || ((replay.inputlog || '').split('\n')
    .find((l) => l.startsWith('>start')) || '').replace('>start ', '');
  const version = parseInputLog(replay.inputlog || '').version;
  if (!replay.inputlog) {
    return {
      id, format, version, kept: false, reason: 'no-inputlog',
      detail: 'the server stored no input log for this battle',
    };
  }
  const result = await runPS(replay);
  if (result.error) {
    return { id, format, version, kept: false, reason: 'error', detail: result.error };
  }
  const mine = normalise(result.lines);
  const theirs = normalise((replay.log || '').split('\n'));
  if (mine.length === 0) {
    return { id, format, version, kept: false, reason: 'error', detail: 'PS produced no log' };
  }
  const revealed = revealedMons(replay.log || '');
  const teamsOk = teamsMatch(revealed, result.teams);
  const generatorDrift = {
    id, format, version, kept: false, reason: 'team-generator',
    detail: `PS generated ${JSON.stringify(result.teams)} but the replay revealed ${JSON.stringify(revealed)}`,
  };
  if (result.errors.length) {
    const e = result.errors[0];
    if (e.includes('|error|')) {
      // PS rejected a recorded choice. When the team PS generated from
      // the seed is not the team the replay revealed, the random team
      // generator changed; otherwise the battle went another way earlier
      // (PS asked for a switch where the recording moved, say).
      if (!teamsOk || e.includes("doesn't have a move matching")) return { ...generatorDrift, detail: e };
      return { id, format, version, kept: false, reason: 'choice-rejected', detail: e };
    }
    if (!e.includes('no recorded choice left')) {
      return { id, format, version, kept: false, reason: 'error', detail: e };
    }
  }
  const myBody = body(mine);
  const theirBody = body(theirs);
  const myHeader = header(mine) || [];
  const theirHeader = header(theirs) || [];
  const same = (a, b) => a.join('\n') === b.join('\n');
  if (myBody && theirBody && (same(myBody, theirBody) || same(myBody.map(fold), theirBody.map(fold)))) {
    // The battle reproduces line for line. The header may still differ:
    // PS moved the `|teamsize|` lines and changed the `|rule|` lines.
    if (!teamsOk) return generatorDrift;
    if (same(myBody, theirBody) && same(myHeader, theirHeader)) {
      return { id, format, version, kept: true, reason: null, detail: null };
    }
    if (same(myHeader.map(fold), theirHeader.map(fold))) {
      // Only the spelling of the text lines differs (`fold`).
      return { id, format, version, kept: true, reason: 'text-drift', detail: 'the log differs only in spelling' };
    }
    return {
      id, format, version, kept: true, reason: 'header-drift',
      detail: `header differs (rules: ${ruleSet(myHeader) === ruleSet(theirHeader) ? 'same set' : 'different set'})`,
    };
  }
  // The battle itself differs: find the first differing line of the body.
  const a = myBody || mine;
  const b = theirBody || theirs;
  const n = Math.min(a.length, b.length);
  let at = n;
  for (let i = 0; i < n; i++) {
    if (fold(a[i]) !== fold(b[i])) { at = i; break; }
  }
  if (!teamsOk) return generatorDrift;
  // The same Pokemon entering at another level: the generator's level
  // table changed (`data/random-battles/<gen>/sets.json`), not the battle.
  const level = (l) => /^\|(switch|drag)\|([^|]*)\|([^,|]*)(?:, L([0-9]+))?/.exec(l || '');
  const la = level(a[at]);
  const lb = level(b[at]);
  if (la && lb && la[2] === lb[2] && la[3] === lb[3] && la[4] !== lb[4]) {
    return { ...generatorDrift, detail: `body line ${at}: PS ${JSON.stringify(a[at])} vs replay ${JSON.stringify(b[at])}` };
  }
  return {
    id, format, version, kept: false, reason: 'version-drift',
    detail: `body line ${at}: PS ${JSON.stringify(a[at])} vs replay ${JSON.stringify(b[at])}`,
  };
}

/// Every cached replay of a format.
function cachedReplays(cache, format, era) {
  const dir = formatDir(cache, format, era);
  if (!fs.existsSync(dir)) return [];
  return fs.readdirSync(dir)
    .filter((f) => f.endsWith('.json') && f !== 'state.json')
    .sort()
    .map((f) => path.join(dir, f));
}

/// The `validate` sub-command.
async function cmdValidate(opts) {
  const formats = opts.format && opts.format !== 'all' ? opts.format.split(',') : FORMATS;
  const eras = opts.era === 'all' ? ['gen5', 'current'] : [opts.era];
  for (const era of eras) {
    const e = ERAS[era];
    const outDir = path.join(opts.cache, e.dir, 'validated');
    fs.mkdirSync(outDir, { recursive: true });
    const summary = {};
    for (const format of formats) {
      const files = cachedReplays(opts.cache, format, era);
      const kept = [];
      const drifted = [];
      const dropped = [];
      const reasons = {};
      const results = [];
      // One worker per job, each with its own slice of the files.
      const jobs = Math.max(1, opts.jobs || 1);
      const per = Math.ceil(files.length / jobs);
      const slices = [];
      for (let i = 0; i < jobs; i++) {
        const start = i * per;
        const end = Math.min(start + per, files.length);
        if (start >= end) break;
        slices.push([start, end]);
      }
      const runSlice = async (slice) => {
        const out = [];
        for (let i = slice[0]; i < slice[1]; i++) {
          out.push(await validateFile(files[i]));
        }
        return out;
      };
      const all = slices.length
        ? (await Promise.all(slices.map((s) => runSlice(s)))).flat()
        : [];
      for (const r of all) {
        if (r.kept) {
          kept.push(r.id);
          if (r.reason === 'header-drift' || r.reason === 'text-drift') drifted.push(r.id);
        } else {
          dropped.push({ id: r.id, reason: r.reason, detail: r.detail });
        }
        reasons[r.reason || 'kept'] = (reasons[r.reason || 'kept'] || 0) + 1;
        results.push(r);
      }
      summary[format] = {
        cached: files.length, kept: kept.length, drifted: drifted.length,
        dropped: dropped.length, reasons,
      };
      fs.writeFileSync(
        path.join(outDir, `${format}.json`),
        JSON.stringify({ format, era, cached: files.length, kept, drifted, dropped, reasons }, null, 1) + '\n');
      process.stdout.write(
        `replays: ${era} ${format} kept ${kept.length} of ${files.length} (${JSON.stringify(reasons)})\n`);
    }
    fs.writeFileSync(path.join(outDir, 'summary.json'), JSON.stringify(summary, null, 1) + '\n');
  }
}

/// The `report` sub-command.
function cmdReport(opts) {
  const formats = opts.format && opts.format !== 'all' ? opts.format.split(',') : FORMATS;
  const eras = opts.era === 'all' ? ['gen5', 'current'] : [opts.era];
  for (const era of eras) {
    const e = ERAS[era];
    const outDir = path.join(opts.cache, e.dir, 'validated');
    for (const format of formats) {
      const file = path.join(outDir, `${format}.json`);
      if (!fs.existsSync(file)) {
        process.stdout.write(`replays: ${era} ${format}: no validation report\n`);
        continue;
      }
      const report = JSON.parse(fs.readFileSync(file, 'utf8'));
      process.stdout.write(
        `replays: ${era} ${format} kept ${report.kept.length} of ${report.cached} (${JSON.stringify(report.reasons)})\n`);
    }
  }
}

function parseArgs(argv) {
  const opts = {
    cmd: null, cache: DEFAULT_CACHE, format: 'all', target: 1000,
    jobs: 4, quiet: false, era: 'all',
  };
  let i = 0;
  while (i < argv.length) {
    const a = argv[i++];
    switch (a) {
      case 'fetch': opts.cmd = 'fetch'; break;
      case 'validate': opts.cmd = 'validate'; break;
      case 'report': opts.cmd = 'report'; break;
      case '--cache': opts.cache = argv[i++]; break;
      case '--format': opts.format = argv[i++]; break;
      case '--target': opts.target = parseInt(argv[i++], 10); break;
      case '--jobs': opts.jobs = parseInt(argv[i++], 10); break;
      case '--era': opts.era = argv[i++]; break;
      case '--quiet': opts.quiet = true; break;
      case '--help': case '-h':
        console.log(
          'Usage: node replays.js fetch|validate|report [--cache DIR] '
          + '[--format id[,id]|all] [--era gen5|current|all] [--target N] [--jobs N]');
        process.exit(0);
      default:
        if (a.startsWith('--')) throw new Error(`unknown option ${a}`);
    }
  }
  if (!opts.cmd) throw new Error('a sub-command is required: fetch, validate, report');
  if (opts.era !== 'all' && !ERAS[opts.era]) throw new Error(`unknown era ${opts.era}`);
  return opts;
}

async function main() {
  const opts = parseArgs(process.argv.slice(2));
  if (opts.cmd === 'fetch') {
    await cmdFetch(opts);
    return;
  }
  if (opts.cmd === 'validate') {
    await cmdValidate(opts);
    return;
  }
  if (opts.cmd === 'report') {
    cmdReport(opts);
    return;
  }
}

if (require.main === module) {
  main().catch((e) => {
    process.stderr.write(`${e && e.stack ? e.stack : e}\n`);
    process.exit(2);
  });
}

module.exports = {
  FORMATS, ERAS, USER_AGENT, DEFAULT_BEFORE, CURRENT_FLOOR, MIN_INTERVAL_MS, SERVER_ONLY,
  RateLimiter, get, sleep, ensureDir, formatDir, loadState, saveState,
  cachedCount, seedsAreNumeric, inputlogEra, seedEra, battleSeedOf, playerSeedsOf, parseArgs,
  parseInputLog, normalise, runPS, validateFile, revealedMons, teamsMatch, takeForces,
  cachedReplays, cmdValidate, cmdReport, body, header, ruleSet,
};
