using System.Collections.Generic;
using System.Text;
using UnityEngine;

// Informazioni a schermo: stato, classifica, inventario, errori brevi, notifiche e dialoghi.
public class Hud : MonoBehaviour
{
    const float ToastSeconds = 3f;
    const float EventSeconds = 8f;
    const int MaxToasts = 4;
    const int MaxScoresShown = 5;
    static readonly Rect EventRectTemplate = new Rect(12f, 0f, 290f, 150f);

    struct Toast
    {
        public string Text;
        public float Until;
    }

    readonly List<Toast> toasts = new List<Toast>();
    string status = "", hint = "", eventTitle = "", eventText = "", inventoryText = "";
    ScoreData[] scores = new ScoreData[0];
    GoalData[] goals = new GoalData[0];
    string myPlayerId;
    float eventUntil;
    Rect eventRect;
    GUIStyle goalStyle, doneGoalStyle, buttonStyle, statusStyle, toastStyle, scoreStyle, mineScoreStyle, hintStyle, boxStyle, eventTitleStyle, eventTextStyle;

    public event System.Action LogoutClicked;

    string[] boardNames = new string[0];
    int currentBoard, pendingBoard = -1;
    System.Action<int> onBoardSelected;
    Rect tabsRect;

    bool logoutVisible;
    Rect logoutRect;

    public void SetStatus(string text) => status = text;

    public void SetLogoutVisible(bool visible) => logoutVisible = visible;

    // Schede per passare da una board all'altra (solo la vista: le pedine non si spostano).
    public void SetBoards(string[] names, int current, System.Action<int> onSelect)
    {
        boardNames = names;
        currentBoard = current;
        onBoardSelected = onSelect;
    }

    // Si esegue fuori da OnGUI: cambiare board ricostruisce la scena e rifa' le schede.
    void Update()
    {
        if (pendingBoard < 0) return;
        int i = pendingBoard;
        pendingBoard = -1;
        onBoardSelected?.Invoke(i);
    }

    public void SetHint(string text) => hint = text;

    public void ShowToast(string text)
    {
        toasts.Add(new Toast { Text = text, Until = Time.time + ToastSeconds });
        if (toasts.Count > MaxToasts) toasts.RemoveAt(0);
    }

    // Dialoghi con gli NPC e notifiche del server (raccolta, sconfitta...).
    public void ShowEvent(string title, string text)
    {
        eventTitle = title;
        eventText = text;
        eventUntil = Time.time + EventSeconds;
    }

    public void SetScores(ScoreData[] newScores, string playerId)
    {
        scores = newScores;
        myPlayerId = playerId;
    }

    public void SetGoals(GoalData[] newGoals) => goals = newGoals ?? new GoalData[0];

    public void SetInventory(ItemData[] items)
    {
        if (items == null || items.Length == 0)
        {
            inventoryText = "";
            return;
        }
        var sb = new StringBuilder($"Inventario ({items.Length}): ");
        for (int i = 0; i < items.Length; i++) sb.Append(i > 0 ? ", " : "").Append(items[i].name);
        inventoryText = sb.ToString();
    }

    public void Clear()
    {
        status = hint = eventTitle = eventText = inventoryText = "";
        logoutVisible = false;
        boardNames = new string[0];
        scores = new ScoreData[0];
        goals = new GoalData[0];
        eventUntil = 0f;
        toasts.Clear();
    }

    // Come per il pannello: OnMouseDown scatta anche sotto i controlli IMGUI.
    public bool BlocksPointer
    {
        get
        {
            var m = Input.mousePosition;
            var p = new Vector2(m.x, Screen.height - m.y);
            if (logoutVisible && logoutRect.Contains(p)) return true;
            if (boardNames.Length > 1 && tabsRect.Contains(p)) return true;
            return Time.time < eventUntil && eventRect.Contains(p);
        }
    }

