using UnityEngine;

// Rappresentazione di un'entita' del server (pedina, NPC, oggetto, struttura). Non decide nulla:
// applica lo stato ricevuto e scorre visivamente verso la casella indicata.
public class Piece : MonoBehaviour
{
    const float SlideSpeed = 10f;

    public EntityData Data { get; private set; }
    public bool Mine { get; private set; }

    Renderer rend;
    Color baseColor;
    Vector3 target;
    float actReadyAt, respawnAt, readyAt;
    bool highlighted;

    public bool Movable => Mine && (Data.kind == Kinds.Champion || Data.kind == Kinds.Minor);
    public bool IsUnit => Data.kind == Kinds.Champion || Data.kind == Kinds.Minor || Data.kind == Kinds.Npc;
    public bool Defeated => IsUnit && Data.health <= 0;
    public float SecondsUntilReady => Mathf.Max(0f, readyAt - Time.time);
    public float SecondsUntilActReady => Mathf.Max(0f, actReadyAt - Time.time);
    public float SecondsUntilRespawn => Mathf.Max(0f, respawnAt - Time.time);

    public string KindLabel
    {
        get
        {
            var team = Mine ? "la tua squadra" : "squadra avversaria";
            switch (Data.kind)
            {
                case Kinds.Champion: return "Champion - " + team;
                case Kinds.Minor: return "Pedina - " + team;
                case Kinds.Structure: return "Struttura - " + team;
                case Kinds.Npc: return "NPC - non controllabile";
                default: return "Oggetto - non controllabile";
            }
        }
    }

    public void Init(EntityData data, bool mine, Color color)
    {
        rend = GetComponent<Renderer>();
        baseColor = color;
        Mine = mine;
        Apply(data);
        transform.position = target;
    }

    public void Apply(EntityData data)
    {
        Data = data;
        target = BoardManager.CellToWorld(data.x, data.y, transform.position.y);
        readyAt = Time.time + data.ready_in_ms / 1000f;
        actReadyAt = Time.time + data.act_ready_in_ms / 1000f;
        respawnAt = Time.time + data.respawn_in_ms / 1000f;
        RefreshColor();
    }

    void Update()
    {
        transform.position = Vector3.MoveTowards(transform.position, target, SlideSpeed * Time.deltaTime);
    }

    void OnMouseDown()
    {
        BoardManager.Instance.OnPieceClicked(this);
    }

    public void SetHighlight(bool on)
    {
        highlighted = on;
        RefreshColor();
    }

    // Una pedina sconfitta resta sulla casella, in grigio scuro, finche' il server non la fa rinascere.
    void RefreshColor()
    {
        var color = Defeated ? Color.Lerp(baseColor, new Color(0.1f, 0.1f, 0.1f), 0.8f) : baseColor;
        rend.material.color = highlighted ? Color.Lerp(color, Color.white, 0.5f) : color;
    }
}
