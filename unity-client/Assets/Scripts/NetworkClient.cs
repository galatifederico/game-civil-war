using System;
using System.Collections;
using System.Collections.Concurrent;
using System.IO;
using System.Net.WebSockets;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using UnityEngine;
using UnityEngine.Networking;

// Unico punto di contatto con il backend: login/registrazione via REST, poi WebSocket.
// Il client e' "dumb": non decide nulla, invia comandi e mostra cio' che il server conferma.
public class NetworkClient : MonoBehaviour
{
    public event Action<ServerMessage> MessageReceived;
    public event Action<string> Disconnected;

    public string PlayerId { get; private set; }
    public string Username { get; private set; }
    public bool Connected => ws != null && ws.State == WebSocketState.Open;
    public bool HasSession => !string.IsNullOrEmpty(sessionToken);
    public bool InWorld => !string.IsNullOrEmpty(sessionWorld);
    public string WorldId => sessionWorld;

    // Indirizzo dell'app web di amministrazione, servita dallo stesso backend.
    public string AdminUrl => string.IsNullOrEmpty(sessionUrl) ? null : sessionUrl.TrimEnd('/') + "/admin/";

    // Il token ottenuto col login e il mondo scelto nella lobby: servono a riconnettersi senza
    // chiedere di nuovo le credenziali.
    string sessionUrl, sessionToken, sessionWorld;

    readonly ConcurrentQueue<Action> mainThread = new ConcurrentQueue<Action>();
    readonly SemaphoreSlim sendLock = new SemaphoreSlim(1, 1);
    ClientWebSocket ws;
    CancellationTokenSource cts;

    void Update()
    {
        while (mainThread.TryDequeue(out var action)) action();
    }

    void OnDestroy() => Close();

    // Login o registrazione: ottiene il token. Poi si sceglie un mondo dalla lobby.
    public void Authenticate(string baseUrl, bool register, string email, string username, string password,
        Action onSuccess, Action<string> onError)
    {
        var body = JsonUtility.ToJson(new AuthRequest { email = email, username = username, password = password });
        var url = baseUrl.TrimEnd('/') + (register ? "/auth/register" : "/auth/login");
        StartCoroutine(Request<AuthResponse>("POST", url, body, null, response =>
        {
            if (string.IsNullOrEmpty(response?.token))
            {
                onError("Risposta del server non valida");
                return;
            }
            PlayerId = response.player_id;
            Username = response.username;
            sessionUrl = baseUrl;
            sessionToken = response.token;
            onSuccess?.Invoke();
        }, onError));
    }

    public void FetchWorlds(Action<WorldEntry[]> onOk, Action<string> onError)
    {
        StartCoroutine(Request<WorldList>("GET", sessionUrl.TrimEnd('/') + "/worlds", null, sessionToken,
            list => onOk(list?.worlds ?? new WorldEntry[0]), onError));
    }

    // Si iscrive al mondo (la prima volta il server crea la squadra) e poi ci entra.
    public void JoinAndEnter(string worldId, string raceId, Action<string> onError)
    {
        var body = JsonUtility.ToJson(new JoinRequest { race_id = raceId ?? "" });
        StartCoroutine(Request<IdResponse>("POST", sessionUrl.TrimEnd('/') + "/worlds/" + worldId + "/join", body, sessionToken,
            _ => Enter(worldId), onError));
    }

    public void CreateWorld(string name, string description, Action<string> onCreated, Action<string> onError)
    {
        var body = JsonUtility.ToJson(new CreateWorldRequest { name = name, description = description });
        StartCoroutine(Request<IdResponse>("POST", sessionUrl.TrimEnd('/') + "/worlds", body, sessionToken,
            r => onCreated(r.id), onError));
    }

    IEnumerator Request<T>(string method, string url, string json, string token, Action<T> onOk, Action<string> onError) where T : class
    {
        using (var req = new UnityWebRequest(url, method))
        {
            if (json != null)
            {
                req.uploadHandler = new UploadHandlerRaw(Encoding.UTF8.GetBytes(json));
                req.SetRequestHeader("Content-Type", "application/json");
            }
            req.downloadHandler = new DownloadHandlerBuffer();
            if (token != null) req.SetRequestHeader("Authorization", "Bearer " + token);
            yield return req.SendWebRequest();

            if (req.result == UnityWebRequest.Result.ConnectionError)
            {
                onError("Server non raggiungibile");
                yield break;
            }
            var text = req.downloadHandler.text;
            if (req.responseCode >= 400)
            {
                var err = JsonUtility.FromJson<ErrorResponse>(text);
                onError(string.IsNullOrEmpty(err?.error) ? $"Errore del server ({req.responseCode})" : err.error);
                yield break;
            }
            onOk(JsonUtility.FromJson<T>(text));
        }
    }

