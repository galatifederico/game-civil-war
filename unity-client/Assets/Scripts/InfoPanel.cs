using System;
using UnityEngine;

// Menu con la descrizione della pedina cliccata (IMGUI: nessuna dipendenza da UGUI).
public class InfoPanel : MonoBehaviour
{
    const float Width = 280f;
    const float Height = 200f;
    const float Margin = 12f;

    Piece piece;
    Rect rect;
    GUIStyle boxStyle, titleStyle, kindStyle, bodyStyle;

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

    public void Show(Piece p) => piece = p;

    public void Hide() => piece = null;

    void OnGUI()
    {
        if (piece == null) return;
        EnsureStyles();

        rect = new Rect(Screen.width - Width - Margin, Margin, Width, Height);
        GUI.Box(rect, GUIContent.none, boxStyle);

        GUILayout.BeginArea(new Rect(rect.x + 12f, rect.y + 10f, Width - 24f, Height - 20f));
        GUILayout.Label(piece.DisplayName, titleStyle);
        GUILayout.Label(piece.KindLabel, kindStyle);
        GUILayout.Space(6f);
        GUILayout.Label(piece.Description, bodyStyle);
        GUILayout.FlexibleSpace();
        GUILayout.Label($"Posizione: {piece.X}, {piece.Z}", kindStyle);
        if (GUILayout.Button("Chiudi"))
        {
            Hide();
            Closed?.Invoke();
        }
        GUILayout.EndArea();
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
    }
}
