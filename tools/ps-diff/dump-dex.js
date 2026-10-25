#!/usr/bin/env node
'use strict';
// Dumps the RESOLVED Pokemon Showdown dex of one generation into the JSON
// files `crates/ps_dex/build.rs` compiles into `&'static` tables.
//
//   node tools/ps-diff/dump-dex.js gen1
//   node tools/ps-diff/dump-dex.js gen3
//   node tools/ps-diff/dump-dex.js all       (both)
//
// Everything comes from `Dex.mod(gen)`, so the values are what that gen's sim
// reads: `ModdedDex.loadData` (PS sim/dex.ts) has already walked the mod chain
// (gen1 -> gen2 -> gen3 -> gen4 -> gen5 -> base), merged `inherit: true`
// entries field by field and dropped `null` ones. Reading data/moves.ts
// directly gives the modern definition, which for most effects is not what
// the old gens run.
//
// Which entries: those `dex.<table>.all()` returns with no `isNonstandard`
// (PS marks entries from later gens 'Future' and retired ones 'Past'), in
// `all()` order, which is `for (id in dex.data.<Table>)` order. That order is
// the `u16` id. The counts match design section 6 of
// docs/architecture/showdown-split-plan.md.
//
// Schema: every entry has every key (null where PS has nothing), so the Rust
// side is a straight table and `crates/ps_dex/tests/roundtrip.rs` can check
// each entry field by field. Every non-function key PS has on an entry is
// either dumped or listed in an IGNORED set below; an unknown key, or a known
// key with a shape the schema does not cover, stops the dump. A new PS
// version therefore cannot drop a field silently.

const fs = require('fs');
const path = require('path');
const { Dex } = require('pokemon-showdown/dist/sim');

const PS_VERSION = require('pokemon-showdown/package.json').version;
// The pin is an upstream commit (docs/architecture/showdown-resync.md
// section 9): master's package.json still says 0.11.11, so the dump also
// records the commit the lockfile resolved, or null for an npm release.
const PS_RESOLVED = require('./package-lock.json').packages['node_modules/pokemon-showdown'].resolved;
const PS_COMMIT = (/\/tar\.gz\/([0-9a-f]{40})$/.exec(PS_RESOLVED) || [null, null])[1];
const DATA_ROOT = path.resolve(__dirname, '../../crates/ps_dex/data');

// PS's toID (sim/dex-data.ts).
function toID(text) {
	return `${text}`.toLowerCase().replace(/[^a-z0-9]+/g, '');
}

function fail(where, message) {
	throw new Error(`${where}: ${message}`);
}

// ---------------------------------------------------------------- handlers

// The keys that sit beside a handler and order it (PS sim/battle.ts
// `resolvePriority` reads `${callbackName}Order`, `...Priority`,
// `...SubOrder`). They are data of the handler, not handlers.
const ORDER_SUFFIX = /(Priority|Order|SubOrder)$/;

/** True for a key PS dispatches on: any function-valued key (event handlers
 * `onX`, and the callbacks `basePowerCallback`, `damageCallback`,
 * `beforeMoveCallback`, `beforeTurnCallback`, `priorityChargeCallback`,
 * `durationCallback`), plus an `onX` key holding a constant. PS's `runEvent`
 * returns a non-function handler's value as is (`onTry: false`,
 * `onCriticalHit: false`, `onLockMove: 'recharge'`). */
function isHandler(key, value) {
	if (value === undefined) return false;
	if (typeof value === 'function') return true;
	return /^on[A-Z]/.test(key) && !ORDER_SUFFIX.test(key);
}

/** PS's `resolvePriority`: `order = effect[`${name}Order`] || false`,
 * `priority = ... || 0`, `subOrder = ... || 0`. The dump keeps the raw
 * number, or null where PS has none. */
function orderValue(obj, key, where) {
	const v = obj[key];
	if (v === undefined) return null;
	if (!Number.isInteger(v)) fail(where, `${key} is ${JSON.stringify(v)}`);
	return v;
}

