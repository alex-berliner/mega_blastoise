#!/usr/bin/env python3
"""The coverage grid: check classes (rows) by subsystems (columns), generated from the register.

    python3 hardware/vv/tools/grid.py           # write hardware/vv/grid.md
    python3 hardware/vv/tools/grid.py --check   # exit 1 when grid.md is not what the register gives

It reads verification.csv (each check's 'classes', 'subsystems' and 'sources', space-separated) and
grid_reasons.csv (class, subsystem, status NOT_NEEDED or ACCEPTED_GAP, reason). Nothing in
grid.md is typed: re-run this after any change to either file (run_checks.py does).

A cell is the checks of board ref_design or both, not CUT or RETIRED, tagged with that class and
that subsystem (the tags are a cross product: tag a check only with classes it really targets for
every subsystem it names). A cell is:

  COVERED      a built check that can clear it targets it; it shows the checks that can clear it and
               the worst status among them (FAIL, NOT_BUILT, NOT_RUN, PASS, worst first), then the
               later checks that measure it but cannot clear it, and the earliest phase when no
               clearing check is an ORDER check (2026-10-27, the class 2-15 round: before, the worst
               was taken over every check, so a bench test that cannot exist before the boards held
               every cell at NOT_BUILT whatever its paper checks said). Classes 16-18 keep the worst
               over every check. Which checks can clear a cell:
                 classes 1-14: a check of phase ORDER or an analysis (method A) that has run (PASS
                   or FAIL). A NOT_RUN one leaves the cell planned until it runs (O-R6, 2026-10-27:
                   before, a NOT_RUN analysis counted, so the display's class 6 cell read COVERED on
                   an EVT simulation nobody had run). A bench test, an inspection or a
                   demonstration of a later phase does not clear it at all: before the boards
                   arrive its NOT_RUN means only that its procedure is written. A clearing check
                   whose phase is not ORDER is printed with its phase (VER-05@EVT).
                 class 15: a paper estimate, an analysis (A) with a result (PASS or FAIL): until
                   it has run no estimate exists.
                 classes 16-18: any built check (they are shown, not gated by the order).
  planned      only checks that cannot clear it yet target it (NOT_BUILT or NOT_RUN): a work item,
               still a GAP for the order gate.
  GAP          nothing that could clear it targets it and grid_reasons.csv gives no reason; the
               cell names the later checks that measure it but cannot clear it.
  NOT_NEEDED   grid_reasons.csv says why the class cannot apply to the subsystem.
  ACCEPTED_GAP grid_reasons.csv proposes it; it counts only when Alex has signed the line
               'GRID-<class>-<subsystem>' ACCEPTED in releases/APPROVALS.md (check_trace passes the
               signed ones in). Until then it is a GAP. It needs both: a signed line with no
               ACCEPTED_GAP row clears nothing either, and is listed (PLAN.md step 9.5).

Two marks on a check (2026-10-28), shown after its id and counted in a cell's worst status:

  BLOCKED      a PASS whose 'sources' name a datasheet that is not in hardware/ref_design/datasheets:
               it rests on a stand-in, so it clears no cell and no gate counts it.
  ACCEPTED     a check that a defect Alex moved to a later gate with a signed DF-ACCEPT line names:
               the order gate lets it through, and it is not a PASS (check_trace passes these in).

The order gate (check_trace.py --gate order, rule R9) fails while any cell of classes 1-15 is
GAP, planned or an unsigned ACCEPTED_GAP (the audit of 2026-10-27: a NOT_RUN bench test of a later
phase no longer clears a cell of classes 1-14, and a class 15 analysis clears only once it has run).
R9 is coverage, not results: a cell's ORDER checks must pass or be ACCEPTED at G-ORD-22, and a FAIL
from a later-phase analysis in the cell does not hold the order. Classes 16-18 belong to EVT, DVT and PVT and are shown,
not gated here; 19 and 20 are whole-register classes, listed under the table (subsystem
'register').

Python 3 standard library only.
"""
import csv
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(HERE)))
# 2026-10-28: a check's 'sources' names the datasheet files it relies on, by their name in this directory; a PASS
# on one that is not here rests on a stand-in, so it shows BLOCKED and clears no cell (check_trace: no gate counts it)
DATASHEETS = os.path.join(ROOT, "hardware", "ref_design", "datasheets")
SOURCE_TOKEN = re.compile(r"[A-Za-z0-9][\w.+-]*\.(?:pdf|zip|csv|txt)")

