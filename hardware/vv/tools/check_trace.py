#!/usr/bin/env python3
"""The trace gate for hardware/vv: requirements, DFMEA, checks, defects, approvals and gates.

    python3 hardware/vv/tools/check_trace.py                     # structure + report
    python3 hardware/vv/tools/check_trace.py --gate order        # exit 1 unless the order gate is met
    python3 hardware/vv/tools/check_trace.py --gate case         # the printed-case gate
    python3 hardware/vv/tools/check_trace.py --gate evt
    python3 hardware/vv/tools/check_trace.py --shopping [--gate evt]   # what to buy, longest lead first
    python3 hardware/vv/tools/check_trace.py --verbose           # every ID in every list

Default mode exits 1 only on a structural error. Structure means: IDs well formed, unique
and defined wherever they are referenced; rows and fields well formed; the action priority
agrees with the AIAG-VDA table; a detection rating is no better than its 'detects' checks
allow; an occurrence of 2 or less is backed by built hardware (or, at 1, by a stated
physical exclusion); a PASS or FAIL carries a commit, a date and an evidence file whose
header matches it (every analysis and every test: COMMIT, TOOLS, RESULT); a 'covers'
pattern matches a file committed to git and, with a regex, a line in it; a check marked
'both' covers a board file or a register file; every check is reached by some gate, requirement or
high-priority risk no later than its own phase; every ORDER check is named by a numbered
step row of PLAN.md section (d) (the 'what moved where' table is not a step), so the
checklist and the gate cannot drift apart (round 2's objection, O-R11);
a requirement has a live check no later than its Phase (F's objection: a check moved later
cannot leave a requirement behind); a detection rating of 2 or 1 rests on two or more
'detects' checks; a status of APPROVED, REJECTED, DEFERRED,
ACCEPTED or a signed waiver has a matching line in releases/APPROVALS.md that Alex
committed in a commit signed with the key README.md pins (O-R4 and round 2: never a merge,
never a Claude commit, never another key in the allowed-signers file; DEFERRED, ACCEPTED and
WAIVED name the last phase they hold for; a line 'REQ-ALL@<commit>' approves every
requirement whose statement is unchanged since that commit, 'DF-H@<commit>' accepts every
H row unchanged since it); an approved requirement's
statement has not changed since; two release records of one board with different BOMs have
different BOARD_REVs; a change record's 'What to re-run' lists every bench test tied to a
part its commits changed (O-R21); no defect is CLOSED without a check, and every defect
names its Gate (O-R5); a RETIRED row names the commit that removed its subject, no live row
leans on a retired check, and no live row's text cites a retired id outside 'Was: ...'
(O-R9); no defect's Status keyword is other than OPEN while its Status text says it stays OPEN
or holds a gate (F-R1); an analysis whose procedure reads assembly.md, bom.md, bringup.md or
pinmap.md names that doc in covers, so a doc edit stales it (F-R3); an occurrence of 3 says what it is 'Similar:' to (O-R8); an inspection's evidence
names WHO and ARTEFACT (O-R3); README.md's design baseline names every design path, equals HEAD and equals the
baseline commit's own blobs (DEF-47: a register that drifted off its design fails here, not
only at a gate; F-R3: a baseline rewritten from HEAD after a design edit fails here too);
a JLC_FIXED entry in design_numbers.py counts only when its evidence file is committed and
names the designator (F-R4); no live row of requirements.md, dfmea.csv or verification.csv
names a part, part number or net that design.py at the baseline commit no longer has, outside
'Was: ...' (tools/stale_names.py, VER-188); every repository path a file under hardware/vv names
is in HEAD's tree, or pinned 'path@<commit>' at a commit that has it (tools/repo_paths.py, VER-220,
2026-10-28: main moved or deleted about 180 of them unseen); every live check names the classes and subsystems
it targets, grid_reasons.csv is well formed and no NOT_NEEDED cell has a check (tools/grid.py);
a defect whose Status is ACCEPTED has Alex's signed 'DF-ACCEPT:DEF-nn' line in APPROVALS.md, whose scope
(the gate it moves to) is later than its Gate (2026-10-28: an unsigned ACCEPTED fails); a check's
'sources' are file names in hardware/ref_design/datasheets.
Since 2026-10-28 (the F1C200s re-target) also: a DROPPED row (a requirement, DFMEA row, check, defect or
gate criterion of a dropped board) names that board in its board column and says 'DROPPED with <board>
(tag <tag>): <why>' with the tag that holds the board's register (DROPPED_BOARDS), and the tag exists; a
live row names a live board (rp2350b or both), never a dropped one; no live row leans on a DROPPED row
(verified by, detects, needs, an allocation) or cites one outside 'Was: ...'; every live requirement
names its subsystems ('- Subsystem:', grid.py's SUBSYSTEMS or 'all'); while README's '- Baseline board:'
is a dropped board, no check of the live board is PASS or FAIL (its design files are another board's) unless its
covers name only docs/static, the datasheets or hardware/vv's text (not its scripts) and whose procedure names no
design file (a dev-board test, a datasheet download: F's audit);
gates.md's new-MCU stages NM-01.. are numbered in order, each entry names the stage before it, each
names an exit, and every token is an ID, DOC:<path>::<regex>, APPROVED:REQ-<AREA> or GATE:<phase> with
no gone check among them. The report prints the stage the register is at: the first stage whose exit
is not met.
Everything else is a report, with warnings for evidence not committed, for design commits on
the order branch after the baseline, for architecture proposal commits on it (docs/living/*proposal*.md,
2026-10-28), for a GRID line signed with no ACCEPTED_GAP row and for a grid.md that is not what
the register gives. The report lists every PASS whose 'sources' name a datasheet that is not in
hardware/ref_design/datasheets as BLOCKED: no gate counts it (2026-10-28, a PASS on a stand-in).
The report gives the coverage grid's GAP count: cells of classes 1-15 with no built check that
can clear them (planned ones count as GAP; tools/grid.py says which checks can clear a cell).

--gate PHASE exits 1 unless every criterion of that gate in gates.md is met for the board
(default ref_design), plus these rules at every gate:
  R1  no requirement in scope is still PROPOSED, or DEFERRED past the phase its deferral
      names;
  R2  every requirement in scope lists a check for the board, and every listed check whose
      phase is at or before the gate has PASSED on fresh evidence;
  R3  every H-priority DFMEA row in scope is ACCEPTED for this phase or a later one, or
      DONE with every one of its 'detects' checks PASSED, fresh, and of a phase at or
      before the gate (a row with a later-phase 'detects' check needs an acceptance);
  R4  no defect of the board whose Gate is this phase or earlier is OPEN (order, case, EVT),
      and the FIXED ones are listed beside it (F-R1); every such one is CLOSED (DVT, PVT);
      an ACCEPTED one with Alex's signed DF-ACCEPT line is listed, not failed, at every gate
      before the one it moved to, and counts as an OPEN one would from that gate on (2026-10-28);
  R5  no PASSED check covers a file with uncommitted changes, and nothing under hardware/vv
      is uncommitted; a check whose evidence file is not committed does not pass (O-R2);
  R6  every commit since the newest release record that touches the design (DESIGN_PATHS:
      design.py, placement.py, lib/, the routed board and its project file, the schematics,
      the board/tools tree, case.scad and case/export.py) is named in a change record
      (ref_design);
  R7  the design files at HEAD are the ones README.md's design baseline names (ref_design);
  R8  (order and case gates) the order branch README.md names has no commit after the
      baseline commit that touches the gate's files: the board's at the order gate, the
      case's at the case gate, as R7 (O-R13; 2026-10-26: a case-only commit on the order
      branch does not hold the board order back);
      at the order gate a case-only commit also has to leave the board's positions alone:
      the positions.json regenerated by case/export.py at the order branch's tip and the one
      regenerated at the baseline commit must hash the same (O-R4; both sha256s are printed
      and go into the release record);
      at the order gate a commit on the order branch after the baseline commit that touches
      docs/living/*proposal*.md fails too: an architecture decision lands in a
      proposal before any design path moves (2026-10-28: dcb6a76 chose another MCU unseen).
  R9  (order gate, 2026-10-27) the coverage grid (grid.md, tools/grid.py) has no GAP in classes
      1-15: each subsystem has, for each class, a built check that runs before the boards exist
      (classes 1-14: an ORDER check or an analysis that has run, PASS or FAIL (O-R6: a NOT_RUN
      one leaves the cell planned); class 15: an analysis with a PASS or FAIL), a NOT_NEEDED reason in grid_reasons.csv, or a gap Alex accepted with
      a signed 'GRID-<class>-<subsystem>' line in releases/APPROVALS.md and an ACCEPTED_GAP row in
      grid_reasons.csv (both: a signed line alone is reported and clears nothing); a cell with only
      later bench checks, NOT_BUILT checks or BLOCKED PASSes is planned and still a GAP; grid.md
      must be what the register gives. R9 is coverage, not results: a cell's ORDER checks are
      G-ORD-22's, and a FAIL from a later-phase analysis in a cell does not hold the order.
Two more results at the order gate:
  G1  the gate state Alex froze with a signed 'GATE@<commit>' line in releases/APPROVALS.md
      (every check's phase, every defect's Gate and a defect leaving OPEN (O-R1), every
      criterion's Needs, every requirement's Phase and Verified-by): anything moved later than that commit without a newer signed
      line fails here (O-R5); until one is signed,
  G0  (information, never a failure) every such move since README's 'Gate moves listed since'
      commit is listed, so Alex reads them before he signs GATE@ (round 3's O-S5);
  K1  the pinned signing key is a hardware key (sk-ssh-ed25519@openssh.com or
      sk-ecdsa-sha2-nistp256@openssh.com in the allowed-signers file). A software key fails
      unless the gate is run with --accept-software-key, whose use is printed here and has to
      be copied into the release record's signing section; no line in APPROVALS.md can grant
      the exception (O-R12).
At DVT and PVT a check of phase EVT or later that is a test (method T) or covers a release,
whatever its method, passes only on a result that covers a release record of that phase
(REL-...-DVT, REL-...-PVT): an EVT unit's result does not speak for the revised spin (O-R11).

Scope: requirements and DFMEA rows marked ref_design or both. 'both' is the product level: a
requirement stated without one board's parts, a check of the register itself. A product
requirement split for the board (REQ-X and REQ-X.R) is verified through the .R child. Since
2026-10-28 ref_design is the only board: prod_board and prod_case were deleted from the repository
in 574c94c (Alex: not needed), so --board, the .P allocations and REQ-PBD-* are gone (their rows
are RETIRED with that commit).

At every gate a check passes only on a PASS whose 'sources' are all in hardware/ref_design/datasheets
(else it is BLOCKED), and a gate criterion (G-ORD-22's PHASE:ORDER among them) lets through, as
ACCEPTED, a check that an ACCEPTED defect names in 'Check added or changed', at the gates before the
one the defect moved to. Its notes start 'accepted:' and print even on a PASS. An acceptance does
not verify a requirement (R2) or an H row (R3): those take their own deferral or acceptance.

A check named only by ADVISORY criteria (and by no requirement or H row) is advisory: it is
printed, and PHASE:<p> does not count it. One check can be advisory on its own: a criterion's
Needs cell may name it as 'ADVISORY:VER-nn', which makes that one check advisory at that gate
and binding at the gate it moved to, while the criterion's other checks stay binding. A
criterion whose Needs is RETIRED (its text 'RETIRED
in <commit>: why') keeps its ID and is printed, never counted.

Evidence is fresh when its commit is an ancestor of HEAD, nothing it covers changed since,
and it is younger than the check's valid_days. 'covers' tokens:
  path/glob[::regex]   a file changed (with a regex: a changed line matching \\b(regex)\\b)
  part:R52,R53,C10-C22 a listed part's value, LCSC number, footprint or pins in design.py
  release:REL-...      a bench result on built units: stale only when a change record
                       made after it names the check under 'What to re-run'

Python 3 standard library only.
"""
import argparse
import ast
import base64
import csv
import datetime
import hashlib
import io
import os
import re
import subprocess
import sys
import tempfile
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
VV = os.path.dirname(HERE)              # replaced by --vv (tools/seed_faults.py uses it)
sys.path.insert(0, HERE)
import grid as cgrid                    # noqa: E402  the coverage grid (classes by subsystems)
import stale_names                      # noqa: E402  live rows naming what design.py no longer has (VER-188)
import repo_paths                       # noqa: E402  repository paths the register names that moved or left (VER-220)

PHASES = ["ORDER", "CASE", "EVT", "DVT", "PVT", "SALE"]
GATE_ARG = {"order": "ORDER", "case": "CASE", "evt": "EVT", "dvt": "DVT", "pvt": "PVT"}
GATE_PREFIX = {"ORDER": "G-ORD", "CASE": "G-CASE", "EVT": "G-EVT", "DVT": "G-DVT", "PVT": "G-PVT"}
PREFIX_PHASE = {v: k for k, v in GATE_PREFIX.items()}
# 2026-10-28: prod_board and prod_case left the repository in 574c94c (Alex: not needed). Later the same day the
# ESP32-S3 board and the RP2354B board were both dropped: the register targets the next spin, the Allwinner F1C200s
# board, whose design will replace the files under hardware/ref_design (docs/static/next_spin_requirements.md)
BOARDS = ("rp2350b",)
BOARD_VALUES = ("rp2350b", "both")
# a RETIRED or CUT row keeps the board it was written for, so its history reads as it was (574c94c: prod_board's
# rows; 'ref_design' is REF-A as it stood when the row left, the RP2354B or the ESP32-S3 board)
RETIRED_BOARDS = ("prod_board", "ref_design")
# DROPPED (2026-10-28): a row that belonged to a dropped board. It is neither a PASS nor a CLOSED: it names the
# board and the tag holding that board's register, and no gate of the next spin counts it
DROPPED_BOARDS = {"esp32s3": "vv-esp32s3-final", "rp2354b": "vv-rp2354b-final", "f1c200s": "vv-f1c200s-final"}
GONE = ("RETIRED", "DROPPED")            # out of every gate: the subject left the design, or its board was dropped
BOARD_ROOTS = {"rp2350b": ("hardware/ref_design/",)}
SHARED_ROOTS = ("hardware/vv/",)
METHODS = set("TAID")
LEVELS = ("product", "subsystem", "board")
REQ_STATUS = ("PROPOSED", "APPROVED", "REJECTED", "DEFERRED", "RETIRED", "DROPPED")
VER_RESULT = ("PASS", "FAIL", "NOT_RUN", "NOT_BUILT", "CUT", "RETIRED", "DROPPED")
DF_STATUS = ("OPEN", "DONE", "ACCEPTED", "RETIRED", "DROPPED")
# ACCEPTED (2026-10-28, both sign-off reviewers of vv-rp2354b-final): a defect Alex moves to a later gate with a
# signed 'DF-ACCEPT:DEF-nn' line; before that gate R4 and the gate's criteria let it and the checks it names through
DEF_STATUS = ("OPEN", "FIXED", "CLOSED", "ACCEPTED", "RETIRED", "DROPPED")
# a requirement's allocation to a board: '.B' the RP2350B board's; '.F' was the F1C200s board's and '.R' the ESP32-S3 board's (DROPPED with them)
ALLOC_SUFFIX = {"B": "rp2350b", "F": "f1c200s", "R": "esp32s3"}
VP_STATUS = ("yes", "partly", "no", "simulated")
VP_METHODS = 218                         # validation_plan.md is archived with 218 methods
DESIGN_PY = "hardware/ref_design/board/design.py"
# What the ordered board is made from (O's sign-off objection, DEF-47): the data, the placement, the
# library, the routed copper and the scripts that build, route, silk and export it, and the case.
# A commit touching any of these after a release needs a change record (R6), and README.md's
# design baseline names each one's git object (R7 and the structure check).
# The second review (F-R13, O-R13) added the schematics, the project file (its DRC rules) and the whole
# board/tools tree (one hash: the route, silk and fab scripts and the routed session), and case/export.py,
# which writes the positions placement.py reads.
# Round 3's sign-off (O-S4) and the 71737ae re-seed added what else builds or checks the board: tools/build.py
# (it runs the build_pcb, route, silk and fab stages), the custom DRC rules (battler.kicad_dru, which VER-04's
# DRC reads), the library tables, and placement_zones.py (placement.py imports it).  pin_review/*.json is not a
# design path: VER-02 covers it, so a pin_review change stales VER-02's evidence instead.
# 2026-10-28 (the ESP32-S3 re-seed at 39cc07f): placement_new.json, which placement.py applies over its own poses (the new
# parts' spots from tools/place_new.py), moves parts as placement.py does
BOARD_FILES = ("board/design.py", "board/placement.py", "board/placement_zones.py", "board/placement_new.json", "board/lib",
               "board/battler.kicad_pcb", "board/battler.kicad_pro", "board/battler.kicad_dru", "board/fp-lib-table",
               "board/sym-lib-table", "board/tools", "tools/build.py")
BOARD_GLOBS = ("board/*.kicad_sch",)
CASE_FILES = ("case/case.scad", "case/export.py")
EVIDENCE_DIR = "hardware/vv/evidence/"
CLAUDE_TRAILER = re.compile(r"^(Co-Authored-By: Claude|Claude-Session:)", re.M | re.I)
# O-R12: the key types that need a touch (a hardware key).  A software key is a choice Alex makes explicit.
HARD_KEY_TYPES = ("sk-ssh-ed25519@openssh.com", "sk-ecdsa-sha2-nistp256@openssh.com")

RE = {
    "REQ": r"REQ-[A-Z]{2,4}-\d{2}(?:\.[RFB])?",        # .P (prod_board's allocation) left with it, 574c94c; .B the RP2350B's, .F the F1C200s's
    "DF": r"DF-\d{2,3}",
    "VER": r"VER-\d{2,3}",
    "DEF": r"DEF-\d{2,3}",
    "CHG": r"CHG-\d{2,3}",
    "REL": r"REL-[A-Z0-9]+(?:-[A-Z0-9]+)+",
    "G": r"G-(?:ORD|CASE|EVT|DVT|PVT)-\d{2}",
    "W": r"W-\d{2}",
    "A": r"A-\d{3}",
    "NM": r"NM-\d{2}",                                  # a stage of gates.md's new-MCU path (2026-10-28)
}
ANY_ID = re.compile(r"(?<![\w-])(" + "|".join(RE[k] for k in ("REQ", "DF", "VER", "DEF", "CHG", "G", "W", "NM"))
                    + r"|REL-[A-Z0-9]+(?:-[A-Z0-9]+)+)(?![\w-])")
VER_ID = re.compile(r"(?<![\w-])VER-\d{2,3}(?![\w-])")
REQ_ID = re.compile(r"(?<![\w-])" + RE["REQ"] + r"(?![\w-])")
DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
HASH = re.compile(r"\b[0-9a-f]{7,40}\b")
TODAY = datetime.date.today()

ERRORS, WARNINGS = [], []


def retired_ok(where, rid, text):
    """A RETIRED row names the commit that removed its subject, and that commit exists."""
    hs = [h for h in HASH.findall(text or "") if resolve(h)]
    if "RETIRED" not in (text or "") or not hs:
        err(where, f"{rid}: RETIRED needs 'RETIRED in <commit>: <why>' naming a commit in this repository")


