#!/usr/bin/env python3
"""Full-system DIYLC layout: every part of the Mega Blastoise console.

  - button matrix (14x 6mm tactile) on the stripboard      -> gen_matrix_diy.py
  - Raspberry Pi Pico (40-pin DIL stand-in)
  - 2x SSD1306 OLED, one per player (I2C0 / I2C1)
  - 2x WS2812B LED strip, one per player
  - 1x passive buzzer
  - the wiring between all of it (signal + power)

Pins come straight from the firmware (mega_blastoise_fw/src/main.rs):
  matrix rows GP5/7/8/9, cols GP10-13
  OLED P1  I2C0 SDA=GP16 SCL=GP17 ; OLED P2 I2C1 SDA=GP18 SCL=GP19
  LED  P1  GP20 ; LED P2 GP22 ; buzzer GP21
Pico physical pins (RP2040 board, GND every 5th pin):
  GP5=7  GP7=10 GP8=11 GP9=12 GP10=14 GP11=15 GP12=16 GP13=17  (left column)
  GP16=21 GP17=22 GP18=24 GP19=25 GP20=26 GP21=27 GP22=29
  3V3(OUT)=36  VBUS(5V)=40  GND=38                              (right column)

Only DIYLC component types with schemas proven against the sample corpus are
used (Rectangle, HookupWire, DIL__IC, SolderPad, Label) so the file loads.
"""
import gen_matrix_diy as g

C = g.comps
size, color, font, T = g.size, g.color, g.font, g.T

# ── Pico geometry ──────────────────────────────────────────────────────────────
PICO_X0, PICO_Y0, PICO_ROW = 0.7, 3.30, 0.7   # left-col x, top y, row spacing
def pin_xy(pin):
    if pin <= 20:  return (PICO_X0, round(PICO_Y0 + 0.1*(pin-1), 2))          # left col, top->bottom = 1..20
    return (round(PICO_X0+PICO_ROW,2), round(PICO_Y0 + 0.1*(40-pin), 2))      # right col, top->bottom = 40..21

# pin -> (label, rgb) for the pins we actually use
GP_PIN = {'GP5':7,'GP7':10,'GP8':11,'GP9':12,'GP10':14,'GP11':15,'GP12':16,'GP13':17}
WIRE_RGB = {'GP5':(200,40,40),'GP7':(230,120,20),'GP8':(170,150,20),'GP9':(40,160,60),
            'GP10':(40,90,200),'GP11':(130,60,190),'GP12':(140,90,50),'GP13':(90,90,90)}
C_SDA,C_SCL,C_3V3,C_5V,C_GND = (40,90,200),(20,150,200),(230,140,20),(210,40,40),(30,30,30)
C_LED1,C_LED2,C_BUZ = (40,160,60),(150,60,190),(150,120,20)

USED_PINS = {}   # pin -> (label, rgb)
for gp,pin in GP_PIN.items(): USED_PINS[pin] = (gp, WIRE_RGB[gp])
for pin,lab,rgb in [(21,'GP16 SDA',C_SDA),(22,'GP17 SCL',C_SCL),(24,'GP18 SDA',C_SDA),
                    (25,'GP19 SCL',C_SCL),(26,'GP20 DIN',C_LED1),(27,'GP21',C_BUZ),
                    (29,'GP22 DIN',C_LED2),(36,'3V3',C_3V3),(38,'GND',C_GND),(40,'VBUS 5V',C_5V)]:
    USED_PINS[pin] = (lab, rgb)

# ── data collected for the preview ─────────────────────────────────────────────
SWITCH_BODIES, WIRES, MODULE_RECTS, MODULE_PADS, MODULE_PAD_LABELS = [],[],[],[],[]

# ── emitters (final board-frame coords; no transpose) ──────────────────────────
def rect(name, x0,y0,x1,y1, fill, border=(0,0,0), alpha=60):
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
    WIRES.append((name, rgb, pts))

def solder(name, x, y, rgb):
    C.append(f'''<org.diylc.components.connectivity.SolderPad>
  <name>{name}</name><size>{size(0.09,'in')}</size><color>{color(*rgb)}</color>
  <point x="{x}" y="{y}"/><type>ROUND</type><holeSize>{size(0.8,'mm')}</holeSize><layer>_1</layer>
</org.diylc.components.connectivity.SolderPad>''')

def rlabel(name, x, y, text, sz=7, halign='LEFT', rgb=(0,0,0)):
    C.append(f'''<org.diylc.components.misc.Label>
  <name>{name}</name><point x="{x}" y="{y}"/><text>{text}</text>{font(sz)}
  <color>{color(*rgb)}</color><center>false</center>
  <horizontalAlignment>{halign}</horizontalAlignment><verticalAlignment>CENTER</verticalAlignment>
  <orientation>DEFAULT</orientation>
</org.diylc.components.misc.Label>''')

def dil(name, value, x0, y0, row, npins):
    per = npins//2
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

