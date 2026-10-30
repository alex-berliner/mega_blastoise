#!/usr/bin/env python3
"""Check every DESIGN.md:<line> cite in the register's live rows against docs/static/DESIGN.md.

    python3 hardware/vv/tools/design_cites.py              # exit 1 on a cite whose text is not on its line
    python3 hardware/vv/tools/design_cites.py --vv <copy>  # another copy of hardware/vv
    python3 hardware/vv/tools/design_cites.py --verbose    # also list the cites that matched

DESIGN.md gains lines (2026-10-30: the power-off line, the case's version), and a cite written
against an older copy then points at a neighbour. For each cite in requirements.md,
verification.csv, dfmea.csv, defects.md, gates.md, PLAN.md and README.md the tool reads the
text that goes with it (the quote in quotes after it, else the words around it up to the next
cite) and looks for that text on the cited line:

  ok         a quote of 12 or more characters is on the cited line;
  ok-words   no quote, but 2 or more content words of the cite's own clause are on the line
             (a paraphrase);
  WRONG      a quote is not on the cited line (the tool names the line that has it, if any);
  WRONG      no quote, no word of the clause on the cited line, and another line has 2 or more;
  review     no quote and fewer than 2 words match: listed for a person, not an error.

Rows of dropped boards (a Board or Status naming DROPPED, RETIRED, esp32s3, f1c200s or
rp2354b) are skipped: their cites refer to older versions of the file. analysis/ holds dated
snapshots and is not read. A cite inside a signed Statement is reported by the person who
fixes it (the tool cannot see signatures); Statements are not edited here.
Python 3 standard library only.
"""
import csv
import io
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DEAD = re.compile(r"DROPPED|RETIRED|esp32s3|f1c200s|rp2354b")
CITE = re.compile(r"DESIGN\.md:(\d+)")
STOP = set("with that this from have been will which their there when then into only than also "
           "each shall ones none line power seat's the and for are not its has was can may".split())


def norm(s):
    return re.sub(r"[^a-z0-9]+", " ", s.lower()).strip()


def words(s):
    return {w for w in norm(s).split() if len(w) >= 4 and w not in STOP}


def clause(text, m, others):
    """The quote after the cite, else the clause around it (to the neighbouring cites)."""
    after = text[m.end():m.end() + 400]
    q = re.match(r"\s*\(\s*'(.{12,}?)'\s*[),]", after) or re.match(r"\s*\(\s*\"(.{12,}?)\"\s*[),]", after)
    if q:
        return q.group(1), None
    nxt = [o.start() for o in others if o.start() > m.start()]
    end = min(nxt + [m.end() + 70])
    tail = text[m.end():end].split(";")[0]
    prev = [o.end() for o in others if o.end() <= m.start()]
    head = text[max(prev + [m.start() - 60]):m.start()].split(";")[-1]
    return None, head + " " + tail


def near_line(para, design):
    return max(range(len(design)), key=lambda i: len(words(para) & words(design[i]))) + 1


def blocks(f, lines):
    """(line number, text) of each live line; requirements.md and defects.md go by block."""
    if f in ("requirements.md", "defects.md"):
        out, i = [], 0
        while i < len(lines):
            if lines[i].startswith("### "):
                j = i + 1
                while j < len(lines) and not lines[j].startswith("### "):
                    j += 1
                dead = any(re.match(r"- (Status|Board):.*(%s)" % DEAD.pattern, l) for l in lines[i:j])
                if not dead:
                    out += [(n + 1, lines[n]) for n in range(i, j)]
                i = j
            else:
                out.append((i + 1, lines[i]))
                i += 1
        return out
    if f in ("verification.csv", "dfmea.csv"):
        rows = list(csv.reader(io.StringIO("\n".join(lines))))
        out, n = [], 0
        for r in rows[1:]:
            if not r:
                continue
            cur = n = _find_row(lines, r[0], n)
            board = r[1] if f == "dfmea.csv" else r[3]
            state = r[-1] if f == "dfmea.csv" else r[7]
            dead = board in ("esp32s3", "f1c200s", "rp2354b") or state.startswith(("DROPPED", "RETIRED"))
            if dead:
                continue
            out += [(k + 1, lines[k]) for k in range(cur, _row_end(lines, cur))]
        return out
    if f == "gates.md":
        return [(n + 1, l) for n, l in enumerate(lines) if not (l.startswith("| ") and DEAD.search(l))]
    return [(n + 1, l) for n, l in enumerate(lines)]


def _find_row(lines, rid, start):
    for k in range(start, len(lines)):
        if lines[k].startswith(rid + ","):
            return k
    return start


def _row_end(lines, k):
    j = k + 1
    while j < len(lines) and not re.match(r"[A-Z]+-\d+,", lines[j]):
        j += 1
    return j


def main():
    vv = os.path.dirname(HERE)
    if "--vv" in sys.argv:
        vv = os.path.abspath(sys.argv[sys.argv.index("--vv") + 1])
    verbose = "--verbose" in sys.argv
    design = open(os.path.join(vv, "..", "..", "docs", "static", "DESIGN.md")).read().split("\n")
    dn = [norm(l) for l in design]
    wrong = review = ok = 0
    for f in ("requirements.md", "verification.csv", "dfmea.csv", "defects.md", "gates.md", "PLAN.md", "README.md"):
        lines = open(os.path.join(vv, f)).read().split("\n")
        for n, text in blocks(f, lines):
            ms = list(CITE.finditer(text))
            for m in ms:
                k = int(m.group(1))
                where = f"{f}:{n}"
                if k > len(design):
                    print(f"WRONG   {where}: DESIGN.md:{k} is past the end ({len(design)} lines)")
                    wrong += 1
                    continue
                quote, para = clause(text, m, ms)
                if quote:
                    q = norm(quote)
                    if q in dn[k - 1]:
                        ok += 1
                        if verbose:
                            print(f"ok      {where}: DESIGN.md:{k}")
                    else:
                        have = [i + 1 for i, d in enumerate(dn) if q in d]
                        print(f"WRONG   {where}: DESIGN.md:{k} does not hold '{quote[:60]}'"
                              + (f" (line {have[0]} does)" if have else " (no line does)"))
                        wrong += 1
                else:
                    common = words(para) & words(design[k - 1])
                    if len(common) >= 2:
                        ok += 1
                        if verbose:
                            print(f"ok-words {where}: DESIGN.md:{k} {sorted(common)}")
                    elif not common and len(words(para) & words(design[near_line(para, design) - 1])) >= 2:
                        best = near_line(para, design)
                        print(f"WRONG   {where}: DESIGN.md:{k} ('{design[k-1][:50]}') shares no word with "
                              f"'{para.strip()[:80]}'; line {best} does")
                        wrong += 1
                    else:
                        review += 1
                        near = max(range(len(design)), key=lambda i: len(words(para) & words(design[i])))
                        print(f"review  {where}: DESIGN.md:{k} = '{design[k-1][:70]}'; clause "
                              f"'{para.strip()[:90]}'; best line {near + 1}")
    print(f"design_cites: {ok} ok, {review} for a person, {wrong} wrong")
    return 1 if wrong else 0


if __name__ == "__main__":
    sys.exit(main())
