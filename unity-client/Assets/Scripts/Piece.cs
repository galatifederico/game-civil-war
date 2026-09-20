using UnityEngine;

// Rappresentazione di un'entita' del server (pedina, NPC, oggetto, struttura) come sprite in
// pixel art. Non decide nulla: applica lo stato ricevuto e scorre visivamente verso la casella.
public class Piece : MonoBehaviour
{
    const float SlideSpeed = 5f;

    public EntityData Data { get; private set; }
    public bool Mine { get; private set; }

    const float WalkFramesPerSecond = 8f;

    SpriteRenderer body, shadow, ring;
    Color baseColor;      // il colore della squadra (base sotto i piedi, territorio)
    Color bodyTint;       // com'e' tinto lo sprite: i personaggi e gli oggetti sono a colori veri, il resto in grigi
    string spriteName;    // lo sprite del personaggio (CharacterSprites), o null
    int facing = CharacterSprites.South;
    Vector3 target;
    float actReadyAt, respawnAt, readyAt;
    bool highlighted;
    float pingUntil;

    public bool Movable => Mine && (Data.kind == Kinds.Champion || Data.kind == Kinds.Minor);
    public bool IsUnit => Data.kind == Kinds.Champion || Data.kind == Kinds.Minor || Data.kind == Kinds.Npc;
    public bool Defeated => IsUnit && Data.health <= 0;
    public float SecondsUntilReady => Mathf.Max(0f, readyAt - Time.time);
    public float SecondsUntilActReady => Mathf.Max(0f, actReadyAt - Time.time);
    public float SecondsUntilRespawn => Mathf.Max(0f, respawnAt - Time.time);

    public Color TeamColor => baseColor;
    public Sprite Icon => body.sprite;
    public Color IconTint => bodyTint;

    // La classe della pedina (ruolo), come la mostra la scheda.
    public string ClassLabel
    {
        get
        {
            switch (Data.kind)
            {
                case Kinds.Champion: return "Campione";
                case Kinds.Minor: return "Pedina";
                case Kinds.Npc: return "NPC";
                case Kinds.Structure: return "Struttura";
                default: return "Oggetto";
            }
        }
    }

    // Fa lampeggiare l'anello attorno alla pedina, per farla trovare sulla mappa.
    public void Ping(float seconds = 3f) => pingUntil = Time.time + seconds;

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
        // I personaggi hanno i loro sprite a colori e la squadra si vede dalla base sotto i piedi; gli
        // oggetti mostrano la loro icona; il resto (strutture) e' in grigi, tinto per squadra.
        spriteName = CharacterSprites.NameFor(data);
        if (spriteName != null && !CharacterSprites.Has(spriteName)) spriteName = null;
        Sprite first;
        if (spriteName != null) { first = CharacterSprites.Idle(spriteName, facing); bodyTint = Color.white; }
        else if (data.kind == Kinds.Item) { first = PixelArt.ItemSprite(data.icon); bodyTint = Color.white; }
        else { first = PixelArt.Piece(data.kind); bodyTint = color; }
        body = NewRenderer("Body", first);
        if (spriteName != null && data.kind != Kinds.Npc)
        {
            shadow.sprite = PixelArt.TeamBase;
            shadow.color = new Color(color.r, color.g, color.b, 0.85f);
        }
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
        target = GridMath.CellToWorld(data.x, data.y) + new Vector2(0f, GridMath.FeetOffset);
        if (changedBoard) transform.position = target;
        readyAt = Time.time + data.ready_in_ms / 1000f;
        actReadyAt = Time.time + data.act_ready_in_ms / 1000f;
        respawnAt = Time.time + data.respawn_in_ms / 1000f;
        RefreshLook();
    }

    void Update()
    {
        var before = transform.position;
        transform.position = Vector3.MoveTowards(transform.position, target, SlideSpeed * Time.deltaTime);
        UpdateSorting();
        UpdateSprite(transform.position - before);
        bool pinging = Time.time < pingUntil && (int)(Time.time * 4f) % 2 == 0;
        ring.enabled = highlighted || pinging;
    }

    // Il personaggio si gira dove va e, mentre si muove, cammina; da fermo sta nella posa verso cui guarda.
    void UpdateSprite(Vector3 step)
    {
        if (spriteName == null) return;
        bool moving = step.sqrMagnitude > 1e-8f;
        if (moving)
        {
            facing = Mathf.Abs(step.x) > Mathf.Abs(step.y)
                ? (step.x > 0f ? CharacterSprites.East : CharacterSprites.West)
                : (step.y > 0f ? CharacterSprites.North : CharacterSprites.South);
        }
        body.sprite = moving && CharacterSprites.CanWalk(spriteName)
            ? CharacterSprites.Walk(spriteName, facing, (int)(Time.time * WalkFramesPerSecond))
            : CharacterSprites.Idle(spriteName, facing);
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
        body.color = down ? Color.Lerp(bodyTint, new Color(0.1f, 0.1f, 0.1f), 0.75f) : bodyTint;
        body.transform.localRotation = Quaternion.Euler(0f, 0f, down ? 90f : 0f);
        body.transform.localPosition = down ? new Vector3(0.12f, 0f, 0f) : Vector3.zero;
        ring.enabled = highlighted;
    }
}
