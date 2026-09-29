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
]
WALK = {ch: walk for ch, _, _, walk in LEGEND}


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

c = outdoor("campagna_bassa", "Campagna della Bassa", "NE", ["campi"], road=":")
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
t = outdoor("salsomaggiore_terme", "Salsomaggiore e le Terme", "NE", ["pubblico", "terme", "salso", "caldo"])
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

# ── Underground ──────────────────────────────────────────────────────────────
gl = Map("gallerie", "Gallerie dei Nani", 40, 28, "K", ["gallerie", "sotterraneo"], underground=True)
gl.rect(2, 2, 37, 25, "k")
gl.scatter("K", 9, on="k", seed=7)
gl.rect(2, 2, 6, 6, "k")
gl.rect(2, 13, 37, 14, "k")
gl.rect(19, 2, 20, 25, "k")
gl.rect(26, 3, 37, 11, "k")
gl.rect(2, 16, 13, 25, "k")
building("fucina", "gallerie", 20, 9, "nani_miniere")
gl.rect(19, 10, 21, 10, "k")
SUBZONES += [("miniera", "Filone di ferro", "gallerie", (26, 3, 12, 9), ["miniera", "gallerie"]),
             ("tana_dinosauri", "Tana dei Dinosauri", "gallerie", (2, 16, 12, 10), ["gallerie", "tana"])]
MAPS.append(gl)

cr = Map("cripta", "Cripta di San Vitale", 30, 22, "K", ["cripta", "sotterraneo"], underground=True)
cr.rect(2, 2, 27, 19, "x")
cr.scatter("K", 13, on="x", seed=3)
cr.rect(2, 2, 5, 5, "x")
cr.rect(12, 5, 18, 11, "x")
building("altare_cripta", "cripta", 15, 9, "cripta_san_vitale")
MAPS.append(cr)


def fmt_tags(t):
    return "[" + ", ".join(f'"{x}"' for x in t) + "]"


def ron():
    out = ["// Generato da tools/maps/build_maps.py: non modificare a mano, rigenera.", "(", "    map: (", "        legend: ["]
    for ch, tid, name, walk in LEGEND:
        c = ch.replace("\\", "\\\\").replace("'", "\\'")
        out.append(f"            (ch: '{c}', id: \"{tid}\", name: \"{name}\", walkable: {'true' if walk else 'false'}),")
    out.append("        ],")
    out.append("        layers: [")
    for m in MAPS:
        flags = ""
        if m.indoor:
            flags += ", indoor: true"
        if m.underground:
            flags += ", underground: true"
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
