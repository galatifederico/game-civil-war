package store

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"strings"
	"unicode/utf8"

	"github.com/jackc/pgx/v5"

	"thegame/backend/internal/game"
)

// The admin app edits a world as a set of lists (boards, races, goals, NPCs...). Every list is
// saved as a whole: entries with an id are updated, entries without one are created and the
// ones left out are deleted. That keeps the API to one call per list and makes what the admin
// sees on screen exactly what ends up in the database.

// InvalidError is a change the admin asked for that the world cannot accept; its text is shown to them.
type InvalidError struct{ Message string }

func (e *InvalidError) Error() string { return e.Message }

func invalid(format string, args ...any) error {
	return &InvalidError{Message: fmt.Sprintf(format, args...)}
}

// Change is one edit of a world. Apply runs it in a transaction.
type Change func(ctx context.Context, tx pgx.Tx) error

// Apply runs a change in a transaction. With dryRun the transaction is rolled back at the end: it
// tells whether the change would be accepted, without touching the world.
func (s *Store) Apply(ctx context.Context, change Change, dryRun bool) error {
	tx, err := s.pool.Begin(ctx)
	if err != nil {
		return err
	}
	defer tx.Rollback(ctx)
	if err := change(ctx, tx); err != nil {
		return err
	}
	if dryRun {
		return nil
	}
	return tx.Commit(ctx)
}

const (
	maxBoards    = 32
	maxBoardSide = 64
	maxRaces     = 16
	maxGoals     = 50
	maxEntities  = 300
	maxLinks     = 300
)

type AdminBoard struct {
	ID     string `json:"id"`
	Name   string `json:"name"`
	Width  int    `json:"width"`
	Height int    `json:"height"`
	Grid   string `json:"grid"`
}

type AdminLink struct {
	FromBoard string `json:"from_board"`
	FromX     int    `json:"from_x"`
	FromY     int    `json:"from_y"`
	ToBoard   string `json:"to_board"`
	ToX       int    `json:"to_x"`
	ToY       int    `json:"to_y"`
}

type AdminRace struct {
	ID            string         `json:"id"`
	Name          string         `json:"name"`
	Description   string         `json:"description"`
	Speed         int            `json:"speed"`
	Health        int            `json:"health"`
	Vision        int            `json:"vision"`
	Strength      int            `json:"strength"`
	BonusSpeed    int            `json:"bonus_speed"`
	BonusHealth   int            `json:"bonus_health"`
	BonusVision   int            `json:"bonus_vision"`
	BonusStrength int            `json:"bonus_strength"`
	TraitsMin     map[string]int `json:"traits_min"`
	TraitsBonus   map[string]int `json:"traits_bonus"`
}

type AdminCompat struct {
	RaceA string `json:"race_a"`
	RaceB string `json:"race_b"`
	Child string `json:"child_race"`
}

type AdminGoal struct {
	ID          string `json:"id"`
	Scope       string `json:"scope"`
	Kind        string `json:"kind"`
	Target      int    `json:"target"`
	Reward      int    `json:"reward"`
	Title       string `json:"title"`
	Description string `json:"description"`
	AchievedBy  string `json:"achieved_by"` // read-only
}

type AdminNPC struct {
	ID          string         `json:"id"`
	BoardID     string         `json:"board_id"`
	Name        string         `json:"name"`
	Description string         `json:"description"`
	X           int            `json:"x"`
	Y           int            `json:"y"`
	Speed       int            `json:"speed"`
	Health      int            `json:"health"`
	Vision      int            `json:"vision"`
	Strength    int            `json:"strength"`
	Dialogue    string         `json:"dialogue"` // one line per reply
	RaceID      string         `json:"race_id"`
	Traits      map[string]int `json:"traits"`
}

type AdminItem struct {
	ID          string      `json:"id"`
	BoardID     string      `json:"board_id"`
	Name        string      `json:"name"`
	Description string      `json:"description"`
	X           int         `json:"x"`
	Y           int         `json:"y"`
	Effect      game.Effect `json:"effect"`
	Icon        string      `json:"icon"` // "" = automatic
}

// AdminTerrain is the terrain of one board: one string per row (top first), one glyph per cell
// (game.TerrainKinds). Every board of the world is listed, plain grass included.
type AdminTerrain struct {
	BoardID string   `json:"board_id"`
	Rows    []string `json:"rows"`
}

type AdminPlayer struct {
	ID            string `json:"id"`
	Username      string `json:"username"`
	Race          string `json:"race"`
	Points        int    `json:"points"`
	Units         int    `json:"units"`
	Kills         int    `json:"kills"`
	ChampionKills int    `json:"champion_kills"`
	Pickups       int    `json:"pickups"`
	Talks         int    `json:"talks"`
	GoalID        string `json:"goal_id"`
	Completed     int    `json:"completed_goals"`
}

// WorldDefinition is everything the admin can edit about a world, plus who is playing in it.
type WorldDefinition struct {
	ID          string          `json:"id"`
	Name        string          `json:"name"`
	Description string          `json:"description"`
	Rules       json.RawMessage `json:"rules"` // only what differs from the defaults
	Boards      []AdminBoard    `json:"boards"`
	Links       []AdminLink     `json:"links"`
	Terrain     []AdminTerrain  `json:"terrain"`
	Races       []AdminRace     `json:"races"`
	Compat      []AdminCompat   `json:"compat"`
	Goals       []AdminGoal     `json:"goals"`
	NPCs        []AdminNPC      `json:"npcs"`
	Items       []AdminItem     `json:"items"`
	Players     []AdminPlayer   `json:"players"`
}

