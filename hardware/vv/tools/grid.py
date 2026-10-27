#!/usr/bin/env python3
"""The coverage grid: check classes (rows) by subsystems (columns), generated from the register.

    python3 hardware/vv/tools/grid.py           # write hardware/vv/grid.md
    python3 hardware/vv/tools/grid.py --check   # exit 1 when grid.md is not what the register gives

It reads verification.csv (each check's 'classes' and 'subsystems', space-separated) and
grid_reasons.csv (class, subsystem, status NOT_NEEDED or ACCEPTED_GAP, reason). Nothing in
grid.md is typed: re-run this after any change to either file (run_checks.py does).

A cell is the checks of board ref_design or both, not CUT or RETIRED, tagged with that class and
that subsystem (the tags are a cross product: tag a check only with classes it really targets for
every subsystem it names). A cell is:

  COVERED      a built check targets it (PASS, FAIL or NOT_RUN); it shows every check's id and the
               worst status among them (FAIL, NOT_BUILT, NOT_RUN, PASS, worst first), and the
               earliest phase when no built check is an ORDER check. Class 15 before the order is
               a paper estimate: it is COVERED only by a built analysis (method A).
  planned      only NOT_BUILT checks target it: a work item, still a GAP for the order gate.
  GAP          nothing targets it and grid_reasons.csv gives no reason.
  NOT_NEEDED   grid_reasons.csv says why the class cannot apply to the subsystem.
  ACCEPTED_GAP grid_reasons.csv proposes it; it counts only when Alex has signed the line
               'GRID-<class>-<subsystem>' ACCEPTED in releases/APPROVALS.md (check_trace passes the
               signed ones in). Until then it is a GAP.

The order gate (check_trace.py --gate order, rule R9) fails while any cell of classes 1-15 is
GAP, planned or an unsigned ACCEPTED_GAP. Classes 16-18 belong to EVT, DVT and PVT and are shown,
not gated here; 19 and 20 are whole-register classes, listed under the table (subsystem
'register').

Python 3 standard library only.
"""
import csv
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))

CLASSES = {
    1: "Requirements", 2: "Architecture fit", 3: "Connectivity", 4: "Values and worst case",
    5: "Ratings and derating", 6: "States and sequencing", 7: "Faults and safety", 8: "Schematic vs layout",
    9: "Footprints", 10: "Fab rules", 11: "Assembly data", 12: "Signals and timing", 13: "Fit", 14: "Mating",
    15: "Thermal", 16: "Bring-up measurements", 17: "Stress and life", 18: "Production test",
    19: "Right baseline", 20: "Can the check fail",
}
GROUPS = {1: "A", 2: "A", 3: "B", 4: "B", 5: "B", 6: "B", 7: "B", 8: "C", 9: "C", 10: "C", 11: "C", 12: "C",
          13: "D", 14: "D", 15: "D", 16: "E", 17: "E", 18: "E", 19: "E", 20: "E"}
SUBSYSTEMS = ["power", "cell", "mcu", "display", "audio", "buttons", "usb_debug", "case"]
REGISTER = "register"                    # the whole register: classes 19 and 20
WHOLE = (19, 20)
ORDER_CLASSES = range(1, 16)             # 1-14, and 15 by a paper estimate
LATER = {16: "EVT", 17: "DVT", 18: "PVT"}
REASON_COLS = ["class", "subsystem", "status", "reason"]
REASON_STATUS = ("NOT_NEEDED", "ACCEPTED_GAP")
WORST = ["FAIL", "NOT_BUILT", "NOT_RUN", "PASS"]
BUILT = ("PASS", "FAIL", "NOT_RUN")
PHASES = ["ORDER", "CASE", "EVT", "DVT", "PVT", "SALE"]
LIVE_BOARDS = ("ref_design", "both")


