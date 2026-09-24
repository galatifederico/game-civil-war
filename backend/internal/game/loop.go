package game

import (
	"context"
	"encoding/json"
	"log"
	"runtime/debug"
	"time"

	"thegame/backend/internal/protocol"
)

const tickInterval = 250 * time.Millisecond

// Persister is what the simulation needs from the database.
type Persister interface {
	InsertUnits(ctx context.Context, units []*Entity) error
	SaveUnit(ctx context.Context, e Entity) error
	InsertStructure(ctx context.Context, s *Entity) error
	SaveItemPosition(ctx context.Context, id string, x, y int) error
	DeleteItem(ctx context.Context, id string) error
	AddInventory(ctx context.Context, worldID, playerID string, item Item) error
	DeleteInventoryItems(ctx context.Context, ids []string) error
	SaveStats(ctx context.Context, worldID, playerID string, stats Stats) error
	AssignGoal(ctx context.Context, playerID, goalID string) error
	CompleteGoal(ctx context.Context, playerID, goalID string) error
	SetGoalAchieved(ctx context.Context, goalID, playerID string) error
	AddPoints(ctx context.Context, worldID, playerID string, delta int) error
}

// Client is a connected player. Send is closed by the loop when the client is dropped.
type Client struct {
	PlayerID string
	Send     chan []byte
}

// Loop owns the world: every read and write of its state happens in the Run goroutine,
// so the rules need no locking. Other goroutines talk to it through do/call. Database writes
// are queued and executed in order by a separate worker, so they never block the simulation.
type Loop struct {
	world   *World
	store   Persister
	cmds    chan func()
	clients map[*Client]struct{}
	seen    map[string]map[string]struct{} // per connected player: the entity ids their client has
	writes  chan func(context.Context) error
	now     func() time.Time
	done    chan struct{}

	stopWrites chan struct{} // closed when the loop stops: the writer drains the queue and ends
}

func NewLoop(world *World, store Persister) *Loop {
	return &Loop{
		world:   world,
		store:   store,
		cmds:    make(chan func(), 256),
		clients: map[*Client]struct{}{},
		seen:    map[string]map[string]struct{}{},
		writes:  make(chan func(context.Context) error, 1024),
		now:     time.Now,
		done:    make(chan struct{}),

		stopWrites: make(chan struct{}),
	}
}

// Done is closed once the loop has stopped and every queued database write has been made.
func (l *Loop) Done() <-chan struct{} { return l.done }

func (l *Loop) Run(ctx context.Context) {
	writerDone := make(chan struct{})
	go l.writeWorker(ctx, writerDone)
	defer func() {
		// Nothing queues writes any more: let the writer empty the queue before saying we are done,
		// so whoever restarts this world reads everything that happened in it.
		close(l.stopWrites)
		<-writerDone
		close(l.done)
	}()
	l.world.ScheduleRespawns(l.now())
	l.world.StartActivity(l.now())

	ticker := time.NewTicker(tickInterval)
	defer ticker.Stop()
	for {
		select {
		case f := <-l.cmds:
			l.guarded(f)
		case <-ticker.C:
			l.guarded(l.tick)
		case <-ctx.Done():
			for c := range l.clients {
				l.drop(c)
			}
			return
		}
	}
}

// guarded runs one piece of work and survives a panic in it: one bad command must not take the
// whole world (and everybody playing in it) down.
func (l *Loop) guarded(f func()) {
	defer func() {
		if r := recover(); r != nil {
			log.Printf("world %s: recovered from panic: %v\n%s", l.world.ID, r, debug.Stack())
		}
	}()
	f()
}

func (l *Loop) tick() {
	now := l.now()
	if l.world.UpdateAFK(now) {
		l.sync(nil, true) // the scoreboard shows who is away
	}
	res := l.world.Tick(now)
	if len(res.Revived) == 0 && len(res.Spawned) == 0 {
		return
	}
	for _, e := range res.Revived {
		l.saveUnit(e)
	}
	for _, u := range res.Spawned {
		l.insertUnit(u)
		l.sendToPlayer(u.OwnerID, protocol.ServerMessage{Type: protocol.TypeEvent, Title: "Nuova pedina", Message: u.Name + " è stata generata da un tuo avamposto."})
		if ev := l.world.Evaluate(u.OwnerID); ev != nil {
			l.publish(u.OwnerID, ev)
		}
	}
	l.sync(append(res.Revived, res.Spawned...), false)
}

