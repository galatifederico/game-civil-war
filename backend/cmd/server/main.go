// Command server is the single authoritative game server binary: REST API
// (auth, world lobby) + WebSocket realtime transport + the per-board
// simulation loops, all in one process (docs/tecnico.md: no microservices).
package main

import (
	"context"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"thegame/internal/auth"
	"thegame/internal/config"
	"thegame/internal/sim"
	"thegame/internal/store"
	"thegame/internal/transport/rest"
	"thegame/internal/transport/ws"
	"thegame/internal/world"
)

func main() {
	cfg := config.Load()

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	pool, err := store.NewPool(ctx, cfg.DatabaseURL)
	if err != nil {
		log.Fatalf("failed to connect to postgres: %v", err)
	}
	defer pool.Close()

	playerRepo := store.NewPlayerRepo(pool)
	worldRepo := store.NewWorldRepo(pool)
	unitRepo := store.NewUnitRepo(pool)

	worlds, err := worldRepo.ListWorlds(ctx)
	if err != nil {
		log.Fatalf("failed to list worlds: %v", err)
	}
	if len(worlds) == 0 {
		log.Fatal("no worlds found — run migrations (they seed the default world/board)")
	}
	defaultWorld := worlds[0]

	defaultBoardRecord, err := worldRepo.FirstBoardOfWorld(ctx, defaultWorld.ID)
	if err != nil {
		log.Fatalf("failed to load default board: %v", err)
	}

	defaultBoard, err := world.NewBoard(
		defaultBoardRecord.ID, defaultBoardRecord.WorldID, defaultBoardRecord.Name,
		world.GridType(defaultBoardRecord.GridType), defaultBoardRecord.Width, defaultBoardRecord.Height,
	)
	if err != nil {
		log.Fatalf("failed to build default board grid: %v", err)
	}

	hub := ws.NewHub()
	boardLoop := sim.NewBoardLoop(defaultBoard, hub, unitRepo)
	hub.RegisterBoard(boardLoop)
	go boardLoop.Run(ctx)

	// Rehydrate any units already persisted on this board (e.g. after a
	// restart) so they show up in the next client's world_snapshot.
	existingUnits, err := unitRepo.ListByBoard(ctx, defaultBoard.ID)
	if err != nil {
		log.Fatalf("failed to load existing units: %v", err)
	}
	for _, u := range existingUnits {
		boardLoop.JoinUnit(sim.FromStoreRecord(u))
	}

	issuer := auth.NewTokenIssuer(cfg.JWTSecret)

	wsHandler := &ws.Handler{
		Hub:            hub,
		Issuer:         issuer,
		UnitRepo:       unitRepo,
		WorldRepo:      worldRepo,
		DefaultWorldID: defaultWorld.ID,
		DefaultBoard:   defaultBoardRecord,
	}

	router := rest.NewRouter(rest.Deps{
		PlayerRepo: playerRepo,
		WorldRepo:  worldRepo,
		Issuer:     issuer,
		WSHandler:  wsHandler,
	})

	srv := &http.Server{
		Addr:              ":" + cfg.Port,
		Handler:           router,
		ReadHeaderTimeout: 5 * time.Second,
	}

	go func() {
		log.Printf("server listening on :%s (world=%s board=%s)", cfg.Port, defaultWorld.Name, defaultBoardRecord.Name)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("server failed: %v", err)
		}
	}()

	<-ctx.Done()
	log.Println("shutting down...")

	shutdownCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := srv.Shutdown(shutdownCtx); err != nil {
		log.Printf("graceful shutdown failed: %v", err)
	}
}