    // Apre il WebSocket sul mondo scelto.
    public void Enter(string worldId)
    {
        sessionWorld = worldId;
        Connect(sessionUrl, sessionToken, worldId);
    }

    void Connect(string baseUrl, string token, string worldId)
    {
        Close();
        cts = new CancellationTokenSource();
        var uri = new UriBuilder(baseUrl)
        {
            Scheme = baseUrl.StartsWith("https") ? "wss" : "ws",
            Path = "/ws",
        }.Uri;
        _ = RunConnection(uri, token, worldId, cts.Token);
    }

    public void Reconnect()
    {
        if (HasSession && InWorld) Connect(sessionUrl, sessionToken, sessionWorld);
    }

    // Lascia il mondo ma resta collegato all'account (si torna alla lobby).
    public void LeaveWorld()
    {
        sessionWorld = null;
        Close();
    }

    public void Logout()
    {
        sessionToken = null;
        sessionWorld = null;
        Close();
    }

    public void Close()
    {
        cts?.Cancel();
        cts = null;
        ws = null;
    }

    async Task RunConnection(Uri uri, string token, string worldId, CancellationToken ct)
    {
        string reason = null;
        try
        {
            using (var socket = new ClientWebSocket())
            {
                ws = socket;
                await socket.ConnectAsync(uri, ct);
                await SendRaw(JsonUtility.ToJson(new ClientMessage { type = "auth", token = token, world_id = worldId }), ct);

                var buffer = new byte[16 * 1024];
                using (var message = new MemoryStream())
                {
                    while (!ct.IsCancellationRequested && socket.State == WebSocketState.Open)
                    {
                        message.SetLength(0);
                        WebSocketReceiveResult result;
                        do
                        {
                            result = await socket.ReceiveAsync(new ArraySegment<byte>(buffer), ct);
                            if (result.MessageType == WebSocketMessageType.Close)
                            {
                                reason = "Connessione chiusa dal server";
                                break;
                            }
                            message.Write(buffer, 0, result.Count);
                        } while (!result.EndOfMessage);

                        if (reason != null) break;
                        var text = Encoding.UTF8.GetString(message.ToArray());
                        mainThread.Enqueue(() => Dispatch(text));
                    }
                }
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception e)
        {
            Debug.LogWarning($"WebSocket: {e.Message}");
            reason = "Connessione persa con il server";
        }
        finally
        {
            if (ws != null && !ct.IsCancellationRequested) ws = null;
        }

        if (!ct.IsCancellationRequested)
            mainThread.Enqueue(() => Disconnected?.Invoke(reason ?? "Connessione persa"));
    }

    void Dispatch(string json)
    {
        var message = JsonUtility.FromJson<ServerMessage>(json);
        if (message != null) MessageReceived?.Invoke(message);
    }

    public void SendMove(string unitId, int x, int y) => SendCommand("move", unitId, null, x, y);

    // type: attack, talk, pickup (con targetId) oppure build (con la casella x, y).
    public void SendCommand(string type, string unitId, string targetId, int x = 0, int y = 0, string method = null)
    {
        var json = JsonUtility.ToJson(new ClientMessage { type = type, unit_id = unitId, target_id = targetId, x = x, y = y, method = method });
        var ct = cts?.Token ?? CancellationToken.None;
        _ = SendRawSafe(json, ct);
    }

    async Task SendRawSafe(string json, CancellationToken ct)
    {
        try { await SendRaw(json, ct); }
        catch (OperationCanceledException) { }
        catch (Exception e) { Debug.LogWarning($"Invio fallito: {e.Message}"); }
    }

    async Task SendRaw(string json, CancellationToken ct)
    {
        var bytes = Encoding.UTF8.GetBytes(json);
        await sendLock.WaitAsync(ct);
        try
        {
            if (ws != null && ws.State == WebSocketState.Open)
                await ws.SendAsync(new ArraySegment<byte>(bytes), WebSocketMessageType.Text, true, ct);
        }
        finally
        {
            sendLock.Release();
        }
    }
}
