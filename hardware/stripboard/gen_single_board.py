#!/usr/bin/env python3
"""Single-board stripboard layout, rev 2 - whole console on one 24x55 board.

Board: strips vertical, cols A..X (0..23), holes 1..55.

Changes from rev 1 (user feedback):
  1. everything pulled to the top - no dead zone between buttons and OLEDs
  2. per-player zones: each player's 7 switches sit directly above their OLED
     (P2 = left half, P1 = right half), like the real console
  3. NeoPixels are OFF the board - each strip gets a 3-pad connector
     (DIN/5V/GND) to wire out to the external strip
  4. MB102 5V supply input: two adjacent pads at the bottom right, "-" on the
     left, "+" on the right; 5V rail (col X) runs next to the 3V3 rail (col W);
     ground is shared with the Pico
  5. Pico mounted on the TOP side (USB to the LEFT); all jumper wires are
     routed on the UNDERSIDE so the top stays clean
  6. orientation indicators: USB tab drawn on the left end, pin-1 marker at the
     bottom-left pin (GP0), VBUS marked n/c
  (no buzzer)

Pico orientation math (face up, USB pointing LEFT):
  top row, left->right  = pins 40..21
  bottom row, left->right = pins 1..20

Correctness is proven by verify() (union-find over strips + cuts + jumpers +
every pad) before the .diy is written.
"""

# ── Pico physical pinout (RP2040 board; GND every 5th pin) ─────────────────────
PICO = {1:'GP0',2:'GP1',3:'GND',4:'GP2',5:'GP3',6:'GP4',7:'GP5',8:'GND',9:'GP6',
        10:'GP7',11:'GP8',12:'GP9',13:'GND',14:'GP10',15:'GP11',16:'GP12',17:'GP13',
        18:'GND',19:'GP14',20:'GP15',21:'GP16',22:'GP17',23:'GND',24:'GP18',25:'GP19',
        26:'GP20',27:'GP21',28:'GND',29:'GP22',30:'RUN',31:'GP26',32:'GP27',33:'GND',
        34:'GP28',35:'VREF',36:'3V3',37:'3V3EN',38:'GND',39:'VSYS',40:'VBUS'}
USED_GP = {5,7,8,9,10,11,12,13,16,17,18,19,20,22}
GND_TAP_PIN = 38

NCOLS, NHOLES = 24, 55

PADS = []      # {name, net, c, h}
CUTS = []      # (c, h)  - cut AT a hole (kills that hole)
JUMPERS = []   # {net, a:(c,h), b:(c,h)}  - wires, routed on the UNDERSIDE
SW_BODIES = [] # (c0,h0,c1,h1)
MODULES = []   # (name, c0,h0,c1,h1)
LABELS = []    # (c, h, text)

def pad(name, net, c, h): PADS.append(dict(name=name, net=net, c=c, h=h))
def cut(c, h):            CUTS.append((c, h))
def jump(net, a, b):      JUMPERS.append(dict(net=net, a=a, b=b))
def label(c, h, t):       LABELS.append((c, h, t))

# ── Pico placement: TOP side, cols 2..21, pin rows 38 (top) and 45 (bottom) ────
PC0, PTOP, PBOT = 2, 38, 45
def pico_col(pin):  # returns (col, hole)
    if pin >= 21: return (PC0 + (40 - pin), PTOP)   # top row: 40..21 left->right
    return (PC0 + (pin - 1), PBOT)                  # bottom row: 1..20 left->right
TOPLAND, BOTLAND = 40, 47                            # where feed jumpers land on stubs

NET_STUB = {}
for pin, net in PICO.items():
    c, h = pico_col(pin)
    if   net == '3V3':  board = '3V3'
    elif net == 'GND':  board = 'GND' if pin == GND_TAP_PIN else f'x{pin}'
    elif net.startswith('GP') and int(net[2:]) in USED_GP: board = net
    else:               board = f'x{pin}'           # VBUS, VSYS, RUN, unused GPs: n/c
    pad(f'pico{pin}', board, c, h)
    if board in ('3V3','GND') or board.startswith('GP'):
        NET_STUB[board] = (c, TOPLAND if h == PTOP else BOTLAND)
# isolate the Pico's two pin rows from the board above and from each other
for c in range(PC0, PC0 + 20):
    cut(c, 37)      # frees everything above (holes 1..36)
    cut(c, 42)      # separates top-row stubs (38..41) from bottom-row (43..55)
def feed(net, target):
    jump(net, NET_STUB[net], target)