// writeWorker makes the queued database writes in order. It keeps going after ctx is cancelled
// (the writes are the world's last moments, so they must not fail with "context canceled") and
// stops only when the loop asks it to and the queue is empty.
func (l *Loop) writeWorker(ctx context.Context, done chan<- struct{}) {
	defer close(done)
	run := func(write func(context.Context) error) {
		wctx, cancel := context.WithTimeout(context.WithoutCancel(ctx), 10*time.Second)
		defer cancel()
		if err := write(wctx); err != nil {
			log.Printf("database write: %v", err)
		}
	}
	for {
		select {
		case write := <-l.writes:
			run(write)
		case <-l.stopWrites:
			for {
				select {
				case write := <-l.writes:
					run(write)
				default:
					return
				}
			}
		}
	}
}

// persist queues a database write; it must not block the simulation, so a full queue drops it.
func (l *Loop) persist(write func(context.Context) error) {
	select {
	case l.writes <- write:
	default:
		log.Printf("write queue full, dropping a database write")
	}
}

func (l *Loop) saveUnit(e *Entity) {
	saved := e.snapshot()
	l.persist(func(ctx context.Context) error { return l.store.SaveUnit(ctx, saved) })
}

func (l *Loop) insertUnit(e *Entity) {
	saved := e.snapshot()
	l.persist(func(ctx context.Context) error { return l.store.InsertUnits(ctx, []*Entity{&saved}) })
}

func (l *Loop) do(f func()) bool {
	select {
	case l.cmds <- f:
		return true
	case <-l.done:
		return false
	}
}

func (l *Loop) call(f func() error) error {
	res := make(chan error, 1)
	if !l.do(func() { res <- f() }) {
		return ErrStopped
	}
	select {
	case err := <-res:
		return err
	case <-l.done:
		return ErrStopped
	}
}

// EnsureTeam creates the player's team the first time it is needed; later calls do nothing.
func (l *Loop) EnsureTeam(ctx context.Context, playerID, username, raceID string) error {
	return l.call(func() error {
		if l.world.HasUnitsOf(playerID) {
			return nil
		}
		if p := l.world.Player(playerID); p != nil && p.RaceID != "" {
			raceID = p.RaceID // the race chosen when joining sticks
		}
		team, err := l.world.PlanTeam(playerID, username, raceID)
		if err != nil {
			return err
		}
		if err := l.store.InsertUnits(ctx, team); err != nil {
			return err
		}
		player := l.world.EnsurePlayer(playerID, username, 0)
		if player.RaceID == "" && l.world.Race(raceID) != nil {
			player.RaceID = raceID
		}
		for _, e := range team {
			l.world.Add(e)
		}
		l.sync(team, true)
		return nil
	})
}

// touch records that a player did something; if it brings them back from afk everybody's scoreboard changes.
func (l *Loop) touch(playerID string) {
	if l.world.Touch(playerID, l.now()) {
		l.sync(nil, true)
	}
}

// AssignGoal is the admin giving a player an individual goal: it replaces the one they had, and
// the player sees it at once if they are connected.
func (l *Loop) AssignGoal(playerID, goalID string) error {
	return l.call(func() error {
		if err := l.world.AssignGoal(playerID, goalID); err != nil {
			return err
		}
		l.persist(func(ctx context.Context) error { return l.store.AssignGoal(ctx, playerID, goalID) })
		l.sendGoals(playerID)
		return nil
	})
}

