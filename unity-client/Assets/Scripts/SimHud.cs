using System.Collections.Generic;
using System.Linq;
using Newtonsoft.Json.Linq;
using UnityEngine;

/// <summary>
/// Immediate-mode HUD. Top bar: time, pause, speed, layers, fog, champion, save/load.
/// Right panel: selected pawn (with orders if it is ours), then the "Mondo" tab (feed, chronicle,
/// market, factions) or the "La mia fazione" tab (members and their obedience, squads, salaries,
/// work priorities of the selected member).
/// </summary>
[RequireComponent(typeof(SimView))]
public class SimHud : MonoBehaviour
{
    SimView view;
    float scale = 1f;
    Vector2 scrollRight;
    Rect topRect, rightRect;
    GUIStyle box, title, small, label, zoneLabel, button, fake, hover, good, toggle, textBox, banner;
    Texture2D panelTex, barTex, barFillTex;
    bool panelHidden;
    string bannerKey;
    float bannerUntil, noticeUntil;
    string noticeKey, noticeText;
    int speedIndex = 1;
    int tab;
    bool mapMenu;
    Vector2 mapScroll;
    Rect mapRect;
    static readonly (string name, int ms)[] Speeds = { ("Lenta", 1000), ("Normale", 400), ("Veloce", 120), ("Turbo", 30) };
    static readonly string[] Tabs = { "Mondo", "La mia fazione" };

    void Awake() => view = GetComponent<SimView>();

    public void ShowFactionTab() => tab = 1;

    public bool IsOverUi(Vector3 mouse)
    {
        var p = new Vector2(mouse.x, Screen.height - mouse.y) / scale;
        return topRect.Contains(p) || (!panelHidden && rightRect.Contains(p)) || (mapMenu && mapRect.Contains(p));
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
        barTex = Tex(new Color(0.2f, 0.25f, 0.3f, 0.25f));
        barFillTex = Tex(new Color(0.35f, 0.78f, 0.45f));
        var slice = new RectOffset(4, 4, 4, 4);
        box = new GUIStyle(GUI.skin.box) { normal = { background = panelTex }, border = slice, padding = new RectOffset(12, 12, 8, 8), alignment = TextAnchor.UpperLeft };
        title = new GUIStyle(GUI.skin.label) { fontStyle = FontStyle.Bold, fontSize = 13, normal = { textColor = new Color(0.2f, 0.36f, 0.66f) } };
        label = new GUIStyle(GUI.skin.label) { wordWrap = true, fontSize = 12, normal = { textColor = ink } };
        small = new GUIStyle(label) { fontSize = 11, normal = { textColor = new Color(0.45f, 0.47f, 0.52f) } };
        fake = new GUIStyle(label) { normal = { textColor = new Color(0.8f, 0.22f, 0.18f) } };
        good = new GUIStyle(label) { normal = { textColor = new Color(0.16f, 0.55f, 0.28f) } };
        zoneLabel = new GUIStyle(GUI.skin.label) { fontSize = 11, fontStyle = FontStyle.Bold, normal = { textColor = new Color(0.98f, 0.98f, 0.95f) } };
        hover = new GUIStyle(box) { fontSize = 12, wordWrap = true, padding = new RectOffset(8, 8, 5, 5), normal = { background = panelTex, textColor = ink } };
        button = new GUIStyle(GUI.skin.button)
        {
            fontSize = 12, border = slice, padding = new RectOffset(6, 6, 4, 4),
            normal = { background = btnTex, textColor = ink }, hover = { background = btnHover, textColor = ink },
            active = { background = btnDown, textColor = ink }, focused = { background = btnTex, textColor = ink },
            onNormal = { background = btnHover, textColor = ink }, onHover = { background = btnHover, textColor = ink },
            onActive = { background = btnDown, textColor = ink },
        };
        toggle = new GUIStyle(GUI.skin.toggle) { normal = { textColor = ink }, onNormal = { textColor = ink }, hover = { textColor = ink }, onHover = { textColor = ink } };
        textBox = new GUIStyle(box) { fontSize = 16, wordWrap = true, padding = new RectOffset(20, 20, 14, 14), normal = { background = panelTex, textColor = ink } };
        banner = new GUIStyle(box) { fontSize = 15, fontStyle = FontStyle.Bold, alignment = TextAnchor.MiddleCenter, normal = { background = panelTex, textColor = ink } };
    }

