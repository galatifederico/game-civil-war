package main

import (
	"context"
	"errors"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"thegame/backend/internal/auth"
	"thegame/backend/internal/config"
	"thegame/backend/internal/game"
	"thegame/backend/internal/store"
	"thegame/backend/internal/transport"
)

func main() {
	if err := run(); err != nil {
		log.Fatal(err)
	}
}

func run() error {
	cfg, err := config.Load()
	if err != nil {
		return err
	}
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	st, err := store.Connect(ctx, cfg.DatabaseURL)
	if err != nil {
		return err
	}
	defer st.Close()
	if err := st.Migrate(ctx); err != nil {
		return err
	}
	world, err := st.LoadWorld(ctx)
	if err != nil {
		return err
	}
	log.Printf("world %q loaded with %d boards", world.Name, len(world.Boards()))

	loop := game.NewLoop(world, st)
	go loop.Run(ctx)

	srv := &http.Server{
		Addr:              ":" + cfg.Port,
		Handler:           (&transport.Server{Store: st, Tokens: auth.NewTokens(cfg.JWTSecret, 30*24*time.Hour), Loop: loop}).Router(),
		ReadHeaderTimeout: 10 * time.Second,
	}
	go func() {
		<-ctx.Done()
		shutdownCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		srv.Shutdown(shutdownCtx)
	}()

	log.Printf("listening on %s", srv.Addr)
	if err := srv.ListenAndServe(); !errors.Is(err, http.ErrServerClosed) {
		return err
	}
	return nil
}
