#!/usr/bin/env python3
"""Generate a unique 1-bit 32x32 sprite for every Gen 1 move.

Reads the canonical move list (the keys of gen1_battle/data/rby_moves.json)
and emits mega_blastoise_core/src/move_sprites.rs: packed 1-bit bitmaps in
the same layout as the mon sprites in sprites.rs (row-major, MSB = leftmost
pixel, ready for embedded_graphics ImageRaw::<BinaryColor>).

Every move has a hand-specified ("bespoke") composition in the BESPOKE table
below: Surf is a curling wave, Earthquake is split ground, Swords Dance is
crossed swords, Substitute is the doll, and so on. Families of similar moves
(punches, kicks, beams, powders, coils) share parameterized scene helpers but
differ in silhouette. A tiny generic fallback exists only for ids missing
from the table (none today) and prints a warning when used.

Legibility rules applied throughout: bold silhouettes, >= 2px strokes for
primary shapes, at most 1-2 focal elements per icon, no load-bearing detail
smaller than 2x2 px.

Uniqueness is verified programmatically: all packed bitmaps must be pairwise
distinct or the script fails. If a collision ever happens, the later move
(in sorted id order) is deterministically re-rendered with a salt that adds
a small marker dot.

Usage:
    python3 scripts/gen_move_sprites.py [--preview sheet3x.png]
                                        [--preview1x sheet1x.png]

Requires Pillow.
"""

import argparse
import hashlib
import json
import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

SIDE = 32
BYTES_PER_ROW = SIDE // 8
SPRITE_BYTES = SIDE * SIDE // 8

REPO = Path(__file__).resolve().parent.parent
MOVES_JSON = REPO / "gen1_battle" / "data" / "rby_moves.json"
OUT_RS = REPO / "mega_blastoise_core" / "src" / "move_sprites.rs"

TAU = 2 * math.pi


# ── deterministic per-move bit source ────────────────────────────────────────

