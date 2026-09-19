// Package game holds the authoritative simulation: board state and its rules.
// It never touches the network or the database directly.
package game

import (
	"fmt"
	"sort"
	"time"
)

type Kind string

const (
	KindChampion Kind = "champion"
	KindMinor    Kind = "minor"
	KindNPC      Kind = "npc"
	KindItem     Kind = "item"
)

const (
	MinorsPerTeam = 12

	championSpeed, championHealth, championVision = 3, 200, 5
	minorSpeed, minorHealth, minorVision          = 2, 100, 3
)

type Point struct{ X, Y int }

type Entity struct {
	ID          string
	OwnerID     string // empty for NPCs and items
	Kind        Kind
	Name        string
	Description string
	X, Y        int
	Speed       int // max cells per move; also sets the cooldown after a move
	Health      int
	MaxHealth   int
	Vision      int
	ReadyAt     time.Time // not persisted: ephemeral real-time state
}

func (e *Entity) IsUnit() bool { return e.Kind != KindItem }

type Board struct {
	ID       string
	Name     string
	Width    int
	Height   int
	entities map[string]*Entity
	cells    map[Point]string
}

func NewBoard(id, name string, width, height int, entities []*Entity) *Board {
	b := &Board{
		ID: id, Name: name, Width: width, Height: height,
		entities: make(map[string]*Entity, len(entities)),
		cells:    make(map[Point]string, len(entities)),
	}
	for _, e := range entities {
		b.Add(e)
	}
	return b
}

func (b *Board) Add(e *Entity) {
	b.entities[e.ID] = e
	b.cells[Point{e.X, e.Y}] = e.ID
}

func (b *Board) All() []*Entity {
	out := make([]*Entity, 0, len(b.entities))
	for _, e := range b.entities {
		out = append(out, e)
	}
	return out
}

func (b *Board) HasUnitsOf(playerID string) bool {
	for _, e := range b.entities {
		if e.OwnerID == playerID {
			return true
		}
	}
	return false
}

func (b *Board) ownerCount() int {
	owners := map[string]struct{}{}
	for _, e := range b.entities {
		if e.OwnerID != "" {
			owners[e.OwnerID] = struct{}{}
		}
	}
	return len(owners)
}

// Move relocates a unit if the rules allow it: the unit must belong to the player,
// be alive and ready, and the target must be free, inside the board and within `Speed` cells
// (Chebyshev distance). After a move the unit is on cooldown for distance/speed seconds.
func (b *Board) Move(playerID, unitID string, to Point, now time.Time) (*Entity, error) {
	e, ok := b.entities[unitID]
	if !ok || !e.IsUnit() {
		return nil, ErrNotFound
	}
	if e.OwnerID == "" || e.OwnerID != playerID {
		return nil, ErrNotYours
	}
	if e.Health <= 0 {
		return nil, ErrDead
	}
	if now.Before(e.ReadyAt) {
		return nil, ErrCooldown
	}
	if to.X < 0 || to.Y < 0 || to.X >= b.Width || to.Y >= b.Height {
		return nil, ErrOutOfBounds
	}
	from := Point{e.X, e.Y}
	if to == from {
		return nil, ErrSameCell
	}
	if _, taken := b.cells[to]; taken {
		return nil, ErrOccupied
	}
	dist := max(abs(to.X-from.X), abs(to.Y-from.Y))
	if e.Speed <= 0 || dist > e.Speed {
		return nil, ErrTooFar
	}

	delete(b.cells, from)
	b.cells[to] = e.ID
	e.X, e.Y = to.X, to.Y
	e.ReadyAt = now.Add(time.Duration(float64(dist) / float64(e.Speed) * float64(time.Second)))
	return e, nil
}

// PlanTeam builds a new player's team (1 champion + MinorsPerTeam minor units) on the free
// cells nearest to a per-player anchor, so teams spawn in separate areas. The entities have no
// ID yet: the persistence layer assigns it.
func (b *Board) PlanTeam(playerID, username string) ([]*Entity, error) {
	slot := b.ownerCount()
	anchor := Point{
		X: min(3+(slot%3)*8, b.Width-1),
		Y: min(2+((slot/3)*4)%max(b.Height-3, 1), b.Height-1),
	}

	type candidate struct {
		p    Point
		cost int
	}
	var free []candidate
	for y := 0; y < b.Height; y++ {
		for x := 0; x < b.Width; x++ {
			p := Point{x, y}
			if _, taken := b.cells[p]; !taken {
				// Weighting rows more than columns gives a wide, flat formation.
				free = append(free, candidate{p, abs(x-anchor.X) + 3*abs(y-anchor.Y)})
			}
		}
	}
	need := 1 + MinorsPerTeam
	if len(free) < need {
		return nil, ErrBoardFull
	}
	sort.Slice(free, func(i, j int) bool {
		a, c := free[i], free[j]
		if a.cost != c.cost {
			return a.cost < c.cost
		}
		if a.p.Y != c.p.Y {
			return a.p.Y < c.p.Y
		}
		return a.p.X < c.p.X
	})

	team := make([]*Entity, 0, need)
	team = append(team, &Entity{
		OwnerID: playerID, Kind: KindChampion, Name: "Champion",
		Description: fmt.Sprintf("Il campione della squadra di %s: forte, carismatico e convinto di essere indispensabile.", username),
		X:           free[0].p.X, Y: free[0].p.Y,
		Speed: championSpeed, Health: championHealth, MaxHealth: championHealth, Vision: championVision,
	})
	for i := 1; i < need; i++ {
		team = append(team, &Entity{
			OwnerID: playerID, Kind: KindMinor, Name: fmt.Sprintf("Pedina %d", i),
			Description: fmt.Sprintf("Una fedele pedina della squadra di %s.", username),
			X:           free[i].p.X, Y: free[i].p.Y,
			Speed: minorSpeed, Health: minorHealth, MaxHealth: minorHealth, Vision: minorVision,
		})
	}
	return team, nil
}

func abs(v int) int {
	if v < 0 {
		return -v
	}
	return v
}
