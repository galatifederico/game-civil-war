#!/usr/bin/env python3
"""Pokémon-style pixel-art buildings (top-down 3/4 view) for the Fidenza world.

Original art drawn procedurally: roof slab with tile rows, light walls, blue windows with a shine, dark
door on the anchor cell, a sign with a small icon. A building is 3×3 cells (48 px) or 4×4 (64 px); the door
is always at the bottom centre, where pawns stand to work. Tall parts (bell tower, windmill) may exceed the
footprint upwards. Output: bld_<id>.png in Resources/Sprites.

    python3 tools/sprites/buildings.py [output_dir] [--preview preview.png]
"""
import sys
from pathlib import Path

from PIL import Image

OUT = (34, 28, 38, 255)


def rgb(h, a=255):
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), a)


def shade(c, f):
    return tuple(max(0, min(255, int(v * f))) for v in c[:3]) + (c[3],)


class Canvas:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        self.px = self.img.load()

    def set(self, x, y, c):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[x, y] = c if isinstance(c, tuple) else rgb(c)

    def rect(self, x0, y0, x1, y1, c):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.set(x, y, c)

    def outline(self):
        pts = []
        for y in range(self.h):
            for x in range(self.w):
                if self.px[x, y][3] == 0:
                    for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                        a, b = x + dx, y + dy
                        if 0 <= a < self.w and 0 <= b < self.h and self.px[a, b][3] > 0:
                            pts.append((x, y))
                            break
        for x, y in pts:
            self.px[x, y] = OUT


# 5×5 sign icons.
ICONS = {
    "cross": ["..#..", "#####", "..#..", "..#..", "..#.."],
    "redcross": [".###.", "#####", "#####", "#####", ".###."],
    "P": ["####.", "#...#", "####.", "#....", "#...."],
    "mug": ["####.", "#..##", "#..#.", "#..##", "####."],
    "bread": [".###.", "#####", "#.#.#", "#####", "....."],
    "grape": [".#.#.", "#.#.#", ".#.#.", "..#..", "....."],
    "gift": ["#.#.#", ".###.", "#####", "#.#.#", "#####"],
    "euro": [".###.", "#....", "####.", "#....", ".###."],
    "ham": ["..##.", ".####", "#####", "####.", ".##.."],
    "anvil": ["#####", ".###.", "..#..", ".###.", "#####"],
    "drop": ["..#..", ".###.", "#####", "#####", ".###."],
    "paper": ["#####", "#.#.#", "#####", "#...#", "#####"],
    "leaf": ["...##", "..###", ".###.", "###..", "#...."],
    "dice": ["#####", "#.#.#", "#####", "#.#.#", "#####"],
    "star": ["..#..", ".###.", "#####", ".#.#.", "#...#"],
    "soup": ["#...#", "#####", "#####", ".###.", "....."],
    "book": ["#####", "#.#.#", "#.#.#", "#.#.#", "#####"],
    "bottle": ["..#..", "..#..", ".###.", ".###.", ".###."],
    "wave": [".....", "#.#.#", ".#.#.", "#.#.#", "....."],
    "lock": [".###.", ".#.#.", "#####", "##.##", "#####"],
    "note": ["..###", "..#.#", "..#.#", "###.#", "##.##"],
    "cone": [".###.", "#####", ".###.", "..#..", "..#.."],
    "film": ["#####", "#.#.#", "#####", "#.#.#", "#####"],
    "ball": [".###.", "#.#.#", "##.##", "#.#.#", ".###."],
}


def icon(cv, name, x0, y0, col):
    for y, row in enumerate(ICONS[name]):
        for x, ch in enumerate(row):
            if ch == "#":
                cv.set(x0 + x, y0 + y, col)


