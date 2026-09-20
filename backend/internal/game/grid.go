package game

import "fmt"

const GridSquare = "square"

// Grid is the shape of a board. Distance, vision, movement range and neighbours all go through
// it. There is only the square grid today (hexagonal boards were dropped: the client is a
// top-down tile game), but the rules keep asking the board's grid instead of assuming it.
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
