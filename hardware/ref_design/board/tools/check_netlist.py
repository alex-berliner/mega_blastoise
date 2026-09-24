"""Check that the schematic says what design.py means.

    python3 tools/check_netlist.py

Exports the netlist with kicad-cli (KiCad's own reading of the sheets) and
compares it with design.py: every part present with its value, footprint and
LCSC number; every net holding exactly the pins intended; every pin marked
"no connect" in design.py left unconnected; no nets design.py doesn't know.
Exits non-zero on any difference.
"""
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD = os.path.dirname(HERE)
sys.path.insert(0, BOARD)
sys.path.insert(0, HERE)

import design  # noqa: E402
import kisym  # noqa: E402


def sexp(path):
    return kisym.parse(open(path).read())


def find(n, key):
    return [c for c in n if isinstance(c, list) and c and c[0] == key]


def val(n, key):
    f = find(n, key)
    return f[0][1] if f and len(f[0]) > 1 else None


def export():
    with tempfile.TemporaryDirectory() as tmp:
        out = os.path.join(tmp, "net.net")
        r = subprocess.run(["kicad-cli", "sch", "export", "netlist", "--format", "kicadsexpr", "-o", out,
                            os.path.join(BOARD, "battler.kicad_sch")], capture_output=True, text=True)
        if r.returncode:
            sys.exit(r.stderr)
        return sexp(out)


def main():
    net = export()
    comps = {}
    for c in find(find(net, "components")[0], "comp"):
        fields = {val(f, "name"): f[2] if len(f) > 2 else "" for f in find(find(c, "fields")[0], "field")} \
            if find(c, "fields") else {}
        comps[val(c, "ref")] = dict(value=val(c, "value"), fp=val(c, "footprint") or "", fields=fields)
    got = {}
    for n in find(find(net, "nets")[0], "net"):
        name = val(n, "name")
        got[name] = sorted((val(x, "ref"), val(x, "pin")) for x in find(n, "node"))

    sheet_name = {f: title for f, title in design.SHEETS}
    problems = []

    # parts
    for p in design.PARTS:
        if p["ref"].startswith("#"):
            continue
        c = comps.get(p["ref"])
        if c is None:
            if p["pins"]:
                problems.append(f"{p['ref']}: missing from the netlist")
            continue
        if c["value"] != p["value"]:
            problems.append(f"{p['ref']}: value {c['value']!r}, want {p['value']!r}")
        if p["fp"] and c["fp"] != p["fp"]:
            problems.append(f"{p['ref']}: footprint {c['fp']!r}, want {p['fp']!r}")
        if p["lcsc"] and c["fields"].get("LCSC") != p["lcsc"]:
            problems.append(f"{p['ref']}: LCSC {c['fields'].get('LCSC')!r}, want {p['lcsc']!r}")
    extra = set(comps) - {p["ref"] for p in design.PARTS}
    if extra:
        problems.append(f"parts not in design.py: {sorted(extra)}")

    # nets
    want = {}
    for name, nodes in design.nets().items():
        kind = design.label_kind(name)
        if kind == "local":
            sheet = next(iter(design.sheets_of(name)))
            key = f"/{sheet_name[sheet]}/{name}"
        else:
            key = name
        want[key] = sorted((r, pin) for r, pin in nodes if not r.startswith("#"))
    for key, nodes in want.items():
        have = got.get(key)
        if have is None:
            problems.append(f"net {key}: not in the netlist")
        elif have != nodes:
            miss = sorted(set(nodes) - set(have))
            more = sorted(set(have) - set(nodes))
            problems.append(f"net {key}: missing {miss} extra {more}")
    for key, nodes in got.items():
        if key in want:
            continue
        if key.startswith("unconnected-"):
            for ref, pin in nodes:
                p = next((q for q in design.PARTS if q["ref"] == ref), None)
                if p is None or pin not in p["pins"] or p["pins"][pin] is not None:
                    problems.append(f"{ref} pin {pin}: unconnected but design.py gives it a net")
            continue
        problems.append(f"net {key}: not in design.py ({nodes[:6]})")

    nc = sum(1 for p in design.PARTS for v in p["pins"].values() if v is None)
    print(f"{len(comps)} parts, {len(got)} nets in KiCad's netlist; {len(want)} named nets and {nc} no-connects in design.py")
    for pr in problems:
        print("  PROBLEM:", pr)
    print("RESULT:", "PASS" if not problems else f"FAIL ({len(problems)})")
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()
