package transport

import (
	"testing"
	"time"

	"thegame/backend/internal/game"
	"thegame/backend/internal/protocol"
)

func TestGoalsOverTheWireAndInTheDatabase(t *testing.T) {
	e := newEnv(t,
		`DELETE FROM goals`,
		`INSERT INTO goals (world_id, scope, kind, target, reward, title, position) SELECT id, 'individual', 'talks', 1, 10, 'Chiacchierone', 0 FROM worlds`,
		`INSERT INTO goals (world_id, scope, kind, target, reward, title, position) SELECT id, 'world', 'pickups', 1, 100, 'Raccolta', 1 FROM worlds`,
	)
	tokA, idA := e.player("a@test.io", "Alice")
	tokB, idB := e.player("b@test.io", "Bob")
	a := e.dial(tokA)
	snap := a.expect("snapshot", ofType(protocol.TypeSnapshot))
	b := e.dial(tokB)
	bobSnap := b.expect("bob snapshot", ofType(protocol.TypeSnapshot))

	goalTitles := func(goals []protocol.Goal) map[string]protocol.Goal {
		out := map[string]protocol.Goal{}
		for _, g := range goals {
			out[g.Title] = g
		}
		return out
	}
	if g := goalTitles(snap.Goals); len(g) != 2 || g["Chiacchierone"].Scope != "individual" || g["Raccolta"].Scope != "world" || g["Chiacchierone"].Target != 1 {
		t.Fatalf("alice's goals = %+v", snap.Goals)
	}
	if len(bobSnap.Goals) != 2 {
		t.Fatalf("bob's goals = %+v", bobSnap.Goals)
	}
	// The assignments are written by the loop's queue, a moment after the snapshots go out.
	var assigned int
	for deadline := time.Now().Add(2 * time.Second); time.Now().Before(deadline); time.Sleep(20 * time.Millisecond) {
		e.db.QueryRow(e.ctx, `SELECT count(*) FROM player_goals`).Scan(&assigned)
		if assigned == 2 {
			break
		}
	}
	if assigned != 2 {
		t.Fatalf("individual goals assigned in the database = %d, want one per player", assigned)
	}

	champ, _ := findEntity(snap.Entities, func(en protocol.Entity) bool { return en.Kind == "champion" && en.OwnerID == idA })
	npc, _ := findEntity(snap.Entities, func(en protocol.Entity) bool { return en.Name == "Test NPC" })
	item, _ := findEntity(snap.Entities, func(en protocol.Entity) bool { return en.Name == "Test Item" })

	// Talking to an NPC completes Alice's individual goal.
	a.send(protocol.ClientMessage{Type: protocol.TypeTalk, UnitID: champ.ID, TargetID: npc.ID})
	a.expect("goal reached", func(m protocol.ServerMessage) bool {
		return m.Type == protocol.TypeEvent && m.Title == "Obiettivo raggiunto"
	})
	done := a.expect("goals update", func(m protocol.ServerMessage) bool {
		return m.Type == protocol.TypeGoals && goalTitles(m.Goals)["Chiacchierone"].Completed
	})
	if g := goalTitles(done.Goals)["Chiacchierone"]; g.Progress != 1 || g.Reward != 10 {
		t.Fatalf("goal after completion = %+v", g)
	}

	// Picking up the item wins the world goal: Bob is told, and sees who won.
	a.send(protocol.ClientMessage{Type: protocol.TypePickup, UnitID: champ.ID, TargetID: item.ID})
	b.expect("victory announcement", func(m protocol.ServerMessage) bool { return m.Type == protocol.TypeEvent && m.Title == "Vittoria" })
	won := b.expect("goals with the winner", func(m protocol.ServerMessage) bool {
		return m.Type == protocol.TypeGoals && goalTitles(m.Goals)["Raccolta"].AchievedBy == "Alice"
	})
	if g := goalTitles(won.Goals)["Raccolta"]; !g.Completed {
		t.Fatalf("world goal for Bob = %+v", g)
	}

	// Everything is saved.
	rules := game.DefaultRules()
	wantPoints := 10 + rules.Points.Pickup + 100
	deadline := time.Now().Add(3 * time.Second)
	for {
		var completed, won, talked, pickups, points int
		e.db.QueryRow(e.ctx, `SELECT count(*) FROM player_goals WHERE player_id::text = $1 AND completed_at IS NOT NULL`, idA).Scan(&completed)
		e.db.QueryRow(e.ctx, `SELECT count(*) FROM goals WHERE title = 'Raccolta' AND achieved_by::text = $1`, idA).Scan(&won)
		if err := e.db.QueryRow(e.ctx, `SELECT COALESCE((stats->>'pickups')::int, 0), points FROM memberships WHERE player_id::text = $1`, idA).Scan(&pickups, &points); err != nil {
			t.Fatal(err)
		}
		if err := e.db.QueryRow(e.ctx, `SELECT count(*) FROM memberships m, jsonb_object_keys(COALESCE(m.stats->'talked', '{}'::jsonb)) WHERE m.player_id::text = $1`, idA).Scan(&talked); err != nil {
			t.Fatal(err)
		}
		if completed == 1 && won == 1 && talked == 1 && pickups == 1 && points == wantPoints {
			break
		}
		if time.Now().After(deadline) {
			t.Fatalf("database: completed=%d won=%d talked=%d pickups=%d points=%d (want points %d)", completed, won, talked, pickups, points, wantPoints)
		}
		time.Sleep(50 * time.Millisecond)
	}

	// A "restarted" server restores the goals and counters.
	world, err := e.store.LoadWorld(e.ctx, e.worldID)
	if err != nil {
		t.Fatal(err)
	}
	alice := world.Player(idA)
	if alice == nil || len(alice.CompletedGoals) != 1 || alice.Stats.Pickups != 1 || len(alice.Stats.Talked) != 1 || alice.GoalID != "" {
		t.Fatalf("alice after reload = %+v", alice)
	}
	for _, g := range world.Goals() {
		if g.Title == "Raccolta" && g.AchievedBy != idA {
			t.Fatalf("the world goal should still be won by Alice, got %q", g.AchievedBy)
		}
	}
	if bob := world.Player(idB); bob == nil || bob.GoalID == "" {
		t.Fatalf("bob keeps his assigned goal after the reload: %+v", bob)
	}
}
