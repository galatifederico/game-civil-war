package transport

import (
	"net/http"
	"strings"
	"testing"

	"thegame/backend/internal/protocol"
	"thegame/backend/internal/store"
)

func terrainOf(d store.WorldDefinition, boardID string) []string {
	for _, t := range d.Terrain {
		if t.BoardID == boardID {
			return append([]string(nil), t.Rows...)
		}
	}
	return nil
}

func TestTerrainSeedEditingAndBlocking(t *testing.T) {
	e := newEnv(t)
	owner, _ := e.register("owner@test.io", "Owner")
	tok, pid := e.player("p@test.io", "Player")
	d := e.definition(owner)

	var piazza store.AdminBoard
	for _, b := range d.Boards {
		if b.Name == "Piazza" {
			piazza = b
		}
	}
	rows := terrainOf(d, piazza.ID)
	if len(rows) != piazza.Height || len(rows[0]) != piazza.Width {
		t.Fatalf("terrain rows are %dx%d for a %dx%d board", len(rows[0]), len(rows), piazza.Width, piazza.Height)
	}
	// The seeded worlds get a line of trees along the border, except where a passage starts.
	if !strings.HasPrefix(rows[0], "TTT") || rows[10][23] == 'T' || rows[12][23] == 'T' {
		t.Fatalf("seeded border: top row %q, cells at the gateways %q %q", rows[0], string(rows[10][23]), string(rows[12][23]))
	}
	if len(d.Boards) != len(d.Terrain) {
		t.Fatalf("%d boards but terrain for %d", len(d.Boards), len(d.Terrain))
	}

	// The client gets it in the snapshot.
	c := e.dial(tok)
	snap := c.expect("snapshot", ofType(protocol.TypeSnapshot))
	var got []string
	for _, b := range snap.Boards {
		if b.ID == piazza.ID {
			got = b.Terrain
		}
	}
	if len(got) != piazza.Height || got[0] != rows[0] {
		t.Fatalf("snapshot terrain = %q", got)
	}

	// Editing: a made-up glyph, a wrong size and a tree on top of an NPC are all refused.
	bad := func(edit func([]string)) []store.AdminTerrain {
		r := terrainOf(d, piazza.ID)
		edit(r)
		return []store.AdminTerrain{{BoardID: piazza.ID, Rows: r}}
	}
	e.save(owner, "terrain", bad(func(r []string) { r[5] = "?" + r[5][1:] }), http.StatusBadRequest)
	e.save(owner, "terrain", bad(func(r []string) { r[5] = r[5][1:] }), http.StatusBadRequest)
	npc := d.NPCs[0]
	if npc.BoardID != piazza.ID {
		t.Skip("the first NPC is not in the Piazza")
	}
	msg := e.save(owner, "terrain", bad(func(r []string) { r[npc.Y] = r[npc.Y][:npc.X] + "T" + r[npc.Y][npc.X+1:] }), http.StatusBadRequest)
	if !strings.Contains(msg, "occupata") {
		t.Fatalf("a tree on an NPC: %q", msg)
	}
	e.save(owner, "terrain", []store.AdminTerrain{{BoardID: "nope", Rows: []string{"."}}}, http.StatusBadRequest)

	// Put a tree next to the player's champion: walking onto it is refused, a path is fine.
	snap = e.dial(tok).expect("snapshot", ofType(protocol.TypeSnapshot))
	champ, _ := findEntity(snap.Entities, func(en protocol.Entity) bool { return en.Kind == "champion" && en.OwnerID == pid })
	occupied := map[[2]int]bool{}
	for _, en := range snap.Entities {
		if en.BoardID == champ.BoardID {
			occupied[[2]int{en.X, en.Y}] = true
		}
	}
	var cell [2]int
	for dy := -2; dy <= 2 && cell == [2]int{}; dy++ {
		for dx := -2; dx <= 2; dx++ {
			if x, y := champ.X+dx, champ.Y+dy; x > 0 && y > 0 && x < piazza.Width-1 && y < piazza.Height-1 && !occupied[[2]int{x, y}] {
				cell = [2]int{x, y}
				break
			}
		}
	}
	e.save(owner, "terrain", bad(func(r []string) { r[cell[1]] = r[cell[1]][:cell[0]] + "T" + r[cell[1]][cell[0]+1:] }), http.StatusOK)
	if got := terrainOf(e.definition(owner), piazza.ID)[cell[1]][cell[0]]; got != 'T' {
		t.Fatalf("the tree was not saved: %q", string(got))
	}

	c2 := e.dial(tok)
	c2.expect("snapshot", ofType(protocol.TypeSnapshot))
	c2.send(protocol.ClientMessage{Type: protocol.TypeMove, UnitID: champ.ID, X: cell[0], Y: cell[1]})
	if m := c2.expect("blocked terrain error", ofType(protocol.TypeError)); m.Code != "blocked_terrain" {
		t.Fatalf("walking onto a tree: code %q", m.Code)
	}
}
