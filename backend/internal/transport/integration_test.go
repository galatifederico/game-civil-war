package transport

import (
	"bytes"
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"net/url"
	"os"
	"strings"
	"testing"
	"time"

	"github.com/coder/websocket"
	"github.com/jackc/pgx/v5"

	"thegame/backend/internal/auth"
	"thegame/backend/internal/game"
	"thegame/backend/internal/protocol"
	"thegame/backend/internal/store"
)

// These tests need a real Postgres: set TEST_DB_ADMIN_URL to a connection string that can create
// databases (see `make test-integration`). Each test run works on its own throwaway database.

type env struct {
	t      *testing.T
	ctx    context.Context
	dsn    string
	db     *pgx.Conn
	store  *store.Store
	server *httptest.Server
}

func newEnv(t *testing.T) *env {
	adminURL := os.Getenv("TEST_DB_ADMIN_URL")
	if adminURL == "" {
		t.Skip("TEST_DB_ADMIN_URL not set")
	}
	ctx, cancel := context.WithCancel(context.Background())
	t.Cleanup(cancel)

	admin, err := pgx.Connect(ctx, adminURL)
	if err != nil {
		t.Fatal(err)
	}
	var suffix [4]byte
	rand.Read(suffix[:])
	name := "the_game_test_" + hex.EncodeToString(suffix[:])
	if _, err := admin.Exec(ctx, "CREATE DATABASE "+name); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		admin.Exec(context.Background(), "DROP DATABASE "+name+" WITH (FORCE)")
		admin.Close(context.Background())
	})

	u, err := url.Parse(adminURL)
	if err != nil {
		t.Fatal(err)
	}
	u.Path = "/" + name
	e := &env{t: t, ctx: ctx, dsn: u.String()}

	e.store, err = store.Connect(ctx, e.dsn)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(e.store.Close)
	if err := e.store.Migrate(ctx); err != nil {
		t.Fatal(err)
	}

	// A test NPC and item close to where the first team spawns (its champion starts near (4,3),
	// with a vision of 5), so the actions can be tried without walking across the board.
	e.db, err = pgx.Connect(ctx, e.dsn)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { e.db.Close(context.Background()) })
	for _, q := range []string{
		`INSERT INTO units (board_id, kind, name, x, y, speed, dialogue) SELECT id, 'npc', 'Test NPC', 3, 6, 0, 'ciao' FROM boards`,
		`INSERT INTO board_items (board_id, name, x, y) SELECT id, 'Test Item', 4, 6 FROM boards`,
	} {
		if _, err := e.db.Exec(ctx, q); err != nil {
			t.Fatal(err)
		}
	}

	e.startServer()
	return e
}

func (e *env) startServer() {
	board, err := e.store.LoadDefaultBoard(e.ctx)
	if err != nil {
		e.t.Fatal(err)
	}
	loop := game.NewLoop(board, e.store)
	go loop.Run(e.ctx)
	srv := &Server{Store: e.store, Tokens: auth.NewTokens("test-secret-test-secret", time.Hour), Loop: loop}
	e.server = httptest.NewServer(srv.Router())
	e.t.Cleanup(e.server.Close)
}

func (e *env) post(path string, body map[string]string) (int, map[string]string) {
	b, _ := json.Marshal(body)
	resp, err := http.Post(e.server.URL+path, "application/json", bytes.NewReader(b))
	if err != nil {
		e.t.Fatal(err)
	}
	defer resp.Body.Close()
	out := map[string]string{}
	json.NewDecoder(resp.Body).Decode(&out)
	return resp.StatusCode, out
}

func (e *env) register(email, username string) (token, id string) {
	status, out := e.post("/auth/register", map[string]string{"email": email, "username": username, "password": "password123"})
	if status != http.StatusCreated {
		e.t.Fatalf("register %s: status %d %v", email, status, out)
	}
	return out["token"], out["player_id"]
}

type wsClient struct {
	t    *testing.T
	conn *websocket.Conn
	msgs chan protocol.ServerMessage
}

