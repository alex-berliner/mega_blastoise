#!/usr/bin/env python3
"""Push every pin stub outward and re-justify the labels that ride on them.

The schematic was generated with stubs pointing *into* each symbol, so all 82
net labels ended up drawn across their own part. This walks the symbol library
geometry, flips each stub to point away from the body, moves its label to the
new free end, and picks a rotation/justification so the text reads outward.
It also lifts Reference and Value off the pins for the parts where the
generator dropped them on top of a terminal.
"""
import math
import sexp

SCH = "mega_blastoise_stripboard.kicad_sch"
STUB = 3.81          # mm from pin to label anchor
GRID = 1.27


def nm(n):
    return n[0] if isinstance(n, list) and isinstance(n[0], str) else None


def get(n, k):
    for c in n:
        if nm(c) == k:
            return c


def getall(n, k):
    return [c for c in n if nm(c) == k]


def at_of(prop):
    """Return a property's position, creating one if the generator omitted it.

    It has to land after the property's name and value atoms or KiCad refuses
    to load the sheet at all.
    """
    at = get(prop, "at")
    if at is None:
        at = ["at", "0", "0", "0"]
        prop.insert(3, at)
    return at


def effects_of(node):
    """Return the node's effects block, creating a default one if it has none."""
    eff = get(node, "effects")
    if eff is None:
        eff = ["effects", ["font", ["size", "1.27", "1.27"]]]
        node.append(eff)
    return eff


def q(v):
    """Round to the schematic grid so KiCad still snaps everything."""
    return round(round(v / GRID) * GRID, 4)


def rot_lib_to_screen(dx, dy, deg):
    """Library vector -> screen vector, including the library's Y flip."""
    r = math.radians(deg)
    c, s = round(math.cos(r)), round(math.sin(r))
    return (dx * c - (-dy) * s, dx * s + (-dy) * c)


root = sexp.parse(open(SCH).read())

symbols = {}
for s_ in [n for n in root if nm(n) == "symbol"]:
    symbols[next(str(p[2]) for p in getall(s_, "property") if str(p[1]) == "Reference")] = s_


def pin_points(sym, libpins):
    lid = str(get(sym, "lib_id")[1])
    at = get(sym, "at")
    X, Y, R = float(at[1]), float(at[2]), float(at[3])
    out = []
    for num, px, py, _ in libpins[lid]:
        dx, dy = rot_lib_to_screen(px, py, R)
        out.append((round(X + dx, 2), round(Y + dy, 2)))
    return out


def shift(node, dx, dy):
    at = get(node, "at")
    at[1] = f"{float(at[1]) + dx:g}"
    at[2] = f"{float(at[2]) + dy:g}"


def move_symbol(ref, dx, dy, libpins):
    """Slide a part and drag its stubs, labels and no-connects along with it."""
    sym = symbols[ref]
    old = set(pin_points(sym, libpins))
    shift(sym, dx, dy)
    for prop in getall(sym, "property"):
        shift(prop, dx, dy)
    ends = set()
    for w in [n for n in root if nm(n) == "wire"]:
        pts = getall(get(w, "pts"), "xy")
        here = [(round(float(p[1]), 2), round(float(p[2]), 2)) for p in pts]
        if not any(h in old for h in here):
            continue
        for p, h in zip(pts, here):
            p[1] = f"{h[0] + dx:g}"
            p[2] = f"{h[1] + dy:g}"
            ends.add(h)
    for n in root:
        if nm(n) not in ("label", "no_connect"):
            continue
        at = get(n, "at")
        p = (round(float(at[1]), 2), round(float(at[2]), 2))
        if p in ends or p in old:
            shift(n, dx, dy)

# ---- library pin table: connection point + the direction pointing away from the body
libpins = {}
for ls in getall(get(root, "lib_symbols"), "symbol"):
    pins = []
    for sub in getall(ls, "symbol"):
        for p in getall(sub, "pin"):
            at = get(p, "at")
            pins.append((str(get(p, "number")[1]), float(at[1]), float(at[2]), float(at[3])))
    libpins[str(ls[1])] = pins

