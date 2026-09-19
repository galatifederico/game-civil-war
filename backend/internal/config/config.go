package config

import (
	"errors"
	"os"
)

type Config struct {
	Port        string
	DatabaseURL string
	JWTSecret   string
	AdminWebDir string // folder of the admin web app; empty = not served
}

func Load() (Config, error) {
	cfg := Config{
		Port:        getenv("PORT", "8090"),
		DatabaseURL: os.Getenv("DATABASE_URL"),
		JWTSecret:   os.Getenv("JWT_SECRET"),
		AdminWebDir: os.Getenv("ADMIN_WEB_DIR"),
	}
	if cfg.DatabaseURL == "" {
		return cfg, errors.New("DATABASE_URL is required")
	}
	if len(cfg.JWTSecret) < 16 {
		return cfg, errors.New("JWT_SECRET is required (at least 16 characters)")
	}
	return cfg, nil
}

func getenv(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}
