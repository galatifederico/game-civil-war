extends Node2D
## Board view: draws the grid, keeps one Unit instance per live unit in sync
## with GameState, and turns left-clicks into move_command messages. All game
## logic (is the move valid? did it happen?) lives on the server — this scene
## only reflects state_delta/world_snapshot back onto the screen.

const UNIT_SCENE := preload("res://scenes/Unit.tscn")
const GRID_COLOR := Color(1, 1, 1, 0.15)

@onready var units_container: Node2D = $UnitsContainer
@onready var status_label: Label = $HUD/StatusLabel

var _unit_nodes: Dictionary = {} # unit_id -> Node2D


func _ready() -> void:
	NetworkClient.ws_connected.connect(_on_ws_connected)
	NetworkClient.ws_disconnected.connect(_on_ws_disconnected)
	NetworkClient.server_error.connect(_on_server_error)
	GameState.units_updated.connect(_on_units_updated)

	status_label.text = "Connessione al server..."
	NetworkClient.connect_to_game()


func _on_ws_connected() -> void:
	status_label.text = "Connesso. Clic sinistro per muovere il campione."
	queue_redraw()


func _on_ws_disconnected() -> void:
	status_label.text = "Disconnesso dal server."


func _on_server_error(message: String) -> void:
	status_label.text = "Errore server: " + message


func _on_units_updated() -> void:
	var seen := {}
	for unit_id in GameState.units.keys():
		seen[unit_id] = true
		var unit: Dictionary = GameState.units[unit_id]
		var node: Node2D = _unit_nodes.get(unit_id)
		if node == null:
			node = UNIT_SCENE.instantiate()
			units_container.add_child(node)
			_unit_nodes[unit_id] = node
		node.apply(unit, unit_id == GameState.my_unit_id)

	for unit_id in _unit_nodes.keys():
		if not seen.has(unit_id):
			_unit_nodes[unit_id].queue_free()
			_unit_nodes.erase(unit_id)

	queue_redraw() # board dimensions may have just arrived with this update


func _draw() -> void:
	if GameState.board_width <= 0:
		return
	var cell := GameState.CELL_SIZE
	for x in range(GameState.board_width + 1):
		draw_line(Vector2(x * cell, 0), Vector2(x * cell, GameState.board_height * cell), GRID_COLOR)
	for y in range(GameState.board_height + 1):
		draw_line(Vector2(0, y * cell), Vector2(GameState.board_width * cell, y * cell), GRID_COLOR)


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		if GameState.my_unit_id == "":
			return
		var cell := GameState.CELL_SIZE
		var world_pos := get_global_mouse_position()
		var grid_x := int(floor(world_pos.x / cell))
		var grid_y := int(floor(world_pos.y / cell))
		NetworkClient.send_move(GameState.my_unit_id, grid_x, grid_y)