def dropped_ok(where, rid, board, text):
    """A DROPPED row (2026-10-28) belonged to a dropped board: its board column is that board, and its text says
    'DROPPED with <board> (tag <tag>): <why>' naming the tag that holds the board's register, which must exist."""
    tag = DROPPED_BOARDS.get(board)
    if tag is None:
        err(where, f"{rid}: DROPPED, but its board {board!r} is not a dropped board ({', '.join(DROPPED_BOARDS)})")
        return
    if f"DROPPED with {board} (tag {tag})" not in (text or ""):
        err(where, f"{rid}: DROPPED needs 'DROPPED with {board} (tag {tag}): <why>' (the board and the tag that "
                   "holds its register)")
    elif not resolve(f"{tag}^{{commit}}"):
        err(where, f"{rid}: DROPPED names tag {tag}, which is not in this repository")


DROPPED_WAS = re.compile(r"\bit was (?:OPEN|FIXED|CLOSED) there\b")
DF_ID = re.compile(r"(?<![\w-])" + RE["DF"] + r"(?![\w-])")
NO_LESSON = "carries to no DF row: "


def dropped_lesson_ok(d, dfs):
    """A DROPPED defect (README, Rules; F's audit, 2026-10-28) says what it was on its board ('it was OPEN|FIXED|CLOSED
    there') and names a live DF row its lesson carries to, or says 'carries to no DF row: <why>' (a lesson only that
    board's part had, such as its radio's keep-out)."""
    s = d["f"].get("Status", "")
    if not DROPPED_WAS.search(s):
        err(d["_where"], f"{d['id']}: DROPPED needs what it was there ('it was OPEN', 'FIXED' or 'CLOSED there')")
    live = [x for x in DF_ID.findall(WAS.sub("", s)) if x in dfs and dfs[x]["live"]]
    why = s.split(NO_LESSON, 1)[1].strip() if NO_LESSON in s else ""
    if not live and len(why) < 10:
        err(d["_where"], f"{d['id']}: DROPPED needs the live DF row its lesson carries to (or '{NO_LESSON}<why>')")


def board_ok(where, rid, board, status):
    """A live row names a live board; a RETIRED or CUT row may keep the board it was written for; a DROPPED row names
    its dropped board (and a dropped board names only DROPPED, RETIRED or CUT rows)."""
    if status == "DROPPED":
        return                                   # dropped_ok says what is wrong with it
    if board in BOARD_VALUES or (status in ("RETIRED", "CUT") and board in RETIRED_BOARDS + tuple(DROPPED_BOARDS)):
        return
    if board in DROPPED_BOARDS:
        err(where, f"{rid}: board {board!r} was dropped: a live row of it is DROPPED, or it moves to "
                   f"{' or '.join(BOARD_VALUES)}")
    else:
        err(where, f"{rid}: board {board!r} not in {BOARD_VALUES}")


def err(where, msg):
    ERRORS.append(f"{where}: {msg}")


def warn(where, msg):
    WARNINGS.append(f"{where}: {msg}")


# ---- git -------------------------------------------------------------------------
def _toplevel(path):
    return subprocess.run(["git", "-C", path, "rev-parse", "--show-toplevel"],
                          capture_output=True, text=True).stdout.strip()


ROOT = _toplevel(HERE)
if not ROOT:
    sys.exit("check_trace: not inside a git work tree")


def git(*args):
    r = subprocess.run(["git", "-C", ROOT] + list(args), capture_output=True, text=True)
    return r.returncode, r.stdout


# committed or staged files only: a 'covers' glob must not be met by an untracked file (F's objection)
TRACKED = set(git("ls-files", "--cached")[1].split("\n")) - {""}
HEAD = git("rev-parse", "HEAD")[1].strip()
# what HEAD itself holds, and what differs from it (staged or not): evidence counts only when committed (O-R2)
IN_HEAD = set(git("ls-tree", "-r", "--name-only", "HEAD")[1].split("\n")) - {""}
CHANGED_VS_HEAD = set(git("diff", "--name-only", "HEAD")[1].split("\n")) - {""}
BOARD_FILES = BOARD_FILES + tuple(sorted(t[len("hardware/ref_design/"):] for t in TRACKED for g in BOARD_GLOBS
                                         if re.fullmatch(r"hardware/ref_design/" + g.replace(".", r"\.")
                                                         .replace("*", r"[^/]*"), t)))
DESIGN_PATHS = tuple("hardware/ref_design/" + p for p in BOARD_FILES + CASE_FILES)
BASELINE_FOR = {"ORDER": BOARD_FILES, "CASE": CASE_FILES}
SIGNERS = git("config", "--get", "gpg.ssh.allowedSignersFile")[1].strip()


def committed_unchanged(relpath):
    """The file is in HEAD's tree and has no staged or unstaged change."""
    return relpath in IN_HEAD and relpath not in CHANGED_VS_HEAD


def is_merge(sha):
    return len(git("rev-list", "--parents", "-n", "1", sha)[1].split()) > 2


def signature(sha):
    """git's %G? for a commit: G is a good signature from a key in the allowed-signers file."""
    return git("log", "-1", "--format=%G?", sha)[1].strip() or "N"


def signing_key(sha):
    """git's %GF: the fingerprint (SHA256:...) of the key that signed the commit, or ''."""
    return git("log", "-1", "--format=%GF", sha)[1].strip()


def allowed_key_type(fingerprint):
    """The key type the allowed-signers file lists for a fingerprint ('sk-ssh-ed25519@openssh.com' is a
    hardware key that wants a touch), or None when the file does not list it."""
    path = os.path.expanduser(SIGNERS) if SIGNERS else ""
    if not path or not os.path.isfile(path):
        return None
    for line in open(path, encoding="utf-8", errors="replace"):
        toks = line.split()
        for i, t in enumerate(toks[:-1]):
            if re.fullmatch(r"(sk-)?(ssh|ecdsa)-[\w@.-]+", t):
                try:
                    blob = base64.b64decode(toks[i + 1])
                except ValueError:
                    continue
                fp = "SHA256:" + base64.b64encode(hashlib.sha256(blob).digest()).decode().rstrip("=")
                if fp == fingerprint:
                    return t
    return None
_resolved, _diff, _lines, _anc, _msg, _blame, _design = {}, {}, {}, {}, {}, {}, {}


def dirty_files():
    out = git("status", "--porcelain", "--untracked-files=no")[1]
    return {line[3:].split(" -> ")[-1] for line in out.splitlines() if line.strip()}


def resolve(commit):
    if commit not in _resolved:
        rc, out = git("rev-parse", "--verify", "--quiet", commit + "^{commit}")
        _resolved[commit] = out.strip() if rc == 0 else None
    return _resolved[commit]


def is_ancestor(a, b="HEAD"):
    if (a, b) not in _anc:
        _anc[(a, b)] = git("merge-base", "--is-ancestor", a, b)[0] == 0
    return _anc[(a, b)]


def changed_since(commit):
    if commit not in _diff:
        _diff[commit] = set(git("diff", "--name-only", commit, "HEAD")[1].split("\n")) - {""}
    return _diff[commit]


def changed_lines(commit, path):
    key = (commit, path)
    if key not in _lines:
        out = git("diff", "-U0", commit, "HEAD", "--", path)[1]
        _lines[key] = [ln[1:] for ln in out.splitlines()
                       if ln[:1] in "+-" and not ln.startswith("+++") and not ln.startswith("---")]
    return _lines[key]


def commit_message(sha):
    if sha not in _msg:
        _msg[sha] = git("log", "-1", "--format=%B", sha)[1]
    return _msg[sha]


def blame(relpath):
    """{line number: commit sha} for a tracked file; uncommitted lines get 40 zeros."""
    if relpath not in _blame:
        out = {}
        rc, text = git("blame", "--porcelain", "--", relpath)
        if rc == 0:
            sha = None
            for ln in text.splitlines():
                m = re.match(r"^([0-9a-f]{40}) \d+ (\d+)", ln)
                if m:
                    sha, final = m.group(1), int(m.group(2))
                elif ln.startswith("\t") and sha:
                    out[final] = sha
        _blame[relpath] = out
    return _blame[relpath]


def vv_rel(*parts):
    """Repo-relative path of a file in the register directory, or None when --vv points
    outside the repository."""
    p = os.path.relpath(os.path.join(VV, *parts), ROOT)
    return None if p.startswith("..") else p


def glob_re(pat):
    i, out = 0, ""
    while i < len(pat):
        if pat.startswith("**/", i):
            out, i = out + "(?:.*/)?", i + 3
        elif pat.startswith("**", i):
            out, i = out + ".*", i + 2
        elif pat[i] == "*":
            out, i = out + "[^/]*", i + 1
        elif pat[i] == "?":
            out, i = out + "[^/]", i + 1
        else:
            out, i = out + re.escape(pat[i]), i + 1
    return re.compile(out + r"\Z")


def expand_refs(spec):
    out = []
    for tok in spec.split(","):
        tok = tok.strip()
        m = re.fullmatch(r"([A-Z]+)(\d+)-([A-Z]+)?(\d+)", tok)
        if m and m.group(3) in (None, m.group(1)) and int(m.group(2)) <= int(m.group(4)):
            out += [f"{m.group(1)}{i}" for i in range(int(m.group(2)), int(m.group(4)) + 1)]
        elif tok:
            out.append(tok)
    return out


def parse_covers(text):
    """[(kind, spec, original token)]. kind glob: spec (path regex, line regex or None);
    part: a list of refs; release: a REL id. Quotes around a regex are stripped and the
    regex is matched as whole words, so 'R5' does not match R52."""
    out = []
    for tok in text.split():
        tok = tok.strip("\"'")
        if tok.startswith("part:"):
            out.append(("part", expand_refs(tok[5:]), tok))
        elif tok.startswith("release:"):
            out.append(("release", tok[8:], tok))
        else:
            path, _, rx = tok.partition("::")
            rx = rx.strip("\"'")
            try:
                crx = re.compile(r"\b(?:" + rx + r")\b") if rx else None
            except re.error as e:
                err("verification.csv", f"covers {tok!r}: bad regex ({e})")
                crx = None
            out.append(("glob", (glob_re(path), crx), tok))
    return out


def design_parts(commit=None):
    """{ref: part dict} from design.py at a commit (None: the working tree), or None."""
    if commit not in _design:
        src = None
        if commit is None:
            p = os.path.join(ROOT, DESIGN_PY)
            src = open(p, encoding="utf-8").read() if os.path.exists(p) else None
        else:
            rc, out = git("show", f"{commit}:{DESIGN_PY}")
            src = out if rc == 0 else None
        parts = None
        if src:
            ns = {"__name__": "design_snapshot"}
            try:
                exec(compile(src, DESIGN_PY, "exec"), ns)
                parts = {p["ref"]: p for p in ns.get("PARTS", [])}
            except Exception:                      # an old design.py that will not run
                parts = None
        _design[commit] = parts
    return _design[commit]


def part_key(p):
    if p is None:
        return None
    return (p.get("value"), p.get("lcsc"), p.get("fp"), p.get("mpn"), p.get("dnp"), p.get("bom"),
            tuple(sorted((str(k), v or "") for k, v in p.get("pins", {}).items())))


# ---- AIAG-VDA action priority (DFMEA) -----------------------------------------------
def action_priority(s, o, d):
    """The AIAG-VDA FMEA handbook (2019) action-priority table, design FMEA."""
    if s >= 9:
        if o >= 6:
            return "H"
        if o >= 4:
            return "H" if d >= 2 else "M"
        if o >= 2:
            return "H" if d >= 7 else ("M" if d >= 5 else "L")
        return "L"
    if s >= 7:
        if o >= 8:
            return "H"
        if o >= 6:
            return "H" if d >= 2 else "M"
        if o >= 4:
            return "H" if d >= 7 else "M"
        if o >= 2:
            return "M" if d >= 5 else "L"
        return "L"
    if s >= 4:
        if o >= 8:
            return "H" if d >= 5 else "M"
        if o >= 6:
            return "M" if d >= 2 else "L"
        if o >= 4:
            return "M" if d >= 7 else "L"
        return "L"
    if s >= 2:
        if o >= 8:
            return "M" if d >= 5 else "L"
        return "L"
    return "L"


# ---- parsers ---------------------------------------------------------------------
def read_csv(name, columns):
    path = os.path.join(VV, name)
    if not os.path.exists(path):
        err(name, "missing")
        return []
    with open(path, newline="", encoding="utf-8") as f:
        rows = list(csv.reader(f))
    if not rows:
        err(name, "empty")
        return []
    if rows[0] != columns:
        err(name, f"header is {rows[0]}, want {columns}")
        return []
    out = []
    for n, r in enumerate(rows[1:], 2):
        if not any(c.strip() for c in r):
            continue
        if len(r) != len(columns):
            err(f"{name}:{n}", f"{len(r)} fields, want {len(columns)}")
            continue
        d = {k: v.strip() for k, v in zip(columns, r)}
        d["_where"] = f"{name}:{n}"
        out.append(d)
    return out


def parse_blocks(name, prefix_re, fields_required):
    """Markdown blocks: '### ID: title' then '- Field: value' lines."""
    path = os.path.join(VV, name)
    if not os.path.exists(path):
        err(name, "missing")
        return {}
    blocks, cur = {}, None
    head_re = re.compile(r"^### (\S+?):? (.+)$")
    for n, line in enumerate(open(path, encoding="utf-8"), 1):
        line = line.rstrip("\n")
        if line.startswith("#"):
            cur = None
            m = head_re.match(line)
            if m and re.match(prefix_re.split("-")[0] + "-", m.group(1)):
                bid = m.group(1)
                if not re.fullmatch(prefix_re, bid):
                    err(f"{name}:{n}", f"malformed ID {bid!r}")
                    continue
                if bid in blocks:
                    err(f"{name}:{n}", f"duplicate ID {bid}")
                    continue
                cur = blocks[bid] = {"id": bid, "title": m.group(2), "_where": f"{name}:{n}", "f": {}, "lines": {}}
            continue
        if cur is not None:
            m = re.match(r"^- ([A-Za-z][A-Za-z ]*?): (.*)$", line)
            if m:
                cur["f"][m.group(1)] = m.group(2).strip()
                cur["lines"][m.group(1)] = [n]
            elif line.startswith("  ") and cur["f"]:
                last = list(cur["f"])[-1]
                cur["f"][last] += " " + line.strip()
                cur["lines"][last].append(n)
    for b in blocks.values():
        for k in fields_required:
            if not b["f"].get(k):
                err(b["_where"], f"{b['id']}: field '{k}' missing or empty")
    return blocks


def table_rows(path):
    """(line number, cells) for every markdown table row of a file."""
    if not os.path.exists(path):
        return []
    out = []
    for n, line in enumerate(open(path, encoding="utf-8"), 1):
        if line.startswith("|"):
            out.append((n, [c.strip() for c in line.strip().strip("|").split("|")]))
    return out


def parse_gates():
    path = os.path.join(VV, "gates.md")
    crit, waivers = {}, []
    if not os.path.exists(path):
        err("gates.md", "missing")
        return crit, waivers
    for n, cells in table_rows(path):
        if re.match(r"G-", cells[0]):
            if not re.fullmatch(RE["G"], cells[0]):
                err(f"gates.md:{n}", f"malformed criterion ID {cells[0]!r}")
                continue
            if len(cells) < 4:
                err(f"gates.md:{n}", f"{cells[0]}: want | ID | criterion | needs | board |")
                continue
            if cells[0] in crit:
                err(f"gates.md:{n}", f"duplicate ID {cells[0]}")
                continue
            gone = cells[2].split()[0] if cells[2].split() in (["RETIRED"], ["DROPPED"]) else ""
            board_ok(f"gates.md:{n}", cells[0], cells[3], gone)
            phase = PREFIX_PHASE[cells[0].rsplit("-", 1)[0]]
            crit[cells[0]] = {"id": cells[0], "text": cells[1], "needs": cells[2].split(), "board": cells[3],
                              "phase": phase, "advisory": "ADVISORY" in cells[2].split(),
                              "retired": bool(gone), "_where": f"gates.md:{n}"}
            if gone == "RETIRED":
                retired_ok(f"gates.md:{n}", cells[0], cells[1])
            elif gone == "DROPPED":
                dropped_ok(f"gates.md:{n}", cells[0], cells[3], cells[1])
        elif re.match(r"W-", cells[0]):
            if not re.fullmatch(RE["W"], cells[0]) or len(cells) < 3:
                err(f"gates.md:{n}", "waiver row wants | W-NN | G-... | reason |")
                continue
            waivers.append({"id": cells[0], "crit": cells[1], "reason": cells[2], "_where": f"gates.md:{n}"})
        elif re.match(r"NM-", cells[0]):
            # 2026-10-28: the new-MCU path, one row a stage: | NM-nn | stage | entry needs | exit needs | board |
            if not re.fullmatch(RE["NM"], cells[0]) or len(cells) < 5:
                err(f"gates.md:{n}", f"{cells[0]}: a stage row wants | NM-nn | stage | entry | exit | board |")
                continue
            if cells[0] in STAGES:
                err(f"gates.md:{n}", f"duplicate ID {cells[0]}")
                continue
            board_ok(f"gates.md:{n}", cells[0], cells[4], "")
            STAGES[cells[0]] = {"id": cells[0], "name": cells[1], "entry": cells[2].split(), "exit": cells[3].split(),
                                "board": cells[4], "_where": f"gates.md:{n}"}
    return crit, waivers


STAGES = {}                                  # gates.md's new-MCU path, filled by parse_gates (2026-10-28)
STAGE_TOKEN = re.compile(r"(?:DOC:\S+::\S+|APPROVED:REQ-[A-Z]{2,4}|GATE:(?:order|case|evt|dvt|pvt))$")


def check_stages(db):
    """2026-10-28: gates.md's new-MCU path is a numbered list of stages, NM-01 first, each with entry and exit needs.
    A stage's entry names the stage before it; every token is an ID this register defines, 'DOC:<path>::<regex>' (a
    line of a committed file), 'APPROVED:REQ-<AREA>' (every live requirement of that area approved) or
    'GATE:<phase>' (that gate met); a check a stage names is live, not DROPPED, RETIRED or CUT."""
    ids = sorted(STAGES)
    want = [f"NM-{i:02d}" for i in range(1, len(ids) + 1)]
    if ids and ids != want:
        err("gates.md", f"the new-MCU stages are {' '.join(ids)}: number them NM-01 to NM-{len(ids):02d} in order")
    for i, sid in enumerate(ids):
        s = STAGES[sid]
        if not s["exit"]:
            err(s["_where"], f"{sid}: a stage names at least one exit need")
        if i and ids[i - 1] not in s["entry"]:
            err(s["_where"], f"{sid}: its entry needs the stage before it ({ids[i - 1]})")
        for t in s["entry"] + s["exit"]:
            if ANY_ID.fullmatch(t):
                v = db["vers"].get(t)
                if v and v["last_result"] in GONE + ("CUT",):
                    err(s["_where"], f"{sid}: needs {t}, which is {v['last_result']}: name the live check")
            elif not STAGE_TOKEN.fullmatch(t):
                err(s["_where"], f"{sid}: token {t!r} is not an ID, DOC:<path>::<regex>, APPROVED:REQ-<AREA> or "
                                 "GATE:<phase>")
            elif t.startswith("DOC:"):
                path, rx = t[4:].split("::", 1)
                if path not in TRACKED:
                    err(s["_where"], f"{sid}: {t}: {path} is not a committed file")


