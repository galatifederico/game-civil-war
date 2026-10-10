#!/usr/bin/env python3
"""Pokémon-style chibi pixel sprites (big head, small body) for the Fidenza world.

Everything is drawn here from ASCII templates, so the art is original (no third-party assets) and can be
regenerated at will. Output: one sheet per race (base layer), one clothing layer to tint with the faction
colour, and one sheet per class accessory. Sheet layout: 3 columns (idle, step A, step B) × 4 rows
(down, left, right, up); every frame is 16×20 px.

    python3 tools/sprites/chibi.py [output_dir] [--preview preview.png]
"""
import sys
from pathlib import Path

from PIL import Image

W, H = 24, 32  # frame; humanoids fill about 18×30, like the overworld trainers of the Pokémon games
DIRS = ["down", "left", "right", "up"]
OUTLINE = (34, 28, 38, 255)

# ── Humanoid parts (16 px wide, drawn at x = 4) ─────────────────────────────
# H hair, L hair light, h hair shadow, S skin, s skin shadow, E eye, W eye shine, M mouth,
# C cloth, l cloth light, c cloth shadow (tinted by the faction), P pants, p pants shadow,
# B shoes, b shoes light.  '.' transparent.
HEAD_DOWN = [
    "......HHHH......",
    "...HHLLLHHHHH...",
    "..HLLLHHHHHHHH..",
    ".HHLHHHHHHHHHHh.",
    ".HHHHHHHHHHHHHh.",
    "HHHHHHHHHHHHHHhh",
    "HHHhSShHHHhSShhh",
    "HHhSSSShHhSSSShh",
    "HhSSSSSSSSSSSSSh",
    "hSSSEESSSSEESSSh",
    "hSSSWESSSSWESSSh",
    "hSSSEESSSSEESSSh",
    ".sSSSSSSSSSSSSs.",
    "..sSSSSMMSSSSs..",
    "....ssssssss....",
]
HEAD_UP = [
    "......HHHH......",
    "...HHLLLHHHHH...",
    "..HLLLHHHHHHHH..",
    ".HLLHHHHHHHHHHh.",
    "HHHHHHHHHHHHHHHh",
    "HHHHHHHHHHHHHHhh",
    "HHHHHHHHHHHHHHhh",
    "HHHHHHHHHHHHHHhh",
    "HHHHHHHHHHHHHhhh",
    "hHHHHHHHHHHHHhhh",
    "hhHHHHHHHHHHhhhh",
    ".hhHHHHHHHHhhhh.",
    "..hhhhhhhhhhhh..",
    "...sSSSSSSSSs...",
    "....ssssssss....",
]
HEAD_LEFT = [
    "......HHHH......",
    "...HHLLLHHHHH...",
    "..HLLLHHHHHHHH..",
    ".HLLHHHHHHHHHHh.",
    "HHHHHHHHHHHHHHHh",
    "HHHHHHHHHHHHHHhh",
    "SShHHhHHHHHHHHhh",
    "SSSSSSShHHHHHHhh",
    "SSSSSSSSSHHHHHhh",
    "SEESSSSSSsSHHHhh",
    "SWESSSSSSssHHhhh",
    "SEESSSSSSSShhhh.",
    ".SSSSSSSSSShhhh.",
    "..sMSSSSSSSshh..",
    "...ssssssss.....",
]
TORSO_FRONT = [
    "....lCCCCCCl....",
    "...lCCCCCCCCC...",
    "..CClCCCCCCCCc..",
    "..CClCCCCCCCcc..",
    "..CCCCCCCCCCcc..",
    "..SSCCCCCCCCSS..",
    "..ssccccccccss..",
    "....PPPPPPPP....",
]
TORSO_UP = [
    "....lCCCCCCl....",
    "...CCCCCCCCCC...",
    "..CCCCCCCCCCcc..",
    "..CCCCCCCCCCcc..",
    "..CCCCCCCCCCcc..",
    "..SSccCCCCccSS..",
    "..ssccccccccss..",
    "....PPPPPPPP....",
]
TORSO_SIDE = [  # facing left; the hand ('A') moves with the step
    ".....lCCCCC.....",
    "....lCCCCCCc....",
    "....CCClCCcc....",
    "....CCClCCcc....",
    "....CCClCCcc....",
    "....CCCCCcccc...",
    "....ccccccccc...",
    ".....PPPPPP.....",
]
HAND_SIDE = [(7, 5), (5, 5), (9, 5)]  # (x, row) of the hand for idle, step A, step B
LEGS_FRONT = [
    ["....PPPPPPPP....", "....PPPppPPP....", "....PPP..PPp....", "....PPP..PPp....", "....pBB..BBp....", "...bBBB..BBBB...", "................"],
    ["....PPPPPPPP....", "....PPPppPPP....", "....PPP..PPp....", "....PPP..BBp....", "....pBB..BBBB...", "...bBBB.........", "................"],
    ["....PPPPPPPP....", "....PPPppPPP....", "....PPP..PPp....", "....bBB..PPp....", "...bBBB..BBp....", ".........BBBB...", "................"],
]
LEGS_SIDE = [
    [".....PPPPPP.....", ".....PPPPpp.....", "......PPPp......", "......PPPp......", "......BBBp......", ".....bBBBB......", "................"],
    [".....PPPPPP.....", "....PPP..ppp....", "...PPP....ppp...", "...PP......pp...", "..bBB......BBp..", "..BB.........BB.", "................"],
    [".....PPPPPP.....", ".....PPPPpp.....", ".....PPp.pp.....", "....PPp...pp....", "....bBB..BBB....", "....BB.....BB...", "................"],
]
HEAD_Y, TORSO_Y, LEGS_Y, X0 = 1, 16, 24, 4

