package game

import "time"

// Away players (design.md: the world never stops, so players who are not there must not become
// free targets, nor keep the scoreboard looking alive). A player is active when they connect,
// disconnect or send a command; Rules.AFK says how long without any of that makes them afk.

// StartActivity counts everybody as active as of now. The loop calls it when it starts: the
// database does not remember when players were last seen.
func (w *World) StartActivity(now time.Time) {
	for _, p := range w.players {
		p.LastActive = now
	}
}

// Touch records activity of a player. It reports whether that brought them back from afk.
func (w *World) Touch(playerID string, now time.Time) bool {
	p := w.players[playerID]
	if p == nil {
		return false
	}
	p.LastActive = now
	back := p.AFK
	p.AFK = false
	return back
}

// UpdateAFK marks the players who have been away for long enough. It reports whether anyone changed.
func (w *World) UpdateAFK(now time.Time) bool {
	after := ms(w.Rules.AFK.AfterMs)
	if after <= 0 {
		return false
	}
	changed := false
	for _, p := range w.players {
		if !p.AFK && !p.LastActive.IsZero() && now.Sub(p.LastActive) >= after {
			p.AFK = true
			changed = true
		}
	}
	return changed
}
