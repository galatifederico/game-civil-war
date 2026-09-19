package transport

import (
	"encoding/json"
	"errors"
	"log"
	"net/http"
	"strings"
	"unicode/utf8"

	"thegame/backend/internal/auth"
	"thegame/backend/internal/game"
	"thegame/backend/internal/hub"
	"thegame/backend/internal/store"
)

type Server struct {
	Store  *store.Store
	Tokens *auth.Tokens
	Hub    *hub.Hub
}

func (s *Server) Router() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /healthz", func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	})
	mux.HandleFunc("POST /auth/register", s.register)
	mux.HandleFunc("POST /auth/login", s.login)
	mux.HandleFunc("GET /worlds", s.authed(s.listWorlds))
	mux.HandleFunc("POST /worlds", s.authed(s.createWorld))
	mux.HandleFunc("POST /worlds/{id}/join", s.authed(s.joinWorld))
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
	// The first player to register administers the world the database was seeded with.
	if err := s.Store.ClaimOwnerlessWorlds(r.Context(), player.ID); err != nil {
		internalError(w, err)
		return
	}
	s.finishAuth(w, player, http.StatusCreated)
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
	s.finishAuth(w, player, http.StatusOK)
}

// finishAuth answers a successful login or registration with a token. Joining a world (and
// getting a team there) is a separate step: the lobby.
func (s *Server) finishAuth(w http.ResponseWriter, p store.Player, status int) {
	token, err := s.Tokens.Issue(p.ID)
	if err != nil {
		internalError(w, err)
		return
	}
	writeJSON(w, status, authResponse{Token: token, PlayerID: p.ID, Username: p.Username})
}

// authed runs a handler for a request that carries a valid "Authorization: Bearer <token>".
func (s *Server) authed(next func(http.ResponseWriter, *http.Request, store.Player)) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		token, ok := strings.CutPrefix(r.Header.Get("Authorization"), "Bearer ")
		if !ok {
			writeError(w, http.StatusUnauthorized, "accesso richiesto")
			return
		}
		playerID, err := s.Tokens.Parse(token)
		if err != nil {
			writeError(w, http.StatusUnauthorized, "sessione non valida o scaduta")
			return
		}
		player, err := s.Store.PlayerByID(r.Context(), playerID)
		if err != nil {
			writeError(w, http.StatusUnauthorized, "sessione non valida o scaduta")
			return
		}
		next(w, r, player)
	}
}

type worldEntry struct {
	ID          string      `json:"id"`
	Name        string      `json:"name"`
	Description string      `json:"description"`
	Boards      int         `json:"boards"`
	Players     int         `json:"players"`
	Joined      bool        `json:"joined"`
	Admin       bool        `json:"admin"`
	Races       []raceEntry `json:"races"`
}

type raceEntry struct {
	ID          string `json:"id"`
	Name        string `json:"name"`
	Description string `json:"description"`
}

func (s *Server) listWorlds(w http.ResponseWriter, r *http.Request, p store.Player) {
	worlds, err := s.Store.ListWorlds(r.Context())
	if err != nil {
		internalError(w, err)
		return
	}
	joined, err := s.Store.Memberships(r.Context(), p.ID)
	if err != nil {
		internalError(w, err)
		return
	}
	out := make([]worldEntry, 0, len(worlds))
	for _, info := range worlds {
		races := make([]raceEntry, 0, len(info.Races))
		for _, r := range info.Races {
			races = append(races, raceEntry{ID: r.ID, Name: r.Name, Description: r.Description})
		}
		out = append(out, worldEntry{
			ID: info.ID, Name: info.Name, Description: info.Description, Boards: info.Boards, Players: info.Players,
			Joined: joined[info.ID], Admin: info.OwnerID == p.ID, Races: races,
		})
	}
	writeJSON(w, http.StatusOK, map[string]any{"worlds": out})
}

// joinWorld enrols the player in a world and, the first time, gives them their team there.
func (s *Server) joinWorld(w http.ResponseWriter, r *http.Request, p store.Player) {
	id := r.PathValue("id")
	loop, ok := s.Hub.Loop(id)
	if !ok {
		writeError(w, http.StatusNotFound, "mondo non trovato")
		return
	}
	var in struct {
		RaceID string `json:"race_id"`
	}
	if r.ContentLength != 0 && !decodeBody(w, r, &in) {
		return
	}
	raceID, err := s.Store.ResolveRace(r.Context(), id, in.RaceID)
	if errors.Is(err, store.ErrNotFound) {
		writeError(w, http.StatusBadRequest, "razza non valida per questo mondo")
		return
	}
	if err != nil {
		internalError(w, err)
		return
	}
	if err := s.Store.Join(r.Context(), id, p.ID, raceID); err != nil {
		internalError(w, err)
		return
	}
	if err := loop.EnsureTeam(r.Context(), p.ID, p.Username, raceID); err != nil {
		if _, isRule := err.(*game.Error); isRule {
			writeError(w, http.StatusServiceUnavailable, err.Error())
			return
		}
		internalError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, map[string]string{"world_id": id})
}

// createWorld makes a new world with the caller as its admin. Its boards and rules are then
// edited from the admin app.
func (s *Server) createWorld(w http.ResponseWriter, r *http.Request, p store.Player) {
	var in struct {
		Name        string `json:"name"`
		Description string `json:"description"`
	}
	if !decodeBody(w, r, &in) {
		return
	}
	in.Name = strings.TrimSpace(in.Name)
	if n := utf8.RuneCountInString(in.Name); n < 2 || n > 40 {
		writeError(w, http.StatusBadRequest, "il nome del mondo deve avere da 2 a 40 caratteri")
		return
	}
	id, err := s.Store.CreateWorld(r.Context(), in.Name, strings.TrimSpace(in.Description), p.ID)
	if err != nil {
		internalError(w, err)
		return
	}
	if err := s.Hub.Load(r.Context(), id); err != nil {
		internalError(w, err)
		return
	}
	writeJSON(w, http.StatusCreated, map[string]string{"id": id})
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
