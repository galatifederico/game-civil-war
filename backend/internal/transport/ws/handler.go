package ws

import (
	"context"
	"encoding/json"
	"log"
	"net/http"
	"time"

	"github.com/coder/websocket"
	"github.com/google/uuid"

	"thegame/internal/auth"
	"thegame/internal/protocol"
	"thegame/internal/sim"
	"thegame/internal/store"
	"thegame/internal/world"
)

// Handler upgrades HTTP requests to WebSocket connections and drives the
// auth handshake + read loop for each one. M1 scope: a single hardcoded
// world/board (multi-world/board selection arrives with the M6 lobby).
type Handler struct {
	Hub            *Hub
	Issuer         *auth.TokenIssuer
	UnitRepo       *store.UnitRepo
	WorldRepo      *store.WorldRepo
	DefaultWorldID string
	DefaultBoard   *store.Board
}

const authHandshakeTimeout = 5 * time.Second

func (h *Handler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	conn, err := websocket.Accept(w, r, &websocket.AcceptOptions{
		OriginPatterns: []string{"*"}, // private LAN game, not a public browser app
	})
	if err != nil {
		log.Printf("ws: accept failed: %v", err)
		return
	}
	defer conn.CloseNow()

	ctx, cancel := context.WithCancel(r.Context())
	defer cancel()

	playerID, err := h.handshake(ctx, conn)
	if err != nil {
		log.Printf("ws: handshake failed: %v", err)
		conn.Close(websocket.StatusPolicyViolation, "auth failed")
		return
	}

	if _, err := h.WorldRepo.EnsureMembership(ctx, playerID, h.DefaultWorldID); err != nil {
		log.Printf("ws: failed to ensure membership for %s: %v", playerID, err)
		conn.Close(websocket.StatusInternalError, "membership error")
		return
	}

	champion, err := h.UnitRepo.GetOrCreateChampion(ctx, uuid.NewString(), playerID, h.DefaultBoard.ID)
	if err != nil {
		log.Printf("ws: failed to load champion for %s: %v", playerID, err)
		conn.Close(websocket.StatusInternalError, "champion error")
		return
	}

	loop, ok := h.Hub.BoardLoop(h.DefaultBoard.ID)
	if !ok {
		log.Printf("ws: board %s has no running loop", h.DefaultBoard.ID)
		conn.Close(websocket.StatusInternalError, "board not running")
		return
	}

	c := newConnection(conn)
	c.playerID = playerID
	c.boardID = h.DefaultBoard.ID
	c.unitID = champion.ID

	loop.JoinUnit(sim.FromStoreRecord(*champion))
	h.Hub.addConn(c.boardID, c)

	go c.writePump(ctx)

	h.sendSnapshot(c, loop)
	h.readLoop(ctx, c, loop)

	loop.LeaveUnit(c.unitID)
	h.Hub.removeConn(c.boardID, c)
}

// handshake blocks until the client sends a valid `auth` message or the
// handshake timeout elapses.
func (h *Handler) handshake(parent context.Context, conn *websocket.Conn) (string, error) {
	ctx, cancel := context.WithTimeout(parent, authHandshakeTimeout)
	defer cancel()

	_, data, err := conn.Read(ctx)
	if err != nil {
		return "", err
	}

	var env protocol.Envelope
	if err := json.Unmarshal(data, &env); err != nil {
		return "", err
	}
	if env.Type != protocol.TypeAuth {
		return "", auth.ErrInvalidToken
	}
	var payload protocol.AuthPayload
	if err := json.Unmarshal(env.Payload, &payload); err != nil {
		return "", err
	}
	return h.Issuer.Verify(payload.Token)
}

func (h *Handler) sendSnapshot(c *Connection, loop *sim.BoardLoop) {
	units := loop.Snapshot()
	states := make([]protocol.UnitState, 0, len(units))
	for _, u := range units {
		states = append(states, u.ToProto())
	}
	msg, err := protocol.Marshal(protocol.TypeWorldSnapshot, protocol.WorldSnapshotPayload{
		BoardID:    h.DefaultBoard.ID,
		Width:      h.DefaultBoard.Width,
		Height:     h.DefaultBoard.Height,
		YourUnitID: c.unitID,
		Units:      states,
	})
	if err != nil {
		log.Printf("ws: failed to marshal world_snapshot: %v", err)
		return
	}
	c.Send(msg)
}

func (h *Handler) readLoop(ctx context.Context, c *Connection, loop *sim.BoardLoop) {
	for {
		_, data, err := c.conn.Read(ctx)
		if err != nil {
			return // normal close, network error, or ctx cancellation
		}

		var env protocol.Envelope
		if err := json.Unmarshal(data, &env); err != nil {
			h.sendError(c, "malformed message")
			continue
		}

		switch env.Type {
		case protocol.TypeMoveCommand:
			h.handleMove(c, loop, env.Payload)
		case protocol.TypeActionCommand:
			h.handleActionCommand(c, loop, env.Payload)
		case protocol.TypeCreateUnitCommand:
			h.handleCreateUnit(c, loop, env.Payload)
		default:
			h.sendError(c, "unknown or not-yet-implemented message type: "+env.Type)
		}
	}
}

func (h *Handler) handleMove(c *Connection, loop *sim.BoardLoop, raw json.RawMessage) {
	var payload protocol.MoveCommandPayload
	if err := json.Unmarshal(raw, &payload); err != nil {
		h.sendError(c, "malformed move_command")
		return
	}
	target := world.Coord{X: payload.Target.X, Y: payload.Target.Y}
	if err := loop.HandleMove(c.playerID, payload.UnitID, target); err != nil {
		h.sendError(c, err.Error())
	}
}

func (h *Handler) handleActionCommand(c *Connection, loop *sim.BoardLoop, raw json.RawMessage) {
	var payload protocol.ActionCommandPayload
	if err := json.Unmarshal(raw, &payload); err != nil {
		h.sendError(c, "malformed action_command")
		return
	}
	target := world.Coord{X: payload.Target.X, Y: payload.Target.Y}
	if err := loop.HandleAction(c.playerID, payload.UnitID, payload.Action, target); err != nil {
		h.sendError(c, err.Error())
	}
}

func (h *Handler) handleCreateUnit(c *Connection, loop *sim.BoardLoop, raw json.RawMessage) {
	var payload protocol.CreateUnitCommandPayload
	if err := json.Unmarshal(raw, &payload); err != nil {
		h.sendError(c, "malformed create_unit_command")
		return
	}
	if err := loop.HandleCreateUnit(c.playerID, payload.ChampionID); err != nil {
		h.sendError(c, err.Error())
	}
}

func (h *Handler) sendError(c *Connection, message string) {
	msg, err := protocol.Marshal(protocol.TypeError, protocol.ErrorPayload{Message: message})
	if err != nil {
		return
	}
	c.Send(msg)
}