// Definition reads a world for the admin app.
func (s *Store) Definition(ctx context.Context, id string) (WorldDefinition, error) {
	d := WorldDefinition{
		ID: id, Boards: []AdminBoard{}, Links: []AdminLink{}, Terrain: []AdminTerrain{}, Races: []AdminRace{}, Compat: []AdminCompat{},
		Goals: []AdminGoal{}, NPCs: []AdminNPC{}, Items: []AdminItem{}, Players: []AdminPlayer{},
	}
	err := s.pool.QueryRow(ctx, `SELECT name, description, rules FROM worlds WHERE id = $1::uuid`, id).
		Scan(&d.Name, &d.Description, &d.Rules)
	if errors.Is(err, pgx.ErrNoRows) {
		return d, ErrNotFound
	}
	if err != nil {
		return d, err
	}

	if err := s.each(ctx, `SELECT id::text, name, width, height, grid_kind FROM boards WHERE world_id = $1::uuid
		ORDER BY position, created_at, name`, []any{id}, func(rows pgx.Rows) error {
		var b AdminBoard
		if err := rows.Scan(&b.ID, &b.Name, &b.Width, &b.Height, &b.Grid); err != nil {
			return err
		}
		d.Boards = append(d.Boards, b)
		return nil
	}); err != nil {
		return d, err
	}
	{
		cells := map[string]map[game.Point]byte{}
		if err := s.each(ctx, `SELECT t.board_id::text, t.x, t.y, t.tile FROM board_terrain t
			JOIN boards b ON b.id = t.board_id WHERE b.world_id = $1::uuid`, []any{id}, func(rows pgx.Rows) error {
			var boardID, tile string
			var p game.Point
			if err := rows.Scan(&boardID, &p.X, &p.Y, &tile); err != nil {
				return err
			}
			if glyph, ok := game.TerrainGlyph(tile); ok {
				if cells[boardID] == nil {
					cells[boardID] = map[game.Point]byte{}
				}
				cells[boardID][p] = glyph
			}
			return nil
		}); err != nil {
			return d, err
		}
		for _, b := range d.Boards {
			rows := make([]string, b.Height)
			for y := range rows {
				row := make([]byte, b.Width)
				for x := range row {
					row[x] = '.'
					if g, ok := cells[b.ID][game.Point{X: x, Y: y}]; ok {
						row[x] = g
					}
				}
				rows[y] = string(row)
			}
			d.Terrain = append(d.Terrain, AdminTerrain{BoardID: b.ID, Rows: rows})
		}
	}
	if err := s.each(ctx, `SELECT l.from_board_id::text, l.from_x, l.from_y, l.to_board_id::text, l.to_x, l.to_y
		FROM board_links l JOIN boards b ON b.id = l.from_board_id WHERE b.world_id = $1::uuid
		ORDER BY b.position, l.from_x, l.from_y`, []any{id}, func(rows pgx.Rows) error {
		var l AdminLink
		if err := rows.Scan(&l.FromBoard, &l.FromX, &l.FromY, &l.ToBoard, &l.ToX, &l.ToY); err != nil {
			return err
		}
		d.Links = append(d.Links, l)
		return nil
	}); err != nil {
		return d, err
	}
	if err := s.each(ctx, `SELECT id::text, name, description, speed, health, vision, strength,
		bonus_speed, bonus_health, bonus_vision, bonus_strength, traits_min, traits_bonus
		FROM races WHERE world_id = $1::uuid ORDER BY position, name`, []any{id}, func(rows pgx.Rows) error {
		var r AdminRace
		if err := rows.Scan(&r.ID, &r.Name, &r.Description, &r.Speed, &r.Health, &r.Vision, &r.Strength,
			&r.BonusSpeed, &r.BonusHealth, &r.BonusVision, &r.BonusStrength, &r.TraitsMin, &r.TraitsBonus); err != nil {
			return err
		}
		r.TraitsMin, r.TraitsBonus = nonNil(r.TraitsMin), nonNil(r.TraitsBonus)
		d.Races = append(d.Races, r)
		return nil
	}); err != nil {
		return d, err
	}
	if err := s.each(ctx, `SELECT race_a::text, race_b::text, child_race::text FROM race_compatibility
		WHERE world_id = $1::uuid`, []any{id}, func(rows pgx.Rows) error {
		var c AdminCompat
		if err := rows.Scan(&c.RaceA, &c.RaceB, &c.Child); err != nil {
			return err
		}
		d.Compat = append(d.Compat, c)
		return nil
	}); err != nil {
		return d, err
	}
	if err := s.each(ctx, `SELECT id::text, scope, kind, target, reward, title, description, COALESCE(achieved_by::text, '')
		FROM goals WHERE world_id = $1::uuid ORDER BY position, title`, []any{id}, func(rows pgx.Rows) error {
		var g AdminGoal
		if err := rows.Scan(&g.ID, &g.Scope, &g.Kind, &g.Target, &g.Reward, &g.Title, &g.Description, &g.AchievedBy); err != nil {
			return err
		}
		d.Goals = append(d.Goals, g)
		return nil
	}); err != nil {
		return d, err
	}
	if err := s.each(ctx, `SELECT u.id::text, u.board_id::text, u.name, u.description, u.x, u.y, u.speed, u.max_health,
		u.vision, u.strength, u.dialogue, COALESCE(u.race_id::text, ''), u.traits
		FROM units u JOIN boards b ON b.id = u.board_id
		WHERE b.world_id = $1::uuid AND u.kind = 'npc' ORDER BY b.position, u.name, u.created_at`, []any{id}, func(rows pgx.Rows) error {
		var n AdminNPC
		if err := rows.Scan(&n.ID, &n.BoardID, &n.Name, &n.Description, &n.X, &n.Y, &n.Speed, &n.Health,
			&n.Vision, &n.Strength, &n.Dialogue, &n.RaceID, &n.Traits); err != nil {
			return err
		}
		n.Traits = nonNil(n.Traits)
		d.NPCs = append(d.NPCs, n)
		return nil
	}); err != nil {
		return d, err
	}
	if err := s.each(ctx, `SELECT i.id::text, i.board_id::text, i.name, i.description, i.x, i.y, i.effect, i.icon
		FROM board_items i JOIN boards b ON b.id = i.board_id WHERE b.world_id = $1::uuid
		ORDER BY b.position, i.name, i.created_at`, []any{id}, func(rows pgx.Rows) error {
		var it AdminItem
		if err := rows.Scan(&it.ID, &it.BoardID, &it.Name, &it.Description, &it.X, &it.Y, &it.Effect, &it.Icon); err != nil {
			return err
		}
		d.Items = append(d.Items, it)
		return nil
	}); err != nil {
		return d, err
	}
	err = s.each(ctx, `SELECT p.id::text, p.username, COALESCE(r.name, ''), m.points, m.stats,
		(SELECT count(*) FROM units u JOIN boards b ON b.id = u.board_id WHERE u.player_id = p.id AND b.world_id = m.world_id),
		COALESCE((SELECT pg.goal_id::text FROM player_goals pg JOIN goals g ON g.id = pg.goal_id
			WHERE pg.player_id = p.id AND g.world_id = m.world_id AND pg.completed_at IS NULL
			ORDER BY pg.assigned_at DESC LIMIT 1), ''),
		(SELECT count(*) FROM player_goals pg JOIN goals g ON g.id = pg.goal_id
			WHERE pg.player_id = p.id AND g.world_id = m.world_id AND pg.completed_at IS NOT NULL)
		FROM memberships m JOIN players p ON p.id = m.player_id LEFT JOIN races r ON r.id = m.race_id
		WHERE m.world_id = $1::uuid ORDER BY m.points DESC, p.username`, []any{id}, func(rows pgx.Rows) error {
		var p AdminPlayer
		var stats game.Stats
		if err := rows.Scan(&p.ID, &p.Username, &p.Race, &p.Points, &stats, &p.Units, &p.GoalID, &p.Completed); err != nil {
			return err
		}
		p.Kills, p.ChampionKills, p.Pickups, p.Talks = stats.Kills, stats.ChampionKills, stats.Pickups, len(stats.Talked)
		d.Players = append(d.Players, p)
		return nil
	})
	return d, err
}

