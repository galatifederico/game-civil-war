using System.Collections.Generic;
using System.Linq;
using Newtonsoft.Json.Linq;
using UnityEngine;

/// <summary>
/// Immediate-mode HUD. Top centre: victory points, guild fund and team size, always on screen.
/// Right column: Mappa (world map with how many of ours stand in each area), Inventario (what the team
/// holds), Team (team summary, then every member with race, class, health, mood, money and activity),
/// Opzioni (time, view, dig, save/load, fog, server, exit). Left column: Piccione Viaggiatore (news
/// channels, badge with the unread messages of the main one), Classifica ufficiale, Risorse mondiali
/// (prices, inflation, money, circumstances) and Manuale. Tapping a pawn opens its card (three bag slots,
/// AZIONI); next to the champion, A / Space opens the actions with whoever is there (talk first), and an
/// item opens its detail (what it does, give it to a member, use it). One button, always visible in the
/// top-left corner, hides all of it and brings it back (Tab). On phones a joystick walks the champion;
/// while a window is open the champion stands still and the joystick steps aside.
/// Esc (the back button on phones) closes what is open, otherwise asks whether to quit.
/// </summary>
[RequireComponent(typeof(SimView))]
public class SimHud : MonoBehaviour
{
    SimView view;
    float scale = 1f;
    GUIStyle box, title, small, label, zoneLabel, zoneMine, button, fake, hover, good, toggle, textBox, banner;
    GUIStyle cell, cellHead, statValue, statLabel, badge, rowButton, padStyle, big, bigButton, slotStyle, promptStyle;
    Texture2D panelTex, barTex, barFillTex, barLowTex, dimTex, badgeTex, padTex, circleBase, circleKnob, circleA;

    enum Win { None, Map, Inventory, Team, Options, Pigeon, Ranking, Economy, Manual, Card, Actions, Item }
    Win win;
    /// Windows to go back to with Esc / "Indietro" (Team → card → item…).
    readonly List<Win> stack = new();
    JObject interactions, itemInfo;
    long actionsTarget;
    string itemId;
    int giveIndex;
    string talkName, talkLine;
    float talkClosedAt;
    /// Pawn or building next to the champion: what A / Space acts on.
    long? nearby;
    string lastResult;
    float toastUntil;
    bool hidden;
    bool exitAsk;
    string bannerKey;
    float bannerUntil, noticeUntil;
    long noticeId = -1;
    string noticeText;
    int speedIndex = 1;
    bool layerMenu;
    string serverField;
    bool serverOpen;
    long seenMain = -1;

    // Areas covered by the HUD in the last frame: the map takes no clicks there.
    readonly List<Rect> uiRects = new();
    Rect winRect, aRect;
    float topBottom;
    Vector2 scrollWin;
    // Touch: windows and the card scroll by dragging a finger on them.
    int touchScroll;
    bool touchDragged;

    bool ConnectShown => view.Map == null || serverOpen;
    bool PendingShown => view.PendingAction != null || view.DigMode;
    bool PadShown => touch && view.State != null && view.ChampionId.HasValue && !ConnectShown;
    /// A window over the middle of the screen (or someone talking): no walking, no joystick.
    bool MenuOpen => (win != Win.None && win != Win.Map) || talkLine != null;
    /// While true the keyboard belongs to the HUD (exit question, server address).
    public bool Modal => exitAsk || ConnectShown;
    bool touch => SimView.TouchUi;
    float rowH => touch ? 46 : 30;
    float colW => touch ? 124 : 116;
    static readonly (string name, int ms)[] Speeds = { ("Lenta", 1000), ("Normale", 400), ("Veloce", 120), ("Turbo", 30) };

    void Awake()
    {
        view = GetComponent<SimView>();
        view.Picked += id => OpenCard(id);
    }

    /// Opens a window by name (mappa, inventario, team, opzioni, piccione, classifica, risorse, manuale),
    /// for automated screenshots; "nascondi" hides the HUD.
    public void OpenByName(string name)
    {
        if (name == "nascondi") SetHidden(true);
        else if (name == "scheda" && (view.Selected ?? view.ChampionId) is long sel) OpenCard(sel);
        else if (name == "vicino" && view.NearbyTarget() is long near) OpenActions(near, Win.None);
        else if (name == "parla" && view.NearbyTarget() is long who) StartTalk(who);
        else if (name.StartsWith("oggetto:")) OpenItem(name.Substring(8), Win.None);
        else if (System.Enum.TryParse<Win>(name switch
                 {
                     "mappa" => "Map", "inventario" => "Inventory", "team" => "Team", "opzioni" => "Options",
                     "piccione" => "Pigeon", "classifica" => "Ranking", "risorse" => "Economy", "manuale" => "Manual", _ => name,
                 }, true, out var w)) Open(w);
    }

    public bool IsOverUi(Vector3 mouse)
    {
        // The exit question is modal: the map takes no input behind it.
        if (exitAsk) return true;
        var p = new Vector2(mouse.x, Screen.height - mouse.y) / scale;
        foreach (var r in uiRects)
            if (r.Contains(p)) return true;
        return false;
    }

    /// Pokémon-like window: white fill, rounded double border (9-sliced).
    static Texture2D Frame(Color fill, Color edge, Color inner)
    {
        const int n = 12;
        var t = new Texture2D(n, n) { filterMode = FilterMode.Point, wrapMode = TextureWrapMode.Clamp };
        var clear = new Color(0, 0, 0, 0);
        for (int y = 0; y < n; y++)
        for (int x = 0; x < n; x++)
        {
            int d = Mathf.Min(Mathf.Min(x, n - 1 - x), Mathf.Min(y, n - 1 - y));
            bool corner = (x < 2 || x > n - 3) && (y < 2 || y > n - 3);
            bool cornerTip = (x == 0 || x == n - 1) && (y == 0 || y == n - 1);
            Color c = d == 0 ? edge : d == 1 ? edge : d == 2 ? inner : fill;
            if (cornerTip) c = clear;
            else if (corner && d == 1 && (x == 1 || x == n - 2) && (y == 1 || y == n - 2)) c = edge;
            t.SetPixel(x, y, c);
        }
        t.Apply();
        return t;
    }

    /// Smooth disc with a ring, for the joystick and the A button.
    static Texture2D Circle(Color fill, Color ring)
    {
        const int n = 96;
        var t = new Texture2D(n, n) { filterMode = FilterMode.Bilinear, wrapMode = TextureWrapMode.Clamp };
        float r = n / 2f - 1;
        for (int y = 0; y < n; y++)
        for (int x = 0; x < n; x++)
        {
            float d = Vector2.Distance(new Vector2(x + 0.5f, y + 0.5f), new Vector2(n / 2f, n / 2f));
            Color c = d > r ? Color.clear : d > r - 4 ? ring : fill;
            c.a *= Mathf.Clamp01(r - d + 1);
            t.SetPixel(x, y, c);
        }
        t.Apply();
        return t;
    }

    static Texture2D Tex(Color c)
    {
        var t = new Texture2D(1, 1);
        t.SetPixel(0, 0, c);
        t.Apply();
        return t;
    }

    void InitStyles()
    {
        if (box != null) return;
        // Palette of the classic handheld menus: white windows, slate borders, dark text.
        var ink = new Color(0.19f, 0.2f, 0.26f);
        var edge = new Color(0.22f, 0.25f, 0.34f);
        panelTex = Frame(new Color(0.97f, 0.97f, 0.96f, 0.97f), edge, new Color(0.56f, 0.66f, 0.8f));
        var btnTex = Frame(new Color(0.88f, 0.92f, 0.97f), edge, new Color(0.75f, 0.82f, 0.92f));
        var btnHover = Frame(new Color(1f, 0.96f, 0.78f), edge, new Color(0.95f, 0.8f, 0.4f));
        var btnDown = Frame(new Color(0.78f, 0.84f, 0.93f), edge, new Color(0.56f, 0.66f, 0.8f));
        badgeTex = Frame(new Color(0.86f, 0.2f, 0.18f), new Color(0.55f, 0.08f, 0.08f), new Color(0.95f, 0.4f, 0.35f));
        padTex = Frame(new Color(0.97f, 0.97f, 0.96f, 0.55f), new Color(0.22f, 0.25f, 0.34f, 0.7f), new Color(0.56f, 0.66f, 0.8f, 0.6f));
        barTex = Tex(new Color(0.2f, 0.25f, 0.3f, 0.25f));
        barFillTex = Tex(new Color(0.35f, 0.78f, 0.45f));
        barLowTex = Tex(new Color(0.9f, 0.4f, 0.3f));
        circleBase = Circle(new Color(1f, 1f, 1f, 0.22f), new Color(0.22f, 0.25f, 0.34f, 0.6f));
        circleKnob = Circle(new Color(0.97f, 0.97f, 0.96f, 0.85f), new Color(0.22f, 0.25f, 0.34f, 0.9f));
        circleA = Circle(new Color(0.86f, 0.3f, 0.28f, 0.85f), new Color(0.45f, 0.1f, 0.1f, 0.95f));
        dimTex = Tex(new Color(0.05f, 0.06f, 0.1f, 0.55f));
        var slice = new RectOffset(4, 4, 4, 4);
        box = new GUIStyle(GUI.skin.box) { normal = { background = panelTex }, border = slice, padding = new RectOffset(12, 12, 8, 8), alignment = TextAnchor.UpperLeft };
        title = new GUIStyle(GUI.skin.label) { fontStyle = FontStyle.Bold, fontSize = 13, normal = { textColor = new Color(0.2f, 0.36f, 0.66f) } };
        label = new GUIStyle(GUI.skin.label) { wordWrap = true, fontSize = 12, normal = { textColor = ink } };
        small = new GUIStyle(label) { fontSize = 11, normal = { textColor = new Color(0.45f, 0.47f, 0.52f) } };
        fake = new GUIStyle(label) { normal = { textColor = new Color(0.8f, 0.22f, 0.18f) } };
        good = new GUIStyle(label) { normal = { textColor = new Color(0.16f, 0.55f, 0.28f) } };
        cell = new GUIStyle(label) { wordWrap = false, clipping = TextClipping.Clip };
        cellHead = new GUIStyle(cell) { fontStyle = FontStyle.Bold, normal = { textColor = new Color(0.2f, 0.36f, 0.66f) } };
        zoneLabel = new GUIStyle(GUI.skin.label) { fontSize = 11, fontStyle = FontStyle.Bold, normal = { textColor = new Color(0.98f, 0.98f, 0.95f) } };
        zoneMine = new GUIStyle(zoneLabel) { normal = { textColor = new Color(0.55f, 1f, 0.6f) } };
        hover = new GUIStyle(box) { fontSize = 12, wordWrap = true, padding = new RectOffset(8, 8, 5, 5), normal = { background = panelTex, textColor = ink } };
        button = new GUIStyle(GUI.skin.button)
        {
            fontSize = 12, border = slice, padding = new RectOffset(6, 6, 4, 4),
            normal = { background = btnTex, textColor = ink }, hover = { background = btnHover, textColor = ink },
            active = { background = btnDown, textColor = ink }, focused = { background = btnTex, textColor = ink },
            onNormal = { background = btnHover, textColor = ink }, onHover = { background = btnHover, textColor = ink },
            onActive = { background = btnDown, textColor = ink },
        };
        rowButton = new GUIStyle(button) { alignment = TextAnchor.MiddleLeft, wordWrap = false, clipping = TextClipping.Clip };
        toggle = new GUIStyle(GUI.skin.toggle) { normal = { textColor = ink }, onNormal = { textColor = ink }, hover = { textColor = ink }, onHover = { textColor = ink } };
        textBox = new GUIStyle(box) { fontSize = 16, wordWrap = true, padding = new RectOffset(20, 20, 14, 14), normal = { background = panelTex, textColor = ink } };
        banner = new GUIStyle(box) { fontSize = 15, fontStyle = FontStyle.Bold, alignment = TextAnchor.MiddleCenter, normal = { background = panelTex, textColor = ink } };
        statValue = new GUIStyle(GUI.skin.label) { fontSize = 17, fontStyle = FontStyle.Bold, alignment = TextAnchor.MiddleCenter, normal = { textColor = ink } };
        statLabel = new GUIStyle(GUI.skin.label) { fontSize = 10, alignment = TextAnchor.MiddleCenter, normal = { textColor = new Color(0.45f, 0.47f, 0.52f) } };
        badge = new GUIStyle(GUI.skin.box) { normal = { background = badgeTex, textColor = Color.white }, border = slice, fontSize = 11, fontStyle = FontStyle.Bold, alignment = TextAnchor.MiddleCenter, padding = new RectOffset(2, 2, 1, 1) };
        big = new GUIStyle(label) { fontSize = 15 };
        bigButton = new GUIStyle(button) { fontSize = 15, fontStyle = FontStyle.Bold };
        slotStyle = new GUIStyle(button) { normal = { background = btnTex }, hover = { background = btnHover } };
        promptStyle = new GUIStyle(box) { fontSize = 13, alignment = TextAnchor.MiddleCenter, wordWrap = false, clipping = TextClipping.Clip, normal = { background = panelTex, textColor = ink } };
        padStyle = new GUIStyle(GUI.skin.box) { normal = { background = padTex, textColor = ink }, border = slice, fontSize = 22, fontStyle = FontStyle.Bold, alignment = TextAnchor.MiddleCenter };
        if (touch)
        {
            // Finger-sized targets: bigger text and taller buttons.
            button.fontSize = rowButton.fontSize = 14;
            button.padding = new RectOffset(10, 10, 10, 10);
            rowButton.padding = new RectOffset(10, 10, 10, 10);
            label.fontSize = small.fontSize = cell.fontSize = cellHead.fontSize = 13;
            fake.fontSize = good.fontSize = 13;
            title.fontSize = 15;
            statValue.fontSize = 19;
            big.fontSize = bigButton.fontSize = 17;
            promptStyle.fontSize = 15;
            statLabel.fontSize = 11;
            box.padding = new RectOffset(10, 10, 6, 6);
        }
    }

