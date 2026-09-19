package game

import (
	"fmt"
	"testing"
	"time"
)

func actionBoard() *World {
	b := newTestWorld("b", "test", 20, 20, []*Entity{
		{ID: "champ", OwnerID: "p1", Kind: KindChampion, Name: "Champion", X: 5, Y: 5, Speed: 3, Health: 200, MaxHealth: 200, Vision: 5, Strength: 30},
		{ID: "minor", OwnerID: "p1", Kind: KindMinor, Name: "Pedina 1", X: 6, Y: 5, Speed: 2, Health: 100, MaxHealth: 100, Vision: 3, Strength: 15},
		{ID: "enemy", OwnerID: "p2", Kind: KindMinor, Name: "Pedina nemica", X: 8, Y: 5, Speed: 2, Health: 100, MaxHealth: 100, Vision: 3, Strength: 15},
		{ID: "far", OwnerID: "p2", Kind: KindMinor, Name: "Lontana", X: 15, Y: 5, Speed: 2, Health: 100, MaxHealth: 100, Vision: 3, Strength: 15},
		{ID: "npc", Kind: KindNPC, Name: "Mercante", X: 5, Y: 7, Health: 100, MaxHealth: 100, Dialogue: []string{"Ho tutto."}},
		{ID: "farnpc", Kind: KindNPC, Name: "Guardia", X: 18, Y: 18, Health: 100, MaxHealth: 100},
		{ID: "item", Kind: KindItem, Name: "Forziere", Description: "Chiuso.", X: 4, Y: 5},
	})
	b.EnsurePlayer("p1", "Anna", 0)
	b.EnsurePlayer("p2", "Bob", 0)
	return b
}

func attack(unit, target string) Action {
	return Action{Kind: ActionAttack, UnitID: unit, TargetID: target}
}

func TestAttackHurtsAndScores(t *testing.T) {
	b := actionBoard()
	out, err := b.Do("p1", attack("champ", "enemy"), t0)
	if err != nil {
		t.Fatal(err)
	}
	if got := b.entities["enemy"].Health; got != 70 {
		t.Fatalf("enemy health = %d, want 70 (champion strength 30)", got)
	}
	if out.Points != rules.Points.Hit || b.Player("p1").Points != rules.Points.Hit {
		t.Fatalf("points = %d / %d, want %d", out.Points, b.Player("p1").Points, rules.Points.Hit)
	}
	if len(out.Dirty) != 1 || out.Dirty[0].ID != "enemy" {
		t.Fatalf("only the target's persisted state changed, got %v", out.Dirty)
	}
}

func TestAttackCooldownThenKillAndRespawn(t *testing.T) {
	b := actionBoard()
	now := t0
	var out *Outcome
	var err error
	for hit := 1; hit <= 4; hit++ { // 4 x 30 damage >= 100 health
		if hit > 1 {
			if _, err := b.Do("p1", attack("champ", "enemy"), now.Add(time.Second)); err != ErrCooldown {
				t.Fatalf("hit %d: expected cooldown, got %v", hit, err)
			}
			now = now.Add(rules.AttackCooldown())
		}
		out, err = b.Do("p1", attack("champ", "enemy"), now)
		if err != nil {
			t.Fatalf("hit %d: %v", hit, err)
		}
	}
	enemy := b.entities["enemy"]
	if enemy.Health != 0 || enemy.RespawnAt.IsZero() {
		t.Fatalf("enemy should be defeated with a respawn time, health=%d", enemy.Health)
	}
	if want := 4*rules.Points.Hit + rules.Points.KillMinor; b.Player("p1").Points != want {
		t.Fatalf("points = %d, want %d", b.Player("p1").Points, want)
	}
	if len(out.Notices) != 2 || out.Notices[0].PlayerID != "p1" || out.Notices[1].PlayerID != "p2" {
		t.Fatalf("notices = %+v", out.Notices)
	}

	if revived := b.Tick(now.Add(rules.RespawnDelay() - time.Second)); len(revived) != 0 {
		t.Fatal("respawned too early")
	}
	revived := b.Tick(now.Add(rules.RespawnDelay()))
	if len(revived) != 1 || revived[0].Health != 100 || !revived[0].RespawnAt.IsZero() {
		t.Fatalf("respawn failed: %+v", revived)
	}
}

