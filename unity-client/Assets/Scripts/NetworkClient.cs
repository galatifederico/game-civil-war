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

    readonly ConcurrentQueue<Action> mainThread = new ConcurrentQueue<Action>();
    readonly SemaphoreSlim sendLock = new SemaphoreSlim(1, 1);
    ClientWebSocket ws;
    CancellationTokenSource cts;

    void Update()
    {
        while (mainThread.TryDequeue(out var action)) action();
    }

    void OnDestroy() => Close();

    public void Authenticate(string baseUrl, bool register, string email, string username, string password, Action<string> onError)
    {
        var body = JsonUtility.ToJson(new AuthRequest { email = email, username = username, password = password });
        var url = baseUrl.TrimEnd('/') + (register ? "/auth/register" : "/auth/login");
        StartCoroutine(AuthRoutine(url, body, response =>
        {
            PlayerId = response.player_id;
            Username = response.username;
            Connect(baseUrl, response.token);
        }, onError));
    }

    IEnumerator AuthRoutine(string url, string json, Action<AuthResponse> onOk, Action<string> onError)
    {
        using (var req = new UnityWebRequest(url, "POST"))
        {
            req.uploadHandler = new UploadHandlerRaw(Encoding.UTF8.GetBytes(json));
            req.downloadHandler = new DownloadHandlerBuffer();
            req.SetRequestHeader("Content-Type", "application/json");
            yield return req.SendWebRequest();

            if (req.result == UnityWebRequest.Result.ConnectionError)
            {
                onError("Server non raggiungibile");
                yield break;
            }
            var response = JsonUtility.FromJson<AuthResponse>(req.downloadHandler.text);
            if (req.responseCode >= 400 || response == null || string.IsNullOrEmpty(response.token))
            {
                onError(string.IsNullOrEmpty(response?.error) ? "Errore del server" : response.error);
                yield break;
            }
            onOk(response);
        }
    }

    void Connect(string baseUrl, string token)
    {
        Close();
        cts = new CancellationTokenSource();
        var uri = new UriBuilder(baseUrl)
        {
            Scheme = baseUrl.StartsWith("https") ? "wss" : "ws",
            Path = "/ws",
        }.Uri;
        _ = RunConnection(uri, token, cts.Token);
    }

    public void Close()
    {
        cts?.Cancel();
        cts = null;
        ws = null;
    }

    async Task RunConnection(Uri uri, string token, CancellationToken ct)
    {
        string reason = null;
        try
        {
            using (var socket = new ClientWebSocket())
            {
                ws = socket;
                await socket.ConnectAsync(uri, ct);
                await SendRaw(JsonUtility.ToJson(new ClientMessage { type = "auth", token = token }), ct);

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
    public void SendCommand(string type, string unitId, string targetId, int x = 0, int y = 0)
    {
        var json = JsonUtility.ToJson(new ClientMessage { type = type, unit_id = unitId, target_id = targetId, x = x, y = y });
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
