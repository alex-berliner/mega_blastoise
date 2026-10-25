#!/usr/bin/env python3
"""Reads crates/ps_harness/coverage-allow.txt and resolves its line entries
against the source (the format is documented at the top of that file).

scripts/ps-coverage.sh imports this; on its own it checks the file:

    scripts/coverage-allow.py --check     # every entry parses and its text
                                          # is in the tree (exit 1 if not)
    scripts/coverage-allow.py --show      # each entry with the lines it
                                          # resolves to now
    scripts/coverage-allow.py --selftest  # the parser and resolver on a
                                          # made-up file (--check runs it
                                          # first)
"""
import os
import re
import sys
from collections import namedtuple

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ALLOW_FILE = os.path.join(ROOT, 'crates/ps_harness/coverage-allow.txt')
SRC = os.path.join(ROOT, 'crates/ps_sim/src')

# One step of a selector: `op` is '' (the first anchor), '>' (the first
# matching line after the previous one, which becomes the start) or '..'
# (the first matching line after the previous one, which becomes the end;
# only more '..' steps may follow).
Step = namedtuple('Step', 'op text index hint')
Handler = namedtuple('Handler', 'gen effect key reason lineno')
Entry = namedtuple('Entry', 'path steps reason lineno raw')

_TOKEN = re.compile(r'\s*(>|\.\.)?\s*`([^`]*)`(?:#(\d+))?(?:@(\d+))?')


def parse(path=ALLOW_FILE):
    """Returns (handlers, entries, errors); errors are strings."""
    handlers, entries, errors = [], [], []
    for n, raw in enumerate(open(path), 1):
        s = raw.strip()
        if not s or s.startswith('#'):
            continue
        m = re.match(r'handler\s+(\d+)\s+(\S+)\s+(\S+)\s+(.*)', s)
        if m:
            handlers.append(Handler(int(m.group(1)), m.group(2), m.group(3), m.group(4), n))
            continue
        m = re.match(r'(\S+)(?=\s+`)', s)
        if not m:
            errors.append(f'coverage-allow.txt:{n}: bad line: {s}')
            continue
        p, rest, steps = m.group(1), s[m.end():], []
        while True:
            t = _TOKEN.match(rest)
            if not t:
                break
            op = t.group(1) or ''
            if (op == '') != (not steps) or (op == '>' and steps[-1].op == '..'):
                errors.append(f'coverage-allow.txt:{n}: bad selector: {s}')
                steps = None
                break
            if op and (t.group(3) or t.group(4)):
                errors.append(f'coverage-allow.txt:{n}: #index and @line go on the first anchor only: {s}')
                steps = None
                break
            if not t.group(2).strip() or t.group(2) != t.group(2).strip():
                errors.append(f'coverage-allow.txt:{n}: anchor text must be the trimmed, non-empty line: {s}')
                steps = None
                break
            steps.append(Step(op, t.group(2), int(t.group(3)) if t.group(3) else None,
                              int(t.group(4)) if t.group(4) else None))
            rest = rest[t.end():]
        if steps is None:
            continue
        if not rest[:1].isspace() or not rest.strip():
            errors.append(f'coverage-allow.txt:{n}: needs a reason after the selector: {s}')
            continue
        entries.append(Entry(p, steps, rest.strip(), n, s))
    return handlers, entries, errors


_lines_cache = {}


def source_lines(path):
    """The trimmed lines of crates/ps_sim/src/<path>, or None."""
    if path not in _lines_cache:
        f = os.path.join(SRC, path)
        _lines_cache[path] = [l.strip() for l in open(f)] if os.path.isfile(f) else None
    return _lines_cache[path]


