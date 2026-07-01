#!/usr/bin/env python3
"""Single-board stripboard layout, rev 3 - face-to-face two-player console.

Board: strips vertical, cols A..X (0..23), holes 1..55. Players sit at the two
short ends, facing each other.

rev 3 changes:
  1. move buttons sit at the four CORNERS of each player's screen (2 above,
     2 below), matching where the moves render on the OLED; the 3 party
     (switch) buttons share one row
  2. P2's whole module is P1's rotated 180 deg (P2 sits across the board):
     board top->bottom = [S3 S2 S1] [M4 M3] [screen 180deg] [M2 M1]  (P2)
                         [M1 M2] [screen] [M3 M4] [S1 S2 S3]         (P1)
     NOTE: firmware must render P2's OLED rotated 180 deg.
  3. LED strip connectors moved to the left edge (col A): P2's at the top,
     P1's at the bottom - out of the play area
  Pico on TOP side at the bottom (USB left), MB102 5V in bottom-right,
  rails GND=B / 3V3=W / 5V=X, all jumpers routed on the UNDERSIDE.

Layout is hand-routed; verify() (union-find over strips+cuts+jumpers+pads,
plus a no-two-wires-per-hole check) must PASS before the .diy is written.
"""

PICO = {1:'GP0',2:'GP1',3:'GND',4:'GP2',5:'GP3',6:'GP4',7:'GP5',8:'GND',9:'GP6',
        10:'GP7',11:'GP8',12:'GP9',13:'GND',14:'GP10',15:'GP11',16:'GP12',17:'GP13',
        18:'GND',19:'GP14',20:'GP15',21:'GP16',22:'GP17',23:'GND',24:'GP18',25:'GP19',
        26:'GP20',27:'GP21',28:'GND',29:'GP22',30:'RUN',31:'GP26',32:'GP27',33:'GND',
        34:'GP28',35:'VREF',36:'3V3',37:'3V3EN',38:'GND',39:'VSYS',40:'VBUS'}
USED_GP = {5,7,8,9,10,11,12,13,16,17,18,19,20,22}
GND_TAP_PIN = 38

NCOLS, NHOLES = 24, 55

PADS = []; CUTS = []; JUMPERS = []; SW_BODIES = []; MODULES = []; LABELS = []
def pad(name, net, c, h): PADS.append(dict(name=name, net=net, c=c, h=h))
def cut(c, h):            CUTS.append((c, h))
def jump(net, a, b):      JUMPERS.append(dict(net=net, a=a, b=b))
def label(c, h, t):       LABELS.append((c, h, t))

# ── Pico: TOP side, bottom of board, USB left. pins rows 47 (top) / 54 (bot) ──
PC0, PTOP, PBOT = 2, 47, 54
# face up, USB left: top row l->r = pins 40..21 ; bottom row l->r = pins 1..20
def pico_col(pin):
    return (42 - pin, PTOP) if pin >= 21 else (pin + 1, PBOT)
NET_STUB = {}
for pin, net in PICO.items():
    c, h = pico_col(pin)
    if   net == '3V3':  board = '3V3'
    elif net == 'GND':  board = 'GND' if pin == GND_TAP_PIN else f'x{pin}'
    elif net.startswith('GP') and int(net[2:]) in USED_GP: board = net
    else:               board = f'x{pin}'
    pad(f'pico{pin}', board, c, h)
    if board in ('3V3','GND') or board.startswith('GP'):
        NET_STUB[board] = (c, 48 if h == PTOP else 52)
for c in range(PC0, PC0 + 20):
    cut(c, 46); cut(c, 50)

# ── rails ──────────────────────────────────────────────────────────────────────
for net, c, hb in [('GND',1,55), ('3V3',22,46), ('5V',23,55)]:
    pad(f'rail_{net}_a', net, c, 2); pad(f'rail_{net}_b', net, c, hb)
label(0.4, 0.3, 'GND'); label(21.2, 0.3, '3V3 5V')

