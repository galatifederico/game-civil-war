package game

// Sprites an NPC can have. The client draws them (one set of walking frames per name); the admin
// picks one per NPC. Player units use the sprite of their role ("champion" or "minor").
var SpriteNames = []string{"champion", "minor", "merchant", "guard", "sage", "blacksmith", "wanderer"}

// ValidSprite tells whether a sprite name is known ("" means "default" and is valid too).
func ValidSprite(name string) bool {
	if name == "" {
		return true
	}
	for _, n := range SpriteNames {
		if n == name {
			return true
		}
	}
	return false
}
