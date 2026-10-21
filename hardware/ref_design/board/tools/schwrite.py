"""Writes schdraw Sheets as KiCad 10 schematic files.

Every file is written whole from the drawing, never patched: the library
symbols are copied out of KiCad's own libraries (a derived symbol flattened
the way KiCad caches it), and every item gets the fields, justification and
instance data KiCad itself saves. UUIDs are derived from what an item is (a
symbol from its reference, a sheet from its file), so rebuilding the same
drawing writes the same file and the board's footprints stay linked to their
symbols across rebuilds.

KiCad then reads the result back three ways in the pipeline: ERC, the netlist
check (tools/check_netlist.py) and DRC with schematic parity.
"""
import os
import re
import uuid

import kisym

VERSION = "20260306"          # KiCad 10.0 schematic format
PROJECT = "battler"
NS = uuid.UUID("6f4c2a1e-3b7d-4e55-9a10-2d8f1c0b7e42")


def uid(*key):
    return str(uuid.uuid5(NS, "/".join(str(k) for k in key)))


ROOT_UUID = uid("root")


# ---- s-expressions ------------------------------------------------------------------
class Q(str):
    """A quoted string (anything else is written bare)."""


def parse(text):
    """KiCad s-expression -> nested lists; quoted strings come back as Q."""
    tok = re.findall(r'\(|\)|"(?:[^"\\]|\\.)*"|[^\s()]+', text)
    stack, cur = [], []
    for t in tok:
        if t == "(":
            stack.append(cur)
            cur = []
        elif t == ")":
            done = cur
            cur = stack.pop()
            cur.append(done)
        elif t.startswith('"'):
            cur.append(Q(re.sub(r'\\(.)', lambda m: "\n" if m.group(1) == "n" else m.group(1), t[1:-1])))
        else:
            cur.append(t)
    return cur[0]


def _atom(a):
    if isinstance(a, Q):
        return '"' + a.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") + '"'
    if isinstance(a, float):
        return num(a)
    return str(a)


def num(v):
    s = f"{round(v, 4):.4f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


def dump(n, depth=0):
    """KiCad's layout: a list's atoms on its first line, each sub-list on its own line."""
    atoms = []
    i = 0
    while i < len(n) and not isinstance(n[i], list):
        atoms.append(_atom(n[i]))
        i += 1
    rest = n[i:]
    if not rest:
        return "(" + " ".join(atoms) + ")"
    if all(isinstance(c, list) and c and c[0] == "xy" for c in rest):
        return "(" + " ".join(atoms) + "\n" + "\t" * (depth + 1) + " ".join(dump(c) for c in rest) + \
            "\n" + "\t" * depth + ")"
    out = "(" + " ".join(atoms)
    for c in rest:
        out += "\n" + "\t" * (depth + 1) + (dump(c, depth + 1) if isinstance(c, list) else _atom(c))
    return out + "\n" + "\t" * depth + ")"


def find(n, key):
    return [c for c in n if isinstance(c, list) and c and c[0] == key]


# ---- library symbols ----------------------------------------------------------------
_libs = {}


def _library(nick):
    path = kisym.PROJECT_LIBS.get(nick) or os.path.join(kisym.SYSLIB, nick + ".kicad_sym")
    if path not in _libs:
        _libs[path] = kisym.library(path, parse)       # each symbol parsed when first used
    return _libs[path]


def _flat(nick, name):
    """The library symbol `name` with any `extends` resolved, as KiCad flattens it: the
    parent's options, graphics and pins under the child's name and fields."""
    node = _library(nick)[name]
    ext = find(node, "extends")
    if not ext:
        return node
    parent = _flat(nick, ext[0][1])
    pname = parent[1]
    own = {c[0]: c for c in node[2:] if isinstance(c, list) and c[0] not in ("property", "extends")}
    props = {p[1]: p for p in find(node, "property")}
    head, fields, units = [], [], []
    for c in parent[2:]:
        if not isinstance(c, list):
            continue
        if c[0] == "property":
            fields.append(props.pop(c[1], c))
        elif c[0] == "symbol":
            units.append(["symbol", Q(name + c[1][len(pname):])] + c[2:])
        elif c[0] != "embedded_fonts":
            head.append(own.pop(c[0], c))
    head += [c for k, c in own.items() if k != "embedded_fonts"]
    fields += list(props.values())
    return ["symbol", Q(name)] + head + fields + units + [["embedded_fonts", "no"]]