func TestKillingAChampionIsWorthMore(t *testing.T) {
	b := actionBoard()
	b.entities["enemy"].Kind = KindChampion
	b.entities["enemy"].Health = 20
	if _, err := b.Do("p1", attack("champ", "enemy"), t0); err != nil {
		t.Fatal(err)
	}
	if want := rules.Points.Hit + rules.Points.KillChampion; b.Player("p1").Points != want {
		t.Fatalf("points = %d, want %d", b.Player("p1").Points, want)
	}
}

func TestAttackRules(t *testing.T) {
	tests := []struct {
		name   string
		mutate func(*World)
		player string
		action Action
		want   *Error
	}{
		{"own unit", nil, "p1", attack("champ", "minor"), ErrInvalidTarget},
		{"npc cannot be attacked", nil, "p1", attack("champ", "npc"), ErrInvalidTarget},
		{"item cannot be attacked", nil, "p1", attack("champ", "item"), ErrInvalidTarget},
		{"unknown target", nil, "p1", attack("champ", "nope"), ErrNoTarget},
		{"out of vision", nil, "p1", attack("minor", "far"), ErrOutOfRange},
		{"not the owner of the attacker", nil, "p2", attack("champ", "enemy"), ErrNotYours},
		{"target already defeated", func(b *World) { b.entities["enemy"].Health = 0 }, "p1", attack("champ", "enemy"), ErrTargetDead},
		{"attacker defeated", func(b *World) { b.entities["champ"].Health = 0 }, "p1", attack("champ", "enemy"), ErrDead},
		{"unknown action", nil, "p1", Action{Kind: "dance", UnitID: "champ"}, ErrUnknownAction},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			b := actionBoard()
			if tc.mutate != nil {
				tc.mutate(b)
			}
			if _, err := b.Do(tc.player, tc.action, t0); err != tc.want {
				t.Fatalf("got %v, want %v", err, tc.want)
			}
		})
	}
}

func TestActingDoesNotBlockMoving(t *testing.T) {
	b := actionBoard()
	if _, err := b.Do("p1", attack("champ", "enemy"), t0); err != nil {
		t.Fatal(err)
	}
	if _, err := b.Move("p1", "champ", Point{5, 4}, t0); err != nil {
		t.Fatalf("moving right after an attack should work: %v", err)
	}
}

func TestTalk(t *testing.T) {
	b := actionBoard()
	talk := func(unit, target string) Action { return Action{Kind: ActionTalk, UnitID: unit, TargetID: target} }

	out, err := b.Do("p1", talk("champ", "npc"), t0)
	if err != nil {
		t.Fatal(err)
	}
	if len(out.Notices) != 1 || out.Notices[0].Title != "Mercante" || out.Notices[0].Text != "Ho tutto." {
		t.Fatalf("notices = %+v", out.Notices)
	}
	if _, err := b.Do("p1", talk("champ", "npc"), t0); err != nil {
		t.Fatalf("talking has no cooldown: %v", err)
	}
	if _, err := b.Do("p1", talk("champ", "enemy"), t0); err != ErrInvalidTarget {
		t.Fatalf("talking to a unit: got %v", err)
	}
	if _, err := b.Do("p1", talk("champ", "farnpc"), t0); err != ErrOutOfRange {
		t.Fatalf("talking too far: got %v", err)
	}
}

func TestPickupGoesToSharedInventory(t *testing.T) {
	b := actionBoard()
	pickup := Action{Kind: ActionPickup, UnitID: "minor", TargetID: "item"}

	out, err := b.Do("p1", pickup, t0) // a minor unit picks it up, 2 cells away
	if err != nil {
		t.Fatal(err)
	}
	inv := b.Player("p1").Inventory
	if len(inv) != 1 || inv[0].Name != "Forziere" || out.Picked == nil {
		t.Fatalf("inventory = %+v", inv)
	}
	if len(out.Removed) != 1 || out.Removed[0] != "item" {
		t.Fatalf("removed = %v", out.Removed)
	}
	if b.Player("p1").Points != rules.Points.Pickup {
		t.Fatalf("points = %d", b.Player("p1").Points)
	}
	if _, err := b.Do("p1", pickup, t0.Add(time.Minute)); err != ErrNoTarget {
		t.Fatalf("picking up twice: got %v", err)
	}
	if _, err := b.Move("p1", "champ", Point{4, 5}, t0); err != nil {
		t.Fatalf("the cell should be free again: %v", err)
	}
}

