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

W, H = 16, 20
DIRS = ["down", "left", "right", "up"]
OUTLINE = (34, 28, 38, 255)

# ── Humanoid templates ────────────────────────────────────────────────────────
# H hair, h hair shadow, S skin, s skin shadow, E eye, W eye shine, M mouth,
# C cloth (tinted), c cloth shadow (tinted), P pants, B boots.  '.' transparent.
HEAD_DOWN = [
    "................",
    ".....HHHHHH.....",
    "...HHHHHHHHHH...",
    "..HHHHHHHHHHHH..",
    "..HHHHHHHHHHHH..",
    "..HSSHSSSSHSSH..",
    "..SSSSSSSSSSSS..",
    "..SSWESSSSWESS..",
    "..SSEESSSSEESS..",
    "..sSSSSSSSSSSs..",
    "...sSSSMMSSSs...",
]
HEAD_UP = [
    "................",
    ".....HHHHHH.....",
    "...HHHHHHHHHH...",
    "..HHHHHHHHHHHH..",
    "..HHHHHHHHHHHH..",
    "..HHHHHHHHHHHH..",
    "..HHHHHHHHHHHH..",
    "..HHHHHHHHHHHH..",
    "..hHHHHHHHHHHh..",
    "..hhHHHHHHHHhh..",
    "...shhhhhhhhs...",
]
HEAD_LEFT = [
    "................",
    ".....HHHHHH.....",
    "...HHHHHHHHHH...",
    "..HHHHHHHHHHHH..",
    "..HHHHHHHHHHHH..",
    "..SHSSSSHHHHHH..",
    "..SSSSSSSHHHHH..",
    "..WESSSSSSHHHH..",
    "..EESSSSSSShHH..",
    "..sSSSSSSSSShh..",
    "...sMSSSSSSSs...",
]
BODY_FRONT = [
    "....CCCCCCCC....",
    "...SCCCCCCCCS...",
    "...SCCCCCCCCS...",
    "...ScCCCCCCcS...",
    "....cccccccc....",
]
BODY_SIDE = [
    ".....CCCCCC.....",
    ".....CCSCCC.....",
    ".....CCSCCC.....",
    ".....cCSCCc.....",
    ".....cccccc.....",
]
LEGS_FRONT = [
    ["....PPP..PPP....", "....PPP..PPP....", "....BBB..BBB....", "................"],
    ["....PPP..PPP....", "....BBB..PPP....", ".........BBB....", "................"],
    ["....PPP..PPP....", "....PPP..BBB....", "....BBB.........", "................"],
]
LEGS_SIDE = [
    ["......PPPP......", "......PPPP......", ".....BBBBB......", "................"],
    [".....PP..PP.....", "....PP....PP....", "...BB......BB...", "................"],
    ["......PPPP......", "......PPPP......", "......BBBB......", "................"],
]

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


def blank():
    return [[None] * W for _ in range(H)]


def put(grid, rows, pal, y0=0):
    for y, row in enumerate(rows):
        for x, ch in enumerate(row):
            if ch != "." and ch in pal and pal[ch] is not None and 0 <= y0 + y < H:
                grid[y0 + y][x] = pal[ch]


def outline(grid):
    out = [row[:] for row in grid]
    for y in range(H):
        for x in range(W):
            if grid[y][x] is None:
                n = [(x + dx, y + dy) for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))]
                if any(0 <= a < W and 0 <= b < H and grid[b][a] is not None for a, b in n):
                    out[y][x] = OUTLINE
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


def humanoid_frames(race):
    """Returns {dir: [base grids]} and {dir: [cloth grids]} for 3 poses."""
    p = RACES[race]
    pal = {k: hexrgba(v) for k, v in p.items() if k != "extras"}
    pal["W"] = (255, 255, 255, 255)
    pal["M"] = hexrgba("#a0524a")
    extras = p.get("extras", [])
    cloth_pal = {"C": (235, 235, 235, 255), "c": (170, 170, 170, 255)}
    base, cloth = {}, {}
    for d in DIRS:
        base[d], cloth[d] = [], []
        for pose in range(3):
            g, c = blank(), blank()
            side = d in ("left", "right")
            head = {"down": HEAD_DOWN, "up": HEAD_UP}.get(d, HEAD_LEFT)
            body = BODY_SIDE if side else BODY_FRONT
            legs = (LEGS_SIDE if side else LEGS_FRONT)[pose]
            bob = 1 if pose == 0 else 0  # idle breathes one pixel lower than the steps
            put(g, head, pal, y0=bob)
            put(g, body, {"S": pal["S"]}, y0=11 + bob)
            put(c, body, cloth_pal, y0=11 + bob)
            put(g, legs, pal, y0=16)
            if "beard" in extras and d != "up":
                beard = ["..H........H..", "..HHHHHHHHHH..", "...HHHHHHHH...", "....HHHHHH...."] if d == "down" else [
                    "..HHHHH.......", "...HHHH.......", "....HH........", "..............."]
                put(g, [" " + r if False else r for r in beard], {"H": pal["H"]}, y0=8 + bob)
            if "ears" in extras:
                for (x, y) in ([(1, 6), (0, 5), (14, 6), (15, 5)] if d in ("down", "up") else [(10, 6), (11, 5)]):
                    g[y + bob][x] = pal["S"]
            if "long_hair" in extras:
                for y in range(5, 12):
                    for x in ([1, 2, 13, 14] if d in ("down", "up") else [10, 11, 12, 13]):
                        if g[y + bob][x] is None or d == "up":
                            g[y + bob][x] = pal["H"] if y < 11 else pal["h"]
            if "tail" in extras and d != "down":
                tail = [(12, 14), (13, 15), (14, 15), (15, 14)] if d in ("left", "up") else []
                for x, y in tail:
                    g[y][x] = pal["s"]
            if d == "right":
                g, c = mirror(g), mirror(c)
            base[d].append(g)
            cloth[d].append(c)
    return base, cloth


def _norm(rows):
    rows = [r.ljust(16, ".")[:16] for r in rows]
    return (rows + ["." * 16] * 20)[:20]


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
            g = blank()
            y0 = 0 if pose == 0 else (-1 if pose == 1 else 0)
            put(g, rows, pal, y0=y0)
            if pose == 2 and name not in ("drone",):
                g = [row[:] for row in g]
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
    elif name == "aureola":
        rect(5, 0, 10, 0, "#fff59d")
    if d == "right":
        px = [(W - 1 - x, y, c) for x, y, c in px]
    return px


ACCESSORY_NAMES = ["cap_polizia", "elmo", "cappello_mago", "fascia_ninja", "corona_foglie", "cappuccio", "cappuccio_sith",
                   "berretto_medico", "cappellino_boomer", "fedora_stampa", "corona", "aureola"]


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
            frames[d] = [g, [row[:] for row in g], [row[:] for row in g]]
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