# ── buttons: (name, rowNet, colNet, nodeCol, busCol, topLegRow) ────────────────
BUTTONS = [
    # P2 (rotated 180): party row, then M4 M3, screen, M2 M1
    ('S3','GP9','GP12', 3, 6, 1), ('S2','GP9','GP11',10,13, 1), ('S1','GP9','GP10',17,20, 1),
    ('M4','GP8','GP13', 4, 7, 5), ('M3','GP8','GP12',15,18, 5),
    ('M2','GP8','GP11', 4, 7,20), ('M1','GP8','GP10',15,18,20),
    # P1: M1 M2, screen, M3 M4, party row
    ('M1','GP5','GP10', 4, 7,24), ('M2','GP5','GP11',15,18,24),
    ('M3','GP5','GP12', 4, 7,39), ('M4','GP5','GP13',15,18,39),
    ('S1','GP7','GP10', 3, 6,43), ('S2','GP7','GP11',10,13,43), ('S3','GP7','GP12',17,20,43),
]
for (nm, rn, cn, node, bus, hb) in BUTTONS:
    for hh in (hb, hb+2):
        pad(f'{rn}_{nm}_b{hh}', rn, bus, hh)
        pad(f'{cn}_{nm}_n{hh}', cn, node, hh)
    SW_BODIES.append((node, hb-0.5, bus, hb+2.5))
    label((node+bus)/2 - 0.35, hb+1, nm)
label(8.0, 0.3, 'PLAYER 2 (faces down)'); label(8.0, 46.0, 'PLAYER 1 (faces up)')

# ── OLED headers: 4 pads. P1 header on top edge; P2 rotated so order reverses ──
# P2 (row 18, header at BOTTOM of its screen): SDA SCL VCC GND
for c, net in [(9,'GP18'),(10,'GP19'),(11,'3V3'),(12,'GND')]:
    pad(f'oled2_{net}', net, c, 18)
# P1 (row 27, header at TOP of its screen): GND VCC SCL SDA
for c, net in [(9,'GND'),(10,'3V3'),(11,'GP17'),(12,'GP16')]:
    pad(f'oled1_{net}', net, c, 27)
MODULES.append(('OLED P2 (180deg)', 5.7, 7.5, 16.3, 18.4))
MODULES.append(('OLED P1',          5.7, 26.6, 16.3, 37.5))

# ── LED strip connectors on the left edge (col A): DIN / 5V / GND ─────────────
pad('led2_din','GP22',0,2); pad('led2_5v','5V',0,5); pad('led2_gnd','GND',0,8)
pad('led1_din','GP20',0,41); pad('led1_5v','5V',0,44); pad('led1_gnd','GND',0,47)
for h in (3,6,9,43,46): cut(0, h)
MODULES.append(('LED P2', -0.45, 1.4, 0.45, 8.6))
MODULES.append(('LED P1', -0.45, 40.4, 0.45, 47.6))
label(0.7, 5.0, '-> LED strip P2 (DIN/5V/GND)')
label(0.7, 44.0, '-> LED strip P1 (DIN/5V/GND)')

# ── MB102 5V input, bottom right: "-" left (W), "+" right (X) ─────────────────
cut(22, 52)
pad('mb102_minus','GND',22,53); pad('mb102_plus','5V',23,53)
jump('GND', (22,54), (1,54))
MODULES.append(('MB102 5V in', 21.4, 52.3, 23.6, 53.7))
label(17.8, 53.0, 'MB102: - +')

# ── track cuts (module borders + shared-strip splits) ─────────────────────────
for c in [3,4,6,7,9,10,11,12,13,15,17,18,20]: cut(c, 23)   # P2 / P1 border
for h in (13, 30): cut(4, h); cut(15, h)                    # split move-node strips
for h in (10, 35): cut(10, h)                               # S2 nodes vs OLED pins

