package game

import (
	"strings"
	"testing"
	"time"
)

// raceWorld: races Balordi (B), Fighetti (F) and their offspring Sbandati (S); B x B -> B, F x F -> F,
// B x F -> S. Player p1 has a champion and two minors of race B next to each other.
func raceWorld() *World {
	w := newTestWorld("w", "test", 20, 20, []*Entity{
		{ID: "champ", OwnerID: "p1", RaceID: "B", Kind: KindChampion, Name: "Champion", X: 5, Y: 5, Speed: 3, Health: 200, MaxHealth: 200, Vision: 5, Strength: 30},
		{ID: "m1", OwnerID: "p1", RaceID: "B", Kind: KindMinor, Name: "Pedina 1", X: 6, Y: 5, Speed: 2, Health: 100, MaxHealth: 100, Vision: 3, Strength: 15},
		{ID: "m2", OwnerID: "p1", RaceID: "F", Kind: KindMinor, Name: "Pedina 2", X: 7, Y: 5, Speed: 3, Health: 70, MaxHealth: 70, Vision: 4, Strength: 10},
		{ID: "enemy", OwnerID: "p2", RaceID: "B", Kind: KindMinor, Name: "Nemica", X: 6, Y: 6, Speed: 2, Health: 100, MaxHealth: 100, Vision: 3, Strength: 15},
	})
	w.AddRace(&Race{ID: "B", Name: "Balordi", Min: UnitStats{Speed: 2, Health: 100, Vision: 3, Strength: 15}, Bonus: UnitStats{Health: 20, Vision: 1, Strength: 5},
		TraitsMin: map[string]int{"soldi": 5, "alcol": 20}, TraitsBonus: map[string]int{"soldi": 10}})
	w.AddRace(&Race{ID: "F", Name: "Fighetti", Min: UnitStats{Speed: 3, Health: 70, Vision: 4, Strength: 10}})
	w.AddRace(&Race{ID: "S", Name: "Sbandati", Min: UnitStats{Speed: 3, Health: 90, Vision: 3, Strength: 12}, Bonus: UnitStats{Health: 20}})
	w.AddCompat("B", "B", "B")
	w.AddCompat("F", "F", "F")
	w.AddCompat("F", "B", "S") // the order of the pair does not matter
	w.EnsurePlayer("p1", "Anna", 0).RaceID = "B"
	w.EnsurePlayer("p2", "Bob", 0)
	return w
}

func TestTeamsAreBuiltFromTheirRace(t *testing.T) {
	w := raceWorld()
	team, err := w.PlanTeam("p3", "Carla", "B")
	if err != nil {
		t.Fatal(err)
	}
	if team[0].Kind != KindChampion || team[0].RaceID != "B" {
		t.Fatalf("champion = %+v", team[0])
	}
	if team[0].Speed != rules.Champion.Speed || team[0].Health != rules.Champion.Health {
		t.Fatalf("the champion keeps the world's champion stats: %+v", team[0])
	}
	if team[0].Traits["soldi"] != 5 || team[0].Traits["alcol"] != 20 {
		t.Fatalf("the champion starts at the race's minimum traits: %v", team[0].Traits)
	}
	for _, m := range team[1:] {
		if m.RaceID != "B" {
			t.Fatalf("minor race = %q", m.RaceID)
		}
		if m.Speed != 2 || m.Health < 100 || m.Health > 120 || m.Vision < 3 || m.Vision > 4 || m.Strength < 15 || m.Strength > 20 {
			t.Fatalf("minor stats outside the race's range: %+v", m)
		}
		if m.MaxHealth != m.Health {
			t.Fatalf("a new unit starts at full health: %+v", m)
		}
		if s := m.Traits["soldi"]; s < 5 || s > 15 || m.Traits["alcol"] != 20 {
			t.Fatalf("minor traits outside the race's range: %v", m.Traits)
		}
	}
}

func TestTeamsWithoutARaceUseTheWorldsDefaults(t *testing.T) {
	for _, raceID := range []string{"", "no-such-race"} {
		w := raceWorld()
		team, err := w.PlanTeam("p3", "Carla", raceID)
		if err != nil {
			t.Fatal(err)
		}
		m := team[1]
		if m.RaceID != "" || m.Speed != rules.Minor.Speed || m.Health != rules.Minor.Health || m.Traits != nil {
			t.Fatalf("race %q: minor = %+v, want the world's default minor", raceID, m)
		}
	}
}

