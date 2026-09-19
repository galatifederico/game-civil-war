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

// Cell is a position in the world: a board and a point on it.
type Cell struct {
	Board string
	X, Y  int
}

func (c Cell) Point() Point { return Point{c.X, c.Y} }

type Entity struct {
	ID          string
	BoardID     string
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

func (e *Entity) Cell() Cell   { return Cell{e.BoardID, e.X, e.Y} }
func (e *Entity) Point() Point { return Point{e.X, e.Y} }

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

// Board is one zone of the world: a grid of cells. The world is made of several boards that
// are joined by gateways, so players walk from one to the next (design.md: contiguous areas).
type Board struct {
	ID     string
	Name   string
	Width  int
	Height int
	Grid   Grid
}

func (b *Board) InBounds(p Point) bool {
	return p.X >= 0 && p.Y >= 0 && p.X < b.Width && p.Y < b.Height
}

// Gateway is a cell that carries whoever steps on it to another board.
type Gateway struct {
	At Point
	To Cell
}

// World is the whole simulated state of one game world: its boards, everything on them, the
// players and the rules. Only the Loop's goroutine touches it, so it needs no locking.
type World struct {
	ID    string
	Name  string
	Rules Rules // the world's parameters, see rules.go

	boards   map[string]*Board
	order    []string // board ids; the first one is where new teams start
	entities map[string]*Entity
	cells    map[Cell]string
	links    map[Cell]Cell
	players  map[string]*Player
}

func NewWorld(id, name string) *World {
	return &World{
		ID: id, Name: name, Rules: DefaultRules(),
		boards:   map[string]*Board{},
		entities: map[string]*Entity{},
		cells:    map[Cell]string{},
		links:    map[Cell]Cell{},
		players:  map[string]*Player{},
	}
}

func (w *World) AddBoard(b *Board) {
	w.boards[b.ID] = b
	w.order = append(w.order, b.ID)
}

func (w *World) Board(id string) *Board { return w.boards[id] }

// Boards lists the boards in order (the first is the spawn board).
func (w *World) Boards() []*Board {
	out := make([]*Board, 0, len(w.order))
	for _, id := range w.order {
		out = append(out, w.boards[id])
	}
	return out
}

// AddLink makes `from` a gateway that leads to `to`. Links are one-way; a two-way passage is two links.
func (w *World) AddLink(from, to Cell) { w.links[from] = to }

// Gateways lists the gateways of a board, in a stable order.
func (w *World) Gateways(boardID string) []Gateway {
	var out []Gateway
	for from, to := range w.links {
		if from.Board == boardID {
			out = append(out, Gateway{At: from.Point(), To: to})
		}
	}
	sort.Slice(out, func(i, j int) bool {
		if out[i].At.Y != out[j].At.Y {
			return out[i].At.Y < out[j].At.Y
		}
		return out[i].At.X < out[j].At.X
	})
	return out
}

// blocked is true where nothing can be put: an occupied cell or a gateway.
func (w *World) blocked(c Cell) bool {
	if _, taken := w.cells[c]; taken {
		return true
	}
	_, gateway := w.links[c]
	return gateway
}

func (w *World) Add(e *Entity) {
	w.entities[e.ID] = e
	w.cells[e.Cell()] = e.ID
}

func (w *World) remove(e *Entity) {
	delete(w.entities, e.ID)
	delete(w.cells, e.Cell())
}

func (w *World) All() []*Entity {
	out := make([]*Entity, 0, len(w.entities))
	for _, e := range w.entities {
		out = append(out, e)
	}
	return out
}

// EnsurePlayer returns the player with this id, creating it if needed.
func (w *World) EnsurePlayer(id, username string, points int) *Player {
	p, ok := w.players[id]
	if !ok {
		p = &Player{ID: id, Username: username, Points: points}
		w.players[id] = p
	}
	return p
}

func (w *World) Player(id string) *Player { return w.players[id] }

// Scores lists every player, best first.
func (w *World) Scores() []Score {
	out := make([]Score, 0, len(w.players))
	for _, p := range w.players {
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

func (w *World) HasUnitsOf(playerID string) bool {
	for _, e := range w.entities {
		if e.OwnerID == playerID && e.Controllable() {
			return true
		}
	}
	return false
}

// VisibleTo returns the ids of the entities a player can see (fog of war): everything that
// belongs to them, plus whatever is within the vision of one of their units still in the game.
// "Vista" is both sight and action range (design.md), so anything a unit can act on is visible.
// Vision does not cross from one board to another.
func (w *World) VisibleTo(playerID string) map[string]struct{} {
	eyes := map[string][]*Entity{} // by board
	for _, e := range w.entities {
		if e.OwnerID == playerID && e.Controllable() && e.Health > 0 {
			eyes[e.BoardID] = append(eyes[e.BoardID], e)
		}
	}
	visible := make(map[string]struct{})
	for _, e := range w.entities {
		if e.OwnerID == playerID {
			visible[e.ID] = struct{}{}
			continue
		}
		grid := w.boards[e.BoardID].Grid
		for _, eye := range eyes[e.BoardID] {
			if grid.Distance(eye.Point(), e.Point()) <= eye.Vision {
				visible[e.ID] = struct{}{}
				break
			}
		}
	}
	return visible
}

// ownedUnit finds a unit and checks the player may give it orders.
func (w *World) ownedUnit(playerID, unitID string) (*Entity, error) {
	e, ok := w.entities[unitID]
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

// Move relocates a unit if the rules allow it: the unit must belong to the player, be alive and
// ready, and the target must be inside its board and within `Speed` steps (as the board's grid
// counts them). Stepping onto a gateway carries the unit to the other board. After a move the
// unit is on cooldown for distance/speed seconds.
func (w *World) Move(playerID, unitID string, to Point, now time.Time) (*Entity, error) {
	e, err := w.ownedUnit(playerID, unitID)
	if err != nil {
		return nil, err
	}
	if now.Before(e.ReadyAt) {
		return nil, ErrCooldown
	}
	board := w.boards[e.BoardID]
	if board == nil || !board.InBounds(to) {
		return nil, ErrOutOfBounds
	}
	from := e.Point()
	if to == from {
		return nil, ErrSameCell
	}
	dest := Cell{e.BoardID, to.X, to.Y}
	link, isGateway := w.links[dest]
	if _, taken := w.cells[dest]; taken && !isGateway {
		return nil, ErrOccupied
	}
	dist := board.Grid.Distance(from, to)
	if e.Speed <= 0 || dist > e.Speed {
		return nil, ErrTooFar
	}
	if isGateway {
		if _, taken := w.cells[link]; taken {
			return nil, ErrGatewayBlocked
		}
		dest = link
	}

	delete(w.cells, e.Cell())
	e.BoardID, e.X, e.Y = dest.Board, dest.X, dest.Y
	w.cells[dest] = e.ID
	e.ReadyAt = now.Add(time.Duration(float64(dist) / float64(e.Speed) * float64(time.Second)))
	return e, nil
}

// Tick advances real-time state and returns the units that came back from defeat.
func (w *World) Tick(now time.Time) []*Entity {
	var revived []*Entity
	for _, e := range w.entities {
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
func (w *World) ScheduleRespawns(now time.Time) {
	for _, e := range w.entities {
		if e.IsUnit() && e.Health <= 0 && e.RespawnAt.IsZero() {
			e.RespawnAt = now.Add(w.Rules.RespawnDelay())
		}
	}
}

// PlanTeam builds a new player's team (1 champion + Rules.MinorsPerTeam minor units) on the spawn
// board, on the free cells nearest to a per-player anchor, so teams start in separate areas.
// The entities have no ID yet: the persistence layer assigns it.
func (w *World) PlanTeam(playerID, username string) ([]*Entity, error) {
	if len(w.order) == 0 {
		return nil, ErrNoBoard
	}
	spawn := w.boards[w.order[0]]
	anchor := spawnAnchor(len(w.players), spawn.Width, spawn.Height)

	type candidate struct {
		p    Point
		cost int
	}
	var free []candidate
	for y := 0; y < spawn.Height; y++ {
		for x := 0; x < spawn.Width; x++ {
			if w.blocked(Cell{spawn.ID, x, y}) {
				continue
			}
			// Weighting rows more than columns gives a wide, flat formation.
			free = append(free, candidate{Point{x, y}, abs(x-anchor.X) + 3*abs(y-anchor.Y)})
		}
	}
	need := 1 + w.Rules.MinorsPerTeam
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
		OwnerID: playerID, BoardID: spawn.ID, Kind: KindChampion, Name: "Champion",
		Description: fmt.Sprintf("Il campione della squadra di %s: forte, carismatico e convinto di essere indispensabile.", username),
		X:           free[0].p.X, Y: free[0].p.Y,
		Speed: w.Rules.Champion.Speed, Health: w.Rules.Champion.Health, MaxHealth: w.Rules.Champion.Health,
		Vision: w.Rules.Champion.Vision, Strength: w.Rules.Champion.Strength,
	})
	for i := 1; i < need; i++ {
		team = append(team, &Entity{
			OwnerID: playerID, BoardID: spawn.ID, Kind: KindMinor, Name: fmt.Sprintf("Pedina %d", i),
			Description: fmt.Sprintf("Una fedele pedina della squadra di %s.", username),
			X:           free[i].p.X, Y: free[i].p.Y,
			Speed: w.Rules.Minor.Speed, Health: w.Rules.Minor.Health, MaxHealth: w.Rules.Minor.Health,
			Vision: w.Rules.Minor.Vision, Strength: w.Rules.Minor.Strength,
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

// freeNeighbor finds a cell next to c (as its board's grid counts neighbours) where something can be put.
func (w *World) freeNeighbor(c Cell) (Cell, bool) {
	board := w.boards[c.Board]
	for _, n := range board.Grid.Neighbors(c.Point()) {
		cell := Cell{c.Board, n.X, n.Y}
		if board.InBounds(n) && !w.blocked(cell) {
			return cell, true
		}
	}
	return Cell{}, false
}

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