    void OnGUI()
    {
        InitStyles();
        scale = Mathf.Max(1f, Screen.height / 900f);
        GUI.matrix = Matrix4x4.Scale(new Vector3(scale, scale, 1));
        float w = Screen.width / scale, h = Screen.height / scale;

        float pw = panelHidden ? 0 : Mathf.Min(400, w * 0.42f);
        if (view.Map != null && view.Zoom < 1f) ZoneLabels();
        LocationBanner();
        Notice(w - pw, h);
        HoverTip();
        if (view.PendingAction != null)
        {
            var m = new Vector2(Input.mousePosition.x, Screen.height - Input.mousePosition.y) / scale;
            GUI.Label(new Rect(m.x + 14, m.y - 26, 260, 22), "Scegli il bersaglio: " + view.PendingAction["name"] + " (Esc annulla)", hover);
        }

        topRect = new Rect(0, 0, w, 34);
        GUILayout.BeginArea(topRect, box);
        GUILayout.BeginHorizontal();
        TopBar();
        GUILayout.EndHorizontal();
        GUILayout.EndArea();

        if (mapMenu && view.Layers != null)
        {
            // Map list grouped as in the data: outdoors, interiors, underground.
            mapRect = new Rect(420, 34, 280, Mathf.Min(h - 60, 22 * view.Layers.Count + 20));
            GUILayout.BeginArea(mapRect, box);
            mapScroll = GUILayout.BeginScrollView(mapScroll);
            for (int i = 0; i < view.Layers.Count; i++)
            {
                var l = view.Layers[i];
                string kind = (bool?)l["underground"] == true ? "⤓ " : (bool?)l["indoor"] == true ? "⌂ " : "";
                var st = new GUIStyle(button) { alignment = TextAnchor.MiddleLeft, fontStyle = i == view.Layer ? FontStyle.Bold : FontStyle.Normal };
                if (GUILayout.Button(kind + (string)l["name"], st))
                {
                    view.SetLayer(i);
                    view.FollowCamera = false;
                    mapMenu = false;
                }
            }
            GUILayout.EndScrollView();
            GUILayout.EndArea();
        }

        if (panelHidden) return;
        rightRect = new Rect(w - pw, 34, pw, h - 34);
        GUILayout.BeginArea(rightRect, box);
        scrollRight = GUILayout.BeginScrollView(scrollRight);
        float inner = pw - 34;
        if (view.State == null)
            GUILayout.Label("In attesa del server…\nAvvia: cd engine && cargo run --release -p fidenza_world -- --serve", label);
        else
        {
            Selected(inner);
            GUILayout.Space(8);
            tab = GUILayout.Toolbar(tab, Tabs, button);
            if (tab == 0) WorldTab(inner);
            else FactionTab(inner);
        }
        GUILayout.EndScrollView();
        GUILayout.EndArea();
    }

