#!/usr/bin/env python3
"""Pokémon-style 16×16 item icons for the Fidenza world, one per item category (bag icons).

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


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else "unity-client/Assets/Resources/Sprites")
    out.mkdir(parents=True, exist_ok=True)
    icons = {k: draw(v) for k, v in ICONS.items()}
    icons.update({k: icons[v] for k, v in ALIASES.items()})
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
