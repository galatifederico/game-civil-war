package game

import (
	"context"
	"encoding/json"
	"log"
	"time"

	"thegame/backend/internal/protocol"
)

const tickInterval = 250 * time.Millisecond

// Persister is what the simulation needs from the database.
type Persister interface {
	InsertUnits(ctx context.Context, units []*Entity) error
	SaveUnit(ctx context.Context, id, boardID string, x, y, health int) error
	InsertStructure(ctx context.Context, s *Entity) error
	SaveItemPosition(ctx context.Context, id string, x, y int) error
	DeleteItem(ctx context.Context, id string) error
	AddInventory(ctx context.Context, worldID, playerID string, item Item) error
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
	}
}

func (l *Loop) Run(ctx context.Context) {
	defer close(l.done)
	go l.writeWorker(ctx)
	l.world.ScheduleRespawns(l.now())

	ticker := time.NewTicker(tickInterval)
	defer ticker.Stop()
	for {
		select {
		case f := <-l.cmds:
			f()
		case <-ticker.C:
			l.tick()
		case <-ctx.Done():
			for c := range l.clients {
				l.drop(c)
			}
			return
		}
	}
}

func (l *Loop) tick() {
	revived := l.world.Tick(l.now())
	if len(revived) == 0 {
		return
	}
	for _, e := range revived {
		l.saveUnit(e)
	}
	l.sync(revived, false)
}

func (l *Loop) writeWorker(ctx context.Context) {
	for {
		select {
		case write := <-l.writes:
			if err := write(ctx); err != nil {
				log.Printf("database write: %v", err)
			}
		case <-ctx.Done():
			return
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
	id, boardID, x, y, health := e.ID, e.BoardID, e.X, e.Y, e.Health
	l.persist(func(ctx context.Context) error { return l.store.SaveUnit(ctx, id, boardID, x, y, health) })
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
func (l *Loop) EnsureTeam(ctx context.Context, playerID, username string) error {
	return l.call(func() error {
		if l.world.HasUnitsOf(playerID) {
			return nil
		}
		team, err := l.world.PlanTeam(playerID, username)
		if err != nil {
			return err
		}
		if err := l.store.InsertUnits(ctx, team); err != nil {
			return err
		}
		l.world.EnsurePlayer(playerID, username, 0)
		for _, e := range team {
			l.world.Add(e)
		}
		l.sync(team, true)
		return nil
	})
}

func (l *Loop) Register(c *Client) {
	l.do(func() {
		l.clients[c] = struct{}{}
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
		})
	})
}

func (l *Loop) Unregister(c *Client) {
	l.do(func() {
		if _, ok := l.clients[c]; ok {
			l.drop(c)
		}
	})
}

func (l *Loop) Move(c *Client, unitID string, to Point) {
	l.do(func() {
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
	if out.Points != 0 {
		points := out.Points
		l.persist(func(ctx context.Context) error { return l.store.AddPoints(ctx, worldID, playerID, points) })
	}

	if len(out.Changed) > 0 || len(out.Removed) > 0 || out.Points != 0 {
		l.sync(out.Changed, out.Points != 0)
	}
	if out.Picked != nil {
		l.sendToPlayer(playerID, protocol.ServerMessage{Type: protocol.TypeInventory, Inventory: l.inventory(playerID)})
	}
	for _, n := range out.Notices {
		l.sendToPlayer(n.PlayerID, protocol.ServerMessage{Type: protocol.TypeEvent, Title: n.Title, Message: n.Text})
	}
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
		dto := protocol.Board{ID: b.ID, Name: b.Name, Width: b.Width, Height: b.Height, Grid: b.Grid.Kind()}
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
		out = append(out, protocol.Entity{
			ID: e.ID, BoardID: e.BoardID, Kind: string(e.Kind), OwnerID: e.OwnerID, Name: e.Name, Description: e.Description,
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
		out = append(out, protocol.Score{PlayerID: s.PlayerID, Username: s.Username, Points: s.Points})
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
		out = append(out, protocol.Item{Name: it.Name, Description: it.Description})
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
