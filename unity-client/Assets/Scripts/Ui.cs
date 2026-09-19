using UnityEngine;

// Misure comuni dell'interfaccia IMGUI, cosi' che lo stesso codice di layout vada bene su un
// monitor e su un telefono: sui display ad alta densita' tutto si ingrandisce (i pulsanti
// restano toccabili) e in verticale il menu parte chiuso e, aperto, copre quasi tutta la mappa.
public static class Ui
{
    // Larghezza minima, in unita' di interfaccia, che il layout deve poter usare.
    const float MinVirtualWidth = 400f;

    // 0 = automatico. Serve a provare il layout da telefono nell'editor.
    public static float ScaleOverride;
    public static bool ForceCompact;

    public static float Scale
    {
        get
        {
            float s = ScaleOverride > 0f ? ScaleOverride : Application.isMobilePlatform ? Mathf.Clamp(Screen.dpi / 160f, 1f, 3f) : 1f;
            return Mathf.Max(0.5f, Mathf.Min(s, Screen.width / MinVirtualWidth));
        }
    }

    public static float Width => Screen.width / Scale;
    public static float Height => Screen.height / Scale;

    // Schermo in verticale (telefono): il menu, largo come lo schermo, parte chiuso.
    public static bool Compact => ForceCompact || Screen.height > Screen.width;

    // Va chiamato all'inizio di ogni OnGUI: da li' in poi si disegna in unita' di interfaccia.
    public static void Begin() => GUI.matrix = Matrix4x4.Scale(new Vector3(Scale, Scale, 1f));

    // Il puntatore in unita' di interfaccia, con l'origine in alto a sinistra come IMGUI.
    public static Vector2 Pointer
    {
        get
        {
            var m = Input.mousePosition;
            return new Vector2(m.x / Scale, (Screen.height - m.y) / Scale);
        }
    }
}
