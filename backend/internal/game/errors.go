package game

import "errors"

// Error is a rule violation that can be reported to the client as-is.
type Error struct {
	Code    string
	Message string
}

func (e *Error) Error() string { return e.Message }

var (
	ErrNotFound        = &Error{"not_found", "pedina non trovata"}
	ErrNotYours        = &Error{"not_yours", "questa pedina non è tua"}
	ErrDead            = &Error{"dead", "la pedina è fuori gioco"}
	ErrCooldown        = &Error{"cooldown", "la pedina non è ancora pronta"}
	ErrOutOfBounds     = &Error{"out_of_bounds", "casella fuori dalla board"}
	ErrSameCell        = &Error{"same_cell", "la pedina è già in questa casella"}
	ErrOccupied        = &Error{"occupied", "la casella è occupata"}
	ErrTooFar          = &Error{"too_far", "la casella è troppo lontana"}
	ErrBoardFull       = &Error{"board_full", "non c'è spazio sulla board"}
	ErrUnknownAction   = &Error{"unknown_action", "azione sconosciuta"}
	ErrNoTarget        = &Error{"no_target", "bersaglio non trovato"}
	ErrInvalidTarget   = &Error{"invalid_target", "bersaglio non valido per questa azione"}
	ErrTargetDead      = &Error{"target_dead", "il bersaglio è già fuori gioco"}
	ErrOutOfRange      = &Error{"out_of_range", "bersaglio fuori portata"}
	ErrChampionOnly    = &Error{"champion_only", "solo il campione può farlo"}
	ErrTooWeak         = &Error{"too_weak", "il campione ha troppa poca vita per creare una pedina"}
	ErrNoSpace         = &Error{"no_space", "non c'è spazio libero accanto al campione"}
	ErrGatewayBlocked  = &Error{"gateway_blocked", "il passaggio è bloccato"}
	ErrNoBoard         = &Error{"no_board", "il mondo non ha nessuna board"}
	ErrDisabled        = &Error{"disabled", "questa possibilità è chiusa in questo mondo"}
	ErrNoItems         = &Error{"no_items", "non ci sono abbastanza oggetti nell'inventario"}
	ErrNoItem          = &Error{"no_item", "oggetto non trovato nell'inventario"}
	ErrNoEffect        = &Error{"no_effect", "questo oggetto non serve a niente"}
	ErrIncompatible    = &Error{"incompatible", "le due razze non possono avere figli insieme"}
	ErrNotRested       = &Error{"not_rested", "una delle due pedine non è ancora pronta a riprodursi"}
	ErrTargetAFK       = &Error{"target_afk", "la squadra avversaria è assente: non si può attaccare"}
	ErrGoalDone        = &Error{"goal_done", "il giocatore ha già completato questo obiettivo"}
	ErrTooManyCommands = &Error{"rate_limited", "troppi comandi in poco tempo: rallenta"}
	ErrStopped         = errors.New("simulation stopped")
)

// Describe returns the code and message to send to a client for any error.
func Describe(err error) (code, message string) {
	var ge *Error
	if errors.As(err, &ge) {
		return ge.Code, ge.Message
	}
	return "internal", "errore interno"
}
