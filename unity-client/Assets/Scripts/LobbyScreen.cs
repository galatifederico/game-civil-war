using UnityEngine;

// Lobby: l'elenco dei mondi a cui ci si puo' unire (design.md: si entra da una lista, non con codici).
// Si puo' anche creare un mondo nuovo, di cui si diventa admin.
public class LobbyScreen : MonoBehaviour
{
    const float Width = 520f;

    NetworkClient net;
    WorldEntry[] worlds = new WorldEntry[0];
    Vector2 scroll;
    readonly System.Collections.Generic.Dictionary<string, int> chosenRace = new System.Collections.Generic.Dictionary<string, int>();
    bool busy;
    string message = "", newWorldName = "";
    GUIStyle boxStyle, titleStyle, nameStyle, infoStyle, bodyStyle, buttonStyle, fieldStyle, messageStyle;

    public bool Visible { get; private set; }

    public event System.Action LoggedOut;

    public void Init(NetworkClient network) => net = network;

    public void Open(string info)
    {
        Visible = true;
        message = info;
        Refresh();
    }

    public void Hide()
    {
        Visible = false;
        busy = false;
    }

    void Refresh()
    {
        busy = true;
        net.FetchWorlds(list =>
        {
            worlds = list;
            busy = false;
        }, error =>
        {
            message = error;
            busy = false;
        });
    }

    void OnGUI()
    {
        if (!Visible) return;
        EnsureStyles();

        float height = Mathf.Min(Screen.height - 40f, 520f);
        var rect = new Rect((Screen.width - Width) / 2f, (Screen.height - height) / 2f, Width, height);
        GUI.Box(rect, GUIContent.none, boxStyle);

        GUILayout.BeginArea(new Rect(rect.x + 20f, rect.y + 16f, Width - 40f, height - 32f));
        GUILayout.Label($"Mondi - {net.Username}", titleStyle);
        GUILayout.Space(6f);

        scroll = GUILayout.BeginScrollView(scroll, GUILayout.Height(height - 190f));
        if (worlds.Length == 0) GUILayout.Label(busy ? "Carico i mondi..." : "Nessun mondo: creane uno.", infoStyle);
        foreach (var w in worlds)
        {
            GUILayout.BeginVertical(GUI.skin.box);
            GUILayout.BeginHorizontal();
            GUILayout.Label(w.name + (w.admin ? "  (admin)" : ""), nameStyle);
            GUILayout.FlexibleSpace();
            GUI.enabled = !busy;
            var races = w.races ?? new RaceEntry[0];
            chosenRace.TryGetValue(w.id, out int pick);
            if (GUILayout.Button(w.joined ? "Gioca" : "Unisciti", buttonStyle, GUILayout.Width(110f)))
            {
                busy = true;
                message = "";
                net.JoinAndEnter(w.id, races.Length > 0 ? races[Mathf.Clamp(pick, 0, races.Length - 1)].id : "", error =>
                {
                    message = error;
                    busy = false;
                });
            }
            GUI.enabled = true;
            GUILayout.EndHorizontal();
            GUILayout.Label($"{w.players} giocatori  ·  {w.boards} board", infoStyle);
            if (!string.IsNullOrEmpty(w.description)) GUILayout.Label(w.description, bodyStyle);
            // La razza si sceglie una volta sola, la prima volta che ci si unisce a un mondo che ne ha.
            if (!w.joined && races.Length > 0)
            {
                var names = new string[races.Length];
                for (int i = 0; i < names.Length; i++) names[i] = races[i].name;
                pick = GUILayout.Toolbar(Mathf.Clamp(pick, 0, races.Length - 1), names, buttonStyle);
                chosenRace[w.id] = pick;
                GUILayout.Label(races[pick].description, infoStyle);
            }
            GUILayout.EndVertical();
        }
        GUILayout.EndScrollView();

        GUILayout.Space(6f);
        GUILayout.BeginHorizontal();
        newWorldName = GUILayout.TextField(newWorldName, 40, fieldStyle);
        GUI.enabled = !busy && newWorldName.Trim().Length >= 2;
        if (GUILayout.Button("Nuovo mondo", buttonStyle, GUILayout.Width(130f)))
        {
            busy = true;
            net.CreateWorld(newWorldName.Trim(), "", _ =>
            {
                newWorldName = "";
                Refresh();
            }, error =>
            {
                message = error;
                busy = false;
            });
        }
        GUI.enabled = true;
        GUILayout.EndHorizontal();

        GUILayout.BeginHorizontal();
        GUI.enabled = !busy;
        if (GUILayout.Button("Aggiorna", buttonStyle)) Refresh();
        GUI.enabled = true;
        if (GUILayout.Button("Esci dall'account", buttonStyle)) LoggedOut?.Invoke();
        GUILayout.EndHorizontal();

        if (!string.IsNullOrEmpty(message)) GUILayout.Label(message, messageStyle);
        GUILayout.EndArea();
    }

    void EnsureStyles()
    {
        if (boxStyle != null) return;

        var bg = new Texture2D(1, 1) { hideFlags = HideFlags.HideAndDontSave };
        bg.SetPixel(0, 0, new Color(0.08f, 0.09f, 0.12f, 0.97f));
        bg.Apply();
        boxStyle = new GUIStyle(GUI.skin.box);
        boxStyle.normal.background = bg;

        titleStyle = new GUIStyle(GUI.skin.label) { fontSize = 22, fontStyle = FontStyle.Bold };
        titleStyle.normal.textColor = Color.white;
        nameStyle = new GUIStyle(GUI.skin.label) { fontSize = 17, fontStyle = FontStyle.Bold };
        nameStyle.normal.textColor = new Color(1f, 0.85f, 0.4f);
        infoStyle = new GUIStyle(GUI.skin.label) { fontSize = 12 };
        infoStyle.normal.textColor = new Color(0.7f, 0.75f, 0.85f);
        bodyStyle = new GUIStyle(GUI.skin.label) { fontSize = 13, wordWrap = true };
        bodyStyle.normal.textColor = new Color(0.9f, 0.9f, 0.9f);
        buttonStyle = new GUIStyle(GUI.skin.button) { fontSize = 14, fixedHeight = 28f };
        fieldStyle = new GUIStyle(GUI.skin.textField) { fontSize = 14, fixedHeight = 28f };
        messageStyle = new GUIStyle(GUI.skin.label) { fontSize = 13, wordWrap = true };
        messageStyle.normal.textColor = new Color(1f, 0.55f, 0.5f);
    }
}