def stage_need(db, tok, gates_cache):
    """(met, why) for one need of a new-MCU stage."""
    if tok.startswith("DOC:"):
        path, rx = tok[4:].split("::", 1)
        full = os.path.join(ROOT, path)
        text = open(full, encoding="utf-8", errors="replace").read() if os.path.exists(full) else ""
        return bool(re.search(rx, text, re.M)), f"{path} has no line matching {rx!r}"
    if tok.startswith("APPROVED:"):
        area = tok.split(":", 1)[1] + "-"
        live = [k for k, r in db["reqs"].items() if k.startswith(area) and r["f"].get("Status") not in GONE]
        bad = [k for k in live if eff_req_status(db, k) != "APPROVED"]
        return not bad, f"{len(bad)} of {len(live)} {area}* not APPROVED by Alex"
    if tok.startswith("GATE:"):
        ph = GATE_ARG[tok[5:]]
        if ph not in gates_cache:
            with open(os.devnull, "w") as null, __import__("contextlib").redirect_stdout(null):
                gates_cache[ph] = gate(db, ph, BOARDS[0], False)
        return gates_cache[ph], f"the {tok[5:]} gate is not met (check_trace.py --gate {tok[5:]})"
    if tok in STAGES:
        return tok in db.get("stages_met", ()), f"{tok} not exited"
    if tok.startswith("VER-"):
        v = db["vers"].get(tok)
        ok = bool(v) and v["last_result"] == "PASS" and not v["missing"] and not staleness(v, db)
        return ok, f"{tok} {v['last_result'] if v else 'undefined'}"
    if tok.startswith("REQ-"):
        return eff_req_status(db, tok) == "APPROVED", f"{tok} {eff_req_status(db, tok)}"
    if tok.startswith("DEF-"):
        d = db["defs"].get(tok)
        return bool(d) and d["status"] == "CLOSED", f"{tok} {d['status'] if d else 'undefined'}"
    if tok.startswith("DF-"):
        d = db["dfs"].get(tok)
        return bool(d) and d["status"] == "DONE", f"{tok} {d['status'] if d else 'undefined'}"
    return False, f"{tok}: cannot evaluate"


def stage_report(db):
    """Which new-MCU stage the register is at: the first stage whose entry and exit needs are not all met. The stage
    an entry names is order, not a need here, so a later stage met out of order is listed as such; every other entry
    need (VER-233 for the chip gate, F's audit) counts as the exit does. The gate tokens of stages after the first
    open one are not evaluated."""
    db["stages_met"], cache, lines, at = set(), {}, [], None
    for sid in sorted(STAGES):
        s = STAGES[sid]
        entry = [t for t in s["entry"] if t not in STAGES]
        if at and any(t.startswith("GATE:") for t in entry + s["exit"]):
            lines.append(f"    {sid} {s['name']}: not evaluated (an earlier stage is open)")
            continue
        miss = ([f"entry: {why}" for t in entry for ok, why in [stage_need(db, t, cache)] if not ok]
                + [why for t in s["exit"] for ok, why in [stage_need(db, t, cache)] if not ok])
        if miss:
            at = at or sid
            lines.append(f"    {sid} {s['name']}: not met: " + "; ".join(miss[:4]) + (" ..." if len(miss) > 4 else ""))
        else:
            db["stages_met"].add(sid)
            lines.append(f"    {sid} {s['name']}: exit met" + (" (out of order: an earlier stage is open)" if at else ""))
    return at, lines


def parse_records():
    """releases/REL-*.md and releases/CHG-*.md: '# ID' or '# ID: title' as the first heading."""
    d = os.path.join(VV, "releases")
    recs = {}
    if not os.path.isdir(d):
        return recs
    for fn in sorted(os.listdir(d)):
        if not fn.endswith(".md") or "TEMPLATE" in fn or fn == "APPROVALS.md":
            continue
        text = open(os.path.join(d, fn), encoding="utf-8").read()
        m = re.search(r"^# (\S+?)(?::|\s|$)", text, re.M)
        rid = m.group(1) if m else None
        if not rid or not (re.fullmatch(RE["REL"], rid) or re.fullmatch(RE["CHG"], rid)):
            err(f"releases/{fn}", "first heading must be '# REL-...' or '# CHG-NN: title'")
            continue
        if rid in recs:
            err(f"releases/{fn}", f"duplicate ID {rid}")
            continue

        def field(name):
            fm = re.search(r"^- " + re.escape(name) + r":[ \t]*(.*)$", text, re.M)
            return fm.group(1).strip() if fm else ""
        rec = {"id": rid, "text": text, "_where": f"releases/{fn}", "kind": rid.split("-")[0],
               "board": field("Board"), "commit": field("Commit (full hash)"),
               "fab_sha": field("Fab zip sha256"), "bom_sha": field("BOM sha256"), "board_rev": field("BOARD_REV"),
               "case_sha": field("Case positions sha256"),
               "case_base_sha": field("Case positions sha256 at the baseline"),
               "commits": re.findall(r"\b[0-9a-f]{7,40}\b", field("Commit(s)"))}
        sect = re.search(r"^## What to re-run\s*$(.*?)(?=^## |\Z)", text, re.M | re.S)
        rec["reruns"] = set(VER_ID.findall(sect.group(1))) if sect else set()
        recs[rid] = rec
    return recs


def pinned_key():
    """README.md's '- Signing key: `SHA256:...`' line (round 2's objection to O-R4: a good signature only
    proves a key in a user-global allowed-signers file, which any process running as alex can edit, so the
    one key that counts is pinned in the repository). Returns (fingerprint, why it cannot be trusted or '')."""
    path = os.path.join(VV, "README.md")
    if not os.path.exists(path):
        return None, "README.md missing"
    rel = vv_rel("README.md")
    bl = blame(rel) if rel and rel in TRACKED else {}
    for n, line in enumerate(open(path, encoding="utf-8"), 1):
        m = re.match(r"^- Signing key: `(SHA256:[A-Za-z0-9+/]+)`", line)
        if not m:
            continue
        fp, sha = m.group(1), bl.get(n, "0" * 40)
        if set(sha) <= {"0"}:
            return fp, "the pinned key's line in README.md is not committed"
        if CLAUDE_TRAILER.search(commit_message(sha)) or is_merge(sha):
            return fp, f"the pinned key's line comes from {sha[:7]}, a Claude or merge commit"
        if signature(sha) != "G" or signing_key(sha) != fp:
            return fp, f"the pinned key's line ({sha[:7]}) is not in a commit signed with that key"
        return fp, ""
    return None, "README.md pins no signing key ('- Signing key: `SHA256:...`', README 'Approvals')"


def file_at(commit, relpath):
    rc, out = git("show", f"{commit}:{relpath}")
    return out if rc == 0 else None


def statements_at(commit):
    """{requirement id: statement} in requirements.md at a commit (for a 'REQ-ALL@<commit>' line)."""
    rel = vv_rel("requirements.md")
    text = file_at(commit, rel) if rel else None
    out, cur, field = {}, None, None
    for line in (text or "").splitlines():
        m = re.match(r"^### (\S+?):? ", line)
        if line.startswith("#"):
            cur = m.group(1) if m and REQ_ID.fullmatch(m.group(1)) else None
            continue
        m = re.match(r"^- ([A-Za-z][A-Za-z ]*?): (.*)$", line)
        if cur and m:
            field = m.group(1)
            if field == "Statement":
                out[cur] = m.group(2).strip()
        elif cur and field == "Statement" and line.startswith("  "):
            out[cur] += " " + line.strip()
    return out


def hrows_at(commit):
    """{DF id: (failure_mode, effect, severity)} of the H rows in dfmea.csv at a commit (a 'DF-H@' line)."""
    rel = vv_rel("dfmea.csv")
    text = file_at(commit, rel) if rel else None
    out = {}
    if text:
        for r in csv.DictReader(io.StringIO(text)):
            if r.get("action_priority") == "H" and r.get("status") not in GONE:
                out[r["id"]] = (r["failure_mode"], r["effect"], r["severity"])
    return out


BULK = re.compile(r"^(REQ-ALL|DF-H|GATE)@([0-9a-f]{7,40})$")
# 2026-10-28: a defect accepted to a later gate.  Its scope is the gate it moves to; its reason says why.
DF_ACCEPT = re.compile(r"^DF-ACCEPT:(DEF-\d{2,3})$")


def parse_approvals():
    """releases/APPROVALS.md rows: | A-NNN | item | decision | scope | reason | by | date |.
    Only lines Alex committed count: a line whose commit carries a Claude trailer is an error,
    and an uncommitted line is pending; a line counts only from a commit signed with the key
    README.md pins. 'REQ-ALL@<commit>' (APPROVED) and 'DF-H@<commit>' (ACCEPTED) are bulk lines
    (round 2's proportionality objection): they cover every requirement, or every H row, unchanged
    since that commit, and each ID is still checked on its own (an edit after it is not covered).
    'GATE@<commit>' (FROZEN) freezes the gate state: each check's phase, each defect's Gate, each
    criterion's Needs and each requirement's Phase and Verified-by, as they stand at that commit;
    anything moved later needs a newer signed line (O-R5)."""
    path = os.path.join(VV, "releases", "APPROVALS.md")
    rel = vv_rel("releases", "APPROVALS.md")
    bl = blame(rel) if rel and rel in TRACKED else {}
    pin, pin_bad = pinned_key()
    rows, seen, gate_lines = [], set(), []
    for n, cells in table_rows(path):
        if not re.match(r"A-", cells[0]):
            continue
        w = f"releases/APPROVALS.md:{n}"
        if not re.fullmatch(RE["A"], cells[0]) or len(cells) != 7:
            err(w, "approval row wants | A-NNN | item | decision | scope | reason | by | date |")
            continue
        aid, item, dec, scope, reason, who, date = cells
        if aid in seen:
            err(w, f"duplicate ID {aid}")
            continue
        seen.add(aid)
        bulk = BULK.match(item)
        kind = "GATE" if (bulk and bulk.group(1) == "GATE") else ("DFACC" if DF_ACCEPT.match(item)
                                                                 else item.split("-")[0])
        if item.startswith("DF-ACCEPT") and kind != "DFACC":
            err(w, f"{aid}: {item!r}: a defect acceptance is 'DF-ACCEPT:DEF-nn', e.g. DF-ACCEPT:DEF-50")
            continue
        if bulk and not resolve(bulk.group(2)):
            err(w, f"{aid}: {item}: commit {bulk.group(2)} is not in this repository")
            continue
        if bulk:
            want_dec = {"REQ-ALL": "APPROVED", "DF-H": "ACCEPTED", "GATE": "FROZEN"}[bulk.group(1)]
            if dec != want_dec:
                err(w, f"{aid}: {item} takes {want_dec}, not {dec!r}")
        want = {"REQ": ("APPROVED", "REJECTED", "DEFERRED"), "DF": ("ACCEPTED",), "W": ("WAIVED",),
                "GATE": ("FROZEN",), "GRID": ("ACCEPTED",), "DFACC": ("ACCEPTED",)}.get(kind)
        if kind == "GRID" and not re.fullmatch(r"GRID-\d{1,2}-[a-z_]+", item):
            # a coverage-grid gap Alex accepts: 'GRID-<class>-<subsystem>' (grid_reasons.csv proposes it)
            err(w, f"{aid}: {item!r}: a grid acceptance is 'GRID-<class>-<subsystem>', e.g. GRID-15-buttons")
            continue
        if item.startswith("KEY@"):
            # O-R12: the software-key exception is out of band (the gate's --accept-software-key, recorded in the
            # release record), so a line in this file that tries to grant it does not count.
            warn(w, f"{aid}: {item} does not count: no line in this file can grant the signing-key exception "
                    "(O-R12); the gate's --accept-software-key and the release record's signing section do")
            continue
        if not want:
            err(w, f"{aid}: item {item!r} is not a REQ, DF, W, GATE, GRID or DF-ACCEPT:DEF id")
            continue
        if dec not in want:
            err(w, f"{aid}: decision {dec!r} for {item}: want one of {want}")
        if kind == "DFACC" and scope not in PHASES[1:]:
            err(w, f"{aid}: {item} needs a scope: the gate the defect moves to, one of {PHASES[1:]}")
        elif (kind in ("DF", "W", "GATE", "GRID") or dec == "DEFERRED") and scope not in PHASES:
            err(w, f"{aid}: {item} {dec} needs a scope phase (the last gate it holds for): one of {PHASES}")
        if not who or who == "-" or "claude" in who.lower():
            err(w, f"{aid}: 'by' must name the person who decided (only Alex signs)")
        if not DATE.match(date):
            err(w, f"{aid}: date {date!r} is not YYYY-MM-DD")
        if dec in ("REJECTED", "DEFERRED", "ACCEPTED", "WAIVED", "FROZEN") and (not reason or reason == "-"):
            err(w, f"{aid}: {dec} needs a reason")
        sha = bl.get(n, "0" * 40)
        committed, pending = not set(sha) <= {"0"}, ""
        if not committed:
            pending = "not committed"
        elif CLAUDE_TRAILER.search(commit_message(sha)):
            err(w, f"{aid}: written in commit {sha[:7]}, which carries a Claude trailer: only Alex signs approvals")
            committed, pending = False, "a Claude commit"
        elif is_merge(sha):
            # O-R4: a merge (or its conflict resolution) is nobody's signed decision
            err(w, f"{aid}: comes from a merge commit ({sha[:7]}): an approval line counts only from Alex's own "
                   "signed commit; commit the line again on its own")
            committed, pending = False, "a merge commit"
        elif not SIGNERS or not os.path.isfile(os.path.expanduser(SIGNERS)):
            # O-R4 revised: every commit has Alex's e-mail, so authorship proves nothing; only his key does
            warn(w, f"{aid}: pending, unverified: commit signing is not set up here (README, 'Approvals'), so "
                    f"{sha[:7]} cannot be shown to be Alex's")
            committed, pending = False, "unverified: signing not set up"
        elif signature(sha) != "G":
            warn(w, f"{aid}: {sha[:7]} is not signed with a key in the allowed-signers file "
                    f"(%G? = {signature(sha)}): it does not count")
            committed, pending = False, "not signed with Alex's key"
        elif not pin or pin_bad:
            # round 2: 'G' only proves some key in a user-global file; the key that counts is pinned here
            warn(w, f"{aid}: pending: {pin_bad}")
            committed, pending = False, "no trusted pinned key"
        elif signing_key(sha) != pin:
            warn(w, f"{aid}: {sha[:7]} is signed with {signing_key(sha) or 'no key'}, not the pinned key {pin}: "
                    "it does not count")
            committed, pending = False, "signed with another key"
        rows.append({"id": aid, "item": item, "decision": dec, "scope": scope, "reason": reason, "who": who,
                     "date": date, "sha": sha, "committed": committed, "pending": pending, "_where": w,
                     "gate": bool(bulk) and bulk.group(1) == "GATE"})
    if pin and not pin_bad:
        kt = allowed_key_type(pin)
        if kt is None:
            warn("README.md", f"the pinned key {pin} is not in the allowed-signers file: no line can count")
        elif not kt.startswith("sk-"):
            warn("README.md", f"the pinned key is a software key ({kt}): it counts, but any process running as alex "
                              "can sign with it while it sits unlocked in an agent (README 'Approvals')")
    eff = {}
    cur_req, cur_h = None, None
    for r in rows:
        if not r["committed"]:
            continue
        b = BULK.match(r["item"])
        if not b:
            eff[r["item"]] = r                     # a later line supersedes an earlier one
            continue
        if b.group(1) == "GATE":
            gate_lines.append(r)                  # the freeze: a later line replaces an earlier one
        elif b.group(1) == "REQ-ALL":
            if cur_req is None:
                n0 = len(ERRORS)
                cur_req = parse_blocks("requirements.md", RE["REQ"], [])
                del ERRORS[n0:]                    # load() reports the file's own errors once
            then = statements_at(b.group(2))
            for rid, blk in cur_req.items():
                if rid in then and then[rid] == blk["f"].get("Statement", "").strip():
                    eff[rid] = dict(r, item=rid)
        else:
            if cur_h is None:
                n0 = len(ERRORS)
                cur_h = {d["id"]: d for d in read_csv("dfmea.csv", DF_COLS)}
                del ERRORS[n0:]
            then = hrows_at(b.group(2))
            for did, d in cur_h.items():
                if did in then and then[did] == (d["failure_mode"], d["effect"], d["severity"]):
                    eff[did] = dict(r, item=did)
    return rows, eff, gate_lines


def parse_baseline():
    """README.md's design baseline: lines '- `path` <40-hex git blob>', and the order branch line
    '- Order branch: `<branch>` from `<commit>`' (O-R13)."""
    path = os.path.join(VV, "README.md")
    out, branch = {}, None
    if not os.path.exists(path):
        return out, branch
    text = open(path, encoding="utf-8").read()
    m = re.search(r"^## Design baseline\s*$(.*?)(?=^## |\Z)", text, re.M | re.S)
    if m:
        for p, h in re.findall(r"^- `(hardware/\S+)` ([0-9a-f]{40})\s*$", m.group(1), re.M):
            out[p] = h
        b = re.search(r"^- Order branch: `([^`]+)` from `([0-9a-f]{7,40})`", m.group(1), re.M)
        if b:
            branch = (b.group(1), b.group(2))
    return out, branch


BASELINE_BOARD = None                    # README's '- Baseline board: `<board>`', read by load() (2026-10-28)


# What a check of the live board may cover and still run while the baseline is a dropped board's files (F's audit,
# 2026-10-28): Alex's spec, the datasheets and the register's own text. Anything else (a part, a release, any other
# path under hardware/ref_design, and the register's scripts, which read design.py whatever their covers say) is taken
# to read the dropped board's files; so is a procedure that names a design file.
OFF_BOARD_ROOTS = ("docs/static/", "hardware/ref_design/datasheets/", "hardware/vv/")
SCRIPT_ROOT = "hardware/vv/tools/"
DESIGN_WORDS = re.compile(r"design\.py|placement\.py|case\.scad|kicad_|hardware/vv/tools/")


def reads_board_files(covers, procedure=""):
    """Does a check's result rest on the design files at the baseline? Yes unless every 'covers' token is a path under
    OFF_BOARD_ROOTS but not a script (a dev-board test covers next_spin_requirements.md; a datasheet download
    SOURCES.md) and its procedure names no design file and runs no register script outside a 'Was: ...' clause."""
    paths = [t.split("::", 1)[0] for t in covers.split()]
    off = paths and all(p.startswith(OFF_BOARD_ROOTS) and not p.startswith(SCRIPT_ROOT) for p in paths)
    return not off or bool(DESIGN_WORDS.search(WAS.sub("", procedure)))