func (e *env) dial(token string) *wsClient {
	wsURL := "ws" + strings.TrimPrefix(e.server.URL, "http") + "/ws"
	conn, _, err := websocket.Dial(e.ctx, wsURL, nil)
	if err != nil {
		e.t.Fatal(err)
	}
	conn.SetReadLimit(1 << 20)
	c := &wsClient{t: e.t, conn: conn, msgs: make(chan protocol.ServerMessage, 256)}
	go func() {
		defer close(c.msgs)
		for {
			_, data, err := conn.Read(e.ctx)
			if err != nil {
				return
			}
			var m protocol.ServerMessage
			json.Unmarshal(data, &m)
			c.msgs <- m
		}
	}()
	e.t.Cleanup(func() { conn.CloseNow() })
	c.send(protocol.ClientMessage{Type: protocol.TypeAuth, Token: token})
	return c
}

func (c *wsClient) send(m protocol.ClientMessage) {
	b, _ := json.Marshal(m)
	if err := c.conn.Write(context.Background(), websocket.MessageText, b); err != nil {
		c.t.Fatalf("send: %v", err)
	}
}

// expect returns the first message that satisfies pred, skipping the others.
func (c *wsClient) expect(what string, pred func(protocol.ServerMessage) bool) protocol.ServerMessage {
	c.t.Helper()
	timeout := time.After(3 * time.Second)
	for {
		select {
		case m, ok := <-c.msgs:
			if !ok {
				c.t.Fatalf("connection closed while waiting for %s", what)
			}
			if pred(m) {
				return m
			}
		case <-timeout:
			c.t.Fatalf("timed out waiting for %s", what)
		}
	}
}

// expectNothingAbout fails if, within the window, the client receives anything about an entity.
func (c *wsClient) expectNothingAbout(id string, window time.Duration) {
	c.t.Helper()
	timeout := time.After(window)
	for {
		select {
		case m, ok := <-c.msgs:
			if !ok {
				return
			}
			if _, found := findEntity(m.Entities, func(en protocol.Entity) bool { return en.ID == id }); found {
				c.t.Fatalf("received an update about %s that is out of sight: %+v", id, m)
			}
			for _, removed := range m.Removed {
				if removed == id {
					c.t.Fatalf("received a removal of %s that was never in sight", id)
				}
			}
		case <-timeout:
			return
		}
	}
}

func ofType(typ string) func(protocol.ServerMessage) bool {
	return func(m protocol.ServerMessage) bool { return m.Type == typ }
}

func findEntity(entities []protocol.Entity, pred func(protocol.Entity) bool) (protocol.Entity, bool) {
	for _, e := range entities {
		if pred(e) {
			return e, true
		}
	}
	return protocol.Entity{}, false
}

func TestAuthEndpoints(t *testing.T) {
	e := newEnv(t)

	if status, _ := e.post("/auth/register", map[string]string{"email": "a@test.io", "username": "Alice", "password": "password123"}); status != http.StatusCreated {
		t.Fatalf("register: %d", status)
	}
	tests := []struct {
		name string
		path string
		body map[string]string
		want int
	}{
		{"duplicate email", "/auth/register", map[string]string{"email": "A@test.io", "username": "Other", "password": "password123"}, http.StatusConflict},
		{"short password", "/auth/register", map[string]string{"email": "b@test.io", "username": "Bob", "password": "short"}, http.StatusBadRequest},
		{"bad email", "/auth/register", map[string]string{"email": "nope", "username": "Bob", "password": "password123"}, http.StatusBadRequest},
		{"short name", "/auth/register", map[string]string{"email": "b@test.io", "username": "B", "password": "password123"}, http.StatusBadRequest},
		{"login ok", "/auth/login", map[string]string{"email": "a@test.io", "password": "password123"}, http.StatusOK},
		{"wrong password", "/auth/login", map[string]string{"email": "a@test.io", "password": "wrongpassword"}, http.StatusUnauthorized},
		{"unknown email", "/auth/login", map[string]string{"email": "nobody@test.io", "password": "password123"}, http.StatusUnauthorized},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			if status, out := e.post(tc.path, tc.body); status != tc.want {
				t.Fatalf("status %d (%v), want %d", status, out, tc.want)
			}
		})
	}

	t.Run("login does not create a second team", func(t *testing.T) {
		var units int
		if err := e.db.QueryRow(e.ctx, `SELECT count(*) FROM units WHERE player_id IS NOT NULL`).Scan(&units); err != nil {
			t.Fatal(err)
		}
		if units != 1+game.MinorsPerTeam {
			t.Fatalf("units owned by players = %d, want one team of %d", units, 1+game.MinorsPerTeam)
		}
	})
}

