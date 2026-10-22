#!/usr/bin/env python3
"""The rebuild pipeline (README.md, Rebuilding), in order, skipping what is already up to date.

    python3 tools/build.py                  # every stage whose inputs changed since its last good run
    python3 tools/build.py --force          # every main stage again (end-of-round ones still only if changed)
    python3 tools/build.py --from route     # route (always) and every stage after it (if changed)
    python3 tools/build.py --only check_fit,fitkit   # just these, always
    python3 tools/build.py --list           # the stages, what each runs, and whether it is up to date
    python3 tools/build.py --dry-run        # what a run would do, without running anything
    python3 tools/build.py --touch [--only ...]      # take the files as they are as built (a fresh
                                                     # checkout of a routed board: no 10 min reroute)
    python3 tools/build.py --keep-going     # a failing check (not a generator) doesn't stop the run

A stage is skipped when its key is the one its last good run left in .build/state.json: a hash
of its command, its parameters (passes, RP2350B_REF, the router's jar, which OpenSCAD and
KiCad) and the contents of its input files, and its outputs still exist. A generated input
counts by content, so a case change that leaves positions.json as it was doesn't rebuild or
reroute the board. The board is the one file several stages rewrite in place:
  - build_pcb makes it from nothing, and reads the schematic only through KiCad's netlist, so
    its key is the netlist's: a schematic only redrawn (same parts, same nets) keeps the board.
  - route and silk key on the board as the stage before them left it (its recorded hash), not
    on the board on disk now. route needs that board exactly: the runner keeps a copy of
    build_pcb's (.build/snap/) and puts it back first, so a changed route.py reroutes without
    a rebuild (or rebuilds when the copy is gone). silk can run again on a finished board.
  - the checks, fab and everything after see the board on disk: edit it by hand and they run
    again. Run the hand steps through the runner (--only route_finish, --only sync_pcb): their
    result then counts as the routed board, and silk runs again on it.
End-of-round stages (fitkit, check_manifold, the print kits, viewer) come after every check and run
only when their inputs changed or when named (--only, --from). On request only, never on their own:
sim (about 40 min), sync_pcb, route_finish. A stage whose tool is missing (P1S kit without
BAMBU_STUDIO) is skipped with a note.

Defaults: RP2350B_REF and FREEROUTING_JAR from ~/.cache/ref_design_tools when not set; OpenSCAD
from tools/scad.py (the Manifold snapshot in ~/Applications when there is one); REF_JOBS renders
at once; ROUTE_ATTEMPTS routes at once (route.py). Each stage's output goes to the terminal and
to .build/logs/STAGE.log; the run ends with a table of what ran and how long it took.
"""
import glob
import hashlib
import json
import os
import re
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
REF = os.path.dirname(HERE)                      # hardware/ref_design
STATE_DIR = os.path.join(REF, ".build")
STATE = os.path.join(STATE_DIR, "state.json")
PY = sys.executable
CACHE = os.path.expanduser("~/.cache/ref_design_tools")
DEFAULT_ENV = {
    "RP2350B_REF": os.path.join(CACHE, "rp2350b", "RPI-RP2350B-MINIMAL_R4-S1_public", "RPI-RP2350B-MINIMAL_R4-S1.kicad_pcb"),
    "FREEROUTING_JAR": os.path.join(CACHE, "freerouting-1.9.0.jar"),
}
sys.path.insert(0, HERE)
import scad  # noqa: E402

BOARD_TOOLS = "board/tools/"
SCH = ["board/*.kicad_sch"]
LIB = ["board/lib/**", "board/fp-lib-table", "board/sym-lib-table"]
FAB_OUT = ["board/fab/gerbers/*", "board/fab/battler_gerbers.zip", "board/fab/battler_bom_jlc.csv",
           "board/fab/battler_cpl_jlc.csv", "board/fab/battler.step", "board/fab/battler_top.png",
           "board/fab/battler_bottom.png", "board/fab/battler_iso.png", "board/fab/battler_schematic.pdf",
           "board/fab/battler_assembly.pdf"]
