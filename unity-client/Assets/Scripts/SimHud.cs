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
    GUIStyle box, title, small, label, zoneLabel, button, fake, hover, good;
    Texture2D panelTex, barTex, barFillTex;
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
        return topRect.Contains(p) || rightRect.Contains(p) || (mapMenu && mapRect.Contains(p));
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
        panelTex = Tex(new Color(0.13f, 0.11f, 0.09f, 0.92f));
        barTex = Tex(new Color(1, 1, 1, 0.15f));
        barFillTex = Tex(new Color(0.4f, 0.8f, 0.5f));
        box = new GUIStyle(GUI.skin.box) { normal = { background = panelTex }, padding = new RectOffset(10, 10, 8, 8), alignment = TextAnchor.UpperLeft };
        title = new GUIStyle(GUI.skin.label) { fontStyle = FontStyle.Bold, fontSize = 13, normal = { textColor = new Color(0.95f, 0.8f, 0.6f) } };
        label = new GUIStyle(GUI.skin.label) { wordWrap = true, fontSize = 12, normal = { textColor = new Color(0.93f, 0.9f, 0.84f) } };
        small = new GUIStyle(label) { fontSize = 11, normal = { textColor = new Color(0.7f, 0.65f, 0.58f) } };
        fake = new GUIStyle(label) { normal = { textColor = new Color(1f, 0.55f, 0.45f) } };
        good = new GUIStyle(label) { normal = { textColor = new Color(0.55f, 0.9f, 0.6f) } };
        zoneLabel = new GUIStyle(GUI.skin.label) { fontSize = 11, fontStyle = FontStyle.Bold, normal = { textColor = new Color(0.25f, 0.2f, 0.15f, 0.85f) } };
        hover = new GUIStyle(box) { fontSize = 12, wordWrap = true, normal = { background = panelTex, textColor = Color.white } };
        button = new GUIStyle(GUI.skin.button) { fontSize = 12 };
    }

    void OnGUI()
    {
        InitStyles();
        scale = Mathf.Max(1f, Screen.height / 900f);
        GUI.matrix = Matrix4x4.Scale(new Vector3(scale, scale, 1));
        float w = Screen.width / scale, h = Screen.height / scale;

        if (view.Map != null) ZoneLabels();
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

        float pw = Mathf.Min(400, w * 0.42f);
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
        view.FollowCamera = GUILayout.Toggle(view.FollowCamera, "Segui", GUILayout.Width(60));
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
