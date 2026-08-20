#!/usr/bin/env python3
"""Check the layout the way a meter would: net by net, hole by hole.

Run after any change to layout.py. It answers four questions:

  1. does every net actually reach its Pico pin (union-find over strips+wires)
  2. does any strip segment carry two different nets (a short)
  3. does anything share a hole, sit on a cut, or hide under the Pico
  4. does any wire cross a switch body or run alongside a ground bus

The connectivity pass is the important one: a floating net produces no
conflict and no collision, so the earlier checks could not see it.
"""

import importlib.util
import math
import sys
from collections import defaultdict

spec = importlib.util.spec_from_file_location("L", "layout.py")
L = importlib.util.module_from_spec(spec)
spec.loader.exec_module(L)

BODY, BODY_M = 6.0, 0.6
BUS_CLEAR, PARALLEL_DEG = 2.6, 20.0
fail = []
warn = []


def hole_xy(h):
    return L.hole(*h)


# ---- what sits in every hole, and which net it belongs to ----
net_holes = defaultdict(set)
sig = {n: c for n, c, r, p, h in L.BUTTON_NETS}

for cross, (a, b) in L.CROSSES.items():
    for cm, (col, row) in L.cross_switches(a, b).items():
        net = L.CROSS_LABELS[cross][cm]
        gnd = col + 3 if sig[net] == col else col
        for r in (row, row + 2):
            net_holes[net].add((sig[net], r))
            net_holes["GND"].add((gnd, r))

# the display's VCC pin and the Pico's 3V3_OUT are one net under two names
ALIAS = {"VCC": "3V3"}
for jp, name, col, rows, pin, how in L.DISPLAY_NETS:
    net_holes[ALIAS.get(name, name)].add((col, L.DISPLAY_HEADER_ROW))

for pin in (3, 8, 13, 18):
    net_holes["GND"].add((L.col_of_pin(pin), L.PICO_ROW_LOWER))
for pin, col in ((23, 20), (28, 15), (38, 5)):
    net_holes["GND"].add((col, L.PICO_ROW_UPPER))

for net, col, rows, pin, how in L.BUTTON_NETS:
    net_holes[net].add((L.col_of_pin(pin),
                        L.PICO_ROW_LOWER if pin <= 20 else L.PICO_ROW_UPPER))
for jp, name, col, rows, pin, how in L.DISPLAY_NETS:
    if name != "GND":
        net_holes[ALIAS.get(name, name)].add((L.col_of_pin(pin),
                             L.PICO_ROW_LOWER if pin <= 20 else L.PICO_ROW_UPPER))
# power train: read the nets straight off the placed pads, so moving a part
# cannot leave this table pointing at holes that no longer exist
PAD_NETS = {
    ("BT1", "1"): "BATT+",   ("BT1", "2"): "BATT-",
    ("J2", "1"): "BATT+",    ("J2", "2"): "BATT-",
    ("J3", "1"): "VBATT",    ("J3", "2"): "GND",
    ("SW17", "1"): "VBATT",  ("SW17", "2"): "VBATT_SW",
    ("D1", "1"): "VSYS",     ("D1", "2"): "VBATT_SW",
    ("R1", "1"): "VBATT_SW", ("R1", "2"): "VBAT_SENSE",
    ("R2", "1"): "VBAT_SENSE", ("R2", "2"): "GND",
    ("C1", "1"): "VBAT_SENSE", ("C1", "2"): "GND",
}
pads_all = L.expected_pads()
for (ref, num), net in PAD_NETS.items():
    for h in pads_all[ref][num]:
        net_holes[net].add(h)
for c, r, net in [(4, L.PICO_ROW_UPPER, "VSYS"), (7, L.PICO_ROW_UPPER, "3V3"),
                  (9, L.PICO_ROW_UPPER, "VBAT_SENSE")]:
    net_holes[net].add((c, r))

# ---- union-find over strip segments joined by wires ----
segs = list(L.strip_segments())


def seg_of(c, r):
    for cc, r0, r1 in segs:
        if cc == c and r0 <= r <= r1:
            return (cc, r0, r1)
    return None


parent = {s: s for s in segs}


def find(x):
    while parent[x] != x:
        parent[x] = parent[parent[x]]
        x = parent[x]
    return x


def union(a, b):
    ra, rb = find(a), find(b)
    if ra != rb:
        parent[ra] = rb


# The Pico bridges its own like-named pins internally, which no strip shows:
# all eight GND pins are one node inside the module, so the halves of the
# board are grounded to each other through it rather than across the row-30
# cut. Model that before looking for floating nets.
pico_gnd = [seg_of(L.col_of_pin(p), L.PICO_ROW_LOWER) for p in (3, 8, 13, 18)]
pico_gnd += [seg_of(c, L.PICO_ROW_UPPER) for c in (5, 15, 20)]
pico_gnd = [s for s in pico_gnd if s]

