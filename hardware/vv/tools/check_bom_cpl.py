#!/usr/bin/env python3
"""BOM, CPL and PCB fields against design.py (VER-130).

    python3 hardware/vv/tools/check_bom_cpl.py          # exit 1 on any difference
    python3 hardware/vv/tools/check_bom_cpl.py --seed   # gives C1 another LCSC number in memory: must exit 1

  1. every part design.py lists against battler.kicad_pcb: the footprint is there, with design.py's value,
     LCSC number and footprint id, on the top side if JLC places it; and no footprint on the board that
     design.py does not list (pcbnew);
  2. the BOM and CPL fab.py writes (its own bom_cpl(), run into a temp dir): every part design.py marks for
     JLC (bom True, not DNP, an LCSC number) exactly once in the CPL and once in the BOM, with its LCSC
     number, top side only; no part design.py leaves out (bom False or DNP) and none of build_pcb.py's
     NO_ASSEMBLY (both read from the files, never typed here);
  3. the counts: BOM lines and CPL rows derived from design.py, written by fab.py and stated in
     docs/assembly.md must agree;
  4. each passive's manufacturer part number decoded against design.py's value: UNI-ROYAL's resistor code
     (4 digits, or 3 and a multiplier letter: J 0.1, K 0.01, L 0.001), Samsung's CLxx (EIA size, value code,
     tolerance letter, voltage letter) and FH's 0402 codes; the voltage letter is printed against the
     highest rail the part's nets reach (a class 5 matter: printed, not judged here).

Nothing is written under hardware/ref_design. Python 3 standard library and pcbnew.
"""
import ast
import csv
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import design_numbers as dn  # noqa: E402

UR_MULT = {"J": 0.1, "K": 0.01, "L": 0.001}
SAMSUNG_SIZE = {"03": "0201", "05": "0402", "10": "0603", "21": "0805", "31": "1206"}
SAMSUNG_V = {"R": 4.0, "Q": 6.3, "P": 10.0, "O": 16.0, "A": 25.0, "B": 50.0, "C": 100.0}
FH_V = {"250": 25.0, "500": 50.0, "100": 10.0, "160": 16.0, "6R3": 6.3}
RAIL_V = {"VBUS": 5.5, "VBUS_C1": 5.5, "VSYS": 4.5, "VBAT": 4.2, "AMP_VDD": 4.5, "LED_A": 4.5, "PWR_EN": 4.5,
          "+3V3": 3.4, "3V3_LDO": 3.4, "LCD_3V3": 3.4, "+1V1": 1.2, "VREG_AVDD": 3.4}


def code_value(code, three_plus_letter=True):
    if re.fullmatch(r"\d{4}", code):
        return int(code[:3]) * 10 ** int(code[3])
    m = re.fullmatch(r"(\d{3})([JKL])", code)
    if m:
        return int(m.group(1)) * UR_MULT[m.group(2)]
    return None


def decode(p):
    """(value in ohm or farad, tolerance, voltage) from the MPN, or None if the MPN's maker is not decoded."""
    mpn = p["mpn"] or ""
    m = re.fullmatch(r"(0402|0603|1206)W[GA4]F(\w{4})T[5C]E", mpn)        # UNI-ROYAL chip resistors, 1% (F)
    if m:
        return code_value(m.group(2)), 0.01, None, m.group(1)
    m = re.fullmatch(r"4D02WGJ(\d{3})(\d)TCE", mpn)                        # UNI-ROYAL 4 x 0402 array, 5% (J)
    if m:
        return int(m.group(1)) * 10 ** int(m.group(2)), 0.05, None, "4x0402"
    m = re.fullmatch(r"CL(\d\d)([AB])(\d)(\d)(\d)([JKM])([A-Z])\w+", mpn)   # Samsung MLCC
    if m:
        pf = int(m.group(3) + m.group(4)) * 10 ** int(m.group(5))
        return pf * 1e-12, {"J": 0.05, "K": 0.10, "M": 0.20}[m.group(6)], SAMSUNG_V.get(m.group(7)), \
            SAMSUNG_SIZE.get(m.group(1))
    m = re.fullmatch(r"0402(?:CG|B)(\d)(\d)(\d)([JKM])(\w{3})NT", mpn)       # FH 0402
    if m:
        pf = int(m.group(1) + m.group(2)) * 10 ** int(m.group(3))
        return pf * 1e-12, {"J": 0.05, "K": 0.10, "M": 0.20}[m.group(4)], FH_V.get(m.group(5)), "0402"
    return None


