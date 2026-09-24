"""KiCad's own reading of the schematic, as Python data.

    comps, nets = netlist.read()
    comps["U4"] -> dict(value, footprint, fields, path)   path = "/<sheet uuid>/<symbol uuid>"
    nets["GND"] -> [("U4", "81"), ...]

Exports with kicad-cli each time, so it always matches the sheets on disk.
"""
import os
import subprocess
import tempfile

import kisym

BOARD = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCH = os.path.join(BOARD, "battler.kicad_sch")


def _find(n, key):
    return [c for c in n if isinstance(c, list) and c and c[0] == key]


def _val(n, key):
    f = _find(n, key)
    return f[0][1] if f and len(f[0]) > 1 else None


def read(sch=SCH):
    with tempfile.TemporaryDirectory() as tmp:
        out = os.path.join(tmp, "net.net")
        r = subprocess.run(["kicad-cli", "sch", "export", "netlist", "--format", "kicadsexpr", "-o", out, sch],
                           capture_output=True, text=True)
        if r.returncode:
            raise SystemExit(r.stderr)
        root = kisym.parse(open(out).read())
    comps = {}
    for c in _find(_find(root, "components")[0], "comp"):
        fields = {}
        for f in _find(_find(c, "fields")[0], "field") if _find(c, "fields") else []:
            fields[_val(f, "name")] = f[2] if len(f) > 2 else ""
        props = {_val(p, "name"): _val(p, "value") for p in _find(c, "property")}
        sheet_ts = _val(_find(c, "sheetpath")[0], "tstamps")
        comps[_val(c, "ref")] = dict(value=_val(c, "value"), footprint=_val(c, "footprint") or "",
                                     fields=fields, props=props, path=sheet_ts + _val(c, "tstamps"),
                                     sheetname=_val(_find(c, "sheetpath")[0], "names"))
    nets = {}
    for n in _find(_find(root, "nets")[0], "net"):
        nets[_val(n, "name")] = [(_val(x, "ref"), _val(x, "pin")) for x in _find(n, "node")]
    return comps, nets
