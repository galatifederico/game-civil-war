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
    static readonly Color MyTerritory = new Color(0.3f, 0.85f, 1f);
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
    List<SpriteRenderer>[,] decor; // alberi, cespugli, staccionate...: si scuriscono con la nebbia come il suolo
    bool[,] visibleCells, hasTint, inRange, isGateway;
    Color[,] tint;

    enum ClickMode { Move, Build, PushItem, Breed }
    ClickMode mode = ClickMode.Move;
    Piece selected;
    Piece pushedItem;
    string rangeKey = "";
    string myPlayerId;
    Vector2Int fittedScreen;
    bool fittedCompact;

    public string CurrentBoardId => current?.id;

    // Piu' in basso sullo schermo = piu' vicino = disegnato sopra; il margine lascia posto agli strati.
    public static int SortOrder(float worldY) => -Mathf.RoundToInt(worldY * 100f) * 4;

    void Awake() => Instance = this;

    public void Init(NetworkClient network, InfoPanel infoPanel, Hud hud, Camera camera)
    {
        net = network;
        panel = infoPanel;
        this.hud = hud;
        cam = camera;
        hud.ItemUseRequested += UseItemFromInventory;
        hud.ZoomRequested += ZoomFromButton;
    }

    // "Usa" nell'inventario: solo il campione usa gli oggetti, quindi e' lui che li usa.
    void UseItemFromInventory(ItemData item)
    {
        Piece champion = null;
        foreach (var p in pieces.Values)
            if (p.Mine && p.Data.kind == Kinds.Champion) champion = p;
        if (champion == null) hud.ShowToast("Non hai un campione");
        else if (champion.Defeated) hud.ShowToast("Il campione e' fuori gioco");
        else net.SendCommand("use_item", champion.Data.id, item.id);
    }

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
        decor = null;
    }

    // Il suolo sta sotto a tutto; alberi, muri e pedine si ordinano per altezza sullo schermo.
    const int GroundOrder = -32000;

    void BuildTiles()
    {
        DestroyTiles();
        int w = current.width, h = current.height;
        tiles = new SpriteRenderer[w, h];
        decor = new List<SpriteRenderer>[w, h];
        visibleCells = new bool[w, h];
        hasTint = new bool[w, h];
        inRange = new bool[w, h];
        isGateway = new bool[w, h];
        tint = new Color[w, h];
        foreach (var g in current.gateways ?? new GatewayData[0]) isGateway[g.x, g.y] = true;

        var rows = current.terrain;
        for (int x = 0; x < w; x++)
        {
            for (int y = 0; y < h; y++)
            {
                var go = new GameObject($"Tile_{x}_{y}");
                go.transform.SetParent(transform);
                var centre = GridMath.CellToWorld(x, y);
                go.transform.position = centre;
                var r = go.AddComponent<SpriteRenderer>();
                r.sprite = TileSheet.Tile(TerrainArt.Ground(rows, x, y));
                r.sortingOrder = GroundOrder;
                tiles[x, y] = r;
                tileObjects.Add(go);

                if (isGateway[x, y])
                {
                    var marker = new GameObject("Portal");
                    marker.transform.SetParent(go.transform, false);
                    var mr = marker.AddComponent<SpriteRenderer>();
                    mr.sprite = PixelArt.Portal;
                    mr.sortingOrder = GroundOrder + 1;
                    (decor[x, y] = decor[x, y] ?? new List<SpriteRenderer>()).Add(mr);
                }
                if (TerrainArt.Decoration(rows, x, y, out int baseTile, out int topTile, out _))
                {
                    var list = decor[x, y] = decor[x, y] ?? new List<SpriteRenderer>();
                    int order = SortOrder(centre.y + GridMath.FeetOffset);
                    list.Add(AddDecor(go.transform, baseTile, Vector3.zero, order));
                    // La cima di un albero sporge sulla casella sopra, ma sta dietro a chi ci cammina davanti.
                    if (topTile >= 0) list.Add(AddDecor(go.transform, topTile, new Vector3(0f, 1f, 0f), order + 1));
                }
                PaintTile(x, y);
            }
        }
    }

    static SpriteRenderer AddDecor(Transform parent, int tile, Vector3 offset, int order)
    {
        var go = new GameObject("Decor");
        go.transform.SetParent(parent, false);
        go.transform.localPosition = offset;
        var r = go.AddComponent<SpriteRenderer>();
        r.sprite = TileSheet.Tile(tile);
        r.sortingOrder = order;
        return r;
    }

    bool InBounds(int x, int y) => current != null && x >= 0 && y >= 0 && x < current.width && y < current.height;

    // Il colore moltiplica il tile: bianco = com'e', piu' scuro = nebbia, una punta di colore = territorio o portata.
    void PaintTile(int x, int y)
    {
        var color = Color.white;
        if (hasTint[x, y]) color = Color.Lerp(color, tint[x, y], 0.45f);
        if (inRange[x, y]) color = Color.Lerp(color, RangeTint, 0.35f);
        if (!visibleCells[x, y]) color *= 0.45f;
        color.a = 1f;
        tiles[x, y].color = color;
        if (decor[x, y] != null)
            foreach (var d in decor[x, y]) d.color = color;
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
                    if (GridMath.Distance(p.Data.x, p.Data.y, x, y) <= r) now[x, y] = true;
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
                bool now = show && GridMath.Distance(selected.Data.x, selected.Data.y, x, y) <= selected.Data.vision;
                if (now == inRange[x, y]) continue;
                inRange[x, y] = now;
                PaintTile(x, y);
            }
        }
    }

    // ---- Camera: tutto lo schermo, con zoom e trascinamento ---------------------------------------

    const float MinOrthoSize = 3f;       // massimo zoom: circa sei caselle in altezza
    const float MaxZoomOutSlack = 1.25f; // si puo' allontanare un po' oltre la board intera
    const float DragThresholdPixels = 8f;

    float fitSize = 6f;
    Rect boardBounds;
    Vector2 pressPosition, lastMouse;
    bool pressing, pressBlocked, dragged;

    const float StartOrthoSize = 6f; // vista iniziale: circa dodici caselle in altezza, attorno al campione

    // La mappa occupa tutto lo schermo; il menu le sta sopra a destra, quindi si inquadra lo spazio
    // che il menu lascia libero. All'inizio la vista e' ravvicinata sul campione (come nei giochi a
    // caselle); con wholeBoard si vede la board intera. Da li' si puo' zoomare e spostare.
    void FitCamera(bool wholeBoard = false)
    {
        cam.rect = new Rect(0f, 0f, 1f, 1f);
        fittedScreen = new Vector2Int(Screen.width, Screen.height);
        fittedCompact = Ui.Compact;
        boardBounds = GridMath.Bounds(current.width, current.height);

        float rightPixels = hud.OccupiedRightPixels;
        float freeAspect = Mathf.Max(0.2f, (Screen.width - rightPixels) / Screen.height);
        fitSize = Mathf.Max(boardBounds.height / 2f, boardBounds.width / 2f / freeAspect) * 1.03f;
        float size = wholeBoard ? fitSize : Mathf.Min(fitSize, StartOrthoSize);
        cam.orthographicSize = size;

        var focus = (Vector3)boardBounds.center;
        var champion = MyChampion();
        if (!wholeBoard && champion != null && champion.Data.board_id == current.id) focus = champion.transform.position;
        float worldPerPixel = 2f * size / Screen.height;
        cam.transform.position = new Vector3(focus.x + rightPixels / 2f * worldPerPixel, focus.y, -10f);
        MoveCamera(Vector3.zero);
    }

    // factor < 1 avvicina, > 1 allontana, 0 = torna a inquadrare la board intera.
    void ZoomFromButton(float factor)
    {
        if (current == null) return;
        if (factor <= 0f) FitCamera(wholeBoard: true);
        else ZoomAt(new Vector2(Screen.width / 2f, Screen.height / 2f), factor);
    }

    void ZoomAt(Vector2 screenPoint, float factor)
    {
        var before = cam.ScreenToWorldPoint(screenPoint);
        cam.orthographicSize = Mathf.Clamp(cam.orthographicSize * factor, MinOrthoSize, Mathf.Max(MinOrthoSize, fitSize * MaxZoomOutSlack));
        var after = cam.ScreenToWorldPoint(screenPoint);
        MoveCamera(before - after);
    }

    void PanByPixels(Vector2 pixels)
    {
        float worldPerPixel = 2f * cam.orthographicSize / Screen.height;
        MoveCamera(new Vector3(-pixels.x, -pixels.y, 0f) * worldPerPixel);
    }

    // Il centro della vista resta dentro la board: non ci si puo' perdere nel vuoto.
    void MoveCamera(Vector3 delta)
    {
        var p = cam.transform.position + delta;
        p.x = Mathf.Clamp(p.x, boardBounds.xMin, boardBounds.xMax);
        p.y = Mathf.Clamp(p.y, boardBounds.yMin, boardBounds.yMax);
        cam.transform.position = p;
    }

    void LateUpdate()
    {
        if (current == null) return;
        if (fittedScreen.x != Screen.width || fittedScreen.y != Screen.height || fittedCompact != Ui.Compact) FitCamera();
        RefreshRange();
    }

    // ---- Input --------------------------------------------------------------------------------

    // Rotella o pizzico = zoom; trascinare (tasto sinistro/destro/centrale o un dito) = sposta la vista;
    // un clic o un tocco breve, senza trascinare, sceglie la casella. Il clic scatta al rilascio.
    void Update()
    {
        if (current == null) return;
        Vector2 mouse = Input.mousePosition;

        if (Input.touchCount >= 2)
        {
            var a = Input.GetTouch(0);
            var b = Input.GetTouch(1);
            float previous = ((a.position - a.deltaPosition) - (b.position - b.deltaPosition)).magnitude;
            float now = (a.position - b.position).magnitude;
            if (previous > 1f && now > 1f) ZoomAt((a.position + b.position) / 2f, previous / now);
            dragged = true;
            lastMouse = mouse;
            return;
        }

        float wheel = Input.mouseScrollDelta.y;
        if (Mathf.Abs(wheel) > 0.01f && !hud.BlocksPointer) ZoomAt(mouse, Mathf.Pow(0.85f, wheel));

        if ((Input.GetMouseButton(1) || Input.GetMouseButton(2)) && !hud.BlocksPointer) PanByPixels(mouse - lastMouse);

        if (Input.GetMouseButtonDown(0))
        {
            pressing = true;
            pressBlocked = hud.BlocksPointer;
            dragged = false;
            pressPosition = mouse;
        }
        if (pressing && !pressBlocked && Input.GetMouseButton(0))
        {
            if (!dragged && (mouse - pressPosition).magnitude > DragThresholdPixels) dragged = true;
            if (dragged) PanByPixels(mouse - lastMouse);
        }
        if (pressing && Input.GetMouseButtonUp(0))
        {
            pressing = false;
            if (!pressBlocked && !dragged)
            {
                var cell = GridMath.WorldToCell(cam.ScreenToWorldPoint(mouse));
                ClickCell(cell.x, cell.y);
            }
        }
        lastMouse = mouse;
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
        if (mode == ClickMode.Breed && selected != null && piece != selected)
        {
            // Il partner della riproduzione e' un'altra pedina della squadra.
            net.SendCommand("breed", selected.Data.id, piece.Data.id);
            SetMode(ClickMode.Move);
            panel.Show(selected, ActionsFor(selected));
            return;
        }
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
            case ClickMode.Breed: hud.SetHint("Riproduzione: clicca l'altra pedina della squadra"); break;
            default: hud.SetHint(""); break;
        }
    }

    // ---- Squadra: elenco e inquadratura --------------------------------------------

    // Le unita' della propria squadra (campione per primo), anche quelle su un'altra board.
    public List<Piece> MyUnits()
    {
        var list = new List<Piece>();
        foreach (var p in pieces.Values)
            if (p.Mine && p.IsUnit) list.Add(p);
        list.Sort((a, b) =>
        {
            int ka = a.Data.kind == Kinds.Champion ? 0 : 1, kb = b.Data.kind == Kinds.Champion ? 0 : 1;
            if (ka != kb) return ka.CompareTo(kb);
            if (a.Data.name.Length != b.Data.name.Length) return a.Data.name.Length.CompareTo(b.Data.name.Length);
            return string.CompareOrdinal(a.Data.name, b.Data.name);
        });
        return list;
    }

    public string BoardName(string boardId) => boards.TryGetValue(boardId ?? "", out var b) ? b.name : "";

    // Mostra la pedina sulla mappa: cambia board se serve, la porta al centro dello spazio libero e
    // ne fa lampeggiare l'anello.
    public void Locate(Piece piece)
    {
        if (current == null || piece == null) return;
        if (piece.Data.board_id != current.id) ShowBoard(piece.Data.board_id);
        float worldPerPixel = 2f * cam.orthographicSize / Screen.height;
        var at = piece.transform.position;
        cam.transform.position = new Vector3(at.x + hud.OccupiedRightPixels / 2f * worldPerPixel, at.y, -10f);
        MoveCamera(Vector3.zero);
        piece.Ping();
    }

    public Piece MyChampion()
    {
        foreach (var p in pieces.Values)
            if (p.Mine && p.Data.kind == Kinds.Champion) return p;
        return null;
    }

    // Come cliccare la pedina sulla mappa: la seleziona (se e' una tua) e ne apre la scheda.
    public void SelectAndShow(Piece piece)
    {
        if (current == null || piece == null) return;
        if (piece.Data.board_id != current.id) ShowBoard(piece.Data.board_id);
        SetMode(ClickMode.Move);
        if (piece.Movable && selected != piece)
        {
            ClearSelection();
            selected = piece;
            piece.SetHighlight(true);
        }
        panel.Show(piece, ActionsFor(piece));
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
            if (!string.IsNullOrEmpty(actor.Data.race))
            {
                list.Add(new PanelAction
                {
                    Label = mode == ClickMode.Breed ? "Annulla riproduzione" : "Riproduci con...",
                    Perform = () =>
                    {
                        SetMode(mode == ClickMode.Breed ? ClickMode.Move : ClickMode.Breed);
                        panel.Show(actor, ActionsFor(actor));
                    },
                    BlockedReason = () => actor.Defeated ? "fuori gioco" : null,
                });
            }
            if (actor.Data.kind == Kinds.Champion)
            {
                list.Add(new PanelAction
                {
                    Label = "Crea pedina (paga con vita)",
                    Perform = () => net.SendCommand("create", actor.Data.id, null, method: "health"),
                    BlockedReason = () => BlockReason(actor, null, needsReady: true),
                });
                list.Add(new PanelAction
                {
                    Label = "Crea pedina (paga con oggetti)",
                    Perform = () => net.SendCommand("create", actor.Data.id, null, method: "resources"),
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
            int dist = GridMath.Distance(actor.Data.x, actor.Data.y, target.Data.x, target.Data.y);
            if (dist > actor.Data.vision) return "fuori portata";
        }
        if (needsReady && actor.SecondsUntilActReady > 0f) return $"pronta tra {actor.SecondsUntilActReady:0.0}s";
        return null;
    }
}
