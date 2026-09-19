package game

import (
	"fmt"
	"math/rand/v2"
	"time"
)

type ActionKind string

const (
	ActionAttack ActionKind = "attack"
	ActionTalk   ActionKind = "talk"
	ActionPickup ActionKind = "pickup"
	ActionBuild  ActionKind = "build"
	ActionCreate ActionKind = "create"
	// ActionMoveItem pushes an object to another cell within the unit's reach.
	ActionMoveItem ActionKind = "move_item"
)

const (
	RespawnDelay   = 10 * time.Second
	AttackCooldown = 1500 * time.Millisecond
	PickupCooldown = 500 * time.Millisecond
	BuildCooldown  = 3 * time.Second
	CreateCooldown = 5 * time.Second
	MoveItemCooldown = 500 * time.Millisecond

	// The champion pays with its own health to bring a new minor unit into the team
	// (design.md: creation by consuming the champion's characteristics).
	CreateHealthCost = 20

	PointsPerHit       = 5
	PointsKillMinor    = 25
	PointsKillChampion = 100 // design.md: taking out a champion is worth a big bonus, not a win
	PointsPickup       = 5
	PointsBuild        = 20
	PointsCreate       = 10
)

// Action is a command a player gives to one of their units. The target is an entity id
// (attack, talk, pickup), a cell (build) or both (move_item: which object, to which cell).
type Action struct {
	Kind     ActionKind
	UnitID   string
	TargetID string
	At       Point
}

// Notice is a message for one player (a line of NPC dialogue, a defeat, a pickup...).
type Notice struct {
	PlayerID string
	Title    string
	Text     string
}

// Outcome is everything that changed because of an action, so the caller can persist and
// broadcast it. The board has already been updated when an Outcome is returned.
type Outcome struct {
	Changed []*Entity // to send to clients
	Dirty   []*Entity // units whose persisted state (position, health) changed
	DirtyItems []*Entity // items whose persisted position changed
	Created []*Entity // new structures and units to insert
	Removed []string  // entity ids that left the board
	Picked  *Item     // added to the actor's team inventory
	Points  int       // awarded to the actor's team
	Notices []Notice
}

// Do applies an action if the rules allow it. Every action needs a live unit of the player,
// off cooldown, and (except building on a chosen cell) a target inside the unit's vision.
func (b *Board) Do(playerID string, a Action, now time.Time) (*Outcome, error) {
	actor, err := b.ownedUnit(playerID, a.UnitID)
	if err != nil {
		return nil, err
	}
	if now.Before(actor.ActReadyAt) {
		return nil, ErrCooldown
	}

	var out *Outcome
	switch a.Kind {
	case ActionAttack:
		out, err = b.attack(actor, a.TargetID, now)
	case ActionTalk:
		out, err = b.talk(actor, a.TargetID)
	case ActionPickup:
		out, err = b.pickup(actor, a.TargetID, now)
	case ActionBuild:
		out, err = b.build(actor, a.At, now)
	case ActionCreate:
		out, err = b.create(actor, now)
	case ActionMoveItem:
		out, err = b.moveItem(actor, a.TargetID, a.At, now)
	default:
		return nil, ErrUnknownAction
	}
	if err != nil {
		return nil, err
	}
	if out.Points != 0 {
		b.EnsurePlayer(actor.OwnerID, "", 0).Points += out.Points
	}
	return out, nil
}

func (b *Board) target(id string) (*Entity, error) {
	t, ok := b.entities[id]
	if !ok {
		return nil, ErrNoTarget
	}
	return t, nil
}

func inRange(actor, t *Entity) bool {
	return distance(Point{actor.X, actor.Y}, Point{t.X, t.Y}) <= actor.Vision
}

func (b *Board) attack(actor *Entity, targetID string, now time.Time) (*Outcome, error) {
	t, err := b.target(targetID)
	if err != nil {
		return nil, err
	}
	if !t.Controllable() || t.OwnerID == actor.OwnerID {
		return nil, ErrInvalidTarget
	}
	if t.Health <= 0 {
		return nil, ErrTargetDead
	}
	if !inRange(actor, t) {
		return nil, ErrOutOfRange
	}

	t.Health = max(0, t.Health-max(actor.Strength, 1))
	actor.ActReadyAt = now.Add(AttackCooldown)
	out := &Outcome{Changed: []*Entity{t, actor}, Dirty: []*Entity{t}, Points: PointsPerHit}
	if t.Health == 0 {
		t.RespawnAt = now.Add(RespawnDelay)
		if t.Kind == KindChampion {
			out.Points += PointsKillChampion
		} else {
			out.Points += PointsKillMinor
		}
		out.Notices = []Notice{
			{actor.OwnerID, "Nemico sconfitto", fmt.Sprintf("Hai sconfitto %s.", t.Name)},
			{t.OwnerID, "Pedina sconfitta", fmt.Sprintf("%s è fuori gioco per %d secondi.", t.Name, int(RespawnDelay.Seconds()))},
		}
	}
	return out, nil
}

func (b *Board) talk(actor *Entity, targetID string) (*Outcome, error) {
	t, err := b.target(targetID)
	if err != nil {
		return nil, err
	}
	if t.Kind != KindNPC {
		return nil, ErrInvalidTarget
	}
	if !inRange(actor, t) {
		return nil, ErrOutOfRange
	}
	text := "Ti guarda in silenzio."
	if len(t.Dialogue) > 0 {
		text = t.Dialogue[rand.IntN(len(t.Dialogue))]
	}
	return &Outcome{Notices: []Notice{{actor.OwnerID, t.Name, text}}}, nil
}