    void TopBar()
    {
        GUILayout.Label("Fidenza & Salsomaggiore", title, GUILayout.Width(180));
        if (view.Error != null)
        {
            GUILayout.Label(view.Error, fake);
            return;
        }
        long tick = (long?)view.State?["snapshot"]?["tick"] ?? 0;
        GUILayout.Label($"tick {tick} · g.{tick / 24 + 1} {tick % 24}:00", label, GUILayout.Width(120));
        bool paused = (bool?)view.Control?["paused"] ?? false;
        if (GUILayout.Button(paused ? "Riprendi" : "Pausa", button, GUILayout.Width(72)))
            view.SendControl(new JObject { ["paused"] = !paused });
        if (GUILayout.Button("+1", button, GUILayout.Width(30)))
            view.SendControl(new JObject { ["step"] = 1 });
        if (GUILayout.Button(Speeds[speedIndex].name, button, GUILayout.Width(70)))
        {
            speedIndex = (speedIndex + 1) % Speeds.Length;
            view.SendControl(new JObject { ["tick_ms"] = Speeds[speedIndex].ms });
        }
        if (view.Layers != null && GUILayout.Button("Mappa: " + (string)view.Layers[view.Layer]["name"] + " ▾", button, GUILayout.Width(250)))
            mapMenu = !mapMenu;
        var factions = Factions();
        string fogName = string.IsNullOrEmpty(view.FogFaction) ? "nessuna" : FactionName(view.FogFaction);
        if (GUILayout.Button("Nebbia: " + fogName, button, GUILayout.Width(200)) && factions.Count > 0)
        {
            var ids = new List<string> { "" };
            ids.AddRange(factions.Select(f => (string)f["id"]));
            int i = ids.IndexOf(view.FogFaction);
            view.ChooseFog(ids[(i + 1) % ids.Count]);
        }
        if (view.ChampionId.HasValue && GUILayout.Button("Campione (C)", button, GUILayout.Width(100)))
        {
            view.Select(view.ChampionId);
            view.FocusOn(view.ChampionId.Value);
        }
        view.FollowCamera = GUILayout.Toggle(view.FollowCamera, "Segui (F)", toggle, GUILayout.Width(80));
        if (GUILayout.Button(view.Zoom >= 1f ? "Panoramica (M)" : "Da vicino (M)", button, GUILayout.Width(115))) view.ToggleOverview();
        if (GUILayout.Button(panelHidden ? "Pannello ◂ (Tab)" : "Pannello ▸ (Tab)", button, GUILayout.Width(120))) panelHidden = !panelHidden;
        if (GUILayout.Button(view.DigMode ? "▸ Scava: trascina (Esc)" : "Scava", button, GUILayout.Width(view.DigMode ? 160 : 60)))
            view.DigMode = !view.DigMode;
        if (GUILayout.Button("Salva", button, GUILayout.Width(55))) view.Save();
        if (GUILayout.Button("Carica", button, GUILayout.Width(60))) view.Load();
        GUILayout.FlexibleSpace();
    }

    List<JToken> Factions() => view.State?["snapshot"]?["factions"]?.ToList() ?? new List<JToken>();

    string FactionName(string id) => (string)Factions().FirstOrDefault(f => (string)f["id"] == id)?["name"] ?? id;

    void Section(string text)
    {
        GUILayout.Space(8);
        GUILayout.Label(text.ToUpperInvariant(), title);
    }

