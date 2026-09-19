package transport

import (
	"context"
	"encoding/json"
	"errors"
	"log"
	"net/http"
	"strings"
	"unicode/utf8"

	"thegame/backend/internal/auth"
	"thegame/backend/internal/game"
	"thegame/backend/internal/store"
)

type Server struct {
	Store  *store.Store
	Tokens *auth.Tokens
	Loop   *game.Loop
}

func (s *Server) Router() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /healthz", func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	})
	mux.HandleFunc("POST /auth/register", s.register)
	mux.HandleFunc("POST /auth/login", s.login)
	mux.HandleFunc("GET /ws", s.serveWS)
	return mux
}

type credentials struct {
	Email    string `json:"email"`
	Username string `json:"username"`
	Password string `json:"password"`
}

type authResponse struct {
	Token    string `json:"token"`
	PlayerID string `json:"player_id"`
	Username string `json:"username"`
}

func (s *Server) register(w http.ResponseWriter, r *http.Request) {
	var in credentials
	if !decodeBody(w, r, &in) {
		return
	}
	in.Email = strings.ToLower(strings.TrimSpace(in.Email))
	in.Username = strings.TrimSpace(in.Username)
	switch {
	case len(in.Email) > 254 || !strings.Contains(in.Email, "@"):
		writeError(w, http.StatusBadRequest, "email non valida")
		return
	case utf8.RuneCountInString(in.Username) < 2 || utf8.RuneCountInString(in.Username) > 24:
		writeError(w, http.StatusBadRequest, "il nome deve avere da 2 a 24 caratteri")
		return
	case len(in.Password) < 8 || len(in.Password) > 72:
		writeError(w, http.StatusBadRequest, "la password deve avere da 8 a 72 caratteri")
		return
	}

	hash, err := auth.HashPassword(in.Password)
	if err != nil {
		internalError(w, err)
		return
	}
	player, err := s.Store.CreatePlayer(r.Context(), in.Email, in.Username, hash)
	if errors.Is(err, store.ErrEmailTaken) {
		writeError(w, http.StatusConflict, "email già registrata")
		return
	}
	if err != nil {
		internalError(w, err)
		return
	}
	s.finishAuth(w, r.Context(), player, http.StatusCreated)
}

// dummyHash keeps login timing similar whether or not the email exists.
var dummyHash, _ = auth.HashPassword("dummy-password")

func (s *Server) login(w http.ResponseWriter, r *http.Request) {
	var in credentials
	if !decodeBody(w, r, &in) {
		return
	}
	player, err := s.Store.PlayerByEmail(r.Context(), strings.ToLower(strings.TrimSpace(in.Email)))
	if errors.Is(err, store.ErrNotFound) {
		auth.CheckPassword(dummyHash, in.Password)
		writeError(w, http.StatusUnauthorized, "credenziali errate")
		return
	}
	if err != nil {
		internalError(w, err)
		return
	}
	if !auth.CheckPassword(player.PasswordHash, in.Password) {
		writeError(w, http.StatusUnauthorized, "credenziali errate")
		return
	}
	s.finishAuth(w, r.Context(), player, http.StatusOK)
}

func (s *Server) finishAuth(w http.ResponseWriter, ctx context.Context, p store.Player, status int) {
	if err := s.Loop.EnsureTeam(ctx, p.ID, p.Username); err != nil {
		if _, isRule := err.(*game.Error); isRule {
			writeError(w, http.StatusServiceUnavailable, err.Error())
			return
		}
		internalError(w, err)
		return
	}
	token, err := s.Tokens.Issue(p.ID)
	if err != nil {
		internalError(w, err)
		return
	}
	writeJSON(w, status, authResponse{Token: token, PlayerID: p.ID, Username: p.Username})
}

func decodeBody(w http.ResponseWriter, r *http.Request, v any) bool {
	r.Body = http.MaxBytesReader(w, r.Body, 4096)
	if err := json.NewDecoder(r.Body).Decode(v); err != nil {
		writeError(w, http.StatusBadRequest, "richiesta non valida")
		return false
	}
	return true
}

func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	if err := json.NewEncoder(w).Encode(v); err != nil {
		log.Printf("write response: %v", err)
	}
}

func writeError(w http.ResponseWriter, status int, message string) {
	writeJSON(w, status, map[string]string{"error": message})
}

func internalError(w http.ResponseWriter, err error) {
	log.Printf("internal error: %v", err)
	writeError(w, http.StatusInternalServerError, "errore interno")
}