func (l *Loop) Register(c *Client) {
	l.do(func() {
		l.clients[c] = struct{}{}
		l.touch(c.PlayerID)
		if ref := l.world.EnsureGoal(c.PlayerID); ref != nil {
			l.persist(func(ctx context.Context) error { return l.store.AssignGoal(ctx, ref.PlayerID, ref.GoalID) })
		}
		visible := l.world.VisibleTo(c.PlayerID)
		l.seen[c.PlayerID] = visible
		entities := make([]*Entity, 0, len(visible))
		for id := range visible {
			entities = append(entities, l.world.entities[id])
		}
		l.send(c, protocol.ServerMessage{
			Type:         protocol.TypeSnapshot,
			Boards:       l.boardDTOs(),
			YourPlayerID: c.PlayerID,
			Entities:     l.dtos(entities),
			Scores:       l.scores(),
			Inventory:    l.inventory(c.PlayerID),
			Goals:        l.goalDTOs(c.PlayerID),
		})
	})
}

func (l *Loop) Unregister(c *Client) {
	l.do(func() {
		if _, ok := l.clients[c]; ok {
			l.drop(c)
		}
		l.touch(c.PlayerID) // being away is counted from the moment they left
	})
}

// Reject tells a client that one of its commands was refused before it reached the world.
func (l *Loop) Reject(c *Client, err error) {
	l.do(func() { l.sendError(c, err) })
}

func (l *Loop) Move(c *Client, unitID string, to Point) {
	l.do(func() {
		l.touch(c.PlayerID)
		e, err := l.world.Move(c.PlayerID, unitID, to, l.now())
		if err != nil {
			l.sendError(c, err)
			return
		}
		l.saveUnit(e)
		l.sync([]*Entity{e}, false)
	})
}

func (l *Loop) Act(c *Client, a Action) {
	l.do(func() {
		l.touch(c.PlayerID)
		out, err := l.world.Do(c.PlayerID, a, l.now())
		if err != nil {
			l.sendError(c, err)
			return
		}
		l.publish(c.PlayerID, out)
	})
}

// publish persists an action's outcome and tells the clients what changed.
func (l *Loop) publish(playerID string, out *Outcome) {
	worldID := l.world.ID
	for _, e := range out.Dirty {
		l.saveUnit(e)
	}
	for _, item := range out.DirtyItems {
		id, x, y := item.ID, item.X, item.Y
		l.persist(func(ctx context.Context) error { return l.store.SaveItemPosition(ctx, id, x, y) })
	}
	for _, created := range out.Created {
		// The worker runs concurrently: give it a copy, never the live entity.
		e := *created
		if e.Kind == KindStructure {
			l.persist(func(ctx context.Context) error { return l.store.InsertStructure(ctx, &e) })
		} else {
			l.persist(func(ctx context.Context) error { return l.store.InsertUnits(ctx, []*Entity{&e}) })
		}
	}
	for _, id := range out.Removed {
		id := id
		l.persist(func(ctx context.Context) error { return l.store.DeleteItem(ctx, id) })
	}
	if out.Picked != nil {
		item := *out.Picked
		l.persist(func(ctx context.Context) error { return l.store.AddInventory(ctx, worldID, playerID, item) })
	}
	if len(out.RemovedItems) > 0 {
		ids := append([]string(nil), out.RemovedItems...)
		l.persist(func(ctx context.Context) error { return l.store.DeleteInventoryItems(ctx, ids) })
	}
	if out.Points != 0 {
		points := out.Points
		l.persist(func(ctx context.Context) error { return l.store.AddPoints(ctx, worldID, playerID, points) })
	}
	if out.StatsChanged {
		l.saveStats(playerID)
	}
	for _, ref := range out.AssignedGoals {
		ref := ref
		l.persist(func(ctx context.Context) error { return l.store.AssignGoal(ctx, ref.PlayerID, ref.GoalID) })
	}
	for _, ref := range out.CompletedGoals {
		ref := ref
		l.persist(func(ctx context.Context) error { return l.store.CompleteGoal(ctx, ref.PlayerID, ref.GoalID) })
	}
	for _, ref := range out.Victories {
		ref := ref
		l.persist(func(ctx context.Context) error { return l.store.SetGoalAchieved(ctx, ref.GoalID, ref.PlayerID) })
	}

	if len(out.Changed) > 0 || len(out.Removed) > 0 || out.Points != 0 {
		l.sync(out.Changed, out.Points != 0)
	}
	if out.InventoryChanged {
		l.sendToPlayer(playerID, protocol.ServerMessage{Type: protocol.TypeInventory, Inventory: l.inventory(playerID)})
	}
	for _, n := range out.Notices {
		l.sendToPlayer(n.PlayerID, protocol.ServerMessage{Type: protocol.TypeEvent, Title: n.Title, Message: n.Text})
	}
	for _, n := range out.Broadcast {
		for _, id := range l.connectedPlayers() {
			l.sendToPlayer(id, protocol.ServerMessage{Type: protocol.TypeEvent, Title: n.Title, Message: n.Text})
		}
	}
	// A world goal won by someone changes the goals of everyone; otherwise only the actor's move.
	switch {
	case len(out.Victories) > 0:
		for _, id := range l.connectedPlayers() {
			l.sendGoals(id)
		}
	case out.GoalsChanged || out.StatsChanged || out.Points != 0:
		l.sendGoals(playerID)
	}
}