def parse_baseline_board():
    """The board whose design files the baseline names (README '- Baseline board: `<board>`'). Since 2026-10-28 they
    are the dropped ESP32-S3 board's until an F1C200s design replaces them; a live board's check may not PASS or FAIL
    on them."""
    path = os.path.join(VV, "README.md")
    m = re.search(r"^- Baseline board: `([\w-]+)`", open(path, encoding="utf-8").read(), re.M) \
        if os.path.exists(path) else None
    if not m:
        err("README.md", "the design baseline names no board ('- Baseline board: `<board>`')")
        return None
    if m.group(1) not in BOARDS and m.group(1) not in DROPPED_BOARDS:
        err("README.md", f"baseline board {m.group(1)!r} is neither {' nor '.join(BOARDS)} nor a dropped board")
    return m.group(1)


def branch_drift(db, paths=None):
    """O-R13: design commits on the order branch after the baseline commit, or why it cannot be read.
    A local branch wins over origin's (a scratch clone has only origin's). paths: the design paths that
    count (the default mode: all of them; the order gate the board's, the case gate the case's, as R7)."""
    if not db["order_branch"]:
        return ["README.md names no order branch ('- Order branch: `<branch>` from `<commit>`')"]
    name, base = db["order_branch"]
    ref = next((r for r in (f"refs/heads/{name}", f"refs/remotes/origin/{name}") if resolve(r)), None)
    if not ref:
        return [f"the order branch {name} is not in this repository"]
    if not resolve(base):
        return [f"the baseline commit {base} is not in this repository"]
    shas = git("log", "--format=%h %s", f"{resolve(base)}..{ref}", "--", *(paths or DESIGN_PATHS))[1].splitlines()
    return [f"{name} has design commits after {base}: " + "; ".join(s[:60] for s in shas[:4])
            + (f" (+{len(shas) - 4})" if len(shas) > 4 else "")] if shas else []


# 2026-10-28: an architecture change is decided in a proposal doc before any design path moves.  dcb6a76 chose
# another MCU in docs/living/esp32s3_board_proposal.md and R8, which reads only the design paths, did not see it.
PROPOSALS = ":(glob)docs/living/*proposal*.md"


def proposal_drift(db):
    """Commits on the order branch after the baseline commit that touch docs/living/*proposal*.md:
    a warning in the default mode, a failure of R8 at the order gate."""
    if not db["order_branch"]:
        return []
    name, base = db["order_branch"]
    ref = next((r for r in (f"refs/heads/{name}", f"refs/remotes/origin/{name}") if resolve(r)), None)
    if not ref or not resolve(base):
        return []
    shas = git("log", "--format=%h %s", f"{resolve(base)}..{ref}", "--", PROPOSALS)[1].splitlines()
    return [f"{name} has architecture proposal commits after {base} (docs/living/*proposal*.md): "
            + "; ".join(s[:60] for s in shas[:4]) + (f" (+{len(shas) - 4})" if len(shas) > 4 else "")
            + ": the board being ordered may no longer be the one Alex wants"] if shas else []


# ---- load and validate ------------------------------------------------------------
REQ_FIELDS = ["Level", "Board", "Statement", "Rationale", "Source", "Method", "Verified by", "Phase", "Status"]
DF_COLS = ["id", "board", "item", "function", "failure_mode", "effect", "severity", "cause", "prevention",
           "detection", "detects", "occurrence", "detection_rating", "action_priority", "action", "status"]
VER_COLS = ["id", "title", "method", "board", "procedure", "pass_limit", "covers", "last_result", "evidence",
            "commit", "date", "phase", "valid_days", "instrument", "accuracy", "needs", "classes", "subsystems",
            "sources"]
VP_COLS = ["num", "title", "maps_to", "cut_reason", "status_file", "status_now", "evidence"]
DEF_FIELDS = ["Found", "Symptom", "Root cause", "Why missed", "Check added or changed", "Updated", "Gate", "Status"]
UNITS = {"mv": ("V", 1e-3), "v": ("V", 1.0), "ua": ("A", 1e-6), "ma": ("A", 1e-3), "a": ("A", 1.0),
         "c": ("C", 1.0), "ohm": ("ohm", 1.0), "kohm": ("ohm", 1e3), "mm": ("mm", 1.0)}


def gate_state_at(commit):
    """The register's gate state at a commit: {checks: {id: phase}, defects: {id: gate}, crit: {id: needs},
    reqs: {id: (phase, verified by)}}.  It is what a signed 'GATE@<commit>' line freezes (O-R5)."""
    out = {"checks": {}, "defects": {}, "dstatus": {}, "crit": {}, "reqs": {}}

    def at(*parts):
        rel = vv_rel(*parts)
        return file_at(commit, rel) if rel else None

    text = at("verification.csv")
    if text:
        for r in csv.DictReader(io.StringIO(text)):
            out["checks"][r.get("id", "")] = (r.get("phase", ""), r.get("last_result", ""))
    text = at("defects.md")
    if text:
        cur = None
        for line in text.splitlines():
            m = re.match(r"^### (DEF-\d{2,3}):", line)
            if m:
                cur = m.group(1)
            elif cur and line.startswith("- Gate:"):
                out["defects"][cur] = line.split(":", 1)[1].strip().split()[0]
            elif cur and line.startswith("- Status:"):
                # O-R1: the Status keyword too, so a defect taken off R4 by a status change is a move
                out["dstatus"][cur] = (line.split(":", 1)[1].strip().split() or [""])[0]
    text = at("gates.md")
    if text:
        for line in text.splitlines():
            if not line.startswith("|"):
                continue
            cells = [c.strip() for c in line.strip().strip("|").split("|")]
            if cells and re.fullmatch(RE["G"], cells[0]):
                out["crit"][cells[0]] = set(cells[2].split()) if len(cells) > 2 else set()
    text = at("requirements.md")
    if text:
        cur, fields = None, {}
        for line in text.splitlines():
            m = re.match(r"^### (REQ-[A-Z]{2,4}-\d{2}(?:\.[RPF])?):", line)
            if m:
                if cur:
                    out["reqs"][cur] = (fields.get("Phase", ""), fields.get("Verified by", ""))
                cur, fields = m.group(1), {}
                continue
            if cur:
                f = re.match(r"^- ([A-Za-z][A-Za-z ]*?): (.*)$", line)
                if f:
                    fields[f.group(1)] = f.group(2).strip()
        if cur:
            out["reqs"][cur] = (fields.get("Phase", ""), fields.get("Verified by", ""))
    return out


def gate_freeze_violations(db, phase=None):
    """O-R5: the gate state Alex froze with a signed 'GATE@<commit>' line.  Anything moved later since that
    commit (a check's phase, a defect's Gate, a criterion's Needs, a requirement's Phase or Verified-by)
    needs a newer signed line.  With no signed line the rule is inert.  Returns (lines, the line)."""
    # a freeze holds up to its scope phase, like an acceptance: past it, the phases may move again
    lines = [a for a in db["approvals"] if a.get("gate") and a["committed"]
             and (phase is None or (a["scope"] in PHASES and phase_le(phase, a["scope"])))]
    if not lines:
        return [], None
    a = lines[-1]                                     # a later line replaces an earlier one
    m = BULK.match(a["item"])
    if not m or not resolve(m.group(2)):
        return [], a
    return state_moves(db, gate_state_at(m.group(2))), a


def unfrozen_moves(db):
    """Round 3's sign-off (O-S5): until Alex signs a 'GATE@<commit>' line, the gate prints every move later (a
    check's phase, a defect's Gate, a criterion's Needs, a requirement's Phase or Verified-by) since the register
    state README names on its '- Gate moves listed since: `<commit>`' line (the last state two reviewers read),
    so he sees the list before he freezes it.  Returns (lines, the commit) or ([], None)."""
    path = os.path.join(VV, "README.md")
    m = re.search(r"^- Gate moves listed since: `([0-9a-f]{7,40})`", open(path, encoding="utf-8").read(), re.M) \
        if os.path.exists(path) else None
    if not m or not resolve(m.group(1)):
        return [], None
    return state_moves(db, gate_state_at(m.group(1))), m.group(1)


def state_moves(db, then):
    """What moved later since a gate state (gate_state_at): the lines G1 and G0 print."""
    bad = []
    for vid, (was, res) in then["checks"].items():
        v = db["vers"].get(vid)
        if v is None:
            bad.append(f"{vid} was deleted")
            continue
        if was in PHASES and v["phase"] in PHASES and PHASES.index(v["phase"]) > PHASES.index(was):
            bad.append(f"{vid} moved from {was} to {v['phase']}")
        if v["last_result"] in ("CUT",) + GONE and res not in ("CUT",) + GONE:
            bad.append(f"{vid} became {v['last_result']}")
    for did, d in db["defs"].items():
        was = then["defects"].get(did)
        if was in PHASES and d["gate"] in PHASES and PHASES.index(d["gate"]) > PHASES.index(was):
            bad.append(f"{did} moved from Gate {was} to Gate {d['gate']}")
        # O-R1: OPEN to FIXED, CLOSED or RETIRED takes a defect off R4 at ORDER, CASE and EVT, the same as a
        # Gate move (FIXED to CLOSED needs its check to pass on fresh evidence, so it is not listed)
        # an ACCEPTED defect with Alex's signed DF-ACCEPT line is his own signed move, not a quiet one
        if then.get("dstatus", {}).get(did) == "OPEN" and d["status"] != "OPEN" \
                and not (d["status"] == "ACCEPTED" and def_accept(db, did)):
            bad.append(f"{did} went from OPEN to {d['status']}")
    for cid, was in then["crit"].items():
        c = db["crit"].get(cid)
        if c is None:
            bad.append(f"{cid} was deleted")
            continue
        now = set(c["needs"])
        if not was <= now:
            bad.append(f"{cid} no longer needs {', '.join(sorted(was - now))}")
        # a criterion made advisory, retired, or one of its checks made advisory: each weakens what the gate counts
        for tok in sorted((now - was) & ({"ADVISORY", "RETIRED", "DROPPED"} | {t for t in now if t.startswith("ADVISORY:")})):
            bad.append(f"{cid} gained {tok}")
    for rid, (p_then, vb_then) in then["reqs"].items():
        r = db["reqs"].get(rid)
        if r is None:
            bad.append(f"{rid} was deleted")
            continue
        p_now, vb_now = r["f"].get("Phase", ""), r["f"].get("Verified by", "")
        if p_then in PHASES and p_now in PHASES and PHASES.index(p_now) > PHASES.index(p_then):
            bad.append(f"{rid} moved from Phase {p_then} to {p_now}")
        dropped = set(VER_ID.findall(vb_then)) - set(VER_ID.findall(vb_now))
        if dropped:
            bad.append(f"{rid} no longer lists {' '.join(sorted(dropped))} in Verified by")
    return bad


def case_positions_hash(commit):
    """The sha256 of the positions.json that case/export.py writes at a commit, or '' when it cannot be built
    here (OpenSCAD missing, or the commit is not in this repository)."""
    if not resolve(commit):
        return ""
    with tempfile.TemporaryDirectory() as tmp:
        tar = os.path.join(tmp, "tree.tar")
        # export.py needs only the case and the shared tools (scad.py: which OpenSCAD to run)
        if git("archive", "-o", tar, commit, "hardware/ref_design/case", "hardware/ref_design/tools")[0] != 0:
            return ""
        if subprocess.run(["tar", "-xf", tar, "-C", tmp], capture_output=True).returncode != 0:
            return ""
        case = os.path.join(tmp, "hardware", "ref_design", "case")
        if not os.path.isfile(os.path.join(case, "export.py")):
            return ""
        try:
            r = subprocess.run([sys.executable, os.path.join(case, "export.py")], cwd=case,
                               capture_output=True, text=True, timeout=600)
        except (OSError, subprocess.TimeoutExpired):
            return ""
        if r.returncode != 0:
            return ""
        p = os.path.join(case, "positions.json")
        if not os.path.isfile(p):
            return ""
        return hashlib.sha256(open(p, "rb").read()).hexdigest()


def case_position_drift(db):
    """O-R4: a case-only commit on the order branch may not move the board's positions: the positions.json
    regenerated at the branch's tip must be the same as the one regenerated at the baseline commit.
    Returns (the lines that fail the order gate, the lines it prints either way: both sha256s)."""
    if not db["order_branch"]:
        return [], []
    name, base = db["order_branch"]
    ref = next((r for r in (f"refs/heads/{name}", f"refs/remotes/origin/{name}") if resolve(r)), None)
    if not ref or not resolve(base) or resolve(ref) == resolve(base):
        return [], []
    case_commits = git("log", "--format=%h", f"{resolve(base)}..{ref}", "--",
                       *(f"hardware/ref_design/{p}" for p in CASE_FILES))[1].split()
    if not case_commits:
        return [], []
    tip = resolve(ref)[:7]
    now, then = case_positions_hash(ref), case_positions_hash(base)
    if not now or not then:
        return [f"the case positions at {tip} could not be rebuilt here (OpenSCAD or the commit is missing): "
                f"the order gate cannot show that the case commits {' '.join(case_commits)} did not move the board"], []
    if now != then:
        return [f"case commit moves board positions: positions.json sha256 {now} at {tip}, {then} at "
                f"{base[:7]} (case commits {' '.join(case_commits)})"], []
    return [], [f"case positions: positions.json sha256 {now} at {tip} and at {base[:7]}: the case commits "
                f"{' '.join(case_commits)} move no board position (copy both into the release record)"]


def check_jlc_fixed():
    """F-R4: a JLC_FIXED entry in design_numbers.py counts only when its evidence file is committed and names
    the designator: otherwise a line of text could silence the CPL check."""
    path = os.path.join(VV, "tools", "design_numbers.py")
    if not os.path.exists(path):
        return
    try:
        tree = ast.parse(open(path, encoding="utf-8").read())
    except SyntaxError as e:
        err("tools/design_numbers.py", f"it does not parse: {e}")
        return
    fixed = None
    for node in tree.body:
        if isinstance(node, ast.Assign) and any(getattr(t, "id", None) == "JLC_FIXED" for t in node.targets):
            try:
                fixed = ast.literal_eval(node.value)
            except ValueError:
                err("tools/design_numbers.py", "JLC_FIXED is not a literal dict")
                return
    if not fixed:
        return
    for ref, e in fixed.items():
        ev = (e or {}).get("evidence", "")
        if not ev:
            err("tools/design_numbers.py", f"JLC_FIXED[{ref}]: the entry names no evidence file")
        elif not os.path.isfile(os.path.join(ROOT, ev)):
            err("tools/design_numbers.py", f"JLC_FIXED[{ref}]: the evidence file {ev} does not exist")
        elif git("ls-files", "--error-unmatch", ev)[0] != 0:
            err("tools/design_numbers.py", f"JLC_FIXED[{ref}]: the evidence file {ev} is not committed")
        elif ref not in open(os.path.join(ROOT, ev), encoding="utf-8", errors="replace").read():
            err("tools/design_numbers.py", f"JLC_FIXED[{ref}]: the evidence file {ev} does not name {ref}")


def in_board(path, board):
    return any(path.startswith(r) for r in BOARD_ROOTS[board])


