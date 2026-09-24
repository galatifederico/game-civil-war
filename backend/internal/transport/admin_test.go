package transport

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"strings"
	"testing"
	"time"

	"thegame/backend/internal/game"
	"thegame/backend/internal/protocol"
	"thegame/backend/internal/store"
)

// adminCall calls the admin API and returns the status and the raw body.
func (e *env) adminCall(method, path, token string, body any) (int, []byte) {
	e.t.Helper()
	var reader io.Reader
	if body != nil {
		b, _ := json.Marshal(body)
		reader = bytes.NewReader(b)
	}
	req, err := http.NewRequest(method, e.server.URL+"/admin/api/worlds/"+e.worldID+path, reader)
	if err != nil {
		e.t.Fatal(err)
	}
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		e.t.Fatal(err)
	}
	defer resp.Body.Close()
	data, _ := io.ReadAll(resp.Body)
	return resp.StatusCode, data
}

func (e *env) definition(token string) store.WorldDefinition {
	e.t.Helper()
	status, data := e.adminCall("GET", "", token, nil)
	if status != http.StatusOK {
		e.t.Fatalf("GET definition: %d %s", status, data)
	}
	var d store.WorldDefinition
	if err := json.Unmarshal(data, &d); err != nil {
		e.t.Fatal(err)
	}
	return d
}

// save saves a list and expects the answer status; for failures it returns the error message.
func (e *env) save(token, list string, items any, want int) string {
	e.t.Helper()
	status, data := e.adminCall("PUT", "/"+list, token, map[string]any{"items": items})
	if status != want {
		e.t.Fatalf("PUT %s: status %d, want %d: %s", list, status, want, data)
	}
	var out struct {
		Error string `json:"error"`
	}
	json.Unmarshal(data, &out)
	return out.Error
}

func TestAdminAccess(t *testing.T) {
	e := newEnv(t)
	owner, _ := e.register("owner@test.io", "Owner") // the first player administers the seeded world
	other, _ := e.register("other@test.io", "Other")

	if status, _ := e.adminCall("GET", "", "", nil); status != http.StatusUnauthorized {
		t.Fatalf("no session: status %d", status)
	}
	if status, _ := e.adminCall("GET", "", other, nil); status != http.StatusForbidden {
		t.Fatalf("not the admin: status %d", status)
	}
	if status, _ := e.adminCall("PUT", "/boards", other, map[string]any{"items": []any{}}); status != http.StatusForbidden {
		t.Fatalf("editing as not the admin: status %d", status)
	}
	if status, _ := e.adminCall("GET", "", owner, nil); status != http.StatusOK {
		t.Fatalf("the admin: status %d", status)
	}
	if status, _ := e.request("GET", "/admin/api/worlds/00000000-0000-0000-0000-000000000000", owner, nil); status != http.StatusNotFound {
		t.Fatalf("unknown world: status %d", status)
	}

	d := e.definition(owner)
	if len(d.Boards) != 3 || len(d.Races) != 3 || len(d.Goals) != 8 || len(d.Links) != 16 || len(d.NPCs) != 8 || len(d.Items) != 7 {
		t.Fatalf("definition: %d boards, %d races, %d goals, %d links, %d npcs, %d items",
			len(d.Boards), len(d.Races), len(d.Goals), len(d.Links), len(d.NPCs), len(d.Items))
	}
}

