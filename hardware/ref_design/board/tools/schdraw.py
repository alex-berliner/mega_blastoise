"""Drawing kit for build_sch.py: hand layouts of schematic blocks, checked before
they are written (tools/schwrite.py).

A Sheet collects what a page should hold (symbols with their field positions,
wires, junctions, labels, power symbols, text), then

  * finish() splits every wire where another wire, pin or label lands on it and
    puts a junction where three or more connections meet;
  * check() traces the drawing (wires, labels, power symbols) and compares every
    group of joined pins with design.py, and looks for drawing faults: wires
    through symbols or text, text on text, wires crossing, parts off the page.

Conventions:

  * Fields are horizontal and centred: a part's Reference above its Value, beside
    or above and below its body (place()'s f).
  * Power symbols are KiCad's own (power:GND, power:+3V3, ...) with the rail's name
    as the symbol's Value by its arrow, GND's under its bars. A power symbol's
    "#PWR" reference is hidden, as KiCad does.
  * A local label sits at the end of a wire with its text on the wire (left or
    down) or running on past its end (right). A global label points the way its
    wire travels: one fed from the right points left.
"""
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.dirname(HERE))

import design  # noqa: E402
import kisym  # noqa: E402

G = 2.54
H = 1.27
FRAME = (12.7, 12.7, 284.3, 197.3)
TITLE = (176.0, 165.0)         # title block: x > 176 and y > 165
CW = 0.97                      # character advance, times the font size
TXT = 1.27


def snap(v, g=1.27):
    return round(round(v / g) * g, 4)


# advance of each glyph of KiCad's stroke font, in units of the font size (measured
# from kicad-cli's SVG); anything not listed counts as 1.0
GLYPH = {"a": 0.905, "b": 0.905, "c": 0.857, "d": 0.905, "e": 0.857, "f": 0.571, "g": 0.905, "h": 0.905,
         "i": 0.476, "j": 0.476, "k": 0.81, "l": 0.524, "m": 1.333, "n": 0.905, "o": 0.905, "p": 0.905,
         "q": 0.905, "r": 0.619, "s": 0.81, "t": 0.571, "u": 0.905, "v": 0.762, "w": 1.048, "x": 0.81,
         "y": 0.762, "z": 0.81, "A": 0.857, "B": 1.0, "C": 1.0, "D": 1.0, "E": 0.905, "F": 0.857, "G": 1.0,
         "H": 1.048, "I": 0.476, "J": 0.762, "K": 1.0, "L": 0.81, "M": 1.143, "N": 1.048, "O": 1.048,
         "P": 1.0, "Q": 1.048, "R": 1.0, "S": 0.952, "T": 0.762, "U": 1.048, "V": 0.857, "W": 1.143,
         "X": 0.952, "Y": 0.857, "Z": 0.952, " ": 0.762, "_": 0.762, "-": 1.238, "+": 1.238, "/": 1.048,
         "(": 0.667, ")": 0.667, ".": 0.476, ",": 0.476, ":": 0.476, ";": 0.476, "=": 1.238, "~": 0.714,
         "{": 0.667, "}": 0.667, "[": 0.667, "]": 0.667, "<": 1.238, ">": 1.238, "%": 1.143, "'": 0.476,
         "!": 0.476, "?": 0.857, "#": 1.0, "&": 1.238, "*": 0.762, "|": 0.952}


def tw(s, size=TXT):
    """Drawn width of a string in KiCad's stroke font."""
    if not s:
        return 0.0
    return (sum(GLYPH.get(ch, 0.952 if ch.isdigit() else 1.0) for ch in s) - 0.19) * size


def _key(p):
    return (round(p[0], 2), round(p[1], 2))


# ---- library geometry ---------------------------------------------------------------
_body = {}


