#!/usr/bin/env python3
"""Pokémon-style 16×16 item icons for the Fidenza world, one per item category (bag icons), plus
one per collection piece (paintings, holiday photos, holy cards) drawn from the piece's id.

Drawn here from ASCII templates (original art) and outlined in dark ink. The client shows item_<id> when
it exists, otherwise item_<category>, otherwise item_varie. Output: item_<name>.png in Resources/Sprites.

    python3 tools/sprites/items.py [output_dir] [--preview preview.png]
"""
import sys
from pathlib import Path

from PIL import Image

OUT = (34, 28, 38, 255)

PALETTE = {
    "w": "#ffffff", "k": "#3a3440", "g": "#9aa0a8", "G": "#5c6470", "y": "#f6d04d", "Y": "#c8962a",
    "o": "#e8a050", "O": "#b06a28", "b": "#d9a066", "B": "#8f5a2e", "r": "#e04848", "R": "#9a2a2a",
    "p": "#f0a0b8", "P": "#c0607a", "c": "#6ec6ff", "C": "#2f7fc0", "e": "#7ccf6a", "E": "#3f8f3a",
    "v": "#b07ce0", "V": "#6a3fa0", "s": "#f2e6c8", "S": "#c9b48a", "n": "#7a5230", "N": "#4a301c",
}

