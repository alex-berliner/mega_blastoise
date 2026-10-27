#!/usr/bin/env node
'use strict';
// Prints the RESOLVED definition of one effect in one gen, handler source
// included, walking the mod chain the way the sim does (PS sim/dex.ts
// `loadData`). Reading data/moves.ts directly gets the modern definition,
// which for most effects is not what gens 1 and 3 run.
//
//   node tools/ps-diff/show-effect.js <gen> <kind> <id>
//   node tools/ps-diff/show-effect.js gen3 ability intimidate
//   node tools/ps-diff/show-effect.js gen1 move bide
//   node tools/ps-diff/show-effect.js gen1 condition par
//
// kind: move | item | ability | species | condition | type.
// Each handler is printed with the mod its function comes from (the
// furthest-up mod in the chain whose data holds that same function), so an
// `inherit: true` override shows which parts are gen-specific.
// Generalises the legacy harness's `show-ability.js` (which went with the
// legacy engines in G3-9).

const { Dex } = require('pokemon-showdown/dist/sim');

const TABLES = {
	move: 'Moves',
	item: 'Items',
	ability: 'Abilities',
	species: 'Pokedex',
	condition: 'Conditions',
	type: 'TypeChart',
};

function toID(text) {
	return `${text}`.toLowerCase().replace(/[^a-z0-9]+/g, '');
}

function usage() {
	console.error('usage: node tools/ps-diff/show-effect.js <gen> <move|item|ability|species|condition|type> <id>');
	process.exit(1);
}

/** The mod chain of a dex, nearest first: gen1, gen2, ..., base. */
function chainOf(dex) {
	const chain = [];
	let mod = dex;
	for (;;) {
		mod.loadData();
		chain.push(mod);
		if (!mod.parentMod) break;
		mod = Dex.mod(mod.parentMod);
	}
	return chain;
}

/** The raw data object for `id` in `mod` under `path` (e.g. ['condition']). */
function rawAt(mod, table, id, path) {
	let obj = mod.data[table] && mod.data[table][id];
	for (const step of path) {
		if (!obj) return undefined;
		obj = obj[step];
	}
	return obj;
}

/** The furthest-up mod whose data holds `fn` at the same place. */
function originOf(chain, table, id, path, key, fn) {
	let origin = chain[0].currentMod;
	for (const mod of chain) {
		const raw = rawAt(mod, table, id, path);
		if (raw && raw[key] === fn) origin = mod.currentMod;
	}
	return origin;
}

function printValue(indent, key, value) {
	console.log(`${indent}${key}: ${JSON.stringify(value)}`);
}

function printObject(chain, table, id, path, obj, indent) {
	const keys = Object.keys(obj).sort((a, b) => {
		const fa = typeof obj[a] === 'function';
		const fb = typeof obj[b] === 'function';
		return fa === fb ? 0 : fa ? 1 : -1;
	});
	for (const key of keys) {
		const value = obj[key];
		if (value === undefined) continue;
		if (typeof value === 'function') {
			const from = originOf(chain, table, id, path, key, value);
			console.log(`${indent}${key}  [from ${from}]`);
			const src = value.toString().split('\n');
			for (const line of src) console.log(`${indent}  | ${line}`);
		} else if (key === 'condition' || key === 'self' ||
			key === 'secondary' || key === 'fling') {
			if (value && typeof value === 'object') {
				console.log(`${indent}${key}:`);
				printObject(chain, table, id, [...path, key], value, `${indent}  `);
			} else {
				printValue(indent, key, value);
			}
		} else if (key === 'secondaries' && Array.isArray(value)) {
			value.forEach((s, i) => {
				console.log(`${indent}secondaries[${i}]:`);
				printObject(chain, table, id, [...path, 'secondaries', i], s, `${indent}  `);
			});
		} else if (key === 'desc' || key === 'shortDesc') {
			continue;
		} else {
			printValue(indent, key, value);
		}
	}
}

function main() {
	const [gen, kind, name] = process.argv.slice(2);
	if (!gen || !kind || !name || !TABLES[kind]) usage();
	const dex = Dex.mod(gen);
	const chain = chainOf(dex);
	const table = TABLES[kind];
	let entry;
	let id = toID(name);
	switch (kind) {
	case 'move': entry = dex.moves.get(name); break;
	case 'item': entry = dex.items.get(name); break;
	case 'ability': entry = dex.abilities.get(name); break;
	case 'species': entry = dex.species.get(name); break;
	case 'condition': entry = dex.conditions.get(name); break;
	case 'type': entry = dex.types.get(name); break;
	}
	if (!entry || !entry.exists) {
		console.log(`-- ${gen} ${kind} ${name}: no such entry`);
		process.exit(2);
	}
	// A condition can live in Conditions or in a move's, ability's or item's
	// `condition` (PS sim/dex-conditions.ts getByID, in that order).
	let rawTable = table;
	let path = [];
	if (kind === 'condition' && !(id in dex.data.Conditions)) {
		for (const t of ['Moves', 'Abilities', 'Items']) {
			if (dex.data[t][id] && dex.data[t][id].condition) {
				rawTable = t;
				path = ['condition'];
				break;
			}
		}
	}
	if (kind === 'type') id = toID(entry.name);
	console.log(`-- ${gen} ${kind} ${entry.id || id} (${entry.name}), mod chain ${chain.map(m => m.currentMod).join(' -> ')}`);
	if (path.length) console.log(`   defined as ${rawTable}.${id}.condition`);
	if (entry.isNonstandard) console.log(`   isNonstandard: ${entry.isNonstandard} (not in the ${gen} dex dump)`);
	printObject(chain, rawTable, kind === 'type' ? id : id, path, entry, '  ');
}

main();
