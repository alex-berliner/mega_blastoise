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
    // The core engines model no abilities yet, so the reference sim runs
    // without them too: otherwise Pressure, Levitate and Huge Power dominate
    // every diff with knowingly-unbuilt behaviour.
    ability: 'noability',
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
  const forSide = (source) =>
    script[source.side.id] ?? {hit: true, crit: false, roll: 100, secondary: false};
  battle.__cur = {hit: true, crit: false, roll: 100};

  const actions = battle.actions;

  // Whose ACTION is starting — set earlier than tryMoveHit, because the
  // full-paralysis roll happens in onBeforeMove, before any hit step.
  const origRun = actions.runMove;
  actions.runMove = function (moveOrMoveName, pokemon, ...rest) {
    battle.__act = forSide(pokemon);
    // A new action begins: any stale pending-accuracy flag is dead. (A
    // type-immune gen 1 move skips its accuracy roll entirely, and the
    // stale flag would otherwise eat the next action's paralysis roll.)
    battle.__accPending = false;
    return origRun.call(this, moveOrMoveName, pokemon, ...rest);
  };

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
    // Gen 1/2 crit: rolled inside getDamage as randomChance(critChance, 256);
    // the wrapper below flags the window so this roll never reads the
    // accuracy or secondary knobs.
    if (battle.__critPending && denominator === 256) {
      battle.__critPending = false;
      return battle.__cur.crit;
    }
    // Confusion (gens 3-4): randomChance(1,2) true means "acts normally",
    // false means the 40 BP self-hit. The script's selfhit knob decides.
    if (numerator === 1 && denominator === 2) {
      return !((battle.__act ?? battle.__cur).selfhit);
    }
    // Full paralysis: rolled in onBeforeMove for the acting side —
    // (1,4) in gen 3+, (63,256) in gens 1-2. Checked before the
    // sub-certain branch so the par roll never reads the secondary knob.
    if ((numerator === 1 && denominator === 4) ||
        (numerator === 63 && denominator === 256)) {
      return battle.__act?.immobile ?? false;
    }
    // After the accuracy roll, a sub-certain chance out of 100 (gen 3+) or
    // 256 (gen 1) during a move is its secondary proc; the script decides.
    // Thaw (1,5) and wake rolls have other denominators and stay pinned
    // off, matching the engines' scripted turns.
    if ((denominator === 100 || denominator === 256) && numerator < denominator) {
      return battle.__cur.secondary ?? false;
    }
    return false;
  };
  battle.prng.randomChance = () => false;
  // Speed ties: the sim shuffles tied actors with its own PRNG. Pin the
  // shuffle to a no-op so a tie keeps insertion order (p1 first), which the
  // engines mirror under a script.
  battle.prng.shuffle = () => {};

  // The 2-5 multi-hit count is drawn via battle.sample from a weighted
  // list; the acting seat's script decides it. Anything else sampled is
  // pinned to the first entry.
  battle.sample = (items) => {
    if (Array.isArray(items) && items.length === 8 && items[0] === 2 && items[7] === 5) {
      const hits = (battle.__act ?? battle.__cur).hits;
      return hits || 2;
    }
    return items[0];
  };

  // The damage roll: gen 3+ multiplies by roll/100 via battle.randomizer;
  // gen 1 multiplies by battle.random(217, 256)/255, so there the script's
  // roll is the 217..255 value itself.
  battle.randomizer = (baseDamage) => Math.floor((baseDamage * battle.__cur.roll) / 100);
  battle.random = (a, b) => {
    if (a === 217 && b === 256) {
      return Math.min(255, Math.max(217, battle.__cur.roll));
    }
    if (b === undefined) {
      // One-arg random(n) compared against a chance: some secondary paths
      // roll this way instead of randomChance. Scripted proc returns the
      // bottom (always under the chance), otherwise the top (never under).
      return battle.__cur.secondary ? 0 : Math.max(0, a - 1);
    }
    return a; // two-arg minimums: multi-hit counts, sleep turns
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
    // Gens 1-2 ignore willCrit and roll their own randomChance(x, 256)
    // inside; flag the window so that roll answers from the crit knob.
    battle.__critPending = true;
    const dmg = origGetDamage(source, target, move, suppressMessages);
    battle.__critPending = false;
    return dmg;
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
  const script = {...(sc.script ?? {})};
  scriptRandomness(b, script);
  for (const id of ['p1', 'p2']) {
    const mon = b.getSide(id).pokemon[0];
    const want = sc[id];
    if (want.status) {
      mon.setStatus(want.status);
      // Gens 1-2 apply the par/brn stat drops at the moment of infliction
      // (scripts.js does it inside the move); a bare setStatus skips them,
      // so a pre-set status re-applies the cartridge drop by hand.
      if (sc.gen <= 2 && mon.modifyStat) {
        if (want.status === 'par') mon.modifyStat('spe', 0.25);
        if (want.status === 'brn') mon.modifyStat('atk', 0.5);
      }
    }
    if (want.boosts) b.boost(want.boosts, mon, mon, null, true, true);
    for (const cond of want.sideConditions ?? []) {
      b.getSide(id).addSideCondition(cond, mon);
    }
  }
  // One turn under sc.script, or several under sc.turns — a list of per-turn
  // {p1, p2} scripts. The scriptRandomness hooks read the same object every
  // roll, so each turn swaps its script in by mutation.
  const turnScripts = sc.turns ?? [script];
  for (const ts of turnScripts) {
    if (b.ended) break;
    if (ts.p1) script.p1 = ts.p1;
    if (ts.p2) script.p2 = ts.p2;
    b.makeChoices('move 1', 'move 1');
  }
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

/// The moves whose WHOLE behaviour both core engines implement: plain,
/// single-hit, fixed-base-power damage. Everything else (recoil, drain,
/// multi-hit, charge/recharge turns, fixed damage, OHKO, self-KO, callbacks)
/// would fuzz unimplemented surface and drown real findings in noise.
/// Secondaries are allowed because the harness pins their rolls off.
/// Era-accurate species and move tables straight from the gen's dex, for the
/// core engines' build scripts to consume. This replaced hand-layering the
/// per-gen delta files, whose direction the fuzzer proved wrong (modern PP
/// counts and even Gen 6 base-stat buffs were leaking into "gen 3" tables).
function runDump(sc) {
  const dex = Dex.mod(`gen${sc.gen}`);
  const species = dex.species.all()
    .filter((s) => s.exists && !s.isNonstandard && s.num > 0)
    .map((s) => ({
      id: s.id,
      name: s.name,
      types: s.types,
      baseStats: s.baseStats,
    }));
  const moves = dex.moves.all()
    .filter((m) => m.exists && !m.isNonstandard && m.id !== 'struggle')
    .map((m) => {
      // A move's one modelled secondary: a status, stat drops on the target,
      // or a flinch. Self-boosts, other volatiles and hook-driven
      // secondaries (Tri Attack picks its status in an onHit) stay null.
      let secondary = null;
      const secs = m.secondaries ?? (m.secondary ? [m.secondary] : []);
      for (const s of secs) {
        if (!s || !s.chance || s.self || s.onHit) continue;
        if (s.volatileStatus === 'flinch') {
          secondary = {chance: s.chance, flinch: true};
          break;
        }
        if (s.volatileStatus === 'confusion') {
          secondary = {chance: s.chance, confusion: true};
          break;
        }
        if (s.volatileStatus) continue;
        if (s.status) {
          secondary = {chance: s.chance, status: s.status};
          break;
        }
        if (s.boosts) {
          secondary = {chance: s.chance, boosts: s.boosts};
          break;
        }
      }
      return {
        id: m.id,
        name: m.name,
        type: m.type,
        basePower: m.basePower,
        accuracy: m.accuracy === true ? 0 : m.accuracy,
        pp: m.pp,
        category: m.category,
        priority: m.priority,
        secondary,
        drain: m.drain ?? null,
        recoil: m.recoil ?? null,
        respectsImmunity: m.category === 'Status' && m.ignoreImmunity === false,
        statusAction: m.category !== 'Status' ? null
          : m.status ? {status: m.status}
          : m.heal ? {heal: m.heal}
          : m.boosts && !m.volatileStatus ? {boosts: m.boosts, self: m.target === 'self'}
          : m.volatileStatus === 'confusion' && !m.boosts ? {confuse: true}
          : null,
        multihit: m.multihit
          ? (Array.isArray(m.multihit) ? m.multihit : [m.multihit, m.multihit])
          : null,
        fixed: typeof m.damage === 'number' ? m.damage
          : m.damage === 'level' ? 'level'
          : m.id === 'superfang' ? 'half'
          : null,
        ohko: !!m.ohko,
        highCrit: (m.critRatio ?? 1) >= 2,
        selfdestruct: m.selfdestruct === 'always',
        charge: !!m.flags['charge'],
        recharge: !!m.flags['recharge'],
      };
    });
  return {species, moves};
}

function runMovelist(sc) {
  const dex = Dex.mod(`gen${sc.gen}`);
  // Dist Move objects strip function-valued properties, so callback-driven
  // moves (Facade's status doubling, Flail's HP curve) are invisible on the
  // dex view. The raw data files still carry them; overlay the gen's mod
  // file on the base data and filter on that.
  const rawBase = require('pokemon-showdown/dist/data/moves').Moves;
  let rawMod = {};
  try {
    rawMod = require(`pokemon-showdown/dist/data/mods/gen${sc.gen}/moves`).Moves;
  } catch (e) { /* no mod overrides for this gen */ }
  const rawOf = (id) => ({...(rawBase[id] ?? {}), ...(rawMod[id] ?? {})});

  const out = [];
  for (const move of dex.moves.all()) {
    if (!move.exists || move.isNonstandard) continue;
    if (move.category === 'Status') {
      // A status move joins the pool when its whole effect is one the
      // engines model — a status, stat stages, or a half heal — with no
      // hooks. (Toxic and the powders carry modern-gen hooks in the base
      // data, so they filter out and stay reference-honest.)
      const raw = rawOf(move.id);
      const hooky = Object.keys(raw).some((k) => k.startsWith('on') || /Callback/.test(k));
      const confuseOnly = raw.volatileStatus === 'confusion' &&
        !raw.status && !raw.boosts && !raw.heal && sc.gen >= 3;
      const entangled = hooky || (raw.volatileStatus && !confuseOnly) || raw.sideCondition ||
        raw.weather || raw.forceSwitch || raw.selfSwitch || raw.pseudoWeather ||
        raw.slotCondition || raw.terrain || raw.self || raw.selfdestruct || raw.ohko;
      const modelable = raw.status || raw.boosts || raw.heal || confuseOnly;
      if (entangled || !modelable) continue;
      if (!['normal', 'any', 'self', 'allAdjacentFoes'].includes(move.target)) continue;
      out.push({id: move.id, priority: move.priority, boostsSelf: false, multihit: false});
      continue;
    }
    // Fixed-damage moves are deterministic and modelled: flat (Sonic Boom),
    // level (Seismic Toss), half-current (Super Fang). Psywave's random
    // callback stays out. OHKO moves KO on their scripted hit.
    const fixedDamage = typeof move.damage === 'number' || move.damage === 'level' ||
      move.id === 'superfang';
    if ((!move.basePower || move.basePower <= 0) && !fixedDamage && !move.ohko) continue;
    if (move.basePowerCallback) continue;
    if ((move.damageCallback || move.damage) && !fixedDamage) continue;
    if (move.mindBlownRecoil) continue;
    if (move.willCrit !== undefined) continue;
    if (move.hasCrashDamage || move.struggleRecoil) continue;
    if (move.flags['futuremove']) continue;
    if (move.volatileStatus) continue; // partial traps tick extra end-of-turn damage
    if (move.sleepUsable || move.id === 'dreameater') continue; // fail unless asleep
    const raw = rawOf(move.id);
    const rawSecs = raw.secondaries ?? (raw.secondary ? [raw.secondary] : []);
    if (rawSecs.some((sec) =>
      sec && (sec.onHit || sec.self ||
        (sec.volatileStatus && sec.volatileStatus !== 'flinch' &&
         !(sec.volatileStatus === 'confusion' && sc.gen >= 3)))
    )) continue;
    // Conditional base power, self-effects (Superpower's drop, Overheat's),
    // and on-hit hooks are behaviour the core engines do not model.
    // Any on* hook means conditional behaviour (Facade's doubling is an
    // onBasePower handler, not a callback property).
    // Charge/recharge machinery lives in on* hooks; those moves' hooks are
    // exactly the machinery being modelled, so they skip the hook filter.
    // (Solar Beam's weather halving rides along unexercised: the fuzz never
    // sets weather.)
    const chargey = move.flags['charge'] || move.flags['recharge'];
    const hooky = !chargey && Object.keys(raw).some((k) =>
      k.startsWith('on') ||
      (/Callback/.test(k) && !(k === 'damageCallback' && fixedDamage)));
    // A recharge move's raw.self IS the mustrecharge volatile — machinery,
    // not an unmodelled self-effect.
    if (hooky || (raw.self && !chargey)) continue;
    // allAdjacent only differs from allAdjacentFoes in doubles; this is 1v1.
    if (!['normal', 'any', 'randomNormal', 'allAdjacentFoes', 'allAdjacent'].includes(move.target)) continue;
    if (move.id === 'struggle') continue;
    out.push({id: move.id, priority: move.priority, boostsSelf: !!move.self, multihit: !!move.multihit});
  }
  return {moves: out};
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
        if (sc.kind === 'movelist') return runMovelist(sc);
        if (sc.kind === 'dump') return runDump(sc);
        return {error: `unknown kind ${sc.kind}`};
      } catch (e) {
        return {error: String(e && e.stack ? e.stack.split('\n')[0] : e)};
      }
    });
    process.stdout.write(JSON.stringify(out));
  });
}

main();
