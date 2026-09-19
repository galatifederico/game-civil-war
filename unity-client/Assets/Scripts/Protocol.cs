using System;

// Messaggi JSON scambiati con il backend (vedi backend/internal/protocol).
// I nomi dei campi sono in snake_case perche' JsonUtility li mappa 1:1.

[Serializable]
public class TraitData
{
    public string name;
    public int value;
}

[Serializable]
public class EntityData
{
    public string id;
    public string board_id;
    public string race_id;
    public string race;
    public TraitData[] traits;
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
    public int strength;
    public int ready_in_ms;
    public int act_ready_in_ms;
    public int respawn_in_ms;
}

[Serializable]
public class ItemData
{
    public string id;
    public string name;
    public string description;
    public string effect;
    public string icon;
}

[Serializable]
public class GoalData
{
    public string id;
    public string scope;
    public string kind;
    public string title;
    public string description;
    public int target;
    public int progress;
    public int reward;
    public bool completed;
    public string achieved_by;
}

[Serializable]
public class ScoreData
{
    public string player_id;
    public string username;
    public int points;
    public bool afk;
}

[Serializable]
public class GatewayData
{
    public int x;
    public int y;
    public string to_board;
    public int to_x;
    public int to_y;
}

[Serializable]
public class BoardData
{
    public string id;
    public string name;
    public string grid;
    public int width;
    public int height;
    public GatewayData[] gateways;
}

[Serializable]
public class ServerMessage
{
    public string type;
    public string your_player_id;
    public string code;
    public string title;
    public string message;
    public BoardData[] boards;
    public EntityData[] entities;
    public string[] removed;
    public ScoreData[] scores;
    public ItemData[] inventory;
    public GoalData[] goals;
}

[Serializable]
public class ClientMessage
{
    public string type;
    public string token;
    public string world_id;
    public string method;
    public string unit_id;
    public string target_id;
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
public class ErrorResponse
{
    public string error;
}

[Serializable]
public class WorldEntry
{
    public string id;
    public string name;
    public string description;
    public int boards;
    public int players;
    public bool joined;
    public bool admin;
    public RaceEntry[] races;
}

[Serializable]
public class RaceEntry
{
    public string id;
    public string name;
    public string description;
}

[Serializable]
public class JoinRequest
{
    public string race_id;
}

[Serializable]
public class WorldList
{
    public WorldEntry[] worlds;
}

[Serializable]
public class CreateWorldRequest
{
    public string name;
    public string description;
}

[Serializable]
public class IdResponse
{
    public string id;
    public string world_id;
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
    public const string Structure = "structure";
}