    /// Keyboard hint after a button label, hidden on phones.
    string Key(string k) => touch ? "" : " (" + k + ")";

    void OnGUI()
    {
        InitStyles();
        // Phones: size from the screen density (keeping at least 360 points of height), PC: from the height.
        scale = touch ? Mathf.Clamp((Screen.dpi > 0 ? Screen.dpi : 320f) / 160f, 1f, Screen.height / 360f) : Mathf.Max(1f, Screen.height / 900f);
        if (touchDragged && (Event.current.type == EventType.MouseUp || Event.current.type == EventType.MouseDown))
        {
            // A finger that scrolled a window does not also press the button it ends on.
            GUIUtility.hotControl = 0;
            Event.current.Use();
        }
        if (Event.current.type == EventType.Layout) uiRects.Clear();
        GUI.matrix = Matrix4x4.Scale(new Vector3(scale, scale, 1));
        float w = Screen.width / scale, h = Screen.height / scale;
        // Behind the exit question nothing else can be pressed.
        GUI.enabled = !exitAsk;

        if (view.Map != null && view.Zoom < 1f) ZoneLabels();
        topBottom = 8;
        if (ConnectShown) ConnectWindow(w, h);
        else if (view.State == null)
            GUI.Label(new Rect(w / 2 - 200, h / 2 - 20, 400, 40), "In attesa del server…", banner);
        else
        {
            if (!hidden)
            {
                TopBar(w);
                LeftColumn();
                RightColumn(w);
                if (win == Win.Map) MapWindow(w, h);
                else if (win != Win.None) Window(w, h);
                if (win == Win.None && talkLine == null) Notice(w, h);
                Toast(w, h);
            }
            LocationBanner(w);
            bool walkable = view.Zoom >= 1f && !MenuOpen && view.ChampionId.HasValue;
            if (PadShown && walkable) Joystick(w, h);
            else if (!touch && walkable && nearby.HasValue && !hidden) Prompt(new Rect(w / 2 - 160, h - 52, 320, 34));
            if (talkLine != null) TalkBox(w, h);
            if (!touch && !exitAsk && !MenuOpen) HoverTip();
        }
        if (PendingShown) PendingBox(w);
        HideToggle();

        GUI.enabled = true;
        if (exitAsk) ExitWindow(w, h);
    }

    Rect Ui(Rect r)
    {
        uiRects.Add(r);
        return r;
    }

    // ── Always on screen ─────────────────────────────────────────────────────

    /// The one button that hides the whole HUD and brings it back.
    void HideToggle()
    {
        var r = Ui(new Rect(8, 8, colW, touch ? 40 : 28));
        if (GUI.Button(r, hidden ? "▸ Mostra" + Key("Tab") : "◂ Nascondi" + Key("Tab"), button)) SetHidden(!hidden);
    }

    void SetHidden(bool on)
    {
        hidden = on;
        if (on && win != Win.Map) Close();
    }

    /// Victory points, guild fund and team size, with the clock underneath.
    void TopBar(float w)
    {
        var p = view.PlayerInfo;
        long tick = (long?)view.State?["snapshot"]?["tick"] ?? 0;
        bool paused = (bool?)view.Control?["paused"] ?? false;
        float bw = Mathf.Min(touch ? 440 : 400, w - 2 * colW - 40), bh = touch ? 58 : 50;
        var r = Ui(new Rect(w / 2 - bw / 2, 6, bw, bh));
        GUI.Box(r, GUIContent.none, box);
        var stats = new (string label, string value)[]
        {
            ("Punti vittoria", p == null ? "–" : Fmt(Score())),
            ("Fondo di gilda", p == null ? "–" : Fmt((double?)p["treasury"] ?? 0) + " €"),
            ("Team", p == null ? "–" : ((int?)p["summary"]?["members"] ?? p["members"]?.Count() ?? 0).ToString()),
        };
        float cw = (bw - 16) / stats.Length;
        for (int i = 0; i < stats.Length; i++)
        {
            var c = new Rect(r.x + 8 + i * cw, r.y + 4, cw, bh - 8);
            GUI.Label(new Rect(c.x, c.y, c.width, c.height * 0.6f), stats[i].value, statValue);
            GUI.Label(new Rect(c.x, c.y + c.height * 0.58f, c.width, c.height * 0.42f), stats[i].label, statLabel);
        }
        string clock = $"Giorno {tick / 24 + 1} · ore {tick % 24}:00" + (paused ? " · in pausa" : "");
        var cr = new Rect(w / 2 - 120, r.yMax + 1, 240, 18);
        GUI.Label(new Rect(cr.x + 1, cr.y + 1, cr.width, cr.height), clock, new GUIStyle(statLabel) { normal = { textColor = new Color(0, 0, 0, 0.6f) } });
        GUI.Label(cr, clock, new GUIStyle(statLabel) { normal = { textColor = Color.white } });
        topBottom = cr.yMax + 4;
    }

    /// Points that count for victory: the faction's victory points plus the relics it holds.
    long Score()
    {
        var f = (string)view.PlayerInfo?["faction"];
        return f == null ? 0 : (long?)view.State?["snapshot"]?["scores"]?[f] ?? (long?)view.PlayerInfo["victory_points"] ?? 0;
    }

    void LeftColumn()
    {
        float y = 8 + (touch ? 40 : 28) + 8;
        int unread = Unread();
        var pigeon = new Rect(8, y, colW, rowH + 6);
        if (SideButton(pigeon, "Piccione" + Key("P"), Win.Pigeon)) Open(Win.Pigeon);
        if (unread > 0)
        {
            var b = new Rect(pigeon.xMax - 26, pigeon.y - 6, 30, 20);
            GUI.Label(b, unread > 39 ? "40+" : unread.ToString(), badge);
        }
        y += rowH + 12;
        if (SideButton(new Rect(8, y, colW, rowH + 6), "Classifica" + Key("L"), Win.Ranking)) Open(Win.Ranking);
        y += rowH + 12;
        if (SideButton(new Rect(8, y, colW, rowH + 6), "Risorse" + Key("R"), Win.Economy)) Open(Win.Economy);
        y += rowH + 12;
        if (SideButton(new Rect(8, y, colW, rowH + 6), "Manuale" + Key("H"), Win.Manual)) Open(Win.Manual);
    }

    void RightColumn(float w)
    {
        float x = w - colW - 8, y = 8;
        if (SideButton(new Rect(x, y, colW, rowH + 6), "Mappa" + Key("M"), Win.Map)) Open(Win.Map);
        y += rowH + 12;
        if (SideButton(new Rect(x, y, colW, rowH + 6), "Inventario" + Key("I"), Win.Inventory)) Open(Win.Inventory);
        y += rowH + 12;
        if (SideButton(new Rect(x, y, colW, rowH + 6), "Team" + Key("T"), Win.Team)) Open(Win.Team);
        y += rowH + 12;
        if (SideButton(new Rect(x, y, colW, rowH + 6), "Opzioni" + Key("O"), Win.Options)) Open(Win.Options);
    }

    bool SideButton(Rect r, string text, Win which) => GUI.Button(Ui(r), win == which ? "▸ " + text : text, button);

    void Open(Win which)
    {
        if (win == which)
        {
            Close();
            return;
        }
        if (win == Win.Map) view.CloseView();
        if (win is Win.Card or Win.Actions && view.Selected.HasValue) view.Select(null);
        win = which;
        stack.Clear();
        hidden = false;
        scrollWin = Vector2.zero;
        layerMenu = false;
        view.PigeonOpen = which == Win.Pigeon;
        view.EconomyOpen = which == Win.Economy;
        if (which == Win.Map) view.ShowOverview(0);
        if (which == Win.Pigeon && view.FeedFilter == "main") MarkRead();
    }

    void Close()
    {
        if (win == Win.Map) view.CloseView();
        // The card, the actions and an item opened from them belong to the selected pawn.
        if ((win is Win.Card or Win.Actions || stack.Contains(Win.Card)) && view.Selected.HasValue) view.Select(null);
        win = Win.None;
        stack.Clear();
        view.PigeonOpen = view.EconomyOpen = false;
    }

    /// The window of the open button, between the two columns.
    void Window(float w, float h)
    {
        float x0 = colW + 16, x1 = w - colW - 16;
        winRect = Ui(new Rect(x0, topBottom, x1 - x0, h - topBottom - (PadShown ? 8 : 8)));
        GUILayout.BeginArea(winRect, box);
        GUILayout.BeginHorizontal();
        GUILayout.Label(WinTitle(win), new GUIStyle(title) { fontSize = title.fontSize + (win is Win.Card or Win.Actions or Win.Item ? 5 : 0) });
        GUILayout.FlexibleSpace();
        if (stack.Count > 0 && GUILayout.Button("◀ Indietro" + Key("Esc"), button, GUILayout.Width(touch ? 120 : 130))) Back();
        if (GUILayout.Button("✕ Chiudi" + (stack.Count == 0 ? Key("Esc") : ""), button, GUILayout.Width(touch ? 110 : 120))) Close();
        GUILayout.EndHorizontal();
        // Vertical only: a row a few points too wide must not bring up a horizontal bar.
        scrollWin = GUILayout.BeginScrollView(scrollWin, false, false, GUIStyle.none, GUI.skin.verticalScrollbar);
        float inner = winRect.width - 44;
        switch (win)
        {
            case Win.Inventory: InventoryWin(inner); break;
            case Win.Team: TeamWin(inner); break;
            case Win.Options: OptionsWin(inner); break;
            case Win.Pigeon: PigeonWin(inner); break;
            case Win.Ranking: RankingWin(inner); break;
            case Win.Economy: EconomyWin(inner); break;
            case Win.Manual: ManualWin(inner); break;
            case Win.Card: CardWin(inner); break;
            case Win.Actions: ActionsWin(inner); break;
            case Win.Item: ItemWin(inner); break;
        }
        GUILayout.EndScrollView();
        GUILayout.EndArea();
    }

