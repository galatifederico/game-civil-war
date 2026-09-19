-- M2: forza e dialoghi delle pedine, punti squadra, strutture (territorio) e inventario condiviso.
-- Idempotente come la 0001.

ALTER TABLE units ADD COLUMN IF NOT EXISTS strength INT NOT NULL DEFAULT 10;
ALTER TABLE units ADD COLUMN IF NOT EXISTS dialogue TEXT NOT NULL DEFAULT '';
ALTER TABLE players ADD COLUMN IF NOT EXISTS points INT NOT NULL DEFAULT 0;
-- Una casella diventa territorio di una squadra quando ci si costruisce una struttura.
CREATE TABLE IF NOT EXISTS structures (
    id          UUID PRIMARY KEY,
    board_id    UUID NOT NULL REFERENCES boards(id),
    player_id   UUID NOT NULL REFERENCES players(id),
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    x           INT NOT NULL,
    y           INT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (board_id, x, y)
);

-- Inventario condiviso della squadra (1 giocatore = 1 squadra).
CREATE TABLE IF NOT EXISTS inventory_items (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player_id   UUID NOT NULL REFERENCES players(id),
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    acquired_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS inventory_items_player_idx ON inventory_items (player_id);

-- Una riga di dialogo per linea: l'NPC ne sceglie una a caso ogni volta che gli si parla.
UPDATE units SET dialogue = $$Ho tutto, tranne quello che ti serve.
Prezzi imbattibili, qualità discutibile.
Non accetto reclami. Né resi. Né domande.$$
WHERE kind = 'npc' AND name = 'Mercante' AND dialogue = '';

UPDATE units SET dialogue = $$Circolare, non c'è niente da vedere. E se vedi qualcosa, non l'hai visto.
Il mio turno finisce tra sei ore. Le ultime cinque le passo così.
Non è il mio reparto.$$
WHERE kind = 'npc' AND name = 'Guardia' AND dialogue = '';

UPDATE units SET dialogue = $$Chi ha fretta arriva tardi. Chi non ne ha arriva lo stesso, ma con più stile.
Un tempo tutto questo era campagna. Poi hanno costruito il resto.
La risposta è dentro di te. Purtroppo la domanda no.$$
WHERE kind = 'npc' AND name = 'Vecchio saggio' AND dialogue = '';

UPDATE units SET dialogue = $$Il martello è quasi pronto. Torna domani.
Una buona lama si fa con pazienza. La mia è ancora in garanzia.
Se si rompe, non l'hai avuta da me.$$
WHERE kind = 'npc' AND name = 'Fabbro' AND dialogue = '';

UPDATE units SET dialogue = $$Vengo da lontano. Non ricordo da dove.
Dicono che chi costruisce un avamposto si prende la piazza. Io preferisco camminare.
Gli oggetti per terra non sono mai solo oggetti per terra.$$
WHERE kind = 'npc' AND name = 'Viandante' AND dialogue = '';
