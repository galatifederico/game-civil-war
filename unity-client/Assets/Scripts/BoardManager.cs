using System.Collections.Generic;
using UnityEngine;

// Mostra la board e le pedine che il server descrive (snapshot + delta) e traduce i click
// in comandi: nulla cambia finche' il server non conferma.
//
// Click su una tua pedina: la seleziona (poi una casella vuota la sposta). Click su una pedina
// non tua, un NPC o un oggetto: ne mostra la scheda e, se hai una pedina selezionata, le azioni
// che quella pedina puo' fare sul bersaglio (attacca, parla, raccogli).
public class BoardManager : MonoBehaviour
{
    public const float CellSize = 1f;
    const float BoardTop = 0.05f;
    static readonly Color LightSquare = new Color(0.85f, 0.85f, 0.75f);
    static readonly Color DarkSquare = new Color(0.35f, 0.25f, 0.2f);
    static readonly Color MyTerritory = new Color(0.3f, 0.85f, 1f);

    public static BoardManager Instance { get; private set; }

    NetworkClient net;
    InfoPanel panel;
    Hud hud;
    Camera cam;

    readonly Dictionary<string, Piece> pieces = new Dictionary<string, Piece>();
    readonly List<GameObject> squareObjects = new List<GameObject>();
    Square[,] grid;
    bool[,] visibleCells;
    bool[,] hasTint;
    Color[,] tint;
    MaterialPropertyBlock paint;
    Material lightMaterial, darkMaterial;
    Piece selected;
    Transform rangeOutline;
    readonly Transform[] rangeBars = new Transform[4];
    enum ClickMode { Move, Build, PushItem }
    ClickMode mode = ClickMode.Move;
    Piece pushedItem;
    string myPlayerId;
    int width, height;
    float fittedAspect;

    public static Vector3 CellToWorld(int x, int y, float height) => new Vector3(x * CellSize, height, y * CellSize);

    void Awake()
    {
        Instance = this;
        paint = new MaterialPropertyBlock();
    }

    public void Init(NetworkClient network, InfoPanel infoPanel, Hud hud, Camera camera)
    {
        net = network;
        panel = infoPanel;
        this.hud = hud;
        cam = camera;
    }

    bool PointerBlocked => panel.BlocksPointer || hud.BlocksPointer;

    public void LoadSnapshot(ServerMessage snapshot)
    {
        Clear();
        myPlayerId = snapshot.your_player_id;
        width = snapshot.board.width;
        height = snapshot.board.height;
        BuildSquares();
        FitCamera();
        foreach (var e in snapshot.entities) Spawn(e);
        RefreshFog(force: true);
    }

    public void ApplyDelta(ServerMessage delta)
    {
        foreach (var e in delta.entities ?? new EntityData[0])
        {
            if (pieces.TryGetValue(e.id, out var piece)) piece.Apply(e);
            else Spawn(e);
        }
        foreach (var id in delta.removed ?? new string[0])
        {
            if (!pieces.TryGetValue(id, out var piece)) continue;
            if (piece == selected) ClearSelection();
            if (panel.Current == piece) panel.Hide();
            if (piece.Data.kind == Kinds.Structure) SetTint(piece.Data.x, piece.Data.y, false, default);
            pieces.Remove(id);
            Destroy(piece.gameObject);
        }
        RefreshFog(force: false);
    }

    public void Clear()
    {
        ClearSelection();
        foreach (var p in pieces.Values) Destroy(p.gameObject);
        pieces.Clear();
        foreach (var s in squareObjects) Destroy(s);
        squareObjects.Clear();
    }

    void BuildSquares()
    {
        if (lightMaterial == null)
        {
            lightMaterial = NewMaterial(LightSquare);
            darkMaterial = NewMaterial(DarkSquare);
        }
        grid = new Square[width, height];
        visibleCells = new bool[width, height];
        hasTint = new bool[width, height];
        tint = new Color[width, height];
        for (int x = 0; x < width; x++)
        {
            for (int y = 0; y < height; y++)
            {
                var cube = GameObject.CreatePrimitive(PrimitiveType.Cube);
                cube.name = $"Square_{x}_{y}";
                cube.transform.SetParent(transform);
                cube.transform.position = CellToWorld(x, y, 0f);
                cube.transform.localScale = new Vector3(CellSize * 0.95f, 0.1f, CellSize * 0.95f);
                cube.GetComponent<Renderer>().sharedMaterial = (x + y) % 2 == 0 ? lightMaterial : darkMaterial;
                var square = cube.AddComponent<Square>();
                square.X = x;
                square.Z = y;
                grid[x, y] = square;
                squareObjects.Add(cube);
            }
        }
    }