    string WinTitle(Win w) => w switch
    {
        Win.Card => ((string)view.SelectedEntity?["name"] ?? "").ToUpperInvariant(),
        Win.Actions => ("Azioni con " + ((string)interactions?["name"] ?? "…")).ToUpperInvariant(),
        Win.Item => ((string)itemInfo?["name"] ?? "Oggetto").ToUpperInvariant(),
        Win.Inventory => "INVENTARIO DEL TEAM",
        Win.Team => "IL TUO TEAM",
        Win.Options => "OPZIONI",
        Win.Pigeon => "IL PICCIONE VIAGGIATORE",
        Win.Ranking => "CLASSIFICA UFFICIALE",
        Win.Economy => "RISORSE MONDIALI",
        Win.Manual => "MANUALE DEL GIOCATORE",
        _ => "",
    };

    // ── Map ──────────────────────────────────────────────────────────────────

    /// Members of our team on each map, and in each area of the map on screen.
    List<JToken> MemberPositions() =>
        view.PlayerInfo?["members"]?.Select(m => m["pos"]).Where(p => p != null && p.Type == JTokenType.Object).ToList() ?? new List<JToken>();

    static bool InZone(JToken z, JToken p) =>
        (int)p["layer"] == (int)z["layer"] && (int)p["x"] >= (int)z["x"] && (int)p["x"] < (int)z["x"] + (int)z["w"]
        && (int)p["y"] >= (int)z["y"] && (int)p["y"] < (int)z["y"] + (int)z["h"];

    /// Side panel next to the map overview: the maps of the world and the areas where ours are.
    void MapWindow(float w, float h)
    {
        float pw = Mathf.Min(touch ? 300 : 280, w * 0.34f);
        winRect = Ui(new Rect(w - colW - 16 - pw, topBottom, pw, h - topBottom - 8));
        GUILayout.BeginArea(winRect, box);
        GUILayout.BeginHorizontal();
        GUILayout.Label("MAPPA DEL MONDO", title);
        GUILayout.FlexibleSpace();
        if (GUILayout.Button("✕", button, GUILayout.Width(touch ? 46 : 30))) Close();
        GUILayout.EndHorizontal();
        scrollWin = GUILayout.BeginScrollView(scrollWin, false, false, GUIStyle.none, GUI.skin.verticalScrollbar);
        float inner = pw - 44;
        var pos = MemberPositions();
        GUILayout.Label("Sulla mappa, accanto al nome di ogni area, quanti dei tuoi ci sono.", small, GUILayout.Width(inner));
        var layers = view.Layers;
        GUILayout.Label("MAPPE", title);
        for (int i = 0; i < layers.Count; i++)
        {
            int n = pos.Count(p => (int)p["layer"] == i);
            var l = layers[i];
            // Besides the surface and the map on screen, only the maps where some of ours are.
            if (n == 0 && i != 0 && i != view.Layer) continue;
            string kind = (bool?)l["underground"] == true ? "⤓ " : (bool?)l["indoor"] == true ? "⌂ " : "";
            string text = kind + (string)l["name"] + (n > 0 ? $"  ·  {n}" : "");
            var st = new GUIStyle(rowButton) { fontStyle = i == view.Layer ? FontStyle.Bold : FontStyle.Normal };
            if (GUILayout.Button(text, st, GUILayout.Width(inner))) view.ShowOverview(i);
        }
        if (GUILayout.Button(layerMenu ? "Tutte le mappe ▴" : "Tutte le mappe ▾", button, GUILayout.Width(inner))) layerMenu = !layerMenu;
        if (layerMenu)
            for (int i = 0; i < layers.Count; i++)
                if (GUILayout.Button("   " + (string)layers[i]["name"], rowButton, GUILayout.Width(inner))) view.ShowOverview(i);

        GUILayout.Label("AREE CON I TUOI", title);
        var zones = view.Map["zones"].Where(z => (int)z["layer"] == view.Layer && (int)z["w"] < (int)layers[view.Layer]["width"])
            .Select(z => (z, n: pos.Count(p => InZone(z, p)))).Where(t => t.n > 0).OrderByDescending(t => t.n).ToList();
        if (zones.Count == 0) GUILayout.Label("Nessuno dei tuoi in quest'area.", small);
        foreach (var (z, n) in zones)
            GUILayout.Label($"{z["name"]}: {n}", label, GUILayout.Width(inner));
        GUILayout.Space(6);
        if (view.ChampionId.HasValue && GUILayout.Button("★ Torna dal campione", button, GUILayout.Width(inner), GUILayout.Height(rowH))) Close();
        GUILayout.EndScrollView();
        GUILayout.EndArea();
    }

    /// Area names over the overview, centred on each area, with how many of ours are inside. Areas as wide
    /// as the whole map and small unnamed corners without any of ours are left out.
    void ZoneLabels()
    {
        var pos = MemberPositions();
        int layerW = (int?)view.Layers?[view.Layer]?["width"] ?? 0;
        foreach (var z in view.Map["zones"].Where(z => (int)z["layer"] == view.Layer && (int)z["w"] < layerW))
        {
            int n = pos.Count(m => InZone(z, m));
            if (n == 0 && (int)z["w"] * (int)z["h"] < 150) continue;
            var sp = view.WorldToScreen(new Vector3((int)z["x"] + (int)z["w"] / 2f, -(int)z["y"] - (int)z["h"] / 2f, 0));
            var p = new Vector2(sp.x, Screen.height - sp.y) / scale;
            var owner = (string)view.Territory((string)z["id"])?["owner"];
            string text = (string)z["name"] + (owner != null ? "\n" + FactionName(owner) : "");
            var st = new GUIStyle(n > 0 ? zoneMine : zoneLabel) { alignment = TextAnchor.MiddleCenter };
            var r = new Rect(p.x - 110, p.y - 22, 220, 34);
            GUI.Label(new Rect(r.x + 1, r.y + 1, r.width, r.height), text, new GUIStyle(st) { normal = { textColor = new Color(0, 0, 0, 0.75f) } });
            GUI.Label(r, text, st);
            if (n > 0) GUI.Label(new Rect(p.x - 18, r.yMax, 36, 20), n.ToString(), badge);
        }
    }

    // ── Inventory and team ───────────────────────────────────────────────────

    void InventoryWin(float width)
    {
        var inv = view.PlayerInfo?["inventory"] as JArray;
        if (inv == null || inv.Count == 0)
        {
            GUILayout.Label("Il team non ha ancora raccolto nulla.", small);
            return;
        }
        GUILayout.Label("Quello che i membri portano addosso e quello che sta negli edifici della fazione. Tocca un oggetto per vedere cosa fa, assegnarlo o usarlo.", small, GUILayout.Width(width));
        GUILayout.Space(6);
        Row(width, new[] { 0.52f, 0.16f, 0.16f, 0.16f }, cellHead, "Oggetto", "Addosso", "Depositi", "Totale");
        foreach (var group in inv.GroupBy(i => (string)i["category"] ?? "varie").OrderBy(g => g.Key))
        {
            Section(group.Key.Replace('_', ' '));
            foreach (var i in group.OrderByDescending(i => (int)i["total"]))
            {
                var cols = new[] { 0.52f, 0.16f, 0.16f, 0.16f };
                GUILayout.BeginHorizontal();
                var r = GUILayoutUtility.GetRect(28, 28, GUILayout.Width(28), GUILayout.Height(28));
                var icon = ItemIcon((string)i["id"], (string)i["category"]);
                if (icon != null) GUI.DrawTexture(new Rect(r.x, r.y - 2, 28, 28), icon);
                if (GUILayout.Button((string)i["name"], rowButton, GUILayout.Width(width * cols[0] - 36))) OpenItem((string)i["id"], Win.Inventory);
                GUILayout.Label(Num(i["carried"]), cell, GUILayout.Width(width * cols[1] - 4));
                GUILayout.Label(Num(i["stored"]), cell, GUILayout.Width(width * cols[2] - 4));
                GUILayout.Label(Num(i["total"]), cell, GUILayout.Width(width * cols[3] - 4));
                GUILayout.EndHorizontal();
            }
        }
    }

    static string Num(JToken t) => (int?)t is int n && n > 0 ? n.ToString() : "–";

    void Row(float width, float[] cols, GUIStyle st, params string[] texts)
    {
        GUILayout.BeginHorizontal();
        for (int i = 0; i < texts.Length; i++) GUILayout.Label(texts[i], st, GUILayout.Width(width * cols[i] - 4));
        GUILayout.EndHorizontal();
    }