func TestBuildClaimsTerritory(t *testing.T) {
	b := actionBoard()
	build := func(x, y int) Action { return Action{Kind: ActionBuild, UnitID: "champ", At: Point{x, y}} }

	out, err := b.Do("p1", build(5, 4), t0)
	if err != nil {
		t.Fatal(err)
	}
	if len(out.Created) != 1 || out.Created[0].Kind != KindStructure || out.Created[0].OwnerID != "p1" || out.Created[0].ID == "" {
		t.Fatalf("created = %+v", out.Created)
	}
	if b.Player("p1").Points != rules.Points.Build {
		t.Fatalf("points = %d", b.Player("p1").Points)
	}
	if _, err := b.Move("p1", "minor", Point{5, 4}, t0); err != ErrOccupied {
		t.Fatalf("the structure should block the cell, got %v", err)
	}
	if _, err := b.Do("p1", build(4, 4), t0.Add(time.Second)); err != ErrCooldown {
		t.Fatalf("building has a cooldown, got %v", err)
	}

	later := t0.Add(rules.BuildCooldown())
	if _, err := b.Do("p1", build(5, 4), later); err != ErrOccupied {
		t.Fatalf("occupied cell: got %v", err)
	}
	if _, err := b.Do("p1", build(5, 12), later); err != ErrOutOfRange {
		t.Fatalf("beyond vision: got %v", err)
	}
	if _, err := b.Do("p1", build(-1, 5), later); err != ErrOutOfBounds {
		t.Fatalf("out of bounds: got %v", err)
	}
}

func TestScoresAreSortedBestFirst(t *testing.T) {
	b := actionBoard()
	b.EnsurePlayer("p2", "", 0).Points = 50
	b.EnsurePlayer("p1", "", 0).Points = 10
	scores := b.Scores()
	if len(scores) != 2 || scores[0].PlayerID != "p2" || scores[1].PlayerID != "p1" {
		t.Fatalf("scores = %+v", scores)
	}
}

func TestScheduleRespawnsForUnitsDefeatedBeforeRestart(t *testing.T) {
	b := actionBoard()
	b.entities["enemy"].Health = 0
	b.ScheduleRespawns(t0)
	if got := b.entities["enemy"].RespawnAt; !got.Equal(t0.Add(rules.RespawnDelay())) {
		t.Fatalf("respawn at %v", got)
	}
}

func TestCreateUnitCostsTheChampionsHealth(t *testing.T) {
	b := actionBoard()
	create := Action{Kind: ActionCreate, UnitID: "champ"}

	out, err := b.Do("p1", create, t0)
	if err != nil {
		t.Fatal(err)
	}
	if got := b.entities["champ"].Health; got != 200-rules.CreateHealthCost {
		t.Fatalf("champion health = %d, want %d", got, 200-rules.CreateHealthCost)
	}
	if len(out.Created) != 1 || out.Created[0].Kind != KindMinor || out.Created[0].OwnerID != "p1" || out.Created[0].ID == "" {
		t.Fatalf("created = %+v", out.Created)
	}
	u := out.Created[0]
	if (squareGrid{}).Distance(Point{u.X, u.Y}, Point{5, 5}) != 1 {
		t.Fatalf("new unit at %d,%d is not next to the champion", u.X, u.Y)
	}
	if _, ok := b.entities[u.ID]; !ok {
		t.Fatal("the new unit is not on the board")
	}
	if out.Points != rules.Points.Create || b.Player("p1").Points != rules.Points.Create {
		t.Fatalf("points = %d", b.Player("p1").Points)
	}
	if len(out.Dirty) != 1 || out.Dirty[0].ID != "champ" {
		t.Fatalf("the champion's health should be persisted, dirty = %v", out.Dirty)
	}
	if _, err := b.Do("p1", create, t0.Add(time.Second)); err != ErrCooldown {
		t.Fatalf("creating has a cooldown, got %v", err)
	}
	if _, err := b.Move("p1", u.ID, Point{u.X, u.Y}, t0); err != ErrSameCell {
		t.Fatalf("the new unit should obey its owner, got %v", err)
	}
}