/** A handler's constant: 'function', false, or an id string. */
function handlerValue(value, where) {
	if (typeof value === 'function') return 'function';
	if (value === false) return false;
	if (typeof value === 'string' && value) return value;
	fail(where, `handler constant ${JSON.stringify(value)}`);
}

/** Every handler of one effect object, in its key order. */
function handlersOf(obj, where) {
	const out = [];
	if (!obj) return out;
	for (const key of Object.keys(obj)) {
		if (!isHandler(key, obj[key])) continue;
		out.push({
			name: key,
			value: handlerValue(obj[key], `${where}.${key}`),
			priority: orderValue(obj, `${key}Priority`, where),
			order: orderValue(obj, `${key}Order`, where),
			subOrder: orderValue(obj, `${key}SubOrder`, where),
		});
	}
	// An order key with no handler beside it (gen 3 sandstorm keeps
	// onModifySpDPriority after the mod drops onModifySpD) is never read by
	// resolvePriority, so it is not dumped here; the one exception, the
	// `onSwitchIn` keys of an ability or item answering SwitchIn with its
	// onStart, is `switchInOrderOf`, and a condition's residual keys, which
	// order its duration end, are `residualOrderOf`.
	return out;
}

/** The `onSwitchInPriority` / `Order` / `SubOrder` keys of an ability or
 * item, or null where it has none. `getCallback` lets an ability or item
 * without `onSwitchIn` answer the SwitchIn event with its `onStart` (or, in
 * gen 3 on turn 0, `onAfterEntryHazard`), and `resolvePriority` still reads
 * the `onSwitchIn` keys for it (gen 3 Intimidate's 2, Trace's 1). */
function switchInOrderOf(obj, where) {
	const priority = orderValue(obj, 'onSwitchInPriority', where);
	const order = orderValue(obj, 'onSwitchInOrder', where);
	const subOrder = orderValue(obj, 'onSwitchInSubOrder', where);
	if (priority === null && order === null && subOrder === null) return null;
	return { priority, order, subOrder };
}

/** A condition's residual ordering keys with no handler beside them:
 * `on{,Side,Field}ResidualOrder` / `Priority` / `SubOrder` where the
 * condition has no `on{,Side,Field}Residual` function (gen 3 Taunt, Yawn,
 * Wish, Reflect...). The residual gathers a condition with a `duration`
 * whether or not it answers (`getKey` 'duration'), and `resolvePriority`
 * reads the keys of the event being run for it all the same, so they
 * order its end. Null where it has none. */
function residualOrderOf(obj, where) {
	let found = null;
	for (const event of ['Residual', 'SideResidual', 'FieldResidual']) {
		const key = `on${event}`;
		if (isHandler(key, obj[key])) continue;
		const priority = orderValue(obj, `${key}Priority`, where);
		const order = orderValue(obj, `${key}Order`, where);
		const subOrder = orderValue(obj, `${key}SubOrder`, where);
		if (priority === null && order === null && subOrder === null) continue;
		if (found) fail(where, `orphan residual keys for ${found.event} and ${event}`);
		found = { event, priority, order, subOrder };
	}
	return found;
}

/** Checks every non-handler key of `obj` is in `known`. */
function checkKeys(obj, known, where) {
	for (const key of Object.keys(obj)) {
		const value = obj[key];
		if (isHandler(key, value)) continue;
		if (/^on[A-Z]/.test(key) && ORDER_SUFFIX.test(key)) continue;
		if (known.has(key)) continue;
		if (value === undefined) continue;
		fail(where, `unknown key ${key} = ${JSON.stringify(value)}`);
	}
}

// ---------------------------------------------------------------- values

const BOOST_STATS = ['atk', 'def', 'spa', 'spd', 'spe', 'accuracy', 'evasion'];

/** A boosts table, or null. Keys outside BOOST_STATS stop the dump. */
function boostsOf(value, where) {
	if (value === undefined || value === null) return null;
	if (typeof value !== 'object') fail(where, `boosts ${JSON.stringify(value)}`);
	const out = {};
	for (const key of Object.keys(value)) {
		if (!BOOST_STATS.includes(key)) fail(where, `boost stat ${key}`);
		if (!Number.isInteger(value[key])) fail(where, `boost ${key}=${value[key]}`);
		out[key] = value[key];
	}
	return out;
}