FITKIT_STL = ["case/fitkit/*.stl"]
CASE_STL = ["case/tub.stl", "case/top.stl", "case/deck.stl"]


class Stage:
    """kind: "main" (in order, every run), "end" (end of the round), "manual" (only when named).
    check: a gate with no product (its failure stops the run unless --keep-going).
    tools: what its key also covers: "scad" (which OpenSCAD and backend), "kicad" (kicad-cli's version).
    env: variables it needs, hashed by the file they name. netlist: key on the netlist, not the sheets."""

    def __init__(self, name, cwd, cmd, inputs, outputs=(), kind="main", check=False, tools=(), env=(),
                 netlist=False, needs=None, fresh=(), amends=None, about=""):
        self.name, self.cwd, self.cmd, self.inputs, self.outputs = name, cwd, cmd, list(inputs), list(outputs)
        self.kind, self.check, self.tools, self.env, self.netlist = kind, check, tools, env, netlist
        self.needs, self.fresh, self.amends, self.about = needs, fresh, amends, about


STAGES = [
    Stage("export", ".", [PY, "case/export.py", "--stl"],
          ["case/export.py", "case/case.scad", "tools/scad.py"],
          ["case/positions.json", "case/board.dxf", *CASE_STL], tools=("scad",),
          about="case -> positions.json, board.dxf, tub/top/deck STLs"),
    Stage("check_buttons", ".", [PY, "case/check_buttons.py"],
          ["case/check_buttons.py", "case/check_buttons.scad", "case/case.scad", "tools/scad.py"],
          check=True, tools=("scad",), about="button stacks against the shell"),
    Stage("build_sch", "board", [PY, "tools/build_sch.py"],
          ["board/design.py", *(BOARD_TOOLS + f for f in ("build_sch.py", "schwrite.py", "sch_sheets.py",
                                                         "schdraw.py", "kisym.py")), *LIB],
          SCH, tools=("kicad",), about="design.py -> schematic"),
    Stage("check_netlist", "board", [PY, "tools/check_netlist.py"],
          ["board/design.py", BOARD_TOOLS + "check_netlist.py", BOARD_TOOLS + "kisym.py", *SCH, *LIB],
          check=True, tools=("kicad",), about="KiCad's netlist == design.py"),
    Stage("check_pins", "board", [PY, "tools/check_pins.py"],
          ["board/design.py", BOARD_TOOLS + "check_pins.py", "board/pin_review/*.json"],
          check=True, about="pins against their datasheet reviews"),
    Stage("erc", "board", ["kicad-cli", "sch", "erc", "--severity-error", "--exit-code-violations", "battler.kicad_sch"],
          [*SCH, *LIB, "board/battler.kicad_pro"], check=True, tools=("kicad",), about="0 ERC errors"),
    Stage("build_pcb", "board", [PY, "tools/build_pcb.py"],
          [BOARD_TOOLS + f for f in ("build_pcb.py", "netlist.py", "kisym.py")] +
          ["board/placement.py", "case/positions.json", "board/battler.kicad_pro", *LIB],
          ["board/battler.kicad_pcb"], tools=("kicad",), env=("RP2350B_REF",), netlist=True,
          about="the board from the netlist, placement and the core"),
    Stage("route", "board", [PY, "tools/route.py", "20"],
          [BOARD_TOOLS + f for f in ("route.py", "maze.py", "build_pcb.py", "netlist.py", "kisym.py", "check_pcb.py")] +
          ["board/placement.py", "case/positions.json", "board/battler.kicad_pro", "board/battler.kicad_pcb"],
          ["board/battler.kicad_pcb", "board/tools/last_route.ses"], tools=("kicad",), env=("FREEROUTING_JAR",),
          fresh=("board/battler.kicad_pcb",), about="pours, pre-routes, Freerouting (20 passes, parallel seeded attempts), repairs"),
    Stage("silk", "board", [PY, "tools/silk.py"],
          [BOARD_TOOLS + "silk.py", "board/placement.py", "board/design.py", "case/positions.json",
           "board/battler.kicad_pcb"],
          ["board/battler.kicad_pcb"], tools=("kicad",), about="silkscreen"),
    Stage("check_pcb", "board", [PY, "tools/check_pcb.py"],
          [BOARD_TOOLS + "check_pcb.py", "board/battler.kicad_pcb", "board/battler.kicad_pro", *SCH, *LIB],
          ["board/fab/drc.txt"], check=True, tools=("kicad",), about="DRC with schematic parity"),
    Stage("fab", "board", [PY, "tools/fab.py"],
          [BOARD_TOOLS + "fab.py", "board/battler.kicad_pcb", "board/battler.kicad_pro", *SCH, *LIB],
          FAB_OUT, tools=("kicad",), about="gerbers, BOM, CPL, STEP, renders, PDFs"),
    Stage("check_fit", "board", [PY, "../case/check_fit.py"],
          ["case/check_fit.py", "case/case.scad", "case/positions.json", "board/battler.kicad_pcb", "tools/scad.py", *LIB],
          ["case/fit_report.txt"], check=True, tools=("kicad",), about="board inside the case"),
    Stage("pinmap", "board", [PY, "tools/pinmap.py"],
          [BOARD_TOOLS + "pinmap.py", "board/design.py"], ["docs/pinmap.md"], about="docs/pinmap.md"),
    Stage("fitkit", "board", [PY, "../case/fitkit.py"],
          ["case/fitkit.py", "case/fitkit.scad", "case/case.scad", "case/check_fit.py", "case/positions.json",
           "board/battler.kicad_pcb", "tools/scad.py", *LIB],
          [*FITKIT_STL, "case/fitkit_parts.scad"], kind="end", tools=("scad", "kicad"),
          about="the printable fit kit"),
    Stage("check_manifold", ".", [PY, "case/check_manifold.py"],
          ["case/check_manifold.py", "case/print_package.py", *CASE_STL, *FITKIT_STL], kind="end", check=True,
          about="every printed part's mesh: each edge on two faces"),
    Stage("print_u1", ".", [PY, "case/print_package.py"],
          ["case/print_package.py", "case/overhangs.py", "case/print/fit_test_v5.3mf", "case/print/u1_system/*.json",
           "case/print_package_README.md", "case/print_tests_README.md", *CASE_STL, *FITKIT_STL],
          ["case/print/package_U1/**", "case/print/mega_blastoise_print_U1.zip", "case/print/tests_U1/**",
           "case/print/overhang_kit.zip"], kind="end",
          about="Snapmaker U1 print package"),
    Stage("print_p1s", ".", [PY, "case/print_package_p1s.py"],
          ["case/print_package.py", "case/print_package_p1s.py", "case/overhangs.py", "case/print/p1s/*.json", "case/print/p1s/system/*.json",
           "case/print_package_README.md", "case/print_tests_README.md", *CASE_STL, *FITKIT_STL],
          ["case/print/package_P1S/**", "case/print/mega_blastoise_print_P1S.zip", "case/print/tests_P1S/**",
           "case/print/overhang_kit.zip"], kind="end",
          env=("BAMBU_STUDIO",), needs="BAMBU_STUDIO", about="Bambu Lab P1S print package (sliced)"),
    Stage("viewer", ".", [PY, "viewer/build.py"],
          ["viewer/build.py", "viewer/pieces.scad", "case/case.scad", "board/battler.kicad_pcb",
           "board/design.py", "tools/scad.py", *CASE_STL],
          ["viewer/site/parts/*"], kind="end", tools=("scad", "kicad"), about="the exploded 3D view"),
    Stage("sim", "board", ["sh", "-c", f"{PY} sim/models.py && {PY} sim/backlight.py"],
          ["board/sim/*.py"], ["board/sim/models/*"], kind="manual", about="backlight sink simulation (about 40 min)"),
    Stage("sync_pcb", "board", [PY, "tools/sync_pcb.py"],
          [BOARD_TOOLS + "sync_pcb.py", BOARD_TOOLS + "netlist.py", "board/battler.kicad_pcb", *SCH],
          ["board/battler.kicad_pcb"], kind="manual", tools=("kicad",), amends="route",
          about="relink a routed board to a redrawn schematic"),
    Stage("route_finish", "board", [PY, "tools/route.py", "--finish"],
          [BOARD_TOOLS + "route.py", BOARD_TOOLS + "maze.py", "board/battler.kicad_pcb"],
          ["board/battler.kicad_pcb"], kind="manual", tools=("kicad",), amends="route",
          about="close what route.py left open"),
]
BY_NAME = {s.name: s for s in STAGES}
ORDER = [s for s in STAGES if s.kind == "main"] + [s for s in STAGES if s.kind == "end"]