func TestWebSocketRejectsBadToken(t *testing.T) {
	e := newEnv(t)
	c := e.dial("not-a-token")
	select {
	case _, ok := <-c.msgs:
		if ok {
			t.Fatal("expected the connection to be closed")
		}
	case <-time.After(3 * time.Second):
		t.Fatal("connection stayed open with an invalid token")
	}
}

func TestGameplayAndPersistence(t *testing.T) {
	e := newEnv(t)
	tokA, idA := e.register("a@test.io", "Alice")
	a := e.dial(tokA)
	snap := a.expect("snapshot", ofType(protocol.TypeSnapshot))
	if snap.YourPlayerID != idA || snap.Board == nil || snap.Board.Width == 0 {
		t.Fatalf("bad snapshot: %+v", snap)
	}

	if _, seen := findEntity(snap.Entities, func(en protocol.Entity) bool { return en.Name == "Mercante" }); seen {
		t.Fatal("the Mercante is far from the team: fog of war should hide it")
	}
	mine := 0
	for _, en := range snap.Entities {
		if en.OwnerID == idA {
			mine++
		}
	}
	if mine != 1+game.MinorsPerTeam {
		t.Fatalf("player owns %d units, want %d", mine, 1+game.MinorsPerTeam)
	}

	// A second player joins far away: Alice learns about the new scoreboard entry, but with
	// fog of war she does not see Bob's team, and Bob does not see hers.
	tokB, idB := e.register("b@test.io", "Bob")
	b := e.dial(tokB)
	bobSnap := b.expect("bob snapshot", ofType(protocol.TypeSnapshot))
	joined := a.expect("scoreboard update", func(m protocol.ServerMessage) bool { return m.Type == protocol.TypeDelta && len(m.Scores) == 2 })
	if len(joined.Entities) != 0 {
		t.Fatalf("Alice should not see Bob's team, got %d entities", len(joined.Entities))
	}
	if _, seen := findEntity(bobSnap.Entities, func(en protocol.Entity) bool { return en.OwnerID == idA }); seen {
		t.Fatal("Bob's snapshot should not contain Alice's units")
	}

	champ, _ := findEntity(snap.Entities, func(en protocol.Entity) bool { return en.Kind == "champion" && en.OwnerID == idA })
	bobChamp, _ := findEntity(bobSnap.Entities, func(en protocol.Entity) bool { return en.Kind == "champion" && en.OwnerID == idB })
	npc, _ := findEntity(snap.Entities, func(en protocol.Entity) bool { return en.Name == "Test NPC" })
	item, _ := findEntity(snap.Entities, func(en protocol.Entity) bool { return en.Name == "Test Item" })
	if npc.ID == "" || item.ID == "" {
		t.Fatal("test NPC or item missing from the snapshot")
	}

	// Movement: a free cell one step away, toward the NPC.
	occupied := map[[2]int]bool{}
	for _, en := range snap.Entities {
		occupied[[2]int{en.X, en.Y}] = true
	}
	var to [2]int
	for _, d := range [][2]int{{0, 1}, {1, 1}, {-1, 1}, {1, 0}, {-1, 0}} {
		if c := [2]int{champ.X + d[0], champ.Y + d[1]}; !occupied[c] {
			to = c
			break
		}
	}
	a.send(protocol.ClientMessage{Type: protocol.TypeMove, UnitID: champ.ID, X: to[0], Y: to[1]})
	// The move may also bring new things into view, so look for the champion among the entities.
	movedMsg := a.expect("alice's own move", func(m protocol.ServerMessage) bool {
		_, ok := findEntity(m.Entities, func(en protocol.Entity) bool { return en.ID == champ.ID })
		return m.Type == protocol.TypeDelta && ok
	})
	moved, _ := findEntity(movedMsg.Entities, func(en protocol.Entity) bool { return en.ID == champ.ID })
	if moved.X != to[0] || moved.Y != to[1] || moved.ReadyInMs <= 0 {
		t.Fatalf("moved entity = %+v, want %v with a cooldown", moved, to)
	}

	// Rule violations come back as errors to the sender only.
	a.send(protocol.ClientMessage{Type: protocol.TypeMove, UnitID: champ.ID, X: to[0], Y: to[1] + 1})
	if m := a.expect("cooldown error", ofType(protocol.TypeError)); m.Code != "cooldown" {
		t.Fatalf("code = %q, want cooldown", m.Code)
	}
	a.send(protocol.ClientMessage{Type: protocol.TypeMove, UnitID: bobChamp.ID, X: bobChamp.X, Y: bobChamp.Y + 1})
	if m := a.expect("not_yours error", ofType(protocol.TypeError)); m.Code != "not_yours" {
		t.Fatalf("code = %q, want not_yours", m.Code)
	}
	a.send(protocol.ClientMessage{Type: protocol.TypeAttack, UnitID: champ.ID, TargetID: bobChamp.ID})
	if m := a.expect("out_of_range error", ofType(protocol.TypeError)); m.Code != "out_of_range" {
		t.Fatalf("code = %q, want out_of_range", m.Code)
	}

	// Talk, pick up, build.
	a.send(protocol.ClientMessage{Type: protocol.TypeTalk, UnitID: champ.ID, TargetID: npc.ID})
	if m := a.expect("npc dialogue", ofType(protocol.TypeEvent)); m.Title != "Test NPC" || m.Message != "ciao" {
		t.Fatalf("dialogue = %q / %q", m.Title, m.Message)
	}

	a.send(protocol.ClientMessage{Type: protocol.TypePickup, UnitID: champ.ID, TargetID: item.ID})
	picked := a.expect("item removed", func(m protocol.ServerMessage) bool { return m.Type == protocol.TypeDelta && len(m.Removed) == 1 })
	if picked.Removed[0] != item.ID {
		t.Fatalf("removed %v, want %s", picked.Removed, item.ID)
	}
	if inv := a.expect("inventory", ofType(protocol.TypeInventory)); len(inv.Inventory) != 1 || inv.Inventory[0].Name != "Test Item" {
		t.Fatalf("inventory = %+v", inv.Inventory)
	}
	b.expectNothingAbout(item.ID, 200*time.Millisecond) // Bob never saw it, so he is not told it is gone

	time.Sleep(game.PickupCooldown + 100*time.Millisecond)
	a.send(protocol.ClientMessage{Type: protocol.TypeBuild, UnitID: champ.ID, X: 3, Y: 7})
	built := a.expect("the new structure", func(m protocol.ServerMessage) bool {
		_, ok := findEntity(m.Entities, func(en protocol.Entity) bool { return en.Kind == "structure" })
		return ok
	})
	b.expectNothingAbout(champ.ID, 200*time.Millisecond) // nothing Alice did was ever in Bob's sight
	structure, _ := findEntity(built.Entities, func(en protocol.Entity) bool { return en.Kind == "structure" })
	if structure.X != 3 || structure.Y != 7 || structure.OwnerID != idA {
		t.Fatalf("structure = %+v", structure)
	}
	wantPoints := game.PointsPickup + game.PointsBuild
	if len(built.Scores) == 0 || built.Scores[0].Points != wantPoints {
		t.Fatalf("scores = %+v, want the leader on %d", built.Scores, wantPoints)
	}

	// Everything must have reached the database (writes are queued): poll until it does.
	deadline := time.Now().Add(3 * time.Second)
	for {
		var points, inventory, structures, items int
		e.db.QueryRow(e.ctx, `SELECT points FROM players WHERE id::text = $1`, idA).Scan(&points)
		e.db.QueryRow(e.ctx, `SELECT count(*) FROM inventory_items WHERE player_id::text = $1`, idA).Scan(&inventory)
		e.db.QueryRow(e.ctx, `SELECT count(*) FROM structures`).Scan(&structures)
		e.db.QueryRow(e.ctx, `SELECT count(*) FROM board_items WHERE name = 'Test Item'`).Scan(&items)
		if points == wantPoints && inventory == 1 && structures == 1 && items == 0 {
			break
		}
		if time.Now().After(deadline) {
			t.Fatalf("database not up to date: points=%d inventory=%d structures=%d items=%d", points, inventory, structures, items)
		}
		time.Sleep(50 * time.Millisecond)
	}

	// A "restarted" server loads the same world back.
	board, err := e.store.LoadDefaultBoard(e.ctx)
	if err != nil {
		t.Fatal(err)
	}
	p := board.Player(idA)
	if p == nil || p.Points != wantPoints || len(p.Inventory) != 1 {
		t.Fatalf("reloaded player = %+v", p)
	}
	var reloaded game.Entity
	for _, en := range board.All() {
		if en.ID == champ.ID {
			reloaded = *en
		}
	}
	if reloaded.X != to[0] || reloaded.Y != to[1] {
		t.Fatalf("champion reloaded at %d,%d, want %v", reloaded.X, reloaded.Y, to)
	}
}
