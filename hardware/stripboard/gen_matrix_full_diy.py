#!/usr/bin/env python3
"""Full-assembly DIYLC layout: the button matrix PLUS every other part -
switch bodies over each footprint, the Raspberry Pi Pico (as a 40-pin DIL),
the 8 ribbon wires from the header pads to the correct Pico pins, and a
parts list.

Builds on gen_matrix_diy.py (imported): that module populates the 81 matrix
components (board, cuts, jumpers, leg pads, headers) into g.comps at import;
here we append the remaining parts and write a second .diy.

Only component types with schemas proven against the DIYLC sample corpus are
used (Rectangle, HookupWire, DIL__IC, Label) so the file loads reliably.
"""
import gen_matrix_diy as g

C = g.comps          # matrix components already built at import
size, color, font, T = g.size, g.color, g.font, g.T

# ── Pico physical-pin map (RP2040 board, left column) ──────────────────────────
# GP -> physical pin number (1-based).  GP6/pin8 is unused.
GP_PIN = {'GP5':7, 'GP7':9, 'GP8':10, 'GP9':11,
          'GP10':12, 'GP11':13, 'GP12':14, 'GP13':15}
WIRE_RGB = {'GP5':(200,40,40), 'GP7':(230,120,20), 'GP8':(170,150,20), 'GP9':(40,160,60),
            'GP10':(40,90,200), 'GP11':(130,60,190), 'GP12':(140,90,50), 'GP13':(90,90,90)}

# ── header pad positions (board frame), recomputed from the matrix geometry ─────
def node_min(c):
    return min(g.node_of(by) for (p,k,nc,gp,by) in g.ROWS if c < nc)
HEADER = {}
for (p,k,nc,gp,by) in g.ROWS:
    HEADER[gp] = T(2.1, by)                       # row nets exit on the bus strip
for (c,gp) in g.COLS:
    HEADER[gp] = T(g.xgap(c), node_min(c))        # col nets exit on the node strip

# ── Pico placement (below the board, floating = the separate module) ───────────
PICO_X0, PICO_Y0, PICO_ROW = 0.7, 3.2, 0.7        # left col x, top y, row spacing
def pico_pin_xy(pin):                              # left column, pin N -> point
    return (PICO_X0, round(PICO_Y0 + 0.1*(pin-1), 2))

SWITCH_BODIES = []   # (x0,y0,x1,y1)
WIRES = []           # (gp, rgb, [p0,p1,p2,p3])

# ── component emitters (final board-frame coords, no transpose) ────────────────
def rect(name, x0, y0, x1, y1, fill, border=(0,0,0), alpha=45):
    C.append(f'''<org.diylc.components.shapes.Rectangle>
  <name>{name}</name><alpha>{alpha}</alpha><value></value>
  <controlPoints><java.awt.Point x="{x0}" y="{y0}"/><java.awt.Point x="{x1}" y="{y1}"/></controlPoints>
  <firstPoint x="{x0}" y="{y0}"/><secondPoint x="{x1}" y="{y1}"/>
  <color>{color(*fill)}</color><borderColor>{color(*border)}</borderColor>
  <borderThickness>{size(0.2,'mm')}</borderThickness><edgeRadius>{size(1.0,'mm')}</edgeRadius>
</org.diylc.components.shapes.Rectangle>''')

def wire(name, rgb, pts):
    p = ''.join(f'<java.awt.Point x="{x}" y="{y}"/>' for (x,y) in pts)
    C.append(f'''<org.diylc.components.connectivity.HookupWire>
  <name>{name}</name><alpha>127</alpha>
  <controlPoints>{p}</controlPoints>
  <color>{color(*rgb)}</color><pointCount>FOUR</pointCount><gauge>_20</gauge>
</org.diylc.components.connectivity.HookupWire>''')