    void TeamWin(float width)
    {
        var p = view.PlayerInfo;
        if (p == null)
        {
            GUILayout.Label("Nessun giocatore in questa partita.", small);
            return;
        }
        var s = p["summary"];
        string mine = (string)p["faction"];
        var terr = view.State?["snapshot"]?["territories"] as JObject;
        var held = terr?.Properties().Where(t => (string)t.Value["owner"] == mine).Select(t => ZoneName(t.Name)).ToList() ?? new List<string>();
        // The team as a whole first.
        Section(FactionName(mine));
        float half = width / 2 - 4;
        GUILayout.BeginHorizontal();
        GUILayout.Label($"Fondo di gilda: {Fmt((double?)p["treasury"] ?? 0)} €", label, GUILayout.Width(half));
        GUILayout.Label($"Punti vittoria: {Fmt(Score())}", label, GUILayout.Width(half));
        GUILayout.EndHorizontal();
        GUILayout.BeginHorizontal();
        GUILayout.Label($"Soldi in tasca: {Fmt((double?)s?["pocket_money"] ?? 0)} €", label, GUILayout.Width(half));
        GUILayout.Label($"Membri: {s?["members"]}", label, GUILayout.Width(half));
        GUILayout.EndHorizontal();
        GUILayout.BeginHorizontal();
        GUILayout.Label($"Umore medio: {s?["avg_mood"]} ({(float?)s?["avg_morale"] ?? 0:0})", label, GUILayout.Width(half));
        GUILayout.Label($"Salute media: {((float?)s?["avg_health"] ?? 0) * 100:0}%", label, GUILayout.Width(half));
        GUILayout.EndHorizontal();
        GUILayout.Label("Quartieri: " + (held.Count == 0 ? "nessuno (stai nei quartieri con i tuoi, possiedi edifici e pianta stendardi)" : string.Join(", ", held)), label, GUILayout.Width(width));

        Section("Membri");
        var cols = new[] { 0.22f, 0.13f, 0.15f, 0.08f, 0.13f, 0.09f, 0.2f };
        Row(width, cols, cellHead, "Nome", "Razza", "Classe", "Vita", "Umore", "Soldi", "Attività");
        foreach (var m in p["members"])
        {
            long id = (long)m["id"];
            bool champ = (bool?)m["champion"] ?? false;
            float hp = (float?)m["health"] ?? 1f;
            GUILayout.BeginHorizontal();
            var st = view.Selected == id ? new GUIStyle(rowButton) { fontStyle = FontStyle.Bold } : rowButton;
            if (GUILayout.Button((champ ? "★ " : "") + (string)m["name"], st, GUILayout.Width(width * cols[0] - 4)))
            {
                view.Select(id, byChampion: true);
                OpenSub(Win.Card, Win.Team);
            }
            GUILayout.Label((string)m["race_name"] ?? (string)m["race"], cell, GUILayout.Width(width * cols[1] - 4));
            GUILayout.Label(string.Join(", ", (m["class_names"] ?? m["classes"]).Select(c => (string)c)), cell, GUILayout.Width(width * cols[2] - 4));
            GUILayout.Label($"{hp * 100:0}%", hp < 0.5f ? new GUIStyle(cell) { normal = { textColor = fake.normal.textColor } } : cell, GUILayout.Width(width * cols[3] - 4));
            GUILayout.Label((string)m["mood"] ?? "", cell, GUILayout.Width(width * cols[4] - 4));
            GUILayout.Label($"{Fmt((double?)m["money"] ?? 0)} €", cell, GUILayout.Width(width * cols[5] - 4));
            GUILayout.Label((string)m["activity"] ?? "", cell, GUILayout.Width(width * cols[6] - 4));
            GUILayout.EndHorizontal();
        }

        // Quarters: how the fight goes where we have influence.
        if (terr != null && terr.Properties().Any(t => ((float?)t.Value["influence"]?[mine] ?? 0f) >= 0.5f))
        {
            Section("Influenza sui quartieri");
            foreach (var t in terr.Properties())
            {
                var inf = t.Value["influence"] as JObject;
                float my = (float?)inf?[mine] ?? 0f;
                if (my < 0.5f) continue;
                var top = inf.Properties().OrderByDescending(x => (float)x.Value).First();
                string owner = (string)t.Value["owner"];
                string state = owner == mine ? "tuo" : owner == null ? "libero" : "di " + FactionName(owner);
                GUILayout.Label($"{ZoneName(t.Name)} ({state}): tua influenza {my:0} · prima {FactionName(top.Name)} {(float)top.Value:0}", small, GUILayout.Width(width));
            }
        }

        Section("Squadre");
        long? champion = view.ChampionId;
        foreach (var q in p["squads"])
        {
            long sid = (long)q["id"];
            var order = q["order"];
            string orderText = order == null || order.Type == JTokenType.Null ? "libera" : order.ToString(Newtonsoft.Json.Formatting.None);
            GUILayout.Label($"{q["name"]} ({q["members"].Count()} membri) — {orderText}", label, GUILayout.Width(width));
            GUILayout.BeginHorizontal();
            if (champion.HasValue && GUILayout.Button("Segui il campione", button))
                SquadOrder(sid, new JObject { ["Follow"] = new JObject { ["target"] = champion.Value, ["distance"] = 3 } });
            if (GUILayout.Button("Tieni posizione", button)) SquadOrder(sid, JValue.CreateString("Hold"));
            if (GUILayout.Button("Libera", button)) SquadOrder(sid, JValue.CreateNull());
            GUILayout.EndHorizontal();
        }
        if (champion.HasValue && GUILayout.Button("Crea una scorta con i 4 membri più vicini al campione", button, GUILayout.Width(width)))
        {
            var champPos = p["members"].FirstOrDefault(m => (bool?)m["champion"] == true)?["pos"];
            var near = p["members"].Where(m => (bool?)m["champion"] != true && m["pos"].Type != JTokenType.Null)
                .OrderBy(m => champPos == null ? 0 : Mathf.Abs((int)m["pos"]["x"] - (int)champPos["x"]) + Mathf.Abs((int)m["pos"]["y"] - (int)champPos["y"]))
                .Take(4).Select(m => (long)m["id"]).ToList();
            view.SendPlayerCommand(new JObject { ["type"] = "player_create_squad", ["name"] = "Scorta", ["members"] = new JArray(near) });
        }

        Section("Stipendi per rango");
        foreach (var r in p["ranks"])
        {
            GUILayout.BeginHorizontal();
            float sal = (float)r["salary"];
            GUILayout.Label($"{r["name"]}: {sal:0} €", label, GUILayout.Width(width * 0.6f));
            if (GUILayout.Button("−", button, GUILayout.Width(touch ? 46 : 30))) SetSalary((string)r["id"], Mathf.Max(0, sal - 5));
            if (GUILayout.Button("+", button, GUILayout.Width(touch ? 46 : 30))) SetSalary((string)r["id"], sal + 5);
            GUILayout.EndHorizontal();
        }

        if (view.Selected.HasValue && view.Member(view.Selected.Value) is JToken sel && sel["work"] is JObject work)
        {
            Section("Priorità di lavoro di " + sel["name"]);
            GUILayout.Label("1 = massima, 4 = minima, 0 = mai. Clic per cambiare.", small);
            foreach (var wt in p["work_types"].Select(x => (string)x))
            {
                int prio = (int?)work[wt] ?? 0;
                GUILayout.BeginHorizontal();
                GUILayout.Label(wt, label, GUILayout.Width(width * 0.6f));
                if (GUILayout.Button(prio == 0 ? "mai" : prio.ToString(), button, GUILayout.Width(50)))
                    view.SendCommand(new JObject { ["type"] = "set_work_priority", ["entity"] = view.Selected.Value, ["work_type"] = wt, ["priority"] = (prio + 1) % 5 });
                GUILayout.EndHorizontal();
            }
        }
        else GUILayout.Label("Scegli un membro dalla lista per vederne le priorità di lavoro qui.", small, GUILayout.Width(width));
    }

    void SquadOrder(long squad, JToken order) =>
        view.SendPlayerCommand(new JObject { ["type"] = "player_squad_order", ["squad"] = squad, ["order"] = order });

    void SetSalary(string rank, float amount) =>
        view.SendCommand(new JObject { ["type"] = "set_salary", ["faction"] = view.PlayerInfo["faction"], ["rank"] = rank, ["amount"] = amount });

    // ── Options ──────────────────────────────────────────────────────────────

    /// The game controls that used to fill the side menu.
    void OptionsWin(float width)
    {
        float third = width / 3f - 6, half = width / 2f - 6;
        var h = GUILayout.Height(rowH);
        bool paused = (bool?)view.Control?["paused"] ?? false;
        Section("Tempo");
        GUILayout.BeginHorizontal();
        if (GUILayout.Button(paused ? "Riprendi" : "Pausa", button, GUILayout.Width(third), h))
            view.SendControl(new JObject { ["paused"] = !paused });
        if (GUILayout.Button("Avanza 1 ora", button, GUILayout.Width(third), h))
            view.SendControl(new JObject { ["step"] = 1 });
        if (GUILayout.Button("Velocità: " + Speeds[speedIndex].name, button, GUILayout.Width(third), h))
        {
            speedIndex = (speedIndex + 1) % Speeds.Length;
            view.SendControl(new JObject { ["tick_ms"] = Speeds[speedIndex].ms });
        }
        GUILayout.EndHorizontal();

        Section("Vista");
        GUILayout.BeginHorizontal();
        GUI.enabled = !exitAsk && view.ChampionId.HasValue;
        if (GUILayout.Button("★ Campione" + Key("C"), button, GUILayout.Width(third), h))
        {
            GoToChampion();
            Close();
        }
        GUI.enabled = !exitAsk;
        view.FollowCamera = GUILayout.Toggle(view.FollowCamera, "Segui" + Key("F"), button, GUILayout.Width(third), h);
        if (GUILayout.Button(view.Zoom >= 1f ? "Panoramica" : "Da vicino", button, GUILayout.Width(third), h))
        {
            if (view.Zoom >= 1f) view.ShowOverview(view.Layer);
            else view.CloseView();
        }
        GUILayout.EndHorizontal();
        var factions = Factions();
        string fogName = string.IsNullOrEmpty(view.FogFaction) ? "nessuna" : FactionName(view.FogFaction);
        if (GUILayout.Button("Nebbia di guerra: " + fogName, button, GUILayout.Width(width), h) && factions.Count > 0)
        {
            var ids = new List<string> { "" };
            ids.AddRange(factions.Select(f => (string)f["id"]));
            int i = ids.IndexOf(view.FogFaction);
            view.ChooseFog(ids[(i + 1) % ids.Count]);
        }

        Section("Azioni");
        if (GUILayout.Button(view.DigMode ? "▸ Scava (trascina un rettangolo)" : "Scava: designa un rettangolo da scavare", button, GUILayout.Width(width), h))
        {
            view.DigMode = !view.DigMode;
            if (view.DigMode) Close();
        }

        Section("Partita");
        GUILayout.BeginHorizontal();
        if (GUILayout.Button("Salva", button, GUILayout.Width(half), h)) view.Save();
        if (GUILayout.Button("Carica", button, GUILayout.Width(half), h)) view.Load();
        GUILayout.EndHorizontal();
        GUILayout.BeginHorizontal();
        if (GUILayout.Button("Server", button, GUILayout.Width(half), h))
        {
            serverField = view.Api.BaseUrl;
            serverOpen = !serverOpen;
        }
        if (GUILayout.Button("Esci" + Key("Ctrl+Q"), button, GUILayout.Width(half), h)) exitAsk = true;
        GUILayout.EndHorizontal();
        if (!string.IsNullOrEmpty(view.LastCommandResult)) GUILayout.Label(view.LastCommandResult, small, GUILayout.Width(width));
        if (view.Error != null) GUILayout.Label(view.Error, fake, GUILayout.Width(width));
    }

    // ── Pigeon ───────────────────────────────────────────────────────────────

    string SeenKey => "pigeon_seen_" + view.PlayerId;

    /// Newest article id of the main channel, -1 when it is still unknown.
    long NewestMain() => (view.MainFeed?["articles"] as JArray)?.FirstOrDefault() is JToken a ? (long)a["id"] : (view.MainFeed != null ? 0 : -1);

    int Unread()
    {
        long newest = NewestMain();
        if (newest < 0 || view.PlayerId == null) return 0;
        if (seenMain < 0)
        {
            // First look: what is already there is not news. A new game (ids from scratch) starts over.
            seenMain = PlayerPrefs.HasKey(SeenKey) ? long.Parse(PlayerPrefs.GetString(SeenKey)) : newest;
        }
        if (newest < seenMain) seenMain = newest;
        return view.MainFeed["articles"].Count(a => (long)a["id"] > seenMain);
    }

    void MarkRead()
    {
        long newest = NewestMain();
        if (newest < 0 || view.PlayerId == null) return;
        seenMain = newest;
        PlayerPrefs.SetString(SeenKey, newest.ToString());
    }

