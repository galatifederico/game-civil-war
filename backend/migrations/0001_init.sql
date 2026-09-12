-- M1: minimal schema for the login + single-board + movement vertical slice.
-- Races, rules, objectives, territory, inventory, multi-board arrive in
-- later migrations (M2-M5) once their sim code exists — see docs/roadmap.md.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS players (
    id            uuid PRIMARY KEY,
    email         text NOT NULL UNIQUE,
    password_hash text NOT NULL,
    created_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS worlds (
    id         uuid PRIMARY KEY,
    name       text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS boards (
    id        uuid PRIMARY KEY,
    world_id  uuid NOT NULL REFERENCES worlds(id) ON DELETE CASCADE,
    name      text NOT NULL,
    grid_type text NOT NULL DEFAULT 'square', -- 'square' | 'hex', see internal/world.Grid
    width     int NOT NULL,
    height    int NOT NULL
);

CREATE TABLE IF NOT EXISTS player_world_membership (
    player_id uuid NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    world_id  uuid NOT NULL REFERENCES worlds(id) ON DELETE CASCADE,
    is_admin  boolean NOT NULL DEFAULT false,
    joined_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (player_id, world_id)
);

CREATE TABLE IF NOT EXISTS units (
    id          uuid PRIMARY KEY,
    player_id   uuid NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    board_id    uuid NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    is_champion boolean NOT NULL DEFAULT false,
    x           int NOT NULL,
    y           int NOT NULL,
    speed       real NOT NULL,
    health      real NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_units_board ON units(board_id);
CREATE INDEX IF NOT EXISTS idx_units_player ON units(player_id);

-- Seed a single default world + board so the server has somewhere to run
-- before the M6 admin panel exists to create worlds interactively.
INSERT INTO worlds (id, name) VALUES
    ('00000000-0000-0000-0000-000000000001', 'Mondo di Prova')
ON CONFLICT (id) DO NOTHING;

INSERT INTO boards (id, world_id, name, grid_type, width, height) VALUES
    ('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000001',
     'Piazza Centrale', 'square', 32, 32)
ON CONFLICT (id) DO NOTHING;
