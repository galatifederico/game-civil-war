#!/usr/bin/env python3
"""Builds the Fidenza & Salsomaggiore region: tile maps (data/maps/*.map) and data/70_mappa.ron
(legend, maps, sub-zones, borders between maps, doors, infrastructure networks, building positions).

Everything comes from this one script so that tiles, doors and buildings always agree.

    python3 tools/maps/build_maps.py
"""
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "engine/crates/fidenza_world/data"

LEGEND = [
    (".", "erba", "Erba", True), (",", "fiori", "Prato fiorito", True), ('"', "erba_alta", "Erba alta", True),
    ("=", "lastricato", "Strada lastricata", True), (":", "sterrato", "Sterrato", True), ("_", "piazza", "Pavimentazione", True),
    ("g", "asfalto", "Asfalto", True), ("s", "sabbia", "Sabbia", True), ("o", "terra", "Terra arata", True),
    ("b", "ponte", "Ponte", True), ("w", "parquet", "Pavimento in legno", True), ("m", "marmo", "Marmo", True),
    ("r", "tappeto", "Tappeto rosso", True), ("x", "pietra", "Pavimento in pietra", True), ("D", "moquette", "Moquette del casinò", True),
    ("k", "grotta", "Suolo di grotta", True), ("d", "zerbino", "Zerbino", True), ("e", "scala", "Scala", True),
    ("~", "acqua", "Acqua", False), ("T", "albero", "Albero", False), ("t", "cespuglio", "Cespuglio", False),
    ("#", "muro", "Muro", False), ("f", "staccionata", "Staccionata", False), ("W", "parete", "Parete", False),
    ("p", "panca", "Panca", False), ("c", "bancone", "Bancone", False), ("K", "roccia", "Roccia", False),
    ("P", "piscina", "Piscina termale", False), ("F", "fontana", "Fontana", False), ("|", "sbarre", "Sbarre", False),
    ("q", "muschio", "Muschio delle caverne", True), ("n", "crosta_salina", "Crosta di sale", True),
    ("<", "scala_su", "Scala in salita", True), (">", "scala_giu", "Scala in discesa", True),
    ("u", "porcino_gigante", "Porcino gigante", False), ("M", "fango_bollente", "Fango termale bollente", False),
    ("R", "rupe", "Rupe", False),
]
# Diggable tiles: character → (tile after digging, (item, quantity) yielded).
DIG = {
    "K": ("k", ("pietra", 1)), "O": ("k", ("ferro", 1)), "S": ("k", ("sale", 2)), "G": ("k", ("gemma_sale_rosa", 1)),
    "h": ("k", ("fossile_ammonite", 1)), "j": ("s", ("fossile_dente_squalo", 1)), "Y": ("k", ("fossile_osso_dinosauro", 1)),
    "A": ("k", ("oro_dei_nani", 1)),
}
LEGEND += [
    ("O", "vena_ferro", "Vena di ferro", False), ("S", "vena_sale", "Vena di sale", False), ("G", "gemma", "Gemma di sale rosa", False),
    ("h", "fossile", "Roccia con fossili", False), ("j", "fossile_riva", "Riva con fossili", False), ("Y", "osso_fossile", "Osso di dinosauro nella roccia", False),
    ("A", "vena_oro", "Vena d'oro", False),
]
WALK = {ch: walk for ch, _, _, walk in LEGEND}


def stable_hash(text):
    """Same value on every run (Python's hash() of strings is salted)."""
    return zlib.crc32(text.encode())


class Rnd:
    def __init__(self, seed):
        self.s = seed & 0x7FFFFFFF or 1

    def next(self):
        self.s = (self.s * 1103515245 + 12345) & 0x7FFFFFFF
        return self.s

    def chance(self, p):
        return (self.next() % 10000) < p * 10000

    def range(self, a, b):
        return a + self.next() % (b - a + 1)


def cave(m, x0, y0, x1, y1, open_p, seed, floor="k", rock="K", steps=4):
    """Natural caverns by cellular automata inside a rectangle."""
    r = Rnd(seed)
    w, h = x1 - x0 + 1, y1 - y0 + 1
    g = [[r.chance(open_p) for _ in range(w)] for _ in range(h)]
    for _ in range(steps):
        n = [[False] * w for _ in range(h)]
        for y in range(h):
            for x in range(w):
                c = sum(1 for dy in (-1, 0, 1) for dx in (-1, 0, 1) if (dx or dy) and 0 <= x + dx < w and 0 <= y + dy < h and g[y + dy][x + dx])
                n[y][x] = c >= 5 or (g[y][x] and c >= 4)
        g = n
    for y in range(h):
        for x in range(w):
            if g[y][x]:
                m.g[y0 + y][x0 + x] = floor
            elif m.g[y0 + y][x0 + x] == floor:
                m.g[y0 + y][x0 + x] = rock


def veins(m, ch, count, length, seed, on="K"):
    """Random-walk veins of a mineral through rock."""
    r = Rnd(seed)
    for _ in range(count):
        x, y = r.range(2, m.w - 3), r.range(2, m.h - 3)
        for _ in range(length):
            if m.g[y][x] == on:
                m.g[y][x] = ch
            x = max(1, min(m.w - 2, x + r.range(-1, 1)))
            y = max(1, min(m.h - 2, y + r.range(-1, 1)))


def tunnel(m, a, b, ch="k", width=1):
    """L-shaped tunnel between two points (always walkable)."""
    (x0, y0), (x1, y1) = a, b
    for x in range(min(x0, x1), max(x0, x1) + 1):
        m.rect(x, y0, x, y0 + width - 1, ch)
    for y in range(min(y0, y1), max(y0, y1) + 1):
        m.rect(x1, y, x1 + width - 1, y, ch)


