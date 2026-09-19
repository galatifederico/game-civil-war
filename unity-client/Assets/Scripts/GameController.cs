using UnityEngine;

// Crea camera, luce e componenti all'avvio (nessun setup manuale nella scena) e li collega:
// login -> snapshot del server -> aggiornamenti in tempo reale.
public class GameController : MonoBehaviour
{
    NetworkClient net;
    BoardManager board;
    InfoPanel panel;
    Hud hud;
    LoginScreen login;

    [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.AfterSceneLoad)]
    static void Bootstrap()
    {
        new GameObject("Game").AddComponent<GameController>();
    }

    void Awake()
    {
        var cam = SetupCamera();
        SetupLight();

        net = gameObject.AddComponent<NetworkClient>();
        hud = gameObject.AddComponent<Hud>();
        panel = gameObject.AddComponent<InfoPanel>();
        board = gameObject.AddComponent<BoardManager>();
        login = gameObject.AddComponent<LoginScreen>();

        board.Init(net, panel, cam);
        login.Init(net);
        panel.Closed += board.ClearSelection;
        net.MessageReceived += OnMessage;
        net.Disconnected += OnDisconnected;
    }

    void OnMessage(ServerMessage message)
    {
        switch (message.type)
        {
            case "snapshot":
                board.LoadSnapshot(message);
                login.Hide();
                hud.SetStatus($"{net.Username} - {message.board.name} {message.board.width}x{message.board.height}");
                break;
            case "delta":
                board.ApplyDelta(message);
                break;
            case "error":
                hud.ShowToast(message.message);
                break;
        }
    }

    void OnDisconnected(string reason)
    {
        board.Clear();
        panel.Hide();
        hud.SetStatus("");
        login.Show(reason);
    }

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
        camGO.transform.position = new Vector3(0f, 10f, 0f);
        camGO.transform.rotation = Quaternion.Euler(90f, 0f, 0f);
        return cam;
    }

    void SetupLight()
    {
        var lightGO = new GameObject("Directional Light");
        lightGO.transform.SetParent(transform);
        lightGO.transform.rotation = Quaternion.Euler(90f, 0f, 0f);
        lightGO.AddComponent<Light>().type = LightType.Directional;
    }
}
