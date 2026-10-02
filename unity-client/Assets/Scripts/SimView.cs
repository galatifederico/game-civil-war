using System.Collections;
using System.Collections.Generic;
using System.Linq;
using Newtonsoft.Json.Linq;
using UnityEngine;

/// <summary>
/// Draws the sim_core world: zones, buildings, pawns, dirt and fog of war. Polls <c>/api/ui/state</c>
/// and keeps no game logic: everything shown comes from the server.
/// World coordinates: one unit per cell, x to the right, map y downwards (so world y = -map y).
/// </summary>
public class SimView : MonoBehaviour
{
    public string serverUrl = "http://127.0.0.1:8787";
    public float pollSeconds = 0.4f;

    public SimApi Api { get; private set; }
    public JObject Map { get; private set; }
    public JObject State { get; private set; }
    public JObject Control { get; private set; }
    public string Error { get; private set; }
    public int Layer { get; private set; }
    public string FogFaction { get; set; } = "";
    public long? Selected { get; private set; }
    public JObject SelectedEntity { get; private set; }
    public JObject SelectedAi { get; private set; }
    public string LastCommandResult { get; set; }

    // Player side: who we are, our faction's data, the orders the selected pawn can take.
    public string PlayerId { get; set; }
    public JObject PlayerInfo { get; private set; }
    public JArray SelectedActions { get; private set; }
    /// Order waiting for a target click (a cell or an entity).
    public JObject PendingAction
    {
        get => pendingAction;
        set
        {
            pendingAction = value;
            if (value != null) DigMode = false;
        }
    }
    JObject pendingAction;
    public bool FollowCamera { get; set; }

    /// Screen pixels per sprite pixel. Whole numbers (close view, Pokémon-like) are pixel-perfect;
    /// below 1 is the map overview.
    public float Zoom { get; set; } = 3f;

    /// Drag a rectangle on the map to designate digging for our faction. It lasts for one rectangle:
    /// outside of it, dragging only moves the view.
    public bool DigMode
    {
        get => digMode;
        set
        {
            digMode = value;
            if (value) pendingAction = null;
            else CancelDig();
        }
    }
    bool digMode;
    Vector2Int? digStart;
    SpriteRenderer digRect;
    Texture2D terrainTex;
    readonly Dictionary<string, SpriteRenderer> quarterTint = new();
    int terrainSeen = -1;
    const float CloseZoom = 3f;
    /// Pigeon channel shown: "main" (default: few messages, the ones that matter), "mine", "all" or a
    /// category id.
    public string FeedFilter { get; set; } = "main";
    /// The channel shown in the pigeon window, and the main channel (polled always, for the unread count).
    public JObject Feed { get; private set; }
    public JObject MainFeed { get; private set; }
    /// Set by the HUD while the pigeon or the world-resources window is open: only then they are polled.
    public bool PigeonOpen { get; set; }
    public bool EconomyOpen { get; set; }
    public JObject Economy { get; private set; }
    float nextEconomy;

    // Champion walked by hand, one cell per step as in Pokémon. Facing: the cell in front of it.
    public Vector2Int Facing { get; private set; } = new(0, 1);
    /// Direction held on the on-screen joystick (set every frame by the HUD); diagonals allowed.
    public Vector2Int PadWalk { get; set; }
    /// Set by the HUD while a menu is open: the champion does not move.
    public bool MovementLocked { get; set; }
    /// A pawn or building picked on the map with a click or a tap (the HUD opens its card).
    public System.Action<long> Picked;
    public JArray ChampionActions { get; private set; }
    const float StepSeconds = 0.17f;
    bool stepInFlight;
    float nextStepAt;
    Vector2Int lastWalk;
    JToken champOverride;
    float champOverrideUntil;
    /// The camera stays on the champion while it walks, even with another pawn selected.
    bool walkFocus;
    bool fogChosen;

    Camera cam;
    Transform zonesRoot, cellsRoot, entitiesRoot;
    readonly Dictionary<long, EntityGo> gos = new();
    readonly List<GameObject> cellPool = new();
    SpriteRenderer fogRenderer, selectionRing;
    Texture2D fogTex;
    bool fogInitialized;
    Vector3 dragOrigin;

    /// Phones and tablets: one finger pans, taps select, a long press sends our pawn, two fingers zoom.
    /// <c>--touch-ui</c> shows the phone layout on a PC (for checks).
    public static bool TouchUi => Application.isMobilePlatform || ForcedTouch;
    static readonly bool ForcedTouch = System.Array.IndexOf(System.Environment.GetCommandLineArgs(), "--touch-ui") >= 0;
    const string ServerPref = "server";
    Vector2 touchStart;
    float touchStartTime, pinchStartDist, pinchStartZoom;
    bool touchMoved, touchOnUi, longPressDone, pinching;
    // Mouse: the left button selects with a click and pans with a drag.
    Vector3 leftStart, leftOrigin;
    bool leftDown, leftDragged;
    /// Marker over our own pawns, only on this player's screen.
    static readonly Color MineColor = new(0.45f, 1f, 0.5f);

    class EntityGo
    {
        public GameObject Root;
        public SpriteRenderer Body, Outline, BarBg, Bar, Marker;
        // Chibi layers: base (race), cloth (tinted with the faction colour), accessory (class or crown).
        public SpriteRenderer Base, Cloth, Acc;
        // Arrow over the head of the pawns of our faction.
        public SpriteRenderer Mine, MineEdge;
        public float MineY;
        public Sprite[] BaseFrames, ClothFrames, AccFrames;
        public int Dir;
        public float Anim;
        public bool Dead;
        public Vector3 Target;
        public bool Building;
    }

    void Start()
    {
        serverUrl = PlayerPrefs.GetString(ServerPref, serverUrl);
        foreach (var a in System.Environment.GetCommandLineArgs())
            if (a.StartsWith("--server=")) serverUrl = a.Substring(9);
        if (TouchUi)
        {
            Screen.sleepTimeout = SleepTimeout.NeverSleep;
            Application.targetFrameRate = 60;
        }
        Api = new SimApi(serverUrl);
        cam = Camera.main;
        if (cam == null)
        {
            var go = new GameObject("Main Camera") { tag = "MainCamera" };
            cam = go.AddComponent<Camera>();
        }
        cam.orthographic = true;
        cam.clearFlags = CameraClearFlags.SolidColor;
        cam.backgroundColor = new Color(0.16f, 0.14f, 0.11f);
        zonesRoot = new GameObject("Zones").transform;
        cellsRoot = new GameObject("Cells").transform;
        entitiesRoot = new GameObject("Entities").transform;
        selectionRing = NewSprite("Selection", null, Shapes.Get("ring"), new Color(1f, 0.45f, 0.3f), 31000);
        selectionRing.enabled = false;
        StartCoroutine(Run());
    }

    /// <summary>Connects to another server (address remembered on this device) by restarting the client.</summary>
    public void SetServer(string url)
    {
        url = url.Trim();
        if (url.Length == 0) return;
        if (!url.Contains("://")) url = "http://" + url;
        // No port given: the default of the sim server.
        if (url.IndexOf(':', url.IndexOf("://") + 3) < 0) url = url.TrimEnd('/') + ":8787";
        PlayerPrefs.SetString(ServerPref, url);
        PlayerPrefs.Save();
        UnityEngine.SceneManagement.SceneManager.LoadScene(UnityEngine.SceneManagement.SceneManager.GetActiveScene().buildIndex);
    }

