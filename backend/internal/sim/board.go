// Package sim runs one goroutine per board (see docs/design.md /
// docs/tecnico.md: real-time world, potentially very many units per player
// but few concurrent players). All state mutation happens on that single
// goroutine — either directly inside Run()'s select loop (for the tick) or
// via queued closures (for commands) — so the unit map never needs a mutex.
//
// M2 scope: real-time movement cooldown, one radius action (attack) with
// cooldown-only death/respawn, and one hardcoded unit-creation method
// (paying with champion health). Pickup/talk/build and the other creation
// methods arrive with the rules engine in M4/M5 — see docs/roadmap.md.
package sim

import (
	"context"
	"log"
	"math"
	"time"

	"github.com/google/uuid"

	"thegame/internal/protocol"
	"thegame/internal/store"
	"thegame/internal/world"
)

const (
	// actionRadius is hardcoded for M2. From M4 it comes from the acting
	// unit's "vista" characteristic instead (docs/design.md).
	actionRadius = 1

	attackDamage = 25.0
	respawnDelay = 10 * time.Second
	tickInterval = 100 * time.Millisecond
)

type BoardLoop struct {
	Board       *world.Board
	units       map[string]*Unit
	inbox       chan func()
	broadcaster Broadcaster
	unitRepo    *store.UnitRepo
}

func NewBoardLoop(board *world.Board, broadcaster Broadcaster, unitRepo *store.UnitRepo) *BoardLoop {
	return &BoardLoop{
		Board:       board,
		units:       make(map[string]*Unit),
		inbox:       make(chan func(), 256),
		broadcaster: broadcaster,
		unitRepo:    unitRepo,
	}
}

// Run processes queued commands and a realtime tick until ctx is cancelled.
// Meant to be started with `go loop.Run(ctx)` once per board.
func (b *BoardLoop) Run(ctx context.Context) {
	ticker := time.NewTicker(tickInterval)
	defer ticker.Stop()

	for {
		select {
		case fn := <-b.inbox:
			fn()
		case <-ticker.C:
			b.tick()
		case <-ctx.Done():
			return
		}
	}
}

// tick runs on the loop goroutine itself (called from Run's select, not
// queued) and processes pending respawns — the only time-driven mechanic so
// far (docs/design.md: death is a cooldown, not a permanent state).
func (b *BoardLoop) tick() {
	now := time.Now()
	changed := false
	for _, u := range b.units {
		if !u.Alive && !u.RespawnAt.IsZero() && now.After(u.RespawnAt) {
			u.Alive = true
			u.Health = u.MaxHealth
			u.RespawnAt = time.Time{}
			changed = true

			go func(unitID string, health float64) {
				if err := b.unitRepo.UpdateCombatState(context.Background(), unitID, health, true, nil); err != nil {
					log.Printf("sim: failed to persist respawn for unit %s: %v", unitID, err)
				}
			}(u.ID, u.Health)
		}
	}
	if changed {
		b.broadcastState()
	}
}

// JoinUnit registers a unit (freshly loaded/created from Postgres) as live on
// this board and broadcasts the updated state to everyone already connected.
func (b *BoardLoop) JoinUnit(u *Unit) {
	done := make(chan struct{})
	b.inbox <- func() {
		b.units[u.ID] = u
		b.broadcastState()
		close(done)
	}
	<-done
}

// LeaveUnit removes a unit from the live board (e.g. on disconnect) without
// deleting it from Postgres — it simply stops being simulated until rejoin.
func (b *BoardLoop) LeaveUnit(unitID string) {
	done := make(chan struct{})
	b.inbox <- func() {
		delete(b.units, unitID)
		b.broadcastState()
		close(done)
	}
	<-done
}

// Snapshot returns a point-in-time copy of every live unit on the board, used
// to build the initial world_snapshot sent to a newly connected client.
func (b *BoardLoop) Snapshot() []Unit {
	resultCh := make(chan []Unit, 1)
	b.inbox <- func() {
		out := make([]Unit, 0, len(b.units))
		for _, u := range b.units {
			out = append(out, *u)
		}
		resultCh <- out
	}
	return <-resultCh
}

// HandleMove validates and applies a move_command.
func (b *BoardLoop) HandleMove(playerID, unitID string, target world.Coord) error {
	errCh := make(chan error, 1)
	b.inbox <- func() {
		errCh <- b.applyMove(playerID, unitID, target)
	}
	return <-errCh
}

func (b *BoardLoop) applyMove(playerID, unitID string, target world.Coord) error {
	u, err := b.ownedLivingUnit(playerID, unitID)
	if err != nil {
		return err
	}
	now := time.Now()
	if now.Before(u.MoveReadyAt) {
		return ErrOnCooldown
	}
	if !b.Board.Grid.InBounds(target) {
		return ErrOutOfBounds
	}
	distance := b.Board.Grid.Distance(u.Pos, target)
	if distance > int(u.Speed) {
		return ErrOutOfRange
	}
	if distance == 0 {
		return nil
	}

	u.Pos = target
	// Movement cooldown: 1 second per cell divided by speed, i.e. speed is
	// "cells per second" (docs/main.md: some moves, like speed, need time to
	// recharge). Ephemeral — not persisted, see Unit doc comment.
	seconds := float64(distance) / u.Speed
	u.MoveReadyAt = now.Add(time.Duration(seconds * float64(time.Second)))

	b.persistPosition(u.ID, u.Pos)
	b.broadcastState()
	return nil
}

