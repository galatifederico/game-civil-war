// Package rest exposes the HTTP API: authentication, the world join lobby,
// and (from M6) the admin CRUD surface consumed by admin-web. Realtime game
// state goes over WebSocket instead (see internal/transport/ws).
package rest

import (
	"net/http"

	"github.com/go-chi/chi/v5"

	"thegame/internal/auth"
	"thegame/internal/store"
)

type Deps struct {
	PlayerRepo *store.PlayerRepo
	WorldRepo  *store.WorldRepo
	Issuer     *auth.TokenIssuer
	WSHandler  http.Handler
}

func NewRouter(deps Deps) http.Handler {
	r := chi.NewRouter()

	r.Get("/healthz", healthzHandler)

	r.Post("/auth/register", registerHandler(deps))
	r.Post("/auth/login", loginHandler(deps))

	r.Group(func(r chi.Router) {
		r.Use(auth.RequireAuth(deps.Issuer))
		r.Get("/worlds", listWorldsHandler(deps))
	})

	r.Handle("/ws", deps.WSHandler)

	return r
}

func healthzHandler(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
	_, _ = w.Write([]byte("ok"))
}
