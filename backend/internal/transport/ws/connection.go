package ws

import (
	"context"
	"log"

	"github.com/coder/websocket"
)

// Connection wraps one client's WebSocket, decoupling the outbound write
// side (a buffered channel drained by writePump) from whatever goroutine
// wants to send a message (a board's single simulation goroutine, via Hub).
type Connection struct {
	conn     *websocket.Conn
	send     chan []byte
	playerID string
	boardID  string
	unitID   string // this connection's champion unit (M1: one unit per connection)
}

func newConnection(conn *websocket.Conn) *Connection {
	return &Connection{
		conn: conn,
		send: make(chan []byte, 64),
	}
}

// Send queues a message for delivery without blocking the caller. Returns
// false if the outbox was full and the message was dropped.
func (c *Connection) Send(msg []byte) bool {
	select {
	case c.send <- msg:
		return true
	default:
		return false
	}
}

func (c *Connection) writePump(ctx context.Context) {
	for {
		select {
		case msg, ok := <-c.send:
			if !ok {
				return
			}
			if err := c.conn.Write(ctx, websocket.MessageText, msg); err != nil {
				log.Printf("ws: write error for player %s: %v", c.playerID, err)
				return
			}
		case <-ctx.Done():
			return
		}
	}
}
