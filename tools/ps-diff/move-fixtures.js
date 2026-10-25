#!/usr/bin/env node
'use strict';

// Records PS 0.11.11 Gen 1 and Gen 3 battles for the ps_sim move pipeline
// tests (tasks P1-8 and P1-9): whole battles, to the end, whose teams use
// only the moves and abilities ps_sim ports (`ported(gen)`,
// crates/ps_sim/src/mods/mod.rs) and no items.
//
// The ported list is ps_sim's own: `ps_sim_cli --ported <gen>` prints the
// `kind:id` entries of `ported(gen)`, and the fixture's first line
// (`#ported ...`) is that list less its species, which the Rust test checks
// against `ported(gen)`. A handler batch therefore never edits this
// recorder: when `ported(gen)` grows, it regenerates the fixture (build the
// CLI first: `cargo build --release -p ps_harness --bin ps_sim_cli`). Gen
// 1's line predates the `kind:id` form and lists bare move ids; gen 3's
// lists `ability:<id>` and `move:<id>` entries.
//
// Teams: PS's own gen<N>randombattle generator gives species, levels and
// stats (DVs and stat exp; gen 3 also natures, IVs and EVs) in two battles
// of three; the third rolls the level and those stats. Either way each
// Pokemon gets 1 to 4 random ported moves, no item, and in gen 3 a random
// ported ability. Choices are random legal ones from PS's request: mostly
// moves, sometimes a switch, now and then an invalid choice or an undo.
// Gen 1 runs with `cartridgeTrunc`, the one PS-side adaptation the
// harnesses make (tools/showdown-parity/harness.js).
//
// Format: as tools/ps-diff/battle-fixtures.js (`#battle`, commands,
// `>update` blocks, requests by FNV-1a 64 hash, `#end`), gzipped. The full
// text, requests included, goes to .scratch/move-fixtures/ for debugging.
//
// Usage:
//   node tools/ps-diff/move-fixtures.js [--gen 1|3] [--ps-cli PATH]          write the fixtures
//   node tools/ps-diff/move-fixtures.js [--gen 1|3] [--ps-cli PATH] --check  regenerate and compare

const fs = require('fs');
const path = require('path');
const zlib = require('zlib');
const {execFileSync} = require('child_process');
const {Battle, Teams, Dex} = require('pokemon-showdown/dist/sim');

const REPO = path.resolve(__dirname, '..', '..');
const OUT_DIR = path.join(REPO, 'crates', 'ps_sim', 'tests', 'fixtures');
const SCRATCH = path.join(REPO, '.scratch', 'move-fixtures');

const FORMATS = {
  1: 'gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod',
  3: 'gen3customgame',
};
const BATTLES = {1: 320, 3: 330};
// The recorder's random stream, per gen.
const SEEDS = {1: 0x9e11 * 7 + 1, 3: 0x9e33 * 7 + 3};

// Battles for the paths random teams rarely reach: a stat boosted at 999
// and a stat dropped at 1 bounce back (gen 1's `boost`).
function edgeSet(species, moves, level, dv, statExp) {
  const stats = v => ({hp: v, atk: v, def: v, spa: v, spd: v, spe: v});
  return Teams.pack([{name: '', species, item: '', ability: 'No Ability', moves, nature: '',
    evs: stats(statExp), ivs: stats(2 * dv), level}]);
}
const EDGE = {
  1: [
    // Swords Dance past 999 Attack.
    [edgeSet('Rhydon', ['swordsdance'], 100, 15, 255), edgeSet('Chansey', ['tackle'], 100, 15, 255)],
    // Growl and Screech down to 1.
    [edgeSet('Caterpie', ['tackle'], 2, 0, 0), edgeSet('Mewtwo', ['growl', 'screech'], 100, 15, 255)],
  ],
  3: [],
};

// ---------------------------------------------------------------------------
// The ported list
// ---------------------------------------------------------------------------