    void PigeonWin(float width)
    {
        // Channels: the main one (few messages, what matters), then the ones still to come into their own.
        var channels = new List<(string id, string name)> { ("main", "Principale"), ("mine", "Su di te") };
        if (view.MainFeed?["categories"] is JArray cats)
            channels.AddRange(cats.Select(c => ((string)c["id"], (string)c["name"])));
        channels.Add(("all", "Tutto"));
        int perRow = touch ? 3 : 4;
        int unread = Unread();
        for (int i = 0; i < channels.Count; i += perRow)
        {
            GUILayout.BeginHorizontal();
            foreach (var (id, name) in channels.Skip(i).Take(perRow))
            {
                bool on = view.FeedFilter == id;
                string text = (on ? "▸ " : "") + name + (id == "main" && unread > 0 ? $" ({unread})" : "");
                if (GUILayout.Button(text, new GUIStyle(button) { fontStyle = on ? FontStyle.Bold : FontStyle.Normal }, GUILayout.Width(width / perRow - 4)))
                {
                    view.FeedFilter = id;
                    scrollWin = Vector2.zero;
                }
            }
            GUILayout.EndHorizontal();
        }
        bool main = view.FeedFilter == "main";
        GUILayout.Label(main ? "Le notizie che contano: quelle sul tuo team e i grandi fatti del mondo." : "Canale secondario: qui finisce tutto il resto.", small, GUILayout.Width(width));
        var feed = main ? view.MainFeed : view.Feed;
        var articles = (string)feed?["filter"] == view.FeedFilter ? feed["articles"] as JArray : null;
        if (articles == null)
        {
            GUILayout.Label("Il piccione sta arrivando…", small);
            return;
        }
        if (articles.Count == 0) GUILayout.Label("Niente da segnalare.", small);
        foreach (var a in articles)
        {
            bool fresh = main && (long)a["id"] > seenMain;
            string tag = ((bool?)a["propaganda"] ?? false) ? "PROPAGANDA · " : "";
            bool mine = (bool?)a["mine"] ?? false;
            var st = new GUIStyle(mine ? good : label) { fontStyle = fresh ? FontStyle.Bold : FontStyle.Normal };
            GUILayout.Label((fresh ? "● " : "") + tag + (string)a["headline"], st, GUILayout.Width(width));
            long t = (long)a["tick"];
            GUILayout.Label($"Giorno {t / 24 + 1}, {t % 24}:00 — {a["author_name"]} · {a["category"]}", small, GUILayout.Width(width));
            GUILayout.Space(4);
        }
        if (main) MarkRead();
    }

    /// Pokémon-like text box at the bottom with each new message of the main channel; a tap opens the pigeon.
    void Notice(float w, float h)
    {
        var a = (view.MainFeed?["articles"] as JArray)?.FirstOrDefault();
        if (a != null && (long)a["id"] != noticeId)
        {
            // The first message after start is not news: no box until something new arrives.
            if (noticeId >= 0) noticeUntil = Time.time + 8f;
            noticeId = (long)a["id"];
            noticeText = $"{a["author_name"]}: {a["headline"]}";
        }
        if (noticeText == null || Time.time > noticeUntil) return;
        float nw = Mathf.Min(760, w - 2 * colW - 48 - (PadShown ? StickRadius * 2 + 140 : 0));
        var r = Ui(new Rect(w / 2 - nw / 2, h - 96, nw, 80));
        GUI.Label(r, noticeText, textBox);
        if (Mathf.Repeat(Time.time, 1f) < 0.6f) GUI.Label(new Rect(r.xMax - 30, r.yMax - 28, 20, 20), "▼", label);
        if (Event.current.type == EventType.MouseDown && r.Contains(Event.current.mousePosition))
        {
            noticeUntil = 0;
            view.FeedFilter = "main";
            Open(Win.Pigeon);
            Event.current.Use();
        }
    }

    // ── Ranking, world resources, manual ─────────────────────────────────────

    void RankingWin(float width)
    {
        var snap = view.State["snapshot"];
        var scores = snap["scores"] as JObject;
        var terr = snap["territories"] as JObject;
        string mine = (string)view.PlayerInfo?["faction"];
        if (snap["winner"] is JToken winner && winner.Type != JTokenType.Null)
            GUILayout.Label("PARTITA VINTA DA: " + (winner.Type == JTokenType.Object ? (string)winner["faction"] ?? winner.ToString() : FactionName((string)winner)), good, GUILayout.Width(width));
        GUILayout.Label("Si vince riunendo il Corpo di San Donnino, mettendo il proprio capo sul Trono degli Ubriaconi o arrivando a 3000 punti vittoria.", small, GUILayout.Width(width));
        var cols = new[] { 0.07f, 0.37f, 0.12f, 0.12f, 0.17f, 0.15f };
        Row(width, cols, cellHead, "#", "Fazione", "Punti", "Membri", "Fondo", "Quartieri");
        int pos = 0;
        foreach (var f in snap["factions"].Where(f => f["absorbed_into"].Type == JTokenType.Null)
                     .OrderByDescending(f => (long?)scores?[(string)f["id"]] ?? 0))
        {
            pos++;
            string id = (string)f["id"];
            int quarters = terr?.Properties().Count(t => (string)t.Value["owner"] == id) ?? 0;
            string star = f["controlled_by"].Type != JTokenType.Null ? " ★" : "";
            var st = id == mine ? new GUIStyle(cell) { fontStyle = FontStyle.Bold, normal = { textColor = good.normal.textColor } } : cell;
            Row(width, cols, st, pos + ".", (string)f["name"] + star, $"{scores?[id] ?? 0}", $"{f["members"]}", $"{Fmt((double)f["treasury"])} €", quarters.ToString());
        }
        GUILayout.Label("★ = fazione guidata da un giocatore. In grassetto la tua.", small, GUILayout.Width(width));
    }

    void EconomyWin(float width)
    {
        var e = view.Economy;
        if (e == null)
        {
            GUILayout.Label("Raccolta dei dati…", small);
            return;
        }
        float half = width / 2 - 4;
        double index = (double?)e["price_index"] ?? 1, infl = (double?)e["inflation_day"] ?? 0;
        Section("Prezzi");
        GUILayout.BeginHorizontal();
        GUILayout.Label($"Indice dei prezzi: {index * 100:0.0} (100 = prezzi base)", label, GUILayout.Width(half));
        GUILayout.Label($"Inflazione ultime 24 ore: {infl * 100:+0.0;-0.0;0.0}%", infl > 0.005 ? fake : infl < -0.005 ? good : label, GUILayout.Width(half));
        GUILayout.EndHorizontal();
        float disruption = (float?)e["disruption"] ?? 0;
        if (disruption > 0.001f) GUILayout.Label($"Logistica in crisi: +{disruption * 100:0}% sui prezzi", fake, GUILayout.Width(width));
        Section("Moneta in circolazione");
        var m = e["money"];
        GUILayout.Label($"In tasca alla gente: {Fmt((double)m["wallets"])} €  ·  Nei fondi delle fazioni: {Fmt((double)m["treasuries"])} €  ·  Totale: {Fmt((double)m["total"])} €", label, GUILayout.Width(width));
        Section("Circostanze in corso");
        var circ = e["circumstances"] as JArray;
        if (circ == null || circ.Count == 0) GUILayout.Label("Nessuna: rifornimenti regolari.", small);
        else
            foreach (var c in circ)
                GUILayout.Label($"{c["name"]} (ancora {c["ticks_left"]} ore)", label, GUILayout.Width(width));
        if (e["local_production"] is JArray local && local.Count > 0)
        {
            Section("Quota prodotta in zona");
            GUILayout.Label("Il resto arriva da fuori con i rifornimenti.", small);
            foreach (var l in local)
                GUILayout.Label($"{GoodName(e, (string)l["item"])}: {(float)l["local_share"] * 100:0}%", label, GUILayout.Width(width));
        }
        Section("Merci (le più mosse prima)");
        var cols = new[] { 0.4f, 0.18f, 0.2f, 0.2f };
        Row(width, cols, cellHead, "Merce", "Prezzo", "Rispetto al base", "Ultime 24 ore");
        foreach (var g in e["goods"].OrderByDescending(g => Mathf.Abs((float)g["price"] / Mathf.Max(0.01f, (float)g["base"]) - 1f)))
        {
            float price = (float)g["price"], b = (float)g["base"];
            float vsBase = (price / Mathf.Max(0.01f, b) - 1f) * 100f;
            float? day = g["day_change"]?.Type == JTokenType.Float || g["day_change"]?.Type == JTokenType.Integer ? (float)g["day_change"] * 100f : null;
            GUILayout.BeginHorizontal();
            GUILayout.Label((string)g["name"], cell, GUILayout.Width(width * cols[0] - 4));
            GUILayout.Label($"{price:0.00} €", cell, GUILayout.Width(width * cols[1] - 4));
            GUILayout.Label($"{vsBase:+0;-0;0}%", Trend(vsBase), GUILayout.Width(width * cols[2] - 4));
            GUILayout.Label(day.HasValue ? $"{day.Value:+0.0;-0.0;0.0}%" : "–", day.HasValue ? Trend(day.Value) : cell, GUILayout.Width(width * cols[3] - 4));
            GUILayout.EndHorizontal();
        }
    }

    GUIStyle Trend(float pct) => pct > 0.5f ? new GUIStyle(cell) { normal = { textColor = fake.normal.textColor } }
        : pct < -0.5f ? new GUIStyle(cell) { normal = { textColor = good.normal.textColor } } : cell;

    static string GoodName(JToken economy, string id) => (string)economy["goods"]?.FirstOrDefault(g => (string)g["id"] == id)?["name"] ?? id;

    static readonly (string title, string text)[] Manual =
    {
        ("Lo scopo", "Guidi una fazione di Fidenza e Salsomaggiore. Si vince in tre modi: riunire le cinque reliquie del Corpo di San Donnino, mettere il proprio capo sul Trono degli Ubriaconi (quando Re Anolino muore) oppure accumulare 3000 punti vittoria. I punti arrivano dalle collezioni di oggetti, dai titoli e dai quartieri controllati."),
        ("Il campione", "Il tuo campione è l'unico personaggio che muovi direttamente, passo per passo: frecce o WASD sul PC, il joystick in basso a sinistra sul telefono (anche in diagonale). Le porte e le scale si attraversano camminandoci sopra. Quando gli sta accanto qualcuno compare il suggerimento: Spazio (o E, o il tasto A sul telefono) apre le azioni possibili, a cominciare da Parla: chiunque ti risponde, con le sue frasi. Finché lo muovi a mano il campione aspetta i tuoi comandi; lasciato a sé per un giorno di gioco torna a badare a fame, sonno e svago. Non muore: va al tappeto, perde parte dei soldi e si rialza. Mentre una finestra è aperta il campione sta fermo."),
        ("Le schede", "Tocca una pedina o un edificio per aprirne la scheda: vita e bisogni, soldi, i tre slot degli oggetti (ognuno tiene una categoria) e il pulsante AZIONI. Tocca un oggetto per vedere cosa fa."),
        ("Gli oggetti", "Nell'Inventario c'è tutto quello che il team possiede, addosso ai membri e negli edifici della fazione. Toccando un oggetto vedi cosa fa (usandolo o portandolo addosso), quanto vale e chi ce l'ha; scegli un membro con le frecce e puoi assegnarglielo o farglielo usare (se non ce l'ha, il team gliene passa uno)."),
        ("Il team", "Gli altri membri della fazione vivono da soli: lavorano, mangiano, si divertono. Puoi dare ordini (seleziona un tuo membro e scegli un ordine; clic destro o pressione lunga lo manda in un punto), ma obbediscono solo in parte: conta l'umore, il dissenso e il rango. Dal pannello Team vedi tutti i membri, crei squadre che seguono il campione, cambi gli stipendi e le priorità di lavoro."),
        ("Soldi e punti", "In alto vedi i punti vittoria, il fondo di gilda (il tesoro della fazione) e quanti siete. Il fondo paga stipendi e tangenti; i membri hanno anche soldi in tasca. Ogni quartiere controllato rende soldi e un punto vittoria al giorno."),
        ("I quartieri", "Un quartiere va a chi ci ha più influenza: membri presenti, edifici posseduti e stendardi piantati (azione Pianta lo stendardo). Per strapparlo a chi lo tiene bisogna superarlo con margine. Nella Mappa vedi quanti dei tuoi ci sono in ogni area."),
        ("Il Piccione Viaggiatore", "È il giornale della città. Il canale Principale porta poche notizie: quelle che riguardano il tuo team e i grandi fatti del mondo; il numero rosso sul pulsante conta quelle non lette. Gli altri canali raccolgono il resto, per argomento. Attenzione: le bufale sembrano vere."),
        ("Risorse mondiali", "I prezzi salgono quando le merci scarseggiano o la domanda cresce, e scendono quando abbondano. Scioperi, nebbia, piene e feste cambiano i rifornimenti da fuori. Il pannello Risorse mostra l'indice dei prezzi, l'inflazione dell'ultimo giorno, la moneta in giro e le circostanze in corso."),
        ("Crimine", "Furti, aggressioni e sabotaggi hanno testimoni: chi è ricercato può essere perquisito e arrestato. Un tuo membro arrestato si libera pagando la tangente dalla sua scheda."),
        ("Comandi", "PC: frecce/WASD muovono il campione (in panoramica spostano la vista), Spazio/E apre le azioni con chi gli sta accanto (e chiude un dialogo), trascina col tasto sinistro per spostare la vista, rotella per lo zoom, C torna al campione, F segue la pedina selezionata. M mappa, I inventario, T team, O opzioni, P piccione, L classifica, R risorse, H manuale, Tab nasconde o mostra l'interfaccia, Esc chiude o esce, Ctrl+Q esci.\nTelefono: joystick e tasto A per il campione, un dito trascina la vista, tocco seleziona, pressione lunga manda la pedina selezionata, due dita zoom, indietro chiude."),
    };