    IEnumerator Run()
    {
        while (Map == null)
        {
            yield return Api.Get("/api/ui/map", j => { Map = (JObject)j; Error = null; }, e => Error = "Server non raggiungibile su " + serverUrl + " (" + e + ")");
            if (Map == null) yield return new WaitForSeconds(2f);
        }
        SetLayer(0);
        while (true)
        {
            var query = string.IsNullOrEmpty(FogFaction) ? "" : "?faction=" + UnityEngine.Networking.UnityWebRequest.EscapeURL(FogFaction);
            yield return Api.Get("/api/ui/state" + query, j => { State = (JObject)j; Error = null; OnState(); }, e => Error = "Server non raggiungibile (" + e + ")");
            yield return Api.Get("/api/control", j => Control = (JObject)j);
            yield return Api.Get("/api/ui/terrain?since=" + Mathf.Max(0, terrainSeen), OnTerrain, _ => { });
            if (PlayerId == null && State?["snapshot"]?["players"] is JObject players && players.Properties().Any())
            {
                PlayerId = players.Properties().First().Name;
                if (!fogChosen) FogFaction = (string)players[PlayerId]["faction"] ?? "";
            }
            if (PlayerId != null)
            {
                bool first = PlayerInfo == null;
                yield return Api.Get("/api/ui/player/" + UnityEngine.Networking.UnityWebRequest.EscapeURL(PlayerId), j => PlayerInfo = (JObject)j, _ => { });
                if (first) RefreshChampionActions();
                // Start like a Pokémon game: close view on our champion.
                if (first && ChampionId.HasValue && gos.ContainsKey(ChampionId.Value))
                {
                    FollowCamera = true;
                    FocusOn(ChampionId.Value);
                    Zoom = CloseZoom;
                }
            }
            var who = PlayerId == null ? "" : "&player=" + UnityEngine.Networking.UnityWebRequest.EscapeURL(PlayerId);
            yield return Api.Get("/api/ui/feed?limit=40&filter=main" + who, j => MainFeed = (JObject)j, _ => { });
            if (PigeonOpen && FeedFilter != "main")
            {
                var filter = FeedFilter;
                yield return Api.Get("/api/ui/feed?limit=40&filter=" + filter + who, j => { if (FeedFilter == filter) Feed = (JObject)j; }, _ => { });
            }
            else Feed = MainFeed;
            if (EconomyOpen && Time.time >= nextEconomy)
            {
                nextEconomy = Time.time + 2f;
                yield return Api.Get("/api/ui/economy", j => Economy = (JObject)j, _ => { });
            }
            if (Selected.HasValue)
            {
                var id = Selected.Value;
                yield return Api.Get($"/api/entities/{id}?truth=true", j => { if (Selected == id) SelectedEntity = (JObject)j; }, _ => { });
                yield return Api.Get($"/api/entities/{id}/ai", j => { if (Selected == id) SelectedAi = (JObject)j; }, _ => { });
            }
            yield return new WaitForSeconds(pollSeconds);
        }
    }

    /// Dug cells and admin edits: patch the tile rows and, for the map on screen, the texture.
    void OnTerrain(JToken j)
    {
        int total = (int)j["total"];
        if (terrainSeen < 0 || total < terrainSeen)
        {
            // First look (the map already has these changes) or a reload: start counting from here.
            if (terrainSeen >= 0) StartCoroutine(ReloadMap());
            terrainSeen = total;
            return;
        }
        var here = new List<Vector2Int>();
        foreach (var c in j["changes"])
        {
            int layer = (int)c[0], x = (int)c[1], y = (int)c[2];
            if (layer >= Layers.Count) continue;
            TileMap.SetChar(Layers[layer], x, y, (string)c[3]);
            if (layer == Layer) here.Add(new Vector2Int(x, y));
        }
        terrainSeen = total;
        if (here.Count > 0) TileMap.Patch(terrainTex, Layers[Layer], Map["legend"] as JObject, here);
    }

    IEnumerator ReloadMap()
    {
        yield return Api.Get("/api/ui/map", j => Map = (JObject)j, _ => { });
        SetLayer(Mathf.Min(Layer, Layers.Count - 1));
    }

    // ── Commands ──────────────────────────────────────────────────────────────

    public void SendControl(JObject body) => StartCoroutine(Api.Post("/api/control", body, j => Control = (JObject)j, e => LastCommandResult = e));

    public void SendCommand(JObject cmd, System.Action done = null) => StartCoroutine(Api.Post("/api/commands?now=true", cmd,
        j => { LastCommandResult = (string)j["ok"] ?? j.ToString(); done?.Invoke(); }, e => { LastCommandResult = "Rifiutato: " + e; done?.Invoke(); }));

    /// Who the camera follows: the selected pawn, otherwise our champion.
    public long? FollowId => !walkFocus && Selected.HasValue && gos.TryGetValue(Selected.Value, out var g) && !g.Building ? Selected : ChampionId;

    /// <summary>Selects a pawn or building; <paramref name="byChampion"/>: picked by the walking champion
    /// (the camera stays on the champion).</summary>
    public void Select(long? id, bool byChampion = false)
    {
        walkFocus = byChampion || (id.HasValue && id == ChampionId);
        // Selecting a pawn puts the camera on it (arrows or middle drag release it, F or "Segui" resume).
        if (id.HasValue && gos.TryGetValue(id.Value, out var picked) && !picked.Building)
        {
            FollowCamera = true;
            if (Zoom < 1f) Zoom = CloseZoom;
        }
        else if (!id.HasValue && Selected.HasValue) FollowCamera = false; // clicking empty ground lets the camera go
        Selected = id;
        SelectedEntity = null;
        SelectedAi = null;
        SelectedActions = null;
        PendingAction = null;
        if (id.HasValue && IsMine(id.Value))
            StartCoroutine(Api.Get($"/api/ui/actions/{id.Value}", j => { if (Selected == id) SelectedActions = (JArray)j; }, _ => { }));
        else if (id.HasValue) RefreshChampionActions();
    }

    /// What the champion can do to someone else (talk, attack, …): shown on the card of other pawns.
    public void RefreshChampionActions()
    {
        if (ChampionId is long c)
            StartCoroutine(Api.Get($"/api/ui/actions/{c}", j => ChampionActions = (JArray)j, _ => { }));
    }

    // ── Champion walked by hand ───────────────────────────────────────────────

    /// One step of the champion per call while a direction is held, at walking pace.
    void Walk(Vector2Int dir)
    {
        if (dir == Vector2Int.zero || !ChampionId.HasValue || PlayerId == null)
        {
            lastWalk = Vector2Int.zero;
            return;
        }
        if (Zoom < 1f) Zoom = CloseZoom;
        bool fresh = dir != lastWalk;
        lastWalk = dir;
        Facing = dir;
        walkFocus = true;
        FollowCamera = true;
        if (gos.TryGetValue(ChampionId.Value, out var go)) go.Dir = DirIndex(dir);
        if (stepInFlight || (!fresh && Time.time < nextStepAt)) return;
        stepInFlight = true;
        nextStepAt = Time.time + StepSeconds;
        var body = new JObject { ["player"] = PlayerId, ["dx"] = dir.x, ["dy"] = dir.y };
        StartCoroutine(Api.Post("/api/ui/step", body, j => { stepInFlight = false; ApplyChampionPos(j["pos"]); }, e => { stepInFlight = false; LastCommandResult = e; }));
    }

