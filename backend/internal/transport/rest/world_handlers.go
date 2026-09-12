package rest

import "net/http"

type worldSummary struct {
	ID   string `json:"id"`
	Name string `json:"name"`
}

// listWorldsHandler backs the join lobby (docs/design.md: players pick a
// world from a list, no invite codes or manual server addresses).
func listWorldsHandler(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		worlds, err := deps.WorldRepo.ListWorlds(r.Context())
		if err != nil {
			writeError(w, http.StatusInternalServerError, "could not list worlds")
			return
		}
		out := make([]worldSummary, 0, len(worlds))
		for _, wd := range worlds {
			out = append(out, worldSummary{ID: wd.ID, Name: wd.Name})
		}
		writeJSON(w, http.StatusOK, out)
	}
}