    void WorldTab(float width)
    {
        var snap = view.State["snapshot"];
        Section((string)snap["feed_name"] ?? "Feed");
        // Filters: important news by default, so the gossip does not bury what matters.
        var filters = new List<(string id, string name)> { ("important", "Importanti"), ("mine", "Per te") };
        if (view.Feed?["categories"] is JArray cats)
            filters.AddRange(cats.Select(c => ((string)c["id"], $"{c["name"]} ({c["count"]})")));
        filters.Add(("all", "Tutto"));
        int perRow = 3;
        for (int i = 0; i < filters.Count; i += perRow)
        {
            GUILayout.BeginHorizontal();
            foreach (var (id, name) in filters.Skip(i).Take(perRow))
            {
                var st = new GUIStyle(button) { fontStyle = view.FeedFilter == id ? FontStyle.Bold : FontStyle.Normal };
                if (GUILayout.Button(view.FeedFilter == id ? "▸ " + name : name, st, GUILayout.Width(width / perRow - 4)))
                    view.FeedFilter = id;
            }
            GUILayout.EndHorizontal();
        }
        var articles = view.Feed?["articles"] as JArray;
        if (articles == null || articles.Count == 0)
            GUILayout.Label("Niente da segnalare.", small);
        else
            foreach (var a in articles.Take(8))
            {
                string tag = ((bool?)a["propaganda"] ?? false) ? "PROPAGANDA · " : "";
                bool mine = (bool?)a["mine"] ?? false;
                GUILayout.Label($"[{a["tick"]}] {tag}{a["headline"]}", mine ? good : label, GUILayout.Width(width));
                GUILayout.Label($"— {a["author_name"]} · {a["category"]}", small);
            }

        Section("Cronaca");
        foreach (var e in view.State["recent_events"].Where(e => (string)e["kind"] != "article").Take(8))
            GUILayout.Label($"[{e["tick"]}] {e["message"]}", label, GUILayout.Width(width));

        Section("Mercato (più mossi)");
        var moved = snap["market"].OrderByDescending(m => Mathf.Abs(((float)m["price"] - (float)m["base"]) / (float)m["base"])).Take(8);
        foreach (var m in moved)
        {
            float d = ((float)m["price"] - (float)m["base"]) / (float)m["base"] * 100f;
            GUILayout.BeginHorizontal();
            GUILayout.Label((string)m["name"], label, GUILayout.Width(width * 0.55f));
            GUILayout.Label($"{(float)m["price"]:0.00} €", label, GUILayout.Width(width * 0.25f));
            GUILayout.Label($"{d:+0;-0}%", d > 0 ? fake : good);
            GUILayout.EndHorizontal();
        }

        Section("Fazioni");
        var scores = snap["scores"] as JObject;
        foreach (var f in snap["factions"].Where(f => f["absorbed_into"].Type == JTokenType.Null)
                     .OrderByDescending(f => (int?)scores?[(string)f["id"]] ?? 0))
        {
            GUILayout.BeginHorizontal();
            string star = f["controlled_by"].Type != JTokenType.Null ? " ★" : "";
            GUILayout.Label((string)f["name"] + star, label, GUILayout.Width(width * 0.55f));
            GUILayout.Label($"{f["members"]} · {(float)f["treasury"]:0} €", small, GUILayout.Width(width * 0.27f));
            GUILayout.Label($"{scores?[(string)f["id"]] ?? 0} PV", small);
            GUILayout.EndHorizontal();
        }
    }

