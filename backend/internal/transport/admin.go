package transport

import (
	"context"
	"encoding/json"
	"errors"
	"io"
	"net/http"
	"os"

	"thegame/backend/internal/game"
	"thegame/backend/internal/store"
)

// The admin API edits the world its owner administers (worlds.owner_id). The admin web app
// (admin-web/) is a static page that talks to it; the backend serves that page under /admin/.

const adminBodyLimit = 1 << 20

func (s *Server) adminRoutes(mux *http.ServeMux) {
	if s.AdminDir != "" {
		if _, err := os.Stat(s.AdminDir); err == nil {
			mux.Handle("GET /admin/", http.StripPrefix("/admin/", http.FileServer(http.Dir(s.AdminDir))))
		}
	}
	mux.HandleFunc("GET /admin/api/worlds/{id}", s.adminWorld(s.adminGet))
	mux.HandleFunc("PUT /admin/api/worlds/{id}/meta", s.adminWorld(s.adminMeta))
	mux.HandleFunc("PUT /admin/api/worlds/{id}/rules", s.adminWorld(s.adminRules))
	mux.HandleFunc("PUT /admin/api/worlds/{id}/boards", s.adminWorld(adminList(s, store.BoardsChange)))
	mux.HandleFunc("PUT /admin/api/worlds/{id}/links", s.adminWorld(adminList(s, store.LinksChange)))
	mux.HandleFunc("PUT /admin/api/worlds/{id}/races", s.adminWorld(adminList(s, store.RacesChange)))
	mux.HandleFunc("PUT /admin/api/worlds/{id}/compat", s.adminWorld(adminList(s, store.CompatChange)))
	mux.HandleFunc("PUT /admin/api/worlds/{id}/goals", s.adminWorld(adminList(s, store.GoalsChange)))
	mux.HandleFunc("PUT /admin/api/worlds/{id}/npcs", s.adminWorld(adminList(s, store.NPCsChange)))
	mux.HandleFunc("PUT /admin/api/worlds/{id}/items", s.adminWorld(adminList(s, store.ItemsChange)))
	mux.HandleFunc("POST /admin/api/worlds/{id}/assign", s.adminWorld(s.adminAssign))
}

// adminWorld lets through only the world's owner.
func (s *Server) adminWorld(next func(http.ResponseWriter, *http.Request, string)) http.HandlerFunc {
	return s.authed(func(w http.ResponseWriter, r *http.Request, p store.Player) {
		id := r.PathValue("id")
		info, err := s.Store.World(r.Context(), id)
		if errors.Is(err, store.ErrNotFound) {
			writeError(w, http.StatusNotFound, "mondo non trovato")
			return
		}
		if err != nil {
			internalError(w, err)
			return
		}
		if info.OwnerID != p.ID {
			writeError(w, http.StatusForbidden, "non sei l'admin di questo mondo")
			return
		}
		next(w, r, id)
	})
}

// adminWorldView is what the admin app loads: the world as edited, the rules with every default
// filled in (to show what can be set) and the menu values the app offers.
type adminWorldView struct {
	store.WorldDefinition
	DefaultRules   game.Rules `json:"default_rules"`
	EffectiveRules game.Rules `json:"effective_rules"`
	GoalKinds      []string   `json:"goal_kinds"`
}

func (s *Server) adminGet(w http.ResponseWriter, r *http.Request, id string) {
	def, err := s.Store.Definition(r.Context(), id)
	if err != nil {
		internalError(w, err)
		return
	}
	eff, err := game.ParseRules(def.Rules)
	if err != nil {
		internalError(w, err)
		return
	}
	view := adminWorldView{WorldDefinition: def, DefaultRules: game.DefaultRules(), EffectiveRules: eff}
	for _, k := range game.GoalKinds() {
		view.GoalKinds = append(view.GoalKinds, string(k))
	}
	writeJSON(w, http.StatusOK, view)
}

func (s *Server) adminMeta(w http.ResponseWriter, r *http.Request, id string) {
	var in struct {
		Name        string `json:"name"`
		Description string `json:"description"`
	}
	if !decodeAdminBody(w, r, &in) {
		return
	}
	s.applyChange(w, r, id, store.MetaChange(id, in.Name, in.Description))
}

func (s *Server) adminRules(w http.ResponseWriter, r *http.Request, id string) {
	raw, err := io.ReadAll(http.MaxBytesReader(w, r.Body, adminBodyLimit))
	if err != nil {
		writeError(w, http.StatusBadRequest, "richiesta non valida")
		return
	}
	s.applyChange(w, r, id, store.RulesChange(id, raw))
}

// adminList makes the handler that saves one list (boards, races...): the body is {"items": [...]}.
func adminList[T any](s *Server, change func(worldID string, items []T) store.Change) func(http.ResponseWriter, *http.Request, string) {
	return func(w http.ResponseWriter, r *http.Request, id string) {
		var in struct {
			Items []T `json:"items"`
		}
		if !decodeAdminBody(w, r, &in) {
			return
		}
		s.applyChange(w, r, id, change(id, in.Items))
	}
}

// applyChange checks a change against the database without touching the world, and only if it is
// acceptable stops the world, applies it and starts the world again.
func (s *Server) applyChange(w http.ResponseWriter, r *http.Request, id string, change store.Change) {
	if err := s.Store.Apply(r.Context(), change, true); err != nil {
		s.changeError(w, err)
		return
	}
	err := s.Hub.Edit(r.Context(), id, func(ctx context.Context) error { return s.Store.Apply(ctx, change, false) })
	if err != nil {
		s.changeError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

func (s *Server) changeError(w http.ResponseWriter, err error) {
	var bad *store.InvalidError
	if errors.As(err, &bad) {
		writeError(w, http.StatusBadRequest, bad.Message)
		return
	}
	internalError(w, err)
}

func (s *Server) adminAssign(w http.ResponseWriter, r *http.Request, id string) {
	var in struct {
		PlayerID string `json:"player_id"`
		GoalID   string `json:"goal_id"`
	}
	if !decodeAdminBody(w, r, &in) {
		return
	}
	loop, ok := s.Hub.Loop(id)
	if !ok {
		writeError(w, http.StatusServiceUnavailable, "il mondo non è disponibile in questo momento")
		return
	}
	if err := loop.AssignGoal(in.PlayerID, in.GoalID); err != nil {
		if rule, isRule := err.(*game.Error); isRule {
			writeError(w, http.StatusBadRequest, rule.Error())
			return
		}
		internalError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

func decodeAdminBody(w http.ResponseWriter, r *http.Request, v any) bool {
	r.Body = http.MaxBytesReader(w, r.Body, adminBodyLimit)
	if err := json.NewDecoder(r.Body).Decode(v); err != nil {
		writeError(w, http.StatusBadRequest, "richiesta non valida")
		return false
	}
	return true
}
