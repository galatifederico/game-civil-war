extends Node
## Singleton (autoload) holding the client's view of the current board.
## Populated entirely from NetworkClient signals — never mutated locally
## from player input, since the server is the sole authority on state.

signal units_updated

const CELL_SIZE := 32

var board_id: String = ""
var board_width: int = 0
var board_height: int = 0
var my_unit_id: String = ""

## unit_id (String) -> unit dict {id, player_id, is_champion, x, y, speed, health}
var units: Dictionary = {}


func _ready() -> void:
	NetworkClient.world_snapshot.connect(_on_world_snapshot)
	NetworkClient.state_delta.connect(_on_state_delta)
	NetworkClient.ws_disconnected.connect(_on_disconnected)


func _on_world_snapshot(payload: Dictionary) -> void:
	board_id = payload.get("board_id", "")
	board_width = payload.get("width", 0)
	board_height = payload.get("height", 0)
	my_unit_id = payload.get("your_unit_id", "")

	units.clear()
	for u in payload.get("units", []):
		units[u["id"]] = u
	units_updated.emit()


func _on_state_delta(payload: Dictionary) -> void:
	for u in payload.get("units", []):
		units[u["id"]] = u
	units_updated.emit()


func _on_disconnected() -> void:
	units.clear()
	units_updated.emit()


func my_unit() -> Dictionary:
	return units.get(my_unit_id, {})