def lib_symbol(lib_id):
    nick, name = lib_id.split(":")
    return ["symbol", Q(lib_id)] + _flat(nick, name)[2:]


def lib_field(lib_id, key):
    for p in find(lib_symbol(lib_id), "property"):
        if p[1] == key:
            return str(p[2])
    return ""


def lib_pins(lib_id):
    return sorted(kisym.pins(lib_id), key=lambda s: (len(s), s))


# ---- items --------------------------------------------------------------------------
def effects(size=1.27, justify=None):
    e = ["effects", ["font", ["size", size, size]]]
    if justify:
        e.append(["justify"] + justify.split())
    return e


def prop(key, value, x, y, angle=0, size=1.27, hide=False, justify=None):
    p = ["property", Q(key), Q(value), ["at", float(x), float(y), angle]]
    if hide:
        p.append(["hide", "yes"])
    p += [["show_name", "no"], ["do_not_autoplace", "no"], effects(size, justify)]
    return p


def field_angle(rot):
    """A field's stored angle turns with its symbol: store 90 on a symbol turned 90 or
    270 so the text reads horizontally."""
    return 90 if rot % 180 else 0


def symbol(lib_id, ref, value, x, y, rot, fields, extra, path, footprint=""):
    """fields: {"Reference"|"Value": (x, y, shown, size)}. extra: [(key, value)] hidden."""
    s = ["symbol", ["lib_id", Q(lib_id)], ["at", float(x), float(y), rot], ["unit", 1], ["body_style", 1],
         ["exclude_from_sim", "no"], ["in_bom", "yes"], ["on_board", "yes"], ["in_pos_files", "yes"],
         ["dnp", "no"], ["uuid", Q(uid("sym", ref))]]
    a = field_angle(rot)
    for key, text in (("Reference", ref), ("Value", value)):
        fx, fy, shown, size = fields[key]
        s.append(prop(key, text, fx, fy, a, size, hide=not shown))
    s.append(prop("Footprint", footprint, x, y, 0, hide=True))
    s.append(prop("Datasheet", lib_field(lib_id, "Datasheet"), x, y, 0, hide=True))
    s.append(prop("Description", lib_field(lib_id, "Description"), x, y, 0, hide=True))
    for key, v in extra:
        s.append(prop(key, v, x, y, 0, hide=True))
    for n in lib_pins(lib_id):
        s.append(["pin", Q(n), ["uuid", Q(uid("pin", ref, n))]])
    s.append(["instances", ["project", Q(PROJECT), ["path", Q(path), ["reference", Q(ref)], ["unit", 1]]]])
    return s


LOCAL_JUSTIFY = {0: "left bottom", 90: "left bottom", 180: "right bottom", 270: "right bottom"}
GLOBAL_JUSTIFY = {0: "left", 90: "left", 180: "right", 270: "right"}


def label(net, x, y, rot, kind, file):
    if kind == "global":
        return ["global_label", Q(net), ["shape", "passive"], ["at", float(x), float(y), rot],
                ["fields_autoplaced", "yes"], effects(justify=GLOBAL_JUSTIFY[rot]),
                ["uuid", Q(uid(file, "glabel", net, x, y))],
                prop("Intersheetrefs", "${INTERSHEET_REFS}", x, y, 0, hide=True)]
    return ["label", Q(net), ["at", float(x), float(y), rot], effects(justify=LOCAL_JUSTIFY[rot]),
            ["uuid", Q(uid(file, "label", net, x, y))]]


def text(s, cx, cy, size, rot, file, i):
    return ["text", Q(s), ["exclude_from_sim", "no"], ["at", float(cx), float(cy), rot],
            effects(size), ["uuid", Q(uid(file, "text", i))]]


def wire(x1, y1, x2, y2, file):
    return ["wire", ["pts", ["xy", float(x1), float(y1)], ["xy", float(x2), float(y2)]],
            ["stroke", ["width", 0], ["type", "default"]], ["uuid", Q(uid(file, "wire", x1, y1, x2, y2))]]


