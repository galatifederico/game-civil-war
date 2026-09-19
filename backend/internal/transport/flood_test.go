package transport

import (
	"context"
	"encoding/json"
	"testing"
	"time"

	"github.com/coder/websocket"

	"thegame/backend/internal/protocol"
)

func TestCommandFloodIsThrottledThenCut(t *testing.T) {
	e := newEnv(t)
	tok, pid := e.player("p@test.io", "Player")
	c := e.dial(tok)
	snap := c.expect("snapshot", ofType(protocol.TypeSnapshot))
	champ, _ := findEntity(snap.Entities, func(en protocol.Entity) bool { return en.Kind == "champion" && en.OwnerID == pid })

	// The server may cut the connection while the flood is still being sent: that is fine.
	payload, _ := json.Marshal(protocol.ClientMessage{Type: protocol.TypeMove, UnitID: champ.ID, X: champ.X, Y: champ.Y})
	for i := 0; i < 500; i++ {
		if err := c.conn.Write(context.Background(), websocket.MessageText, payload); err != nil {
			break
		}
	}
	c.expect("a rate limit error", func(m protocol.ServerMessage) bool {
		return m.Type == protocol.TypeError && m.Code == "rate_limited"
	})

	// A script that keeps flooding is disconnected: the message channel closes.
	deadline := time.After(3 * time.Second)
	for open := true; open; {
		select {
		case _, open = <-c.msgs:
		case <-deadline:
			t.Fatal("the flooding connection was never cut")
		}
	}

	// Normal play is not affected: a new connection works.
	c2 := e.dial(tok)
	c2.expect("snapshot after reconnecting", ofType(protocol.TypeSnapshot))
}
