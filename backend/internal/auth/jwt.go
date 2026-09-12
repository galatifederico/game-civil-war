package auth

import (
	"errors"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

var ErrInvalidToken = errors.New("invalid or expired token")

type TokenIssuer struct {
	secret []byte
	ttl    time.Duration
}

func NewTokenIssuer(secret string) *TokenIssuer {
	return &TokenIssuer{secret: []byte(secret), ttl: 7 * 24 * time.Hour}
}

type claims struct {
	PlayerID string `json:"player_id"`
	jwt.RegisteredClaims
}

func (t *TokenIssuer) Issue(playerID string) (string, error) {
	c := claims{
		PlayerID: playerID,
		RegisteredClaims: jwt.RegisteredClaims{
			ExpiresAt: jwt.NewNumericDate(time.Now().Add(t.ttl)),
			IssuedAt:  jwt.NewNumericDate(time.Now()),
		},
	}
	token := jwt.NewWithClaims(jwt.SigningMethodHS256, c)
	return token.SignedString(t.secret)
}

// Verify returns the player ID embedded in a valid token.
func (t *TokenIssuer) Verify(tokenString string) (string, error) {
	parsed, err := jwt.ParseWithClaims(tokenString, &claims{}, func(tok *jwt.Token) (any, error) {
		if _, ok := tok.Method.(*jwt.SigningMethodHMAC); !ok {
			return nil, ErrInvalidToken
		}
		return t.secret, nil
	})
	if err != nil || !parsed.Valid {
		return "", ErrInvalidToken
	}
	c, ok := parsed.Claims.(*claims)
	if !ok || c.PlayerID == "" {
		return "", ErrInvalidToken
	}
	return c.PlayerID, nil
}
