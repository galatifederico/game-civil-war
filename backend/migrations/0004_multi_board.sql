-- M3: un mondo e' fatto di piu' board collegate da passaggi (gateway), anche di forma diversa.
-- Idempotente come le altre.

ALTER TABLE boards ADD COLUMN IF NOT EXISTS position INT NOT NULL DEFAULT 0;

-- Una casella (from) che porta chi ci mette piede in un'altra casella (to), di solito su un'altra
-- board. I collegamenti sono a senso unico: un passaggio percorribile nei due versi sono due righe.
CREATE TABLE IF NOT EXISTS board_links (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_board_id UUID NOT NULL REFERENCES boards(id),
    from_x        INT NOT NULL,
    from_y        INT NOT NULL,
    to_board_id   UUID NOT NULL REFERENCES boards(id),
    to_x          INT NOT NULL,
    to_y          INT NOT NULL,
    UNIQUE (from_board_id, from_x, from_y)
);

-- Seed: due board nuove nel mondo di prova. La Piazza (quadrata) resta la board di partenza.
INSERT INTO boards (world_id, name, width, height, grid_kind, position)
SELECT w.id, 'Bosco', 20, 16, 'square', 1
FROM worlds w
WHERE NOT EXISTS (SELECT 1 FROM boards WHERE name = 'Bosco')
ORDER BY w.created_at
LIMIT 1;

INSERT INTO boards (world_id, name, width, height, grid_kind, position)
SELECT w.id, 'Alveare', 12, 12, 'hex', 2
FROM worlds w
WHERE NOT EXISTS (SELECT 1 FROM boards WHERE name = 'Alveare')
ORDER BY w.created_at
LIMIT 1;

-- Piazza (lato est) <-> Bosco (lato ovest), Bosco (lato nord) <-> Alveare (lato sud).
-- Chi entra in una casella-passaggio arriva una casella oltre il passaggio opposto.
INSERT INTO board_links (from_board_id, from_x, from_y, to_board_id, to_x, to_y)
SELECT a.id, 23, 10 + k, b.id, 1, 6 + k
FROM boards a, boards b, generate_series(0, 3) AS k
WHERE a.name = 'Piazza' AND b.name = 'Bosco'
ON CONFLICT DO NOTHING;

INSERT INTO board_links (from_board_id, from_x, from_y, to_board_id, to_x, to_y)
SELECT b.id, 0, 6 + k, a.id, 22, 10 + k
FROM boards a, boards b, generate_series(0, 3) AS k
WHERE a.name = 'Piazza' AND b.name = 'Bosco'
ON CONFLICT DO NOTHING;

INSERT INTO board_links (from_board_id, from_x, from_y, to_board_id, to_x, to_y)
SELECT b.id, 8 + k, 15, h.id, 2 + k, 1
FROM boards b, boards h, generate_series(0, 3) AS k
WHERE b.name = 'Bosco' AND h.name = 'Alveare'
ON CONFLICT DO NOTHING;

INSERT INTO board_links (from_board_id, from_x, from_y, to_board_id, to_x, to_y)
SELECT h.id, 2 + k, 0, b.id, 8 + k, 14
FROM boards b, boards h, generate_series(0, 3) AS k
WHERE b.name = 'Bosco' AND h.name = 'Alveare'
ON CONFLICT DO NOTHING;

-- Abitanti e oggetti delle board nuove.
INSERT INTO units (board_id, kind, name, description, x, y, speed, health, max_health, vision, dialogue)
SELECT b.id, 'npc', 'Boscaiolo', 'Taglia legna dall''alba. Dice che il bosco "ricresce", ma nessuno l''ha mai visto.', 10, 8, 0, 100, 100, 3,
$$Il bosco è più grande di come sembra. Soprattutto quando ci si perde.
Sei venuto dalla Piazza? Lì sono tutti molto rumorosi.
A nord c'è un alveare. Non fare rumore, e non chiedere del miele.$$
FROM boards b
WHERE b.name = 'Bosco' AND NOT EXISTS (SELECT 1 FROM units WHERE name = 'Boscaiolo');

INSERT INTO units (board_id, kind, name, description, x, y, speed, health, max_health, vision, dialogue)
SELECT b.id, 'npc', 'Apicoltore', 'Ha uno sguardo stanco e un cappello con la rete. Le api non sembrano trattarlo meglio del resto del mondo.', 6, 6, 0, 100, 100, 3,
$$Le celle dell'alveare sono esagonali. Un caso? Non credo.
Ogni ape sa esattamente cosa fare. Invidio le api.
Il miele è ovunque, e il conto arriva sempre dopo.$$
FROM boards b
WHERE b.name = 'Alveare' AND NOT EXISTS (SELECT 1 FROM units WHERE name = 'Apicoltore');

INSERT INTO board_items (board_id, name, description, x, y)
SELECT b.id, v.name, v.description, v.x, v.y
FROM boards b,
     (VALUES
        ('Bosco', 'Fungo', 'Ha un aspetto sospetto. Meglio non cucinarlo.', 5, 4),
        ('Bosco', 'Ascia', 'Un''ascia ben affilata. Nessun boscaiolo la sta cercando, a quanto pare.', 14, 10),
        ('Alveare', 'Miele', 'Denso e dorato. Le api lo rivogliono indietro.', 8, 4)
     ) AS v(board, name, description, x, y)
WHERE b.name = v.board AND NOT EXISTS (SELECT 1 FROM board_items i WHERE i.name = v.name);
