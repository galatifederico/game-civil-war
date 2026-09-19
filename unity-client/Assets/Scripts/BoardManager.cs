using System.Collections.Generic;
using UnityEngine;

// Mostra il mondo che il server descrive (snapshot + delta) e traduce i clic in comandi: nulla
// cambia finche' il server non conferma. La vista e' isometrica 2D e mostra una board alla volta
// (le altre si raggiungono passando dai gateway o con le schede in alto).
//
// Clic su una tua pedina: la seleziona (poi una casella vuota la sposta). Clic su una pedina
// non tua, un NPC o un oggetto: ne mostra la scheda e, se hai una pedina selezionata, le azioni
// che quella pedina puo' fare sul bersaglio (attacca, parla, raccogli, sposta).
public class BoardManager : MonoBehaviour
{
    static readonly Color LightTile = new Color(0.86f, 0.84f, 0.74f);
    static readonly Color DarkTile = new Color(0.55f, 0.42f, 0.34f);
    static readonly Color MyTerritory = new Color(0.3f, 0.85f, 1f);
    static readonly Color PortalTint = new Color(0.8f, 0.55f, 1f);
    static readonly Color RangeTint = new Color(0.4f, 0.9f, 1f);

    public static BoardManager Instance { get; private set; }

    NetworkClient net;
    InfoPanel panel;
    Hud hud;
    Camera cam;

    readonly Dictionary<string, BoardData> boards = new Dictionary<string, BoardData>();
    readonly List<string> boardOrder = new List<string>();
    BoardData current;

    readonly Dictionary<string, Piece> pieces = new Dictionary<string, Piece>();
    readonly List<GameObject> tileObjects = new List<GameObject>();
    SpriteRenderer[,] tiles;
    bool[,] visibleCells, hasTint, inRange, isGateway;
    Color[,] tint;

    enum ClickMode { Move, Build, PushItem }
    ClickMode mode = ClickMode.Move;
    Piece selected;
    Piece pushedItem;
    string rangeKey = "";
    string myPlayerId;
    float fittedAspect;

    public string CurrentBoardId => current?.id;

    public string GridOf(string boardId) => boards.TryGetValue(boardId ?? "", out var b) ? b.grid : GridMath.Square;

    // Piu' in basso sullo schermo = piu' vicino = disegnato sopra; il margine lascia posto agli strati.
    public static int SortOrder(float worldY) => -Mathf.RoundToInt(worldY * 100f) * 4;

    void Awake() => Instance = this;

    public void Init(NetworkClient network, InfoPanel infoPanel, Hud hud, Camera camera)
    {
        net = network;
        panel = infoPanel;
        this.hud = hud;
        cam = camera;
    }

    bool PointerBlocked => panel.BlocksPointer || hud.BlocksPointer;

    // ---- Snapshot e aggiornamenti ------------------------------------------------------------

    public void LoadSnapshot(ServerMessage snapshot)
    {
        Clear();
        myPlayerId = snapshot.your_player_id;
        foreach (var b in snapshot.boards)
        {
            boards[b.id] = b;
            boardOrder.Add(b.id);
        }
        foreach (var e in snapshot.entities) Spawn(e);

        // Si parte dalla board dove sta il proprio campione.
        string start = boardOrder[0];
        foreach (var p in pieces.Values)
            if (p.Mine && p.Data.kind == Kinds.Champion) start = p.Data.board_id;
        ShowBoard(start);
    }