def load():
    global BASELINE_BOARD
    BASELINE_BOARD = parse_baseline_board()
    reqs = parse_blocks("requirements.md", RE["REQ"], REQ_FIELDS)
    for r in reqs.values():
        f, w = r["f"], r["_where"]
        if f.get("Level") and f["Level"] not in LEVELS:
            err(w, f"{r['id']}: Level {f['Level']!r} not in {LEVELS}")
        if f.get("Board"):
            board_ok(w, r["id"], f["Board"], f.get("Status", ""))
        if f.get("Status") == "DROPPED":
            dropped_ok(w, r["id"], f.get("Board", ""), f.get("Dropped", ""))
        elif f.get("Status") != "RETIRED":
            # 2026-10-28: every live requirement is allocated to the subsystems of grid.md it falls in
            subs = f.get("Subsystem", "").split()
            if not subs:
                err(w, f"{r['id']}: a live requirement names its subsystems ('- Subsystem: ...', of "
                       f"{' '.join(cgrid.SUBSYSTEMS)}, or 'all')")
            for s in subs:
                if s not in cgrid.SUBSYSTEMS + ["all"]:
                    err(w, f"{r['id']}: subsystem {s!r} is not one of {' '.join(cgrid.SUBSYSTEMS)} (or 'all')")
        meth = set(re.split(r"[,\s/]+", f.get("Method", ""))) - {""}
        if not meth or not meth <= METHODS:
            err(w, f"{r['id']}: Method {f.get('Method')!r}: use T, A, I, D")
        if f.get("Phase") and f["Phase"] not in PHASES:
            err(w, f"{r['id']}: Phase {f['Phase']!r} not in {PHASES}")
        if f.get("Status") and f["Status"] not in REQ_STATUS:
            err(w, f"{r['id']}: Status {f['Status']!r} not in {REQ_STATUS}")
        if f.get("Status") == "RETIRED":
            retired_ok(w, r["id"], "RETIRED " + f.get("Retired", ""))
        vb = WAS.sub("", f.get("Verified by", ""))      # a 'Was: ...' clause is history, not a check or a child
        r["vers"] = VER_ID.findall(vb)
        r["children"] = [c for c in REQ_ID.findall(vb) if c.startswith(r["id"] + ".")]
        if vb and vb.lower() != "none" and not r["vers"] and not r["children"]:
            err(w, f"{r['id']}: 'Verified by' names no VER id and no allocation (write 'none' if there is none yet)")
        if "." in r["id"]:
            parent, suffix = r["id"].rsplit(".", 1)
            if parent not in reqs:
                err(w, f"{r['id']}: allocation of {parent}, which does not exist")
            if f.get("Board") != ALLOC_SUFFIX.get(suffix):
                err(w, f"{r['id']}: a .{suffix} allocation is Board: {ALLOC_SUFFIX.get(suffix)}")
    for r in reqs.values():
        r["alloc"] = {reqs[c]["f"].get("Board"): c for c in reqs if c.startswith(r["id"] + ".")}
        for c in r["children"]:
            if c not in reqs:
                err(r["_where"], f"{r['id']}: allocated to {c}, which does not exist")

    vers = {}
    for v in read_csv("verification.csv", VER_COLS):
        w = v["_where"]
        if not re.fullmatch(RE["VER"], v["id"]):
            err(w, f"malformed ID {v['id']!r}")
            continue
        if v["id"] in vers:
            err(w, f"duplicate ID {v['id']}")
            continue
        meth = set(v["method"].split("/")) - {""}
        if not meth or not meth <= METHODS:
            err(w, f"{v['id']}: method {v['method']!r}: use T, A, I, D or e.g. I/D")
        board_ok(w, v["id"], v["board"], v["last_result"])
        if v["last_result"] == "DROPPED":
            dropped_ok(w, v["id"], v["board"], v["evidence"])
        elif (v["last_result"] in ("PASS", "FAIL") and v["board"] in BOARDS and BASELINE_BOARD in DROPPED_BOARDS
              and reads_board_files(v["covers"], v["procedure"])):
            # 2026-10-28: the design files at the baseline are a dropped board's, so no result on them speaks for
            # the next spin's board: its checks wait, NOT_RUN or NOT_BUILT, until its own design is the baseline.
            # A check that reads none of them (a dev-board test, a datasheet download) may run (F's audit: the chip
            # gate NM-03 could not exit before the F1C200s design.py, which lands at NM-05)
            err(w, f"{v['id']}: {v['last_result']} for {v['board']}, but the design baseline is the dropped "
                   f"{BASELINE_BOARD} board's files (README '- Baseline board:'): no {v['board']} design exists to "
                   "check yet")
        if v["phase"] not in PHASES:
            err(w, f"{v['id']}: phase {v['phase']!r}")
        if v["last_result"] not in VER_RESULT:
            err(w, f"{v['id']}: last_result {v['last_result']!r} not in {VER_RESULT}")
        for k in ("title", "procedure", "pass_limit"):
            if not v[k]:
                err(w, f"{v['id']}: {k} is empty")
        if v["last_result"] in ("PASS", "FAIL"):
            for k in ("evidence", "commit", "date", "covers"):
                if not v[k]:
                    err(w, f"{v['id']}: {v['last_result']} needs {k}")
            if v["date"] and not DATE.match(v["date"]):
                err(w, f"{v['id']}: date {v['date']!r} is not YYYY-MM-DD")
            if v["commit"] and not resolve(v["commit"]):
                err(w, f"{v['id']}: commit {v['commit']!r} is not in this repository")
        elif v["commit"] or v["date"]:
            err(w, f"{v['id']}: {v['last_result']} must not carry a commit or date")
        if v["last_result"] == "CUT" and not v["evidence"]:
            err(w, f"{v['id']}: CUT needs its reason in the evidence column")
        if v["last_result"] == "RETIRED":
            retired_ok(w, v["id"], v["evidence"])
        if v["valid_days"] and not re.fullmatch(r"\d+", v["valid_days"]):
            err(w, f"{v['id']}: valid_days {v['valid_days']!r} is not a whole number of days")
        if v["needs"] and not re.search(r"\b\d+d\b", v["needs"]):
            warn(w, f"{v['id']}: 'needs' has no lead time token like '10d'")
        if v["needs"] and v["last_result"] == "PASS":
            warn(w, f"{v['id']}: PASS but 'needs' still lists {v['needs'][:40]!r}: clear it")
        # 2026-10-28: 'sources' names the datasheet files the check relies on, by their name in
        # hardware/ref_design/datasheets; a PASS on one that is not there is BLOCKED (gate(), grid.py)
        for tok in v["sources"].split():
            if not cgrid.SOURCE_TOKEN.fullmatch(tok):
                err(w, f"{v['id']}: sources token {tok!r} is not a file name in hardware/ref_design/datasheets "
                       "(e.g. bq24074.pdf)")
        v["missing"] = cgrid.missing_sources(v) if v["last_result"] == "PASS" else []
        v["cov"] = parse_covers(v["covers"])
        v["release"] = [s for k, s, _ in v["cov"] if k == "release"]
        if v["last_result"] in GONE:               # its subject or its board is gone: nothing to cover or prove
            vers[v["id"]] = v
            continue
        # F-R3: an analysis that reads a board doc names it in covers, so a doc edit stales it (a doc is not a
        # design path: it needs no change record, only the re-run); 'for pinmap.md' or 'into' it is a proposal
        if "A" in v["method"].split("/") and v["last_result"] not in ("CUT",):
            for doc in sorted(set(BOARD_DOC.findall(v["procedure"]))):
                if not re.search(rf"(?:^|\s)\S*docs/(?:\*|{re.escape(doc)})\.md(?:::|\s|$)", v["covers"]):
                    err(w, f"{v['id']}: its procedure reads {doc}.md, but covers does not name "
                           f"hardware/ref_design/docs/{doc}.md (F-R3)")
        for kind, spec, tok in v["cov"]:
            if kind == "glob":
                g, rx = spec
                files = [t for t in TRACKED if g.match(t)]
                if not files:
                    err(w, f"{v['id']}: covers {tok!r} matches no tracked file")
                elif rx is not None and not any(
                        rx.search(ln) for f_ in files if os.path.exists(os.path.join(ROOT, f_))
                        for ln in open(os.path.join(ROOT, f_), encoding="utf-8", errors="replace")):
                    err(w, f"{v['id']}: covers {tok!r}: the regex matches no current line of its file")
            elif kind == "part":
                cur = design_parts(None) or {}
                missing = [r for r in spec if r not in cur]
                if missing:
                    err(w, f"{v['id']}: covers {tok!r}: {', '.join(missing)} not in design.py")
            elif kind == "release":
                if not re.fullmatch(RE["REL"], spec):
                    err(w, f"{v['id']}: covers {tok!r}: not a REL id")
        if v["board"] == "both":
            paths = [t for k, s, _ in v["cov"] if k == "glob" for t in TRACKED if s[0].match(t)]
            shared = any(p.startswith(SHARED_ROOTS) for p in paths)
            if not shared and not all(any(in_board(p, b) for p in paths) for b in BOARDS):
                err(w, f"{v['id']}: a 'both' check must cover each board's files (or a shared file under "
                       f"{' or '.join(SHARED_ROOTS)})")
        for tok in v["evidence"].split():
            if re.match(r"^hardware/\S+\.\w+$", tok) and not os.path.exists(os.path.join(ROOT, tok)):
                err(w, f"{v['id']}: evidence file {tok} does not exist")
        tur_check(v)
        vers[v["id"]] = v

    for r in reqs.values():
        if r["f"].get("Status") in GONE + ("REJECTED",):
            continue
        for t in r["vers"]:
            if t in vers and vers[t]["last_result"] in GONE:
                err(r["_where"], f"{r['id']}: verified by {t}, which is {vers[t]['last_result']}: name the check that "
                                 "replaced it")
        for c in r["children"]:
            if c in reqs and reqs[c]["f"].get("Status") in GONE:
                err(r["_where"], f"{r['id']}: allocated to {c}, which is {reqs[c]['f']['Status']}: name the live "
                                 "allocation (keep the old one inside 'Was: ...')")

    dfs = {}
    for d in read_csv("dfmea.csv", DF_COLS):
        w = d["_where"]
        if not re.fullmatch(RE["DF"], d["id"]):
            err(w, f"malformed ID {d['id']!r}")
            continue
        if d["id"] in dfs:
            err(w, f"duplicate ID {d['id']}")
            continue
        board_ok(w, d["id"], d["board"], d["status"])
        try:
            s, o, dt = int(d["severity"]), int(d["occurrence"]), int(d["detection_rating"])
            if not all(1 <= x <= 10 for x in (s, o, dt)):
                raise ValueError
        except ValueError:
            err(w, f"{d['id']}: severity, occurrence and detection_rating must be integers 1-10")
            continue
        d["S"], d["O"], d["D"] = s, o, dt
        want = action_priority(s, o, dt)
        if d["action_priority"] not in ("H", "M", "L"):
            err(w, f"{d['id']}: action_priority {d['action_priority']!r}")
        elif d["action_priority"] != want:
            err(w, f"{d['id']}: action_priority {d['action_priority']} but S{s} O{o} D{dt} gives {want} (AIAG-VDA table)")
        if d["status"] not in DF_STATUS:
            err(w, f"{d['id']}: status {d['status']!r} not in {DF_STATUS}")
        for k in ("item", "function", "failure_mode", "effect", "cause"):
            if not d[k]:
                err(w, f"{d['id']}: {k} is empty")
        d["vers"] = VER_ID.findall(d["detection"])
        d["det"] = d["detects"].split()
        d["live"] = d["status"] not in GONE
        if d["status"] == "RETIRED":
            retired_ok(w, d["id"], d["action"])
        elif d["status"] == "DROPPED":
            dropped_ok(w, d["id"], d["board"], d["action"])
        for t in d["det"]:
            if not VER_ID.fullmatch(t):
                err(w, f"{d['id']}: detects token {t!r} is not a VER id")
            elif t not in d["vers"]:
                err(w, f"{d['id']}: detects {t}, which the detection text does not describe")
        dfs[d["id"]] = d

    # DFMEA against the checks (O-2, O-26)
    for d in dfs.values():
        if not d["live"]:
            continue
        for t in d["det"]:
            if t in vers and vers[t]["last_result"] in GONE:
                err(d["_where"], f"{d['id']}: detects {t}, which is {vers[t]['last_result']}: name the check that "
                                 "replaced it")
        w, live = d["_where"], [t for t in d["det"] if t in vers and vers[t]["last_result"] not in ("CUT",) + GONE]
        if not live and d["D"] != 10:
            err(w, f"{d['id']}: no live 'detects' check, so detection_rating must be 10 (it is {d['D']})")
        elif live and all(vers[t]["last_result"] == "NOT_BUILT" for t in live) and d["D"] < 8:
            err(w, f"{d['id']}: every 'detects' check is NOT_BUILT, so detection_rating must be 8 or more (it is {d['D']})")
        if d["status"] == "DONE" and not live:
            err(w, f"{d['id']}: DONE needs at least one 'detects' check")
        if d["D"] <= 2 and len(live) < 2:
            # F's objection: the README's 2 is 'two independent checks', 1 'cannot pass unseen'
            err(w, f"{d['id']}: detection_rating {d['D']} needs two or more independent 'detects' checks "
                   f"(it has {len(live)}): rate it 3-4 for one script that can fail")
        if d["O"] <= 2:
            built = any("T" in vers[t]["method"] and vers[t]["release"] and vers[t]["last_result"] == "PASS"
                        for t in live)
            excluded = d["O"] == 1 and "Excluded:" in d["prevention"]
            if not built and not excluded:
                err(w, f"{d['id']}: occurrence {d['O']} needs a passing T check on built hardware in 'detects', "
                       f"or occurrence 1 with 'Excluded: <why the cause cannot happen>' in prevention")
        if d["O"] == 3 and "Similar:" not in d["prevention"]:
            # O-R8: 3 is 'proven on similar hardware'; a part never built rates 4 or more by the scale
            err(w, f"{d['id']}: occurrence 3 needs 'Similar: <the board and part it is proven on>' in prevention, "
                   "or a rating of 4-8 by the README's scale")

    plan_n = set()
    vps = {}
    for p in read_csv("vp_map.csv", VP_COLS):
        w = p["_where"]
        try:
            num = int(p["num"])
        except ValueError:
            err(w, f"num {p['num']!r} is not a number")
            continue
        if num in vps:
            err(w, f"duplicate method #{num}")
            continue
        if not 1 <= num <= VP_METHODS:
            err(w, f"#{num}: validation_plan.md (archived) has methods 1-{VP_METHODS}")
        plan_n.add(num)
        if p["status_now"] not in VP_STATUS:
            err(w, f"#{num}: status_now {p['status_now']!r} not in {VP_STATUS}")
        if p["status_now"] != p["status_file"] and not p["evidence"]:
            err(w, f"#{num}: status changed from {p['status_file']} to {p['status_now']} with no evidence")
        toks = p["maps_to"].split()
        p["cut"] = toks == ["CUT"]
        if p["cut"] and not p["cut_reason"]:
            err(w, f"#{num}: CUT needs a cut_reason")
        if not p["cut"]:
            for t in toks:
                if t.startswith("DOC:"):
                    if not os.path.exists(os.path.join(VV, t[4:])):
                        err(w, f"#{num}: {t} does not exist")
                elif not ANY_ID.fullmatch(t):
                    err(w, f"#{num}: maps_to token {t!r} is not an ID, DOC:<file> or CUT")
        p["ids"] = [t for t in toks if ANY_ID.fullmatch(t)]
        vps[num] = p
    missing = sorted(set(range(1, VP_METHODS + 1)) - plan_n)
    if missing:
        err("vp_map.csv", f"methods missing: {missing}")

    defs = parse_blocks("defects.md", RE["DEF"], DEF_FIELDS)
    for d in defs.values():
        st = d["f"].get("Status", "").split()[0] if d["f"].get("Status") else ""
        d["status"] = st
        d["board"] = d["f"].get("Board", "ref_design")     # a defect with no Board line is REF-A's, as it was
        board_ok(d["_where"], d["id"], d["board"], st)
        if st not in DEF_STATUS:
            err(d["_where"], f"{d['id']}: Status must start with one of {DEF_STATUS}")
        d["vers"] = VER_ID.findall(d["f"].get("Check added or changed", ""))
        d["gate"] = d["f"].get("Gate", "").split()[0] if d["f"].get("Gate") else ""
        if d["gate"] not in PHASES:
            err(d["_where"], f"{d['id']}: Gate {d['gate']!r}: the first gate it must be fixed by, one of {PHASES} (O-R5)")
        if st == "RETIRED":
            retired_ok(d["_where"], d["id"], d["f"].get("Status", ""))
        elif st == "DROPPED":
            dropped_ok(d["_where"], d["id"], d["board"], d["f"].get("Status", ""))
            dropped_lesson_ok(d, dfs)
        if st == "CLOSED" and not d["vers"]:
            err(d["_where"], f"{d['id']}: CLOSED with no VER in 'Check added or changed' (no defect closes without a check)")

    STAGES.clear()
    crit, waivers = parse_gates()
    recs = parse_records()
    approvals, eff, gate_lines = parse_approvals()
    db = dict(reqs=reqs, dfs=dfs, vers=vers, vps=vps, defs=defs, crit=crit, waivers=waivers, recs=recs,
              approvals=approvals, eff=eff, gate_lines=gate_lines)
    db["baseline"], db["order_branch"] = parse_baseline()

    base = db["baseline"]
    if not base:
        err("README.md", "no design baseline (## Design baseline, one '- `path` <git object>' line per design path)")
    else:
        miss = [p_ for p_ in DESIGN_PATHS if p_ not in base]
        if miss:
            err("README.md", "the design baseline does not name " + " ".join(miss))
        head_drift, base_drift = baseline_diff(db, None)
        if head_drift:
            err("README.md", "design baseline differs from HEAD at " + " ".join(head_drift) + ": the register describes "
                             "another design (DEF-47); re-check the rows those files touch, then update the baseline")
        if base_drift:
            err("README.md", "design baseline differs from the baseline commit " + base7(db) + " at "
                             + " ".join(base_drift) + ": the register describes another design (F-R3); the baseline must "
                             "name the design as it stands at that commit")
    for m in branch_drift(db):
        warn("README.md", m + ": the gate whose files they touch (order: the board's, case: the case's) fails until "
                              "those rows are re-checked (R8)")
    for m in proposal_drift(db):
        warn("README.md", m + ": the order gate fails (R8) until the register is re-seeded for the board the "
                              "proposal describes, or the baseline moves past it")
    for c in crit.values():
        for t in c["needs"]:
            if t in vers and vers[t]["last_result"] in GONE and not c["retired"]:
                err(c["_where"], f"{c['id']}: needs {t}, which is {vers[t]['last_result']}: name the check that "
                                 "replaced it")
    db["advisory_only"] = advisory_only(db)
    for v in vers.values():
        check_evidence(v, recs)
        ev = v["evidence"].split()[0] if v["last_result"] in ("PASS", "FAIL") and v["evidence"].split() else ""
        if ev.startswith("hardware/") and vv_rel() and not committed_unchanged(ev):
            warn(v["_where"], f"{v['id']}: its evidence {ev} is not committed, or changed since: it counts at no gate "
                              "until committed (O-R2)")
    check_retired_citations(db)
    check_stale_names()
    check_repo_paths()
    check_grid(db)
    check_jlc_fixed()
    for d in defs.values():
        if d["status"] == "CLOSED":
            for vid in d["vers"]:
                if vid in vers and (vers[vid]["last_result"] != "PASS" or staleness(vers[vid], db)):
                    err(d["_where"], f"{d['id']}: CLOSED but {vid} has not passed on fresh evidence")
                elif vid in vers and vers[vid]["missing"]:
                    warn(d["_where"], f"{d['id']}: CLOSED on {vid}, whose PASS rests on "
                                      f"{' '.join(vers[vid]['missing'])}, not on disk (BLOCKED): re-run it once they are")
        # F-R1: R4 reads the Status keyword only, so a Status line whose own words say the defect still holds a
        # gate while the keyword is not OPEN takes it off the gate unseen. The text is read for this
        # consistency check only, never by a gate rule (O-R1 (b)).
        hold = HOLDS_GATE.search(d["f"].get("Status", ""))
        if d["status"] != "OPEN" and hold:
            err(d["_where"], f"{d['id']}: Status is {d['status']} but its own text says '{hold.group(0)}': "
                             "set it OPEN or reword it (F-R1)")

    cross_check_approvals(db)
    check_reach(db)
    check_req_phase(db)
    check_plan_steps(db)
    check_releases(db)
    check_change_reruns(db)
    check_stages(db)

    # every ID referenced anywhere must be defined somewhere
    defined = (set(reqs) | set(dfs) | set(vers) | set(defs) | set(crit) | set(recs) | {w["id"] for w in waivers}
               | set(STAGES))
    texts = []
    for r in reqs.values():
        texts += [(r["_where"], v) for v in r["f"].values()]
    for d in dfs.values():
        texts += [(d["_where"], d[k]) for k in DF_COLS if k != "id"]
    for v in vers.values():
        texts += [(v["_where"], v[k]) for k in ("title", "procedure", "pass_limit", "evidence")]
    for p in vps.values():
        texts += [(p["_where"], p["maps_to"]), (p["_where"], p["evidence"])]
    for d in defs.values():
        texts += [(d["_where"], v) for v in d["f"].values()]
    for c in crit.values():
        texts += [(c["_where"], c["text"])]
        for t in c["needs"]:
            if not (ANY_ID.fullmatch(t) or re.fullmatch(r"PHASE:(" + "|".join(PHASES) + ")", t)
                    or t in ("DEFECTS:CLOSED", "DFMEA:COVERAGE", "ADVISORY")
                    or re.fullmatch(r"ADVISORY:VER-\d{2,3}", t) or (t in GONE and c["retired"])):
                err(c["_where"], f"{c['id']}: needs token {t!r} is not an ID, PHASE:<phase>, "
                                 "DEFECTS:CLOSED, DFMEA:COVERAGE, ADVISORY, ADVISORY:<VER id> or (alone) RETIRED or "
                                 "DROPPED")
            texts.append((c["_where"], t))
    for w in waivers:
        texts.append((w["_where"], w["crit"] + " " + w["reason"]))
    for s in STAGES.values():
        texts.append((s["_where"], " ".join(s["entry"] + s["exit"])))
    for rr in recs.values():
        texts.append((rr["_where"], rr["text"]))
    for a in approvals:
        texts.append((a["_where"], a["item"]))
    seen_bad = set()
    for where, t in texts:
        for i in ANY_ID.findall(t):
            if i not in defined and (where, i) not in seen_bad:
                seen_bad.add((where, i))
                err(where, f"{i} is referenced but not defined")
    for w in waivers:
        if w["crit"] not in crit:
            err(w["_where"], f"{w['id']}: waives {w['crit']!r}, which is not a criterion")
    return db


