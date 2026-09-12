package store

import "time"

type Player struct {
	ID           string
	Email        string
	PasswordHash string
	CreatedAt    time.Time
}

type World struct {
	ID        string
	Name      string
	CreatedAt time.Time
}

type Board struct {
	ID       string
	WorldID  string
	Name     string
	GridType string
	Width    int
	Height   int
}

type Unit struct {
	ID         string
	PlayerID   string
	BoardID    string
	IsChampion bool
	X          int
	Y          int
	Speed      float64
	Health     float64
	MaxHealth  float64
	Alive      bool
	RespawnAt  *time.Time
	CreatedAt  time.Time
}
