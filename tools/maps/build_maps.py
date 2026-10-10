#!/usr/bin/env python3
"""Builds the world: four big surface maps (Fidenza, Salsomaggiore, Fidenza Village and the fields, the Bassa)
joined at their borders, the dwarves' underground level under Fidenza and the dungeons as separate maps reached
by stairs. Writes the tile maps (data/maps/*.map) and data/70_mappa.ron (legend, maps, zones, borders, doors,
infrastructure networks, building positions).

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
RW, RH, GRID = 40, 28, 6  # every surface map is a 6×6 grid of 40×28 regions (240×168 cells)

# Surface maps: id → (name, tags, filler for the cells without a named place).
WORLDS = {
    "fidenza": ("Fidenza", ["superficie", "fidenza"]),
    "salsomaggiore": ("Salsomaggiore Terme", ["superficie", "salso"]),
    "village": ("Fidenza Village e i campi", ["superficie", "campi"]),
    "bassa": ("La Bassa", ["superficie", "campi", "selvatico"]),
}


def filler_kind(world, gx, gy):
    if world == "fidenza":
        return "citta" if 1 <= gx <= 4 and gy <= 3 else "campi"
    if world == "salsomaggiore":
        return "citta" if 1 <= gx <= 3 and gy <= 1 else "colline"
    if world == "village":
        return "campi"
    return "bassa"


def building(b, mid, x, y, owner=None, name=None):
    BUILDINGS.append((b, mid, x, y, owner, name))


def door(name, a, ax, ay, b, bx, by):
    PORTALS.append((name, (a, ax, ay), (b, bx, by)))


def outdoor(mid, name, openings, tags=(), road="=", trees=True, world="fidenza", at=(0, 0)):
    m = Map(mid, name, RW, RH, ".", tags)
    m.road, m.world, m.at = road, world, at
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


def dungeon(mid, name, w, h, tags, fill="K"):
    """A dungeon: its own map, reached through stairs from the surface."""
    m = Map(mid, name, w, h, fill, tags, underground=True)
    m.depth = -1
    m.dungeon = True
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


def room(m, x0, y0, x1, y1, floor, doors=(), wall="W"):
    """A building drawn on the map: walls all round, the floor inside, door mats in the walls."""
    m.rect(x0, y0, x1, y1, wall)
    m.rect(x0 + 1, y0 + 1, x1 - 1, y1 - 1, floor)
    for (x, y) in doors:
        m.g[y][x] = "d"


def hall(m, x0, y0, x1, y1, floor="x"):
    """A room carved in the rock of a dungeon."""
    m.rect(x0, y0, x1, y1, floor)


def corridor(m, a, b, ch="x", width=2):
    tunnel(m, a, b, ch, width)


# ══ FIDENZA ═════════════════════════════════════════════════════════════════
p = outdoor("piazza_garibaldi", "Piazza Garibaldi", "NSEW", ["pubblico", "pattuglia", "caldo", "fidenza"], at=(2, 2))
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
building("forno", "piazza_garibaldi", 24, 21, "anarchici_commercio", "Forno di Piazza")
building("galleria_borgazzi", "piazza_garibaldi", 34, 5, None)
room(p, 28, 15, 37, 25, "x", doors=[(32, 15)])          # Comando di Polizia
p.rect(29, 18, 31, 18, "c")
p.rect(32, 19, 36, 19, "|")
p.rect(32, 19, 32, 24, "|")
p.g[19][34] = "x"
building("stazione_polizia", "piazza_garibaldi", 32, 16, "polizia_neutra")
SUBZONES += [("stazione_polizia", "Comando di Polizia", "piazza_garibaldi", (29, 16, 8, 9), ["fidenza", "pattuglia"]),
             ("celle", "Celle di Detenzione", "piazza_garibaldi", (33, 20, 4, 5), ["celle"]),
             ("centro_piazza", "Centro di Piazza Garibaldi", "piazza_garibaldi", (12, 8, 16, 12), ["fidenza"]),
             ("redazione", "Redazione della Gazzetta", "piazza_garibaldi", (2, 18, 9, 9), ["fidenza"])]
p.g[20][12] = "_"
door("Tombino di Piazza Garibaldi", "piazza_garibaldi", 11, 20, "gallerie", 3, 3)

st = outdoor("stazione", "Stazione di Fidenza", "S", ["pubblico", "fidenza", "stazione"], at=(2, 0))
st.rect(0, 5, 39, 6, "x")                        # the Milano–Bologna line
st.rect(0, 4, 39, 4, "f")
st.rect(0, 7, 39, 7, "f")
st.rect(5, 8, 34, 9, "_")                        # platform
st.rect(19, 0, 20, 7, "=")                       # level crossing towards the Bassa
room(st, 12, 10, 27, 16, "m", doors=[(19, 10), (19, 16)])
st.rect(13, 12, 16, 12, "c")
st.rect(19, 17, 20, 27, "=")
st.rect(3, 19, 12, 24, "g")
building("banco_mercato", "stazione", 24, 15, "anarchici_commercio", "Edicola della Stazione")
SUBZONES.append(("stazione_fs", "Stazione di Fidenza", "stazione", (13, 11, 14, 5), ["fidenza", "stazione"]))

s = outdoor("cattedrale", "Piazza del Duomo", "NSE", ["pubblico", "chiesa", "fidenza"], at=(2, 1))
room(s, 4, 3, 16, 21, "m", doors=[(10, 21), (16, 12)])   # Duomo di San Donnino
s.rect(9, 6, 11, 20, "r")
for y in range(8, 20, 2):
    s.rect(6, y, 8, y, "p")
    s.rect(12, y, 14, y, "p")
s.rect(6, 4, 14, 5, "x")
s.rect(17, 12, 18, 12, "_")
s.rect(3, 22, 20, 24, "_")
building("cattedrale", "cattedrale", 10, 20, "chiesa")
SUBZONES.append(("duomo_interno", "Interno del Duomo", "cattedrale", (5, 4, 11, 17), ["chiesa", "sacro"]))
door("Scala della Cripta di San Donnino", "cattedrale", 15, 4, "cripta_duomo", 3, 3)
building("banchetto_santini", "cattedrale", 14, 24, "chiesa")
building("mensa_poveri", "cattedrale", 27, 10, "chiesa")
clear_below(s, 27, 10)
link(s, 27, 10)
building("laboratorio_medico", "cattedrale", 33, 10, "chiesa")
clear_below(s, 33, 10)
link(s, 33, 10)

b = outdoor("borgo_templari", "Borgo dei Templari", "NEW", ["pubblico", "templari", "fidenza", "caldo"], at=(3, 2))
b.rect(4, 17, 35, 18, "=")
room(b, 3, 3, 15, 11, "w", doors=[(9, 11)])              # Bar degli Ubriaconi
b.rect(4, 8, 9, 8, "c")
b.rect(11, 4, 14, 6, "r")
b.g[12][9] = "="
building("bar_ubriaconi", "borgo_templari", 9, 10, "ubriaconi")
building("trono_ubriaconi", "borgo_templari", 12, 6, "ubriaconi")
SUBZONES += [("bar_ubriaconi", "Bar degli Ubriaconi", "borgo_templari", (4, 4, 11, 7), ["pubblico", "caldo", "fidenza"]),
             ("trono_ubriaconi", "Trono degli Ubriaconi", "borgo_templari", (11, 4, 4, 3), ["trono"])]
building("fucina", "borgo_templari", 28, 16, "templari_borgo")
building("forno", "borgo_templari", 27, 10, "templari_borgo", "Forno del Borgo")
clear_below(b, 27, 10)
link(b, 27, 10)
b.rect(31, 5, 36, 9, "x")

n = outdoor("fumetteria", "Quartiere Nerd", "SEW", ["pubblico", "nerd", "fidenza"], at=(3, 1))
room(n, 14, 3, 26, 11, "w", doors=[(20, 11)])            # La Fumetteria
n.rect(16, 5, 18, 5, "c")
n.rect(22, 5, 24, 6, "c")
n.rect(16, 8, 17, 8, "c")
n.g[12][20] = "="
building("fumetteria", "fumetteria", 20, 10, "gilda_nerd")
SUBZONES.append(("negozio_fumetti", "Fumetteria", "fumetteria", (15, 4, 11, 7), ["nerd", "negozio"]))
door("Botola dello Scantinato", "fumetteria", 16, 9, "scantinato_nerd", 3, 13)
n.rect(24, 18, 34, 22, "_")
building("banco_mercato", "fumetteria", 28, 20, "gilda_nerd", "Bancarella della Gilda")

v = outdoor("impero_vegano", "Impero Vegano del Monolite", "S", ["vegano"], road=":", at=(1, 1))
v.rect(5, 16, 14, 23, "o")
building("monolite_soia", "impero_vegano", 20, 10, "impero_vegano")
v.rect(18, 3, 22, 11, ":")
building("cattedrale_idroponica", "impero_vegano", 9, 11, "impero_vegano")
clear_below(v, 9, 11, ":")
link(v, 9, 11, ":")
v.rect(9, 13, 30, 14, ":")
building("bar_estratti", "impero_vegano", 30, 11, "impero_vegano")
clear_below(v, 30, 11, ":")
link(v, 30, 11, ":")
building("laboratorio_fake_meat", "impero_vegano", 30, 23, "impero_vegano")
clear_below(v, 30, 23, ":")
v.rect(30, 15, 30, 23, ":")
building("campo_soia", "impero_vegano", 9, 22, "impero_vegano")

r = outdoor("rotonde", "Le Rotonde", "NSEW", ["rotonda", "pattuglia"], road="g", at=(1, 2))
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
building("banco_mercato", "rotonde", 9, 12, "circolo_boomer", "Chiosco del Circolo")

ov = outdoor("ospedale_vaio", "Ospedale di Vaio", "EW", ["pubblico", "fidenza", "ospedale"], road="g", at=(4, 2))
room(ov, 8, 3, 31, 11, "m", doors=[(20, 11)])
for x in range(10, 30, 4):
    ov.rect(x, 5, x + 1, 5, "c")
ov.rect(10, 9, 14, 9, "c")
ov.g[12][20] = "g"
building("laboratorio_medico", "ospedale_vaio", 24, 10, None, "Pronto Soccorso di Vaio")
SUBZONES.append(("ospedale", "Ospedale di Vaio", "ospedale_vaio", (9, 4, 22, 7), ["ospedale", "medicina"]))
ov.rect(6, 17, 33, 23, "g")

sp = outdoor("strada_provinciale", "Strada Provinciale", "NSEW", ["strada"], road="g", at=(2, 3))
sp.rect(1, 20, 38, 21, "~")
sp.rect(19, 20, 20, 21, "b")
sp.rect(3, 22, 36, 22, "s")
sp.rect(3, 19, 36, 19, "s")
sp.g[19][19] = sp.g[19][20] = "g"
sp.g[22][19] = sp.g[22][20] = "g"

# Teatro Magnani: the stalls, the stage and the boxes.
tm = outdoor("teatro_magnani", "Teatro Magnani", "NW", ["pubblico", "fidenza", "teatro"], at=(3, 3))
room(tm, 6, 3, 33, 23, "w", doors=[(19, 3)])
tm.rect(7, 4, 32, 4, "_")
tm.g[2][19] = tm.g[1][19] = "="
for y in range(6, 15, 2):
    tm.rect(10, y, 17, y, "p")
    tm.rect(22, y, 29, y, "p")
tm.rect(8, 17, 31, 22, "r")                     # stage
tm.rect(7, 16, 32, 16, "c")
tm.g[16][19] = tm.g[16][20] = "r"
SUBZONES.append(("teatro", "Teatro Magnani", "teatro_magnani", (7, 4, 26, 19), ["teatro", "svago"]))
building("cinema", "teatro_magnani", 20, 21, "anarchici_commercio", "Palco del Teatro Magnani")

# The cemetery: graves in rows, the chapel, and the mausoleum with stairs to San Donnino's crypt.
ci = outdoor("cimitero", "Cimitero di Fidenza", "NE", ["cimitero", "fidenza"], at=(1, 3))
room(ci, 3, 3, 36, 24, ".", doors=[(19, 3), (36, 13)], wall="#")
for y in range(6, 23, 3):
    for x in range(6, 34, 3):
        if x not in (18, 21) and not (24 <= x <= 33 and y >= 15):
            ci.g[y][x] = "K"
ci.rect(19, 4, 20, 23, ":")
ci.rect(4, 13, 35, 14, ":")
room(ci, 26, 16, 33, 22, "x", doors=[(29, 16)])
ci.g[15][29] = ":"
SUBZONES.append(("cimitero", "Cimitero di Fidenza", "cimitero", (4, 4, 32, 20), ["cimitero"]))
door("Mausoleo dei Vescovi", "cimitero", 30, 20, "cripta_duomo", 55, 35)

# ══ FIDENZA VILLAGE E CAMPI ═════════════════════════════════════════════════
fv = outdoor("fidenza_village", "Fidenza Village", "SW", ["pubblico", "shopping"], road="=", world="village", at=(1, 1))
room(fv, 6, 2, 31, 11, "m", doors=[(20, 11)])             # outlet
for x in range(8, 26, 5):
    fv.rect(x, 4, x + 2, 7, "c")
fv.g[12][20] = "="
building("outlet", "fidenza_village", 20, 10, "cda_fidenza_village")
building("casseforti_cda", "fidenza_village", 28, 5, "cda_fidenza_village")
SUBZONES.append(("outlet_interno", "Negozi dell'Outlet", "fidenza_village", (7, 3, 24, 8), ["shopping"]))
door("Ascensore riservato del CdA", "fidenza_village", 29, 9, "covo_rettiliano", 3, 3)
fv.rect(6, 16, 32, 24, "g")
for x in range(7, 32, 3):
    if x not in (19, 20):
        fv.rect(x, 18, x, 22, "_")
fv.rect(35, 0, 36, 27, "g")                      # Autostrada A1
fv.rect(34, 0, 34, 27, "f")
fv.rect(37, 0, 37, 27, "f")

cy = outdoor("capannone_regali", "Capannone di Babbo Natale Estivo", "NEW", ["logistica"], road="g", world="village", at=(1, 2))
cy.rect(6, 4, 34, 22, "g")
building("capannone_regali", "capannone_regali", 20, 12, "babbo_natale")
building("banco_mercato", "capannone_regali", 10, 18, "babbo_natale", "Mensa aziendale degli Elfi")

c = outdoor("campagna_bassa", "Campi e cascine", "NSEW", ["campi"], road=":", world="village", at=(2, 2))
c.rect(2, 8, 37, 9, ":")
c.rect(2, 19, 37, 20, ":")
for bid, x in [("mulino", 6), ("forno", 12), ("cantina", 25), ("birrificio", 31), ("distilleria", 36)]:
    building(bid, "campagna_bassa", x, 7, "contadini_bassa")
for bid, x in [("macello", 6), ("salumificio", 12), ("porcilaia", 26), ("orto", 32)]:
    building(bid, "campagna_bassa", x, 18, "contadini_bassa")
for bid, x in [("vigna", 4), ("luppoleto", 10), ("campo_orzo", 16), ("campo_grano", 24), ("campo_grano", 28), ("orto", 34)]:
    c.rect(x - 1, 22, x + 1, 24, "o")
    building(bid, "campagna_bassa", x, 25, "contadini_bassa")
c.rect(2, 25, 37, 25, ":")
c.rect(19, 13, 20, 27, ":")
c.rect(14, 16, 16, 18, "x")                      # the salumificio's cellar door
door("Botola delle Cantine", "campagna_bassa", 15, 17, "cantine_culatello", 3, 3)

cs = outdoor("caseificio", "Caseificio del Parmigiano", "S", ["campi", "produzione"], road=":", world="village", at=(3, 1))
room(cs, 8, 4, 31, 12, "x", doors=[(19, 12)])
for y in (6, 8, 10):
    cs.rect(10, y, 17, y, "c")
    cs.rect(22, y, 29, y, "c")
cs.g[13][19] = ":"
building("banco_mercato", "caseificio", 20, 11, "contadini_bassa", "Spaccio del Caseificio")
SUBZONES.append(("magazzino_forme", "Magazzino delle forme", "caseificio", (9, 5, 22, 7), ["campi", "produzione"]))
cs.rect(6, 17, 14, 23, "o")
cs.rect(25, 17, 33, 23, "o")

# ══ SALSOMAGGIORE ═══════════════════════════════════════════════════════════
bo = outdoor("bosco_stirone", "Bosco dello Stirone", "NSE", ["bosco", "selvatico"], road=":", world="salsomaggiore", at=(2, 0))
bo.scatter("T", 4, seed=21)
bo.scatter("t", 9, seed=22)
bo.rect(15, 1, 18, 26, "~")
for y in range(1, 27):
    bo.g[y][14] = "j" if y % 3 else "s"
    bo.g[y][19] = "j" if y % 4 == 1 else "s"
bo.rect(14, 13, 19, 14, "b")
bo.rect(20, 13, 39, 14, ":")
bo.rect(2, 13, 13, 14, ":")
bo.rect(19, 0, 20, 27, ":")
for (x, y) in [(6, 5), (8, 20), (28, 6), (32, 21)]:
    bo.rect(x - 2, y - 2, x + 2, y + 2, ",")

t = outdoor("salsomaggiore_terme", "Salsomaggiore e le Terme", "NESW", ["pubblico", "terme", "salso", "caldo"], world="salsomaggiore", at=(2, 1))
room(t, 22, 2, 37, 11, "m", doors=[(29, 11)])             # Terme Berzieri
t.rect(25, 5, 34, 8, "P")
t.g[12][29] = "="
building("stabilimento_termale", "salsomaggiore_terme", 29, 10, "cripta_san_vitale")
SUBZONES.append(("terme_interno", "Terme Berzieri", "salsomaggiore_terme", (23, 3, 14, 8), ["terme", "salso"]))
door("Scale delle viscere", "salsomaggiore_terme", 24, 3, "cripta", 3, 3)
t.rect(4, 4, 16, 10, "_")
t.rect(6, 17, 34, 23, "_")
building("banco_mercato", "salsomaggiore_terme", 12, 20, "anarchici_commercio")
building("forno", "salsomaggiore_terme", 28, 21, "contadini_bassa")
t.rect(27, 22, 29, 22, "_")
building("orto", "salsomaggiore_terme", 6, 25, "contadini_bassa")

k = outdoor("casino", "Casinò Diablo Tentator", "NEW", ["pubblico", "vizio", "caldo"], world="salsomaggiore", at=(3, 1))
k.rect(6, 3, 17, 11, "_")
k.rect(22, 3, 33, 11, "_")
room(k, 11, 16, 31, 25, "D", doors=[(20, 16)])
for (x, y) in [(13, 19), (17, 19), (23, 19), (13, 22), (17, 22)]:
    k.rect(x, y, x + 2, y, "c")
k.g[15][20] = "="
building("sala_giochi", "casino", 20, 17, "casino_diablo")
building("spacciatore_casino", "casino", 28, 24, "casino_diablo")
SUBZONES.append(("sala_casino", "Sala da gioco", "casino", (12, 17, 19, 8), ["vizio", "caldo"]))

gh = outdoor("palazzo_congressi", "Palazzo dei Congressi", "E", ["pubblico", "salso", "turismo"], world="salsomaggiore", at=(1, 1))
room(gh, 4, 3, 33, 22, "m", doors=[(33, 13)])
gh.rect(8, 6, 29, 19, "r")                       # the Moorish ballroom
for (x, y) in [(10, 8), (27, 8), (10, 17), (27, 17)]:
    gh.g[y][x] = "F"
gh.rect(14, 11, 23, 14, "_")
SUBZONES.append(("salone_moresco", "Salone Moresco", "palazzo_congressi", (5, 4, 28, 18), ["salso", "turismo", "svago"]))
building("balera", "palazzo_congressi", 19, 14, None, "Gran Ballo del Palazzo dei Congressi")

tb = outdoor("tabiano_terme", "Tabiano Terme", "W", ["pubblico", "terme", "salso"], world="salsomaggiore", at=(4, 1))
room(tb, 6, 3, 18, 11, "m", doors=[(12, 11)])
tb.rect(8, 5, 16, 8, "P")
tb.g[12][12] = "="
building("banco_mercato", "tabiano_terme", 15, 10, None, "Bottega delle Terme di Tabiano")
SUBZONES.append(("terme_tabiano", "Terme di Tabiano", "tabiano_terme", (7, 4, 11, 7), ["terme", "salso"]))
tb.rect(25, 2, 36, 11, "R")
room(tb, 27, 3, 34, 9, "x", doors=[(30, 9)], wall="#")
tb.rect(30, 10, 30, 13, ":")
SUBZONES.append(("castello_tabiano", "Castello di Tabiano", "tabiano_terme", (28, 4, 6, 5), ["castello", "salso"]))

co = outdoor("colline_di_salso", "Colline di Salso", "NE", ["colline", "selvatico"], road=":", world="salsomaggiore", at=(2, 2))
rr = Rnd(31)
for _ in range(14):
    cx_, cy_ = rr.range(4, 35), rr.range(4, 23)
    for y in range(cy_ - 2, cy_ + 3):
        for x in range(cx_ - 3, cx_ + 4):
            if 1 < x < 38 and 1 < y < 26 and abs(x - cx_) + abs(y - cy_) < 5 and co.g[y][x] not in ":=":
                co.g[y][x] = "R"
co.rect(27, 18, 33, 22, "R")
co.rect(29, 21, 31, 22, "k")
co.rect(19, 14, 30, 15, ":")
co.rect(30, 15, 30, 22, ":")
door("Bocca della Miniera di Sale", "colline_di_salso", 30, 22, "miniera_di_sale", 3, 3)

sc = outdoor("scipione", "Castello di Scipione", "E", ["castello", "salso", "colline"], road=":", world="salsomaggiore", at=(1, 2))
sc.rect(6, 3, 30, 22, "R")
room(sc, 9, 5, 27, 19, "x", doors=[(27, 13)], wall="#")
room(sc, 11, 7, 16, 12, "w", doors=[(16, 10)], wall="#")  # the keep
sc.rect(28, 13, 38, 14, ":")
SUBZONES.append(("rocca_scipione", "Rocca di Scipione", "scipione", (10, 6, 17, 13), ["castello", "salso"]))

# Leisure venues: (building, quarter or None, owner, name, footprint w, h, map for None).
VENUES = [
    ("gelateria", "piazza_garibaldi", "anarchici_commercio", "Gelateria di Piazza", 3, 2, None),
    ("osteria", "borgo_templari", "templari_borgo", "Osteria del Borgo", 3, 3, None),
    ("bocciofila", "rotonde", "circolo_boomer", "Bocciofila del Circolo", 5, 3, None),
    ("cinema", "cattedrale", "chiesa", "Cinema Parrocchiale", 5, 4, None),
    ("sala_slot", "casino", "casino_diablo", "Sala Slot Diablo Junior", 3, 3, None),
    ("balera", None, "contadini_bassa", "Balera della Bassa", 5, 4, "village"),
    ("campetto", "strada_provinciale", None, "Campetto della Provinciale", 5, 3, None),
    ("gelateria", "salsomaggiore_terme", None, "Gelateria delle Terme", 3, 2, None),
    ("cinema", None, "cda_fidenza_village", "Multisala del Village", 5, 4, "village"),
    ("parco_giochi", "capannone_regali", None, "Parco giochi degli Elfi", 3, 2, None),
    ("parco_giochi", "impero_vegano", None, "Parco giochi a impatto zero", 3, 2, None),
    ("osteria", "colline_di_salso", "contadini_bassa", "Osteria delle Colline", 3, 3, None),
    ("gelateria", "bosco_stirone", None, "Chiosco dello Stirone", 3, 2, None),
    ("osteria", "fumetteria", "anarchici_commercio", "Osteria del Nerd Affamato", 3, 3, None),
    ("osteria", None, "contadini_bassa", "Trattoria di campagna", 3, 3, "village"),
    ("balera", None, "contadini_bassa", "Balera sotto le stelle", 5, 4, "village"),
    ("campetto", "ospedale_vaio", None, "Campetto dell'oratorio", 5, 3, None),
    ("parco_giochi", None, None, "Parco della Bassa", 3, 2, "village"),
    ("bocciofila", None, "circolo_boomer", "Bocciofila di campagna", 5, 3, "village"),
    ("osteria", "scipione", "contadini_bassa", "Locanda del Castello", 3, 3, None),
]

# ══ IL REGNO DEI NANI (one underground level under Fidenza) ═════════════════
def under(mid, name, w, h, at, tags):
    m = Map(mid, name, w, h, "K", tags, underground=True)
    m.depth, m.at = -1, at
    MAPS.append(m)
    return m


gl = under("gallerie", "Fortezza dei Nani", 64, 40, (88, 73), ["gallerie", "sotterraneo", "fortezza"])
veins(gl, "O", 14, 18, 101)
veins(gl, "S", 8, 14, 102)
veins(gl, "A", 2, 5, 103)
gl.rect(2, 2, 8, 8, "x")
gl.rect(8, 5, 40, 6, "x")
gl.rect(20, 8, 34, 18, "x")
gl.rect(26, 7, 27, 7, "x")
gl.rect(40, 3, 52, 11, "x")
gl.rect(36, 12, 38, 22, "x")
cave(gl, 4, 22, 20, 36, 0.55, 104)
tunnel(gl, (10, 6), (10, 24), "x")
cave(gl, 42, 18, 61, 35, 0.48, 105)
veins(gl, "O", 10, 10, 106, on="K")
tunnel(gl, (37, 22), (45, 26), "k")
gl.rect(58, 33, 60, 36, "k")
tunnel(gl, (50, 30), (59, 34), "k")
tunnel(gl, (38, 22), (50, 30), "k")
tunnel(gl, (33, 15), (37, 15), "x")             # great hall → east corridor
tunnel(gl, (37, 11), (41, 11), "x")             # east corridor → forge             # keeps the halls joined whatever the caves do
building("deposito", "gallerie", 27, 17, "nani_miniere")
building("fungaia_porcini", "gallerie", 23, 11, "nani_miniere", "Fungaia della Fortezza")
building("banco_mercato", "gallerie", 31, 11, "nani_miniere", "Dispensa dei Nani")
building("fucina", "gallerie", 46, 10, "nani_miniere")
SUBZONES += [("miniera", "Filone di ferro", "gallerie", (42, 18, 20, 18), ["miniera", "gallerie"]),
             ("tana_dinosauri", "Tana dei Dinosauri", "gallerie", (4, 22, 17, 15), ["gallerie", "tana"])]

mp = under("miniere_profonde", "Miniere Profonde", 72, 44, (160, 62), ["sotterraneo", "profondo", "miniera"])
cave(mp, 2, 2, 69, 41, 0.42, 201)
veins(mp, "S", 20, 22, 202)
veins(mp, "O", 14, 16, 203)
veins(mp, "G", 8, 6, 204)
veins(mp, "h", 10, 6, 205)
veins(mp, "Y", 3, 3, 206)
veins(mp, "A", 4, 5, 207)
for y in range(4, 40):
    mp.g[y][34 + (y // 5) % 3] = "~"
mp.rect(2, 2, 6, 6, "k")
tunnel(mp, (5, 5), (66, 39), "k")
mp.rect(32, 21, 38, 21, "b")

cv = under("caverne", "Caverne dei Porcini", 80, 48, (152, 116), ["sotterraneo", "profondo", "caverna"])
cave(cv, 2, 2, 77, 45, 0.58, 301)
for i, (x, y) in enumerate([(20, 12), (52, 30), (64, 10), (14, 34)]):
    for yy in range(y - 4, y + 5):
        for xx in range(x - 6, x + 7):
            if cv.g[yy][xx] == "k":
                cv.g[yy][xx] = "u" if (xx * 7 + yy * 13 + i) % 5 == 0 else "q"
cv.rect(36, 18, 46, 26, "~")
veins(cv, "G", 6, 5, 302)
cv.rect(2, 2, 6, 6, "k")
tunnel(cv, (5, 5), (74, 43), "q")
cv.rect(36, 22, 46, 22, "b")
for (x, y) in [(20, 17), (52, 35), (64, 15)]:
    cv.rect(x - 1, y - 1, x + 1, y + 1, "q")
    building("fungaia_porcini", "caverne", x, y, None)

ct = under("cuore_termale", "Cuore Termale", 60, 40, (8, 120), ["sotterraneo", "profondo", "termale"])
ct.rect(2, 2, 57, 37, "M")
cave(ct, 2, 2, 57, 37, 0.35, 401, floor="n", rock="M")
ct.rect(2, 2, 7, 7, "n")
tunnel(ct, (5, 5), (30, 20), "n")
ct.rect(27, 17, 33, 23, "n")
building("scrigno_antico", "cuore_termale", 30, 20, None)

# Tunnels between the dwarves' halls (global coordinates on the level).
DWARF_TUNNELS = [((147, 107), (164, 66)), ((226, 101), (156, 120)), ((226, 159), (12, 124))]

# ══ DUNGEONS (separate maps) ════════════════════════════════════════════════
# Cripta di San Donnino, under the Duomo: the bishops' tombs, the ossuary maze, the relic chapel.
cd = dungeon("cripta_duomo", "Cripta di San Donnino", 60, 40, ["cripta", "sotterraneo", "dungeon", "sacro"])
hall(cd, 1, 1, 9, 8, "m")
corridor(cd, (8, 4), (20, 4), "x")
hall(cd, 18, 2, 32, 10, "m")                    # bishops' tombs
for x in range(20, 31, 3):
    cd.rect(x, 4, x + 1, 4, "p")
    cd.rect(x, 8, x + 1, 8, "p")
maze(cd, 34, 1, 7, 6, 811)                      # ossuary
cd.rect(34, 2, 35, 3, "x")
corridor(cd, (31, 6), (35, 2), "x")
corridor(cd, (25, 10), (25, 22), "x")
hall(cd, 14, 20, 36, 30, "m")                   # relic chapel
cd.rect(22, 21, 28, 22, "r")
cd.rect(24, 23, 26, 29, "r")
building("reliquiario_donnino", "cripta_duomo", 25, 22, "chiesa")
corridor(cd, (36, 26), (55, 26), "x")
corridor(cd, (54, 20), (54, 36), "x")
hall(cd, 50, 32, 58, 38, "x")                   # under the cemetery
corridor(cd, (5, 8), (5, 34), "x")
hall(cd, 2, 30, 12, 38, "k")                    # flooded well
cd.rect(4, 33, 9, 36, "~")
corridor(cd, (12, 34), (14, 28), "x")
SUBZONES += [("ossario", "Ossario", "cripta_duomo", (34, 1, 22, 19), ["cripta", "dungeon"]),
             ("cappella_reliquie", "Cappella delle Reliquie", "cripta_duomo", (14, 20, 23, 11), ["cripta", "sacro"])]

# Cripta e catacombe di San Vitale, under the Terme.
cr = dungeon("cripta", "Cripta di San Vitale", 48, 32, ["cripta", "sotterraneo", "dungeon"])
maze(cr, 1, 1, 15, 10, 501)
cr.rect(18, 12, 29, 20, "x")
cr.rect(2, 2, 5, 5, "x")
cr.rect(40, 26, 45, 29, "x")
tunnel(cr, (29, 16), (43, 27), "x")
building("altare_cripta", "cripta", 24, 17, "cripta_san_vitale")

ca = dungeon("catacombe", "Catacombe di San Vitale", 60, 40, ["cripta", "sotterraneo", "profondo", "dungeon"])
maze(ca, 1, 1, 19, 12, 601)
veins(ca, "h", 8, 4, 602)
ca.rect(2, 2, 5, 5, "x")
door("Scala delle Catacombe", "cripta", 44, 29, "catacombe", 3, 3)

# Scantinato della Fumetteria: the nerds' dungeon (the octopus's room is the first one).
sn = dungeon("scantinato_nerd", "Scantinato della Fumetteria", 64, 40, ["nerd", "dungeon", "scantinato"])
sn.rect(1, 1, 20, 14, "x")
for (x, y) in [(3, 3), (3, 7), (16, 3), (16, 9)]:
    sn.rect(x, y, x + 2, y + 1, "c")
sn.rect(8, 5, 13, 9, "~")
corridor(sn, (20, 12), (30, 12))
hall(sn, 28, 2, 46, 16, "w")                    # LAN party room
for y in range(4, 15, 3):
    sn.rect(30, y, 36, y, "c")
    sn.rect(39, y, 44, y, "c")
corridor(sn, (37, 16), (37, 22))
maze(sn, 24, 20, 8, 5, 911, floor="w", wall="c")  # comics archive
corridor(sn, (49, 9), (56, 9))
corridor(sn, (46, 9), (49, 9))
hall(sn, 48, 20, 62, 37, "r")                   # the Dungeon Master's lair
sn.rect(52, 22, 58, 24, "c")
corridor(sn, (56, 9), (56, 21))
corridor(sn, (49, 35), (48, 35))
building("baule_nerd", "scantinato_nerd", 55, 34, None)
SUBZONES += [("sala_lan", "Sala LAN", "scantinato_nerd", (28, 2, 19, 15), ["nerd", "svago"]),
             ("archivio_fumetti", "Archivio dei Fumetti", "scantinato_nerd", (24, 20, 25, 16), ["nerd", "dungeon"]),
             ("tana_dungeon_master", "Tana del Dungeon Master", "scantinato_nerd", (48, 20, 15, 18), ["nerd", "dungeon"])]

# Cantine del Culatello, under the salumificio: the gluttons' cellar.
cc = dungeon("cantine_culatello", "Cantine del Culatello", 64, 40, ["cantina", "sotterraneo", "dungeon", "ciccioni"])
hall(cc, 1, 1, 8, 8, "x")
corridor(cc, (8, 4), (14, 4))
hall(cc, 12, 1, 40, 12, "x")                    # hanging culatelli
for y in range(3, 12, 2):
    for x in range(14, 39, 6):
        cc.rect(x, y, x + 3, y, "c")
corridor(cc, (26, 12), (26, 18))
hall(cc, 10, 18, 44, 30, "w")                   # banquet hall
cc.rect(14, 23, 40, 24, "c")
cc.rect(14, 22, 40, 22, "r")
cc.rect(14, 25, 40, 25, "r")
corridor(cc, (44, 24), (50, 24))
hall(cc, 48, 14, 62, 34, "w")                   # the throne of the Great Glutton
cc.rect(53, 16, 57, 30, "r")
corridor(cc, (2, 8), (2, 34))
hall(cc, 1, 32, 12, 38, "x")                    # cheese vault
for x in range(3, 11, 3):
    cc.rect(x, 34, x + 1, 36, "c")
building("dispensa_proibita", "cantine_culatello", 55, 17, None)
SUBZONES += [("sala_banchetto", "Sala del Banchetto", "cantine_culatello", (10, 18, 35, 13), ["ciccioni", "dungeon"]),
             ("trono_gran_mangione", "Trono del Gran Mangione", "cantine_culatello", (48, 14, 15, 21), ["ciccioni", "trono"])]

# Covo dei Rettiliani, under the outlet: servers, the egg hatchery, the council chamber.
cvr = dungeon("covo_rettiliano", "Covo dei Rettiliani", 64, 40, ["rettiliani", "sotterraneo", "dungeon"], fill="#")
hall(cvr, 1, 1, 9, 8, "m")
corridor(cvr, (8, 4), (16, 4), "m")
hall(cvr, 14, 1, 34, 12, "m")                   # server room
for x in range(16, 33, 4):
    cvr.rect(x, 3, x + 1, 10, "c")
corridor(cvr, (24, 12), (24, 18), "m")
hall(cvr, 6, 18, 40, 30, "x")                   # hatchery
for (x, y) in [(9, 21), (15, 21), (21, 21), (27, 21), (33, 21), (9, 26), (15, 26), (27, 26), (33, 26)]:
    cvr.rect(x, y, x + 2, y + 1, "P")
corridor(cvr, (40, 24), (46, 24), "m")
hall(cvr, 44, 6, 62, 36, "m")                   # council chamber
cvr.rect(48, 10, 58, 32, "r")
cvr.rect(50, 18, 56, 24, "c")
cvr.rect(50, 18, 56, 18, "r")
building("caveau_rettiliano", "covo_rettiliano", 53, 9, "cda_fidenza_village")
SUBZONES += [("sala_server", "Sala Server", "covo_rettiliano", (14, 1, 21, 12), ["rettiliani"]),
             ("incubatoio", "Incubatoio", "covo_rettiliano", (6, 18, 35, 13), ["rettiliani", "dungeon"]),
             ("consiglio_rettiliano", "Sala del Consiglio", "covo_rettiliano", (44, 6, 19, 31), ["rettiliani", "dungeon"])]

# Miniera di Sale abbandonata, under the hills of Salso.
ms = dungeon("miniera_di_sale", "Miniera di Sale abbandonata", 64, 40, ["sotterraneo", "miniera", "salso", "dungeon"])
cave(ms, 2, 2, 61, 37, 0.45, 701)
veins(ms, "S", 28, 24, 702)
veins(ms, "G", 10, 6, 703)
ms.rect(2, 2, 7, 7, "n")
tunnel(ms, (6, 6), (40, 30), "n")
ms.rect(10, 8, 18, 12, "n")
tunnel(ms, (6, 6), (14, 12), "n")
tunnel(ms, (14, 12), (24, 21), "n")
building("deposito", "miniera_di_sale", 14, 11, None)
building("fungaia_porcini", "miniera_di_sale", 24, 20, None, "Fungaia dei minatori")


# ── Assembling the maps ─────────────────────────────────────────────────────
SIDES = {"North": (0, -1), "South": (0, 1), "East": (1, 0), "West": (-1, 0)}
REGIONS = []  # (zone id, name, layer id, rect, tags)
PLACE = {}  # region id → (layer id, x offset, y offset)
# Roads across the borders between surface maps: (map, side, map, cells along the border).
CROSSINGS = [
    ("fidenza", "East", "village", [41, 42, 69, 70, 97, 98]),
    ("fidenza", "South", "salsomaggiore", [99, 100, 179, 180]),
    ("fidenza", "North", "bassa", [59, 60, 99, 100, 179, 180]),
]


def citta(m, ox, oy, seed):
    """City block: streets on the cross, houses you can walk into, little gardens and squares."""
    r = Rnd(seed)
    m.rect(ox, oy, ox + RW - 1, oy + RH - 1, ".")
    m.rect(ox, oy + 12, ox + RW - 1, oy + 15, "_")
    m.rect(ox + 18, oy, ox + 21, oy + RH - 1, "_")
    m.rect(ox, oy + 13, ox + RW - 1, oy + 14, "g")
    m.rect(ox + 19, oy, ox + 20, oy + RH - 1, "g")
    for (bx, top) in [(1, True), (22, True), (1, False), (22, False)]:
        kind = r.range(0, 9)
        if kind == 0:  # a little park
            m.rect(ox + bx + 1, oy + (3 if top else 17), ox + bx + 15, oy + (10 if top else 24), ",")
            m.g[oy + (6 if top else 20)][ox + bx + 8] = "F"
            for (dx, dy) in [(2, 1), (14, 1), (2, 6), (14, 6)]:
                m.g[oy + (3 if top else 17) + dy][ox + bx + dx] = "T"
            continue
        for hx in (bx + 1, bx + 9):
            if r.chance(0.15):
                continue
            w = 6 + r.range(0, 1)
            y0, y1 = (oy + 4, oy + 10) if top else (oy + 17, oy + 23)
            x0 = ox + hx
            dx = x0 + w // 2
            dy = y1 if top else y0
            floor = "w" if r.chance(0.6) else "x"
            room(m, x0, y0, x0 + w, y1, floor, doors=[(dx, dy)])
            if top:
                m.g[oy + 11][dx] = "_"
            else:
                m.g[oy + 16][dx] = "_"
            m.g[(y0 + 2) if top else (y1 - 2)][x0 + 2] = "c"
            if r.chance(0.5):
                m.g[(y0 + 2) if top else (y1 - 2)][x0 + w - 2] = "p"
        if r.chance(0.5):
            m.g[oy + (2 if top else 25)][ox + bx + r.range(2, 14)] = "t"


def campi(m, ox, oy, seed, wild=False):
    """Fields of the plain: strips of crops, hedgerows, poplars along the tracks, a ditch."""
    r = Rnd(seed)
    m.rect(ox, oy, ox + RW - 1, oy + RH - 1, ".")
    crops = ["o", '"', "o", ",", '"']
    for (qx0, qy0, qx1, qy1) in [(1, 1, 17, 11), (22, 1, 38, 11), (1, 16, 17, 26), (22, 16, 38, 26)]:
        kind = r.range(0, 9)
        if wild and kind < 4:  # untouched wood or marsh
            cave(m, ox + qx0, oy + qy0, ox + qx1, oy + qy1, 0.55, r.next() & 0xFFFF, floor=".", rock="T", steps=3)
            if kind == 0:
                m.rect(ox + qx0 + 4, oy + qy0 + 3, ox + qx1 - 4, oy + qy1 - 3, "~")
            continue
        crop = crops[r.range(0, len(crops) - 1)]
        for y in range(qy0, qy1 + 1):
            for x in range(qx0, qx1 + 1):
                m.g[oy + y][ox + x] = crop if (y - qy0) % 3 != 2 or crop == "," else '"'
        if r.chance(0.5):
            m.rect(ox + qx0, oy + qy0, ox + qx1, oy + qy0, "t")
    for x in range(0, RW, 4):
        m.g[oy + 12][ox + x] = "T"
        m.g[oy + 15][ox + x] = "T"
    if r.chance(0.6):
        cx = ox + (10 if r.chance(0.5) else 29)
        m.rect(cx, oy, cx, oy + RH - 1, "~")
        m.rect(cx, oy + 13, cx, oy + 14, "b")
    m.rect(ox, oy + 13, ox + RW - 1, oy + 14, ":")
    m.rect(ox + 19, oy, ox + 20, oy + RH - 1, ":")


def colline(m, ox, oy, seed):
    """Hills: rocky outcrops, woods, vineyards, a winding track."""
    r = Rnd(seed)
    m.rect(ox, oy, ox + RW - 1, oy + RH - 1, ".")
    cave(m, ox + 1, oy + 1, ox + RW - 2, oy + RH - 2, 0.62, seed, floor=".", rock="T", steps=3)
    for _ in range(4):
        cx, cy = ox + r.range(4, 35), oy + r.range(3, 24)
        for y in range(cy - 2, cy + 3):
            for x in range(cx - 3, cx + 4):
                if abs(x - cx) + abs(y - cy) < 4:
                    m.g[y][x] = "R"
    if r.chance(0.5):
        vx, vy = ox + r.range(3, 22), oy + r.range(2, 16)
        for y in range(vy, vy + 8):
            for x in range(vx, vx + 12):
                m.g[y][x] = "t" if y % 2 == 0 else "o"
    m.rect(ox, oy + 13, ox + RW - 1, oy + 14, ":")
    m.rect(ox + 19, oy, ox + 20, oy + RH - 1, ":")


def bassa(m, ox, oy, seed, gy):
    campi(m, ox, oy, seed, wild=True)
    if gy == 0:  # il Po
        m.rect(ox, oy + 1, ox + RW - 1, oy + 1, "s")
        m.rect(ox, oy + 2, ox + RW - 1, oy + 9, "~")
        m.rect(ox, oy + 10, ox + RW - 1, oy + 10, "s")
        r = Rnd(seed)
        if r.chance(0.4):
            m.rect(ox + r.range(5, 20), oy + 5, ox + r.range(22, 34), oy + 6, "s")


def build_world(world):
    name, tags = WORLDS[world]
    W, H = RW * GRID, RH * GRID
    surf = Map(world, name, W, H, ".", tags)
    used = {}
    for m in [m for m in MAPS if getattr(m, "world", None) == world]:
        gx, gy = m.at
        used[(gx, gy)] = m
        ox, oy = gx * RW, gy * RH
        PLACE[m.id] = (world, ox, oy)
        rnd = Rnd(stable_hash(m.id) & 0xFFFF)
        for y in range(m.h):
            for x in range(m.w):
                ch = m.g[y][x]
                ring = min(x, y, m.w - 1 - x, m.h - 1 - y)
                if ring == 0 and ch == "T":
                    ch = "T" if rnd.chance(0.2) else ("t" if rnd.chance(0.15) else ".")
                elif ring == 1 and ch == "t":
                    ch = "t" if rnd.chance(0.1) else "."
                surf.g[oy + y][ox + x] = ch
        REGIONS.append((m.id, m.name, world, (ox, oy, m.w, m.h), m.tags + ["quartiere"]))
    for gy in range(GRID):
        for gx in range(GRID):
            if (gx, gy) in used:
                continue
            seed = stable_hash(f"{world}:{gx}:{gy}") & 0xFFFF
            kind = filler_kind(world, gx, gy)
            ox, oy = gx * RW, gy * RH
            if kind == "citta":
                citta(surf, ox, oy, seed)
            elif kind == "campi":
                campi(surf, ox, oy, seed)
            elif kind == "colline":
                colline(surf, ox, oy, seed)
            else:
                bassa(surf, ox, oy, seed, gy)
    if world in ("fidenza", "village"):  # the Via Emilia, east–west through the middle row
        for gx in range(GRID):
            if (gx, 2) not in used:
                surf.rect(gx * RW, 2 * RH + 13, gx * RW + RW - 1, 2 * RH + 14, "g")
    if world == "fidenza":  # the railway, along the station's row
        for gx in range(GRID):
            if (gx, 0) not in used:
                surf.rect(gx * RW, 5, gx * RW + RW - 1, 6, "x")
                surf.rect(gx * RW + 19, 5, gx * RW + 20, 6, "g")
    if world == "village":  # the A1 motorway, north–south
        for gy in range(GRID):
            if (1, gy) not in used:
                surf.rect(RW + 35, gy * RH, RW + 36, gy * RH + RH - 1, "g")
    for x in range(W):
        surf.g[0][x] = surf.g[H - 1][x] = "T"
    for y in range(H):
        surf.g[y][0] = surf.g[y][W - 1] = "T"
    if world == "bassa":
        REGIONS.append(("la_bassa", "La Bassa", world, (0, 0, W, H), ["campi", "selvatico", "quartiere"]))
        REGIONS.append(("fiume_po", "Il Po", world, (0, 0, W, 12), ["fiume", "selvatico"]))
    return surf


def build_under():
    W, H = RW * GRID, RH * GRID
    z = Map("sottosuolo_1", "Regno dei Nani", W, H, "K", ["sotterraneo"], underground=True)
    z.depth = -1
    n = W * H // 2000
    veins(z, "O", n * 2, 16, 299)
    veins(z, "S", n, 12, 309)
    veins(z, "G", max(1, n // 3), 4, 319)
    veins(z, "A", max(1, n // 4), 4, 329)
    veins(z, "h", n, 6, 339)
    veins(z, "Y", max(1, n // 3), 4, 349)
    r = Rnd(361)
    for _ in range(n):
        x, y = r.range(4, W - 16), r.range(4, H - 12)
        cave(z, x, y, x + r.range(6, 12), y + r.range(5, 9), 0.5, r.next() & 0xFFFF, floor="k", rock="K", steps=3)
    for m in [m for m in MAPS if m.underground and not getattr(m, "dungeon", False)]:
        ox, oy = m.at
        for y in range(m.h):
            for x in range(m.w):
                z.g[oy + y][ox + x] = m.g[y][x]
        PLACE[m.id] = (z.id, ox, oy)
        REGIONS.append((m.id, m.name, z.id, (ox, oy, m.w, m.h), m.tags + ["quartiere"]))
    for a, b in DWARF_TUNNELS:
        tunnel(z, a, b, "k", 2)
    return z


def merge():
    global MAPS, PORTALS, EDGES, SUBZONES, BUILDINGS
    surfaces = [build_world(w) for w in WORLDS]
    z = build_under()
    dungeons = [m for m in MAPS if getattr(m, "dungeon", False)]
    by_layer = {m.id: m for m in surfaces + [z] + dungeons}

    def glob(mid, x, y):
        lid, ox, oy = PLACE.get(mid, (mid, 0, 0))
        return lid, x + ox, y + oy

    new_portals = []
    for name, a, b in PORTALS:
        ga, gb = glob(*a), glob(*b)
        src, dst = by_layer[ga[0]], by_layer[gb[0]]
        if dst.underground:
            src.g[ga[2]][ga[1]] = ">"
            dst.g[gb[2]][gb[1]] = "<"
            for dy in (-1, 0, 1):  # a little landing so the stairs are never walled in
                for dx in (-1, 0, 1):
                    yy, xx = gb[2] + dy, gb[1] + dx
                    if (dx or dy) and 0 < yy < dst.h - 1 and 0 < xx < dst.w - 1 and not WALK.get(dst.g[yy][xx], False):
                        dst.g[yy][xx] = "x"
        new_portals.append((name, ga, gb))
    PORTALS = new_portals
    EDGES = []
    for a, side, b, cells in CROSSINGS:
        ma, mb = by_layer[a], by_layer[b]
        for c in cells:
            if side == "East":
                ma.g[c][ma.w - 1] = ma.g[c][ma.w - 2] = ":"
                mb.g[c][0] = mb.g[c][1] = ":"
            elif side == "South":
                ma.g[ma.h - 1][c] = ma.g[ma.h - 2][c] = ":"
                mb.g[0][c] = mb.g[1][c] = ":"
            elif side == "North":
                ma.g[0][c] = ma.g[1][c] = ":"
                mb.g[mb.h - 1][c] = mb.g[mb.h - 2][c] = ":"
        EDGES.append((a, side, b))
    SUBZONES = [(zid, n, PLACE[l][0], (r[0] + PLACE[l][1], r[1] + PLACE[l][2], r[2], r[3]), t) if l in PLACE else (zid, n, l, r, t)
                for zid, n, l, r, t in SUBZONES]
    for m in dungeons:
        REGIONS.append((m.id, m.name, m.id, (0, 0, m.w, m.h), m.tags + ["quartiere"]))
    SUBZONES = REGIONS + SUBZONES
    BUILDINGS = [(b, mid, x + PLACE[mid][1], y + PLACE[mid][2], o, n) if mid in PLACE else (b, mid, x, y, o, n)
                 for b, mid, x, y, o, n in BUILDINGS]
    MAPS = surfaces + [z] + dungeons
    place_venues(by_layer)


GRASS = set('.,"o')  # where a venue can be built (fields get cleared)


def place_venues(by_layer):
    taken = set()
    for _, mid, x, y, _, _ in BUILDINGS:
        lid = PLACE.get(mid, (mid,))[0]
        for yy in range(y - 6, y + 3):
            for xx in range(x - 4, x + 5):
                taken.add((lid, xx, yy))
    free = {}
    for w in WORLDS:
        used = {(r[0] // RW, r[1] // RH) for zid, n, l, r, t in REGIONS if l == w and r[2] == RW}
        free[w] = iter([(gx * RW, gy * RH) for gy in range(GRID) for gx in range(GRID) if (gx, gy) not in used])
    for bid, region, owner, name, fw, fh, world in VENUES:
        if region:
            lid, ox, oy = PLACE[region]
            zone = region
        else:
            lid = world
            ox, oy = next(free[world])
            zone = world
        surf = by_layer[lid]
        cx, cy = ox + RW // 2, oy + RH // 2
        best = None
        for ay in range(oy + fh + 1, oy + RH - 3):
            for ax in range(ox + fw // 2 + 2, ox + RW - fw // 2 - 2):
                cells = [(x, y) for y in range(ay - fh, ay + 2) for x in range(ax - fw // 2 - 1, ax + fw // 2 + 2)]
                if any((lid, *c) in taken or surf.g[c[1]][c[0]] not in GRASS for c in cells):
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
                taken.add((lid, x, y))
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
    merge()
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