def tur_check(v):
    """F-38: warn when a pass band is narrower than 4 x the instrument's stated accuracy."""
    if not v["accuracy"]:
        return
    a = re.search(r"\+-\s*([\d.]+)\s*(mV|V|uA|mA|A|C|ohm|kohm|mm)\b", v["accuracy"])
    b = re.search(r"([\d.]+)\s*-\s*([\d.]+)\s*(mV|V|uA|mA|A|C|ohm|kohm|mm)\b", v["pass_limit"])
    if not a or not b:
        return
    ua, ub = UNITS[a.group(2).lower()], UNITS[b.group(3).lower()]
    if ua[0] != ub[0]:
        return
    acc = float(a.group(1)) * ua[1]
    half = (float(b.group(2)) - float(b.group(1))) * ub[1] / 2
    if half > 0 and half < 4 * acc:
        warn(v["_where"], f"{v['id']}: pass band +-{half:g} {ub[0]} is {half / acc:.1f} x the instrument's "
                          f"+-{acc:g} {ua[0]} (want 4 x): use a better instrument or a reference")


def check_evidence(v, recs):
    """O-4, F-3, O-6, O-R3: a PASS or FAIL rests on a saved output under evidence/ whose header matches
    the row: COMMIT and RESULT always; TOOLS for an analysis (A) or a test (T); WHO (the reviewer) and
    ARTEFACT (what was inspected: a screenshot folder with sha256s, a spec sheet's sha256, a dated page)
    for an inspection or demonstration (I, D). Only an I or D check with no release may point at a
    design file instead, and only one its own 'covers' names (VER-104, VER-124: the rule is in the file)."""
    if v["last_result"] not in ("PASS", "FAIL"):
        return
    w, ev = v["_where"], v["evidence"]
    if re.search(r"\bgit (show|log)\b", ev):
        err(w, f"{v['id']}: a commit message is not evidence: save the check's output under {EVIDENCE_DIR}")
        return
    first = ev.split()[0] if ev.split() else ""
    if not first.startswith("hardware/") or not os.path.isfile(os.path.join(ROOT, first)):
        err(w, f"{v['id']}: evidence must start with a file in the repository")
        return
    meth = set(v["method"].split("/"))
    if not first.startswith(EVIDENCE_DIR):
        pointer = not ({"A", "T"} & meth) and not v["release"] and any(
            k == "glob" and s[0].match(first) for k, s, _ in v["cov"])
        if not pointer:
            err(w, f"{v['id']}: needs its saved output under {EVIDENCE_DIR} with a header (an analysis, a test, a "
                   "bench result, or an inspection of anything but a file its own 'covers' names)")
        return
    text = open(os.path.join(ROOT, first), encoding="utf-8", errors="replace").read()
    m = re.search(r"^COMMIT: ([0-9a-f]{7,40})\b", text, re.M)
    if not m or not (m.group(1).startswith(v["commit"]) or v["commit"].startswith(m.group(1))):
        err(w, f"{v['id']}: {first} has no 'COMMIT: {v['commit']}' line")
    if {"A", "T"} & meth and not re.search(r"^TOOLS: \S", text, re.M):
        err(w, f"{v['id']}: {first} has no 'TOOLS:' line (tool versions)")
    if not ({"A", "T"} & meth):
        for k, what in (("WHO", "who inspected it"), ("ARTEFACT", "what was inspected, with its sha256 or date")):
            if not re.search(r"^" + k + r": \S", text, re.M):
                err(w, f"{v['id']}: {first} has no '{k}:' line ({what})")
    res = re.findall(r"^RESULT: (PASS|FAIL)\b", text, re.M)
    if not res or res[-1] != v["last_result"]:
        err(w, f"{v['id']}: the row says {v['last_result']} but {first} ends "
               f"{'RESULT: ' + res[-1] if res else 'with no RESULT line'}")
    for r in v["release"]:
        if not re.search(r"^RELEASE: " + re.escape(r) + r"\b", text, re.M):
            err(w, f"{v['id']}: {first} has no 'RELEASE: {r}' line")
        if not re.search(r"^UNITS: \S", text, re.M):
            err(w, f"{v['id']}: {first} has no 'UNITS:' line (the serials it ran on)")
        rr = recs.get(r)
        if rr is None:
            err(w, f"{v['id']}: covers {r}, which has no release record")
        elif rr["fab_sha"] and rr["fab_sha"] not in text:
            err(w, f"{v['id']}: {first} does not name {r}'s fab zip sha256")


def eff_req_status(db, rid, phase=None):
    """Alex's decision for a requirement; at a gate (phase given) a deferral counts only up to
    the phase it names, after which the requirement is PROPOSED again."""
    st = db["reqs"].get(rid, {}).get("f", {}).get("Status")
    if st in GONE and rid not in db["eff"]:
        return st
    a = db["eff"].get(rid)
    if not a:
        return "PROPOSED"
    if phase and a["decision"] == "DEFERRED" and not phase_le(phase, a["scope"]):
        return "PROPOSED"
    return a["decision"]


def eff_df_accept(db, did):
    a = db["eff"].get(did)
    return a if a and a["decision"] == "ACCEPTED" else None


def def_accept(db, did):
    """Alex's signed 'DF-ACCEPT:DEF-nn' line for a defect (2026-10-28), or None.  Its scope is the gate the defect
    moves to."""
    a = db["eff"].get("DF-ACCEPT:" + did)
    return a if a and a["decision"] == "ACCEPTED" and a["scope"] in PHASES else None


def def_moved(db, did, phase):
    """The signed line that lets an ACCEPTED defect past this gate: the gate comes before the one it moved to.
    At that gate and after it, the defect counts as it would OPEN (R4) and its checks count as they are."""
    d = db["defs"].get(did)
    a = def_accept(db, did) if d and d["status"] == "ACCEPTED" else None
    return a if a and PHASES.index(phase) < PHASES.index(a["scope"]) else None


def accepted_checks(db, phase, board=None):
    """{VER id: 'DEF-nn to <gate>, A-nnn'}: the checks an ACCEPTED defect names in 'Check added or changed', which
    the gate's criteria (G-ORD-22's PHASE:ORDER among them) let through before the gate the defect moved to.  A
    check no accepted defect names gets nothing.  It does not verify a requirement (R2) or an H row (R3)."""
    out = {}
    for did, d in sorted(db["defs"].items()):
        if board and not for_board(d["board"], board):
            continue
        a = def_moved(db, did, phase)
        if a:
            for vid in d["vers"]:
                out.setdefault(vid, f"{did} to {a['scope']}, {a['id']}")
    return out


def cross_check_approvals(db):
    """O-45: statuses that need Alex must match a line he committed in APPROVALS.md."""
    for r in db["reqs"].values():
        st = r["f"].get("Status")
        if st == "PROPOSED" and db["eff"].get(r["id"], {}).get("decision") == "APPROVED":
            # 2026-10-29: a signed statement edited since is not approved any more; the row goes back to PROPOSED
            # (Status says so) and waits for Alex's new line. The old line stays in APPROVALS.md as history; while
            # Status stays APPROVED the error below still fires.
            a = db["eff"][r["id"]]
            rel = vv_rel("requirements.md")
            bl = blame(rel) if rel and rel in TRACKED else {}
            if any(set(bl.get(n, "0" * 40)) <= {"0"} or not is_ancestor(bl.get(n, "0" * 40), a["sha"])
                   for n in r["lines"].get("Statement", [])):
                warn(r["_where"], f"{r['id']}: its statement changed after Alex approved it ({a['id']}): PROPOSED "
                                  "again until he approves the new text")
                del db["eff"][r["id"]]
        eff = eff_req_status(db, r["id"])
        if st in GONE and eff != st:
            err(r["_where"], f"{r['id']}: {st}, but Alex decided it ({db['eff'][r['id']]['id']}): he rejects it instead")
        elif st != "PROPOSED" and st != eff:
            pend = [f"{a['id']} pending: {a['pending']}" for a in db["approvals"] if a["item"] == r["id"] and not a["committed"]]
            err(r["_where"], f"{r['id']}: Status {st} but releases/APPROVALS.md "
                             f"{'says ' + eff if eff != 'PROPOSED' else 'has no committed line for it'}"
                             + (f" ({'; '.join(pend)})" if pend else ""))
        elif st == "PROPOSED" and eff != "PROPOSED":
            warn(r["_where"], f"{r['id']}: APPROVALS.md says {eff}: update its Status")
        if eff == "APPROVED":
            a = db["eff"][r["id"]]
            rel = vv_rel("requirements.md")
            bl = blame(rel) if rel and rel in TRACKED else {}
            for n in r["lines"].get("Statement", []):
                sha = bl.get(n, "0" * 40)
                if set(sha) <= {"0"} or not is_ancestor(sha, a["sha"]):
                    err(r["_where"], f"{r['id']}: its statement changed after Alex approved it ({a['id']}): "
                                     "he approves it again")
                    break
    for d in db["dfs"].values():
        acc = eff_df_accept(db, d["id"])
        if d["status"] == "ACCEPTED" and not acc:
            pend = [f"{a['id']} pending: {a['pending']}" for a in db["approvals"] if a["item"] == d["id"] and not a["committed"]]
            err(d["_where"], f"{d['id']}: ACCEPTED but releases/APPROVALS.md has no committed line for it"
                + (f" ({'; '.join(pend)})" if pend else ""))
        elif d["status"] != "ACCEPTED" and acc:
            warn(d["_where"], f"{d['id']}: APPROVALS.md accepts it for {acc['scope']}: set status ACCEPTED")
    # 2026-10-28: a defect ACCEPTED to a later gate needs Alex's signed DF-ACCEPT line; an unsigned one fails here
    for d in db["defs"].values():
        acc = def_accept(db, d["id"])
        if d["status"] == "ACCEPTED":
            if not acc:
                pend = [f"{a['id']} pending: {a['pending']}" for a in db["approvals"]
                        if a["item"] == "DF-ACCEPT:" + d["id"] and not a["committed"]]
                err(d["_where"], f"{d['id']}: ACCEPTED but releases/APPROVALS.md has no signed 'DF-ACCEPT:{d['id']}' "
                                 "line naming the gate it moves to and why" + (f" ({'; '.join(pend)})" if pend else ""))
            elif d["gate"] in PHASES and PHASES.index(acc["scope"]) <= PHASES.index(d["gate"]):
                err(d["_where"], f"{d['id']}: {acc['id']} moves it to {acc['scope']}, which is not later than its "
                                 f"Gate {d['gate']}")
        elif acc and d["status"] == "OPEN":
            warn(d["_where"], f"{d['id']}: {acc['id']} accepts it to {acc['scope']}: set its Status ACCEPTED")
    for a in db["approvals"]:
        if not a["committed"] and a["pending"] == "not committed":
            warn(a["_where"], f"{a['id']} ({a['item']} {a['decision']}) is not committed by Alex yet: it does not count")


def binding_reach(db):
    """Checks a requirement (not REJECTED or RETIRED: a deferral ends) or a live H row lists."""
    out = set()
    for r in db["reqs"].values():
        if eff_req_status(db, r["id"]) not in ("REJECTED",) + GONE:
            out |= set(r["vers"])
    for d in db["dfs"].values():
        if d["action_priority"] == "H" and d["live"]:
            out |= set(d["det"])
    return out


def advisory_only(db):
    """Checks that only ADVISORY criteria name (F-32: VER-40), and checks a criterion names as
    'ADVISORY:<id>' (O-R9: advisory at one gate, binding at a later one): printed, never counted by PHASE:."""
    named, adv = binding_reach(db), set()
    for c in db["crit"].values():
        for t in c["needs"]:
            if t.startswith("ADVISORY:"):
                adv.add(t.split(":", 1)[1])
            elif t.startswith("VER-"):
                (adv if c["advisory"] else named).add(t)
    return adv - named


def check_reach(db):
    """O-1: a check must be reached by a gate, a requirement or an H row no later than its phase."""
    vers, crit = db["vers"], db["crit"]
    reached = binding_reach(db) | db["advisory_only"]
    for c in crit.values():
        if c["advisory"]:
            continue
        for vid, v in vers.items():
            if c["board"] != "both" and v["board"] not in (c["board"], "both"):
                continue
            if PHASES.index(c["phase"]) > PHASES.index(v["phase"]):
                continue
            if vid in c["needs"] or any(t.startswith("PHASE:") and phase_le(v["phase"], t[6:]) for t in c["needs"]):
                reached.add(vid)
    for vid, v in vers.items():
        if v["last_result"] not in ("CUT",) + GONE and vid not in reached:
            err(v["_where"], f"{vid}: phase {v['phase']}, but no gate criterion at or before {v['phase']}, "
                             "requirement or H-priority risk reaches it: that gate can pass without it")


def check_req_phase(db):
    """F's objection: a requirement proven by its Phase has a live check of that phase or earlier, so a check
    moved to a later phase cannot quietly leave a requirement that still says it is proven earlier (R2 would
    pass it there on no evidence)."""
    for r in db["reqs"].values():
        if r["f"].get("Status") in ("REJECTED",) + GONE or r["f"].get("Phase") not in PHASES:
            continue
        live = [(PHASES.index(db["vers"][v]["phase"]), v) for v in r["vers"]
                if v in db["vers"] and db["vers"][v]["last_result"] not in ("CUT",) + GONE]
        if not live:
            continue
        first, vid = min(live)
        if first > PHASES.index(r["f"]["Phase"]):
            err(r["_where"], f"{r['id']}: Phase {r['f']['Phase']}, but its earliest live check ({vid}) is "
                             f"{PHASES[first]}: nothing proves it by its Phase; move the requirement's Phase or a check's")


def check_plan_steps(db):
    """Round 2's objection: every ORDER check (advisory-only, CUT and RETIRED ones aside) is named by a step of
    PLAN.md section (d), so the order checklist cannot drift from the gate again.  O-R11: only the numbered
    step rows count: the 'what moved where' table is history."""
    path = os.path.join(VV, "PLAN.md")
    if not os.path.exists(path):
        err("PLAN.md", "missing")
        return
    text = open(path, encoding="utf-8").read()
    m = re.search(r"^## \(d\).*?(?=^## )", text, re.M | re.S)
    if not m:
        err("PLAN.md", "no section '## (d)' (the steps before the order)")
        return
    named = set()
    for line in m.group(0).splitlines():
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if not cells or not re.fullmatch(r"\d+(\.\d+)?", cells[0]):
            continue
        named |= set(VER_ID.findall(line))
    for vid, v in db["vers"].items():
        if (v["phase"] == "ORDER" and v["last_result"] not in ("CUT",) + GONE
                and vid not in db["advisory_only"] and vid not in named):
            err(v["_where"], f"{vid}: an ORDER check that no numbered step of PLAN.md section (d) names: add it to a "
                             "step (with its effort) or move it to the phase where it matters")


WAS = re.compile(r"\bWas:[^.;|]*")
REF_RX = re.compile(r"\b([A-Z]{1,3})(\d{1,3})(?:\s*-\s*([A-Z]{1,3})?(\d{1,3}))?\b")


def refs_named(text):
    """Every design ref a text names, ranges like C10-C22 expanded."""
    out = set()
    for m in REF_RX.finditer(text):
        a, n1, b, n2 = m.groups()
        if n2 and b in (None, a) and int(n1) <= int(n2):
            out |= {f"{a}{i}" for i in range(int(n1), int(n2) + 1)}
        else:
            out.add(a + n1)
            if n2 and b:
                out.add(b + n2)
    return out


def check_retired_citations(db):
    """O-R9, F-R14: a live row may not lean on a RETIRED requirement, DFMEA row or check in its text,
    except inside a 'Was: ...' clause (history). Defects are history and are not read."""
    gone = ({k: r["f"]["Status"] for k, r in db["reqs"].items() if r["f"].get("Status") in GONE}
            | {k: d["status"] for k, d in db["dfs"].items() if not d["live"]}
            | {k: v["last_result"] for k, v in db["vers"].items() if v["last_result"] in GONE})
    texts = []
    for r in db["reqs"].values():
        if r["f"].get("Status") not in GONE:
            texts += [(r["_where"], r["id"], v) for k, v in r["f"].items() if k not in ("Retired", "Dropped")]
    for d in db["dfs"].values():
        if d["live"]:
            texts += [(d["_where"], d["id"], d[k]) for k in DF_COLS if k != "id"]
    for v in db["vers"].values():
        if v["last_result"] not in GONE + ("CUT",):
            texts += [(v["_where"], v["id"], v[k]) for k in ("title", "procedure", "pass_limit", "covers")]
    for where, rid, t in texts:
        bad = sorted({i for i in ANY_ID.findall(WAS.sub("", t)) if i in gone and i != rid})
        if bad:
            what = " or ".join(sorted({gone[i] for i in bad}))
            err(where, f"{rid}: cites {', '.join(bad)}, which {'is' if len(bad) == 1 else 'are'} {what}: name the "
                       "live row, or keep the history inside 'Was: ...'")


BOARD_DOC = re.compile(r"(?<!for )(?<!into )\b(assembly|bom|bringup|pinmap)\.md\b")
HOLDS_GATE = re.compile(r"stays OPEN|the \w+ gate fails on it|holds the \w+ gate|holds Gate \w+", re.I)


def check_repo_paths():
    """2026-10-28 (VER-220): main moved or deleted about 180 of the paths this register named and no rule saw it.
    Every repository path a file under hardware/vv names (the saved outputs aside) is in HEAD's tree, ignored by
    git (a generated output), or pinned 'path@<commit>' at a commit that has it. tools/repo_paths.py holds the scan."""
    bad, _ = repo_paths.findings(VV)
    for where, tok, msg in bad:
        err(where, msg)


def check_stale_names():
    """2026-10-27 (VER-188): a live row may not name a part, part number or net that design.py at the baseline
    commit no longer has, outside 'Was: ...'. tools/stale_names.py holds the scan; it runs here on every change."""
    for where, rid, msg in stale_names.findings(VV):
        err(where, f"{rid}: {msg}: retire the row, or rewrite it for the design and keep the history in 'Was: ...'")


def grid_signed(db, phase=None):
    """The coverage-grid gaps Alex has accepted with a signed 'GRID-<class>-<subsystem>' line (for the phase)."""
    return frozenset(k for k, a in db["eff"].items() if k.startswith("GRID-") and a["decision"] == "ACCEPTED"
                     and (phase is None or phase_le(phase, a["scope"])))


def check_grid(db):
    """2026-10-27: every live check names the classes and subsystems it targets, grid_reasons.csv is well formed,
    a NOT_NEEDED cell has no check, and grid.md is what tools/grid.py makes of the register (a warning: it is
    generated; run_checks.py and grid.py rewrite it)."""
    signed, accepted = grid_signed(db), accepted_checks(db, "ORDER")
    vers, reasons, grid, problems = cgrid.build(VV, signed, accepted)
    for where, msg in problems:
        err(where, msg)
    db["grid"] = grid
    for item in cgrid.signed_without_row(reasons, signed):
        warn("grid_reasons.csv", f"{item} is signed in APPROVALS.md but grid_reasons.csv has no ACCEPTED_GAP row for "
                                 "it: an accepted gap needs both, so the cell stays a GAP")
    if not problems:
        path = os.path.join(VV, "grid.md")
        old = open(path, encoding="utf-8").read() if os.path.exists(path) else ""
        if not cgrid.same(old, cgrid.render(vers, reasons, grid, accepted, signed)):
            warn("grid.md", "not what the register gives: run python3 hardware/vv/tools/grid.py")