function strOrNull(value, where) {
	if (value === undefined || value === null || value === '') return null;
	if (typeof value !== 'string') fail(where, `expected a string, got ${JSON.stringify(value)}`);
	return value;
}

/** A field naming a condition (status, volatileStatus, sideCondition, ...).
 * PS resolves it with `dex.conditions.get`, which applies toID (gen 3 Wish's
 * `slotCondition: 'Wish'` names the condition `wish`), so the dump records
 * the key it resolves to. */
function condRef(value, where) {
	const s = strOrNull(value, where);
	return s === null ? null : toID(s);
}

function intOrNull(value, where) {
	if (value === undefined || value === null) return null;
	if (!Number.isInteger(value)) fail(where, `expected an integer, got ${JSON.stringify(value)}`);
	return value;
}

function bool(value, where) {
	if (value === undefined || value === null) return false;
	if (typeof value !== 'boolean') fail(where, `expected a boolean, got ${JSON.stringify(value)}`);
	return value;
}

/** PS's [numerator, denominator] fields (drain, recoil, heal). */
function fractionOf(value, where) {
	if (value === undefined || value === null) return null;
	if (Array.isArray(value) && value.length === 2 && value.every(Number.isInteger)) {
		return [value[0], value[1]];
	}
	fail(where, `fraction ${JSON.stringify(value)}`);
}

// ---------------------------------------------------------------- moves

// The move booleans the sim reads (sim/dex-moves.ts `MoveData` and the
// DataMove constructor). Fixed list, fixed order: it is the bit order of
// `MoveBools` in the Rust table.
const MOVE_BOOLS = [
	'affectsFainted', 'alwaysHit', 'breaksProtect', 'callsMove', 'forceSTAB',
	'forceSwitch', 'hasCrashDamage', 'hasSheerForceBoost', 'ignoreAbility',
	'ignoreAccuracy', 'ignoreDefensive', 'ignoreEvasion', 'ignoreImmunity',
	'ignoreNegativeOffensive', 'ignoreOffensive', 'ignorePositiveDefensive',
	'ignorePositiveEvasion', 'isConfusionSelfHit', 'isMax', 'isZ',
	'multiaccuracy', 'noCopy', 'noDamageVariance', 'noPPBoosts', 'ohko',
	'sleepUsable', 'smartTarget', 'spreadHit', 'stallingMove', 'stealsBoosts',
	'struggleRecoil', 'thawsTarget', 'tracksTarget', 'willCrit',
];

// The move fields dumped as their own keys.
const MOVE_FIELDS = [
	'num', 'type', 'category', 'target', 'priority', 'basePower', 'accuracy',
	'pp', 'critRatio', 'damage', 'multihit', 'drain', 'recoil', 'heal',
	'selfdestruct', 'selfSwitch', 'volatileStatus',
	'sideCondition', 'slotCondition', 'weather', 'pseudoWeather', 'status',
	'boosts', 'secondary', 'secondaries', 'self', 'condition', 'flags',
	'placeholderFor', 'baseMoveType', 'name', 'id',
];

// Fields PS has on a move that no gen 1 or gen 3 sim path reads: text,
// bookkeeping set by BasicEffect, Z/Max data (gen 7/8), contest data.
const MOVE_IGNORED = [
	'fullname', 'effectType', 'exists', 'gen', 'shortDesc', 'desc',
	'isNonstandard', 'duration', 'sourceEffect', 'maxMove', 'zMove',
	'contestType', 'realMove',
];

const MOVE_KNOWN = new Set([...MOVE_BOOLS, ...MOVE_FIELDS, ...MOVE_IGNORED]);

const SECONDARY_KNOWN = new Set(['chance', 'status', 'volatileStatus', 'boosts', 'self']);
const SELF_KNOWN = new Set(['chance', 'status', 'volatileStatus', 'boosts', 'sideCondition', 'pseudoWeather']);