# Race palettes: skin, skin shadow, hair, hair shadow, eye, pants, boots, extras.
RACES = {
    "fidentino": dict(S="#f2c9a0", s="#d9a77e", H="#5a3a22", h="#3f2716", E="#26202a", P="#3b4a6b", B="#2b2b2b"),
    "salsese": dict(S="#eab891", s="#cf9a72", H="#23232e", h="#141420", E="#26202a", P="#4b3b2b", B="#2b2420"),
    "nano": dict(S="#e8b48a", s="#c9936a", H="#b5501f", h="#8a3a14", E="#26202a", P="#5a4632", B="#3a2a1a", extras=["beard"]),
    "elfo_logistica": dict(S="#f5d6b8", s="#dbb898", H="#e8c45a", h="#c49e36", E="#2a4a2a", P="#2f6b3a", B="#5a3a22", extras=["ears"]),
    "rettiliano": dict(S="#6fae5a", s="#4e8a3e", H="#6fae5a", h="#4e8a3e", E="#e8d23a", P="#2f3b2f", B="#1f2a1f", extras=["tail"]),
    "zombie": dict(S="#9db58a", s="#7d9468", H="#3d3d3d", h="#2a2a2a", E="#c0392b", P="#4a4a3a", B="#2a2a2a"),
    "androide": dict(S="#fad9c8", s="#e8bca6", H="#f07ab0", h="#c85a8e", E="#3a6ad8", P="#2a2a3a", B="#1a1a2a", extras=["long_hair"]),
}