func TestCreateUnitRules(t *testing.T) {
	create := func(unit string) Action { return Action{Kind: ActionCreate, UnitID: unit} }

	t.Run("only the champion", func(t *testing.T) {
		if _, err := actionBoard().Do("p1", create("minor"), t0); err != ErrChampionOnly {
			t.Fatalf("got %v", err)
		}
	})
	t.Run("too weak", func(t *testing.T) {
		b := actionBoard()
		b.entities["champ"].Health = rules.CreateHealthCost
		if _, err := b.Do("p1", create("champ"), t0); err != ErrTooWeak {
			t.Fatalf("got %v", err)
		}
		if b.entities["champ"].Health != rules.CreateHealthCost {
			t.Fatal("a refused creation must not cost health")
		}
	})
	t.Run("no free cell around", func(t *testing.T) {
		b := actionBoard()
		for dy := -1; dy <= 1; dy++ {
			for dx := -1; dx <= 1; dx++ {
				p := Point{5 + dx, 5 + dy}
				if _, taken := b.cells[mainCell(p.X, p.Y)]; !taken {
					b.Add(&Entity{ID: fmt.Sprintf("wall%d%d", dx, dy), BoardID: "main", Kind: KindItem, X: p.X, Y: p.Y})
				}
			}
		}
		if _, err := b.Do("p1", create("champ"), t0); err != ErrNoSpace {
			t.Fatalf("got %v", err)
		}
	})
}

func TestMoveItem(t *testing.T) {
	move := func(unit, target string, x, y int) Action {
		return Action{Kind: ActionMoveItem, UnitID: unit, TargetID: target, At: Point{x, y}}
	}

	t.Run("pushes the object, frees its old cell and takes the new one", func(t *testing.T) {
		b := actionBoard() // item at (4,5); champion (5,5), vision 5; minor (6,5), speed 2
		out, err := b.Do("p1", move("champ", "item", 4, 8), t0)
		if err != nil {
			t.Fatal(err)
		}
		if item := b.entities["item"]; item.X != 4 || item.Y != 8 {
			t.Fatalf("item at %d,%d, want 4,8", item.X, item.Y)
		}
		if len(out.DirtyItems) != 1 || out.DirtyItems[0].ID != "item" {
			t.Fatalf("the item's position should be persisted, got %v", out.DirtyItems)
		}
		if _, err := b.Move("p1", "minor", Point{4, 5}, t0); err != nil {
			t.Fatalf("the old cell should be free: %v", err)
		}
		if _, err := b.Move("p1", "champ", Point{4, 8}, t0); err != ErrOccupied {
			t.Fatalf("the new cell should be taken, got %v", err)
		}
	})

	t.Run("has a cooldown", func(t *testing.T) {
		b := actionBoard()
		if _, err := b.Do("p1", move("champ", "item", 4, 8), t0); err != nil {
			t.Fatal(err)
		}
		if _, err := b.Do("p1", move("champ", "item", 4, 9), t0.Add(rules.MoveItemCooldown()/2)); err != ErrCooldown {
			t.Fatalf("got %v", err)
		}
		if _, err := b.Do("p1", move("champ", "item", 4, 9), t0.Add(rules.MoveItemCooldown())); err != nil {
			t.Fatalf("should be ready after the cooldown: %v", err)
		}
	})

	t.Run("item beyond the unit's vision", func(t *testing.T) {
		b := actionBoard()
		b.Add(&Entity{ID: "distant", BoardID: "main", Kind: KindItem, X: 15, Y: 15})
		if _, err := b.Do("p1", move("champ", "distant", 15, 16), t0); err != ErrOutOfRange {
			t.Fatalf("got %v", err)
		}
	})

	tests := []struct {
		name string
		act  Action
		want *Error
	}{
		{"destination occupied", move("champ", "item", 5, 5), ErrOccupied},
		{"destination out of the board", move("champ", "item", -1, 5), ErrOutOfBounds},
		{"destination beyond the unit's vision", move("minor", "item", 4, 9), ErrOutOfRange},
		{"same cell", move("champ", "item", 4, 5), ErrSameCell},
		{"only items can be pushed", move("champ", "enemy", 8, 9), ErrInvalidTarget},
		{"unknown item", move("champ", "nope", 5, 6), ErrNoTarget},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			if _, err := actionBoard().Do("p1", tc.act, t0); err != tc.want {
				t.Fatalf("got %v, want %v", err, tc.want)
			}
		})
	}
}
