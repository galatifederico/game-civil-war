# Orchestrates the whole local dev stack: Postgres+Redis (Docker), DB
# migrations, then the backend server in the foreground (Ctrl+C stops it,
# Postgres/Redis keep running — see `make stop-local`).
#
# Go is not installed on this host, so the backend itself runs via
# backend/Makefile, which wraps every Go command in the golang Docker image.

.PHONY: infra-up migrate start-local start\:local stop-local

infra-up:
	cd deploy && docker compose up -d

# Every file in backend/migrations/ is written to be safely re-run (CREATE
# TABLE IF NOT EXISTS, ON CONFLICT DO NOTHING, ...), so just replaying all of
# them in order is enough — no separate migration-tracking table needed yet.
migrate: infra-up
	@echo "waiting for postgres..."
	@until docker exec deploy-postgres-1 pg_isready -U thegame >/dev/null 2>&1; do sleep 1; done
	@for f in backend/migrations/*.sql; do \
		echo "applying $$f"; \
		docker exec -i deploy-postgres-1 psql -U thegame -d thegame < $$f || exit 1; \
	done

start-local: migrate
	$(MAKE) -C backend run

# Alias so `make start:local` also works, npm-script style.
start\:local: start-local

stop-local:
	-pkill -f "go run ./cmd/server"
	cd deploy && docker compose down