def maze(m, x0, y0, cols, rows, seed, floor="x", wall="K"):
    """Perfect maze (recursive backtracker) of 2-cell corridors."""
    r = Rnd(seed)
    m.rect(x0, y0, x0 + cols * 3, y0 + rows * 3, wall)
    seen = {(0, 0)}
    stack = [(0, 0)]
    m.rect(x0 + 1, y0 + 1, x0 + 2, y0 + 2, floor)
    while stack:
        cx, cy = stack[-1]
        nb = [(cx + dx, cy + dy, dx, dy) for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)) if 0 <= cx + dx < cols and 0 <= cy + dy < rows and (cx + dx, cy + dy) not in seen]
        if not nb:
            stack.pop()
            continue
        nx, ny, dx, dy = nb[r.next() % len(nb)]
        seen.add((nx, ny))
        m.rect(x0 + 1 + nx * 3, y0 + 1 + ny * 3, x0 + 2 + nx * 3, y0 + 2 + ny * 3, floor)
        m.rect(min(x0 + 1 + cx * 3, x0 + 1 + nx * 3), min(y0 + 1 + cy * 3, y0 + 1 + ny * 3),
               max(x0 + 2 + cx * 3, x0 + 2 + nx * 3), max(y0 + 2 + cy * 3, y0 + 2 + ny * 3), floor)
        stack.append((nx, ny))


