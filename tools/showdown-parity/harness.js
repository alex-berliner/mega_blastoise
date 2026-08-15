// Runs the REAL pokemon-showdown simulator over a batch of scenarios and
// reports end states, for parity testing against the core engines.
//
// Randomness is scripted, not disabled: each scenario says whether the move
// hits, whether it crits, and the exact damage roll (85..100), and the sim is
// forced down that branch. The Rust side runs the same script, so every
// random-dependent surface — crit stage rules, misses spending PP, roll
// extremes — is compared deterministically instead of being averaged away.
//
// stdin:  JSON array of scenarios
// stdout: JSON array of results, same order
//
// Scenario kinds:
//   {kind:"stats", gen, species, level, nature, ivs, evs}
//     -> {hp, atk, def, spa, spd, spe}
//   {kind:"chart", gen}
//     -> {types: [...], mult: [[x10 multiplier]]}  (full attack x defend)
//   {kind:"turn", gen, p1:{...mon}, p2:{...mon}, moves:[m1, m2],
//    script:{p1:{hit,crit,roll}, p2:{hit,crit,roll}}}
//     -> {p1: endMon, p2: endMon, order: ["p1"|"p2"...], log: [...]}
//   mon: {species, level, nature?, ivs?, evs?, status?, boosts?, sideConditions?[]}
//   endMon: {hp, maxhp, pp: [..], status, fainted}

'use strict';

const {Battle, Dex} = require('pokemon-showdown/dist/sim');

function teamMon(dex, m) {
  const species = dex.species.get(m.species);
  if (!species.exists) throw new Error(`no species ${m.species}`);
  return {
    name: species.name,
    species: species.name,
    level: m.level ?? 50,
    nature: m.nature ?? 'Hardy',
    ability: species.abilities['0'] ?? '',
    ivs: m.ivs ?? {hp: 31, atk: 31, def: 31, spa: 31, spd: 31, spe: 31},
    evs: m.evs ?? {hp: 0, atk: 0, def: 0, spa: 0, spd: 0, spe: 0},
    moves: m.moves,
    item: '',
  };
}

function newBattle(gen, p1mon, p2mon) {
  const dex = Dex.mod(`gen${gen}`);
  const battle = new Battle({formatid: `gen${gen}customgame`});
  battle.setPlayer('p1', {team: [teamMon(dex, p1mon)]});
  battle.setPlayer('p2', {team: [teamMon(dex, p2mon)]});
  return battle;
}

/// Force the scripted branch for both sides. `script` is
/// {p1:{hit,crit,roll}, p2:{hit,crit,roll}}.
///
/// Old-gen mods (gen1-4) do not use the modern hit-step pipeline: accuracy is
/// rolled inline in the mod's own tryMoveHit via battle.randomChance, and the
/// damage roll via battle.randomizer. So the hooks live at those three seams:
/// tryMoveHit (to know whose script applies), randomChance (accuracy), and
/// randomizer (the 85..100 roll); crits ride getDamage's willCrit.
function scriptRandomness(battle, script) {
  const forSide = (source) => script[source.side.id] ?? {hit: true, crit: false, roll: 100};
  battle.__cur = {hit: true, crit: false, roll: 100};

  const actions = battle.actions;

  // Whose move is resolving right now. Old gens go through the mod's own
  // tryMoveHit; modern gens through spreadMoveHit — hook whichever exists.
  for (const name of ['tryMoveHit', 'trySpreadMoveHit']) {
    const orig = actions[name];
    if (!orig) continue;
    actions[name] = function (targetOrTargets, pokemon, move, ...rest) {
      battle.__cur = forSide(pokemon);
      // One accuracy roll is pending for this move, unless it cannot miss.
      battle.__accPending = move.accuracy !== true;
      return orig.call(this, targetOrTargets, pokemon, move, ...rest);
    };
  }

  // Accuracy: the first chance rolled out of 100 (gen 3+) or 256 (gen 1)
  // after a move starts IS the accuracy roll; it answers from the script.
  // Everything else random (secondary procs, full paralysis, crit rolls that
  // escaped willCrit) is pinned to "no".
  battle.randomChance = (numerator, denominator) => {
    if (battle.__accPending && (denominator === 100 || denominator === 256)) {
      battle.__accPending = false;
      return battle.__cur.hit;
    }
    return false;
  };
  battle.prng.randomChance = () => false;

  // The damage roll: gen 3+ multiplies by roll/100 via battle.randomizer;
  // gen 1 multiplies by battle.random(217, 256)/255, so there the script's
  // roll is the 217..255 value itself.
  battle.randomizer = (baseDamage) => Math.floor((baseDamage * battle.__cur.roll) / 100);
  battle.random = (a, b) => {
    if (a === 217 && b === 256) {
      return Math.min(255, Math.max(217, battle.__cur.roll));
    }
    return a; // other uses (multi-hit counts) pin to the minimum
  };

  // Crits: pin willCrit on an active copy of the move, which every gen's
  // getDamage honours in place of rolling.
  const origGetDamage = actions.getDamage.bind(actions);
  actions.getDamage = function (source, target, move, suppressMessages) {
    battle.__cur = forSide(source);
    if (typeof move !== 'number') {
      const active = this.dex.getActiveMove(typeof move === 'string' ? move : move.id);
      active.willCrit = forSide(source).crit;
      move = active;
    }
    return origGetDamage(source, target, move, suppressMessages);
  };
}

