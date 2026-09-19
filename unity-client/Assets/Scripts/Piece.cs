using UnityEngine;

// Rappresentazione di un'entita' del server (pedina, NPC, oggetto, struttura) come sprite in
// pixel art. Non decide nulla: applica lo stato ricevuto e scorre visivamente verso la casella.
public class Piece : MonoBehaviour
{
    const float SlideSpeed = 5f;

    public EntityData Data { get; private set; }
    public bool Mine { get; private set; }

    SpriteRenderer body, shadow, ring;
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
            var race = string.IsNullOrEmpty(Data.race) ? "" : " · " + Data.race;
            switch (Data.kind)
            {
                case Kinds.Champion: return "Champion - " + team + race;
                case Kinds.Minor: return "Pedina - " + team + race;
                case Kinds.Structure: return "Struttura - " + team;
                case Kinds.Npc: return "NPC - non controllabile";
                default: return "Oggetto - non controllabile";
            }
        }
    }

    public void Init(EntityData data, bool mine, Color color)
    {
        Mine = mine;
        baseColor = color;
        shadow = NewRenderer("Shadow", PixelArt.Shadow);
        ring = NewRenderer("Ring", PixelArt.Ring);
        ring.enabled = false;
        body = NewRenderer("Body", PixelArt.Piece(data.kind));
        Apply(data);
        transform.position = target;
        UpdateSorting();
    }

    SpriteRenderer NewRenderer(string name, Sprite sprite)
    {
        var go = new GameObject(name);
        go.transform.SetParent(transform, false);
        var r = go.AddComponent<SpriteRenderer>();
        r.sprite = sprite;
        return r;
    }

    public void Apply(EntityData data)
    {
        bool changedBoard = Data != null && Data.board_id != data.board_id;
        Data = data;
        var grid = BoardManager.Instance.GridOf(data.board_id);
        target = GridMath.CellToWorld(grid, data.x, data.y);
        if (changedBoard) transform.position = target;
        readyAt = Time.time + data.ready_in_ms / 1000f;
        actReadyAt = Time.time + data.act_ready_in_ms / 1000f;
        respawnAt = Time.time + data.respawn_in_ms / 1000f;
        RefreshLook();
    }

    void Update()
    {
        transform.position = Vector3.MoveTowards(transform.position, target, SlideSpeed * Time.deltaTime);
        UpdateSorting();
    }

    // Piu' in basso sullo schermo = piu' vicino = disegnato sopra.
    void UpdateSorting()
    {
        int order = BoardManager.SortOrder(transform.position.y);
        shadow.sortingOrder = order + 1;
        ring.sortingOrder = order + 2;
        body.sortingOrder = order + 3;
    }

    public void SetHighlight(bool on)
    {
        highlighted = on;
        RefreshLook();
    }

    // Una pedina sconfitta resta sulla casella, sdraiata e scura, finche' il server non la fa rinascere.
    void RefreshLook()
    {
        bool down = Defeated;
        body.color = down ? Color.Lerp(baseColor, new Color(0.1f, 0.1f, 0.1f), 0.75f) : baseColor;
        body.transform.localRotation = Quaternion.Euler(0f, 0f, down ? 90f : 0f);
        body.transform.localPosition = down ? new Vector3(0.12f, 0f, 0f) : Vector3.zero;
        ring.enabled = highlighted;
    }
}
