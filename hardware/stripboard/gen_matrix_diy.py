#!/usr/bin/env python3
"""Generate a DIYLC (.diy) stripboard layout for the Mega Blastoise button matrix.

Target board: strips run VERTICALLY, one continuous rail per letter column
(A..X across, holes 1..55 down) -- this matches Alex's physical stripboard.

4x4 key matrix, 14 tactile switches:
  row nets  = vertical bus strips (uncut): GP5 (P1 moves), GP7 (P1 party),
              GP8 (P2 moves), GP9 (P2 party)
  col nets  = adjacent node strips, CUT between rows, tied by jumpers: GP10..GP13

Switch footprint: 6mm tactile, legs land on a 0.2" x 0.3" (3x4 hole) rectangle.
  The two legs of one terminal sit 0.2" apart ALONG a strip; the two terminals
  sit 0.3" apart ACROSS strips (3 letters). Measured from Alex's actual parts.

The layout is authored in a natural "strips horizontal" frame, then transposed
to "strips vertical" on emit (T()), so it drops onto the real board 1:1.
Coordinates are INCHES on a 0.1" grid (DIYLC's native units).
"""

SIZE_UNIT = 'org.diylc.core.measures.SizeUnit'

# ── switch geometry (edit here if your switches differ) ─────────────────────────
VSPAN = 0.3   # bus->node span  (long axis of the switch, 4 holes inclusive)
HSPAN = 0.2   # leg pair along a strip (short axis, 3 holes inclusive)
COLPITCH = 0.4  # spacing between buttons in a row

# ── transpose: author frame (strips horizontal) -> board frame (strips vertical)
# author x (position along a row)      -> board Y (hole number, 1..55, down)
# author y (which strip)               -> board X (letter column, A..X, across)
def T(x, y):
    return (round(y - 0.3, 2), round(x - 2.0 + 0.5, 2))

def size(v, u):
    return f'<value>{v}</value><unit class="{SIZE_UNIT}">{u}</unit>'

def color(r, g, b, a=255):
    return f'<red>{r}</red><green>{g}</green><blue>{b}</blue><alpha>{a}</alpha>'

def font(sz):
    attrs = [('weight', '<null/>'), ('transform', '<null/>'), ('width', '<null/>'),
             ('size', f'<float>{sz}.0</float>'), ('tracking', '<null/>'),
             ('family', '<string>Tahoma</string>'), ('superscript', '<null/>'),
             ('posture', '<null/>')]
    e = ''.join(f'<entry><awt-text-attribute>{k}</awt-text-attribute>{v}</entry>' for k, v in attrs)
    return f'<font><attributes>{e}</attributes></font>'

comps = []

def vero(x1, y1, x2, y2):
    comps.append(f'''<org.diylc.components.boards.VeroBoard>
  <name>Matrix board</name><alpha>127</alpha><value></value>
  <controlPoints><java.awt.Point x="{x1}" y="{y1}"/><java.awt.Point x="{x2}" y="{y2}"/></controlPoints>
  <firstPoint x="{x1}" y="{y1}"/><secondPoint x="{x2}" y="{y2}"/>
  <boardColor>{color(248,235,179)}</boardColor><borderColor>{color(173,164,125)}</borderColor>
  <coordinateColor>{color(120,120,120)}</coordinateColor><drawCoordinates>true</drawCoordinates>
  <spacing>{size(0.1,'in')}</spacing><stripColor>{color(218,138,103)}</stripColor>
  <orientation>VERTICAL</orientation>
</org.diylc.components.boards.VeroBoard>''')

def cut(name, x, y):
    px, py = T(x, y)
    comps.append(f'''<org.diylc.components.connectivity.TraceCut>
  <name>{name}</name><size>{size(0.07,'in')}</size>
  <fillColor>{color(255,255,255)}</fillColor><borderColor>{color(255,0,0)}</borderColor>
  <boardColor>{color(248,235,179)}</boardColor><cutBetweenHoles>false</cutBetweenHoles>
  <holeSpacing>{size(0.1,'in')}</holeSpacing><point x="{px}" y="{py}"/>
</org.diylc.components.connectivity.TraceCut>''')

def jumper(name, x1, y1, x2, y2):
    a = T(x1, y1); b = T(x2, y2)
    comps.append(f'''<org.diylc.components.connectivity.Jumper>
  <name>{name}</name><alpha>100</alpha>
  <points><java.awt.Point x="{a[0]}" y="{a[1]}"/><java.awt.Point x="{b[0]}" y="{b[1]}"/></points>
  <bodyColor>{color(60,60,60)}</bodyColor><borderColor>{color(0,0,0)}</borderColor>
  <labelColor>{color(0,0,0)}</labelColor><leadColor>{color(0,90,200)}</leadColor>
  <display>NONE</display><flipStanding>false</flipStanding>
</org.diylc.components.connectivity.Jumper>''')

def pad(name, x, y, rgb):
    px, py = T(x, y)
    comps.append(f'''<org.diylc.components.connectivity.SolderPad>
  <name>{name}</name><size>{size(0.08,'in')}</size><color>{color(*rgb)}</color>
  <point x="{px}" y="{py}"/><type>ROUND</type><holeSize>{size(0.8,'mm')}</holeSize><layer>_1</layer>
</org.diylc.components.connectivity.SolderPad>''')

