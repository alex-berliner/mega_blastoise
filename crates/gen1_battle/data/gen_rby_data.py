#!/usr/bin/env python3
"""Regenerate rby_moves.json and rby_species.json (run from this directory).

Sources (fetched):
  - pret/pokered data/moves/moves.asm            → cartridge move table
  - smogon/pokemon-showdown data/mods/gen1/pokedex.ts → RBY base stats
Cross-referenced against the vendored gen1_moves.json / gen1_mons.json for
display names and Gen 1 typing (with the Fairy/Steel retcons reverted).
"""
import json
import re
import urllib.request

MOVES_ASM_URL = "https://raw.githubusercontent.com/pret/pokered/master/data/moves/moves.asm"
POKEDEX_TS_URL = "https://raw.githubusercontent.com/smogon/pokemon-showdown/master/data/mods/gen1/pokedex.ts"


def fetch(url):
    with urllib.request.urlopen(url) as r:
        return r.read().decode()


moves_json = json.load(open("gen1_moves.json"))
mons_json = json.load(open("gen1_mons.json"))

# ── Moves ────────────────────────────────────────────────────────────────────
TYPE_MAP = {
    "NORMAL": "Normal", "FIGHTING": "Fighting", "FLYING": "Flying", "POISON": "Poison",
    "GROUND": "Ground", "ROCK": "Rock", "BUG": "Bug", "GHOST": "Ghost", "FIRE": "Fire",
    "WATER": "Water", "GRASS": "Grass", "ELECTRIC": "Electric", "PSYCHIC_TYPE": "Psychic",
    "PSYCHIC": "Psychic", "ICE": "Ice", "DRAGON": "Dragon",
}
ID_FIXES = {
    "hijumpkick": "highjumpkick",  # modern PS id spelling
    "vicegrip": "visegrip",
    "psychicm": "psychic",         # pokered constant is PSYCHIC_M
}

rows = []
line_re = re.compile(r"^\s*move\s+(\w+),\s+(\w+),\s+(\d+),\s+(\w+),\s+(\d+),\s+(\d+)")
for line in fetch(MOVES_ASM_URL).splitlines():
    m = line_re.match(line)
    if not m:
        continue
    name_const, effect, power, mtype, acc, pp = m.groups()
    mid = name_const.replace("_", "").lower()
    mid = ID_FIXES.get(mid, mid)
    rows.append((mid, effect, int(power), TYPE_MAP[mtype], int(acc), int(pp)))


# Gen 1 always-hit set: the cartridge skips the accuracy check for Swift and
# for self/field-targeting moves. The vendored modern JSON already encodes
# that set as accuracy == "exempt"; Bide additionally never misses in Gen 1.
def always_hits(mid, effect):
    if effect == "SWIFT_EFFECT" or mid == "bide":
        return True
    j = moves_json.get(mid)
    return bool(j) and j.get("accuracy") == "exempt"


out_moves = {}
unmatched = []
for mid, effect, power, mtype, acc, pp in rows:
    j = moves_json.get(mid)
    if j is None:
        unmatched.append(mid)
    name = (j or {}).get("name") or mid.title()
    out_moves[mid] = {
        "name": name,
        "effect": effect,
        "power": power,
        "type": mtype,
        "accuracy": 0 if always_hits(mid, effect) else acc,
        "pp": pp,
    }
assert not unmatched, f"ids missing from gen1_moves.json: {unmatched}"
assert len(out_moves) == 165, len(out_moves)
json.dump(out_moves, open("rby_moves.json", "w"), indent=1, sort_keys=True)

# ── Species ──────────────────────────────────────────────────────────────────
# RBY base stats from Showdown's gen1 pokedex (spa == spd == RBY Special;
# several physical stats also differ from modern data due to Gen 6+ buffs).
ts = fetch(POKEDEX_TS_URL)
entry_re = re.compile(
    r"^\t(\w+): \{[^}]*?baseStats: \{ hp: (\d+), atk: (\d+), def: (\d+), spa: (\d+), spd: (\d+), spe: (\d+) \}",
    re.M | re.S,
)
ps_stats = {}
for m in entry_re.finditer(ts):
    sid, hp, atk, df, spa, spd, spe = m.groups()
    ps_stats[sid] = dict(hp=int(hp), atk=int(atk), def_=int(df), spa=int(spa), spd=int(spd), spe=int(spe))

# Gen 1 type retcons vs modern data (Fairy and Steel didn't exist).
TYPE_OVERRIDES = {
    "magnemite": ("Electric", "None"),
    "magneton": ("Electric", "None"),
    "clefairy": ("Normal", "None"),
    "clefable": ("Normal", "None"),
    "jigglypuff": ("Normal", "None"),
    "wigglytuff": ("Normal", "None"),
    "mrmime": ("Psychic", "None"),
}

out_species = {}
for sid, v in mons_json.items():
    if v.get("name", "") != v.get("base_species", ""):
        continue  # formes/megas
    ps = ps_stats[sid]
    assert ps["spa"] == ps["spd"], f"{sid}: gen1 pokedex spa != spd"
    prim = v.get("primary_type", "Normal")
    sec = v.get("secondary_type") or "None"
    prim, sec = TYPE_OVERRIDES.get(sid, (prim, sec))
    out_species[sid] = {
        "base_stats": {
            "hp": ps["hp"], "atk": ps["atk"], "def": ps["def_"],
            "spc": ps["spa"], "spe": ps["spe"],
        },
        "primary_type": prim,
        "secondary_type": sec,
    }
assert len(out_species) == 151, len(out_species)
json.dump(out_species, open("rby_species.json", "w"), indent=1, sort_keys=True)
print(f"wrote {len(out_moves)} moves, {len(out_species)} species")
