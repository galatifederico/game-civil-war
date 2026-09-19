package game

import "errors"

// Error is a rule violation that can be reported to the client as-is.
type Error struct {
	Code    string
	Message string
}

func (e *Error) Error() string { return e.Message }

var (
	ErrNotFound    = &Error{"not_found", "pedina non trovata"}
	ErrNotYours    = &Error{"not_yours", "questa pedina non è tua"}
	ErrDead        = &Error{"dead", "la pedina è fuori gioco"}
	ErrCooldown    = &Error{"cooldown", "la pedina non è ancora pronta a muoversi"}
	ErrOutOfBounds = &Error{"out_of_bounds", "casella fuori dalla board"}
	ErrSameCell    = &Error{"same_cell", "la pedina è già in questa casella"}
	ErrOccupied    = &Error{"occupied", "la casella è occupata"}
	ErrTooFar      = &Error{"too_far", "la casella è troppo lontana"}
	ErrBoardFull   = &Error{"board_full", "non c'è spazio sulla board"}
	ErrStopped     = errors.New("simulation stopped")
)

// Describe returns the code and message to send to a client for any error.
func Describe(err error) (code, message string) {
	var ge *Error
	if errors.As(err, &ge) {
		return ge.Code, ge.Message
	}
	return "internal", "errore interno"
}
