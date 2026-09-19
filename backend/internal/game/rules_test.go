package game

import (
	"strings"
	"testing"
	"time"
)

func TestDefaultRulesAreValid(t *testing.T) {
	if err := DefaultRules().Validate(); err != nil {
		t.Fatal(err)
	}
}

func TestParseRulesOverridesOnlyWhatIsSet(t *testing.T) {
	r, err := ParseRules([]byte(`{"minors_per_team": 3, "minor": {"speed": 7}, "points": {"hit": 9}, "cooldowns_ms": {"attack": 250}}`))
	if err != nil {
		t.Fatal(err)
	}
	def := DefaultRules()
	if r.MinorsPerTeam != 3 || r.Minor.Speed != 7 || r.Points.Hit != 9 || r.AttackCooldown() != 250*time.Millisecond {
		t.Fatalf("overrides not applied: %+v", r)
	}
	if r.Minor.Health != def.Minor.Health || r.Champion != def.Champion || r.Points.KillChampion != def.Points.KillChampion ||
		r.RespawnMs != def.RespawnMs || r.CooldownsMs.Build != def.CooldownsMs.Build {
		t.Fatalf("anything not set must keep its default: %+v", r)
	}
}

func TestParseRulesEmptyMeansDefaults(t *testing.T) {
	for _, raw := range []string{"", "  ", "{}", "null"} {
		r, err := ParseRules([]byte(raw))
		if err != nil {
			t.Fatalf("%q: %v", raw, err)
		}
		if r != DefaultRules() {
			t.Fatalf("%q: not the defaults", raw)
		}
	}
}

func TestParseRulesRejectsMistakes(t *testing.T) {
	tests := []struct {
		name string
		raw  string
		want string
	}{
		{"unknown field", `{"minors_per_tem": 3}`, "unknown field"},
		{"unknown nested field", `{"points": {"hitt": 3}}`, "unknown field"},
		{"wrong type", `{"minors_per_team": "many"}`, "parse rules"},
		{"not json", `{`, "parse rules"},
		{"unit that cannot move", `{"champion": {"speed": 0}}`, "champion.speed"},
		{"unit without health", `{"minor": {"health": 0}}`, "minor.health"},
		{"blind unit", `{"minor": {"vision": 0}}`, "minor.vision"},
		{"negative cooldown", `{"cooldowns_ms": {"build": -1}}`, "cooldowns_ms.build"},
		{"negative team size", `{"minors_per_team": -1}`, "minors_per_team"},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			_, err := ParseRules([]byte(tc.raw))
			if err == nil || !strings.Contains(err.Error(), tc.want) {
				t.Fatalf("got %v, want an error mentioning %q", err, tc.want)
			}
		})
	}
}

func TestPointsMayBeNegative(t *testing.T) {
	if _, err := ParseRules([]byte(`{"points": {"hit": -3}}`)); err != nil {
		t.Fatalf("design.md allows actions that cost points: %v", err)
	}
}

func TestRulesChangeTheBehaviorOfTheBoard(t *testing.T) {
	t.Run("team size and stats", func(t *testing.T) {
		b := newTestWorld("b", "test", 24, 24, nil)
		b.Rules.MinorsPerTeam = 3
		b.Rules.Minor = UnitStats{Speed: 6, Health: 40, Vision: 2, Strength: 9}
		b.Rules.Champion.Vision = 8
		team, err := b.PlanTeam("p1", "Anna")
		if err != nil {
			t.Fatal(err)
		}
		if len(team) != 4 {
			t.Fatalf("team size = %d, want 1 champion + 3 minors", len(team))
		}
		if team[0].Vision != 8 || team[0].Health != rules.Champion.Health {
			t.Fatalf("champion = %+v", team[0])
		}
		if m := team[1]; m.Speed != 6 || m.Health != 40 || m.MaxHealth != 40 || m.Vision != 2 || m.Strength != 9 {
			t.Fatalf("minor = %+v", m)
		}
	})

	t.Run("points, cooldown and respawn", func(t *testing.T) {
		b := actionBoard()
		b.Rules.Points.Hit = 7
		b.Rules.CooldownsMs.Attack = 100
		b.Rules.RespawnMs = 2000
		b.entities["enemy"].Health = 1

		out, err := b.Do("p1", attack("champ", "enemy"), t0)
		if err != nil {
			t.Fatal(err)
		}
		if want := 7 + rules.Points.KillMinor; out.Points != want {
			t.Fatalf("points = %d, want %d", out.Points, want)
		}
		if got := b.entities["champ"].ActReadyAt.Sub(t0); got != 100*time.Millisecond {
			t.Fatalf("attack cooldown = %v", got)
		}
		if got := b.entities["enemy"].RespawnAt.Sub(t0); got != 2*time.Second {
			t.Fatalf("respawn delay = %v", got)
		}
	})

	t.Run("creation cost and cooldown", func(t *testing.T) {
		b := actionBoard()
		b.Rules.CreateHealthCost = 50
		b.Rules.Minor.Speed = 4
		out, err := b.Do("p1", Action{Kind: ActionCreate, UnitID: "champ"}, t0)
		if err != nil {
			t.Fatal(err)
		}
		if got := b.entities["champ"].Health; got != 150 {
			t.Fatalf("champion health = %d, want 150", got)
		}
		if out.Created[0].Speed != 4 {
			t.Fatalf("the new unit should use the world's minor stats, speed = %d", out.Created[0].Speed)
		}
	})

	t.Run("a free action (no cooldown)", func(t *testing.T) {
		b := actionBoard()
		b.Rules.CooldownsMs.Attack = 0
		if _, err := b.Do("p1", attack("champ", "enemy"), t0); err != nil {
			t.Fatal(err)
		}
		if _, err := b.Do("p1", attack("champ", "enemy"), t0); err != nil {
			t.Fatalf("with no cooldown the unit can act again at once: %v", err)
		}
	})
}
