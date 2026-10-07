"""Fetch the vendor SPICE models and translate them for ngspice.

    python3 sim/models.py          # writes sim/models/mcp6001.lib and sim/models/ao3400.lib

The models are the vendors' (Microchip's MCP6001/2/4 macromodel REV F, Alpha & Omega's
AO3400 subcircuit), so like the datasheets they are not committed: this downloads them
from public mirrors pinned to a commit and translates them.

Ubuntu's libngspice has no XSPICE code models, so PSpice POLY and TABLE sources are
rewritten as B sources named B + the original name: POLY as its polynomial, TABLE as pwl(). ngspice's pwl() extends
its end segments where PSpice's TABLE holds the end values; that only matters for GD31
and GD32, whose inputs sit far outside their tables in normal operation, so those get
nearly flat ends. Temperature coefficients are dropped (everything runs at 27 C).
"""
import os
import re
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "models")
MCP = ("https://raw.githubusercontent.com/land-boards/lb-boards/0fd705beff245706d690c88a77d7ed0b58c3f108/"
       "Power%20Cards/CCLOAD-01/SpiceSim/MCP6001.txt")
AO = ("https://raw.githubusercontent.com/rdmeneze/LTSpiceModels/5317aae70fc8d73b6eed53a70ff46ec8005ce2fa/"
      "AO3400.mod")


def fetch(url):
    with urllib.request.urlopen(url, timeout=30) as r:
        return r.read().decode("latin-1")


def poly(n, ctrl, coeffs):
    """PSpice POLY(n) with linear terms only (all this model uses)."""
    pairs = [f"V({ctrl[2 * k]},{ctrl[2 * k + 1]})".replace(",0)", ")") for k in range(n)]
    if len(coeffs) != n + 1:
        raise ValueError("non-linear POLY term")
    terms = [coeffs[0]] + [f"{c}*{p}" for c, p in zip(coeffs[1:], pairs)]
    return " + ".join(terms).replace("+ -", "- ")


def translate_mcp(src):
    # join continuation lines
    lines = []
    for raw in src.splitlines():
        if raw.startswith("+") and lines:
            lines[-1] += " " + raw[1:]
        else:
            lines.append(raw)
    out = ["* Microchip MCP6001/2/4 macromodel REV F, translated for ngspice by sim/models.py",
           "* Pins: 1 IN+  2 IN-  3 V+  4 V-  5 OUT"]
    for ln in lines:
        s = ln.strip()
        if not s or s.startswith("*"):
            out.append(ln)
            continue
        tok = s.split()
        name = tok[0].upper()
        if name.startswith(".MODEL") and tok[1].upper() in ("REXP", "DL"):
            continue
        if name[0] in "EG" and len(tok) > 3 and tok[3].upper().startswith("POLY"):
            n = int(re.search(r"\((\d+)\)", tok[3]).group(1))
            ctrl = tok[4:4 + 2 * n]
            coeffs = tok[4 + 2 * n:]
            kind = "V" if name[0] == "E" else "I"
            out.append(f"B{tok[0]} {tok[1]} {tok[2]} {kind} = {poly(n, ctrl, coeffs)}")
            continue
        if name[0] == "G" and "TABLE" in s.upper():
            expr = re.search(r"\{([^}]*)\}", s).group(1).strip().replace(" ", "")
            pts = re.findall(r"\(\s*([-\d.eEmMuUnNpPkK]+)\s*,\s*([-\d.eEmMuUnNpPkK]+)\s*\)", s.split("}", 1)[1])
            if name == "GD31":
                pts = [("-1e3", "-1.001")] + pts
            if name == "GD32":
                pts = pts + [("1e3", "1.001")]
            body = ", ".join(f"{x},{y}" for x, y in pts)
            out.append(f"B{tok[0]} {tok[1]} {tok[2]} I = pwl({expr}, {body})")
            continue
        if name[0] == "E" and "VALUE" in s.upper():
            expr = re.search(r"\{(.*)\}", s).group(1).strip()
            out.append(f"B{tok[0]} {tok[1]} {tok[2]} V = {expr}")
            continue
        if name[0] == "R":
            # R name n1 n2 [model] value [TC...]
            if len(tok) >= 5 and tok[3].upper() == "REXP":
                out.append(f"{tok[0]} {tok[1]} {tok[2]} {tok[4]}")
            else:
                out.append(f"{tok[0]} {tok[1]} {tok[2]} {tok[3]}")
            continue
        out.append(ln)
    return "\n".join(out) + "\n"


def main():
    os.makedirs(OUT, exist_ok=True)
    open(os.path.join(OUT, "mcp6001.lib"), "w").write(translate_mcp(fetch(MCP)))
    open(os.path.join(OUT, "ao3400.lib"), "w").write(
        "* Alpha & Omega AO3400 (pins D G S), from " + AO + "\n" + fetch(AO))
    print("wrote", os.path.join(OUT, "mcp6001.lib"), "and ao3400.lib")


if __name__ == "__main__":
    main()
