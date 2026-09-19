using System.Collections.Generic;
using UnityEngine;

// L'interfaccia sopra la mappa. Il menu e' uno solo, in sovrapposizione sulla sinistra, e cambia
// contenuto a seconda di cio' che succede:
//   - scheda della pedina o dell'oggetto cliccato (con le sue azioni), se c'e';
//   - altrimenti l'inventario, se e' stato aperto;
//   - altrimenti la squadra: classifica, obiettivi ed "Esci".
// Sopra a tutto compaiono i messaggi (dialoghi con gli NPC, sconfitte...). Fuori dal menu restano
// solo le schede delle board, il suggerimento dell'azione in corso, i messaggi d'errore e i
// pulsanti dello zoom, cosi' la mappa occupa tutto lo schermo.
public class Hud : MonoBehaviour
{
    const float ToastSeconds = 3f;
    const float EventSeconds = 8f;
    const int MaxToasts = 4;
    const int MaxScoresShown = 8;
    const float MenuWidth = 330f;
    const float Margin = 10f;

    enum Tab { Home, Inventory }

    struct Toast
    {
        public string Text;
        public float Until;
    }

    // Oggetti uguali (stesso nome, effetto e icona) si mostrano come una riga sola con la quantita'.
    class Stack
    {
        public ItemData First;
        public int Count;
    }

    readonly List<Toast> toasts = new List<Toast>();
    readonly List<Stack> stacks = new List<Stack>();
    string status = "", hint = "", eventTitle = "", eventText = "";
    int inventoryCount;
    ScoreData[] scores = new ScoreData[0];
    GoalData[] goals = new GoalData[0];
    string myPlayerId;
    float eventUntil;

    InfoPanel card;
    Tab tab = Tab.Home;
    bool menuOpen = true;
    Piece lastCardPiece;
    bool lastCompact;
    Vector2 scroll;
    Rect menuRect, toggleRect, tabsRect, zoomRect, eventRect;
    Stack pendingUse;
    int pendingBoard = -1;
    string[] boardNames = new string[0];
    int currentBoard;
    System.Action<int> onBoardSelected;
    bool inWorld;
    Texture2D boxTexture;

    GUIStyle goalStyle, doneGoalStyle, buttonStyle, statusStyle, toastStyle, scoreStyle, mineScoreStyle, hintStyle, boxStyle,
        eventTitleStyle, eventTextStyle, headingStyle, itemNameStyle, itemInfoStyle, quantityStyle, mutedStyle;

    public event System.Action LogoutClicked;
    public event System.Action<ItemData> ItemUseRequested;
    public event System.Action<float> ZoomRequested; // fattore: < 1 avvicina, > 1 allontana, 0 = adatta

    public void Init(InfoPanel infoPanel)
    {
        card = infoPanel;
        lastCompact = Ui.Compact;
        menuOpen = !lastCompact;
    }

    public void SetStatus(string text) => status = text;

    public void SetLogoutVisible(bool visible) => inWorld = visible;