func (l *Loop) saveStats(playerID string) {
	worldID := l.world.ID
	p := l.world.Player(playerID)
	if p == nil {
		return
	}
	stats := p.Stats
	stats.Talked = make(map[string]bool, len(p.Stats.Talked))
	for id := range p.Stats.Talked {
		stats.Talked[id] = true
	}
	l.persist(func(ctx context.Context) error { return l.store.SaveStats(ctx, worldID, playerID, stats) })
}

func (l *Loop) sendGoals(playerID string) {
	l.sendToPlayer(playerID, protocol.ServerMessage{Type: protocol.TypeGoals, Goals: l.goalDTOs(playerID)})
}

func (l *Loop) goalDTOs(playerID string) []protocol.Goal {
	views := l.world.GoalViews(playerID)
	out := make([]protocol.Goal, 0, len(views))
	for _, v := range views {
		out = append(out, protocol.Goal{
			ID: v.ID, Scope: string(v.Scope), Kind: string(v.Kind), Title: v.Title, Description: v.Description,
			Target: v.Target, Progress: v.Progress, Reward: v.Reward, Completed: v.Completed, AchievedBy: v.AchievedBy,
		})
	}
	return out
}

// sync tells every connected player what changed, as far as they can see (fog of war): entities
// that came into view are sent in full, changed entities that were already in view are updated,
// and entities that left the view (or the board) are reported as removed.
func (l *Loop) sync(changed []*Entity, withScores bool) {
	changedByID := make(map[string]*Entity, len(changed))
	for _, e := range changed {
		changedByID[e.ID] = e
	}
	for _, playerID := range l.connectedPlayers() {
		visible := l.world.VisibleTo(playerID)
		before := l.seen[playerID]

		var send []*Entity
		for id := range visible {
			if _, had := before[id]; !had {
				send = append(send, l.world.entities[id])
			} else if e, ok := changedByID[id]; ok {
				send = append(send, e)
			}
		}
		var removed []string
		for id := range before {
			if _, ok := visible[id]; !ok {
				removed = append(removed, id)
			}
		}
		l.seen[playerID] = visible

		if len(send) == 0 && len(removed) == 0 && !withScores {
			continue
		}
		msg := protocol.ServerMessage{Type: protocol.TypeDelta, Entities: l.dtos(send), Removed: removed}
		if withScores {
			msg.Scores = l.scores()
		}
		l.sendToPlayer(playerID, msg)
	}
}

func (l *Loop) connectedPlayers() []string {
	seen := make(map[string]struct{}, len(l.clients))
	var out []string
	for c := range l.clients {
		if _, dup := seen[c.PlayerID]; !dup {
			seen[c.PlayerID] = struct{}{}
			out = append(out, c.PlayerID)
		}
	}
	return out
}

func (l *Loop) boardDTOs() []protocol.Board {
	boards := l.world.Boards()
	out := make([]protocol.Board, 0, len(boards))
	for _, b := range boards {
		dto := protocol.Board{ID: b.ID, Name: b.Name, Width: b.Width, Height: b.Height, Grid: b.Grid.Kind(), Terrain: b.TerrainRows()}
		for _, g := range l.world.Gateways(b.ID) {
			dto.Gateways = append(dto.Gateways, protocol.Gateway{X: g.At.X, Y: g.At.Y, ToBoard: g.To.Board, ToX: g.To.X, ToY: g.To.Y})
		}
		out = append(out, dto)
	}
	return out
}

