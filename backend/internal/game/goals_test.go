package game

import (
	"testing"
	"time"

	"thegame/backend/internal/protocol"
)

func individual(id string, kind GoalKind, target, reward int) *Goal {
	return &Goal{ID: id, Scope: GoalIndividual, Kind: kind, Target: target, Reward: reward, Title: "Goal " + id}
}

func worldGoal(id string, kind GoalKind, target, reward int) *Goal {
	return &Goal{ID: id, Scope: GoalWorld, Kind: kind, Target: target, Reward: reward, Title: "Victory " + id}
}

func TestGoalsAreMeasuredByTheirKind(t *testing.T) {
	w := actionBoard()
	w.entities["enemy"].Health = 1
	p := w.Player("p1")

	killOut, err := w.Do("p1", attack("champ", "enemy"), t0)
	if err != nil {
		t.Fatal(err)
	}
	if !killOut.StatsChanged {
		t.Fatal("a kill changes the counters, which must be saved")
	}
	if p.Stats.Kills != 1 || p.Stats.ChampionKills != 0 {
		t.Fatalf("stats after killing a minor = %+v", p.Stats)
	}
	w.entities["enemy"].Health = 0
	w.entities["far"].Kind = KindChampion
	w.entities["far"].X, w.entities["far"].Y = 8, 5
	w.entities["far"].Health = 1
	if _, err := w.Do("p1", attack("champ", "far"), t0.Add(time.Minute)); err != nil {
		t.Fatal(err)
	}
	if p.Stats.Kills != 2 || p.Stats.ChampionKills != 1 {
		t.Fatalf("stats after killing a champion = %+v", p.Stats)
	}

	talk := func(target string, at time.Time) {
		t.Helper()
		if _, err := w.Do("p1", Action{Kind: ActionTalk, UnitID: "champ", TargetID: target}, at); err != nil {
			t.Fatal(err)
		}
	}
	talk("npc", t0.Add(2*time.Minute))
	talk("npc", t0.Add(2*time.Minute+time.Second))
	if len(p.Stats.Talked) != 1 {
		t.Fatalf("talking to the same NPC twice counts once, got %d", len(p.Stats.Talked))
	}

	out, err := w.Do("p1", Action{Kind: ActionPickup, UnitID: "champ", TargetID: "item"}, t0.Add(time.Hour))
	if err != nil {
		t.Fatal(err)
	}
	if p.Stats.Pickups != 1 || !out.StatsChanged {
		t.Fatalf("pickups = %d, stats saved: %v", p.Stats.Pickups, out.StatsChanged)
	}

	check := func(kind GoalKind, want int) {
		t.Helper()
		if got := w.progress(p, &Goal{Kind: kind}); got != want {
			t.Errorf("progress(%s) = %d, want %d", kind, got, want)
		}
	}
	check(GoalKills, 2)
	check(GoalChampionKills, 1)
	check(GoalTalks, 1)
	check(GoalPickups, 1)
	check(GoalPoints, p.Points)
	check(GoalUnits, 2) // the champion and the minor
	check(GoalStructures, 0)
	if _, err := w.Do("p1", Action{Kind: ActionBuild, UnitID: "champ", At: Point{5, 4}}, t0.Add(2*time.Hour)); err != nil {
		t.Fatal(err)
	}
	check(GoalStructures, 1)
}

func TestCompletingAnIndividualGoalRewardsOnceAndMovesOn(t *testing.T) {
	w := actionBoard()
	w.AddGoal(individual("chat", GoalTalks, 1, 20))
	w.AddGoal(individual("loot", GoalPickups, 1, 30))
	p := w.Player("p1")
	if err := w.AssignGoal("p1", "chat"); err != nil {
		t.Fatal(err)
	}

	out, err := w.Do("p1", Action{Kind: ActionTalk, UnitID: "champ", TargetID: "npc"}, t0)
	if err != nil {
		t.Fatal(err)
	}
	if !p.CompletedGoals["chat"] || out.Points != 20 || p.Points != 20 {
		t.Fatalf("after completing: completed=%v out.Points=%d p.Points=%d", p.CompletedGoals, out.Points, p.Points)
	}
	if len(out.CompletedGoals) != 1 || out.CompletedGoals[0].GoalID != "chat" || !out.GoalsChanged || !out.StatsChanged {
		t.Fatalf("outcome = %+v", out)
	}
	if p.GoalID != "loot" || len(out.AssignedGoals) != 1 || out.AssignedGoals[0].GoalID != "loot" {
		t.Fatalf("with random assignment the next goal follows: %q, assigned %+v", p.GoalID, out.AssignedGoals)
	}
	var titles []string
	for _, n := range out.Notices {
		titles = append(titles, n.Title)
	}
	if len(titles) < 3 { // the dialogue, the completion, the new goal
		t.Fatalf("notices = %v", titles)
	}

	// Talking again does not pay again.
	before := p.Points
	if _, err := w.Do("p1", Action{Kind: ActionTalk, UnitID: "champ", TargetID: "npc"}, t0.Add(time.Second)); err != nil {
		t.Fatal(err)
	}
	if p.Points != before {
		t.Fatalf("a completed goal paid again: %d -> %d", before, p.Points)
	}
}

