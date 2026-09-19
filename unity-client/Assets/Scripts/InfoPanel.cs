using System;
using System.Collections.Generic;
using UnityEngine;

// Un pulsante del menu. BlockedReason viene valutato a ogni frame: null = disponibile,
// altrimenti il pulsante e' disattivato e il motivo compare tra parentesi.
public class PanelAction
{
    public string Label;
    public Action Perform;
    public Func<string> BlockedReason;
}

// Menu con la descrizione della pedina cliccata e le azioni possibili (IMGUI: nessuna
// dipendenza da UGUI).
public class InfoPanel : MonoBehaviour
{
    const float Width = 290f;
    const float BaseHeight = 250f;
    const float ButtonHeight = 36f;
    const float Margin = 12f;

    Piece piece;
    List<PanelAction> actions = new List<PanelAction>();
    PanelAction pending;
    Rect rect;
    GUIStyle boxStyle, titleStyle, kindStyle, bodyStyle, statsStyle, buttonStyle;

    public Piece Current => piece;

    public event Action Closed;

    // OnMouseDown scatta anche sotto i controlli IMGUI: la scena deve ignorare i click sul pannello.
    public bool BlocksPointer
    {
        get
        {
            if (piece == null) return false;
            var m = Input.mousePosition;
            return rect.Contains(new Vector2(m.x, Screen.height - m.y));
        }
    }

    public void Show(Piece p, List<PanelAction> panelActions = null)
    {
        piece = p;
        actions = panelActions ?? new List<PanelAction>();
    }

    public void Hide() => piece = null;

    // Le azioni cambiano lo stato del pannello: si eseguono fuori da OnGUI, tra un frame e l'altro.
    void Update()
    {
        if (pending == null) return;
        var action = pending;
        pending = null;
        action.Perform();
    }

    void OnGUI()
    {
        if (piece == null) return;
        EnsureStyles();

        var data = piece.Data;
        float height = BaseHeight + actions.Count * ButtonHeight;
        rect = new Rect(Screen.width - Width - Margin, Margin, Width, height);
        GUI.Box(rect, GUIContent.none, boxStyle);

        GUILayout.BeginArea(new Rect(rect.x + 12f, rect.y + 10f, Width - 24f, height - 20f));
        GUILayout.Label(data.name, titleStyle);
        GUILayout.Label(piece.KindLabel, kindStyle);
        GUILayout.Space(6f);
        GUILayout.Label(data.description, bodyStyle);
        GUILayout.FlexibleSpace();
        if (piece.IsUnit)
        {
            GUILayout.Label($"Velocità {data.speed}  ·  Vita {data.health}/{data.max_health}  ·  Vista {data.vision}", statsStyle);
        }
        GUILayout.Label($"Posizione: {data.x}, {data.y}{StatusSuffix()}", kindStyle);

        foreach (var action in actions)
        {
            var reason = action.BlockedReason?.Invoke();
            GUI.enabled = reason == null;
            if (GUILayout.Button(reason == null ? action.Label : $"{action.Label} ({reason})", buttonStyle))
                pending = action;
            GUI.enabled = true;
        }

        if (GUILayout.Button("Chiudi", buttonStyle))
        {
            Hide();
            Closed?.Invoke();
        }
        GUILayout.EndArea();
    }

    string StatusSuffix()
    {
        if (piece.Defeated) return $"  ·  fuori gioco, torna tra {piece.SecondsUntilRespawn:0}s";
        if (piece.Movable && piece.SecondsUntilReady > 0f) return $"  ·  pronta tra {piece.SecondsUntilReady:0.0}s";
        return "";
    }

    void EnsureStyles()
    {
        if (boxStyle != null) return;

        var bg = new Texture2D(1, 1) { hideFlags = HideFlags.HideAndDontSave };
        bg.SetPixel(0, 0, new Color(0.08f, 0.09f, 0.12f, 0.94f));
        bg.Apply();

        boxStyle = new GUIStyle(GUI.skin.box);
        boxStyle.normal.background = bg;

        titleStyle = new GUIStyle(GUI.skin.label) { fontSize = 20, fontStyle = FontStyle.Bold };
        titleStyle.normal.textColor = Color.white;

        kindStyle = new GUIStyle(GUI.skin.label) { fontSize = 12, fontStyle = FontStyle.Italic };
        kindStyle.normal.textColor = new Color(0.7f, 0.75f, 0.85f);

        bodyStyle = new GUIStyle(GUI.skin.label) { fontSize = 14, wordWrap = true };
        bodyStyle.normal.textColor = new Color(0.9f, 0.9f, 0.9f);

        statsStyle = new GUIStyle(GUI.skin.label) { fontSize = 13 };
        statsStyle.normal.textColor = new Color(0.95f, 0.85f, 0.5f);

        buttonStyle = new GUIStyle(GUI.skin.button) { fontSize = 14, fixedHeight = 30f };
    }
}