    static int DirIndex(Vector2Int d) => d.x < 0 ? 1 : d.x > 0 ? 2 : d.y < 0 ? 3 : 0;

    /// Puts the champion where the server says it now is, without waiting for the next state poll.
    void ApplyChampionPos(JToken pos)
    {
        if (pos == null || !ChampionId.HasValue) return;
        long id = ChampionId.Value;
        champOverride = pos.DeepClone();
        champOverrideUntil = Time.time + 1.2f;
        if (EntityById(id) is JObject e) e["pos"] = pos.DeepClone();
        if (!gos.TryGetValue(id, out var go)) return;
        go.Target = CellCenter(pos);
        int layer = (int)pos["layer"];
        if (layer != Layer)
        {
            // Through a door or down the stairs: appear there at once.
            SetLayer(layer);
            go.Root.transform.position = go.Target;
            cam.transform.position = new Vector3(go.Target.x, go.Target.y, -10f);
        }
        go.Root.SetActive(true);
    }

    /// The pawn (or else the building) in front of the champion or next to it, if any.
    public long? NearbyTarget()
    {
        if (!ChampionId.HasValue || State == null || EntityById(ChampionId.Value)?["pos"] is not JObject me) return null;
        int layer = (int)me["layer"], x = (int)me["x"], y = (int)me["y"];
        int ax = x + Facing.x, ay = y + Facing.y;
        long? best = null;
        int bestScore = int.MaxValue;
        foreach (var e in State["snapshot"]["entities"])
        {
            if (e["pos"] is not JObject p || (int)p["layer"] != layer || (long)e["id"] == ChampionId.Value || (bool?)e["dead"] == true) continue;
            int px = (int)p["x"], py = (int)p["y"];
            bool building = (string)e["kind"] == "building";
            int score = px == ax && py == ay ? 0 : Mathf.Max(Mathf.Abs(px - x), Mathf.Abs(py - y)) <= 1 ? 1 : int.MaxValue;
            if (score == int.MaxValue) continue;
            score = score * 2 + (building ? 1 : 0);
            if (score < bestScore)
            {
                bestScore = score;
                best = (long)e["id"];
            }
        }
        return best;
    }

    /// What the champion can do with a pawn or building (talk, jobs, abilities, follow).
    public void Interactions(long target, System.Action<JObject> ok) =>
        StartCoroutine(Api.Get($"/api/ui/interactions/{UnityEngine.Networking.UnityWebRequest.EscapeURL(PlayerId)}/{target}", j => ok((JObject)j), e => LastCommandResult = e));

    /// The champion talks to someone: answers (name, line) or the reason it cannot.
    public void Talk(long target, System.Action<string, string> ok) =>
        StartCoroutine(Api.Post("/api/ui/talk", new JObject { ["player"] = PlayerId, ["target"] = target },
            j => ok((string)j["name"], (string)j["line"]), e => ok(null, e)));

    /// An item explained, with who in the team holds it.
    public void ItemInfo(string item, System.Action<JObject> ok) =>
        StartCoroutine(Api.Get($"/api/ui/item/{UnityEngine.Networking.UnityWebRequest.EscapeURL(item)}?player={UnityEngine.Networking.UnityWebRequest.EscapeURL(PlayerId ?? "")}", j => ok((JObject)j), e => LastCommandResult = e));

    /// The whole of a map seen from above (the world map), leaving the champion where it is.
    public void ShowOverview(int layer)
    {
        FollowCamera = false;
        walkFocus = false;
        if (layer != Layer) SetLayer(layer);
        var l = Layers[Layer];
        FitCamera((int)l["width"], (int)l["height"]);
    }

    /// Back to the close view on the champion.
    public void CloseView()
    {
        Zoom = CloseZoom;
        walkFocus = true;
        FollowCamera = ChampionId.HasValue;
        if (ChampionId is long c) FocusOn(c);
    }

    public void ChooseFog(string faction)
    {
        FogFaction = faction;
        fogChosen = true;
    }

    public long? ChampionId => (long?)PlayerInfo?["champion"];

    /// True for the player's champion and the members of its faction.
    public bool IsMine(long id) => PlayerInfo?["members"]?.Any(m => (long)m["id"] == id) ?? false;

    public JToken Member(long id) => PlayerInfo?["members"]?.FirstOrDefault(m => (long)m["id"] == id);

    public void SendPlayerOrder(long entity, JObject order)
    {
        if (PlayerId == null) return;
        SendCommand(new JObject { ["type"] = "player_order", ["player"] = PlayerId, ["entity"] = entity, ["order"] = order });
    }

    public void SendPlayerCommand(JObject cmd, System.Action done = null)
    {
        if (PlayerId == null) return;
        cmd["player"] = PlayerId;
        SendCommand(cmd, done);
    }

    public void Save() => StartCoroutine(Api.Post("/api/save", new JObject { ["path"] = "saves/quick.json" }, j => LastCommandResult = (string)j["ok"], e => LastCommandResult = e));

    public void Load() => StartCoroutine(Api.Post("/api/load", new JObject { ["path"] = "saves/quick.json" }, j => { LastCommandResult = (string)j["ok"]; Select(null); }, e => LastCommandResult = e));

    public void FocusOn(long id)
    {
        if (!gos.TryGetValue(id, out var go)) return;
        var e = EntityById(id);
        var layer = (int?)e?["pos"]?["layer"] ?? Layer;
        if (layer != Layer) SetLayer(layer);
        var p = go.Target;
        cam.transform.position = new Vector3(p.x, p.y, -10f);
        if (Zoom < 2f) Zoom = CloseZoom;
    }

    /// Resolves a pending order with a clicked cell or entity.
    void ResolvePending(Vector3 screen)
    {
        var a = PendingAction;
        PendingAction = null;
        if (a == null || !Selected.HasValue) return;
        long who = Selected.Value;
        string kind = (string)a["kind"], id = (string)a["id"], needs = (string)a["needs_target"];
        var w = cam.ScreenToWorldPoint(screen);
        var cell = new JObject { ["layer"] = Layer, ["x"] = Mathf.FloorToInt(w.x), ["y"] = Mathf.FloorToInt(-w.y) };
        long? target = Pick(screen);
        if (needs == "cell" || kind == "move")
        {
            SendPlayerOrder(who, new JObject { ["kind"] = "move", ["pos"] = cell });
            return;
        }
        if (!target.HasValue)
        {
            LastCommandResult = "Nessun bersaglio sotto il cursore";
            return;
        }
        SendPlayerOrder(who, OrderFor(kind, id, target));
    }

    public static JObject OrderFor(string kind, string id, long? target) => kind switch
    {
        "job" => new JObject { ["kind"] = "job", ["job"] = id, ["target"] = target.HasValue ? target.Value : null },
        "ability" => new JObject { ["kind"] = "ability", ["ability"] = id, ["target"] = target.HasValue ? target.Value : null },
        "follow" => new JObject { ["kind"] = "follow", ["target"] = target ?? 0, ["distance"] = 2 },
        _ => new JObject { ["kind"] = "stop" },
    };

    // ── Map and layers ────────────────────────────────────────────────────────

    public JArray Layers => (JArray)Map?["layers"];

