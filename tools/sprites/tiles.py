#!/usr/bin/env python3
"""Pokémon-style 16×16 terrain tiles for the tile maps (one per legend id): tile_<id>.png.

    python3 tools/sprites/tiles.py [output_dir] [--preview preview.png]
"""
import sys
from pathlib import Path

from PIL import Image

S = 16


def rgb(h, a=255):
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), a)


def noise(x, y, seed):
    n = (x * 374761393 + y * 668265263 + seed * 2246822519) & 0xFFFFFFFF
    n = ((n ^ (n >> 13)) * 1274126177) & 0xFFFFFFFF
    return n % 100


class T:
    def __init__(self, base):
        self.img = Image.new("RGBA", (S, S), rgb(base))
        self.p = self.img.load()

    def set(self, x, y, c):
        if 0 <= x < S and 0 <= y < S:
            self.p[x, y] = rgb(c) if isinstance(c, str) else c

    def rect(self, x0, y0, x1, y1, c):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.set(x, y, c)

    def specks(self, cols, pct, seed):
        for y in range(S):
            for x in range(S):
                n = noise(x, y, seed)
                if n < pct:
                    self.set(x, y, cols[n % len(cols)])


def grass(seed=1):
    t = T("#78c850")
    t.specks(["#5da83a", "#8fd862"], 10, seed)
    for (x, y) in [(3, 4), (11, 2), (7, 11), (13, 13), (1, 13)]:
        t.set(x, y, "#4e9a2e")
        t.set(x + 1, y - 1, "#4e9a2e")
        t.set(x + 2, y, "#4e9a2e")
    return t