    void ManualWin(float width)
    {
        foreach (var (t, text) in Manual)
        {
            Section(t);
            GUILayout.Label(text, label, GUILayout.Width(width));
        }
    }

    // ── Card of a pawn or building ───────────────────────────────────────────

    /// Opens the card of a pawn or building (a big window like the others).
    void OpenCard(long id)
    {
        if (view.Selected != id) view.Select(id, byChampion: true);
        OpenSub(Win.Card, Win.None);
    }

    /// A window reached from another one: Esc goes back to <paramref name="from"/>.
    void OpenSub(Win which, Win from)
    {
        if (win == Win.Map) view.CloseView();
        if (from != Win.None && from != Win.Map) stack.Add(from);
        win = which;
        hidden = false;
        scrollWin = Vector2.zero;
        view.PigeonOpen = view.EconomyOpen = false;
    }

    void CardWin(float width)
    {
        var e = view.SelectedEntity;
        if (e == null)
        {
            GUILayout.Label("Caricamento…", big);
            return;
        }
        bool mine = view.IsMine((long)e["id"]);
        var truth = e["truth"];
        if (truth != null && truth.Type != JTokenType.Null)
            GUILayout.Label($"Vera identità: {truth["name"]} ({truth["race"]}, {truth["faction"]}) · copertura {(float)truth["cover"]:0}", new GUIStyle(big) { normal = { textColor = fake.normal.textColor } }, GUILayout.Width(width));
        if (e["building"] != null && e["building"].Type != JTokenType.Null)
        {
            var b = e["building"];
            GUILayout.Label($"Edificio · {FactionName((string)b["owner"]?["Faction"] ?? "")}", big, GUILayout.Width(width));
            Bar(width, "Integrità", (float)b["hp"] / Mathf.Max(1f, (float)b["max_hp"]));
            var stock = ((JObject)b["stock"]).Properties().Where(p => (int)p.Value > 0).ToList();
            Section("Magazzino");
            if (stock.Count == 0) GUILayout.Label("vuoto", big);
            ItemGrid(width, stock.Select(p => ((string)p.Name, (string)p.Name, (int)p.Value, (string)null)).ToList(), 0);
            GUILayout.Space(10);
            ActionsButton(width, (long)e["id"]);
            return;
        }
        var classes = string.Join(", ", e["classes"].Select(c => (string)c));
        GUILayout.Label($"{e["race"]} · {classes} · {FactionName((string)e["faction"])} · {e["rank"]}{(mine ? "  (tuo)" : "")}", big, GUILayout.Width(width));
        var act = e["activity"];
        GUILayout.Label($"Sta facendo: {act?["label"]}  ·  umore: {act?["mood"]}", big, GUILayout.Width(width));
        string missing = e["missing_parts"] is JArray mp && mp.Count > 0 ? " (manca: " + string.Join(", ", mp.Select(x => (string)x)) + ")" : "";
        Bar(width, "Vita" + missing, (float)e["health"]);
        if (e["needs"] is JObject needs)
            foreach (var n in needs.Properties()) Bar(width, Cap(n.Name), (float)n.Value);
        GUILayout.Label($"Soldi {Fmt((double)e["money"])} €  ·  Ricercato {(float)e["wanted"]:0.0}  ·  Dissenso {(float)e["dissent"]:0}", big, GUILayout.Width(width));
        var statuses = e["statuses"]?.Select(s => (string)s["name"] + (s["stage"]?.Type == JTokenType.String ? $" ({s["stage"]})" : "")).ToList();
        if (statuses?.Count > 0) GUILayout.Label("Status: " + string.Join(", ", statuses), big, GUILayout.Width(width));

        // The three slots of the bag: one category each, with its items.
        Section("Oggetti");
        int max = Mathf.Max(3, (int?)e["max_slots"] ?? 3);
        var slots = (e["slots"] as JArray)?.ToList() ?? new List<JToken>();
        var cells = new List<(string id, string name, int qty, string category)>();
        for (int i = 0; i < max; i++)
        {
            var it = i < slots.Count ? slots[i]["items"]?.FirstOrDefault() : null;
            int more = i < slots.Count ? slots[i]["items"].Count() - 1 : 0;
            cells.Add(it == null ? (null, null, 0, null)
                : ((string)it[0], (string)it[1] + (more > 0 ? $" +{more}" : ""), (int)it[2], (string)slots[i]["category"]));
        }
        ItemGrid(width, cells, max);

        bool detained = act?["flags"]?.Any(f => (string)f == "detained") ?? false;
        if (detained && view.PlayerInfo != null)
        {
            string payer = (string)view.PlayerInfo["faction"];
            if (GUILayout.Button($"Paga la tangente ({FactionName(payer)})", bigButton, GUILayout.Width(width), GUILayout.Height(rowH + 8)))
                view.SendCommand(new JObject { ["type"] = "bribe", ["faction"] = payer, ["target"] = e["id"] });
        }
        GUILayout.Space(10);
        ActionsButton(width, (long)e["id"]);
    }

    static string Cap(string s) => string.IsNullOrEmpty(s) ? s : char.ToUpperInvariant(s[0]) + s.Substring(1);

    void Bar(float width, string name, float value)
    {
        GUILayout.BeginHorizontal();
        float bw = Mathf.Min(width * 0.55f, 420);
        GUILayout.Label(name, big, GUILayout.Width(Mathf.Min(width * 0.3f, 200)));
        var r = GUILayoutUtility.GetRect(bw, touch ? 16 : 14, GUILayout.Width(bw));
        r.y += touch ? 6 : 5;
        GUI.DrawTexture(r, barTex);
        float v = Mathf.Clamp01(value);
        GUI.DrawTexture(new Rect(r.x, r.y, r.width * v, r.height), v < 0.3f ? barLowTex : barFillTex);
        GUILayout.Label($"{v * 100:0}%", big, GUILayout.Width(60));
        GUILayout.FlexibleSpace();
        GUILayout.EndHorizontal();
    }

    /// Square boxes with an item's icon, name and quantity (empty boxes when <paramref name="min"/> asks for them);
    /// a filled box opens the item.
    void ItemGrid(float width, List<(string id, string name, int qty, string category)> cells, int min)
    {
        float box = touch ? 120 : 110, gap = 10;
        int perRow = Mathf.Max(1, (int)((width + gap) / (box + gap)));
        for (int i = 0; i < cells.Count; i += perRow)
        {
            GUILayout.BeginHorizontal();
            foreach (var c in cells.Skip(i).Take(perRow))
            {
                var r = GUILayoutUtility.GetRect(box, box + 22, GUILayout.Width(box), GUILayout.Height(box + 22));
                var slot = new Rect(r.x, r.y, box, box);
                if (c.id == null)
                {
                    GUI.Box(slot, GUIContent.none, slotStyle);
                    GUI.Label(new Rect(slot.x, slot.y, box, box), "vuoto", new GUIStyle(small) { alignment = TextAnchor.MiddleCenter });
                    GUILayout.Space(gap);
                    continue;
                }
                if (GUI.Button(slot, GUIContent.none, slotStyle)) OpenItem(c.id, win);
                var icon = ItemIcon(c.id, c.category);
                if (icon != null) GUI.DrawTexture(new Rect(slot.x + box / 2 - 32, slot.y + 12, 64, 64), icon);
                if (c.qty > 1) GUI.Label(new Rect(slot.xMax - 40, slot.y + 6, 34, 20), "×" + c.qty, new GUIStyle(small) { alignment = TextAnchor.UpperRight, fontStyle = FontStyle.Bold });
                GUI.Label(new Rect(slot.x + 4, slot.y + box - 30, box - 8, 28), ItemName(c.id, c.name), new GUIStyle(small) { alignment = TextAnchor.MiddleCenter, wordWrap = true, clipping = TextClipping.Clip });
                GUILayout.Space(gap);
            }
            GUILayout.EndHorizontal();
        }
    }

    /// Item name from the slot, or from the team inventory when only the id is known.
    string ItemName(string id, string name)
    {
        if (name != null && name != id) return name;
        return (string)view.PlayerInfo?["inventory"]?.FirstOrDefault(i => (string)i["id"] == id)?["name"] ?? id.Replace('_', ' ');
    }

    readonly Dictionary<string, Texture2D> icons = new();

    /// Bag icon: item_<id>, else item_<category>, else the generic pouch.
    Texture2D ItemIcon(string id, string category)
    {
        string key = id + "|" + category;
        if (icons.TryGetValue(key, out var t)) return t;
        t = Resources.Load<Texture2D>("Sprites/item_" + id);
        if (t == null && category != null) t = Resources.Load<Texture2D>("Sprites/item_" + category);
        if (t == null) t = Resources.Load<Texture2D>("Sprites/item_varie");
        if (t != null) t.filterMode = FilterMode.Point;
        icons[key] = t;
        return t;
    }

    void ActionsButton(float width, long target)
    {
        if (GUILayout.Button("AZIONI" + Key("Spazio"), bigButton, GUILayout.Width(width), GUILayout.Height(rowH + 14))) OpenActions(target, Win.Card);
    }

    // ── Actions with a pawn or building ──────────────────────────────────────

    /// The screen of what can be done with <paramref name="target"/>: talk and the champion's actions, plus
    /// the orders it takes if it is one of ours.
    void OpenActions(long target, Win from)
    {
        actionsTarget = target;
        interactions = null;
        if (view.PlayerId != null) view.Interactions(target, j => { if (actionsTarget == target) interactions = j; });
        if (view.Selected != target) view.Select(target, byChampion: true);
        OpenSub(Win.Actions, from);
    }