// HandleAction validates and applies an action_command (attack for M2;
// pickup/talk/build are accepted on the wire but rejected until M4/M5).
func (b *BoardLoop) HandleAction(playerID, unitID, action string, target world.Coord) error {
	errCh := make(chan error, 1)
	b.inbox <- func() {
		switch action {
		case protocol.ActionAttack:
			errCh <- b.applyAttack(playerID, unitID, target)
		case protocol.ActionPickup, protocol.ActionTalk, protocol.ActionBuild:
			errCh <- ErrNotImplemented
		default:
			errCh <- ErrUnknownAction
		}
	}
	return <-errCh
}

func (b *BoardLoop) applyAttack(playerID, unitID string, target world.Coord) error {
	attacker, err := b.ownedLivingUnit(playerID, unitID)
	if err != nil {
		return err
	}
	if b.Board.Grid.Distance(attacker.Pos, target) > actionRadius {
		return ErrOutOfActionRange
	}

	defender := b.findLivingUnitAt(target)
	if defender == nil {
		return ErrNoTargetThere
	}
	if defender.PlayerID == playerID {
		return ErrCannotAttackAlly
	}

	defender.Health = math.Max(0, defender.Health-attackDamage)
	var respawnAt *time.Time
	if defender.Health <= 0 {
		defender.Alive = false
		defender.RespawnAt = time.Now().Add(respawnDelay)
		respawnAt = &defender.RespawnAt
	}

	go func(unitID string, health float64, alive bool, respawnAt *time.Time) {
		if err := b.unitRepo.UpdateCombatState(context.Background(), unitID, health, alive, respawnAt); err != nil {
			log.Printf("sim: failed to persist combat state for unit %s: %v", unitID, err)
		}
	}(defender.ID, defender.Health, defender.Alive, respawnAt)

	b.broadcastState()
	return nil
}

// HandleCreateUnit validates and applies a create_unit_command: the champion
// pays a fixed health cost to spawn a new minor unit next to itself
// (docs/design.md: "consumo di caratteristiche del campione", one of several
// admin-configurable creation methods — hardcoded here until M4).
func (b *BoardLoop) HandleCreateUnit(playerID, championID string) error {
	errCh := make(chan error, 1)
	b.inbox <- func() {
		errCh <- b.applyCreateUnit(playerID, championID)
	}
	return <-errCh
}

func (b *BoardLoop) applyCreateUnit(playerID, championID string) error {
	champion, err := b.ownedLivingUnit(playerID, championID)
	if err != nil {
		return err
	}
	if !champion.IsChampion {
		return ErrNotOwner
	}
	if champion.Health <= store.ChampionCostPerMinorUnit {
		return ErrInsufficientHealth
	}

	champion.Health -= store.ChampionCostPerMinorUnit
	go func(unitID string, health float64) {
		if err := b.unitRepo.UpdateCombatState(context.Background(), unitID, health, true, nil); err != nil {
			log.Printf("sim: failed to persist champion cost for unit %s: %v", unitID, err)
		}
	}(champion.ID, champion.Health)

	// Synchronous (unlike position/combat updates above): creation needs the
	// generated row back before the new unit can join the live board, and it
	// happens far less often than movement/combat, so blocking this board's
	// goroutine on one insert is an acceptable tradeoff for now.
	spawnPos := champion.Pos
	newUnit, err := b.unitRepo.CreateMinorUnit(context.Background(), uuid.NewString(), playerID, b.Board.ID, spawnPos.X, spawnPos.Y)
	if err != nil {
		return err
	}

	b.units[newUnit.ID] = &Unit{
		ID: newUnit.ID, PlayerID: newUnit.PlayerID, BoardID: newUnit.BoardID, IsChampion: false,
		Pos: world.Coord{X: newUnit.X, Y: newUnit.Y}, Speed: newUnit.Speed,
		Health: newUnit.Health, MaxHealth: newUnit.MaxHealth, Alive: newUnit.Alive,
	}

	b.broadcastState()
	return nil
}

// ownedLivingUnit looks up a unit and checks it exists, belongs to playerID,
// and is currently alive — the shared precondition for move/attack/create.
func (b *BoardLoop) ownedLivingUnit(playerID, unitID string) (*Unit, error) {
	u, ok := b.units[unitID]
	if !ok {
		return nil, ErrUnitNotFound
	}
	if u.PlayerID != playerID {
		return nil, ErrNotOwner
	}
	if !u.Alive {
		return nil, ErrDead
	}
	return u, nil
}

func (b *BoardLoop) findLivingUnitAt(coord world.Coord) *Unit {
	for _, u := range b.units {
		if u.Alive && u.Pos == coord {
			return u
		}
	}
	return nil
}

func (b *BoardLoop) persistPosition(unitID string, pos world.Coord) {
	go func(unitID string, pos world.Coord) {
		if err := b.unitRepo.UpdatePosition(context.Background(), unitID, pos.X, pos.Y); err != nil {
			log.Printf("sim: failed to persist position for unit %s: %v", unitID, err)
		}
	}(unitID, pos)
}

// broadcastState must only be called from the loop goroutine (i.e. from
// inside a queued closure, or from tick/Run directly) since it reads
// b.units without a lock.
func (b *BoardLoop) broadcastState() {
	states := make([]protocol.UnitState, 0, len(b.units))
	for _, u := range b.units {
		states = append(states, u.ToProto())
	}
	msg, err := protocol.Marshal(protocol.TypeStateDelta, protocol.StateDeltaPayload{Units: states})
	if err != nil {
		log.Printf("sim: failed to marshal state_delta: %v", err)
		return
	}
	b.broadcaster.BroadcastToBoard(b.Board.ID, msg)
}