# ── jumpers (all routed on the UNDERSIDE) ─────────────────────────────────────
def feed(net, target): jump(net, NET_STUB[net], target)
# matrix row nets
feed('GP5', (7,42));  jump('GP5', (7,44), (18,44))
feed('GP7', (6,31));  jump('GP7', (6,33), (13,33));  jump('GP7', (13,31), (20,31))
feed('GP8', (7,10));  jump('GP8', (7,12), (18,12))
feed('GP9', (6,10));  jump('GP9', (6,12), (13,12));  jump('GP9', (13,14), (20,14))
# matrix column nets (chain every segment of each column net)
feed('GP10', (4,27)); jump('GP10', (4,29), (3,29))
jump('GP10', (3,26), (17,22));  jump('GP10', (17,20), (15,19))
feed('GP11', (15,28)); jump('GP11', (15,25), (10,5)); jump('GP11', (10,7), (10,38))
jump('GP11', (10,9), (4,16))   # P2 M2 node segment (col 4, rows 14-22)
feed('GP12', (17,30)); jump('GP12', (17,32), (4,32))
jump('GP12', (4,34), (3,12));   jump('GP12', (3,14), (15,10))
feed('GP13', (15,34)); jump('GP13', (15,36), (4,10))
# OLED signals
feed('GP16', (12,30)); feed('GP17', (11,30))
feed('GP18', (9,15));  feed('GP19', (10,15))
# LED strip DIN
feed('GP20', (0,40));  feed('GP22', (0,1))
# power
feed('3V3', (22,48))
jump('3V3', (11,15), (22,15));  jump('3V3', (10,32), (22,32))
feed('GND', (1,49))
jump('GND', (12,15), (1,15));   jump('GND', (9,30), (1,30))
jump('GND', (0,7), (1,7));      jump('GND', (0,48), (1,48))
jump('5V', (0,4), (23,4));      jump('5V', (0,45), (23,45))

# ── orientation indicators ─────────────────────────────────────────────────────
MODULES.append(('USB', 0.9, 49.2, 1.9, 51.8))
label(0.1, 48.6, 'USB')
label(1.6, 55.6, 'pin 1 (GP0)')
label(1.4, 45.3, 'VBUS n/c')
label(4.0, 55.6, 'Pico on TOP side - route all jumpers on the UNDERSIDE')
label(16.0, 54.6, '(5V: LED strips; Pico from USB; GND shared)')

# ── verifier ───────────────────────────────────────────────────────────────────
def verify():
    cutcols = {}
    for (c, h) in CUTS: cutcols.setdefault(c, []).append(h)
    ok = True; msgs = []
    for (c, h) in CUTS:
        if any(p['c'] == c and p['h'] == h for p in PADS):
            ok = False; msgs.append(f'pad on cut ({c},{h})')
    def seg(c, h): return (c, sum(1 for k in cutcols.get(c, []) if k < h))
    parent = {}
    def find(s):
        parent.setdefault(s, s)
        while parent[s] != s: parent[s] = parent[parent[s]]; s = parent[s]
        return s
    def union(a, b): parent[find(a)] = find(b)
    for j in JUMPERS: union(seg(*j['a']), seg(*j['b']))
    from collections import defaultdict, Counter
    net_roots = defaultdict(set); root_nets = defaultdict(set)
    for p in PADS:
        r = find(seg(p['c'], p['h']))
        net_roots[p['net']].add(r); root_nets[r].add(p['net'])
    for j in JUMPERS:
        for e in (j['a'], j['b']):
            r = find(seg(*e)); net_roots[j['net']].add(r); root_nets[r].add(j['net'])
    real = lambda n: n.startswith('GP') or n in ('3V3','5V','GND')
    for net in sorted(net_roots):
        if real(net) and len(net_roots[net]) != 1:
            ok = False; msgs.append(f'OPEN  {net}: {len(net_roots[net])} disconnected groups')
    for r, nets in root_nets.items():
        realnets = {n for n in nets if real(n)}
        if len(realnets) > 1:
            ok = False; msgs.append(f'SHORT {sorted(realnets)} share a strip segment')
    # physical: no hole takes two wire ends / a wire end on a pad
    padpos = {(p['c'], p['h']) for p in PADS}
    ends = [e for j in JUMPERS for e in (j['a'], j['b'])]
    for e, n in Counter(ends).items():
        if n > 1: ok = False; msgs.append(f'two wire ends in hole {e}')
    for e in ends:
        if e in padpos: ok = False; msgs.append(f'wire end on pad hole {e}')
        if e in set(CUTS): ok = False; msgs.append(f'wire end on cut {e}')
    return ok, msgs, {'pads':len(PADS),'cuts':len(CUTS),'jumpers':len(JUMPERS),
                      'nets':len([n for n in net_roots if real(n)])}

