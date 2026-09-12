# Go is not installed on this host; every target runs inside the official
# golang Docker image instead, with cached module/build dirs so repeated
# runs are fast. Once Go is installed natively these targets become
# unnecessary (`go build ./...`, `go run ./cmd/server` work directly).

GO_IMAGE := golang:1.23-alpine
DOCKER_RUN := docker run --rm -v "$(CURDIR)":/app -w /app \
	-v thegame_gomodcache:/go/pkg/mod -v thegame_gobuildcache:/root/.cache/go-build \
	-e GOTOOLCHAIN=auto

.PHONY: build vet tidy run

build:
	$(DOCKER_RUN) $(GO_IMAGE) go build -o bin/server ./cmd/server

vet:
	$(DOCKER_RUN) $(GO_IMAGE) go vet ./...

tidy:
	$(DOCKER_RUN) $(GO_IMAGE) go mod tidy

# Runs the server attached to the same Docker network as deploy/docker-compose.yml
# so it can reach Postgres/Redis by container hostname. Start those first:
#   (cd ../deploy && docker compose up -d)
run:
	$(DOCKER_RUN) --network deploy_default -p 8080:8080 \
		-e DATABASE_URL=postgres://thegame:thegame@postgres:5432/thegame?sslmode=disable \
		-e REDIS_URL=redis://redis:6379/0 \
		-e JWT_SECRET=dev-secret-change-me \
		$(GO_IMAGE) go run ./cmd/server