    // Stesso shader delle primitive: funziona con qualunque render pipeline del progetto.
    static Material NewMaterial(Color color)
    {
        var probe = GameObject.CreatePrimitive(PrimitiveType.Cube);
        var material = new Material(probe.GetComponent<Renderer>().sharedMaterial) { color = color };
        Destroy(probe);
        material.SetFloat("_Glossiness", 0f);
        return material;
    }

    // Il territorio si vede: la casella sotto una struttura prende il colore della squadra.
    void SetTint(int x, int y, bool on, Color owner)
    {
        hasTint[x, y] = on;
        tint[x, y] = owner;
        PaintSquare(x, y);
    }

    // Nebbia di guerra: le caselle fuori dalla vista delle mie pedine in gioco sono scurite.
    // Il server manda solo cio' che vedo; qui si limita a mostrarlo.
    void RefreshFog(bool force)
    {
        var now = new bool[width, height];
        foreach (var p in pieces.Values)
        {
            if (!p.Movable || p.Defeated) continue;
            int r = p.Data.vision;
            for (int x = Mathf.Max(0, p.Data.x - r); x <= Mathf.Min(width - 1, p.Data.x + r); x++)
                for (int y = Mathf.Max(0, p.Data.y - r); y <= Mathf.Min(height - 1, p.Data.y + r); y++)
                    now[x, y] = true;
        }
        for (int x = 0; x < width; x++)
        {
            for (int y = 0; y < height; y++)
            {
                if (!force && now[x, y] == visibleCells[x, y]) continue;
                visibleCells[x, y] = now[x, y];
                PaintSquare(x, y);
            }
        }
    }

    void PaintSquare(int x, int y)
    {
        var color = (x + y) % 2 == 0 ? LightSquare : DarkSquare;
        if (hasTint[x, y]) color = Color.Lerp(color, tint[x, y], 0.55f);
        if (!visibleCells[x, y]) color *= 0.4f;
        color.a = 1f;
        paint.SetColor("_Color", color);
        grid[x, y].GetComponent<Renderer>().SetPropertyBlock(paint);
    }

    void FitCamera()
    {
        fittedAspect = cam.aspect;
        cam.orthographicSize = Mathf.Max(height * CellSize * 0.5f + 1f, (width * CellSize * 0.5f + 1f) / cam.aspect);
        cam.transform.position = new Vector3((width - 1) * CellSize / 2f, 10f, (height - 1) * CellSize / 2f);
    }

    void LateUpdate()
    {
        if (width > 0 && !Mathf.Approximately(fittedAspect, cam.aspect)) FitCamera();
        UpdateRangeOutline();
    }

    // Contorno della portata (la "vista") della pedina selezionata: e' il raggio delle sue azioni.
    void UpdateRangeOutline()
    {
        if (rangeOutline == null)
        {
            rangeOutline = new GameObject("RangeOutline").transform;
            rangeOutline.SetParent(transform);
            var material = NewMaterial(new Color(0.4f, 0.9f, 1f));
            for (int i = 0; i < rangeBars.Length; i++)
            {
                var bar = GameObject.CreatePrimitive(PrimitiveType.Cube);
                bar.GetComponent<Collider>().enabled = false;
                bar.GetComponent<Renderer>().sharedMaterial = material;
                bar.transform.SetParent(rangeOutline);
                rangeBars[i] = bar.transform;
            }
        }

        bool show = selected != null && !selected.Defeated;
        rangeOutline.gameObject.SetActive(show);
        if (!show) return;

        const float thickness = 0.08f, y = 0.11f;
        float half = (selected.Data.vision + 0.5f) * CellSize;
        float side = half * 2f + thickness;
        var c = selected.transform.position;
        rangeBars[0].position = new Vector3(c.x, y, c.z + half);
        rangeBars[1].position = new Vector3(c.x, y, c.z - half);
        rangeBars[2].position = new Vector3(c.x + half, y, c.z);
        rangeBars[3].position = new Vector3(c.x - half, y, c.z);
        rangeBars[0].localScale = rangeBars[1].localScale = new Vector3(side, thickness, thickness);
        rangeBars[2].localScale = rangeBars[3].localScale = new Vector3(thickness, thickness, side);
    }

