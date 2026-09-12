package sim

import (
	"time"

	"thegame/internal/protocol"
	"thegame/internal/store"
	"thegame/internal/world"
)

// Unit is the in-memory, per-tick representation of a pedina living on a
// board's goroutine. It is hydrated from store.Unit on board start and
// persisted back via UnitRepo as commands are applied.
//
// MoveReadyAt/RespawnAt are ephemeral realtime timers. They are NOT persisted
// on every tick — RespawnAt is durable (survives restarts via units.respawn_at,
// docs/design.md: death is cooldown-only, never permanent) while MoveReadyAt
// is allowed to reset to "ready" on a restart since a lost fraction of a
// movement cooldown is harmless. This in-memory approach is a stand-in for
// the Redis hot-state layer planned in docs/tecnico.md, deferred until entity
// counts actually demand it.
type Unit struct {
	ID          string
	PlayerID    string
	BoardID     string
	IsChampion  bool
	Pos         world.Coord
	Speed       float64
	Health      float64
	MaxHealth   float64
	Alive       bool
	RespawnAt   time.Time
	MoveReadyAt time.Time
}

// FromStoreRecord builds the in-memory runtime unit from its persisted row,
// used both when a player joins a board and when rehydrating a board's units
// from Postgres on server startup.
func FromStoreRecord(rec store.Unit) *Unit {
	u := &Unit{
		ID:         rec.ID,
		PlayerID:   rec.PlayerID,
		BoardID:    rec.BoardID,
		IsChampion: rec.IsChampion,
		Pos:        world.Coord{X: rec.X, Y: rec.Y},
		Speed:      rec.Speed,
		Health:     rec.Health,
		MaxHealth:  rec.MaxHealth,
		Alive:      rec.Alive,
	}
	if rec.RespawnAt != nil {
		u.RespawnAt = *rec.RespawnAt
	}
	return u
}

func (u *Unit) ToProto() protocol.UnitState {
	return protocol.UnitState{
		ID:         u.ID,
		PlayerID:   u.PlayerID,
		IsChampion: u.IsChampion,
		X:          u.Pos.X,
		Y:          u.Pos.Y,
		Speed:      u.Speed,
		Health:     u.Health,
		MaxHealth:  u.MaxHealth,
		Alive:      u.Alive,
	}
}
