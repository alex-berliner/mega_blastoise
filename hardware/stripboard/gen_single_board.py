#!/usr/bin/env python3
"""Single-board stripboard layout for the WHOLE console on one 24x55 board.

Everything on one board (strips vertical, cols A..X, holes 1..55):
  - Raspberry Pi Pico on the UNDERSIDE, across strips at the bottom
  - 14x 6mm tactile switch button matrix
  - 2x SSD1306 OLED footprints (4-pad each)
  - 2x WS2812B strip footprints (3-pad each)
  - GND / 5V / 3V3 power rails
  (no buzzer)

Architecture that makes it fit 24 wide:
  The Pico eats 20 of 24 columns, so we isolate it. Cut every Pico column just
  above its top pin row -> the whole UPPER board (holes 1..46) becomes free
  strips. We then feed the ~16 used Pico nets up into the free board with
  jumper wires (insulated, they cross freely). The matrix, OLEDs, LED pads and
  power rails are laid out on the free upper board with normal technique.

Correctness is proven by verify() (union-find over strips+cuts+jumpers+pads)
before anything is emitted.
"""

# ── Pico physical pinout (RP2040 board; GND every 5th pin) ─────────────────────
PICO = {1:'GP0',2:'GP1',3:'GND',4:'GP2',5:'GP3',6:'GP4',7:'GP5',8:'GND',9:'GP6',
        10:'GP7',11:'GP8',12:'GP9',13:'GND',14:'GP10',15:'GP11',16:'GP12',17:'GP13',
        18:'GND',19:'GP14',20:'GP15',21:'GP16',22:'GP17',23:'GND',24:'GP18',25:'GP19',
        26:'GP20',27:'GP21',28:'GND',29:'GP22',30:'RUN',31:'GP26',32:'GP27',33:'GND',
        34:'GP28',35:'VREF',36:'3V3',37:'3V3EN',38:'GND',39:'VSYS',40:'VBUS'}

# ── the netlist model ──────────────────────────────────────────────────────────
PADS = []      # {name, net, c, h}
CUTS = []      # (c, h)
JUMPERS = []   # {net, a:(c,h), b:(c,h)}
SW_BODIES = [] # (c0,h0,c1,h1) switch outlines (for drawing)
MODULES = []   # (name, c0,h0,c1,h1)
LABELS = []    # (c, h, text)

def pad(name, net, c, h):        PADS.append(dict(name=name, net=net, c=c, h=h))
def cut(c, h):                   CUTS.append((c, h))
def jump(net, a, b):             JUMPERS.append(dict(net=net, a=a, b=b))

# ── Pico placement: underside, cols 2..21, top row h48, bottom row h55 ─────────
PC0 = 2                       # left column of the Pico (col index of pin at "j=0")
PTOP, PBOT = 48, 55           # the two pin rows
def pico_top_col(pin): return PC0 + (pin-1)      # pins 1..20 -> cols 2..21
def pico_bot_col(pin): return PC0 + (40-pin)     # pins 21..40 -> cols 21..2
USED_GP = {5,7,8,9,10,11,12,13,16,17,18,19,20,22}
GND_TAP_PIN = 38              # the single GND pin we wire to the board rail
NET_COL = {}                  # net -> (col, row-hole) where the Pico exposes it
for pin,net in PICO.items():
    c, h = (pico_top_col(pin), PTOP) if pin <= 20 else (pico_bot_col(pin), PBOT)
    if   net == 'VBUS':                              board = '5V'
    elif net == '3V3':                               board = '3V3'
    elif net == 'GND':                               board = 'GND' if pin == GND_TAP_PIN else f'x{pin}'
    elif net.startswith('GP') and int(net[2:]) in USED_GP: board = net
    else:                                            board = f'x{pin}'   # unused pin, isolated
    pad(f'pico{pin}', board, c, h)
    if board in ('5V','3V3','GND') or (board.startswith('GP') and int(board[2:]) in USED_GP):
        NET_COL[board] = (c, h)
# the Pico spans cols 2..21; isolate the upper board from the top pins, and the
# two rows from each other
for c in range(PC0, PC0+20):
    cut(c, 47)     # isolate upper board (holes 1..46) from the top-pin stubs
    cut(c, 51)     # isolate top row (48-50) from bottom row (52-55)
LABELS.append((PC0, PTOP-1.4, 'Raspberry Pi Pico  (UNDERSIDE, 40-pin)'))