    void ActionsWin(float width)
    {
        long target = actionsTarget;
        var i = interactions;
        if (i == null)
        {
            GUILayout.Label("…", big);
            return;
        }
        int? dist = (int?)i["distance"];
        GUILayout.Label(dist == null ? "Su un'altra mappa." : dist <= 1 ? "È accanto al campione." : $"A {dist} passi dal campione: per parlarci devi avvicinarti.", small, GUILayout.Width(width));
        var champActions = (i["actions"] as JArray)?.ToList() ?? new List<JToken>();
        if (champActions.Count > 0)
        {
            Section("Il campione con " + i["name"]);
            BigButtons(width, champActions, a =>
            {
                if ((string)a["kind"] == "talk") StartTalk(target);
                else if (view.ChampionId is long c)
                {
                    view.SendPlayerOrder(c, SimView.OrderFor((string)a["kind"], (string)a["id"], target));
                    Close();
                }
            });
        }
        if (view.IsMine(target) && view.Selected == target && view.SelectedActions != null && target != view.ChampionId)
        {
            var m = view.Member(target);
            Section($"Ordini a {i["name"]} (obbedienza {((float?)m?["obedience"] ?? 0) * 100:0}%)");
            BigButtons(width, view.SelectedActions, a =>
            {
                if ((string)a["needs_target"] == "none") view.SendPlayerOrder(target, SimView.OrderFor((string)a["kind"], (string)a["id"], null));
                else
                {
                    // The order waits for a target on the map: the windows step aside.
                    win = Win.None;
                    stack.Clear();
                    view.PendingAction = (JObject)a;
                    return;
                }
                Close();
            });
        }
        else if (target == view.ChampionId && view.SelectedActions != null)
        {
            Section("Ordini al campione");
            BigButtons(width, view.SelectedActions, a =>
            {
                if ((string)a["needs_target"] == "none") view.SendPlayerOrder(target, SimView.OrderFor((string)a["kind"], (string)a["id"], null));
                else
                {
                    win = Win.None;
                    stack.Clear();
                    view.PendingAction = (JObject)a;
                    return;
                }
                Close();
            });
        }
        if (champActions.Count == 0 && !view.IsMine(target)) GUILayout.Label("Nessuna azione possibile.", big);
    }

    void BigButtons(float width, IEnumerable<JToken> actions, System.Action<JToken> run)
    {
        int perRow = width > 520 ? 3 : 2, col = 0;
        GUILayout.BeginHorizontal();
        foreach (var a in actions)
        {
            if (GUILayout.Button((string)a["name"], bigButton, GUILayout.Width(width / perRow - 8), GUILayout.Height(rowH + 12))) run(a);
            if (++col % perRow == 0)
            {
                GUILayout.EndHorizontal();
                GUILayout.BeginHorizontal();
            }
        }
        GUILayout.EndHorizontal();
    }

    // ── Talking ──────────────────────────────────────────────────────────────

    void StartTalk(long target)
    {
        Close();
        talkName = "";
        talkLine = "…";
        view.Talk(target, (name, line) =>
        {
            talkName = name;
            talkLine = line;
        });
    }

    /// Pokémon text box with what the pawn says; a tap, A or Space closes it.
    void TalkBox(float w, float h)
    {
        float tw = Mathf.Min(820, w - 32), th = touch ? 130 : 120;
        var r = Ui(new Rect(w / 2 - tw / 2, h - th - 12, tw, th));
        string text = string.IsNullOrEmpty(talkName) ? talkLine : talkName + ":\n" + talkLine;
        GUI.Label(r, text, textBox);
        if (Mathf.Repeat(Time.time, 1f) < 0.6f) GUI.Label(new Rect(r.xMax - 30, r.yMax - 28, 20, 20), "▼", label);
        if (Event.current.type == EventType.MouseDown && r.Contains(Event.current.mousePosition))
        {
            CloseTalk();
            Event.current.Use();
        }
    }

    void CloseTalk()
    {
        talkLine = talkName = null;
        talkClosedAt = Time.time;
    }

    // ── Item detail ──────────────────────────────────────────────────────────

    void OpenItem(string id, Win from)
    {
        itemId = id;
        itemInfo = null;
        view.ItemInfo(id, j => { if (itemId == id) itemInfo = j; });
        OpenSub(Win.Item, from);
    }

    void ItemWin(float width)
    {
        var d = itemInfo;
        if (d == null)
        {
            GUILayout.Label("…", big);
            return;
        }
        GUILayout.BeginHorizontal();
        var r = GUILayoutUtility.GetRect(96, 96, GUILayout.Width(96), GUILayout.Height(96));
        GUI.Box(r, GUIContent.none, slotStyle);
        var icon = ItemIcon((string)d["id"], (string)d["category"]);
        if (icon != null) GUI.DrawTexture(new Rect(r.x + 16, r.y + 16, 64, 64), icon);
        GUILayout.Space(12);
        GUILayout.BeginVertical();
        GUILayout.Label((string)d["name"], new GUIStyle(title) { fontSize = title.fontSize + 6 });
        GUILayout.Label(Cap(((string)d["category"]).Replace('_', ' ')) + ((bool?)d["unique"] == true ? " · pezzo unico" : ""), big);
        double price = (double?)d["price"] ?? (double)d["base_price"];
        GUILayout.Label($"Valore: {price:0.00} € (base {(double)d["base_price"]:0.00} €)", big);
        GUILayout.EndVertical();
        GUILayout.EndHorizontal();
        if (!string.IsNullOrEmpty((string)d["description"])) GUILayout.Label((string)d["description"], big, GUILayout.Width(width));
        Section("Cosa fa");
        foreach (var line in d["effects"]) GUILayout.Label("• " + (string)line, big, GUILayout.Width(width));

        Section("Chi ce l'ha nel team");
        var holders = (d["holders"] as JArray)?.ToList() ?? new List<JToken>();
        if (holders.Count == 0) GUILayout.Label("Nessuno.", big);
        foreach (var hd in holders)
            GUILayout.Label($"{hd["name"]}{((bool?)hd["building"] == true ? " (edificio)" : "")}: {hd["qty"]}", big, GUILayout.Width(width));

        var members = view.PlayerInfo?["members"]?.ToList();
        if (members == null || members.Count == 0) return;
        Section("Assegna o usa");
        giveIndex = Mathf.Clamp(giveIndex, 0, members.Count - 1);
        var who = members[giveIndex];
        GUILayout.BeginHorizontal();
        if (GUILayout.Button("◀", bigButton, GUILayout.Width(rowH + 8), GUILayout.Height(rowH + 8))) giveIndex = (giveIndex + members.Count - 1) % members.Count;
        GUILayout.Label(((bool?)who["champion"] == true ? "★ " : "") + (string)who["name"], new GUIStyle(big) { alignment = TextAnchor.MiddleCenter, fontStyle = FontStyle.Bold }, GUILayout.Width(width * 0.45f), GUILayout.Height(rowH + 8));
        if (GUILayout.Button("▶", bigButton, GUILayout.Width(rowH + 8), GUILayout.Height(rowH + 8))) giveIndex = (giveIndex + 1) % members.Count;
        GUILayout.EndHorizontal();
        GUILayout.BeginHorizontal();
        long whoId = (long)who["id"];
        string item = (string)d["id"];
        if (GUILayout.Button($"Assegna a {who["name"]}", bigButton, GUILayout.Width(width / 2 - 6), GUILayout.Height(rowH + 12)))
            view.SendPlayerCommand(new JObject { ["type"] = "player_give_item", ["item"] = item, ["to"] = whoId, ["qty"] = 1 }, () => view.ItemInfo(item, j => { if (itemId == item) itemInfo = j; }));
        GUI.enabled = !exitAsk && (bool?)d["usable"] == true;
        if (GUILayout.Button((bool?)d["usable"] == true ? $"{who["name"]} lo usa" : "Non si usa", bigButton, GUILayout.Width(width / 2 - 6), GUILayout.Height(rowH + 12)))
            view.SendPlayerCommand(new JObject { ["type"] = "player_use_item", ["entity"] = whoId, ["item"] = item }, () => view.ItemInfo(item, j => { if (itemId == item) itemInfo = j; }));
        GUI.enabled = !exitAsk;
        GUILayout.EndHorizontal();
        if (!string.IsNullOrEmpty(view.LastCommandResult)) GUILayout.Label(view.LastCommandResult, big, GUILayout.Width(width));
    }

    // ── Joystick and A button ────────────────────────────────────────────────

    const float StickRadius = 62, KnobRadius = 26;
    Vector2 stickCenter, stickKnob;
    int stickFinger = -1;

    bool ControlsShown => !hidden || touch;

    /// Circle to drag for walking the champion (bottom left, beside the column) and the A button that
    /// offers the actions with whoever is next to the champion.
    void Joystick(float w, float h)
    {
        stickCenter = new Vector2(colW + 24 + StickRadius, h - 20 - StickRadius);
        var baseRect = Ui(new Rect(stickCenter.x - StickRadius, stickCenter.y - StickRadius, StickRadius * 2, StickRadius * 2));
        GUI.DrawTexture(baseRect, circleBase);
        var k = stickCenter + stickKnob;
        GUI.DrawTexture(new Rect(k.x - KnobRadius, k.y - KnobRadius, KnobRadius * 2, KnobRadius * 2), circleKnob);
        aRect = Ui(new Rect(w - 24 - 84, h - 24 - 84, 84, 84));
        bool any = nearby.HasValue;
        GUI.color = any ? Color.white : new Color(1, 1, 1, 0.45f);
        GUI.DrawTexture(aRect, circleA);
        GUI.Label(aRect, "A", new GUIStyle(padStyle) { normal = { background = null, textColor = Color.white }, fontSize = 30 });
        GUI.color = Color.white;
        if (any) Prompt(new Rect(aRect.x - 200, aRect.y - 40, 284, 32));
    }

    /// What A (or Space) would do now.
    void Prompt(Rect r)
    {
        var e = view.EntityById(nearby.Value);
        if (e == null) return;
        bool building = (string)e["kind"] == "building";
        string text = (touch ? "" : "Spazio: ") + (building ? "azioni con " : "parla con ") + (string)e["name"];
        GUI.Label(r, text, promptStyle);
    }

    /// The finger on the joystick sets the walking direction (8 ways); a tap on A opens the actions.
    void TouchControls()
    {
        var dir = Vector2Int.zero;
        bool shown = PadShown && view.Zoom >= 1f && !MenuOpen && !exitAsk;
        bool held = false;
        for (int i = 0; shown && i < Input.touchCount; i++)
        {
            var t = Input.GetTouch(i);
            var p = new Vector2(t.position.x, Screen.height - t.position.y) / scale;
            if (t.phase == TouchPhase.Began)
            {
                if (Vector2.Distance(p, stickCenter) <= StickRadius * 1.4f) stickFinger = t.fingerId;
                else if (aRect.Contains(p) && nearby.HasValue) OpenActions(nearby.Value, Win.None);
            }
            if (t.fingerId != stickFinger) continue;
            if (t.phase == TouchPhase.Ended || t.phase == TouchPhase.Canceled)
            {
                stickFinger = -1;
                continue;
            }
            held = true;
            var d = p - stickCenter;
            stickKnob = Vector2.ClampMagnitude(d, StickRadius);
            if (d.magnitude > StickRadius * 0.3f)
            {
                // Eight sectors of 45°; screen y grows downwards like the map's.
                int sector = Mathf.RoundToInt(Mathf.Atan2(d.y, d.x) / (Mathf.PI / 4f));
                var dirs = new[] { new Vector2Int(1, 0), new Vector2Int(1, 1), new Vector2Int(0, 1), new Vector2Int(-1, 1), new Vector2Int(-1, 0), new Vector2Int(-1, -1), new Vector2Int(0, -1), new Vector2Int(1, -1) };
                dir = dirs[(sector + 8) % 8];
            }
        }
        if (!held)
        {
            stickKnob = Vector2.zero;
            if (!shown) stickFinger = -1;
        }
        view.PadWalk = dir;
    }

