package game

import (
	"testing"
	"time"
)

// twoBoards: a square board "A" (10x10) and a hex board "B" (8x8), joined at row 5:
// stepping on A(9,5) leads to B(1,5), and stepping on B(0,5) leads back to A(8,5).
func twoBoards(entities ...*Entity) *World {
	w := NewWorld("w", "test")
	w.AddBoard(&Board{ID: "A", Name: "A", Width: 10, Height: 10, Grid: squareGrid{}})
	w.AddBoard(&Board{ID: "B", Name: "B", Width: 8, Height: 8, Grid: hexGrid{}})
	w.AddLink(Cell{"A", 9, 5}, Cell{"B", 1, 5})
	w.AddLink(Cell{"B", 0, 5}, Cell{"A", 8, 5})
	for _, e := range entities {
		w.Add(e)
	}
	w.EnsurePlayer("p1", "Anna", 0)
	w.EnsurePlayer("p2", "Bob", 0)
	return w
}

func walker(id, board string, x, y int) *Entity {
	return &Entity{ID: id, OwnerID: "p1", BoardID: board, Kind: KindMinor, X: x, Y: y, Speed: 3, Health: 100, MaxHealth: 100, Vision: 4, Strength: 10}
}

func TestGatewayCarriesAUnitToTheOtherBoard(t *testing.T) {
	w := twoBoards(walker("u", "A", 7, 5))

	e, err := w.Move("p1", "u", Point{9, 5}, t0) // two steps, onto the gateway
	if err != nil {
		t.Fatal(err)
	}
	if e.BoardID != "B" || e.X != 1 || e.Y != 5 {
		t.Fatalf("unit at %s(%d,%d), want B(1,5)", e.BoardID, e.X, e.Y)
	}
	if _, taken := w.cells[Cell{"A", 7, 5}]; taken {
		t.Fatal("the old cell should be free")
	}
	if w.cells[Cell{"B", 1, 5}] != "u" {
		t.Fatal("the destination cell should be taken")
	}
	if _, taken := w.cells[Cell{"A", 9, 5}]; taken {
		t.Fatal("nobody stays on the gateway itself")
	}
	if want := 2 * time.Second / 3; e.ReadyAt.Sub(t0) != want {
		t.Fatalf("cooldown = %v, want %v (two steps at speed 3)", e.ReadyAt.Sub(t0), want)
	}

	// And back: B(0,5) is one step west of B(1,5), on a hex row.
	e, err = w.Move("p1", "u", Point{0, 5}, t0.Add(time.Minute))
	if err != nil {
		t.Fatal(err)
	}
	if e.BoardID != "A" || e.X != 8 || e.Y != 5 {
		t.Fatalf("unit at %s(%d,%d), want A(8,5)", e.BoardID, e.X, e.Y)
	}
}

func TestGatewayNeedsEnoughSpeedAndAFreeLanding(t *testing.T) {
	t.Run("too far to reach the gateway", func(t *testing.T) {
		w := twoBoards(walker("u", "A", 4, 5))
		if _, err := w.Move("p1", "u", Point{9, 5}, t0); err != ErrTooFar {
			t.Fatalf("got %v", err)
		}
	})
	t.Run("landing cell occupied", func(t *testing.T) {
		w := twoBoards(walker("u", "A", 7, 5), &Entity{ID: "blocker", OwnerID: "p2", BoardID: "B", Kind: KindMinor, X: 1, Y: 5, Health: 10})
		if _, err := w.Move("p1", "u", Point{9, 5}, t0); err != ErrGatewayBlocked {
			t.Fatalf("got %v", err)
		}
		if e := w.entities["u"]; e.BoardID != "A" || e.X != 7 {
			t.Fatalf("a refused crossing must leave the unit where it was: %s(%d,%d)", e.BoardID, e.X, e.Y)
		}
	})
}

func TestNothingCanBePutOnAGateway(t *testing.T) {
	w := twoBoards(walker("u", "A", 8, 4))
	w.Add(&Entity{ID: "champ", OwnerID: "p1", BoardID: "A", Kind: KindChampion, X: 8, Y: 6, Speed: 3, Health: 200, MaxHealth: 200, Vision: 5})
	w.Add(&Entity{ID: "item", BoardID: "A", Kind: KindItem, X: 7, Y: 5})

	if _, err := w.Do("p1", Action{Kind: ActionBuild, UnitID: "u", At: Point{9, 5}}, t0); err != ErrOccupied {
		t.Fatalf("building on a gateway: got %v", err)
	}
	if _, err := w.Do("p1", Action{Kind: ActionMoveItem, UnitID: "u", TargetID: "item", At: Point{9, 5}}, t0); err != ErrOccupied {
		t.Fatalf("pushing an item onto a gateway: got %v", err)
	}
	// Fill every free neighbour of the champion except the gateway (9,5)... which is not adjacent to (8,6)
	// only diagonally: the gateway must simply never be chosen as the place for a new unit.
	for i := 0; i < 8; i++ {
		out, err := w.Do("p1", Action{Kind: ActionCreate, UnitID: "champ"}, t0.Add(time.Duration(i)*time.Minute))
		if err != nil {
			break
		}
		if u := out.Created[0]; u.BoardID == "A" && u.X == 9 && u.Y == 5 {
			t.Fatal("a unit was created on a gateway")
		}
		w.entities["champ"].Health = 200
	}
}

