package game

import (
	"context"
	"sync"
	"testing"
	"time"
)

// slowStore counts the units saved, slowly, so the queue is still full when the loop stops.
type slowStore struct {
	noStore
	mu    sync.Mutex
	saved int
	ctxOK bool
}

func (s *slowStore) SaveUnit(ctx context.Context, _ Entity) error {
	time.Sleep(5 * time.Millisecond)
	s.mu.Lock()
	defer s.mu.Unlock()
	s.saved++
	s.ctxOK = ctx.Err() == nil
	return nil
}

func TestStopDrainsQueuedWrites(t *testing.T) {
	world := newTestWorld("b", "drain", 10, 10, []*Entity{
		{ID: "u", OwnerID: "p1", Kind: KindChampion, Name: "C", X: 1, Y: 1, Speed: 3, Health: 100, MaxHealth: 100, Vision: 3},
	})
	store := &slowStore{}
	loop := NewLoop(world, store)
	ctx, cancel := context.WithCancel(context.Background())
	go loop.Run(ctx)

	const n = 20
	loop.call(func() error {
		for i := 0; i < n; i++ {
			loop.saveUnit(world.All()[0])
		}
		return nil
	})
	cancel()
	select {
	case <-loop.Done():
	case <-time.After(3 * time.Second):
		t.Fatal("the loop never finished")
	}
	store.mu.Lock()
	defer store.mu.Unlock()
	if store.saved != n {
		t.Fatalf("saved %d of %d queued writes before stopping", store.saved, n)
	}
	if !store.ctxOK {
		t.Fatal("writes after the stop were made with a cancelled context")
	}
}

func TestPanicInACommandDoesNotKillTheLoop(t *testing.T) {
	loop, _, p1, _ := fogWorld(t)
	loop.do(func() { panic("boom") })

	loop.Register(p1)
	if m := recv(t, p1); m.Type != "snapshot" {
		t.Fatalf("expected a snapshot after the panic, got %q", m.Type)
	}
}

func TestAdminAssignsGoal(t *testing.T) {
	loop, _, p1, _ := fogWorldWith(t, func(w *World) {
		w.Rules.GoalAssignment = "manual"
		w.AddGoal(&Goal{ID: "g1", Scope: GoalIndividual, Kind: GoalPickups, Target: 2, Reward: 5, Title: "Raccogli"})
	})
	loop.Register(p1)
	recv(t, p1) // snapshot

	if err := loop.AssignGoal("p1", "g1"); err != nil {
		t.Fatal(err)
	}
	m := recv(t, p1)
	if m.Type != "goals" || len(m.Goals) == 0 || m.Goals[0].ID != "g1" {
		t.Fatalf("expected the new goal to reach the player, got %+v", m)
	}
	if err := loop.AssignGoal("nobody", "g1"); err == nil {
		t.Fatal("assigning to an unknown player must fail")
	}
	if err := loop.AssignGoal("p1", "nope"); err == nil {
		t.Fatal("assigning an unknown goal must fail")
	}
}
