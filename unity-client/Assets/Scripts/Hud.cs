using System.Collections.Generic;
using UnityEngine;

// L'interfaccia sopra la mappa (che occupa tutto lo schermo).
//
// A sinistra c'e' una barra fissa con tre icone: la prima mostra o nasconde il menu (riaprendolo
// sull'ultima pagina), la seconda apre l'inventario, la terza il menu generale (inventario,
// obiettivi, classifica, squadra). Il menu e' uno solo, accanto alla barra, e cambia pagina:
//   Home, Inventario, Dettaglio oggetto, Obiettivi, Classifica, Squadra e la Scheda della pedina
//   (che si apre da sola quando si clicca una pedina sulla mappa).
// Sopra alla pagina compaiono i messaggi del server (dialoghi con gli NPC, sconfitte...).
// Fuori dal menu restano solo le schede delle board, il suggerimento dell'azione in corso, gli
// errori brevi e i pulsanti dello zoom.
public class Hud : MonoBehaviour
{
    const float ToastSeconds = 3f;
    const float EventSeconds = 8f;
    const int MaxToasts = 4;
    const float PanelWidth = 330f;
    const float Margin = 10f;
    const float RailButton = 44f;
    const float RailGap = 6f;
    const float RailWidth = RailButton + 2f * 4f;

    enum Page { Home, Inventory, ItemDetail, Goals, Scores, Squad, Card }

    struct Toast
    {
        public string Text;
        public float Until;
    }

    // Oggetti uguali (stesso nome, effetto, icona e descrizione) si mostrano come una riga sola con la quantita'.
    class Stack
    {
        public ItemData First;
        public int Count;
        public string Key => Identify(First);
    }

    static string Identify(ItemData i) => i.name + "|" + i.effect + "|" + i.icon + "|" + i.description;

    readonly List<Toast> toasts = new List<Toast>();
    readonly List<Stack> stacks = new List<Stack>();
    string status = "", hint = "", eventTitle = "", eventText = "";
    int inventoryCount;
    ScoreData[] scores = new ScoreData[0];
    GoalData[] goals = new GoalData[0];
    string myPlayerId;
    float eventUntil;

    InfoPanel card;
    Page page = Page.Home, cardBack = Page.Home;
    bool menuOpen = true;
    Piece lastCardPiece;
    bool lastCompact;
    string detailKey;
    Piece squadOpen;
    float previewRefreshAt;
    string previewKey;
    Texture2D previewCache;
    Vector2 scroll;
    Rect railRect, panelRect, tabsRect, zoomRect, eventRect;
    Stack pendingUse;
    int pendingBoard = -1;
    string[] boardNames = new string[0];
    int currentBoard;
    System.Action<int> onBoardSelected;
    bool inWorld;
    Texture2D boxTexture;

    GUIStyle goalStyle, doneGoalStyle, buttonStyle, statusStyle, toastStyle, scoreStyle, mineScoreStyle, hintStyle, boxStyle,
        eventTitleStyle, eventTextStyle, headingStyle, itemNameStyle, itemInfoStyle, quantityStyle, mutedStyle, bigButtonStyle,
        titleStyle, railStyle, rowStyle, rowTitleStyle;

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

