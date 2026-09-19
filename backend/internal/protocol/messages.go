// Package protocol defines the JSON messages exchanged over the game WebSocket.
// It has no dependencies so both the simulation and the transport can use it.
package protocol

const (
	// Client -> server.
	TypeAuth   = "auth"
	TypeMove   = "move"
	TypeAttack = "attack"
	TypeTalk   = "talk"
	TypePickup = "pickup"
	TypeBuild  = "build"
	TypeCreate = "create"
	TypeMoveItem = "move_item"

	// Server -> client.
	TypeSnapshot  = "snapshot"
	TypeDelta     = "delta"
	TypeEvent     = "event"
	TypeInventory = "inventory"
	TypeError     = "error"
)

type Entity struct {
	ID           string `json:"id"`
	Kind         string `json:"kind"`
	OwnerID      string `json:"owner_id"`
	Name         string `json:"name"`
	Description  string `json:"description"`
	X            int    `json:"x"`
	Y            int    `json:"y"`
	Speed        int    `json:"speed"`
	Health       int    `json:"health"`
	MaxHealth    int    `json:"max_health"`
	Vision       int    `json:"vision"`
	Strength     int    `json:"strength"`
	ReadyInMs    int    `json:"ready_in_ms"`     // until the unit can move again
	ActReadyInMs int    `json:"act_ready_in_ms"` // until the unit can act again
	RespawnInMs  int    `json:"respawn_in_ms"`   // for defeated units
}

type Board struct {
	ID     string `json:"id"`
	Name   string `json:"name"`
	Width  int    `json:"width"`
	Height int    `json:"height"`
	Grid   string `json:"grid"`
}

type Item struct {
	Name        string `json:"name"`
	Description string `json:"description"`
}

type Score struct {
	PlayerID string `json:"player_id"`
	Username string `json:"username"`
	Points   int    `json:"points"`
}

type ClientMessage struct {
	Type     string `json:"type"`
	Token    string `json:"token"`
	UnitID   string `json:"unit_id"`
	TargetID string `json:"target_id"`
	X        int    `json:"x"`
	Y        int    `json:"y"`
}

// ServerMessage is one flat envelope for every server message; unused fields are omitted.
// Slices are only present when the message updates them (scores and inventory only ever grow
// or change as a whole, so "absent" means "unchanged").
type ServerMessage struct {
	Type         string   `json:"type"`
	Board        *Board   `json:"board,omitempty"`
	YourPlayerID string   `json:"your_player_id,omitempty"`
	Entities     []Entity `json:"entities,omitempty"`
	Removed      []string `json:"removed,omitempty"`
	Scores       []Score  `json:"scores,omitempty"`
	Inventory    []Item   `json:"inventory,omitempty"`
	Code         string   `json:"code,omitempty"`
	Title        string   `json:"title,omitempty"`
	Message      string   `json:"message,omitempty"`
}