func (l *Loop) dtos(entities []*Entity) []protocol.Entity {
	now := l.now()
	untilMs := func(t time.Time) int { return max(0, int(t.Sub(now).Milliseconds())) }
	out := make([]protocol.Entity, 0, len(entities))
	for _, e := range entities {
		icon := ""
		if e.Kind == KindItem {
			icon = ResolveIcon(e.Icon, e.Effect)
		}
		out = append(out, protocol.Entity{
			Icon: icon, Sprite: e.Sprite, ID: e.ID, BoardID: e.BoardID, RaceID: e.RaceID, Race: l.raceName(e.RaceID), Traits: l.traitDTOs(e), Kind: string(e.Kind), OwnerID: e.OwnerID, Name: e.Name, Description: e.Description,
			X: e.X, Y: e.Y, Speed: e.Speed, Health: e.Health, MaxHealth: e.MaxHealth,
			Vision: e.Vision, Strength: e.Strength,
			ReadyInMs: untilMs(e.ReadyAt), ActReadyInMs: untilMs(e.ActReadyAt), RespawnInMs: untilMs(e.RespawnAt),
		})
	}
	return out
}

func (l *Loop) scores() []protocol.Score {
	scores := l.world.Scores()
	out := make([]protocol.Score, 0, len(scores))
	for _, s := range scores {
		out = append(out, protocol.Score{PlayerID: s.PlayerID, Username: s.Username, Points: s.Points, AFK: s.AFK})
	}
	return out
}

func (l *Loop) raceName(id string) string {
	if r := l.world.Race(id); r != nil {
		return r.Name
	}
	return ""
}

// traitDTOs lists a unit's extended characteristics: the world's named ones first, in its order.
// traitDTOs lists every extended characteristic of the world, in the world's order, for a unit:
// the ones it does not have (yet) show as 0, so the player sees all of them. Things that are not
// units (items, structures) have none. A value left over from a characteristic the world no
// longer has is not shown.
func (l *Loop) traitDTOs(e *Entity) []protocol.Trait {
	if !e.IsUnit() {
		return nil
	}
	out := make([]protocol.Trait, 0, len(l.world.Rules.TraitNames))
	for _, name := range l.world.Rules.TraitNames {
		out = append(out, protocol.Trait{Name: name, Value: e.Traits[name]})
	}
	return out
}

func (l *Loop) inventory(playerID string) []protocol.Item {
	p := l.world.Player(playerID)
	if p == nil {
		return nil
	}
	out := make([]protocol.Item, 0, len(p.Inventory))
	for _, it := range p.Inventory {
		out = append(out, protocol.Item{ID: it.ID, Name: it.Name, Description: it.Description, Effect: it.Effect.Summary(), Icon: it.ResolvedIcon(), EffectLines: it.Effect.Lines()})
	}
	return out
}

func (l *Loop) sendError(c *Client, err error) {
	code, msg := Describe(err)
	l.send(c, protocol.ServerMessage{Type: protocol.TypeError, Code: code, Message: msg})
}

func (l *Loop) send(c *Client, msg protocol.ServerMessage) {
	data, err := json.Marshal(msg)
	if err != nil {
		log.Printf("encode message: %v", err)
		return
	}
	l.sendRaw(c, data)
}

func (l *Loop) sendToPlayer(playerID string, msg protocol.ServerMessage) {
	data, err := json.Marshal(msg)
	if err != nil {
		log.Printf("encode message: %v", err)
		return
	}
	for c := range l.clients {
		if c.PlayerID == playerID {
			l.sendRaw(c, data)
		}
	}
}

// sendRaw never blocks the loop: a client whose buffer is full is too slow and gets dropped.
func (l *Loop) sendRaw(c *Client, data []byte) {
	select {
	case c.Send <- data:
	default:
		l.drop(c)
	}
}

func (l *Loop) drop(c *Client) {
	delete(l.clients, c)
	close(c.Send)
	for other := range l.clients {
		if other.PlayerID == c.PlayerID {
			return
		}
	}
	delete(l.seen, c.PlayerID)
}
