package game

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"time"
)

// UnitStats are the starting characteristics of a kind of unit.
type UnitStats struct {
	Speed    int `json:"speed"`    // max cells per move
	Health   int `json:"health"`   // hit points
	Vision   int `json:"vision"`   // sight and action range, in cells
	Strength int `json:"strength"` // damage of an attack
}

// Rules are the parameters of a world. They are data, not code (tecnico.md: the rules engine
// must be configurable per world): the defaults below apply to anything a world does not set,
// and the database stores only the differences (worlds.rules, as JSON). Durations are in
// milliseconds so the JSON stays plain numbers.
type Rules struct {
	Champion         UnitStats `json:"champion"`
	Minor            UnitStats `json:"minor"`
	MinorsPerTeam    int       `json:"minors_per_team"` // minor units a new team starts with
	RespawnMs        int       `json:"respawn_ms"`      // how long a defeated unit stays out
	CreateHealthCost int       `json:"create_health_cost"`

	CooldownsMs struct {
		Attack   int `json:"attack"`
		Pickup   int `json:"pickup"`
		Build    int `json:"build"`
		Create   int `json:"create"`
		MoveItem int `json:"move_item"`
	} `json:"cooldowns_ms"`

	// Points awarded to the acting team; design.md allows them to be negative too.
	Points struct {
		Hit          int `json:"hit"`
		KillMinor    int `json:"kill_minor"`
		KillChampion int `json:"kill_champion"` // a big bonus, not a win (design.md)
		Pickup       int `json:"pickup"`
		Build        int `json:"build"`
		Create       int `json:"create"`
	} `json:"points"`
}

func DefaultRules() Rules {
	r := Rules{
		Champion:         UnitStats{Speed: 3, Health: 200, Vision: 5, Strength: 30},
		Minor:            UnitStats{Speed: 2, Health: 100, Vision: 3, Strength: 15},
		MinorsPerTeam:    12,
		RespawnMs:        10000,
		CreateHealthCost: 20,
	}
	r.CooldownsMs.Attack = 1500
	r.CooldownsMs.Pickup = 500
	r.CooldownsMs.Build = 3000
	r.CooldownsMs.Create = 5000
	r.CooldownsMs.MoveItem = 500
	r.Points.Hit = 5
	r.Points.KillMinor = 25
	r.Points.KillChampion = 100
	r.Points.Pickup = 5
	r.Points.Build = 20
	r.Points.Create = 10
	return r
}

func ms(v int) time.Duration { return time.Duration(v) * time.Millisecond }

func (r Rules) RespawnDelay() time.Duration     { return ms(r.RespawnMs) }
func (r Rules) AttackCooldown() time.Duration   { return ms(r.CooldownsMs.Attack) }
func (r Rules) PickupCooldown() time.Duration   { return ms(r.CooldownsMs.Pickup) }
func (r Rules) BuildCooldown() time.Duration    { return ms(r.CooldownsMs.Build) }
func (r Rules) CreateCooldown() time.Duration   { return ms(r.CooldownsMs.Create) }
func (r Rules) MoveItemCooldown() time.Duration { return ms(r.CooldownsMs.MoveItem) }

// Validate rejects values that would break the simulation, so a bad world configuration is
// reported when the server starts instead of showing up as odd behavior during play.
func (r Rules) Validate() error {
	var problems []error
	for name, s := range map[string]UnitStats{"champion": r.Champion, "minor": r.Minor} {
		switch {
		case s.Speed < 1:
			problems = append(problems, fmt.Errorf("%s.speed must be at least 1", name))
		case s.Health < 1:
			problems = append(problems, fmt.Errorf("%s.health must be at least 1", name))
		case s.Vision < 1:
			problems = append(problems, fmt.Errorf("%s.vision must be at least 1", name))
		case s.Strength < 0:
			problems = append(problems, fmt.Errorf("%s.strength cannot be negative", name))
		}
	}
	if r.MinorsPerTeam < 0 {
		problems = append(problems, errors.New("minors_per_team cannot be negative"))
	}
	if r.RespawnMs < 0 || r.CreateHealthCost < 0 {
		problems = append(problems, errors.New("respawn_ms and create_health_cost cannot be negative"))
	}
	for name, v := range map[string]int{
		"attack": r.CooldownsMs.Attack, "pickup": r.CooldownsMs.Pickup, "build": r.CooldownsMs.Build,
		"create": r.CooldownsMs.Create, "move_item": r.CooldownsMs.MoveItem,
	} {
		if v < 0 {
			problems = append(problems, fmt.Errorf("cooldowns_ms.%s cannot be negative", name))
		}
	}
	return errors.Join(problems...)
}

// ParseRules builds a world's rules from its stored JSON: the defaults, overridden by whatever
// the JSON sets. Unknown fields are an error (a typo in a rule name would otherwise be silently
// ignored) and so are values that Validate rejects.
func ParseRules(raw []byte) (Rules, error) {
	r := DefaultRules()
	if len(bytes.TrimSpace(raw)) > 0 {
		dec := json.NewDecoder(bytes.NewReader(raw))
		dec.DisallowUnknownFields()
		if err := dec.Decode(&r); err != nil {
			return Rules{}, fmt.Errorf("parse rules: %w", err)
		}
	}
	if err := r.Validate(); err != nil {
		return Rules{}, fmt.Errorf("invalid rules: %w", err)
	}
	return r, nil
}
