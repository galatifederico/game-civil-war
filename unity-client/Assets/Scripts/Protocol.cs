using System;

// Messaggi JSON scambiati con il backend (vedi backend/internal/protocol).
// I nomi dei campi sono in snake_case perche' JsonUtility li mappa 1:1.

[Serializable]
public class EntityData
{
    public string id;
    public string kind;
    public string owner_id;
    public string name;
    public string description;
    public int x;
    public int y;
    public int speed;
    public int health;
    public int max_health;
    public int vision;
    public int ready_in_ms;
}

[Serializable]
public class BoardData
{
    public string id;
    public string name;
    public string grid;
    public int width;
    public int height;
}

[Serializable]
public class ServerMessage
{
    public string type;
    public string your_player_id;
    public string code;
    public string message;
    public BoardData board;
    public EntityData[] entities;
    public string[] removed;
}

[Serializable]
public class ClientMessage
{
    public string type;
    public string token;
    public string unit_id;
    public int x;
    public int y;
}

[Serializable]
public class AuthRequest
{
    public string email;
    public string username;
    public string password;
}

[Serializable]
public class AuthResponse
{
    public string token;
    public string player_id;
    public string username;
    public string error;
}

public static class Kinds
{
    public const string Champion = "champion";
    public const string Minor = "minor";
    public const string Npc = "npc";
    public const string Item = "item";
}
