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

// Action is a command a player gives to one of their units. The target is an entity id
// (attack, talk, pickup), a cell (build) or both (move_item: which object, to which cell).
// Cells are always on the acting unit's board.
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
// broadcast it. The world has already been updated when an Outcome is returned.
type Outcome struct {
	Changed    []*Entity // to send to clients
	Dirty      []*Entity // units whose persisted state (position, health) changed
	DirtyItems []*Entity // items whose persisted position changed
	Created    []*Entity // new structures and units to insert
	Removed    []string  // entity ids that left the world
	Picked     *Item     // added to the actor's team inventory
	Points     int       // awarded to the actor's team
	Notices    []Notice
}

// Do applies an action if the rules allow it. Every action needs a live unit of the player,
// off cooldown, and (except building on a chosen cell) a target inside the unit's vision.
func (w *World) Do(playerID string, a Action, now time.Time) (*Outcome, error) {
	actor, err := w.ownedUnit(playerID, a.UnitID)
	if err != nil {
		return nil, err
	}
	if now.Before(actor.ActReadyAt) {
		return nil, ErrCooldown
	}

	var out *Outcome
	switch a.Kind {
	case ActionAttack:
		out, err = w.attack(actor, a.TargetID, now)
	case ActionTalk:
		out, err = w.talk(actor, a.TargetID)
	case ActionPickup:
		out, err = w.pickup(actor, a.TargetID, now)
	case ActionBuild:
		out, err = w.build(actor, a.At, now)
	case ActionCreate:
		out, err = w.create(actor, now)
	case ActionMoveItem:
		out, err = w.moveItem(actor, a.TargetID, a.At, now)
	default:
		return nil, ErrUnknownAction
	}
	if err != nil {
		return nil, err
	}
	if out.Points != 0 {
		w.EnsurePlayer(actor.OwnerID, "", 0).Points += out.Points
	}
	return out, nil
}

func (w *World) target(id string) (*Entity, error) {
	t, ok := w.entities[id]
	if !ok {
		return nil, ErrNoTarget
	}
	return t, nil
}

// inRange: the target is on the actor's board and within the actor's vision.
func (w *World) inRange(actor, t *Entity) bool {
	return actor.BoardID == t.BoardID && w.boards[actor.BoardID].Grid.Distance(actor.Point(), t.Point()) <= actor.Vision
}

// cellInRange is inRange for a cell of the actor's board.
func (w *World) cellInRange(actor *Entity, p Point) bool {
	return w.boards[actor.BoardID].Grid.Distance(actor.Point(), p) <= actor.Vision
}

func (w *World) attack(actor *Entity, targetID string, now time.Time) (*Outcome, error) {
	t, err := w.target(targetID)
	if err != nil {
		return nil, err
	}
	if !t.Controllable() || t.OwnerID == actor.OwnerID {
		return nil, ErrInvalidTarget
	}
	if t.Health <= 0 {
		return nil, ErrTargetDead
	}
	if !w.inRange(actor, t) {
		return nil, ErrOutOfRange
	}

	t.Health = max(0, t.Health-max(actor.Strength, 1))
	actor.ActReadyAt = now.Add(w.Rules.AttackCooldown())
	out := &Outcome{Changed: []*Entity{t, actor}, Dirty: []*Entity{t}, Points: w.Rules.Points.Hit}
	if t.Health == 0 {
		t.RespawnAt = now.Add(w.Rules.RespawnDelay())
		if t.Kind == KindChampion {
			out.Points += w.Rules.Points.KillChampion
		} else {
			out.Points += w.Rules.Points.KillMinor
		}
		out.Notices = []Notice{
			{actor.OwnerID, "Nemico sconfitto", fmt.Sprintf("Hai sconfitto %s.", t.Name)},
			{t.OwnerID, "Pedina sconfitta", fmt.Sprintf("%s è fuori gioco per %d secondi.", t.Name, int(w.Rules.RespawnDelay().Seconds()))},
		}
	}
	return out, nil
}

func (w *World) talk(actor *Entity, targetID string) (*Outcome, error) {
	t, err := w.target(targetID)
	if err != nil {
		return nil, err
	}
	if t.Kind != KindNPC {
		return nil, ErrInvalidTarget
	}
	if !w.inRange(actor, t) {
		return nil, ErrOutOfRange
	}
	text := "Ti guarda in silenzio."
	if len(t.Dialogue) > 0 {
		text = t.Dialogue[rand.IntN(len(t.Dialogue))]
	}
	return &Outcome{Notices: []Notice{{actor.OwnerID, t.Name, text}}}, nil
}

