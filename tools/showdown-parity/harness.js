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

function teamMon(dex, m, slot) {
  const species = dex.species.get(m.species);
  if (!species.exists) throw new Error(`no species ${m.species}`);
  return {
    // In battle mode every team member carries its ORIGINAL slot as a
    // nickname. The sim swaps entries around inside side.pokemon whenever
    // something switches, so a bare index is not a stable address; the
    // nickname is, and the core engines keep `party` in its original order.
    name: slot === undefined ? species.name : `m${slot}`,
    species: species.name,
    level: m.level ?? 50,
    nature: m.nature ?? 'Hardy',
    // Only the abilities the core engines model are ever handed out; a
    // scenario that leaves this blank is asking for a mon with none, which
    // is what the sim calls 'noability'.
    ability: m.ability || 'noability',
    ivs: m.ivs ?? {hp: 31, atk: 31, def: 31, spa: 31, spd: 31, spe: 31},
    evs: m.evs ?? {hp: 0, atk: 0, def: 0, spa: 0, spd: 0, spe: 0},
    moves: m.moves,
    // Only the items the core engines model are ever handed out; blank asks
    // for a mon holding nothing.
    item: m.item ?? '',
  };
}

/// The format the core engines actually play. Gen 1 runs under the random-
/// battle clauses — a second sleep or freeze from an enemy move fails while
/// one is already on that side — and `customgame` carries neither, so they
/// are added explicitly rather than left to diverge.
function formatFor(gen) {
  return gen === 1
    ? 'gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod'
    : `gen${gen}customgame`;
}

