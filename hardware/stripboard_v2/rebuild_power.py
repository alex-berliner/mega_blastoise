#!/usr/bin/env python3
"""Redraw the power train as one connected module instead of nine loose parts.

Every net on this sheet was joined by name, which is fine for the button banks
where each switch really is an island, and wrong for the power train, where the
whole point is that the cell reaches VSYS through a chain you can follow with a
finger. This tears out the wires, labels and junctions belonging to those nine
symbols, lays them out left to right in the order the current actually flows,
and wires them together. Net names survive: each one keeps a single label,
placed on the wire rather than hung off a pin.

Idempotent. It deletes everything it owns before drawing, so running it twice
is the same as running it once. Run it after tidy_schematic.py.
"""
import sexp

SCH = "mega_blastoise_stripboard.kicad_sch"
PWR = ("BT1", "J2", "J3", "SW17", "D1", "R1", "R2", "C1", "#PWR001")

# ---- where each part goes: (x, y, rotation)
PLACE = {
    "BT1":     (180.34, 88.90, 0),
    "J2":      (198.12, 83.82, 0),
    "J3":      (198.12, 99.06, 0),
    "SW17":    (223.52, 99.06, 0),
    # mirrored, not rotated 180: the cathode has to face right the way the
    # current flows, and a 180 rotation takes the field text upside down with it
    "D1":      (252.73, 99.06, 0, "y"),
    "#PWR001": (261.62, 99.06, 0),
    "R1":      (234.95, 111.76, 0),
    "R2":      (234.95, 130.81, 0),
    "C1":      (250.19, 130.81, 0),
}

# ---- Reference and Value: (x, y, rotation you want to SEE, justify or None)
#      None instead of a tuple hides the field
FIELDS = {
    "BT1":     {"Reference": (180.34, 78.74, 0, None),
                "Value": (176.53, 88.90, 90, None)},
    "J2":      {"Reference": (203.20, 83.82, 0, "left"),
                "Value": (203.20, 86.36, 0, "left")},
    "J3":      {"Reference": (203.20, 104.14, 0, "left"),
                "Value": (203.20, 107.95, 0, "left")},
    "SW17":    {"Reference": (223.52, 93.98, 0, None),
                "Value": (223.52, 104.14, 0, None)},
    "D1":      {"Reference": (252.73, 93.98, 0, None),
                "Value": (252.73, 104.14, 0, None)},
    "#PWR001": {"Reference": None, "Value": (257.81, 91.44, 0, None)},
    "R1":      {"Reference": (231.14, 111.76, 90, None),
                "Value": (238.76, 111.76, 90, None)},
    "R2":      {"Reference": (231.14, 130.81, 90, None),
                "Value": (238.76, 130.81, 90, None)},
    "C1":      {"Reference": (246.38, 130.81, 90, None),
                "Value": (254.00, 130.81, 90, None)},
}

# ---- the module, drawn in the order the current takes
WIRES = [
    # cell to the charger's battery terminals
    ((180.34, 83.82), (193.04, 83.82)),
    ((180.34, 91.44), (187.96, 91.44)),
    ((187.96, 91.44), (187.96, 86.36)),
    ((187.96, 86.36), (193.04, 86.36)),
    # protected output, through the switch, through the diode, out to VSYS
    ((193.04, 99.06), (218.44, 99.06)),
    ((228.60, 99.06), (234.95, 99.06)),
    ((234.95, 99.06), (248.92, 99.06)),
    ((256.54, 99.06), (261.62, 99.06)),
    ((261.62, 99.06), (269.24, 99.06)),
    # the sense divider hangs off the switched rail, ahead of the diode
    ((234.95, 99.06), (234.95, 107.95)),
    ((234.95, 115.57), (234.95, 127.00)),
    ((234.95, 120.65), (250.19, 120.65)),
    ((250.19, 120.65), (250.19, 127.00)),
    ((250.19, 120.65), (269.24, 120.65)),
    # one ground rail along the foot of the module
    ((193.04, 101.60), (193.04, 140.97)),
    ((187.96, 140.97), (250.19, 140.97)),
    ((234.95, 134.62), (234.95, 140.97)),
    ((250.19, 134.62), (250.19, 140.97)),
]

JUNCTIONS = [(234.95, 99.06), (261.62, 99.06), (234.95, 120.65),
             (250.19, 120.65), (193.04, 140.97), (234.95, 140.97)]

