extends Node
## Singleton (autoload) that owns all communication with the Go backend:
## REST for auth, WebSocket for realtime game state. Godot is a "dumb"
## client here (docs/tecnico.md) — it never decides game outcomes itself,
## only sends commands and renders whatever state the server broadcasts.

signal auth_success(token: String)
signal auth_failed(message: String)
signal ws_connected
signal ws_disconnected
signal world_snapshot(payload: Dictionary)
signal state_delta(payload: Dictionary)
signal server_error(message: String)

const BASE_URL := "http://localhost:8080"
const WS_URL := "ws://localhost:8080/ws"

var token: String = ""

var _socket := WebSocketPeer.new()
var _ws_active := false
var _http_register: HTTPRequest
var _http_login: HTTPRequest


func _ready() -> void:
	_http_register = HTTPRequest.new()
	add_child(_http_register)
	_http_register.request_completed.connect(_on_register_completed)

	_http_login = HTTPRequest.new()
	add_child(_http_login)
	_http_login.request_completed.connect(_on_login_completed)


func register(email: String, password: String) -> void:
	var body := JSON.stringify({"email": email, "password": password})
	_http_register.request(BASE_URL + "/auth/register", ["Content-Type: application/json"], HTTPClient.METHOD_POST, body)


func login(email: String, password: String) -> void:
	var body := JSON.stringify({"email": email, "password": password})
	_http_login.request(BASE_URL + "/auth/login", ["Content-Type: application/json"], HTTPClient.METHOD_POST, body)


func _on_register_completed(_result: int, code: int, _headers: PackedStringArray, body: PackedByteArray) -> void:
	_handle_auth_response(code, body)


func _on_login_completed(_result: int, code: int, _headers: PackedStringArray, body: PackedByteArray) -> void:
	_handle_auth_response(code, body)


func _handle_auth_response(code: int, body: PackedByteArray) -> void:
	var parsed = JSON.parse_string(body.get_string_from_utf8())
	if code == 200 or code == 201:
		token = parsed.get("token", "")
		auth_success.emit(token)
	else:
		var message: String = "unknown error"
		if parsed is Dictionary and parsed.has("error"):
			message = parsed["error"]
		auth_failed.emit(message)


func connect_to_game() -> void:
	var err := _socket.connect_to_url(WS_URL)
	if err != OK:
		server_error.emit("could not open websocket: %s" % err)
		return
	_ws_active = true


func _process(_delta: float) -> void:
	if not _ws_active:
		return

	_socket.poll()
	var state := _socket.get_ready_state()

	if state == WebSocketPeer.STATE_OPEN:
		if not _has_authed and token != "":
			_send_envelope("auth", {"token": token})
			_has_authed = true
		while _socket.get_available_packet_count() > 0:
			_handle_packet(_socket.get_packet())
	elif state == WebSocketPeer.STATE_CLOSED:
		_ws_active = false
		_has_authed = false
		_snapshot_received = false
		ws_disconnected.emit()


var _has_authed := false
var _snapshot_received := false


func _handle_packet(raw: PackedByteArray) -> void:
	var parsed = JSON.parse_string(raw.get_string_from_utf8())
	if not (parsed is Dictionary) or not parsed.has("type"):
		return

	var msg_type: String = parsed["type"]
	var payload: Dictionary = parsed.get("payload", {})

	match msg_type:
		"world_snapshot":
			if not _snapshot_received:
				_snapshot_received = true
				ws_connected.emit()
			world_snapshot.emit(payload)
		"state_delta":
			state_delta.emit(payload)
		"error":
			server_error.emit(payload.get("message", "unknown server error"))


func send_move(unit_id: String, x: int, y: int) -> void:
	_send_envelope("move_command", {"unit_id": unit_id, "target": {"x": x, "y": y}})


func _send_envelope(msg_type: String, payload: Dictionary) -> void:
	if _socket.get_ready_state() != WebSocketPeer.STATE_OPEN:
		return
	var envelope := {"type": msg_type, "payload": payload}
	_socket.send_text(JSON.stringify(envelope))