/** A move's `self` (or a secondary's `self`), PS's `HitEffect` subset. */
function selfOf(value, where) {
	if (value === undefined || value === null) return null;
	checkKeys(value, SELF_KNOWN, where);
	return {
		chance: intOrNull(value.chance, where),
		status: condRef(value.status, where),
		volatileStatus: condRef(value.volatileStatus, where),
		sideCondition: condRef(value.sideCondition, where),
		pseudoWeather: condRef(value.pseudoWeather, where),
		boosts: boostsOf(value.boosts, where),
		handlers: handlersOf(value, where),
	};
}

/** One of a move's `secondaries`. */
function secondaryOf(value, where) {
	checkKeys(value, SECONDARY_KNOWN, where);
	return {
		chance: intOrNull(value.chance, where),
		status: condRef(value.status, where),
		volatileStatus: condRef(value.volatileStatus, where),
		boosts: boostsOf(value.boosts, where),
		self: selfOf(value.self, `${where}.self`),
		handlers: handlersOf(value, where),
	};
}

function dumpMove(key, move) {
	const where = `move ${key}`;
	checkKeys(move, MOVE_KNOWN, where);
	if (toID(move.name) !== key) fail(where, `name ${move.name} is not its key`);
	// DataMove: `if (data.placeholderFor) this.id = toID(data.placeholderFor)`
	// (the Hidden Power hack: hiddenpowerbug is keyed by its own name but
	// its id is hiddenpower).
	const expectedId = move.placeholderFor ? toID(move.placeholderFor) : key;
	if (move.id !== expectedId) fail(where, `id ${move.id}`);

	let damage = null;
	if (move.damage !== undefined && move.damage !== null) {
		if (move.damage === 'level' || Number.isInteger(move.damage)) damage = move.damage;
		else fail(where, `damage ${JSON.stringify(move.damage)}`);
	}
	let multihit = null;
	if (move.multihit !== undefined) {
		if (Number.isInteger(move.multihit)) multihit = [move.multihit, move.multihit];
		else if (Array.isArray(move.multihit) && move.multihit.length === 2) multihit = move.multihit.slice();
		else fail(where, `multihit ${JSON.stringify(move.multihit)}`);
	}
	let selfdestruct = null;
	if (move.selfdestruct !== undefined) {
		if (move.selfdestruct === 'always' || move.selfdestruct === 'ifHit') selfdestruct = move.selfdestruct;
		else fail(where, `selfdestruct ${JSON.stringify(move.selfdestruct)}`);
	}
	let selfSwitch = null;
	if (move.selfSwitch !== undefined) {
		if (move.selfSwitch === true || move.selfSwitch === 'copyvolatile') selfSwitch = move.selfSwitch;
		else fail(where, `selfSwitch ${JSON.stringify(move.selfSwitch)}`);
	}
	if (move.accuracy !== true && !(Number.isInteger(move.accuracy) && move.accuracy > 0)) {
		fail(where, `accuracy ${JSON.stringify(move.accuracy)}`);
	}
	// PS keeps both `secondary` and `secondaries`; the DataMove constructor
	// makes `secondaries = data.secondaries || [data.secondary]` and the sim
	// reads `secondaries`. Check they agree, dump the list.
	const secondaries = move.secondaries || null;
	if (move.secondary && (!secondaries || secondaries[0] !== move.secondary)) {
		fail(where, 'secondary is not secondaries[0]');
	}

	const bools = [];
	for (const name of MOVE_BOOLS) {
		if (bool(move[name], `${where}.${name}`)) bools.push(name);
	}
	const flags = [];
	for (const name of Object.keys(move.flags || {}).sort()) {
		if (move.flags[name] !== 1) fail(where, `flag ${name}=${move.flags[name]}`);
		flags.push(name);
	}
	return {
		key,
		id: move.id,
		name: move.name,
		num: move.num,
		type: move.type,
		baseMoveType: move.baseMoveType,
		category: move.category,
		target: move.target,
		priority: move.priority,
		basePower: move.basePower,
		accuracy: move.accuracy,
		pp: move.pp,
		critRatio: move.critRatio,
		damage,
		multihit,
		drain: fractionOf(move.drain, `${where}.drain`),
		recoil: fractionOf(move.recoil, `${where}.recoil`),
		heal: fractionOf(move.heal, `${where}.heal`),
		selfdestruct,
		selfSwitch,
		status: condRef(move.status, where),
		volatileStatus: condRef(move.volatileStatus, where),
		sideCondition: condRef(move.sideCondition, where),
		slotCondition: condRef(move.slotCondition, where),
		weather: condRef(move.weather, where),
		pseudoWeather: condRef(move.pseudoWeather, where),
		boosts: boostsOf(move.boosts, `${where}.boosts`),
		flags,
		bools,
		secondaries: secondaries && secondaries.map((s, i) => secondaryOf(s, `${where}.secondaries[${i}]`)),
		self: selfOf(move.self, `${where}.self`),
		// The move's own condition, looked up by the conditions table under
		// the move's key (PS sim/dex-conditions.ts getByID).
		condition: move.condition ? key : null,
		handlers: handlersOf(move, where),
	};
}