    public void SetLayer(int layer)
    {
        Layer = layer;
        foreach (Transform c in zonesRoot) Destroy(c.gameObject);
        var l = Layers[layer];
        int w = (int)l["width"], h = (int)l["height"];
        if (terrainTex != null) Destroy(terrainTex);
        var tileTex = TileMap.Build(l, Map["legend"] as JObject);
        terrainTex = tileTex;
        if (tileTex != null)
        {
            // Pixel-art terrain composed from the map's tiles.
            var ground = NewSprite("Terrain", zonesRoot, Sprite.Create(tileTex, new Rect(0, 0, tileTex.width, tileTex.height), new Vector2(0, 1), Chibi.PixelsPerUnit), Color.white, 0);
            ground.transform.position = Vector3.zero;
        }
        else
        {
            var ground = NewSprite("Ground", zonesRoot, Shapes.Pixel, new Color(0.84f, 0.8f, 0.7f), 0);
            ground.transform.localScale = new Vector3(w, h, 1);
            foreach (var z in Map["zones"].Where(z => (int)z["layer"] == layer))
            {
                var sr = NewSprite((string)z["name"], zonesRoot, Shapes.Pixel, ZoneColor(z), 1);
                sr.transform.position = new Vector3((int)z["x"], -(int)z["y"], 0);
                sr.transform.localScale = new Vector3((int)z["w"], (int)z["h"], 1);
            }
        }
        foreach (var wall in Map["walls"] ?? new JArray())
        {
            // [name, layer, x, y, w, h]
            if ((int)wall[1] != layer) continue;
            string wname = (string)wall[0];
            var color = wname.Contains("Torrente") || wname.Contains("Fiume") ? new Color(0.3f, 0.5f, 0.75f) : new Color(0.35f, 0.3f, 0.27f);
            var sr = NewSprite(wname, zonesRoot, Shapes.Pixel, color, 3);
            sr.transform.position = new Vector3((int)wall[2], -(int)wall[3], 0);
            sr.transform.localScale = new Vector3((int)wall[4], (int)wall[5], 1);
        }
        foreach (var p in Map["portals"])
        {
            // Borders between maps and building doors need no marker; other passages (manholes, stairs) do.
            if (((string)p["name"]).Contains("→")) continue;
            foreach (var end in new[] { p["a"], p["b"] })
                if ((int)end["layer"] == layer && !IsBuildingDoor(end))
                {
                    var sr = NewSprite("Passage", zonesRoot, Shapes.Get("circle"), new Color(0.2f, 0.18f, 0.2f, 0.85f), 6);
                    sr.transform.position = CellCenter(end);
                    sr.transform.localScale = Vector3.one * 0.8f;
                }
        }
        // Trees: tall sprites whose canopy covers the cell above and pawns walking behind them.
        var treeSprite = Chibi.LoadSingle("tree_tall");
        if (treeSprite != null)
            foreach (var c in TileMap.Trees(l, Map["legend"] as JObject))
            {
                var t = NewSprite("Tree", zonesRoot, treeSprite, Color.white, 0);
                t.transform.position = new Vector3(c.x + 0.5f, -c.y - 1f, 0);
                t.sortingOrder = Chibi.OrderForY(-c.y - 1f) + 7;
            }
        // Quarters: tinted with their owner's colour in the overview.
        quarterTint.Clear();
        foreach (var z in Map["zones"].Where(z => (int)z["layer"] == layer && (z["tags"]?.Any(t => (string)t == "quartiere") ?? false)))
        {
            var q = NewSprite("Quarter " + (string)z["id"], zonesRoot, Shapes.Pixel, Color.clear, 30000);
            q.transform.position = new Vector3((int)z["x"], -(int)z["y"], 0);
            q.transform.localScale = new Vector3((int)z["w"], (int)z["h"], 1);
            quarterTint[(string)z["id"]] = q;
        }
        // Fog texture sized to the layer.
        if (fogRenderer == null) fogRenderer = NewSprite("Fog", null, Shapes.Pixel, Color.white, 32000);
        fogTex = new Texture2D(w, h, TextureFormat.RGBA32, false) { filterMode = FilterMode.Bilinear, wrapMode = TextureWrapMode.Clamp };
        fogRenderer.sprite = Sprite.Create(fogTex, new Rect(0, 0, w, h), new Vector2(0, 1), 1);
        fogRenderer.transform.position = Vector3.zero;
        fogInitialized = false;
        if (!FollowCamera) FitCamera(w, h);
        if (State != null) OnState();
    }

    bool IsBuildingDoor(JToken pos) =>
        State?["snapshot"]?["entities"]?.Any(e => (string)e["kind"] == "building" && e["pos"]?.Type == JTokenType.Object
            && (int)e["pos"]["layer"] == (int)pos["layer"] && (int)e["pos"]["x"] == (int)pos["x"] && (int)e["pos"]["y"] == (int)pos["y"]) ?? false;

    /// Index of a map by id.
    public int LayerIndex(string id)
    {
        if (Layers == null) return -1;
        for (int i = 0; i < Layers.Count; i++)
            if ((string)Layers[i]["id"] == id) return i;
        return -1;
    }

    static Color ZoneColor(JToken z)
    {
        var tags = z["tags"]?.Select(t => (string)t).ToList() ?? new List<string>();
        Color c = new(0.74f, 0.68f, 0.56f);
        if (tags.Contains("campi")) c = new Color(0.62f, 0.72f, 0.45f);
        else if (tags.Contains("vegano")) c = new Color(0.55f, 0.75f, 0.5f);
        else if (tags.Contains("terme")) c = new Color(0.55f, 0.7f, 0.78f);
        else if (tags.Contains("sotterraneo") || tags.Contains("gallerie")) c = new Color(0.45f, 0.4f, 0.36f);
        else if (tags.Contains("celle")) c = new Color(0.45f, 0.47f, 0.6f);
        else if (tags.Contains("trono")) c = new Color(0.85f, 0.7f, 0.3f);
        else if (tags.Contains("vizio")) c = new Color(0.72f, 0.45f, 0.45f);
        else if (tags.Contains("pubblico")) c = new Color(0.8f, 0.74f, 0.62f);
        return c;
    }

    void FitCamera(int w, int h)
    {
        float aspect = (float)Screen.width / Mathf.Max(1, Screen.height);
        // Leave room for the side panel on the right.
        float usable = 0.72f;
        cam.orthographicSize = Mathf.Max(h / 2f, w / (2f * aspect * usable)) * 1.05f;
        Zoom = Screen.height / (2f * Chibi.PixelsPerUnit * cam.orthographicSize);
        float viewW = cam.orthographicSize * 2f * aspect;
        cam.transform.position = new Vector3(w / 2f + viewW * (1f - usable) / 2f, -h / 2f, -10f);
    }

    // ── State → sprites ───────────────────────────────────────────────────────

    public JToken SpriteDef(string key) => State?["sprites"]?[key];