    void OnGUI()
    {
        EnsureStyles();
        toasts.RemoveAll(t => t.Until < Time.time);

        float y = 10f;
        if (!string.IsNullOrEmpty(status))
        {
            GUI.Label(new Rect(12f, y, 420f, 24f), status, statusStyle);
            y += 28f;
        }
        for (int i = 0; i < Mathf.Min(scores.Length, MaxScoresShown); i++)
        {
            var s = scores[i];
            bool mine = s.player_id == myPlayerId;
            GUI.Label(new Rect(12f, y, 420f, 20f), $"{i + 1}. {s.username}{(mine ? " (tu)" : "")}  {s.points} pt", mine ? mineScoreStyle : scoreStyle);
            y += 20f;
        }
        if (!string.IsNullOrEmpty(inventoryText))
        {
            y += 6f;
            GUI.Label(new Rect(12f, y, 420f, 22f), inventoryText, scoreStyle);
            y += 24f;
        }
        if (goals.Length > 0)
        {
            y += 6f;
            foreach (var g in goals)
            {
                string line;
                if (g.scope == "world")
                    line = g.completed ? $"Mondo: {g.title} - vinto da {g.achieved_by}" : $"Mondo: {g.title} {g.progress}/{g.target}";
                else
                    line = g.completed ? $"[fatto] {g.title}" : $"Obiettivo: {g.title} {g.progress}/{g.target} (+{g.reward} pt)";
                GUI.Label(new Rect(12f, y, 420f, 20f), line, g.completed ? doneGoalStyle : goalStyle);
                y += 20f;
            }
        }
        y += 6f;
        foreach (var toast in toasts)
        {
            GUI.Label(new Rect(12f, y, 420f, 24f), toast.Text, toastStyle);
            y += 26f;
        }

        if (logoutVisible)
        {
            logoutRect = new Rect(Screen.width - 92f, Screen.height - 42f, 80f, 30f);
            if (GUI.Button(logoutRect, "Esci", buttonStyle)) LogoutClicked?.Invoke();
        }

        if (boardNames.Length > 1)
        {
            const float tabWidth = 110f, tabHeight = 28f;
            float total = boardNames.Length * tabWidth;
            tabsRect = new Rect(Screen.width / 2f - total / 2f, 8f, total, tabHeight);
            for (int i = 0; i < boardNames.Length; i++)
            {
                var previous = GUI.backgroundColor;
                GUI.backgroundColor = i == currentBoard ? new Color(1f, 0.85f, 0.4f) : Color.white;
                if (GUI.Button(new Rect(tabsRect.x + i * tabWidth, tabsRect.y, tabWidth - 4f, tabHeight), boardNames[i], buttonStyle) && i != currentBoard)
                    pendingBoard = i;
                GUI.backgroundColor = previous;
            }
        }

        if (!string.IsNullOrEmpty(hint))
            GUI.Label(new Rect(Screen.width / 2f - 220f, boardNames.Length > 1 ? 42f : 8f, 440f, 26f), hint, hintStyle);

        if (Time.time < eventUntil)
        {
            eventRect = new Rect(EventRectTemplate.x, Screen.height - EventRectTemplate.height - 12f, EventRectTemplate.width, EventRectTemplate.height);
            GUI.Box(eventRect, GUIContent.none, boxStyle);
            GUI.Label(new Rect(eventRect.x + 12f, eventRect.y + 8f, eventRect.width - 24f, 24f), eventTitle, eventTitleStyle);
            GUI.Label(new Rect(eventRect.x + 12f, eventRect.y + 36f, eventRect.width - 24f, eventRect.height - 44f), eventText, eventTextStyle);
        }
    }

    void EnsureStyles()
    {
        if (statusStyle != null) return;
        buttonStyle = new GUIStyle(GUI.skin.button) { fontSize = 13 };
        statusStyle = new GUIStyle(GUI.skin.label) { fontSize = 14, fontStyle = FontStyle.Bold };
        statusStyle.normal.textColor = new Color(0.85f, 0.9f, 1f);
        scoreStyle = new GUIStyle(GUI.skin.label) { fontSize = 13 };
        scoreStyle.normal.textColor = new Color(0.75f, 0.8f, 0.9f);
        mineScoreStyle = new GUIStyle(scoreStyle) { fontStyle = FontStyle.Bold };
        mineScoreStyle.normal.textColor = new Color(1f, 0.85f, 0.4f);
        goalStyle = new GUIStyle(GUI.skin.label) { fontSize = 13 };
        goalStyle.normal.textColor = new Color(0.65f, 0.9f, 0.75f);
        doneGoalStyle = new GUIStyle(goalStyle);
        doneGoalStyle.normal.textColor = new Color(0.5f, 0.55f, 0.6f);
        toastStyle = new GUIStyle(GUI.skin.label) { fontSize = 14 };
        toastStyle.normal.textColor = new Color(1f, 0.55f, 0.5f);
        hintStyle = new GUIStyle(GUI.skin.label) { fontSize = 15, fontStyle = FontStyle.Bold, alignment = TextAnchor.MiddleCenter };
        hintStyle.normal.textColor = new Color(0.5f, 0.9f, 1f);

        var bg = new Texture2D(1, 1) { hideFlags = HideFlags.HideAndDontSave };
        bg.SetPixel(0, 0, new Color(0.08f, 0.09f, 0.12f, 0.94f));
        bg.Apply();
        boxStyle = new GUIStyle(GUI.skin.box);
        boxStyle.normal.background = bg;
        eventTitleStyle = new GUIStyle(GUI.skin.label) { fontSize = 16, fontStyle = FontStyle.Bold };
        eventTitleStyle.normal.textColor = new Color(1f, 0.85f, 0.4f);
        eventTextStyle = new GUIStyle(GUI.skin.label) { fontSize = 15, wordWrap = true };
        eventTextStyle.normal.textColor = new Color(0.92f, 0.92f, 0.92f);
    }
}