ICONS = {
    # bread loaf
    "cibo": [
        "................",
        "................",
        "................",
        "....bbbbbbbb....",
        "..bbbobbobbobb..",
        ".bbobbobbobbobb.",
        ".bbbbbbbbbbbbbb.",
        ".bbbbbbbbbbbbbb.",
        ".BbbbbbbbbbbbbB.",
        ".BBbbbbbbbbbbBB.",
        "..BBBBBBBBBBBB..",
        "................",
    ],
    # bottle
    "bevanda": [
        "......kk........",
        "......SS........",
        "......EE........",
        "......EE........",
        ".....EEEE.......",
        "....EeeEEE......",
        "....EeEEEE......",
        "....EsssEE......",
        "....EsrsEE......",
        "....EsssEE......",
        "....EeEEEE......",
        "....EEEEEE......",
    ],
    # medicine bottle with red cross
    "medicina": [
        "................",
        ".....wwwwww.....",
        ".....gggggg.....",
        "....wwwwwwww....",
        "....wwwrrwww....",
        "....wwwrrwww....",
        "....wrrrrrrw....",
        "....wrrrrrrw....",
        "....wwwrrwww....",
        "....wwwrrwww....",
        "....wwwwwwww....",
        "....gggggggg....",
    ],
    # golden bone (relic)
    "reliquia": [
        "................",
        ".yy.........yy..",
        "yyyy.......yyyy.",
        "yyyyy.....yyyyy.",
        ".yyyyyyyyyyyyy..",
        "..YyyyyyyyyyyY..",
        "..YyyyyyyyyyyY..",
        ".yyyyyyyyyyyyy..",
        "yyyyY.....Yyyyy.",
        "yyyY.......Yyyy.",
        ".YY.........YY..",
        "................",
    ],
    # small golden cup
    "reliquia_minore": [
        "................",
        "...yyyyyyyyy....",
        "...yYyyyyyyY....",
        "...yYyyyyyyY....",
        "....YyyyyyY.....",
        ".....YyyyY......",
        "......yyy.......",
        "......YyY.......",
        "......YyY.......",
        "....yyyyyyy.....",
        "....YYYYYYY.....",
        "................",
    ],
    # sword
    "arma": [
        "...........gw...",
        "..........gwg...",
        ".........gwg....",
        "........gwg.....",
        ".......gwg......",
        "......gwg.......",
        "..YY.gwg........",
        "...YYwg.........",
        "...nYY..........",
        "..nN.Y..........",
        ".nN.............",
        ".N..............",
    ],
    # wrench
    "attrezzi": [
        "................",
        "..gg.......gg...",
        "..ggg.....ggg...",
        "...ggggggggg....",
        "....GGGGGGG.....",
        "......gg........",
        "......gg........",
        "......gg........",
        "......gg........",
        "......gg........",
        ".....nnnn.......",
        ".....NNNN.......",
    ],
    # round bomb
    "gadget": [
        "..........y.....",
        ".........yo.....",
        "........k.......",
        ".......kk.......",
        ".....kkkkkk.....",
        "....kkkwkkkk....",
        "...kkkwkkkkkk...",
        "...kkkkkkkkkk...",
        "...kkkkkkkkkk...",
        "....kkkkkkkk....",
        ".....kkkkkk.....",
        "................",
    ],
    # pill
    "sostanze": [
        "................",
        "................",
        "........vvvv....",
        ".......vvvvvv...",
        "......wvvvvvv...",
        ".....wwwvvvv....",
        "....wwwwwvv.....",
        "...wwwwwww......",
        "...wwwwww.......",
        "....wwww........",
        "................",
        "................",
    ],
    # spiral shell fossil
    "fossile": [
        "................",
        ".....SSSSSS.....",
        "...SSssssssSS...",
        "..SssSSSSSSssS..",
        "..SsSssssssSsS..",
        "..SsSsSSSSsSsS..",
        "..SsSsSssSsSsS..",
        "..SsSsSSsSsSsS..",
        "..SsSssssSsSs...",
        "..SssSSSSSssS...",
        "...SSsssssSS....",
        ".....SSSSS......",
    ],
    # gem
    "gemma": [
        "................",
        "....cccccccc....",
        "...cwccCCccCc...",
        "..cwwccCCccCCc..",
        "..CCCCCCCCCCCC..",
        "...cccccccccc...",
        "....cccccCcc....",
        ".....ccccCc.....",
        "......ccCc......",
        ".......cc.......",
        "................",
        "................",
    ],
    # rock / ore
    "minerale": [
        "................",
        "................",
        "......gggg......",
        "....gggggggg....",
        "...ggGgyggggg...",
        "..gggggggGgggg..",
        "..ggyggggggyGg..",
        "..GgggGggggggG..",
        "..GGggggggyGGG..",
        "...GGGGGGGGGG...",
        "................",
        "................",
    ],
    # framed painting
    "arte": [
        "................",
        ".YYYYYYYYYYYYYY.",
        ".YccccccccccccY.",
        ".YccccccyyccccY.",
        ".YcccccccyccccY.",
        ".YcceeccccccccY.",
        ".YceeeeccceeccY.",
        ".YeeeeeeeeeeeeY.",
        ".YEEEEEEEEEEEEY.",
        ".YYYYYYYYYYYYYY.",
        "................",
        "................",
    ],
    # album
    "collezione": [
        "................",
        "...RRRRRRRRRR...",
        "...RrrrrrrrrRs..",
        "...RrsssssrrRs..",
        "...RrsyysssrRs..",
        "...RrssssssrRs..",
        "...RrsssssrrRs..",
        "...RrrrrrrrrRs..",
        "...RrrrrrrrrRs..",
        "...RRRRRRRRRRs..",
        "....ssssssssss..",
        "................",
    ],
    # wrapped gift
    "regali": [
        "................",
        ".....rr..rr.....",
        "......rrrr......",
        "..eeeeerreeeee..",
        "..EEEEErrEEEEE..",
        "..eeeeerreeeee..",
        "..eeeeerreeeee..",
        "..rrrrrrrrrrrr..",
        "..eeeeerreeeee..",
        "..eeeeerreeeee..",
        "..EEEEErrEEEEE..",
        "................",
    ],
    # crown
    "insegne": [
        "................",
        "................",
        "..y....y....y...",
        "..yy..yyy..yy...",
        "..yyy.yyy.yyy...",
        "..yyyyyyyyyyy...",
        "..yyryyyyyryy...",
        "..yyyyyyyyyyy...",
        "..YYYYYYYYYYY...",
        "................",
        "................",
        "................",
    ],
    # scroll / paper
    "documenti": [
        "................",
        "...ssssssssss...",
        "...sggggggggs...",
        "...ssssssssss...",
        "...sggggggggs...",
        "...ssssssssss...",
        "...sgggggggss...",
        "...ssssssssss...",
        "...sggggsssss...",
        "...ssssssrrss...",
        "...sssssrrrrs...",
        "................",
    ],
    # pig head
    "bestiame": [
        "................",
        "...pp......pp...",
        "...ppp....ppp...",
        "...pppppppppp...",
        "..pppppppppppp..",
        "..ppkpppppkppp..",
        "..pppppppppppp..",
        "..pppPPPPPpppp..",
        "..pppPkPkPpppp..",
        "...ppPPPPPppp...",
        "....pppppppp....",
        "................",
    ],
    # wheat sheaf
    "raccolto": [
        "...y...y...y....",
        "..yYy.yYy.yYy...",
        "..yYy.yYy.yYy...",
        "...Y...Y...Y....",
        "....Y..Y..Y.....",
        ".....Y.Y.Y......",
        "......YYY.......",
        ".....rrrrr......",
        "......YYY.......",
        ".....Y.Y.Y......",
        "....Y..Y..Y.....",
        "................",
    ],
    # crate of goods
    "merce": [
        "................",
        "..nnnnnnnnnnnn..",
        "..nbbbbbbbbbbn..",
        "..nbNbbbbbbNbn..",
        "..nbbNbbbbNbbn..",
        "..nbbbNbbNbbbn..",
        "..nbbbbNNbbbbn..",
        "..nbbbNbbNbbbn..",
        "..nbbNbbbbNbbn..",
        "..nbNbbbbbbNbn..",
        "..nnnnnnnnnnnn..",
        "................",
    ],
    # scooter
    "mezzo": [
        "................",
        "..........kk....",
        "..........k.....",
        "..........k.....",
        "..rrrrrrrrk.....",
        ".rrrrrrrrrr.....",
        ".rrwwrrrrrrr....",
        "..rrrrrrrrrrr...",
        "..kk......kk....",
        ".kggk....kggk...",
        "..kk......kk....",
        "................",
    ],
    # pouch (anything else)
    "varie": [
        "................",
        "......n..n......",
        ".......nn.......",
        "......bbbb......",
        ".....bbbbbb.....",
        "....bbbbbbbb....",
        "...bbbbobbbbb...",
        "...bbbbbbbbbb...",
        "...Bbbbbbbbbb...",
        "....BBbbbbBB....",
        ".....BBBBBB.....",
        "................",
    ],
}
ICONS["bustina"] = [
    "................",
    "...cccccccccc...",
    "...cCcCcCcCcc...",
    "...cwcccccccc...",
    "...cwccyyyccc...",
    "...cccyyyyycc...",
    "...cccyywyycc...",
    "...cccyyyyycc...",
    "...ccccyyyccc...",
    "...cccccccccc...",
    "...cCcCcCcCcc...",
    "................",
]
# Categories that share a drawing.
ALIASES = {"sballo": "sostanze"}