func TestAdminRulesTakeEffect(t *testing.T) {
	e := newEnv(t)
	owner, _ := e.register("owner@test.io", "Owner")

	status, data := e.adminCall("PUT", "/rules", owner, map[string]any{"minors_per_team": 3, "cooldowns_ms": map[string]int{"attack": 800}})
	if status != http.StatusOK {
		t.Fatalf("saving rules: %d %s", status, data)
	}
	if got := string(e.definition(owner).Rules); got != `{"cooldowns_ms":{"attack":800},"minors_per_team":3}` {
		t.Fatalf("stored rules = %s: only the differences from the defaults should be kept", got)
	}

	// The world restarted with the new rules: a new team is smaller.
	tok, pid := e.player("p@test.io", "Player")
	snap := e.dial(tok).expect("snapshot", ofType(protocol.TypeSnapshot))
	mine := 0
	for _, en := range snap.Entities {
		if en.OwnerID == pid {
			mine++
		}
	}
	if mine != 4 {
		t.Fatalf("the team has %d units, want a champion and 3", mine)
	}

	for name, body := range map[string]map[string]any{
		"invalid value": {"champion": map[string]int{"speed": 0}},
		"unknown field": {"champions_per_team": 2},
		"bad goal mode": {"goal_assignment": "sometimes"},
	} {
		status, data := e.adminCall("PUT", "/rules", owner, body)
		if status != http.StatusBadRequest {
			t.Fatalf("%s: status %d %s", name, status, data)
		}
	}
	if got := string(e.definition(owner).Rules); !strings.Contains(got, `"minors_per_team":3`) {
		t.Fatalf("a rejected change altered the rules: %s", got)
	}
}

func TestAdminEditsReachPlayersAndKeepTheirProgress(t *testing.T) {
	e := newEnv(t)
	owner, _ := e.register("owner@test.io", "Owner")
	tok, pid := e.player("p@test.io", "Player")

	c := e.dial(tok)
	snap := c.expect("snapshot", ofType(protocol.TypeSnapshot))
	var champ protocol.Entity
	for _, en := range snap.Entities {
		if en.OwnerID == pid && en.Kind == "champion" {
			champ = en
		}
	}
	occupied := map[[2]int]bool{}
	for _, en := range snap.Entities {
		if en.BoardID == champ.BoardID {
			occupied[[2]int{en.X, en.Y}] = true
		}
	}
	var to [2]int
	for dy := -3; dy <= 3 && to == [2]int{}; dy++ {
		for dx := -3; dx <= 3; dx++ {
			cell := [2]int{champ.X + dx, champ.Y + dy}
			if cell[0] >= 1 && cell[1] >= 1 && !occupied[cell] {
				to = cell
				break
			}
		}
	}
	c.send(protocol.ClientMessage{Type: protocol.TypeMove, UnitID: champ.ID, X: to[0], Y: to[1]})
	if m := c.expect("move answer", func(m protocol.ServerMessage) bool {
		return m.Type == protocol.TypeDelta || m.Type == protocol.TypeError
	}); m.Type == protocol.TypeError {
		t.Fatalf("the move was refused: %+v (from %d,%d to %v)", m, champ.X, champ.Y, to)
	}

	// The admin adds an NPC right away: the world restarts, the player is disconnected...
	d := e.definition(owner)
	var piazza string
	for _, b := range d.Boards {
		if b.Name == "Piazza" {
			piazza = b.ID
		}
	}
	npcs := append(d.NPCs, store.AdminNPC{BoardID: piazza, Name: "Oste", Description: "Serve da bere.", X: 20, Y: 20, Health: 50, Vision: 2, Dialogue: "Cosa ti porto?"})
	e.save(owner, "npcs", npcs, http.StatusOK)
	select {
	case _, ok := <-c.msgs:
		for ok {
			_, ok = <-c.msgs
		}
	case <-time.After(3 * time.Second):
		t.Fatal("the player was not disconnected when the world restarted")
	}

	// ...and back in, they see their unit where they left it (nothing was lost in the restart).
	c2 := e.dial(tok)
	snap2 := c2.expect("snapshot", ofType(protocol.TypeSnapshot))
	moved, _ := findEntity(snap2.Entities, func(en protocol.Entity) bool { return en.ID == champ.ID })
	if moved.X != to[0] || moved.Y != to[1] {
		t.Fatalf("champion at (%d,%d) after the restart, want %v", moved.X, moved.Y, to)
	}
	if got := len(e.definition(owner).NPCs); got != len(d.NPCs)+1 {
		t.Fatalf("NPCs = %d after adding one to %d", got, len(d.NPCs))
	}
}

