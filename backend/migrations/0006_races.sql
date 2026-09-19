-- M4: razze, compatibilita' per la riproduzione, caratteristiche estese ed effetti degli oggetti.
-- Sono tutti dati per mondo (li modifica l'admin). Idempotente come le altre.

CREATE TABLE IF NOT EXISTS races (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    world_id       UUID NOT NULL REFERENCES worlds(id),
    name           TEXT NOT NULL,
    description    TEXT NOT NULL DEFAULT '',
    position       INT NOT NULL DEFAULT 0,
    -- caratteristiche minime di una pedina di questa razza...
    speed          INT NOT NULL,
    health         INT NOT NULL,
    vision         INT NOT NULL,
    strength       INT NOT NULL,
    -- ...piu' un valore casuale da 0 a `bonus_*` sopra ogni minimo (design.md: ereditarieta')
    bonus_speed    INT NOT NULL DEFAULT 0,
    bonus_health   INT NOT NULL DEFAULT 0,
    bonus_vision   INT NOT NULL DEFAULT 0,
    bonus_strength INT NOT NULL DEFAULT 0,
    traits_min     JSONB NOT NULL DEFAULT '{}'::jsonb,
    traits_bonus   JSONB NOT NULL DEFAULT '{}'::jsonb,
    UNIQUE (world_id, name)
);

-- Quali coppie di razze possono avere figli e di che razza (una riga per coppia non ordinata).
CREATE TABLE IF NOT EXISTS race_compatibility (
    world_id   UUID NOT NULL REFERENCES worlds(id),
    race_a     UUID NOT NULL REFERENCES races(id),
    race_b     UUID NOT NULL REFERENCES races(id),
    child_race UUID NOT NULL REFERENCES races(id),
    PRIMARY KEY (race_a, race_b)
);

ALTER TABLE units ADD COLUMN IF NOT EXISTS race_id UUID REFERENCES races(id);
ALTER TABLE units ADD COLUMN IF NOT EXISTS traits JSONB NOT NULL DEFAULT '{}'::jsonb;
ALTER TABLE memberships ADD COLUMN IF NOT EXISTS race_id UUID REFERENCES races(id);
-- Cosa fa usare l'oggetto (vedi game.Effect): {"heal": n, "points": n, "strength": n, "traits": {...}}.
ALTER TABLE board_items ADD COLUMN IF NOT EXISTS effect JSONB NOT NULL DEFAULT '{}'::jsonb;
ALTER TABLE inventory_items ADD COLUMN IF NOT EXISTS effect JSONB NOT NULL DEFAULT '{}'::jsonb;

-- Seed: tre razze nel mondo di prova.
INSERT INTO races (world_id, name, description, position, speed, health, vision, strength,
                   bonus_speed, bonus_health, bonus_vision, bonus_strength, traits_min, traits_bonus)
SELECT w.id, 'Balordi', 'Gente di strada: robusta, testarda e sempre a corto di contanti. Reggono bene i colpi e ancora meglio le serate.',
       0, 2, 100, 3, 15, 0, 20, 1, 5,
       '{"soldi": 5, "alcol": 20, "alpha": 10, "thc": 5, "beatitudine": 0, "mana": 0}',
       '{"soldi": 10, "alcol": 10, "alpha": 10, "thc": 10}'
FROM (SELECT id FROM worlds ORDER BY created_at LIMIT 1) w
WHERE NOT EXISTS (SELECT 1 FROM races WHERE name = 'Balordi' AND world_id = w.id);

INSERT INTO races (world_id, name, description, position, speed, health, vision, strength,
                   bonus_speed, bonus_health, bonus_vision, bonus_strength, traits_min, traits_bonus)
SELECT w.id, 'Fighetti', 'Veloci, attenti e fragili come un aperitivo in un bicchiere sbagliato. Hanno sempre qualcosa da mostrare.',
       1, 3, 70, 4, 10, 0, 15, 1, 5,
       '{"soldi": 40, "alcol": 0, "alpha": 5, "thc": 0, "beatitudine": 10, "mana": 10}',
       '{"soldi": 30, "mana": 10}'
FROM (SELECT id FROM worlds ORDER BY created_at LIMIT 1) w
WHERE NOT EXISTS (SELECT 1 FROM races WHERE name = 'Fighetti' AND world_id = w.id);

INSERT INTO races (world_id, name, description, position, speed, health, vision, strength,
                   bonus_speed, bonus_health, bonus_vision, bonus_strength, traits_min, traits_bonus)
SELECT w.id, 'Sbandati', 'Nati dall''incontro tra le due strade. Nessuno sa bene cosa siano, loro per primi.',
       2, 3, 90, 3, 12, 0, 20, 1, 5,
       '{"soldi": 5, "alcol": 15, "alpha": 0, "thc": 20, "beatitudine": 15, "mana": 5}',
       '{"thc": 10, "beatitudine": 10, "mana": 5}'
FROM (SELECT id FROM worlds ORDER BY created_at LIMIT 1) w
WHERE NOT EXISTS (SELECT 1 FROM races WHERE name = 'Sbandati' AND world_id = w.id);

-- Compatibilita': ognuno si riproduce con i suoi; Balordi e Fighetti insieme danno Sbandati, e gli
-- Sbandati si riproducono con tutti dando altri Sbandati.
INSERT INTO race_compatibility (world_id, race_a, race_b, child_race)
SELECT w.id, a.id, b.id, c.id
FROM (SELECT id FROM worlds ORDER BY created_at LIMIT 1) w,
     (VALUES ('Balordi', 'Balordi', 'Balordi'), ('Fighetti', 'Fighetti', 'Fighetti'), ('Balordi', 'Fighetti', 'Sbandati'),
             ('Sbandati', 'Sbandati', 'Sbandati'), ('Sbandati', 'Balordi', 'Sbandati'), ('Sbandati', 'Fighetti', 'Sbandati')) AS v(a, b, c),
     races a, races b, races c
WHERE a.world_id = w.id AND a.name = v.a AND b.world_id = w.id AND b.name = v.b AND c.world_id = w.id AND c.name = v.c
ON CONFLICT DO NOTHING;

-- Le squadre che esistevano prima sono di razza Balordi (cosi' possono riprodursi).
UPDATE units SET race_id = (SELECT r.id FROM races r WHERE r.name = 'Balordi' ORDER BY r.position LIMIT 1)
WHERE player_id IS NOT NULL AND race_id IS NULL;
UPDATE memberships SET race_id = (SELECT r.id FROM races r WHERE r.name = 'Balordi' AND r.world_id = memberships.world_id LIMIT 1)
WHERE race_id IS NULL;

-- Effetti degli oggetti del mondo di prova.
UPDATE board_items SET effect = '{"points": 30}' WHERE name = 'Forziere' AND effect = '{}'::jsonb;
UPDATE board_items SET effect = '{"heal": 60, "traits": {"mana": 10}}' WHERE name = 'Cristallo' AND effect = '{}'::jsonb;
UPDATE board_items SET effect = '{"traits": {"thc": 10, "beatitudine": 5}}' WHERE name = 'Fungo' AND effect = '{}'::jsonb;
UPDATE board_items SET effect = '{"strength": 5}' WHERE name = 'Ascia' AND effect = '{}'::jsonb;
UPDATE board_items SET effect = '{"heal": 40, "traits": {"beatitudine": 5}}' WHERE name = 'Miele' AND effect = '{}'::jsonb;