def changed_parts(sha):
    """Refs whose value, LCSC number, footprint or pins a commit changed in design.py."""
    old, new = design_parts(sha + "^"), design_parts(sha)
    if old is None or new is None:
        return set()
    return {r for r in set(old) | set(new) if part_key(old.get(r)) != part_key(new.get(r))}


def check_change_reruns(db):
    """O-R21: a change record's 'What to re-run' is not hand-picked. Every bench test (method T, a
    release cover) that a DFMEA row or a requirement ties to a part the recorded commits changed
    must be listed, or the old units' results would stay fresh for a board that is no longer theirs."""
    vers = db["vers"]
    bench = [v for v in vers.values() if "T" in v["method"].split("/") and v["release"]
             and v["last_result"] not in ("CUT",) + GONE]
    for ch in db["recs"].values():
        if ch["kind"] != "CHG":
            continue
        moved = {}
        for x in ch["commits"]:
            sha = resolve(x)
            if sha:
                for r in changed_parts(sha):
                    moved.setdefault(r, x)
        if not moved:
            continue
        missing = []
        for v in bench:
            if v["id"] in ch["reruns"]:
                continue
            why = [(d["id"], r) for d in db["dfs"].values() if d["live"] and v["id"] in d["det"]
                   for r in refs_named(" ".join(d[k] for k in ("item", "failure_mode", "cause"))) if r in moved]
            why += [(q["id"], r) for q in db["reqs"].values() if v["id"] in q["vers"]
                    for r in refs_named(q["f"].get("Statement", "") + " " + q["f"].get("Source", "")) if r in moved]
            if why:
                missing.append(f"{v['id']} ({why[0][0]} names {why[0][1]}, changed in {moved[why[0][1]]})")
        if missing:
            err(ch["_where"], f"{ch['id']}: 'What to re-run' omits " + "; ".join(missing[:6])
                + (f" (+{len(missing) - 6})" if len(missing) > 6 else ""))


def check_releases(db):
    """O-35: a release record whose BOM differs from another's must carry a new BOARD_REV.  O-R4: it must also
    carry the case's positions hash, so a case commit cannot have moved the board unnoticed."""
    rels = [r for r in db["recs"].values() if r["kind"] == "REL"]
    for i, a in enumerate(rels):
        for b in rels[i + 1:]:
            if a["board"] == b["board"] and a["bom_sha"] and b["bom_sha"] and a["board_rev"] and b["board_rev"]:
                if a["bom_sha"] != b["bom_sha"] and a["board_rev"] == b["board_rev"]:
                    err(b["_where"], f"{b['id']}: its BOM differs from {a['id']}'s but BOARD_REV is still "
                                     f"{b['board_rev']}: bump design.py's BOARD_REV (the silkscreen's revision) with the BOM change")
        if a["commit"] and not resolve(a["commit"]):
            err(a["_where"], f"{a['id']}: commit {a['commit']!r} is not in this repository")
        if not a["case_sha"]:
            err(a["_where"], f"{a['id']}: no 'Case positions sha256' field: the record must carry the sha256 of "
                             f"case/positions.json at the commit above (O-R4)")
        if not a["case_base_sha"]:
            err(a["_where"], f"{a['id']}: no 'Case positions sha256 at the baseline' field: the record must carry "
                             f"the same file's hash at the baseline commit, so the two can be compared (O-R4)")


# ---- freshness ---------------------------------------------------------------------
def staleness(v, db, dirty=None):
    """None if fresh, else a short reason."""
    if v["last_result"] not in ("PASS", "FAIL"):
        return None
    c = resolve(v["commit"])
    if not c:
        return "commit unknown"
    if not is_ancestor(c):
        return f"{v['commit']} is not an ancestor of HEAD"
    hits = []
    changed = changed_since(c)
    for kind, spec, tok in v["cov"]:
        if kind == "glob":
            g, rx = spec
            for f in sorted(changed):
                if g.match(f) and (rx is None or any(rx.search(ln) for ln in changed_lines(c, f))):
                    hits.append(f)
            if dirty:
                hits += [f + " (uncommitted)" for f in sorted(dirty) if g.match(f)]
        elif kind == "part":
            old, new = design_parts(c), design_parts(None)
            if old is None or new is None:
                hits.append("design.py could not be read at " + v["commit"])
                continue
            moved = [r for r in spec if part_key(old.get(r)) != part_key(new.get(r))]
            if moved:
                hits.append("design.py parts " + ",".join(moved[:6]))
        elif kind == "release":
            for ch in db["recs"].values():
                if ch["kind"] == "CHG" and v["id"] in ch["reruns"] and (
                        not ch["commits"] or any(not (resolve(x) and is_ancestor(resolve(x), c)) for x in ch["commits"])):
                    hits.append(f"{ch['id']} asks for a re-run")
    if v["valid_days"] and v["date"]:
        age = (TODAY - datetime.date.fromisoformat(v["date"])).days
        if age > int(v["valid_days"]):
            hits.append(f"expired: {age} days old, valid {v['valid_days']}")
    if hits:
        uniq = sorted(set(hits))
        return "changed: " + ", ".join(uniq[:4]) + (f" (+{len(uniq) - 4})" if len(uniq) > 4 else "")
    return None


def for_board(item_board, board):
    return item_board in (board, "both")


def phase_le(a, b):
    return PHASES.index(a) <= PHASES.index(b)


def req_scope(db, board):
    """Requirements a board's gate evaluates (O-7)."""
    reqs = db["reqs"]
    live = {k: r for k, r in reqs.items() if r["f"].get("Status") not in GONE}
    return {k: r for k, r in live.items() if r["f"].get("Board") in (board, "both")}


def df_scope(db, board):
    return [d for d in db["dfs"].values() if d["board"] in (board, "both") and d["live"]]


def coverage(db):
    """O-55: bom=True refs and every contact that no DFMEA row names (ranges like C10-C22 count)."""
    parts = design_parts(None) or {}
    want = [r for r, p in parts.items() if not r.startswith("#") and (p.get("bom") or r.startswith("SW"))]
    named = set()
    rx = re.compile(r"\b([A-Z]{1,3})(\d{1,3})(?:\s*-\s*([A-Z]{1,3})?(\d{1,3}))?\b")
    for d in db["dfs"].values():
        if not d["live"]:
            continue
        text = " ".join(d[k] for k in ("item", "function", "failure_mode", "effect", "cause", "prevention", "action"))
        for m in rx.finditer(text):
            a, n1, b, n2 = m.groups()
            if n2 and b in (None, a) and int(n1) <= int(n2):
                named |= {f"{a}{i}" for i in range(int(n1), int(n2) + 1)}
            else:
                named.add(a + n1)
                if n2 and b:
                    named.add(b + n2)
    return want, [r for r in want if r not in named]


# ---- report --------------------------------------------------------------------------
def show(label, ids, verbose, limit=40):
    ids = list(ids)
    print(f"  {label}: {len(ids)}")
    if not ids:
        return
    shown = ids if verbose else ids[:limit]
    line = "    "
    for i in shown:
        if len(line) + len(i) + 1 > 98:
            print(line.rstrip())
            line = "    "
        line += i + " "
    if line.strip():
        print(line.rstrip())
    if len(ids) > len(shown):
        print(f"    ... and {len(ids) - len(shown)} more (--verbose)")


def base7(db):
    """The baseline commit's short hash (the order branch's base), or '' when README names none."""
    return db["order_branch"][1][:7] if db["order_branch"] else "?"


def baseline_diff(db, rels):
    """F-R3: README's baseline hashes must match HEAD's blobs and the baseline commit's own blobs: a README
    hash rewritten after a design.py edit would otherwise pass, and the register would describe a design that
    was never ordered.  Returns (head_drift, base_drift): the paths whose README hash differs from HEAD, and
    the paths whose README hash differs from the baseline commit (the order branch's base)."""
    head, base_out = [], []
    base = db["order_branch"][1] if db["order_branch"] else None
    for path, blob in db["baseline"].items():
        if rels and not any(path.endswith(r) for r in rels):
            continue
        rc, now = git("rev-parse", f"HEAD:{path}")
        if rc != 0 or now.strip() != blob:
            head.append(path)
            continue
        if base and resolve(base):
            rc, there = git("rev-parse", f"{base}:{path}")
            if rc != 0 or there.strip() != blob:
                base_out.append(path)
    return head, base_out


def report(db, verbose):
    reqs, dfs, vers, vps, defs = db["reqs"], db["dfs"], db["vers"], db["vps"], db["defs"]
    print(f"trace report at {HEAD[:7]} ({ROOT})")
    print(f"  requirements {len(reqs)} " + str(dict(Counter(eff_req_status(db, r) for r in reqs))))
    print(f"  DFMEA rows {len(dfs)}, action priority " + str(dict(sorted(Counter(d['action_priority'] for d in dfs.values()).items()))))
    print(f"  checks {len(vers)} " + str(dict(Counter(v['last_result'] for v in vers.values()))))
    cut = [p for p in vps.values() if p["cut"]]
    print(f"  validation-plan methods {len(vps)}: {len(vps) - len(cut)} mapped, {len(cut)} cut (archived plan)")
    print(f"  defects {len(defs)} " + str(dict(Counter(d['status'] for d in defs.values()))))
    print(f"  approvals by Alex: {sum(1 for a in db['approvals'] if a['committed'])} counted (signed), "
          f"{sum(1 for a in db['approvals'] if not a['committed'])} pending"
          + ("" if SIGNERS else "; commit signing not set up, so none can count yet")
          + ("; " + pinned_key()[1] if pinned_key()[1] else f"; pinned key {pinned_key()[0]}"))
    bdr = branch_drift(db)
    print("  order branch: " + ("; ".join(bdr) if bdr else f"{db['order_branch'][0]} has no design commit after "
                                                            f"{db['order_branch'][1]}"))
    props = proposal_drift(db)
    print("  architecture proposals: " + ("; ".join(props) + " (warning here, R8 fails the order gate)" if props
                                          else "none changed on the order branch since the baseline"))
    head_drift, base_drift = baseline_diff(db, None)
    print("  design baseline: " + (("differs from HEAD: " + " ".join(head_drift)) if head_drift else "matches HEAD")
          + (("; differs from the baseline commit " + base7(db) + ": " + " ".join(base_drift)) if base_drift else "")
          if db["baseline"] else "  design baseline: none in README.md")
    grid = db.get("grid") or {}
    held = cgrid.order_gaps(grid)
    k = Counter(c["state"] for (cl, _), c in grid.items() if cl in cgrid.ORDER_CLASSES)
    print(f"  coverage grid (grid.md), classes 1-15 by {len(cgrid.SUBSYSTEMS)} subsystems: {k['COVERED']} covered, "
          f"{k['NOT_NEEDED']} not needed, {k['ACCEPTED_GAP']} accepted; GAP {len(held)} "
          f"({k['GAP']} with no check, {k['planned']} planned: no check that can clear them built or run), "
          "which the order gate holds (R9)")
    per = Counter(cl for cl, _, _ in held)
    if per:
        print("    GAP per class: " + ", ".join(f"{cl} {cgrid.CLASSES[cl]} {n}" for cl, n in sorted(per.items())))
    print()
    print("requirements")
    for b in BOARDS:
        scope = req_scope(db, b)
        show(f"in scope for {b} with no check for it",
             [r for r, q in scope.items() if not q["alloc"].get(b) and not any(
                 for_board(vers[v]["board"], b) for v in q["vers"] if v in vers)], verbose)
    live = {k: q for k, q in reqs.items() if q["f"].get("Status") not in GONE}
    show("no listed check has passed", [r for r in live if live[r]["vers"] and not any(
        vers.get(v, {}).get("last_result") == "PASS" for v in live[r]["vers"])], verbose)
    show("statement has no number (INCOSE: state a value and tolerance)",
         [r for r in live if not re.search(r"\d", live[r]["f"].get("Statement", ""))], verbose)
    show("RETIRED (their subject left the design)", [r for r in reqs if reqs[r]["f"].get("Status") == "RETIRED"],
         verbose)
    show("DROPPED (they belonged to a dropped board; its tag holds them)",
         [r for r in reqs if reqs[r]["f"].get("Status") == "DROPPED"], verbose)
    print("DFMEA")
    show("RETIRED rows", [d["id"] for d in dfs.values() if d["status"] == "RETIRED"], verbose)
    show("DROPPED rows", [d["id"] for d in dfs.values() if d["status"] == "DROPPED"], verbose)
    hi = [d for d in dfs.values() if d["action_priority"] == "H" and d["live"]]
    show("H rows", [d["id"] for d in hi], verbose)
    show("H rows with no 'detects' check", [d["id"] for d in hi if not d["det"]], verbose)
    show("H rows whose 'detects' checks have not all passed on fresh evidence",
         [d["id"] for d in hi if d["det"] and not all(
             vers.get(v, {}).get("last_result") == "PASS" and not staleness(vers[v], db) for v in d["det"])
          and not eff_df_accept(db, d["id"])], verbose)
    show("severity 9-10 rows", [d["id"] for d in dfs.values() if d["S"] >= 9 and d["live"]], verbose)
    if BASELINE_BOARD in BOARDS:
        want, miss = coverage(db)
        show(f"design.py parts and contacts named in no row (of {len(want)})", miss, verbose)
    else:
        print(f"  design.py parts named in no row: not counted (design.py is the {BASELINE_BOARD} board's; "
              f"no {BOARDS[0]} design.py yet)")
    print("checks")
    for res in ("FAIL", "NOT_RUN", "NOT_BUILT", "CUT", "RETIRED", "DROPPED"):
        show(res, [v for v in vers if vers[v]["last_result"] == res], verbose)
    show("advisory only (named only by ADVISORY criteria; PHASE: does not count them)",
         sorted(db["advisory_only"]), verbose)
    blocked = [v for v in vers if vers[v]["last_result"] == "PASS" and vers[v]["missing"]]
    print(f"  BLOCKED (a PASS that rests on a datasheet not in hardware/ref_design/datasheets; no gate counts it): "
          f"{len(blocked)}")
    for v in blocked:
        print(f"    {v}: {' '.join(vers[v]['missing'])}")
    acc = accepted_checks(db, "ORDER")
    show("ACCEPTED at the order gate (named by a defect Alex moved to a later gate with a signed line)",
         [f"{v}({acc[v].split(',')[0]})" for v in sorted(acc)], verbose)
    stale = [(v, staleness(vers[v], db)) for v in vers]
    stale = [(v, s) for v, s in stale if s]
    print(f"  stale evidence: {len(stale)}")
    for v, s in stale:
        print(f"    {v} ({vers[v]['last_result']}, {vers[v]['commit']}): {s}")
    print("validation plan (archived; vp_map.csv is a one-time mapping)")
    show("unmapped methods", [f"#{n}" for n, p in sorted(vps.items()) if not p["cut"] and not p["ids"]
                              and not p["maps_to"].startswith("DOC:")], verbose)
    print("defects")
    show("OPEN", [d for d in defs if defs[d]["status"] == "OPEN"], verbose)
    show("FIXED but no check yet passes (cannot close)", [d for d in defs if defs[d]["status"] == "FIXED"], verbose)
    show("ACCEPTED (moved to a later gate by Alex's signed DF-ACCEPT line)",
         [f"{d}(to {def_accept(db, d)['scope']})" if def_accept(db, d) else d
          for d in defs if defs[d]["status"] == "ACCEPTED"], verbose)
    show("RETIRED (the part or function it concerned left the design)",
         [d for d in defs if defs[d]["status"] == "RETIRED"], verbose)
    show("DROPPED (it belonged to a dropped board: not fixed, not closed; its tag holds it)",
         [d for d in defs if defs[d]["status"] == "DROPPED"], verbose)
    at, lines = stage_report(db)
    print(f"new-MCU path (gates.md): at stage {at or 'none open'}"
          + (f" ({STAGES[at]['name']})" if at in STAGES else ""))
    for ln in lines:
        print(ln)
    if WARNINGS:
        print(f"warnings: {len(WARNINGS)}")
        for wn in WARNINGS if verbose else WARNINGS[:15]:
            print("  " + wn)
        if not verbose and len(WARNINGS) > 15:
            print(f"  ... and {len(WARNINGS) - 15} more (--verbose)")


def shopping(db, phase, board):
    """F-50: what the next gate's unrun checks still need, longest lead time first."""
    vers, crit = db["vers"], db["crit"]
    named = {t for c in crit.values() if c["phase"] == phase for t in c["needs"]}
    rows = []
    for vid, v in vers.items():
        if not v["needs"] or v["last_result"] not in ("NOT_RUN", "NOT_BUILT") or not for_board(v["board"], board):
            continue
        if phase_le(v["phase"], phase) or vid in named:
            m = re.findall(r"\b(\d+)d\b", v["needs"])
            rows.append((-max(int(x) for x in m) if m else 1, vid, v["needs"]))
    print(f"shopping for the {phase} gate ({board}): {len(rows)} checks still need something")
    for days, vid, need in sorted(rows):
        print(f"  {'?' if days == 1 else -days:>4}d  {vid}  {need}")
    if any(d == 1 for d, _, _ in rows):
        print("  (?: no lead time given: add '<N>d' to the 'needs' entry)")


