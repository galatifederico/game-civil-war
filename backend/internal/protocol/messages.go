// Package protocol defines the JSON messages exchanged over the game WebSocket.
// It has no dependencies so both the simulation and the transport can use it.
package protocol

const (
	// Client -> server.
	TypeAuth     = "auth"
	TypeMove     = "move"
	TypeAttack   = "attack"
	TypeTalk     = "talk"
	TypePickup   = "pickup"
	TypeBuild    = "build"
	TypeCreate   = "create"
	TypeMoveItem = "move_item"
	TypeBreed    = "breed"
	TypeUseItem  = "use_item"

	// Server -> client.
	TypeSnapshot  = "snapshot"
	TypeDelta     = "delta"
	TypeEvent     = "event"
	TypeInventory = "inventory"
	TypeGoals     = "goals"
	TypeError     = "error"
)

// Trait is one of a unit's extended characteristics (soldi, alcol, ...).
type Trait struct {
	Name  string `json:"name"`
	Value int    `json:"value"`
}

type Entity struct {
	ID           string  `json:"id"`
	BoardID      string  `json:"board_id"`
	RaceID       string  `json:"race_id"`
	Race         string  `json:"race"` // the race's name, empty in worlds without races
	Traits       []Trait `json:"traits,omitempty"`
	Kind         string  `json:"kind"`
	OwnerID      string  `json:"owner_id"`
	Name         string  `json:"name"`
	Description  string  `json:"description"`
	X            int     `json:"x"`
	Y            int     `json:"y"`
	Speed        int     `json:"speed"`
	Health       int     `json:"health"`
	MaxHealth    int     `json:"max_health"`
	Vision       int     `json:"vision"`
	Strength     int     `json:"strength"`
	ReadyInMs    int     `json:"ready_in_ms"`     // until the unit can move again
	ActReadyInMs int     `json:"act_ready_in_ms"` // until the unit can act again
	RespawnInMs  int     `json:"respawn_in_ms"`   // for defeated units
}

// Gateway is a cell of a board that carries whoever steps on it to another board.
type Gateway struct {
	X       int    `json:"x"`
	Y       int    `json:"y"`
	ToBoard string `json:"to_board"`
	ToX     int    `json:"to_x"`
	ToY     int    `json:"to_y"`
}

type Board struct {
	ID       string    `json:"id"`
	Name     string    `json:"name"`
	Width    int       `json:"width"`
	Height   int       `json:"height"`
	Grid     string    `json:"grid"` // "square"
	Gateways []Gateway `json:"gateways,omitempty"`
	// Terrain has one string per row (top first), one glyph per cell (game.TerrainKinds); absent
	// when the whole board is grass.
	Terrain []string `json:"terrain,omitempty"`
}

type Item struct {
	ID          string `json:"id"`
	Name        string `json:"name"`
	Description string `json:"description"`
	Effect      string `json:"effect"` // a short description of what using it does
	Icon        string `json:"icon"`   // which icon to draw (game.ItemIcons)
	// EffectLines is the effect in full, one line each, for the detail page.
	EffectLines []string `json:"effect_lines"`
}

// Goal is a goal as one player sees it: every world goal plus their own individual ones.
type Goal struct {
	ID          string `json:"id"`
	Scope       string `json:"scope"` // "world" or "individual"
	Kind        string `json:"kind"`
	Title       string `json:"title"`
	Description string `json:"description"`
	Target      int    `json:"target"`
	Progress    int    `json:"progress"`
	Reward      int    `json:"reward"`
	Completed   bool   `json:"completed"`
	AchievedBy  string `json:"achieved_by"` // world goals: the name of whoever won it
}

type Score struct {
	PlayerID string `json:"player_id"`
	Username string `json:"username"`
	Points   int    `json:"points"`
	AFK      bool   `json:"afk,omitempty"` // away: no activity for a while
}

type ClientMessage struct {
	Type     string `json:"type"`
	Token    string `json:"token"`
	WorldID  string `json:"world_id"` // with "auth": the world to play in
	UnitID   string `json:"unit_id"`
	TargetID string `json:"target_id"`
	Method   string `json:"method"` // with "create": "health" or "resources"
	X        int    `json:"x"`
	Y        int    `json:"y"`
}

// ServerMessage is one flat envelope for every server message; unused fields are omitted.
// Slices are only present when the message updates them (scores and inventory only ever grow
// or change as a whole, so "absent" means "unchanged").
type ServerMessage struct {
	Type         string   `json:"type"`
	Boards       []Board  `json:"boards,omitempty"`
	YourPlayerID string   `json:"your_player_id,omitempty"`
	Entities     []Entity `json:"entities,omitempty"`
	Removed      []string `json:"removed,omitempty"`
	Scores       []Score  `json:"scores,omitempty"`
	Inventory    []Item   `json:"inventory,omitempty"`
	Goals        []Goal   `json:"goals,omitempty"`
	Code         string   `json:"code,omitempty"`
	Title        string   `json:"title,omitempty"`
	Message      string   `json:"message,omitempty"`
}
