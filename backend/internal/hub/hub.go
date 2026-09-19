// Package hub runs every world of the server: one simulation Loop per world, side by side.
package hub

import (
	"context"
	"fmt"
	"sync"
	"time"

	"thegame/backend/internal/game"
	"thegame/backend/internal/store"
)

type Hub struct {
	ctx   context.Context
	store *store.Store

	mu      sync.RWMutex
	running map[string]*running
	editing map[string]*sync.Mutex // one edit at a time per world
}

// running is a world's loop together with what it takes to stop it.
type running struct {
	loop   *game.Loop
	cancel context.CancelFunc
}

// New makes a hub whose loops run until ctx is cancelled.
func New(ctx context.Context, st *store.Store) *Hub {
	return &Hub{ctx: ctx, store: st, running: map[string]*running{}, editing: map[string]*sync.Mutex{}}
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
	loopCtx, cancel := context.WithCancel(h.ctx)
	loop := game.NewLoop(world, h.store)
	h.mu.Lock()
	h.running[id] = &running{loop: loop, cancel: cancel}
	h.mu.Unlock()
	go loop.Run(loopCtx)
	return nil
}

// Edit changes a world's definition in the database and restarts the world so that it picks the
// change up. The world is stopped first (its loop finishes writing everything that happened so
// far), then change runs against a quiet database, then the world is loaded again; connected
// clients are disconnected and come back on their own. While it is going on the world is not
// available (Loop reports false). If change fails the world is loaded again unchanged.
func (h *Hub) Edit(ctx context.Context, id string, change func(context.Context) error) error {
	h.mu.Lock()
	lock, ok := h.editing[id]
	if !ok {
		lock = &sync.Mutex{}
		h.editing[id] = lock
	}
	h.mu.Unlock()
	lock.Lock()
	defer lock.Unlock()

	h.stop(id)
	changeErr := change(ctx)
	if err := h.Load(ctx, id); err != nil {
		if changeErr != nil {
			return fmt.Errorf("%w (and the world could not be reloaded: %v)", changeErr, err)
		}
		return fmt.Errorf("the world was changed but could not be reloaded: %w", err)
	}
	return changeErr
}

// stop takes the world out of service and waits for its loop to finish.
func (h *Hub) stop(id string) {
	h.mu.Lock()
	r := h.running[id]
	delete(h.running, id)
	h.mu.Unlock()
	if r == nil {
		return
	}
	r.cancel()
	<-r.loop.Done()
}

func (h *Hub) Loop(worldID string) (*game.Loop, bool) {
	h.mu.RLock()
	defer h.mu.RUnlock()
	r, ok := h.running[worldID]
	if !ok {
		return nil, false
	}
	return r.loop, true
}

func (h *Hub) Count() int {
	h.mu.RLock()
	defer h.mu.RUnlock()
	return len(h.running)
}

// Wait blocks until every running loop has finished (after the hub's context is cancelled), or
// until the timeout, so that their last database writes are made before the database closes.
func (h *Hub) Wait(timeout time.Duration) {
	h.mu.RLock()
	var done []<-chan struct{}
	for _, r := range h.running {
		done = append(done, r.loop.Done())
	}
	h.mu.RUnlock()
	deadline := time.After(timeout)
	for _, d := range done {
		select {
		case <-d:
		case <-deadline:
			return
		}
	}
}
