"""Price the JLC-assembled parts from the local JLCPCB parts database.

    python3 tools/cost.py [boards]     # default 10 boards

Prices and stock are from Konnect's JLCPCB snapshot (~/.konnect/jlcpcb.db,
see its last_updated column), so treat them as estimates. Extended parts
carry JLC's per-part setup fee; basic and preferred parts don't.
"""
import json
import os
import sqlite3
import sys
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))
import design  # noqa: E402

DB = os.path.expanduser("~/.konnect/jlcpcb.db")
EXTENDED_FEE = 3.0     # USD per unique extended part per order (JLC economic assembly)


def price_at(price_json, qty):
    tiers = sorted(json.loads(price_json or "[]"), key=lambda t: t["qty"])
    p = tiers[0]["price"] if tiers else 0.0
    for t in tiers:
        if qty >= t["qty"]:
            p = t["price"]
    return p


def main():
    boards = int(sys.argv[1]) if len(sys.argv) > 1 else 10
    db = sqlite3.connect(DB)
    count = Counter(p["lcsc"] for p in design.PARTS if p["lcsc"])
    rows, total, fees = [], 0.0, 0.0
    for lcsc, n in sorted(count.items()):
        r = db.execute("select mfr_part, library_type, stock, price_json, last_updated from components where lcsc=?",
                       (lcsc,)).fetchone()
        if not r:
            rows.append((lcsc, "?", "missing", 0, n, 0.0))
            continue
        mpn, lib, stock, pj, updated = r
        unit = price_at(pj, n * boards)
        total += unit * n
        if lib == "Extended":
            fees += EXTENDED_FEE
        rows.append((lcsc, mpn, lib, stock, n, unit))
    print(f"{'LCSC':10} {'part':28} {'library':9} {'stock':>8} {'per board':>9} {'unit $':>8}")
    for lcsc, mpn, lib, stock, n, unit in sorted(rows, key=lambda r: -r[5] * r[4]):
        flag = "  LOW STOCK" if isinstance(stock, int) and stock < 50 * boards else ""
        print(f"{lcsc:10} {mpn[:28]:28} {lib:9} {stock:>8} {n:>9} {unit:>8.3f}{flag}")
    ext = sum(1 for r in rows if r[2] == "Extended")
    print(f"\n{len(rows)} unique parts ({ext} extended), {sum(count.values())} placements per board")
    print(f"parts per board: ${total:.2f};  extended-part setup: ${fees:.0f} per order "
          f"(${fees / boards:.2f} per board at {boards})")


if __name__ == "__main__":
    main()