# ---- gates ---------------------------------------------------------------------------
def gate(db, phase, board, verbose, accept_sw=False):
    reqs, dfs, vers, defs, crit = db["reqs"], db["dfs"], db["vers"], db["defs"], db["crit"]
    dirty = dirty_files()
    waived = {}
    for w in db["waivers"]:
        a = db["eff"].get(w["id"])
        c = crit.get(w["crit"])
        if a and a["decision"] == "WAIVED" and c and phase_le(c["phase"], a["scope"]):
            waived[w["crit"]] = (w, a)

    accepted = accepted_checks(db, phase, board)

    def ver_ok(vid, honour=False):
        """Whether a check passes here.  honour: a gate criterion's view, which lets through a check an ACCEPTED
        defect names before the gate it moved to (2026-10-28); R2 and R3 do not (a requirement or an H row takes
        its own deferral or acceptance)."""
        ok, why = _ver_ok(vid)
        if not ok and honour and vid in accepted:
            return True, f"accepted: {vid} ACCEPTED ({accepted[vid]}; it is {why.split(' ', 1)[-1]})"
        return ok, why

    def _ver_ok(vid):
        v = vers.get(vid)
        if v is None:
            return False, f"{vid} undefined"
        if v["last_result"] != "PASS":
            return False, f"{vid} {v['last_result']}"
        if v["missing"]:
            # 2026-10-28: a PASS that rests on a datasheet not on disk is a stand-in's PASS
            return False, (f"{vid} BLOCKED: its PASS rests on {' '.join(v['missing'])}, not in "
                           "hardware/ref_design/datasheets")
        s = staleness(v, db, dirty)
        if s:
            return False, f"{vid} stale ({s})"
        ev = v["evidence"].split()[0] if v["evidence"].split() else ""
        if vv_rel() and not committed_unchanged(ev):
            return False, f"{vid} evidence {ev} not committed, or changed since (O-R2)"
        # O's objection, O-R11, F-R1: at DVT and PVT a result on the earlier spin's units does not count,
        # whatever the method (an EVT first article does not inspect the first PVT unit)
        if phase in ("DVT", "PVT") and phase_le("EVT", v["phase"]) \
                and ("T" in v["method"].split("/") or v["release"]) \
                and not any(r.endswith("-" + phase) for r in v["release"]):
            return False, f"{vid} covers no {phase} release (its result is from {' '.join(v['release']) or 'no release'})"
        return True, ""

    def df_ok(d):
        acc = eff_df_accept(db, d["id"])
        if acc and phase_le(phase, acc["scope"]):
            return True, ""
        if acc:
            return False, f"{d['id']} accepted only up to {acc['scope']} ({acc['id']})"
        if d["status"] != "DONE":
            return False, f"{d['id']} {d['status']}"
        bad = [t for t in d["det"] if not (t in vers and phase_le(vers[t]["phase"], phase) and ver_ok(t)[0])]
        return (not bad), (f"{d['id']} DONE but detects not passing by {phase}: {' '.join(bad)}" if bad else "")

    def need_ok(tok):
        if tok == "ADVISORY":
            return None, ""
        if tok.startswith("ADVISORY:"):
            # O-R9: this check is advisory at this gate and binding at a later one: it is printed, never fails
            return None, f"{tok.split(':', 1)[1]} is advisory here and binding at the gate it moved to"
        if tok.startswith("PHASE:"):
            p = tok.split(":")[1]
            mine = [v for v in vers if for_board(vers[v]["board"], board) and phase_le(vers[v]["phase"], p)
                    and vers[v]["last_result"] not in ("CUT",) + GONE and v not in db["advisory_only"]
                    and not ver_ok(v)[0]]
            bad = [v for v in mine if not ver_ok(v, honour=True)[0]]
            acc = [f"{v} ({accepted[v]})" for v in mine if v not in bad]
            msg = (f"{len(bad)} checks at or before {p} not passing: " + " ".join(bad[:8])
                   + (" ..." if len(bad) > 8 else "")) if bad else ""
            if acc:
                msg = (msg + "; " if msg else "") + f"accepted: {len(acc)} ACCEPTED: " + "; ".join(acc)
            return (not bad), msg
        if tok == "DEFECTS:CLOSED":
            bad = [d for d in defs if for_board(defs[d]["board"], board) and defs[d]["status"] not in ("CLOSED",) + GONE
                   and not def_moved(db, d, phase)]
            return (not bad), (f"not CLOSED: {' '.join(bad[:8])}" + (" ..." if len(bad) > 8 else "")) if bad else ""
        if tok == "DFMEA:COVERAGE":
            if BASELINE_BOARD != board:
                return False, (f"DFMEA:COVERAGE reads design.py, which is the {BASELINE_BOARD} board's: no {board} "
                               "design.py exists to cover yet")
            _, miss = coverage(db)
            return (not miss), (f"{len(miss)} parts in no DFMEA row: " + " ".join(miss[:10])) if miss else ""
        if tok.startswith("VER-"):
            if tok in vers and not for_board(vers[tok]["board"], board):
                return None, f"{tok} is a {vers[tok]['board']} check"
            if tok in vers and vers[tok]["last_result"] == "CUT":
                return False, f"{tok} CUT ({vers[tok]['evidence'][:60]}): waive the criterion or replace the check"
            return ver_ok(tok, honour=True)
        if tok.startswith("REQ-"):
            r = reqs.get(tok)
            if r is None:
                return False, f"{tok} undefined"
            if eff_req_status(db, tok, phase) != "APPROVED":
                return False, f"{tok} {eff_req_status(db, tok, phase)}"
            target = reqs.get(r["alloc"].get(board), r)
            bad = [v for v in target["vers"] if v in vers and for_board(vers[v]["board"], board)
                   and phase_le(vers[v]["phase"], phase) and not ver_ok(v)[0]]
            return (not bad), (f"{tok} checks not passing: {' '.join(bad)}" if bad else "")
        if tok.startswith("DF-"):
            d = dfs.get(tok)
            return (False, f"{tok} undefined") if d is None else df_ok(d)
        if tok.startswith("DEF-"):
            d = defs.get(tok)
            if d is None:
                return False, f"{tok} undefined"
            return d["status"] == "CLOSED", (f"{tok} {d['status']}" if d["status"] != "CLOSED" else "")
        return False, f"{tok}: cannot evaluate"

    results = []                                    # (id, state PASS FAIL N/A INFO, reasons)
    prefix = GATE_PREFIX[phase]
    for cid, c in sorted(crit.items()):
        if c["phase"] != phase:
            continue
        if c["retired"]:                            # before the board: prod_board's keep their column (574c94c)
            results.append((cid, c["needs"][0], [c["text"][:90]]))
            continue
        if not for_board(c["board"], board):
            results.append((cid, "N/A", [f"a {c['board']} criterion"]))
            continue
        checked = [need_ok(t) for t in c["needs"]]
        reasons = [r for ok, r in checked if ok is False]
        notes = [r for ok, r in checked if ok is not False and r]      # advisory, N/A and accepted notes
        if all(ok is None for ok, _ in checked):
            reasons = [f"nothing here applies to {board}: give it its own check ("
                       + "; ".join(r for _, r in checked if r) + ")"]
        if c["advisory"]:
            results.append((cid, "INFO", reasons or ["met"]))
        elif reasons and cid in waived:
            w, a = waived[cid]
            results.append((cid, "PASS", [f"waived by {a['who']} {a['date']} ({w['id']}, {a['id']})"]))
        else:
            results.append((cid, "FAIL" if reasons else "PASS", reasons + notes))

    mine = req_scope(db, board)
    prop = [k for k in mine if eff_req_status(db, k, phase) == "PROPOSED"]
    ended = [k for k in prop if eff_req_status(db, k) == "DEFERRED"]
    rs = []
    if prop:
        rs.append(f"{len(prop)} PROPOSED: " + " ".join(prop[:10]) + (" ..." if len(prop) > 10 else ""))
    if ended:
        rs.append("deferral ended before " + phase + ": " + " ".join(
            f"{k} ({db['eff'][k]['id']}, to {db['eff'][k]['scope']})" for k in ended[:8]))
    results.append(("R1 requirements approved", "FAIL" if prop else "PASS", rs))
    nover, notpass = [], []
    for k, r in mine.items():
        if eff_req_status(db, k, phase) in ("REJECTED", "DEFERRED") or r["alloc"].get(board):
            continue                                   # a split requirement is checked through its child
        vs = [v for v in r["vers"] if v in vers and for_board(vers[v]["board"], board)]
        if not vs:
            nover.append(k)
            continue
        if any(phase_le(vers[v]["phase"], phase) and not ver_ok(v)[0] for v in vs):
            notpass.append(k)
    rs = []
    if nover:
        rs.append(f"{len(nover)} with no check for {board}: " + " ".join(nover[:10]) + (" ..." if len(nover) > 10 else ""))
    if notpass:
        rs.append(f"{len(notpass)} with a check at or before {phase} not passing: " + " ".join(notpass[:10])
                  + (" ..." if len(notpass) > 10 else ""))
    results.append(("R2 requirements verified", "FAIL" if rs else "PASS", rs))
    bad = [d["id"] for d in df_scope(db, board) if d["action_priority"] == "H" and not df_ok(d)[0]]
    ended = [f"{d} accepted only up to {eff_df_accept(db, d)['scope']} ({eff_df_accept(db, d)['id']})"
             for d in bad if eff_df_accept(db, d)]
    results.append(("R3 high-priority risks covered", "FAIL" if bad else "PASS",
                    ([f"{len(bad)} H rows neither ACCEPTED for {phase} nor DONE with every 'detects' check passing: "
                      + " ".join(bad[:12]) + (" ..." if len(bad) > 12 else "")] + (["acceptance ended: " + "; ".join(ended)]
                                                                              if ended else [])) if bad else []))
    # O-R5: a defect blocks the gate it names and every later one, not an earlier one
    mydefs = [d for d in defs if for_board(defs[d]["board"], board) and defs[d]["gate"] in PHASES
              and phase_le(defs[d]["gate"], phase)]
    # 2026-10-28: an ACCEPTED defect with Alex's signed DF-ACCEPT line is listed, not failed, at the gates before the
    # one it moved to; at that gate and after it, it counts as an OPEN one would (an unsigned one is a structure error)
    moved = {d: def_moved(db, d, phase) for d in mydefs if defs[d]["status"] == "ACCEPTED"}
    acc_due = [d for d, a in moved.items() if not a]
    acc_note = ([f"accepted: ACCEPTED, moved past this gate by a signed line (listed, not failed): "
                 + "; ".join(f"{d} to {a['scope']} ({a['id']})" for d, a in moved.items() if a)]
                if any(moved.values()) else [])
    if phase in ("ORDER", "CASE", "EVT"):
        bad = [d for d in mydefs if defs[d]["status"] == "OPEN"]
        # F-R1 (3): the FIXED ones due by this gate are listed beside it, not failed: their closing checks are
        # mostly EVT unit checks, and one that still holds the gate must say OPEN (a structure rule above)
        fixed = [d for d in mydefs if defs[d]["status"] == "FIXED"]
        results.append(("R4 no open defects due by this gate", "FAIL" if bad or acc_due else "PASS",
                        ([f"OPEN (Gate {phase} or earlier): {' '.join(bad)}"] if bad else [])
                        + ([f"ACCEPTED only until this gate: {' '.join(acc_due)}"] if acc_due else [])
                        + ([f"FIXED, closing check not yet passed (listed, not failed): {' '.join(fixed)}"]
                           if fixed else []) + acc_note))
    else:
        bad = [d for d in mydefs if defs[d]["status"] not in ("CLOSED",) + GONE and not moved.get(d)]
        results.append(("R4 every defect due by this gate closed", "FAIL" if bad else "PASS",
                        ([f"not CLOSED: {' '.join(bad)}"] if bad else []) + acc_note))
    dirty_hits = sorted({f for v in vers.values() if v["last_result"] == "PASS"
                         for k, s, _ in v["cov"] if k == "glob" for f in dirty if s[0].match(f)})
    rel = vv_rel()
    # O-R2: the register itself and its evidence must be committed, or a typed PASS opens the gate
    vv_dirty = git("status", "--porcelain", "--", rel)[1].splitlines() if rel else []
    rs5 = ([" ".join(dirty_hits[:6])] if dirty_hits else []) + (
        [f"uncommitted register changes: " + " ".join(x[3:] for x in vv_dirty[:6])
         + (f" (+{len(vv_dirty) - 6})" if len(vv_dirty) > 6 else "")] if vv_dirty else [])
    results.append(("R5 nothing uncommitted under passed evidence or in the register", "FAIL" if rs5 else "PASS", rs5))
    # O-R5: the gate state Alex froze with a signed GATE@ line may not move later without a newer signed line
    frozen, aline = gate_freeze_violations(db, phase)
    if aline:
        results.append(("G1 the gate state Alex froze", "FAIL" if frozen else "PASS",
                        ([f"moved after {aline['id']} ({aline['item']}) without a newer signed line: "
                          + "; ".join(frozen[:8])] if frozen else
                         [f"frozen at {aline['item']} by {aline['id']} ({aline['who']} {aline['date']})"])))
    else:
        moves, since = unfrozen_moves(db)
        if since:
            results.append(("G0 moves later since the reviewed register state (O-S5; not frozen yet)", "INFO",
                            ([f"{len(moves)} since {since[:7]}: " + "; ".join(moves)] if moves else
                             [f"none since {since[:7]}"]) + ["read these before signing GATE@ (README, Approvals)"]))
    # O-R12: the key that counts must be a hardware key; a software key is Alex's explicit choice
    pin, pin_bad = pinned_key()
    kt = allowed_key_type(pin) if (pin and not pin_bad) else None
    if phase == "ORDER" and kt:
        if kt in HARD_KEY_TYPES:
            results.append(("K1 the pinned signing key is a hardware key", "PASS", [f"{pin} is {kt}"]))
        elif accept_sw:
            results.append(("K1 the pinned signing key is a hardware key", "PASS",
                            [f"--accept-software-key used: {pin} is a software key ({kt}); copy this line into the "
                             f"release record's signing section"]))
        else:
            results.append(("K1 the pinned signing key is a hardware key", "FAIL",
                            [f"{pin} is a software key ({kt}): any process running as alex can sign with it. "
                             f"Pin a hardware key (README, 'Approvals'; DEC-30) or run the gate with "
                             f"--accept-software-key and record that in the release record"]))
    if board in BOARDS:
        rels = [r for r in db["recs"].values() if r["kind"] == "REL" and r["board"] in (board, "ref_design")
                and r["commit"] and resolve(r["commit"]) and is_ancestor(resolve(r["commit"]))]
        unrec = []
        if rels:
            newest = min(rels, key=lambda r: int(git("rev-list", "--count", f"{resolve(r['commit'])}..HEAD")[1] or 0))
            shas = git("log", "--format=%H", f"{resolve(newest['commit'])}..HEAD", "--", *DESIGN_PATHS)[1].split()
            named = [x for ch in db["recs"].values() if ch["kind"] == "CHG" for x in ch["commits"]]
            unrec = [s for s in shas if not any(s.startswith(x) for x in named)]
        results.append(("R6 every design change since the last release has a change record",
                         "FAIL" if unrec else "PASS",
                         [f"unrecorded design change {' '.join(s[:7] for s in unrec[:8])}"] if unrec else
                         ([] if rels else ["no release record yet: nothing to compare against"])))
        which = None if phase not in BASELINE_FOR else BASELINE_FOR[phase]
        head_drift, base_drift = baseline_diff(db, which) if db["baseline"] else (["README.md has no design baseline"], [])
        bd = head_drift + [f"{p} (not at {base7(db)})" for p in base_drift]
        if BASELINE_BOARD != board:
            # 2026-10-28: the baseline's files are the dropped board's; the register does not describe them
            bd.append(f"the baseline is the {BASELINE_BOARD} board's files (README '- Baseline board:'), not {board}'s")
        results.append(("R7 the register describes this design", "FAIL" if bd else "PASS",
                        ["gate evaluated against a design it does not describe: " + " ".join(bd)] if bd else []))
        if phase in ("ORDER", "CASE"):
            # as R7: the order gate reads the board's files, the case gate the case's (a case-only commit on the
            # order branch does not hold the board order back, and the case release must re-check it)
            what = "board" if phase == "ORDER" else "case"
            bdr = branch_drift(db, ["hardware/ref_design/" + p for p in BASELINE_FOR[phase]])
            other = branch_drift(db)
            note = [] if bdr or not other else [f"not counted here (not {what} files): " + "; ".join(other)]
            if phase == "ORDER":
                shifted, same = case_position_drift(db)
                bdr += shifted + proposal_drift(db)       # an architecture proposal holds the order (2026-10-28)
                note += same
            results.append((f"R8 the order branch has not moved past the baseline ({what} files"
                            + (", architecture proposals" if phase == "ORDER" else "") + ")",
                            "FAIL" if bdr else "PASS", bdr + note))
        if phase == "ORDER":
            # 2026-10-27: classes 1-14 and 15's paper estimate are covered for every subsystem before the order;
            # a cell with only NOT_BUILT checks is planned, still a gap; an accepted gap counts only when signed
            signed_, accepted_ = grid_signed(db, phase), accepted_checks(db, phase)
            vers_, reasons_, grid_, _ = cgrid.build(VV, signed_, accepted_)
            held = sorted(cgrid.order_gaps(grid_), key=lambda x: x[2]["state"] != "GAP")   # no check first
            stale = []
            path = os.path.join(VV, "grid.md")
            if not cgrid.same(open(path, encoding="utf-8").read() if os.path.exists(path) else "",
                              cgrid.render(vers_, reasons_, grid_, accepted_, signed_)):
                stale = ["grid.md is not what the register gives: run python3 hardware/vv/tools/grid.py"]
            # step 9.5: an accepted gap is a grid_reasons.csv ACCEPTED_GAP row and Alex's signed GRID line, both
            stale += [f"{item} is signed but grid_reasons.csv has no ACCEPTED_GAP row for it: the cell stays a GAP"
                      for item in cgrid.signed_without_row(reasons_, signed_)]
            rs9 = ([f"{len(held)} cells of classes 1-15 are GAP ("
                    + "; ".join(f"{c} {s}" + (f" planned: {' '.join(v['id'] for v in cell['planned'])}"
                                             if cell["state"] == "planned" else " no check")
                                for c, s, cell in held[:12]) + (" ..." if len(held) > 12 else "") + ")"]
                   if held else [])
            results.append(("R9 the coverage grid has no GAP in classes 1-15 (grid.md)",
                            "FAIL" if held or stale else "PASS", rs9 + stale))

    print(f"gate {phase} for {board} at {HEAD[:7]}")
    # printed even on a PASS (O-R4, O-R12; an acceptance, 2026-10-28)
    always = ("waived by", "case positions:", "--accept-software-key used", "accepted:")
    for cid, state, reasons in results:
        print(f"  {state:4}  {cid}" + (f"  {crit[cid]['text'][:70]}" if cid in crit and state not in GONE else ""))
        for r in reasons if (verbose or state in ("FAIL", "INFO") + GONE) else [x for x in reasons if x.startswith(always)]:
            print(f"          {r}")
    counted = [s for _, s, _ in results if s in ("PASS", "FAIL")]
    n_bad = counted.count("FAIL")
    print(f"RESULT: {'PASS' if not n_bad else f'FAIL ({n_bad} of {len(counted)} not met)'}"
          + (f"; {sum(1 for _, s, _ in results if s == 'N/A')} N/A, "
             f"{sum(1 for _, s, _ in results if s == 'INFO')} advisory, "
             f"{sum(1 for _, s, _ in results if s == 'RETIRED')} retired, "
             f"{sum(1 for _, s, _ in results if s == 'DROPPED')} dropped" if len(counted) < len(results) else ""))
    return n_bad == 0


def main():
    global VV
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--gate", choices=sorted(GATE_ARG), help="evaluate a phase gate (exit 1 unless met)")
    ap.add_argument("--shopping", action="store_true", help="list what unrun checks still need, by lead time")
    ap.add_argument("--accept-software-key", action="store_true",
                    help="let the order gate pass with a software signing key (O-R12): the choice is printed and "
                         "must be copied into the release record's signing section")
    ap.add_argument("--verbose", action="store_true")
    ap.add_argument("--vv", help=argparse.SUPPRESS)        # another copy of hardware/vv (seed_faults.py)
    a = ap.parse_args()
    a.board = BOARDS[0]                                    # the one board: the RP2350B board since 2026-10-29
    if a.vv:
        VV = os.path.abspath(a.vv)
    db = load()
    if ERRORS:
        print(f"STRUCTURE: {len(ERRORS)} error(s)")
        for e in ERRORS:
            print("  " + e)
        sys.exit(1)
    print("STRUCTURE: OK" + (f" ({len(WARNINGS)} warnings)" if WARNINGS else ""))
    if a.shopping:
        shopping(db, GATE_ARG[a.gate] if a.gate else "ORDER", a.board)
        sys.exit(0)
    if a.gate:
        sys.exit(0 if gate(db, GATE_ARG[a.gate], a.board, a.verbose, a.accept_software_key) else 1)
    report(db, a.verbose)
    sys.exit(0)


if __name__ == "__main__":
    main()
