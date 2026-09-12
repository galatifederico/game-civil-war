// Package world contains the grid abstraction shared by every board.
//
// Board shape is per-world admin configuration (see docs/design.md, "Struttura
// della board"): some boards are square grids, others may be hex. All movement
// and action-radius code must go through the Grid interface instead of
// assuming square/cartesian math, so adding a new grid shape later does not
// require touching sim logic.
package world

// Coord is a generic grid coordinate. For SquareGrid it's cartesian (x, y).
// For a future HexGrid it would be axial (q, r) using the same struct shape.
type Coord struct {
	X int `json:"x"`
	Y int `json:"y"`
}

// Grid is implemented per board shape (square, hex, ...).
type Grid interface {
	// InBounds reports whether c is a valid coordinate on this grid.
	InBounds(c Coord) bool
	// Neighbors returns the coordinates directly adjacent to c.
	Neighbors(c Coord) []Coord
	// Distance returns the grid distance between a and b (number of steps).
	Distance(a, b Coord) int
}

// SquareGrid is a rectangular grid with 4-directional adjacency.
type SquareGrid struct {
	Width  int
	Height int
}

func NewSquareGrid(width, height int) *SquareGrid {
	return &SquareGrid{Width: width, Height: height}
}

func (g *SquareGrid) InBounds(c Coord) bool {
	return c.X >= 0 && c.Y >= 0 && c.X < g.Width && c.Y < g.Height
}

func (g *SquareGrid) Neighbors(c Coord) []Coord {
	candidates := []Coord{
		{X: c.X + 1, Y: c.Y},
		{X: c.X - 1, Y: c.Y},
		{X: c.X, Y: c.Y + 1},
		{X: c.X, Y: c.Y - 1},
	}
	out := make([]Coord, 0, len(candidates))
	for _, n := range candidates {
		if g.InBounds(n) {
			out = append(out, n)
		}
	}
	return out
}

func (g *SquareGrid) Distance(a, b Coord) int {
	return abs(a.X-b.X) + abs(a.Y-b.Y)
}

func abs(v int) int {
	if v < 0 {
		return -v
	}
	return v
}