def expand(patterns):
    """The files a list of paths and globs (relative to ref_design/) names, sorted."""
    out = set()
    for p in patterns:
        for f in glob.glob(os.path.join(REF, p), recursive=True):
            if os.path.isfile(f) and "__pycache__" not in f:
                out.add(os.path.relpath(f, REF))
    return sorted(out)


def file_hash(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


_tool_ids = {}


def tool_id(tool):
    if tool not in _tool_ids:
        if tool == "scad":
            _tool_ids[tool] = scad.describe()
        else:
            r = subprocess.run(["kicad-cli", "--version"], capture_output=True, text=True)
            _tool_ids[tool] = "kicad-cli " + r.stdout.strip()
    return _tool_ids[tool]


def netlist_text():
    """KiCad's netlist of the sheets, without its date: what build_pcb reads of them."""
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        out = os.path.join(tmp, "n.net")
        subprocess.run(["kicad-cli", "sch", "export", "netlist", "--format", "kicadsexpr", "-o", out,
                        os.path.join(REF, "board", "battler.kicad_sch")], capture_output=True, cwd=tmp)
        text = open(out).read() if os.path.exists(out) else "no netlist"
    return re.sub(r'\(date "[^"]*"\)', "", text)


def producer(stage, f, state):
    """The stage before this one in the pipeline that last wrote f (from its record), or None."""
    before = ORDER[:ORDER.index(stage)] if stage in ORDER else []
    return next((s for s in reversed(before) if f in state.get(s.name, {}).get("outputs", {})), None)


def key(stage, state):
    """The stage's key: its command, parameters, tools and the contents of its inputs."""
    h = hashlib.sha256()

    def add(*parts):
        for p in parts:
            h.update(str(p).encode() + b"\0")
    add("v1", stage.name, *stage.cmd, stage.cwd)
    for t in stage.tools:
        add(t, tool_id(t))
    for var in stage.env:
        v = os.environ.get(var, "")
        add(var, v, file_hash(v) if os.path.isfile(v) else "")
    if stage.netlist:
        add("netlist", hashlib.sha256(netlist_text().encode()).hexdigest())
    rewritten = set(expand(stage.outputs))
    for f in expand(stage.inputs):
        # a file this stage rewrites in place: as the stage before it that writes it left it
        p = producer(stage, f, state) if f in rewritten else None
        add(f, state[p.name]["outputs"][f] if p else file_hash(os.path.join(REF, f)))
    for p in stage.inputs:
        if not expand([p]):
            add(p, "missing")
    return h.hexdigest()


def outputs_present(stage):
    return all(expand([p]) for p in stage.outputs)


def load_state():
    try:
        return json.load(open(STATE))
    except (OSError, ValueError):
        return {}


def save_state(state):
    os.makedirs(STATE_DIR, exist_ok=True)
    tmp = STATE + ".tmp"
    json.dump(state, open(tmp, "w"), indent=1, sort_keys=True)
    os.replace(tmp, STATE)


def snap_path(stage, f):
    return os.path.join(STATE_DIR, "snap", stage.name, os.path.basename(f))


def record(stage, state, k, secs, snapshot=True):
    """A good run: its key and what it wrote. A file a later stage needs exactly as this one
    left it (route's board) is copied to .build/snap/."""
    import shutil
    outs = {f: file_hash(os.path.join(REF, f)) for f in expand(stage.outputs)}
    later = ORDER[ORDER.index(stage) + 1:] if stage in ORDER else []
    needed = [f for f in outs if any(f in s.fresh for s in later)]
    if not snapshot:
        # touched, not made: the file on disk isn't this stage's (a routed board isn't the one
        # build_pcb makes), so it never matches, and route makes it again before it routes
        outs.update({f: "touched " + outs[f] for f in needed})
    state[stage.name] = {"key": k, "ok": True, "secs": round(secs, 1), "when": time.strftime("%Y-%m-%d %H:%M:%S"),
                         "outputs": outs}
    for f in needed if snapshot else ():
        os.makedirs(os.path.dirname(snap_path(stage, f)), exist_ok=True)
        shutil.copy(os.path.join(REF, f), snap_path(stage, f))
    if stage.amends and stage.amends in state:       # a hand step on the routed board: now it is the routed board
        for f in outs:
            if f in state[stage.amends].get("outputs", {}):
                state[stage.amends]["outputs"][f] = outs[f]
    save_state(state)


def restore(stage, state, env):
    """Put back each file the stage needs exactly as the stage before it left it (route: the board
    as build_pcb made it), from .build/snap/; with no good copy, run that stage again first.
    Returns False when that fails."""
    import shutil
    for f in stage.fresh:
        p = producer(stage, f, state)
        if p is None:
            continue
        want, path = state[p.name]["outputs"][f], os.path.join(REF, f)
        if os.path.exists(path) and file_hash(path) == want:
            continue
        snap = snap_path(p, f)
        if os.path.exists(snap) and file_hash(snap) == want:
            shutil.copy(snap, path)
            print(f"\n{f}: put back as {p.name} left it (.build/snap/)", flush=True)
            continue
        print(f"\n{f}: no copy as {p.name} left it, so {p.name} runs again first", flush=True)
        t0 = time.time()
        k = key(p, state)
        if run(p, env):
            return False
        record(p, state, k, time.time() - t0)
    return True


def run(stage, env):
    """Run one stage, its output to the terminal and to .build/logs/STAGE.log. Returns the exit code."""
    os.makedirs(os.path.join(STATE_DIR, "logs"), exist_ok=True)
    print(f"\n=== {stage.name}: {' '.join(stage.cmd)}  (in {stage.cwd})", flush=True)
    with open(os.path.join(STATE_DIR, "logs", stage.name + ".log"), "w") as log:
        p = subprocess.Popen(stage.cmd, cwd=os.path.join(REF, stage.cwd), env=env, stdout=subprocess.PIPE,
                             stderr=subprocess.STDOUT, text=True, bufsize=1)
        for line in p.stdout:
            sys.stdout.write("  " + line)
            sys.stdout.flush()
            log.write(line)
        return p.wait()


def parse(argv):
    opts = {"force": False, "only": None, "from": None, "list": False, "dry": False, "touch": False, "keep": False}
    it = iter(argv)
    for a in it:
        name, _, val = a.partition("=")
        if a in ("--force", "-f"):
            opts["force"] = True
        elif name in ("--only", "--from"):
            val = val or next(it, "")
            names = [v for v in val.split(",") if v]
            bad = [n for n in names if n not in BY_NAME]
            if bad or not names:
                sys.exit(f"unknown stage {bad or val!r}: stages are {', '.join(BY_NAME)}")
            opts[name[2:]] = names
        elif a == "--list":
            opts["list"] = True
        elif a in ("--dry-run", "-n"):
            opts["dry"] = True
        elif a == "--touch":
            opts["touch"] = True
        elif a in ("--keep-going", "-k"):
            opts["keep"] = True
        else:
            sys.exit(__doc__)
    return opts


def selection(opts):
    """[(stage, forced)] in pipeline order."""
    if opts["only"]:
        names = set(opts["only"])
        return [(s, True) for s in STAGES if s.name in names]
    if opts["from"]:
        first = BY_NAME[opts["from"][0]]
        if first not in ORDER:
            return [(first, True)]
        return [(s, s is first) for s in ORDER[ORDER.index(first):]]
    return [(s, opts["force"] and s.kind == "main") for s in ORDER]


def main():
    opts = parse(sys.argv[1:])
    env = dict(os.environ)
    for var, path in DEFAULT_ENV.items():
        if not env.get(var) and os.path.exists(path):
            env[var] = path
    os.environ.update({k: v for k, v in env.items() if k in DEFAULT_ENV})
    state = load_state()
    print(f"OpenSCAD: {scad.describe()}, {scad.jobs()} renders at once")
    if opts["list"]:
        for s in STAGES:
            up = "" if s.kind == "manual" else (
                "up to date" if state.get(s.name, {}).get("ok") and state[s.name]["key"] == key(s, state)
                and outputs_present(s) else "out of date")
            print(f"  {s.name:14s} {s.kind:6s} {up:12s} {s.about}")
        return 0
    rows, failed, t_run = [], [], time.time()
    for stage, forced in selection(opts):
        if stage.needs and not env.get(stage.needs):
            rows.append((stage.name, f"skipped: set {stage.needs}", 0.0))
            continue
        k = key(stage, state)
        last = state.get(stage.name, {})
        fresh = last.get("ok") and last.get("key") == k and outputs_present(stage)
        if opts["touch"]:
            if not outputs_present(stage) and stage.outputs:
                rows.append((stage.name, "not touched: outputs missing", 0.0))
                continue
            record(stage, state, k, 0.0, snapshot=False)   # (no snapshot: the board on disk may be routed)
            rows.append((stage.name, "touched", 0.0))
            continue
        if fresh and not forced:
            rows.append((stage.name, "up to date", 0.0))
            continue
        if opts["dry"]:
            rows.append((stage.name, "would run" + (" (forced)" if forced and fresh else ""), 0.0))
            continue
        t0 = time.time()
        if stage.fresh:
            if not restore(stage, state, env):
                rows.append((stage.name, "FAILED: could not restore its input", time.time() - t0))
                failed.append(stage.name)
                break
            k = key(stage, state)
        rc = run(stage, env)
        secs = time.time() - t0
        if rc == 0:
            record(stage, state, k, secs)
            rows.append((stage.name, "ran", secs))
            continue
        state[stage.name] = dict(last, ok=False)
        save_state(state)
        rows.append((stage.name, f"FAILED (exit {rc})", secs))
        failed.append(stage.name)
        if not (stage.check and opts["keep"]):
            break
    sim = BY_NAME["sim"]
    if "sim" in state and state["sim"].get("key") != key(sim, state) and not opts["only"]:
        print("\nnote: the backlight simulation's inputs changed since it last ran: tools/build.py --only sim")
    print(f"\n{'stage':14s} {'result':34s} {'time':>8s}")
    for name, what, secs in rows:
        print(f"{name:14s} {what:34s} {secs:7.1f}s" if secs else f"{name:14s} {what:34s} {'':>8s}")
    print(f"{'total':14s} {'':34s} {time.time() - t_run:7.1f}s")
    if failed:
        print("FAILED: " + ", ".join(failed) + " (logs in .build/logs/)")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