func TestRandomBonusIsInRangeAndActuallyVaries(t *testing.T) {
	r := &Race{Min: UnitStats{Speed: 2, Health: 100, Vision: 3, Strength: 15}, Bonus: UnitStats{Health: 20}}
	seen := map[int]bool{}
	for i := 0; i < 300; i++ {
		stats, _ := r.roll(DefaultRules().BoundsFor)
		if stats.Health < 100 || stats.Health > 120 || stats.Speed != 2 {
			t.Fatalf("stats out of range: %+v", stats)
		}
		seen[stats.Health] = true
	}
	if len(seen) < 10 {
		t.Fatalf("only %d different health values in 300 rolls: the bonus is not random", len(seen))
	}
}

func TestCreateWithResourcesConsumesInventoryItems(t *testing.T) {
	w := raceWorld()
	w.Player("p1").Inventory = []Item{{ID: "i1", Name: "Uno"}, {ID: "i2", Name: "Due"}, {ID: "i3", Name: "Tre"}}
	create := Action{Kind: ActionCreate, UnitID: "champ", Method: "resources"}

	out, err := w.Do("p1", create, t0)
	if err != nil {
		t.Fatal(err)
	}
	if got := w.entities["champ"].Health; got != 200 {
		t.Fatalf("paying with items must not cost health, got %d", got)
	}
	if inv := w.Player("p1").Inventory; len(inv) != 1 || inv[0].ID != "i3" {
		t.Fatalf("the oldest %d items should be consumed, inventory = %+v", rules.Creation.ResourceItems, inv)
	}
	if len(out.RemovedItems) != 2 || out.RemovedItems[0] != "i1" || !out.InventoryChanged {
		t.Fatalf("outcome = %+v", out)
	}
	if u := out.Created[0]; u.RaceID != "B" || u.OwnerID != "p1" {
		t.Fatalf("the new unit takes the team's race: %+v", u)
	}

	if _, err := w.Do("p1", create, t0.Add(time.Minute)); err != ErrNoItems {
		t.Fatalf("with one item left: got %v", err)
	}
}

func TestCreationMethodsCanBeClosedPerWorld(t *testing.T) {
	w := raceWorld()
	w.Player("p1").Inventory = []Item{{ID: "i1"}, {ID: "i2"}}

	w.Rules.Creation.ResourceItems = 0
	if _, err := w.Do("p1", Action{Kind: ActionCreate, UnitID: "champ", Method: "resources"}, t0); err != ErrDisabled {
		t.Fatalf("resources closed: got %v", err)
	}
	w.Rules.Creation.HealthEnabled = false
	if _, err := w.Do("p1", Action{Kind: ActionCreate, UnitID: "champ"}, t0); err != ErrDisabled {
		t.Fatalf("health closed: got %v", err)
	}
	if _, err := w.Do("p1", Action{Kind: ActionCreate, UnitID: "champ", Method: "magic"}, t0); err != ErrUnknownAction {
		t.Fatalf("unknown method: got %v", err)
	}
	if _, err := w.Do("p1", Action{Kind: ActionCreate, UnitID: "m1", Method: "resources"}, t0); err != ErrChampionOnly {
		t.Fatalf("only the champion creates: got %v", err)
	}
}