def pico_stub(net):
    """A point on the Pico stub carrying `net`, for a feed-jumper endpoint."""
    c, h = NET_COL[net]
    return (c, 49 if h == PTOP else 53)

# ── power rails: full-length strips on Pico-free columns ───────────────────────
RAIL = {'GND':1, '5V':22, '3V3':23}          # col A(0) left as margin
for net,c in RAIL.items():
    for h in (2, 46):                        # anchor pads top & bottom of the rail
        pad(f'rail_{net}_{h}', net, c, h)
    LABELS.append((c, 1, net))
# feed the rails from the Pico power pins
jump('5V',  pico_stub('5V'),  (RAIL['5V'], 53))
jump('GND', pico_stub('GND'), (RAIL['GND'], 44))
jump('3V3', pico_stub('3V3'), (RAIL['3V3'], 44))

def rail_tap(net, c, h):
    """Jumper from a power rail to (c,h)."""
    jump(net, (RAIL[net], h), (c, h))

# ── button matrix (verified topology), holes 1..23 ─────────────────────────────
# row = (net, busCol, nodeCol, ncols).  node is 3 cols left of the bus.
MROWS = [('GP5',21,18,4), ('GP7',15,12,3), ('GP8',9,6,4), ('GP9',3,0,3)]
def btn_base(c): return 4 + 4*c              # first leg hole of column c
MCUT = 24                                    # isolate matrix (<=23) from the peripheral zone
NODECOLS = {r[0]: r[2] for r in MROWS}
for (net, bus, node, ncols) in MROWS:
    for c in range(ncols):
        hb = btn_base(c)
        for hh in (hb, hb+2):
            pad(f'{net}_c{c}_r{hh}', net, bus, hh)          # row-terminal legs on bus strip
            pad(f'{net}_c{c}_n{hh}', f'COL{c}', node, hh)   # col-terminal legs on node strip
        SW_BODIES.append((node, hb-1, bus, hb+3))
    for c in range(ncols-1):                                 # isolate each column node
        cut(node, hb_cut := btn_base(c)+3)
# column buses: tie the c-th node segment across all rows that have column c, and
# name that net COL c (== GPcol)
COL_NET = {0:'GP10', 1:'GP11', 2:'GP12', 3:'GP13'}
for c in range(4):
    rows_c = [r for r in MROWS if c < r[3]]
    hj = btn_base(c) + 1
    nodes = [r[2] for r in rows_c]
    for a,b in zip(nodes[:-1], nodes[1:]):
        jump(f'COL{c}', (a, hj), (b, hj))
# rename COLc pads/jumpers to their GP net so feeds line up
for p in PADS:
    if p['net'].startswith('COL'): p['net'] = COL_NET[int(p['net'][3:])]
for j in JUMPERS:
    if j['net'].startswith('COL'): j['net'] = COL_NET[int(j['net'][3:])]
# isolate the matrix columns from the peripheral zone above
for colc in sorted({r[1] for r in MROWS} | {r[2] for r in MROWS}):
    cut(colc, MCUT)
# feed the 8 matrix nets from the Pico up into the matrix
FEED_ROW = {'GP5':(21,22), 'GP7':(15,22), 'GP8':(9,22), 'GP9':(3,22)}
for net,tgt in FEED_ROW.items():
    jump(net, pico_stub(net), tgt)
FEED_COL = {'GP10':(0,5), 'GP11':(0,9), 'GP12':(0,13), 'GP13':(18,17)}
for net,tgt in FEED_COL.items():
    jump(net, pico_stub(net), tgt)
LABELS.append((0, 2, 'BUTTON MATRIX'))

# ── OLED + LED footprints on the free peripheral zone (holes 25..46) ───────────
def footprint(name, hrow, cols_nets):
    c0 = cols_nets[0][0]; c1 = cols_nets[-1][0]
    MODULES.append((name, c0-0.4, hrow-1.0, c1+0.4, hrow+1.0))
    LABELS.append((c0, hrow-1.3, name))
    for (c, net) in cols_nets:
        pad(f'{name}_{net}', net, c, hrow)

# each peripheral gets its own columns so nothing shares a strip; power pins are
# tapped from the rails, signal pins fed from the Pico
def oled(name, hrow, cols, sda_net, scl_net):
    cS,cC,cV,cG = cols          # SDA, SCL, VCC(3V3), GND
    footprint(name, hrow, [(cS,sda_net),(cC,scl_net),(cV,'3V3'),(cG,'GND')])
    jump(sda_net, pico_stub(sda_net), (cS, hrow))
    jump(scl_net, pico_stub(scl_net), (cC, hrow))
    rail_tap('3V3', cV, hrow); rail_tap('GND', cG, hrow)