def read(vv):
    """(checks, reasons, problems): the register's rows and what is malformed in them."""
    problems = []
    with open(os.path.join(vv, "verification.csv"), newline="", encoding="utf-8") as f:
        vers = [dict(r, _where=f"verification.csv:{n}") for n, r in enumerate(csv.DictReader(f), 2)]
    reasons = []
    path = os.path.join(vv, "grid_reasons.csv")
    if not os.path.exists(path):
        problems.append(("grid_reasons.csv", "missing"))
    else:
        with open(path, newline="", encoding="utf-8") as f:
            rows = list(csv.reader(f))
        if not rows or rows[0] != REASON_COLS:
            problems.append(("grid_reasons.csv", f"header is {rows[0] if rows else []}, want {REASON_COLS}"))
        else:
            for n, r in enumerate(rows[1:], 2):
                if not any(c.strip() for c in r):
                    continue
                if len(r) != len(REASON_COLS):
                    problems.append((f"grid_reasons.csv:{n}", f"{len(r)} fields, want {len(REASON_COLS)}"))
                    continue
                reasons.append(dict(zip(REASON_COLS, (c.strip() for c in r)), _where=f"grid_reasons.csv:{n}"))
    for v in vers:
        v["cls"], v["subs"] = [], []
        if "classes" not in v or "subsystems" not in v:
            problems.append((v["_where"], "verification.csv has no 'classes' and 'subsystems' columns"))
            break
        live = v["last_result"] not in ("CUT", "RETIRED")
        for tok in v["classes"].split():
            if not re.fullmatch(r"\d{1,2}", tok) or int(tok) not in CLASSES:
                problems.append((v["_where"], f"{v['id']}: class {tok!r} is not 1-20"))
            else:
                v["cls"].append(int(tok))
        for tok in v["subsystems"].split():
            if tok not in SUBSYSTEMS + [REGISTER]:
                problems.append((v["_where"], f"{v['id']}: subsystem {tok!r} is not one of "
                                              f"{' '.join(SUBSYSTEMS + [REGISTER])}"))
            else:
                v["subs"].append(tok)
        if live and (not v["cls"] or not v["subs"]):
            problems.append((v["_where"], f"{v['id']}: a live check names its classes and subsystems "
                                          "(tag what it really targets)"))
        if live and any(c in WHOLE for c in v["cls"]) != (REGISTER in v["subs"]):
            problems.append((v["_where"], f"{v['id']}: classes 19 and 20 go with the subsystem 'register', and "
                                          "'register' only with them"))
    seen = set()
    for r in reasons:
        w = r["_where"]
        try:
            c = int(r["class"])
        except ValueError:
            c = None
        if c not in CLASSES or c in WHOLE:
            problems.append((w, f"class {r['class']!r} is not 1-18"))
        if r["subsystem"] not in SUBSYSTEMS:
            problems.append((w, f"subsystem {r['subsystem']!r} is not one of {' '.join(SUBSYSTEMS)}"))
        if r["status"] not in REASON_STATUS:
            problems.append((w, f"status {r['status']!r} is not one of {REASON_STATUS}"))
        if len(r["reason"]) < 20:
            problems.append((w, "a reason is a written sentence saying why"))
        key = (r["class"], r["subsystem"])
        if key in seen:
            problems.append((w, f"duplicate cell {r['class']} {r['subsystem']}"))
        seen.add(key)
        r["c"] = c
    return vers, reasons, problems


def cells(vers, reasons, signed=frozenset()):
    """{(class, subsystem): cell}. signed: the 'GRID-<class>-<subsystem>' items Alex has signed ACCEPTED."""
    live = [v for v in vers if v.get("board") in LIVE_BOARDS and v["last_result"] not in ("CUT", "RETIRED")]
    why = {(r.get("c"), r["subsystem"]): r for r in reasons}
    out, problems = {}, []
    for c in CLASSES:
        if c in WHOLE:
            continue
        for s in SUBSYSTEMS:
            ids = [v for v in live if c in v["cls"] and s in v["subs"]]
            # class 15 before the order is a paper estimate: only an analysis covers it or plans it
            counts_ = [v for v in ids if c != 15 or "A" in v["method"].split("/")]
            built = [v for v in counts_ if v["last_result"] in BUILT]
            r = why.get((c, s))
            cell = {"checks": ids, "built": built, "reason": r}
            if r and r["status"] == "NOT_NEEDED" and ids:
                problems.append((r["_where"], f"class {c} {s} is NOT_NEEDED, but {' '.join(v['id'] for v in ids)} "
                                              "target it: drop the reason or the tag"))
            if built:
                cell["state"] = "COVERED"
            elif r and r["status"] == "NOT_NEEDED":
                cell["state"] = "NOT_NEEDED"
            elif r and r["status"] == "ACCEPTED_GAP":
                cell["state"] = "ACCEPTED_GAP" if f"GRID-{c}-{s}" in signed else "GAP"
            elif counts_:
                cell["state"] = "planned"
            else:
                cell["state"] = "GAP"
            cell["planned"] = [v for v in counts_ if v["last_result"] == "NOT_BUILT"]
            cell["worst"] = min((v["last_result"] for v in ids), key=WORST.index) if ids else ""
            phases = [v["phase"] for v in built] or [v["phase"] for v in counts_] or [v["phase"] for v in ids]
            cell["earliest"] = min(phases, key=PHASES.index) if phases else ""
            out[(c, s)] = cell
    return out, problems


