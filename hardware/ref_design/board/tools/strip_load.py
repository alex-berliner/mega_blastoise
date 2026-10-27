"""How hard is the board to route past the cell's hole? Counts, from design.py's nets and
placement.py's positions (no routing), the nets that must run through each strip beside the hole
and the total ratsnest length, and the parts with a pad in each strip.

    python3 tools/strip_load.py                     # this design
    python3 tools/strip_load.py --levers BASE_DIR   # BASE_DIR's design.py/placement.py, each lever
                                                    # of this one alone on top of it, and all four
                                                    # (BASE_DIR/../case/positions.json must exist;
                                                    # BASE_DIR/battler.kicad_pcb, if there, gives
                                                    # the pads of parts this design dropped)
    python3 tools/strip_load.py -v                  # and each net through a strip

Each net's pads are joined by a minimum spanning tree whose edges go round the hole (the shortest
way past its corners: across the board's full-width ends or up a strip) and round the RP2354B's
body (a pin leaves from its own side). A net "crosses" a strip
when one of its edges runs through it (0.3 mm or more inside: a pad in the strip counts too). GND
is left out (the pours carry it). Pads are placed from placement.py with each footprint's pad
offsets read from battler.kicad_pcb (the core's parts, which placement.py doesn't list, as they
are on the board).
"""
import importlib.util
import json
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD = os.path.dirname(HERE)
SKIP = {"GND"}


def load(dirpath, name):
    spec = importlib.util.spec_from_file_location(f"{name}_{abs(hash(dirpath))}", os.path.join(dirpath, f"{name}.py"))
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def pad_cache(pcb):
    import pcbnew
    b = pcbnew.LoadBoard(pcb)
    out = {}
    for fp in b.Footprints():
        pads = {}
        for p in fp.Pads():
            a, r = p.GetPosition(), p.GetFPRelativePosition()
            pads.setdefault(p.GetNumber(), (a.x / 1e6, a.y / 1e6, r.x / 1e6, r.y / 1e6))
        out[fp.GetReference()] = pads
    bb = b.GetBoardEdgesBoundingBox()
    return out, (bb.GetX() / 1e6, bb.GetRight() / 1e6)


def place(x, y, rot, side, rx, ry):
    if side == "B":
        rx = -rx
    a = math.radians(rot)
    return x + rx * math.cos(a) + ry * math.sin(a), y - rx * math.sin(a) + ry * math.cos(a)


class Geo:
    def __init__(self, hole, xmin, xmax, others=()):
        """hole: the cell's hole; others: more rectangles a connection can't cross (the core's
        body, whose pads sit on its edge)."""
        (self.x0, self.y0), (self.x1, self.y1) = hole
        self.walls = [(self.x0, self.y0, self.x1, self.y1)] + [tuple(r) for r in others]
        self.corners = [c for (x0, y0, x1, y1) in self.walls for c in ((x0, y0), (x1, y0), (x1, y1), (x0, y1))]
        self.strips = {"left": (xmin, self.y0, self.x0, self.y1), "right": (self.x1, self.y0, xmax, self.y1)}

    def blocked(self, a, b):
        """Does segment a-b pass through the inside of the hole or another wall?"""
        e = 1e-6
        for x0, y0, x1, y1 in self.walls:
            t0, t1 = clip(a, b, (x0 + e, y0 + e, x1 - e, y1 - e))
            if t1 - t0 > 1e-9:
                return True
        return False

    def path(self, a, b):
        """The shortest path a -> b round the hole (Dijkstra over a, b and the hole's corners)."""
        nodes = [a, b] + self.corners
        dist, prev, todo = {0: 0.0}, {}, set(range(len(nodes)))
        while todo:
            i = min((n for n in todo if n in dist), key=dist.get, default=None)
            if i is None or i == 1:
                break
            todo.discard(i)
            for j in todo:
                if not self.blocked(nodes[i], nodes[j]):
                    d = dist[i] + math.dist(nodes[i], nodes[j])
                    if d < dist.get(j, 1e18):
                        dist[j], prev[j] = d, i
        pts, k = [b], 1
        while k != 0:
            k = prev[k]
            pts.append(nodes[k])
        return dist[1], pts[::-1]

    def uses(self, pts):
        out = set()
        for a, b in zip(pts, pts[1:]):
            for name, r in self.strips.items():
                t0, t1 = clip(a, b, r)
                if (t1 - t0) * math.dist(a, b) >= 0.3:
                    out.add(name)
        return out


def clip(a, b, r):
    """Liang-Barsky: the parameter range of segment a-b inside rectangle r (empty: t1 <= t0)."""
    t0, t1 = 0.0, 1.0
    dx, dy = b[0] - a[0], b[1] - a[1]
    for p, q in ((-dx, a[0] - r[0]), (dx, r[2] - a[0]), (-dy, a[1] - r[1]), (dy, r[3] - a[1])):
        if abs(p) < 1e-12:
            if q < 0:
                return 0.0, 0.0
        else:
            t = q / p
            if p < 0:
                t0 = max(t0, t)
            else:
                t1 = min(t1, t)
    return t0, t1


def net_tree(pts, geo):
    """Prim's tree over pts with path lengths round the walls: (length, strips used)."""
    cache = {}

    def pd(i, j):
        k = (min(i, j), max(i, j))
        if k not in cache:
            cache[k] = geo.path(pts[i], pts[j])
        return cache[k]
    inside, used, total = {0}, set(), 0.0
    while len(inside) < len(pts):
        i, j = min(((i, j) for i in inside for j in range(len(pts)) if j not in inside), key=lambda e: pd(*e)[0])
        d, path = pd(i, j)
        total += d
        used |= geo.uses(path)
        inside.add(j)
    return total, used