    // Larghezza, in pixel di schermo, che l'interfaccia copre a sinistra: la mappa si inquadra nel resto.
    public float OccupiedLeftPixels
    {
        get
        {
            float width = Margin + RailWidth + 8f;
            if (menuOpen && !Ui.Compact) width += PanelWidth + Margin;
            return width * Ui.Scale;
        }
    }

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
        scores = newScores ?? new ScoreData[0];
        myPlayerId = playerId;
    }

    public void SetGoals(GoalData[] newGoals) => goals = newGoals ?? new GoalData[0];

    public void SetInventory(ItemData[] items)
    {
        stacks.Clear();
        inventoryCount = items?.Length ?? 0;
        foreach (var item in items ?? new ItemData[0])
        {
            var stack = stacks.Find(s => s.Key == Identify(item));
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
        page = Page.Home;
        detailKey = null;
        squadOpen = null;
        menuOpen = !Ui.Compact;
        lastCardPiece = null;
    }

    // OnMouseDown scatta anche sotto i controlli IMGUI: la mappa ignora i clic che cadono su di essi.
    public bool BlocksPointer
    {
        get
        {
            if (!inWorld) return false;
            var p = Ui.Pointer;
            if (railRect.Contains(p) || zoomRect.Contains(p)) return true;
            if (menuOpen && panelRect.Contains(p)) return true;
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

        bool compact = Ui.Compact;
        if (compact != lastCompact)
        {
            // Lo schermo e' stato girato: in verticale il menu parte chiuso, in orizzontale aperto.
            lastCompact = compact;
            menuOpen = !compact;
        }

        // Una pedina appena cliccata apre la sua scheda, anche se il menu era chiuso.
        var shown = card != null ? card.Current : null;
        if (shown != null && shown != lastCardPiece)
        {
            if (page != Page.Card) cardBack = page;
            page = Page.Card;
            menuOpen = true;
        }
        lastCardPiece = shown;
        if (page == Page.Card && shown == null) page = cardBack;
        if (page == Page.ItemDetail && FindDetail() == null) page = Page.Inventory;

        float railRight = Margin + RailWidth;
        float panelWidth = Mathf.Min(PanelWidth, Ui.Width - railRight - 2f * Margin);
        float mapLeft = menuOpen ? railRight + 8f + panelWidth : railRight;
        float mapCenter = compact ? Ui.Width / 2f : (mapLeft + Ui.Width) / 2f;
        // In verticale il menu aperto copre quasi tutta la mappa: il resto si nasconde finche' non si chiude.
        bool overlayHidden = compact && menuOpen;

        if (!overlayHidden)
        {
            DrawBoardTabs(mapCenter);
            DrawHint(mapCenter);
            DrawToasts(mapCenter);
            DrawZoomButtons();
        }
        else
        {
            tabsRect = zoomRect = default;
        }
        DrawRail();
        if (menuOpen) DrawPanel(new Rect(railRight + 8f, Margin, panelWidth, Ui.Height - 2f * Margin));
        else if (Time.time < eventUntil)
        {
            eventRect = new Rect(railRight + 8f, Margin, Mathf.Min(PanelWidth, Ui.Width - railRight - 2f * Margin), 110f);
            DrawEvent(eventRect);
        }
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

    // ---- la barra a sinistra -----------------------------------------------------------------

    void DrawRail()
    {
        railRect = new Rect(Margin, Margin, RailWidth, 3f * RailButton + 2f * RailGap + 8f);
        GUI.Box(railRect, GUIContent.none, boxStyle);
        float x = railRect.x + 4f, y = railRect.y + 4f;

        // 1. Mostra/nasconde il menu: riapre l'ultima pagina.
        if (RailIcon(new Rect(x, y, RailButton, RailButton), "ui_panel", menuOpen)) menuOpen = !menuOpen;
        y += RailButton + RailGap;
        // 2. L'inventario.
        if (RailIcon(new Rect(x, y, RailButton, RailButton), "ui_bag", menuOpen && (page == Page.Inventory || page == Page.ItemDetail)))
        {
            if (menuOpen && (page == Page.Inventory || page == Page.ItemDetail)) menuOpen = false;
            else { page = Page.Inventory; menuOpen = true; }
        }
        y += RailButton + RailGap;
        // 3. Il menu generale.
        if (RailIcon(new Rect(x, y, RailButton, RailButton), "ui_menu", menuOpen && page == Page.Home))
        {
            if (menuOpen && page == Page.Home) menuOpen = false;
            else { page = Page.Home; menuOpen = true; }
        }
    }

    bool RailIcon(Rect r, string icon, bool active)
    {
        var previous = GUI.backgroundColor;
        GUI.backgroundColor = active ? new Color(1f, 0.85f, 0.4f) : Color.white;
        bool clicked = GUI.Button(r, GUIContent.none, railStyle);
        GUI.backgroundColor = previous;
        GUI.DrawTexture(new Rect(r.x + 8f, r.y + 8f, r.width - 16f, r.height - 16f), PixelArt.ItemIcon(icon), ScaleMode.ScaleToFit);
        return clicked;
    }

    // ---- il menu -----------------------------------------------------------------------------

    void DrawPanel(Rect rect)
    {
        panelRect = rect;
        GUI.Box(rect, GUIContent.none, boxStyle);
        GUILayout.BeginArea(new Rect(rect.x + 10f, rect.y + 8f, rect.width - 20f, rect.height - 16f));

        GUILayout.BeginHorizontal();
        if (page != Page.Home && GUILayout.Button("<", buttonStyle, GUILayout.Width(34f))) GoBack();
        GUILayout.Label(PageTitle(), titleStyle);
        GUILayout.FlexibleSpace();
        if (GUILayout.Button("x", buttonStyle, GUILayout.Width(34f))) menuOpen = false;
        GUILayout.EndHorizontal();
        GUILayout.Space(4f);

        if (Time.time < eventUntil)
        {
            eventRect = GUILayoutUtility.GetRect(rect.width - 20f, 96f);
            DrawEvent(eventRect);
            GUILayout.Space(6f);
        }

        scroll = GUILayout.BeginScrollView(scroll);
        switch (page)
        {
            case Page.Inventory: DrawInventory(); break;
            case Page.ItemDetail: DrawItemDetail(); break;
            case Page.Goals: DrawGoals(); break;
            case Page.Scores: DrawScores(); break;
            case Page.Squad: DrawSquad(); break;
            case Page.Card: card.DrawCard(); break;
            default: DrawHome(); break;
        }
        GUILayout.EndScrollView();
        GUILayout.EndArea();
    }

    string PageTitle()
    {
        switch (page)
        {
            case Page.Inventory: return $"Inventario ({inventoryCount})";
            case Page.ItemDetail: return "Oggetto";
            case Page.Goals: return "Obiettivi";
            case Page.Scores: return "Classifica";
            case Page.Squad: return "Squadra";
            case Page.Card: return "Scheda";
            default: return "Menu";
        }
    }

    void GoBack()
    {
        scroll = Vector2.zero;
        switch (page)
        {
            case Page.ItemDetail: page = Page.Inventory; break;
            case Page.Card:
                card.Close();
                page = cardBack;
                break;
            default: page = Page.Home; break;
        }
    }

    void Go(Page next)
    {
        page = next;
        scroll = Vector2.zero;
    }

    void DrawEvent(Rect r)
    {
        GUI.Box(r, GUIContent.none, boxStyle);
        GUI.Label(new Rect(r.x + 10f, r.y + 6f, r.width - 20f, 22f), eventTitle, eventTitleStyle);
        GUI.Label(new Rect(r.x + 10f, r.y + 30f, r.width - 20f, r.height - 34f), eventText, eventTextStyle);
    }

    // ---- pagine ------------------------------------------------------------------------------

    void DrawHome()
    {
        if (!string.IsNullOrEmpty(status)) GUILayout.Label(status, statusStyle);
        GUILayout.Space(6f);
        if (GUILayout.Button($"Inventario ({inventoryCount})", bigButtonStyle)) Go(Page.Inventory);
        if (GUILayout.Button("Obiettivi", bigButtonStyle)) Go(Page.Goals);
        if (GUILayout.Button("Classifica", bigButtonStyle)) Go(Page.Scores);
        int units = BoardManager.Instance != null ? BoardManager.Instance.MyUnits().Count : 0;
        if (GUILayout.Button($"Squadra ({units} pedine)", bigButtonStyle)) Go(Page.Squad);
        GUILayout.Space(16f);
        if (GUILayout.Button("Esci dal mondo", buttonStyle)) LogoutClicked?.Invoke();
    }

    void DrawScores()
    {
        if (scores.Length == 0) GUILayout.Label("Nessun punteggio.", mutedStyle);
        for (int i = 0; i < scores.Length; i++)
        {
            var s = scores[i];
            bool mine = s.player_id == myPlayerId;
            GUILayout.Label($"{i + 1}. {s.username}{(mine ? " (tu)" : "")}{(s.afk ? " (assente)" : "")}  -  {s.points} pt", mine ? mineScoreStyle : scoreStyle);
        }
    }

    void DrawGoals()
    {
        if (goals.Length == 0) GUILayout.Label("Questo mondo non ha obiettivi.", mutedStyle);
        foreach (var g in goals)
        {
            GUILayout.BeginVertical(GUI.skin.box);
            string scope = g.scope == "world" ? "Mondo" : "Tuo";
            GUILayout.Label($"{g.title}  ({scope})", g.completed ? doneGoalStyle : headingStyle);
            if (!string.IsNullOrEmpty(g.description)) GUILayout.Label(g.description, mutedStyle);
            if (g.scope == "world" && g.completed) GUILayout.Label($"Vinto da {g.achieved_by}", doneGoalStyle);
            else if (g.completed) GUILayout.Label("Completato", doneGoalStyle);
            else
            {
                DrawBar(g.target > 0 ? g.progress / (float)g.target : 0f, $"{g.progress}/{g.target}", new Color(0.35f, 0.75f, 0.55f));
                GUILayout.Label($"Premio: +{g.reward} pt", itemInfoStyle);
            }
            GUILayout.EndVertical();
        }
    }

    // ---- squadra: l'elenco delle pedine, con l'anteprima di dove sono ---------------------------

    void DrawSquad()
    {
        var board = BoardManager.Instance;
        var units = board != null ? board.MyUnits() : new List<Piece>();
        if (units.Count == 0) GUILayout.Label("Nessuna pedina.", mutedStyle);
        foreach (var unit in units)
        {
            bool open = squadOpen == unit;
            var rect = GUILayoutUtility.GetRect(10f, 44f, GUILayout.ExpandWidth(true));
            if (GUI.Button(rect, GUIContent.none, rowStyle))
            {
                squadOpen = open ? null : unit;
                if (!open) board.Locate(unit);
            }
            var d = unit.Data;
            var previous = GUI.color;
            GUI.color = unit.TeamColor;
            GUI.DrawTexture(new Rect(rect.x + 6f, rect.y + 6f, 32f, 32f), unit.Icon.texture, ScaleMode.ScaleToFit);
            GUI.color = previous;
            GUI.Label(new Rect(rect.x + 44f, rect.y + 3f, rect.width - 50f, 20f), d.name + (d.kind == Kinds.Champion ? "  (campione)" : ""), rowTitleStyle);
            string where = board.BoardName(d.board_id);
            string life = unit.Defeated ? "fuori gioco" : $"vita {d.health}/{d.max_health}";
            GUI.Label(new Rect(rect.x + 44f, rect.y + 22f, rect.width - 50f, 18f), $"{life}  ·  {where}", mutedStyle);

            if (!open) continue;

            GUILayout.BeginVertical(GUI.skin.box);
            // L'anteprima si ridisegna solo se la pedina si e' mossa o ogni mezzo secondo (le altre si muovono).
            string previewId = $"{d.id}|{d.board_id}|{d.x}|{d.y}";
            if (previewId != previewKey || Time.time >= previewRefreshAt)
            {
                previewKey = previewId;
                previewRefreshAt = Time.time + 0.5f;
                previewCache = board.BoardPreview(unit, 280, 140);
            }
            var preview = previewCache;
            if (preview != null)
            {
                var pr = GUILayoutUtility.GetRect(10f, preview.height * (280f / Mathf.Max(preview.width, 1)), GUILayout.ExpandWidth(true));
                GUI.DrawTexture(pr, preview, ScaleMode.ScaleToFit);
            }
            GUILayout.Label($"{board.BoardName(d.board_id)}  ·  casella ({d.x}, {d.y})", mutedStyle);
            GUILayout.BeginHorizontal();
            if (GUILayout.Button("Scheda", buttonStyle))
            {
                if (page != Page.Card) cardBack = Page.Squad;
                board.SelectAndShow(unit);
            }
            if (GUILayout.Button("Mostra sulla mappa", buttonStyle)) board.Locate(unit);
            GUILayout.EndHorizontal();
            GUILayout.EndVertical();
        }
    }

    // ---- inventario ------------------------------------------------------------------------

    void DrawInventory()
    {
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
            GUILayout.BeginHorizontal();
            GUILayout.Label(item.name, itemNameStyle);
            GUILayout.FlexibleSpace();
            GUILayout.Label($"x{stack.Count}", quantityStyle);
            GUILayout.EndHorizontal();
            if (!string.IsNullOrEmpty(item.effect)) GUILayout.Label(item.effect, itemInfoStyle);
            GUILayout.BeginHorizontal();
            if (!string.IsNullOrEmpty(item.effect) && GUILayout.Button("Usa", buttonStyle)) pendingUse = stack;
            if (GUILayout.Button("Descrizione", buttonStyle))
            {
                detailKey = stack.Key;
                Go(Page.ItemDetail);
            }
            GUILayout.EndHorizontal();
            GUILayout.EndVertical();

            GUILayout.EndHorizontal();
        }
    }

    Stack FindDetail() => detailKey == null ? null : stacks.Find(s => s.Key == detailKey);

    // La pagina di un oggetto: tutto quello che serve per decidere se usarlo.
    void DrawItemDetail()
    {
        var stack = FindDetail();
        if (stack == null) return;
        var item = stack.First;

        GUILayout.BeginHorizontal();
        var iconRect = GUILayoutUtility.GetRect(64f, 64f, GUILayout.Width(64f), GUILayout.Height(64f));
        GUI.DrawTexture(iconRect, PixelArt.ItemIcon(item.icon), ScaleMode.ScaleToFit);
        GUILayout.BeginVertical();
        GUILayout.Label(item.name, titleStyle);
        GUILayout.Label($"Ne hai {stack.Count}", mutedStyle);
        GUILayout.EndVertical();
        GUILayout.EndHorizontal();

        GUILayout.Space(8f);
        GUILayout.Label("Descrizione", headingStyle);
        GUILayout.Label(string.IsNullOrEmpty(item.description) ? "Nessuna descrizione." : item.description, bodyBright);

        GUILayout.Space(8f);
        GUILayout.Label("Effetto", headingStyle);
        var lines = item.effect_lines != null && item.effect_lines.Length > 0 ? item.effect_lines : new[] { string.IsNullOrEmpty(item.effect) ? "Nessun effetto." : item.effect };
        foreach (var line in lines) GUILayout.Label("•  " + line, itemInfoStyle);

        GUILayout.Space(8f);
        GUILayout.Label("Come si usa", headingStyle);
        bool usable = !string.IsNullOrEmpty(item.effect);
        GUILayout.Label(usable
            ? "Lo usa il campione (deve essere in gioco). L'effetto parte da lui e l'oggetto viene consumato."
            : "Non si può usare: serve solo come oggetto d'ambiente.", bodyBright);

        GUILayout.Space(10f);
        GUILayout.BeginHorizontal();
        if (usable && GUILayout.Button("Usa", bigButtonStyle)) pendingUse = stack;
        if (GUILayout.Button("Torna all'inventario", bigButtonStyle)) Go(Page.Inventory);
        GUILayout.EndHorizontal();
    }

    void DrawBar(float share, string text, Color fill)
    {
        var rect = GUILayoutUtility.GetRect(10f, 16f, GUILayout.ExpandWidth(true));
        var previous = GUI.color;
        GUI.color = new Color(0.15f, 0.16f, 0.2f, 1f);
        GUI.DrawTexture(rect, Texture2D.whiteTexture);
        GUI.color = fill;
        GUI.DrawTexture(new Rect(rect.x, rect.y, rect.width * Mathf.Clamp01(share), rect.height), Texture2D.whiteTexture);
        GUI.color = previous;
        GUI.Label(rect, text, barTextStyle);
    }

    GUIStyle bodyBright, barTextStyle;

    void EnsureStyles()
    {
        if (statusStyle != null) return;
        buttonStyle = new GUIStyle(GUI.skin.button) { fontSize = 13, fixedHeight = 30f };
        bigButtonStyle = new GUIStyle(GUI.skin.button) { fontSize = 15, fixedHeight = 42f };
        railStyle = new GUIStyle(GUI.skin.button);
        rowStyle = new GUIStyle(GUI.skin.button);
        statusStyle = new GUIStyle(GUI.skin.label) { fontSize = 16, fontStyle = FontStyle.Bold };
        statusStyle.normal.textColor = new Color(0.85f, 0.9f, 1f);
        titleStyle = new GUIStyle(GUI.skin.label) { fontSize = 18, fontStyle = FontStyle.Bold, wordWrap = true };
        titleStyle.normal.textColor = Color.white;
        headingStyle = new GUIStyle(GUI.skin.label) { fontSize = 14, fontStyle = FontStyle.Bold, wordWrap = true };
        headingStyle.normal.textColor = new Color(1f, 0.85f, 0.4f);
        scoreStyle = new GUIStyle(GUI.skin.label) { fontSize = 14 };
        scoreStyle.normal.textColor = new Color(0.75f, 0.8f, 0.9f);
        mineScoreStyle = new GUIStyle(scoreStyle) { fontStyle = FontStyle.Bold };
        mineScoreStyle.normal.textColor = new Color(1f, 0.85f, 0.4f);
        goalStyle = new GUIStyle(GUI.skin.label) { fontSize = 13, wordWrap = true };
        goalStyle.normal.textColor = new Color(0.65f, 0.9f, 0.75f);
        doneGoalStyle = new GUIStyle(headingStyle);
        doneGoalStyle.normal.textColor = new Color(0.5f, 0.55f, 0.6f);
        mutedStyle = new GUIStyle(GUI.skin.label) { fontSize = 12, wordWrap = true };
        mutedStyle.normal.textColor = new Color(0.6f, 0.65f, 0.72f);
        bodyBright = new GUIStyle(GUI.skin.label) { fontSize = 13, wordWrap = true };
        bodyBright.normal.textColor = new Color(0.9f, 0.9f, 0.9f);
        rowTitleStyle = new GUIStyle(GUI.skin.label) { fontSize = 14, fontStyle = FontStyle.Bold };
        rowTitleStyle.normal.textColor = Color.white;
        itemNameStyle = new GUIStyle(GUI.skin.label) { fontSize = 15, fontStyle = FontStyle.Bold, wordWrap = true };
        itemNameStyle.normal.textColor = Color.white;
        itemInfoStyle = new GUIStyle(GUI.skin.label) { fontSize = 13, wordWrap = true };
        itemInfoStyle.normal.textColor = new Color(0.95f, 0.85f, 0.5f);
        quantityStyle = new GUIStyle(GUI.skin.label) { fontSize = 15, fontStyle = FontStyle.Bold, alignment = TextAnchor.MiddleRight };
        quantityStyle.normal.textColor = new Color(0.85f, 0.9f, 1f);
        barTextStyle = new GUIStyle(GUI.skin.label) { fontSize = 12, fontStyle = FontStyle.Bold, alignment = TextAnchor.MiddleCenter };
        barTextStyle.normal.textColor = Color.white;
        toastStyle = new GUIStyle(GUI.skin.label) { fontSize = 14, alignment = TextAnchor.MiddleCenter };
        toastStyle.normal.textColor = new Color(1f, 0.55f, 0.5f);
        hintStyle = new GUIStyle(GUI.skin.label) { fontSize = 15, fontStyle = FontStyle.Bold, alignment = TextAnchor.MiddleCenter };
        hintStyle.normal.textColor = new Color(0.5f, 0.9f, 1f);

        boxTexture = new Texture2D(1, 1) { hideFlags = HideFlags.HideAndDontSave };
        boxTexture.SetPixel(0, 0, new Color(0.08f, 0.09f, 0.12f, 0.92f));
        boxTexture.Apply();
        boxStyle = new GUIStyle(GUI.skin.box);
        boxStyle.normal.background = boxTexture;
        eventTitleStyle = new GUIStyle(GUI.skin.label) { fontSize = 15, fontStyle = FontStyle.Bold };
        eventTitleStyle.normal.textColor = new Color(1f, 0.85f, 0.4f);
        eventTextStyle = new GUIStyle(GUI.skin.label) { fontSize = 14, wordWrap = true };
        eventTextStyle.normal.textColor = new Color(0.92f, 0.92f, 0.92f);
    }
}