    void Spawn(EntityData e)
    {
        bool mine = !string.IsNullOrEmpty(e.owner_id) && e.owner_id == myPlayerId;
        Look(e, mine, out var shape, out var scale, out var halfHeight, out var color);

        var go = GameObject.CreatePrimitive(shape);
        go.name = $"{e.kind}_{e.name}";
        go.transform.SetParent(transform);
        go.transform.localScale = scale;
        go.transform.position = CellToWorld(e.x, e.y, BoardTop + scale.y * halfHeight);

        var piece = go.AddComponent<Piece>();
        piece.Init(e, mine, color);
        pieces[e.id] = piece;

        if (e.kind == Kinds.Structure) SetTint(e.x, e.y, true, color);
    }

    static void Look(EntityData e, bool mine, out PrimitiveType shape, out Vector3 scale, out float halfHeight, out Color color)
    {
        switch (e.kind)
        {
            case Kinds.Champion:
                shape = PrimitiveType.Cylinder; scale = new Vector3(0.7f, 0.55f, 0.7f); halfHeight = 1f;
                color = mine ? new Color(1f, 0.8f, 0.15f) : OwnerColor(e.owner_id, 0.95f);
                break;
            case Kinds.Minor:
                shape = PrimitiveType.Capsule; scale = new Vector3(0.55f, 0.45f, 0.55f); halfHeight = 1f;
                color = mine ? new Color(0.2f, 0.45f, 1f) : OwnerColor(e.owner_id, 0.75f);
                break;
            case Kinds.Npc:
                shape = PrimitiveType.Sphere; scale = new Vector3(0.6f, 0.6f, 0.6f); halfHeight = 0.5f;
                color = new Color(0.7f, 0.3f, 0.85f);
                break;
            case Kinds.Structure:
                shape = PrimitiveType.Cube; scale = new Vector3(0.75f, 0.9f, 0.75f); halfHeight = 0.5f;
                color = mine ? MyTerritory : OwnerColor(e.owner_id, 0.7f);
                break;
            default:
                shape = PrimitiveType.Cube; scale = new Vector3(0.55f, 0.4f, 0.55f); halfHeight = 0.5f;
                color = new Color(0.45f, 0.75f, 0.5f);
                break;
        }
    }

    // Ogni squadra avversaria ha il proprio colore, stabile tra un avvio e l'altro.
    static Color OwnerColor(string ownerId, float value)
    {
        int hash = 17;
        foreach (char c in ownerId) hash = hash * 31 + c;
        float hue = (hash & 0xFFFF) / 65535f;
        // Il blu e' riservato alla tua squadra: salta la fascia di tinte vicine.
        hue = (hue * 0.55f + 0.85f) % 1f;
        return Color.HSVToRGB(hue, 0.7f, value);
    }

    public void OnPieceClicked(Piece piece)
    {
        if (PointerBlocked) return;

        if (piece == selected)
        {
            ClearSelection();
            panel.Hide();
            return;
        }

        SetMode(ClickMode.Move);
        if (piece.Movable)
        {
            ClearSelection();
            selected = piece;
            piece.SetHighlight(true);
        }
        else if (selected == null || piece.Mine)
        {
            ClearSelection();
        }
        // Altrimenti la pedina selezionata resta tale: il clic indica un bersaglio per le sue azioni.
        panel.Show(piece, ActionsFor(piece));
    }