# ── net colours ────────────────────────────────────────────────────────────────
NETRGB = {'GND':(30,30,30),'5V':(210,40,40),'3V3':(230,140,20),
          'GP5':(200,40,40),'GP7':(230,120,20),'GP8':(170,150,20),'GP9':(40,160,60),
          'GP10':(40,90,200),'GP11':(130,60,190),'GP12':(140,90,50),'GP13':(90,90,90),
          'GP16':(40,90,200),'GP17':(20,150,200),'GP18':(90,140,220),'GP19':(60,180,210),
          'GP20':(40,160,60),'GP22':(150,60,190)}
def netrgb(n): return NETRGB.get(n, (150,150,150))

# ── DIYLC emit ─────────────────────────────────────────────────────────────────
SIZE_UNIT = 'org.diylc.core.measures.SizeUnit'
def _sz(v,u): return f'<value>{v}</value><unit class="{SIZE_UNIT}">{u}</unit>'
def _col(r,g,b,a=255): return f'<red>{r}</red><green>{g}</green><blue>{b}</blue><alpha>{a}</alpha>'
def _font(s):
    a=[('weight','<null/>'),('transform','<null/>'),('width','<null/>'),('size',f'<float>{s}.0</float>'),
       ('tracking','<null/>'),('family','<string>Tahoma</string>'),('superscript','<null/>'),('posture','<null/>')]
    e=''.join(f'<entry><awt-text-attribute>{k}</awt-text-attribute>{v}</entry>' for k,v in a)
    return f'<font><attributes>{e}</attributes></font>'
def X(c): return round(c*0.1, 3)
def Y(h): return round(h*0.1, 3)

