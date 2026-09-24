-- Limiti delle caratteristiche per razza: per ogni caratteristica, il minimo e/o il massimo che la
-- razza cambia rispetto ai valori del mondo (game.Rules.Characteristics). Vuoto = nessuna modifica.
-- Idempotente come le altre.

ALTER TABLE races ADD COLUMN IF NOT EXISTS bounds JSONB NOT NULL DEFAULT '{}'::jsonb;
