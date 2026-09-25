#!/usr/bin/env python3
"""Costruisce la tavola degli sprite dei personaggi a partire dall'eroe standard.

Un solo personaggio (art-src/hero: 4 pose ferme + camminata, un avventuriero chibi con
mantello/cappuccio, generato originale con Pixellab - non deriva da nessuna immagine protetta,
solo uno stile descritto a parole); tutti gli altri sono quel personaggio con qualcosa di cambiato:
  - la RAZZA le da' un colore (LOOKS: il cappuccio/mantello tinto con un'altra tonalita');
  - il RUOLO aggiunge un accessorio: il campione porta una corona;
  - gli NPC hanno colori e copricapo loro (NPCS).
I nomi delle righe sono quelli che il server manda (backend/internal/game/sprites.go: Looks,
NPCSprites, UnitSprite): tienili allineati.

Uso:  python3 tools/sprites/build_characters.py [--preview anteprima.png]
Scrive unity-client/Assets/Resources/Art/characters.bytes (un PNG) e characters-layout.json.
"""
import argparse
import colorsys
import json
import os
from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
SRC = os.path.join(ROOT, 'art-src', 'hero')
OUT = os.path.join(ROOT, 'unity-client', 'Assets', 'Resources', 'Art')
# Una seconda copia, in un formato normale (.png), per l'app admin: mostra l'aspetto di ogni
# razza/classe/NPC (vedi admin-web/app.js, spritePreview) senza duplicare il disegno in JS.
ADMIN_OUT = os.path.join(ROOT, 'admin-web', 'art')

DIRS = ['south', 'east', 'north', 'west']   # l'ordine delle celle: vedi CharacterSprites.cs
SRC_CELL = 32                               # la dimensione dei fotogrammi generati
CELL = 40                                   # la cella della tavola: lascia spazio sopra la testa per i copricapo
MARGIN_X, MARGIN_Y = (CELL - SRC_CELL) // 2, CELL - SRC_CELL
WALK_FRAMES = 8
FEET = 29 + MARGIN_Y                        # la riga (dall'alto) dove poggiano i piedi

# Il mantello/cappuccio generato e' un verdeacqua (tonalita' ~140-200 gradi); pelle, capelli,
# stivali e cintura vivono in un'altra fascia di tonalita' (~15-45) e restano intatti.
CLOAK_HUE_MIN, CLOAK_HUE_MAX = 140, 200

# Ogni razza tinge il mantello/cappuccio con una tonalita' diversa (gradi 0-360); 'sat' scala
# la saturazione originale (1.0 = uguale, <1 = piu' spento, per 'slate').
LOOKS = {
    'salmon': {'hue': 352, 'sat': 1.05},
    'azure':  {'hue': 208, 'sat': 1.0},
    'moss':   {'hue': 100, 'sat': 1.0},
    'sun':    {'hue': 38, 'sat': 1.0},
    'violet': {'hue': 272, 'sat': 1.0},
    'slate':  {'hue': 212, 'sat': 0.35},
}

# Accessori: disegnati una volta e appoggiati in cima alla testa, dove che sia la pedina in ogni
# posa e fotogramma. Legenda: k contorno, g/G oro, r gemma, s/S acciaio, n/N legno, b/B blu, y giallo, w bianco.
COLORS = {
    'k': (15, 8, 42), 'g': (250, 208, 70), 'G': (196, 142, 30), 'r': (222, 66, 66),
    's': (176, 182, 198), 'S': (112, 118, 136), 'n': (160, 108, 62), 'N': (102, 66, 38),
    'b': (70, 100, 200), 'B': (40, 56, 140), 'y': (255, 236, 120), 'w': (238, 238, 238),
}
ACCESSORIES = {
    'crown': ['k.k.k.k.k', 'kgkgkgkgk', 'kgggrgggk', 'kGGGGGGGk', 'kkkkkkkkk'],
    'merchant': ['....kkkkk....', '...knnnnnk...', 'kkknNNNNNnkkk', 'knnnnnnnnnnnk', '.kkkkkkkkkkk.'],
    'helmet': ['...kkkkk...', '..ksswssk..', '.kssssssSk.', 'kSSSSSSSSSk', '.kkkkkkkkk.'],
    'wizard': ['.....k.....', '....kbk....', '....kbk....', '...kbbbk...', '...kbybk...', '..kbbbbbk..', '.kbbbbbbbk.', 'kkkkkkkkkkk'],
    'cap': ['..kkkkkkk..', '.kSSSSSSSk.', 'kSSSSSSSSSk', 'kkkkkkkkkkk'],
    'hood': ['...kkkkk...', '..knnnnnk..', '.knnNNNnnk.', 'knnnnnnnnnk', 'kn.......nk'],
}

# Cosa e' ogni NPC: (tavolozza, accessorio).
NPCS = {
    'merchant': ('moss', 'merchant'), 'guard': ('slate', 'helmet'), 'sage': ('violet', 'wizard'),
    'blacksmith': ('sun', 'cap'), 'wanderer': ('azure', 'hood'),
}


def load(name):
    return Image.open(os.path.join(SRC, name)).convert('RGBA')