// ---------------------------------------------------------------- species

const STAT_ORDER = ['hp', 'atk', 'def', 'spa', 'spd', 'spe'];

function dumpSpecies(key, species) {
	const where = `species ${key}`;
	if (species.id !== key || toID(species.name) !== key) fail(where, `id ${species.id} name ${species.name}`);
	const slots = Object.keys(species.abilities);
	for (const slot of slots) if (slot !== '0' && slot !== '1') fail(where, `ability slot ${slot}`);
	const ratio = species.genderRatio;
	if (!ratio || typeof ratio.M !== 'number' || typeof ratio.F !== 'number') fail(where, 'genderRatio');
	for (const v of [ratio.M, ratio.F]) {
		if (!Number.isInteger(v * 8)) fail(where, `genderRatio ${v} is not in eighths`);
	}
	if (!Number.isInteger(species.weighthg)) fail(where, `weighthg ${species.weighthg}`);
	return {
		key,
		name: species.name,
		num: species.num,
		baseSpecies: species.baseSpecies,
		forme: species.forme || null,
		types: species.types.slice(),
		baseStats: STAT_ORDER.map(s => species.baseStats[s]),
		abilities: [species.abilities['0'] || null, species.abilities['1'] || null],
		gender: species.gender || null,
		genderRatio: [ratio.M, ratio.F],
		weighthg: species.weighthg,
		nfe: !!species.nfe,
		battleOnly: strOrNull(species.battleOnly, where),
		maxHP: intOrNull(species.maxHP, where),
		requiredItem: strOrNull(species.requiredItem, where),
		// The formes that differ only in looks (Unown's letters): PS builds
		// each as its own species object when a set names it, but the table
		// lists only the base. Their names go in the details line.
		cosmeticFormes: species.cosmeticFormes ? species.cosmeticFormes.slice() : null,
	};
}

// ---------------------------------------------------------------- items

const ITEM_KNOWN = new Set([
	'name', 'id', 'num', 'isBerry', 'isGem', 'isPokeball', 'isChoice',
	'ignoreKlutz', 'naturalGift', 'fling', 'itemUser', 'condition', 'boosts',
	// Unread in gens 1 and 3, or text and bookkeeping.
	'fullname', 'effectType', 'exists', 'gen', 'shortDesc', 'desc',
	'isNonstandard', 'duration', 'noCopy', 'affectsFainted', 'sourceEffect',
	'spritenum', 'isPrimalOrb', 'megaStone', 'megaEvolves', 'zMove',
	'zMoveType', 'zMoveFrom', 'onPlate', 'onDrive', 'onMemory', 'forcedForme',
	'status', 'weather',
]);

