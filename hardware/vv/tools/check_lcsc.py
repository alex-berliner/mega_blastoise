#!/usr/bin/env python3
"""The LCSC identity of every JLC-assembled line against design.py (VER-186).

    python3 hardware/vv/tools/check_lcsc.py            # exit 1 on an unexplained difference, 2 if the database is missing
    python3 hardware/vv/tools/check_lcsc.py --seed     # R61's LCSC number swapped for R5's (C25890) in memory: must exit 1

It reads design.py's parts (through design_numbers.design()) and JLC's parts database as Konnect keeps it on this
machine (~/.konnect/jlcpcb.db, the same snapshot board/tools/cost.py prices from; read only). The script prints the
database's size, sha256 and snapshot date, so the evidence names what it read. For every LCSC number on the BOM
(each once, with all its designators) it compares:
  1. the MPN: design.py's 'mpn' against the record's mfr_part, and the maker against its manufacturer;
  2. the package: the footprint's package (0402, SOT-23-5, TSSOP-16, ...) against the record's, or for a connector, a
     module or a switch the footprint's pitch or size against the record's;
  3. the pin count: the footprint's numbered copper pads on the routed board against the record's solder joints;
  4. the value of a resistor, a capacitor or the thermistor: design.py's value against the record's description, and a
     resistor's tolerance against the 1% design_numbers.R_TOL assumes;
and prints, without judging, the record's ratings (voltage, power, temperature: VER-31 judges ratings), its library
class (Basic, Preferred, Extended) and its stock at the snapshot against 30 boards (VER-41 reads stock on order
day). A difference the EXPLAINED table below names, with its source, is printed as explained; any other fails.

Python 3 standard library and pcbnew (the routed board's pads).
"""
import datetime
import hashlib
import os
import re
import sqlite3
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import design_numbers as dn  # noqa: E402

DB = os.path.expanduser("~/.konnect/jlcpcb.db")
BOARDS = 30                      # the second order (the first is 5)
ATTRITION = 1.1                  # VER-41's stock rule: quantity x 1.1 (+ JLC's attrition, not modelled here)
# (LCSC, field): why a difference is not a defect, with its source
EXPLAINED = {
    ("C13612", "package"): "JLC names SOT-23-6 'SOT-23-6L'; the land is VER-125's (srv05-4.pdf)",
    ("C15220", "package"): "JLC's 'QFN-16-EP(3x3)' is TI's RGT0016C VQFN, 3 x 3 mm with an exposed pad (VER-125, DEF-84)",
    ("C94625", "package"): "the FM8002A's own datasheet names its package SOP-8 (fm8002a.txt:17), KiCad's SOIC-8 3.9 x 4.9 mm",
    ("C2028", "package"): "JLC's '0402x4' is the 4 x 0402 convex array (uniroyal_4d02.pdf)",
    ("C165948", "package"): "JLC gives only 'SMD'; the land is HRO's drawing (VER-125, hro_type-c-31-m-12.pdf)",
    ("C165948", "pins"): "the 4 shell tabs (SH) are on the board and not among JLC's 16 joints (the 16 A/B pins)",
    ("C114218", "package"): "JLC gives only 'SMD'; the land is Hirose's DM3 drawing (VER-125, hirose_dm3_catalog_C114218.pdf)",
    ("C2759975", "package"): "JLC's 3 x 2.8 mm is the body, 3.00 x 2.75 mm on ROCPU's drawing (rocpu_tps850.txt:91, 102)",
    ("C2759975", "pins"): "JLC counts 6 joints; this is the 'V no pegs' part (rocpu_tps850.txt:73), 4 J-leads on 4 pads "
                          "(rocpu_tps850.txt:118, 127); JLC's preview (VER-28) shows which land it places on",
    ("C3013946", "package"): "JLC's 'SMD,19.2x18mm' is the WROOM-1U's 19.2 x 18.0 mm outline (esp32-s3-wroom-1.txt:3264, 3449)",
    ("C3013946", "pins"): "the 40 castellations and the exposed pad 41, which the footprint pastes through 9 windows of "
                          "0.9 mm; JLC counts 40 + 9 = 49 joints (the pad's size is DEF-84's)",
    ("C2028", "tolerance"): "RN1-RN4 are 47k pull-ups on the buttons' 74HC165 inputs: 5% moves no threshold "
                            "(no design_numbers check reads them)",
}
PKG = [(r"^[RC]_(0402|0603|0805)_", r"\1"), (r"^R_Array_Convex_4x0402", "0402x4"), (r"^SOT-23-5", "SOT-23-5"),
       (r"^SOT-23-6", "SOT-23-6"), (r"^SOT-23$", "SOT-23"), (r"^D_SOD-323", "SOD-323"), (r"^SOIC-8_", "SOIC-8"),
       (r"^TSSOP-16_", "TSSOP-16"), (r"^VQFN-16-1EP_3x3mm", "VQFN-16")]