# a module = a body rect + a title + N pins on its left edge, each wired to a Pico pin
RBUS = round(PICO_X0+PICO_ROW+0.35, 2)   # vertical bus x, right of the Pico, for module wires
def module(tag, x0,y0,x1,y1, title, pins):
    rect(f'{tag}_body', x0,y0,x1,y1, fill=(35,40,55), alpha=70)
    rlabel(f'{tag}_t', round(x0+0.05,2), round(y0-0.13,2), title, 8, 'LEFT', (30,30,40))
    MODULE_RECTS.append((tag, x0,y0,x1,y1, title))
    n = len(pins); pad_x = round(x0+0.05,2)
    for i,(plabel, pico_pin, rgb) in enumerate(pins):
        py = round(y0 + (i+1)*(y1-y0)/(n+1), 2)
        solder(f'{tag}_p{i}', pad_x, py, rgb)
        rlabel(f'{tag}_l{i}', round(pad_x+0.10,2), py, plabel, 6, 'LEFT', rgb)
        MODULE_PADS.append((pad_x, py, rgb)); MODULE_PAD_LABELS.append((round(pad_x+0.10,2), py, plabel, rgb))
        px, pyp = pin_xy(pico_pin)
        wire(f'{tag}_w{i}', rgb, [(pad_x,py),(RBUS,py),(RBUS,pyp),(round(px+0.15,2),pyp)])

# ── 1) switch bodies over every footprint ──────────────────────────────────────
for (p,k,nc,gp,by) in g.ROWS:
    ny = g.node_of(by)
    for c in range(nc):
        x = g.xL(c)
        pts = [T(x,by), T(round(x+g.HSPAN,2),by), T(x,ny), T(round(x+g.HSPAN,2),ny)]
        xs=[q[0] for q in pts]; ys=[q[1] for q in pts]
        b = (round(min(xs)-0.06,2),round(min(ys)-0.06,2),round(max(xs)+0.06,2),round(max(ys)+0.06,2))
        SWITCH_BODIES.append(b); rect(f'SW_{p}{k}{c+1}', *b, fill=(60,60,60))

# ── 2) the Pico ────────────────────────────────────────────────────────────────
dil('Pico', 'RP2040', PICO_X0, PICO_Y0, PICO_ROW, 40)
rlabel('PicoLbl', round(PICO_X0,2), round(PICO_Y0-0.16,2), 'Raspberry Pi Pico', 8, 'LEFT', (40,40,45))
for pin,(lab,rgb) in USED_PINS.items():                    # label the used Pico pins
    px,py = pin_xy(pin)
    ha = 'RIGHT' if pin<=20 else 'LEFT'; dx = -0.10 if pin<=20 else 0.10
    rlabel(f'pin{pin}', round(px+dx,2), py, f'{lab} p{pin}', 6, ha, tuple(min(v+20,120) for v in rgb) if rgb==(30,30,30) else rgb)

# ── 3) matrix header pads -> left-column Pico pins ────────────────────────────
def node_min(c): return min(g.node_of(by) for (p,k,nc,gp,by) in g.ROWS if c<nc)
HEADER = {}
for (p,k,nc,gp,by) in g.ROWS: HEADER[gp] = T(2.1, by)
for (c,gp) in g.COLS:         HEADER[gp] = T(g.xgap(c), node_min(c))
for gp,pin in GP_PIN.items():
    hx,hy = HEADER[gp]; px,py = pin_xy(pin)
    wire(f'w_{gp}', WIRE_RGB[gp], [(hx,hy),(hx,2.95),(round(px-0.30,2),py),(px,py)])

# ── 4) the player modules ──────────────────────────────────────────────────────
module('OLED1', 2.30,3.35,3.25,3.95, 'OLED P1 (SSD1306 I2C0)',
       [('SDA GP16',21,C_SDA),('SCL GP17',22,C_SCL),('VCC 3V3',36,C_3V3),('GND',38,C_GND)])
module('OLED2', 2.30,4.15,3.25,4.75, 'OLED P2 (SSD1306 I2C1)',
       [('SDA GP18',24,C_SDA),('SCL GP19',25,C_SCL),('VCC 3V3',36,C_3V3),('GND',38,C_GND)])
module('LED1', 2.30,4.95,3.45,5.20, 'WS2812B strip P1',
       [('DIN GP20',26,C_LED1),('5V',40,C_5V),('GND',38,C_GND)])
module('LED2', 2.30,5.40,3.45,5.65, 'WS2812B strip P2',
       [('DIN GP22',29,C_LED2),('5V',40,C_5V),('GND',38,C_GND)])
module('BUZ', 1.80,5.45,2.15,5.75, 'Buzzer',
       [('GP21',27,C_BUZ),('GND',38,C_GND)])

# ── 5) parts list ──────────────────────────────────────────────────────────────
PARTS = [
    'Parts:',
    '1x Raspberry Pi Pico (RP2040)',
    '14x 6mm tactile switch (matrix, stripboard)',
    '2x SSD1306 OLED  - P1 I2C0 GP16/17, P2 I2C1 GP18/19',
    '2x WS2812B strip  - P1 GP20, P2 GP22 (5V)',
    '1x passive buzzer - GP21',
    'power: 3V3 -> OLEDs, 5V(VBUS) -> strips, common GND',
    'wire: 8-way ribbon + ~10 jumpers (col buses)',
]
for i,line in enumerate(PARTS):
    rlabel(f'parts{i}', 3.6, round(3.45+0.22*i,2), line, 8 if i==0 else 7, 'LEFT')

if __name__ == '__main__':
    import sys
    path = sys.argv[1] if len(sys.argv) > 1 else 'mega_blastoise_matrix_full.diy'
    xml = g.project_xml(C, title='Mega Blastoise - full system',
                        desc='Button matrix + Pico + 2 OLEDs + 2 WS2812B strips + buzzer, with wiring.')
    with open(path,'w') as f: f.write(xml)
    print(f'wrote {path}: {len(C)} components')