function dumpItem(key, item) {
	const where = `item ${key}`;
	checkKeys(item, ITEM_KNOWN, where);
	if (item.id !== key || toID(item.name) !== key) fail(where, `id ${item.id}`);
	let naturalGift = null;
	if (item.naturalGift) naturalGift = { basePower: item.naturalGift.basePower, type: item.naturalGift.type };
	let fling = null;
	if (item.fling) {
		checkKeys(item.fling, new Set(['basePower', 'status', 'volatileStatus', 'effect']), `${where}.fling`);
		fling = {
			basePower: item.fling.basePower,
			// Mental Herb and White Herb carry a `fling.effect` function. Fling
			// is a gen 4 move, so no gen 1 or gen 3 battle reaches it.
			effect: typeof item.fling.effect === 'function',
			status: strOrNull(item.fling.status, where),
			volatileStatus: strOrNull(item.fling.volatileStatus, where),
		};
	}
	return {
		key,
		name: item.name,
		num: item.num,
		isBerry: bool(item.isBerry, where),
		isGem: bool(item.isGem, where),
		isPokeball: bool(item.isPokeball, where),
		isChoice: bool(item.isChoice, where),
		ignoreKlutz: bool(item.ignoreKlutz, where),
		naturalGift,
		fling,
		itemUser: item.itemUser ? item.itemUser.slice() : null,
		boosts: boostsOf(item.boosts, where),
		condition: item.condition ? key : null,
		handlers: handlersOf(item, where),
		switchInOrder: switchInOrderOf(item, where),
	};
}

// ---------------------------------------------------------------- abilities

const ABILITY_KNOWN = new Set([
	'name', 'id', 'num', 'rating', 'suppressWeather', 'flags', 'condition',
	'fullname', 'effectType', 'exists', 'gen', 'shortDesc', 'desc',
	'isNonstandard', 'duration', 'noCopy', 'affectsFainted', 'sourceEffect',
	'status', 'weather',
]);

function dumpAbility(key, ability) {
	const where = `ability ${key}`;
	checkKeys(ability, ABILITY_KNOWN, where);
	if (ability.id !== key || toID(ability.name) !== key) fail(where, `id ${ability.id}`);
	const flags = [];
	for (const name of Object.keys(ability.flags || {}).sort()) {
		if (ability.flags[name] !== 1) fail(where, `flag ${name}`);
		flags.push(name);
	}
	return {
		key,
		name: ability.name,
		num: ability.num,
		rating: ability.rating,
		suppressWeather: bool(ability.suppressWeather, where),
		flags,
		condition: ability.condition ? key : null,
		handlers: handlersOf(ability, where),
		switchInOrder: switchInOrderOf(ability, where),
	};
}

// ---------------------------------------------------------------- conditions

const CONDITION_KNOWN = new Set([
	'name', 'effectType', 'duration', 'counterMax', 'noCopy', 'affectsFainted',
]);

/** One condition as `dex.conditions.getByID(key)` resolves it. `source` is
 * where PS found it: the Conditions table, or a move's, ability's or item's
 * `condition` field (in that lookup order). */
function dumpCondition(key, raw, source, dex) {
	const where = `condition ${key}`;
	checkKeys(raw, CONDITION_KNOWN, where);
	const resolved = dex.conditions.get(key);
	if (!resolved.exists) fail(where, 'does not resolve');
	const out = {
		key,
		name: resolved.name,
		effectType: resolved.effectType,
		source,
		duration: intOrNull(raw.duration, where),
		counterMax: intOrNull(raw.counterMax, where),
		noCopy: bool(raw.noCopy, where),
		affectsFainted: bool(raw.affectsFainted, where),
		handlers: handlersOf(raw, where),
	};
	// Only where there is one, so the other entries' dump stays as it was.
	const residualOrder = residualOrderOf(raw, where);
	if (residualOrder) out.residualOrder = residualOrder;
	return out;
}

// ---------------------------------------------------------------- typechart

/** The typechart: PS's type order, and per defending type the code PS
 * stores for each attacking type (0 normal, 1 weak, 2 resists, 3 immune)
 * plus its non-type immunities (`psn`, `sandstorm`, ...), as
 * `Dex.getImmunity` / `getEffectiveness` read them. */
