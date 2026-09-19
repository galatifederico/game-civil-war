package game

// Icons an item can have in the inventory. The client draws them; the admin picks one per item.
var ItemIcons = []string{"potion", "coin", "sword", "gem", "chest", "sign", "box"}

// ValidIcon tells whether an icon key is known ("" means "automatic" and is valid too).
func ValidIcon(icon string) bool {
	if icon == "" {
		return true
	}
	for _, k := range ItemIcons {
		if k == icon {
			return true
		}
	}
	return false
}

// ResolveIcon is the icon to show for an item: the one it was given or, if none, one that fits
// what the item does.
func ResolveIcon(icon string, fx Effect) string {
	switch {
	case icon != "":
		return icon
	case fx.Heal > 0:
		return "potion"
	case fx.Points != 0:
		return "coin"
	case fx.Strength != 0:
		return "sword"
	case len(fx.Traits) > 0:
		return "gem"
	}
	return "box"
}

// ResolvedIcon is the icon the client should draw for this item.
func (i Item) ResolvedIcon() string { return ResolveIcon(i.Icon, i.Effect) }