CLASSES = {
    1: "Requirements", 2: "Architecture fit", 3: "Connectivity", 4: "Values and worst case",
    5: "Ratings and derating", 6: "States and sequencing", 7: "Faults and safety", 8: "Schematic vs layout",
    9: "Footprints", 10: "Fab rules", 11: "Assembly data", 12: "Signals and timing", 13: "Fit", 14: "Mating",
    15: "Thermal", 16: "Bring-up measurements", 17: "Stress and life", 18: "Production test",
    19: "Right baseline", 20: "Can the check fail",
}
GROUPS = {1: "A", 2: "A", 3: "B", 4: "B", 5: "B", 6: "B", 7: "B", 8: "C", 9: "C", 10: "C", 11: "C", 12: "C",
          13: "D", 14: "D", 15: "D", 16: "E", 17: "E", 18: "E", 19: "E", 20: "E"}
# 'storage' since 39cc07f: the microSD slot J8, its bus, its supply through JP6 and its opening in the left wall
SUBSYSTEMS = ["power", "cell", "mcu", "display", "audio", "buttons", "usb_debug", "storage", "case"]
REGISTER = "register"                    # the whole register: classes 19 and 20
WHOLE = (19, 20)
ORDER_CLASSES = range(1, 16)             # 1-14, and 15 by a paper estimate
LATER = {16: "EVT", 17: "DVT", 18: "PVT"}
REASON_COLS = ["class", "subsystem", "status", "reason"]
REASON_STATUS = ("NOT_NEEDED", "ACCEPTED_GAP")
# BLOCKED: a PASS on a datasheet not on disk; ACCEPTED: a check an ACCEPTED defect lets through the order gate
WORST = ["FAIL", "BLOCKED", "NOT_BUILT", "NOT_RUN", "ACCEPTED", "PASS"]
BUILT = ("PASS", "FAIL", "NOT_RUN")      # classes 16-18: any built check
RESULTS = ("PASS", "FAIL")               # classes 1-15: a clearing check counts once it has run (O-R6)
PHASES = ["ORDER", "CASE", "EVT", "DVT", "PVT", "SALE"]
LIVE_BOARDS = ("ref_design", "both")


def missing_sources(v, ds=DATASHEETS):
    """The files a check's 'sources' names that are not in hardware/ref_design/datasheets."""
    return [s for s in (v.get("sources") or "").split() if not os.path.isfile(os.path.join(ds, s))]


def shown(v, accepted):
    """The status a check shows in a cell: ACCEPTED when an accepted defect lets it through the order gate and it
    has not passed cleanly; BLOCKED for a PASS on a datasheet not on disk; otherwise its last result."""
    blocked = v["last_result"] == "PASS" and v.get("missing")
    if v["id"] in accepted and (blocked or v["last_result"] != "PASS"):
        return "ACCEPTED"
    return "BLOCKED" if blocked else v["last_result"]


def signed_without_row(reasons, signed):
    """GRID-<class>-<subsystem> lines Alex signed with no ACCEPTED_GAP row in grid_reasons.csv: an accepted gap
    needs both (PLAN.md step 9.5), so these do nothing and are reported."""
    rows = {f"GRID-{r['class']}-{r['subsystem']}" for r in reasons if r["status"] == "ACCEPTED_GAP"}
    return sorted(set(signed) - rows)


def can_clear(v, c):
    """Whether check v can clear a cell of class c (the module docstring): before the order only what runs
    before the boards exist, an ORDER check or an analysis; class 15 only an analysis."""
    analysis = "A" in v["method"].split("/")
    if c == 15:
        return analysis
    if c in ORDER_CLASSES:
        return analysis or v["phase"] == "ORDER"
    return True


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
        if "classes" not in v or "subsystems" not in v or "sources" not in v:
            problems.append((v["_where"], "verification.csv has no 'classes', 'subsystems' and 'sources' columns"))
            break
        v["missing"] = missing_sources(v) if v["last_result"] == "PASS" else []
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


