extends Node2D
## Visual representation of one unit ("pedina"). Purely a renderer: it never
## decides its own position, only tweens toward whatever the server last said
## (docs/tecnico.md: Godot is a "dumb" client, the server is authoritative).

const CHAMPION_COLOR := Color(1.0, 0.8, 0.1)
const MINOR_COLOR := Color(0.3, 0.6, 1.0)
const MINE_OUTLINE := Color(0.2, 1.0, 0.2)

@onready var body: ColorRect = $Body
@onready var label: Label = $Label

var unit_id: String = ""
var _tween: Tween
var _placed := false


func apply(unit: Dictionary, is_mine: bool) -> void:
	unit_id = unit.get("id", "")
	body.color = CHAMPION_COLOR if unit.get("is_champion", false) else MINOR_COLOR
	label.text = ("★ " if unit.get("is_champion", false) else "") + unit_id.substr(0, 4)
	if is_mine:
		label.text += " (tu)"

	var target_pos := Vector2(unit.get("x", 0), unit.get("y", 0)) * GameState.CELL_SIZE
	if not _placed:
		_placed = true
		global_position = target_pos
	else:
		_move_to(target_pos)


func _move_to(target_pos: Vector2) -> void:
	if _tween:
		_tween.kill()
	_tween = create_tween()
	_tween.tween_property(self, "global_position", target_pos, 0.2)
