-- M6 (lato giocatore): piu' mondi in parallelo. Un account ha una squadra per ogni mondo a cui
-- si iscrive; punti e inventario sono per mondo. Idempotente come le altre.

ALTER TABLE worlds ADD COLUMN IF NOT EXISTS description TEXT NOT NULL DEFAULT '';
-- Chi amministra il mondo (puo' anche giocarci, vedi design.md). NULL = nessuno l'ha ancora reclamato.
ALTER TABLE worlds ADD COLUMN IF NOT EXISTS owner_id UUID REFERENCES players(id);

CREATE TABLE IF NOT EXISTS memberships (
    player_id UUID NOT NULL REFERENCES players(id),
    world_id  UUID NOT NULL REFERENCES worlds(id),
    points    INT NOT NULL DEFAULT 0,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (player_id, world_id)
);
CREATE INDEX IF NOT EXISTS memberships_world_idx ON memberships (world_id);

ALTER TABLE inventory_items ADD COLUMN IF NOT EXISTS world_id UUID REFERENCES worlds(id);

-- Tutto cio' che esisteva prima appartiene al primo mondo (players.points resta ma non e' piu' usato).
INSERT INTO memberships (player_id, world_id, points)
SELECT p.id, w.id, p.points
FROM players p, (SELECT id FROM worlds ORDER BY created_at LIMIT 1) w
ON CONFLICT DO NOTHING;

UPDATE inventory_items
SET world_id = (SELECT id FROM worlds ORDER BY created_at LIMIT 1)
WHERE world_id IS NULL;

ALTER TABLE inventory_items ALTER COLUMN world_id SET NOT NULL;
CREATE INDEX IF NOT EXISTS inventory_items_world_idx ON inventory_items (world_id, player_id);

-- Il primo giocatore registrato amministra il mondo di prova.
UPDATE worlds
SET owner_id = (SELECT id FROM players ORDER BY created_at LIMIT 1)
WHERE owner_id IS NULL AND id = (SELECT id FROM worlds ORDER BY created_at LIMIT 1);

UPDATE worlds
SET description = 'Il mondo di prova: una piazza, un bosco e un alveare esagonale, con qualche NPC e qualche oggetto.'
WHERE description = '' AND name = 'Mondo di prova';
