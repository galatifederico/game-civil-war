package transport

import (
	"net/http"
	"testing"
	"time"

	"thegame/backend/internal/game"
	"thegame/backend/internal/protocol"
)

func worldList(t *testing.T, e *env, token string) []map[string]any {
	t.Helper()
	status, out := e.request("GET", "/worlds", token, nil)
	if status != http.StatusOK {
		t.Fatalf("GET /worlds: status %d %v", status, out)
	}
	var worlds []map[string]any
	for _, w := range out["worlds"].([]any) {
		worlds = append(worlds, w.(map[string]any))
	}
	return worlds
}

func TestLobbyListsWorldsAndAdmins(t *testing.T) {
	e := newEnv(t)

	if status, _ := e.request("GET", "/worlds", "", nil); status != http.StatusUnauthorized {
		t.Fatalf("listing worlds without a session: status %d", status)
	}
	if status, _ := e.request("GET", "/worlds", "not-a-token", nil); status != http.StatusUnauthorized {
		t.Fatalf("listing worlds with a bad token: status %d", status)
	}

	tokA, _ := e.register("a@test.io", "Alice")
	worlds := worldList(t, e, tokA)
	if len(worlds) != 1 {
		t.Fatalf("worlds = %v", worlds)
	}
	w := worlds[0]
	if w["id"] != e.worldID || w["boards"] != float64(3) || w["players"] != float64(0) || w["joined"] != false {
		t.Fatalf("before joining: %v", w)
	}
	if w["admin"] != true {
		t.Fatal("the first player to register should administer the seeded world")
	}

	e.join(tokA, e.worldID)
	if w := worldList(t, e, tokA)[0]; w["joined"] != true || w["players"] != float64(1) {
		t.Fatalf("after joining: %v", w)
	}

	tokB, _ := e.register("b@test.io", "Bob")
	if w := worldList(t, e, tokB)[0]; w["admin"] != false || w["joined"] != false || w["players"] != float64(1) {
		t.Fatalf("for the second player: %v", w)
	}
}

func TestJoinAndConnectRules(t *testing.T) {
	e := newEnv(t)
	token, _ := e.register("a@test.io", "Alice")

	if status, _ := e.request("POST", "/worlds/nope/join", token, nil); status != http.StatusNotFound {
		t.Fatalf("joining an unknown world: status %d", status)
	}
	if status, _ := e.request("POST", "/worlds/"+e.worldID+"/join", "", nil); status != http.StatusUnauthorized {
		t.Fatalf("joining without a session: status %d", status)
	}

	// Connecting to a world requires having joined it.
	c := e.dial(token)
	select {
	case _, open := <-c.msgs:
		if open {
			t.Fatal("a player who has not joined must not receive the world")
		}
	case <-time.After(3 * time.Second):
		t.Fatal("the connection should have been closed")
	}
	// Nor is an unknown world accepted.
	c = e.dialWorld(token, "00000000-0000-0000-0000-000000000000")
	select {
	case _, open := <-c.msgs:
		if open {
			t.Fatal("an unknown world must not be served")
		}
	case <-time.After(3 * time.Second):
		t.Fatal("the connection should have been closed")
	}

	e.join(token, e.worldID)
	e.dial(token).expect("snapshot after joining", ofType(protocol.TypeSnapshot))
}

func TestWorldsAreIndependent(t *testing.T) {
	e := newEnv(t)
	token, id := e.register("a@test.io", "Alice")

	if status, _ := e.request("POST", "/worlds", "", map[string]string{"name": "Nuovo"}); status != http.StatusUnauthorized {
		t.Fatalf("creating a world without a session: status %d", status)
	}
	if status, _ := e.request("POST", "/worlds", token, map[string]string{"name": "x"}); status != http.StatusBadRequest {
		t.Fatalf("a one-letter world name: status %d", status)
	}
	status, out := e.request("POST", "/worlds", token, map[string]string{"name": "Mondo nuovo", "description": "Vuoto"})
	if status != http.StatusCreated {
		t.Fatalf("creating a world: status %d %v", status, out)
	}
	newID := out["id"].(string)

	var created map[string]any
	for _, w := range worldList(t, e, token) {
		if w["id"] == newID {
			created = w
		}
	}
	if created == nil || created["name"] != "Mondo nuovo" || created["boards"] != float64(1) || created["admin"] != true || created["joined"] != false {
		t.Fatalf("the new world in the lobby: %v", created)
	}

	// Alice plays in both worlds, with a separate team in each.
	e.join(token, e.worldID)
	e.join(token, newID)
	fresh := e.dialWorld(token, newID).expect("snapshot of the new world", ofType(protocol.TypeSnapshot))
	if len(fresh.Boards) != 1 || fresh.Boards[0].Name != "Piazza" || fresh.YourPlayerID != id {
		t.Fatalf("new world snapshot: %+v", fresh.Boards)
	}
	team := 1 + game.DefaultRules().MinorsPerTeam
	if len(fresh.Entities) != team {
		t.Fatalf("the new world has %d entities in view, want just Alice's team of %d", len(fresh.Entities), team)
	}
	seeded := e.dial(token).expect("snapshot of the seeded world", ofType(protocol.TypeSnapshot))
	if len(seeded.Boards) != 3 {
		t.Fatalf("seeded world boards = %d, want 3", len(seeded.Boards))
	}
	for _, en := range fresh.Entities {
		for _, other := range seeded.Entities {
			if en.ID == other.ID {
				t.Fatalf("entity %s appears in both worlds", en.ID)
			}
		}
	}

	// Each world keeps its own points.
	var points int
	if err := e.db.QueryRow(e.ctx, `SELECT count(*) FROM memberships WHERE player_id::text = $1`, id).Scan(&points); err != nil || points != 2 {
		t.Fatalf("memberships = %d (%v), want 2", points, err)
	}
}
