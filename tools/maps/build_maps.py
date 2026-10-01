#!/usr/bin/env python3
"""Builds the Fidenza & Salsomaggiore region: tile maps (data/maps/*.map) and data/70_mappa.ron
(legend, maps, sub-zones, borders between maps, doors, infrastructure networks, building positions).

Everything comes from this one script so that tiles, doors and buildings always agree.

    python3 tools/maps/build_maps.py
"""
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
    m.scatter(",", 11, seed=hash(mid) & 0xFFFF)
    m.scatter('"', 17, seed=(hash(mid) >> 3) & 0xFFFF)
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
        m = by[mid]
        if not WALK.get(m.g[y][x], False):
            problems.append(f"edificio {bid} in {mid} ({x},{y}) su '{m.g[y][x]}'")
    return problems


def main():
    (DATA / "maps").mkdir(parents=True, exist_ok=True)
    for m in MAPS:
        (DATA / "maps" / f"{m.id}.map").write_text(m.text())
    (DATA / "70_mappa.ron").write_text(ron())
    probs = check()
    print(f"{len(MAPS)} mappe, {len(PORTALS)} porte, {len(EDGES)} bordi, {len(BUILDINGS)} edifici")
    for p in probs:
        print("ATTENZIONE:", p)


if __name__ == "__main__":
    main()