def cells(vers, reasons, signed=frozenset(), accepted=None):
    """{(class, subsystem): cell}. signed: the 'GRID-<class>-<subsystem>' items Alex has signed ACCEPTED.
    accepted: {VER id: why} for the checks an ACCEPTED defect lets through the order gate (check_trace)."""
    accepted = accepted or {}
    live = [v for v in vers if v.get("board") in LIVE_BOARDS and v["last_result"] not in ("CUT", "RETIRED")]
    for v in live:
        v["shown"] = shown(v, accepted)
    why = {(r.get("c"), r["subsystem"]): r for r in reasons}
    out, problems = {}, []
    for c in CLASSES:
        if c in WHOLE:
            continue
        for s in SUBSYSTEMS:
            ids = [v for v in live if c in v["cls"] and s in v["subs"]]
            counts_ = [v for v in ids if can_clear(v, c)]
            # a BLOCKED PASS rests on a stand-in, so it has not run on the part's own numbers: it clears nothing
            built = [v for v in counts_ if v["last_result"] in (RESULTS if c in ORDER_CLASSES else BUILT)
                     and v["shown"] != "BLOCKED"]
            r = why.get((c, s))
            cell = {"checks": ids, "built": built, "reason": r, "c": c}
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
            cell["counts"] = counts_
            cell["planned"] = [v for v in counts_ if v not in built]
            # classes 1-15: the worst among the checks that can clear the cell (a later bench test cannot, and its
            # NOT_BUILT says only that the boards do not exist yet); classes 16-18: among all of them
            judged = counts_ if c in ORDER_CLASSES and counts_ else ids
            cell["worst"] = min((v["shown"] for v in judged), key=WORST.index) if judged else ""
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


def tag(v):
    """A clearing check's id, with its phase when it is not an ORDER check (O-R6), and ACCEPTED or BLOCKED when it
    shows that instead of its result (2026-10-28)."""
    t = v["id"] if v["phase"] == "ORDER" else f"{v['id']}@{v['phase']}"
    return t + (f" {v['shown']}" if v.get("shown") in ("ACCEPTED", "BLOCKED") else "")


def text_cell(cell):
    st = cell["state"]
    # a planned or GAP cell names first the checks that can clear it, then the ones that cannot
    later = " ".join(v["id"] for v in cell["checks"] if v not in cell["counts"])
    later = f"; measures, does not clear: {later}" if later else ""
    if st == "COVERED":
        late = f", earliest {cell['earliest']}" if cell["earliest"] != "ORDER" else ""
        if cell["c"] in ORDER_CLASSES:
            return f"COVERED, worst {cell['worst']}{late}: {' '.join(tag(v) for v in cell['counts'])}{later}"
        return f"COVERED, worst {cell['worst']}{late}: " + " ".join(
            v["id"] + (f" {v['shown']}" if v["shown"] in ("ACCEPTED", "BLOCKED") else "") for v in cell["checks"])
    if st == "planned":
        return f"planned ({cell['earliest']}): {' '.join(tag(v) for v in cell['counts'])}{later}"
    if st == "NOT_NEEDED":
        return "NOT_NEEDED"
    if st == "ACCEPTED_GAP":
        return "ACCEPTED_GAP (signed)"
    if cell["reason"]:
        return "GAP (acceptance not signed)"
    return f"GAP{later}"


