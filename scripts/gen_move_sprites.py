#!/usr/bin/env python3
"""Generate a unique 1-bit 32x32 sprite for every Gen 1 move.

Reads the canonical move list (the keys of gen1_battle/data/rby_moves.json)
and emits mega_blastoise_core/src/move_sprites.rs: packed 1-bit bitmaps in
the same layout as the mon sprites in sprites.rs (row-major, MSB = leftmost
pixel, ready for embedded_graphics ImageRaw::<BinaryColor>).

Each sprite is composed procedurally from:
  * a type-family base motif (flame, droplet, bolt, leaf, fist, skull,
    spiral, crystal, wing, boulder, fissure, bug, ghost, serpent, burst),
  * an effect-family overlay derived from the move's mechanics (multi-hit,
    drain, recoil, OHKO, screens, healing, stat changes, status auras,
    trapping rings, charge chevrons, fixed damage, plus one-off glyphs for
    the odd ducks like Metronome or Substitute),
  * deterministic per-move-name variation (sha256-seeded mirroring, center
    jitter, motif phase, and accent pixels) so no two sprites are identical.

Uniqueness is verified programmatically: all packed bitmaps must be pairwise
distinct or the script fails. If a collision ever happens, the later move
(in sorted id order) is re-rendered with an incremented hash salt, which is
deterministic across runs.

Usage:
    python3 scripts/gen_move_sprites.py [--preview /path/to/sheet.png]

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
        """Uniform-ish integer in [lo, hi] inclusive."""
        return lo + self.take(16) % (hi - lo + 1)


# ── small drawing helpers ────────────────────────────────────────────────────

def pts(norm, cx, cy, r, mirror=False):
    """Scale unit-space (-1..1) points onto the canvas around (cx, cy)."""
    out = []
    for x, y in norm:
        if mirror:
            x = -x
        out.append((cx + x * r, cy + y * r))
    return out


def ring(d, cx, cy, rad, width=1):
    d.ellipse([cx - rad, cy - rad, cx + rad, cy + rad], outline=1, width=width)


def dot(d, x, y, rad=1.0, fill=1):
    d.ellipse([x - rad, y - rad, x + rad, y + rad], fill=fill)


def arrow_up(d, x, y, h=9, w=7):
    d.polygon([(x, y), (x - w / 2, y + 4), (x + w / 2, y + 4)], fill=1)
    d.rectangle([x - 1, y + 4, x + 1, y + h], fill=1)


def arrow_down(d, x, y, h=9, w=7):
    d.rectangle([x - 1, y, x + 1, y + h - 4], fill=1)
    d.polygon([(x, y + h), (x - w / 2, y + h - 4), (x + w / 2, y + h - 4)], fill=1)


def chevron(d, x, y, w=10, up=True):
    dy = 3 if up else -3
    d.line([(x - w / 2, y + dy), (x, y), (x + w / 2, y + dy)], fill=1, width=2)


def inward_arrow(d, x0, y0, x1, y1):
    """Short arrow from (x0,y0) toward (x1,y1) with a head at the far end."""
    d.line([(x0, y0), (x1, y1)], fill=1, width=1)
    ang = math.atan2(y1 - y0, x1 - x0)
    for off in (math.radians(150), math.radians(-150)):
        hx = x1 + 3.5 * math.cos(ang + off)
        hy = y1 + 3.5 * math.sin(ang + off)
        d.line([(x1, y1), (hx, hy)], fill=1, width=1)


def cycle_arrows(d, cx, cy, rad):
    d.arc([cx - rad, cy - rad, cx + rad, cy + rad], 300, 60, fill=1, width=2)
    d.arc([cx - rad, cy - rad, cx + rad, cy + rad], 120, 240, fill=1, width=2)
    for ang_deg in (60, 240):
        a = math.radians(ang_deg)
        px, py = cx + rad * math.cos(a), cy + rad * math.sin(a)
        tx, ty = -math.sin(a), math.cos(a)  # clockwise tangent
        nx, ny = math.cos(a), math.sin(a)
        d.polygon(
            [(px + 4 * tx, py + 4 * ty), (px + 2.2 * nx, py + 2.2 * ny),
             (px - 2.2 * nx, py - 2.2 * ny)],
            fill=1,
        )


def crack(d, x, ytop, ybot, bits):
    """Jagged erased fissure from (x, ytop) down to (x±, ybot)."""
    path = [(x, ytop)]
    y = ytop
    xx = x
    flip = 1 if bits.take(1) else -1
    while y < ybot:
        y += 4
        xx += flip * bits.rng(2, 3)
        flip = -flip
        path.append((xx, min(y, ybot)))
    d.line(path, fill=0, width=2)


def mini_star(d, cx, cy, r, points=5, phase=0.0):
    poly = []
    for i in range(points * 2):
        rad = r if i % 2 == 0 else r * 0.45
        a = phase + math.pi * i / points - math.pi / 2
        poly.append((cx + rad * math.cos(a), cy + rad * math.sin(a)))
    d.polygon(poly, fill=1)


# ── type-family base motifs ──────────────────────────────────────────────────
# Each takes (draw, cx, cy, r, mirror, bits) and draws a bold filled shape.

def m_fire(d, cx, cy, r, mirror, bits):
    flame = [(0.02, -1.0), (0.30, -0.55), (0.18, -0.30), (0.62, -0.10),
             (0.55, 0.45), (0.0, 0.95), (-0.55, 0.45), (-0.62, -0.10),
             (-0.30, -0.42)]
    d.polygon(pts(flame, cx, cy, r, mirror), fill=1)
    inner = [(0.0, -0.05), (0.26, 0.32), (0.0, 0.60), (-0.26, 0.32)]
    d.polygon(pts(inner, cx, cy, r, mirror), fill=0)


def m_water(d, cx, cy, r, mirror, bits):
    d.ellipse([cx - 0.62 * r, cy - 0.25 * r, cx + 0.62 * r, cy + 0.95 * r], fill=1)
    d.polygon(pts([(0, -1.0), (0.5, 0.05), (-0.5, 0.05)], cx, cy, r, mirror), fill=1)
    hx = cx + (0.20 * r if mirror else -0.20 * r)
    d.ellipse([hx - 0.14 * r, cy + 0.12 * r, hx + 0.14 * r, cy + 0.44 * r], fill=0)


def m_electric(d, cx, cy, r, mirror, bits):
    bolt = [(0.25, -1.0), (-0.45, 0.10), (-0.06, 0.10), (-0.30, 1.0),
            (0.45, -0.18), (0.05, -0.18)]
    d.polygon(pts(bolt, cx, cy, r, mirror), fill=1)


def m_grass(d, cx, cy, r, mirror, bits):
    leaf = [(0.0, -1.0), (0.55, -0.45), (0.62, 0.25), (0.0, 1.0),
            (-0.38, 0.35), (-0.38, -0.45)]
    d.polygon(pts(leaf, cx, cy, r, mirror), fill=1)
    d.line(pts([(0.0, -0.80), (0.0, 0.85)], cx, cy, r, mirror), fill=0, width=1)


def m_fighting(d, cx, cy, r, mirror, bits):
    d.rounded_rectangle(
        [cx - 0.70 * r, cy - 0.50 * r, cx + 0.70 * r, cy + 0.55 * r],
        radius=0.25 * r, fill=1)
    for kx in (-0.35, 0.0, 0.35):
        p = pts([(kx, -0.50), (kx, -0.12)], cx, cy, r, mirror)
        d.line(p, fill=0, width=1)
    tx = cx + (0.70 * r if mirror else -0.70 * r)
    d.ellipse([tx - 0.16 * r, cy - 0.05 * r, tx + 0.16 * r, cy + 0.40 * r], fill=1)


def m_poison(d, cx, cy, r, mirror, bits):
    d.ellipse([cx - 0.55 * r, cy - 0.90 * r, cx + 0.55 * r, cy + 0.25 * r], fill=1)
    d.rectangle([cx - 0.30 * r, cy + 0.05 * r, cx + 0.30 * r, cy + 0.48 * r], fill=1)
    for ex in (-0.24, 0.24):
        p = pts([(ex, -0.35)], cx, cy, r, mirror)[0]
        dot(d, p[0], p[1], 0.15 * r, fill=0)
    for tx in (-0.15, 0.0, 0.15):
        p = pts([(tx, 0.10), (tx, 0.48)], cx, cy, r, mirror)
        d.line(p, fill=0, width=1)
    dx = 0.42 if bits.take(1) else -0.42
    p = pts([(dx, 0.85)], cx, cy, r, mirror)[0]
    dot(d, p[0], p[1], 1.2)


def m_psychic(d, cx, cy, r, mirror, bits):
    phase = bits.take(3) * math.pi / 4
    sgn = -1.0 if mirror else 1.0
    prev = None
    for i in range(0, 57):
        t = i / 56
        a = phase + 2 * math.pi * 2.2 * t
        rad = r * (0.10 + 0.90 * t)
        p = (cx + sgn * rad * math.cos(a), cy + rad * math.sin(a))
        if prev is not None:
            d.line([prev, p], fill=1, width=2)
        prev = p


def m_ice(d, cx, cy, r, mirror, bits):
    phase = (bits.take(2) * 15) * math.pi / 180
    for k in range(6):
        a = phase + k * math.pi / 3
        ex, ey = cx + r * math.cos(a), cy + r * math.sin(a)
        d.line([(cx, cy), (ex, ey)], fill=1, width=2)
        bx, by = cx + 0.55 * r * math.cos(a), cy + 0.55 * r * math.sin(a)
        for off in (math.radians(35), math.radians(-35)):
            tx = bx + 0.32 * r * math.cos(a + off)
            ty = by + 0.32 * r * math.sin(a + off)
            d.line([(bx, by), (tx, ty)], fill=1, width=1)
    dot(d, cx, cy, 0.16 * r)


def m_flying(d, cx, cy, r, mirror, bits):
    wing = [(-0.95, 0.40), (-0.55, -0.25), (0.05, -0.62), (0.95, -0.70),
            (0.70, -0.25), (0.35, 0.10), (-0.05, 0.40), (-0.50, 0.55)]
    d.polygon(pts(wing, cx, cy, r, mirror), fill=1)
    for sx in (-0.55, -0.05, 0.45):
        p = pts([(sx, 0.55)], cx, cy, r, mirror)[0]
        dot(d, p[0], p[1], 0.22 * r, fill=0)
    p = pts([(-0.80, 0.30), (0.60, -0.55)], cx, cy, r, mirror)
    d.line(p, fill=0, width=1)


def m_rock(d, cx, cy, r, mirror, bits):
    boulder = [(-0.55, -0.75), (0.35, -0.90), (0.90, -0.15), (0.55, 0.75),
               (-0.40, 0.85), (-0.90, 0.10)]
    d.polygon(pts(boulder, cx, cy, r, mirror), fill=1)
    d.line(pts([(0.35, -0.90), (0.05, 0.05), (0.55, 0.75)], cx, cy, r, mirror),
           fill=0, width=1)
    d.line(pts([(-0.90, 0.10), (0.05, 0.05)], cx, cy, r, mirror), fill=0, width=1)


def m_ground(d, cx, cy, r, mirror, bits):
    d.rectangle([cx - r, cy + 0.35 * r, cx + r, cy + 0.85 * r], fill=1)
    d.polygon(pts([(-0.55, 0.45), (-0.20, -0.55), (0.10, 0.45)], cx, cy, r, mirror),
              fill=1)
    d.polygon(pts([(0.10, 0.45), (0.45, -0.95), (0.80, 0.45)], cx, cy, r, mirror),
              fill=1)
    p = pts([(-0.60, 0.35)], cx, cy, r, mirror)[0]
    crack(d, p[0], cy + 0.35 * r, cy + 0.85 * r + 1, bits)


def m_bug(d, cx, cy, r, mirror, bits):
    d.ellipse([cx - 0.35 * r, cy - 0.15 * r, cx + 0.35 * r, cy + 0.90 * r], fill=1)
    d.ellipse([cx - 0.26 * r, cy - 0.60 * r, cx + 0.26 * r, cy - 0.05 * r], fill=1)
    for sx in (-1, 1):
        top = (cx + sx * 0.50 * r, cy - 1.0 * r)
        d.line([(cx + sx * 0.12 * r, cy - 0.55 * r), top], fill=1, width=1)
        dot(d, top[0], top[1], 1.0)
        for ly in (0.15, 0.42, 0.68):
            d.line([(cx + sx * 0.30 * r, cy + ly * r),
                    (cx + sx * 0.70 * r, cy + (ly + 0.18) * r)], fill=1, width=1)
    for ly in (0.25, 0.55):
        d.line([(cx - 0.30 * r, cy + ly * r), (cx + 0.30 * r, cy + ly * r)],
               fill=0, width=1)


def m_ghost(d, cx, cy, r, mirror, bits):
    d.pieslice([cx - 0.65 * r, cy - 0.95 * r, cx + 0.65 * r, cy + 0.35 * r],
               180, 360, fill=1)
    d.rectangle([cx - 0.65 * r, cy - 0.32 * r, cx + 0.65 * r, cy + 0.55 * r], fill=1)
    for sx in (-0.43, 0.0, 0.43):
        p = pts([(sx, 0.62)], cx, cy, r, mirror)[0]
        dot(d, p[0], p[1], 0.22 * r, fill=0)
    for ex in (-0.26, 0.26):
        p = pts([(ex, -0.30)], cx, cy, r, mirror)[0]
        dot(d, p[0], p[1], 0.13 * r, fill=0)
    p = pts([(0.18, 0.10)], cx, cy, r, mirror)[0]
    dot(d, p[0], p[1], 0.10 * r, fill=0)


def m_dragon(d, cx, cy, r, mirror, bits):
    sgn = -1.0 if mirror else 1.0
    for i in range(13):
        t = i / 12
        x = cx + sgn * 0.55 * r * math.sin(t * math.pi * 2.4)
        y = cy - r + 2 * r * t
        rad = 0.30 * r * (1.0 - 0.55 * t) + 1.0
        dot(d, x, y, rad)
    hx = cx + sgn * 0.55 * r * math.sin(0)
    head = [(0.0, -1.25), (0.35, -0.75), (-0.35, -0.75)]
    d.polygon(pts(head, hx, cy + 0.12 * r, r, mirror), fill=1)
    p = pts([(0.0, -1.02)], hx, cy + 0.12 * r, r, mirror)[0]
    d.point((round(p[0]), round(p[1])), fill=0)


def m_normal(d, cx, cy, r, mirror, bits):
    points = 5 + bits.take(2)  # 5..8 point burst
    phase = bits.take(3) * math.pi / 8
    mini_star(d, cx, cy, r, points=points, phase=phase)


TYPE_MOTIFS = {
    "Fire": m_fire,
    "Water": m_water,
    "Electric": m_electric,
    "Grass": m_grass,
    "Fighting": m_fighting,
    "Poison": m_poison,
    "Psychic": m_psychic,
    "Ice": m_ice,
    "Flying": m_flying,
    "Rock": m_rock,
    "Ground": m_ground,
    "Bug": m_bug,
    "Ghost": m_ghost,
    "Dragon": m_dragon,
    "Normal": m_normal,
}


def draw_motif(d, type_name, cx, cy, r, mirror, bits):
    TYPE_MOTIFS[type_name](d, cx, cy, r, mirror, bits)


# ── effect-family categorization ─────────────────────────────────────────────

def categorize(effect: str) -> str:
    if effect in ("TWO_TO_FIVE_ATTACKS_EFFECT", "ATTACK_TWICE_EFFECT",
                  "TWINEEDLE_EFFECT"):
        return "multi"
    if effect == "OHKO_EFFECT":
        return "ohko"
    if effect == "EXPLODE_EFFECT":
        return "explode"
    if effect in ("LIGHT_SCREEN_EFFECT", "REFLECT_EFFECT", "MIST_EFFECT"):
        return "screen"
    if effect == "HEAL_EFFECT":
        return "heal"
    if effect in ("DRAIN_HP_EFFECT", "DREAM_EATER_EFFECT", "LEECH_SEED_EFFECT"):
        return "drain"
    if effect in ("RECOIL_EFFECT", "JUMP_KICK_EFFECT"):
        return "recoil"
    if effect in ("SLEEP_EFFECT", "PARALYZE_EFFECT", "POISON_EFFECT",
                  "CONFUSION_EFFECT", "DISABLE_EFFECT"):
        return "status"
    if effect == "TRAPPING_EFFECT":
        return "trap"
    if effect in ("SPECIAL_DAMAGE_EFFECT", "SUPER_FANG_EFFECT"):
        return "fixed"
    if effect in ("CHARGE_EFFECT", "FLY_EFFECT", "HYPER_BEAM_EFFECT"):
        return "charge"
    if effect == "SWITCH_AND_TELEPORT_EFFECT":
        return "switch"
    if "SIDE_EFFECT" in effect:
        return "side"
    if "_UP" in effect or effect == "FOCUS_ENERGY_EFFECT":
        return "statup"
    if "_DOWN" in effect:
        return "statdown"
    if effect in ("BIDE_EFFECT", "CONVERSION_EFFECT", "HAZE_EFFECT",
                  "METRONOME_EFFECT", "MIMIC_EFFECT", "MIRROR_MOVE_EFFECT",
                  "PAY_DAY_EFFECT", "RAGE_EFFECT", "SPLASH_EFFECT",
                  "SUBSTITUTE_EFFECT", "SWIFT_EFFECT", "TRANSFORM_EFFECT",
                  "THRASH_PETAL_DANCE_EFFECT"):
        return "weird"
    return "plain"  # NO_ADDITIONAL_EFFECT and anything unmapped


# Micro-glyphs marking a damaging move's secondary ("side") effect family.

def g_burn(d, x, y):
    d.polygon([(x, y - 3), (x + 2.5, y + 2), (x, y + 3.5), (x - 2.5, y + 2)], fill=1)


def g_freeze(d, x, y):
    for a in range(0, 180, 60):
        rad = math.radians(a)
        d.line([(x - 3 * math.cos(rad), y - 3 * math.sin(rad)),
                (x + 3 * math.cos(rad), y + 3 * math.sin(rad))], fill=1, width=1)


def g_paralyze(d, x, y):
    d.line([(x + 1, y - 3), (x - 2, y), (x + 2, y), (x - 1, y + 3)], fill=1, width=1)


def g_poison(d, x, y):
    ring(d, x, y, 2.5)
    d.point((x, y), fill=1)


def g_flinch(d, x, y):
    d.line([(x, y - 3), (x, y + 1)], fill=1, width=2)
    d.point((x, y + 3), fill=1)


def g_confuse(d, x, y):
    d.arc([x - 3, y - 3, x + 3, y + 3], 300, 200, fill=1, width=1)
    d.point((x, y), fill=1)


def g_statdown(d, x, y):
    d.line([(x - 3, y - 2), (x, y + 1), (x + 3, y - 2)], fill=1, width=1)
    d.line([(x - 3, y + 1), (x, y + 4), (x + 3, y + 1)], fill=1, width=1)


def side_glyph(effect: str):
    if "BURN" in effect:
        return g_burn
    if "FREEZE" in effect:
        return g_freeze
    if "PARALYZE" in effect:
        return g_paralyze
    if "POISON" in effect:
        return g_poison
    if "FLINCH" in effect:
        return g_flinch
    if "CONFUSION" in effect:
        return g_confuse
    return g_statdown  # ATTACK/DEFENSE/SPEED/SPECIAL/ACCURACY down sides


# ── per-move composition ─────────────────────────────────────────────────────

def render(move_id: str, info: dict, salt: int) -> Image.Image:
    img = Image.new("1", (SIDE, SIDE), 0)
    d = ImageDraw.Draw(img)
    bits = Bits(f"{move_id}/{salt}")

    mirror = bits.take(1) == 1
    cx = 15.5 + bits.rng(-1, 1)
    cy = 15.5 + bits.rng(-1, 1)

    tname = info["type"]
    effect = info["effect"]
    power = info.get("power") or 0
    cat = categorize(effect)

    if cat == "plain":
        draw_motif(d, tname, cx, cy, 10.5 if power >= 90 else 9.5, mirror, bits)

    elif cat == "side":
        draw_motif(d, tname, cx, cy, 9.0, mirror, bits)
        corner = bits.take(2)
        gx = 26 if corner & 1 else 5
        gy = 26 if corner & 2 else 5
        side_glyph(effect)(d, gx, gy)

    elif cat == "multi":
        count = 2 if effect in ("ATTACK_TWICE_EFFECT", "TWINEEDLE_EFFECT") else 3
        spots = [(10, 10), (22, 12), (15, 23)][:count]
        for sx, sy in spots:
            draw_motif(d, tname, sx + bits.rng(-1, 1), sy + bits.rng(-1, 1),
                       4.5, mirror, bits)
        if effect == "TWINEEDLE_EFFECT":
            g_poison(d, 26, 26)

    elif cat == "ohko":
        draw_motif(d, tname, 9, 8, 5.0, mirror, bits)
        d.line([(7, 8), (26, 27)], fill=1, width=3)
        d.line([(26, 8), (7, 27)], fill=1, width=3)

    elif cat == "explode":
        draw_motif(d, tname, cx, cy, 11.0, mirror, bits)
        crack(d, cx + bits.rng(-2, 2), 4, 28, bits)
        for a_deg in (30, 150, 270):
            a = math.radians(a_deg + bits.rng(-10, 10))
            d.line([(cx + 12 * math.cos(a), cy + 12 * math.sin(a)),
                    (cx + 15 * math.cos(a), cy + 15 * math.sin(a))],
                   fill=1, width=2)

    elif cat == "screen":
        shield = [(15.5, 2), (28, 7), (28, 16), (15.5, 29), (3, 16), (3, 7)]
        d.polygon(shield, outline=1, width=2)
        draw_motif(d, tname, 15.5, 13, 5.5, mirror, bits)

    elif cat == "heal":
        draw_motif(d, tname, 10.5, 19, 6.5, mirror, bits)
        d.rectangle([21, 5, 25, 17], fill=1)
        d.rectangle([17, 9, 29, 13], fill=1)

    elif cat == "drain":
        draw_motif(d, tname, cx, cy, 6.5, mirror, bits)
        for sx, sy in ((3, 3), (28, 3), (3, 28), (28, 28)):
            ex = sx + (6 if sx < 16 else -6)
            ey = sy + (6 if sy < 16 else -6)
            inward_arrow(d, sx, sy, ex, ey)

    elif cat == "recoil":
        draw_motif(d, tname, cx, cy, 9.5, mirror, bits)
        crack(d, cx + bits.rng(-3, 3), 3, 29, bits)
        d.line([(2, 13), (5, 15)], fill=1, width=1)
        d.line([(29, 17), (26, 15)], fill=1, width=1)

    elif cat == "status":
        draw_motif(d, tname, cx, cy, 6.5, mirror, bits)
        ring(d, cx, cy, 10.5)
        ring(d, cx, cy, 13.5)

    elif cat == "trap":
        draw_motif(d, tname, cx, cy, 7.0, mirror, bits)
        for ry in (4.0, 6.5):
            d.ellipse([cx - 13, cy - ry, cx + 13, cy + ry], outline=1, width=1)

    elif cat == "fixed":
        draw_motif(d, tname, 15.5, 11.5, 7.5, mirror, bits)
        d.rectangle([9, 23, 23, 24], fill=1)
        d.rectangle([9, 27, 23, 28], fill=1)

    elif cat == "charge":
        draw_motif(d, tname, 15.5, 20, 7.0, mirror, bits)
        chevron(d, 15.5, 8, up=True)
        chevron(d, 15.5, 3, up=True)

    elif cat == "switch":
        draw_motif(d, tname, cx, cy, 6.0, mirror, bits)
        cycle_arrows(d, cx, cy, 11)

    elif cat == "statup":
        draw_motif(d, tname, 11, 19, 6.5, mirror, bits)
        arrow_up(d, 25, 5)
        if "UP2" in effect:
            arrow_up(d, 25, 17)

    elif cat == "statdown":
        draw_motif(d, tname, 11, 12, 6.5, mirror, bits)
        arrow_down(d, 25, 17)
        if "DOWN2" in effect:
            arrow_down(d, 25, 5)

    elif cat == "weird":
        draw_weird(d, effect, tname, cx, cy, mirror, bits)

    # Deterministic accent pixels: three dots on a radius-14 ring at
    # hash-chosen angles. Combined with mirroring and jitter these make every
    # sprite unique even within a (type, effect) family.
    for _ in range(3):
        a = math.radians(bits.rng(0, 359))
        ax = min(max(15.5 + 14 * math.cos(a), 1), 30)
        ay = min(max(15.5 + 14 * math.sin(a), 1), 30)
        dot(d, ax, ay, 0.8)

    return img


def draw_weird(d, effect, tname, cx, cy, mirror, bits):
    if effect == "BIDE_EFFECT":
        draw_motif(d, tname, 10, 11, 6.0, mirror, bits)
        hg = [(19, 18), (29, 18), (19, 29), (29, 29)]
        d.polygon([hg[0], hg[1], (24, 23.5)], outline=1)
        d.polygon([(24, 23.5), hg[2], hg[3]], outline=1)
    elif effect == "CONVERSION_EFFECT":
        d.rectangle([5, 5, 14, 14], fill=1)
        d.rectangle([18, 18, 27, 27], outline=1, width=2)
        inward_arrow(d, 13, 13, 19, 19)
    elif effect == "HAZE_EFFECT":
        draw_motif(d, tname, 15.5, 8, 5.5, mirror, bits)
        for wy in (18, 23, 28):
            path = [(x, wy + 2 * math.sin(x / 3.0 + wy)) for x in range(2, 30, 2)]
            d.line(path, fill=1, width=1)
    elif effect == "METRONOME_EFFECT":
        d.line([(15.5, 5), (23, 24)], fill=1, width=2)
        dot(d, 23, 24, 2.5)
        d.arc([5.5, 9, 25.5, 29], 200, 340, fill=1, width=1)
        draw_motif(d, tname, 8, 25, 4.0, mirror, bits)
    elif effect == "MIMIC_EFFECT":
        draw_motif(d, tname, 9.5, 16, 6.0, mirror, bits)
        draw_motif(d, tname, 22.5, 16, 6.0, mirror, bits)
    elif effect == "MIRROR_MOVE_EFFECT":
        for yy in range(3, 29, 4):
            d.line([(15.5, yy), (15.5, yy + 2)], fill=1, width=1)
        draw_motif(d, tname, 8, 16, 5.5, mirror, bits)
        draw_motif(d, tname, 23, 16, 5.5, not mirror, bits)
    elif effect == "PAY_DAY_EFFECT":
        draw_motif(d, tname, 8.5, 8.5, 5.0, mirror, bits)
        ring(d, 20, 20, 8, width=2)
        d.line([(20, 16), (20, 24)], fill=1, width=2)
    elif effect == "RAGE_EFFECT":
        draw_motif(d, tname, cx, cy, 9.0, mirror, bits)
        for a_deg in (200, 250, 290, 340):
            a = math.radians(a_deg)
            d.line([(26 + 2 * math.cos(a), 6 + 2 * math.sin(a)),
                    (26 + 5 * math.cos(a), 6 + 5 * math.sin(a))], fill=1, width=1)
    elif effect == "SPLASH_EFFECT":
        draw_motif(d, tname, 15.5, 21, 6.0, mirror, bits)
        for a_deg, rr in ((210, 12), (270, 13), (330, 12)):
            a = math.radians(a_deg)
            sx, sy = 15.5 + rr * math.cos(a), 21 + rr * math.sin(a)
            dot(d, sx, sy, 1.2)
            d.line([(sx, sy), (sx + 2 * math.cos(a), sy + 2 * math.sin(a))],
                   fill=1, width=1)
    elif effect == "SUBSTITUTE_EFFECT":
        d.rectangle([11, 6, 21, 14], outline=1, width=2)
        d.rectangle([9, 15, 23, 27], outline=1, width=2)
        d.point((14, 10), fill=1)
        d.point((18, 10), fill=1)
        draw_motif(d, tname, 27, 27, 3.5, mirror, bits)
    elif effect == "SWIFT_EFFECT":
        for sx, sy in ((9, 9), (21, 14), (13, 24)):
            mini_star(d, sx, sy, 5.0, points=5, phase=bits.take(3) * 0.3)
        d.line([(2, 6), (6, 6)], fill=1, width=1)
        d.line([(24, 25), (29, 25)], fill=1, width=1)
    elif effect == "TRANSFORM_EFFECT":
        cycle_arrows(d, 15.5, 15.5, 12)
        d.ellipse([11, 11, 20, 20], fill=1)
        d.point((14, 14), fill=0)
    elif effect == "THRASH_PETAL_DANCE_EFFECT":
        draw_motif(d, tname, cx, cy, 8.5, mirror, bits)
        d.arc([1, 3, 29, 29], 120, 220, fill=1, width=2)
        d.arc([2, 3, 30, 29], 300, 40, fill=1, width=2)


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


def emit_preview(entries, path: Path):
    scale = 3
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
        big = big.resize((SIDE * scale, SIDE * scale), Image.NEAREST)
        sheet.paste(big, (x0, y0))
        ds.rectangle([x0 - 1, y0 - 1, x0 + SIDE * scale, y0 + SIDE * scale],
                     outline=70)
        ds.text((x0, y0 + SIDE * scale + 2), mid, fill=200, font=font)
    sheet.save(path)


# ── main ─────────────────────────────────────────────────────────────────────

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--preview", type=Path, default=None,
                    help="optional PNG contact-sheet path")
    args = ap.parse_args()

    moves = json.loads(MOVES_JSON.read_text())
    ids = sorted(moves.keys())

    rendered = []          # (id, PIL image) in sorted-id order
    packed_entries = []    # (id, packed bytes) in sorted-id order
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
          f"{total} bytes = {total / 1024:.1f} KiB of flash)")

    if args.preview:
        args.preview.parent.mkdir(parents=True, exist_ok=True)
        emit_preview(rendered, args.preview)
        print(f"wrote preview {args.preview}")


if __name__ == "__main__":
    main()