func TestManualAssignmentGivesNoNextGoal(t *testing.T) {
	w := actionBoard()
	w.Rules.GoalAssignment = "manual"
	w.AddGoal(individual("chat", GoalTalks, 1, 20))
	w.AddGoal(individual("loot", GoalPickups, 1, 30))
	if ref := w.EnsureGoal("p1"); ref != nil {
		t.Fatalf("manual worlds must not assign by themselves: %+v", ref)
	}
	if err := w.AssignGoal("p1", "chat"); err != nil {
		t.Fatal(err)
	}
	out, err := w.Do("p1", Action{Kind: ActionTalk, UnitID: "champ", TargetID: "npc"}, t0)
	if err != nil {
		t.Fatal(err)
	}
	if w.Player("p1").GoalID != "" || len(out.AssignedGoals) != 0 {
		t.Fatal("after a completed goal the player waits for the admin")
	}
}

func TestEnsureGoalAssignsAtRandomOnce(t *testing.T) {
	w := actionBoard()
	if ref := w.EnsureGoal("p1"); ref != nil {
		t.Fatalf("no goals defined: %+v", ref)
	}
	w.AddGoal(individual("a", GoalTalks, 1, 1))
	w.AddGoal(worldGoal("w", GoalPoints, 99, 1))
	ref := w.EnsureGoal("p1")
	if ref == nil || ref.GoalID != "a" {
		t.Fatalf("assignment = %+v, want the only individual goal", ref)
	}
	if again := w.EnsureGoal("p1"); again != nil {
		t.Fatal("a player who already has a goal must not get another")
	}
	if ref := w.EnsureGoal("nobody"); ref != nil {
		t.Fatal("unknown player")
	}
}

func TestAssignGoalRules(t *testing.T) {
	w := actionBoard()
	w.AddGoal(individual("a", GoalTalks, 1, 1))
	w.AddGoal(worldGoal("w", GoalPoints, 99, 1))
	w.Player("p1").CompletedGoals["a"] = true

	if err := w.AssignGoal("nobody", "a"); err != ErrNotFound {
		t.Errorf("unknown player: %v", err)
	}
	if err := w.AssignGoal("p1", "nope"); err != ErrInvalidTarget {
		t.Errorf("unknown goal: %v", err)
	}
	if err := w.AssignGoal("p1", "w"); err != ErrInvalidTarget {
		t.Errorf("a world goal cannot be assigned: %v", err)
	}
	if err := w.AssignGoal("p1", "a"); err != ErrGoalDone {
		t.Errorf("a completed goal: %v", err)
	}
}

func TestOnlyTheFirstPlayerWinsAWorldGoal(t *testing.T) {
	w := actionBoard()
	w.AddGoal(worldGoal("rich", GoalPickups, 1, 100))
	w.Add(&Entity{ID: "item2", BoardID: "main", Kind: KindItem, Name: "Altro", X: 8, Y: 6})
	w.entities["enemy"].Vision = 5

	out, err := w.Do("p1", Action{Kind: ActionPickup, UnitID: "champ", TargetID: "item"}, t0)
	if err != nil {
		t.Fatal(err)
	}
	g := w.goals["rich"]
	if g.AchievedBy != "p1" || w.Player("p1").Points != 100+rules.Points.Pickup {
		t.Fatalf("winner = %q, points = %d", g.AchievedBy, w.Player("p1").Points)
	}
	if len(out.Victories) != 1 || len(out.Broadcast) != 1 || out.Broadcast[0].PlayerID != "" {
		t.Fatalf("a victory is announced to everyone: %+v", out)
	}

	if _, err := w.Do("p2", Action{Kind: ActionPickup, UnitID: "enemy", TargetID: "item2"}, t0); err != nil {
		t.Fatal(err)
	}
	if g.AchievedBy != "p1" || w.Player("p2").Points != rules.Points.Pickup {
		t.Fatalf("the second player must not win it too: winner %q, p2 points %d", g.AchievedBy, w.Player("p2").Points)
	}
}

