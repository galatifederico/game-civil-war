package game

import "testing"

func terrainWorld() *World {
	w := twoBoards(walker("u", "A", 5, 5))
	a := w.Board("A")
	a.SetTerrain(Point{6, 5}, "tree")
	a.SetTerrain(Point{5, 6}, "path")
	w.Board("B").SetTerrain(Point{1, 5}, "wall") // where the gateway on A(9,5) lands
	return w
}

func TestBlockingTerrainStopsMoves(t *testing.T) {
	w := terrainWorld()
	if _, err := w.Move("p1", "u", Point{6, 5}, t0); err != ErrBlockedTerrain {
		t.Fatalf("moving onto a tree: got %v", err)
	}
	if _, err := w.Move("p1", "u", Point{5, 6}, t0); err != nil {
		t.Fatalf("moving onto a path: %v", err)
	}
}

func TestGatewayLandingOnBlockingTerrainIsBlocked(t *testing.T) {
	w := twoBoards(walker("u", "A", 8, 5))
	w.Board("B").SetTerrain(Point{1, 5}, "wall")
	if _, err := w.Move("p1", "u", Point{9, 5}, t0); err != ErrGatewayBlocked {
		t.Fatalf("stepping on a gateway that lands on a wall: got %v", err)
	}
}

func TestNothingIsPlacedOnBlockingTerrain(t *testing.T) {
	w := twoBoards(&Entity{ID: "champ", OwnerID: "p1", BoardID: "A", Kind: KindChampion, X: 5, Y: 5, Speed: 3, Health: 200, MaxHealth: 200, Vision: 5, Strength: 30})
	a := w.Board("A")
	for _, n := range a.Grid.Neighbors(Point{5, 5}) {
		a.SetTerrain(n, "bush")
	}
	if _, err := w.Do("p1", Action{Kind: ActionBuild, UnitID: "champ", At: Point{6, 5}}, t0); err != ErrOccupied {
		t.Fatalf("building on a bush: got %v", err)
	}
	if _, err := w.Do("p1", Action{Kind: ActionCreate, UnitID: "champ"}, t0); err == nil {
		t.Fatal("a unit was created although every neighbouring cell is a bush")
	}
}

func TestTeamsAvoidBlockingTerrain(t *testing.T) {
	w := NewWorld("w", "test")
	w.AddBoard(&Board{ID: "A", Name: "A", Width: 24, Height: 24, Grid: squareGrid{}})
	a := w.Board("A")
	for y := 0; y < 24; y++ { // a wall of trees down the left third
		for x := 0; x < 8; x++ {
			a.SetTerrain(Point{x, y}, "tree")
		}
	}
	team, err := w.PlanTeam("p1", "Anna", "")
	if err != nil {
		t.Fatal(err)
	}
	for _, e := range team {
		if a.BlocksAt(e.Point()) {
			t.Fatalf("%s placed on blocking terrain at %v", e.Name, e.Point())
		}
	}
}

func TestTerrainRowsRoundTrip(t *testing.T) {
	b := &Board{ID: "A", Width: 4, Height: 3, Grid: squareGrid{}}
	if b.TerrainRows() != nil {
		t.Fatal("an all-grass board has no rows")
	}
	b.SetTerrain(Point{0, 0}, "tree")
	b.SetTerrain(Point{3, 1}, "path")
	b.SetTerrain(Point{2, 2}, "flowers")
	want := []string{"T...", "...=", "..,."}
	got := b.TerrainRows()
	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("rows = %q, want %q", got, want)
		}
	}
	b.SetTerrain(Point{0, 0}, "grass")
	if b.TerrainAt(Point{0, 0}) != "grass" || b.BlocksAt(Point{0, 0}) {
		t.Fatal("setting grass should clear the cell")
	}
	for _, k := range TerrainKinds() {
		name, ok := TerrainName(k.Glyph[0])
		if !ok || name != k.Name {
			t.Fatalf("glyph %q does not map back to %q", k.Glyph, k.Name)
		}
	}
}
