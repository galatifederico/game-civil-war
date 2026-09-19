package transport

import (
	"net/http"
	"testing"
	"time"

	"thegame/backend/internal/game"
	"thegame/backend/internal/protocol"
)

func raceIDs(t *testing.T, e *env, token string) map[string]string {
	t.Helper()
	ids := map[string]string{}
	for _, w := range worldList(t, e, token) {
		if w["id"] != e.worldID {
			continue
		}
		for _, r := range w["races"].([]any) {
			r := r.(map[string]any)
			ids[r["name"].(string)] = r["id"].(string)
		}
	}
	return ids
}

func TestRacesAreLoadedAndChosenWhenJoining(t *testing.T) {
	e := newEnv(t)
	world, err := e.store.LoadWorld(e.ctx, e.worldID)
	if err != nil {
		t.Fatal(err)
	}
	var names []string
	byName := map[string]*game.Race{}
	for _, r := range world.Races() {
		names = append(names, r.Name)
		byName[r.Name] = r
	}
	if len(names) != 3 || names[0] != "Balordi" || names[1] != "Fighetti" || names[2] != "Sbandati" {
		t.Fatalf("races = %v", names)
	}
	if b := byName["Balordi"]; b.Min.Health != 100 || b.Bonus.Health != 20 || b.TraitsMin["alcol"] != 20 || b.TraitsBonus["soldi"] != 10 {
		t.Fatalf("Balordi = %+v", b)
	}

	token, id := e.register("a@test.io", "Alice")
	ids := raceIDs(t, e, token)
	if len(ids) != 3 {
		t.Fatalf("the lobby should list the 3 races, got %v", ids)
	}

	if status, _ := e.request("POST", "/worlds/"+e.worldID+"/join", token, map[string]string{"race_id": "not-a-race"}); status != http.StatusBadRequest {
		t.Fatalf("joining with a race that does not exist: status %d", status)
	}
	if status, out := e.request("POST", "/worlds/"+e.worldID+"/join", token, map[string]string{"race_id": ids["Fighetti"]}); status != http.StatusOK {
		t.Fatalf("joining as Fighetti: %d %v", status, out)
	}

	snap := e.dial(token).expect("snapshot", ofType(protocol.TypeSnapshot))
	champions := 0
	for _, en := range snap.Entities {
		if en.OwnerID != id {
			continue
		}
		if en.Race != "Fighetti" || en.RaceID != ids["Fighetti"] {
			t.Fatalf("unit of the team has race %q", en.Race)
		}
		if en.Kind == "champion" {
			champions++
			if len(en.Traits) == 0 || en.Traits[0].Name != "soldi" || en.Traits[0].Value != 40 {
				t.Fatalf("champion traits = %+v, want soldi 40 first (Fighetti minimum, in the world's trait order)", en.Traits)
			}
		}
		if en.Kind == "minor" && (en.Speed != 3 || en.Health < 70 || en.Health > 85) {
			t.Fatalf("a Fighetti minor: %+v", en)
		}
	}
	if champions != 1 {
		t.Fatalf("champions = %d", champions)
	}

	// Joining again with another race does not change the team's race.
	e.request("POST", "/worlds/"+e.worldID+"/join", token, map[string]string{"race_id": ids["Balordi"]})
	var raceName string
	if err := e.db.QueryRow(e.ctx, `SELECT r.name FROM memberships m JOIN races r ON r.id = m.race_id WHERE m.player_id::text = $1`, id).Scan(&raceName); err != nil || raceName != "Fighetti" {
		t.Fatalf("membership race = %q (%v), want Fighetti", raceName, err)
	}

	// Without a choice, the world's first race is used.
	tokB, _ := e.register("b@test.io", "Bob")
	e.join(tokB, e.worldID)
	var bobRace string
	if err := e.db.QueryRow(e.ctx, `SELECT r.name FROM memberships m JOIN races r ON r.id = m.race_id JOIN players p ON p.id = m.player_id WHERE p.username = 'Bob'`).Scan(&bobRace); err != nil || bobRace != "Balordi" {
		t.Fatalf("default race = %q (%v), want Balordi", bobRace, err)
	}
}

