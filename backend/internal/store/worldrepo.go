package store

import (
	"context"
	"errors"

	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

type WorldRepo struct {
	pool *pgxpool.Pool
}

func NewWorldRepo(pool *pgxpool.Pool) *WorldRepo {
	return &WorldRepo{pool: pool}
}

// ListWorlds backs the join lobby (docs/design.md: players join via a list of
// existing worlds, not invite codes or manual addresses).
func (r *WorldRepo) ListWorlds(ctx context.Context) ([]World, error) {
	rows, err := r.pool.Query(ctx, `SELECT id, name, created_at FROM worlds ORDER BY created_at`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var out []World
	for rows.Next() {
		var w World
		if err := rows.Scan(&w.ID, &w.Name, &w.CreatedAt); err != nil {
			return nil, err
		}
		out = append(out, w)
	}
	return out, rows.Err()
}

func (r *WorldRepo) GetWorld(ctx context.Context, id string) (*World, error) {
	const q = `SELECT id, name, created_at FROM worlds WHERE id = $1`
	var w World
	err := r.pool.QueryRow(ctx, q, id).Scan(&w.ID, &w.Name, &w.CreatedAt)
	if errors.Is(err, pgx.ErrNoRows) {
		return nil, ErrNotFound
	}
	return &w, err
}

// FirstBoardOfWorld returns the entry-point board for a world. With multiple
// boards (M3) this becomes "spawn board" selection instead of "the only one".
func (r *WorldRepo) FirstBoardOfWorld(ctx context.Context, worldID string) (*Board, error) {
	const q = `SELECT id, world_id, name, grid_type, width, height
	           FROM boards WHERE world_id = $1 ORDER BY name LIMIT 1`
	var b Board
	err := r.pool.QueryRow(ctx, q, worldID).Scan(&b.ID, &b.WorldID, &b.Name, &b.GridType, &b.Width, &b.Height)
	if errors.Is(err, pgx.ErrNoRows) {
		return nil, ErrNotFound
	}
	return &b, err
}

func (r *WorldRepo) GetBoard(ctx context.Context, boardID string) (*Board, error) {
	const q = `SELECT id, world_id, name, grid_type, width, height FROM boards WHERE id = $1`
	var b Board
	err := r.pool.QueryRow(ctx, q, boardID).Scan(&b.ID, &b.WorldID, &b.Name, &b.GridType, &b.Width, &b.Height)
	if errors.Is(err, pgx.ErrNoRows) {
		return nil, ErrNotFound
	}
	return &b, err
}

// EnsureMembership joins a player to a world if not already a member.
// The first player to join a world becomes its admin (docs/design.md: an
// admin can also play in their own world) — good enough default until M6
// gives admins an explicit way to promote others.
func (r *WorldRepo) EnsureMembership(ctx context.Context, playerID, worldID string) (isAdmin bool, err error) {
	const selectQ = `SELECT is_admin FROM player_world_membership WHERE player_id = $1 AND world_id = $2`
	err = r.pool.QueryRow(ctx, selectQ, playerID, worldID).Scan(&isAdmin)
	if err == nil {
		return isAdmin, nil
	}
	if !errors.Is(err, pgx.ErrNoRows) {
		return false, err
	}

	const countQ = `SELECT count(*) FROM player_world_membership WHERE world_id = $1`
	var memberCount int
	if err := r.pool.QueryRow(ctx, countQ, worldID).Scan(&memberCount); err != nil {
		return false, err
	}
	isAdmin = memberCount == 0

	const insertQ = `INSERT INTO player_world_membership (player_id, world_id, is_admin) VALUES ($1, $2, $3)`
	if _, err := r.pool.Exec(ctx, insertQ, playerID, worldID, isAdmin); err != nil {
		return false, err
	}
	return isAdmin, nil
}
