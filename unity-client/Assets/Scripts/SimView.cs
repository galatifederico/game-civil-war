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
    public JObject PendingAction { get; set; }
    public bool FollowCamera { get; set; }
    bool fogChosen;

    Camera cam;
    Transform zonesRoot, cellsRoot, entitiesRoot;
    readonly Dictionary<long, EntityGo> gos = new();
    readonly List<GameObject> cellPool = new();
    SpriteRenderer fogRenderer, selectionRing;
    Texture2D fogTex;
    bool fogInitialized;
    Vector3 dragOrigin;

    class EntityGo
    {
        public GameObject Root;
        public SpriteRenderer Body, Outline, BarBg, Bar, Marker;
        // Chibi layers: base (race), cloth (tinted with the faction colour), accessory (class or crown).
        public SpriteRenderer Base, Cloth, Acc;
        public Sprite[] BaseFrames, ClothFrames, AccFrames;
        public int Dir;
        public float Anim;
        public bool Dead;
        public Vector3 Target;
        public bool Building;
    }

    void Start()
    {
        foreach (var a in System.Environment.GetCommandLineArgs())
            if (a.StartsWith("--server=")) serverUrl = a.Substring(9);
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
            if (PlayerId == null && State?["snapshot"]?["players"] is JObject players && players.Properties().Any())
            {
                PlayerId = players.Properties().First().Name;
                if (!fogChosen) FogFaction = (string)players[PlayerId]["faction"] ?? "";
            }
            if (PlayerId != null)
                yield return Api.Get("/api/ui/player/" + UnityEngine.Networking.UnityWebRequest.EscapeURL(PlayerId), j => PlayerInfo = (JObject)j, _ => { });
            if (Selected.HasValue)
            {
                var id = Selected.Value;
                yield return Api.Get($"/api/entities/{id}?truth=true", j => { if (Selected == id) SelectedEntity = (JObject)j; }, _ => { });
                yield return Api.Get($"/api/entities/{id}/ai", j => { if (Selected == id) SelectedAi = (JObject)j; }, _ => { });
            }
            yield return new WaitForSeconds(pollSeconds);
        }
    }

    // ── Commands ──────────────────────────────────────────────────────────────

    public void SendControl(JObject body) => StartCoroutine(Api.Post("/api/control", body, j => Control = (JObject)j, e => LastCommandResult = e));

    public void SendCommand(JObject cmd) => StartCoroutine(Api.Post("/api/commands?now=true", cmd,
        j => LastCommandResult = (string)j["ok"] ?? j.ToString(), e => LastCommandResult = "Rifiutato: " + e));

    public void Select(long? id)
    {
        Selected = id;
        SelectedEntity = null;
        SelectedAi = null;
        SelectedActions = null;
        PendingAction = null;
        if (id.HasValue && IsMine(id.Value))
            StartCoroutine(Api.Get($"/api/ui/actions/{id.Value}", j => { if (Selected == id) SelectedActions = (JArray)j; }, _ => { }));
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

    public void SendPlayerCommand(JObject cmd)
    {
        if (PlayerId == null) return;
        cmd["player"] = PlayerId;
        SendCommand(cmd);
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
        cam.orthographicSize = Mathf.Min(cam.orthographicSize, 14f);
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
        var ground = NewSprite("Ground", zonesRoot, Shapes.Pixel, new Color(0.84f, 0.8f, 0.7f), 0);
        ground.transform.localScale = new Vector3(w, h, 1);
        foreach (var z in Map["zones"].Where(z => (int)z["layer"] == layer))
        {
            var sr = NewSprite((string)z["name"], zonesRoot, Shapes.Pixel, ZoneColor(z), 1);
            sr.transform.position = new Vector3((int)z["x"], -(int)z["y"], 0);
            sr.transform.localScale = new Vector3((int)z["w"], (int)z["h"], 1);
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
            foreach (var end in new[] { p["a"], p["b"] })
                if ((int)end["layer"] == layer)
                {
                    var sr = NewSprite("Portal", zonesRoot, Shapes.Get("diamond"), new Color(0.85f, 0.3f, 0.2f), 6);
                    sr.transform.position = CellCenter(end);
                    sr.transform.localScale = Vector3.one * 0.9f;
                }
        // Fog texture sized to the layer.
        if (fogRenderer == null) fogRenderer = NewSprite("Fog", null, Shapes.Pixel, Color.white, 32000);
        fogTex = new Texture2D(w, h, TextureFormat.RGBA32, false) { filterMode = FilterMode.Bilinear, wrapMode = TextureWrapMode.Clamp };
        fogRenderer.sprite = Sprite.Create(fogTex, new Rect(0, 0, w, h), new Vector2(0, 1), 1);
        fogRenderer.transform.position = Vector3.zero;
        fogInitialized = false;
        FitCamera(w, h);
        if (State != null) OnState();
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
        float viewW = cam.orthographicSize * 2f * aspect;
        cam.transform.position = new Vector3(w / 2f + viewW * (1f - usable) / 2f, -h / 2f, -10f);
    }

    // ── State → sprites ───────────────────────────────────────────────────────

    public JToken SpriteDef(string key) => State?["sprites"]?[key];

    void OnState()
    {
        if (State == null || Map == null) return;
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
            go.Body.color = Shapes.Parse((string)sp?["color"], new Color(0.63f, 0.53f, 0.5f));
            bool destroyed = (float?)e["building"]?["hp"] <= 0f;
            go.Body.color = destroyed ? go.Body.color * 0.45f : go.Body.color;
            go.Outline.color = outline;
            return;
        }
        var race = (string)e["race"];
        var rs = SpriteDef("race:" + race);
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

    void DrawFog()
    {
        var fog = State["fog"];
        bool on = fog != null && fog.Type != JTokenType.Null;
        fogRenderer.enabled = on;
        if (!on) return;
        int w = fogTex.width, h = fogTex.height;
        var dark = new Color32(20, 16, 12, 150);
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
            int order = 100 + Mathf.RoundToInt(-go.Root.transform.position.y * 4f) * 8;
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

    void CameraControls()
    {
        var hud = GetComponent<SimHud>();
        bool overUi = hud != null && hud.IsOverUi(Input.mousePosition);
        float scroll = Input.mouseScrollDelta.y;
        if (!overUi && Mathf.Abs(scroll) > 0.01f)
        {
            var before = cam.ScreenToWorldPoint(Input.mousePosition);
            cam.orthographicSize = Mathf.Clamp(cam.orthographicSize * (1f - scroll * 0.1f), 3f, 120f);
            var after = cam.ScreenToWorldPoint(Input.mousePosition);
            cam.transform.position += before - after;
        }
        if (Input.GetMouseButtonDown(2)) dragOrigin = cam.ScreenToWorldPoint(Input.mousePosition);
        if (Input.GetMouseButton(2))
            cam.transform.position += dragOrigin - cam.ScreenToWorldPoint(Input.mousePosition);
        var move = new Vector3(Input.GetAxisRaw("Horizontal"), Input.GetAxisRaw("Vertical"), 0);
        if (move.sqrMagnitude > 0) FollowCamera = false;
        cam.transform.position += move * cam.orthographicSize * Time.deltaTime * 1.5f;
        if (FollowCamera && ChampionId.HasValue && gos.TryGetValue(ChampionId.Value, out var champ) && champ.Root.activeSelf)
        {
            var t = champ.Root.transform.position;
            cam.transform.position = Vector3.Lerp(cam.transform.position, new Vector3(t.x, t.y, -10f), 1f - Mathf.Exp(-Time.deltaTime * 4f));
        }
        if (overUi) return;
        if (Input.GetMouseButtonDown(0))
        {
            if (PendingAction != null) ResolvePending(Input.mousePosition);
            else Select(Pick(Input.mousePosition));
        }
        // Right click with one of our pawns selected: go there.
        if (Input.GetMouseButtonDown(1) && Selected.HasValue && IsMine(Selected.Value))
        {
            PendingAction = new JObject { ["kind"] = "move", ["id"] = "move", ["needs_target"] = "cell" };
            ResolvePending(Input.mousePosition);
        }
        if (Input.GetKeyDown(KeyCode.Escape)) PendingAction = null;
        if (Input.GetKeyDown(KeyCode.C) && ChampionId.HasValue)
        {
            Select(ChampionId);
            FocusOn(ChampionId.Value);
        }
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

    Vector3 BuildingCenter(JToken pos) => new((int)pos["x"] + 1f, -(int)pos["y"] - 1f, 0);

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