func TestBreeding(t *testing.T) {
	breed := func(unit, partner string) Action { return Action{Kind: ActionBreed, UnitID: unit, TargetID: partner} }

	t.Run("compatible parents have a child of the race the world says", func(t *testing.T) {
		w := raceWorld()
		out, err := w.Do("p1", breed("m1", "m2"), t0) // Balordi x Fighetti -> Sbandati
		if err != nil {
			t.Fatal(err)
		}
		child := out.Created[0]
		if child.RaceID != "S" || child.OwnerID != "p1" || child.Kind != KindMinor || child.ID == "" {
			t.Fatalf("child = %+v", child)
		}
		if child.Speed != 3 || child.Health < 90 || child.Health > 110 {
			t.Fatalf("the child's stats come from its own race: %+v", child)
		}
		if _, ok := w.entities[child.ID]; !ok {
			t.Fatal("the child is not in the world")
		}
		if out.Points != rules.Points.Breed || w.Player("p1").Points != rules.Points.Breed {
			t.Fatalf("points = %d", w.Player("p1").Points)
		}
	})

	t.Run("both parents need to rest afterwards", func(t *testing.T) {
		w := raceWorld()
		if _, err := w.Do("p1", breed("m1", "m2"), t0); err != nil {
			t.Fatal(err)
		}
		later := t0.Add(rules.BreedCooldown() / 2)
		if _, err := w.Do("p1", breed("m1", "m2"), later); err != ErrNotRested {
			t.Fatalf("both parents again: got %v", err)
		}
		if _, err := w.Do("p1", breed("champ", "m1"), later); err != ErrNotRested {
			t.Fatalf("one rested parent and one tired: got %v", err)
		}
		if _, err := w.Do("p1", breed("m1", "m2"), t0.Add(rules.BreedCooldown())); err != nil {
			t.Fatalf("after resting: %v", err)
		}
	})

	tests := []struct {
		name   string
		mutate func(*World)
		action Action
		want   *Error
	}{
		{"races that cannot mix", func(w *World) { w.entities["m2"].RaceID = "S" }, breed("m1", "m2"), ErrIncompatible},
		{"a unit without a race", func(w *World) { w.entities["m1"].RaceID = "" }, breed("m1", "m2"), ErrIncompatible},
		{"too far apart", func(w *World) { w.entities["m2"].X = 10 }, breed("m1", "m2"), ErrOutOfRange},
		{"another player's unit", nil, breed("m1", "enemy"), ErrInvalidTarget},
		{"with itself", nil, breed("m1", "m1"), ErrInvalidTarget},
		{"unknown partner", nil, breed("m1", "nope"), ErrNoTarget},
		{"defeated partner", func(w *World) { w.entities["m2"].Health = 0 }, breed("m1", "m2"), ErrTargetDead},
		{"closed in this world", func(w *World) { w.Rules.Creation.BreedingEnabled = false }, breed("m1", "m2"), ErrDisabled},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			w := raceWorld()
			if tc.mutate != nil {
				tc.mutate(w)
			}
			if _, err := w.Do("p1", tc.action, t0); err != tc.want {
				t.Fatalf("got %v, want %v", err, tc.want)
			}
		})
	}
}

func TestBreedRangeIsPerWorld(t *testing.T) {
	w := raceWorld()
	w.entities["m2"].X = 8 // two steps from m1
	if _, err := w.Do("p1", Action{Kind: ActionBreed, UnitID: "m1", TargetID: "m2"}, t0); err != ErrOutOfRange {
		t.Fatalf("with range 1: got %v", err)
	}
	w.Rules.Creation.BreedRange = 2
	if _, err := w.Do("p1", Action{Kind: ActionBreed, UnitID: "m1", TargetID: "m2"}, t0); err != nil {
		t.Fatalf("with range 2: %v", err)
	}
}

func TestUsingItems(t *testing.T) {
	use := func(unit, item string) Action { return Action{Kind: ActionUseItem, UnitID: unit, TargetID: item} }
	setup := func() *World {
		w := raceWorld()
		w.entities["champ"].Health = 150
		w.Player("p1").Inventory = []Item{
			{ID: "potion", Name: "Cristallo", Effect: Effect{Heal: 60, Points: 10, Traits: map[string]int{"mana": 10}}},
			{ID: "axe", Name: "Ascia", Effect: Effect{Strength: 5}},
			{ID: "junk", Name: "Cartello"},
		}
		return w
	}

	t.Run("the effect falls on the champion and the item is used up", func(t *testing.T) {
		w := setup()
		out, err := w.Do("p1", use("champ", "potion"), t0)
		if err != nil {
			t.Fatal(err)
		}
		c := w.entities["champ"]
		if c.Health != 200 {
			t.Fatalf("healing is capped at the maximum: health %d", c.Health)
		}
		if c.Traits["mana"] != 10 {
			t.Fatalf("traits = %v", c.Traits)
		}
		if w.Player("p1").Points != 10 || out.Points != 10 {
			t.Fatalf("points = %d", w.Player("p1").Points)
		}
		if inv := w.Player("p1").Inventory; len(inv) != 2 || inv[0].ID != "axe" {
			t.Fatalf("inventory = %+v", inv)
		}
		if len(out.RemovedItems) != 1 || out.RemovedItems[0] != "potion" || !out.InventoryChanged || len(out.Dirty) != 1 {
			t.Fatalf("outcome = %+v", out)
		}
	})

	t.Run("a permanent bonus", func(t *testing.T) {
		w := setup()
		if _, err := w.Do("p1", use("champ", "axe"), t0); err != nil {
			t.Fatal(err)
		}
		if got := w.entities["champ"].Strength; got != 35 {
			t.Fatalf("strength = %d, want 35", got)
		}
	})

	tests := []struct {
		name   string
		action Action
		want   *Error
	}{
		{"only the champion manages the inventory", use("m1", "potion"), ErrChampionOnly},
		{"an item that is not in the inventory", use("champ", "nope"), ErrNoItem},
		{"an item that does nothing is not wasted", use("champ", "junk"), ErrNoEffect},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			w := setup()
			if _, err := w.Do("p1", tc.action, t0); err != tc.want {
				t.Fatalf("got %v, want %v", err, tc.want)
			}
			if len(w.Player("p1").Inventory) != 3 {
				t.Fatal("a refused use must not consume anything")
			}
		})
	}
}