def order_gaps(grid):
    """The cells the order gate holds: classes 1-15 that are GAP or planned."""
    return [(c, s, cell) for (c, s), cell in sorted(grid.items()) if c in ORDER_CLASSES
            and cell["state"] in ("GAP", "planned")]


def counts(grid, c):
    k = {"COVERED": 0, "planned": 0, "GAP": 0, "NOT_NEEDED": 0, "ACCEPTED_GAP": 0}
    for s in SUBSYSTEMS:
        k[grid[(c, s)]["state"]] += 1
    return k


def text_cell(cell):
    ids = " ".join(v["id"] for v in cell["checks"])
    st = cell["state"]
    if st == "COVERED":
        late = f", earliest {cell['earliest']}" if cell["earliest"] != "ORDER" else ""
        return f"COVERED, worst {cell['worst']}{late}: {ids}"
    if st == "planned":
        return f"planned ({cell['earliest']}): {ids}"
    if st == "NOT_NEEDED":
        return "NOT_NEEDED"
    if st == "ACCEPTED_GAP":
        return "ACCEPTED_GAP (signed)"
    if cell["reason"]:
        return "GAP (acceptance not signed)"
    return f"GAP{(' (measured only, no paper analysis): ' + ids) if ids else ''}"


def render(vers, reasons, grid):
    live = [v for v in vers if v.get("board") in LIVE_BOARDS and v["last_result"] not in ("CUT", "RETIRED")]
    L = ["# Coverage grid: check classes by subsystem (REF-A, hardware/ref_design)", "",
         "Generated by `python3 hardware/vv/tools/grid.py` from verification.csv (each check's `classes` and "
         "`subsystems`) and grid_reasons.csv. Do not edit by hand: change those files and re-run it. "
         "`check_trace.py --gate order` fails (R9) while any cell of classes 1-15 is GAP or planned. "
         "Class 15 counts before the order only through a built analysis (A). Classes 16-18 belong to EVT, DVT "
         "and PVT and are shown, not gated by the order; 19 and 20 are whole-register classes, listed below. "
         "Only checks of board ref_design or both count; CUT and RETIRED checks never do.", "",
         "A cell reads: COVERED, worst <status> among its checks (FAIL, NOT_BUILT, NOT_RUN, PASS) and, "
         "when no built check is an ORDER check, the earliest phase; planned (only NOT_BUILT checks: a GAP for "
         "the order gate until one is built); GAP; NOT_NEEDED (grid_reasons.csv says why).", ""]
    L.append("| Class | " + " | ".join(SUBSYSTEMS) + " |")
    L.append("|---|" + "---|" * len(SUBSYSTEMS))
    for c in CLASSES:
        if c in WHOLE:
            continue
        L.append(f"| {GROUPS[c]}{c} {CLASSES[c]}{' (' + LATER[c] + ')' if c in LATER else ''} | "
                 + " | ".join(text_cell(grid[(c, s)]) for s in SUBSYSTEMS) + " |")
    L += ["", "## Counts per class", "",
          "| Class | covered | planned | gap | not needed | accepted gap | held by the order gate |",
          "|---|---|---|---|---|---|---|"]
    tot = {"COVERED": 0, "planned": 0, "GAP": 0, "NOT_NEEDED": 0, "ACCEPTED_GAP": 0}
    held = 0
    for c in CLASSES:
        if c in WHOLE:
            continue
        k = counts(grid, c)
        h = k["GAP"] + k["planned"] if c in ORDER_CLASSES else 0
        held += h
        for x in tot:
            tot[x] += k[x]
        L.append(f"| {c} {CLASSES[c]} | {k['COVERED']} | {k['planned']} | {k['GAP']} | {k['NOT_NEEDED']} | "
                 f"{k['ACCEPTED_GAP']} | {h if c in ORDER_CLASSES else '-'} |")
    L.append(f"| all | {tot['COVERED']} | {tot['planned']} | {tot['GAP']} | {tot['NOT_NEEDED']} | "
             f"{tot['ACCEPTED_GAP']} | {held} |")
    L += ["", "## Whole-register classes", ""]
    for c in WHOLE:
        ids = [v for v in live if c in v["cls"]]
        worst = min((v["last_result"] for v in ids), key=WORST.index) if ids else ""
        L.append(f"- {GROUPS[c]}{c} {CLASSES[c]}: " + (f"{worst}: " + " ".join(v["id"] for v in ids) if ids else "GAP"))
    gaps = order_gaps(grid)
    later = [(c, s, cell) for (c, s), cell in sorted(grid.items()) if c not in ORDER_CLASSES and cell["state"] == "GAP"]
    L += ["", f"## Gaps, each with its proposed check", "",
          f"The order gate holds {len(gaps)} cells of classes 1-15 "
          f"({sum(1 for *_, x in gaps if x['state'] == 'GAP')} GAP, "
          f"{sum(1 for *_, x in gaps if x['state'] == 'planned')} planned); classes 16-18 have {len(later)} GAP, "
          "for their own phases. A planned cell names the NOT_BUILT checks that would cover it, with what each "
          "computes or measures and its pass limit, from verification.csv; a bare GAP has none yet: add one as "
          "a NOT_BUILT row.", ""]
    for c, s, cell in gaps + later:
        held = "" if c in ORDER_CLASSES else f" ({LATER[c]}, not the order gate)"
        if not cell["planned"]:
            L.append(f"- {c} {CLASSES[c]} / {s}{held}: GAP, no check proposed yet")
        for v in cell["planned"]:
            L.append(f"- {c} {CLASSES[c]} / {s}{held}: {v['id']} {v['title']} ({v['method']}, NOT_BUILT, "
                     f"{v['phase']}). Pass: {v['pass_limit']}")
    fails = [v for v in live if v["last_result"] == "FAIL"]
    L += ["", f"## FAIL ({len(fails)})", ""]
    for v in fails:
        L.append(f"- {v['id']} {v['title']} ({v['phase']}): classes {v['classes']}; {v['subsystems']}")
    nn = [r for r in reasons if r["status"] == "NOT_NEEDED"]
    L += ["", f"## Not needed ({len(nn)}), from grid_reasons.csv", ""]
    for r in sorted(nn, key=lambda r: (r["c"] or 0, SUBSYSTEMS.index(r["subsystem"])
                                       if r["subsystem"] in SUBSYSTEMS else 99)):
        L.append(f"- {r['class']} {CLASSES.get(r['c'], '?')} / {r['subsystem']}: {r['reason']}")
    acc = [r for r in reasons if r["status"] == "ACCEPTED_GAP"]
    if acc:
        L += ["", f"## Accepted gaps proposed ({len(acc)}): each counts only once Alex signs GRID-<class>-<subsystem>", ""]
        for r in acc:
            L.append(f"- {r['class']} {CLASSES.get(r['c'], '?')} / {r['subsystem']}: {r['reason']}")
    return "\n".join(L) + "\n"