func TestBreedingAndUsingItemsOverTheWire(t *testing.T) {
	e := newEnv(t, `UPDATE board_items SET effect = '{"points": 40, "strength": 5, "traits": {"mana": 7}}' WHERE name = 'Test Item'`)
	token, id := e.player("a@test.io", "Alice")
	c := e.dial(token)
	snap := c.expect("snapshot", ofType(protocol.TypeSnapshot))

	// --- breeding: two adjacent minors of the team (both Balordi, who breed among themselves).
	var minors []protocol.Entity
	var champ protocol.Entity
	for _, en := range snap.Entities {
		switch {
		case en.OwnerID == id && en.Kind == "minor":
			minors = append(minors, en)
		case en.OwnerID == id && en.Kind == "champion":
			champ = en
		}
	}
	var a, b protocol.Entity
	for i := range minors {
		for j := range minors {
			dx, dy := minors[i].X-minors[j].X, minors[i].Y-minors[j].Y
			if i != j && dx >= -1 && dx <= 1 && dy >= -1 && dy <= 1 {
				a, b = minors[i], minors[j]
			}
		}
	}
	if a.ID == "" {
		t.Fatal("no two adjacent minors in the starting team")
	}
	c.send(protocol.ClientMessage{Type: protocol.TypeBreed, UnitID: a.ID, TargetID: b.ID})
	born := c.expect("the child", func(m protocol.ServerMessage) bool {
		return m.Type == protocol.TypeDelta && len(m.Entities) > 0 && len(m.Scores) > 0
	})
	var child protocol.Entity
	for _, en := range born.Entities {
		if en.ID != a.ID && en.ID != b.ID && en.Kind == "minor" {
			child = en
		}
	}
	if child.ID == "" || child.Race != "Balordi" || child.OwnerID != id {
		t.Fatalf("child = %+v in %+v", child, born.Entities)
	}
	c.send(protocol.ClientMessage{Type: protocol.TypeBreed, UnitID: a.ID, TargetID: b.ID})
	if m := c.expect("rest error", ofType(protocol.TypeError)); m.Code != "not_rested" {
		t.Fatalf("breeding again at once: code %q", m.Code)
	}
	var born1 int
	deadline := time.Now().Add(3 * time.Second)
	for {
		e.db.QueryRow(e.ctx, `SELECT count(*) FROM units WHERE id::text = $1 AND race_id IS NOT NULL`, child.ID).Scan(&born1)
		if born1 == 1 || time.Now().After(deadline) {
			break
		}
		time.Sleep(50 * time.Millisecond)
	}
	if born1 != 1 {
		t.Fatal("the child was not saved with its race")
	}

	// --- creating a unit with items needs 2 items; only 1 exists so far in reach, so it is refused.
	c.send(protocol.ClientMessage{Type: protocol.TypeCreate, UnitID: champ.ID, Method: "resources"})
	if m := c.expect("no items", ofType(protocol.TypeError)); m.Code != "no_items" {
		t.Fatalf("creating with resources and an empty inventory: code %q", m.Code)
	}

	// --- items: pick up the test item, then use it.
	var item protocol.Entity
	for _, en := range snap.Entities {
		if en.Name == "Test Item" {
			item = en
		}
	}
	c.send(protocol.ClientMessage{Type: protocol.TypePickup, UnitID: champ.ID, TargetID: item.ID})
	inv := c.expect("inventory", ofType(protocol.TypeInventory))
	if len(inv.Inventory) != 1 || inv.Inventory[0].ID == "" || inv.Inventory[0].Effect == "" {
		t.Fatalf("inventory = %+v, want the item with an id and an effect description", inv.Inventory)
	}
	time.Sleep(game.DefaultRules().PickupCooldown() + 100*time.Millisecond)
	c.send(protocol.ClientMessage{Type: protocol.TypeUseItem, UnitID: b.ID, TargetID: inv.Inventory[0].ID})
	if m := c.expect("champion only", ofType(protocol.TypeError)); m.Code != "champion_only" {
		t.Fatalf("a minor using an item: code %q", m.Code)
	}
	c.send(protocol.ClientMessage{Type: protocol.TypeUseItem, UnitID: champ.ID, TargetID: inv.Inventory[0].ID})
	after := c.expect("champion after use", func(m protocol.ServerMessage) bool {
		en, ok := findEntity(m.Entities, func(en protocol.Entity) bool { return en.ID == champ.ID })
		return m.Type == protocol.TypeDelta && ok && en.Strength == 35
	})
	if len(after.Scores) == 0 {
		t.Fatal("the item's points should reach the scoreboard")
	}
	if used := c.expect("empty inventory", ofType(protocol.TypeInventory)); len(used.Inventory) != 0 {
		t.Fatalf("the item should be used up, inventory = %+v", used.Inventory)
	}

	// Everything is saved: the champion's strength and mana, the empty inventory, the points.
	deadline = time.Now().Add(3 * time.Second)
	for {
		var strength, mana, inventory, points int
		e.db.QueryRow(e.ctx, `SELECT strength, COALESCE((traits->>'mana')::int, 0) FROM units WHERE id::text = $1`, champ.ID).Scan(&strength, &mana)
		e.db.QueryRow(e.ctx, `SELECT count(*) FROM inventory_items WHERE player_id::text = $1`, id).Scan(&inventory)
		e.db.QueryRow(e.ctx, `SELECT points FROM memberships WHERE player_id::text = $1 AND world_id::text = $2`, id, e.worldID).Scan(&points)
		want := game.DefaultRules().Points.Breed + game.DefaultRules().Points.Pickup + 40
		if strength == 35 && mana >= 7 && inventory == 0 && points == want {
			break
		}
		if time.Now().After(deadline) {
			t.Fatalf("database: strength=%d mana=%d inventory=%d points=%d (want points %d)", strength, mana, inventory, points, want)
		}
		time.Sleep(50 * time.Millisecond)
	}
}
