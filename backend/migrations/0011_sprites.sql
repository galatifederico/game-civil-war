-- Aspetto degli NPC (grafica dall'alto): la chiave di uno sprite tra quelli noti al client (vedi
-- game.SpriteNames). Vuota = predefinito. Le pedine dei giocatori usano lo sprite del loro ruolo.
-- Idempotente come le altre.

ALTER TABLE units ADD COLUMN IF NOT EXISTS sprite TEXT NOT NULL DEFAULT '';

-- Gli NPC del mondo di prova.
UPDATE units SET sprite = 'merchant'   WHERE kind = 'npc' AND sprite = '' AND name = 'Mercante';
UPDATE units SET sprite = 'guard'      WHERE kind = 'npc' AND sprite = '' AND name = 'Guardia';
UPDATE units SET sprite = 'sage'       WHERE kind = 'npc' AND sprite = '' AND name = 'Vecchio saggio';
UPDATE units SET sprite = 'blacksmith' WHERE kind = 'npc' AND sprite = '' AND name IN ('Fabbro', 'Boscaiolo');
UPDATE units SET sprite = 'wanderer'   WHERE kind = 'npc' AND sprite = '' AND name IN ('Viandante', 'Apicoltore');