    public void ApplyDelta(ServerMessage delta)
    {
        foreach (var e in delta.entities ?? new EntityData[0])
        {
            if (pieces.TryGetValue(e.id, out var piece))
            {
                piece.Apply(e);
                piece.gameObject.SetActive(e.board_id == current.id);
                // La pedina selezionata ha cambiato board (un passaggio): la vista la segue.
                if (piece == selected && e.board_id != current.id) ShowBoard(e.board_id);
            }
            else Spawn(e);
        }
        foreach (var id in delta.removed ?? new string[0])
        {
            if (!pieces.TryGetValue(id, out var piece)) continue;
            if (piece == selected) ClearSelection();
            if (panel.Current == piece) panel.Hide();
            if (piece.Data.kind == Kinds.Structure && piece.Data.board_id == current.id)
                SetTint(piece.Data.x, piece.Data.y, false, default);
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
        DestroyTiles();
        boards.Clear();
        boardOrder.Clear();
        current = null;
    }

    void Spawn(EntityData e)
    {
        bool mine = !string.IsNullOrEmpty(e.owner_id) && e.owner_id == myPlayerId;
        var go = new GameObject($"{e.kind}_{e.name}");
        go.transform.SetParent(transform);
        var piece = go.AddComponent<Piece>();
        piece.Init(e, mine, PieceColor(e, mine));
        pieces[e.id] = piece;

        bool onCurrent = current != null && e.board_id == current.id;
        go.SetActive(onCurrent);
        if (onCurrent && e.kind == Kinds.Structure) SetTint(e.x, e.y, true, PieceColor(e, mine));
    }

    static Color PieceColor(EntityData e, bool mine)
    {
        switch (e.kind)
        {
            case Kinds.Champion: return mine ? new Color(1f, 0.8f, 0.15f) : OwnerColor(e.owner_id, 0.95f);
            case Kinds.Minor: return mine ? new Color(0.25f, 0.5f, 1f) : OwnerColor(e.owner_id, 0.8f);
            case Kinds.Npc: return new Color(0.75f, 0.4f, 0.9f);
            case Kinds.Structure: return mine ? MyTerritory : OwnerColor(e.owner_id, 0.75f);
            default: return new Color(0.8f, 0.6f, 0.35f);
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

    // ---- Board e caselle ------------------------------------------------------------------

    public void ShowBoard(string boardId)
    {
        if (!boards.TryGetValue(boardId, out var board)) return;
        current = board;
        SetMode(ClickMode.Move);
        rangeKey = "";
        BuildTiles();
        foreach (var p in pieces.Values)
        {
            bool here = p.Data.board_id == board.id;
            p.gameObject.SetActive(here);
            if (here && p.Data.kind == Kinds.Structure) SetTint(p.Data.x, p.Data.y, true, PieceColor(p.Data, p.Mine));
        }
        FitCamera();
        RefreshFog(force: true);
        panel.Hide();

        var names = new string[boardOrder.Count];
        for (int i = 0; i < names.Length; i++) names[i] = boards[boardOrder[i]].name;
        hud.SetBoards(names, boardOrder.IndexOf(board.id), i => ShowBoard(boardOrder[i]));
    }

    void DestroyTiles()
    {
        foreach (var t in tileObjects) Destroy(t);
        tileObjects.Clear();
        tiles = null;
    }

    void BuildTiles()
    {
        DestroyTiles();
        int w = current.width, h = current.height;
        tiles = new SpriteRenderer[w, h];
        visibleCells = new bool[w, h];
        hasTint = new bool[w, h];
        inRange = new bool[w, h];
        isGateway = new bool[w, h];
        tint = new Color[w, h];
        foreach (var g in current.gateways ?? new GatewayData[0]) isGateway[g.x, g.y] = true;

        var sprite = PixelArt.Tile(current.grid);
        for (int x = 0; x < w; x++)
        {
            for (int y = 0; y < h; y++)
            {
                var go = new GameObject($"Tile_{x}_{y}");
                go.transform.SetParent(transform);
                var centre = GridMath.CellToWorld(current.grid, x, y);
                go.transform.position = centre;
                var r = go.AddComponent<SpriteRenderer>();
                r.sprite = sprite;
                r.sortingOrder = SortOrder(centre.y);
                tiles[x, y] = r;
                tileObjects.Add(go);

                if (isGateway[x, y])
                {
                    var marker = new GameObject("Portal");
                    marker.transform.SetParent(go.transform, false);
                    var mr = marker.AddComponent<SpriteRenderer>();
                    mr.sprite = PixelArt.Portal;
                    mr.sortingOrder = r.sortingOrder + 1;
                }
                PaintTile(x, y);
            }
        }
    }

    bool InBounds(int x, int y) => current != null && x >= 0 && y >= 0 && x < current.width && y < current.height;

    void PaintTile(int x, int y)
    {
        var color = (x + y) % 2 == 0 ? LightTile : DarkTile;
        if (isGateway[x, y]) color = Color.Lerp(color, PortalTint, 0.6f);
        if (hasTint[x, y]) color = Color.Lerp(color, tint[x, y], 0.55f);
        if (inRange[x, y]) color = Color.Lerp(color, RangeTint, 0.4f);
        if (!visibleCells[x, y]) color *= 0.4f;
        color.a = 1f;
        tiles[x, y].color = color;
    }

    // Il territorio si vede: la casella sotto una struttura prende il colore della squadra.
    void SetTint(int x, int y, bool on, Color owner)
    {
        if (!InBounds(x, y)) return;
        hasTint[x, y] = on;
        tint[x, y] = owner;
        PaintTile(x, y);
    }

    // Nebbia di guerra: le caselle fuori dalla vista delle mie pedine in gioco sono scurite.
    // Il server manda solo cio' che vedo; qui si limita a mostrarlo.
    void RefreshFog(bool force)
    {
        if (current == null) return;
        var now = new bool[current.width, current.height];
        foreach (var p in pieces.Values)
        {
            if (!p.Movable || p.Defeated || p.Data.board_id != current.id) continue;
            int r = p.Data.vision;
            for (int x = Mathf.Max(0, p.Data.x - r); x <= Mathf.Min(current.width - 1, p.Data.x + r); x++)
                for (int y = Mathf.Max(0, p.Data.y - r); y <= Mathf.Min(current.height - 1, p.Data.y + r); y++)
                    if (GridMath.Distance(current.grid, p.Data.x, p.Data.y, x, y) <= r) now[x, y] = true;
        }
        for (int x = 0; x < current.width; x++)
        {
            for (int y = 0; y < current.height; y++)
            {
                if (!force && now[x, y] == visibleCells[x, y]) continue;
                visibleCells[x, y] = now[x, y];
                PaintTile(x, y);
            }
        }
    }

    // La portata (la "vista") della pedina selezionata e' il raggio delle sue azioni: si evidenzia.
    void RefreshRange()
    {
        if (current == null) return;
        bool show = selected != null && !selected.Defeated && selected.Data.board_id == current.id;
        string key = show ? $"{current.id}:{selected.Data.x}:{selected.Data.y}:{selected.Data.vision}" : "";
        if (key == rangeKey) return;
        rangeKey = key;
        for (int x = 0; x < current.width; x++)
        {
            for (int y = 0; y < current.height; y++)
            {
                bool now = show && GridMath.Distance(current.grid, selected.Data.x, selected.Data.y, x, y) <= selected.Data.vision;
                if (now == inRange[x, y]) continue;
                inRange[x, y] = now;
                PaintTile(x, y);
            }
        }
    }

    // Sui display larghi la fascia di destra e' riservata al menu della pedina: la board si inquadra
    // nello spazio che resta, cosi' non finisce mai sotto il menu.
    const float PanelStripWidth = 310f;

    void FitCamera()
    {
        float width = Screen.width;
        cam.rect = new Rect(0f, 0f, width > 700f ? (width - PanelStripWidth) / width : 1f, 1f);
        fittedAspect = cam.aspect;
        var bounds = GridMath.Bounds(current.grid, current.width, current.height);
        cam.orthographicSize = Mathf.Max(bounds.height / 2f, bounds.width / 2f / cam.aspect) * 1.03f;
        cam.transform.position = new Vector3(bounds.center.x, bounds.center.y, -10f);
    }

    void LateUpdate()
    {
        if (current == null) return;
        if (!Mathf.Approximately(fittedAspect, cam.aspect)) FitCamera();
        RefreshRange();
    }

    // ---- Input --------------------------------------------------------------------------------

    void Update()
    {
        if (current == null || !Input.GetMouseButtonDown(0) || PointerBlocked) return;
        var world = cam.ScreenToWorldPoint(Input.mousePosition);
        var cell = GridMath.WorldToCell(current.grid, world);
        ClickCell(cell.x, cell.y);
    }

    // Un clic (o un tocco) su una casella della board mostrata: se c'e' una pedina e' un clic su di lei.
    public void ClickCell(int x, int y)
    {
        if (!InBounds(x, y)) return;
        var piece = PieceAt(x, y);
        if (piece != null) OnPieceClicked(piece);
        else OnCellClicked(x, y);
    }

    Piece PieceAt(int x, int y)
    {
        foreach (var p in pieces.Values)
            if (p.gameObject.activeSelf && p.Data.board_id == current.id && p.Data.x == x && p.Data.y == y) return p;
        return null;
    }

    public void OnPieceClicked(Piece piece)
    {
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

    void OnCellClicked(int x, int y)
    {
        if (selected == null)
        {
            panel.Hide();
            return;
        }
        if (selected.Data.board_id != current.id)
        {
            hud.ShowToast("La pedina selezionata e' su un'altra board");
            return;
        }
        switch (mode)
        {
            case ClickMode.Build:
                net.SendCommand("build", selected.Data.id, null, x, y);
                SetMode(ClickMode.Move);
                panel.Show(selected, ActionsFor(selected));
                break;
            case ClickMode.PushItem:
                net.SendCommand("move_item", selected.Data.id, pushedItem.Data.id, x, y);
                var item = pushedItem;
                SetMode(ClickMode.Move);
                panel.Show(item, ActionsFor(item));
                break;
            default:
                net.SendMove(selected.Data.id, x, y);
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

    // ---- Azioni nel menu ------------------------------------------------------------------------

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
                var inReach = attack.BlockedReason;
                attack.BlockedReason = () => piece.Defeated ? "già fuori gioco" : inReach();
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
    string BlockReason(Piece actor, Piece target, bool needsReady)
    {
        if (actor.Defeated) return "fuori gioco";
        if (target != null)
        {
            if (target.Data.board_id != actor.Data.board_id) return "su un'altra board";
            int dist = GridMath.Distance(GridOf(actor.Data.board_id), actor.Data.x, actor.Data.y, target.Data.x, target.Data.y);
            if (dist > actor.Data.vision) return "fuori portata";
        }
        if (needsReady && actor.SecondsUntilActReady > 0f) return $"pronta tra {actor.SecondsUntilActReady:0.0}s";
        return null;
    }
}