// --- helpers shared by the changes ---

func checkName(what, name string, max int) (string, error) {
	name = strings.TrimSpace(name)
	if n := utf8.RuneCountInString(name); n < 1 || n > max {
		return "", invalid("%s: il nome deve avere da 1 a %d caratteri", what, max)
	}
	return name, nil
}

func checkText(what, text string, max int) error {
	if utf8.RuneCountInString(text) > max {
		return invalid("%s: testo troppo lungo (massimo %d caratteri)", what, max)
	}
	return nil
}

// checkTraits accepts only the characteristics the world defines and non-negative values.
func checkTraits(what string, traits map[string]int, names map[string]bool) error {
	for name, v := range traits {
		if !names[name] {
			return invalid("%s: caratteristica sconosciuta %q (quelle del mondo sono nelle regole: trait_names)", what, name)
		}
		if v < 0 {
			return invalid("%s: la caratteristica %q non può essere negativa", what, name)
		}
	}
	return nil
}

func traitNames(ctx context.Context, tx pgx.Tx, worldID string) (map[string]bool, error) {
	var raw []byte
	if err := tx.QueryRow(ctx, `SELECT rules FROM worlds WHERE id = $1::uuid`, worldID).Scan(&raw); err != nil {
		return nil, err
	}
	rules, err := game.ParseRules(raw)
	if err != nil {
		return nil, err
	}
	names := map[string]bool{}
	for _, n := range rules.TraitNames {
		names[n] = true
	}
	return names, nil
}

type boardDims struct{ w, h int }