def emit_diy():
    C = []
    C.append(f'''<org.diylc.components.boards.VeroBoard>
  <name>Board</name><alpha>127</alpha><value></value>
  <controlPoints><java.awt.Point x="{X(0)}" y="{Y(1)}"/><java.awt.Point x="{X(23)}" y="{Y(55)}"/></controlPoints>
  <firstPoint x="{X(0)}" y="{Y(1)}"/><secondPoint x="{X(23)}" y="{Y(55)}"/>
  <boardColor>{_col(248,235,179)}</boardColor><borderColor>{_col(173,164,125)}</borderColor>
  <coordinateColor>{_col(120,120,120)}</coordinateColor><drawCoordinates>true</drawCoordinates>
  <spacing>{_sz(0.1,'in')}</spacing><stripColor>{_col(218,138,103)}</stripColor>
  <orientation>VERTICAL</orientation>
</org.diylc.components.boards.VeroBoard>''')
    def rect(n, c0,h0,c1,h1, fill, a=70):
        C.append(f'''<org.diylc.components.shapes.Rectangle>
  <name>{n}</name><alpha>{a}</alpha><value></value>
  <controlPoints><java.awt.Point x="{X(c0)}" y="{Y(h0)}"/><java.awt.Point x="{X(c1)}" y="{Y(h1)}"/></controlPoints>
  <firstPoint x="{X(c0)}" y="{Y(h0)}"/><secondPoint x="{X(c1)}" y="{Y(h1)}"/>
  <color>{_col(*fill)}</color><borderColor>{_col(0,0,0)}</borderColor>
  <borderThickness>{_sz(0.2,'mm')}</borderThickness><edgeRadius>{_sz(1.0,'mm')}</edgeRadius>
</org.diylc.components.shapes.Rectangle>''')
    rect('Pico', PC0-0.5, PTOP-0.7, PC0+19.5, PBOT+0.7, (70,70,80), a=60)
    for (nm,c0,h0,c1,h1) in MODULES: rect(nm.replace(' ','_'), c0,h0,c1,h1, (35,40,55), a=55)
    for i,(c0,h0,c1,h1) in enumerate(SW_BODIES): rect(f'sw{i}', c0-0.2,h0,c1+0.2,h1, (60,60,60), a=55)
    for i,(c,h) in enumerate(CUTS):
        C.append(f'''<org.diylc.components.connectivity.TraceCut>
  <name>xc{i}</name><size>{_sz(0.07,'in')}</size><fillColor>{_col(255,255,255)}</fillColor>
  <borderColor>{_col(255,0,0)}</borderColor><boardColor>{_col(248,235,179)}</boardColor>
  <cutBetweenHoles>false</cutBetweenHoles><holeSpacing>{_sz(0.1,'in')}</holeSpacing>
  <point x="{X(c)}" y="{Y(h)}"/>
</org.diylc.components.connectivity.TraceCut>''')
    for i,j in enumerate(JUMPERS):
        r = netrgb(j['net']); (c1,h1),(c2,h2) = j['a'], j['b']
        C.append(f'''<org.diylc.components.connectivity.Jumper>
  <name>j{i}</name><alpha>100</alpha>
  <points><java.awt.Point x="{X(c1)}" y="{Y(h1)}"/><java.awt.Point x="{X(c2)}" y="{Y(h2)}"/></points>
  <bodyColor>{_col(60,60,60)}</bodyColor><borderColor>{_col(0,0,0)}</borderColor>
  <labelColor>{_col(0,0,0)}</labelColor><leadColor>{_col(*r)}</leadColor>
  <display>NONE</display><flipStanding>false</flipStanding>
</org.diylc.components.connectivity.Jumper>''')
    for i,p in enumerate(PADS):
        C.append(f'''<org.diylc.components.connectivity.SolderPad>
  <name>p{i}</name><size>{_sz(0.09,'in')}</size><color>{_col(*netrgb(p['net']))}</color>
  <point x="{X(p['c'])}" y="{Y(p['h'])}"/><type>ROUND</type><holeSize>{_sz(0.8,'mm')}</holeSize><layer>_1</layer>
</org.diylc.components.connectivity.SolderPad>''')
    for i,(c,h,t) in enumerate(LABELS):
        C.append(f'''<org.diylc.components.misc.Label>
  <name>t{i}</name><point x="{X(c)}" y="{Y(h)}"/><text>{t}</text>{_font(7)}
  <color>{_col(0,0,0)}</color><center>false</center>
  <horizontalAlignment>LEFT</horizontalAlignment><verticalAlignment>CENTER</verticalAlignment>
  <orientation>DEFAULT</orientation>
</org.diylc.components.misc.Label>''')
    body = '\n'.join('    '+l for comp in C for l in comp.splitlines())
    return f'''<?xml version="1.0" encoding="UTF-8" ?>
<org.diylc.core.Project>
  <fileVersion><major>3</major><minor>32</minor><build>0</build></fileVersion>
  <title>Mega Blastoise - single board rev 3</title><author>generated</author>
  <description>Face-to-face console on one 24x55 stripboard: move buttons at screen corners, P2 module rotated 180deg, LED connectors on the edge, Pico on top, MB102 5V input.</description>
  <width>{_sz(24.0,'cm')}</width><height>{_sz(30.0,'cm')}</height>
  <gridSpacing>{_sz(0.1,'in')}</gridSpacing>
  <components>
{body}
  </components>
  <groups/><lockedLayers/>
</org.diylc.core.Project>
'''

if __name__ == '__main__':
    import sys
    ok, msgs, stats = verify()
    print('stats:', stats)
    for m in msgs: print('  ', m)
    print('VERIFY:', 'PASS' if ok else 'FAIL')
    if ok:
        path = sys.argv[1] if len(sys.argv) > 1 else 'mega_blastoise_single_board.diy'
        with open(path, 'w') as f: f.write(emit_diy())
        print('wrote', path)