# ---- one label per net, sitting on the wire that carries it
LABELS = [
    ("BATT+",      182.88, 83.82, 0, "left"),
    ("BATT-",      181.61, 91.44, 0, "left"),
    ("VBATT",      201.93, 99.06, 0, "left"),
    ("VBATT_SW",   236.22, 99.06, 0, "left"),
    ("VSYS",       269.24, 99.06, 0, "left"),
    ("VBAT_SENSE", 269.24, 120.65, 0, "left"),
    ("GND",        187.96, 140.97, 0, "right"),
]

BOX = (172.72, 71.12, 264.16, 146.05)
BOX_TITLE = ("power train", 172.72, 68.58)

# ---- the notes move to the empty band under the button banks, because the
#      column they were in is where the module now lives
NOTES = [
    "All 16 buttons are direct-to-GPIO, active low, internal pull-ups. "
    "No matrix, no diodes: ghosting is impossible by construction.",
    "J1 pin order is VERIFIED against the real module, which is REVERSED "
    "from the usual ST7796 boards: socket position 1 receives module pin 8 "
    "(BL) and position 8 receives GND. SCK/MOSI land on GP9/GP8, so the "
    "firmware drives the panel by PIO, not hardware SPI.",
    "Power: the charger's USB-C and the Pico's micro-USB may be plugged in "
    "at the same time.",
    "VBUS reaches VSYS through the Pico's own Schottky, the cell reaches it "
    "through D1; the higher rail wins and neither back-feeds the other.",
]
NOTE_X, NOTE_Y, NOTE_STEP, NOTE_COLS = 21.59, 161.29, 3.81, 86


def nm(n):
    return n[0] if isinstance(n, list) and isinstance(n[0], str) else None


def get(n, k):
    for c in n:
        if nm(c) == k:
            return c


def getall(n, k):
    return [c for c in n if nm(c) == k]


def at_of(prop):
    """A property's position, created after its name and value if it has none."""
    a = get(prop, "at")
    if a is None:
        a = ["at", "0", "0", "0"]
        prop.insert(3, a)
    return a


def effects_of(node):
    e = get(node, "effects")
    if e is None:
        e = ["effects", ["font", ["size", "1.27", "1.27"]]]
        node.append(e)
    return e


def set_justify(node, just):
    eff = effects_of(node)
    for old in getall(eff, "justify"):
        eff.remove(old)
    if just:
        eff.append(["justify", just])


def wrap(text, cols):
    lines, cur = [], ""
    for word in text.split():
        if cur and len(cur) + 1 + len(word) > cols:
            lines.append(cur)
            cur = word
        else:
            cur = f"{cur} {word}".strip()
    if cur:
        lines.append(cur)
    return lines


def uid(tag):
    return sexp.Str(f"pwr-{tag}".replace(".", "_").replace(" ", "-"))


def xy(x, y):
    return ["xy", f"{x:g}", f"{y:g}"]


root = sexp.parse(open(SCH).read())

symbols = {}
for s in [n for n in root if nm(n) == "symbol"]:
    ref = next(str(p[2]) for p in getall(s, "property") if str(p[1]) == "Reference")
    symbols[ref] = s

# ---- library pin table, so we know where a moved part's pins land
libpins = {}
for ls in getall(get(root, "lib_symbols"), "symbol"):
    libpins[str(ls[1])] = [(str(get(p, "number")[1]),
                            float(get(p, "at")[1]), float(get(p, "at")[2]))
                           for sub in getall(ls, "symbol")
                           for p in getall(sub, "pin")]

import math


def pin_points(ref):
    sym = symbols[ref]
    at = get(sym, "at")
    X, Y, R = float(at[1]), float(at[2]), float(at[3])
    mir = get(sym, "mirror")
    mx = -1 if mir and str(mir[1]) == "y" else 1
    my = -1 if mir and str(mir[1]) == "x" else 1
    r = math.radians(R)
    c, s = round(math.cos(r)), round(math.sin(r))
    out = {}
    for num, px, py in libpins[str(get(sym, "lib_id")[1])]:
        px, py = px * mx, py * my
        dx = px * c - (-py) * s
        dy = px * s + (-py) * c
        out[num] = (round(X + dx, 2), round(Y + dy, 2))
    return out