def led(name, hrow, cols, din_net):
    cD,cV,cG = cols             # DIN, 5V, GND
    footprint(name, hrow, [(cD,din_net),(cV,'5V'),(cG,'GND')])
    jump(din_net, pico_stub(din_net), (cD, hrow))
    rail_tap('5V', cV, hrow); rail_tap('GND', cG, hrow)

oled('OLED P1', 30, (3,4,5,6),   'GP16','GP17')
oled('OLED P2', 34, (8,9,10,11), 'GP18','GP19')
led ('LED P1',  38, (13,14,15),  'GP20')
led ('LED P2',  42, (17,18,19),  'GP22')

# ── verifier ───────────────────────────────────────────────────────────────────
def verify():
    cutcols = {}
    for (c,h) in CUTS: cutcols.setdefault(c, []).append(h)
    def seg(c,h): return (c, sum(1 for k in cutcols.get(c, []) if k < h))
    parent = {}
    def find(s):
        parent.setdefault(s, s)
        while parent[s] != s: parent[s] = parent[parent[s]]; s = parent[s]
        return s
    def union(a,b): parent[find(a)] = find(b)
    for j in JUMPERS: union(seg(*j['a']), seg(*j['b']))
    # net -> set of roots ; root -> set of nets
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
    for r,nets in root_nets.items():
        realnets = {n for n in nets if real(n)}
        if len(realnets) > 1:
            ok = False; msgs.append(f'SHORT {sorted(realnets)} share a strip segment')
    return ok, msgs, {'pads':len(PADS),'cuts':len(CUTS),'jumpers':len(JUMPERS),'nets':len([n for n in net_roots if real(n)])}

# ── net colours (shared by DIYLC + preview) ────────────────────────────────────
NETRGB = {'GND':(30,30,30),'5V':(210,40,40),'3V3':(230,140,20),
          'GP5':(200,40,40),'GP7':(230,120,20),'GP8':(170,150,20),'GP9':(40,160,60),
          'GP10':(40,90,200),'GP11':(130,60,190),'GP12':(140,90,50),'GP13':(90,90,90),
          'GP16':(40,90,200),'GP17':(20,150,200),'GP18':(90,140,220),'GP19':(60,180,210),
          'GP20':(40,160,60),'GP22':(150,60,190)}
def netrgb(n): return NETRGB.get(n,(150,150,150))

# ── DIYLC emit ─────────────────────────────────────────────────────────────────
SIZE_UNIT='org.diylc.core.measures.SizeUnit'
def _sz(v,u): return f'<value>{v}</value><unit class="{SIZE_UNIT}">{u}</unit>'
def _col(r,g,b,a=255): return f'<red>{r}</red><green>{g}</green><blue>{b}</blue><alpha>{a}</alpha>'
def _font(s):
    a=[('weight','<null/>'),('transform','<null/>'),('width','<null/>'),('size',f'<float>{s}.0</float>'),
       ('tracking','<null/>'),('family','<string>Tahoma</string>'),('superscript','<null/>'),('posture','<null/>')]
    e=''.join(f'<entry><awt-text-attribute>{k}</awt-text-attribute>{v}</entry>' for k,v in a)
    return f'<font><attributes>{e}</attributes></font>'
def X(c): return round(c*0.1,2)
def Y(h): return round(h*0.1,2)

