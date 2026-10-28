"""DRC with schematic parity, summarised; writes fab/drc.txt.

    python3 tools/check_pcb.py

Every violation fails the check, errors and warnings alike (V&V DEF-36: 13 warning types were never failed), except the
named cosmetic ones (silkscreen: JLCPCB clips silk off pads, and none of it is visible in the case).
"""
import collections
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
BOARD_DIR = os.path.dirname(HERE)
PCB = os.path.join(BOARD_DIR, "battler.kicad_pcb")
COSMETIC = {"silk_overlap", "silk_over_copper", "silk_edge_clearance", "text_height", "text_thickness"}


def main():
    with tempfile.TemporaryDirectory() as tmp:
        out = os.path.join(tmp, "drc.json")
        subprocess.run(["kicad-cli", "pcb", "drc", "--format", "json", "--severity-all", "--schematic-parity",
                        "-o", out, PCB], capture_output=True, text=True)
        d = json.load(open(out))
    lines = []
    viol = d.get("violations", [])
    unconn = d.get("unconnected_items", [])
    parity = d.get("schematic_parity", [])
    errors = [v for v in viol if v["type"] not in COSMETIC]
    by = collections.Counter((v["severity"], v["type"]) for v in viol)
    lines.append(f"DRC: {len(viol)} violations, {len(unconn)} unconnected, {len(parity)} schematic parity issues")
    for (sev, typ), n in sorted(by.items()):
        lines.append(f"  {n:4} {sev:8} {typ}{'  (cosmetic)' if typ in COSMETIC else ''}")
    for v in errors[:40]:
        lines.append(f"    {v['type']}: " + " / ".join(i["description"][:70] for i in v["items"]))
    for u in unconn[:40]:
        lines.append("    unconnected: " + " / ".join(i["description"][:70] for i in u["items"]))
    for p in parity[:20]:
        lines.append(f"    parity: {p['description'][:80]} " + " / ".join(i["description"][:50] for i in p["items"]))
    ok = not errors and not unconn and not parity
    lines.append("RESULT: " + ("PASS" if ok else "FAIL"))
    text = "\n".join(lines)
    print(text)
    os.makedirs(os.path.join(BOARD_DIR, "fab"), exist_ok=True)
    open(os.path.join(BOARD_DIR, "fab", "drc.txt"), "w").write(text + "\n")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
