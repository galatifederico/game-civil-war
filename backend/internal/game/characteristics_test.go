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
	if got := w.BoundsFor("", "", "speed"); got != (Bounds{1, 10}) {
		t.Fatalf("world bounds for speed = %+v", got)
	}
	if got := w.BoundsFor("r", "", "speed"); got != (Bounds{1, 5}) {
		t.Fatalf("the race lowers the maximum only: %+v", got)
	}
	if got := w.BoundsFor("r", "", "strength"); got != (Bounds{5, 20}) {
		t.Fatalf("the race sets both: %+v", got)
	}
	if got := w.BoundsFor("r", "", "mana"); got != (Bounds{0, 100}) {
		t.Fatalf("an untouched characteristic keeps the world's bounds: %+v", got)
	}
	if got := w.BoundsFor("", "", "no-such-trait"); got != fallbackBounds {
		t.Fatalf("unknown characteristic: %+v", got)
	}
}

func TestClassBoundsUnionWithRaceTakingTheGreaterLimit(t *testing.T) {
	w := NewWorld("w", "test")
	five, eight, three, twelve := 5, 8, 3, 12
	w.AddRace(&Race{ID: "r", Name: "Elfi", Bounds: map[string]BoundsOverride{
		"speed":  {Max: &five},               // race: speed <= 5 (world default min 1)
		"vision": {Min: &three, Max: &eight}, // race: 3 <= vision <= 8
	}})

	// No class: same as the race alone.
	if got := w.BoundsFor("r", "", "speed"); got != (Bounds{1, 5}) {
		t.Fatalf("no class: %+v", got)
	}
	if got := w.BoundsFor("r", "unknown-class", "speed"); got != (Bounds{1, 5}) {
		t.Fatalf("an unknown class id is ignored: %+v", got)
	}

	// A class with a higher max widens the ceiling; a class untouching a key changes nothing.
	w.AddClass(&Class{ID: "c", Name: "Guerriero", Bounds: map[string]BoundsOverride{
		"speed": {Max: &eight}, // class: speed <= 8, greater than the race's 5
	}})
	if got := w.BoundsFor("r", "c", "speed"); got != (Bounds{1, 8}) {
		t.Fatalf("the class's greater max should win: %+v", got)
	}
	if got := w.BoundsFor("r", "c", "vision"); got != (Bounds{3, 8}) {
		t.Fatalf("a key the class does not touch keeps the race's bounds: %+v", got)
	}
	// With no race override the world's default (max 10) already beats the class's narrower 8; a
	// class that widens past the world's default does take effect, with no race involved at all.
	if got := w.BoundsFor("", "c", "speed"); got != (Bounds{1, 10}) {
		t.Fatalf("the union never narrows below the world's own default: %+v", got)
	}
	twentyFive := 25
	w.AddClass(&Class{ID: "giant", Name: "Gigante", Bounds: map[string]BoundsOverride{"speed": {Max: &twentyFive}}})
	if got := w.BoundsFor("", "giant", "speed"); got != (Bounds{1, 25}) {
		t.Fatalf("a class alone can still widen past the world's default: %+v", got)
	}

	// A class with a lower max never narrows what the race already allows (max(5, 3) = 5).
	w.AddClass(&Class{ID: "weak", Name: "Debole", Bounds: map[string]BoundsOverride{"speed": {Max: &three}}})
	if got := w.BoundsFor("r", "weak", "speed"); got != (Bounds{1, 5}) {
		t.Fatalf("the union always keeps the greater max: %+v", got)
	}
	// A higher class minimum raises the floor, even above the race's own minimum. The class's own
	// max is untouched by it, so it falls back to the world's default (12) for the union, which
	// also beats the race's max (8).
	w.AddClass(&Class{ID: "tough", Name: "Robusto", Bounds: map[string]BoundsOverride{"vision": {Min: &twelve}}})
	if got := w.BoundsFor("r", "tough", "vision"); got != (Bounds{12, 12}) {
		t.Fatalf("the union always keeps the greater min too: %+v", got)
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
		stats, traits := r.roll(func(key string) Bounds { return w.BoundsFor("r", "", key) })
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

func TestUnitsShowEveryCharacteristicEvenAtZero(t *testing.T) {
	loop, _, p1, _ := fogWorld(t)
	loop.Register(p1)
	m := recv(t, p1)
	names := DefaultRules().TraitNames
	for _, en := range m.Entities {
		switch en.Kind {
		case "champion", "minor", "npc":
			if len(en.Traits) != len(names) {
				t.Fatalf("%s has %d characteristics, want all %d (zeros included): %+v", en.Name, len(en.Traits), len(names), en.Traits)
			}
			for i, tr := range en.Traits {
				if tr.Name != names[i] {
					t.Fatalf("%s: characteristic %d is %q, want %q (the world's order)", en.Name, i, tr.Name, names[i])
				}
			}
		default:
			if len(en.Traits) != 0 {
				t.Fatalf("%s (%s) should have no characteristics", en.Name, en.Kind)
			}
		}
	}
}

func TestUnitSprites(t *testing.T) {
	for _, tc := range []struct {
		kind Kind
		look string
		want string
	}{
		{KindChampion, "", "champion"}, {KindMinor, "", "minor"},
		{KindChampion, "azure", "champion-azure"}, {KindMinor, "moss", "minor-moss"},
	} {
		if got := UnitSprite(tc.kind, tc.look); got != tc.want {
			t.Errorf("UnitSprite(%v, %q) = %q, want %q", tc.kind, tc.look, got, tc.want)
		}
	}
	if !ValidLook("") || !ValidLook("violet") || ValidLook("neon") {
		t.Error("ValidLook: empty and the known looks are valid, others are not")
	}
	if !ValidSprite("guard") || !ValidSprite("minor-slate") || ValidSprite("champion") || ValidSprite("dragon") {
		t.Error("ValidSprite: NPC characters and plain pawns only")
	}
	for _, name := range SpriteChoices() {
		if !ValidSprite(name) {
			t.Errorf("%q is offered but not valid", name)
		}
	}
}