# ---- R1 sat close enough to R2 that their facing VBAT_SENSE labels collided,
#      SW17's output label ran into D1's designator, and the two GND labels
#      at the foot of the divider hung over the title block
move_symbol("R1", 0, -27.94, libpins)
move_symbol("R2", 0, -10.16, libpins)
move_symbol("C1", 0, -10.16, libpins)
move_symbol("SW17", -2.54, 0, libpins)

# ---- resolve every pin to a screen point and an outward unit vector
pins = {}
for s in [n for n in root if nm(n) == "symbol"]:
    lid = str(get(s, "lib_id")[1])
    at = get(s, "at")
    X, Y, R = float(at[1]), float(at[2]), float(at[3])
    ref = next(str(p[2]) for p in getall(s, "property") if str(p[1]) == "Reference")
    for num, px, py, pang in libpins[lid]:
        dx, dy = rot_lib_to_screen(px, py, R)
        out = rot_lib_to_screen(math.cos(math.radians(pang + 180)),
                                math.sin(math.radians(pang + 180)), R)
        pins[(ref, num)] = ((round(X + dx, 2), round(Y + dy, 2)),
                            (round(out[0]), round(out[1])))

at_pin = {}
for key, (pt, out) in pins.items():
    at_pin.setdefault(pt, (key, out))

# ---- flip stubs outward, remembering where each free end moved
moved = {}
for w in [n for n in root if nm(n) == "wire"]:
    a, b = getall(get(w, "pts"), "xy")
    pa = (round(float(a[1]), 2), round(float(a[2]), 2))
    pb = (round(float(b[1]), 2), round(float(b[2]), 2))
    if pa in at_pin and pb in at_pin:
        continue                      # pin-to-pin run, leave it alone
    if pa in at_pin:
        pin, free, endnode = pa, pb, b
    elif pb in at_pin:
        pin, free, endnode = pb, pa, a
    else:
        continue
    ox, oy = at_pin[pin][1]
    new = (q(pin[0] + ox * STUB), q(pin[1] + oy * STUB))
    endnode[1], endnode[2] = f"{new[0]:g}", f"{new[1]:g}"
    moved[free] = (new, (ox, oy))

# ---- labels sitting straight on a pin (J2/J3) get a stub of their own
new_wires = []
for lb in [n for n in root if nm(n) == "label"]:
    at = get(lb, "at")
    p = (round(float(at[1]), 2), round(float(at[2]), 2))
    if p in moved or p not in at_pin:
        continue
    ox, oy = at_pin[p][1]
    new = (q(p[0] + ox * STUB), q(p[1] + oy * STUB))
    new_wires.append(["wire",
                      ["pts", ["xy", f"{p[0]:g}", f"{p[1]:g}"], ["xy", f"{new[0]:g}", f"{new[1]:g}"]],
                      ["stroke", ["width", "0"], ["type", "default"]],
                      ["uuid", sexp.Str(f"stub-{p[0]:g}-{p[1]:g}".replace(".", "_"))]])
    moved[p] = (new, (ox, oy))

# ---- follow each label to its new anchor and aim the text outward
#      right -> rot 0 left-justified, left -> rot 0 right-justified,
#      up -> rot 90 left-justified, down -> rot 90 right-justified
AIM = {(1, 0): (0, "left"), (-1, 0): (0, "right"),
       (0, -1): (90, "left"), (0, 1): (90, "right")}
relabelled = 0
for lb in [n for n in root if nm(n) == "label"]:
    at = get(lb, "at")
    p = (round(float(at[1]), 2), round(float(at[2]), 2))
    if p not in moved:
        continue
    new, out = moved[p]
    rot, just = AIM[out]
    at[1], at[2], at[3] = f"{new[0]:g}", f"{new[1]:g}", f"{rot:g}"
    eff = effects_of(lb)
    for old in getall(eff, "justify"):
        eff.remove(old)
    eff.append(["justify", just])
    relabelled += 1

