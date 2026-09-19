package game

// newTestWorld is a world with one square board called "main"; entities without a board are put on it.
func newTestWorld(id, name string, width, height int, entities []*Entity) *World {
	w := NewWorld(id, name)
	w.AddBoard(&Board{ID: "main", Name: "main", Width: width, Height: height, Grid: squareGrid{}})
	for _, e := range entities {
		if e.BoardID == "" {
			e.BoardID = "main"
		}
		w.Add(e)
	}
	return w
}

func mainCell(x, y int) Cell { return Cell{"main", x, y} }
