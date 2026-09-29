using System.Collections.Generic;
using System.Linq;
using Newtonsoft.Json.Linq;
using UnityEngine;

/// <summary>
/// Immediate-mode HUD: top bar (time, pause, speed, layer, fog), right panel (selected pawn with its
/// Utility AI, feed, chronicle, market, factions) and zone labels on the map.
/// </summary>
[RequireComponent(typeof(SimView))]
public class SimHud : MonoBehaviour
{
    SimView view;
    float scale = 1f;
    Vector2 scrollRight;
    Rect topRect, rightRect;
    GUIStyle box, title, small, label, zoneLabel, button, fake, hover;
    Texture2D panelTex, barTex, barFillTex;
    int speedIndex = 1;
    static readonly (string name, int ms)[] Speeds = { ("Lenta", 1000), ("Normale", 400), ("Veloce", 120), ("Turbo", 30) };

    void Awake() => view = GetComponent<SimView>();

    public bool IsOverUi(Vector3 mouse)
    {
        // Input.mousePosition has y up, GUI has y down.
        var p = new Vector2(mouse.x, Screen.height - mouse.y) / scale;
        return topRect.Contains(p) || rightRect.Contains(p);
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

        topRect = new Rect(0, 0, w, 34);
        GUILayout.BeginArea(topRect, box);
        GUILayout.BeginHorizontal();
        TopBar();
        GUILayout.EndHorizontal();
        GUILayout.EndArea();

        float pw = Mathf.Min(380, w * 0.4f);
        rightRect = new Rect(w - pw, 34, pw, h - 34);
        GUILayout.BeginArea(rightRect, box);
        scrollRight = GUILayout.BeginScrollView(scrollRight);
        RightPanel(pw - 30);
        GUILayout.EndScrollView();
        GUILayout.EndArea();
    }

    void TopBar()
    {
        GUILayout.Label("Fidenza & Salsomaggiore", title, GUILayout.Width(190));
        if (view.Error != null)
        {
            GUILayout.Label(view.Error, fake);
            return;
        }
        long tick = (long?)view.State?["snapshot"]?["tick"] ?? 0;
        GUILayout.Label($"tick {tick} · giorno {tick / 24 + 1}, ore {tick % 24}:00", label, GUILayout.Width(170));
        bool paused = (bool?)view.Control?["paused"] ?? false;
        if (GUILayout.Button(paused ? "Riprendi" : "Pausa", button, GUILayout.Width(80)))
            view.SendControl(new JObject { ["paused"] = !paused });
        if (GUILayout.Button("+1 tick", button, GUILayout.Width(60)))
            view.SendControl(new JObject { ["step"] = 1 });
        if (GUILayout.Button("Velocità: " + Speeds[speedIndex].name, button, GUILayout.Width(120)))
        {
            speedIndex = (speedIndex + 1) % Speeds.Length;
            view.SendControl(new JObject { ["tick_ms"] = Speeds[speedIndex].ms });
        }
        if (view.Layers != null)
            for (int i = 0; i < view.Layers.Count; i++)
            {
                var style = new GUIStyle(button) { fontStyle = i == view.Layer ? FontStyle.Bold : FontStyle.Normal };
                if (GUILayout.Button((string)view.Layers[i]["name"], style)) view.SetLayer(i);
            }
        var factions = Factions();
        string fogName = string.IsNullOrEmpty(view.FogFaction) ? "nessuna (admin)" : FactionName(view.FogFaction);
        if (GUILayout.Button("Nebbia: " + fogName, button, GUILayout.Width(230)) && factions.Count > 0)
        {
            var ids = new List<string> { "" };
            ids.AddRange(factions.Select(f => (string)f["id"]));
            int i = ids.IndexOf(view.FogFaction);
            view.FogFaction = ids[(i + 1) % ids.Count];
        }
        GUILayout.FlexibleSpace();
    }

    List<JToken> Factions() => view.State?["snapshot"]?["factions"]?.ToList() ?? new List<JToken>();

    string FactionName(string id) => (string)Factions().FirstOrDefault(f => (string)f["id"] == id)?["name"] ?? id;

    void Section(string text)
    {
        GUILayout.Space(8);
        GUILayout.Label(text.ToUpperInvariant(), title);
    }

    void RightPanel(float width)
    {
        if (view.State == null)
        {
            GUILayout.Label("In attesa del server…\nAvvia: cd engine && cargo run --release -p fidenza_world -- --serve", label);
            return;
        }
        Selected(width);
        var snap = view.State["snapshot"];

        Section((string)snap["feed_name"] ?? "Feed");
        foreach (var a in snap["feed"].Take(6))
        {
            bool isFake = (string)a["truth"] == "Fake";
            GUILayout.Label($"[{a["tick"]}] {(isFake ? "FAKE · " : "")}{a["headline"]}", isFake ? fake : label, GUILayout.Width(width));
            GUILayout.Label("— " + a["author_name"], small);
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
            GUILayout.Label($"{d:+0;-0}%", d > 0 ? fake : label);
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

    void Selected(float width)
    {
        Section("Pedina selezionata");
        var e = view.SelectedEntity;
        if (!view.Selected.HasValue)
        {
            GUILayout.Label("Clic sinistro su una pedina. Tasto destro o centrale per spostare la vista, rotella per lo zoom.", small, GUILayout.Width(width));
            return;
        }
        if (e == null)
        {
            GUILayout.Label("Caricamento…", small);
            return;
        }
        GUILayout.Label($"{e["name"]}  #{e["id"]}", title);
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
        var statuses = e["statuses"]?.Select(s => (string)s["name"] + (s["stage"]?.Type == JTokenType.String ? $" ({s["stage"]})" : "")).ToList();
        if (statuses?.Count > 0) GUILayout.Label("Status: " + string.Join(", ", statuses), label, GUILayout.Width(width));
        var inv = e["inventory"]?.Select(i => $"{i[1]}× {i[0]}").ToList();
        if (inv?.Count > 0) GUILayout.Label("Inventario: " + string.Join(", ", inv), label, GUILayout.Width(width));

        var ai = view.SelectedAi?["scores"];
        if (ai != null)
        {
            GUILayout.Label("Utility AI:", small);
            foreach (var s in ai.Take(5))
                GUILayout.Label($"  {s["action"]}  {(float)s["score"]:0.00}{((float)s["momentum"] > 0 ? " (+momentum)" : "")}", small);
        }

        bool detained = act?["flags"]?.Any(f => (string)f == "detained") ?? false;
        if (detained)
        {
            string payer = !string.IsNullOrEmpty(view.FogFaction) ? view.FogFaction : (string)e["faction"];
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