function dumpTypeChart(dex) {
	const types = dex.types.all().filter(t => !t.isNonstandard);
	const names = types.map(t => t.name);
	const rows = types.map(type => {
		const attackers = {};
		const special = {};
		for (const k of Object.keys(type.damageTaken)) {
			const code = type.damageTaken[k];
			if (![0, 1, 2, 3].includes(code)) fail(`type ${type.name}`, `${k}=${code}`);
			if (/^[A-Z]/.test(k)) {
				if (names.includes(k)) attackers[k] = code;
			} else {
				special[k] = code;
			}
		}
		const damageTaken = {};
		for (const n of names) damageTaken[n] = attackers[n] ?? 0;
		return { name: type.name, damageTaken, special };
	});
	return { order: names, types: rows };
}

// ---------------------------------------------------------------- gen

function dumpGen(gen) {
	const dex = Dex.mod(gen);
	const data = dex.data;
	const standard = entry => !entry.isNonstandard;

	// `all()` builds the list from `for (id in data.X)` and getByID; the key
	// is the data table key, which for every standard entry equals toID(name).
	const keyed = (table, getter) => {
		const out = [];
		for (const key in table) {
			const entry = getter(key);
			if (entry.exists && standard(entry)) out.push([key, entry]);
		}
		return out;
	};
	const species = keyed(data.Pokedex, k => dex.species.getByID(k));
	const moves = keyed(data.Moves, k => dex.moves.getByID(k));
	const items = keyed(data.Items, k => dex.items.getByID(k));
	const abilities = keyed(data.Abilities, k => dex.abilities.getByID(k));
	// Cross-check against all(): same entries, same order.
	const same = (a, b) => a.length === b.length && a.every((e, i) => e[1] === b[i]);
	if (!same(species, dex.species.all().filter(standard))) fail(gen, 'species order');
	if (!same(moves, dex.moves.all().filter(standard))) fail(gen, 'moves order');
	if (!same(items, dex.items.all().filter(standard))) fail(gen, 'items order');
	if (!same(abilities, dex.abilities.all().filter(standard))) fail(gen, 'abilities order');
	// `No Ability` is 'Past' from gen 3 on, but a custom game takes it (the
	// validator lets any ability through there), and the carried gen 3
	// corpus names it for a mon with none, as the old harness did. It goes
	// last, so every standard ability keeps its id.
	if (!abilities.some(([k]) => k === 'noability')) {
		const none = dex.abilities.getByID('noability');
		if (!none.exists) fail(gen, 'no noability');
		abilities.push(['noability', none]);
	}

	const out = { gen: dex.gen, psVersion: PS_VERSION, psCommit: PS_COMMIT };
	out.species = species.map(([k, s]) => dumpSpecies(k, s));
	out.moves = moves.map(([k, m]) => dumpMove(k, m));
	out.items = items.map(([k, i]) => dumpItem(k, i));
	out.abilities = abilities.map(([k, a]) => dumpAbility(k, a));

	// Conditions: the Conditions table in its order, then the `condition`
	// of every standard move, ability and item (getByID's lookup order: a
	// key already found earlier shadows a later one).
	const conditions = [];
	const seen = new Set();
	const add = (key, raw, source) => {
		if (seen.has(key)) return;
		seen.add(key);
		conditions.push(dumpCondition(key, raw, source, dex));
	};
	for (const key in data.Conditions) add(key, data.Conditions[key], 'condition');
	for (const [key, m] of moves) if (data.Moves[key].condition) add(key, m.condition, 'move');
	for (const [key, a] of abilities) if (data.Abilities[key].condition) add(key, a.condition, 'ability');
	for (const [key, i] of items) if (data.Items[key].condition) add(key, i.condition, 'item');
	out.conditions = conditions;
	out.typechart = dumpTypeChart(dex);

	// Every effect a move names must be in the conditions table, so the Rust
	// side can store it as a ConditionId.
	const condKeys = new Set(conditions.map(c => c.key));
	const needCond = (id, where) => {
		if (id && !condKeys.has(id)) fail(where, `names condition ${id}, not in the table`);
	};
	for (const m of out.moves) {
		for (const k of ['status', 'volatileStatus', 'sideCondition', 'slotCondition', 'weather', 'pseudoWeather']) {
			needCond(m[k], `move ${m.key}.${k}`);
		}
		for (const s of [m.self, ...(m.secondaries || []), ...(m.secondaries || []).map(s => s.self)]) {
			if (!s) continue;
			for (const k of ['status', 'volatileStatus', 'sideCondition', 'pseudoWeather']) {
				needCond(s[k], `move ${m.key} effect.${k}`);
			}
		}
	}

	// Design section 6 counts functions (not constants), nested `condition`,
	// `secondary` and `self` included, read off the raw entries.
	const fns = hs => hs.filter(h => h.value === 'function').length;
	const rawCond = (table, key) => (table[key] && table[key].condition ? handlersOf(table[key].condition, key) : []);
	const moveFns = m => fns(m.handlers) + fns(rawCond(data.Moves, m.key)) +
		(m.self ? fns(m.self.handlers) : 0) +
		(m.secondaries || []).reduce((n, s) => n + fns(s.handlers) + (s.self ? fns(s.self.handlers) : 0), 0);
	const plainFns = (table, e) => fns(e.handlers) + fns(rawCond(table, e.key)) + (e.fling && e.fling.effect ? 1 : 0);
	const tally = (list, f) => [list.filter(e => f(e) > 0).length, list.reduce((n, e) => n + f(e), 0)];
	const [mw, mf] = tally(out.moves, moveFns);
	const [iw, iff] = tally(out.items, e => plainFns(data.Items, e));
	const [aw, af] = tally(out.abilities, e => plainFns(data.Abilities, e));
	out.counts = {
		species: out.species.length,
		moves: out.moves.length,
		items: out.items.length,
		abilities: out.abilities.length,
		conditions: out.conditions.length,
		types: out.typechart.order.length,
		movesWithHandlers: mw,
		moveFunctions: mf,
		itemsWithHandlers: iw,
		itemFunctions: iff,
		abilitiesWithHandlers: aw,
		abilityFunctions: af,
		handlers: out.moves.reduce((n, m) => n + m.handlers.length + (m.self ? m.self.handlers.length : 0) +
			(m.secondaries || []).reduce((k, s) => k + s.handlers.length + (s.self ? s.self.handlers.length : 0), 0), 0) +
			[out.items, out.abilities, conditions].reduce((n, l) => n + l.reduce((k, e) => k + e.handlers.length, 0), 0),
	};
	return out;
}