func TestAdminBoardsAndPassages(t *testing.T) {
	e := newEnv(t)
	owner, _ := e.register("owner@test.io", "Owner")
	e.player("p@test.io", "Player")
	d := e.definition(owner)

	// A new board, then a passage to it.
	// Hexagonal boards no longer exist.
	e.save(owner, "boards", append(d.Boards, store.AdminBoard{Name: "Esagono", Width: 8, Height: 8, Grid: "hex"}), http.StatusBadRequest)
	boards := append(d.Boards, store.AdminBoard{Name: "Grotta", Width: 8, Height: 8, Grid: "square"})
	e.save(owner, "boards", boards, http.StatusOK)
	d = e.definition(owner)
	var grotta, piazza string
	for _, b := range d.Boards {
		switch b.Name {
		case "Grotta":
			grotta = b.ID
			if b.Grid != "square" || b.Width != 8 {
				t.Fatalf("new board = %+v", b)
			}
		case "Piazza":
			piazza = b.ID
		}
	}
	if grotta == "" {
		t.Fatal("the new board was not created")
	}
	links := append(d.Links, store.AdminLink{FromBoard: piazza, FromX: 22, FromY: 22, ToBoard: grotta, ToX: 0, ToY: 0})
	e.save(owner, "links", links, http.StatusOK)

	// Mistakes are reported without changing anything.
	bad := map[string]struct {
		list  string
		items any
	}{
		"no boards":           {"boards", []store.AdminBoard{}},
		"too big":             {"boards", []store.AdminBoard{{ID: piazza, Name: "Piazza", Width: 500, Height: 4, Grid: "square"}}},
		"bad grid":            {"boards", append(d.Boards, store.AdminBoard{Name: "X", Width: 4, Height: 4, Grid: "round"})},
		"same name":           {"boards", append(d.Boards, store.AdminBoard{Name: "piazza", Width: 4, Height: 4, Grid: "square"})},
		"passage out of side": {"links", append(d.Links, store.AdminLink{FromBoard: piazza, FromX: 99, FromY: 0, ToBoard: grotta, ToX: 0, ToY: 1})},
		"passage to nowhere":  {"links", append(d.Links, store.AdminLink{FromBoard: piazza, FromX: 1, FromY: 1, ToBoard: "nope", ToX: 0, ToY: 1})},
		"duplicate passage":   {"links", append(links, store.AdminLink{FromBoard: piazza, FromX: 22, FromY: 22, ToBoard: grotta, ToX: 1, ToY: 1})},
	}
	for name, c := range bad {
		if msg := e.save(owner, c.list, c.items, http.StatusBadRequest); msg == "" {
			t.Fatalf("%s: no explanation given", name)
		}
	}
	// The Piazza cannot shrink below where the NPCs and the players' units are.
	small := []store.AdminBoard{{ID: piazza, Name: "Piazza", Width: 3, Height: 3, Grid: "square"}}
	for _, b := range d.Boards {
		if b.ID != piazza {
			small = append(small, b)
		}
	}
	if msg := e.save(owner, "boards", small, http.StatusBadRequest); !strings.Contains(msg, "più piccola") {
		t.Fatalf("shrinking a board under its content: %q", msg)
	}
	if got := len(e.definition(owner).Boards); got != 4 {
		t.Fatalf("boards = %d after the failed changes, want 4", got)
	}

	// Deleting an empty board takes its passages with it; the Piazza holds the player's team.
	var without []store.AdminBoard
	for _, b := range e.definition(owner).Boards {
		if b.ID != grotta {
			without = append(without, b)
		}
	}
	e.save(owner, "boards", without, http.StatusOK)
	d = e.definition(owner)
	if len(d.Boards) != 3 || len(d.Links) != 16 {
		t.Fatalf("after deleting the Grotta: %d boards, %d links", len(d.Boards), len(d.Links))
	}
	var withoutPiazza []store.AdminBoard
	for _, b := range d.Boards {
		if b.ID != piazza {
			withoutPiazza = append(withoutPiazza, b)
		}
	}
	if msg := e.save(owner, "boards", withoutPiazza, http.StatusBadRequest); !strings.Contains(msg, "giocatori") {
		t.Fatalf("deleting a board with a team on it: %q", msg)
	}
}

