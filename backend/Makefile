# Go non e' installato sull'host: ogni comando gira nel container golang.
# --user evita file di proprieta' root; la cache di build sta in .cache (ignorata da git).
GO_IMAGE ?= golang:1.26-alpine
DOCKER_GO = docker run --rm --user $(shell id -u):$(shell id -g) \
	-e HOME=/tmp -e GOCACHE=/cache/build -e GOMODCACHE=/cache/mod -e GOTOOLCHAIN=auto \
	-v $(CURDIR):/src -v $(CURDIR)/.cache:/cache -w /src

.PHONY: build vet test tidy run

build:
	@mkdir -p .cache bin
	$(DOCKER_GO) $(GO_IMAGE) go build -o bin/server ./cmd/server

vet:
	@mkdir -p .cache
	$(DOCKER_GO) $(GO_IMAGE) go vet ./...

test:
	@mkdir -p .cache
	$(DOCKER_GO) $(GO_IMAGE) go test ./...

tidy:
	@mkdir -p .cache
	$(DOCKER_GO) $(GO_IMAGE) go mod tidy

# Serve Postgres attivo (deploy/docker-compose.yml). Legge le variabili da ../deploy/.env.
run:
	@mkdir -p .cache
	docker run --rm --network host --env-file ../deploy/.env --user $(shell id -u):$(shell id -g) \
		-e HOME=/tmp -e GOCACHE=/cache/build -e GOMODCACHE=/cache/mod -e GOTOOLCHAIN=auto \
		-v $(CURDIR):/src -v $(CURDIR)/.cache:/cache -w /src $(GO_IMAGE) go run ./cmd/server