    public void OnSquareClicked(Square square)
    {
        if (PointerBlocked) return;

        if (selected == null)
        {
            panel.Hide();
            return;
        }
        switch (mode)
        {
            case ClickMode.Build:
                net.SendCommand("build", selected.Data.id, null, square.X, square.Z);
                SetMode(ClickMode.Move);
                panel.Show(selected, ActionsFor(selected));
                break;
            case ClickMode.PushItem:
                net.SendCommand("move_item", selected.Data.id, pushedItem.Data.id, square.X, square.Z);
                var item = pushedItem;
                SetMode(ClickMode.Move);
                panel.Show(item, ActionsFor(item));
                break;
            default:
                net.SendMove(selected.Data.id, square.X, square.Z);
                break;
        }
    }

    public void ClearSelection()
    {
        SetMode(ClickMode.Move);
        if (selected != null) selected.SetHighlight(false);
        selected = null;
    }

    // Dopo "Costruisci" o "Sposta" il clic successivo su una casella non muove la pedina:
    // e' la destinazione dell'azione, e la scritta in alto lo ricorda.
    void SetMode(ClickMode newMode, Piece item = null)
    {
        mode = newMode;
        pushedItem = item;
        switch (newMode)
        {
            case ClickMode.Build: hud.SetHint("Costruzione: clicca una casella libera"); break;
            case ClickMode.PushItem: hud.SetHint($"Sposta {item.Data.name}: clicca una casella libera"); break;
            default: hud.SetHint(""); break;
        }
    }

    List<PanelAction> ActionsFor(Piece piece)
    {
        var list = new List<PanelAction>();
        var actor = selected;
        if (actor == null) return list;

        if (piece == actor)
        {
            list.Add(new PanelAction
            {
                Label = mode == ClickMode.Build ? "Annulla costruzione" : "Costruisci avamposto",
                Perform = () =>
                {
                    SetMode(mode == ClickMode.Build ? ClickMode.Move : ClickMode.Build);
                    panel.Show(actor, ActionsFor(actor));
                },
                BlockedReason = () => BlockReason(actor, null, needsReady: true),
            });
            if (actor.Data.kind == Kinds.Champion)
            {
                list.Add(new PanelAction
                {
                    Label = "Crea pedina (costa vita)",
                    Perform = () => net.SendCommand("create", actor.Data.id, null),
                    BlockedReason = () => BlockReason(actor, null, needsReady: true),
                });
            }
            return list;
        }
        if (piece.Mine) return list;

        switch (piece.Data.kind)
        {
            case Kinds.Npc:
                list.Add(Command("Parla", "talk", actor, piece, needsReady: false));
                break;
            case Kinds.Item:
                list.Add(Command("Raccogli", "pickup", actor, piece, needsReady: true));
                list.Add(new PanelAction
                {
                    Label = mode == ClickMode.PushItem && pushedItem == piece ? "Annulla spostamento" : "Sposta",
                    Perform = () =>
                    {
                        bool cancel = mode == ClickMode.PushItem && pushedItem == piece;
                        SetMode(cancel ? ClickMode.Move : ClickMode.PushItem, piece);
                        panel.Show(piece, ActionsFor(piece));
                    },
                    BlockedReason = () => BlockReason(actor, piece, needsReady: true),
                });
                break;
            case Kinds.Champion:
            case Kinds.Minor:
                var attack = Command("Attacca", "attack", actor, piece, needsReady: true);
                var inRange = attack.BlockedReason;
                attack.BlockedReason = () => piece.Defeated ? "già fuori gioco" : inRange();
                list.Add(attack);
                break;
        }
        return list;
    }

    PanelAction Command(string label, string type, Piece actor, Piece target, bool needsReady)
    {
        return new PanelAction
        {
            Label = label,
            Perform = () => net.SendCommand(type, actor.Data.id, target.Data.id),
            BlockedReason = () => BlockReason(actor, target, needsReady),
        };
    }

    // Anticipa gli errori piu' comuni; il server resta comunque l'unico a decidere.
    static string BlockReason(Piece actor, Piece target, bool needsReady)
    {
        if (actor.Defeated) return "fuori gioco";
        if (target != null)
        {
            int dist = Mathf.Max(Mathf.Abs(actor.Data.x - target.Data.x), Mathf.Abs(actor.Data.y - target.Data.y));
            if (dist > actor.Data.vision) return "fuori portata";
        }
        if (needsReady && actor.SecondsUntilActReady > 0f) return $"pronta tra {actor.SecondsUntilActReady:0.0}s";
        return null;
    }
}
