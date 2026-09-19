package game

import (
	"errors"
	"testing"
	"time"

	"thegame/backend/internal/protocol"
)

func TestAwayPlayersAreShielded(t *testing.T) {
	w := actionBoard()
	w.Rules.AFK.AfterMs = 60_000
	w.Rules.AFK.Shield = true
	w.StartActivity(t0)

	if w.UpdateAFK(t0.Add(59 * time.Second)) {
		t.Fatal("marked afk too early")
	}
	// Bob does nothing; Anna keeps playing.
	w.Touch("p1", t0.Add(50*time.Second))
	if !w.UpdateAFK(t0.Add(61 * time.Second)) {
		t.Fatal("Bob should be afk after a minute of nothing")
	}
	if !w.Player("p2").AFK || w.Player("p1").AFK {
		t.Fatalf("afk: bob=%v anna=%v", w.Player("p2").AFK, w.Player("p1").AFK)
	}
	if w.UpdateAFK(t0.Add(62 * time.Second)) {
		t.Fatal("nothing changed, nothing should be reported")
	}

	if _, err := w.Do("p1", attack("champ", "enemy"), t0.Add(62*time.Second)); !errors.Is(err, ErrTargetAFK) {
		t.Fatalf("attacking an away team: %v, want ErrTargetAFK", err)
	}
	if got := w.entities["enemy"].Health; got != 100 {
		t.Fatalf("the shielded unit lost health: %d", got)
	}
	var afk []string
	for _, s := range w.Scores() {
		if s.AFK {
			afk = append(afk, s.Username)
		}
	}
	if len(afk) != 1 || afk[0] != "Bob" {
		t.Fatalf("scoreboard afk = %v", afk)
	}

	// Coming back removes the shield at once.
	if !w.Touch("p2", t0.Add(63*time.Second)) {
		t.Fatal("touching an afk player should report that they are back")
	}
	if _, err := w.Do("p1", attack("champ", "enemy"), t0.Add(63*time.Second)); err != nil {
		t.Fatalf("attacking a returned player: %v", err)
	}
}

func TestAwayWithoutShieldOrWhenDisabled(t *testing.T) {
	w := actionBoard()
	w.Rules.AFK.AfterMs = 60_000
	w.Rules.AFK.Shield = false
	w.StartActivity(t0)
	w.UpdateAFK(t0.Add(2 * time.Minute))
	if !w.Player("p2").AFK {
		t.Fatal("bob should be afk")
	}
	if _, err := w.Do("p1", attack("champ", "enemy"), t0.Add(2*time.Minute)); err != nil {
		t.Fatalf("without the shield an away team can be attacked: %v", err)
	}

	w = actionBoard()
	w.Rules.AFK.AfterMs = 0
	w.StartActivity(t0)
	if w.UpdateAFK(t0.Add(100 * time.Hour)) {
		t.Fatal("afk is off with after_ms = 0")
	}
}

func TestAFKRulesValidateAndDefault(t *testing.T) {
	r := DefaultRules()
	if r.AFK.AfterMs != 600_000 || !r.AFK.Shield {
		t.Fatalf("defaults = %+v", r.AFK)
	}
	r.AFK.AfterMs = -1
	if r.Validate() == nil {
		t.Fatal("a negative afk.after_ms must be rejected")
	}
}

func TestScoreboardShowsAFKOverTheWire(t *testing.T) {
	loop, advance, p1, _ := fogWorldWith(t, func(w *World) { w.Rules.AFK.AfterMs = 60_000 })
	loop.Register(p1)
	recv(t, p1) // snapshot

	advance(2 * time.Minute)
	m := recv(t, p1)
	if m.Type != "delta" || len(m.Scores) == 0 {
		t.Fatalf("expected the scoreboard to be re-sent, got %+v", m)
	}
	var flagged []protocol.Score
	for _, s := range m.Scores {
		if s.AFK {
			flagged = append(flagged, s)
		}
	}
	if len(flagged) != 2 { // nobody did anything for two minutes, connected or not
		t.Fatalf("afk flags = %+v", m.Scores)
	}

	// A command from Anna brings her back and tells everybody.
	loop.Move(p1, "champ", Point{X: 2, Y: 3})
	back := recv(t, p1)
	for _, s := range back.Scores {
		if s.PlayerID == "p1" && s.AFK {
			t.Fatalf("anna is still afk after playing: %+v", back.Scores)
		}
	}
}
