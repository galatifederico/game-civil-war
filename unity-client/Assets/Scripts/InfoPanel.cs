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

// La scheda della pedina cliccata (descrizione e azioni possibili). Non ha una finestra sua: il
// menu unico (Hud) la mostra al posto del contenuto normale finche' c'e' una pedina da mostrare.
public class InfoPanel : MonoBehaviour
{
    const float ButtonHeight = 32f;

    Piece piece;
    List<PanelAction> actions = new List<PanelAction>();
    PanelAction pending;
    GUIStyle titleStyle, kindStyle, bodyStyle, statsStyle, traitStyle, buttonStyle;

    public Piece Current => piece;

    public event Action Closed;

    public void Show(Piece p, List<PanelAction> panelActions = null)
    {
        piece = p;
        actions = panelActions ?? new List<PanelAction>();
    }

    // Toglie la scheda dal menu (la pedina resta selezionata sulla mappa).
    public void Hide() => piece = null;

    // Le azioni cambiano lo stato del pannello: si eseguono fuori da OnGUI, tra un frame e l'altro.
    void Update()
    {
        if (pending == null) return;
        var action = pending;
        pending = null;
        action.Perform();
    }

    // Disegna la scheda con controlli GUILayout, dentro l'area del menu.
    public void DrawCard()
    {
        if (piece == null) return;
        EnsureStyles();

        var data = piece.Data;
        GUILayout.Label(data.name, titleStyle);
        GUILayout.Label(piece.KindLabel, kindStyle);
        GUILayout.Space(6f);
        GUILayout.Label(data.description, bodyStyle);
        GUILayout.Space(8f);
        if (piece.IsUnit)
            GUILayout.Label($"Velocità {data.speed}  ·  Vita {data.health}/{data.max_health}  ·  Vista {data.vision}", statsStyle);
        string traits = TraitsText(data);
        if (traits.Length > 0) GUILayout.Label(traits, traitStyle);
        GUILayout.Label($"Posizione: {data.x}, {data.y}{StatusSuffix()}", kindStyle);
        GUILayout.Space(8f);

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
    }

    // Le caratteristiche estese della pedina (soldi, alcol...): il server le manda gia' nell'ordine del mondo.
    static string TraitsText(EntityData data)
    {
        if (data.traits == null || data.traits.Length == 0) return "";
        var parts = new string[data.traits.Length];
        for (int i = 0; i < parts.Length; i++) parts[i] = $"{data.traits[i].name} {data.traits[i].value}";
        return string.Join("  ·  ", parts);
    }

    string StatusSuffix()
    {
        if (piece.Defeated) return $"  ·  fuori gioco, torna tra {piece.SecondsUntilRespawn:0}s";
        if (piece.Movable && piece.SecondsUntilReady > 0f) return $"  ·  pronta tra {piece.SecondsUntilReady:0.0}s";
        return "";
    }

    void EnsureStyles()
    {
        if (titleStyle != null) return;

        titleStyle = new GUIStyle(GUI.skin.label) { fontSize = 20, fontStyle = FontStyle.Bold, wordWrap = true };
        titleStyle.normal.textColor = Color.white;

        kindStyle = new GUIStyle(GUI.skin.label) { fontSize = 12, fontStyle = FontStyle.Italic, wordWrap = true };
        kindStyle.normal.textColor = new Color(0.7f, 0.75f, 0.85f);

        bodyStyle = new GUIStyle(GUI.skin.label) { fontSize = 14, wordWrap = true };
        bodyStyle.normal.textColor = new Color(0.9f, 0.9f, 0.9f);

        statsStyle = new GUIStyle(GUI.skin.label) { fontSize = 13, wordWrap = true };
        statsStyle.normal.textColor = new Color(0.95f, 0.85f, 0.5f);

        traitStyle = new GUIStyle(GUI.skin.label) { fontSize = 12, wordWrap = true };
        traitStyle.normal.textColor = new Color(0.65f, 0.85f, 0.75f);

        buttonStyle = new GUIStyle(GUI.skin.button) { fontSize = 14, fixedHeight = ButtonHeight };
    }
}