def house(tiles=3, roof="#c0503a", wall="#f1e3c8", sign=None, sign_col="#26202a", chimney=False, awning=None,
          roof_h=16, wall_h=16, flat=False, windows=True, door="#6b3f22", extra_top=0, trim=None):
    w = tiles * 16
    h = tiles * 16 + extra_top
    cv = Canvas(w, h)
    base = h - 2
    wall_top = base - wall_h
    x0, x1 = 3, w - 4
    R, Wc = rgb(roof), rgb(wall)
    # Walls with a darker base line and side edges.
    cv.rect(x0, wall_top, x1, base, Wc)
    cv.rect(x0, base - 1, x1, base, shade(Wc, 0.8))
    cv.rect(x0, wall_top, x0, base, shade(Wc, 0.88))
    cv.rect(x1, wall_top, x1, base, shade(Wc, 0.88))
    if trim:
        cv.rect(x0, wall_top, x1, wall_top + 1, rgb(trim))
    # Roof slab (overhangs the walls by 2 px).
    rt = wall_top - roof_h
    if flat:
        cv.rect(x0 - 1, rt + 6, x1 + 1, wall_top + 1, shade(R, 0.9))
        cv.rect(x0 - 1, rt + 6, x1 + 1, rt + 7, shade(R, 1.15))
    else:
        for y in range(rt, wall_top + 2):
            inset = max(0, (rt + 3 - y))  # rounded ridge
            c = R
            if (y - rt) % 4 == 3:
                c = shade(R, 0.82)
            if y >= wall_top:
                c = shade(R, 0.62)
            cv.rect(x0 - 2 + inset, y, x1 + 2 - inset, y, c)
        cv.rect(x0 + 2, rt, x1 - 2, rt, shade(R, 1.2))
    if chimney:
        cx = x1 - 8
        cv.rect(cx, rt - 5, cx + 4, rt + 3, rgb("#8d6e63"))
        cv.rect(cx, rt - 5, cx + 4, rt - 4, rgb("#5d4037"))
        cv.set(cx + 1, rt - 8, rgb("#cfcfcf", 200))
        cv.set(cx + 2, rt - 9, rgb("#e0e0e0", 180))
        cv.set(cx + 3, rt - 11, rgb("#eeeeee", 150))
    if awning:
        a1, a2 = rgb(awning), rgb("#f5f5f5")
        for x in range(x0 - 1, x1 + 2):
            col = a1 if (x // 3) % 2 == 0 else a2
            cv.rect(x, wall_top, x, wall_top + 3, col)
        for x in range(x0 - 1, x1 + 2, 3):
            cv.set(x, wall_top + 4, a1)
    # Door on the anchor cell (bottom centre).
    dx = w // 2 - 4
    cv.rect(dx - 1, base - 11, dx + 8, base, shade(rgb(door), 0.7))
    cv.rect(dx, base - 10, dx + 7, base, rgb(door))
    cv.rect(dx + 3, base - 10, dx + 4, base, shade(rgb(door), 0.8))
    cv.set(dx + 6, base - 5, rgb("#f2c12e"))
    # Windows.
    if windows:
        wy = base - 11
        for wx in ([x0 + 3, x1 - 9] if tiles == 3 else [x0 + 3, x0 + 12, x1 - 18, x1 - 9]):
            cv.rect(wx - 1, wy - 1, wx + 6, wy + 6, rgb("#6d5a4a"))
            cv.rect(wx, wy, wx + 5, wy + 5, rgb("#8fd3ff"))
            cv.rect(wx, wy + 3, wx + 5, wy + 5, rgb("#6bb9ea"))
            cv.set(wx + 1, wy + 1, rgb("#ffffff"))
            cv.set(wx + 2, wy + 1, rgb("#ffffff"))
    # Sign above the door.
    if sign:
        sx, sy = w // 2 - 4, wall_top - (0 if awning else 1) + (5 if awning else 1)
        sy = max(rt + 2, min(sy, base - 17))
        cv.rect(sx - 1, sy - 1, sx + 7, sy + 5, rgb("#5d4037"))
        cv.rect(sx, sy, sx + 6, sy + 4, rgb("#fff3d6"))
        icon(cv, sign, sx + 1, sy, rgb(sign_col))
    cv.outline()
    return cv.img


def field(crop, sign=None, soil="#8d6e4a"):
    cv = Canvas(48, 40)
    cv.rect(2, 10, 45, 37, rgb(soil))
    for y in range(12, 36, 4):
        cv.rect(3, y + 2, 44, y + 2, shade(rgb(soil), 0.8))
        for x in range(4, 44, 3):
            cv.set(x, y, rgb(crop))
            cv.set(x, y + 1, shade(rgb(crop), 0.75))
            if (x + y) % 2:
                cv.set(x + 1, y, shade(rgb(crop), 1.2))
    # Fence posts.
    for x in range(2, 46, 6):
        cv.rect(x, 8, x, 11, rgb("#a1887f"))
    cv.rect(2, 9, 45, 9, rgb("#bcaaa4"))
    if sign:
        cv.rect(21, 30, 27, 36, rgb("#5d4037"))
        cv.rect(22, 31, 26, 35, rgb("#fff3d6"))
        cv.rect(24, 37, 24, 39, rgb("#5d4037"))
    cv.outline()
    return cv.img


def pen():
    cv = Canvas(48, 40)
    cv.rect(3, 12, 44, 37, rgb("#7a5a3a"))
    for (x, y) in [(10, 20), (22, 26), (33, 18), (16, 31), (38, 30)]:
        cv.rect(x, y, x + 4, y + 2, rgb("#5d4030"))
    # A pig.
    cv.rect(26, 28, 33, 32, rgb("#f4a6b8"))
    cv.rect(24, 29, 25, 30, rgb("#e0788f"))
    cv.set(27, 29, rgb("#26202a"))
    for x in range(2, 47, 5):
        cv.rect(x, 9, x + 1, 37, rgb("#a1887f")) if x in (2, 45) else cv.rect(x, 9, x + 1, 13, rgb("#a1887f"))
    cv.rect(2, 10, 46, 11, rgb("#bcaaa4"))
    cv.rect(2, 36, 46, 37, rgb("#bcaaa4"))
    cv.outline()
    return cv.img


def windmill():
    img = house(tiles=3, roof="#8d6e63", wall="#e0d6c8", sign="bread", extra_top=16, roof_h=10, wall_h=18, windows=False)
    cv = Canvas(img.width, img.height)
    cv.img.paste(img)
    cv.px = cv.img.load()
    cx, cy = 24, 18
    for i in range(-14, 15):
        for t in (-1, 0, 1):
            cv.set(cx + i, cy + i + t, rgb("#f5f0dc"))
            cv.set(cx + i, cy - i + t, rgb("#f5f0dc"))
    for i in range(-14, 15, 3):
        cv.set(cx + i, cy + i, rgb("#8d6e63"))
        cv.set(cx + i, cy - i, rgb("#8d6e63"))
    cv.rect(cx - 1, cy - 1, cx + 1, cy + 1, rgb("#5d4037"))
    cv.outline()
    return cv.img


def church():
    img = house(tiles=4, roof="#8e3b2e", wall="#e8dcc3", sign="cross", sign_col="#b8860b", extra_top=24, roof_h=18, wall_h=20)
    cv = Canvas(img.width, img.height)
    cv.img.paste(img)
    cv.px = cv.img.load()
    base = cv.h - 2
    # Bell tower on the left, standing on the ground in front of the nave.
    tx = 4
    cv.rect(tx, 7, tx + 12, base, rgb("#ddd0b4"))
    cv.rect(tx, 7, tx + 1, base, rgb("#c9bb9c"))
    cv.rect(tx - 1, 3, tx + 13, 8, rgb("#7a3226"))
    cv.rect(tx + 1, 1, tx + 11, 3, rgb("#8e3b2e"))
    cv.rect(tx + 4, 14, tx + 8, 21, rgb("#3e2a1e"))
    cv.set(tx + 6, 17, rgb("#f2c12e"))
    cv.rect(tx + 4, 30, tx + 8, 35, rgb("#8fd3ff"))
    cv.rect(tx + 4, base - 8, tx + 8, base, rgb("#6b3f22"))
    cv.rect(tx + 6, -1, tx + 6, 1, rgb("#f2c12e"))
    cv.set(tx + 5, 0, rgb("#f2c12e"))
    cv.set(tx + 7, 0, rgb("#f2c12e"))
    cv.outline()
    return cv.img


def dome(glass="#9be7c4", base_col="#dfe7e1", sign="leaf"):
    img = house(tiles=3, roof="#b0bec5", wall=base_col, sign=sign, sign_col="#2e7d32", flat=True, roof_h=8, wall_h=14)
    cv = Canvas(img.width, img.height)
    cv.img.paste(img)
    cv.px = cv.img.load()
    cx, cy, r = 24, 29, 14
    for y in range(cy - r, cy + 1):
        for x in range(cx - r, cx + r + 1):
            if (x - cx) ** 2 + (y - cy) ** 2 <= r * r:
                c = rgb(glass)
                if (x - cx) % 5 == 0 or (y - cy) % 5 == 0:
                    c = shade(c, 0.8)
                if (x - cx + 6) ** 2 + (y - cy + 7) ** 2 < 6:
                    c = rgb("#ffffff")
                cv.set(x, y, c)
    cv.outline()
    return cv.img


def obelisk():
    cv = Canvas(32, 56)
    cv.rect(9, 6, 22, 51, rgb("#6fbf4a"))
    cv.rect(9, 6, 11, 51, rgb("#5aa33a"))
    cv.rect(11, 2, 20, 6, rgb("#8ed36a"))
    cv.rect(14, 0, 17, 2, rgb("#b5f08f"))
    for y in range(12, 48, 6):
        cv.rect(14, y, 17, y + 2, rgb("#e6ff8f"))
    cv.rect(5, 50, 26, 54, rgb("#9e9e9e"))
    cv.outline()
    return cv.img


def warehouse():
    img = house(tiles=4, roof="#78909c", wall="#b0bec5", sign="gift", sign_col="#c62828", flat=True, roof_h=10, wall_h=22, windows=False, door="#546e7a")
    cv = Canvas(img.width, img.height)
    cv.img.paste(img)
    cv.px = cv.img.load()
    for x in range(5, 60, 3):
        cv.rect(x, 34, x, 44, rgb("#9fb0b9"))
    cv.rect(40, 42, 56, 61, rgb("#607d8b"))
    for y in range(43, 61, 2):
        cv.rect(41, y, 55, y, rgb("#78909c"))
    cv.outline()
    return cv.img


def stall(awning="#e53935", goods=("#ffca28", "#8bc34a", "#e57373")):
    cv = Canvas(48, 40)
    cv.rect(6, 22, 41, 30, rgb("#a1887f"))
    cv.rect(6, 22, 41, 23, rgb("#bcaaa4"))
    for i, x in enumerate(range(8, 40, 4)):
        cv.rect(x, 20, x + 2, 21, rgb(goods[i % len(goods)]))
    cv.rect(7, 31, 8, 37, rgb("#6d4c41"))
    cv.rect(39, 31, 40, 37, rgb("#6d4c41"))
    cv.rect(7, 8, 8, 21, rgb("#6d4c41"))
    cv.rect(39, 8, 40, 21, rgb("#6d4c41"))
    for x in range(4, 44):
        cv.rect(x, 5, x, 11, rgb(awning) if (x // 4) % 2 == 0 else rgb("#fafafa"))
    for x in range(4, 44, 4):
        cv.set(x + 1, 12, rgb(awning))
    cv.outline()
    return cv.img


def throne():
    cv = Canvas(32, 40)
    cv.rect(8, 4, 23, 30, rgb("#d4a017"))
    cv.rect(10, 8, 21, 22, rgb("#b71c1c"))
    cv.rect(6, 20, 25, 30, rgb("#d4a017"))
    cv.rect(8, 22, 23, 26, rgb("#c62828"))
    cv.rect(7, 31, 9, 36, rgb("#a67c00"))
    cv.rect(22, 31, 24, 36, rgb("#a67c00"))
    for x in (9, 15, 21):
        cv.rect(x, 1, x + 1, 4, rgb("#f2c12e"))
    cv.outline()
    return cv.img


def gnome():
    cv = Canvas(16, 24)
    cv.rect(6, 1, 9, 3, rgb("#d32f2f"))
    cv.rect(5, 4, 10, 7, rgb("#d32f2f"))
    cv.rect(5, 8, 10, 11, rgb("#f2c9a0"))
    cv.set(6, 9, rgb("#26202a"))
    cv.set(9, 9, rgb("#26202a"))
    cv.rect(5, 12, 10, 14, rgb("#f5f5f5"))
    cv.rect(4, 15, 11, 19, rgb("#1e88e5"))
    cv.rect(5, 20, 6, 21, rgb("#5d4037"))
    cv.rect(9, 20, 10, 21, rgb("#5d4037"))
    cv.set(10, 16, rgb("#ff5252"))  # the hidden camera light
    cv.outline()
    return cv.img


def vault():
    cv = Canvas(48, 44)
    cv.rect(6, 8, 41, 41, rgb("#78909c"))
    cv.rect(6, 8, 41, 11, rgb("#90a4ae"))
    cv.rect(10, 14, 37, 38, rgb("#607d8b"))
    for y in range(20, 33):
        for x in range(17, 31):
            if (x - 23.5) ** 2 + (y - 26) ** 2 <= 36:
                cv.set(x, y, rgb("#b0bec5"))
    cv.rect(22, 20, 25, 32, rgb("#455a64"))
    cv.rect(17, 25, 30, 27, rgb("#455a64"))
    icon(cv, "lock", 34, 30, rgb("#f2c12e"))
    cv.outline()
    return cv.img


def altar():
    cv = Canvas(48, 40)
    cv.rect(6, 20, 41, 34, rgb("#8d8d8d"))
    cv.rect(6, 20, 41, 22, rgb("#a8a8a8"))
    cv.rect(18, 24, 29, 31, rgb("#5e5e5e"))
    for x in (9, 15, 32, 38):
        cv.rect(x, 12, x + 1, 19, rgb("#f5f0dc"))
        cv.set(x, 10, rgb("#ffb300"))
        cv.set(x, 11, rgb("#ff7043"))
    icon(cv, "cross", 21, 25, rgb("#c0a060"))
    cv.outline()
    return cv.img


def water_tower():
    cv = Canvas(32, 48)
    cv.rect(6, 4, 25, 20, rgb("#90a4ae"))
    cv.rect(6, 4, 25, 6, rgb("#b0bec5"))
    for x in (8, 23):
        cv.rect(x, 21, x + 1, 44, rgb("#6d4c41"))
    for y in (28, 36):
        cv.rect(8, y, 24, y, rgb("#6d4c41"))
    icon(cv, "drop", 13, 10, rgb("#1e88e5"))
    cv.outline()
    return cv.img


def pitch():
    """Five-a-side pitch: grass with white lines and two goals."""
    cv = Canvas(80, 48)
    for y in range(6, 46):
        for x in range(2, 78):
            cv.set(x, y, rgb("#5aa845") if (x // 6) % 2 == 0 else rgb("#4f9a3c"))
    W = rgb("#f5f5f5")
    cv.rect(4, 8, 75, 8, W)
    cv.rect(4, 43, 75, 43, W)
    cv.rect(4, 8, 4, 43, W)
    cv.rect(75, 8, 75, 43, W)
    cv.rect(39, 8, 40, 43, W)
    for a in range(0, 360, 20):
        import math
        cv.set(int(40 + 6 * math.cos(math.radians(a))), int(25 + 6 * math.sin(math.radians(a))), W)
    for gx in (1, 76):
        cv.rect(gx, 19, gx + 2, 32, rgb("#e0e0e0"))
        cv.rect(gx + (1 if gx == 1 else 0), 20, gx + (2 if gx == 1 else 1), 31, rgb("#9e9e9e"))
    cv.rect(30, 27, 32, 29, W)
    cv.set(31, 28, OUT)
    cv.outline()
    return cv.img


def playground():
    """Sand patch with a swing and a slide."""
    cv = Canvas(48, 36)
    cv.rect(2, 16, 45, 34, rgb("#e8d39a"))
    cv.rect(2, 16, 45, 17, rgb("#f3e2b0"))
    # Swing frame.
    cv.rect(5, 4, 6, 30, rgb("#c62828"))
    cv.rect(22, 4, 23, 30, rgb("#c62828"))
    cv.rect(5, 3, 23, 4, rgb("#8e2020"))
    for sx in (10, 17):
        cv.rect(sx, 5, sx, 22, rgb("#9e9e9e"))
        cv.rect(sx + 3, 5, sx + 3, 22, rgb("#9e9e9e"))
        cv.rect(sx - 1, 23, sx + 4, 24, rgb("#1e88e5"))
    # Slide.
    cv.rect(30, 10, 31, 30, rgb("#757575"))
    for i in range(0, 14):
        cv.rect(32 + i, 10 + i, 34 + i, 11 + i, rgb("#fdd835"))
    cv.rect(29, 9, 33, 10, rgb("#43a047"))
    cv.outline()
    return cv.img


def deposit():
    """Stockpile: crates, a stone heap and ore."""
    cv = Canvas(48, 34)
    cv.rect(2, 14, 45, 32, rgb("#8d7b68"))
    for (x, y) in [(5, 14), (14, 18), (5, 22)]:
        cv.rect(x, y, x + 8, y + 8, rgb("#a1714a"))
        cv.rect(x, y, x + 8, y + 1, rgb("#c49464"))
        cv.rect(x + 4, y, x + 4, y + 8, rgb("#7a5234"))
    for y in range(12, 31):
        half = (y - 12) // 2 + 2
        for x in range(36 - half, 36 + half):
            cv.set(x, y, rgb("#9e9e9e") if (x + y) % 5 else rgb("#bdbdbd"))
    for (x, y, c) in [(30, 26, "#b06a40"), (38, 24, "#e0b020"), (34, 20, "#f080b0"), (40, 28, "#b06a40")]:
        cv.rect(x, y, x + 1, y + 1, rgb(c))
    cv.outline()
    return cv.img


def mushroom_farm():
    cv = Canvas(32, 28)
    cv.rect(1, 18, 30, 27, rgb("#4a3a30"))
    for (x, y, r) in [(8, 14, 6), (21, 12, 7), (14, 21, 4), (26, 22, 3)]:
        cv.rect(x - 1, y, x + 1, y + r, rgb("#efe4c8"))
        for yy in range(y - r, y + 1):
            for xx in range(x - r, x + r + 1):
                if (xx - x) ** 2 + ((yy - y) * 1.6) ** 2 <= r * r:
                    cv.set(xx, yy, rgb("#a85a28") if yy < y - 1 else rgb("#8a4a20"))
        cv.set(x - r // 2, y - r + 2, rgb("#d08a50"))
    cv.outline()
    return cv.img


def chest():
    cv = Canvas(16, 16)
    cv.rect(1, 6, 14, 14, rgb("#8d5524"))
    cv.rect(1, 3, 14, 7, rgb("#a8692e"))
    cv.rect(1, 7, 14, 7, rgb("#5d3a17"))
    for x in (1, 7, 14):
        cv.rect(x, 3, x, 14, rgb("#e0b020"))
    cv.rect(6, 8, 8, 10, rgb("#fff080"))
    cv.outline()
    return cv.img


def banner():
    """Faction banner on a pole (the client tints nothing: the cloth is a neutral heraldic red-gold)."""
    cv = Canvas(16, 32)
    cv.rect(3, 2, 4, 30, rgb("#6d4c41"))
    cv.rect(2, 1, 5, 2, rgb("#f2c12e"))
    cv.rect(5, 4, 13, 14, rgb("#c62828"))
    cv.rect(5, 4, 13, 5, rgb("#e53935"))
    for x in range(5, 14):
        if x % 2 == 0:
            cv.set(x, 15, rgb("#c62828"))
    cv.rect(8, 7, 10, 11, rgb("#f2c12e"))
    cv.rect(1, 29, 6, 30, rgb("#5d4037"))
    cv.outline()
    return cv.img


def build_all():
    return {
        "vigna": field("#7b1fa2", sign=True),
        "luppoleto": field("#9ccc65", sign=True),
        "campo_orzo": field("#e0c068", sign=True),
        "campo_grano": field("#f2c14e", sign=True),
        "orto": field("#66bb6a", sign=True),
        "campo_soia": field("#aed581", sign=True),
        "porcilaia": pen(),
        "cattedrale_idroponica": dome(),
        "monolite_soia": obelisk(),
        "mulino": windmill(),
        "forno": house(roof="#b85c38", sign="bread", chimney=True),
        "cantina": house(roof="#6a1b4d", wall="#e8d8c4", sign="grape", sign_col="#6a1b9a"),
        "birrificio": house(roof="#8d6e63", wall="#efe0c0", sign="mug", sign_col="#b8860b", chimney=True),
        "distilleria": house(roof="#5d4037", wall="#e6d6bc", sign="bottle", sign_col="#6d4c41", chimney=True),
        "macello": house(roof="#9e3a2a", wall="#f0e6e0", sign="ham", sign_col="#b71c1c"),
        "salumificio": house(roof="#b23b3b", wall="#f5e9dc", sign="ham", sign_col="#c62828", awning="#c62828"),
        "bar_estratti": house(roof="#43a047", wall="#f1f8e9", sign="leaf", sign_col="#2e7d32", awning="#66bb6a"),
        "laboratorio_fake_meat": house(roof="#26a69a", wall="#e0f2f1", sign="leaf", sign_col="#00796b", flat=True),
        "fucina": house(roof="#4e4e4e", wall="#a1887f", sign="anvil", sign_col="#37474f", chimney=True),
        "laboratorio_medico": house(roof="#90caf9", wall="#fafafa", sign="redcross", sign_col="#e53935", flat=True),
        "bar_ubriaconi": house(roof="#8d4b2b", wall="#f0dcb8", sign="mug", sign_col="#8d4b2b", awning="#2e7d32"),
        "outlet": house(tiles=4, roof="#5c6bc0", wall="#eceff1", sign="euro", sign_col="#283593", flat=True, trim="#7986cb"),
        "banco_mercato": stall(),
        "spacciatore_casino": house(roof="#3e2723", wall="#6d4c41", sign="dice", sign_col="#212121", windows=False),
        "fumetteria": house(roof="#7e57c2", wall="#ede7f6", sign="book", sign_col="#4527a0", awning="#9575cd"),
        "mensa_poveri": house(roof="#a1887f", wall="#fff8e1", sign="soup", sign_col="#6d4c41", chimney=True),
        "banchetto_santini": stall(awning="#fbc02d", goods=("#fff59d", "#90caf9", "#f48fb1")),
        "stazione_polizia": house(roof="#1e3a8a", wall="#e3e8f5", sign="P", sign_col="#1e3a8a", flat=True, trim="#3949ab"),
        "redazione": house(roof="#455a64", wall="#eceff1", sign="paper", sign_col="#263238"),
        "capannone_regali": warehouse(),
        "gnomo_da_giardino": gnome(),
        "casseforti_cda": vault(),
        "altare_cripta": altar(),
        "trono_ubriaconi": throne(),
        "cattedrale": church(),
        "stabilimento_termale": dome(glass="#81d4fa", base_col="#e1f5fe", sign="wave"),
        "sala_giochi": house(tiles=4, roof="#b71c1c", wall="#3e2723", sign="dice", sign_col="#f2c12e", flat=True, trim="#f2c12e"),
        "acquedotto": water_tower(),
        "osteria": house(roof="#7b3f20", wall="#f3dfc1", sign="grape", sign_col="#8e2448", awning="#8e2448", chimney=True),
        "bocciofila": house(tiles=5, roof="#6d8a3a", wall="#efe6cf", sign="ball", sign_col="#37474f", flat=True, trim="#8bc34a"),
        "cinema": house(tiles=5, roof="#263238", wall="#ffebee", sign="film", sign_col="#b71c1c", flat=True, trim="#e53935", extra_top=4),
        "sala_slot": house(roof="#4a148c", wall="#311b92", sign="dice", sign_col="#f2c12e", flat=True, trim="#f2c12e", door="#212121"),
        "parco_giochi": playground(),
        "campetto": pitch(),
        "balera": house(tiles=5, roof="#c62828", wall="#fff3e0", sign="note", sign_col="#c62828", awning="#fbc02d"),
        "gelateria": house(roof="#f48fb1", wall="#fff8fb", sign="cone", sign_col="#ad1457", awning="#f06292"),
        "deposito": deposit(),
        "fungaia_porcini": mushroom_farm(),
        "scrigno_antico": chest(),
        "stendardo": banner(),
    }


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else "unity-client/Assets/Resources/Sprites")
    out.mkdir(parents=True, exist_ok=True)
    imgs = build_all()
    for k, img in imgs.items():
        img.save(out / f"bld_{k}.png")
    print(f"{len(imgs)} edifici → {out}")
    if "--preview" in sys.argv:
        path = sys.argv[sys.argv.index("--preview") + 1]
        cols = 10
        cw, ch = 72, 96
        rows = (len(imgs) + cols - 1) // cols
        sheet = Image.new("RGBA", (cols * cw, rows * ch), rgb("#7cb342"))
        for i, (k, img) in enumerate(imgs.items()):
            x, y = (i % cols) * cw + (cw - img.width) // 2, (i // cols) * ch + ch - img.height - 4
            sheet.alpha_composite(img, (x, y))
        sheet = sheet.resize((sheet.width * 3, sheet.height * 3), Image.NEAREST)
        sheet.save(path)
        print("anteprima:", path)


if __name__ == "__main__":
    main()