root.extend(new_wires)

# ---- lift Reference/Value off the pins
#      values are the position and the rotation you want to SEE; KiCad adds the
#      symbol's own rotation to a property's angle, so D1 (placed at 270) needs
#      its fields booked at -180 to come out vertical
FIELDS = {
    "U1":  {"Reference": (91.44, 48.26, 0, "left"),   "Value": (91.44, 52.07, 0, "left")},
    "J1":  {"Reference": (213.36, 31.75, 0, "left"),  "Value": (213.36, 27.94, 0, "left")},
    "J2":  {"Reference": (158.75, 148.59, 0, "left"), "Value": (158.75, 151.13, 0, "left")},
    "J3":  {"Reference": (158.75, 157.48, 0, "left"), "Value": (158.75, 160.02, 0, "left")},
    "BT1": {"Reference": (120.65, 149.86, 90, None),  "Value": (128.27, 149.86, 90, None)},
    "R1":  {"Reference": (236.22, 106.68, 90, None),  "Value": (243.84, 106.68, 90, None)},
    "R2":  {"Reference": (236.22, 149.86, 90, None),  "Value": (243.84, 149.86, 90, None)},
    "C1":  {"Reference": (257.81, 149.86, 90, None),  "Value": (265.43, 149.86, 90, None)},
    "D1":  {"Reference": (205.74, 144.78, 90, None),  "Value": (213.36, 144.78, 90, None)},
    "SW17": {"Reference": (182.88, 140.97, 0, None),  "Value": (182.88, 148.59, 0, None)},
    # the flag arrived with no field positions at all, so KiCad's own writer
    # parked them at the sheet origin; None here means hide the field instead
    "#PWR001": {"Reference": None, "Value": (199.39, 136.53, 0, None)},
}
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
        at = at_of(prop)
        at[1], at[2], at[3] = f"{x:g}", f"{y:g}", f"{(rot - sym_rot) % 360:g}"
        for old in getall(eff, "justify"):
            eff.remove(old)
        if just:
            eff.append(["justify", just])

# ---- the four notes were centred on the sheet at size 1.6, so the long ones ran
#      off the page and across J1. Re-flow them left-justified into the empty
#      column to the right of the Pico.
NOTES = [
    "All 16 buttons are direct-to-GPIO, active low, internal pull-ups. "
    "No matrix, no diodes: ghosting is impossible by construction.",
    "J1 pin order is ASSUMED: 1=GND 2=VCC 3=SCL/SCK 4=SDA/MOSI 5=RES 6=DC "
    "7=CS 8=BLK. VERIFY against the module silkscreen before soldering.",
    "Power: the charger's USB-C and the Pico's micro-USB may be plugged in "
    "at the same time.",
    "VBUS reaches VSYS through the Pico's own Schottky, the cell reaches it "
    "through D1; the higher rail wins and neither back-feeds the other.",
]
NOTE_X, NOTE_Y, NOTE_STEP, NOTE_COLS = 165.1, 66.04, 3.81, 52


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


note_nodes = [n for n in root if nm(n) == "text"]
for n in note_nodes:
    root.remove(n)
y = NOTE_Y
for i, body in enumerate(NOTES):
    for line in wrap(body, NOTE_COLS):
        root.append(["text", sexp.Str(line),
                     ["at", f"{NOTE_X:g}", f"{y:g}", "0"],
                     ["effects", ["font", ["size", "1.27", "1.27"]], ["justify", "left"]],
                     ["uuid", sexp.Str(f"note-{i}-{y:g}".replace(".", "_"))]])
        y += NOTE_STEP
    y += NOTE_STEP / 2

open(SCH, "w").write(sexp.dumps(root) + "\n")
print(f"stubs flipped: {len(moved)}   labels re-aimed: {relabelled}   stubs added: {len(new_wires)}")