function newBattle(gen, p1mon, p2mon) {
  const dex = Dex.mod(`gen${gen}`);
  const battle = new Battle({formatid: formatFor(gen)});
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
    battle.__actMon = pokemon;
    // Per-action once-only rolls: confusion's (1,2) and full paralysis's
    // (1,4) fire at most once each in onBeforeMove; later same-shape rolls
    // are Protect's stall ladder.
    battle.__confRolled = false;
    battle.__paraRolled = false;
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
    // Shed Skin's end-of-turn third of a chance is not one of the scenario's
    // knobs, and it fires outside any move — after an accuracy window that a
    // never-rolling move may have left open, and with `__cur` still holding
    // whichever action ran last. Pin it off first, but ONLY outside a move:
    // an accuracy of 33 is a real number a move can have (Poison Gas at two
    // stages down is exactly that), and swallowing its roll here made the
    // reference miss where the script said hit.
    if (numerator === 33 && denominator === 100 && !battle.activeMove) return false;
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
    // Confusion: randomChance(1,2) in gens 3-4 (once, in onBeforeMove,
    // only for a confused actor), randomChance(128,256) in gen 1 — true
    // means "acts normally", false the 40 BP self-hit.
    if (((numerator === 1 && denominator === 2 &&
          battle.__actMon?.volatiles['confusion'] && !battle.__confRolled) ||
         (numerator === 128 && denominator === 256))) {
      battle.__confRolled = true;
      return !((battle.__act ?? battle.__cur).selfhit);
    }
    // Full paralysis: (1,4) in gen 3+ (once, in onBeforeMove, only for a
    // paralyzed actor), (63,256) in gens 1-2. Later (1,4)s are Protect's
    // stall ladder.
    if (numerator === 1 && denominator === 4 &&
        battle.__actMon?.status === 'par' && !battle.__paraRolled) {
      battle.__paraRolled = true;
      return battle.__act?.immobile ?? false;
    }
    if (numerator === 63 && denominator === 256) {
      return battle.__act?.immobile ?? false;
    }
    // Protect/Detect/Endure's consecutive-use stall ladder: 1/2, 1/4, 1/8
    // in this era. Reached for (1,2) only when the actor is not confused
    // (that roll went to the selfhit knob above) and for (1,4)/(1,8) after
    // the paralysis check. The script's stall knob decides.
    if (numerator === 1 && (denominator === 2 || denominator === 4 || denominator === 8)) {
      return (battle.__act ?? battle.__cur).stall ?? false;
    }
    // After the accuracy roll, a sub-certain chance out of 100 (gen 3+) or
    // 256 (gen 1) during a move is its secondary proc; the script decides.
    // Thaw (1,5) and wake rolls have other denominators and stay pinned
    // off, matching the engines' scripted turns.
    if ((denominator === 100 || denominator === 256) && numerator < denominator) {
      return battle.__cur.secondary ?? false;
    }
    // A CERTAIN chance (Fake Out's 100% flinch) is no roll at all.
    if ((denominator === 100 || denominator === 256) && numerator >= denominator) {
      return true;
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
      // One-arg random(n) compared against a chance: secondary procs, and
      // Magnitude/Psywave's spreads (rolled in onModifyMove, BEFORE __cur
      // updates — hence the acting seat's knob). Scripted proc returns the
      // bottom, otherwise the top.
      return ((battle.__act ?? battle.__cur).secondary) ? 0 : Math.max(0, a - 1);
    }
    if (a === 1 && b > 9) {
      // Gen 1 Psywave's random(1, 1.5*level): the secondary knob picks the
      // floor (1) or the ceiling (b-1). Sleep turns are random(1,8) and
      // keep the minimum below.
      return ((battle.__act ?? battle.__cur).secondary) ? 1 : b - 1;
    }
    return a; // two-arg minimums: multi-hit counts, sleep turns
  };

  // Crits: pin willCrit on a copy of the move, which every gen's getDamage
  // honours in place of rolling. The copy must PRESERVE runtime mutations
  // (Weather Ball's onModifyMove retype/redouble, Future Sight's fixed
  // launch damage) — a fresh dex copy silently reverted them, which warped
  // several move families (fuzz-found via Weather Ball in sun).
  const origGetDamage = actions.getDamage.bind(actions);
  actions.getDamage = function (source, target, move, suppressMessages) {
    battle.__cur = forSide(source);
    // Only real moves get the willCrit-pinned copy: secondary/self-boost
    // moveData blobs carry no id, and swapping them for a broken active
    // move silently ate their boosts (fuzz-found via Metal Claw).
    if (typeof move === 'string') {
      const active = this.dex.getActiveMove(move);
      if (active.willCrit === undefined) active.willCrit = forSide(source).crit;
      move = active;
    } else if (typeof move !== 'number' && move.id) {
      const copy = Object.assign(Object.create(Object.getPrototypeOf(move)), move);
      // Moves that inherently never crit (Counter) keep that; only an
      // undecided willCrit takes the script's.
      if (copy.willCrit === undefined) copy.willCrit = forSide(source).crit;
      move = copy;
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
/// A whole battle: six-mon teams, switching, played until someone wins or
/// the scripted turns run out.
///
/// Switch targets are named by ORIGINAL team slot, translated here to the
/// sim's live `side.pokemon` index via the per-slot nickname. Forced
/// replacements after a faint follow the same rule the core engines use --
/// the lowest-numbered living slot -- so neither side needs to be asked.
function runBattle(sc) {
  const dex = Dex.mod(`gen${sc.gen}`);
  const battle = new Battle({formatid: formatFor(sc.gen)});
  battle.setPlayer('p1', {team: sc.p1.team.map((m, i) => teamMon(dex, m, i))});
  battle.setPlayer('p2', {team: sc.p2.team.map((m, i) => teamMon(dex, m, i))});
  normalizePp(battle, dex);

  const script = {};
  scriptRandomness(battle, script);

  for (const id of ['p1', 'p2']) {
    for (const [i, want] of (sc[id].team ?? []).entries()) {
      if (!want.status) continue;
      const mon = battle.getSide(id).pokemon.find((p) => p.name === `m${i}`);
      mon.setStatus(want.status);
      if (sc.gen <= 2 && mon.modifyStat) {
        if (want.status === 'par') mon.modifyStat('spe', 0.25);
        if (want.status === 'brn') mon.modifyStat('atk', 0.5);
      }
    }
  }

  // Live sim index (1-based, as `switch N` wants) of an original team slot.
  const liveIndex = (sideId, slot) => {
    const arr = battle.getSide(sideId).pokemon;
    const i = arr.findIndex((p) => p.name === `m${slot}`);
    return i < 0 ? null : i + 1;
  };
  // The replacement rule both engines use: lowest living slot not already out.
  const forcedChoice = (sideId) => {
    const side = battle.getSide(sideId);
    const arr = side.pokemon;
    let best = null;
    for (let slot = 0; slot < arr.length; slot++) {
      const p = arr.find((q) => q.name === `m${slot}`);
      if (!p || p.fainted || p.isActive) continue;
      best = slot;
      break;
    }
    return best === null ? 'pass' : `switch ${liveIndex(sideId, best)}`;
  };

  const seatChoice = (sideId, seat) => {
    if (!seat) return 'move 1';
    if (seat.action === 'switch') {
      const idx = liveIndex(sideId, seat.slot);
      if (idx === null) return 'move 1';
      return `switch ${idx}`;
    }
    // A locked mon's request offers exactly one move (the lock itself), so
    // any higher slot is out of range and the whole choice is rejected. The
    // core engines ignore the chosen index in the same situation.
    const offered = battle.getSide(sideId).activeRequest?.active?.[0]?.moves?.length ?? 4;
    const slot = Math.min(seat.slot ?? 0, Math.max(offered - 1, 0));
    return `move ${slot + 1}`;
  };

  const partyOf = (sideId) => {
    const arr = battle.getSide(sideId).pokemon;
    return (sc[sideId].team ?? []).map((_, i) => {
      const p = arr.find((q) => q.name === `m${i}`);
      return {...endMon(p), active: p.isActive};
    });
  };
  const snapshot = () => ({p1: partyOf('p1'), p2: partyOf('p2')});

  const errors = [];
  const states = [];
  // A rejected choice says only "Not all choices done" on its own; pair it
  // with what was asked for and what the sim was actually requesting.
  const describe = (attempted, e) => {
    const req = (id) => {
      const r = battle.getSide(id).activeRequest;
      if (!r) return 'none';
      if (r.forceSwitch) return `forceSwitch:${JSON.stringify(r.forceSwitch)}`;
      if (r.wait) return 'wait';
      const a = r.active?.[0];
      return `move trapped=${!!a?.trapped} moves=${(a?.moves ?? [])
        .map((m) => `${m.id}${m.disabled ? '(off)' : ''}:${m.pp}`)
        .join(',')}`;
    };
    return `${String(e.message || e)} | tried ${attempted} | p1 ${req('p1')} | p2 ${req('p2')}`;
  };
  for (const ts of sc.turns ?? []) {
    if (battle.ended) break;
    if (ts.p1) script.p1 = ts.p1;
    if (ts.p2) script.p2 = ts.p2;
    const c1 = seatChoice('p1', ts.p1);
    const c2 = seatChoice('p2', ts.p2);
    try {
      battle.makeChoices(c1, c2);
    } catch (e) {
      errors.push(describe(`${c1} / ${c2}`, e));
      break;
    }
    // Faints open a replacement request; answer it the same way the core
    // engines do until the battle stops asking.
    let guard = 0;
    while (!battle.ended && guard++ < 12) {
      const need1 = battle.p1.activeRequest?.forceSwitch?.some(Boolean);
      const need2 = battle.p2.activeRequest?.forceSwitch?.some(Boolean);
      if (!need1 && !need2) break;
      // Answer ONLY the side that was asked. Handing the other side a
      // 'pass' through makeChoices wipes the move it already has queued,
      // which silently ate an action every time a mon fainted mid-turn.
      const f1 = need1 ? forcedChoice('p1') : null;
      const f2 = need2 ? forcedChoice('p2') : null;
      try {
        if (f1) battle.choose('p1', f1);
        if (f2) battle.choose('p2', f2);
      } catch (e) {
        errors.push(describe(`forced ${f1} / ${f2}`, e));
        break;
      }
    }
    states.push(snapshot());
  }

  const order = battle.log
    .filter((l) => l.startsWith('|move|'))
    .map((l) => (l.includes('|move|p1a') ? 'p1' : 'p2'));

  return {
    p1: partyOf('p1'),
    p2: partyOf('p2'),
    states,
    order,
    ended: battle.ended,
    winner: battle.ended ? (battle.winner || null) : null,
    errors,
    log: battle.log.filter((l) =>
      /move|damage|crit|supereffective|resisted|immune|miss|faint|switch|win|drag/.test(l)),
  };
}

function runDump(sc) {
  const dex = Dex.mod(`gen${sc.gen}`);
  const species = dex.species.all()
    .filter((s) => s.exists && !s.isNonstandard && s.num > 0)
    .map((s) => ({
      id: s.id,
      name: s.name,
      types: s.types,
      baseStats: s.baseStats,
      weightkg: s.weightkg,
    }));
  const moves = dex.moves.all()
    .filter((m) => m.exists && !m.isNonstandard && m.id !== 'struggle')
    .map((m) => {
      // A move's one modelled secondary: a status, stat drops on the target,
      // or a flinch. Self-boosts, other volatiles and hook-driven
      // secondaries (Tri Attack picks its status in an onHit) stay null.
      let secondary = null;
      if (m.id === 'triattack') secondary = {chance: 20, tri: true};
      if (m.id === 'secretpower') secondary = {chance: 30, status: 'par'};
      const secs = m.secondaries ?? (m.secondary ? [m.secondary] : []);
      for (const s of secs) {
        if (!s || !s.chance || s.onHit) continue;
        if (s.self && s.self.boosts && !s.self.volatileStatus &&
            !s.status && !s.boosts && !s.volatileStatus) {
          secondary = {chance: s.chance, selfBoosts: s.self.boosts};
          break;
        }
        if (s.self) continue;
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
        // Whether the move needs a LIVING foe to go off at all. The sim's
        // useMove computes lacksTarget for every non-field target and bails
        // with -notarget; a self- or field-aimed move never does.
        needsTarget: !['self', 'all', 'foeSide', 'allySide', 'allyTeam'].includes(m.target),
        // Whether a shield stops it. Most self- and field-aimed moves carry
        // no protect flag, and neither do the delayed hits.
        protectable: !!(m.flags && m.flags.protect),
        // Which called-move lists refuse to carry it.
        noSleepTalk: !!(m.flags && m.flags.nosleeptalk),
        noAssist: !!(m.flags && m.flags.noassist),
        selfDrop: m.self && m.self.boosts && m.category !== 'Status' ? m.self.boosts : null,
        statusAction: m.category !== 'Status' ? null
          : m.id === 'haze' ? {haze: true}
          : ['moonlight', 'morningsun', 'synthesis'].includes(m.id) ? {wheal: true}
          : m.id === 'refresh' ? {refresh: true}
          : m.id === 'bellydrum' ? {bellydrum: true}
          : m.id === 'psychup' ? {psychup: true}
          : m.id === 'yawn' ? {yawn: true}
          : m.id === 'wish' ? {wish: true}
          : m.id === 'perishsong' ? {perish: true}
          : m.id === 'destinybond' ? {destiny: true}
          : ['block', 'meanlook', 'spiderweb'].includes(m.id) ? {meanlook: true}
          : m.id === 'mudsport' ? {sport: 'mud'}
          : m.id === 'watersport' ? {sport: 'water'}
          : m.id === 'spikes' ? {spikes: true}
          : m.id === 'memento' ? {memento: true}
          : m.id === 'painsplit' ? {painsplit: true}
          : m.id === 'taunt' ? {taunt: true}
          : m.id === 'nightmare' ? {nightmare: true}
          : m.id === 'stockpile' ? {stockpile: true}
          : m.id === 'swallow' ? {swallow: true}
          : ['protect', 'detect'].includes(m.id) ? {protect: true}
          : m.id === 'endure' ? {endure: true}
          : ['foresight', 'odorsleuth'].includes(m.id) ? {identify: true}
          : ['lockon', 'mindreader'].includes(m.id) ? {lockon: true}
          : m.id === 'charge' ? {charge: true}
          : m.id === 'spite' ? {spite: true}
          : m.id === 'grudge' ? {grudge: true}
          : m.id === 'torment' ? {torment: true}
          : m.id === 'encore' ? {encore: true}
          : m.id === 'disable' ? {disable: true}
          : m.id === 'naturepower' ? {naturepower: true}
          : m.id === 'camouflage' ? {camouflage: true}
          : m.id === 'curse' ? {curse: true}
          : m.id === 'conversion2' ? {conversion2: true}
          : m.id === 'ingrain' ? {ingrain: true}
          : ['healbell', 'aromatherapy'].includes(m.id) ? {healbell: true}
          : m.id === 'followme' ? {noopsuccess: true}
          : ['roar', 'whirlwind'].includes(m.id) && sc.gen >= 3 ? {forceswitch: true}
          : m.id === 'sleeptalk' && sc.gen >= 3 ? {sleeptalk: true}
          : m.id === 'assist' && sc.gen >= 3 ? {assist: true}
          : m.id === 'batonpass' && sc.gen >= 3 ? {batonpass: true}
          : m.id === 'conversion' ? {conversion: true}
          : m.id === 'imprison' ? {imprison: true}
          : ['recycle', 'trick', 'roleplay', 'skillswap'].includes(m.id)
            ? {noopfail: m.id}
          : ['assist', 'sleeptalk'].includes(m.id) ? {noopfail: m.id}
          : m.id === 'magiccoat' ? {magiccoat: true}
          : m.id === 'snatch' ? {snatch: true}
          : m.id === 'mirrormove' ? {mirror: true}
          : m.id === 'mimic' ? {mimic: true}
          : m.id === 'sketch' ? {sketch: true}
          : m.id === 'transform' ? {transform: true}
          : m.id === 'rest' ? {rest: true}
          : m.id === 'focusenergy' ? {focus: true}
          : m.id === 'minimize' ? {minimize: true}
          : m.id === 'defensecurl' ? {boosts: m.boosts, self: true}
          : m.volatileStatus === 'confusion' && m.boosts ? {boosts: m.boosts, confuse: true}
          : m.status ? {status: m.status}
          : m.heal ? {heal: m.heal}
          : m.boosts && !m.volatileStatus ? {boosts: m.boosts, self: m.target === 'self'}
          : m.volatileStatus === 'confusion' && !m.boosts ? {confuse: true}
          : ['reflect', 'lightscreen', 'safeguard', 'mist'].includes(m.sideCondition)
            ? {side: m.sideCondition}
          : m.id === 'leechseed' ? {seed: true}
          : ['sunnyday', 'raindance', 'sandstorm', 'hail'].includes(m.id)
            ? {weather: m.id}
          : m.id === 'substitute' ? {sub: true}
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
        trap: m.volatileStatus === 'partiallytrapped',
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
        !raw.status && !raw.boosts && !raw.heal;
      const seed = raw.volatileStatus === 'leechseed' && move.id === 'leechseed';
      // These carry hooks that either self-guard by generation (the powders'
      // Grass immunity is gen 6+) or ARE the modelled effect (Rest, Focus
      // Energy, Swagger's boost+confuse pair).
      const allowlisted = (sc.gen >= 3 ? [
        'sleeppowder', 'stunspore', 'poisonpowder', 'spore', 'cottonspore',
        'growth', 'toxic', 'swagger', 'flatter', 'defensecurl', 'minimize',
        'focusenergy', 'rest', 'splash', 'teleport', 'substitute', 'haze',
        'moonlight', 'morningsun', 'synthesis', 'refresh', 'bellydrum',
        'psychup', 'yawn', 'wish', 'perishsong', 'destinybond', 'block',
        'meanlook', 'spiderweb', 'mudsport', 'watersport', 'spikes',
        'memento', 'painsplit', 'taunt', 'nightmare', 'stockpile', 'swallow',
        'protect', 'detect', 'endure', 'foresight', 'odorsleuth', 'lockon',
        'mindreader', 'charge', 'spite', 'grudge', 'torment', 'encore',
        'disable', 'naturepower', 'camouflage', 'conversion', 'imprison',
        'curse', 'conversion2', 'ingrain', 'healbell', 'aromatherapy',
        'followme', 'roar', 'whirlwind', 'batonpass', 'teeterdance',
        'metronome', 'mirrormove', 'magiccoat', 'snatch',
        'assist', 'sleeptalk', 'recycle', 'trick', 'roleplay', 'skillswap',
        'mimic', 'sketch', 'transform',
      ] : [
        // Gen 1: the cartridge engine implements all of these; their sim
        // hooks are the era mechanics themselves.
        'recover', 'softboiled', 'reflect', 'lightscreen', 'haze', 'growth',
        'defensecurl', 'minimize', 'focusenergy', 'splash', 'teleport',
        'toxic', 'poisonpowder', 'sleeppowder', 'stunspore', 'substitute', 'rest',
        'roar', 'whirlwind', 'spore', 'mist', 'conversion', 'disable',
        'mimic', 'transform', 'mirrormove', 'metronome',
      ]).includes(move.id);
      const weather = sc.gen >= 3 &&
        ['sunnyday', 'raindance', 'sandstorm', 'hail'].includes(move.id);
      const teamCond = sc.gen >= 3 &&
        ['reflect', 'lightscreen', 'safeguard', 'mist'].includes(raw.sideCondition);
      const entangled = (hooky && !seed) || (raw.volatileStatus && !confuseOnly && !seed) ||
        (raw.sideCondition && !teamCond) ||
        (raw.weather && !weather) || raw.forceSwitch || raw.selfSwitch || raw.pseudoWeather ||
        raw.slotCondition || raw.terrain || raw.self || raw.selfdestruct || raw.ohko;
      const modelable = raw.status || raw.boosts || raw.heal || confuseOnly || teamCond || seed || weather;
      if (!allowlisted && (entangled || !modelable)) continue;
      if (!['normal', 'any', 'self', 'allAdjacentFoes', 'allAdjacent', 'allySide', 'allyTeam',
            'all', 'foeSide'].includes(move.target)) continue;
      out.push({id: move.id, priority: move.priority, boostsSelf: false, multihit: false});
      continue;
    }
    // Fixed-damage moves are deterministic and modelled: flat (Sonic Boom),
    // level (Seismic Toss), half-current (Super Fang). Psywave's random
    // callback stays out. OHKO moves KO on their scripted hit.
    const g1plain = sc.gen === 1 && [
      'payday', 'blizzard', 'thunder', 'dreameater', 'highjumpkick',
      'jumpkick', 'lowkick', 'triattack', 'rage', 'psywave',
      'thrash', 'petaldance', 'wrap', 'bind', 'firespin', 'clamp', 'bide',
    ].includes(move.id);
    const fixedDamage = typeof move.damage === 'number' || move.damage === 'level' ||
      move.id === 'superfang';
    // Counter (both eras) and Mirror Coat (gen 3) bounce the turn's damage.
    const counterish = move.id === 'counter' || (sc.gen >= 3 && move.id === 'mirrorcoat');
    // Modelled specials: conditional powers, self-drops, screen-breaking,
    // spin-away, Tri Attack's status pick.
    const g3special = sc.gen >= 3 && [
      'brickbreak', 'payday', 'return', 'frustration', 'falseswipe', 'facade',
      'smellingsalts', 'eruption', 'waterspout', 'pursuit', 'rapidspin',
      'revenge', 'focuspunch', 'triattack', 'superpower', 'overheat',
      'psychoboost', 'knockoff', 'thief', 'covet', 'endeavor', 'flail',
      'thrash', 'petaldance', 'outrage', 'rage', 'furycutter', 'snore',
      'bide', 'rollout', 'iceball', 'hiddenpower', 'magnitude',
      'psywave', 'futuresight', 'doomdesire',
      'reversal', 'weatherball', 'secretpower', 'highjumpkick', 'jumpkick',
      'spitup', 'blizzard', 'dreameater', 'fakeout', 'present', 'triplekick',
      'lowkick', 'beatup', 'uproar',
    ].includes(move.id);
    if ((!move.basePower || move.basePower <= 0) && !fixedDamage && !move.ohko && !counterish
        && !g3special && !(g1plain && move.id === 'bide')) continue;
    if (move.basePowerCallback && !g3special) continue;
    if ((move.damageCallback || move.damage) && !fixedDamage && !counterish && !g3special
        && !g1plain) continue;
    if (move.mindBlownRecoil) continue;
    if (move.willCrit !== undefined && !counterish) continue;
    if ((move.hasCrashDamage && !g3special && !g1plain) || move.struggleRecoil) continue;
    if (move.flags['futuremove'] && !g3special) continue;
    // Gen 3 partial traps are modelled; other damaging volatiles are not.
    if (move.volatileStatus && !g1plain &&
        !(move.volatileStatus === 'partiallytrapped' && sc.gen >= 3) &&
        !(move.volatileStatus === 'bide' && sc.gen >= 3)) continue;
    if ((move.sleepUsable || move.id === 'dreameater') &&
        !(move.id === 'dreameater') &&
        !(sc.gen >= 3 && move.id === 'snore')) continue; // fail unless asleep
    const raw = rawOf(move.id);
    const rawSecs = raw.secondaries ?? (raw.secondary ? [raw.secondary] : []);
    const selfBoostOk = (sec) => sc.gen >= 3 && sec.self && sec.self.boosts &&
      !sec.self.volatileStatus && !sec.status && !sec.boosts && !sec.volatileStatus;
    if (!g3special && rawSecs.some((sec) =>
      sec && (sec.onHit || (sec.self && !selfBoostOk(sec)) ||
        (sec.volatileStatus && sec.volatileStatus !== 'flinch' &&
         sec.volatileStatus !== 'confusion'))
    )) continue;
    // Conditional base power, self-effects (Superpower's drop, Overheat's),
    // and on-hit hooks are behaviour the core engines do not model.
    // Any on* hook means conditional behaviour (Facade's doubling is an
    // onBasePower handler, not a callback property).
    // Charge/recharge machinery lives in on* hooks; those moves' hooks are
    // exactly the machinery being modelled, so they skip the hook filter.
    // (Solar Beam's weather halving rides along unexercised: the fuzz never
    // sets weather.)
    // Thunder's weather-accuracy hook is modelled (never-miss in rain,
    // halved in sun).
    const thundery = sc.gen >= 3 && move.id === 'thunder';
    // Gen 1: coins are cosmetic, and Blizzard/Thunder predate weather.

    const chargey = move.flags['charge'] || move.flags['recharge'];
    const hooky = !chargey && !thundery && !g1plain && !counterish && !g3special &&
      Object.keys(raw).some((k) =>
      k.startsWith('on') ||
      (/Callback/.test(k) && !(k === 'damageCallback' && fixedDamage)));
    // A recharge move's raw.self IS the mustrecharge volatile — machinery,
    // not an unmodelled self-effect.
    if (hooky || (raw.self && !chargey && !g3special && !g1plain)) continue;
    // allAdjacent only differs from allAdjacentFoes in doubles; this is 1v1.
    if (!['normal', 'any', 'randomNormal', 'allAdjacentFoes', 'allAdjacent'].includes(move.target)
        && !(counterish && move.target === 'scripted')
        && !((g3special || g1plain) && move.target === 'self')) continue;
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
        if (sc.kind === 'battle') return runBattle(sc);
        return {error: `unknown kind ${sc.kind}`};
      } catch (e) {
        return {error: String(e && e.stack ? e.stack.split('\n')[0] : e)};
      }
    });
    process.stdout.write(JSON.stringify(out));
  });
}

main();