# ── power rails ────────────────────────────────────────────────────────────────
RAIL = {'GND':1, '3V3':22, '5V':23}      # 5V rail right next to the 3V3 rail
_rail_next = {'GND':26, '3V3':26, '5V':26}
def rail_tap(net, c, h):
    r = _rail_next[net]; _rail_next[net] += 1
    jump(net, (c, h), (RAIL[net], r))
for net, c in RAIL.items():
    pad(f'rail_{net}_top', net, c, 2)
    pad(f'rail_{net}_bot', net, c, 34)
label(0.2, 1.0, 'GND'); label(21.4, 1.0, '3V3  5V')
# rail feeds from the Pico
feed('GND', (RAIL['GND'], TOPLAND))       # pin 38, top row
feed('3V3', (RAIL['3V3'], TOPLAND))       # pin 36, top row

# ── button matrix: bases 3,8,13,18 (5-hole pitch), legs 2 apart, node 3 left ───
# row = (net, busCol, nodeCol, nbuttons, kind)
MROWS = [('GP5', 21, 18, 4, 'M'),   # P1 moves
         ('GP7', 15, 12, 3, 'S'),   # P1 party
         ('GP8',  9,  6, 4, 'M'),   # P2 moves
         ('GP9',  3,  0, 3, 'S')]   # P2 party
def base(c): return 3 + 5*c
MATRIX_CUT = 22
for (net, bus, node, n, kind) in MROWS:
    for c in range(n):
        hb = base(c)
        for hh in (hb, hb+2):
            pad(f'{net}_b{c}_{hh}', net, bus, hh)          # row-terminal legs
            pad(f'{net}_n{c}_{hh}', f'COL{c}', node, hh)   # col-terminal legs
        SW_BODIES.append((node, hb-0.5, bus, hb+2.5))
        label((node+bus)/2 - 0.35, hb+1, f'{kind}{c+1}')
    for c in range(n-1):
        cut(node, base(c)+4)                               # isolate column nodes
for colc in sorted({r[1] for r in MROWS} | {r[2] for r in MROWS}):
    cut(colc, MATRIX_CUT)                                  # matrix / peripheral border
# column buses: horizontal jumpers between node strips. Consecutive links in a
# chain alternate between the leg-gap hole (base+1) and the spare hole (base+3)
# so no strip hole ever takes two wire ends.
COL_NET = {0:'GP10', 1:'GP11', 2:'GP12', 3:'GP13'}
for c in range(4):
    nodes = sorted(r[2] for r in MROWS if c < r[3])
    for i, (a, b) in enumerate(zip(nodes[:-1], nodes[1:])):
        hj = base(c) + (1 if i % 2 == 0 else 3)
        jump(f'COL{c}', (a, hj), (b, hj))
for p in PADS:
    if p['net'].startswith('COL'): p['net'] = COL_NET[int(p['net'][3:])]
for j in JUMPERS:
    if j['net'].startswith('COL'): j['net'] = COL_NET[int(j['net'][3:])]
# matrix feeds from the Pico (bottom row pins; all land on free strip holes)
feed('GP5',  (21, 21)); feed('GP7', (15, 21))
feed('GP8',  ( 9, 21)); feed('GP9', ( 3, 21))
feed('GP10', (0, base(0)+3)); feed('GP11', (0, base(1)+3))
feed('GP12', (0, base(2)+3)); feed('GP13', (18, 21))
label(0.2, 2.2, 'PLAYER 2'); label(12.2, 2.2, 'PLAYER 1')

# ── LED strip connectors (strips are external; 3 solder pads each) ────────────
# row 23, in the gap between the matrix (ends 20) and the OLED headers (24)
LEDCONN = [('LED P2', 'GP22', 2),    # DIN,5V,GND at cols 2,3,4
           ('LED P1', 'GP20', 13)]   # DIN,5V,GND at cols 13,14,15
for (nm, din, c0) in LEDCONN:
    pad(f'{nm}_din', din,   c0,   23)
    pad(f'{nm}_5v',  '5V',  c0+1, 23)
    pad(f'{nm}_gnd', 'GND', c0+2, 23)
    MODULES.append((f'-> {nm} strip', c0-0.45, 22.5, c0+2.45, 23.5))
    label(c0-0.4, 22.0, f'-> {nm}: DIN 5V GND')
    feed(din, (c0, 24))              # DIN from the Pico (lands next to the pad)
    rail_tap('5V',  c0+1, 24)
    if c0+2 != 4:                    # col 4 is shared with OLED P2's GND pad;
        rail_tap('GND', c0+2, 24)    # that strip is tapped once by the OLED

