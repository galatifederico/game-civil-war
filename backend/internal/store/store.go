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

// LoadWorld loads a world with its rules, boards, gateways and everything on the boards, plus the
// players that joined it with their points and inventories.
func (s *Store) LoadWorld(ctx context.Context, id string) (*game.World, error) {
	var name string
	var rulesJSON []byte
	err := s.pool.QueryRow(ctx, `SELECT name, rules FROM worlds WHERE id = $1::uuid`, id).Scan(&name, &rulesJSON)
	if errors.Is(err, pgx.ErrNoRows) {
		return nil, fmt.Errorf("world %s not found", id)
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

	err = s.each(ctx, `SELECT t.board_id::text, t.x, t.y, t.tile FROM board_terrain t
		JOIN boards b ON b.id = t.board_id WHERE b.world_id = $1::uuid`, []any{id}, func(rows pgx.Rows) error {
		var boardID, tile string
		var p game.Point
		if err := rows.Scan(&boardID, &p.X, &p.Y, &tile); err != nil {
			return err
		}
		if b := world.Board(boardID); b != nil {
			b.SetTerrain(p, tile)
		}
		return nil
	})
	if err != nil {
		return nil, err
	}

	err = s.each(ctx, `SELECT id::text, name, description, speed, health, vision, strength,
		bonus_speed, bonus_health, bonus_vision, bonus_strength, traits_min, traits_bonus
		FROM races WHERE world_id = $1::uuid ORDER BY position, name`,
		[]any{id}, func(rows pgx.Rows) error {
			r := &game.Race{}
			if err := rows.Scan(&r.ID, &r.Name, &r.Description, &r.Min.Speed, &r.Min.Health, &r.Min.Vision, &r.Min.Strength,
				&r.Bonus.Speed, &r.Bonus.Health, &r.Bonus.Vision, &r.Bonus.Strength, &r.TraitsMin, &r.TraitsBonus); err != nil {
				return err
			}
			world.AddRace(r)
			return nil
		})
	if err != nil {
		return nil, err
	}
	err = s.each(ctx, `SELECT race_a::text, race_b::text, child_race::text FROM race_compatibility WHERE world_id = $1::uuid`,
		[]any{id}, func(rows pgx.Rows) error {
			var a, b, child string
			if err := rows.Scan(&a, &b, &child); err != nil {
				return err
			}
			world.AddCompat(a, b, child)
			return nil
		})
	if err != nil {
		return nil, err
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
		u.x, u.y, u.speed, u.health, u.max_health, u.vision, u.strength, u.dialogue,
		COALESCE(u.race_id::text, ''), u.traits
		FROM units u JOIN boards b ON b.id = u.board_id WHERE b.world_id = $1::uuid`,
		[]any{id}, func(rows pgx.Rows) error {
			e := &game.Entity{}
			var kind, dialogue string
			if err := rows.Scan(&e.ID, &e.BoardID, &e.OwnerID, &kind, &e.Name, &e.Description,
				&e.X, &e.Y, &e.Speed, &e.Health, &e.MaxHealth, &e.Vision, &e.Strength, &dialogue, &e.RaceID, &e.Traits); err != nil {
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

	err = s.each(ctx, `SELECT i.id::text, i.board_id::text, i.name, i.description, i.x, i.y, i.effect, i.icon
		FROM board_items i JOIN boards b ON b.id = i.board_id WHERE b.world_id = $1::uuid`,
		[]any{id}, func(rows pgx.Rows) error {
			e := &game.Entity{Kind: game.KindItem}
			if err := rows.Scan(&e.ID, &e.BoardID, &e.Name, &e.Description, &e.X, &e.Y, &e.Effect, &e.Icon); err != nil {
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

	err = s.each(ctx, `SELECT p.id::text, p.username, m.points, COALESCE(m.race_id::text, ''), m.stats FROM memberships m
		JOIN players p ON p.id = m.player_id WHERE m.world_id = $1::uuid`, []any{id}, func(rows pgx.Rows) error {
		var pid, username, raceID string
		var points int
		var stats game.Stats
		if err := rows.Scan(&pid, &username, &points, &raceID, &stats); err != nil {
			return err
		}
		player := world.EnsurePlayer(pid, username, points)
		player.RaceID = raceID
		player.Stats = stats
		return nil
	})
	if err != nil {
		return nil, err
	}

	err = s.each(ctx, `SELECT player_id::text, id::text, name, description, effect, icon FROM inventory_items
		WHERE world_id = $1::uuid ORDER BY acquired_at`, []any{id},
		func(rows pgx.Rows) error {
			var pid string
			var item game.Item
			if err := rows.Scan(&pid, &item.ID, &item.Name, &item.Description, &item.Effect, &item.Icon); err != nil {
				return err
			}
			p := world.Player(pid)
			p.Inventory = append(p.Inventory, item)
			return nil
		})
	if err != nil {
		return nil, err
	}

	err = s.each(ctx, `SELECT id::text, scope, kind, target, reward, title, description, COALESCE(achieved_by::text, '')
		FROM goals WHERE world_id = $1::uuid ORDER BY position, title`, []any{id}, func(rows pgx.Rows) error {
		g := &game.Goal{}
		var scope, kind string
		if err := rows.Scan(&g.ID, &scope, &kind, &g.Target, &g.Reward, &g.Title, &g.Description, &g.AchievedBy); err != nil {
			return err
		}
		g.Scope, g.Kind = game.GoalScope(scope), game.GoalKind(kind)
		world.AddGoal(g)
		return nil
	})
	if err != nil {
		return nil, err
	}
	// The latest goal a player was given and has not completed is the one they are working on.
	err = s.each(ctx, `SELECT pg.player_id::text, pg.goal_id::text, pg.completed_at IS NOT NULL
		FROM player_goals pg JOIN goals g ON g.id = pg.goal_id WHERE g.world_id = $1::uuid ORDER BY pg.assigned_at`,
		[]any{id}, func(rows pgx.Rows) error {
			var playerID, goalID string
			var done bool
			if err := rows.Scan(&playerID, &goalID, &done); err != nil {
				return err
			}
			p := world.Player(playerID)
			if p == nil {
				return nil
			}
			if done {
				p.CompletedGoals[goalID] = true
			} else {
				p.GoalID = goalID
			}
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
				(id, board_id, player_id, kind, name, description, x, y, speed, health, max_health, vision, strength, race_id, traits)
				VALUES (COALESCE(NULLIF($1, '')::uuid, gen_random_uuid()), $2::uuid, NULLIF($3, '')::uuid, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
				        NULLIF($14, '')::uuid, $15)
				RETURNING id::text`,
				u.ID, u.BoardID, u.OwnerID, string(u.Kind), u.Name, u.Description, u.X, u.Y,
				u.Speed, u.Health, u.MaxHealth, u.Vision, u.Strength, u.RaceID, nonNil(u.Traits)).Scan(&u.ID)
			if err != nil {
				return err
			}
		}
		return nil
	})
}

