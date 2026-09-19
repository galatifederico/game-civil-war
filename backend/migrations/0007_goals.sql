-- M5: obiettivi e condizioni di vittoria. Definiti dall'admin di ogni mondo (dati, non codice).
-- Idempotente come le altre.

CREATE TABLE IF NOT EXISTS goals (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    world_id    UUID NOT NULL REFERENCES worlds(id),
    -- world: vince chi lo raggiunge per primo; individual: se ne assegna uno a ogni giocatore
    scope       TEXT NOT NULL CHECK (scope IN ('world', 'individual')),
    kind        TEXT NOT NULL CHECK (kind IN ('points', 'kills', 'champion_kills', 'pickups', 'structures', 'units', 'talks')),
    target      INT NOT NULL CHECK (target > 0),
    reward      INT NOT NULL DEFAULT 0,
    title       TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    position    INT NOT NULL DEFAULT 0,
    achieved_by UUID REFERENCES players(id),
    achieved_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS goals_world_idx ON goals (world_id);

-- L'obiettivo individuale assegnato a un giocatore e, se finito, quando.
CREATE TABLE IF NOT EXISTS player_goals (
    player_id    UUID NOT NULL REFERENCES players(id),
    goal_id      UUID NOT NULL REFERENCES goals(id),
    assigned_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    PRIMARY KEY (player_id, goal_id)
);

-- Contatori con cui si misurano gli obiettivi (sconfitte, oggetti raccolti, NPC incontrati...).
ALTER TABLE memberships ADD COLUMN IF NOT EXISTS stats JSONB NOT NULL DEFAULT '{}'::jsonb;

-- Seed: obiettivi nel mondo di prova.
INSERT INTO goals (world_id, scope, kind, target, reward, title, description, position)
SELECT w.id, v.scope, v.kind, v.target, v.reward, v.title, v.description, v.position
FROM (SELECT id FROM worlds ORDER BY created_at LIMIT 1) w,
     (VALUES
        ('individual', 'talks',      3,  20, 'Chiacchierone',  'Parla con 3 NPC diversi. Qualcuno ti dirà qualcosa di utile, prima o poi.', 0),
        ('individual', 'pickups',    3,  25, 'Raccoglitore',   'Raccogli 3 oggetti. Non importa che cosa siano.', 1),
        ('individual', 'kills',      3,  40, 'Attaccabrighe',  'Metti fuori gioco 3 pedine avversarie.', 2),
        ('individual', 'structures', 2,  30, 'Costruttore',    'Costruisci 2 avamposti: il territorio non si conquista da solo.', 3),
        ('individual', 'units',      20, 35, 'Famiglia numerosa', 'Arriva ad avere 20 pedine nella squadra.', 4),
        ('world',      'points',     300, 100, 'Il più ricco',  'Il primo a raggiungere 300 punti vince.', 5),
        ('world',      'champion_kills', 2, 150, 'Il più temuto', 'Il primo a sconfiggere 2 campioni avversari vince.', 6),
        ('world',      'structures', 6, 120, 'Imperatore della Piazza', 'Il primo a possedere 6 avamposti vince.', 7)
     ) AS v(scope, kind, target, reward, title, description, position)
WHERE NOT EXISTS (SELECT 1 FROM goals WHERE world_id = w.id);