# ── Creatures (own templates: down, left; up = down, right = mirrored left) ──
# Letters are looked up in the creature palette.
CREATURES = {
    "maiale": dict(
        pal=dict(A="#f4a6b8", a="#d9849a", N="#e0788f", E="#26202a", F="#c26a80"),
        down=[
            "................", "................", "................", "................", "................",
            "................", "................", "................", "...aa......aa...", "...AAAAAAAAAA...",
            "..AAAAAAAAAAAA..", "..AAEAAAAAAEAA..", "..AAAANNNNAAAA..", "..AAAANaaNAAAA..", "..AAAAAAAAAAAA..",
            "...AAAAAAAAAA...", "...aaaaaaaaaa...", "...AA......AA...", "...FF......FF...", "................"],
        left=[
            "................", "................", "................", "................", "................",
            "................", "................", "................", "....aa..........", "...AAAAAAAAAAa..",
            "..AAAAAAAAAAAAa.", ".NAEAAAAAAAAAAA.", "NNAAAAAAAAAAAAA.", "NaAAAAAAAAAAAAa.", ".AAAAAAAAAAAAAa.",
            "..aaaaaaaaaaaa..", "...AA.AA.AA.AA..", "...FF.FF.FF.FF..", "................", "................"],
    ),
    "dinosauro": dict(
        pal=dict(A="#5d8a3a", a="#44692a", D="#8fbf5a", E="#f2e14b", T="#f5f0dc"),
        down=[
            "................", "................", "................", "....AAAAAAAA....", "...AAAAAAAAAA...",
            "...AEAAAAAAEA...", "...AAAAAAAAAA...", "...ATATATATAA...", "....AAAAAAAA....", "...AAAAAAAAAA...",
            "..AAADDDDDDAAA..", "..AAADDDDDDAAA..", "..AAADDDDDDAAA..", "..AAAADDDDAAAA..", "...AAAAAAAAAA...",
            "...AAA....AAA...", "...AAA....AAA...", "..aaaa....aaaa..", "................", "................"],
        left=[
            "................", "................", "................", "..AAAAAA........", ".AEAAAAAA.......",
            "AAAAAAAAA.......", "ATATATAAA.......", ".AAAAAAAA.......", "....AAAAAAAAA...", "...AAAAAAAAAAAA.",
            "...ADDDDAAAAAAAA", "...ADDDDDAAAAAaa", "....DDDDAAAAAa..", ".....AAAAAAAA...", ".....AA...AA....",
            ".....AA...AA....", "....aaa..aaa....", "................", "................", "................"],
    ),
    "leone": dict(
        pal=dict(A="#e8b04a", a="#c48e2e", M="#8a4a1a", m="#6a3410", E="#26202a", N="#4a2a1a"),
        down=[
            "................", "................", "................", "....MMMMMMMM....", "...MMMMMMMMMM...",
            "..MMMAAAAAAMMM..", "..MMAAEAAEAAMM..", "..MMAAAAAAAAMM..", "..MMAAANNAAAMM..", "..MMMAAAAAAMMM..",
            "...MMMMMMMMMM...", "....AAAAAAAA....", "...AAAAAAAAAA...", "...AAAAAAAAAA...", "...aAAAAAAAAa...",
            "...AA......AA...", "...AA......AA...", "...aa......aa...", "................", "................"],
        left=[
            "................", "................", "................", "...MMMMM........", "..MMMMMMM.......",
            ".AAAAAMMMM......", ".AEAAAMMMM......", "NAAAAAMMMM......", ".AAAAMMMMAAAAA..", "..MMMMMAAAAAAAAa",
            "....AAAAAAAAAAAa", "....AAAAAAAAAA.m", "....AA.AA.AA.AAm", "....AA.AA.AA.AA.", "....aa.aa.aa.aa.",
            "................", "................", "................", "................", "................"],
    ),
    "robot": dict(
        pal=dict(A="#9e9e9e", a="#757575", L="#e53935", G="#4fc3f7", D="#546e7a"),
        down=[
            "................", "................", ".......L........", ".......D........", "...AAAAAAAAAA...",
            "...AGGAAAAGGA...", "...AGGAAAAGGA...", "...AAAAAAAAAA...", "...AADDDDDDAA...", "...aaaaaaaaaa...",
            "....DAAAAAAD....", "...DDAAAAAADD...", "...D.AALLAA.D...", "...D.AAAAAA.D...", ".....aaaaaa.....",
            ".....DD..DD.....", ".....DD..DD.....", "....aaa..aaa....", "................", "................"],
        left=[
            "................", "................", "........L.......", "........D.......", "....AAAAAAAA....",
            "....GGAAAAAA....", "....GGAAAAAA....", "....AAAAAAAA....", "....DDDAAAAA....", "....aaaaaaaa....",
            ".....AAAAAA.....", ".....AADAAA.....", ".....AADAAA.....", ".....AADAAA.....", ".....aaaaaa.....",
            "......DDDD......", "......DDDD......", ".....aaaaa......", "................", "................"],
    ),
    "drone": dict(
        pal=dict(A="#607d8b", a="#455a64", R="#cfd8dc", L="#ff5252", G="#80deea"),
        down=[
            "................", "................", "................", "................", "................",
            "RRRR........RRRR", ".aa..........aa.", "..aa........aa..", "...aAAAAAAAAa...", "...AAAAGGAAAA...",
            "...AAAGGGGAAA...", "...AAAAGGAAAA...", "...aAAAAAAAAa...", "..aa..LAAL..aa..", ".aa..........aa.",
            "RRRR........RRRR", "................", "................", "................", "................"],
        left=None,
    ),
    "droide": dict(
        pal=dict(A="#eceff1", a="#b0bec5", B="#1e88e5", b="#1565c0", L="#e53935", G="#26202a"),
        down=[
            "................", "................", "................", "................", ".....BBBBBB.....",
            "....BAAAAAAB....", "....AAGAALAA....", "....AAAAAAAA....", "...aBBBBBBBBa...", "...AAAABBAAAA...",
            "...AABBAABBAA...", "...AAAAAAAAAA...", "...AABBBBBBAA...", "...AAAAAAAAAA...", "...aaaaaaaaaa...",
            "..bb........bb..", "..AA........AA..", ".aaaa......aaaa.", "................", "................"],
        left=None,
    ),
    "talpa": dict(
        pal=dict(A="#5b4a42", a="#3f322c", N="#f48fb1", E="#16121a", C="#f8bbd0", H="#c0392b"),
        down=[
            "", "", "", "", "", "",
            "......HHHH......", ".....HHHHHH.....", "....AAAAAAAA....", "...AAAAAAAAAA...",
            "..AAEAAAAAAEAA..", "..AAAAANNAAAAA..", "..AAAAAaaAAAAA..", ".CAAAAAAAAAAAAC.",
            ".CCaAAAAAAAAaCC.", "...aaaaaaaaaa...", "..CC........CC..", "", "", ""],
        left=[
            "", "", "", "", "", "",
            ".......HHHH.....", "......HHHHHH....", "....AAAAAAAAA...", "...AAAAAAAAAAA..",
            ".NAEAAAAAAAAAAa.", "NNAAAAAAAAAAAAa.", ".AAAAAAAAAAAAAa.", "..CAAAAAAAAAAa..",
            ".CC.aaaaaaaaa...", "......C...C.....", ".....CC..CC.....", "", "", ""],
    ),
    "verme": dict(
        pal=dict(A="#9c2a4a", a="#6e1a32", L="#c94a6e", E="#f5f0dc", P="#16121a"),
        down=[
            "", "", "", "",
            "......AAAA......", ".....ALLLLA.....", ".....AEPPEA.....", ".....AAAAAA.....",
            "......aaaa......", ".....ALLLLA.....", ".....AAAAAA.....", "......aaaa......",
            ".....ALLLLA.....", ".....AAAAAA.....", "......aaaa......", ".....ALLLLA.....",
            "......AAAA......", "", "", ""],
        left=[
            "", "", "", "", "", "", "", "",
            "..AAAA..........", ".ALLLLA.AA..AA..", ".AEPAAAALLAALLA.", ".AAAAAaAAAaaAAAA",
            "..aaaa.aaa..aaa.", "", "", "", "", "", "", ""],
    ),
    "ragno": dict(
        pal=dict(A="#c8b78a", a="#a08f62", G="#7cb342", E="#c62828", L="#4e3b24"),
        down=[
            "", "", "", "", "", "",
            "L..............L", ".L...AAAAAA...L.", "..L.AAGGGGAA.L..", "LLLLAAGAAGAALLLL",
            "....AAGGGGAA....", "LLLLAAAAAAAALLLL", "..L.AAEAAEAA.L..", ".L...aAAAAa...L.",
            "L.....aaaa.....L", "", "", "", "", ""],
        left=[
            "", "", "", "", "", "",
            "..L....L..L.....", "...L.AAAAAA.L...", "....AAGGGGAAA...", ".LLAAGAAGGAAAA..",
            "...EAAAAAAAAAA..", ".LLAAAAAAAAAAa..", "....aaaaaaaaa...", "...L.L.L.L.L....",
            "..L..L..L..L....", "", "", "", "", ""],
    ),
    "polipo": dict(
        pal=dict(A="#8e44ad", a="#6c3483", E="#f5f0dc", P="#16121a", S="#f1948a", M="#b07cc6"),
        down=[
            "", "", "",
            ".....AAAAAA.....", "....AMMAAAAA....", "...AMMAAAAAAA...", "...AAAAAAAAAA...",
            "...AAEEAAEEAA...", "...AAEPAAEPAA...", "...AAAAAAAAAA...", "....AAAaaAAA....",
            "...aAAAAAAAAa...", "..AAaA.AA.AaAA..", ".AA.AA.AA.AA.AA.", ".A.AS..AA..SA.A.",
            ".S.A..AS.SA..S..", "...S..S...S.....", "", "", ""],
        left=None,
    ),
    "cinghiale": dict(
        pal=dict(A="#6d4c33", a="#4a3220", B="#3a2616", E="#f2e14b", T="#f5f0dc", N="#2a1a10"),
        down=[
            "", "", "", "", "",
            "....BBBBBBBB....", "...BAAAAAAAAB...", "..BAAEAAAAEAAB..", "..AAAAANNAAAAA..",
            "..TAAANNNNAAAT..", "..TAAAAAAAAAAT..", "...AAAAAAAAAA...", "..BAAAAAAAAAAB..",
            "..BAAAAAAAAAAB..", "...aaaaaaaaaa...", "...AA......AA...", "...aa......aa...", "", "", ""],
        left=[
            "", "", "", "", "", "",
            "....BBBBBBBB....", "...BBAAAAAAAABB.", ".AEAAAAAAAAAAAA.", "NNAAAAAAAAAAAAAa",
            "NTAAAAAAAAAAAAAa", ".TAAAAAAAAAAAAa.", "..aaaaaaaaaaaa..", "...AA.AA.AA.AA..",
            "...aa.aa.aa.aa..", "", "", "", "", ""],
    ),
}

