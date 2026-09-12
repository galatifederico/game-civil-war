// Package ws is the WebSocket transport: it decodes client messages into
// calls on the right sim.BoardLoop and re-encodes sim broadcasts back to
// connected clients. It never mutates game state itself.
package ws

import (
	"log"
	"sync"

	"thegame/internal/sim"
)

// Hub tracks live connections per board and the board loops they talk to. It
// implements sim.Broadcaster.
type Hub struct {
	mu     sync.RWMutex
	boards map[string]*sim.BoardLoop
	conns  map[string]map[*Connection]struct{} // boardID -> set of connections
}

func NewHub() *Hub {
	return &Hub{
		boards: make(map[string]*sim.BoardLoop),
		conns:  make(map[string]map[*Connection]struct{}),
	}
}

func (h *Hub) RegisterBoard(loop *sim.BoardLoop) {
	h.mu.Lock()
	defer h.mu.Unlock()
	h.boards[loop.Board.ID] = loop
}

func (h *Hub) BoardLoop(boardID string) (*sim.BoardLoop, bool) {
	h.mu.RLock()
	defer h.mu.RUnlock()
	loop, ok := h.boards[boardID]
	return loop, ok
}

func (h *Hub) addConn(boardID string, c *Connection) {
	h.mu.Lock()
	defer h.mu.Unlock()
	if h.conns[boardID] == nil {
		h.conns[boardID] = make(map[*Connection]struct{})
	}
	h.conns[boardID][c] = struct{}{}
}

func (h *Hub) removeConn(boardID string, c *Connection) {
	h.mu.Lock()
	defer h.mu.Unlock()
	delete(h.conns[boardID], c)
}

// BroadcastToBoard implements sim.Broadcaster. Called from a board's own
// goroutine, so it must never block on a slow client — Connection.Send
// already drops instead of blocking when a client's outbox is full.
func (h *Hub) BroadcastToBoard(boardID string, msg []byte) {
	h.mu.RLock()
	defer h.mu.RUnlock()
	for c := range h.conns[boardID] {
		if !c.Send(msg) {
			log.Printf("ws: dropped message to a slow connection on board %s", boardID)
		}
	}
}