/// Strip the sim's default 3 PP Ups so pp compares against base PP directly.
function normalizePp(battle, dex) {
  for (const side of battle.sides) {
    for (const mon of side.pokemon) {
      for (const slot of mon.moveSlots) {
        const base = dex.moves.get(slot.id).pp;
        slot.maxpp = base;
        slot.pp = base;
      }
    }
  }
}

function endMon(pokemon) {
  return {
    hp: pokemon.hp,
    maxhp: pokemon.maxhp,
    pp: pokemon.moveSlots.map((s) => s.pp),
    status: pokemon.status || null,
    fainted: pokemon.fainted,
  };
}

function runStats(sc) {
  const b = newBattle(sc.gen, {...sc, moves: ['splash']}, {species: 'Rattata', level: 5, moves: ['splash']});
  const p = b.p1.pokemon[0];
  return {hp: p.maxhp, ...p.storedStats};
}

function runChart(sc) {
  const dex = Dex.mod(`gen${sc.gen}`);
  const types = dex.types.all().filter((t) => !t.isNonstandard).map((t) => t.name);
  const mult = types.map((atk) =>
    types.map((def) => {
      if (!dex.getImmunity(atk, [def])) return 0;
      const e = dex.getEffectiveness(atk, [def]); // log2 of the multiplier
      return Math.round(10 * 2 ** e);
    })
  );
  return {types, mult};
}

function runTurn(sc) {
  const b = newBattle(sc.gen, {...sc.p1, moves: [sc.moves[0]]}, {...sc.p2, moves: [sc.moves[1]]});
  normalizePp(b, Dex.mod(`gen${sc.gen}`));
  scriptRandomness(b, sc.script ?? {});
  for (const id of ['p1', 'p2']) {
    const mon = b.getSide(id).pokemon[0];
    const want = sc[id];
    if (want.status) mon.setStatus(want.status);
    if (want.boosts) b.boost(want.boosts, mon, mon, null, true, true);
    for (const cond of want.sideConditions ?? []) {
      b.getSide(id).addSideCondition(cond, mon);
    }
  }
  b.makeChoices('move 1', 'move 1');
  const order = b.log
    .filter((l) => l.startsWith('|move|'))
    .map((l) => (l.includes('|move|p1a') ? 'p1' : 'p2'));
  return {
    p1: endMon(b.p1.pokemon[0]),
    p2: endMon(b.p2.pokemon[0]),
    order,
    log: b.log.filter((l) => /move|damage|crit|supereffective|resisted|immune|miss|faint|win/.test(l)),
  };
}

function main() {
  let input = '';
  process.stdin.on('data', (d) => (input += d));
  process.stdin.on('end', () => {
    const scenarios = JSON.parse(input);
    const out = scenarios.map((sc) => {
      try {
        if (sc.kind === 'stats') return runStats(sc);
        if (sc.kind === 'chart') return runChart(sc);
        if (sc.kind === 'turn') return runTurn(sc);
        return {error: `unknown kind ${sc.kind}`};
      } catch (e) {
        return {error: String(e && e.stack ? e.stack.split('\n')[0] : e)};
      }
    });
    process.stdout.write(JSON.stringify(out));
  });
}

main();
