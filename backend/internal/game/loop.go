package game

import (
	"context"
	"encoding/json"
	"log"
	"time"

	"thegame/backend/internal/protocol"
)

// Persister is what the simulation needs from the database.
type Persister interface {
	InsertUnits(ctx context.Context, boardID string, units []*Entity) error
	SaveUnitPosition(ctx context.Context, id string, x, y int) error
}

// Client is a connected player. Send is closed by the loop when the client is dropped.
type Client struct {
	PlayerID string
	Send     chan []byte
}

type positionSave struct {
	id   string
	x, y int
}

// Loop owns the board: every read and write of its state happens in the Run goroutine,
// so the rules need no locking. Other goroutines talk to it through do/call.
type Loop struct {
	board   *Board
	store   Persister
	cmds    chan func()
	clients map[*Client]struct{}
	saves   chan positionSave
	now     func() time.Time
	done    chan struct{}
}

func NewLoop(board *Board, store Persister) *Loop {
	return &Loop{
		board:   board,
		store:   store,
		cmds:    make(chan func(), 256),
		clients: map[*Client]struct{}{},
		saves:   make(chan positionSave, 1024),
		now:     time.Now,
		done:    make(chan struct{}),
	}
}

func (l *Loop) Run(ctx context.Context) {
	defer close(l.done)
	go l.saveWorker(ctx)
	for {
		select {
		case f := <-l.cmds:
			f()
		case <-ctx.Done():
			for c := range l.clients {
				l.drop(c)
			}
			return
		}
	}
}

func (l *Loop) saveWorker(ctx context.Context) {
	for {
		select {
		case s := <-l.saves:
			if err := l.store.SaveUnitPosition(ctx, s.id, s.x, s.y); err != nil {
				log.Printf("save position %s: %v", s.id, err)
			}
		case <-ctx.Done():
			return
		}
	}
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
		for _, e := range team {
			l.board.Add(e)
		}
		l.broadcast(l.delta(team))
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
			code, msg := Describe(err)
			l.send(c, protocol.ServerMessage{Type: protocol.TypeError, Code: code, Message: msg})
			return
		}
		select {
		case l.saves <- positionSave{e.ID, e.X, e.Y}:
		default:
			log.Printf("save queue full, position of %s not persisted", e.ID)
		}
		l.broadcast(l.delta([]*Entity{e}))
	})
}

func (l *Loop) delta(entities []*Entity) protocol.ServerMessage {
	return protocol.ServerMessage{Type: protocol.TypeDelta, Entities: l.dtos(entities)}
}

func (l *Loop) dtos(entities []*Entity) []protocol.Entity {
	now := l.now()
	out := make([]protocol.Entity, 0, len(entities))
	for _, e := range entities {
		ready := max(0, int(e.ReadyAt.Sub(now).Milliseconds()))
		out = append(out, protocol.Entity{
			ID: e.ID, Kind: string(e.Kind), OwnerID: e.OwnerID, Name: e.Name, Description: e.Description,
			X: e.X, Y: e.Y, Speed: e.Speed, Health: e.Health, MaxHealth: e.MaxHealth, Vision: e.Vision,
			ReadyInMs: ready,
		})
	}
	return out
}

func (l *Loop) send(c *Client, msg protocol.ServerMessage) {
	data, err := json.Marshal(msg)
	if err != nil {
		log.Printf("encode message: %v", err)
		return
	}
	l.sendRaw(c, data)
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