func TestRewardsCanCompleteOtherGoals(t *testing.T) {
	w := actionBoard()
	w.AddGoal(individual("chat", GoalTalks, 1, 50))
	w.AddGoal(worldGoal("ten", GoalPoints, 40, 5))
	if err := w.AssignGoal("p1", "chat"); err != nil {
		t.Fatal(err)
	}
	if _, err := w.Do("p1", Action{Kind: ActionTalk, UnitID: "champ", TargetID: "npc"}, t0); err != nil {
		t.Fatal(err)
	}
	if w.goals["ten"].AchievedBy != "p1" {
		t.Fatal("the 50 points from the first goal should have won the points goal")
	}
	if w.Player("p1").Points != 55 {
		t.Fatalf("points = %d, want 50 + 5", w.Player("p1").Points)
	}
}

func TestGoalViewsShowWhatAPlayerShouldSee(t *testing.T) {
	w := actionBoard()
	w.AddGoal(individual("mine", GoalPickups, 3, 10))
	w.AddGoal(individual("done", GoalTalks, 1, 10))
	w.AddGoal(individual("other", GoalKills, 2, 10))
	w.AddGoal(worldGoal("open", GoalPoints, 1000, 0))
	w.AddGoal(worldGoal("won", GoalUnits, 1, 0))
	w.Player("p1").GoalID = "mine"
	w.Player("p1").CompletedGoals["done"] = true
	w.Player("p2").GoalID = "other"
	w.goals["won"].AchievedBy = "p2"
	w.Player("p1").Stats.Pickups = 7 // beyond the target

	byID := map[string]GoalView{}
	for _, v := range w.GoalViews("p1") {
		byID[v.ID] = v
	}
	if _, seen := byID["other"]; seen {
		t.Fatal("another player's individual goal must be hidden")
	}
	if v := byID["mine"]; v.Progress != 3 || v.Completed || v.Target != 3 {
		t.Fatalf("mine = %+v (progress is capped at the target)", v)
	}
	if v := byID["done"]; !v.Completed {
		t.Fatalf("done = %+v", v)
	}
	if v := byID["open"]; v.Completed || v.AchievedBy != "" {
		t.Fatalf("open = %+v", v)
	}
	if v := byID["won"]; !v.Completed || v.AchievedBy != "Bob" {
		t.Fatalf("won = %+v, want it shown as won by Bob", v)
	}
	if len(w.GoalViews("nobody")) != 0 {
		t.Fatal("unknown player")
	}
}

func TestGoalKinds(t *testing.T) {
	for _, k := range []GoalKind{GoalPoints, GoalKills, GoalChampionKills, GoalPickups, GoalStructures, GoalUnits, GoalTalks} {
		if !k.Valid() {
			t.Errorf("%s should be valid", k)
		}
	}
	if GoalKind("magic").Valid() {
		t.Error("an unknown kind must not be valid")
	}
}

func TestLoopSendsGoalsAndAnnouncesVictories(t *testing.T) {
	loop, _, p1, p2 := fogWorldWith(t, func(w *World) {
		w.AddGoal(individual("chat", GoalTalks, 1, 20))
		w.AddGoal(worldGoal("loot", GoalPickups, 1, 100))
	})
	loop.Register(p1)
	snap := recv(t, p1)
	if len(snap.Goals) != 2 {
		t.Fatalf("snapshot goals = %+v, want the world goal and the individual one assigned at random", snap.Goals)
	}
	loop.Register(p2)
	if s := recv(t, p2); len(s.Goals) != 2 {
		t.Fatalf("p2 snapshot goals = %+v", s.Goals)
	}

	// p1 picks up the item and wins the world goal: everybody hears about it.
	loop.Act(p1, Action{Kind: ActionPickup, UnitID: "champ", TargetID: "item"})
	var announced, goalsUpdate bool
	for i := 0; i < 8; i++ {
		select {
		case data := <-p2.Send:
			var m protocol.ServerMessage
			mustUnmarshal(t, data, &m)
			if m.Type == protocol.TypeEvent && m.Title == "Vittoria" {
				announced = true
			}
			if m.Type == protocol.TypeGoals {
				for _, g := range m.Goals {
					if g.ID == "loot" && g.Completed && g.AchievedBy == "Anna" {
						goalsUpdate = true
					}
				}
			}
		case <-time.After(200 * time.Millisecond):
		}
	}
	if !announced || !goalsUpdate {
		t.Fatalf("p2 should hear the victory (event %v) and see the goal won by Anna (goals %v)", announced, goalsUpdate)
	}
}
