-- Aspetto delle razze: il colore della pedina standard (vedi game.Looks). Ogni razza e' la stessa
-- pedina con una tavolozza diversa; il ruolo (campione, pedina) aggiunge il resto.
-- Vuoto = predefinito (salmone). Idempotente come le altre.

ALTER TABLE races ADD COLUMN IF NOT EXISTS look TEXT NOT NULL DEFAULT '';

-- Le razze del mondo di prova.
UPDATE races SET look = 'salmon' WHERE look = '' AND name = 'Balordi';
UPDATE races SET look = 'azure'  WHERE look = '' AND name = 'Fighetti';
UPDATE races SET look = 'moss'   WHERE look = '' AND name = 'Sbandati';
