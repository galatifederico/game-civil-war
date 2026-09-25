-- Le classi: come le razze, ma la classe di una pedina puo' cambiare nel tempo (la razza no) e una
-- classe non ha valori iniziali propri: cambia solo i limiti delle caratteristiche (si sommano a
-- quelli della razza: vince sempre il limite maggiore, vedi game.World.BoundsFor). Una pedina
-- comincia senza classe (class_id nullo). Idempotente come le altre.

CREATE TABLE IF NOT EXISTS classes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    world_id    UUID NOT NULL REFERENCES worlds(id),
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    position    INT NOT NULL DEFAULT 0,
    bounds      JSONB NOT NULL DEFAULT '{}'::jsonb,
    look        TEXT NOT NULL DEFAULT '', -- il colore della pedina quando si indossa questa classe; vuoto = quello della razza
    UNIQUE (world_id, name)
);

ALTER TABLE units ADD COLUMN IF NOT EXISTS class_id UUID REFERENCES classes(id);
