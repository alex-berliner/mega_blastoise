#!/usr/bin/env node
'use strict';

// Records PS 0.11.11 battles for the ps_sim turn-loop tests (task P1-6).
//
// ps_sim has no move pipeline yet (P1-8, P1-9 bring it), so these battles
// are the ones the turn loop can play on its own:
//
//   prefix   the first update and the first two requests of the P1-4 corpus
//            battles (`record-logs.js`, same seeds, same teams), i.e.
//            everything up to the first move;
//   switch   both players only ever switch, until the turn-1000 tie;
//   noprog   Gen 1's no-progress tie (every Pokemon knows only Transform);
//   hook     battles whose moves run a stub instead of PS's `runMove`: the
//            same stub ps_sim's tests install (`STUB` below and
//            `stub_run_move` in crates/ps_sim/src/battle_tests.rs). It
//            exercises faints, forced switches, wait requests, the gen 1
//            queue clear, the gen 3 cancelAction, `checkWin` and the double
//            KO tie, without any move mechanics.
//
// The fixtures are stored gzipped (`battles_gen<g>.txt.gz`), as
// record-logs.js stores its own. Gzip is only the storage format: `--check`
// compares the text inside, so a different zlib build cannot make it fail.
//
// Battles are driven like `BattleStream` does it: one `Battle`, a `send`
// that collects `update` / `sideupdate` messages, `sendUpdates()` after each
// command chunk. Output format, one battle after another:
//
//   #battle <kind> gen=<g> index=<i>
//   >start {...}                  a command (consecutive commands before a
//   >player p1 {...}              block were written as one chunk)
//   >update                       an update block, lines follow
//   >sideupdate p1                a side update block, lines follow
//   >hash update <fnv64> <n>      a block recorded by hash only (n lines)
//   >hash sideupdate p1 <fnv64>
//   #end
//
// Requests of the long battles are hashed (FNV-1a 64 over the UTF-8 block
// text) to keep the fixtures small; their full text goes to
// .scratch/battle-fixtures/ for debugging.
//
// Usage:
//   node tools/ps-diff/battle-fixtures.js            write the fixtures
//   node tools/ps-diff/battle-fixtures.js --check    regenerate and compare

const fs = require('fs');
const path = require('path');
const zlib = require('zlib');
const {Battle, Teams, Dex} = require('pokemon-showdown/dist/sim');

const REPO = path.resolve(__dirname, '..', '..');
const OUT_DIR = path.join(REPO, 'crates', 'ps_sim', 'tests', 'fixtures');
const SCRATCH = path.join(REPO, '.scratch', 'battle-fixtures');

const FORMATS = {
  1: 'gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod',
  3: 'gen3customgame',
};

// ---------------------------------------------------------------------------
// The corpus derivation of record-logs.js (kept identical on purpose)
// ---------------------------------------------------------------------------

const MASTER_SEED = 0x5eed1e;

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

function corpusBattle(gen, index) {
  const seedRng = mulberry32((MASTER_SEED + gen * 7919 + index * 104729) >>> 0);
  const battleSeed = seedOf(seedRng);
  const teamSeed1 = seedOf(seedRng);
  const teamSeed2 = seedOf(seedRng);
  const team1 = Teams.pack(Teams.getGenerator(`gen${gen}randombattle`, teamSeed1).getTeam());
  const team2 = Teams.pack(Teams.getGenerator(`gen${gen}randombattle`, teamSeed2).getTeam());
  return {seed: battleSeed, team1, team2};
}

// ---------------------------------------------------------------------------
// The stub move pipeline
// ---------------------------------------------------------------------------

// What a move does in a hook battle, by the move's slot: slot 1 and 4 deal a
// third of the target's max HP (at least 1), slot 2 knocks the target out,
// slot 3 makes the user faint, then knocks the target out (a double KO
// when both are last). No PRNG call anywhere. Mirrored by `stub_run_move`.
function STUB(move, pokemon) {
  const battle = pokemon.battle;
  const target = pokemon.side.foe.active[0];
  let slot = pokemon.moveSlots.findIndex(s => s.id === move.id);
  if (slot < 0) slot = 0;
  battle.add('move', pokemon, move.name, target);
  if (slot === 2) pokemon.faint();
  if (!target || target.fainted) return;
  const damage = slot === 1 || slot === 2 ? target.hp : Math.max(1, Math.floor(target.maxhp / 3));
  battle.directDamage(damage, target, pokemon);
}

