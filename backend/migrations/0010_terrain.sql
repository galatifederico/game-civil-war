-- Terreno delle caselle (grafica dall'alto): ogni casella e' erba, salvo quelle elencate qui.
-- Alcuni tipi bloccano il passaggio (alberi, cespugli, staccionate, muri): vedi game.TerrainKinds.
-- Idempotente come le altre.

CREATE TABLE IF NOT EXISTS board_terrain (
    board_id UUID NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    x        INT NOT NULL,
    y        INT NOT NULL,
    tile     TEXT NOT NULL CHECK (tile IN ('flowers', 'path', 'stone', 'tree', 'bush', 'fence', 'wall')),
    PRIMARY KEY (board_id, x, y)
);

-- Seed una volta sola per le board che esistevano gia': un filare di alberi lungo il bordo, tranne
-- dove c'e' un passaggio, l'arrivo di un passaggio o qualcuno. Le board create dopo partono senza
-- terreno (default true) e non vengono toccate.
ALTER TABLE boards ADD COLUMN IF NOT EXISTS terrain_seeded BOOLEAN NOT NULL DEFAULT false;

INSERT INTO board_terrain (board_id, x, y, tile)
SELECT b.id, gx.x, gy.y, 'tree'
FROM boards b
CROSS JOIN LATERAL generate_series(0, b.width - 1) AS gx(x)
CROSS JOIN LATERAL generate_series(0, b.height - 1) AS gy(y)
WHERE NOT b.terrain_seeded
  AND (gx.x = 0 OR gy.y = 0 OR gx.x = b.width - 1 OR gy.y = b.height - 1)
  AND NOT EXISTS (SELECT 1 FROM board_links l WHERE (l.from_board_id = b.id AND l.from_x = gx.x AND l.from_y = gy.y)
                                                 OR (l.to_board_id = b.id AND l.to_x = gx.x AND l.to_y = gy.y))
  AND NOT EXISTS (SELECT 1 FROM units u WHERE u.board_id = b.id AND u.x = gx.x AND u.y = gy.y)
  AND NOT EXISTS (SELECT 1 FROM board_items i WHERE i.board_id = b.id AND i.x = gx.x AND i.y = gy.y)
  AND NOT EXISTS (SELECT 1 FROM structures s WHERE s.board_id = b.id AND s.x = gx.x AND s.y = gy.y)
ON CONFLICT DO NOTHING;

UPDATE boards SET terrain_seeded = true WHERE NOT terrain_seeded;
ALTER TABLE boards ALTER COLUMN terrain_seeded SET DEFAULT true;