    void FactionTab(float width)
    {
        var p = view.PlayerInfo;
        if (p == null)
        {
            GUILayout.Label("Nessun giocatore in questa partita.", small);
            return;
        }
        Section(FactionName((string)p["faction"]));
        GUILayout.Label($"Fondo di gilda {(float)p["treasury"]:0} € · {p["victory_points"]} punti vittoria", label, GUILayout.Width(width));

        Section("Membri (obbedienza)");
        foreach (var m in p["members"])
        {
            long id = (long)m["id"];
            bool champ = (bool?)m["champion"] ?? false;
            float ob = (float?)m["obedience"] ?? 1f;
            GUILayout.BeginHorizontal();
            if (GUILayout.Button((champ ? "★ " : "") + (string)m["name"], button, GUILayout.Width(width * 0.45f)))
            {
                view.Select(id);
                view.FocusOn(id);
            }
            GUILayout.Label(champ ? "sempre" : $"{ob * 100:0}%", ob < 0.5f && !champ ? fake : small, GUILayout.Width(width * 0.15f));
            GUILayout.Label((string)m["activity"] ?? "", small, GUILayout.Width(width * 0.38f));
            GUILayout.EndHorizontal();
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
        if (champion.HasValue && GUILayout.Button("Crea una scorta con i 4 membri più vicini al campione", button))
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
            if (GUILayout.Button("−", button, GUILayout.Width(30))) SetSalary((string)r["id"], Mathf.Max(0, sal - 5));
            if (GUILayout.Button("+", button, GUILayout.Width(30))) SetSalary((string)r["id"], sal + 5);
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
    }

    void SquadOrder(long squad, JToken order) =>
        view.SendPlayerCommand(new JObject { ["type"] = "player_squad_order", ["squad"] = squad, ["order"] = order });

    void SetSalary(string rank, float amount) =>
        view.SendCommand(new JObject { ["type"] = "set_salary", ["faction"] = view.PlayerInfo["faction"], ["rank"] = rank, ["amount"] = amount });

    void Selected(float width)
    {
        Section("Pedina selezionata");
        var e = view.SelectedEntity;
        if (!view.Selected.HasValue)
        {
            GUILayout.Label("Clic sinistro: seleziona. Clic destro: muovi la tua pedina selezionata. Tasto centrale o WASD: sposta la vista. Rotella: zoom. C: campione.", small, GUILayout.Width(width));
            if (!string.IsNullOrEmpty(view.LastCommandResult)) GUILayout.Label(view.LastCommandResult, small, GUILayout.Width(width));
            return;
        }
        if (e == null)
        {
            GUILayout.Label("Caricamento…", small);
            return;
        }
        bool mine = view.IsMine((long)e["id"]);
        GUILayout.Label($"{e["name"]}  #{e["id"]}{(mine ? "  (tuo)" : "")}", title);
        var truth = e["truth"];
        if (truth != null && truth.Type != JTokenType.Null)
            GUILayout.Label($"Vera identità: {truth["name"]} ({truth["race"]}, {truth["faction"]}) · copertura {(float)truth["cover"]:0}", fake, GUILayout.Width(width));
        if (e["building"] != null && e["building"].Type != JTokenType.Null)
        {
            var b = e["building"];
            GUILayout.Label($"Edificio {b["def"]} · HP {(float)b["hp"]:0}/{(float)b["max_hp"]:0}", label);
            var stock = string.Join(", ", ((JObject)b["stock"]).Properties().Select(p => $"{p.Value}× {p.Name}"));
            GUILayout.Label("Magazzino: " + (stock.Length > 0 ? stock : "vuoto"), label, GUILayout.Width(width));
            return;
        }
        var classes = string.Join(", ", e["classes"].Select(c => (string)c));
        GUILayout.Label($"{e["race"]} · {classes}", label, GUILayout.Width(width));
        GUILayout.Label($"{FactionName((string)e["faction"])} · {e["rank"]}", small, GUILayout.Width(width));
        var act = e["activity"];
        GUILayout.Label($"{act?["label"]}  ({act?["mood"]})", label, GUILayout.Width(width));
        var r = GUILayoutUtility.GetRect(width, 8);
        GUI.DrawTexture(r, barTex);
        GUI.DrawTexture(new Rect(r.x, r.y, r.width * ((float?)act?["progress"] ?? 0f), r.height), barFillTex);
        string missing = e["missing_parts"] is JArray mp && mp.Count > 0 ? " · manca: " + string.Join(", ", mp.Select(x => (string)x)) : "";
        GUILayout.Label($"Salute {(float)e["health"] * 100:0}%{missing}", label, GUILayout.Width(width));
        GUILayout.Label($"Soldi {(float)e["money"]:0} € · Ricercato {(float)e["wanted"]:0.0} · Dissenso {(float)e["dissent"]:0}", label, GUILayout.Width(width));
        var needs = e["needs"] as JObject;
        if (needs != null) GUILayout.Label("Bisogni: " + string.Join(", ", needs.Properties().Select(p => $"{p.Name} {(float)p.Value * 100:0}%")), small, GUILayout.Width(width));
        var statuses = e["statuses"]?.Select(s => (string)s["name"] + (s["stage"]?.Type == JTokenType.String ? $" ({s["stage"]})" : "")).ToList();
        if (statuses?.Count > 0) GUILayout.Label("Status: " + string.Join(", ", statuses), label, GUILayout.Width(width));
        var inv = e["inventory"]?.Select(i => $"{i[1]}× {i[0]}").ToList();
        if (inv?.Count > 0) GUILayout.Label("Inventario: " + string.Join(", ", inv), label, GUILayout.Width(width));

        if (mine && view.SelectedActions != null)
        {
            var m = view.Member((long)e["id"]);
            bool champ = (bool?)m?["champion"] ?? false;
            GUILayout.Label(champ ? "Ordini (il campione obbedisce sempre):" : $"Ordini (obbedienza {((float?)m?["obedience"] ?? 0) * 100:0}%):", title);
            int col = 0;
            GUILayout.BeginHorizontal();
            foreach (var a in view.SelectedActions)
            {
                if (GUILayout.Button((string)a["name"], button, GUILayout.Width(width / 2f - 4)))
                {
                    if ((string)a["needs_target"] == "none")
                        view.SendPlayerOrder((long)e["id"], SimView.OrderFor((string)a["kind"], (string)a["id"], null));
                    else
                        view.PendingAction = (JObject)a;
                }
                if (++col % 2 == 0)
                {
                    GUILayout.EndHorizontal();
                    GUILayout.BeginHorizontal();
                }
            }
            GUILayout.EndHorizontal();
        }
        else
        {
            var ai = view.SelectedAi?["scores"];
            if (ai != null)
            {
                GUILayout.Label("Utility AI:", small);
                foreach (var s in ai.Take(5))
                    GUILayout.Label($"  {s["action"]}  {(float)s["score"]:0.00}{((float)s["momentum"] > 0 ? " (+momentum)" : "")}", small);
            }
        }

        bool detained = act?["flags"]?.Any(f => (string)f == "detained") ?? false;
        if (detained && view.PlayerInfo != null)
        {
            string payer = (string)view.PlayerInfo["faction"];
            if (GUILayout.Button($"Paga la tangente ({FactionName(payer)})", button))
                view.SendCommand(new JObject { ["type"] = "bribe", ["faction"] = payer, ["target"] = e["id"] });
        }
        if (!string.IsNullOrEmpty(view.LastCommandResult)) GUILayout.Label(view.LastCommandResult, small, GUILayout.Width(width));
    }

    void Update()
    {
        if (Input.GetKeyDown(KeyCode.Tab)) panelHidden = !panelHidden;
    }

    /// Place name shown for a moment when entering a map, as in the handheld games.
    void LocationBanner()
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
        if (depth < 0) name += $"  ·  livello {depth}";
        if (name != bannerKey)
        {
            bannerKey = name;
            bannerUntil = Time.time + 3f;
        }
        if (Time.time > bannerUntil || view.Zoom < 1f) return;
        GUI.Label(new Rect(14, 46, Mathf.Max(220, name.Length * 9 + 40), 40), name, banner);
    }

