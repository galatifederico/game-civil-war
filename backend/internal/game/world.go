// Package game holds the authoritative simulation: board state and its rules.
// It never touches the network or the database directly.
package game

import (
	crand "crypto/rand"
	"fmt"
	"math/rand/v2"
	"sort"
	"strings"
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
	RaceID      string         // units only; empty when the world has no races
	Traits      map[string]int // the extended characteristics (soldi, alcol...)
	Effect      Effect         // items only: what using one does

	// Ephemeral real-time state, not persisted.
	ReadyAt    time.Time // next moment the unit can move
	ActReadyAt time.Time // next moment the unit can act
	RespawnAt  time.Time // set while the unit is defeated

	BreedReadyAt time.Time // next moment the unit can have offspring
	NextSpawnAt  time.Time // structures: when they next make a unit (if the world allows it)
}

// snapshot copies an entity so another goroutine (the database writer) can read it safely.
func (e *Entity) snapshot() Entity {
	c := *e
	if e.Traits != nil {
		c.Traits = make(map[string]int, len(e.Traits))
		for k, v := range e.Traits {
			c.Traits[k] = v
		}
	}
	return c
}

// Effect is what using an item does. Only the champion uses items (design.md: the shared
// inventory is managed by the champion alone), and the effect falls on the champion.
type Effect struct {
	Heal     int            `json:"heal,omitempty"`     // health restored, up to the maximum
	Points   int            `json:"points,omitempty"`   // points for the team
	Strength int            `json:"strength,omitempty"` // permanent bonus to the champion's strength
	Traits   map[string]int `json:"traits,omitempty"`   // changes to the champion's extended characteristics
}

func (e Effect) IsZero() bool {
	return e.Heal == 0 && e.Points == 0 && e.Strength == 0 && len(e.Traits) == 0
}

// Summary is a short description of the effect for the interface.
func (e Effect) Summary() string {
	var parts []string
	if e.Heal != 0 {
		parts = append(parts, fmt.Sprintf("cura %d", e.Heal))
	}
	if e.Points != 0 {
		parts = append(parts, fmt.Sprintf("%+d punti", e.Points))
	}
	if e.Strength != 0 {
		parts = append(parts, fmt.Sprintf("%+d forza", e.Strength))
	}
	names := make([]string, 0, len(e.Traits))
	for name := range e.Traits {
		names = append(names, name)
	}
	sort.Strings(names)
	for _, name := range names {
		parts = append(parts, fmt.Sprintf("%+d %s", e.Traits[name], name))
	}
	return strings.Join(parts, ", ")
}

// Race is a kind of unit defined by the world's admin: minimum characteristics plus a random
// bonus added on top for each new unit (design.md: inheritance is minimums per race + chance).
type Race struct {
	ID          string
	Name        string
	Description string
	Min         UnitStats
	Bonus       UnitStats // the most that can be added on top of Min, per characteristic
	TraitsMin   map[string]int
	TraitsBonus map[string]int
}

// roll gives a new unit's characteristics: the race's minimums plus a random part.
func (r *Race) roll() (UnitStats, map[string]int) {
	bonus := func(most int) int {
		if most <= 0 {
			return 0
		}
		return rand.IntN(most + 1)
	}
	stats := UnitStats{
		Speed: r.Min.Speed + bonus(r.Bonus.Speed), Health: r.Min.Health + bonus(r.Bonus.Health),
		Vision: r.Min.Vision + bonus(r.Bonus.Vision), Strength: r.Min.Strength + bonus(r.Bonus.Strength),
	}
	traits := map[string]int{}
	for name, min := range r.TraitsMin {
		traits[name] = min + bonus(r.TraitsBonus[name])
	}
	for name, most := range r.TraitsBonus {
		if _, ok := traits[name]; !ok {
			traits[name] = bonus(most)
		}
	}
	return stats, traits
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
	ID          string
	Name        string
	Description string
	Effect      Effect
}

// Player is the roster owner: one player is exactly one team (see design.md).
type Player struct {
	ID             string
	Username       string
	RaceID         string // the race of the team (empty when the world has no races)
	Stats          Stats  // counters goals are measured with
	GoalID         string // the individual goal the player is working on ("" = none)
	CompletedGoals map[string]bool
	Points         int
	Inventory      []Item // shared by the whole team
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

	boards    map[string]*Board
	order     []string // board ids; the first one is where new teams start
	entities  map[string]*Entity
	cells     map[Cell]string
	links     map[Cell]Cell
	players   map[string]*Player
	races     map[string]*Race
	raceIDs   []string             // in the order the admin defined them
	compat    map[[2]string]string // sorted pair of race ids -> race of the offspring
	goals     map[string]*Goal
	goalOrder []string
}