func TestAdminRacesGoalsNPCsAndItems(t *testing.T) {
	e := newEnv(t)
	owner, _ := e.register("owner@test.io", "Owner")
	e.player("p@test.io", "Player") // takes the first race
	d := e.definition(owner)

	// Races: a new one can be added, the one in use cannot be deleted, unknown traits are refused.
	races := append(d.Races, store.AdminRace{Name: "Elfi", Speed: 3, Health: 80, Vision: 4, Strength: 10, TraitsMin: map[string]int{"mana": 30}})
	e.save(owner, "races", races, http.StatusOK)
	d = e.definition(owner)
	if len(d.Races) != 4 {
		t.Fatalf("races = %d", len(d.Races))
	}
	elfi := d.Races[3]
	if elfi.Name != "Elfi" || elfi.TraitsMin["mana"] != 30 {
		t.Fatalf("new race = %+v", elfi)
	}
	e.save(owner, "races", append(d.Races[:3:3], store.AdminRace{ID: elfi.ID, Name: "Elfi", Speed: 3, Health: 80, Vision: 4, TraitsMin: map[string]int{"fortuna": 1}}), http.StatusBadRequest)
	e.save(owner, "races", d.Races[1:], http.StatusBadRequest) // the first race belongs to the player
	e.save(owner, "races", d.Races[:3], http.StatusOK)         // an unused one can go
	d = e.definition(owner)
	if len(d.Races) != 3 {
		t.Fatalf("races = %d after deleting Elfi", len(d.Races))
	}
	e.save(owner, "compat", []store.AdminCompat{{RaceA: d.Races[0].ID, RaceB: d.Races[0].ID, Child: d.Races[1].ID}, {RaceA: d.Races[0].ID, RaceB: d.Races[0].ID, Child: d.Races[1].ID}}, http.StatusBadRequest)
	e.save(owner, "compat", []store.AdminCompat{{RaceA: d.Races[0].ID, RaceB: d.Races[1].ID, Child: d.Races[2].ID}}, http.StatusOK)
	if got := e.definition(owner).Compat; len(got) != 1 || got[0].Child != d.Races[2].ID {
		t.Fatalf("compat = %+v", got)
	}

	// Goals.
	goals := append(d.Goals, store.AdminGoal{Scope: "individual", Kind: "units", Target: 15, Reward: 50, Title: "Esercito"})
	e.save(owner, "goals", goals, http.StatusOK)
	d = e.definition(owner)
	if len(d.Goals) != 9 || d.Goals[8].Title != "Esercito" {
		t.Fatalf("goals = %+v", d.Goals)
	}
	e.save(owner, "goals", append(d.Goals[:8:8], store.AdminGoal{ID: d.Goals[8].ID, Scope: "individual", Kind: "dancing", Target: 1, Title: "Balla"}), http.StatusBadRequest)
	e.save(owner, "goals", append(d.Goals[:8:8], store.AdminGoal{ID: d.Goals[8].ID, Scope: "individual", Kind: "units", Target: 0, Title: "Niente"}), http.StatusBadRequest)
	e.save(owner, "goals", d.Goals[:8], http.StatusOK)

	// NPCs and items: cells must be free and inside the board.
	var piazza string
	for _, b := range d.Boards {
		if b.Name == "Piazza" {
			piazza = b.ID
		}
	}
	taken := d.NPCs[0]
	item := store.AdminItem{BoardID: piazza, Name: "Pozione", X: taken.X, Y: taken.Y}
	if taken.BoardID != piazza {
		t.Skip("the first NPC is not in the Piazza")
	}
	e.save(owner, "items", append(d.Items, item), http.StatusBadRequest) // an NPC stands there
	item.X, item.Y = 21, 21
	item.Effect.Heal = 40
	e.save(owner, "items", append(d.Items, item), http.StatusOK)
	e.save(owner, "items", append(e.definition(owner).Items, store.AdminItem{BoardID: piazza, Name: "Fuori", X: 24, Y: 0}), http.StatusBadRequest)
	got := e.definition(owner).Items
	if len(got) != len(d.Items)+1 {
		t.Fatalf("items = %d", len(got))
	}
	var pozione store.AdminItem
	for _, it := range got {
		if it.Name == "Pozione" {
			pozione = it
		}
	}
	if pozione.Effect.Heal != 40 {
		t.Fatalf("the item's effect was lost: %+v", pozione)
	}

	// Icons: the seeded items have theirs, a made-up one is refused, and one can be set.
	icons := map[string]string{}
	for _, it := range got {
		icons[it.Name] = it.Icon
	}
	if icons["Forziere"] != "chest" || icons["Cristallo"] != "gem" || icons["Pozione"] != "" {
		t.Fatalf("icons = %v", icons)
	}
	withIcon := append([]store.AdminItem(nil), got...)
	for i := range withIcon {
		if withIcon[i].Name == "Pozione" {
			withIcon[i].Icon = "banana"
		}
	}
	e.save(owner, "items", withIcon, http.StatusBadRequest)
	for i := range withIcon {
		if withIcon[i].Name == "Pozione" {
			withIcon[i].Icon = "potion"
		}
	}
	e.save(owner, "items", withIcon, http.StatusOK)
	for _, it := range e.definition(owner).Items {
		if it.Name == "Pozione" && it.Icon != "potion" {
			t.Fatalf("the icon was not saved: %+v", it)
		}
	}

	// Every NPC has a sprite; a made-up one is refused and a known one is saved.
	sprites := map[string]string{}
	for _, n := range d.NPCs {
		sprites[n.Name] = n.Sprite
	}
	if sprites["Mercante"] != "merchant" || sprites["Guardia"] != "guard" || sprites["Fabbro"] != "blacksmith" {
		t.Fatalf("seeded NPC sprites = %v", sprites)
	}
	withSprite := append([]store.AdminNPC(nil), d.NPCs...)
	withSprite[0].Sprite = "dragon"
	e.save(owner, "npcs", withSprite, http.StatusBadRequest)
	withSprite[0].Sprite = "guard"
	e.save(owner, "npcs", withSprite, http.StatusOK)
	if got := e.definition(owner).NPCs[0].Sprite; got != "guard" {
		t.Fatalf("the sprite was not saved: %q", got)
	}
	d = e.definition(owner)

	// Two NPCs can swap places in one save.
	a, b := d.NPCs[0], d.NPCs[1]
	if a.BoardID == b.BoardID {
		a.X, a.Y, b.X, b.Y = b.X, b.Y, a.X, a.Y
		e.save(owner, "npcs", append([]store.AdminNPC{a, b}, d.NPCs[2:]...), http.StatusOK)
	}
}

