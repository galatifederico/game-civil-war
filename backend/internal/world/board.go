package world

import "fmt"

type GridType string

const (
	GridSquare GridType = "square"
	GridHex    GridType = "hex"
)

// Board is one contiguous area of a world's map (docs/design.md: boards are
// stitched together into a single open world, not isolated instanced rooms).
type Board struct {
	ID      string
	WorldID string
	Name    string
	Type    GridType
	Grid    Grid
}

func NewBoard(id, worldID, name string, gridType GridType, width, height int) (*Board, error) {
	var g Grid
	switch gridType {
	case GridSquare, "":
		g = NewSquareGrid(width, height)
	case GridHex:
		g = NewHexGrid(width, height)
	default:
		return nil, fmt.Errorf("unknown grid type %q", gridType)
	}
	return &Board{ID: id, WorldID: worldID, Name: name, Type: gridType, Grid: g}, nil
}