UNIT = {"p": 1e-12, "n": 1e-9, "u": 1e-6, "µ": 1e-6, "m": 1e-3, "": 1.0, "k": 1e3, "K": 1e3, "M": 1e6}


def package_of(fp):
    name = fp.split(":")[-1]
    for rx, rep in PKG:
        if re.search(rx, name):
            return re.sub(r"^.*$", re.sub(rx, rep, re.search(rx, name).group(0)), name)
    return None


def pitch_of(fp):
    m = re.search(r"_P(\d+\.\d+)mm", fp)
    return float(m.group(1)) if m else None


def num(s):
    m = re.fullmatch(r"(\d+(?:\.\d+)?)([pnuµmkKM]?)", s)
    return float(m.group(1)) * UNIT[m.group(2)] if m else None


def main():
    seed = "--seed" in sys.argv
    if not os.path.isfile(DB):
        print(f"BLOCKED: {DB} (JLC's parts database, Konnect's snapshot) is not on this machine")
        print("RESULT: BLOCKED")
        return 2
    h = hashlib.sha256()
    with open(DB, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    db = sqlite3.connect(f"file:{DB}?mode=ro", uri=True)
    n, lo, hi = db.execute("select count(*), min(last_updated), max(last_updated) from components").fetchone()
    when = datetime.datetime.fromtimestamp(hi, datetime.timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    print(f"[{DB}: {os.path.getsize(DB)} bytes, sha256 {h.hexdigest()}; {n} parts, snapshot {when}]")
    parts, _ = dn.design()
    if seed:
        parts["R61"] = dict(parts["R61"], lcsc="C25890")
        print("SEEDED: R61's LCSC number is R5's (C25890)")
    pcb, b = dn.board()
    pads = {}
    for f in b.GetFootprints():
        pads[f.GetReference()] = sum(1 for p in f.Pads() if p.GetNumber() and p.GetAttribute() != pcb.PAD_ATTRIB_NPTH)
    lines = {}
    for ref, p in parts.items():
        if p["bom"] and p["lcsc"] and not p.get("dnp"):
            lines.setdefault(p["lcsc"], []).append(ref)
    bad, explained, extended, short = 0, 0, [], []
    for lcsc, refs in sorted(lines.items(), key=lambda kv: kv[1][0]):
        refs.sort(key=lambda r: (re.sub(r"\d", "", r), int(re.sub(r"\D", "", r) or 0)))
        p = parts[refs[0]]
        row = db.execute("select mfr_part, package, solder_joints, manufacturer, library_type, stock, description "
                         "from components where lcsc=?", (lcsc,)).fetchone()
        head = f"{lcsc} {','.join(refs)} ({len(refs)})"
        if not row:
            print(f"  FAIL  {head}: not in the database")
            bad += 1
            continue
        mpn, pkg, joints, maker, lib, stock, desc = row
        diffs = []
        # every designator on one LCSC number must be the same part: value, MPN, maker and footprint
        kinds = {(parts[r]["value"], parts[r]["mpn"], parts[r]["mfr"], parts[r]["fp"]) for r in refs}
        if len(kinds) > 1:
            diffs.append(("designators", "designators disagree: " + "; ".join(
                f"{','.join(r for r in refs if (parts[r]['value'], parts[r]['mpn'], parts[r]['mfr'], parts[r]['fp']) == k)} "
                f"{k[0]} {k[1]}" for k in sorted(kinds))))
        if (p["mpn"] or "").replace(" ", "").upper() != (mpn or "").replace(" ", "").upper():
            diffs.append(("mpn", f"MPN {p['mpn']} != {mpn}"))
        mk = re.sub(r"[^a-z]", "", (p["mfr"] or "").lower())[:6]
        if mk and mk not in re.sub(r"[^a-z]", "", (maker or "").lower()):
            diffs.append(("maker", f"maker {p['mfr']} != {maker}"))
        mine = package_of(p["fp"])
        pitch = pitch_of(p["fp"])
        if mine:
            if mine != pkg:
                diffs.append(("package", f"package {mine} (footprint) != {pkg}"))
        elif pitch is not None:
            m = re.search(r"P=(\d+(?:\.\d+)?)mm", pkg or "")
            if not m or abs(float(m.group(1)) - pitch) > 1e-6:
                diffs.append(("package", f"pitch {pitch} mm (footprint) != {pkg}"))
        else:
            diffs.append(("package", f"footprint {p['fp'].split(':')[-1]} against {pkg}"))
        mine_pins = pads.get(refs[0])
        if mine_pins != joints:
            diffs.append(("pins", f"{mine_pins} numbered pads on the board != {joints} solder joints"))
        val = p["value"].split()[0]
        if refs[0][0] in "RC" or refs[0].startswith(("TH", "RN")):
            unit = "F" if refs[0][0] == "C" else "Ω"
            want = num(val.replace("R", ""))
            got = re.search(r"(\d+(?:\.\d+)?)([pnuµmkKM]?)" + unit, desc or "")
            gv = num(got.group(1) + got.group(2)) if got else None
            if want is None or gv is None or abs(gv - want) > 1e-6 * max(abs(want), 1e-15):
                diffs.append(("value", f"value {p['value']} != {got.group(0) if got else 'none in the description'}"))
            if refs[0].startswith("R"):              # resistors and the arrays: design_numbers assumes 1%
                tol = re.search(r"±(\d+(?:\.\d+)?)%", desc or "")
                if not tol or float(tol.group(1)) > dn.R_TOL * 100 + 1e-9:
                    diffs.append(("tolerance", f"tolerance {tol.group(0) if tol else 'not given'} against "
                                               f"{dn.R_TOL:.0%} (design_numbers.R_TOL)"))
            if refs[0].startswith("TH"):
                bv = re.search(r"B(\d{4})", p["value"])
                if bv and f"{bv.group(1)}K" not in (desc or ""):
                    diffs.append(("value", f"B{bv.group(1)} not in the description"))
        need = len(refs) * BOARDS * ATTRITION
        if (stock or 0) < need:
            short.append(f"{lcsc} {stock} < {need:.0f}")
        if lib == "Extended":
            extended.append(lcsc)
        ratings = " ".join(t for t in re.findall(r"-?\d+℃~\+?\d+℃(?:@\(Tj\))?|\d+(?:\.\d+)?m?W|\d+(?:\.\d+)?V(?:~\d+(?:\.\d+)?V)?",
                                                  desc or "")[:4])
        unexplained = [t for k, t in diffs if (lcsc, k) not in EXPLAINED]
        why = [f"{t} (explained: {EXPLAINED[(lcsc, k)]})" for k, t in diffs if (lcsc, k) in EXPLAINED]
        explained += len(why)
        bad += bool(unexplained)
        print(("  FAIL  " if unexplained else "  ok    ") + f"{head}: {mpn}, {pkg}, {joints} joints, {lib}, stock "
              f"{stock} at the snapshot; ratings {ratings or '-'}" + ("; " + "; ".join(unexplained) if unexplained else "")
              + ("; " + "; ".join(why) if why else ""))
    print(f"{len(lines)} LCSC lines, {sum(len(v) for v in lines.values())} placements; {bad} with an unexplained "
          f"difference, {explained} differences explained; {len(extended)} Extended lines (JLC's per-part fee): "
          f"{' '.join(extended)}")
    print(f"stock at the snapshot ({when}) under {BOARDS} boards x {ATTRITION}: " + (", ".join(short) or "none")
          + " (stock is VER-41's, read on order day)")
    print("RESULT: " + ("PASS" if not bad else "FAIL"))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
