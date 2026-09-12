// Package protocol defines the JSON WebSocket message shapes exchanged
// between the Godot client and the Go server. Kept as plain JSON (no binary
// framing) since the expected scale is small (docs/tecnico.md: 6-15 players).
package protocol

import "encoding/json"

// Envelope wraps every WS message. Payload is decoded based on Type.
type Envelope struct {
	Type    string          `json:"type"`
	Payload json.RawMessage `json:"payload,omitempty"`
}

// --- Client -> Server ---

const (
	TypeAuth              = "auth"
	TypeMoveCommand       = "move_command"
	TypeActionCommand     = "action_command"
	TypeCreateUnitCommand = "create_unit_command"
)

// Action kinds for TypeActionCommand. M2 implements only Attack; Pickup/Talk/
// Build are defined for wire-compatibility with the client but rejected by
// the server until M4/M5 add the systems behind them.
const (
	ActionAttack = "attack"
	ActionPickup = "pickup"
	ActionTalk   = "talk"
	ActionBuild  = "build"
)

type AuthPayload struct {
	Token string `json:"token"`
}

type MoveCommandPayload struct {
	UnitID string `json:"unit_id"`
	Target Coord  `json:"target"`
}

type ActionCommandPayload struct {
	UnitID string `json:"unit_id"`
	Action string `json:"action"` // attack | pickup | talk | build
	Target Coord  `json:"target"`
}

type CreateUnitCommandPayload struct {
	ChampionID string `json:"champion_id"`
}

// Coord mirrors world.Coord in wire form to avoid a protocol->world import.
type Coord struct {
	X int `json:"x"`
	Y int `json:"y"`
}

// --- Server -> Client ---

const (
	TypeWorldSnapshot = "world_snapshot"
	TypeStateDelta    = "state_delta"
	TypeEvent         = "event"
	TypeError         = "error"
)

type UnitState struct {
	ID         string  `json:"id"`
	PlayerID   string  `json:"player_id"`
	IsChampion bool    `json:"is_champion"`
	X          int     `json:"x"`
	Y          int     `json:"y"`
	Speed      float64 `json:"speed"`
	Health     float64 `json:"health"`
	MaxHealth  float64 `json:"max_health"`
	Alive      bool    `json:"alive"`
}

type WorldSnapshotPayload struct {
	BoardID    string      `json:"board_id"`
	Width      int         `json:"width"`
	Height     int         `json:"height"`
	YourUnitID string      `json:"your_unit_id"`
	Units      []UnitState `json:"units"`
}

type StateDeltaPayload struct {
	Units []UnitState `json:"units"`
}

type ErrorPayload struct {
	Message string `json:"message"`
}

func Marshal(msgType string, payload any) ([]byte, error) {
	raw, err := json.Marshal(payload)
	if err != nil {
		return nil, err
	}
	return json.Marshal(Envelope{Type: msgType, Payload: raw})
}
