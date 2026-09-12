extends Control

@onready var email_edit: LineEdit = $CenterContainer/VBoxContainer/EmailEdit
@onready var password_edit: LineEdit = $CenterContainer/VBoxContainer/PasswordEdit
@onready var status_label: Label = $CenterContainer/VBoxContainer/StatusLabel


func _ready() -> void:
	NetworkClient.auth_success.connect(_on_auth_success)
	NetworkClient.auth_failed.connect(_on_auth_failed)


func _on_register_pressed() -> void:
	status_label.text = "Registrazione in corso..."
	NetworkClient.register(email_edit.text, password_edit.text)


func _on_login_pressed() -> void:
	status_label.text = "Accesso in corso..."
	NetworkClient.login(email_edit.text, password_edit.text)


func _on_auth_success(_token: String) -> void:
	status_label.text = "OK, connessione al mondo..."
	get_tree().change_scene_to_file("res://scenes/Game.tscn")


func _on_auth_failed(message: String) -> void:
	status_label.text = "Errore: " + message