# ---- tear out everything the module owns, so a second run is a no-op
owned = set()
for ref in PWR:
    owned.update(pin_points(ref).values())

doomed_ends = set(owned)
for w in [n for n in root if nm(n) == "wire"]:
    pts = [(round(float(p[1]), 2), round(float(p[2]), 2))
           for p in getall(get(w, "pts"), "xy")]
    if any(p in owned for p in pts):
        doomed_ends.update(pts)
        root.remove(w)

for n in list(root):
    kind = nm(n)
    if kind not in ("label", "junction", "text", "rectangle"):
        continue
    u = get(n, "uuid")
    if u is not None and str(u[1]).startswith("pwr-"):
        root.remove(n)          # ours from a previous run, whatever it is
        continue
    if kind in ("text", "rectangle"):
        root.remove(n)          # notes and the module box are both reissued
        continue
    a = get(n, "at")
    if (round(float(a[1]), 2), round(float(a[2]), 2)) in doomed_ends:
        root.remove(n)

# ---- place the parts
for ref, spec in PLACE.items():
    x, y, rot = spec[0], spec[1], spec[2]
    mirror = spec[3] if len(spec) > 3 else None
    sym = symbols[ref]
    at = get(sym, "at")
    at[1], at[2], at[3] = f"{x:g}", f"{y:g}", f"{rot:g}"
    for old in getall(sym, "mirror"):
        sym.remove(old)
    if mirror:
        sym.insert(sym.index(at) + 1, ["mirror", mirror])

for ref, spec in FIELDS.items():
    sym_rot = float(get(symbols[ref], "at")[3])
    for prop in getall(symbols[ref], "property"):
        key = str(prop[1])
        if key not in spec:
            continue
        eff = effects_of(prop)
        for old in getall(eff, "hide"):
            eff.remove(old)
        if spec[key] is None:
            eff.append(["hide", "yes"])
            continue
        x, y, rot, just = spec[key]
        a = at_of(prop)
        # KiCad adds the symbol's own rotation to a property's angle
        a[1], a[2], a[3] = f"{x:g}", f"{y:g}", f"{(rot - sym_rot) % 360:g}"
        set_justify(prop, just)

# ---- draw it
for i, (a, b) in enumerate(WIRES):
    root.append(["wire", ["pts", xy(*a), xy(*b)],
                 ["stroke", ["width", "0"], ["type", "default"]],
                 ["uuid", uid(f"w{i}")]])

for i, (x, y) in enumerate(JUNCTIONS):
    root.append(["junction", ["at", f"{x:g}", f"{y:g}"], ["diameter", "0"],
                 ["color", "0", "0", "0", "0"], ["uuid", uid(f"j{i}")]])

for i, (net, x, y, rot, just) in enumerate(LABELS):
    root.append(["label", sexp.Str(net), ["at", f"{x:g}", f"{y:g}", f"{rot:g}"],
                 ["effects", ["font", ["size", "1.27", "1.27"]],
                  ["justify", just]],
                 ["uuid", uid(f"l{i}")]])

root.append(["rectangle",
             ["start", f"{BOX[0]:g}", f"{BOX[1]:g}"],
             ["end", f"{BOX[2]:g}", f"{BOX[3]:g}"],
             ["stroke", ["width", "0.254"], ["type", "dash"]],
             ["fill", ["type", "none"]],
             ["uuid", uid("box")]])

root.append(["text", sexp.Str(BOX_TITLE[0]),
             ["at", f"{BOX_TITLE[1]:g}", f"{BOX_TITLE[2]:g}", "0"],
             ["effects", ["font", ["size", "1.8", "1.8"]], ["justify", "left"]],
             ["uuid", uid("boxtitle")]])

y = NOTE_Y
for i, body in enumerate(NOTES):
    for line in wrap(body, NOTE_COLS):
        root.append(["text", sexp.Str(line),
                     ["at", f"{NOTE_X:g}", f"{y:g}", "0"],
                     ["effects", ["font", ["size", "1.27", "1.27"]],
                      ["justify", "left"]],
                     ["uuid", uid(f"note{i}-{y:g}")]])
        y += NOTE_STEP
    y += NOTE_STEP / 2

open(SCH, "w").write(sexp.dumps(root) + "\n")
print(f"module: {len(WIRES)} wires, {len(JUNCTIONS)} junctions, "
      f"{len(LABELS)} labels, {len(PLACE)} parts placed")