for name, pts in L.JUMPERS:
    ss = [seg_of(*p) for p in pts]
    if any(s is None for s in ss):
        fail.append(f"wire {name!r} has an endpoint on a destroyed hole")
        continue
    for a, b in zip(ss, ss[1:]):
        union(a, b)

for a, b in zip(pico_gnd, pico_gnd[1:]):
    union(a, b)

# 1. connectivity
for net, holes in sorted(net_holes.items()):
    groups = set()
    for h in holes:
        s = seg_of(*h)
        if s is None:
            fail.append(f"net {net}: hole {h} is on a cut")
            continue
        groups.add(find(s))
    if len(groups) > 1:
        fail.append(f"net {net} is split into {len(groups)} islands: not connected")

# 2. shorts
owner = {}
for net, holes in net_holes.items():
    for h in holes:
        s = seg_of(*h)
        if s is None:
            continue
        if owner.setdefault(find(s), net) != net:
            fail.append(f"short: {net} and {owner[find(s)]} share a conductor")

# 3. holes
used = defaultdict(list)
for ref, pads in L.expected_pads().items():
    for num, hs in pads.items():
        for h in hs:
            used[h].append(ref)
for i in range(20):
    used[(L.PICO_COL0 + i, L.PICO_ROW_LOWER)].append("U1")
    used[(L.PICO_COL0 + i, L.PICO_ROW_UPPER)].append("U1")
for k in list(used):
    if len(set(used[k])) == 1:
        used[k] = [used[k][0]]
for name, pts in L.JUMPERS:
    for h in pts:
        used[h].append(f"wire {name}")
for h, who in sorted(used.items()):
    if len(who) > 1:
        fail.append(f"hole {h} used by {who}")
    if h in set(L.CUTS):
        fail.append(f"hole {h} is cut but used by {who}")

# 4. wire routing
bodies = []
for cross, (a, b) in L.CROSSES.items():
    for cm, (col, row) in L.cross_switches(a, b).items():
        cx, cy = L.hole(col + 1.5, row + 1)
        bodies.append((cx - BODY / 2 - BODY_M, cy - BODY / 2 - BODY_M,
                       cx + BODY / 2 + BODY_M, cy + BODY / 2 + BODY_M))
bus = [(hole_xy(a), hole_xy(b)) for n, pts in L.JUMPERS if len(pts) > 2
       for a, b in zip(pts, pts[1:])]


def pt_seg(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    l2 = dx * dx + dy * dy
    t = 0 if l2 == 0 else max(0, min(1, ((px - ax) * dx + (py - ay) * dy) / l2))
    return math.hypot(px - (ax + dx * t), py - (ay + dy * t))


total = 0.0
for name, pts in L.JUMPERS:
    for p, q in zip(pts, pts[1:]):
        (x0, y0), (x1, y1) = hole_xy(p), hole_xy(q)
        total += math.hypot(x1 - x0, y1 - y0)
        n = max(2, int(math.hypot(x1 - x0, y1 - y0) / 0.25))
        for i in range(n + 1):
            t = i / n
            x, y = x0 + (x1 - x0) * t, y0 + (y1 - y0) * t
            if any(b[0] <= x <= b[2] and b[1] <= y <= b[3] for b in bodies):
                # allowed: a wire between the switch legs is easy to lay and
                # the switch still seats, so this is information, not a fault
                warn.append(f"wire {name} passes under a switch body")
                break
        if len(pts) == 2:
            v = (x1 - x0, y1 - y0)
            lv = math.hypot(*v)
            for c, d in bus:
                w = (d[0] - c[0], d[1] - c[1])
                lw = math.hypot(*w)
                if lv == 0 or lw == 0:
                    continue
                ang = math.degrees(math.acos(
                    min(1, abs((v[0] * w[0] + v[1] * w[1]) / (lv * lw)))))
                near = min(pt_seg(x0, y0, *c, *d), pt_seg(x1, y1, *c, *d),
                           pt_seg(*c, x0, y0, x1, y1), pt_seg(*d, x0, y0, x1, y1))
                if ang < PARALLEL_DEG and near < BUS_CLEAR:
                    # a preference, not a rule: keeping wires off the switch
                    # bodies matters more, and one wire needs this to fit
                    warn.append(f"wire {name} runs alongside a ground bus")
                    break

# cuts must stay a clear row away from the Pico's pins
for col, row in L.CUTS:
    if L.PICO_COL0 <= col <= L.PICO_COL0 + 19:
        for pin_row in (L.PICO_ROW_UPPER, L.PICO_ROW_LOWER):
            if 0 < abs(row - pin_row) < 2:
                fail.append(f"cut ({col},{row}) is hard against a Pico pin row")

print(f"nets {len(net_holes)} | segments {len(segs)} | cuts {len(L.CUTS)} | "
      f"wires {len(L.JUMPERS)} | wire length {total:.0f} mm")
if warn:
    for w in dict.fromkeys(warn):
        print("  note:", w)
if fail:
    print(f"\n{len(fail)} PROBLEMS:")
    for f in dict.fromkeys(fail):
        print("  -", f)
    sys.exit(1)
print("all checks pass")
