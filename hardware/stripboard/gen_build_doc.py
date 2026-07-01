#!/usr/bin/env python3
"""Generate BUILD.md - step-by-step build instructions for the rev 3 single
board, derived directly from the verified model in gen_single_board.py so
every coordinate in the doc is exactly what the verifier passed.

Coordinates: column letter (A..X) + hole number (1..55), e.g. D23.
Each build section includes its own underside jumper wires (tagged 'sec' in
the model), so a section is fully done - wires and parts - before moving on.
"""
import gen_single_board as S
from collections import defaultdict

L = "ABCDEFGHIJKLMNOPQRSTUVWX"
def P(c, h): return f'{L[c]}{h}'

ok, msgs, stats = S.verify()
assert ok, f'model does not verify: {msgs}'

by_sec = defaultdict(list)
for j in S.JUMPERS: by_sec[j['sec']].append(j)
assert 'misc' not in by_sec, 'untagged jumpers: ' + str(by_sec['misc'])

def wire_table(w, jumpers):
    w('| net | wire (underside) |')
    w('|---|---|')
    by_net = defaultdict(list)
    for j in jumpers: by_net[j['net']].append(j)
    for net in sorted(by_net, key=lambda n: (not n.startswith('GP'),
                      int(n[2:]) if n.startswith('GP') else 0, n)):
        runs = '  ,  '.join(f"{P(*j['a'])} -> {P(*j['b'])}" for j in by_net[net])
        w(f'| **{net}** | {runs} |')
    w('')

out = []
w = out.append

w('# Build instructions - Mega Blastoise single board (rev 3)')
w('')
w('Generated from `gen_single_board.py` (netlist-verified: '
  f"{stats['pads']} pads, {stats['cuts']} cuts, {stats['jumpers']} jumpers, "
  f"{stats['nets']} nets, no opens/shorts). Open "
  '`mega_blastoise_single_board.diy` in DIYLC alongside this - same layout.')
w('')
w('**Coordinates** are `<column letter><hole number>`: columns A..X left to')
w('right, holes 1..55 top to bottom, looking at the **component (top) side**')
w('with the copper strips running vertically underneath. P2 sits at the top')
w('edge, P1 at the bottom.')
w('')
w('Each section below is self-contained: solder its jumper wires (thin')
w('insulated wire on the UNDERSIDE, soldered only at the two end holes), then')
w('its components, then move to the next section.')
w('')

w('## 0. Parts and tools')
w('')
w('- 1x stripboard, 24 strips x 55 holes (A..X)')
w('- 1x Raspberry Pi Pico + 2x20 male headers')
w('- 14x 6 mm tactile switch (legs on a 0.2" x 0.3" footprint)')
w('- 2x SSD1306 0.96" OLED (4-pin header: GND VCC SCL SDA)')
w('- 2x WS2812B LED strip (off-board; 3 wires each)')
w('- MB102 breadboard supply (5 V out) + 2 wires to the board')
w('- ~%d insulated jumper wires, thin (30 AWG wire-wrap wire is ideal)' % stats['jumpers'])
w('- spot face cutter (or 3 mm drill bit), soldering iron, multimeter')
w('')

w('## 1. Mark and cut the tracks (%d cuts)' % stats['cuts'])
w('')
w('Do ALL cuts first - several strips are shared between sections, so cutting')
w('later risks slicing under an already-soldered part.')
w('')
w('Cuts are ON a hole (the hole is destroyed). To avoid mirror-image mistakes:')
w('push a pin through each listed hole from the component side, then cut the')
w('copper around the pin hole on the strip side. Verify every cut afterwards')
w('with a continuity meter (probe the two holes either side of the cut).')
w('')
cuts = sorted(S.CUTS)
line46 = sorted(c for (c,h) in cuts if h == 46 and 2 <= c <= 21)
line50 = sorted(c for (c,h) in cuts if h == 50 and 2 <= c <= 21)
line23 = sorted(c for (c,h) in cuts if h == 23)
rest = [(c,h) for (c,h) in cuts
        if not (h == 46 and 2 <= c <= 21) and not (h == 50 and 2 <= c <= 21)
        and h != 23]
w(f'- **Pico line 1** - hole 46 across cols {L[line46[0]]}..{L[line46[-1]]} (20 cuts)')
w(f'- **Pico line 2** - hole 50 across cols {L[line50[0]]}..{L[line50[-1]]} (20 cuts)')
w(f'- **P1/P2 border** - hole 23 on cols {", ".join(L[c] for c in line23)} ({len(line23)} cuts)')
w('- **Singles** - ' + ', '.join(P(c,h) for (c,h) in sorted(rest, key=lambda x:(x[1],x[0]))))
w('')