def body_box(lib_id):
    """Graphic extent of a symbol (lib coordinates, Y up), pins excluded."""
    if lib_id in _body:
        return _body[lib_id]
    nick, name = lib_id.split(":")
    path = kisym.PROJECT_LIBS.get(nick) or os.path.join(kisym.SYSLIB, nick + ".kicad_sym")
    sym = kisym.library(path)[name]
    ext = kisym.find(sym, "extends")
    if ext:
        return body_box(f"{nick}:{ext[0][1]}")
    xs, ys = [], []

    def walk(n):
        if not isinstance(n, list) or not n:
            return
        if n[0] == "pin":
            return
        if n[0] == "xy" or (n[0] in ("start", "end", "mid", "center") and len(n) >= 3):
            xs.append(float(n[1])); ys.append(float(n[2]))
        if n[0] == "circle":
            c = kisym.find(n, "center")[0]
            r = float(kisym.find(n, "radius")[0][1])
            xs.extend([float(c[1]) - r, float(c[1]) + r]); ys.extend([float(c[2]) - r, float(c[2]) + r])
            return
        for c in n:
            walk(c)

    for unit in kisym.find(sym, "symbol"):
        if unit[1].endswith(("_0_0", "_0_1", "_1_0", "_1_1")):
            for item in unit[2:]:
                if isinstance(item, list) and item and item[0] in ("polyline", "rectangle", "circle", "arc", "bezier"):
                    walk(item)
    _body[lib_id] = (min(xs), min(ys), max(xs), max(ys)) if xs else (0, 0, 0, 0)
    return _body[lib_id]


def rot_pt(x, y, rot):
    """Lib point (Y up) to schematic offset (Y down) for a symbol rotated rot (CCW)."""
    r = math.radians(rot)
    lx = x * math.cos(r) - y * math.sin(r)
    ly = x * math.sin(r) + y * math.cos(r)
    return lx, -ly


def placed_box(lib_id, x, y, rot):
    x0, y0, x1, y1 = body_box(lib_id)
    pts = [rot_pt(a, b, rot) for a in (x0, x1) for b in (y0, y1)]
    return (x + min(p[0] for p in pts), y + min(p[1] for p in pts),
            x + max(p[0] for p in pts), y + max(p[1] for p in pts))


