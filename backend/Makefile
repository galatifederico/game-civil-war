# Go non e' installato sull'host: ogni comando gira nel container golang.
# --user evita file di proprieta' root; la cache di build sta in .cache (ignorata da git).
GO_IMAGE ?= golang:1.26-alpine
DOCKER_GO = docker run --rm --user $(shell id -u):$(shell id -g) \
	-e HOME=/tmp -e GOCACHE=/cache/build -e GOMODCACHE=/cache/mod -e GOTOOLCHAIN=auto \
	-v $(CURDIR):/src -v $(CURDIR)/.cache:/cache -w /src

COMPOSE = docker compose -f ../deploy/docker-compose.yml

.PHONY: up db build vet test test-integration tidy image run stop restart logs wait-healthy

# Avvia Postgres e Redis (idempotente).
up:
	$(COMPOSE) up -d

# Crea il database `the_game` se non esiste (idempotente); le tabelle le crea il backend all'avvio.
db:
	@$(COMPOSE) exec -T postgres psql -U thegame -d postgres -tc "SELECT 1 FROM pg_database WHERE datname = 'the_game'" | grep -q 1 \
		|| $(COMPOSE) exec -T postgres psql -U thegame -d postgres -c "CREATE DATABASE the_game"

build:
	@mkdir -p .cache bin
	$(DOCKER_GO) $(GO_IMAGE) go build -o bin/server ./cmd/server

vet:
	@mkdir -p .cache
	$(DOCKER_GO) $(GO_IMAGE) go vet ./...

test:
	@mkdir -p .cache
	$(DOCKER_GO) $(GO_IMAGE) go test ./...

# Test di integrazione su un Postgres vero (ogni esecuzione usa un database temporaneo). Serve `make up`.
test-integration:
	@mkdir -p .cache
	docker run --rm --network host --user $(shell id -u):$(shell id -g) \
		-e HOME=/tmp -e GOCACHE=/cache/build -e GOMODCACHE=/cache/mod -e GOTOOLCHAIN=auto \
		-e TEST_DB_ADMIN_URL='postgres://thegame:thegame@localhost:5433/postgres?sslmode=disable' \
		-v $(CURDIR):/src -v $(CURDIR)/.cache:/cache -w /src $(GO_IMAGE) go test ./internal/transport/ -count=1 -v

tidy:
	@mkdir -p .cache
	$(DOCKER_GO) $(GO_IMAGE) go mod tidy

# --- Il server come container in background ---------------------------------------------------
# L'immagine si chiama $(IMAGE) (backend/Dockerfile: contiene anche l'app admin) e il container
# $(CONTAINER). Serve Postgres attivo (make up db); l'ambiente viene da ../deploy/.env.
IMAGE ?= thegame-backend:dev
CONTAINER ?= thegame-server
PORT ?= $(or $(shell sed -n 's/^PORT=//p' ../deploy/.env 2>/dev/null),8090)

# Costruisce l'immagine (la cache dei livelli rende veloci i rebuild).
image:
	docker build -q -f $(CURDIR)/Dockerfile -t $(IMAGE) .. >/dev/null
	@echo "Immagine pronta: $(IMAGE)"

# Costruisce l'immagine, avvia il server in background e aspetta che risponda.
run: image
	@if [ -n "$$(docker ps -q --filter name='^$(CONTAINER)$$')" ]; then \
		echo "Il server '$(CONTAINER)' e' gia' in esecuzione: usa 'make restart' per riavviarlo o 'make stop' per fermarlo."; exit 1; fi
	@docker rm -f $(CONTAINER) >/dev/null 2>&1 || true
	docker run -d --name $(CONTAINER) --network host --env-file ../deploy/.env -e PORT=$(PORT) $(IMAGE) >/dev/null
	@$(MAKE) --no-print-directory wait-healthy

# Aspetta (fino a 60 s) che il container sia vivo e che /healthz risponda; altrimenti mostra i log.
wait-healthy:
	@echo "Verifico che il server risponda su http://localhost:$(PORT) ..."; \
	for i in $$(seq 1 60); do \
		if [ "$$(docker inspect -f '{{.State.Running}}' $(CONTAINER) 2>/dev/null)" != "true" ]; then \
			echo "ERRORE: il container si e' fermato. Ultimi log:"; docker logs --tail 20 $(CONTAINER); exit 1; fi; \
		if curl -sf --max-time 2 http://localhost:$(PORT)/healthz >/dev/null; then \
			echo "OK: '$(CONTAINER)' e' attivo su http://localhost:$(PORT) (console admin: /admin/)"; exit 0; fi; \
		sleep 1; \
	done; \
	echo "ERRORE: il server non risponde dopo 60 secondi. Ultimi log:"; docker logs --tail 20 $(CONTAINER); exit 1

# Ferma il server e verifica che la porta sia libera. Ferma anche i vecchi container senza nome
# ("go run ./cmd/server") lasciati da versioni precedenti di 'make run'. docker stop aspetta la
# chiusura ordinata (il server scrive su database le ultime modifiche prima di uscire).
stop:
	@ids="$$( (docker ps -q --filter name='^$(CONTAINER)$$'; docker ps --filter ancestor=$(GO_IMAGE) --format '{{.ID}} {{.Command}}' | grep 'cmd/server' | cut -d' ' -f1) | sort -u )"; \
	if [ -n "$$ids" ]; then echo "Fermo il server..."; docker stop -t 20 $$ids >/dev/null && echo "Fermato."; else echo "Il server non era in esecuzione."; fi; \
	docker rm -f $(CONTAINER) >/dev/null 2>&1 || true; \
	if curl -sf --max-time 2 http://localhost:$(PORT)/healthz >/dev/null 2>&1; then \
		echo "ATTENZIONE: qualcosa risponde ancora su http://localhost:$(PORT):"; ss -ltnp 2>/dev/null | grep ":$(PORT) "; exit 1; \
	else echo "Verificato: la porta $(PORT) e' libera."; fi

# Ferma il server, ricostruisce l'immagine e lo riavvia, verificando ogni passo.
restart:
	@$(MAKE) --no-print-directory stop
	@$(MAKE) --no-print-directory run

# Segue i log del server (Ctrl+C per uscire: il server resta acceso).
logs:
	docker logs -f --tail 50 $(CONTAINER)