w('## 2. Pico and power rails')
w('')
w('Wires first (they feed the rails from the Pico):')
w('')
wire_table(w, by_sec['pico'])
w('Then the Pico, on the TOP side, pin rows at holes 47 and 54, cols C..V:')
w('')
w('- Orientation: **USB connector points LEFT** (toward col A).')
w(f'- Pin 1 (GP0) is the **bottom-left** pin at {P(*S.pico_col(1))}.')
w(f'- Top pin row = hole {S.PTOP} (pins 40..21 left to right); bottom row = hole {S.PBOT} (pins 1..20).')
w('- VBUS (top-left pin, C47) is intentionally not connected.')
w('- Solder headers to the Pico first, drop it in, check it sits between the')
w('  two cut lines (holes 46 and 50 isolate its pin rows), then solder.')
w('')
w('Rails after this section: **B = GND**, **W = 3V3**, **X = 5V** (X stays')
w('dead until the MB102 section).')
w('')

w('## 3. Button matrix (%d wires, 14 switches)' % len(by_sec['switches']))
w('')
w('All the matrix nets - row nets GP5/7/8/9 and column nets GP10-13:')
w('')
wire_table(w, by_sec['switches'])
w('Then the switches. Each spans two strips 3 columns apart; all four legs')
w('get soldered:')
w('')
w('| player | button | left legs (col, holes) | right legs (col, holes) |')
w('|---|---|---|---|')
for (nm, rn, cn, node, bus, hb) in S.BUTTONS:
    player = 'P2' if hb < 23 else 'P1'
    w(f'| {player} | **{nm}** | {L[node]}{hb} + {L[node]}{hb+2} | {L[bus]}{hb} + {L[bus]}{hb+2} |')
w('')
w("P2's buttons are labeled from P2's seat: their M1 is at the board's")
w('bottom-right of their screen (rows 20-22), their party row S1 S2 S3 reads')
w('right-to-left in board coordinates. Follow the table and it comes out right.')
w('')

w('## 4. OLED screens (%d wires, 2 modules)' % len(by_sec['oled']))
w('')
w('I2C signals plus each screen\'s 3V3/GND taps to the rails:')
w('')
wire_table(w, by_sec['oled'])
w('Then the headers:')
w('')
w('- **P1 OLED**: header at hole 27, screen hangs DOWN (rows 27-37).')
w('  Pin order left to right: GND=J27, VCC=K27, SCL=L27, SDA=M27.')
w('- **P2 OLED**: rotated 180 deg - header at hole 18, screen hangs UP')
w('  (rows 8-18). Pin order left to right: SDA=J18, SCL=K18, VCC=L18, GND=M18.')
w('- Double-check your module silkscreen: if its pin order is not GND-VCC-SCL-SDA,')
w('  tell the generator, do not improvise.')
w('')

w('## 5. LED strip connectors (%d wires, left edge)' % len(by_sec['led']))
w('')
w('DIN feeds from the Pico plus the 5V/GND taps for each connector:')
w('')
wire_table(w, by_sec['led'])
w('The strips are off-board; their 3 wires each solder into column A:')
w('')
w('- **LED strip P2** (top left edge): DIN=A2, 5V=A5, GND=A8.')
w('- **LED strip P1** (bottom left edge): DIN=A41, 5V=A44, GND=A47.')
w('')

w('## 6. MB102 supply input (%d wire)' % len(by_sec['mb102']))
w('')
wire_table(w, by_sec['mb102'])
w('- **MB102 supply** (bottom right): "-" to W53, "+" to X53.')
w('- Power plan: MB102 5V drives the LED strips via the X rail; the Pico runs')
w('  from its own USB; grounds are shared through the B rail.')
w('')

w('## 7. Test before power')
w('')
w('With the meter on continuity, no power applied:')
w('')
w('1. **Rails**: B2-B55 beeps (GND), W2-W46 beeps (3V3), X2-X55 beeps (5V).')
w('   W53 must NOT beep to W2 (cut at W52), but W53-B2 must beep (shared GND).')
w('2. **No rail shorts**: B-W, B-X, W-X all silent.')
w('3. **Every button**: hold it pressed, meter from its row net Pico pin to its')
w('   column net Pico pin - beeps pressed, silent released:')
w('')
w('| net | Pico pin (physical) | net | Pico pin (physical) |')
w('|---|---|---|---|')
w('| GP5 (P1 moves) | 7 | GP10 (col 1) | 14 |')
w('| GP7 (P1 party) | 10 | GP11 (col 2) | 15 |')
w('| GP8 (P2 moves) | 11 | GP12 (col 3) | 16 |')
w('| GP9 (P2 party) | 12 | GP13 (col 4) | 17 |')
w('')
w('4. **OLEDs**: SDA/SCL of each header to Pico pins - P1: M27-pin21(GP16),')
w('   L27-pin22(GP17); P2: J18-pin24(GP18), K18-pin25(GP19).')
w('5. Plug in USB only (no MB102): OLEDs must come up. Then add the MB102 and')
w('   the strips.')
w('')
w('## 8. Firmware note')
w('')
w("- P2's OLED is physically rotated 180 deg - the firmware needs to set the")
w('  SSD1306 segment-remap + COM-scan-direction flip for the P2 display.')
w('- Pin map is unchanged from the breadboard build (rows GP5/7/8/9, cols')
w('  GP10-13, OLEDs GP16/17 + GP18/19, strips GP20/GP22).')
w('')

with open('BUILD.md', 'w') as f:
    f.write('\n'.join(out) + '\n')
print(f'wrote BUILD.md ({len(out)} lines)')