def emit_diy():
    C=[]
    C.append(f'''<org.diylc.components.boards.VeroBoard>
  <name>Board</name><alpha>127</alpha><value></value>
  <controlPoints><java.awt.Point x="{X(0)}" y="{Y(1)}"/><java.awt.Point x="{X(23)}" y="{Y(55)}"/></controlPoints>
  <firstPoint x="{X(0)}" y="{Y(1)}"/><secondPoint x="{X(23)}" y="{Y(55)}"/>
  <boardColor>{_col(248,235,179)}</boardColor><borderColor>{_col(173,164,125)}</borderColor>
  <coordinateColor>{_col(120,120,120)}</coordinateColor><drawCoordinates>true</drawCoordinates>
  <spacing>{_sz(0.1,'in')}</spacing><stripColor>{_col(218,138,103)}</stripColor>
  <orientation>VERTICAL</orientation>
</org.diylc.components.boards.VeroBoard>''')
    def rect(n,c0,h0,c1,h1,fill,a=70):
        C.append(f'''<org.diylc.components.shapes.Rectangle>
  <name>{n}</name><alpha>{a}</alpha><value></value>
  <controlPoints><java.awt.Point x="{X(c0)}" y="{Y(h0)}"/><java.awt.Point x="{X(c1)}" y="{Y(h1)}"/></controlPoints>
  <firstPoint x="{X(c0)}" y="{Y(h0)}"/><secondPoint x="{X(c1)}" y="{Y(h1)}"/>
  <color>{_col(*fill)}</color><borderColor>{_col(0,0,0)}</borderColor>
  <borderThickness>{_sz(0.2,'mm')}</borderThickness><edgeRadius>{_sz(1.0,'mm')}</edgeRadius>
</org.diylc.components.shapes.Rectangle>''')
    # Pico body + module + switch bodies
    rect('Pico', PC0-0.35, PTOP-0.4, PC0+19+0.35, PBOT+0.4, (70,70,80), a=60)
    for (nm,c0,h0,c1,h1) in MODULES: rect(nm.replace(' ','_'), c0,h0,c1,h1, (35,40,55), a=70)
    for i,(c0,h0,c1,h1) in enumerate(SW_BODIES): rect(f'sw{i}', c0-0.05,h0+0.0,c1+0.05,h1, (60,60,60), a=55)
    # cuts
    for i,(c,h) in enumerate(CUTS):
        C.append(f'''<org.diylc.components.connectivity.TraceCut>
  <name>xc{i}</name><size>{_sz(0.07,'in')}</size><fillColor>{_col(255,255,255)}</fillColor>
  <borderColor>{_col(255,0,0)}</borderColor><boardColor>{_col(248,235,179)}</boardColor>
  <cutBetweenHoles>false</cutBetweenHoles><holeSpacing>{_sz(0.1,'in')}</holeSpacing>
  <point x="{X(c)}" y="{Y(h)}"/>
</org.diylc.components.connectivity.TraceCut>''')
    # jumpers
    for i,j in enumerate(JUMPERS):
        r=netrgb(j['net']); (c1,h1),(c2,h2)=j['a'],j['b']
        C.append(f'''<org.diylc.components.connectivity.Jumper>
  <name>j{i}</name><alpha>100</alpha>
  <points><java.awt.Point x="{X(c1)}" y="{Y(h1)}"/><java.awt.Point x="{X(c2)}" y="{Y(h2)}"/></points>
  <bodyColor>{_col(60,60,60)}</bodyColor><borderColor>{_col(0,0,0)}</borderColor>
  <labelColor>{_col(0,0,0)}</labelColor><leadColor>{_col(*r)}</leadColor>
  <display>NONE</display><flipStanding>false</flipStanding>
</org.diylc.components.connectivity.Jumper>''')
    # pads
    for i,p in enumerate(PADS):
        C.append(f'''<org.diylc.components.connectivity.SolderPad>
  <name>p{i}</name><size>{_sz(0.09,'in')}</size><color>{_col(*netrgb(p['net']))}</color>
  <point x="{X(p['c'])}" y="{Y(p['h'])}"/><type>ROUND</type><holeSize>{_sz(0.8,'mm')}</holeSize><layer>_1</layer>
</org.diylc.components.connectivity.SolderPad>''')
    # labels
    for i,(c,h,t) in enumerate(LABELS):
        C.append(f'''<org.diylc.components.misc.Label>
  <name>t{i}</name><point x="{X(c)}" y="{Y(h)}"/><text>{t}</text>{_font(7)}
  <color>{_col(0,0,0)}</color><center>false</center>
  <horizontalAlignment>LEFT</horizontalAlignment><verticalAlignment>CENTER</verticalAlignment>
  <orientation>DEFAULT</orientation>
</org.diylc.components.misc.Label>''')
    body='\n'.join('    '+l for comp in C for l in comp.splitlines())
    return f'''<?xml version="1.0" encoding="UTF-8" ?>
<org.diylc.core.Project>
  <fileVersion><major>3</major><minor>32</minor><build>0</build></fileVersion>
  <title>Mega Blastoise - single board</title><author>generated</author>
  <description>Whole console on one 24x55 stripboard: Pico (underside) + matrix + 2 OLED + 2 WS2812B + power.</description>
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
        with open(path,'w') as f: f.write(emit_diy())
        print('wrote', path)
