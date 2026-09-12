package sim

import "errors"

var (
	ErrUnitNotFound       = errors.New("unit not found on this board")
	ErrNotOwner           = errors.New("unit does not belong to this player")
	ErrOutOfBounds        = errors.New("target coordinate is out of bounds")
	ErrOutOfRange         = errors.New("target is further than the unit's speed allows")
	ErrOnCooldown         = errors.New("unit is still on cooldown")
	ErrDead               = errors.New("unit is dead and respawning")
	ErrInsufficientHealth = errors.New("champion does not have enough health to pay this cost")
	ErrOutOfActionRange   = errors.New("target is outside the unit's action radius")
	ErrNoTargetThere      = errors.New("no valid target at that coordinate")
	ErrCannotAttackAlly   = errors.New("cannot attack a unit belonging to yourself")
	ErrUnknownAction      = errors.New("unknown action")
	ErrNotImplemented     = errors.New("this action is not implemented yet")
)