func (w *World) pickup(actor *Entity, targetID string, now time.Time) (*Outcome, error) {
	t, err := w.target(targetID)
	if err != nil {
		return nil, err
	}
	if t.Kind != KindItem {
		return nil, ErrInvalidTarget
	}
	if !w.inRange(actor, t) {
		return nil, ErrOutOfRange
	}

	// The item goes straight into the shared team inventory, however far the champion is.
	item := Item{Name: t.Name, Description: t.Description}
	team := w.EnsurePlayer(actor.OwnerID, "", 0)
	team.Inventory = append(team.Inventory, item)
	w.remove(t)
	actor.ActReadyAt = now.Add(w.Rules.PickupCooldown())
	return &Outcome{
		Changed: []*Entity{actor},
		Removed: []string{t.ID},
		Picked:  &item,
		Points:  w.Rules.Points.Pickup,
		Notices: []Notice{{actor.OwnerID, "Oggetto raccolto", fmt.Sprintf("%s è nell'inventario della squadra.", t.Name)}},
	}, nil
}

// build puts a structure on a free cell within the actor's vision: that cell becomes the
// team's territory (design.md: territory is conquered by building, not by standing there).
func (w *World) build(actor *Entity, at Point, now time.Time) (*Outcome, error) {
	board := w.boards[actor.BoardID]
	if !board.InBounds(at) {
		return nil, ErrOutOfBounds
	}
	if w.blocked(Cell{actor.BoardID, at.X, at.Y}) {
		return nil, ErrOccupied
	}
	if !w.cellInRange(actor, at) {
		return nil, ErrOutOfRange
	}

	owner := w.EnsurePlayer(actor.OwnerID, "", 0)
	s := &Entity{
		ID: newID(), BoardID: actor.BoardID, OwnerID: actor.OwnerID, Kind: KindStructure, Name: "Avamposto",
		Description: fmt.Sprintf("Un avamposto di %s: questa casella è territorio della sua squadra.", owner.Username),
		X:           at.X, Y: at.Y,
	}
	w.Add(s)
	actor.ActReadyAt = now.Add(w.Rules.BuildCooldown())
	return &Outcome{
		Changed: []*Entity{s, actor},
		Created: []*Entity{s},
		Points:  w.Rules.Points.Build,
	}, nil
}

// create makes a new minor unit on a free cell next to the champion, paid with the champion's health.
func (w *World) create(actor *Entity, now time.Time) (*Outcome, error) {
	if actor.Kind != KindChampion {
		return nil, ErrChampionOnly
	}
	if actor.Health <= w.Rules.CreateHealthCost {
		return nil, ErrTooWeak
	}
	spot, ok := w.freeNeighbor(actor.Cell())
	if !ok {
		return nil, ErrNoSpace
	}

	minors := 0
	for _, e := range w.entities {
		if e.OwnerID == actor.OwnerID && e.Kind == KindMinor {
			minors++
		}
	}
	owner := w.EnsurePlayer(actor.OwnerID, "", 0)
	u := &Entity{
		ID: newID(), BoardID: spot.Board, OwnerID: actor.OwnerID, Kind: KindMinor, Name: fmt.Sprintf("Pedina %d", minors+1),
		Description: fmt.Sprintf("Una fedele pedina della squadra di %s.", owner.Username),
		X:           spot.X, Y: spot.Y,
		Speed: w.Rules.Minor.Speed, Health: w.Rules.Minor.Health, MaxHealth: w.Rules.Minor.Health,
		Vision: w.Rules.Minor.Vision, Strength: w.Rules.Minor.Strength,
	}
	w.Add(u)
	actor.Health -= w.Rules.CreateHealthCost
	actor.ActReadyAt = now.Add(w.Rules.CreateCooldown())
	return &Outcome{
		Changed: []*Entity{u, actor},
		Dirty:   []*Entity{actor},
		Created: []*Entity{u},
		Points:  w.Rules.Points.Create,
		Notices: []Notice{{actor.OwnerID, "Nuova pedina", fmt.Sprintf("%s si è unita alla squadra (il campione perde %d vita).", u.Name, w.Rules.CreateHealthCost)}},
	}, nil
}

// moveItem pushes an object to a free cell. Both the object and the destination must be
// within the unit's vision.
func (w *World) moveItem(actor *Entity, targetID string, to Point, now time.Time) (*Outcome, error) {
	t, err := w.target(targetID)
	if err != nil {
		return nil, err
	}
	if t.Kind != KindItem {
		return nil, ErrInvalidTarget
	}
	if !w.inRange(actor, t) {
		return nil, ErrOutOfRange
	}
	if !w.boards[t.BoardID].InBounds(to) {
		return nil, ErrOutOfBounds
	}
	if to == t.Point() {
		return nil, ErrSameCell
	}
	dest := Cell{t.BoardID, to.X, to.Y}
	if w.blocked(dest) {
		return nil, ErrOccupied
	}
	if !w.cellInRange(actor, to) {
		return nil, ErrOutOfRange
	}

	delete(w.cells, t.Cell())
	w.cells[dest] = t.ID
	t.X, t.Y = to.X, to.Y
	actor.ActReadyAt = now.Add(w.Rules.MoveItemCooldown())
	return &Outcome{Changed: []*Entity{t, actor}, DirtyItems: []*Entity{t}}, nil
}
