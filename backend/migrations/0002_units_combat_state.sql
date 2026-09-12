-- M2: adds durable combat/respawn state to units, plus max_health (health can
-- now drop below max from attacks and needs a ceiling to respawn back to),
-- so that "dead, waiting on cooldown" survives a server restart instead of
-- only living in the board goroutine's memory.

ALTER TABLE units
    ADD COLUMN IF NOT EXISTS max_health real NOT NULL DEFAULT 100,
    ADD COLUMN IF NOT EXISTS alive boolean NOT NULL DEFAULT true,
    ADD COLUMN IF NOT EXISTS respawn_at timestamptz;

UPDATE units SET max_health = health WHERE max_health IS DISTINCT FROM health;
