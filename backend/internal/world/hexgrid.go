package world

// HexGrid is a pointy-top hexagonal grid using axial coordinates (q, r),
// stored in the same Coord{X, Y} shape as SquareGrid (X=q, Y=r). Bounded to a
// rectangular axial range for simplicity (Width/Height act as q/r bounds).
type HexGrid struct {
	Width  int
	Height int
}

func NewHexGrid(width, height int) *HexGrid {
	return &HexGrid{Width: width, Height: height}
}

func (g *HexGrid) InBounds(c Coord) bool {
	return c.X >= 0 && c.Y >= 0 && c.X < g.Width && c.Y < g.Height
}

// axial direction vectors for a pointy-top hex grid.
var hexDirections = []Coord{
	{X: 1, Y: 0}, {X: 1, Y: -1}, {X: 0, Y: -1},
	{X: -1, Y: 0}, {X: -1, Y: 1}, {X: 0, Y: 1},
}

func (g *HexGrid) Neighbors(c Coord) []Coord {
	out := make([]Coord, 0, len(hexDirections))
	for _, d := range hexDirections {
		n := Coord{X: c.X + d.X, Y: c.Y + d.Y}
		if g.InBounds(n) {
			out = append(out, n)
		}
	}
	return out
}

func (g *HexGrid) Distance(a, b Coord) int {
	// axial distance formula
	dx := a.X - b.X
	dy := a.Y - b.Y
	dz := (-a.X - a.Y) - (-b.X - b.Y)
	return maxInt(abs(dx), maxInt(abs(dy), abs(dz)))
}

func maxInt(a, b int) int {
	if a > b {
		return a
	}
	return b
}
