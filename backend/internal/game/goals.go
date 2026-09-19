package game

import (
	"fmt"
	"math/rand/v2"
)

// GoalScope says who a goal is for. design.md: the end of the game is not one global event, so
// goals exist for the whole world (the first to reach one wins it) and for each player.
type GoalScope string

const (
	GoalWorld      GoalScope = "world"
	GoalIndividual GoalScope = "individual"
)

// GoalKind is what a goal measures.
type GoalKind string

const (
	GoalPoints        GoalKind = "points"         // the team's points
	GoalKills         GoalKind = "kills"          // enemy units defeated
	GoalChampionKills GoalKind = "champion_kills" // enemy champions defeated
	GoalPickups       GoalKind = "pickups"        // objects picked up
	GoalStructures    GoalKind = "structures"     // structures the team owns
	GoalUnits         GoalKind = "units"          // units the team has
	GoalTalks         GoalKind = "talks"          // different NPCs spoken to
)

// GoalKinds lists every kind of goal (the admin app offers them in a menu).
func GoalKinds() []GoalKind {
	return []GoalKind{GoalPoints, GoalKills, GoalChampionKills, GoalPickups, GoalStructures, GoalUnits, GoalTalks}
}

func (k GoalKind) Valid() bool {
	for _, known := range GoalKinds() {
		if k == known {
			return true
		}
	}
	return false
}

// Goal is defined by the world's admin. Completing an individual goal (or winning a world goal)
// gives the player Reward points.
type Goal struct {
	ID          string
	Scope       GoalScope
	Kind        GoalKind
	Target      int
	Reward      int
	Title       string
	Description string
	AchievedBy  string // world goals: the player who reached it first ("" = still open)
}

// Stats are the counters goals are measured with, kept per player.
type Stats struct {
	Kills         int             `json:"kills"`
	ChampionKills int             `json:"champion_kills"`
	Pickups       int             `json:"pickups"`
	Talked        map[string]bool `json:"talked"` // ids of the NPCs already spoken to
}

// GoalRef points at a goal of a player, for persistence.
type GoalRef struct {
	PlayerID string
	GoalID   string
}

// GoalView is a goal as one player sees it.
type GoalView struct {
	ID          string
	Scope       GoalScope
	Kind        GoalKind
	Title       string
	Description string
	Target      int
	Progress    int
	Reward      int
	Completed   bool
	AchievedBy  string // the winner's name, for world goals
}

func (w *World) AddGoal(g *Goal) {
	w.goals[g.ID] = g
	w.goalOrder = append(w.goalOrder, g.ID)
}

func (w *World) Goals() []*Goal {
	out := make([]*Goal, 0, len(w.goalOrder))
	for _, id := range w.goalOrder {
		out = append(out, w.goals[id])
	}
	return out
}

// progress is how far a player is from a goal, measured by the goal's kind.
func (w *World) progress(p *Player, g *Goal) int {
	switch g.Kind {
	case GoalPoints:
		return p.Points
	case GoalKills:
		return p.Stats.Kills
	case GoalChampionKills:
		return p.Stats.ChampionKills
	case GoalPickups:
		return p.Stats.Pickups
	case GoalTalks:
		return len(p.Stats.Talked)
	}
	n := 0
	for _, e := range w.entities {
		if e.OwnerID != p.ID {
			continue
		}
		if (g.Kind == GoalStructures && e.Kind == KindStructure) || (g.Kind == GoalUnits && e.Controllable()) {
			n++
		}
	}
	return n
}

// GoalViews lists what a player sees: every world goal, plus their own individual goals (the
// current one and the ones they completed).
func (w *World) GoalViews(playerID string) []GoalView {
	p := w.players[playerID]
	if p == nil {
		return nil
	}
	var out []GoalView
	for _, g := range w.Goals() {
		v := GoalView{
			ID: g.ID, Scope: g.Scope, Kind: g.Kind, Title: g.Title, Description: g.Description,
			Target: g.Target, Progress: min(w.progress(p, g), g.Target), Reward: g.Reward,
		}
		switch g.Scope {
		case GoalWorld:
			if g.AchievedBy != "" {
				v.Completed = true
				if winner := w.players[g.AchievedBy]; winner != nil {
					v.AchievedBy = winner.Username
				}
			}
		case GoalIndividual:
			if !p.CompletedGoals[g.ID] && p.GoalID != g.ID {
				continue
			}
			v.Completed = p.CompletedGoals[g.ID]
		}
		out = append(out, v)
	}
	return out
}