// `ps_sim_cli`, for the ported list (`--ps-cli PATH`, default the release
// build in this tree).
function cliPath() {
  const i = process.argv.indexOf('--ps-cli');
  const cli = i >= 0 ? process.argv[i + 1] : path.join(REPO, 'target', 'release', 'ps_sim_cli');
  if (!fs.existsSync(cli)) {
    throw new Error(`no ${cli}: run cargo build --release -p ps_harness --bin ps_sim_cli`);
  }
  return cli;
}

/// `{moves, abilities, header}`: the ported moves and abilities of `gen`
/// and the fixture's `#ported` line, from ps_sim's own list.
function portedFor(gen) {
  const entries = execFileSync(cliPath(), ['--ported', String(gen)], {encoding: 'utf8'})
    .split('\n')
    .map(l => l.trim())
    .filter(e => e && !e.startsWith('species:'));
  const ids = kind => entries.filter(e => e.startsWith(`${kind}:`)).map(e => e.slice(kind.length + 1));
  const moves = ids('move');
  if (gen === 1) return {moves, abilities: [], header: `#ported ${moves.join(',')}`};
  return {moves, abilities: ids('ability'), header: `#ported ${entries.join(',')}`};
}

function toID(text) {
  return String(text).toLowerCase().replace(/[^a-z0-9]+/g, '');
}

let PORTED = null;

// ---------------------------------------------------------------------------
// Random numbers and teams
// ---------------------------------------------------------------------------

