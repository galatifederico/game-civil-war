package game

import (
	"testing"
	"time"
)

func TestBoundsClampAndOverride(t *testing.T) {
	b := Bounds{Min: 1, Max: 10}
	if b.Clamp(0) != 1 || b.Clamp(5) != 5 || b.Clamp(99) != 10 {
		t.Fatal("Clamp keeps a value inside the bounds")
	}

	w := NewWorld("w", "test")
	five, twenty := 5, 20
	w.AddRace(&Race{ID: "r", Name: "Elfi", Bounds: map[string]BoundsOverride{
		"speed":    {Max: &five},
		"strength": {Min: &five, Max: &twenty},
	}})
	if got := w.BoundsFor("", "speed"); got != (Bounds{1, 10}) {
		t.Fatalf("world bounds for speed = %+v", got)
	}
	if got := w.BoundsFor("r", "speed"); got != (Bounds{1, 5}) {
		t.Fatalf("the race lowers the maximum only: %+v", got)
	}
	if got := w.BoundsFor("r", "strength"); got != (Bounds{5, 20}) {
		t.Fatalf("the race sets both: %+v", got)
	}
	if got := w.BoundsFor("r", "mana"); got != (Bounds{0, 100}) {
		t.Fatalf("an untouched characteristic keeps the world's bounds: %+v", got)
	}
	if got := w.BoundsFor("", "no-such-trait"); got != fallbackBounds {
		t.Fatalf("unknown characteristic: %+v", got)
	}
}

func TestStartingValuesStayInsideTheBounds(t *testing.T) {
	five := 5
	r := &Race{Min: UnitStats{Speed: 8, Health: 5000, Vision: 3, Strength: 15}, Bonus: UnitStats{Speed: 5, Health: 50},
		TraitsMin: map[string]int{"mana": 500}, TraitsBonus: map[string]int{"mana": 5}}
	w := NewWorld("w", "test")
	r.ID = "r"
	r.Bounds = map[string]BoundsOverride{"speed": {Max: &five}}
	w.AddRace(r)
	for i := 0; i < 100; i++ {
		stats, traits := r.roll(func(key string) Bounds { return w.BoundsFor("r", key) })
		if stats.Speed != 5 || stats.Health != 1000 || traits["mana"] != 100 {
			t.Fatalf("stats %+v traits %v not clamped to the bounds (speed 5, health 1000, mana 100)", stats, traits)
		}
	}
}

func TestItemsCannotPushACharacteristicOutOfItsBounds(t *testing.T) {
	w := actionBoard()
	champ := w.entities["champ"]
	champ.Strength = 195
	champ.Traits = map[string]int{"mana": 95}
	w.Player("p1").Inventory = []Item{{ID: "i1", Name: "Elisir", Effect: Effect{Strength: 50, Traits: map[string]int{"mana": 30, "alcol": -20}}}}
	if _, err := w.Do("p1", Action{Kind: ActionUseItem, UnitID: "champ", TargetID: "i1"}, time.Unix(0, 0)); err != nil {
		t.Fatal(err)
	}
	if champ.Strength != 200 || champ.Traits["mana"] != 100 || champ.Traits["alcol"] != 0 {
		t.Fatalf("strength %d, traits %v: want 200, mana 100 and alcol 0 (the bounds)", champ.Strength, champ.Traits)
	}
}

func TestCharacteristicsRules(t *testing.T) {
	r := DefaultRules()
	if b := r.BoundsFor("speed"); b != (Bounds{1, 10}) {
		t.Fatalf("default speed bounds %+v", b)
	}
	r.Characteristics["speed"] = Bounds{Min: 2, Max: 8}
	if got := string(r.Diff()); got != `{"characteristics":{"speed":{"max":8,"min":2}}}` {
		t.Fatalf("the diff keeps the whole entry: %s", got)
	}
	back, err := ParseRules(r.Diff())
	if err != nil || back.Characteristics["speed"] != (Bounds{2, 8}) || back.Characteristics["health"] != (Bounds{1, 1000}) {
		t.Fatalf("round trip: %v %+v", err, back.Characteristics)
	}
	for name, edit := range map[string]func(*Rules){
		"min above max":  func(r *Rules) { r.Characteristics["speed"] = Bounds{9, 3} },
		"speed min zero": func(r *Rules) { r.Characteristics["speed"] = Bounds{0, 5} },
		"unknown name":   func(r *Rules) { r.Characteristics["velocity"] = Bounds{1, 5} },
		"negative":       func(r *Rules) { r.Characteristics["mana"] = Bounds{-1, 5} },
	} {
		r := DefaultRules()
		edit(&r)
		if r.Validate() == nil {
			t.Errorf("%s should be rejected", name)
		}
	}
}
