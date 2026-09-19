using UnityEngine;

// Form di accesso (IMGUI). Ricorda server, email e nome; la password no.
public class LoginScreen : MonoBehaviour
{
    const string PrefServer = "server_url";
    const string PrefEmail = "email";
    const string PrefUsername = "username";
    const float Width = 340f;

    NetworkClient net;
    string serverUrl, email, username, password = "";
    bool registerMode, busy;
    string message = "";
    GUIStyle boxStyle, labelStyle, fieldStyle, buttonStyle, messageStyle, titleStyle;

    public bool Visible { get; private set; } = true;

    public event System.Action LoggedIn;

    public void Init(NetworkClient network)
    {
        net = network;
        serverUrl = PlayerPrefs.GetString(PrefServer, "http://localhost:8090");
        email = PlayerPrefs.GetString(PrefEmail, "");
        username = PlayerPrefs.GetString(PrefUsername, "");
    }

    public void Show(string info)
    {
        Visible = true;
        busy = false;
        message = info;
    }

    public void Hide()
    {
        Visible = false;
        busy = false;
        message = "";
    }

    void OnGUI()
    {
        if (!Visible) return;
        EnsureStyles();

        float height = registerMode ? 420f : 380f;
        var rect = new Rect((Screen.width - Width) / 2f, (Screen.height - height) / 2f, Width, height);
        GUI.Box(rect, GUIContent.none, boxStyle);

        GUILayout.BeginArea(new Rect(rect.x + 20f, rect.y + 16f, Width - 40f, height - 32f));
        GUILayout.Label("The Game", titleStyle);
        GUILayout.Space(8f);

        GUI.enabled = !busy;
        GUILayout.Label("Server", labelStyle);
        serverUrl = GUILayout.TextField(serverUrl, fieldStyle);
        GUILayout.Label("Email", labelStyle);
        email = GUILayout.TextField(email, fieldStyle);
        if (registerMode)
        {
            GUILayout.Label("Nome", labelStyle);
            username = GUILayout.TextField(username, 24, fieldStyle);
        }
        GUILayout.Label("Password", labelStyle);
        password = GUILayout.PasswordField(password, '*', fieldStyle);
        GUILayout.Space(8f);

        if (GUILayout.Button(busy ? "Connessione..." : registerMode ? "Registrati" : "Accedi", buttonStyle))
            Submit();
        if (GUILayout.Button(registerMode ? "Ho già un account" : "Crea un account", buttonStyle))
        {
            registerMode = !registerMode;
            message = "";
        }
        GUI.enabled = true;

        if (!string.IsNullOrEmpty(message)) GUILayout.Label(message, messageStyle);
        GUILayout.EndArea();
    }

    void Submit()
    {
        busy = true;
        message = "";
        PlayerPrefs.SetString(PrefServer, serverUrl);
        PlayerPrefs.SetString(PrefEmail, email);
        PlayerPrefs.SetString(PrefUsername, username);
        net.Authenticate(serverUrl, registerMode, email, username, password, () =>
        {
            password = "";
            LoggedIn?.Invoke();
        }, error => Show(error));
    }

    void EnsureStyles()
    {
        if (boxStyle != null) return;

        var bg = new Texture2D(1, 1) { hideFlags = HideFlags.HideAndDontSave };
        bg.SetPixel(0, 0, new Color(0.08f, 0.09f, 0.12f, 0.96f));
        bg.Apply();
        boxStyle = new GUIStyle(GUI.skin.box);
        boxStyle.normal.background = bg;

        titleStyle = new GUIStyle(GUI.skin.label) { fontSize = 24, fontStyle = FontStyle.Bold };
        titleStyle.normal.textColor = Color.white;
        labelStyle = new GUIStyle(GUI.skin.label) { fontSize = 12 };
        labelStyle.normal.textColor = new Color(0.7f, 0.75f, 0.85f);
        fieldStyle = new GUIStyle(GUI.skin.textField) { fontSize = 15, fixedHeight = 26f };
        buttonStyle = new GUIStyle(GUI.skin.button) { fontSize = 15, fixedHeight = 30f };
        messageStyle = new GUIStyle(GUI.skin.label) { fontSize = 13, wordWrap = true };
        messageStyle.normal.textColor = new Color(1f, 0.55f, 0.5f);
    }
}
