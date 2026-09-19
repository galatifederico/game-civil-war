package store

import (
	"context"
	"errors"
	"fmt"
	"io/fs"
	"time"

	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgconn"
	"github.com/jackc/pgx/v5/pgxpool"

	"thegame/backend/internal/game"
	"thegame/backend/migrations"
)

var (
	ErrNotFound   = errors.New("not found")
	ErrEmailTaken = errors.New("email already registered")
)

type Player struct {
	ID           string
	Email        string
	Username     string
	PasswordHash string
}

type Store struct {
	pool *pgxpool.Pool
}

func Connect(ctx context.Context, dsn string) (*Store, error) {
	pool, err := pgxpool.New(ctx, dsn)
	if err != nil {
		return nil, err
	}
	pingCtx, cancel := context.WithTimeout(ctx, 5*time.Second)
	defer cancel()
	if err := pool.Ping(pingCtx); err != nil {
		pool.Close()
		return nil, fmt.Errorf("ping database: %w", err)
	}
	return &Store{pool: pool}, nil
}

func (s *Store) Close() { s.pool.Close() }

// Migrate applies the embedded SQL files in name order, once each.
func (s *Store) Migrate(ctx context.Context) error {
	if _, err := s.pool.Exec(ctx, `CREATE TABLE IF NOT EXISTS schema_migrations (
		name TEXT PRIMARY KEY, applied_at TIMESTAMPTZ NOT NULL DEFAULT now())`); err != nil {
		return err
	}
	entries, err := fs.ReadDir(migrations.FS, ".")
	if err != nil {
		return err
	}
	for _, entry := range entries {
		name := entry.Name()
		if len(name) < 4 || name[len(name)-4:] != ".sql" {
			continue
		}
		var applied bool
		if err := s.pool.QueryRow(ctx, `SELECT EXISTS (SELECT 1 FROM schema_migrations WHERE name = $1)`, name).Scan(&applied); err != nil {
			return err
		}
		if applied {
			continue
		}
		sqlText, err := fs.ReadFile(migrations.FS, name)
		if err != nil {
			return err
		}
		if err := pgx.BeginFunc(ctx, s.pool, func(tx pgx.Tx) error {
			if _, err := tx.Exec(ctx, string(sqlText)); err != nil {
				return err
			}
			_, err := tx.Exec(ctx, `INSERT INTO schema_migrations (name) VALUES ($1)`, name)
			return err
		}); err != nil {
			return fmt.Errorf("migration %s: %w", name, err)
		}
	}
	return nil
}

func (s *Store) CreatePlayer(ctx context.Context, email, username, passwordHash string) (Player, error) {
	p := Player{Email: email, Username: username, PasswordHash: passwordHash}
	err := s.pool.QueryRow(ctx,
		`INSERT INTO players (email, username, password_hash) VALUES ($1, $2, $3) RETURNING id::text`,
		email, username, passwordHash).Scan(&p.ID)
	var pgErr *pgconn.PgError
	if errors.As(err, &pgErr) && pgErr.Code == "23505" {
		return Player{}, ErrEmailTaken
	}
	return p, err
}

func (s *Store) PlayerByEmail(ctx context.Context, email string) (Player, error) {
	return s.player(ctx, `SELECT id::text, email, username, password_hash FROM players WHERE email = $1`, email)
}

func (s *Store) PlayerByID(ctx context.Context, id string) (Player, error) {
	return s.player(ctx, `SELECT id::text, email, username, password_hash FROM players WHERE id::text = $1`, id)
}

func (s *Store) player(ctx context.Context, query string, arg any) (Player, error) {
	var p Player
	err := s.pool.QueryRow(ctx, query, arg).Scan(&p.ID, &p.Email, &p.Username, &p.PasswordHash)
	if errors.Is(err, pgx.ErrNoRows) {
		return Player{}, ErrNotFound
	}
	return p, err
}

// LoadDefaultBoard loads the first board with all its units and items.
func (s *Store) LoadDefaultBoard(ctx context.Context) (*game.Board, error) {
	var id, name string
	var width, height int
	err := s.pool.QueryRow(ctx, `SELECT id::text, name, width, height FROM boards ORDER BY created_at LIMIT 1`).
		Scan(&id, &name, &width, &height)
	if errors.Is(err, pgx.ErrNoRows) {
		return nil, errors.New("no board in the database")
	}
	if err != nil {
		return nil, err
	}

	var entities []*game.Entity

	rows, err := s.pool.Query(ctx, `SELECT id::text, COALESCE(player_id::text, ''), kind, name, description,
		x, y, speed, health, max_health, vision FROM units WHERE board_id = $1`, id)
	if err != nil {
		return nil, err
	}
	for rows.Next() {
		e := &game.Entity{}
		var kind string
		if err := rows.Scan(&e.ID, &e.OwnerID, &kind, &e.Name, &e.Description,
			&e.X, &e.Y, &e.Speed, &e.Health, &e.MaxHealth, &e.Vision); err != nil {
			rows.Close()
			return nil, err
		}
		e.Kind = game.Kind(kind)
		entities = append(entities, e)
	}
	rows.Close()
	if err := rows.Err(); err != nil {
		return nil, err
	}

	rows, err = s.pool.Query(ctx, `SELECT id::text, name, description, x, y FROM board_items WHERE board_id = $1`, id)
	if err != nil {
		return nil, err
	}
	for rows.Next() {
		e := &game.Entity{Kind: game.KindItem}
		if err := rows.Scan(&e.ID, &e.Name, &e.Description, &e.X, &e.Y); err != nil {
			rows.Close()
			return nil, err
		}
		entities = append(entities, e)
	}
	rows.Close()
	if err := rows.Err(); err != nil {
		return nil, err
	}

	return game.NewBoard(id, name, width, height, entities), nil
}

// InsertUnits stores new units atomically and fills in their IDs.
func (s *Store) InsertUnits(ctx context.Context, boardID string, units []*game.Entity) error {
	return pgx.BeginFunc(ctx, s.pool, func(tx pgx.Tx) error {
		for _, u := range units {
			err := tx.QueryRow(ctx, `INSERT INTO units
				(board_id, player_id, kind, name, description, x, y, speed, health, max_health, vision)
				VALUES ($1, NULLIF($2, '')::uuid, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id::text`,
				boardID, u.OwnerID, string(u.Kind), u.Name, u.Description, u.X, u.Y,
				u.Speed, u.Health, u.MaxHealth, u.Vision).Scan(&u.ID)
			if err != nil {
				return err
			}
		}
		return nil
	})
}

func (s *Store) SaveUnitPosition(ctx context.Context, id string, x, y int) error {
	_, err := s.pool.Exec(ctx, `UPDATE units SET x = $2, y = $3 WHERE id::text = $1`, id, x, y)
	return err
}