    void OnState()
    {
        if (State == null || Map == null) return;
        // A state asked for before our last step must not pull the champion back.
        if (champOverride != null && Time.time < champOverrideUntil && ChampionId.HasValue && EntityById(ChampionId.Value) is JObject ce)
            ce["pos"] = champOverride.DeepClone();
        var seen = new HashSet<long>();
        foreach (var e in State["snapshot"]["entities"])
        {
            var pos = e["pos"];
            if (pos == null || pos.Type == JTokenType.Null) continue;
            long id = (long)e["id"];
            seen.Add(id);
            bool building = (string)e["kind"] == "building";
            if (!gos.TryGetValue(id, out var go))
            {
                go = Create(id, building);
                gos[id] = go;
                go.Root.transform.position = building ? BuildingCenter(pos) : CellCenter(pos);
            }
            go.Root.SetActive((int)pos["layer"] == Layer);
            go.Target = building ? BuildingCenter(pos) : CellCenter(pos);
            Style(go, e);
        }
        foreach (var id in gos.Keys.ToList())
            if (!seen.Contains(id))
            {
                Destroy(gos[id].Root);
                gos.Remove(id);
            }
        DrawCells();
        DrawFog();
        DrawQuarters();
    }

    EntityGo Create(long id, bool building)
    {
        var root = new GameObject(building ? $"Building {id}" : $"Pawn {id}");
        root.transform.SetParent(entitiesRoot, false);
        var go = new EntityGo { Root = root, Building = building };
        if (building)
        {
            go.Outline = NewSprite("Outline", root.transform, Shapes.Get("square"), Color.black, 4);
            go.Outline.transform.localScale = Vector3.one * 2.1f;
            go.Body = NewSprite("Body", root.transform, Shapes.Get("square"), Color.gray, 5);
            go.Body.transform.localScale = Vector3.one * 1.8f;
        }
        else
        {
            go.Outline = NewSprite("Outline", root.transform, Shapes.Get("circle"), Color.black, 10);
            go.Outline.transform.localScale = Vector3.one * 1.05f;
            go.Body = NewSprite("Body", root.transform, Shapes.Get("circle"), Color.white, 11);
            go.Body.transform.localScale = Vector3.one * 0.8f;
            go.BarBg = NewSprite("BarBg", root.transform, Shapes.Pixel, new Color(0, 0, 0, 0.5f), 12);
            go.BarBg.transform.localPosition = new Vector3(-0.45f, -0.5f, 0);
            go.BarBg.transform.localScale = new Vector3(0.9f, 0.14f, 1);
            go.Bar = NewSprite("Bar", root.transform, Shapes.Pixel, new Color(0.35f, 0.85f, 0.45f), 13);
            go.Bar.transform.localPosition = new Vector3(-0.45f, -0.5f, 0);
            go.Marker = NewSprite("Marker", root.transform, Shapes.Get("cross"), new Color(0.15f, 0.15f, 0.2f, 0.9f), 14);
            go.Marker.transform.localScale = Vector3.one * 0.7f;
            var feet = new Vector3(0, -0.45f, 0);
            go.Base = NewSprite("Chibi", root.transform, null, Color.white, 11);
            go.Cloth = NewSprite("Cloth", root.transform, null, Color.white, 12);
            go.Acc = NewSprite("Accessory", root.transform, null, Color.white, 13);
            foreach (var sr in new[] { go.Base, go.Cloth, go.Acc }) sr.transform.localPosition = feet;
            // Above trees, roofs and fog: our pawns are always found at a glance.
            go.MineEdge = NewSprite("MineEdge", root.transform, Shapes.Get("triangle"), new Color(0.08f, 0.12f, 0.1f, 0.9f), 31700);
            go.Mine = NewSprite("Mine", root.transform, Shapes.Get("triangle"), MineColor, 31701);
            go.Mine.transform.localRotation = go.MineEdge.transform.localRotation = Quaternion.Euler(0, 0, 180);
        }
        return go;
    }

    void Style(EntityGo go, JToken e)
    {
        string faction = (string)e["faction"];
        var fsprite = SpriteDef("faction:" + faction);
        Color outline = Shapes.Parse((string)fsprite?["outline"], Shapes.FromId(faction ?? "none") * 0.6f);
        if (go.Building)
        {
            var def = (string)e["building"]?["def"];
            var sp = SpriteDef("building:" + def) ?? SpriteDef("building:default");
            var pic = Chibi.LoadSingle((string)SpriteDef("building:" + def)?["sheet"]);
            if (pic != null)
            {
                // Pixel-art building: the door stands on the building's cell, the rest rises behind it.
                go.Body.sprite = pic;
                go.Body.transform.localScale = Vector3.one;
                go.Body.transform.localPosition = new Vector3(0, -0.5f, 0);
                bool ruined = (float?)e["building"]?["hp"] <= 0f;
                float hpRatio = Mathf.Clamp01(((float?)e["building"]?["hp"] ?? 1f) / Mathf.Max(1f, (float?)e["building"]?["max_hp"] ?? 1f));
                go.Body.color = ruined ? new Color(0.35f, 0.33f, 0.32f) : Color.Lerp(new Color(0.6f, 0.55f, 0.5f), Color.white, 0.4f + 0.6f * hpRatio);
                go.Body.sortingOrder = Chibi.OrderForY(go.Target.y) - 1;
                go.Outline.enabled = false;
                return;
            }
            go.Body.color = Shapes.Parse((string)sp?["color"], new Color(0.63f, 0.53f, 0.5f));
            bool destroyed = (float?)e["building"]?["hp"] <= 0f;
            go.Body.color = destroyed ? go.Body.color * 0.45f : go.Body.color;
            go.Outline.color = outline;
            return;
        }
        var race = (string)e["race"];
        var rs = SpriteDef("race:" + race);
        go.Mine.enabled = go.MineEdge.enabled = IsMine((long)e["id"]) && (bool?)e["dead"] != true;
        bool championChibi = ChampionId.HasValue && (long)e["id"] == ChampionId.Value;
        go.Dead = (bool?)e["dead"] ?? false;
        go.BaseFrames = Chibi.Load((string)rs?["sheet"]);
        if (go.BaseFrames != null)
        {
            // Chibi mode: an oval "team" shadow in the faction colour under the feet.
            go.ClothFrames = Chibi.Load((string)rs?["sheet"] + "_cloth");
            string accSheet = null;
            foreach (var c in e["classes"] ?? new JArray())
            {
                accSheet ??= (string)SpriteDef("class:" + (string)c)?["sheet"];
            }
            if (championChibi) accSheet = "acc_corona";
            go.AccFrames = Chibi.Load(accSheet);
            go.MineY = 1.15f;
            go.Body.enabled = false;
            go.Outline.sprite = Shapes.Get("circle");
            go.Outline.color = championChibi ? new Color(1f, 0.82f, 0.2f, 0.9f) : new Color(outline.r, outline.g, outline.b, 0.75f);
            go.Outline.transform.localScale = new Vector3(championChibi ? 1.1f : 0.85f, 0.32f, 1f);
            go.Outline.transform.localPosition = new Vector3(0, -0.42f, 0);
            go.Cloth.color = outline;
            var chibiAct = e["activity"];
            var chibiFlags = chibiAct?["flags"]?.Select(f => (string)f).ToList() ?? new List<string>();
            float chibiProgress = (float?)chibiAct?["progress"] ?? 0f;
            go.Bar.transform.localScale = new Vector3(0.9f * Mathf.Clamp01(chibiProgress), 0.1f, 1);
            go.Bar.enabled = go.BarBg.enabled = !go.Dead && chibiProgress > 0f;
            go.BarBg.transform.localPosition = go.Bar.transform.localPosition = new Vector3(-0.45f, 0.95f, 0);
            go.Marker.enabled = chibiFlags.Contains("detained") || chibiFlags.Contains("knocked_out");
            go.Marker.transform.localPosition = new Vector3(0, 0.2f, 0);
            go.Marker.color = chibiFlags.Contains("knocked_out") ? new Color(1f, 0.9f, 0.3f, 0.9f) : new Color(0.2f, 0.25f, 0.6f, 0.9f);
            var tint = go.Dead ? new Color(0.55f, 0.55f, 0.55f, 0.6f) : Color.white;
            go.Base.color = tint;
            go.Acc.color = tint;
            if (go.Dead) go.Cloth.color = new Color(0.4f, 0.4f, 0.4f, 0.6f);
            var rot = go.Dead || chibiFlags.Contains("knocked_out") ? Quaternion.Euler(0, 0, 90) : Quaternion.identity;
            foreach (var sr in new[] { go.Base, go.Cloth, go.Acc }) sr.transform.localRotation = rot;
            return;
        }
        go.Body.enabled = true;
        go.MineY = 0.8f;
        var shape = (string)rs?["shape"];
        go.Body.sprite = Shapes.Get(shape);
        go.Outline.sprite = Shapes.Get(shape);
        go.Body.color = Shapes.Parse((string)rs?["color"], Shapes.FromId(race));
        go.Outline.color = outline;
        bool champion = ChampionId.HasValue && (long)e["id"] == ChampionId.Value;
        go.Outline.transform.localScale = Vector3.one * (champion ? 1.45f : 1.05f);
        go.Body.transform.localScale = Vector3.one * (champion ? 1.05f : 0.8f);
        if (champion) go.Outline.color = new Color(1f, 0.82f, 0.2f);
        var act = e["activity"];
        var flags = act?["flags"]?.Select(f => (string)f).ToList() ?? new List<string>();
        bool dead = (bool?)e["dead"] ?? false;
        float progress = (float?)act?["progress"] ?? 0f;
        go.Bar.transform.localScale = new Vector3(0.9f * Mathf.Clamp01(progress), 0.14f, 1);
        go.Bar.enabled = go.BarBg.enabled = !dead && progress > 0f;
        go.Marker.enabled = dead || flags.Contains("detained");
        go.Marker.color = dead ? new Color(0.2f, 0.2f, 0.2f, 0.9f) : new Color(0.2f, 0.25f, 0.6f, 0.9f);
        if (dead)
        {
            go.Body.color = new Color(0.5f, 0.5f, 0.5f, 0.5f);
            go.Outline.color = new Color(0.3f, 0.3f, 0.3f, 0.5f);
        }
    }