func TestPickedUpItemsKeepTheirEffect(t *testing.T) {
	w := raceWorld()
	w.Add(&Entity{ID: "crystal", BoardID: "main", Kind: KindItem, Name: "Cristallo", X: 4, Y: 5, Effect: Effect{Heal: 60}})
	out, err := w.Do("p1", Action{Kind: ActionPickup, UnitID: "champ", TargetID: "crystal"}, t0)
	if err != nil {
		t.Fatal(err)
	}
	if out.Picked.ID == "" || out.Picked.Effect.Heal != 60 || !out.InventoryChanged {
		t.Fatalf("picked = %+v", out.Picked)
	}
}

func TestStructuresMakeUnitsWhenTheWorldAllowsIt(t *testing.T) {
	w := raceWorld()
	w.Add(&Entity{ID: "tower", BoardID: "main", OwnerID: "p1", Kind: KindStructure, Name: "Avamposto", X: 15, Y: 15})

	if res := w.Tick(t0); len(res.Spawned) != 0 {
		t.Fatal("building spawns are closed by default")
	}
	w.Rules.Creation.BuildingIntervalMs = 10000
	w.Tick(t0) // starts the clock of the structure
	if res := w.Tick(t0.Add(9 * time.Second)); len(res.Spawned) != 0 {
		t.Fatal("spawned too early")
	}
	res := w.Tick(t0.Add(10 * time.Second))
	if len(res.Spawned) != 1 {
		t.Fatalf("spawned %d units, want 1", len(res.Spawned))
	}
	u := res.Spawned[0]
	if u.OwnerID != "p1" || u.Kind != KindMinor || u.RaceID != "B" || u.ID == "" {
		t.Fatalf("spawned unit = %+v", u)
	}
	if (squareGrid{}).Distance(u.Point(), Point{15, 15}) != 1 {
		t.Fatalf("the unit should appear next to the structure, at %v", u.Point())
	}
	if _, ok := w.entities[u.ID]; !ok {
		t.Fatal("the unit is not in the world")
	}
	if again := w.Tick(t0.Add(11 * time.Second)); len(again.Spawned) != 0 {
		t.Fatal("it must wait for the next interval")
	}
	if again := w.Tick(t0.Add(20 * time.Second)); len(again.Spawned) != 1 {
		t.Fatal("it should spawn again every interval")
	}
}

func TestEffectSummary(t *testing.T) {
	e := Effect{Heal: 40, Points: 30, Strength: 5, Traits: map[string]int{"thc": 10, "beatitudine": -2}}
	got := e.Summary()
	for _, want := range []string{"cura 40", "+30 punti", "+5 forza", "+10 thc", "-2 beatitudine"} {
		if !strings.Contains(got, want) {
			t.Errorf("summary %q lacks %q", got, want)
		}
	}
	if !(Effect{}).IsZero() || e.IsZero() {
		t.Fatal("IsZero is wrong")
	}
}

func TestNewRulesFields(t *testing.T) {
	r, err := ParseRules([]byte(`{"creation": {"resource_items": 5, "breeding_enabled": false, "building_interval_ms": 30000}, "goal_assignment": "manual", "trait_names": ["fame"]}`))
	if err != nil {
		t.Fatal(err)
	}
	if r.Creation.ResourceItems != 5 || r.Creation.BreedingEnabled || r.Creation.BuildingIntervalMs != 30000 || r.GoalAssignment != "manual" || len(r.TraitNames) != 1 {
		t.Fatalf("rules = %+v", r)
	}
	if !r.Creation.HealthEnabled || r.Creation.BreedRange != 1 {
		t.Fatal("what the JSON does not mention keeps its default")
	}
	for _, bad := range []string{`{"goal_assignment": "sometimes"}`, `{"creation": {"breed_range": 0}}`, `{"creation": {"resource_items": -1}}`} {
		if _, err := ParseRules([]byte(bad)); err == nil {
			t.Errorf("%s should be rejected", bad)
		}
	}
}