    /// Text box at the bottom with the newest headline of the chosen feed filter.
    void Notice(float areaW, float h)
    {
        var a = (view.Feed?["articles"] as JArray)?.FirstOrDefault();
        if (a != null)
        {
            string key = $"{a["tick"]}|{a["headline"]}";
            if (key != noticeKey)
            {
                // The first headline after start is not news: no box until something new happens.
                if (noticeKey != null) noticeUntil = Time.time + 9f;
                noticeKey = key;
                noticeText = $"{a["author_name"]}: {a["headline"]}";
            }
        }
        if (noticeText == null || Time.time > noticeUntil) return;
        var r = new Rect(16, h - 96, Mathf.Min(760, areaW - 32), 80);
        GUI.Label(r, noticeText, textBox);
        if (Mathf.Repeat(Time.time, 1f) < 0.6f) GUI.Label(new Rect(r.xMax - 30, r.yMax - 28, 20, 20), "▼", label);
        if (Event.current.type == EventType.MouseDown && r.Contains(Event.current.mousePosition)) noticeUntil = 0;
    }

    void ZoneLabels()
    {
        foreach (var z in view.Map["zones"].Where(z => (int)z["layer"] == view.Layer))
        {
            var sp = view.WorldToScreen(new Vector3((int)z["x"] + 0.3f, -(int)z["y"] - 0.2f, 0));
            var p = new Vector2(sp.x, Screen.height - sp.y) / scale;
            GUI.Label(new Rect(p.x, p.y, 240, 20), (string)z["name"], zoneLabel);
        }
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
