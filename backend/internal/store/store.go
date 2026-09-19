package store

import (
	"context"
	"errors"
	"fmt"
	"io/fs"
	"strings"
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
		if !strings.HasSuffix(name, ".sql") {
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

// LoadWorld loads the first world with its rules, boards, gateways and everything on the boards,
// plus the players and their points and inventories.
func (s *Store) LoadWorld(ctx context.Context) (*game.World, error) {
	var id, name string
	var rulesJSON []byte
	err := s.pool.QueryRow(ctx, `SELECT id::text, name, rules FROM worlds ORDER BY created_at LIMIT 1`).
		Scan(&id, &name, &rulesJSON)
	if errors.Is(err, pgx.ErrNoRows) {
		return nil, errors.New("no world in the database")
	}
	if err != nil {
		return nil, err
	}
	rules, err := game.ParseRules(rulesJSON)
	if err != nil {
		return nil, fmt.Errorf("world rules: %w", err)
	}
	world := game.NewWorld(id, name)
	world.Rules = rules

	err = s.each(ctx, `SELECT id::text, name, width, height, grid_kind FROM boards
		WHERE world_id = $1::uuid ORDER BY position, created_at, name`,
		[]any{id}, func(rows pgx.Rows) error {
			b := &game.Board{}
			var kind string
			if err := rows.Scan(&b.ID, &b.Name, &b.Width, &b.Height, &kind); err != nil {
				return err
			}
			grid, err := game.NewGrid(kind)
			if err != nil {
				return fmt.Errorf("board %q: %w", b.Name, err)
			}
			b.Grid = grid
			world.AddBoard(b)
			return nil
		})
	if err != nil {
		return nil, err
	}
	if len(world.Boards()) == 0 {
		return nil, errors.New("the world has no boards")
	}

	err = s.each(ctx, `SELECT l.from_board_id::text, l.from_x, l.from_y, l.to_board_id::text, l.to_x, l.to_y
		FROM board_links l JOIN boards b ON b.id = l.from_board_id WHERE b.world_id = $1::uuid`,
		[]any{id}, func(rows pgx.Rows) error {
			var from, to game.Cell
			if err := rows.Scan(&from.Board, &from.X, &from.Y, &to.Board, &to.X, &to.Y); err != nil {
				return err
			}
			world.AddLink(from, to)
			return nil
		})
	if err != nil {
		return nil, err
	}

	err = s.each(ctx, `SELECT u.id::text, u.board_id::text, COALESCE(u.player_id::text, ''), u.kind, u.name, u.description,
		u.x, u.y, u.speed, u.health, u.max_health, u.vision, u.strength, u.dialogue
		FROM units u JOIN boards b ON b.id = u.board_id WHERE b.world_id = $1::uuid`,
		[]any{id}, func(rows pgx.Rows) error {
			e := &game.Entity{}
			var kind, dialogue string
			if err := rows.Scan(&e.ID, &e.BoardID, &e.OwnerID, &kind, &e.Name, &e.Description,
				&e.X, &e.Y, &e.Speed, &e.Health, &e.MaxHealth, &e.Vision, &e.Strength, &dialogue); err != nil {
				return err
			}
			e.Kind = game.Kind(kind)
			if dialogue != "" {
				e.Dialogue = strings.Split(dialogue, "\n")
			}
			world.Add(e)
			return nil
		})
	if err != nil {
		return nil, err
	}

	err = s.each(ctx, `SELECT i.id::text, i.board_id::text, i.name, i.description, i.x, i.y
		FROM board_items i JOIN boards b ON b.id = i.board_id WHERE b.world_id = $1::uuid`,
		[]any{id}, func(rows pgx.Rows) error {
			e := &game.Entity{Kind: game.KindItem}
			if err := rows.Scan(&e.ID, &e.BoardID, &e.Name, &e.Description, &e.X, &e.Y); err != nil {
				return err
			}
			world.Add(e)
			return nil
		})
	if err != nil {
		return nil, err
	}

	err = s.each(ctx, `SELECT st.id::text, st.board_id::text, st.player_id::text, st.name, st.description, st.x, st.y
		FROM structures st JOIN boards b ON b.id = st.board_id WHERE b.world_id = $1::uuid`,
		[]any{id}, func(rows pgx.Rows) error {
			e := &game.Entity{Kind: game.KindStructure}
			if err := rows.Scan(&e.ID, &e.BoardID, &e.OwnerID, &e.Name, &e.Description, &e.X, &e.Y); err != nil {
				return err
			}
			world.Add(e)
			return nil
		})
	if err != nil {
		return nil, err
	}

	err = s.each(ctx, `SELECT id::text, username, points FROM players`, nil, func(rows pgx.Rows) error {
		var pid, username string
		var points int
		if err := rows.Scan(&pid, &username, &points); err != nil {
			return err
		}
		world.EnsurePlayer(pid, username, points)
		return nil
	})
	if err != nil {
		return nil, err
	}

	err = s.each(ctx, `SELECT player_id::text, name, description FROM inventory_items ORDER BY acquired_at`, nil,
		func(rows pgx.Rows) error {
			var pid string
			var item game.Item
			if err := rows.Scan(&pid, &item.Name, &item.Description); err != nil {
				return err
			}
			p := world.Player(pid)
			p.Inventory = append(p.Inventory, item)
			return nil
		})
	if err != nil {
		return nil, err
	}
	return world, nil
}

// each runs a query and calls fn for every row.
func (s *Store) each(ctx context.Context, query string, args []any, fn func(pgx.Rows) error) error {
	rows, err := s.pool.Query(ctx, query, args...)
	if err != nil {
		return err
	}
	defer rows.Close()
	for rows.Next() {
		if err := fn(rows); err != nil {
			return err
		}
	}
	return rows.Err()
}

// InsertUnits stores new units atomically. Units that already have an ID keep it; the others
// get one from the database, which is filled in.
func (s *Store) InsertUnits(ctx context.Context, units []*game.Entity) error {
	return pgx.BeginFunc(ctx, s.pool, func(tx pgx.Tx) error {
		for _, u := range units {
			err := tx.QueryRow(ctx, `INSERT INTO units
				(id, board_id, player_id, kind, name, description, x, y, speed, health, max_health, vision, strength)
				VALUES (COALESCE(NULLIF($1, '')::uuid, gen_random_uuid()), $2::uuid, NULLIF($3, '')::uuid, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
				RETURNING id::text`,
				u.ID, u.BoardID, u.OwnerID, string(u.Kind), u.Name, u.Description, u.X, u.Y,
				u.Speed, u.Health, u.MaxHealth, u.Vision, u.Strength).Scan(&u.ID)
			if err != nil {
				return err
			}
		}
		return nil
	})
}

func (s *Store) SaveUnit(ctx context.Context, id, boardID string, x, y, health int) error {
	_, err := s.pool.Exec(ctx, `UPDATE units SET board_id = $2::uuid, x = $3, y = $4, health = $5 WHERE id::text = $1`,
		id, boardID, x, y, health)
	return err
}

func (s *Store) InsertStructure(ctx context.Context, st *game.Entity) error {
	_, err := s.pool.Exec(ctx, `INSERT INTO structures (id, board_id, player_id, name, description, x, y)
		VALUES ($1::uuid, $2::uuid, $3::uuid, $4, $5, $6, $7)`,
		st.ID, st.BoardID, st.OwnerID, st.Name, st.Description, st.X, st.Y)
	return err
}

func (s *Store) SaveItemPosition(ctx context.Context, id string, x, y int) error {
	_, err := s.pool.Exec(ctx, `UPDATE board_items SET x = $2, y = $3 WHERE id::text = $1`, id, x, y)
	return err
}

func (s *Store) DeleteItem(ctx context.Context, id string) error {
	_, err := s.pool.Exec(ctx, `DELETE FROM board_items WHERE id::text = $1`, id)
	return err
}

func (s *Store) AddInventory(ctx context.Context, playerID string, item game.Item) error {
	_, err := s.pool.Exec(ctx, `INSERT INTO inventory_items (player_id, name, description) VALUES ($1::uuid, $2, $3)`,
		playerID, item.Name, item.Description)
	return err
}

func (s *Store) AddPoints(ctx context.Context, playerID string, delta int) error {
	_, err := s.pool.Exec(ctx, `UPDATE players SET points = points + $2 WHERE id::text = $1`, playerID, delta)
	return err
}