    // Larghezza, in pixel di schermo, che il menu copre a sinistra: la mappa si inquadra nel resto.
    public float OccupiedLeftPixels => menuOpen && !Ui.Compact ? (MenuWidth + 2f * Margin) * Ui.Scale : 0f;

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
        if (pendingUse != null)
        {
            var use = pendingUse;
            pendingUse = null;
            ItemUseRequested?.Invoke(use.First);
        }
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
        stacks.Clear();
        inventoryCount = items?.Length ?? 0;
        foreach (var item in items ?? new ItemData[0])
        {
            var stack = stacks.Find(s => s.First.name == item.name && s.First.effect == item.effect
                && s.First.icon == item.icon && s.First.description == item.description);
            if (stack == null) stacks.Add(stack = new Stack { First = item });
            stack.Count++;
        }
    }

    public void Clear()
    {
        status = hint = eventTitle = eventText = "";
        inWorld = false;
        boardNames = new string[0];
        scores = new ScoreData[0];
        goals = new GoalData[0];
        stacks.Clear();
        inventoryCount = 0;
        eventUntil = 0f;
        toasts.Clear();
        tab = Tab.Home;
        menuOpen = !Ui.Compact;
        lastCardPiece = null;
    }

    // OnMouseDown scatta anche sotto i controlli IMGUI: la mappa ignora i clic che cadono su di essi.
    public bool BlocksPointer
    {
        get
        {
            var p = Ui.Pointer;
            if (menuOpen ? menuRect.Contains(p) : toggleRect.Contains(p)) return true;
            if (inWorld && zoomRect.Contains(p)) return true;
            if (boardNames.Length > 1 && tabsRect.Contains(p)) return true;
            return !menuOpen && Time.time < eventUntil && eventRect.Contains(p);
        }
    }

    void OnGUI()
    {
        Ui.Begin();
        EnsureStyles();
        toasts.RemoveAll(t => t.Until < Time.time);
        if (!inWorld) return;

        // Una pedina appena cliccata apre il menu, anche se era chiuso.
        var shown = card != null ? card.Current : null;
        if (shown != null && shown != lastCardPiece) menuOpen = true;
        lastCardPiece = shown;

        bool compact = Ui.Compact;
        if (compact != lastCompact)
        {
            // Lo schermo e' stato girato: in verticale il menu parte chiuso, in orizzontale aperto.
            lastCompact = compact;
            menuOpen = !compact;
        }
        float menuRight = menuOpen ? Margin + Mathf.Min(MenuWidth, Ui.Width - 2f * Margin) : 0f;
        // In verticale il menu aperto copre quasi tutta la mappa: il resto si nasconde finche' non si chiude.
        bool overlayHidden = compact && menuOpen;
        float mapCenter = compact ? Ui.Width / 2f : (menuRight + Ui.Width) / 2f;

        if (!overlayHidden)
        {
            DrawBoardTabs(mapCenter);
            DrawHint(mapCenter);
            DrawToasts(mapCenter);
            DrawZoomButtons();
        }
        if (menuOpen) DrawMenu();
        else DrawCollapsed();
    }

    // ---- fuori dal menu ----------------------------------------------------------------------

    void DrawBoardTabs(float center)
    {
        if (boardNames.Length <= 1) return;
        const float tabHeight = 28f;
        float tabWidth = Mathf.Min(110f, (Ui.Width - 16f) / boardNames.Length);
        float total = boardNames.Length * tabWidth;
        tabsRect = new Rect(center - total / 2f, 8f, total, tabHeight);
        for (int i = 0; i < boardNames.Length; i++)
        {
            var previous = GUI.backgroundColor;
            GUI.backgroundColor = i == currentBoard ? new Color(1f, 0.85f, 0.4f) : Color.white;
            if (GUI.Button(new Rect(tabsRect.x + i * tabWidth, tabsRect.y, tabWidth - 4f, tabHeight), boardNames[i], buttonStyle) && i != currentBoard)
                pendingBoard = i;
            GUI.backgroundColor = previous;
        }
    }

    void DrawHint(float center)
    {
        if (string.IsNullOrEmpty(hint)) return;
        float width = Mathf.Min(440f, Ui.Width - 16f);
        GUI.Label(new Rect(center - width / 2f, boardNames.Length > 1 ? 42f : 8f, width, 26f), hint, hintStyle);
    }

    void DrawToasts(float center)
    {
        float width = Mathf.Min(480f, Ui.Width - 16f);
        float y = Ui.Height - 12f - toasts.Count * 26f;
        foreach (var toast in toasts)
        {
            GUI.Label(new Rect(center - width / 2f, y, width, 24f), toast.Text, toastStyle);
            y += 26f;
        }
    }

    void DrawZoomButtons()
    {
        const float size = 34f, gap = 6f;
        zoomRect = new Rect(Ui.Width - size - Margin, Ui.Height - 3f * size - 2f * gap - Margin, size, 3f * size + 2f * gap);
        if (GUI.Button(new Rect(zoomRect.x, zoomRect.y, size, size), "+", buttonStyle)) ZoomRequested?.Invoke(0.75f);
        if (GUI.Button(new Rect(zoomRect.x, zoomRect.y + size + gap, size, size), "-", buttonStyle)) ZoomRequested?.Invoke(1.33f);
        if (GUI.Button(new Rect(zoomRect.x, zoomRect.y + 2f * (size + gap), size, size), "[ ]", buttonStyle)) ZoomRequested?.Invoke(0f);
    }

    // ---- il menu -----------------------------------------------------------------------------

    void DrawCollapsed()
    {
        toggleRect = new Rect(Margin, Margin, 76f, 32f);
        if (GUI.Button(toggleRect, "Menu", buttonStyle)) menuOpen = true;
        if (Time.time < eventUntil)
        {
            eventRect = new Rect(Margin, toggleRect.yMax + 6f, Mathf.Min(MenuWidth, Ui.Width - 2f * Margin), 120f);
            DrawEvent(eventRect);
        }
    }

    void DrawMenu()
    {
        float width = Mathf.Min(MenuWidth, Ui.Width - 2f * Margin);
        menuRect = new Rect(Margin, Margin, width, Ui.Height - 2f * Margin);
        GUI.Box(menuRect, GUIContent.none, boxStyle);

        GUILayout.BeginArea(new Rect(menuRect.x + 10f, menuRect.y + 8f, width - 20f, menuRect.height - 16f));

        bool showingCard = card != null && card.Current != null;
        GUILayout.BeginHorizontal();
        if (TabButton("Squadra", !showingCard && tab == Tab.Home)) { card?.Hide(); tab = Tab.Home; }
        if (TabButton($"Inventario ({inventoryCount})", !showingCard && tab == Tab.Inventory)) { card?.Hide(); tab = Tab.Inventory; }
        if (GUILayout.Button("<<", buttonStyle, GUILayout.Width(36f))) menuOpen = false;
        GUILayout.EndHorizontal();
        GUILayout.Space(6f);

        if (Time.time < eventUntil)
        {
            eventRect = GUILayoutUtility.GetRect(width - 20f, 92f);
            DrawEvent(eventRect);
            GUILayout.Space(6f);
        }

        scroll = GUILayout.BeginScrollView(scroll);
        if (showingCard) card.DrawCard();
        else if (tab == Tab.Inventory) DrawInventory();
        else DrawHome();
        GUILayout.EndScrollView();

        GUILayout.EndArea();
    }

    bool TabButton(string label, bool active)
    {
        var previous = GUI.backgroundColor;
        GUI.backgroundColor = active ? new Color(1f, 0.85f, 0.4f) : Color.white;
        bool clicked = GUILayout.Button(label, buttonStyle);
        GUI.backgroundColor = previous;
        return clicked;
    }

    void DrawEvent(Rect r)
    {
        GUI.Box(r, GUIContent.none, boxStyle);
        GUI.Label(new Rect(r.x + 10f, r.y + 6f, r.width - 20f, 22f), eventTitle, eventTitleStyle);
        GUI.Label(new Rect(r.x + 10f, r.y + 30f, r.width - 20f, r.height - 34f), eventText, eventTextStyle);
    }

    void DrawHome()
    {
        if (!string.IsNullOrEmpty(status)) GUILayout.Label(status, statusStyle);

        GUILayout.Space(6f);
        GUILayout.Label("Classifica", headingStyle);
        for (int i = 0; i < Mathf.Min(scores.Length, MaxScoresShown); i++)
        {
            var s = scores[i];
            bool mine = s.player_id == myPlayerId;
            GUILayout.Label($"{i + 1}. {s.username}{(mine ? " (tu)" : "")}{(s.afk ? " (assente)" : "")}  {s.points} pt", mine ? mineScoreStyle : scoreStyle);
        }

        if (goals.Length > 0)
        {
            GUILayout.Space(10f);
            GUILayout.Label("Obiettivi", headingStyle);
            foreach (var g in goals)
            {
                string line;
                if (g.scope == "world")
                    line = g.completed ? $"Mondo: {g.title} - vinto da {g.achieved_by}" : $"Mondo: {g.title} {g.progress}/{g.target}";
                else
                    line = g.completed ? $"[fatto] {g.title}" : $"Obiettivo: {g.title} {g.progress}/{g.target} (+{g.reward} pt)";
                GUILayout.Label(line, g.completed ? doneGoalStyle : goalStyle);
            }
        }

        GUILayout.Space(14f);
        if (GUILayout.Button("Esci dal mondo", buttonStyle)) LogoutClicked?.Invoke();
    }

    void DrawInventory()
    {
        GUILayout.Label("Inventario di squadra", headingStyle);
        if (stacks.Count == 0)
        {
            GUILayout.Label("Vuoto. Raccogli gli oggetti sulla mappa: il campione poi li usa da qui.", mutedStyle);
            return;
        }
        foreach (var stack in stacks)
        {
            var item = stack.First;
            GUILayout.BeginHorizontal(GUI.skin.box);

            var iconRect = GUILayoutUtility.GetRect(40f, 40f, GUILayout.Width(40f), GUILayout.Height(40f));
            GUI.DrawTexture(iconRect, PixelArt.ItemIcon(item.icon), ScaleMode.ScaleToFit);

            GUILayout.BeginVertical();
            GUILayout.Label(item.name, itemNameStyle);
            if (!string.IsNullOrEmpty(item.effect)) GUILayout.Label(item.effect, itemInfoStyle);
            if (!string.IsNullOrEmpty(item.description)) GUILayout.Label(item.description, mutedStyle);
            GUILayout.EndVertical();

            GUILayout.FlexibleSpace();
            GUILayout.BeginVertical(GUILayout.Width(48f));
            GUILayout.Label($"x{stack.Count}", quantityStyle);
            if (!string.IsNullOrEmpty(item.effect) && GUILayout.Button("Usa", buttonStyle)) pendingUse = stack;
            GUILayout.EndVertical();

            GUILayout.EndHorizontal();
        }
        GUILayout.Space(6f);
        GUILayout.Label("Gli oggetti si usano con il campione.", mutedStyle);
    }

    void EnsureStyles()
    {
        if (statusStyle != null) return;
        buttonStyle = new GUIStyle(GUI.skin.button) { fontSize = 13, fixedHeight = 30f };
        statusStyle = new GUIStyle(GUI.skin.label) { fontSize = 16, fontStyle = FontStyle.Bold };
        statusStyle.normal.textColor = new Color(0.85f, 0.9f, 1f);
        headingStyle = new GUIStyle(GUI.skin.label) { fontSize = 14, fontStyle = FontStyle.Bold };
        headingStyle.normal.textColor = new Color(1f, 0.85f, 0.4f);
        scoreStyle = new GUIStyle(GUI.skin.label) { fontSize = 13 };
        scoreStyle.normal.textColor = new Color(0.75f, 0.8f, 0.9f);
        mineScoreStyle = new GUIStyle(scoreStyle) { fontStyle = FontStyle.Bold };
        mineScoreStyle.normal.textColor = new Color(1f, 0.85f, 0.4f);
        goalStyle = new GUIStyle(GUI.skin.label) { fontSize = 13, wordWrap = true };
        goalStyle.normal.textColor = new Color(0.65f, 0.9f, 0.75f);
        doneGoalStyle = new GUIStyle(goalStyle);
        doneGoalStyle.normal.textColor = new Color(0.5f, 0.55f, 0.6f);
        mutedStyle = new GUIStyle(GUI.skin.label) { fontSize = 12, wordWrap = true };
        mutedStyle.normal.textColor = new Color(0.6f, 0.65f, 0.72f);
        itemNameStyle = new GUIStyle(GUI.skin.label) { fontSize = 15, fontStyle = FontStyle.Bold, wordWrap = true };
        itemNameStyle.normal.textColor = Color.white;
        itemInfoStyle = new GUIStyle(GUI.skin.label) { fontSize = 12, wordWrap = true };
        itemInfoStyle.normal.textColor = new Color(0.95f, 0.85f, 0.5f);
        quantityStyle = new GUIStyle(GUI.skin.label) { fontSize = 16, fontStyle = FontStyle.Bold, alignment = TextAnchor.MiddleRight };
        quantityStyle.normal.textColor = new Color(0.85f, 0.9f, 1f);
        toastStyle = new GUIStyle(GUI.skin.label) { fontSize = 14, alignment = TextAnchor.MiddleCenter };
        toastStyle.normal.textColor = new Color(1f, 0.55f, 0.5f);
        hintStyle = new GUIStyle(GUI.skin.label) { fontSize = 15, fontStyle = FontStyle.Bold, alignment = TextAnchor.MiddleCenter };
        hintStyle.normal.textColor = new Color(0.5f, 0.9f, 1f);

        boxTexture = new Texture2D(1, 1) { hideFlags = HideFlags.HideAndDontSave };
        boxTexture.SetPixel(0, 0, new Color(0.08f, 0.09f, 0.12f, 0.9f));
        boxTexture.Apply();
        boxStyle = new GUIStyle(GUI.skin.box);
        boxStyle.normal.background = boxTexture;
        eventTitleStyle = new GUIStyle(GUI.skin.label) { fontSize = 15, fontStyle = FontStyle.Bold };
        eventTitleStyle.normal.textColor = new Color(1f, 0.85f, 0.4f);
        eventTextStyle = new GUIStyle(GUI.skin.label) { fontSize = 14, wordWrap = true };
        eventTextStyle.normal.textColor = new Color(0.92f, 0.92f, 0.92f);
    }
}
