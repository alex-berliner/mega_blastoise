#!/usr/bin/env node
'use strict';

// Records short scripted PS 0.11.11 battles for ps_sim's handler tests
// (task G1-6 onward): fixed teams, a fixed seed and a fixed list of
// choices, so each one reaches the effects it is written for (Transform,
// Mimic, Counter, Mirror Move, Metronome). Each scenario's expected log is
// every `update` block PS sends, joined, without the `|t:|` lines (the
// timestamp). Gen 1 runs with the cartridge trunc, as every harness does.
//
// Output: crates/ps_sim/tests/fixtures/scripted_gen1.json, an array of
// `{name, format, seed, p1, p2, choices, log}`; `choices` is a list of
// `[p1, p2]` pairs, `null` for a side that does not choose that turn.
//
// Usage: node tools/ps-diff/scripted-fixtures.js

const fs = require('fs');
const path = require('path');
const { Battle } = require('pokemon-showdown/dist/sim');

const FORMAT = 'gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod';
const EVS = '255,255,255,255,255,255';
const DVS = '30,30,30,30,30,30';
// Level 100 is packed as an empty field.
const mon = (species, moves) => `${species}|||NoAbility|${moves}||${EVS}||${DVS}|||`;

const SCENARIOS = [
  {
    // Transform (stats, moves at 5 PP, boosts), the copied moves, a
    // switch out and back (the real moves return), Transform again; Mimic
    // and the mimicked move.
    name: 'transform_mimic',
    seed: [1, 2, 3, 4],
    p1: [mon('Ditto', 'transform'), mon('Clefairy', 'mimic,tackle')],
    p2: [mon('Snorlax', 'tackle,growl,harden,swordsdance')],
    choices: [
      ['move 1', 'move 3'], ['move 2', 'move 1'], ['switch 2', 'move 2'], ['move 1', 'move 1'],
      ['move 1', 'move 1'], ['switch 2', 'move 1'], ['move 1', 'move 4'], ['move 1', 'move 1'],
    ],
  },
  {
    // Mirror Move with no foe move (fails), then copying Tackle and
    // Growl; Counter after a switch (succeeds), after Growl (fails), with
    // no damage dealt (fails), after Karate Chop, and Counter against
    // Counter.
    name: 'counter_mirrormove',
    seed: [1, 2, 3, 4],
    p1: [mon('Pidgeot', 'mirrormove'), mon('Hitmonchan', 'counter,growl')],
    p2: [mon('Snorlax', 'tackle,growl'), mon('Machamp', 'counter,karatechop')],
    choices: [
      ['move 1', 'move 1'], ['move 1', 'move 2'], ['move 1', 'move 1'], ['switch 2', 'move 1'],
      ['move 1', 'move 1'], ['move 1', 'move 2'], ['move 2', 'switch 2'], ['move 2', 'move 1'],
      ['move 1', 'move 2'], ['move 1', 'move 1'],
    ],
  },
  {
    // Metronome on both sides; seed 223 draws only ported moves (found by
    // search: most seeds call a move whose port is another batch's).
    name: 'metronome',
    seed: [223, 2, 3, 4],
    p1: [mon('Clefable', 'metronome')],
    p2: [mon('Chansey', 'metronome')],
    choices: [['move 1', 'move 1'], ['move 1', 'move 1'], ['move 1', 'move 1']],
  },
];

function play(sc) {
  const out = [];
  const battle = new Battle({
    formatid: FORMAT, seed: sc.seed,
    send: (type, data) => { if (type === 'update') out.push(Array.isArray(data) ? data.join('\n') : data); },
  });
  battle.trunc = (num, bits = 0) => (bits ? (num >>> 0) % (2 ** bits) : num >>> 0);
  battle.setPlayer('p1', { name: 'Player 1', team: sc.p1.join(']') });
  battle.setPlayer('p2', { name: 'Player 2', team: sc.p2.join(']') });
  battle.sendUpdates();
  for (const [a, b] of sc.choices) {
    if (a) battle.choose('p1', a);
    if (b) battle.choose('p2', b);
    battle.sendUpdates();
  }
  return out.join('\n').split('\n').filter((l) => !l.startsWith('|t:|')).join('\n');
}

function main() {
  const out = SCENARIOS.map((sc) => {
    return { name: sc.name, format: FORMAT, seed: sc.seed, p1: sc.p1.join(']'), p2: sc.p2.join(']'), choices: sc.choices, log: play(sc) };
  });
  const file = path.join(__dirname, '..', '..', 'crates', 'ps_sim', 'tests', 'fixtures', 'scripted_gen1.json');
  fs.writeFileSync(file, JSON.stringify(out, null, 1) + '\n');
  process.stdout.write(`wrote ${out.length} scenarios to ${file}\n`);
}

main();