    void DrawCells()
    {
        int i = 0;
        foreach (var c in State["cells"] ?? new JArray())
        {
            var p = c["pos"];
            float dirt = (float?)c["dirt"] ?? 0f;
            int pathogens = (int?)c["pathogens"] ?? 0;
            if ((int)p["layer"] != Layer || (dirt < 0.3f && pathogens == 0)) continue;
            if (i >= cellPool.Count) cellPool.Add(NewSprite("Cell", cellsRoot, Shapes.Pixel, Color.clear, 2).gameObject);
            var go = cellPool[i++];
            go.SetActive(true);
            go.transform.position = new Vector3((int)p["x"], -(int)p["y"], 0);
            float a = Mathf.Min(0.35f, 0.05f + dirt * 0.03f);
            go.GetComponent<SpriteRenderer>().color = pathogens > 0 ? new Color(0.45f, 0.65f, 0.15f, a + 0.1f) : new Color(0.4f, 0.3f, 0.15f, a);
        }
        for (; i < cellPool.Count; i++) cellPool[i].SetActive(false);
    }

    /// Colour of a faction (its outline in the sprite mapping), or a stable colour from its id.
    public Color FactionColor(string faction)
    {
        if (string.IsNullOrEmpty(faction)) return Color.clear;
        var hex = (string)SpriteDef("faction:" + faction)?["outline"];
        if (hex != null && ColorUtility.TryParseHtmlString(hex, out var c)) return c;
        return Color.HSVToRGB((faction.GetHashCode() & 0xFFFF) / 65535f, 0.6f, 0.85f);
    }

    public JToken Territory(string zone) => State?["snapshot"]?["territories"]?[zone];

    void DrawQuarters()
    {
        bool show = Zoom < 1f;
        foreach (var kv in quarterTint)
        {
            var owner = (string)Territory(kv.Key)?["owner"];
            var c = FactionColor(owner);
            c.a = owner == null ? 0f : 0.22f;
            kv.Value.color = c;
            kv.Value.enabled = show;
        }
    }

    void DrawFog()
    {
        var fog = State["fog"];
        bool on = fog != null && fog.Type != JTokenType.Null;
        fogRenderer.enabled = on;
        if (!on) return;
        int w = fogTex.width, h = fogTex.height;
        // Light veil: the world stays readable, unseen areas are just dimmer.
        var dark = new Color32(24, 28, 48, 95);
        var px = new Color32[w * h];
        for (int i = 0; i < px.Length; i++) px[i] = dark;
        foreach (var o in fog["observers"])
        {
            var p = o[0];
            int r = (int)o[1];
            if ((int)p["layer"] != Layer) continue;
            int cx = (int)p["x"], cy = (int)p["y"];
            for (int y = Mathf.Max(0, cy - r - 1); y <= Mathf.Min(h - 1, cy + r + 1); y++)
            for (int x = Mathf.Max(0, cx - r - 1); x <= Mathf.Min(w - 1, cx + r + 1); x++)
            {
                float d = Mathf.Sqrt((x - cx) * (x - cx) + (y - cy) * (y - cy));
                if (d <= r + 0.5f) px[(h - 1 - y) * w + x] = new Color32(0, 0, 0, 0);
            }
        }
        fogTex.SetPixels32(px);
        fogTex.Apply();
        fogInitialized = true;
    }

    // ── Frame update: movement, camera, selection ────────────────────────────

    void Update()
    {
        foreach (var go in gos.Values)
        {
            if (!go.Root.activeSelf) continue;
            var before = go.Root.transform.position;
            go.Root.transform.position = Vector3.Lerp(before, go.Target, 1f - Mathf.Exp(-Time.deltaTime * 8f));
            if (go.Mine != null && go.Mine.enabled)
            {
                // Bobbing arrow, grown in the overview so it still shows when the pawns are dots.
                float grow = Mathf.Clamp(0.7f / Mathf.Max(0.05f, Zoom), 1f, 6f);
                var at = new Vector3(0, go.MineY + 0.2f * (grow - 1f) + Mathf.Sin(Time.time * 4f) * 0.06f, 0);
                go.Mine.transform.localPosition = go.MineEdge.transform.localPosition = at;
                go.Mine.transform.localScale = Vector3.one * 0.44f * grow;
                go.MineEdge.transform.localScale = Vector3.one * 0.66f * grow;
            }
            if (go.BaseFrames == null) continue;
            var delta = go.Target - before;
            bool moving = delta.sqrMagnitude > 0.0025f && !go.Dead;
            go.Dir = Chibi.Direction(delta, go.Dir);
            go.Anim = moving ? go.Anim + Time.deltaTime * 6f : 0f;
            // idle, step A, idle, step B …
            int[] cycle = { 0, 1, 0, 2 };
            int pose = moving ? cycle[(int)go.Anim % 4] : 0;
            int i = go.Dir * 3 + pose;
            go.Base.sprite = go.BaseFrames[i];
            go.Cloth.sprite = go.ClothFrames?[i];
            go.Acc.sprite = go.AccFrames?[i];
            // Depth: lower on the map is drawn in front.
            int order = Chibi.OrderForY(go.Root.transform.position.y);
            go.Outline.sortingOrder = order;
            go.Base.sortingOrder = order + 1;
            go.Cloth.sortingOrder = order + 2;
            go.Acc.sortingOrder = order + 3;
            go.Marker.sortingOrder = order + 4;
            go.BarBg.sortingOrder = order + 5;
            go.Bar.sortingOrder = order + 6;
        }
        if (Selected.HasValue && gos.TryGetValue(Selected.Value, out var sel) && sel.Root.activeSelf)
        {
            selectionRing.enabled = true;
            selectionRing.transform.position = sel.Root.transform.position;
            selectionRing.transform.localScale = Vector3.one * (sel.Building ? 2.6f : 1.4f);
        }
        else selectionRing.enabled = false;
        CameraControls();
    }