# ---- the sheet ----------------------------------------------------------------------
class Sheet:
    def __init__(self, file):
        self.file = file
        self.parts = {p["ref"]: p for p in design.PARTS if p["sheet"] == file}
        self.sym = {}          # ref -> dict(x, y, rot, fields={key: (cx, cy, shown, size)})
        self.wires = []        # [(x1, y1, x2, y2)]
        self.labels = []       # [(net, x, y, rot, kind)]  kind: local | global
        self.power = []        # [(net, x, y, rot, (value cx, cy) or None)]
        self.texts = []        # [(cx, cy, size, text, rot)] centre-anchored
        self.sheets = []       # root only: [(file, name, x, y, w, h)]
        self.ncs = []
        self.junctions = []
        self.notes = []        # (x, y, text) problems found by check()

    # -- parts --
    def place(self, ref, x, y, rot=0, f=None):
        """Put ref's origin at x, y. f: where Reference and Value go (Reference on top):
        'r'/'l' or ('r'|'l', dy): stacked beside the body; 'c': Reference above the body
        and Value below it; (dx, dy) or ('at', cx, cy): stacked around that point;
        ('value', cx, cy, size): Reference hidden, Value alone there."""
        p = self.parts[ref]
        x, y = snap(x), snap(y)
        x0, y0, x1, y1 = placed_box(p["lib"], x, y, rot)
        if f is None:
            vertical2 = len(p["pins"]) == 2 and rot in (0, 180) and p["lib"] in (
                "Device:R", "Device:C", "Device:L", "Device:Thermistor_NTC")
            f = "r" if vertical2 else "c"
        size = TXT
        if f == "c":
            ref_at, val_at = (x, y0 - 1.6), (x, y1 + 1.6)
        elif f[0] == "value":
            ref_at, val_at = None, (f[1], f[2])
            size = f[3] if len(f) > 3 else TXT
        else:
            if f in ("r", "l") or f[0] in ("r", "l"):
                side, dy = (f, 0) if isinstance(f, str) else f
                w = max(tw(ref), tw(p["value"])) / 2
                cx = x1 + 0.6 + w if side == "r" else x0 - 0.6 - w
                cx = (math.ceil if side == "r" else math.floor)(cx / 0.635) * 0.635
                cy = y + dy
            elif f[0] == "at":
                cx, cy = f[1], f[2]
            else:
                cx, cy = x + f[0], y + f[1]
            ref_at, val_at = (cx, cy - H), (cx, cy + H)
        fields = {"Reference": (round(ref_at[0], 4), round(ref_at[1], 4), True, size) if ref_at else (x, y, False, size),
                  "Value": (round(val_at[0], 4), round(val_at[1], 4), True, size)}
        self.sym[ref] = dict(x=x, y=y, rot=rot, fields=fields)
        return self

    def pin(self, ref, num):
        s = self.sym[ref]
        p = kisym.placed(self.parts[ref]["lib"], s["x"], s["y"], s["rot"])[num]
        return (snap(p["sx"]), snap(p["sy"]))

    def pout(self, ref, num):
        s = self.sym[ref]
        return kisym.placed(self.parts[ref]["lib"], s["x"], s["y"], s["rot"])[num]["out"]

    def P(self, spec):
        """'R1.2' -> pin tip, or a point tuple as is."""
        if isinstance(spec, str):
            ref, num = spec.rsplit(".", 1)
            return self.pin(ref, num)
        return (snap(spec[0]), snap(spec[1]))

    # -- wires --
    def wire(self, *pts, v=False):
        """Polyline through points (pin specs or tuples). A diagonal step becomes an L:
        horizontal first, or vertical first with v=True."""
        pts = [self.P(p) for p in pts]
        out = [pts[0]]
        for b in pts[1:]:
            a = out[-1]
            if a[0] != b[0] and a[1] != b[1]:
                out.append((a[0], b[1]) if v else (b[0], a[1]))
            out.append(b)
        for a, b in zip(out, out[1:]):
            if a != b:
                self.wires.append((a[0], a[1], b[0], b[1]))
        return out[-1]

    def stub(self, spec, length=G, d=None):
        """Wire from a pin straight out (its own direction, or d) by length. Returns the end."""
        a = self.P(spec)
        if d is None:
            ref, num = spec.rsplit(".", 1)
            d = self.pout(ref, num)
        dx, dy = {"right": (1, 0), "left": (-1, 0), "up": (0, -1), "down": (0, 1)}[d]
        b = (snap(a[0] + dx * length), snap(a[1] + dy * length))
        if length:
            self.wire(a, b)
        return b

    # -- power --
    def gnd(self, at):
        """GND symbol with its pin at `at`. Its name goes under the bars, or beside them where
        that is taken (gnd_names, once the sheet is drawn)."""
        at = self.P(at)
        self.power.append(("GND", at[0], at[1], 0, None))
        return at

    def gnd_names(self):
        """Name every GND symbol (Alex, 2026-10-20: a bare triangle didn't say what it was):
        under its bars, else to their right, else to their left, wherever the text clears
        everything else on the sheet."""
        w = tw("GND") / 2
        for i, (net, x, y, rot, val) in enumerate(self.power):
            if net != "GND" or val:
                continue
            others = [b for k, o, b in self.boxes()] + [(min(a, c) - 0.2, min(b, d) - 0.2, max(a, c) + 0.2, max(b, d) + 0.2)
                                                       for a, b, c, d in self.wires]
            for cx, cy in ((x, y + 3.81), (x + 1.9 + w, y + 1.8), (x - 1.9 - w, y + 1.8)):
                box = (cx - w, cy - 0.75, cx + w, cy + 0.75)
                if not any(_overlap(box, b, 0.05) for b in others):
                    break
            else:
                cx, cy = x, y + 3.81             # nowhere clear: under it, and the check says so
            self.power[i] = (net, x, y, rot, (round(cx, 4), round(cy, 4)))

    def rail(self, net, at, d="up"):
        """Power symbol for net with its pin at `at`, pointing d (up/left/right/down), the
        rail's name (the symbol's Value) by its arrow."""
        at = self.P(at)
        x, y = at
        rot = {"up": 0, "left": 90, "down": 180, "right": 270}[d]
        val = {"up": (x, y - 3.81), "down": (x, y + 3.81), "left": (x - 3.3 - tw(net) / 2, y),
               "right": (x + 3.3 + tw(net) / 2, y)}[d]
        self.power.append((net, x, y, rot, (round(val[0], 4), round(val[1], 4))))
        return at

    def flag(self, ref, at):
        """PWR_FLAG ref (from design.py) on the wire at `at`, flag pointing up, its name
        small to the right of it."""
        at = self.P(at)
        self.place(ref, at[0], at[1], 0, f=("value", at[0] + 1.6 + tw("PWR_FLAG", 1.0) / 2, at[1] - 1.9, 1.0))
        return at

    # -- labels --
    def kind(self, net):
        return design.label_kind(net)

    def label(self, net, at, d="right", vertical=False):
        """Label for net at the end `at` of a wire that arrives travelling d.
        Local labels: text lies along the wire (d left/down) or runs on past its end.
        Global labels point the way the wire travels (left, or right for right, up and
        down); vertical=True stands either kind on end, reading upwards."""
        at = self.P(at)
        x, y = at
        if self.kind(net) == "power":
            raise SystemExit(f"{net} is a power net: use rail()/gnd()")
        kind = self.kind(net)
        rot = 90 if vertical else 180 if (kind == "global" and d == "left") else 0
        self.labels.append((net, x, y, rot, kind))
        return at

    def tag(self, spec, length=None, d=None, vertical=False, net=None):
        """Stub out of a pin and label it with the pin's net (power nets get their
        symbol). Left/down stubs of local labels get long enough to carry the text."""
        ref, num = spec.rsplit(".", 1)
        net = net or self.parts[ref]["pins"][num]
        d = d or self.pout(ref, num)
        kind = self.kind(net)
        if length is None:
            if kind == "local" and d in ("left", "down"):
                length = math.ceil((tw(net) + 1.0) / G) * G
            else:
                length = G
        end = self.stub(spec, length, d)
        if net == "GND":
            if d != "down":
                end = self.wire(end, (end[0], end[1] + G)) if d in ("left", "right") else end
            self.gnd(end)
        elif kind == "power":
            self.rail(net, end, "up" if d in ("up", "left", "right") else "down")
        else:
            self.label(net, end, d, vertical)
        return end

    def nc(self, spec):
        self.ncs.append(self.P(spec))

    def text(self, x, y, s, size=TXT, align="l", rot=0):
        """Text with its left end (align 'l') or centre ('c') at x; y is its middle."""
        cx = x + tw(s, size) / 2 if align == "l" else x
        self.texts.append((cx, y, size, s, rot))

    def heading(self, x, y, s):
        self.text(x, y, s, size=1.8)

    # -- finishing --------------------------------------------------------------------
    def points(self):
        """Every connection point that is not a wire end: pins, labels, power pins."""
        pts = {}
        for ref, s in self.sym.items():
            p = self.parts[ref]
            for num, pin in kisym.placed(p["lib"], s["x"], s["y"], s["rot"]).items():
                pts.setdefault(_key((pin["sx"], pin["sy"])), []).append(("pin", ref, num))
        for net, x, y, rot, kind in self.labels:
            pts.setdefault(_key((x, y)), []).append(("label", net, kind))
        for net, x, y, rot, val in self.power:
            pts.setdefault(_key((x, y)), []).append(("power", net))
        return pts

    def name_nets(self):
        """KiCad names a net after its label; a net drawn entirely in wire would get an
        automatic name (Net-(R1-Pad2)) and lose the name design.py and the PCB use. Put
        a local label on one wire of every group that carries none: the longest
        horizontal wire (text lying on it), else the longest vertical one. self.names
        {net: (x, y, rot)} places one by hand instead."""
        pts = self.points()
        parent = {}

        def find(a):
            while parent.setdefault(a, a) != a:
                parent[a] = parent[parent[a]]
                a = parent[a]
            return a

        for x1, y1, x2, y2 in self.wires:
            parent[find(_key((x1, y1)))] = find(_key((x2, y2)))
        for k in pts:
            find(k)
        groups = {}
        for k in list(parent):
            groups.setdefault(find(k), set()).add(k)
        for members in groups.values():
            items = [i for m in members for i in pts.get(m, [])]
            if any(i[0] in ("label", "power") for i in items):
                continue
            nets = {self.parts[i[1]]["pins"].get(i[2]) for i in items if i[0] == "pin"}
            if len(nets) != 1 or None in nets:
                continue
            net = nets.pop()
            if self.kind(net) != "local":
                continue
            if net in getattr(self, "names", {}):
                x, y, rot = self.names[net]
                self.labels.append((net, x, y, rot, "local"))
                continue
            segs = [w for w in self.wires if _key(w[:2]) in members or _key(w[2:]) in members]
            need = tw(net) + 1.0
            hs = sorted((abs(w[2] - w[0]), w) for w in _runs(segs, 1))
            vs = sorted((abs(w[3] - w[1]), w) for w in _runs(segs, 0))
            if hs and hs[-1][0] >= need:
                w = hs[-1][1]
                x = snap(min(w[0], w[2]) + (abs(w[2] - w[0]) - need) / 2, G / 2)
                self.labels.append((net, max(x, min(w[0], w[2])), w[1], 0, "local"))
            elif vs:
                w = vs[-1][1]
                self.labels.append((net, w[0], max(w[1], w[3]), 90, "local"))
            elif hs:
                w = hs[-1][1]
                self.labels.append((net, min(w[0], w[2]), w[1], 0, "local"))
            else:
                self.notes.append(f"{net}: no wire to name it on")

    def finish(self):
        self.split()
        self.name_nets()
        self.split()
        self.gnd_names()
        self.junctions = []
        pts = self.points()
        deg = {}
        for x1, y1, x2, y2 in self.wires:
            for k in (_key((x1, y1)), _key((x2, y2))):
                deg[k] = deg.get(k, 0) + 1
        for k, n in deg.items():
            here = pts.get(k, [])
            # stacked pins of one symbol (the RP2350B's IOVDD) count once
            extra = len({i[1] for i in here if i[0] == "pin"}) + len([i for i in here if i[0] == "power"])
            if n + extra >= 3:
                self.junctions.append(k)

    def split(self):
        """Merge duplicate wires and split every wire at the connection points on it."""
        pts = self.points()
        cand = set(pts)
        for w in self.wires:
            cand.add(_key(w[:2])); cand.add(_key(w[2:]))
        out = set()
        for x1, y1, x2, y2 in self.wires:
            cuts = [(x1, y1), (x2, y2)]
            for c in cand:
                if _inside(c, (x1, y1, x2, y2)):
                    cuts.append(c)
            if x1 == x2:
                cuts.sort(key=lambda c: c[1])
            else:
                cuts.sort(key=lambda c: c[0])
            for a, b in zip(cuts, cuts[1:]):
                if _key(a) != _key(b):
                    seg = (_key(a), _key(b)) if _key(a) < _key(b) else (_key(b), _key(a))
                    out.add(seg)
        self.wires = [(a[0], a[1], b[0], b[1]) for a, b in sorted(out)]

    # -- checking ---------------------------------------------------------------------
    def check(self):
        """Trace the drawing and compare with design.py. Returns (errors, warnings)."""
        errors, warns = list(self.notes), []
        missing = set(self.parts) - set(self.sym)
        if missing:
            errors.append(f"not placed: {sorted(missing)}")
        pts = self.points()
        parent = {}

        def find(a):
            while parent.setdefault(a, a) != a:
                parent[a] = parent[parent[a]]
                a = parent[a]
            return a

        def union(a, b):
            parent[find(a)] = find(b)

        for x1, y1, x2, y2 in self.wires:
            union(_key((x1, y1)), _key((x2, y2)))
        for k in pts:
            find(k)
        groups = {}
        for k in list(parent):
            groups.setdefault(find(k), []).append(k)
        nc = {_key(p) for p in self.ncs}
        net_groups = {}
        for g, members in groups.items():
            items = [i for m in members for i in pts.get(m, [])]
            nets = set()
            for i in items:
                if i[0] == "pin":
                    nets.add(self.parts[i[1]]["pins"].get(i[2], "?"))
            names = {i[1] for i in items if i[0] in ("label", "power")}
            pins = [i for i in items if i[0] == "pin"]
            wired = any(_key(w[:2]) in members or _key(w[2:]) in members for w in self.wires)
            if None in nets:
                if len(items) > 1 or wired or len(pins) > 1:
                    errors.append(f"no-connect pin joined: {pins}")
                for i in pins:
                    if self.parts[i[1]]["pins"][i[2]] is None and _key(self.pin(i[1], i[2])) not in nc:
                        errors.append(f"{i[1]}.{i[2]} has no X")
                continue
            if len(nets) > 1:
                errors.append(f"short: {sorted(nets)} via {pins[:6]}")
                continue
            if not nets:
                if names or wired:
                    errors.append(f"dangling {sorted(names) or 'wire'} near {members[0]}")
                continue
            net = nets.pop()
            if names and names != {net}:
                errors.append(f"net {net} labelled {sorted(names)}")
            if not names and len(pins) == 1:
                errors.append(f"{pins[0][1]}.{pins[0][2]} ({net}) is connected to nothing")
            net_groups.setdefault(net, []).append((names, pins))
        for net, gs in net_groups.items():
            kind = self.kind(net)
            if (len(gs) > 1 or kind != "local") and any(not names for names, _ in gs):
                bad = [pins for names, pins in gs if not names]
                errors.append(f"net {net}: a group with no label/symbol: {bad[0][:4]}")
            if kind == "local":
                total = sum(len(p) for _, p in gs)
                want = len([1 for r, p in design.nets()[net] if r in self.parts])
                if total != want:
                    errors.append(f"net {net}: {total} of {want} pins drawn")
        for net, x, y, rot, kind in self.labels:
            want = "global" if self.kind(net) == "global" else "local"
            if kind != want:
                errors.append(f"label {net} is {kind}, should be {want}")
        # wire ends hanging in the air
        ends = {}
        for x1, y1, x2, y2 in self.wires:
            for k in (_key((x1, y1)), _key((x2, y2))):
                ends[k] = ends.get(k, 0) + 1
        for k, n in ends.items():
            if n == 1 and k not in pts:
                errors.append(f"wire end at {k} touches nothing")
        warns += self.geometry()
        return errors, warns

    def boxes(self):
        """(kind, owner, box) for symbol bodies and every piece of text."""
        out = []
        for ref, s in self.sym.items():
            p = self.parts[ref]
            out.append(("body", ref, placed_box(p["lib"], s["x"], s["y"], s["rot"])))
            for key, text in (("Reference", ref), ("Value", p["value"])):
                cx, cy, shown, size = s["fields"][key]
                if shown:
                    h, w = 0.6 * size, tw(text, size) / 2
                    out.append(("field", ref, (cx - w, cy - h, cx + w, cy + h)))
        for net, x, y, rot, kind in self.labels:
            w = tw(net) + (2.0 if kind == "global" else 0)
            if kind == "global":
                b = {0: (x, y - 1.2, x + w, y + 1.2), 180: (x - w, y - 1.2, x, y + 1.2),
                     90: (x - 1.2, y - w, x + 1.2, y)}[rot]
            else:
                b = (x + 0.2, y - 1.9, x + w, y - 0.3) if rot == 0 else (x - 1.9, y - w, x - 0.3, y - 0.2)
            out.append(("label", net, b))
        for net, x, y, rot, val in self.power:
            if net == "GND":
                out.append(("body", "GND", (x - 1.27, y + 1.0, x + 1.27, y + 2.6)))
            else:
                (a, b), (c, d) = rot_pt(-0.8, 1.0, rot), rot_pt(0.8, 2.6, rot)
                out.append(("body", net, (x + min(a, c), y + min(b, d), x + max(a, c), y + max(b, d))))
            if val:
                w = tw(net) / 2
                out.append(("text", net, (val[0] - w, val[1] - 0.75, val[0] + w, val[1] + 0.75)))
        for file, name, x, y, w, h in self.sheets:
            out.append(("sheet", name, (x, y, x + w, y + h)))
            out.append(("text", name, (x, y - 2.6, x + tw(name, 1.524), y - 0.6)))
            out.append(("text", file, (x, y + h + 0.5, x + tw(file), y + h + 2.2)))
        for cx, cy, size, s, rot in self.texts:
            w, h = tw(s, size) / 2, size * 0.6
            out.append(("text", s, (cx - w, cy - h, cx + w, cy + h)))
        return out

    def geometry(self):
        warns = []
        bx = self.boxes()
        for i, (k1, o1, b1) in enumerate(bx):
            if not _on_page(b1):
                warns.append(f"{k1} {o1} off the page or on the title block {tuple(round(v, 1) for v in b1)}")
            for k2, o2, b2 in bx[i + 1:]:
                if "sheet" in (k1, k2) and _within(b1 if k2 == "sheet" else b2, b2 if k2 == "sheet" else b1):
                    continue                     # a sheet box's notes are written inside it
                if k1 == "body" and k2 == "body":
                    if _overlap(b1, b2, 0.1):
                        warns.append(f"bodies overlap: {o1} {o2}")
                    continue
                if o1 == o2 and k1 == k2 == "field":
                    continue
                if o1 == o2 and {k1, k2} == {"body", "field"} and o1 in ("U4",):
                    continue                     # its name sits in the middle of the chip
                if _overlap(b1, b2, 0.05):
                    warns.append(f"{k1} {o1} overlaps {k2} {o2}")
        for w in self.wires:
            for k, o, b in bx:
                if _seg_box(w, b, 0.25):
                    warns.append(f"wire {w} through {k} {o}")
        # pin lines: a wire crossing another part's pin away from its tip
        for ref, s in self.sym.items():
            p = self.parts[ref]
            for num, pin in kisym.placed(p["lib"], s["x"], s["y"], s["rot"]).items():
                if pin["length"] < 0.5:
                    continue
                dx, dy = {"right": (-1, 0), "left": (1, 0), "up": (0, 1), "down": (0, -1)}[pin["out"]]
                seg = (pin["sx"] + dx * 0.3, pin["sy"] + dy * 0.3,
                       pin["sx"] + dx * pin["length"], pin["sy"] + dy * pin["length"])
                for w in self.wires:
                    if _seg_seg(w, seg):
                        warns.append(f"wire {w} crosses {ref} pin {num}")
        # wire crossings (look like nothing, but worth knowing about)
        for i, a in enumerate(self.wires):
            for b in self.wires[i + 1:]:
                if _cross(a, b):
                    warns.append(f"wires cross at {_cross(a, b)}")
        return warns