// ---------------------------------------------------------------------------
// Teams
// ---------------------------------------------------------------------------

// Gen 3 hook and switch teams carry nothing with handlers ps_sim lacks: no
// item, a handler-free ability, no move with a beforeTurnCallback.
const GEN3_FREE_ABILITIES = ['Early Bird', 'Levitate', 'Run Away'];
const BEFORE_TURN = new Set(['counter', 'mirrorcoat', 'pursuit', 'focuspunch']);

function pick(rng, list) {
  return list[Math.floor(rng() * list.length) % list.length];
}

function customGen3Team(rng, size) {
  const dex = Dex.mod('gen3');
  const species = dex.species.all().filter(s => !s.isNonstandard && s.num > 0 && !s.forme);
  const moves = dex.moves.all().filter(m => !m.isNonstandard && m.id !== 'struggle' && !BEFORE_TURN.has(m.id));
  const natures = dex.natures.all();
  const team = [];
  for (let i = 0; i < size; i++) {
    const s = pick(rng, species);
    const set = {
      name: rng() < 0.3 ? `Nick${i}` : '',
      species: s.name,
      item: '',
      ability: pick(rng, GEN3_FREE_ABILITIES),
      moves: [],
      nature: pick(rng, natures).name,
      evs: {hp: 0, atk: 0, def: 0, spa: 0, spd: 0, spe: 0},
      ivs: {hp: 31, atk: 31, def: 31, spa: 31, spd: 31, spe: 31},
      level: 50 + Math.floor(rng() * 51),
      shiny: rng() < 0.2,
      happiness: Math.floor(rng() * 256),
    };
    for (const k of Object.keys(set.evs)) set.evs[k] = Math.floor(rng() * 64) * 4;
    for (const k of Object.keys(set.ivs)) set.ivs[k] = Math.floor(rng() * 32);
    if (rng() < 0.4) set.gender = pick(rng, ['M', 'F']);
    const n = 1 + Math.floor(rng() * 4);
    while (set.moves.length < n) {
      const m = rng() < 0.15 ? pick(rng, ['return', 'frustration', 'hiddenpower']) : pick(rng, moves).id;
      if (!set.moves.includes(m)) set.moves.push(m);
    }
    team.push(set);
  }
  return Teams.pack(team);
}

function randomTeam(gen, rng) {
  return Teams.pack(Teams.getGenerator(`gen${gen}randombattle`, seedOf(rng)).getTeam());
}

// ---------------------------------------------------------------------------
// Playing
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
  constructor(kind, gen, index, {hashRequests = false, hashUpdates = null} = {}) {
    this.lines = [`#battle ${kind} gen=${gen} index=${index}`];
    this.full = [this.lines[0]];
    this.hashRequests = hashRequests;
    this.hashUpdates = hashUpdates; // (blockIndex) => bool
    this.pending = [];
    this.blocks = 0;
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
      const hashed = type === 'sideupdate' ?
        this.hashRequests && body.startsWith('|request|') :
        this.hashUpdates && this.hashUpdates(this.blocks);
      if (hashed) {
        const n = body.split('\n').length;
        this.lines.push(type === 'update' ? `>hash update ${fnv64(body)} ${n}` : `>hash ${header} ${fnv64(body)}`);
      } else {
        this.lines.push(`>${header}`, ...body.split('\n'));
      }
      if (type === 'update') this.blocks++;
    }
    this.pending = [];
  }

  end() {
    this.lines.push('#end');
    this.full.push('#end');
  }
}