ACCESSORIES = {
    # name: (pixels for down, pixels for left) as lists of (x, y, colour); up reuses down, right mirrors left.
}


def hexrgba(h):
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), 255)


def shade(c, k):
    """Lighter (k > 0) or darker (k < 0) version of a colour, keeping the hue."""
    r, g, b, a = c
    if k >= 0:
        return (round(r + (255 - r) * k), round(g + (255 - g) * k), round(b + (255 - b) * k), a)
    return (round(r * (1 + k)), round(g * (1 + k)), round(b * (1 + k)), a)


def blank():
    return [[None] * W for _ in range(H)]


def put(grid, rows, pal, y0=0, x0=0):
    for y, row in enumerate(rows):
        for x, ch in enumerate(row):
            if ch != "." and ch in pal and pal[ch] is not None and 0 <= y0 + y < H and 0 <= x0 + x < W:
                grid[y0 + y][x0 + x] = pal[ch]


def outline(grid):
    """Pokémon-style outline: a dark line in the hue of what it surrounds (darker where two shapes meet)."""
    out = [row[:] for row in grid]
    for y in range(H):
        for x in range(W):
            if grid[y][x] is None:
                n = [grid[b][a] for a, b in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)) if 0 <= a < W and 0 <= b < H and grid[b][a] is not None]
                if n:
                    r = sum(c[0] for c in n) // len(n)
                    g = sum(c[1] for c in n) // len(n)
                    bb = sum(c[2] for c in n) // len(n)
                    out[y][x] = (r * 28 // 100 + 12, g * 24 // 100 + 10, bb * 28 // 100 + 16, 255)
    return out


def mirror(grid):
    return [list(reversed(row)) for row in grid]


def to_img(grid):
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    for y in range(H):
        for x in range(W):
            if grid[y][x] is not None:
                img.putpixel((x, y), grid[y][x])
    return img


def palette(race):
    p = RACES[race]
    pal = {k: hexrgba(v) for k, v in p.items() if k != "extras"}
    pal.setdefault("L", shade(pal["H"], 0.35))
    pal.setdefault("p", shade(pal["P"], -0.3))
    pal.setdefault("b", shade(pal["B"], 0.35))
    pal["W"] = (255, 255, 255, 255)
    pal["M"] = hexrgba("#b8574d")
    return pal


def humanoid_frames(race, hair=None):
    """Returns {dir: [base grids]} and {dir: [cloth grids]} for 3 poses. `hair`: "long" (women) or "bob"
    (non-binary) adds hair down the sides of the head."""
    pal = palette(race)
    extras = list(RACES[race].get("extras", []))
    if hair == "long" and "long_hair" not in extras:
        extras.append("long_hair")
    if hair == "bob":
        extras.append("bob_hair")
    cloth_pal = {"l": (255, 255, 255, 255), "C": (228, 228, 228, 255), "c": (160, 160, 160, 255)}
    base, cloth = {}, {}
    for d in DIRS:
        base[d], cloth[d] = [], []
        for pose in range(3):
            g, c = blank(), blank()
            side = d in ("left", "right")
            head = {"down": HEAD_DOWN, "up": HEAD_UP}.get(d, HEAD_LEFT)
            torso = TORSO_SIDE if side else (TORSO_UP if d == "up" else TORSO_FRONT)
            legs = (LEGS_SIDE if side else LEGS_FRONT)[pose]
            bob = 1 if pose else 0  # the steps sit one pixel lower, as in the games
            body_pal = {k: pal[k] for k in "SsPp"}
            put(g, legs, pal, y0=LEGS_Y, x0=X0)
            put(g, torso, body_pal, y0=TORSO_Y + bob, x0=X0)
            put(c, torso, cloth_pal, y0=TORSO_Y + bob, x0=X0)
            if side:
                hx, hy = HAND_SIDE[pose]
                g[TORSO_Y + bob + hy][X0 + hx] = pal["S"]
                g[TORSO_Y + bob + hy + 1][X0 + hx] = pal["s"]
                c[TORSO_Y + bob + hy][X0 + hx] = c[TORSO_Y + bob + hy + 1][X0 + hx] = None
            if "long_hair" in extras:
                cols = [0, 1, 14, 15] if not side else [10, 11, 12, 13, 14]
                for y in range(8, 22):
                    for x in cols:
                        if d == "up" or g[HEAD_Y + bob + y][X0 + x] is None or y >= 15 and x in (0, 15, 13, 14):
                            g[HEAD_Y + bob + y][X0 + x] = pal["H"] if y < 18 else pal["h"]
                if d == "up":
                    for y in range(15, 21):
                        for x in range(1 + (y > 18), 15 - (y > 18)):
                            g[HEAD_Y + bob + y][X0 + x] = pal["h"] if x >= 12 - (y > 18) else pal["H"]
            put(g, head, pal, y0=HEAD_Y + bob, x0=X0)
            if "bob_hair" in extras:
                for y in range(6, 14):
                    for x in ([0, 1, 14, 15] if not side else [9, 10, 11, 12, 13, 14, 15]):
                        if d == "up" or x in (0, 1, 14, 15) or g[HEAD_Y + bob + y][X0 + x] in (pal["H"], pal["h"], None):
                            g[HEAD_Y + bob + y][X0 + x] = pal["H"] if y < 12 else pal["h"]
            if "long_hair" in extras and d == "down":
                for y in range(8, 18):
                    for x in (0, 15):
                        g[HEAD_Y + bob + y][X0 + x] = pal["H"] if y < 15 else pal["h"]
            if "beard" in extras and d != "up":
                beard = ["..H..........H..", "..HHHHHHHHHHHH..", "...HHHHMMHHHH...", "....HHHHHHHH....", ".....hhhhhh....."] if d == "down" else [
                    "HHHHHHH.........", "HHHMHHHH........", ".HHHHHH.........", "..hhhh.........."]
                put(g, beard, pal, y0=HEAD_Y + bob + 10, x0=X0)
            if "ears" in extras:
                pts = [(-1, 8), (-2, 7), (-1, 9), (16, 8), (17, 7), (16, 9)] if not side else [(9, 7), (10, 6), (11, 5), (9, 8)]
                for (x, y) in pts:
                    g[HEAD_Y + bob + y][X0 + x] = pal["S"]
            if "tail" in extras and d != "down":
                for x, y in ([(12, 22), (13, 23), (14, 24), (15, 24), (16, 23)] if d != "up" else [(8, 24), (8, 25), (9, 26), (10, 27)]):
                    g[y][X0 + x] = pal["s"]
            hair_cols = (pal["H"], pal["h"])
            for y in range(H):
                for x in range(W):
                    if g[y][x] in hair_cols:
                        c[y][x] = None
            if d == "right":
                g, c = mirror(g), mirror(c)
            base[d].append(g)
            cloth[d].append(c)
    return base, cloth


def _norm(rows):
    rows = [r.ljust(16, ".")[:16] for r in rows]
    return (rows + ["." * 16] * 20)[:20]


# Creatures are drawn on a 16×20 grid; the big ones are scaled up 1.5× to stand next to people.
BIG = {"dinosauro", "leone", "cinghiale", "robot", "droide", "maiale"}


def autoshade(grid):
    """Light from the top-left: a lighter rim where the shape faces up/left, a darker one down/right."""
    out = [row[:] for row in grid]
    for y in range(H):
        for x in range(W):
            c = grid[y][x]
            if c is None:
                continue
            up = y == 0 or grid[y - 1][x] is None
            left = x == 0 or grid[y][x - 1] is None
            down = y == H - 1 or grid[y + 1][x] is None
            right = x == W - 1 or grid[y][x + 1] is None
            if up or left:
                out[y][x] = shade(c, 0.22)
            elif down or right:
                out[y][x] = shade(c, -0.22)
    return out


def place(small, big):
    """A 16×20 grid into the frame, bottom-centred (scaled 1.5× when big)."""
    g = blank()
    if big:
        sw, sh = 24, 30
        for y in range(sh):
            for x in range(sw):
                g[H - 1 - sh + y][x] = small[y * 2 // 3][x * 2 // 3]
    else:
        for y in range(20):
            for x in range(16):
                g[H - 21 + y][4 + x] = small[y][x]
    return g


def creature_frames(name):
    spec = dict(CREATURES[name])
    spec["down"] = _norm(spec["down"])
    if spec["left"] is not None:
        spec["left"] = _norm(spec["left"])
    pal = {k: hexrgba(v) for k, v in spec["pal"].items()}
    frames = {}
    for d in DIRS:
        rows = spec["down"] if d in ("down", "up") or spec["left"] is None else spec["left"]
        frames[d] = []
        for pose in range(3):
            small = [[None] * 16 for _ in range(20)]
            for y, row in enumerate(rows):
                for x, ch in enumerate(row):
                    yy = y + (-1 if pose == 1 else 0)
                    if ch != "." and ch in pal and 0 <= yy < 20:
                        small[yy][x] = pal[ch]
            g = autoshade(place(small, name in BIG))
            if d == "right" and spec["left"] is not None:
                g = mirror(g)
            frames[d].append(g)
    return frames


# ── Class accessories (drawn over the head) ──
def acc(name, d):
    """Pixels (x, y, colour) of an accessory for a direction (right = mirrored left)."""
    px = []
    side = d in ("left", "right")

    def rect(x0, y0, x1, y1, col):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                px.append((x, y, col))

    if name == "cap_polizia":
        rect(3, 1, 12, 3, "#1e3a8a")
        rect(4, 1, 11, 1, "#2f55c8")
        if d == "down":
            rect(3, 4, 12, 4, "#111827")
            px.append((7, 2, "#f2d74e"))
        elif side:
            rect(0, 4, 5, 4, "#111827")
    elif name == "elmo":
        rect(2, 1, 13, 6, "#9aa3ad")
        rect(3, 1, 12, 2, "#c5ccd3")
        if d == "down":
            rect(4, 7, 11, 7, "#9aa3ad")
            rect(4, 6, 11, 6, "#2b2b2b")
        elif side:
            rect(1, 6, 5, 6, "#2b2b2b")
    elif name == "cappello_mago":
        rect(6, 0, 9, 0, "#6a1b9a")
        rect(4, 1, 11, 2, "#6a1b9a")
        rect(2, 3, 13, 4, "#7b1fa2")
        px.append((8, 1, "#f2d74e"))
    elif name == "fascia_ninja":
        if d != "up":
            rect(2, 5, 13, 5, "#1f1f1f")
            rect(2, 9, 13, 10, "#1f1f1f")
        rect(2, 4, 13, 4, "#c62828")
        if d in ("up", "right", "left"):
            px.append((13 if d != "left" else 14, 5, "#c62828"))
    elif name == "corona_foglie":
        for x in range(3, 13, 2):
            px.append((x, 2, "#43a047"))
            px.append((x + 1, 3, "#2e7d32"))
    elif name == "cappuccio":
        rect(2, 1, 13, 4, "#5d4037")
        rect(2, 5, 2, 10, "#5d4037")
        rect(13, 5, 13, 10, "#5d4037")
        if d == "up":
            rect(2, 1, 13, 10, "#5d4037")
    elif name == "cappuccio_sith":
        rect(2, 1, 13, 4, "#1a1a1a")
        rect(2, 5, 2, 10, "#1a1a1a")
        rect(13, 5, 13, 10, "#1a1a1a")
        if d == "up":
            rect(2, 1, 13, 10, "#1a1a1a")
    elif name == "berretto_medico":
        rect(3, 1, 12, 3, "#f5f5f5")
        if d != "up":
            rect(7, 1, 8, 3, "#e53935")
            rect(6, 2, 9, 2, "#e53935")
    elif name == "cappellino_boomer":
        rect(3, 1, 12, 3, "#d32f2f")
        if d == "down":
            rect(3, 4, 12, 4, "#b71c1c")
        elif side:
            rect(0, 4, 4, 4, "#b71c1c")
    elif name == "fedora_stampa":
        rect(4, 1, 11, 2, "#5f5f5f")
        rect(2, 3, 13, 3, "#3f3f3f")
        if d != "up":
            rect(10, 1, 11, 2, "#f5f5f5")
    elif name == "corona":
        rect(4, 1, 11, 2, "#f2c12e")
        for x in (4, 7, 11):
            px.append((x, 0, "#f2c12e"))
        px.append((7, 1, "#e53935"))
    elif name == "goth":
        # Black hood with a purple bow, dark lipstick shade on the face.
        rect(2, 1, 13, 3, "#16121c")
        rect(2, 4, 2, 11, "#16121c")
        rect(13, 4, 13, 11, "#16121c")
        if d == "up":
            rect(2, 1, 13, 11, "#16121c")
        else:
            rect(11, 1, 12, 2, "#7b2d9e")
            if d == "down":
                px.append((7, 10, "#3a1240"))
                px.append((8, 10, "#3a1240"))
    elif name == "aureola":
        rect(5, 0, 10, 0, "#fff59d")
    # Drawn on the old 16-px frame (head at x 2..13, y 1..10): scale every pixel onto the new head.
    big = []
    for x, y, c in px:
        for nx in range(4 + (x - 2) * 4 // 3, 4 + (x - 1) * 4 // 3):
            for ny in range(1 + (y - 1) * 3 // 2, 1 + y * 3 // 2):
                big.append((nx, max(0, ny), c))
    px = big
    if d == "right":
        px = [(W - 1 - x, y, c) for x, y, c in px]
    return px


ACCESSORY_NAMES = ["cap_polizia", "elmo", "cappello_mago", "fascia_ninja", "corona_foglie", "cappuccio", "cappuccio_sith",
                   "berretto_medico", "cappellino_boomer", "fedora_stampa", "corona", "aureola", "goth"]


def sheet(frames):
    img = Image.new("RGBA", (W * 3, H * 4), (0, 0, 0, 0))
    for r, d in enumerate(DIRS):
        for c in range(3):
            img.paste(to_img(frames[d][c]), (c * W, r * H))
    return img


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else "unity-client/Assets/Resources/Sprites")
    out.mkdir(parents=True, exist_ok=True)
    made = []
    for race in RACES:
        base, cloth = humanoid_frames(race)
        sheet({d: [outline(g) for g in base[d]] for d in DIRS}).save(out / f"chibi_{race}.png")
        # Cloth outline comes from the base layer, so the tint never covers it.
        sheet(cloth).save(out / f"chibi_{race}_cloth.png")
        # Women (long hair) and non-binary pawns (bob): same body, other hair.
        for hair, suffix in (("long", "f"), ("bob", "nb")):
            hb, _ = humanoid_frames(race, hair)
            sheet({d: [outline(g) for g in hb[d]] for d in DIRS}).save(out / f"chibi_{race}_{suffix}.png")
        made.append(race)
    for name in CREATURES:
        f = creature_frames(name)
        sheet({d: [outline(g) for g in f[d]] for d in DIRS}).save(out / f"chibi_{name}.png")
        made.append(name)
    for name in ACCESSORY_NAMES:
        frames = {}
        for d in DIRS:
            g = blank()
            for x, y, col in acc(name, d):
                if 0 <= x < W and 0 <= y < H:
                    g[y][x] = hexrgba(col)
            g = outline(autoshade(g))
            frames[d] = [g] + [[[None] * W] + [row[:] for row in g[:-1]] for _ in range(2)]
        sheet(frames).save(out / f"acc_{name}.png")
    print(f"sprite: {', '.join(made)}; accessori: {', '.join(ACCESSORY_NAMES)} → {out}")
    if "--preview" in sys.argv:
        path = sys.argv[sys.argv.index("--preview") + 1]
        preview(out, path)


def preview(out, path):
    """Contact sheet: every race with a tinted cloth and an accessory, all directions, 6× zoom."""
    names = list(RACES) + list(CREATURES)
    tints = [(211, 47, 47), (30, 58, 138), (46, 125, 50), (251, 192, 45), (106, 27, 154), (255, 112, 67), (141, 110, 99)]
    accs = ["", "cap_polizia", "elmo", "cappuccio", "corona", "fascia_ninja", "cappello_mago", "berretto_medico", "fedora_stampa",
            "cappellino_boomer", "corona_foglie", "cappuccio_sith", "aureola"]
    cell_w, cell_h = W * 3 + 4, H * 4 + 4
    img = Image.new("RGBA", (cell_w * len(names), cell_h), (60, 90, 60, 255))
    for i, n in enumerate(names):
        base = Image.open(out / f"chibi_{n}.png")
        layer = base.copy()
        cloth_path = out / f"chibi_{n}_cloth.png"
        if cloth_path.exists():
            cloth = Image.open(cloth_path).convert("RGBA")
            t = tints[i % len(tints)]
            px = cloth.load()
            for y in range(cloth.height):
                for x in range(cloth.width):
                    r, g, b, a = px[x, y]
                    if a:
                        px[x, y] = (r * t[0] // 255, g * t[1] // 255, b * t[2] // 255, a)
            layer.alpha_composite(cloth)
            a = accs[i % len(accs)]
            if a:
                layer.alpha_composite(Image.open(out / f"acc_{a}.png"))
        img.alpha_composite(layer, (i * cell_w + 2, 2))
    img = img.resize((img.width * 6, img.height * 6), Image.NEAREST)
    img.save(path)
    print("anteprima:", path)


if __name__ == "__main__":
    main()