def _runs(segs, horizontal):
    """Collinear touching wires merged into runs: horizontal ones (1) or vertical (0)."""
    if horizontal:
        lines = sorted((w[1], min(w[0], w[2]), max(w[0], w[2])) for w in segs if w[1] == w[3])
    else:
        lines = sorted((w[0], min(w[1], w[3]), max(w[1], w[3])) for w in segs if w[0] == w[2])
    out = []
    for c, a, b in lines:
        if out and abs(out[-1][0] - c) < 0.01 and a <= out[-1][2] + 0.01:
            out[-1][2] = max(out[-1][2], b)
        else:
            out.append([c, a, b])
    if horizontal:
        return [(a, c, b, c) for c, a, b in out]
    return [(c, a, c, b) for c, a, b in out]


def _inside(c, w):
    x1, y1, x2, y2 = w
    if x1 == x2 and abs(c[0] - x1) < 0.01:
        return min(y1, y2) + 0.01 < c[1] < max(y1, y2) - 0.01
    if y1 == y2 and abs(c[1] - y1) < 0.01:
        return min(x1, x2) + 0.01 < c[0] < max(x1, x2) - 0.01
    return False


def _within(a, b):
    """Box a wholly inside box b."""
    return b[0] <= a[0] and b[1] <= a[1] and a[2] <= b[2] and a[3] <= b[3]