def measure(conn, pos, geo, verbose=False):
    """conn: {net: [(ref, pin)]}; pos: {(ref, pin): (x, y)}."""
    total, cross, per = 0.0, {"left": [], "right": []}, {}
    for net, pins in sorted(conn.items()):
        if net in SKIP:
            continue
        pts = list(dict.fromkeys(pos[rp] for rp in pins if rp in pos))
        if len(pts) < 2:
            continue
        d, used = net_tree(pts, geo)
        total += d
        per[net] = d
        for s in used:
            cross[s].append(net)
    return len(cross["left"]), len(cross["right"]), total, cross, per


def design_data(dirpath):
    d, p = load(dirpath, "design"), load(dirpath, "placement")
    parts = {q["ref"]: dict(q["pins"]) for q in d.PARTS if not q["ref"].startswith("#")}
    return parts, dict(p.P), p.CUTOUTS[0]["box"]


def conn_pos(parts, P, pads):
    conn, pos = {}, {}
    for ref, pins in parts.items():
        for pin, net in pins.items():
            if net is None or pin not in pads.get(ref, {}):
                continue
            ax, ay, rx, ry = pads[ref][pin]
            pos[(ref, pin)] = place(*P[ref], rx, ry) if ref in P else (ax, ay)
            conn.setdefault(net, []).append((ref, pin))
    return conn, pos


# ---- the four levers of 2026-10-24, as differences between BASE and this design --------------
U4_REMOVED = {"REV0", "REV1", "REV2", "REV3", "UART_TX", "UART_RX"}
LEVERS = {
    "1 pins": dict(pins=True),
    "2 no REV": dict(remove={"R48", "R49", "R50", "R51"}, nets={"REV0", "REV1", "REV2", "REV3"}),
    "3 debug": dict(remove={"J6", "C41"}, nets={"UART_TX", "UART_RX"}, move="debug"),
    "4 short runs": dict(move="latch"),
}


def apply(base, final, lever, groups):
    parts, P = {r: dict(v) for r, v in base[0].items()}, dict(base[1])
    for ref in lever.get("remove", ()):
        parts.pop(ref, None)
        P.pop(ref, None)
    u4 = parts["U4"]
    for pin, net in list(u4.items()):
        if net in lever.get("nets", ()):
            u4[pin] = None
    if lever.get("pins"):
        fu4 = final[0]["U4"]
        moved = {net: pin for pin, net in fu4.items() if net and net not in U4_REMOVED and u4.get(pin) != net}
        for pin, net in list(u4.items()):
            if net in moved:
                u4[pin] = None
        for net, pin in moved.items():
            assert u4.get(pin) is None, f"lever 1 puts {net} on pin {pin}, which holds {u4[pin]} without the others"
            u4[pin] = net
    for ref in groups.get(lever.get("move"), ()):
        P[ref] = final[1][ref]
    return parts, P


def main():
    verbose = "-v" in sys.argv
    pads, (xmin, xmax) = pad_cache(os.path.join(BOARD, "battler.kicad_pcb"))
    final = design_data(BOARD)
    box = final[2]
    u4 = [xy[:2] for xy in pads["U4"].values()]
    core = (min(p[0] for p in u4), min(p[1] for p in u4), max(p[0] for p in u4), max(p[1] for p in u4))
    geo = Geo(((box[0], box[1]), (box[2], box[3])), xmin, xmax, [core])
    rows = []
    if "--levers" in sys.argv:
        bdir = os.path.abspath(sys.argv[sys.argv.index("--levers") + 1])
        base = design_data(bdir)
        if os.path.exists(os.path.join(bdir, "battler.kicad_pcb")):    # pads of parts this board dropped
            for ref, pp in pad_cache(os.path.join(bdir, "battler.kicad_pcb"))[0].items():
                pads.setdefault(ref, pp)
        # a lever's moved parts: those placed differently in this design, by which lever moved them
        groups = {"debug": [], "latch": []}
        for ref, at in final[1].items():
            if ref in base[1] and base[1][ref] != at:
                groups["latch" if ref in LATCH_GROUP else "debug"].append(ref)
        print("moved for lever 3:", " ".join(sorted(groups["debug"])))
        print("moved for lever 4:", " ".join(sorted(groups["latch"])))
        rows.append(("current board", base))
        for name, lever in LEVERS.items():
            rows.append((f"lever {name}", apply(base, final, lever, groups)))
    rows.append(("all four (this design)" if "--levers" in sys.argv else "this design", final))
    print(f"{'':28} {'nets left':>9} {'nets right':>10} {'parts L/R':>9} {'ratsnest mm':>12}")
    for name, (parts, P, *_) in rows:
        conn, pos = conn_pos(parts, P, pads)
        nl, nr, total, cross, per = measure(conn, pos, geo)
        inside = {k: sorted({ref for ref in P if ref in pads and any(
            r[0] <= x <= r[2] and r[1] <= y <= r[3] for (x, y) in
            (place(*P[ref], rx, ry) for (_, _, rx, ry) in pads[ref].values()))}) for k, r in geo.strips.items()}
        print(f"{name:28} {nl:9} {nr:10} {len(inside['left']):>4}/{len(inside['right']):<4} {total:12.0f}")
        if verbose:
            for s in cross:
                print(f"    {s:5}: {' '.join(cross[s])}  (parts: {' '.join(inside[s])})")
            print("    longest:", ", ".join(f"{n} {d:.0f}" for n, d in sorted(per.items(), key=lambda kv: -kv[1])[:25]))


# parts lever 4 moves: the latch's parts out of the right strip, C42 (CC1's spare) out of it too
LATCH_GROUP = {"Q1", "R15", "R16", "C47", "TP6", "D8", "Q7", "R62", "R63", "C42"}

if __name__ == "__main__":
    main()
