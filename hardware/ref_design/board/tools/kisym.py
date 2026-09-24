"""Read pin geometry out of KiCad symbol libraries (.kicad_sym).

Konnect reports a pin's root, not its tip, so labels placed at its coordinates
land inside the symbol and never connect. This computes the true connection
point of every pin for a placed symbol instead.
"""
import math
import os
import re

SYSLIB = "/usr/share/kicad/symbols"


def parse(text):
    tok = re.findall(r'\(|\)|"(?:[^"\\]|\\.)*"|[^\s()]+', text)
    stack, cur = [], []
    for t in tok:
        if t == "(":
            stack.append(cur); cur = []
        elif t == ")":
            done = cur; cur = stack.pop(); cur.append(done)
        else:
            cur.append(t[1:-1] if t.startswith('"') else t)
    return cur[0]


_cache = {}


def library(path):
    if path not in _cache:
        root = parse(open(path).read())
        _cache[path] = {n[1]: n for n in root[1:] if isinstance(n, list) and n[0] == "symbol"}
    return _cache[path]


def find(n, key):
    return [c for c in n if isinstance(c, list) and c and c[0] == key]


def pins(lib_id, extra_libs=None):
    """{pin_number: dict(name, x, y, angle, length, type)} in library coords (Y up), unit 1."""
    nick, name = lib_id.split(":")
    path = (extra_libs or {}).get(nick) or os.path.join(SYSLIB, nick + ".kicad_sym")
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