def _overlap(a, b, m=0.0):
    return a[0] < b[2] - m and b[0] < a[2] - m and a[1] < b[3] - m and b[1] < a[3] - m


def _seg_box(w, b, m):
    x1, y1, x2, y2 = w
    box = (min(x1, x2) - 0.01, min(y1, y2) - 0.01, max(x1, x2) + 0.01, max(y1, y2) + 0.01)
    return _overlap(box, (b[0] + m, b[1] + m, b[2] - m, b[3] - m))


def _seg_seg(w, s):
    """Does wire w touch segment s (both axis-aligned)?"""
    a = (min(w[0], w[2]), min(w[1], w[3]), max(w[0], w[2]), max(w[1], w[3]))
    b = (min(s[0], s[2]), min(s[1], s[3]), max(s[0], s[2]), max(s[1], s[3]))
    return a[0] <= b[2] + 0.01 and b[0] <= a[2] + 0.01 and a[1] <= b[3] + 0.01 and b[1] <= a[3] + 0.01


def _cross(a, b):
    ha, hb = a[1] == a[3], b[1] == b[3]
    if ha == hb:
        if ha and abs(a[1] - b[1]) < 0.01 and max(min(a[0], a[2]), min(b[0], b[2])) < min(max(a[0], a[2]), max(b[0], b[2])) - 0.01:
            return ("overlap", a[1])
        if not ha and abs(a[0] - b[0]) < 0.01 and max(min(a[1], a[3]), min(b[1], b[3])) < min(max(a[1], a[3]), max(b[1], b[3])) - 0.01:
            return ("overlap", a[0])
        return None
    h, v = (a, b) if ha else (b, a)
    x, y = v[0], h[1]
    if min(h[0], h[2]) + 0.01 < x < max(h[0], h[2]) - 0.01 and min(v[1], v[3]) + 0.01 < y < max(v[1], v[3]) - 0.01:
        return (round(x, 2), round(y, 2))
    return None


def _on_page(b):
    if b[0] < FRAME[0] or b[1] < FRAME[1] or b[2] > FRAME[2] or b[3] > FRAME[3]:
        return False
    return not (b[2] > TITLE[0] and b[3] > TITLE[1])