FOOTER = re.compile(r"^Last updated: .*$", re.M)


def same(a, b):
    return FOOTER.sub("", a).rstrip() == FOOTER.sub("", b).rstrip()


def build(vv, signed=frozenset()):
    vers, reasons, problems = read(vv)
    grid, p2 = cells(vers, reasons, signed)
    return vers, reasons, grid, problems + p2


def main():
    import datetime
    vv = os.path.dirname(HERE)
    if "--vv" in sys.argv:
        vv = os.path.abspath(sys.argv[sys.argv.index("--vv") + 1])
    sys.path.insert(0, HERE)
    import check_trace as ct               # the signed approvals, read the one way check_trace reads them
    ct.VV = vv
    _, eff, _ = ct.parse_approvals()
    signed = frozenset(k for k, a in eff.items() if k.startswith("GRID-") and a["decision"] == "ACCEPTED")
    vers, reasons, grid, problems = build(vv, signed)
    for w, m in problems:
        print(f"  {w}: {m}")
    if problems:
        print(f"grid: {len(problems)} problem(s) in the register; grid.md not written")
        return 1
    body = render(vers, reasons, grid)
    path = os.path.join(vv, "grid.md")
    old = open(path, encoding="utf-8").read() if os.path.exists(path) else ""
    if "--check" in sys.argv:
        ok = same(old, body)
        print("grid.md " + ("matches the register" if ok else "is stale: run python3 hardware/vv/tools/grid.py"))
        return 0 if ok else 1
    if not same(old, body):
        stamp = datetime.datetime.now().astimezone().strftime("%Y-%m-%d %H:%M %Z")
        open(path, "w", encoding="utf-8").write(body + f"\nLast updated: {stamp} (generated)\n")
        print(f"wrote {os.path.relpath(path)}")
    else:
        print("grid.md unchanged")
    gaps = order_gaps(grid)
    print(f"{len(gaps)} cells of classes 1-15 the order gate holds "
          f"({sum(1 for *_, c in gaps if c['state'] == 'GAP')} GAP, "
          f"{sum(1 for *_, c in gaps if c['state'] == 'planned')} planned)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
