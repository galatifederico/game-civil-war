package game

import (
	"context"
	"encoding/json"
	"sync"
	"testing"
	"time"

	"thegame/backend/internal/protocol"
)

type noStore struct{}

func (noStore) InsertUnits(context.Context, []*Entity) error             { return nil }
func (noStore) SaveUnit(context.Context, Entity) error                   { return nil }
func (noStore) DeleteInventoryItems(context.Context, []string) error     { return nil }
func (noStore) InsertStructure(context.Context, *Entity) error           { return nil }
func (noStore) SaveItemPosition(context.Context, string, int, int) error { return nil }
func (noStore) DeleteItem(context.Context, string) error                 { return nil }
func (noStore) AddInventory(context.Context, string, string, Item) error { return nil }
func (noStore) AddPoints(context.Context, string, string, int) error     { return nil }

// fogWorld: player p1 has a champion at (2,2) (vision 5); p2 has a minor unit at (8,2) (vision 3).
// An NPC and an item sit near p1, another NPC is far from everyone.
func fogWorld(t *testing.T) (loop *Loop, advance func(time.Duration), p1, p2 *Client) {
	board := newTestWorld("b", "fog", 30, 30, []*Entity{
		{ID: "champ", OwnerID: "p1", Kind: KindChampion, Name: "Champion", X: 2, Y: 2, Speed: 3, Health: 200, MaxHealth: 200, Vision: 5, Strength: 30},
		{ID: "minor", OwnerID: "p2", Kind: KindMinor, Name: "Pedina", X: 8, Y: 2, Speed: 2, Health: 100, MaxHealth: 100, Vision: 3, Strength: 15},
		{ID: "near", Kind: KindNPC, Name: "Vicino", X: 4, Y: 4, Health: 100, MaxHealth: 100},
		{ID: "far", Kind: KindNPC, Name: "Lontano", X: 25, Y: 25, Health: 100, MaxHealth: 100},
		{ID: "item", Kind: KindItem, Name: "Forziere", X: 3, Y: 3},
	})
	board.EnsurePlayer("p1", "Anna", 0)
	board.EnsurePlayer("p2", "Bob", 0)

	loop = NewLoop(board, noStore{})
	var mu sync.Mutex
	clock := time.Date(2026, 1, 1, 12, 0, 0, 0, time.UTC)
	loop.now = func() time.Time { mu.Lock(); defer mu.Unlock(); return clock }
	advance = func(d time.Duration) { mu.Lock(); clock = clock.Add(d); mu.Unlock() }

	ctx, cancel := context.WithCancel(context.Background())
	t.Cleanup(cancel)
	go loop.Run(ctx)

	p1 = &Client{PlayerID: "p1", Send: make(chan []byte, 32)}
	p2 = &Client{PlayerID: "p2", Send: make(chan []byte, 32)}
	return loop, advance, p1, p2
}

func recv(t *testing.T, c *Client) protocol.ServerMessage {
	t.Helper()
	select {
	case data, ok := <-c.Send:
		if !ok {
			t.Fatal("client was dropped")
		}
		var m protocol.ServerMessage
		if err := json.Unmarshal(data, &m); err != nil {
			t.Fatal(err)
		}
		return m
	case <-time.After(time.Second):
		t.Fatal("timed out waiting for a message")
		return protocol.ServerMessage{}
	}
}

func expectSilence(t *testing.T, c *Client) {
	t.Helper()
	select {
	case data := <-c.Send:
		t.Fatalf("unexpected message: %s", data)
	case <-time.After(150 * time.Millisecond):
	}
}

func ids(entities []protocol.Entity) map[string]bool {
	out := map[string]bool{}
	for _, e := range entities {
		out[e.ID] = true
	}
	return out
}

func TestSnapshotOnlyContainsWhatThePlayerCanSee(t *testing.T) {
	loop, _, p1, p2 := fogWorld(t)

	loop.Register(p1)
	got := ids(recv(t, p1).Entities)
	for _, want := range []string{"champ", "near", "item"} {
		if !got[want] {
			t.Errorf("p1 should see %s", want)
		}
	}
	for _, hidden := range []string{"minor", "far"} {
		if got[hidden] {
			t.Errorf("p1 should not see %s", hidden)
		}
	}

	loop.Register(p2)
	got = ids(recv(t, p2).Entities)
	if !got["minor"] || len(got) != 1 {
		t.Errorf("p2 should see only its own unit, got %v", got)
	}
}

