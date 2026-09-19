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
	InsertUnits(ctx context.Context, boardID string, units []*Entity) error
	SaveUnit(ctx context.Context, id string, x, y, health int) error
	InsertStructure(ctx context.Context, boardID string, s *Entity) error
	DeleteItem(ctx context.Context, id string) error
	AddInventory(ctx context.Context, playerID string, item Item) error
	AddPoints(ctx context.Context, playerID string, delta int) error
}

// Client is a connected player. Send is closed by the loop when the client is dropped.
type Client struct {
	PlayerID string
	Send     chan []byte
}

// Loop owns the board: every read and write of its state happens in the Run goroutine,
// so the rules need no locking. Other goroutines talk to it through do/call. Database writes
// are queued and executed in order by a separate worker, so they never block the simulation.
type Loop struct {
	board   *Board
	store   Persister
	cmds    chan func()
	clients map[*Client]struct{}
	writes  chan func(context.Context) error
	now     func() time.Time
	done    chan struct{}
}

func NewLoop(board *Board, store Persister) *Loop {
	return &Loop{
		board:   board,
		store:   store,
		cmds:    make(chan func(), 256),
		clients: map[*Client]struct{}{},
		writes:  make(chan func(context.Context) error, 1024),
		now:     time.Now,
		done:    make(chan struct{}),
	}
}

func (l *Loop) Run(ctx context.Context) {
	defer close(l.done)
	go l.writeWorker(ctx)
	l.board.ScheduleRespawns(l.now())

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
	revived := l.board.Tick(l.now())
	if len(revived) == 0 {
		return
	}
	for _, e := range revived {
		l.saveUnit(e)
	}
	l.broadcast(l.delta(revived))
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
	id, x, y, health := e.ID, e.X, e.Y, e.Health
	l.persist(func(ctx context.Context) error { return l.store.SaveUnit(ctx, id, x, y, health) })
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
		if l.board.HasUnitsOf(playerID) {
			return nil
		}
		team, err := l.board.PlanTeam(playerID, username)
		if err != nil {
			return err
		}
		if err := l.store.InsertUnits(ctx, l.board.ID, team); err != nil {
			return err
		}
		l.board.EnsurePlayer(playerID, username, 0)
		for _, e := range team {
			l.board.Add(e)
		}
		msg := l.delta(team)
		msg.Scores = l.scores()
		l.broadcast(msg)
		return nil
	})
}

func (l *Loop) Register(c *Client) {
	l.do(func() {
		l.clients[c] = struct{}{}
		l.send(c, protocol.ServerMessage{
			Type: protocol.TypeSnapshot,
			Board: &protocol.Board{
				ID: l.board.ID, Name: l.board.Name, Width: l.board.Width, Height: l.board.Height, Grid: "square",
			},
			YourPlayerID: c.PlayerID,
			Entities:     l.dtos(l.board.All()),
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
		e, err := l.board.Move(c.PlayerID, unitID, to, l.now())
		if err != nil {
			l.sendError(c, err)
			return
		}
		l.saveUnit(e)
		l.broadcast(l.delta([]*Entity{e}))
	})
}

func (l *Loop) Act(c *Client, a Action) {
	l.do(func() {
		out, err := l.board.Do(c.PlayerID, a, l.now())
		if err != nil {
			l.sendError(c, err)
			return
		}
		l.publish(c.PlayerID, out)
	})
}

// publish persists an action's outcome and tells the clients what changed.
func (l *Loop) publish(playerID string, out *Outcome) {
	for _, e := range out.Dirty {
		l.saveUnit(e)
	}
	for _, s := range out.Created {
		s := s
		l.persist(func(ctx context.Context) error { return l.store.InsertStructure(ctx, l.board.ID, s) })
	}
	for _, id := range out.Removed {
		id := id
		l.persist(func(ctx context.Context) error { return l.store.DeleteItem(ctx, id) })
	}
	if out.Picked != nil {
		item := *out.Picked
		l.persist(func(ctx context.Context) error { return l.store.AddInventory(ctx, playerID, item) })
	}
	if out.Points != 0 {
		points := out.Points
		l.persist(func(ctx context.Context) error { return l.store.AddPoints(ctx, playerID, points) })
	}

	if len(out.Changed) > 0 || len(out.Removed) > 0 || out.Points != 0 {
		msg := l.delta(out.Changed)
		msg.Removed = out.Removed
		if out.Points != 0 {
			msg.Scores = l.scores()
		}
		l.broadcast(msg)
	}
	if out.Picked != nil {
		l.sendToPlayer(playerID, protocol.ServerMessage{Type: protocol.TypeInventory, Inventory: l.inventory(playerID)})
	}
	for _, n := range out.Notices {
		l.sendToPlayer(n.PlayerID, protocol.ServerMessage{Type: protocol.TypeEvent, Title: n.Title, Message: n.Text})
	}
}

func (l *Loop) delta(entities []*Entity) protocol.ServerMessage {
	return protocol.ServerMessage{Type: protocol.TypeDelta, Entities: l.dtos(entities)}
}

func (l *Loop) dtos(entities []*Entity) []protocol.Entity {
	now := l.now()
	untilMs := func(t time.Time) int { return max(0, int(t.Sub(now).Milliseconds())) }
	out := make([]protocol.Entity, 0, len(entities))
	for _, e := range entities {
		out = append(out, protocol.Entity{
			ID: e.ID, Kind: string(e.Kind), OwnerID: e.OwnerID, Name: e.Name, Description: e.Description,
			X: e.X, Y: e.Y, Speed: e.Speed, Health: e.Health, MaxHealth: e.MaxHealth,
			Vision: e.Vision, Strength: e.Strength,
			ReadyInMs: untilMs(e.ReadyAt), ActReadyInMs: untilMs(e.ActReadyAt), RespawnInMs: untilMs(e.RespawnAt),
		})
	}
	return out
}

func (l *Loop) scores() []protocol.Score {
	scores := l.board.Scores()
	out := make([]protocol.Score, 0, len(scores))
	for _, s := range scores {
		out = append(out, protocol.Score{PlayerID: s.PlayerID, Username: s.Username, Points: s.Points})
	}
	return out
}

func (l *Loop) inventory(playerID string) []protocol.Item {
	p := l.board.Player(playerID)
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

func (l *Loop) broadcast(msg protocol.ServerMessage) {
	data, err := json.Marshal(msg)
	if err != nil {
		log.Printf("encode message: %v", err)
		return
	}
	for c := range l.clients {
		l.sendRaw(c, data)
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
}