def rgb(h):
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), 255)


def draw(rows):
    img = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
    px = img.load()
    top = (16 - len(rows)) // 2
    for y, row in enumerate(rows):
        for x, ch in enumerate(row):
            if ch != ".":
                px[x, y + top] = rgb(PALETTE[ch]) if ch != "k" else rgb(PALETTE["k"])
    # Dark outline around the drawing, as in the handheld games' bag icons.
    edge = [(x, y) for y in range(16) for x in range(16) if px[x, y][3] == 0
            and any(0 <= x + dx < 16 and 0 <= y + dy < 16 and px[x + dx, y + dy][3] > 0 and px[x + dx, y + dy] != OUT
                    for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))]
    for x, y in edge:
        px[x, y] = OUT
    return img


import random
import re

COLLECTIONS_RON = Path(__file__).resolve().parents[2] / "engine/crates/fidenza_world/data/32_collezioni.ron"


def piece_ids():
    """Ids of the collection pieces, from the content file."""
    text = COLLECTIONS_RON.read_text(encoding="utf-8") if COLLECTIONS_RON.exists() else ""
    return re.findall(r'\(id: "((?:opera_borgazzi|foto_pag|foto_di_pag|santino)[a-z0-9_]*)"', text)


def piece_icon(item):
    """A small different picture for every piece, stable for its id: a framed painting, a holiday
    snapshot or a holy card."""
    rnd = random.Random(item)
    img = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
    px = img.load()
    col = lambda: (rnd.randint(40, 230), rnd.randint(40, 230), rnd.randint(40, 230), 255)

    def rect(x0, y0, x1, y1, c):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                px[x, y] = c

    if item.startswith("opera_borgazzi"):
        gold, dark = rgb("#f6d04d"), rgb("#c8962a")
        rect(1, 2, 14, 13, dark)
        rect(2, 3, 13, 12, gold)
        sky, ground = col(), col()
        horizon = rnd.randint(6, 9)
        rect(3, 4, 12, horizon, sky)
        rect(3, horizon + 1, 12, 11, ground)
        # A subject: a blob somewhere in the middle.
        cx, cy, r = rnd.randint(5, 10), rnd.randint(5, 9), rnd.randint(1, 2)
        subject = col()
        for y in range(cy - r, cy + r + 1):
            for x in range(cx - r, cx + r + 1):
                if 3 <= x <= 12 and 4 <= y <= 11 and (x - cx) ** 2 + (y - cy) ** 2 <= r * r + 1:
                    px[x, y] = subject
    elif item.startswith("foto"):
        rect(2, 1, 13, 14, rgb("#ffffff"))
        sky, sea, sand = (rnd.randint(90, 160), rnd.randint(170, 220), 255, 255), (30, rnd.randint(110, 160), rnd.randint(170, 220), 255), rgb("#f2e6c8")
        rect(3, 2, 12, 5, sky)
        rect(3, 6, 12, 8, sea)
        rect(3, 9, 12, 11, sand)
        for _ in range(rnd.randint(1, 3)):
            x, y = rnd.randint(4, 11), rnd.randint(7, 10)
            body = col()
            px[x, y - 1] = rgb("#f2c9a0")
            px[x, y] = body
        if rnd.random() < 0.5:
            px[rnd.randint(9, 11), 3] = rgb("#f6d04d")
    else:
        rect(3, 1, 12, 14, rgb("#c8962a"))
        rect(4, 2, 11, 13, rgb("#f2e6c8"))
        robe = col()
        rect(6, 3, 9, 3, rgb("#f6d04d"))
        rect(6, 4, 9, 6, rgb("#f2c9a0"))
        rect(5, 7, 10, 12, robe)
        rect(7, 7, 8, 12, tuple(min(255, v + 40) for v in robe[:3]) + (255,))
    # Same dark outline as the other icons.
    edge = [(x, y) for y in range(16) for x in range(16) if px[x, y][3] == 0
            and any(0 <= x + dx < 16 and 0 <= y + dy < 16 and px[x + dx, y + dy][3] > 0 and px[x + dx, y + dy] != OUT
                    for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))]
    for x, y in edge:
        px[x, y] = OUT
    return img


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else "unity-client/Assets/Resources/Sprites")
    out.mkdir(parents=True, exist_ok=True)
    icons = {k: draw(v) for k, v in ICONS.items()}
    icons.update({k: icons[v] for k, v in ALIASES.items()})
    # Every collection piece has its own picture.
    icons.update({k: piece_icon(k) for k in piece_ids()})
    for k, img in icons.items():
        img.save(out / f"item_{k}.png")
    if "--preview" in sys.argv:
        path = sys.argv[sys.argv.index("--preview") + 1]
        sheet = Image.new("RGBA", (len(icons) * 18, 18), (240, 240, 235, 255))
        for i, img in enumerate(icons.values()):
            sheet.paste(img, (i * 18 + 1, 1), img)
        sheet = sheet.resize((sheet.width * 4, sheet.height * 4), Image.NEAREST)
        sheet.save(path)
    print(f"{len(icons)} icone in {out}")


if __name__ == "__main__":
    main()