def remove_specks(im, min_size=40):
    """Toglie i frammenti staccati dal corpo (rimasugli del disegno di partenza)."""
    w, h = im.size
    px = im.load()
    seen = set()
    for y in range(h):
        for x in range(w):
            if px[x, y][3] == 0 or (x, y) in seen:
                continue
            comp, stack = [], [(x, y)]
            seen.add((x, y))
            while stack:
                cx, cy = stack.pop()
                comp.append((cx, cy))
                for dx in (-1, 0, 1):
                    for dy in (-1, 0, 1):
                        nx, ny = cx + dx, cy + dy
                        if 0 <= nx < w and 0 <= ny < h and (nx, ny) not in seen and px[nx, ny][3]:
                            seen.add((nx, ny))
                            stack.append((nx, ny))
            if len(comp) < min_size:
                for cx, cy in comp:
                    px[cx, cy] = (0, 0, 0, 0)
    return im


def recolor(im, look):
    """Tinge il mantello/cappuccio (i pixel la cui tonalita' cade nella fascia del verdeacqua
    originale) con la tonalita' della razza, mantenendo la sua stessa luminosita' e sfumatura;
    pelle, capelli, cintura e stivali hanno un'altra tonalita' e restano intatti."""
    hue, sat_mult = LOOKS[look]['hue'] / 360, LOOKS[look]['sat']
    out = im.copy()
    px = out.load()
    for y in range(out.height):
        for x in range(out.width):
            r, g, b, a = px[x, y]
            if not a:
                continue
            h, s, v = colorsys.rgb_to_hsv(r / 255, g / 255, b / 255)
            if CLOAK_HUE_MIN / 360 <= h <= CLOAK_HUE_MAX / 360:
                nr, ng, nb = colorsys.hsv_to_rgb(hue, min(1.0, s * sat_mult), v)
                px[x, y] = (round(nr * 255), round(ng * 255), round(nb * 255), a)
    return out


def add_accessory(im, name):
    """Appoggia l'accessorio in cima alla testa (la riga piu' alta del corpo, al suo centro)."""
    art = ACCESSORIES[name]
    px = im.load()
    top = next((y for y in range(im.height) if any(px[x, y][3] for x in range(im.width))), None)
    if top is None:
        return im
    xs = [x for y in (top, top + 1) for x in range(im.width) if px[x, y][3]]
    cx = round(sum(xs) / len(xs))
    out = im.copy()
    op = out.load()
    h, w = len(art), len(art[0])
    y0 = top + 1 - (h - 1)            # la riga piu' bassa dell'accessorio copre la seconda riga del corpo
    for j, row in enumerate(art):
        for i, ch in enumerate(row):
            if ch == '.':
                continue
            x, y = cx - w // 2 + i, y0 + j
            if 0 <= x < out.width and 0 <= y < out.height:
                op[x, y] = COLORS[ch] + (255,)
    return out


def frames():
    idle = {d: remove_specks(load(f'idle_{d}.png')) for d in DIRS}
    walk = {}
    for d in DIRS:
        walk[d] = [remove_specks(load(f'walk_{d}_{i}.png')) for i in range(1, WALK_FRAMES + 1)]
    return idle, walk


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--preview', help="scrive anche un'anteprima ingrandita")
    args = ap.parse_args()
    idle, walk = frames()

    rows = []   # (nome, tavolozza, accessorio, cammina)
    for look in LOOKS:
        rows.append((f'champion-{look}', look, 'crown', True))
    for look in LOOKS:
        rows.append((f'minor-{look}', look, None, True))
    rows.append(('champion', 'salmon', 'crown', True))
    rows.append(('minor', 'salmon', None, True))
    for name, (look, acc) in NPCS.items():
        rows.append((name, look, acc, False))

    columns = 4 + 4 * WALK_FRAMES
    atlas = Image.new('RGBA', (columns * CELL, len(rows) * CELL), (0, 0, 0, 0))
    layout = {'cell': CELL, 'feet': FEET, 'directions': DIRS, 'walkFrames': WALK_FRAMES, 'characters': []}
    for r, (name, look, acc, walks) in enumerate(rows):
        def make(im):
            cell = Image.new('RGBA', (CELL, CELL), (0, 0, 0, 0))
            cell.alpha_composite(im, (MARGIN_X, MARGIN_Y))
            im = recolor(cell, look)
            return add_accessory(im, acc) if acc else im
        for di, d in enumerate(DIRS):
            atlas.alpha_composite(make(idle[d]), (di * CELL, r * CELL))
            if walks:
                for f, frame in enumerate(walk[d]):
                    atlas.alpha_composite(make(frame), ((4 + di * WALK_FRAMES + f) * CELL, r * CELL))
        layout['characters'].append({'name': name, 'row': r, 'walk': walks})

    os.makedirs(OUT, exist_ok=True)
    atlas.save(os.path.join(OUT, 'characters.bytes'), format='PNG')
    with open(os.path.join(OUT, 'characters-layout.json'), 'w') as f:
        json.dump(layout, f)
    os.makedirs(ADMIN_OUT, exist_ok=True)
    atlas.save(os.path.join(ADMIN_OUT, 'characters.png'), format='PNG')
    with open(os.path.join(ADMIN_OUT, 'characters-layout.json'), 'w') as f:
        json.dump(layout, f)
    print(f'{len(rows)} varianti, tavola {atlas.width}x{atlas.height}')
    if args.preview:
        bg = Image.new('RGBA', atlas.size, (123, 201, 111, 255))
        bg.alpha_composite(atlas)
        bg.resize((atlas.width * 3, atlas.height * 3), Image.NEAREST).save(args.preview)


if __name__ == '__main__':
    main()
