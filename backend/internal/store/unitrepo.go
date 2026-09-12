package store

import (
	"context"
	"errors"
	"time"

	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

const (
	defaultChampionSpeed  = 3.0
	defaultChampionHealth = 100.0

	// M2 hardcoded minor-unit stats and creation cost (one of the four
	// creation methods from docs/design.md: "consumo caratteristiche del
	// campione"). Becomes admin-configurable data in M4.
	MinorUnitSpeed           = 2.0
	MinorUnitHealth          = 50.0
	ChampionCostPerMinorUnit = 20.0
)

type UnitRepo struct {
	pool *pgxpool.Pool
}

func NewUnitRepo(pool *pgxpool.Pool) *UnitRepo {
	return &UnitRepo{pool: pool}
}

const unitColumns = `id, player_id, board_id, is_champion, x, y, speed, health, max_health, alive, respawn_at, created_at`

func scanUnit(row pgx.Row, u *Unit) error {
	return row.Scan(&u.ID, &u.PlayerID, &u.BoardID, &u.IsChampion, &u.X, &u.Y,
		&u.Speed, &u.Health, &u.MaxHealth, &u.Alive, &u.RespawnAt, &u.CreatedAt)
}

func (r *UnitRepo) ListByBoard(ctx context.Context, boardID string) ([]Unit, error) {
	rows, err := r.pool.Query(ctx, `SELECT `+unitColumns+` FROM units WHERE board_id = $1`, boardID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var out []Unit
	for rows.Next() {
		var u Unit
		if err := scanUnit(rows, &u); err != nil {
			return nil, err
		}
		out = append(out, u)
	}
	return out, rows.Err()
}

// GetOrCreateChampion returns the player's champion on this board, spawning
// one at (0,0) the first time the player enters (docs/design.md: champion +
// N minor units per player; the champion always exists first).
func (r *UnitRepo) GetOrCreateChampion(ctx context.Context, id, playerID, boardID string) (*Unit, error) {
	const selectQ = `SELECT ` + unitColumns + ` FROM units WHERE player_id = $1 AND board_id = $2 AND is_champion = true`
	var u Unit
	err := scanUnit(r.pool.QueryRow(ctx, selectQ, playerID, boardID), &u)
	if err == nil {
		return &u, nil
	}
	if !errors.Is(err, pgx.ErrNoRows) {
		return nil, err
	}

	const insertQ = `
		INSERT INTO units (id, player_id, board_id, is_champion, x, y, speed, health, max_health, alive)
		VALUES ($1, $2, $3, true, 0, 0, $4, $5, $5, true)
		RETURNING ` + unitColumns
	err = scanUnit(r.pool.QueryRow(ctx, insertQ, id, playerID, boardID, defaultChampionSpeed, defaultChampionHealth), &u)
	if err != nil {
		return nil, err
	}
	return &u, nil
}

// CreateMinorUnit persists a new non-champion unit (docs/design.md: units can
// be created during play without limit). M2 hardcodes the stats/cost; M4
// makes both admin-configurable per world.
func (r *UnitRepo) CreateMinorUnit(ctx context.Context, id, playerID, boardID string, x, y int) (*Unit, error) {
	const insertQ = `
		INSERT INTO units (id, player_id, board_id, is_champion, x, y, speed, health, max_health, alive)
		VALUES ($1, $2, $3, false, $4, $5, $6, $7, $7, true)
		RETURNING ` + unitColumns
	var u Unit
	err := scanUnit(r.pool.QueryRow(ctx, insertQ, id, playerID, boardID, x, y, MinorUnitSpeed, MinorUnitHealth), &u)
	if err != nil {
		return nil, err
	}
	return &u, nil
}

func (r *UnitRepo) UpdatePosition(ctx context.Context, unitID string, x, y int) error {
	const q = `UPDATE units SET x = $2, y = $3 WHERE id = $1`
	_, err := r.pool.Exec(ctx, q, unitID, x, y)
	return err
}

// UpdateCombatState persists the result of an attack or a respawn tick:
// current health, whether the unit is alive, and (if dead) when it respawns.
// Death is cooldown-only (docs/design.md: no permadeath, no other penalty).
func (r *UnitRepo) UpdateCombatState(ctx context.Context, unitID string, health float64, alive bool, respawnAt *time.Time) error {
	const q = `UPDATE units SET health = $2, alive = $3, respawn_at = $4 WHERE id = $1`
	_, err := r.pool.Exec(ctx, q, unitID, health, alive, respawnAt)
	return err
}
