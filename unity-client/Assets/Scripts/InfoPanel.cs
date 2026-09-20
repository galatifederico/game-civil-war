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

// La scheda della pedina cliccata: nome, razza, classe, vita, azioni e, in fondo, le caratteristiche.
// Non ha una finestra sua: il menu unico (Hud) la mostra come una delle sue pagine.
public class InfoPanel : MonoBehaviour
{
    const int ActionsPerRow = 2;

    Piece piece;
    List<PanelAction> actions = new List<PanelAction>();
    PanelAction pending;
    GUIStyle titleStyle, labelStyle, valueStyle, bodyStyle, headingStyle, buttonStyle, statusStyle, barTextStyle;

    public Piece Current => piece;

    public event Action Closed;

    public void Show(Piece p, List<PanelAction> panelActions = null)
    {
        piece = p;
        actions = panelActions ?? new List<PanelAction>();
    }

    // Toglie la scheda dal menu (la pedina resta selezionata sulla mappa).
    public void Hide() => piece = null;

    // Chiude la scheda e deseleziona la pedina.
    public void Close()
    {
        Hide();
        Closed?.Invoke();
    }

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

        GUILayout.BeginHorizontal();
        var iconRect = GUILayoutUtility.GetRect(48f, 48f, GUILayout.Width(48f), GUILayout.Height(48f));
        PixelArt.DrawSprite(iconRect, piece.Icon, piece.IconTint);
        GUILayout.BeginVertical();
        GUILayout.Label(data.name, titleStyle);
        GUILayout.Label(piece.Mine ? "La tua squadra" : piece.Data.kind == Kinds.Npc || piece.Data.kind == Kinds.Item ? "Non controllabile" : "Squadra avversaria", labelStyle);
        GUILayout.EndVertical();
        GUILayout.EndHorizontal();
        GUILayout.Space(6f);

        if (!string.IsNullOrEmpty(data.race)) Row("Razza", data.race);
        Row("Classe", piece.ClassLabel);
        if (piece.IsUnit) DrawLife(data);
        string status = StatusText();
        if (status.Length > 0) GUILayout.Label(status, statusStyle);
        if (!string.IsNullOrEmpty(data.description))
        {
            GUILayout.Space(4f);
            GUILayout.Label(data.description, bodyStyle);
        }

        if (actions.Count > 0)
        {
            GUILayout.Space(8f);
            GUILayout.Label("Azioni", headingStyle);
            for (int i = 0; i < actions.Count; i += ActionsPerRow)
            {
                GUILayout.BeginHorizontal();
                for (int j = i; j < Mathf.Min(i + ActionsPerRow, actions.Count); j++) DrawAction(actions[j]);
                GUILayout.EndHorizontal();
            }
        }

        if (piece.IsUnit)
        {
            GUILayout.Space(8f);
            GUILayout.Label("Caratteristiche", headingStyle);
            Row("Velocità", data.speed.ToString());
            Row("Vista", data.vision.ToString());
            Row("Forza", data.strength.ToString());
            foreach (var trait in data.traits ?? new TraitData[0]) Row(trait.name, trait.value.ToString());
        }
    }

    void DrawAction(PanelAction action)
    {
        var reason = action.BlockedReason?.Invoke();
        GUI.enabled = reason == null;
        if (GUILayout.Button(reason == null ? action.Label : $"{action.Label}\n({reason})", buttonStyle, GUILayout.MinHeight(38f)))
            pending = action;
        GUI.enabled = true;
    }

    void Row(string label, string value)
    {
        GUILayout.BeginHorizontal();
        GUILayout.Label(label, labelStyle, GUILayout.Width(96f));
        GUILayout.Label(value, valueStyle);
        GUILayout.EndHorizontal();
    }

    // Una barra: piena = vita intera; il colore va dal verde al rosso.
    void DrawLife(EntityData data)
    {
        GUILayout.BeginHorizontal();
        GUILayout.Label("Vita", labelStyle, GUILayout.Width(96f));
        var rect = GUILayoutUtility.GetRect(10f, 18f, GUILayout.ExpandWidth(true));
        float share = data.max_health > 0 ? Mathf.Clamp01(data.health / (float)data.max_health) : 0f;
        var previous = GUI.color;
        GUI.color = new Color(0.15f, 0.16f, 0.2f, 1f);
        GUI.DrawTexture(rect, Texture2D.whiteTexture);
        GUI.color = share > 0.5f ? new Color(0.3f, 0.75f, 0.4f) : share > 0.25f ? new Color(0.95f, 0.7f, 0.25f) : new Color(0.9f, 0.3f, 0.3f);
        GUI.DrawTexture(new Rect(rect.x, rect.y, rect.width * share, rect.height), Texture2D.whiteTexture);
        GUI.color = previous;
        GUI.Label(rect, $"{data.health}/{data.max_health}", barTextStyle);
        GUILayout.EndHorizontal();
    }

    string StatusText()
    {
        if (piece.Defeated) return $"Fuori gioco, torna tra {piece.SecondsUntilRespawn:0}s";
        if (piece.Movable && piece.SecondsUntilReady > 0f) return $"Pronta tra {piece.SecondsUntilReady:0.0}s";
        return "";
    }

    void EnsureStyles()
    {
        if (titleStyle != null) return;

        titleStyle = new GUIStyle(GUI.skin.label) { fontSize = 20, fontStyle = FontStyle.Bold, wordWrap = true };
        titleStyle.normal.textColor = Color.white;

        labelStyle = new GUIStyle(GUI.skin.label) { fontSize = 13, wordWrap = true };
        labelStyle.normal.textColor = new Color(0.62f, 0.68f, 0.78f);

        valueStyle = new GUIStyle(GUI.skin.label) { fontSize = 14, fontStyle = FontStyle.Bold, wordWrap = true };
        valueStyle.normal.textColor = new Color(0.95f, 0.95f, 0.95f);

        bodyStyle = new GUIStyle(GUI.skin.label) { fontSize = 13, fontStyle = FontStyle.Italic, wordWrap = true };
        bodyStyle.normal.textColor = new Color(0.8f, 0.82f, 0.88f);

        headingStyle = new GUIStyle(GUI.skin.label) { fontSize = 14, fontStyle = FontStyle.Bold };
        headingStyle.normal.textColor = new Color(1f, 0.85f, 0.4f);

        statusStyle = new GUIStyle(GUI.skin.label) { fontSize = 13, wordWrap = true };
        statusStyle.normal.textColor = new Color(0.95f, 0.75f, 0.4f);

        barTextStyle = new GUIStyle(GUI.skin.label) { fontSize = 12, fontStyle = FontStyle.Bold, alignment = TextAnchor.MiddleCenter };
        barTextStyle.normal.textColor = Color.white;

        buttonStyle = new GUIStyle(GUI.skin.button) { fontSize = 13, wordWrap = true };
    }
}