def no_assembly():
    src = open(os.path.join(dn.BOARD, "tools", "build_pcb.py")).read()
    tree = ast.parse(src)
    for node in tree.body:
        if isinstance(node, ast.Assign) and any(getattr(t, "id", "") == "NO_ASSEMBLY" for t in node.targets):
            return set(ast.literal_eval(node.value))
    sys.exit("check_bom_cpl: build_pcb.py has no NO_ASSEMBLY")


def main():
    pcb, b = dn.board()
    parts, _ = dn.design()
    if "--seed" in sys.argv:
        parts["C1"] = dict(parts["C1"], lcsc="C15849")
        print("SEEDED: C1's LCSC number C96446 -> C15849 in design.py's parts (in memory)")
    bad = []

    def fail(msg):
        bad.append(msg)
        print("  FAIL  " + msg)

    real = {r: p for r, p in parts.items() if not r.startswith("#")}
    fps = {f.GetReference(): f for f in b.GetFootprints()}
    print("1. design.py against battler.kicad_pcb's footprints")
    for ref, p in sorted(real.items()):
        f = fps.get(ref)
        if f is None:
            fail(f"{ref} is in design.py, not on the board")
            continue
        fpid = f"{f.GetFPID().GetLibNickname()}:{f.GetFPID().GetLibItemName()}"
        if fpid != p["fp"]:
            fail(f"{ref} footprint {fpid}, design.py {p['fp']}")
        if f.GetValue() != p["value"]:
            fail(f"{ref} value {f.GetValue()!r}, design.py {p['value']!r}")
        lcsc = f.GetFieldText("LCSC") if f.HasField("LCSC") else ""
        want = (p["lcsc"] or "") if p["bom"] and not p["dnp"] else ""
        if lcsc != want and not (not p["bom"] and lcsc == ""):
            fail(f"{ref} LCSC {lcsc!r} on the board, design.py {want!r}")
        if want and f.IsFlipped():
            fail(f"{ref} is on the bottom: JLC's Economic assembly is top side only")
    extra = sorted(set(fps) - set(real))
    if extra:
        fail(f"on the board, not in design.py: {', '.join(extra)}")
    print(f"  {len(real)} parts in design.py, {len(fps)} footprints on the board")

    print("2. fab.py's BOM and CPL")
    fab = dn.fab_module()
    n_lines, n_rows, skipped = fab.bom_cpl()
    bom = list(csv.DictReader(open(os.path.join(fab.OUT, "battler_bom_jlc.csv"))))
    cpl = list(csv.DictReader(open(os.path.join(fab.OUT, "battler_cpl_jlc.csv"))))
    placed = {r: p for r, p in real.items() if p["bom"] and not p["dnp"] and p["lcsc"]}
    cpl_refs = [r["Designator"] for r in cpl]
    dup = sorted({r for r in cpl_refs if cpl_refs.count(r) > 1})
    if dup:
        fail(f"in the CPL more than once: {', '.join(dup)}")
    if set(cpl_refs) != set(placed):
        fail(f"CPL against design.py: missing {sorted(set(placed) - set(cpl_refs))}, extra {sorted(set(cpl_refs) - set(placed))}")
    side = sorted(r["Designator"] for r in cpl if r["Layer"] != "Top")
    if side:
        fail(f"CPL rows not on Top: {', '.join(side)}")
    bom_ref = {}
    for line in bom:
        for ref in line["Designator"].split(","):
            if ref in bom_ref:
                fail(f"{ref} on two BOM lines")
            bom_ref[ref] = line["LCSC Part #"]
    if set(bom_ref) != set(placed):
        fail(f"BOM against design.py: missing {sorted(set(placed) - set(bom_ref))}, extra {sorted(set(bom_ref) - set(placed))}")
    for ref, lcsc in sorted(bom_ref.items()):
        if ref in placed and placed[ref]["lcsc"] != lcsc:
            fail(f"{ref} BOM LCSC {lcsc}, design.py {placed[ref]['lcsc']}")
    left_out = {r for r, p in real.items() if not p["bom"] or p["dnp"]} | no_assembly()
    leaked = sorted((set(bom_ref) | set(cpl_refs)) & left_out)
    if leaked:
        fail(f"in the BOM or CPL though design.py or NO_ASSEMBLY leaves them out: {', '.join(leaked)}")
    print(f"  CPL {len(cpl)} rows, BOM {len(bom)} lines; {len(placed)} parts design.py gives JLC; "
          f"{len(left_out)} left out (bom False or DNP, or NO_ASSEMBLY)")

    print("3. the counts")
    lines = len({p["lcsc"] for p in placed.values()})
    text = open(os.path.join(dn.REF, "docs", "assembly.md")).read()
    m = re.search(r"(\d+) placements, (\d+) part numbers", text)
    stated = (int(m.group(1)), int(m.group(2))) if m else None
    print(f"  design.py: {len(placed)} placements, {lines} part numbers; fab.py: {n_rows} rows, {n_lines} lines; "
          f"assembly.md: {f'{stated[0]} placements, {stated[1]} part numbers' if stated else 'no count found'}")
    if (len(placed), lines) != (n_rows, n_lines):
        fail("fab.py's counts differ from design.py's")
    if stated != (len(placed), lines):
        fail(f"assembly.md says {stated}, design.py gives ({len(placed)}, {lines})")

    print("4. each passive's part number against its value")
    undecoded = []
    decoded = 0
    for ref, p in sorted(placed.items(), key=lambda kv: (kv[0].rstrip('0123456789'), int(re.sub(r'\D', '', kv[0]) or 0))):
        if p["lib"] not in ("Device:R", "Device:C", "Device:R_Pack04"):
            continue
        d = decode(p)
        if d is None:
            undecoded.append(f"{ref} {p['mpn']}")
            continue
        val, tol, volt, size = d
        decoded += 1
        want = dn.ohms(p["value"]) if p["lib"] != "Device:C" else dn.farads(p["value"])
        if want == 0:
            ok = val == 0
        else:
            ok = val is not None and abs(val - want) / want < 1e-6
        fpsize = re.search(r"_(\d{4})_|4x0402", p["fp"])
        size_ok = size is None or fpsize is None or size in p["fp"]
        vmax = max([RAIL_V.get(n, 3.4) for n in p["pins"].values() if n and n != "GND"] or [0])
        vtxt = f", rated {volt:g} V for {vmax:g} V" if volt else ""
        if not ok or not size_ok:
            fail(f"{ref} {p['mpn']} decodes to {val} ({size}), design.py {p['value']} ({p['fp'].split(':')[1]})")
        elif volt and volt < vmax:
            print(f"  note  {ref} {p['mpn']}{vtxt}: under its rail (class 5, VER-31's)")
    print(f"  {decoded} passives' part numbers decoded against design.py's value and footprint size")
    if undecoded:
        print(f"  not decoded (no rule for the maker): {', '.join(undecoded)}")
    print(f"{len(bad)} differences")
    print("RESULT: " + ("PASS" if not bad else "FAIL"))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
