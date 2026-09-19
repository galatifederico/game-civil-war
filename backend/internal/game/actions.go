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
)

const (
	RespawnDelay   = 10 * time.Second
	AttackCooldown = 1500 * time.Millisecond
	PickupCooldown = 500 * time.Millisecond
	BuildCooldown  = 3 * time.Second

	PointsPerHit       = 5
	PointsKillMinor    = 25
	PointsKillChampion = 100 // design.md: taking out a champion is worth a big bonus, not a win
	PointsPickup       = 5
	PointsBuild        = 20
)

// Action is a command a player gives to one of their units. The target is an entity id
// (attack, talk, pickup) or a cell (build).
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
	Created []*Entity // new structures to insert
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