func TestEntitiesEnterAndLeaveTheView(t *testing.T) {
	loop, advance, p1, p2 := fogWorld(t)
	loop.Register(p1)
	recv(t, p1)
	loop.Register(p2)
	recv(t, p2)

	// p2's minor steps to (7,2): distance 5 from p1's champion, so it comes into view.
	loop.Move(p2, "minor", Point{7, 2})
	appeared := recv(t, p1)
	if appeared.Type != protocol.TypeDelta || len(appeared.Entities) != 1 || appeared.Entities[0].ID != "minor" || appeared.Entities[0].X != 7 {
		t.Fatalf("p1 should be shown the minor unit, got %+v", appeared)
	}
	// p2 hears about its own move, and its new position also brings the NPC at (4,4) into its view.
	if own := ids(recv(t, p2).Entities); !own["minor"] || !own["near"] || len(own) != 2 {
		t.Fatalf("p2 should get its own move and the newly visible NPC, got %v", own)
	}

	// While it stays in view, its moves are ordinary updates.
	advance(time.Second)
	loop.Move(p2, "minor", Point{6, 2})
	if moved := recv(t, p1); len(moved.Entities) != 1 || moved.Entities[0].X != 6 || len(moved.Removed) != 0 {
		t.Fatalf("expected a plain update, got %+v", moved)
	}
	recv(t, p2)

	// Walking away takes it out of view: p1 is told to forget it.
	advance(time.Second)
	loop.Move(p2, "minor", Point{8, 2})
	advance(time.Second)
	loop.Move(p2, "minor", Point{10, 2})
	recv(t, p2)
	recv(t, p2)
	gone := recv(t, p1) // the update to (8,2) is 6 cells away: already out of view
	if len(gone.Removed) != 1 || gone.Removed[0] != "minor" || len(gone.Entities) != 0 {
		t.Fatalf("p1 should be told the unit left its view, got %+v", gone)
	}
	expectSilence(t, p1) // and nothing about it afterwards
}

func TestChangesOutOfSightAreNotSent(t *testing.T) {
	loop, _, p1, p2 := fogWorld(t)
	loop.Register(p1)
	recv(t, p1)
	loop.Register(p2)
	recv(t, p2)

	// p2's unit moves far from p1: p1 must hear nothing.
	loop.Move(p2, "minor", Point{9, 2})
	recv(t, p2)
	expectSilence(t, p1)
}

func TestPickingUpAnItemRemovesItOnlyForThoseWhoSawIt(t *testing.T) {
	loop, _, p1, p2 := fogWorld(t)
	loop.Register(p1)
	recv(t, p1)
	loop.Register(p2)
	recv(t, p2)

	loop.Act(p1, Action{Kind: ActionPickup, UnitID: "champ", TargetID: "item"})
	m := recv(t, p1)
	if m.Type != protocol.TypeDelta || len(m.Removed) != 1 || m.Removed[0] != "item" {
		t.Fatalf("p1 should see the item removed, got %+v", m)
	}

	// p2 never saw the item, but the scoreboard is public: it gets the scores and nothing else.
	s := recv(t, p2)
	if len(s.Scores) != 2 || len(s.Entities) != 0 || len(s.Removed) != 0 {
		t.Fatalf("p2 should get only the scores, got %+v", s)
	}
}

func TestVisibleToRules(t *testing.T) {
	b := newTestWorld("b", "fog", 30, 30, []*Entity{
		{ID: "champ", OwnerID: "p1", Kind: KindChampion, X: 2, Y: 2, Health: 10, Vision: 5},
		{ID: "dead", OwnerID: "p1", Kind: KindMinor, X: 20, Y: 20, Health: 0, Vision: 9},
		{ID: "structure", OwnerID: "p1", Kind: KindStructure, X: 28, Y: 28},
		{ID: "edge", Kind: KindNPC, X: 7, Y: 2},
		{ID: "beyond", Kind: KindNPC, X: 8, Y: 2},
		{ID: "seenByDead", Kind: KindNPC, X: 21, Y: 20},
		{ID: "theirs", OwnerID: "p2", Kind: KindMinor, X: 3, Y: 2, Health: 10, Vision: 1},
	})
	v := b.VisibleTo("p1")
	for id, want := range map[string]bool{
		"champ": true, "dead": true, "structure": true, // everything of one's own
		"edge": true, "beyond": false, // vision is inclusive
		"seenByDead": false, // a defeated unit does not see
		"theirs":     true,  // an enemy inside the vision range
	} {
		if _, ok := v[id]; ok != want {
			t.Errorf("visible[%s] = %v, want %v", id, ok, want)
		}
	}
}