def render(vers, reasons, grid, accepted=None, signed=frozenset()):
    accepted = accepted or {}
    live = [v for v in vers if v.get("board") in LIVE_BOARDS and v["last_result"] not in ("CUT", "RETIRED")]
    L = ["# Coverage grid: check classes by subsystem (REF-A, hardware/ref_design)", "",
         "Generated by `python3 hardware/vv/tools/grid.py` from verification.csv (each check's `classes` and "
         "`subsystems`) and grid_reasons.csv. Do not edit by hand: change those files and re-run it. "
         "`check_trace.py --gate order` fails (R9) while any cell of classes 1-15 is GAP or planned. "
         "A cell of classes 1-14 is cleared only by a check that runs before the boards exist and has run (PASS "
         "or FAIL): an ORDER check or an analysis (A), printed with its phase when that is not ORDER "
         "(VER-05@EVT); a NOT_RUN one leaves the cell planned; a later bench test, inspection or demonstration measures it but does not "
         "clear it. Class 15 is cleared only by a paper estimate that has run: an analysis with a PASS or FAIL. "
         "Classes 16-18 belong to EVT, DVT and PVT and are shown, not gated by the order; 19 and 20 are "
         "whole-register classes, listed below. Only checks of board ref_design or both count; CUT and RETIRED "
         "checks never do.", "",
         "A cell reads: COVERED, worst <status> (FAIL, BLOCKED, NOT_BUILT, NOT_RUN, ACCEPTED, PASS) among the checks "
         "that can clear it (classes 16-18: among all its checks) and, when no clearing check is an ORDER check, the "
         "earliest phase, then the later checks that measure it but cannot clear it; planned (the checks that could clear it "
         "once built and run, then the ones that measure it but cannot clear it: a GAP for the order gate); "
         "GAP; NOT_NEEDED (grid_reasons.csv says why); ACCEPTED_GAP (signed: a grid_reasons.csv ACCEPTED_GAP row "
         "and Alex's signed GRID-<class>-<subsystem> line, both). BLOCKED is a PASS that rests on a datasheet not in "
         "hardware/ref_design/datasheets: it clears nothing and no gate counts it. ACCEPTED is a check that a "
         "defect Alex moved to a later gate with a signed DF-ACCEPT line names: the order gate lets it through, "
         "and it is not a PASS.", "",
         "What 'classes 1-15 pass before the order' means: R9 holds a cell until a check that can clear it has run, "
         "and G-ORD-22 holds every ORDER check until it passes or is ACCEPTED with a signed line. A FAIL in a cell "
         "from a check of a later phase (VER-103@EVT, say) does not hold the order: its defect's Gate does.", ""]
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
          "for their own phases. A planned cell names the checks that would clear it once built and run, with what each computes or measures and its pass limit, from verification.csv; a bare "
          "GAP has none yet: add one as a NOT_BUILT row.", ""]
    for c, s, cell in gaps + later:
        held = "" if c in ORDER_CLASSES else f" ({LATER[c]}, not the order gate)"
        if not cell["planned"]:
            L.append(f"- {c} {CLASSES[c]} / {s}{held}: GAP, no check proposed yet")
        for v in cell["planned"]:
            L.append(f"- {c} {CLASSES[c]} / {s}{held}: {v['id']} {v['title']} ({v['method']}, {v['last_result']}, "
                     f"{v['phase']}). Pass: {v['pass_limit']}")
    fails = [v for v in live if v["last_result"] == "FAIL" and v["shown"] != "ACCEPTED"]
    L += ["", f"## FAIL ({len(fails)})", ""]
    for v in fails:
        L.append(f"- {v['id']} {v['title']} ({v['phase']}): classes {v['classes']}; {v['subsystems']}")
    acc = [v for v in live if v["shown"] == "ACCEPTED"]
    L += ["", f"## ACCEPTED at the order gate ({len(acc)}): not a PASS", ""]
    L += [f"- {v['id']} {v['title']} ({v['phase']}, {v['last_result']}): {accepted[v['id']]}" for v in acc] or ["- none"]
    blk = [v for v in live if v["shown"] == "BLOCKED"]
    L += ["", f"## BLOCKED ({len(blk)}): a PASS that rests on a datasheet not in hardware/ref_design/datasheets", ""]
    L += [f"- {v['id']} {v['title']} ({v['phase']}): {' '.join(v['missing'])}" for v in blk] or ["- none"]
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
    orphans = signed_without_row(reasons, signed)
    if orphans:
        L += ["", f"## Signed with no ACCEPTED_GAP row ({len(orphans)}): these cells stay a GAP until grid_reasons.csv "
                  "has the row too", ""] + [f"- {x}" for x in orphans]
    return "\n".join(L) + "\n"


FOOTER = re.compile(r"^Last updated: .*$", re.M)


def same(a, b):
    return FOOTER.sub("", a).rstrip() == FOOTER.sub("", b).rstrip()


def build(vv, signed=frozenset(), accepted=None):
    vers, reasons, problems = read(vv)
    grid, p2 = cells(vers, reasons, signed, accepted)
    return vers, reasons, grid, problems + p2


def main():
    import datetime
    vv = os.path.dirname(HERE)
    if "--vv" in sys.argv:
        vv = os.path.abspath(sys.argv[sys.argv.index("--vv") + 1])
    sys.path.insert(0, HERE)
    import check_trace as ct               # the signed approvals and acceptances, read the one way check_trace reads them
    ct.VV = vv
    db = ct.load()
    signed, accepted = ct.grid_signed(db), ct.accepted_checks(db, "ORDER")
    vers, reasons, grid, problems = build(vv, signed, accepted)
    for w, m in problems:
        print(f"  {w}: {m}")
    if problems:
        print(f"grid: {len(problems)} problem(s) in the register; grid.md not written")
        return 1
    body = render(vers, reasons, grid, accepted, signed)
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
    for x in signed_without_row(reasons, signed):
        print(f"  {x} is signed but grid_reasons.csv has no ACCEPTED_GAP row for it: the cell stays a GAP")
    gaps = order_gaps(grid)
    print(f"{len(gaps)} cells of classes 1-15 the order gate holds "
          f"({sum(1 for *_, c in gaps if c['state'] == 'GAP')} GAP, "
          f"{sum(1 for *_, c in gaps if c['state'] == 'planned')} planned)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
