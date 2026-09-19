-- Le regole di un mondo sono dati (vedi tecnico.md): qui solo le differenze rispetto ai valori
-- predefiniti del backend (game.DefaultRules). '{}' = tutto predefinito.
-- Esempio: UPDATE worlds SET rules = '{"minors_per_team": 8, "champion": {"speed": 4}}';
ALTER TABLE worlds ADD COLUMN IF NOT EXISTS rules JSONB NOT NULL DEFAULT '{}'::jsonb;