// nonNil turns a missing map into an empty one, so it is stored as {} and not as null.
func nonNil(m map[string]int) map[string]int {
	if m == nil {
		return map[string]int{}
	}
	return m
}

func (s *Store) SaveUnit(ctx context.Context, e game.Entity) error {
	_, err := s.pool.Exec(ctx, `UPDATE units SET board_id = $2::uuid, x = $3, y = $4, health = $5, strength = $6, traits = $7
		WHERE id::text = $1`, e.ID, e.BoardID, e.X, e.Y, e.Health, e.Strength, nonNil(e.Traits))
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

func (s *Store) AddInventory(ctx context.Context, worldID, playerID string, item game.Item) error {
	_, err := s.pool.Exec(ctx, `INSERT INTO inventory_items (id, world_id, player_id, name, description, effect, icon)
		VALUES (COALESCE(NULLIF($1, '')::uuid, gen_random_uuid()), $2::uuid, $3::uuid, $4, $5, $6, $7)`,
		item.ID, worldID, playerID, item.Name, item.Description, item.Effect, item.Icon)
	return err
}

func (s *Store) DeleteInventoryItems(ctx context.Context, ids []string) error {
	_, err := s.pool.Exec(ctx, `DELETE FROM inventory_items WHERE id::text = ANY($1)`, ids)
	return err
}

func (s *Store) AddPoints(ctx context.Context, worldID, playerID string, delta int) error {
	_, err := s.pool.Exec(ctx, `UPDATE memberships SET points = points + $3
		WHERE world_id = $1::uuid AND player_id = $2::uuid`, worldID, playerID, delta)
	return err
}

func (s *Store) SaveStats(ctx context.Context, worldID, playerID string, stats game.Stats) error {
	_, err := s.pool.Exec(ctx, `UPDATE memberships SET stats = $3 WHERE world_id = $1::uuid AND player_id = $2::uuid`,
		worldID, playerID, stats)
	return err
}

func (s *Store) AssignGoal(ctx context.Context, playerID, goalID string) error {
	_, err := s.pool.Exec(ctx, `INSERT INTO player_goals (player_id, goal_id) VALUES ($1::uuid, $2::uuid) ON CONFLICT DO NOTHING`,
		playerID, goalID)
	return err
}

func (s *Store) CompleteGoal(ctx context.Context, playerID, goalID string) error {
	_, err := s.pool.Exec(ctx, `UPDATE player_goals SET completed_at = now()
		WHERE player_id = $1::uuid AND goal_id = $2::uuid AND completed_at IS NULL`, playerID, goalID)
	return err
}

func (s *Store) SetGoalAchieved(ctx context.Context, goalID, playerID string) error {
	_, err := s.pool.Exec(ctx, `UPDATE goals SET achieved_by = $2::uuid, achieved_at = now()
		WHERE id = $1::uuid AND achieved_by IS NULL`, goalID, playerID)
	return err
}

// WorldInfo is what the lobby shows about a world.
type WorldInfo struct {
	ID          string
	Name        string
	Description string
	OwnerID     string
	Boards      int
	Players     int
	Races       []RaceInfo
}

// RaceInfo is what a player sees of a race when choosing one to join with.
type RaceInfo struct {
	ID          string
	Name        string
	Description string
}

func (s *Store) WorldIDs(ctx context.Context) ([]string, error) {
	var ids []string
	err := s.each(ctx, `SELECT id::text FROM worlds ORDER BY created_at`, nil, func(rows pgx.Rows) error {
		var id string
		if err := rows.Scan(&id); err != nil {
			return err
		}
		ids = append(ids, id)
		return nil
	})
	return ids, err
}

func (s *Store) ListWorlds(ctx context.Context) ([]WorldInfo, error) {
	var out []WorldInfo
	err := s.each(ctx, `SELECT w.id::text, w.name, w.description, COALESCE(w.owner_id::text, ''),
		(SELECT count(*) FROM boards b WHERE b.world_id = w.id),
		(SELECT count(*) FROM memberships m WHERE m.world_id = w.id)
		FROM worlds w ORDER BY w.created_at`, nil, func(rows pgx.Rows) error {
		var w WorldInfo
		if err := rows.Scan(&w.ID, &w.Name, &w.Description, &w.OwnerID, &w.Boards, &w.Players); err != nil {
			return err
		}
		out = append(out, w)
		return nil
	})
	if err != nil {
		return nil, err
	}
	for i := range out {
		if out[i].Races, err = s.races(ctx, out[i].ID); err != nil {
			return nil, err
		}
	}
	return out, nil
}

func (s *Store) races(ctx context.Context, worldID string) ([]RaceInfo, error) {
	var out []RaceInfo
	err := s.each(ctx, `SELECT id::text, name, description FROM races WHERE world_id = $1::uuid ORDER BY position, name`,
		[]any{worldID}, func(rows pgx.Rows) error {
			var r RaceInfo
			if err := rows.Scan(&r.ID, &r.Name, &r.Description); err != nil {
				return err
			}
			out = append(out, r)
			return nil
		})
	return out, err
}

// ResolveRace checks the race a player picked when joining: it must belong to the world. With no
// pick, the world's first race is used (or none, if the world has no races).
func (s *Store) ResolveRace(ctx context.Context, worldID, raceID string) (string, error) {
	races, err := s.races(ctx, worldID)
	if err != nil {
		return "", err
	}
	if len(races) == 0 {
		return "", nil
	}
	if raceID == "" {
		return races[0].ID, nil
	}
	for _, r := range races {
		if r.ID == raceID {
			return raceID, nil
		}
	}
	return "", ErrNotFound
}

// World returns one world's lobby entry.
func (s *Store) World(ctx context.Context, id string) (WorldInfo, error) {
	var w WorldInfo
	err := s.pool.QueryRow(ctx, `SELECT w.id::text, w.name, w.description, COALESCE(w.owner_id::text, ''),
		(SELECT count(*) FROM boards b WHERE b.world_id = w.id),
		(SELECT count(*) FROM memberships m WHERE m.world_id = w.id)
		FROM worlds w WHERE w.id::text = $1`, id).Scan(&w.ID, &w.Name, &w.Description, &w.OwnerID, &w.Boards, &w.Players)
	if errors.Is(err, pgx.ErrNoRows) {
		return WorldInfo{}, ErrNotFound
	}
	return w, err
}

// Memberships lists the ids of the worlds a player has joined.
func (s *Store) Memberships(ctx context.Context, playerID string) (map[string]bool, error) {
	out := map[string]bool{}
	err := s.each(ctx, `SELECT world_id::text FROM memberships WHERE player_id::text = $1`, []any{playerID}, func(rows pgx.Rows) error {
		var id string
		if err := rows.Scan(&id); err != nil {
			return err
		}
		out[id] = true
		return nil
	})
	return out, err
}

func (s *Store) IsMember(ctx context.Context, worldID, playerID string) (bool, error) {
	var ok bool
	err := s.pool.QueryRow(ctx, `SELECT EXISTS (SELECT 1 FROM memberships WHERE world_id::text = $1 AND player_id::text = $2)`,
		worldID, playerID).Scan(&ok)
	return ok, err
}

// Join adds a player to a world with the race they chose; joining twice is harmless (the first race sticks).
func (s *Store) Join(ctx context.Context, worldID, playerID, raceID string) error {
	_, err := s.pool.Exec(ctx, `INSERT INTO memberships (player_id, world_id, race_id)
		VALUES ($2::uuid, $1::uuid, NULLIF($3, '')::uuid) ON CONFLICT DO NOTHING`, worldID, playerID, raceID)
	return err
}

// ClaimOwnerlessWorlds makes the player the admin of every world that has none yet (so the first
// player to register administers the seeded world).
func (s *Store) ClaimOwnerlessWorlds(ctx context.Context, playerID string) error {
	_, err := s.pool.Exec(ctx, `UPDATE worlds SET owner_id = $1::uuid WHERE owner_id IS NULL`, playerID)
	return err
}

// CreateWorld makes a new world with a single empty 24x24 board; its admin then shapes it.
func (s *Store) CreateWorld(ctx context.Context, name, description, ownerID string) (string, error) {
	var id string
	err := pgx.BeginFunc(ctx, s.pool, func(tx pgx.Tx) error {
		if err := tx.QueryRow(ctx, `INSERT INTO worlds (name, description, owner_id) VALUES ($1, $2, $3::uuid) RETURNING id::text`,
			name, description, ownerID).Scan(&id); err != nil {
			return err
		}
		_, err := tx.Exec(ctx, `INSERT INTO boards (world_id, name, width, height, grid_kind, position)
			VALUES ($1::uuid, 'Piazza', 24, 24, 'square', 0)`, id)
		return err
	})
	return id, err
}