func TestTeamsNeverStartOnAGateway(t *testing.T) {
	w := NewWorld("w", "test")
	w.AddBoard(&Board{ID: "A", Name: "A", Width: 24, Height: 24, Grid: squareGrid{}})
	w.AddBoard(&Board{ID: "B", Name: "B", Width: 8, Height: 8, Grid: hexGrid{}})
	for y := 0; y < 24; y++ { // a wall of gateways all around the anchor of the first team
		for x := 0; x < 10; x++ {
			w.AddLink(Cell{"A", x, y}, Cell{"B", 1, 1})
		}
	}
	team, err := w.PlanTeam("p1", "Anna", "")
	if err != nil {
		t.Fatal(err)
	}
	for _, e := range team {
		if e.BoardID != "A" {
			t.Fatalf("team member on board %q, want the spawn board A", e.BoardID)
		}
		if _, gateway := w.links[e.Cell()]; gateway {
			t.Fatalf("team member placed on the gateway %v", e.Cell())
		}
	}
}

func TestRangeAndVisionStopAtTheBoardEdge(t *testing.T) {
	w := twoBoards(
		&Entity{ID: "champ", OwnerID: "p1", BoardID: "A", Kind: KindChampion, X: 5, Y: 5, Speed: 3, Health: 200, MaxHealth: 200, Vision: 5, Strength: 30},
		&Entity{ID: "sameSpot", OwnerID: "p2", BoardID: "B", Kind: KindMinor, X: 5, Y: 5, Health: 100, MaxHealth: 100, Vision: 3},
		&Entity{ID: "neighbour", OwnerID: "p2", BoardID: "A", Kind: KindMinor, X: 6, Y: 5, Health: 100, MaxHealth: 100, Vision: 3},
	)

	if _, err := w.Do("p1", Action{Kind: ActionAttack, UnitID: "champ", TargetID: "sameSpot"}, t0); err != ErrOutOfRange {
		t.Fatalf("attacking a unit on another board: got %v", err)
	}
	visible := w.VisibleTo("p1")
	if _, ok := visible["sameSpot"]; ok {
		t.Fatal("a unit on another board must not be visible")
	}
	if _, ok := visible["neighbour"]; !ok {
		t.Fatal("a unit next to the champion on the same board must be visible")
	}
	// p2 sees the champion because of its unit next to it on board A, not because of the one at
	// the same coordinates on board B.
	if _, ok := w.VisibleTo("p2")["champ"]; !ok {
		t.Fatal("p2's unit next to the champion should see it")
	}
	w.entities["neighbour"].Health = 0
	if _, ok := w.VisibleTo("p2")["champ"]; ok {
		t.Fatal("with its unit on board A defeated, p2 must not see the champion from board B")
	}
}

func TestHexBoardUsesHexDistances(t *testing.T) {
	// On a hex grid (0,0) and (2,2) are 3 steps apart; on a square grid they would be 2.
	w := twoBoards(&Entity{ID: "u", OwnerID: "p1", BoardID: "B", Kind: KindMinor, X: 0, Y: 0, Speed: 2, Health: 10, MaxHealth: 10, Vision: 2})
	if _, err := w.Move("p1", "u", Point{2, 2}, t0); err != ErrTooFar {
		t.Fatalf("moving 3 hex steps with speed 2: got %v", err)
	}
	if _, err := w.Move("p1", "u", Point{1, 2}, t0); err != nil {
		t.Fatalf("moving 2 hex steps with speed 2: %v", err)
	}
}

func TestCreatedUnitsOnHexBoardsLandOnARealNeighbour(t *testing.T) {
	w := twoBoards(&Entity{ID: "champ", OwnerID: "p1", BoardID: "B", Kind: KindChampion, X: 4, Y: 3, Speed: 3, Health: 200, MaxHealth: 200, Vision: 5})
	out, err := w.Do("p1", Action{Kind: ActionCreate, UnitID: "champ"}, t0)
	if err != nil {
		t.Fatal(err)
	}
	u := out.Created[0]
	if u.BoardID != "B" || (hexGrid{}).Distance(Point{4, 3}, u.Point()) != 1 {
		t.Fatalf("new unit at %s%v, want a hex neighbour of (4,3) on B", u.BoardID, u.Point())
	}
}
