// Package hub runs every world of the server: one simulation Loop per world, side by side.
package hub

import (
	"context"
	"fmt"
	"sync"

	"thegame/backend/internal/game"
	"thegame/backend/internal/store"
)

type Hub struct {
	ctx   context.Context
	store *store.Store

	mu    sync.RWMutex
	loops map[string]*game.Loop
}

// New makes a hub whose loops run until ctx is cancelled.
func New(ctx context.Context, st *store.Store) *Hub {
	return &Hub{ctx: ctx, store: st, loops: map[string]*game.Loop{}}
}

// LoadAll loads every world in the database and starts its loop.
func (h *Hub) LoadAll(ctx context.Context) error {
	ids, err := h.store.WorldIDs(ctx)
	if err != nil {
		return err
	}
	for _, id := range ids {
		if err := h.Load(ctx, id); err != nil {
			return fmt.Errorf("world %s: %w", id, err)
		}
	}
	return nil
}

// Load reads one world from the database and starts running it.
func (h *Hub) Load(ctx context.Context, id string) error {
	world, err := h.store.LoadWorld(ctx, id)
	if err != nil {
		return err
	}
	loop := game.NewLoop(world, h.store)
	h.mu.Lock()
	h.loops[id] = loop
	h.mu.Unlock()
	go loop.Run(h.ctx)
	return nil
}

func (h *Hub) Loop(worldID string) (*game.Loop, bool) {
	h.mu.RLock()
	defer h.mu.RUnlock()
	l, ok := h.loops[worldID]
	return l, ok
}

func (h *Hub) Count() int {
	h.mu.RLock()
	defer h.mu.RUnlock()
	return len(h.loops)
}
