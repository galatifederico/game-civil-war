package store

import (
	"context"
	"errors"

	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

var ErrNotFound = errors.New("not found")
var ErrAlreadyExists = errors.New("already exists")

type PlayerRepo struct {
	pool *pgxpool.Pool
}

func NewPlayerRepo(pool *pgxpool.Pool) *PlayerRepo {
	return &PlayerRepo{pool: pool}
}

func (r *PlayerRepo) Create(ctx context.Context, id, email, passwordHash string) (*Player, error) {
	const q = `
		INSERT INTO players (id, email, password_hash)
		VALUES ($1, $2, $3)
		RETURNING id, email, password_hash, created_at`
	var p Player
	err := r.pool.QueryRow(ctx, q, id, email, passwordHash).Scan(&p.ID, &p.Email, &p.PasswordHash, &p.CreatedAt)
	if err != nil {
		if isUniqueViolation(err) {
			return nil, ErrAlreadyExists
		}
		return nil, err
	}
	return &p, nil
}

func (r *PlayerRepo) FindByEmail(ctx context.Context, email string) (*Player, error) {
	const q = `SELECT id, email, password_hash, created_at FROM players WHERE email = $1`
	var p Player
	err := r.pool.QueryRow(ctx, q, email).Scan(&p.ID, &p.Email, &p.PasswordHash, &p.CreatedAt)
	if errors.Is(err, pgx.ErrNoRows) {
		return nil, ErrNotFound
	}
	if err != nil {
		return nil, err
	}
	return &p, nil
}

func (r *PlayerRepo) FindByID(ctx context.Context, id string) (*Player, error) {
	const q = `SELECT id, email, password_hash, created_at FROM players WHERE id = $1`
	var p Player
	err := r.pool.QueryRow(ctx, q, id).Scan(&p.ID, &p.Email, &p.PasswordHash, &p.CreatedAt)
	if errors.Is(err, pgx.ErrNoRows) {
		return nil, ErrNotFound
	}
	if err != nil {
		return nil, err
	}
	return &p, nil
}

func isUniqueViolation(err error) bool {
	var pgErr interface{ SQLState() string }
	if errors.As(err, &pgErr) {
		return pgErr.SQLState() == "23505"
	}
	return false
}
