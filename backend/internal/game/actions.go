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
	// ActionBreed: two of the player's units, side by side, have offspring (explicit command).
	ActionBreed ActionKind = "breed"
	// ActionUseItem: the champion uses an object from the shared inventory.
	ActionUseItem ActionKind = "use_item"
)

// Action is a command a player gives to one of their units. The target is an entity id
// (attack, talk, pickup, breed: the partner), an inventory item id (use_item), a cell (build) or
// both (move_item: which object, to which cell). Cells are always on the acting unit's board.
type Action struct {
	Kind     ActionKind
	UnitID   string
	TargetID string
	At       Point
	Method   string // create: "health" (default) or "resources"
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
	// RemovedItems are inventory items that were used up; InventoryChanged asks for the player's
	// inventory to be sent again.
	RemovedItems     []string
	InventoryChanged bool
	Points           int // awarded to the actor's team
	Notices          []Notice

	// Goals and counters (see goals.go).
	StatsChanged   bool      // the actor's counters changed and must be saved
	GoalsChanged   bool      // someone's goals changed: goals are sent again
	AssignedGoals  []GoalRef // new individual goals to save
	CompletedGoals []GoalRef // completed individual goals to save
	Victories      []GoalRef // world goals won for the first time
	Broadcast      []Notice  // announcements for every connected player
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
		out, err = w.create(actor, a.Method, now)
	case ActionBreed:
		out, err = w.breed(actor, a.TargetID, now)
	case ActionUseItem:
		out, err = w.useItem(actor, a.TargetID, now)
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
	out.merge(w.Evaluate(actor.OwnerID))
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
	if owner := w.players[t.OwnerID]; owner != nil && owner.AFK && w.Rules.AFK.Shield {
		return nil, ErrTargetAFK
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
		killer := w.EnsurePlayer(actor.OwnerID, "", 0)
		killer.Stats.Kills++
		if t.Kind == KindChampion {
			killer.Stats.ChampionKills++
		}
		out.StatsChanged = true
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
	out := &Outcome{Notices: []Notice{{actor.OwnerID, t.Name, text}}}
	talker := w.EnsurePlayer(actor.OwnerID, "", 0)
	if talker.Stats.Talked == nil {
		talker.Stats.Talked = map[string]bool{}
	}
	if !talker.Stats.Talked[t.ID] {
		talker.Stats.Talked[t.ID] = true
		out.StatsChanged = true
	}
	return out, nil
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
	item := Item{ID: newID(), Name: t.Name, Description: t.Description, Effect: t.Effect, Icon: t.Icon}
	team := w.EnsurePlayer(actor.OwnerID, "", 0)
	team.Inventory = append(team.Inventory, item)
	team.Stats.Pickups++
	w.remove(t)
	actor.ActReadyAt = now.Add(w.Rules.PickupCooldown())
	return &Outcome{
		Changed:          []*Entity{actor},
		Removed:          []string{t.ID},
		Picked:           &item,
		InventoryChanged: true,
		StatsChanged:     true,
		Points:           w.Rules.Points.Pickup,
		Notices:          []Notice{{actor.OwnerID, "Oggetto raccolto", fmt.Sprintf("%s è nell'inventario della squadra.", t.Name)}},
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

// create makes a new minor unit on a free cell next to the champion, and only the champion can do
// it. The method says what it costs, and the world decides which methods are open: "health" (the
// champion pays hit points) or "resources" (it consumes items from the shared inventory).
func (w *World) create(actor *Entity, method string, now time.Time) (*Outcome, error) {
	if actor.Kind != KindChampion {
		return nil, ErrChampionOnly
	}
	owner := w.EnsurePlayer(actor.OwnerID, "", 0)
	var cost string
	var consumed []Item
	switch method {
	case "", "health":
		if !w.Rules.Creation.HealthEnabled {
			return nil, ErrDisabled
		}
		if actor.Health <= w.Rules.CreateHealthCost {
			return nil, ErrTooWeak
		}
		cost = fmt.Sprintf("il campione perde %d vita", w.Rules.CreateHealthCost)
	case "resources":
		n := w.Rules.Creation.ResourceItems
		if n <= 0 {
			return nil, ErrDisabled
		}
		if len(owner.Inventory) < n {
			return nil, ErrNoItems
		}
		consumed = owner.Inventory[:n]
		cost = fmt.Sprintf("consumati %d oggetti dell'inventario", n)
	default:
		return nil, ErrUnknownAction
	}
	spot, ok := w.freeNeighbor(actor.Cell())
	if !ok {
		return nil, ErrNoSpace
	}

	u := w.newMinor(actor.OwnerID, owner.Username, w.teamRace(actor), spot, w.nextMinorName(actor.OwnerID))
	u.ID = newID()
	w.Add(u)
	actor.ActReadyAt = now.Add(w.Rules.CreateCooldown())
	out := &Outcome{
		Changed: []*Entity{u, actor},
		Created: []*Entity{u},
		Points:  w.Rules.Points.Create,
		Notices: []Notice{{actor.OwnerID, "Nuova pedina", fmt.Sprintf("%s si è unita alla squadra (%s).", u.Name, cost)}},
	}
	if consumed != nil {
		for _, it := range consumed {
			out.RemovedItems = append(out.RemovedItems, it.ID)
		}
		owner.Inventory = append([]Item(nil), owner.Inventory[len(consumed):]...)
		out.InventoryChanged = true
	} else {
		actor.Health -= w.Rules.CreateHealthCost
		out.Dirty = []*Entity{actor}
	}
	return out, nil
}

func (w *World) teamRace(actor *Entity) string {
	if p := w.players[actor.OwnerID]; p != nil && p.RaceID != "" {
		return p.RaceID
	}
	return actor.RaceID
}

func (w *World) nextMinorName(ownerID string) string {
	minors := 0
	for _, e := range w.entities {
		if e.OwnerID == ownerID && e.Kind == KindMinor {
			minors++
		}
	}
	return fmt.Sprintf("Pedina %d", minors+1)
}

// breed: two of the player's units next to each other have offspring, if their races are
// compatible in this world. The offspring is a new minor unit of the race the compatibility says,
// with that race's minimum characteristics plus a random bonus. Both parents then need to rest.
func (w *World) breed(actor *Entity, partnerID string, now time.Time) (*Outcome, error) {
	if !w.Rules.Creation.BreedingEnabled {
		return nil, ErrDisabled
	}
	partner, ok := w.entities[partnerID]
	if !ok {
		return nil, ErrNoTarget
	}
	if partner == actor || !partner.Controllable() || partner.OwnerID != actor.OwnerID {
		return nil, ErrInvalidTarget
	}
	if partner.Health <= 0 {
		return nil, ErrTargetDead
	}
	if actor.BoardID != partner.BoardID ||
		w.boards[actor.BoardID].Grid.Distance(actor.Point(), partner.Point()) > w.Rules.Creation.BreedRange {
		return nil, ErrOutOfRange
	}
	if now.Before(actor.BreedReadyAt) || now.Before(partner.BreedReadyAt) {
		return nil, ErrNotRested
	}
	childRace, ok := w.offspringRace(actor.RaceID, partner.RaceID)
	if !ok {
		return nil, ErrIncompatible
	}
	spot, ok := w.freeNeighbor(actor.Cell())
	if !ok {
		if spot, ok = w.freeNeighbor(partner.Cell()); !ok {
			return nil, ErrNoSpace
		}
	}

	owner := w.EnsurePlayer(actor.OwnerID, "", 0)
	child := w.newMinor(actor.OwnerID, owner.Username, childRace, spot, w.nextMinorName(actor.OwnerID))
	child.ID = newID()
	w.Add(child)
	actor.BreedReadyAt = now.Add(w.Rules.BreedCooldown())
	partner.BreedReadyAt = actor.BreedReadyAt
	raceName := childRace
	if r := w.races[childRace]; r != nil {
		raceName = r.Name
	}
	return &Outcome{
		Changed: []*Entity{child},
		Created: []*Entity{child},
		Points:  w.Rules.Points.Breed,
		Notices: []Notice{{actor.OwnerID, "Nuova pedina", fmt.Sprintf("%s e %s hanno avuto un figlio: %s (%s).", actor.Name, partner.Name, child.Name, raceName)}},
	}, nil
}

// useItem: the champion uses an object from the shared inventory; it is used up and its effect
// falls on the champion (or on the team's points).
func (w *World) useItem(actor *Entity, itemID string, now time.Time) (*Outcome, error) {
	if actor.Kind != KindChampion {
		return nil, ErrChampionOnly
	}
	owner := w.EnsurePlayer(actor.OwnerID, "", 0)
	at := -1
	for i, it := range owner.Inventory {
		if it.ID == itemID {
			at = i
		}
	}
	if at < 0 {
		return nil, ErrNoItem
	}
	item := owner.Inventory[at]
	if item.Effect.IsZero() {
		return nil, ErrNoEffect
	}

	fx := item.Effect
	actor.Health = min(actor.MaxHealth, actor.Health+fx.Heal)
	actor.Strength += fx.Strength
	if len(fx.Traits) > 0 && actor.Traits == nil {
		actor.Traits = map[string]int{}
	}
	for name, delta := range fx.Traits {
		actor.Traits[name] += delta
	}
	owner.Inventory = append(append([]Item(nil), owner.Inventory[:at]...), owner.Inventory[at+1:]...)
	actor.ActReadyAt = now.Add(w.Rules.PickupCooldown())
	return &Outcome{
		Changed:          []*Entity{actor},
		Dirty:            []*Entity{actor},
		RemovedItems:     []string{item.ID},
		InventoryChanged: true,
		Points:           fx.Points,
		Notices:          []Notice{{actor.OwnerID, "Oggetto usato", fmt.Sprintf("%s: %s.", item.Name, fx.Summary())}},
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