def sheet_block(file, name, x, y, w, h, page):
    return ["sheet", ["at", float(x), float(y)], ["size", float(w), float(h)], ["exclude_from_sim", "no"],
            ["in_bom", "yes"], ["on_board", "yes"], ["dnp", "no"],
            ["stroke", ["width", 0.1524], ["type", "solid"]], ["fill", ["color", 0, 0, 0, 0.0]],
            ["uuid", Q(uid("sheet", file))],
            prop("Sheetname", name, x, y - 0.7116, 0, 1.524, justify="left bottom"),
            prop("Sheetfile", file, x, y + h + 0.5984, 0, 1.27, justify="left top"),
            ["instances", ["project", Q(PROJECT), ["path", Q("/" + ROOT_UUID), ["page", Q(str(page))]]]]]


# ---- files --------------------------------------------------------------------------
class Writer:
    """Writes every sheet of the project; power symbols are numbered across the sheets."""

    def __init__(self, board, titles, company):
        self.board = board
        self.titles = titles     # {file: title}
        self.company = company
        self.pwr = 0

    def path(self, sh):
        if sh.file == PROJECT + ".kicad_sch":
            return "/" + ROOT_UUID
        return f"/{ROOT_UUID}/{uid('sheet', sh.file)}"

    def emit(self, sh, rev):
        root = sh.file == PROJECT + ".kicad_sch"
        f = sh.file
        path = self.path(sh)
        doc = ["kicad_sch", ["version", VERSION], ["generator", Q("battler_build_sch")],
               ["generator_version", Q("10.0")], ["uuid", Q(ROOT_UUID if root else uid("sheet", f))],
               ["paper", Q("A4")],
               ["title_block", ["title", Q(self.titles[f])], ["rev", Q(rev)], ["company", Q(self.company)],
                ["comment", 1, Q("Generated from board/design.py by tools/build_sch.py: edit design.py, not this sheet")]]]
        libs = sorted({sh.parts[r]["lib"] for r in sh.sym} | {POWER_SYM[n[0]] for n in sh.power})
        doc.append(["lib_symbols"] + [lib_symbol(lib) for lib in libs])
        for x, y in sh.junctions:
            doc.append(["junction", ["at", float(x), float(y)], ["diameter", 0], ["color", 0, 0, 0, 0],
                        ["uuid", Q(uid(f, "junction", x, y))]])
        for x, y in sh.ncs:
            doc.append(["no_connect", ["at", float(x), float(y)], ["uuid", Q(uid(f, "nc", x, y))]])
        for w in sh.wires:
            doc.append(wire(*w, f))
        for i, (cx, cy, size, s, rot) in enumerate(sh.texts):
            doc.append(text(s, cx, cy, size, rot, f, i))
        for net, x, y, rot, kind in sh.labels:
            doc.append(label(net, x, y, rot, kind, f))
        for ref, s in sh.sym.items():
            p = sh.parts[ref]
            extra = [(name, p[key]) for key, name in (("lcsc", "LCSC"), ("mpn", "MPN"), ("mfr", "Manufacturer"))
                     if p.get(key)]
            if not p["bom"] and not ref.startswith("#"):
                extra.append(("BOM", "exclude"))
            doc.append(symbol(p["lib"], ref, p["value"], s["x"], s["y"], s["rot"], s["fields"], extra, path,
                              p["fp"] or ""))
        for net, x, y, rot, val in sh.power:
            self.pwr += 1
            ref = f"#PWR{self.pwr:03d}"
            fields = {"Reference": (x, y, False, 1.27),
                      "Value": (val[0], val[1], True, 1.27) if val else (x, y, False, 1.27)}
            doc.append(symbol(POWER_SYM[net], ref, net, x, y, rot, fields, [], path))
        for i, (file, name, x, y, w, h) in enumerate(sh.sheets):
            doc.append(sheet_block(file, name, x, y, w, h, i + 2))
        if root:
            doc.append(["sheet_instances", ["path", Q("/"), ["page", Q("1")]]])
        doc.append(["embedded_fonts", "no"])
        with open(os.path.join(self.board, f), "w") as fh:
            fh.write(dump(doc) + "\n")


# power symbols: KiCad names the net after the Value, so VSYS and VBAT are the generic
# rail arrow (power:VCC) with their own name, as KiCad's library intends
POWER_SYM = {"GND": "power:GND", "+3V3": "power:+3V3", "+1V1": "power:+1V1", "VBUS": "power:VBUS",
             "VSYS": "power:VCC", "VBAT": "power:VCC"}
