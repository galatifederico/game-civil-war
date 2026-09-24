package game

// How the client draws a unit. There is one standard pawn (a round little creature); everything
// else is that pawn with something changed:
//   - the race gives it a colour (Race.Look, one of Looks);
//   - the role adds something: the champion wears a crown, a minor unit nothing;
//   - an NPC is a pawn with its own colours and headgear (NPCSprites).
// The client has one row of frames for every name UnitSprite and SpriteChoices can return.

// Looks are the colours a race can give the pawn (salmon is the original).
var Looks = []string{"salmon", "azure", "moss", "sun", "violet", "slate"}

// ValidLook tells whether a look is known ("" means the default and is valid too).
func ValidLook(look string) bool {
	if look == "" {
		return true
	}
	for _, l := range Looks {
		if l == look {
			return true
		}
	}
	return false
}

// NPCSprites are the characters an NPC can be, besides a plain coloured pawn.
var NPCSprites = []string{"merchant", "guard", "sage", "blacksmith", "wanderer"}

// SpriteChoices lists everything an admin can pick as an NPC's sprite: the NPC characters and a
// plain pawn in each look ("minor-azure"...).
func SpriteChoices() []string {
	out := append([]string(nil), NPCSprites...)
	for _, l := range Looks {
		out = append(out, "minor-"+l)
	}
	return out
}

// ValidSprite tells whether a sprite name is known ("" means the default and is valid too).
func ValidSprite(name string) bool {
	if name == "" {
		return true
	}
	for _, n := range SpriteChoices() {
		if n == name {
			return true
		}
	}
	return false
}

// UnitSprite is the sprite of a player's unit: its role, in the look of its race.
func UnitSprite(kind Kind, look string) string {
	role := "minor"
	if kind == KindChampion {
		role = "champion"
	}
	if look == "" {
		return role
	}
	return role + "-" + look
}
