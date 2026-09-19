# Immagine del backend (per il deploy: vedi docs/deploy.md). Il contesto di build e' la radice del
# repository, perche' l'app admin (admin-web/) viene servita dallo stesso processo.
FROM golang:1.26-alpine AS build
WORKDIR /src
COPY backend/go.mod backend/go.sum ./
RUN go mod download
COPY backend/ ./
RUN CGO_ENABLED=0 go build -trimpath -ldflags="-s -w" -o /out/server ./cmd/server

FROM alpine:3.21
RUN adduser -D -u 10001 thegame
COPY --from=build /out/server /usr/local/bin/server
COPY admin-web /admin-web
ENV ADMIN_WEB_DIR=/admin-web PORT=8090
USER thegame
EXPOSE 8090
HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://127.0.0.1:${PORT}/healthz || exit 1
CMD ["server"]
