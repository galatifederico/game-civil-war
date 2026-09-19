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

	player, loop, ok := s.authenticate(ctx, conn)
	if !ok {
		return
	}
	if err := loop.EnsureTeam(ctx, player.ID, player.Username, ""); err != nil {
		conn.Close(websocket.StatusInternalError, "no team")
		return
	}

	client := &game.Client{PlayerID: player.ID, Send: make(chan []byte, 64)}
	loop.Register(client)
	defer loop.Unregister(client)

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
			loop.Move(client, msg.UnitID, at)
		case protocol.TypeAttack:
			loop.Act(client, game.Action{Kind: game.ActionAttack, UnitID: msg.UnitID, TargetID: msg.TargetID})
		case protocol.TypeTalk:
			loop.Act(client, game.Action{Kind: game.ActionTalk, UnitID: msg.UnitID, TargetID: msg.TargetID})
		case protocol.TypePickup:
			loop.Act(client, game.Action{Kind: game.ActionPickup, UnitID: msg.UnitID, TargetID: msg.TargetID})
		case protocol.TypeMoveItem:
			loop.Act(client, game.Action{Kind: game.ActionMoveItem, UnitID: msg.UnitID, TargetID: msg.TargetID, At: at})
		case protocol.TypeCreate:
			loop.Act(client, game.Action{Kind: game.ActionCreate, UnitID: msg.UnitID, Method: msg.Method})
		case protocol.TypeBreed:
			loop.Act(client, game.Action{Kind: game.ActionBreed, UnitID: msg.UnitID, TargetID: msg.TargetID})
		case protocol.TypeUseItem:
			loop.Act(client, game.Action{Kind: game.ActionUseItem, UnitID: msg.UnitID, TargetID: msg.TargetID})
		case protocol.TypeBuild:
			loop.Act(client, game.Action{Kind: game.ActionBuild, UnitID: msg.UnitID, At: at})
		}
	}
}

// authenticate reads the first message, which must be an "auth" with a valid token and the world to
// play in; the player must have joined that world through the lobby.
func (s *Server) authenticate(ctx context.Context, conn *websocket.Conn) (store.Player, *game.Loop, bool) {
	reject := func(reason string) (store.Player, *game.Loop, bool) {
		conn.Close(websocket.StatusPolicyViolation, reason)
		return store.Player{}, nil, false
	}
	authCtx, cancel := context.WithTimeout(ctx, authTimeout)
	defer cancel()
	_, data, err := conn.Read(authCtx)
	if err != nil {
		return reject("auth required")
	}
	var msg protocol.ClientMessage
	if err := json.Unmarshal(data, &msg); err != nil || msg.Type != protocol.TypeAuth {
		return reject("auth required")
	}
	playerID, err := s.Tokens.Parse(msg.Token)
	if err != nil {
		return reject("invalid token")
	}
	p, err := s.Store.PlayerByID(authCtx, playerID)
	if err != nil {
		return reject("unknown player")
	}
	loop, ok := s.Hub.Loop(msg.WorldID)
	if !ok {
		return reject("unknown world")
	}
	member, err := s.Store.IsMember(authCtx, msg.WorldID, p.ID)
	if err != nil || !member {
		return reject("join the world first")
	}
	return p, loop, true
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
