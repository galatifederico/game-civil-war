#!/usr/bin/env python3
"""Costruisce la tavola degli sprite dei personaggi a partire dalla pedina standard.

Una sola pedina (art-src/pawn: 4 pose ferme + camminata, generate con Pixellab da img/pawn.png);
tutte le altre sono quella pedina con qualcosa di cambiato:
  - la RAZZA le da' un colore (LOOKS: la stessa pedina con un'altra tavolozza);
  - il RUOLO aggiunge un accessorio: il campione porta una corona;
  - gli NPC hanno colori e copricapo loro (NPCS).
I nomi delle righe sono quelli che il server manda (backend/internal/game/sprites.go: Looks,
NPCSprites, UnitSprite): tienili allineati.

Uso:  python3 tools/sprites/build_characters.py [--preview anteprima.png]
Scrive unity-client/Assets/Resources/Art/characters.bytes (un PNG) e characters-layout.json.
"""
import argparse
import json
import os
from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
SRC = os.path.join(ROOT, 'art-src', 'pawn')
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

# Colori della pedina originale (salmone) e classi in cui si dividono.
BASE = {
    'outline': (15, 8, 42), 'main': (237, 104, 101), 'shadow': (152, 53, 84),
    'light': (248, 188, 188), 'white': (248, 246, 252), 'accent': (96, 11, 84),
}

# Le altre tavolozze: cambiano il corpo (main, shadow, light, accent); contorno e occhi restano.
LOOKS = {
    'salmon': {},
    'azure':  {'main': (98, 160, 232), 'shadow': (52, 92, 168), 'light': (186, 220, 252), 'accent': (30, 50, 110)},
    'moss':   {'main': (120, 196, 110), 'shadow': (60, 124, 76), 'light': (200, 236, 170), 'accent': (30, 70, 60)},
    'sun':    {'main': (244, 196, 84), 'shadow': (190, 120, 48), 'light': (252, 232, 170), 'accent': (120, 60, 30)},
    'violet': {'main': (170, 120, 220), 'shadow': (108, 72, 156), 'light': (222, 196, 248), 'accent': (60, 30, 100)},
    'slate':  {'main': (150, 160, 176), 'shadow': (90, 98, 120), 'light': (214, 220, 232), 'accent': (50, 56, 76)},
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


def remove_scribbles(im):
    """Toglie le linee sottili di contorno che non toccano il corpo (rimasugli del disegno di partenza):
    un pixel di contorno senza nessun colore pieno del corpo nei dintorni non fa parte della pedina."""
    px = im.load()
    for _ in range(3):
        gone = []
        for y in range(im.height):
            for x in range(im.width):
                if px[x, y][3] == 0 or classify(px[x, y][:3]) != 'outline':
                    continue
                near = False
                for dy in range(-2, 3):
                    for dx in range(-2, 3):
                        nx, ny = x + dx, y + dy
                        if 0 <= nx < im.width and 0 <= ny < im.height and px[nx, ny][3] and classify(px[nx, ny][:3]) in ('main', 'shadow', 'light', 'accent'):
                            near = True
                if not near:
                    gone.append((x, y))
        for x, y in gone:
            px[x, y] = (0, 0, 0, 0)
    return im


def classify(rgb):
    best, dist = None, 1e9
    for name, c in BASE.items():
        d = sum((a - b) ** 2 for a, b in zip(rgb, c))
        if d < dist:
            best, dist = name, d
    return best if dist < 60 * 60 else None


def recolor(im, look):
    palette = LOOKS[look]
    if not palette:
        return im.copy()
    out = im.copy()
    px = out.load()
    for y in range(out.height):
        for x in range(out.width):
            r, g, b, a = px[x, y]
            if a:
                cls = classify((r, g, b))
                if cls in palette:
                    px[x, y] = palette[cls] + (a,)
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
    idle = {d: remove_scribbles(remove_specks(load(f'idle_{d}.png'))) for d in DIRS}
    walk = {}
    for d in DIRS:
        walk[d] = [remove_scribbles(remove_specks(load(f'walk_{d}_{i}.png'))) for i in range(1, WALK_FRAMES + 1)]
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
