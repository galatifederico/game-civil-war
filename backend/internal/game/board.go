// Package game holds the authoritative simulation: board state and its rules.
// It never touches the network or the database directly.
package game

import (
	crand "crypto/rand"
	"fmt"
	"sort"
	"time"
)

type Kind string

const (
	KindChampion  Kind = "champion"
	KindMinor     Kind = "minor"
	KindNPC       Kind = "npc"
	KindItem      Kind = "item"
	KindStructure Kind = "structure"
)

const (
	MinorsPerTeam = 12

	championSpeed, championHealth, championVision, championStrength = 3, 200, 5, 30
	minorSpeed, minorHealth, minorVision, minorStrength             = 2, 100, 3, 15
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
	Vision      int // sight and action range, in cells
	Strength    int // damage dealt by an attack
	Dialogue    []string

	// Ephemeral real-time state, not persisted.
	ReadyAt    time.Time // next moment the unit can move
	ActReadyAt time.Time // next moment the unit can act
	RespawnAt  time.Time // set while the unit is defeated
}

// IsUnit is true for anything with hit points: champions, minor units and NPCs.
func (e *Entity) IsUnit() bool {
	return e.Kind == KindChampion || e.Kind == KindMinor || e.Kind == KindNPC
}

// Controllable units belong to a player and take orders from them.
func (e *Entity) Controllable() bool { return e.Kind == KindChampion || e.Kind == KindMinor }

type Item struct {
	Name        string
	Description string
}

// Player is the roster owner: one player is exactly one team (see design.md).
type Player struct {
	ID        string
	Username  string
	Points    int
	Inventory []Item // shared by the whole team
}

type Score struct {
	PlayerID string
	Username string
	Points   int
}

type Board struct {
	ID       string
	Name     string
	Width    int
	Height   int
	entities map[string]*Entity
	cells    map[Point]string
	players  map[string]*Player
}

func NewBoard(id, name string, width, height int, entities []*Entity) *Board {
	b := &Board{
		ID: id, Name: name, Width: width, Height: height,
		entities: make(map[string]*Entity, len(entities)),
		cells:    make(map[Point]string, len(entities)),
		players:  map[string]*Player{},
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

func (b *Board) remove(e *Entity) {
	delete(b.entities, e.ID)
	delete(b.cells, Point{e.X, e.Y})
}

func (b *Board) All() []*Entity {
	out := make([]*Entity, 0, len(b.entities))
	for _, e := range b.entities {
		out = append(out, e)
	}
	return out
}

// EnsurePlayer returns the player with this id, creating it if needed.
func (b *Board) EnsurePlayer(id, username string, points int) *Player {
	p, ok := b.players[id]
	if !ok {
		p = &Player{ID: id, Username: username, Points: points}
		b.players[id] = p
	}
	return p
}

func (b *Board) Player(id string) *Player { return b.players[id] }

// Scores lists every player, best first.
func (b *Board) Scores() []Score {
	out := make([]Score, 0, len(b.players))
	for _, p := range b.players {
		out = append(out, Score{PlayerID: p.ID, Username: p.Username, Points: p.Points})
	}
	sort.Slice(out, func(i, j int) bool {
		if out[i].Points != out[j].Points {
			return out[i].Points > out[j].Points
		}
		return out[i].Username < out[j].Username
	})
	return out
}

func (b *Board) HasUnitsOf(playerID string) bool {
	for _, e := range b.entities {
		if e.OwnerID == playerID && e.Controllable() {
			return true
		}
	}
	return false
}

// ownedUnit finds a unit and checks the player may give it orders.
func (b *Board) ownedUnit(playerID, unitID string) (*Entity, error) {
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
	return e, nil
}

// Move relocates a unit if the rules allow it: the unit must belong to the player,
// be alive and ready, and the target must be free, inside the board and within `Speed` cells
// (Chebyshev distance). After a move the unit is on cooldown for distance/speed seconds.
func (b *Board) Move(playerID, unitID string, to Point, now time.Time) (*Entity, error) {
	e, err := b.ownedUnit(playerID, unitID)
	if err != nil {
		return nil, err
	}
	if now.Before(e.ReadyAt) {
		return nil, ErrCooldown
	}
	if !b.inBounds(to) {
		return nil, ErrOutOfBounds
	}
	from := Point{e.X, e.Y}
	if to == from {
		return nil, ErrSameCell
	}
	if _, taken := b.cells[to]; taken {
		return nil, ErrOccupied
	}
	dist := distance(from, to)
	if e.Speed <= 0 || dist > e.Speed {
		return nil, ErrTooFar
	}

	delete(b.cells, from)
	b.cells[to] = e.ID
	e.X, e.Y = to.X, to.Y
	e.ReadyAt = now.Add(time.Duration(float64(dist) / float64(e.Speed) * float64(time.Second)))
	return e, nil
}

// Tick advances real-time state and returns the units that came back from defeat.
func (b *Board) Tick(now time.Time) []*Entity {
	var revived []*Entity
	for _, e := range b.entities {
		if e.IsUnit() && e.Health <= 0 && !e.RespawnAt.IsZero() && !now.Before(e.RespawnAt) {
			e.Health = e.MaxHealth
			e.RespawnAt = time.Time{}
			revived = append(revived, e)
		}
	}
	return revived
}

// ScheduleRespawns gives a respawn time to units that were already defeated when the server
// (re)started, so they do not stay out of the game forever.
func (b *Board) ScheduleRespawns(now time.Time) {
	for _, e := range b.entities {
		if e.IsUnit() && e.Health <= 0 && e.RespawnAt.IsZero() {
			e.RespawnAt = now.Add(RespawnDelay)
		}
	}
}

// PlanTeam builds a new player's team (1 champion + MinorsPerTeam minor units) on the free
// cells nearest to a per-player anchor, so teams spawn in separate areas. The entities have no
// ID yet: the persistence layer assigns it.
func (b *Board) PlanTeam(playerID, username string) ([]*Entity, error) {
	slot := len(b.players)
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
		Speed: championSpeed, Health: championHealth, MaxHealth: championHealth,
		Vision: championVision, Strength: championStrength,
	})
	for i := 1; i < need; i++ {
		team = append(team, &Entity{
			OwnerID: playerID, Kind: KindMinor, Name: fmt.Sprintf("Pedina %d", i),
			Description: fmt.Sprintf("Una fedele pedina della squadra di %s.", username),
			X:           free[i].p.X, Y: free[i].p.Y,
			Speed: minorSpeed, Health: minorHealth, MaxHealth: minorHealth,
			Vision: minorVision, Strength: minorStrength,
		})
	}
	return team, nil
}

func (b *Board) inBounds(p Point) bool {
	return p.X >= 0 && p.Y >= 0 && p.X < b.Width && p.Y < b.Height
}

// distance is the Chebyshev distance: diagonal steps cost the same as straight ones.
func distance(a, b Point) int { return max(abs(a.X-b.X), abs(a.Y-b.Y)) }

func abs(v int) int {
	if v < 0 {
		return -v
	}
	return v
}

// newID returns a random UUID (v4), for entities created by the simulation itself.
func newID() string {
	var b [16]byte
	if _, err := crand.Read(b[:]); err != nil {
		panic(err)
	}
	b[6] = b[6]&0x0f | 0x40
	b[8] = b[8]&0x3f | 0x80
	return fmt.Sprintf("%x-%x-%x-%x-%x", b[0:4], b[4:6], b[6:8], b[8:10], b[10:])
}