function startBattle(rec, gen, seed, team1, team2, hook) {
  const formatid = FORMATS[gen];
  const start = {formatid, seed};
  const p1 = {name: 'Player 1', team: team1};
  const p2 = {name: 'Player 2', team: team2};
  rec.command(`>start ${JSON.stringify(start)}`);
  rec.command(`>player p1 ${JSON.stringify(p1)}`);
  rec.command(`>player p2 ${JSON.stringify(p2)}`);
  const battle = new Battle({...start, send: (t, d) => rec.send(t, d)});
  if (hook) {
    battle.actions.runMove = function (moveOrMoveName, pokemon) {
      STUB(this.dex.getActiveMove(moveOrMoveName), pokemon);
    };
  }
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

// One choice for `side`'s current request: `mode` 'switch' only switches
// (while it can), 'moves' always uses the first move, 'mixed' moves mostly
// and sometimes switches, makes a mistake or undoes.
function choose(battle, rec, side, rng, mode) {
  const req = rec.requests[side];
  if (!req || req.wait) return;
  const send = (choice) => {
    rec.command(`>${side} ${choice}`);
    if (choice === 'undo') battle.undoChoice(side);
    else battle.choose(side, choice);
    battle.sendUpdates();
    rec.flush();
  };
  if (req.forceSwitch) {
    if (mode === 'mixed' && rng() < 0.1) send('switch 1'); // an active Pokemon: rejected
    const sw = legalSwitches(req);
    send(sw.length ? `switch ${pick(rng, sw)}` : 'default');
    return;
  }
  const sw = legalSwitches(req);
  if (mode === 'moves') {
    send('move 1');
    return;
  }
  if (mode === 'switch' && sw.length) {
    send(`switch ${pick(rng, sw)}`);
    return;
  }
  if (mode === 'mixed' && rng() < 0.05) send(`move ${5 + Math.floor(rng() * 4)}`); // no such move
  const moves = [];
  (req.active[0].moves || []).forEach((m, i) => {
    if (!m.disabled && !BEFORE_TURN.has(m.id)) moves.push(i + 1);
  });
  let choice;
  if (sw.length && (rng() < 0.2 || !moves.length)) choice = `switch ${pick(rng, sw)}`;
  else choice = moves.length ? `move ${pick(rng, moves)}` : 'default';
  send(choice);
}

function play(battle, rec, rng, mode, maxTurns) {
  for (let guard = 0; guard < 20000 && !battle.ended && battle.turn <= maxTurns; guard++) {
    const before = battle.inputLog.length;
    const sides = ['p1', 'p2'];
    if (mode === 'mixed' && rng() < 0.5) sides.reverse();
    for (const side of sides) {
      if (battle.ended) break;
      const req = rec.requests[side];
      if (!req || req.wait || battle.getSide(side).isChoiceDone()) continue;
      choose(battle, rec, side, rng, mode);
      // Undo and choose again sometimes, while the other side is still
      // choosing (an undo after both have chosen is too late).
      if (mode === 'mixed' && !battle.ended && rng() < 0.1 && battle.getSide(side).isChoiceDone() &&
        !battle.allChoicesDone()) {
        rec.command(`>${side} undo`);
        battle.undoChoice(side);
        battle.sendUpdates();
        rec.flush();
        choose(battle, rec, side, rng, mode);
      }
    }
    if (battle.inputLog.length === before) break;
  }
}

// ---------------------------------------------------------------------------
// The battle list
// ---------------------------------------------------------------------------

const PREFIX_PER_GEN = 60;
const HOOK_PER_GEN = 24;

function recordGen(gen) {
  const out = [];
  const full = [];
  const keep = (rec) => {
    out.push(...rec.lines);
    full.push(...rec.full);
  };

  // prefix: the P1-4 corpus up to the first move.
  for (let i = 0; i < PREFIX_PER_GEN; i++) {
    const {seed, team1, team2} = corpusBattle(gen, i);
    const rec = new Recorder('prefix', gen, i);
    startBattle(rec, gen, seed, team1, team2, false);
    rec.end();
    keep(rec);
  }

  const rng = mulberry32((0xb477 + gen * 977) >>> 0);
  const team = (size) => (gen === 3 ? customGen3Team(rng, size) : randomTeam(1, rng));

  // switch: switch every turn, to the turn-1000 tie. Updates past the
  // first five and before the last ten are hashed.
  for (let i = 0; i < 2; i++) {
    const rec = new Recorder('switch', gen, i, {hashRequests: true, hashUpdates: (b) => b >= 5 && b < 990});
    const battle = startBattle(rec, gen, seedOf(rng), team(6), team(6), true);
    play(battle, rec, rng, 'switch', 1001);
    rec.end();
    keep(rec);
  }

  // noprog: gen 1's endless battle check.
  if (gen === 1) {
    const ditto = (name) => `${name}|Ditto||NoAbility|transform||255,255,255,255,255,255||30,30,30,30,30,30||${name === 'D1' ? 70 : ''}|`;
    // A slow Rattata whose one move chips (slot 1), against a Ditto that
    // chips back faster: the Rattata faints first, then only Transform is
    // left on the field.
    const rattata = 'Rattata|||NoAbility|tackle||255,255,255,255,255,255||30,30,30,30,30,30||2|';
    let rec = new Recorder('noprog', gen, 0);
    let battle = startBattle(rec, gen, seedOf(rng), ditto('D1'), ditto('D2'), true);
    rec.end();
    keep(rec);
    // The tie comes once the last non-Transform Pokemon faints.
    rec = new Recorder('noprog', gen, 1);
    battle = startBattle(rec, gen, seedOf(rng), `${rattata}]${ditto('D1')}`, ditto('D2'), true);
    play(battle, rec, rng, 'moves', 50);
    rec.end();
    keep(rec);
  }

  // hook: random play with the stub pipeline.
  for (let i = 0; i < HOOK_PER_GEN; i++) {
    const rec = new Recorder('hook', gen, i, {hashRequests: true});
    const size = 1 + (i % 6);
    const battle = startBattle(rec, gen, seedOf(rng), team(size), team(6 - (i % 3)), true);
    play(battle, rec, rng, 'mixed', 300);
    rec.end();
    keep(rec);
  }

  // dko: a double KO of both last Pokemon (slot 3 is the stub's
  // explosion), a speed tie decided by the PRNG.
  {
    const mon = gen === 1 ?
      'Electrode|||NoAbility|thunderbolt,thunder,explosion||255,255,255,255,255,255||30,30,30,30,30,30||80|' :
      'Electrode|||Levitate|thunderbolt,thunder,explosion|||M|||80|';
    const rec = new Recorder('dko', gen, 0);
    const battle = startBattle(rec, gen, seedOf(rng), mon, mon, true);
    for (const side of ['p1', 'p2']) {
      rec.command(`>${side} move 3`);
      battle.choose(side, 'move 3');
      battle.sendUpdates();
      rec.flush();
    }
    rec.end();
    keep(rec);
  }

  return {text: out.join('\n') + '\n', full: full.join('\n') + '\n'};
}

function main() {
  const check = process.argv.includes('--check');
  const frozenNow = 1700000000000;
  const realNow = Date.now;
  Date.now = () => frozenNow;
  const files = {};
  try {
    for (const gen of [1, 3]) files[gen] = recordGen(gen);
  } finally {
    Date.now = realNow;
  }
  fs.mkdirSync(SCRATCH, {recursive: true});
  let bad = 0;
  for (const gen of [1, 3]) {
    const name = `battles_gen${gen}.txt.gz`;
    fs.writeFileSync(path.join(SCRATCH, `full_gen${gen}.txt`), files[gen].full);
    const target = path.join(OUT_DIR, name);
    if (check) {
      const old = fs.existsSync(target) ? zlib.gunzipSync(fs.readFileSync(target)).toString('utf8') : null;
      const same = old === files[gen].text;
      if (!same) bad++;
      console.log(`${same ? 'ok     ' : 'DIFFERS'}  ${name}`);
      continue;
    }
    fs.mkdirSync(OUT_DIR, {recursive: true});
    const bytes = zlib.gzipSync(Buffer.from(files[gen].text, 'utf8'), {level: 9});
    fs.writeFileSync(target, bytes);
    console.log(`wrote    ${path.relative(REPO, target)} (${(files[gen].text.length / 1024).toFixed(1)} KB, ` +
      `${(bytes.length / 1024).toFixed(1)} KB gzipped)`);
  }
  if (bad) process.exit(1);
}

main();
