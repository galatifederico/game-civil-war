-- Schema iniziale (M1). Idempotente: si puo' rilanciare senza effetti collaterali.

CREATE TABLE IF NOT EXISTS players (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT NOT NULL UNIQUE,
    username      TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS worlds (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name       TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS boards (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    world_id   UUID NOT NULL REFERENCES worlds(id),
    name       TEXT NOT NULL,
    width      INT NOT NULL,
    height     INT NOT NULL,
    grid_kind  TEXT NOT NULL DEFAULT 'square',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Pedine: campioni, pedine minori (player_id valorizzato) e NPC (player_id NULL).
CREATE TABLE IF NOT EXISTS units (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    board_id    UUID NOT NULL REFERENCES boards(id),
    player_id   UUID REFERENCES players(id),
    kind        TEXT NOT NULL CHECK (kind IN ('champion', 'minor', 'npc')),
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    x           INT NOT NULL,
    y           INT NOT NULL,
    speed       INT NOT NULL DEFAULT 2,
    health      INT NOT NULL DEFAULT 100,
    max_health  INT NOT NULL DEFAULT 100,
    vision      INT NOT NULL DEFAULT 3,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (board_id, x, y)
);
CREATE INDEX IF NOT EXISTS units_player_idx ON units (player_id);

-- Oggetti sulla board (non sono pedine: possono essere spostati/raccolti, vedi design.md).
CREATE TABLE IF NOT EXISTS board_items (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    board_id    UUID NOT NULL REFERENCES boards(id),
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    x           INT NOT NULL,
    y           INT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (board_id, x, y)
);

-- Seed: un mondo, una board 24x24, 5 NPC e 3 oggetti (solo se il DB e' vuoto).
INSERT INTO worlds (name)
SELECT 'Mondo di prova' WHERE NOT EXISTS (SELECT 1 FROM worlds);

INSERT INTO boards (world_id, name, width, height)
SELECT w.id, 'Piazza', 24, 24
FROM worlds w
WHERE NOT EXISTS (SELECT 1 FROM boards)
ORDER BY w.created_at
LIMIT 1;

INSERT INTO units (board_id, kind, name, description, x, y, speed, health, max_health, vision)
SELECT b.id, 'npc', v.name, v.description, v.x, v.y, 0, 100, 100, 3
FROM boards b,
     (VALUES
        ('Mercante',       'Vende merci di dubbia provenienza a prezzi ancora più dubbi. Non si allontana dal suo banco.', 9, 9),
        ('Guardia',        'Sorveglia la piazza con aria annoiata. Non è nella tua squadra e non prende ordini da te.', 14, 8),
        ('Vecchio saggio', 'Ha una risposta per tutto, quasi mai a una domanda che gli hai fatto.', 18, 13),
        ('Fabbro',         'Ripara armi e armature. Sostiene che il martello sia sempre "quasi pronto".', 10, 15),
        ('Viandante',      'Di passaggio, come sempre. Nessuno sa da dove venga né dove stia andando.', 15, 16)
     ) AS v(name, description, x, y)
WHERE NOT EXISTS (SELECT 1 FROM units WHERE kind = 'npc')
  AND b.id = (SELECT id FROM boards ORDER BY created_at LIMIT 1);

INSERT INTO board_items (board_id, name, description, x, y)
SELECT b.id, v.name, v.description, v.x, v.y
FROM boards b,
     (VALUES
        ('Forziere',  'Chiuso a chiave. Nessuno ricorda dove sia finita la chiave.', 12, 11),
        ('Cristallo', 'Emette un debole bagliore. Meglio non toccarlo.', 16, 10),
        ('Cartello',  'Recita: "Lavori in corso". I lavori non sono mai iniziati.', 11, 14)
     ) AS v(name, description, x, y)
WHERE NOT EXISTS (SELECT 1 FROM board_items)
  AND b.id = (SELECT id FROM boards ORDER BY created_at LIMIT 1);