func TestAdminAssignsGoals(t *testing.T) {
	e := newEnv(t)
	owner, _ := e.register("owner@test.io", "Owner")
	if status, data := e.adminCall("PUT", "/rules", owner, map[string]any{"goal_assignment": "manual"}); status != http.StatusOK {
		t.Fatalf("rules: %d %s", status, data)
	}
	tok, pid := e.player("p@test.io", "Player")
	c := e.dial(tok)
	snap := c.expect("snapshot", ofType(protocol.TypeSnapshot))
	for _, g := range snap.Goals {
		if g.Scope == "individual" && g.Title != "" {
			// with manual assignment nothing is handed out on its own
			t.Fatalf("a goal was assigned by itself: %+v", g)
		}
	}

	d := e.definition(owner)
	var goal store.AdminGoal
	for _, g := range d.Goals {
		if g.Scope == "individual" {
			goal = g
			break
		}
	}
	status, data := e.adminCall("POST", "/assign", owner, map[string]string{"player_id": pid, "goal_id": goal.ID})
	if status != http.StatusOK {
		t.Fatalf("assign: %d %s", status, data)
	}
	m := c.expect("the assigned goal", func(m protocol.ServerMessage) bool {
		if m.Type != protocol.TypeGoals {
			return false
		}
		for _, g := range m.Goals {
			if g.ID == goal.ID {
				return true
			}
		}
		return false
	})
	_ = m
	if status, _ := e.adminCall("POST", "/assign", owner, map[string]string{"player_id": pid, "goal_id": "nope"}); status != http.StatusBadRequest {
		t.Fatalf("assigning an unknown goal: status %d", status)
	}
	// It is saved (the write queue is drained when the world restarts).
	e.adminCall("PUT", "/meta", owner, map[string]string{"name": "Rinominato", "description": "Un mondo di prova"})
	got := e.definition(owner)
	if got.Name != "Rinominato" || got.Description != "Un mondo di prova" {
		t.Fatalf("meta = %q %q", got.Name, got.Description)
	}
	var current string
	for _, p := range got.Players {
		if p.ID == pid {
			current = p.GoalID
		}
	}
	if current != goal.ID {
		t.Fatalf("the player's goal in the database = %q, want %q", current, goal.ID)
	}
}

