package game

import "fmt"

// A characteristic is a named value a unit has: the four base ones (speed, health, vision,
// strength) and the extended ones a world defines (Rules.TraitNames: soldi, alcol...). Each has
// a lowest and highest value it can take, set for the whole world (Rules.Characteristics) and
// overridable by each race (Race.Bounds). Starting values and the effects of items are kept
// inside these bounds.

// Bounds are the lowest and highest value of a characteristic.
type Bounds struct {
	Min int `json:"min"`
	Max int `json:"max"`
}

func (b Bounds) Clamp(v int) int { return max(b.Min, min(b.Max, v)) }

// BoundsOverride is what a race changes of a characteristic's bounds; nil keeps the world's value.
type BoundsOverride struct {
	Min *int `json:"min,omitempty"`
	Max *int `json:"max,omitempty"`
}

// BaseCharacteristics are the four every unit has, in the order they are shown.
var BaseCharacteristics = []string{"speed", "health", "vision", "strength"}

func IsBaseCharacteristic(key string) bool {
	for _, k := range BaseCharacteristics {
		if k == key {
			return true
		}
	}
	return false
}

// fallbackBounds apply to an extended characteristic the rules say nothing about.
var fallbackBounds = Bounds{Min: 0, Max: 100}

// DefaultCharacteristics are the bounds a world starts with.
func DefaultCharacteristics() map[string]Bounds {
	return map[string]Bounds{
		"speed": {1, 10}, "health": {1, 1000}, "vision": {1, 12}, "strength": {0, 200},
		"soldi": {0, 1_000_000}, "alcol": {0, 100}, "alpha": {0, 100}, "thc": {0, 100}, "beatitudine": {0, 100}, "mana": {0, 100},
	}
}

// BoundsFor is the world's bounds for a characteristic.
func (r Rules) BoundsFor(key string) Bounds {
	if b, ok := r.Characteristics[key]; ok {
		return b
	}
	return fallbackBounds
}

// applyOverride replaces the fields of b that o sets; a field o leaves nil keeps b's.
func applyOverride(b Bounds, o BoundsOverride) Bounds {
	if o.Min != nil {
		b.Min = *o.Min
	}
	if o.Max != nil {
		b.Max = *o.Max
	}
	return b
}

// BoundsFor is the bounds of a characteristic for a unit: the world's, overridden by its race,
// unioned with what its class overrides (classID may be "": a unit starts without a class, see
// design.md). The union always takes the greater limit, both for the floor and the ceiling: a
// class layered on a race only ever widens the maximum or raises the minimum further, never
// narrows what the race alone allows.
func (w *World) BoundsFor(raceID, classID, key string) Bounds {
	b := w.Rules.BoundsFor(key)
	if race := w.races[raceID]; race != nil {
		if o, ok := race.Bounds[key]; ok {
			b = applyOverride(b, o)
		}
	}
	class := w.classes[classID] // classes[""] is always nil
	if class == nil {
		return b
	}
	o, ok := class.Bounds[key]
	if !ok {
		return b
	}
	cb := applyOverride(w.Rules.BoundsFor(key), o)
	return Bounds{Min: max(b.Min, cb.Min), Max: max(b.Max, cb.Max)}
}

// validateCharacteristics rejects bounds that make no sense (used by Rules.Validate).
func (r Rules) validateCharacteristics() []error {
	var problems []error
	known := map[string]bool{}
	for _, k := range BaseCharacteristics {
		known[k] = true
	}
	for _, n := range r.TraitNames {
		known[n] = true
	}
	for n := range DefaultCharacteristics() { // a trait removed from trait_names keeps its bounds
		known[n] = true
	}
	for key, b := range r.Characteristics {
		switch {
		case !known[key]:
			problems = append(problems, fmt.Errorf("characteristics.%s: unknown characteristic (a base one or a name in trait_names)", key))
		case b.Min > b.Max:
			problems = append(problems, fmt.Errorf("characteristics.%s: min %d is above max %d", key, b.Min, b.Max))
		case b.Min < 0 || b.Max > 10_000_000:
			problems = append(problems, fmt.Errorf("characteristics.%s: bounds must be between 0 and 10000000", key))
		case (key == "speed" || key == "health" || key == "vision") && b.Min < 1:
			problems = append(problems, fmt.Errorf("characteristics.%s: min must be at least 1", key))
		}
	}
	return problems
}
