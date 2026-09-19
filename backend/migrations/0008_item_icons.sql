-- Icona degli oggetti (chiave scelta dall'admin tra quelle note al client: vedi game.ItemIcons).
-- Vuota = automatica, ricavata dall'effetto. Idempotente come le altre.

ALTER TABLE board_items ADD COLUMN IF NOT EXISTS icon TEXT NOT NULL DEFAULT '';
ALTER TABLE inventory_items ADD COLUMN IF NOT EXISTS icon TEXT NOT NULL DEFAULT '';

-- Gli oggetti del mondo di prova.
UPDATE board_items SET icon = 'chest' WHERE icon = '' AND name = 'Forziere';
UPDATE board_items SET icon = 'gem'   WHERE icon = '' AND name = 'Cristallo';
UPDATE board_items SET icon = 'sign'  WHERE icon = '' AND name = 'Cartello';
UPDATE inventory_items SET icon = 'chest' WHERE icon = '' AND name = 'Forziere';
UPDATE inventory_items SET icon = 'gem'   WHERE icon = '' AND name = 'Cristallo';
UPDATE inventory_items SET icon = 'sign'  WHERE icon = '' AND name = 'Cartello';