func TestAdminCharacteristicsAndRaceBounds(t *testing.T) {
	e := newEnv(t)
	owner, _ := e.register("owner@test.io", "Owner")
	d := e.definition(owner)

	byKey := func(items []store.AdminCharacteristic) map[string]store.AdminCharacteristic {
		m := map[string]store.AdminCharacteristic{}
		for _, c := range items {
			m[c.Key] = c
		}
		return m
	}
	c := byKey(d.Characteristics)
	if len(d.Characteristics) != 10 || c["speed"].Kind != "base" || c["speed"].Min != 1 || c["speed"].Max != 10 || c["mana"].Kind != "extended" || c["soldi"].Max != 1_000_000 {
		t.Fatalf("default characteristics = %+v", d.Characteristics)
	}
	if d.Characteristics[0].Key != "speed" || d.Characteristics[4].Key != "soldi" {
		t.Fatalf("base ones come first, then the world's own order: %+v", d.Characteristics)
	}

	// Change a default, add a characteristic, remove one.
	var next []store.AdminCharacteristic
	for _, it := range d.Characteristics {
		switch it.Key {
		case "thc":
			continue // removed
		case "speed":
			it.Max = 8
		}
		next = append(next, it)
	}
	next = append(next, store.AdminCharacteristic{Key: "coraggio", Kind: "extended", Min: 0, Max: 50})
	e.save(owner, "characteristics", next, http.StatusOK)
	d = e.definition(owner)
	c = byKey(d.Characteristics)
	if c["speed"].Max != 8 || c["coraggio"].Max != 50 || c["thc"].Key != "" || len(d.Characteristics) != 10 {
		t.Fatalf("after the change: %+v", d.Characteristics)
	}
	if !strings.Contains(string(d.Rules), `"speed":{"max":8,"min":1}`) || !strings.Contains(string(d.Rules), `"coraggio"`) {
		t.Fatalf("the rules store the differences: %s", d.Rules)
	}
	for _, r := range d.Races {
		if _, has := r.TraitsMin["thc"]; has {
			t.Fatalf("race %s still has the removed characteristic: %v", r.Name, r.TraitsMin)
		}
	}

	// Mistakes.
	drop := func(key string) []store.AdminCharacteristic {
		var out []store.AdminCharacteristic
		for _, it := range d.Characteristics {
			if it.Key != key {
				out = append(out, it)
			}
		}
		return out
	}
	e.save(owner, "characteristics", drop("speed"), http.StatusBadRequest)
	e.save(owner, "characteristics", append(d.Characteristics, store.AdminCharacteristic{Key: "Bad Name", Kind: "extended", Max: 5}), http.StatusBadRequest)
	e.save(owner, "characteristics", append(d.Characteristics, store.AdminCharacteristic{Key: "mana", Kind: "extended", Max: 5}), http.StatusBadRequest)
	e.save(owner, "characteristics", append(d.Characteristics, store.AdminCharacteristic{Key: "forza", Kind: "base", Max: 5}), http.StatusBadRequest)
	swapped := append([]store.AdminCharacteristic(nil), d.Characteristics...)
	swapped[0].Min, swapped[0].Max = 9, 3
	e.save(owner, "characteristics", swapped, http.StatusBadRequest)

	// A race can override the bounds; the champion of a new team follows them.
	five, three := 5, 3
	races := d.Races
	races[0].Bounds = map[string]game.BoundsOverride{"speed": {Min: &five, Max: &five}}
	e.save(owner, "races", races, http.StatusOK)
	if got := e.definition(owner).Races[0].Bounds["speed"]; got.Min == nil || *got.Min != 5 {
		t.Fatalf("race bounds were not saved: %+v", got)
	}
	tok, pid := e.player("p@test.io", "Player") // joins with the first race
	snap := e.dial(tok).expect("snapshot", ofType(protocol.TypeSnapshot))
	champ, _ := findEntity(snap.Entities, func(en protocol.Entity) bool { return en.Kind == "champion" && en.OwnerID == pid })
	if champ.Speed != 5 {
		t.Fatalf("the champion's speed is %d: the race's bounds say 5", champ.Speed)
	}

	badRaces := func(o map[string]game.BoundsOverride) []store.AdminRace {
		out := append([]store.AdminRace(nil), e.definition(owner).Races...)
		out[0].Bounds = o
		return out
	}
	e.save(owner, "races", badRaces(map[string]game.BoundsOverride{"velocity": {Max: &five}}), http.StatusBadRequest)
	e.save(owner, "races", badRaces(map[string]game.BoundsOverride{"speed": {Min: &five, Max: &three}}), http.StatusBadRequest)
	// The new default minimum of speed cannot go above what a race allows.
	next = append([]store.AdminCharacteristic(nil), e.definition(owner).Characteristics...)
	for i := range next {
		if next[i].Key == "speed" {
			next[i].Min, next[i].Max = 6, 9
		}
	}
	e.save(owner, "races", badRaces(map[string]game.BoundsOverride{"speed": {Max: &five}}), http.StatusOK)
	e.save(owner, "characteristics", next, http.StatusBadRequest)
}

