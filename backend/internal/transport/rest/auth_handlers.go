package rest

import (
	"errors"
	"net/http"
	"strings"

	"github.com/google/uuid"

	"thegame/internal/auth"
	"thegame/internal/store"
)

type credentialsPayload struct {
	Email    string `json:"email"`
	Password string `json:"password"`
}

type tokenResponse struct {
	Token string `json:"token"`
}

func registerHandler(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var payload credentialsPayload
		if err := readJSON(r, &payload); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}
		payload.Email = strings.ToLower(strings.TrimSpace(payload.Email))

		if !strings.Contains(payload.Email, "@") {
			writeError(w, http.StatusBadRequest, "invalid email")
			return
		}
		if len(payload.Password) < 8 {
			writeError(w, http.StatusBadRequest, "password must be at least 8 characters")
			return
		}

		hash, err := auth.HashPassword(payload.Password)
		if err != nil {
			writeError(w, http.StatusInternalServerError, "could not process password")
			return
		}

		player, err := deps.PlayerRepo.Create(r.Context(), uuid.NewString(), payload.Email, hash)
		if errors.Is(err, store.ErrAlreadyExists) {
			writeError(w, http.StatusConflict, "an account with this email already exists")
			return
		}
		if err != nil {
			writeError(w, http.StatusInternalServerError, "could not create account")
			return
		}

		token, err := deps.Issuer.Issue(player.ID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, "could not issue token")
			return
		}
		writeJSON(w, http.StatusCreated, tokenResponse{Token: token})
	}
}

func loginHandler(deps Deps) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var payload credentialsPayload
		if err := readJSON(r, &payload); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}
		payload.Email = strings.ToLower(strings.TrimSpace(payload.Email))

		player, err := deps.PlayerRepo.FindByEmail(r.Context(), payload.Email)
		if errors.Is(err, store.ErrNotFound) {
			writeError(w, http.StatusUnauthorized, "invalid email or password")
			return
		}
		if err != nil {
			writeError(w, http.StatusInternalServerError, "could not look up account")
			return
		}
		if !auth.CheckPassword(player.PasswordHash, payload.Password) {
			writeError(w, http.StatusUnauthorized, "invalid email or password")
			return
		}

		token, err := deps.Issuer.Issue(player.ID)
		if err != nil {
			writeError(w, http.StatusInternalServerError, "could not issue token")
			return
		}
		writeJSON(w, http.StatusOK, tokenResponse{Token: token})
	}
}