func (b *Board) pickup(actor *Entity, targetID string, now time.Time) (*Outcome, error) {
	t, err := b.target(targetID)
	if err != nil {
		return nil, err
	}
	if t.Kind != KindItem {
		return nil, ErrInvalidTarget
	}
	if !inRange(actor, t) {
		return nil, ErrOutOfRange
	}

	// The item goes straight into the shared team inventory, however far the champion is.
	item := Item{Name: t.Name, Description: t.Description}
	team := b.EnsurePlayer(actor.OwnerID, "", 0)
	team.Inventory = append(team.Inventory, item)
	b.remove(t)
	actor.ActReadyAt = now.Add(PickupCooldown)
	return &Outcome{
		Changed: []*Entity{actor},
		Removed: []string{t.ID},
		Picked:  &item,
		Points:  PointsPickup,
		Notices: []Notice{{actor.OwnerID, "Oggetto raccolto", fmt.Sprintf("%s è nell'inventario della squadra.", t.Name)}},
	}, nil
}

// build puts a structure on a free cell within the actor's vision: that cell becomes the
// team's territory (design.md: territory is conquered by building, not by standing there).
func (b *Board) build(actor *Entity, at Point, now time.Time) (*Outcome, error) {
	if !b.inBounds(at) {
		return nil, ErrOutOfBounds
	}
	if _, taken := b.cells[at]; taken {
		return nil, ErrOccupied
	}
	if distance(Point{actor.X, actor.Y}, at) > actor.Vision {
		return nil, ErrOutOfRange
	}

	owner := b.EnsurePlayer(actor.OwnerID, "", 0)
	s := &Entity{
		ID: newID(), OwnerID: actor.OwnerID, Kind: KindStructure, Name: "Avamposto",
		Description: fmt.Sprintf("Un avamposto di %s: questa casella è territorio della sua squadra.", owner.Username),
		X:           at.X, Y: at.Y,
	}
	b.Add(s)
	actor.ActReadyAt = now.Add(BuildCooldown)
	return &Outcome{
		Changed: []*Entity{s, actor},
		Created: []*Entity{s},
		Points:  PointsBuild,
	}, nil
}

// create makes a new minor unit on a free cell next to the champion, paid with the champion's health.
func (b *Board) create(actor *Entity, now time.Time) (*Outcome, error) {
	if actor.Kind != KindChampion {
		return nil, ErrChampionOnly
	}
	if actor.Health <= CreateHealthCost {
		return nil, ErrTooWeak
	}
	spot, ok := b.freeNeighbor(Point{actor.X, actor.Y})
	if !ok {
		return nil, ErrNoSpace
	}

	minors := 0
	for _, e := range b.entities {
		if e.OwnerID == actor.OwnerID && e.Kind == KindMinor {
			minors++
		}
	}
	owner := b.EnsurePlayer(actor.OwnerID, "", 0)
	u := &Entity{
		ID: newID(), OwnerID: actor.OwnerID, Kind: KindMinor, Name: fmt.Sprintf("Pedina %d", minors+1),
		Description: fmt.Sprintf("Una fedele pedina della squadra di %s.", owner.Username),
		X:           spot.X, Y: spot.Y,
		Speed: minorSpeed, Health: minorHealth, MaxHealth: minorHealth, Vision: minorVision, Strength: minorStrength,
	}
	b.Add(u)
	actor.Health -= CreateHealthCost
	actor.ActReadyAt = now.Add(CreateCooldown)
	return &Outcome{
		Changed: []*Entity{u, actor},
		Dirty:   []*Entity{actor},
		Created: []*Entity{u},
		Points:  PointsCreate,
		Notices: []Notice{{actor.OwnerID, "Nuova pedina", fmt.Sprintf("%s si è unita alla squadra (il campione perde %d vita).", u.Name, CreateHealthCost)}},
	}, nil
}

func (b *Board) freeNeighbor(p Point) (Point, bool) {
	for dy := -1; dy <= 1; dy++ {
		for dx := -1; dx <= 1; dx++ {
			c := Point{p.X + dx, p.Y + dy}
			if c == p || !b.inBounds(c) {
				continue
			}
			if _, taken := b.cells[c]; !taken {
				return c, true
			}
		}
	}
	return Point{}, false
}

// moveItem pushes an object to a free cell. Both the object and the destination must be
// within the unit's vision.
func (b *Board) moveItem(actor *Entity, targetID string, to Point, now time.Time) (*Outcome, error) {
	t, err := b.target(targetID)
	if err != nil {
		return nil, err
	}
	if t.Kind != KindItem {
		return nil, ErrInvalidTarget
	}
	if !inRange(actor, t) {
		return nil, ErrOutOfRange
	}
	if !b.inBounds(to) {
		return nil, ErrOutOfBounds
	}
	from := Point{t.X, t.Y}
	if to == from {
		return nil, ErrSameCell
	}
	if _, taken := b.cells[to]; taken {
		return nil, ErrOccupied
	}
	if distance(Point{actor.X, actor.Y}, to) > actor.Vision {
		return nil, ErrOutOfRange
	}

	delete(b.cells, from)
	b.cells[to] = t.ID
	t.X, t.Y = to.X, to.Y
	actor.ActReadyAt = now.Add(MoveItemCooldown)
	return &Outcome{Changed: []*Entity{t, actor}, DirtyItems: []*Entity{t}}, nil
}
