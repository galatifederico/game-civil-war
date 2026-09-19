package transport

import (
	"context"
	"encoding/json"
	"log"
	"net/http"
	"time"

	"github.com/coder/websocket"

	"thegame/backend/internal/game"
	"thegame/backend/internal/protocol"
	"thegame/backend/internal/store"
)

const (
	authTimeout  = 5 * time.Second
	pingInterval = 20 * time.Second
	writeTimeout = 5 * time.Second
	readLimit    = 4096
)

// serveWS upgrades the connection, expects an "auth" message with a valid token as the very
// first message, then streams the board to the client and relays its move commands.
func (s *Server) serveWS(w http.ResponseWriter, r *http.Request) {
	// The game client is a native app, not a browser: there is no Origin to check.
	conn, err := websocket.Accept(w, r, &websocket.AcceptOptions{InsecureSkipVerify: true})
	if err != nil {
		return
	}
	conn.SetReadLimit(readLimit)
	ctx, cancel := context.WithCancel(r.Context())
	defer cancel()

	player, ok := s.authenticate(ctx, conn)
	if !ok {
		return
	}
	if err := s.Loop.EnsureTeam(ctx, player.ID, player.Username); err != nil {
		conn.Close(websocket.StatusInternalError, "no team")
		return
	}

	client := &game.Client{PlayerID: player.ID, Send: make(chan []byte, 64)}
	s.Loop.Register(client)
	defer s.Loop.Unregister(client)

	go s.writeLoop(ctx, cancel, conn, client)

	for {
		_, data, err := conn.Read(ctx)
		if err != nil {
			conn.Close(websocket.StatusNormalClosure, "")
			return
		}
		var msg protocol.ClientMessage
		if err := json.Unmarshal(data, &msg); err != nil {
			continue
		}
		at := game.Point{X: msg.X, Y: msg.Y}
		switch msg.Type {
		case protocol.TypeMove:
			s.Loop.Move(client, msg.UnitID, at)
		case protocol.TypeAttack:
			s.Loop.Act(client, game.Action{Kind: game.ActionAttack, UnitID: msg.UnitID, TargetID: msg.TargetID})
		case protocol.TypeTalk:
			s.Loop.Act(client, game.Action{Kind: game.ActionTalk, UnitID: msg.UnitID, TargetID: msg.TargetID})
		case protocol.TypePickup:
			s.Loop.Act(client, game.Action{Kind: game.ActionPickup, UnitID: msg.UnitID, TargetID: msg.TargetID})
		case protocol.TypeBuild:
			s.Loop.Act(client, game.Action{Kind: game.ActionBuild, UnitID: msg.UnitID, At: at})
		}
	}
}

func (s *Server) authenticate(ctx context.Context, conn *websocket.Conn) (store.Player, bool) {
	authCtx, cancel := context.WithTimeout(ctx, authTimeout)
	defer cancel()
	_, data, err := conn.Read(authCtx)
	if err != nil {
		conn.Close(websocket.StatusPolicyViolation, "auth required")
		return store.Player{}, false
	}
	var msg protocol.ClientMessage
	if err := json.Unmarshal(data, &msg); err != nil || msg.Type != protocol.TypeAuth {
		conn.Close(websocket.StatusPolicyViolation, "auth required")
		return store.Player{}, false
	}
	playerID, err := s.Tokens.Parse(msg.Token)
	if err != nil {
		conn.Close(websocket.StatusPolicyViolation, "invalid token")
		return store.Player{}, false
	}
	p, err := s.Store.PlayerByID(authCtx, playerID)
	if err != nil {
		conn.Close(websocket.StatusPolicyViolation, "unknown player")
		return store.Player{}, false
	}
	return p, true
}

func (s *Server) writeLoop(ctx context.Context, cancel context.CancelFunc, conn *websocket.Conn, c *game.Client) {
	defer cancel()
	ping := time.NewTicker(pingInterval)
	defer ping.Stop()
	for {
		select {
		case data, ok := <-c.Send:
			if !ok {
				conn.Close(websocket.StatusPolicyViolation, "too slow")
				return
			}
			writeCtx, done := context.WithTimeout(ctx, writeTimeout)
			err := conn.Write(writeCtx, websocket.MessageText, data)
			done()
			if err != nil {
				log.Printf("ws write: %v", err)
				return
			}
		case <-ping.C:
			pingCtx, done := context.WithTimeout(ctx, writeTimeout)
			err := conn.Ping(pingCtx)
			done()
			if err != nil {
				return
			}
		case <-ctx.Done():
			return
		}
	}
}