class Map:
    def __init__(self, mid, name, w, h, fill=".", tags=(), indoor=False, underground=False):
        self.id, self.name, self.w, self.h = mid, name, w, h
        self.g = [[fill] * w for _ in range(h)]
        self.tags, self.indoor, self.underground = list(tags), indoor, underground

    def rect(self, x0, y0, x1, y1, ch):
        for y in range(max(0, y0), min(self.h, y1 + 1)):
            for x in range(max(0, x0), min(self.w, x1 + 1)):
                self.g[y][x] = ch

    def frame(self, ch, openings=()):
        """Border of `ch` except the openings: 'N','S','E','W' → the standard road gaps."""
        for x in range(self.w):
            self.g[0][x] = self.g[self.h - 1][x] = ch
        for y in range(self.h):
            self.g[y][0] = self.g[y][self.w - 1] = ch
        cx, cy = self.w // 2 - 1, 13
        for o in openings:
            if o == "N":
                self.rect(cx, 0, cx + 1, 0, self.road)
            if o == "S":
                self.rect(cx, self.h - 1, cx + 1, self.h - 1, self.road)
            if o == "W":
                self.rect(0, cy, 0, cy + 1, self.road)
            if o == "E":
                self.rect(self.w - 1, cy, self.w - 1, cy + 1, self.road)

    def roads(self, openings, road="=", width=2, to=None):
        """Roads from every opening to the centre (or to `to`)."""
        self.road = road
        cx, cy = (to or (self.w // 2 - 1, 13))
        for o in openings:
            if o == "N":
                self.rect(cx, 0, cx + width - 1, cy + 1, road)
            if o == "S":
                self.rect(cx, cy, cx + width - 1, self.h - 1, road)
            if o == "W":
                self.rect(0, cy, cx + 1, cy + width - 1, road)
            if o == "E":
                self.rect(cx, cy, self.w - 1, cy + width - 1, road)

    def scatter(self, ch, every, on=".", seed=1):
        n = seed
        for y in range(1, self.h - 1):
            for x in range(1, self.w - 1):
                n = (n * 1103515245 + 12345) & 0x7FFFFFFF
                if self.g[y][x] == on and n % every == 0:
                    self.g[y][x] = ch

    def text(self):
        return "\n".join("".join(r) for r in self.g) + "\n"


MAPS, SUBZONES, PORTALS, EDGES, BUILDINGS = [], [], [], [], []


def building(b, mid, x, y, owner=None, name=None):
    BUILDINGS.append((b, mid, x, y, owner, name))


def door(name, a, ax, ay, b, bx, by):
    PORTALS.append((name, (a, ax, ay), (b, bx, by)))


def outdoor(mid, name, openings, tags=(), road="=", trees=True):
    m = Map(mid, name, 40, 28, ".", tags)
    m.road = road
    if trees:
        m.frame("T", openings)
        m.rect(1, 1, 38, 1, "t")
        m.rect(1, 26, 38, 26, "t")
        m.rect(1, 1, 1, 26, "t")
        m.rect(38, 1, 38, 26, "t")
    m.roads(openings, road)
    m.scatter(",", 11, seed=stable_hash(mid) & 0xFFFF)
    m.scatter('"', 17, seed=(stable_hash(mid) >> 3) & 0xFFFF)
    MAPS.append(m)
    return m


def clear_below(m, x, y, w=3, ch=None):
    """Walkable strip below a door so it can be reached."""
    m.rect(x - 1, y + 1, x + 1, y + 1, ch or m.road)
    m.g[y][x] = ch or m.road


def link(m, x, y, road=None):
    """Path from below a door down/up to the nearest road row 13-14 or column 19-20."""
    r = road or m.road
    ty = 13 if y + 1 < 13 else 14
    for yy in range(min(y + 1, ty), max(y + 1, ty) + 1):
        m.g[yy][x] = r


# ── Fidenza ──────────────────────────────────────────────────────────────────
p = outdoor("piazza_garibaldi", "Piazza Garibaldi", "NSEW", ["pubblico", "pattuglia", "caldo", "fidenza"])
p.rect(7, 5, 32, 22, "_")
p.rect(18, 12, 21, 15, "F")
p.rect(3, 19, 10, 25, ".")
for x in (8, 31):
    for y in (6, 21):
        p.g[y][x] = "t"
building("banco_mercato", "piazza_garibaldi", 13, 9, "anarchici_commercio")
building("acquedotto", "piazza_garibaldi", 26, 8, None)
building("redazione", "piazza_garibaldi", 6, 23, "redazione_gazzetta")
clear_below(p, 6, 23, ch="=")
link(p, 6, 23, "=")
building("forno", "piazza_garibaldi", 27, 21, "anarchici_commercio", "Forno di Piazza")
building("stazione_polizia", "piazza_garibaldi", 34, 22, "polizia_neutra")
clear_below(p, 34, 22, ch="=")
link(p, 34, 22, "=")
p.rect(33, 23, 35, 24, "=")
p.g[20][12] = "_"
door("Tombino di Piazza Garibaldi", "piazza_garibaldi", 11, 20, "gallerie", 3, 3)
SUBZONES.append(("redazione", "Redazione della Gazzetta", "piazza_garibaldi", (2, 18, 9, 9), ["fidenza"]))
EDGES += [("piazza_garibaldi", "North", "cattedrale"), ("piazza_garibaldi", "East", "borgo_templari"),
          ("piazza_garibaldi", "South", "strada_provinciale"), ("piazza_garibaldi", "West", "rotonde")]

s = outdoor("cattedrale", "Sagrato del Duomo", "SEW", ["pubblico", "chiesa", "fidenza"])
s.rect(10, 11, 29, 20, "_")
s.rect(18, 3, 21, 12, "_")
building("cattedrale", "cattedrale", 20, 11, "chiesa")
building("banchetto_santini", "cattedrale", 14, 17, "chiesa")
building("mensa_poveri", "cattedrale", 8, 22, "chiesa")
clear_below(s, 8, 22)
link(s, 8, 22)
building("laboratorio_medico", "cattedrale", 31, 22, "chiesa")
clear_below(s, 31, 22)
link(s, 31, 22)
EDGES += [("cattedrale", "East", "fumetteria")]

b = outdoor("borgo_templari", "Borgo dei Templari", "NEW", ["pubblico", "templari", "fidenza", "caldo"])
b.rect(4, 17, 35, 18, "=")
building("bar_ubriaconi", "borgo_templari", 10, 16, "ubriaconi")
building("fucina", "borgo_templari", 28, 16, "templari_borgo")
building("forno", "borgo_templari", 20, 10, "templari_borgo", "Forno del Borgo")
b.rect(19, 11, 21, 12, "=")
b.rect(8, 5, 16, 9, "x")
b.rect(24, 5, 32, 9, "x")
EDGES += [("borgo_templari", "East", "fidenza_village")]

n = outdoor("fumetteria", "Quartiere Nerd", "SW", ["pubblico", "nerd", "fidenza"])
building("fumetteria", "fumetteria", 20, 11, "gilda_nerd")
n.rect(24, 18, 34, 22, "_")
EDGES += [("fumetteria", "South", "borgo_templari")]

v = outdoor("impero_vegano", "Impero Vegano del Monolite", "SE", ["vegano"], road=":")
v.rect(5, 16, 14, 23, "o")
building("monolite_soia", "impero_vegano", 20, 10, "impero_vegano")
v.rect(18, 3, 22, 11, ":")
building("cattedrale_idroponica", "impero_vegano", 9, 11, "impero_vegano")
clear_below(v, 9, 11, ":")
link(v, 9, 11, ":")
building("bar_estratti", "impero_vegano", 30, 11, "impero_vegano")
clear_below(v, 30, 11, ":")
link(v, 30, 11, ":")
building("laboratorio_fake_meat", "impero_vegano", 30, 23, "impero_vegano")
clear_below(v, 30, 23, ":")
v.rect(30, 15, 30, 23, ":")
building("campo_soia", "impero_vegano", 9, 22, "impero_vegano")
EDGES += [("impero_vegano", "East", "cattedrale"), ("impero_vegano", "South", "rotonde")]

r = outdoor("rotonde", "Le Rotonde", "NSE", ["rotonda", "pattuglia"], road="g")
for (cx, cy) in [(20, 13), (20, 5), (20, 22)]:
    r.rect(cx - 3, cy - 2, cx + 3, cy + 3, "g")
    r.rect(cx - 1, cy, cx + 1, cy + 1, ",")
r.rect(4, 4, 12, 10, "_")
r.rect(4, 4, 12, 4, "f")
r.rect(4, 4, 4, 10, "f")
r.rect(12, 4, 12, 8, "f")
r.rect(5, 9, 12, 9, "_")
r.rect(12, 9, 18, 10, "g")
SUBZONES.append(("circolo_boomer", "Circolo dei Boomer", "rotonde", (5, 5, 7, 5), ["boomer"]))
EDGES += [("rotonde", "East", "piazza_garibaldi"), ("rotonde", "South", "campagna_bassa")]

fv = outdoor("fidenza_village", "Fidenza Village", "W", ["pubblico", "shopping"], road="=")
fv.rect(8, 16, 34, 24, "g")
for x in range(9, 34, 3):
    fv.rect(x, 18, x, 22, "_")
fv.rect(12, 4, 30, 14, "_")
building("outlet", "fidenza_village", 21, 12, "cda_fidenza_village")

sp = outdoor("strada_provinciale", "Strada Provinciale", "NSEW", ["strada"], road="g")
sp.rect(1, 20, 38, 21, "~")
sp.rect(19, 20, 20, 21, "b")
sp.rect(3, 22, 36, 22, "s")
sp.rect(3, 19, 36, 19, "s")
sp.g[19][19] = sp.g[19][20] = "g"
sp.g[22][19] = sp.g[22][20] = "g"
EDGES += [("strada_provinciale", "East", "capannone_regali"), ("strada_provinciale", "South", "salsomaggiore_terme")]
WALLS_NOTE = "Torrente Stirone: acqua (~) con il ponte (b) della provinciale"

cy = outdoor("capannone_regali", "Capannone di Babbo Natale Estivo", "W", ["logistica"], road="g")
cy.rect(6, 4, 34, 22, "g")
building("capannone_regali", "capannone_regali", 20, 12, "babbo_natale")
building("banco_mercato", "capannone_regali", 10, 18, "babbo_natale", "Mensa aziendale degli Elfi")
building("banco_mercato", "fumetteria", 28, 20, "gilda_nerd", "Bancarella della Gilda")
building("banco_mercato", "rotonde", 9, 12, "circolo_boomer", "Chiosco del Circolo")

c = outdoor("campagna_bassa", "Campagna della Bassa", "NEW", ["campi"], road=":")
c.rect(2, 8, 37, 9, ":")
c.rect(2, 19, 37, 20, ":")
row_a = [("mulino", 6), ("forno", 12), ("cantina", 18), ("birrificio", 25), ("distilleria", 31)]
for bid, x in row_a:
    building(bid, "campagna_bassa", x, 7, "contadini_bassa")
row_b = [("macello", 6), ("salumificio", 12), ("porcilaia", 26), ("orto", 32)]
for bid, x in row_b:
    building(bid, "campagna_bassa", x, 18, "contadini_bassa")
for i, (bid, x) in enumerate([("vigna", 4), ("luppoleto", 10), ("campo_orzo", 16), ("campo_grano", 22), ("campo_grano", 28), ("orto", 34)]):
    c.rect(x - 1, 22, x + 1, 24, "o")
    building(bid, "campagna_bassa", x, 25, "contadini_bassa")
c.rect(2, 25, 37, 25, ":")

# ── Salsomaggiore ───────────────────────────────────────────────────────────
t = outdoor("salsomaggiore_terme", "Salsomaggiore e le Terme", "NES", ["pubblico", "terme", "salso", "caldo"])
t.rect(12, 4, 28, 12, "_")
building("stabilimento_termale", "salsomaggiore_terme", 20, 10, "cripta_san_vitale")
t.rect(6, 17, 34, 23, "_")
building("banco_mercato", "salsomaggiore_terme", 12, 20, "anarchici_commercio")
building("forno", "salsomaggiore_terme", 28, 21, "contadini_bassa")
t.rect(27, 22, 29, 22, "_")
building("orto", "salsomaggiore_terme", 6, 25, "contadini_bassa")
EDGES += [("salsomaggiore_terme", "East", "casino")]

k = outdoor("casino", "Casinò Diablo Tentator", "W", ["pubblico", "vizio", "caldo"])
k.rect(10, 4, 32, 22, "_")
building("sala_giochi", "casino", 21, 12, "casino_diablo")

# ── Interiors ────────────────────────────────────────────────────────────────
def interior(mid, name, w, h, floor, tags=(), mat=None):
    m = Map(mid, name, w, h, floor, tags, indoor=True)
    m.frame("W")
    m.rect(1, 1, w - 2, 1, "W")
    mx = w // 2
    m.g[h - 1][mx] = "W"
    m.g[h - 2][mx] = "d"
    MAPS.append(m)
    return m, (mx, h - 2)


di, dm = interior("duomo_interno", "Interno del Duomo", 24, 18, "m", ["chiesa", "sacro"])
di.rect(10, 2, 13, 15, "r")
for y in range(5, 15, 2):
    di.rect(3, y, 8, y, "p")
    di.rect(15, y, 20, y, "p")
di.rect(8, 2, 15, 3, "x")
door("Portone del Duomo", "cattedrale", 20, 11, "duomo_interno", *dm)

ps, pm = interior("stazione_polizia", "Comando di Polizia", 20, 14, "x", ["fidenza", "pattuglia"])
ps.rect(12, 2, 18, 7, "x")
ps.rect(12, 8, 18, 8, "|")
ps.rect(12, 2, 12, 8, "|")
ps.g[8][15] = "x"
ps.rect(3, 4, 8, 4, "c")
SUBZONES.append(("celle", "Celle di Detenzione", "stazione_polizia", (13, 2, 5, 6), ["celle"]))
door("Ingresso del Comando", "piazza_garibaldi", 34, 22, "stazione_polizia", *pm)

bu, bm = interior("bar_ubriaconi", "Bar degli Ubriaconi", 20, 14, "w", ["pubblico", "caldo", "fidenza"])
bu.rect(2, 7, 11, 7, "c")
bu.rect(13, 2, 17, 5, "r")
building("trono_ubriaconi", "bar_ubriaconi", 15, 4, "ubriaconi")
SUBZONES.append(("trono_ubriaconi", "Trono degli Ubriaconi", "bar_ubriaconi", (13, 2, 5, 4), ["trono"]))
door("Porta del Bar", "borgo_templari", 10, 16, "bar_ubriaconi", *bm)

ou, om = interior("outlet_interno", "Negozi dell'Outlet", 26, 16, "m", ["shopping"])
for x in range(3, 23, 5):
    ou.rect(x, 4, x + 2, 9, "c")
building("casseforti_cda", "outlet_interno", 21, 4, "cda_fidenza_village")
door("Ingresso dell'Outlet", "fidenza_village", 21, 12, "outlet_interno", *om)

ti, tm = interior("terme_interno", "Stabilimento Termale", 24, 16, "m", ["terme", "salso"])
ti.rect(6, 4, 17, 9, "P")
ti.rect(2, 2, 3, 3, "e")
door("Scale delle viscere", "terme_interno", 2, 2, "cripta", 3, 3)
door("Ingresso delle Terme", "salsomaggiore_terme", 20, 10, "terme_interno", *tm)

sc, sm = interior("sala_casino", "Sala da gioco", 22, 14, "D", ["vizio", "caldo"])
for (x, y) in [(4, 4), (9, 4), (4, 8), (9, 8)]:
    sc.rect(x, y, x + 2, y + 1, "c")
building("spacciatore_casino", "sala_casino", 17, 6, "casino_diablo")
door("Porta della Sala da gioco", "casino", 21, 12, "sala_casino", *sm)

# ── Wild surface ────────────────────────────────────────────────────────────
bo = outdoor("bosco_stirone", "Bosco dello Stirone", "E", ["bosco", "selvatico"], road=":")
bo.scatter("T", 4, seed=21)
bo.scatter("t", 9, seed=22)
bo.rect(15, 1, 18, 26, "~")
for y in range(1, 27):
    bo.g[y][14] = "j" if y % 3 else "s"
    bo.g[y][19] = "j" if y % 4 == 1 else "s"
bo.rect(14, 13, 19, 14, "b")
bo.rect(20, 13, 39, 14, ":")
bo.rect(2, 13, 13, 14, ":")
for (x, y) in [(6, 5), (8, 20), (28, 6), (32, 21)]:
    bo.rect(x - 2, y - 2, x + 2, y + 2, ",")
EDGES += [("campagna_bassa", "West", "bosco_stirone")]

co = outdoor("colline_di_salso", "Colline di Salso", "N", ["colline", "selvatico"], road=":")
r = Rnd(31)
for _ in range(14):
    cx, cy = r.range(4, 35), r.range(4, 23)
    for y in range(cy - 2, cy + 3):
        for x in range(cx - 3, cx + 4):
            if 1 < x < 38 and 1 < y < 26 and abs(x - cx) + abs(y - cy) < 5 and co.g[y][x] not in ":=":
                co.g[y][x] = "R"
co.rect(27, 18, 33, 22, "R")
co.rect(29, 21, 31, 22, "k")
co.rect(19, 14, 30, 15, ":")
co.rect(30, 15, 30, 22, ":")
door("Bocca della Miniera di Sale", "colline_di_salso", 30, 22, "miniera_di_sale", 3, 3)
EDGES += [("salsomaggiore_terme", "South", "colline_di_salso")]

# ── Under Fidenza: the dwarves' fortress and the depths ─────────────────────
def under(mid, name, w, h, depth, tags):
    m = Map(mid, name, w, h, "K", tags, underground=True)
    m.depth = depth
    MAPS.append(m)
    return m


gl = under("gallerie", "Fortezza dei Nani", 64, 40, -1, ["gallerie", "sotterraneo", "fortezza"])
veins(gl, "O", 14, 18, 101)
veins(gl, "S", 8, 14, 102)
veins(gl, "A", 2, 5, 103)
gl.rect(2, 2, 8, 8, "x")                          # entrance from the manhole
gl.rect(8, 5, 40, 6, "x")                         # main corridor
gl.rect(20, 8, 34, 18, "x")                       # great hall
gl.rect(26, 7, 27, 7, "x")
gl.rect(40, 3, 52, 11, "x")                       # forge room
gl.rect(36, 12, 38, 22, "x")
cave(gl, 4, 22, 20, 36, 0.55, 104)                 # dinosaur den (natural)
tunnel(gl, (10, 6), (10, 24), "x")
cave(gl, 42, 18, 61, 35, 0.48, 105)                # the mine
veins(gl, "O", 10, 10, 106, on="K")
tunnel(gl, (37, 22), (45, 26), "k")
gl.rect(58, 33, 60, 36, "k")
tunnel(gl, (50, 30), (59, 34), "k")
gl.g[35][59] = ">"
building("deposito", "gallerie", 27, 17, "nani_miniere")
building("fungaia_porcini", "gallerie", 23, 11, "nani_miniere", "Fungaia della Fortezza")
building("banco_mercato", "gallerie", 31, 11, "nani_miniere", "Dispensa dei Nani")
building("fungaia_porcini", "miniera_di_sale", 24, 20, None, "Fungaia dei minatori")
building("fucina", "gallerie", 46, 10, "nani_miniere")
SUBZONES += [("miniera", "Filone di ferro", "gallerie", (42, 18, 20, 18), ["miniera", "gallerie"]),
             ("tana_dinosauri", "Tana dei Dinosauri", "gallerie", (4, 22, 17, 15), ["gallerie", "tana"])]

mp = under("miniere_profonde", "Miniere Profonde", 72, 44, -2, ["sotterraneo", "profondo", "miniera"])
cave(mp, 2, 2, 69, 41, 0.42, 201)
veins(mp, "S", 20, 22, 202)
veins(mp, "O", 14, 16, 203)
veins(mp, "G", 8, 6, 204)
veins(mp, "h", 10, 6, 205)
veins(mp, "Y", 3, 3, 206)
veins(mp, "A", 4, 5, 207)
for y in range(4, 40):
    x = 34 + (y // 5) % 3
    mp.g[y][x] = "~"
mp.rect(2, 2, 6, 6, "k")
mp.g[3][3] = "<"
tunnel(mp, (5, 5), (66, 39), "k")
mp.rect(32, 21, 38, 21, "b")
mp.g[40][67] = ">"
door("Scala delle Miniere Profonde", "gallerie", 59, 35, "miniere_profonde", 3, 3)

cv = under("caverne", "Caverne dei Porcini", 80, 48, -3, ["sotterraneo", "profondo", "caverna"])
cave(cv, 2, 2, 77, 45, 0.58, 301)
for i, (x, y) in enumerate([(20, 12), (52, 30), (64, 10), (14, 34)]):
    for yy in range(y - 4, y + 5):
        for xx in range(x - 6, x + 7):
            if cv.g[yy][xx] == "k":
                cv.g[yy][xx] = "u" if (xx * 7 + yy * 13 + i) % 5 == 0 else "q"
cv.rect(36, 18, 46, 26, "~")
veins(cv, "G", 6, 5, 302)
cv.rect(2, 2, 6, 6, "k")
cv.g[3][3] = "<"
tunnel(cv, (5, 5), (74, 43), "q")
cv.rect(36, 22, 46, 22, "b")
cv.g[44][75] = ">"
for (x, y) in [(20, 17), (52, 35), (64, 15)]:
    cv.rect(x - 1, y - 1, x + 1, y + 1, "q")
    building("fungaia_porcini", "caverne", x, y, None)
door("Scala delle Caverne", "miniere_profonde", 67, 40, "caverne", 3, 3)

ct = under("cuore_termale", "Cuore Termale", 60, 40, -4, ["sotterraneo", "profondo", "termale"])
ct.rect(2, 2, 57, 37, "M")
cave(ct, 2, 2, 57, 37, 0.35, 401, floor="n", rock="M")
ct.rect(2, 2, 7, 7, "n")
ct.g[3][3] = "<"
tunnel(ct, (5, 5), (30, 20), "n")
ct.rect(27, 17, 33, 23, "n")
building("scrigno_antico", "cuore_termale", 30, 20, None)
door("Scala del Cuore Termale", "caverne", 75, 44, "cuore_termale", 3, 3)

# ── Under Salsomaggiore: the crypt, the catacombs and the salt mine ─────────
cr = under("cripta", "Cripta di San Vitale", 48, 32, -1, ["cripta", "sotterraneo"])
maze(cr, 1, 1, 15, 10, 501)
cr.rect(18, 12, 29, 20, "x")
cr.rect(2, 2, 5, 5, "x")
cr.g[3][3] = "e"
cr.g[29][44] = ">"
cr.rect(40, 26, 45, 29, "x")
tunnel(cr, (29, 16), (43, 27), "x")
building("altare_cripta", "cripta", 24, 17, "cripta_san_vitale")

ca = under("catacombe", "Catacombe di San Vitale", 60, 40, -2, ["cripta", "sotterraneo", "profondo"])
maze(ca, 1, 1, 19, 12, 601)
veins(ca, "h", 8, 4, 602)
ca.rect(2, 2, 5, 5, "x")
ca.g[3][3] = "<"
door("Scala delle Catacombe", "cripta", 44, 29, "catacombe", 3, 3)

ms = under("miniera_di_sale", "Miniera di Sale abbandonata", 64, 40, -1, ["sotterraneo", "miniera", "salso"])
cave(ms, 2, 2, 61, 37, 0.45, 701)
veins(ms, "S", 28, 24, 702)
veins(ms, "G", 10, 6, 703)
ms.rect(2, 2, 7, 7, "n")
ms.g[3][3] = "<"
tunnel(ms, (6, 6), (40, 30), "n")
ms.rect(10, 8, 18, 12, "n")
building("deposito", "miniera_di_sale", 14, 11, None)


# ── One continuous world, Dwarf Fortress style ───────────────────────────────
# The maps above are kept as regions: the outdoor ones are stitched by their borders into a single
# surface layer, the underground ones are carved into full-size z-levels of diggable rock under their
# entrances. Interiors stay separate maps reached through their doors.
RW, RH = 40, 28
SIDES = {"North": (0, -1), "South": (0, 1), "East": (1, 0), "West": (-1, 0)}
ZLEVELS = {-1: "Sottosuolo", -2: "Profondità", -3: "Caverne Profonde", -4: "Viscere Termali"}
REGIONS = []  # (zone id, name, layer id, rect, tags)
PLACE = {}  # old map id → (layer id, x offset, y offset)


def grid_positions(outdoor):
    pos = {outdoor[0].id: (0, 0)}
    changed = True
    while changed:
        changed = False
        for a, side, b in EDGES:
            dx, dy = SIDES[side]
            if a in pos and b not in pos:
                pos[b] = (pos[a][0] + dx, pos[a][1] + dy)
                changed = True
            if b in pos and a not in pos:
                pos[a] = (pos[b][0] - dx, pos[b][1] - dy)
                changed = True
    return pos


def countryside(m, ox, oy, seed, sides):
    """Filler region between the named places: meadows, woods, a pond, dirt tracks to the neighbours."""
    r = Rnd(seed)
    for y in range(oy, oy + RH):
        for x in range(ox, ox + RW):
            m.g[y][x] = "."
    cave(m, ox + 1, oy + 1, ox + RW - 2, oy + RH - 2, 0.6 + (seed % 3) * 0.05, seed, floor=".", rock="T", steps=3)
    if r.chance(0.6):
        cx, cy, rx, ry = ox + r.range(10, 30), oy + r.range(8, 20), r.range(3, 6), r.range(2, 4)
        for y in range(cy - ry - 1, cy + ry + 2):
            for x in range(cx - rx - 1, cx + rx + 2):
                d = ((x - cx) / (rx + 1)) ** 2 + ((y - cy) / (ry + 1)) ** 2
                if d <= 1.0:
                    m.g[y][x] = "~" if ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1.0 else "s"
    for y in range(oy, oy + RH):
        for x in range(ox, ox + RW):
            if m.g[y][x] == "." and r.chance(0.08):
                m.g[y][x] = "," if r.chance(0.5) else '"'
            elif m.g[y][x] == "T" and r.chance(0.12):
                m.g[y][x] = "t"
    cx, cy = ox + RW // 2 - 1, oy + 13
    for side in sides:
        dx, dy = SIDES[side]
        if dx:
            x0, x1 = (cx, ox + RW - 1) if dx > 0 else (ox, cx + 1)
            m.rect(x0, cy, x1, cy + 1, ":")
        else:
            y0, y1 = (cy, oy + RH - 1) if dy > 0 else (oy, cy + 1)
            m.rect(cx, y0, cx + 1, y1, ":")


def free_spot(placed, w, h, want, W, H):
    """Nearest offset to `want` where a w×h rectangle fits inside W×H without touching `placed`."""
    def ok(x, y):
        if x < 1 or y < 1 or x + w > W - 1 or y + h > H - 1:
            return False
        return all(x + w + 2 <= px or px + pw + 2 <= x or y + h + 2 <= py or py + ph + 2 <= y for px, py, pw, ph in placed)
    wx, wy = want
    for rad in range(0, max(W, H)):
        best = None
        for y in range(wy - rad, wy + rad + 1):
            for x in range(wx - rad, wx + rad + 1):
                if max(abs(x - wx), abs(y - wy)) == rad and ok(x, y):
                    d = (x - wx) ** 2 + (y - wy) ** 2
                    if best is None or d < best[0]:
                        best = (d, x, y)
        if best:
            return best[1], best[2]
    raise SystemExit(f"nessuno spazio per {w}x{h}")


def merge():
    global MAPS, PORTALS, EDGES, SUBZONES, BUILDINGS
    outdoor = [m for m in MAPS if not m.indoor and not m.underground]
    interiors = [m for m in MAPS if m.indoor]
    unders = [m for m in MAPS if m.underground]
    pos = grid_positions(outdoor)
    minx, miny = min(p[0] for p in pos.values()), min(p[1] for p in pos.values())
    cols, rows = max(p[0] for p in pos.values()) - minx + 1, max(p[1] for p in pos.values()) - miny + 1
    W, H = cols * RW, rows * RH
    surf = Map("fidenza", "Fidenza e Salsomaggiore", W, H, ".", ["superficie"])
    place = {}  # map id → (layer id, ox, oy)
    by_cell = {(p[0] - minx, p[1] - miny): mid for mid, p in pos.items()}
    for m in outdoor:
        gx, gy = pos[m.id][0] - minx, pos[m.id][1] - miny
        ox, oy = gx * RW, gy * RH
        place[m.id] = ("fidenza", ox, oy)
        rnd = Rnd(stable_hash(m.id) & 0xFFFF)
        for y in range(m.h):
            for x in range(m.w):
                ch = m.g[y][x]
                ring = min(x, y, m.w - 1 - x, m.h - 1 - y)
                # The old borders between maps open up: a few trees stay, the hedges go.
                if ring == 0 and ch == "T":
                    ch = "T" if rnd.chance(0.2) else ("t" if rnd.chance(0.15) else ".")
                elif ring == 1 and ch == "t":
                    ch = "t" if rnd.chance(0.1) else "."
                surf.g[oy + y][ox + x] = ch
        REGIONS.append((m.id, m.name, "fidenza", (ox, oy, m.w, m.h), m.tags + ["quartiere"]))
    for gy in range(rows):
        for gx in range(cols):
            if (gx, gy) in by_cell:
                continue
            sides = [s for s, (dx, dy) in SIDES.items() if 0 <= gx + dx < cols and 0 <= gy + dy < rows]
            countryside(surf, gx * RW, gy * RH, 7919 * (gx + 1) + 104729 * (gy + 1), sides)
    # Edge of the world: the woods of the Po valley.
    for x in range(W):
        surf.g[0][x] = surf.g[H - 1][x] = "T"
    for y in range(H):
        surf.g[y][0] = surf.g[y][W - 1] = "T"

    # Underground z-levels: solid rock with veins and pockets, the old dungeons carved in.
    levels = {}
    for d, name in ZLEVELS.items():
        z = Map(f"sottosuolo_{-d}", f"{name} (livello {d})", W, H, "K", ["sotterraneo"], underground=True)
        z.depth = d
        n = W * H // 2000
        veins(z, "O", n * 2, 16, 300 + d)
        veins(z, "S", n, 12, 310 + d)
        veins(z, "G", max(1, n // 3) * (1 - d) // 2, 4, 320 + d)
        veins(z, "A", max(1, n // 4) * -d, 4, 330 + d)
        if d >= -2:
            veins(z, "h", n, 6, 340 + d)
        if d <= -2:
            veins(z, "Y", max(1, n // 3), 4, 350 + d)
        r = Rnd(360 - d)
        for _ in range(n):
            x, y = r.range(4, W - 16), r.range(4, H - 12)
            cave(z, x, y, x + r.range(6, 12), y + r.range(5, 9), 0.5, r.next() & 0xFFFF, floor="k", rock="K", steps=3)
        levels[d] = (z, [])
    by = {m.id: m for m in MAPS}
    parent = {}
    for name, a, b in PORTALS:
        if by[b[0]].underground and not by[a[0]].underground or (by[a[0]].underground and by[b[0]].underground and by[b[0]].depth < by[a[0]].depth):
            parent[b[0]] = (a, b[1], b[2])
    door_of = {b[0]: a for name, a, b in PORTALS if by[b[0]].indoor and not by[a[0]].indoor and not by[a[0]].underground}

    def glob(mid, x, y):
        lid, ox, oy = place.get(mid, (mid, 0, 0))
        return lid, x + ox, y + oy

    stairs = []
    for m in sorted(unders, key=lambda m: -m.depth):
        z, placed = levels[m.depth]
        (pm, px, py), ex, ey = parent[m.id]
        if by[pm].indoor:  # entered from an interior: lie under the building's door
            pm, px, py = door_of[pm]
        _, gx, gy = glob(pm, px, py)
        ox, oy = free_spot(placed, m.w, m.h, (gx - ex, gy - ey), W, H)
        placed.append((ox, oy, m.w, m.h))
        for y in range(m.h):
            for x in range(m.w):
                z.g[oy + y][ox + x] = m.g[y][x]
        place[m.id] = (z.id, ox, oy)
        REGIONS.append((m.id, m.name, z.id, (ox, oy, m.w, m.h), m.tags + ["quartiere"]))
        if not by[parent[m.id][0][0]].indoor:
            # Stairs straight down from the parent cell, then a tunnel to the old entrance if it moved.
            tunnel(z, (gx, gy), (ox + ex, oy + ey))
            stairs.append((m.id, gx, gy))
    MAPS = [surf] + interiors + [levels[d][0] for d in sorted(levels, reverse=True)]
    new_portals = []
    for name, a, b in PORTALS:
        ga, gb = glob(*a), glob(*b)
        if b[0] in dict((s[0], 0) for s in stairs) and not by[a[0]].indoor:
            gb = (gb[0], ga[1], ga[2])
            # Stair tiles at both ends.
            src = next(mm for mm in MAPS if mm.id == ga[0])
            dst = next(mm for mm in MAPS if mm.id == gb[0])
            src.g[ga[2]][ga[1]] = ">"
            dst.g[gb[2]][gb[1]] = "<"
        new_portals.append((name, ga, gb))
    PORTALS = new_portals
    EDGES = []
    SUBZONES = [(zid, n, place[l][0], (r[0] + place[l][1], r[1] + place[l][2], r[2], r[3]), t) if l in place else (zid, n, l, r, t)
                for zid, n, l, r, t in SUBZONES]
    SUBZONES = REGIONS + SUBZONES
    BUILDINGS = [(b, mid, x + place[mid][1], y + place[mid][2], o, n) if mid in place else (b, mid, x, y, o, n)
                 for b, mid, x, y, o, n in BUILDINGS]
    place_venues(surf, place, pos, minx, miny, cols, rows)
    return place


# Leisure venues: (building, quarter or None for open countryside, owner, name, footprint w, h).
VENUES = [
    ("gelateria", "piazza_garibaldi", "anarchici_commercio", "Gelateria di Piazza", 3, 2),
    ("osteria", "borgo_templari", "templari_borgo", "Osteria del Borgo", 3, 3),
    ("bocciofila", "rotonde", "circolo_boomer", "Bocciofila del Circolo", 5, 3),
    ("cinema", "cattedrale", "chiesa", "Cinema Parrocchiale", 5, 4),
    ("sala_slot", "casino", "casino_diablo", "Sala Slot Diablo Junior", 3, 3),
    ("balera", None, "contadini_bassa", "Balera della Bassa", 5, 4),
    ("campetto", "strada_provinciale", None, "Campetto della Provinciale", 5, 3),
    ("gelateria", "salsomaggiore_terme", None, "Gelateria delle Terme", 3, 2),
    ("cinema", "fidenza_village", "cda_fidenza_village", "Multisala del Village", 5, 4),
    ("parco_giochi", "capannone_regali", None, "Parco giochi degli Elfi", 3, 2),
    ("parco_giochi", "impero_vegano", None, "Parco giochi a impatto zero", 3, 2),
    ("osteria", "colline_di_salso", "contadini_bassa", "Osteria delle Colline", 3, 3),
    ("gelateria", None, None, "Chiosco dello Stirone", 3, 2),
    ("osteria", "fumetteria", "anarchici_commercio", "Osteria del Nerd Affamato", 3, 3),
    ("osteria", None, "contadini_bassa", "Trattoria di campagna", 3, 3),
    ("balera", None, "contadini_bassa", "Balera sotto le stelle", 5, 4),
    ("campetto", None, None, "Campetto dell'oratorio", 5, 3),
    ("parco_giochi", None, None, "Parco della Bassa", 3, 2),
    ("bocciofila", None, "circolo_boomer", "Bocciofila di campagna", 5, 3),
]
GRASS = set('.,"')


def place_venues(surf, place, pos, minx, miny, cols, rows):
    taken = set()
    for _, mid, x, y, _, _ in BUILDINGS:
        if place.get(mid, ("",))[0] == "fidenza" or mid == "fidenza":
            for yy in range(y - 6, y + 3):
                for xx in range(x - 4, x + 5):
                    taken.add((xx, yy))
    used_cells = {(p[0] - minx, p[1] - miny) for p in pos.values()}
    empty = [(gx * RW, gy * RH) for gy in range(rows) for gx in range(cols) if (gx, gy) not in used_cells]
    free_rects = iter(empty)
    for bid, region, owner, name, fw, fh in VENUES:
        if region:
            _, ox, oy = place[region]
            zone = region
        else:
            ox, oy = next(free_rects)
            zone = "fidenza"
        cx, cy = ox + RW // 2, oy + RH // 2
        best = None
        for ay in range(oy + fh + 1, oy + RH - 3):
            for ax in range(ox + fw // 2 + 2, ox + RW - fw // 2 - 2):
                cells = [(x, y) for y in range(ay - fh, ay + 2) for x in range(ax - fw // 2 - 1, ax + fw // 2 + 2)]
                if any(c in taken or surf.g[c[1]][c[0]] not in GRASS for c in cells):
                    continue
                d = (ax - cx) ** 2 + (ay - cy) ** 2
                if best is None or d < best[0]:
                    best = (d, ax, ay)
        if best is None:
            print("ATTENZIONE: nessun posto per", name)
            continue
        _, ax, ay = best
        for y in range(ay - fh - 2, ay + 4):
            for x in range(ax - fw // 2 - 3, ax + fw // 2 + 4):
                taken.add((x, y))
        for y in range(ay - fh + 1, ay + 1):
            for x in range(ax - fw // 2, ax + fw // 2 + 1):
                surf.g[y][x] = "."
        surf.g[ay + 1][ax] = ":"
        BUILDINGS.append((bid, zone, ax, ay, owner, name))


def fmt_tags(t):
    return "[" + ", ".join(f'"{x}"' for x in t) + "]"


def ron():
    out = ["// Generato da tools/maps/build_maps.py: non modificare a mano, rigenera.", "(", "    map: (", "        legend: ["]
    for ch, tid, name, walk in LEGEND:
        c = ch.replace("\\", "\\\\").replace("'", "\\'")
        dig = ""
        if ch in DIG:
            to, (item, n) = DIG[ch]
            dig = f", dig_to: '{to}', yields: (\"{item}\", {n})"
        out.append(f"            (ch: '{c}', id: \"{tid}\", name: \"{name}\", walkable: {'true' if walk else 'false'}{dig}),")
    out.append("        ],")
    out.append("        layers: [")
    for m in MAPS:
        flags = ""
        if m.indoor:
            flags += ", indoor: true"
        if m.underground:
            flags += ", underground: true"
        if getattr(m, "depth", 0):
            flags += f", depth: {m.depth}"
        out.append(f'            (id: "{m.id}", name: "{m.name}", tiles_file: "maps/{m.id}.map", tags: {fmt_tags(m.tags)}{flags}),')
    out.append("        ],")
    out.append("        zones: [")
    for zid, name, layer, rect, tags in SUBZONES:
        out.append(f'            (id: "{zid}", name: "{name}", layer: "{layer}", rect: {rect}, tags: {fmt_tags(tags)}),')
    out.append("        ],")
    out.append("        portals: [")
    for name, a, b in PORTALS:
        out.append(f'            (name: "{name}", a: ("{a[0]}", {a[1]}, {a[2]}), b: ("{b[0]}", {b[1]}, {b[2]})),')
    out.append("        ],")
    out.append("        edges: [")
    for a, side, bb in EDGES:
        out.append(f'            (a: "{a}", side: {side}, b: "{bb}"),')
    out.append("        ],")
    out.append("        networks: [")
    out.append('            (id: "acquedotto", name: "Acquedotto di Fidenza", zones: ["piazza_garibaldi", "cattedrale", "redazione", "stazione_polizia", "bar_ubriaconi", "fidenza_village", "borgo_templari"]),')
    out.append('            (id: "fogne_termali", name: "Fogne termali", zones: ["salsomaggiore_terme", "terme_interno", "cripta"]),')
    out.append("        ],")
    out.append("    ),")
    out.append("    placements: [")
    for bid, mid, x, y, owner, name in BUILDINGS:
        extra = f', owner_faction: "{owner}"' if owner else ""
        extra += f', name: "{name}"' if name else ""
        out.append(f'        Building(building: "{bid}", zone: "{mid}", at: ({x}, {y}){extra}),')
    out.append("    ],")
    out.append(")")
    return "\n".join(out) + "\n"


def check():
    """Every door and building anchor must be walkable and reachable from a walkable neighbour."""
    by = {m.id: m for m in MAPS}
    problems = []
    for name, a, b in PORTALS:
        for mid, x, y in (a, b):
            m = by[mid]
            if not WALK.get(m.g[y][x], False):
                problems.append(f"porta '{name}': ({mid},{x},{y}) su '{m.g[y][x]}'")
    for bid, mid, x, y, _, _ in BUILDINGS:
        m = by[PLACE.get(mid, (mid,))[0]]
        if not WALK.get(m.g[y][x], False):
            problems.append(f"edificio {bid} in {mid} ({x},{y}) su '{m.g[y][x]}'")
    return problems


def main():
    place = merge()
    PLACE.update(place)
    print("origine delle regioni:", {k: v for k, v in place.items()})
    (DATA / "maps").mkdir(parents=True, exist_ok=True)
    for old in (DATA / "maps").glob("*.map"):
        old.unlink()
    for m in MAPS:
        (DATA / "maps" / f"{m.id}.map").write_text(m.text())
    (DATA / "70_mappa.ron").write_text(ron())
    probs = check()
    print(f"{len(MAPS)} mappe, {len(PORTALS)} porte, {len(EDGES)} bordi, {len(BUILDINGS)} edifici")
    for p in probs:
        print("ATTENZIONE:", p)


if __name__ == "__main__":
    main()
