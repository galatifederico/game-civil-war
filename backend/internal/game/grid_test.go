package game

import "testing"

func TestNewGrid(t *testing.T) {
	for kind, want := range map[string]string{"": GridSquare, "square": GridSquare, "hex": GridHex} {
		g, err := NewGrid(kind)
		if err != nil || g.Kind() != want {
			t.Fatalf("NewGrid(%q) = %v, %v", kind, g, err)
		}
	}
	if _, err := NewGrid("triangle"); err == nil {
		t.Fatal("an unknown grid kind should be an error")
	}
}

func TestSquareGrid(t *testing.T) {
	g := squareGrid{}
	if got := len(g.Neighbors(Point{3, 3})); got != 8 {
		t.Fatalf("neighbours = %d, want 8", got)
	}
	for _, tc := range []struct {
		a, b Point
		want int
	}{
		{Point{0, 0}, Point{0, 0}, 0},
		{Point{0, 0}, Point{1, 1}, 1}, // diagonal steps cost the same as straight ones
		{Point{0, 0}, Point{3, 1}, 3},
		{Point{2, 7}, Point{-1, 3}, 4},
	} {
		if got := g.Distance(tc.a, tc.b); got != tc.want {
			t.Errorf("Distance(%v, %v) = %d, want %d", tc.a, tc.b, got, tc.want)
		}
	}
}

func TestHexGridNeighborsAreAtDistanceOne(t *testing.T) {
	g := hexGrid{}
	for y := -2; y <= 8; y++ {
		for x := -2; x <= 8; x++ {
			p := Point{x, y}
			ns := g.Neighbors(p)
			if len(ns) != 6 {
				t.Fatalf("%v has %d neighbours, want 6", p, len(ns))
			}
			isNeighbor := map[Point]bool{}
			for _, n := range ns {
				isNeighbor[n] = true
				if d := g.Distance(p, n); d != 1 {
					t.Fatalf("neighbour %v of %v is at distance %d", n, p, d)
				}
				// Neighbourhood is symmetric.
				back := false
				for _, m := range g.Neighbors(n) {
					if m == p {
						back = true
					}
				}
				if !back {
					t.Fatalf("%v is a neighbour of %v but not the other way round", n, p)
				}
			}
			// Nothing else is at distance 1.
			for dy := -3; dy <= 3; dy++ {
				for dx := -3; dx <= 3; dx++ {
					q := Point{x + dx, y + dy}
					if q != p && !isNeighbor[q] && g.Distance(p, q) < 2 {
						t.Fatalf("%v is at distance %d from %v but not a neighbour", q, g.Distance(p, q), p)
					}
				}
			}
		}
	}
}

func TestHexGridDistance(t *testing.T) {
	g := hexGrid{}
	for _, tc := range []struct {
		a, b Point
		want int
	}{
		{Point{0, 0}, Point{0, 0}, 0},
		{Point{0, 0}, Point{3, 0}, 3}, // along a row
		{Point{0, 0}, Point{0, 2}, 2}, // two rows down
		{Point{0, 0}, Point{1, 1}, 2}, // (1,1) is not adjacent to (0,0): odd rows shift right
		{Point{1, 0}, Point{1, 1}, 1}, // ...but it is adjacent to (1,0)
		{Point{0, 0}, Point{4, 4}, 6}, // farther than a square grid would say
	} {
		if got := g.Distance(tc.a, tc.b); got != tc.want {
			t.Errorf("Distance(%v, %v) = %d, want %d", tc.a, tc.b, got, tc.want)
		}
		if got := g.Distance(tc.b, tc.a); got != tc.want {
			t.Errorf("Distance(%v, %v) = %d, want %d (symmetry)", tc.b, tc.a, got, tc.want)
		}
	}
}