function writeGen(gen) {
	const dump = dumpGen(gen);
	const dir = path.join(DATA_ROOT, gen);
	fs.mkdirSync(dir, { recursive: true });
	const files = {
		'species.json': dump.species,
		'moves.json': dump.moves,
		'items.json': dump.items,
		'abilities.json': dump.abilities,
		'conditions.json': dump.conditions,
		'typechart.json': dump.typechart,
		'meta.json': { gen: dump.gen, psVersion: dump.psVersion, psCommit: dump.psCommit, counts: dump.counts, moveBools: MOVE_BOOLS },
	};
	// One entry per line: small, and a re-dump diffs entry by entry.
	const format = value => (Array.isArray(value)
		? `[\n${value.map(e => JSON.stringify(e)).join(',\n')}\n]\n`
		: `${JSON.stringify(value, null, 1)}\n`);
	for (const [name, value] of Object.entries(files)) {
		fs.writeFileSync(path.join(dir, name), format(value));
	}
	const c = dump.counts;
	console.error(
		`${gen}: ${c.species} species, ${c.moves} moves (${c.movesWithHandlers} with handlers, ` +
		`${c.moveFunctions} functions), ${c.items} items (${c.itemsWithHandlers}, ${c.itemFunctions}), ` +
		`${c.abilities} abilities (${c.abilitiesWithHandlers}, ${c.abilityFunctions}), ` +
		`${c.conditions} conditions, ${c.types} types -> ${path.relative(process.cwd(), dir)}`
	);
}

function main() {
	const arg = process.argv[2];
	const gens = arg === 'all' ? ['gen1', 'gen3'] : [arg];
	if (!gens.every(g => /^gen[1-9]$/.test(g || ''))) {
		console.error('usage: node tools/ps-diff/dump-dex.js <gen1|gen3|all>');
		process.exit(1);
	}
	for (const gen of gens) writeGen(gen);
}

main();