func worldBoards(ctx context.Context, tx pgx.Tx, worldID string) (map[string]boardDims, error) {
	rows, err := tx.Query(ctx, `SELECT id::text, width, height FROM boards WHERE world_id = $1::uuid`, worldID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	out := map[string]boardDims{}
	for rows.Next() {
		var id string
		var d boardDims
		if err := rows.Scan(&id, &d.w, &d.h); err != nil {
			return nil, err
		}
		out[id] = d
	}
	return out, rows.Err()
}

func (d boardDims) contains(x, y int) bool { return x >= 0 && y >= 0 && x < d.w && y < d.h }

func existingIDs(ctx context.Context, tx pgx.Tx, query, worldID string) (map[string]bool, error) {
	rows, err := tx.Query(ctx, query, worldID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	out := map[string]bool{}
	for rows.Next() {
		var id string
		if err := rows.Scan(&id); err != nil {
			return nil, err
		}
		out[id] = true
	}
	return out, rows.Err()
}

func count(ctx context.Context, tx pgx.Tx, query string, args ...any) (int, error) {
	var n int
	err := tx.QueryRow(ctx, query, args...).Scan(&n)
	return n, err
}

// noTwoOnACell rejects a world where two things (unit, item, structure) share a cell.
func noTwoOnACell(ctx context.Context, tx pgx.Tx, worldID string) error {
	var name string
	var x, y int
	err := tx.QueryRow(ctx, `SELECT b.name, t.x, t.y FROM (
			SELECT board_id, x, y FROM units UNION ALL SELECT board_id, x, y FROM board_items
			UNION ALL SELECT board_id, x, y FROM structures) t
		JOIN boards b ON b.id = t.board_id WHERE b.world_id = $1::uuid
		GROUP BY b.name, t.board_id, t.x, t.y HAVING count(*) > 1 LIMIT 1`, worldID).Scan(&name, &x, &y)
	if errors.Is(err, pgx.ErrNoRows) {
		return nil
	}
	if err != nil {
		return err
	}
	return invalid("due elementi occupano la stessa casella (%d, %d) della board %q", x, y, name)
}

// --- the changes ---

// MetaChange renames the world and changes its description.
func MetaChange(worldID, name, description string) Change {
	return func(ctx context.Context, tx pgx.Tx) error {
		name, err := checkName("mondo", name, 40)
		if err != nil {
			return err
		}
		if err := checkText("descrizione", description, 500); err != nil {
			return err
		}
		_, err = tx.Exec(ctx, `UPDATE worlds SET name = $2, description = $3 WHERE id = $1::uuid`,
			worldID, name, strings.TrimSpace(description))
		return err
	}
}

// RulesChange stores the rules. It keeps only what differs from the defaults, so a later change
// of a default reaches every world that never set that value.
func RulesChange(worldID string, raw []byte) Change {
	return func(ctx context.Context, tx pgx.Tx) error {
		rules, err := game.ParseRules(raw)
		if err != nil {
			return invalid("%v", err)
		}
		_, err = tx.Exec(ctx, `UPDATE worlds SET rules = $2::jsonb WHERE id = $1::uuid`, worldID, rules.Diff())
		return err
	}
}

// BoardsChange replaces the world's boards. Deleting a board deletes its NPCs, items and passages,
// but not while players have units or structures on it.
func BoardsChange(worldID string, boards []AdminBoard) Change {
	return func(ctx context.Context, tx pgx.Tx) error {
		if len(boards) == 0 || len(boards) > maxBoards {
			return invalid("un mondo ha da 1 a %d board", maxBoards)
		}
		existing, err := existingIDs(ctx, tx, `SELECT id::text FROM boards WHERE world_id = $1::uuid`, worldID)
		if err != nil {
			return err
		}
		names := map[string]bool{}
		kept := map[string]bool{}
		for i, b := range boards {
			name, err := checkName("board", b.Name, 30)
			if err != nil {
				return err
			}
			if names[strings.ToLower(name)] {
				return invalid("due board si chiamano %q", name)
			}
			names[strings.ToLower(name)] = true
			if b.Width < 1 || b.Height < 1 || b.Width > maxBoardSide || b.Height > maxBoardSide {
				return invalid("board %q: le dimensioni devono essere da 1 a %d", name, maxBoardSide)
			}
			if _, err := game.NewGrid(b.Grid); err != nil {
				return invalid("board %q: griglia %q sconosciuta (esiste solo square)", name, b.Grid)
			}
			if b.ID == "" {
				_, err = tx.Exec(ctx, `INSERT INTO boards (world_id, name, width, height, grid_kind, position)
					VALUES ($1::uuid, $2, $3, $4, $5, $6)`, worldID, name, b.Width, b.Height, b.Grid, i)
			} else {
				if !existing[b.ID] {
					return invalid("board sconosciuta: %s", b.ID)
				}
				kept[b.ID] = true
				_, err = tx.Exec(ctx, `UPDATE boards SET name = $2, width = $3, height = $4, grid_kind = $5, position = $6
					WHERE id = $1::uuid`, b.ID, name, b.Width, b.Height, b.Grid, i)
			}
			if err != nil {
				return err
			}
		}

		for id := range existing {
			if kept[id] {
				continue
			}
			n, err := count(ctx, tx, `SELECT (SELECT count(*) FROM units WHERE board_id = $1::uuid AND player_id IS NOT NULL)
				+ (SELECT count(*) FROM structures WHERE board_id = $1::uuid)`, id)
			if err != nil {
				return err
			}
			if n > 0 {
				return invalid("una board da eliminare ha pedine o strutture dei giocatori: non si può cancellare")
			}
			for _, q := range []string{
				`DELETE FROM board_links WHERE from_board_id = $1::uuid OR to_board_id = $1::uuid`,
				`DELETE FROM board_items WHERE board_id = $1::uuid`,
				`DELETE FROM units WHERE board_id = $1::uuid`,
				`DELETE FROM boards WHERE id = $1::uuid`,
			} {
				if _, err := tx.Exec(ctx, q, id); err != nil {
					return err
				}
			}
		}

		// Terrain outside a board that got smaller is simply dropped.
		if _, err := tx.Exec(ctx, `DELETE FROM board_terrain t USING boards b WHERE b.id = t.board_id
			AND b.world_id = $1::uuid AND (t.x >= b.width OR t.y >= b.height)`, worldID); err != nil {
			return err
		}
		// Nothing may end up outside a board that got smaller.
		var name string
		err = tx.QueryRow(ctx, `SELECT b.name FROM boards b WHERE b.world_id = $1::uuid AND (
			EXISTS (SELECT 1 FROM units u WHERE u.board_id = b.id AND (u.x >= b.width OR u.y >= b.height))
			OR EXISTS (SELECT 1 FROM board_items i WHERE i.board_id = b.id AND (i.x >= b.width OR i.y >= b.height))
			OR EXISTS (SELECT 1 FROM structures s WHERE s.board_id = b.id AND (s.x >= b.width OR s.y >= b.height))
			OR EXISTS (SELECT 1 FROM board_links l WHERE (l.from_board_id = b.id AND (l.from_x >= b.width OR l.from_y >= b.height))
				OR (l.to_board_id = b.id AND (l.to_x >= b.width OR l.to_y >= b.height)))) LIMIT 1`, worldID).Scan(&name)
		if err == nil {
			return invalid("la board %q è più piccola di dove si trovano alcuni elementi (pedine, oggetti, strutture o passaggi)", name)
		}
		if !errors.Is(err, pgx.ErrNoRows) {
			return err
		}
		return nil
	}
}

// LinksChange replaces the passages between boards.
func LinksChange(worldID string, links []AdminLink) Change {
	return func(ctx context.Context, tx pgx.Tx) error {
		if len(links) > maxLinks {
			return invalid("troppi passaggi (massimo %d)", maxLinks)
		}
		dims, err := worldBoards(ctx, tx, worldID)
		if err != nil {
			return err
		}
		seen := map[[3]any]bool{}
		for _, l := range links {
			from, ok1 := dims[l.FromBoard]
			to, ok2 := dims[l.ToBoard]
			if !ok1 || !ok2 {
				return invalid("un passaggio usa una board che non esiste")
			}
			if !from.contains(l.FromX, l.FromY) || !to.contains(l.ToX, l.ToY) {
				return invalid("un passaggio ha una casella fuori dalla board")
			}
			if l.FromBoard == l.ToBoard && l.FromX == l.ToX && l.FromY == l.ToY {
				return invalid("un passaggio non può portare sulla stessa casella")
			}
			key := [3]any{l.FromBoard, l.FromX, l.FromY}
			if seen[key] {
				return invalid("due passaggi partono dalla stessa casella (%d, %d)", l.FromX, l.FromY)
			}
			seen[key] = true
		}
		if _, err := tx.Exec(ctx, `DELETE FROM board_links WHERE from_board_id IN (SELECT id FROM boards WHERE world_id = $1::uuid)`, worldID); err != nil {
			return err
		}
		for _, l := range links {
			if _, err := tx.Exec(ctx, `INSERT INTO board_links (from_board_id, from_x, from_y, to_board_id, to_x, to_y)
				VALUES ($1::uuid, $2, $3, $4::uuid, $5, $6)`, l.FromBoard, l.FromX, l.FromY, l.ToBoard, l.ToX, l.ToY); err != nil {
				return err
			}
		}
		return noBlockingTerrainOnThings(ctx, tx, worldID)
	}
}

// RacesChange replaces the races. A race that units or players use cannot be deleted.
func RacesChange(worldID string, races []AdminRace) Change {
	return func(ctx context.Context, tx pgx.Tx) error {
		if len(races) > maxRaces {
			return invalid("troppe razze (massimo %d)", maxRaces)
		}
		traits, err := traitNames(ctx, tx, worldID)
		if err != nil {
			return err
		}
		existing, err := existingIDs(ctx, tx, `SELECT id::text FROM races WHERE world_id = $1::uuid`, worldID)
		if err != nil {
			return err
		}
		names, kept := map[string]bool{}, map[string]bool{}
		for i, r := range races {
			name, err := checkName("razza", r.Name, 30)
			if err != nil {
				return err
			}
			if names[strings.ToLower(name)] {
				return invalid("due razze si chiamano %q", name)
			}
			names[strings.ToLower(name)] = true
			if err := checkText("razza "+name, r.Description, 300); err != nil {
				return err
			}
			if r.Speed < 1 || r.Health < 1 || r.Vision < 1 || r.Strength < 0 {
				return invalid("razza %q: velocità, vita e vista devono essere almeno 1, la forza non può essere negativa", name)
			}
			if r.BonusSpeed < 0 || r.BonusHealth < 0 || r.BonusVision < 0 || r.BonusStrength < 0 {
				return invalid("razza %q: i bonus non possono essere negativi", name)
			}
			if err := checkTraits("razza "+name, r.TraitsMin, traits); err != nil {
				return err
			}
			if err := checkTraits("razza "+name, r.TraitsBonus, traits); err != nil {
				return err
			}
			tmin, tbonus := nonNil(r.TraitsMin), nonNil(r.TraitsBonus)
			if r.ID == "" {
				_, err = tx.Exec(ctx, `INSERT INTO races (world_id, name, description, position, speed, health, vision, strength,
					bonus_speed, bonus_health, bonus_vision, bonus_strength, traits_min, traits_bonus)
					VALUES ($1::uuid, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)`,
					worldID, name, r.Description, i, r.Speed, r.Health, r.Vision, r.Strength,
					r.BonusSpeed, r.BonusHealth, r.BonusVision, r.BonusStrength, tmin, tbonus)
			} else {
				if !existing[r.ID] {
					return invalid("razza sconosciuta: %s", r.ID)
				}
				kept[r.ID] = true
				_, err = tx.Exec(ctx, `UPDATE races SET name = $2, description = $3, position = $4, speed = $5, health = $6,
					vision = $7, strength = $8, bonus_speed = $9, bonus_health = $10, bonus_vision = $11, bonus_strength = $12,
					traits_min = $13, traits_bonus = $14 WHERE id = $1::uuid`,
					r.ID, name, r.Description, i, r.Speed, r.Health, r.Vision, r.Strength,
					r.BonusSpeed, r.BonusHealth, r.BonusVision, r.BonusStrength, tmin, tbonus)
			}
			if err != nil {
				return err
			}
		}
		for id := range existing {
			if kept[id] {
				continue
			}
			n, err := count(ctx, tx, `SELECT (SELECT count(*) FROM units WHERE race_id = $1::uuid)
				+ (SELECT count(*) FROM memberships WHERE race_id = $1::uuid)`, id)
			if err != nil {
				return err
			}
			if n > 0 {
				return invalid("una razza da eliminare è usata da pedine o giocatori: non si può cancellare")
			}
			if _, err := tx.Exec(ctx, `DELETE FROM race_compatibility WHERE race_a = $1::uuid OR race_b = $1::uuid OR child_race = $1::uuid`, id); err != nil {
				return err
			}
			if _, err := tx.Exec(ctx, `DELETE FROM races WHERE id = $1::uuid`, id); err != nil {
				return err
			}
		}
		return nil
	}
}

// CompatChange replaces which pairs of races can have children, and of what race.
func CompatChange(worldID string, pairs []AdminCompat) Change {
	return func(ctx context.Context, tx pgx.Tx) error {
		races, err := existingIDs(ctx, tx, `SELECT id::text FROM races WHERE world_id = $1::uuid`, worldID)
		if err != nil {
			return err
		}
		seen := map[string]bool{}
		for _, c := range pairs {
			if !races[c.RaceA] || !races[c.RaceB] || !races[c.Child] {
				return invalid("una compatibilità usa una razza che non esiste")
			}
			a, b := c.RaceA, c.RaceB
			if a > b {
				a, b = b, a
			}
			if seen[a+"|"+b] {
				return invalid("la stessa coppia di razze compare due volte")
			}
			seen[a+"|"+b] = true
		}
		if _, err := tx.Exec(ctx, `DELETE FROM race_compatibility WHERE world_id = $1::uuid`, worldID); err != nil {
			return err
		}
		for _, c := range pairs {
			if _, err := tx.Exec(ctx, `INSERT INTO race_compatibility (world_id, race_a, race_b, child_race)
				VALUES ($1::uuid, $2::uuid, $3::uuid, $4::uuid)`, worldID, c.RaceA, c.RaceB, c.Child); err != nil {
				return err
			}
		}
		return nil
	}
}

// GoalsChange replaces the goals. Deleting a goal also takes it away from the players who had it.
func GoalsChange(worldID string, goals []AdminGoal) Change {
	return func(ctx context.Context, tx pgx.Tx) error {
		if len(goals) > maxGoals {
			return invalid("troppi obiettivi (massimo %d)", maxGoals)
		}
		existing, err := existingIDs(ctx, tx, `SELECT id::text FROM goals WHERE world_id = $1::uuid`, worldID)
		if err != nil {
			return err
		}
		kept := map[string]bool{}
		for i, g := range goals {
			title, err := checkName("obiettivo", g.Title, 60)
			if err != nil {
				return err
			}
			if err := checkText("obiettivo "+title, g.Description, 300); err != nil {
				return err
			}
			if game.GoalScope(g.Scope) != game.GoalWorld && game.GoalScope(g.Scope) != game.GoalIndividual {
				return invalid("obiettivo %q: ambito %q sconosciuto (world o individual)", title, g.Scope)
			}
			if !game.GoalKind(g.Kind).Valid() {
				return invalid("obiettivo %q: tipo %q sconosciuto", title, g.Kind)
			}
			if g.Target < 1 || g.Reward < 0 {
				return invalid("obiettivo %q: il traguardo deve essere almeno 1 e il premio non può essere negativo", title)
			}
			if g.ID == "" {
				_, err = tx.Exec(ctx, `INSERT INTO goals (world_id, scope, kind, target, reward, title, description, position)
					VALUES ($1::uuid, $2, $3, $4, $5, $6, $7, $8)`, worldID, g.Scope, g.Kind, g.Target, g.Reward, title, g.Description, i)
			} else {
				if !existing[g.ID] {
					return invalid("obiettivo sconosciuto: %s", g.ID)
				}
				kept[g.ID] = true
				_, err = tx.Exec(ctx, `UPDATE goals SET scope = $2, kind = $3, target = $4, reward = $5, title = $6,
					description = $7, position = $8 WHERE id = $1::uuid`, g.ID, g.Scope, g.Kind, g.Target, g.Reward, title, g.Description, i)
			}
			if err != nil {
				return err
			}
		}
		for id := range existing {
			if kept[id] {
				continue
			}
			if _, err := tx.Exec(ctx, `DELETE FROM player_goals WHERE goal_id = $1::uuid`, id); err != nil {
				return err
			}
			if _, err := tx.Exec(ctx, `DELETE FROM goals WHERE id = $1::uuid`, id); err != nil {
				return err
			}
		}
		return nil
	}
}

// NPCsChange replaces the world's NPCs (the units nobody owns).
func NPCsChange(worldID string, npcs []AdminNPC) Change {
	return func(ctx context.Context, tx pgx.Tx) error {
		if len(npcs) > maxEntities {
			return invalid("troppi NPC (massimo %d)", maxEntities)
		}
		dims, err := worldBoards(ctx, tx, worldID)
		if err != nil {
			return err
		}
		traits, err := traitNames(ctx, tx, worldID)
		if err != nil {
			return err
		}
		races, err := existingIDs(ctx, tx, `SELECT id::text FROM races WHERE world_id = $1::uuid`, worldID)
		if err != nil {
			return err
		}
		existing, err := existingIDs(ctx, tx, `SELECT u.id::text FROM units u JOIN boards b ON b.id = u.board_id
			WHERE b.world_id = $1::uuid AND u.kind = 'npc'`, worldID)
		if err != nil {
			return err
		}
		kept := map[string]bool{}
		// Positions are unique per board: free them all first so NPCs can swap places.
		for id := range existing {
			if _, err := tx.Exec(ctx, `UPDATE units SET x = -1 - x, y = -1 - y WHERE id = $1::uuid`, id); err != nil {
				return err
			}
		}
		for _, n := range npcs {
			name, err := checkName("NPC", n.Name, 40)
			if err != nil {
				return err
			}
			if err := checkText("NPC "+name, n.Description, 300); err != nil {
				return err
			}
			if err := checkText("NPC "+name, n.Dialogue, 1000); err != nil {
				return err
			}
			d, ok := dims[n.BoardID]
			if !ok {
				return invalid("NPC %q: board inesistente", name)
			}
			if !d.contains(n.X, n.Y) {
				return invalid("NPC %q: la casella (%d, %d) è fuori dalla board", name, n.X, n.Y)
			}
			if n.Speed < 0 || n.Health < 1 || n.Vision < 0 || n.Strength < 0 {
				return invalid("NPC %q: vita almeno 1, gli altri valori non negativi", name)
			}
			if n.RaceID != "" && !races[n.RaceID] {
				return invalid("NPC %q: razza inesistente", name)
			}
			if err := checkTraits("NPC "+name, n.Traits, traits); err != nil {
				return err
			}
			if n.ID == "" {
				_, err = tx.Exec(ctx, `INSERT INTO units (board_id, kind, name, description, x, y, speed, health, max_health,
					vision, strength, dialogue, race_id, traits)
					VALUES ($1::uuid, 'npc', $2, $3, $4, $5, $6, $7, $7, $8, $9, $10, NULLIF($11, '')::uuid, $12)`,
					n.BoardID, name, n.Description, n.X, n.Y, n.Speed, n.Health, n.Vision, n.Strength, n.Dialogue, n.RaceID, nonNil(n.Traits))
			} else {
				if !existing[n.ID] {
					return invalid("NPC sconosciuto: %s", n.ID)
				}
				kept[n.ID] = true
				_, err = tx.Exec(ctx, `UPDATE units SET board_id = $2::uuid, name = $3, description = $4, x = $5, y = $6, speed = $7,
					health = $8, max_health = $8, vision = $9, strength = $10, dialogue = $11, race_id = NULLIF($12, '')::uuid, traits = $13
					WHERE id = $1::uuid`,
					n.ID, n.BoardID, name, n.Description, n.X, n.Y, n.Speed, n.Health, n.Vision, n.Strength, n.Dialogue, n.RaceID, nonNil(n.Traits))
			}
			if err != nil {
				return invalidIfUnique(err, "NPC %q: la casella (%d, %d) è già occupata", name, n.X, n.Y)
			}
		}
		for id := range existing {
			if !kept[id] {
				if _, err := tx.Exec(ctx, `DELETE FROM units WHERE id = $1::uuid`, id); err != nil {
					return err
				}
			}
		}
		if err := noTwoOnACell(ctx, tx, worldID); err != nil {
			return err
		}
		return noBlockingTerrainOnThings(ctx, tx, worldID)
	}
}

// ItemsChange replaces the items lying on the boards (the ones in team inventories are not touched).
func ItemsChange(worldID string, items []AdminItem) Change {
	return func(ctx context.Context, tx pgx.Tx) error {
		if len(items) > maxEntities {
			return invalid("troppi oggetti (massimo %d)", maxEntities)
		}
		dims, err := worldBoards(ctx, tx, worldID)
		if err != nil {
			return err
		}
		traits, err := traitNames(ctx, tx, worldID)
		if err != nil {
			return err
		}
		existing, err := existingIDs(ctx, tx, `SELECT i.id::text FROM board_items i JOIN boards b ON b.id = i.board_id
			WHERE b.world_id = $1::uuid`, worldID)
		if err != nil {
			return err
		}
		kept := map[string]bool{}
		for id := range existing {
			if _, err := tx.Exec(ctx, `UPDATE board_items SET x = -1 - x, y = -1 - y WHERE id = $1::uuid`, id); err != nil {
				return err
			}
		}
		for _, it := range items {
			name, err := checkName("oggetto", it.Name, 40)
			if err != nil {
				return err
			}
			if err := checkText("oggetto "+name, it.Description, 300); err != nil {
				return err
			}
			d, ok := dims[it.BoardID]
			if !ok {
				return invalid("oggetto %q: board inesistente", name)
			}
			if !d.contains(it.X, it.Y) {
				return invalid("oggetto %q: la casella (%d, %d) è fuori dalla board", name, it.X, it.Y)
			}
			if !game.ValidIcon(it.Icon) {
				return invalid("oggetto %q: icona %q sconosciuta", name, it.Icon)
			}
			if it.Effect.Heal < 0 {
				return invalid("oggetto %q: la cura non può essere negativa", name)
			}
			for trait := range it.Effect.Traits {
				if !traits[trait] {
					return invalid("oggetto %q: caratteristica sconosciuta %q", name, trait)
				}
			}
			if it.ID == "" {
				_, err = tx.Exec(ctx, `INSERT INTO board_items (board_id, name, description, x, y, effect, icon)
					VALUES ($1::uuid, $2, $3, $4, $5, $6, $7)`, it.BoardID, name, it.Description, it.X, it.Y, it.Effect, it.Icon)
			} else {
				if !existing[it.ID] {
					return invalid("oggetto sconosciuto: %s", it.ID)
				}
				kept[it.ID] = true
				_, err = tx.Exec(ctx, `UPDATE board_items SET board_id = $2::uuid, name = $3, description = $4, x = $5, y = $6,
					effect = $7, icon = $8 WHERE id = $1::uuid`, it.ID, it.BoardID, name, it.Description, it.X, it.Y, it.Effect, it.Icon)
			}
			if err != nil {
				return invalidIfUnique(err, "oggetto %q: la casella (%d, %d) è già occupata", name, it.X, it.Y)
			}
		}
		for id := range existing {
			if !kept[id] {
				if _, err := tx.Exec(ctx, `DELETE FROM board_items WHERE id = $1::uuid`, id); err != nil {
					return err
				}
			}
		}
		if err := noTwoOnACell(ctx, tx, worldID); err != nil {
			return err
		}
		return noBlockingTerrainOnThings(ctx, tx, worldID)
	}
}

// invalidIfUnique turns a violated unique constraint into a message for the admin.
func invalidIfUnique(err error, format string, args ...any) error {
	var pgErr interface{ SQLState() string }
	if errors.As(err, &pgErr) && pgErr.SQLState() == "23505" {
		return invalid(format, args...)
	}
	return err
}

// TerrainChange replaces the terrain of the boards it lists. Blocking terrain cannot go where
// something already is (a unit, an item, a structure) or where a passage starts or ends.
func TerrainChange(worldID string, boards []AdminTerrain) Change {
	return func(ctx context.Context, tx pgx.Tx) error {
		dims, err := worldBoards(ctx, tx, worldID)
		if err != nil {
			return err
		}
		seen := map[string]bool{}
		for _, t := range boards {
			d, ok := dims[t.BoardID]
			if !ok {
				return invalid("terreno: board inesistente")
			}
			if seen[t.BoardID] {
				return invalid("terreno: la stessa board compare due volte")
			}
			seen[t.BoardID] = true
			if len(t.Rows) != d.h {
				return invalid("terreno: la board ha %d righe, non %d", d.h, len(t.Rows))
			}
			type cell struct {
				x, y int
				name string
			}
			var cells []cell
			for y, row := range t.Rows {
				if len(row) != d.w {
					return invalid("terreno: la riga %d ha %d caselle, non %d", y+1, len(row), d.w)
				}
				for x := 0; x < len(row); x++ {
					name, ok := game.TerrainName(row[x])
					if !ok {
						return invalid("terreno: carattere sconosciuto %q alla casella (%d, %d)", string(row[x]), x, y)
					}
					if name != "grass" {
						cells = append(cells, cell{x, y, name})
					}
				}
			}
			if _, err := tx.Exec(ctx, `DELETE FROM board_terrain WHERE board_id = $1::uuid`, t.BoardID); err != nil {
				return err
			}
			for _, c := range cells {
				if _, err := tx.Exec(ctx, `INSERT INTO board_terrain (board_id, x, y, tile) VALUES ($1::uuid, $2, $3, $4)`,
					t.BoardID, c.x, c.y, c.name); err != nil {
					return err
				}
			}
		}
		return noBlockingTerrainOnThings(ctx, tx, worldID)
	}
}

// noBlockingTerrainOnThings rejects a world where blocking terrain sits under a unit, an item, a
// structure or an end of a passage.
func noBlockingTerrainOnThings(ctx context.Context, tx pgx.Tx, worldID string) error {
	var name string
	var x, y int
	blocking := []string{}
	for _, k := range game.TerrainKinds() {
		if k.Blocks {
			blocking = append(blocking, k.Name)
		}
	}
	err := tx.QueryRow(ctx, `SELECT b.name, t.x, t.y FROM board_terrain t JOIN boards b ON b.id = t.board_id
		WHERE b.world_id = $1::uuid AND t.tile = ANY($2) AND (
			EXISTS (SELECT 1 FROM units u WHERE u.board_id = t.board_id AND u.x = t.x AND u.y = t.y)
			OR EXISTS (SELECT 1 FROM board_items i WHERE i.board_id = t.board_id AND i.x = t.x AND i.y = t.y)
			OR EXISTS (SELECT 1 FROM structures s WHERE s.board_id = t.board_id AND s.x = t.x AND s.y = t.y)
			OR EXISTS (SELECT 1 FROM board_links l WHERE (l.from_board_id = t.board_id AND l.from_x = t.x AND l.from_y = t.y)
				OR (l.to_board_id = t.board_id AND l.to_x = t.x AND l.to_y = t.y))) LIMIT 1`, worldID, blocking).Scan(&name, &x, &y)
	if errors.Is(err, pgx.ErrNoRows) {
		return nil
	}
	if err != nil {
		return err
	}
	return invalid("la casella (%d, %d) della board %q è occupata (pedina, oggetto, struttura o passaggio) e non può avere un terreno che blocca", x, y, name)
}
