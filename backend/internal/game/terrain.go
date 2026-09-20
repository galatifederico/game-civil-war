package game

// Terrain is what a cell of a board is made of. Every cell is grass unless the board says
// otherwise. Some kinds block: a unit cannot end a move there and nothing can be put there.
// Movement has no path-finding (a unit jumps to any free cell within its speed), so blocking
// terrain only decides where one can stand.

type terrainKind struct {
	Glyph  byte   // how a board row spells it (Board rows of TerrainRows)
	Name   string // how the database and the admin app spell it
	Blocks bool
}

var terrainKinds = []terrainKind{
	{'.', "grass", false},
	{',', "flowers", false},
	{'=', "path", false},
	{':', "stone", false},
	{'T', "tree", true},
	{'B', "bush", true},
	{'F', "fence", true},
	{'#', "wall", true},
}

const grassGlyph = '.'

// TerrainNames lists every kind, grass first (the admin app offers them as a palette).
func TerrainNames() []string {
	out := make([]string, len(terrainKinds))
	for i, k := range terrainKinds {
		out[i] = k.Name
	}
	return out
}

// TerrainGlyph is the one-character spelling of a terrain kind; ok is false for unknown kinds.
func TerrainGlyph(name string) (byte, bool) {
	for _, k := range terrainKinds {
		if k.Name == name {
			return k.Glyph, true
		}
	}
	return 0, false
}

// TerrainName is the kind spelled by a glyph; ok is false for unknown glyphs.
func TerrainName(glyph byte) (string, bool) {
	for _, k := range terrainKinds {
		if k.Glyph == glyph {
			return k.Name, true
		}
	}
	return "", false
}

// TerrainBlocks tells whether a kind blocks (grass and unknown kinds do not).
func TerrainBlocks(name string) bool {
	for _, k := range terrainKinds {
		if k.Name == name {
			return k.Blocks
		}
	}
	return false
}

// SetTerrain puts a terrain kind on a cell of the board ("grass" clears it).
func (b *Board) SetTerrain(p Point, name string) {
	if name == "" || name == "grass" {
		delete(b.terrain, p)
		return
	}
	if b.terrain == nil {
		b.terrain = map[Point]string{}
	}
	b.terrain[p] = name
}

// TerrainAt is the terrain of a cell ("grass" if nothing else is set).
func (b *Board) TerrainAt(p Point) string {
	if name, ok := b.terrain[p]; ok {
		return name
	}
	return "grass"
}

// BlocksAt tells whether the terrain of a cell blocks.
func (b *Board) BlocksAt(p Point) bool {
	name, ok := b.terrain[p]
	return ok && TerrainBlocks(name)
}

// TerrainRows spells the whole board, one string per row (top row first), or nil if every cell
// is grass. This is how the terrain travels to the client and to the admin app.
func (b *Board) TerrainRows() []string {
	if len(b.terrain) == 0 {
		return nil
	}
	rows := make([]string, b.Height)
	for y := 0; y < b.Height; y++ {
		row := make([]byte, b.Width)
		for x := 0; x < b.Width; x++ {
			glyph := byte(grassGlyph)
			if g, ok := TerrainGlyph(b.TerrainAt(Point{x, y})); ok {
				glyph = g
			}
			row[x] = glyph
		}
		rows[y] = string(row)
	}
	return rows
}

// TerrainInfo describes a kind of terrain for the admin app's palette.
type TerrainInfo struct {
	Name   string `json:"name"`
	Glyph  string `json:"glyph"`
	Blocks bool   `json:"blocks"`
}

func TerrainKinds() []TerrainInfo {
	out := make([]TerrainInfo, len(terrainKinds))
	for i, k := range terrainKinds {
		out[i] = TerrainInfo{Name: k.Name, Glyph: string(k.Glyph), Blocks: k.Blocks}
	}
	return out
}