func TestRaceLooksReachTheClient(t *testing.T) {
	e := newEnv(t)
	owner, _ := e.register("owner@test.io", "Owner")
	d := e.definition(owner)
	if d.Races[0].Look != "salmon" || d.Races[1].Look != "azure" || d.Races[2].Look != "moss" {
		t.Fatalf("seeded looks = %q %q %q", d.Races[0].Look, d.Races[1].Look, d.Races[2].Look)
	}

	spriteOf := func(tok, pid, kind string) string {
		snap := e.dial(tok).expect("snapshot", ofType(protocol.TypeSnapshot))
		en, ok := findEntity(snap.Entities, func(en protocol.Entity) bool { return en.Kind == kind && en.OwnerID == pid })
		if !ok {
			t.Fatalf("no %s in the snapshot", kind)
		}
		return en.Sprite
	}
	tok, pid := e.player("p@test.io", "Player") // the first race
	if got := spriteOf(tok, pid, "champion"); got != "champion-salmon" {
		t.Fatalf("champion sprite = %q", got)
	}
	if got := spriteOf(tok, pid, "minor"); got != "minor-salmon" {
		t.Fatalf("minor sprite = %q", got)
	}

	races := d.Races
	races[0].Look = "azure"
	e.save(owner, "races", races, http.StatusOK)
	if got := spriteOf(tok, pid, "champion"); got != "champion-azure" {
		t.Fatalf("after the admin changed the look: %q", got)
	}
	races[0].Look = "neon"
	e.save(owner, "races", races, http.StatusBadRequest)

	// An NPC can be a plain pawn in a look; a role sprite is not an NPC sprite.
	npcs := e.definition(owner).NPCs
	npcs[0].Sprite = "minor-moss"
	e.save(owner, "npcs", npcs, http.StatusOK)
	npcs[0].Sprite = "champion"
	e.save(owner, "npcs", npcs, http.StatusBadRequest)
}
