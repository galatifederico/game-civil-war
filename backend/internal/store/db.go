// Package store holds Postgres repositories, one file per aggregate. All
// queries are plain SQL via pgx (no ORM) — see docs/tecnico.md for why
// Postgres was chosen (JSONB for admin-configurable rules, concurrent writes).
package store

import (
	"context"

	"github.com/jackc/pgx/v5/pgxpool"
)

func NewPool(ctx context.Context, databaseURL string) (*pgxpool.Pool, error) {
	pool, err := pgxpool.New(ctx, databaseURL)
	if err != nil {
		return nil, err
	}
	if err := pool.Ping(ctx); err != nil {
		pool.Close()
		return nil, err
	}
	return pool, nil
}