class Bits:
    """Deterministic bit stream seeded from a string (sha256, cycled)."""

    def __init__(self, key: str):
        self._data = hashlib.sha256(key.encode()).digest()
        self._pos = 0

    def take(self, n: int) -> int:
        val = 0
        for _ in range(n):
            byte = self._data[(self._pos // 8) % len(self._data)]
            val = (val << 1) | ((byte >> (7 - self._pos % 8)) & 1)
            self._pos += 1
        return val

    def rng(self, lo: int, hi: int) -> int:
        return lo + self.take(16) % (hi - lo + 1)


# ── primitives ───────────────────────────────────────────────────────────────

def L(d, pts, w=2, on=1):
    d.line(pts, fill=on, width=w)


def C(d, x, y, r, on=1):
    d.ellipse([x - r, y - r, x + r, y + r], fill=on)


def CO(d, x, y, r, w=2, on=1):
    d.ellipse([x - r, y - r, x + r, y + r], outline=on, width=w)


def P(d, pts, on=1):
    d.polygon(pts, fill=on)


def PO(d, pts, w=2, on=1):
    d.polygon(pts, outline=on, width=w)


def R(d, box, on=1):
    d.rectangle(box, fill=on)


def A(d, box, a0, a1, w=2, on=1):
    d.arc(box, a0, a1, fill=on, width=w)


def rot(pts, ang, cx=0.0, cy=0.0):
    ca, sa = math.cos(ang), math.sin(ang)
    return [(cx + (x - cx) * ca - (y - cy) * sa,
             cy + (x - cx) * sa + (y - cy) * ca) for x, y in pts]


def scale_to(norm, cx, cy, r, mirror=False):
    out = []
    for x, y in norm:
        if mirror:
            x = -x
        out.append((cx + x * r, cy + y * r))
    return out


# ── glyph library ────────────────────────────────────────────────────────────

def star(d, cx, cy, r, n=5, inner=0.45, ph=-math.pi / 2):
    poly = []
    for i in range(n * 2):
        rad = r if i % 2 == 0 else r * inner
        a = ph + math.pi * i / n
        poly.append((cx + rad * math.cos(a), cy + rad * math.sin(a)))
    P(d, poly)


def spark(d, x, y, r=2, w=2):
    L(d, [(x - r, y), (x + r, y)], w)
    L(d, [(x, y - r), (x, y + r)], w)


def arrow(d, p0, p1, w=2, hs=5):
    x0, y0 = p0
    x1, y1 = p1
    ang = math.atan2(y1 - y0, x1 - x0)
    sx = x1 - hs * 0.7 * math.cos(ang)
    sy = y1 - hs * 0.7 * math.sin(ang)
    L(d, [(x0, y0), (sx, sy)], w)
    left = (x1 + hs * math.cos(ang + 2.65), y1 + hs * math.sin(ang + 2.65))
    right = (x1 + hs * math.cos(ang - 2.65), y1 + hs * math.sin(ang - 2.65))
    P(d, [(x1, y1), left, right])


def bolt(d, cx, cy, h, wf=0.62):
    sy = h / 2.0
    sx = sy * wf
    norm = [(0.30, -1), (-0.42, 0.04), (-0.10, 0.04),
            (-0.32, 1), (0.42, -0.08), (0.08, -0.08)]
    P(d, [(cx + x * sx, cy + y * sy) for x, y in norm])


def flame(d, cx, cy, r):
    norm = [(0.02, -1.0), (0.30, -0.55), (0.18, -0.30), (0.62, -0.10),
            (0.55, 0.45), (0.0, 0.95), (-0.55, 0.45), (-0.62, -0.10),
            (-0.30, -0.42)]
    P(d, scale_to(norm, cx, cy, r))
    if r >= 7:
        inner = [(0.0, -0.05), (0.26, 0.32), (0.0, 0.60), (-0.26, 0.32)]
        P(d, scale_to(inner, cx, cy, r), on=0)


def droplet(d, cx, cy, r):
    d.ellipse([cx - 0.62 * r, cy - 0.25 * r, cx + 0.62 * r, cy + 0.95 * r], fill=1)
    P(d, scale_to([(0, -1.0), (0.5, 0.05), (-0.5, 0.05)], cx, cy, r))
    if r >= 6:
        d.ellipse([cx - 0.34 * r, cy + 0.12 * r, cx - 0.06 * r, cy + 0.44 * r],
                  fill=0)


def snowflake(d, cx, cy, r, w=2):
    for k in range(6):
        a = k * math.pi / 3
        L(d, [(cx, cy), (cx + r * math.cos(a), cy + r * math.sin(a))], w)
        if r >= 5:
            bx, by = cx + 0.55 * r * math.cos(a), cy + 0.55 * r * math.sin(a)
            for off in (0.6, -0.6):
                L(d, [(bx, by), (bx + 0.32 * r * math.cos(a + off),
                                 by + 0.32 * r * math.sin(a + off))], 1)
    C(d, cx, cy, max(1.4, 0.16 * r))


def leafg(d, cx, cy, r, ang=0.0):
    norm = [(0.0, -1.0), (0.55, -0.45), (0.62, 0.25), (0.0, 1.0),
            (-0.62, 0.25), (-0.55, -0.45)]
    P(d, [(cx + x * r, cy + y * r) for x, y in rot(norm, ang)])
    if r >= 7:
        rib = rot([(0.0, -0.8), (0.0, 0.85)], ang)
        L(d, [(cx + x * r, cy + y * r) for x, y in rib], 1, on=0)


def fist(d, cx, cy, r):
    d.rounded_rectangle([cx - 0.85 * r, cy - 0.35 * r, cx + 0.85 * r,
                         cy + 0.62 * r], radius=0.3 * r, fill=1)
    for kx in (-0.6, -0.2, 0.2, 0.6):
        C(d, cx + kx * r, cy - 0.36 * r, 0.24 * r)


def open_hand(d, cx, cy, r):
    """Palm-forward hand, fingers up. r ~ half height."""
    d.rounded_rectangle([cx - 0.55 * r, cy - 0.1 * r, cx + 0.55 * r,
                         cy + 0.9 * r], radius=0.25 * r, fill=1)
    for i, fx in enumerate((-0.38, 0.0, 0.38)):
        R(d, [cx + fx * r - 0.13 * r,
              cy - 0.95 * r + (0.15 * r if i != 1 else 0),
              cx + fx * r + 0.13 * r, cy + 0.1 * r])
    R(d, [cx + 0.55 * r, cy + 0.1 * r, cx + 0.85 * r, cy + 0.5 * r])  # thumb


def boot(d, cx, cy, s, flip=1):
    """L-shaped boot, toe pointing +x when flip=1. (cx,cy) = heel corner."""
    pts = [(cx - 0.45 * s * flip, cy - 1.4 * s),
           (cx + 0.25 * s * flip, cy - 1.4 * s),
           (cx + 0.25 * s * flip, cy - 0.5 * s),
           (cx + 1.15 * s * flip, cy - 0.5 * s),
           (cx + 1.15 * s * flip, cy + 0.3 * s),
           (cx - 0.45 * s * flip, cy + 0.3 * s)]
    P(d, pts)


def skullg(d, cx, cy, r):
    d.ellipse([cx - 0.72 * r, cy - 0.95 * r, cx + 0.72 * r, cy + 0.35 * r], fill=1)
    R(d, [cx - 0.4 * r, cy + 0.2 * r, cx + 0.4 * r, cy + 0.62 * r])
    er = max(1.4, 0.2 * r)
    C(d, cx - 0.3 * r, cy - 0.28 * r, er, on=0)
    C(d, cx + 0.3 * r, cy - 0.28 * r, er, on=0)
    if r >= 8:
        for tx in (-0.2, 0.2):
            L(d, [(cx + tx * r, cy + 0.3 * r), (cx + tx * r, cy + 0.6 * r)],
              1, on=0)


def bust(d, cx, cy, s, filled=True):
    """Head-and-shoulders silhouette."""
    if filled:
        C(d, cx, cy - 0.55 * s, 0.36 * s)
        d.pieslice([cx - 0.68 * s, cy - 0.15 * s, cx + 0.68 * s, cy + 1.2 * s],
                   180, 360, fill=1)
    else:
        CO(d, cx, cy - 0.55 * s, 0.36 * s, 2)
        A(d, [cx - 0.68 * s, cy - 0.15 * s, cx + 0.68 * s, cy + 1.2 * s],
          180, 360, 2)
        L(d, [(cx - 0.66 * s, cy + 0.52 * s), (cx + 0.66 * s, cy + 0.52 * s)], 2)


def ghostg(d, cx, cy, r):
    d.pieslice([cx - 0.65 * r, cy - 0.95 * r, cx + 0.65 * r, cy + 0.35 * r],
               180, 360, fill=1)
    R(d, [cx - 0.65 * r, cy - 0.32 * r, cx + 0.65 * r, cy + 0.55 * r])
    for sx in (-0.43, 0.0, 0.43):
        C(d, cx + sx * r, cy + 0.62 * r, 0.22 * r, on=0)
    for ex in (-0.28, 0.28):
        C(d, cx + ex * r, cy - 0.3 * r, max(1.5, 0.16 * r), on=0)


def bugg(d, cx, cy, r):
    d.ellipse([cx - 0.38 * r, cy - 0.15 * r, cx + 0.38 * r, cy + 0.9 * r], fill=1)
    d.ellipse([cx - 0.28 * r, cy - 0.6 * r, cx + 0.28 * r, cy - 0.02 * r], fill=1)
    for sx in (-1, 1):
        top = (cx + sx * 0.55 * r, cy - 1.05 * r)
        L(d, [(cx + sx * 0.14 * r, cy - 0.55 * r), top], 2)
        C(d, top[0], top[1], 1.3)
        for ly in (0.2, 0.55):
            L(d, [(cx + sx * 0.32 * r, cy + ly * r),
                  (cx + sx * 0.75 * r, cy + (ly + 0.22) * r)], 2)
    L(d, [(cx - 0.3 * r, cy + 0.35 * r), (cx + 0.3 * r, cy + 0.35 * r)], 1, on=0)


def cloud_fill(d, cx, cy, rx):
    ry = rx * 0.55
    C(d, cx - 0.55 * rx, cy + 0.1 * ry, 0.5 * rx)
    C(d, cx + 0.05 * rx, cy - 0.25 * ry, 0.62 * rx)
    C(d, cx + 0.55 * rx, cy + 0.15 * ry, 0.45 * rx)
    R(d, [cx - 0.55 * rx, cy + 0.15 * ry, cx + 0.55 * rx, cy + 0.8 * ry])


def crescent(d, cx, cy, r, dx, dy, r2f=0.75):
    C(d, cx, cy, r)
    C(d, cx + dx, cy + dy, r * r2f, on=0)


def heartg(d, cx, cy, r):
    C(d, cx - 0.45 * r, cy - 0.25 * r, 0.55 * r)
    C(d, cx + 0.45 * r, cy - 0.25 * r, 0.55 * r)
    P(d, [(cx - 0.95 * r, cy - 0.05 * r), (cx + 0.95 * r, cy - 0.05 * r),
          (cx, cy + r)])


def zg(d, x, y, s):
    L(d, [(x, y), (x + s, y)], 2)
    L(d, [(x + s, y), (x, y + s)], 2)
    L(d, [(x, y + s), (x + s, y + s)], 2)


def note(d, x, y):
    C(d, x, y, 2.3)
    L(d, [(x + 2, y), (x + 2, y - 8)], 2)
    L(d, [(x + 2, y - 8), (x + 6, y - 6)], 2)


def spiralg(d, cx, cy, r, turns=1.75, w=2, ph=0.0):
    prev = None
    steps = 48
    for i in range(steps + 1):
        t = i / steps
        a = ph + TAU * turns * t
        rad = r * (0.12 + 0.88 * t)
        p = (cx + rad * math.cos(a), cy + rad * math.sin(a))
        if prev is not None:
            L(d, [prev, p], w)
        prev = p


def tornado(d, cx, cy, s, loops=3):
    for i in range(loops):
        t = i / max(1, loops - 1)
        rx = s * (1.0 - 0.6 * t)
        y = cy - s * 0.9 + i * s * 0.75
        d.ellipse([cx - rx, y - s * 0.28, cx + rx, y + s * 0.28],
                  outline=1, width=2)
    ytail = cy - s * 0.9 + (loops - 1) * s * 0.75 + s * 0.3
    L(d, [(cx - 0.25 * s, ytail), (cx - 0.5 * s, min(ytail + s * 0.9, 30))], 2)


def impact_v(d, x, y, s=3, up=True):
    dy = -s if up else s
    L(d, [(x - s, y + dy), (x, y), (x + s, y + dy)], 2)


def speed_lines(d, lines, w=2):
    for p0, p1 in lines:
        L(d, [p0, p1], w)


def coin(d, cx, cy, r, w=2):
    CO(d, cx, cy, r, w)
    if r >= 6:
        CO(d, cx, cy, r - 3, 1)
        R(d, [cx - 1, cy - r * 0.4, cx + 1, cy + r * 0.4])


def qmark(d, cx, cy, r):
    A(d, [cx - r, cy - r, cx + r, cy + r], 135, 45, 3)
    ex = cx + r * math.cos(math.radians(45))
    ey = cy + r * math.sin(math.radians(45))
    L(d, [(ex, ey), (cx, cy + r + 3)], 3)
    C(d, cx, cy + r + 7, 1.8)


def eyeg(d, cx, cy, rx, ry, pupil=2.6):
    d.ellipse([cx - rx, cy - ry, cx + rx, cy + ry], outline=1, width=2)
    C(d, cx, cy, pupil)


def sword(d, tip, base, guard_frac=0.28, w=3):
    """Blade from base to tip with crossguard and pommel."""
    x0, y0 = base
    x1, y1 = tip
    L(d, [(x0, y0), (x1, y1)], w)
    gx = x0 + (x1 - x0) * guard_frac
    gy = y0 + (y1 - y0) * guard_frac
    ang = math.atan2(y1 - y0, x1 - x0) + math.pi / 2
    L(d, [(gx + 4 * math.cos(ang), gy + 4 * math.sin(ang)),
          (gx - 4 * math.cos(ang), gy - 4 * math.sin(ang))], 2)
    C(d, x0, y0, 1.8)


def crackline(d, x, ytop, ybot, on=0, w=2, step=4, amp=3):
    path = [(x, ytop)]
    y, xx, flip = ytop, x, 1
    while y < ybot:
        y += step
        xx += flip * amp
        flip = -flip
        path.append((xx, min(y, ybot)))
    L(d, path, w, on=on)


def dartg(d, tail, tip, w=3, hs=6):
    """Thick dart/needle: shaft plus solid triangular head."""
    arrow(d, tail, tip, w=w, hs=hs)


def sc_powder(d, emitter):
    """Falling dust plus an emitter drawn at top."""
    for x, y in ((10, 17), (16, 21), (22, 16), (13, 26), (20, 27), (25, 22)):
        C(d, x, y, 1.4)
    emitter(d)


# ── bespoke compositions ─────────────────────────────────────────────────────

def s_surf(d, B):
    P(d, [(1, 25), (5, 23), (10, 25), (15, 23), (20, 25), (25, 23), (30, 25),
          (30, 30), (1, 30)])
    C(d, 13, 13, 10)
    C(d, 16.5, 16.5, 6.8, on=0)
    for fx, fy in ((21, 5.5), (24.5, 8.5), (26, 13)):
        C(d, fx, fy, 2.2)
    C(d, 28, 18, 1.4)


def s_waterfall(d, B):
    L(d, [(2, 6), (20, 6)], 3)
    for x in (5, 10, 15):
        L(d, [(x, 8), (x, 23)], 2)
    d.ellipse([2, 24, 24, 30], fill=1)
    A(d, [18, 16, 30, 28], 250, 20, 2)
    C(d, 27, 15, 1.5)
    C(d, 24, 10, 1.3)


def s_watergun(d, B):
    C(d, 6, 11, 3.5)
    L(d, [(9, 12), (25, 19)], 3)
    for x, y in ((27, 17), (28, 21), (25, 23)):
        C(d, x, y, 1.6)


def s_hydropump(d, B):
    CO(d, 6, 6, 3.5, 2)
    L(d, [(9, 8), (27, 16)], 3)
    L(d, [(8, 9), (22, 27)], 3)
    for x, y in ((29, 14), (29, 19), (24, 29), (19, 28)):
        C(d, x, y, 1.6)


def s_bubble(d, B):
    CO(d, 11, 12, 6.5, 2)
    C(d, 9, 10, 1.4)
    CO(d, 23, 8, 3.8, 2)
    CO(d, 22, 22, 4.8, 2)
    C(d, 21, 21, 1.2)


def s_bubblebeam(d, B):
    L(d, [(3, 12), (18, 15)], 2)
    L(d, [(3, 20), (18, 17)], 2)
    CO(d, 23, 12, 3.6, 2)
    CO(d, 27, 20, 3.0, 2)
    CO(d, 21, 24, 2.4, 2)


def s_clamp(d, B):
    # open clam shell (side view), mouth to the right, snapping shut
    C(d, 13, 16, 11)
    P(d, [(13, 16), (34, 3), (34, 29)], on=0)
    for a_deg in (135, 180, 225):
        a = math.radians(a_deg)
        L(d, [(13 + 3 * math.cos(a), 16 + 3 * math.sin(a)),
              (13 + 11 * math.cos(a), 16 + 11 * math.sin(a))], 1, on=0)
    impact_v(d, 27, 10, 3, up=False)
    impact_v(d, 27, 22, 3)


def s_crabhammer(d, B):
    crescent(d, 17, 11, 9, 4, 4, 0.62)
    P(d, [(9, 16), (16, 21), (7, 22)])
    star(d, 9, 26, 4.5, 5)


def s_withdraw(d, B):
    d.pieslice([4, 8, 28, 30], 180, 360, fill=1)
    L(d, [(4, 19), (28, 19)], 2, on=0)
    for x in (10, 16, 22):
        L(d, [(16, 9), (x, 19)], 1, on=0)
    L(d, [(2, 24), (30, 24)], 2)


def s_thunderbolt(d, B):
    bolt(d, 16, 15, 26, 0.72)
    impact_v(d, 10, 30, 3)
    impact_v(d, 22, 30, 3)


def s_thunder(d, B):
    cloud_fill(d, 16, 7, 12)
    bolt(d, 16, 21, 17, 0.6)
    spark(d, 5, 18, 2)
    spark(d, 27, 20, 2)


def s_thundershock(d, B):
    bolt(d, 10, 10, 12, 0.6)
    bolt(d, 21, 21, 12, 0.6)
    spark(d, 25, 7, 2)
    spark(d, 6, 25, 2)


def s_thunderwave(d, B):
    bolt(d, 16, 15, 13, 0.62)
    A(d, [6, 5, 26, 25], 300, 240, 2)
    A(d, [2, 1, 30, 29], 320, 220, 2)


def s_thunderpunch(d, B):
    fist(d, 13, 21, 7.5)
    bolt(d, 23, 9, 13, 0.6)
    spark(d, 6, 8, 2)


def s_ember(d, B):
    flame(d, 16, 18, 9)
    C(d, 8, 8, 1.5)
    C(d, 24, 6, 1.7)
    C(d, 26, 12, 1.3)


def s_fireblast(d, B):
    tips = [(16, 3), (3, 11), (29, 11), (7, 28), (25, 28)]
    for tx, ty in tips:
        L(d, [(16, 15), (tx, ty)], 3)
        C(d, tx, ty, 2.4)
    C(d, 16, 15, 3)


def s_flamethrower(d, B):
    R(d, [2, 13, 6, 19])
    P(d, [(6, 15), (26, 5), (30, 16), (26, 27), (6, 17)])
    P(d, [(12, 15.5), (24, 10), (26, 16), (24, 22), (12, 16.5)], on=0)
    P(d, [(16, 15.5), (24, 13), (24, 19), (16, 16.5)])


def s_firepunch(d, B):
    fist(d, 14, 21, 7.5)
    flame(d, 22, 8, 6.5)


def s_firespin(d, B):
    flame(d, 16, 16, 8)
    A(d, [3, 3, 29, 29], 320, 140, 2)
    A(d, [0, 6, 26, 32], 150, 240, 2)


def s_absorb(d, B):
    leafg(d, 13, 16, 7.5, -0.5)
    arrow(d, (29, 4), (20, 11), 2)
    arrow(d, (29, 28), (20, 21), 2)


def s_megadrain(d, B):
    leafg(d, 16, 16, 8, -0.5)
    arrow(d, (2, 2), (9, 9), 2)
    arrow(d, (30, 2), (23, 9), 2)
    arrow(d, (2, 30), (9, 23), 2)
    arrow(d, (30, 30), (23, 23), 2)


def s_leechseed(d, B):
    d.ellipse([10, 18, 22, 27], fill=1)
    L(d, [(16, 18), (16, 11)], 2)
    leafg(d, 12, 8, 4, -1.0)
    leafg(d, 20, 8, 4, 1.0)
    arrow(d, (4, 28), (9, 24), 2, 4)
    arrow(d, (28, 28), (23, 24), 2, 4)


def s_razorleaf(d, B):
    leafg(d, 11, 11, 6.5, -0.9)
    leafg(d, 22, 20, 6.5, -0.9)
    speed_lines(d, [((2, 20), (9, 18)), ((13, 28), (20, 26)),
                    ((2, 27), (8, 26))], 1)


def s_vinewhip(d, B):
    L(d, [(3, 29), (7, 20), (14, 13), (22, 10)], 3)
    L(d, [(22, 10), (28, 6)], 2)
    spark(d, 29, 3, 2)


def s_petaldance(d, B):
    for k in range(5):
        a = -math.pi / 2 + k * TAU / 5
        C(d, 16 + 8 * math.cos(a), 15 + 8 * math.sin(a), 3.4)
    C(d, 16, 15, 2.2)
    A(d, [2, 1, 30, 29], 250, 320, 1)
    A(d, [2, 1, 30, 29], 70, 140, 1)


def s_solarbeam(d, B):
    C(d, 7, 7, 4.5)
    for k in range(8):
        a = k * TAU / 8
        L(d, [(7 + 6 * math.cos(a), 7 + 6 * math.sin(a)),
              (7 + 9 * math.cos(a), 7 + 9 * math.sin(a))], 2)
    P(d, [(11, 13), (15, 9), (30, 24), (26, 28)])


def s_growth(d, B):
    L(d, [(13, 28), (13, 13)], 2)
    leafg(d, 8, 10, 4.5, -1.0)
    leafg(d, 18, 10, 4.5, 1.0)
    L(d, [(5, 28), (21, 28)], 2)
    arrow(d, (27, 24), (27, 8), 2, 5)


def s_sleeppowder(d, B):
    def emitter(dd):
        d.pieslice([8, 3, 20, 13], 0, 180, fill=1)
        L(d, [(14, 3), (14, 7)], 2)
    sc_powder(d, emitter)
    zg(d, 24, 4, 5)


def s_stunspore(d, B):
    def emitter(dd):
        star(d, 14, 8, 5.5, 6, 0.5)
    sc_powder(d, emitter)
    bolt(d, 27, 7, 9, 0.6)


def s_poisonpowder(d, B):
    def emitter(dd):
        d.pieslice([8, 3, 20, 13], 0, 180, fill=1)
        L(d, [(14, 3), (14, 7)], 2)
        C(d, 11.5, 7.5, 1.3, on=0)
        C(d, 16.5, 7.5, 1.3, on=0)
    sc_powder(d, emitter)
    CO(d, 26, 7, 3, 2)


def s_spore(d, B):
    d.pieslice([8, 2, 24, 14], 180, 360, fill=1)
    R(d, [13, 8, 19, 13])
    sc_powder(d, lambda dd: None)


def s_icebeam(d, B):
    L(d, [(2, 7), (8, 12), (13, 8), (18, 13)], 3)
    snowflake(d, 23, 19, 7, 2)


def s_blizzard(d, B):
    for y in (7, 16, 25):
        L(d, [(2, y + 1), (12, y), (20, y + 1.5), (29, y)], 2)
    C(d, 9, 11.5, 6.5, on=0)
    snowflake(d, 9, 11.5, 5.5, 2)
    C(d, 23, 20.5, 6.5, on=0)
    snowflake(d, 23, 20.5, 5.5, 2)


def s_aurorabeam(d, B):
    for off in (-3, 0, 3):
        L(d, [(3, 10 + off), (12, 13 + off), (21, 9 + off), (29, 12 + off)], 2)
    snowflake(d, 6, 24, 5, 2)


def s_icepunch(d, B):
    fist(d, 14, 21, 7.5)
    snowflake(d, 23, 8, 6, 2)


def s_haze(d, B):
    R(d, [2, 6, 22, 9])
    R(d, [8, 14, 29, 17])
    R(d, [4, 22, 25, 25])
    C(d, 26, 7.5, 1.5)
    C(d, 4, 15.5, 1.5)
    C(d, 28.5, 23.5, 1.5)


def s_mist(d, B):
    C(d, 9, 12, 5, on=1)
    C(d, 16, 9, 6.5, on=1)
    C(d, 23, 12, 5, on=1)
    R(d, [9, 13, 23, 16])
    C(d, 9, 12, 3, on=0)
    C(d, 16, 9, 4.5, on=0)
    C(d, 23, 12, 3, on=0)
    R(d, [11, 13, 21, 14], on=0)
    for x, y in ((8, 22), (15, 25), (22, 22), (27, 27), (11, 29)):
        C(d, x, y, 1.4)


def s_counter(d, B):
    L(d, [(18, 4), (18, 28)], 3)
    arrow(d, (3, 10), (15, 13), 2, 4)
    arrow(d, (15, 21), (3, 25), 3, 6)


def s_karatechop(d, B):
    d.rounded_rectangle([13, 2, 19, 17], radius=2, fill=1)
    for y in (6, 9, 12):
        L(d, [(14, y), (18, y)], 1, on=0)
    R(d, [4, 22, 13, 25])
    R(d, [19, 22, 28, 25])
    impact_v(d, 16, 21, 3)
    impact_v(d, 16, 27, 3, up=False)


def s_doublekick(d, B):
    boot(d, 8, 10, 4.5, 1)
    boot(d, 19, 24, 4.5, 1)
    spark(d, 28, 6, 2)
    spark(d, 28, 21, 2)


def s_megakick(d, B):
    boot(d, 8, 20, 6.5, 1)
    star(d, 26, 20, 5, 5)
    speed_lines(d, [((3, 27), (10, 27)), ((2, 30), (12, 30))], 2)


def s_jumpkick(d, B):
    L(d, [(6, 28), (13, 18), (23, 14)], 3)
    boot(d, 24, 15, 3.5, 1)
    speed_lines(d, [((3, 20), (7, 24)), ((7, 16), (10, 20))], 2)


def s_highjumpkick(d, B):
    L(d, [(5, 29), (11, 16), (21, 7)], 3)
    boot(d, 22, 8, 3.5, 1)
    speed_lines(d, [((3, 16), (6, 21)), ((7, 10), (10, 15)),
                    ((12, 5), (14, 10))], 2)


def s_lowkick(d, B):
    L(d, [(2, 26), (30, 26)], 2)
    L(d, [(4, 24), (14, 22), (24, 19)], 3)
    boot(d, 25, 19, 3, 1)
    A(d, [6, 12, 28, 30], 220, 300, 1)
    star(d, 28, 12, 3, 5)


def s_rollingkick(d, B):
    A(d, [5, 5, 27, 27], 80, 350, 2)
    boot(d, 24, 10, 4, 1)
    speed_lines(d, [((2, 14), (5, 14)), ((3, 20), (6, 20))], 2)


def s_seismictoss(d, B):
    A(d, [4, 4, 28, 28], 165, 320, 2)
    bust(d, 8, 25, 7, True)
    d.pieslice([18, 3, 26, 10], 0, 180, fill=1)
    C(d, 22, 12, 2.4)
    arrow(d, (28, 18), (27, 24), 2, 5)


def s_submission(d, B):
    L(d, [(8, 5), (8, 19), (22, 19)], 4)
    L(d, [(24, 27), (24, 13), (10, 13)], 4)
    spark(d, 28, 5, 2)


def s_strength(d, B):
    L(d, [(6, 24), (13, 11)], 4)
    C(d, 13, 13, 4)
    L(d, [(13, 11), (24, 15)], 4)
    C(d, 25, 15, 3.2)
    spark(d, 27, 5, 2.5)


def s_toxic(d, B):
    skullg(d, 16, 11, 9)
    for x in (11, 16, 21):
        L(d, [(x, 18), (x, 24)], 2)
        C(d, x, 26, 1.6)
    A(d, [8, 26, 24, 32], 180, 360, 1)


def s_acid(d, B):
    R(d, [4, 21, 28, 25])
    for x in (10, 17, 24):
        C(d, x, 21, 2, on=0)
    droplet(d, 10, 9, 4)
    droplet(d, 21, 6, 3.5)
    spark(d, 28, 15, 1.5, 1)


def s_acidarmor(d, B):
    PO(d, [(16, 3), (26, 7), (26, 15), (16, 24), (6, 15), (6, 7)], 2)
    L(d, [(16, 6), (16, 21)], 2)
    for x in (11, 21):
        L(d, [(x, 22), (x, 27)], 2)
        C(d, x, 29, 1.4)


def s_poisongas(d, B):
    cloud_fill(d, 10, 11, 8)
    C(d, 8, 10, 1.4, on=0)
    C(d, 12.5, 10, 1.4, on=0)
    cloud_fill(d, 23, 22, 7)
    speed_lines(d, [((16, 29), (22, 29)), ((3, 22), (7, 22))], 1)


def s_poisonsting(d, B):
    dartg(d, (5, 5), (24, 24), 3, 7)
    L(d, [(4, 9), (8, 5)], 2)
    L(d, [(8, 12), (11, 9)], 2)
    droplet(d, 28, 29, 2.5)


def s_sludge(d, B):
    C(d, 12, 9, 5)
    C(d, 19, 8, 5.5)
    C(d, 15, 13, 6)
    for x, y2 in ((10, 21), (16, 24), (22, 20)):
        L(d, [(x, 16), (x, y2)], 3)
        C(d, x, y2 + 1, 1.7)
    d.ellipse([6, 27, 26, 31], fill=1)


def s_smog(d, B):
    cloud_fill(d, 16, 14, 13)
    C(d, 11, 12, 1.8, on=0)
    C(d, 20, 12, 1.8, on=0)
    L(d, [(10, 18), (13, 16.5), (16, 18), (19, 16.5), (22, 18)], 2, on=0)
    L(d, [(8, 26), (11, 29)], 2)
    L(d, [(22, 27), (24, 30)], 2)


def s_agility(d, B):
    for i, x in enumerate((5, 13, 21)):
        L(d, [(x, 8), (x + 6, 16), (x, 24)], i + 1)
    L(d, [(2, 28), (12, 28)], 1)


def s_amnesia(d, B):
    qmark(d, 17, 11, 7)
    C(d, 5, 12, 1.5)
    C(d, 4, 19, 1.2)


def s_barrier(d, B):
    R(d, [5, 7, 27, 27])
    for y in (13.5, 20.5):
        L(d, [(5, y), (27, y)], 2, on=0)
    L(d, [(16, 7), (16, 13.5)], 2, on=0)
    L(d, [(10.5, 13.5), (10.5, 20.5)], 2, on=0)
    L(d, [(21.5, 13.5), (21.5, 20.5)], 2, on=0)
    L(d, [(16, 20.5), (16, 27)], 2, on=0)


def s_confusion(d, B):
    spiralg(d, 16, 15, 11, 1.75, 2, ph=B.take(3) * 0.7)
    star(d, 27, 5, 3, 4)
    star(d, 5, 27, 2.6, 4)


def s_psychic(d, B):
    C(d, 16, 16, 2)
    A(d, [10, 10, 22, 22], 200, 120, 2)
    A(d, [5, 5, 27, 27], 20, 300, 2)
    A(d, [1, 1, 31, 31], 240, 160, 2)


def s_psybeam(d, B):
    P(d, [(3, 13), (29, 16), (3, 19)])
    CO(d, 13, 16, 4.5, 2)
    CO(d, 22, 16, 6, 2)


def s_psywave(d, B):
    C(d, 6, 16, 2.6)
    for tx, ty in ((29, 7), (29, 16), (29, 25)):
        pts = []
        for i in range(9):
            t = i / 8
            x = 6 + (tx - 6) * t
            y = 16 + (ty - 16) * t + 2.4 * math.sin(t * math.pi * 3)
            pts.append((x, y))
        L(d, pts, 2)


def s_hypnosis(d, B):
    eyeg(d, 11, 16, 8.5, 5.5, 2.8)
    A(d, [16, 8, 32, 24], 300, 60, 2)
    A(d, [20, 11, 34, 21], 300, 60, 2)


def s_meditate(d, B):
    C(d, 16, 8, 3.5)
    P(d, [(16, 12), (24, 23), (8, 23)])
    d.rounded_rectangle([8, 23, 24, 26], radius=1.5, fill=1)
    L(d, [(11, 30), (21, 30)], 2)
    spark(d, 4, 10, 2)
    spark(d, 28, 10, 2)


def s_kinesis(d, B):
    d.ellipse([4, 4, 14, 17], fill=1)
    d.ellipse([7, 7, 12, 14], fill=0)
    L(d, [(12, 15), (17, 20), (27, 22)], 3)
    A(d, [11, 10, 25, 24], 290, 20, 2)
    A(d, [13, 5, 31, 23], 300, 10, 1)


def s_lightscreen(d, B):
    PO(d, [(8, 5), (26, 10), (26, 25), (8, 20)], 2)
    L(d, [(12, 8), (12, 21)], 2)
    spark(d, 20, 15, 2)


def s_reflect(d, B):
    PO(d, [(6, 10), (24, 5), (24, 20), (6, 25)], 2)
    arrow(d, (28, 30), (26, 22), 2, 4)
    arrow(d, (25, 18), (30, 10), 2, 4)


def s_rest(d, B):
    zg(d, 19, 3, 8)
    zg(d, 12, 12, 6)
    zg(d, 6, 19, 4.5)
    A(d, [16, 22, 28, 32], 200, 340, 2)


def s_teleport(d, B):
    bust(d, 21, 16, 9, False)
    for x in (4, 8, 12):
        L(d, [(x, 12), (x, 14)], 2)
        L(d, [(x - 1, 22), (x - 1, 24)], 2)
    arrow(d, (5, 5), (13, 7), 2, 4)


def s_dreameater(d, B):
    crescent(d, 12, 15, 8.5, 4, -2, 0.8)
    C(d, 7, 21, 3, on=0)
    zg(d, 23, 4, 5)
    arrow(d, (26, 13), (19, 18), 2, 5)


def s_disable(d, B):
    star(d, 13, 13, 5, 5)
    CO(d, 16, 16, 11.5, 2)
    L(d, [(8, 8), (24, 24)], 3)


def s_barrage(d, B):
    for x, y in ((8, 9), (16, 14), (24, 19)):
        C(d, x, y, 3.6)
        L(d, [(x - 7, y - 3), (x - 4, y - 2)], 2)
    impact_v(d, 27, 26, 3)


def s_bide(d, B):
    C(d, 17, 20, 6.5)
    C(d, 21, 15, 3)
    arrow(d, (3, 8), (10, 14), 2, 4)
    arrow(d, (29, 6), (23, 11), 2, 4)
    arrow(d, (14, 29), (5, 29), 3, 6)


def s_bite(d, B):
    R(d, [5, 4, 27, 8])
    for x in (8, 14, 20, 26):
        P(d, [(x - 3, 8), (x + 1, 8), (x - 1, 14)])
    R(d, [5, 24, 27, 28])
    for x in (11, 17, 23):
        P(d, [(x - 3, 24), (x + 1, 24), (x - 1, 18)])


def s_bodyslam(d, B):
    d.ellipse([8, 7, 25, 19], fill=1)
    L(d, [(3, 26), (29, 26)], 2)
    speed_lines(d, [((6, 2), (6, 6)), ((26, 2), (26, 6))], 2)
    impact_v(d, 8, 24, 3)
    impact_v(d, 24, 24, 3)


def s_boneclub(d, B):
    L(d, [(8, 23), (21, 10)], 4)
    for dx, dy in ((-1.5, -3), (3, 1.5)):
        C(d, 8 + dx, 23 + dy, 2.4)
        C(d, 21 - dx, 10 - dy, 2.4)
    star(d, 27, 22, 4.5, 5)


def s_bonemerang(d, B):
    L(d, [(11, 9), (21, 9)], 3)
    for dx in (-2.7, 2.7):
        C(d, 10, 9 + dx, 2)
        C(d, 22, 9 + dx, 2)
    A(d, [5, 12, 27, 32], 15, 165, 2)
    arrow(d, (6, 21), (7, 16), 2, 5)


def s_cometpunch(d, B):
    fist(d, 19, 21, 7)
    star(d, 6, 6, 3.4, 5)
    star(d, 12, 11, 2.6, 5)
    speed_lines(d, [((3, 12), (8, 15)), ((5, 18), (10, 20))], 2)


def s_conversion(d, B):
    R(d, [5, 9, 13, 17])
    PO(d, [(19, 15), (27, 15), (27, 23), (19, 23)], 2)
    arrow(d, (14, 12), (24, 12), 2, 4)
    arrow(d, (18, 26), (8, 26), 2, 4)


def s_cut(d, B):
    P(d, [(5, 19), (21, 3), (25, 7), (9, 23)])
    P(d, [(4, 22), (8, 26), (5, 29), (1, 25)])
    L(d, [(12, 29), (27, 14)], 1)


def s_defensecurl(d, B):
    C(d, 16, 17, 9.5)
    A(d, [11, 12, 21, 22], 270, 180, 2, on=0)
    C(d, 22, 9, 3.4)
    C(d, 23, 8.4, 1.1, on=0)


def s_dizzypunch(d, B):
    fist(d, 16, 20, 7.5)
    star(d, 7, 7, 3, 5)
    star(d, 16, 4.5, 3, 5)
    star(d, 25, 7, 3, 5)


def s_doubleedge(d, B):
    P(d, [(3, 16), (8, 12), (24, 12), (29, 16), (24, 20), (8, 20)])
    L(d, [(9, 13.5), (23, 13.5)], 1, on=0)
    crackline(d, 15, 2, 11, on=1, w=2, step=3, amp=2)
    crackline(d, 17, 21, 30, on=1, w=2, step=3, amp=2)


def s_doubleslap(d, B):
    open_hand(d, 10, 12, 7)
    open_hand(d, 22, 18, 7)
    spark(d, 28, 5, 2)


def s_doubleteam(d, B):
    bust(d, 11, 15, 10, True)
    bust(d, 22, 15, 10, False)


def s_eggbomb(d, B):
    d.ellipse([8, 11, 22, 28], fill=1)
    C(d, 12, 17, 1.8, on=0)
    L(d, [(17, 11), (22, 6)], 2)
    star(d, 24, 4.5, 3.6, 5)
    speed_lines(d, [((2, 22), (6, 22)), ((3, 26), (7, 26))], 2)


def s_explosion(d, B):
    star(d, 16, 16, 8, 6, 0.5)
    for k in range(10):
        a = k * TAU / 10 + 0.3
        L(d, [(16 + 10 * math.cos(a), 16 + 10 * math.sin(a)),
              (16 + 15 * math.cos(a), 16 + 15 * math.sin(a))], 2)


def s_selfdestruct(d, B):
    C(d, 16, 16, 4)
    for k in range(8):
        a = k * TAU / 8
        P(d, [(16 + 6 * math.cos(a - 0.18), 16 + 6 * math.sin(a - 0.18)),
              (16 + 6 * math.cos(a + 0.18), 16 + 6 * math.sin(a + 0.18)),
              (16 + 14 * math.cos(a), 16 + 14 * math.sin(a))])


def s_flash(d, B):
    C(d, 16, 15, 3)
    for k in range(8):
        a = k * TAU / 8 + TAU / 16
        L(d, [(16 + 6 * math.cos(a), 15 + 6 * math.sin(a)),
              (16 + 13 * math.cos(a), 15 + 13 * math.sin(a))], 2)
    spark(d, 28, 28, 2)


def s_focusenergy(d, B):
    bust(d, 16, 20, 8, True)
    L(d, [(6, 18), (4, 8)], 2)
    L(d, [(26, 18), (28, 8)], 2)
    L(d, [(11, 12), (9, 4)], 2)
    L(d, [(21, 12), (23, 4)], 2)
    spark(d, 16, 5, 2)


def s_furyattack(d, B):
    dartg(d, (4, 27), (22, 9), 3, 7)
    star(d, 27, 5, 3, 5)
    star(d, 29, 12, 2.6, 5)
    star(d, 22, 2.5, 2.4, 5)


def s_furyswipes(d, B):
    for x in (4, 11, 18):
        A(d, [x, 4, x + 15, 28], 295, 65, 3)


def s_glare(d, B):
    P(d, [(3, 14), (14, 11), (14, 16), (3, 17)])
    P(d, [(29, 14), (18, 11), (18, 16), (29, 17)])
    L(d, [(4, 9), (14, 7)], 2)
    L(d, [(28, 9), (18, 7)], 2)
    L(d, [(10, 22), (12, 26)], 2)
    L(d, [(21, 22), (19, 26)], 2)


def s_growl(d, B):
    P(d, [(3, 12), (10, 15), (3, 20)])
    A(d, [8, 8, 22, 24], 300, 60, 2)
    A(d, [12, 5, 30, 27], 300, 60, 2)


def s_guillotine(d, B):
    L(d, [(6, 6), (26, 26)], 4)
    L(d, [(26, 6), (6, 26)], 4)
    CO(d, 6, 28, 2.6, 2)
    CO(d, 26, 28, 2.6, 2)
    P(d, [(4, 4), (10, 5), (5, 10)])
    P(d, [(28, 4), (22, 5), (27, 10)])


def s_gust(d, B):
    tornado(d, 16, 13, 7, 3)
    C(d, 25, 24, 1.5)
    C(d, 7, 26, 1.5)


def s_whirlwind(d, B):
    tornado(d, 13, 15, 9, 4)
    arrow(d, (20, 8), (28, 5), 2, 4)
    bust(d, 27, 10, 4, False)


def s_harden(d, B):
    pts = [(16 + 10 * math.cos(k * TAU / 6 - math.pi / 2),
            16 + 10 * math.sin(k * TAU / 6 - math.pi / 2)) for k in range(6)]
    PO(d, pts, 2)
    for k in (0, 2, 4):
        L(d, [pts[k], (16, 16)], 1)
    spark(d, 28, 5, 2)


def s_headbutt(d, B):
    R(d, [25, 4, 29, 28])
    C(d, 13, 15, 6.5)
    P(d, [(13, 9), (21, 12), (21, 18), (13, 21)])
    impact_v(d, 23, 9, 3)
    impact_v(d, 23, 22, 3, up=False)
    speed_lines(d, [((2, 12), (6, 12)), ((2, 18), (6, 18))], 2)


def s_hornattack(d, B):
    d.pieslice([1, 21, 19, 39], 180, 360, fill=1)
    P(d, [(6, 24), (16, 24), (25, 6), (10, 19)])
    star(d, 26, 4, 4, 5)


def s_horndrill(d, B):
    P(d, [(10, 4), (22, 4), (16, 29)])
    for y in (9, 15, 21):
        w = (29.0 - y) * 12 / 25 / 2
        L(d, [(16 - w, y + 1.2), (16 + w, y - 1.2)], 2, on=0)
    A(d, [6, 1, 26, 9], 200, 250, 2)
    A(d, [6, 0, 26, 8], 290, 340, 2)


def s_hyperbeam(d, B):
    C(d, 6, 16, 3.6)
    P(d, [(9, 13.5), (29, 7), (29, 25), (9, 18.5)])
    for a_deg in (120, 160, 200, 240):
        a = math.radians(a_deg)
        L(d, [(6 + 5 * math.cos(a), 16 + 5 * math.sin(a)),
              (6 + 9 * math.cos(a), 16 + 9 * math.sin(a))], 2)
    spark(d, 27, 3, 2)
    spark(d, 27, 29, 2)


def s_hyperfang(d, B):
    R(d, [5, 3, 27, 8])
    P(d, [(7, 8), (15, 8), (11, 22)])
    P(d, [(17, 8), (25, 8), (21, 22)])
    star(d, 16, 27, 3.5, 5)


def s_superfang(d, B):
    P(d, [(8, 3), (20, 3), (14, 25)])
    CO(d, 25, 23, 5, 2)
    d.pieslice([20, 18, 30, 28], 270, 90, fill=1)


def s_leechlife(d, B):
    bugg(d, 14, 16, 7.5)
    arrow(d, (28, 4), (21, 10), 2, 4)
    arrow(d, (29, 27), (22, 22), 2, 4)
    droplet(d, 27, 14, 2.5)


def s_leer(d, B):
    d.ellipse([6, 10, 26, 22], outline=1, width=2)
    C(d, 16, 16, 3.2)
    L(d, [(6, 8), (26, 12)], 3)


def s_lick(d, B):
    d.rounded_rectangle([12, 3, 20, 24], radius=3, fill=1)
    C(d, 16, 24, 4)
    L(d, [(16, 8), (16, 25)], 1, on=0)
    droplet(d, 25, 27, 2.5)
    A(d, [8, 0, 24, 8], 20, 160, 2)


def s_lovelykiss(d, B):
    P(d, [(6, 18), (11, 14), (15, 17), (19, 14), (24, 18)])
    d.pieslice([7, 15, 23, 26], 0, 180, fill=1)
    L(d, [(6, 18), (24, 18)], 1, on=0)
    heartg(d, 26, 7, 4)


def s_megapunch(d, B):
    fist(d, 14, 17, 9.5)
    A(d, [4, 2, 30, 32], 300, 40, 2)
    A(d, [8, 6, 26, 28], 305, 35, 2)


def s_metronome(d, B):
    C(d, 15, 25, 4.5)
    L(d, [(15, 22), (10, 7)], 3)
    C(d, 9.5, 5.5, 1.8)
    A(d, [6, 2, 28, 24], 280, 330, 2)
    A(d, [9, 5, 25, 21], 285, 325, 1)


def s_mimic(d, B):
    bust(d, 9, 17, 8, True)
    bust(d, 23, 17, 8, False)
    arrow(d, (12, 4), (20, 4), 2, 4)


def s_minimize(d, B):
    bust(d, 16, 13, 11, False)
    bust(d, 16, 20, 4.5, True)
    arrow(d, (3, 28), (9, 25), 2, 4)
    arrow(d, (29, 28), (23, 25), 2, 4)


def s_mirrormove(d, B):
    for y in range(3, 29, 5):
        L(d, [(16, y), (16, y + 2.5)], 2)
    arrow(d, (3, 10), (13, 10), 2, 4)
    arrow(d, (29, 22), (19, 22), 2, 4)


def s_nightshade(d, B):
    ghostg(d, 16, 16, 10)
    for a_deg in (200, 240, 300, 340):
        a = math.radians(a_deg)
        L(d, [(16 + 12 * math.cos(a), 14 + 12 * math.sin(a)),
              (16 + 15 * math.cos(a), 14 + 15 * math.sin(a))], 2)


def s_payday(d, B):
    coin(d, 13, 13, 9, 2)
    coin(d, 25, 24, 4, 2)
    coin(d, 17, 28, 3, 1)
    L(d, [(24, 16), (25, 19)], 1)


def s_pound(d, B):
    fist(d, 16, 12, 8)
    L(d, [(5, 25), (27, 25)], 2)
    impact_v(d, 10, 22, 3)
    impact_v(d, 22, 22, 3)
    L(d, [(16, 18), (16, 21)], 2)


def s_quickattack(d, B):
    arrow(d, (3, 16), (28, 16), 3, 7)
    speed_lines(d, [((2, 9), (14, 9)), ((6, 23), (18, 23))], 2)


def s_rage(d, B):
    CO(d, 14, 17, 9, 2)
    L(d, [(9, 13), (13, 15.5)], 2)
    L(d, [(19, 15.5), (23, 13)], 2)
    R(d, [10, 20, 18, 23])
    L(d, [(12.5, 20), (12.5, 23)], 1, on=0)
    L(d, [(15.5, 20), (15.5, 23)], 1, on=0)
    for a_deg in (200, 250, 290, 340):
        a = math.radians(a_deg)
        L(d, [(27 + 2 * math.cos(a), 6 + 2 * math.sin(a)),
              (27 + 5 * math.cos(a), 6 + 5 * math.sin(a))], 2)


def s_razorwind(d, B):
    crescent(d, 11, 12, 8, 5, 0, 0.8)
    crescent(d, 22, 21, 7, 4.5, 0, 0.8)
    speed_lines(d, [((2, 26), (9, 26)), ((4, 29), (13, 29))], 1)


def s_recover(d, B):
    R(d, [13, 5, 19, 27])
    R(d, [5, 13, 27, 19])
    spark(d, 6, 6, 2.5)
    spark(d, 26, 6, 2.5)
    spark(d, 6, 26, 2.5)
    spark(d, 26, 26, 2.5)


def s_roar(d, B):
    P(d, [(2, 6), (16, 6), (16, 12), (2, 15)])
    for x in (6, 11, 15):
        P(d, [(x - 2, 11), (x + 1, 11), (x, 16)])
    P(d, [(2, 25), (16, 21), (16, 25), (2, 28)])
    for r0 in (4, 9, 14):
        pts = []
        for i in range(7):
            a = math.radians(-40 + i * 80 / 6)
            rr = r0 + (2.2 if i % 2 else 0)
            pts.append((19 + rr * math.cos(a), 16 + rr * math.sin(a)))
        L(d, pts, 2)


def s_rockslide(d, B):
    def boulder(cx, cy, r):
        P(d, [(cx - 0.9 * r, cy - 0.2 * r), (cx - 0.3 * r, cy - r),
              (cx + 0.7 * r, cy - 0.7 * r), (cx + r, cy + 0.3 * r),
              (cx + 0.2 * r, cy + r), (cx - 0.8 * r, cy + 0.6 * r)])
    boulder(9, 8, 5)
    boulder(21, 13, 6)
    boulder(12, 23, 4.5)
    speed_lines(d, [((6, 1), (6, 3)), ((20, 3), (20, 5)),
                    ((28, 20), (28, 23))], 2)


def s_rockthrow(d, B):
    P(d, [(15, 6), (23, 4), (28, 10), (26, 17), (18, 18), (13, 12)])
    A(d, [2, 8, 26, 32], 100, 190, 2)
    arrow(d, (10, 9), (13, 7), 2, 4)


def s_sandattack(d, B):
    P(d, [(3, 29), (10, 25), (5, 22)])
    for tx, ty in ((20, 6), (25, 12), (28, 19)):
        L(d, [(8, 24), (tx, ty)], 1)
    for x, y in ((14, 17), (18, 12), (22, 16), (17, 21), (23, 8), (25, 22)):
        C(d, x, y, 1.3)
    d.ellipse([22, 1, 30, 7], outline=1, width=2)


def s_scratch(d, B):
    for x in (7, 14, 21):
        L(d, [(x, 4), (x + 5, 27)], 2)
        P(d, [(x + 4, 24), (x + 7, 29), (x + 3, 28)])


def s_screech(d, B):
    C(d, 5, 16, 2.5)
    for r0 in (6, 12, 18):
        pts = []
        for i in range(9):
            a = math.radians(-55 + i * 110 / 8)
            rr = r0 + (2.5 if i % 2 else 0)
            pts.append((5 + rr * math.cos(a), 16 + rr * math.sin(a)))
        L(d, pts, 2)


def s_sharpen(d, B):
    PO(d, [(16, 4), (26, 27), (6, 27)], 2)
    L(d, [(16, 4), (16, 27)], 1)
    L(d, [(24, 8), (28, 4)], 2)
    L(d, [(26, 13), (30, 9)], 2)


def s_sing(d, B):
    note(d, 8, 22)
    note(d, 17, 16)
    note(d, 25, 24)
    A(d, [0, 4, 22, 26], 300, 20, 1)


def s_skullbash(d, B):
    skullg(d, 13, 13, 8.5)
    speed_lines(d, [((2, 8), (6, 8)), ((1, 14), (5, 14)), ((2, 20), (6, 20))], 2)
    impact_v(d, 27, 13, 4)


def s_skyattack(d, B):
    P(d, [(8, 4), (16, 17), (12, 2)])
    P(d, [(24, 4), (16, 17), (20, 2)])
    P(d, [(14, 17), (18, 17), (16, 23)])
    for a_deg in (35, 75, 105, 145):
        a = math.radians(a_deg)
        L(d, [(16 + 8 * math.cos(a), 17 + 8 * math.sin(a)),
              (16 + 13 * math.cos(a), 17 + 13 * math.sin(a))], 2)


def s_slam(d, B):
    L(d, [(5, 4), (12, 12), (22, 21)], 4)
    L(d, [(3, 27), (29, 27)], 2)
    star(d, 25, 23, 4, 5)


def s_slash(d, B):
    L(d, [(4, 26), (26, 4)], 4)
    L(d, [(10, 30), (30, 10)], 3)
    spark(d, 5, 7, 2)


def s_smokescreen(d, B):
    C(d, 10, 18, 6)
    C(d, 18, 13, 7)
    C(d, 25, 20, 5)
    R(d, [10, 20, 25, 25])
    C(d, 7, 8, 1.6)
    C(d, 12, 5, 1.4)


def s_softboiled(d, B):
    d.ellipse([9, 6, 23, 24], fill=1)
    L(d, [(10, 15), (13, 17), (16, 14), (19, 17), (22, 15)], 2, on=0)
    heartg(d, 26, 26, 4)


def s_sonicboom(d, B):
    star(d, 6, 16, 3.5, 5)
    for x, r in ((12, 6), (18, 9), (24, 12)):
        A(d, [x - r, 16 - r, x + r, 16 + r], 300, 60, 3)


def s_spikecannon(d, B):
    CO(d, 5, 16, 3.5, 2)
    dartg(d, (10, 10), (24, 5), 2, 5)
    dartg(d, (11, 16), (27, 16), 2, 5)
    dartg(d, (10, 22), (24, 27), 2, 5)


def s_splash(d, B):
    d.ellipse([7, 12, 21, 21], fill=1)
    P(d, [(20, 16.5), (27, 11), (27, 22)])
    C(d, 11, 15.5, 1.3, on=0)
    A(d, [4, 22, 16, 30], 20, 160, 2)
    A(d, [16, 24, 26, 32], 20, 160, 2)
    C(d, 6, 7, 1.5)
    C(d, 24, 5, 1.5)


def s_stomp(d, B):
    d.rounded_rectangle([9, 8, 24, 21], radius=4, fill=1)
    for x in (11, 15.5, 20):
        C(d, x, 7.5, 2)
    impact_v(d, 11, 26, 3, up=False)
    impact_v(d, 21, 26, 3, up=False)
    L(d, [(4, 29), (28, 29)], 2)


def s_stringshot(d, B):
    P(d, [(2, 14), (8, 12), (8, 20), (2, 18)])
    for ty in (8, 16, 24):
        pts = []
        for i in range(8):
            t = i / 7
            pts.append((8 + 14 * t, 16 + (ty - 16) * t + 1.5 * math.sin(t * 9)))
        L(d, pts, 2)
    for off in (-3, 0, 3):
        L(d, [(23 + off, 10), (23 + off, 24)], 1)
        L(d, [(19, 17 + off), (28, 17 + off)], 1)


def s_struggle(d, B):
    arrow(d, (4, 11), (26, 11), 3, 6)
    arrow(d, (28, 21), (6, 21), 3, 6)
    crackline(d, 16, 26, 31, on=1, w=2, step=3, amp=2)


def s_substitute(d, B):
    d.rounded_rectangle([10, 15, 22, 28], radius=3, fill=1)
    d.rounded_rectangle([11, 5, 21, 17], radius=3, fill=1)
    P(d, [(11, 7), (7, 2), (14, 5)])
    P(d, [(21, 7), (25, 2), (18, 5)])
    C(d, 13.5, 10, 1.4, on=0)
    C(d, 18.5, 10, 1.4, on=0)
    L(d, [(13, 13.5), (15, 12.8), (17, 13.5), (19, 12.8)], 1, on=0)


def s_supersonic(d, B):
    C(d, 16, 16, 2)
    for r in (6.5, 11, 15):
        pts = []
        n = 14
        for i in range(n + 1):
            a = TAU * i / n
            rr = r + (1.8 if i % 2 else 0)
            pts.append((16 + rr * math.cos(a), 16 + rr * math.sin(a)))
        L(d, pts, 1 if r > 13 else 2)


def s_swift(d, B):
    star(d, 9, 9, 5.5, 5)
    star(d, 20, 15, 4.5, 5)
    star(d, 27, 23, 3.5, 5)
    speed_lines(d, [((2, 14), (7, 13)), ((10, 21), (15, 20)),
                    ((18, 28), (23, 27))], 1)


def s_swordsdance(d, B):
    sword(d, (6, 4), (25, 27))
    sword(d, (26, 4), (7, 27))
    spark(d, 16, 3, 2)


def s_tackle(d, B):
    d.ellipse([12, 10, 29, 22], fill=1)
    C(d, 10, 17, 5)
    speed_lines(d, [((1, 8), (7, 8)), ((0, 14), (4, 14)), ((1, 24), (8, 24))], 2)


def s_tailwhip(d, B):
    L(d, [(4, 27), (10, 24), (15, 18), (17, 11), (15, 5)], 3)
    A(d, [12, 2, 26, 16], 220, 320, 1)
    A(d, [15, 4, 29, 18], 230, 310, 1)


def s_takedown(d, B):
    d.ellipse([11, 9, 28, 21], fill=1)
    C(d, 9, 16, 5)
    crackline(d, 19, 3, 28, on=0, w=2, step=4, amp=3)
    star(d, 4, 8, 3, 5)


def s_thrash(d, B):
    star(d, 16, 16, 7, 6, 0.5)
    A(d, [2, 2, 30, 30], 20, 90, 2)
    A(d, [2, 2, 30, 30], 140, 210, 2)
    A(d, [2, 2, 30, 30], 260, 330, 2)
    star(d, 27, 6, 2.5, 4)
    star(d, 5, 26, 2.5, 4)


def s_transform(d, B):
    L(d, [(15, 6), (6, 6), (6, 26), (15, 26)], 2)
    A(d, [6, 6, 26, 26], 270, 90, 2)
    for y in range(8, 25, 5):
        L(d, [(16, y), (16, y + 2.5)], 1)
    arrow(d, (20, 2), (26, 4), 2, 4)
    arrow(d, (12, 30), (6, 28), 2, 4)


def s_triattack(d, B):
    PO(d, [(16, 4), (28, 25), (4, 25)], 2)
    C(d, 16, 6, 3)
    C(d, 26.5, 24, 3)
    C(d, 5.5, 24, 3)


def s_visegrip(d, B):
    A(d, [6, 2, 26, 20], 200, 340, 3)
    P(d, [(24, 6), (28, 11), (22, 10)])
    A(d, [6, 12, 26, 30], 20, 160, 3)
    P(d, [(24, 26), (28, 21), (22, 22)])
    R(d, [13, 14, 19, 18])
    spark(d, 4, 16, 2)


def s_wrap(d, B):
    L(d, [(16, 3), (16, 29)], 4)
    for y in (9, 16, 23):
        d.ellipse([8, y - 3.2, 24, y + 3.2], outline=1, width=2)


def s_bind(d, B):
    L(d, [(6, 26), (26, 6)], 4)
    CO(d, 12, 20, 5.5, 2)
    CO(d, 20, 12, 5.5, 2)
    L(d, [(26, 24), (29, 27)], 2)


def s_constrict(d, B):
    CO(d, 16, 16, 5.5, 2)
    L(d, [(2, 2), (8, 4), (12, 9)], 3)
    L(d, [(30, 2), (24, 4), (20, 9)], 3)
    L(d, [(2, 30), (8, 28), (12, 23)], 3)
    L(d, [(30, 30), (24, 28), (20, 23)], 3)


def s_earthquake(d, B):
    P(d, [(1, 17), (14, 17), (12, 21), (14, 25), (11, 30), (1, 30)])
    P(d, [(18, 20), (30, 20), (30, 30), (15, 30), (17, 26), (15, 23)])
    L(d, [(4, 12), (8, 13)], 2)
    L(d, [(23, 15), (27, 16)], 2)
    C(d, 14, 8, 1.6)
    C(d, 20, 11, 1.4)


def s_fissure(d, B):
    R(d, [1, 14, 30, 30])
    P(d, [(13, 14), (19, 14), (24, 30), (8, 30)], on=0)
    P(d, [(13, 14), (16, 19), (12, 24), (15, 30), (8, 30)], on=0)
    P(d, [(19, 14), (17, 20), (21, 25), (24, 30), (18, 30)], on=0)
    C(d, 16, 22, 1.5)
    C(d, 15, 27, 1.2)


def s_dig(d, B):
    R(d, [2, 19, 30, 23])
    d.ellipse([9, 18, 23, 26], fill=0)
    A(d, [9, 18, 23, 26], 0, 180, 2)
    C(d, 27, 15, 3.2)
    C(d, 24, 11, 2.2)
    C(d, 7, 11, 1.5)
    C(d, 12, 7, 1.5)
    arrow(d, (16, 4), (16, 15), 2, 5)


def s_peck(d, B):
    C(d, 10, 12, 5.5)
    C(d, 8.5, 10.5, 1.4, on=0)
    P(d, [(14, 9), (27, 13), (14, 16)])
    impact_v(d, 29, 13, 3)
    L(d, [(7, 18), (7, 24)], 2)


def s_drillpeck(d, B):
    P(d, [(4, 16), (24, 6), (24, 26)])
    for x in (11, 16, 21):
        t = (x - 4) / 20.0
        h = 10 * t
        L(d, [(x, 16 - h + 1), (x, 16 + h - 1)], 2, on=0)
    A(d, [18, 1, 30, 13], 210, 300, 2)
    A(d, [18, 19, 30, 31], 60, 150, 2)


def s_wingattack(d, B):
    for m in (1, -1):
        P(d, [(16 + m * 2, 22), (16 + m * 14, 3), (16 + m * 13, 12),
              (16 + m * 8, 14), (16 + m * 9, 18), (16 + m * 3, 19)])
    C(d, 16, 23, 2.6)


def s_fly(d, B):
    A(d, [3, 6, 17, 20], 200, 340, 3)
    A(d, [15, 6, 29, 20], 200, 340, 3)
    C(d, 16, 13, 2)
    L(d, [(4, 26), (10, 26)], 2)
    L(d, [(18, 24), (26, 24)], 2)
    L(d, [(10, 29), (16, 29)], 2)


def s_pinmissile(d, B):
    dartg(d, (3, 8), (16, 5), 2, 5)
    dartg(d, (3, 16), (18, 13), 2, 5)
    dartg(d, (3, 24), (16, 21), 2, 5)
    dartg(d, (12, 29), (24, 26), 2, 5)
    spark(d, 27, 8, 2)


def s_twineedle(d, B):
    dartg(d, (5, 10), (24, 5), 3, 6)
    dartg(d, (5, 20), (24, 15), 3, 6)
    droplet(d, 27, 24, 2.5)


def s_confuseray(d, B):
    C(d, 13, 12, 4.5)
    L(d, [(16, 15), (21, 21), (19, 27)], 3)
    for a_deg in (150, 210, 270, 330, 30):
        a = math.radians(a_deg)
        L(d, [(13 + 6.5 * math.cos(a), 12 + 6.5 * math.sin(a)),
              (13 + 9.5 * math.cos(a), 12 + 9.5 * math.sin(a))], 2)
    star(d, 27, 7, 3, 4)


def s_dragonrage(d, B):
    # serpentine dragon: horned head upper-right, thick S-body, tail lower-right
    L(d, [(18, 10), (9, 13), (6, 19), (10, 25), (17, 27)], 5)
    L(d, [(17, 27), (24, 26), (28, 22)], 3)
    P(d, [(14, 3), (27, 8), (15, 13)])
    L(d, [(23, 5), (27, 1)], 2)
    C(d, 19, 7, 1.5, on=0)


# The whole table: every Gen 1 move id -> its composition.
BESPOKE = {
    "absorb": s_absorb, "acid": s_acid, "acidarmor": s_acidarmor,
    "agility": s_agility, "amnesia": s_amnesia, "aurorabeam": s_aurorabeam,
    "barrage": s_barrage, "barrier": s_barrier, "bide": s_bide,
    "bind": s_bind, "bite": s_bite, "blizzard": s_blizzard,
    "bodyslam": s_bodyslam, "boneclub": s_boneclub,
    "bonemerang": s_bonemerang, "bubble": s_bubble,
    "bubblebeam": s_bubblebeam, "clamp": s_clamp,
    "cometpunch": s_cometpunch, "confuseray": s_confuseray,
    "confusion": s_confusion, "constrict": s_constrict,
    "conversion": s_conversion, "counter": s_counter,
    "crabhammer": s_crabhammer, "cut": s_cut, "defensecurl": s_defensecurl,
    "dig": s_dig, "disable": s_disable, "dizzypunch": s_dizzypunch,
    "doubleedge": s_doubleedge, "doublekick": s_doublekick,
    "doubleslap": s_doubleslap, "doubleteam": s_doubleteam,
    "dragonrage": s_dragonrage, "dreameater": s_dreameater,
    "drillpeck": s_drillpeck, "earthquake": s_earthquake,
    "eggbomb": s_eggbomb, "ember": s_ember, "explosion": s_explosion,
    "fireblast": s_fireblast, "firepunch": s_firepunch,
    "firespin": s_firespin, "fissure": s_fissure,
    "flamethrower": s_flamethrower, "flash": s_flash, "fly": s_fly,
    "focusenergy": s_focusenergy, "furyattack": s_furyattack,
    "furyswipes": s_furyswipes, "glare": s_glare, "growl": s_growl,
    "growth": s_growth, "guillotine": s_guillotine, "gust": s_gust,
    "harden": s_harden, "haze": s_haze, "headbutt": s_headbutt,
    "highjumpkick": s_highjumpkick, "hornattack": s_hornattack,
    "horndrill": s_horndrill, "hydropump": s_hydropump,
    "hyperbeam": s_hyperbeam, "hyperfang": s_hyperfang,
    "hypnosis": s_hypnosis, "icebeam": s_icebeam, "icepunch": s_icepunch,
    "jumpkick": s_jumpkick, "karatechop": s_karatechop,
    "kinesis": s_kinesis, "leechlife": s_leechlife,
    "leechseed": s_leechseed, "leer": s_leer, "lick": s_lick,
    "lightscreen": s_lightscreen, "lovelykiss": s_lovelykiss,
    "lowkick": s_lowkick, "meditate": s_meditate, "megadrain": s_megadrain,
    "megakick": s_megakick, "megapunch": s_megapunch,
    "metronome": s_metronome, "mimic": s_mimic, "minimize": s_minimize,
    "mirrormove": s_mirrormove, "mist": s_mist, "nightshade": s_nightshade,
    "payday": s_payday, "peck": s_peck, "petaldance": s_petaldance,
    "pinmissile": s_pinmissile, "poisongas": s_poisongas,
    "poisonpowder": s_poisonpowder, "poisonsting": s_poisonsting,
    "pound": s_pound, "psybeam": s_psybeam, "psychic": s_psychic,
    "psywave": s_psywave, "quickattack": s_quickattack, "rage": s_rage,
    "razorleaf": s_razorleaf, "razorwind": s_razorwind,
    "recover": s_recover, "reflect": s_reflect, "rest": s_rest,
    "roar": s_roar, "rockslide": s_rockslide, "rockthrow": s_rockthrow,
    "rollingkick": s_rollingkick, "sandattack": s_sandattack,
    "scratch": s_scratch, "screech": s_screech,
    "seismictoss": s_seismictoss, "selfdestruct": s_selfdestruct,
    "sharpen": s_sharpen, "sing": s_sing, "skullbash": s_skullbash,
    "skyattack": s_skyattack, "slam": s_slam, "slash": s_slash,
    "sleeppowder": s_sleeppowder, "sludge": s_sludge, "smog": s_smog,
    "smokescreen": s_smokescreen, "softboiled": s_softboiled,
    "solarbeam": s_solarbeam, "sonicboom": s_sonicboom,
    "spikecannon": s_spikecannon, "splash": s_splash, "spore": s_spore,
    "stomp": s_stomp, "strength": s_strength, "stringshot": s_stringshot,
    "struggle": s_struggle, "stunspore": s_stunspore,
    "submission": s_submission, "substitute": s_substitute,
    "superfang": s_superfang, "supersonic": s_supersonic, "surf": s_surf,
    "swift": s_swift, "swordsdance": s_swordsdance, "tackle": s_tackle,
    "tailwhip": s_tailwhip, "takedown": s_takedown, "teleport": s_teleport,
    "thrash": s_thrash, "thunder": s_thunder, "thunderbolt": s_thunderbolt,
    "thunderpunch": s_thunderpunch, "thundershock": s_thundershock,
    "thunderwave": s_thunderwave, "toxic": s_toxic,
    "transform": s_transform, "triattack": s_triattack,
    "twineedle": s_twineedle, "vinewhip": s_vinewhip,
    "visegrip": s_visegrip, "waterfall": s_waterfall,
    "watergun": s_watergun, "whirlwind": s_whirlwind,
    "wingattack": s_wingattack, "withdraw": s_withdraw, "wrap": s_wrap,
}

TYPE_FALLBACK_GLYPH = {
    "Fire": lambda d: flame(d, 16, 16, 9),
    "Water": lambda d: droplet(d, 16, 16, 9),
    "Electric": lambda d: bolt(d, 16, 16, 18),
    "Grass": lambda d: leafg(d, 16, 16, 9),
    "Ice": lambda d: snowflake(d, 16, 16, 10),
    "Fighting": lambda d: fist(d, 16, 16, 9),
    "Poison": lambda d: skullg(d, 16, 16, 9),
    "Psychic": lambda d: spiralg(d, 16, 16, 11),
    "Bug": lambda d: bugg(d, 16, 16, 9),
    "Ghost": lambda d: ghostg(d, 16, 16, 9),
    "Normal": lambda d: star(d, 16, 16, 10, 6),
}


def fallback(d, move_id, info, bits):
    """Generic composer for ids without a bespoke design (none today)."""
    TYPE_FALLBACK_GLYPH.get(info.get("type", "Normal"),
                            TYPE_FALLBACK_GLYPH["Normal"])(d)
    for _ in range(3):
        a = math.radians(bits.rng(0, 359))
        C(d, 16 + 13.5 * math.cos(a), 16 + 13.5 * math.sin(a), 1.2)


# ── per-move rendering ───────────────────────────────────────────────────────

def render(move_id: str, info: dict, salt: int) -> Image.Image:
    img = Image.new("1", (SIDE, SIDE), 0)
    d = ImageDraw.Draw(img)
    bits = Bits(f"{move_id}/{salt}")

    fn = BESPOKE.get(move_id)
    if fn is not None:
        fn(d, bits)
    else:
        print(f"warning: no bespoke design for {move_id}, using fallback")
        fallback(d, move_id, info, bits)

    if salt > 0:
        # Deterministic collision-breaking marker (normally never used).
        a = math.radians(bits.rng(0, 359))
        C(d, min(max(16 + 14 * math.cos(a), 1.5), 30.5),
          min(max(16 + 14 * math.sin(a), 1.5), 30.5), 1.2)
    return img


# ── packing / emission ───────────────────────────────────────────────────────

def pack(img: Image.Image) -> bytes:
    """Row-major, MSB = leftmost pixel: ImageRaw::<BinaryColor> layout."""
    data = bytearray(SPRITE_BYTES)
    px = img.load()
    for y in range(SIDE):
        for x in range(SIDE):
            if px[x, y]:
                data[y * BYTES_PER_ROW + x // 8] |= 0x80 >> (x % 8)
    return bytes(data)


def popcount(packed: bytes) -> int:
    return sum(bin(b).count("1") for b in packed)


def bytes_literal(packed: bytes, indent: str) -> str:
    lines = []
    for i in range(0, len(packed), 16):
        chunk = ", ".join(f"0x{b:02x}" for b in packed[i:i + 16])
        lines.append(f"{indent}{chunk},")
    return "[\n" + "\n".join(lines) + "\n" + indent[:-4] + "]"


def emit_rust(entries, out_path: Path):
    ids = [mid for mid, _ in entries]
    total = len(entries) * SPRITE_BYTES
    parts = []
    parts.append(f"""\
//! 1-bit move icons for the battle screen: one {SIDE}x{SIDE} sprite per Gen 1
//! move, keyed by gen1_battle lowercase move id (the keys of
//! `gen1_battle/data/rby_moves.json`).
//!
//! Generated by `scripts/gen_move_sprites.py`; do not edit by hand, rerun the
//! script instead. Packed row-major, {BYTES_PER_ROW} bytes per row, MSB =
//! leftmost pixel: the layout `embedded_graphics::image::ImageRaw::<BinaryColor>`
//! expects (same convention as the mon sprites in `sprites.rs`).
//!
//! Flash cost: {len(entries)} sprites x {SPRITE_BYTES} B = {total} B (~{total / 1024:.1f} KiB).

pub const MOVE_SPRITE_SIDE: u32 = {SIDE};
/// Bytes per packed sprite ({SIDE} * {SIDE} / 8).
pub const MOVE_SPRITE_BYTES: usize = {SPRITE_BYTES};

/// `(move id, packed bitmap)` pairs, sorted by move id for binary search.
static MOVE_SPRITES: &[(&str, [u8; MOVE_SPRITE_BYTES])] = &[
""")
    for mid, packed in entries:
        parts.append(f'    ("{mid}", {bytes_literal(packed, " " * 8)}),\n')
    parts.append("""];

/// Look up a move's sprite by lowercase move id (e.g. "surf"). O(log N).
pub fn move_sprite(move_id: &str) -> Option<&'static [u8]> {
    MOVE_SPRITES
        .binary_search_by(|(id, _)| (*id).cmp(move_id))
        .ok()
        .map(|i| &MOVE_SPRITES[i].1[..])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every gen1_battle move id (the keys of rby_moves.json), all 165.
    const GEN1_MOVE_IDS: [&str; """)
    parts.append(f"{len(ids)}] = [\n")
    line = "        "
    for mid in ids:
        item = f'"{mid}", '
        if len(line) + len(item) > 96:
            parts.append(line.rstrip() + "\n")
            line = "        "
        line += item
    parts.append(line.rstrip() + "\n    ];\n")
    parts.append("""
    #[test]
    fn every_gen1_move_resolves() {
        assert_eq!(MOVE_SPRITES.len(), GEN1_MOVE_IDS.len());
        for id in GEN1_MOVE_IDS {
            let sprite =
                move_sprite(id).unwrap_or_else(|| panic!("no sprite for move id {id}"));
            assert_eq!(sprite.len(), MOVE_SPRITE_BYTES);
        }
        assert!(move_sprite("notamove").is_none());
        assert!(move_sprite("").is_none());
    }
}
""")
    out_path.write_text("".join(parts))


def emit_preview(entries, path: Path, scale=3):
    cols = 11
    rows = math.ceil(len(entries) / cols)
    cell_w = SIDE * scale + 8
    cell_h = SIDE * scale + 16
    sheet = Image.new("L", (cols * cell_w + 8, rows * cell_h + 8), 24)
    ds = ImageDraw.Draw(sheet)
    font = ImageFont.load_default()
    for i, (mid, img) in enumerate(entries):
        col, row = i % cols, i // cols
        x0 = 8 + col * cell_w
        y0 = 8 + row * cell_h
        big = img.convert("L").point(lambda v: 255 if v else 0)
        if scale != 1:
            big = big.resize((SIDE * scale, SIDE * scale), Image.NEAREST)
        sheet.paste(big, (x0, y0))
        ds.rectangle([x0 - 1, y0 - 1, x0 + SIDE * scale, y0 + SIDE * scale],
                     outline=70)
        ds.text((x0, y0 + SIDE * scale + 2), mid, fill=200, font=font)
    sheet.save(path)


def emit_preview_1x(entries, path: Path):
    """Actual-scale sheet: judge true OLED legibility."""
    cols = 15
    cw, ch = 72, 48
    rows = math.ceil(len(entries) / cols)
    sheet = Image.new("L", (cols * cw + 8, rows * ch + 8), 24)
    ds = ImageDraw.Draw(sheet)
    font = ImageFont.load_default()
    for i, (mid, img) in enumerate(entries):
        x = 8 + (i % cols) * cw
        y = 8 + (i // cols) * ch
        sheet.paste(img.convert("L").point(lambda v: 255 if v else 0), (x, y))
        ds.rectangle([x - 1, y - 1, x + 32, y + 32], outline=60)
        ds.text((x + 35, y + 10), mid[:11], fill=170, font=font)
    sheet.save(path)


# ── main ─────────────────────────────────────────────────────────────────────

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--preview", type=Path, default=None,
                    help="3x labeled PNG contact-sheet path")
    ap.add_argument("--preview1x", type=Path, default=None,
                    help="1x actual-scale PNG contact-sheet path")
    args = ap.parse_args()

    moves = json.loads(MOVES_JSON.read_text())
    ids = sorted(moves.keys())

    extra = set(BESPOKE) - set(ids)
    assert not extra, f"bespoke designs for unknown ids: {sorted(extra)}"

    rendered = []
    packed_entries = []
    seen = {}
    for mid in ids:
        for salt in range(64):
            img = render(mid, moves[mid], salt)
            packed = pack(img)
            n = popcount(packed)
            if packed not in seen and 30 <= n <= 800:
                seen[packed] = mid
                rendered.append((mid, img))
                packed_entries.append((mid, packed))
                if salt:
                    print(f"note: {mid} needed salt {salt}")
                break
        else:
            raise SystemExit(f"could not find a unique bitmap for {mid}")

    # Programmatic uniqueness guarantee: all packed bitmaps pairwise distinct.
    assert len(packed_entries) == len(ids) == 165, len(packed_entries)
    assert len({p for _, p in packed_entries}) == len(packed_entries), \
        "duplicate sprite bitmaps"

    emit_rust(packed_entries, OUT_RS)
    total = len(packed_entries) * SPRITE_BYTES
    print(f"wrote {OUT_RS} ({len(packed_entries)} sprites, "
          f"{total} bytes = {total / 1024:.1f} KiB of flash, "
          f"{len(BESPOKE)} bespoke designs)")

    if args.preview:
        args.preview.parent.mkdir(parents=True, exist_ok=True)
        emit_preview(rendered, args.preview, 3)
        print(f"wrote preview {args.preview}")
    if args.preview1x:
        args.preview1x.parent.mkdir(parents=True, exist_ok=True)
        emit_preview_1x(rendered, args.preview1x)
        print(f"wrote 1x preview {args.preview1x}")


if __name__ == "__main__":
    main()
