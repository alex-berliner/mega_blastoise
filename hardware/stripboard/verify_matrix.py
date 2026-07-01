#!/usr/bin/env python3
"""Electrical verification of the stripboard matrix layout.
Models vertical strips (per letter), track cuts (sever a strip at a hole),
jumpers (tie segments), and every switch leg. Computes nets by union-find and
asserts each button connects exactly its row GPIO <-> col GPIO, no stray shorts.
"""
VSPAN, HSPAN, COLPITCH = 0.3, 0.2, 0.4
def T(x, y): return (round(y - 0.3, 2), round(x - 2.0 + 0.5, 2))
def L(X): return int(round(X / 0.1))   # letter index
def H(Y): return int(round(Y / 0.1))   # hole index
ROWS = [('P1','M',4,'GP5',2.4),('P1','S',3,'GP7',1.8),
        ('P2','M',4,'GP8',1.2),('P2','S',3,'GP9',0.6)]
def node_of(by): return round(by - VSPAN, 2)
def xL(c): return round(2.4 + COLPITCH*c, 2)
def xgap(c): return round(2.4 + COLPITCH*c + 0.1, 2)
COLS = [(0,'GP10'),(1,'GP11'),(2,'GP12'),(3,'GP13')]

# collect geometry exactly as the generator emits it
legs = []   # (button_id, role, letter, hole)  role in {'row','col'}
cuts = []   # (letter, hole)
jumpers = []# ((letter,hole),(letter,hole))
headers = []# (gpio, letter, hole)

for (p,k,nc,gp,by) in ROWS:
    ny = node_of(by)
    for c in range(nc):
        x = xL(c)
        bid = f'{p}{k}{c+1}'
        # all 4 physical legs: terminal-1 (row) both at y=by; terminal-2 (col) both at y=ny
        for lx in (x, round(x+HSPAN,2)):
            X,Y = T(lx, by);  legs.append((bid,'row',L(X),H(Y)))
            X,Y = T(lx, ny);  legs.append((bid,'col',L(X),H(Y)))
for (p,k,nc,gp,by) in ROWS:
    ny = node_of(by)
    for c in range(nc-1):
        X,Y = T(round(xL(c)+HSPAN+0.1,2), ny); cuts.append((L(X),H(Y)))
for (c,gp) in COLS:
    nodes = sorted(node_of(by) for (p,k,nc,g,by) in ROWS if c<nc)
    x = xgap(c)
    for a,b in zip(nodes[:-1], nodes[1:]):
        A=T(x,a); B=T(x,b); jumpers.append(((L(A[0]),H(A[1])),(L(B[0]),H(B[1]))))
for (p,k,nc,gp,by) in ROWS:
    X,Y = T(2.1, by); headers.append((gp,L(X),H(Y)))
for (c,gp) in COLS:
    nodes = sorted(node_of(by) for (p,k,nc,g,by) in ROWS if c<nc)
    X,Y = T(xgap(c), nodes[0]); headers.append((gp,L(X),H(Y)))

# segment id for a point on a letter strip: how many cuts on that letter are below it
cuts_by_letter = {}
for (lt,h) in cuts: cuts_by_letter.setdefault(lt,[]).append(h)
def seg(lt,h):
    return (lt, sum(1 for ch in cuts_by_letter.get(lt,[]) if ch < h))

# union-find over segments; jumpers merge segments
parent = {}
def find(s):
    parent.setdefault(s,s)
    while parent[s]!=s: parent[s]=parent[parent[s]]; s=parent[s]
    return s
def union(a,b): parent[find(a)] = find(b)
for (a,b) in jumpers: union(seg(*a), seg(*b))

# name each net by the header pad it contains
net_name = {}
for (gp,lt,h) in headers:
    net_name[find(seg(lt,h))] = gp

# expected matrix: button -> (rowGPIO, colGPIO)
expect = {}
for (p,k,nc,gp,by) in ROWS:
    for c in range(nc):
        expect[f'{p}{k}{c+1}_{gp}'] = None  # placeholder; keyed differently below
# build per-button expectation
btn_expect = {}
for (p,k,nc,gp,by) in ROWS:
    for c in range(nc):
        btn_expect[(p,k,c+1,gp)] = (gp, COLS[c][1])

ok = True
def check(cond, msg):
    global ok
    print(('  OK  ' if cond else ' FAIL ')+msg); ok = ok and cond

# group legs by button
from collections import defaultdict
bl = defaultdict(list)
for (bid,role,lt,h) in legs: bl[bid].append((role,lt,h))

print('=== per-button net check ===')
for (p,k,nc,gp,by) in ROWS:
    for c in range(nc):
        bid = f'{p}{k}{c+1}'
        rows_nets = {net_name.get(find(seg(lt,h)),'?') for (role,lt,h) in bl[bid] if role=='row'}
        col_nets  = {net_name.get(find(seg(lt,h)),'?') for (role,lt,h) in bl[bid] if role=='col'}
        exp_r, exp_c = gp, COLS[c][1]
        check(rows_nets=={exp_r}, f'{bid}: row legs -> {rows_nets} (want {{{exp_r}}})')
        check(col_nets=={exp_c},  f'{bid}: col legs -> {col_nets} (want {{{exp_c}}})')

print('=== no-stray-short check (each net = exactly its intended pads) ===')
# every leg/header assigned to a net root; count how many DISTINCT gpio names per root
root_names = defaultdict(set)
for (bid,role,lt,h) in legs: root_names[find(seg(lt,h))]  # touch
for (gp,lt,h) in headers: root_names[find(seg(lt,h))].add(gp)
# a root should map to exactly one header gpio (the net). Legs on it must match that gpio's role membership -- already checked above.
named = [r for r in root_names if root_names[r]]
check(all(len(root_names[r])==1 for r in named), f'{len(named)} named nets, each has exactly one header GPIO')
check(len(named)==8, f'exactly 8 nets present (found {len(named)})')

print('\nRESULT:', 'ALL CHECKS PASSED' if ok else 'FAILURES ABOVE')