def dil(name, value, x0, y0, row, npins):
    per = npins // 2
    pts = [(x0, round(y0+0.1*i,2)) for i in range(per)] + \
          [(round(x0+row,2), round(y0+0.1*i,2)) for i in range(per)]
    cp = ''.join(f'<java.awt.Point x="{x}" y="{y}"/>' for (x,y) in pts)
    C.append(f'''<org.diylc.components.semiconductors.DIL__IC>
  <name>{name}</name><alpha>100</alpha><value>{value}</value><orientation>DEFAULT</orientation>
  <pinCount>_{npins}</pinCount><pinSpacing>{size(0.1,'in')}</pinSpacing><rowSpacing>{size(row,'in')}</rowSpacing>
  <controlPoints>{cp}</controlPoints><display>NAME</display>
  <bodyColor>{color(70,70,80)}</bodyColor><borderColor>{color(40,40,45)}</borderColor>
  <labelColor>{color(230,230,230)}</labelColor><indentColor>{color(40,40,45)}</indentColor>
  <displayNumbers>NO</displayNumbers>
</org.diylc.components.semiconductors.DIL__IC>''')

def raw_label(name, x, y, text, sz=7, halign='LEFT', rgb=(0,0,0)):
    C.append(f'''<org.diylc.components.misc.Label>
  <name>{name}</name><point x="{x}" y="{y}"/><text>{text}</text>{font(sz)}
  <color>{color(*rgb)}</color><center>false</center>
  <horizontalAlignment>{halign}</horizontalAlignment><verticalAlignment>CENTER</verticalAlignment>
  <orientation>DEFAULT</orientation>
</org.diylc.components.misc.Label>''')

# ── 1) switch bodies over every footprint ──────────────────────────────────────
for (p,k,nc,gp,by) in g.ROWS:
    ny = g.node_of(by)
    for c in range(nc):
        x = g.xL(c)
        pts = [T(x,by), T(round(x+g.HSPAN,2),by), T(x,ny), T(round(x+g.HSPAN,2),ny)]
        xs = [q[0] for q in pts]; ys = [q[1] for q in pts]
        b = (round(min(xs)-0.06,2), round(min(ys)-0.06,2),
             round(max(xs)+0.06,2), round(max(ys)+0.06,2))
        SWITCH_BODIES.append(b)
        rect(f'SW_{p}{k}{c+1}', *b, fill=(60,60,60))

# ── 2) the Pico ────────────────────────────────────────────────────────────────
dil('Pico', 'RP2040', PICO_X0, PICO_Y0, PICO_ROW, 40)
raw_label('PicoLbl', round(PICO_X0,2), round(PICO_Y0-0.18,2), 'Raspberry Pi Pico', 8, 'LEFT', (40,40,45))

# ── 3) 8 ribbon wires: header pad -> correct Pico pin ─────────────────────────
for gp, pin in GP_PIN.items():
    hx, hy = HEADER[gp]
    px, py = pico_pin_xy(pin)
    route = [(hx, hy), (hx, 2.95), (round(px-0.30,2), py), (px, py)]  # down below board, in from left
    WIRES.append((gp, WIRE_RGB[gp], route))
    wire(f'w_{gp}', WIRE_RGB[gp], route)

# ── 4) parts list ──────────────────────────────────────────────────────────────
PARTS = [
    'Parts:',
    '14x 6mm tactile push switch',
    '1x stripboard (>=24 x 24 holes, A-X)',
    '1x Raspberry Pi Pico (RP2040)',
    '8-way ribbon: board headers -> Pico',
    '~10x insulated jumper wire (col buses)',
]
for i, line in enumerate(PARTS):
    raw_label(f'parts{i}', 1.75, round(3.30 + 0.20*i, 2), line, 8 if i==0 else 7, 'LEFT')

if __name__ == '__main__':
    import sys
    path = sys.argv[1] if len(sys.argv) > 1 else 'mega_blastoise_matrix_full.diy'
    xml = g.project_xml(C, title='Mega Blastoise Button Matrix - full assembly',
                        desc='4x4 matrix + Raspberry Pi Pico + ribbon wiring, on stripboard.')
    with open(path, 'w') as f:
        f.write(xml)
    print(f'wrote {path}: {len(C)} components')