func NewWorld(id, name string) *World {
	return &World{
		ID: id, Name: name, Rules: DefaultRules(),
		boards:   map[string]*Board{},
		entities: map[string]*Entity{},
		cells:    map[Cell]string{},
		links:    map[Cell]Cell{},
		players:  map[string]*Player{},
		races:    map[string]*Race{},
		compat:   map[[2]string]string{},
		goals:    map[string]*Goal{},
	}
}

func (w *World) AddRace(r *Race) {
	w.races[r.ID] = r
	w.raceIDs = append(w.raceIDs, r.ID)
}

func (w *World) Race(id string) *Race { return w.races[id] }

// Races lists the world's races in the order they were defined.
func (w *World) Races() []*Race {
	out := make([]*Race, 0, len(w.raceIDs))
	for _, id := range w.raceIDs {
		out = append(out, w.races[id])
	}
	return out
}

func pairKey(a, b string) [2]string {
	if a > b {
		a, b = b, a
	}
	return [2]string{a, b}
}

// AddCompat says that units of races a and b (in either order) can have offspring of race child.
func (w *World) AddCompat(a, b, child string) { w.compat[pairKey(a, b)] = child }

func (w *World) offspringRace(a, b string) (string, bool) {
	child, ok := w.compat[pairKey(a, b)]
	return child, ok
}

// newMinor makes a minor unit of the given race (the world's default stats when it has none).
func (w *World) newMinor(ownerID, username, raceID string, at Cell, name string) *Entity {
	stats, traits := w.Rules.Minor, map[string]int(nil)
	if r := w.races[raceID]; r != nil {
		stats, traits = r.roll()
	} else {
		raceID = ""
	}
	return &Entity{
		OwnerID: ownerID, BoardID: at.Board, Kind: KindMinor, Name: name, RaceID: raceID,
		Description: fmt.Sprintf("Una fedele pedina della squadra di %s.", username),
		X:           at.X, Y: at.Y, Traits: traits,
		Speed: stats.Speed, Health: stats.Health, MaxHealth: stats.Health, Vision: stats.Vision, Strength: stats.Strength,
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
		p = &Player{ID: id, Username: username, Points: points, CompletedGoals: map[string]bool{}}
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

// TickResult is what changed over time: units back from defeat, and units made by structures.
type TickResult struct {
	Revived []*Entity
	Spawned []*Entity
}

// Tick advances real-time state: defeated units come back after their respawn delay, and (if the
// world allows it) each structure makes a minor unit for its team every building interval.
func (w *World) Tick(now time.Time) TickResult {
	var res TickResult
	interval := w.Rules.BuildingInterval()
	for _, e := range w.entities {
		switch {
		case e.IsUnit() && e.Health <= 0 && !e.RespawnAt.IsZero() && !now.Before(e.RespawnAt):
			e.Health = e.MaxHealth
			e.RespawnAt = time.Time{}
			res.Revived = append(res.Revived, e)
		case e.Kind == KindStructure && interval > 0:
			if e.NextSpawnAt.IsZero() {
				e.NextSpawnAt = now.Add(interval)
			} else if !now.Before(e.NextSpawnAt) {
				e.NextSpawnAt = now.Add(interval)
				owner := w.players[e.OwnerID]
				spot, ok := w.freeNeighbor(e.Cell())
				if owner == nil || !ok {
					continue
				}
				u := w.newMinor(e.OwnerID, owner.Username, owner.RaceID, spot, w.nextMinorName(e.OwnerID))
				u.ID = newID()
				w.Add(u)
				res.Spawned = append(res.Spawned, u)
			}
		}
	}
	return res
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
func (w *World) PlanTeam(playerID, username, raceID string) ([]*Entity, error) {
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

	if w.races[raceID] == nil {
		raceID = ""
	}
	var championTraits map[string]int
	if r := w.races[raceID]; r != nil && len(r.TraitsMin) > 0 {
		championTraits = make(map[string]int, len(r.TraitsMin))
		for name, v := range r.TraitsMin {
			championTraits[name] = v
		}
	}
	team := make([]*Entity, 0, need)
	team = append(team, &Entity{
		OwnerID: playerID, BoardID: spawn.ID, Kind: KindChampion, Name: "Champion", RaceID: raceID,
		Description: fmt.Sprintf("Il campione della squadra di %s: forte, carismatico e convinto di essere indispensabile.", username),
		X:           free[0].p.X, Y: free[0].p.Y, Traits: championTraits,
		Speed: w.Rules.Champion.Speed, Health: w.Rules.Champion.Health, MaxHealth: w.Rules.Champion.Health,
		Vision: w.Rules.Champion.Vision, Strength: w.Rules.Champion.Strength,
	})
	for i := 1; i < need; i++ {
		team = append(team, w.newMinor(playerID, username, raceID, Cell{spawn.ID, free[i].p.X, free[i].p.Y}, fmt.Sprintf("Pedina %d", i)))
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
