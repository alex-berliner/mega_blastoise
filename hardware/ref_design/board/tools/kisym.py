"""Read pin geometry out of KiCad symbol libraries (.kicad_sym).

A library pin's (at ...) is its connection point (its tip, not its root at the
body); this places those for a symbol at a given position and rotation, for the
drawing kit (tools/schdraw.py) to wire to.
"""
import math
import os
import re

SYSLIB = "/usr/share/kicad/symbols"
# the project's own symbol library (tools/sym_lib.py), as the project's sym-lib-table names it
PROJECT_LIBS = {"battler": os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                                        "lib", "battler.kicad_sym")}


def parse(text):
    tok = re.findall(r'\(|\)|"(?:[^"\\]|\\.)*"|[^\s()]+', text)
    stack, cur = [], []
    for t in tok:
        if t == "(":
            stack.append(cur); cur = []
        elif t == ")":
            done = cur; cur = stack.pop(); cur.append(done)
        else:
            cur.append(re.sub(r'\\(.)', lambda m: "\n" if m.group(1) == "n" else m.group(1), t[1:-1])
                       if t.startswith('"') else t)
    return cur[0]


class Library:
    """A symbol library's symbols by name, each parsed when first asked for. KiCad's own
    libraries are large (Device.kicad_sym alone takes seconds to parse whole) and a schematic
    uses a few symbols from each: the top-level symbols are found by their lines (KiCad writes
    each as a tab and (symbol "NAME"), and only a used symbol's block is parsed. A symbol not
    found that way comes from the whole library, parsed as before. parse_fn: the parser
    (schwrite's keeps quoted strings apart)."""

    HEAD = re.compile(r'^\t\(symbol ("(?:[^"\\]|\\.)*")\s', re.M)
    TOKEN = re.compile(r'\(|\)|"(?:[^"\\]|\\.)*"')

    def __init__(self, path, parse_fn):
        self.text, self.parse, self.whole, self.done = open(path).read(), parse_fn, None, {}
        self.heads = {}
        for m in self.HEAD.finditer(self.text):
            self.heads.setdefault(_unquote(m.group(1)), []).append(m.start() + 1)

    def _whole(self):
        if self.whole is None:
            root = self.parse(self.text)
            self.whole = {n[1]: n for n in root[1:] if isinstance(n, list) and n[0] == "symbol"}
        return self.whole

    def __getitem__(self, name):
        if name not in self.done:
            self.done[name] = self._block(name)
        return self.done[name]

    def _block(self, name):
        at = self.heads.get(name, [])
        if len(at) == 1 and self.whole is None:
            depth = 0
            for m in self.TOKEN.finditer(self.text, at[0]):
                depth += {"(": 1, ")": -1}.get(m.group(0), 0)
                if depth == 0:
                    node = self.parse(self.text[at[0]:m.end()])
                    if isinstance(node, list) and node[:2] == ["symbol", name]:
                        return node
                    break
        return self._whole()[name]


def _unquote(t):
    return re.sub(r'\\(.)', lambda m: "\n" if m.group(1) == "n" else m.group(1), t[1:-1])


_cache = {}


def library(path, parse_fn=None):
    if (path, parse_fn) not in _cache:
        _cache[path, parse_fn] = Library(path, parse_fn or parse)
    return _cache[path, parse_fn]


def find(n, key):
    return [c for c in n if isinstance(c, list) and c and c[0] == key]


def pins(lib_id, extra_libs=None):
    """{pin_number: dict(name, x, y, angle, length, type)} in library coords (Y up), unit 1."""
    nick, name = lib_id.split(":")
    path = (extra_libs or PROJECT_LIBS).get(nick) or os.path.join(SYSLIB, nick + ".kicad_sym")
    lib = library(path)
    sym = lib[name]
    ext = find(sym, "extends")
    if ext:
        return pins(f"{nick}:{ext[0][1]}", extra_libs)
    out = {}
    for unit in find(sym, "symbol"):
        m = re.match(r".*_(\d+)_(\d+)$", unit[1])
        u, style = int(m.group(1)), int(m.group(2))
        if u not in (0, 1) or style not in (0, 1):
            continue
        for p in find(unit, "pin"):
            at = find(p, "at")[0]
            ln = find(p, "length")[0]
            num = find(p, "number")[0][1]
            nm = find(p, "name")[0][1]
            out[num] = dict(name=nm, x=float(at[1]), y=float(at[2]),
                            angle=float(at[3]) if len(at) > 3 else 0.0,
                            length=float(ln[1]), type=p[1])
    return out


def placed(lib_id, x, y, rot=0, extra_libs=None):
    """Pin tips in schematic coords (Y down) for a symbol placed at x,y rotated rot (CCW deg).
    Also returns the outward direction for a wire stub."""
    r = math.radians(rot)
    res = {}
    for num, p in pins(lib_id, extra_libs).items():
        lx = p["x"] * math.cos(r) - p["y"] * math.sin(r)
        ly = p["x"] * math.sin(r) + p["y"] * math.cos(r)
        sx, sy = round(x + lx, 4), round(y - ly, 4)
        a = math.radians(p["angle"] + 180 + rot)   # pin points at the body; outward is opposite
        dx, dy = round(math.cos(a)), round(-math.sin(a))
        d = {(1, 0): "right", (-1, 0): "left", (0, -1): "up", (0, 1): "down"}[(dx, dy)]
        res[num] = dict(p, sx=sx, sy=sy, out=d)
    return res