    void ApplyZoom() => cam.orthographicSize = Screen.height / (2f * Chibi.PixelsPerUnit * Mathf.Max(0.05f, Zoom));

    /// Toggles between the close Pokémon-like view on the champion and the whole map.
    public void ToggleOverview()
    {
        if (Zoom >= 1f)
        {
            FollowCamera = false;
            var l = Layers[Layer];
            FitCamera((int)l["width"], (int)l["height"]);
        }
        else
        {
            Zoom = CloseZoom;
            FollowCamera = FollowId.HasValue;
        }
    }

    void CameraControls()
    {
        ApplyZoom();
        foreach (var q in quarterTint.Values) q.enabled = Zoom < 1f;
        var hud = GetComponent<SimHud>();
        bool overUi = hud != null && hud.IsOverUi(Input.mousePosition);
        float scroll = TouchUi ? 0f : Input.mouseScrollDelta.y;
        if (!overUi && Mathf.Abs(scroll) > 0.01f)
        {
            if (Zoom >= 1f && (Zoom > 1f || scroll > 0)) ZoomAt(Input.mousePosition, Mathf.Round(Zoom) + Mathf.Sign(scroll));
            else ZoomAt(Input.mousePosition, Zoom * (scroll > 0 ? 1.25f : 0.8f));
        }
        bool click = false;
        if (TouchUi) Pinch();
        else
        {
            if (Input.GetMouseButtonDown(2)) dragOrigin = cam.ScreenToWorldPoint(Input.mousePosition);
            if (Input.GetMouseButton(2))
                cam.transform.position += dragOrigin - cam.ScreenToWorldPoint(Input.mousePosition);
            click = LeftButton(overUi);
        }
        // Arrows/WASD walk the champion in the close view and move the camera over the map overview.
        bool keysFree = hud == null || !hud.Modal;
        int kx = keysFree ? (Key(KeyCode.RightArrow, KeyCode.D) ? 1 : 0) - (Key(KeyCode.LeftArrow, KeyCode.A) ? 1 : 0) : 0;
        int ky = keysFree ? (Key(KeyCode.DownArrow, KeyCode.S) ? 1 : 0) - (Key(KeyCode.UpArrow, KeyCode.W) ? 1 : 0) : 0;
        bool walking = Zoom >= 1f && ChampionId.HasValue && PlayerId != null;
        if (MovementLocked) Walk(Vector2Int.zero);
        else if (walking || PadWalk != Vector2Int.zero)
        {
            var dir = PadWalk != Vector2Int.zero ? PadWalk : new Vector2Int(kx, ky);
            if (!walking && dir != Vector2Int.zero && ChampionId.HasValue) CloseView();
            Walk(dir);
        }
        else
        {
            var move = new Vector3(kx, -ky, 0);
            if (move.sqrMagnitude > 0) FollowCamera = false;
            cam.transform.position += move * cam.orthographicSize * Time.deltaTime * 1.5f;
        }
        if (Input.GetMouseButtonDown(2)) FollowCamera = false;
        if (Input.GetKeyDown(KeyCode.F)) FollowCamera = FollowId.HasValue;
        var fid = FollowCamera ? FollowId : null;
        if (fid.HasValue && EntityById(fid.Value)?["pos"] is JObject cp && (int)cp["layer"] != Layer)
            SetLayer((int)cp["layer"]);
        if (fid.HasValue && gos.TryGetValue(fid.Value, out var champ) && champ.Root.activeSelf)
        {
            var t = champ.Root.transform.position;
            cam.transform.position = Vector3.Lerp(cam.transform.position, new Vector3(t.x, t.y, -10f), 1f - Mathf.Exp(-Time.deltaTime * 4f));
        }
        if (Zoom >= 1f)
        {
            // Snap to the screen pixel grid so pixel art never shimmers.
            float unit = 1f / (Chibi.PixelsPerUnit * Mathf.Round(Zoom));
            var cp2 = cam.transform.position;
            cam.transform.position = new Vector3(Mathf.Round(cp2.x / unit) * unit, Mathf.Round(cp2.y / unit) * unit, -10f);
        }
        if (TouchUi)
        {
            TouchControls(hud);
            return;
        }
        if (overUi && !digStart.HasValue) return;
        if (DigMode && DigControls()) return;
        if (click)
        {
            if (PendingAction != null) ResolvePending(Input.mousePosition);
            else PickAt(Input.mousePosition);
        }
        // Right click with one of our pawns selected: go there.
        if (Input.GetMouseButtonDown(1) && Selected.HasValue && IsMine(Selected.Value))
        {
            PendingAction = new JObject { ["kind"] = "move", ["id"] = "move", ["needs_target"] = "cell" };
            ResolvePending(Input.mousePosition);
        }
        if (Input.GetKeyDown(KeyCode.C) && ChampionId.HasValue)
        {
            Select(ChampionId);
            FocusOn(ChampionId.Value);
        }
    }

    /// Left button on the map outside of dig mode: a drag moves the view, a click (released without
    /// moving) is returned true to select or to pick the order's target.
    bool LeftButton(bool overUi)
    {
        if (DigMode)
        {
            leftDown = false;
            return false;
        }
        var mp = Input.mousePosition;
        if (Input.GetMouseButtonDown(0) && !overUi)
        {
            leftDown = true;
            leftDragged = false;
            leftStart = mp;
            leftOrigin = cam.ScreenToWorldPoint(mp);
        }
        if (!leftDown) return false;
        if (!leftDragged && Vector2.Distance(mp, leftStart) > 6f) leftDragged = true;
        if (leftDragged)
        {
            FollowCamera = false;
            cam.transform.position += leftOrigin - cam.ScreenToWorldPoint(mp);
        }
        if (Input.GetMouseButton(0)) return false;
        leftDown = false;
        return !leftDragged;
    }

    /// <summary>Esc or back: drops the order waiting for a target or the dig; false if there was none.</summary>
    public bool CancelCurrent()
    {
        if (PendingAction == null && !DigMode) return false;
        PendingAction = null;
        DigMode = false;
        return true;
    }

