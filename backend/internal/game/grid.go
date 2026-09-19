package game

import "fmt"

const (
	GridSquare = "square"
	GridHex    = "hex"
)

// Grid is the shape of a board. Distance, vision, movement range and neighbours all go through
// it, so the rules work the same on any kind of board (design.md: mixed grids).
type Grid interface {
	Kind() string
	// Distance is the number of steps between two cells.
	Distance(a, b Point) int
	// Neighbors are the cells one step away, without checking the board's bounds.
	Neighbors(p Point) []Point
}

func NewGrid(kind string) (Grid, error) {
	switch kind {
	case "", GridSquare:
		return squareGrid{}, nil
	case GridHex:
		return hexGrid{}, nil
	}
	return nil, fmt.Errorf("unknown grid kind %q", kind)
}

// squareGrid: eight neighbours, diagonal steps cost the same as straight ones (Chebyshev).
type squareGrid struct{}

func (squareGrid) Kind() string { return GridSquare }

func (squareGrid) Distance(a, b Point) int { return max(abs(a.X-b.X), abs(a.Y-b.Y)) }

func (squareGrid) Neighbors(p Point) []Point {
	out := make([]Point, 0, 8)
	for dy := -1; dy <= 1; dy++ {
		for dx := -1; dx <= 1; dx++ {
			if dx != 0 || dy != 0 {
				out = append(out, Point{p.X + dx, p.Y + dy})
			}
		}
	}
	return out
}

// hexGrid: six neighbours. Cells are stored as "odd-r" offset coordinates: rows are horizontal
// and odd rows are shifted half a cell to the right.
type hexGrid struct{}

func (hexGrid) Kind() string { return GridHex }

func (hexGrid) Distance(a, b Point) int {
	aq, ar := hexAxial(a)
	bq, br := hexAxial(b)
	dq, dr := aq-bq, ar-br
	return max(abs(dq), abs(dr), abs(dq+dr))
}

// hexAxial converts odd-r offset coordinates to axial ones, where distance is simple.
func hexAxial(p Point) (q, r int) { return p.X - (p.Y-(p.Y&1))/2, p.Y }

var hexNeighborOffsets = [2][6]Point{
	{{1, 0}, {0, -1}, {-1, -1}, {-1, 0}, {-1, 1}, {0, 1}}, // even rows
	{{1, 0}, {1, -1}, {0, -1}, {-1, 0}, {0, 1}, {1, 1}},   // odd rows
}

func (hexGrid) Neighbors(p Point) []Point {
	offsets := hexNeighborOffsets[p.Y&1]
	out := make([]Point, 0, 6)
	for _, o := range offsets {
		out = append(out, Point{p.X + o.X, p.Y + o.Y})
	}
	return out
}