# ── OLEDs: 4-pin header (GND,VCC,SCL,SDA) at row 24, screen hangs below ───────
OLEDS = [('OLED P2 (I2C1)', 4, 'GP18', 'GP19'),   # pads at cols 4..7
         ('OLED P1 (I2C0)', 16, 'GP16', 'GP17')]  # pads at cols 16..19
for (nm, c0, sda, scl) in OLEDS:
    pad(f'{nm}_gnd', 'GND', c0,   24)
    pad(f'{nm}_vcc', '3V3', c0+1, 24)
    pad(f'{nm}_scl', scl,   c0+2, 24)
    pad(f'{nm}_sda', sda,   c0+3, 24)
    rail_tap('GND', c0,   25)
    rail_tap('3V3', c0+1, 25)
    feed(scl, (c0+2, 25))
    feed(sda, (c0+3, 25))
    MODULES.append((nm, c0-2.3, 24.0, c0+5.7, 35.0))   # 27x28mm screen body
    label(c0-0.5, 36.0, nm)
# note: rail_tap/feed land at rows 25 on the same strips as the row-24 pads

# ── MB102 5V supply input: "-" left, "+" right, adjacent, bottom right ────────
MB_H = 49
cut(RAIL['3V3'], 48)                       # free the bottom of col W for the "-"
pad('mb102_minus', 'GND', 22, MB_H)
pad('mb102_plus',  '5V',  23, MB_H)
jump('GND', (22, MB_H+1), (1, MB_H))       # shared ground (own hole, below the pad)
MODULES.append(('MB102 5V in', 21.4, 48.2, 23.6, 49.8))
label(18.3, 49.0, 'MB102:  - +')
label(15.0, 50.8, '(5V for LED strips; Pico runs from USB; GND shared)')

# ── orientation indicators ─────────────────────────────────────────────────────
MODULES.append(('USB', 0.9, 40.2, 1.9, 42.8))      # USB tab, left end of the Pico
label(0.15, 41.5, 'USB')
label(1.55, 46.6, 'pin 1 (GP0)')
label(1.4, 36.2, 'VBUS n/c')
label(2.0, 53.0, 'Pico on the TOP side - route all jumper wires on the UNDERSIDE')

# ── verifier ───────────────────────────────────────────────────────────────────
def verify():
    cutcols = {}
    for (c, h) in CUTS: cutcols.setdefault(c, []).append(h)
    for (c, h) in CUTS:
        assert not any(p['c'] == c and p['h'] == h for p in PADS), f'pad on cut ({c},{h})'
    def seg(c, h): return (c, sum(1 for k in cutcols.get(c, []) if k < h))
    parent = {}
    def find(s):
        parent.setdefault(s, s)
        while parent[s] != s: parent[s] = parent[parent[s]]; s = parent[s]
        return s
    def union(a, b): parent[find(a)] = find(b)
    for j in JUMPERS: union(seg(*j['a']), seg(*j['b']))
    from collections import defaultdict
    net_roots = defaultdict(set); root_nets = defaultdict(set)
    for p in PADS:
        r = find(seg(p['c'], p['h']))
        net_roots[p['net']].add(r); root_nets[r].add(p['net'])
    for j in JUMPERS:
        for e in (j['a'], j['b']):
            r = find(seg(*e)); net_roots[j['net']].add(r); root_nets[r].add(j['net'])
    ok = True; msgs = []
    real = lambda n: n.startswith('GP') or n in ('3V3','5V','GND')
    for net in sorted(net_roots):
        if real(net) and len(net_roots[net]) != 1:
            ok = False; msgs.append(f'OPEN  {net}: {len(net_roots[net])} disconnected groups')
    for r, nets in root_nets.items():
        realnets = {n for n in nets if real(n)}
        if len(realnets) > 1:
            ok = False; msgs.append(f'SHORT {sorted(realnets)} share a strip segment')
    return ok, msgs, {'pads':len(PADS),'cuts':len(CUTS),'jumpers':len(JUMPERS),
                      'nets':len([n for n in net_roots if real(n)])}

# ── net colours (shared by DIYLC + preview) ────────────────────────────────────
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
  <title>Mega Blastoise - single board rev 2</title><author>generated</author>
  <description>Console on one 24x55 stripboard: Pico on top (USB left), per-player button+OLED zones, external LED strip connectors, MB102 5V input.</description>
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
