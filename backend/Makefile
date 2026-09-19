# Go non e' installato sull'host: ogni comando gira nel container golang.
# --user evita file di proprieta' root; la cache di build sta in .cache (ignorata da git).
GO_IMAGE ?= golang:1.26-alpine
DOCKER_GO = docker run --rm --user $(shell id -u):$(shell id -g) \
	-e HOME=/tmp -e GOCACHE=/cache/build -e GOMODCACHE=/cache/mod -e GOTOOLCHAIN=auto \
	-v $(CURDIR):/src -v $(CURDIR)/.cache:/cache -w /src

COMPOSE = docker compose -f ../deploy/docker-compose.yml

.PHONY: up db build vet test test-integration tidy run

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

# Serve Postgres attivo (deploy/docker-compose.yml). Legge le variabili da ../deploy/.env.
run:
	@mkdir -p .cache
	docker run --rm --network host --env-file ../deploy/.env --user $(shell id -u):$(shell id -g) \
		-e HOME=/tmp -e GOCACHE=/cache/build -e GOMODCACHE=/cache/mod -e GOTOOLCHAIN=auto \
		-e ADMIN_WEB_DIR=/admin-web -v $(CURDIR)/../admin-web:/admin-web:ro \
		-v $(CURDIR):/src -v $(CURDIR)/.cache:/cache -w /src $(GO_IMAGE) go run ./cmd/server