    /// Zoom keeping the world point under <paramref name="screen"/> still: whole steps in the close
    /// view (pixel-perfect), free below 1 (map overview).
    void ZoomAt(Vector3 screen, float target)
    {
        var before = cam.ScreenToWorldPoint(screen);
        Zoom = target >= 1f ? Mathf.Clamp(Mathf.Round(target), 1f, 8f) : Mathf.Clamp(target, 0.1f, 1f);
        ApplyZoom();
        cam.transform.position += before - cam.ScreenToWorldPoint(screen);
    }

    /// Two fingers: pinch to zoom, move together to pan.
    void Pinch()
    {
        if (Input.touchCount < 2)
        {
            pinching = false;
            return;
        }
        Touch a = Input.GetTouch(0), b = Input.GetTouch(1);
        Vector2 mid = (a.position + b.position) / 2f;
        float dist = Mathf.Max(1f, Vector2.Distance(a.position, b.position));
        if (!pinching)
        {
            pinching = true;
            pinchStartDist = dist;
            pinchStartZoom = Zoom;
            dragOrigin = cam.ScreenToWorldPoint(mid);
            touchMoved = true;
            CancelDig();
        }
        FollowCamera = false;
        float ratio = dist / pinchStartDist;
        // Close view zooms in whole steps: ask for a little more stretch before each step.
        float target = pinchStartZoom >= 1f ? pinchStartZoom * Mathf.Pow(ratio, 1.3f) : pinchStartZoom * ratio;
        if (pinchStartZoom >= 1f && target < 1f && ratio > 0.6f) target = 1f;
        ZoomAt(mid, target);
        cam.transform.position += dragOrigin - cam.ScreenToWorldPoint(mid);
    }

    /// One finger: drag pans (or draws the dig rectangle), tap selects or picks the order's target,
    /// long press sends the selected pawn of ours there. Touches starting on the HUD are left to it.
    void TouchControls(SimHud hud)
    {
        if (Input.touchCount != 1 || pinching)
        {
            if (Input.touchCount == 0) pinching = false;
            return;
        }
        var t = Input.GetTouch(0);
        var w = cam.ScreenToWorldPoint(t.position);
        var cell = new Vector2Int(Mathf.FloorToInt(w.x), Mathf.FloorToInt(-w.y));
        if (t.phase == TouchPhase.Began)
        {
            touchStart = t.position;
            touchStartTime = Time.time;
            touchMoved = longPressDone = false;
            touchOnUi = hud != null && hud.IsOverUi(t.position);
            dragOrigin = w;
            if (DigMode && !touchOnUi) digStart = cell;
            return;
        }
        if (touchOnUi) return;
        float slop = (Screen.dpi > 0 ? Screen.dpi : 160f) * 0.1f;
        if (!touchMoved && Vector2.Distance(t.position, touchStart) > slop) touchMoved = true;
        if (DigMode && digStart.HasValue)
        {
            DrawDigRect(digStart.Value, cell);
            if (t.phase == TouchPhase.Ended) FinishDig(digStart.Value, cell);
            if (t.phase == TouchPhase.Canceled) CancelDig();
            return;
        }
        if (touchMoved)
        {
            FollowCamera = false;
            cam.transform.position += dragOrigin - w;
            return;
        }
        if (!longPressDone && Time.time - touchStartTime > 0.5f && PendingAction == null && Selected.HasValue && IsMine(Selected.Value))
        {
            longPressDone = true;
            PendingAction = new JObject { ["kind"] = "move", ["id"] = "move", ["needs_target"] = "cell" };
            ResolvePending(t.position);
#if UNITY_ANDROID || UNITY_IOS
            Handheld.Vibrate();
#endif
            return;
        }
        if (t.phase == TouchPhase.Ended && !longPressDone)
        {
            if (PendingAction != null) ResolvePending(t.position);
            else PickAt(t.position);
        }
    }

    /// Click or tap on the map: selects what is there and tells the HUD.
    void PickAt(Vector3 screen)
    {
        var id = Pick(screen);
        Select(id);
        if (id.HasValue) Picked?.Invoke(id.Value);
    }

    static bool Key(KeyCode a, KeyCode b) => Input.GetKey(a) || Input.GetKey(b);

    Vector2Int MouseCell()
    {
        var w = cam.ScreenToWorldPoint(Input.mousePosition);
        return new Vector2Int(Mathf.FloorToInt(w.x), Mathf.FloorToInt(-w.y));
    }

    /// Rectangle drag for digging; true while it consumes the mouse.
    bool DigControls()
    {
        if (Input.GetMouseButtonDown(1))
        {
            DigMode = false;
            return true;
        }
        if (Input.GetMouseButtonDown(0)) digStart = MouseCell();
        if (!digStart.HasValue) return false;
        DrawDigRect(digStart.Value, MouseCell());
        if (Input.GetMouseButtonUp(0)) FinishDig(digStart.Value, MouseCell());
        return true;
    }

    static RectInt DigArea(Vector2Int a, Vector2Int b) =>
        new(Mathf.Min(a.x, b.x), Mathf.Min(a.y, b.y), Mathf.Abs(a.x - b.x) + 1, Mathf.Abs(a.y - b.y) + 1);

    void DrawDigRect(Vector2Int a, Vector2Int b)
    {
        if (digRect == null) digRect = NewSprite("DigRect", null, Shapes.Pixel, new Color(1f, 0.75f, 0.2f, 0.35f), 31500);
        var r = DigArea(a, b);
        digRect.enabled = true;
        digRect.transform.position = new Vector3(r.x, -r.y, 0);
        digRect.transform.localScale = new Vector3(r.width, r.height, 1);
    }

    void FinishDig(Vector2Int a, Vector2Int b)
    {
        var r = DigArea(a, b);
        DigMode = false;
        SendPlayerCommand(new JObject
        {
            ["type"] = "designate", ["layer"] = (string)Layers[Layer]["id"],
            ["rect"] = new JArray(r.x, r.y, r.width, r.height),
        });
    }

    void CancelDig()
    {
        digStart = null;
        if (digRect != null) digRect.enabled = false;
    }

    /// <summary>Entity under a screen point (pawns win over buildings).</summary>
    public long? Pick(Vector3 screen)
    {
        var w = cam.ScreenToWorldPoint(screen);
        long? best = null;
        float bd = float.MaxValue;
        foreach (var kv in gos)
        {
            if (!kv.Value.Root.activeSelf) continue;
            float d = Vector2.Distance(w, kv.Value.Root.transform.position) + (kv.Value.Building ? 0.6f : 0f);
            float limit = kv.Value.Building ? 1.6f : 0.8f;
            if (d < limit && d < bd)
            {
                bd = d;
                best = kv.Key;
            }
        }
        return best;
    }

    public JToken EntityById(long id) => State?["snapshot"]?["entities"]?.FirstOrDefault(e => (long)e["id"] == id);

    public Vector3 WorldToScreen(Vector3 world) => cam.WorldToScreenPoint(world);

    public Vector3 CellCenter(JToken pos) => new((int)pos["x"] + 0.5f, -(int)pos["y"] - 0.5f, 0);

    Vector3 BuildingCenter(JToken pos) => CellCenter(pos);

    static SpriteRenderer NewSprite(string name, Transform parent, Sprite sprite, Color color, int order)
    {
        var go = new GameObject(name);
        if (parent != null) go.transform.SetParent(parent, false);
        var sr = go.AddComponent<SpriteRenderer>();
        sr.sprite = sprite;
        sr.color = color;
        sr.sortingOrder = order;
        return sr;
    }

    public bool FogReady => fogInitialized;
}