    /// Short message after a command (an order taken or refused, an item given…).
    void Toast(float w, float h)
    {
        var msg = view.LastCommandResult;
        if (msg != lastResult)
        {
            lastResult = msg;
            if (!string.IsNullOrEmpty(msg)) toastUntil = Time.time + 4f;
        }
        if (string.IsNullOrEmpty(msg) || Time.time > toastUntil || win == Win.Item) return;
        float tw = Mathf.Min(560, w - 2 * colW - 48);
        GUI.Label(new Rect(w / 2 - tw / 2, topBottom + 6, tw, 34), msg, promptStyle);
    }

    // ── Small windows ────────────────────────────────────────────────────────

    /// The order waiting for a target (or the dig rectangle) and a way out of it.
    void PendingBox(float w)
    {
        string what = view.DigMode ? "Scava: trascina un rettangolo sulla mappa"
            : (touch ? "Tocca" : "Clicca") + " il bersaglio: " + view.PendingAction["name"];
        float bw = Mathf.Min(touch ? 480 : 440, w - 2 * colW - 40);
        var r = Ui(new Rect(w / 2 - bw / 2, topBottom + 4, bw, rowH + 16));
        GUILayout.BeginArea(r, box);
        GUILayout.BeginHorizontal();
        GUILayout.Label(what, label);
        if (GUILayout.Button("Annulla" + Key("Esc"), button, GUILayout.Width(touch ? 100 : 110))) view.CancelCurrent();
        GUILayout.EndHorizontal();
        GUILayout.EndArea();
    }

    void GoToChampion()
    {
        if (!view.ChampionId.HasValue) return;
        view.Select(view.ChampionId);
        view.FocusOn(view.ChampionId.Value);
    }

    void ExitWindow(float w, float h)
    {
        GUI.DrawTexture(new Rect(0, 0, w, h), dimTex);
        float ew = Mathf.Min(touch ? 440 : 380, w - 32), eh = touch ? 190 : 150;
        GUILayout.BeginArea(new Rect(w / 2 - ew / 2, h / 2 - eh / 2, ew, eh), box);
        GUILayout.Label("Vuoi uscire dal gioco?", title);
        GUILayout.Label("Il mondo continua a girare sul server: rientrando lo ritrovi dove l'hai lasciato.", label);
        GUILayout.FlexibleSpace();
        GUILayout.BeginHorizontal();
        if (GUILayout.Button("Esci" + Key("Invio"), button, GUILayout.Height(rowH))) Quit();
        if (GUILayout.Button("Annulla" + Key("Esc"), button, GUILayout.Height(rowH))) exitAsk = false;
        GUILayout.EndHorizontal();
        GUILayout.EndArea();
    }

    static void Quit()
    {
#if UNITY_EDITOR
        UnityEditor.EditorApplication.isPlaying = false;
#else
        Application.Quit();
#endif
    }

    /// Esc / back button: close the exit question, else cancel what is in progress, else close the open
    /// window or card, else ask to quit.
    void Back()
    {
        if (exitAsk) exitAsk = false;
        else if (talkLine != null) CloseTalk();
        else if (view.CancelCurrent()) { }
        else if (serverOpen && view.Map != null) serverOpen = false;
        else if (win != Win.None && stack.Count > 0)
        {
            win = stack[^1];
            stack.RemoveAt(stack.Count - 1);
            scrollWin = Vector2.zero;
            view.PigeonOpen = win == Win.Pigeon;
            view.EconomyOpen = win == Win.Economy;
        }
        else if (win != Win.None) Close();
        else if (view.Selected.HasValue) view.Select(null);
        else exitAsk = true;
    }

    string ZoneName(string id) => (string)view.Map?["zones"]?.FirstOrDefault(z => (string)z["id"] == id)?["name"] ?? id;

    List<JToken> Factions() => view.State?["snapshot"]?["factions"]?.ToList() ?? new List<JToken>();

    string FactionName(string id) => (string)Factions().FirstOrDefault(f => (string)f["id"] == id)?["name"] ?? id;

    /// Whole number with dots between thousands, as written in Italy (whatever the device's language).
    static string Fmt(double v)
    {
        long n = (long)System.Math.Round(v);
        string digits = System.Math.Abs(n).ToString();
        var sb = new System.Text.StringBuilder();
        for (int i = 0; i < digits.Length; i++)
        {
            if (i > 0 && (digits.Length - i) % 3 == 0) sb.Append('.');
            sb.Append(digits[i]);
        }
        return (n < 0 ? "-" : "") + sb;
    }

    void Section(string text)
    {
        GUILayout.Space(8);
        GUILayout.Label(text.ToUpperInvariant(), title);
    }

    void Update()
    {
        if (Input.GetKeyDown(KeyCode.Escape)) Back();
        if (exitAsk && (Input.GetKeyDown(KeyCode.Return) || Input.GetKeyDown(KeyCode.KeypadEnter))) Quit();
        bool ctrl = Input.GetKey(KeyCode.LeftControl) || Input.GetKey(KeyCode.RightControl) || Input.GetKey(KeyCode.LeftCommand);
        if (ctrl && Input.GetKeyDown(KeyCode.Q)) exitAsk = true;
        if (!Modal && !ctrl && view.State != null)
        {
            if (Input.GetKeyDown(KeyCode.Tab)) SetHidden(!hidden);
            if (Input.GetKeyDown(KeyCode.M)) Open(Win.Map);
            if (Input.GetKeyDown(KeyCode.I)) Open(Win.Inventory);
            if (Input.GetKeyDown(KeyCode.T)) Open(Win.Team);
            if (Input.GetKeyDown(KeyCode.O)) Open(Win.Options);
            if (Input.GetKeyDown(KeyCode.P)) Open(Win.Pigeon);
            if (Input.GetKeyDown(KeyCode.L)) Open(Win.Ranking);
            if (Input.GetKeyDown(KeyCode.R)) Open(Win.Economy);
            if (Input.GetKeyDown(KeyCode.H) || Input.GetKeyDown(KeyCode.F1)) Open(Win.Manual);
        }
        nearby = view.State != null && view.Zoom >= 1f ? view.NearbyTarget() : null;
        // Space / E / Enter: closes what someone is saying, otherwise the actions with whoever is next to us.
        bool act = !Modal && (Input.GetKeyDown(KeyCode.Space) || Input.GetKeyDown(KeyCode.E) || Input.GetKeyDown(KeyCode.Return));
        if (act && talkLine != null && Time.time - talkClosedAt > 0.1f) CloseTalk();
        else if (act && !MenuOpen && nearby.HasValue && Time.time - talkClosedAt > 0.2f) OpenActions(nearby.Value, Win.None);
        else if (act && win == Win.Card && view.Selected is long sel) OpenActions(sel, Win.Card);
        view.MovementLocked = MenuOpen || exitAsk || ConnectShown;
        if (touch)
        {
            TouchControls();
            TouchScroll();
        }
    }

    /// IMGUI scroll views only follow their scrollbars: on a phone the finger drags the content.
    void TouchScroll()
    {
        if (Input.touchCount != 1)
        {
            if (Input.touchCount == 0) touchDragged = false;
            touchScroll = 0;
            return;
        }
        var t = Input.GetTouch(0);
        var p = new Vector2(t.position.x, Screen.height - t.position.y) / scale;
        if (t.phase == TouchPhase.Began)
        {
            touchDragged = false;
            touchScroll = exitAsk || hidden ? 0 : win != Win.None && winRect.Contains(p) ? 1 : 0;
            return;
        }
        if (touchScroll == 0 || t.phase != TouchPhase.Moved) return;
        var d = t.deltaPosition / scale;
        if (!touchDragged && d.magnitude < 0.5f) return;
        touchDragged = true;
        scrollWin.y = Mathf.Max(0, scrollWin.y + d.y);
    }

    /// Address of the sim server: asked when it cannot be reached (and from the "Server" button),
    /// remembered on the device.
    void ConnectWindow(float w, float h)
    {
        serverField ??= view.Api?.BaseUrl ?? "";
        float cw = Mathf.Min(520, w - 16);
        var connectRect = Ui(new Rect(Mathf.Max(8, w / 2 - cw / 2), Mathf.Max(10, h / 2 - 120), cw, 210));
        GUILayout.BeginArea(connectRect, box);
        GUILayout.Label("Collegati al server", title);
        GUILayout.Label(view.Error ?? "Collegamento in corso…", small);
        GUILayout.Label("Indirizzo del PC che fa girare il server (stessa rete Wi-Fi), es. 192.168.1.186:8787", label);
        serverField = GUILayout.TextField(serverField, new GUIStyle(GUI.skin.textField) { fontSize = touch ? 18 : 13, padding = new RectOffset(8, 8, 8, 8) });
        GUILayout.BeginHorizontal();
        if (GUILayout.Button("Connetti", button)) view.SetServer(serverField);
        if (view.Map != null && GUILayout.Button("Chiudi", button, GUILayout.Width(90))) serverOpen = false;
        if (GUILayout.Button("Esci", button, GUILayout.Width(80))) exitAsk = true;
        GUILayout.EndHorizontal();
        GUILayout.EndArea();
    }

    /// Place name shown for a moment when entering a map, as in the handheld games.
    void LocationBanner(float w)
    {
        if (view.Layers == null) return;
        // The map, or on the big maps the quarter at the centre of the screen.
        var l = view.Layers[view.Layer];
        var c = Camera.main.transform.position;
        int cx = Mathf.FloorToInt(c.x), cy = Mathf.FloorToInt(-c.y);
        var zone = view.Map["zones"]?
            .Where(z => (int)z["layer"] == view.Layer && (int)z["w"] * (int)z["h"] >= 400
                        && cx >= (int)z["x"] && cx < (int)z["x"] + (int)z["w"] && cy >= (int)z["y"] && cy < (int)z["y"] + (int)z["h"]
                        && (int)z["w"] < (int)l["width"])
            .OrderBy(z => (int)z["w"] * (int)z["h"]).FirstOrDefault();
        int depth = (int?)l["depth"] ?? 0;
        string name = zone != null ? (string)zone["name"] : (string)l["name"];
        var owner = zone != null ? (string)view.Territory((string)zone["id"])?["owner"] : null;
        if (owner != null) name += "  ·  " + FactionName(owner);
        if (depth < 0) name += $"  ·  livello {depth}";
        if (name != bannerKey)
        {
            bannerKey = name;
            bannerUntil = Time.time + 3f;
        }
        if (Time.time > bannerUntil || view.Zoom < 1f) return;
        float bw = Mathf.Max(220, name.Length * 9 + 40);
        GUI.Label(new Rect(w / 2 - bw / 2, hidden ? 12 : topBottom + (PendingShown ? rowH + 24 : 4), bw, 40), name, banner);
    }

    void HoverTip()
    {
        if (view.State == null || IsOverUi(Input.mousePosition)) return;
        var id = view.Pick(Input.mousePosition);
        if (!id.HasValue) return;
        var e = view.EntityById(id.Value);
        if (e == null) return;
        string text = (string)e["name"] + "\n" + ((string)e["activity"]?["label"] ?? (string)e["building"]?["def"] ?? "");
        var m = new Vector2(Input.mousePosition.x, Screen.height - Input.mousePosition.y) / scale;
        GUI.Label(new Rect(m.x + 14, m.y + 10, 240, 44), text, hover);
    }
}
