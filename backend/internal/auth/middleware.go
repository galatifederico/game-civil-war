package auth

import (
	"context"
	"net/http"
	"strings"
)

type ctxKey string

const playerIDKey ctxKey = "player_id"

// RequireAuth validates the Bearer token and stashes the player ID in the
// request context for downstream handlers.
func RequireAuth(issuer *TokenIssuer) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			header := r.Header.Get("Authorization")
			token, ok := strings.CutPrefix(header, "Bearer ")
			if !ok || token == "" {
				http.Error(w, "missing bearer token", http.StatusUnauthorized)
				return
			}
			playerID, err := issuer.Verify(token)
			if err != nil {
				http.Error(w, "invalid token", http.StatusUnauthorized)
				return
			}
			ctx := context.WithValue(r.Context(), playerIDKey, playerID)
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}

func PlayerIDFromContext(ctx context.Context) (string, bool) {
	v, ok := ctx.Value(playerIDKey).(string)
	return v, ok
}
