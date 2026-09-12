package sim

// Broadcaster decouples sim from the WS transport: the board loop only knows
// it can push a raw message to everyone connected to a board, not how
// connections are managed. Implemented by ws.Hub.
type Broadcaster interface {
	BroadcastToBoard(boardID string, msg []byte)
}
