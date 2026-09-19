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
	ID     string
	Name   string
	Width  int
	Height int
	Rules  Rules // the world's parameters, see rules.go

	entities map[string]*Entity
	cells    map[Point]string
	players  map[string]*Player
}

func NewBoard(id, name string, width, height int, entities []*Entity) *Board {
	b := &Board{
		ID: id, Name: name, Width: width, Height: height, Rules: DefaultRules(),
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

// VisibleTo returns the ids of the entities a player can see (fog of war): everything that
// belongs to them, plus whatever is within the vision of one of their units still in the game.
// "Vista" is both sight and action range (design.md), so anything a unit can act on is visible.
func (b *Board) VisibleTo(playerID string) map[string]struct{} {
	var eyes []*Entity
	for _, e := range b.entities {
		if e.OwnerID == playerID && e.Controllable() && e.Health > 0 {
			eyes = append(eyes, e)
		}
	}
	visible := make(map[string]struct{})
	for _, e := range b.entities {
		if e.OwnerID == playerID {
			visible[e.ID] = struct{}{}
			continue
		}
		for _, eye := range eyes {
			if distance(Point{eye.X, eye.Y}, Point{e.X, e.Y}) <= eye.Vision {
				visible[e.ID] = struct{}{}
				break
			}
		}
	}
	return visible
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
			e.RespawnAt = now.Add(b.Rules.RespawnDelay())
		}
	}
}

// PlanTeam builds a new player's team (1 champion + Rules.MinorsPerTeam minor units) on the free
// cells nearest to a per-player anchor, so teams spawn in separate areas. The entities have no
// ID yet: the persistence layer assigns it.
func (b *Board) PlanTeam(playerID, username string) ([]*Entity, error) {
	anchor := spawnAnchor(len(b.players), b.Width, b.Height)

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
	need := 1 + b.Rules.MinorsPerTeam
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
		Speed: b.Rules.Champion.Speed, Health: b.Rules.Champion.Health, MaxHealth: b.Rules.Champion.Health,
		Vision: b.Rules.Champion.Vision, Strength: b.Rules.Champion.Strength,
	})
	for i := 1; i < need; i++ {
		team = append(team, &Entity{
			OwnerID: playerID, Kind: KindMinor, Name: fmt.Sprintf("Pedina %d", i),
			Description: fmt.Sprintf("Una fedele pedina della squadra di %s.", username),
			X:           free[i].p.X, Y: free[i].p.Y,
			Speed: b.Rules.Minor.Speed, Health: b.Rules.Minor.Health, MaxHealth: b.Rules.Minor.Health,
			Vision: b.Rules.Minor.Vision, Strength: b.Rules.Minor.Strength,
		})
	}
	return team, nil
}

// spawnAnchor spreads the teams over the board: the first six along the two sides, then the
// middle. With fog of war, teams that start out of each other's sight is what makes exploring
// and meeting the others matter. Later rounds are shifted so they do not pile up on one anchor.
func spawnAnchor(slot, width, height int) Point {
	fractions := [][2]float64{{0.17, 0.13}, {0.83, 0.13}, {0.17, 0.5}, {0.83, 0.5}, {0.17, 0.87}, {0.83, 0.87}, {0.5, 0.3}, {0.5, 0.7}}
	f := fractions[slot%len(fractions)]
	round := slot / len(fractions)
	return Point{
		X: min(int(f[0]*float64(width))+round*3, width-1),
		Y: min(int(f[1]*float64(height))+round*2, height-1),
	}
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
