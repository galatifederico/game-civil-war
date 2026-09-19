// Package protocol defines the JSON messages exchanged over the game WebSocket.
// It has no dependencies so both the simulation and the transport can use it.
package protocol

const (
	TypeAuth     = "auth"
	TypeMove     = "move"
	TypeSnapshot = "snapshot"
	TypeDelta    = "delta"
	TypeError    = "error"
)

type Entity struct {
	ID          string `json:"id"`
	Kind        string `json:"kind"`
	OwnerID     string `json:"owner_id"`
	Name        string `json:"name"`
	Description string `json:"description"`
	X           int    `json:"x"`
	Y           int    `json:"y"`
	Speed       int    `json:"speed"`
	Health      int    `json:"health"`
	MaxHealth   int    `json:"max_health"`
	Vision      int    `json:"vision"`
	ReadyInMs   int    `json:"ready_in_ms"`
}

type Board struct {
	ID     string `json:"id"`
	Name   string `json:"name"`
	Width  int    `json:"width"`
	Height int    `json:"height"`
	Grid   string `json:"grid"`
}

type ClientMessage struct {
	Type   string `json:"type"`
	Token  string `json:"token"`
	UnitID string `json:"unit_id"`
	X      int    `json:"x"`
	Y      int    `json:"y"`
}

// ServerMessage is one flat envelope for every server message; unused fields are omitted.
type ServerMessage struct {
	Type         string   `json:"type"`
	Board        *Board   `json:"board,omitempty"`
	YourPlayerID string   `json:"your_player_id,omitempty"`
	Entities     []Entity `json:"entities,omitempty"`
	Removed      []string `json:"removed,omitempty"`
	Code         string   `json:"code,omitempty"`
	Message      string   `json:"message,omitempty"`
}