def tiles():
    out = {}
    out["erba"] = grass()
    f = grass(2)
    for (x, y, c) in [(3, 3, "#f8f8f8"), (11, 5, "#f06060"), (6, 11, "#f8d030"), (13, 12, "#f8f8f8"), (1, 8, "#f06060")]:
        f.set(x, y, c)
        f.set(x + 1, y, c)
        f.set(x, y + 1, c)
        f.set(x + 1, y + 1, "#d8a820" if c == "#f8d030" else c)
    out["fiori"] = f
    ta = T("#58a838")
    for y0 in (1, 9):
        for x0 in (0, 8):
            for i in range(3):
                ta.set(x0 + 1 + i, y0 + 4 - i, "#3a7a22")
                ta.set(x0 + 5 - i, y0 + 4 - i, "#3a7a22")
                ta.set(x0 + 3, y0 + 1 + i, "#8fd862")
            ta.rect(x0, y0 + 5, x0 + 7, y0 + 6, "#40882a")
    out["erba_alta"] = ta
    c = T("#b8b0a0")
    for row in range(4):
        off = 0 if row % 2 == 0 else 2
        for col in range(-1, 4):
            x0, y0 = col * 4 + off, row * 4
            for y in range(y0, y0 + 3):
                for x in range(x0, x0 + 3):
                    c.set(x, y, "#c8c0b0" if (x + y) % 3 else "#d8d0c0")
            c.set(x0 + 2, y0 + 2, "#9a9282")
    out["lastricato"] = c
    d = T("#d8b878")
    d.specks(["#c0a060", "#e8c888", "#b09058"], 14, 3)
    out["sterrato"] = d
    p = T("#e8d8b8")
    for i in range(S):
        p.set(i, 7, "#d0c0a0")
        p.set(i, 15, "#d0c0a0")
        p.set(7, i, "#d0c0a0")
        p.set(15, i, "#d0c0a0")
    p.set(2, 2, "#f4e8cc")
    p.set(10, 10, "#f4e8cc")
    out["piazza"] = p
    a = T("#686870")
    a.specks(["#5a5a62", "#78787f"], 16, 4)
    out["asfalto"] = a
    s = T("#f0e0a0")
    s.specks(["#e0c880", "#f8f0c0"], 14, 5)
    out["sabbia"] = s
    o = T("#9a6a3a")
    for y in (1, 5, 9, 13):
        o.rect(0, y, 15, y, "#7a4e26")
        o.rect(0, y + 1, 15, y + 1, "#b07e4a")
    out["terra"] = o
    b = T("#b07840")
    for x in (0, 4, 8, 12):
        b.rect(x, 0, x, 15, "#805028")
    b.rect(0, 0, 15, 0, "#704018")
    b.rect(0, 15, 15, 15, "#704018")
    out["ponte"] = b
    w = T("#c89058")
    for y in (3, 7, 11, 15):
        w.rect(0, y, 15, y, "#a07040")
    for (x, y) in [(5, 0), (11, 4), (3, 8), (9, 12)]:
        w.rect(x, y, x, y + 2, "#a07040")
    out["parquet"] = w
    m = T("#f0f0f0")
    for y in range(S):
        for x in range(S):
            if (x // 8 + y // 8) % 2:
                m.set(x, y, "#d8d8e0")
    out["marmo"] = m
    r = T("#c03030")
    r.rect(0, 0, 0, 15, "#e0b040")
    r.rect(15, 0, 15, 15, "#e0b040")
    r.specks(["#a82828"], 10, 6)
    out["tappeto"] = r
    x = T("#a8a8a8")
    for i in range(S):
        x.set(i, 7, "#909090")
        x.set(7, i, "#909090")
    x.specks(["#b8b8b8"], 8, 7)
    out["pietra"] = x
    dm = T("#6a1a2a")
    for y in range(0, S, 4):
        for xx in range(0, S, 4):
            dm.set(xx + (y // 4) % 2 * 2, y, "#c89830")
    out["moquette"] = dm
    g = T("#5a4a40")
    g.specks(["#4a3a30", "#6a5a50"], 18, 8)
    out["grotta"] = g
    z = T("#7a5030")
    for y in range(2, 14, 3):
        z.rect(1, y, 14, y, "#a07040")
    out["zerbino"] = z
    st = T("#a8a8a8")
    for y in range(0, S, 4):
        st.rect(0, y, 15, y, "#707070")
        st.rect(0, y + 1, 15, y + 1, "#c8c8c8")
    out["scala"] = st
    wa = T("#58a0e8")
    for (x0, y0) in [(1, 3), (8, 7), (3, 11), (10, 13), (11, 1)]:
        wa.rect(x0, y0, x0 + 3, y0, "#a8d8ff")
        wa.set(x0 + 1, y0 - 1, "#a8d8ff")
    out["acqua"] = wa
    tr = grass(9)
    for y in range(0, 12):
        for xx in range(1, 15):
            dx, dy = xx - 7.5, y - 5.5
            if dx * dx + dy * dy * 1.3 < 44:
                tr.set(xx, y, "#3a8a3a" if dx + dy > 2 else ("#58b058" if dx + dy < -4 else "#48a048"))
    tr.rect(6, 11, 9, 14, "#7a5030")
    tr.rect(6, 11, 6, 14, "#5a3818")
    for (xx, y) in [(4, 3), (9, 2), (5, 7), (10, 6)]:
        tr.set(xx, y, "#78c870")
    out["albero"] = tr
    bu = grass(10)
    for y in range(5, 15):
        for xx in range(2, 14):
            dx, dy = xx - 7.5, y - 10
            if dx * dx + dy * dy * 1.6 < 30:
                bu.set(xx, y, "#3a8030" if dy > 1 else "#50a040")
    out["cespuglio"] = bu
    mu = T("#8a8a92")
    for row in range(4):
        y0 = row * 4
        mu.rect(0, y0 + 3, 15, y0 + 3, "#5a5a62")
        off = 0 if row % 2 == 0 else 4
        for xx in range(off, S, 8):
            mu.rect(xx, y0, xx, y0 + 3, "#5a5a62")
        mu.rect(0, y0, 15, y0, "#a8a8b0")
    out["muro"] = mu
    fe = grass(11)
    fe.rect(0, 6, 15, 7, "#c89868")
    fe.rect(0, 11, 15, 12, "#c89868")
    for xx in (1, 8):
        fe.rect(xx, 4, xx + 1, 14, "#a07040")
    out["staccionata"] = fe
    pa = T("#e8dcc0")
    pa.rect(0, 12, 15, 15, "#b8a888")
    pa.rect(0, 12, 15, 12, "#8a7a5a")
    for xx in range(0, S, 5):
        pa.rect(xx, 0, xx, 11, "#dccfb0")
    out["parete"] = pa
    pc = T("#f0f0f0")
    pc.rect(0, 4, 15, 11, "#8a5a30")
    pc.rect(0, 4, 15, 5, "#a87848")
    pc.rect(0, 11, 15, 11, "#5a3818")
    out["panca"] = pc
    bc = T("#c89058")
    bc.rect(0, 2, 15, 13, "#7a4a28")
    bc.rect(0, 2, 15, 4, "#a06a38")
    bc.rect(0, 13, 15, 13, "#4a2a18")
    out["bancone"] = bc
    ro = T("#3a3028")
    for y in range(S):
        for xx in range(S):
            dx, dy = xx - 7.5, y - 8
            if dx * dx + dy * dy < 50:
                ro.set(xx, y, "#6a5a4a" if dx + dy < -3 else ("#4a3a30" if dx + dy > 3 else "#5a4a3a"))
    out["roccia"] = ro
    pi = T("#40c0c8")
    for (x0, y0) in [(2, 3), (9, 8), (4, 12)]:
        pi.rect(x0, y0, x0 + 3, y0, "#a0f0f0")
    out["piscina"] = pi
    fo = T("#58a0e8")
    fo.rect(0, 0, 15, 1, "#c0c0c8")
    fo.rect(0, 14, 15, 15, "#c0c0c8")
    fo.rect(0, 0, 1, 15, "#c0c0c8")
    fo.rect(14, 0, 15, 15, "#c0c0c8")
    fo.rect(6, 6, 9, 9, "#e0f4ff")
    out["fontana"] = fo
    sb = T("#a8a8a8")
    for xx in range(1, S, 4):
        sb.rect(xx, 0, xx + 1, 15, "#404048")
    sb.rect(0, 2, 15, 2, "#404048")
    sb.rect(0, 13, 15, 13, "#404048")
    out["sbarre"] = sb
    return {k: v.img for k, v in out.items()}


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else "unity-client/Assets/Resources/Sprites")
    out.mkdir(parents=True, exist_ok=True)
    ts = tiles()
    for k, img in ts.items():
        img.save(out / f"tile_{k}.png")
    print(f"{len(ts)} caselle → {out}")
    if "--preview" in sys.argv:
        path = sys.argv[sys.argv.index("--preview") + 1]
        keys = list(ts)
        sheet = Image.new("RGBA", (len(keys) * 36, 36), (40, 40, 40, 255))
        for i, k in enumerate(keys):
            block = Image.new("RGBA", (32, 32))
            for yy in (0, 16):
                for xx in (0, 16):
                    block.paste(ts[k], (xx, yy))
            sheet.paste(block, (i * 36 + 2, 2))
        sheet.resize((sheet.width * 3, sheet.height * 3), Image.NEAREST).save(path)
        print("anteprima:", path)


if __name__ == "__main__":
    main()
