package game

import "testing"

func TestNewGrid(t *testing.T) {
	for kind, want := range map[string]string{"": GridSquare, "square": GridSquare} {
		g, err := NewGrid(kind)
		if err != nil || g.Kind() != want {
			t.Fatalf("NewGrid(%q) = %v, %v", kind, g, err)
		}
	}
	for _, kind := range []string{"triangle", "hex"} {
		if _, err := NewGrid(kind); err == nil {
			t.Fatalf("grid kind %q should be an error", kind)
		}
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
