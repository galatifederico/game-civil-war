# The Game

MMO realtime a griglia, mondo persistente, tono satirico/adulto. Concept e decisioni in [docs/main.md](docs/main.md), [docs/design.md](docs/design.md), [docs/tecnico.md](docs/tecnico.md). Stato di avanzamento e prossimi passi in [docs/roadmap.md](docs/roadmap.md).

## Avvio rapido (sviluppo locale)

Go non è installato sull'host di sviluppo: tutti i comandi passano per Docker (vedi `backend/Makefile`).

```bash
# Postgres + Redis, migrazioni (idempotenti, si possono rilanciare sempre), backend in foreground
make start-local   # oppure: make start:local

# in un altro terminale, il client
godot --path godot-client
```

`make start-local` resta in foreground mostrando i log del server — `Ctrl+C` lo ferma (Postgres/Redis restano su). Per fermare anche quelli: `make stop-local`.

Server su http://localhost:8080 — `/healthz`, `/auth/register`, `/auth/login`, `/ws`. Porta Postgres: **5433** (non 5432, occupata da un altro progetto su questa macchina). Redis: 6379.

## Struttura

- `backend/` — server Go autoritativo (REST + WebSocket + simulazione board)
- `godot-client/` — client Godot 4.x
- `admin-web/` — pannello admin (non ancora iniziato, vedi roadmap M6)
- `deploy/` — docker-compose per Postgres/Redis