def resolve(entry):
    """Returns ((first, last), None) with 1-based lines, or (None, why).
    `why` starting with 'ambiguous' is an error in the file; anything else
    means the entry is stale."""
    lines = source_lines(entry.path)
    if lines is None:
        return None, 'no such file'
    first = entry.steps[0]
    hits = [i for i, l in enumerate(lines) if l == first.text]
    if not hits:
        return None, f'text not found: `{first.text}`'
    if first.index is not None:
        if first.index < 1 or first.index > len(hits):
            return None, f'`{first.text}` occurs {len(hits)} times, not #{first.index}'
        cur = hits[first.index - 1]
    elif len(hits) == 1:
        cur = hits[0]
    elif first.hint is not None:
        cur = min(hits, key=lambda i: (abs(i + 1 - first.hint), i))
    else:
        return None, (f'ambiguous: `{first.text}` occurs {len(hits)} times (lines '
                      + ','.join(str(i + 1) for i in hits) + '); add #<n>, @<line> or a `>` chain')
    start = cur
    for st in entry.steps[1:]:
        nxt = next((i for i in range(cur + 1, len(lines)) if lines[i] == st.text), None)
        if nxt is None:
            return None, f'text not found after line {cur + 1}: `{st.text}`'
        cur = nxt
        if st.op == '>':
            start = cur
    return (start + 1, cur + 1), None


def selftest():
    import tempfile
    src = ['fn a() {', '    if x {', '        y();', '    }', '}', 'fn b() {', '    if x {',
           '        y();', '    }', '    z();', '}']
    _lines_cache['t.rs'] = [l.strip() for l in src]
    cases = [
        ('`fn a() {`', (1, 1)),
        ('`y();`', 'ambiguous'),
        ('`y();`#2', (8, 8)),
        ('`y();`#3', 'occurs 2 times'),
        ('`y();`@9', (8, 8)),
        ('`y();`@1', (3, 3)),
        ('`fn b() {` > `}`', (9, 9)),
        ('`fn b() {` > `}` > `}`', (11, 11)),
        ('`fn a() {` .. `}`', (1, 4)),
        ('`fn a() {` .. `y();` .. `}`', (1, 4)),
        ('`fn b() {` > `if x {` .. `z();` .. `}`', (7, 11)),
        ('`gone();`', 'text not found'),
        ('`fn b() {` > `gone();`', 'text not found after line 6'),
    ]
    bad = []
    with tempfile.NamedTemporaryFile('w', suffix='.txt', delete=False) as f:
        for sel, _ in cases:
            f.write(f't.rs  {sel}  reason\n')
        f.write('handler 1 move:dig onPrepareHit  reason\n')
        for wrong in ['t.rs  `y();`', 't.rs  `fn a() {` .. `}` > `}`  r', 't.rs  `a` > `b`#2  r',
                      't.rs  ` y();`  r', 't.rs:12  reason']:
            f.write(wrong + '\n')
        name = f.name
    try:
        handlers, entries, errors = parse(name)
    finally:
        os.unlink(name)
    if len(handlers) != 1 or handlers[0].key != 'onPrepareHit':
        bad.append(f'handlers: {handlers}')
    if len(errors) != 5:
        bad.append(f'expected 5 errors, got {errors}')
    for e, (sel, want) in zip(entries, cases):
        r, why = resolve(e)
        if (r != want) if isinstance(want, tuple) else (r is not None or want not in why):
            bad.append(f'{sel}: got {r} {why}, want {want}')
    if len(entries) != len(cases):
        bad.append(f'{len(entries)} entries for {len(cases)} cases')
    del _lines_cache['t.rs']
    for msg in bad:
        print('coverage-allow selftest:', msg)
    return not bad


def main(argv):
    mode = argv[1] if len(argv) > 1 else '--check'
    if mode in ('--selftest', '--check') and not selftest():
        return 1
    if mode == '--selftest':
        print('coverage-allow selftest: ok')
        return 0
    handlers, entries, errors = parse()
    bad = list(errors)
    for e in entries:
        r, why = resolve(e)
        if r is None:
            bad.append(f'coverage-allow.txt:{e.lineno}: {e.path}: {why}')
        elif mode == '--show':
            a, b = r
            print(f'{e.path}:{a}' + (f'-{b}' if b != a else '') + f'  <- line {e.lineno}')
    for msg in bad:
        print(msg)
    if mode == '--check':
        print(f'coverage-allow: {len(handlers)} handler entries, {len(entries)} line entries, '
              f'{len(bad)} problems')
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