// pickGoal chooses an individual goal at random among those the player has not completed.
func (w *World) pickGoal(p *Player) *Goal {
	var candidates []*Goal
	for _, g := range w.Goals() {
		if g.Scope == GoalIndividual && !p.CompletedGoals[g.ID] && g.ID != p.GoalID {
			candidates = append(candidates, g)
		}
	}
	if len(candidates) == 0 {
		return nil
	}
	return candidates[rand.IntN(len(candidates))]
}

// AssignGoal gives a player an individual goal (the admin does it in worlds with "manual"
// assignment). It replaces the goal they had if they had not completed it.
func (w *World) AssignGoal(playerID, goalID string) error {
	p, g := w.players[playerID], w.goals[goalID]
	if p == nil {
		return ErrNotFound
	}
	if g == nil || g.Scope != GoalIndividual {
		return ErrInvalidTarget
	}
	if p.CompletedGoals[goalID] {
		return ErrGoalDone
	}
	p.GoalID = goalID
	return nil
}

// EnsureGoal hands a player their first individual goal when the world assigns them at random.
// It returns the assignment, if one was made.
func (w *World) EnsureGoal(playerID string) *GoalRef {
	p := w.players[playerID]
	if p == nil || p.GoalID != "" || w.Rules.GoalAssignment != "random" {
		return nil
	}
	if g := w.pickGoal(p); g != nil {
		p.GoalID = g.ID
		return &GoalRef{playerID, g.ID}
	}
	return nil
}

// Evaluate checks a player's goals against their progress and returns what changed (or nil). A
// reward can itself push a "points" goal over its target, so it repeats until nothing changes.
// The reward points are already added to the player; the outcome carries them for persistence.
func (w *World) Evaluate(playerID string) *Outcome {
	p := w.players[playerID]
	if p == nil {
		return nil
	}
	out := &Outcome{}
	reward := func(n int) {
		p.Points += n
		out.Points += n
	}
	for range 5 {
		changed := false
		if g := w.goals[p.GoalID]; g != nil && !p.CompletedGoals[g.ID] && w.progress(p, g) >= g.Target {
			p.CompletedGoals[g.ID] = true
			p.GoalID = ""
			reward(g.Reward)
			out.CompletedGoals = append(out.CompletedGoals, GoalRef{playerID, g.ID})
			out.Notices = append(out.Notices, Notice{playerID, "Obiettivo raggiunto", fmt.Sprintf("%s: +%d punti.", g.Title, g.Reward)})
			if w.Rules.GoalAssignment == "random" {
				if next := w.pickGoal(p); next != nil {
					p.GoalID = next.ID
					out.AssignedGoals = append(out.AssignedGoals, GoalRef{playerID, next.ID})
					out.Notices = append(out.Notices, Notice{playerID, "Nuovo obiettivo", fmt.Sprintf("%s: %s", next.Title, next.Description)})
				}
			}
			changed = true
		}
		for _, g := range w.Goals() {
			if g.Scope != GoalWorld || g.AchievedBy != "" || w.progress(p, g) < g.Target {
				continue
			}
			g.AchievedBy = p.ID
			reward(g.Reward)
			out.Victories = append(out.Victories, GoalRef{playerID, g.ID})
			out.Broadcast = append(out.Broadcast, Notice{"", "Vittoria", fmt.Sprintf("%s ha vinto: %s.", p.Username, g.Title)})
			changed = true
		}
		if !changed {
			break
		}
	}
	if len(out.CompletedGoals) == 0 && len(out.Victories) == 0 {
		return nil
	}
	out.GoalsChanged = true
	return out
}

// merge adds what evaluating goals produced to the outcome of the action that caused it.
func (o *Outcome) merge(ev *Outcome) {
	if ev == nil {
		return
	}
	o.Points += ev.Points
	o.Notices = append(o.Notices, ev.Notices...)
	o.Broadcast = append(o.Broadcast, ev.Broadcast...)
	o.CompletedGoals = append(o.CompletedGoals, ev.CompletedGoals...)
	o.AssignedGoals = append(o.AssignedGoals, ev.AssignedGoals...)
	o.Victories = append(o.Victories, ev.Victories...)
	o.GoalsChanged = o.GoalsChanged || ev.GoalsChanged
}