function mulberry32(a) {
  let s = a >>> 0;
  return function () {
    s = (s + 0x6d2b79f5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function u16(rng) {
  return Math.floor(rng() * 65536) & 0xffff;
}

function seedOf(rng) {
  return [u16(rng), u16(rng), u16(rng), u16(rng)];
}

function pick(rng, list) {
  return list[Math.floor(rng() * list.length) % list.length];
}

function randomMoves(rng) {
  const n = 1 + Math.floor(rng() * 4);
  const moves = [];
  while (moves.length < n) {
    const m = pick(rng, PORTED.moves);
    if (!moves.includes(m)) moves.push(m);
  }
  return moves;
}

const NATURES = Dex.natures.all().map(n => n.name).sort();

// PS's random battle sets, with ported moves (and in gen 3 a ported
// ability, no item); `wide` re-rolls the level and the stats: gen 1's DVs
// (even IVs, as gen 1 stores them) and stat exp, gen 3's IVs, EVs and
// nature.
function team(rng, gen, size, wide) {
  const sets = Teams.getGenerator(`gen${gen}randombattle`, seedOf(rng)).getTeam().slice(0, size);
  for (const set of sets) {
    set.moves = randomMoves(rng);
    if (gen === 3) {
      set.item = '';
      set.ability = pick(rng, PORTED.abilities);
    }
    if (wide) {
      set.level = 2 + Math.floor(rng() * 99);
      if (gen === 1) {
        for (const k of Object.keys(set.ivs)) set.ivs[k] = 2 * Math.floor(rng() * 16);
        for (const k of Object.keys(set.evs)) set.evs[k] = Math.floor(rng() * 256);
      } else {
        for (const k of Object.keys(set.ivs)) set.ivs[k] = Math.floor(rng() * 32);
        for (const k of Object.keys(set.evs)) set.evs[k] = 4 * Math.floor(rng() * 64);
        set.nature = pick(rng, NATURES);
      }
    }
  }
  return Teams.pack(sets);
}

// ---------------------------------------------------------------------------
// Recording (as battle-fixtures.js)
// ---------------------------------------------------------------------------

function fnv64(text) {
  let h = 0xcbf29ce484222325n;
  for (const byte of Buffer.from(text, 'utf8')) {
    h ^= BigInt(byte);
    h = (h * 0x100000001b3n) & 0xffffffffffffffffn;
  }
  return h.toString(16).padStart(16, '0');
}

class Recorder {
  constructor(gen, index) {
    this.lines = [`#battle moves gen=${gen} index=${index}`];
    this.full = [this.lines[0]];
    this.pending = [];
    this.requests = {p1: null, p2: null};
  }

  command(line) {
    this.lines.push(line);
    this.full.push(line);
  }

  send(type, data) {
    if (Array.isArray(data)) data = data.join('\n');
    this.pending.push([type, data]);
  }

  flush() {
    for (const [type, data] of this.pending) {
      if (type === 'end') continue;
      let header;
      let body = data;
      if (type === 'sideupdate') {
        const nl = data.indexOf('\n');
        const side = data.slice(0, nl);
        body = data.slice(nl + 1);
        header = `sideupdate ${side}`;
        const req = body.split('\n').find(l => l.startsWith('|request|'));
        if (req) this.requests[side] = JSON.parse(req.slice('|request|'.length));
      } else {
        header = 'update';
      }
      this.full.push(`>${header}`, ...body.split('\n'));
      if (type === 'sideupdate' && body.startsWith('|request|')) {
        this.lines.push(`>hash ${header} ${fnv64(body)}`);
      } else {
        this.lines.push(`>${header}`, ...body.split('\n'));
      }
    }
    this.pending = [];
  }

  end() {
    this.lines.push('#end');
    this.full.push('#end');
  }
}

/// The old harness's `cartridgeTrunc`: Gen 1's bit-width truncation back.
function cartridgeTrunc(battle) {
  battle.trunc = (num, bits = 0) => (bits ? (num >>> 0) % (2 ** bits) : num >>> 0);
  return battle;
}

function startBattle(rec, gen, seed, team1, team2) {
  const start = {formatid: FORMATS[gen], seed};
  const p1 = {name: 'Player 1', team: team1};
  const p2 = {name: 'Player 2', team: team2};
  rec.command(`>start ${JSON.stringify(start)}`);
  rec.command(`>player p1 ${JSON.stringify(p1)}`);
  rec.command(`>player p2 ${JSON.stringify(p2)}`);
  let battle = new Battle({...start, send: (t, d) => rec.send(t, d)});
  if (gen === 1) battle = cartridgeTrunc(battle);
  battle.setPlayer('p1', p1);
  battle.setPlayer('p2', p2);
  battle.sendUpdates();
  rec.flush();
  return battle;
}

function legalSwitches(req) {
  const out = [];
  (req.side?.pokemon || []).forEach((p, i) => {
    if (!p.active && p.condition !== '0 fnt') out.push(i + 1);
  });
  return out;
}

function choose(battle, rec, side, rng) {
  const req = rec.requests[side];
  if (!req || req.wait) return;
  const send = (choice) => {
    rec.command(`>${side} ${choice}`);
    if (choice === 'undo') battle.undoChoice(side);
    else battle.choose(side, choice);
    battle.sendUpdates();
    rec.flush();
  };
  const sw = legalSwitches(req);
  if (req.forceSwitch) {
    if (rng() < 0.05) send('switch 1'); // an active Pokemon: rejected
    send(sw.length ? `switch ${pick(rng, sw)}` : 'default');
    return;
  }
  if (rng() < 0.02) send(`move ${5 + Math.floor(rng() * 4)}`); // no such move
  const moves = [];
  (req.active[0].moves || []).forEach((m, i) => {
    if (!m.disabled) moves.push(i + 1);
  });
  // A hard-locked Pokemon (the second turn of a two-turn move) cannot
  // switch: PS rejects the choice and the battle stalls.
  const locked = !!req.active[0].trapped;
  let choice;
  if (!locked && sw.length && (rng() < 0.12 || !moves.length)) choice = `switch ${pick(rng, sw)}`;
  else choice = moves.length ? `move ${pick(rng, moves)}` : 'default';
  send(choice);
  // A `maybeTrapped` Pokemon (a foe's Magnet Pull, Arena Trap or Shadow
  // Tag might hold it) finds out it is trapped when PS rejects the switch
  // and re-sends the request with `trapped`: it picks a move then. (No
  // draw unless that happens, so earlier fixtures replay unchanged.)
  if (choice.startsWith('switch') && /trapped/.test(battle.getSide(side).choice.error)) {
    const again = [];
    (rec.requests[side].active[0].moves || []).forEach((m, i) => {
      if (!m.disabled) again.push(i + 1);
    });
    send(again.length ? `move ${pick(rng, again)}` : 'default');
  }
}

function play(battle, rec, rng) {
  for (let guard = 0; guard < 50000 && !battle.ended; guard++) {
    const before = battle.inputLog.length;
    const sides = rng() < 0.5 ? ['p1', 'p2'] : ['p2', 'p1'];
    for (const side of sides) {
      if (battle.ended) break;
      const req = rec.requests[side];
      if (!req || req.wait || battle.getSide(side).isChoiceDone()) continue;
      choose(battle, rec, side, rng);
      if (!battle.ended && rng() < 0.03 && battle.getSide(side).isChoiceDone() && !battle.allChoicesDone()) {
        rec.command(`>${side} undo`);
        battle.undoChoice(side);
        battle.sendUpdates();
        rec.flush();
        choose(battle, rec, side, rng);
      }
    }
    if (battle.inputLog.length === before) break;
  }
  if (!battle.ended) throw new Error('battle did not end');
}

function record(gen) {
  PORTED = portedFor(gen);
  const out = [PORTED.header];
  const full = [out[0]];
  const rng = mulberry32(SEEDS[gen]);
  for (let i = 0; i < BATTLES[gen]; i++) {
    const rec = new Recorder(gen, i);
    const wide = i % 3 === 2;
    const size1 = 1 + Math.floor(rng() * 6);
    const size2 = 1 + Math.floor(rng() * 6);
    const battle = startBattle(rec, gen, seedOf(rng), team(rng, gen, size1, wide), team(rng, gen, size2, wide));
    play(battle, rec, rng);
    rec.end();
    out.push(...rec.lines);
    full.push(...rec.full);
  }
  EDGE[gen].forEach(([team1, team2], k) => {
    const rec = new Recorder(gen, BATTLES[gen] + k);
    const battle = startBattle(rec, gen, seedOf(rng), team1, team2);
    play(battle, rec, rng);
    rec.end();
    out.push(...rec.lines);
    full.push(...rec.full);
  });
  return {text: out.join('\n') + '\n', full: full.join('\n') + '\n'};
}

/// Records (or with `check`, compares) one gen's fixture; false when it
/// differs.
function run(gen, check) {
  const out = path.join(OUT_DIR, `moves_gen${gen}.txt.gz`);
  const realNow = Date.now;
  Date.now = () => 1700000000000;
  let files;
  try {
    files = record(gen);
  } finally {
    Date.now = realNow;
  }
  fs.mkdirSync(SCRATCH, {recursive: true});
  fs.writeFileSync(path.join(SCRATCH, `full_gen${gen}.txt`), files.full);
  const name = path.relative(REPO, out);
  if (check) {
    const old = fs.existsSync(out) ? zlib.gunzipSync(fs.readFileSync(out)).toString('utf8') : null;
    const same = old === files.text;
    console.log(`${same ? 'ok     ' : 'DIFFERS'}  ${name}`);
    return same;
  }
  const bytes = zlib.gzipSync(Buffer.from(files.text, 'utf8'), {level: 9});
  fs.writeFileSync(out, bytes);
  console.log(`wrote    ${name} (${(files.text.length / 1024).toFixed(1)} KB, ${(bytes.length / 1024).toFixed(1)} KB gzipped)`);
  return true;
}

function main() {
  const check = process.argv.includes('--check');
  const g = process.argv.indexOf('--gen');
  const gens = g >= 0 ? [Number(process.argv[g + 1])] : [1, 3];
  for (const gen of gens) {
    if (!FORMATS[gen]) throw new Error(`no gen ${gen}`);
  }
  let ok = true;
  for (const gen of gens) ok = run(gen, check) && ok;
  if (!ok) process.exit(1);
}

main();