def label(name, x, y, text, sz=7, halign='CENTER', raw=False):
    px, py = (x, y) if raw else T(x, y)
    comps.append(f'''<org.diylc.components.misc.Label>
  <name>{name}</name><point x="{px}" y="{py}"/><text>{text}</text>{font(sz)}
  <color>{color(0,0,0)}</color><center>false</center>
  <horizontalAlignment>{halign}</horizontalAlignment><verticalAlignment>CENTER</verticalAlignment>
  <orientation>DEFAULT</orientation>
</org.diylc.components.misc.Label>''')

# ── geometry (author frame: strips horizontal) ─────────────────────────────────
# row = (player, kind, ncols, gpio, bus_y);  node strip is VSPAN below the bus
ROWS = [
    ('P1', 'M', 4, 'GP5', 2.4),
    ('P1', 'S', 3, 'GP7', 1.8),
    ('P2', 'M', 4, 'GP8', 1.2),
    ('P2', 'S', 3, 'GP9', 0.6),
]
def node_of(by): return round(by - VSPAN, 2)
def xL(c):   return round(2.4 + COLPITCH * c, 2)        # first leg column
def xgap(c): return round(2.4 + COLPITCH * c + 0.1, 2)  # jumper runs in the leg gap

# board in BOARD-FRAME coords (already transposed): 24 letters (A..X) wide
vero(0.0, 0.30, 2.3, 2.60)

# buttons: 2 marker pads (row leg green on bus, col leg blue on node) + name label
for (p, k, ncols, gp, by) in ROWS:
    ny = node_of(by)
    for c in range(ncols):
        x = xL(c)
        pad(f'{p}{k}{c+1}r', x,             by, (0, 150, 0))   # row-net leg (bus strip)
        pad(f'{p}{k}{c+1}c', round(x+HSPAN,2), ny, (0, 90, 200))  # col-net leg (node strip)
        label(f'{p}{k}{c+1}L', round(x+HSPAN/2,2), round((by+ny)/2,2), f'{k}{c+1}', 7)

# node-strip isolation cuts (between column groups)
for (p, k, ncols, gp, by) in ROWS:
    ny = node_of(by)
    for c in range(ncols - 1):
        cut(f'X_{gp}_{c}', round(xL(c)+HSPAN+0.1,2), ny)

# column jumper buses (tie each column's node strips together)
COLS = [(0, 'GP10'), (1, 'GP11'), (2, 'GP12'), (3, 'GP13')]
for (c, gp) in COLS:
    nodes = sorted(node_of(by) for (p, k, ncols, g, by) in ROWS if c < ncols)
    x = xgap(c)
    for a, b in zip(nodes[:-1], nodes[1:]):
        jumper(f'{gp}_{a}', x, a, x, b)

# ── header solder points (wires to the Pico) ──────────────────────────────────
for (p, k, ncols, gp, by) in ROWS:                 # rows exit at the bus strip
    pad(f'H{gp}', 2.1, by, (200, 60, 0))
    label(f'L{gp}', 2.05, round(by+0.02,2), gp, 8)
for (c, gp) in COLS:                                # cols exit at bottom-most node
    nodes = sorted(node_of(by) for (p, k, ncols, g, by) in ROWS if c < ncols)
    x = xgap(c)
    pad(f'H{gp}', x, nodes[0], (200, 60, 0))
    label(f'L{gp}', round(x+0.02,2), round(nodes[0]-0.02,2), gp, 7)

# title labels (board-frame coords)
label('T1', 0.0, 0.12, 'Mega Blastoise 4x4 button matrix (14x 6mm tactile) - strips run down the columns', 8, 'LEFT', raw=True)
label('T2', 0.0, 2.78, 'rows GP5/7/8/9 = bus strips   cols GP10-13 = node strips   red X = cut track   blue = col jumpers', 7, 'LEFT', raw=True)

def project_xml(components, title='Mega Blastoise Button Matrix',
                desc='4x4 key matrix on stripboard, 14x 6mm tactile switches, vertical strips.'):
    body = '\n'.join('    ' + line for comp in components for line in comp.splitlines())
    return f'''<?xml version="1.0" encoding="UTF-8" ?>
<org.diylc.core.Project>
  <fileVersion><major>3</major><minor>32</minor><build>0</build></fileVersion>
  <title>{title}</title>
  <author>generated</author>
  <description>{desc}</description>
  <width>{size(29.0,'cm')}</width>
  <height>{size(21.0,'cm')}</height>
  <gridSpacing>{size(0.1,'in')}</gridSpacing>
  <components>
{body}
  </components>
  <groups/>
  <lockedLayers/>
</org.diylc.core.Project>
'''

if __name__ == '__main__':
    import sys
    path = sys.argv[1] if len(sys.argv) > 1 else 'mega_blastoise_matrix.diy'
    with open(path, 'w') as f:
        f.write(project_xml(comps))
    print(f'wrote {path}: {len(comps)} components')
