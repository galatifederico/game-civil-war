using UnityEngine;

// Crea camera e componenti all'avvio (nessun setup manuale nella scena) e li collega:
// login -> lobby dei mondi -> snapshot del server -> aggiornamenti in tempo reale.
public class GameController : MonoBehaviour
{
    const int MaxReconnectAttempts = 8;

    int reconnectAttempts;
    NetworkClient net;
    BoardManager board;
    InfoPanel panel;
    Hud hud;
    LoginScreen login;
    LobbyScreen lobby;

    [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.AfterSceneLoad)]
    static void Bootstrap()
    {
        new GameObject("Game").AddComponent<GameController>();
    }

    void Awake()
    {
        var cam = SetupCamera();

        net = gameObject.AddComponent<NetworkClient>();
        hud = gameObject.AddComponent<Hud>();
        panel = gameObject.AddComponent<InfoPanel>();
        board = gameObject.AddComponent<BoardManager>();
        login = gameObject.AddComponent<LoginScreen>();
        lobby = gameObject.AddComponent<LobbyScreen>();

        hud.Init(panel);
        board.Init(net, panel, hud, cam);
        login.Init(net);
        lobby.Init(net);
        panel.Closed += board.ClearSelection;
        net.MessageReceived += OnMessage;
        net.Disconnected += OnDisconnected;
        hud.LogoutClicked += LeaveWorld;
        login.LoggedIn += () =>
        {
            login.Hide();
            lobby.Open("");
        };
        lobby.LoggedOut += Logout;
    }

    void OnMessage(ServerMessage message)
    {
        switch (message.type)
        {
            case "snapshot":
                reconnectAttempts = 0;
                board.LoadSnapshot(message);
                login.Hide();
                lobby.Hide();
                hud.SetLogoutVisible(true);
                hud.SetStatus(net.Username);
                hud.SetScores(message.scores, message.your_player_id);
                hud.SetInventory(message.inventory);
                hud.SetGoals(message.goals);
                break;
            case "delta":
                board.ApplyDelta(message);
                if (message.scores != null && message.scores.Length > 0) hud.SetScores(message.scores, net.PlayerId);
                break;
            case "inventory":
                hud.SetInventory(message.inventory);
                break;
            case "goals":
                hud.SetGoals(message.goals);
                break;
            case "event":
                hud.ShowEvent(message.title, message.message);
                break;
            case "error":
                hud.ShowToast(message.message);
                break;
        }
    }

    // Se il server cade o si riavvia si riprova da soli con il token gia' ottenuto, a intervalli
    // crescenti; solo dopo troppi tentativi torna alla lobby (o al login se la sessione non c'e' piu').
    void OnDisconnected(string reason)
    {
        board.Clear();
        panel.Hide();
        hud.Clear();
        if (net.HasSession && net.InWorld && reconnectAttempts < MaxReconnectAttempts)
        {
            reconnectAttempts++;
            hud.SetStatus($"Connessione persa, riprovo ({reconnectAttempts}/{MaxReconnectAttempts})...");
            StartCoroutine(ReconnectAfter(Mathf.Min(2f * reconnectAttempts, 10f)));
            return;
        }
        reconnectAttempts = 0;
        net.LeaveWorld();
        lobby.Open(reason);
    }

    System.Collections.IEnumerator ReconnectAfter(float seconds)
    {
        yield return new WaitForSeconds(seconds);
        net.Reconnect();
    }

    // "Esci" in gioco: si lascia il mondo e si torna alla lobby, restando collegati all'account.
    void LeaveWorld()
    {
        StopAllCoroutines();
        reconnectAttempts = 0;
        net.LeaveWorld();
        board.Clear();
        panel.Hide();
        hud.Clear();
        lobby.Open("");
    }

    void Logout()
    {
        net.Logout();
        lobby.Hide();
        login.Show("");
    }

    // Vista 2D: la camera guarda la scena da davanti; e' BoardManager a inquadrare la board.
    static Camera SetupCamera()
    {
        var camGO = Camera.main != null ? Camera.main.gameObject : new GameObject("Main Camera");
        if (camGO.GetComponent<Camera>() == null)
        {
            camGO.AddComponent<Camera>();
            camGO.tag = "MainCamera";
        }
        if (camGO.GetComponent<AudioListener>() == null)
            camGO.AddComponent<AudioListener>();

        var cam = camGO.GetComponent<Camera>();
        cam.orthographic = true;
        cam.orthographicSize = 6f;
        cam.clearFlags = CameraClearFlags.SolidColor;
        cam.backgroundColor = new Color(0.1f, 0.1f, 0.12f);
        camGO.transform.position = new Vector3(0f, 0f, -10f);
        camGO.transform.rotation = Quaternion.identity;
        return cam;
    }
}
