package game

import (
	"fmt"
	"testing"
	"time"
)

var (
	t0    = time.Date(2026, 1, 1, 12, 0, 0, 0, time.UTC)
	rules = DefaultRules() // boards built by NewBoard use these unless a test changes them
)

func testBoard() *Board {
	return NewBoard("b", "test", 10, 10, []*Entity{
		{ID: "u1", OwnerID: "p1", Kind: KindMinor, X: 1, Y: 1, Speed: 3, Health: 10, MaxHealth: 10},
		{ID: "u2", OwnerID: "p2", Kind: KindMinor, X: 5, Y: 5, Speed: 2, Health: 10, MaxHealth: 10},
		{ID: "u3", OwnerID: "p2", Kind: KindMinor, X: 1, Y: 2, Speed: 2, Health: 10, MaxHealth: 10},
		{ID: "dead", OwnerID: "p1", Kind: KindMinor, X: 8, Y: 8, Speed: 2, Health: 0, MaxHealth: 10},
		{ID: "npc", Kind: KindNPC, X: 3, Y: 3, Health: 10, MaxHealth: 10},
		{ID: "item", Kind: KindItem, X: 4, Y: 4},
	})
}

func TestMoveRules(t *testing.T) {
	tests := []struct {
		name   string
		player string
		unit   string
		to     Point
		want   *Error
	}{
		{"ok within speed", "p1", "u1", Point{4, 1}, nil},
		{"diagonal move within speed", "p1", "u1", Point{3, 2}, nil},
		{"too far", "p1", "u1", Point{5, 1}, ErrTooFar},
		{"not the owner", "p1", "u2", Point{5, 6}, ErrNotYours},
		{"cannot move an npc", "p1", "npc", Point{3, 4}, ErrNotYours},
		{"cannot move an item", "p1", "item", Point{4, 5}, ErrNotFound},
		{"unknown unit", "p1", "nope", Point{2, 2}, ErrNotFound},
		{"occupied by another player's unit", "p1", "u1", Point{5, 5}, ErrOccupied},
		{"occupied by an npc", "p1", "u1", Point{3, 3}, ErrOccupied},
		{"occupied by an item", "p1", "u1", Point{4, 4}, ErrOccupied},
		{"out of bounds", "p1", "u1", Point{-1, 1}, ErrOutOfBounds},
		{"same cell", "p1", "u1", Point{1, 1}, ErrSameCell},
		{"dead unit", "p1", "dead", Point{8, 9}, ErrDead},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			b := testBoard()
			_, err := b.Move(tc.player, tc.unit, tc.to, t0)
			if tc.want == nil {
				if err != nil {
					t.Fatalf("unexpected error: %v", err)
				}
				return
			}
			if err != tc.want {
				t.Fatalf("got %v, want %v", err, tc.want)
			}
		})
	}
}

func TestMoveUpdatesCells(t *testing.T) {
	b := testBoard()
	if _, err := b.Move("p1", "u1", Point{2, 1}, t0); err != nil {
		t.Fatal(err)
	}
	if _, err := b.Move("p2", "u3", Point{2, 1}, t0); err != ErrOccupied {
		t.Fatalf("new cell should be taken, got %v", err)
	}
	if _, err := b.Move("p2", "u3", Point{1, 1}, t0); err != nil {
		t.Fatalf("old cell should be free, got %v", err)
	}
}

func TestCooldownScalesWithDistanceOverSpeed(t *testing.T) {
	b := testBoard()
	e, err := b.Move("p1", "u1", Point{4, 1}, t0) // 3 cells at speed 3 = 1s
	if err != nil {
		t.Fatal(err)
	}
	if got := e.ReadyAt.Sub(t0); got != time.Second {
		t.Fatalf("cooldown = %v, want 1s", got)
	}
	if _, err := b.Move("p1", "u1", Point{5, 1}, t0.Add(500*time.Millisecond)); err != ErrCooldown {
		t.Fatalf("expected cooldown, got %v", err)
	}
	if _, err := b.Move("p1", "u1", Point{5, 1}, t0.Add(time.Second)); err != nil {
		t.Fatalf("should be ready after 1s: %v", err)
	}
}

func TestPlanTeam(t *testing.T) {
	b := NewBoard("b", "test", 24, 24, []*Entity{
		{ID: "npc", Kind: KindNPC, X: 3, Y: 2},
	})
	team, err := b.PlanTeam("p1", "Anna")
	if err != nil {
		t.Fatal(err)
	}
	if len(team) != 1+rules.MinorsPerTeam {
		t.Fatalf("team size = %d", len(team))
	}
	if team[0].Kind != KindChampion {
		t.Fatalf("first unit is %s, want champion", team[0].Kind)
	}
	seen := map[Point]bool{}
	for _, e := range team {
		p := Point{e.X, e.Y}
		if seen[p] {
			t.Fatalf("two units on %v", p)
		}
		if p == (Point{3, 2}) {
			t.Fatal("unit placed on an occupied cell")
		}
		seen[p] = true
		if e.OwnerID != "p1" {
			t.Fatalf("owner = %q", e.OwnerID)
		}
	}
}

func TestFirstTeamsSpawnOutOfEachOthersSight(t *testing.T) {
	b := NewBoard("b", "test", 24, 24, nil)
	players := []string{"p0", "p1", "p2", "p3", "p4", "p5"}
	for _, id := range players {
		team, err := b.PlanTeam(id, id)
		if err != nil {
			t.Fatal(err)
		}
		for j, e := range team {
			e.ID = fmt.Sprintf("%s-%d", id, j)
			b.Add(e)
		}
		b.EnsurePlayer(id, id, 0)
	}
	for _, id := range players {
		for entityID := range b.VisibleTo(id) {
			if owner := b.entities[entityID].OwnerID; owner != id {
				t.Errorf("%s sees a unit of %s at the start (%s)", id, owner, entityID)
			}
		}
	}
}

func TestPlanTeamBoardFull(t *testing.T) {
	b := NewBoard("b", "tiny", 3, 3, nil)
	if _, err := b.PlanTeam("p1", "Anna"); err != ErrBoardFull {
		t.Fatalf("got %v, want ErrBoardFull", err)
	}
}
