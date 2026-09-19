using System.Collections.Generic;
using UnityEngine;

// Stato della connessione e messaggi brevi (es. errori del server) in alto a sinistra.
public class Hud : MonoBehaviour
{
    const float ToastSeconds = 3f;
    const int MaxToasts = 4;

    struct Toast
    {
        public string Text;
        public float Until;
    }

    readonly List<Toast> toasts = new List<Toast>();
    string status = "";
    GUIStyle statusStyle, toastStyle;

    public void SetStatus(string text) => status = text;

    public void ShowToast(string text)
    {
        toasts.Add(new Toast { Text = text, Until = Time.time + ToastSeconds });
        if (toasts.Count > MaxToasts) toasts.RemoveAt(0);
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
        foreach (var toast in toasts)
        {
            GUI.Label(new Rect(12f, y, 420f, 24f), toast.Text, toastStyle);
            y += 26f;
        }
    }

    void EnsureStyles()
    {
        if (statusStyle != null) return;
        statusStyle = new GUIStyle(GUI.skin.label) { fontSize = 14, fontStyle = FontStyle.Bold };
        statusStyle.normal.textColor = new Color(0.85f, 0.9f, 1f);
        toastStyle = new GUIStyle(GUI.skin.label) { fontSize = 14 };
        toastStyle.normal.textColor = new Color(1f, 0.55f, 0.5f);
    }
}
