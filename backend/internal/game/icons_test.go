package game

import "testing"

func TestResolveIcon(t *testing.T) {
	tests := []struct {
		icon string
		fx   Effect
		want string
	}{
		{"chest", Effect{Heal: 5}, "chest"}, // an explicit icon wins
		{"", Effect{Heal: 30}, "potion"},
		{"", Effect{Points: 10}, "coin"},
		{"", Effect{Points: -10}, "coin"},
		{"", Effect{Strength: 2}, "sword"},
		{"", Effect{Traits: map[string]int{"mana": 5}}, "gem"},
		{"", Effect{}, "box"},
	}
	for _, tc := range tests {
		if got := ResolveIcon(tc.icon, tc.fx); got != tc.want {
			t.Errorf("ResolveIcon(%q, %+v) = %q, want %q", tc.icon, tc.fx, got, tc.want)
		}
		if !ValidIcon(ResolveIcon(tc.icon, tc.fx)) {
			t.Errorf("the resolved icon of %+v is not one of ItemIcons", tc)
		}
	}
	if !ValidIcon("") || ValidIcon("banana") {
		t.Error("ValidIcon: empty is valid (automatic), unknown keys are not")
	}
}

func TestPickedUpItemKeepsItsIcon(t *testing.T) {
	w := actionBoard()
	w.entities["item"].Icon = "chest"
	if _, err := w.Do("p1", Action{Kind: ActionPickup, UnitID: "champ", TargetID: "item"}, t0); err != nil {
		t.Fatal(err)
	}
	inv := w.Player("p1").Inventory
	if len(inv) != 1 || inv[0].Icon != "chest" || inv[0].ResolvedIcon() != "chest" {
		t.Fatalf("inventory = %+v", inv)
	}
}
